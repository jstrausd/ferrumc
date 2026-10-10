# ferrumc-core

Shared leaf types that every other crate in the server builds on. Depends on no
other project crate, and never on Tokio, storage, or networking.

Provides:

- **Identifiers** — `PlayerId` (UUID-backed, with the exact vanilla
  case-sensitive offline-mode UUID-v3 derivation and a random UUID-v4
  constructor), `ConnectionId`, `EntityId`, `EntityUuid`, `WorldId`, `DimensionId`, and
  `PluginId`.
- **`Tick`** — the server tick counter, with overflow-aware advance helpers.
- **`ServerError`** and **`Result<T>`** — the shared, classifying error root.
- **`TextComponent`** / **`TextColor`** — a small structured text model for
  chat, disconnect reasons, and command output.
- **`GameMode`** — game modes with protocol id mapping.

Serde support for these value types is available behind the default-on `serde`
feature.

`PlayerId::offline` MD5-hashes the verbatim UTF-8 bytes of
`"OfflinePlayer:" + username`, matching Java Edition. The shipping login path
derives that identity once and carries the same typed value through access
control, sessions, persistence, metrics, and plugin callbacks.

`EntityId` is the existing signed i32 runtime/protocol reference. `EntityUuid`
is a separate immutable durable identity: explicit UUID and sixteen-byte
constructors preserve every bit, including nil, non-v4 and non-RFC variants.
Byte order is canonical/network UUID order. Text parsing delegates to the
existing `uuid` parser; display is lowercase and hyphenated. The optional
`serde` feature uses the underlying UUID representation transparently.

The UUID value contains no world, dimension or wire ID, generates no random
identity, and supplies no NBT/storage codec. There is no conversion from
`EntityId` or `PlayerId`. Entity owners must enforce uniqueness, lifetime and
scope, and supply any internal generational handle. Entity storage keys,
spawn/tracking/lifecycle and plugin integration remain separate consumers.

## Invariants

See `INVARIANTS.md` in this directory.
