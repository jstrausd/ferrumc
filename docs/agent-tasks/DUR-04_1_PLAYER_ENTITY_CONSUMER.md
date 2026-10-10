# DUR-04.1 — besessener Playerzustand und bestätigte Drop-/Pickuptransfers

Status: abhängiges Folgeticket vorbereitet, noch nicht vergeben/implementiert.
Autor besitzt ausschließlich `server/crates/ferrumc-sim/**` und diesen Brief.
Voraussetzungen: CLEAN integrierte DUR-03.1 Storageauthority sowie vorhandene
ITEMS-TRANSFER-001 und SIM-03.3. App-/Session-/Storage-/Itemsedits und Git bleiben
außerhalb dieses Tickets. Vor Code einen konkreten bounded API-/State-/Datei-
Entwurf liefern, den Root innerhalb der vorhandenen Autorisierung prüft.

## Tatsächliche Ausgangslücke

Die aktive App hält heute das46-Slot-Fenster, den Cursor und die ausgewählte
Hotbar **in der Connection** (`server/app/src/inventory.rs`). SimPlayerState
hält Position/Look/GameMode und besitzt dieses Inventar nicht. Der jetzige
Appencoder speichert `FCPS`-Payloadschema3; Sim kann daraus noch keinen joint
Player↔Item-Entitypostimage planen. Ein zweites generisches Transferobjekt ohne
diesen tatsächlichen Consumer würde Drop/Pickup nicht aktivieren.

Dieses Ticket führt den vollständigen nativen Playerzustand in die besessene
Simauthority ein und implementiert daraus konkrete Drop-/Pickupplanung,
Reservation, Pendingrequest und bestätigte gemeinsame Veröffentlichung. Die
App wird anschließend in DUR-05 angebunden und verliert dort ihre parallele
Inventar-Schreibauthority. Bis dahin bleiben alte Apppfade getrennt und werden
nicht als sichere Onlineverbraucher bezeichnet.

## Konkreter native Playervertrag

- Ein Player bleibt global UUID-identifiziert, sein gegenwärtiger Scope und
  die tatsächliche Generationidentity sind explizit gebunden. Kein
  Crossdimension-/Crossworldwriter in `.1`; keine erfundene Legacyzuordnung.
- Eigenes private-field Playersnapshot/Admission mit Position, Look, GameMode,
  ausgewähltem Hotbarindex,46 original indizierten Slots und **einem** Cursor.
  Slot0 ist Rezeptanzeige und keine frei verfügbare Itemquelle. Keine kreative
  Starterausrüstung aus einem bestätigten Survival-Missingrecord ableiten.
- Bestehendes natives `FCPS` mit Payloadschema3 exakt lesen/schreiben:
  Header41Bytes, danach47 u32-length-framed lossless FCIS-Stapel. GameMode
  gehört in das vorhandene PlayerRecordfeld. Position-/Look-/Selection-
  Admission bleibt ausdrücklich; nicht selected auf0 clampen oder
  Komponenten/ungültige Daten in leere Stacks umdeuten.
- Originale Game-/Itemdaten und reine Codecmodelle sind keine private live
  Schreibauthority. Konkrete Owner-/Stale-/Clone-/Rejoin-/Scopegates für
  Playerzulassung und Reservation im bestehenden shardbesessenen Zustand.
  Kein `Clone` von SimShard darf eine zweite aktive Player-/Pendingauthority
  legitimieren. Stabile Window-/Connectionids bleiben getrennt von
  dauerhaften Ownershiprevisionen.
- FCPS1/2-JSONmigration bleibt zunächst der vorhandene explizite Appconsumer;
  der neue Simcodec braucht keine neue serde-/JSONengine. DUR-05 muss vor
  Admission die deklarierte Migration durchführen oder typed verweigern.
- Aggregate Player-/Pending-/Postimage- und Clone-/Decodebudgets vor Allocation
  konkretisieren. Kein Socket-/Packet-/DB-/Tokiohandle in Sim und keine
  neue Dependency ohne Rootbesitz-/Lockabgleich.

## Konkrete Transfers und Pendingauthority

1. Genau ein Player und eine vorhandene/fresh Item-Entity werden unter ihren
   tatsächlichen Authorities reserviert. Positive feste Pending-/Bytecaps und
   Ressourcenprüfung vor Events/Clones. Zwei Pickups desselben Items und
   parallele Playerautosaves dürfen nicht unabhängig planen/publizieren.
2. Der vollständige Storage-Pairsnapshot liefert beide private Liveexpectations.
   Drop-Single/Drop-All sowie Teil-/Fullpickup konsumieren die vorhandenen
   lossless Itemspläne und FCIE-Itemstate. Shape/Menge/Komponentenbilanz wird in
   Sim geprüft; Storage versteht die Bytes absichtlich nicht.
3. Ein konkreter versionierter Pendingrecord enthält ID/SHAquery, Scope/volle
   Generationidentity, Keys, ursprüngliche Stamps und die exakten gebundenen
   Before-/After-/Requestdaten. Keine Neuwahl von Items unter derselben ID;
   Events/RNG werden bei Retry nicht erneut als neuer Transfer ausgeführt.
