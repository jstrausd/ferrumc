# DUR-03.3 — joint Player + Block durability

**Concrete design for Root review. Runtime remains READONLY until Root-CLEAN.**
Exclusive files: this brief and `.codex-tmp/dur03_3/**`. Later implementation
owns only `ferrumc-storage`, with no new dependencies. Root owns board/catalog,
migration sequencing and later Sim/App consumers. Root reports published main
`d3fca66c`, containing independently CLEAN DUR-03.2 and SIM-03.4 `b9f15db8`;
the earlier DUR-03.2 integration report was
`localmain242c8f07f3ea724538a099a3339d4c65b276e863`.
These are Root reports, not independent Git verification.
No Git, runtime edits, subagents or production freezes were used.
Accepted dependencies are pinned from Root's read-only d3fca66c archive at
`.codex-tmp/gen05_7-root-runtime-base/archive` (1224 files, manifest
`c148936b29cc58d7ac25b2904618523c3b3a163476a2bf380b1d00fadf07c7b0`).
Shared World GEN-05.7 and Sim SIM-03.4a WIP are observed deltas only, not authority
or a dependency approval. Shared Sim remains read-only here.
The final design/evidence freeze is `.codex-tmp/dur03_3/DESIGN_EVIDENCE_FREEZE.json`;
it records this reviewable design, not runtime approval or passed acceptance.

## Actual boundary and decision

The actual Format6 producer already seals Player/Entity CAS through private
Weak issuer identity, immutable actual generation descriptors, positive Player
history, unified Memory state and Immediate redb transactions. WorldStore
full/overlay saves/deletion and both journal appends remain separate unchecked
block writers. SchemaVersion, overlay ticks and entity-set epochs are not block
revisions. App's overlay-plus-journal flush ack proves no joint Player durability.
Whole dependency/reference pins and symbol review are in
`.codex-tmp/dur03_3/SOURCE_REVIEW.md` and its JSON manifests.

Choose explicit **Format7**, retaining Format6 semantics unchanged in Format6.
Use the existing Player revision authority, one new Chunk history for its
complete full-record/overlay pair, exact retained new operation bodies and a
separate scoped block journal. All permitted writers use these authorities;
old blind methods refuse. Fresh/Reopen7 belongs to this ticket. Existing6→7
requires the separate offline migration described below, not an automatic open.

One joint gameplay operation covers one explicit scope, a complete Live Player
before/postimage and 1..8 distinct Chunk keys, including door halves and neighbor
read/write dependencies. One actual transaction checks originals, writes Player,
chunk/overlay/block-entity postimages, exact scoped journal effects and receipt.
Standalone block/container/command actions use the same engine without Player.
Chunk import/autosave/compaction uses an image-CAS kind on the same Chunk history.

PlayerRecord remains opaque: schema, GameMode and every payload byte are bound,
without selecting slots. The actual FCPS3 producer preserves position/look/
selection, 46 complete FCIS slots and carried stack; it is not a complete future
Survival player model. Current world codecs support signs/chests and complete
current item components. No item-cost, loot, permission, placement/support or
event policy is invented. Mining requiring entities/XP/loot, scheduled effects
or other persistent resources needs a separate larger-union producer.

## Sealed typed API and complete originals

Add BlockDurability mode and explicit
`InMemoryStore::with_player_block_durability(config)` /
`RedbStore::open_with_player_block_durability(path, config, scratch_directory)`.
The new dyn-compatible PlayerBlockDurabilityStore has five methods:

```rust
async fn load_owned_blocks(&self, spec: BlockSnapshotSpec)
    -> BlockResult<OwnedBlockSnapshot>;
async fn commit_player_blocks(&self, request: PlayerBlockRequest)
    -> BlockResult<BlockCommitOutcome>;
async fn commit_blocks(&self, request: BlockMutationRequest)
    -> BlockResult<BlockCommitOutcome>;
async fn compare_exchange_chunk_images(&self, request: ChunkImageCasRequest)
    -> BlockResult<BlockCommitOutcome>;
async fn load_block_receipt(&self, query: BlockReceiptQuery)
    -> BlockResult<Option<BlockReceipt>>;
```

