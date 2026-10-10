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
- Named fresh-fence computation and full recomputation share the existing
  `NeighborQuery::is_fence_connectable` evaluation without broadening its
  predicate or collision-shape semantics. Named computation keeps its existing
  default-state inheritance contract.
- Reference reviewed: Paper 1.21.8 Build 60, commit
  `29c8822d90899c89d2689338e81a98f690bcba12`, fully generated
  `FenceBlock.updateShape` and `CrossCollisionBlock`. Horizontal shape updates
  preserve the rest of the existing state. Water tick scheduling, support and
  full connection/shape rules remain host responsibilities; source review and
  these deterministic tests do not constitute Paper differential acceptance.
- No unsafe code, production `unwrap`/`expect`, or unbounded input allocation.
  Every public item has rustdoc.
