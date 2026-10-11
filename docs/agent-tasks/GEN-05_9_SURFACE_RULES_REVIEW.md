# GEN-05.9 — Root-Review des Surface-Rules-Vertrags und Original-Oracles

Unabhängiges Review, 11.10.2026. Der Reviewer hat den Kandidaten nicht
erstellt. Geschrieben wurde nur diese Datei und `.codex-tmp/gen05_9-root-review/**`.
Es gab kein Git, keinen Agenten und keine Änderung an Brief, Evidenz oder
Produktion. Gebaut wurde ausschließlich in einer privaten Archivkopie mit
eigenem Target-Verzeichnis. Maschinenlesbar:
`.codex-tmp/gen05_9-root-review/final-assessment.json`.

## Verdikt: FINDINGS — ein blockierender, eng begrenzter Befund

Das Oracle reproduziert byte-genau. Die echten G- und C-Stufen sind
authentisch, und der Vertrag erfüllt den Workorder fast vollständig.
**Blockierend ist allein B1:** Die gesamte `generateFlatBedrock=true`-Evidenz
(13.608 Vergleichszeilen, davon 2.481 „Abweichungen“) entstand auf einem
Original-Context, der nie `updateXZ` erhalten hat. Das ist ein fabrizierter
Spaltenzustand. Ein privater Diagnoselauf mit korrigiertem Tracking ergibt 987
statt 2.481 Abweichungen; 1.494 sind Artefakte. Der Brief bezeichnet diese
Werte als echte Evidenz (P2), und die zum Commit vorgeschlagene Fixture trägt sie
als Paper-Ergebnisse. Die Behebung
ist klein (siehe B1). Danach bleiben nur Auflagen für das Runtime-Ticket und
Hinweise. Ein CLEAN_CONTRACT ist nach dieser Korrektur ohne weitere Vertragsarbeit
erreichbar.

Geprüfter Kandidat (alle Hashes vor und nach dem Review identisch):

| Gegenstand | SHA-256 |
| --- | --- |
| Brief `GEN-05_9_SURFACE_RULES.md` | `59e62a776639c420ad055bcb9a90e55b9e3ff946a6cdde627d3eb7a8f3ca55be` |
| `delivery/freeze-manifest.json` (224 Einträge) | `74bfa9259bf4dce37e3922e2118a9ea977e57a32d39df6ceff4981248fbd01fd` |
| `delivery/final-compare.json` | `517e425d961f512e2e97bd5376aa166e87bfc55773c93c7f8591f384586162b6` |
| `oracle/input-pins.json` (2061 Eingaben) | `fdfb8b2492053dc6544473f1493591e5a5c20234d81d31a61b19fb7c41ee67cf` |
| `final-normal/manifest.json` = `final-optimized/manifest.json` | `15e4dbff186d8845143f72a0873f28bd35a87cbff4063dfe2e01d1e222969073` |
| Workorder `GEN-05_9_SURFACE_RULES_TASK.md` | Maßstab, unverändert |

## 1. Frische Reproduktion (ausgeführt)

**Methode.** Wie im GEN-05.8-Review: `runner/` enthält private Kopien von
`run_oracle.py`, `check_guards.py`, `project_originals.py` und
`SurfaceRulesOracle.java`. Helper und Projektion sind byte-gleich mit den
gepinnten Quellen (`eff09a3d…`, `3f4fff06…`). Der Patch steht in
`runner.diff` und ändert nur dreierlei:

- Das Ausgabegebiet ist das Review-Verzeichnis.
- Der Canonical-Check vergleicht den gepinnten Digest statt des Pfads.
- Pins, Design, Mappings, Aliase und Library-Origins werden aus den
  eingefrorenen Originalen gelesen.

JARs, JDK und Archiv bleiben unverändert und gepinnt. Preflight hat alle 2061
Inventar-Digests, 1292 Archiv- und 454 JDK-Einträge vor und nach jedem Export
geprüft.

| Lauf | Exit | Ergebnis |
| --- | --- | --- |
| `python3 run_oracle.py repro-normal` | 0 | Alle 29 Differentialdateien byte-gleich zu `final-normal` **und** `final-optimized`; `manifest.json` byte-gleich (`15e4dbff…`) |
| `python3 -O run_oracle.py repro-optimized` | 0 | identisch |
| `python3 check_guards.py` | 0 | 16/16 verweigert; `guards.json` byte-gleich (`838ecefb…`) |
| `python3 -O check_guards.py` | 0 | 16/16; byte-gleich (`ac03d213…`); einziger Unterschied zu normal: `entrypoint_python_optimize` |

Nur `diagnostics-manifest.json` weicht ab, weil es die zeitgestempelten Logs
listet. Das ist erwartet; die Freeze behandelt diese Logs als nicht
differentiell.

