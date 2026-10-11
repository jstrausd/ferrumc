# DUR-03.2 — explizite Bestandsmigration in die Format6-Ownershipauthority

Status: Produktionscode freigegeben und implementiert; finale Authorprüfungen
bestanden und Lieferung eingefroren; unabhängige Rootabnahme steht aus. Voraussetzung vor produktiver
DUR-05-Adoption vorhandener Daten. Der Autor besitzt ausschließlich
`server/crates/ferrumc-storage/**` und diesen Brief. App-/Sim-/Config-/Items-
Änderungen, vorhandene generierte Daten und Git bleiben ausgeschlossen.

## Tatsächliche Lücke

Die produktive App startet Format4. Der geprüfte UUID-Entityproducer besitzt
Format5, der Player-/Entity-Ownershipproducer benötigt Format6.
`RedbStore::open_with_player_entity_ownership` verweigert Format3..5 ausdrücklich.
Die vorhandene UUID-Migration4→5 bewahrt Player-/Journal-/Chunkbytes, erzeugt
aber keine Player-Scopezuordnung, Ownershipheader oder Ownershipreceipts.
Ein Formatmarkerwechsel oder ein leerer neuer Store würde vorhandene
Inventare, Entities und ihre dauerhafte Autorität nicht korrekt übernehmen.

## Konkreter geforderter Producer

Vor Runtime einen vollständigen bounded API-/Datei-/Row-/Commit-/Resumeentwurf
liefern. Ein enger erster Producer kann ausschließlich gültiges Format5→6
besitzen; Format4 erreicht Format5 über den bereits geprüften eigenen
Offlineproducer. Kein automatisches Upgrade oder Mehrformat-Sammelumbau.

Jeder vorhandene PlayerUUID braucht eine ausdrückliche unveränderliche
GenerationKey-Zuordnung aus einer begrenzten Mappingquelle. Der UUID-keyed
Playerrecord allein enthält diese welt-/dimensionsgebundene Authority nicht.
Missing-, doppelte-, fremde-, widersprüchliche- und ungenutzte Zuordnungen
erhalten typisierte Regeln; keine stillschweigende globale Defaultdimension.
Die zugehörige vollständige GenerationIdentity muss tatsächlich persistiert
sein. Bereits UUID-identifizierte Entities haben ihren tatsächlichen Scope
und Chunk aus den gültigen Format5-Record-/Locator-/Indexdaten.

Das vollständige Format6-Ergebnis braucht seine exakte15-Table-Grammatik,
persistierte Capacityagreement, Player-/Entityheader, Revision-/Receiptcounts
und passende echte historische Bootstrapbelege. Die vorhandene Grammar
verknüpft Header mit Receipts; freie erfundene Revision1header ohne diesen
Zusammenhang sind kein zulässiges Ergebnis. Ein Migrationreceipt muss seine
aus dem eingefrorenen Sourcepayload hergeleitete Requestidentity, Scope,
Generation, Resourcekeys, before/after Stamps und Epochregeln exakt belegen.
Solche Offlinebootstrapbelege sind von einem bereits laufenden Simtransfer
zu unterscheiden. Eine neue Receiptkind-/On-disk-Grammatikänderung benötigt
vorher einen eigenen begründeten Vertrag und unabhängige Abnahme.

Playerpayload-/GameMode-/Schema- und vollständige FCUE/Entitybytes bleiben
verlustfrei; Chunks/Overlays, Generatoridentitäten, Plugins, Journalreceipts
und deren Max-ID behalten ihren geprüften Inhalt. Die Storage-Crate besitzt
keine FCPS1/2→3-Gameplaymigration; deren konkreter nachfolgender Appconsumer
muss native Payloadadmission vor Liveplayerzulassung übernehmen. Nicht
unterstützte Bestände werden vor Commit typisiert verweigert.

## Offlineauthority und Recovery

Die bereits unabhängig geprüften4→5-Verträge dienen als konkrete Grundlage:
exklusive Quelle, Pfad-/Inode-/Alias-/Symlinkprüfungen, echte closed Backupcopy,
unveränderlicher Checkpoint vor Sourcecommit, explizite positive Scratchcaps,
vollständige bounded Sourcevalidation, atomarer Gesamtdelta und exakte
postcommit/resume Validation. Keine In-place-Handedits, automatisches
Reparieren fremder Daten oder Ersetzen/Überschreiben vorhandener Evidenz.

Alle Mapping-/Player-/Entity-/Bootstrap-/Receipt-/Revision-/Bytecounts werden
vor Allocation und Commit gegen konkrete positive Operatorlimits geprüft.
Mapping und Quellenfingerprint sind im Checkpoint gebunden. Kein vollständiger
unbounded Payloadclonebestand und kein DB-I/O auf dem Simulationthread.
CommitUncertain bleibt ungewiss bis zum tatsächlichen readonly Resumecheck;
ein Fehler nach erfolgreichem Commit darf kein erneutes Bootstrap auslösen.

## Meaningful Abnahme und Verbraucher

Reale redb-Prepare/Commit/Reopen/Resumefälle, echte Sourcebytes vor/nach und
unabhängige vollständige Format6-Open-/Ownershipreads. Nichtleere Player und
Entities mit Komponenten, verschiedene Scopes/Generationen, vorhandene
Chunkepochs/Journal-/Plugin-/Overlaydaten und alle Mappingfehlformen.
Cap N/N+1, Header-/Receipt-/Count-/Indexfehlformen, source/mapping drift,
Backup-/Checkpointänderung, fremde/resumed/bereits migrierte Dateien,
Cutpoints vor jedem Write und vor/nach tatsächlichem Commit. Der genaue
zugelassene Delta muss independently herleitbar bleiben.

Whole-Storage Debug/Release, strict All-target-Clippy/Fmt/Rustdoc, exakter
Freeze und unabhängiger Review. App-CLI/Startflag/Driver erhält danach sein
eigenes App-Ticket. Dieses Storage-Ticket allein aktiviert weder Drop/Pickup
noch vollständiges Survival, Paperplugins oder Nether/End/Dragon.

## Konkreter Entscheidungsentwurf A — bestehende echte Singles-CASauthority

