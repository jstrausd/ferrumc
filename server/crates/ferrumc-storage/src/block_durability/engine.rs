//! Shared private plan, complete endpoint proof and bounded nonrecursive historical relations.
use super::{
    ownership_adapter, raw, Arc, BlockDurabilityConfig, BlockDurabilityError, BlockPos,
    BlockReceipt, BlockReceiptQuery, BlockResult, BlockSnapshotSpec, BlockStateId, Chunk, ChunkKey,
    GenerationIdentity, GenerationKey, OwnedBlockColumn, OwnedBlockSnapshot, OwnershipStamp,
    OwnershipTransferId, Pair, Request, StorageError, Weak, MAX_BLOCK_CHUNKS,
};
use crate::ownership::{engine as owned, raw as owner_raw};
use crate::persistent_entity::engine::EntityRead;
use ferrumc_items::persisted_stack_size;
use ferrumc_world::{
    decode_block_entity, try_encode_block_entity, BlockEntity, FlatWorldGenerator, SignFace,
    MAX_BLOCK_ENTITY_PAYLOAD_LEN,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Table {
    Full,
    Overlay,
    Header,
    Journal,
}
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Counts {
    pub(crate) chunks: u64,
    pub(crate) receipt_bytes: u64,
    pub(crate) journal_rows: u64,
    pub(crate) journal_bytes: u64,
    pub(crate) watermark: Option<u64>,
}
/// One budget per operation; decoding/composition reserve uninstrumented whole traversals.
#[derive(Clone)]
pub(crate) struct Budget {
    bytes: std::rc::Rc<std::cell::Cell<u64>>,
    cells: std::rc::Rc<std::cell::Cell<u64>>,
    max_bytes: u64,
    max_cells: u64,
    startup: bool,
    #[cfg(test)]
    pub(crate) frame_work: std::rc::Rc<std::cell::Cell<(u64, u64)>>,
    // Per-call caches bound repeated work, never authority: one call reads one coherent state.
    validated: std::rc::Rc<std::cell::RefCell<BTreeSet<OwnershipTransferId>>>,
    proved: std::rc::Rc<std::cell::RefCell<BTreeSet<OwnershipTransferId>>>,
    columns: std::rc::Rc<std::cell::RefCell<BTreeSet<(OwnershipTransferId, usize)>>>,
}
impl Budget {
    pub(crate) fn hard() -> Self {
        Self {
            bytes: std::rc::Rc::new(std::cell::Cell::new(0)),
            cells: std::rc::Rc::new(std::cell::Cell::new(0)),
            max_bytes: 512 * 1024 * 1024,
            max_cells: 16_777_216,
            startup: false,
            #[cfg(test)]
            frame_work: std::rc::Rc::new(std::cell::Cell::new((0, 0))),
            validated: std::rc::Rc::new(std::cell::RefCell::new(BTreeSet::new())),
            proved: std::rc::Rc::new(std::cell::RefCell::new(BTreeSet::new())),
            columns: std::rc::Rc::new(std::cell::RefCell::new(BTreeSet::new())),
        }
    }
    pub(crate) fn runtime(config: BlockDurabilityConfig) -> Self {
        Self {
            max_bytes: config.limits.bytes,
            max_cells: config.limits.cells,
            ..Self::hard()
        }
    }
    pub(crate) fn startup(config: BlockDurabilityConfig) -> Self {
        Self {
            max_bytes: config.limits.relation_bytes,
            max_cells: config.limits.relation_cells,
            startup: true,
            ..Self::hard()
        }
    }
    fn err(&self, what: &'static str) -> BlockDurabilityError {
        if self.startup {
            BlockDurabilityError::ScanCapacity(what)
        } else {
            BlockDurabilityError::ValidationCapacity(what)
        }
    }
    pub(crate) fn bytes(&mut self, n: u64) -> BlockResult<()> {
        self.bytes.set(
            self.bytes
                .get()
                .checked_add(n)
                .ok_or_else(|| self.err("bytes"))?,
        );
        if self.bytes.get() > self.max_bytes {
            Err(self.err("bytes"))
        } else {
            Ok(())
        }
    }
    #[cfg(test)]
    pub(crate) fn used_bytes(&self) -> u64 {
        self.bytes.get()
    }
    pub(crate) fn cells(&mut self, n: u64) -> BlockResult<()> {
        self.cells.set(
            self.cells
                .get()
                .checked_add(n)
                .ok_or_else(|| self.err("cells"))?,
        );
        if self.cells.get() > self.max_cells {
            Err(self.err("cells"))
        } else {
            Ok(())
        }
    }
}
// Every physical row access is charged exactly once, by the budgeted reader that performs it
// (key plus value bytes), so these helpers and `Meter` add no second charge for one read.
fn get<'a, R: owned::Read>(r: &'a R, t: Table, key: &[u8]) -> BlockResult<Option<R::Bytes<'a>>> {
    Ok(r.block(t, key)?)
}
fn own<'a, R: owned::Read>(
    r: &'a R,
    t: owned::Table,
    key: &[u8],
) -> BlockResult<Option<R::Bytes<'a>>> {
    Ok(r.owned(t, key)?)
}
fn identity(
    r: &impl EntityRead,
    scope: GenerationKey,
    expected: GenerationIdentity,
) -> BlockResult<()> {
    match r.identity(scope)? {
        Some(actual) if actual == expected => Ok(()),
        Some(_) => Err(BlockDurabilityError::GenerationMismatch),
        None => Err(StorageError::MissingGenerationIdentity { key: Some(scope) }.into()),
    }
}
pub(crate) fn resolve(
    key: ChunkKey,
    p: raw::PairRef<'_>,
    identity: GenerationIdentity,
    b: &mut Budget,
) -> BlockResult<Chunk> {
    crate::record::validate_chunk_position(key.pos())?;
    b.cells(98_304)?;
    let mut chunk = if let Some(bytes) = p.full {
        b.bytes(bytes.len() as u64)?;
        crate::codec::decode_chunk_record(bytes)?.into_chunk()
    } else {
        let generator = FlatWorldGenerator::with_seed(identity.seed());
        if generator.identity() != identity {
            return Err(BlockDurabilityError::GenerationMismatch);
        }
        generator.generate(key.pos())
    };
    if chunk.pos() != key.pos() {
        return Err(StorageError::ChunkPositionMismatch {
            expected: key.pos(),
            actual: chunk.pos(),
        }
        .into());
    }
    if let Some(bytes) = p.overlay {
        b.bytes(bytes.len() as u64)?;
        b.cells(98_304)?;
        let overlay = crate::codec::decode_chunk_overlay_record(bytes)?;
        if overlay.pos() != key.pos() {
            return Err(StorageError::ChunkPositionMismatch {
                expected: key.pos(),
                actual: overlay.pos(),
            }
            .into());
        }
        if p.full.is_some() && overlay.section_count() != 0 {
            b.cells(chunk.block_entity_count() as u64)?;
            let positions: Vec<_> = chunk.block_entities().map(|(p, _)| p).collect();
            for p in positions {
                b.cells(1)?;
                chunk.remove_block_entity(p);
            }
        }
        // Checked apply clones and visits all carried cells; reserve before that call.
        b.cells(98_304)?;
        overlay.try_apply_to_chunk(&mut chunk)?;
    }
    Ok(chunk)
}
/// Exact length of an entity's canonical World encoding, derived borrowed before encoding.
/// The codec writes `0x7f ++ schema(u16) ++ tag`, then a sign's kind, waxed flag and two faces
/// (`len(u16) ++ color`, glow flag, four `len(u16) ++ line`), or a chest's slots, each
/// `len(u32) ++ item` sized by the item codec's own non-allocating preflight. The result is
/// capped at the 16 KiB form bound, which the encoder checks before every append. An entity
/// kind unknown here or a stack that preflight refuses (the encoder then fails too) is
/// charged that bound. A refused encoding is charged its would-be length; the encoder never
/// writes more than that before it refuses.
pub(crate) fn entity_len(entity: &BlockEntity) -> u64 {
    let face =
        |f: &SignFace| 3 + f.color().len() + f.lines().iter().map(|l| 2 + l.len()).sum::<usize>();
    let len = match entity {
        BlockEntity::Sign(sign) => Some(6 + face(sign.front()) + face(sign.back())),
        BlockEntity::Chest(chest) => chest.slots().iter().try_fold(4usize, |n, stack| {
            Some(n + 4 + persisted_stack_size(stack).ok()?)
        }),
        _ => None,
    };
    len.map_or(MAX_BLOCK_ENTITY_PAYLOAD_LEN, |n| {
        n.min(MAX_BLOCK_ENTITY_PAYLOAD_LEN)
    }) as u64
}
fn entity_bytes(chunk: &Chunk, pos: BlockPos, b: &mut Budget) -> BlockResult<Option<Vec<u8>>> {
    b.cells(1)?;
    chunk
        .block_entity(pos)
        .map(|e| {
            b.bytes(entity_len(e))?;
            let mut bytes = Vec::new();
            try_encode_block_entity(e, &mut bytes)
                .map_err(|_| raw::bad("endpoint", "unrepresentable entity"))?;
            Ok(bytes)
        })
        .transpose()
}
fn entities(chunk: &Chunk, b: &mut Budget) -> BlockResult<Vec<(BlockPos, Vec<u8>)>> {
    let mut out = Vec::with_capacity(chunk.block_entity_count());
    for (p, e) in chunk.block_entities() {
        b.cells(1)?;
        b.bytes(entity_len(e))?;
        let mut bytes = Vec::new();
        try_encode_block_entity(e, &mut bytes)
            .map_err(|_| raw::bad("endpoint", "unrepresentable entity"))?;
        out.push((p, bytes));
    }
    out.sort_by_key(|(p, _)| (p.x(), p.y(), p.z()));
    Ok(out)
}
/// Complete proofs compare every cell and canonical entity byte, independently of proposal representation.
pub(crate) fn endpoints(q: &raw::Q<'_>, b: &mut Budget) -> BlockResult<()> {
    union(q, true, b)?;
    for e in &q.entries {
        column(q, e, false, b)?;
    }
    Ok(())
}
/// Union, coverage and per-effect validity of the whole operation; no cell work.
/// `effect_blobs` checks every effect entity here; otherwise each column checks its own.
fn union(q: &raw::Q<'_>, effect_blobs: bool, b: &mut Budget) -> BlockResult<()> {
    if let Some(bytes) = q.before_player {
        b.bytes(bytes.len() as u64)?;
        crate::codec::decode_player_record(bytes)?;
    }
    if let Some(bytes) = q.after_player {
        b.bytes(bytes.len() as u64)?;
        crate::codec::decode_player_record(bytes)?;
    }
    let mut union = BTreeSet::new();
    for e in &q.entries {
        if q.kind == 3 || e.action != 0 {
            union.insert(e.key);
        }
    }
    for (ordinal, p) in q.guards.iter().enumerate() {
        crate::record::validate_entity_position(p.to_chunk_pos(), *p)?;
        let k = ChunkKey::new(q.scope.world(), q.scope.dimension(), p.to_chunk_pos());
        if !q.entries.iter().any(|e| e.key == k) {
            return Err(BlockDurabilityError::InvalidEffect {
                ordinal,
                reason: "uncovered guard",
            });
        }
        union.insert(k);
    }
    for (ordinal, e) in q.effects.iter().enumerate() {
        crate::record::validate_entity_position(e.pos.to_chunk_pos(), e.pos)?;
        if effect_blobs {
            effect_entities(e, b)?;
        }
        if e.old == e.new && e.old_entity == e.new_entity {
            return Err(BlockDurabilityError::InvalidEffect {
                ordinal,
                reason: "unchanged effect",
            });
        }
        let k = ChunkKey::new(q.scope.world(), q.scope.dimension(), e.pos.to_chunk_pos());
        if !q.entries.iter().any(|e| e.key == k) {
            return Err(BlockDurabilityError::InvalidEffect {
                ordinal,
                reason: "uncovered effect",
            });
        }
        union.insert(k);
    }
    if union.len() != q.entries.len() {
        return Err(raw::bad("union", "extra Keep-only snapshot chunk"));
    }
    Ok(())
}
fn effect_entities(e: &raw::Effect<'_>, b: &mut Budget) -> BlockResult<()> {
    for bytes in [e.old_entity, e.new_entity].into_iter().flatten() {
        // The decode inspects the stored bytes; the canonical re-encoding is charged its own
        // exact length (equal to the input's for a canonical entity).
        b.bytes(bytes.len() as u64)?;
        raw::canonical_entity_charged(bytes, |n| b.bytes(n))?;
    }
    Ok(())
}
/// One column's endpoint proof; returns its resolved postimage.
// Chain execution and the whole-column comparison share one budget and stay adjacent.
fn column(
    q: &raw::Q<'_>,
    e: &raw::Entry<'_>,
    effect_blobs: bool,
    b: &mut Budget,
) -> BlockResult<Chunk> {
    let mut before = resolve(e.key, e.before, q.identity, b)?;
    let after = resolve(e.key, e.after, q.identity, b)?;
    if q.kind == 3 {
        return Ok(after);
    }
    for p in q.guards.iter().filter(|p| p.to_chunk_pos() == e.key.pos()) {
        b.cells(1)?;
        if before.get_block(*p).is_none() {
            return Err(raw::bad("guard", "invalid cell"));
        }
        entity_bytes(&before, *p, b)?;
    }
    for (ordinal, f) in q
        .effects
        .iter()
        .enumerate()
        .filter(|(_, f)| f.pos.to_chunk_pos() == e.key.pos())
    {
        if effect_blobs {
            effect_entities(f, b)?;
        }
        b.cells(1)?;
        if before.get_block(f.pos).map(BlockStateId::as_u32) != Some(f.old)
            || entity_bytes(&before, f.pos, b)?.as_deref() != f.old_entity
        {
            return Err(BlockDurabilityError::InvalidEffect {
                ordinal,
                reason: "old cell/entity disagrees with preceding chain",
            });
        }
        b.cells(1)?;
        before
            .set_block(f.pos, BlockStateId::new(f.new))
            .map_err(|_| raw::bad("effect", "invalid position"))?;
        b.cells(1)?;
        if let Some(bytes) = f.new_entity {
            // Applying the effect decodes its new entity once more; charge that inspection.
            b.bytes(bytes.len() as u64)?;
            let entity =
                decode_block_entity(bytes).map_err(|_| raw::bad("effect", "invalid entity"))?;
            before
                .set_block_entity(f.pos, entity)
                .map_err(|_| raw::bad("effect", "entity map capacity"))?;
        } else {
            before.remove_block_entity(f.pos);
        }
    }
    let origin = e.key.pos().origin_block(-64);
    for y in -64..320 {
        for z in 0..16 {
            for x in 0..16 {
                let p = BlockPos::new(origin.x() + x, y, origin.z() + z);
                b.cells(2)?;
                if before.get_block(p) != after.get_block(p) {
                    return Err(BlockDurabilityError::IncompleteDelta { key: e.key });
                }
            }
        }
    }
    if entities(&before, b)? != entities(&after, b)? {
        return Err(BlockDurabilityError::IncompleteDelta { key: e.key });
    }
    Ok(after)
}
fn next(
    before: OwnershipStamp,
    present: bool,
    changed: bool,
    resource: &'static str,
    key: Option<ChunkKey>,
) -> BlockResult<OwnershipStamp> {
    if !changed {
        return Ok(before);
    }
    let revision = before
        .revision()
        .map_or(0, super::super::ownership::OwnershipRevision::get)
        .checked_add(1)
        .ok_or(BlockDurabilityError::RevisionExhausted { resource, key })?;
    Ok(OwnershipStamp::checked(
        if present { 1 } else { 2 },
        revision,
    )?)
}
pub(crate) fn after_stamps(
    q: &raw::Q<'_>,
) -> BlockResult<(OwnershipStamp, Vec<(ChunkKey, OwnershipStamp)>)> {
    for e in &q.entries {
        if e.action == 2 && !e.before_stamp.is_live() {
            return Err(BlockDurabilityError::AlreadyAbsent { key: e.key });
        }
    }
    let player = next(
        q.player_stamp,
        q.player.is_some(),
        q.before_player != q.after_player,
        "Player",
        None,
    )?;
    let mut chunks = Vec::with_capacity(q.entries.len());
    for e in &q.entries {
        if e.action == 2 && !e.before_stamp.is_live() {
            return Err(BlockDurabilityError::AlreadyAbsent { key: e.key });
        }
        let changed = e.before != e.after
            || q.effects
                .iter()
                .any(|f| f.pos.to_chunk_pos() == e.key.pos());
        chunks.push((
            e.key,
            next(
                e.before_stamp,
                e.after.present(),
                changed,
                "Chunk",
                Some(e.key),
            )?,
        ));
    }
    Ok((player, chunks))
}
/// A bounded per-call cache is an optimization, not a retained-receipt cap.
fn remember<T: Ord>(set: &std::cell::RefCell<BTreeSet<T>>, value: T) {
    let mut set = set.borrow_mut();
    if set.len() >= 128 {
        set.clear();
    }
    set.insert(value);
}
/// Complete FCBR relations plus the endpoint proof of every column (lookup, replay, reopen).
pub(crate) fn receipt_relations(
    r: &impl owned::Read,
    receipt: &raw::Frame<'_>,
    b: &mut Budget,
) -> BlockResult<()> {
    if b.proved.borrow().contains(&receipt.query.id) {
        return Ok(());
    }
    b.bytes(receipt.request_bytes().len() as u64)?;
    let q = raw::q(receipt.request_bytes())?;
    proved(r, receipt, &q, b)
}
fn proved(
    r: &impl owned::Read,
    receipt: &raw::Frame<'_>,
    q: &raw::Q<'_>,
    b: &mut Budget,
) -> BlockResult<()> {
    if b.proved.borrow().contains(&receipt.query.id) {
        return Ok(());
    }
    validated(r, receipt, q, b)?;
    for (index, e) in q.entries.iter().enumerate() {
        if !b.columns.borrow().contains(&(receipt.query.id, index)) {
            column(q, e, true, b)?;
        }
    }
    remember(&b.proved, receipt.query.id);
    Ok(())
}
/// Encountered history without endpoint re-derivation: union, outcome stamps and the
/// retained Player/Chunk/journal relations. Runtime readers add only touched columns' proofs.
// Short names mirror the FCBQ/FCBR field layout within this one relation check.
#[allow(clippy::many_single_char_names)]
fn validated(
    r: &impl owned::Read,
    receipt: &raw::Frame<'_>,
    q: &raw::Q<'_>,
    b: &mut Budget,
) -> BlockResult<()> {
    if b.validated.borrow().contains(&receipt.query.id) {
        return Ok(());
    }
    identity(r, q.scope, q.identity)?;
    union(q, false, b)?;
    let (player, chunks) = after_stamps(q)?;
    if player != receipt.player || chunks != receipt.chunks {
        return Err(raw::bad("FCBR", "outcome transition mismatch"));
    }
    if let Some(id) = q.player {
        let key = crate::codec::player_key_bytes(id);
        let h = own(r, owned::Table::PlayerHeader, &key)?
            .ok_or_else(|| raw::bad("FCBR", "missing Player history"))?;
        let h = owner_raw::player_header(h.as_ref())?;
        historical(h.stamp, player)?;
        if h.scope != q.scope {
            return Err(raw::bad("FCBR", "Player history scope"));
        }
        if h.stamp == player {
            let bytes = own(r, owned::Table::Player, &key)?;
            if bytes.as_ref().map(std::convert::AsRef::as_ref) != q.after_player {
                return Err(raw::bad("FCBR", "equal Player history postimage"));
            }
        }
    }
    for (e, (_, after)) in q.entries.iter().zip(&receipt.chunks) {
        let key = crate::codec::chunk_key_bytes(e.key);
        let h = get(r, Table::Header, &key)?;
        if let Some(bytes) = h {
            let h = raw::header(bytes.as_ref())?;
            historical(h.stamp, *after)?;
            if h.stamp == *after {
                let f = get(r, Table::Full, &key)?;
                let o = get(r, Table::Overlay, &key)?;
                if (raw::PairRef {
                    full: f.as_ref().map(std::convert::AsRef::as_ref),
                    overlay: o.as_ref().map(std::convert::AsRef::as_ref),
                }) != e.after
                {
                    return Err(raw::bad("FCBR", "equal Chunk history postimage"));
                }
            }
        } else {
            if !after.is_fresh() {
                return Err(raw::bad("FCBR", "missing Chunk history"));
            }
            if get(r, Table::Full, &key)?.is_some() || get(r, Table::Overlay, &key)?.is_some() {
                return Err(raw::bad("FCBR", "Fresh chunk has payload"));
            }
        }
    }
    for (ordinal, e) in q.effects.iter().enumerate() {
        let sequence = receipt
            .first
            .ok_or_else(|| raw::bad("FCBR", "missing journal range"))?
            .checked_add(ordinal as u64)
            .ok_or(BlockDurabilityError::SequenceExhausted)?;
        let bytes = get(r, Table::Journal, &sequence.to_be_bytes())?
            .ok_or_else(|| raw::bad("FCBR", "missing journal row"))?;
        let expected = raw::journal(q.scope, q.id, ordinal as u16, sequence, e.bytes);
        b.bytes(expected.len() as u64)?;
        if bytes.as_ref() != expected {
            return Err(raw::bad("FCBR", "journal exact ordinal disagreement"));
        }
    }
    remember(&b.validated, receipt.query.id);
    Ok(())
}
fn historical(current: OwnershipStamp, after: OwnershipStamp) -> BlockResult<()> {
    let a = current
        .revision()
        .map_or(0, super::super::ownership::OwnershipRevision::get);
    let b = after
        .revision()
        .map_or(0, super::super::ownership::OwnershipRevision::get);
    if a < b || a == b && current != after {
        Err(raw::bad(
            "FCBR",
            "retained revision does not dominate outcome",
        ))
    } else {
        Ok(())
    }
}
/// Raw history reads here avoid recursively walking other last receipts.
pub(crate) fn player_last(
    r: &impl owned::Read,
    h: owner_raw::Header,
    key: &[u8],
    identity: GenerationIdentity,
    bytes: &[u8],
) -> crate::OwnershipResult<()> {
    let config = r.block_config().ok_or(crate::OwnershipError::Unsupported)?;
    let mut b = r.block_budget().unwrap_or_else(|| Budget::runtime(config));
    let (receipt, q) =
        raw::frame_q(&h.receipt.to_bytes(), bytes, &mut b).map_err(ownership_adapter)?;
    if q.player.is_none_or(|id| id.as_uuid().as_bytes() != key)
        || q.scope != h.scope
        || q.identity != identity
        || receipt.player != h.stamp
        || q.player_stamp == receipt.player
    {
        return Err(owner_raw::bad(
            "FCBR",
            "Player last changing receipt mismatch",
        ));
    }
    // Only the Player part of this receipt is touched here; startup re-derives every FCBR.
    // The relations reuse the one charged parse of Q above.
    validated(r, &receipt, &q, &mut b).map_err(ownership_adapter)
}
/// Per-call cache of the distinct last receipts that a call's at most eight columns share.
/// `rows` is an append-only arena of borrowed stored rows; each row is read, digested and
/// parsed once per call. It holds only borrowed rows and their parsed frames and is dropped
/// with the call, so no receipt state survives across calls.
pub(crate) type Rows<B> = [std::cell::OnceCell<B>; MAX_BLOCK_CHUNKS];
pub(crate) fn rows<B>() -> Rows<B> {
    std::array::from_fn(|_| std::cell::OnceCell::new())
}
pub(crate) struct Lasts<'c, B> {
    rows: &'c Rows<B>,
    parsed: Vec<(raw::Frame<'c>, raw::Q<'c>)>,
}
impl<'c, B: AsRef<[u8]>> Lasts<'c, B> {
    pub(crate) fn new(rows: &'c Rows<B>) -> Self {
        Self {
            rows,
            parsed: Vec::with_capacity(MAX_BLOCK_CHUNKS),
        }
    }
    /// Index of `id`'s parsed frame; a miss reads, digests and parses the row exactly once.
    pub(crate) fn load(
        &mut self,
        id: OwnershipTransferId,
        b: &mut Budget,
        read: impl FnOnce() -> BlockResult<Option<B>>,
    ) -> BlockResult<usize> {
        if let Some(at) = self.parsed.iter().position(|(f, _)| f.query.id == id) {
            return Ok(at);
        }
        // A call reads at most MAX_BLOCK_CHUNKS columns, each adding at most one receipt.
        let slot = self
            .rows
            .get(self.parsed.len())
            .ok_or_else(|| raw::bad(raw::CHUNK_OWNER, "more last receipts than columns"))?;
        let row = read()?.ok_or_else(|| raw::bad(raw::CHUNK_OWNER, "last receipt absent"))?;
        if slot.set(row).is_err() {
            return Err(raw::bad(raw::CHUNK_OWNER, "receipt cache slot reused"));
        }
        let bytes: &'c [u8] = slot
            .get()
            .ok_or_else(|| raw::bad(raw::CHUNK_OWNER, "receipt cache slot empty"))?
            .as_ref();
        self.parsed.push(raw::frame_q(&id.to_bytes(), bytes, b)?);
        Ok(self.parsed.len() - 1)
    }
}
/// Validates the column's encountered history: its last receipt's relations once per call,
/// plus the endpoint proof of this column's own entry, whose resolved postimage is the
/// current view. Startup instead re-derives the whole receipt, as Reopen requires.
#[allow(clippy::many_single_char_names)] // Grammar-local names, see validated.
fn inspect_chunk<'r, R: owned::Read, T>(
    r: &'r R,
    key: ChunkKey,
    identity: GenerationIdentity,
    b: &mut Budget,
    lasts: &mut Lasts<'_, R::Bytes<'r>>,
    read: impl FnOnce(OwnershipStamp, raw::PairRef<'_>, Option<Chunk>, &mut Budget) -> BlockResult<T>,
) -> BlockResult<T> {
    let k = crate::codec::chunk_key_bytes(key);
    let f = get(r, Table::Full, &k)?;
    let o = get(r, Table::Overlay, &k)?;
    let p = raw::PairRef {
        full: f.as_ref().map(std::convert::AsRef::as_ref),
        overlay: o.as_ref().map(std::convert::AsRef::as_ref),
    };
    let h = get(r, Table::Header, &k)?;
    let Some(h) = h else {
        if p.present() {
            return Err(raw::bad(raw::CHUNK_OWNER, "payload without history"));
        }
        return read(OwnershipStamp::FRESH, p, None, b);
    };
    let h = raw::header(h.as_ref())?;
    if h.stamp.is_live() != p.present() {
        return Err(raw::bad(raw::CHUNK_OWNER, "stamp/pair presence"));
    }
    let at = lasts.load(h.id, b, || own(r, owned::Table::Receipt, &h.id.to_bytes()))?;
    let (receipt, q) = &lasts.parsed[at];
    let index = q
        .entries
        .iter()
        .position(|e| e.key == key)
        .ok_or_else(|| raw::bad(raw::CHUNK_OWNER, "last receipt column missing"))?;
    let e = &q.entries[index];
    if q.identity != identity
        || receipt.chunks[index].1 != h.stamp
        || e.before_stamp == h.stamp
        || e.after != p
    {
        return Err(raw::bad(
            raw::CHUNK_OWNER,
            "last changing receipt/postimage mismatch",
        ));
    }
    let id = receipt.query.id;
    let view = if b.startup {
        proved(r, receipt, q, b)?;
        None
    } else {
        validated(r, receipt, q, b)?;
        if b.proved.borrow().contains(&id) || b.columns.borrow().contains(&(id, index)) {
            None
        } else {
            let view = column(q, e, true, b)?;
            remember(&b.columns, (id, index));
            Some(view)
        }
    };
    read(h.stamp, p, view, b)
}
/// One column's stamp and owned pair, for single-column loads and the startup scan.
pub(crate) fn chunk_state(
    r: &impl owned::Read,
    key: ChunkKey,
    identity: GenerationIdentity,
    b: &mut Budget,
) -> BlockResult<(OwnershipStamp, Pair)> {
    let rows = rows();
    inspect_chunk(
        r,
        key,
        identity,
        b,
        &mut Lasts::new(&rows),
        |stamp, p, _, b| {
            // The owned pair is a real copy for the caller, charged before it is made.
            b.bytes(
                p.full.map_or(0, <[u8]>::len) as u64 + p.overlay.map_or(0, <[u8]>::len) as u64,
            )?;
            Ok((stamp, p.owned()))
        },
    )
}

