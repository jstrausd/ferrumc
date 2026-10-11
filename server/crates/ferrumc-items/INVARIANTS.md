# Invariants: ferrumc-items

- The authoritative `ItemStack` count is checked against its registry item's
  maximum before trusted network emission and storage encoding. A
  `max_stack_size` component cannot raise that registry ceiling.
- Creative input normalization is deliberately lossy (clamping and stripping).
  Persistence never calls it, never silently strips a patch, and never clamps
  a count.
- Persistence schema 1 is independent of Minecraft slot framing: `FCIS`, a
  big-endian `u16` schema, an empty/present byte, a bounded canonical item name,
  count, bounded patch counts, and explicitly tagged, length-framed additions
  followed by removal ids. Its component ids and opaque payload semantics are
  pinned to Minecraft 1.21.8; changing that model requires a schema migration.
- The storage decoder accepts exactly one record. Unknown schemas/tags,
  trailing bytes, truncated fields, out-of-range ids, duplicate patch ids,
  addition/removal conflicts, invalid typed values, and exceeded budgets fail
  with classified errors before the corresponding allocation.
- At most 64 patch entries, 8192 payload bytes per component, and 256 KiB of
  combined component payloads are allowed per stored stack. The entire record
  is bounded by `MAX_PERSISTED_STACK_BYTES`. These caps are declared encoded-byte
  caps, not decoded-heap caps. Inventory/chunk callers must separately bound
  aggregate records rather than multiplying the whole per-stack allowance.
- Typed NBT uses bounded file-form NBT with an empty root name. Encoding performs
  an allocation-free size/structure preflight because the shared NBT writer
  does not enforce every configured output resource limit. Decoder limits and
  preflight limits agree. Modified UTF-8 length, homogeneous lists, sequence
  counts, and nesting depth are checked before output allocation.
- Storage preserves supported model representation and order, including opaque
  components with modeled ids, signed damage values, in-range removals (even
  removal of a dangerous type), duplicate names inside NBT compounds, NBT key
  order, and exact float bits. NBT compound duplicates are distinct from invalid
  duplicate component-patch ids.
- Opaque bytes originate from the checked server-authored passthrough type;
  their internal wire semantics are not newly interpreted by persistence.
- `encode_item_entity_metadata` performs the unchanged lossless model preflight
  before present-AIR refusal, exact complete trusted slot sizing, or allocation.
  The whole DATA_ITEM entry including index/serializer/end is capped at 256 KiB.
  Canonical empty is `08 07 00 ff`; present-AIR is unsupported without rewriting.
- Metadata uses one fallible exact output reservation and checks append bounds;
  typed NBT writes directly through the checked shared network-root producer.
  No source clones, temporary NBT output, normalization, component reordering,
  or inner-schema/sanitizer admission for trusted opaque bytes is introduced.
  Metadata bytes grant no entity, durable revision or publication authority.
- Pure split/transfer planning validates both models before any result clone,
  including no-op and incompatible transfers. It shares persistence's
  allocation-free structural/count/encoded-byte preflight; preflight scratch
  uses a fixed 64-entry size array and a fixed 96-id set.
- `persisted_stack_size` returns the exact complete schema-1 FCIS record size
  through the existing lossless preflight, without encoding or allocation.
  Empty has seven bytes; the present count has fixed width. This is not a
  decoded heap allowance, slot size, or aggregate budget reservation.
- `same_stack_representation` preflights source then destination before any
  boolean decision, even for incompatible or empty inputs. It reuses the
  transfer's exact ordered representation comparison without encoding,
  allocation, normalization, or cloning. Valid counts are ignored only after
  checking; two canonical empty match, empty/present differ, and present-air
  remains present. Source failure wins when both inputs are invalid.
- A transfer never swaps/overwrites incompatible stacks. Item id plus exact
  ordered model representation defines merge compatibility: variant tags,
  opaque bytes, removals, NBT compound duplicates/order and float bits survive.
  Identical NaN bits match; distinct NaN payloads and signed zeros do not.
- Requests are bounded `u8` upper quantities. Total destination capacity is
  independently capped by the registry maximum. An existing destination above
  supplied capacity is an error, even for a zero request; no count is clamped.
  Present-result counts and component quantities are conserved, while drained
  results restore canonical empty.
- Transfer plans are owned snapshots, not revision-checked commits or durable
  transactions. The consumer must publish both outputs under exclusive
  authority, supply gameplay/slot capacity policy, and separately implement
  persistent atomicity/replay protection and aggregate memory limits. Encoded
  component byte caps do not equal decoded/clone heap caps.
- No unsafe code, no production `unwrap`/`expect`, no unbounded untrusted
  allocation, and rustdoc on every public item.
- The closed normal crafting catalog admits only the original 673 shaped and
  295 shapeless recipes. Real tags/remainders/fixed result patches come from the
  pinned original JAR producer; item/tag suffix heuristics are forbidden.
- Crafting grids are borrowed 2×2/3×3 slices with at most nine slots. Every
  stack passes the lossless persistence preflight before any clone; present-air
  is a typed refusal and is never silently turned into empty.
- Shaped input/pattern trimming, inner holes, original offsets and horizontal
  mirrors agree with the original. Shapeless matching finds a complete
  bipartite assignment; stack totals cannot replace occupied positions.
- Normal ingredients match item membership regardless of patches. Each craft
  consumes one item per occupied position, preserves surviving patch
  representation, drains to canonical empty, and copies only the fixed output.
- Grid leftovers and actual crafting remainders remain separate vectors with
  original indices. The consumer owns remainder merge/inventory/drop placement,
  semantic component eligibility, selection priority, events, and durability.
- Grid aggregate encoded/clone budgets are 512 KiB/1 MiB, and complete-plan
  budgets are 1 MiB/2 MiB. Clone accounting walks validated logical lengths and
  inline node/entry values without allocation; allocator/RSS costs and retained
  caller capacities are not represented. All postimages are accounted before
  copying. A craftplan has no revision or durable-commit authority.
