# GEN-05.10 — Surface-Column-Driver-Runtime: unabhängiger Root-Review

Review vom 12.10.2026 durch einen unabhängigen Reviewer, der den Kandidaten nicht
erstellt hat. Geprüft wurden:

- der eingefrorene Kandidat: 324 World-Dateien, `source.freeze.json` SHA-256
  `3b457b81…6d999e2`, Kandidatenmanifest `cae56ed8…3f991b93`;
- die [Delivery](GEN-05_10_SURFACE_COLUMN_DRIVER_RUNTIME_DELIVERY.md) (`cf491ee2…`);
- die Runtime-Evidenz (`evidence.freeze.json` `af30fb8f…`).

Maßstab waren die [Runtime-Freigabe](GEN-05_10_SURFACE_COLUMN_DRIVER_RUNTIME_TASK.md)
(`05d68fef…`) mit D1–D5, A1–A6, RR-H1 und RR-H2, der
[Vertrag](GEN-05_10_SURFACE_COLUMN_DRIVER.md) (`bd69a4ea…`), dessen
[Review](GEN-05_10_SURFACE_COLUMN_DRIVER_REVIEW.md) (`c5cf5ab5…`) und die
[Workorder](GEN-05_10_SURFACE_COLUMN_DRIVER_TASK.md) (`f9862c96…`). Das eingefrorene
Original-Oracle liegt unter `.codex-tmp/gen05_10/**` (Freeze `ce970306…`).

## Verdikt: **CLEAN_RUNTIME**

Es gibt keinen blockierenden und keinen mittleren Befund. Fünf Hinweise (RR10-H1 bis
RR10-H5) blockieren nicht.

- **Checks:** Alle frischen Checks sind grün.
- **Replay:** Ich habe das ganze Fixture-Replay in Debug und Release selbst ausgeführt.
  Zusätzlich habe ich die Spaltenschleife, beide Extensions, die Live-Höhen und die
  Treiber-Arbeit über einen eigenen Python-Zweitweg direkt aus der Fixture
  nachgerechnet: 0 Abweichungen.
- **Allokation:** Die Allokationsmessung ist reproduziert. Zusätzlich habe ich vier
  Verweigerungsarten gemessen, die die Delivery nicht misst: 0/0/0.
- **Mutanten:** 21 eigene Mutanten plus eine unveränderte Baseline.
  - 15 werden getötet.
  - 3 sind beweisbar äquivalent, darunter der überlebende Mutant des Erstellers.
  - 3 decken echte Testlücken auf: die Produktions-Höhenableitung bei y 319 und die
    Unterscheidung von Wasser und Lava in beiden Extensions. Der Code ist an diesen
    Stellen korrekt. Meine Probes sind auf dem Kandidaten grün und töten alle drei
    Mutanten (RR10-H1, RR10-H2).

**Root-Entscheidung (Frage 1): annehmen.** `W_col = 2698` und `R = 888 832` sind
arithmetisch richtig. Sie sind die minimale Folge der wörtlichen A4-Fassung der
Freigabe (Abschnitt „Antworten“).

## Antworten auf die Root-Fragen

### 1. `W_col` 2698 statt 2697, `R` 888 832 statt 888 576

**Herleitung im Vertrag (§10, Zeile 486).** Frozen = 3 Noises + 1 Temperatur + 2
Int-Draws + 384 Iterationen + 384 Double-Draws + 384 Writes = 1158. Die 384 Iterationen
beruhen auf der Klemmung in §6.2 auf `[max(min_surface, −64), min(start, 319)]`.

**Was A4 ändert.** Die Freigabe fasst A4 verbindlich als „iterates the frozen upper
bound from `min(start, 320)`“. Damit gilt:

- Der Treiber besucht höchstens die 385 Y-Werte 320 … −64 (`driver.rs:632`). Nach der
  §10-Einheit „1 per loop step“ sind das 385 Einheiten. Meine Probe misst genau das:
  Start 320 kostet 385 Besuche, Start 319 kostet 384.
- Die Iteration bei 320 zieht nie:
  - `start = max(i4, (int) d4 + 1) ≤ 320` (die Verweigerung oberhalb 320 sichert das ab),
    also gilt `(int) d4 ≤ 319` und `320 < (int) d4` ist falsch;
  - `VOID_AIR` ist kein Wasser.

  Draws und Writes bleiben also ≤ 384. Tatsächlich sind es sogar ≤ 383, weil
  `y < (int) d4 ≤ 319` gilt. 384 bleibt aber eine gültige obere Schranke.

**Rechnung:**

