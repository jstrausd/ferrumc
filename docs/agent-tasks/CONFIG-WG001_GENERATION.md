# CONFIG-WG001 / GEN-12.0 — Optional explicit generation declarations

Ownership: `server/crates/ferrumc-config/**` and this document only. No App,
World, Sim, Storage, board, root lockfile or Git mutation belongs to this ticket.

## Consumer API

```rust,ignore
WorldGenerationPreset::FerrumcFlat
WorldConfig::default()
config.seed() -> Option<i64>
config.preset() -> Option<WorldGenerationPreset>
config.with_generation(Option<i64>, Option<WorldGenerationPreset>) -> WorldConfig
config.anvil_import_dir() -> Option<&Path>
```

Both new fields are private. The preset is a closed supported set with
`#[non_exhaustive]` so downstream matches already require a fallback arm; only the implemented
`FerrumcFlat` variant is representable/accepted today. Its exact TOML spelling
is `"ferrumc_flat"`. There is no free-form string/catch-all fallback or default
enum value. Typed construction cannot represent a seed outside `i64` or an
unsupported preset, and retains `None` instead of resolving it.

`serde(default, deny_unknown_fields)` behavior remains. Omitted `[world]`, an
empty table and absent fields keep seed/preset as `None`, independently of
Anvil import. An explicit numeric zero remains `Some(0)`. Numeric TOML integer
syntax, including base prefixes/underscores, is accepted; quoted numbers,
text/random tokens, floats, boolean/container values and overflow are rejected.
Unknown presets/field names and duplicate choices refuse parsing.

`with_generation` replaces optional declarations and preserves import. The
historical public `anvil_import_dir` field retains existing direct field reads/writes;
no new public fields were introduced. Adding private fields prevents external
struct literals, which must now use `Default`, the typed builder or serde. No
external `WorldConfig { ... }` literals were found in existing repository
consumers. Existing App calls use the import getter and remain unchanged here.

## WG-004 runtime resolution boundary

The config layer chooses no defaults, reads no metadata, constructs no generator
and introduces no World/Sim/Storage dependencies. WG-004 resolves omitted
seed/preset against the existing persisted generator identity; explicit
conflicting choices must refuse startup rather than alter existing metadata.
For an actually fresh world, App can explicitly document/select seed 0 and the
implemented FerrumC flat profile. This ticket alone does not activate that
runtime behavior; the existing App remains unchanged.

Anvil import remains an independent initial content source. Metadata describes
the declared generation fallback for chunks absent from the import. Import
neither recovers a Vanilla seed/preset nor clears/disables generation choices.
No text-seed hashing, random seed selection or unimplemented noise/Vanilla
profile is accepted.

## Original reference review

Pinned Paper 1.21.8 Build 60, commit
`29c8822d90899c89d2689338e81a98f690bcba12`, under
`.codex-tmp/paper-1.21.8-reference/`.

Read the complete applied production sources:

- `net/minecraft/world/level/levelgen/WorldOptions.java`: its stored `seed` is a
  signed Java `long`, separate from generation-feature and bonus-chest flags.
  `parseSeed` trims input, parses a long or hashes a nonnumeric text seed; other
  helpers choose random seeds. FerrumC intentionally accepts only explicit
  numeric i64 configuration and retains omission for persisted-identity
  resolution; it does not implement those text/random helpers in this stage.
- `WorldDimensions.java`: dimensions own their configured `LevelStem` and
  generators; an Overworld is mandatory. `replaceOverworldGenerator`,
  `withOverworld`, `overworld`, stability checks and bake keep profile/dimension
  selection distinct from seed choice. FerrumC's optional profile declaration
  represents only its existing Overworld generator, not arbitrary dimensions.
- `levelgen/presets/WorldPresets.java`: full preset registrations, settings-to-
  preset mapping, normal/flat dimension creation and Bootstrap show that
  Vanilla presets select concrete generators/dimension stems. Its Vanilla flat
  profile uses `FlatLevelSource`/`FlatLevelGeneratorSettings`, while normal uses
  noise generation. FerrumC names its own implemented profile `ferrumc_flat`
  explicitly and rejects Vanilla/noise preset names.

No relevant WorldOptions/WorldDimensions source or feature patch was found in
the pinned patch tree. This declarative parser has no gameplay event ordering;
source review does not establish Minecraft/Paper terrain or runtime parity.

## Verification

Eight new public API/parser tests cover absence versus explicit zero, both i64
extremes, TOML integer typing, rejected seed types/overflow, the only supported
preset, 16 independent import/seed/preset combinations, typed construction and
unknown/duplicate keys. Existing four world-import/default tests remain intact.

Implementation freeze evidence (all exit 0):

- `cargo test -p ferrumc-config`: **30 passed** (22 existing unit tests, 8 new
  public API/parser tests); no doctests. Log: `/tmp/config-wg001-test.log`.
- `cargo clippy -p ferrumc-config --all-targets -- -D warnings`: passed without
  lint suppressions. Log: `/tmp/config-wg001-clippy.log`.
- `cargo fmt -p ferrumc-config -- --check`: passed.
  Log: `/tmp/config-wg001-fmt.log`. Owned diff whitespace check also passed.
- Exact six-file freeze: `/tmp/config-wg001-freeze.sha256`.

Runtime implementation is frozen for independent review/integration; App
resolution/activation remains the separately assigned WG-004 consumer work.
No new dependencies, production I/O, public fields, root lockfile edits or Git
mutations were introduced by this agent.
