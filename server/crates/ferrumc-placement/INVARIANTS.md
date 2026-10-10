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
- No unsafe code, production `unwrap`/`expect`, or unbounded input allocation.
  Every public item has rustdoc.
