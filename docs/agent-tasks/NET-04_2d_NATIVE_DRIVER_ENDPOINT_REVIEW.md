# NET-04.2d — unabhängige Vertragsprüfung

Stand: 11.10.2026. **Verdikt: FINDINGS (kein CLEAN_CONTRACT).**
Drei blockierende Befunde betreffen die Nachbarverträge DUR-04.2a/DUR-05, die
Producer-Kette L1/L2/Release und die Root-Auflage G3 (Matrix). Der Endpoint- und
Lane-Kern (G1), die G2-Clientbehandlung, die Formatgates (G4) und die Rejoin-/
Release-Serialisierung sind tragfähig. Daneben gibt es zehn Auflagen für die
Runtime-Tickets und sechs Hinweise.

Geprüft wurde der eingefrorene Kandidat; er wurde nicht verändert:

| Gegenstand | SHA-256 |
|---|---|
| Brief `NET-04_2d_NATIVE_DRIVER_ENDPOINT.md` | `11af260b8e8e8da0fd4c30989593f934e9a656ec58b5d48093705532f1c045eb` |
| `design.freeze.json` | `00a758ba14a2c01a5d5d207d1ea6645bb3e34e9552692179b624e3327938592c` |
| Workorder `..._TASK.md` | `6919a9b70a4b445048ebf72fee9ae57dd91a4a7d08ef6e9522ea40e75c3ceb5c` |
| Basis-Manifest (394d4c27, 1274 Dateien) | `4c964d711c1dd4499bf914327266134c734f8aa13aefe9010a5dab27d80f519b` |

Der Code wurde ausschließlich aus `.codex-tmp/net04_2d-root-base/archive` gelesen.
Die Paper-Referenz stammt aus `.codex-tmp/paper-1.21.8-reference`, HEAD
`29c8822d90899c89d2689338e81a98f690bcba12`. Die Root-Entscheidungen G1–G4, G5 und
L1/L2 wurden im Ersteller-Thread nur lesend nachgelesen.

## A. Ausführung (reproduziert)

- `pins.py` und `freeze.py` liefen in einer privaten Kopie
  (`.codex-tmp/net04_2d-root-review/repro/`). Geändert waren nur ROOT und OUT. Exit 0,
  87/87 Anker mit Token auf der Startzeile, 0 Basisabweichungen, 25/25 .2c-Referenzpins
  identisch, Paper-HEAD korrekt. `baseline.verify.json`, `producer-pins.json`,
  `reference-pins.json` und `design.freeze.json` sind **byteidentisch** mit der
  Kandidatenevidenz (Hashes in `final-assessment.json`).
- Der Forbidden-Claims-Scan lief mit den Funktionen `markdown_blocks` und `scan` aus
  `scripts/check-forbidden-claims.py` (SHA-256 `eb2671f2…40ff`) direkt auf Brief und
  EVIDENCE.md, jeweils normal und `force_plugin`: **0 Treffer**
  (`forbidden-scan.json`). Die Muster des Scanners sind englisch; siehe H5.
- Es wurden keine Builds, Tests, Server oder Clients gestartet.

## B. Quellenprüfung (Source-Review, keine Runtime-Abnahme)

### Anker von Hand

Ich habe 41 zitierte Bereiche geöffnet und gegen die Aussage im Brief geprüft
(`manual-anchor-sample.json`). Alle stimmen inhaltlich. Der Bereich
`player_ownership.rs:1976-2029` (`apply_deferred`, Brief Z. 59-60) ist zitiert, fehlt
aber in `producer-pins.json`. Damit stimmt die Aussage „jede zitierte Stelle ist
gepinnt“ (Brief Z. 32-34) nicht ganz (H1).

### Paper-/Vanilla-Abgleich

