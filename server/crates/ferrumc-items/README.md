# ferrumc-items

The shared item-stack model and slot wire codec for Minecraft Java 1.21.8
(protocol 772, data-component items).

This crate sits *below* the proto and sim lanes so neither has to depend on the
other for a common item type. The generated protocol packets carry every
slot-bearing field as opaque `remaining_bytes` (the same precedent as the
Brigadier `Commands` packet); this crate produces and consumes those bytes.

## What it provides

- `ItemId` — a registry-validated protocol item id (`id()`, `name()`,
  `max_stack()`, `placeable_block()`).
- `ItemStack` — the canonical, *trusted* item stack (clientbound form), with a
  registry-checked constructor and a fail-closed trusted slot encoder.
- `UntrustedItemStack` — the *untrusted* serverbound wire form (creative slot),
  with `decode` and `into_validated` for hostile-input normalization.
- `ComponentValue` / `ComponentPatch` / `ComponentTypeId` — the data-component
  model, with a few typed variants and an `Opaque` passthrough.
- `encode_container_content_payload` — builds the `SetContainerContent` body.
- `encode_persisted_stack` / `decode_persisted_stack` — a separate versioned,
  bounded storage codec that preserves the supported authoritative model.
- `plan_split` / `plan_transfer` — checked pure separation and non-swapping
  partial transfer plans, preserving exact supported component representation.
- `persisted_stack_size` / `same_stack_representation` — lossless borrowed
  size/identity probes without encoding, allocating, or cloning components.

## Trusted vs untrusted slots

The two directions share the same outer framing (`itemCount` varint; if non-zero,
`itemId`, added/removed counts, the component arrays) but differ in **component
data framing**:

- **Trusted (clientbound)** component data is *typed and unprefixed* — a switch on
  the component type id (e.g. `max_stack_size` => varint, `unbreakable` => void,
  `custom_data` => network NBT). This is what the server emits.
- **Untrusted (serverbound)** component data is a *varint-length-prefixed blob*
  (`ByteArray`), so unknown or dangerous components are bounded, skippable, and
  strippable without parsing their internals — exactly what hostile-input
  handling needs.

They are therefore two separate, self-consistent codecs (each round-trips for its
own direction), **not** a cross-direction round trip.

## Trusted construction and emission

`ItemStack::try_new` is the checked path for data-driven counts and accepts only
values in `1..=ItemId::max_stack()`. The source-compatible `ItemStack::new`
retains its caller-validated `NonZeroU8` contract while callers migrate; it must
not be used as proof that the registry maximum was checked.

Every trusted `encode_slot` call revalidates the registry count before emission.
A `max_stack_size` component cannot raise that registry ceiling, and any count
or component-encoding error restores the output buffer to its original length.
Container-content encoding routes every list and carried stack through the same
check.

## Hostile-input normalization

`UntrustedItemStack::into_validated` rejects an unknown item id, clamps the count
to `1..=max_stack`, bounds the component count and total bytes, strips
`block_entity_data` (51), `container` (66), and out-of-range component types, and
validates NBT component data against `ferrumc_nbt::NbtLimits`. Anything malformed
returns an `Err` rather than panicking.

## Independent item persistence

The storage codec is an S02 foundation. Player inventories and world containers
must integrate it through their own versioned records; adding this codec alone
does not make existing player/world persistence retain components.

Creative decoding is intentionally lossy and must never be used to read stored
items. The storage codec preserves patch order, all seven `ComponentValue`
representations, opaque payloads even for modeled component ids, and removed ids.
It preserves NBT compound key order and duplicate names, strings including NUL
and astral characters, and exact floating-point bits. Duplicate component patch
ids and an id both added and removed are rejected rather than merged. In-range
removals of dangerous types are safe removals and remain preserved.

Schema 1 uses fixed big-endian lengths and integers:

