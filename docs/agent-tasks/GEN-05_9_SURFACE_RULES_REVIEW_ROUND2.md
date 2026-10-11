# GEN-05.9 — Root-Review Runde 2 des Surface-Rules-Vertrags

Unabhängiges Review, 11.10.2026. Der Reviewer hat weder den Kandidaten noch den
Review der Runde 1 erstellt. Geschrieben wurden nur diese Datei und
`.codex-tmp/gen05_9-root-review-r2/**`. Es gab kein Git, keinen Agenten und
keine Änderung an Brief, Evidenz, früherem Review oder Produktion. Gebaut wurde
nur in einer eigenen privaten Archivkopie mit eigenem Target-Verzeichnis.
Maschinenlesbar: `.codex-tmp/gen05_9-root-review-r2/final-assessment.json`.

## Verdikt: FINDINGS — B1 behoben, ein neuer blockierender, eng begrenzter Befund

Der blockierende Befund B1 aus Runde 1 ist **behoben**:

- Die Flat-Evidenz ist als ungültig markiert und bleibt erhalten.
- Sie ist aus Fixture, Erwartungen und Aussagen entfernt.
- `Enabled` wird typisiert als Policy verweigert.

Oracle, Guards und Fixture reproduzieren byte-genau. A2, A5 (8 von 9 Noises)
und H1–H11 stimmen an den Daten.

**Neu und blockierend ist R2-B1.** Die in Runde 2 eingeführte Replay-Regel (A1)
verlangt Originalgleichheit für 2.065 Fixture-Zeilen, die außerhalb der
P4-Domains liegen. Die Runtime muss genau diese Zeilen verweigern. Die Regel
verbietet zugleich, die Klassen zu lockern. Zwei gültige `full`-Zeilen hängen
außerdem am Cache-Zustand einer solchen verweigerten Zeile. Ein korrekter
Evaluator kann die Fixture also nicht bestehen.

Die Behebung ist mechanisch und klein (siehe R2-B1). Dazu kommt eine mittlere
Lücke in A3 (R2-M1). Nach diesen beiden Korrekturen ist ein CLEAN_CONTRACT ohne
weitere Vertragsarbeit erreichbar.

Geprüfter Kandidat (vor und nach dem Review identisch):

| Gegenstand | SHA-256 |
| --- | --- |
| Brief `GEN-05_9_SURFACE_RULES.md` | `496a3ddd7ecc2039158cf90d027dd02a5837fe8b1e486732be041f775cbae7d7` |
| `round2/delivery/freeze-manifest.json` (372 Einträge) | `b4adf851d5baac783b0dc7d906d67c5f8e515e663934c26745fe0bbd292ab148` |
| `round2/delivery/final-compare.json` | `0a15cdfdc0f15c03f23342986f9dc80a704de50e2b4f7f8592cd9eacdad472af` |
| `round2/delivery/round1-diff.json` | `a84e9eac580c3720cac7d6125d9c474703d8b3446501959751f7154bdc537422` |
| Fixture-`manifest.json` | `998e113a2eff5e2568c2f7954e7890f6e36c7918d871f5608a8d40a269459e48` |
| Workorder `GEN-05_9_SURFACE_RULES_TASK.md` | `fedaa21b…4405` (Maßstab, unverändert) |

## Status der Runde-1-Befunde

| Befund | Status | Begründung |
| --- | --- | --- |
| **B1** Flat-Kontext ohne `updateXZ` | **behoben** | §2.2, §11 P2, §12; Fixture ohne Flat-Spalte; `invalid_evidence` in Fixture- und Freeze-Manifest; Rohdaten byte-gleich erhalten |
| A1 Replay-Semantik | **teilweise** | Regel mechanisch, Zählung stimmt; Konflikt mit P4 (R2-B1) |
| A2 Paper-Baumform | **behoben** | `tree-paper.tsv` und `paper-tree-substitution.json` mechanisch gegen JAR-Daten bestätigt |
| A3 Budgets und Abrechnung | **teilweise** | Domains und `467 + m` korrekt; Verhalten von `begin_column` und `min_surface_level` bei zu kleinem Restbudget offen (R2-M1) |
| A4 Retained-Caps pinnen | Auflage, korrekt übernommen | §8, §13 |
| A5 Paper-Basis aller Noises | **behoben** (8/9, eigene Reproduktion) | `clay_bands_offset` offen als indirekt ausgewiesen; Aussagekraft siehe R2-H1 |
| A6 R-A1 | Auflage, übernommen | §9, §13 |
| A7 Ownership und Doku | **behoben** | Zielpfade, `lib.rs`-Freigabe, INVARIANTS-Abschnitt |
| A8 Unterscheidbare Verweigerungen, Steep-Kontrolle | **behoben** | `ResolverMismatch`, eigene Policy-Kinds, Kontrolltest |
| H1–H11 | **behoben** (alle elf) | Zahlen nachgezählt, siehe §8 |

