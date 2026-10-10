# STORAGE-ENTITY001.2 — Explizite Format-4 → 5-Migration

Status: **Implementiert und lokal grün; zur unabhängigen Implementierungsprüfung eingefroren**.
Owner: `select_world_ticket`. STORAGE-ENTITY001.1 ist unabhängig CLEAN geprüft
und als `a5d11819` integriert. Der vollständige Entwurf wurde vor Freigabe von
Root gelesen; die folgenden zwei Präzisierungen sind verbindlich:

- Neue Source-/Backup-/Checkpoint-/Scratch-Locks verweigern bereits belegte
  Dateien mit typisiertem Busyfehler; kein unbegrenztes Warten unter anderen
  Filelocks. Der bestehende3→4-Vertrag bleibt unverändert.
- Der neue Sourceformatwechsel verwendet ausdrücklich redb
  `Durability::Immediate`. Erfolg folgt dem tatsächlichen Commit; ein
  Commit-I/O-Fehler bleibt ungewiss und verlangt checked Resume. Das ersetzt
  keinen Prozesskill-/Powerlossnachweis.

Der unveränderte geprüfte Entwurf vor diesen Rootstatus-/Vertragszusätzen hatte
SHA256 `9118dd675b9247d0064b48f42bf5d4ad0ee9a51ff7e3eaef80b1c4f41755e385`.
Alle weiteren API-/Authority-/Delta-/Budget-/Checkpointregeln unten gelten.
Der `.1`-Brief und die älteren Design-/Reviewdokumente bleiben unverändert.
Dieses Dokument wurde aus der tatsächlich vorhandenen `.1`-Production und
WG-002-MIG abgeleitet. Die neuen APIs, Guards und Tests sind implementiert;
der vollständige lokale Nachweis und der Dateifreeze folgen unten. Am `.1`-
Validator ist nur der kontrollierte komplette Migrationeintritt samt begrenztem
Receiptbranch geändert; Defaultgrammatik/-fingerprint/-fehler bleiben erhalten.
Die alte neun-Tabellen-3→4-Migration bleibt bytegleich.

## Ziel und verbindliche Grenze

Ein vorhandener, vollständig gültiger Format4-Store mit **leerer Entitytabelle**
kann explizit offline nach Format5 überführt werden. Die neun bestehenden
Tabellen und alle Rawrecords bleiben erhalten; einzige Delta sind der
Formatmarker `4 → 5` und genau drei neue, leere Entityindextabellen.
Generationidentitäten werden übernommen, nicht deklariert, vereinheitlicht oder
aus Weltinhalten erraten. Mehrere tatsächlich gespeicherte Identitäten sind
zulässig. Opaque Player-, Chunk-, Plugin- und Journalbytes werden nicht
dekodiert/reencodiert, um sie zu kopieren.

Jede vorhandene wire-keyed Entityrow verweigert dieses Ticket, auch eine
decodierbare Row mit vermeintlich erkennbarem Body. Es gibt keine UUID-/Chunk-
Ableitung aus EntityId oder Payload, Löschung, Quarantäne oder reparierende
Migration. Format3 benötigt zuerst den separaten WG-002-MIG-Schritt nach4.
Normalopener und die alte 3→4-API behalten ihre bisherigen strikten Verträge.
Ein generischer erfolgreicher Format5-Open beweist keinen Migrationretry.

Productionumfang ausschließlich `ferrumc-storage`; Root besitzt Git/App.
Kein Runtimeconsumer, automatischer Startupupgrade, Welt-/Entitycodec,
Gameplay, Player↔Entity-Transfer, Vanilla-/Anvilimport oder Javaaktivierung.
Die Migration erhält keinen neuen Metakey und keine zusätzliche dauerhafte
redb-Tabelle. Ein separater Checkpoint ist Offlineevidenz, kein Gameplayreceipt.

Verbindliche Grundlage sind der
[Schemaentwurf](STORAGE-ENTITY001_UUID_PERSISTENCE_DESIGN.md), R4 des
[Designreviews](STORAGE-ENTITY001_UUID_PERSISTENCE_DESIGN_REVIEW.md) und der
[`.1`-Producervertrag](STORAGE-ENTITY001_1_UUID_STORE.md). Der Checkpoint- und
Prüfkopievertrag unten konkretisiert R4; Änderungen daran benötigen vor Code
Rootreview.

## Bestand und implementierte Producer

| Vorhanden | Verwendung in `.2` / konkret fehlender Teil |
| --- | --- |
| `migration::validation::scan`, neun Counter, `LegacyGenerationDeclaration` | Unverändert erhalten. Dieser Validator verlangt die erklärte Flat-/Seed0-Identität und bei unscoped State genau einen deklarierten Scope; er ist kein allgemeiner Format4-Validator. Nicht auf12 Tabellen oder beliebige Identitäten lockern. |
| `migration::copy_snapshot` | Unverändert für Format3. Neues `uuid_migration::backup::copy_format4` kopiert exakt alle neun Tabellen einschließlich `ferrumc:generation`. |
| `migration::{FileIdentity, LockedFile, SharedBackend}` und Header-/Geometry-/Hash-/Syncguards | Altes Modul unverändert. Neue eng begrenzte Offlineguards in `uuid_migration::files` behalten den alten Vertrag getrennt; `.2` besitzt eigene Alias-/Checkpoint-/Budget-/Busyregeln. |
| `uuid_validation::{validate_catalog, scan}` | `.1` prüft vollständig Format5. `.2` verwendet `validate_complete_migration` für **Catalog und Scan zusammen**, mit eigenem begrenzten Receiptindex-/Cleanupbranch; Defaultregeln und Startupfingerprint bleiben erhalten. |
| `.1`-Fingerprint `FerrumC UUID storage logical snapshot v1` | Bleibt eigener Startupfingerprint. Nicht als Format4-/Migrationsdeltafingerprint umdeuten. Neuer versionierter 12-Slot-Fingerprint mit Tabellenpräsenz und dualer4/5-Projektion. |
| Format5-Entitygrammatiken und echte Generationadmission | Wiederverwenden für strikten5-Scan; keine neue Entityinterpretation in `.2`. |
| Neue Offline-APIs, Typestates, Checkpointproducer, Read-only-Backupprüfkopie | Implementiert und unten spezifiziert; kein vorhandener Runtimeopener aktiviert sie. |
| Appbefehle/Deploymentwiring | Fehlen bewusst; eigenes Rootticket. Storage-Rustdoc muss angeben, dass alle neuen Methoden synchron/blocking und ausdrücklich offline sind. |

## API und Authority

