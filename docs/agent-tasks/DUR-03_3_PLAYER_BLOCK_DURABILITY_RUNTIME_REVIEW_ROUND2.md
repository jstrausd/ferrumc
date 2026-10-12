# DUR-03.3 — unabhängiger Runtime-Review, Runde 2 (Kandidat Runde 5)

Stand: 11.10.2026. **Verdikt: FINDINGS.**

A1, A2, H1, H3 und H7 sind behoben, H4 ist teilweise behoben. Die Runde-5-Änderungen
(geliehene Frames, Receipt-Cache pro Aufruf, Einmal-Abrechnung an der Quelle) sind
korrekt: Ich habe keinen Lesepfad ohne Abrechnung gefunden, keine Lebenszeit- oder
Borrow-Lücke und keine Regression bei Fehlerreihenfolge, Atomizität, Replay, Reopen,
Race-Matrix oder Format6. Alle frischen Checks sind grün (Zahlen unten).

Offen ist **ein neuer Befund (B1)** zur harten 512-MiB-Byte-Grenze. Die Runde-5-Delivery
und `INVARIANTS.md` nennen als einzigen Restfall „8 verschiedene maximal große letzte
Receipts“. Meine Probe 7 zeigt auf Memory und redb mit harten Default-Grenzen, dass
deutlich kleinere, gewöhnliche Formen ebenfalls mit `ValidationCapacity("bytes")`
enden. Ursache ist die feste 16-KiB-Reserve je Block-Entity. Das ist keine
Runde-5-Regression, die Ursache besteht seit Runde 3. Die Root-Entscheidung zur
Byte-Grenze stützt sich aber auf eine unvollständige Darstellung. Über die Integration
entscheidet Root.

Diesen Review habe ich unabhängig erstellt; Kandidat und Runde-1-Review stammen nicht
von mir. Gebaut und getestet habe ich nur in privaten Kopien unter
`.codex-tmp/dur03_3-runtime-root-review-r2/`. Kandidat, Delivery, Evidenz und
Produktion habe ich nicht verändert. Git habe ich nicht benutzt, Agents habe ich keine
gestartet. Dieser Review macht keine Aussage zu App-Aktivierung, Survival,
Paper-Gleichheit, RSS, Prozessabbruch oder Stromausfall.

## Prüfbasis und Integrität

| Objekt | Ergebnis |
| --- | --- |
| `combined/` gegen `combined.freeze.json` (`3673a4a8…a373`) | vor dem Start, nach den Checks und nach allen eigenen Probes jeweils 1327/1327 identisch, keine fehlt, keine zusätzlich |
| 90 Kandidatendateien gegen `round5-candidate-manifest.json` (Datei `493ebb50…1a4c`) | Digest `1d8455ef…f678` nachgerechnet (Schema `pfad\tbytes\tsha256\n`, Manifest-Reihenfolge). 90/90 identisch in `combined/`, `round5-candidate-src/`, Arbeitsbaum und `work/`; keine Zusatzdatei im Storage-Crate. Gleicher Stand vor und nach allen Läufen |
| Delivery | `f23cff1c…58ff` ✓ |
| Evidence-Freeze `round5-evidence-freeze.json` | `eb268473…ab49` ✓; alle 37 gelisteten Evidenzdateien hashgleich |
| Vertrag / Vertrags-Review / Design-Freeze | `2edfc588…42ab` ✓ / `3a8c0986…d0f3` / `3710c572…e92f` ✓ |
| Runde-1-Review / -Assessment | `8adcc9ee…5069` ✓ / `8f6dba93…7cac` ✓ |
| Runde-4-Kandidat (Gegenprobe) | 89/89 Dateien gegen `round4-candidate-manifest.json` (`4e665f47…5387`) identisch in meiner Kopie `r4probe/` |
| `INVARIANTS.md`/`README.md` von main `ce39d817` (Claims-Baseline) | Runde-1-`mainref` hashgleich mit `round5-base.freeze.json` |

## Teil A — Ausführung

