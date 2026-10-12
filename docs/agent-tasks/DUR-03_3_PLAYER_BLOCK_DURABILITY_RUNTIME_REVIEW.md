# DUR-03.3 — unabhängiger Runtime-Review (Storage Format7)

Stand: 11.10.2026. **Verdikt: FINDINGS.** Kein blockierender Befund, aber zwei
Auflagen, die meine eigenen Probes auf Memory und redb belegen. Dazu kommen
Hinweise. Ob integriert wird, entscheidet Root. Dieser Review ist keine Aussage
zu App-Aktivierung, Survival, Paper-Gleichheit, RSS, Prozessabbruch oder Stromausfall.

Ich habe den Kandidaten nicht erstellt. Alle Builds und Tests liefen in privaten
Kopien unter `.codex-tmp/dur03_3-runtime-root-review/`. Am Kandidaten, an der
Delivery, an der Evidenz und an der Produktion habe ich nichts geändert. Git habe
ich nur lesend benutzt (`git diff`/`git show` gegen `75f9824a`, `d3fca66c`), um die
geänderten Dateien und die main-Baseline für den Claims-Scan zu ermitteln.

## Prüfbasis und Integrität

| Objekt | Ergebnis |
| --- | --- |
| `combined/` gegen `combined.freeze.json` (`450eb14f…3e47c40f`) | vor und nach allen Läufen 1305/1305 Dateien identisch, keine fehlt, keine zusätzlich |
| Kandidat im Arbeitsbaum gegen `round3-candidate-manifest.json` | 89/89 Dateien byte-identisch, keine Zusatzdatei im Storage-Crate. Manifest-Digest `49668fdf…6d77552` nachgerechnet (Schema aus `manifest.py`: `pfad\tbytes\tsha256\n`) |
| Delivery | SHA-256 `e9b9af47…1f736166` ✓ |
| Evidence-Freeze | SHA-256 `234109eb…34117f4117` ✓. Alle 16 darin gelisteten Evidenzdateien hashgleich |
| Vertrag / Design-Freeze | `2edfc588…4c342ab` ✓ / `3710c572…ca92de92f` ✓ |
| Storage, `Cargo.toml`, `Cargo.lock` | zwischen `d3fca66c` und main `75f9824a` unverändert (lesendes `git diff --quiet`) |

## Teil A — Ausführung (frisch, Kombination main `75f9824a` + 89 Kandidatendateien)

Toolchain `rustc 1.99.0 (b940084d7 2026-09-28)`, `--jobs 2 --locked`, private
Kopie `work/`, eigenes Target dort. Logs liegen unter `.codex-tmp/dur03_3-runtime-root-review/logs/`.

| Check | Ergebnis |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo test -p ferrumc-storage --no-fail-fast` (Debug) | exit 0, **334/334** in 16 Binaries (Lib 249, Doctests 12), 0 failed, 0 ignored, 822 s |
| dito `--release` | exit 0, **334/334** in 16 Binaries (Lib 249), 355 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, 0 Warnungen, 75 s |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-storage --no-deps` | exit 0, 0 Warnungen |
| `cargo build --workspace --all-targets` | exit 0, 0 Warnungen, 294 s |
| Verbraucher `cargo test -p ferrumc-sim -p ferrumc-app -p ferrumc-testkit -p ferrumc-bench --no-fail-fast` | exit 0, **849 bestanden, 0 fehlgeschlagen, 1 ignoriert** (`java_compat`, braucht Java) in 62 Ergebnisblöcken; Sim-Lib 237, App-Lib 287, Testkit 46 |
| Eigene Probes (Teil C), private Kopie `probe/` | 2 bestanden, 2 rot (= Befunde A1, A2) |
| Fixture-Handprüfung (`fixture_handcheck.py`) | 47/47 Erwartungen stimmen mit der Vertragsgrammatik |
| Forbidden-Claims-Scan (Funktionen aus `scripts/check-forbidden-claims.py`) | Delivery 0 Treffer. Im CI-Umfang (`README.md`, `docs/**`) 0 Treffer. Siehe H6 |

Die Testzahlen der Delivery (334/334 Debug und Release, Lib 249, 16 Binaries)
habe ich auf dem aktuellen main reproduziert.

## Teil B — Source-Review gegen den Vertrag

### Bestätigt (ohne Befund)