**Kompakt-Fixture** (private Kopie `fixture/`, Skript byte-gleich):

- `generate` normal und `-O`: beide Ausgaben byte-gleich mit
  `fixture/surface-rules-original-1.21.8` (alle 11 Dateien).
- `validate` normal und `-O`: `regenerated_equal: true`,
  `expanded_rows_equal_raw: 38891`.
- Eigene Gegenprobe (`fixture-mutation-checks.json`): `expand_and_compare`
  direkt auf mutierte Kopien. Verweigert werden Ergebnisindex, Surface-Depth,
  State-Tabelle, Spalten-Noise-Bits, Pattern-Outcome, Steep-Höhen,
  Vanilla-Zelle und Gradient-Draw (8/8). Die Expansion prüft also wirklich und
  hängt nicht nur am Regenerationsvergleich.

**Registry-/Sampler-Probe** (private Archivkopie `probe/archive`, eigenes
`probe/target`, nur `surface_design_probe.rs` hinzugefügt; Archivkopie gegen
`base.freeze.json` geprüft: 0 Abweichungen):

- Debug und Release bauen mit rustc 1.99.0 (`~/.cargo/bin`).
- `debug.tsv` und `release.tsv` sind untereinander und mit
  `registry-probe/final-probe/*` byte-gleich.
- 100/100 Block-Regeln über `compute_state_id` gleich `Block.getId`; 27/27
  Biome; 7/7 Noises.
- Vanilla-Surface-Noise, Draw und Surface-Depth: 26593/26593 bitgleich.
- Gradient-`nextFloat`: 1052432/1052432 (Vanilla) und 1051413/1051413 (Paper).
- Paper-`nextDouble` == `f64::from(vanilla as f32)`: 26572/26572; mit dem
  ungerundeten Wert: 0.

**Pins gegen die echten Originale** (Stichprobe, ausgeführt):

- Vanilla-Bundler SHA-1 `6bce4ef4…` gleich `downloads.server.sha1` in Mojangs
  `version.json`. Der Inner-JAR `2bee159e…` steht in `META-INF/versions.list`.
- Paperclip `8de7c52c…` gleich dem in `docs/PAPER_REFERENCE.md` dokumentierten
  offiziellen Build-60-Checksum (nicht erneut online abgerufen). Der Inner-JAR
  `c64fb7db…` steht in Paperclips `versions.list`; sein Manifest zeigt
  `Build-Number: 60` und `Git-Commit: 29c8822`.
- Alle 45 Vanilla- und 120 Paper-Runtime-Eingaben sind identisch mit den
  GEN-05.8-Pins.
- `noise_settings/{overworld,large_biomes,amplified,caves}.json` aus beiden
  Inner-JARs (per `unzip -p`) sind byte-gleich mit `sources/copies/jar-data/**`.
- `design/registry-overworld-surface-rule.json` ist JSON-gleich mit
  `surface_rule` der Overworld-Definition im Registry-Export
  `docs/fixtures/minecraft-1.21.8-original/worldgen.json` (SHA-256
  `bc0dbfa4…1b30`, wie in `worldgen_codegen.rs:31-34` gepinnt). Die
  Vanilla-JAR-Datei ist JSON-gleich mit dem ganzen Exporteintrag. Die Paper-Datei
  unterscheidet sich nur in `surface_rule`.

## 2. Echtheit (Source-Review des Helpers plus Daten)

- **Keine eigene Formel als Oracle.** `SurfaceRulesOracle.java` ruft
  ausschließlich echte Objekte: `RuleSource.apply` auf echten Contexts (`:355-359`),
  `SurfaceRule.tryApply` (`:384`) und echtes `SurfaceSystem.buildSurface`
  (`:580-581`). Die Bäume kommen aus dem Original-Codec (`:270-278`) bzw. aus
  `NoiseGeneratorSettings.surfaceRule()`. `project_originals.py` und
  `surface_fixture.py` kopieren nur Tokens.
- **Der aufzeichnende Proxy fabriziert in der G-Stufe nichts.** Er wendet den
  echten JAR-Baum auf den echten Context an, den `buildSurface` selbst gebaut hat
  (`:550-554`), und leitet jedes `tryApply` an die echte Regel weiter
  (`:563-571`). Zusätzlich liest er nur seiteneffektfrei: Heightmap-`getHeight`,
  `NormalNoise.getValue`, frische `PositionalRandomFactory.at(...)` und
  `BiomeManager.getBiome`. Der `above`-Test läuft nur bei gecachtem
  Minimum (`:409-410`). Stone-Depths, Wasserhöhe, Surface-Depth, Heightmaps und
  Biome stammen also aus Originalcode. Die echte Laziness ist in den Daten
  sichtbar (eigene Prüfung über alle 10.296 G-Spalten): XZ-Knoten
  (`noise_threshold`, `steep`, `hole`) werden nie doppelt pro Spalte berechnet,
  Traces stehen in Preorder, nicht-lazy Arten tauchen nie auf.
