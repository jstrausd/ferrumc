# WG-004 / GEN-12.1 — Generatorbindung im produktiven Apppfad

Status: **implementiert und zur unabhängigen Abnahme eingefroren; noch nicht von Root integriert**.
Produktions-Crate: `ferrumc-app`. Producer sind unabhängig geprüft und
integriert: Migration `b299a5fc` (177 Tests auch unabhängig grün), Sim
`da61c638` (204 Tests, Review CLEAN nach unabhängiger Await-RED/GREEN-Probe),
Config `402df480`. Der Agent besitzt ab Vergabe dieses Briefing.

## Vorhandene Producer-APIs

- Config: `WorldConfig::seed() -> Option<i64>`,
  `preset() -> Option<WorldGenerationPreset>`,
  `with_generation(Option<i64>, Option<WorldGenerationPreset>)`;
  einzig unterstütztes Preset `FerrumcFlat`, TOML `"ferrumc_flat"`.
  `#[non_exhaustive]` matchen; kein Text-/Randomseed im Producer.
- Storage: `InMemoryStore::with_generation_metadata()`,
  `RedbStore::open_with_generation_metadata(path)`;
  `WorldStore::load_generation_identity` und `ensure_generation_identity`
  mit `GenerationKey::new(world, dimension)`.
- Migration: `LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero(scopes)`,
  `RedbStore::prepare_generation_migration(path, declaration)` liefert
  `Required(prepared)` oder `AlreadyCompatible(report)`;
  `prepared.create_backup(path)` und ausdrücklich hashgebundene Alternative
  `verify_existing_backup(path, [u8; 32])` liefern `backed_up.migrate()`.
  Report-/Backupgetter stehen im finalen WG-002-MIG-Taskdoc. Alle Methoden
  synchron/blocking, zuerst eigene Reports klonen/notieren, keine DBhandles.
- Sim: `GenerationBinding::new(key, identity)`, `validate_store(&dyn WorldStore)`
  und `verify_store(&Arc<dyn WorldStore>)` liefern einen opaque
  `VerifiedGenerationBinding`. `SimShard::with_generation` beziehungsweise
  `with_generation_and_inbox_capacity` verlangen deklarierte Bindung.
  `try_take_persist_dirty_verified(&Arc<dyn WorldStore>, &token, tick, limit)`
  ist synchron ohne I/O. `acquire/acquire_spawn` nehmen `&dyn ChunkGenerator`.
  Der Token ist an exakt dieselbe Arcallocation gebunden, nicht nur Dateipfad
  oder gleiche Descriptorwerte. Methodenargumente vor Wiring gegen Exports
  prüfen; keine unchecked Konstruktion oder Legacy-No-op-Drains benutzen.

## Ziel und Besitz

Der produktive Server erstellt frische Format-4-Welten und verwendet die
dauerhafte Identität für Flat-v1-Regeneration und Overlayanwendung. Alte
Format-3-Welten erhalten eine konkrete Offline-Migrationsanweisung; normaler
Start führt keine Migration und keine Seedvermutung aus. Der vorhandene
Anvilimport bindet vor seinem ersten Write die ausdrücklich deklarierte
Fallbackbasis für bisher nicht importierte Chunks. Das ist keine Übernahme
des Vanilla-Generators aus einer importierten Welt.

Besitz bei Vergabe: `server/app/**` und dieses Briefing. Keine Storage-/Sim-/
World-/Config-Crateänderung; fehlende Producer werden vor Vergabe separat
bereitgestellt. CLI, Appkonfiguration, Worldsetup, Driver und Storageworker
gehören zusammen zur App, erhalten aber klar getrennte Tests.

## Vertrag

1. Ein einzelnes Backend wird off-thread geöffnet und als derselbe `Arc` an
   Startup, Driver und Worker verteilt. redb-Öffnung/Dateiarbeit und Offline-
   Migration blockieren keinen Tokio- oder Simulationsthread.
2. Persistierte gültige Identität bestimmt den Generator. Explizit konfigurierte
   abweichende Werte scheitern vor Import, Spawn, Spielerload und Connections.
   Bei frischer Welt wird der gewählte numerische Seed einmalig gespeichert;
   ein Default darf nicht als rekonstruierter historischer Seed bezeichnet werden.
   Der Client erhält im `SpawnInfo` den originalen `BiomeManager.obfuscateSeed`
   dieser gespeicherten Identität statt eines festen Seed-Platzhalters; SHA-256-
   Byteordnung und numerische Grenzen benötigen unabhängige Java-Goldens.