Frisches, vorher nicht existentes Target `work-target/`, private Kopie `work/` von
`combined/`, `--jobs 2 --locked`, `rustc 1.99.0 (b940084d7 2026-09-28)`,
`cargo 1.99.0`. Skript `run_checks.sh`, Logs unter
`.codex-tmp/dur03_3-runtime-root-review-r2/logs/` (SHA-256 in `logs/sha256-logs.txt`).

| Check | Ergebnis |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo test -p ferrumc-storage --no-fail-fast` (Debug) | exit 0, **346/346** in 17 Ergebnisblöcken: Lib 253, 15 Integrations-Binaries, Doctests 12; 0 failed, 0 ignored, 1165 s |
| dito `--release` | exit 0, **346/346** in 17 Ergebnisblöcken, 487 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, 0 Warnungen; `ferrumc-storage` frisch geprüft (frisches Target) |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-storage --no-deps` | exit 0, „Documenting ferrumc-storage“, 0 Warnungen |
| `cargo build --workspace --all-targets` | exit 0, 0 Warnungen, 282 s |
| Verbraucher `cargo test -p ferrumc-sim -p ferrumc-app -p ferrumc-testkit --no-fail-fast` | exit 0, **844 bestanden, 0 fehlgeschlagen, 1 ignoriert** (`genuine_paper_api_plugin_…`, braucht Java und Maven-JARs) in 58 Ergebnisblöcken |
| Original-Probes Runde 1 (unveränderte Datei `d74de1b9…4754453`, eigene Kopie `probe/`) | exit 0, **4/4**: Probe 1 Kollision ohne Mutation; Probe 2 Joint 5 Siege, 11 Niederlagen in 16 Runden, Gesamtzustand konsistent; Probe 3 `ScratchUnavailable("resolve parent …/missing-scratch: No such file or directory")`, keine Datei, Retry ok; Probe 4 1..8 Spalten mit verschiedener Historie auf Memory und redb zugelassen |
| `.so`-Testartefakte (keine Quellen) | Der App-Plugin-Testhost hat in `work/` zwei Dateien erzeugt: `.codex-tmp/p57-plugin-testhost-2126706-artifacts/libplugin_testhost_fixture.so` und `…/librenamed_fixture.so` (je 8.459.416 Bytes). Das sind die einzigen Zusatzdateien in `work/`; `combined/` ist unverändert |

Die Delivery-Zahlen sind reproduziert: 346/346 Debug und Release, Lib 253,
17 Ergebnisblöcke; Verbraucher 844 + 1 ignoriert in 58 Blöcken.

**Eigene Probes** (private Kopie `myprobe/`, eigenes Target `myprobe-target/`;
Dateien in `probes/`):

| Probe | Risiko | Memory | redb |
| --- | --- | --- | --- |
| 5 `root_review_r2_probes.rs` (`15eefd30…7d92`) | Lookup und exakter Replay eines großen Receipts (R = 1.707.225, Q = 1.707.082) genau an der Byte-Grenze, Sollwert von mir hergeleitet: Lookup 1067 + 2R + 2Q + 2ΣF = 10.242.913, Replay 1067 + 2R + 3Q + 2ΣF = 11.949.995 | grün: N zugelassen (Bytes identisch, `Replayed`), N−1 → `ValidationCapacity("bytes")`, keine Mutation | grün, Replay des alten Requests nach Reopen |
| 6 (gleiche Datei) | Zwei Receipts im Aufruf-Cache (R2 neue Revision über R1, R1 mit 4 historischen Einträgen). Exakter Snapshot-Bedarf 6.770.595 nach meinem Modell. Danach Scoped-Commit R3 über die volle Union, Replay, veralteter Snapshot, alle drei Receipts | grün: Views = Einzelspalten-Snapshots, Revisionen 2/1 → 3/2, Replay byte-identisch, veralteter Snapshot → `CasConflict`, R1/R2/R3 lesbar | grün, zusätzlich exakt N zugelassen / N−1 verweigert und alles nach Reopen |
| 7 `root_review_r2_entity_probes.rs` (`a74d74c3…94dd`) | Harte Default-Grenzen, entity-dichte Spalten | **Befund B1** (s. u.) | **Befund B1** |

