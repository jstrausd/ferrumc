# SIM-03.4 — bestätigte Itemfakten für Sessionviewer

Status: konkreter API-/Cap-/Orderingentwurf zur Rootprüfung; ausdrücklich noch
keine Simproduktion. Root wählte Richtung A: Publicationplatz und immutable
Bodycharge vor Workrequest reservieren. Dieser Entwurf ändert ausschließlich
diesen Brief und eigene Scratchevidenz; die 48 ganzen Simdateien bleiben
readonly. Nach separater Vertragsfreigabe besitzt ein Autor ausschließlich
`server/crates/ferrumc-sim/**` und diesen Brief. Voraussetzung sind die
integrierten DUR-04.1/.2a-Ownershipconsumer und ITEM-SIZE-01. Items-/NBTmetadata
bleibt ein nachgelagerter Sessionconsumer; keine Net-/Protoimports oder neue
Dependency. NET-04.2a ist unabhängig geprüft auf main als `67a9aa94` integriert.
Root integriert Git. Keine Runtime vor Review dieses konkreten Vertrags.

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

## Gewählte Queue- und Admissionform

Eine neue **opt-in** `ItemPublicationState` lebt in der tatsächlichen
`PlayerOwnershipState`; Standardzustand ist `None`. Einmaliges Initialisieren
ist nur beim echten, nicht draining/stopped/snapshot-only Ownershipowner mit
leerem EntityStore, ohne Input/Pending/Entityreservation erlaubt. Die wirkliche
Chunk-Generationbindung und Entityowner-Scope werden wie im heutigen
`ownership_state()` überprüft. Keine Disable-/Reset-/Reconfigure-API; damit
wird eine lokale Publicationfolge nicht im gleichen Owner wiederverwendet.

Initialisierung baut zunächst einen privaten `VecDeque<ItemPublicationGroup>`
und reserviert fallible genau die konfigurierte maximale Gruppenzahl. Erst nach
Erfolg wird der vollständige Zustand installiert. Tatsächlich gewährte
Allocatorcapacity darf größer sein; die logische Countpolicy bleibt unverändert.
Danach benötigt der Publicationpush keinen weiteren Queueallocate/Reserve.
Die vorhandene Ownershipowner-Allocation liefert die private Weakidentity für
Acktickets; kein neuer starker Store-/Socket-/Storagehandle wird gehalten.

`PendingPlayerMutation` bekommt opt-in ein privates
`Option<PreparedItemPublication>`. Der private Producer bereitet daraus vor dem
echten Workrequest genau eine mögliche Gruppe vor: Spawn/Drop, Pickuprest oder
Removal/Despawn. Autosave mit ausschließlich Counter-/Saveänderung reserviert
keine Gruppe. Die neue Admission benutzt borrowed `persisted_stack_size` und
gegebenenfalls `same_stack_representation` plus **gesonderten Countvergleich**;
kein FCIS-Wegwerfencode für Publicationgröße oder Gleichheit. Bestehende
Ownershiprecord-/PreparedEntityproduktion behält ihre eigene bisherige
Admissionalgorithmik und Budgets.

Vor jeder **zusätzlichen Publicationbodykopie** sind Modell, neue Count-/Byte-/
Preparecaps und Sequencecredit geprüft. Der Stack wird anschließend genau
einmal für diese reservierte Gruppe kopiert, ohne Normalisierung/Sortieren;
die Kopie liegt im privaten Pending und wird später in die Outbox verschoben.
Dieses Ticket darf die bereits bounded und vor Mutation geprüften vorhandenen
`ItemStack`-/NBT-/Component-Deepclones benutzen. Diese Standardclones und
bisherige Arc/BTreeMapallocationen sind infallible Allocatorgrenzen, keine
all-OOM-`Result`-Garantie. Neue Queueallocation/-capacity und checked Budget-
Refusals sind recoverable. Kein weiterer Items-Falliblecloneproducer ist
Voraussetzung für diesen ausdrücklich begrenzten Vertrag.

