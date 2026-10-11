# NET-04.2c — tatsächliche Collector-/Sessionbindung

Status: vorbereitet, noch nicht vergeben. Ein Autor erhält ausschließlich
`ferrumc-session`, diesen Brief und eigene Scratchevidenz. Zuerst konkreter
readonly Entwurf; Runtime folgt auf dessen Review. SIM-03.4 wird parallel
implementiert und benötigt vor einem Runtimeconsumer unabhängige Abnahme.
App, Sim, Storage, Items, Net, Proto und Rootboards bleiben readonly.

Die Originalmethode `LivingEntity.take` benutzt Collectortracking einschließlich
Collector selbst als Audience. Das Itemviewer-Set und PlayerId allein reichen
dafür nicht. Session besitzt heute den tatsächlichen Remoteplayer-Wirewert;
Sim liefert einen versiegelten `ItemCollectorBinding` mit echter Owner-Weak,
Admission und voller Generationbindung. Dessen genehmigter readonly Vergleich
`matches_owned_player` kann eine vorhandene echte Actoradmission vergleichen,
beweist allein aber weder aktuelle Session noch Connected-/Scopezustand.

Root hat zusätzlich eine konkrete Grenze gelesen: `router.rs` vergibt
Remoteplayer-WireIDs ab2 und erhöht derzeit wrapping. `app/src/world.rs`
verwendet `PLAYER_ENTITY_ID=1` im tatsächlichen JoinGame für jeden Client.
Der Collector hat daher bei sich selbst momentan einen anderen Wirewert als
bei Observern. Eine globale Takecollector-ID darf nicht an alle Viewer
weitergereicht werden. Die App-/JoinGamebindung bleibt getrennte Folgearbeit;
ein Sessionconstructor darf callerlieferbare Zahlen nicht als Socketreceipt
oder tatsächlichen JoinGameproof ausgeben.

Der Entwurf liest alle wirklichen Join-/Rejoin-/Move-/Deliveredbaseline-/
Leave-/Endpointrotationpfade sowie Originaltracker-/Take-/Removalcode und
Source-/Featurepatches der gepinnten Minecraft1.21.8/PaperBuild60-Referenz.
Er legt APIs, tatsächliche Router-/Sessionincarnation, volle World-/Dimension-/
Generator-/Owner-/Admissionbindung, positive Count-/Scratch-/Arbeitscaps und
jede Fehlerdisposition vor. Keine öffentlich konstruierbare Fakebinding,
Mutationshandle aus einem Fact, Weakadresse als Identität oder neue
Socket-/DB-/SimShardcapability.

Audience verwendet die wirklich admitted Collector-Trackerbaselines und den
Collector selbst, mit viewerspezifischem tatsächlichem Wirewert. Farviewer,
gleiche PlayerId in fremdem Owner, Endpointreplacement, abgewickelter oder
noch pending Disconnect, Rejoin, Dimensions-/Scopewechsel und veraltete
Admission brauchen explizite Zustandsfolgen. Ein alter Collector darf nicht
auf die neue Session desselben Spielers umgebunden werden. Fehlende aktuelle
Binding darf keinen erfundenen Take erzeugen; die ganze bestätigte Itemgruppe
und deren weitere Changed-/Removephasen bleiben im Producer erhalten, bis
der vollständige Consumervertrag ihre Aufnahme rechtfertigt.

Wire-Allocatorüberlauf wird vor PlayerJoin/Sessioninstallation überprüft;
keine wiederverwendete kollidierende ID nach wrapping. Die gemeinsame
Namespaceabstimmung mit zukünftigen Item-WireIDs ist eine ausdrücklich zu
benennende Grenze. Neue Refusals behalten die wirklichen Mapping-/Leave-
Retryverträge und unverändert die bestehende Mandatory-Lane.

Meaningful Tests benutzen echte bounded Routerkanäle, tatsächlich erzeugte
Sim-Admissions-/Collectorbindings und Decoderframes: Collector/Observer mit
verschiedenen Self-/Remotewerten, echte Trackingaudience, stale/foreign/
Rejoin-/Scopefälle, MAX-/N/N+1-Grenzen und voller/geschlossener Control-/
Outboundlane mit retained Leave. Ein eigener Calleradapter ersetzt keine
Original- oder Authorityabnahme. WholeSession Debug/Release und strictchecks,
exakter Freeze und unabhängiger Rootreview folgen vor Integration.

Dieses Ticket liefert die geprüfte Sessionbindung für NET-04.2b. Es aktiviert
keine Itemphysik, automatische Aufnahme, produktiven Format6driver oder
Javaevents und schließt keine Survivaletappe ab.
