# NET-04.2d — unabhängige Vertragsprüfung, Runde 3

Stand: 11.10.2026. **Verdikt: CLEAN_CONTRACT mit sechs Auflagen und sieben
Hinweisen. Es gibt keinen blockierenden Befund.**

- **Behoben:** N1, N2, N3 und R2-A1 bis A5. R2-A6 ist als DUR-05b-Auflage
  behoben. Die Hinweise R2-H1 bis H7 sind umgesetzt.
- **Keine Regression** in G1–G4, L1/L2, der G3-Matrix, dem Session-Producer und
  den SIM-03.4b-APIs. Die Ticketfolge bleibt zyklenfrei, mit einer
  Production-Crate je Ticket.
- **Neu, nicht blockierend** (Auflagen R3-A1 bis A6, §C):
  - Die Signatur der `.2c-Ergänzung R2` (D7/D8) lässt sich nach den
    Rust-Borrow-Regeln in dieser Form nicht übersetzen. Der Fehler stammt
    unverändert aus Runde 3.
  - Der Bewegungsdurchsatz der Lane liegt bei höchstens 10–16 Inputs je
    Boundary. Der Fehler bestand schon vorher; das Gate verschärft ihn leicht.
  - Eine konfigurierbare Stop-Frist fehlt. Die Obergrenze aus der Formel liegt
    bei etwa 7,4 h, nicht bei 37 min.
  - Der Crash-Verlustumfang verbundener Spieler ist nicht deklariert; es gibt
    keinen periodischen Player-Autosave.
  - Ein quarantiniertes Entity-Pending führt sicher in `Degraded`; diese Folge
    steht nicht im Brief.
  - Für einen gescheiterten Recovery-Boundary fehlt eine Disposition.

Geprüft wurde der eingefrorene Kandidat. Brief, Evidenz, frühere Reviews und
Produktion wurden nicht verändert.

| Gegenstand | SHA-256 |
|---|---|
| Brief `NET-04_2d_NATIVE_DRIVER_ENDPOINT.md` (1587 Zeilen) | `e0530e0a522b8c2e57a06268206a7721b9ca512b2fd2af3f7f71126fcdbba14f` |
| `.codex-tmp/net04_2d/design.freeze.json` | `fe8c32073f3f6da3ce155c0e5fc0c48042956aa34d085c4cb963cece5123f25d` |
| Review Runde 2 / `final-assessment.json` | `9453905f…1f9f` / `69df514c…4806` |
| Review Runde 1 | `7ece2449…1d16` |
| Workorder | `6919a9b7…5c3ceb5c` |
| Basis-Manifest (394d4c27, 1274 Dateien) | `4c964d711c1dd4499bf914327266134c734f8aa13aefe9010a5dab27d80f519b` |
| DUR-04.1 / DUR-04.2 / DUR-03.3 / DUR-05 / SIM-03.4a / NET-04.2c | `71f26af1…5b63` / `a655af05…c2e8` / `2edfc588…42ab` / `3206b075…3a96` / `a89b50cd…5d48` / `3dff58ac…d04c` (frisch, identisch mit Brief §1) |

Code habe ich nur aus `.codex-tmp/net04_2d-root-base/archive` gelesen.
- „Z.“ bezeichnet Zeilen des Briefs.
- Pfade ohne Präfix liegen unter `server/crates/ferrumc-sim/src/`.
- „counters.rs“ steht für `shard/player_ownership/counters.rs`.

Die Root-Vorgaben für Runde 4 und die früheren Entscheidungen habe ich nur aus
der Aufgabenstellung übernommen. Threads habe ich nicht gelesen und nichts
gesendet.

## A. Ausführung (reproduziert)

Alle Läufe fanden in `.codex-tmp/net04_2d-root-review-r3/` statt.

- **`round4/pins.py`** lief als private Kopie; geändert war nur `OUT`.
  - Ergebnis: Exit 0, 202/202 Anker mit Token auf der Startzeile, 0
    Basisabweichungen, `net04_2c_pins_ok`, Paper-HEAD `29c8822d…`.
  - `baseline.verify.json`, `producer-pins.json` und `reference-pins.json`
    sind **byteidentisch** mit der Kandidatenevidenz.
- **`round4/citation_coverage.py`** (Kopie): Exit 0, 131 Zitate, 0 fehlend.
  `citation-coverage.json` ist byteidentisch.
- **`round4/freeze.py`** (Kopie, nur `OUT` geändert): Die erzeugte
  `design.freeze.json` ist byteidentisch, SHA-256 `fe8c3207…5d`.
- **Anker:** Die ersten 137 Runde-4-Anker sind listengleich (Pfad, Zeilen,
  Token) mit den 137 aus Runde 3; 65 sind neu.
- **Forbidden-Claims-Scan** mit dem unveränderten Runde-2-Skript
  (`c50129cd…e104e3`, Muster aus `scripts/check-forbidden-claims.py`
  `eb2671f2…40ff`). Gescannt wurden Brief, `round4/EVIDENCE.md` und
  `n2_stop_walkthrough.md`, jeweils normal und mit `force_plugin`.
  - Ergebnis: **0 Treffer**, Ausgabe byteidentisch mit
    `round4/forbidden-scan.json`.
  - Deutsche Formulierungen habe ich von Hand geprüft (siehe B.8).