Alle falliblen neuen Proben/Kopien erfolgen vor `reserve_owned`, Pending-
installation, Player-/Entitymutation und Ausgabe des Workrequests. Der
vorbereitete Body ist zunächst staged; Budgetcounter werden erst gemeinsam mit
der tatsächlichen Pendinginstallation übernommen, nachdem alle recoverable
Admissionprüfungen bestanden sind. Eine vorherige Refusal verwirft nur staged
private Daten; kein reservierter Platz/Bytecredit bleibt zurück. Kein neuer
fallibler Publicationschritt nach Installation oder bestätigter Mutation.

## Konkrete öffentliche Datentypen und APIs

Der folgende APIentwurf ist ein Vertrag, keine bereits implementierte API:

```rust
pub struct ItemPublicationLimits { /* private positive limits */ }
impl ItemPublicationLimits {
    pub fn new(
        groups: NonZeroUsize,
        retained_bytes: NonZeroUsize,
        prepare_bytes: NonZeroUsize,
    ) -> Result<Self, ItemPublicationError>;
}

impl SimShard {
    pub fn initialize_item_publications(
        &mut self,
        limits: ItemPublicationLimits,
    ) -> Result<(), PlayerOwnershipError>;

    pub fn item_publication_status(
        &self,
    ) -> Result<ItemPublicationStatus, PlayerOwnershipError>;

    pub fn peek_item_publication(
        &self,
    ) -> Result<Option<&ItemPublicationGroup>, PlayerOwnershipError>;

    pub fn acknowledge_item_publication(
        &mut self,
        ticket: &ItemPublicationTicket,
    ) -> Result<(), PlayerOwnershipError>;
}

pub struct ItemPublicationGroup { /* sealed immutable fields; no Clone */ }
impl ItemPublicationGroup {
    pub fn instance(&self) -> &ItemInstanceBinding;
    pub fn sequence(&self) -> ItemPublicationSequence;
    pub fn origin(&self) -> &ItemPublicationOrigin;
    pub fn kind(&self) -> ItemPublicationKind;
    pub fn postimage(&self) -> Option<&ItemPublicationPostimage>;
    pub fn take(&self) -> Option<&ItemTakeFact>;
    pub fn ticket(&self) -> ItemPublicationTicket;
    pub fn encoded_charge(&self) -> usize;
}
```

`ItemPublicationKind` ist ein readonly Copyenum Spawn/Restore/Changed/Removed.
Alle übrigen genannten öffentlichen Datentypen besitzen private Felder und
Rustdocgetters. Group/Postimage/Origin/Take haben keine öffentliche Konstruktion,
`Clone`, mutable Bytes/Stackaccessor, `from_snapshot`, Arbitraryparts- oder
Deserializationfactory. Group enthält keine Packetbytes oder Layercallbacks.
Postimage borrowt den ganzen unveränderten ItemStack und liefert tatsächlichen
Transform und sealed `ItemEntityProperties` inklusive Motion/Scalars.

`ItemInstanceBinding` hält intern das **wirkliche** EntityHandle/Weaktoken,
ShardId und volle `GenerationBinding`, UUID, Wire-ID und registrierten Entitytyp.
Sie ist cheap Clone/Eq ohne Bodykopie. Getter liefern Scope/Generation und
scalar IDs/ursprüngliche Slotgeneration; das mutatorfähige EntityHandle oder
dessen Slotindex werden nicht herausgegeben. Eq umfasst Owner-Scope, komplette
Generationidentity, UUID/Wire/Typ und Handle-Eq mit tatsächlicher Store-
allocation und Slotgeneration. Identische UUID/Wire allein sind keine Instance.
Weakidentity wird nie in eine Adresse/Nummer umgewandelt oder serialisiert.

`ItemPublicationSequence` ist private `NonZeroU64` mit readonly `get`; sie ist
keine Storage-/Player-/Counterrevision. `ItemPublicationOrigin` unterscheidet
intern ConfirmedReceipt und NativeRestore. Getters liefern Herkunftskind,
optional echte Receiptquery, vorhandenen bestätigten Ownershipstamp und
Generationdaten, keine Storagefactory oder CAS-/Write-/Tickcapability. Die
echte Storebinding bleibt privat geprüft und versiegelt. Restore erzeugt
ausdrücklich keine Receiptquery aus seiner UUID oder lokalen Reservation-ID.