Defaults/formats3..6 return typed Unsupported, even for empty inputs.
BlockSnapshotSpec contains scope, desired GenerationIdentity, optional PlayerId
and Chunk keys; these are declarations only. Validate 1..8 unique bounded keys
and common scope before IO. One actual redb read transaction or Memory read
boundary compares the actual stored descriptor, captures Player and all Chunk
payloads/histories coherently, and issues OwnedBlockSnapshot. Its private data
bind Weak actual backend incarnation, verified scope/full identity, existing
Player expectation, each Chunk stamp and owned exact original raw pairs.
Arc clones share authority. Another backend/reopen, matching descriptors,
caller digests, decoded stamps or read-only receipts cannot create expectations.
No public expectation/snapshot/evidence constructors, writable fields or
serialization; immutable world views may be cloned into proposals.

Requests consume one genuine snapshot:
PlayerBlockRequest::new(id, snapshot, complete_player_record, images, guards,
effects); BlockMutationRequest::new(id, snapshot_without_player, images, guards,
effects); ChunkImageCasRequest::new(id, snapshot_without_player, images).
Joint requests require Live Player; existing Player CAS owns bootstrap/deletion.
Exactly one ChunkImageChange per snapshot key supplies Keep, Replace(ChunkImage)
or Erase. ChunkImage has optional full/overlay records, at least one for Replace,
checked lossless capture/encoding and original caller schema values. Keep retains
original raw bytes, including readable legacy entity spellings; Erase removes
both rows. Caller-supplied purported originals are never authority.

Guards are 0..4096 distinct BlockPos read dependencies, canonically sorted;
their original state/entity comes from the sealed snapshot. Effects are an
ordered chain, permitting repeated positions. Gameplay union is exactly chunks
of guards, effects and explicit Replace/Erase proposals, equal to the snapshot
keys: no uncovered effect or extra Keep-only key. Door halves and cross-border
neighbors enter this union. Storage proves the declared union and endpoints;
later Sim must prove that its gameplay reads/effects are complete. Storage
cannot infer undeclared physics/support dependencies. Image-CAS's union is its
proposal keys and it has no guards/effects.

Resolve before/post pairs using the actual loaded::compose_chunk rule, sealed
here rather than implicitly tracking a future Sim change: full record as base
if present, otherwise FlatWorldGenerator::with_seed(actual_identity.seed()),
checking its entire identity first. Apply the checked overlay. A full base
plus an overlay carrying sections replaces the complete base entity map before
apply; a zero-section overlay merges entities, preserving the current empty
overlay no-op. Preserve schemas; this storage rule does not certify Sim's
separate overlay-schema admission. Only existing FCGI flat-r1/1.21.8/Overworld
is supported; no inferred scope/seed or future natural-generation authority.

Gameplay requests execute effects privately from the resolved original.
Every old state and complete old entity must match the preceding cell; new
state/entity defines the next cell. Compare all 98,304 block cells and the entire
entity map per Chunk with the resolved postimage, detecting omissions/extra
changes, including entity-only inventory changes. Reject an individual effect
whose state and entity are both unchanged, invalid chains and incomplete
endpoints; an empty effect vector is valid when endpoints are unchanged.
Compare current lossless canonical entity
bytes; original stored legacy bytes remain exactly bound separately.
Image-CAS permits arbitrary complete image replacement and explicitly zero
journal effects. It is a storage image operation, never a fallback for an
unresolved gameplay action; its receipt retains full before/post pairs.

## One authority for every writer

