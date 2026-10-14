# Playable Overworld generation

`ferrumc_natural_playable` selects seed-bound revision 5. It composes the same
accepted Overworld density, stored biomes, surface rules and carvers as the
older natural presets, followed by oak/birch, grass, coal/iron/copper/gold/
redstone/lapis/diamond and the new native decorations below. Older revisions
1–4 keep their previous algorithms and persistent generation identities.
Changing the preset of an existing save is not an implicit terrain migration.

## Concrete additions

| Surface biome | Added native content |
| --- | --- |
| Taigas and grove | Spruce-shaped trees; ferns and mature berry bushes on suitable uncovered soil. Snow-covered soil can admit trees. |
| Jungles | Jungle-shaped trees and ground grass. |
| Savannas | Acacia wood/leaf trees and grass/flowers. |
| Dark forest / pale garden | Dark oak / pale oak wood and leaf trees. |
| Mangrove swamp / ordinary swamp | Mangrove / oak wood and leaf trees on exposed suitable soil. Underwater mangrove-root generation is omitted. |
| Cherry grove | Cherry wood and leaf trees on exposed suitable soil. |
| Other suitable exposed grass/dirt terrain | Short grass, poppies, dandelions, forest lily-of-the-valley, swamp/dark-forest mushrooms. |
| Desert and badlands sand | Conservatively admitted cactus columns. |
| Exposed suitable soil/sand beside water | Sugar cane with actual adjacent water at ground level. |

The added trees use small native crown/trunk shapes, not the original species'
complete trunk/foliage/root/decorator processors. Existing oak/birch keeps its
accepted kernel. This implementation does not claim Vanilla seed parity,
fallen/mega trees, tree-grown beehives, creaking hearts, leaf litter, bamboo,
all double-height plants, crop/random-tick growth or a complete FEATURES stage.
Deserts, oceans, rivers, beaches, barren badlands and treeless mountain biomes
do not acquire an artificial universal tree layer. On unsupported soil, steep
terrain, obstructions or underwater land, an attempt can be refused whole.

## Structures and loot

One potential surface-structure source is selected per signed 8×8 chunk region.
Its local source offset is clamped to 1–6, leaving enough separation between
neighboring regions for complete structure rectangles not to overlap.
The actual footprint must satisfy the admitted biome, slope, ground and complete
clearance predicates. An unsuccessful candidate does not create partial walls.

- Desert-well shells have sandstone foundations, five source-water cells,
  sandstone/slab edging, four pillars and a slab roof. The complete pinned
  original implementation informed this shell. Suspicious sand, brushable
  block entities and archaeology loot are not generated.
- Swamp-hut shells use the pinned 7×9 footprint, supported corner logs, spruce
  platform/walls, oriented corner/edge stairs, fences, potted mushroom,
  crafting table and cauldron. The native terrain policy uses a conservative
  maximum-ground placement instead of the original average-ground algorithm.
  Witch/cat structure entity spawning is not implemented.
- Abandoned cabins are original FerrumC structures with cobblestone foundations,
  oak walls/supports, damaged roofs, windows, a crafting table and a real typed
  chest. Its checked deterministic loot consists of bread, coal, iron nuggets,
  wheat and oak saplings. The chest is handled by the ordinary owner inventory
  and complete chunk-overlay persistence, not an extra loot-on-open grant.

There are no generated villages, strongholds, mineshafts, monuments, trial
chambers, desert/jungle temples, ruined portals or a complete jigsaw/template
processor system. The new cabins are not presented as replacements for those
Vanilla structures.

For seed 0, one deterministic admitted cabin chest is at **(-150, 97, -149)**.
Its footprint crosses a negative chunk seam. This is a generation witness,
not a promise that every spawn contains an immediately visible structure.

## Ordering and resource bounds

The native stage reads immutable raw terrain and its surface stored-biome quart.
It does not claim the original zoomed placed-feature biome predicate. Plans are
fully evaluated per source and projected from a 3×3 source neighborhood in
coordinate order. Query order, worker scheduling, cache eviction and restart
cannot change the projected blocks or original chest loot. Trunks replace
competing source leaves, while foliage never erases logs. Complete tree crowns
are excluded from potential structure clearings before placement, so structures
do not cut trunks under floating canopies. Failed structures retain the small
conservative clearing.

One source admits at most nine raw chunks, 1,048,576 charged work units, 8,192
writes, six supplemental tree attempts and 192 botanical attempts. Heightmap
admission includes a worst-case 98,304 block-read charge per raw chunk. Plans
retain at most 32 FIFO entries and eight MiB, including conservative node,
chest, Arc, deque and cache-structure charges. The separate established raw/
climate/tree/ore limits remain in force. Unknown states or exhausted work/memory
caps fail generation; there is no flat-terrain substitution.

Cactus column admission checks every layer and every horizontal neighbor against
raw terrain and the local plan before any write. Even-coordinate origins prevent
neighboring native cactus columns across independently projected source seams.
Sugar cane requires actual neighboring raw water at the support level. These are
conservative static generation checks; complete subsequent plant physics is not
claimed.

## Reference and validation scope

Reference: Paper 1.21.8 build 60, commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Complete generated original server
methods read: `DesertWellFeature.place`, `SwampHutPiece.postProcess/spawnCat`,
`RandomPatchFeature.place`, `RandomSpreadStructurePlacement`, complete
`VegetationFeatures.bootstrap`, `SugarCaneBlock` and `CactusBlock`, with the
applicable source patches and a search of feature patches. Reference paths,
sizes and hashes are recorded in the work evidence `reference.json`.

Original Paper uses ordered placements/configured features, its own salted RNG,
registry/tag predicates, structure persistence, brushable loot and creature
spawn events. FerrumC's added stage uses its own seeded source policy and
static admitted shell subset. It invokes no Java plugin generation, creature
spawn or grow events. Source review is not a passed Paper differential run.

Focused native tests exercise signed source seams and seeds, complete typed
chest loot, whole obstruction refusals, species routing, genuine water-dependent
sugar cane, upper-layer/planned cactus collision refusals, shell corner geometry,
explicit limits and complete real pipeline output. The real cabin test compares
all projected force writes/chest contents and complete generated chunks across
fresh generators, reversed chunk request order and genuine cache-miss FIFO
publication/recomputation. Unrelated retained entries are instrumented solely to
force cache eviction and are explicitly distinguished from generated fixtures.
Minecraft graphical rendering and full survival/plugin parity are separate
acceptance work.
