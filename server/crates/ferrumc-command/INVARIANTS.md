# Invariants: ferrumc-command

> Rules that must hold for all code in this crate. Violating these is a bug.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

- Custom suggestion providers apply only to argument nodes. When present, the
  Brigadier node sets flag `0x10` and encodes its provider string after the
  parser-specific properties; when absent, neither the flag nor string appears.
- Permission filtering retains suggestion metadata only on surviving nodes and
  assigns child indices over that retained set.
