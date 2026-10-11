# GEN-05.9 Surface Rules: Review Runde 3

Unabhängiges Review, 11.10.2026. Den Kandidaten und die früheren Reviews habe
ich nicht erstellt. Der Prüfumfang ist eng: Sind R2-B1 und R2-M1 behoben, und
entstehen durch die Änderungen der Runde 3 neue Fehler?

## Verdikt: CLEAN_CONTRACT

R2-B1 und R2-M1 sind behoben, ebenso die Hinweise R2-H1 bis R2-H5. Einen neuen
blockierenden oder mittleren Befund habe ich nicht gefunden. Drei nicht
blockierende Hinweise (R3-H1 bis R3-H3) betreffen Wortlaut und Testschärfe; sie
können im Runtime-Ticket mitgenommen werden.

Kernbelege:

- **Klassen:** Ich habe sie auf einem eigenen, vierten Weg neu abgeleitet. Er
  nutzt nur die rohen Paper-Zeilen, die behaltenen Ordinals aus `rows.tsv` und
  die P4-Bereiche aus §5. Ergebnis: G 2.490 / 4.230 / 0 und
  C 5.478 / 24.626 / 2.065 (`full` / `partial` / `refused`). In allen
  38.889 Zeilen stimmen `run_start`, Klasse und Refusal-Art mit `replay.tsv`
  überein (0 Abweichungen).
- **Bytegleichheit:** 12 von 14 Fixture-Dateien sind identisch.
  - `replay.tsv` hat nur die neue Spalte `refusal` und 2.067 Klassenwechsel.
  - Im Manifest haben sich genau die 6 deklarierten Schlüssel geändert.
  - Rohdaten und Guards sind unverändert.
  - Die Fixture ist frisch regeneriert: normal, `-O` und aus `oracle-optimized`
    bytegleich.
- **Clay-Probe:** In eigener Archivkopie mit eigenem Modul reproduziert:
  3.393 / 3.393 Spalten mit verschiedenen Bits, 0 / 3.393 mit verschiedenem
  Band-Offset; Debug = Release.
- **Integrität:** Alle 458 Freeze-Einträge waren vor und nach dem Review
  unverändert.

## Geprüfter Kandidat

| Artefakt | SHA-256 |
| --- | --- |
| Brief `docs/agent-tasks/GEN-05_9_SURFACE_RULES.md` | `1906f57e52d3b7745b1a2aae2322eaa36caa3658c9a00bd6e98e5cc592a20e6f` |
| Freeze `.codex-tmp/gen05_9/round3/delivery/freeze-manifest.json` (458 Einträge) | `9f09cbd5bd191f3e173bf1843ab45fc5b5b9411de350fe45ff6a4429a8fa0e11` |
| Work Order | `fedaa21b…` (unverändert) |
| Review Runde 1 / Runde 2 | `472bce61…` / `6a3ddd8c…` (unverändert) |
| Assessment Runde 2 | `16f74333…` (unverändert) |
| Runde-2-Kandidat `round2-candidate/` | Brief `496a3ddd…`, Freeze `b4adf851…` (unverändert) |

Der Brief-Diff Runde 2 → 3 (`brief-r2-r3.diff`, 582 Zeilen) berührt nur die in
§16 genannten Stellen: §2.2 (H4), §2.4/§4/§12/§13 A5 (Clay), §5 (Resolver-Zeile,
Steep nach Verweigerung), §6 (Replay-Regel, Zählungen), §7 (Mismatch-Arten,
Reihenfolge, P4-Evidenz), §8 (Abrechnung, Fortsetzung, Matrix), §9 (Signaturen),
§10 (Fixture, Bytestabilität), §11 (P4, P5, P6), §13 (A1, A3, A8), §14 und §16.

## Einstufung der Runde-2-Befunde

