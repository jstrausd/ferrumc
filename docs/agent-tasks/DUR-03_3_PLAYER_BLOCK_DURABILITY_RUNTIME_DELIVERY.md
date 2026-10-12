# DUR-03.3 — Runtime-Lieferung Storage (Format7)

> **Runde 6 (12.10.2026) ist der aktuelle Stand**, siehe Abschnitt „Runde 6“.
> - Block-Entities werden jetzt mit ihrer echten Kodierlänge abgerechnet statt
>   pauschal mit 16 KiB.
> - Die Restfall-Darstellung der Runde 5 ist korrigiert (Korrekturvermerke dort).
> - Der folgende Absatz beschreibt den Stand von Runde 5.

Stand: 11.10.2026, **Runde 5** (Nachbesserung zur Byte-Grenze nach Root-Entscheidung:
Receipt-Dedup pro Aufruf, keine Abrechnung nicht stattfindender Kopien, Restfall
offengelegt). **Lieferung des Implementierungsagenten zur Root-Prüfung; keine
Root-Abnahme.** Git, Integration und der unabhängige Review liegen beim Root.
Der Abschnitt „Runde 5“ ist der aktuelle Stand. „Runde 4“ (Runtime-Review
`DUR-03_3_PLAYER_BLOCK_DURABILITY_RUNTIME_REVIEW.md`, Verdikt FINDINGS, A1/A2) und die
Runde-3-Abschnitte gelten fort, soweit Runde 5 sie nicht ersetzt (Kandidat, finale
Checks, offene Byte-Frage, Grenzen).

## Runde 6

Stand: 12.10.2026. Nachbesserung zu B1 nach Root-Entscheidung (echte Entity-Länge
statt 16-KiB-Pauschale) und zu den Hinweisen R2-H1 bis R2-H3 aus dem unabhängigen
Review Runde 2. **Lieferung des Implementierungsagenten zur Root-Prüfung; keine
Root-Abnahme.** Git, Integration und Nachprüfung liegen beim Root.

### Grundlagen und Prüfbasis

| Grundlage | SHA-256 |
| --- | --- |
| Vertrag `DUR-03_3_PLAYER_BLOCK_DURABILITY.md` (maßgeblich Z. 248–272) | `2edfc588fd8d7e96753f6beb4bb7c2ad50a064e4c266681ab5ff94550c4342ab` |
| Review Runde 2 `DUR-03_3_PLAYER_BLOCK_DURABILITY_RUNTIME_REVIEW_ROUND2.md` | `cba8b4af7fe55f3b54dda54921cff3002cb03a87d69d942b203e02854d035f47` |
| `.codex-tmp/dur03_3-runtime-root-review-r2/final-assessment.json` | `4590e10f4f6fcb1da5ce9903ac66176f2059dec684236e3743db5f067ea863a1` |
| Delivery vor dieser Ergänzung (Stand Runde 5) | `f23cff1c097207745847b17958ccf80aa9155ad8c58de8cfb0bace539a1d58ff` |
| Ausgangskandidat Runde 5 (Manifest, 90 Dateien; Dateihash `493ebb50…1a4c`) | `1d8455ef0b8740068c54cf28e96947f8f82c8d8ff0b6c079e6da0d768a75f678` |

- **Ausgangslage:** Der Arbeitsbaum `server/crates/ferrumc-storage/**` war zu Beginn
  byte-identisch mit dem Runde-5-Manifest (`round6-tools.py check-start`, 90/90).
- **Prüfbasis:** main `8da4c770222d7d40b747ce2abb7a08ff9706fa25` als private Kopie per
  `git archive` in `.codex-tmp/dur03_3-runtime/round6-isolated/`, 1313 Dateien.
  Basis-Freeze `round6-base.freeze.json`, Digest
  `066b00846f527b554374e7a8166ff463093306f75a03583a8c7882b9cf57c3c0`.
  - Storage, `Cargo.toml` und `Cargo.lock` sind dort identisch zu `ce39d817`; nur
    `Cargo.lock` hat zwei Zeilen eines anderen Pakets mehr.
  - main stand bei Arbeitsbeginn bereits auf `0c0fdb4d`. Zwischen `8da4c770` und
    `0c0fdb4d` sind Storage, `Cargo.toml` und `Cargo.lock` unverändert (`git diff`
    leer, nur lesend). Wie vorgegeben gilt `8da4c770`.
- **Arbeitsweise:** Entwickelt wurde direkt in der isolierten Kopie, mit eigenem
  Dev-Target (`round6-dev-target`, per Reflink aus Runde 5 vorbelegt). Der
  gemeinsame Arbeitsbaum erhielt nur den finalen Stand (`round6-tools.py sync`);
  gebaut wurde dort nie.
- **Toolchain:** `rustc 1.99.0 (b940084d7 2026-09-28)`, `cargo 1.99.0`, `--jobs 2
  --locked`. Kein Git außer `git archive`/`git diff` (nur lesend), keine Agents,
  kein anderes Crate.
- **Dependency:** `ferrumc-items` ist von `[dev-dependencies]` nach `[dependencies]`
  gewandert, mit Begründung in `Cargo.toml`. Das ist Root-Entscheidung im Dialog;
  das Crate war schon Workspace- und Lock-Mitglied (`ferrumc-world` hängt davon ab).
  `Cargo.lock` ist byte-identisch (`183450e7…fe4a`), `--locked` grün. Keine neue
  externe Dependency.

### Kandidat und Freeze

Vor den finalen Checks eingefroren (2026-10-12T12:05:47+02:00, `round6-candidate-frozen-at.txt`):
`ferrumc-storage`, **91 Dateien**, Manifest-SHA-256
`3c53838631108f43b76c689ebfd89dc2e3104901005ef1a9ae454d93039c6664`.

- **Manifest:** `round6-candidate-manifest.json`, Dateihash
  `d21db90ec56958c965c2a8fb288d21b82ad5af22c8bda9cd0d471cd894abd0d7`, Schema
  `pfad\tbytes\tsha256\n`.
- **Identität:** Isolierte Kopie = Arbeitsbaum, geprüft mit
  `worktree_identical: true`. Schreibgeschützte Kopie: `round6-candidate-src/`.
- **Gegenüber Runde 5 geändert:** `Cargo.toml`, `INVARIANTS.md`, `README.md`,
  `src/block_durability/engine.rs`, `src/block_durability/raw.rs`,
  `src/block_durability/tests.rs`, `src/redb_store/block_validation.rs`,
  `src/redb_store/block_validation/tests.rs`, `src/redb_store/blocks.rs`,
  `tests/player_block_bytes.rs`.
- **Neu:** `tests/player_block_entity_bytes.rs`.
- **Unverändert:** Writer, Grammatik der persistierten Formate, Fehlerreihenfolge
  und Zellmodell.

### B1 — echte Entity-Länge statt Pauschale

**Mechanik (`src/block_durability/engine.rs`):**

- **Längenfunktion:** `entity_len` (`engine.rs:192`) bestimmt die Länge der
  kanonischen World-Kodierung *vor* dem Kodieren, geliehen und ohne Allokation:
  - Schild: `0x7f ++ schema ++ tag`, Art, Wachs, zwei Seiten aus
    `len(u16) ++ Farbe`, Glühen und vier `len(u16) ++ Zeile`.
  - Truhe: `0x7f ++ schema ++ tag`, dann 27 × `len(u32) ++ Item`. Die Item-Länge
    liefert `ferrumc_items::persisted_stack_size`, die nicht allokierende Preflight
    des Item-Codecs, laut Doku exakt; der Encoder ruft intern dieselbe Preflight auf.
  - Das Ergebnis ist auf die 16-KiB-Formgrenze gekappt. Der Encoder prüft diese
    Grenze vor jedem Anhängen, erzeugt also nie mehr.
- **Abrechnung vor jeder Kodierung:**
  - `entity_bytes` (Guards, ersetzte Effekt-Entity), `engine.rs:211`;
  - `entities` (Entity-Mengen vorher/nachher im Spaltenbeweis), `engine.rs:223`;
  - `view_encoded_len` (View), `engine.rs:737`.
- **Formgrenze bleibt hart:** Grammatik (`raw.rs`, `blob(16 * 1024)`),
  `canonical_entity` und der World-Codec verweigern unverändert jede Entity über
  16 KiB.

**Stellen mit Obergrenze statt echter Länge (vorab reserviert, nicht erstattet):**

| Stelle | Begründung |
| --- | --- |
| `engine.rs:192–205` (`entity_len`), Arm `_ =>` | `BlockEntity` ist `#[non_exhaustive]`. Für eine künftige, Storage unbekannte Entity-Art gibt es keine geliehene Längenrechnung, also werden 16 KiB reserviert. Heute unerreichbar (nur Schild und Truhe). |
| `engine.rs:192–205`, Truhe mit Item, das `persisted_stack_size` verweigert | Der Encoder scheitert an derselben Preflight; 16 KiB begrenzen seine Arbeit bis dahin. |
| verweigerte Kodierung allgemein (z. B. Schildzeile > 1024 Bytes, Truhe > 16 KiB) | Abgerechnet wird die rechnerische Länge, gekappt auf 16 KiB. Der Encoder schreibt vor dem Abbruch höchstens so viel. Das ist eine Obergrenze, keine Erstattung. |

**Doppelabrechnung geprüft:**

- Ein Gameplay-Spaltenbeweis kodiert zwei *verschiedene* Chunks vollständig:
  `entities(&before)` ist das aufgelöste Vorbild plus die angewandte Effektkette,
  `entities(&after)` das dekodierte gespeicherte Nachbild. Beide werden
  verglichen. Das sind zwei echte Kodierungen, jede wird einmal mit ihrer eigenen
  Länge abgerechnet.
- Die View kodiert ein drittes Mal (`view_encoded_len` misst und prüft die
  Darstellbarkeit). Auch das ist echte Arbeit und wird einmal abgerechnet.
- Eine Zahl wird nirgends zweimal für denselben Arbeitsschritt gebucht.

**Weitere echte, bisher nicht oder zu niedrig abgerechnete Arbeit (eigenständig,
bitte im Review prüfen):**

1. **Apply-Decode:** Beim Anwenden eines Effekts wird die neue Entity ein weiteres
   Mal dekodiert (`engine.rs:346`). Das war nicht abgerechnet und kostet jetzt ihre
   Länge.
2. **Kanonik-Prüfung:** `effect_entities` (`engine.rs:293`) rechnete pauschal
   2 × Eingabelänge ab. Bei einer nicht kanonischen Eingabe (z. B. Legacy-Truhe
   mit 28 Bytes) erzeugt die Neukodierung mehr, hier 301 Bytes.
   - Neu über `raw::canonical_entity_charged` (`raw.rs:29`): Der Decode wird mit
     der Eingabelänge abgerechnet, die Neukodierung mit ihrer exakten Länge
     `entity_len` vor dem Kodieren.
   - Für kanonische Eingaben ist beides gleich, die Zahlen ändern sich nicht.
3. **Größen-Preflight:** Die Preflight selbst wird nicht als Byte abgerechnet. Sie
   erzeugt keine Bytes, liest keine gespeicherten Bytes und hasht nichts; sie
   läuft geliehen über das Item-Modell. Root hat diese „geliehene Größenberechnung“
   ausdrücklich als Weg benannt. Die Zellarbeit (1 je Entity) läuft unverändert
   vorab.

### Hinweise R2-H1 bis R2-H3

- **R2-H1 (zweites Q-Parsen) — entfernt:**
  - `raw::frame_q` (`raw.rs:580`) gibt das eine, abgerechnete Parsen von Q mit
    zurück; `raw::frame` ist ein dünner Wrapper (`raw.rs:571`).
  - `Lasts::load` (`engine.rs:633`) parst nicht mehr ein zweites Mal.
  - `player_last` (`engine.rs:575`, `:589`) hatte dasselbe Muster (Parsen in
    `frame`, dann unabgerechnet `raw::q`, dann in `receipt_history` noch einmal
    abgerechnet). Es nutzt jetzt das eine Parsen und ruft `validated` direkt auf.
    Dadurch fallen ein unabgerechnetes und ein abgerechnetes Parsen weg;
    `receipt_history` hatte keinen weiteren Aufrufer und ist entfernt.
  - Unverändert, weil abgerechnet: Lookup, Replay, Reopen
    (`block_validation.rs`, FCBR-Zweig) und der v1-Receipt-Digest parsen Q für die
    Relationen ein zweites Mal und rechnen es ab. Das steht so in `INVARIANTS.md`.
