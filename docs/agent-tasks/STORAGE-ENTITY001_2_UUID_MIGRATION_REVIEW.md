# STORAGE-ENTITY001.2 — Unabhängiger Implementierungsreview

Reviewer: `feature_catalog`, 10.10.2026. Root besitzt Git und App; dieser
Review schreibt ausschließlich den eigenen Report. Temporäre Runtimeprobes
wurden nur im isolierten Checkout installiert und anschließend bytegenau
restauriert.

## Ergebnis

**CLEAN** für den eingefrorenen, ausdrücklich offline verwendeten
Format4→5-Migrationsproducer. Keine offene Runtimebeanstandung, keine
Runtimekorrektur angefordert. Vollständige unabhängige Debugsuite: **243/243**;
striktes AlltargetClippy und Fmt grün. Roots unabhängige vollständige
Releaseprüfung desselben Storageinhalts: **243/243**. Zusätzlich bestehen fünf
eigene Runtimeprobes und drei erwartete negative Authority-Compilerprobes.

Der Producer erfüllt den reservierten R4-Vertrag: vollständige tatsächliche
Format4-Baseline, unveränderte geschlossene Backupevidenz, atomarer
Checkpointproducer, eine echte Immediate-Writertransaction mit genau
Marker4→5/3-empty-table-Delta und vollständige checkpointgebundene
Wiederaufnahmeprüfung. Ein generischer Format5-Open autorisiert keinen Retry.

**F-doc, erledigt:** Der ursprüngliche Schluss des Briefs behauptete noch,
die Producer seien nicht implementiert. Root korrigierte ausschließlich diesen
Satz und aktualisierte den Freeze. Die übrigen zwölf eingefrorenen Dateien
blieben bytegleich. Die korrigierte Aussage entspricht dem geprüften Code;
der Review bezieht sich ausdrücklich auf diesen neuen Freeze.

Diese Abnahme aktiviert keinen Serverconsumer und beendet weder DUR-03 noch
Survival, Weltgeneration oder Java-/Paper-Kompatibilität. Prozesskill,
Powerloss, Host-ENOSPC und Live-Paper-Differenzialabnahme sind nicht ausgeführt.

## Eingefrorener Stand und Reviewbesitz

Verbindlicher Brief:
`docs/agent-tasks/STORAGE-ENTITY001_2_UUID_MIGRATION.md`, SHA256
`5234a93e800e3dcfdfeb7f4bcf10239566ec0d49667f6bcb25f181131dd6e958`.
Verbindlicher 13-Dateien-Freeze:
`.codex-tmp/storage-entity001_2-freeze.sha256`, Manifest-SHA256
`1e2a0c0c7bc26378fcee87556c90fa3f79661d6a8a62174bff9fdec423c3c101`.

Der ursprüngliche Autorenfreeze bleibt getrennt als
`.codex-tmp/storage-entity001_2-author-freeze.sha256`, SHA256
`d93374c6a9e1e5ebeb5bca215f197d592b141d4ac0c58e92f8dc013b2c04fa17`.
Die ausschließlich dokumentarische Rootkorrektur ist in
`.codex-tmp/storage-entity001_2-root-doc-correction.json` festgehalten.

Eigener Reviewcheckout:
`.codex-tmp/storage-entity001_2-isolated`, a5d11819-Basis mit Roots separater,
inzwischen als9025c687 integrierter expliziter `&[u8]`-Typisierung im .1-Test
und exakt den 13 Freeze-Dateien. Eigener Targetpfad:
`.codex-tmp/storage-entity001_2-review-target`. Roots Releasecheckout verwendet
d9feaa82 plus dieselben 13 Freeze-Dateien und einen separaten Targetpfad.
Kein Git-/Worktree-/Lockfile-/App-/Productionedit durch den Reviewer.

Nach allen Probes wurden alle 13 Dateien in Main und im isolierten Checkout
gegen den aktuellen Freeze geprüft. Weitere **26 Storage-Dateien** sind gegen
9025c687 bytegleich; der gesamte vorhandene Testmodulsuffix von
`uuid_validation.rs` ebenfalls. Der alte 3→4-Producer, seine Validation und
Tests bleiben unverändert. Der finale Integritätsrecord liegt in
`.codex-tmp/storage-entity001_2-review-integrity.json`.