Historischer Designstatus vor Freigabe: A samt Epochdelta und atomarer
Offlineimportauthority intern gewählt, damals noch ohne Codefreigabe. Dieser
Abschnitt präzisiert den später freigegebenen Pfad;
Alternative B darunter bleibt ein separates
Grammatikprojekt. Die vorhandene Format6-Grammatik bleibt in A unverändert:
FCTR1 kind2 Playeronly, kind3 Entityonly, Fresh→Live Revision1 durch tatsächliche
CASausführung; kein frei konstruiertes Header-/Receiptbootstrap.

### Tatsächlich geprüfte Ausgangskette

- `ownership/raw.rs::player_key` verwendet weiterhin16UUIDbytes. Scope ist im
  FCOP39BHeader, nicht in einer heimlich geänderten Playerrowkeygrammatik.
  FCOR35B bindet Dimension, World/UUID20B bleibt im Headerkey.
- `ownership/raw.rs::request_identity` bindet actual kind, reale16B ID, Scope,
  vollständige27B GenerationIdentity, ResourceUUIDs, before Stamps, Schema/
  GameMode und vollständige native Proposalbytes. Die Owner-Arc ist lokale
  Laufzeitauthority und wird zu Recht nicht als dauerhafte DBidentity kopiert.
- `ownership/engine.rs::prepare`/`prepare_player`/`prepare_entity` und
  `persistent_entity::engine` erzeugen selbst Header, Revision, Query und
  Receipt. Ein Fresh Entityput ist Changed: die echte Chunkepoch erhöht sich
  um1; `raw::receipt_epochs` verlangt dieselbe reale betroffene Chunkunion.
- `redb_store/ownership.rs::apply` schreibt diese echten Plans im jeweiligen
  realen writerTransaction. `ownership_validation` prüft alle15Tabellen und
  Header↔Receipt↔Row-/Index-/Scopebeziehungen. Freie revision1Header im
  ursprünglichen Format5 würden diese Authority nicht ersetzen.

### Expliziter Delta und Epochentscheidung vor Code

A übernimmt jeden originalen Player- und Entityrecord bytegenau, einschließlich
GameMode, payloadschema, FCUEenvelope und opaque Komponenten. Chunk-/Overlay-,
Generator-, Plugin-, Journal-/Journalreceiptbytes und deren watermark bleiben
exakt. Entityrow/Locator-/Indexbytes bleiben ebenfalls exakt; kein neuer wireID,
UUID, Chunk oder Dimension entsteht.

**Die historischen Chunkepochwerte bleiben in A ausdrücklich nicht unverändert.**
Sei `n(c)` die tatsächliche Anzahl live Format5-Entities in Chunk c. Der echte
Freshbootstrap erhöht dessen vorhandene positive Epoch exakt auf
`source_epoch(c) + n(c)`. Leerchunkhistorie ohne Entity erhält0Delta. Es entsteht
kein neues Gameplayitem und kein Move; die Epochfortschritte belegen nur die
neue, echte Ownership-CASadmission. Jedes einzelne Entityreceipt bindet seinen
tatsächlichen Zwischenwert vorher→nachher, sortierte Sourcekeyreihenfolge macht
auch diese Zuordnung deterministisch und reviewbar.

Alle Chunkcounts und deren checked Gesamtsummen werden in bounded privater
Diskscratch ermittelt **vor** Stage/CAS/Commit. MAX oder Overflow verweigert A
mit typisiertem `EpochExhausted(chunk,source,count)`, bevor irgendein Source-
oder Stagebootstrap ausgeführt wird. Kein wrapping, Reset, leeres Historytable
oder anders deklarierter No-op. Dieser ausdrücklich deklarierte Epochdelta
ersetzt für A jede Interpretation früherer Brieftexte als Epochbytekonstanz;
Root hat diesen konkreten Delta intern freigegeben. B wäre erforderlich, falls Epochkonstanz
zum verbindlichen Ziel wird.

### Unveränderliche Inputs und konkrete IDs

Eine versionierte bounded Mappingdatei besteht aus einer festen Magic/Schema,
einem checked Count und strikt nach PlayerUUID sortierten fixed-width Rows:
PlayerUUID16 + GenerationKey8 + vollständige GenerationIdentity27. Jede
SourceplayerUUID erhält genau eine Row; missing/duplicate/unsorted/unused Rows
verweigern den gesamten Entwurf. Mappingidentität muss exakt mit der bestehenden
Sourcegeneration übereinstimmen; weder Defaultscope noch Seedinferenz. Gleiches
UUID in mehreren Scopes erhält keine zweite Playerauthority. Die tatsächliche
Entityscope/Generation kommt ausschließlich aus dem gültigen SourceUUIDrecord
und Sourcegenerationdescriptor, nicht aus dem Playermapping.

Der private Importplan enthält pro tatsächlichem Player/Entity genau eine
wirkliche `OwnershipTransferId` aus bestehender UUIDv4authority, vor jedem CAS
bereits diskgebunden. Keine UUID-Namespace-/Countermischung behauptet globale
Eindeutigkeit. Die Migration prüft alle IDs gegeneinander in bounded Diskindex;
Kollision verweigert Vorbereitung, Resume erzeugt niemals neue IDs. Ein
immutable Planrow bindet Resourcekind, vollständigen Sourcekey, tatsächliche
Scope/Generation, UUIDv4 ID und den später durch echten Requestproducer
hergeleiteten Queryhash. Planfingerprint bindet Sourcefingerprint, Mapping-
fingerprint, Capacityagreement, Reihenfolge, Counts, Targetgrammar und alle IDs.

Mapping- und Plandatei werden wie Source/Backup über echte offene Fileidentity
geprüft, gehasht und an den create_new-Checkpoint gebunden. Pfadreplacement,
Hardlinkalias, Fremddatei, Symlink oder neue Mappingbytes erlaubt kein Resume
gegen andere Daten. Source-/Mappingrawhash ist eine Evidenz; keine darin
enthaltene Zeichenkette darf einen arbitrary Filesystempfad autorisieren.

### Echte private Stagingauthority und atomarer Sourcecommit

1. Source wird mit denselben Unix-/redb2.6.3-Header-/Recovery-/Inode-/Lockregeln
   wie4→5 ausschließlich offline geöffnet und bleibt bis Commit/Drop exklusiv.
   Die komplette gültige12-Table-Format5-Grammatik wird bounded gestreamt, nicht
   als vollständiger Payloadvektor im Heap gehalten. Stage, Backup, Plan und
   Checkpoint sind separate create_new Dateien mit eigener geschützter Identity.