`ItemTakeFact` besitzt die echte Iteminstance derselben Gruppe, Original-
Animationscount, tatsächlich moved-Menge und einen versiegelten
`ItemCollectorBinding`. Letzterer wird vor Workrequest aus dem tatsächlich
validierten OwnedPlayerHandle übernommen: PlayerId, tatsächliche Admission und
private Ownershipowner-Weakidentity, gemeinsam mit der echten Scopebindung.
Keine callerlieferbare Collectorwire-ID, kein UUID-/Wire-IDcast. Getters
liefern PlayerId/Admission und readonly Scope; keine Player-Mutationsauthority.

`ItemPublicationTicket` ist cheap Clone, private Felder/Constructor, und wird
nur aus einer existierenden versiegelten Gruppe gemintet. Es bindet tatsächliche
Ownershipowner-Weakidentity, volle Scope-/Generation-/Iteminstance und deren
Publicationsequence; es besitzt keinen Stack. `ItemPublicationStatus` hat
readonly getters für retained/reserved Gruppen und Charges, Preparationlimit
und zuletzt publizierte Folge. Deskriptive Statusdaten erzeugen keine Facts.

`ItemPublicationError` ist non-exhaustive mit klassifizierten InvalidLimits-
Budgetangaben, NotInitialized/AlreadyInitialized/NotEmpty, Capacity
`{ budget, used, requested, max }`, SizeOverflow, SequenceExhausted,
AllocationFailed mit tatsächlichem `TryReserveError`, Model mit bestehendem
`ItemPersistenceError`, AckWithoutHead und AckMismatch. Scope-/Snapshot-/
Drainingerrors bleiben vorhandene `PlayerOwnershipError`-Kategorien; ein
neuer transparenter Publicationvariant bindet eigene Fehler ohne Stringerror
oder rekursiven Errorgraph. Keine stillschweigende leere Ersatzgruppe.

## Positive Caps, Charge und Sequencereservation

Vorgeschlagene lokale Hardmaxima für Rootreview: **256 Gruppen**, **8 MiB**
retained encoded charge, **1 MiB** Preparecharge. Constructorargumente sind
NonZero und dürfen die Maxima nicht übersteigen; kleine Werte bleiben für
Grenztests konfigurierbar. Kein unbounded/default/automatisches Hochsetzen.
Die Queue-Countpolicy gilt gemeinsam für Pendingreservierungen und Outbox:

```text
reserved_groups + retained_groups <= groups_limit
reserved_charge + retained_charge <= retained_bytes_limit
one_staged_publication_charge <= prepare_bytes_limit
```

Lokale Charge einer Gruppe: fixer **256-Byte-Scalaraccountinganteil** plus exakt
borrowed FCISstackgröße bei Spawn/Restore/Changed; Removal einschließlich
optional Take hat nur diesen fixen Anteil. Scalaraccounting begrenzt logisch
Instance/Origin/Collector/Properties/Sequenceheader; es ist ausdrücklich kein
neues serialisiertes Wireformat und keine Behauptung über `size_of` oder
Allocatoroverhead. Keine separate zweite Bodykopie für Take oder jeden Viewer.
FCIE-/Owner-/Pending-/Input-/Counter-/Netbudgets werden damit nicht ersetzt.
Encoded Charges sind kein decoded Heap-/RSSbudget; vorhandene Modellentry-,
Byte-/Sequence-/Depthcaps und Gruppencap begrenzen die repräsentierbaren Daten.

Alle Additionen/Konvertierungen und available-space-Rechnungen sind checked.
Private Ledger enthält `last_published: u64` initial0 und
`reserved_sequence_credits`. Vor Workrequest oder Restoreadmission gilt:

```text
last_published + reserved_sequence_credits + 1 <= u64::MAX
```

Der tatsächliche neue NonZero-Sequencewert entsteht **erst** bei erfolgreichem
Confirm/Restorepublication in dessen wirklicher Reihenfolge. Dann wird ein
Credit verbraucht und last_published genau einmal erhöht. Dies erhält
Confirmreihenfolge bei umgekehrter Completionreihenfolge und verhindert Lücken
durch abgebrochene Reservations. Retry/ReceiptNone/Superseded/Busy vergeben
keine Folgen. Eine noch unbestätigte Reservedbodykopie hat keine öffentliche
Sequence/Ticket/Publicationauthority.

