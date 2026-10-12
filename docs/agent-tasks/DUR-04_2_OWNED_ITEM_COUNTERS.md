# DUR-04.2a / SRV-02.2 — Authorimplementation zur unabhängigen Prüfung

Ausgangspunkt ist der unveränderte DUR-04.1-Simfreeze
`de0b202ab2fe52246758bddc1f8ab3ac7b0d8ff7c5f6c26201786e640fc94aca`.
Kein Code dieses Folgetickets wurde in `.1` aufgenommen. Basis ist die inzwischen
unabhängig CLEAN integrierte `.1` auf `4f7d5037`. `.2a` ist im Autorcheckout
umgesetzt und wird separat geprüft; produktiver Appbetrieb benötigt DUR-05.

## 1. Originalphase und enger Folgezuschnitt

Paper1.21.8 Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`, vollständig angewandtes
`ItemEntity.java`: `inactiveTick`134ff und `tick`156ff wurden nochmals gelesen.
Beide Pfade dekrementieren nur positive Delaywerte außer32767, inkrementieren
Age außer-32768 mit Java-i32-Wrapping und prüfen danach exakt signed
`age >= despawnRate`. SentinelAge wird nicht zusätzlich von einer negativen
Ratenprüfung ausgenommen. Health<=0 ist kein erfundener Timertod.

Active Originalordnung: super/base tick → Delay → Water/Lava/Gravity →
noCollision/noPhysics/escape → move/Blockeffekte/friction/bounce → Merge
(bewegter Block: alle2ticks, sonst40) → Age → Fluidpushing/Impulse → Despawn.
Inactive enthält Counter und Despawn, aber keine aktive Bewegung/Merge.
Cancellation des echten DespawnEvents setzt Age auf0; Discard folgt nur bei
nicht abgebrochenem Event. Originale Itemmerge verlangt passende TargetUUIDs,
exakte effektive Components/Cap, Events und gemeinsame Änderung beider Items.

`.2a` aktiviert echte ownerlokale Delay/Age- und Dirty/Saveauthority. Sie nennt
sich Counterconsumer und beansprucht keinen vollständigen aktiven Itemtick,
keine Physik/Events/Activation/Paperparität. Delay40 muss nach40 zugelassenen
residenten logischen Counterticks0 sein. Die oben genannten Zwischenphasen
werden ausdrücklich von `.2b`/folgenden Tickets besessen, nicht übersprungen
und danach als vollständige Vanillaabnahme gekennzeichnet.

## 2. Wirkliche Slotauthority, bestätigter Beforestate und positive Caps

Der vorhandene echte Entityslot erhält neben OwnedEntityExpectation:

- last_confirmed_native_record (inklusive durable Chunk),
- current_dirty_generation und saved_generation (positive, nichtwrappend),
- last_counter_tick (für strikte einmalige eligible Tickclaim),
- konkret gebundenen pending query/Generation und begrenzten Counterdeferstate.

Aktueller Itemstack/Properties/Transform bleibt genau derselbe private Slot.
Keine zweite schreibbare Entität. Countermutation ändert nur diese Properties,
nicht FCIS/Components; sie braucht keine Encode/Clone/DBoperation pro Tick.
Nur eine tatsächlich unterschiedliche native Property/Transform/Stackänderung
markiert Dirty und erhöht die vorgeprüfte lokale Generation.

Der bestätigte Record ist zusätzlich retained data. Er muss **vor Admission**
in einen expliziten positiven confirmed-native-bytecap eingehen; nicht die
bisherige16MiB-Livebodycap so darstellen, als decke sie diese zweite Kopie.
Counter-/Generation-/Defermetadaten wachsen nur mit vorhandenen Entityslotcaps.
Savecount, Savechargedbytes, Completioncount/bytes und pro-owner logical Defer
bleiben positiv und explizit. Alle Decode/Clonepfade prüfen die gemeinsame
aktuelle + confirmed + Pendingrepresentation zuerst; keine RSSzusage.

## 3. Einmaliger Tick und konkrete Pendingordnung

Der Consumer benötigt eine wirkliche logische Tickidentity: `.1` besitzt im
SimShard noch keinen solchen Counter und seine alte Methode nimmt keinen Tick.
Entwurf: zusätzliche checked `run_tick_with_player_ownership_at(Tick)` mit
strict next-Tickclaim; nach Opt-in Countermode darf der alte Entry diese Claim
nicht umgehen. Scheduler/App muss denselben echten Tick durchreichen. Doppelte,
alte und sprunghafte Tickinputs verweigern **vor** Inbox-/Propertymutation.

Boundary: genuine Completion/Reconcileprefix → Deferpublikation → konkrete
Actions → einmalige Counterphase über private ownerlokale registrierte Items.
Die Phase benutzt vorhandene bounded Chunkbuckets/Slots und wirkliche Resident-
facts; unloaded/offline bekommen keine Schritte und kein Wallclockcatch-up.
Resident ist noch keine native Paper ActivationRangeentscheidung. `.2a` weist
das offen aus; `.2b` muss active/inactive Eligibility tatsächlich bestimmen.

Pending hält weiterhin den ganzen gebundenen Postimage unverändert. Counter-
Mutation des reservierten Items während Pair/Save ist verboten. Es darf weder
nach einem Receipt der alte Timerzustand einen neueren Zustand überschreiben,
noch darf ein Tick still doppelt zählen. Wegen `.1`s whole-store Reservation
muss `.2` die Behandlung auch der übrigen Slots ausdrücklich entscheiden.
Vorschlag für Rootreview: ein bounded logischer Deferledger mit exakt geprüften
eligible Tickclaims, keine Wallclockzeit und keine erlaubte zweite Worker-
auslieferung. Completion wendet den ledger erst auf den bestätigten Rest-/Spawn-
postimage an; Fullpickup/Despawn-Tombstone entsorgen keinen zweiten Livekörper.

Counterdefer darf nicht pauschal alle Schritte arithmetisch komprimieren, wenn
Despawn-/Cancelphasen durchlaufen würden: solche Ereignisse ändern Age und
Aufrufordnung. `.2a` darf nur den eventfreien Counterbereich exakt falten;
bei fälliger Eventgrenze benötigt der konkrete Driver eine typisierte begrenzte
Dueentscheidung. Ungeklärte Commitauthority behält Pending statt Itemverlust.
Ledgercap/+1, Generationoverflow und anhaltend verlorene Worker müssen einen
konkreten Backpressure/Drainpfad erhalten. Eine dauerhafte Pause während DB-
Pending wäre eine explizite Gameplayabweichung, keine native Timerparität; für
quarantinierte Pendings ist genau diese Abweichung in §11 deklariert.
Vor Runtime muss Root diese Pendingzeit-/Eventgrenzentscheidung prüfen.

## 4. Pickup/Save benutzt confirmed Before und aktuellen live Postimage

Die aktuelle `.1`-Pickupvalidierung encodiert den gegenwärtigen Itemslot und
vergleicht ihn mit dem echten DBread. Nach ownerlokalen Timern ist das **falsch**:
DB hat den last-confirmed Age/Delay, live hat weitergetickte Properties.

Der `.2`-Planner bindet daher genau:

1. echten Pairread an last_confirmed record + expectation/stamp,
2. aktuellen dirty/local generation und tatsächlichen Inventory/Itemhandle,
3. konservierende Stackplanung aus dem gegenwärtigen Liveitem,
4. joint Postimage aus aktuellen Countern/Transform + neuem Rest/Playerzustand.

Pending preflight/reserviert beides; Requestbefore ist der bestätigte DBrecord,
Requestafter ist der vollständige aktuelle Postimage. Retry wiederholt denselben
Snapshot, keine neuen Timer/RNG/Items unter derselben Query. Matching receipt +
real current evidence bestätigt saved_generation und last_confirmed_record.
Anschließende Defermutation erhöht live dirty generation wieder; sie ist kein
Saveack. Falls live saved generation bereits überholt ist, darf ein alter Ack
Dirty nicht löschen. Entity-only No-opCAS muss original Revision behalten;
`.1`s entity changed=true ist nur für Spawn/Pickup gültig und darf nicht blind
für spätere Autosaves übernommen werden.

## 5. Bounded Autosave, Unload und Neustart

Ein konkreter EntityAutosaveInput nimmt echten Entityhandle + dirty generation,
keinen generic callback. Eine fair fortgesetzte ownerlokale Saveauswahl scannt
höchstens das konfigurierte Countbudget pro Batch. Nur diese ausgewählten dirty
Records werden encoded/clone-budgetiert und als echte DUR03entityCASwork
versendet; keine CAS/Serialization aller Items in jedem Tick.

Vorgeschlagene kontrollierbare Defaultsaveperiode100 logische Ticks plus Count-
und bytebudget, unabhängig von20TPSwallclock; Operatorcaps sind ausdrücklich.
Drop/Pickup/Despawn, kontrollierter Unload und Stop triggern Prioritätsflush.
Timeout/Busy/QueueFull behalten dirty/pending und exakte Workauthority. Last-
ticket release entzieht Tickeligibility, darf einen dirty/pending Körper aber
nicht aus seinem Owner entfernen. Eviction/Stop erst nach tatsächlicher
confirmed generation; native Registrierungen bleiben bis dahin gespeichert.

Neustart lädt ausschließlich bestätigte native FCIE/FCPS mit frischen lokalen
und Wirehandles; positive Pendingreceiptklärung zuerst. Keine Offline-/
Wallclockcounter. Ein Prozesscrash vor Autosave verliert höchstens deklarierte
unbestätigte Timer-/Transformfortschritte; Itemmengen/Stackownership dürfen
keine zweifache Veröffentlichung oder Dropverlust bekommen. `.1` konserviert
Menge an jeder bestätigten Transfergrenze. Ein Write-ahead-WAL für jeden Tick
wäre ein eigenes Persistenz-/Performanceagreement, kein notwendiger DB-CAS/Tick.

## 6. Physics-/Survivalabhängigkeiten ausdrücklich eigenständig

- `.2b`: echte EntitybaseTick/Fluid-/Gravity-/Collision-/support-shape-/friction-
  und Blockeffektproducer aus gepinnten Methoden + bounded Chunkquery. Bewegung
  über Chunk-/Shardgrenzen benötigt konkrete residency-/ownerhandoffauthority;
  nicht force-load, clamp oder ungeprüften Mutcallback. Motionänderungen bleiben
  ownerlocal dirty, Tracking/Packets konsumieren konkrete Intents.
- `.2c`: wirkliche ActivationRange-/entityTicking-Eligibility und Multiplayer-
  contact/pickup order; `.1` proximity envelope ist keine Collisionabnahme.
- `.2d`: DespawnDue/Eventcancel/confirmed durable Deleteconsumer und Owner-
  lifecycle. NegativeRate/Sentinel/Wrapfälle original prüfen. Javaeventbridge
  bleibt eigene aktive Abhängigkeit, ein Callerbool ist keine getestete Bridge.
- `.2e`: native Merge ist ein Zwei-Entity-Ownershiptransfer. Der jetzige DUR03-
  Store besitzt Player+EntityPair und Entitysingle, **keinen** atomic Entity+
  Entitypairproducer. Zwei Singlesaves sind keine atomare Mergeconservation.
  Vor Merge eine eigene Storageauthority mit receiptgestütztem gemeinsamen
  Postimage liefern; durchgehende Komponenten-/Target-/Count-/Stateabnahme.
- DUR-05: tatsächliche Format6-/Memoryadmission, atomarer Appinventoryowner-
  wechsel, bounded real Driver, Load/Unload/Tracking/Client-/Restartcutpoints.
  Der heutige AppFormat4start und Bestandsmigration bleiben separate Lücken.

Meaningful `.2`-Abnahme:40 echte resident Counterticks versus unloaded0ticks;
duplicate Tick/legacy bypass; positive/max Delay,32767, Age-32768, signed wraps,
negative dueRate; dirty counter live != confirmed DB und erfolgreicher Partial/
Fullpickup; Autosave ack/defer/resume und no-op Revision; Pickup↔Save↔Unload↔
Stopreservation; Delay/Age während genuine Busy/Uncertain/Lostreply und Due-
Eventgrenze; Restart mit tatsächlich erneut geöffnetem redb. Keine Bezeichnung
als vollständiger Itemtick/Survival bevor Physics/Tracking/Events live abgenommen
sind. Sourceinspection allein ist keine gestartete Paper-Differentialabnahme.

## 7. Präzisierung nach Rootreview auf integriertem `4f7d503`

Neue Basis: unabhängig CLEAN integriertes DUR-04.1
`4f7d50371a8a12560a9d93957a70ac009dc6b9bb` einschließlich Rootconsumerproben.
Der ursprüngliche `.1`-Freeze bleibt unverändert. Diese Präzisierung ersetzt
oben offene Alternativen. Die Autorimplementation folgt diesen präzisierten
Verträgen; die damalige Entwurfsfreigabe war keine produktive Appabnahme.

### Positive Ressourcenwerte und tatsächlicher Speicherbesitz

Ein neues checked `OwnedItemCounterLimits` deklariert alle Werte positiv;
Operatoren dürfen sie absenken, nicht über folgende compile-time Obergrenzen
hinaus erweitern. Konkrete Defaults:

| Größe | Default / Maximum | Ablehnungsverhalten |
|---|---:|---|
| Retained-confirmed native FCIEbytes/Shard |16MiB /16MiB|vor Recordclone/Admission refuses unverändert|
| Deferclaims pro gebundenem Item/Pending |1024 /1024|Tick1025 preflightrefusal, RecoveryRequired|
| Private counter ledger entries |1 pro Entitypending; max1 aktiv gebunden/Shard, dazu eingefrorene quarantinierte bis zu den §11-Grenzen|keine zweite aktive Entityoperation|
| Neue Entitysavejobs je Boundary |1 /1|kein weiterer candidate/clone|
| Shared Pendingcharge |bestehende8MiB /8MiB|vor Postimage/Requestclone|
| Entitysavecharge je Job |2MiB /2MiB|vor Postimage/Requestclone|
| Residency-/Dirtyscan je Boundary |64Slots /256Slots|faire Fortsetzung, keine fullscanbehauptung|
| Prioritydue-/flushmarks |vorhandene Entityslotcap, max65536|ein Bit + private gebundene Metadaten pro Slot|
| Noch unbeantwortete Dueentscheidung |1 /1|kein zweiter Eventauftrag, global recovery bei Deadline|
| Completion/Inputqueue |bestehende32 /32 und8MiB /8MiB|vollständiger move-only Refusal|
| Savefälligkeit nach letzter Bestätigung |100 logische Ticks /konfigurierbar bounded u32|weder Wallclocktimer noch garantierte Latenz|

Die aktuelle native Livebudgetgrenze bleibt max16MiB, getrennt von den neuen
max16MiB confirmed bytes. Daraus folgen max32MiB geprüfte retained native body-
bytes vor weiteren vorhandenen Pending-/Inboxcharges. Es ist keine32MiB-RSS-
Zusage; FCIS/NBTdecoded heap und Maps/Handles/Allocator sind ausdrücklich anders.
Die kleinen Ledger-/Prioritymetadata wachsen ausschließlich innerhalb der realen
Slotcap, nicht mit Tickzahl. Ein Ledger speichert höchstens einen Countertripel-
Postimage plus Count/first/last Tick-/Generationidentity;1024Steps brauchen kein
Vec mit1024Einträgen. Counterprojektion wird trotzdem pro logischem Tick exakt
berechnet, auch an Wrapping-/Duegrenzen. Die1024Grenze begrenzt unbestätigte
logische Fortschrittsauthority, nicht zufällig einen Arrayumfang.

Native Spawn/Restore/Partialpickup/Autosave reservieren den zusätzlichen
confirmed-bytepostimage vor Clone. Der momentan last-confirmed Record bleibt
unverändert, solange kein passender genuine Receipt/currentread vorliegt.
Fullpickup/Despawn entfernen nach Bestätigung dessen retained charge exakt;
refused Spawn hat vorher keinen live confirmedbody. Snapshotclone bleibt
SnapshotOnly und muss beide bodybudgets vor seinem ersten Clone prüfen.

Entitysavecharge exakt: bestehendes `661 + 2*beforebytes + 5*afterbytes`,
bei je256KiB max1,835,669Bytes, also kleiner als2MiB. Andere Pairjobs behalten
die strengere bestehende Sharedpendingcap. Der gleiche `.1`-Workbufferlease-
vertrag gilt; kein zusätzliches Event-/Saveobjekt darf eine zweite Worker-
auslieferung erzeugen. Eigene scalar timer outputseams passen in vorhandene
Slot-/Countbudgets, verlangen keine FCISclone pro Tick.

### Tickidentity und klarer Mutationsschnitt

`initialize_owned_item_counters(config, tick_baseline: Tick)` übernimmt einen
konkreten letzten abgeschlossenen echten DriverTick. Ein neuer Server startet
mit explizitem Baseline `Tick::ZERO` (raw0); erster zulässiger Tick ist1.
Bei Anschluss an laufenden App-/Schedulerbetrieb wird der tatsächliche zuletzt
abgeschlossene Tick verwendet, nicht0 geraten. `run_tick..._at(Tick)` akzeptiert
nur `last.get().checked_add(1)==Some(tick.get())`. Maxu64 verweigert typisiert
vor Mutation; es wird nicht gewrappt. Gaps und DuplicateTick sind kein Catchup.

Vor jeglichem Inputdrain werden Tickclaim, jeder resident counter-/defer-
Generation-/1024-/Duecap und Jobbudget preflighted. Scheitert das, werden Tick,
Inbox, Completionlease, Workrequest, before/afterRecords und actual properties
nicht verändert. Ein eigener **completion-only recovery boundary** verarbeitet
nur bereits echte Completion/readonly Clarify und Recovery/Drain, erzeugt keinen
Countertick und führt keine neuen Gameplayinputs aus. Sonst könnte die volle
Ledgercap genau das Completion verhindern, das sie wieder freigibt.

Normaler Erfolgsboundary:

1. Alle admitted genuine Completions in Queuefolge;
2. receiptbestätigte Veröffentlichung und passende private Counterdeferauflösung;
3. Actions in FIFO einschließlich Autosave/Drop/Pickup/Despawnentscheidungen;
4. exakt ein resident Counterstep pro dafür preflightetem lebendem Item;
5. bounded fair Saveauswahl/Output, danach genuine Driverarbeit off-tick.

Ein in1/2 dieses Boundaries tatsächlich bestätigter Spawn ist in4 bereits ein
lebendes resident Item und erhält **einen** Step (Delay40→39). Ein erst in3
vorbereiteter Spawn ist noch unsichtbarer Workpostimage, erhält bis zum ersten
Receipt keinen Schritt und keinen fiktiven Deferledger. Fullpickupentfernte Items
bekommen in4 keinen Schritt. Partialpickup erhält auf den bestätigten Rest erst
seinen vorherigen Defer und dann den aktuellen Step. Dieses Modell wird als
konkrete eigene Boundaryordnung differential überprüft; keine unbelegte
Behauptung identischer Paperentitytick-before-playerTouch-Integration.

### Strukturreservation, echte Counterclaims und Pending

Keine breitere per-slot Reservation in `.2a`: weiterhin höchstens **eine**
aktive Entityoperation pro Shard; eine quarantinierte gibt die Reservation am
Ende ihres Quarantäneboundaries frei und zählt danach nicht mehr (§11). Die
bestehende ganze Storestrukturreservation bleibt
für Spawn/Remove/Stackreplace/ownerhandoff zuständig. Ein enger privater
counter-only method darf ausschließlich Delay/Age/Dirtygeneration der **anderen**
vorhandenen Slots ändern. Er verändert keine UUID-/wire-/bucket-/slotindex,
keinen Stack, kein Nativebodylength und kein `PreparedOwnedEntity::used`.
Alle anderen Struktur-/Payloadwriters behalten die Reservationgates.

Für das konkrete gebundene vorhandene Item bleiben visible Properties und
gefrorener Requestafter intakt. Ein privater `DeferredOwnedCounters` im echten
Pendingrecord bindet ownerincarnation, Handle/UUID, pending query/generation,
logische Start-/LastTick und count<=1024. Er projiziert scalar Fortschritte aus
**diesem** gefrorenen Itempostimage, nicht aus Callerbooleans oder späterem DB.
Nicht zu einem gebundenen Item gehörige Player-only Work blockiert keine Timer.

Bei Pickuprest-/Saveconfirmation wird zuerst genau Requestafter als genuine
confirmed Record veröffentlicht. Danach wird die private Projection auf den
Rest desselben Items angewandt und Dirtygeneration vorgeprüft erhöht. Bei
Fullpickup/Despawnremove existiert kein Rest; Deferfortschritte gestatten weder
Doppelspawn noch eine nachträgliche zweite Itemquelle. Definitiver No-start-
Refusal wendet Projection auf das ursprüngliche tatsächliche Liveitem an und
behält seine passende bestätigte Recordauthority. Conflict/Superseded bleibt
quarantined und ermächtigt keine neue CAS gegen erfundene Erwartungen; die
Projection wird eingefroren und nie angewandt, das Item abgeschirmt (§11).

Die zusätzliche private Projection darf nicht als Receiptconfirmed Timefield
oder Storagebefore benutzt werden. Jeder neue Transfer bindet last-confirmed
Record/Stamps und die konkrete aktuelle dirty generation; seine Afteritems
kommen aus dem tatsächlichen freigegebenen Livezustand. Savedgeneration folgt
nur dem gebundenen Snapshot. Nach Defer ist sie kleiner als livegeneration;
Dirty bleibt gesetzt, alter Saveack kann es nicht clearing.

### Duegrenze und kein stilles Pausieren

`.2a` bekommt eine ausdrückliche `VanillaNoPlugins`-Despawnpolicy: bei signed
Due wird ein ownergebundener Dueintent/mark gesetzt; keine Callercancelbool.
Diese Auswahl liefert keine Java-/Papereventabnahme. `.2d` ersetzt sie später
mit einer tatsächlichen sealed eventconsumerdecision; Pluginbetrieb ohne diesen
Consumer wird nicht freigegeben.

Ist das Item frei, beendet erster Duecounterstep dessen Gameplayzulassung und
Prioritätsdelete wird über den realen DUR03EntityCASprepared. Der native Körper
bleibt bis Receipt im realen Owner; Clients erhalten erst dessen bestätigte
Removal. Wiederholte Ticks führen nicht erneut einen Event-/Deleteauftrag aus.
Keine Insertion/Pickup/Merge aus einem bereits fälligen Item.

Fällt Due in eine gebundene Counterprojection, trägt sie genau firstDueTick;
weitere claimed Countersteps bleiben bis1024 zählbar, erzeugen aber keine
zusätzlichen Dueentscheidungen. Matching Partial-/Savecompletion führt den
freigegebenen Rest in PendingDespawn; Fullpickup entfernt den Körper und
verbraucht keine weitere Deleteauthority. Das ist die deklarierte konservative
Durabilityabweichung: uncertain bereits gestarteter Pickup ist nicht nachträglich
abortierbar. Native Tick/Eventparität an diesem DB-Cutpoint ist nicht behauptet.

Bei künftigem echten Cancel muss die Decision mit dem exakten owner/handle/
firstDueTick/state generation gebunden und move-only sein. Age reset0 am
ursprünglichen Eventboundary; nachfolgende claims müssen auf diesem resetstate
replayed werden. Das ist kein arithmetisches Nstepsadd und kein pauschaler
Sentinelskip. Ein verspätetes/ungebundenes Callerbool entscheidet nichts.
`.2a` implementiert solche Cancellation noch nicht.

**Ledger1024/+1, unbeantwortete Decisiondeadline oder lokale Generation-/
Tickoverflow:** next logicalTick verweigert vor Mutation mit konkretem
`RecoveryRequired`/`GenerationExhausted`/`TickOverflow`. Driver führt nur
completion-only recovery und expliziten controlled Drain aus. Er darf den
fehlgeschlagenen Tick weder als erfolgreiches20TPSGameplay noch als getickte
Items reporten. Falls genuine readonly Receipt fehlt, Pending bleibt retained;
kein Autoabort, Itemdroprefund oder frisches IDreplan. Dauerhafte Uncertainty
macht den Shard ausdrücklich recovery-required, nicht still stehengebliebene
Items bei behaupteter erfolgreicher Weltzeit. Keine Wallclocktimeoutauthority.
Eine Quarantäne ist keine Uncertainty: Sie bindet nach ihrem Quarantäneboundary
keine Claims und führt nicht in diesen Modus; ihr Item ist typisiert
abgeschirmt statt still stehengeblieben (§11).

Das löst nicht perfekte ununterbrochene native Timerparität bei beliebig langer
DBuncertainty; mit strikt unverändertem Requestafter ist sie ohne einen extra
logischen Defer-/Eventledger nicht belegbar. Der hier deklarierte bounded
Recoverymodus ist die konkrete `.2a`-Fehlersemantik. Nutzerziel voll spielbarer
Survival bleibt erhalten; aktive Multiplayer-/Paperabnahme muss dieses failure-
modeagreement mit DUR-05 testen statt es zu verschweigen.

### Starvationfreie Savefortsetzung bei einem Entityjob

100Ticks bedeutet **fällig**, nicht guaranteed5s-confirmed. Mit4096 dirty Items,
max1Job/Boundary und DBSingleowner sind selbst bei erfolgreicher20TPSAbnahme
mindestens4096Boundaries≈204.8s erforderlich, zuzüglich Scan-/Workerlatenz.
Kein gegenteiliger Performanceclaim. Performanceoptimierung zu echter Multi-
resource/Batchauthority ist ein eigenes reviewbares Storage-/Reservationscope.

Zwei private round-robin Cursor über vorhandene slots (opaque slotgeneration,
keine unbeschränkte UUIDliste): normal dirty und priority due/unload/stop. Jede
Boundary darf insgesamt64Slots prüfen. Cursor bewegt sich **nach jedem wirklich
geprüften Slot**, auch clean/unloaded/reserved/Caprefusal. Wrap überspringt retired
slots und bindet bestehende Generation; Spawn kann alte Cursors nicht zu einer
neuen falschen Handleauthority machen. Pro Selection wird fair resümiert, nicht
immer der kleinste UUID gescannt.

Priorityflush bekommt drei von vier freien Selectionturns; jeder vierte Turn
gehört zwingend normaldirty. Innerhalb jeder Lane ebenfalls round-robin.
Slotmarks sind bitweise vorhanden, keine unbounded priority queue. Bei Stop/
Unload bleibt jeder betroffene Slot markiert; due mark entzieht Gameplay sofort,
Deleteflush hat innerhalb der Prioritylane denselben fairen Cursor. Ein permanenter
Stream von Prioritäten darf normale fällige Saves nicht vollständig verdrängen.

Wenn schon ein aktives Entitypending die Storestruktur reserviert: **keinen**
neuen Savecandidate encodieren, keine Cursor-/Turnfortschritte, nur tatsächliche
Counter-/Deferphase. Ein quarantiniertes Entitypending reserviert ab seinem
Folgeboundary nicht; die Auswahl läuft weiter und überspringt nur den
abgeschirmten Slot (§11). Nach genuine
Completion/Refusal kann derselbe Boundary wieder auswählen. Bei no-start Busy/
Historycapacity bleibt der identische Pendingrequest unter derselben Query;
kein Replan und kein Auswahlcursorreset. Der Savecursor steht schon hinter
seinem bei ursprünglicher Admission geprüften Slot. Wiederholt Busy blockiert
wegen der ausdrücklich einzelnen Authority weitere Jobs; das ist Backpressure,
keine behauptete Fairness trotz eines nicht abgeschlossenen Jobs. Retry erhält
mindestens denselben logischen turn, keine neue RNG-/Count-/Stackauswahl.

Bei local count/byte/generationadmissionrefusal bleibt Dirty/priority mark
intakt, Cursor steht nach dem geprüften Slot; er wird in einem späteren Umlauf
wieder geprüft. Unveränderliche Nichtadmissibilität (etwa configured confirmedcap
kleiner als dieser native Record) muss typed sichtbar werden und beim Drain
`NotQuiescent` erklären; sie darf nicht durch unendlichen Erfolgsemptyloop als
Saveack gelten. Hardgenerationoverflow geht in RecoveryRequired statt unfairen
weiteren Spin. Due/Unload/Stop erfordern echte Receiptbestätigung, nicht dass
Saveauswahl den Slot einmal besucht hat.

### Entityno-opCAS und exact changed-check

Neuer entity-only Save vergleicht schema/chunk **und komplette Nativebytes**
von last-confirmed Before und captured Liveafter. Sind sie identisch, ist
`changed=false`, der genuine Storereceipt hat denselben Stempel/Revision und
keine affected-chunk-Epochänderung; Bestätigung darf saved generation nur auf
die bereits gebundene Capturegeneration setzen. Ohne native Änderung ist keine
andere Revision erfunden. Bei echter Veränderung erwartet die native Prüfung
Revision+1 und jeweils genuine before/after affected chunk epoch+1. Spawn,
Partial-/Fullpickup und Delete erhalten ihre jeweilige wirkliche Changedshape.
Der `.1` helper `changed_stamp(..., true)` für Entitytransfers wird nicht blind
auf Autosave angewandt. Die Epochhelper muss no-op leere Epochs erlauben;
sonst wäre echte identische EntityCASreceipt fälschlich refused.

Vor Runtimeabnahme liefern: exact empty/no-op entity save versus dirty timer save
auf Memory/redb,40steps/new-same-boundary spawn, full-/partial pickup+defer,
1024/+1 preflight und completion-only recovery, generation/Tickmax, negative
rates/sentinels/wrap, unrelated counters während Strukturreservation,
64scanboundary/Fairnesscursor/priority3:1 und reale Retrybusy/Uncertain, Stop/
Unload und tatsächliche redb-Reopen. `.2a` trägt keine vollständige Physics-
oder Paper-/Javapluginabnahme vor.

### Ergänzende genaue Grenzsemantik

Jeder echte Counterstep prüft den neuen scalar dirty generation Wert vor der
Propertyänderung. Für gebundene Defersteps wird eine eigene positive checked
projection generation erhöht; sie gehört ausschließlich zum Pending und ist
keine durable Revision. Beide maxu64 verweigern den nächsten logicalTick vor
allen Mutation-/Drainphasen. Bei der Confirmation werden release-generation
und der maximal benötigte Deferpublicationzuwachs vorgeprüft, bevor Pending aus
der Tabelle entfernt wird. Eine Overflowrefusal darf seinen echten Commit
nicht als Abort behandeln; Completion bleibt intakt und Pending unresolved.

Bei Dueintent in `VanillaNoPlugins` entsteht keine offene Eventdecisionfuture,
sondern ein sofort checked native Delete-/Prioritymark für den realen Slot. Die
Tabelle max1 offene Decision betrifft nur den künftigen tatsächlichen `.2d`-
Eventconsumer. Dafür ist die vorgeschlagene positive Decisiondeadline konkret
1024 **zugelassene logical Counterclaims** pro ownergebundener firstDueDecision,
entsprechend dem Ledgercap; Step1025 wird RecoveryRequired. Keine wallclock-
Sekunden oder Inputcaller-Timeouts dürfen diesen Beleg ersetzen. `.2a` führt
weder solche Future noch die1Eventqueue ein, bis `.2d` den tatsächlichen Producer
liefert; die markierten max65536 Slots sind keine vorgetäuschten Javaevents.

Die tatsächliche Storage-no-op-Semantik wurde zusätzlich read-only geprüft:
`ferrumc-storage/src/ownership/engine.rs::next_stamp`397ff und Entityplan497ff,
`ownership/raw.rs::receipt_epochs`237ff. Identische Entityputbytes erzeugen keine
Planänderung, halten den Stempel und haben leere Epochs; Originalrawvalidation
verlangt bei no-op auch identische Before-/Afterchunk. `.2` übernimmt genau
sowohl Body-/Chunkvergleich als auch diesen shapeabhängigen Epochvertrag.

## 8. Letzte Rootpräzisierung: Fullresidenttick und readonly Boundarypreflight

**Counterphase ist vollständig und unabhängig vom Savecursor.** Sowohl
Counterpreflight als auch erfolgreiche Counterphase besuchen pro tatsächlichem
logicalTick sämtliche vorhandenen ownerlokalen, residenten berechtigten
Itemslots bis zur bereits positiven Entityslotcap (max65536). Jeder bekommt
exakt einen Step, gegebenenfalls in seine gebundene private Pendingprojection.
Unloaded/offline und quarantäneabgeschirmte Slots (§11) erhalten0;
RecoveryRequired meldet keinen erfolgreichen Tick.
64/max256 ist ausschließlich der budgetierte **Saveauswahlsweep**, niemals
Activation-/Countereligibility oder Sampling nur64Items/Tick. Bei4096residenten
Items werden in40erfolgreichen Ticks alle4096Delay40-Werte0, abgesehen vom
explizit definierten späteren ersten Spawn-Tick und abgeschirmten Slots (§11).
Die Kosten sind bounded O(N)
pro logicalTick; kein Anspruch O(64) für4096tatsächliche Countersteps.

**Readonly Preflight umfasst den zukünftigen Boundaryzustand.** Vor dem ersten
realen QueueDrain entsteht ein bounded scalar `OwnedCounterBoundaryPlan`:
keine Itemstack-/FCIS-/Nativebodyclones, keine Storagecalls, keine Livewriter-
capabilities. Seine Deskriptoren borgen originale opaque Handles/Pendingpostimages
und checked property scalar values. Der Plan ist maximal die vorhandene
Entityslotcap plus höchstens eine tatsächlich vorbereitete Entityspawnreservation
und bestehende bounded Inputcount groß; Spawncap wird vor jeder Erweiterung
geprüft. Entries werden fallible vor Drain reserviert. Planallocationfehler
verweigert den ganzen logicalTick intakt.

Planer durchläuft zunächst gegenwärtige registrations und dann die tatsächlich
admitted Completionqueue in exakt der späteren Completionprefixfolge. Für jede
Completion ruft er dieselben privaten lease-/query-/owner-/generation-/
receipt-/currentimage-/epoch-Prüfer **readonly** auf. Nur wirklich admissible
Confirmedcompletion legt einen virtuellen Publicationoverlay über seinen
originalen Pendingpostimage; Refused/Unresolved/ReceiptNone/foreign/stale ändern
keinen virtuellen Body, erhalten aber ihre deterministische Status-/Refusal-
Disposition. Ein privates consumedPending-Mark verhindert, dass dieselbe ID
im Shadowmodell zweimal publiziert. Das verleiht dem Plan keine neue
Completionauthority; die genuine move-owned Completion bleibt in der Inbox.

Spawnoverlay übernimmt genau das original PreparedOwnedEntity/afterrecord-
Countertripel und reservierte Slot-/Handleidentity. Fullremoveoverlay entfernt
den virtuellen Slot aus eligibility. Partial-/Saveoverlay übernimmt den
bestätigten Rest, wendet dessen **gebundenen** private Deferprojection gemäß
unserer Ordnung an und prüft die dafür erforderlichen lokalen dirty generation
Schritte. Der endgültige erste Counterstep des Rest-/Spawnslots gehört bereits
zu diesem readonly Plan. So darf weder auf dem alten Beforebody geprüft und
anschließend ein maxgeneration Rest verwendet werden, noch ein neu sichtbarer
Spawn außerhalb der Prüfmenge auftauchen. Definitive Refusals projizieren auf
den erhaltenen ursprünglichen Body; uncertain/completionfehlende Items bleiben
an die entsprechende private Projection gebunden. Ein bei Planbeginn
quarantiniertes Pending bindet keine Projection; sein Item ist nicht eligible
(§11).

Danach klassifiziert derselbe readonly Boundaryplanner admitted Actions in
FIFO. Item-restore zählt als potentiell neu sichtbarer Body und wird ebenfalls
in eligibility aufgenommen: native checked header/counterview aus dem sealed
Read, Scope/UUID/wire/resident/bodybudget/slotcap und neue lokale Generation
vor tatsächlicher Decode-/Publication. Ein nicht zulässiger Restore bleibt
Refusal und fügt keinen Body hinzu. Actionspawn/drop sind unbestätigte Work-
postimages, noch keine visible bodies und erhalten keinen erfundenen Spawnstep.
Pickup/Save setzt im Shadowmodell die echte zu erwartende Reservation und
Counterbasis/Defercap aus dem konkreten vorhandenen Plan; Count-/Sourceplanung
benutzt dieselben bounded borrowed Inventory-/Itemsplanner ohne Componentclone.
Ohne echte Completion erhält Fullpickup erst einen Pendingremove, keine
vorzeitige virtuelle Tombstonepublication. Neue due-/prioritymarks werden
scalar mit ihren tatsächlichen Count-/Generation-/claimcaps vorbereitet.

Zuletzt preflightet der Planner über **alle** so ermittelten eligibleBodies:
nextTick, residente Tickclaim, positive/genau-next lokale Dirty-/Projection-
generation, Defercount<=1024, erste Dueentscheidung/mark sowie dafür benötigte
Publication-/Defergenerationen und tatsächliche fixed Metadata-/Recordbudgets.
Die genuine State-/Storageevidenceprüfung und die scalar Transitionrechnung
teilen dieselben privaten checked Funktionen mit dem späteren Applypfad.
Der Plan hält deterministische input dispositions und Countertargets; keine
zweite abweichende Eligibilityberechnung nach dem QueueDrain. Der shard hat
keine concurrent writer, Plan und Anwendung sind synchron unmittelbar danach.

Jede entdeckte Counter-/Claim-/Generation-/Slots-/Budgetverletzung inklusive
neu in diesem Boundary bestätigt sichtbarer Spawn-/Restslots verweigert **vor**
Drain: Inbox, original Completion, pending lease, requestbefore/after,
Storestruktur, Properties und Tick bleiben intakt. Danach darf die Counter-
Applyphase keine neue recoverable Counterprüfung mehr entdecken; sie wendet
bereits checked scalar Transitions exakt an. Unmögliche private Stateabweichung
ist ein interner fail-stop, kein partieller erfolgreicher Tick. Ordinary
Actionrefusals bleiben dispositionierte intact Inputs wie `.1`; externe
Storageworkausführung findet erst nach dem ganzen erfolgreichen Boundary statt.

Meaningful neue Tests:4096resident Delay40items, Counterstepzahl nach40Ticks;
Saveauswahlsweep trotzdem höchstens64Slots/Boundary. Genuine Completionspawn
und Partialpickuprest genau im overflow-/Due-/Defergrenzboundary; inject private
near-max dirtygeneration nur in internen Tests. Erwartung bei +1: kein Drain,
keine Publication, kein Tickadvance, Completion/Lease intakt; completion-only
recovery bleibt verfügbar. Newly admitted restore und gemischte Completion+
ActionFIFO prüfen denselben zukünftigen Body-/Generationvertrag. Keine allein
selbstgespiegelten roundtriptests für diese State-/Queuegrenzen.

## Autorbesitz, Pins und Umsetzungstart

Root hat DUR-04.2a/SRV-02.2 auf `4f7d5037` nach vollständigem Entwurf77c7e70a
intern freigegeben. Ausschließlich `server/crates/ferrumc-sim/**` und dieser
neue Brief sind mutierbar; `.1`Brief/Review, Board, Storage/App/Items/Core/Cargo/
Lock/Git bleiben readonly. Keine `.2b`Physics/Activation/Merge/Eventfuture.

Vor Runtimecode: isolierter git-archive Checkout des realen4f7d503 erstellt,
`.codex-tmp/dur04_2-before.sha256` SHA
`709a45f91c88905ff99efc8739051f825930b6a202e56f8586412757febc77a3`,
Referenzmanifest `.codex-tmp/dur04_2-reference.sha256` SHA
`49a63d6218462e18758a2a3fa7dbf4b7648fe32c2d470f36e815e2c7a5f32431`,
22 Original-/Applied-/Patchfiles unverändert checksumverifiziert. Die in `.1`
vollständig gelesene Methodenkette und die nochmals gelesenen echten angewandten
ItemEntitytick-/inactive-/Despawnmethoden sind dieselben Pins, kein Interface-
oder Excerpt-only Ersatz. Neue Storage-no-op-Methoden sind explizit readonly
engine::next_stamp/Entityplan und raw::receipt_epochs. Sourceinspection und
strukturelle Tests sind weiterhin keine gestartete Paper-Differentialabnahme.

Der scalar Boundaryplan erhält positive eigene Count-/Bytecaps. Hardcount:
`MAX_ENTITIES_PER_SHARD + MAX_PLAYER_OWNERSHIP_INPUTS + 1` (65569), Hardbytes:
9MiB; Defaults genau diese Obergrenzen, Operator kann absenken. **Vor** fallible
Allocation prüft checked arithmetic `needed_entries * size_of::<Entry>()` und
jedes zusätzliche scalar Metadataelement gegen seine actual Rustshape. Ist die
tatsächliche maximale Shape größer9MiB, muss Autor Root melden, bevor ein Cap
angehoben wird. Scratch wird bei tatsächlichem Bedarf fallible erweitert und
später retained/reused; kein pauschaler65536entry-Neuaufbauallocation pro Tick.
Der Plan hält nur private scalar descriptors; Component-/Recordclones sind
in den bereits genehmigten echten Pending-/Admissioncharges separat begrenzt.


## Freigegebener konkreter Save-ID-Producer (Rootpräzisierung)

Automatische EntityCAS nutzt einen begrenzten move-owned Save-ID-Pool vom
Driver, keine Sim-RNG und keinen vermeintlich global eindeutigen Namespace.
Positive konfigurierbare Poolcap<=32 IDs, maximal512 raw IDbytes/Shard. Ein
opaker Ownerhandle bindet jeden Refillinput an echte Simincarnation; die Queue-
charge enthält alle16Bytes je ID und eigene begrenzte Metadaten. Lokale
Duplicates gegen Pool/aktuelle Pending verweigern den ganzen Input intakt.
Globale Wiederverwendung entscheidet die echte Storage-Receipt-ID/SHA-Prüfung.
Eine ID wird ausschließlich nach erfolgreicher konkreter Pendingadmission
verbraucht; Uncertainty behält dieselbe Query und erlaubt keinen ID-Replan.
Leerer Pool erzeugt höchstens einen `SaveIdsNeeded`-Intent/Boundary, belässt
Dirty/Priority und hält Counterticks nicht an. DUR-05 liefert vorher IDs aus
seiner vorhandenen UUIDauthority; Tests verwenden deterministische eindeutige
IDs. Poolallocation, checked lokale Generation und Auswahlcursortransition
werden vor Saveadmission geprüft. Confirmed Slotexpectations kommen aus der
bestehenden echten Storeauthority; es werden keine neuen Stamps erraten.


## 9. Konkrete Autorimplementation und enge Abnahmesemantik

Die Implementation besitzt echte opt-in `OwnedItemCounterLimits`, opaken
Owner-/Save-ID-Refillhandle, `run_tick_with_player_ownership_at(Tick)` und
`recover_player_ownership_completions()`. Jede zugelassene Boundary beginnt mit
read-only genuine Completion-/Futureactionpreflight, reserviert fallible
Slot-Vec-Kapazität vor Drain, publiziert Completion/Defer, führt Actions FIFO aus,
tickt alle berechtigten residenten Itemslots und scannt danach bounded Saves.
Die Kapazitätsreservation ist privat: keine ID, Registry oder Property entsteht.
Die Registry kann bei max lokalem Handlegenerationwert einen entfernten Slot
retiren; diese verringerte effektive Restorekapazität wird im Plan mitgezählt.

Erste Schrittzahl für genuine Rest/Spawn/Restore ist dieselbe zukünftige
Boundary. Der checked shardweite Tick und das gebundene Projectionfirst/last
verhindern Doubleclaims; es wurde kein redundanter frei schreibbarer
per-item Tickzähler eingeführt. Neue Bodypublication ist nur UUIDgebunden
aus dem genuine Originalplan. Missing body/durability/slot ist interner fail-stop;
ein bestätigtes Fullremove ruft keinen Rest-/Deferpublicationpfad auf.

FIFO-Playeradmission wird scalar aus dem zukünftigen Zustand modelliert:
accepted Rejoin invalidiert alten Handle, refused Rejoin tut das nicht. Native
FCPSvalidierung und deren Position stammen aus dem echten sealed Offtick-Read;
keine Playerstackclone entsteht in diesem Plan. Dirty nach Move/Mode/Select,
Completiondefergeneration nach tatsächlicher sortierter Playerrank, Bootstrap-
und Restoreressourcen sowie verbleibende Stopinputs werden exakt mitgezählt.

Root hat die engere terminale Ownerauthorityregel intern freigegeben:
`PlayerLocalGeneration::MAX` verweigert jeden folgenden logicalTick vor
Scratch/Drain/Counter/Cursor/Tickmutation auch bei clean/no-ID Zustand.
`MAX-1` bleibt zugelassen; die echte nächste Save-/Publicationauthority darf MAX
binden. Completion-only Recovery bleibt aufrufbar, preflightet aber tatsächliche
weitere Confirmation-/Defergenerationen. Ist Publikation unmöglich, bleibt
genuine Completion/Pending intakt mit typed GenerationExhausted; kein Saveack.
Diese Regel ersetzt einen selektiven zweiten Autosavecursor-Shadow.

CasConflict hält Quarantined Pending und sichtbaren Body strikt unverändert;
die private ursprüngliche Counterprojection wird am Ende dieses Boundaries
eingefroren und nicht weitergerechnet, die Reservation endet dort (§11).
Kein fresh Retryauthority/Refund entsteht. Busyrefusal behält ihren intact
Completioninput im Outcome und damit die genuine Lease: Retry im selben
Boundary ist Reserved. Erst Drop dieses refused Inputs ermöglicht den echten
exakten Retry. Receipt None bleibt Uncertain und ist niemals Autoabort.

Bestätigter Save setzt `saved_tick` auf die aktuelle Publicationboundary,
Completion-only auf den letzten bereits abgeschlossenen Tick. Unload-/Drain-
Priorität bleibt nach Confirmation erhalten, falls Defer erneut Dirty erzeugt.
Damit kann echter completion-only Flush quiescent werden; fortgesetzte residente
Counterticks dürfen weiterhin Dirty erzeugen und sind kein erfolgreicher Stop.

DUR-04.1 hatte bereits sämtliche Ordinary GameInputs am Enqueue typisiert
verweigert. `.2a` behält diese Semantik; owned Move/Mode/Select sind weiter echt
nutzbar. Reguläre Block-/Commandkonsumenten und Appintegration benötigen eigene
Sim-/DUR-05-Verträge. LegacyTick/Scheduler dürfen explicit Tickauthority nicht
umgehen. Dieses Ticket ist weder gesamte Survival-/Physics-/Paper-/Javaabnahme
noch produktiver Format6-Appbetrieb.

Die Autorproben enthalten 30 neue Countertests einschließlich 4096/40/64,
Tick-/Generationmax, tatsächlichem Busyretry, echter CasConflictquarantine,
accepted/refused Rejoin+Pickup, Move+Stop, Completionremove+Restore bei Slotcap1,
confirmed-byte-/savecharge-/plan-count-/plan-byte+1, privater Snapshotcorruption,
Unload-Deferpriorityflush, no-op/change auf Memory und erneut geöffnetem redb.
Sourceinspection und deterministische native Consumerproben sind keine live
Paperdifferentialtests. Freeze-/Checkmanifest wird separat an Root übergeben.

## 10. ITEM-SIZE-01-Adoption und getrennte Post-fixabnahme

Root hat ITEM-SIZE-01 unabhängig CLEAN geprüft und separat als
`b55513957d7277c1d79544b7ae8d7b4952d3e703` auf Main integriert; Review unter
`docs/agent-tasks/ITEM-SIZE-01_BORROWED_STACK_ACCOUNTING_REVIEW.md`.
Der Producerfreeze ist
`a2191dbc36cd92c2e6851d31e14d4311ec1f79456db8e6749bb439ff035a95b6`,
dessen Evidencefreeze
`6c11bdff0ef26a422a83a9c833607d85dbf2f0c787f83ac3ab1cd6fea66d9bcf`.
Die enge Consumerkorrektur ersetzt ausschließlich unnötige FCIS-Serialisierung
für Größe/Repräsentation: InputKind::SpawnItemcharge, InventoryPlan::new/pickup,
scalar Spawn-/Dropboundary sowie die beiden vor tatsächlicher Materialisierung
liegenden Spawn-/Drop-Pendingchargepreflights benutzen `persisted_stack_size`; der private
InventoryPlan::same-Empty-/ItemIDguard bleibt erhalten und ruft danach
`same_stack_representation` auf. Alle47 InventorySources wurden vorher durch
InventoryPlan::new vollständig borrowed-validiert; der Pickupsource wird vor
der ersten same-Anwendung genauso validiert. Itemcount beeinflusst weiterhin
keine Repräsentationsgleichheit; exakte ordered Komponenten/NBTfloatbits bleiben
erhalten. Ausgewählte Materialisierung und tatsächliche Persistenceencodes
bleiben unverändert. Keine Items-/App-/Storageänderung gehört zur Adoption.

Der vorangehende Simfreeze `308412a7...`, Archiv `eec267c0...`, Autorresults
`2429fca2...` und Rootassessment CHANGES_REQUIRED werden unverändert behalten.
Neue isolierte Consumerchecks verwenden den echten b5551395-Ausgangsstand
mit ausschließlich dem neuen48-Datei-Simoverlay. Neue Author-/Source-/Evidence-
Artefakte liegen getrennt unter `.codex-tmp/dur04_2-postfix-final/`; unabhängiges CLEAN
und Integration bleiben Rootauthority. Whole320 Debug/Release und strict
Alltargetclippy/Fmt/Rustdoc werden nach dieser tatsächlichen Codeänderung neu
ausgeführt. Die30 Meaningful Countertests bleiben erhalten; neue reine
Encoder-/Wrapper-Spiegeltests sind für diese semantikerhaltende Adoption unnötig.

Der endgültige Autorenlauf auf genau diesem Simquellstand ist abgeschlossen:
WholeSim Debug320 und Release320 bestanden, striktes Alltargetclippy,
Workspacefmt im isolierten Archiv und Simrustdoc mit `-D warnings` bestanden.
Alle48 Overlay-,28 integrierten Items-,22 Paperreferenz- und4 historischen
Pre-fixartefaktpins wurden vor/nach den Checks identisch geprüft. Der erste
Zwischenlauf unter `.codex-tmp/dur04_2-postfix/` hatte Debug320 bestanden und
wurde vor den zwei ergänzten reinen Pendingchargesizingstellen sauber beendet;
seine Records bleiben mit separatem SUPERSEDED-Sidecar unverändert erhalten.
Der endgültige Quellstand wurde danach vor Capture mit Simfmt formatiert und
bis zum Ende des vollständigen neuen Checklaufs unverändert gehalten.

`source.freeze.json` und `source.archive.tar.gz` frieren alle48 Simdateien und
diesen finalen Brief ein; `evidence.freeze.json` bindet tatsächliche Commands,
vollständige Logs, Authorresults, Capture-/Prerequisitepins und den engen
Pre-/Postfixdiff. Die SHA-Werte werden außerhalb dieses Dokuments an Root
übergeben. Status ist **AUTHOR_CHECKS_PASS; unabhängiges CLEAN nicht behauptet**.
Keine zusätzlichen Gameplay-/App-/Storage-/Paper-/RSS-/TPS-Zusagen folgen aus
dieser geprüften borrowed-Größen-/Repräsentationsadoption.

## 11. Nachtrag SIM-03.5 (Root): quarantinierte Item-Pendings sind abgeschirmt

Gilt nur im Countermodus (`item_counters.is_some()`), in jedem normalen und
completion-only Recovery-Boundary. Der DUR-04.1-Legacypfad ohne Counter bleibt
unverändert: Dort hält ein Quarantined Pending die ganze Storereservation bis
zum Neustart, und Reconcile bleibt zugelassen. Dafür angepasst sind §3
(Abweichung deklariert), §7 Tabelle (Private counter ledger entries), §7
„Strukturreservation“ (eine aktive Entityoperation; Conflict/Superseded), §7
„Duegrenze“ (Quarantäne ist keine Uncertainty), §7 „Savefortsetzung“ (aktives
Entitypending), §8 (Counterphase, Plan) und §9 (CasConflict). DUR-04.1 §3 und
§6 verweisen hierher.

1. **Quarantäne ist im Prozess terminal.** Ein Pending wird `Quarantined` durch
   `Refused(CasConflict)` (Storage verweigert vor jedem Commit; es existiert kein
   Receipt) oder durch einen echten Receipt, dessen Current-Read
   `confirm_current` verfehlt. Danach gibt es weder Veröffentlichung noch
   Freigabe: keine Completion (Status ≠ `InFlight`), kein Retry (Status ≠
   `Prepared`). `Reconcile` wird mit `Ineligible("quarantined pending")`
   verweigert, weil eine Receiptklärung einen CasConflict fälschlich als
   `Uncertain` einstufen und Plan und Anwendung trennen würde. Das gilt für jede
   Quarantäne im Countermodus, auch für reine Player-Pendings.
2. **Projection eingefroren.** Die private `DeferredOwnedCounters` eines
   quarantinierten Pendings behält ihren Wert vom Ende des quarantinierenden
   Boundaries. Ab dem folgenden Boundary bindet der Plan sie nicht mehr; sie
   zählt keine Claims und wird nie angewandt. Der quarantinierende Boundary zählt
   wie bisher höchstens einen Claim. Überschreitet dieser `defer`, verweigert der
   Normal-Boundary vor dem Drain, und der completion-only Recovery-Boundary
   wendet die Completion an.
3. **Item abgeschirmt.** Hält ein quarantiniertes Pending das Replace- oder
   Remove-Postimage eines vorhandenen Items, ist dieses Item ab dem folgenden
   Boundary nicht eligible: kein Counterstep und damit keine neue Duemarkierung,
   keine Saveauswahl (der Cursor läuft weiter, kein `ItemSaveBlocked`). Pickup und
   Save dieses Items werden vor jeder Kandidaten- oder Bodykopie mit
   `Ineligible("quarantined item")` verweigert. Das gilt für Pickup schon im
   quarantinierenden Boundary, weil Completions dort vor allen übrigen Inputs
   angewandt werden; eine Save verweigert dort noch die bestehende Reservation
   (`Reserved`). Eine
   bereits gesetzte Duemarkierung bleibt ohne neuen Delete. Sichtbarer Körper,
   Pending-Postimage und zuletzt bestätigte Recordauthority bleiben unverändert;
   es entsteht keine CAS gegen die überholte Erwartung. Kein Altern, kein Despawn
   und keine Aufnahme bis zum Neustart sind eine deklarierte Gameplayabweichung,
   keine stille Pause: Der Boundary meldet für das Item keinen Schritt, und der
   Status ist per Checkpoint abfragbar.
4. **Reservation endet.** Am Ende des Boundaries (normal oder completion-only),
   der das Pending quarantiniert hat, endet seine ganze Storereservation. Ab dem
   folgenden Boundary zählt es im Plan nicht als reservierende Entityoperation.
   „Höchstens eine Entityoperation pro Shard“ gilt für aktive, nicht
   quarantinierte Pendings. Ein quarantiniertes Spawn- oder Drop-Pending (kein
   vorhandener Körper) verliert ebenso die Reservation und schirmt nichts ab.
   Checkpoint, Stop-Prädikat und Release-Gates bleiben unverändert.
5. **Unverändert:** `Uncertain`, `NeedsRefresh`, `InFlight` und `Prepared` binden
   ihre Projection und halten ihre Reservation weiter. Dauerhafte Uncertainty
   führt nach `defer` Claims wie bisher zu `CounterRecoveryRequired` (§7). Diese
   Abschirmung wird nicht auf `Uncertain` ausgedehnt.
6. **Auflösung.** Ein quarantiniertes Pending endet nur mit dem Prozess. Der Shard
   wird mit Quarantäne nie quiescent, Stop endet `Unresolved`, der zuletzt
   bestätigte Durable-Stand bleibt maßgeblich, und ein Neustart lädt ihn über
   Restore. Unbestätigter Timer- und Transformfortschritt des abgeschirmten Items
   geht wie beim Crash verloren (§5). Eine In-Prozess-Übernahme des Durable-Stands
   ist nicht Teil dieses Nachtrags.
7. **Gebundene Ressourcen.** Jede Quarantäne behält bis zum Neustart ihren
   Pendingplatz (`limits.pending`, höchstens 32) und ihre Pendingladung
   `661 + 2*before + 5*after` in der gemeinsamen Ladung (`limits.pending_bytes`,
   höchstens 8 MiB). Eine maximale Entitysaveladung von 1,835,669 Bytes passt
   viermal hinein; die fünfte wird mit `Capacity("pending bytes")` verweigert.
   Je nach Art bleiben außerdem gebunden: der abgeschirmte Körper mit Slot,
   Live- und Confirmed-Bytes; bei Pickup und Drop der Spieler (Spielerplatz,
   Spielerbytes, Lifecycle-Zelle; Release und Rejoin verweigert); bei Pickup,
   Drop, Spawn und Despawn ein Publikationskredit und bei animiertem Take ein
   Lifecycle-Pin; bei einer Save oder einem Despawn die verbrauchte Save-ID.
   Erschöpfte Grenzen verweigern neue Pendings typisiert (`Capacity("pending
   count")`, `Capacity("pending bytes")`, `Capacity("player count")`,
   Publikations- oder Lifecyclekapazität). Eine Eskalationsschwelle legt DUR-05a
   fest.

Originalsemantik: Vanilla und Paper altern einen getickten Item-Körper je Tick
und entfernen ihn an der Schwelle; eine Persistenz-Projection kennen sie nicht
(Source-Review SIM-03.5 §3.3, kein Differenziallauf). Abnahme: SIM-03.5-Matrix
M1–M15 und die Reviewproben R1–R4 auf Memory und redb mit echter
Work/Completion.
