# NET-04.2d — unabhängige Vertragsprüfung, Runde 2

Stand: 11.10.2026. **Verdikt: FINDINGS (kein CLEAN_CONTRACT).**

B3 ist behoben. B1 und B2 sind nur teilweise behoben. Die Nachbesserung bringt
drei neue blockierende Fehler mit, alle im Bereich Counter-Boundary und Stop:

- **N1:** Der Recovery-Modus kann sich selbst blockieren.
- **N2:** Das Stop-Protokoll A/B/C erreicht auch bei sauberem Ablauf nicht sicher
  `Stopped`.
- **N3:** Beim Apply abgelehnte Completions werden erneut eingereiht. Ihr Lease
  blockiert damit Retry und Reconcile auf Dauer.

Jeder der drei Fehler lässt sich mit wenigen Sätzen im Vertrag beheben. Dazu
kommen sechs Auflagen und sieben Hinweise. Der Endpoint- und Lane-Kern (G1), G2,
G4, G3 mit Matrix, der Session-Producer für Leave-Save und Release sowie die
SIM-03.4b-APIs sind tragfähig.

Geprüft wurde der eingefrorene Kandidat. Er wurde nicht verändert.

| Gegenstand | SHA-256 |
|---|---|
| Brief `NET-04_2d_NATIVE_DRIVER_ENDPOINT.md` (1088 Zeilen) | `900bc35c05e3d8cae769dd81ba918653faa14e3693c516351a71c19c74f9c8db` |
| `.codex-tmp/net04_2d/design.freeze.json` | `6c6ac6884cc10bae9cea9e9e8376541898f0796c862a9450506d8d7e2314fef0` |
| Review Runde 1 / `final-assessment.json` | `7ece2449…1d16` / `e46c5da2…e19f0` |
| Workorder | `6919a9b7…5c3ceb5c` |
| Basis-Manifest (394d4c27, 1274 Dateien) | `4c964d711c1dd4499bf914327266134c734f8aa13aefe9010a5dab27d80f519b` |

Code wurde nur aus `.codex-tmp/net04_2d-root-base/archive` gelesen. Die
Root-Entscheidungen G1–G5, L1/L2 und die der Runde 3 (B1/B2/B3, R3-1..3) habe
ich im Runde-3-Thread nur lesend nachgelesen. Zeilenangaben „Z.“ beziehen sich
auf den Brief, Pfade ohne Präfix auf `server/crates/ferrumc-sim/src/` der Basis.

## A. Ausführung (reproduziert)

- `round3/pins.py` und `round3/freeze.py` liefen in einer privaten Kopie
  (`.codex-tmp/net04_2d-root-review-r2/repro/`); geändert war nur `OUT`.
  Ergebnis: Exit 0, 137/137 Anker mit Token auf der Startzeile, 0
  Basisabweichungen, 1274/1274 Dateien readonly, 56/56 SIM-03.4a-Dateien,
  25/25 .2c-Pins, Paper-HEAD `29c8822d…`, alle neun Nachbar-Hashes
  identisch.
- `baseline.verify.json`, `producer-pins.json`, `reference-pins.json` und
  `design.freeze.json` sind **byteidentisch** mit der Kandidatenevidenz.
- Die ersten 87 Anker von `round3/pins.py` sind listengleich mit den 87 aus
  Runde 2; keiner fehlt.
- Forbidden-Claims-Scan mit `markdown_blocks` und `scan` aus
  `scripts/check-forbidden-claims.py` (`eb2671f2…40ff`), direkt auf Brief und
  `round3/EVIDENCE.md`, jeweils normal und mit `force_plugin`: **0 Treffer**
  (`forbidden-scan.json`). Die Muster sind englisch. Deshalb habe ich deutsche
  Formulierungen zusätzlich von Hand geprüft: „kein Verlust“ (Z. 990) steht nur
  als Testerwartung da, alle gewählten Werte sind als „nicht gemessen“
  gekennzeichnet (Z. 432, 645, 656, 720, 875-882, §11).
- Keine Builds, Tests, Server oder Clients wurden gestartet.

## B. Quellenprüfung (Source-Review, keine Runtime-Abnahme)

### B.1 Neue Anker von Hand

