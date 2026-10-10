# Vierte Agentenwelle: Generation-Apppfad und Entitygrundlagen

Stand: 10.10.2026. Minecraft Java1.21.8 / Protokoll772 / DataVersion4440.
Ausgangspunkt `fb0e96e0`; kombiniert geprüfte Runtimebasis **`62a5b74d`**.
Danach separat geprüfte Entitygrundlage als **`916251da`** und originale
kubische Splines als **`245134eb`** integriert.
Root orchestriert Review, Tests und Git; Produktionscode schreiben die Agents.
Branch bleibt ausschließlich `main`.

Die [Survival-Gesamtabnahme](SURVIVAL_END_TO_END_ACCEPTANCE.md) bleibt in
allen sechs Etappen offen. Die aktive App verwendet jetzt Format4 und eine
verifizierte dauerhafte Seed-/Generatorbindung, weiterhin mit **Flat-v1**.
Diese Welle aktiviert keine natürliche Terrainpipeline, keine Item-/Mob-Entities,
keinen Nether-/Endfortschritt und keine vollständige Java-/Paper-API.

## Getrennt integrierte und geprüfte Verträge

| Ticket / Commit | Nachweis | Grenze |
| --- | --- | --- |
| REG-ENTITY001, `cd32fbd2` | Opaquer EntityTypeId und alle 151 Entitytypen aus dem gepinnten Originalreport generiert; allocfreie Name-/Wirelookups und vollständige Iteration. Strikte Generatorvalidation. Agent37 Unit/3 Build/5 Public/8 Doc; Rootreview CLEAN, acht zusätzliche öffentliche Contracts und unabhängiger Originalreexport grün. Freeze13/13 identisch. | Producer für SIM-03.2; keine Entitykomponenten, Physik, Tracking oder Mobs. |
| GEN-04.2, `09fd6657` | Normal-, Simplex- und BlendedNoise mit originaler Auswertungsreihenfolge und RNG. 153 Worldtests; Releasebestand134 und strictchecks grün. 16.974 tatsächliche Original-JAR-Records. Root Python-`-O`-Reexport von Fixture/Manifest bytegleich und Publiccontracts7 grün. Fremdreview CLEAN, Freeze12/12 identisch. | Zusätzliche explizite FerrumC-Ressourcengrenzen. Noch keine Densityrouter, Aquifers, Surface/Biome oder natürliche Chunkgenerierung. |
| WG-004, `62a5b74d` | Produktiver Format4-Start, gespeicherte Seedautorität, überprüfte Generatorbindung vor Import/Spawn/Connections. Verifizierte Tickcapture und Storezuordnung, Retry-/Unload-/Stopbarrieren sowie explizite Offline-Migration. Fremdreview CLEAN; sieben Rootclientharnesses und zusätzliche echte Migration/Read-only-Restartprobe grün. Freeze26/26 identisch. | Nur Flat-v1 aktiviert. Keine automatische Migration oder Reparatur; keine gemeinsame Player-/Worldcrashtransaction und kein Prozesskillnachweis. |
| SIM-03.2, `916251da` | Begrenzte shardbesessene Non-playerregistrierungen, UUID-/Wire-/Chunkindizes, generationaler Incarnationhandle und residenter Tickentry. 164 Unit + 56 Integration + 3 Doc beim Implementer; unabhängige Debug-/Releaseprüfung jeweils227 einschließlich vier eigener Probes. Review CLEAN nach unveränderter F1-RED→GREEN-Probe; Freeze7/7 identisch. | Noch keine Appaktivierung, Living-/Itemkomponenten, Speicherung, Tracking, Physik, KI oder Entityevents. |
| GEN-04.3, `245134eb` | Immutable begrenzte originale f32-Splinegraphen, f32-Diagnostik und geordnete Callbackauswertung. Rootreview CLEAN nach dokumentarischem Clippy-F1; isoliert163 Worlddebugtests,20 Releasecontracts und frischer bytegleicher Originalreexport. Zusätzlich32 Graphen/4294 tatsächliche Original-JAR-Records, darunter3462 finiteSamples und160 Sourceexceptiontraces. | Zusätzliche eigene Caps/nonfinite Fehlerpolicy; noch kein Densitycompiler, Router oder aktiver Naturalgenerator. |