Öffentliche implementierte Namen; alle Typen besitzen private Felder und
dokumentierte Getter. Reports/Evidenz dürfen Clone sein, Authoritytypen nicht.
Kein Raw-DBhandle wird veröffentlicht; kein `async fn` kaschiert Blocking-I/O.

```rust
// Öffentlicher Vertrag; Authorityfelder und Backendhandles bleiben privat.
UuidEntityMigrationLimits::new(
    max_verification_copy_bytes: u64,
    max_receipt_index_bytes: u64,
) -> Result<UuidEntityMigrationLimits, UuidEntityMigrationError>;

RedbStore::prepare_uuid_entity_migration(
    source_path: impl AsRef<Path>,
    limits: UuidEntityMigrationLimits,
) -> Result<PreparedUuidEntityMigration, UuidEntityMigrationError>;

PreparedUuidEntityMigration::create_backup(
    self,
    backup_path: impl AsRef<Path>,
) -> Result<BackedUpUuidEntityMigration, UuidEntityMigrationError>;

PreparedUuidEntityMigration::verify_existing_backup(
    self,
    backup_path: impl AsRef<Path>,
    remembered_closed_file_sha256: [u8; 32],
) -> Result<BackedUpUuidEntityMigration, UuidEntityMigrationError>;

BackedUpUuidEntityMigration::write_checkpoint(
    self,
    checkpoint_path: impl AsRef<Path>,
) -> Result<CheckpointedUuidEntityMigration, UuidEntityMigrationError>;

CheckpointedUuidEntityMigration::migrate(
    self,
) -> Result<UuidEntityMigrationOutcome, UuidEntityMigrationError>;

RedbStore::resume_uuid_entity_migration(
    source_path: impl AsRef<Path>,
    checkpoint_path: impl AsRef<Path>,
    backup_path: impl AsRef<Path>,
    limits: UuidEntityMigrationLimits,
) -> Result<UuidEntityMigrationResume, UuidEntityMigrationError>;

enum UuidEntityMigrationResume {
    Ready(CheckpointedUuidEntityMigration), // volle unveränderte4-Baseline
    AlreadyCompleted(UuidEntityMigrationOutcome), // volle exakte5-Delta
}
```

`prepare` akzeptiert ausschließlich Format4. Format5 verweigert mit
`CompletedVerificationRequiresCheckpoint`; es gibt dort kein evidenzloses
`AlreadyCompatible`. Die Wiederaufnahme ist die explicit checked Retryvariante
aus R4: ihr Checkpoint enthält die vorher festgehaltene Report-/Fileidentity-/
Backuphash-/Logicalsnapshot-Evidenz. Sie validiert Source und Backup erneut;
das Einlesen des Checkpoints allein erzeugt keine Commitauthority.

`Prepared` hält Source-Database und exklusiven Source-Filelock. `BackedUp`
ergänzt den exklusiven, **nicht als redb geöffneten** Backup-Filelock und seine
verifizierte geschlossene Evidenz. Ausschließlich `Checkpointed` besitzt die
Commitmethode; zusätzlich hält es den Lock des atomar publizierten, gesyncten
Checkpoints. Ein Clone von Report/Backup/Checkpoint hält keinen Lock und kann
nicht migrieren. Feld-/Dropordnung schließt jede Source-Database vor Freigabe
ihres letzten FileBackend-Locks. Die Limits werden durch alle Typestates
getragen und können beim Commit nicht heimlich erhöht werden.

`UuidEntityMigrationReport` enthält Sourceformat4, kanonischen Berichtspfad,
Sourcefileidentity, beobachtete Sourcefilelänge, Counter aller12 Slots,
Rawlogicalbytes, kompletten4-Fingerprint und erwarteten5-Fingerprint.
`UuidEntityMigrationBackup` enthält Pfad, Fileidentity, **geschlossene** Länge/
Physical-SHA256 und den4-Logicalsnapshot. `Outcome` enthält diese Baseline,
die komplett geprüfte5-Evidenz, Checkpointpfad/-hash und die Herkunft
`Committed`/`VerifiedCompleted`. Erfolg wird erst nach wirklichem Commit bzw.
voller abgeschlossener Wiederaufnahmeprüfung berichtet.

## Vollständiger Format4-Preflight

Erfolgreicher Preflight prüft sämtliche Zeilen; Fehler dürfen sofort abbrechen,
ohne einen teilweise geprüften Store als migrierbar zu melden. Ein eigener
versionierter Validator verwendet keine `LegacyGenerationDeclaration`.

| Kategorie | Exakte Admission |
| --- | --- |
| Catalog | Genau neun normale Tabellen; keine fehlende/fremde Tabelle oder Multimap. `meta: &str → u64`, alle acht übrigen `&[u8] → &[u8]`. Typen tatsächlich öffnen; Writertransaction zum Preflight anschließend abbrechen. |
| Metadaten | `format_version=4` vorhanden; ausschließlich dieser Key und optional `last_mutation_id`. Kein fehlender Marker, zusätzlicher Key oder erfundener Migrationreceipt. |
| Generation | Key8, genaue bestehende27-Byte-Identitydecodierung, höchstens4096 Descriptoren. Tatsächliche Seeds/Revisionen/Dimensionregeln erhalten. Descriptorpointlookup für jede scoped Row; kein Deklarationsersatz. |
| Chunk/Overlay | Key16, vollständige Chunkorigin-/Localoffset-Domäne, wirklicher Descriptor im Scope, vorhandene bounded Codecprüfung inklusive aller Block-Entity-/Itemkomponenten. Recordpos muss dem Key entsprechen. Max3MiB encoded Record und bestehende untergeordnete Caps erhalten. Auch leere Overlays zählen und werden kopiert. |
| Entity | Tabelle muss leer sein. Erste Row liefert `LegacyEntitiesPresent`; keine Bodyanalyse zum UUIDgewinn, keine Löschung. Malformed Catalog/Typen bleiben eigene Fehler. |
| Player | Key16, bestehender bounded Opaque-Playercodec. Jede Row verlangt mindestens einen tatsächlichen Descriptor. Keine Zuordnung zu einer nachträglich geratenen Welt; mehrere bestehende Descriptoren sind zulässig. |
| Plugin | BE-u32 Namespacebytezahl, höchstens4096 gültige UTF-8-Bytes; nachfolgender UTF-8-Key höchstens512 Bytes, Value höchstens1MiB. Checked Prefixaddition, keine Allocation aus ungeprüfter Länge. Namespace4097 wird ausdrücklich vor Backup/Commit verweigert, obwohl ältere Format4-Writer dies noch zulassen konnten. |
| Journal | Key8/BE-u64, Sequenz lückenlos ab0, Record-ID=Key-ID, bestehender bounded Codec. Jede Row verlangt mindestens einen Descriptor. Overflow am Sequenzende verweigert zusätzliche IDs. |
| Watermark | Genau letzter Journal-ID oder bei leerem Journal abwesend. Ein gespeicherter Watermark ohne passenden Journalinhalt verweigert; keine Reparatur. |
| Receipt | UUIDkey16, bestehende genaue Receiptgrammatik und Batchcap64. Auch leere Receipts verlangen einen tatsächlichen Descriptor. Nichtleere Ranges vollständig im geprüften contiguous Journal; unterschiedliche Ranges überschneiden sich nicht. Eigener begrenzter Diskindex, keine Welt-/Receipt-Vec. |

