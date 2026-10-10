# WG-004 / GEN-12.1 — Generatorbindung im produktiven Apppfad

Status: **vergeben an `select_world_ticket`, Umsetzung beginnt**.
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