Alle **50 neuen** Anker habe ich geöffnet und gegen die Aussage im Brief geprüft
(`manual-anchor-check.json`). 45 stimmen, 4 nur teilweise, 1 ist falsch:

- **falsch:** `counters.rs:174` (Z. 88-89 „jeder Boundary erhöht die shardweite
  lokale Generation“). Die Zeile prüft nur vorab, ob `next()` möglich ist, und
  verwirft den Wert. Die Generation wächst nur über `next_generation` bzw. bei
  der Veröffentlichung (`player_ownership.rs:928`, `1718`). Die Schätzung in
  §6.1 bleibt dadurch konservativ; das hat keine Sicherheitsfolge (R2-H1).
- **teilweise:** `player_ownership.rs:1594-1629` (R2-H4), `counters.rs:872-909`
  (Stop muss das letzte Nicht-Completion-Input sein, siehe N2),
  `collector_lifecycle.rs:274` (R2-H3) und `evidence.rs:316-323` (R2-H2).

### B.2 Runde-1-Befunde

| Befund | Status | Begründung |
|---|---|---|
| **B1** Counter-Modus | **teilweise behoben** | Umgesetzt sind: Init (§10.2), `_at` mit `NativeTickClock` (Z. 361-373), Recovery-Boundary, Refill nur aus `counter_owner` (Z. 304, 311-322), `SaveIdsNeeded` mit höchstens einem Refill in flight (Z. 469-483) und der Source-Scan. Die Recovery-Disposition (N1), die Stop-Quiescence (N2) und die Refusal-Behandlung (N3) sind fehlerhaft. Damit sind die Root-Teilauflagen „Recovery-Boundary“, „CounterRecoveryRequired“ und „Stop-Quiescence“ nicht tragfähig erfüllt. |
| **B2** Leave-Save-/Release-Producer | **teilweise behoben** | Der Widerspruch ist aufgelöst: Es gibt genau einen Producer in der Session, der Record hält das Original-Handle bis zum Release, der Driver hat nur ein Token, der Release läuft über einen Closure, Retries sind begrenzt und danach greift eine typisierte Quarantäne (§7.1-7.5). Offen sind zwei Punkte. Im Runde-1-Szenario fehlt die Transition `Due` + fremdes `blocking` (R2-A1). Außerdem verhindert N3, dass Busy- oder Uncertain-Leave-Saves je per Retry bzw. Reconcile aufgelöst werden. |
| **B3** G3-Race-Matrix | **behoben** | Matrixzeilen Z. 1003-1010 decken (a)–(d) ab, dazu Verbindungsende zwischen Schritt 5 und 10, kapazitätsunfehlbare Adoption, Adoption-Invariantenbruch und Stop mit `PendingNativeJoin`. Endpoint-Replacement während G3 deckt die Unregister-Zeile Z. 989 ab. |
| A1 | aufgenommen, Inhalt fehlerhaft | Die Zuordnung (DUR-05a/f) stimmt. Die Begründung Z. 419-427 hält aber nicht (N1, N3). |
| A2–A8, A10 | aufgenommen und inhaltlich korrekt | Zuordnung in §13. A7: die Countershadow-Formulierung ist ungenau (R2-H3). |
| A9 | aufgenommen, widersprüchlich | §6.5 (Retry/Reconcile) widerspricht §6.3 (N3). |
| H1–H4 | umgesetzt | H1 ist gepinnt, H2 gekennzeichnet, H3 steht als Abweichung 6 in §12, H4 ist e1 zugeordnet. |

### B.3 Szenario aus Runde 1: Disconnect mit laufendem Pickup

Ich spiele das Szenario gegen den neuen Text und den Code der Basis durch.

1. Spieler A ist Active, sein Pickup P ist pending. Die Connection endet. D1:
   L1 wandert in den Driver.
2. D2: `disconnect_native(key)` setzt die Session auf Leaving und baut
   `disconnect(original)` als Control-Item auf der Lane; das Token geht an den
   Driver.
3. Das Lane-Fenster reiht das Item ein. Im Normal-Boundary läuft
   `apply_native_disconnect` (`lifecycle.rs:178-206`). Es prüft kein Pending;
   `connected` wird false und der Fact wird versiegelt. Es folgt
   `PlayerDetached{A}` (`player_ownership.rs:557-563`).