In Format7 old WorldStore full/overlay save/delete and both unscoped journal
append methods refuse before validation/scheduling, including empty/oversized
batches. Old PlayerStore, wire entity and UUID entity mutation/move paths remain
refused. No call-through to legacy maps or transactions. Chunk/overlay reads
validate encountered history through unified state; two independent legacy
reads do not become a coherent joint snapshot. Block/container actions use
scoped mutation CAS; ordinary full/overlay writers use image-CAS on that same
revision. Larger operations cannot split one logical confirmation or reopen
a blind batch writer.

All seven existing PlayerEntityOwnershipStore methods remain supported in7,
with their genuine issuer and v1 request/receipt grammar. Joint block writes,
Player autosave/bootstrap/deletion and Player/Entity transfers use the same
Player rows/header. Internal last-receipt checks explicitly dispatch FCTR/FCBR.
Joint rev N→N+1 conflicts with an old autosave/transfer at N and vice versa.
Block writes do not advance Entity UUID indexes/entity-set epochs.

FCTR and FCBR IDs share ownership_receipt's namespace. Cross-kind ID reuse is
ReceiptCollision, regardless of a read-only digest. Old receipt lookup returns
typed ReceiptKindMismatch for FCBR, never fabricated FCTR. Old historical
relations permit a newer Player header whose last receipt is FCBR. The7
adapter checks marker7 and shared receipt byte agreement explicitly; exact6
reader/validator stays unchanged. Both families share non-waiting worker
admission and receipt row/byte accounting, including v1 no-ops.
Plugin KV retains isolation/bounds and cannot reach these world tables.

Memory moves7 full/overlay raw maps, Chunk headers, scoped journal and counters
inside existing ownership State alongside Player/Entity/receipts. Lock order:
existing shared mutation boundary, generation read lock, single state lock;
generation initialization retains its exclusive boundary. Every7 read
of owned Player/Entity/Chunk data, including UUID compatibility pages, uses
unified state. Generation and isolated plugin reads keep their own maps.
Freshness includes new
tombstones, receipts, journal and counters. No guard crosses await. redb uses
the same private Database/incarnation and writer boundary.

## Exact request and on-disk grammar

Integers are fixed-width big-endian. Scope8, ChunkKey16 and identity27 retain
existing encodings; UUID/operation ID16; BlockPos = three i32; state = u32;
stamp9 = existing tag/u64 grammar. Request keys sort by typed world/dimension/
signed x/z; guards by signed x/y/z, not raw redb byte ordering.
Blob = present:u8 + len:u32 + bytes; absent exactly0/0, present exactly1 with
a valid record. Pair = full:Blob + overlay:Blob, each envelope at most3MiB.
Unknown tags, inactive bytes and trailing bytes refuse.

```text
Q := "FCBQ" + u16=1 + kind:u8 + operation_id16 + scope8 + actual_identity27
   + player_uuid16 + before_player_stamp9 + before_player:Blob + after_player:Blob
   + chunk_count:u8 + ChunkEntry[chunk_count]
   + guard_count:u16 + BlockPos[guard_count]
   + effect_count:u16 + (effect_len:u32 + Effect)[effect_count]
ChunkEntry := key16 + before_stamp9 + before:Pair + action:u8 + after:Pair
Effect := schema:u32 + tick:u64 + actor:u8 + actor_uuid16 + cause:u8
        + position12 + old_state:u32 + new_state:u32
        + old_entity:Blob + new_entity:Blob
```

Kinds1 joint gameplay,2 scoped gameplay without Player,3 image-CAS without
Player. Inactive Player fields are all zero/absent/Fresh; kind1 has two present
complete Player records and Live before stamp. Action0 Keep requires exact
equal pairs;1 Replace a nonempty after pair;2 Erase an empty after pair.
ScopedBlockEffect is a separate private-field typed record with an explicitly
declared GenerationKey and these effect fields, never an inferred wrapper of
BlockMutationLogRecord. Every effect scope must equal the request scope; Q and
each journal row encode that common scope once. Actor0 System has zero UUID;
1 Player retains all bits. New scoped cause tags:
0 PlayerAction,1 Command,2 Plugin,3 Test. They do not redefine old PlayerCreative.
Effect entity blobs are absent or complete current versioned lossless world
records (16KiB max), validated for codec and canonical encoding. No provisional
journal IDs, caller hashes or inferred scope. Effects preserve semantic order;
unordered keys/guards are sorted and duplicates refuse.

