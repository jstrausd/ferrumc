# DUR-04.1 — besessener Playerzustand und bestätigte Drop-/Pickuptransfers

Status: durch Root an `select_world_ticket` vergeben; konkreter Entwurf unten
intern freigegeben, Autorstand implementiert und eingefroren; unabhängige CLEAN-Abnahme offen.
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

## Konkreter Autorentwurf auf Baseline `eaedabd2`

DUR03.1 ist unabhängig CLEAN integriert. Besitz bleibt ausschließlich dieses
Dokument und `server/crates/ferrumc-sim/**`; Cargo-/Lock-/Storage-/Items-/App-/
Session-/Gitmutationen bleiben ausgeschlossen. Keine neue Dependency.

### 1. Tatsächliche Playerauthority und FCPS3

`shard::PlayerState` verliert `Copy` und enthält opt-in einen privaten
`OwnedPlayerState`: genau46 indizierte Slots, einen Cursor, Selection0..8,
ScopedPlayerKey, bestätigte DUR03expectation, die zuletzt bestätigten nativen
Storagebytes, positive nichtwrappende lokale Admission-/Mutationgeneration
und Connected/Detached/Pendingstatus. Position/Look/GameMode bleiben dieselben
Felder dieses tatsächlichen PlayerState; kein zweiter Liveplayer und kein
Connectioninventory im neuen Consumer. Legacyspieler behalten ihre bisherigen
Pfade. Der neue Modus ist nur bei leerem Player-/Entitybestand ausdrücklich
initialisierbar, an den deklarierten `GenerationBinding` und kanonischen
`ShardId` gebunden. Ein späterer Austausch der Chunkmap muss denselben Scope
und dieselbe volle Generationidentity behalten, sonst Refusal vor Work/Tick.

`NativePlayerSnapshot` ist ein privater-field reiner Wert mit Position/Look/
GameMode/Selection/46Slots/Cursor. Öffentliche read-only Getter und ein checked
Recorddecoder geben keine Live-Schreibberechtigung. `OwnedPlayerHandle`
bindet schwache Simincarnation, PlayerUUID und positive Admissiongeneration;
alten Session-/Rejoinhandles fehlt damit Authority. Er ist von WindowstateID
und dauerhaftem Ownershipstamp verschieden. Es gibt keinen raw mut accessor.

Der konkrete lokale Codec liest/schreibt genau FCPS Version3, Payloadschema3:
Magic0..4, Version4..6, x/y/z6..30, yaw30..34, pitch34..38, selected38,
Slotzahl39..41 und47 u32-BE-geframte FCIS-Stapel. Er verwendet die bestehenden
Itemsdecoder/-encoder; ein eigener kleiner bounded Feldreader benötigt keine
neue Codecdependency. Position jeweils finite/abs<=30.000.000, Look finite,
selected<=8, Slotzahl46, Frame-/Aggregatecaps vor Decode/Allocation und keine
Trailingbytes. Floatbits einschließlich signed zero werden erhalten. JSON1/2
und andere Schemata verweigern typisiert; DUR05 behält die explizite Migration.
Nur ein echter Fresh-Miss erlaubt Empty-Survivalbootstrap; Tombstone ist kein
stiller Freshdefault. Slot0 wird erhalten, bleibt als Rezeptanzeige gesperrt.

### 2. Konkrete Grenze zum externen Storageconsumer

Keine DB-/Tokio-/Sockethandles im shardbesessenen Zustand. Ein privater
`OwnershipStoreBinding` enthält nur eine schwache Identität der ausdrücklich
zugewiesenen `Arc<dyn PlayerEntityOwnershipStore>` und den GenerationBinding,
wie die vorhandene Weak-Allokationsprüfung bei Generationcapture. Sie hält
keinen Backendowner am Leben und gestattet keine Datenbankoperation selbst.

Ein konkreter off-tick Helper liest echte DUR03-Snapshots vom genau gebundenen
Store und erzeugt versiegelte Admission-/Pairread-Evidence. Caller können
keine Evidence aus frei übergebenen Expectations/Receipts konstruieren. Der
Helper muss Scope, volle Generationidentity und Resourcekeys genau prüfen;
Default-/alte Modi liefern den bestehenden typed Unsupported. Bereits
gespeicherte Player-/Itembytes werden vor Admission vollständig decodiert.

`OwnedPlayerInput` ist ein move-owned konkreter Input mit checked Constructors
für Restore/Fresh-Survivalbootstrap, Restore/SpawnItem, DropSelected(One/All),
Pickup, PlayerAutosave, Disconnect/Rejoin, Receiptklärung und Completion.
Drop/Pickup tragen den tatsächlichen Playerhandle, für Pickup den tatsächlichen
Entityhandle und den gemeinsamen readonly Pairsnapshot. Der Snapshot muss
beide aktuellen bestätigten Expectations und bestätigten Bytes treffen;
Playerbewegung seit dem letzten Autosave darf im neuen vollständigen
Playerpostimage enthalten sein. Weder ein bloßes Requestfuture noch ein
Callerboolean erzeugt Commit-/Owner-/Lifecycleevidence.