- **C-Stufe.** Sie verwendet einen echten Context mit echtem System,
  `NoiseChunk`, Chunk und dem `biomeManager::getBiome`-Delegat (`:530-538`). Die
  deklarierten Stone-/Water-Werte laufen über das originale `updateXZ`/`updateY`
  – dieselben Einstiege, die `topMaterial` mit festen `1,1` nutzt. Die Zeilen
  sind als kontrolliert gekennzeichnet. Das ist mit dem Workorder vereinbar,
  weil Stone-Depth und Wasser im Vertrag explizite Eingaben des späteren Drivers
  sind. Es gibt **eine Ausnahme: B1**.
- **„3 Bäume, 1,26 Mio. Kreuzvergleiche, 0 Abweichungen“** stimmt an den Daten:
  - Jede C-Zeile trägt `cross_trees = "="` (Vanilla 632.096, Paper 631.401).
  - Die Zähler 1.264.192 und 1.262.802 sind genau 2 × C-Zeilen.
  - Bootstrap- und Registry-Contexts erhalten in jeder Zeile korrekt `updateXZ`,
    weil `c.column` erst nach allen drei Probes gesetzt wird (`:613-630`).
  - Die Bäume sind auch unabhängig gleich: Die JSON-Slice ist identisch mit dem
    JAR-`surface_rule`, und `tree-*.tsv` sind kanonisch gleich (Paper nur Knoten
    2).
- **Der Flat-Carrier** ist ein per `Unsafe` allokiertes `ServerLevel`, in dem
  nur `paperConfig().environment.generateFlatBedrock` gesetzt ist. Er trägt
  Konfiguration, keinen Spaltenzustand, und ist offengelegt. Unbedenklich:
  `minY`/`height` kommen laut Paper-Patch aus Chunk und Generator, nicht aus dem
  Level (`WorldGenerationContext.java.patch`).

## 3. Vanilla vs. Paper – an den Daten nachgerechnet

Eigenes Skript `independent_modes.py`, ohne `project_originals.py`, direkt auf
den Rohzeilen (Ergebnis `independent-modes-final-normal.json`):

| Behauptung (Brief Z. 218-233) | Eigene Zählung | Bewertung |
| --- | --- | --- |
| Alle Spalten-Noise- und `nextDouble`-Werte weichen ab (26556) | 18.984 gemeinsame Spalten, alle mit anderem Noise **und** Draw. 26.556 ist die Zahl gemeinsamer *Zeilen* mit Spaltenzelle | bestätigt (H5) |
| Paper-Draw = `f32(vanilla)` | 18.984/18.984 gemeinsame Spalten; ungerundet 0 | bestätigt |
| Surface-Depth-Formel | `(int)(n·2,75+3+d·0,25)` trifft 26.593/26.593 (Vanilla) und 26.572/26.572 (Paper, mit dem eigenen Paper-Draw) | Brief-Formel bestätigt |
| 510 Ergebnisse | 510. Ursachen: 449 Biom (alle im Wüsten-Chunk), 59 Position (Terrainhöhe), 2 Surface-Depth | bestätigt, alle erklärt |
| 36 Surface-Depths | 36 **Zeilen**, alle aus **einer** Spalte (`sweep/jagged_peaks/1`, x=−1272, z=−8177: 2→3). Treiber ist die Noise-Basis: Vanilla-Noise mit Paper-Draw ergibt weiter 2 | Zahl stimmt, Formulierung irreführend (H1) |
| 682 direkte Biome | 682, **alle** in `biome/desert`; Positionen gleich | bestätigt |
| 3 von 720 Biom-Chunks (98 Werte) | `snowy_slopes (95,191)` 1, `desert (0,-128)` 96, `lush_caves (-16,47)` 1 = 98 | bestätigt |
| 14 Gruppen mit anderen Höhen | 14 Gruppen, 20 Spalten; Ecken 0 | bestätigt |
| Gradient-Draws gleich | 1.050.710 Vergleiche an gleicher Position, alle bitgleich | bestätigt |
| Clay Bands, Anchors gleich | 80/80 Gruppen gleich (4 Band-Sätze für 4 Seeds; 1 Anchor-Satz) | bestätigt |
| Baumstruktur gleich | Registry-Baum in beiden Modi gleich; Paper-JAR nur Knoten 2 | bestätigt |
| Shared-Outcomes, Ausnahmen, D-Zeilen | exakt wie Brief-Tabelle; 0 Ausnahmen in G/C | bestätigt |