- **Eigene Nachrechnung N2:** `stop_recount.py`, unabhängig vom Modell des
  Erstellers aus dem Basiscode geschrieben, Ergebnis in `stop_recount.json`.
  Das ist ein Quellmodell, keine Runtime-Messung (B.4).
- Keine Builds, Tests, Server oder Clients wurden gestartet.

**Grenze der Coverage-Prüfung:** `citation_coverage.py` prüft nur Zitate der Form
`datei.rs:N`. Zeilenangaben ohne Datei wie `1610-1614` erfasst es nicht. Diese
habe ich stichprobenartig von Hand geprüft: `739`, `755`, `1020`, `1700`,
`1905`, `1939`, `1969`, `counters.rs` `666-679`, `1951` und `2090-2095`. Alle
stimmen.

## B. Quellenprüfung (Source-Review, keine Runtime-Abnahme)

### B.1 Neue Anker von Hand

Alle **65 neuen** Anker habe ich geöffnet und gegen die Aussage im Brief geprüft
(`manual-anchor-check.json`): 65 stimmen, keiner nur teilweise, keiner ist
falsch.

- Die R2-H1-Korrektur stimmt. `counters.rs:174` verwirft den Wert von
  `generation.next()?`.
- Die Generation wächst über `next_generation` (`player_ownership.rs:924-928`),
  `dispatch_retained` (`1396-1402`) und die Veröffentlichung (`1718`).

### B.2 Status der Befunde aus Runde 2

| Befund | Status | Gegenprobe am Basiscode |
|---|---|---|
| **N1** Recovery-Selbstblockade | **behoben** | B.3 |
| **N2** Stop A/B/C | **behoben** | B.4 |
| **N3** Apply-Refusals wieder eingereiht | **behoben** | B.5 |
| **R2-A1** `Due` + fremdes `blocking`, Status-Closure-Fehler | **behoben** | Z. 848-870. Die Transition nach `Waiting` ohne Versuch existiert; `Ineligible`, `StaleHandle`, Foreign und Snapshot sind typisiert. Das Szenario aus Runde 2, B.3, läuft durch: `plan_owned_player_write` verlangt weder `dirty` noch `connected` und verweigert nur bei eigenem Pending (`player_ownership.rs:1020`). |
| **R2-A2** Revalidierung ohne Driver-Handle | **behoben** (Signaturdetail siehe R3-A1) | Z. 282-284, 334-349. Die Session ruft die Closure mit dem gespeicherten Handle auf; der Driver hält kein Handle. |
| **R2-A3** Token-Tabelle, Lineage | **behoben** | Z. 778-786, 935-940, 1179-1182. Die Tabelle ist fallibel auf das Sessions-Limit vorreserviert; die Session vergibt die Lineage (checked u64). |
| **R2-A4** Storage-Typen in der Session-API | **behoben** | Z. 987-999. `ferrumc-sim` hängt bereits von Storage ab; `pub use player::*` (`lib.rs:140`) exportiert `OwnershipPendingStatus` schon heute. Der Re-Export erzeugt keinen Namenskonflikt: Das Sim-Modul `ownership` exportiert nur Shard-Typen (`lib.rs:136-139`). Die Session bleibt ohne Storage-Dependency (`ferrumc-session/Cargo.toml:33`). Die Kante e2 → SIM-03.4b ist gesetzt (Z. 1408, 1419). |
| **R2-A5** Rückweg für `held` | **behoben** | Z. 271-272, 307, 315, 376-385. `OwnedPlayerInputRefusal::into_parts()` liefert das Input zurück (`player/pending.rs:337-340`). |
| **R2-A6** Phase-B-Grenze | **behoben als DUR-05b-Auflage**, Ergänzung R3-A3 | B.4, B.6 |
| R2-H1 … H7 | umgesetzt | H1 siehe B.1. H2 Z. 158-163. H3 Z. 1029-1038 (`counters.rs:510`, `collector_lifecycle.rs:525-526`). H4 Z. 86-91. H5 Z. 981-986. H6 Z. 1011-1021. H7 Z. 1415. |

### B.3 N1 — NonRecoveryGate gegen die echten Queue-Grenzen

**Zählung (Count).**
- `enqueue_owned_player_input` prüft für jede Inputart dieselben Grenzen
  `input.len() ≥ limits.inputs` und Bytes (`player_ownership.rs:258-263`).
- Recovery nimmt die Queue komplett heraus. Klasse-1-Inputs (Completion,
  Reconcile, Retry, Drain, Refill) wendet es in Queue-Reihenfolge an, alle
  anderen reiht es zurück (`counters.rs:272-293`).