// Only one bounded temporary Chunk and one <=16KiB entity encoding are used here.
// Compute exact output size before transferring/copying the resolved view into output.
fn view_encoded_len(chunk: &Chunk, b: &mut Budget) -> BlockResult<u64> {
    b.cells(98_304)?;
    let mut len = 49u64;
    for section in chunk.sections() {
        if !section.is_empty() {
            len += 4096 * 4;
        }
    }
    for (pos, entity) in chunk.block_entities() {
        b.cells(1)?;
        b.bytes(entity_len(entity))?;
        let mut encoded = Vec::new();
        try_encode_block_entity(entity, &mut encoded)
            .map_err(|source| StorageError::BlockEntityCodec { pos, source })?;
        len = len
            .checked_add(16 + encoded.len() as u64)
            .ok_or(BlockDurabilityError::ValidationCapacity("output"))?;
    }
    crate::record::check_record_size(len as usize, crate::MAX_CHUNK_RECORD_LEN)?;
    Ok(len)
}
pub(crate) fn snapshot(
    r: &impl owned::Read,
    owner: &Arc<()>,
    spec: BlockSnapshotSpec,
    config: BlockDurabilityConfig,
    b: &mut Budget,
) -> BlockResult<OwnedBlockSnapshot> {
    identity(r, spec.scope, spec.identity)?;
    r.counts()?;
    r.block_counts()?;
    let mut output = 35u64;
    let mut player_stamp = OwnershipStamp::FRESH;
    let mut player_raw = None;
    let mut player_record = None;
    output_admit(output, config)?;
    if let Some(id) = spec.player {
        player_stamp = owned::player_state(
            r,
            crate::ScopedPlayerKey::new(spec.scope, id),
            spec.identity,
        )?
        .stamp;
        let bytes = own(r, owned::Table::Player, &crate::codec::player_key_bytes(id))?;
        let len = bytes.as_ref().map_or(0, |v| v.as_ref().len()) as u64;
        // Raw authoritative envelope and decoded opaque Player output both survive; both
        // copies are snapshot output, and decoding inspects the row once more.
        output = output
            .checked_add(55 + 2 * len)
            .ok_or(BlockDurabilityError::ValidationCapacity("output"))?;
        output_admit(output, config)?;
        b.bytes(len)?;
        player_record = bytes
            .as_ref()
            .map(|v| crate::codec::decode_player_record(v.as_ref()))
            .transpose()?;
        player_raw = bytes.map(|v| Arc::from(v.as_ref()));
    }
    let mut columns = Vec::with_capacity(spec.keys.len());
    let rows = rows();
    let mut lasts = Lasts::new(&rows);
    for key in &spec.keys {
        let column = inspect_chunk(
            r,
            *key,
            spec.identity,
            b,
            &mut lasts,
            |stamp, p, proved, b| {
                let raw_len =
                    p.full.map_or(0, <[u8]>::len) as u64 + p.overlay.map_or(0, <[u8]>::len) as u64;
                output = output
                    .checked_add(47 + raw_len)
                    .ok_or(BlockDurabilityError::ValidationCapacity("output"))?;
                output_admit(output, config)?;
                // A proved column's resolved postimage is exactly this pair's composition.
                let view = match proved {
                    Some(view) => view,
                    None => resolve(*key, p, spec.identity, b)?,
                };
                output = output
                    .checked_add(view_encoded_len(&view, b)?)
                    .ok_or(BlockDurabilityError::ValidationCapacity("output"))?;
                output_admit(output, config)?;
                // The raw pair copy is snapshot output, already admitted above.
                Ok(OwnedBlockColumn {
                    key: *key,
                    stamp,
                    pair: p.owned(),
                    view,
                })
            },
        )?;
        columns.push(column);
    }
    Ok(OwnedBlockSnapshot {
        spec,
        owner: Arc::downgrade(owner),
        player_stamp,
        player_raw,
        player_record,
        columns,
        request_limit: config.capacity.request,
    })
}
fn output_admit(n: u64, config: BlockDurabilityConfig) -> BlockResult<()> {
    if n > config.limits.snapshot {
        Err(BlockDurabilityError::ByteCapacity {
            resource: "snapshot",
            len: n,
            max: config.limits.snapshot,
        })
    } else {
        Ok(())
    }
}
pub(crate) fn lookup(
    r: &impl owned::Read,
    query: BlockReceiptQuery,
    b: &mut Budget,
) -> BlockResult<Option<BlockReceipt>> {
    r.counts()?;
    r.block_counts()?;
    let Some(bytes) = own(r, owned::Table::Receipt, &query.id.to_bytes())? else {
        return Ok(None);
    };
    if bytes.as_ref().starts_with(b"FCTR") {
        let receipt = crate::ownership::raw::receipt(&query.id.to_bytes(), bytes.as_ref())?;
        owned::receipt_relations(r, &receipt)?;
        return Err(BlockDurabilityError::ReceiptKindMismatch { id: query.id });
    }
    let frame = raw::frame(&query.id.to_bytes(), bytes.as_ref(), b)?;
    receipt_relations(r, &frame, b)?;
    if query != frame.query {
        return Err(BlockDurabilityError::ReceiptCollision { id: query.id });
    }
    Ok(Some(frame.owned(b)?))
}
/// Outer `None` leaves a row untouched; inner `None` removes it.
#[allow(clippy::option_option)]
pub(crate) struct ChunkChange {
    pub(crate) key: ChunkKey,
    pub(crate) full: Option<Option<Vec<u8>>>,
    pub(crate) overlay: Option<Option<Vec<u8>>>,
    pub(crate) header: [u8; 31],
}
pub(crate) struct Plan {
    pub(crate) player: Option<owned::PlayerChange>,
    pub(crate) chunks: Vec<ChunkChange>,
    pub(crate) journal: Vec<(u64, Vec<u8>)>,
    pub(crate) receipt: BlockReceipt,
    pub(crate) counts: Counts,
    pub(crate) receipts: u64,
    pub(crate) revisions: u64,
}
pub(crate) enum Prepared {
    Replay(BlockReceipt),
    Commit(Plan),
}
fn capacity(current: u64, additional: u64, max: u64, resource: &'static str) -> BlockResult<u64> {
    let n = current
        .checked_add(additional)
        .ok_or(BlockDurabilityError::HistoryCapacity { resource, max })?;
    if n > max {
        Err(BlockDurabilityError::HistoryCapacity { resource, max })
    } else {
        Ok(n)
    }
}
/// The retained count is the next ID; MAX rows end at ID MAX-1.
pub(crate) fn allocate_sequences(
    count: u64,
    watermark: Option<u64>,
    effects: u16,
) -> BlockResult<(Option<u64>, u64)> {
    if watermark != count.checked_sub(1) {
        return Err(raw::bad("meta", "scoped count/watermark"));
    }
    let after = count
        .checked_add(u64::from(effects))
        .ok_or(BlockDurabilityError::SequenceExhausted)?;
    Ok(((effects != 0).then_some(count), after))
}
// The documented refusal order is easier to audit as one linear preflight.
#[allow(clippy::too_many_lines)]
pub(crate) fn prepare(
    r: &impl owned::Read,
    owner: &Arc<()>,
    request: &Request,
    config: BlockDurabilityConfig,
    b: &mut Budget,
) -> BlockResult<Prepared> {
    let (receipt_count, revision_count) = r.counts()?;
    let mut counts = r.block_counts()?;
    b.bytes(request.q.len() as u64)?;
    let q = raw::q(&request.q)?;
    if let Some(bytes) = own(r, owned::Table::Receipt, &q.id.to_bytes())? {
        if bytes.as_ref().starts_with(b"FCTR") {
            let receipt = owner_raw::receipt(&q.id.to_bytes(), bytes.as_ref())?;
            owned::receipt_relations(r, &receipt)?;
            return Err(BlockDurabilityError::ReceiptCollision { id: q.id });
        }
        let frame = raw::frame(&q.id.to_bytes(), bytes.as_ref(), b)?;
        receipt_relations(r, &frame, b)?;
        if frame.request_bytes() != request.q.as_ref() || frame.query != request.query {
            return Err(BlockDurabilityError::ReceiptCollision { id: q.id });
        }
        return Ok(Prepared::Replay(frame.owned(b)?));
    }
    if !Weak::ptr_eq(&request.snapshot.owner, &Arc::downgrade(owner))
        || request.snapshot.owner.upgrade().is_none()
    {
        return Err(BlockDurabilityError::BackendOwnerMismatch);
    }
    identity(r, q.scope, q.identity)?;
    if let Some(id) = q.player {
        let state = owned::player_state(r, crate::ScopedPlayerKey::new(q.scope, id), q.identity)?;
        if state.stamp != q.player_stamp {
            return Err(BlockDurabilityError::CasConflict {
                resource: "Player",
                key: None,
                expected: q.player_stamp,
                actual: state.stamp,
            });
        }
        let bytes = own(r, owned::Table::Player, &crate::codec::player_key_bytes(id))?;
        if bytes.as_ref().map(std::convert::AsRef::as_ref) != q.before_player {
            return Err(BlockDurabilityError::BeforeImageMismatch {
                resource: "Player",
                key: None,
            });
        }
    }
    // The union's columns share one per-call receipt cache; stored originals are compared
    // borrowed, without copying them.
    let rows = rows();
    let mut lasts = Lasts::new(&rows);
    for e in &q.entries {
        let (stamp, equal) =
            inspect_chunk(r, e.key, q.identity, b, &mut lasts, |stamp, p, _, _| {
                Ok((stamp, p == e.before))
            })?;
        if stamp != e.before_stamp {
            return Err(BlockDurabilityError::CasConflict {
                resource: "Chunk",
                key: Some(e.key),
                expected: e.before_stamp,
                actual: stamp,
            });
        }
        if !equal {
            return Err(BlockDurabilityError::BeforeImageMismatch {
                resource: "Chunk",
                key: Some(e.key),
            });
        }
    }
    endpoints(&q, b)?;
    let (player_after, chunk_after) = after_stamps(&q)?;
    let (first, journal_rows) = allocate_sequences(
        counts.journal_rows,
        counts.watermark,
        q.effects.len() as u16,
    )?;
    let mut journal = Vec::with_capacity(q.effects.len());
    let mut journal_bytes = 0u64;
    for (i, e) in q.effects.iter().enumerate() {
        let id = counts
            .journal_rows
            .checked_add(i as u64)
            .ok_or(BlockDurabilityError::SequenceExhausted)?;
        if get(r, Table::Journal, &id.to_be_bytes())?.is_some() {
            return Err(BlockDurabilityError::JournalIdCollision { id });
        }
        let bytes = raw::journal(q.scope, q.id, i as u16, id, e.bytes);
        b.bytes(bytes.len() as u64)?;
        journal_bytes = journal_bytes
            .checked_add(8 + bytes.len() as u64)
            .ok_or(BlockDurabilityError::SequenceExhausted)?;
        journal.push((id, bytes));
    }
    let receipt_bytes = raw::encode_receipt(
        &request.q,
        player_after,
        &chunk_after,
        first,
        q.effects.len() as u16,
    );
    b.bytes(receipt_bytes.len() as u64)?;
    let receipts = capacity(
        receipt_count,
        1,
        config.capacity.ownership.max_receipts(),
        "receipt rows",
    )?;
    counts.receipt_bytes = capacity(
        counts.receipt_bytes,
        16 + receipt_bytes.len() as u64,
        config.capacity.receipt_bytes,
        "receipt bytes",
    )?;
    let revisions = capacity(
        revision_count,
        0,
        config.capacity.ownership.max_revision_rows(),
        "Player/Entity rows",
    )?;
    let mut chunks = Vec::with_capacity(q.entries.len());
    let mut additional = 0;
    for (e, (_, after)) in q.entries.iter().zip(&chunk_after) {
        if *after != e.before_stamp {
            additional += u64::from(e.before_stamp.is_fresh());
            b.bytes(
                e.after.full.map_or(0, <[u8]>::len) as u64
                    + e.after.overlay.map_or(0, <[u8]>::len) as u64,
            )?;
            chunks.push(ChunkChange {
                key: e.key,
                full: (e.before.full != e.after.full).then(|| e.after.full.map(<[u8]>::to_vec)),
                overlay: (e.before.overlay != e.after.overlay)
                    .then(|| e.after.overlay.map(<[u8]>::to_vec)),
                header: raw::encode_header(raw::Header {
                    stamp: *after,
                    id: q.id,
                }),
            });
        }
    }
    counts.chunks = capacity(
        counts.chunks,
        additional,
        config.capacity.chunks,
        "Chunk rows",
    )?;
    counts.journal_rows = capacity(
        counts.journal_rows,
        q.effects.len() as u64,
        config.capacity.journal_rows,
        "journal rows",
    )?;
    counts.journal_bytes = capacity(
        counts.journal_bytes,
        journal_bytes,
        config.capacity.journal_bytes,
        "journal bytes",
    )?;
    counts.watermark = journal_rows.checked_sub(1);
    let player = if q.player_stamp == player_after {
        None
    } else {
        Some(owned::PlayerChange {
            key: crate::codec::player_key_bytes(
                q.player.ok_or(BlockDurabilityError::PlayerNotLive)?,
            ),
            record: q.after_player.map(<[u8]>::to_vec),
            header: owner_raw::Header {
                scope: q.scope,
                stamp: player_after,
                receipt: q.id,
            },
        })
    };
    let receipt = raw::receipt(&q.id.to_bytes(), &receipt_bytes, b)?;
    Ok(Prepared::Commit(Plan {
        player,
        chunks,
        journal,
        receipt,
        counts,
        receipts,
        revisions,
    }))
}