- **R2-H2 (Fehlermaskierung) — behoben:**
  - `keep_cause` und `then_cleanup` (`block_validation.rs:164`, `:173`): Ein
    fehlgeschlagener Open, Init oder Scan meldet seinen eigenen Fehler. Scheitert
    der Scratch-Cleanup zusätzlich, wird das per `tracing::warn!` mit beiden
    Fehlern protokolliert, statt den Fehler zu ersetzen. Nur ohne vorherigen Fehler
    ist der Cleanup-Fehler der gemeldete.
  - Angewandt in `initialize` (`:227`), `scan` (`:347`) und an allen drei
    `index.finish()`-Stellen des Openers (`blocks.rs:388`, `:402`, `:412`).
  - Test: `original_failure_is_reported_when_scratch_cleanup_also_fails`
    (`block_validation/tests.rs:473`, redb). Er sabotiert jeweils die eigene
    Scratch-Datei durch eine fremde Ersetzung und prüft drei Fälle:
    - Init-Fehler: leere redb-Datei als existing, ergibt `CatalogMismatch("empty
      redb file…")`.
    - Scan-Fehler: fremde Tabelle, ergibt `CatalogMismatch`.
    - Nur Cleanup-Fehler (fresh): ergibt `ScratchCleanup`, nichts committet.
    - Die fremde Datei bleibt jeweils erhalten. Vorher lieferten die ersten beiden
      Fälle `ScratchCleanup`.
- **R2-H3 (TOCTOU in `remove_owned_path`) — nur dokumentiert:**
  - Die übernommene DUR-03.2-Funktion (`uuid_migration/files.rs:539–555`) ist
    unverändert. Restrisiko steht jetzt in `INVARIANTS.md` (Opener-Abschnitt): Wer
    Schreibrechte im Verzeichnis hat und im Fenster zwischen Inode-Prüfung und
    `unlink` eine fremde Datei an den Pfad umbenennt, verliert sie.
  - Die Review-Einordnung bleibt gültig: Andere Öffner schreiben nie in eine leere
    vorhandene Datei.

### Restfälle der harten 512-MiB-Grenze (korrigierte Darstellung)

**Korrektur:** Runde 5 nannte „8 verschiedene maximale Receipts“ als einzigen
Restfall (Abschnitt „Rechnung“, „Offene Punkte“; dort jetzt mit Korrekturvermerk).
Das war unvollständig. Die 8 sind nur die Schwelle für die *billigste*
Zusammensetzung. Rechnung: `round6-thresholds.py`, Ausgabe `round6-thresholds.log`.
Kostenklassen wie im Code: Zeilen-Lesen einmal, Q-Digest einmal je Receipt,
Image-Decodes, Entity-Kodierungen mit exakter Länge, Effekt-Entities 2× (Kanonik)
+ 1× (Apply bzw. Encode der ersetzten Entity), Journalzeilen lesen und
neu kodieren.

**Entity-Zahlen je Spalte:** Allein führen sie nicht mehr zur Verweigerung. Der
Entity-Tail eines Chunks ist auf 2 MiB begrenzt (Σ(16 + L)). Ein Snapshot kodiert
damit höchstens 8 · 3 · 2 MiB = 48 MiB Entities, ein Commit höchstens
16 · 2 · 2 MiB = 64 MiB.

| Entity | Länge L | max. je Spalte | Ergebnis |
| --- | --- | --- | --- |
| leere Truhe | 301 | 4096 (`MAX_BLOCK_ENTITIES`) | Probe 7 (4096 × 3 Spalten Snapshot, 2-Spalten-Commit; 1100 × 8 Spalten Commit) jetzt zugelassen, **ausgeführt** |
| Test-Truhe `chest_bytes` | 1651 | 1258 | zugelassen |
| an der Formgrenze | 16 384 | 127 | Entity-Kodierungen ≤ 48/64 MiB, allein keine Verweigerung; Verweigerung erst über Receipt- und Effektbytes (unten) |

**Verschiedene maximale letzte Receipts (Q an der 32-MiB-Grenze), Snapshot:**

| Zusammensetzung je Receipt | Kosten je Receipt | Schwelle |
| --- | --- | --- |
| (a) jede: nur Lesen und Digest, 16 + R + Q | ≥ 64,0 MiB | **8** immer verweigert (512 MiB + 1.272 Bytes) |
| (b) Image-CAS über 8 Spalten aus Fresh, alle Nachbilder aktuell | ≈ 3 Q = 96,0 MiB + eigene Spalte | **6** immer verweigert; **5** (480,0 MiB) nur, wenn eigene Spalten (Lesen, Nachbild-Decode, View-Entities) zusammen über 32,0 MiB liegen (die eigenen Paare und Views sind über die 32-MiB-Ausgabegrenze beschränkt) — gerechnet |
| (c) Gameplay mit Vor- und Nachbildern, kleine Entities | ≈ 2,5 Q = 80,0 MiB + eigene Spalte | **7** verweigert; **6** (480,0 MiB) abhängig von den eigenen Spalten — gerechnet |
| (d) 1-Spalten-Gameplay, 1019 Effekte, die eine Truhe an der Formgrenze umschalten (Q = 33.488.850) | ≈ 7,0 Q = 223,4 MiB | **3** verweigert (670,2 MiB), **2** zugelassen (446,8 MiB) — **ausgeführt**, N/N−1 exakt |

**Commit mit eigenem maximalem Request (Q0 = 32 MiB):**

| eigener Request | Kosten | Historien | Schwelle |
| --- | --- | --- | --- |
| Image-Art über 8 Spalten, Vor- = Nachbildgröße | ≈ 6 Q0 = 192 MiB | (a) 16 + R + Q | **5** — gerechnet |
| dito | dito | (b) Image-CAS | **4** (Reviewer-Wert bestätigt) — gerechnet |
| dito | dito | (c) Gameplay | **4** — gerechnet |
| effekt-dicht wie (d) | ≈ 8 Q0 = 255,3 MiB | (d) | **2** verweigert (≈ 702 MiB), **1** zugelassen (≈ 478,7 MiB) — **ausgeführt** |

Die Reviewer-Werte „ab 6“ (Snapshot) und „ab 4“ (Commit) sind damit für (b)
bestätigt. Effekt-dichte Receipts mit Entities an der Formgrenze liegen darunter
(3 bzw. 2). Alle Restfälle enden typisiert mit `ValidationCapacity("bytes")` vor
jedem Schreibzugriff und ohne Teilergebnis. `INVARIANTS.md` (Abschnitt „Residual“)
und `README.md` sind entsprechend korrigiert. Einordnung wie in Runde 5: Vertrag
Z. 269f. („Maximum shape does not guarantee admission“); über die Annahme dieser
Restfälle entscheidet Root.

### Neuer Befund R6-B2 (nur dokumentiert, Root-Entscheidung im Dialog)

**Befund:** Der Startup-Scan (Reopen) liest im Journal-Zweig für **jede**
Journalzeile das zugehörige Receipt und hasht dessen Q erneut
(`block_validation.rs`, Zweig `raw::JOURNAL`: `owned(Receipt)` + `raw::frame` je
Zeile). Die Relationskosten wachsen damit mit m · (16 + R + Q) je Receipt mit
m Effekten.

| Fall | Relationskosten |
| --- | --- |
| maximales effekt-dichtes Receipt (1019 Effekte, Q ≈ 32 MiB) | ≈ 64 GiB |
| Bulk-Edit, 1024 kleine Effekte, Q ≈ 2 MiB | ≈ 4 GiB |

- **Folge:** Unter einer kleineren Relations-Allowance verweigert der Reopen
  typisiert mit `ScanCapacity("bytes")`, ohne Schaden. Das ist trotzdem ein
  Betriebs- und Liveness-Thema.
- **Herkunft:** besteht seit Runde 3, unabhängig von B1.
- **Umgang:** Auf Rückfrage hat Root entschieden, den Befund nur zu dokumentieren;
  der Code bleibt unverändert. Er steht in `INVARIANTS.md` (Reopen-Abschnitt).
- **Abhilfe (Vorschlag für einen eigenen Auftrag):** Die Zeilen einer Operation sind
  zusammenhängend. Das zuletzt gerahmte Receipt kann für Folgezeilen
  wiederverwendet werden, dann kostet jedes Receipt nur einmal.
- **Testfolge:** Der Restfall-Test (d) öffnet deshalb jeden redb-Store frisch unter
  seiner Grenze, statt ihn wieder zu öffnen.

### Tests (Memory und redb)

Alle Sollwerte stammen aus den festen FCBQ/FCBR/FCBL-Layouts und den
Kodierlängen des World-Codecs, nie aus einem von Storage gemeldeten Bedarf.

**Neu: `tests/player_block_entity_bytes.rs` (je Test Memory und redb)**

- `probe7a_max_entity_density_columns_are_admitted_at_the_hard_default_caps`:
  Probe 7a des Reviewers als Regressionstest. 4096 leere Truhen je Spalte, harte
  Default-Grenzen.
  - Der 3-Spalten-Snapshot nach einem 3-Spalten-Gameplay wird zugelassen
    (Revision 2, 4096 Entities je Spalte).
  - Ebenso der 2-Spalten-Gameplay-Commit: Das Receipt ist gespeichert und gleich,
    nur die committeten Spalten rücken eine Revision vor.
  - Danach wird auch ein 1-Spalten-Commit zugelassen.
- `probe7b_eight_columns_with_1100_entities_are_admitted_at_the_hard_default_caps`:
  Probe 7b, 1100 leere Truhen je Spalte. 8-Spalten- und danach 4-Spalten-Commit
  werden zugelassen, mit denselben Prüfungen.
- `realistic_entity_union_admits_its_exact_byte_need_and_refuses_one_less`:
  N/N−1 mit Entity-Daten.
  - Aufbau: 4 Spalten mit je 200 realistischen Truhen (`chest_bytes`, L = 1651),
    dann je Spalte ein eigenes Gameplay, das eine weitere Truhe setzt
    (Entity-only-Effekt). Die Union hat damit vier verschiedene Gameplay-Receipts.
  - Sollwert je Spalte, unabhängig berechnet: eigene Lesezugriffe; Receipt-Zeile
    und Q-Digest; Relationen (Identität, Header/Full/Overlay, Journalzeile gelesen
    und neu kodiert); beide Image-Decodes; Effekt-Entity 3·n; 3 · (Σ L der
    View-Entities). R und Q werden gegen die Layouts geprüft.
  - Ergebnis: N zugelassen (Stamps, Full-Bytes und Chunks gleich der Referenz),
    N−1 → `ValidationCapacity("bytes")`.
  - Zusätzlich wird geprüft, dass schon die frühere Pauschale (3 · E · 16 KiB)
    allein über N läge.
- `form_bound_entities_exceed_a_byte_cap_typed_without_mutation`: Negativfall.
  - Kappe 16 · 64 · 16 384 − 1. Das ist die hergeleitete Untergrenze der
    entity-proportionalen Abrechnung von 64 gesetzten Truhen an der Formgrenze
    (Kommentar im Test).
  - Der Commit wird typisiert verweigert, ohne Mutation: Stamp und Full-Bytes
    unverändert, kein Receipt.
  - Derselbe Commit mit realistischen Truhen wird unter derselben Kappe zugelassen.
- `distinct_effect_dense_form_bound_receipts_are_the_hard_cap_residual`:
  Restfall (d), siehe oben.
  - Exaktes N für 2 Spalten aus dem Modell; N zugelassen, N−1 verweigert, je
    Backend in einem frisch aufgebauten Store.
  - 3 Spalten verweigert; Plausibilität 6 Q < N/2 < 8 Q.
  - Bei harter Grenze: 2-Spalten-Commit verweigert ohne Mutation,
    1-Spalten-Commit zugelassen.
  - Laufzeit ≈ 250 s in Debug.

