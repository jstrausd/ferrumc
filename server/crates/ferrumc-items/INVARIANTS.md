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

- Ordinary container exchange and shift-transfer consumers resolve max_stack_size
  additions (typed or one exact trusted VarInt) and removals before any exchange.
  Component maxima can lower the registry ceiling; removal uses fallback one.
  Invalid capacities refuse mutation without normalization. A chest plan has
  exactly 27+36 slots, one MiB aggregate persisted bytes before cloning, and no
  cursor/revision/durability authority. Both postimages must publish together.

- Closed ordinary item-rule tables contain all 40 builtin food components and
  all 68 builtin durability defaults from the hash-pinned 1.21.8 registry
  snapshot. The owning generator retains exact f32 bits and refuses a changed
  input hash; food/material/armor contracts are reviewed against Paper build 60.
- Rule admission validates lossless stacks before cloning. FOOD scalar and
  DAMAGE/MAX_DAMAGE/UNBREAKABLE patches have bounded, exact payload parsers;
  unsupported consumable, remainder, attribute, equippable and enchantment
  overrides return typed refusals instead of applying unrelated default rules.
- Durability postimages modify only DAMAGE, preserve all unrelated patch order
  and values, and consume exactly one item only on break. Creative, unbreakable
  and zero-wear decisions retain the exact original representation. Descriptors
  and plans own no use timer, effect RNG, inventory revision, event or commit
  authority; the live consumer must wire these responsibilities.

## Ordinary furnace plans

- The closed 1.21.8 normal catalog contains 72 resource recipes (140 expanded
  ingredient rows) and 269 fuels, generated from 107 hash-pinned Paper build60
  complete resources/source inputs. Source receipt digest is immutable; ordinary
  and Python-optimized regeneration must retain byte-identical catalog output.
  All normal cooking durations are 200 ticks. Nether nonflammable wood is
  removed through the actual tag, without suffix heuristics.
- Exactly three borrowed slots and four bounded scalar timers describe one
  tick. Every authoritative stack, effective capacity and aggregate encoded
  representation passes admission before source cloning. Complete furnace bytes
  include 24 framing bytes and are at most 16 KiB; this is not an RSS allowance.
- One tick first spends existing burn, then admits fresh fuel only with a
  compatible result and enough output space, then advances cooking once. Fuel
  and input consumption preserve surviving patches and drain to canonical empty.
  Lava leaves one bucket; one wet sponge may fill one bucket with water.
  Multiple bucket replacement refuses instead of losing surplus buckets.
- Fixed recipe output uses original defaults. Output merging requires the
  conservative exact ordered representation comparison, stronger than Paper's
  effective-component equality. No new normalization, Java events, recipe/XP
  accumulation, automation or original cached selection history is claimed.
- The consumer owns input-identity progress resets, simultaneous postimage
  commit, exclusive loaded-chunk/tick/revision authority, lit block writes,
  durability, menus and persistence. No World dependency is added to Items.