**Gegenprobe auf dem unveränderten Runde-4-Kandidaten** (`r4probe/`, Log
`r4-counterprobe.log`):
- Die Kandidaten-Testdatei `player_block_bytes.rs` (`af4f7627…`) scheitert dort
  3/3 mit `ValidationCapacity("bytes")` an denselben Stellen wie in der Delivery
  (Z. 184, 215, 314).
- Meine Probes 5 und 6 scheitern dort ebenfalls (Runde 4 zählte mehrfach). Meine
  Modelle unterscheiden also die Kandidaten.
- Probe 7 verweigert dort dieselben Formen, beim 8-Spalten-Fall schon den Snapshot.
  B1 ist demnach älter als Runde 5.

## Teil B — Source-Review

Zeilenangaben beziehen sich auf den Kandidaten (`round5-candidate-src/` = `combined/`).

### Befund (nach Schwere)

**B1 (mittel, Auflage) — Die harte 512-MiB-Byte-Grenze verweigert gewöhnliche
entity-dichte Unions, nicht nur „8 verschiedene maximale Receipts“.**

- *Stellen:* `block_durability/engine.rs:193-205` (`entities`: `b.bytes(16 * 1024)`
  je Block-Entity, aufgerufen in `column` `:339` für Vor- **und** Nachbild),
  `:180-192` (`entity_bytes`, Guards/Effekte), `:710-730` (`view_encoded_len`:
  noch einmal 16 KiB je Entity der View). Die harte Grenze steht in `Budget::hard`
  (`:42-53`). Die Darstellung des Restfalls findet sich in der Delivery Z. 161-172
  und Z. 252-254 sowie in `INVARIANTS.md:418-421`.
- *Mechanik:* Jeder Gameplay-Spaltenbeweis (Historie kind 1/2 und neuer Request)
  rechnet 2 · E · 16 KiB ab, jede Snapshot-View weitere E · 16 KiB, und zwar
  unabhängig von der echten Kodierungsgröße. Gemessen: 4096 leere Truhen ergeben
  einen Full-Record von 1.298.481 Bytes, also ≤ 317 Bytes je Entity. Die Reserve ist
  damit ≥ 51-mal so groß. Die Zellen bleiben dabei weit unter der Grenze
  (≤ 823.296 je Spaltenbeweis).
- *Rechnung (aus dem Code):*
  - Snapshot-Spalte mit Gameplay-Historie ≈ 48 KiB · E. Damit liegen k Spalten über
    512 MiB ab E > 10.922 / k, also 3 Spalten bei 4096 und 8 Spalten ab etwa 1366.
  - Gameplay-Commit ≈ 64 KiB · E je Spalte (Historie und Request). Damit liegen
    k Spalten ab E ≥ 8192 / k über der Grenze, also 2 Spalten bei 4096 und 8 Spalten
    ab etwa 1000 Entities je Spalte.
- *Beleg (Probe 7, `root_review_r2_entity_probes.rs`, Memory **und** redb, alle
  Grenzen auf hartem Default):*
  - 7a, 4096 leere Truhen je Spalte: Der 3-Spalten-Gameplay-Commit G1
    (Image-CAS-Historie) läuft durch. Danach wird ein 3-Spalten-Snapshot verweigert
    (`ValidationCapacity("bytes")`), der 2-Spalten-Snapshot läuft, und der
    2-Spalten-Gameplay-Commit wird verweigert. Es gibt keine Mutation, das Receipt
    fehlt. Der 1-Spalten-Commit läuft.
  - 7b, 1100 leere Truhen je Spalte (27 % des World-Maximums): Der
    8-Spalten-Snapshot läuft, der 8-Spalten-Gameplay-Commit wird verweigert. Der
    4-Spalten-Commit läuft.
  - Ein 2-Spalten-Commit über Spalten mit 4096 Entities entspricht der Vertragsform
    „Türhälften/Nachbar über die Chunkgrenze“. Mit kleineren Entity-Zahlen trifft
    dasselbe 8-Spalten-Unions.
