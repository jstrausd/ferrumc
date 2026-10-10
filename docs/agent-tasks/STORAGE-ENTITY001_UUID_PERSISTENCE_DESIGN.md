# STORAGE-ENTITY001 — Reviewentwurf für UUID-basierte Entitypersistenz

Stand: 10.10.2026. **Nur Design, noch keine Produktionsimplementierung.**
Root hat dieses Dokument exklusiv an `select_world_ticket` vergeben. Geplanter
Produktionsbesitz nach eigener Vergabe: ausschließlich `ferrumc-storage` und
dieses Dokument; App-/Sim-/Core-/World-/Registry-/Git-/Lockänderungen gehören
nicht in das Producerticket. Version 5 und die Namen unten sind ein konkreter
Vorschlag, den Root vor Umsetzung als Vertrag reserviert.

## Problem und Abgrenzung

Der heutige `EntityKey` enthält `(WorldId, DimensionId, EntityId)` und kodiert
genau zwölf Bytes. `EntityId` ist die signed-i32-Netzwerkreferenz und kann nach
Restart oder Unload eine andere Entity bezeichnen. `EntityRecord` ist dagegen
ein begrenzter, schema-markierter opaquer Payload. Die bestehende
Format-3/4-Migrationsvalidierung verlangt ausdrücklich zwölf Entitykeybytes.
Es gibt weder eine UUID-Persistenz-API noch eine Chunkenumeration.

Der geprüfte Coreproducer `EntityUuid` erhält alle 128 supplied Bits, darunter
nil und Nicht-v4-Werte. Er erzeugt keine UUID und garantiert keine Eindeutigkeit.
Dieses Ticket liefert dauerhafte Adressierung, einen begrenzten Chunklesepfad
und atomare Indexpflege. Es liefert keine Item-/Mobkomponenten, Entityphysik,
globale Wire-ID-Allocation, Trackingpakete oder Mengen-/Crashtransfers zwischen
Spielerinventar und Entity. Insbesondere ist `save_entities` heute nur innerhalb
der Entitytabelle atomar; ein separater `save_player` ist kein gemeinsamer Commit.

Die bestehende App speichert 46 Inventarslots plus Cursor in einem strikten
binären Schema-3-Payload. Die Inventarautorität ist gegenwärtig connection-owned
und speichert beim Disconnect separat. Ein späterer gemeinsamer Transfer braucht
eine eigene Inventarautorität und Writerumstellung; dieser Producer verdeckt die
fehlende Grundlage nicht.

## Referenz und zusätzliche FerrumC-Verträge

Minecraft Java 1.21.8 / Protokoll 772, Paper Build 60 / Commit
`29c8822d90899c89d2689338e81a98f690bcba12`.

Gelesen wurden vollständige angewandte `EntityStorage` und `PlayerDataStorage`,
vollständige `Entity.saveAsPassenger/save/saveWithoutId/setUUID/getUUID`, die
vollständigen Item-Entity-Save-/Load-/Merge-/Pickupmethoden und benötigten
`Util.safeReplaceFile/safeReplaceOrMoveFile`-Callees. Der vollständige
PlayerDataStorage-Sourcepatch und die vollständigen aktiven Moonrise-
`saveEntities`, `saveEntityChunk0` und `save`-Methoden samt relevantem Featurepatch
wurden berücksichtigt. Weitergehende ursprüngliche Entity-/Inventar-/Eventcallee-
Lektüre aus ITEMS-TRANSFER-001 und SIM-03.1 bleibt deren eigener Referenznachweis;
diese Designlektüre ist keine neue Live-Paper-Abnahme.

Paper speichert Entity-UUID, Position und eigene Payloads; Entityregionsdaten
liegen chunkweise und Spielerdateien UUID-weise separat. Moonrise plant
Entity- und Chunk-/POI-I/O separat und behandelt transiente Entitychunks beim
Unload eigens. Paper-Pickup führt Attempt-/Legacy-/Entityevents vor der
Inventarmutation aus, stellt Counts bei Cancellation wieder her und entfernt
die vollständig aufgenommene Item-Entity. Hieraus wird keine ursprüngliche
gemeinsame Spieler-/Entity-Disktransaction abgeleitet. FerrumCs spätere CAS-
und Receipttransaktion ist ein eigener Persistenzvertrag. Seine zusätzlichen
Bounds, strikten Fehler und internen Bytes sind keine Vanilla-NBT-Kompatibilität.

