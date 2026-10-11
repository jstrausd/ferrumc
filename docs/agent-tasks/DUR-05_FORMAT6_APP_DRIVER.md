# DUR-05 — produktiver Format6-/Inventar-/Entitydriver

Status: vorbereitet, noch nicht vergeben. Appcrate ausschließlich
`server/app/**` plus dieser Brief. Vorausgesetzt sind geprüfte DUR-04.2a-
Counterauthority, NET-04.1/.1b, konkreter Itemmetadatabuilder/Sessiontracking
und die getrennte DUR-03.2-Bestandsmigration. Keine gleichzeitigen Appwriter.

## Aktuelle Lücken und Freigabeschnitt

Die heutige App verwendet Format4 und Connectioninventare. Geprüfte Player-
und Itemownership verwendet Format6, native Playerpayload FCPS3 und opaque
genuine Read-/Receipt-/Completionauthority. Zwei Inventarwriter würden
Mengenprüfung und Reservierungen umgehen; die Umschaltung muss deshalb vor
Spieleradmission vollständig erfolgen.

Vor Runtime konkretisiert der Autor Freshstart, unveränderte Bestände und
explizite Offline-Migration getrennt. Format4→5→6 nutzt die geprüften eigenen
Producer; FCPS1/2→3 braucht eine ausdrücklich lossless Apppayloadadmission.
Kein blinder Markerwechsel, automatische leere Ersatzwelt oder erfundener
UUID-/Generation-Scope. Memory und redb müssen denselben Ownershipvertrag
beachten. Unsupported oder widersprüchliche Daten verweigern vor Liveadmission.

Ownedmode verweigert seit DUR-04.1 gewöhnliches `SimShard::enqueue(GameInput)`.
DUR-04.2a schützt zusätzlich die checked Tickidentity gegen Legacytickpfade.
Owned Move/Mode/Select sind vorhanden; generische Block-/Commandmutationen
besitzen noch keinen entsprechenden autorisierten Consumer. **Vor vollständiger
Appumschaltung** ist diese Simabhängigkeit in eigene Tickets aufzuteilen und
zu implementieren. Blockplacement mit Inventarverbrauch verlangt zudem echte
gemeinsame Player-/Chunkdurability; vorhandener Player+EntityCAS belegt das
nicht. Diese tatsächliche Storageabhängigkeit ist in
[DUR-03.3](DUR-03_3_PLAYER_BLOCK_DURABILITY.md) vorbereitet, vor Runtime mit
eigener Grammatik-/APIabnahme. Legacyhandler dürfen die neue Ownerauthority
nicht umgehen. Sessionpublication benötigt zuerst den konkreten
[SIM-03.4-Producer](SIM-03_4_ITEM_PUBLICATION_FACTS.md).

## Konkrete Driververträge

App gibt jedem Shard denselben tatsächlichen logischen Tick. Completions und
Aktionsinputs behalten den geprüften Boundaryvertrag; Uncertainty/Busy/Fullqueue
verlieren weder Worklease noch Pendingquery. Storage läuft ausschließlich auf
seinem Worker, mit bounded Counts/Bytes und echten abgeschlossenen Antworten.
Recovery-only Boundaries zählen keinen Gameplaytick und erzeugen keinen Catchup.

Save-IDs kommen aus der tatsächlichen Driver-UUIDauthority. Move-owned Refills
beachten den konfigurierten max32-Pool, lokale Duplicateregeln und echte durable
Collisionprüfung. Eine leere IDquelle darf Dirty/Priority nicht quittieren.
Neue genuine Workrequests werden außerhalb des Simulationthreads ausgeführt;
Retries behalten ihre exakte Query und ihren gefrorenen Snapshot.

Connection-/Container-/Command-/Plugin-/Equipment-/Savehandler konsumieren
dieselbe echte Ownerauthority. Keine speculative Inventoryspiegelmutation oder
nachträgliche Zweitpublikation. Actorresync liest den aktuellen Ownerzustand;
bestätigter Takecount und wirklich eingelagerte Teilmenge bleiben verschieden.
Spawn/Metadata, Partialupdate und Take/Remove gehen durch das tatsächlich
geprüfte Viewer-/Prioritäts-/Backpressuretracking.

Entityload nutzt checked bounded UUID-/Chunkpages, die echte Generatoridentity
und bestätigte native Records; lokale Handles/WireIDs werden neu vergeben.
Chunkunload, Leave und Stop behalten dirty/reservierte Daten bis tatsächlicher
Bestätigung. Sauberer Stop ist von Crash/Lostreply zu unterscheiden. Aktivierung
mit neuen und migrierten Beständen ist jeweils eine eigene Abnahme.

## Meaningful Abnahme

Reale Actor-/Observerclients und echte Prozessneustarts: Einzel-/Stackdrop,
Delay40→0, konservierende Teil-/Vollaufnahme, zwei Spieler, volle Tasche,
Inventoryclicks, Movement/Mode/Selection, Viewerwechsel und Reconnect. Für
Scope- und Countergates zusätzlich deterministische Tests. ReceiptNone,
QueueFull/Busy, verlorene Antworten, Workerfehler, Disconnect, kontrollierter
Unload/Stop und echte Crashcutpoints behalten eindeutige Items/Authority.

Vor Umschaltung alle alten Appclientharnesses bestehen lassen; gewöhnliche
Block-/Command-/Containeraktionen müssen ihren neuen Consumer tatsächlich
erreichen. Debug/Release/strictchecks, Generatorcheck, Freeze, unabhängiger
Review und vollständige CI für den veröffentlichten Head. Ein erfolgreiches
Wiring ist noch kein vollständiger Itemtick: Physics/Activation/Merge und
Javaevents behalten ihre konkreten eigenen Abhängigkeiten. Survival S1–S6
wird ausschließlich durch die Gesamtabnahme geschlossen.