- **EntityRemove vor TabRemove (Brief Z. 293, 491): bestätigt.** In Vanilla ruft
  `PlayerList.remove` (Base Z. 334-367) zuerst `removePlayerImmediately` auf. Daraus
  folgt synchron `Entity.setRemoved` → `onRemove` → `stopTracking` → `onTrackingEnd`
  → `ChunkMap.removeEntity` → `broadcastRemoved` → `ServerEntity.removePairing`, das
  `ClientboundRemoveEntitiesPacket` sendet. Erst danach folgt
  `broadcastAll(ClientboundPlayerInfoRemovePacket)`. Paper (Applied Z. 573 vor
  Z. 591-596) läuft über Moonrise `EntityLookup.onRemove` → `entityStatusChange`
  (synchron) → `onTrackingEnd` → `ChunkMap.removeEntity:984-998` →
  `removePairing:322-325`. Das InfoRemove geht an alle Spieler mit `canSee`.
  Unterschiede in der Audience siehe H3.
- **JoinGame als erstes Outbound-Paket: bestätigt.** Vanilla sendet in
  `placeNewPlayer` vor `ClientboundLoginPacket` nichts. Bei Paper ist Z. 267 der erste
  `send` nach Z. 151. Danach folgen Difficulty, Abilities, HeldSlot, Recipes usw. wie
  in der Brief-Tabelle Z. 478.
- **G2: Positionskorrektur ohne WorldBorder: bestätigt.** `handleMovePlayer`
  (Base 991-1126) teleportiert bei verworfener Bewegung auf die alte Position.
  `ferrumc-proto` enthält kein InitializeBorder/WorldBorder-Paket (nur das Feld
  `world_border_hit`). `SetActionBarText` ist generiert (`generated/play.rs:3565`).
  Die Region umfasst 8×16 = 128 Blöcke (`SHARD_WIDTH_CHUNKS = 8`).
  `PendingTeleport::is_pending` hält Clientpositionen bis zum Ack zurück
  (`teleport.rs:57-59`).

### Restore-Aktualität (Frage 3)

Die Annahme „ein Read nach bestätigtem redb-Commit sieht ihn“ (Brief Z. 334-335,
548) ist in der Quelle **enger gedeckt, als der Brief angibt**.
`PlayerOwnershipWork::execute` (`evidence.rs:329-372`) erzeugt `Confirmed` nur
zusammen mit einem Current-Read nach dem Commit. `confirm_current`
(`player_ownership.rs:1500-1510`) verlangt, dass dieser Read genau den After-Stamp
und den After-Record zeigt. L1 wird erst nach dieser Beobachtung frei. Jeder spätere
`read_player` beginnt damit kausal danach, auf derselben Database bzw. unter
demselben Memory-State-Lock (DUR-03.3 Z. 158-166, Immediate-Commit Z. 292). Für redb
bleibt nur die MVCC-Monotonie neuer Lesetransaktionen als Bibliotheksannahme. Ein
Runtime-Beweis ist das nicht.

### Leave-Save und Release-Nachweis (Frage 3/4)

- Autosave verlangt weder `dirty` noch `connected` (`plan_owned_player_write`
  Z. 1010-1064). Es entsteht immer echte Work; auch ein No-op hat einen Receipt
  (DUR-03.3 Z. 245). Ein sauberer Spieler blockiert den Nachweis also nicht.
  `Confirmed{current: Ok}` ist der richtige Fakt (Z. 1630-1648). Unresolved und
  ReceiptAbsent führen zu Uncertain, Busy zu Prepared, `Confirmed{Err}` zu
  NeedsRefresh (Z. 1594-1629). Das passt zu DUR-04.1 §3 und DUR-03.3 Z. 297-302.
- Ein laufender Leave-Save blockiert den Rejoin in der Sim: `pending` führt zu
  `Reserved` (Z. 739-741, 755-757). Eine Completion „nach dem Rejoin“ ist damit
  ausgeschlossen. Nach einer Disconnect lehnen Move, Mode und Select mit `Ineligible`
  ab (Z. 1905, 1939, 1969). Neue Deferred-Werte können also nicht entstehen; ein
  gesetztes `saved_after_detach` wird nicht nachträglich entwertet.
- Rejoin gegen Release ist in beiden Reihenfolgen deterministisch. Ein Permit ist
  offen, bis `NativeJoin` ihn konsumiert. Im selben synchronen Schritt wird der
  Rejoin-Input enqueued, und das SIM-03.4b-Gate „kein Input für dieselbe PlayerId“
  (Brief Z. 359) schließt das Fenster bis zum Apply. Release zuerst setzt den Permit
  auf stale und führt zu `StaleSnapshot`. Release und Join laufen seriell im Driver.