- Frozen = 3 + 1 + 2 + 385 + 384 + 384 = 1159.
- `W_col` = 1 + max(1155, 1159) + 1538 = 2698.
- `R` = 98 304 + 1 536 + 98 304 + 256 · 2698 = 198 144 + 690 688 = 888 832.
- `R` liegt weiter unter dem Domänenmaximum 1 048 576. Die Grenzen N−1/N/N+1 sind am
  neuen `R` getestet; Mutant M17 (`<=` statt `<`) wird getötet.
- Mein Python-Zweitweg rechnet mit derselben Formel. Er reproduziert die beobachteten
  Maxima 517 pro Spalte und 270 785 pro Chunk (`C0` als tatsächlich verbraucht), wie in
  der Delivery.

**Korrekt und minimal.** Unter der verbindlichen A4-Fassung ändert sich genau ein Term
(+1 Besuch); alle anderen Schranken bleiben.

**Anmerkung.** Der Schritt bei 320 ist ergebnisneutral. Mutant M09 („Schleife ab
`min(start, 319)`“) überlebt die volle Suite und ist äquivalent, weil die Verweigerung
oberhalb 320 bestehen bleibt. Die zweite A4-Alternative des Vertrags-Reviews (Start bei
319 plus defensive Sicherung) hätte also 2697/888 576 behalten. Die Freigabe hat aber
320 festgelegt. Der Ersteller ist dem wörtlich gefolgt und hat die Folge offengelegt.
Ich empfehle, die Werte so anzunehmen; eine Codeänderung ist nicht nötig. Die Klemmung
in Vertrag §6.2 und die Zahl in §10 sind damit durch A4 überholt. `INVARIANTS.md`
dokumentiert bereits 320 und 2698/888 832.

### 2. Start oberhalb 320: Verweigerung statt Klemmung

- **Für zugelassene Eingaben unerreichbar.**
  - `i4` ist die Live-Höhe + 1 mit Höhe ≤ 319, also `i4 ≤ 320`.
  - `(int) d4 + 1 ≤ 278`. Grund: `|iceberg_pillar_roof| ≤ max_value = 10/3`, daraus
    `m1 ≤ ceil(5 · 40) + 14 = 214` und `d4 ≤ 277`. Melt senkt `d4` nur.
  - `max_value` ist die Original-Schranke `edgeValue(2.0)` (`noise.rs:573-583`), also
    eine echte Schranke mit großem Abstand. Meine Probe bestätigt sie:
    - `max_value` = 3,333… in allen sechs Fixture-Bindungen;
    - empirisches Maximum 1,29 über je 160 801 Samples.
  - NaN ergibt `(int) 0`.
  - Größter beobachteter Start in der Fixture: 95 (Zweitweg).
- **Policy akzeptabel.**
  - Der Fall ist als `Noise`, Kategorie `Defensive`, typisiert. Rustdoc
    (`error.rs:73-75`) und `INVARIANTS.md` nennen ihn. Über D3 bleibt der Puffer
    `Empty`.
  - Eine stille Klemmung würde hier, anders als bei Eroded, die Draw-Folge ändern,
    falls der Fall je eintritt.
  - Ein defensiver `Noise`-Fehler für einen unerreichbaren Zahlenbereich hat ein
    Vorbild in GEN-05.9: den negativen Band-Index.
- **Getestet.** `frozen_fill_from_min_start_320…` prüft Start 321 → `Noise`, Counts
  (321, 320), `Defensive` (`control.rs:417-429`).
  - Der Mutant des Erstellers (stille Klemmung) wird getötet.
  - Meine Mutanten M11 (Verweigerung erst oberhalb 321) und M12 (Verweigerung entfernt)
    werden ebenfalls getötet.
  - Der öffentliche Pfad kann den Fall nicht auslösen. „`Empty` nach einem Fehler in
    der Schleife“ ist allgemein über `UnclassifiedState` getestet.

### 3. A2: vom Aufrufer vertraute Ecken

- **Dokumentiert:**
  - Rustdoc von `SurfaceChunkInput` und `new` (`driver.rs:34-55`);
  - Rustdoc von `PreliminarySurfaceCorners` (`surface_rules.rs:243-248`);
  - `INVARIANTS.md` und `README.md`.
- **Getestet:**
  - `check_producers` vergleicht für alle 70 `full`-Gruppen die GEN-05.3-Ecken mit
    `groups.tsv` (`replay.rs:34-35`).
  - `preliminary_corners_are_caller_trusted_input` (`refusals.rs:418-451`) treibt
    fremde Ecken (−40). Das Ergebnis wird trotzdem unter der Identität des Materials
    veröffentlicht, aber Digest und `min_surface` weichen ab.
- A2 ist damit in der zweiten erlaubten Variante erfüllt.

