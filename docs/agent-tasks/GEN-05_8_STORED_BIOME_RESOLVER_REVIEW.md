# GEN-05.8 — Root review of the stored-biome resolver contract and oracle

Independent review, 11.10.2026. The reviewer did not author the candidate.
Written only here and in `.codex-tmp/gen05_8-root-review/**`; no Git, no
production, brief or `.codex-tmp/gen05_8` change. Machine-readable result:
`.codex-tmp/gen05_8-root-review/final-assessment.json`.

## Verdict: CLEAN_CONTRACT (with conditions for the runtime work order)

No blocking finding. The oracle is genuine and reproduces byte-for-byte. The
contract matches the work order. Five conditions (A1–A5) belong in the separate
runtime ticket. Eight hints (H1–H8) are non-blocking text corrections.

Reviewed candidate:

| Item | SHA-256 |
| --- | --- |
| Brief `GEN-05_8_STORED_BIOME_RESOLVER.md` | `75ffda46a8f54c66061b715b92ceebe91c34e5047020e05b2606e59999563765` |
| `delivery/freeze-manifest.json` | `d838de6c3af6d512d9b47741d8ed2be8b476337885bb0487d1a51435ea296cee` |
| `delivery/final-compare.json` | `ce9978d8b28d0385694e01874c0fe94d59a02eb48508652a92b7aa8401e76540` |
| `oracle/input-pins.json` | `bffbed18ac9f64025c5defcafb5df1b9ffec25191c2fb86c71ff319124fe50d8` |

After the review, all 211 authoritative freeze entries were hashed again and
still match. No `__pycache__` was created in `.codex-tmp/gen05_8`.

## 1. Fresh reproduction (executed)

Method: `runner/` holds private copies of `run_oracle.py`, `check_guards.py`,
`project_originals.py` and `StoredBiomeOracle.java`. The helper is
byte-identical to the pinned source. The patch is in `runner.diff`. It changes
only three things: the output area becomes the review directory, the
canonical-helper check uses the pinned digest instead of the path, and pins,
design, mappings, aliases and library origins are read from the frozen
originals. JARs, JDK and archive are unchanged and pinned.

| Run | Exit | Result |
| --- | --- | --- |
| `python3 run_oracle.py repro-normal` | 0 | All 28 differential files are byte-identical to `final-normal` and `final-optimized`; `manifest.json` is byte-identical too (`6d90ba8f…`) |
| `python3 -O run_oracle.py repro-optimized` | 0 | Same result |
| `python3 check_guards.py` (normal) | 0 | 15/15 refused; `guards.json` byte-identical (`6b9359b4…`) |
| `python3 -O check_guards.py` | 0 | 15/15 refused; byte-identical (`ec5e1404…`); the only difference from normal is `entrypoint_python_optimize` |

The timestamped bootstrap logs (`logs/*`, `paper.log`, `vanilla.log`) are
listed only in `diagnostics-manifest.json`. They were not compared, and the
freeze treats them as non-differential.

Pins against the originals (executed). Preflight checked all 2173 inventory
digests, the 1263 archive entries and the 454 JDK entries before and after each
export. Independent provenance:

- Vanilla bundler SHA-1 `6bce4ef4…` equals `downloads.server.sha1` in Mojang's
  `version.json`.
- The inner jar `2bee159e…` is listed in the bundler's `versions.list`.
- Paperclip `8de7c52c…` equals the official checksum for build 60 from
  `fill.papermc.io` v3 (fetched 11.10.2026).
- The inner jar `c64fb7db…` is listed in paperclip's `versions.list`. Its
  manifest shows `Build-Number: 60` and commit `29c8822`.

## 2. Oracle authenticity (source review plus executed outputs)

- Values come from actual getters. `StoredBiomeOracle.java:146-167` reads the
  chunk, section and palette getters on real `ProtoChunk`s (CodeSource is
  checked) and records `Strategy.getIndex`. Inputs are populated only through
  `PalettedContainer.getAndSetUnchecked` (`:177-184`). Nowhere does a getter,
  clamp, index or zoom formula appear as an expected value.
  `project_originals.py` only copies outcomes.
- Queries are genuine `BiomeManager` queries through actual `LevelReader`
  default methods (`InvocationHandler.invokeDefault`, `:295-316`).
- `createBiomes` uses the actual WorldPreset, RandomState, StructureManager and
  Blender paths (`:241-282`).
- Vanilla and Paper run on different threads, and the harness handles this
  correctly:
  - Base `NoiseBasedChunkGenerator.java:76-81` uses
    `Util.backgroundExecutor().forName("init_biomes")`. `TracingExecutor.forName`
    returns the same `service` outside the IDE and Tracy.
  - Paper uses `Runnable::run`.
  - The harness reads, resets and finally reads the ThreadLocal `RTree.lastResult`
    in Vanilla on the same pool worker (`generationLane`, `:31-37`). The
    structure callback checks that `createBiomes` ran on exactly that worker
    (`:268`). Paper reads directly on the calling thread.