## Code-, Vertrags- und Quellenprüfung

Vollständig gelesen: `uuid_migration.rs`, sämtliche neuen
`backup/checkpoint/files/validation/tests`-Module, die öffentliche
`tests/uuid_entity_migration.rs`, alle geänderten Teile des bestehenden
`uuid_validation.rs`, APIexports/Modulregistrierung und Storage-README sowie
INVARIANTS. Der Defaultvalidator und seine alten Tests wurden zusätzlich
gegen die .1-Basis verglichen. Lokale Agentregeln, Storagearchitektur,
ADR0004/0007, der Schemaentwurf und insbesondere R4/DUR-03 des Designreviews
gehören zur geprüften Grundlage.

| Grenze | Prüfergebnis und konkrete Evidenz |
| --- | --- |
| API/Authority | Private Felder; die synchronen Offline-APIs tragen Locks und unveränderliche Limits durch Prepared→BackedUp→Checkpointed. Nur Checkpointed besitzt `migrate`; Reports erzeugen keine Authority. Source-Database wird vor Freigabe ihres letzten Backendlocks geschlossen. Eigene Compilerprobes bestätigen fehlenden Commit für Prepared/Reportclone und fehlendes Clone für Checkpointed. |
| Format4-Catalog | Genau neun normale Tabellen und tatsächliche Key-/Valuetypen; keine Multimap, fremde oder partielle Tabelle. Metadaten erlauben nur format_version und optional last_mutation_id. Erste vorhandene wire-Entityrow verweigert unabhängig vom Body; keine UUID-/Chunkinferenz, Löschung oder Quarantäne. |
| Vollständiger Format4-Scan | Alle Rows mit bestehenden bounded Codecs, key/record-pos-Domänen und tatsächlichen Generationdeskriptoren; Cap4096, keine neue Flat-/Seed0-Deklaration. Unscoped Player/Journal/auch Emptyreceipt verlangen einen wirklichen Descriptor. Plugin UTF-8-Namespace4096, Key512, Value1MiB; Journal ab0 kontiguierlich, ID=Key, genauer Watermark, Receiptform/-ranges/-nonoverlap. Blank/plugin-only erlaubt ohne Descriptorinferenz. |
| Eigene Fingerprintgrammatik | Versionierter Domainseparator, zwölf feste Slots, Tabelle/Name/Typ/Präsenz, Rowlängen/Rawbytes und Counts. Abwesend und leer-vorhanden unterscheiden sich. Duale Format4/erwartete5-Projektion ändert ausschließlich Marker und drei Präsenzbytes. Checked Count-/Byteaddition; keine Umdeutung des .1-Startupfingerprints. |
| Backup | Generation-inclusive Rawkopie aller neun Tabellen, kein Decode/Reencode zur Speicherung. Fresh nur create_new, echte Commit-/Close-/File-/Directory-sync und closed Physical-SHA. Reuse benötigt remembered SHA und vollen Logicalsnapshotvergleich. Das recorded Original wird niemals redb-geöffnet; vollständige Validation erfolgt auf genau einer budgetierten physischen Copy. Eigene fremde-Backup-Probe verweigert auch bei richtigem Physical-SHA und gleichen Counts/Längen. |
| Files/Locks | Kanonischer Parent, kein finaler Symlink, regular/nonempty, identity und gepinnte redb-Header/Geometry-/Recoveryguards vor Open. Source/Backup/Copy nlink1 und getrennte Rollen; neue Locks fail-fast Busy. Keine automatische Recovery. Cleanup nach Handleclose nur bei geprüfter eigener device/inode; fremde Ersetzungen bleiben erhalten. Advisorylocks sind kein Schutz gegen fremde Rawwriter außerhalb des Vertrags. |
| Checkpoint | Genau344 Bytes, Body312+SHA32, versionierte Schema-/Identity-/Fingerprintfelder, Reserved0, Capturedsource/backup/12Counts/Rawbytes/4- und5-Hash sowie own stagingUUID. create_new→write→fsync→readback→hardlink no-clobber→dirsync→ownunlink→dirsync. Nur genau bekannte ownpair darf nlink2 besitzen; kein Globcleanup oder fremdes Delete. Checksum ist Parserintegrität und ersetzt keine vollständige Wiederaufnahmeprüfung. |
| Transaktionsdelta | Echte einzelne redb-WriteTransaction, ausdrücklich `Durability::Immediate`; nur drei leere UUIDindextabellen und Marker4→5. Staged5 wird mit vollständigem Catalog/Rows/Relations geprüft, anschließend bounded Rawvergleich gegen das volle Backup4. Späte Source-/Backup-/Checkpoint-/Cleanupguards vor dem tatsächlichen Commit. Returned errors vor Commit erhalten die ganze Raw4-Baseline, nicht nur den Marker. |
| Commit-I/O | Der echte redb-sync_data-Callee kann im privaten Testbackend fehlschlagen; der Producer meldet CommitUncertain. Ein operationaler Commitfehler wird nicht als garantierter Abort umgedeutet. Resume prüft erneut4/5 oder verweigert Recovery-required Source. |
| Resume4/5 | Gleiche Sourceinode, immutable recorded Backupidentity/Physicalhash und unveränderlicher Checkpoint. Source4 muss volle Capturedbaseline erfüllen; Source5 zuerst vollständigen strikten12-Tablecheck, dann genaue erwartete Delta, Rawcommonrows und leere Entity-/Locator-/Index-/Epochtabellen. Sourcefilelänge/-Physicalhash sind informational; redb-Housekeeping ist erlaubt. Gespieltes, sonst gültiges5 verweigert Retry, auch nach Entitydelete mit historischer Epoch. |
| Cleanupreihenfolge | Voller Sourcecatalog/Scan/Delta vor Stabilisierung eines bekannten Checkpointaliases. Eigene Source4/5-Probe mit zusätzlicher normaler Tabelle bestätigt: Refusal vor jedem fsync/unlink-Observer, knownstage/nlink2 und Backupevidenz erhalten; kein beschädigter Store wird durch Cleanup vorab normalisiert. |
| Ressourcen | Zwei ausdrücklich positive Budgets, overflowfreie Summe+344. Bounded Backend verweigert jedes set_len und checked offset+len über Cap vor Delegation; spezifischer Budgetcause bleibt erhalten. Höchstens eine Verificationcopy/ein Receiptindex, sequenzielle Scans,64KiB Copy-/Hashbuffer, bounded Descriptor-/Rowbestand. Cachepeak128MiB+16MiB Receiptcache; decoded-/redb-/OS-Overhead zusätzlich. Kein Ganzwelt-Vec, kein totaler RSS-/Hostdiskquotenclaim. |
| Regression | Alte 3→4-Validation/Producer/Tests bytegleich; .1-Defaultgrammatik, Fingerprint und Errorcodes erhalten. Der neue Migrationeintritt prüft Catalog und Scan zusammen und besitzt ausschließlich seinen eigenen bounded Receiptbranch. Runtimeopener, App/Sim und bestehende Format5-Entitywriter werden nicht aktiviert oder als Transfers autorisiert. |