Damit sind mehr als 20 behauptete Abweichungen einzeln nachgerechnet (24
Stichproben in `result_difference_samples`). Die Zoom-Semantik trägt **null**
Unterschiede bei: Alle 682 Biom-Abweichungen liegen in der Gruppe mit dem
abweichenden Array. Das bestätigt GEN-05.8 H-2 und ist für P6 relevant. Die
GEN-05.8-Lehre gilt hier umgekehrt: Die Zahlen stimmen. Einzig „36
Surface-Depths“ suggeriert 36 Spalten, gemeint ist eine.

## 4. Vertrag gegen Workorder, Punkt für Punkt

| Workorder-Punkt | Befund |
| --- | --- |
| Kompilierter begrenzter Baum aus Originaldaten, keine Handabschrift, Drift-Erkennung | Erfüllt. Quelle ist `definition().json()` des gepinnten CAT-02.1-Exports. Census selbst nachgezählt (428 Knoten, Tabelle Z. 311-327, Tiefe 13). Definition 129.737 Byte und 3.411 Knoten unabhängig bestätigt (Rohtext inklusive Whitespace; Werte 1.818 + Keys 1.593). Drift: Registry-SHA-Pin, Census-Assert, `tree.tsv`. Lücke für die Paper-Form siehe A2 |
| Eingaben, Reihenfolge, Laziness jeder Condition | Erfüllt. Tabelle Z. 409-422 gegen `SurfaceRules.java:257-858` geprüft: `LazyXZ` (noise, steep, hole), `LazyY` (biome, y, water, stone, gradient, flat, temperature), nicht gecacht (`not`, `above`), Biome-Memo pro Block. Steep-Achsen asymmetrisch korrekt (`:400-415`). Stone-Depth `(int)Mth.map` mit Trunkierung. Water `MIN_VALUE` → true. `bandlands` mit `Math.round` → long → int (H9) |
| Steep als geliehene Sicht pro Aufruf | Gegen das Original korrekt. `SteepMaterialCondition.compute` liest `chunk.getHeight(WORLD_SURFACE_WG, …)` erst beim ersten Test der Spalte (`SurfaceRules.java:393-417`, `LazyXZCondition`). `buildSurface` schreibt zwischen den `tryApply`-Aufrufen in denselben Chunk (`SurfaceSystem.java:149-153`). `minecraft:air` ist ein Regelergebnis (`states.tsv` Index 23). Am Chunkrand liest der Clamp die eigene Spalte. Einfrieren bei `begin_column` wäre also falsch. In den Daten macht der Zeitpunkt nie einen Unterschied (838/838 echte Steep-Tests lesen die Endhöhen, H6) |
| Per-Block-/Per-Column-Kontext mit Produzenten | Erfüllt (Tabelle Z. 371-382). Die Ecken sind pro Chunk konstant, weil die Surface-Zelle gleich dem Chunk ist (`>>4`, `SurfaceRules.java:337-357`). `i32::MAX` gilt als „keine Fläche“ (`NoiseChunk.java:212`; FerrumC `density/chunk.rs:348-350`) |
| Ergebnistyp, typisierte Verweigerung | Erfüllt: `State`/`NoRule`, 20 Fehlerarten, feste Reihenfolge, Poison, keine Default-Blöcke |
| Kapazitäten, Work, N-1/N/N+1, allokationsfrei | Rechnerisch geprüft: 287 + 141 + 33 + 1 + 1 + 2 + 1 + 1 = 467; 133.778; 14.757 (58 Oktav-Slots + 56 × 259 laut `noise.rs:119-135,306-320` + 192 + 3); 45.908.736. Lücken bei Budget-Domains und Retained-Caps siehe A3, A4 |
| Block-, Quart-, Resource-IDs getrennt | Erfüllt. `BlockStateId`, `BiomeResourceId` und `NoiseResourceId` sind getrennte Typen. Quarts erscheinen nicht, weil der Resolver `BlockPos` nimmt |
| Oracle: echte Objekte und Contexts, alle Arten, Schwellen, Grenzen, Enden, Wasser, Steep/Hole, Temperatur, echte Stored-Biome, beide Modi | Erfüllt, außer B1. Alle 127 Lazy-Knoten zeigen beide Ausgänge. Jede endliche Schwelle ist unter, in und über dem Bereich belegt. Gradient-y −66…10, Enden −65/−64/319/320/i32-Extreme |
| Runner, Manifeste, Guards normal/-O | Erfüllt und reproduziert |
| Rust-Erwartungen nur mechanische Projektion | Erfüllt für Default-Modus; der Flat-Spalte der Fixture fehlt die Echtheit (B1) |
| Policy-/Malformed-Tests getrennt | Erfüllt (Z. 528-530, 594-595) |
| R-A1 eingeplant | Ja (Z. 562-564) |

## 5. Policies – Root-Fragen mit Empfehlung