2. Ein geschlossener fsynced Backup behält exakt Sourceformat5 und alle originalen
   logischen Rows; vollständig bounded Copy-onlyvalidation und closedhashcheck
   erfolgen vor Sourcecommit. Das Originalbackup wird nicht redbgeöffnet.
   Quelle bleibt logisch unverändert. Die bekannten physischen redb allocator-
   Header-/Recoverybytes sind ausdrücklich keine unverändert-Hashzusage.
3. Eine private disposable echte Format6-redb Stage besitzt das tatsächliche
   Capacityagreement und dieselben Generationdescriptor-/Commonrows inklusive
   originaler historischer Epochs. Players/Entities/Locator-/Index-/Ownerheader/
   Ownershipreceipts sind zunächst leer. Ein einzelner geprüfter Stageaufbau-
   Transaction richtet diesen vollständigen Ausgangszustand ein. Stage besitzt
   keine App-/Sim-/Driver- oder publikationsfähige Livecapability.
4. Pro Sourceplayer wird ein echtes Fresh OwnedPlayerSnapshot aus dieser Stage
   gelesen; sein originales Schema/GameMode/Payload wird über den existierenden
   realen PlayerCASproducer geschrieben. Pro Sourceentity folgt echter Stage-
   Freshread plus existierender EntityCASproducer mit originalem Key/FCUErecord.
   Beide benutzen denselben privaten realen Stageowner/Runtime und tatsächliche
   Requests, Plans, writerTransactions und Outcomes. Keine freie Receipt-/Stamp-
   Konstruktion; jeder Outcome muss wirklich Committed sein. Alle Stagecalls sind
   offline außerhalb Simulation/Asyncworker, seriell bounded, ohne zweiten Writer.
5. Stage wird geschlossen, erneut mit unverändertem Ownershipconfig über den
   vollständigen Format6-Opener validiert und mit Source/Plan verglichen: exakt
   alle originalen Common-/Player-/Entity-/Locator-/Indexrows, einzig deklarierter
   Epochcountdelta, je tatsächlich1Header+1genuine Receipt pro importiertem
   Resource, Revisioncount=Receiptcount=Players+Entities. Requestquery, Resource,
   Scope/Generation, Nativepostimage und Zwischenepochs müssen Plan/Source exakt
   entsprechen. FCPS1/2 werden dabei weder interpretiert noch nachFCPS3 geändert.
   Danach echte StageDB schließen, Stagefile fsync und finalen cleanheader/
   identity/physicalhash prüfen; dieser final reclosed Stand wird versiegelt.
6. Der create_new fsynced Checkpoint bindet Source12-Table-fingerprint, Source-
   identity, closed Backuphash, Mapping-/Importplanhash, actual fully validated
   Stage15-Table-targetfingerprint und alle Limits. Stage ist unter eigener
   Fileidentity exklusiv und immutable; Parentdirectoryfsync vor Sourcecommit.
   Von jetzt an werden Stage/Backup ausschließlich als Fileevidence behandelt.
   Die importierbaren Rows kommen aus einer unten beschriebenen checked Copy.
7. Genau **eine** echte SourcewriterTransaction revalidiert Sourcefingerprint/
   Backup-/Mapping-/Plan-/Stageidentity. Sie schreibt ausschließlich die drei
   Ownerheader-/Receipt-Tabellen, tatsächlichen Ownershipcapacity/countmeta,
   deklarierten Epochwerte und Formatmarker6. Originalpayloads/-locator/-index/
   Journal werden nicht neu encodiert. Vollständiger Format6validator und exakter
   erwarteter Targetfingerprint laufen noch im uncommitted Transaction; erst
   danach ein actual durable Commit. Fehler davor abortiert alles atomar.
8. Nach Commit folgt echte close/reopen/readonly vollständige Targetvalidation.
   Postcommitfehler sind `CommitUncertain`; Resume prüft realen Zustand, statt
   Bootstrap zu wiederholen oder Backup automatisch zurückzuschreiben. Resultate
   dürfen erst nach belegtem committed Target übergeben werden.

Die Sourcecommitphase kopiert echte Stageheader/Receipts und Epochwerte, aber
behauptet keine nie ausgeführte Source-LiveCAS. Diese Receipts sind genuine
Offlinebootstrap-CASbelege aus der privaten Stage und werden mit ihren vollständigen
immutable Source-/Mapping-/Plan-/Targetbelegen als Gesamtdelta atomar importiert.
Root hat diese konkrete Trustboundary intern gewählt; die Runtimefreigabe folgt
erst nach der endgültigen API-/Datei-/Capreview. Eine zusätzliche Source-only-
CASbedingung gilt für A nicht.

### Positive Caps und Dateien

Vorgeschlagene positive Operatorlimits, vor Code zu pinnen:

| Ressource | Default / Hardmaximum | Vorher geprüfte Einheit |
|---|---:|---|
| Mappingrows |65536 /1048576|actual51B Rows + fixed64B envelope|
| Importplanrows |131072 /1048576|actual checked fixed rowshape und metadata|
| Gesamte Source-/Targetscanrows |1048576 /16777216|alle logischen Tabellenrows|
| Source-/Stage-/Backuplogicalbytes |512MiB /8GiB je Datei|checked key+valuebytes, ohne redbpages|
| Disposable Plan-/Chunkcountscratch |64MiB /512MiB zusammen|actual raw Indexrows+fixedmetadata|
| Einzelne physische Verificationcopy |2GiB /32GiB|actual Dateiumfang einschließlich redbhousekeeping, vor Copy/growth|
| Einzelne copied Proposal |existing Ownershiprequestcap|checked size, vor Payloadclone|
| Ownershiphistory |Operator OwnershipCapacity|Players+Entities gegen Receipts/Revisions|

Stage/Backup/source haben jeweils64MiB redbCache wie geprüfte Offlineproducer;
Diskindex16MiB. Keine Behauptung, die Tabelle begrenze RSS oder physischen
Dateiumfang. Auch bei1MRows liegt jeweils nur eine vorhandene bounded Row plus
checked Request/Plan im Heap. ScratchlimitN/+1 muss vor ID-/Countindexinsert
refusen. Die persistierte Capacity bleibt actual Operatoragreement; Migration
verbraucht echte Historyslots. Fehlen Runtimeheadroomslots, weist Report das
vor Commit ausdrücklich aus; kein verstecktes Überbuchen oder Historypurge.