Publication kann niemals an späterem Sequenceoverflow scheitern: alle
potenziellen Pendinggruppen besitzen zuvor gleichzeitig geprüfte Credits.
Materialisierung verschiebt reserved in retained, ohne Gesamtcount/-charge zu
erhöhen. Bereits retained Gruppen zählen nicht erneut als Sequencecredits;
last_published enthält deren Werte bereits. Folge MAX darf einmal publiziert
und bestätigt werden; danach verweigert jede neue sichtbare Admission vor
Workrequest. Timer-only Autosaves benötigen weiterhin keinen Credit.

## Wirklicher Cutpoint und die immutable Daten

Confirmedcompletion behält die gesamte heutige Proofkette: actual Owner/Store,
Ticketgeneration und Workbuffer-Weak, echte Receipt/current Reads, Scope/
Generation/Recordgleichheit, aktuelle Resourceclaims und Counter-/Deferred-
preflight. Der neue Pendingpublicationbody existiert schon vorher und ist
modell-/capgeprüft. Vor `remove_pending` wird zusätzlich readonly die Integrität
seines tatsächlichen Reservations-/Sequencecredits überprüft; keine neue
Allocation, Deepclone oder Eventprepare im Completionpfad.

Für Spawn/Changed: heutige gemeinsame Player-/Entitymutation und
`publish_counter_confirmation` laufen in ihrer bestehenden Reihenfolge. **Direkt
nach** `publish_counter_confirmation`, noch vor `publish_entity_intent`, Stats,
Takelegacyintent, `apply_deferred` und dem späteren `apply_counter_boundary`,
wird die vorbereitete Publication versiegelt und vollständig in die Outbox
verschoben. Nur tatsächliche scalar Werte werden an diesem Cutpoint aus dem
soeben publizierten Storeeintrag übernommen: wirklicher Handle/Slotgeneration,
Transform, Motion und aktuelle Properties/Counters. Der Stack stammt aus der
vorab unveränderten Kopie genau des reservierten PreparedOwnedEntitypostimages;
Counterconfirmation mutiert nur Scalars, nicht dessen Stack. Kein späterer
Lookup und kein bestätigtes FCIE-beforeimage wird als live-after ausgegeben.

Dieser Seal-/Pushschritt muss privat infallible Moves/Scalarcopies verwenden;
das aktuelle Entitypublicationresult und der vorher geprüfte direkte Slotzugriff
liefern seine tatsächliche Instance. Kein neuer fallibler öffentlicher
`snapshot_item`-Call nach Mutation. Unerreichbare interne Korruption wird nicht
als recoverable Outputrefusal mit verlorener Mutation behandelt.

Für Removal: volle wirkliche Instance einschließlich ursprünglichem Handle/
Slotgeneration wird bereits unter der tatsächlichen Reservation **vor**
`remove_slot` captured. Preparedpublication bewahrt sie auch nach Slotfreigabe;
das tatsächliche Removed-EntityData muss dazu passen. Keine Rückermittlung über
UUID/Wire in einem inzwischen wiederbelegten Slot. Take und Removal gehören
zur einen bestätigten Mutation/Gruppe. Nach Counter-/Entitypublication erfolgt
derselbe creditgedeckte Sealschritt ohne Bodykopie.

Restore: echte `OwnedEntityRead`-/Store-/Generation-/native Admission, bestehender
FCIEdecode und gesamte Recordgleichheit bleiben erforderlich. Nach readonly
Preflight werden Gruppe/Bodycharge/Seqcredit und zusätzliche Bodykopie vor
`reserve_owned`/`publish_owned` vorbereitet. Seal liegt nach tatsächlichem Spawn
und gegebenenfalls `install_confirmed`, vor ItemRestoredlegacyintent. Die
Herkunft ist NativeRestore mit tatsächlicher Ownershipstamp-/Generationevidenz,
kein erfundener Commitreceipt. Refusal erhält den wirklichen Readinput.

Ein späterer Counterboundaryschritt kann Scalars derselben Liveiteminstance
erneut verändern. Alte immutable Gruppen behalten bewusst exakt ihren obigen
Cutpoint; sie werden weder nachträglich aktualisiert noch in den nächsten
Zustand umgedeutet. Ausschließlich Timer-/Dirty-/Autosavegenerationänderungen
erzeugen keine neue Gruppe, keinen Metadatafact und keine zusätzliche
Publicationstackkopie. Vorhandene Autosavekopien bleiben separate alte Kosten.