### 4. A3–A6, RR-H1, RR-H2 und der überlebende Mutant

Ich habe jede Auflage im Code gelesen und mit eigenen Mutanten gegengeprüft. Die
Mutanten wurden im Release-Build gegen die ganze Column-Suite gefahren.

| Auflage | Code und Test | Eigene Gegenprobe |
| --- | --- | --- |
| A3 | Echter Schreibpfad über `build_observed`, synthetische Regelbäume auf echten Chunks. Eroded: 512 Steep-Lesungen, davon 102 aus einer früheren und 6 aus der aktuellen Spalte. Luft-Rescan: 256 Lesungen, davon 480 früher und 64 aktuell. Alle Werte sind identisch reproduziert. | **M01** (Sicht als Schnappschuss bei Spaltenbeginn, also nur die eigenen Writes der aktuellen Spalte unsichtbar) wird nur vom C3-Luft-Test getötet: Die aktuelle Spalte diskriminiert. **M02** (Eroded-Writes heben die Höhe nicht) wird von C3, C5 und Replay getötet. |
| A4 | `Synthetic` über die internen Routinen, als Vertragstests gelabelt; Frozen ab `min(start, 320)`, 320 als `VOID_AIR`. | **M08** (Untergrenze −63) getötet; **M09** äquivalent (Frage 1); **M11** und **M12** getötet (Frage 2). |
| A5 | Das Alphabet wird bei der Bindung abgeleitet: GEN-05.4-Material über `material_alphabet`, 24 Template-Zustände, Bänder, Extensions (`bind.rs:338-383`). 35 für alle drei Settings; Klassen nach Namen geprüft (`work.rs:61-110`). `UnclassifiedState` wird per Alphabet-Entzug und über `oak_log` im Material erreicht. | Mein Zweitweg klassifiziert unabhängig nach Namen. Alle Materialschreibstellen von GEN-05.4 (`material.rs:92-117`, `aquifer.rs`, `ore.rs`) liegen in der Zehnermenge. |
| A6 | Manifest-Digest, sieben Datei-Digests und Plattenbytes werden vor jeder Nutzung geprüft (`tests.rs:191-200`). Zustände werden über `compute_state_id` aus dem Namen bestimmt; keine Flag-Spalte wird gelesen. | **F01** (Flip von `blocks_motion` bei `granite` in `states.tsv`) wird von 17 Tests getötet. |
| RR-H1 | `seed_precedes_settings_when_both_mismatch` testet die Session und `build_chunk`. | **S01** (Settings vor Seed in `session.rs`) wird nur von diesem Test getötet. |
| RR-H2 | Ein Arbeitsmodell pro Aufruf im Replay: der Resolver wird genau dann belastet, wenn ein `biome`-Knoten oder das `temperature`-Singleton berechnet wird. Resolver befragt in 405 539 Aufrufen, nicht befragt in 1 856 098, maximal 72 Einheiten; identisch reproduziert. | **S02** (Memo nie gefüllt) wird getötet, unter anderem vom Replay. |

**`snow-counter-only-in-range` ist äquivalent.** Ich habe ihn als M10 reproduziert; er
überlebt. Er unterscheidet sich nur bei einem Treffer außerhalb von `[−64, 319]`.

- Unter −64 wird nie iteriert (`low = max(min_surface, −64)`).
- Oberhalb 319 gibt es nur y = 320. Dort ist `VOID_AIR`, es gilt `320 < (int) d4` nicht,
  und es ist kein Wasser.

Die Äquivalenz hängt an der Verweigerung oberhalb 320, und diese ist getestet (M11, M12).

**Weiterer äquivalenter Mutant: M18.** Frozen mit der Live-Höhe statt des veralteten
`i4`.

- In Frozen-Spalten läuft kein Prepass, also gilt `i5 = i4`.
- Regel-Writes können die Höhe nur senken (Rescan nach Luft an der Spitze).
- Der verkürzte Bereich liegt vollständig über `(int) d4 + 1` und über der Live-Spitze.
  Dort gibt es nur Luft ohne Draw.

### 5. Änderungen außerhalb des neuen Moduls

Der Diff gegen die Runtime-Basis `9f842bc4` umfasst nur World. Zwischen dieser Basis
und der Review-Basis `b5e41784` liegen nur Dokumente.

- `surface_rules.rs`:
  - `mod column` und Re-Exporte;
  - Rustdoc-Text;
  - private `const fn set` an `WorldSurfaceHeights`, wie in H3 vorgesehen. Sie ist
    rein additiv; kein bestehender Pfad ruft sie auf außer dem neuen Treiber.