**Neu: Unit-Tests**

- `entity_len_is_the_exact_canonical_encoding_length_or_the_form_bound`
  (`block_durability/tests.rs:238`):
  - 9 kodierbare Entities (leere Truhe, Schilder beider Arten, Farb-, Glüh- und
    Wachsvarianten, 512er-Mehrbyte-Zeilen, Truhen mit Komponenten, die exakte
    16-KiB-Truhe des World-Golden-Layouts): `entity_len` = tatsächliche
    Kodierlänge.
  - Unabhängige Anker: 301 (leere Truhe), 38 (leeres Schild, Farbe „black“),
    16 384.
  - Verweigerte Kodierungen: Truhe 1 Byte über der Grenze ergibt 16 384; zu lange
    Schildzeile ergibt die rechnerische Länge 4138.
  - Kanonik-Abrechnung: Legacy-Truhe mit 28 Bytes ergibt 301, kanonische Eingabe
    ihre eigene Länge.
- `original_failure_is_reported_when_scratch_cleanup_also_fails` (R2-H2, siehe
  oben).

**Geändert: `tests/player_block_bytes.rs` (Sollwerte hingen an der Pauschale)**

| Test | Runde 5 | Runde 6 | Änderung |
| --- | --- | --- | --- |
| `shared_receipt_union_…` | N = 28.389.537 | N = 20.846.241 | Entity-Term 8·64·16.384 durch 8·64·1.651 ersetzt; R, Q, Full unverändert (Handrechnung des Reviewers, nur dieser Term getauscht) |
| `shared_near_cap_receipt_union_…` | 239.155.769 | 133.078.169 | − 8·900·(16.384 − 1.651); weiterhin ≤ 512 MiB geprüft |
| `distinct_receipts_…` | 13.656.795 | unverändert | keine Entities |

- **Modell:** `column()` nimmt jetzt die World-kodierte Länge der View-Entities
  statt `ENTITY · Anzahl`; die Konstante `ENTITY` ist entfernt. `sizes()` erhält
  die Spaltenzahl als Parameter.
- **`shared_…` exakte Grenze nur auf redb:**
  - Memory fixiert seine Kappe bei der Konstruktion, das Setup läuft also unter N.
  - Der Setup-Write kostet je Spalte strukturell mehr als dieser Snapshot: Parse,
    Decode, Encode, Kopie, Frame und Retain ergeben ≈ 6·A, der Snapshot dagegen
    Lesen, Digest, Relation und Decode ≈ 5·A plus ΣL < A. Nur die frühere Pauschale
    (+8 MiB) hob N über den Write.
  - Memory prüft hier deshalb Zulassung und Kohärenz.
- **Neuer Test `half_shared_union_admits_its_exact_byte_need_and_refuses_one_less`:**
  4 Spalten teilen ein Receipt, 4 haben je ein eigenes. Der Snapshot übersteigt
  jeden Setup-Write. Damit prüft er exakt N/N−1 auf Memory **und** redb, und die
  Einmal-Lesung des geteilten Receipts unterscheidet N von N−1.

**Bestand:** Alle übrigen Tests sind unverändert und grün.
- Dev-Lauf vor dem Freeze (`round6-dev-tests-full.log`): **354/354** in
  18 Ergebnisblöcken (Lib 255, 16 Integrations-Binaries mit 87, Doctests 12).
- Runde 5 hatte 346. Neu sind 2 Unit- und 6 Integrationstests (5 in
  `player_block_entity_bytes`, 1 in `player_block_bytes`).

### Finale Checks (eingefrorener Kandidat, ohne Source-Änderung)

`round6-final-checks.sh` in `round6-isolated` (main 8da4c770 plus Kandidat).

- **Ablauf:** Detached gestartet (`setsid nohup`), 2026-10-12T12:05:59 bis
  12:42:03 (+02:00). Während der Checks gab es keinen Sync und keine Quelländerung.
- **Target:** frisch und leer (`round6-final-target`); das Skript bricht ab, falls
  es schon existiert. Clippy hat `ferrumc-storage` nachweislich neu geprüft
  („Checking ferrumc-storage“), rustdoc neu dokumentiert.
- **Flaky-Wiederholung:** Der Verbraucher-Lauf scheiterte nicht, die vorgesehene
  Einzelwiederholung von `multiplayer_visibility` wurde deshalb nicht ausgelöst.