- Ein erfolgreicher Normal-Boundary nimmt die ganze Queue (`player_ownership.rs:305-307`).
- `_at` scheitert vor dem Queue-Drain (`counters.rs:185-204`).
- Damit gilt `G ≤ 16` als Invariante. `G` wächst nur im Normal-Modus und nur
  über `admit`; nach einem gescheiterten Normal-Versuch folgt sofort der
  Recovery-Modus mit geschlossenem Gate (Z. 504-506).
- Klasse-1-Inputs, die nach einem gescheiterten Normal-Versuch liegen bleiben,
  wendet der nächste Boundary an. Das ist immer ein Recovery-Boundary, denn
  zwischen zwei Boundaries kann kein Fortschritt beobachtet werden (Regel (i)).
- **Szenario N1(a) aus Runde 2 ist ausgeschlossen.**

**Bytes.** Die Charges habe ich am Code nachgerechnet
(`player/pending.rs:280-323`, `player/evidence.rs:290-309`):

| Input | Charge |
|---|---|
| Move, Mode, Select, Disconnect, Autosave, Stop, Drain, Retry, Reconcile | je 512 |
| Refill | 16 je ID + 512 |
| Restore/Rejoin | Record + 512 |
| Bootstrap | 512 (Record `None`) |
| Completion | 512 + Player + Entity |

- Die Recordgrenzen 1 MiB bzw. 256 KiB stehen in DUR-04.1 Z. 353.
- `2·1 049 088 + 14·512 = 2 105 344` und `8 388 608 − 2 105 344 = 6 283 264`
  stimmen. `⌊6 283 264 / 1 311 232⌋ = 4` stimmt.
- Count- und Bytegrenze sind beide vereinbar.
- `plan_entries` zählt `input.len()` mit (`counters.rs:470-487`). Die Belegung
  bleibt ≤ 32, also bleibt der Default ausreichend.
- Zu SIM-03.4a: Lifecyclezellen sind keine Queue-Grenze; die Prüfung bei
  Admission ist korrekt beschrieben (Z. 619-623).

**Terminierung und `Degraded`.**
- **K-Regel:** Sie garantiert spätestens alle 21 Iterationen einen
  Normal-Versuch, unabhängig von Completions. Damit sind 60 Fehlversuche nach
  höchstens 1 260 Iterationen erreicht (bei schnellem Fortschritt nach
  ≈ 120), deterministisch.
- **`Degraded` ist kein Endzustand.** Es ist typisiert und sichtbar (Metrik,
  Log, `NativeShardDegraded` für Permits/Joins) und wird beim ersten `Ok`
  verlassen. Ein Stop in `Degraded` endet über Phase A (≤ 1 200) und den
  Phase-B-Wachhund (1 200) nach etwa 2 min mit `Unresolved`.
- **Kein Datenverlust innerhalb des Prozesses:** Die Sim-Queue und die Pendings
  bleiben vollständig erhalten.
- **Zwei Einschränkungen**, die der Brief nicht nennt:
  - Ein quarantiniertes Entity-Pending löst `CounterRecoveryRequired` sicher
    aus und hält den Shard dauerhaft in `Degraded` (R3-A5).
  - Im Recovery- bzw. Degraded-Modus laufen keine Leave-Saves, weil sie
    Normal-Boundaries brauchen. Ein vom Betreiber ausgelöster Neustart verliert
    deshalb den Pose-/Modus-Stand seit dem Join (R3-A4).

### B.4 N2 — Phase-B-Drain, Invariante P und 4096-Item-Rechnung

**Drain als erstes Input in Phase B, Schritt für Schritt:**
1. Am Ende von Phase A ist die Queue leer: Der letzte Boundary war ein
   erfolgreicher Normal-Boundary, danach wurde nichts gated eingereiht
   (Z. 1222-1227, `player_ownership.rs:305-307`).
2. `enqueue` prüft nur `stopped`, nicht `draining` (`player_ownership.rs:254`).
3. Recovery wendet `Drain` an (`counters.rs:277-287`).
4. Die Allow-Liste unter `draining` enthält `Drain`
   (`player_ownership.rs:395-410`). `stopped` ist in Phase B falsch.
5. `mark_counter_flush(None)` setzt `priority` für jeden Slot mit Entity
   (`entity/counters.rs:262-273`).

**Invariante P, Schritt für Schritt:**
- `priority = false` schreibt nur `install_confirmed`
  (`entity/counters.rs:98-105`; Volltextsuche nach `priority` im Sim-Crate).
  Dabei gilt `saved = generation`.
- Im selben Schritt setzt `apply_counter_publication` die Generation auf die
  Projektion (`entity/counters.rs:120-123`). `restore_counter_flush` setzt die
  Priorität wieder, wenn sie vorher gesetzt war und `generation ≠ saved`
  (`counters.rs:408-420`, `entity/counters.rs:254-260`).
- `release_counter_projection` und `quarantine_counter_conflict` löschen
  `priority` nicht (`counters.rs:425-450`).
- Countersteps gibt es nur in `_at`, also in Normal-Boundaries
  (`counters.rs:212-221`, `674-686`).
- Neue Slots in Phase B entstehen nur durch bestätigte Spawns aus Phase A und
  sind sauber (`install_confirmed`).
