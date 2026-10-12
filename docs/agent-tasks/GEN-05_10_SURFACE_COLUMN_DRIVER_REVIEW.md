# GEN-05.10 — Review: Vertrag und Original-Oracle des Overworld-SurfaceSystem-Columndrivers (Runde 1)

**Verdikt: CLEAN_CONTRACT.** Es gibt keinen blockierenden Befund. Sechs Auflagen gehören ins Runtime-Ticket (A1–A6), dazu kommen sechs Hinweise (H1–H6).

Unabhängiges Root-Review vom 12.10.2026 für den [Vertrag](GEN-05_10_SURFACE_COLUMN_DRIVER.md) (SHA-256 `bd69a4ea…30e3`) und den Freeze `.codex-tmp/gen05_10/delivery/freeze-manifest.json` (`ce970306…887d`) zur [Workorder](GEN-05_10_SURFACE_COLUMN_DRIVER_TASK.md). Der Reviewer hat den Kandidaten nicht erstellt. Geschrieben wurden nur `.codex-tmp/gen05_10-root-review/**` und dieses Dokument. Brief, Evidenz und Produktion blieben unverändert, es gab weder Git noch weitere Agents. Gebaut wurde nur in privaten Archivkopien mit eigenem Target und `--jobs 2`. Dieses Review trifft keine Aussage über eine Runtime, TPS, RSS oder Geschwindigkeit.

## 1. Methode

- **Integrität.** Alle 321 Freeze-Einträge, der Brief und die Workorder wurden vor und nach dem Review gehasht (`integrity-{before,after}.json`): 0 Abweichungen. Unter `scratch/`, `probe/archive/` und `probe/target/` steht kein Freeze-Eintrag, außerhalb dieser Ausnahmen gibt es keine nicht gelistete Datei.
- **Runner.** Private Kopie unter `runner/`, digest-gleich zu den gepinnten Originalen. Der Patch `runner.diff` hat drei Teile: ein eigener Ausgabebereich, die Kanonik-Prüfung über den gepinnten Digest, und alle Eingaben werden aus dem eingefrorenen Bereich gelesen.
- **Eigene Werkzeuge:**
  - `recount.py`: Nachzählung direkt aus den Rohzeilen, ohne `project_originals.py`.
  - `mutations_and_coverage.py`, `forbidden_claims_review.py`, `compare.py`, `integrity.py`, `write_assessment.py`.

## 2. Frische Reproduktion

| Prüfung | Ergebnis |
| --- | --- |
| Original-Oracle normal / `python3 -O` (Scan + Vanilla + Paper) | jeweils 28/28 fachliche Ausgaben bytegleich zum Freeze, `manifest.json` bytegleich (`ddfb7e28…0fcf`). Nur `diagnostics-manifest.json` weicht ab; sie enthält Zeitstempel-Logs. |
| 16 Guards normal / `-O` | `guards.json` bytegleich zum Freeze (`838ecefb…`, `ac03d213…`). Sie unterscheiden sich untereinander nur in `entrypoint_python_optimize`. |
| Fixture-Generator (normal, `-O`, aus dem `-O`-Lauf) | 3 × 8 Dateien bytegleich zu `surface-column-original-1.21.8` (Manifest `98fcf62a…`, 3.571.619 B) |
| Fixture-Validator normal / `-O` | gleich, und gleich `delivery/fixture-validate-*.json`: 72 Gruppen (70 `full`, 1 + 1 `refused`), 18.432 Spalten, 2.261.637 `full`-Aufrufe in P4, 72 rekonstruierte Digests, 2602 Noise-Zeilen |
| Design-Probe, eigene Archivkopie, `ferrumc-world` frisch kompiliert (`cargo clean -p`) | Eingaben bytegleich; `debug.tsv` und `release.tsv` bytegleich zum Freeze, Debug = Release. Sechs Noises in Paper-Basis = Paper 18.432/18.432 je Noise, Vanilla-Basis = Vanilla, Kontrolle 0/18.432. Draws und Melt 18.432/18.432 in beiden Originalen. GEN-05.4-Material, Fluid-Flags, GEN-05.3-Ecken und GEN-05.5-Arrays je 70/70; GEN-05.8-Spaltenbiom 17.920/17.920. |
| Pins, Stichproben | Laufzeit-Inputs beider Originale gleich den akzeptierten GEN-05.8-Pins (`bffbed18…`). 10 JAR-Digests neu berechnet. 92/92 Quellkopien bytegleich zu ihrer Herkunft. 10 als „unpatched“ deklarierte Closure-Klassen sind in Base und Applied identisch. |
| Bytecode-Pins | Paper `XoroshiroRandomSource.nextDouble`: `l2f; ldc 1.110223E-16f; fmul; f2d`. Vanilla `elb.j()`: `l2d; ldc2_w 1.1102230246251565E-16; dmul`. Damit ist die f32-Behauptung (P1) am echten JAR belegt. |