## FIFO peek/ack, Ausgabeauthority und vollständige Retryordnung

Die Produceroutbox ist retained FIFO. `peek_item_publication` borrowt nur ihren
Head; sie drainet keine Daten und reserviert keinen neuen Platz. Driver arbeitet
außerhalb des Simticks: borrowte ganze Gruppe an den Sessionconsumer geben,
dessen gesamte Aufnahme/Outputadmission durchführen, erst danach das gemintete
Ticket an `acknowledge_item_publication` übergeben. Kein Caller-Socketcallback
im Producer, kein `take_intents`/`mem::take` als Publicationtransport.

**Ackauthority** ist der tatsächliche mutable SimShard plus dessen gemintetes
Ticket. Ack überprüft active echte Ownershipauthority/Generationbindung, die
private Owner-Weakidentity und den exakt aktuellen FIFOhead mit voller
Instance/Sequence. Foreign/SnapshotOnly/stale/reordered/doppelte Tickets
verweigern, ohne Head oder Buchführung zu verändern. Ein vorhandenes Ticket
überlebt eine Ackrefusal; die API borrowt es. Gleiche Headernummern aus einem
anderen Store, Scope oder Owner genügen nicht.

Diese API kann selbst keinen Socketversand beweisen: der autorisierte Driver
muss das Ticket erst nach **vollständiger** Sessionannahme bestätigen. Ein
gemintetes Ticket ist ein Besitz-/FIFOproof, kein Netzwerkreceipt. Ack entfernt
genau eine ganze Gruppe, gibt deren retained Count-/Bytecharge frei und droppt
deren Body; last_published wird nicht zurückgesetzt. Partielle Viewerannahme
rechtfertigt kein Ack. Session muss vollständige gruppierte Admission oder
expliziten idempotenten Fortschritt pro tatsächlichem Viewer/Gruppe liefern;
diese Consumerimplementierung gehört NET-04.2b. Verweigerte Ausgabe erhält
derselbe Body/Sequence/Charge und FIFOposition für nächsten Versuch.

Keine Komponenten-/Bodydeepclone pro Viewer in Sim. Metadataencoding kann aus
dem borrowten Stack durch NET-04.2a erfolgen; Sim garantiert keinen erfolgreichen
Metadataencode für alle FCISmodelle (z.B. PresentAIR oder trusted Opaqueinner-
schema). Solche Consumerrefusals dürfen die Quelle weder normalisieren noch
stillschweigend acknowledge/discarden. Source-/Clientschemaadmission bleibt
gesonderte Folgearbeit.

## Platz-/Bytefreigabe in jeder tatsächlichen Verzweigung