- **Format7 / Format6:** Fresh legt genau 17 Tabellen und 15 Pflicht-Metaschlüssel
  mit Zählern 0 in einer Immediate-Transaktion an (`redb_store/block_validation.rs:184-219`).
  Existing7 validiert in einem Writer ohne Commit. Existing3..6 liefert
  `MigrationRequired`, ein unbekannter oder fehlender Marker `CatalogMismatch`
  (`:220-244`). Format6-Pfade verzweigen nur bei `block_config.is_some()`
  (`redb_store/ownership.rs` Marker `7 : 6`, `ownership/engine.rs` FCBR-Dispatch nur
  mit Block-Config, Memory-Reader-Budget nur mit Block-Config). FCTR1 bleibt 256
  Bytes; der Format6-Parser ist unverändert.
- **Grammatik:** FCBQ 98+46n+12g+64e+Payload, FCOB 31, FCBL 100+Payload, FCBR
  71+|Q|+9n (Footprint 87+…) stimmen in Encoder und Parser (`block_durability/raw.rs`).
  Die harte Byte-Admission läuft geprüft und inkrementell vor Allokation und Hash
  (`raw.rs:116-163`). Der Replay-Vergleich prüft das retainte Q bytegenau
  (`engine.rs:759`).
- **Backend-ausgestellte Autorität:** `OwnedBlockSnapshot` trägt ein privates
  `Weak<()>` der echten Inkarnation. Es gibt keine öffentlichen Konstruktoren und
  keine öffentlichen Felder. `Clone` teilt die Autorität (vom Vertrag erlaubt).
  `BlockReceiptQuery::from_bytes` ist nur lesend. Die Prüfung prepare →
  `Weak::ptr_eq` und `upgrade` läuft nach der ID-Prüfung (`engine.rs:764-768`), wie
  im Vertrag: Ein committeter alter Request wird replayed, ein fremder ohne Receipt
  verweigert. Memory-`share()` ist crate-privat, ein öffentliches `Clone` gibt es nicht.
- **Kohärenter Snapshot:** Player plus 1..8 Chunks (full, overlay, Entity-Map)
  stammen aus einer redb-Read-Transaktion bzw. aus einem Lock-Block
  (Boundary → Generation → State) (`engine.rs:585-659`, `memory/blocks.rs:153-190`).
- **Gemeinsame Player-Revision, neue Chunk-Historie:** Der Joint-Writer schreibt
  über dieselben Player-Zeilen und denselben Header. v1-CAS in Format7 läuft über
  `commit_v1` mit derselben Receipt-Tabelle und denselben Bytezählern. Der
  Chunk-Header FCOB folgt dem Paar bzw. dem Journal-Effekt (`engine.rs:341-377`).
  Guard-only bewahrt den Stamp.
- **FCTR/FCBR-Namespace:** In beide Richtungen `ReceiptCollision`
  (`engine.rs:751-763`, `ownership/engine.rs` prepare). Eine v1-Lookup auf FCBR
  liefert `ReceiptKindMismatch`, eine Block-Lookup auf FCTR ebenfalls. Der
  validierte FCBR-Digest wird als `stored` gemeldet (Korrektur Runde 3 korrekt).
- **Blinde Writer:** Alle WorldStore-Full/Overlay/Delete/Journal-Writer sowie
  PlayerStore und Wire-Entity verweigern als erste Anweisung, also auch bei leerem
  oder zu großem Batch (`memory.rs`, `redb_store.rs` `refuse_blind_block_writer`).
  Lesepfade laufen über den vereinigten State bzw. denselben Read-Txn.
- **98.304 Zellen / Entity-Endpoint:** Die Kette läuft auf dem aufgelösten
  Original. Jeder alte State und jede alte Entity werden gegen die Vorzelle geprüft,
  danach werden alle 384×16×16 Zellen und die kanonischen Entity-Bytes verglichen
  (`engine.rs:260-318`). Ein unveränderter Effekt wird verweigert. Die Union ist
  gleich den Snapshot-Keys.
- **Commit:** `Durability::Immediate`. Die Reihenfolge Player → sortierte Paare →
  Header → Journal → FCBR → Meta → Watermark stimmt. Ein Hook-Fehler vor dem Commit
  verwirft die ganze Transaktion. Ein Commit-IO-Fehler ergibt `CommitUncertain`,
  ein After-Commit- oder Join-Fehler `ExecutionUncertain` (`redb_store/blocks.rs:278-322`).
  In Memory gibt es Preflight, einen Preapply-Hook und danach ein infallibles Apply
  unter einem Lock (`memory/blocks.rs:293-317`).
- **Reopen:** Raw-Zeilen und Bytes werden vor dem Parsen zugelassen. Das
  Relationsbudget ist separat. Jeder FCBR wird neu abgeleitet. Journal und Ordinal
  werden in beide Richtungen geprüft. Die Nicht-Überlappung prüft ein begrenzter
  Disk-Index mit Cleanup, der auch bei Fehler läuft (`block_validation.rs:275-540`).