- **P1 „ausschließlich Paper-Numerik“ – annehmen.**
  - Begründung: Der einzige Terrain-, Material-, Climate- und
    Preliminary-Produzent ist die Paper-Session
    (`SeededRouter::fresh_paper_noise_chunk`, INVARIANTS „Fresh Paper NoiseChunk
    consumer“). Sie kopiert Improved-Offsets über f32 (`density/chunk/state.rs:183`,
    `cold.rs:454`). Die Stored-Arrays aus GEN-05.5 sind daher Paper.
  - Nur der Zoom (GEN-05.6/.8) kennt beide Semantiken. Er ist in beiden Modi
    ergebnisgleich (GEN-05.8 H-2; hier 0 zoombedingte Abweichungen).
  - Ein Vanilla-Surface auf Paper-Terrain wäre ein Hybrid, den es in keinem
    Original gibt.
  - Verweigerung: Die API hat keinen Numerik-Parameter. Ein „Vanilla“-Wunsch ist
    nur über die Zoom-Semantik des Resolvers ausdrückbar, und P6 verweigert ihn
    typisiert (`InconsistentResolver`). Es wird also nichts still als Paper
    gerechnet.
  - Empfehlung: Das Profil sichtbar machen, entweder im Typ oder per Rustdoc, und
    die Verweigerung unterscheidbar machen (A8).
- **P2 Flat-Bedrock – Empfehlung: nur `Disabled` ausliefern, `Enabled`
  typisiert verweigern**, bis ein Welt-Konfigurationsticket existiert.
  - Begründung 1: Die einzige Evidenz für `Enabled` ist nach B1 ungültig. Alle
    13.608 Zeilen liefen auf fabriziertem Context. Nur die 987 tiefen
    Bedrock-Zeilen sind zufällig richtig, weil sie nicht vom Spaltenzustand
    abhängen.
  - Begründung 2: Für Paper's `environment.generate-flat-bedrock` gibt es in
    FerrumC weder Produzenten noch Eigentümer, und Nether/Caves-Roof sind out of
    scope.
  - Das Enum `PaperFlatBedrock::{Disabled, Enabled}` sollte trotzdem bleiben,
    damit das spätere Ticket die Signatur nicht ändert.
  - Variante A (beide ausliefern) ist erst sauber, wenn B1 per Harness-Fix
    behoben und die Flat-Evidenz frisch erzeugt ist.
- **P3 nur `overworld`/`large_biomes`/`amplified` – annehmen.**
  `tree-variants.tsv` belegt die kanonische Gleichheit in beiden Modi.
  `caves`/`floating_islands` haben andere Bäume. Korrekt als Policy markiert.
- **P4 Domains `y ∈ [-64,319]`, Stone `1..=4096`, Wasser `≠ i32::MIN` – annehmen.**
  - Das Original liefert in `buildSurface` Stone-Depths `1..=384` (das
    Void-Air unter `minY` beendet die Stone-Suche) und Wasser `≤ 320`.
  - Das Original rechnet auch außerhalb dieser Domains ohne Exception, und der
    Brief belegt das mit Rückhalte-Zeilen. Als Policy korrekt gekennzeichnet
    (Z. 477-487).
- **P5 Vorabprüfung 467 Work-Units, Poison bei Defensivfehlern – annehmen**
  mit Präzisierung (A3). Es ist eine reine FerrumC-Grenze und als Policy
  markiert.
- **P6 Resolver muss Paper-Zoom nutzen – annehmen als Konsistenz-Policy.** Sie
  ist ergebnisneutral, weil beide Zoom-Modi bitgleich sind. Das sollte der Brief
  so begründen, nicht als numerische Notwendigkeit. Markiert ist sie in §11,
  aber nicht in der Liste Z. 477-483 (H11).

## 6. Risiken

- **Definitionsgröße 129.737 von 131.072 Byte.**
  - Die Registry erzwingt das Limit beim Build (`worldgen_codegen.rs:23-30`),
    und `worldgen.json` ist per SHA gepinnt. Drift kann also nur durch einen
    bewussten Neu-Export entstehen und schlägt dann zuerst im Registry-Build fehl,
    nicht still in World.
  - Der Rand besteht überwiegend aus Pretty-Print-Whitespace (kompakt 36.145
    Byte). Der Surface-Compile wiederholt die Grenze defensiv.
  - Inhaltsdrift fangen Census-Assert und `tree.tsv`-Vergleich. Ausreichend.
- **Rohausgaben** sind 474.301.331 Byte pro Lauf (≈ 474 MB bzw. 452 MiB; der
  Ersteller-Bericht nennt 453 „MB“, H4). Sie werden nicht committet, nur die
  4,1-MB-Fixture.