- The 21 Vanilla/Paper differences are diagnostics only (executed analysis of
  `exceptions-differences.jsonl`). Values, queries and histories show 0
  differences. The 21 rows are:
  - 10 value rows on unaligned accessors (-63/17 ×7, 1/17 ×3);
  - 10 accessors with height 0 (Vanilla wraps them in `ReportedException`; for
    negative Y Paper fails at index 0 instead of -1);
  - 1 injected null-section fault (see H2).
  No row lies in the admitted `BiomeQuartGeometry`.
- The controlled fallback hook is cleanly separated. All 5568 rows per mode:
  - come only from `axis/0..6`;
  - have outcome `controlled_null_hook`, resource `null` and `fallback_calls=1`;
  - in every case select a chunk outside the center's 3×3 window.
  No `createBiomes` row hits the hook; the projection refuses that
  (`project_originals.py:83-85`).
- The 37 Paper arrays match the accepted .5 evidence (re-checked independently
  against `arrays-compact.jsonl`). Whole values, quart membership and final
  ordinal are equal, with 1–4 distinct biomes per array. Vanilla arrays equal
  Paper arrays.
- The zoom seed is derived once. For world rows, `zoom_bits` equals the first 8
  LE bytes of SHA-256(seed LE) for 0, -1 and i64 min/max, and matches .6
  `hash-0`. Obfuscated controls pass through unchanged.
- No WorldGenRegion proof is fabricated. Manifest, receipts and brief name
  helper scope only (`brief:323-332`).

## 3. Contract against the work order (source review)

| Point | Assessment |
| --- | --- |
| Only the complete producer; identity and geometry | Correct. Only sealed `ChunkBiomeSamples` (private fields, no Clone or constructor, `arrays.rs:76-86`). Identity is made of existing checked types. Histories are checked only within each array; they legitimately differ across chunks (`brief:146-153`). |
| Zoom seed | Correct. `BiomeZoomSeed::from_world_seed(identity.seed())` once; no caller zoom seed (`brief:70-74`). |
| Signed chunks, mask, clamp, index | Correct. Matches the .5 storage `((qy-minQy)*4+x)*4+z` (`arrays.rs:107-111`). The last execution slot equals the last storage slot (`arrays.rs` fill order section→x→y→z). Vanilla clamp `ChunkAccess.java:437-450` and Paper section shortcut `applied ChunkAccess.java:508-525` are equivalent on aligned geometry. |
| Block, quart and resource IDs | Kept separate (`StoredBiomeQuartPos`, `BiomeQuartPos`, `ClimateQuartPos`; compile_fail tests required). |
| .6 i32 vs .5 domain | Kept separate. Range arithmetic checked: `admit_chunk` → [-1_875_000, 1_874_999]; surface centers [-1_874_999, 1_874_998]. |
| Typed refusal | `OutsideNeighborhood` → `CoordinateDomain` → `MissingChunk`; no Plains or nearest-chunk fallback. |
| Capacity and N-1/N/N+1 | Work formula `2+10N+values` checked (76 / 18524 / 18533). Borrowed and retained memory are counted separately. |
| Surface BIOMES vs applyCarvers | Separate (`NoiseBasedChunkGenerator.java:219-256`; `withDifferentSource` only in `applyCarvers`). |
| 3×3 neighborhood | Confirmed. Base and Paper `ChunkPyramid` SURFACE: `STRUCTURE_STARTS 8`, `BIOMES 1`; the only diff is `LIGHT` (paperweight base/applied equal the pinned copies). `WorldGenRegion.getChunk` (`applied:143-176`) throws for any distance ≥2 with BIOMES. Differentially, the union of the 20 groups uses all 9 offsets; 4 groups use all 9; there are 0 G queries outside the center column. The design argument (block → quart `[4cx-1,4cx+4]`) checks out. |
| MultiNoise history check | Correct and O(1). Ordinal < `parameters().len()` (7593/5), `entry.biome()==selection.biome()`, final biome equals the last slot. Only defensive, since .5 seals the arrays. |
| FerrumC policy | Block X/Z ±30M and the full halo are labelled as policy (`brief:183-186`, `432-433`). Correct, because the originals delegate at ±30_000_001 (`fulli32` rows). |

## 4. Honesty of claims

Layout, scratch and allocation figures are labelled as a design census
(`brief:233-243`, `428-429`; `design_layout.rs` prints the same scope). The
reviewer did not re-execute the layout probe. There is no runtime, Survival,
TPS, RSS or latency claim (`brief:434-437`). The forbidden-claims scan applied
`markdown_blocks`/`scan` from `scripts/check-forbidden-claims.py` to brief and
task (working tree, no Git index, no self-test because that needs `git init`):
0 findings.

## Findings by severity

### Blocking

None.

### Conditions for the runtime ticket

