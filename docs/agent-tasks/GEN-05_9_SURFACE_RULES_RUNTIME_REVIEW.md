# GEN-05.9 — Surface-Rules-Runtime: unabhängiger Root-Review

Review vom 12.10.2026 durch einen unabhängigen Reviewer, der den Kandidaten nicht
erstellt hat. Geprüft wurden der eingefrorene Kandidat (291 World-Dateien,
`source.freeze.json` SHA-256 `dd65ad0e…a2a8a3eb`, Manifest `candidate-01.source.json`
`3fe6f959…dba08291`), die [Delivery](GEN-05_9_SURFACE_RULES_RUNTIME_DELIVERY.md)
(`a761ff39…`) und die Runtime-Evidenz (`evidence.freeze.json` `ec676844…`). Maßstab
waren die [Runtime-Freigabe](GEN-05_9_SURFACE_RULES_RUNTIME_TASK.md) und der
[Vertrag](GEN-05_9_SURFACE_RULES.md) (`1906f57e…`) mit §13 A1–A8, §8 und den
Policies P1–P6, außerdem die drei Vertrags-Reviews (R3-H1–H3) und R-A1 aus GEN-05.8.

## Verdikt: **CLEAN_RUNTIME**

Es gibt keinen blockierenden und keinen mittleren Befund. Fünf Hinweise (RR-H1 bis
RR-H5) blockieren nicht. Alle frischen Checks sind grün. Ich habe die Replay-Klassen
aus den eingefrorenen Rohzeilen unabhängig neu abgeleitet; sie stimmen überein. A5
(`clay_bands_offset`) habe ich mit einer eigenen Reproduktion des erweiterten Helpers
bestätigt, und zwar über alle 20 480 beobachteten echten Spalten, nicht nur über die
3393 Badlands-Spalten. Die Kompetenz der Tests habe ich mit 15 eigenen Mutanten
geprüft. Kandidat, `combined/` und die eingefrorene Evidenz waren vor und nach dem
Review bytegleich.

## Prüfgrundlage und Isolation

- Prüfbasis ist das readonly `combined/` (`git archive` von main `e18efd22` plus die
  291 World-Dateien): 1371 Dateien, `combined.freeze.json` `70e8cb49…`. Ich habe
  davon drei private Kopien unter `.codex-tmp/gen05_9-runtime-root-review/` angelegt:
  - `work/` für die frischen Checks, mit frischem Target `target-review`;
  - `probe/` für eigene Tests;
  - `mutant-copy/` für Mutanten.

  Im Arbeitsbaum wurde nichts gebaut.
- Integrität (`review/integrity.py`, Bericht vorher und nachher identisch,
  `cbf72771…`):

  | Prüfgegenstand | Ergebnis |
  | --- | --- |
  | `combined/` | 1371/1371 unverändert |
  | World-Dateien im Arbeitsbaum | 291/291 gleich `source.freeze.json`, exakte Dateimenge |
  | `candidate-01-src/` | 291/291 |
  | Runtime-Evidenz | 105/105 |
  | Vertragsevidenz `.codex-tmp/gen05_9/**` | 458/458 Freeze-Einträge |
  | Gepinnte Dokumente | 7/7 |

  Im World-Verzeichnis des Arbeitsbaums liegen zusätzlich vier unversionierte
  `__pycache__/*.pyc` vom 11.10. (GEN-05.6/.7). Sie waren schon vorher da und sind
  unverändert.
- Nach den Checks sind alle 1371 Quelldateien in `work/` unverändert. Dort liegen nur
  Testartefakte: `.codex-tmp/p57-plugin-testhost-*` (2 Dateien) und nach dem
  Zusatzlauf `.codex-tmp/p58-region-guard-*` (1 Datei).

## Teil A — Ausführung

### A.1 Frische Checks (cargo/rustc 1.99.0, `--jobs 2 --locked --offline`, frisches Target)