- **Freeze-Ausnahmen.** `scratch/` und `registry-probe/{archive,target}` sind
  sauber ausgenommen und im Manifest-Scope deklariert. Nicht deklariert, aber
  auch nicht eingefroren sind `registry-probe/dev2-probe/*` und
  `registry-probe/warm-debug.log` (H7). `final-optimized` ist nur über seine
  zwei Manifeste eingefroren; die 35 Dateien sind über deren Hashes indirekt
  abgedeckt und hier frisch reproduziert.

## 7. Runtime-Tauglichkeit

Der Brief reicht für ein eindeutiges Ein-Crate-Ticket in `ferrumc-world`:

- neues Modul `surface_rules`;
- `pub mod surface_rules` in `lib.rs`, ausdrücklich freizugeben, wie A1 in
  GEN-05.8;
- keine Registry-Änderung und keine Dependency;
- `paper_build60_basis` und `SampleWork` sind schon `pub(crate)`.

Alle referenzierten Produzenten-APIs existieren im Basisarchiv:

- `SurfaceBiomeResolver::biome_at_block` (`stored_biome.rs:983`);
- `BiomeZoomSemantics` (`biome_zoom.rs:49`);
- `BuiltinBiomeTemperatures::sample` (`biome_temperature.rs:232`);
- `NoiseChunkSession::preliminary_surface_level` (`density/chunk.rs:350`).

R-A1 ist eingeplant (Z. 562-564). Die Auflagen A1–A8 machen das Ticket
eindeutig.

## 8. Ehrlichkeit

- Keine Runtime-, Survival-, TPS-, RSS- oder Aktivierungsaussage
  (Z. 35-36, 537, 673-674).
- Source-Review (§2.1, `sources/`, `bytecode/`) und Differential-Evidenz (§2.2,
  `final-*`, `projection/`) sind getrennt gelabelt.
- Forbidden-Claims: `markdown_blocks` + `scan` aus
  `scripts/check-forbidden-claims.py` (SHA-256 `eb2671f2…`) direkt auf Brief und
  Workorder angewendet, jeweils mit `force_plugin` false und true. Ergebnis:
  **0 Funde** (`forbidden-claims-brief.json`).
- Eine sachlich falsche Evidenzaussage ist B1 (Z. 205, 637).

## Befunde nach Schwere

### Blockierend

**B1 — Die Flat-Bedrock-`true`-Evidenz entsteht auf einem Context ohne
`updateXZ` (fabrizierter Spaltenzustand).**

- **Ort:** `oracle/SurfaceRulesOracle.java:605-610` (`updateBoth` prüft das
  gemeinsame `c.column`) und `:622-630` (der Flat-Probe wird nur bei `y ≤ 10`
  angefasst, `c.column` aber nach *jeder* Zeile gesetzt). Behauptungen im Brief:
  Z. 186-193, 205, 613, 636-639. Fixture: `surface_fixture.py:70-71,115`.
- **Mechanismus:** Die erste Zeile jeder `Controlled`-Instanz ist eine Zeile
  mit `y > 10` (Surface, Sweep, Steep, Extreme). Danach gilt `c.column ==
  columnKey`, und der Flat-Context bekommt für keine Spalte jemals `updateXZ`.
  `updateY` setzt nur `blockY` und das Biome-Memo (`SurfaceRules.java:315-322`).
  Damit rechnet der Flat-Context mit `blockX = blockZ = 0`,
  `surfaceDepth = 0` (also `hole = true`) und dem initialen `lastUpdateXZ`.
  Diese Kombination erzeugt weder `buildSurface` noch `topMaterial`.
- **Beleg 1 (Rekonstruktion aus den eingefrorenen Rohzeilen,
  `flat-stale-sim.json`):** 13.608 von 13.608 Flat-Auswertungen sind betroffen;
  in keiner Zeile hatte der Flat-Context die aktuelle Spalte. Die Übergänge
  `none→deepslate` 1.213, `grass_block→deepslate` 120 und `deepslate→none` 49
  liegen bei `y ∈ [−2, 10]`. Dort kann die Bedrock-Bedingung (wirksam nur bei
  `y ≤ −59`) keinen Unterschied erzeugen.
- **Beleg 2 (privater Diagnoselauf `flatdiag/`, Paper-Stufe mit Pro-Probe-
  `updateXZ`; ein Zweizeilen-Patch, `flatdiag/patch.diff`):**
  - Alle G-, C- und D-Zellen außer der Flat-Spalte bleiben byte-gleich, ebenso
    `arrays`, `groups`, `origins`, alle Bäume und der Digest aller echten
    Aufrufe (`53a677ba…`).
  - 1.494 Flat-Zellen ändern sich.
  - Korrekt bleiben genau **987** Abweichungen von 13.608, alle
    `bedrock→deepslate` bei `y ∈ [−63, −60]` (382/292/207/106, mit der Höhe
    fallend, wie es der Gradient verlangt). Das ist die einzige Wirkung, die die
    Paper-Klasse haben kann (`OptionallyFlatBedrockConditionSource.apply`: flach
    bei `min(T,F) = −64`).
  - Die übrigen 1.494 der 2.481 behaupteten Abweichungen sind Artefakte.
  - Ergebnis in `flatdiag/flat-fix-compare.json`. Der Diagnoselauf ist
    Reviewer-Evidenz, keine Kandidaten-Evidenz.