- **Verbotsmuster:** In den 6 neuen Produktionsmodulen kein `unwrap`/`expect`,
  kein `unsafe` (Crate `forbid(unsafe_code)`), keine Channels. Alle `as u8/u16/u32`
  sind durch vorherige Shape- oder Byte-Grenzen gedeckt. Allokationen über
  Caller-Daten passieren erst nach `shape`/`add` (Request) bzw. nach dem
  Budget-Charge (Receipt-Parse `raw.rs:538`, Reopen `RawBudget`).

### Auflagen

**A1 — Die harte Zellgrenze verhindert 8-Chunk-Unions mit unterschiedlicher Historie.**
`block_durability/engine.rs:502-549` (`inspect_chunk` → `receipt_relations` bei `:547`),
`:380-391` (`receipt_relations` führt bei jedem Lesezugriff den vollständigen
Endpunktbeweis **aller** Einträge des letzten Receipts erneut aus), Reservierungen
`:138/:158/:176`; dieselbe Funktion wird von `snapshot` (`:628`) und `prepare`
(`:794`) aufgerufen.
*Fehlerszenario:* Acht gewöhnliche Overlay-only-Image-CAS-Writes zu je 8 Chunks
(die normale Autosave-Form). Danach ein Snapshot über je einen Chunk aus jeder
Gruppe: Jede Spalte kostet ≈ 8×393.216 (Receipt-Revalidierung) + 393.216 (View)
≈ 3,54 M Zelleinheiten.
*Beleg (Probe 4, Memory und redb identisch):* 1–4 Spalten werden zugelassen, ab
5 Spalten kommt `ValidationCapacity("cells")` bei Standardgrenze = harter Grenze
16.777.216. Höher einstellen lässt sich das nicht. Weil kein Snapshot entsteht,
kann auch kein Commit über diese Union gebaut werden. Abhilfe ist nur, die Spalten
vorher einzeln oder in kleineren Operationen neu zu schreiben.
*Bewertung:* Der Vertrag fixiert 8 Chunks und 16.777.216 als harte Grenze und sagt
nur, dass das Maximum unter *engeren* Grenzen nicht garantiert ist. Er verlangt
„resolve one Chunk pair at a time“. Die volle Endpunkt-Neuableitung pro FCBR
schreibt er für **Reopen** vor; für Lesezugriffe verlangt er nur „validate
encountered history“. Daten gehen nicht verloren, die Verweigerung ist typisiert
und schreibt nichts. Die zulässige Maximalform ist aber historienabhängig
unerreichbar.
*Auflage:* Vor einer Sim/App-Adoption entweder die Laufzeitprüfung auf den
eigenen Eintrag der Spalte begrenzen (Digest, Stamp, After-Paar gegen den
gespeicherten Stand; volle Neuableitung bleibt beim Reopen), oder Root akzeptiert
und dokumentiert die Einschränkung ausdrücklich im Vertrag. Die Tests der Delivery
zu 8/4096/1024 nutzen nur frische bzw. gemeinsame Historie und decken das nicht ab.

**A2 — Ein gescheiterter Fresh-Open hinterlässt eine unbrauchbare Datenbankdatei.**
`redb_store/blocks.rs:337-353`: `create_new` + `builder.create_file` vor
`initialize`. `redb_store/block_validation.rs:184-219`: Fresh führt `scan` →
`Index::create` (`:293`) vor dem Commit aus. `:24-35`: Ein Scratch-Fehler wird zu
`Backend(..)`.
*Fehlerszenario:* Der erste Start nennt ein nicht existierendes Scratch-Verzeichnis.
*Beleg (Probe 3, redb):* Der erste Open gibt
`Backend("UUID migration resolve parent failed for …/missing-scratch: …")` zurück
und lässt eine 3.686.400-Byte-redb-Datei ohne FerrumC-Katalog zurück. Der
korrigierte zweite Open mit gültigem Scratch liefert dauerhaft
`CatalogMismatch("missing marker table")`.
*Bewertung:* Logisch wurde nichts committet; dem Wortlaut nach ist der Vertrag
erfüllt. Der Pfad ist aber bis zum manuellen Löschen blockiert, und die Meldung
führt in die Irre (UUID-Migration, Katalog). Das widerspricht der Erwartung
„verweigerter Open ohne Mutation“.
*Auflage:* Den Scratch vor `create_new` prüfen, oder eine selbst angelegte Datei
bei einem Fresh-Fehler identitätsgesichert entfernen. Dazu einen typisierten
Scratch-Fehler einführen und einen Test für den Retry-Pfad ergänzen.