Originalreferenz: Minecraft1.21.8, Paper Build60/Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Vollständige Methoden,
Callees und relevante Base-/Featurepatches wurden gemäß den jeweiligen
[Taskdocs](IMPLEMENTATION_BOARD.md) gelesen. Quellenreview ist keine
Live-Paper-Gesamtabnahme. Kein CPU-/RAMvorteil wird aus der Programmiersprache
oder diesen isolierten Verträgen abgeleitet.

## Unabhängige Noiseprüfung

Die eingecheckte Compositefixture enthält 16.974 Records, 4.681.344 Bytes,
SHA256 `af477c940f8a807334686d1aa2ebfcc69618130f856baf6ee93fac12c31baa6d`.
Root erzeugte sie und ihr Manifest aus den hashgeprüften Originalartefakten
mit frisch kompiliertem Reflectionhelper erneut; beide Dateien bytegleich.
Logs: `/tmp/gen042-root-oracle.log`, `/tmp/gen042-root-contract.log`.

Reviewer `select_world_ticket` prüfte außerdem acht neue Seeds, 39 angrenzende
und dezimale Koordinaten, 34 i32-Blockpositionen, neue Blendedparameter und
nichtbinäre signed Amplituden: **25.950 Records / 16.768 Samples** gegen die
tatsächlichen Original-JAR-Methoden, sieben zusätzliche Releasecontracts grün.
Zusatzfixture-SHA256
`f1d753c5c5f2e81bdf66746df92ac61579f84cb75aab9c1bc587e3ca613ba9a9`.
Report `.codex-tmp/gen04_2-independent-review.md`; Logs
`.codex-tmp/gen04_2-review-{tests,release,clippy,fmt,extra-oracle,extra-tests}.log`.
Crate-/Reviewerzahlen sind wiederholte oder zusätzliche Prüfungen und werden
nicht zu einem einmaligen Workspace-Testbestand addiert.

## Unabhängige Appprüfung

Der [WG-004-Fremdreviewbericht](agent-tasks/WG-004_APP_GENERATION_REVIEW.md)
hält eigene **287 Unit + 38 Integration + 1 tatsächlichen Java-Vertrag**
fest. Java21 und Bridge-/Bukkitfixture-JARs wurden hashgeprüft. Die vorhandene
Java-API-Teilmenge blieb erfolgreich; Scheduler-/World-Unsupportedzweige
bleiben ausdrücklich erwartete Fehlerfälle.

Die acht Clientseedgoldens wurden separat mit frisch kompiliertem Helper
nach Prüfung der Original-JAR-, Mapping- und aller 43 Librarypins reproduziert.
Der tatsächliche Originalaufruf `BiomeManager.obfuscateSeed` liefert dieselben
Bytes; keine nachgeschriebene Hashformel diente als Oracle.
Logs `/tmp/wg004-review-{lib,contracts,java,biome-oracle}.log`.

Rootclientharnesses verwenden dasselbe eingefrorene Binary:

```text
.codex-tmp/wg004-freeze/ferrumc
SHA256 7333b12585c321f24b7fd4b3aa7138edb320f90265547795a513a1247139ff0b
```