| Tatsächlicher Pfad | Preparedpub/Count/Bytes/Seqcredit | Öffentliche Gruppe |
|---|---|---|
| Admissionrefusal vor Pendinginstallation | Staged Daten verworfen; kein Ledgercredit übernommen | Keine |
| Echte Confirm/current-Abnahme mit sichtbarer Mutation | Ein Credit von reserved nach retained; Gesamtcharge/-count identisch; eine neue Sequence | Genau eine ganze Gruppe |
| Counter-only/No-opautosaveconfirm | Kein Publicationcredit reserviert oder vergeben | Keine |
| Busy/HistoryCapacity -> Prepared | Derselbe ganze Credit/Body retained | Keine |
| Lostreply/Unresolved/ReceiptAbsent -> Uncertain | Derselbe ganze Credit/Body retained | Keine |
| Confirmed + current Readerror -> NeedsRefresh | Derselbe ganze Credit/Body retained | Keine |
| Superseded/Mismatch/CAS-Counterconflict -> Quarantined | Retained; fehlende Authority ist keine Cancelberechtigung | Keine |
| Retry/Reconcile/erneuter Workbuffer | Vorhandenes Preparedpub wiederverwenden; kein zweiter Platz/Body/Seqcredit | Erst tatsächlicher einmaliger Confirm |
| Stale/doppelte Completion nach Confirm | Bestehende Proofprüfung verweigert; keine neue Buchführung | Alte Gruppe unverändert |
| Definitive bisherige Abort-/Refusedfreigabe über release_pending | Genau einmal reserved Count/Bytes/Seqcredit freigeben, Preparedpub droppen; alte Spawnrefusal-/Claimreleasepfade bleiben | Keine |
| Restore erfolgreich | Vorbereiteten Credit direkt nach retained verschieben; eine neue Sequence | Eine NativeRestoregruppe |
| Restore-/Resource-/Recordrefusal | Nur staged Daten droppen; Read/Registrierungen unverändert | Keine |
| Playerdisconnect oder Rejoinversuch | Kein Cancel/Drop/Freigabe des echten Pending/Outboxbody | Früheres/folgendes echtes Confirm bleibt erhalten |
| Letzter Chunckticketrelease/Unload | Registrierungen/Pending bleiben heute erhalten; Credits/Outbox unverändert, Counterflushpriority bleibt | Kein erfundener InstanceRemove |
| Drain | Neue Gameplayadmission stoppt; Clarification/Save/Outputack bleiben möglich; vorhandene Credits retained | Bestehende/echte Confirmgruppen liefern |
| Verweigerte/partielle Sessionausgabe oder Ackrefusal | Vollständig retained, kein Byte-/Countrelease | Derselbe FIFOhead retrybar |
| Gültiger Ack des vollständig angenommenen Heads | Genau dessen retained Count/Bytes freigeben; keine Seqwiederverwendung | Head erst dann entfernt |
| Terminal Stop/Schedulerdrain | Nur bei alter Quiescence **und** reserved=0/retained=0 | Keine unbestätigte Ausgabe vergessen |

Ein bloß fehlender Receipt, disconnect, unloaded Chunk, DeadWeak aus einer
externen Session oder anderer Shutdownwunsch ist keine neue Abortauthority.
Wirkliche Store-/Generation-/Incarnationersetzung ist im active Ownershipmode
bereits durch die vorhandenen mutatorfreien APIs verweigert. Kein automatisches
Clear/detach der Publicationledger; eine neue Prozessinstanz rekonstruiert nur
aus echten gespeicherten Reads/Restore, ohne alte Netzwerksequence zu fälschen.

Counter-save Auswahl behält bei neuer Publicationcapacityrefusal für Despawn
das echte Save-ID-/Dirty-/Prioritymaterial: `select_counter_save` entfernt heute
die ID erst nach erfolgreichem `install_candidate`; neuer Error folgt demselben
`ItemSaveBlocked`-Pfad. Nicht sichtbare Autosaves beanspruchen keine Outbox.
Terminal Stop und `is_tick_quiescent` werden im opt-in Mode um leere Publication-
ledger ergänzt; nicht opt-in Verhalten bleibt dasselbe.

## Anschluss an den bestehenden Counter-/Completionplan

Direction A beseitigt post-Commit Queuefull: jede mögliche bestätigte sichtbare
Mutation hat schon einen Platz/Bodycharge/Seqcredit. Der vorhandene readonly
Counterboundaryplan muss trotzdem neue **Admissionrefusals** und Stopquiescence
genau berücksichtigen. Eine zweite Body-/NBT-/FCISauthority ist nicht erlaubt.

Ein privates readonly `PublicationAdmissionPreview` liefert aus den vorhandenen
borrowten Candidate-/Shadowinputs nur Kind, wirkliche prospective Bindung,
exakte Stackcharge und vorhandene Take-/Removalform. Derselbe Preview-/Budget-
predicate wird von tatsächlicher Pendingadmission und Countershadowplanung
benutzt. Der Shadowplan führt zusätzliche **scalare** reserved/retained/count/
charge/Sequencecredits durch die gleiche echte Completion- und Actionprefix-
reihenfolge: Confirm überträgt, definitive Freigabe entfernt, Uncertain/Busy/
Quarantine behält, Restore nimmt Platz auf, neue Actions reservieren. Keine
Bodykopie, Queueoperation oder Mutation in diesem Shadowledger.