| Check | Ergebnis |
| --- | --- |
| World Debug | exit 0: 360 passed, 0 failed, 3 ignored in 9 Suites (lib 304 + 2, Integration 46 + 1, Doc 10) |
| World Release | exit 0: identische Zahlen |
| Clippy `--workspace --all-targets -D warnings` | exit 0 |
| `fmt --all --check` | exit 0 |
| World-rustdoc `-D warnings` | exit 0 |
| Workspace-Build | exit 0 |
| Direkte World-Verbraucher (storage, sim, anvil, session, testkit, bench, app; `--no-fail-fast`) | exit 0: 86 Suites, 1426 passed, 0 failed, 1 ignored. `multiplayer_visibility` war im ersten Lauf grün; ein Wiederholungslauf war nicht nötig. |
| Transitive Verbraucher (zusätzlich, RR-H4): `ferrumc-plugin-block-rules`, `ferrumc-plugin-region-guard` | exit 0, 15 passed |
| Allokationsproben, gezielt je einmal (die drei ignorierten Tests über ihre Runner) | siehe unten |

Ergebnisse der Allokationsproben:

- `surface_rules`, Debug und Release: CONTROL 1 Aufruf / 4096 B / 1 Free. BUILD 3341
  Aufrufe / 638 395 B / 3248 Frees. EVALUATE 0/0/0 über 38 889 Sessions. Der
  BUILD-Stack-High-Water liegt bei 136 488 B (Debug) bzw. 21 688 B (Release). Alle
  Werte sind identisch mit der Delivery.
- `stored_biome`: Kontrolle positiv, alle anderen Phasen 0/0/0.
- `biome_temperature`: Kontrolle positiv, alle anderen Phasen 0/0/0.

Logs liegen in `checks/`, die Zusammenfassung in `checks/summary.txt`.

### A.2 Replay (A1): eigene Nachzählung und Abdeckungsnachweis

- **Unabhängige Ableitung aus den Rohdaten** (`review/recount_replay.py`): Grundlage
  ist das eingefrorene `round2/oracle-normal/paper/rows.tsv` (765 138 Rohzeilen,
  `ef85672c…`). Daraus habe ich für jede gehaltene G/C-Zeile drei Dinge nach der
  §6-Regel selbst abgeleitet:
  - `run_start` (Spaltenzelle gesetzt, Fortsetzung über gleiche
    tier/group/`update_xz`/ctx_x/ctx_z);
  - die Klasse: `full` heißt, der ganze Präfix ist gehalten und zugelassen;
  - die P4-Art (`y`, dann Steinwerte; Roh-Wasser `Integer.MIN_VALUE` = `None`).

  Ergebnis:

  | Tier | `full` | `partial` | `refused` |
  | --- | --- | --- | --- |
  | G | 2490 | 4230 | 0 |
  | C | 5478 | 24 626 | 2065 |

  Gegen `replay.tsv` gibt es 0 Abweichungen bei `run_start`, Klasse und
  Verweigerungsart. Die Fixture-Eingaben sind gleich den Rohzeilen (0
  Abweichungen; `x/y/z` = `ctx_x/ctx_y/ctx_z` in allen 38 889 Zeilen).
  `steep-probe` umfasst genau die 1536 `steep/`-Zeilen.
- **Abdeckung der Runtime-Tests:** In der Probe-Kopie protokolliert
  `every_fixture_row_replays_under_its_declared_class` jede Zeile, und zwar erst,
  nachdem alle ihre Assertions bestanden sind. Ergebnis:
  - alle 38 889 Replay-Zeilen wurden genau einmal besucht;
  - es gibt 0 Klassenabweichungen;
  - jede Verweigerung hat die erwartete Art;
  - jede `full`-Zeile lief in ihrem Run ab `run_start`.

  `full`-Zeilen mit verglichenen Zellen: Noise 2455, Draws 7968, Steep 871, Minimum
  2776, Secondary 4, Spaltenzelle 2776. Das deckt sich mit der Delivery.
- **Arrays:** Die 720 von der Runtime berechneten GEN-05.5-Digests sind gleich
  SHA-256 der Original-Rosterindizes im eingefrorenen `paper/arrays.tsv`
  (`b9bfc8bc…`): 720/720, gleiche Schlüsselmenge. Die Prüfung läuft also direkt
  gegen das Rohoriginal, nicht nur gegen die Fixture.
