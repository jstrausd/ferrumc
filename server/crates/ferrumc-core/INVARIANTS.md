# Invariants: ferrumc-core

> Rules that must hold for all code in this crate. Violating these is a bug.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

- `EntityId` remains the signed i32 runtime/protocol reference; it is not
  durable entity identity across unloads or restarts.
- `EntityUuid` preserves all 128 supplied bits, including nil, non-v4 and
  non-RFC-variant values. It does not infer identity from a wire/player ID or
  create identifiers from random/time/counter state.
- UUID bytes are the canonical/network sequence with no little-endian field
  swapping, truncation or version/variant stamping. This API claims no NBT,
  packet or persistent-record codec.
- Parsing delegates to `uuid::Uuid::parse_str`; optional Serde transparently
  delegates to the same UUID type. Invalid input is refused, never defaulted.
- Durable values carry no world/dimension, allocation generation or ownership
  token. Entity owners remain responsible for uniqueness, lifetime, scope and
  any internal generational handle; this value type activates no consumers.
