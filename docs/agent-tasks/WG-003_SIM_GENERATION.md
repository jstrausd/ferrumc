# WG-003 / GEN-01.2 — Sim generator admission and checked overlay capture

Scope: `server/crates/ferrumc-sim/**` and this document only. Storage metadata
(WG-002), explicit offline format migration and App activation are separate
tickets. No Git operations belong to this implementation ticket.

## Result and limits

Sim consumes the existing World `ChunkGenerator` / `GenerationIdentity`
contract for one explicitly bound storage world/dimension. It verifies existing
metadata before world reads/regeneration and before resident ticket increments.
It regenerates the declared baseline on a true chunk miss, composes current
overlays losslessly, and rejects mismatched generator/store descriptors, missing
metadata, corrupt metadata, wrong scope or wrong returned columns. No code
initializes, infers or repairs a descriptor.

The only currently supported descriptor remains MC 1.21.8 / FerrumC flat
revision 1 / Overworld. This ticket does not implement Vanilla noise terrain,
biomes, features, structures, staged generation, lighting, mobs, Survival or
Paper differential acceptance. The App still uses its legacy format-3 path;
metadata activation is WG-004.

## Public consumer contract

```rust,ignore
GenerationBinding::new(GenerationKey, GenerationIdentity) -> GenerationBinding
binding.key() -> GenerationKey
binding.identity() -> GenerationIdentity
binding.validate_store(&dyn WorldStore).await -> SimResult<()>
binding.verify_store(&Arc<dyn WorldStore>).await -> SimResult<VerifiedGenerationBinding>
verified.binding() -> GenerationBinding

LoadedChunkMap::with_generation(GenerationBinding) -> LoadedChunkMap
LoadedChunkMap::with_generation_and_schema_version(GenerationBinding, SchemaVersion)
    -> LoadedChunkMap
map.generation_binding() -> Option<GenerationBinding>
SimShard::with_generation(ShardPos, GenerationBinding) -> SimShard
SimShard::with_generation_and_inbox_capacity(ShardPos, GenerationBinding, NonZeroUsize)
    -> SimShard

load_or_generate_bound(&dyn WorldStore, &dyn ChunkGenerator, GenerationBinding, ChunkKey)
    .await -> SimResult<(Chunk, ChunkProvenance)>
map.acquire(&dyn WorldStore, &dyn ChunkGenerator, ChunkPos, ChunkTicket)
    .await -> SimResult<ChunkAcquired>
map.acquire_spawn(&dyn WorldStore, &dyn ChunkGenerator, &SpawnChunkTickets)
    .await -> SimResult<usize>

map.try_take_persist_dirty_verified(
    &Arc<dyn WorldStore>, &VerifiedGenerationBinding, u64, NonZeroUsize
) -> SimResult<Vec<(ChunkKey, ChunkOverlayRecord)>>
map.try_take_persist_dirty_bound(&dyn WorldStore, u64, NonZeroUsize)
    .await -> SimResult<Vec<(ChunkKey, ChunkOverlayRecord)>>
```

`GenerationBinding::new` is declarative, not a store attestation. Off-tick
bootstrap first initializes/confirms explicitly selected metadata through
Storage or performs the separately authorized offline migration. It then calls
`verify_store` on the `Arc<dyn WorldStore>` the driver and storage worker will
share. The opaque token retains only a `Weak` allocation identity; it does not
keep a database open or expose a public unchecked constructor. The metadata
contract of `WorldStore` is immutable. A custom backend that changes/deletes
confirmed metadata violates that contract.

The driver can reuse `try_take_persist_dirty_verified` synchronously, with zero
metadata reads on its tick path. The map's exact scope/descriptor and the
token's store allocation must match. `Arc` clones work; a separate same-content
store does not. A weak token neither keeps its store alive nor permits a new
allocation to impersonate it. The fresh async alternative is available for
off-tick consumers but should not add storage I/O to App's synchronous tick.

Both checked paths finish all metadata/authorization checks and the complete
bounded immutable overlay/codec preflight before clearing **any** selected
persist-dirty mask or advancing the fair cursor. Capacity is clamped to
`MAX_SAVE_BATCH` before allocation. Errors preserve chunks, tickets, masks,
component contents and cursor, including a valid early record before a later
record's codec failure. Cumulative edited masks remain set; unselected overflow
stays dirty, and deferred higher keys precede repeatedly re-dirtied lower keys.