- Die L1/L2-Kopplung erzeugt keinen Zyklus. L1 hängt nur an D5 und am Save, L2 an
  D8, am Save und am Release. Lebendigkeitslücken gibt es aber an B2, A1 und A5.

## C. Befunde

### Blockierend

**B1 — DUR-04.2a/DUR-05-Counterautorität widersprochen bzw. nicht integriert.**
Brief Z. 223-231 (Allowlist der App-Konstruktoren plus Source-Scan, der „andere
`OwnedPlayerInput::*`-Konstruktoren im App-Crate“ verweigert), Z. 270-281
(Intent-/Work-Schleife), Z. 451-455 (Init-Reihenfolge) und Z. 95-99 (Behauptung
voller Konsistenz).
- *Belege:* DUR-05 Z. 3-6 setzt „geprüfte DUR-04.2a-Counterautorität“ voraus, Z. 38
  „App gibt jedem Shard denselben tatsächlichen logischen Tick“, Z. 44-46 Save-ID-
  Refills aus der Driver-UUID-Autorität. DUR-04.2 Z. 552-566 sagt: „DUR-05 liefert
  vorher IDs“. Der Konstruktor ist `OwnedPlayerInput::refill_item_save_ids`
  (`pending.rs:134`). Im Counter-Modus verweigert
  `run_tick_with_player_ownership` mit `CounterTickMismatch`
  (`player_ownership.rs:293-295`); der echte Boundary ist
  `run_tick_with_player_ownership_at(Tick)` (`counters.rs:155`), Recovery läuft über
  `recover_player_ownership_completions` (`counters.rs:237`). Beide sind in
  `producer-pins.json` gepinnt, werden im Brief aber nie verwendet. Zudem fehlen
  `SaveIdsNeeded` und `RecoveryRequired` in der Intent-Behandlung, und
  `initialize_owned_item_counters` fehlt in der Init-Reihenfolge.
- *Fehlerszenario:* DUR-05a/NET-04.2f werden nach dem Brief gebaut. Variante 1: ohne
  Counter-Modus. Das widerspricht DUR-05, und Delay40 erreicht nie 0. Variante 2: mit
  Counter-Modus. Dann läuft jeder Boundary auf `CounterTickMismatch`. Selbst mit
  korrektem Tick könnte `SaveIdsNeeded` nicht beantwortet werden, weil der
  vertraglich festgelegte Scan `refill_item_save_ids` in der App verbietet. Dirty-
  Items bleiben ungespeichert, und `ownership_quiescent` (Z. 709-712,
  `counter_quiescent`) wird beim Stop nie wahr.
- *Mindestkorrektur:* Counter-Modus explizit festlegen (an oder aus, mit
  Begründung). Den Driver-Boundary auf `run_tick_with_player_ownership_at` mit
  echter Tick-Identität, Recovery-Boundary und `RecoveryRequired`-Disposition
  festlegen. `refill_item_save_ids` (Save-ID-Quelle) in die App-Allowlist aufnehmen
  und `SaveIdsNeeded` in §6 behandeln. Die Init-Reihenfolge in §10 ergänzen.

**B2 — Producer für L2-Handle und Leave-Save widersprüchlich bzw. fehlend.**
Brief Z. 226-230 („App baut … `autosave` für den Leave-Save“, „App speichert keine
Handles gebundener Sessions“) gegen Z. 292 und 300-302 („Session baut
`autosave(stored, id)`“). Dazu Z. 320-321 (`NativeRetention{original_handle}` im
Driver) und Z. 349-352 (`release_detached_owned_player(original:
&OwnedPlayerHandle, …)`).
- *Belege:* `PlayerDetached` trägt nur die `PlayerId` (`pending.rs:546-549`). Ab dem
  DetachFact gibt es für gebundene Sessions keinen genannten Weg, auf dem der Driver
  `original_handle` erhält, ohne gegen die eigene Regel (d) zu verstoßen. e2
  (Z. 498) enthält keinen solchen Session-Accessor. Ein Leave-Save kann mehrfach
  verweigert werden (`Reserved` bei einer laufenden Pickup-Operation:
  Z. 1020-1022; `Capacity`). D5 entfernt das Mapping aber unabhängig davon
  (Z. 293). Ob `native_leave_save(key)` auf Leaving- bzw. Retired-Records wirkt, ist
  nicht festgelegt.