3. Identität wird vor jedem World-bearing Startupwrite initialisiert/bestätigt.
   Checked `SimShard`/`LoadedChunkMap`, `ChunkGenerator` und der verifizierte
   Bindungstoken teilen Scope und Storeinstanz. Kein Legacyconstructor im
   produktiven Format-4-Pfad und kein stiller Rückfall bei Metadatafehler.
4. Tickcapture prüft den von WG-003 gelieferten opaque Token synchron ohne
   Datenbank-I/O. Vor Capture wird ein bounded Queueslot reserviert; bei Fehler
   bleiben Dirtyfacts, Journal und Mutation-IDs vollständig erhalten.
   End-of-tick, Unloadbarriere und finaler Shutdown verwenden denselben Vertrag.
5. Worker validiert die erwartete Bindung gegen seinen tatsächlichen Zielstore
   vor Writes/Retry und wahrt vorhandene bounded Retry-/Lost-Reply-/Ackregeln.
   Chunkrelease erhält erst nach Commit aller vorherigen Anfragen sein Ack.
6. Offline-CLI konsumiert den Prepare-/Backup-/Migration-Typestate. Sie zeigt
   Format, ausdrücklich erklärte Legacybasis, Scopes, Datensatzmengen und
   geprüfte Backup-/Snapshotidentität. Backup ist eine neue explizite Zieldatei;
   keine Überschreibung, kein automigrated Startup, keine automatische Reparatur.
   Malformed-/mehrdeutiger Bestand wird vor Migration abgelehnt.

## Abnahme

- Frische in-memory/redb-Welt, gespeicherte Identität und Neustart; gleiche
  Konfiguration erhält dieselbe Basis und vorhandene Overlay-/Containerdaten.
- Expliziter Seed-/Scope-/Generator-/Versionmismatch verweigert Start vor
  World-bearing Mutation; vollständiger logischer Snapshot bleibt gleich.
- Format3 normaler Start verweigert konkrete Migration; CLI Prepare/Backup/
  Migrate erzeugt vollständiges Format4; erneuter gleicher Aufruf ist geprüft
  idempotent. Fehler und aktive Sourcehandle verändern keine Daten.
- Anvilimport vor Spawn; fehlender oder malformed Import wird nicht durch Flat
  ersetzt. Import-Fallback ist als FerrumC-flat-v1 dokumentiert.
- Voller/geschlossener Storagekanal, späte Capturefehler und falsche Store-
  instanz erhalten den pending Zustand; Descriptorreject im Worker erhält
  Fehler statt Durability-Ack. Getrennte Shutdown-/Unloadbarrieren testen.
- Vorhandene App-/Java-Verträge, striktes Clippy/Format und unabhängige Client-
  Neustartharnesses auf einem kopierten unveränderlichen Binary laufen grün.

Das Ticket aktiviert eine dauerhafte Flatbasis. Natürliche Overworld, Nether,
End, Entitysimulation und vollständiges Survival bleiben nach diesem Ticket
offen. Die Quellen-/Event-/Ownershipmatrix folgt dem gepinnten
[Papervertrag](../PAPER_REFERENCE.md); kein Quellenreview wird als bestandener
Live-Papervergleich ausgegeben.


## Konkretes Appdesign und Quellen-/Fehlermatrix