- `counter_quiescent` ignoriert `priority` (`entity/counters.rs:248-253`).
- DUR-04.2 Z. 611-615 beschreibt genau diesen „completion-only Flush“ als Weg
  zur Quiescence. **P hält.**

**Phase C, Stop als letztes Input:**
- `stop_ticket()` gibt es nur bei `G = 0`. Das Gate ist zu, Completions werden
  verworfen. Retry, Reconcile und Refill entstehen nicht, weil
  `owned_stop_ready` kein Pending zulässt.
- `remaining` zählt nur nachfolgende Nicht-Completion-Inputs
  (`counters.rs:794-805`); mit Stop allein gilt `remaining == 0`.
- Nach dem Herausnehmen der Queue gilt `boundary_remaining = 0`
  (`player_ownership.rs:318-325`), und `ownership_quiescent` ist auf demselben
  Zustand wahr.
- **Stop ist strukturell das letzte Input.**

**Eigene Nachrechnung** (`stop_recount.py`; nur Quellmodell):

| Fall | Recovery-Boundaries | Saves | Refills |
|---|---:|---:|---:|
| L = 1 / 2 / 3 / 6 (wie Ersteller) | **5 696** / 9 792 / 13 888 / 26 176 | 4 096 | 127 |
| L = 1, Start-Turn 1–3 bzw. Cursor 2000 | 5 697 / 5 696 | 4 096 | 127 |
| L = 1, Save aus Phase A noch unterwegs | 5 695 | 4 096 | 127 |
| L = 1, Normal-Lane findet Dirty (alter `saved_tick`) | 4 224 | 4 096 | 127 |
| L = 1 / 6, Pool vom Betreiber auf 1 gesenkt | 11 350 / 31 830 | 4 096 | 4 095 |
| L = 1, ohne Phase-B-Drain (Protokoll Runde 3) | nie (0 Saves in 12 000) | 0 | 0 |
| **N = 65 536**, L = 1 / 2 | **92 134 / 157 670** | 65 536 | 2 047 |

- Die Zahlen des Erstellers sind bestätigt. Die Differenz 5 696 − 5 462
  erklärt sich wie im Brief: Turn und Cursor rücken vor der ID-Prüfung weiter
  (`counters.rs:1929-1951`).
- Alle 4096-Item-Varianten liegen unter `B_max = 44 768`, auch bei Pool 1 und
  L = 6.
- Bei 65 536 Items (`MAX_ENTITIES_PER_SHARD`, `entity.rs:28`) dauert schon der
  saubere Stop bei L = 1 ≈ 77 min. Die Formelgrenze liegt dann bei
  `B_max = 536 288` ≈ 7,4 h (R3-A3).

### B.5 N3 — Drop der Apply-Refusals und Folgeaktion aus dem Checkpoint

**Mechanik:**
- Die Refusal-Liste gibt `take_refusals()` heraus (`player/pending.rs:589-591`).
- Ein Drop gibt die `Arc<()>`-Lease frei. Danach gilt in `dispatch_retained`
  `strong_count == 0` (`player_ownership.rs:1390-1392`).
- DUR-04.2 Z. 606-609 und DUR-04.1 Z. 475-477 verlangen genau das.
  `held_completions` bleibt Enqueue-Refusals vorbehalten (DUR-04.1 Z. 563-565).

**Disposition je Status**, verglichen mit `complete_owned` (`1586-1650`) und
`validate_completion` (`1415-1455`):

| Status nach Apply | Ursache | Folge im Brief |
|---|---|---|
| `Prepared` | Busy/HistoryCapacity | `retry` (verlangt Prepared + freie Lease) |
| `Uncertain` | Unresolved, ReceiptAbsent | `reconcile` |
| `NeedsRefresh` | `Confirmed{Err}` | `reconcile` |
| `InFlight` | Validierungsfehler (Generation, Buffer, `local_before`, `check_reserved`) oder Abbruch von `preflight_deferred_for` bzw. `publish_confirmed` vor der Mutation | `reconcile` — korrekt, weil die Lease nach dem Drop frei ist |
| `Quarantined` | CasConflict im Counter-Modus, `confirm_current`-Fehler | nichts |
| kein Pending | Veröffentlichung oder definitive Refusal | nichts |

- **Vollständig:** Jeder Ausgang von `complete_owned` landet in genau einer
  Zeile.
- **Keine hängende Completion mit Lease:**
  - Completions liegen nur im Executor, im Kanal (≤ 32), in
    `held_completions`, in der Sim-Queue oder in `outcome.refusals`.
  - Der letzte Ort wird sofort geleert.
  - Die Sim-Queue wird von jedem erfolgreichen Boundary geleert.
  - Ausnahme ohne Spezifikation: Ein Recovery-Boundary, der selbst scheitert,
    lässt die Completion samt Lease in der Queue liegen (R3-A6).
