# WG-002-MIG / GEN-11.2 — ausdrücklich ausgelöste Offline-Migration

Status: **lokal umgesetzt und geprüft; unabhängiges Review ausstehend**.
Ausgangsbaseline `347080f48fb2c3f480d503dbbf1e4005dd2f0b83`.
Ein Crate: `ferrumc-storage`; kein App-/Sim-Wiring, keine Startautomatik.

## Vertrag und Grenzen

Format 3 enthält keinen historischen Generatorseed. Der Operator bestätigt
ausdrücklich `LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero(scopes)`:
Minecraft 1.21.8, FerrumC-Flat Revision 1, Overworld-Geometrie; der numerische
Seed 0 ist eine **bewusste Legacy-Zuweisung**, keine rekonstruierte Information.
Es sind 1 bis 4096 eindeutige typisierte Welt-/Dimensionsscopes erforderlich.
Der Blockinhalt wird nicht als Beweis dieser Erklärung missverstanden.

Alle tatsächlich gespeicherten Chunk-, Overlay- und Entityscopes müssen in der
Erklärung stehen. Zusätzliche ausdrücklich deklarierte, bislang leere Scopes
sind zulässig. Player, Journal, Receipts (einschließlich leerer Receipts) und
Watermark besitzen keine Scopefelder: bei einem solchen Bestand ist genau ein
deklarierter Scope erforderlich. Mehrere Scopes werden dann verweigert.

Vor jeder Mutation werden der vollständige Tabellenkatalog, Tabellenarten,
bekannte Metadaten, Schlüssel und alle Records geprüft. Die bestehenden
begrenzten Recordcodecs prüfen die Recordgrammatik; opaque Entity-/Playerdaten
werden nicht als vollständig validierter Gameplayzustand ausgegeben. Plugins
behalten ihre Namespaces und Rohwerte. Für deren bisher unbeschränkte Plugin-ID
gilt im Migrationsscan ausdrücklich ein Limit von 4096 UTF-8-Bytes; Key-/Value-
Limits bleiben 512 Bytes/1 MiB. Unknown-Metadaten werden konservativ verweigert.
Journalschlüssel beginnen bei 0, sind lückenlos und stimmen mit Record-IDs sowie
Watermark überein. Receipts sind begrenzt, vollständig und überschneiden sich
nicht. Der Overlapnachweis nutzt einen temporären diskbasierten redb-Index,
nicht eine unbeschränkte Heapliste aller Receipts.

## API und Lebensdauer

```rust
RedbStore::prepare_generation_migration(path, declaration)
    -> Result<GenerationMigrationPreparation, GenerationMigrationError>

// Required(PreparedGenerationMigration) oder AlreadyCompatible(report).
prepared.report()
prepared.create_backup(backup_path) -> Result<BackedUpGenerationMigration, _>
prepared.verify_existing_backup(backup_path, expected_closed_file_sha256)
    -> Result<BackedUpGenerationMigration, _>
backed_up.report()
backed_up.backup()
backed_up.migrate() -> Result<GenerationMigrationOutcome, _>
```

Alle Methoden sind synchron und gehören ausschließlich auf eine Offline-/
Blocking-Lane. Kein Raw-DB-Handle verlässt das Crate. Prepare hält einen exklusiv
geöffneten redb-Sourcebestand bis Backup und Commit. Normale Storeöffnungen und
andere redb-Prozesse können in diesem Zeitraum nicht schreiben. Dateiidentität
und Pfad werden vor Backup und Migration erneut geprüft; finale Symlinks und
ausgetauschte Pfade werden verweigert. Belastbare Inodeprüfung und Directory-
fsync werden zunächst nur auf Unix unterstützt; andere Plattformen werden
klassifiziert verweigert, nicht scheinbar abgesichert.

Der Report enthält deklarierte Scopes/Baseline, Source-/Zielformat, Source-
Dateigröße, Tabellenanzahlen, logische Raw-Byteanzahl und einen inkrementellen,
längen-/tabellengetrennten SHA-256-Fingerprint. Es gibt keine vollständige
Heapkopie der Welt. Source- und Backupcache sind auf je 64 MiB, der temporäre
Receiptindex auf 16 MiB begrenzt; Recordcodecs gelten zusätzlich. Diese Caps
sind keine Garantie für den Gesamt-RSS des redb-Backends oder decoded Records.

## Backup, Commit, Retry und Rollback

