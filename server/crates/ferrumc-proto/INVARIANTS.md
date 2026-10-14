# Invariants: ferrumc-proto

> Rules that must hold for all code in this crate. Violating these is a bug.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

- Generated packet changes originate in docs/protocol/1_21_8/packets.toml and
  pass cargo xtask generate --check; generated files are never edited by hand.
- Play Disconnect is clientbound packet 0x1C for protocol 772. Its reason is a
  bounded anonymous-root NBT text component, using the same wire type as System
  Chat. Login Disconnect's JSON-string encoding must not be reused in play.
- Item-entity Play packets are SetEntityData 0x5C, SetEntityMotion 0x5E, and
  TakeItemEntity 0x75 for protocol 772. Motion accepts already quantized i16
  wire values; Take preserves all three signed VarInts without count policy.
- SetEntityData's metadata_raw is an unprefixed opaque BoundedBytes<263168>
  tail. The local cap is checked before tail read/copy/allocation; refusal
  leaves the reader at the tail entry, after any consumed entity ID. The cap
  is not a Vanilla metadata limit or a global trusted ItemStack guarantee.
  No EOF/serializer/Slot/component/NBT/trailing-byte validation or insertion
  occurs here. Semantic builders must validate their complete output within
  the cap in separately reviewed consumer tickets.

- Survival wire IDs follow protocol772: UpdateHealth0x61, experience0x60, Respawn0x4B, UpdateLight0x2A, BlockDestruction0x05, EntityEvent0x1E, CombatDeath0x3D, ClientStatus0x0B, UseItem0x40 and InteractEntity0x19. InteractEntity has an opaque18-byte capped tail; Session must validate exact action-specific shape, hand and finite coordinates. Packet codecs establish wire format only, not gameplay or full light semantics.