| Check | Ergebnis | Log |
| --- | --- | --- |
| `cargo test -p ferrumc-storage --jobs 2 --locked --no-fail-fast` (Debug) | exit 0, **354/354** in 18 Ergebnisblöcken (Lib 255, 16 Integrations-Binaries, Doctests 12), 0 failed, 0 ignored, 1103 s | `round6-final-debug-tests.log` |
| dito `--release` | exit 0, **354/354** in 18 Blöcken, 416 s | `round6-final-release-tests.log` |
| `cargo clippy --workspace --all-targets --jobs 2 --locked -- -D warnings` | exit 0, 0 Warnungen, 65 s | `round6-final-clippy.log` |
| `cargo fmt --all --check` | exit 0 | `round6-final-fmt.log` |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-storage --no-deps --jobs 2 --locked` | exit 0, 0 Warnungen | `round6-final-rustdoc.log` |
| `cargo build --workspace --all-targets --jobs 2 --locked` | exit 0, 0 Warnungen, 225 s | `round6-final-build.log` |
| `cargo test -p ferrumc-sim -p ferrumc-app -p ferrumc-testkit --jobs 2 --locked --no-fail-fast` | exit 0, **844 bestanden, 0 fehlgeschlagen, 1 ignoriert** (App-Java-Kompatibilitätstest, braucht Java/Maven-JARs), 58 Ergebnisblöcke, 211 s; kein Flaky-Fehler | `round6-final-consumers.log` |
| Manifest vor/nach | identisch `3c538386…6664`, isolierte Kopie = Arbeitsbaum = Kandidat | `round6-final-manifest-{before,after}.json` |
| Basis-Abgleich außerhalb Storage | alle 1313 Archivdateien unverändert (`outside_unchanged: true`); Zusatzdateien siehe unten | `round6-final-summary.log` |

**Reviewer-Probes** (unveränderte Dateien in der separaten Kopie
`round6-probe/` des eingefrorenen Baums, Debug):

| Probe (Datei, SHA-256) | Ergebnis | Einordnung |
| --- | --- | --- |
| Runde 1 `root_review_probes.rs` (`d74de1b9…4754453`) | exit 0, **4/4** (Probe 1 Kollision ohne Mutation, Probe 2 Race atomar, Probe 3 `ScratchUnavailable` ohne Datei, Probe 4 1..8 Spalten zugelassen) | unverändert grün |
| Runde 2 `root_review_r2_probes.rs` (`15eefd30…7d92`) | exit 101: Probe 5 grün (Lookup/Replay exakt), Probe 6 scheitert **nur** an N−1 auf redb (N = 6.770.595 und N−1 zugelassen) | **erwartet:** Der Probe-6-Sollwert enthält die alte Pauschale `ENTITY · B_CHESTS` je View |
| dito, angepasste Kopie `round6-probe6-adapted.rs` (`049afbf2…3979`; einzige Änderung: View-Term durch World-kodierte Entity-Länge ersetzt) | exit 0: N = **5.827.683** zugelassen, N−1 → `ValidationCapacity("bytes")`, Memory und redb, Rest der Probe unverändert grün | Abstand 942.912 = 64 · (16.384 − 1.651), genau die entfallene Pauschale |
| Runde 2 `root_review_r2_entity_probes.rs` (Probe 7, `a74d74c3…94dd`) | exit 101: 7a 3-Spalten-Snapshot und 7b 8-Spalten-Commit werden jetzt **zugelassen** statt verweigert | **beabsichtigt** (B1). Dieselben Formen sind im Kandidaten als Regressionstest grün (`probe7a_…`, `probe7b_…`) |

**Nebenprodukte (separat ausgewiesen, keine Quellen):**
- Der Testhost-Test der App hat in die Prüfkopie der finalen Checks geschrieben:
  `.codex-tmp/p57-plugin-testhost-3387908-artifacts/libplugin_testhost_fixture.so`
  und `…/librenamed_fixture.so` (je 8.454.176 Bytes).
- `scripts/__pycache__/check-forbidden-claims.cpython-314.pyc` entstand durch
  meinen Forbidden-Claims-Scan: Python hat das Skript aus der isolierten Kopie
  importiert, kein Check-Artefakt.
- Nur wegen dieser drei Dateien meldet das Skript
  `verify-copy=FAIL (see extra_outside)`. Die Probe-Kopie wurde ohne die
  `.so`-Dateien angelegt.

**Forbidden-Claims-Scan** (`scripts/check-forbidden-claims.py`,
`round6-claims-scan.log`):
- Delivery: 0 Treffer.
- Geänderte Rust-Dateien (rustdoc): 0 Treffer.
- `INVARIANTS.md`/`README.md`: nur bestehende, schon in Runde 5 gemeldete Stellen
  außerhalb der Runde-6-Änderungen (Rust-Fachbegriff für Trait-Objekte und
  Kompatibilitätswortlaut des Bestands).

### Offene Punkte und Grenzen (Runde 6)

- **Restfälle** der harten Byte-Grenze, wie oben beziffert: Snapshot ab 3 bis 8,
  Commit ab 2 bis 5 verschiedenen großen Receipts, abhängig von der
  Zusammensetzung. Die Annahme entscheidet Root.
- **R6-B2:** Startup-Kosten je Journalzeile, nur dokumentiert.
- **R2-H3:** TOCTOU-Restrisiko, nur dokumentiert.
- **Messung nur teilweise:** Die Schwellen (b), (c) und die Commit-Fälle außer (d)
  sind gerechnet, nicht ausgeführt.
- **Keine Aussagen** zu RSS, Prozessabbruch, Stromausfall, Survival,
  Paper-Gleichheit oder App-Aktivierung. DUR-03.3a (Format6→7) bleibt separat.

### Evidenz (Runde 6)

Unter `.codex-tmp/dur03_3-runtime/`:
- **Kandidat und Basis:** `round6-candidate-manifest.json`,
  `round6-candidate-frozen-at.txt`, `round6-candidate-src/` (schreibgeschützt),
  `round6-base.freeze.json`.
- **Finale Checks:** `round6-final-checks.sh`, `round6-final-*.log`,
  `round6-final-summary.log`, `round6-final-manifest-{before,after}.json`,
  `round6-final-nohup.out`.
- **Restfall-Rechnung:** `round6-thresholds.py`, `round6-thresholds.log`.
- **Reviewer-Probes:** angepasste Kopie `round6-probe6-adapted.rs`; Probe-Kopie
  `round6-probe/`.
- **Entwicklung:** `round6-dev-*.log`, darunter die volle Dev-Suite
  `round6-dev-tests-full.log`, `round6-tools.py`, `round6-run.sh`,
  `round6-delivery-draft.md`.
- **Freeze:** alle Dateien mit SHA-256 und dem Hash dieser Delivery in
  `round6-evidence-freeze.json`.

Die Evidenz der Runden 3 bis 5 und die `round{3,4,5}-candidate-src/` sind
unverändert.

## Runde 5

### Grundlagen und Prüfbasis

| Grundlage | SHA-256 |
| --- | --- |
| Vertrag `DUR-03_3_PLAYER_BLOCK_DURABILITY.md` (maßgeblich Z. 248–272) | `2edfc588fd8d7e96753f6beb4bb7c2ad50a064e4c266681ab5ff94550c4342ab` |
| Delivery vor dieser Ergänzung (Stand Runde 4) | `415c8c3559dc39b4f102179b3ca81bdd0268b79dd9ed9786c1aeb0f94afb2eb4` |
| Ausgangskandidat Runde 4 (Manifest, 89 Dateien) | `616a42792bfc1e5ac4e8cf1dedb40bc3e8e78f21775e3aba5c5d47230b4a42d4` |

Der Arbeitsbaum `server/crates/ferrumc-storage/**` war zu Beginn und unmittelbar vor
dem Freeze byte-identisch mit dem Runde-4-Manifest (keine fremde Änderung).
Prüfbasis ist main `ce39d817aa7216beacc2db9bca236d3942910b98` (Storage, `Cargo.toml`,
`Cargo.lock` identisch zu `75f9824a`, `git diff` leer): private Kopie per `git archive`
in `.codex-tmp/dur03_3-runtime/round5-isolated/`, 1297 Dateien, Basis-Freeze
`round5-base.freeze.json` (Digest
`8ba0f423e03896a4cfcf8093fab63bc958e3cf8a93e548024fee560bc6d630f4`). Entwickelt
wurde direkt in dieser Kopie mit eigenem Dev-Target; der Arbeitsbaum wurde erst
beim Freeze synchronisiert, gebaut wurde dort nie. Toolchain `rustc 1.99.0
(b940084d7 2026-09-28)`, `cargo 1.99.0`, `--jobs 2 --locked`, keine Dependency-,
Lockfile- oder Fremd-Crate-Änderung, kein Git.

### Kandidat und Freeze

Vor den finalen Checks eingefroren (2026-10-11T21:13:11+02:00): `ferrumc-storage`,
**90 Dateien**, Manifest-SHA-256
`1d8455ef0b8740068c54cf28e96947f8f82c8d8ff0b6c079e6da0d768a75f678`
(`round5-candidate-manifest.json`, Dateihash
`493ebb50339b44ccfdb383bdcd35d56e15472ebca207006ff045dbfe19f51a4c`, Schema
`pfad\tbytes\tsha256\n`). Schreibgeschützte Kopie: `round5-candidate-src/`.
Gegenüber Runde 4 geändert: `INVARIANTS.md`, `README.md`,
`src/block_durability/engine.rs`, `src/block_durability/raw.rs`,
`src/block_durability/tests.rs`, `src/memory/ownership.rs`,
`src/redb_store/ownership.rs`, `src/redb_store/block_validation.rs`,
`src/ownership/engine.rs`; neu: `tests/player_block_bytes.rs`.

### Befund: Was die Byte-Grenze beim Lesen tatsächlich abrechnete

Messung auf dem Runde-4-Kandidaten (redb, ein 8-Spalten-Image-CAS, alle acht
Spalten teilen ein Receipt mit R = 3.146.729 Bytes; minimale zugelassene
Byte-Grenze per Binärsuche, `round5-dev-probe1.log`): 1 Spalte 23.600.866 (7,5 R),
2 Spalten 40.907.634 (13 R), 8 Spalten **144.748.242 (46 R)**, also +5,5 R je
weiterer Spalte. Die Zerlegung je gelesener Spalte im Runde-4-Code:

1. **Lesen, dreifach gezählt:** Eine physische Receipt-Zeile wurde dreimal
   abgerechnet: im budgetierten Reader (`memory/ownership.rs:140` bzw.
   `redb_store/ownership.rs:118`), im `Meter` (`block_durability/engine.rs:1074–1076`)
   und in `own()` (`engine.rs:113–122`). Block-Tabellen wurden doppelt gezählt
   (`Meter`, `engine.rs:1092–1094`, und `get()`, `engine.rs:103–112`). Das sind
   keine zusätzlichen Zugriffe.
2. **Hashen:** Der FCBQ-Body wird einmal für Parse und Digest gezählt
   (`raw.rs:538`). Das ist ein echter Digest und bleibt unverändert.
3. **„Kopieren“, ein echter Vertragsverstoß:** `raw::receipt` rechnete die ganze
   Zeile ab und kopierte sie mit `Arc::from(bytes)` (`raw.rs:567–569`). Aufgerufen
   wurde es im Validierungspfad von `inspect_chunk` (`engine.rs:610`) und
   `player_last` (`engine.rs:563`), auch beim Reopen (`block_validation.rs:496`,
   `:553–555`) und in `block_receipt_digest` (`ownership/engine.rs:352`). Die Kopie
   wurde nur validiert und danach verworfen. Vertrag Z. 267f. („Parse receipt
   frames borrowed“) verlangt geliehenes Parsen. Die Kopie ist deshalb entfernt,
   nicht nur ihre Abrechnung.
4. **Snapshot-Ausgabe doppelt:** Die rohe Paar-Kopie einer Spalte
   (`engine.rs:738`) und die beiden Player-Kopien (`engine.rs:713`, `2·len`) wurden
   zusätzlich zur Ausgabegrenze (`engine.rs:726`, `:710`) unter die
   Validierungsbytes gebucht. Nach Vertrag Z. 266f. belastet die Ausgabegrenze
   genau diese Kopien („charges keys, headers, images and materialized encoded
   view sizes before copying“). Eine doppelte Buchung verlangt der Vertrag nicht.

Antwort auf die Root-Frage 2: Die dritte Zählung entsprach einer tatsächlichen
Kopie, die vertragswidrig war; sie ist entfernt (Punkt 3). Kopien, die in die
Snapshot-Ausgabe gehen, zählen nur noch unter die 32-MiB-Ausgabegrenze (Punkt 4).
Kopien, die tatsächlich stattfinden und keine Snapshot-Ausgabe sind, bleiben
abgerechnet. Das betrifft das zurückgegebene Receipt bei Lookup, Replay und Commit
(`raw.rs:540–551`) und das Paar aus `chunk_state` für Einzel-Loads und Reopen
(`engine.rs:699`).

**Eigenständige Entscheidung, bitte im Review prüfen:** Punkt 1 stand nicht
ausdrücklich in der Root-Liste. Ich habe ihn nach demselben Prinzip behoben
(„keine Abrechnung für nicht stattfindende Arbeit“), weil ein unabhängiger
N/N+1-Sollwert sonst eine Mehrfachzählung pro Zugriff festschreiben müsste. Jede
physische Zeile wird jetzt genau einmal an der Quelle abgerechnet (Schlüssel plus
Wert), im budgetierten Reader. Alle `Meter`-Instanzen umhüllen Reader mit derselben
`Budget`-Instanz (gleiches `Rc`): Memory `block_read`/`block_write`, redb
`commit`/`commit_v1`/`block_read` und Reopen. Kein Pfad verliert dadurch seine
Abrechnung. Die Zählklassen sind unverändert: Zeilen-Lesen, Q-Digest einmal je
Receipt, Decodes, Encodings mit 16-KiB-Reserve je Block-Entity, echte Kopien.
Harte Grenzen, Zellmodell und Startup-Struktur sind ebenfalls unverändert.

### Umsetzung (Datei:Zeile im Kandidaten)

- **Geliehene Frames:** `raw::Frame` (`raw.rs:523`), `raw::frame`
  (`raw.rs:562`; Body einmal für Parse und Digest abgerechnet, `:579`), ohne Kopie.
  `Frame::owned` (`raw.rs:540`) erzeugt den einen retained Buffer nur für
  zurückgegebene Receipts und rechnet vor dem Kopieren ab: Lookup
  `engine.rs:846`, Replay `:917`, Commit-Plan. `raw::receipt` = `frame` + `owned`
  (`raw.rs:554`).
  `receipt_relations`/`receipt_history`/`validated`/`proved` arbeiten auf `&Frame`.
- **Receipt-Cache pro Aufruf:** `Rows`/`Lasts` (`engine.rs:572–619`).
  - Aufbau: höchstens `MAX_BLOCK_CHUNKS` = 8 Slots als `OnceCell`-Arena, Schlüssel
    Receipt-ID. Gehalten werden nur geliehene Zeilen und deren geparste
    `(Frame, Q)`.
  - Lebensdauer: lokal angelegt in `snapshot` (`:769`), `prepare` (`:945`) und
    `chunk_state` (`:691`), mit dem Aufruf verworfen. Es gibt keinen Zustand über
    Aufrufe hinweg.
  - Laden: Ein Fehlgriff liest, hasht und parst die Zeile genau einmal, bevor
    damit gearbeitet wird (`:592–620`). Jeder weitere Zugriff auf dieselbe ID
    nutzt den geparsten Frame; die Prüfung bleibt fail-fast.
  - Grenze: Ein 9. verschiedenes Receipt wäre typisiert `MalformedData`. Das kann
    nicht eintreten, weil jede gelesene Spalte höchstens ein Receipt beiträgt.
  - Nutzung: `inspect_chunk` (`engine.rs:624`, `:650`) holt das letzte Receipt
    über den Cache. Die Relationsprüfung (`validated`, einmal je ID) und der
    Spaltenbeweis (einmal je `(ID, Index)`) sind unverändert aus Runde 4.
- **Prepare:** vergleicht die gespeicherten Originale geliehen (`engine.rs:945–950`)
  statt über eine eigene Paar-Kopie.
- **Einmalige Lese-Abrechnung:**
  - `get`/`own`/`identity` ohne Zweitbuchung (`engine.rs:107–122`).
  - `Meter` reicht Lesezugriffe durch und behält nur die festen Zählerlesungen
    128/256 (`engine.rs:1087ff`, `:1119`, `:1132`).
  - `Reader::block` rechnet jetzt an der Quelle ab: `memory/ownership.rs:158`,
    `redb_store/ownership.rs:147`.
  - Die doppelte Reopen-Buchung der Journal-Receipt-Zeile ist entfernt
    (`block_validation.rs:553`).
- **Snapshot:** Die rohe Paar-Kopie belastet nur die Ausgabegrenze
  (`engine.rs:794`). Beim Player wird nur das Decodieren einmal gezählt
  (`engine.rs:761`); beide Kopien belasten die Ausgabegrenze.
- **Dokumentation:** Byte-Modell, Cache und Restfall stehen in `INVARIANTS.md:402–421`,
  eine Kurzfassung in `README.md`.

### Rechnung

Byte-Modell (`INVARIANTS.md:402–421`) für einen Snapshot ohne Player:

- **Fester Anteil:** 419 Bytes (Identitätszeile 35, Zählerlesungen 128 + 256).
- **Je Spalte:** eigene Lesezugriffe (79 Bytes Schlüssel und Header plus Full- und
  Overlay-Zeilen), das Decodieren ihres bewiesenen Paars und 16 KiB je
  View-Entity.
- **Je verschiedenem letztem Receipt:** 16 + R + Q, dazu seine Relationen
  (Identität 35, je Eintrag Header, Full und Overlay).

Für ein 8-Spalten-Image-CAS aus Fresh mit geteiltem Receipt ergibt das
**N = 1.734 + R + Q + 3·ΣFull + 8·E·16 KiB**. Die Größen R und Q werden gegen das
FCBR-/FCBQ-Layout geprüft (R = Q + 71 + 9·8, Q = 98 + 46·8 + ΣPaare).

| Szenario | Werte | Ergebnis |
| --- | --- | --- |
| Messung ohne Entities, R = 3.146.729 | Bedarf 15.733.409 = **5,0 R**, exakt nach Modell (`round5-dev-probe2.log`) | Runde 4: 46 R |
| Test „geteilt“ | R = 4.000.233, Q = 4.000.090, ΣFull = 3.999.624, E = 64 | **N = 28.389.537**. Wer pro Spalte neu liest und hasht, braucht mindestens 8·(16+R+Q) = 64.002.712 > N |
| Test „nahe Q-Grenze“ | Q = 30.297.506 (28,9 MiB), R = 30.297.649, je 900 Truhen | Bedarf 239.155.769 (228,1 MiB) ≤ 512 MiB |
| Test „verschieden“ | 8 Receipts, je R = 525.289, Q = 525.146, berührte Spalte 65.585 | **N = 13.656.795** |

> **Korrekturvermerk Runde 6:** Die folgende Restfall-Darstellung ist
> unvollständig. „Acht“ ist nur die Schwelle der billigsten Zusammensetzung (nur
> Lesen und Digest). Je nach Zusammensetzung verweigern schon 6 (Image-CAS) oder
> 3 (effekt-dichte Receipts mit Entities an der Formgrenze) verschiedene große
> Receipts einen Snapshot, ein Commit mit eigenem großem Request schon 4 bzw. 2.
> Die 16-KiB-Pauschale je Entity, auf der Teile dieser Rechnung beruhen, ist
> entfernt. Maßgeblich ist „Runde 6 → Restfälle“.

**Restfall (vertragskonforme Grenze).** Acht verschiedene letzte Receipts an der
Q-Grenze brauchen allein für Lesen und Digest
8 · (16 + (32 MiB + 143) + 32 MiB) = 536.872.184 = 512 MiB + 1.272 Bytes.
- **Verhalten:** Eine solche Union endet spätestens beim achten Receipt typisiert
  mit `ValidationCapacity("bytes")`. Der Snapshot ist rein lesend und liefert kein
  Teilergebnis. Ein Commit scheitert im Preflight vor jedem Schreibzugriff.
- **Einordnung:** Laut Root-Entscheidung ist das durch Vertrag Z. 269f. gedeckt
  („Maximum shape does not guarantee admission under a tighter work/byte cap“).
  Der Wortlaut nennt die engere Grenze; die Einordnung bei der harten Grenze ist
  die Root-Entscheidung dieser Runde.
- **Absicherung:** Die Rechnung steht als `const`-Assertion in
  `tests/player_block_bytes.rs:39` und in `INVARIANTS.md:418–421`.

### Tests (Memory und redb)

`tests/player_block_bytes.rs` (neu). Alle Sollwerte stammen aus dem Modell oben
und den festen Layouts, nicht aus gemeldeten Bedarfswerten.

- **`shared_receipt_union_admits_its_exact_byte_need_and_refuses_one_less`:**
  8-Spalten-Union mit einem gemeinsamen Receipt (24 verrauschte Sektionen, 64
  Truhen je Spalte).
  - An der Grenze N wird zugelassen; Stamps, Zellen und Full-Bytes stimmen mit
    der Referenz überein.
  - Bei N−1 kommt `ValidationCapacity("bytes")`.
  - Memory setzt die Grenze ab Konstruktion, deshalb läuft das Setup unter
    derselben Grenze. redb öffnet neu mit der Grenze.
- **`shared_near_cap_receipt_union_is_admitted_at_the_hard_default_byte_cap`:**
  Ein zweites 8-Spalten-Image-CAS über volle Bilder (Q = 28,9 MiB) wird bei der
  harten Default-Grenze 512 MiB zugelassen; Revision 2 für alle Spalten, redb auch
  nach Reopen. Q ist dabei durch die 32-MiB-Snapshot-Ausgabegrenze der Union
  begrenzt.
- **`distinct_receipts_refuse_typed_at_the_byte_cap_without_mutation`:**
  - Verschiedene Receipts: N/N+1 nach Modell.
  - Bei genau N wird ein Gameplay-Commit über die Union (ein Effekt je Spalte)
    typisiert mit `ValidationCapacity("bytes")` verweigert. Danach sind Stamps
    und Full-Bytes unverändert, und das Receipt existiert nicht.
- **Unit `per_call_receipt_cache_loads_each_row_once_and_holds_at_most_eight`**
  (`block_durability/tests.rs:169`):
  - Dreimal dieselbe ID ergibt einen Lesezugriff und genau einen Q-Digest.
  - 8 verschiedene IDs ergeben 8 Digests; die 9. wird typisiert verweigert.
  - Eine fehlende oder gefälschte Zeile wird verweigert.

**Gegenprobe „war vorher verweigert“.** Dieselbe, byte-identische Testdatei
(SHA-256 `af4f7627…97fb3e4`) lief auf dem unveränderten Runde-4-Kandidaten in
einer eigenen Kopie (`round5-baseline/`, `round5-baseline-round4-candidate.log`).
Alle drei Tests scheitern dort mit `ValidationCapacity("bytes")`:
- geteilter Receipt bei N;
- „nahe Q-Grenze“ schon beim Snapshot nach dem ersten, etwa 15 MB großen Write,
  bei 512 MiB;
- verschiedene Receipts bei N.

Bestehende Tests: alle 342 aus Runde 4 laufen unverändert grün (Dev-Lauf
`round5-dev-tests1.log` vor Ergänzung der neuen Tests).

### Finale Checks (eingefrorener Kandidat, ohne Source-Änderung)

`round5-final-checks.sh` in `round5-isolated` (main ce39d817 plus Kandidat). Während
der Checks gab es keinen Sync und keine Quelländerung. Gebaut wurde in einem
**frischen, leeren Target-Verzeichnis** `round5-final-target`; das Skript bricht ab,
falls es schon existiert. Damit gab es keine Cache-Wiederverwendung aus Entwicklung
oder Runde 4; Clippy und rustdoc haben `ferrumc-storage` nachweislich neu geprüft
bzw. dokumentiert. Gestartet wurde mit `setsid nohup`, damit kein Zeitlimit eines
Hintergrund-Jobs die Checks abschneidet. Ein erster Start wurde nach wenigen Sekunden
aus genau diesem Grund abgebrochen und mit wieder leerem Target-Verzeichnis neu
gestartet.

| Check | Ergebnis | Log |
| --- | --- | --- |
| `cargo test -p ferrumc-storage --jobs 2 --locked --no-fail-fast` (Debug) | exit 0, **346/346** in 17 Ergebnisblöcken (Lib 253, 15 Integrations-Binaries, Doctests), 0 failed, 0 ignored, 1493 s | `round5-final-debug-tests.log` |
| dito `--release` | exit 0, **346/346**, 550 s | `round5-final-release-tests.log` |
| `cargo clippy --workspace --all-targets --jobs 2 --locked -- -D warnings` | exit 0, 0 Warnungen, 158 s | `round5-final-clippy.log` |
| `cargo fmt --all --check` | exit 0 | `round5-final-fmt.log` |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-storage --no-deps --jobs 2 --locked` | exit 0 | `round5-final-rustdoc.log` |
| `cargo build --workspace --all-targets --jobs 2 --locked` | exit 0, 0 Warnungen, 502 s | `round5-final-build.log` |
| `cargo test -p ferrumc-sim -p ferrumc-app -p ferrumc-testkit --jobs 2 --locked --no-fail-fast` | exit 0, **844 bestanden, 0 fehlgeschlagen, 1 ignoriert** (App-Java-Kompatibilitätstest, braucht Java/Maven-JARs), 58 Ergebnisblöcke | `round5-final-consumers.log` |
| Original-Probes des Reviewers (unveränderte Datei, SHA-256 `d74de1b9…4754453`, separate Kopie `round5-probe/` des eingefrorenen Baums) | exit 0, **4/4** auf Memory und redb (Probe 1 Kollision ohne Mutation; Probe 2 Joint 6 Siege, 10 Niederlagen; Probe 3 `ScratchUnavailable`, keine Datei, Retry ok; Probe 4 1..8 Spalten zugelassen) | `round5-final-probes.log` |
| Manifest vor/nach | identisch `1d8455ef…f678`, isolierte Kopie = Arbeitsbaum = Kandidat | `round5-final-manifest-{before,after}.json` |
| Basis-Abgleich außerhalb Storage | alle 1297 Archivdateien unverändert (`outside_unchanged: true`) | `round5-final-summary.log` |

Neue Tests in den finalen Läufen: `player_block_bytes` 3/3 (Debug 121 s, Release
18 s) und der Cache-Unit-Test.

**Erzeugte `.so`-Testartefakte (separat ausgewiesen, keine Quellen):** Der
App-Plugin-Testhost-Test hat in der isolierten Kopie zwei Dateien geschrieben,
`.codex-tmp/p57-plugin-testhost-2012593-artifacts/libplugin_testhost_fixture.so`
und `…/librenamed_fixture.so` (je 8.453.256 Bytes). Nur ihretwegen meldet das
Skript `verify-copy=FAIL (see extra_outside)`; der Probe-Kopie wurden sie vorher
entzogen.

### Offene Punkte und Grenzen (Runde 5)

- **Korrekturvermerk Runde 6:** Der nächste Punkt nennt nicht den einzigen
  Restfall. Die korrigierten Schwellen stehen in „Runde 6 → Restfälle“.
- **Restfall:** Acht verschiedene maximal große letzte Receipts überschreiten
  512 MiB und werden typisiert verweigert (Rechnung oben). Das ist eine
  vertragskonforme Grenze laut Root-Entscheidung, keine Lockerung.
- **Bitte im Review prüfen:** die Vereinheitlichung auf eine Lese-Abrechnung je
  Zugriff (siehe Befund, Punkt 1). Sie senkt die gezählten Bytes aller
  Format7-Aufrufe, auch v1-Pfade in Format7 und Reopen, ohne eine Zählklasse
  wegzulassen.
- **Startup-Scan:** nutzt je `chunk_state` einen eigenen Cache, wie bisher je Zeile.
  Das ist seine separate kumulative Relations-Allowance (Vertrag Z. 264f.); über
  Zeilen hinweg wird nicht dedupliziert.
- **Lookup und Replay:** parsen Q für die Relationsprüfung weiterhin ein zweites
  Mal und zählen den Body dabei (unverändert aus Runde 4).
- **Keine Aussagen** zu RSS, Prozessabbruch, Stromausfall, Survival,
  Paper-Gleichheit oder App-Aktivierung. DUR-03.3a (Format6→7) bleibt separat.

### Evidenz (Runde 5)

Unter `.codex-tmp/dur03_3-runtime/`:
- **Kandidat und Basis:** `round5-candidate-manifest.json`,
  `round5-candidate-frozen-at.txt`, `round5-candidate-src/` (schreibgeschützt),
  `round5-base.freeze.json`.
- **Finale Checks:** `round5-final-*.log`, `round5-final-summary.log`,
  `round5-final-manifest-{before,after}.json`.
- **Gegenprobe:** `round5-baseline-round4-candidate.log`; dazu die Vorfassung mit
  der Testdatei vor `cargo fmt`, `round5-baseline-round4-candidate-prefmt.log`.
- **Messungen:** `round5-dev-probe1.log` (Runde 4), `round5-dev-probe2.log`
  (Runde 5), `round5-dev-numbers.log` (Testgrößen); die Mess-Probe als Text in
  `round5-dev-probe.rs.txt`, nicht Teil des Kandidaten.
- **Weitere Entwicklungslogs:** `round5-dev-*.log`, darunter der unveränderte
  Bestand 342/342 vor den neuen Tests (`round5-dev-tests1.log`).
- **Skripte:** `round5-tools.py`, `round5-run.sh`, `round5-final-checks.sh`,
  `round5-baseline-run.sh`.
- **Freeze:** alle Dateien mit SHA-256 und dem Hash dieser Delivery in
  `round5-evidence-freeze.json`.

Die Evidenz aus Runde 3 und 4, `round3-candidate-src/` und
`round4-candidate-src/` sind unverändert.

## Runde 4

### Grundlagen und Prüfbasis

| Grundlage | SHA-256 |
| --- | --- |
| Vertrag `DUR-03_3_PLAYER_BLOCK_DURABILITY.md` | `2edfc588fd8d7e96753f6beb4bb7c2ad50a064e4c266681ab5ff94550c4342ab` |
| `.codex-tmp/dur03_3/DESIGN_EVIDENCE_FREEZE.json` | `3710c57265084f9a6cd59d48fc77b9340fcd255bd0f235545c7ca639b92de92f` |
| Runtime-Review `DUR-03_3_PLAYER_BLOCK_DURABILITY_RUNTIME_REVIEW.md` | `8adcc9ee4af273793d631b6fbd2bb123a0dcda732f7f855a753f553c83295069` |
| `.codex-tmp/dur03_3-runtime-root-review/final-assessment.json` | `8f6dba9314f1765b3567c83899faf5e21168eb3e6b1b204561814b7c85617cac` |
| Ausgangskandidat Runde 3 (Manifest, 89 Dateien) | `49668fdfecd87c8344d8deebd290156251bd38b8501dccc09cfab95e66d77552` |
| Delivery Runde 3 (vor dieser Aktualisierung) | `e9b9af47f77c475ac26fab1bc81915c804d01db2b5b15716918f9c763f736166` |

Der Arbeitsbaum `server/crates/ferrumc-storage/**` war zu Beginn byte-identisch mit
`.codex-tmp/dur03_3-runtime/round3-candidate-src/` (unverändert gelassen).
Prüfbasis ist jetzt main `75f9824a8190c5ce21ef29bacb537a9f92794327`: private Kopie per
`git archive` (nur lesend) in `.codex-tmp/dur03_3-runtime/round4-isolated/`,
1277 Dateien, Basis-Freeze `round4-base.freeze.json` (Digest
`a803e84503e3e72d6c0d37bbdaade5b134eaef8c49e18960c23e16de97f795f3`), darauf das
vollständige Storage-Overlay (`round4-tools.py overlay`). Gebaut wurde nur dort,
nie im gemeinsamen Arbeitsbaum. Toolchain `rustc 1.99.0 (b940084d7 2026-09-28)`,
`cargo 1.99.0`, `--jobs 2 --locked`, keine Dependency-/Lockfile-Änderung.

### Kandidat und Freeze

Vor den finalen Checks eingefroren (2026-10-11T20:03:10+02:00):
`ferrumc-storage`, 89 Dateien, Manifest-SHA-256
`616a42792bfc1e5ac4e8cf1dedb40bc3e8e78f21775e3aba5c5d47230b4a42d4`
(`round4-candidate-manifest.json`, Dateihash
`4e665f47199b45578971ef0b1471ad1209eedebd832589e220a87966d2285387`; Schema wie
Runde 3: `pfad\tbytes\tsha256\n`). Schreibgeschützte Kopie:
`round4-candidate-src/`. Das Manifest wurde vor und nach allen Checks neu
berechnet und ist identisch (`round4-final-manifest-{before,after}.json`,
`round4-fresh-manifest-{before,after}.json`); der Arbeitsbaum ist byte-identisch.
Gegenüber Runde 3 geändert (keine Datei neu oder entfernt): `INVARIANTS.md`,
`README.md`, `src/block_durability.rs`, `src/block_durability/engine.rs`,
`src/ownership/engine.rs`, `src/redb_store/blocks.rs`,
`src/redb_store/block_validation.rs`, `src/memory/blocks/tests.rs`,
`src/redb_store/blocks/tests.rs`, `tests/player_block_bounds.rs`,
`tests/player_block_matrix.rs`.

### A1 — Lesepfad prüft genau die berührte Historie

Ursache: `inspect_chunk` rief pro Spalte `receipt_relations` auf, also den
vollständigen Endpunktbeweis **aller** bis zu 8 Einträge des letzten Receipts;
nur gleiche Receipt-IDs wurden dedupliziert.

Umsetzung (`block_durability/engine.rs`):

- Der Endpunktbeweis ist in `union` (Union/Abdeckung/Effektgültigkeit, ohne
  Zellarbeit) und `column` (Beweis eines Eintrags, liefert dessen aufgelöstes
  Postimage) zerlegt. `endpoints` (neue Requests) = `union` + alle `column` –
  Verhalten und Fehlerreihenfolge unverändert.
- `validated`: Relationen eines Receipts ohne Endpunktbeweis (Identität, Union,
  Outcome-Stamps, Player-Header/-Postimage, Chunk-Header/-Paare aller Einträge,
  exakte Journalzeilen). Höchstens einmal pro Aufruf (Cache `validated`, über alle
  Spalten dedupliziert).
- `inspect_chunk` (Snapshot, `chunk_state` in Prepare, Legacy-Leser): Prüfung der
  eigenen Spalte im letzten Receipt wie bisher (Index, Identität, Stamp, After-Paar),
  dann `validated` (einmal je Receipt) und der Endpunktbeweis **nur des eigenen
  Eintrags** (Cache `(id, index)`). Dessen aufgelöstes Postimage ist genau die
  Komposition des gespeicherten Paars und wird im Snapshot als View
  wiederverwendet (keine zweite Auflösung).
- `player_last` (Player-Lesen, v1) prüft nur noch die Relationen des letzten FCBR
  des Players (`receipt_history`), keinen Chunk-Endpunktbeweis.
- Voller Beweis ganzer Receipts bleibt bei `load_block_receipt`, beim Replay in
  Prepare, bei der v1-Lookup eines FCBR und beim Reopen. Beim Reopen ruft
  `inspect_chunk` wie bisher den vollen Beweis (Cache `proved`, 128 Einträge);
  die Startup-Kosten (H2) sind damit unverändert.
- Die harte Zellgrenze 16.777.216 und alle Budgets sind unverändert.

Rechnung (Zellmodell aus dem Code, in `INVARIANTS.md` dokumentiert; C = 98.304
Zellen einer Spalte): Paar auflösen = C (Basis) + mit Overlay 2C (Decode, Apply)
+ 2 je ersetzter Basis-Entity, max. 3C + 8.192 = 303.104. Gameplay-Spaltenbeweis
= 2 Paare + 2/Guard + 4/Effekt + 2C Vergleich + 1/Entity je Seite, unter den harten
Grenzen (4096 Entities/Guards, 1024 Effekte) max. 2·303.104 + 8.192 + 4.096 +
196.608 + 8.192 = **823.296**. View-Größe = C + 1/Entity ≤ 102.400.
Snapshot-Spalte ≤ 925.696, also **8-Spalten-Snapshot ≤ 7.405.568**, Commit
(8 Historien- + 8 Request-Spaltenbeweise) ≤ **13.172.736** – beides unabhängig
davon, welches Receipt jede Spalte zuletzt geändert hat, und unter 16.777.216.
Vorher kostete eine Spalte mit 8-Spalten-Historie bis zu 8 Spaltenbeweise
(Probe 4: ≈ 3,54 M je Spalte, ab 5 Spalten verweigert). Die Rechnung steht als
`const`-Assertion in `tests/player_block_bounds.rs`.

Tests (Memory und redb):

- `max_union_with_distinct_histories_snapshots_and_commits_under_hard_cell_cap`
  (Probe 4): acht 8-Spalten-Overlay-Image-CAS, danach Snapshots über 1..8 Spalten
  mit verschiedener Historie und über 8 Spalten gemeinsamer Historie, dann ein
  Scoped-Gameplay-Commit über die volle 8-Spalten-Union (ein Effekt je Spalte);
  Zellen, Revision 2, Receipt und Nachbarspalten geprüft, redb auch nach Reopen.
- `distinct_history_union_cell_work_matches_the_independent_model_exactly`:
  N/N−1 an der Zellgrenze mit unabhängig berechnetem Sollwert (nicht aus
  gemeldeten Bedarfswerten): Snapshot 7 bzw. 8 Spalten = n·491.520
  (Historienbeweis 4C + View C) – Grenze N läuft, N−1 →
  `ValidationCapacity("cells")`; Commit über die volle Union = 7.864.352
  (8·(4C + 3C + C + 4 + 2C)) – N committet, N−1 verweigert ohne Receipt.
- `reads_prove_touched_history_and_refuse_forged_or_inconsistent_history`
  (Unit, Memory und redb, Zeilen unter offenem Store umgeschrieben): (a) Effekt
  15,64 in FCBR-Q und Journalzeile konsistent gefälscht, Digest neu berechnet –
  berührte Spalte → `IncompleteDelta{key(0)}`, Snapshot über beide Spalten und
  `load_block_receipt` ebenfalls verweigert; die unberührte Spalte und der Player
  lesen weiter (genau die vertragsgemäße Semantik „encountered history“). (b) nur
  die Journalzeile verändert – jede Lesart, die dieses Receipt berührt (Spalte 0,
  Spalte 1, Player, Lookup), → `MalformedData{FCBR, journal exact ordinal
  disagreement}`. redb: Reopen verweigert beide Fälschungen.
- Unverändert grün: alle bisherigen Endpunkt-, Korruptions- und Reopen-Tests.

### A2 — Fresh create_new ist atomar

- `open_with_player_block_durability` legt **zuerst** den Scratch-Index an
  (`Index::create`, gleiche Grenze wie bisher) – das prüft Verzeichnis, Symlink,
  Rechte und Extent-Budget real – und erst danach wird der Datenbankpfad
  angefasst. Der Index wird in `initialize`/`scan` auf jedem Pfad mit typisiertem
  Cleanup beendet. Die Konfiguration ist bereits durch ihre geprüften
  Konstruktoren validiert; ein leerer Fresh-Scan hat keine weitere
  konfigurationsabhängige Verweigerung.
- Neue Variante `BlockDurabilityError::ScratchUnavailable(String)` (Enum ist
  `non_exhaustive`); die Meldung nennt Operation, Pfad und OS-Ursache ohne
  „UUID migration“. `ScratchCleanup` ebenso ohne diesen Wortlaut.
- Scheitert die Initialisierung trotzdem (z. B. IO), entfernt `abandon_fresh`
  die Datei nur, wenn (1) sie in diesem Aufruf per `create_new` entstand und der
  Pfad noch dasselbe Inode benennt (`remove_owned_path`, Symlink-/Ersatzschutz),
  und (2) über das noch offene Handle nachweislich keine einzige Tabelle
  existiert bzw. – falls redb die Datei nie initialisierte – sie 0 Bytes hat.
  Sonst bleibt sie liegen; zurück kommt immer der ursprüngliche typisierte Fehler.
- Existierende Dateien werden nie gelöscht. Eine tabellenlose redb-Datei (Rest
  einer älteren Version) → `CatalogMismatch("empty redb file without any FerrumC
  table (likely left by an interrupted fresh create); nothing was committed:
  verify, move it aside and open again")`.

Tests: `refused_fresh_open_leaves_nothing_and_the_corrected_retry_opens`
(Probe 3: fehlendes Scratch und Scratch als Datei → `ScratchUnavailable` ohne
„UUID“, keine Datei; zu kleines Scratch-Extent → `ScratchCapacity`, keine
Datei; danach korrekter Open, Initialisierung, Schreiben, Reopen; tabellenlose
und fremde Datei → verweigert, Bytes unverändert);
`abandoned_fresh_file_is_removed_only_with_identity_and_empty_catalog` (Unit:
leer → entfernt; mit Tabelle, ohne Identität, ersetzter Pfad, nicht-leere Datei
ohne Handle → bleibt; 0-Byte ohne Handle → entfernt).

### Hinweise des Reviews

| Hinweis | Umsetzung |
| --- | --- |
| H1 Fehlerreihenfolge v1 in Format7 | `ownership::engine::admit_shared_receipt_bytes` prüft die gemeinsame Receipt-Byte-Grenze direkt nach den Receipt-Zeilen und vor der Player/Entity-Historie (nur mit Format7-Konfiguration; Format6 unverändert). Test `v1_receipt_bytes_refuse_before_player_history` (beide Grenzen erschöpft → `HistoryCapacity{"receipt bytes"}`, Memory und redb). Die spätere Prüfung im Writer bleibt als Verteidigung bestehen. |
| H2 Startup-Aufwand | Nicht geändert, bewusst: Reopen muss jeden FCBR voll neu ableiten (Vertrag Z. 330ff.); Verhalten und Cache wie Runde 3. |
| H3 Race-Verlierer | Transfer-/Autosave-Race: Zellen, Chest-Entity, Spalten-Stamps, Joint-Receipt und bei Transfer die Entity (fehlt/Fresh, wenn Joint gewinnt). Verschiedene Player: Zellen beider Seiten und beide Receipts. Überlappende Unions: genau ein Inkrement, Revision +1, geguardete Spalte unverändert, genau das Gewinner-Receipt. |
| H4 Selbstreferenzielle Grenzen | Zellgrenzen jetzt mit unabhängigem Modell (A1-Tests); neu `memory_snapshot_output_and_cells_match_independent_sizes_exactly`: Snapshot-Ausgabe aus Player-Envelope-Länge und den nicht-leeren Sections des World-Generators (35 + 55 + 2·Player + 2·(47 + 49 + 16.384·Sections)), N läuft, N−1 → `ByteCapacity{len=N}`; Zellen 4C, N/N−1. Nicht umgesetzt: Scratch-Lebenszyklusgrenze – sie hängt vom redb-Dateilayout ab, das der Vertrag nicht festlegt; Sollwert bleibt die gemessene Grenze. Die redb-Snapshot-Grenze bleibt iterativ bestimmt. |
| H5 | Unverändert zulässig, keine Änderung. |
| H6 Claims-Scan | Der Rust-Fachbegriff für Trait-Objekt-Fähigkeit aus Vertrag Z. 58 steht nur in INVARIANTS und Rustdoc, außerhalb des CI-Scanumfangs; keine Änderung. Diese Delivery: 0 Treffer. |
| H7 | Begründungskommentar am `too_many_arguments`-`allow` ergänzt. |
| H8 Commit-Fault-Test | Nicht geändert: der Review hält den Reopen-Nachweis für vertretbar; der Test prüft beim Fall „Receipt vorhanden“ weiter über die vollständige Reopen-Validierung. |
| H9 Delivery-Satz | Ersetzt, siehe offene Punkte. |

### Finale Checks (eingefrorener Kandidat, ohne Source-Änderung)

`round4-final-checks.sh` in `round4-isolated` (main 75f9824a + Overlay), kein
Overlay-Sync während der Checks:

| Check | Ergebnis | Log |
| --- | --- | --- |
| `cargo test -p ferrumc-storage --jobs 2 --locked --no-fail-fast` (Debug) | exit 0, **342/342** in 16 Binaries (Lib 252), 0 failed, 0 ignored, 704 s | `round4-final-debug-tests.log` |
| dito `--release` | exit 0, **342/342** in 16 Binaries, 354 s | `round4-final-release-tests.log` |
| `cargo clippy --workspace --all-targets --jobs 2 --locked -- -D warnings` | exit 0, 0 Befunde | `round4-final-clippy.log` |
| `cargo fmt --all --check` | exit 0 | `round4-final-fmt.log` |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-storage --no-deps --jobs 2 --locked` | exit 0 | `round4-final-rustdoc.log` |
| `cargo build --workspace --all-targets --jobs 2 --locked` | exit 0, 0 Warnungen, 256 s | `round4-final-build.log` |
| `cargo test -p ferrumc-sim -p ferrumc-app -p ferrumc-testkit --jobs 2 --locked --no-fail-fast` | exit 0, **844 bestanden, 0 fehlgeschlagen, 1 ignoriert** (App-Java-Kompatibilitätstest, braucht Java/Maven-JARs) in 58 Ergebnisblöcken | `round4-final-consumers.log` |
| Manifest vor/nach | identisch `616a4279…42d4`, isolierte Kopie identisch | `round4-final-summary.log` |
| Basis-Abgleich außerhalb Storage | alle 1277 Archivdateien unverändert; das Skript meldet `verify-copy=FAIL` nur wegen zwei vom App-Plugin-Testhost-Test erzeugter Artefakte (`.codex-tmp/p57-plugin-testhost-*/lib*.so`), keine Quelldatei | `round4-final-summary.log` |

Clippy und Rustdoc liefen im Final-Skript aus dem Cargo-Cache eines
quellidentischen Entwicklungslaufs (1 s). Deshalb zusätzlich
`round4-fresh-checks.sh`: nach `cargo clean -p ferrumc-storage` bzw.
`cargo clean --doc` frisch – clippy workspace all-targets exit 0 (Storage, Sim,
Testkit, Session, App, Bench, Plugins neu geprüft), rustdoc exit 0
(`round4-final-{clippy,rustdoc}-fresh.log`). Danach die **unveränderte**
Probe-Datei des Reviewers (SHA-256 `d74de1b9…4754453`) in einer separaten Kopie
`round4-probe/`: alle 4 Probes grün auf Memory und redb (Probe 3: erster Open
`ScratchUnavailable("resolve parent …/missing-scratch: No such file or
directory")`, keine Datei, Retry ok; Probe 4: 1..8 Spalten zugelassen; Probe 2:
Joint 6 Siege/10 Niederlagen) – `round4-final-probes.log`.