- `density/chunk/material.rs`: neue `pub(crate) fn material_alphabet`. Sie ruft das
  unveränderte `MaterialBlocks::new` mit eigenem `SampleWork(16384)` auf und liest nur.
  `MaterialRun` und die Probenahme sind unverändert.
- `density.rs` und `density/chunk.rs`: zwei `pub(crate) use` sowie ein
  `#[cfg(test)]`-`with_states`. Das ist nur im Testbuild vorhanden und klont
  ausschließlich.

Kein Funktionsrumpf von GEN-04 oder GEN-05.3–05.9 ist geändert. `Cargo.toml`,
`Cargo.lock` und `lib.rs` sind unverändert. Alle GEN-05.3–05.9-Suites sind in Debug und
Release Teil der grünen World-Läufe (383 Tests). **Keine Verhaltensänderung.**

### 6. Replay und Allokation selbst reproduziert

- **Replay:** `every_full_group_replays_the_original_trace_exactly`, Debug und Release
  auf frischem Target. Die `GEN0510_REPLAY`-Summen sind identisch mit der Delivery:
  - 70 Gruppen, 17 920 Spalten, 2 261 637 Aufrufe;
  - eroded 139, rule 1 195 094, frozen 9886, steep 838;
  - Spaltenmaximum 517, Chunkmaximum 270 785, Session-Maximum 784 542.
- **Zweitweg** (`tools/recount.py`): Ich habe die Klassen aus der Policy-Tabelle neu
  abgeleitet: 70 / 1 / 1. Pro Spalte habe ich dann alles nach Vertragstext nachgerechnet:
  - `i4` und `i5`;
  - alle Aufrufeingaben (gegen `call_runs`);
  - Eroded- und Frozen-Writes aus den echten Paper-Noise-Bits; die Draws werden
    gezählt, nicht gezogen;
  - Rescans und die finalen Höhen.

  Ergebnis: 0 Abweichungen in 17 920 Spalten. Regel-Outcomes stammen hier aus der
  Fixture; sie sind Sache von GEN-05.9. Der Zweitweg liefert außerdem: P4-Maxima
  220 / 87, 4 Rescans, maximale Frozen-Iterationen 77, `noise`-Zeilen 2570, Melt 2269.
- **Allokation** (`measure_allocations.py`, unverändert):

  | Profil | CONTROL | BUILD | DRIVE (70 veröffentlichte + 70 verweigerte Chunks) |
  | --- | --- | --- | --- |
  | Debug | 1 / 4096 B | 3150 / 984 339 B | **0 / 0 / 0** |
  | Release | 1 / 4096 B | 3150 / 984 339 B | **0 / 0 / 0** |

  Die Stack-Werte sind identisch mit der Delivery: 24 656 B (Debug) und 16 384 B
  (Release).

## Prüfgrundlage und Isolation

- Basis ist das readonly Archiv `.codex-tmp/gen05_10-runtime-review-base/archive`:
  1431 Dateien, `base.freeze.json` `106ad3f3…`. Ich habe davon drei private Kopien
  unter `.codex-tmp/gen05_10-runtime-review/` angelegt:
  - `work/` für die frischen Checks, mit eigenem frischem Target `target/`;
  - `probe/` für eigene Tests;
  - `mutants/` für Mutanten.

  Im Arbeitsbaum wurde nichts gebaut. Ich habe kein Git verwendet und keine weiteren
  Agents gestartet.
- Die World im Archiv ist gleich `source.freeze.json` und gleich dem
  Kandidatenmanifest: 324/324.
- Fixture und Referenzskripte (8 + 11 Dateien) sind bytegleich mit dem eingefrorenen
  Oracle. Neu sind nur `measure_allocations.py` und die README.
- Integrität (`tools/integrity.py`), vorher und nachher bytegleich (`bac7cf77…`):

  | Prüfgegenstand | Ergebnis |
  | --- | --- |
  | Review-Archiv | 1431/1431 unverändert |
  | Oracle-Freeze `.codex-tmp/gen05_10/**` | 321/321 |
  | Runtime-Evidenz | 60/60 |
  | World im Arbeitsbaum | 324/324 gleich `source.freeze.json` |
  | gepinnte Dokumente im Arbeitsbaum | 5/5 gleich dem Archiv |

- Nach den Checks sind alle 1431 Quelldateien in `work/` unverändert. Dort liegen nur
  die bekannten App- und Plugin-Testartefakte `.codex-tmp/p57-…`, `p58-…` und ein
  leeres `p52-…`.
- Die Probe-Kopie unterscheidet sich von der Basis nur durch `review_probe.rs` und
  dessen Registrierung; `tools/probe_diff.py` prüft das bei jedem Lauf. Alle Mutanten
  wurden per Hash wiederhergestellt; `mutants/work` ist danach gleich der Basis.

