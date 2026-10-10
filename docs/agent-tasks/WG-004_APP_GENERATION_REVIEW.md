# WG-004 — unabhängige App-Review

Status: **CLEAN für den eingefrorenen WG-004-/GEN-12.1-Appvertrag**.
Reviewer: `item_transfer_primitives`, 2026-10-10. Unabhängige Quellenprüfung und
nach dem finalen Runtimefreeze ausgeführte Vertragsprüfung; keine bestätigten
Findings. Schreibbesitz ausschließlich dieses Reports. Produktionsdateien,
Producer, Lockfile, Git und Board wurden durch den Reviewer nicht geändert.
Root besitzt Integration; `select_world_ticket` besitzt die Appimplementation.

## Geprüfter Stand

Das Owned-Manifest `.codex-tmp/wg004-freeze.sha256` wurde vor und nach den
unabhängigen Checks vollständig geprüft: **26/26 identische Dateien**.
Auch die drei kopierten Javaartefakte aus
`.codex-tmp/wg004-java-artifacts.sha256` blieben unverändert.

Eingefrorenes Binary: `.codex-tmp/wg004-freeze/ferrumc`, SHA256
`7333b12585c321f24b7fd4b3aa7138edb320f90265547795a513a1247139ff0b`.
Der Reviewer prüfte dessen Hash, führte seine Rustverträge nach ausdrücklicher
Freigabe des Implementers im separaten `.codex-tmp/wg004-target` aus und benutzte
zu keinem Zeitpunkt das Root vorbehaltene `server/target`.

Produktionspfade vollständig gelesen: `persistence.rs`, `world.rs` einschließlich
Open/Reconcile/Import/Spawn/Joinkit, `main.rs`, CLI-Parser und Offlineoperation,
Serverstart und Shutdown, Drivercapture und alle drei Flush-/Releasepfade sowie
der komplette Storageworker. Die veränderten bestehenden Testfixtures wurden
gegen ihren vorherigen Stand gelesen; ihre Daten-/Fehlerassertions wurden
erhalten, die Backends auf deklarierte Format4-Identitäten umgestellt.

Producerreview: Config-Seed-/Presetvertrag, Worldidentity und Generatorvertrag,
Sim-GenerationBinding/Weak-Storetoken/verified Capture und Storage-Metadatenpfade.
Auch Offline-Prepare, LockedFile/Headerprüfung, vollständiger Datensatzscan,
Receiptindex, Backupkopie/-validierung und atomarer Migrationcommit wurden gelesen.
Die bereits unabhängig geprüften Producer sind separat integriert: Config
`402df480`, Storage `b299a5fc`, Sim `da61c638`.

## Quellen- und Verhaltensmatrix

Referenz: Minecraft Java **1.21.8**, Protokoll772, Paper Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Basequellen und angewandte Paperquellen
aus `.codex-tmp/paper-1.21.8-reference` wurden benutzt, keine APIinterface allein
als Beleg eines Serverpfads.

| Pfad | Gelesene echte Implementierung | Beobachtung und Appvertrag |
| --- | --- | --- |
| Startup | Vollständige Original-/Paper-Methoden `MinecraftServer.loadLevel`, Original `createLevels`, Paper `loadWorld0`/`initWorld`, `setInitialSpawn`/`setupDebugLevel`/`prepareLevels` und relevante vollständige Sourcepatch-Hunks. | Existierende Worldgenoptionen werden vor Levelkonstruktion geladen. Frische Optionen gelten nur ohne gespeicherte Welt. FerrumC bestätigt seinen eigenen Descriptor vor Import/Spawn und Connections; fester Spawn und Flat-v1 bleiben ausdrückliche Abweichungen. Paper besitzt Fallback-/Restoreverhalten für level.dat; FerrumC führt keine automatische Reparatur aus. |
| Clientseed | Vollständiger originaler und angewandter `BiomeManager`, `ServerPlayer.createCommonSpawnInfo`, `PlayerList.placeNewPlayer` einschließlich passendem Sourcepatchkontext. Guava33.3.1 `Hashing.sha256`, `AbstractHashFunction.hashLong`, `AbstractByteHasher.putLong`, `MessageDigestHasher.hash`, `HashCode.BytesHashCode.asLong`/`padToLong` samt Bytebuffersetup. | Original verschleiert den tatsächlichen Weltseed: SHA256 über acht Little-Endian-Seedbytes; erste acht Digestbytes als Little-Endian-i64. App verwendet den gespeicherten Seed, einschließlich signed64-Grenzen. Echte Original-JAR-Goldens wurden unabhängig reproduziert. |
| Save/Close | Vollständige Original-/Paper-Methoden `MinecraftServer.saveAllChunks`/`saveEverything`/`stopServer`, Paper `ServerLevel.saveIncrementally`/`save`/`saveLevelData`, `ServerChunkCache.save`/`close`. Vollständiger Incremental-saving-Patch; relevante Moonrise-Save-/Close-/RegionIO-Hunks und angewandte `ChunkHolderManager.close`/`saveAllChunks`/`autoSave`, `NewChunkHolder.save`/`saveChunk`/`saveEntities`/`savePOI`, `MoonriseRegionFileIO.flush`/`partialFlush`/`flushRegionStorages`. | Paper wartet Save-/IOarbeit vor dem vollständigen Stop. Rust-Arc-, Codec-, Token- und redb-Transaktionsregeln folgen FerrumCs eigenem Producervertrag. Seine Shutdown-/Unloadpfade müssen Commitbelege abwarten und Storagefehler erhalten. Eine gemeinsame Player-/World-/Journalcrashtransaction wird nicht behauptet. |
| Events | Die vollständigen Startup-/Shutdownmethoden und die Savepatches oben. | Paper sendet WorldInitEvent vor Spawnchunkgeneration, WorldLoad nach Vorbereitung und ServerLoad vor acceptConnections; stopServer deaktiviert Plugins vor Playersave/Worldsave. Dieses Ticket führt diese World-/Saveevents nicht neu in FerrumCs Javabrücke ein. Die vorhandenen Javaadapterverträge sind separat erneut bestanden. |