### Offene Punkte und Grenzen (Runde 4)

- **Offene Frage an Root (Bytes, nicht Zellen):** Die Zellgrenze ist für jede
  Union mit beliebiger Historie gesichert. Die harte Byte-Grenze von 512 MiB ist
  es nicht für maximal große Receipts: Jede gelesene Spalte zählt ihr letztes
  FCBR-Row beim Lesen und beim Hash/Kopieren (≈ 3·|FCBR| je Spalte, auch bei
  gemeinsamem Receipt; Relationen nur einmal). Eine 8-Spalten-Union, deren
  letzte Receipts im Mittel über ≈ 21 MiB groß sind (Request-Grenze bis 32 MiB),
  kann daher mit `ValidationCapacity("bytes")` verweigert werden. Das bestand
  schon in Runde 3 (dort mehr Zählung je Spalte). Für gemeinsame Receipts ließe
  es sich durch einen aufrufweiten Receipt-Cache auf ≈ 1·|FCBR| senken; bei 8
  verschiedenen 32-MiB-Receipts bleibt Lesen + Hashen arithmetisch an der
  512-MiB-Grenze. Entscheidung (Cache, Byte-Zählweise oder dokumentierte
  Einschränkung analog „maximum shape does not guarantee admission“) liegt bei Root.