`enqueue_owned_player_input` prüft gemeinsame Count-/Bytecaps und gibt bei
Refusal den ganzen move-owned Input zurück. Es mutiert keine Player-/Item-
Authority. `run_tick_with_player_ownership` drainiert diesen konkreten Input-
Completionprefix am Tickboundary, danach den Action-FIFO und den bisherigen
GameInput-FIFO; kein generischer Transfercallback. Das Resultat enthält konkret typisierte
Storagework und Publikations-/Resync-/Lifecycleintents sowie normale Outputs.

`PlayerOwnershipStorageWork` ist move-owned, enthält private Completionticket,
requestkind, gebundenen Store/Simowner und genau den bereits vorbereiteten
DUR03-Pair-/Player-/Entityrequest. Ein konkreter async Evidencehelper läuft
nur im externen Driver: er verwendet den borrowed gebundenen Store, führt
genau diesen DUR03-Call aus, klärt bei Bedarf die originale Receiptquery und
liest aktuelle readonly Resourcepostimages. Er liefert die versiegelte
`PlayerOwnershipCompletion`. Kein freier Outcome-/Closure-/Booleanparameter
kann dadurch eine Completion fälschen. Sim hält/awaitet keine Future. Diese
pure Crateboundary ist noch kein produktiver Appdriver und kein Queue-/
Thread-/Socketconsumer; DUR05 übernimmt tatsächliche Ausführung/Backpressure.

### 3. Reservierung, Pending und Veröffentlichung

`PlayerOwnershipState` hält bounded Pendingrecords in geordneten Containern.
Ein Record hat explizite Version1, ID/SHAquery, Scope/Generation, Resourcekeys,
ursprüngliche dauerhafte Stamps, private Sim-/Player-/Entityauthority,
lokale Beforegeneration, exakte native Before-/Afterbytes, vorbereitete
Postimages, gebundenen identischen Request und Prepared/InFlight/Uncertain/
NeedsRefresh/Quarantinedstatus. Request und Events/RNG werden beim Retry
niemals neu geplant. Eine zweite Operation desselben Players wird verweigert.

Für `.1` wird konservativ höchstens **eine** Entityoperation pro Shard
reserviert: Drop/Pickup/Itemspawn besitzen während Pending die tatsächliche
EntityStore-Mutationsgrenze einschließlich Slotcapacity, IDs und Bytebudget.
Receiptgestützte Player-only-Autosaves anderer Spieler können parallel planen.
Vor Dispatch werden komplette spawn-/replace-/remove-Capacity-/FCIE-/Index-
Preflights einschließlich fallible Slotvecreservation abgeschlossen. Keine
sichtbare Dummyentity. Bestehende generic EntityTickContextwriter dürfen
im neuen Ownershipmodus keine besessenen Itembytes verändern; während der
Entityreservation verweigern alle konkurrierenden Storewriter. Die private
bestätigte Veröffentlichung hat die einzige entsprechende Mutationscapability.
Nonitem-/Legacyverträge außerhalb des ausdrücklich gewählten Modus bleiben.

Ein Pairreceipt muss Query, Scope, volle Generation, Player-/EntityUUID,
Beforestamps und erwartete Aftertags/-revisionen/Chunkwechsel treffen. Zusätzlich
müssen die frisch gelesenen aktuellen Records exakt die vorbereiteten
Postimages und Receipt-Afterstamps treffen. Historische Replayexpectations sind
kein Currentbeweis: superseded Postimages führen zu Quarantined/Resync, niemals
zum erneuten Publizieren oder zu einem CAS mit alter Afterrevision. Stale,
foreign, doppelte und abgeschlossene Completiontickets verweigern ohne Mengen-
oder Spawnänderung. Private Reservationsauthority wird nochmals geprüft.

Erst dann werden Playerpostimage und Itemspawn/-replace/-remove innerhalb
derselben synchronen, vorab geprüften Tickgrenze veröffentlicht. Konkrete
Intents nennen Operationquery, Playerhandle/Mutationgeneration, Entityhandle/
WireID/UUID, Inventaränderung und Take-/Statdaten. Ein bounded checked read-only
Snapshotconsumer liefert danach die tatsächlichen Slots; keine mehrfachen
unbudgetierten Inventoryclones im Output. Bootstrap und Itemspawn verwenden
DUR03-Einzel-CAS, keine Dummyresource; Autosave verwendet Player-CAS.

Definitive Returned-refusals aus dem echten Storecall geben die Reservations-
authority frei, lassen sämtliche Beforeitems erhalten und erzeugen höchstens
Resync. Busy/Capacity können denselben unveränderten Request erneut erhalten.
CommitUncertain/ExecutionUncertain/Lostreply oder ein Fehler beim frischen
Postimageread behalten Pending samt Reservation. Ein fehlendes Receipt gibt
niemals Abortautorität; insbesondere keine `None -> Release`-Abkürzung.