Q size = 98 + 46*chunks + 12*guards + 64*effects + all Blob payload lengths.
Hard byte admission precedes encoding/hash/worker work, with checked incremental
reservation. Q excludes Weak addresses and includes every original/postimage,
schema, tick, actor and cause. Producer query digest is
SHA256("FerrumC block operation v1\0" + Q); replay compares retained Q
byte for byte, not caller digests or hash equality alone.

Format7 has exactly existing15 typed tables plus these two byte→byte tables:

| Row | Exact encoding/relationship |
| --- | --- |
| ferrumc:chunk_ownership | Key16 → "FCOB"+u16=1+tag:u8+revision:u64+last_id16. Value31, footprint47. Fresh has neither header nor pair; Live has a nonempty pair; Tombstone an empty pair and positive retained revision. |
| ferrumc:scoped_mutation_log | Sequence8 → "FCBL"+u16=1+scope8+operation_id16+ordinal:u16+sequence:u64+Effect. Value100+entity payloads. Each row binds its actual FCBR and ordinal. |
| New variant in ferrumc:ownership_receipt | ID16 → "FCBR"+u16=1+q_len:u32+Q+after_player_stamp9+chunk_count:u8+after_chunk_stamp9[count]+range_present:u8+first_id:u64+effect_count:u16+query_sha25632+reserved_zero8. Value71+len(Q)+9*chunks; footprint87+len(Q)+9*chunks. |

FCTR1 remains exactly256 bytes and kinds1..3. The7 catalog explicitly admits
FCTR1 or FCBR1; no resized FCTR, kind4 or changed6 parser. Player39/Entity35
headers remain bytewise unchanged; only7's last-ID resolution dispatches both.
Chunk/overlay envelopes remain version2, caller schema values unchanged.

Keep six existing mandatory meta keys with marker7; add exactly nine:
block_request_limit, block_revision_limit, block_revision_count,
operation_receipt_byte_limit, operation_receipt_bytes, scoped_journal_row_limit,
scoped_journal_byte_limit, scoped_journal_count, scoped_journal_bytes.
Only last_mutation_id and last_scoped_mutation_id are optional extras.
Old journal/batch rows and watermark remain unscoped historical records, with
no new writes or inferred attribution. New scoped sequence is independently
contiguous from0, retained permanently. Count accounting permits at mostu64MAX
rows/highest ID MAX-1; count/range/watermark overflow never wraps. FCBR range
is0/0/0 when empty, otherwise1/first/count matching Q and exact journal rows.

## Transitions, bounds, refusal order and real commit

Player follows existing raw-equal no-op/revision rules. Chunk advances once if
its raw pair changes OR any nonempty journal effect touches it, even a cycle
returning to the original endpoint. After tag follows pair presence; Fresh→
Live1, or Tombstone1 for a journaled cycle ending at unstored baseline, is valid.
Erase Fresh/Tombstone is AlreadyAbsent. Keep/raw-equal Replace with no effects
preserves stamp/last pointer even at MAX; guarded-only neighbors do not advance.
Real change at MAX refuses the entire operation. Histories/scope bindings and
receipts never expire. A logical no-op still consumes a receipt/byte capacity.

Config combines existing immutable OwnershipCapacity with a new immutable
positive agreement. Hard caps:8 chunks,4096 guards,1024 effects,32MiB Q,
3MiB each full/overlay,1MiB Player payload,16KiB each effect entity. Persisted
block request cap is144..=32MiB. New Chunk history rows, common receipt bytes,
scoped journal rows/bytes are positive bounded capacities. Common receipt rows
and Player/Entity histories retain existing supplied caps. V1 receipt charges272;
FCBR and journal charge exact key/value bytes. Config checks
receipt_bytes + 55*ownership_revision_cap + 47*chunk_revision_cap +
journal_bytes fitsu64. These are logical retained bounds, not world/plugin
data/redb allocation/RSS quotas; there are no invented deployment defaults.