- Laufzeit-Lesepfade prüfen nur die berührte Historie (vertragsgemäß); eine
  Fälschung in einer **unberührten** Spalte eines berührten Receipts, die alle
  Relationen erfüllt, erkennt erst Lookup/Replay/Reopen (Test belegt beides).
- Keine Aussagen zu RSS, Prozessabbruch, Stromausfall, Survival, Paper-Gleichheit
  oder App-Aktivierung. DUR-03.3a (Format6→7) bleibt separat.
- Autoritätsfragen: keine festgestellt.

### Evidenz (Runde 4)

Unter `.codex-tmp/dur03_3-runtime/`: `round4-candidate-manifest.json`,
`round4-candidate-frozen-at.txt`, `round4-candidate-src/` (schreibgeschützt),
`round4-base.freeze.json`, `round4-final-*.log`, `round4-final-summary.log`,
`round4-fresh-summary.log`, `round4-final-manifest-{before,after}.json`,
`round4-fresh-manifest-{before,after}.json`, Skripte `round4-tools.py`,
`round4-run.sh`, `round4-final-checks.sh`, `round4-fresh-checks.sh`;
Freeze aller Dateien mit SHA-256 und dem Hash dieser Delivery:
`round4-evidence-freeze.json`. Entwicklungslogs `round4-dev-*.log` (darunter der
rote Zwischenstand `round4-dev-focused1.log`: Memory-Setup überschritt bei n=1 das
eigene Testlimit, Test auf n ∈ {7, 8} korrigiert) bleiben erhalten. Die
Runde-3-Evidenz und `round3-candidate-src/` sind unverändert.