- **Szenario:** Root wählt Variante A. Die Runtime testet
  `PaperFlatBedrock::Enabled` gegen die 4.383 Flat-Zellen der Fixture (2.481 mit
  Abweichung). Die Zellen sind keine Originalergebnisse für die angegebene
  Position. Ein korrekter Evaluator fällt durch, oder er wird an fabrizierte
  Werte angepasst. Bei Variante B bleiben fabrizierte Werte als
  „Paper-Ergebnisse“ (Fixture-Manifest: „Paper rows are the production
  expectations“) im Commit, und P2 behauptet falsch „backed by genuine evidence“.
- **Behebung (eine der beiden), danach neue Freeze:**
  1. Pro-Probe-Spaltentracking in `updateBoth`, frische Läufe normal/`-O`,
     Fixture neu erzeugen, Zahlen in Z. 205 und 637 korrigieren.
  2. Alternativ: Flat-Vergleich aus Oracle und Fixture entfernen, P2 auf
     Variante B festlegen und die Brief-Aussagen streichen.
- **Abgrenzung:** Default-Modus, G-Stufe, C-Stufe, Kreuzbaumvergleich und alle
  übrigen Zahlen sind nicht betroffen. Der Diagnoselauf ändert außerhalb der
  Flat-Zellen kein einziges Byte.

### Auflagen fürs Runtime-Ticket

- **A1 — Replay-Semantik der Fixture festlegen.**
  - Ort: Brief Z. 428-432, 625-629.
  - Szenario: „The runtime emits the same projection … and compares it“ ist für
    isolierte Zeilen nicht erfüllbar. Nur 2.490 der 6.720 G-Fixture-Zeilen haben
    ihr vollständiges Spaltenpräfix in der Fixture. Für die übrigen fehlen die
    früheren Aufrufe, die XZ-Caches (`noise_threshold`, `steep`, `hole`,
    Secondary) gefüllt haben. Die C-Zeilen haben dasselbe Problem.
  - Auflage: Das Ticket legt fest:
    - Ergebnis und per-Block-`computed`-Einträge gelten für jede Zeile.
    - Volle Traces gelten nur bei vollständigem Präfix. Alternativ die
      aufgezeichneten G-Präfixe vollständig in die Fixture aufnehmen.
    - Heights-View: Endhöhen aus `groups.tsv` für echte Chunks. Das ist
      belegt, denn 838/838 echte Steep-Tests lesen Endhöhen. Für Steep-Probe-
      Zeilen die aufgezeichneten `steep_heights` bzw. die deklarierte synthetische
      Höhenformel als Eingabe.
- **A2 — Paper-Baumform mechanisch committen.** Ort: Z. 299-302. Die Fixture
  enthält nur `tree.tsv` in Vanilla-/Registry-Form; `paper/tree-jar.tsv` liegt
  nur in den nicht committeten Rohdaten. Auflage: Paper-Projektion bzw.
  `paper-tree-substitution.json` in die Fixture aufnehmen, damit die
  Post-Substitution-Prüfung nicht von Hand geschrieben wird.
- **A3 — Budget-Domains und Work-Abrechnung präzisieren.**
  - Ort: Z. 509-526, 544-545.
  - Wertebereiche von `SurfaceRuleBuildBudget` und `SurfaceRuleBindBudget` sind
    nicht genannt; nur der Session-Bereich `1..=67_108_864`.
  - Offen ist auch, ob die in `evaluate` ausgelöste Min-Surface-Berechnung im
    Knotenpreis enthalten ist oder zusätzlich 1 Unit kostet (sonst wären es
    468).
  - Auflage: Domains plus N-1/N/N+1 an beiden Bereichsenden aller drei Budgets
    (`InvalidBudget`) und eine eindeutige Abrechnungsregel festlegen.
- **A4 — Retained-Kapazitäten exakt pinnen.** Ort: Z. 491-507. 16/32/4 KiB
  sind Design-Caps. Auflage: Die Runtime misst die exakten Bytes,
  pinnt sie und testet `NativeLimit` mit N-1/N/N+1. Eine Cap-Änderung ist eine
  Root-Entscheidung und wird nicht still angehoben. Bei 428 Knoten ist 16 KiB
  knapp, sobald ein Knoten 40 Byte übersteigt.
