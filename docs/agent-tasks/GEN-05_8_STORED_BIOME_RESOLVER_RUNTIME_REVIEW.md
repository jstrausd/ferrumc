# GEN-05.8 — unabhängige Runtimeabnahme Stored Biome Resolver

Stand: 11.10.2026. **Verdikt: CLEAN_RUNTIME** — kein blockierender Befund.
Es gibt eine nicht blockierende Auflage (R-A1: Testlücke bei der dokumentierten
Halo-Reihenfolge) und sechs Hinweise. Der Reviewer hat den Kandidaten nicht
erstellt. Geschrieben wurde nur hier und unter
`.codex-tmp/gen05_8-runtime-root-review/**` (außer `combined/`). Es gab kein
Git, keinen Agenten und keine Änderung an Kandidat, Evidenz oder Produktion.
Maschinenlesbar: `.codex-tmp/gen05_8-runtime-root-review/final-assessment.json`.

## Geprüfter Kandidat

| Gegenstand | SHA-256 |
| --- | --- |
| Delivery `GEN-05_8_STORED_BIOME_RESOLVER_RUNTIME_DELIVERY.md` | `64545852d59bed9de25fe161caea46444046453f4b02482732678303297ed80b` |
| `source.freeze.json` (246 Einträge) | `b73701c330553e6f03adbf300ad98553ba19d4e55840a7a225b823672b88fa47` |
| `evidence.freeze.json` (32 Einträge) | `41cc29bad4eb2da78e3030f1e7af950f92b851f607cce0d88718140354e95ace` |
| `candidate-01.source.json` | `24a90f864a3d2b3c780d5b8b17e398f06b6948cad88e79869d2b5a705bf26d95` |
| `combined.freeze.json` (1290 Dateien) | `cd01b062bd6e94cd7326e4ba542cd2ae20a92a917110e4f5937ced54c2c356eb` |
| Oracle-Freeze `.codex-tmp/gen05_8/delivery/freeze-manifest.json` | `d838de6c3af6d512d9b47741d8ed2be8b476337885bb0487d1a51435ea296cee` |
| `stored_biome.rs` | `6bf875eddc023d2c4fb3dbf5d9d2e5cdb080eb5502c5e8333af2c2cac68768a3` |

Audit vor und nach allen Läufen (`audit-before.json`, `audit-after.json`,
Skript `audit.py`):

- `combined/` ist gleich dem Freeze (1290 Dateien).
- Alle 246 Kandidateneinträge im Arbeitsbaum stimmen.
- Im Arbeitsbaum gibt es keine zusätzliche World-Datei, nur die vier bekannten
  `__pycache__`-Caches.
- Der Kandidat ist bytegleich zu `combined`.
- Alle 32 Evidenzeinträge stimmen.
- Die private Kopie `work/` war vor den Checks (ohne `server/target`)
  bytegleich zu `combined/`. Danach enthält sie nur zwei zusätzliche
  Testartefakte des Plugin-Testhosts (`work/.codex-tmp/p57-plugin-testhost-*/…so`),
  die beim Verbraucherlauf entstanden sind; keine Quelldatei wurde verändert.

Gegenüber der Basis `75f9824a` ändert die Kombination nur:

- `INVARIANTS.md`;
- `density/chunk.rs` (`pub(crate) fn admit_chunk`);
- `density.rs` (eine Zeile `pub(crate) use`);
- `lib.rs` (Moduldoku und `pub mod stored_biome`);
- neu: `stored_biome.rs`, `stored_biome/tests/**`, die Fixture,
  `tests/reference/stored-biome/**` und die Delivery.

## 1. Ausführung (frisch, auf der privaten Kopie `work/`, Target `target/`)

Alle Cargo-Läufe mit `--jobs 2 --locked`, Rust 1.99.0, x86_64.