## Runde 3 — Grundlagen (vor Beginn neu geprüft)

| Grundlage | SHA-256 |
| --- | --- |
| Vertrag `DUR-03_3_PLAYER_BLOCK_DURABILITY.md` | `2edfc588fd8d7e96753f6beb4bb7c2ad50a064e4c266681ab5ff94550c4342ab` |
| `.codex-tmp/dur03_3/DESIGN_EVIDENCE_FREEZE.json` | `3710c57265084f9a6cd59d48fc77b9340fcd255bd0f235545c7ca639b92de92f` |
| d3fca66c-Archivmanifest `base.freeze.json` (1224 Dateien) | `c148936b29cc58d7ac25b2904618523c3b3a163476a2bf380b1d00fadf07c7b0` |

Alle Builds/Tests liefen in der privaten Kopie
`.codex-tmp/dur03_3-runtime/isolated` (echtes d3fca66c-Archiv + vollständiges
Storage-Overlay via `overlay.py`). `verify_copy.py` bestätigt: alle 1224
Archivdateien unverändert, alle Dateien außerhalb von `ferrumc-storage`
byte-identisch, keine Zusatzdateien. Toolchain 1.99.0 (`rust-toolchain.toml`),
`--jobs 2 --locked`, keine Dependency-/Cargo.lock-Änderung. Arbeitsbasis des
Repos war main `e22b3338`; Storage-Produktion/API dort unverändert gegenüber
d3fca66c laut Root.

## Runde 3 — Kandidat und Freeze (abgelöst durch Runde 4)

Kandidat (vor den finalen Checks eingefroren, 2026-10-11T18:38:50+02:00):
`ferrumc-storage`, 89 Dateien, Manifest-SHA-256
`49668fdfecd87c8344d8deebd290156251bd38b8501dccc09cfab95e66d77552` (`.codex-tmp/dur03_3-runtime/round3-candidate-manifest.json`).
Das Manifest wurde vor und nach allen Checks neu berechnet und ist identisch
(`round3-final-manifest-before.json` = `round3-final-manifest-after.json` = Kandidat; isolierte Kopie byte-identisch). Dieses Dokument selbst liegt außerhalb des Kandidaten.

## Umfang der Implementierung

Nur `server/crates/ferrumc-storage/**`. Neue Module: `block_durability`
(`mod.rs`-Datei, `raw.rs` Grammatiken, `engine.rs` Plan/Endpunktbeweis/Relationen),
`memory/blocks.rs`, `redb_store/blocks.rs` (Writer/Adapter),
`redb_store/block_validation.rs` (Fresh/Reopen-Scan, Scratch-Index). Angepasst:
`error.rs`, `generation.rs`, `lib.rs`, `memory.rs`, `memory/ownership.rs`,
`memory/uuid_entities.rs`, `ownership.rs`, `ownership/engine.rs`,
`redb_store.rs`, `redb_store/ownership.rs`, `redb_store/ownership_validation.rs`,
`redb_store/uuid_entities.rs`, `store.rs`, `INVARIANTS.md`, `README.md`.

Öffentliche Ergänzungen: `PlayerBlockDurabilityStore` (fünf Methoden, Default
`Unsupported`), `InMemoryStore::with_player_block_durability`,
`RedbStore::open_with_player_block_durability(path, config, scratch)`,
`BlockDurabilityConfig/Capacity/Limits`, `BlockSnapshotSpec`, `OwnedBlockSnapshot`,
`OwnedBlockColumn`, `ChunkImage`, `ChunkImageAction`, `ChunkImageChange`,
`ScopedBlockEffect`, `ScopedBlockCause`, `PlayerBlockRequest`,
`BlockMutationRequest`, `ChunkImageCasRequest`, `BlockReceiptQuery`,
`BlockReceipt`, `BlockCommitOutcome`, `BlockDurabilityError`, `BlockResult`,
`MAX_BLOCK_*`; additiv `StorageError::BlockValidation`,
`OwnershipError::{ReceiptKindMismatch, Block}` (beide Enums `non_exhaustive`).
Keine öffentlichen Felder, Konstruktoren für Erwartungen/Snapshots/Receipts oder
Serialisierung von Autorität.

Vertragspunkte, wie umgesetzt:

- Format7 mit genau 17 Tabellen und 15 Pflicht-Metaschlüsseln (+ optional
  `last_mutation_id`, `last_scoped_mutation_id`); Format6 und FCTR1 bleiben
  unverändert. Fresh in einer Immediate-Transaktion; Existing7 validiert ohne
  logischen Schreibvorgang; Existing3..6 → `MigrationRequired` (auch leer);
  verweigerte Opens lassen Tabellen/Zeilen/Metadaten unverändert.
- Autorität nur über `OwnedBlockSnapshot` aus einer echten Lese-Grenze derselben
  Backend-Inkarnation (privates `Weak`); fremde/reopened Backends →
  `BackendOwnerMismatch`, abweichende Generation → `GenerationMismatch`.
- FCBQ/FCOB/FCBL/FCBR exakt nach Vertrag; Q-Größenformel und harte
  Byte-Admission vor Encoding/Hash; Replay vergleicht das retainte Q bytegenau.
- Gemeinsame Player-Revision und neue Chunk-Historie; Joint-, Scoped- und
  Image-CAS-Writer nutzen dieselbe Historie; alle blinden Block-/Player-Writer
  verweigern vor Validierung (`Unsupported`, auch leere Batches).
- Endpunktbeweis: Effektkette auf dem aufgelösten Original, Vergleich aller
  98.304 Zellen und der kanonischen Entity-Map je Chunk; Union = Guards ∪ Effekte
  ∪ Replace/Erase; Auflösung full/overlay/flat-Generator nach Vertrag.