| Befund | Einstufung | Begründung (Kurzform) |
| --- | --- | --- |
| **R2-B1** Replay-Regel gegen P4 | **behoben** | dritte Klasse `refused` mit typisierter Art; `full` nur bei vollständig behaltenem und zulässigem Präfix; Klassen auf eigenem Weg exakt nachgezählt |
| R2-B1 Folgefehler (Run ab 8721) | **behoben** | 8725 und 8726 sind `partial`; kein zulässiges `full` folgt einer verweigerten Zeile (0 von 4.976 Fällen) |
| **R2-H2** §7 „D rows“ | **behoben** | §7 nennt nun C-Zeilen `end/`, `gradient/`, `extreme/`; Wasser-Extreme als zulässig und `partial` |
| **R2-M1** Restbudget `begin_column` / `min_surface_level` | **behoben** | Reihenfolge, Fehlerarten, Rückgabetypen, Wirkungslosigkeit und Fortsetzung vollständig und mit A3 konsistent; Wortlaut-Hinweis R3-H1 |
| **R2-H1** `clay_bands_offset` | **behoben** | ehrlich als „keine Basis-Evidenz“ korrigiert; Probe reproduziert; A5 als Pflichtauflage klar genug (Schärfung R3-H2) |
| **R2-H3** Flat-Picker-Zeilen | **behoben** | in §10 offengelegt, im Manifest `kept_only_by_invalid_flat_picker = 2390` |
| **R2-H4** Spaltenzählung | **behoben** | „distinct per group“, physisch 18.728 |
| **R2-H5** Resolver-Quelle und -Geometrie | **behoben** | `Source` (P6) und `Geometry` (Konsistenz) ergänzt, Reihenfolge wie `StoredBiomeIdentity`; Belege stimmen (Hinweis R3-H3) |

## 1. Bytegleichheit gegen Runde 2

Eigener Diff (`bytediff_r3.py` → `bytediff-r3.json`); die Delivery-Datei
`round2-diff.json` habe ich dafür nicht gelesen.

- **Fixture-Dateien:** 14 → 14, keine hinzugefügt oder entfernt.
  - Identisch sind `arrays`, `bands`, `biomes`, `columns`, `groups`,
    `paper-tree-substitution.json`, `patterns`, `rows`, `states`,
    `steep-heights`, `tree-paper` und `tree`.
  - Geändert sind nur `manifest.json` und `replay.tsv`.
- **`replay.tsv`:** 38.891 Zeilen in beiden Runden.
  - Der Header ist um `refusal` erweitert.
  - `ordinal`, `run_start` und `heights` sind in 0 Zeilen geändert.
  - Die Klasse hat sich in 2.067 Zeilen geändert, und zwar genau
    6 `full→refused`, 2.059 `partial→refused` und 2 `full→partial`
    (Ordinals 8725, 8726).
  - `refusal`-Werte: 36.824 `-`, 2.059 `BlockOutsideGenerationHeight`,
    6 `InvalidStoneDepth`. `refusal ≠ -` gilt genau dann, wenn die Klasse
    `refused` ist.
- **`manifest.json`:**
  - Geändert sind genau `counts`, `files`, `p4_refusal_order` (neu), `replay`,
    `round` und `scope`.
  - In `files` hat sich nur der Digest von `replay.tsv` geändert. Alle Digests
    passen zu den Dateien.
  - Unverändert sind `invalid_evidence`, `paper_tree`, `raw_sha256`,
    `selection`, `source_manifest_sha256` und `validate_invariants`.
  - In `counts` sind `kept_only_by_invalid_flat_picker`,
    `refused_by_tier_kind_case` und
    `replay.C/partial_complete_prefix_with_refused_row = 2` neu.
- **Oracle-Rohdaten:** Alle 72 `round2/oracle-*`-Einträge stimmen mit dem
  Runde-2-Freeze überein. Das Manifest ist `15e4dbff…`. Der Runde-3-Freeze
  enthält jeden Runde-2-Eintrag unverändert und 86 neue Einträge.