- Die `GEN059_*`-Ausgaben des Probe-Laufs stimmen mit der Delivery überein:
  - Temperatur: 28 560 geprüft, davon 165 diskriminierend;
  - Lazy-Probe: maximal 75 Einheiten pro Aufruf, also weit unter 467.

### A.3 A5 Clay: eigene Reproduktion

- **Aufbau:** Eigene Runner-Kopie unter `clay-repro/runner/`. Gegenüber der Kopie des
  Erstellers unterscheidet sich nur `OUT_AREA`; die Pins sind neu berechnet. Der
  erweiterte Helper ist `69ac0750…`. Er läuft normal und mit `-O`, danach die Guards
  in beiden Modi. Alle vier Läufe enden mit exit 0.
- **Helper nur additiv:** `diff` gegen den eingefrorenen Helper (`eff09a3d…`) zeigt
  nur drei Ergänzungen: zwei Felder, einen reflektiven Lesezugriff
  `NormalNoise.getValue(SurfaceSystem.clayBandsOffsetNoise, x, 0.0, z)` am echten
  `SurfaceSystem` der Gruppe und den Writer für `clay.tsv`.
  - Die Feldauflösung geht über die eingefrorenen Mappings: Paper
    `clayBandsOffsetNoise`, Vanilla `m`.
  - Die Werte sind echte Original-Ausgaben, keine eigene Formel. Gegenprobe: In
    0 von 18 984 Spalten sind die Clay-Bits gleich den Bits der Surface-Noise.
- **Bestehender Output unverändert:** Alle 29 eingefrorenen Outputs habe ich auf
  beiden Seiten von der Platte neu gehasht.
  - 23 Daten-Outputs sind bytegleich mit Runde 2.
  - Abweichend sind nur die 6 frisch kompilierten Helper-Klassen.
  - Neu sind nur `paper/clay.tsv` und `vanilla/clay.tsv`.
  - `report` und Input-Pins sind gleich, normal = `-O`.
  - Beide `clay.tsv` sind bytegleich mit denen des Erstellers (`9ca939a4…`,
    `56c43547…`).
  - Guards 16/16 in beiden Modi; beide `guards.json` sind bytegleich mit den
    eingefrorenen Runde-3-Dateien (`838ecefb…`, `ac03d213…`).
- **Spaltenmenge und Fixture:** Die Spaltenmenge habe ich selbst aus den Rohzeilen
  abgeleitet: 3393 Spalten, gleich der Menge in der Fixture. Alle 3393
  Fixture-Tokens sind gleich meinen reproduzierten Tokens.
- **Bitgleichheit (eigener Vergleich):** Ein Probe-Test schreibt für jede beobachtete
  echte Spalte die Runtime-Bits der Paper- und der Vanilla-Basis heraus
  (`probe-out/clay-runtime.tsv`); der Vergleich ist `review/clay-compare.json`.

  | Spaltenmenge | Paper = echt Paper | Vanilla = echt Vanilla | echte Basen verschieden |
  | --- | --- | --- | --- |
  | alle 20 480 beobachteten | 20 480/20 480 | 20 480/20 480 | 20 480/20 480 |
  | 3393 Badlands | 3393/3393 | 3393/3393 | 3393/3393 |
- Die eingefrorene Evidenz unter `.codex-tmp/gen05_9/` ist unverändert (458/458).

### A.4 Eigene Probes und Mutanten

Eigene Tests in `probe/…/surface_rules/tests/review_probes.rs` (`f61c4c90…`), alle grün:

| Probe | Zweck | Ergebnis |
| --- | --- | --- |
| Replay- und Array-Rekorder | Abdeckung nachweisen (A.2) | 38 889 / 720 |
| `review_seed_precedes_settings_when_both_mismatch` | Bindung mit `large_biomes` und Seed 1 gegen einen Resolver mit Overworld und Seed 0 | `Seed` |
| `review_biome_memo_is_lazy_and_each_call_respects_its_precheck` | Für jede zugelassene Zeile: Resolver genau dann gerufen, wenn ein `biome`-Knoten oder der `temperature`-Singleton berechnet wurde; Aufrufkosten ≤ `467 + m` | 29 803 / 7021 / max. 75 |
| `review_clay_dump_over_every_reproduced_genuine_column` | A5 (A.3) | 20 480 Spalten |

Die 15 Mutanten (`review/mutants.py`, Ergebnisse in `review/mutants-results.json`
`b1e9299d…`) liefen jeweils gegen die ganze `surface_rules`-Suite plus
`halo_order`, im Release-Build. Ausgangslage: 43 Tests grün. Alle Quellen wurden per
Hash wiederhergestellt.

| ID | Mutation | Ergebnis |
| --- | --- | --- |
| R01 | Resolver: Settings vor Seed | **überlebt die Kandidaten-Suite**; getötet nur durch die eigene Probe (RR-H1) |
| R02 | Gradient `y < T` statt `y <= T` | äquivalent (siehe unten) |
| R03 | Gradient `y > F` statt `y >= F` | äquivalent (siehe unten) |
| R04 | `above_preliminary_surface` mit `y > min` | getötet (Replay) |
| R05 | Steep: X-Nachbarn vertauscht | getötet (Replay) |
| R06 | Biome-Memo eager bei jedem zugelassenen `evaluate` (Laziness) | **überlebt die Kandidaten-Suite**; getötet durch die eigene Probe (RR-H2) |
| R07 | Secondary-Noise eager auch bei `range == 0` (Laziness) | getötet (Replay) |
| R08 | Minimum-Lerp: Achsen vertauscht | getötet (Replay) |
| R09 | Per-Block-Zustand wird nicht zurückgesetzt | getötet (Replay, Temperatur) |
| R10 | `surface_depth` ohne Poisoned-Prüfung | getötet (`poisoned_is_checked_first…`) |
| R11 | `min_surface_level` prüft Work vor dem Cache | getötet (Work-Matrix) |
| R12 | Water mit `>` statt `>=` | getötet (Replay) |
| R13 | `hole` nicht pro Spalte gecacht | getötet (Replay) |
| R14 | M15 des Erstellers: einphasiger Halo (R-A1) | getötet (`halo_order`) |
| R15 | M16 des Erstellers: verweigerter Aufruf setzt Per-Block-Zustand zurück | getötet (P4-Test) |

R02 und R03 sind mathematisch äquivalent:

- Bei `y = T` ist `map(T, T, F, 1, 0) = 1.0`, und `nextFloat() < 1.0` ist immer wahr.
- Bei `y = F` ist die Chance `0.0`, und der Vergleich ist immer falsch.

Es ändert sich also nur, ob gezogen und als Work gezählt wird. Die
Fixture-Draw-Bits zeichnet das Oracle für jeden berechneten Gradientenknoten
unabhängig auf (`SurfaceRulesOracle.java:395`), und der Trace bildet das nach. Das
ist keine Testlücke.

### A.5 Forbidden Claims

Scanner sind die Funktionen aus `combined/scripts/check-forbidden-claims.py` (gleich
der Basis): `markdown_blocks`, jeweils ohne und mit `force_plugin`, und
`rustdoc_blocks`. Gescannt habe ich direkt die ganze Delivery, die **ganzen** World-
Dateien `README.md` und `INVARIANTS.md`, die Referenz-README und das Rustdoc aller
neuen `surface_rules`-Dateien sowie von `lib.rs` und `density.rs`, zusammen 25
Einheiten.

- Neuer Text hat 0 Befunde.
- Die Ganzdatei-Scans melden README 1 und INVARIANTS 5 Treffer (`isolated`,
  generischer englischer Kompatibilitätsbegriff). Alle liegen mit identischem Block-Digest schon in der
  Basis `e18efd22` vor, sind also nicht vom Kandidaten eingeführt
  (`review/forbidden-claims.json`).

## Teil B — Source-Review