## 3. Echtheit

- **R ist nur aufzeichnend.**
  - `SurfaceColumnOracle.java:425-471`: Der Proxy ruft `settings.surfaceRule().apply(ctx)` genau einmal auf und reicht `tryApply` unverändert weiter.
  - Zusätzlich liest er nur:
    - Context-Felder;
    - `LazyCondition.lastUpdate` und `result` (ein Feldzugriff löst kein `compute` aus);
    - `ProtoChunk.getBlockState`;
    - `ChunkAccess.getHeight(WORLD_SURFACE_WG)`, und zwar nur für die vorhandene Heightmap (`n=2`), also ohne Priming-Nebenwirkung.
  - Die Steep-Höhen werden nach der Rückkehr von `tryApply` und vor `setBlock` gelesen (`SurfaceSystem.java:150-152`). Das ist dieselbe Höhe, die `compute()` gesehen hat.
  - Laziness, Reihenfolge und Zustand bleiben unberührt. Belegt wird das durch U = R nach NOISE und nach SURFACE in 72/72 Gruppen in beiden Modi, selbst nachgezählt. `same()` vergleicht Zustände, beide Heightmaps, die Post-Processing-Listen in Reihenfolge und die Block-Entity-Zahlen.
- **Kein fabrizierter Spaltenkontext.**
  - Jede Zeile der Herkunftstabelle (Brief §2.2) entspricht dem Original. `i4` stammt aus S0, was exakt ist, weil eine Spalte nur ihre eigene Heightmap-Zelle schreibt.
  - Das Spaltenbiom kommt aus einem separaten echten `BiomeManager.getBiome` mit denselben Argumenten. Die Noise-Werte stammen aus echtem `NormalNoise.getValue` mit den Quellargumenten aus `SurfaceSystem.java:207-239`. Die Draws kommen aus einem frischen echten `noiseRandom.at`.
  - Alle sind rein (pure).
  - Der Paper-`Level`-Träger ist ein per `Unsafe` angelegtes `ServerLevel`, dessen einziger Lesepfad in SURFACE `OptionallyFlatBedrockConditionSource.java:49` ist (`generateFlatBedrock = false`, akzeptiertes GEN-05.9-P2).
- **Spalten ohne Regelaufruf.** Das Snapshot-Design könnte eine Spalte ohne `tryApply` nicht beobachten. Die Projektion schlägt dann aber fail-closed fehl (`cols_by[gid][k]`). Tatsächlich haben alle 18.432 Spalten mindestens einen Aufruf.
- **Python-Zweitweg.**
  - `project_originals.py` verwendet den eigenen Xoroshiro nur in `model_frozen` und im Draw-Abgleich (Zeilen 247-256, 428-431).
  - Die Fixture kopiert die echten Diagnose-Bits (`surface_column_fixture.py:221-224`). `frozen_writes` und `eroded_writes` sind echte Snapshot-Differenzen (Zeilen 200, 211).
  - Damit gilt: Der Python-Zweitweg verifiziert nur und liefert nie einen Oracle-Wert.
- **Reviewer-Nachbau (`recount.py`).** Er ist eigenständig: eigener Xoroshiro, Klassifikation nur nach Namen gemäß D1, WORLD_SURFACE_WG über die Original-`update`-Regel (Anheben bzw. Rescan) statt Neuberechnung, Eroded und Frozen mit den Runtime-Clamps aus §6. Ergebnis in beiden Originalen:
  - Kontrolltext gegen echte Schreibvorgänge, je 0 Abweichungen bei: Aufrufliste, Eroded-Spalte, Frozen-Spalte, `i5`, Steep-Sicht und Steep-Ergebnis (838/838), finale Heightmaps, Fluid-Flagmenge.
  - Draws gegen echte Werte: 0 Abweichungen.
  - Mindest-Oberfläche aus den echten Ecken und der echten Oberflächentiefe nach der GEN-05.9-Formel: 0 Abweichungen.
  - Die echten Flags stimmen auf allen 32 Zuständen mit der D1-Namensklassifikation überein.