- **Guards:** Die Runde-3-`guards.json` (normal und `-O`) sind byteidentisch
  zu Runde 2: 16 Guards, die sich nur in `entrypoint_python_optimize`
  unterscheiden.
- **Frische Regeneration** in `.codex-tmp/gen05_9-root-review-r3/fixture/`, mit
  dem Generator `31d5e466…` (gleich dem eingefrorenen):
  - `generate` normal, `generate -O` und `generate` aus `round2/oracle-optimized`
    sind in allen 14 Dateien identisch mit der Kandidaten-Fixture, und die
    stdout-Logs gleichen denen des Kandidaten.
  - `validate` normal und `-O` geben dieselbe Ausgabe, byte-identisch zu
    `fixture-validate-{normal,optimized}.json`: 38.891 expandierte Zeilen
    gleich roh, H8-Invarianten, Klassen wie oben, 3.529 Steep-Reads
    konsistent.
  - stderr ist überall leer.

## 2. R2-B1: Replay-Regel und P4

### Eigene Nachzählung (`recount_r3.py` → `recount-r3.json`)

Dieser Weg ist unabhängig von Generator, `validate` (Weg 2) und `freeze.py`
(Weg 3):

1. **Runs** werden aus den rohen Paper-Zeilen gebildet: Ein Run beginnt bei
   gesetzter Spaltenzelle und läuft über aufeinanderfolgende Ordinals mit
   gleichem `(tier, group, update_xz, ctx_x, ctx_z)`.
2. **P4** wird aus den Rohwerten bestimmt: `ctx_y ∈ [-64, 319]`, Stone oben und
   unten `1..=4096`, Wasser `i32::MIN` = `None`. Die Reihenfolge folgt §7
   Schritt 6: y, oben, unten, Wasser.
3. **`full`** gilt, wenn alle Ordinals von `run_start` bis zur Zeile behalten
   und zulässig sind.

| Tier | `full` | `partial` | `refused` |
| --- | --- | --- | --- |
| G | 2.490 | 4.230 | 0 |
| C | 5.478 | 24.626 | 2.065 |

- **Refusal-Art je Zeile:** `end/` 2.016 und `gradient/` 43 sind
  `BlockOutsideGenerationHeight`, `extreme/` 6 ist `InvalidStoneDepth`.
- **Abgleich mit `replay.tsv`:** 0 Abweichungen in `run_start`, Klasse oder
  Art bei 38.889 Zeilen.
- **Manifest:** Die Zählungen sind identisch.
- **Rohzeilen außerhalb der Domains (alle Tiers):** Es sind 3.030 C-Zeilen
  (`end/` 2.016, `gradient/` 1.008, `extreme/` 6), alle mit Exception `none`.
  - Die `y`-Werte sind `end/` {i32::MIN, −65, 320, i32::MAX} und
    `gradient/` {−65, −66}.
  - Keine G- oder D-Zeile liegt außerhalb.
  - Das bestätigt die korrigierte Evidenzliste in §7.

### Präfix-Regel und der Run ab 8721

| Ordinal | Stone (oben, unten) | Wasser | Runde 2 | Runde 3 |
| --- | --- | --- | --- | --- |
| 8721 | (i32::MAX, 1) | None | full | refused (`InvalidStoneDepth`); Original berechnet hier `min_surface` |
| 8722 | (i32::MIN, 1) | None | full | refused |
| 8723 | (1, i32::MAX) | None | full | refused |
| 8724 | (1, i32::MIN) | None | full | refused |
| 8725 | (1, 1) | i32::MAX | full | **partial** |
| 8726 | (1, 1) | i32::MIN+1 | full | **partial** |
| 8727 | (0, 0) | None | full | refused |
| 8728 | (−1, −1) | None | full | refused |

Das stimmt mit §6 überein („begins with four refused stone-depth rows“). Über
alle Runs gilt: Alle 4.976 behaltenen zulässigen Zeilen, deren Run-Präfix eine
verweigerte Rohzeile enthält, sind `partial`; keine einzige ist `full`.

