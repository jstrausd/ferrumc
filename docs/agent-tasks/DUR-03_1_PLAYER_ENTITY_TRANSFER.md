# DUR-03.1 — Atomare Player↔Item-Entity-Ownership

Status: **an `select_world_ticket` vergeben; vollständiges Producerproposal
zur internen Rootprüfung vorgelegt**. Produktionscode benötigt die ausdrückliche
Rootfreigabe dieses Entwurfs und die Freigabe der STORAGE-ENTITY001.2-Files nach
ihrem unabhängigen Review. Der verbindliche Auftrag unten bleibt erhalten.

## Ziel

Eine begrenzte Storageoperation überträgt einen Player- und einen UUID-
Entitysnapshot atomar mit dauerhaften Revisionen und Transferreceipt. Drop,
Teilpickup und Fullpickup müssen danach einen sicheren Sim-/Appconsumer
erhalten können. Der Storageproducer behauptet keine semantische Itembilanz.

## Crate und Dateibesitz

Ein später zugewiesener Autor besitzt ausschließlich `ferrumc-storage` und
diesen Brief. Root besitzt Git, Board, Katalog und Integration. Bestehende
STORAGE-ENTITY001.1/.2-Files bleiben bis zu ihrer Abnahme eingefroren.
Keine Items-/Sim-/App-/Core-/Lockfileänderung und kein generisches
Transaktionsframework. Neue Identitätswerte dürfen zunächst konkrete private-
field Storagewerte sein; ein anderer Crateproducer erhält bei Bedarf ein
eigenes Ticket.

## Kontext

- [Storagearchitektur](../architecture/storage.md), [Backendentscheidung](../adr/0004-storage-backend.md)
  und `server/crates/ferrumc-storage/INVARIANTS.md`.