4. D3: `native_detached(A, |h| collector_lifecycle_view(h))` setzt den Record
   auf Detached. D5 retiriert ihn mit EntityRemove vor TabRemove.
5. D4: Der Status liefert `saved_after_detach = None` und
   `blocking = (P, InFlight)`. **Hier fehlt eine Regel.** Z. 594 lässt Submit nur
   bei `blocking = None` zu, Punkt 5 behandelt nur `Waiting`. Ohne die
   Transition `Due` → `Waiting` zählt kein Versuch (R2-A1).
6. Gehen wir davon aus, dass die Transition nach `Waiting` führt. P wird
   bestätigt: `publish_confirmed` erzeugt `PlayerChanged`
   (`player_ownership.rs:1700`) und `apply_deferred` macht den Spieler dirty
   (`1976-2029`). Der Driver findet keine View und ruft
   `native_detached_intent`; der Record geht auf `Due`.
7. Submit `autosave(original, id)`. `plan_owned_player_write` verlangt weder
   `dirty` noch `connected` (`1010-1064`); das Pending wird zugelassen und der
   Record geht auf `Pending`.
8. `Confirmed{Ok}` setzt `saved_after_detach` (SIM-03.4b). Daraus folgt
   `SaveConfirmed`, dann D7/D8 (Zelle frei, `collector_lifecycle.rs:525-526`),
   D9 gibt L1 frei. Das Release-Gate ist erfüllt; Record und Token werden
   entfernt, L2 ist frei.

**Ergebnis:** Mit der Transition aus R2-A1 läuft die Kette durch, ohne dass die
UUID dauerhaft `AlreadyReserved` bleibt. Ist P dagegen Busy oder Uncertain, wird
er wegen N3 nie per Retry oder Reconcile aufgelöst. Der Record endet dann nach
etwa 900 Boundaries in Quarantäne, statt den Release zu erreichen.

### B.4 Stop im Counter-Modus

Phase A läuft mit Normal-Boundaries; das Drain ist dort enthalten. Phase B nutzt
nur Recovery-Boundaries und endet, wenn `owned_stop_ready()` wahr ist. Phase C
reiht Stop ein und führt einen Normal-Boundary aus. Die Quiescence verlangt eine
leere Queue, Zellen = 0, kein Pending, saubere Spieler und
`counter_quiescent` (`player_ownership.rs:701-720`,
`entity/counters.rs:248-253`).

Dass `owned_stop_ready` über denselben Helfer läuft, ist konsistent mit dem
Apply-Check des Stop (`player_ownership.rs:578`). Unabhängig davon prüft der Plan
eigene Bedingungen (`counters.rs:872-909`, inklusive `remaining == 0`).
Gegenproben siehe N2.

### B.5 SaveIdsNeeded unter Last

Die Sim meldet höchstens einen Intent je Boundary, und nur bei leerem Pool
(`counters.rs:1941-1952`). Der Refill wird im selben Boundary vor der
Save-Auswahl angewandt (`run_player_ownership_boundary` vor
`select_counter_save`, `counters.rs:212`, `231`), auch in Recovery
(`counters.rs:284`, `294`). Mit höchstens einem Refill in flight und der
Poolgröße `limits.save_ids` greift keine Capacity-Refusal
(`counters.rs:109-116`). Ein voller Queue-Platz macht den Refill „unsent“; der
nächste Intent stößt ihn erneut an. Dirty bleibt gesetzt (DUR-05 Z. 46).

Das ist konsistent, aber nur ohne N1/N3. Unter N3 belegen kreisende Completions
die Klasse-1-Plätze vor dem Refill. Unter N1 kommt der Refill gar nicht mehr in
die Queue.

### B.6 Weitere Prüffragen

- **Wartender Record ohne Versuchszählung:** Begrenzt ist er nur über Punkt 5
  (Uncertain je 100, sonst nach 1200 Boundaries je 100) und den Zähler
  `applied`, der nur in Normal-Boundaries wächst. Unbegrenzt hängt ein Record
  (a) in `Due` mit fremdem `blocking`, weil dafür keine Regel existiert
  (R2-A1), und (b) in einem dauerhaften Recovery-Modus (N1), weil `applied` dort
  steht. Ein Stop beendet beides spätestens mit `Unresolved`.