Separate positive local limits bound raw startup scan rows/bytes, relation work,
one scratch index physical extent, snapshot output and workers1..64. Read/write/
receipt calls cap cumulative inspected/hashed/encoded validation bytes at512MiB and
cell-work units at16,777,216, with configurable tighter limits; charge each cell
access/comparison before work. Before an existing uninstrumented World codec or
flat generator call, reserve its bounded whole cell traversal; Storage supplies
the accounting without modifying World. Startup uses its separate cumulative
scan/relation allowance, never one reset budget per row.
Snapshot hard output cap32MiB charges keys,
headers, images and materialized encoded view sizes before copying. Parse
receipt frames borrowed, resolve one Chunk pair at a time, never recursively
walk another owner's last receipt. No budget resets inside a call. Maximum
shape does not guarantee admission under a tighter work/byte cap.
Receipt output shares one retained Q buffer with borrowed/Arc-backed fields;
its logical encoded row bound is32MiB+159 bytes, with no second full Q copy.

Refusal order: mode; hard shape/scope/duplicates and persisted request bytes;
fail-fast shared permit; actual transaction/agreement/encountered grammar;
existing ID (validate actual receipt, exact replay or collision before
owner/CAS/history); all Weak issuers, actual descriptor/scope/current relations;
Player then sorted Chunk stamps and exact raw originals; complete post codecs,
union/guards/effect chains/endpoints; AlreadyAbsent, Player then Chunk revision
overflow, scoped range overflow/collision; checked receipt rows/bytes,
Player/Entity history, Chunk history and scoped journal rows/bytes; full private
plan. IO fails where actually accessed. No repairs. A matching stamp with
different original bytes is BeforeImageMismatch, not success.

Started blocking workers retain their shared permit after future cancellation.
Busy means no worker started; no awaiting admission queue. redb writer
serialization occurs after bounded admission, without a promised lock timeout.
Memory preflight and one returned-error preapply hook precede infallible apply
under its single lock; midapply hooks only synchronize observers. Allocation/
panic is not represented as returned-error rollback.

One actual redb writer sets Durability::Immediate: all preflight; changed Player
payload/header; sorted full/overlay insert/remove; Chunk headers; exact scoped
journal; optional watermark; FCBR; shared receipt count/bytes, ownership history
count, Chunk count and journal counts/bytes. Preserve unchanged rows/pointers.
Hooks bracket BeforePlan, every actual row/meta insert/remove, BeforeCommit and
AfterCommit. Returned precommit failure drops/aborts the whole transaction.
Actual commit IO failure is CommitUncertain(query,cause); started worker join
failure or postcommit/lost reply is ExecutionUncertain(query,cause).
Neither authorizes compensation, another ID, visible completion or releasing
pending resource ownership. Retain original ID/Q. Receipt absence while a worker
runs does not prove abort.

Outcome exposes actual original receipt and Committed/Replayed only; further
write authority requires a new coherent snapshot. Same old-incarnation request
replays after reopen when committed. If absent after exclusive close/reopen
proves the old worker ended, obtain genuine snapshots and reconstruct the same
Q/ID only if every original/stamp still matches; otherwise reconciliation,
never rebase that ID. ID/digest query reconstruction is read-only authority.

Typed BlockDurabilityError covers Unsupported, MigrationRequired, Catalog/
CapacityContractMismatch, MalformedData, Scope/GenerationMismatch,
BackendOwnerMismatch, PlayerNotLive, CasConflict, BeforeImageMismatch,
AlreadyAbsent, ReceiptCollision/ReceiptKindMismatch, Batch/Request/Snapshot
capacity, InvalidEffect/IncompleteDelta, HistoryCapacity, Revision/Sequence
exhaustion, JournalIdCollision, Validation/Scan/ScratchCapacity, Busy, Backend,
CommitUncertain, ExecutionUncertain, UnsupportedPlatform and ScratchCleanup,
with resource/key/ordinal/query as
applicable. Explicit typed7 kind/byte/work adapters extend non-exhaustive
Ownership errors; no classification by error strings.