- *Zweiter, kleinerer Teil (nur gerechnet, nicht ausgeführt):* Auch beim Receipt-Restfall
  ist „acht“ zu hoch angesetzt. `validated` liest für jeden *aktuellen* Eintrag
  eines berührten Receipts dessen gespeichertes Paar (`engine.rs:487-503`). Probe 6
  bestätigt diese Abrechnung exakt mit N/N−1. Ein Receipt mit Q ≈ 32 MiB, dessen
  unberührte Einträge noch aktuell sind, kostet deshalb ≈ 16 + R + Q + Σ Paare
  ≈ 3 · 32 MiB. Ein Snapshot liegt damit ab **6** solchen Receipts über 512 MiB, nicht
  erst ab 8. Ein Commit mit eigenem Q ≈ 32 MiB (Request, Endpunkt-Decode, Encode,
  Frame, Kopie ≈ 4Q + 2R ≈ 192 MiB) liegt schon ab **4** verschiedenen solchen
  Historien darüber. Die Aussage in `INVARIANTS.md:418-421` („need at least“)
  bleibt als untere Schranke wahr. Die Delivery-Formulierung „Restfall“ (Z. 161ff.,
  252ff.) stellt sie aber als einzigen Fall dar.
- *Bewertung:* Die Verweigerung ist typisiert, rein lesend bzw. Preflight und ohne
  Mutation; Sicherheit und Atomizität sind nicht betroffen. Das Muster ist dasselbe
  wie bei A1, nur in Bytes statt Zellen: Eine vertraglich zulässige Form ist unter
  der harten Grenze abhängig vom Inhalt unerreichbar. Hier ist die Form nicht einmal
  maximal. Die Root-Entscheidung zu Runde 5 deckt ausdrücklich nur den
  Receipt-Restfall. Der Root-Grundsatz „keine Abrechnung für Arbeit, die nicht
  stattfindet“ spricht gegen eine 16-KiB-Pauschale für eine Kodierung, die
  tatsächlich ≤ 317 Bytes erzeugt.
- *Auflage (Root entscheidet):* Entweder (a) die Entity-Kodierung mit ihrer echten
  Länge abrechnen. Der Encoder ist pro Entity schon auf 16 KiB begrenzt, und die
  Zellabrechnung (1 je Entity) läuft weiterhin vorab. Dazu einen N/N−1-Test mit
  unabhängigem Sollwert und Probe 7 als Regressionstest aufnehmen. Oder (b) Root
  nimmt die Einschränkung ausdrücklich an. Dann gehört sie mit den Schwellen
  (k Spalten × E Entities, Receipt-Fall ab 6 bzw. 4) in Delivery und `INVARIANTS.md`,
  und die Formulierung „einziger Restfall“ wird korrigiert.

### Hinweise

- **R2-H1 (gering) Doppeltes Parsen im Cache:** `Lasts::load` (`engine.rs:614-615`)
  parst Q zweimal: in `raw::frame` (`raw.rs:580`) und danach noch einmal ohne
  Abrechnung. Das zweite Parsen ist strukturell (Slices, Keep-Paarvergleich) und auf
  ≤ 8 Receipts je Aufruf begrenzt. `INVARIANTS.md:407-409` („read, digested and
  parsed once per call“) und Delivery Z. 115f. stimmen deshalb nicht ganz. Kein
  Sicherheitsproblem. Abhilfe: das in `frame` geparste Q zurückgeben oder die
  Formulierung anpassen.
- **R2-H2 (gering) Fehlermaskierung bei Doppelfehler (A2-Umfeld):** `initialize`
  (`block_validation.rs:202-213`) und `scan` (`:327-335`) geben bei einem Fehler
  von `index.finish()` diesen zurück, statt des ursprünglichen Scan- bzw.
  Initialisierungsfehlers. Ebenso
  `blocks.rs:387-389` beim `create_file`-Fehler. Das betrifft nur den Fall, dass
  beide Fehler zusammen auftreten. Die Datei wird trotzdem korrekt behandelt
  (`abandon_fresh` läuft vorher), die Meldung nennt dann aber nur den
  Scratch-Cleanup.