| Harness | Ergebnis / lokale Artefakte |
| --- | --- |
| smoke.mjs | Login, Streaming, Bewegung und 28 Blockmuster nach Rejoin/Neustart grün; `/tmp/wg004-root-smoke.log`, `/tmp/ferrumc-wg004-smoke-run`. |
| chest-component-persistence.mjs | Komponenten und exakte NBT-NaN-/Zero-/Duplicate-Key-Bytes nach zwei Neustarts grün; `/tmp/wg004-root-chest-components.log`, `/tmp/ferrumc-chest-components-eaWTXL`. |
| inventory-cursor.mjs | Full-/Cursor-/Stale-/UUID-/Rejoin- und zwei Neustartverträge grün; `/tmp/wg004-root-inventory-cursor.log`, `/tmp/ferrumc-inventory-client-dslG2r`. |
| door-hinge.mjs | 14 Itemplatzierungen und 28 Half-States bei Actor/Observer vor und nach SIGINT-Neustart grün; `/tmp/wg004-root-doors.log`, `/tmp/ferrumc-door-hinge-dwHi2I`. |
| player-inventory.mjs | Vorhandene Survivalclicks, Komponenten, Window-/Stategates, Equipment, Spectator und Close0 grün; `/tmp/wg004-root-player-inventory.log`, `/tmp/ferrumc-player-inventory-jWKLkQ`. |
| operator-commands.mjs | Zwei Clients, Grant/Revoke, Gamemode und CLI-Recovery nach zwei Neustarts grün; `/tmp/wg004-root-operator-commands.log`, `/tmp/ferrumc-operator-client-TR2GVB`. |
| java-plugin.mjs | Bestehender begrenzter Command-/Permission-/Event-/Shutdownvertrag grün; `/tmp/wg004-root-java-plugin-client.log`, `/tmp/ferrumc-java-client-9ZNA9m`. |

Alle sieben Harnesses endeten mit Exitcode0. Das Binary blieb nach den Läufen
hashgleich. Geprüft wurden bestehende aktive Consumer mit der neuen Format4-App,
keine komplette Survivalrunde oder ein neuer Entity-/Terrainconsumer.

## Tatsächliche Offline-Migration und Neustart

Root arbeitete ausschließlich auf einer eigenen Kopie der aus Welle3 bekannten
Flat-v1-/Seed0-Welt. Der damalige Quelldatensatz blieb unverändert. Das echte
Binary verweigerte zunächst den gewöhnlichen Format3-Start mit konkretem
Migrationshinweis. Danach: Prepare-only, neues verifiziertes Backup, atomarer
Migrationcommit und erneute vollständige Compatiblevalidation beim CLI-Retry.
Log `/tmp/wg004-root-migration-process.log`, Arbeitskopie
`.codex-tmp/wg004-root-migration-process/world/world.redb`.

Der kopierte Ausgangsbestand enthält zwei Overlays, einen Player, 87
Journalrecords und 72 Receipts. Neues Backup `legacy-backup.redb`, physischer
SHA256 `a21aab3d830df4df3da8c4a4096eebb1ba0ba453fd637754bd9a9ba020cf38a3`.
Vorher-/Backuplogicalsnapshot
`b6333fd98618b0d132381a848ba504038ebe3043b6a306df36d33d6ce6510ef6`.

Ein erster Einsatz des Fresh-world-Smokeharness auf dieser bestehenden Welt
schlug fehl: der erhaltene Spieler stand bei x40; das Harness erwartet seinen
frischen Spawn und setzt Blöcke bei x10 außerhalb der erlaubten Reichweite.
Das Log `/tmp/wg004-root-migrated-smoke.log` ist **kein bestandener Lauf**.
Es bestätigt allein keine Produktionsregression. Das Harness versuchte dabei
Creativeinventaraktionen; deshalb ist dieser Clientlauf kein Nachweis, dass
der gesamte Playerpayload gegenüber dem Ausgangsbestand unverändert blieb.

Root prüfte anschließend mit einer unabhängigen Read-only-Clientprobe die aus
den Welle3-Neustartartefakten abgeleiteten **28 Blocknamen/-properties und die
gespeicherte Spielerposition40/64/8** über zwei echte Starts/saubere SIGINT-Stops
der migrierten Welt. Exitcode0; `/tmp/wg004-root-migration-readonly.log`.
Die Probe macht keine Platzierungen oder Bewegungen. Vollständige Rawrecord-
Erhaltung wird durch die getrennten Storage-/App-Snapshotverträge abgedeckt.

## Kombinierte Prüfung und Veröffentlichung

Detached Checkout `.codex-tmp/wave04-checkout` auf `62a5b74d`, kein zusätzlicher
Branch. Er enthält ausschließlich die integrierte Runtime, getrennt von den
aktiven SIM-03.2-/GEN-04.3-Agentänderungen.