## Öffentliche API des Producertickets

Alle Typfelder bleiben privat; Konstruktoren/Getter erhalten Rustdoc. Keine
Konvertierung aus `EntityId` oder `PlayerId`, keine UUIDgeneration im Storage.

| Typ/API | Konkreter Vertrag |
| --- | --- |
| `PersistentEntityKey::new(WorldId, DimensionId, EntityUuid)` | Opaquer, kopierbarer scoped Key; Getter `world/dimension/uuid`. UUIDbits bleiben vollständig erhalten. |
| `PersistentEntityRecord::new(ChunkPos, EntityRecord)` | Checked Wrapper für den Storage-lokalen Chunkheader und den unverändert opaquen, höchstens 256-KiB-Payload. Chunkursprung und maximale lokale x/z-Offets müssen im Blockkoordinatendomän passen. Keine Height-/Mob-/NBTinterpretation. |
| `PersistentEntityWrite::new(key, expected_identity, record)` | Verknüpft jede Schreiboperation mit der exakt erwarteten WG-001-Identität. Der Backend prüft den Descriptor innerhalb derselben echten Writertransaction; ein vorhandener fremder Descriptor ist kein ausreichendes Admissionkriterium. |
| `PersistentEntityStore::load_persistent_entity(key, expected_identity)` | Ein Readsnapshot prüft Scope/Identität, Key-/Envelope-/Indexkonsistenz und gibt checked Record oder bestätigtes `None` zurück. Unsupported Mode und Korruption sind kein Miss. |
| `save_persistent_entity(write)` / `save_persistent_entities(Vec<write>)` | Gesamter Preflight vor Änderung; atomare Entity-/Locator-/Chunkindex-/Chunkepochpflege. Neue UUID oder Update im selben Scope zulässig. Dieselbe Welt-UUID in einer anderen Dimension wird verweigert. Keine Last-write-wins-Duplikate innerhalb eines Batches. |
| `delete_persistent_entity(key, expected_identity)` | Entfernt Record, Chunkindex und Locator atomar. Bestätigtes Fehlen ist `false` und erhöht keinen Chunkepoch. Widersprüchliche Indexreste sind Korruption, keine autorisierte Reparatur. |
| `move_persistent_entity(source, destination, expected_source, expected_destination, new_record)` | Eigene atomare Scopeverlagerung: UUID und WorldId identisch, Quelle vorhanden, Ziel frei, beide Descriptoren geprüft; alte und neue Chunkindizes/Locator werden gemeinsam geändert. Verschiedene Welten und mehrteilige Gameplay-/Inventartransfers bleiben Unsupported. |
| `load_persistent_entities_page(chunk_key, expected_identity, cursor, page_limit)` | Prefixrangescan; bounded Page mit checked Records, `ChunkEntityEpoch` und opaquem Cursor. Kein Scan sämtlicher Entities und kein unbounded Ergebnis-Vec. |

`PersistentEntityStore` ist ein getrenntes dyn-kompatibles async Trait. Es gibt
keine Raw-DB-/Transactionhandles zurück. Legacy-Instanzen beziehungsweise
nicht adoptierte Implementierungen geben `Unsupported` zurück; sie dürfen
neue APIaufrufe nicht als erfolgreiche Abwesenheit tarnen. Die alten
`WorldStore`-Entitymethoden bleiben in Format 3/4 unverändert. Im UUIDmodus
verweigern sie Reads und Writes mit `Unsupported`, damit keine wire-keyed
Parallelautorität entsteht. Bestehende Chunk-/Overlay-/Player-/Journal-/Plugin-
APIs bleiben in diesem Producerschritt unverändert; DUR-03 schränkt ihre
Ownershipwriter erst in seinem ausdrücklich aktivierten Modus ein.

## Format-5-Schema: genau zwölf Tabellen