- **R2-H3 (gering, Restrisiko) TOCTOU beim Entfernen:** `remove_owned_path`
  (`uuid_migration/files.rs:539-555`) prüft Inode und Datentyp per
  `symlink_metadata` und entfernt dann über den Pfad. Wer im Zeitfenster dazwischen
  eine fremde Datei an den Pfad umbenennt, verliert sie. Dafür braucht es
  Schreibrechte im Verzeichnis. Es ist dieselbe Maschinerie wie in DUR-03.2, und der
  Vertrag verlangt deren Übernahme. Andere Öffner schreiben nie in eine leere
  vorhandene Datei: Format7, Format6 und der Legacy-Pfad verweigern
  `(Existing, None)` (`redb_store.rs:615`). Ein Wettlauf über die
  redb-Lock-Freigabe in `abandon_fresh` (`blocks.rs:48-56`, `drop(db)` vor dem
  Entfernen) kann deshalb nichts Fremdes löschen.
- **H2, H8:** unverändert, wie in Runde 1 als vertretbar eingestuft.

### A1, A2, H1, H3, H4, H7 — Status mit Gegenprobe

| Befund | Status | Gegenprobe |
| --- | --- | --- |
| A1 Lesepfad prüft volle Endpunkte je Spalte | **behoben** | Original-Probe 4 grün (1..8 Spalten, Memory und redb). Code: `inspect_chunk` (`engine.rs:624-683`) prüft den eigenen Eintrag (Index, Identität, Stamp, `e.after == p`), dazu `validated` einmal je Receipt (Identität, Union, Outcome-Stamps, Player-Header/-Postimage, Header und Paare aller Einträge, exakte Journalzeilen) und den Endpunktbeweis nur dieses Eintrags (`column`, Cache `(id, index)`). Die View ist entweder genau dieses bewiesene Nachbild (wegen `e.after == p` identisch zur Komposition des gespeicherten Paars) oder `resolve` des gespeicherten Paars. Einen Pfad, der eine View aus einem unbewiesenen Eintrag liefert, gibt es nicht. Eine gefälschte berührte Spalte erkennt der Unit-Test `reads_prove_touched_history_and_refuse_forged_or_inconsistent_history` (`IncompleteDelta` bzw. `MalformedData`, Reopen verweigert). Eine Fälschung in einer **unberührten** Spalte, die alle Relationen erfüllt, erkennen erst Lookup, Replay und Reopen. Das entspricht der Root-Entscheidung. Zellrechnung selbst nachgerechnet: Paar auflösen ≤ 3·98.304 + 2·4096 = 303.104; Spaltenbeweis ≤ 2·303.104 + 2·4096 (Guards) + 4·1024 (Effekte) + 2·98.304 (Vergleich) + 2·4096 (Entities) = **823.296**; View ≤ 98.304 + 4096 = 102.400; Snapshot ≤ 8·925.696 = **7.405.568**; Commit (8 Historien- + 8 Request-Beweise, keine weitere Zellarbeit in `prepare`) ≤ 16·823.296 = **13.172.736** < 16.777.216 ✓. |
| A2 Fresh-Open hinterlässt blockierende Datei | **behoben** | Original-Probe 3 grün (`ScratchUnavailable`, keine Datei, Retry ok). Code: `Index::create` vor jedem Pfadzugriff (`blocks.rs:368-371`). Inode-Identität aus dem `create_new`-Handle (`:381`); ohne sie wird nie entfernt. `abandon_fresh` entfernt nur bei positivem Leerbeweis: keine Tabelle und keine Multimap-Tabelle über das offene Handle, ohne Handle nur bei 0 Bytes. Dazu muss der Pfad noch dieselbe reguläre Datei/Inode sein (`remove_owned_path`). Der Existing-Pfad entfernt nie. Eine tabellenlose Datei liefert `CatalogMismatch` mit klarer Meldung. Restrisiken: R2-H2, R2-H3. |
| H1 Fehlerreihenfolge v1 in Format7 | **behoben** | `admit_shared_receipt_bytes` (`ownership/engine.rs:574-593`) läuft nach der Receipt-Zeilen-Grenze und vor der Revisionshistorie (`:668`), nur mit Format7-Konfiguration. |
| H3 Race-Verlierer | **behoben** | `player_block_matrix.rs`: Zellen, Entity, Spalten-Stamps, Receipt beider Seiten; bei Transfer die Entity; bei überlappenden Unions genau ein Inkrement und das Gewinner-Receipt. |
| H4 Selbstreferenzielle Grenzen | **teilweise** | Zellen und Snapshot-Ausgabe (Memory) haben jetzt unabhängige Sollwerte. Die Bytes haben in Runde 5 ein Modell, das meine Probes 5 und 6 bestätigen. Die Scratch-Lebenszyklusgrenze und die redb-Snapshot-Ausgabegrenze bleiben gemessen bzw. iterativ (von der Delivery offengelegt). |
| H7 `too_many_arguments` ohne Begründung | **behoben** | `block_durability.rs:544-546`. |