### Weitere Zeilen, die eine korrekte Runtime nicht reproduzieren könnte

- **Cache-Füllung durch verweigerte Zeilen:** Nur zwei verweigerte Rohzeilen
  berechnen im Original einen Per-Spalten-Wert zuerst.
  - Ordinal 8721 berechnet `min_surface`.
  - Ordinal 335480 (`end/15,0/320`) berechnet Knoten 145.
  - Alle folgenden Zeilen ihrer Runs sind `partial`, und dort sind
    Per-Spalten-Werte nur Evidenz.
  - Die `partial`-Teilmenge (Ergebnis, Tiefe, Biom, Temperatur, Per-Block-
    Knoten) hängt nur von reinen Eingaben ab und ist reproduzierbar.
- **Voraussetzungen für „evaluate allein nach `begin_column`“:** Bei allen
  38.889 Zeilen gilt Folgendes.
  - Die Blockposition ist gleich der Kontextposition.
  - Die Spalte liegt im Zentrum-Chunk der Gruppe, sodass `begin_column` nicht
    vorher `ColumnOutsideCentre` liefert.
  - Die Oberflächentiefe ist pro Run und pro `(Gruppe, x, z)` konstant.
- **Steep-Sicht, erweitert auf alle Rohzeilen** (`steep_all_raw.py`): Alle
  3.529 aufgezeichneten Steep-Reads stimmen mit der deklarierten Sicht überein,
  auch die 2.654 aus *nicht* behaltenen Zeilen. Eine `partial`-Zeile, deren
  Steep-Ergebnis im Original eine nicht behaltene Zeile gecacht hat, ist unter
  der festen Sicht also reproduzierbar. `validate` prüft nur die behaltenen
  Zeilen; die Lücke ist damit geschlossen.
- **P3, P1, P2, P5, P6:**
  - P3: Alle 80 Gruppen nutzen zugelassene Settings (normal 78,
    large_biomes 1, amplified 1).
  - P1: Die Erwartungen sind Paper-Werte.
  - P2: Es gibt keine Flat-Erwartung, die Flat-Zellen sind nicht projiziert.
  - P6: Die Gruppen nutzen die Overworld-Multinoise-Quelle mit Geometrie
    `(-64, 384)` in 80/80 Gruppen.
  - P5: Der längste rohe Run hat 275 Zeilen, also höchstens
    2 + 1 + 275 × 467 Einheiten, weit unter jeder zulässigen Session-Allowance.
    Ein Replay pro Run in eigener Session kann nicht an A3 scheitern.
- **Verweigerungen und A3:** P4 steht vor der Work-Prüfung und kostet nichts.
  Eine `refused`-Zeile liefert auch bei Rest 0 die P4-Art, wie die Matrixzeile
  „input outside a P4 domain | 0“ verlangt.

**Wasser:** Die Aussage, Wasser könne aus der Fixture nicht verweigert werden,
ist korrekt. Das Original kodiert `None` als `i32::MIN`. Im Fixture-Token
wird daraus `-`, und `Some(i32::MIN)` ist nicht darstellbar. Von den Zeilen
haben 16.370 `Some`, 22.519 `None`.

## 3. R2-M1: Restbudget und Fortsetzung

**Reihenfolge und Fehlerarten (§7 Schritte 3–7, §8):**

- `begin_column`: `Poisoned` → `ColumnOutsideCentre` → `WorkBudgetExceeded`
  (Rest < 2).
- `evaluate`: `Poisoned` → `NoActiveColumn` → y → Stone oben → Stone unten →
  Wasser → `467 + m`.
- `min_surface_level`: `Poisoned` → `NoActiveColumn` → `WorkBudgetExceeded`
  (nur ungecacht, Rest < 1; Minimum bleibt ungecacht).
- `surface_depth`: `Poisoned` → `NoActiveColumn`.
- Alle Verweigerungen erfolgen vor jeder Mutation.
- Die Rückgabetypen sind in §8 und §9 gleich.