## 1. Bytegleichheit (eigener Diff, ausgeführt)

- **Freeze:** Alle 224 Einträge des Runde-1-Freezes stehen mit unverändertem
  Hash im Runde-2-Freeze. Geändert oder entfernt wurde keiner, neu sind 148.
  Dazu gehören `oracle/SurfaceRulesOracle.java`, `run_oracle.py`,
  `check_guards.py`, `project_originals.py` und `oracle/input-pins.json`. Damit
  sind Helper, Runner und Pins unverändert.
  `round1-candidate/` ist byte-gleich mit dem Runde-1-Brief und dem Runde-1-Freeze.
- **Oracle:** `round2/oracle-normal` und `round2/oracle-optimized` sind in allen
  Dateien byte-gleich mit `final-normal` **und** `final-optimized`, einschließlich
  `manifest.json` (`15e4dbff…`). Abweichend sind nur die zeitgestempelten Logs
  und `diagnostics-manifest.json`.
- **Guards:** `round2/guards-{normal,optimized}/guards.json` sind byte-gleich mit
  Runde 1. Normal und `-O` unterscheiden sich nur in
  `entrypoint_python_optimize`.
- **Fixture:** Der Generator (`2285eefa…`, byte-gleich kopiert) wurde frisch
  ausgeführt, normal und mit `-O`. Beide Ausgaben sind byte-gleich mit der
  gelieferten Fixture (alle 14 Dateien).
  - `validate` normal und `-O`: identische Ausgabe, byte-gleich mit
    `fixture-validate-normal.json` (`39dead17…`): 38.891 Zeilen, H8-Invarianten,
    38.889 Replay-Zeilen, 3.529 Steep-Lesungen.
  - Eigener Diff gegen Runde 1 (`indep_r2.py`): 8 Dateien byte-gleich. Jede der
    38.892 Zeilen von `rows.tsv` ist gleich der Runde-1-Zeile ohne Feld 19
    (`flat`); 4.383 entfernte Flat-Zellen waren nicht leer. `manifest.json` hat
    sich geändert, 4 Dateien sind neu, keine wurde entfernt.
  - Die Behauptung aus der Workorder ist damit bestätigt.
- **Rückführung auf Rohdaten:** Eine eigene Expansion, unabhängig vom
  Generator-Code, führt alle 38.891 Fixture-Zeilen auf die rohen Paper- und
  Vanilla-Tokens zurück. Geprüft werden Position, Tiefen, Wasser, Ergebnis,
  Biom, Trace mit Sampler-Bits, Singletons, Steep, Min-Surface, Secondary,
  Spaltenzelle und Vanilla-Zelle. Keine Abweichung.
- **Keine Flat-Erwartung mehr:**
  - `rows.tsv` hat keine Flat-Spalte.
  - „flat“ kommt nur noch im Kindnamen `paper_optionally_flat_bedrock` von
    `tree-paper.tsv` bzw. der Substitution vor (Disabled-Form) und in der
    Ungültig-Markierung des Manifests.
  - Die Auswahlregel nutzt die ungültigen Zellen weiter als Zeilenpicker.
    2.390 C-Zeilen (`y ∈ [−63, 10]`) stehen *nur* deshalb in der Fixture (R2-H3).
    Ihre Erwartungen stammen aus dem echten Default-Kontext. Aus Flat-Daten
    entsteht also keine Erwartung.

## 2. B1

- **Brief.** Der Diff Runde 1 → 2 entfernt beide Belegaussagen: die
  Ergebniszeile (alt Z. 205) und „Both are backed by genuine evidence“ (alt
  Z. 637). Außerdem entfernt er die Enabled-Semantik aus der Bedingungstabelle
  (alt Z. 414). `Enabled` erscheint nur noch an diesen Stellen:
  - als Quellbefund, ausdrücklich „source review only … no valid differential
    evidence“ (Z. 91–96);
  - als ungültige Evidenz (§2.2, §12);
  - als Verweigerung (Z. 432, 519, 729, 860–863, 883).
