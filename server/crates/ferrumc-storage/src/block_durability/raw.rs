//! Exact borrowed FCBQ/FCOB/FCBL/FCBR grammars; no parser manufactures authority.
use super::{
    engine, Arc, BlockDurabilityError, BlockPos, BlockReceipt, BlockReceiptQuery, BlockResult,
    ChunkImageAction, ChunkImageChange, ChunkKey, GenerationIdentity, GenerationKey, MutationActor,
    OwnedBlockSnapshot, OwnershipStamp, OwnershipTransferId, Pair, PlayerId, PlayerRecord, Request,
    ScopedBlockCause, ScopedBlockEffect, MAX_BLOCK_CHUNKS, MAX_BLOCK_EFFECTS, MAX_BLOCK_GUARDS,
    MAX_BLOCK_REQUEST_BYTES,
};
use ferrumc_world::{decode_block_entity, try_encode_block_entity};
use sha2::{Digest, Sha256};

pub(crate) const CHUNK_OWNER: &str = "ferrumc:chunk_ownership";
pub(crate) const JOURNAL: &str = "ferrumc:scoped_mutation_log";
pub(crate) fn bad(table: &'static str, reason: &'static str) -> BlockDurabilityError {
    BlockDurabilityError::MalformedData { table, reason }
}
pub(crate) fn shape(resource: &'static str, len: usize, min: usize, max: usize) -> BlockResult<()> {
    if len < min || len > max {
        Err(BlockDurabilityError::BatchCapacity { resource, len, max })
    } else {
        Ok(())
    }
}
pub(crate) fn canonical_entity(bytes: &[u8]) -> BlockResult<()> {
    canonical_entity_charged(bytes, |_| Ok(()))
}
/// As [`canonical_entity`]; `charge` receives the exact length of the canonical re-encoding
/// before it is made, which differs from the input's length for a non-canonical input.
pub(crate) fn canonical_entity_charged(
    bytes: &[u8],
    charge: impl FnOnce(u64) -> BlockResult<()>,
) -> BlockResult<()> {
    if bytes.len() > 16 * 1024 {
        return Err(bad("effect", "entity exceeds16KiB"));
    }
    let entity = decode_block_entity(bytes).map_err(|_| bad("effect", "invalid entity"))?;
    charge(engine::entity_len(&entity))?;
    let mut encoded = Vec::new();
    try_encode_block_entity(&entity, &mut encoded)
        .map_err(|_| bad("effect", "unrepresentable entity"))?;
    if encoded != bytes {
        return Err(bad("effect", "entity not current canonical encoding"));
    }
    Ok(())
}
fn blob(out: &mut Vec<u8>, bytes: Option<&[u8]>) {
    out.push(u8::from(bytes.is_some()));
    out.extend_from_slice(&(bytes.map_or(0, <[u8]>::len) as u32).to_be_bytes());
    if let Some(b) = bytes {
        out.extend_from_slice(b);
    }
}
fn pair(out: &mut Vec<u8>, p: &Pair) {
    blob(out, p.full.as_deref());
    blob(out, p.overlay.as_deref());
}
fn pos(out: &mut Vec<u8>, p: BlockPos) {
    for n in [p.x(), p.y(), p.z()] {
        out.extend_from_slice(&n.to_be_bytes());
    }
}
pub(crate) fn effect_bytes(e: &ScopedBlockEffect) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        60 + e.old_entity.as_ref().map_or(0, |b| b.len())
            + e.new_entity.as_ref().map_or(0, |b| b.len()),
    );
    out.extend_from_slice(&e.schema.get().to_be_bytes());
    out.extend_from_slice(&e.tick.to_be_bytes());
    match e.actor {
        MutationActor::System => {
            out.push(0);
            out.extend_from_slice(&[0; 16]);
        }
        MutationActor::Player(p) => {
            out.push(1);
            out.extend_from_slice(p.as_uuid().as_bytes());
        }
    }
    out.push(match e.cause {
        ScopedBlockCause::PlayerAction => 0,
        ScopedBlockCause::Command => 1,
        ScopedBlockCause::Plugin => 2,
        ScopedBlockCause::Test => 3,
    });
    pos(&mut out, e.pos);
    out.extend_from_slice(&e.old.as_u32().to_be_bytes());
    out.extend_from_slice(&e.new.as_u32().to_be_bytes());
    blob(&mut out, e.old_entity.as_deref());
    blob(&mut out, e.new_entity.as_deref());
    out
}
pub(crate) fn digest(q: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"FerrumC block operation v1\0");
    h.update(q);
    h.finalize().into()
}
// One canonical encoder keeps hard admission and every Q field order adjacent.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(crate) fn request(
    id: OwnershipTransferId,
    kind: u8,
    snapshot: OwnedBlockSnapshot,
    player: Option<&PlayerRecord>,
    mut images: Vec<ChunkImageChange>,
    mut guards: Vec<BlockPos>,
    effects: Vec<ScopedBlockEffect>,
) -> BlockResult<Request> {
    shape("chunks", images.len(), 1, MAX_BLOCK_CHUNKS)?;
    shape("guards", guards.len(), 0, MAX_BLOCK_GUARDS)?;
    shape("effects", effects.len(), 0, MAX_BLOCK_EFFECTS)?;
    if (kind == 1) != (snapshot.spec.player.is_some() && player.is_some())
        || (kind != 1 && (snapshot.spec.player.is_some() || player.is_some()))
    {
        return Err(BlockDurabilityError::ScopeMismatch);
    }
    if kind == 1 && !snapshot.player_stamp.is_live() {
        return Err(BlockDurabilityError::PlayerNotLive);
    }
    images.sort_by_key(|c| c.key);
    guards.sort_by_key(|p| (p.x(), p.y(), p.z()));
    if images.len() != snapshot.columns.len() {
        return Err(bad("request", "proposal union mismatch"));
    }
    let limit = snapshot.request_limit;
    let mut len = 98usize + 46 * images.len() + 12 * guards.len() + 64 * effects.len();
    let mut add = |n: usize| -> BlockResult<()> {
        len = len
            .checked_add(n)
            .ok_or(BlockDurabilityError::ByteCapacity {
                resource: "request",
                len: u64::MAX,
                max: limit as u64,
            })?;
        if len > limit {
            return Err(BlockDurabilityError::ByteCapacity {
                resource: "request",
                len: len as u64,
                max: limit as u64,
            });
        }
        Ok(())
    };
    add(snapshot.player_raw.as_ref().map_or(0, |b| b.len()))?;
    add(player.map_or(0, |r| 5 + r.data().len()))?;
    for (c, b) in images.iter().zip(&snapshot.columns) {
        if c.key != b.key {
            return Err(bad("request", "proposal keys differ from snapshot"));
        }
        add(b.pair.len())?;
        add(match &c.action {
            ChunkImageAction::Keep => b.pair.len(),
            ChunkImageAction::Replace(i) => i.0.len(),
            ChunkImageAction::Erase => 0,
        })?;
    }
    for (i, p) in guards.iter().enumerate() {
        if i > 0 && guards[i - 1] == *p {
            return Err(BlockDurabilityError::InvalidEffect {
                ordinal: i,
                reason: "duplicate guard",
            });
        }
        crate::record::validate_entity_position(p.to_chunk_pos(), *p)?;
    }
    for e in &effects {
        if e.scope != snapshot.spec.scope {
            return Err(BlockDurabilityError::ScopeMismatch);
        }
        add(e.old_entity.as_ref().map_or(0, |b| b.len()))?;
        add(e.new_entity.as_ref().map_or(0, |b| b.len()))?;
    }
    let mut q = Vec::with_capacity(len);
    q.extend_from_slice(b"FCBQ\0\x01");
    q.push(kind);
    q.extend_from_slice(&id.to_bytes());
    q.extend_from_slice(&snapshot.spec.scope.encode());
    q.extend_from_slice(&snapshot.spec.identity.encode());
    q.extend_from_slice(
        &snapshot
            .spec
            .player
            .map_or([0; 16], |p| *p.as_uuid().as_bytes()),
    );
    q.extend_from_slice(&snapshot.player_stamp.encode());
    blob(&mut q, snapshot.player_raw.as_deref());
    let p = player.map(crate::codec::encode_player_record);
    blob(&mut q, p.as_deref());
    q.push(images.len() as u8);
    for (c, b) in images.iter().zip(&snapshot.columns) {
        q.extend_from_slice(&crate::codec::chunk_key_bytes(c.key));
        q.extend_from_slice(&b.stamp.encode());
        pair(&mut q, &b.pair);
        match &c.action {
            ChunkImageAction::Keep => {
                q.push(0);
                pair(&mut q, &b.pair);
            }
            ChunkImageAction::Replace(i) => {
                q.push(1);
                pair(&mut q, &i.0);
            }
            ChunkImageAction::Erase => {
                q.push(2);
                pair(&mut q, &Pair::default());
            }
        }
    }
    q.extend_from_slice(&(guards.len() as u16).to_be_bytes());
    for p in guards {
        pos(&mut q, p);
    }
    q.extend_from_slice(&(effects.len() as u16).to_be_bytes());
    for e in effects {
        let b = effect_bytes(&e);
        q.extend_from_slice(&(b.len() as u32).to_be_bytes());
        q.extend_from_slice(&b);
    }
    if q.len() != len {
        return Err(bad("request", "internal encoded length mismatch"));
    }
    let query = BlockReceiptQuery {
        id,
        digest: digest(&q),
    };
    Ok(Request {
        snapshot,
        q: Arc::from(q),
        query,
    })
}