| Check | Ergebnis |
| --- | --- |
| `cargo test -p ferrumc-world` Debug | 9 Suites: 320 bestanden, 0 fehlgeschlagen, 2 ignoriert (lib 264+1, Integration 46+1, Doc 10) |
| dasselbe `--release` | identische Zahlen |
| `cargo fmt --all --check` | bestanden |
| `cargo clippy --workspace --all-targets -- -D warnings` (Workspace-pedantic) | bestanden; strenger als die Delivery, die nur `-p ferrumc-world` prüfte |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-world --no-deps` | bestanden, auch in einem expliziten zweiten Lauf |
| Verbraucher `cargo test -p ferrumc-anvil -p ferrumc-storage -p ferrumc-sim -p ferrumc-testkit -p ferrumc-bench -p ferrumc-app` (auf Basis-Storage, nicht auf dem WIP) | 77 Suites: 1195 bestanden, 0 fehlgeschlagen, 1 ignoriert; Exit 0 (die Panic-Meldung im Log stammt aus einem absichtlichen Panic-Isolationstest, der besteht) |
| ignorierter Harness `stored_biome::…::measured_build_clone_and_every_original_query`, Debug und Release (eigener kalter Prozess, `measure_allocations.py`) | BUILD, CLONE und QUERY: je 0 Aufrufe, 0 Bytes, 0 Frees. CONTROL: 1 Aufruf, 4096 Bytes, 1 Free. 67 zugelassene und 60 verweigerte Builds, 100 Kopierrunden, 532576 Abfragen. Stack-Writes BUILD: Debug 20652, Release unter der 16384-Marge |
| ignorierter Harness GEN-05.7 `measured_compile_clone_and_every_original_query`, Debug und Release | 0/0/0 in COMPILE, CLONE und QUERY; CONTROL 4096 B. Stack COMPILE: Debug 425864, Release 72632 (gleich der .7-Abnahme) |
| Layout (Debug = Release, bytegleiches Log) | identity 24, budget 4, quart 12, neighborhood 128, surface 136, error 88, error_kind 8, Referenz 8, Array-Objekt 56, Scratch-Zensus 1472, geliehen 504 + 18432 |
| compile_fail (private Kopie des Autoren-Extraktors auf `work/`) | Alle 6 Beispiele scheitern mit genau einem Fehler (5× E0308, 1× E0515); alle 6 korrigierten Gegenstücke kompilieren |

Die Binärhashes weichen von der Delivery ab (Debug `9f801ee4…`, Release
`126e824f…` statt `1b1fde15…`/`345cc9e9…`), weil der Zielpfad in die Binaries
eingeht. Die Messwerte sind identisch. Delivery-Zahlen 131392 / 254592 /
482400 / 0 Allokationen / 4096 B / Layout / 6 compile_fail sind durch die
Census-Assertions der bestandenen Tests und die eigenen Messungen bestätigt.
Die 11 Autorenmutanten (`mutants/results.json`, `99535de1…`) sind als
Evidenz unverändert; eigene Mutanten siehe Abschnitt 4.

## 2. Fixtures (A2, ausgeführt)

- `stored_biome_fixture.py generate`, normal und `-O`, in frische
  Verzeichnisse: alle vier Dateien bytegleich zur committeten Fixture.
  - `queries.tsv` `7e326103…`
  - `arrays.tsv` `e496dfcc…`
  - `histories.tsv` `45a423cc…`
  - `manifest.json` `79732818…`
- `validate`, normal und `-O`: beide Exit 0 mit bytegleicher Ausgabe.
- Eigenes, vom Generator unabhängiges Skript (`fixture/handcheck.py`):
  - prüft den Freeze-Hash und die vier Pins der **rohen Java-TSV-Ausgaben**
    (`final-normal/{vanilla,paper}/{queries,values}.tsv`), also nicht der
    JSONL-Projektion;
  - expandiert alle 220960 Fixture-Zeilen mit den Manifest-Tabellen und
    vergleicht jedes Feld mit beiden Modi: 0 Abweichungen, Exception-Feld
    überall `none`;
  - vergleicht alle 243 Arrays (300096 Werte) mit den rohen Getter-Zeilen
    (`values.tsv`, Indexfolge 0..n-1) beider Modi: 0 Abweichungen.
- Hook-Zensus unabhängig nachgezählt: B outside 1872, G outside 336, G mit
  Block-X/Z außerhalb ±30M 1008, Q outside 2352. Zusammen 5568. Die 1008 sind
  ausschließlich G-Zeilen.
- 22 Zeilen von Hand gegen die Rohdaten geprüft, alle in beiden Modi gleich
  (Details in `fixture/handcheck.json`):
  - negative Saum-Ecke;
  - Clamp unter `minY` und über `maxY`;
  - Y = i32-Extreme in den Geometrien 2147483632/16 und i32::MIN/16;
  - B negativ und positiv;
  - G mit Weltseed −1 sowie den obfuskierten Seeds 0 und 2^28;
  - Hook-Zeilen bei X = i32::MIN und X = 30000001, Q-Hook, B-Hook;
  - generierte Gruppen: cold, forward (−17/9), Nether reverse, End bei Chunk
    65, i64 min und max, Accessor −80/416, single_biome_surface −64/512.
- Herkunft der Erwartungswerte: Der Generator kopiert nur Tokens (`resource`,
  `qx..cz`) und ersetzt Namen durch Tabellenpositionen. Seine einzige
  Sortierprüfung ist eine Ordnungsbehauptung über die eigenen Original-Quarts,
  kein Erwartungswert. Die Rust-Tests vergleichen gegen diese Tokens. Der
  delegierte Quart wird zwar durch den .6-Selector bzw. `>>2` berechnet, aber
  **gegen** den Original-Quart geprüft, nicht als Erwartung eingesetzt. Kein
  Erwartungswert stammt aus der Rust-Implementierung oder einer eigenen Formel.

## 3. Source-Review gegen Vertrag und A1–A5

| Punkt | Bewertung |
| --- | --- |
| Identität, Nachbarschaft (`stored_biome.rs:638-690`) | Reihenfolge Fenster → Key-Domain → Duplikat → Seed → Settings → Source → Geometrie → `ValuesPerChunk` → Länge → `BorrowedValues` → Historie. Entspricht dem Vertrag; die zwei Kapazitätsprüfungen sind offengelegt und dort unerreichbar oder harmlos |
| Arbeit vor Scan/Hash (`:525-542`) | `2+10N+values(+9)`, `charged > allowance`, dann Scan, dann einmaliges `from_world_seed`. Keine Mutation bei Fehlern (nur `&`-Borrows, Rückgabe per `Result`) |
| Signed Chunk, Maske, Clamp, Index (`:575-603`) | `>>2`, Fensterdeltas in i64, Domain, dann Missing. Clamp nur auf Y in i64, Index in i64 mit `try_from`/`get`. Kein Überlauf möglich (Quart Y nach dem Clamp ≤ 511 über `minQY`) |
| Getrennte ID-Arten | `StoredBiomeQuartPos`, `BiomeQuartPos`, `ClimateQuartPos`, `BlockPos`, `BiomeResourceId`; durch compile_fail belegt |
| Typisierte Verweigerung ohne Fallback | Keine Default-/Plains-Pfade; `InvalidStoredIndex` ist nur defensiv |
| Kapazitäten N-1/N/N+1 | `native_limit` direkt für alle vier Limits; Retained über den echten Admissionspfad; ValuesPerChunk 2048/2049 über Controls; BorrowedValues an der echten 18432-Grenze |
| Allokationsfreiheit | Siehe Ausführung. Der Beobachtungspunkt existiert auch in der gemessenen Testbinary, allokiert aber nicht |
| Autorität und Lebenszeit | Kein Getter gibt `&ChunkBiomeSamples` heraus. `neighborhood()` liefert nur `&StoredBiomeNeighborhood<'a>`. Der Kern ist privat. `ChunkBiomeSamples` hat private Felder, keinen Clone und keinen Konstruktor (`biome_selection/arrays.rs:76-85`). Die Außenslice wird nicht gehalten |
| `unwrap`/`expect`/`unsafe`/`panic` außerhalb von Tests | Keine (nur in `compile_fail`-Doku); `forbid(unsafe_code)` unverändert |
| Rustdoc | Jedes öffentliche Item, jede Variante und jedes Variantenfeld in `stored_biome.rs` dokumentiert (gelesen; zusätzlich `clippy -p ferrumc-world --lib -- -W missing_docs`: 0 Warnungen in `stored_biome.rs`. Der Lint war aktiv, er meldet 71 Warnungen in vorbestehenden Modulen, siehe `checks/missing-docs.log`) |
| **A1** | `admit_chunk` ist `pub(crate)` (`density/chunk.rs:563`), die Re-Exportzeile `pub(crate) use chunk::admit_chunk;` steht in `density.rs:16`. Nicht öffentlich exportiert: `density` ist `pub mod`, aber `pub(crate)` hebt die Sichtbarkeit nicht über das Crate hinaus. Die Zeile ist zwingend, weil `mod chunk` privat ist; minimal und offengelegt (siehe H-1). Grenzen ±1_875_000/1_874_999 auf beiden Achsen und die i32-Extreme sind getestet |
| **A2** | Siehe Abschnitt 2. Die 5568 Hook-Zeilen sind Policy-Fälle: jede wird am Kernel mit `OutsideNeighborhood` und dem Original-Chunk geprüft (`tests/oracle.rs:73-90`), nie als Gleichheit gezählt. Die 32 Exception-Zeilen sind nur Evidenz; die 30 Accessoren werden von `BiomeQuartGeometry` verweigert |
| **A3 (Ersatz)** | `OBSERVED_ZOOM` und der Schreibzugriff stehen ausschließlich unter `#[cfg(test)]` (`stored_biome.rs:481-485`, `618-619`). Keine Signatur oder API ändert sich. Der Lauf ohne `cfg(test)` (Reviewer-Probe R5, Integrationstest) liefert alle 3840 G- und 360 Q-Zeilen der forward-Gruppe identisch. Die Mutanten M1, M2, M3, M4, M5 und M7 werden getötet, M6 nicht (Grenze, siehe H-2). Bitgleichheit aller 1024 Buckets (`(b/1024-0.5)·0.9` vs `(b-512)·(0.9/1024)`) unabhängig nachgerechnet: 0 Differenzen |
| **A4** | `BlockCoordinateDomain { axis, coordinate }`, X vor Z (`:726-740`), vor Spalten- und Lookup-Prüfung. `block()` gesetzt, `chunk()` leer. Als Policy dokumentiert (`:249`, INVARIANTS) |
| **A5** | Offengelegt in Delivery, Fixture-Manifest (per Test geprüft), Referenz-README und INVARIANTS |
| Halo zweiphasig (`:999-1010`) | Erfüllt „all nine required keys in slot order: their .3 domain first, then presence“. Kein Fehler wird verloren: Ein außerhalb der Domain liegender Slot kann nie geliefert werden, weil die Per-Array-Admission ihn vorher verweigert. Ein Randzentrum kann den Halo also nie vervollständigen. `CoordinateDomain` ist dort die grundlegende Ursache; ein zusätzlich fehlender In-Domain-Slot bleibt für den Sparse-Lookup als `MissingChunk` sichtbar (R1). Siehe Auflage R-A1 |
| 1008 Hook-Zeilen | Vertragsreihenfolge „Block methods first refuse X, then Z … before .6 selection“ wird eingehalten. Jede dieser Zeilen wird zusätzlich am Kernel gegen den originalen delegierten Quart und Chunk mit `OutsideNeighborhood` geprüft. Damit bleiben alle 5568 vollständig als Policy-Fall abgedeckt; A2 ist erfüllt |