Descriptor4097, überlange Pluginnamespace, ungültige Journal-/Receiptform und
fehlende Scopes müssen bereits im vollständigen4-Preflight scheitern, bevor
irgendein dauerhaftes Backup/Checkpoint entsteht. Der alte neuner Validator
und seine genau-ein-Deklarations-Regel bleiben davon unabhängig unverändert.
Blank Format4 mit leerer Generationtabelle ist migrierbar, sofern sämtliche
scoped/unscoped Statekategorien leer sind; Plugin-only ist zulässig. Dieses
Ticket initialisiert dabei keinen Descriptor.

## Fingerprint und exakte Migrationdelta

Neuer Domainseparator: `FerrumC UUID entity migration logical snapshot v1\0`.
Slotreihenfolge und öffentliches eigenes Tableenum sind unveränderlich:

| Slot | Tabelle | Format4-Präsenz | Format5-Präsenz |
| --- | --- | --- | --- |
| 0 | `ferrumc:meta` | vorhanden | vorhanden |
| 1 | `ferrumc:generation` | vorhanden | vorhanden |
| 2 | `ferrumc:chunk` | vorhanden | vorhanden |
| 3 | `ferrumc:chunk_overlay` | vorhanden | vorhanden |
| 4 | `ferrumc:entity` | vorhanden, leer | vorhanden, leer beim Migrationretry |
| 5 | `ferrumc:entity_locator` | abwesend | vorhanden, leer |
| 6 | `ferrumc:entity_chunk` | abwesend | vorhanden, leer |
| 7 | `ferrumc:entity_chunk_epoch` | abwesend | vorhanden, leer |
| 8 | `ferrumc:player` | vorhanden | vorhanden |
| 9 | `ferrumc:plugin` | vorhanden | vorhanden |
| 10 | `ferrumc:mutation_log` | vorhanden | vorhanden |
| 11 | `ferrumc:mutation_batch` | vorhanden | vorhanden |

Jeder Slot hasht `table\0`, BE-u64 Namenslänge, Namen, feste typisierte
Key-/Valuetagbytes und ein Präsenzbyte. Jede vorhandene Row in redb-Keyordnung
hasht `row\0`, BE-u64 Keylänge, Rawkey, BE-u64 Valuelänge, Rawvalue. Metakeys
werden UTF-8-bytegleich und u64-Metavalues als BE8 erfasst. Jeder Slot endet
mit `end\0` und BE-u64 Rowcount; abwesende Slots haben Counter0, aber ein
anderes Präsenzbyte als leere vorhandene Tabellen. Checked Count-/Byteaddition;
kein Zusammenmischen mit den bestehenden9-/12-Startupfingerprints.

Der4-Scanner führt zwei Hashzustände parallel. Der tatsächliche4-Hash erfasst
die vorhandene Baseline. Im **erwarteten5-Hash** ändern sich ausschließlich
der bereits validierte Markerwert zu5 und die Präsenzbytes der Slots5–7 zu1;
diese Slots bleiben leer. Kein anderer Rowwert und kein Count wird ersetzt.
Der complete5-Scanner erzeugt denselben Migrationsfingerprint tatsächlich.
Rohlogische Key-/Valuebytes und sämtliche12 Counts werden zusätzlich verglichen.

Wiederaufnahme mit Source5 erfordert zuerst `.1`-strikten vollständigen
12-Tabellen-Catalog-/Row-/Relations-/Descriptor-/Journal-/Receiptcheck.
Danach müssen Counts, Bytes und Hash exakt der Checkpointprojektion entsprechen;
Entity/Locator/Index/Epochs sind alle leer. Zusätzlich vergleicht
`compare_exact_migration_delta` Source5 und vollständig geprüfte Backup4
zeilenweise über die neun gemeinsamen Tabellen: gleiche Keys/Rawvalues, mit
einziger Ausnahme Marker4/5. Kein Hashvergleich ersetzt diese bounded
Rawdelta-Prüfung. Neue Chunk-/Player-/Plugin-/Descriptor-/Journalbytes verweigern
ebenso wie neue UUID-Entities oder historische Epochs nach Delete. Bereits
gespieltes, allgemein gültiges Format5 kann somit nicht als derselbe Retry
durchgehen. Eine partielle/malformed5-Delta wird nie vervollständigt oder repariert.

## Source-, Backup- und Prüfkopie-Files

Alle Offlinepfade bleiben Unix-only; `device/inode` bezeichnet **Fileidentity**,
Physical-SHA256 die momentanen Dateibytes. redb Open/Close kann Verwaltungsbytes
und Dateilänge verändern, ohne Fileidentity oder FerrumC-Key/Valuebytes zu ändern.
Sourcephysicalhash und beobachtete Sourcefilelänge sind deshalb ausdrücklich
**keine** Retrygleichheitskriterien. Der Sourcecheckpoint bindet Fileidentity,
kompletten Logicalsnapshot, Counts und alle Tabellenpräsenzen. Pathreplacement
oder eine andere inode verweigert auch bei gleichen Rows. Ein Restore auf eine
neue inode bzw. ein Hostwechsel benötigt eine eigene ausdrücklich geprüfte
Neuaufnahme, nicht automatisches Absenken des Identityguards.

Vor Open: kanonischen Parent ermitteln, finalen Symlink verweigern, regular/
nonempty prüfen, exklusiven redb-FileBackend-Lock halten, Path-/Handleidentity
vergleichen, gepinnten redb2.6.3 Header/Geometry-/Recoveryguard ausführen und
Repaircallback abortieren. `.2` verlangt für Source, Backup und Prüfkopie
`nlink=1`; dieselbe canonical path oder device/inode in zwei Rollen verweigert.
Checkpointpublication ist die unten genau begrenzte own-Hardlink-Ausnahme.
Diese redb-Headerguards gelten für Source/Backup/Prüfkopie. Checkpointfiles
werden ausschließlich mit dem344-Byte-Parser geprüft und niemals redb-geöffnet.
Keine automatische Recovery oder Pfadreparatur. Guards laufen erneut an den
späten Übergängen; advisory Locks behaupten keine Sicherheit gegen fremde
Rawwriter oder bösartige Parentdirectorymutation außerhalb dieses Vertrags.