- **A1 gegen die Count-Grenzen:** Die Grenze `inputs ≤ 32` wird gemeinsam
  geprüft (`player_ownership.rs:258-260`). Im Recovery-Modus bleiben alle
  Nicht-Recovery-Inputs liegen (`counters.rs:277-293`). Lane 16 von 32 allein
  schützt nicht, denn Klasse 2 enthält ebenfalls nicht anwendbare Inputs (N1).
  Completions zuerst plus Wiedereinreihen abgelehnter Completions blockiert sich
  selbst (N3).
- **Tick-Identität:** konsistent mit DUR-04.2 Z. 225-231 und DUR-05 Z. 38-42.
  Baseline `Tick::ZERO` auf leerem Owner, strikt `last+1`, Fortschreiben nur
  bei `Ok`, Recovery zählt nicht. Persistiert werden keine Ticks, auch
  `saved_tick` nicht.
- **UUID v4 und DUR-05:** vereinbar. `OwnershipTransferId::from_bytes` ist
  öffentlich (`ferrumc-storage/src/ownership.rs:207-213`), `uuid` mit `v4` ist
  App-Dependency (`app/src/operators.rs:359`). Die durable Kollision entscheidet
  der Receipt (DUR-04.2 Z. 557-558, DUR-03.3 Z. 149-151).
- **Kein Spieler-Handle im Driver:** Für L2, Release, Status, G3 und Stop ist
  das erfüllt. Handles aus Intents sind nur transient; `held` enthält ein
  opakes `OwnedPlayerInput` ohne Accessor auf das Handle. **Nicht erfüllbar**
  ist die Revalidierung nach §4.3 (R2-A2).
- **Adoption kapazitätsunfehlbar:** auf der Session-Seite ja (Eintrag ab Schritt
  6, entspricht .2c Z. 461-469 „Retirementslot je NativeJoin“). Ende zu Ende
  fehlt die Vorreservierung der Driver-Token-Tabelle (R2-A3).
- **SIM-03.4b-APIs:** minimal und ohne Autoritätsleck.
  `detached_owned_player_status` und `owned_stop_ready` sind rein lesend. Die
  sichtbare `OwnershipReceiptQuery` ist Daten, keine Autorität
  (Rustdoc `player/evidence.rs:286` „data rather than publication authority“).
  Retry und Reconcile prüfen zusätzlich die Lease. Der Release ist nur im
  Session-Closure erreichbar (Scan Z. 316-317). Offen sind Typinhalt und
  Crate-Grenze (R2-H5, R2-A4) sowie die Erkennung „Zelle acked“ (R2-H6).

## C. Befunde

### Blockierend

**N1 — Der Recovery-Modus kann sich selbst blockieren** (A1/B1; neu in Runde 3).
Belege im Brief: Z. 380, Z. 409-427, Z. 918, Matrix Z. 992.

- *Beleg im Code:*
  - `_at` scheitert vor jeder Mutation, die Queue bleibt intakt
    (`counters.rs:185-204`, vor `212`).
  - Recovery wendet nur Completion, Reconcile, Retry, Drain und Refill an und
    reiht alles andere zurück (`counters.rs:277-293`).
  - Alle Inputs teilen die Grenze 32 (`player_ownership.rs:258-260`).
  - `CounterRecoveryRequired` entsteht am Defer-Cap eines Entity-Pendings
    (`counters.rs:2090-2095`). Auflösen kann ihn nur dessen Completion, also
    Reconcile oder Retry plus Completion. Beide brauchen einen Queue-Platz.
- *Szenario a (Queue blockiert):*
  1. Im Normal-Modus reiht der Driver ein: 0 Completions (Work verloren bzw.
     Uncertain), 16 Klasse-2-Inputs (≤ 8 Retention-Submits plus Join-Inputs
     ≤ offene Reservationen, Z. 918) und 16 Lane-Items. Die Queue ist mit 32
     Inputs voll.
  2. `_at` scheitert mit `CounterRecoveryRequired`.
  3. Im Recovery-Modus ist die Lane zu, aber Completions und `reconcile` werden
     mit `Capacity("input count")` verweigert.
  4. Recovery wendet nichts an. Nach Z. 380 folgt ein Normal-Versuch nur, wenn
     eine Completion angewandt wurde; das geschieht hier nie.
  5. Klasse 2 ist im Recovery-Modus zudem nicht eingeschränkt. Neue
     Retention- oder Join-Inputs sammeln sich deshalb auch dann an, wenn
     anfangs Platz war.