4. Sim liefert dem externen Driver einen begrenzten Storageauftrag und behält
   seine Reservation. Eine Future oder ein Apprequest allein ist kein
   Commitbeleg. Neue bestätigte Storageoutcomes werden am Tickboundary als
   typisierte Completioninput angenommen.
5. Erst bestätigter Commit/identisches Replay mit genau diesem Request erlaubt
   gemeinsame Veröffentlichung von Playerpostimage und Itemstack/Spawn/
   Removal. Danach konkrete Inventar-/Entity-/Take-/Statintents. Completion
   aus fremdem Owner/Scope/ID/SHA, doppelte/stale Ack und Konflikt dürfen weder
   Doppelspawn noch erneute Menge erzeugen.
6. CommitUncertain/ExecutionUncertain und Lostreply behalten Pendingauthority;
   ein Receiptread `None` beweist keinen Abort eines laufenden Workers.
   Restartklärung verwendet zuerst die readonly Receiptquery, danach nur bei
   geklärter alter Workerlebensdauer und identischen frischen Stamps denselben
   rekonstruierbaren Request mit identischer SHA. Kein stilles Replanning.
7. Disconnect/Unload/Stop behalten oder übertragen Pendingauthority ausdrücklich.
   Nicht während pending per LegacyPlayerStore autosaven. Die Reservation
   umfasst den vollständigen Playerpostimage; Position/Look/GameMode-/Inventar-
   writers dürfen ihn nicht unbemerkt während des Auftrags verändern. Konkrete
   begrenzte Defer-/Resync-/Completionordnung im Entwurf festlegen.

Bootstrap/Spawn-/Autosave benutzen die receiptgestützten DUR-03.1-Einzelschreiber,
keine Dummyresource oder blinden Nebenwriter. Bestätigte Currentexpectations
und originale Replay-Afterexpectations sind unterschiedlich: eine alte
Replayrevision darf kein späteres CAS überschreiben.

## Originalmechanik und sichtbare Outputs

Vollständige angewandte/Vanilla-Methoden, Callees und Source-/Featurepatches
von Player/LivingEntity/ServerPlayerdrop, Inventory.add/canHold/slotchoice,
ItemEntity.playerTouch und PlayerActionhandler lesen. DUR-03.1s Sourcebefunde
geben die reale Event-/Cancellationordnung vor; keine interface-only Prüfung.

Originale Pickupdelay-/Owner-/Thrower-/Count-/Motionregeln, gewählter Slot vor
Maininventory, tatsächlich passenden Offhandstack und volle Taschen beachten.
Normale Slotmengen ersetzen keine unbelegte effektive Componentgleichheit:
fehlende Default-/Componentpolicy konkret benennen und getrennt nachziehen.
Paper-Plugins können Stacks ersetzen; das ist eine benannte externe Mutation,
keine reine Zweiressourcenconservation. Javaevents/Packetencoding folgen als
eigene Verbraucher; strukturelle Eventseams allein sind keine fertige Bridge.

Für den ersten Consumer konkretisieren, welche Physics-/Tracking-/Loaded-
Lifecyclegates bereits verfügbar sind und welche benannten SRV-02-Untertickets
unmittelbar folgen. Kein generisches Callback darf ungeprüfte fremde Owner-
oder Lifecycleauthority einschleusen. Drop-/Pickupplanung darf die geforderte
Motion-/Animation-/Persistenz-/Mehrspielerabnahme nicht als abgeschlossen
kennzeichnen, solange deren aktive Verbraucher fehlen.

## Meaningful Abnahme und Folgeintegration

Unabhängiger Mengen-/Komponenten-/State-Machinevergleich gegen echte Memory-
und redb-DUR-03.1-Producer: Bootstrap, Drop1/All, Partial/Fullpickup, volle Tasche,
Offhand/Hotbarreihenfolge, zwei Pickups, paralleler Autosave, Capacity/Busy,
stale/foreign Completion, Reopen/ABA und identischer Retry nach späterem Write.
Jede Refusal erhält Calleritems; jede Veröffentlichung hat einen tatsächlichen
passenden Commitbeleg und aktuelle private Reservationsauthority.

Native FCPS-Fixtures werden unabhängig aus dem bisher geprüften Appcodec
festgehalten; echte hardcoded Offsets und vollständige malformed/version/
trailing/aggregate/component/finite/Selectionfälle, keine ausschließlichen
Selbstroundtrips. Completion-/Reservation-/Pendinglimits an Grenze/+1 und
unvollständige Driverergebnisse ohne Publish testen. Whole-Sim Debug/Release,
strict Clippy/Fmt/Rustdoc, exakter Freeze und unabhängiger Fremdreview.

Danach: DUR-05-Appdriver mit expliziter Format6admission/Bestandsmigration,
atomarem Playerownerwechsel statt paralleler Connectioninventory, bounded
Storagejobqueue, echter Itemload-/Unload-/Trackinganbindung und Client-/Restart-
Cutpoints. SRV-06/07 binden den normalen Rezeptproducer in dieselbe tatsächliche
Playerauthority ein. Physik/Merge/Despawn, Mining/Loot, Kampf/Mobs, Nether/End/
Dragon und weitere Survivalstrecke bleiben benannte abhängige Folgearbeit.