## WG-004 App and writer requirements

1. Select the supported seed/profile explicitly from validated configuration.
   Do not infer a seed from absent format-3 metadata or silently reset/change a
   stored descriptor. Format-3 migration remains explicit and offline.
2. Open the explicitly metadata-enabled Storage API, initialize/confirm the
   exact binding before world-bearing saves, and obtain the opaque token
   off-tick. Build the bound shard; preserve existing bounded/nonblocking driver
   admission and storage-worker capacity reservation.
3. Give driver and writer clones of the exact same `Arc<dyn WorldStore>`;
   supply it and its token to `try_take_persist_dirty_verified` only after
   reserving the bounded queue slot. A refused capture queues nothing and
   clears nothing. An accepted capture transfers dirty ownership into the
   move-owned output records, not into durable storage.
4. The writer retains the expected `GenerationBinding` and calls
   `binding.validate_store(store.as_ref()).await` before the initial durable
   world-bearing save and each retry. Storage's admission checks descriptor
   **existence**, not a writer's requested descriptor. Keep a failed batch for
   retry and retain existing journal ordering/idempotency/receipt guarantees;
   never retarget captured records to another store, scope or baseline.
5. A final-ticket release is allowed only after successful checked capture and
   durable-retry ownership handoff. Metadata/capture errors must not become
   empty successes, legacy fallbacks or chunk misses. Sim owns no retry queue,
   database handle or flush policy.

## Explicit historic format-3 API

`LoadedChunkMap::new_legacy` and `with_legacy_schema_version` select the historic
MC 1.21.8 / FerrumC flat revision 1 / Overworld / seed-0 baseline explicitly.
`new` and `with_schema_version` remain aliases. The free
`load_or_generate(&dyn WorldStore, &FlatWorldGenerator, ChunkKey)` signature
remains unchanged and aliases `load_or_generate_legacy`; both refuse metadata
backends and a changed generator identity. Existing direct `acquire` and
`acquire_spawn` calls with `&FlatWorldGenerator` still compile through trait
coercion; their generator parameter now accepts `&dyn ChunkGenerator` for bound
maps. A declared incompatible legacy generator refuses before generation or
existing-resident ticket increment. A forged same-identity implementation would
violate `ChunkGenerator`'s trusted deterministic contract.

The existing synchronous signature
`try_take_persist_dirty_bounded(u64, NonZeroUsize) -> Result<Batch, StorageError>`
remains legacy-only. For a metadata map it returns
`GenerationMetadataUnsupported` because this API has no target authorization.
Infallible `take_dirty`, `take_persist_dirty` and the experimental scheduler's
internal bounded legacy drain are documented no-ops on metadata maps, retaining
all masks and cursors. Their empty results must **never** stand in for metadata
flush success. WG-004 consumes only the bound `Result` API. The legacy lossy
drains and experimental scheduler are not activated for metadata persistence.

Stored full-record schema handling is unchanged; full imports remain the base,
even for a caller-preserved schema 9. Overlays must be exactly application schema
3. Empty overlays preserve imported block entities; nonempty overlays supply
the complete entity snapshot. Loaded compositions remain network/persist clean,
while cumulative edited sections are restored for later complete overlays.

## Reference review

Pinned Paper: Java Edition 1.21.8, Build 60, commit
`29c8822d90899c89d2689338e81a98f690bcba12`, under
`.codex-tmp/paper-1.21.8-reference/`.

Reviewed applied sources:

- `net/minecraft/server/level/ServerChunkCache.java`: actual `syncLoad` and
  future-loading methods call Moonrise's `ChunkTaskScheduler.scheduleChunkLoad`.
- `net/minecraft/server/level/ChunkMap.java`: chunk storage/tag-upgrade context;
  its old `scheduleChunkLoad`, `applyStep` and scheduling helpers are replaced
  with `UnsupportedOperationException` by the active chunk-system rewrite and
  are not treated as current runtime implementations.
- Complete `ChunkStatusTasks.java` and `ChunkPyramid.java`, and `ChunkStep.apply`
  / completion: separate persisted-status loading and generation stages,
  structures, biomes, noise, surface, carvers, features, lighting, spawning and
  conversion to a full chunk. These stages remain follow-up FerrumC work.