### Hinweise

- **H1 Refusal-Reihenfolge v1 in Format7:** Die gemeinsame Receipt-Byte-Grenze
  wird erst nach `owned::prepare` geprüft (`redb_store/blocks.rs:166-177`,
  `memory/ownership.rs:276`), also nach der Player/Entity-Historienprüfung
  (`ownership/engine.rs:641ff`). Der Vertrag (Z. 280-281) verlangt Receipt-Zeilen
  und -Bytes vor der Historie. Das betrifft nur die Fehlerart, wenn beide Grenzen
  zugleich erschöpft sind.
- **H2 Startup-Aufwand:** Jede gespeicherte Spalte leitet den Endpunktbeweis ihres
  letzten FCBR neu ab, bis zu 3× pro Spalte (full-, overlay- und header-Zeile),
  abgefedert durch einen Cache mit 128 Einträgen (`block_validation.rs:409-421`,
  `engine.rs:453-457`). Die Relationsgrenzen haben keine harte Obergrenze. Das ist
  vertragskonform, die Startzeit wächst aber mit der Weltgröße × Receipt-Breite.
- **H3 Race-Verlierer nur teilweise geprüft:** Bei der überlappenden Block-Union
  (`player_block_matrix.rs:313-321`) wird der Zustand des Verlierers nicht
  geprüft. Bei der Transfer-Race (`:159-222`) wird nicht geprüft, dass die Entity
  fehlt, wenn Joint gewinnt. Meine Probe 2 prüft den Gesamtzustand und ist grün.
  Die Root-Klarstellung ist sonst umgesetzt: Guard-only bewahrt den Stamp
  (`:297-311`); ein nachfolgender echter Nachbar-Write committet (`:312`);
  disjunkte Unions committen beide (`:284-289`); Drift ergibt CAS (`:290-295`).
- **H4 Selbstreferenzielle Grenzen:** Die exakte Snapshot-Ausgabegrenze, die
  Scratch-Lebenszyklusgrenze und die Laufzeit-Zellgrenze werden aus Werten der
  Implementierung ermittelt bzw. nur grob geprüft (98.303 Zellen). Der Vertrag
  legt dafür keine Byteformel fest; zulässig, aber kein unabhängiger Sollwert.
- **H5 Vom Ersteller genannte Einschränkungen:** (a) Frei gewählte States der
  Türhälften sind zulässig, weil Storage keine States interpretiert; echte Türen
  belegt erst Sim. (b) Legacy-Entity-Bytes bei Keep: Der Code bewahrt sie
  konstruktionsbedingt bytegenau (`raw.rs:186-189` kopiert das Rohpaar,
  `engine.rs:871-873` schreibt nur geänderte Paare). Ein Laufzeittest gehört in
  DUR-03.3a, wo solche Bytes erst entstehen. Zulässig. (c) MAX nur auf redb:
  `after_stamps`/`prepare` sind gemeinsam, Memory hat keine eigene Stamp-Logik.
  Zulässig. (d) Snapshot-Grenze: siehe H4. Zulässig.
- **H6 Claims-Scan:** Die Delivery hat 0 Treffer. Neu gegenüber main sind nur
  Treffer zum Rust-Fachbegriff „dyn-compatible“ (`INVARIANTS.md:341`,
  Rustdoc `block_durability.rs:785`). Beide liegen außerhalb des Prüfumfangs des
  Skripts (`README.md`, `docs/**`, Plugin-Quellen). Die übrigen 8 Treffer in
  Storage-INVARIANTS/README bestehen schon auf main. Die Delivery behauptet keine
  RSS-, Crash-, Stromausfall-, Survival- oder App-Aktivierungsaussagen; alle
  entsprechenden Stellen sind ausdrücklich verneint.
- **H7** `block_durability.rs:540` `#[allow(clippy::too_many_arguments)]` hat
  keinen Begründungskommentar; die Delivery behauptet, alle `allow`s seien begründet.
- **H8** Der Commit-Fault-Test (`redb_store/blocks/tests.rs:280-299`) vergleicht
  im Fall „Receipt vorhanden“ den neuen Zustand nicht zeilenweise. Er verlässt sich
  auf die Reopen-Validierung, was vertretbar ist.
- **H9 Delivery-Satz** „Keine ungelösten Autoritäts- oder Vertragsfragen
  festgestellt“: Autoritätsfragen habe ich keine gefunden. A1 ist eine offene
  Kapazitäts- und Vertragsfrage für Root.

### Korrekturen in Runde 3