**Konsistenz mit A3:** Die Obergrenze 256 × (2 + 1 + 384 × 467) = 45.908.736
habe ich nachgerechnet.

- Pro Spalte genügen `begin_column` mit 2 Einheiten, höchstens 384 zulässige
  `evaluate` mit je höchstens 467 und das Minimum einmal mit 1 Einheit.
- Die Vorprüfungen (≥ 2, ≥ 467 + m, ≥ 1) werden bei dieser Allowance nie
  verletzt, weil die tatsächlichen Belastungen den Worst Case nicht
  überschreiten.
- Ein Aufruf von `min_surface_level` aus dem Frozen-Ocean-Treiber ersetzt das
  `m` des `evaluate`; die Summe bleibt gleich. Im Original ruft
  `frozenOceanExtension` `getMinSurfaceLevel()` erst nach der Blockschleife
  der Spalte auf, das ändert also keine `evaluate`-Trace.
- Die Grenzmatrix deckt jede Belastung mit N−1/N/N+1 ab.

**„Poisoned zuerst“** passt zu den bestehenden Regeln: Ein defensiver Fehler
(Biome, Temperatur, Noise, negativer Band-Index) vergiftet die Session.
Vergiftet ist terminal, und `WorkBudgetExceeded` vergiftet nicht. Die
Klarstellung macht nur explizit, was vorher implizit war. Siehe aber R3-H1
zu `work_used`.

**Fortsetzungsregel gegen die Original-Laziness:** Sie ist korrekt.

- Nach `WorkBudgetExceeded` kann die Spalte in derselben Session nie fertig
  werden. Die Allowance wächst nicht, und jede weitere `evaluate` braucht
  mindestens 467. Auch der Umweg über `min_surface_level` ändert das nicht:
  Rest r < 468 ergibt nach dem Caching r − 1 < 467.
- Der Neustart in einer neuen Session ab `begin_column` und dem ersten Block,
  vom selben Höhenzustand aus, erzeugt dieselbe Lazy-Reihenfolge, dieselben
  Per-Spalten-Caches und denselben ersten Steep-Test. Damit sind auch
  Ergebnis, Trace und `work_used` gleich.
- Das Verbot halber Caches und gemischter Ergebnisse schließt genau die Fälle
  aus, in denen der Zeitpunkt des Steep-Tests oder die Belastung des Minimums
  abweichen könnte.
- Spalten übergreifender Zustand existiert nicht. Die Session hält nur
  unveränderliche Ecken, Resolver und Temperaturen.

## 4. R2-H1: `clay_bands_offset`

Eigene Archivkopie unter `.codex-tmp/gen05_9-root-review-r3/probe/`:

- Basis `c9a6fd4a…`, eigenes Target (Reflink-Seed aus Runde 2), `--jobs 2`.
- Die Kopie weicht nur in zwei `cfg(test)`-Modulen und deren
  Registrierungszeilen in `lib.rs` ab.
- Das Kandidatenmodul `surface_clay_basis_probe.rs` (`a8eba7b7…`) ist gegen den
  Freeze byte-geprüft.
- Das zweite ist ein eigenes Modul, `review_r3_clay_probe.rs`.

| Größe | Kandidat (`final/debug.tsv`) | Kandidatenmodul, eigener Lauf | Eigenes Modul |
| --- | --- | --- | --- |
| Badlands-Spalten (Gruppe, x, z) | 3.393 | 3.393 | 3.393 (physisch ebenfalls 3.393) |
| Spalten mit verschiedenen f64-Bits | 3.393 | 3.393 | 3.393 |
| Spalten mit verschiedenem `round(4·clay)` | 0 | 0 | 0 |
| max. Wertdifferenz | 7,564e-6 | 7,564e-6 | 7,564e-6 |
| min. Rundungsabstand von `4·v` | 1,063e-4 (Paper) | 1,063e-4 | 1,063e-4 (Paper), 1,125e-4 (Vanilla) |