Format 5 übernimmt die neun Format-4-Tabellen. Der Formatmarker ändert sich
explizit; `ferrumc:entity` erhält eine neue Key-/Envelopegrammatik. Drei neue
Tabellen bilden Locator, Chunkindex und den stabilen Paginierungsepoch.

| Tabelle | Key / Value in Format 5 |
| --- | --- |
| `ferrumc:entity` | Key genau 24 Bytes: `WorldId(u32 BE) ++ DimensionId(u32 BE) ++ EntityUuid(16 canonical bytes)`. Value `FCUE(4) ++ envelope_schema(u16 BE=1) ++ chunk_x(i32 BE) ++ chunk_z(i32 BE) ++ payload_schema(u32 BE) ++ payload_len(u32 BE) ++ payload`; exakt 22 Headerbytes plus höchstens 256 KiB. Keine wire-ID oder lokalen Handles. |
| `ferrumc:entity_locator` | Key genau 20 Bytes: `WorldId(u32 BE) ++ EntityUuid(16)`. Value genau zwölf Bytes: `DimensionId(u32 BE) ++ chunk_x(i32 BE) ++ chunk_z(i32 BE)`. Erzwingt höchstens einen lebenden Record derselben UUID je Welt über Dimensiongrenzen. Verschiedene Weltnamespaces dürfen dieselben UUIDbits enthalten. |
| `ferrumc:entity_chunk` | Key genau 32 Bytes: bestehender 16-Byte-`ChunkKey` ++ EntityUuid(16). Value exakt leer. Jeder Eintrag bezeichnet genau einen passenden Entityrecord; keine Bucket-Vecs oder Persistenz lokaler Simhandles. |
| `ferrumc:entity_chunk_epoch` | Key genau bestehender 16-Byte-`ChunkKey`. Value genau `u64 BE`, beginnend mit 1 nach der ersten tatsächlichen Änderung. Ein nie veränderter Chunk hat den konzeptionellen Epoch 0. Leere einst veränderte Chunks behalten ihre Epochrow; keine Rücksetzung oder Wraparoundaliasierung. |

Die übrigen acht Tabellen bewahren ihre bestehenden Bytegrammatiken. Für
`ferrumc:meta` werden keine neuen unbestimmten Metakey-Namen eingeführt. Der
exakte Catalog umfasst Meta, Chunk, Overlay, Entity, Player, Plugin,
Mutationlog, Mutationbatch, Generation, Locator, Chunkindex und Chunkepoch;
keine zusätzlichen Multimaptabellen. Tabellen haben die bekannten redb-
Key-/Valuetypen. Schema, Formatversion, opaquer Payloadschemawert und
Generatorrevision bleiben unabhängige Versionen.

Die Locatorgrenze ist eine bewusst explizite Welt-UUID-Policy. Sie liefert keine
prozessweite globale Wire-ID-Eindeutigkeit und keine vollständige
Crossworld-/Crossshard-Lifetime. Basisposition/Entityart und eine gegebenenfalls
im opaquen Body enthaltene UUID werden vom späteren Simcodec geprüft; Storage
behauptet nicht, den unbekannten Payloadinhalt validiert zu haben.

## Cursor, Bounds und deterministischer Fortschritt

- Savebatch: höchstens **64** Entitywrites und **4 MiB** gemeinsam kodierte
  Keys, Header und Payloads. Einzelpayload weiterhin höchstens **256 KiB**.
  Größenaddition checked; Count-/Byte-/Scope-/Duplikat-/Epochoverflowprüfung
  vor Backendänderung. Die 4096er Legacybatchgrenze wird nicht als ausreichende
  neue Heapgrenze übernommen.
- Page: expliziter `NonZeroU16`-Limitwert in **1..=64**, höchstens **1 MiB**
  gemeinsam kodierte zurückgegebene Records. Ein maximaler Einzelrecord passt
  immer in eine Page. Der Backend stoppt vor dem nächsten nicht passenden
  Record und liefert einen Fortschrittcursor; vollständiges Vorlesen aller
  weiteren Records ist verboten.