- **A5 — Paper-Basis aller neun Noises bitgenau belegen.** Ort: Z. 241-252. Die
  Probe prüft Surface-Noise nur in Vanilla-Basis. `clay_bands_offset` wird im
  Oracle nirgends direkt aufgezeichnet, nur indirekt über `bandlands`-Ergebnisse.
  Auflage: noise- und knotenweise Bitvergleiche für die 7 Schwellen-Noises,
  `surface` und `surface_secondary` gegen die Oracle-Zellen. Für
  `clay_bands_offset` ist die Belegart (indirekt über Bänder) offen zu nennen.
- **A6 — R-A1 übernehmen** (Z. 562-564): einen R1-gleichwertigen Test in
  `stored_biome/tests`, wie im GEN-05.8-Runtime-Review verlangt.
- **A7 — Ownership und Doku.** `pub mod surface_rules` in `lib.rs`, die
  Fixture unter `server/crates/ferrumc-world/tests/fixtures/surface-rules-original-1.21.8`
  (Brief nennt keinen Zielpfad), Referenzskripte unter `tests/reference/`.
  INVARIANTS/README bekommen einen eigenen Abschnitt: Es gibt dann einen zweiten
  Paper-Basis-Konsumenten neben der NoiseChunk-Session. Die INVARIANTS sagen
  heute: „Only this Paper Build60 session privately copies Improved offsets
  through f32“.
- **A8 — Policy-Verweigerungen unterscheidbar.** `InconsistentResolver` soll
  den Grund tragen (Seed, Settings oder Zoom-Semantik). Bei Variante B soll
  `Enabled` eine eigene, als Policy markierte Verweigerung tragen.
  Steep-Zeitpunkt: ein kontrollierter Test, in dem sich die Heights-View nach
  dem ersten Steep-Test einer Spalte ändert und das Ergebnis gleich bleibt
  (Original-Evidenz deckt das nicht ab, H6).

### Hinweise

- **H1** Z. 223: „36 surface depths“ sind 36 Zeilen einer einzigen Spalte; der
  Treiber ist die Noise-Basis, nicht die f32-Rundung des Draws.
- **H2** Z. 200 und Ersteller-Bericht: „Genuine `buildSurface` calls“ sind
  `tryApply`-Aufrufe aus echtem `buildSurface` (80 `buildSurface`-Aufrufe pro
  Modus).
- **H3** Z. 331: 180 statt 181 geteilte Bootstrap-Positionen (beide Modi
  nachgezählt).
- **H4** Z. 665: 474 MB (dezimal) = 452 MiB; Ersteller-Bericht „453 MB“.
  Einheit vereinheitlichen.
- **H5** Z. 222: 26.556 sind gemeinsame Zeilen mit Spaltenzelle; gemeinsame
  Spalten sind 18.984.
- **H6** Z. 388-393: Die Notwendigkeit der Per-Call-View ist quellbasiert
  richtig. Kein Differentialfall belegt sie: In allen 838 echten Steep-Tests
  waren Testzeit- und Endhöhen gleich.
- **H7** `registry-probe/dev2-probe/*` und `warm-debug.log` sind weder
  eingefroren noch als Ausnahme deklariert.
- **H8** `expand_and_compare` prüft `exception`, `cross_trees` und
  `biome_memo` nicht. Folgenlos, weil alle Zellen `none`/`=` sind; für die
  Runtime besser explizit.
- **H9** Z. 403: `mod 192` ist Java-`%`. Für die Policy-Domain ist der Operand
  nichtnegativ, solange `|round(noise·4)| < 128` gilt. Das kurz begründen oder
  als Invariante testen.
- **H10** Z. 244-248: Die Probe deckt die Paper-Noise-Basis nicht ab; Z. 244
  sagt „Vanilla“, das ist korrekt, aber leicht zu überlesen (siehe A5).
- **H11** P6 steht in §11, fehlt aber in der Policy-Liste Z. 477-483;
  Begründung als Konsistenz-Policy ergänzen.

## Integrität nach dem Review

- Alle 224 Freeze-Einträge sind vor und nach dem Review neu gehasht: 0
  Abweichungen, 0 fehlend (`integrity-before.json`, `integrity-after.json`).
- Brief `59e62a77…` ist unverändert, und in `.codex-tmp/gen05_9` gibt es kein
  `__pycache__`.
- Seit Reviewbeginn ist dort keine Datei außerhalb der deklarierten Ausnahmen
  neu oder verändert.
- Alle Läufe, Builds und Diagnosen liegen unter `.codex-tmp/gen05_9-root-review/`.
  Das sind `runner/`, `repro-*`, `guards-*`, `fixture/`, `probe/` (eigene
  Archivkopie und eigenes Target), `flatdiag/` und die eigenen Skripte
  `independent_modes.py`, `flat_stale_sim.py`, `compare.py` und
  `write_assessment.py`.