### 4. Clone, Disconnect, Unload und Stop

`SimShard::clone` wird ausdrücklich implementiert: bounded checked Statecopy
mit frischer Incarnation, aber **SnapshotOnly** für Ownershipmode; keine
aktivierbare Player-/Pending-/Storageworkauthority, kein Clone->Initialize-
Reaktivierungspfad. Sourcehandles werden verweigert. Bereits Prepared/Uncertain
Records dürfen als lesbare Checkpointdaten kopiert werden, ihre laufenden
Completion-/Reservationscapabilities niemals. Legacy-Cloneregession bleibt.
EntityStoreclone bleibt ebenfalls ein Snapshot und kann keinen neuen
Ownershipmodecontainer legitimieren.

Der komplette Playerpostimage ist während Pending unveränderlich. Legacy
PlayerJoin/SetGameMode und Inventar-/Placement-/Container-Nebenwriter können
besessene Player nicht überschreiben. Pending Move/Look wird bounded als
ein coalescter letzter gültiger Wert pro Player behalten; Completion wird
zuerst abgearbeitet, danach explizit bei neuer Generation angewandt oder
resynct. Kein unbounded Deferfifo. Disconnect markiert Detached und verwirft
keine Pending-/Itemauthority. Rejoin während Pending verweigert; später
erzeugt er einen neuen local Handle. Ticketunload erhält Entityregistrierung
und Pending; neue Transfers in nichtresidenten Zielen verweigern.

Drain nimmt keine neuen Gameplayintents an, klärt vorhandene Completions und
Autosaves. Stop/Quiescence verweigert bei jedem Pending oder ungespeichertem
Ownedplayer; der bestehende Schedulerquiescencecheck umfasst diese echten
Zustände. Kein Legacyautosave am Pending vorbei. Ohne produktiven DUR05driver
gibt es bewusst kein erfolgreich vorgetäuschtes Stop-/Restartflushsignal.

Versionierte checked Checkpointdaten enthalten ID/SHA, Scope, volle Generation,
Stamps, Before-/Afterrecords und native Postimages; sie erzeugen keine lebende
CAS-/Simauthority. Receiptpositive Wiederzulassung benötigt zusätzlich echten
Currentread und neue lokale Owneradmission. Bei `Receipt None` nach einem
Prozess-/Driverwechsel verweigert `.1` typisiert `UnprovenWorkerLifetime`:
das Erlöschen einer Weak-Storeallokation beweist nicht das Ende eines redb-
Workers. Die tatsächlich belegte Queue-/Workerquiescence und dauerhaftes
Checkpointwiring gehören zum unmittelbaren DUR05driver; keine erfundene
Boolean-/Weak-liveness-Recoveryfreigabe. Innerhalb derselben lebenden Authority
bleibt der exakte Preparedrequest für idempotente Wiederholung erhalten.

### 5. Originalmechanik und ausdrücklich verfügbare Policies