- Cursor enthält privat genau Backendinkarnation, Chunkkey, Expectedidentity,
  Chunkepoch und die zuletzt vollständig ausgegebene UUID. UUIDordnung ist
  kanonischer unsigned Bytevergleich, keine Java-UUIDsortierungszusage. Fremde
  Backends/Chunks/Identitäten, malformed Werte und ein geänderter Epoch werden
  vor einem Ergebnis als klassifizierter Cursorfehler verweigert. Cursor sind
  transient und werden weder persistiert noch als Minecraftpaket behandelt.
- Erste Page, Epoch, Indexrange und Entityrecords werden aus derselben
  Readtransaction gelesen. Jede Folgepage prüft ihren unveränderten Epoch in
  ihrem Readsnapshot. Die spätere Chunkloadreservation veröffentlicht erst
  nach vollständig erfolgreicher Pagination; ein Epochkonflikt erfordert
  bounded Retry/Backpressure, niemals partielles Admission.
- Jede tatsächliche Änderung eines Recordpayloads oder seiner Chunk-/Scope-
  Zuordnung erhöht die betroffenen Chunkeepochs in derselben Writertransaction.
  Identische Rawrecordwrites sind idempotente No-ops. U64overflow wird ohne
  Änderung als Capacity/Sequenceexhaustion klassifiziert. Epochrows werden
  nicht blind gelöscht; deren sichere GC braucht einen eigenen Cursorlifetime-
  beziehungsweise Formatvertrag.
- Recordparser prüfen Header, Gesamtlänge, Payloadlength und Chunkgrenzen vor
  kopierender Allocation; unbekannte Envelopes, trailing bytes und inkonsistente
  Indizes werden verweigert. Encoding arbeitet zunächst privat und verändert
  keinen Callerbuffer bei Fehler. Encodedlimits sind keine gleichgroßen
  Decodedheap-/RSSlimits.

## Backendtransaktion und Memoryparität

redb führt die vollständige Operation einschließlich Encoding/Decoding in
`spawn_blocking` aus. Eine Writertransaction prüft Mode/Catalog, jede exakte
Generationidentity, alle Existingrecord-/Indexbeziehungen, global je Welt die
UUIDlocator und sämtliche Epochincrements **vor** der Änderung. Erst danach
werden Records, beide Indizes und Epochrows aktualisiert und committed.
Ein Backendfehler oder Transactiondrop hinterlässt keine halbe Indexmutation.
Die Durability bleibt `Immediate`; redb 2.6.3 verwendet diese standardmäßig.
Queue-/Tickarbeit und Storage-I/O bleiben getrennt.

Der InMemory-UUIDmodus ist explizit. Seine Entityrecords, Locator, Chunkindex
und Epochrows liegen in **einem owned State unter einer gemeinsamen
Mutationgrenze**, gemeinsam mit der Generationadmission. Kein mehrstufiges
Insert in vier unabhängig freigegebenen Locks und kein Guard über `await`.
Nach abgeschlossenem Preflight wird die gesamte Änderung synchron publiziert.
Reads sehen konsistente Snapshots; Pagination ist bytegleich geordnet und
prüft denselben Epochvertrag. Kein Global-/DashMap-/Rawhandle.

Die Backends teilen dieselben Count-/Byte-/Schema-/Scope-/Duplikat-/Cursor-
Fehlerverträge. Memory speichert naturgemäß den ganzen Testbestand im RAM,
redb hält ihn auf Disk; der Producer behauptet keinen gleichen Gesamt-RSS.
Tests verwenden begrenzte Bestände und müssen zusätzlich Work/Allocation je
Page/Request messen. Produktionsresident-Entitycaps gehören SIM-03.2; dieser
Diskindex lädt keinen vollständigen weltweiten Entitybestand in den Prozess.

## Explizite Format-4 → 5-Migration

Normalopener: Format-3/4-Opener bleiben strikt für ihr bisheriges Layout;
`open_with_uuid_entities` öffnet ausschließlich Format 5 oder einen atomar
neu angelegten leeren Store. Ein vorhandenes Format 4 meldet einen konkreten
Migration-required-Fehler, niemals einen Upgrade-/Repairversuch. Format 3
benötigt zunächst den bereits implementierten WG-002-MIG-Schritt. Kein
automatischer kombinerter Sprung 3 → 5.

