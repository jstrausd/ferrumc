# Versioned oak and birch tree preview

Starting point: `21b40fd63e7986276083418a2071cbfb229e20f0`.
Minecraft Java 1.21.8 / protocol 772. Original reference: Paper build 60,
commit `29c8822d90899c89d2689338e81a98f690bcba12`, actual inner JAR SHA256
`c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`.

## Delivered behavior

`ferrumc_natural_tree_preview` selects natural algorithm 2 revision 3, with
terrain, biomes, surface, carvers, an explicit oak/birch tree subset and grass.
Old revision-1/2 saves retain their identities and generated baselines. A new
world directory is required; no existing world is silently relabelled.

The supported shapes are the original undecorated configured `oak` and `birch`
TreeFeature kernels. Outer placement preserves original counts, global step-9
indices 24/25/52, decoration seeding, modifier order, biome membership, live
heightmap predicates, water-depth and sapling checks. Straight oak/birch selector
outcomes run the undecorated shape; fancy/fallen outcomes are omitted, and bees
and leaf-litter decorators are omitted. Those omissions change later RNG and
source history compared with the complete original route. This is a versioned
subset, not full Paper FEATURES or full tree-population parity.

Each source evaluates its full raw halo and all its attempts before projection;
trees precede grass within that history. Targets combine the canonical 3x3
source neighborhood. Conflicts prioritize logs over soil, leaves and grass,
with equal priority resolved by source-coordinate order. A foreign crown cannot
erase another source's trunk. Per-source original leaf distances are retained;
they may exceed distance to an overlapping foreign tree. Leaf decay, scheduled
leaf/fluid tick execution, other tree species and complete lighting are open.
Raw terrain and source-output caches are bounded and seed-bound. Generated trees
are baseline content, not edited overlays; player edits remain overlays on the
exact revision-3 baseline.

The shape kernel checks both sides of exposed faces. It applies original
short-grass removal and snowy-soil flag changes within its closed state domain;
unsupported snow removal refuses the complete output. Scheduled leaf ticks
are compared as a deduplicated set, without claiming original iteration order
or active gameplay ticking.

Revision-3 spawn scans a bounded Overworld column to dry support below the canopy,
stops at fluids/unknown obstructions, rejects leaves/logs as support and checks
two clear body cells. It centers the player within the checked column. Earlier
revisions keep their coordinate and surface-selection behavior.

## Verification

Author reference coverage includes 1,260 actual original-JAR configured shape
cases, 576 actual original-method placement cases using an explicitly modified
subset wrapper, and 27,946 original state rows covering 628 admitted states.
Normal and optimized Python reimports reproduce the frozen fixtures byte for
byte. The wrapper is not evidence of complete original population parity.

Independent review accepted the frozen 395-file World source with no open
findings. Five independent Release probes compare old revision-1/2 block and
biome baselines, forward/reverse/concurrent cold generation, cache isolation,
empty generated persist masks, a negative-coordinate crown seam, and private
snow/grass/soil boundary instrumentation.

Root checked the combined accepted baseline and exact frozen World/App/Config
sources in a private copy, without unrelated workspace work in progress:

- Complete World Release tests: **435 passed, 7 existing ignored**.
- Complete App and Config Debug tests: **430 passed, 1 existing ignored**.
- Focused World tree Debug tests: **12 passed, none ignored**.
- Workspace formatting and all-target workspace Clippy with `-D warnings`: pass.
- Standard locked/offline Release application build: pass.
- Public World/Config rustdoc with warnings denied: pass. App rustdoc exposes
  preexisting links to private items; blanket workspace rustdoc success is not
  claimed. A cancelled App Release-test compilation never ran tests and is not
  counted as acceptance.

The standard deployment executable has SHA256
`d7520f5103af4439a01301397f86acb165993f3c2bd583218b72615f96322215`.
An independent protocol client on seed 0 received all 169 chunk columns at
view distance 6, with 3,418 oak logs, 35,239 oak leaves, 1,087 birch logs and
9,387 birch leaves. It observed a six-block oak trunk at `(-14,64,-1)` with
62 sampled crown cells spanning the negative Z seam. Player-body placement
refusals, creative block placement and authoritative removal of one tree log
passed. After graceful stop/reopen, all 169 edited block and quart-biome column
snapshots matched, including the removed log. This is protocol-level acceptance,
not a visual Minecraft-client or complete Survival/Paper-server comparison.

Complete initial views took **173.439 seconds** and **172.998 seconds** after
restart. The destination view after teleport took 29.367 seconds. These timings
measure receipt of the entire view, not time to the first visible chunk. Cold
generation/streaming remains slow; no performance completion is claimed.
Unedited baseline columns are regenerated after restart, while saved edits are
reapplied to their versioned baseline.

The client waits for destination chunk readiness and two observed live tick
updates before harvesting after teleport: movement and editing have distinct
simulation tick boundaries. An earlier immediate-dig attempt was refused at the
old position and is retained as failed evidence. A supplemental test-feature
executable is recorded separately; final acceptance uses the standard binary
hash above.

Exact logs, manifests, independent review and client results are retained under
`.codex-tmp/trees-preview-20261013/`, including `root-checks.json`,
`review/assessment.json`, `root-binary.json` and
`client-tree-standard-view6-r2/acceptance.json`. Other tree species, decorators,
leaf decay, sapling growth, complete lighting and complete Survival remain open.

See [local commands and limits](LOCAL_NATURAL_WORLD.md).