### Runde 5 im Detail

- **Geliehene Frames:** `raw::Frame<'a>` (`raw.rs:523-552`) hält nur `&'a [u8]`.
  Im Lesepfad gibt es kein `Arc::from` mehr. Eine eigene Kopie entsteht nur in
  `Frame::owned` (`:540`), und dort wird vor dem Kopieren abgerechnet: bei Lookup
  `engine.rs:846`, bei Replay `:917` und im Commit über `raw::receipt` (`:1072`).
  Der Cache (`engine.rs:576-618`) legt die geliehenen redb-`AccessGuard`s (`RawGuard<'r>`)
  in einer lokalen `OnceCell`-Arena ab, Lebenszeit `'r` am Reader. Der Crate hat
  `forbid(unsafe_code)`, also schließt der Borrow-Checker ein Use-after-free aus. In
  redb endet der Reader und mit ihm die Arena am Ende des `prepared`-Blocks
  (`redb_store/blocks.rs:319-326`), vor `apply`. In Memory leiht der Reader den
  Write-Guard, und `apply` braucht `&mut`. Ein gecachtes Receipt kann deshalb
  nicht neben einem Schreibvorgang derselben Transaktion leben; das ist
  strukturell ausgeschlossen. Probe 6 belegt das Verhalten.
- **Dedup-Cache:** Er hat höchstens 8 Slots. Der Slot wird vor dem Lesen gewählt, ein
  9. verschiedenes Receipt wird typisiert verweigert. Schlüssel ist die Receipt-ID,
  gelesen wird nur aus `ownership_receipt` (eine Tabelle, ID als Schlüssel), in einer
  Read-Txn oder unter einem Lock. `raw::frame` prüft Magie `FCBR` (ein FCTR unter
  dieser ID wird verweigert) und `Q.id == key` (`raw.rs:571-583`). Eine ID-Kollision
  mit anderem Inhalt ist damit nicht möglich. Andere Tabellen und Namespaces
  speisen den Cache nie. Ein Scope-Wechsel scheitert an `q.identity != identity`.
  Der Cache lebt nur innerhalb eines Aufrufs: in `snapshot`, `prepare` und
  `chunk_state`.
- **Einmal-Abrechnung an der Quelle:** Alle Format7-Leser tragen ein Budget:
  - Memory `read_owned_inner` und `write_owned_inner` (`memory/ownership.rs:214`,
    `:259`) sowie `block_read`/`block_write` (`memory/blocks.rs:179`, `:283`);
  - redb `extend_read` und `reader` (`redb_store/blocks.rs:148`, `:163`), damit auch
    v1-Lesen (`ownership.rs:450-453`) und `commit_v1`;
  - Reopen (`block_validation.rs:351`).

  Jede `Meter`-Instanz umhüllt einen Leser mit derselben `Rc`-Budgetinstanz
  (6 Stellen geprüft). Jeder Zugriff über `block`, `owned`, `get`, `identity` oder
  `index_rows` wird damit genau einmal abgerechnet. Einen Format7-Leser mit
  `block_config = Some` und `budget = None` gibt es nicht. `Budget::runtime`-Fallbacks
  (`engine.rs:555`, `ownership/engine.rs:351`) greifen nur theoretisch. Format6:
  `budget = None`, `block()` ist unerreichbar, die einzige Änderung in
  `ownership/engine.rs` betrifft ausschließlich FCBR. Format6 ist damit unverändert.
  Ein unabgerechneter Lesepfad existiert nicht. Den einzigen unabgerechneten Rest
  beschreibt R2-H1: ein zweites Parsen, kein Lesen.