## 4. Vanilla gegen Paper, an den Daten nachgezählt

| Größe | Ergebnis |
| --- | --- |
| sechs Extension-Noises, Bitmuster verschieden | je 18.432 / 18.432 |
| `nextDouble` verschieden | 18.432 Spalten, 73.728 / 73.728 Werte |
| `nextInt(4)`, `nextInt(10)`, Melt verschieden | 0 / 18.432 |
| Eroded-Ergebnis verschieden bei gleichem S0 und gleichem Spaltenbiom | 0 / 412 (alle Eroded-Spalten) |
| Frozen-Ergebnis verschieden bei gleicher Loop-Spalte und gleicher Mindest-Oberfläche | 0 / 1134 (von 1135 Frozen-Spalten sind 1134 vergleichbar) |
| finale Spalten verschieden | 62 in 25 Gruppen |

Diese Zählung ist stärker als die Erstursachen-Klassifikation des Briefs (Brief:216-218) und bestätigt sie.

**Die 3 zusätzlichen `tryApply`-Aufrufe** (Paper 2.318.375, Vanilla 2.318.372) entstehen in genau den 29 Spalten, deren Post-NOISE-Terrain zwischen den Originalen abweicht. Jede dieser Spalten hat ±1 Default-Stone-Besuch (16 × +1, 13 × −1), netto +3. In jeder Spalte mit gleichem S0 ist die Differenz 0. Die Ursache liegt also im NOISE-Schritt (Produzent GEN-05.3/.4), nicht in der Treibersemantik.

Damit sind Tests, die einen Unterschied voraussetzen, nur als direkte Bitvergleiche gerechtfertigt (C4). Das hält der Brief richtig fest.

## 5. Vertrag gegen Workorder

| Workorder-Punkt | Befund |
| --- | --- |
| Spaltenschleife, Y-Reihenfolge, Stone-Depth oben/unten, Wasserhöhe, Oberfläche | §3 entspricht `SurfaceSystem.java:106-161` Zeile für Zeile. Die Schranke Σ(L+1) ≤ 385 der Stone-below-Suche ist richtig, weil die Suche am unteren Ende jedes Laufs erneut feuert (`i8 == y`). |
| Mindest-Oberfläche, Heightmaps (WORLD_SURFACE_WG laufend) | §4, §7. Die Update-Regel entspricht `Heightmap.java:81-102`. Ein Luft-Schreibvorgang in einer reinen Luftsektion (`ProtoChunk.java:146-148`) kann die Spaltenspitze nie treffen. |
| Steep/Hole mit Zeitpunkt | §5. Die Sicht wird live beim ersten Steep-Test gelesen. Das ist an 838 echten Tests belegt, eine Kombination mit geänderter Höhe kam nicht vor (siehe A3). |
| Bandlands-/Iceberg-/Eroded-/Frozen-Extras mit Original-Randoms | §6. Verzweigungen in beiden Originalen nachgezählt: Eroded 244 / 113 / 2 / 53, Frozen 830 / 304 / 1. Bands laufen über GEN-05.9. |
| Chunk-Mutation, D3, Block-Entities | §7. Block-Entities 0 vor und nach SURFACE. Nachbarn bleiben unverändert (72/72). |
| Kapazitäten | Arithmetik geprüft: `W_col` = 1 + 1158 + 1538 = 2697, `R` = 198.144 + 256 × 2697 = 888.576. Der Zensus 8320 + 35 = 8355 folgt der GEN-05.9-Konvention (`bind.rs:161-173`). Gemessenes Maximum nach §10-Einheiten: 517 pro Spalte und 325.554 pro Chunk, also deutlich unter den Schranken. |
| GEN-05.9-API | 256 × (2 + 1 + 384 × 467) = 45.908.736 (`surface_rules.rs:70-72`). Die Vorprüfung `467 + m` (`session.rs:286`) hält bis zum 384. Aufruf und bis zum Frozen-Minimum. `evaluate_work` ist für alle drei Settings gleich, weil die kompilierten Tabellen identisch sind (`tests/tree.rs:148-160`). `WorkBudgetExceeded` ist damit unerreichbar. |
| RR-H1 / RR-H2 | Übernommen (§8, §16). RR-H1 entspricht `session.rs:683-686`. |
| Vanilla/Paper zuerst prüfen; Fixture mit Validator; Guards normal/`-O` | erfüllt (§2, §4) |

## 6. Replay-Klassen gegen die Policy-Bereiche (Lehre R2-B1)