Das Backup ist ein **logisch byteerhaltender redb-Snapshot** in einem separat
`create_new` erzeugten File. Alle ursprünglichen Raw-Key-/Valuebytes werden
streamend kopiert; Records werden nicht decoded/reencoded normalisiert.
Metadaten sind typisiert und behalten dieselben Werte. Ein vorhandener Pfad
wird niemals überschrieben. Nach Commit/Close, File- und Directory-fsync wird
das Backup vollständig gegen denselben Snapshotfingerprint geprüft; anschließend
wird der SHA-256 des geschlossenen Files ermittelt. Der Backupfilelock bleibt
auch über Close/Prüfung/Hash hinweg gehalten. Vor Sourcecommit wird dieser
Fingerprint erneut geprüft. Eine bewusst wiederverwendete Datei benötigt ihren
zuvor notierten closed-file SHA-256 sowie exakt gleichen logischen Bestand.

redb selbst verändert beim Open/Close unter Umständen Recovery-/Allocator-
Haushaltsbytes. **Physische Filehash-Gleichheit ist kein Source-Erhaltungsvertrag.**
FerrumC-Tabellen, Datensätze und Formatmarker bleiben bei Prepare, Refusal oder
vor Migration logisch unverändert. Vorbestehender Recoverybedarf wird anhand
des begrenzten 320-Byte-Header-/Geometriechecks (redb 2.6.3, Defaultseiten 4096
Bytes; fremde Customgeometrie wird verweigert) und des
abbrechenden Repaircallbacks klassifiziert verweigert; es gibt keine automatische
FerrumC-Datenreparatur.

Unter der echten Source-Write-Transaction wird der komplette Bestand erneut
geprüft und der Preparefingerprint verglichen. Generationstabelle, sämtliche
27-Byte-Identitäten und Formatmarker 4 werden gemeinsam committed. Keine
bestehenden Recordbytes, Journal-IDs, Journalreihenfolge, Receipts oder sonstigen
Metadatenwerte werden verändert. Der normale Startpfad migriert weiterhin nicht.
Ein Retry auf Format 4 liefert `AlreadyCompatible` nur nach komplettem Scan und
exakt gleicher vollständiger Descriptor-/Erklärungsmenge; fehlende oder fremde
Descriptors sind kein Erfolg. Atomare cutpoints müssen vollständiges Format 3
oder vollständiges Format 4 zeigen.

Ein fehlgeschlagener oder abgebrochener Backupversuch kann sein eigenes File
hinterlassen; es wird nicht blind entfernt oder überschrieben. Der Operator
kann einen neuen Backuppfad wählen oder ein vollständiges Backup mit bekanntem
Hash ausdrücklich erneut prüfen. Eigene temporäre Receiptindexfiles werden
nach erfolgreicher/fehlgeschlagener Validierung gezielt entfernt; nach einem
Prozessabbruch können solche eindeutig benannten Files zurückbleiben.

Rollback ist eine **manuelle Offline-Restaurierung**: alle FerrumC-/redb-Handles
schließen, den zu ersetzenden Stand separat sichern, verifiziertes Format-3-
Backup in eine neue Zieldatei kopieren, Kopie vor Aktivierung prüfen und erst
dann den Speicherpfad austauschen. Keine automatische Überschreibung nach einem
unklaren Commit. Das Backup bildet den geprüften Stand vor Migration ab, keine
späteren Spielmutationen. Prozesskill-/Stromausfallnachweis und Operator-CLI
bleiben separate Tickets; Transaktionscutpoints ersetzen diese Abnahme nicht.

## Gelesene Referenz und Abweichungen

Paper 1.21.8 Build 60, Commit `29c8822d90899c89d2689338e81a98f690bcba12`:
vollständige `LevelStorageSource.getLevelDataAndDimensions`,
`LevelStorageAccess`-Konstruktor/`checkLock`, `saveDataTag`/`saveLevelData`,
`makeWorldBackup`, `restoreLevelDataFromOld`; vollständige
`PrimaryLevelData.createTag`/`setTagData`; `Util.safeReplaceFile`/
`safeReplaceOrMoveFile`. Sourcepatches beider Storageklassen und relevante
LevelStorageSource-Hunks des Dataconverter-Featurepatches wurden mitgelesen.
Paper persistiert vorhandene WorldGenSettings, hält einen Worldlock, erstellt
getrennte Backups und besitzt explizite Restorewege. FerrumC verwendet eigene
redb-Tabellen und eine bewusst erklärte Legacybindung statt NBT/DataFixer.
Hier werden keine Gameplay-/Plugin-Events ausgelöst. Dies ist Quellenprüfung,
kein bestandener Paper-Differenziallauf und kein vollständiges Survival.

