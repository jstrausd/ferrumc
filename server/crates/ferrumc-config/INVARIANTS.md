# Invariants: ferrumc-config

> Rules that must hold for all code in this crate. Violating these is a bug.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

- `WorldConfig` is declarative only: no storage/world/simulation dependency,
  filesystem I/O, random seed selection or persisted identity resolution.
- Omitted `seed` and `preset` remain `None`, independently of one another and
  of Anvil import. `Some(0)` is an explicit numeric seed, not omission. Only the
  application may resolve absent values against stored metadata or explicitly
  choose documented fresh-world defaults.
- The numeric seed is private `Option<i64>` and preserves all signed 64-bit
  values. TOML integer typing applies, including valid base-prefixed integers;
  text, random tokens, floats, other types and out-of-range values are rejected.
- The private preset choice accepts `ferrumc_flat` and the explicitly experimental
  `ferrumc_natural_preview`, `ferrumc_natural_grass_preview` and
  `ferrumc_natural_tree_preview` variants. Grass/tree profiles select separate
  revisions with isolated short-grass and bounded oak/birch subsets.
  These previews do not declare complete features or Survival. Unknown presets and unknown/duplicate keys reject the
  configuration; no stringly typed or catch-all fallback is accepted.
- `with_generation` accepts bounded typed optional declarations, preserves the
  import directory and never resolves omission. New generation fields are
  private; the historical public import field remains solely for compatibility.
- Anvil import is an independent initial content source. Generation declarations
  describe the fallback for unimported chunks; import never reconstructs a
  Vanilla seed/preset or disables supplied generation choices at this layer.