- *Szenario b (keine Completion möglich):* `AllocationFailed` oder
  `Capacity("counter plan")` tritt auf, wenn der Betreiber `plan_entries`
  gesenkt hat (`needed` zählt `input.len()` mit, `counters.rs:470-487`). Ist
  kein Pending ausstehend, kommt nie eine Completion; Z. 380 setzt dann nie
  wieder einen Normal-Boundary an.
- *Folge:* Der Shard macht dauerhaft keinen Fortschritt: keine Joins, Leave-Saves
  zählen nicht. Der Stop endet mit `Unresolved`, weil Phase B eine leere Queue
  braucht. Die Matrixzeile Z. 992 prüft nur Lane-Items und würde bestehen.
  Szenario a ist erst mit Entity-Pendings erreichbar (DUR-05b), Szenario b
  jederzeit.
- *Mindestkorrektur:*
  - Alle nicht recovery-anwendbaren Inputs (Lane plus Klasse-2-Join, Retention
    und Stop) gemeinsam auf ≤ 16 Plätze der Sim-Queue begrenzen.
  - Im Recovery-Modus nur Completion, Retry, Reconcile, Refill und Drain
    einreihen.
  - Normal-Wiederholung auch ohne angewandte Completion ansetzen (z. B. wenn
    kein Pending aussteht oder nach N fortschrittslosen Recovery-Boundaries).
  - Matrixzeilen für Szenario a und b.

**N2 — Das Stop-Protokoll A/B/C erreicht bei korrektem Ablauf nicht sicher
`Stopped`** (R3-1/B1; neu). Belege im Brief: Z. 875-906, Matrix Z. 1025-1026.

- *(a) Priority-Verlust bei eingefrorenem Tick:*
  - Beleg:
    - `install_confirmed` setzt `priority = false`
      (`entity/counters.rs:98-105`).
    - `restore_counter_flush` setzt sie nur wieder, wenn bei der Bestätigung
      `generation ≠ saved` gilt (`entity/counters.rs:254-259`,
      `counters.rs:408-420`).
    - Im selben Normal-Boundary folgt danach der Counterstep
      (`counters.rs:212-221`); Age++ macht das Item dirty (Z. 104-106).
    - `drain()` markiert die Priority nur zum Zeitpunkt seines Apply
      (`player_ownership.rs:567-575`, `entity/counters.rs:262-273`).
    - Phase B claimt keinen Tick. Die Normal-Lane verlangt
      `tick − saved_tick ≥ save_period` (`counters.rs:1917-1923`; Default 100,
      DUR-04.2 Z. 196).
  - Szenario: Phase A wendet `drain()` an (Z. 878) und läuft weiter, etwa für
    Final-Save oder Acks. Item X wird bei Tick t per Priority gespeichert:
    `priority = false`, sofort ein Age-Step, X ist dirty. Phase A endet bei
    t+5. In Phase B findet die Priority-Lane X nicht, und die Normal-Lane
    erreicht `tick − saved ≥ 100` nie. `counter_quiescent` bleibt falsch.
  - Folge: Nach 12 000 Recovery-Boundaries endet der Stop mit `Unresolved`, und
    die UUIDs gehen in L1-Quarantäne, obwohl alles speicherbar wäre. Die
    Matrixzeile Z. 1026 behauptet dagegen `Stopped`.
- *(b) Nicht anwendbare Inputs in Phase B:* `ownership_quiescent` verlangt
  `input.is_empty()` (`player_ownership.rs:707`), und Recovery wendet
  Autosave, Disconnect und Join nicht an. Der Brief verbietet in Phase B aber
  weder Retention-Submits (Z. 589 „je Boundary“) noch den G3-Disconnect
  (Z. 765-767) noch Join-Inputs. Ein einziges solches Input macht
  `owned_stop_ready` dauerhaft falsch.