- **Policy.** `UnsupportedFlatBedrock` ist eine eigene Fehlerart (Z. 586) und
  als P2 gekennzeichnet (Z. 631). Im Bind wird sie vor der Work-Prüfung geprüft
  (Z. 614). Das Enum bleibt erhalten (Z. 433, 729).
- **Daten.** `invalid_evidence.paper_flat_bedrock` im Freeze-Manifest nennt
  alle Fundorte: `rows.tsv` Spalte 27, `counts.tsv`, Manifest-Report,
  Runde-1-Fixture und Runde-1-Brief. Gesetzt sind
  `kept_unchanged_not_deleted: true` und die Markierung „ungültig, nicht
  maßgeblich (fabrizierter Kontext, Review B1)“. Das Fixture-Manifest trägt
  dieselbe Markierung. Die Rohzellen sind erhalten:
  - `counts.tsv` enthält weiter 13.608 und 2.481;
  - die Spalte `paper_flat_bedrock` ist vorhanden;
  - beides ist byte-gleich mit Runde 1.
- Die Mechanik-Beschreibung in §2.2 (Z. 203–221) stimmt mit
  `SurfaceRulesOracle.java:605-630` überein.

## 3. A1 — Replay-Regel

**Mechanik und Zählung (bestätigt).** Die Klassen habe ich auf einem dritten,
unabhängigen Weg nachgezählt:

- **G-Zeilen:** über den Aufrufzähler im Label `c<idx>/<n>/top<h>`. Laut Helper
  ist `n` die Nummer des Aufrufs seit `updateXZ` (`:558-562`).
- **C-Zeilen:** über Kontiguität von (Gruppe, `ctx_x`, `ctx_z`, `update_xz`).

Ergebnis:

- In G ist jedes aufgezeichnete Fenster ein lückenloser Präfix der Spalte. Die
  Spaltenzelle steht genau bei `n = 0`.
- In C steht sie genau am Run-Start.
- Zählung: G 2.490 `full` / 4.230 `partial`, C 5.486 / 26.683. Gesamt 38.889,
  0 Abweichungen zu `replay.tsv`.

**partial** vergleicht nur Ergebnis, Surface-Depth, Biom, das
`temperature`-Singleton und die `computed`-Einträge der LazyY-Arten. Die Liste
`biome, vertical_gradient, paper_optionally_flat_bedrock, y_above, water,
stone_depth` ist genau die LazyY-Menge aus `SurfaceRules.java`. Das ist korrekt
begründet:

- Die Per-Spalten-Werte sind rein.
- Der einzige zeitabhängige Wert ist der erste Steep-Test einer Spalte. Weil
  G-Fenster Präfixe sind, liegt er immer in den Rohdaten und ist dort gegen die
  deklarierte Sicht geprüft.

**Heights-View.**

- `steep-heights.tsv` habe ich aus der Helper-Formel neu berechnet: `base`
  ist `final_world_surface_wg[7,7]` (der Steep-Probe läuft nach `buildSurface`),
  dazu `3·((7lx+3lz) mod 5) + (lx+lz) mod 2`. Für alle 3 Gruppen ist das
  identisch mit der Datei.
- Alle 3.529 aufgezeichneten Steep-Lesungen (838 G, 2.107 C final, 584 C
  steep-probe) passen zur deklarierten Sicht. 3.529 ist auch die Zahl der
  Steep-Singletons.

**Konflikt mit P4.** Siehe R2-B1.

## 4. A2 — Paper-Baumform

- Die JAR-Daten `overworld`, `large_biomes` und `amplified` habe ich JSON-tief
  verglichen (Paper gegen Vanilla). Sie unterscheiden sich in allen drei Presets
  nur in `/surface_rule/sequence[0]/if_true/{type,is_roof}`. Das entspricht den
  gepinnten Resource-Patches.
- Eine eigene Preorder-Projektion (Elternteil, Tiefe, Art) des Paper-JAR-JSON
  ist gleich `tree-paper.tsv`. Die Projektion des Vanilla-JAR-JSON ist gleich
  `tree.tsv`. Beide haben 428 Knoten.