pub(crate) struct Cursor<'a> {
    b: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    pub(crate) fn new(b: &'a [u8]) -> Self {
        Self { b, at: 0 }
    }
    pub(crate) fn take(&mut self, n: usize) -> BlockResult<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or_else(|| bad("block grammar", "length overflow"))?;
        let b = self
            .b
            .get(self.at..end)
            .ok_or_else(|| bad("block grammar", "truncated field"))?;
        self.at = end;
        Ok(b)
    }
    pub(crate) fn array<const N: usize>(&mut self) -> BlockResult<[u8; N]> {
        self.take(N)?
            .try_into()
            .map_err(|_| bad("block grammar", "fixed field length"))
    }
    pub(crate) fn u8(&mut self) -> BlockResult<u8> {
        Ok(self.array::<1>()?[0])
    }
    pub(crate) fn u16(&mut self) -> BlockResult<u16> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    pub(crate) fn u32(&mut self) -> BlockResult<u32> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    pub(crate) fn u64(&mut self) -> BlockResult<u64> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    pub(crate) fn stamp(&mut self) -> BlockResult<OwnershipStamp> {
        Ok(OwnershipStamp::decode(self.take(9)?)?)
    }
    pub(crate) fn pos(&mut self) -> BlockResult<BlockPos> {
        Ok(BlockPos::new(
            i32::from_be_bytes(self.array()?),
            i32::from_be_bytes(self.array()?),
            i32::from_be_bytes(self.array()?),
        ))
    }
    fn blob(&mut self, max: usize) -> BlockResult<Option<&'a [u8]>> {
        let tag = self.u8()?;
        let len = self.u32()? as usize;
        if tag > 1 || tag == 0 && len != 0 || len > max {
            return Err(bad("blob", "invalid tag/length"));
        }
        let b = self.take(len)?;
        Ok(if tag == 0 { None } else { Some(b) })
    }
    pub(crate) fn finish(self) -> BlockResult<()> {
        if self.at == self.b.len() {
            Ok(())
        } else {
            Err(bad("block grammar", "trailing bytes"))
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PairRef<'a> {
    pub(crate) full: Option<&'a [u8]>,
    pub(crate) overlay: Option<&'a [u8]>,
}
impl<'a> PairRef<'a> {
    fn read(c: &mut Cursor<'a>) -> BlockResult<Self> {
        Ok(Self {
            full: c.blob(crate::MAX_CHUNK_RECORD_LEN)?,
            overlay: c.blob(crate::MAX_CHUNK_RECORD_LEN)?,
        })
    }
    pub(crate) fn present(self) -> bool {
        self.full.is_some() || self.overlay.is_some()
    }
    pub(crate) fn owned(self) -> Pair {
        Pair {
            full: self.full.map(Arc::from),
            overlay: self.overlay.map(Arc::from),
        }
    }
}
impl<'a> From<&'a Pair> for PairRef<'a> {
    fn from(p: &'a Pair) -> Self {
        Self {
            full: p.full.as_deref(),
            overlay: p.overlay.as_deref(),
        }
    }
}
pub(crate) struct Entry<'a> {
    pub(crate) key: ChunkKey,
    pub(crate) before_stamp: OwnershipStamp,
    pub(crate) before: PairRef<'a>,
    pub(crate) action: u8,
    pub(crate) after: PairRef<'a>,
}
pub(crate) struct Effect<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) pos: BlockPos,
    pub(crate) old: u32,
    pub(crate) new: u32,
    pub(crate) old_entity: Option<&'a [u8]>,
    pub(crate) new_entity: Option<&'a [u8]>,
}
pub(crate) fn effect(b: &[u8]) -> BlockResult<Effect<'_>> {
    let mut c = Cursor::new(b);
    c.u32()?;
    c.u64()?;
    let actor = c.u8()?;
    let uuid = c.array::<16>()?;
    let cause = c.u8()?;
    if actor > 1 || actor == 0 && uuid != [0; 16] || cause > 3 {
        return Err(bad("effect", "actor/cause/inactive UUID"));
    }
    let pos = c.pos()?;
    let old = c.u32()?;
    let new = c.u32()?;
    let old_entity = c.blob(16 * 1024)?;
    let new_entity = c.blob(16 * 1024)?;
    c.finish()?;
    Ok(Effect {
        bytes: b,
        pos,
        old,
        new,
        old_entity,
        new_entity,
    })
}
pub(crate) struct Q<'a> {
    pub(crate) kind: u8,
    pub(crate) id: OwnershipTransferId,
    pub(crate) scope: GenerationKey,
    pub(crate) identity: GenerationIdentity,
    pub(crate) player: Option<PlayerId>,
    pub(crate) player_stamp: OwnershipStamp,
    pub(crate) before_player: Option<&'a [u8]>,
    pub(crate) after_player: Option<&'a [u8]>,
    pub(crate) entries: Vec<Entry<'a>>,
    pub(crate) guards: Vec<BlockPos>,
    pub(crate) effects: Vec<Effect<'a>>,
}
pub(crate) fn q(b: &[u8]) -> BlockResult<Q<'_>> {
    if b.len() > MAX_BLOCK_REQUEST_BYTES {
        return Err(bad("FCBQ", "over hard cap"));
    }
    let mut c = Cursor::new(b);
    if c.take(6)? != b"FCBQ\0\x01" {
        return Err(bad("FCBQ", "magic/version"));
    }
    let kind = c.u8()?;
    if !(1..=3).contains(&kind) {
        return Err(bad("FCBQ", "kind"));
    }
    let id = OwnershipTransferId::from_bytes(c.array()?);
    let scope = GenerationKey::decode(c.take(8)?)?;
    let identity = GenerationIdentity::decode(c.take(27)?).map_err(|_| bad("FCBQ", "identity"))?;
    let uuid = c.array::<16>()?;
    let player_stamp = c.stamp()?;
    let before_player = c.blob(5 + crate::MAX_PLAYER_DATA_LEN)?;
    let after_player = c.blob(5 + crate::MAX_PLAYER_DATA_LEN)?;
    let player = if kind == 1 {
        if !player_stamp.is_live() || before_player.is_none() || after_player.is_none() {
            return Err(bad("FCBQ", "joint Player inactive"));
        }
        Some(PlayerId::from_uuid(uuid::Uuid::from_bytes(uuid)))
    } else {
        if uuid != [0; 16]
            || !player_stamp.is_fresh()
            || before_player.is_some()
            || after_player.is_some()
        {
            return Err(bad("FCBQ", "inactive Player bytes"));
        }
        None
    };
    let n = c.u8()? as usize;
    shape("chunks", n, 1, MAX_BLOCK_CHUNKS)?;
    let mut entries: Vec<Entry<'_>> = Vec::with_capacity(n);
    for _ in 0..n {
        let key = crate::persistent_entity::raw::decode_chunk(c.take(16)?, "FCBQ")?;
        if GenerationKey::new(key.world(), key.dimension()) != scope
            || entries.last().is_some_and(|e| e.key >= key)
        {
            return Err(bad("FCBQ", "scope/key ordering"));
        }
        let before_stamp = c.stamp()?;
        let before = PairRef::read(&mut c)?;
        let action = c.u8()?;
        let after = PairRef::read(&mut c)?;
        if before_stamp.is_live() != before.present()
            || action > 2
            || action == 0 && before != after
            || action == 1 && !after.present()
            || action == 2 && after.present()
        {
            return Err(bad("FCBQ", "stamp/action/pair"));
        }
        entries.push(Entry {
            key,
            before_stamp,
            before,
            action,
            after,
        });
    }
    let n = c.u16()? as usize;
    shape("guards", n, 0, MAX_BLOCK_GUARDS)?;
    let mut guards: Vec<BlockPos> = Vec::with_capacity(n);
    for _ in 0..n {
        let p = c.pos()?;
        if guards
            .last()
            .is_some_and(|b| (b.x(), b.y(), b.z()) >= (p.x(), p.y(), p.z()))
        {
            return Err(bad("FCBQ", "guard ordering"));
        }
        guards.push(p);
    }
    let n = c.u16()? as usize;
    shape("effects", n, 0, MAX_BLOCK_EFFECTS)?;
    let mut effects = Vec::with_capacity(n);
    for _ in 0..n {
        let len = c.u32()? as usize;
        effects.push(effect(c.take(len)?)?);
    }
    if kind == 3 && (!guards.is_empty() || !effects.is_empty()) {
        return Err(bad("FCBQ", "imageCAS carries gameplay"));
    }
    c.finish()?;
    Ok(Q {
        kind,
        id,
        scope,
        identity,
        player,
        player_stamp,
        before_player,
        after_player,
        entries,
        guards,
        effects,
    })
}
#[derive(Clone, Copy)]
pub(crate) struct Header {
    pub(crate) stamp: OwnershipStamp,
    pub(crate) id: OwnershipTransferId,
}
pub(crate) fn header(b: &[u8]) -> BlockResult<Header> {
    let mut c = Cursor::new(b);
    if c.take(6)? != b"FCOB\0\x01" {
        return Err(bad(CHUNK_OWNER, "magic/version"));
    }
    let stamp = c.stamp()?;
    if stamp.is_fresh() {
        return Err(bad(CHUNK_OWNER, "stored Fresh"));
    }
    let id = OwnershipTransferId::from_bytes(c.array()?);
    c.finish()?;
    Ok(Header { stamp, id })
}
pub(crate) fn encode_header(h: Header) -> [u8; 31] {
    let mut b = [0; 31];
    b[..6].copy_from_slice(b"FCOB\0\x01");
    b[6..15].copy_from_slice(&h.stamp.encode());
    b[15..].copy_from_slice(&h.id.to_bytes());
    b
}
pub(crate) fn encode_receipt(
    q: &[u8],
    player: OwnershipStamp,
    chunks: &[(ChunkKey, OwnershipStamp)],
    first: Option<u64>,
    count: u16,
) -> Vec<u8> {
    let mut b = Vec::with_capacity(71 + q.len() + 9 * chunks.len());
    b.extend_from_slice(b"FCBR\0\x01");
    b.extend_from_slice(&(q.len() as u32).to_be_bytes());
    b.extend_from_slice(q);
    b.extend_from_slice(&player.encode());
    b.push(chunks.len() as u8);
    for (_, stamp) in chunks {
        b.extend_from_slice(&stamp.encode());
    }
    b.push(u8::from(first.is_some()));
    b.extend_from_slice(&first.unwrap_or(0).to_be_bytes());
    b.extend_from_slice(&count.to_be_bytes());
    b.extend_from_slice(&digest(q));
    b.extend_from_slice(&[0; 8]);
    b
}
/// One FCBR row parsed borrowed: validation reads every field from the stored row and never
/// copies it. Only a receipt actually returned to a caller becomes an owned `BlockReceipt`.
pub(crate) struct Frame<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) query: BlockReceiptQuery,
    pub(crate) q: std::ops::Range<usize>,
    pub(crate) player: OwnershipStamp,
    pub(crate) chunks: Vec<(ChunkKey, OwnershipStamp)>,
    pub(crate) first: Option<u64>,
    pub(crate) count: u16,
}
impl<'a> Frame<'a> {
    pub(crate) fn request_bytes(&self) -> &'a [u8] {
        &self.bytes[self.q.clone()]
    }
    pub(crate) fn journal_range(&self) -> Option<(u64, u16)> {
        self.first.map(|n| (n, self.count))
    }
    /// The one retained output buffer of a returned receipt, charged before it is copied.
    pub(crate) fn owned(&self, budget: &mut engine::Budget) -> BlockResult<BlockReceipt> {
        budget.bytes(self.bytes.len() as u64)?;
        Ok(BlockReceipt {
            bytes: Arc::from(self.bytes),
            query: self.query,
            q: self.q.clone(),
            player: self.player,
            chunks: self.chunks.clone(),
            first: self.first,
            count: self.count,
        })
    }
}
/// Parses and digests one FCBR row and returns an owned receipt for a caller.
pub(crate) fn receipt(
    key: &[u8],
    bytes: &[u8],
    budget: &mut engine::Budget,
) -> BlockResult<BlockReceipt> {
    frame(key, bytes, budget)?.owned(budget)
}
/// Parses and digests one FCBR row borrowed; its body is charged once for parse and digest.
pub(crate) fn frame<'a>(
    key: &[u8],
    bytes: &'a [u8],
    budget: &mut engine::Budget,
) -> BlockResult<Frame<'a>> {
    frame_q(key, bytes, budget).map(|(frame, _)| frame)
}
/// As [`frame`], also returning the one charged parse of its Q, so callers that need the
/// parsed body never parse it a second time.
pub(crate) fn frame_q<'a>(
    key: &[u8],
    bytes: &'a [u8],
    budget: &mut engine::Budget,
) -> BlockResult<(Frame<'a>, Q<'a>)> {
    if bytes.len() > MAX_BLOCK_REQUEST_BYTES + 143 {
        return Err(bad("FCBR", "row exceeds hard bound"));
    }
    let mut c = Cursor::new(bytes);
    if c.take(6)? != b"FCBR\0\x01" {
        return Err(bad("FCBR", "magic/version"));
    }
    let len = c.u32()? as usize;
    if len > MAX_BLOCK_REQUEST_BYTES {
        return Err(bad("FCBR", "Q length"));
    }
    let body = c.take(len)?;
    budget.bytes(body.len() as u64)?;
    #[cfg(test)]
    budget
        .frame_work
        .set((budget.frame_work.get().0 + 1, budget.frame_work.get().1));
    let q = q(body)?;
    if q.id.to_bytes().as_slice() != key {
        return Err(bad("FCBR", "key/ID"));
    }
    let player = c.stamp()?;
    let n = c.u8()? as usize;
    if n != q.entries.len() {
        return Err(bad("FCBR", "chunk count"));
    }
    let mut chunks = Vec::with_capacity(n);
    for e in &q.entries {
        chunks.push((e.key, c.stamp()?));
    }
    let present = c.u8()?;
    let first = c.u64()?;
    let count = c.u16()?;
    let digest = c.array::<32>()?;
    if present > 1
        || present == 0 && (first != 0 || count != 0)
        || present == 1 && (count == 0 || first.checked_add(u64::from(count)).is_none())
        || usize::from(count) != q.effects.len()
        || digest != {
            #[cfg(test)]
            budget
                .frame_work
                .set((budget.frame_work.get().0, budget.frame_work.get().1 + 1));
            self::digest(body)
        }
        || c.array::<8>()? != [0; 8]
    {
        return Err(bad("FCBR", "range/digest/reserved"));
    }
    c.finish()?;
    let query = BlockReceiptQuery { id: q.id, digest };
    Ok((
        Frame {
            bytes,
            query,
            q: 10..10 + len,
            player,
            chunks,
            first: if present == 1 { Some(first) } else { None },
            count,
        },
        q,
    ))
}
pub(crate) fn journal(
    scope: GenerationKey,
    id: OwnershipTransferId,
    ordinal: u16,
    sequence: u64,
    effect: &[u8],
) -> Vec<u8> {
    let mut b = Vec::with_capacity(40 + effect.len());
    b.extend_from_slice(b"FCBL\0\x01");
    b.extend_from_slice(&scope.encode());
    b.extend_from_slice(&id.to_bytes());
    b.extend_from_slice(&ordinal.to_be_bytes());
    b.extend_from_slice(&sequence.to_be_bytes());
    b.extend_from_slice(effect);
    b
}
pub(crate) fn journal_ref(
    b: &[u8],
) -> BlockResult<(GenerationKey, OwnershipTransferId, u16, u64, Effect<'_>)> {
    let mut c = Cursor::new(b);
    if c.take(6)? != b"FCBL\0\x01" {
        return Err(bad(JOURNAL, "magic/version"));
    }
    let scope = GenerationKey::decode(c.take(8)?)?;
    let id = OwnershipTransferId::from_bytes(c.array()?);
    let ordinal = c.u16()?;
    let sequence = c.u64()?;
    let e = effect(&b[c.at..])?;
    Ok((scope, id, ordinal, sequence, e))
}