- *Fehlerszenario:* Spieler A disconnectet mit laufendem Pickup. D4 läuft auf
  `Reserved`, D5 retiriert die Session, und der Retry über `native_leave_save(key)`
  findet keine aktive Session. Der Leave-Save wird nie eingereicht. L1 wird nie frei
  (D9), A bleibt bis zum Neustart `AlreadyReserved`, und L2 belegt dauerhaft einen
  der 128 Sim-Plätze. Weicht die Implementierung aus, indem die App das Handle aus
  `PlayerAdmitted` behält, verletzt sie die eigene Regel (d) und Root G1(d). Der
  Source-Scan würde zudem Autosave/Disconnect generell für die App erlauben.
- *Mindestkorrektur:* Festlegen, dass `native_leave_save` für Leaving- und Retired-
  Records funktioniert, bis der Save installiert ist (Retired-Record behält das
  Handle). Der Release läuft wie D5/D7 über einen Session-Closure, etwa
  `router.release_native_retention(key, &mut |h| shard.release_detached_owned_player(h, q))`,
  sodass die App nie ein Handle gebundener Sessions speichert. Die App-Allowlist für
  `autosave`/`disconnect` auf den G3-Zweig beschränken und das im Scan prüfen.

**B3 — Root-Auflage G3 „Jeder Fall bekommt Matrixzeilen“ nicht vollständig erfüllt.**
Die G3-Races sind im Text geregelt (Brief Z. 400-413). In der Matrix (Z. 518-540)
fehlen aber eigene Zeilen für:
(a) Join-Enqueue in-flight mit Restore/Rejoin-Admission bei geschlossener Reply (nur
der Bootstrap-Fall steht in Z. 527);
(b) späte Events einer nie gebundenen Connection;
(c) Rejoin derselben UUID während der G3-Retirement;
(d) Reply-Send-Fehler nach erfolgreicher Bindung (Schritt 10, Z. 251).
Die Korrektur ist rein dokumentarisch. Die Root-Bedingung ist aber ausdrücklich, und
die Prüffrage 4 („vollständig umgesetzt?“) ist damit zu verneinen.

### Auflagen für die Runtime-Tickets

- **A1 (NET-04.2f/DUR-05a) — Completions und Lane konkurrieren um die Sim-Input-Queue.**
  `enqueue_owned_player_input` prüft eine gemeinsame Grenze
  (`inputs ≤ 32`, `player_ownership.rs:258-263`). Erst nach dem Enqueue werden
  Completions vorgezogen (Z. 311-317). Brief Z. 205-216 und 277-281 regeln nur
  `held ≤ 1` für Lane-Items. Szenario: Bewegungsspam füllt jeden Boundary 32 Plätze,
  und die Leave-Save-Completion verhungert. L1 und L2 hängen dann ohne Verlust, aber
  ohne Fortschritt. DUR-04.1 Z. 562-565 und DUR-05 Z. 39-40 verlangen: Completion
  intakt zurück, Lease bleibt beim Driver. Auflage: Completions vor Lane-Items
  enqueuen, Kapazität reservieren, bei Refusal die Completion halten und eine
  Matrixzeile ergänzen.
- **A2 (NET-04.2e2) — Übergang nach Active festlegen.** D5 retiriert nur „ehemals
  Active“ (Z. 293, .2c Z. 474-476), G3 nur „nie fertiggestellte“ Admissions
  (Z. 388). Ausdrücklich festlegen, dass ein erfolgreiches `admit_native_join`
  (Self-JoinGame enqueued) Active bzw. `ever_active` setzt. Sonst wird der Fall aus
  Schritt 10 von keinem der beiden Zweige erfasst.