- `tree-paper.tsv` ist gleich den ersten fünf Spalten von
  `paper/tree-jar.tsv`. `tree.tsv` ist gleich `projection/tree.tsv`.
- Die beiden Bäume unterscheiden sich nur in Zeile 2. Das ist genau
  `paper-tree-substitution.json`.
- Census des Paper-Baums: `vertical_gradient` 1 und
  `paper_optionally_flat_bedrock` 1. Alles andere ist wie in der Brief-Tabelle.

## 5. A3 — Budget und Abrechnung

- **Nachgerechnet:**
  - pro Aufruf 287 + 141 + 33 + 1 + 1 + 2 + 1 + 1 = 467;
  - pro Chunk 256 × (2 + 1 + 384 × 467) = 256 × 179.331 = **45.908.736**;
  - das liegt innerhalb von `1..=67_108_864`.
- **Grenzfall „exakt reicht“:** Ich habe den schlechtesten Fall simuliert. Vor
  der letzten Spalte bleiben 179.331 Einheiten. Nach `begin_column` sind es
  179.329. Der erste Aufruf braucht 468 (m = 1). Danach bleiben genau
  383 × 467, und der letzte Aufruf trifft genau 467.
- **Konsistenz mit dem Original:**
  - Die Surface-Depth entsteht eager in `updateXZ`; das passt zu den 2 Einheiten
    von `begin_column`.
  - Das Minimum wird lazy und einmal pro Spalte gecacht
    (`lastMinSurfaceLevelUpdate`). Die Ecken sind Session-Eingabe. Das passt zu
    „1 Einheit, einmal pro Spalte“.
  - Secondary und die Noise-Knoten sind LazyXZ und im Per-Call-Worst-Case
    konservativ enthalten.
  - Die Tests 466/467/468 (gecacht) und 467/468/469 (ungecacht) sind richtig
    gesetzt.
- **Lücke:** siehe R2-M1.

## 6. A5 — Paper-Basis-Probe (eigene Reproduktion)

**Aufbau.**

- Eigene Kopie des Root-Archivs, geprüft gegen `base.freeze.json`
  (`c9a6fd4a…`).
- Zusätzliche Dateien: das Kandidatenmodul (byte-gleich, `cd7f06ec…`) und
  mein eigenes `review_clay_probe.rs`.
- In `lib.rs` sind nur die Registrierungszeilen angehängt. Der Runner
  verweigert jede andere Abweichung.
- Eigenes `CARGO_TARGET_DIR`, geseedet per Reflink aus dem Runde-1-Reviewer-Target,
  nicht aus dem Kandidaten-Target.
- `--jobs 2`, rustc 1.99.0.

**Ergebnis.**

- `debug.tsv` ist gleich `release.tsv` und gleich der Kandidatenausgabe
  (`round2/paper-basis-probe/final/debug.tsv`), Eingabe dabei
  `round2/oracle-normal`.
- Paper-Basis auf Paper-Zeilen ist bitgleich:
  - alle 33 Schwellenknoten, zusammen 35.958;
  - je Noise `surface` 23.104, `powder_snow` 5.927, `calcite` 2.425,
    `gravel` 1.904, `packed_ice` 1.430, `ice` 1.147, `surface_swamp` 21;
  - Spalten-Surface-Noise 26.572;
  - `surface_secondary` 1.182;
  - Draw 26.572;
  - Surface-Depth 26.572.
- Das sind 8 von 9 Noises plus Surface-Noise, Secondary, Draw und Depth.
  Bestätigt.
- **Gegenprobe echt.** Die Vanilla-Basis trifft auf Paper-Zeilen in **keiner**
  Noise-Zelle (0/…). Ebenso trifft die Paper-Basis auf Vanilla-Zeilen nie.
  Die Positivkontrolle (Vanilla auf Vanilla) ist vollständig gleich.
- Die einzige Depth-Abweichung ist die H1-Spalte.

**`clay_bands_offset`** ist im Brief offen als nur indirekt belegt ausgewiesen
(§2.4, §4, §12, A5). Meine Zusatzprobe zeigt, wie schwach diese indirekte
Evidenz ist:

- 60.846 Paper-Zeilen erreichen laut Trace einen `bandlands`-Knoten (70 oder
  99).