## Appbefunde

- Ein konkretes Backend wird über `spawn_blocking` geöffnet. Seine WorldStore-
  und PlayerStore-Traitviews teilen dieselbe Arcallocation. `PersistenceTarget`
  kapselt privaten Storearc und opaque WG003-Token; nur die geprüfte Konstruktion
  ist produktiv verfügbar. Clone kopiert Arc/Token, erzeugt kein neues Backend.
- Omission verwendet gespeicherten Seed; frische Omission wählt ausdrücklich
  Seed0/Flat-v1. Expliziter Seedmismatch und unsupported Descriptorversion,
  Generatorkind, Revision oder malformed Descriptor verweigern Startup. Die
  unabhängigen Snapshotfixtures vergleichen alle Tabellen-/Katalog-/Recordbytes
  und bestätigen keinen Worldwrite nach dieser Ablehnung.
- Import läuft erst nach Descriptorbestätigung und vor Spawn. Directoryscan und
  Regiondecode laufen in Blockingtasks, Speicherwrites sind begrenzt. Fehlender,
  malformed oder leerer Import wird abgelehnt. Noch nicht importierte Chunks
  verwenden ausdrücklich FerrumC-flat-v1; Vanilla-Weltgenoptionen werden nicht
  rekonstruiert. Auf einer frischen Welt kann ein später abgelehnter Import bereits
  Metadaten hinterlassen; das widerspricht dem verlangten Write-before-Identity-
  Verbot nicht und ist keine erfolgreiche Importabnahme.
- End-of-tick reserviert mit `try_reserve` vor synchroner Capture. Voll bedeutet
  verlustfreies Defer; geschlossen bedeutet Fehler. Wrong-Arc- und späte
  Capturefehler erhalten Dirtyfacts, Journal, Mutation-IDs und den Queueslot.
  Die Capture ruft keine Metadaten-I/O auf. Finale Abgabe und Unload reservieren
  ebenfalls zuerst und verarbeiten sämtliche begrenzten Capturerequests.
- Worker validiert die erwartete Bindung gegen seinen tatsächlichen Zielstore
  vor dem leeren Ack, jedem Overlayprefix, jedem Journalappend und Retry. Jeder
  Overlayprefix prüft außerdem seinen Scope gegen den Targetscope. Erfolgreiche
  Prefixe werden erst nach bestätigtem Write entfernt; fehlgeschlagene Payloads
  und Lost-Reply-Journaltokens bleiben für begrenzte Recovery erhalten. Ein Ack
  meldet den ersten Fehler konservativ auch bei einer später erfolgreichen Retry.
- Unload gibt Tickets erst nach allen Batchacks frei; auch eine leere Barriere
  beweist die vorangegangene Workerarbeit. Ein fehlgeschlagener Tail behält alle
  Tickets. Shutdown hält den Driver während Connectionteardown am Leben,
  verarbeitet finalen Capturetail, schließt und drained den Worker und beobachtet
  sämtliche Tasks. Storagefehler haben Vorrang vor sauberer Shutdownmeldung.
- Format3 normaler Start nennt die konkrete Offline-CLI. Deren Dispatch erfolgt
  vor Configbootstrap und Netzwerk. Explizite Deklaration und Scopeparse führen
  zu Prepare→Required→neues exklusives oder ausdrücklich hashgebundenes Backup
  →Migration; kompatibles Format4 wird vollständig erneut validiert. Fehlender
  Source wird nicht erzeugt, bestehendes Backup nicht überschrieben. Fehler nach
  verifiziertem Backup erhalten den Report und physischen SHA256 für expliziten
  Retry. Das tatsächlich ausgeführte Offlinebinary schreibt keine Config oder
  default-world-Verzeichnis.

