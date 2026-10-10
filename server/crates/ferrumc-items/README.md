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