- *(c) Reihenfolge in Phase C:* Der Plan nimmt Stop nur an, wenn kein
  Nicht-Completion-Input folgt (`remaining == 0`, `counters.rs:794-805`,
  `896`). §6.2 reiht Retention-Inputs nach Drain/Stop ein (Z. 413-415) und
  danach im Normal-Modus bis zu 16 Lane-Items (Z. 416-417). Wird Stop
  abgelehnt, läuft der Counterstep, alle Items sind wieder dirty, und der
  Ablauf fällt zurück nach Phase B, wo wieder (a) greift.
- *Mindestkorrektur:*
  - Phase B beginnt mit einem erneuten `drain()`. Recovery wendet Drain an und
    markiert dabei alle Slots neu. Alternativ: Drain muss im letzten
    Normal-Boundary von Phase A liegen.
  - Ab Phase B nur recovery-anwendbare Inputs; der Retention-Service reicht
    nichts mehr ein.
  - Phase C: Lane 0 und keine Retention; Stop ist das letzte Nicht-Completion-
    Input.
  - Matrixzeilen für (a)–(c).

**N3 — Beim Apply abgelehnte Completions werden zurückgehalten und erneut
eingereiht** (A1/A9/B1; neu, `held_completions` fehlt im Runde-2-Kandidaten).
Belege im Brief: Z. 453-454 „Completions → `held_completions`“ zusammen mit
Z. 409-412, im Widerspruch zu Z. 494-499.

- *Beleg im Code:*
  - `complete_owned` gibt bei Unresolved, ReceiptAbsent, jedem Refused (auch
    Busy), `Confirmed{Err}` und Quarantäne `Err` zurück
    (`player_ownership.rs:1594-1641`).
  - `apply_owned_input` legt die Completion dann in `outcome.refusals`
    (`337-349`).
  - Die Completion trägt die Lease `buffer: Arc<()>` (`player/evidence.rs:280-284`).
  - `dispatch_retained` verweigert Retry und Reconcile, solange die Lease lebt
    (`player_ownership.rs:1390-1392`).
  - Nachbarverträge: DUR-04.1 Z. 475-477 („refused Completionobjekt hält diese
    Lease und verhindert eine zweite Workauslieferung“) und DUR-04.2
    Z. 606-609 („Erst Drop dieses refused Inputs ermöglicht den echten exakten
    Retry“).
- *Szenario:*
  1. Ein Leave-Save läuft auf Busy: Pending Prepared, die Completion kommt als
     Refusal zurück und landet in `held_completions`.
  2. Die nächste Iteration reiht sie in Klasse 1 wieder ein; sie wird erneut
     abgelehnt, und so weiter.
  3. `retry(query)` nach Z. 499 ist immer `Reserved`. Uncertain-Completions
     erlauben ebenso nie ein Reconcile. Definitive Refusals kreisen als
     `CompletionMismatch` weiter.
- *Folge:* Bis zu 32 tote Completions belegen dauerhaft die vorderen
  Queue-Plätze. Kein Busy- oder Uncertain-Pending wird je aufgelöst. Leave-Saves
  enden in Quarantäne, der Stop wird nie quiescent und endet mit `Unresolved`.
- *Mindestkorrektur:* `held_completions` nur für Enqueue-Refusals verwenden
  (DUR-04.1 Z. 563-565). Apply-Refusals droppen, damit die Lease frei wird;
  danach Retry oder Reconcile je nach Pending-Status. Matrixzeile „Busy →
  Drop → Retry bestätigt“.

### Auflagen (vor bzw. in den Runtime-Tickets)

- **R2-A1 (e2; B2-Rest) — Leave-Save-Zustandsmaschine vervollständigen.**
  - `Due` mit `status.blocking = Some(fremd)` braucht eine eigene Transition
    nach `Waiting{since, blocking}` (Z. 594-613); siehe B.3, Schritt 5.
  - Ein Fehler der Status-Closure (StaleHandle, Foreign, Snapshot) braucht eine
    typisierte Disposition.