- Alle 60.846 erfüllen `bands[(y + round(4·clay) + 192) % 192] == Ergebnis`.
  Das gilt **in beiden Basen**.
- In allen 3.393 Badlands-Spalten ist `round(4·clay)` in Paper- und
  Vanilla-Basis identisch.

Die Bandlands-Vergleiche bestätigen also Bandtabelle und Formel, können die
Basis aber nicht unterscheiden (R2-H1).

Das cfg(test)-Modul existiert nur in den Probe-Kopien. Im Arbeitsbaum liegt
kein Probe-Modul, und `ferrumc-world/src/lib.rs` ist unverändert.
`server/crates/ferrumc-world` ist gleich dem Archiv, bis auf zwei
`__pycache__`-Verzeichnisse unter `tests/reference/`. Diese stammen vom
11.10. 01:48 bzw. 02:21, also aus früheren Tickets.

## 7. Policies P1 und P6

- **P1.**
  - `SurfaceNumericProfile::{PaperBuild60, Vanilla}` ist ein Parameter von
    `bind`.
  - `Vanilla` wird als eigene Fehlerart `UnsupportedNumericProfile` verweigert,
    zuerst im Bind und vor jedem Sampler.
  - Die Modul-Doku nennt das Profil.
  - Es gibt keinen Pfad, auf dem ein Vanilla-Wunsch still mit Paper-Numerik
    gerechnet wird.
- **P6.**
  - `InconsistentResolver(ResolverMismatch::{Seed, Settings, ZoomSemantics})`
    mit fester Reihenfolge Seed → Settings → Zoom.
  - Die Begründung als Konsistenzregel ist korrekt: 0 zoombedingte Unterschiede.
  - Umsetzbar ohne GEN-05.8-Änderung: `SurfaceBiomeResolver` bietet `identity()`
    (mit `seed()` und `resource()`) und `semantics()` (`stored_biome.rs:958-974`).
- **Kleine Lücke:** Quelle und Geometrie der Identity werden nicht geprüft
  (R2-H5).

## 8. Hinweise H1–H11 an den Daten

| Hinweis | Brief | Eigene Zählung |
| --- | --- | --- |
| H1 | 36 Zeilen, eine Spalte, Treiber Noise-Basis | 36 Zeilen, nur `sweep/jagged_peaks/1` (−1272, −8177). Vanilla-Noise mit Paper-Draw ergibt 2, Paper-Noise mit Vanilla-Draw ergibt 3. Stimmt |
| H2 | 80 `buildSurface` pro Modus | 80 Gruppen, je ein Aufruf. Stimmt |
| H3 | 180 geteilte Positionen | 180 in beiden Modi (`tree-bootstrap.tsv`). Stimmt |
| H4 | 474.301.331 Byte = 452 MiB | `final-normal` 474.301.331; `oracle-normal` 474.300.753; 29 Differentialdateien 473.872.928. Stimmt |
| H5 | 18.984 Spalten, 26.556 Zeilen | 18.984 gruppenbezogene Spalten mit Zelle in beiden Originalen; 26.556 gemeinsame Zeilen. Stimmt (Präzisierung R2-H4) |
| H6–H11 | übernommen | Text an den genannten Stellen vorhanden. H7: `dev2-probe/*` und `warm-debug.log` sind im Scope ausgenommen und nicht eingefroren. H8: `validate` prüft die Invarianten. H9: Schranke 10/3, also \|i\| ≤ 14; `128 + i ≥ 0`. Stimmt |

## 9. Regressionen

Der Brief-Diff Runde 1 → 2 ändert nichts an diesen Teilen:

- Regelbaum: Compile, Census, Tiefe, Kappen;
- Laziness: §2.1 Punkt 4 und die Tabelle in §6;
- Kontextdefinition (§5): nur das P6-Label ist neu;
- Steep-Sicht: Text nur ergänzt;
- Guards, Pins und Oracle: Hashes unverändert.

Neu sind der Abschnitt §2.4, die Replay-Regel, die Budget-Domains und
-Abrechnung, die Policy-Kinds und §13 bis §15. Eine Regression in tragfähigen
Teilen der Runde 1 gibt es nicht.

## 10. Ehrlichkeit

- **Keine Runtime-, TPS-, RSS- oder Aktivierungsaussage.** Die Stellen Z. 4,
  716 und 912–913 sind ausdrücklich Negativaussagen. „runtime“ steht sonst nur
  im Sinn des späteren Tickets.