- [UUID-Designreview](STORAGE-ENTITY001_UUID_PERSISTENCE_DESIGN_REVIEW.md#dur-03-bleibt-eine-eigene-versionierungs--und-writergrenze)
  und [UUID-Design](STORAGE-ENTITY001_UUID_PERSISTENCE_DESIGN.md#dur-03-als-eigenes-späteres-producerpaket).
- UUIDproducer `.1`, explizite Format4→5-Migration `.2`, SIM-03.3s sealed
  Itemstate/FCIE und ITEMS-TRANSFER-001s verlustfreie Transferpläne.
- [Paperreferenz](../PAPER_REFERENCE.md): vollständige ItemEntity.playerTouch,
  Player.drop und Inventory-Aufnahme-/Dropcallees sowie Savepfade/Patches lesen.
  Event-/Cancellation-/Threadordnung für spätere Consumer dokumentieren.
  Originaldateisaves sind keine atomare redb-Transferreferenz.

## Verbindliche Verhaltenseckpunkte für den Entwurf

1. Genau ein Player und eine UUID-Entity im ausdrücklich geprüften gleichen
   World-/Dimensionscope. Ein tatsächlicher Backendowner führt Receiptlookup,
   beide CAS-Prüfungen, Player-/Entitywrite oder Deletion, alle UUIDindizes,
   Chunkeepochs, Ownershiprevisionen und Receipt in einer realen gemeinsamen
   Writertransaction aus. Memorybackend besitzt dieselbe atomare Grenze.
2. Dauerhafte Revisionen werden vom Storage vergeben, sind positiv und wrappen
   nicht. Delete hinterlässt einen revisionierten Tombstone. Never-seen absence
   und versionierte Tombstoneabsence müssen verschieden sein; kein ABA nach
   Create/Delete/Recreate und keine stille Rücksetzung durch GC.
3. Receiptlookup geschieht vor live CAS. Ein identischer bereits committed
   Retry liefert die ursprüngliche Outcomeevidenz, auch nach späteren Writes.
   Dieselbe Transfer-ID mit anderer Requestidentität verweigert vollständig.
   Der Entwurf muss exakte bounded Canonicalbytes oder ausdrücklich benannte
   kryptografische SHA256-Identität wählen und den Ressourcenvertrag begründen.
4. Receipts dürfen nicht still ablaufen oder gelöscht werden, sodass alte IDs
   erneut ausführbar werden. Expliziter bounded Admission-/Retentionvertrag
   mit typisiertem Capacityfehler; sichere Retirement-/Replaygrenzen bei Bedarf
   separat. Ein Lostreply ist kein Abortbeweis.
5. Jeder Writer, der die beiden Ressourcen überschreiben könnte, ist im
   neuen Ownershipmodus versionsgeführt oder Unsupported. Insbesondere keine
   blinden UUIDsave/delete/move-, Player-save/delete- oder Autosavepfade neben
   CAS. Revisionsincrement allein verhindert konkurrierendes Überschreiben
   nicht. Alte Modi behalten ihre vollständigen bisherigen Verträge.
6. Playerrecords sind heute unscoped. Neue dauerhafte Playerownership führt
   den Scope ausdrücklich; mehrere vorhandene Descriptoren erlauben keine
   stille Weltwahl. Eigener versionierter Layout-/Modevertrag, keine nebenbei
   hinzugefügte Format5-Tabelle. Bestehende Daten verweigern mit expliziter
   Migrationanforderung; nötige Scope-Migration wird getrennt spezifiziert.
7. Explizite positive Request-/Payload-/Batch-/Receipt-/Scanlimits vor
   Allocation oder Write. Wiederverwenden der vorhandenen bounded Record-
   grammatik und vollständigen Generationidentityprüfung in derselben
   Mutationsgrenze. Neue API veröffentlicht keine DB-/Mutex-/Transactionhandles.
8. Konkrete Error- und Outcomevarianten unterscheiden unsupported mode,
   malformed data, identity mismatch, CAS conflict, Tombstone-/Freshnessfehler,
   Receiptcollision, Capacity, Revision/Epoch exhaustion, Backendfehler und
   ungewisses Commit. Keine Normalisierung oder ersetzenden leeren Inventare.

## Begrenzung

Container-/Overlay-/Hoppertransfers, Itemmerge, mehrere Player/Entities,
Crossdimension-/Crossshardtransfer, Javaeventbridge und App-/Simaktivierung
folgen separat. Der erste Producer darf sie nicht durch unfertige allgemeine
APIs vorwegnehmen. Kein automatisches Upgrade einer Welt und kein Claim von
Itemconservation allein aus opaquen Storagebytes.

## Konkreter Entwurf vor Umsetzung

Der Autor muss die vorhandenen Writer/Backends vollständig inventarisieren und
ein umsetzbares Proposal mit exakten Methoden/privaten Typen, Tablecatalog/
Key-/Value-/Receiptgrammatiken, Read-/CAS-/Commit-/Retryordnung, Default-
Unsupportedverhalten, Modeadmission, Migrationgrenzen, Byte-/Cachelimits,
Tombstone-/Receiptretention und Event-/Consumerseams liefern. Dazu konkrete
Testfälle und Dateiliste. Root prüft diesen vollständigen Entwurf und gibt
ihn innerhalb der bestehenden Nutzerautorisierung frei; keine erneute
Betreibergenehmigung für Routineimplementierungsentscheidungen.

## Abnahme und Invarianten

- Keine Teilwrites bei Konflikt/Returned-error; keine fehlende Tombstone-
  History; identischer Retry vor/nach Lostreply und Reopen, kollidierende ID,
  späterer CAS, zwei Pickups und Delete/Recreate mit alten Expectations.
- Reale Producer-Cutpoints vor/nach jedem Tablewrite und tatsächlichem
  Commit; vollständige Rawrelations-/Revision-/Receiptprüfung. Ungewisses
  Commit als solches berichten. Prozesskill-/Powerlossabnahme separat benennen.
- Beide Backends gleich; vollständiger Catalog-/Parser-/Malformed-/Boundary-
  Scan einschließlich maximaller Payloads, unknown schemas, fremden Scopes,
  fehlenden Descriptors, Revision/EpochMAX und allen blindwriterrefusals.
- Meaningful independent model/probes statt ausschließlich
  Encoder↔Decoder-Selbsterwartungen. Semantische Itembilanz gehört zusätzlich
  zum nachfolgenden Simconsumer mit ITEMS-TRANSFER-001.
- Vollständige Storage-Regression Debug/Release, strict all-target Clippy
  `-D warnings`, owned fmt und Rustdoc. Exact SHA256-Freeze und unabhängiges
  Fremdreview vor Rootintegration.

Nach CLEAN folgen DUR-04s Reservations-/Pending-/Ackconsumer und DUR-05s
produktiver Driver: sichtbare Pickup-/Take-/Removeoutputs erst nach bestätigtem
Commit; Disconnect/Unload/Shutdown behalten die Pendingauthority bis Klärung.
Dieses Ticket schließt die verbleibenden DUR-03-Erweiterungen nicht ab.

## Producerproposal v1 — intern zur Implementierung freigegeben

Dieser Abschnitt ist der konkrete vorgeschlagene Vertrag, noch keine Aussage
über implementiertes oder im Spiel aktiviertes Verhalten. Root hat ergänzend
receiptgestützte CAS-Einzelschreiber für Bootstrap/Autosave zugelassen, eine
globale Player-UUID mit ausdrücklich gebundenem aktuellem Scope verlangt und
die dauerhafte Verifikation des Capacityvertrags bei Reopen vorgeschrieben.
Crossdimensionwriter bleiben in `.1` verweigert. Alle folgenden APIs gehören
ausschließlich `ferrumc-storage`; kein zusätzlicher Crateproducer ist nötig.

Root hat am 2026-10-10 den vollständigen Entwurf mit SHA256
`b8ffca1ceff5c84e9626e983ea70047f70d785805fb4f5d25a38ed8699a68794`
innerhalb der bestehenden Nutzerautorisierung geprüft und zur Umsetzung
freigegeben. STORAGE-ENTITY001.2 ist nach unabhängiger Abnahme als `34acc8e9`
auf `main` integriert; dessen Dateifreeze ist für die hier benannten
Folgeänderungen aufgehoben. Bestehende Format3/4/5- und Migrationsverträge
bleiben verbindlich. Diese Freigabe ist keine Runtime-, Spiel- oder
Durabilityabnahme; vor Integration folgen die vollständigen Prüfungen und ein
unabhängiger Review des tatsächlichen Producers.

### 1. Inventar der bestehenden Authorities und neue Modegrenze

| Bestehender Einstieg | Heute | Im neuen Ownershipmodus |
| --- | --- | --- |
| `PlayerStore::load_player` | Unscoped UUID-Read | `Unsupported`; neuer explizit scoped Read |
| `PlayerStore::save_player/delete_player` | Blinder eigener Write | `Unsupported`, vor Eingabe-/Generationverarbeitung |
| `WorldStore` wire-Entity Load/Save/Batch/Delete | Legacy EntityId-Authority | Alle vier `Unsupported`, auch leerer Batch |
| `PersistentEntityStore` UUID Point/Page | UUID-/Locator-/Index-/Epochsnapshot | Nur readonly über dieselbe vereinheitlichte Ownership-Readgrenze mit Revision-/Receiptprüfung; keine CAS-Authority durch diesen Rückgabewert |
| UUID Save/Savebatch/Delete/Move | Blindwriter, obwohl Indizes/Epochs atomar | Alle vier `Unsupported`, einschließlich leerer Batch und Missingdelete |
| Neuer Player-CAS | Bisher nicht vorhanden | Genau ein receiptgestützter versionsgeführter Playerwrite |
| Neuer Entity-CAS | Bisher nicht vorhanden | Genau ein receiptgestützter versionsgeführter UUID-Entitywrite |
| Neuer Pairtransfer | Bisher nicht vorhanden | Genau ein Player und eine UUID-Entity, ein Owner/eine Writertransaction |
| Generation Ensure/Load | Unveränderliche tatsächliche Descriptoren | Beibehalten; Freshness umfasst sämtliche Ownershiphistory |
| Chunk/Overlay Save/Delete | Getrennte Worldauthority | Bisheriger scoped Generationvertrag; keine Player-/Item-Entityauthority |
| Journal Append/Receiptappend | Getrennte append-only Authority | Bisheriger Vertrag; kein Ersatz für Ownershipreceipt |
| Plugin KV Put/Get/Delete/Keys | Eigene Namespaceauthority | Bisherige Payload-/Keylimits; auch hier 4096 UTF-8 Namespacebytes vor Allokation |
| Offline 3→4 und 4→5 Migration | Explizite gesonderte Typestates | Unverändert; kein Ownershipupgrade oder neuer Scopeentscheid |

`InMemoryStore` hat heute getrennte Player- und UUID-State-Locks. Im neuen Modus
bleiben diese alten Maps leer und werden weder für Reads noch Writes benutzt.
Ein neuer `OwnershipState` enthält Playerpayloads, UUIDengine-State,
Player-/Entityownerheader, Receipts und Zähler in genau einem `RwLock`.
Der bereits vorhandene Generation-Mutationsboundary bleibt die äußere Grenze.
Insbesondere werden Pairreads nicht durch zwei getrennte öffentliche Reads
zusammengesetzt. `RedbStore` hat einen `Arc<Database>`; eine Ownershipoperation
kopiert genau diesen Arc in einen Worker und eröffnet genau eine Transaction.
Kein Zusammensetzen unabhängig übergebener `Arc<dyn PlayerStore>` und
`Arc<dyn PersistentEntityStore>` und kein Aufruf öffentlicher async UUIDwrites
innerhalb des neuen Producers.

### 2. Modeadmission, Konfiguration und persistenter Capacityvertrag

Eigener `GenerationMode::Ownership`, Datenbanklayout **Format6**. Neue konkrete
Konstruktoren:

```rust
InMemoryStore::with_player_entity_ownership(config: OwnershipConfig)
    -> OwnershipResult<Self>;
RedbStore::open_with_player_entity_ownership(
    path: impl AsRef<Path>, config: OwnershipConfig,
) -> OwnershipResult<Self>;
```

Keine neue implizite Defaultwahl. Die bestehenden Konstruktoren behalten exakt
ihre Format3/4/5-Verträge. Fresh ist ausschließlich ein tatsächlich durch
`create_new` neu angelegtes File; ein bestehendes leeres/markerloses File ist
kein Freshstore. Bestehende Formate3/4/5 verweigern vor irgendeiner logischen
Tabellenänderung mit `MigrationRequired { actual, required: 6 }`. Format6 muss
das vollständige unten beschriebene Catalog-/Grammatik-/Relationsreview
bestehen. Unknownfuture, falsche Tabellenform, Multimap, fehlender Marker,
fehlender Ownerheader oder halbes Upgrade werden nicht repariert.
redb Recovery-/Allocatorbytes sind weiterhin kein unveränderter physischer
SHA256-Vertrag; logische Tabellen/Rows dürfen bei Refusal nicht geändert sein.

Die private-field Konfiguration wird aus zwei konkreten Werten gebildet:

- `OwnershipCapacity::new(max_request_bytes: NonZeroUsize,
  max_receipts: NonZeroU64, max_revision_rows: NonZeroU64)`;
- `OwnershipOpenLimits::new(max_scan_rows: NonZeroU64,
  max_scan_bytes: NonZeroU64, max_journal_index_bytes: NonZeroU64,
  max_inflight: NonZeroU16)`;
- `OwnershipConfig::new(capacity, open_limits)`.

Alle haben Getter, keinen ungeprüften `Default` und keine öffentlichen Fields.
Requestlimit liegt in `149..=2*1024*1024`; kleiner als der feste Canonicalheader
ist ungültig. Receiptlimit muss `count*272` und Revisionlimit `count*55` in u64
darstellen können; die gemeinsam addierte logische Historyobergrenze wird
checked berechnet. Scan-/Scratchlimits sind ausdrücklich positive Obergrenzen;
`max_inflight` liegt in `1..=64`. Kein saturating arithmetic.

Die drei Capacitywerte werden bei Freshinitialisierung in derselben
Format6-Transaction dauerhaft gespeichert. Ein normaler Reopen bestätigt sie
**exakt**, unabhängig von den bereits verbrauchten Zählern. Ein anderer
Receipt-/Revision-/Requestvertrag wird mit `CapacityContractMismatch`
verweigert, auch wenn er größer wäre. Es gibt in `.1` keinen Reset, keine
Kapazitätserhöhung, kein Receiptretirement und keine GC-API. Eine spätere
administrative Vertragserweiterung benötigt ihr eigenes geprüftes Ticket.
Scan-/Scratch-/Inflightlimits sind lokale Ressourcenadmission und dürfen beim
Reopen geändert werden; sie ändern keine dauerhafte History oder Replaygrenze.
Wird ein bestehender Snapshot unter dem Scanlimit nicht vollständig geprüft,
verweigert Reopen ausdrücklich mit `ScanCapacity`.

Ein Arc-geteilter Semaphorepermit deckt jede neue Ownershipoperation und die
readonly UUIDownership-Adapter bis zum Ende ihres Workers ab. Admission per
`try_acquire_owned`: voller Producer liefert `Busy`, startet keinen Worker,
alloziert keine interne Payloadkopie und blockiert nicht in einer unbegrenzten
Wartequeue. Die Closure besitzt den Permit, sodass ein gedroppter Future die
noch laufende I/O nicht aus dem Budget nimmt. Memory verwendet denselben
Permitvertrag inline, ohne await unter Locks. Vom Caller bereits gehaltene
Payloads/Futures sind kein behauptetes Storage-RSS-Limit.

### 3. Exaktes Format6-Catalog und Ownergrammatiken

Genau **15 einfache Tabellen**, keine Multimaps:

| Tabelle | Key/Value-Typ und Grammatik |
| --- | --- |
| `ferrumc:meta` | `&str -> u64`, exakt erlaubte Keys unten |
| `ferrumc:generation` | Bisherige Key8/WG27-Grammatik, maximal4096 tatsächliche Descriptoren |
| `ferrumc:chunk` | Unveränderte ChunkKey16/Recordgrammatik |
| `ferrumc:chunk_overlay` | Unveränderte Overlaygrammatik |
| `ferrumc:entity` | Bisherige scoped UUID-Key24/FCUE22+Payload, keine wire-EntityId-Rows |
| `ferrumc:entity_locator` | Unveränderte World+UUID-Key20/Dimension+Chunk-Value12 |
| `ferrumc:entity_chunk` | Unveränderte Chunk+UUID-Key32/empty-Value |
| `ferrumc:entity_chunk_epoch` | Unveränderte ChunkKey16/positiver u64-Value8 |
| `ferrumc:player` | Globale PlayerId-Key16; unverändertes `schema:u32 + gamemode:u8 + opaque payload` |
| `ferrumc:plugin` | Bisherige checked Namespace/Key/Valuegrammatik |
| `ferrumc:mutation_log` | Bisherige zusammenhängende Journalgrammatik |
| `ferrumc:mutation_batch` | Bisherige Journalreceiptgrammatik |
| `ferrumc:player_ownership` | Globale PlayerId-Key16/Ownerheader39 |
| `ferrumc:entity_ownership` | World+UUID-Key20/Ownerheader35 |
| `ferrumc:ownership_receipt` | Caller-TransferId-Key16/festes Receipt256 |

Nur diese Metaeinträge:

- verpflichtend `format_version = 6`;
- optional bisheriges `last_mutation_id`, mit unverändertem Journalabgleich;
- verpflichtend `ownership_receipt_count` und `ownership_revision_count`;
- verpflichtend `ownership_request_limit`, `ownership_receipt_limit` und
  `ownership_revision_limit`.

Zähler dürfen0 sein; Capacitylimits nicht. Reopen zählt die tatsächlichen Rows
und verlangt exakte Gleichheit sowie `count <= persisted limit`. Revisioncount
ist die Summe aller Player-/Entityownerheader, einschließlich Tombstones.
Delete senkt keinen Historyzähler. Unknownmetadata, duplizierte/falsche Keytypen
und nicht passende Werte verweigern. Beide Countupdates, Headerwrites und
Receiptinsert gehören zur selben Writertransaction wie die Payloads.

Alle numerischen Felder sind big-endian, UUIDs die vollständigen16 Bytes.
Header sind exakt, ohne trailing bytes:

| Playerheader39-Offset | Inhalt |
| --- | --- |
| `0..4` | Magic `FCOP` |
| `4..6` | Headerschema u16 `1` |
| `6` | State `1=Live`, `2=Tombstone`; `0=Fresh` nie gespeichert |
| `7..11` | WorldId u32 |
| `11..15` | DimensionId u32 |
| `15..23` | positive Ownershiprevision u64 |
| `23..39` | ID des Receipts, das diese Revision zuletzt änderte |

| Entityheader35-Offset | Inhalt |
| --- | --- |
| `0..4` | Magic `FCOR` |
| `4..6` | Headerschema u16 `1` |
| `6` | State `1=Live`, `2=Tombstone` |
| `7..11` | DimensionId u32; WorldId steht im Key20 |
| `11..19` | positive Ownershiprevision u64 |
| `19..35` | ID des Receipts, das diese Revision zuletzt änderte |

Ein Player bleibt global UUID-keyed; sein Scope steht ausdrücklich im Header.
Ein Entityheader ist World+UUID-keyed und bewahrt auch nach Delete die Dimension.
In `.1` verweigert jeder Read-/CASscope, der einem gespeicherten Owner widerspricht,
mit `ScopeConflict`; kein leerer Ersatzsnapshot und kein zweiter Player in
einem anderen Scope. Spätere legitimierte Scopewechsel dürfen diese monotone
UUIDhistory nicht zurücksetzen. Crossworld-/Crossdimensionänderungen haben
hier keinen Writer.

Liveheader verlangt genau einen passenden Payloadrecord; Tombstone verlangt
dessen Abwesenheit. Jedes Payloadrecord verlangt den entsprechenden Liveheader.
Entitylocator/index/epoch gelten zusätzlich exakt wie bisher. Tombstone verlangt
keinen world/UUID-Locator; seine genaue frühere Scopedrow fehlt. Orphanindices
werden im vollständigen rückwärts gerichteten Scan erkannt. Fresh bedeutet
kein Header, kein Payload und keine entgegenstehende world/UUID-Locatorrelation.
Jeder Header, einschließlich Tombstone, verlangt seinen tatsächlichen Scope-
Descriptor und ein vorhandenes Lastchangingreceipt mit exakt dieser Resource,
Scope, Generationidentity und Afterstamp. Damit kann Reopen aktuelle Revisionen
direkt auf ihren committed Änderungsbeleg zurückführen, ohne Receiptgeschichte
weltweit in den Heap zu laden. Ein No-op-Receipt ersetzt diesen Headerzeiger nicht.

### 4. Konkrete öffentliche private-field Werte und API

Neue Datei `ownership.rs`, konkrete Storageidentitäten:

- `ScopedPlayerKey::new(GenerationKey, PlayerId)` plus Scope/ID-Getter;
- `OwnershipTransferId::from_bytes([u8;16])`, `to_bytes`; alle128 Bits bleiben
  erhalten, auch all-zero ist eine zulässige ID;
- `OwnershipRevision`: positive Storage-/Parserausgabe, `get`, kein öffentlicher
  ungeprüfter u64-Konstruktor;
- `OwnershipStamp`: Fresh/Live/Tombstone mit Getter und exakter fixed9-
  `encode/decode`-Grammatik: Tag0/1/2 plus u64; Fresh exakt0, andere positiv.
  Ein deserialisierter Stamp ist **keine** live CAS-Expectation;
- `OwnedPlayerExpectation` und `OwnedEntityExpectation`: private Resourcekey,
  kompletter Generationdescriptor, Stamp, Weak-Backendincarnation. Keine rohe
  Konstruktion oder öffentliche Owner-/Mutex-/DBhandles;
- `OwnedPlayerSnapshot` / `OwnedEntitySnapshot`: Key, tatsächlicher Stamp,
  `Option<PlayerRecord/PersistentEntityRecord>` und private-field Expectation.
  Fresh und Tombstone sind anhand des Stamps eindeutig verschieden;
- `OwnedPlayerEntitySnapshot`: beide Snapshots aus einer Readgrenze, expliziter
  gemeinsamer Scope/Generationidentity, Getter/`into_parts`;
- `PlayerOwnershipProposal::put(PlayerRecord)` / `delete()` und
  `EntityOwnershipProposal::put(PersistentEntityRecord)` / `delete()`;
  private-field Proposalwerte, keine beliebigen Actions/Tabellenhandles;
- `PlayerEntityTransferRequest::new(id, player_expectation,
  entity_expectation, player_proposal, entity_proposal)`;
- `PlayerOwnershipCasRequest::new(id, player_expectation, proposal)` und
  `EntityOwnershipCasRequest::new(id, entity_expectation, proposal)`;
- `OwnershipRequestIdentity`: fixed SHA256-Identität, Bytegetter, keine
  Schreibauthority. Jeder Request stellt `receipt_query()` bereit;
- `OwnershipReceiptQuery::from_bytes(id, [u8;32])`: zulässige rekonstruierbare
  readonly Abfrage für Crash-/Restartklärung, keine Erwartungsrevision;
- `OwnershipReceipt`, `OwnershipEpochChange`, `OwnershipCommitOutcome`:
  private-field Evidence mit Getter; Outcome unterscheidet `Committed` und
  `Replayed`, ohne die ursprüngliche Receiptbefunde umzuschreiben. Outcome
  liefert zusätzlich ownergebundene Expectations der ursprünglichen
  Afterstamps für aktive Ressourcen. Eine alte Replayrevision bleibt beim
  nächsten Write selbstverständlich CAS-stale;
- `OwnershipCapacity`, `OwnershipOpenLimits`, `OwnershipConfig` wie oben;
- `OwnershipError` und `OwnershipResult<T>` behalten konkrete Fehlerinformation
  cross-crate, insbesondere ungewisse Commitzustände. Keine neue Coreänderung.

Requests übernehmen die bereits bounded Payloads ohne Clone. Der synchrone
Konstruktor prüft Shape, gleiches ausdrückliches Scope, gleiche vollständige
Generationidentity und checked Canonicalgröße. Er prüft keine aktuelle
Backendincarnation oder lebende Revision vorweg; diese Prüfung folgt erst nach
Receiptlookup. Snapshot-/Expectationskeys bestimmen die betroffenen Ressourcen,
es gibt keinen getrennten ersetzbaren Resourcekey im Writeargument.

Dyn-kompatibler `#[async_trait] PlayerEntityOwnershipStore: Send + Sync`:

```rust
async fn load_owned_player(
    &self, key: ScopedPlayerKey, expected: GenerationIdentity,
) -> OwnershipResult<OwnedPlayerSnapshot>;
async fn load_owned_entity(
    &self, key: PersistentEntityKey, expected: GenerationIdentity,
) -> OwnershipResult<OwnedEntitySnapshot>;
async fn load_owned_pair(
    &self, player: ScopedPlayerKey, entity: PersistentEntityKey,
    expected: GenerationIdentity,
) -> OwnershipResult<OwnedPlayerEntitySnapshot>;
async fn commit_player_entity_transfer(
    &self, request: PlayerEntityTransferRequest,
) -> OwnershipResult<OwnershipCommitOutcome>;
async fn compare_exchange_owned_player(
    &self, request: PlayerOwnershipCasRequest,
) -> OwnershipResult<OwnershipCommitOutcome>;
async fn compare_exchange_owned_entity(
    &self, request: EntityOwnershipCasRequest,
) -> OwnershipResult<OwnershipCommitOutcome>;
async fn load_ownership_receipt(
    &self, query: OwnershipReceiptQuery,
) -> OwnershipResult<Option<OwnershipReceipt>>;
```

Alle Traitdefaults sowie Legacy/Metadata/UuidEntities-Modi liefern konkretes
`Unsupported`. Kein Default delegiert auf Blindwriter. Der Pairtransfer ist
die erste Operation und bleibt exakt1+1. Die beiden Einzelschreiber sind nötig,
um Player erstmals zu initialisieren, Entities zu spawnen und Autosaves zu
versionieren; sie teilen denselben Receipt-/Historyvertrag. Keine Dummyentity
für Playerautosave, kein Dummyplayer für Spawn, keine beliebige Vec-Batchform.

Readonly UUID-Point/Page im Ownershipmodus nutzen eine Ownership-Readadapter-
Variante von `EntityRead`. Jeder Zugriff auf Entitybytes prüft zusätzlich den
Liveownerheader und dessen Lastchangingreceipt, auch die bounded Page-Lookahead-
Row. Pointmissing geht durch den Fresh-/Tombstone-/Scopecheck. Memory hält die
vereinheitlichte Readlock, redb eine Readtransaction während aller Row-/Header-
Lookups. Page1..64/1MiB, UUIDcursor/incarnation/epoch und vollständiger
Generationcheck bleiben erhalten. Diese Seiten sind keine Ownershiptokens;
der reservierende spätere Consumer lädt einen expliziten Pair-/Owned-Snapshot
bevor er einen Transfer plant. Kein neuer unbounded Enumerationseinstieg.

### 5. Revision-, CAS-, Tombstone- und No-op-Regeln

| Vorher | Proposal | Outcome ohne Concurrentkonflikt |
| --- | --- | --- |
| Fresh0 | Put | Live1, neuer Header, Revisioncount+1 |
| TombstoneN | Put | LiveN+1, vorhandener Header bleibt dieselbe History |
| LiveN | anderes Put | LiveN+1 |
| LiveN | exakt rawgleiches Put | LiveN unverändert, neuer No-op-Receipt |
| LiveN | Delete | TombstoneN+1; Payload/Entityrelations weg, Header bleibt |
| Fresh oder Tombstone | Delete | `AlreadyAbsent`, keinerlei Write oder Receipt |

Rawgleich bedeutet Player Schema+GameMode+Payloadbytefolge bzw. Entity
Envelopechunk+Payloadschema+Payloadbytefolge. Unchanged Player ist keine
Entityausnahme. Vor beiden CASvergleichen werden encountered Relations/
Header/Lastreceipt vollständig geprüft; beschädigte Daten sind Corruption,
kein normaler CASconflict. Expectations müssen Resource, Scope, vollständige
Generationidentity und aktuellen Stamp bestätigen. Incarnations müssen
denselben tatsächlichen Backendowner bestätigen; Arcclones teilen ihn,
ein anderer Store oder Reopen nicht. Fehler nennt die betroffene Ressource und
expected/actual Stamps, ohne Teilwrite.

Revisionsincrement erfolgt für jeden tatsächlich geänderten Resourcezustand
genau einmal. Ein `u64MAX`-Header darf einen rawgleichen No-op bestätigen;
jede echte Änderung verweigert mit `RevisionExhausted`, die ganze Paaroperation
bleibt unverändert. Entityepoch ändert sich bei jeder echten Entityänderung
für die Vereinigung von alter/neuer Chunkspalte genau einmal pro Chunk,
maximal2. Auch reine Payloadänderung im gleichen Chunk erhöht den Epoch;
Player-only und Entityraw-No-op tun es nicht. EpochMAX verweigert die gesamte
echte Änderung; der erlaubte raw-No-op benötigt keinen Increment.

Das letzte Änderungsreceipt wird nur bei tatsächlicher Revisionänderung im
Header ersetzt. Revision-/Tombstonehistory wird nie gelöscht und verhindert
Fresh→Create→Delete→Recreate-ABA. Frische Generationinitialisierung muss selbst
nach komplettem Delete wegen Headern, Receipts oder leeren Chunkeepochs verweigern.

### 6. SHA256-Requestidentität und fixed256-Receipt

Explizite **kryptografische SHA256-Äquivalenz**, keine Behauptung mathematisch
exakter historischer Payloadgleichheit. SHA256 hält die Receiptkosten fest;
das vollständige bis1,25MiB Proposal wird nicht dauerhaft je Transfer dupliziert.
Die transiente Ownerincarnation ist absichtlich nicht Bestandteil des Hashes.
Canonicalstream, exact order:

```text
domain[29] = b"FerrumC ownership request v1\0"
kind:u8 = 1 Pair / 2 Player-CAS / 3 Entity-CAS
transfer_id[16]
scope[8] = WorldId + DimensionId
generation[27] = vollständige gespeicherte WG-Identitygrammatik
player_id[16] (inaktiv bei Entity-CAS: alles0)
entity_uuid[16] (inaktiv bei Player-CAS: alles0)
expected_player_stamp[9] (inaktiv: Fresh0)
expected_entity_stamp[9] (inaktiv: Fresh0)
player_proposal_tag:u8 + player_bytes_len:u64 + player_bytes
entity_proposal_tag:u8 + entity_bytes_len:u64 + entity_bytes
```

Proposaltag0=inaktiv,1=Put,2=Delete. Inaktiv/Delete haben Länge0 und keine Bytes.
Put hat die exakten vorhandenen kodierten Recordbytes. Canonicalbase149 Byte;
Playerhöchstwert1MiB+5, Entityhöchstwert256KiB+22, zusammen maximal1.310.896 Byte
und damit kleiner als die harte Requestgrenze2MiB. Fullgeneration, beide
Expectedstamps, GameMode/Schema, Chunk, Itempayloadbytefolge und Operationkind
gehen in die Identität ein. Der Konstruktor kennt diese Länge checked vor
Encoding; runtime bestätigt das gespeicherte `max_request_bytes` vor internen
Kopien. Hashen streamt Header und Recordfields/body; kein vollständiges
Canonical-Vec zusätzlich zu den encoded Recordbuffers.

Receipttabelle Key16 ist die supplied ID; fixed256-Value:

| Offset | Inhalt |
| --- | --- |
| `0..4` | Magic `FCTR` |
| `4..6` | Receiptschema u16 `1` |
| `6` | Kind1/2/3 |
| `7..23` | supplied TransferId, muss Tablekey entsprechen |
| `23..55` | SHA256 des vollständigen Canonicalrequests |
| `55..63` | Scope8 |
| `63..90` | Generationidentity27 |
| `90..106` | PlayerId16, inactive bei Kind3 exakt0 |
| `106..122` | EntityUUID16, inactive bei Kind2 exakt0 |
| `122..131` | Beforeplayerstamp9 |
| `131..140` | Afterplayerstamp9 |
| `140..149` | Beforeentitystamp9 |
| `149..158` | Afterentitystamp9 |
| `158` | Epochentrycount0..2 |
| `159..191`, `191..223` | Je ChunkKey16+Beforeepoch8+Afterepoch8 |
| `223` | Beforeentitychunkpresent0/1 |
| `224..232` | Beforeentity ChunkPos x/z i32, inactive alles0 |
| `232` | Afterentitychunkpresent0/1 |
| `233..241` | Afterentity ChunkPos x/z i32, inactive alles0 |
| `241..256` | Reserved15, zwingend alles0 |

Inaktive Resourcehälften haben Null-ID und Fresh0-Before/After; aktive
Afterstamps sind nie Fresh. Parser prüft zulässige Transitionen aus Abschnitt5:
Fresh→Live1, Tomb→LiveN+1, Live→LiveN oderN+1, Live→TombN+1; nie wrap, nie
Tomb→Tomb. Liveentitystamp verlangt den entsprechenden checked ChunkPos,
Fresh/Tomb verlangt absent/zero. No-op-Entity hat gleiche Before-/Afterchunks
und0 Epocheinträge. Bei echter Entityänderung sind die1..2 Epocheinträge genau
die sortierte distinct Vereinigung der vorhandenen Before-/Afterchunks im
gemeinsamen Scope. Inaktive entries sind Nullbytes; aktiv `after=before+1`,
After positiv; keine Duplicatechunk/-fremden Scopes oder inkonsistenten Flags.

Startup prüft für jedes Receipt tatsächlichen Descriptor/volle Identity,
alle aktiven Ownerhistories und `receipt.after_revision <= retained_revision`,
sowie `receipt.after_epoch <= retained_epoch`. Historische Afterpayloads oder
Live/Tombstates müssen nicht dem heutigen Zustand entsprechen. Bei gleicher
Revision muss der heutige Stamp dem historischen Afterstamp entsprechen;
bei einem neueren Header darf ein historischer Livezustand inzwischen deleted
oder recreated sein. Ownerheader-Lastreceipt verlangt zusätzlich exakte aktuelle
Afterrevision/-state und bei Liveentity den korrekten Afterchunk. Receipts
behalten sämtliche ursprüngliche Outcomeevidence; Replay rewritet sie nicht.

### 7. Transactionordnung, Retry, ungewisses Commit und Reads

Neue Writes laufen nach Mode-/Shape-/Byte-/Busyadmission auf einem einzigen
Worker; im Worker nach `begin_write`:

1. IDreceipt laden und vollständig parsen. Andere RequestSHA: `ReceiptCollision`,
   keinerlei Write. Identische SHA: Originalevidence zurück, bevor aktuelle
   live Expectations, Ownerincarnation oder volle Capacity geprüft werden.
   Damit funktioniert auch Retry nach späteren CASwrites, Delete/Recreate,
   Reopen, Lostreply und an der voll belegten Historygrenze.
2. Für eine neue ID genaue Scope-/Generationidentity und actual Ownerbindung
   bestätigen; Header/Payload/Locator/Index/Epoch/Lastreceiptrelationen in
   dieser Writertransaction prüfen. Beide Expectedstamps vergleichen.
3. Deleteabsence, neue Header-/Receiptadmission, alle Revision-/Epochincrements
   und sämtliche Recordencodings preflight. Plan enthält höchstens ein
   Playerwrite, einen Entityengine-Plan, zwei Headerwrites, zwei Chunkeepochs,
   ein Receipt und zwei Counterupdates. Kein weltweiter Scan/Stateclone.
4. Alle Payload-/Index-/Locator-/Epoch-/Header-/Receipt-/Counterwrites in dieser
   einen realen Transaction. Keine öffentliche nested Mutation/Transaction.
5. `Durability::Immediate` und tatsächlicher `commit`. Nur dessen bestätigter
   Erfolg liefert `Committed`. Worker hält Owner-Arc und Admissionpermit bis
   zum Abschluss, auch bei gedropptem Aufruferfuture.

Receiptlookup vor Live-CAS hebt Mode-/grammatische Admission nicht auf. Ein
malformed Receipt darf nie eine erfolgreiche Replayantwort erzeugen; eine
bytewise gleiche SHA auf beschädigter Evidence wird Corruption. Ein Request
aus fremden Expectations kann einen existierenden identischen Beleg abfragen,
aber ohne identischen Beleg keinen neuen Write an einem fremden Owner ausführen.

`commit`-Fehler ist `CommitUncertain` mit ID, Requestidentity und konkretem
Backendcause. Ein Join-/Workerfehler nach Start ist konservativ
`ExecutionUncertain`, ebenfalls mit ID/Identity; er kann nach Commit liegen.
Ein Lostreply oder dropped Future ist kein Abortbeweis. Errors vor Commit und
testweise Returnederrors an den Tablecutpoints aborten und erzeugen keinen
Receipt. `load_ownership_receipt -> None` beweist nur Abwesenheit in dieser
Readtransaction, nicht, dass ein noch laufender Worker nie später committet.
Consumer müssen Pendingauthority behalten und identische IDs weiterklären.

Restartrecovery ohne öffentliche rohe live Expectations:

- Consumer persistiert sein eigenes bounded Pendingproposal, Scope/Identity,
  Resourcekeys, die exakten fixed9 Expectedstamps sowie `ReceiptQuery`;
  diese Consumergrammatik ist DUR-04, keine versteckte Storage-API in `.1`.
- Nach Reopen zuerst readonly `ReceiptQuery`: Originalbeleg erlaubt Klärung
  unabhängig von neuen live Revisionen. Gleiche ID/falsche SHA kollidiert.
- Nur bei fehlendem Beleg und nach geklärter Worker-/Ownerlebensdauer darf der
  Consumer einen frischen Pair-/Ownedread durchführen. Nur wenn dessen Stamps,
  Scope, Ressourcen und Generation exakt den gespeicherten Expectations
  entsprechen, erhält er aktuelle private Liveexpectations und rekonstruiert
  denselben Request. Vor Submit verlangt er exakte Gleichheit von neuer und
  gespeicherter SHAquery. Bei anderem Zustand nicht umplanen unter derselben
  ID; Pendingkonflikt bleibt ausdrückliche Consumerentscheidung.

Reads werden auf Memory innerhalb `boundary.read → generations.read →
ownership_state.read` ausgeführt. Beide Payloads und beide Header/Expectations
werden unter derselben State-Readlock erfasst; kein Zwischenzustand während
Apply. redb verwendet genau eine Readtransaction für Descriptor, beide Payloads,
Header und Receiptrelationen. Sequential getrennte Reads ersetzen diesen
Pairread nicht. Receiptread ist gleichfalls ein kohärenter einzelner Snapshot.
Keine Lock-/Transactionguard überschreitet ein await.

Memorywrites nehmen dieselbe äußere Grenze, dann eine einzige Ownership-
Statelock. Vollständiger fallibler Preflight vor Veröffentlichung. Anwendung
hat keinen falliblen Returnederror nach dem ersten Publish; Allocationabort
ist kein behaupteter Rollback. Es gibt kein gesamtes Weltclone für Rollback.
Reader sehen ausschließlich den Zustand vor oder nach vollständigem Apply.
Testhooks unterscheiden Preapplyrefusal und Postpublish-Lostreply; echte
tableweise Returnederrorabortbeweise werden beim redb Producer geprüft.

### 8. Ressourcen, vollständiger Scan und konkrete Wiederverwendung

Harte Payloadgrenzen bleiben1MiB Player und256KiB Entity; Batch ist exakt1+1
oder genau ein Einzelschreiber. Interne encoded Payloadbuffers sind höchstens
1.310.747 Byte, zusätzlich höchstens zwei bounded geladene Snapshots, ein
konstanter Receipt256 und fixed Rows/Plan für maximal2 Chunks. Kein Speicher-
oder Performancevergleich allein wegen Rust oder dieser kodierten Bytegrenze.
redb Maincache64MiB, disposable Journalreceiptindexcache16MiB, Decoderheap und
redbinternals kommen hinzu. Memory behält alle tatsächlich gespeicherten Daten
und Mapoverhead; positive Historykapazitäten begrenzen Rows, nicht gesamten RSS.

Neuer eigenständiger `redb_store/ownership_validation.rs`:

- exact15 Catalog-/Typ-/Metagates vor Dateninterpretation, eigener Format6-
  Fingerprintdomain und fixed15 Countslots;
- checked Scanrow-/Byteadmission vor Decoder-/Bodycopy. Jeder streamed key+value
  zählt einschließlich Metarows. Relationpointlookups sind eine feste bounded
  Anzahl je Row und erzeugen keinen vollständigen Weltindex im Heap;
- vollständige Hin-/Rückprüfung aller UUIDrelations, Playerpayload↔Owner,
  Entitypayload↔Owner, Histories↔Lastchangingreceipt, Receiptgrammar/-historic
  revision/-epoch, Count/Capacityvertrag, tatsächliche Descriptoren, sämtliche
  bisherigen Chunk-/Overlay-/Plugin-/Journalrecordgrammatiken;
- shared UUID `raw`-Parser und `engine::verify/lookup/epoch` werden über konkrete
  Reader benutzt. Die neue Ownerprüfung ist zusätzlicher Adapter, keine
  Revision aus dem Chunkeepoch;
- bestehendes `uuid_validation::validate_payload_row` wird ausschließlich als
  crate-private Helper freigegeben, ohne Body-/Format5-Vertragsänderung.
  Format6 delegiert nur Generation/Chunk/Overlay/UUIDrelation/Pluginrowgrammatik
  dorthin; Playerdispatch ist neu scoped, Meta/Catalog/Fingerprint separat;
- Journalprüfung über den bereits bounded
  `uuid_migration::validation::JournalValidation` mit dessen bestehendem
  `max_journal_index_bytes` und `finish`-Cleanup. Nur ein konkreter Adapter von
  dessen Fehlern zu OwnershipError, kein marker5-Trick oder Änderung der
  unveränderten3→4-/4→5-Migrationssemantik;
- Scratchindex ist eigener `create_new`-Tempfile, cache16MiB und physisch
  budgetgeführt. Entfernt wird nur die eigene verifizierte Dateidentität nach
  DB-/Transactiondrop; Replacementpath bleibt erhalten. Unsupported Platform
  für diese Fileidentity wird ausdrücklich berichtet. Keine unbegrenzte
  Range-Vec und kein Startscan pro Gameplaycall.

Runtime prüft encountered Relations; unknown orphan Rows sind vollständige
Startupscanaufgabe. Die neuen History-/Receiptparser sind fixed-width und
prüfen Länge/Schema/Tag/Null-/Reservedfelder vor jede Kopie. Opaque Simpayload-
schemas werden weiterhin bytewise erhalten und nicht als FCIE/FCPP verstanden;
Unknownownershipheaderschema oder Unknownreceiptschema verweigert dagegen.

### 9. Konkrete Fehleroberfläche

`OwnershipError` als nicht-exhaustive konkrete Enum bzw. private-field
Fehlerpayloads mit Getter. Neue Traitreturns nutzen `OwnershipResult`, nicht
string-only Klassifikation. Geplante Varianten:

- `Unsupported`, `InvalidConfiguration`, `MigrationRequired`,
  `CapacityContractMismatch`, `CatalogMismatch`;
- `MalformedOwnershipData` mit Table/klassifiziertem festen Reason und
  `Record` mit vorhandener StorageErrorquelle;
- `MissingGenerationIdentity`, `GenerationIdentityMismatch`, `ScopeMismatch`,
  `ScopeConflict`, `BackendOwnerMismatch`;
- `CasConflict` mit konkret Player/Entity, expected/actual Stamp;
- `AlreadyAbsent` mit konkret Player/Entity und Fresh/Tombstonebefund;
- `ReceiptCollision` mit supplied ID und expected/stored SHAidentitäten;
- `RequestBytesExceeded`, `HistoryCapacity` mit Receipt/Revisioncount,
  `ScanCapacity`, `ScratchCapacity`, `Busy`;
- `RevisionExhausted`, vorhandener klassifizierter `EpochExhausted`;
- `Backend`, `CommitUncertain`, `ExecutionUncertain`.

Errorquellen bleiben programmatic unterscheidbar; kein Catch-all, das einen
ungewissen Write als sicheren Abort verkauft. Alte Traitmethoden mappen die
neue Unsupported-/Readonlycorruption-Klassifikation ausdrücklich nach
ServerError, ohne ihre alten Modefehler zu verändern. Neue Writer bleiben
typed; kein Core-/App-/Lockfileedit zur Fehlertransportierung.

### 10. Paperreferenz und konkrete Consumergrenzen

Pin: Minecraft Java1.21.8/772; Paper1.21.8 Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Frisch gelesene Quellen/Patches stehen
im lokalen Manifest `.codex-tmp/dur03_1-reference.sha256`, SHA256
`7d172f9a6b59b4a0790eb729d3fcfa3637781defbc828438ba95239ce802a10c`.
Das Manifest enthält11 applied Javaquellen und5 sourcepatches; die Source-
patchhunks wurden zusätzlich zu den vollständig relevanten applied Methoden
gelesen. Vorherige Storage001-Savereferenz wird als wiederverwendetes Source-
review ausgewiesen, kein neuer Paperlauf oder Differentialpass.

Frisch geprüft:

- Applied `ItemEntity.playerTouch`, insbesondere414–504, plus Sourcepatch-
  Pickupeventhunk183–258: serverseitig zuerst AttemptPickup bei Delay<=0,
  dann bei Kapazität>0 Legacy PlayerPickup, danach EntityPickup. Legacy/New
  Cancellation stellt vorherige Count zurück; Attempt/Legacy `flyAtPlayer`
  darf auch bei Cancellation eine reine Takeanimation auslösen. Plugin-
  ersetzter Itemstack wird nach Events erneut gelesen. Danach Delay-/Target-
  UUID-/`Inventory.add`-Gate; Take, bei empty RemovalcausePICKUP, Stats und
  onItemPickup. Ein Wrapperinterface allein hätte diese Ordnung nicht gezeigt.
- Vollständige relevante `Inventory.canHold`, remaining-space-/addResource-
  overloads, Slotwahl, `add`354–405, selected removal und `ContainerHelper`
  sowie `ItemStack` split/copy/count/maxstack/componentmatching. Diese Regeln
  bleiben Items-/Simauthority, Storage ist opaque.
- `Player.drop`, `LivingEntity.drop`803–850, `createItemStackToDrop`3930–3968,
  ServerPlayerdropoverloads und relevante Sourcepatches: Stackkopie mit
  Originalcount0, Pickupdelay40/Thrower/Motion, callback vor DropEvent,
  Cancellationreturn samt Return-to-hand/fallback, erst danach addFreshEntity.
- Gesamte relevante `handlePlayerAction`1883–1999: main-thread gate,
  immobile/client-loaded/spectator und Single-Drop-Spam-/Ticklimits sowie DropAll.
- Vollständige `PlayerDataStorage` angewandte Quelle+Sourcepatch: getrennte
  komprimierte Temp-`.dat`/safeReplace-Saves, optional deaktiviert und eigene
  Fehlerbehandlung. `EntityStorage` und aktive Moonrise `ChunkEntitySlices`
  save/saveEntityChunk/saveEntityChunk0 sowie `NewChunkHolder.saveEntities`
 1791–1835: getrennte entity-region Speichervorgänge und async IO.

Daraus folgt für nachfolgende DUR-04/05-Consumer:

- Event-/Cancellation-/Stackreplacement-Logik und Transfermengen vor persistiertem
  Submit unter der reservierten tatsächlichen Simauthority planen. Nach einem
  Retry Events nicht erneut als neuer Pickup/Drop ausführen.
- Pluginbedingte Mengenänderung ist ausdrückliche external Mutationauthority;
  keine Behauptung einer reinen Zweiressourcenconservation bei arbitrary
  Pluginwrites. ITEMS-TRANSFER-001 validiert quantities/components; Storage
  validiert nur Request-/Revision-/Persistenzvertrag.
- FerrumC publiziert tatsächliche Inventar-/Entity-/Take-/Remove-/Statänderung
  erst nach bestätigtem Commit; die pure Paper-cancelled-flyAtPlayer-Animation
  kann als gesonderter kosmetischer Eventeffekt modelliert werden. Diese
  bewusst getrennte Klärung ist keine vorgezogene Javaeventbridge.
- Reservation bleibt über Disconnect/Chunkunload/Shutdown und ungewisses Commit
  bestehen, bis Receipt/Retry sicher geklärt ist. Autosaves müssen denselben
  versionsgeführten Writer benutzen, niemals alten PlayerStore nebenher.
- Papers getrennte Datei-/Region-Saves beweisen keine joint redb-Transaction.
  Dieser Producer beansprucht noch keine Paper-Runtime-/Pluginabnahme, kein
  Survivalgameplay, keine Quantityconservation und keine produktive Aktivierung.

### 11. Meaningful unabhängige Probes und Abnahmematrix

Eine testseitige unabhängige Map-/Integer-State-Machine modelliert Player,
Entity, expliziten Scope, Fresh/Live/Tombstone, monotone Revision, Chunkeepoch,
ID→Requestbytes-/Receiptmapping und persistierte Capacity. Sie verwendet weder
Productionplan noch Productioncodec als alleinige Sollwertquelle. Kleine
hardcoded Rawfixtures mit independently berechneten Offsets/Hashstreams
prüfen die Grammatik; keine ausschließlichen Encoder↔Decoder-Selbsttests.
Deterministische Operationstraces gegen Memory/redb mit kompletter raw
Relation-/Header-/Receipt-/Counterprüfung nach jedem Schritt.

| Gruppe | Konkrete Abnahme |
| --- | --- |
| Jointwrites | Drop mit Freshentity, Teilpickup mit Restentity, Fullpickup mit Tombstone, Player/Entity-Singlebootstrap/-autosave, same-/different-chunk Entityupdate |
| Concurrency | Zwei konkurrierende Pickups desselben Items; konkurrierender Playerautosave; genau ein CAS gewinnt, Verlierer hat keine Deltas/Receipt |
| Owner/Scope | Arcclones erlaubt; fremder Memory-/redbowner und Reopen für neue CAS verweigert; identischer committed Retry mit alter Incarnation erfolgreich; globaler PlayerScope und EntityworldUUID geprüft |
| Readboundary | Kontrollierte Memoryreader während blockiertem Apply sehen ausschließlich vollständiges Vorher/Nachher; Pairpayloads+Stamps gemeinsam; UUIDpoint/page/lookahead verwenden Ownershipstate und erkennen malformed Header |
| ABA | Fresh→Live→Tomb→Live; alte Fresh-/Live-/Tombexpectations verweigern; Header/Counter nie reset; späterer Versuch neue Generation nach Delete verweigert |
| Receipt | Identischer Retry vor/nach späterem Write/Delete/Recreate/Lostreply/Reopen; SHAcollision-ID bei geänderter Menge/Schema/GameMode/Chunk/Scope/Generation/kind/Expectation; Originalevidence bleibt gleich |
| Boundaries | max Player/Entitypayload, requestlimit exact/one-over, receipt/revision cap exact/one-over, neue No-op-Receipt bei cap verweigert, existierender Retry bei cap erlaubt, persisted Capacity exakt beim Reopen |
| Exhaustion | Live/TombRevisionMAX, EpochMAX in alter/neuer Spalte, erlaubter raw-No-op beiMAX, Counter-/Canonicalchecked-overflow; ganzes Paar unverändert |
| Parsers/Reopen | Jede fixed-length Short/Long/Trailingcase, Magic/schema/tag/revision0, unknownreserved, inactive garbage, ungültiger GameMode/chunkorigin, wrongID, receipttransition/-epoch/-chunk, missing/foreign Descriptor, Ownerlastreceipt missing/wrong, bidirektionale Orphans, unbekannte/missing/wrongtype Catalog-/Metarows |
| Writerrefusals | Alle alten Player-/wire-/UUIDwriter/move inOwnership, einschließlich0-Batch/Missingdelete; alte Modi retain komplette Regression |
| Resources | scanrows-/scanbytes-/scratch exact/one-over, ownscratch cleanup/Replacementpreservation, maxinflight/Busy, gedroppter Future hält laufenden Permit, kein unbegrenzter Queue |

Reale redb-Hookpunkte im tatsächlichen Producer: vor Writeplan und **vor/nach
jeder tatsächlichen insert/remove** von Playerpayload, Entitypayload, Locator,
Chunkindex, Chunkeepoch, Playerheader, Entityheader, Receipt und beiden Meta-
Counters; außerdem direkt vor `commit` und nach erfolgreichem tatsächlichem
Commit/vor Reply. An jedem Returnederrorhook vollständiger Raw-Vorherzustand
inklusive Counter/Receipt; bei Postcommit-Lostreply vollständiger Nachherzustand
und identischer Retry. Ein eigenes bounded Backend-/Commitfehlerprobe benennt
`CommitUncertain` und klärt Originalreceipt; Workerpanic nach Commit zeigt
`ExecutionUncertain`, nicht sicheren Abort. Failing preflight/admission und
ambiguous commit sind unterschiedliche Testklassen.

Memoryprobe testet echten Preapplyabort, echte zusammenhängende Veröffentlichung
und Postpublish-Lostreply; keine behauptete fallible Memory-Rowrollback oder
Processcrash-Durability. Prozesskill/Powerloss-, Festplattenhardware- und echte
Paper-/Client-/Plugin-Differentialabnahme bleiben getrennt benannt.

Nach Umsetzung: alle Storage-Unit-/Integration-/Rustdoctests Debug **und**
Release, strict all-target Clippy `-D warnings`, owned fmt mit Rust1.99.0,
Edition2021, `-j2`, eigener Tasktarget. Keine Tests von anderen Crates als
unbegründete Regressionsausweitung und kein edition2024-Standalonefmt. Exaktes
SHA256-Freeze aller owned Files plus Brief; unabhängiges Fremdreview und
Rootintegration, keine Selfapproval und keine Gitmutation durch diesen Autor.

### 12. Konkreter Dateiplan und getrennte Folgetickets

Neue Producerdateien:

- `server/crates/ferrumc-storage/src/ownership.rs` — API/private Types/config/errors;
- `.../src/ownership/raw.rs` — fixed Headers/Receipts/CanonicalSHAgrammatik;
- `.../src/ownership/engine.rs` — bounded Read/CAS/Plan/Receiptrelationen;
- `.../src/memory/ownership.rs` — unified State-/Reader-/Apply-/Traitproducer;
- `.../src/redb_store/ownership.rs` — eine tatsächliche Writertransaction,
  Readadapter, Cutpoints, ungewisse Commitklassifikation;
- `.../src/redb_store/ownership_validation.rs` — eigener exact15 Format6scan;
- `.../src/ownership/tests.rs` und `.../tests/ownership_contract.rs` — unabhängiges
  Model, Hardfixtures, Backendprobes, real Cutpoints.

Bestehende owned Files nach STORAGE-ENTITY001.2-Freigabe:

- `src/lib.rs` — neue Exports;
- `src/generation.rs` — zusätzlicher explicit Mode;
- `src/memory.rs` — zusätzlicher unified State/config, Freshness, alte Player-/wire-
  Refusals; alte Modepfade unverändert;
- `src/redb_store.rs` — zusätzlicher opener/config und routing/Freshness/Refusals;
- `src/memory/uuid_entities.rs`, `src/redb_store/uuid_entities.rs` — konkrete
  Readonlyownership-Adapter und explizite Blindwriterrefusals;
- `src/store.rs` — auch inOwnership Pluginnamespace4096;
- `src/redb_store/uuid_validation.rs` — nur Visibility des bestehenden bounded
  Payloadrowhelpers; kein Format5scan-/Error-/Fingerprintfunktionswechsel;
- `README.md`, `INVARIANTS.md` — neuer separater Vertrag/Limit-/Uncertaintygrenzen;
- dieser DUR03brief — approved Design, gemessene Tests/Freeze, offene Consumer.

Keine Cargo-/Lock-, App-, Sim-, Items-, Core- oder anderen Docedits. Werden
zusätzliche Produktionfiles nötig, zuerst Rootbesitz-/Scopeabgleich.

Gesondert offen bleiben5→6-Offline-Migration mit ausdrücklich deklarierter
Player→Scopezuordnung für **jeden** bestehenden Player, auch bei genau einem
Descriptor; Crossdimensionscopewriter mit monotoner globaler Playerhistory;
Historycapacityerweiterung/Replayretirement; Container/Hopper/Itemmerge-/Mehr-
ressourcentransfers; DUR-04s Pending-/Reservations-/Ackproducer, DUR-05s
Appdriver und eigentliche Drop-/Pickup-/Autosaveaktivierung. Keine existente
Format5welt wird zum Test dieses neuen Modus still konvertiert.