- **Eigene Ableitung.** `normal`/`large_biomes`/`amplified` mit Zentrum in `[−1874999, 1874998]` ergibt 70 `full`. `outside/normal/0/+x` (cx 1874999) wird `refused:CentreOutsideDomain`, `policy/single_biome_surface/0` wird `refused:InconsistentResolver(Source)`. Das ist identisch zu allen drei Wegen des Kandidaten.
- **Keine `full`-Zeile verlässt eine Policy-Domain:**
  - P4: 0 von 2.261.637 Aufrufen. Maximum Stone-Depth 220, Maximum Wasserhöhe 87.
  - D1: 0 Zustände außerhalb des Alphabets.
  - D4 und P6: konsistent.
  - `W_col` hält in jeder Spalte.
- **Die Domänengrenze D4 folgt aus dem Code:** `admit_chunk` (`density/chunk.rs:563-571`, Ursprung ± 30.000.000) und `admit_domain` (`stored_biome.rs:697-701`).
- **Die 10 Einzelzell-Mutationen.** Jede ändert genau eine Zelle und wird gegen den frischen Rohlauf mit dem erwarteten Grund verweigert.
- **Eigene Zusatzmutationen.** Biomindex, Mindest-Oberfläche, finale Höhen und Melt werden verweigert. Ein Flip von `blocks_motion` bei `granite` in `states.tsv` passiert dagegen Route 2 und wird nur von der Byte-Regeneration in `validate` gefangen (A6).

## 7. Policies D1–D5: Empfehlungen an den Root

- **D1 (35 Zustände): annehmen, mit Auflage A5.**
  - Das Alphabet ist vollständig für alle möglichen Schreibvorgänge. Es setzt sich zusammen aus:
    - den 24 Template-Ergebnissen;
    - den 7 Bandzuständen (überlappend 28);
    - den Extension-Zuständen stone, snow_block und packed_ice (bereits enthalten);
    - der GEN-05.4-Materialmenge (+7).
  - Alle 32 beobachteten Zustände liegen darin. Nicht beobachtet wurden `calcite`, `red_sandstone` (beide echt in GEN-05.9) und `raw_copper_block`.
- **D2 (Vorprüfung auf Chunk-Ebene): annehmen.** Weil `R` konstant ist, wirkt das Budget praktisch als feste Schwelle. Das ist mit GEN-05.x konsistent.
- **D3 (alles oder nichts): annehmen.** Alle Verweigerungen fallen vor die Spaltenarbeit. Das Original ist nicht transaktional, für zugelassene Eingaben weicht D3 aber nicht davon ab.
- **D4 (`[−1874999, 1874998]`, durchgesetzt durch die Produzenten): annehmen, mit Auflage A1.**
- **D5 (OCEAN_FLOOR_WG und Post-Processing-Reihenfolge nicht veröffentlichen, Fluid-Flags als Menge): annehmen.** Kein späterer Verbraucher verliert zwingende Information:
  1. OCEAN_FLOOR_WG ist exakt ableitbar. Die inkrementelle `update`-Regel ist gleich der Top-Prädikat-Funktion, geprüft auf 2 × 72 Endchunks. Dafür braucht es eine `blocksMotion`-Tabelle; das ist eine Voraussetzung für Carver und Features, nicht für dieses Ticket.
  2. Duplikate sind konstruktiv unmöglich:
     - Regelschreibvorgänge treffen nur Default-Stone, jedes `y` einmal.
     - NOISE-Marken liegen auf Fluid-Positionen; die Schnittmenge ist 0, nachgezählt.
     - Eroded- und Frozen-Schreibvorgänge sind kein Fluid.
  3. Die Reihenfolge ist rekonstruierbar. Die NOISE-Reihenfolge ist die `fillFromNoise`-Iterationsreihenfolge, die SURFACE-Reihenfolge die Schleifenreihenfolge, beide sind Funktionen der Positionen. Die Trennung gelingt mit der GEN-05.4-Bitmap (H5).

## 8. Offene Punkte des Kandidaten

- **Oberkante Y 319 und Steep mit im Chunk geänderter Höhe (C5/C3): reicht, mit Auflagen A3/A4.**
  - Die Mechanik der Live-Heightmap ist echt belegt: 139 Eroded-Anhebungen, 9886 Frozen-Schreibvorgänge, 4 Rescans nach Luft an der Spitze, 838 Steep-Lesevorgänge und 18.432 Endhöhen, alle über die Update-Regel nachgebaut.
  - Der GEN-05.8-Resolver lässt `y = 320` mit vertikaler Klemmung zu (`stored_biome.rs:920`).
  - Unbeobachtet ist nur die Kombination selbst. Sie ist quellbestimmt.
