# Invariants: ferrumc-placement

- Placement and connection helpers are pure synchronous functions. They neither
  mutate world state nor schedule ticks, dispatch plugin events, or perform I/O.
- State IDs and property values come from the pinned registry. No hardcoded
  property bit masks or state multipliers are introduced by connection rules.
- `update_fence_connection_state` accepts only a recognized fence and one
  horizontal direction. It replaces exactly that connection with the requested
  boolean and preserves every other property, including waterlogging and arms
  towards unavailable neighbours. Unknown/non-fence states and vertical
  directions return `None`.
- `recompute_fence_connection_state` explicitly replaces all four connection
  booleans. Callers must have all four neighbour observations; one-direction
  neighbour notifications use the single-connection helper instead.
- Named fresh-fence computation and full recomputation share
  `NeighborQuery::is_fence_connectable_on_face`, passing the neighbour's face
  towards the fence (the opposite of the fence-to-neighbour direction). Its
  default delegates to the legacy query so existing hosts remain compatible;
  named computation keeps its default-state inheritance contract.
- `fence_connects_to` accepts only known fence sources, known neighbour states
  and horizontal faces. It applies the pinned formula
  `(!exception && caller_full_face) || same_family || perpendicular_gate`.
  Crimson/warped belong to the wooden family; Nether-Brick remains separate.
  The twelve pinned registry gate names have a horizontal facing schema checked
  against the raw snapshot and correspond to reviewed FenceGateBlock
  registrations. Gate open/in_wall/powered do not affect the gate branch.
- The caller supplies a proven state/face support fact. The helper neither
  treats `boundingBox` as a FULL shape nor replaces the caller's value, including
  for different-family fences and parallel gates. Real fence/gate horizontal
  support shapes are not full; hosts must provide `false` for that shape fact.
- Reference reviewed: Paper 1.21.8 Build 60, commit
  `29c8822d90899c89d2689338e81a98f690bcba12`, fully generated
  `FenceBlock.connectsTo`, `isSameFence`, `updateShape`, `FenceGateBlock` and
  `CrossCollisionBlock`, relevant block registrations and source/feature
  patches. Horizontal shape updates
  preserve the rest of the existing state. Water tick scheduling, support and
  full connection/shape rules remain host responsibilities; source review and
  these deterministic tests do not constitute Paper differential acceptance.
- Door hinge decisions follow the same reference's complete
  `DoorBlock.getHinge`: the ordered lower/upper left/right obstacle weight,
  lower-half pairing flags for all 21 pinned door families, then strict
  orientation-specific cursor comparisons. Exact center ties choose `left`.
  Both output halves share the resulting hinge and facing. Cursor coordinates
  remain clicked-cell-local; the hinge calculation subtracts the horizontal
  clicked-face step because `PlacementContext::position` is already stepped.
- Door obstacle observations retain the existing `is_solid_cube` proxy and
  exclude all doors from full obstacles; they do not establish state-specific
  full collision shapes. This decision correction neither adds support checks,
  powered placement/opening, neighbor physics nor Paper event parity. The pure
  helper schedules no work, reads no I/O and emits no events.
- No unsafe code, production `unwrap`/`expect`, or unbounded input allocation.
  Every public item has rustdoc.

## Placement collision geometry

- `standing_player_body` promotes the original float width/height (0.6F/1.8F)
  before constructing the body. Nonfinite bodies refuse. Only standing-player
  dimensions are authoritative in the current host; other poses are not inferred.
- `placement_intersects_body` uses genuine placement-context collision boxes,
  never visual boundingBox metadata or support/outline shapes. It allocates
  nothing and translates boxes by the final candidate cell. Positive overlap
  must exceed original shape epsilon 1e-7 on all axes; face touching is clear.
- All 27,946 state IDs/names/properties match the pinned registry before fixture
  generation. 27,902 states use 313 frozen shapes; 44 offset or moving-piston
  states refuse with None. Fresh closed shulkers and placement-empty scaffolding
  and powder snow are supported. Unknown IDs or malformed bodies also return
  None, requiring the authoritative caller to refuse before mutation.
- `data/collision/CollisionShapeOracle.java` calls the unmodified pinned Paper
  runtime's actual placement context, collision shape, EntityDimensions body,
  and Shapes.joinIsNotEmpty. It verifies jar hash, original class provenance,
  release and protocol. The differential test checks all supported states at
  16 genuine body probes each (origin and translated world coordinate), including
  touching and 5e-8/2e-7 overlap controls. See data/collision/README.md for replay.
- This fixture validates static/fresh placement geometry only. World support,
  arbitrary block-entity animation, non-player entities, alternate poses,
  canSee and Paper plugin/event behavior remain outside this geometry contract.

## Narrow short-grass replacement

- `is_short_grass_replaceable(current, held)` accepts only the pinned known
  short_grass state with a known different held block family. It follows
  Blocks.SHORT_GRASS's replaceable registration and inherited
  BlockBehaviour.canBeReplaced's exclusion of the block's own item. Unknown
  states fail closed; other vegetation and item-specific replacement remain
  outside this narrow helper.
- BlockPlaceContext in the reviewed original selects the clicked cell when its
  state canBeReplaced(context), otherwise the adjacent face cell. The host must
  read the authoritative clicked state and choose the target before placement
  hooks. This helper does not read a world, select a target or emit events.