/// Runs reused ownership/UUID raw relations under the same cumulative call budget. Each row
/// access is charged once by the budgeted reader underneath; the meter adds only the fixed
/// counter-read charges, which the readers do not meter.
pub(crate) struct Meter<'a, R: owned::Read> {
    pub(crate) reader: &'a R,
    pub(crate) budget: Budget,
}
impl<R: owned::Read> EntityRead for Meter<'_, R> {
    type Bytes<'a>
        = R::Bytes<'a>
    where
        Self: 'a;
    fn get(
        &self,
        t: crate::persistent_entity::engine::RawTable,
        k: &[u8],
    ) -> Result<Option<Self::Bytes<'_>>, StorageError> {
        self.reader.get(t, k)
    }
    fn identity(&self, k: GenerationKey) -> Result<Option<GenerationIdentity>, StorageError> {
        self.reader.identity(k)
    }
    fn index_rows(
        &self,
        lower: std::ops::Bound<Vec<u8>>,
        upper: std::ops::Bound<Vec<u8>>,
    ) -> Result<crate::persistent_entity::engine::RawRows<'_, Self::Bytes<'_>>, StorageError> {
        self.reader.index_rows(lower, upper)
    }
}
impl<R: owned::Read> owned::Read for Meter<'_, R> {
    fn owned(&self, t: owned::Table, k: &[u8]) -> crate::OwnershipResult<Option<Self::Bytes<'_>>> {
        self.reader.owned(t, k)
    }
    fn counts(&self) -> crate::OwnershipResult<(u64, u64)> {
        self.budget.clone().bytes(128).map_err(ownership_adapter)?;
        self.reader.counts()
    }
    fn block_config(&self) -> Option<BlockDurabilityConfig> {
        self.reader.block_config()
    }
    fn block_budget(&self) -> Option<Budget> {
        Some(self.budget.clone())
    }
    fn block(&self, t: Table, k: &[u8]) -> crate::OwnershipResult<Option<Self::Bytes<'_>>> {
        self.reader.block(t, k)
    }
    fn block_counts(&self) -> crate::OwnershipResult<Counts> {
        self.budget.clone().bytes(256).map_err(ownership_adapter)?;
        self.reader.block_counts()
    }
}