- **Forbidden-Claims:** `markdown_blocks` + `scan` aus
  `scripts/check-forbidden-claims.py` (`eb2671f2…`, gleich im Arbeitsbaum und
  im Archiv), direkt auf den Brief, mit `force_plugin` false und true. Ergebnis:
  **0 Funde** (`forbidden-claims-r2.json`). Auch dieses Review-Dokument ergibt
  0 Funde.
- **Sachlich falsche Aussage:** R2-H2.

## Befunde nach Schwere

### Blockierend

**R2-B1 — Die Replay-Regel (A1) verlangt Originalgleichheit für Zeilen, die P4
verweigern muss.**

- **Ort:** Brief Z. 538–575 (Replay-Regel), 637–639 (P4-Evidenz), 920–924
  (A1: „must not add rows or relax the classes“). Außerdem `replay.tsv` und
  `surface_fixture.py:193-205`.
- **Mechanismus:**
  - `replay.tsv` klassifiziert alle 38.889 G/C-Zeilen als `full` oder
    `partial`. Beide Klassen verlangen mindestens Ergebnisgleichheit.
  - 2.065 dieser Zeilen liegen außerhalb der P4-Domains: alle in C, 6 davon
    `full` und 2.059 `partial`. Im Einzelnen:
    - 2.016 `end/`-Zeilen mit `y ∈ {i32::MIN, −65, 320, i32::MAX}`;
    - 43 `gradient/`-Zeilen mit `y < −64`;
    - 6 `extreme/`-Zeilen mit Stone-Depth `i32::MIN/MAX/0/−1`.
  - Für sie schreibt §7 vor, dass `evaluate` mit `BlockOutsideGenerationHeight`
    bzw. `InvalidStoneDepth` verweigert, und zwar vor jeder Mutation.
- **Folgefehler:** Der `full`-Lauf ab Ordinal 8721 (`extreme/…`, Gruppe
  `origin/normal/0`, Spalte (7, 7), y = 62) beginnt mit vier verweigerten Zeilen.
  - Zeile 8721 berechnet im Original als erste die Mindest-Oberfläche (`min`
    gesetzt) und füllt die Spalten-Caches.
  - Die gültigen `full`-Zeilen 8725 (Wasser `i32::MAX`) und 8726 (Wasser
    `MIN+1`) erwarten deshalb `min = -` und gecachte Noise-Knoten.
  - Eine korrekte Runtime hat 8721–8724 verweigert. Sie berechnet diese Werte
    erst in 8725, und die verlangte volle Trace-Gleichheit schlägt fehl.
  - Nachweis: `replay-policy-check.json`.
- **Szenario:** Die Runtime implementiert P4 korrekt und spielt die Fixture nach
  §6 ab. Dann fallen 2.065 Zeilen durch (Verweigerung statt Ergebnis), dazu 2
  gültige `full`-Zeilen. Die Runtime müsste die Klassen lockern, was §13 A1
  verbietet, oder die Policy aufweichen.
- **Verwandter Textfehler (R2-H2):** §7 Z. 638 nennt die Stone-Depth- und
  Wasser-Extreme „D rows“. Es sind C-Zeilen (`extreme/`) im Replay. D-Zeilen
  gibt es nur zwei (`paper-jar-tree-without-level`, `tryApply-before-update`).
  Das war schon in Runde 1 so.
- **Behebung:** mechanisch, ohne Oracle-Lauf.
  1. `replay.tsv` bekommt eine dritte Klasse, z. B. `refused`, mit der
     erwarteten Policy-Art. Sie gilt für jede Zeile außerhalb von y
     `[-64, 319]`, Stone `1..=4096` oder Wasser `≠ i32::MIN`. Erwartet wird die
     typisierte Verweigerung ohne Zustandsänderung. Die Originalwerte bleiben
     Evidenz.
  2. `full` gilt nur, wenn im Run-Präfix keine `refused`-Zeile einen
     Per-Spalten-Wert zuerst berechnet hat. Sonst wird die Zeile `partial`.
     Einfacher ist: `full` nur, wenn das ganze Präfix zulässig ist.
  3. `validate` leitet die Klasse auf dem zweiten Weg neu ab. Die Zählungen in
     §6 werden korrigiert, und Z. 638 heißt dann „C rows (`extreme/`)“.

