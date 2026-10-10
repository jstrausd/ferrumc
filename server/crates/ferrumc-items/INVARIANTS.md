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
- Pure split/transfer planning validates both models before any result clone,
  including no-op and incompatible transfers. It shares persistence's
  allocation-free structural/count/encoded-byte preflight; preflight scratch
  uses a fixed 64-entry size array and a fixed 96-id set.
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