- **Doppelte Folgeaktion:** möglich, aber harmlos.
  - `OutstandingWork` kennt keinen Zustand „Retry/Reconcile eingereiht“
    (Z. 699-703, 712).
  - Scheitert der Normal-Versuch, der ein eingereihtes `retry` anwenden soll,
    reiht die nächste Iteration ein zweites ein.
  - Das zweite wird mit `Reserved` verworfen, weil die neue Work-Lease lebt.
  - Es kostet aber Klasse-1-Plätze und verfälscht die Klärungszählung (R3-H1).
- **Verpasste Aktion:** keine gefunden.
  - Jedes Pending entsteht mit Work, also mit einem Eintrag.
  - Join, Refill und Disconnect haben je Identität höchstens ein Exemplar in
    flight (Permit je UUID, `refill_pending`, idempotenter Disconnect). Die
    Ableitung aus Intent, Work bzw. Status ist deshalb eindeutig.
  - Die Refill-Erkennung „`SaveIdsNeeded` im anwendenden Boundary“ ist korrekt:
    Den Pool verbraucht nur die Auswahl, höchstens einmal je Boundary, nach dem
    Refill.

**Accessor in SIM-03.4b:**
- **Für die Korrektheit nicht nötig.** Der Checkpoint-Status ist genau die
  Größe, die `dispatch_retained` prüft. Eine Refusal-Zuordnung könnte ihn nur
  wiederholen.
- **Empfohlen**, aber keine Pflicht: ein rein lesendes
  `OwnedPlayerInputRefusal::query()` bzw. eine Art-Unterscheidung. Damit wird
  `native_apply_refusal` diagnostisch (heute nur `reason()`,
  `player/pending.rs:332-340`), und Tests können ohne Umweg prüfen (R3-H2).

### B.6 Betriebliche Grenze und harter Kill

**Dauer:**
- Vor DUR-05b gilt `N_B = 0`. Ein sauberer Stop dauert dann Phase A
  (Disconnects, ≤ 4 Leave-Saves je Iteration, also bei 128 Spielern ≈ 32
  Iterationen plus Latenz) und eine kurze Phase B.
- Der ungünstigste Fall ist ein hängender Storage: 1 200 + 1 200 Iterationen
  ≈ 2 min bis `Unresolved`.
- Ab DUR-05b bestimmt die Zahl der Items die Dauer:
  - 4 096 Items: ≈ 285 s im sauberen Fall, Grenze 37 min.
  - 65 536 Items: ≈ 77 min im sauberen Fall, Grenze ≈ 7,4 h.
- Die Dauer kommt aus zwei Quellen:
  - einer Entity-Operation zugleich (DUR-04.2 Z. 339-342),
  - dem festen 50-ms-Takt auch in Phase B (Z. 474). Recovery-Boundaries claimen
    keinen Tick, der Takt ist dort also eine selbst gewählte Bremse.
- Übliche Stop-Fristen (Panel, Docker, systemd, Kubernetes) liegen im Bereich
  von Sekunden bis wenigen Minuten. Danach folgt SIGKILL.

**Harter Kill** (Quellenargument; Process-Kill und Power-Loss sind in DUR-03.3
Z. 388 und 409 ausdrücklich **nicht** abgenommen):

| Zeitpunkt | Durable-Stand | Verlust |
|---|---|---|
| Phase A | konsistent: jede Player-/Pair-/Entity-CAS ist eine redb-Transaktion mit `Durability::Immediate` (DUR-03.3 Z. 292); Uncertain bedeutet „ganz alt oder ganz neu“ | Pose, Modus und Auswahl jedes Spielers seit dem Join, falls sein Leave-Save noch nicht committed ist. Es gibt keinen periodischen Player-Autosave (Z. 397, Scan Z. 409-412). Ab DUR-05b zusätzlich der unbestätigte Counterfortschritt (deklariert in DUR-04.2 Z. 133-136). Mengen werden weder verdoppelt noch verloren. |
| Phase B | konsistent | nur der Counterfortschritt ungespeicherter Items |
| Phase C | unverändert (keine Writes) | nichts |

- Der Neustart liest den Durable-Stand über Restore.
- Die L1-Quarantäne liegt nur im Speicher und entfällt.
- `Unresolved` wird bei SIGKILL **nie** gemeldet; der Betreiber erfährt nicht,
  welche UUIDs und Items betroffen waren.

**Bewertung:**
- Das ist kein blockierender Befund. Die Konsistenz hängt nicht am geordneten
  Stop, und vor DUR-05b ist Phase B kurz.
- Eine reine DUR-05b-Messauflage reicht aber **nicht**. Der Vertrag soll
  verlangen:
  - eine konfigurierbare Gesamtfrist mit typisiertem Ergebnis,
  - die ehrliche Obergrenze aus der Formel,
  - die Erlaubnis, Phase B nicht an den 50-ms-Takt zu binden.
- Dazu muss der Crash-Verlustumfang verbundener Spieler deklariert werden
  (R3-A3, R3-A4).

### B.7 Regressionen und Ticketfolge

Ich habe den Runde-3-Kandidaten gegen Runde 4 verglichen (`diff`).