**A1 — `admit_chunk` reuse is not unambiguous.** `brief:109-111` and `:162`
require ".3 `admit_chunk`". The function is private in a private module
(`ferrumc-world/src/density/chunk.rs:563`; `density.rs:8` `mod chunk;`, not
re-exported). `brief:31-33` allows "one resolver module" but does not mention
`lib.rs` registration (`lib.rs:36-56`) or a visibility change. Failure scenario:
the runtime agent either edits a .3 file outside its authorization, or copies
the predicate, which can drift silently. The runtime ticket must decide
explicitly. Either allow `pub(crate)` on `admit_chunk` plus `pub mod` in
`lib.rs`, or require a separate predicate with a boundary-differential test
against `admit_chunk` (-1_875_000/-1_875_001/1_874_999/1_875_000).

**A2 — The compact fixture projection must be specified as a mechanical
derivation.** Requirements:

- source: the frozen `final-normal` projection by hash;
- Vanilla/Paper deduplication only when equality is proven (0 differences);
- every `value` row with resource, delegated quart, chunk and zoom bits;
- the 5568 `controlled_null_hook` rows mapped to FerrumC `OutsideNeighborhood`
  (all lie outside the window, as executed above), marked as policy and not as
  original equality;
- the 32 exception rows not as runtime equality but as evidence of excluded
  geometries;
- a validator that rebuilds the compact form from the full projection and
  compares hashes.

Without this the brief does not unambiguously define which rows count as the
"approved corpus" (`brief:265-266`, `424-427`). This is a permissible follow-up
condition, not a blocker.

**A3 — Zoom semantics forwarding is not covered discriminatingly.** The corpus
has 0 Vanilla/Paper query differences (`brief:383-385`); .6 itself has only 5
(`biome-zoom-original-1.21.8/projection.json`). Failure scenario: a runtime that
wrongly hardcodes `Vanilla1_21_8` in `biome_at_block` passes the whole .8
corpus. Required: a test that runs the 5 .6 divergence positions through the
same selection/lookup composition (cfg(test) kernel with distinct values at both
quarts) and gets different results.

**A4 — Error kind and payload of the block policy refusal are not specified.**
`brief:183-186` defines order (X, then Z) and range, but not the kind
(presumably `CoordinateDomain` with `block()`) and not whether `chunk()` stays
empty. Failure scenario: the tests and the API pick different kinds. The runtime
ticket must fix this.

**A5 — Disclose the controlled Vanilla history environment.** The runner
starts Vanilla with `-Dmax.bg.threads=1` (`run_oracle.py:106`); the helper
enforces parallelism 1 (`StoredBiomeOracle.java:385-386`). Shared Vanilla
histories are therefore single-worker sequences. With several background
workers, production Vanilla would give per-thread histories. The brief does not
mention this (`brief:336-338`). Runtime evidence should name the setting so
that "shared forward/reverse" is not read as Vanilla production
scheduling.

### Hints

- **H1** `brief:424`: "about 36 MB of query JSONL per mode" is the size of
  `queries.tsv` (36.7 MB). `projection/queries.jsonl` is 86.9 MB per mode, plus
  10.9 MB `arrays.jsonl`.
- **H2** `brief:374-379`: the null-section row does not lie "outside
  `BiomeQuartGeometry` admission" (geometry -64/384). It is an injected fault
  that a valid ProtoChunk cannot reach.
- **H3** `brief:286`: "invalid allowance 0/1/1048576/1048577". Here 1 and
  1048576 are valid budgets (failing work admission at most); only 0 and
  1048577 are `InvalidBudget`.
- **H4** `brief:201-205`: the surface checks the halo only after the full value
  scan and seed hash. The order is deterministic and allowed, but an incomplete
  halo costs the full scan; this is worth a sentence in the runtime ticket.
- **H5** End arrays are not checked to contain only the five End IDs; this is
  optional and defensive.
- **H6** Fallback producer: in the original SURFACE context, `WorldGenRegion`
  never calls the hook; it throws (`applied WorldGenRegion.java:161-175`). The
  typed refusal is therefore the faithful analogue. The missing fallback
  producer affects only later non-worldgen consumers and does not block.
- **H7** The 3×3 differential evidence is complete only in the union of groups
  (seed-0 cold groups use 6 offsets). The design argument carries the claim;
  the wording "from its own nine real chunks" (`brief:99-102`) is correct but
  does not say that all nine are exercised.
- **H8** Not reproduced: layout probe build and the `resume-audit` layer
  reconstructions. For ChunkPyramid/ChunkStatusTasks the reviewer compared the
  copies directly against the paperweight base/applied trees.

## Runtime readiness

Together with A1–A4 the brief suffices for exactly one production crate
(`ferrumc-world`). It covers API, refusal order, budget formula, test
boundaries, compile_fail separation and measurement model concretely enough.
The compact fixture projection (A2) and the missing fallback producer (H6) are
permissible follow-up conditions; neither blocks design acceptance.