- Debug = Release in beiden Modulen.
- Das eigene Modul hat Bit-Digests über alle Spalten: Paper `f74ef398…`,
  Vanilla `cdbeecb4…`.
- `4·Δ ≤ 3,03e-5` ist kleiner als der kleinste Rundungsabstand. Ein Kippen
  des Offsets ist also ausgeschlossen.

**Ehrlichkeit:** Die Schlussfolgerung in §2.4, §4, §12 und §13 A5 ist korrekt
und ehrlich.

- Die Bandlands-Ergebnisse prüfen Bandtabelle und Indexformel, aber nicht die
  Basis.
- Ein direkter Bitvergleich würde in jeder Spalte unterscheiden.
- Es gibt keine echten Clay-Werte im eingefrorenen Oracle.
- Die Paper-Basis ruht für dieses Noise nur auf dem Pfad, der für die anderen
  acht bitgleich belegt ist.

**A5 als Pflichtauflage:** Für ein Runtime-Ticket klar genug. Benannt sind:

- die Zielgröße `SurfaceSystem.clayBandsOffsetNoise.getValue(x, 0, z)`;
- die Kontrolle durch die Vanilla-Basis;
- der Umfang „mindestens 3.393 Spalten, beide Originale“;
- das Artefakt: mechanische Fixture-Projektion aus einer von Root
  freizugebenden, gepinnten Helper-Erweiterung;
- die Konsequenz: ohne diesen Vergleich ist A5 nicht erfüllt.

Schärfung siehe R3-H2.

## 5. R2-H5: Resolver-Quelle und -Geometrie

- **Reihenfolge:**
  - Der Brief prüft Seed, Settings, Source, Geometry, ZoomSemantics, dann das
    Zentrum.
  - Die GEN-05.8-Felder von `StoredBiomeIdentity` sind `seed`, `resource`,
    `source`, `geometry` (`stored_biome.rs:81-86`), danach kommt der
    Zoom-Getter `SurfaceBiomeResolver::semantics()`
    (`stored_biome.rs:960, 972`).
  - Die Reihenfolge stimmt.
- **Bedingungen:**
  - `BiomeSourceIdentity` hat die Varianten `MultiNoise`, `Fixed` und `End`
    (`biome_selection/arrays.rs:65-72`).
  - `BiomeQuartGeometry` stellt `min_y()` und `height()` bereit
    (`arrays.rs:36-41`).
  - Die Bedingungen `MultiNoise(minecraft:overworld)` und `(-64, 384)` sind
    damit formulierbar und testbar.
- **Belege:**
  - `SurfaceRulesOracle.java:447` ist `Height.create(-64, 384)` in
    `protoChunk`.
  - `:496-503` holt den Generator über Weltpreset → `LevelStem overworld`.
  - In `groups.tsv` zeigen 80 / 80 Gruppen `min_gen_y = -64` und
    `gen_depth = 384`.
  - Die Presets sind normal 78, large_biomes 1, amplified 1.
- **Einordnung:**
  - `Source` als Policy P6 ist richtig: Das Original würde die Regeln auch
    über andere Quellen auswerten.
  - `Geometry` als Konsistenzprüfung ist vertretbar. Die Session löst ihre
    Anker fest mit der Level-Höhe `(-64, 384)` auf (§4), und ein Resolver
    anderer Geometrie widerspräche dieser Annahme. Siehe R3-H3.

## 6. Regressionen und Ehrlichkeit

- **Unverändert gegenüber Runde 2** (laut Diff keine Änderung): Regelbaum und
  Bedingungstabelle (§6), Laziness- und Cache-Spalten, Kontexttabelle bis auf
  die Resolver-Zeile, Steep-Sicht (nur ein ergänzender Satz zum ersten
  *zulässigen* Steep-Test), Guards, Pins (`input-pins.json` `fdfb8b24…`),
  Kapazitäten, Zensus und frühere Fixes (B1-Flat-Kennzeichnung,
  `invalid_evidence` unverändert).