## 4. Eigene Probes und Mutanten (private Kopie `mutwork/`)

Reviewer-Tests (alle bestehen auf dem Kandidaten):

- **R1** `edge_surface_halo_reports_domain_before_any_missing_in_domain_slot`:
  Randzentren auf X+, X− und Z+. Zusätzlich fehlt ein In-Domain-Slot. Die
  Surface liefert dennoch den ersten Domain-Key, der Sparse-Lookup liefert
  `MissingChunk`.
- **R2** `negative_centre_surface_column_edges_with_real_forward_arrays`:
  echte forward-Arrays bei (−17, 9). Spaltenränder −273/−272/−257/−256 und
  143/144/159/160, fünf Y-Werte und beide Semantiken.
- **R3** `combined_defects_in_one_entry_follow_the_documented_order`: mehrere
  gleichzeitige Defekte in einem Eintrag, Duplikat vor Identität, Fenster vor
  Key-Domain.
- **R4** `block_policy_boundaries_both_signs_and_axis_priority`.
- **R5** `tests/reviewer_public_composition.rs`: nur öffentliche API, ohne
  `cfg(test)`. Echte forward-Gruppe mit einer geteilten Session, alle G-Zeilen
  in beiden Semantiken (Nachbarschaft und Surface) und alle Q-Zeilen.