1. Eigener `prepare_uuid_entity_migration` hält den vorhandenen Source exklusiv
   gesperrt und validiert formatabhängig den kompletten exakten Catalog,
   Tabletypen, erlaubte Metakeys, alle Keys/Records/Descriptoren und bestehenden
   Journal-/Receipt-/Watermarkinvarianten. Zeilenweise, ohne World-Vec.
2. **Jede vorhandene wire-keyed Entityrow verweigert diesen ersten
   Migrationsvertrag.** Ein opaquer Payload beweist keine UUID oder Chunkposition.
   Kein Padding/Hashen der wire-ID, keine zufällige UUID, kein stilles Löschen
   oder Quarantinieren. Eine später ausdrücklich supplied und vollständig
   geprüfte Identitymappingmigration wäre ein eigenes Ticket.
3. Vor Änderung entsteht ein `create_new`-Backup des gesamten gültigen
   Format-4-Layouts. Alle bestehenden Rawkeys/-values, Descriptoren, Journal-
   und Receiptbytes bleiben bytegleich. Backup schließen, fsync, erneut komplett
   prüfen; Logicalhash und geschlossener Physicalhash getrennt berichten.
   Bestehenden Backup nur mit explizitem remembered Physicalhash und exakt
   passendem validiertem Logicalsnapshot wiederverwenden.
4. Nur ein backed-up Typestate erlaubt den Commit. Unmittelbar zuvor werden
   Source-/Backup-Pathidentität, gesperrter Sourcesnapshot und geschlossener
   Backuphash nochmals geprüft. Eine Writertransaction legt genau die drei
   leeren Indextabellen an und setzt den Formatmarker auf 5; vorhandene Rawrows
   werden nicht decodiert/reencodiert oder neu sortiert.
5. Ein Retry von Format 5 ist erfolgreich erst nach vollständigem
   format-5-spezifischem Catalog-/Row-/Index-/Descriptor-/Journal-/Receiptcheck.
   Die präparierte Migration akzeptiert kein neu auftauchendes Format-5-
   Entitygameplay als identischen Migrationssnapshot. Sourcefingerprint und
   Backupvergleich müssen die **Migrationdelta** (Marker und drei leere neue
   Tabellen) explizit erkennen; allgemeines „Version 5 genügt“ ist unzulässig.

Die vorhandene Generationmigration hat feste neuner Countarrays und explizite
12-Byte-Entitychecks. Sie wird **nicht** durch gelockerte Anycatalog-/Anykey-
Checks erweitert. Der neue Validator wählt klar Format 4 oder 5; neue Counter/
Fingerprints sind versioniert und berücksichtigen alle zwölf Tabellen. Die
ältere Prepare-API verweigert Format 5 weiterhin. Zusätzlich zur zweifachen
Entity↔Chunkindex↔Locator-Prüfung muss jede nichtleere Entitychunkzuordnung
einen positiven Epoch besitzen. Orphanrows, doppelte Welt-UUIDs, fremde Scopes
und widersprüchliche Valueheader werden klassifiziert verweigert.

Backup/Migrations-I/O bleibt offline/blocking, cachebegrenzt und übernimmt
WG-002-MIGs Fileidentity-/Lock-/Unix-/Recoveryguardvertrag. Zurückrollen bleibt
explizite Offlinewiederherstellung des verifizierten Format-4-Backups nach
Schließen sämtlicher Handles und separatem Erhalt der ersetzten Daten. Eine
abgeschlossene Migration beweist noch keine Powerloss- oder Itemconservation.

## Bestehender Journal-/Receiptvertrag bleibt erhalten

Mutationlog bleibt append-only mit storage-allokierten contiguous U64-IDs,
exaktem Watermark und unveränderten Recordbytes. `JournalBatchId` und dessen
16-Byte-Receiptgrammatik bleiben unverändert. Same-ID/same-normalized-payload
liefert den ursprünglichen Receipt; IDreuse mit anderen Bytes bleibt Fehler.
Emptyreceipts zählen als State. Journal- und Receiptwrites bleiben gemeinsam
atomar, Batches bounded und normalisierte ProvisionalIDs weiterhin von ihrer
Payloadidentität ausgenommen. Neue Entitywrites appendieren keine erfundenen
Blockmutationen und missbrauchen Journalreceipts nicht als Transferreceipts.