Frisches Backup: ausschließlich `create_new`, eigener Lock, exakte Rawkopie
aller neun Tabellen in einer Backupwritertransaction; Source weiter gesperrt.
Backup-Database schließen, Datei **und Parentdirectory** fsyncen, sauberen
geschlossenen Header prüfen und geschlossenen Physicalhash/Länge erfassen.
Anschließend die vollständige unabhängige Backupprüfung unten durchführen.
Kein bestehender Backup wird überschrieben. Bei Fehlern darf ein eigenes
unvollständiges dauerhaftes Backup für explizite Inspektion liegen bleiben;
es wird nicht anhand des Namens oder eines neu geratenen Hashs wiederverwendet.

Bestehendes Backup: caller liefert einen **vorher aufgezeichneten**
geschlossenen SHA256. Source/Backuppath und Identitäten prüfen, Backup exklusiv
sperren und diesen SHA/Länge prüfen, **bevor** irgendein redb-Open auf dessen
Bytes erfolgt. Neue Prepare-Reuse nimmt die tatsächlich gelockte Backupidentity
in die Evidence auf; Checkpointresume verlangt die bereits aufgezeichnete
Backupidentity zusätzlich. Backup niemals direkt redb-öffnen, auch nicht bei
Resume; ansonsten könnte ein späterer Fehler/Lostreply seinen remembered Hash
durch Housekeeping ungültig machen.

`verify_closed_backup_copy` erzeugt stattdessen eine own `create_new`-Prüfdatei
mit eindeutigem UUIDnamen, höchstens16 Kollisionsversuchen und64KiB Copybuffer.
Geschlossene Backupbytes unter Lock kopieren; Kopierlänge und währenddessen
berechneter Hash müssen der aufgezeichneten Evidence entsprechen. Prüfdatei
fsyncen, Header/Geometry prüfen und ihren Hash **vor** redb-Open vergleichen;
Originalbackup danach erneut hashen. Erst die Prüfkopie redb-öffnen, alle neun
Format4-Tabellen/Rows vollständig prüfen und mit dem Report vergleichen.
Nach Close/Cleanup der Prüfkopie Originalhash/-länge/-identity erneut prüfen.
Das Originalbackup bleibt durch jeden gelungenen/fehlgeschlagenen Prüfpfad
ungeöffnet und physical-bytegleich; dies ist keine Sourcephysicalgarantie.

Temporäre Filecleanup löscht ausschließlich die noch exakt eigene regular
device/inode nach Schließen sämtlicher Handles. Ersetzte Pfade bleiben erhalten
und ergeben einen präzisen Cleanup-/Identityfehler. Vor Commit dürfen keine
ausstehenden falliblen Receipt-/Prüfkopiecleanups bleiben. Kein blindes Löschen
von ähnlich benannten Dateien beim nächsten Start.

## Checkpointformat und atomare Publication

Der Checkpoint ist ein eigenständiges, genau **344-Byte**-Record. Vorschlag für
den festen Schema1-Body (312 Bytes, anschließend32 SHA256-Checksumbytes):

| Feld | Bytes |
| --- | --- |
| Magic `FCUMIG01` | 8 |
| Checkpointschema1, Unix-Identityschema1, Fingerprintgrammar1, Reserved0; je BE-u16 | 8 |
| Sourceformat4 / Zielformat5; je BE-u64 | 16 |
| Source device/inode; je BE-u64 | 16 |
| beobachtete Sourcefilelänge, nur Bericht | 8 |
| Backup device/inode; je BE-u64 | 16 |
| geschlossene Backuplänge | 8 |
| remembered Backup-Physical-SHA256 | 32 |
| zwölf BE-u64 Rowcounter | 96 |
| logische Key-/Valuebytezahl | 8 |
| tatsächlicher4-Logicalsnapshot-SHA256 | 32 |
| erwarteter5-Logicalsnapshot-SHA256 | 32 |
| eigene Checkpoint device/inode; je BE-u64 | 16 |
| bekannte own-Staging-UUID | 16 |
| SHA256 über den vollständigen312-Byte-Body | 32 |

Keine variablen Pfade, Payloads oder Descriptor-Vec im Record. Source-/Backup-
pfade liefert der Caller; tatsächliche Fileidentitäten müssen passen. Die
Stagingbasename ist aus der UUID eindeutig `.ferrumc-uuid-checkpoint-<uuid>.tmp`
im **selben kanonischen Parent wie der Finalcheckpoint**. Parser prüft genaue
Länge vor Allocation, Magic/Schemas/Reserved/Checksum,4→5-Konstanten,
Entity-/neue Indexcounts0, Descriptorcap und Metacount1..2; anschließend sind
trotzdem die echten vollständigen Filescans nötig. Checksum ist keine Signatur
oder Authentizitätszusage gegenüber einem Angreifer mit Schreibzugriff.

Publication nach verifiziertem Backup, Source unverändert gesperrt:

1. Eindeutige Stagingdatei `create_new` im Zielparent, regular/nlink1,
   eigener exklusiver Filelock; ihre identity im Checkpointbody erfassen.
2. Alle344 Bytes schreiben, vollständig read-back prüfen und Datei fsyncen.
3. `fs::hard_link(staging, final)` als atomare **no-clobber** Publication im
   selben Dateisystem. Kein rename mit implizitem Overwrite. Existing final,
   Symlink oder unrelated Hardlink verweigert; Existing-Reuse nur über Resume
   mit voller Prüfung, niemals durch `write_checkpoint`.
4. Finalpath auf genau eigene identity/complete bytes prüfen und Parentdirectory
   fsyncen. Publication hat nun temporär **nlink2**; das ist nur dieses bekannte
   Staging/Final-Paar, keine allgemeine Erlaubnis für weitere Hardlinks.
5. Nur eigene bekannte Stagingalias entfernen, Parent erneut fsyncen,
   Finalidentity/-hash und nlink1 prüfen. Erst dann `Checkpointed` zurückgeben.