Baseline in `mutwork/`: 33 bestanden (29 Kandidat + 4 Reviewer), 1 ignoriert,
R5 bestanden. 21 Mutanten (`mutants.py`, `mutants/results.json`); nach jedem
Mutanten wurde die Quelle per Hash zurückgesetzt (`6bf875ed…`):

| ID | Mutation | Ergebnis |
| --- | --- | --- |
| M1 | Vanilla fest im Nachbarschaftspfad | getötet (Beobachtung) |
| M2 | Paper fest nur im Surface-Pfad | getötet (Beobachtung) |
| M3 | nicht abgeleiteter Seed (`from_obfuscated_bits(seed)`) | getötet |
| M4 | aus falschem Weltseed abgeleitet (`seed ^ 1`) | getötet |
| M5 | Semantik vertauscht | getötet |
| M6 | korrekt beobachtet, aber mit festem Vanilla selektiert | **überlebt** (erwartet, H-2) |
| M7 | korrekt beobachtet, aber mit Seed 0 selektiert | getötet über Ausgabe (Achsen- und echte Gruppen) |
| P1 | lokale X/Z-Maske nur bei negativem qX vertauscht | getötet |
| P1b | Chunk-Key vertauscht nur bei qX<0 und qZ<0 | getötet (Achsen-Oracle) |
| P2 | oberer Y-Clamp ohne −1 | getötet |
| P2b | unterer Clamp −1 | getötet |
| P3 | Halo-Slot-Keys transponiert | getötet |
| P4 | Domain-Durchlauf des Halos entfernt | getötet |
| P4b | Halo einphasig (je Slot Domain, dann Presence) | **nur durch R1 getötet** (R-A1) |
| P5 | Surface-Spalte per `/16` statt `>>4` | getötet |
| P6 | Policy-Untergrenze um 1 verschoben | getötet |
| P7 | Domain-Prüfung des gewählten Chunks entfernt | getötet |
| P8 | Initial-Selektion nicht validiert | getötet |
| P9 | Surface-Arbeit (+9) nicht berechnet | getötet |
| P12 | A4-Payload meldet X statt Z | getötet |
| P13 | A4-Reihenfolge Z vor X | getötet |

