# Actual original oak/birch reference and explicit placement subset

Authority: Minecraft 1.21.8, Paper build 60, commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Original inner-JAR SHA256
`c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`.
`reference-manifest.json` pins original runtime/JDK inputs and inspected source
bytes as well as these fixture/helper/resource files. The helper compiles freshly
in an isolated temporary directory and verifies the original inputs before and
after invocation. Its origins record distinguishes original classes from helper
classes; original JARs never contain the helper namespace.

Reproduce with `FERRUMC_REFERENCE_ROOT=/path/to/ferrumc python3 oracle/run.py`
from this directory. The pinned reference-root runtime and JDK inventories from
`.codex-tmp/gen06_1/oracle/input-pins.json` are required. The original runner
uses original registries, builtin pack resources, normal Overworld biome membership
and actual original classes. The helper observes final states and RNG/ticks; Rust
output is never fixture input. `oracle/import_facts.py` mechanically exports u16
flags and verifies sapling species agreement and every leaf-distance ID mapping.
Normal and Python `-O` exports are compared byte-for-byte separately.

`original.tsv.oak` and `.birch`: 1260 total configured TreeFeature cases, six signed
seeds, five bottom/top/seam Y positions, 21 ground/obstruction/leaf/log/fluid/snow
and grass scenarios. These compare result boolean, full original int draws/final
RNG, all 588 final states and leaf-tick membership. The floating-grass scenario
exercises original non-leaf shape removal. Snowy-grass and snowy-mycelium exercise
original soil shape changes. Unsupported floating snow remains a typed refusal
covered separately by an original source-proven witness; no parity claim for it.

`original.tsv.placement`: 576 genuine original-method cases over four signed seeds,
three signed source coordinates, four actual biomes, six supports and two slopes.
The helper constructs an explicitly modified original RandomSelectorFeature:
supported original undecorated oak/birch configurations replace decorator variants,
and original NO_OP replaces omitted fancy/fallen kernels. Original modifiers and
top-feature identity are retained. Full original mixed RNG draw traces and full
source changed-state histories are compared. Full Paper route booleans/RNG/output
are not claimed. The active grass phase follows trees in the same source history.

`original.tsv.facts`: all 27946 builtin original block states. Original predicates
include air, blocksMotion, replaceable-by-trees, LOGS/LEAVES/dirt/SNOW tags, oak and
birch sapling survival, persistent/leaf distance variants, source-water and
SnowLayer survival. An explicit inert terrain/log/leaf closed domain admits 628
states. Snow unsupported-removal refuses; grass and snowy-soil updates use actual
original state changes. Delayed fluid/falling/leaf ticks are not executed.

The active target union is deterministic FerrumC isolated-source policy with
log/soil/leaf/grass precedence, not original full feature scheduling. Full route
decorators, other trees, structure contexts and live plugin/Survival behavior
remain outside this delivery.