- **R2-A2 (e1/e2; fehlende .2c-Ergänzung) — Revalidierung ohne Driver-Handle.**
  - Problem: Z. 254-257 verlangt den Aufruf
    `revalidate_native_endpoint(key, &slot.lane, &view)` mit einer View aus
    `slot.shard`. Eine View entsteht nur aus einem `OwnedPlayerHandle`
    (`lifecycle.rs:59-76`), und der Driver darf keines halten (Z. 318-319).
    So ist die Revalidierung nicht implementierbar.
  - Lösung: eine closure-basierte Revalidierung nach dem Muster R1/R2, etwa
    `revalidate_native_endpoints(&lane, &mut |h| shard.connected_owned_player_view(h))`,
    markiert als „.2c-Ergänzung R4“.
- **R2-A3 (f) — Adoption Ende zu Ende kapazitätsunfehlbar machen.**
  - Die Driver-Token-Tabelle für L2 (Z. 652) beim Start fallibel auf das
    Sessions-Limit vorreservieren; nach D2 bzw. Adoption wird nur eingetragen
    (Z. 571).
  - Festlegen, wer die Lineage vergibt. Z. 534-535 nennt den Driver-Zähler, die
    Session-API (Z. 544, 738-740) bekommt ihn aber nicht.
- **R2-A4 (e2/SIM-03.4b; Crate-Graph und Ticketabhängigkeit).**
  - Problem: Die Session-Signaturen nennen Storage-Typen: `OwnershipTransferId`
    (Z. 550, `Submitted{id}`) und `OwnershipReceiptQuery` in
    `DetachedOwnedPlayerStatus` (Z. 687-688). Beide sind in ferrumc-storage
    definiert (`ownership.rs:207`, `605`). ferrumc-sim exportiert sie nicht weiter
    (`lib.rs:136-140`, `player.rs:25-30`), und ferrumc-session hat keine
    Storage-Dependency (Crate-Map: „NEVER raw storage“).
  - Lösung: Re-Export in SIM-03.4b oder begründete Dependency festlegen.
  - Zusätzlich die Voraussetzung **e2 → SIM-03.4b** in §13 ergänzen (Z. 964);
    e2 benutzt `DetachedOwnedPlayerStatus` und `ReleasedOwnedPlayer`.
- **R2-A5 (f) — Rückgabeweg für `held` festlegen.**
  - Problem: Z. 234 sieht `Option<NativeShardInput>` vor. `into_input()`
    verbraucht den Wrapper, und eine Enqueue-Refusal gibt nur
    `OwnedPlayerInput` zurück (`player_ownership.rs:248-286`).
  - FIFO je Session und das Verwerfen bei detached Sessions (Z. 288-291)
    brauchen aber den Sessionkey. Festlegen: Rewrap durch die Session oder
    `held` mit Key.
- **R2-A6 (DUR-05b) — Phase-B-Grenze gegen Save-Durchsatz neu bewerten.**
  - Durchsatz: höchstens eine Entity-Operation je Boundary
    (`counters.rs:1884-1886`), Priority-Lane in 3 von 4 Turns (`1898`).
  - Folge: 4096 dirty Items brauchen mindestens etwa 5462 Recovery-Boundaries.
    Ab etwa 2,2 Boundaries Latenz je Save überschreitet ein regulärer Stop
    12 000 Boundaries.
  - Vor DUR-05b gibt es keine Items. Die Werte sind nicht gemessen; die Grenze
    muss vor DUR-05b bewertet werden.

### Hinweise

- **R2-H1:** Die Aussage zu `counters.rs:174` stimmt nicht (siehe B.1).
- **R2-H2:** Z. 496-497 nennt `into_receipt_clarification`. Reconcile läuft
  tatsächlich über `dispatch_retained(clarify = true)`
  (`player_ownership.rs:1373-1413`), das ebenfalls eine freie Lease braucht.
- **R2-H3:** Z. 707-709 „entfernt … auch die `LifecycleCredits`-Kopie im
  Countershadow“ ist irreführend. Die Kopie entsteht je Plan
  (`counters.rs:510`), und die Zelle ist schon beim Ack frei
  (`collector_lifecycle.rs:525-526`).
- **R2-H4:** §2.7 (Z. 78-80) nennt die Counter-Modus-Disposition
  CasConflict → Quarantined (`player_ownership.rs:1610-1614`) und die
  definitive Refusal → `release_pending` + Resync (`1615-1619`) nicht. §7.3
  verwendet sie aber.
- **R2-H5:** Der Inhalt von `ReleasedOwnedPlayer` (Z. 683) ist offen.
  Festlegen, dass er keine Autorität trägt.