## Befunde nach Schwere

### Blockierend

Keine.

### Auflage

**R-A1 — Die dokumentierte zweiphasige Halo-Reihenfolge ist von der
Kandidaten-Suite nicht festgelegt.**

- Ort: `stored_biome.rs:999-1010`; Behauptung in Delivery Z. 109-112 („An edge
  centre therefore always refuses with `CoordinateDomain`, regardless of which
  arrays were supplied“) und INVARIANTS.
- Szenario: Ein Refactor auf eine einphasige Prüfung (Mutant P4b) liefert für
  ein Randzentrum mit zusätzlich fehlendem In-Domain-Slot `MissingChunk` statt
  `CoordinateDomain`. Alle 29 Kandidatentests bestehen trotzdem. Der einzige
  Randtest (`sparse_and_surface_centre_domains_at_signed_chunk_extremes`)
  liefert alle In-Domain-Slots, und dann stimmen beide Varianten überein.
- Beleg: `mutants/results.json` → P4b `killed_only_by_reviewer: true`.
- Kein Produktionsfehler: Das Verhalten des Kandidaten ist korrekt und
  vertragskonform (R1 besteht).
- Auflage: Einen R1-gleichwertigen Test im nächsten World-Ticket oder bei der
  Integration übernehmen. Die Refusal-Reihenfolge gehört laut Freigabe in die
  Tests.

