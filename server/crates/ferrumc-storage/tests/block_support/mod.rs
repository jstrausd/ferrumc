//! Public-API fixture. Expectations are fixed independent bytes; no Storage plan/codec oracle.
#![allow(dead_code)]
use ferrumc_core::{DimensionId, GameMode, PlayerId, WorldId};
use ferrumc_math::{BlockPos, ChunkPos};
use ferrumc_storage::*;
use ferrumc_world::{decode_block_entity, BlockStateId, Chunk, GenerationIdentity, WorldSeed};
use redb::{Database, ReadableTable, TableDefinition, TableHandle};
use std::{
    collections::BTreeMap,
    num::{NonZeroU16, NonZeroU64, NonZeroUsize},
    path::Path,
};

pub fn nz(n: u64) -> NonZeroU64 {
    NonZeroU64::new(n).unwrap()
}
/// Every positive local/persisted bound used by the public fixtures; `Default` is the shared baseline.
#[derive(Debug, Clone, Copy)]
pub struct Caps {
    pub receipts: u64,
    pub revisions: u64,
    pub request: usize,
    pub chunks: u64,
    pub receipt_bytes: u64,
    pub journal_rows: u64,
    pub journal_bytes: u64,
    pub scan_rows: u64,
    pub scan_bytes: u64,
    pub scratch: u64,
    pub inflight: u16,
    pub bytes: u64,
    pub cells: u64,
    pub snapshot: u64,
    pub relation_bytes: u64,
    pub relation_cells: u64,
}
impl Default for Caps {
    fn default() -> Self {
        Self {
            receipts: 1000,
            revisions: 1000,
            request: MAX_BLOCK_REQUEST_BYTES,
            chunks: 1000,
            receipt_bytes: 256 * 1024 * 1024,
            journal_rows: 10_000,
            journal_bytes: 64 * 1024 * 1024,
            scan_rows: 100_000,
            scan_bytes: 512 * 1024 * 1024,
            scratch: 16 * 1024 * 1024,
            inflight: 4,
            bytes: 512 * 1024 * 1024,
            cells: 16_777_216,
            snapshot: 32 * 1024 * 1024,
            relation_bytes: 2 * 1024 * 1024 * 1024,
            relation_cells: 1_000_000_000,
        }
    }
}
pub fn config_with(c: Caps) -> BlockResult<BlockDurabilityConfig> {
    let owned = OwnershipConfig::new(
        OwnershipCapacity::new(
            NonZeroUsize::new(MAX_OWNERSHIP_REQUEST_BYTES).unwrap(),
            nz(c.receipts),
            nz(c.revisions),
        )
        .unwrap(),
        OwnershipOpenLimits::new(
            nz(c.scan_rows),
            nz(c.scan_bytes),
            nz(c.scratch),
            NonZeroU16::new(c.inflight).unwrap(),
        )
        .unwrap(),
    );
    BlockDurabilityConfig::new(
        BlockDurabilityCapacity::new(
            owned.capacity(),
            NonZeroUsize::new(c.request).unwrap(),
            nz(c.chunks),
            nz(c.receipt_bytes),
            nz(c.journal_rows),
            nz(c.journal_bytes),
        )?,
        owned,
        BlockDurabilityLimits::new(
            nz(c.bytes),
            nz(c.cells),
            nz(c.snapshot),
            nz(c.relation_bytes),
            nz(c.relation_cells),
        )?,
    )
}
pub fn config() -> BlockDurabilityConfig {
    config_with(Caps::default()).unwrap()
}
pub fn scope() -> GenerationKey {
    GenerationKey::new(WorldId::new(17), DimensionId::new(3))
}
pub fn generation() -> GenerationIdentity {
    GenerationIdentity::flat_overworld(WorldSeed::new(-827))
}
pub fn player() -> ScopedPlayerKey {
    ScopedPlayerKey::new(
        scope(),
        PlayerId::from_uuid(uuid::Uuid::from_bytes([0x8d; 16])),
    )
}
pub fn key(x: i32) -> ChunkKey {
    ChunkKey::new(scope().world(), scope().dimension(), ChunkPos::new(x, -1))
}
pub fn id(n: u8) -> OwnershipTransferId {
    OwnershipTransferId::from_bytes([n; 16])
}
pub fn spec(player_present: bool, keys: Vec<ChunkKey>) -> BlockSnapshotSpec {
    BlockSnapshotSpec::new(
        scope(),
        generation(),
        player_present.then_some(player().id()),
        keys,
    )
    .unwrap()
}
/// Fixed FCIS: ordered typed damage, opaque bytes and removal. No encoder is the oracle.
pub fn stack(count: u8) -> Vec<u8> {
    let mut b = b"FCIS\0\x01\x01\0\x0fminecraft:stone".to_vec();
    b.extend_from_slice(&[count, 2, 1]);
    b.extend_from_slice(&[2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 7]);
    b.extend_from_slice(&[7, 0, 0, 0, 12, 0, 0, 0, 4, 0xde, 0xad, 0x00, 0xff]);
    b.extend_from_slice(&[0, 0, 0, 4]);
    b
}
/// All46 complete FCIS slots plus carried, position/look/selection and opaque `GameMode` envelope.
pub fn player_bytes(count: u8) -> Vec<u8> {
    let mut b = b"FCPS\0\x03".to_vec();
    for n in [-15.25f64, 64.5, -0.0] {
        b.extend_from_slice(&n.to_bits().to_be_bytes());
    }
    for n in [17.5f32, -0.0] {
        b.extend_from_slice(&n.to_bits().to_be_bytes());
    }
    b.push(7);
    b.extend_from_slice(&46u16.to_be_bytes());
    for i in 0..47 {
        let s = stack(if i == 7 { count } else { (i % 63 + 1) as u8 });
        b.extend_from_slice(&(s.len() as u32).to_be_bytes());
        b.extend_from_slice(&s);
    }
    b
}
pub fn p(count: u8) -> PlayerRecord {
    PlayerRecord::new(
        SchemaVersion::new(3),
        GameMode::Survival,
        player_bytes(count),
    )
    .unwrap()
}
pub fn player_envelope(count: u8) -> Vec<u8> {
    let mut b = 3u32.to_be_bytes().to_vec();
    b.push(0);
    b.extend_from_slice(&player_bytes(count));
    b
}
pub fn chest_bytes(count: u8) -> Vec<u8> {
    let mut b = vec![0x7f, 0, 1, 1];
    for i in 0..27 {
        let s = stack(if i == 0 { count } else { (i + 1) as u8 });
        b.extend_from_slice(&(s.len() as u32).to_be_bytes());
        b.extend_from_slice(&s);
    }
    b
}
pub fn pos(x: i32, y: i32) -> BlockPos {
    BlockPos::new(x, y, -1)
}
pub fn effect(
    position: BlockPos,
    old: u32,
    new: u32,
    old_entity: Option<Vec<u8>>,
    new_entity: Option<Vec<u8>>,
) -> ScopedBlockEffect {
    effect_in(
        scope(),
        Meta::default(),
        position,
        old,
        new,
        old_entity,
        new_entity,
    )
}
/// Retained non-cell effect fields; `Default` is the fixed fixture spelling.
#[derive(Debug, Clone, Copy)]
pub struct Meta {
    pub schema: u32,
    pub tick: u64,
    pub cause: ScopedBlockCause,
}
impl Default for Meta {
    fn default() -> Self {
        Self {
            schema: 97,
            tick: 481,
            cause: ScopedBlockCause::PlayerAction,
        }
    }
}
pub fn effect_in(
    scope: GenerationKey,
    meta: Meta,
    position: BlockPos,
    old: u32,
    new: u32,
    old_entity: Option<Vec<u8>>,
    new_entity: Option<Vec<u8>>,
) -> ScopedBlockEffect {
    ScopedBlockEffect::new(
        scope,
        SchemaVersion::new(meta.schema),
        meta.tick,
        MutationActor::Player(player().id()),
        meta.cause,
        position,
        BlockStateId::new(old),
        BlockStateId::new(new),
        old_entity,
        new_entity,
    )
    .unwrap()
}
pub trait Store:
    PlayerBlockDurabilityStore
    + PlayerEntityOwnershipStore
    + WorldStore
    + PlayerStore
    + PersistentEntityStore
{
}
impl<
        T: PlayerBlockDurabilityStore
            + PlayerEntityOwnershipStore
            + WorldStore
            + PlayerStore
            + PersistentEntityStore,
    > Store for T
{
}
pub async fn initialize(s: &dyn Store) {
    s.ensure_generation_identity(scope(), generation())
        .await
        .unwrap();
    let before = s.load_owned_player(player(), generation()).await.unwrap();
    s.compare_exchange_owned_player(
        PlayerOwnershipCasRequest::new(
            id(1),
            before.expectation().clone(),
            PlayerOwnershipProposal::put(p(11)),
        )
        .unwrap(),
    )
    .await
    .unwrap();
}
pub async fn seed_air(s: &dyn Store) {
    let snap = s
        .load_owned_blocks(spec(false, vec![key(0), key(1)]))
        .await
        .unwrap();
    let images = [key(0), key(1)]
        .into_iter()
        .map(|k| {
            ChunkImageChange::new(
                k,
                ChunkImageAction::Replace(
                    ChunkImage::new(
                        Some(
                            ChunkRecord::try_new(SchemaVersion::new(55), Chunk::new(k.pos()))
                                .unwrap(),
                        ),
                        None,
                    )
                    .unwrap(),
                ),
            )
        })
        .collect();
    s.compare_exchange_chunk_images(ChunkImageCasRequest::new(id(2), snap, images).unwrap())
        .await
        .unwrap();
}
pub async fn joint(s: &dyn Store, n: u8) -> PlayerBlockRequest {
    let snapshot = s
        .load_owned_blocks(spec(true, vec![key(0), key(1)]))
        .await
        .unwrap();
    joint_variant(id(n), snapshot, Variant::default())
}
/// One retained field per knob; `Default` reproduces the fixed joint fixture exactly.
#[derive(Debug, Clone, Copy)]
pub struct Variant {
    pub post_schema: u32,
    pub chest: u8,
    pub player: u8,
    pub meta: Meta,
    pub swap: bool,
}
impl Default for Variant {
    fn default() -> Self {
        Self {
            post_schema: 57,
            chest: 8,
            player: 10,
            meta: Meta::default(),
            swap: false,
        }
    }
}
/// Door halves (two stacked cells), a chest and a cross-border neighbor read/write.
pub fn joint_parts(
    snapshot: &OwnedBlockSnapshot,
    v: Variant,
) -> (Vec<ChunkImageChange>, Vec<BlockPos>, Vec<ScopedBlockEffect>) {
    let mut a = snapshot.columns()[0].chunk().clone();
    let mut b = snapshot.columns()[1].chunk().clone();
    let old = |c: &Chunk, p: BlockPos| c.get_block(p).unwrap().as_u32();
    let (a0, a1, a2, b0) = (
        old(&a, pos(15, 64)),
        old(&a, pos(15, 65)),
        old(&a, pos(14, 64)),
        old(&b, pos(16, 64)),
    );
    a.set_block(pos(15, 64), BlockStateId::new(100)).unwrap();
    a.set_block(pos(15, 65), BlockStateId::new(101)).unwrap();
    a.set_block(pos(14, 64), BlockStateId::new(200)).unwrap();
    a.set_block_entity(
        pos(14, 64),
        decode_block_entity(&chest_bytes(v.chest)).unwrap(),
    )
    .unwrap();
    b.set_block(pos(16, 64), BlockStateId::new(300)).unwrap();
    let images = vec![
        ChunkImageChange::new(
            key(1),
            ChunkImageAction::Replace(
                ChunkImage::new(
                    Some(ChunkRecord::try_new(SchemaVersion::new(56), b).unwrap()),
                    None,
                )
                .unwrap(),
            ),
        ),
        ChunkImageChange::new(
            key(0),
            ChunkImageAction::Replace(
                ChunkImage::new(
                    Some(ChunkRecord::try_new(SchemaVersion::new(v.post_schema), a).unwrap()),
                    None,
                )
                .unwrap(),
            ),
        ),
    ];
    let e = |p, o, n, entity| effect_in(scope(), v.meta, p, o, n, None, entity);
    let mut effects = vec![
        e(pos(15, 64), a0, 100, None),
        e(pos(15, 65), a1, 101, None),
        e(pos(14, 64), a2, 200, Some(chest_bytes(v.chest))),
        e(pos(16, 64), b0, 300, None),
    ];
    if v.swap {
        effects.swap(0, 1);
    }
    (images, vec![pos(16, 65), pos(15, 63)], effects)
}
pub fn joint_variant(
    n: OwnershipTransferId,
    snapshot: OwnedBlockSnapshot,
    v: Variant,
) -> PlayerBlockRequest {
    let (images, guards, effects) = joint_parts(&snapshot, v);
    PlayerBlockRequest::new(n, snapshot, p(v.player), images, guards, effects).unwrap()
}
/// Independently constructs the full envelope for the fixed air-base fixture.
pub fn full(
    k: ChunkKey,
    schema: u32,
    changes: &[(BlockPos, u32)],
    entities: &[(BlockPos, Vec<u8>)],
) -> Vec<u8> {
    let mut b = schema.to_be_bytes().to_vec();
    b.extend_from_slice(&k.pos().x().to_be_bytes());
    b.extend_from_slice(&k.pos().z().to_be_bytes());
    b.extend_from_slice(b"FCHK\0\0\0\x02");
    b.push(24);
    for section in 0..24 {
        let base = -64 + section * 16;
        if changes
            .iter()
            .any(|(p, _)| p.y() >= base && p.y() < base + 16)
        {
            b.push(1);
            for y in base..base + 16 {
                for z in k.pos().z() * 16..k.pos().z() * 16 + 16 {
                    for x in k.pos().x() * 16..k.pos().x() * 16 + 16 {
                        let p = BlockPos::new(x, y, z);
                        let state = changes.iter().find(|(q, _)| *q == p).map_or(0, |(_, s)| *s);
                        b.extend_from_slice(&state.to_be_bytes());
                    }
                }
            }
        } else {
            b.push(0);
        }
    }
    b.extend_from_slice(&(entities.len() as u32).to_be_bytes());
    for (p, v) in entities {
        for n in [p.x(), p.y(), p.z()] {
            b.extend_from_slice(&n.to_be_bytes());
        }
        b.extend_from_slice(&(v.len() as u32).to_be_bytes());
        b.extend_from_slice(v);
    }
    b
}
pub type Rows = BTreeMap<String, BTreeMap<Vec<u8>, Vec<u8>>>;
pub fn raw17(path: &Path) -> Rows {
    let db = Database::open(path).unwrap();
    let t = db.begin_read().unwrap();
    assert_eq!(t.list_tables().unwrap().count(), 17);
    assert_eq!(t.list_multimap_tables().unwrap().count(), 0);
    let mut out = Rows::new();
    for name in t.list_tables().unwrap() {
        let n = name.name();
        let rows = if n == "ferrumc:meta" {
            t.open_table(TableDefinition::<&str, u64>::new(n))
                .unwrap()
                .iter()
                .unwrap()
                .map(|r| {
                    let (k, v) = r.unwrap();
                    (
                        k.value().as_bytes().to_vec(),
                        v.value().to_be_bytes().to_vec(),
                    )
                })
                .collect()
        } else {
            t.open_table(TableDefinition::<&[u8], &[u8]>::new(n))
                .unwrap()
                .iter()
                .unwrap()
                .map(|r| {
                    let (k, v) = r.unwrap();
                    (k.value().to_vec(), v.value().to_vec())
                })
                .collect()
        };
        out.insert(n.to_string(), rows);
    }
    out
}
pub fn chunk_key(k: ChunkKey) -> Vec<u8> {
    let mut b = Vec::new();
    for n in [k.world().get(), k.dimension().get()] {
        b.extend_from_slice(&n.to_be_bytes());
    }
    b.extend_from_slice(&k.pos().x().to_be_bytes());
    b.extend_from_slice(&k.pos().z().to_be_bytes());
    b
}
pub fn meta(rows: &Rows, k: &str) -> u64 {
    u64::from_be_bytes(
        rows["ferrumc:meta"][k.as_bytes()]
            .as_slice()
            .try_into()
            .unwrap(),
    )
}
/// Raw read-only independent decoder, not the author's parser.
pub struct Read<'a> {
    b: &'a [u8],
    pub at: usize,
}
impl<'a> Read<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Self { b, at: 0 }
    }
    pub fn take(&mut self, n: usize) -> &'a [u8] {
        let b = &self.b[self.at..self.at + n];
        self.at += n;
        b
    }
    pub fn n(&mut self, n: usize) -> u64 {
        self.take(n).iter().fold(0, |a, b| (a << 8) | u64::from(*b))
    }
    pub fn blob(&mut self) -> Option<&'a [u8]> {
        let tag = self.n(1);
        let n = self.n(4) as usize;
        match tag {
            0 => {
                assert_eq!(n, 0);
                None
            }
            1 => Some(self.take(n)),
            _ => panic!("invalid blob"),
        }
    }
    pub fn done(&self) {
        assert_eq!(self.at, self.b.len());
    }
}

/// Reads fixed whole-table fixtures; expected bytes never come from a runtime codec.
pub fn fixed_rows(text: &str) -> Rows {
    fn hex(s: &str) -> Vec<u8> {
        assert_eq!(s.len() % 2, 0);
        s.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| {
                let d = |b: u8| match b {
                    b'0'..=b'9' => b - b'0',
                    b'a'..=b'f' => b - b'a' + 10,
                    _ => panic!("fixture hex"),
                };
                d(p[0]) * 16 + d(p[1])
            })
            .collect()
    }
    let mut out = Rows::new();
    let mut current = None;
    for line in text.lines() {
        let parts = line.split('\t').collect::<Vec<_>>();
        match parts.as_slice() {
            [table, "-", "-"] => {
                assert!(out.insert((*table).into(), BTreeMap::new()).is_none());
                current = Some((*table).to_string());
            }
            [key, value] => {
                assert!(out
                    .get_mut(current.as_ref().unwrap())
                    .unwrap()
                    .insert(hex(key), hex(value))
                    .is_none());
            }
            _ => panic!("fixture field count"),
        }
    }
    assert_eq!(out.len(), 17);
    out
}