| Pfad | Quelle und beobachteter Vertrag | Appownership, Fehler und Abweichung |
| --- | --- | --- |
| Startup / persistierte Basis | Vollständiger gepinnter `MinecraftServer.loadLevel/loadWorld0`: existierende WorldGenSettings vor Levelkonstruktion, fresh Einstellungen nur ohne Worlddaten; vollständiges `initWorld/setInitialSpawn/prepareLevels`. Passende Sourcepatch-Hunks vollständig gelesen, Moonrise-Featurehunks zur Levelvorbereitung gelesen. | App `spawn_blocking` öffnet genau einen Format4-Backendarc; Scope0:0 wird vor Import/Spawn bestätigt, expliziter Seedmismatch abgelehnt. Omission behält gespeicherten Seed; frische Omission wählt ausdrücklich0/flat-v1. FerrumC nutzt eigenen unveränderlichen Descriptor, festen konfigurierten Spawn und keinen Vanilla-/Paper-Generator. |
| Clientseed | Vollständiger originaler und angewandter `BiomeManager`; vollständiges `ServerPlayer.createCommonSpawnInfo`; vollständiges `PlayerList.placeNewPlayer` samt Sourcepatchkontext. | SpawnInfo erhält die Obfuscation des gespeicherten Seeds. Die tatsächliche Mojang-JAR-Methode `dnz.a(long)` liefert acht unabhängige Goldens inklusive Endian-/i64-Grenzen. Kein seed0-Platzhalter. |
| Tickcapture / Writer | Paperquellen begründen nicht FerrumCs Arc-, Codec- oder Transaktionsvertrag. Vollständiges `MinecraftServer.saveAllChunks/saveEverything/stopServer`, Moonrise-Save-/Close-/RegionIO-Hunks und vollständiger Incremental-saving Featurepatch gelesen. | App-opaque `PersistenceTarget` besitzt privaten Storearc und WG003-Token. `try_reserve` vor purem verified Capture; volle Queue deferiert ohne Journal-/ID-/Dirtytransfer. Worker prüft erwarteten Descriptor auch bei leerem Ack, vor jedem Write und Retry; Receiptpayloads und LostReply-Tokens bleiben erhalten. Finaler Driverstop gibt begrenzt ab, Workerclose drained und Fehler bleibt im Taskjoin sichtbar. Unload behält sämtliche Tickets bis allen Batchacks. Keine gemeinsame Player-/Worldtransaction. |
| Offline format3→4 | Vollständige Storagequellen, Lock-/Backup-/Save-/Restore-Callees und Dataconverter-Patch laut WG002MIG separat geprüft. | Eigenes explizites `migrate-generation` vor normaler Configinitialisierung und Netzwerkbind. Prepare-only oder neu/exklusiv verifiziertes Backup; Wiederverwendung nur mit vorher bekanntem SHA256. Keine Sourceerzeugung, Seedvermutung, Überschreibung, Reparatur oder Startupmigration. Vollständig kompatibles Format4 wird erneut komplett validiert. |
| Events / Java | `initWorld` sendet WorldInitEvent vor initialer Spawnchunkgeneration; `loadWorld0` sendet WorldLoadEvent nach Levelvorbereitung und ServerLoadEvent vor acceptConnections; `stopServer` deaktiviert Plugins vor Playersave/Worldsave. | Diese World-/Generator-/Saveevents werden mit diesem Ticket nicht neu in die Java-Brücke eingebunden; bestehende App-Java-Start-/Quit-/Shutdownverträge bleiben erhalten. Kein behaupteter Paper-Lifecyclegleichstand oder vollständiger Pluginvertrag. |

Produktions-API: `PersistenceTarget::verify(Arc<dyn WorldStore>, GenerationBinding)`
verifiziert off-tick und kapselt `VerifiedGenerationBinding`; `capture(&mut
SimShard, tick, NonZeroUsize)` ist synchron ohne I/O; `validate_write()` prüft
Writer-/Retryadmission. Keine unchecked Produktionskonstruktion. Öffnen,
Anvildirectoryscan/-decode und Offline-Typestatearbeit laufen in Blockingtasks;
DBtraitaufrufe verwenden den Storageproducer mit eigenem Blockingtransactionpfad.
Die vorhandene produktive Single-Shard-App bleibt auf ihrem bisherigen Tokiodriver;
dieses Ticket stellt keinen neuen Simthreadpool her.

Vorläufige Nachweise: `cargo check --offline --locked -j2 -p ferrumc-app`
besteht im separaten `.codex-tmp/wg004-target`; vorhandene279 Appunittests
bestanden nach Umstellung ihrer Persistence-Fixtures auf deklarierte Format4.
Weitere neue Contract- und Restartchecks sind in Arbeit. Quellenlektüre und
dieser Zwischenstand sind keine Live-Paper-Differenzialabnahme.


## Implementierte Abnahme und Freeze

- Gesamte bestehende App-Suite:374 bestandene Tests, inklusive2 Rustdoc- und2
  Binaryunittests; ein Java-Akzeptanztest war im Normallauf ausdrücklich ignored.
  Nach dem zusätzlichen Wrong-Arc-Regressionstest bestehen alle287 Appunittests
  und alle8 neuen Generation-Startup-/CLItests erneut auf finalem Runtimecode.
  Damit umfasst der final geprüfte normale Vertragsbestand375 Tests.
