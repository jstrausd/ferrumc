# NET-04.2b — besessenes Item-Viewertracking im Sessionlayer

Status: vorbereitet, noch nicht vergeben. Ein Autor besitzt ausschließlich
`server/crates/ferrumc-session/**` und diesen Brief. Vorausgesetzt sind die
geprüften NET-04.1/.1b-Pakete und der Items-only NET-04.2a-Metadatabuilder.
Sim-/App-/Storage-/Net-/Protokolländerungen und Rootboards sind readonly.

## Wirkliche Grenze und Publicationbedarf

Das vorhandene Routertracking betrifft Spieler, keine Item-Entities. Die
neuen Pakettypen allein erzeugen weder sichtbare Drops noch automatische
Aufnahme. Dieser Consumer erhält ausschließlich ausdrücklich veröffentlichte
Itemfakten über die Bridge. Er hält keinen SimShard, EntityStore, Chunk oder
DB-Handle und erzeugt keine dauerhafte Mutation oder Itemkopie im Inventar.

Die bestehenden OwnershipIntents enthalten teils lokale Handles, keine
vollständigen immutable Itempublicationfacts. Der Autor liest ihre tatsächlichen
Producer vor seinem APIentwurf. Fehlt ein bestätigter Snapshot-/Entfernungsproducer,
wird er zuerst als separates Simticket konkretisiert; ein Sessionconstructor
darf fehlende Scope-/Generation-/Publicationauthority nicht vortäuschen.
Insbesondere werden Pendingpostimages vor bestätigter Mutation nicht gespiegelt.

Referenz Minecraft1.21.8/772/4440, PaperBuild60/Commit
`29c8822d90899c89d2689338e81a98f690bcba12`: tatsächliche Entitytracker-/Pairing-,
dirty Metadata-, Motion-, ItemEntity.playerTouch- und Removalpfade einschließlich
Source-/Featurepatches und relevanter Konfiguration lesen. NET-04.1bs geprüfte
Quellenpins wiederverwenden; zusätzliche Consumerpfade gezielt ergänzen.

## Konkreter Vertrag vor Code

Autor legt APIs, Scope-/UUID-/Incarnation-/Wire-IDbindungen, positive Count-/
Byte-/Arbeitscaps und die vollständige Zustandsfolge vor. Welt und Dimension
bestimmen Sichtbarkeit; ein gleicher Wire-IDwert in einem anderen Scope oder
nach Reconnect ist keine gültige alte Entity. Unsupported oder veraltete
Publications erhalten explizite Dispositionen, keinen stillen Defaultspawn.

Pro Viewer wird eine Itembaseline erst nach tatsächlicher Layer-A-Admisson
publiziert. Spawn, vollständige Itemmetadata und nötige initiale Motion sowie
Take und Remove verwenden gemeinsame State/Mandatory-Lane. Ein Partialpickup
verwendet die wirklich veröffentlichte Restmetadata, der Takecount bleibt der
ursprüngliche Animationscount. Die wirklich eingelagerte Menge wird hier nicht
neu berechnet. Die Gruppe hat nur dann Atomicität, wenn diese ausdrücklich
implementiert und geprüft wird; NET-04.1b liefert FIFO bei Einzeladmission.

Full/Closed bei einem Mandatoryfolger schließt den langsamen Viewer über den
vorhandenen echten Lifecyclevertrag. Ein dabei verweigerter PlayerLeave bleibt
retrybar repräsentiert; kein Log-only-Verlust oder Phantomtracking. Writer-
Layer-B-Refusal wird später vom tatsächlichen Appcaller eskaliert. Erfolgreiches
Sessionenqueue ist keine Bestätigung eines bereits gesendeten Socketframes.

Ein- und Austritt durch Bewegung, Chunk-/Dimensionswechsel, Join, Reconnect,
Entfernung und Shutdown benötigen explizite Orderingregeln. Retained Tracking
und Metadatabytes einschließlich Multiplikation über Viewer bleiben bounded;
kein globaler Lock, unbounded Mapwachstum oder Componentclone je Viewer.
Ein neuer Dependencyedge zur Items-Crate benötigt eine konkrete Begründung
im Manifest; untergeordnete Crates importieren keine Session-/Nettypen.

## Meaningful Abnahme

Reale bounded Routerchannels und echte Writerframes prüfen: Actor/Observer,
Enter/Leave/Reenter, Reconnect, gleicher Wire-IDwert in verschiedenem Scope,
stale Generation, Spawn→Metadata→Motion, Partialrest und Take→Remove.
Paket-/Byte-/Trackingcaps N/N+1/0, volle/geschlossene Layer-A-Lane, verweigerter
Lifecycleleave und tatsächlicher Retry; kein Viewerghost nach verweigertem
Mandatoryfolger. Geprüfte Originalmetadata unverändert weiterverwenden.

WholeSession Debug/Release, strict Alltargetclippy/Fmt/Rustdoc, exakter Freeze
und unabhängiger Review. DUR-05 besitzt die produktive Driveradoption und
echte Client-/Neustartabnahme. Itemphysik, Merge, automatische Pickupentscheidung,
Mobmetadata, Javaevents und Survival S1–S6 bleiben eigene Consumer/Abnahmen.