### Hinweise

- **H-1** `density.rs:16`: A1 nannte nur `density/chunk.rs` und `lib.rs`; die
  zusätzliche `pub(crate) use`-Zeile in einer .3-Datei ist technisch zwingend,
  crate-privat, minimal und offengelegt. Akzeptabel.
- **H-2** A3-Ersatz, Grenze: Der Beobachtungspunkt erfasst das an `zoomed`
  übergebene `BiomeZoom` unmittelbar vor `zoom.select_quart`. Eine Mutation,
  die innerhalb von `zoomed` danach einen anderen Modus verwendet (M6), bleibt
  unentdeckt. Da beide Modi auf allen 1024 Buckets bitgleich sind, hat das
  keine beobachtbare Auswirkung. Der Seed dagegen ist auch über die Ausgabe
  diskriminiert (M7). Die Delivery formuliert das korrekt als
  „observational“.
- **H-3** Forbidden-Claims: Die Scan-Funktionen (`markdown_blocks`,
  `rustdoc_blocks`, `scan` aus `scripts/check-forbidden-claims.py`, SHA-256
  `eb2671f2…`) wurden direkt auf Delivery, World-README, INVARIANTS,
  Referenz-README sowie `stored_biome.rs`/`lib.rs`-Rustdoc angewendet.
  - 0 Funde in Delivery, Referenz-README und Rustdoc.
  - 5 Funde in World-README (Z. 162 „isolated“) und INVARIANTS (Z. 218, 230,
    514, 563, englischer Kompatibilitätsbegriff). Alle sind mit identischem Block-Digest bereits
    in der Basis vorhanden (dort Z. 180/192/476/525), stammen also nicht vom
    Kandidaten. Sie liegen außerhalb des CI-Scopes (`README.md` im Root und
    `docs/`).
  - Der neue GEN-05.8-Abschnitt erzeugt keinen Fund.
- **H-4** Die Delivery nennt Clippy nur für `-p ferrumc-world`; der hier
  ausgeführte Workspace-Lauf mit allen Targets ist ebenfalls grün.
- **H-5** H5 (End-IDs) ist in der Delivery korrekt als offen ausgewiesen
  (Z. 300). Der Vertrag nennt es optional.
- **H-6** Methodik: `cargo test` bricht nach der ersten fehlschlagenden
  Test-Binary ab. Für getötete lib-Mutanten lief R5 deshalb nicht. Das ändert
  keine Tötungsentscheidung.

## Delivery-Abgleich

- Zahlen: 320+2, 131392, 254592, 482400, 0 Allokationen mit Kontrolle 4096 B,
  Layout, 6 compile_fail. Die 11 Mutanten liegen als unveränderte Evidenz vor.
  Alle Zahlen sind bestätigt.
- Offenlegungen sind zutreffend:
  - Halo zweiphasig;
  - 1008 Hook-Zeilen;
  - Fehlergröße 88 statt 80;
  - Nachbarschaft 128 statt der Designzahl 136.
- Nicht-Behauptungen sind zutreffend. Es gibt keine TPS-, RSS-, Latenz-,
  Survival-, Plugin- oder Aktivierungs-Claims, keine SurfaceSystem- oder
  Carver-Ausführung und keine Mehr-Worker-Vanilla-Behauptung. Der Fallback ist
  als offen ausgewiesen, H5 als offen.

Diese Abnahme aktiviert keine natürliche Welt und behauptet keine
Gesamtserver-Verbesserung. Der nächste Schritt sind die originalen
Surface-Regeln und der SurfaceSystem-Treiber.

## Root-Integrationsvermerk

Root hat bei der Integration am 11.10.2026 nur Zeile 223 umformuliert: das
zitierte englische Fundwort wurde durch „englischer Kompatibilitätsbegriff“
ersetzt, weil der CI-Scanner das Zitat sonst als Aussage wertet. Der vom
Reviewer gelieferte Bericht hatte SHA-256 `dcb153d3272441379995f4f77a2966959ce871cc5cc0beb15cd07699f711ab42`.