### Mittel

**R2-M1 — A3: Das Verhalten von `begin_column` und `min_surface_level` bei zu
kleinem Restbudget ist nicht festgelegt.**

- **Ort:** Brief Z. 609–621 (Refusal-Reihenfolge) und 691–705 (Abrechnung).
- **Lücke:** `begin_column` kostet 2 Einheiten, `min_surface_level()` beim
  ersten Aufruf 1 Einheit. Die Refusal-Reihenfolge nennt für `begin_column`
  aber nur die Spalten-Domain. Für `min_surface_level` nennt sie gar nichts:
  weder `NoActiveColumn` noch `WorkBudgetExceeded`, auch keinen Rückgabetyp.
- **Szenario:** Die Session hat noch 1 Einheit übrig, und `begin_column` wird
  gerufen. Eine Implementierung kann dann verweigern, in den negativen Bereich
  laufen oder sättigen. Das Ticket müsste diese Abrechnungsregel selbst
  entscheiden, A3 verlangte aber eine eindeutige Regel.
- **Behebung:** Zwei Sätze in §7/§8:
  - `begin_column` verweigert `WorkBudgetExceeded` vor jeder Mutation, wenn
    der Rest unter 2 liegt. Das geschieht nach der Domain-Prüfung.
  - `min_surface_level()` → `Result`: zuerst `NoActiveColumn`, dann
    `WorkBudgetExceeded`, wenn das Minimum nicht gecacht ist und der Rest unter
    1 liegt.
  - Dazu Tests N-1/N.

### Hinweise

- **R2-H1** `clay_bands_offset`: Die indirekte Evidenz kann die Basis nicht
  unterscheiden. In 0 von 3.393 Badlands-Spalten weicht `round(4·clay)` zwischen
  Paper und Vanilla ab, und alle 60.846 Bandlands-Zeilen stimmen in beiden
  Basen. Der Satz in Z. 331–333 sollte das sagen. Die Paper-Basis dieses Noise
  ruht dann auf demselben `paper_build60_basis`-Pfad wie die acht bitgleich
  belegten. Das Risiko ist gering und ergebnisneutral überall, wo beobachtet
  wurde.
- **R2-H2** Z. 638 nennt die `extreme/`-Zeilen „D rows“. Es sind C-Zeilen
  (siehe R2-B1).
- **R2-H3** 2.390 C-Zeilen stehen nur über den ungültigen Flat-Picker in der
  Fixture. Das ist deklariert, und die Erwartungen sind echt. Kein
  Handlungsbedarf, nur zur Kenntnis.
- **R2-H4** 18.984 zählt Spalten pro Gruppe. Physisch, also nach (Preset, Seed,
  x, z), sind es 18.728, weil ein Chunk in zwei Gruppen liegt (256 Spalten
  doppelt). Für die Aussage reicht „distinct per group“.
- **R2-H5** P6/`InconsistentResolver` prüft Seed, Settings und Zoom, nicht aber
  `source()` und `geometry()` der `StoredBiomeIdentity`. Das Ticket sollte
  festhalten, ob die Geometrie gleich `min_y`/`height` der Settings sein muss,
  dann mit eigener Mismatch-Art, oder warum sie frei bleibt.

## Integrität

- **Freeze:** Alle 372 Einträge sind vor und nach dem Review neu gehasht:
  0 Abweichungen, 0 fehlend. `integrity-before.json` und `integrity-after.json`
  sind identisch.
- **Unverändert:**
  - Brief `496a3ddd…` und Freeze `b4adf851…`;
  - die Liste der nicht eingefrorenen, nicht ausgenommenen Dateien (36, nur
    `final-optimized` über Manifeste und das Freeze-Manifest selbst, wie in
    Runde 1);
  - kein `__pycache__` in `.codex-tmp/gen05_9`.
- **Ablage:** Alle Läufe und Builds liegen unter
  `.codex-tmp/gen05_9-root-review-r2/`:
  - `fixture/` (frische Generierung und Validierung);
  - `probe/` (eigene Archivkopie, eigenes Target, Runner `run_review_probe.py`);
  - die Skripte `integrity.py`, `indep_r2.py`, `replay_policy_check.py`,
    `forbidden_claims_r2.py` und `write_assessment.py`.