## Teil A — Ausführung

### A.1 Frische Checks

cargo/rustc 1.99.0, `--jobs 2 --locked`, eigenes frisches Target.

| Check | Ergebnis |
| --- | --- |
| World Debug | exit 0: 383 passed, 0 failed, 4 ignored in 9 Suites (lib 327 + 3); die Column-Suite hat 23 + 1 |
| World Release | exit 0: identische Zahlen |
| Clippy `--workspace --all-targets -D warnings` | exit 0 |
| `fmt --all --check` | exit 0 |
| World-rustdoc `-D warnings` | exit 0 |
| Workspace-Build | exit 0 |
| World-Verbraucher (storage, sim, anvil, session, testkit, bench, app, plugin-block-rules, plugin-region-guard; `--no-fail-fast`) | exit 0: 94 Suites, 1489 passed, 0 failed, 1 ignored. `multiplayer_visibility` war im ersten Lauf grün; ein Wiederholungslauf war nicht nötig. |

Die Zahlen sind identisch mit der Delivery. Logs liegen in `logs/`, die
Zusammenfassung in `evidence/checks-summary.json`.

### A.2 Eigene Probes

Die Probes liegen in `probe/…/column/tests/review_probe.rs` (`2d73d048…`). Alle sind in
Debug und Release grün.

| Probe | Zweck | Ergebnis |
| --- | --- | --- |
| `review_probe_frozen_work_bound` | Frage 1 | Start 320 kostet 385 Besuche, Start 319 kostet 384. Draw-lastiger Grenzfall 1144, Mischfall Wasser/Luft 1128, beide ≤ 1153 (= 1159 − 6). |
| `review_probe_frozen_start_reachability` | Frage 2 | `max_value` 10/3, Supremum des Starts 320, empirisch \|roof\| ≤ 1,29 |
| `review_probe_production_path_reads_y_319` | Lücke zu M13 | Kandidat: `i4 = 320`, erster Aufruf bei y 319. Mit M13: 63 bzw. y 61, **getötet** |
| `review_probe_lava_is_not_water_in_extensions` | Lücke zu M14/M15 | Kandidat: 55 Eroded-Writes, 7 Frozen-Treffer, Draws synchron. M14: 28 Treffer, M15: 0 Writes, **beide getötet** |
| `review_probe_refusal_kinds_allocate_nothing` (ignoriert, extern gemessen) | Allokation weiterer Verweigerungen | je 10 × Binding, Material, Session-Zulassung, `UnclassifiedState` und 10 Erfolge: DRIVE **0/0/0** in Debug und Release, Kontrolle positiv |

Ein erster Debug-Lauf der Arbeitsprobe scheiterte an einem Fehler in der Probe selbst.
Bei flacher Form ist `(int) d4 = 0`, und Luft unter y 0 zieht dann; das ist
Originalverhalten. Ich habe die Probe-Säule korrigiert und neu laufen lassen; der
Vorgang ist in `logs/summary.log` offengelegt.

### A.3 Mutanten

`tools/run_mutants.py`, Ergebnisse in `evidence/mutants.json` (`d3b01d3d…`). Jeder
Mutant ist eine exakte Einzelersetzung in `mutants/work`. Gefahren wurde die
Release-Column-Suite; die Baseline M00 ist grün.

| ID | Mutation | Ergebnis |
| --- | --- | --- |
| M01 | Steep-Sicht als Schnappschuss bei Spaltenbeginn | getötet (C3-Luft) |
| M02 | Eroded-Writes heben die Höhe nicht | getötet (C3, C5, Replay) |
| M03 | Stone-below-Suche hält nur an Luft | getötet (Replay, C5 `i4 = 320`) |
| M04 | Wasserhöhe wird bei Luft nicht zurückgesetzt | getötet (Replay) |
| M05 | Eroded `i4 >= floor` | getötet (C5, Replay) |
| M06 | Schneelinie `>=` | getötet (Replay) |
| M07 | Schneezähler `<` | getötet (C5, Replay) |
| M08 | Frozen-Untergrenze −63 | getötet (C5) |
| M09 | Frozen ab `min(start, 319)` | überlebt, **äquivalent** (Frage 1) |
| M10 | Schneezähler nur im Bereich (Mutant des Erstellers) | überlebt, **äquivalent** (Frage 4) |
| M11 | Startverweigerung erst oberhalb 321 | getötet (C5) |
| M12 | Startverweigerung entfernt (Klemmung) | getötet (C5) |
| M13 | Produktions-Höhenableitung ab y 318 | **überlebt die Kandidaten-Suite**, getötet durch die eigene Probe (RR10-H1) |
| M14 | Frozen-Wassertest akzeptiert Lava | **überlebt die Kandidaten-Suite**, getötet durch die eigene Probe (RR10-H2) |
| M15 | Eroded bricht auch an Lava ab | **überlebt die Kandidaten-Suite**, getötet durch die eigene Probe (RR10-H2) |
| M16 | Melt ignoriert | getötet (Replay) |
| M17 | Chunk-Allowance `<=` | getötet (9 Tests) |
| M18 | Frozen mit Live-Höhe statt `i4` | überlebt, **äquivalent** (Frage 4) |
| S01 | Resolver: Settings vor Seed | getötet (RR-H1-Test) |
| S02 | Biome-Memo nie gefüllt | getötet (Replay mit RR-H2-Modell, C5, A2-Test) |
| F01 | `blocks_motion`-Flip in `states.tsv` | getötet (17 Tests) |