| Prüfung | Ergebnis / Log |
| --- | --- |
| `cargo test --workspace --all-targets --offline --locked -j2` | **2.304 bestanden, 0 fehlgeschlagen, 1 ignoriert**, 130 Ergebnisgruppen; `/tmp/wave04-workspace-tests.log`, Exitcode0. |
| `cargo fmt --all --check` | Bestanden, Exitcode0; `/tmp/wave04-workspace-fmt.log`. |
| `cargo clippy --workspace --all-targets --offline --locked -j2 -- -D warnings` | Bestanden, Exitcode0; `/tmp/wave04-workspace-clippy.log`. |
| `cargo test --workspace --doc --offline --locked -j2` | **38 bestanden**, 0 fehlgeschlagen, 35 Ergebnisgruppen; `/tmp/wave04-workspace-doctests.log`, Exitcode0. |
| `cargo xtask generate --check` | Generierter Protokollcode aktuell, Exitcode0; `/tmp/wave04-generated.log`. |

`cargo nextest` ist lokal nicht installiert; der vollständige Cargo-Testlauf
ist der lokale Ersatz. CI verwendet nextest und prüft Java-/Tool-/Clientgates
erneut. Java und Clientnachweise oben betreffen den separat eingefrorenen
neuen Apppfad. CI für den veröffentlichten Commit wird nach Abschluss ergänzt.
Die erfolgreiche CI aus Welle3 für `fb0e96e0` ist kein Nachweis für diese Basis.