- **Keine Aussagen zu Runtime-Leistung, TPS oder RSS.** Die einzigen
  Fundstellen sind die Verneinungen in Z. 875 und Z. 1132–1133.
- **Forbidden-Claims-Scan:** `markdown_blocks` + `scan` aus
  `scripts/check-forbidden-claims.py` (Worktree und Archiv, beide
  `eb2671f2…`), `force_plugin` false und true. Er lief direkt auf Brief und
  Work Order und lieferte 0 Funde.
- **Freeze:**
  - Alle 458 Einträge waren vor und nach dem Review neu gehasht: 0
    Abweichungen, 0 fehlend.
  - Brief, Work Order, beide Reviews und das Runde-2-Assessment sind
    unverändert.
  - Die Liste der nicht eingefrorenen, nicht ausgenommenen Dateien ist gleich
    geblieben: 36, wie in Runde 2 nur `final-optimized` und das
    Freeze-Manifest selbst.
  - Es gibt kein `__pycache__`.
  - `integrity-before.json` und `integrity-after.json` sind identisch.

## Befunde

### Blockierend

Keine.

### Mittel

Keine.

### Hinweise (nicht blockierend, für das Runtime-Ticket)

- **R3-H1 — `Poisoned` und `work_used`:** §7 Schritt 4 (Z. 701) und die
  Matrixzeile Z. 863 sagen „every / any session method“ liefert auf einer
  vergifteten Session `Poisoned`. §8 Z. 817 macht
  `work_used(&self) -> u32` aber unfehlbar. Gemeint ist offensichtlich „jede
  fehlbare Session-Methode“ (`begin_column`, `evaluate`, `min_surface_level`,
  `surface_depth`). Das Ticket sollte es so formulieren, und `work_used` bleibt
  auch vergiftet lesbar.
- **R3-H2 — A5 mechanisch schärfen:** Das Ticket sollte zwei Dinge
  ausschreiben.
  - Die Spaltenmenge, wie die Probe sie bildet: distinkte
    `(group, ctx_x, ctx_z)` der Paper-G/C-Zeilen mit `biome_direct` in
    {badlands, eroded_badlands, wooded_badlands}.
  - Das Bestehenskriterium:
    - echte Paper-Bits gleich der Runtime-Paper-Basis in jeder Spalte;
    - echte Vanilla-Bits gleich der Vanilla-Basis als Kontrolle;
    - jede Abweichung heißt, dass A5 nicht erfüllt ist.
- **R3-H3 — Resolver-Tests:** Neben `Fixed` und `End` sollte ein `Source`-Test
  `MultiNoise(minecraft:nether)` verwenden. Das ist die naheliegendste
  Verwechslung: gleiche Variante, andere Parameterliste. Außerdem könnte §11
  sagen, dass die Level-Höhe `(-64, 384)` eine Annahme der drei zugelassenen
  Settings ist, auf der die `Geometry`-Konsistenzprüfung beruht.

## Ablage und Reproduktion

Alle Läufe dieses Reviews liegen unter `.codex-tmp/gen05_9-root-review-r3/`:

- `integrity.py` → `integrity-before.json`, `integrity-after.json`;
- `bytediff_r3.py` → `bytediff-r3.json`;
- `recount_r3.py` → `recount-r3.json`, die eigene Klassenableitung mit
  Run-8721-Detail;
- `steep_all_raw.py` → `steep-all-raw.json`;
- `fixture/` mit `run.sh`, drei frischen Generierungen und zwei Validierungen;
- `probe/` mit `run_review_probe.py`, `review_r3_clay_probe.rs`,
  `final/receipt.json`, eigener Archivkopie und eigenem Target;
- `forbidden_claims_r3.py` → `forbidden-claims-r3.json`;
- `brief-r2-r3.diff`;
- `write_assessment.py` → `final-assessment.json`.