### Resume und Meaningful Gegenproben

Resume akzeptiert ausschließlich den bestehenden immutable Checkpoint und
vorhandene exact Backup/Mapping/Plan/Stage. Source ist entweder vollständig
ursprünglicher Format5-fingerprint oder vollständig erwarteter Format6-target.
Format5 kann nach vollständigen gleichen Checks exakt denselben Sourcecommit
wiederholen; Format6 meldet nur nach exact Targetvalidation AlreadyCommitted.
Jeder andere Marker/Katalog, partialdelta oder fingerprint drift verweigert
unverändert. Fehlende/ersetzte fremde Stage wird nicht als leere DB recreated.

Unabhängige Proben: mindestens2GenerationScopes, nonempty Playerpayloads mit
verschiedenem GameMode/Schema und opaque FCPS1/2, mehrere Entities im selben
Chunk und gleichen UUIDbits in unterschiedlichen Worlds, historische Leerchunk-
epoch, Journalrange/Receipt/watermark und Plugin-/Overlaybytes. Echte private
StageCASreceipts und Sourcecommit separat nachweisen. Wrongmapping/IDduplicate,
Epoch MAX/MAX-n/N+1, row-/byte-/request-/historycap N/N+1; Source/Mapping/Backup/
Checkpoint/Stage-/Planreplacement; Cutpoints vor/nach jedem Write, tatsächlicher
Commit und reopen. Erwarteter15-Table-Target ist unabhängig vom Migrationwriter
herzuleiten, keine lediglich self-roundtrip Abnahme. WholeStorageChecks folgen
auf echte Umsetzung; App-CLI und FCPSgameplaymigration bleiben eigene Tickets.

## Alternative B — neue explizite Importreceiptgrammar

Wenn exakte historische Epochbytekonstanz oder Source-only Receiptentstehung
verbindlich wird, kann die bestehende FCTR1 Changedbootstrapgrammar nicht als
No-op umgedeutet werden. B braucht vor Implementation einen eigenen, unabhängig
geprüften On-disk-Vertrag: spezifische offline Player-/Entityimportkinds, passende
versionierte Receiptgrammar, genau gebundene Source-/Mapping-/Payloadrequest-
identity und Fresh→Live1importheader ohne erfundene Epoch+1. Der alte Runtime-
CASproducer bleibt mit unveränderten FCTR1kinds1..3 lesbar und deterministisch;
ein älterer Opener verweigert neue Receiptgrammar ausdrücklich.

B ist kein optionaler bool in A, kein marker6-only Upgrade und keine heimliche
Änderung vorhandener Receiptsemantik. Payload-, Backup-, Cap-, Checkpoint- und
atomic-sourcecommitcontracts bleiben gleich streng. Wegen der zusätzlichen
Runtimegrammar-/Compatibilityprüfung wird A empfohlen, falls dessen konkrete
Epoch-/Stageimportauthority Root freigibt. Keine neue Nutzerfreigabe ist für
diese normale interne Implementierungswahl erforderlich; Code beginnt erst
nach konkretem Rootreview des gewählten Storagevertrags.

## Endpräzisierung — öffentliche API, actual Bytegrammatik und Dateibesitz

Root hat A einschließlich `source_epoch(c)+n(c)` und echter Stage-CAS-/atomarer
Offlineimportauthority intern gewählt. Die folgenden Shapes sind ein konkreter
letzter Entwurf vor Runtimefreigabe. B wird nicht gemeinsam implementiert.

### Öffentliche Storage-only API und konkrete Refusals

Vorgeschlagene eigene nicht clonbare Typen ohne öffentliche Felder:

- `OwnershipMigrationLimits::new(NonZeroU64 scan_rows, scan_bytes,
  mapping_rows, plan_rows, scratch_bytes, verification_copy_bytes,
  NonZeroUsize checkpoint_bytes)` prüft obige Hardcaps sowie alle
  `usize`-/checked arithmetic Konversionen. `verification_copy_bytes` begrenzt
  zusätzlich jede neue tatsächliche Stage-/Backupdatei beim Wachstum;
  `3*verification_copy_bytes + scratch_bytes + checkpoint_bytes` muss checked
  darstellbar sein. Höchstens eine Disposableverificationcopy ist gleichzeitig
  offen. Logische Scan-/Scratchbytes ersetzen diesen physischen Cap nicht.
  `OwnershipConfig` bleibt der vorhandene tatsächliche Format6-Vertrag.
- `RedbStore::prepare_player_entity_ownership_migration(source: &Path,
  mapping: &Path, config: OwnershipConfig, limits: OwnershipMigrationLimits)`
  liefert `PreparedOwnershipMigration`. Es besitzt SourceDB/-Filelock,
  Mappingfile/-lock und bounded Count-/IDindex; kein StageCAS zu diesem Zeitpunkt.
  Report/Source-/Mapping-/Capacityfingerprints sind readonly getter.
- `PreparedOwnershipMigration::prepare_files(self, paths:
  OwnershipMigrationOutputPaths)` liefert `BoundedOwnershipMigration` nach
  vollständiger echter Backup-, Stage-, Importplan- und Checkpointerzeugung.
  `OutputPaths::new(backup, stage, plan, checkpoint, scratch_directory)` enthält
  ausschließlich tatsächlich vom Caller gewählte Pfade, keine implicit Temp-
  oder Checkpointpfade. Alle neuen Dateien create_new, alle Aliasprüfungen vorab.
- `BoundedOwnershipMigration::migrate(self)` führt genau einen Sourcecommit
  samt postcommit Validation aus und liefert `OwnershipMigrationOutcome`.
  Kein Clone/Serialize/Callback vermittelt Commitauthority. Der Outcome bietet
  exact source/target logical hashes, Rows/Bytes/Receipts/Revisions und actual
  Epochdelta, niemals einen offenen Live-Store oder `OwnedExpectation`.
- `RedbStore::resume_player_entity_ownership_migration(inputs:
  OwnershipMigrationResumePaths, config, limits)` nimmt **alle** Callerpfade
  Source/Mapping/Backup/Stage/Plan/Checkpoint explizit. Checkpointbytepfade werden
  nur mit diesen bereits autorisierten canonical Pfaden verglichen, niemals als
  neue Filesystemauthority verfolgt. Result ist entweder eine neu tatsächlich
  gelockte `ReadyToCommit(BoundedOwnershipMigration)` auf exact Format5 oder
  `AlreadyCommitted(OwnershipMigrationReport)` auf exact Format6.