Absturz nach Final-link, aber vor Directory-fsync/Tempunlink kann einen
vollständigen Finalcheckpoint mit bekannter own-Stagingalias hinterlassen.
Resume akzeptiert nlink2 ausschließlich, wenn die im Body gespeicherte
Stagingbasename im gleichen Parent regular/kein Symlink ist, dieselbe gespeicherte
device/inode hat und genau dieses Zweierpaar vorliegt. Bei nlink1 muss die
Stagingalias fehlen; fremde Row am Stagingpfad wird nicht gelöscht/adoptiert.
nlink>2, nlink2 ohne bekannte Alias oder falsche gespeicherte Fileidentity
verweigern als `UnexpectedFileAlias`/`CheckpointStagingChanged`.

Nach vollständiger Source-/Backup-/Deltaprüfung fsynct Resume den vorhandenen
Finalcheckpoint und Parent erneut, beseitigt ausschließlich die bestätigte own
Alias und fsynct Parent/nlink1. Für Source4 geschieht dies **vor** Rückgabe der
Commitauthority, für Source5 **vor** Rückgabe von `AlreadyCompleted`. Bei
Deltamismatch bleibt die checkpointbezogene Alias für Inspektion erhalten;
disposable Receipt-/Prüfdateien werden dennoch identitychecked bereinigt.
Das Vorhandensein des Finalpath allein wird weder als gelungene Publication
noch als abgeschlossene Migration gewertet.

Absturz vor Final-link hinterlässt allenfalls eine own-Stagingdatei und Source4;
es konnte noch kein Commit erfolgen. Kein automatisches Glob-/Alias-Guessing.
Erneutes Prepare und explizit hashgebundene Backupreuse dürfen einen **neuen**
Checkpoint an einem freien Finalpath erzeugen; alte Stagingreste bleiben ohne
explizit verifizierte Cleanupentscheidung unangetastet. Fehlt der remembered
Backuphash, wird ein neuer create_new-Backup nötig, kein blindes Reuse.
Teilweise/truncated/umschriebene Checkpoints verweigern. Der Checkpoint wird
nach Publication und auch nach Commit **nicht geändert**: kein Commitflag-
Rewrite, der selbst bei Lostreply eine neue Evidence verlieren könnte.

## Commit und Wiederaufnahme

`migrate` beginnt eine echte Sourcewritertransaction. Innerhalb derselben
Transaktion komplette4-Validierung und Vergleich mit captured Report/Checkpoint,
dann genau Locator-/Chunkindex-/Epochtabelle leer anlegen und Marker5 schreiben.
Die Entitytabelle bleibt leer; alle übrigen Rawrows bleiben unangetastet. Ein
vollständiger staged5-Check plus exakte Projektion prüft die finale Delta noch
vor Commit. Kein Metadata-/Descriptor-/Journalwrite wird zusätzlich erzeugt.

Unmittelbar vor `txn.commit`: Sourcepath/identity/nlink, Backupidentity/nlink/
geschlossener Hash/Länge und Checkpointidentity/nlink/ganze344 Bytes erneut
prüfen. Fehler oder Producercutpoint vor Commit verwirft die **gesamte**
Writertransaction. Kein Erfolg vor dem tatsächlichen Commit. Nach Commit gibt
es keinen automatischen Rollback; fehlende Antwort oder Commit-I/O-Fehler ist
kein Beweis, dass Format4 erhalten blieb. Checkpoint und Originalbackup bleiben
unverändert als Eingaben für den explicit checked Resume.

Resume hält Source, Backup und Checkpoint exklusiv, verweigert Rollenalias und
Recovery-required Files, prüft remembered Backupbytes vor Prüfkopiecreation,
validiert Source vollständig und Backup4 über die Prüfkopie. Source4 muss
captured4-Catalog/präsenz/counts/hash/rawbytes exakt entsprechen; es liefert
danach `Ready`. Source5 benötigt den strikten12-Tablecheck, expected5-Counter/
Hash/präsenz und den bounded Rawvergleich gegen Backup4; danach
`AlreadyCompleted`. Alle anderen Marker oder Deltas verweigern. Der
Checkpoint wird bei keiner Variante an eine neue Baseline angepasst.

Eine ordnungsgemäße4→5-Migration liefert damit überprüfbare logische
Idempotenz des Offlineablaufs. Das ist keine allgemeine Runtime-Entityretry-,
CAS-, Transfer-, Prozesskill-/Powerloss- oder Itemconservationzusage.
Recovery-required Source nach unklarem Commit verlangt separate kontrollierte
Recovery; Resume setzt den Recoveryguard nicht eigenmächtig außer Kraft.
Rollback bleibt ausdrücklich separate Offlinewiederherstellung des verifizierten
Format4-Backups nach Schließen sämtlicher Handles und Erhalt der ersetzten
Source; eine neue Sourceinode wird nicht unter altem Checkpoint adoptiert.

## Ressourcenvertrag

`Limits::new` fordert beide expliziten positiven Budgets und ihre mit344
Stagingbytes overflowfreie Summe; keine implizite
Unlimited-Default. MaxVerificationCopy begrenzt physische Prüfkopielänge samt
etwaigem redb-Openwachstum, MaxReceiptIndex begrenzt den eigenen temporären
Receiptindex. Vor Copy vollständige Backuplänge admissionieren. Ein privater
budgetierter Backendwrapper verweigert jedes `set_len` und jedes checked
`offset+data.len()` über Budget **vor** dem eigentlichen Filewrite. Errors
erhalten einen eigenen Budgetcause; redb-I/O-Wrapping darf diesen nicht als
unbekannte Corruption verlieren. Ohne budgetierten Receiptproducer/kompletten
5-Eintritt ist `.2` nicht fertig. `.1`-Runtimeopener erhält weiterhin seinen
bestehenden Vertrag; die neue Migrationkonfiguration lockert ihn nicht.

Die Operation hält höchstens eine Prüfkopie und einen Receiptindex gleichzeitig;
Scans verschiedener DBs laufen sequenziell, sodass nicht mehrere Receiptindizes
parallel entstehen. Temporäre logische Fileextents sind pro Aufruf auf die
Summe beider Budgets plus344 Checkpointstagingbytes begrenzt. Filesystemblock-
rundung, Metadaten und nach fehlgeschlagenem eigenen Cleanup verbleibende
Inspektionsdateien sind keine behauptete globale Hostdiskquote; diese Fälle
werden explizit berichtet, nicht stillschweigend vergessen. Das dauerhaft
erforderliche Backup und Sourcewriterwachstum sind separat und können ENOSPC
liefern, ohne Vor-Commit-Erfolg.