| Field | Representation |
| --- | --- |
| Signature and schema | `FCIS`, `u16 = 1` |
| Stack discriminator | `u8`: 0 empty, 1 present; empty ends here |
| Item | `u16` UTF-8 byte length plus canonical resource name, then `u8` count |
| Patch counts | `u8` additions, `u8` removals |
| Each addition | `u8` model representation, `i32` component id, `u32` payload length, payload |
| Each removal | `i32` component id |

The addition representations are 1 `MaxStackSize` (`u8`), 2 `Damage` (`i32`),
3 `MaxDamage` (`i32`), 4 `Unbreakable` (empty), 5 `CustomName` (file-form NBT),
6 `CustomData` (file-form NBT), and 7 `Opaque` (unchanged trusted bytes). File-form
NBT has an empty root name. Item names avoid coupling storage to item protocol
numbers, but component ids and opaque payload semantics remain pinned to
Minecraft 1.21.8. Updating that model requires an explicit schema migration.

The codec permits at most 64 total patch entries, 8192 payload bytes per
component, and 256 KiB total payload bytes. `MAX_PERSISTED_STACK_BYTES` bounds
the whole record before parsing. These are encoded-byte caps; they are not a
decoded-heap limit. Inventory and chunk consumers must enforce a separate
aggregate record budget. NBT output receives an allocation-free size/structure
preflight before the shared NBT writer is invoked, so its allocation remains
bounded as well.

Unknown schemas, discriminators, ids, noncanonical names, duplicate patch ids,
malformed values, oversized declarations, truncation, and trailing bytes fail
with `ItemPersistenceError`. A stored stack is never clamped or stripped.
`max_stack_size` values must be in `1..=99`; they cannot raise the registry count
ceiling. The codec preserves the current model's signed damage values and
present-air representation without adding gameplay normalization.

## Checked split and transfer plans

`persisted_stack_size(&stack)` returns the exact whole schema-1 FCIS record
size using persistence's existing allocation-free validation. Empty has seven
bytes. Present counts occupy one fixed byte, so any valid present count has the
same size for the same item and patch. This is an encoded size, not a slot
payload size, decoded heap budget, or aggregate reservation.

`same_stack_representation(&source, &destination)` validates source first and
destination second before returning a boolean, even for incompatible/empty
inputs. It ignores valid counts and compares item identity plus the transfer's
exact ordered patch representation, including typed/opaque variants, removal
order, NBT duplicates/order, and float bits. Two canonical empty stacks match;
empty/present differ and present-air remains present. Invalid source/destination
retain their existing `StackTransferError` classification; source error wins
when both fail. Neither probe changes input representation or clones a tree.
They do not resolve gameplay stackability, Paper component defaults, slot
capacity, or permission to publish a durable change.

`plan_split(source, requested)` returns a `StackTransferPlan` with the source
remainder, a separated destination stack, and `moved()`. `plan_transfer(source,
destination, requested, destination_capacity)` returns the same owned result
for a non-swapping partial transfer. Neither API mutates its borrowed inputs.
`into_stacks()` yields `(source_after, destination_after)` for the owning caller.

The `u8` request is a bounded upper quantity: zero is a no-op and a request
above the source quantity can move at most the source quantity. Capacity is the
**total** destination count limit, not free space; all `u8` capacities are
accepted and the item's registry maximum remains an independent upper bound.
Thus capacity 255 cannot enlarge a stone stack beyond 64 or a sword beyond 1.
An existing destination above the supplied capacity fails, even for a zero
request; it is never silently reduced. Empty outputs are canonical empty.

Both input models are validated before cloning, including no-op, full, and
incompatible cases. Transfer and persistence use the same allocation-free
preflight, including registry counts, 64 patch entries, valid/nonconflicting
ids, typed values, NBT structure/depth, 8192 encoded bytes per component, and
256 KiB total component payloads. Preflight uses a fixed 64-entry size array
and a fixed 96-id set. These byte allowances describe encoding; they are not
equal decoded-heap allowances. Successful plans own at most two copies of the
validated logical component trees. Clone allocation is proportional to the
logical entries, strings, arrays, and NBT nodes rather than the caller's retained
vector capacities; decoded enum/container and allocator overhead is additional.
Consumers must separately bound the number of simultaneous plans/records.