Spezifischer `OwnershipMigrationError`, kein generisches successful fallback:
`InvalidLimit`, `UnsupportedPlatform`, `SourceBusy`, `UnsupportedSourceFormat`,
`RecoveryRequired`, `CatalogMismatch`, `MalformedSourceRow(table,keykind)`,
`MappingMissingPlayer(uuid)`, `MappingUnusedPlayer(uuid)`, `MappingDuplicate`,
`MappingUnsorted`, `GenerationMismatch(scope)`, `DuplicateImportId`,
`EpochExhausted(chunk,source,count)`, `ScanCapacity(kind,required,limit)`,
`PlanCapacity`, `ScratchCapacity`, `VerificationCopyCapacity`,
`CheckpointCapacity`, `HistoryCapacity`,
`RequestCapacity`, `PathAlias`, `SymlinkRefused`, `FileReplaced(role)`,
`EvidenceExists(role)`, `MalformedMapping`, `MalformedPlan`,
`MalformedCheckpoint`, `SourceDrift`, `EvidenceDrift(role)`,
`StageCasFailure(OwnershipError)`, `StageOutcomeMismatch`,
`TargetDeltaMismatch`, `Io(operation,role,source)`, `Backend(operation,cause)`
und `CommitUncertain` mit tatsächlich vorhandenem Query-/Checkpointreport.
Errorextents/identities sind begrenzte typed scalars; kein Clone kompletter
ungültiger Payloads in Fehlerwerten. Cleanupfehler meldet die konkrete eigene
Fileidentity und entfernt niemals eine am Pfad ersetzte fremde Datei.

### Exact Mappingdatei — FCOMapping1

Alle Integer big endian. Header genau64Bytes: Magic4 `FCOM`, schema u16=1,
reserved u16=0, row_count u64, danach48zero reserved. Rows genau51Bytes:
PlayerUUID16, GenerationKey8, GenerationIdentity27. UUID aufsteigend strikt,
kein trailingbyte, `64 + count*51` checked vor Allocation/Read. Der eigene
Context unterscheidet diesen Typ von der Checkpointmagic; seine Headerlength/
Dateirolle wird ausdrücklich geprüft. Empty Mapping ist genau dann zulässig,
wenn Sourceplayers0 ist; Ressourcencaps bleiben trotzdem positiv.

### Exact immutable Importplan — FCOPlan1

Header genau128Bytes: Magic4 `FCIP`, schema u16=1, reserved u16=0,
row_count u64, SourceFormat5logicalSHA32, MappingfileSHA32,
OwnershipCapacitySHA32, Playercount u64, Entitycount u64. CapacitySHA ist
SHA256 über das exact Literal `FerrumC ownership migration capacity v1\0`
(mit echtem finalem NULbyte), danach die vorhandenen drei exact big-endian u64
Request-/Receipt-/Revisionlimits in dieser Reihenfolge.

Jede Planrow ist genau148Bytes:

| Offset | Bytes | Tatsächlicher Inhalt |
|---:|---:|---|
|0|1|bestehender Requestkind2=Player,3=Entity|
|1|24|Sourcekey; PlayerUUID16+8zero, Entitykey24 exakt|
|25|8|GenerationKey|
|33|27|vollständige GenerationIdentity|
|60|16|wirkliche recorded UUIDv4 OwnershipTransferId|
|76|32|actual bestehenden Requestproducer-QuerySHA|
|108|32|SHA256 der originalen vollständig encoded Proposalbytes|
|140|8|deren actual encoded Byteumfang|

Strikt sortiert nach `(kind, sourcekey)`, je actual Sourceowner genau eine Row,
keine Zusatzrow, checked `128 + count*148`, keine trailing/reservedbytes.
Proposalbytes sind beim Player das unveränderte schema4/mode1/bodyenvelope,
beim Entity das original vollständige FCUE22+nativebody. Hashes ersetzen keine
bounded Rowvalidation. QuerySHA entsteht aus einem **wirklichen** Stage-Fresh-
OwnedSnapshot/Requestconstructor nach initialem Stageaufbau und vor dem ersten
CAS. Vorbereitung liest Source/Stage streaming pro Row, gibt danach Proposal-
heap frei und friert den vollständigen Plan. Die später neu aus derselben
Source Row/Fresh-Stageauthority erzeugte Request muss denselben Query besitzen;
keine Callerhash-/beforeStampkonstruktion oder IDs nach begonnenem CAS.

Private scratch IDindexrows haben actual UUID16key + ordinal8value=24Bytes.
Chunkcountindexrows haben ChunkKey16 + Sourceepoch8 + count8=32Bytes.
Weitere Table-/countmetadata wird ausdrücklich mit128Bytes gecharged;
`planbytes + 24*resources + 32*chunks + 128` checked vor jeder Indexerweiterung.
Mappingbytes haben ihre separate `64+51*players` cap. redb page-/cache overhead
bleibt gesondert, kein fälschliches logisches Scratchlimit als RSSlimit.

### Exact checkpoint — FCOCheckpoint1

Fixed Header genau376Bytes, Integer big endian:
Magic4 `FCMC`, schema u16=1, sourceformat u8=5, targetformat u8=6;
CheckpointUUID16; Source-/TargetlogicalSHA und Mapping-/PlanfileSHA (4×32);
Capacity Request-/Receipt-/Revisionlimit (3×u64);
Limits scanrows/scanbytes/mappingrows/planrows/scratchbytes/
verificationcopybytes/checkpointbytes (7×u64);
Counts sourcerows/sourcebytes/targetrows/targetbytes/players/entities/chunks/
receipts/revisions (9×u64); ContractSHA32; ReferenceproducerSHA32;
file_evidence_count u16=5; reserved6zero. Contract-/ReferenceSHA bindet die vor
Code gepinnte A-Bytegrammatik und tatsächliche Producerketten, keine automatisch
frei interpretierbare zukünftige Version. `ContractSHA` ist SHA256 der exakt
finalreviewten vollständigen Briefbytes, extern im Rootfreeze festgehalten und
als tatsächliches Parserliteral einzupinnen; dieses Dokument enthält seinen
eigenen Hash nicht rekursiv. `ReferenceproducerSHA` ist der SHA256 des sortierten
14-Datei-Producerchecksummanifests (UTF8, `hexsha256`, zwei Spaces, relativer Pfad,
LF pro Row): `646ff51b9293858e76849e3e0d7f38bc3790176935c1f8a2e99304270fa0ce51`.
Aktueller checked Producerstand ist Main
`765147ca3935cd69260a4bf682d756299cce4aed`; das Manifest liegt zur Review in
`.codex-tmp/dur03_2-producer-reference.sha256` und wird vor Storagecode in den
Ticketfreeze aufgenommen. Unknown schema/digest verweigert.

