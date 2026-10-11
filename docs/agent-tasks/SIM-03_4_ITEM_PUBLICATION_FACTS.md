# SIM-03.4 — bestätigte Itemfakten für Sessionviewer

Status: vorbereitet, noch nicht vergeben. Ein Autor besitzt ausschließlich
`server/crates/ferrumc-sim/**` und diesen Brief. Voraussetzung sind die
integrierten DUR-04.1/.2a-Ownershipconsumer. Items-/NBTmetadata ist ein
nachgelagerter Sessionconsumer; dieses Ticket importiert keine Net-/Prototypen.
Root integriert Git. Vor Runtime folgt ein konkreter API-/Cap-/Orderingentwurf.

## Tatsächlich gefundene Lücke

Root hat `OwnershipIntent`, `publish_confirmed`, `publish_entity_intent`,
`EntityStore::publish_owned`, Handle-/Snapshot- und Countercode gelesen.
Spawn/Changed/Restored enthalten lediglich einen lokalen EntityHandle;
Removal enthält UUID und Wire-ID. Das ergibt noch keine vollständige
immutable Publication für eine asynchron entkoppelte Session. Ein späterer
Handlelookup kann bereits eine neuere Änderung oder einen entfernten Slot
sehen. UUID/Wire-ID allein unterscheiden nicht Scope und Storeincarnation.

`EntityStore::snapshot_item` ist eine allgemeine readonly Datakopie und
serialisiert aktuell FCIS zur Validierung. Sie garantiert weder bestätigten
Publicationzeitpunkt noch dauerhafte Mutationsauthority. Session darf daraus
keinen bestätigten Spawn konstruieren. Counter-dirty live-after ist zudem
von confirmed-before und reserviertem Body getrennt; ihre Images dürfen nicht
vertauscht werden.

## Producervertrag vor Code

Der Autor konkretisiert einen engen move-owned, versiegelten Datentyp mit
readonly Gettern. Tatsächliche Shard-/World-/Dimension-/Generationidentity,
UUID, Wire-ID, Incarnation/Slotgeneration und nicht wrapping Publicationfolge
müssen gemeinsam gebunden sein. Session erhält immutable sichtbare Daten,
keine EntityStore-, Storage-, Tick- oder Mutationscapability. Ein kopierter
Shard und beliebige Callerfelder dürfen keinen neuen bestätigten Eventproducer
erzeugen. Readonly Kopien dürfen keine zweite Simulation aktivieren.

Spawn/Restore/Metadataänderung enthalten die tatsächlich publizierte vollständige
Itemrepräsentation, Position/Rotation und Motion. Removal bewahrt genau die
entfernte Instancebindung auch nach Slotfreigabe. Take/Partialrest/Remove binden
dieselbe wirkliche Mutation; Animationscount und eingelagerte Menge bleiben
unterschiedlich. Der heutige interne Intentvektor ist keine automatische
Wireorderingzusage: spätere Sessiongruppen müssen Take vor Remove bzw. die
wirkliche Partialrestmetadata in der konkret geprüften Reihenfolge senden.

Keine Pendingpostimages vor bestätigtem Commit veröffentlichen. Bei Lostreply,
ReceiptNone, Superseded und Retry entsteht weder Doppelspawn noch zweiter Take.
Restored muss eine echte Store-/Generation-/Nativeadmission besitzen. Der Autor
legt ausdrücklich fest, welches live-after bei Counteränderungen sichtbar ist;
reine Timerupdates erzeugen keine unnötige Metadata- oder Bodykopie je Tick.

Counts, retained Payloadbytes und Scratchwork erhalten positive explizite Caps
vor Allocation/Kopie. Eine gemeinsame readonly Itemgrößenprobe verwendet
ITEM-SIZE-01; keine Wegwerfserialisierung für Länge/Gleichheit. Keine Komponenten-
kopie je Viewer. Fallible Eventvorbereitung und volle Publicationqueues dürfen
eine bestätigte Mutation nicht vergessen: nachgelagerte Ausgabe muss vollständig
retrybar repräsentiert bleiben oder vor Mutation reserviert werden. Der genaue
Cutpoint-/Backpressurevertrag ist Bestandteil des Entwurfs. Alle alten Workleases,
Completions, Reservations, Counter-,Save-,Unload- und Stopverträge bleiben erhalten.

## Referenz und Abnahme

Minecraft1.21.8/772/4440, PaperBuild60/Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Vor Implementierung tatsächlichen
ServerEntity-/ChunkMap-Pairing-,dirty Metadata-,Motion-,ItemEntity.playerTouch-
und Removalcode samt Source-/Featurepatches und Callees lesen. Vorhandene
NET-04.1/.1b-Pins gezielt wiederverwenden; Datenaufnahme, Pluginereignisse und
Netpublikation als unterschiedliche Originalschritte dokumentieren.

Meaningful Tests benutzen wirkliche Memory-/redb-Ownershipreads und
Completionpfade: Spawn, Restore, Partialpickup, vollständige Removal, gleiche
Wire-ID in anderem Scope/Incarnation, Slotreuse, veraltete Publication,
SnapshotOnly, Lostreply/Retry/Superseded, N/N+1-Count-/Bytecaps und verweigerte
Ausgabe vor/nach bestätigter Mutation. Immutable alte Fakten bleiben nach
späterer Änderung/Removal exakt; Counterdirty-/Reservedimages bleiben getrennt.
WholeSim Debug/Release, strict Alltargetclippy/Fmt/Rustdoc und unabhängiger Review.

NET-04.2b konsumiert diesen Producer danach im Sessionlayer. DUR-05 aktiviert
den produktiven Driver mit echten Actor-/Observer-/Neustartprüfungen. Dieses
Ticket implementiert keine Itemphysik, Mergeentscheidung, automatische Aufnahme,
Javaevents oder vollständige Survivaletappe.
