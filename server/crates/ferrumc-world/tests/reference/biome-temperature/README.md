# Fixed biome temperature reference

The immutable approved GEN-05.7 authority is copied under `fixtures/authority/`.
`import_originals.py` is a fresh-output mechanical converter; it contains no
numerical temperature/noise/predicate expectation formula. It validates the
approved SHA pins and all119 oracle,64 design and757 actual input files before
and after import. Original Java/JDK/runner/guard expectations retain their
original canonical locations and exact bytes. To reproduce genuine original
calls, follow the copied canonical `oracle/run_oracle.py` and `check_guards.py`
using the frozen inputs under `.codex-tmp/gen05_7/`; those calls need the pinned
original JARs/libraries/JDK. This implementation does not invent a new receipt
or pretend that copying these files executes Java again. Root's genuine run and
independent CLEAN review are the authority for the imported outputs.

Run the importer from the repo containing those immutable inputs:

```sh
python3 server/crates/ferrumc-world/tests/reference/biome-temperature/import_originals.py \
  --repo . --out /fresh/fixture-directory
```

`temperatures.tsv` preserves all original tokens for129588 rows:93584 `S`,80
`M`,3156 `C`,32768 `F`. Each starts with the added kind, then the exact original
six case-identity columns, the remaining Vanilla columns, then the remaining
Paper columns. For `S`/`F`, the remaining columns are private temperature,
modifier, public temperature, warm/cold/melt/precipitation, and (for `F`) three
actual original noise observations. Vanilla's unexported public/predicate
columns remain literal `-`; they never become fabricated expectations. `M`
retains private mutable/immutable plus Paper public mutable/immutable results.
`C` retains actual packed key, private, retained public and cold public for each
mode. Only enormous full FIFO strings are omitted from this test projection;
complete originals remain independently pinned and every difference plus
selected genuine FIFO snapshots remains in `diagnostics.json`.

Rust tests compare exact original private temperatures and actual Paper public
results/predicates; all original modifiers and noise observations are compared.
They retain all case labels, source identities and distinction columns. Metadata
and direct noise are byte-identical across originals and imported once with both
original hashes. Both constructor files remain separate: five full original512-
entry permutations, offsets, factors, counts and final Legacy states. Rust's
used256 entries and unused-original zero half are checked. Original draw-state
comparison reuses the unchanged Rust Legacy/Simplex constructor, verifies the
original count through an independent consumed stream and admits the genuine
exported final state by inverting only the existing seed scramble.

All15 actual Paper offsets differ because its nextDouble executes l2f/fmul/f2d,
where Vanilla executes l2d/dmul. Existing Rust Simplex offsets match Vanilla;
these fixed noises only sample offsets=false. No Paper offset construction bit
parity or generic offset API is claimed. Vanilla's57 public-cache differences
remain explicit diagnostics; Rust never imitates that cache. Frozen coverage is
19296modified,13024outerfalse,448innerfalse across the complete declared grid.

## Allocation and native scratch measurement

`biome_temperature_allocation.rs` is safe Rust and prepares all requests before
measurement. The ignored test is run explicitly in Debug/Release through
`measure_allocations.py`. Its external test-only Linux/glibc preload library
counts malloc/calloc/realloc/alignment allocations and frees on the measured
thread between raw stdout boundaries. A positive4096-byte allocation control
must be observed before accepting zero calls in three successful/three refused
compiles,100clones and388764 original queries. There is no new Cargo dependency,
production allocator, unsafe Rust, or mutable production global.

```sh
python3 server/crates/ferrumc-world/tests/reference/biome-temperature/measure_allocations.py \
  --binary /exact/built/biome_temperature_allocation-test-binary --out /fresh/report
```

The same external probe reports observed stack-write high water. It marks only
unused thread-stack memory below a protected16KiB marker-frame margin, then
checks writes after the operation. This measures observed writes, not every
unwritten compiler stack reservation. Separate `size_of` capacity accounting
covers every fixed parser/frame/buffer/table/noise and conservative compiler
initialization/move temporaries; a dedicated test constructs/clones/queries on
an explicit1MiB stack. The pinned64-bit layout retains2232bytes and conservatively
admits570216bytes simultaneous native scratch. Results are binary/toolchain
specific and establish no RSS/TPS/latency or live-world acceptance.