A present destination merges only the identical item and exact ordered patch
representation. Typed and opaque representations, removal order, duplicate NBT
keys, compound order, and float bits are distinct. Identical NaN bits match;
different NaN payloads and +0/-0 do not. This comparison preserves representation
and does not use the NBT model's derived IEEE float equality. Incompatible
stacks yield a zero-movement plan and never swap or overwrite.

This is a model primitive, not Paper's complete inventory policy. Paper resolves
prototype/default component maps and uses semantic Java component equality;
these plans deliberately do not normalize defaults, reorder maps, collapse
duplicate NBT keys, decode opaque components, or evaluate damage/stackability.
The later consumer must enforce eligibility and supply a capacity reflecting
resolved `max_stack_size` and slot limits. The existing model still permits
present-air and signed durability values. Nothing in this API activates drops,
pickup, hopper transfers, or Java events.

The caller must hold exclusive authority when publishing **both** outputs.
Plans do not contain inventory revisions, compare-and-swap checks, replay guards,
transaction identifiers, storage writes, or crash recovery. Applying one output,
reusing a stale plan, or saving the outputs separately can lose or duplicate
items. Persistent item conservation remains a separate consumer/Storage contract.

## Normal crafting plans

`NormalRecipes::builtin()` materializes the 968 original 1.21.8 normal recipes:
673 shaped and 295 shapeless, including all 16 Suspicious-Stew component outputs.
Private generated descriptors come from the actual original `Recipe.CODEC`,
bound vanilla item tags, item remainder methods, and result stream codecs.
The catalog shares 479 static ingredient sets (1506 memberships), without a
runtime JSON parser or added Rust dependencies. Construction checks descriptor
limits and all 1416 original item id/name/default-count-ceiling pins before
materializing fixed outputs. The actual producer and cases live in
`tests/reference` and `tests/fixtures`.

Select a recipe explicitly with `recipe("minecraft:stick")`; `iter()` is in
resource-name order and does not define Paper recipe priority. Admit borrowed
slots using `CraftingGrid::new(2, &slots)` or `CraftingGrid::new(3, &slots)`, then
call `matches` or `plan_one`. Grid validation uses the existing lossless stack
preflight, refuses present-air, and bounds the entire grid before copying.
Empty outer input space is trimmed; original indices remain stable. Shaped
patterns match horizontal mirrors and preserve internal holes. Shapeless
matching finds a complete assignment with at most nine ingredients and slots.
Count64 in one position remains one ingredient position. Normal item/tag
ingredients ignore input patches, while fixed results retain their original
patches and never inherit unrelated input data.

A `SingleCraftPlan` consumes exactly one item from each occupied position.
`grid_after()` preserves surviving count/patch representation and restores
canonical empty on draining. `crafting_remainders()` separately preserves every
actual rest (for example all three cake buckets). Sim must decide whether each
rest belongs in an empty grid position, a semantically compatible stack, the
inventory, or a drop; this API never silently overwrites a full/incompatible
grid leftover. `into_parts()` transfers ownership to that consumer.

The grid admits at most 512 KiB encoded schema-1 bytes and 1 MiB logical clone
bytes; the entire plan admits at most 1 MiB encoded and 2 MiB logical clone
bytes. Logical accounting includes inline stacks/components, cloned NBT nodes,
compound entries/names, lists, arrays, strings, and opaque lengths. Retained
caller capacities and allocator overhead are excluded, so this is not an RSS
claim. No plan clones an unchecked tree or returns a partial result.

Special recipes, noncrafting serializers, plugin `ExactChoice`, Paper manager
priority/cache, result clicks, shift-crafting, recipebook, Java events, and
durable commits are separate consumer tickets. The current model's registry
count ceiling and opaque passthrough contract remain unchanged; crafting does
not introduce prototype/component semantic normalization.