Danach genau5 Fileevidencerecords in fester Rolle Source, Mapping, Backup, Plan,
Stage. Fixedteil je60Bytes: role u8, flags u8, pathlen u16, device u64,
inode u64, observed_file_len u64, closedphysicalSHA32; danach `pathlen` originale
canonical absolute Unix-OsStrbytes (max4096, keine NULbytes). Keine arbitrary
UTF8normalisierung. Sourceflag0 hat32zero SHAbytes: Sourcelänge informativ,
sourceLogicalSHA bindend, physischer Sourcehash wird nicht behauptet. Andere
Rollenflag1 binden tatsächliche geschlossene Hashes **nach** finaler echter
Stagevalidation, Reclose und fsync. Nach Checkpointseal werden Stage/Backup
niemals selbst als redb wieder geöffnet; ausschließlich bounded eigene Kopien
werden geöffnet und logical geprüft. So bleiben die recorded physical Hashes
auch bei verweigertem Sourcecommit und Resume tatsächlich unverändert. Der
verschlossene Checkpoint selbst erhält anschließend
SHA256 über alle vorherigen Bytes als32B Footer.

Maximaler actual Umfang: `376 + 5*(60+4096) + 32 = 21188` Bytes, Default/
Hardcheckpointcap32KiB. Operator kann bis positiv kleiner21188 absenken; benötigte
actual Pfadlängen werden vor create_new checked. Roleorder, Flags, counts,
reserved und Footer werden vor Kopien streng geprüft. Keine redundant eingelesene
unbounded Stringliste. Alle für Sourcecommit benötigten Dateien sind durch
Callerpaths + echte OS-/redblocks + recorded identity gebunden, nicht durch SHA
allein. Ein digest ist kein Filesystemcapabilitytoken.

### Versiegelte Stage und Backups sowie tatsächlicher Copyproducer

Konkrete vorhandene Referenz sind
`redb_store/uuid_migration/backup.rs::verify_closed_copy` und
`uuid_migration/files.rs::{copy_closed,DiskBudget,TemporaryFile}`. Dort wird
das gelockte immutable Original niemals als redb geöffnet: cleanheader und
physicalhash prüfen, actual Copyextent gegen positive DiskBudget admitten,
eigene create_new Disposablecopy schreiben und hashen, ausschließlich die
Kopie redböffnen/scan/compare/abortieren, schließen und identitygebunden löschen,
Originalhash/identity danach erneut prüfen. redbhousekeeping ist lediglich in
dieser eigenen Kopie zulässig. A besitzt dieselbe konkrete Filesystemboundary;
keine erfundene read-only Öffnung eines tatsächlich schreibenden redb-Openers.

Die Reihenfolge ist verbindlich:

1. Genuine StageCAS beenden, echte StageDB schließen, realen Format6-Opener
   einmal vor Seal öffnen und alle15Tabellen mit Source/Plan vergleichen.
   Danach StageDB vollständig schließen, eigenes File fsync/cleanheadercheck,
   finalen physicalhash und actual Länge erfassen. Backup ist bereits in gleicher
   Copy-onlyweise vollständig logisch geprüft und selbst geschlossen geblieben.
2. Erst jetzt immutable Checkpoint mit den finalen hashes schreiben/fsync und
   Parentdirectorysync. Ein StageReopen ist ab diesem Zeitpunkt untersagt.
3. Vor SourcewriterTransaction und auf jedem Resume Source/Mapping/Plan/
   Checkpointidentities, immutable Backup-/Stagecleanheader und original
   physicalhashes erneut prüfen. Vor **jeder** Copycreation tatsächliche
   Originaldateilänge gegen `verification_copy_bytes` prüfen; Copybudget ist
   Default2GiB/Hardmax32GiB. N erlaubt den bestehenden tatsächlichen Fileextent,
   N+1 verweigert schon vor create_new. Bei redbhousekeeping wird jeder spätere
   growth erneut gegen denselben physischen DiskBudget geprüft.
4. Backupcopy create_new im expliziten `scratch_directory`, bounded Copy/
   Format5scan/exactSourcevergleich, echte copyDB schließen, eigene Copyidentity
   löschen und Originalhash erneut prüfen. Danach dieselbe Kopierkette für Stage
   durchführen: actual15Tablescan, genuine Header-/Receipt-/Plan-/Epochvergleich,
   echten Targetlogicalhash prüfen. Nur die jetzt geprüfte eigene StagecopyDB
   und ihre readonly Rows bleiben für den kommenden Sourceimport offen. Ein
   möglicherweise zur Validierung benutzter CopywriterTransaction ist vorher
   wirklich abortiert. Keine genuine neue CASausführung auf der Copy.
5. SourcewriterTransaction revalidiert den gelockten Sourcezustand im selben
   Snapshot; importiert ausschließlich geprüfte Stagecopyrows und das deklarierte
   Gesamtdelta; vollständiger Targetscan vor actual Commit. Mapping/Plan/
   Checkpoint-/Original-Stage/-Backupidentities und hashes bleiben checked. Nach
   Commit oder jedem Refusal eigene StagecopyDB schließen/identitygebunden
   cleanup und Originalhashes erneut vergleichen. Source-Postcommitreopen ist
   eine ausdrücklich erwartete Operation; Sourceflag0 bindet logical state und
   tatsächliche Fileidentity, nicht fälschlich unveränderte allocatorbytes.