### B.1 Semantik gegen das Original (`session.rs`, `bind.rs`, `compile.rs`)

Die einzelnen Konstrukte habe ich gegen den Original-Quelltext abgeglichen.

| Konstrukt | Fundstelle | Abgleich |
| --- | --- | --- |
| Sequence, Condition, Block, Bandlands | `session.rs:322-384` | First-match-Semantik (Sequence liefert den ersten Treffer, eine falsche Bedingung liefert `null`); expliziter Stack mit 33 Frames, keine Rekursion |
| `not` | `session.rs:387-405` | iterativ, jedes Kettenglied wird gezählt |
| Noise-Threshold | `:429-443` | `v ≥ min && v ≤ max`, pro Knoten und Spalte |
| Vertical-Gradient und Paper-Flat (Disabled) | `:531-551` | `Mth.map` als `lerp(inverseLerp)` in f64, Draw nur innerhalb `(T, F)` |
| `y_above` | `:448-455` | Java-`int`-Wrapping |
| `water` | `:456-465` | `None` = wahr, Java-`int`-Wrapping |
| `stone_depth` | `:554-580` | `(int)`-Truncation und Saturierung, Secondary pro Spalte |
| Steep | `:602-620` | asymmetrische Achsen exakt wie `SteepMaterialCondition`, Höhen lazy beim ersten Test |
| Hole | `:472-478` | pro Spalte |
| Minimum | `:623-648` | `lerp2(dx, dz, c00, c10, c01, c11)` mit f32-Deltas, `Mth.floor` + `surfaceDepth − 8` |
| Band | `:651-669` | JDK-21-`Math.round`, `(int)` auf long, Java-`%`; negativer Index ist ein defensiver `Noise`-Fehler |
| Biome und Temperatur | `:493-528` | Memo pro Block, Meeresspiegel aus den Settings (63) |
| Surface Depth | `:226-232` | `(int)(n·2.75 + 3.0 + f64(f32(nextDouble))·0.25)` |
| Clay-Bänder | `bind.rs:184-258` | `generateBands`/`makeBands` mit Original-Zugreihenfolge, inklusive `l − 1 > 0` |
| Anker | `compile.rs:92-102` | `resolveY` |
| Paper-Substitution | `compile.rs:936-944` | |

### B.2 Vertrag

- **22 typisierte Verweigerungsarten** (`error.rs:80-128`): vollständig wie in §7.
  Jede Art hat Rustdoc mit Kategorie, `category()` gruppiert sie, und
  `refusal_categories_group…` testet das.
- **Policy-Reihenfolge:**
  - Bind: P1 vor P2 vor Bind-Zensus, alles vor dem ersten Sampler
    (`bind.rs:74-84`).
  - Session `new`: Seed, Settings, Source, Geometry, ZoomSemantics, danach
    Zentrum und Session-Bytes (`session.rs:146-163`, `:673-704`).
  - Tests: einzeln je Feld; Source mit `Fixed(plains)`, `End` und
    `MultiNoise(minecraft:nether)` (R3-H3 erfüllt); Geometry mit `(-80, 416)` und
    `(-64, 400)`, jeweils mit beiden Zoom-Semantiken. Kombinationen zeigen frühere
    Felder vor ZoomSemantics, Source vor Geometry, Settings und Seed vor
    Source/Geometry. Zu Seed gegen Settings siehe RR-H1.
- **Budget `467 + m`** (`session.rs:274-297`):
  - Reihenfolge: Poisoned, NoActiveColumn, `y`, Stein oben/unten, Wasser, dann Work.
  - `467` wird aus dem Template berechnet (`compile.rs:990-1007`); der Test pinnt
    es.
  - Gegen die Schranke 467 + m zeigt mein Probe-Test, dass der Meter sie in allen
    36 824 zugelassenen Aufrufen einhält.
  - `Poisoned` ist die erste Prüfung jeder fehlbaren Methode (`admit()`).
    `work_used` ist eine unfehlbare `const fn`.
  - `WorkBudgetExceeded` vergiftet nicht. Alle Matrixzeilen aus §8 sind getestet
    (`work.rs`); „remaining 0 ohne aktive Spalte“ ist nur über `set_work_used`
    erreichbar und offengelegt.