- **Tests `player_block_bytes.rs`:** Die Sollwerte sind ein Modell der Zählklassen;
  Eingaben sind echte Zeilenlängen, die gegen das FCBQ-/FCBR-Layout geprüft werden.
  Gemeldete Bedarfswerte werden nicht zurückgelesen. Zwei Werte habe ich von Hand
  nachgerechnet:
  - **geteilt:** 419 + (16 + 4.000.233 + 4.000.090) + (35 + 8·79 + 3.999.624)
    + (8·79 + 2·3.999.624 + 8·64·16.384) = **28.389.537** ✓;
  - **verschieden:** 419 + 8·[(16 + 525.289 + 525.146) + (35 + 632 + 524.680)
    + (79 + 2·65.585)] = **13.656.795** ✓;
  - Layout: R = Q + 143 und Q = 466 + ΣFull ✓; 30.297.506 B = 28,9 MiB ✓.

  Meine eigenen, unabhängig hergeleiteten Modelle (Probes 5 und 6) treffen N/N−1
  exakt. Gegenprobe auf dem Runde-4-Kandidaten: Die Testdatei scheitert dort 3/3
  wie von der Delivery angegeben (Teil A).

### Regressionen (Diff Runde 4 → Runde 5)

Geändert sind 9 Dateien plus ein neuer Test; Writer-Dateien (`memory/blocks.rs`,
`redb_store/blocks.rs`) sind unverändert.
- In `prepare` bleibt die Reihenfolge gleich: Mode, Shape, Permit, Agreement,
  bestehende ID/Replay, Owner/Issuer, Identity, Player, dann die sortierten Chunks
  mit Stamp-CAS vor dem Vorbild.
- Der borrowed Vergleich `p == e.before` ersetzt die Kopie aus `chunk_state`, ohne
  die Fehlerart zu ändern.
- Weniger Abrechnung kann nur dazu führen, dass statt `ValidationCapacity` jetzt ein
  späterer, echter Fehler erscheint. Das ist beabsichtigt.
- Atomizität (Hooks, `apply`), Reopen-Struktur und Race-Matrix sind nicht berührt.
  Die Cutpoint-, Reopen- und Race-Tests laufen unverändert grün (Teil A).

### Delivery

- Die Zahlen stimmen mit meinem Lauf überein (Teil A).
- Hashes: Kandidat `1d8455ef…f678` (90/90), Delivery `f23cff1c…`, Evidence-Freeze
  `eb268473…` mit 37/37 hashgleichen Evidenzdateien. Die Testdatei der Gegenprobe
  `af4f7627…` ist identisch mit der des Kandidaten.
- Nicht-Behauptungen (RSS, Prozessabbruch, Stromausfall, Survival, Paper,
  App-Aktivierung) sind vorhanden.
- Unvollständig ist nur die Restfall-Darstellung (B1).
- Forbidden-Claims-Scan (Funktionen aus `scripts/check-forbidden-claims.py`,
  `tools/forbidden_scan.py`):
  - Delivery: 0 Treffer;
  - CI-Umfang: 0;
  - `INVARIANTS.md`, neu gegenüber main `ce39d817`: nur der Rust-Fachbegriff
    „dyn-compatible“ (wie Runde 1, H6).

## Ergebnis

`.codex-tmp/dur03_3-runtime-root-review-r2/final-assessment.json` enthält Verdikt,
Zahlen und Hashes.

Empfehlung an Root:
- Die Runde-5-Nachbesserungen sind technisch sauber und ohne Regression. A1 und A2
  sind behoben.
- Vor CLEAN_RUNTIME sollte Root über B1 entscheiden. Möglich ist eine Korrektur:
  Entity-Kodierung mit echter Länge abrechnen, mit unabhängigem N/N−1-Test und
  Probe 7 als Regressionstest.
- Alternativ nimmt Root B1 ausdrücklich an. Dann muss die Restfall-Darstellung in
  Delivery und `INVARIANTS.md` mit den tatsächlichen Schwellen korrigiert werden.
- R2-H1 bis R2-H3 blockieren nicht.