Insbesondere Stop nach einer Confirmcompletion bleibt blocked, solange deren
Gruppe retained ist; Stop nach tatsächlicher definitiver Abortfreigabe darf bei
sonst leerer ledger dieselbe bisherige Grenze erreichen. Kein pauschaler Check
nur des alten Outboxzustands und kein konservativ anders tickender Stopplan.
Queuefull für neue Action ist genau dieselbe Refusal im Shadowplan und im
tatsächlichen Actionconsumer; alle bisherigen Counterclaims, completion-only
Recovery ohne Tick, Reservation-/Counterprojektionen, Fairness und Saveordnung
bleiben erhalten. Die bestehende komplette Counterpreflight läuft weiterhin
vor Inputdrain/Counterclaim. Kein fallibles Eventbuild in `apply_owned_input`
nachdem echte Confirmationmutation schon stattgefunden hat.

## Originalordnung und getrennte Collectorbindung

Die vollständigen ausgewählten tatsächlichen Vanilla-/Applied-Papermethoden
und Callees samt relevanten Source-/Featurepatches sind im unveränderten
Zwischen-Sourcepinrecord festgehalten: 21 Source-/Patchpins. Keine neue
identische Exportserie oder nicht belegte Gameplaydifferentialabnahme.

ServerEntity-Pairing gibt AddEntity vor tatsächlichen nichtdefault Metadata aus;
Motion ist bereits im AddEntitybody, zusätzliche Motionupdates brauchen ihre
eigene Originalbedingung. Dirtyaufnahme, immutable serializer.copy und späterer
Trackerpacketversand bleiben getrennte Schritte. Neue Gruppen enthalten die
Datenaufnahme, keine Behauptung einer schon gesendeten Packetreihenfolge.

Original `ItemEntity.playerTouch` übernimmt echte Inventoryänderung, sends
normalen Take mit **Originalcount**, dann bei empty discard/PICKUP; Remaining-
Metadata kommt später aus dirty Tracking. Die logische Gruppenordnung ist
daher optional Take **vor** Changedrest bzw. Removed derselben Mutation. Ohne
Animation gibt es nur Changed/Removed. Spawn/Restore besitzen keine Takephase.
Der heutige Legacyintentvektor (Itemintent,Stat,Take) bleibt unverändert und ist
keine neue Wireorder; im opt-in Consumer wird allein die sealed Gruppe benutzt.
Animationscount und conserving moved bleiben ausdrücklich getrennt.

Applied Paper kann auf Eventcancellation mit flyAtPlayer einen Take **ohne**
Inventorymutation schicken. Solche Javaevents/Animation-only Facts werden
hier nicht erfunden oder als fertig behauptet; das braucht eigene Eventarbeit.
InstanceRemove folgt echter Removal-/Trackingendkette; Viewer-Unpairing allein
ist kein Scope-/InstanceRemovefact.

`LivingEntity.take` verwendet den **COLLECTOR** als broadcastAndSend-Quelle.
Takeaudience muss deshalb aus dem tatsächlichen Collectortracking einschließlich
Collector selbst bestimmt werden, nicht aus Itemviewers. Die echte Collector-
Wire-ID besitzt heute ausschließlich SessionRouter. Ein separat durch Root zu
vergebendes **Session-Collectorbindungsticket** muss echte PlayerId/Admission/
Owner-/Scopebindung auf die tatsächliche aktuelle Session-/Collectorwire und
deren Audience abbilden, stale/disconnected/Rejoinfälle und allocator overflow
prüfen. Sim liefert hier keine fake ID und aktiviert keinen Takesocketconsumer.
NET-04.2b muss diese reale Bindung vor Takeausgabe konsumieren.

## Konkrete Implementationsdateien nach Vertragsfreigabe

Nur Sim: neuer `src/item_publication.rs` für sealed Daten/Limits/Errors/ledger,
gegebenenfalls enger private Counter-/Admissionhelper in eigener Subdatei;
Reexports und README/INVARIANTS; private Pendingstate-/snapshot fields in
`player/pending.rs`; enge Admission-/Restore-/Confirm-/Releaseintegration in
`shard/player_ownership.rs`; scalare Shadow-/Stop-/Despawnadmission in dessen
`counters.rs`; echter Instance-/Scalarzugriff in `entity/ownership.rs` oder
`entity.rs`, ohne zweite Entitydatenhaltung. Sinnvolle echte Memory-/redbtests
in Sim. Keine Änderung an Items-/NBT-/FCIS-/Storageauthority/Dependencies,
kein Appdriver, Sessionviewertracking, Socketaudience, Physics/Merge/Pickup-
eligibility oder neue Javaeventimplementierung in diesem Ticket.