- Java-Akzeptanz wurde zusätzlich ausdrücklich mit echtem Java21 und kopierten,
  SHA256-erfassten Bridge-/gewöhnlichen Bukkit-Fixture-JARs ausgeführt:1 Test
  bestanden. Die Fixture umfasst Commands, Events, Permissions, Restart und
  Startuprollback; erwartete Unsupported-Schedulerzweige bleiben ausdrücklich
  unimplementiert. Bestehende Adapterfähigkeit ist damit erhalten, kein neuer
  vollständiger Paper-World-/Schedulervertrag entstanden.
- Neue Snapshotabnahme vergleicht sämtliche vorhandenen Tabellen, Katalog und
  rohe Schlüssel-/Recordbytes vor/nach Seed-, fremdem Scope-, Version-,
  Generatorkind-, Revision- und malformed Descriptorreject. Format3 normaler
  Start bleibt unverändert und nennt die Offlineanweisung. Prepare-only,
  exclusive Backup, echte Backupbytegleichheit im logischen Snapshot, Format4-
  Commit, vollständiger idempotenter Retry und anschließender Serverstart bestehen.
- CLIbinarydispatch wurde mit einem fehlenden normalen Configpfad ausgeführt:
  Offlineprepare schreibt keine Config und erzeugt kein default-world-Verzeichnis.
  Aktiv gehaltener Sourcehandle und existierendes Backup refuseieren unverändert;
  bestehendes Backup braucht den korrekten vorher erfassten SHA256. Falscher Hash
  verändert weder Source-Records noch Backupdatei.
- Reuse von stored nonzero bei Configomission, explizite fresh0-Option, original-
  JAR-Seedoracle, pure Capture ohne Metadatenread und Wrong-Arcallocation vor
  Dirty-/Journal-/IDtransfer bestehen. Volle/geschlossene Queue, late Codecfehler,
  bounded Tailcapture, Unloadacks, finaler Stop und LostReply-Receiptretry waren
  schon konkrete Appverträge und bestehen mit checked Target erneut.
- Writerdescriptorreject verweigert auch eine leere Ackbarriere. Die erste
  fehlgeschlagene Journaltransaktion und danach geänderte Descriptorfault haben
  genau1 Datenwriteversuch: der Retryvalidierer scheitert vor dem zweiten Write.
  Tests nutzen deterministische Gates bzw. pausierte Tokiodauer, keine Sleeps.
- Striktes `cargo clippy --offline --locked -j2 -p ferrumc-app --all-targets --
  -D warnings` besteht. Owned-rustfmt und scoped `git diff --check` bestehen.
  Keine Lock-/Storage-/Sim-/World-/Config-/Coreänderung dieses Tickets.

Alle eigenen Rustbuilds benutzen `.codex-tmp/wg004-target`, maximal2 Buildjobs;
der bestehende Native-Fixturetest baut seine dynamische Testbibliothek mit1 Job
in einem eigenen Repo-Disk-Verzeichnis. Das vorhandene Standardtarget bleibt
unberührt. `sha2` wurde von App-Devdependency zu Runtimeabhängigkeit verschoben,
weil originaler Biomehash es benötigt; dieselbe bereits gelockte Version,
keine Änderung von Cargo.lock.

Logs und Artefakte, relativ zum FerrumC-Repo:

- `.codex-tmp/wg004-app-tests.log` — vollständige normale Suite.
- `.codex-tmp/wg004-final-contract-tests.log` — final287 Unit +8 Startup/CLI.
- `.codex-tmp/wg004-java-tests.log` — ausdrücklich aktivierte echte Javaakzeptanz.
- `.codex-tmp/wg004-java-artifacts.sha256` — drei kopierte Runtime-/Fixture-JARs.
- `.codex-tmp/wg004-clippy.log` — striktes Alltargetclippy.
- `.codex-tmp/wg004-fmt.log` — Owned-Rustfmt.
- `.codex-tmp/wg004-freeze.sha256` — vollständiges Owned-Dateimanifest.
- `.codex-tmp/wg004-freeze/ferrumc` — unveränderliche Kopie des geprüften Binarys,
  SHA256 in `.codex-tmp/wg004-freeze/binary.sha256`.

Paper1.21.8 Build60 wurde für diese Appabnahme nicht als laufender Server
verglichen. Die echte Original-JAR-Prüfung betrifft ausschließlich die
Biomeobfuscation. Root führt die unabhängige Consumer-/Restartabnahme und
Integration durch; bis dahin bleiben Ticketstatus und Runtimefreeze getrennt
von vollständigem Survival-/Naturgeneratorfortschritt.