Generationfreshness berücksichtigt im UUIDmodus alle neuen Entities, Locator,
Chunkindex und Epochrows. Auch ein Epochrest eines inzwischen leeren Chunks
ist historischer Worldstate und berechtigt nicht zu einer frischen Baseline.
Migration übernimmt vorhandene Generationdescriptoren exakt und erzeugt
keinen historischen Seed aus Entity- oder Chunkdaten.

## DUR-03 als eigenes späteres Producerpaket

Nach diesem UUIDproducer braucht ein separates DUR-03-Unterticket eine API
aus **einem Backend-Arc**, die Playerrecord, Entityrecord/Deletion, Indizes,
persistierte Ownershipversionen und Transferreceipt in **einer** echten
Writertransaction CAS-prüft und committed. Vorschlag: neuer
`OwnershipTransferId`-Typ, private storage-vergebene monotone Revisionen,
`ExpectedAbsent/ExpectedRevision` und bounded `PlayerEntityTransferRequest`.
UUID und Revision sind keine wire-ID oder lokal generationalen Simhandles.

- Erste Consumergrenze bewusst ein Player und eine Entity im selben geprüften
  Scope; Drop, Partialpickup und Fullpickup passen darauf. Container-/Overlay-,
  Hopper-, Entitymerge-, Crossdimension-/Crossshardtransaktionen werden eigene
  Erweiterungen. Der allgemeine DUR-03-Katalog bleibt bis zu diesen Erweiterungen
  offen. Storagevalidierung begrenzter opaquer Snapshots ist kein semantischer
  Mengenbeweis; ITEMS-TRANSFER-001 und der Simconsumer prüfen Conservation.
- Player- und Entityrevisionen müssen auch nach Delete als Tombstone verbleiben,
  damit Delete/Recreate derselben Identität keine ABA-Aliasierung ermöglicht.
  Der Chunkeepoch dieses Producertickets ist ein Paginierungswert und **keine**
  Ownership-CASrevision. U64exhaustion verweigern, niemals wrappen.
- Receiptlookup vor live CAS prüfen: ein bereits committed Retry muss den
  ursprünglichen Receipt liefern, obwohl sich die live Revisionen bereits
  geändert haben. Andere Requestidentität für dieselbe Transfer-ID verweigern.
  Der konkrete Receiptidentitätsvertrag wird vor Umsetzung festgelegt:
  vollständige bounded Canonicalbytes erlauben exakten Vergleich, kosten jedoch
  bis zum Requestlimit Disk pro Transfer; ein konstanter SHA256-Receipt spart
  Platz, bietet kryptografische Identität statt mathematischer Bytegleichheit.
  Diese Wahl und sichere Retention/Replaygrenzen nicht stillschweigend raten.
- Im aktivierten Ownershipmodus dürfen blindes `save_player`, Legacyentity-
  save/delete und später auch Containeroverlay-Autosaves keinen Transfer
  überschreiben. **Jeder** betroffene Writer wird versionsgeführt umgestellt
  oder in diesem Modus Unsupported. Ein bloßes Revisionincrement beim Blindsave
  verhindert das spätere Überschreiben des bereits committed Transfers nicht.
- Playerkeys/Payloads sind heute unscoped. Neue Ownershipversionheader müssen
  deren aktuellen Scope ausdrücklich führen. Migration bestehender Spieler
  benötigt einen expliziten vollständigen Scopevertrag; mehrere Descriptoren
  erlauben keine stillschweigende Scopewahl. Format 5 wird dafür nicht nebenbei
  nachträglich um neue Tabellen/Grammatiken ergänzt: eigenes versioniertes
  Format-/Migrationspaket nach reserviertem DUR-03-Schema.