Gepinnte originale Referenz: Minecraft Java1.21.8/Protokoll772,
Paper1.21.8 Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`.
Referenzmanifest `.codex-tmp/storage-entity001-reference.sha256`, SHA256
`61d43cf3e4af7ba1bc1b451ba67943b17faa8ad3b8804d436a662d33ac7d0e89`;
alle zwölf darin gehashten Dateien nochmals bestätigt.

Erneut gelesen wurden vollständige Basis und angewandte
`EntityStorage.loadEntities/storeEntities` und `PlayerDataStorage.save/load`
samt vollständigem Sourcepatch; `Util.safeReplaceFile/safeReplaceOrMoveFile`
mit Renamer/Deleter/created/deletedChecks/executeInSequence/runWithRetries;
Moonrise `ChunkEntitySlices.save/saveEntityChunk/saveEntityChunk0/copyEntities`
und `NewChunkHolder.save/saveEntities`; der vollständige Featurepatch0022 für
Entitysave-/Loadlimits, der betreffende DataConverterpatch und
`PaperHooks.modifySavedEntities/forceNoSave`.

Paper speichert chunkweise Entities und UUIDweise Player auf getrennten Pfaden.
Diese Quellen beweisen weder eine gemeinsame Player↔Entity-Transaction noch
ein UUIDmapping aus FerrumCs opaken Alt-Wirebodies. Saveflags, Passengers,
Savelimits und DataVersion bleiben spätere Gameplay-/Codecverträge. FerrumCs
no-clobber Backup/Checkpoint, strikte Refusals und redb-Schema sind eigene
Offlinepolicy und keine behauptete Paper-Recovery-/Dateiformatparität.

Für den tatsächlichen Commit-/Lockcallee wurden außerdem die gepinnten
**redb2.6.3**-Quellen gelesen: vollständiger Unix-FileBackend mit
`flock(LOCK_EX|LOCK_NB)` und Dropunlock; `WriteTransaction.set_durability`,
`commit/durable_commit`, PageManager `commit/commit_inner` mit Headerflip,
Flush und needs_recovery bei I/O-Fehler sowie CachedFile
`flush_write_buffer/flush`, das reale Writes und `backend.sync_data` aufruft.
Das stützt die Zuordnung zum echten Producer; es ersetzt keinen Kill- oder
Powerlossversuch.

## Ausgeführte Abnahme

Rust/Cargo/Clippy/rustfmt1.99.0, Workspaceedition2021, `--offline --locked`,
höchstens zwei Compilejobs und `--test-threads=2`. Alle eigenen Checks liefen
auf dem isolierten eingefrorenen Storage mit eigenem Target.

| Check | Ergebnis | Log |
| --- | --- | --- |
| Eigene vollständige Debugsuite: `cargo +1.99.0 test -p ferrumc-storage --offline --locked -j2 -- --test-threads=2` | **243 bestanden**,0 fehlgeschlagen/ignoriert:192 Unit+41 Integration+10 Doctests. Unitzeit211,08s. | `.codex-tmp/storage-entity001_2-review-debug.log` |
| Eigene strict Clippy: `cargo +1.99.0 clippy -p ferrumc-storage --all-targets --offline --locked -j2 -- -D warnings` | Grün, keine Warnungen. | `.codex-tmp/storage-entity001_2-review-clippy.log` |
| Eigene Fmt: `cargo +1.99.0 fmt -p ferrumc-storage --check` | Grün, leerer Erfolgslog. | `.codex-tmp/storage-entity001_2-review-fmt.log` |
| Roots vollständige Releaseprüfung desselben Freeze | **243 bestanden**,0 fehlgeschlagen/ignoriert:192 Unit+41 Integration+10 Doctests. Unitzeit12,92s. Vom Reviewer vollständigen Log geprüft; kein eigener zweiter Releaseclaim. | `.codex-tmp/storage-entity001_2-root-release.log` |
| Eigene zusätzliche Runtimeprobes: `cargo +1.99.0 test -p ferrumc-storage --lib independent_review --offline --locked -j2 -- --test-threads=2` | **5 bestanden**,0 fehlgeschlagen;40,29s. | `.codex-tmp/storage-entity001_2-review-probes.log` |
| Eigene negative rustc-Authorityprobes | **3 erwartete E0599**; keine Import-/Buildfehler als vermeintliche Refusal gezählt. | `.codex-tmp/storage-entity001_2-review-authority.log` und `.codex-tmp/storage-entity001_2-review-authority/*.stderr` |

Die originalen 243 Tests decken die bestehenden Storage-/WG-002-MIG-/UUID-
Regressionen und die neuen Migrationtests ab. Die fünf zusätzlichen Probes
sind separate Reviewerfälle; sie sind kein Bestandteil dieses ursprünglichen
243-Zählers. Die drei negativen Compilerfälle sind keine Runtime- oder
Doctestzählung. Wegen unveränderter Runtimebytes und Roots vollständiger
Releaseabnahme wurde kein pauschaler weiterer Releasewiederholungslauf erzeugt.

## Zusätzliche unabhängige Probes

Quelltext ausschließlich
`.codex-tmp/storage-entity001_2-independent-probes.rs`, SHA256
`949af5ce14f029c251897e52dedc27bc54106702ef6ee665e92b30638bd522f5`.
Er wurde nach der originalen Abnahme nur an das isolierte Migrationtestmodul
angehängt. Nach erfolgreichem Lauf wurde dieses aus den gespeicherten
Originalbytes restauriert; Main erhielt keine Probetests.

| Eigener Fall | Geprüftes Verhalten |
| --- | --- |
| Korrekt neu checksummierte falsche Checkpointevidenz | Byteflip in logischen Bytes an absolutem Offset208, Baseline4hash216 und Target5hash248, jeweils+7, anschließend korrekte SHA des312-Byte-Bodys. Parser akzeptiert Grammatik; volle Source4-Resumeprüfung verweigert alle mit SourceChanged. Source5:216→BackupMismatch,208/248→MigrationDeltaMismatch. Komplette raw Source und immutable Backupbytes erhalten; falscher Checkpoint wird nicht repariert. Wiederherstellung der Originalevidenz ermöglicht Ready/AlreadyCompleted. |
| Fullcatalog vor known-alias Cleanup | Echter CheckpointLinked-Cutpoint erzeugt ownstage/nlink2, dann zusätzliche leere Tabelle `independent:unexpected`. Source4→CatalogMismatch, Source5→Validation; kein Stabilization-/fsync-/unlink-Observer läuft. Knownstage/nlink2/Backupbytes erhalten. Nach ausdrücklicher Testkorrektur der Tabelle echte Resume4/5 erfolgreich, nlink1 und ownstage entfernt, unveränderte Checkpointbytes. |
| Fremdes geschlossenes Backup mit richtigem SHA | Gleiche Tabellen/Counts/logische Länge, aber unterschiedliche Pluginrawvalue1/2 unter demselben Key. Korrekt gehashtes fremdes Backup wird mit BackupMismatch verweigert; Source und Originalbackupbytes bleiben unverändert. Physical-SHA-/Counts-only reichen nicht zur Autorisierung. |
| Late content guard an echtem BeforeCommit | Nach vollständiger staged5-Prüfung schreibt der reale Observer ein Byte in das bestehende Backup bzw. den Checkpoint bei gleicher inode/Len. BackupMismatch bzw. CheckpointChanged; ganze Source4 einschließlich Catalog abortiert. Die vom Test-Rawwriter geänderte Evidenz wird nicht repariert/gelöscht. Daraus folgt keine physische Backupimmutabilität gegenüber einem fremden Rawwriter. |
| Jede Parserlängengrenze | Alle344 Präfixlängen0..343 und Suffixlängen1/32/344 werden als CheckpointMalformed("length") verweigert; Originalevidenz bleibt erhalten. |

Die eigenständigen Compilerquellen und ihre vollständigen stderr liegen unter
`.codex-tmp/storage-entity001_2-review-authority/`:

- `report_clone_no_commit.rs`: `UuidEntityMigrationReport.clone().migrate()`
  fehlt mit E0599.
- `prepared_no_commit.rs`: `PreparedUuidEntityMigration.migrate()` fehlt
  mit E0599.
- `checkpointed_no_clone.rs`: `CheckpointedUuidEntityMigration.clone()`
  fehlt mit E0599.

Die Probes verwenden echte productiongenutzte Cutpoints/Validatoren und
Snapshots. Die Parserprobe prüft die echte implementierte Bytegrammatik;
kein separates Testmigrationsmodell ersetzt den Producer.

## Konkrete nächste Survivalgrenze: DUR-03

Dieser Review gibt ausschließlich STORAGE-ENTITY001.2 frei. Der nächste sichere
Schritt für Drop/Pickup ist ein begrenztes, separat versioniertes
**Player↔UUIDEntity-CAS/Tombstone/Receipt-Producerticket**, bevor ein Online-
Consumer aktiviert wird. Der vorbereitete Brief
`DUR-03_1_PLAYER_ENTITY_TRANSFER.md` erhält durch diesen Review keine
Produktionsfreigabe. R4 und DUR-03 im bestehenden Designreview bleiben bindend.

| Wiederverwendbarer Seam | Erforderliche neue Writergrenze |
| --- | --- |
| `persistent_entity/engine.rs`: EntityRead, require_identity, lookup/verify, prepare, bounded Plan/Change und distinct Chunkepochpreflight | Die bestehenden UUID-/Generation-/Encountered-/Epochprüfungen intern wiederverwenden. UUID, Chunkeepoch und generationaler Simhandle sind keine Ownershiprevision; monotone private Player-/Entityrevisionen samt Tombstones zusätzlich benötigen. |
| `redb_store/uuid_entities.rs`: write_reader und apply_plan auf geliehener echter WriteTransaction | Ein neuer gemeinsamer Writer kann intern Entityplan/Indizes/Epochen in derselben Transaktion wie Playerheader/-record, Ownershiprevisionen und Receipt anwenden. Der heutige commit_plan committed nur den Entityplan; nicht separat aufrufen und anschließend Player-save als vermeintlich atomaren Transfer verwenden. |
| `redb_store.rs`: save_player/delete_player | Heute jeweils eigene unscoped Blindwritertransactions. Im ausdrücklich neuen Ownershipmodus versionsgeführt umstellen oder Unsupported; auch alle konkurrierenden UUIDsave/delete/move/Batchwriter müssen diese Grenze einhalten. Ein bloßer Revisionincrement bei einem späteren Blindwrite genügt nicht. |
| `memory.rs`: Playerstate und separater UUIDentitystate unter mutation_boundary | Ein gemeinsamer Readguard der Mutationsgrenze ist keine gemeinsame Player↔Entityatomarität. Neues exklusives/vereinheitlichtes bounded Ownerplan-Apply, vollständige Vorprüfung vor Mutation und gleiche CAS-/Receipt-/Revisionsemantik wie redb nötig. |
| Vorhandene .1/.2-Format-/Admission-/Validatorproducer | Eigene explizite versionierte Ownershipgrammatik, Fresh/Strict-Modus und reserviertes Migrationspaket. Format5 nicht still um Transfermetadaten/-tabellen erweitern. Bisher unscoped Player dürfen keinem geratenen Descriptor zugeordnet werden. |

Erster Umfang: genau ein Player und eine Entity im ausdrücklich geprüften
gleichen Scope, bounded Request/Plan/Receipt. Fresh-never-seen-Abwesenheit
muss von versionierter Tombstoneabwesenheit unterschieden werden; Delete/
Recreate darf keine ABA-Aliasierung oder Revisionreset bewirken. U64exhaustion
verweigert, niemals wrappt.

Receiptlookup erfolgt **vor live CAS**: gleicher Transfer und gleiche Request-
Identität liefert das ursprüngliche Ergebnis trotz später veränderter live
Revisionen; gleiche ID mit anderer Identität verweigert typisiert. Bounded
Canonicalbytes gegenüber kryptografischer SHA-Identität sowie sichere
Retention/Replaygrenzen müssen vor Code ausdrücklich reserviert werden.
Storage prüft opaque Snapshots und Atomizität; der Simconsumer prüft
Itemconservation. Container-/Overlay-, Hopper-, Merge-, Crossdimension- und
Crossshard-Transfers bleiben spätere DUR-03-Erweiterungen.

DUR-04 benötigt exklusive Simauthority, Reservationen, bounded Pending und
Lostreply-/Disconnect-/Unload-/Shutdownbehandlung; DUR-05 denselben Backend
und Ack-before-visible einschließlich Dirty/Pendingretention. Vorhandene
Itemtransferprimitives, sealed SIMstate, UUIDstorage und diese Offlinemigration
allein autorisieren kein sicheres Online-Drop/Pickup. Javaevent-Reentrancy und
Pluginmutationen bleiben eigene Consumerverträge.

## Evidenz und finale Integrität

Eigenes Artefaktmanifest:
`.codex-tmp/storage-entity001_2-review-evidence.sha256`. Es erfasst diesen
Report, Debug-/Clippy-/Fmt-/Probe-/Authority-/Rootreleaselogs, Probequellen und
Compilerstderr, Originaltest-Restorebytes, beide Freezemanifeste,
Rootdokumentkorrekturrecord und finale Integritätsrecords. Das Manifest wird
nicht selbstreferenziell gehasht; sein SHA256 und der endgültige Reporthash
werden Root mit der Abschlussmeldung übergeben.

Der abschließende Check nach Reporterstellung bestätigt nochmals alle13
Freezedateien sowohl in Main als auch isoliert, aktuellen Manifest-/Briefhash,
26 unveränderte übrige Storage-Dateien und den alten Validationtestsuffix.
Alle zwölf Paperreferenzdateien sind erneut bestätigt. Eigene Probecalls
ließen nach Restore keine Runtimeänderung zurück; kein weiterführendes Ticket
wurde während dieser unabhängigen Prüfung implementiert.