Die 10 Mutanten des Erstellers (`results.json` `31f4a133…`) stimmen mit der Delivery
überein. Sein einziger Überlebender, `snow-counter-only-in-range`, ist M10.

### A.4 Forbidden Claims

Scanner sind die Funktionen aus dem Archiv `scripts/check-forbidden-claims.py`:
`markdown_blocks`, jeweils ohne und mit `force_plugin`, und `rustdoc_blocks`.
Gescannt habe ich:

- die Delivery, die Referenz-README und das Rustdoc aller neuen und geänderten
  World-Dateien: 0 Befunde;
- die ganzen Dateien `README.md` und `INVARIANTS.md`: 1 bzw. 4/5 Treffer. Alle haben
  denselben Block-Digest wie in der Basis `9f842bc4`; keiner liegt in den neuen
  Abschnitten.

Eine Schlüsselwortsuche nach Leistungs-, Kompatibilitäts- und Garantieaussagen findet
in den neuen Texten nur Ausschlüsse.

## Teil B — Source-Review

### B.1 Semantik gegen das Original (`SurfaceSystem.java`, Paper-applied)

| Konstrukt | Fundstelle | Abgleich |
| --- | --- | --- |
| Spaltenreihenfolge, `i4`, Biom bei `(x, i4, z)`, Prepass, `i5`, `updateXZ` | `driver.rs:367-401` gegen `:106-121` | Reihenfolge exakt; `useLegacyRandomSource` ist für alle drei Settings `false` |
| Schleife: Luft, Fluid (`i7` nur beim ersten), Stein mit `i8`-Suche bis `minY − 1` | `driver.rs:404-454` gegen `:124-154` | Neue Suche bei `run_bottom >= y`, also auch erneut am Laufende; `evaluate` nur beim exakten Default-Zustand |
| `setBlock` und Heightmap | `driver.rs:459-490` gegen `BlockColumn`/`ProtoChunk`/`Heightmap.update` | nur in `[−64, 319]`; Fluid-Flag; Anheben bei Nicht-Luft über der Spitze, Rescan bei Luft genau an der Spitze bis −65 |
| Eroded | `driver.rs:503-553` gegen `:205-231` | `!(min <= 0)`, NaN-Semantik, `Mth.floor`, Abbruch bei `is(WATER)`, Halt an `is(stone)`, Start bei `min(floor, 319)` (äquivalent, C5) |
| Frozen | `driver.rs:556-655` gegen `:233-270` | `!(min <= 1.8)`, Melt auf dem Spaltenbiom, `m1`/`d3`/`d4`, frisches `root.at(x, 0, z)`, `nextInt(4)` und `nextInt(10)`, Short-Circuit mit höchstens einem f32-gerundeten Draw, Schnee vor Packeis, veraltetes `i4` |
| Java-Helfer | `driver.rs:667-720` | `Math.min` mit NaN und −0.0, `(int)` saturierend, `Mth.floor` mit Wrapping |

### B.2 Vertrag

- **Refusal-Reihenfolge** (`driver.rs:163-253`):
  1. Puffer `Empty`;
  2. Binding (Seed, Settings);
  3. Material (Seed, Settings, Geometrie inklusive Array-Längen, Chunk = Resolver-Zentrum);
  4. Session-Zulassung;
  5. Allowance.

  Die Tests pinnen jede Präzedenz mit Kombinationen und Budget `R − 1`
  (`refusals.rs:66-165`).
- **D1–D5** sind umgesetzt:
  - D3 über `published = None` am Anfang und in jedem Fehlerarm.
  - D5: Es wird kein `OCEAN_FLOOR_WG` veröffentlicht; die Fluid-Flags sind
    Material-Bitmap ODER Fluid-Writes.
  - Die Policies stehen im Modul-Rustdoc.