Source-/Backup-/Prüfkopie-redb Caches je64MiB; Backup-DB ist vor Prüfkopie-Open
geschlossen. Peak konfigurierte Caches höchstens128MiB plus ein16MiB Receipt-
Cache. Zusätzlich: höchstens4096 Descriptoradmissionen bzw. Pointlookups,
64KiB Copy-/Hashbuffer, konstante344-Byte Checkpointdaten und eine bounded
decodierte Row. Decoded-heap-/redb-internal-/Filesystem-Cacheoverhead extra;
Encodedcaps sind keine vollständige RSSgarantie. Keine World-Vec, Receipt-Vec,
unbounded Queue oder Tick-/Tokio-worker-I/O.

## Producer-Cutpoints und Fehlermatrix

`UuidEntityMigrationError` unterscheidet mindestens InvalidLimits,
UnsupportedPlatform, WrongFormat/CompletedVerificationRequiresCheckpoint,
Catalog/Metadata/Key/Record/Descriptor/LegacyEntitiesPresent, Journal/Receipt,
SourceChanged, BackupMismatch, CheckpointMalformed/Changed/Publication,
UnexpectedFileAlias/CheckpointStagingChanged, MigrationDeltaMismatch,
ScratchBudgetExceeded, Cleanup sowie strukturierte I/O-/redb-Backendfehler
mit Operation/Pfad/ursprünglichem Cause. Commit-I/O-Fehler werden ausdrücklich
als möglicherweise committed gemeldet; kein allgemeines Backenderror wird als
garantierter Transactionabort ausgegeben. Reports benutzen Scanvergleich,
nicht Whole-report-Equality mit physischer Sourcefilelänge.

Testobserver liegen an **echten** gemeinsam von Production genutzten Übergängen,
nicht an einer separat erfundenen Testmigration. Deterministische Tests ohne
Sleeps; vollständige Rawcatalog-/Rowsnapshots vor/nach Returned-error.

| Cutpoint / Fehler | Erwarteter Zustand und Recovery |
| --- | --- |
| Catalog-/Descriptor-/Payload-/Entity-/Journal-/Budgetrefusal | Kein Source-Record-/Marker-/Catalogwrite, kein dauerhaftes Backup/Checkpoint; temporäre own Files werden kontrolliert bereinigt. Sourcephysicalbytes dürfen von redb-Housekeeping abweichen. |
| Backup create_new/Copy/Commit/Close-sync/Hash/Prüfscanfehler | Source4 unverändert; eigenes unvollständiges Backup kann bestehen. Kein backed-up Typestate/Commit; Reuse ausschließlich remembered SHA + vollständige neue Prüfung. |
| Bestehendes Backup falscher SHA/Identity/Len, dirty Header oder Rollenalias | Refusal vor Backup-redb-Open und vor Runtimewrite; Originalbackup bleibt ungeöffnet. |
| Prüfkopie Create/Read/Write/fsync/Open/Scan/Close/Cleanupfehler | Originalbackup bytes/hash bleiben exakt; keine Commitauthority. Eigene Tempfileidentity wird geprüft, fremder Ersatz erhalten; Cleanupfehler klassifiziert. |
| Checkpointpartialwrite/read-back/fsync, Fehler vor Final-link | Kein Sourcecommit; nur eigene Stagingreste möglich. Kein Globguess/Overwrite. |
| Existing Final, Fehler nach Link vor dirsync, nach dirsync vor unlink, nach unlink vor dirsync oder vor reply | Source4, kein ausgegebener Commitwert. Final wenn vorhanden ausschließlich full guarded Resume; bekannte stagingalias/nlink2 gezielt prüfen. Fsyncfehler nicht als durable Publication melden. |
| Source-/Backup-/Checkpointpath oder Rawbytes während später Checks ersetzt | Refusal; gesamte Sourcewritertransaction abort. Keine fremde Datei löschen oder anderen Snapshot annehmen. |
| Nach jedem der drei tatsächlichen Tablecreates und nach Markerwrite | Gesamter9-Table-4-Snapshot unverändert nach Abort, keine teilweise5-Indizes. |
| Staged5-Validator/Delta-/Cleanup-/später Filecheckfehler | Ganze writertransaction abort; unveränderte immutable Checkpoint-/Backupbasis. |
| Unmittelbar vor Commit | Returned-error erhält vollständige4-Baseline. |
| Nach tatsächlichem Commit vor reply | Injected Lostreply bei vollständigem12-Table-5-Snapshot; checkpointgebundener Resume bestätigt exakt dieselbe Migration, ohne neue Rows/Hashrefresh. |
| Resume Source4/Source5 mit known stagingalias | Fullscan/delta vor checkpointbezogener Cleanup; anschließend fsync/unlink/fsync/nlink1. Fehler erzeugt keinen Completed-Erfolg. |
| Source5 gültig, aber inzwischen verändert | `MigrationDeltaMismatch`, selbst wenn Entities wieder gelöscht wurden; Epochhistory zählt. Kein Rollback/Checkpointrewrite. |

## Abnahme und implementierte Dateien

Neue Dateien: `src/redb_store/uuid_migration.rs`, seine
`{files,validation,checkpoint,backup,tests}.rs`-Module sowie
`tests/uuid_entity_migration.rs`. Das Filemodul hält tatsächlich genutzte Guards;
`lib.rs` veröffentlicht nur neue APItypen, `redb_store.rs` registriert das Modul.
`uuid_validation.rs` besitzt den kontrollierten kompletten Migrationeintritt
und budgetierten Receiptbranch. README/INVARIANTS benennen Mode-/File-/Checkpoint-/
Delta-/Limitvertrag. Kein Lockfile-/Fremdcrateedit; private konstante Boxfelder
begrenzen die Größe der Resumeenum, ohne ihre öffentlichen Varianten zu ändern.

Pflichtfälle:

- Gültige leere, plugin-only und volle Multiworld/Multidimension4-Baselines mit
  unterschiedlichen echten Generationidentitäten; Rawgleichheit aller neun
  Tabellen und genau3-empty-table/Markerdelta nach Commit; anschließender
  strikter Format5-Open und alte Format3-/4-Openerrefusal.
- Keine Descriptorinferenz; missing/4096/4097, verschiedene Seeds/Revisionen,
  falsche Chunkpos/-domäne, jede malformed/over-limit Recordkategorie,
  unknown Metadata/Catalog/multimap/type, vorhandene wire-Entity inklusive
  Emptybody und maximaler Payload. Jede Refusal ohne Rawteilmutation.