## Dateibesitz und Abnahme

Eigene Dateien: Storage `Cargo.toml`, `src/lib.rs`, `src/codec.rs`,
`src/record.rs`, `src/redb_store.rs`, neues `src/redb_store/migration.rs` samt
Validation-/Testmodulen, README/INVARIANTS und dieses Dokument. Nur der
Storage-Dependencyeintrag von `server/Cargo.lock` erhält vorhandenes `sha2`;
Packageversionen bleiben unverändert. Board und App/Sim bleiben beim Orchestrator.

Bestanden: **177 Storage-Tests** (144 Unit, 7 Metadata-Integration, 2 öffentliche
Migrationcontracts, 13 redb-Roundtrips, 9 InMemory-Roundtrips, 2 Rustdoc).
Davon 28 neue private Migrations-/Validationtests und 2 öffentliche Contracts:

1. Alle Tabellen/Schlüssel/Records, Scopevollständigkeit, jeder unscoped Typ
   einzeln, mehrere scoped Welten und ausdrücklich leere Scopes.
2. Fehlende/fremde/multimap/falsch typisierte Tabellen, fehlende/fremde Marker,
   Unknown-Metadaten, truncated/überlange Schlüssel und Records, Bytecaps,
   column mismatch sowie unsupported Descriptorwerte/Seeds.
3. Journal-ID/Record-ID/Watermark, lückenlose Sequenz, malformed/overflow/
   over-limit/fehlende/überschneidende Receiptranges und Indexcleanup.
4. Exakte 4096-Scopegrenze einschließlich Commit/Retry; infinite iterator
   stoppt beim 4097. Element. Plugin-ID-/Key-/Valuegrenzen in UTF-8-Bytes.
5. Der Rawbyte-Erhalt verwendet bewusst eine gültige dichte All-Air-Sektion,
   deren Reencode anders wäre: Source nach Migration und separates Backup
   behalten ihre ursprünglichen Bytes, Schemas und Journalreihenfolge.
6. Unabhängige konkurrierende Storeöffnung scheitert bei gehaltenem Source-/
   Backuphandle; keine Sleeps. Vorhandene und fremde Backups, falsche Hashes,
   unpassender logischer Bestand, finale Symlinks/Hardlinks und ausgetauschte
   Sourcepfade werden verweigert. Source-/Backupänderung während Staging wird
   unmittelbar vor Commit nochmals geprüft und vollständig abgebrochen.
7. Shared production cutpoints nach Tablecreate, nach erstem/zweitem Descriptor
   und nach Markerwrite: Abort/Drop erhält vollständiges Format 3. Erfolgreicher
   Commit mit verlorener Antwort: kompletter Format-4-Retry ohne Dopplungen.
8. Nicht vorhandene/leere/fremde Quellen werden nicht erstellt/normalisiert;
   dirty Header, malformed 320-Byte-Header und unmögliche/abweichende Geometrie
   werden vor redb-Recovery verweigert. Der private Indexdecoder und alle
   Reportcounter haben Malformed-/Max-/Overflowfälle; Pfadcleanup löscht keine
   inzwischen fremde Ersatzdatei.

Logs: `/tmp/wg002-mig-red.log` (bewusst fehlende neue API),
`/tmp/wg002-mig-storage-tests.log` (volle grüne Suite),
`/tmp/wg002-mig-clippy.log` (strict all-targets),
`/tmp/wg002-mig-fmt.log` (eigene Dateien).
Fsyncnachweis: Produktionspfad `backup commit -> Database close -> File sync_all
-> Parent directory sync_all -> vollständiger Scan -> Database close -> File /
Directory sync_all -> closed-file SHA-256`, bei exklusiv bleibendem FileBackend.
Dies prüft erfolgreiche Rückgaben/Einbindung der OS-Fsyncs; ein Stromausfall-
oder injizierter OS-Fsyncfehler ist **kein** bestandener Nachweis dieses Tickets.
Physische redb-Housekeepingbytes und Prozesskill bleiben ausdrücklich getrennt.