## Fresh/Reopen and separate migration

Fresh create_new writes exactly17 tables/all15 mandatory meta keys with zero
counters and no watermarks in one Immediate transaction. Existing7 opens
validate without committing initialization/normalization. Existing3..6 requires
migration, including empty6; unknown/missing markers, tables/types/multimaps,
meta or agreement refuse without logical writes. Exact6 opener refuses7.

Reopen streams all17 tables, admitting borrowed rows before parse/hash/copy
within raw and relation-work limits. Validate reused descriptors, Player/Entity
headers/indexes/epochs and old journal/batch relationships; explicitly dispatch
receipt variants. Check Chunk pairs/history both ways, positive revisions,
last-changing operation/transition and exact current raw postimage. Each FCBR
rederives Q digest/union/chains/endpoints/after stamps. Historical retained
revisions must dominate receipt outcomes; tags/images match at equality.
Fresh-after no-op Chunk entries legitimately have no header, or a later positive
history; an absent current header permits only Fresh outcomes. Equal Fresh
requires an absent pair. Relations read raw headers without recursive
last-receipt walks. Every scoped
journal row and FCBR ordinal agrees in both directions, including scope,
allocated ID and full effect bytes. Check contiguous sequences/watermarks and
actual counter/byte sums/caps.

Range nonoverlap uses one bounded disposable ordered disk index with separate
legacy/scoped domains, not an unbounded vector or quadratic scan. Explicit
scratch directory, create_new/inode ownership, symlink refusal, preservation
of foreign replacements and typed cleanup after DB release. The actual existing
file-identity machinery is Unix; redb7 refuses unsupported platforms explicitly,
without weakening ownership. No logical FerrumC repair is performed; redb's
physical recovery is not a bytewise source-preservation claim. Main cache64MiB,
index cache16MiB plus bounded row/Chunk/request/plan/output and backend overhead;
no total RSS or physical source-hash guarantee.

**Required separate ticket: DUR-03.3a, offline Format6→7 block-authority migration,
ordered by Root before DUR-05.** DUR-03.3 does not implement it or broaden
DUR-03.2. Preserve all
original descriptors, Player/Entity/index/epoch/header/FCTR, plugin, full/overlay
and old unscoped journal/batch bytes. Plan exact unique Chunk key union; record
collision-free IDs before Stage CAS; preflight new row/byte histories. Private
Stage7 retains non-Chunk state but initially omits both Chunk payload tables.
Genuine Stage snapshots observe Fresh pairs; actual image-CAS imports each
exact checked original raw pair through a private actual Source-read factory,
creating revision1/FCBR without invented gameplay journal effects. This needs
its own migration producer; ordinary reencoding cannot preserve legacy bytes.

Independent original-plus-delta comparison and full Stage close/reopen precede
sealing. Only marker/new meta/two tables, real Chunk headers/FCBRs and common
receipt counters change; payloads/entity epochs remain exact, scoped log empty.
Move-only checkpoint binds Source inode, explicit paths, immutable closed
Backup/Stage/ID-plan hashes, complete source/target catalog/logical fingerprints,
producer pins and limits. Revalidate/import exact delta in one Immediate Source
writer, then close/reopen. Resume proves exact original6 or committed7 and
refuses intervening gameplay; generic valid7 is not retry proof. Never reopen
sealed originals; use bounded identity-owned verification copies and retained
DUR-03.2 file/extent/cleanup disciplines. Its concrete checkpoint grammar,
resource limits and real cutpoint tests must be reviewed before that ticket is
authorized. No automatic rollback or live7 activation follows this brief.