- Source patches for `ChunkStatusTasks` and relevant `ChunkMap` tag-upgrade
  context; feature patch `0016-Moonrise-optimisation-patches.patch`: active
  `ChunkTaskScheduler.scheduleChunkLoad`, `createTask` and `schedule`, complete
  `ChunkLoadTask` including `ChunkDataLoadTask`, `ChunkUpgradeGenericStatusTask`
  run/constructor/scheduling, and full-conversion `ChunkFullTask.run`, plus the
  applied `ChunkStatusTasks` patch. Moonrise chooses loading versus generation
  pyramids from persisted status and schedules required neighbors.

Paper loads complete serialized chunk data and has no FerrumC sparse overlay
format. Its Moonrise chunk-data task logs certain I/O/parse failures and falls
back to an empty protochunk. FerrumC deliberately refuses corrupt metadata or
overlay admission instead of silently defaulting world data. This documented
storage policy difference does not assert Vanilla/Paper differential parity.
Reviewed source is a reference, not a passing runtime oracle.

## Verification

New public tests cover explicit binding/shard construction, scope/domain
preflight, generator/stored descriptor mismatch, missing/unsupported/corrupt
metadata, already-resident refusal, malformed/unstable generator admission,
stored full/empty/nonempty overlay entity semantics, seed-0 legacy guards,
opaque same-Arc authorization/no tick I/O, wrong store/scope/identity, weak
store lifetime, metadata-await identity changes before the first world read,
and real redb close/reopen/cumulative-overlay regeneration.

Private capture tests inspect the exact fairness cursor and every chunk/ticket
fact after metadata or late codec failures, unverified legacy drain refusal,
and maximum-batch clamping with a deferred tail.

Implementation freeze results:

- `cargo test -p ferrumc-sim`: **204 passed** (161 unit, 10 loading, 19 new public
  generation admission/capture, 6 ownership, 7 harness, 1 doctest).
  Log: `/tmp/wg003-sim-test.log`.
- `cargo fmt -p ferrumc-sim -- --check`: passed, exit 0.
  Log: `/tmp/wg003-sim-fmt.log`. Owned diff whitespace check also passed.
- Root preliminary review found F1: generalizing the generator argument to
  `dyn ChunkGenerator` initially let another declared descriptor reach the
  legacy load path. A meaningful regression failed before the fix:
  `/tmp/wg003-legacy-baseline-red.log`. The fixed explicit seed-0 guard checks
  both new and already-resident acquisitions; the regression passes in the
  final full suite. This also rechecks stable identity across generation/awaits.
- Independent review found F2: the free bound loader initially entered
  `load_chunk` before rechecking generator identity after the metadata await.
  The acquire path already rechecked. A deterministic read-only metadata
  callback switches only a deliberately contract-violating generator's
  identity from seed -982 to 77; persisted metadata remains immutable. Own RED
  evidence: `/tmp/wg003-await-identity-red.log` (free loader made one world read
  instead of zero, acquire control refused before any world read).
  The minimal fix rechecks the generator at the shared composer's entrance,
  before the first storage chunk read, without changing public APIs. It covers
  both free load paths; the concrete immutable legacy FlatWorldGenerator does
  not itself permit this dynamic identity-change fixture in safe Rust.
  Two persistent public tests use an actual `yield_now().await` callback with
  no sleeps/timing assumptions and pass: `/tmp/wg003-await-identity-green.log`.
  The independent unchanged original RED harness passes 2/2 after the fix:
  `/tmp/wg003-independent-metadata-await-green.log` (original failure evidence:
  `/tmp/wg003-independent-metadata-await-red.log`). The known Rustdoc
  `FerrumC` backtick lint was corrected in the same authorized review-fix wave.
- `cargo clippy -p ferrumc-sim --all-targets -- -D warnings`: **passed**, exit 0,
  including dependencies. Log: `/tmp/wg003-sim-clippy.log`. No dependency code
  or lint suppressions were changed by this ticket. Necessary Sim style fixes
  retain the same base/generation branches and empty-batch expectations.
- Exact owned-file hashes: `/tmp/wg003-sim-freeze.sha256`.

This is a reviewed runtime implementation freeze, **not** integration or App
activation. The independent review's original F2 harness passes after the fix;
Root owns final integration and WG-004 activation. No other crate, board or Git
state was edited by this implementation agent.