`PlayerOwnershipState::clone` hält wie bisher keinerlei Runnableinput,
Worktickets, Pendingreservierungen oder Outbox; snapshot_only und frische
Owneridentity bleiben. Eine readonly EntityStorekopie hat weiter frische
Incarnation. Snapshotfactories können den Producer weder initialisieren noch
acknowledge oder versiegelte Gruppen erzeugen. Checkpointcodec/-version bleibt
unverändert: Publicationcredits/Body sind ephemeral Runtimeownership, kein
durables Socketoutboxformat. Echte Checkpointclarification liefert vorhandene
Reads; erneuter Prozess admitted diese über NativeRestore.

## Meaningful Abnahme nach separater Runtimefreigabe

- Echte Memory- und redb-Reads/CAS-/Completion-/Restorepfade erzeugen die ganze
  Spawn-/Drop-/Partialpickup-/Fullpickup-/Despawngruppe erst bei Bestätigung.
  NativeRestore hat native Stamp-/Generationevidenz und keinen FakeReceipt.
- Exakte Originalcount/moved und tatsächliche partial Stack-/Scalarpostimages;
  Grouporder Take->Changed/Removed und originale Komponenten-/NBTrepräsentation
  bleiben ungewandelt. Alte immutable Gruppen nach späterer Mutation/Remove
  sind byte-/representationgleich, einschließlich floatbits und Patchorder.
- Reserved FCIEcounterbefore und wirklicher Liveafter am definierten Seal-
  cutpoint unterscheiden; späterer Counterstep ändert alte Gruppen nicht.
  Timer-only/No-opautosaves erzeugen null zusätzliche Gruppe/Bodykopie.
- Gleiche UUID/Wire in anderem World/Dimension/Generation/Storeincarnation,
  originale Slotreuse/Maxgenerationretire und SnapshotOnly verweigern Aliasing;
  entfernte Instance bleibt auch nach neuem Slotinhalt eindeutig.
- Lostreply, echte ReceiptAbsent, Busy, NeedsRefresh, Superseded/Quarantine,
  Retry/Reconcile, stale/doppelte Completion und umgekehrte Confirmationorder:
  genau einmal Group/Seq, unveränderte komplette Credits bis echter Freigabe.
- Positive Config-/Group-/retained-/Preparegrenzen N-1/N/N+1; alle neuen Caps
  vor zusätzlicher Bodycopy/Workrequest. Queuefull vor Admission verweigert
  unverändert; bereits reservierter Confirm succeeds trotz inzwischen vollem
  Outboxanteil. Abort/Refused gibt genau einen Credit frei, niemals doppelt.
- FIFO peek liefert wiederholt gleiche immutable Gruppe/Ticket ohne drain;
  foreign/stale/reordered/doppelter Ack und vollständige/partielle Consumer-
  Refusals erhalten Body/Seq/Charge. Gültiger Headack gibt exakt dessen
  Charge/Count frei. Keine public arbitrary Group-/Instance-/Ticketfactory.
- Sequencecredit bei MAX-1/MAX/MAX+1, konkurrierenden Reservations und Abort:
  kein wrapping, keine postCommitrefusal, keine durch Retry vergebenen Werte.
- Disconnect/Rejoin und letzter Ticketunload behalten echte Pending/Outbox;
  Counterpriority/-saveids bleiben auf Refusal erhalten. Stop-/Shadowplan
  unterscheidet Confirm mit retained Ausgabe von definitiver Freigabe.
- Alte Worklease-/Completion-/Counter-/Save-/Unload-/Stopverträge in nicht
  opt-in Mode bestehen unverändert. WholeSim Debug/Release und strict
  Alltargetclippy/Fmt/Rustdoc, exakte Preservation/Freezes, unabhängiger Rootreview.

Dies ist Source-/Vertragsprüfung, noch keine bestandene Runtime-/Viewer-/
Collectorbinding-/Survival-/Pluginabnahme oder Performance-/RSSmessung.
Der vorherige pausierte Zwischenfreeze
`f917614ab13ffe3c7302b4079167bd37edd0f7790da73bce83196dce4a3db7d9` bleibt
unverändert; dieser konkrete Entwurf bekommt einen eigenen Freeze.