- **Unverändert tragfähig:**
  - G1: Endpointart, Exklusivität, Replacement, Unregister.
  - G2, G4 und L1/L2: Permit-Epoche getrennt von der Session-Lineage, ohne
    Bedeutungsverlust.
  - G3 mit allen Matrixzeilen. Die „Stop mit `PendingNativeJoin`“-Regel ist
    korrekt verschärft: Drain verweigert Joins, und ein vor dem Drain
    eingereihter Join wird zuerst angewandt (FIFO).
  - Session als einziger Producer für Leave-Save und Release.
  - SIM-03.4b mit Release-Gate und `lifecycle_acked` „genau dieser
    Admission“.
- **Geändert, aber korrekt:**
  - §5 Schritt 7: Enqueue-Refusal geht zurück ins FIFO statt in den Cancel.
  - §10.4 Phase A: Der Drain liegt jetzt am Anfang statt am Ende. Joins und
    Lane-Data werden ab dann von Sim verweigert. Data-Refusals verbundener
    Spieler erzeugen `Resync` (`player_ownership.rs:351-380`); das ist harmlos.
- **Quantitative Verschlechterung:** Die Lane sinkt von 16 auf ≥ 10 Items je
  Iteration (Z. 577). Das verschärft einen schon vorher bestehenden
  Durchsatzmangel (R3-A2).
- **Ticketfolge:**
  - Je Ticket eine Production-Crate. SIM-03.4b bleibt reine Sim, e1/e2 reine
    Session, DUR-05a/f reine App, ACC Testkit (R2-H7).
  - Der Graph ist zyklenfrei: .2c → e1 → e2 → {b, f}; SIM-03.4a, DUR-04.2a →
    SIM-03.4b → {e2, DUR-05a}; DUR-03.3 → 3a → DUR-05a; e1 → DUR-05a;
    DUR-05a, e2 → f → DUR-05b → ACC.

### B.8 Ehrlichkeit

- Alle gewählten Werte sind als „nicht gemessen“ gekennzeichnet (§6.1a, §6.2,
  §7.3, §10.4, §11).
- Die 4096-Item-Werte sind als Quellmodell gekennzeichnet.
- Es gibt keine Runtime-, Survival-, Plugin- oder Performance-Aussage (Z. 3-8,
  §15.10). Der Scan ergab 0 Treffer.
- Handprüfung:
  - „kein Verlust“ steht nur als Testerwartung (Z. 1436).
  - „Beweis“ in §18 (Z. 1573) meint die Quellenbegründung aus §10.4. Besser
    wäre „Begründung“ (R3-H7).
  - „der zuletzt bestätigte Durable-Stand bleibt maßgeblich“ (Z. 1333-1334) ist
    eine Designaussage. Sie stützt sich auf DUR-03.3, dort ist Process-Kill
    nicht abgenommen (R3-A4).

## C. Befunde

### Blockierend

Keine.

### Auflagen

**R3-A1 (e2/f, vor Implementierung) — Closure-Signaturen der Session-API sind
in dieser Form nicht übersetzbar.**
- **Belege:** Z. 228, Z. 828 (D7/D8); Z. 282-284, 792, 1065 (Lifetimes). Die
  Prüfung beruht auf Quellenlektüre; kompiliert habe ich nicht.
- **Doppel-Borrow:** `router.acknowledge_retired_native_records(&mut |h|
  shard.collector_lifecycle_view(h)…, &mut |t|
  shard.acknowledge_collector_lifecycle(t))` erzeugt beide Closures gleichzeitig
  als Argumente. Die eine leiht `slot.shard` unveränderlich
  (`collector_lifecycle_view(&'a self)`, `shard/player_ownership/lifecycle.rs:80-83`),
  die andere veränderlich (`acknowledge_collector_lifecycle(&mut self)`,
  `lifecycle.rs:115-118`). Das verstößt gegen die Borrow-Regeln (E0502).
- **Lifetimes:** `&mut dyn FnMut(&OwnedPlayerHandle) -> Option<…View<'_>>`
  bindet `'_` an das Handle-Argument (`for<'h>`). Die Views leihen aber den
  Shard (`'a self`, `lifecycle.rs:59-62`, `80-83`). Eine Closure über
  `&slot.shard` erfüllt diese Signatur nicht.
- **Folge:** e2 müsste die API beim Implementieren ändern; dafür gibt es keine
  Root-Vorgabe. Der Fehler stammt unverändert aus Runde 3 und ist in Review 2
  nicht aufgefallen.
- **Korrektur:**
  - D7/D8 als **eine** Closure, die View holt, Ticket kopiert (das Ticket ist
    owned, `collector_lifecycle.rs:166-169`), View fallen lässt und quittiert.
    Alternativ: Die Session liefert einen begrenzten Ticket-Auftrag zurück, der
    Driver quittiert und meldet zurück.
  - Explizite Lifetime-Parameter (`fn …<'s>(…, view: &mut dyn
    FnMut(&OwnedPlayerHandle) -> Option<View<'s>>)`).
  - Matrixzeile „kompiliert mit echtem `NativeShardSlot`“.