- **A3 (e1/e2) — EndpointSuspended.** Festlegen, wie Disconnect/Control während der
  Suspension geroutet wird und ob `revalidate_native_endpoint` auch Leaving-Sessions
  mit noch verbundenem Sim-Spieler revalidiert (Z. 182-191, .2c Z. 717).
- **A4 (NET-04.2f) — Keine zweite Inventarautorität ausdrücklich festschreiben.** Im
  Nativpfad entfallen Connection-`PlayerInventory`, `mirror_game_mode_change`
  (`play.rs:1205-1212`) und der Legacy-Leave-Save
  (`play.rs:666-678`, `inventory.return_carried`/`to_record`). Das verlangt DUR-04.1
  Z. 421. Der Brief verbietet bisher nur den Creative-JoinKit und SetGameMode
  (Z. 255-257). Dazu einen Test vorsehen.
- **A5 (NET-04.2f) — Begrenzte Leave-Save-Wiederholung.** Wird der Save dauerhaft
  durch ein Uncertain- oder Quarantined-Pending desselben Spielers blockiert, wird er
  jeden Boundary unbegrenzt wiederholt. Jeder Versuch erzeugt einen Resync-Intent
  (`resync_refused_input` Z. 351-379). Auflage: Übergang zur L1-Quarantäne festlegen.
  Außerdem dürfen `PlayerChanged` und `Resync` für einen detached Spieler den Driver
  nicht stoppen: Autosave-Confirm erzeugt `PlayerChanged` (Z. 1700), und
  `connected_owned_player_view` verweigert detached Spieler (`lifecycle.rs:68-70`).
  Brief Z. 271-275 nimmt dafür stets eine View an.
- **A6 (e2) — .2c-API-Abweichung kennzeichnen.** `retire_detached_native_sessions`
  und `release_retired_native_records` mit Closure (Z. 293, 295) ersetzen das
  .2c-`retire_native_session(key, fact)` (.2c Z. 540-545). Das widerspricht der
  Aussage „alle .2c-Entscheidungen unverändert“ (Z. 95-99). Entweder die .2c-Signatur
  verwenden oder die Änderung als „.2c-Ergänzung“ markieren.
- **A7 (SIM-03.4b) — Nachschärfen.** Das Verhältnis zum DUR-04.2a-Boundary-Plan
  festlegen (der Plan modelliert Player-Admissions skalar: DUR-04.2 Z. 588-593); der
  Release läuft ausdrücklich zwischen Boundaries. „Countershadow“ eindeutig auf
  `collector_lifecycle.rs:274` beziehen. Die Arbeitsgrenze 128+384+32 (Z. 364-365)
  um die Scans von Input-Queue, Publication-Ledger und Entity-Reservation ergänzen.
  Matrixzeilen für Release-Refusals (Pins, Reservierungen, Snapshot-, Foreign-,
  Stale- und wiederholtes Handle) ergänzen (Root G5 Punkt 3).
- **A8 (DUR-05a) — Format7 statt „wie heute“.** Die Chunk-Ticket-Freigabe „wie
  heute“ (Z. 253) läuft über `release_chunks_acked` und `StorageFlushRequest` mit
  `save_chunk_overlays`/`append_block_mutation_batch` (`storage_worker.rs:366-400`).
  Diese Writer verweigert Format7 (DUR-03.3 Z. 132-141), und die Generation-Metadaten
  sind heute Format4. Für den Nativmodus die Format7-Open-API
  (`open_with_player_block_durability(path, config, scratch_directory)`, DUR-03.3
  Z. 55-57) und die Persistenzbarriere konkret festlegen.
- **A9 (NET-04.2f) — Work-Executor.** Den Refusal-Pfad von `execute`
  (`evidence.rs:333-339`, Work kommt zurück) und das dauerhafte Checkpoint-Wiring
  (DUR-04.1 Z. 296-301 weist es DUR-05 zu) benennen. Für Shutdown mit Uncertain
  bleibt „melden und L1 quarantinieren“ korrekt.
- **A10 (Session, e1) — Regionsprüfung.** Die Session muss dieselbe Chunkableitung
  wie die Sim verwenden (`EntityTransform::chunk` in `require_player_region`
  Z. 906-921), sonst entstehen unnötige Resyncs an der Grenze. Den Zustand
  „laufender Release“ (Z. 329-330) definieren oder streichen, weil der Release
  synchron ist.

