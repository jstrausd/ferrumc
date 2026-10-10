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