- Sim-DUR-04 braucht exklusiven Player-/Entityauthority, Reservationen und
  bounded Pendingtransfers. Queuefull erhält Originalzustand; Lostreply hält
  die Reservation bis Receiptklärung. Disconnect, Unload und Shutdown dürfen
  keine konkurrierende Autosave-/Releaseauthority erzeugen. Cancellation und
  erneute Pluginmutationen werden vor dem finalen Plan/CAS berücksichtigt;
  Retries führen Pluginereignisse nicht nochmals aus.
- App-DUR-05 verwendet den tatsächlich gleichen Backend und erhält Dirty-/
  Pendingzustand bis Ack. Erst nach bestätigt durable Commit werden neue
  Spieler-/Entitysicht und Take-/Removeoutputs publiziert. Der aktuelle
  connection-owned Inventar-/Leave-save-Pfad muss dafür vollständig ersetzt
  werden. Java-synchrone Mutation-/Eventreentrancy ist ein eigener Bridge-
  Consumervertrag; das Storageproducerticket behauptet ihn nicht.

## Review- und Abnahmepaket vor Integration

| Paket | Konkrete unabhängige Abnahme |
| --- | --- |
| Typed API/Codec | UUIDnil/extrema/Nicht-v4 unverändert; Compile-fail für wire-/Player-ID; exakte 24/20/32/16-Byte-Keys, 22-Byte-Header, Endian-Goldens, malformed Länge/Magic/Schema/Chunk/Payload/trailing bytes; 256-KiB-Grenze und Callerbuffer unverändert bei Encodingfehler. |
| Writes/Index | Spawn/update/delete/no-op/move, negative Chunkgrenzen, mehrere UUIDs je Chunk; fremde Dimensionduplikate und verschiedene Weltnamespaces; komplette Index-/Epochsnapshots unverändert bei fehlendem/mismatched Descriptor, Duplicatebatch, falscher Quelle, Kapazitäts-/Byte-/Epochoverflow und korrupten Existingrows. |
| Pagination | Leere/mehrseitige Chunks, 1/64 Limit, exakt 1 MiB und nächster nicht passender Record; kein Verlust/Duplikat bei statischem Bestand; falsche Scope-/Backend-/Identitycursor und Change-between-pages; Delete-last/Recreate erzeugt keinen alten gültigen Epoch. Keine Fullworldenumeration. |
| Backendparität | Dieselben öffentlichen Contracts gegen Memory und redb; parallele Reader sehen komplette Vor-/Nachzustände. Transactionabort an jedem Indexwritecutpoint und Lostreply nach Commit; Reopen stellt Record/Locator/Chunkindex/Epoch exakt wieder her. Keine Tokio-Workerblockierung. |
| Migration | Format4 mit jeder vorhandenen Kategorie/Schema/rawopaque Payload, leere Entitytabelle; exact Catalog/Rawrowhashes vor/nach; Prepare-only, Backup, Commit, checked Retry, Reopen; vorhandene Legacywire-Entityrow, wrong Catalog/type/key/descriptor, Lock/Pathreplacement, dirty redb, bestehender Backup/wronghash, IO-/Commitfehler und unbekannte Format5-Katalogdelta bewahren Source/Backup. |
| Journalregression | Alle bisherigen Journalsequence-/normalization-/receipt-/retry-/emptybatch-/generationfreshness- und vollständigen WG-002-MIG-Tests unverändert grün. Neue Format5-Statekategorien verhindern frische Descriptorinitialisierung. |
| Werkzeuge/Evidenz | Ganze Storagecrate, strict all-target Clippy und owned fmt/diff; exaktes SHA256-Freeze aller geänderten Dateien, Build-/Testlogs, erwartete Source-/Eventabweichungen. Fremdreview gegen genau den Freeze. Root integriert Git. |

Producer-Transactionabort/Reopen/Lostreplytests erlauben keine pauschale
Powerlosszusage. Spätere echte Prozesskill-/Diskfull-/Recovery-/Clienttests
werden am DUR-03/04/05-Consumer mit independently bilanzierter Itemmenge und
Komponentenbytes ausgeführt. Dieses Producerticket allein macht Survival,
Mobs und Plugininventare noch nicht vollständig spielbar.