- **Fortsetzung:** `restart_in_a_new_session_equals_an_uninterrupted_column`
  vergleicht Outcomes, Traces und `work_used` (`work.rs:307-348`).
- **Chunk-Allowance:** 45 908 736 reicht, um alle 98 304 Blöcke eines Chunks
  auszuwerten. Rechnerisch ist sie exakt: pro Spalte 2 + 468 + 383·467 im
  ungünstigsten Fall.
- **Gemessene Bytes:** Template 6622, Bindung 19 216, Session 400, gepinnt in Debug
  und Release (`limits.rs:11-49`). Die Grenzen N−1, N und N+1 sind über die echte
  Zulassung getestet (`bind_with`/`new_with`/`compile_with`), ebenso die 8
  Zensus-Caps am realen Zensus und die synthetischen Grenzen.
- **0 Allokationen** mit positiver 4096-B-Kontrolle: siehe A.1.
- **Steep-Timing** (`steep.rs`):
  - Beide A8-Kontrolltests verwenden eine echte Zeile, deren Outcome zwischen
    steiler und flacher Ansicht wechselt; der Test diskriminiert also.
  - Die Höhen werden erst beim ersten zugelassenen Steep-Test gelesen; ein
    verweigerter Aufruf liest nichts.
  - Mutant R05 bestätigt die Achsenempfindlichkeit über das Replay.
- **R-A1** (`stored_biome/tests/halo_order.rs`): Der Test pinnt die zweiphasige
  Halo-Reihenfolge; diskriminierend sind die Ränder X+ und Z+.
  `stored_biome.rs` ist unverändert (Diff gegen die Basis). R14 (einphasig) wird
  getötet.
- **Density-Sichtbarkeit (Root-Entscheidung, angenommen):** Diff gegen die Basis:
  - `density/json.rs:18-37`: `Value`, `ValueKind`, `Stats` und `parse` von
    `pub(super)` auf `pub(crate)`;
  - `density/compile.rs:891, 925`: `number` und `integer` von privat auf
    `pub(crate)`;
  - `density.rs:23-24`: zwei `pub(crate) use`.

  Kein Funktionsrumpf ist geändert. Die Module `json` und `compile` bleiben privat
  (`density.rs:9, 12`), nichts davon wird öffentlich exportiert. Die öffentliche API
  ändert sich nur durch `pub mod surface_rules` (`lib.rs:64`, A7 autorisiert). Die
  Density-Tests sind Teil der grünen World-Läufe.
- **Fixture und Referenz:** Die 14 Fixture-Dateien sind bytegleich mit
  `round3/fixture/surface-rules-original-1.21.8`. Helper, Runner, Guards,
  Projektion, `prepare.py` und `surface_fixture.py` sind bytegleich mit den
  eingefrorenen Originalen, `a5-clay-extension/*` bytegleich mit
  `.codex-tmp/gen05_9-runtime/clay-oracle`.

### B.3 Offengelegte Punkte

1. **Zensus-Budget vor Settings-Zulassung** (`compile.rs:293-302`): §7 nennt für die
   Konstruktion „budget, settings admission, then source limits…“. Die Allowance
   (`bytes + JSON-Knoten` aus der Registry-Statistik `+ 428 + 202`) ist das Budget
   dieser Reihenfolge. Danach folgen die Zulassung und ab `compile.rs:304` die
   Quellgrenzen. Das ist **vertragskonform**; die Offenlegung ist korrekt, eine
   Abweichung liegt nicht vor. Die Drift wird nach dem Kompilieren gegen 428/202
   geprüft.