- **Clamping auf [−64, 319]:**
  - Eroded ist unbedingt äquivalent: Void-Luft ist weder Stein noch Wasser, und die Schreibvorgänge werden verworfen.
  - Frozen unten ist äquivalent, weil die Draws nach dem letzten zulässigen `y` verworfen werden.
  - Frozen oben ist nur dank `max_value` äquivalent (`(int) d4 ≤ 277 < 320`) → A4.
  - In den Daten kommt kein solcher Fall vor (Floor > 319, Minimum < −64 und Start > 319 je 0).
- **Fehlende `blocksMotion`- und Fluid-Tabelle.** Für diesen Treiber ist das kein Mangel, weil D1 nur Luft und Fluid nach Name bzw. Bereich braucht. Die Übereinstimmung von `is_solid_cube` auf 32 Zuständen beweist nichts darüber hinaus. Das wird ein Voraussetzungs-Ticket vor Carvern und Features (H6).

## 9. Befunde nach Schwere

### Blockierend

Keine.

### Auflagen fürs Runtime-Ticket

- **A1 — die erwartete Verweigerung von `refused:CentreOutsideDomain` ist nicht benannt.**
  - **Belege:**
    - `surface_column_fixture.py:256-257` verlangt „exactly the typed refusal named by the class“.
    - Brief:439-441 sagt jedoch, es gebe keine Treiber-Art dafür.
    - Brief:424 nennt zwei Produzenten-Verweigerungen.
  - **Szenario:** Ein Runtime-Test kann die Klasse nicht mechanisch auf eine Fehlerart abbilden. Er könnte also jede beliebige Verweigerung akzeptieren.
  - **Auflage:** Der Runtime-Test pinnt
    - `fresh_paper_noise_chunk((1875000, 0))` → `ChunkOutOfRange { axis: "x", coordinate: 1875000 }` (`density/chunk.rs:563-571`);
    - `SurfaceBiomeResolver::new` mit Zentrum (1874999, 0) → `CoordinateDomain` (Domäne vor Präsenz, `stored_biome.rs:928-929`, `697-701`);
    - dass `build_chunk` nie erreicht wird und der Puffer `Empty` bleibt;
    - zusätzlich die zugelassenen Kanten cx bzw. cz ∈ {−1874999, 1874998} gegen {−1875000, 1874999}. Für die Kanten gibt es keine Oracle-Gruppe (nächste: ±1874997/8), daher nur Produzenten-Zulassung, kein Originalvergleich.
- **A2 — die Herkunft der Vorläufigen-Oberflächen-Ecken ist ungeprüft.**
  - **Beleg:** `SurfaceChunkInput { …, corners, … }` (Brief:526); `PreliminarySurfaceCorners` trägt keine Identität (`surface_rules.rs:231-254`); die Verweigerungsreihenfolge (Brief:443-452) prüft Material, Bindung und Resolver, aber keine Ecken.
  - **Szenario:** Falsche Ecken (anderer Chunk oder Seed) verändern `above_preliminary_surface` und den Frozen-Bereich. Das Ergebnis wird trotzdem als `Published(chunk, seed, settings)` veröffentlicht.
  - **Auflage:** Die Ecken werden aus demselben GEN-05.3-Produzenten wie das Material mit Identität übernommen, oder sie werden in Rustdoc und INVARIANTS ausdrücklich als vom Aufrufer vertrauter Input geführt. C2 testet die Gleichheit mit `groups.tsv`.
- **A3 — C3 muss diskriminieren.** Der Test läuft über den echten Schreibpfad des Treibers, nicht über eine fabrizierte Höhensicht.
  - Er deckt je einen Fall ab mit Änderung in einer früheren Spalte (Eroded- oder Frozen-Anhebung) und in der aktuellen Spalte (Prepass-Anhebung bzw. Rescan nach einem Luft-Schreibvorgang).
  - Über die `cfg(test)`-Projektion prüft er, dass die vier gelesenen Höhen der Live-Sicht entsprechen und sich von Post-NOISE unterscheiden.
  - Ein Mutant, der die Post-NOISE-Höhen übergibt, muss scheitern.
