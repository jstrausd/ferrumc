# ferrumc-config

Server configuration parsing and validation. TOML-based.

`WorldConfig` preserves optional generation declarations for App bootstrap:
`seed()` returns `Option<i64>` and `preset()` returns
`Option<WorldGenerationPreset>`. Omitted values stay `None`; explicit `seed = 0`
is distinct. Only numeric TOML integers and the implemented profile
`preset = "ferrumc_flat"` are accepted. Text/random/float seeds, overflow,
unknown profiles and misspelled fields refuse configuration.

```toml
[world]
seed = -982
preset = "ferrumc_flat"
# Optional initial content source; generation remains independent.
anvil_import_dir = "/srv/maps/spawn/region"
```

`WorldConfig::default().with_generation(seed, preset)` provides bounded typed
construction and preserves an existing import directory. This crate does not
select defaults, read storage or initialize a generation identity. The separate
App ticket resolves omissions against persisted metadata; for a fresh world it
may explicitly select the documented seed-0 flat baseline. Import describes
prebuilt chunks, not recovery of their Vanilla seed or generation settings.

New seed/preset fields are private. Historical public `anvil_import_dir` access
remains supported, but external struct literals must use `Default`/typed
construction or deserialization now that the type has private fields.

## Invariants

See `INVARIANTS.md` in this directory.