- Plugin ASCII-/Multibytenamespace4096/4097, Key512/513, Value1MiB/+1,
  malformed Prefix/UTF-8. Journal gaps/from1/Record-ID mismatch/watermark,
  Receiptshape/count/range/empty/overlap und fehlende tatsächliche Descriptoren.
- Drei absent/empty/present-new-table-Fingerprintfälle; Slot-/Row-/Typetag-
  Trennung; Counter-/Byteoverflow ohne riesige Allocation. Duale4/5-Projektion
  und gezielte Änderung **jeder** der zwölf Kategorien schlagen fehl.
- Backup exakt neun Tabellen samt Generationbytes; fresh create_new refusal,
  remembered-hash reuse und falscher/fehlender hash, Source-/Backuphardlink/
  Symlink/Pathreplacement/dirty Header. Originalbackup-SHA bleibt bei allen
  Prüfcopysuccess-/error-/Lostreplypfaden unverändert.
- Alle344-Byte-Checkpointparsergrenzen, schema/reserved/checksum/identity/
  counts; Publicationcutpoints einschließlich finalExists, Directory-fsync-
  Fehler, own nlink2, unrelated nlink2/nlink3, missing/fremde Stagingalias,
  no-clobber Final und identitychecked Cleanup ohne Fremddelete.
- Resume4 vor Commit, Resume5 nach tatsächlichem Lostreply, Wiederholung und
  Neustart mit gespeicherter Checkpointdatei; keine Sourcefilelen-/Physicalhash-
  Falsemismatches durch redb-Housekeeping. Neue Sourceinode verweigert.
- Gültiges5 mit anschließendem Chunk/Player/Plugin/Generation/Journalwrite,
  UUIDsave und UUIDdelete mit verbleibender Epoch verweigert Migrationretry;
  generischer5-Open darf weiterhin seinen eigenen gültigen Store akzeptieren.
- Copy-/Receiptbudget genau an Grenze/+1, checked Offsetoverflow und Wachstum
  beim redb-Open; alle Budget-I/O-Errors spezifisch, Originals unberührt;
  maximal eine Copy/Indexdatei, safe Cleanup bei Pathreplacement/ENOSPC.
- Producerabort nach jedem wirklichen tablecreate/markerwrite; Commit-/Reply-
  Cutpoint mit tatsächlichem backendcommit; komplette Storage-/WG-002-MIG-
  Regression, all-target Clippy `-D warnings`, owned fmt und Rustdocfälle.

Logs/Hashes/Dateifreeze und unabhängiger Implementierungsreview gehen vor
Rootintegration. Kein Source-only-, Abort-/Reopen- oder Lostreplytest wird
nachträglich als Paper-Differenzial-/Prozesskill-/Powerlossabnahme umbenannt.

## Implementierungsnachweis und Reviewfreeze — 2026-10-10

Root stellte den isolierten `a5d11819`-Baseline unter
`.codex-tmp/storage-entity001_2-isolated` bereit. Ausschließlich der eigene
Storagebereich wurde dort kopiert, einschließlich der separaten, inzwischen von
Root als`9025c687` integrierten expliziten `&[u8]`-Typisierung im `.1`-Test.
Root besitzt Git/App und die abschließende Workspaceintegration. Toolchain:
Rust/Cargo/Clippy/rustfmt`1.99.0`, Workspaceedition2021, offline/locked, Compile
`-j2`, Tests`--test-threads=2`; eigener externer Targetpfad
`.codex-tmp/storage-entity001_2-target`.