**R3-A2 (e1/f, Root-Entscheidung vor e1) — Bewegungsdurchsatz der Lane.**
- **Belege:** Z. 368-375, 577, Matrixzeile Z. 1437.
- **Grenzen:**
  - Die Sim nimmt je Boundary höchstens 32 Inputs an (DUR-04.1, `player.rs:51`).
  - Das Gate begrenzt die Lane auf `16 − G`, im Normal-Modus also auf 10–16 je
    Iteration, das sind 200–320 Inputs/s.
  - Der Router coalesct nicht (`router.rs:1941-1956`). Eine volle Lane führt
    für Coalescible/Authoritative zum kontrollierten Session-Leave (Z. 371-373).
- **Szenario:**
  1. Ein bewegter oder sich umsehender Client sendet etwa 20 Move-Pakete/s.
  2. Ab etwa 11 bis 16 gleichzeitig bewegten Spielern wächst die Lane.
  3. Bei 32 Spielern ist sie nach ≈ 2–3 s voll (1 008 Data-Plätze), und
     Sessions werden getrennt.
  4. Im Recovery-Modus (K = 20 ≈ 1 s ohne Lane-Fenster) gilt das schon früher.
- **Matrix:** Die Zeile „Bewegungsspam (32 Spieler)“ würde trotzdem bestehen.
  Sie prüft nur die Priorität der Completions.
- **Einordnung:** Der Mangel bestand vor Runde 4 (Lane 16); das Gate verschärft
  ihn auf ≥ 10. Er ist keine Sicherheits- oder Konsistenzfrage, aber ohne
  Abhilfe ist der Nativpfad mit mehr als etwa 10–16 gleichzeitig bewegten
  Spielern nicht spielbar.
- **Korrektur** (Root wählt):
  - Coalescing in e1: höchstens ein ausstehender Move je Session, der Driver
    liest die neueste Pose beim Entnehmen. `DeliveryPolicy::Coalescible`
    erlaubt das ausdrücklich, `ferrumc-session/src/delivery.rs:16-18`.
  - oder eine deklarierte, typisierte Spielergrenze.
  - In beiden Fällen eine Matrixzeile „N bewegte Spieler × 60 s ohne
    Session-Leave“ (ACC-Messung).

**R3-A3 (DUR-05a, Messung DUR-05b) — Stop-Frist für den Betrieb.**
- **Belege:** Z. 474, 1264-1273, 1366, 1522-1525.
- **Ergänzungen:**
  1. Eine konfigurierbare Gesamtfrist (Wall-Clock oder Boundaries) über A+B+C.
     Läuft sie ab, folgt `Unresolved{reason: StopDeadline}` mit
     UUID-/Pending-/Itemzahlen und Log, bevor der Prozess endet. Dazu ein
     Hinweis, dass externe Stop-Timeouts größer als diese Frist sein müssen.
  2. §15.12 soll die echte Formelgrenze nennen: bis
     `12 000 + 8·65 536 = 536 288` Boundaries ≈ 7,4 h; sauberer Stop bei
     65 536 Items ≈ 77 min (B.4).
  3. Phase B darf Recovery-Boundaries bei Eingang einer Completion statt im
     50-ms-Takt ausführen (kein Tick, kein Catch-up). Zulässig mit
     DUR-05b-Messung.
- Die Werte bleiben „nicht gemessen“.

**R3-A4 (f bzw. DUR-05b; Deklaration jetzt) — Crash-Verlustumfang verbundener
Spieler.**
- **Belege:** Z. 397, 409-412, 1330-1335, §12, §15.
- **Befund:** `autosave` ist nur als Leave-Save erlaubt; der Source-Scan
  verbietet ihn in der App. Ein periodischer Save verbundener Spieler existiert
  nicht.
- **Folge:** Bei Kill oder Crash, und bei jedem Neustart aus `Degraded`, gehen
  Pose, Modus und Auswahl seit dem Join verloren. Vanilla speichert periodisch
  (`saveAll`). DUR-05 verlangt, Crash und sauberen Stop zu unterscheiden.
- **Korrektur** (eine der beiden Wege):
  - periodischer gated Klasse-2-Autosave aus der Session mit Driver-ID und
    Ticketzuordnung,
  - oder eine ausdrückliche Abweichung in §12/§15 mit Root-Zustimmung.
- Dazu eine Matrixzeile „Kill in Phase A/B/C → Neustart → Restore liefert
  konsistenten letzten Commit, keine Mengenänderung“, gekennzeichnet als
  Cutpoint-Test, nicht als Power-Loss-Abnahme.

**R3-A5 (DUR-05a/b) — Quarantiniertes Entity-Pending führt sicher in
`Degraded`.**
- **Belege:** Z. 517-523, 721.
- **Mechanik:**
  - `Refused(CasConflict)` im Counter-Modus setzt `Quarantined` und behält
    `pending.counters` (`player_ownership.rs:1610-1614`,
    `counters.rs:437-450`).
  - Der Plan bindet die Projektion jedes Pendings mit `counters` ohne
    Statusfilter (`counters.rs:494-499`) und zählt `claims` je residentem
    Normal-Boundary hoch (`2090-2095`, Rückschrieb `1781-1806`).
  - DUR-04.2 Z. 604-606 will das so.