1. **Scratch-Index-Grenze:** Richtig, redb schreibt beim Anlegen und beim Abort
   transient über die Endlänge hinaus. Die Lebenszyklus-Bisektion prüft N/N−1
   und das Aufräumen; der Sollwert bleibt selbstreferenziell (H4).
2. **Shapes-Test:** Richtig, `Chunk::mark_persist_dirty` ist echte World-API
   (`ferrumc-world/src/chunk.rs:263`); `section_count` wird explizit geprüft. Die
   Produktion war korrekt.
3. **`Clone` → `share()`:** Richtig. `InMemoryStore` war auf main nie `Clone`;
   `share()` ist `pub(crate)` und teilt dieselbe Inkarnation.
4. **Entpacken verschachtelter Fehler:** Richtig. `From<StorageError>` und
   `From<OwnershipError>` packen `BlockValidation`/`Block` strukturell aus, und
   `ownership_adapter` packt `Ownership(e)` aus. Eine Endlosverschachtelung ist
   nicht möglich, und eine String-Klassifikation gibt es nicht.

## Teil C — Testehrlichkeit und eigene Probes

**Fixtures:** `make_fixtures.py` importiert keinen FerrumC-Code. Die Tests lesen
die eingefrorenen Bytes und vergleichen sie mit unabhängig gelesenen
17-Tabellen-Zeilen. Mein eigener Decoder (`fixture_handcheck.py`, nur aus der
Vertragsgrammatik Z. 168-235 geschrieben) bestätigt 47 Erwartungen, darunter:
seed/joint FCBQ = 98+46n+12g+64e+Payload (288 bzw. 42.578 Bytes); FCBR =
71+|Q|+9n (377 bzw. 42.667); FCBR-Digest = SHA256(Domain‖Q); reserved zero8;
Range 1/0/4; kind3 inaktive Player-Felder Null/Fresh; Fresh → Live1;
Joint-Player-Stamp 1→2; FCOB 31 Bytes Live rev2 last=op3; FCBL = 100+Payload mit
Scope/Op/Ordinal/Sequenz; `scoped_journal_bytes` = Σ(8+v);
`operation_receipt_bytes` = 272 + Σ(16+|FCBR|); 15 Pflicht-Meta + nur
`last_scoped_mutation_id`; Player-Header 39 Bytes rev2; alle unbeteiligten Tabellen
unverändert. Der FCPS3-Header passt zur echten Sim-Grammatik
(`ferrumc-sim/src/player/codec.rs`: Magic, v3, 3×f64, 2×f32, sel<9, u16=46,
47 Frames). Die FCIS- und Chest-Bytes prüft zur Laufzeit der unabhängige
World/Items-Codec (kanonisches Re-Encoding).

**Probes** (`probes/root_review_probes.rs`, SHA-256
`d74de1b921985d7c5b39e1e8c96cc85fc0ca2c80bd026fafda79f1e974754453`, nur in der
privaten Kopie `probe/`; Logs `logs/probes-run1.log`, `logs/probes-run2.log`):

| Probe | Risiko | Memory | redb |
| --- | --- | --- | --- |
| 1 Operation-ID über verschiedene Spieler | Spieler B nutzt A's FCBR-ID 3, A's FCTR-ID 1, eigene FCTR-ID 5; v1 von B mit ID 3 | grün: jeweils `ReceiptCollision`, B-Player, Stamps und A-Receipt unverändert, derselbe Request unter neuer ID committet | grün, auch nach Reopen |
| 2 Dreiwege-Race Player- vs. Chunk-Revision | Joint (Player+2 Chunks) gegen v1-Autosave und Image-CAS auf demselben Stand, Barrier, 8 Runden je Backend | grün: nur {Joint} oder {Autosave, CAS}; Gesamtzustand inkl. Entity und Receipt-Absenz konsistent | grün, auch nach Reopen (gesamt 5 Joint-Siege / 11 Niederlagen) |
| 3 Gescheiterter Fresh-Open | Scratch fehlt beim ersten Start | – | **rot → A2** |
| 4 8-Chunk-Union mit getrennter Historie | harte Zellgrenze | **rot → A1** (ab 5 Spalten) | **rot → A1** (ab 5 Spalten) |

## Ergebnis

`final-assessment.json` enthält Verdikt, Zahlen und Hashes. Empfehlung an Root:
Die Storage-Integration ist technisch sauber, die Format6-Erhaltung und die
Autoritätsgrenzen sind ohne Befund. Vor CLEAN_RUNTIME sollten A1 und A2 behoben
werden; A1 kann Root alternativ ausdrücklich als Vertragseinschränkung annehmen.