- **Schranken:** `W_col`, `R`, `C0` und der Bind-Zensus 8355 werden aus den Formeln
  berechnet und gepinnt. Budget-Domänen 0/1/max/max+1, Zensus 8354/8355/8356,
  Alphabet-Cap 34/35/36 und die Retained-Byte-Caps N−1/N/N+1 sind getestet.
  Gemessen: Bindung 10 816 B, Puffer 406 592 B.
- **A1:**
  - Die Halo-Verweigerung `ChunkOutOfRange { x, 1875000 }` und `CoordinateDomain` am
    Zentrum (1874999, 0) sind mit dem eigenen echten Array gepinnt; damit gilt Domäne
    vor Präsenz.
  - Die zugelassenen Kanten {−1874999, 1874998} sind auf beiden Achsen getestet: Alle
    neun Halo-Produzenten laufen, und der Resolver scheitert nur an der Präsenz.
  - Dass `build_chunk` nie erreicht wird, folgt aus dem Typ, weil `SurfaceChunkInput`
    einen gebauten Resolver verlangt. Siehe RR10-H5.
- **Unerreichbare defensive Arten:** `Temperature` ist ungetestet; die Delivery legt
  das offen. Frozen-Spaltenbiome stehen immer in der Tabelle. `Noise` aus dem Sampler
  ist unerreichbar, weil die Koordinatengrenze 2,1·10⁹ weit über dem
  Domänenmaximum von etwa 3,8·10⁷ liegt.

### B.3 Offengelegte Punkte der Delivery

1. **`W_col`/`R`-Abweichung:** akzeptieren (Frage 1).
2. **Start oberhalb 320 als Verweigerung:** akzeptiert (Frage 2).
3. **Quellbasierte Kanten ohne echten Fall** (`i4 = 320`, Steep mit im Chunk
   geänderter Höhe, Floor über 319, Minimum unter −64): Die Vertragstests decken sie
   ab, mit der Einschränkung aus RR10-H1.
4. **Prozessabweichung des Erstellers:** Ein lesendes `git diff --stat` lief einmal
   versehentlich im gemeinsamen Baum; es ist offengelegt. Das ist für Artefakte ohne
   Folge: Archiv, Freeze und gemeinsamer World-Baum sind gleich den Pins. Die
   Bewertung als Prozessverstoß liegt beim Root.

## Befunde nach Schwere

**Blockierend:** keine. **Mittel:** keine.

**Hinweise (nicht blockierend; Testauflagen für das nächste World-Ticket):**

- **RR10-H1 — Die Produktions-Höhenableitung bei y 319 ist nicht getestet.**
  - Ort: `driver.rs:285-290` (`run`). Der C5-Test `column_loop_at_i4_320…`
    (`control.rs:432-503`) läuft über `controls::Synthetic::new` (`driver.rs:753-758`),
    und dort steht eine Kopie derselben Schleife.
  - Szenario: Mutant M13 (Ableitung ab 318) besteht alle 23 Tests. Ein Material mit
    einem Block bei y 319 bekäme dann `i4 = 63` statt 320.
  - Beleg: `mutants.json` M13 und `evidence/probe-kills-m13.json`. Die Probe ist auf
    dem Kandidaten grün und tötet M13. Der Code ist korrekt.
  - Empfehlung: die Probe übernehmen. Sie nutzt `build_observed` mit `with_states`
    (Stein bei y 319) und prüft `i4`, den ersten Aufruf und die veröffentlichte Höhe.
    Alternativ sollte `Synthetic::new` die Produktionsableitung aufrufen statt sie zu
    kopieren.
- **RR10-H2 — Wasser gegen Lava ist in den Extensions nicht gepinnt.**
  - Ort: `driver.rs:538` (Eroded-Abbruch) und `:639` (Frozen-Wasserklausel). Die
    C5-Säulen enthalten keine Lava.
  - Szenario: M14 (Lava zählt als Wasser im Frozen-Loop) und M15 (Eroded bricht an
    Lava ab) bestehen alle Tests. Für zugelassene Eingaben ist das nur
    unwahrscheinlich, nicht beweisbar unerreichbar: Aquifer-Lava liegt tief, und
    Höhlen sind möglich.
  - Beleg: `mutants.json` M14 und M15 sowie
    `evidence/probe-kills-review_probe_lava_is_not_water_in_extensions.json`. Die
    Probe ist auf dem Kandidaten grün und tötet beide. Der Code ist korrekt.
  - Empfehlung: die Probe übernehmen.