Diese Aussagen setzen den dokumentierten unveränderlichen Generationidentity-
Vertrag der echten Storeproducer voraus. Faultdoubles ändern Descriptors nur,
um die Writer-/Retryadmission zu prüfen; sie erweitern den Runtimevertrag nicht
zu live editierbaren Generatoren.

## Unabhängige Prüfnachweise

Alle Checks unten liefen **nach** dem finalen Runtimefreeze:

| Prüfung | Ergebnis | Log |
| --- | --- | --- |
| `cargo test --offline --locked -j2 -p ferrumc-app --lib` | **287 bestanden**, einschließlich ExactArc-/LateCapture-/Full-/Closedqueue-/Ack-/Finaldrain-/Descriptor-/Retryfixtures. | `/tmp/wg004-review-lib.log` |
| Relevante Appintegration, elf Testtargets | **38 bestanden**: CLI12, Generationstartup8, Persistenz-/Container-/Identity-/Login-/Movement-/Player-/Routingverträge18. | `/tmp/wg004-review-contracts.log` |
| Echte ausdrücklich aktivierte `java_compat -- --ignored` | **1 bestanden** mit Java21 und hashgeprüften kopierten Bridge-/gewöhnlichen Bukkit-Fixture-JARs. Commands, Events, Permissions, Restart und Startuprollback; Unsupportedzweige sind erwartete Vertragsfälle. | `/tmp/wg004-review-java.log` |
| Original-JAR-Biomegoldens | **8/8 byteidentisch**, frischer Helperbytecode, Original-JAR und Originalbibliotheken vor Helper im Classpath. | `/tmp/wg004-review-biome-oracle.log` |
| Owned-/Java-/Binaryhashes | **26/26**, **3/3**, unveränderliches Binary mit obigem SHA. | `.codex-tmp/wg004-freeze.sha256`, `.codex-tmp/wg004-java-artifacts.sha256` |

Relevante Integrationstesttargets: `cli`, `generation_startup`,
`persistence_overlay`, `overlay_multi_section_persistence`,
`block_entity_persistence`, `canonical_identity`, `login_boundaries`,
`movement_validation`, `player_component_persistence`, `player_load_failure`,
`restored_position_routing`. Die Appunit- und Integrationstests ergeben
**325 normale unabhängige Tests plus eine echte Javaakzeptanz**.

Der Reviewer verifizierte die Originalartefakte vor Oracleausführung:
Bundle SHA256 `2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7`,
innerer Server SHA256 `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`,
Mappings SHA256 `e373ba569839e6f3c7bd4880247f3e85577d92cde2c605adfa2024eb9cae503e`,
Guava SHA256 `4bf0e2c5af8e4525c96e8fde17a4f7307f97f8478f11c4c8e35a0e3298ae4e90`
und sämtliche **43** Originalbibliotheken gegen die gebündelte libraries.list.
`javac -proc:none` kompilierte ausschließlich den Reflectionhelper in einem
frischen Tempverzeichnis. Tatsächlich aufgerufen wurde Mojangs `dnz.a(long)`;
kein nachimplementierter Hash diente als Oracle. Der unveränderte Fixturehash ist
`0b3544b356a470be4bc1f40d6cc98fbec9ae73e554d9e049fac65b4e1ca638ff`.

Implementernachweise wurden zusätzlich gelesen, getrennt von den eigenen Läufen:
normaler Vertragsbestand375, echte Javaakzeptanz1, striktes Alltargetclippy
einschließlich Dependencies mit `-D warnings`, Owned-rustfmt und scoped Diffcheck.
Ihre Logs stehen in `.codex-tmp/wg004-{app-tests,final-contract-tests,java-tests,
clippy,fmt}.log`. Keine neue Lintsuppression oder abgeschwächte bestehende
Fehlerassertion wurde festgestellt. Roots getrennte Clientrestartabnahme ist
nicht Bestandteil der hier als eigenständig ausgewiesenen Testzahl.

## Grenzen des Urteils

Kein bestätigtes Finding, daher kein eigener RED→GREEN-Fixzyklus erforderlich.
Das CLEAN betrifft die dauerhafte **Flat-v1-Basis und Appverdrahtung**. Es
beweist keine natürliche Overworld-/Nether-/Endweltgeneration, Entitysimulation,
vollständiges Survival, vollständigen Java-/Paper-APIvertrag oder CPU-/RSSgewinn.
Der tatsächliche Paper60-Server wurde durch dieses Review nicht live verglichen;
der Original-JAR-Differentialcheck betrifft ausschließlich die Seedobfuscation.