Höchstens eine tatsächliche Verificationcopy ist gleichzeitig vorhanden; der
vor Copycreation reservierte Slot wird erst nach belegtem owned cleanup frei.
Auch bei copied-/opened-/scanned-/precommit-/postcommit-Cutpoints bleibt jede
immutable Originaldatei bytegleich zum Checkpoint. Fremdes Pfadreplacement wird
nicht gelöscht; eigener Cleanupfehler ist typisiert und erhält den Copybudgetslot
bis zum Gesamtausgang. Precommitfehler abortiert Source; ein Fehler nach actual
Commit wird `CommitUncertain` samt unveränderter bestehender Evidence. Ein
abgebrochener Lauf und dessen Resume öffnen dadurch dieselben originalen
physicalhashes, erzeugen keine legitime eigene EvidenceDrift und brauchen keine
neuen ImportIDs oder Bootstrapreceipts. Private deterministic Tests schneiden
zusätzlich vor/nach Copycreation, Copywrite, copyDBopen, scan, close und cleanup.

### Eigentum an den tatsächlichen Dateien und Commitcutpoints

`PreparedOwnershipMigration` hält Source exklusiv bis `prepare_files` oder Drop,
Mappingreadonlylock bis Gesamtausgang. `BoundedOwnershipMigration` übernimmt
move-only diese Authority und eigene Fileidentityhandles für alle5 Evidenzrollen
plus checkpoint. Private Stageowner/Runtime bleibt ausschließlich intern und
hat keine Socket-/Sim-/Appcapability. Source- und Stagewriter sind seriell;
SourcewriterTransaction darf erst nach sämtlichen StageCAS-/close/reopen-
Validierungen beginnen. Während des Sourcecommit gibt es keinen offenen
Stagewriter und keinerlei neues UUID-/Mapping-/CASreplan.

Jede neue Datei gehört genau dem durch create_new nachgewiesenen Handle.
Checkpoint wird als letzte immutable Datei vollständig geschrieben und fsynced;
Parentdirectoryfsync vor Sourcecommit. Bestehende Evidenz wird weder aktualisiert
noch ersetzt. CommitUncertain behält Evidenz, die Source darf dann nur mit
geprüftem Resume klassifiziert werden. Drop kann nur eigene disposable
Scratch-/nicht veröffentlichte Stage/Planteile entfernen; published Backup und
Checkpoint bleiben ausdrücklich erhalten. Cleanup auf replacedpath unterbleibt.

Sourcecommit bestätigt Sourcefingerprint nochmals **im selben** echten
writerTransaction, importiert Stageheaders/Receipts/meta/Epochdelta, überprüft
komplette15TableGrammar und exactTargetSHA vor Commit. Cutpoints: before_each_row,
after_each_row, before_meta, after_meta, before_marker, after_marker,
before_full_target_scan, after_full_target_scan, before_actual_commit,
after_actual_commit, before_reopen, after_reopen. Precommitfehler abortiert den
Transaction; aftercommit-/reopenfehler ist Uncertain. Die Referenz/Tests dürfen
keinen observercallback als alternative Runtimeauthority anbieten; Hooks sind
private deterministic Testcutpoints wie bestehende Migrationproducer.

## Historische erste Umsetzungspräzisierung nach Freigabe (kein Ersatz des Designpins)

Root hat am tatsächlichen Ausgangspunkt
`65f050fba91f252173709ccc5201755b3821bb20` die Storageproduktion freigegeben.
Der genehmigte Brief mit SHA
`4e66fd1b7bac1a9b00fc66c7b6c17653bc77ec6922cb20c6609302f9f40d7377`
und alle14 Originalproducerpins bleiben unverändert unter
`.codex-tmp/dur03_2/design.md` und `design.freeze.json` erhalten. Dieser
genehmigte Originalhash bleibt das tatsächliche Parser-ContractSHA-Literal;
nachfolgende Author-/Reviewnotizen ändern keine bestehende Grammarbindung.

Die folgende scratchfreie Präzisierung wurde durch die separate APIkorrektur
darunter ersetzt und beschreibt keinen aktuellen Produktionspfad:
`prepare(source,mapping,config,limits)` erhält noch keinen Caller-Scratchpfad.
Root hat deshalb die enge Präzisierung bestätigt: diese erste Phase erzeugt
keine Scratchdatei. Vollständige Sourcevalidation und checked Epoch-plus-
Entitycount laufen streaming über den vorhandenen gültigen Chunk-/UUIDindex
mit checked Rows/Bytes. Eigene bounded Count-/IDindices entstehen ausschließlich
in `prepare_files` nach expliziten OutputPaths/scratch_directory und vor Stage-
oder Sourcebootstrap. Ihre Werte werden gegen den weiterhin gelockten ursprüng-
lichen Sourcefingerprint erneut geprüft. Kein impliziter Scratchpfad oder
zusätzlicher unbounded Heapbestand entsteht. Auch `ResumePaths` erhält den
expliziten lokalen scratch_directory-Pfad für Disposablecopies; er ist eine
aktuelle Caller-Filesystemauthority und keine zusätzliche immutable Evidenzrolle.

## Separate APIkorrektur: explizite Scratchauthority bereits in prepare

Root hat nach vollständiger `uuid_validation`-Quellenprüfung den noch
unveröffentlichten neuen Preparevertrag korrigiert: auch
`prepare(source,mapping,scratch_directory,config,limits)` erhält ausdrücklich
ein Caller-Scratchverzeichnis. Der vorhandene Source-Receiptindex sortiert
UUIDs, nicht Journalintervalle. Deshalb benutzt die vollständige Prepareprüfung
den bestehenden bounded ReceiptIndexproducer mit identitygebundenem Cleanup
im tatsächlich ausgewählten Verzeichnis. Keine eigene Datei entsteht vor
kanonischer Directoryprüfung und positiver Boundadmission. Bestehende
Startup-/UUIDmigrationseinstiege behalten ihre Semantik.

Der vorgeschlagene scratchfreie Paarvergleich wurde verworfen, einschließlich
der möglichen checked P=R*(R−1)/2-Grenzen gegen Scanrows und Scanbytes; kein
quadratischer Produktionspfad wird eingeführt. Diese separate Zusatzentscheidung
ersetzt die frühere scratchfreie Preparepräzisierung oben, nicht den unveränderten
originalen Design-/Producerpin. Parser-ContractSHA bleibt `4e66fd…`.

Der tatsächliche Journalindex darf höchstens
`min(config.open_limits.max_journal_index_bytes, limits.max_scratch_bytes)`
physisch wachsen. Der zusätzliche eigene Count-/IDindex hat unabhängig eine
physische Dateigrenze von `limits.max_scratch_bytes`, Cache16MiB und den
separaten ursprünglichen logischen Plan+Count+ID-Bound. Es können diese zwei
Indexdateien gleichzeitig bestehen; ihre combined physische Obergrenze ist
2×Scratchbytes, zusätzlich zum Plan und höchstens einer Verificationcopy.
Die tatsächlichen Stage-/Backup-/Verificationcopy-Extents behalten ihren eigenen
Physicalbound. Dies behauptet weder Filesystem-Allocatedbytes noch Gesamt-RSS.