2. **Depth-Cap 32 über JSON unerreichbar:** Der geprüfte Parser begrenzt die
   JSON-Tiefe auf 32. Die kompilierte Tiefe ist daher höchstens 31: `chain(30)`
   kompiliert mit Tiefe 31, `chain(31)` wird als `JsonDepth` verweigert
   (`limits.rs:393-405`). Der `Depth`-Cap ist über die echte Zulassung mit
   abgesenkten Caps 12/13/14 getestet. Der Auswertungsstack hat 33 Frames und kann
   deshalb nicht überlaufen. **Akzeptiert.** Die wörtlichen §8-Grenzen 31/32/33 sind
   aus technischem Zwang nicht vollständig herstellbar; das ist offengelegt.
3. **Poison-Tests über ein privat beschädigtes Template** (`work.rs:234-280`):
   **Akzeptiert.** Biome-, Temperatur- und Noise-Fehler sind für zugelassene
   Eingaben unerreichbar. `evaluate` vergiftet bei jedem Fehler aus `run` über
   denselben Arm (`session.rs:295`). Rest siehe RR-H3.

## Befunde nach Schwere

**Blockierend:** keine. **Mittel:** keine.

**Hinweise (nicht blockierend):**

- **RR-H1 — Reihenfolge Seed gegen Settings ist nicht gemeinsam getestet.**
  - Ort: `session.rs:683-686`, Tests `policy.rs:189-194`.
  - Szenario: Vertauscht man die Prüfungen von Seed und Settings (Mutant R01),
    bleiben alle Kandidaten-Tests grün. Kein Test kombiniert eine fremde Seed mit
    fremden Settings.
  - Beleg: `mutants-results.json` R01; die Probe
    `review_seed_precedes_settings_when_both_mismatch` tötet den Mutanten. Der Code
    selbst ist korrekt (Probe grün).
  - Einordnung: A8 verlangt nur „one mismatching field at a time“, ein
    Vertragsverstoß liegt also nicht vor. Der Delivery-Satz „combinations show the
    order“ ist für dieses eine Paar aber zu weit gefasst.
  - Empfehlung: Den Probe-Test im nächsten World-Ticket übernehmen.
- **RR-H2 — Die Laziness des Biome-Memos ist nicht gepinnt.**
  - Ort: `session.rs:493-508`.
  - Szenario: Ein eager Resolver-Aufruf bei jedem zugelassenen `evaluate` (Mutant
    R06) bleibt in allen Kandidaten-Tests grün. Outcome, Trace und Replay ändern
    sich nicht, nur `work_used` (+1 pro Aufruf, innerhalb der Schranke).
  - Beleg: R06; die Probe `review_biome_memo_is_lazy…` tötet den Mutanten. Der Code
    ist korrekt: in 7021 Aufrufen wurde nicht, in 29 803 Aufrufen wurde der
    Resolver gerufen, jeweils genau passend.
  - Empfehlung: die Probe übernehmen.
- **RR-H3 — Poison-Pfade nur über die private Strukturkontrolle.**
  - Ort: `session.rs:227` (Noise-Fehler in `begin_column` vergiftet), `:295`.
  - Szenario: Die Vergiftung durch einen Noise-Fehler in `begin_column` sowie durch
    Biome-, Temperatur- und Noise-Fehler in `evaluate` wird nie ausgelöst. Der Test
    nutzt `MalformedDefinition` aus dem beschädigten Template. Alle diese Pfade sind
    für zugelassene Eingaben unerreichbar, und `evaluate` behandelt jeden Fehler im
    selben Arm. Kein Risiko für die Abnahme.
- **RR-H4 — Verbraucherliste nur direkt.**
  - Ort: Delivery, Tabelle „Final checks“.
  - Szenario: Die Freigabe verlangt die Tests „jedes World-Verbrauchers“. Laut
    `cargo metadata` hängen transitiv auch `ferrumc-plugin-block-rules` und
    `ferrumc-plugin-region-guard` von World ab. Der Ersteller hat sie nicht getestet.
  - Beleg: Ich habe sie nachgezogen (`checks/transitive-plugins.log`): exit 0,
    15 passed.