Gelesen: vollständige angewandte LivingEntity.drop/createItemStackToDrop,
Player.drop, ServerPlayer.drop(boolean)/drop(stack), Inventory.removeFromSelected,
canHold/getFreeSlot/getSlotWithRemainingSpace/add/addResource,
ItemEntity.playerTouch/Konstruktor/setTarget/setThrower/delay und
ServerGamePacketListenerImpl.handlePlayerAction samt Source-/EARpatches.
Pin bleibt Paper1.21.8 Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`, MCJava1.21.8/protocol772.
Vor Runtimecode folgen exakte vollständige Source-/Patchhashes im Taskmanifest.

DropOne/All entnehmen nur aus dem ausgewählten Hotbarslot36+selected, behalten
die Komponenten und verwenden die bestehenden checked `plan_split`-Ergebnisse.
Der standing-native Drop erzeugt eyeY-0.3f, Pickupdelay40, Age0/Health5,
TargetNone und PlayerUUID als unabhängigen Thrower. Motion/RNGformeln und
ursprüngliche Constructorbob-/Rotationbasis werden einmal erfasst; erneute
Requests verwenden diese gespeicherten Werte. Vorhandener World-Randomproducer
ist wiederverwendbar; ein vollständiger Pose-/Eyeheight-/Vanilla-Mth-/Physics-
Differentialclaim benötigt separate tatsächliche Consumerabnahme.

Pickup respektiert residenten ownerlokalen Itemhandle, actual native properties,
Delay/OwnerUUID und Suppliedpolicy. Inventorysearch folgt dem tatsächlichen
Javaindexlayout: passende selectedHotbar, passende **belegte** Offhand45,
dann Hotbar36..44 vor Main9..35 für Merge/erste freie Slots. Leere Offhand,
Armor, Craftslots, Recipe0 und Cursor sind keine automatische Pickupquelle/
destination. Teilpickup behält exakt den Remainder; Fullpickup erzeugt durable
Entitytombstone und erst danach sichtbares Remove. Volle Taschen erzeugen
keine Mengenmutation. Spectator-Drop ist gesperrt. Creative-destruction-on-
full-inventory wird nicht als conserving Survivaltransfer ausgegeben.

ITEMS-TRANSFER-001 vergleicht exakte Componentrepräsentation; Defaultpatch-
Normalisierung und opaque effektive `max_stack_size`/durability sind darin
ausdrücklich noch keine Paperpolicy. Der Consumer löst supported typed Limits
konservativ mit Registryceiling und benennt/refused ungelöste gameplayrelevante
opaque/removed Policies typisiert; keine größere Itemcountzulassung als Items
beherrscht. Representation-exact konservierende Transfers sind geprüft,
effektive Komponentenparität bleibt ein unmittelbares Items-/Registryticket.

Papers Order: Drop-Consumer/Event vor Worldadd; Pickup-Attempt, Legacy und
Entityevent vor tatsächlicher Inventoryadd; Countrestoration bei Cancellation,
Ownercheck danach, Take/Remove/Stats nach Aufnahme. Paper verwendet im
gelesenen Teilpickup die ursprüngliche Countvariable für Take/Stats; Menge der
tatsächlichen Einlagerung wird separat erfasst. Javaevents können Stacks ersetzen
und sind eine externe Mutation, keine Zweiressourcenconservation. `.1` hat
konkrete native Cancellation-/Animationpolicy ohne generischen mutierenden
Callback; Javaevents und das Single-drop-Spam-/Pose-/Collision-/Trackingwiring
gehören zur unmittelbar anschließenden App-/SRV02-/Javaeventarbeit.

### 6. Feste positive Ressourcenverträge

- Native Playerpayload höchstens1MiB wie Storage; Itempayload höchstens256KiB
  einschließlich FCIEheader.47frames und46Slots sind feste Shapes.
- Config: max128 Ownedplayer und höchstens32MiB checked aggregierte
  Playerencodedbytes; beide positive/operatorabgesenkte Caps vor Admission.
- Max32 Pendingrecords und höchstens8MiB Pendingchargedbytes. Pro Record wird
  **vor** Kandidaten-/Request-/Work-/Checkpointclone exakt
  `512 + 2*before_record_bytes + 5*after_record_bytes + 149` belastet, mit
  checked arithmetic. Die512Bytes sind fester Metadatavorbehalt,149Bytes der
  DUR03-Canonicalrequestoverhead; beide Recordbytesummen enthalten die
  vollständigen tatsächlichen Player-/Entitypayloads. Zwei Beforekopien
  decken retained Before/Checkpoint ab, fünf Afterkopien native Postimages,
  retained Afterrecord, Request, einen Work/Completionpuffer und Checkpoint.
  Höchstens ein Work/Completionpuffer pro Pending; keine unbudgetierte zweite
  Auslieferung unter einer ID. Ein maximales Pair kann daher dieses8MiB-
  Admissionagreement überschreiten und wird unverändert refused; Operator-
  Capacityerweiterung wird nicht still vorgenommen. Das ist eine explizite
  `.1`-Ressourcengrenze zusätzlich zu Storagepayloadcaps.
- Max32 Ownedinput-/Completionentries und höchstens8MiB gemeinsame queued
  encodedbytes, refuse-newest mit vollständiger Inputrückgabe. Ein Workbatch
  besteht nur aus zugelassenen Pendingrecords, höchstens32 und8MiB charged.
- Eine Entityreservation pro Shard, ein coalescter Move pro zugelassenem
  Player. Alle Tabellen/Buckets/IDs bleiben durch bestehende Entityslotcaps
  und maximal16MiB native Entityencodedbytes begrenzt.
- Diese Caps begrenzen geprüfte encodedbytes/counts und temporäre konkrete
  Clonepfade, nicht vollständige NBT-RSS oder eine Prozessspeicherzusage.

### 7. Exakter Dateiplan, Prüfung und Grenzen

Neue konkrete Files:
`src/player.rs` (native snapshot/handles/config/errors),
`src/player/codec.rs` (FCPS3), `src/player/evidence.rs` (konkrete off-tick
Store-/Receipt-/Currentreadfactorys), `src/player/pending.rs` (versionierte
Pending-/Checkpointwerte), `src/shard/player_ownership.rs` (tatsächliche
Admission/Plan/Reservation/Completion/Tickconsumer),
`src/entity/ownership.rs` (private actual-slot Entityreservation/commitgates),
`src/player/tests.rs`, `tests/player_ownership.rs`,
`tests/player_entity_transfer.rs`, `tests/player_ownership_boundaries.rs`.
Bestehend: `src/lib.rs` Exports, `src/shard.rs` PlayerState/Clone/Tickgates/
Quiescence, `src/entity.rs` und `src/entity/item.rs` tatsächliche Legacywriter-
gates, `README.md`/`INVARIANTS.md` mit getrennten aktiven Consumergrenzen und
dieser Brief. Keine neuen allgemeinen Transaction-/Callback-/Pluginframeworks.

Abnahme mit tatsächlichen Memory/redb-Ownershipstores: indépendante FCPS3-
Appfixtures/Offsets, exhaustive malformed/version/trailing/finite/selection/
frame/aggregatefälle; echte Bootstrap/Restore/Drop1/All/Partial/Fullpickup,
voll/Offhand/Hotbar, eigenständige Spawn-/Autosave-CAS, Komponentenbilanzmodel,
zwei Collectorintents, Autosaverace, echte Busy/Capacity/CAS/ABA/Reopen,
Commit-/Executionuncertain, `None` ohne Abort, historische Replayrevisionen,
foreign Store/Owner/Handle/Completion, double/stale Ack, Movement-/Gamemode-
Reservation, Disconnect/Unload/Drain/Stop/Clone, Grenzen exakt/+1.

Whole-Sim Debug/Release, strict all-target Clippy/Rustdoc und owned fmt mit
Rust1.99.0, `-j2`, eigenem Target im Archivbaselinecheckout`eaedabd2`; nur dessen
Simcopy wird synchronisiert. Exakter Autorfreeze und unabhängiges CLEAN vor
Rootintegration. Keine Appaktivierung, Prozesskill-/Powerlossabnahme oder
vollständige Survival-/Paper-/Plugin-/Dragonbehauptung aus diesem Ticket.

### 8. Bestehende Writer und der einzelne nachfolgende Appowner

| Tatsächlicher heutiger Writer | Konkreter Weg im neuen Simowner |
| --- | --- |
| `shard.rs` PlayerMove und coalescter Applypass | ownerlokaler checked Move/Look; während Pending ein letzter gültiger Deferwert, Completionprefix zuerst, anschließend neue lokale Mutationgeneration/dirty; kein Update des gebundenen Pendingpostimage |
| `shard.rs` SetGameMode / App `inventory.set_game_mode` | im Ownershipmodus nur `OwnedPlayerInput::SetGameMode(handle, mode)`; Pending behält einen letzten Defermodus, nach Completion neue Mutationgeneration/dirty und Resyncintent; Legacyinput verweigert |
| App `inventory.set_selected` / SetCarriedItemhandler | `OwnedPlayerInput::SelectHotbar(handle, selected)`; striktes0..8, Pending maximal ein letzter Deferindex; nach Completion Held-/Inventoryresync bei neuer local generation |
| App window0 `normal_click` | konkreter `NormalSlotClick(handle, expected_local_generation, slot, Left/Right)` plant die vorhandene Slot-/Cursorregel aus tatsächlichen Simslots; bounded lossless split/transfer oder kompletter validierter Swap; Slot0 und Armorinsert/Spectator verweigern; Einzel-CAS vor Veröffentlichung, stale generation führt zu Resync |
| App creative-slot/raw `set_carried`/`exchange_carried` und Screenclose | keine Rawwriter im neuen Owner; ungeimplementierte nichtconservierende Creative-/Close-/Return-/GUIarten verweigern/resyncen ausdrücklich bis zu ihrem konkreten nachfolgenden Producer; niemals zurück zur Connection-Schreibauthority |
| Sim `container_left_click` + App chest-/cursormirror | Legacyroute verweigert für Ownedplayer mit unverändertem Callercursor; echte Player↔Containermehrressourcentransaktion ist eigenes abhängiges Ticket, keine Kopie von Cursor/46Slots in einer zweiten Authority |
| Legacy Placement-/Breakinputs mit App-resolved heldstack | für Ownedplayer refuse/resync bis zum konkreten Mining-/Placementitemconsumer; kein unsicherer Nativeinventarwriter und keine free-survival-creative-Blockmutation |
| App leave-save und Driverautosaves | nur vollständiger Sim-owned snapshot aus aktueller Generation, receiptgestützter Player-CAS; Pending/Detachedzustände werden nicht per PlayerStore nebenher gespeichert |

Bei Gameplayrefusal bleiben alle ursprünglichen Player-/Cursor-/Itemwerte
besessen und lesbar. Die konkreten Resyncintents tragen Handle und aktuelle
local generation; DUR05 baut daraus das maßgebliche Window-/Equipmentbild.
Der produktive jetzige Startup öffnet Format4 (`world.rs` generation_metadata),
nicht Format6. Zunächst kann DUR05 diesen nativen Consumer mit Memory und
explizitem Fresh-Format6store an einen einzelnen echten Appowner anbinden.
Bestehende Format4/5daten brauchen das separat fehlende explizite Format6-
Migrationsticket mit Scopezuordnung; kein blindes Copy, automatisches Upgrade
oder paralleler legacy PlayerStorewriter. Diese Feststellung erweitert den
Besitz dieses Simtickets nicht.

## Interne Implementierungsfreigabe und vollständige Quellenbasis

Root hat den vollständigen Autorenentwurf einschließlich Writertabelle mit
SHA256`a1860957ff0c9482e5bde73035830de4d1c6af9df419d951712a1469b0a2bfd3`
gelesen und intern zur konkreten Umsetzung freigegeben. Die vorhandene
Nutzerautorisierung genügt; keine erneute Betreiberfreigabe. Root verlangt
explizit Schutz sämtlicher Legacy-/Entity-/Tick-/Chunkmapwriter einschließlich
SnapshotOnly-Reaktivierung, Defer erst nach passender Completion mit neuer
local generation und retained Reservation nach jedem Postimagereadfehler.

Vor Runtimebeginn wurden die vollständigen betroffenen Methoden einschließlich
der Callees in **Basis und angewandtem Server** gegengeprüft und22 vollständige
Dateien gehasht. Manifest `.codex-tmp/dur04_1-reference.sha256`, SHA256
`49a63d6218462e18758a2a3fa7dbf4b7648fe32c2d470f36e815e2c7a5f32431`;
Hashprüfung Exit0. Die originalen App-Fixtureinputs sind separat gepinnt in
`.codex-tmp/dur04_1-app-fixture-input.sha256`, SHA256
`e6af2f557fa040bab132900a3bcaffb63214a9243776d1c1cca1d38eacbc352a`.

| Quelle in Basis und Applied | Vollständige konkrete Methods/Callees | Nachgezogene Patches / Grenze |
| --- | --- | --- |
| Player / LivingEntity / ServerPlayer | alle Playerdrop-Overloads, removeFromSelected-call, createItemStackToDrop, Worldadd, Statpfad | LivingEntity und ServerPlayer-Sourcepatches: Consumer/Event vor Worldadd, Countkopie/Statcount; Vanillaeinzelrecords ersetzen keinen gemeinsamen Storagecommit |
| Inventory / ItemStack | hasRemainingSpaceForItem, canHold, getSlotWithRemainingSpace, getFreeSlot, beide addResource/add, removeFromSelected; getMaxStackSize/isStackable/isDamageableItem/isDamaged | Inventory-Sourcepatch: canHold und Komponenten-/Stackableorder; supported FCIS-Modell bleibt strengere Representationpolicy |
| ItemEntity | vollständiges playerTouch, Konstruktoren, target/thrower/delaysetter | ItemEntity-Sourcepatch: Attempt→Legacy→Entityevent, negativeDelay-normalization nur beim möglichen Pickup, Originalcount-Take/Stats; EARfeature: inactiveCounter sind keine Offline-/Unloadticks |
| ServerGamePacketListenerImpl | vollständiges handlePlayerAction | vollständiger Sourcepatch: SingleDrop-spam, Spectator, immobile/hasClientLoaded und optional Equipmentupdate; produktiver Packetconsumer folgt |
| Mth | vollständige SIN-Initialisierung sowie sin/cos | Basis und Applied identische65536-entry-LUT und float-index-Trunkierung; normale mathematische sin/cos allein wäre kein exakter Mthbeleg |

Der konkrete Appcodec und bestehende Tests ab430 liefern unabhängig festgelegte
FCPS3-Offsets,47 Frames, selected8, signed-zero Pos/Look und NaN-/Order-/
Typed/Opaque-Components. Solche Bytefixtures werden vor Simcodec-Roundtrips
festgelegt. Sourceinspection und Parser-/Simproben sind keine gestartete
Paper-Survival-/Plugin-Differentialabnahme.

## Implementierter Autorstand und offene Abnahme

Der genehmigte Entwurf ist auf `eaedabd2` im angegebenen Simscope umgesetzt.
Die tatsächliche Playerauthority ersetzt im opt-in Modus keine Appauthority
heimlich: der produktive Consumer wird erst in DUR-05 atomar angeschlossen.
Keine Storage-/App-/Session-/Core-/Items-/Cargo-/Lock-/Gitmutation in `.1`.

Konkrete zusätzliche Statebefunde aus der Implementierung:

- Private Work/Completionbuffer tragen eine einzige Lease pro Pendingrecord.
  Ein vom Caller behaltenes refused Completionobjekt hält diese Lease und
  verhindert eine zweite Workauslieferung. Leaseablauf beweist ausschließlich
  Bufferfreigabe; readonly Klärung mit ReceiptNone erhält Pending. Erst echter
  no-start Busy/Historycapacity setzt Prepared für identischen Retry.
- Der einzelne private Receiptboxpointer verschiebt den bereits vorhandenen
  Receiptwert, klont ihn nicht. Er liegt im fixed **logical**512Metadatacharge;
  es gibt keine zusätzliche Completionbox. Die encoded/count Caps sind keine
  exakte Summe sämtlicher Rust-/Allocator-/NBT-/RSSbytes. Wiederholte externe
  Callerretention von bounded Snapshots/Checkpoints ist keine Simretention.
- Definitive Fresh-Spawn-CASconflicts geben den ganzen ursprünglichen
  Callerproposal samt Stack/Properties als move-only ItemSpawnRefused zurück.
  Abgewiesene Playerdrops behalten dagegen die tatsächlichen Beforeitems und
  erzeugen keine zweite Refundkopie der gestageten Dropdaten.
- Requestcharge wird nochmals vor tatsächlichen Requestclones geprüft. Das
  feste558Byte Emptybootstrap prüft Pending vor eigener Slot-/Recordallocation.
  Restore-, Owner-, Count-, Byte-, Generation- und Residencygates bleiben
  concrete, keine Callerbool-/Dummyentityauthority.
- Checkpointklärung benutzt für Pairwork einen echten kohärenten Pairread;
  laufende und Restartprüfung kontrollieren die identischen Chunkepochs.
  Nach bestätigtem Partialpickup wird negativer nativer Delay auf0 normalisiert.
- Normale Armorclicks sind insgesamt konservativ verweigert (nicht nur Insert),
  Rezeptanzeige0 ist gesperrt. Opaque-/Removedcapacitypolicy1..3 verweigert
  konkret. Native Cancel ist ein früher konservativer unveränderter Refusal;
  echte Paper-Eventnormalisierung/Pluginstackänderungen fehlen weiterhin.

Der Currentread ist ein tatsächlicher off-tick Readboundary, keine laufende
Storagelease gegen beliebige unabhängige direkte Writer **nach** Evidencecapture.
DUR-05 muss eine einzige tatsächliche Runtimeowner-/Driverauthority pro Scope
und UUID sowie atomaren Inventarownerwechsel sicherstellen. Ein frischer
Currentread nach späterem Backendwrite erkennt supersedete Receipts; das ist
getestet. Eine verteilte Sperre beliebiger Fremdwriter wird nicht behauptet.

### Unabhängige Inputs und tatsächliche Consumerprüfungen

Referenz: Paper1.21.8 Build60,
`29c8822d90899c89d2689338e81a98f690bcba12`. Basis+vollständig angewandte Methoden,
Callees/Featurepatches in22 gepinnten Files; Manifest
`.codex-tmp/dur04_1-reference.sha256`, SHA
`49a63d6218462e18758a2a3fa7dbf4b7648fe32c2d470f36e815e2c7a5f32431`.
Genehmigter Proposal SHA
`a1860957ff0c9482e5bde73035830de4d1c6af9df419d951712a1469b0a2bfd3`.

Appcodec-/Fixturereferenzmanifest
`.codex-tmp/dur04_1-app-fixture-input.sha256`, SHA
`e6af2f557fa040bab132900a3bcaffb63214a9243776d1c1cca1d38eacbc352a`.
Unabhängig per Pythonstruct/MUTF/NBTgrammar gerahmtes
`tests/fixtures/native_player_fcps3/rich_app_layout.bin`:10616Bytes, SHA
`e624803678cdb1fa10293400e3e4094a6bbb756009c6947cdf7d3ed9cc6ee801`.
Der Simdecoder wurde nicht als Fixtureproducer benutzt.

`tests/player_entity_transfer.rs` führt34 konkrete Consumerfälle über tatsächliche
DUR-03 Memory-/redb-Producer aus: Drop1/All, Full-/Partialpickup, Component- und
Countconservation, Auswahl-/Offhand-/Hotbar-/Mainreihenfolge, Ausschlüsse,
Cancel/Delay/Spectator/full inventory, normale Links-/Rechtsclicks,
Doppelpickup/paralleler Autosave, Defer-/Disconnect-/Rejoin-/Stopordnung,
Fresh- und Tombstonebootstrap, no-op CAS, echte Fremdbackend-/Ownerrefusal,
Commit/current-read/Lostreply/Busycutpoints, identischer Retry und historische
Supersession, Workbufferverlust/Readonly-ReceiptNone, whole spawn refund,
exakte Input-/Player-/Pendingbytes und Counts, unloaded target,
Delete/Recreate-ABA sowie echte redb-Reopen mit neuen Wire-/Ownerhandles.

FCPSparser: unabhängige fixe Headeroffsets,47frames und Komponentenbits,
jede Truncation, Versions-/Schema-/Trailing-/Frame-/Shape-/Finite-/Range-/
Selection-/Nestedfehlformen. FCPQ: jede Truncation, feste Kind-/Phase-/Presence-/
Stamp-/Mode-/Nativeoffsets, Oversize/Trailing/Resourceform und positive/fehlende/
supersedete echte Receiptklärung. Faultstores delegieren tatsächliche
Commits/Receipts an die Originalproducer und injizieren ausschließlich Reply-/
Readgrenzen; kein arbiträrer Receipt-/Completionconstructor wird benutzt.

Whole-SimDebug auf dem isolierten Archiv:287 PASS
(174 Unit,108 Integration,5 Doctests); Logs
`.codex-tmp/dur04_1-debug.log`. Owned-Fmtcheck und beide Pinverifikationen PASS.
Release, finale All-target-Clippy und Rustdoc sowie exakter Sourcefreeze folgen
in der abschließenden Autornotiz. Autortests sind keine unabhängige CLEAN-Abnahme
und keine gestartete Minecraft-/Paper-Differentialabnahme.

### Konkreter DUR-05-Anschlussvertrag

1. Fresh Format6-/Memorybackend und Generationbinding ausdrücklich starten;
   aktuelle App startet Format4. FCPS1/2 und vorhandene Format4daten dürfen
   weder umetikettiert noch still ignoriert werden. Bestandsmigration bleibt
   eigenes fehlendes Ticket beziehungsweise typed refusal vor Admission.
2. Pro Scope/UUID genau einen Simowner und begrenzten externen Writerdriver
   bestimmen. Vor dem Besitzwechsel vorhandene Connectioninventory und
   Sessionwriters abschalten, echte Snapshotadmission vorbereiten und im
   Boundary den Owner atomar ersetzen. Keine parallele schreibbare Kopie.
3. Actioninputs mit aktuellen opaque Owner-/Mutationhandles am Boundary senden;
   Producerwork move-only in bounded queue ausliefern. Completionprefix vor
   neuen Actions verarbeiten; volle Inputqueue gibt Completion intakt zurück
   und Driver hält dessen Lease bis erneuter Admission.
4. Nur `PlayerAdmitted/Changed`, `ItemSpawned/Changed/Removed`, `Take/Stat`,
   `Resync` und Refusedspawnrefund aus diesem tatsächlichen Consumer auf
   vorhandene Session-/Tracking-/Packetpfade übersetzen. Kein Clientpostimage
   vor receiptbestätigter Publication. Keine erfundene Javaeventabnahme.
5. Native entity load/unload mit UUID-/Wireallokation/Residency einschalten,
   Dirtyautosave/Disconnect/Drain/Stop und Lostreply/Restart cutpoints real testen.
   Ohne anschließende SRV-02.1/DUR-04.2timers dürfen Delay40drops nicht als
   spielbar abgenommen werden. Pose, collision, tracking und echte Paper-
   Differentialspiele bleiben weitere explizite Checks.

## Abschließender Autorfreeze

Simsource unverändert ab jetzt:43 Files in
`.codex-tmp/dur04_1-source.sha256`, Manifest SHA
`de0b202ab2fe52246758bddc1f8ab3ac7b0d8ff7c5f6c26201786e640fc94aca`.
Reiner Simsnapshot `.codex-tmp/dur04_1-source-freeze.tar.gz`, SHA
`6bc4cae0a1ede74ae4955ad187744e67139393655089595ad2f1d84614e6979e`.
Baseline für isolierte Abnahme bleibt `eaedabd2`; andere laufende World-/App-/
Proto-/Docboardarbeiten sind kein Input des Autorcheckouts.

Alle endgültigen Checks mit Rust1.99.0, `-j2` und ausschließlich diesem Archiv:

- `cargo test -p ferrumc-sim`:287/287 PASS
  (174 Unit,108 Integration,5 Doctests).
- `cargo test -p ferrumc-sim --release`:287/287 PASS.
- `cargo clippy -p ferrumc-sim --all-targets -- -D warnings`:PASS.
- `cargo fmt -p ferrumc-sim --check`:PASS.
- `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps -p ferrumc-sim`:PASS,
  ohne `--document-private-items` oder Warningallows.
- Source-/Appfixturepinchecks und abschließender Simsourcechecksum:PASS.

Logs jeweils `.codex-tmp/dur04_1-{debug,release,clippy,fmt,rustdoc}.log`.
Rustdoc deckte sechs bestehende private Kommentarlinks in SimShard/WorldTime
auf; sie wurden nur zu sichtbarem Text korrigiert. Diese zusätzliche reine
Kommentaränderung in `src/time.rs` gehört zum Simfreeze und berührt keine
Time-/Gameplaylogik. Whole-Debug/Release und strict checks wurden danach auf
dem endgültigen Source wiederholt.

Das maschinenlesbare Resultat und Checksum-Snapshot stehen in
`.codex-tmp/dur04_1-author-results.json` und
`.codex-tmp/dur04_1-owned-freeze.sha256`. Der Brief wird zusammen mit diesem
Autorstand für Fremdabnahme eingefroren. Keine Commit-/Gitintegration durch den
Autor. Root führt einen getrennten frischen Archiv-/Storage-/State-/Racecheck
vor CLEAN-Abnahme aus; Autor-PASS ersetzt ihn nicht.

Nach diesem Freeze steht ein separater, noch nicht runtimefreigegebener Entwurf
`.codex-tmp/dur04_2-proposal.md` für DUR-04.2/SRV-02.2 zur Rootprüfung bereit.
Er behandelt tatsächliche Slotdirtyauthority, confirmed versus live Counter-
beforeimages, einmalige logische Counterticks, Pendingzeit/Eventgrenzen,
bounded Entityautosaves und Unload/Restart sowie echte Physics/Activation/
Merge-/Despawnabhängigkeiten. Keine Erweiterung des eingefrorenen `.1`.
Der Gesamtauftrag bleibt vollständiges spielbares Survival einschließlich
Entities/Nether/End/Dragon und Paper-/Pluginstrecke; diese Abhängigkeiten sind
kein Verzicht auf den vom Nutzer angeforderten Umfang.