| Check | Ergebnis | Vollständiger Log / SHA256 |
| --- | --- | --- |
| `cargo +1.99.0 test -p ferrumc-storage --all-targets --offline --locked -j2 -- --test-threads=2` | **233 bestanden**,0 fehlgeschlagen:192 Unit und41 Integration, inklusive aller bisherigen Storage-/WG-002-MIG-Tests. Unitzeit209,62s; öffentliche neue Integrationen10,43s. | `.codex-tmp/storage-entity001_2-final-tests.log`, `9908d3ce389ac32fb66383af5aeab3812fe65b06a28b48d084a36572a3f177fa` |
| `cargo +1.99.0 test -p ferrumc-storage --doc --offline --locked -j2` | **10 bestanden**,0 fehlgeschlagen; darunter beide README-Offlinesignaturen und Compile-fail für BackedUp ohne Checkpoint. | `.codex-tmp/storage-entity001_2-final-doc-tests.log`, `229d517885e7ec5e35d65f41a0f6fba080bbe8681e2dc5eb9a5e6b18a8a00f3a` |
| `cargo +1.99.0 clippy -p ferrumc-storage --all-targets --offline --locked -j2 -- -D warnings` | **Grün**,keine Warnungen. | `.codex-tmp/storage-entity001_2-final-clippy.log`, `e95f6f9394a4b61f8d571b41027e9dffe24d71aba86dbbec1a02042d92af3cdb` |
| `cargo +1.99.0 fmt -p ferrumc-storage --check` | **Grün**,leerer normaler Erfolgslog. | `.codex-tmp/storage-entity001_2-final-fmt.log`, `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

Insgesamt**243** Tests grün, davon25 neue Migration-Units,3 öffentliche
Integrationstests und3 neue Rustdocfälle. Keine separate Release-,
Prozesskill-, Powerloss- oder Paper-Runtimeausführung wird daraus abgeleitet.
Nach dem vollständigen Unit-/Integrationslauf wurden nur README/INVARIANTS,
Brief und die Dokumentation/Fehlermeldung von InvalidLimits präzisiert; die
abschließenden Rustdoc-/Clippy-/Formatprüfungen verwenden diese Enddateien.

| Tatsächlicher Nachweis | Konkrete Deckung und Grenze |
| --- | --- |
| Raw9-Baseline / Raw12-Ziel | Multiworld/-dimension mit3 echten unterschiedlichen Seeds, volle Chunk/Overlay/Player/Plugin/Journal/Receiptwerte, separate generation-inclusive Backupprüfung, genau marker4→5 plus3 leere Tabellen; strikter5-Open,4/3-Openerrefusal, Wiederaufnahme vor/nach Commit. |
| Admission | Blank/plugin-only ohne Seedinferenz,4096/4097 Descriptoren, alle Catalog-/Type-/Multimaprefusals, malformed/over-limit Chunk/Overlay/Player, Chunkpos-/Koordinatendomäne, tatsächliche gültige Empty/Max-Altentities, Plugin UTF8-/Namespace-/Key-/Valuegrenzen und Journal-/Receiptranges. Vollständige Rawrefusalsnapshots. WG-001 lässt nur seine bekannte Generatorrevision zu; andere Revisionen werden nicht als kompatibel erfunden. |
| Fingerprint / Retrydelta | Presence-/Typetag-/Length-/Counteroverflowgrammatik; alle12 Tabellenkategorien. Gültige Chunk/Overlay/Player/Plugin/Generation-/Journal-/Receiptsänderungen und Entitysave/delete mit Epochhistory verweigern den alten Retry; malformed neue Locator/Index/Epochgrammatiken werden komplett validiert und nicht repariert. |
| Reale Producerübergänge |11 Sourcecutpoints,10 Backupcutpointarten,7 Checkpointpublicationarten; tatsächliche tablecreate/markerwrite/Backupcopy/Commit/closed-filehash/fsync/link/unlink-Übergänge. Vor Commit Raw4 unverändert, nach echtem Commit/lostreply CheckedResume als exakte5-Completion. |
| Commit-I/O | Ein pro Sourcebackend aktivierbarer privater Testfault lässt den realen redb-`StorageBackend::sync_data`-Callback fehlschlagen. Ergebnis ist CommitUncertain mit immutable Backup/Checkpoint; anschließender FullResume prüft4/5 oder verlangt separate Recovery. Kein erfundener Aborterfolg. |
| Checkpoint/files |344-Byte Parser-/Konstanten-/Checksum-/Countergrenzen, source-inode-binding, source-Länge ausdrücklich informational, Busylocks ohne Wartepfad, hardlink-/known-stage-/unknown-stage-/Final-no-clobber und Pathreplacementrefusals; fremde Replacementdateien bleiben erhalten. |
| Physische Scratchgrenzen | Realer SharedBackend set_len/write am Cap,+1 und checked Overflow mit unverändertem File; echte Verificationcopy-/Receiptdatabudgetrefusals und Copyexactcap; typed Budgetfehler und own Cleanup. Keine globale Diskquote oder garantierter Prozess-RSS behauptet. |
| Fehlergrenze | Directory-fsync-Publicationgrenzen sind Fehler-/Lostreplyinjektionen am echten Observerübergang, kein tatsächlicher Hostfilesystem-fsync-Ausfall. Es gibt keinen realen Host-ENOSPC-/fstat-/Prozesskill-/Powerlossversuch. Ein neues unverifizierbares File wird per Cleanupfehler erhalten; verifizierte eigene Creationfailures werden identitychecked bereinigt. |

Der13-Dateien-Freeze liegt in
`.codex-tmp/storage-entity001_2-freeze.sha256`; Artefaktchecks in
`.codex-tmp/storage-entity001_2-evidence.sha256`. Alle eingefrorenen eigenen
Dateien sind bytegleich zur isolierten Prüfung. Die fünf geänderten bisherigen
Storagedateien sind README, INVARIANTS, lib.rs, redb_store.rs und
uuid_validation.rs; dazu kommen6 neue interne Moduldateien,1 öffentliche
Integrationstestdatei und dieser Brief. Keine Lockfile-/Fremdcrate-/App-
oder alte Design-/Reviewdatei geändert. Alter3→4-Producer/Validator/Tests und
sonstige `.1`-Dateien sind gegen`9025c687` bytegleich.

Während der Umsetzung wurde eine vermeidbare Edition2024-rustfmt-Reformatierung
von14 unverändert gemeinten alten Storagefiles erkannt und vollständig aus
der veröffentlichten Rootbasis restauriert. Seitdem wird ausschließlich
`cargo +1.99.0 fmt -p ferrumc-storage` mit Edition2021 benutzt. Das finale
Diff-/Regression-/Freezeergebnis enthält diese alten Formatierungsänderungen
nicht. Es gab keine Gitmutation durch diesen Agent.

Unabhängiger Implementierungsreview und Rootintegration sind noch offen.
Keine automatische Migration/CLI, App-/Simaktivierung, neue Worldgeneration,
Gameplay-/Entitycodec oder Survival-/Paperparität gehört zu diesem Freeze.

## Gepinnte Referenz und eigene Policy

MC Java1.21.8/772; Paper1.21.8 Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Referenzmanifest aus `.1`:
`.codex-tmp/storage-entity001-reference.sha256`, SHA256
`61d43cf3e4af7ba1bc1b451ba67943b17faa8ad3b8804d436a662d33ac7d0e89`.
Vollständige relevante Methoden und Callees wurden bereits für den ursprünglichen
Entwurf und `.1` gelesen; EntityStorage/PlayerDataStorage samt Sourcepatch und
Util-Replace-/Retrycallees wurden für diesen Entwurf erneut geprüft.

| Quelle | Aussage / Grenze für `.2` |
| --- | --- |
| Vollständige Basis und applied `EntityStorage.loadEntities/storeEntities` | Entities werden chunkweise mit Position/DataVersion gespeichert; Saveflags/Passengers und Paper-Savelimits gehören der späteren Gameplay-/Codecschicht. Ein opaquer FerrumC-wire-Entitybody liefert dadurch keine autoritative UUID-/Chunkmappingmigration. |
| Vollständige Basis und applied `PlayerDataStorage.save/load`, Sourcepatch | Playerdateien sind UUIDbezogen; Tempfile, `.dat_old` und Paperkonversion sind echte getrennte Pfade. Das beweist keine gemeinsame Player-/Entitytransaction und keinen redb-Migrationscontract. |
| `Util.safeReplaceFile/safeReplaceOrMoveFile`, Renamer/Delete/Checks/Retrycallees | Paper besitzt eigene Replace-/Restoreabläufe. FerrumCs create_new-No-clobber, immutable evidence, strikte Corruptionrefusal und Hardlinkcheckpoint sind ausdrücklich eigene Offlinepolicy; keine Behauptung identischer Dateiformate oder Recoverylogik. |
| Moonrise `ChunkEntitySlices.save/saveEntityChunk0`, `NewChunkHolder.save/saveEntities` aus `.1` | Tatsächlicher Chunk-/Entity-Lifecycle bleibt separate Sim/Apparbeit. Dieser Offlineversionsschritt aktiviert keine geladenen Entities, Saveevents oder Transfers. |

Migration hat keine Bukkit/Paper-Events oder Thread-/Reentrancybridge; alle
Runtimewriter sind offline geschlossen. Keine neue Paper-Laufzeit-,
Crash-/Powerloss-, Last- oder Gameplayabnahme wurde für diesen Offlineproducer
durchgeführt. Die Source-/Backup-/Checkpointgarantien sind FerrumC-Verträge;
die konkreten Producer sind implementiert und lokal geprüft. Die unabhängige
Implementierungsabnahme ist noch offen.