### Hinweise

- **H1:** `apply_deferred` 1976-2029 ist zitiert, aber nicht gepinnt (siehe oben).
- **H2:** Lane ≤ 1024 entspricht `DEFAULT_SHARD_INPUT_CAPACITY` (`router.rs:32`).
  Die Reserve ≥ 1 ist schwächer als der Default 16 (`router.rs:41`). Read-Permits
  ≤ Sessions-Limit (≤ 1024, .2c), L2 ≤ 128 und Release 4/Boundary passen zu den
  Grenzen Sim-Inputs 32, Pending 32 und Players 128. Nur die Release-Rate ist in §15
  ausdrücklich als „nicht gemessen“ markiert (Z. 552). Lane-, Reserve- und
  Permitwerte sollten ebenso gekennzeichnet werden.
- **H3:** Die Reihenfolge ist angeglichen, die Audience nicht. FerrumC sendet
  TabRemove nur an Viewer mit Baseline (Tab-Add ebenfalls nur `within_view`,
  `router.rs:635-748`). Vanilla nutzt `broadcastAll`, Paper alle mit `canSee`. Das
  gehört nach Workorder in die Abweichungsliste in §12.
- **H4:** Die Session-INVARIANTS („Router tauscht nur GameInput/GameOutput/
  ClientboundPlayPacket“) müssen in e1 für `NativeShardInput` angepasst werden.
- **H5:** Der Forbidden-Claims-Scanner prüft englische Muster. „kein Verlust“
  (Z. 368, 536) habe ich manuell geprüft: Es steht als Vertragsziel und
  Testerwartung da, nicht als behauptete Abnahme.
- **H6:** Der Brief deklariert ehrlich, dass DUR-03.3 nur teilweise und DUR-04.1/.2
  nicht als ganze Briefe gelesen wurden (Z. 28-31). B1 und A8 sind genau die Folgen
  dieser Lücke.

## D. Bewertung der Prüfpunkte

| Punkt | Ergebnis |
|---|---|
| 1 Pins/Freeze | reproduziert und byteidentisch; 41 Anker von Hand korrekt; H1 |
| 2 Workorder | Owner-Hook (Slot + Receiver-Borrow + echte View, Sim-Recheck), Erstbindung/Replacement getrennt, kein SimShard in Session, Join 1-12 und D1-D10 konkret, kein Retarget: erfüllt. Lücken: B2 (D4/D10-Producer), A4 (Inventar), A1 |
| 3 Nachbarverträge | .2c: A6. SIM-03.4a: konsistent. DUR-03.3: Restore-Aktualität und Uncertainty konsistent, Format7-Barriere A8. DUR-04.1/.2: **B1** |
| 4 G1-G5, L1/L2 | G1, G2, G4 umgesetzt. G3 Text ja, Matrix **B3**. G5 bis auf A7. L1/L2 ohne Zyklus und ohne verlorene Saves, Lebendigkeit B2/A1/A5. Rejoin/Release in beiden Reihenfolgen deterministisch |
| 5 Werte | begründet und kompatibel; Kennzeichnung unvollständig (H2) |
| 6 Paper | EntityRemove→TabRemove, JoinGame zuerst und G2-Korrektur ohne WorldBorder bestätigt (Source-Review); Audience H3 |
| 7 Ehrlichkeit | keine Runtime-, Survival-, Plugin- oder Performance-Abnahme behauptet; Scan 0 Treffer |

## E. Empfehlung an Root

Den Brief nachbessern (B1-B3) und die Auflagen A1-A10 in den jeweiligen
Runtime-Tickets binden. Danach ist eine erneute, kurze Prüfung der geänderten
Abschnitte nötig. NET-04.2e1 (ferrumc-session) hängt an keinem Blocker und könnte nach
Root-Ermessen getrennt freigegeben werden. SIM-03.4b, DUR-05a und NET-04.2f sollten
erst nach der Korrektur freigegeben werden. Diese Prüfung ist eine Vertragsprüfung;
sie nimmt weder Runtime, Survival, Paper-Parität noch Plugins ab.
