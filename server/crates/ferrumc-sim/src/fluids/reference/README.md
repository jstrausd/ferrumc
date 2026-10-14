# Ordinary water subset contract and pinned evidence

Reference: Paper 1.21.8 build 60, commit
`29c8822d90899c89d2689338e81a98f690bcba12`, verified inner JAR SHA-256
`c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`.
`provenance.json` records inspected applied sources, relevant source/Moonrise
patches and frozen export hashes.

`WaterRules::builtin()` binds original water registry states. Its `plan_tick(pos,
source_conversion, read)` accepts only a resident block reader returning
`Option<BlockStateId>`; missing cells stay closed. It does no I/O or mutation.
`WaterPlan::writes()` contains at most six original-order writes with private
`WaterWrite` fields exposed through `pos()`, `before()` and `after()`. The owner
validates every before-state, admits the complete commit, and uses its existing
mutation journal, persistence and output funnel. Earlier staged writes feed
later reads. Source conversion is the explicit original game rule, not an
inferred permission or a different source-state owner.

The pure rules preserve the original non-source update, >=2 cardinal-source
conversion with solid/source below, falling amount-8 from above, downward
preference, >=3-source downward/side exception, dropoff 1, source/falling side
rules, slope distance 4, minimum-distance selection and EnumMap side-write
order. Water levels 8..15 share the original falling fluid state; unchanged
fluid states retain their original block representation. A source written into
an admitted simple container retains its exact other properties and sets only
`waterlogged=true`; flowing water is refused by original simple-container rules.

Facts cover all 27946 original block states. Only static on-grid collision
geometry is admitted (27761 states); dynamic or off-grid passage produces
`UnsupportedCollision`. Faces come from the actual Moonrise clamped collision
API, with its original full-block shortcut. Every shape bound must align to the
1/16 grid before admission, making the 16x16 union masks exact in that domain.
Relevant lava/other fluids and custom-container writes return typed refusals.
At most 512 distinct cells are read, six cells are written, and slope recursion
is bounded by the original distance 4. No event cancellation, lava mixing,
dynamic piston/scaffolding geometry, entity fluid forces, destroyed-block drops
or complete server differential is claimed by this pure module.

`WaterTickQueue::new(cap)` admits `1..=65536` unique cells. `schedule_at(due,pos)`
coalesces to the earliest due time and deletes the old index entry immediately;
there is no stale-heap growth. `pop_due(now,limit)` transfers at most 256 cells in
canonical due/coordinate order and retains future and overflow tails. This
deterministic tie policy is explicit; original server sub-tick FIFO parity is
not claimed.

The placement/removal/neighbor hook is
`schedule_after_change(&rules, now, changed, read)`. It inspects the exact
committed cell plus six cardinal neighbors and schedules only actual resident
water-containing states at `now+5`. All seven admissions are preflighted;
QueueFull, coordinate/tick overflow and unknown state preserve both indexes.
The caller must retain a bounded retry fact or refuse its write before commit
when scheduling cannot be admitted, and must not ignore QueueFull. Removed
sources still schedule their flowing neighbors. Chunk-arrival/departure border
reconciliation and initial natural-water scheduling remain owner integration.

`ExportWaterFacts.java` exports actual fluid, solidity, holding/container and
Moonrise collision facts; it invokes the original private canHoldAnyFluid.
`ExportWaterRules.java` invokes the actual protected getNewLiquid and getSpread
methods against controlled reads. It uses JVM Unsafe only to bypass ServerLevel,
ServerChunkCache and LevelChunk constructors, replaces their data reads and
supplies a genuine original GameRules object. This creates no running server,
world, socket, plugin manager or fluid tick consumer. The 144 rows cover twelve
controlled layouts, conversion on/off and six original block levels: 144 exact
new-liquid results and 120 positive-flow spread cases (level 7 has no side flow).

Compile with JDK 21 and the verified runtime classpath, running helpers from a
private directory because Minecraft Bootstrap may create logs:

```sh
javac -proc:none -cp "$PAPER_CLASSPATH" -d "$HELPER_DIR" ExportWaterFacts.java ExportWaterRules.java
java -cp "$HELPER_DIR:$PAPER_CLASSPATH" ExportWaterFacts "$DATA_DIR"
java -cp "$HELPER_DIR:$PAPER_CLASSPATH" ExportWaterRules "$RULES_OUTPUT"
```

Four focused Rust tests compare these original method results and exercise
actual placement scheduling, flow across a chunk border, exact radius 7,
removed-source decay to empty, falling/downward writes, source merging and
support guard, simple waterlogging, queue coalescing/full atomic refusal and
typed lava/unknown-state refusal. Local verification uses a standalone
`notes/fluids-harness.rs` module declaration with the existing World/Math/Registry
rlibs; Sim lib/shard/message and other existing files were not instrumented.
Owner integration still needs its own focused journal/persistence/output checks.