- **A4 — C5 über interne Spaltenroutinen.**
  - Die öffentliche API nimmt kein fabriziertes Material an. Deshalb laufen die C5-Fälle über `cfg(test)`-Routinen und werden als Vertragstests gelabelt, nicht als Originalvergleich.
  - Frozen-Obergrenze: entweder ab `min(start, 320)` iterieren und 320 als Void-Luft behandeln (`start ≤ 320` gilt immer), oder `(int) d4 ≤ 320` defensiv sichern. So hängt die Äquivalenz nicht an `max_value` (Brief:370-374).
- **A5 — D1-Alphabet nicht von Hand kodieren.**
  - Es wird beim Binden aus Template-Zuständen, GEN-05.4-Materialmenge und {stone, snow_block, packed_ice} abgeleitet. Für alle drei Settings wird die Größe 35 asserted (Brief:461-466).
  - `UnclassifiedState` wird über eine `cfg(test)`-Injektion erreicht.
- **A6 — die Fixture-Flags sind durch Route 2 nicht abgesichert.**
  - **Belege:**
    - `check()` liest die `states.tsv`-Flags (`surface_column_fixture.py:290-291`), vergleicht sie aber nicht mit den Rohdaten.
    - Die zehn Mutationskontrollen laufen nur über `check` (`delivery/freeze.py:143`).
    - Reproduktion: Ein Flip von `blocks_motion` bei `granite` wird nicht verweigert.
  - **Auflage:** Die Runtime prüft vor der Nutzung die Datei-Digests aus `manifest.json` und klassifiziert nach D1 über den Namen, nie über Fixture-Flags.

### Hinweise

- **H1:** Brief:313-315 schreibt die Prüfung von „water `Some(v)` mit `v ≤ 320`“ und von „Stone 1..=384“ dem Validator zu. Der Validator prüft aber nur `y` und Stone gegen 1..=4096 (`surface_column_fixture.py:343-348`). Die Aussage stimmt, belegt durch die Nachzählung (Maxima 87 und 220); die Zuschreibung ist ungenau.
- **H2:** Brief:321 listet die Steep-Lesevorgänge in der Reihenfolge `lz+1, lz−1, …`. Der Trace und GEN-05.9 (`session.rs:608-613`) verwenden `[lz−1, lz+1, lx−1, lx+1]`; das muss der Trace-Vergleich einhalten.
- **H3:** `WorldSurfaceHeights::set` (Brief:333) genügt privat, weil `surface_rules::column` ein Kindmodul ist. `pub(super)` würde die Sichtbarkeit bis zur Crate-Wurzel erweitern.
- **H4:** RR-H2 (Brief:428-430) sollte die Fixture benennen, entweder GEN-05.9 `surface-rules-original-1.21.8` oder den GEN-05.10-Trace.
- **H5:** Spätere Post-Processing-Verbraucher (Fluid-Ticks in Listenreihenfolge) müssen die Reihenfolge nach §7 rekonstruieren. Die NOISE-Reihenfolge als Positionsfunktion ist dafür in deren Ticket zu belegen.
- **H6:** Die `blocksMotion`- und Fluid-Tabelle wird Voraussetzung für Carver und Features (Brief:589-592), nicht für dieses Ticket.

## 10. Ehrlichkeit und Ausschlüsse

- **Forbidden-Claims-Scan:** `markdown_blocks` und `scan` aus `scripts/check-forbidden-claims.py` (Arbeitsbaum = Archiv, `eb2671f2…`), jeweils mit und ohne `force_plugin`, auf Brief, Workorder und dieses Review: 0 Befunde.
- **Schlüsselwort-Durchsicht:** Laufzeit, TPS und RSS erscheinen nur als Ausschluss oder als Auflage.
- **`scratch/`:** Ausgeschlossen. Der verworfene erste Lauf dort (71 Gruppen, 0 Modellabweichungen) unterscheidet sich nur durch die spätere Gruppe `rule_air_write`.
- **`probe/archive` und `probe/target`:** ebenfalls sauber ausgeschlossen.

## 11. Artefakte

`.codex-tmp/gen05_10-root-review/`:

- `final-assessment.json` (Verdikt, Hashes, Bytevergleich);
- `repro-{normal,optimized}/`, `guards-{normal,optimized}/`, `repro-compare.json`;
- `fixture/` (drei Generierungen);
- `probe/` (eigene Archivkopie, `final/receipt.json`);
- `recount.json`, `mutations-coverage.json`, `forbidden-claims-review.json`;
- `integrity-{before,after}.json`, `runner.diff`, `logs/`.