- **RR-H5 — Mutationslauf des Erstellers vor dem eingefrorenen Kandidaten.**
  - Ort: `results-round1.json` (00:56) und `results-round2.json` (01:01) gegenüber
    `candidate-01.source.json` (01:07).
  - Szenario: Die Mutanten liefen mit engen Testfiltern auf den Quellen vor dem
    Kandidaten.
  - Beleg: Alle 18 Anker kommen im Kandidaten genau einmal vor. M15 und M16 habe ich
    auf dem Kandidaten mit der vollen Suite erneut getötet (R14, R15). Die Zahl
    „18 Mutanten, alle getötet“ stimmt.

## Delivery-Abgleich

- **Zahlen bestätigt:**
  - World 360 + 3 ignoriert (304 + 2, 46 + 1, 10);
  - Verbraucher 86 Suites, 1426 + 1;
  - 18 Mutanten (16 in Runde 1, 2 in Runde 2; Ergebnisdateien `310b2c28…`,
    `6f2b9ed3…`);
  - Replay G 2490/4230/0, C 5478/24 626/2065; Trace-Zahlen 2455/7968/871/2776/4/2776;
  - 720 Arrays;
  - Clay 3393/3393/3393;
  - Bytes 6622 / 19 216 / 400;
  - Zensus 133 778 / 14 757 (58 + 56 × 259 + 192 + 3);
  - Chunk 45 908 736;
  - Allokationszahlen und Stack-Werte identisch.

  Nicht vergleichbar sind die Binär-Hashes der Allokationsläufe; sie hängen vom
  Pfad und vom Build ab und gelten laut Delivery nur für die genannten Binaries.
- **Hashes bestätigt:** Vertrag `1906f57e…`, Freigabe `69e40ca7…`,
  `source.freeze` `dd65ad0e…`, `evidence.freeze` `ec676844…`, Kandidat `3fe6f959…`,
  Vertragsfreeze `9f09cbd5…`, Fixture-Manifest-Digests, Clay `9ca939a4…` /
  `56c43547…` / `4c95743b…`, Guards `838ecefb…` / `ac03d213…`, Allocator-Probe
  `1e7a61bd…`.
- **Nicht-Behauptungen:** Es gibt keine TPS-, RSS-, Latenz- oder Vollserver-
  Aussagen, keine Aussagen zu Survival, Plugins oder Aktivierung, keinen
  Spaltentreiber und keinen Chunk-Write. `Enabled` wird verweigert, B1 bleibt
  offen, und der Steep-Zeitpunkt ist als quellbasiert benannt. Ich habe keine
  widersprechende Behauptung gefunden. Die Zeile „Session calls do not allocate“ in
  der README ist eine Entwurfseigenschaft (kein Heap) und durch die Messung
  gedeckt.

## Artefakte (`.codex-tmp/gen05_9-runtime-root-review/`)

| Datei | Inhalt | SHA-256 |
| --- | --- | --- |
| `final-assessment.json` | Verdikt, Zahlen, Hashes | — |
| `review/integrity-before.json` / `integrity-final.json` | identisch | `cbf72771…` |
| `checks/summary.txt` und Logs | frische Checks | — |
| `review/recount.json` | A1-Nachzählung und Abdeckung | `109eee32…` |
| `review/clay-compare.json` | A5 | `239f9e3a…` |
| `review/mutants-results.json` | 15 Mutanten | `b1e9299d…` |
| `review/forbidden-claims.json` | Scan | `75ca91e6…` |
| Skripte `review/*.py`, `review/checks.sh`; Probes `probe/…/review_probes.rs` | | Probes `f61c4c90…` |

## Root-Integrationsvermerk

Root hat bei der Integration am 12.10.2026 nur die Fundstellenzeile im
Abschnitt zum Forbidden-Claims-Scan umformuliert, weil der CI-Scanner das
zitierte englische Fundwort sonst als Aussage wertet. Die Hinweise RR-H1
(kombinierter Seed-/Settings-Test) und RR-H2 (Biome-Memo-Laziness festlegen)
gehen als Testauflagen an das SurfaceSystem-Columndriverticket. Der vom Reviewer
gelieferte Bericht hatte SHA-256
`3ff58263a9161008986a2df8e0afd44ac699a470801e9bc9dbf6b63564213e0d`.