- **R2-H6:** „Zelle acknowledged“ lässt sich in der Sim nur als „keine Zelle
  mehr“ beobachten (`collector_lifecycle.rs:425-438`, `525`). SIM-03.4b muss das
  vom Fall „nie eine Zelle“ unterscheiden, etwa über den Pflicht-Lifecycle-Modus.
- **R2-H7:** Die ACC-Zeile (Z. 972) nennt zwei Crates (testkit/xtask); laut
  CLAUDE.md sind Primary-Crate plus testkit erlaubt.

## D. Root-Auflagen der Runde 3

| Auflage | Ergebnis |
|---|---|
| B1 Init / `_at` / Refill aus Counter-Handle / ≤ 1 Intent je Boundary / Ticketzuordnung | erfüllt |
| B1 Recovery-Boundary und `CounterRecoveryRequired` | Text vorhanden, Disposition fehlerhaft (N1, N3) |
| B1 Stop-Quiescence | nicht tragfähig (N2) |
| B2 Producer in der Session, Record bis Release, Token, Closures, Retries begrenzt, Quarantäne, Grenzen | erfüllt |
| B2 „Reserved bei laufendem Pickup“ | Ablauf beschrieben, Transition fehlt (R2-A1) |
| B3 alle G3-Races mit Matrixzeile | erfüllt |
| A1–A10 als Ticketauflagen | zugeordnet; A1/A9 inhaltlich fehlerhaft (N1, N3) |
| R3-1: `owned_stop_ready` mit gleichem Helfer, Phase B begrenzt, `Unresolved` + Quarantäne, Pickups in Phase B, terminale Grenze | formal erfüllt, Ablauf nicht tragfähig (N2) |
| R3-2: `saved_after_detach` von der Sim gesetzt und zurückgesetzt, Status rein lesend, typisierte Refusals | erfüllt |
| R3-3: Platz ab Reservation, Freigabe bei Abbruch, fatale typisierte Invariantenbrüche, kein unbemerktes Handle, Matrixzeilen | auf Session-Seite erfüllt; Driver-Seite R2-A3 |

## E. Ehrlichkeit und Ticketfolge

- Werte sind als „nicht gemessen“ gekennzeichnet. Es gibt keine Runtime-,
  Survival-, Plugin- oder Performance-Aussage (Z. 3-8, §15). Format7-APIs werden
  nur aus dem DUR-03.3-Vertrag zitiert (Z. 42-44, §10.1, §15.9), konsistent mit
  DUR-03.3 Z. 55-57, 132-142 und 324-328. Der Scan ergab 0 Treffer.
- Je Ticket gibt es eine Production-Crate. DUR-03.3a nennt das App-CLI als
  eigenes Ticket; zu ACC siehe R2-H7.
- Der Abhängigkeitsgraph ist zyklenfrei:
  - .2c → e1 → e2 → {NET-04.2b, f}
  - SIM-03.4a, DUR-04.2a → SIM-03.4b → DUR-05a
  - DUR-03.3 → DUR-03.3a → DUR-05a
  - e1 → DUR-05a
  - DUR-05a, e2 → f → DUR-05b → ACC
- Unvollständig ist nur die fehlende Kante e2 → SIM-03.4b (R2-A4).

## F. Empfehlung an Root

N1–N3 im Brief korrigieren. Das sind Regeln in §6.1, §6.2, §6.3 und §10.4 plus
Matrixzeilen. R2-A1, R2-A2 und R2-A4 gehören vor der Freigabe von e2 bzw.
SIM-03.4b in den Vertragstext; R2-A3, R2-A5 und R2-A6 sind Ticketauflagen.
Danach genügt eine kurze Prüfung von §6.1-6.5, §7.3, §4.3, §10.4, §13 und §14.

NET-04.2e1 hängt an keinem Blocker; R2-A2 betrifft e1 nur über die Signatur der
Revalidierung. SIM-03.4b ist inhaltlich tragfähig bis auf R2-A4 und R2-H5/H6.
DUR-05a und NET-04.2f erst nach der Korrektur freigeben.

Diese Prüfung ist eine Vertragsprüfung. Sie nimmt weder Runtime, Survival,
Paper-Parität noch Plugins ab.