- **RR10-H3 — Die Retained-Byte-Pins sind nur teilweise literal.**
  - Ort: `work.rs:12-14` (`10_240 + size_of::<…>()`).
  - Szenario: Ändert sich die Größe von `SeededSurfaceColumns` oder
    `SurfaceChunkBuffer`, bleibt der Test grün, solange der Cap hält. GEN-05.9 hat
    die Bytes literal gepinnt.
  - Empfehlung: 10 816 und 406 592 literal pinnen. Die Zahlen gelten für x86-64 mit
    dieser Toolchain und sind von mir gemessen.
- **RR10-H4 — Die Allokationsprobe misst nur die Allowance-Verweigerung.**
  - Ort: `tests/allocation.rs`.
  - Beleg: Meine Messung über Binding, Material, Session und `UnclassifiedState`
    ergibt 0/0/0. Die README-Aussage „does not allocate“ ist damit gedeckt.
  - Empfehlung: diese Arten in den Probe-Test aufnehmen.
- **RR10-H5 — Die A1-Assertion „Puffer bleibt `Empty`“ ist leer.**
  - Ort: `refusals.rs:317-318`. Ein frischer Puffer ist immer `Empty`.
  - Die Aussage selbst stimmt aus strukturellem Grund: Ohne Resolver gibt es kein
    `SurfaceChunkInput`. Kein Handlungsbedarf außer einer genaueren Testbeschreibung.

## Delivery-Abgleich

- **Bestätigte Zahlen:**
  - World 383 + 4 ignoriert in 9 Suites, Column 23 + 1;
  - Verbraucher 94 Suites, 1489 + 1;
  - Replay 70 / 17 920 / 2 261 637; Writes 139 / 1 195 094 / 9886; 838 Steep;
  - C4: 2570 Zeilen, 15 420 / 15 420 / Kontrolle 0, Draws 2570, ungerundet 2570,
    Melt 2570 (2269 wahr);
  - C3: 512 / 102 / 6 und 256 / 480 / 64;
  - RR-H2: 405 539 / 1 856 098, maximal 72;
  - Maxima 517 / 270 785 / 784 542;
  - Bytes 10 816 / 406 592 (576 / 1088 Wertanteil);
  - Zensus 8355;
  - Allokation 3150 / 984 339 und DRIVE 0;
  - die 10 Mutanten.
- **Nicht vergleichbar** sind die Binär-Hashes der Allokationsläufe; sie hängen vom
  Pfad ab.
- **Bestätigte Hashes:** Vertrag `bd69a4ea…`, Review `c5cf5ab5…`, Freigabe
  `05d68fef…`, Workorder `f9862c96…`, Oracle-Freeze `ce970306…`, `source.freeze`
  `3b457b81…`, `evidence.freeze` `af30fb8f…`, Kandidat `cae56ed8…`, Fixture-Manifest
  `98fcf62a…`.
- **Nicht-Behauptungen:**
  - keine TPS-, RSS-, Latenz- oder Vollserver-Aussage;
  - keine Aussage zu Survival, Java-Plugins oder Aktivierung;
  - nichts installiert den Puffer in einen Live-`Chunk`.

  Ich habe keine widersprechende Behauptung gefunden.

## Artefakte (`.codex-tmp/gen05_10-runtime-review/`)

| Datei | Inhalt |
| --- | --- |
| `review.freeze.json` | Freeze der ganzen Review-Evidenz einschließlich SHA-256 dieses Dokuments |
| `evidence/integrity-before.json`, `evidence/integrity-after.json` | Integrität vorher und nachher |
| `evidence/checks-summary.json`, `logs/` | frische Checks |
| `evidence/recount.json`, `tools/recount.py` | Python-Zweitweg |
| `evidence/mutants.json`, `tools/run_mutants.py`, `logs/mutants/` | 22 Läufe |
| `evidence/alloc-{debug,release}/`, `evidence/alloc-refusals-{debug,release}/` | Allokationsmessungen |
| `evidence/probe-kills-*.json`, `probe/review_probe.rs` | Probes und Lückenbelege |
| `evidence/forbidden-claims-{candidate,base}.json` | Scan |

## Root-Integrationsvermerk (12.10.2026)

Root übernimmt das Verdikt CLEAN_RUNTIME und nimmt die durch A4 bedingten
Grenzen `W_col` = 2698 und `R` = 888.832 an (Antwort 1 oben). Die Hinweise
RR10-H1 bis RR10-H5 sind nicht blockierend und gehen als Testauflagen an das
nächste Worldticket. Original-SHA-256 dieses Dokuments vor dem Vermerk:
`b3533f610afc8b6193c96cda8477e23d9fc8fd48cafdde3bf03c7f942ee72a01`.