GitHub-CI für **`ba50369e`**, dieselbe Runtime wie `62a5b74d`, ist erfolgreich:
[CI-Lauf38073935031](https://github.com/jstrausd/ferrumc/actions/runs/38073935031).
Alle Rust-/Java-/Tool-/Generator-/Claims- und sieben Clientgates bestanden.
Dieser Lauf enthält den späteren SIM-03.2-Commit noch nicht.
GitHub-CI für **`b4c88476`**, einschließlich SIM-03.2, ist ebenfalls erfolgreich:
[CI-Lauf38075509053](https://github.com/jstrausd/ferrumc/actions/runs/38075509053).
Alle Rust-/Java-/Tool-/Generator-/Claims- und sieben Clientgates bestanden.
GEN-04.3 ist in diesem zweiten Lauf noch nicht enthalten. GitHub-CI für
**`3a5b4d62866d5f81d9820ec05dd3843389d32d90`**, einschließlich GEN-04.3,
ist ebenfalls erfolgreich:
[CI-Lauf38076109486](https://github.com/jstrausd/ferrumc/actions/runs/38076109486).
Alle Rust-/Java-/Tool-/Generator-/Claims- und sieben Clientgates bestanden.
Der nachfolgende Registrycommit555ee2aa ist darin noch nicht enthalten.

## Unabhängige Originaldatenprüfung

REG-WORLDGEN001 wurde als **555ee2aa** auf `main` integriert und veröffentlicht.
Der [Registryreviewbericht](agent-tasks/REG-WORLDGEN001_BUILTIN_DENSITY_DATA_REVIEW.md)
hält den isolierten zwölf-Dateien-Freeze, vollständigen Source-/Codecvergleich
und Grenzen fest. Root Debug/Release jeweils69, vier zusätzliche eigene
Proben, strict Alltargetclippy/fmt und Originalhelper normal/`-O` grün.
Alle102 Ressourcen und195 Noiseamplitudenbits sind vollständig geprüft;
Fixture-Slices sind bytegleich zur Registry, datagen/JAR-Ressourcen separat
raw-bytegleich. Keine behauptete natürliche Chunk- oder Paper-Spielabnahme.
Indexclaimcheck:94 Selftests und161 gescannte Dateien clean.

## Unabhängige Entityprüfung nach F1-Fix

Der [Entityreviewbericht](agent-tasks/SIM-03_2_ENTITY_OWNERSHIP_REVIEW.md)
hält Source-/Patch-/Hashmatrix und alle unabhängigen Logs fest. Reviewer
`select_world_ticket` verwendete eine isolierte Basis `62a5b74d` plus exakt
sieben SIM-03.2-Freeze-Dateien. Aktive World-/Registryänderungen blieben draußen.
Debug und Release: jeweils **227 bestanden**, keine fehlgeschlagenen oder
ignorierten Tests. Davon164 Unit,56 Producerintegration,4 eigene Integration
und3 Doctests. Strict Alltargetclippy/fmt ebenfalls grün.

F1 wurde unabhängig RED reproduziert: Ein kompletter Austausch der öffentlichen
Chunkmap auf fremde Welt/Dimension legitimierte die Entityaufnahme und drained
die Playerinbox. Der Implementer ergänzt die Owner-/Mapscopeprüfung vor der
Closure und vor Inboxdrain; keine automatische Reparatur. Dieselbe unveränderte
Probe ist GREEN. Zusätzliche Livehandle-/Neverclosure-/Ticket-/Inboxverträge
prüfen Welt-, Dimensions- und kombinierte Abweichung sowie erfolgreiche Arbeit
nach Wiederherstellung der ursprünglichen Map.

Ein eigenes BTreeMap-Modell vergleicht **12.000 deterministische Interleavings**
über alle64 Buckets und positive/negative Owner mit dem echten Producer.
Spawn/Move/Remove/Duplicate/Scope/Stale/Clone/Recreate erhalten die erwarteten
Indizes und Zustände. Eine weitere unabhängige Probe prüft alle151 Typen:
150 Non-playertypen angenommen, Player verweigert; signed Wireextrema und ±0.
Das belegt Typ-/Owneridentität und keine Verhaltensimplementation dieser Mobs.

## Gesamt-CI nach Originalregistryintegration

Der veröffentlichte Gesamtstand **`9c35e769`**, einschließlich der geprüften
REG-WORLDGEN001-Daten, hat die komplette
[GitHub-CI](https://github.com/jstrausd/ferrumc/actions/runs/38077316149)
erfolgreich abgeschlossen. Rust-Workspace, Java- und Plugincontracts,
Originaldaten-/Generator-/Claimchecks und alle sieben unabhängigen Clientgates
sind grün. Die noch unintegrierten Storage- und Worldcompileränderungen gehören
nicht zu diesem Lauf.

## Laufende Folgearbeit

- `feature_catalog`: REG-WORLDGEN001 abgeschlossen555ee2aa; STORAGE-ENTITY001.1 unabhängig CLEAN geprüft und
  integriert `a5d11819`. Jetzt konkreter Sim-Item-Entity-Vertrag; keine
  vorzeitige Gameplayaktivierung.
- `item_transfer_primitives`: GEN-04.3 originale kubische Splines integriert;
  freigegebene GEN-05.1-Compilerimplementierung. Der separate echte JAR-Codec-
  und Strukturoracle-Auftrag ist beendet. Holderhelper-/Splinepflichtfelder und kombinierte Tiefe
  werden gegen die Originalcodecs geprüft. Anschließend
  Densityregistry/-router, NoiseChunk, Aquifers, Surface/Biome und Chunkgeneration.
- `select_world_ticket`: STORAGE-ENTITY001.1 integriert `a5d11819`;
  unabhängige Debug-/Releaseabnahme je212 Tests plus9 eigene Probes,39 tatsächliche
  Writeabortcutpoints und Lostreply/Reopen sowie strictchecks grün.
  [Implementationreview](agent-tasks/STORAGE-ENTITY001_1_UUID_STORE_REVIEW.md) CLEAN.
  Konkrete .2-Migration nach vollständigem Rootdesignreview in Umsetzung.
  Root reserviert die vier Präzisierungen des unabhängigen Designreviews;
  Online prüft angetroffene Beziehungen, Pages sind nach tatsächlichen Bytes
  begrenzt, Änderungen haben definierte Epoch-/No-opregeln. Explizite4→5-
  Migration folgt als separates .2-Ticket; keine bestehende Welt wird automatisch
  umgestellt. Atomare Inventory↔Entity-CAS-/Receipttransfers bleiben danach offen.

Drop/Pickup, Mining/Loot/Crafting, Physik/Tracking/Kampf/Mobs und sämtliche
Dimension-/Bossconsumer bleiben anhand des [Katalogs](MINECRAFT_FEATURE_CATALOG.md)
und der Survival-Gesamtabnahme offen. Dieses Dokument beendet den Arbeitsauftrag
nicht und verspricht keinen Hintergrunddienst nach beendeter Sitzung.