## Meaningful implementation acceptance and Root decisions

The following are planned tests, not passed storage or Paper evidence:

| Case | Required actual producer observation |
| --- | --- |
| Memory/redb joint multi-Chunk | Real v1 bootstrap + genuine snapshot; door halves, cross-border neighbor, chest/sign/entity-only delta and complete Player components. Assert whole independent expected delta, unaffected bytes/cells, one receipt and exact journal; actual redb close/reopen. |
| Every real writer cutpoint | Before/after every plan row/meta and BeforeCommit errors leave all17 logical tables wholly old, receipt absent, including reopen. Actual AfterCommit/Lostreply is wholly new; same-ID replay changes no rows/counters. Memory errors only preapply; reader cannot capture midapply Player/world. |
| Commit IO uncertainty | Actual redb StorageBackend commit write/fsync fault; release/reopen proves whole old or new with corresponding receipt. Never assert IO error proves abort. Process-kill/power-loss remains separate. |
| Concurrent Player/Chunk | Barrier-driven joint vs v1 autosave and Player/Entity transfer at one stamp: one changing winner, whole loser conflicts. Different Players/same Chunk: one wins, loser inventory unchanged. Disjoint unions both commit; include guard-only neighbor drift and overlapping multi-Chunk unions. |
| Exact replay/issuer/generation | Same-ID same-Q races commit once. Changed kind/original/post/component/schema/tick/cause/effect order/scope collides. Altered query digest collides. Foreign/reopened issuer without receipt refuses; committed old request replays. Historical receipt cannot substitute current state; identical terrain/different seed still refuses. |
| Pair/endpoint/bypass | Full, overlay, both, neither; imported entities replaced by section overlay; zero-section merge/empty overlay. Reject broken chains, hidden/missing cells, wrong union/column/height, oversized/malformed entities, duplicate keys/guards. Image-CAS advances same history and invalidates pending joint. Old full/overlay/delete/journal writers refuse even empty/malformed batches. |
| No-op/overflow/caps | Exact no-op preserves pointers/MAX stamps but charges receipt bytes; guarded Fresh stays Fresh, journal cycle advances once. Absent Erase refuses; real Player/Chunk MAX change refuses. Checked sequence allocator boundary plus malformed actual MAX watermark refuses without writes. N/N+1 for8/4096/1024, Q/envelopes/components, every retained row/byte, runtime work/output, scan and scratch extent. |
| Grammar/reopen | Every truncated prefix/length, trailing/reserved/inactive byte, kind/version/tag, orphan header/receipt/log/ordinal/range, descriptor/counter and range overlap refuses without repair. Bounded physical index writes and cleanup preserve foreign replacements. |
| Busy/cancelled future | Real worker barrier holds shared permit; v1/new caller returns Busy without starting. Dropped future retains worker permit; release, query and exact retry original. No wall-clock sleeps. |
| Root-independent contract | Separate public-API Memory/redb fixture with fixed complete FCPS3/FCIS/component expected bytes, no forged expectations/receipts. After dropping DB handle independently decode actual17-table rows/expected delta, without author's plan/codec helpers. Include loss/reopen and shared Player/Chunk races; backend agreement alone is insufficient. |

Require whole Storage Debug/Release tests, fmt check, storage clippy -D warnings,
and Root's independent contract run with pinned dependencies/toolchain. Root
arranges whole-workspace strict checks at a quiet shared-tree boundary; shared
Sim is not edited to pass. Recheck whole read-only pins; drift invalidates the
affected dependency review. Reference source review is not differential runtime
acceptance and does not prove FerrumC storage.

Root decisions: review this concrete7 contract and authorize only Storage after
CLEAN; retain assigned DUR-03.3a before DUR-05; amend downstream
Sim/App briefs to consume actual issued authority and retain unresolved work.
No remaining gameplay-policy question blocks Storage. Deployment capacities
belong to later operator configuration. No live App/Survival, Java-plugin,
Paper comparison or power-loss completion is declared.
