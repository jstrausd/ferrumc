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
- No unsafe code, no production `unwrap`/`expect`, no unbounded untrusted
  allocation, and rustdoc on every public item.