- **Folge:** Nach höchstens `limits.defer` (≤ 1 024) Normal-Boundaries folgt
  dauerhaft `CounterRecoveryRequired`. §6.5 erlaubt für `Quarantined` kein
  Input, deshalb geht der ganze Shard nach F_max dauerhaft in `Degraded`.
- **Lücke im Brief:** Die Fallanalyse in §6.1a („verschwindet genau dann …
  Completion, Retry oder Reconcile passt immer“) deckt diesen Fall nicht ab.
- **Korrektur:** in §6.1a und §15 ausdrücklich nennen, dazu eine Matrixzeile
  „Entity-CasConflict → nach ≤ defer Boundaries Recovery → `Degraded`; Stop →
  `Unresolved`“. Ob ein einzelner Item-Konflikt den Shard stoppen darf,
  entscheidet Root.

**R3-A6 (DUR-05a) — Disposition eines gescheiterten Recovery-Boundaries.**
- **Belege:** Z. 484-493, 706.
- **Mechanik:** `recover_player_ownership_completions` kann **vor** dem
  Queue-Take mit `Err` enden (`counters.rs:249-266`):
  - `preflight_counter_completion` liefert `Capacity("confirmed item bytes")`
    oder `GenerationExhausted`,
  - `generation.next()?` liefert `GenerationExhausted`.
- **Folge:** Die Completion bleibt samt Lease in der Sim-Queue. Ein
  Normal-Versuch scheitert am selben Preflight (`counters.rs:539`).
- **Lücke im Brief:** Er regelt nur Normal-Fehler. Offen ist, ob ein
  gescheiterter Recovery-Boundary als Boundary ohne Fortschritt für K und den
  Wachhund zählt und wann er zum Fail-Stop führt.
- **Korrektur:** `GenerationExhausted` → `Degraded{terminal}`, zählt als ohne
  Fortschritt; andere Fehler → Fail-Stop `Unresolved`. Dazu eine Matrixzeile.
- Vor DUR-05b ist der Fall praktisch unerreichbar.

### Hinweise

- **R3-H1:** In `OutstandingWork` fehlt ein Zustand „Retry/Reconcile
  eingereiht“ (Z. 699-703). Doppelte Einreihung ist harmlos (`Reserved`, B.5),
  verfälscht aber die Klärungszählung und kostet Plätze.
- **R3-H2:** Ein rein lesender Accessor `OwnedPlayerInputRefusal::query()`
  bzw. eine Art ist empfohlen, aber keine Pflicht (B.5).
- **R3-H3:** `player_ownership_checkpoint` kann außer `CompletionMismatch`
  auch Validierungsfehler liefern (`checkpoint.validate()?`,
  `player_ownership.rs:697`). Zu regeln ist: alles außer `CompletionMismatch`
  → Fail-Stop.
- **R3-H4:** Für Enqueue-Refusals von `retry`, `reconcile`, `drain` und `stop`
  fehlt eine Disposition (Z. 661-666). Vorschlag: nächste Iteration erneut.
  Bei `drain` in Phase B genügt das, weil P nur verlangt, dass der Drain vor
  dem Ende von Phase B angewandt wird; in Phase B gibt es keinen Counterschritt.
- **R3-H5:** §6.2 nennt „zuerst Refill … und Drain“ (Z. 600), §10.4 dagegen
  „Drain das erste Input“ (Z. 1234). Beides ist unschädlich; der Wortlaut
  sollte aber gleich sein.
- **R3-H6:** In Phase A erzeugen verweigerte Lane-Data verbundener Spieler
  `Resync`, also `SynchronizePlayerPosition` an Clients, die gerade getrennt
  werden. Das ist harmlos und sollte als erwartet gelten.
- **R3-H7:** „Invariante P mit Beweis“ (Z. 1573) besser „mit
  Quellenbegründung“.

## D. Empfehlung an Root

**CLEAN_CONTRACT für NET-04.2d.** Die Blocker aus Runde 2 sind behoben, ohne
dass tragfähige Teile beschädigt wurden. Bindung der Auflagen:

| Auflage | Wohin | Wann |
|---|---|---|
| R3-A1 | Vertragstext bzw. e2-Ticket | vor Implementierung von e2 |
| R3-A2 | Root-Entscheidung, Coalescing oder deklarierte Grenze | vor Freigabe von e1 |
| R3-A3, R3-A6 | DUR-05a | — |
| R3-A4 | Deklaration im Vertrag sofort; Producer in f oder DUR-05b | — |
| R3-A5 | §6.1a/§15 plus Root-Entscheidung | vor DUR-05b |

SIM-03.4b und NET-04.2e1 hängen an keinem Blocker. Für e1 gilt R3-A2, für e2
R3-A1.

Diese Prüfung ist eine Vertragsprüfung. Sie nimmt weder Runtime, Survival,
Paper-Parität, Plugins noch Performance ab.