- FCTR/FCBR teilen Namespace, Worker-Admission und Receipt-Zeilen/-Bytes (FCTR 272,
  FCBR/FCBL exakte Key+Value-Bytes); Kollisionsrichtung in beide Richtungen typisiert.
- Immediate-Commit; `CommitUncertain`/`ExecutionUncertain` mit Original-Query;
  Memory: Preflight + ein Preapply-Hook, danach infallibles Apply unter einem Lock.
- Reopen-Scan aller 17 Tabellen mit Raw-Row/Byte- und separatem Relations-Budget,
  beidseitige Relationen, Bereichs-Nichtüberlappung über einen begrenzten
  Disk-Index mit Identitäts-Cleanup.

## Änderungen in dieser Runde (Runde 3) gegenüber dem übernommenen WIP

1. `block_validation/tests.rs` kompilierte nicht (falsche Konstantennamen,
   fehlende redb-Traits, Namenskonflikt `initialize`) → behoben.
2. Scratch-Index-Test nahm die Dateilänge nach dem Anlegen als Admissionsgrenze an;
   redb schreibt beim Anlegen und beim Abort transient darüber hinaus. Test sucht
   jetzt die echte Lebenszyklusgrenze (create+abort+cleanup): N läuft, N−1
   → `ScratchCapacity`, jeweils ohne verbleibende Datei. Produktion unverändert.
3. Shapes-Test erzeugte versehentlich ein Overlay ohne Sections
   (`Chunk::set_block` markiert keine persist-edited Section) → Test nutzt
   `mark_persist_dirty`, prüft `section_count` explizit und die vollständige
   Section-Ersetzung; Implementierung war korrekt.
4. `ownership_adapter` fabrizierte bei `ReceiptCollision` Null-Digests; v1-Prepare
   setzte `stored` = eigener Digest. Jetzt: typisierter `Block`-Adapter bzw. echter
   validierter FCBR-Digest als `stored`.
5. Verschachtelte Adapterfehler (`Ownership(Block(..))`, `Record(BlockValidation(..))`)
   werden strukturell zu typisierten `BlockDurabilityError`-Varianten entpackt
   (manuelle `From`-Impls statt `#[from]`); `ServerError`-Abbildung klassifiziert
   `Ownership` über deren eigene Konvertierung.
6. Das WIP hatte `InMemoryStore` öffentlich `Clone` gemacht (Aliasing von Autorität);
   ersetzt durch crate-privates `share()`.
7. Pedantic-Clippy für den Workspace bereinigt (u. a. Referenzparameter,
   `match`, Typaliasse, Helfer `refuse_blind_block_writer`, ausgelagerte
   Format7-Freshness `blocks::state_is_empty`); verbleibende `allow`s nur mit
   Begründung nach bestehender Crate-Konvention (`too_many_lines`,
   `many_single_char_names`, `option_option`, `double_must_use`).
8. INVARIANTS.md/README.md um Format7 ergänzt.
9. Ergänzte Matrix-Tests (siehe unten, markiert mit ★).

## Testmatrix (tatsächliche Producer-Beobachtungen)

Öffentliche API (`tests/`, Memory und redb, unabhängige Erwartungen):

- `player_block_contract.rs`: feste FCPS3 (46 FCIS-Slots + carried, Position/Look,
  −0.0), FCIS mit typisiertem Schaden/opaken Bytes/Removal, Chest-Komponenten;
  redb: komplette 17-Tabellen-Zeilenmengen vor/nach (`before17.tsv`,
  `after17.tsv`) und `joint.fcbr` aus `make_fixtures.py` (Python-Assembler ohne
  FerrumC-Code/-Runtime), unabhängiger Feld-für-Feld-Decoder des FCBR/FCBQ/FCBL,
  Ganz-Delta inkl. unveränderter Tabellen; Reopen, alte Inkarnation →
  `BackendOwnerMismatch`, gleiche Q-Rekonstruktion, Replay nach Reopen;
  Entity-only-Delta, versteckte Zelle → `IncompleteDelta`.
- `player_block_matrix.rs`: Barrier-Races Joint vs v1-Autosave und vs
  Player/Entity-Transfer (ein Gewinner, Verlierer ohne Teilmutation);
  verschiedene Player/gleicher Chunk; disjunkte Unions beide; Guard-Drift →
  CAS; guard-only bewahrt Stamp (nachfolgender Nachbar-Write gültig);
  überlappende Multi-Chunk-Unions; Same-ID-Race committed einmal; Namespace-
  Kollisionen; fremde Autorität; andere Seed-Identität; Image-CAS invalidiert
  Pending-Joint; alle blinden Writer verweigern; 8/4096/1024-Grenzen;
  Fresh-Journalzyklus → Tombstone1; Erase auf Tombstone → `AlreadyAbsent`.
  ★ alle Feldvarianten einer ID (Original, Post, Komponente, Player, Schema,
  Tick, Cause, Effektreihenfolge, Kind 2/3, Scope) → `ReceiptCollision` ohne
  Schreiben, Replay danach, Reopen; ★ FCTR-ID über Block-Lookup →
  `ReceiptKindMismatch`; ★ veralteter Snapshot mit neuer ID → Player- bzw.
  Chunk-CAS, No-op invalidiert nicht; ★ Union: Extra-Keep-only, unbedeckter
  Guard/Effekt, falsche/doppelte Proposal-Keys, Player/Kind-Mismatch; ★ Schild
  (versioniert, gewachst, beide Seiten) und Entity-only-Textänderung mit exakten
  Journal-Bytes.
- `player_block_shapes.rs`: full/overlay/beide/keins, Section-Overlay ersetzt
  Basis-Entities und ganze Section, Null-Section-Overlay merged, leeres Overlay,
  Erase → Tombstone über Reopen; Chain-/Hidden-Cell-/Höhen-/Spalten-/
  Entity-Format-Verweigerungen.
- ★ `player_block_bounds.rs`: N/N+1 für Chunk-Historienzeilen, gemeinsame
  Receipt-Zeilen, gemeinsame Receipt-Bytes (Grenze aus den festen Fixture-Längen
  272+16+|seed.fcbr|+16+|joint.fcbr|, inkl. v1-No-op), Journal-Zeilen,
  Journal-Bytes (aus `after17.tsv`), persistierte Request-Grenze (|joint.fcbq|);
  unveränderliche Vereinbarung → `CapacityContractMismatch` ohne Zeilenänderung;
  Konfigurationsüberläufe; Laufzeit-Zell-/Byte-Budget → `ValidationCapacity`;
  Snapshot-Ausgabe an exakter Grenze; Reopen-Scan Zeilen/Bytes N/N+1 (unabhängig
  gezählt), Relations-Bytes/-Zellen, Scratch-Extent, alle ohne Schreibvorgang
  und ohne verbleibende Scratch-Datei; alle fünf Methoden `Unsupported` in
  Legacy/Metadata/UUID/Format6 (Memory und redb), Format3..6 →
  `MigrationRequired{n}` ohne Änderung, alle Format3..6-Opener verweigern
  Format7 ohne Änderung, Blind-Writer → `ServerError::Unsupported`.

Unit (`src/**/tests.rs`): alle abgeschnittenen FCBQ/FCBR/FCOB/FCBL-Präfixe,
Magic/Kind/Längen/Inaktiv/Reserviert/Digest/Range/Trailing; Endpunktbeweis und
Kettenfehler; Sequenz-Allocator an u64::MAX; jeder echte redb-Writer-Cutpoint
(38) und BeforeCommit → alle 17 Tabellen alt, auch nach Reopen; Pair-Remove/
Overlay-Insert-Abbrüche; AfterCommit-Verlust und Worker-Panic → Replay nach
Reopen; echter Commit-Write- und fsync-Fehler über `StorageBackend` →
`CommitUncertain`, nach exklusivem Reopen ganz alt oder ganz neu (beides
zugelassen, kein Abbruchschluss); Cancel hält Permit, Busy startet keinen Worker;
Memory: Preapply-Fehler alt, Postapply-Verlust exakter Receipt, Leser sieht keinen
Mid-Apply-Zustand; 45 Korruptionsfälle beim Reopen ohne Reparatur und ohne
Fingerprint-Änderung; >128 retainte Receipts reopen; ★ echte Revision-MAX für
Player und Chunk (vom Reopen-Validator akzeptiert): exakter No-op bewahrt Stamps
und Last-Pointer und verbraucht Receipt-Bytes, echte Änderung →
`RevisionExhausted` ohne Zeilenänderung.

## Runde 3 — Finale Checks (abgelöst durch Runde 4)

Alle im isolierten d3fca66c-Archiv mit vollständigem Storage-Overlay, ohne
Overlay-Sync während der Checks (`final_checks.sh`):

| Check | Ergebnis | Log |
| --- | --- | --- |
| `cargo test -p ferrumc-storage --jobs 2 --locked --no-fail-fast` (Debug) | grün: 334 Tests in 16 Binaries (Lib 249, Integration, Doctests), 0 Fehler | `round3-final-debug-tests.log` |
| dito `--release` | grün: 334/334, 0 Fehler | `round3-final-release-tests.log` |
| `cargo clippy --workspace --all-targets --jobs 2 --locked -- -D warnings` | grün, 0 Befunde | `round3-final-clippy.log` |
| `cargo fmt --all --check` (Workspace) | grün | `round3-final-fmt.log` |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-storage --no-deps --jobs 2 --locked` | grün | `round3-final-rustdoc.log` |
| `verify_copy.py` nach den Checks | 1224 Archivdateien unverändert, außerhalb Storage byte-identisch, keine Zusatzdateien | `round3-final-summary.log` |

Rustdoc wurde für das geänderte Crate geprüft; Workspace-weite Strict-Checks
gegen aktuelles main bleiben laut Brief Root-Aufgabe.

## Grenzen, offene Punkte, keine stillen Vertragsänderungen

- „Türhälften“ sind zwei übereinanderliegende Zellen eines Chunks mit frei
  gewählten State-IDs (Storage interpretiert States nicht; keine Registry-
  Dependency). Echte Tür-States und Gameplay-Abhängigkeiten belegt erst Sim.
- Erhalt lesbarer Legacy-Entity-Schreibweisen bei Keep ist implementiert
  (Rohbytes), aber nicht laufzeitgetestet: in Fresh-Format7 entstehen sie nur
  über die separate Migration DUR-03.3a.
- Der MAX-Test läuft auf redb; Memory teilt denselben `engine::prepare`/
  `after_stamps`-Pfad, hat aber keinen eigenen MAX-Fixture-Test.
- (Runde 3; für Memory in Runde 4 mit unabhängigem Sollwert ergänzt) Die exakte Snapshot-Ausgabegrenze wird aus den vom Backend gemeldeten
  Bedarfswerten iterativ ermittelt (Exaktheit/Konsistenz, keine unabhängige
  Sollgröße). Alle übrigen Byte-Grenzen stammen aus den festen Fixtures bzw. aus
  unabhängigem Rohzählen.
- Ressourcenaussagen sind logische Admissions-Grenzen; RSS, redb-Allokation und
  Cachegrößen wurden nicht gemessen.
- Prozessabbruch/Stromausfall, Live-Paper/Client-Vergleich, App/Sim-Aktivierung
  und die Offline-Migration Format6→7 (DUR-03.3a, vor DUR-05) sind nicht Teil
  dieser Lieferung.
- (Runde 3) Keine ungelösten Autoritäts- oder Vertragsfragen festgestellt – in
  Runde 4 ersetzt: A1 ist behoben; offen ist die Byte-Frage oben.

## Evidenz

Logs unter `.codex-tmp/dur03_3-runtime/`: `round3-final-*.log`,
`round3-final-summary.log`, Manifeste `round3-candidate-manifest.json`,
`round3-final-manifest-{before,after}.json`. Fehlgeschlagene Entwicklungslogs
früherer Runden und dieser Runde (`round2-*.log`,
`round3-initial-audit-test.path-failure.log`, `round3-audit-test.log`,
`round3-focused-*.log`, `round3-clippy-pre*.log`) bleiben unverändert erhalten.