## Tatsächlicher Authorabschlusskandidat

Der Storageproducer ist implementiert. `PreparedOwnershipMigration` und
`BoundedOwnershipMigration` bleiben move-only; `OwnershipMigrationResume` bietet
nur `ReadyToCommit` oder `AlreadyCommitted`. Der öffentliche Prepareeinstieg
lautet tatsächlich
`RedbStore::prepare_player_entity_ownership_migration(source, mapping,
scratch_directory, config, limits)`. `prepare_files(OutputPaths)` erzeugt alle
neuen Evidenzdateien; `migrate()` liefert ausschließlich das Offlineoutcome.
Resume nimmt die aktuellen sechs Callerpfade und ein ausdrücklich ausgewähltes
lokales Scratchverzeichnis, den unveränderten Capacityvertrag und Limits.

Die elf neuen Runtime-Module einschließlich Orchestration und die Tests verändern
keinen UUID-/Ownershipvalidator und keine Player-/Entity-/Receiptgrammar.
Die vollständigen14 originalen Producerpins stehen unverändert in
`.codex-tmp/dur03_2/producer-reference.sha256`; die tatsächlichen neuen Hashes
und Abweichungsgründe stehen in `producer-current.sha256` und
`producer-deltas.json`. Zwölf Producerdateien sind bytegleich. Die zwei bewusst
abweichenden Dateien sind `redb_store/ownership.rs` (interne Sichtbarkeit des
echten Freshreaders) und `uuid_migration/files.rs` (interne Datei-/Readonly-/
Cachehelper sowie die folgende zusätzliche bounded I/O-Regel).

`LockedFile::hash_expected_len(expected)` prüft gebundene Identity und exakte
Länge vor dem ersten Hashbyte. Es liest höchstens die erwartete Länge plus ein
einziges nicht gehashtes Byte zur Wachstumserkennung; danach werden Identity
und Länge erneut geprüft. Verkürzung, Wachstum oder Extentabweichung verweigern
mit dem vorhandenen `BackupMismatch`; immutable Evidencechecks bilden dies auf
den konkreten `EvidenceDrift(role)` ab. Der gewöhnliche `hash()` delegiert mit
einem festen Anfangsextent. Gültige alte Hashbytes bleiben gleich. Auch
`copy_closed` liest/kopiert nur die vorher admittierte erwartete Länge plus ein
Byte Lookahead; keine dynamische EOFschleife bleibt in diesen Pfaden.

Die initiale Capturephase admittiert Stage-/Backuphashes gegen den konkreten
Physicalcap, Plan gegen Scratchbytes, Mapping gegen seine feste maximale
64+51×Rows-Ausdehnung und Checkpoint gegen den expliziten Checkpointcap. Die
Mappingparse→Hash-Callsites benutzen den bereits geprüften tatsächlichen
Readerextent. Resume und sämtliche versiegelten Kopie-/Evidencechecks benutzen
die aufgezeichneten exakten Extents. Ein vor Copycreation gewachsenes Original
erzeugt keine Disposablecopy.

Es bestehen19 spezifische Migrationstests. Zwei gezielte zusätzliche echte
postcommit-Stagecopytests treffen ausdrücklich die zweite Migrationcopy:
`after_committed_stage_copy_cleanup_is_uncertain_and_exactly_resumable` und
`replaced_committed_stage_copy_preserves_foreign_file_and_resumes_exact_target`.
Beide verlangen das tatsächliche `AfterCommit`, das unabhängige gesamte
15-Table-Target, bytegleiche fünf geschlossene Evidenzdateien und
`AlreadyCommitted` beim exakten Resume. Der zweite Test erhält außerdem die
wirkliche Fremddatei nach identitygebundenem Cleanuprefusal. Weitere gezielte
Tests prüfen alle fünf Evidenzrollen bei sparse Wachstum um1GiB, Verkürzung,
Hashcap N/N−1 und Postcopy-Wachstum/Cleanup. Diese vier Zusatztests bestehen.

Die vollständigen finalen Debug-/Releaseprüfungen bestehen auf dem isolierten
Gitarchiv `65f050fba91f252173709ccc5201755b3821bb20` plus ausschließlich dem
aktuellen vollständigen Storageoverlay. Der gebundene61-Dateimap hat SHA
`a602d69bd775b59d4c9c19d3e07dca2e76d14eb9966bf17fe0139888d3c04959`.
Strict All-target-Clippy, Package-Fmt und Rustdoc mit `-Dwarnings` bestehen
auf genau diesem Kandidat. Finale Ergebnisse und komplette Quell-/Evidenzmaps
sind nach den tatsächlichen Wholechecks eingefroren. Root prüft
sein aktuelles Mainarchiv separat; die Authorbasis behauptet keinen Main- oder
Workspaceabschluss. Zwischenstände und abgebrochene Kandidaten sind keine
Abnahmeevidence. Dieser Producer aktiviert weiterhin kein Survival oder Java.


## Finale Authorabnahme und Freeze

Alle sechs finalen Authorchecks bestehen auf dem oben gebundenen unveränderten
Storagekandidat. Debug und Release prüfen jeweils die vollständige Storage-Crate
einschließlich sämtlicher UUID-/Legacyregressionen und Dokutests; genaue Suitecounts
und Commands stehen in `.codex-tmp/dur03_2/author-results.json`, tatsächliche Logs
unter `logs/`. Die komplette Quellmap umfasst Storage plus diesen eigenen Brief;
`tested-storage.freeze.json` bleibt exakt der unveränderte61-Datei-Testkandidat.
Originaldesign/-Producerpins bleiben unverändert. `source.freeze.tar.gz` enthält
den vollständigen Sourcefreeze; `evidence.freeze.json` bindet alle Authorartefakte
außer der selbstreferenziellen Evidenzmap und `delivery.json`. Root besitzt weiter
die unabhängige Abnahme/Integration; kein Author-Gitcommit und kein Main-/Workspace-
oder spielbares Survival-/Paperabschluss wird behauptet.
