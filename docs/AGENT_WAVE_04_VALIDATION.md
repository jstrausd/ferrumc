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

## Unabhängige UUID-Storage- und Compilerintegration

STORAGE-ENTITY001.1 wurde als `a5d11819` integriert. Der
[Storage-Review](agent-tasks/STORAGE-ENTITY001_1_UUID_STORE_REVIEW.md) ist CLEAN:
Debug/Release je212 Originaltests, zusätzlich9 eigene Probes,39 tatsächliche
Writeabortcutpoints, Lostreply/Reopen, strict Alltargetclippy und fmt grün.
Format5 bleibt opt-in; die aktive Format4-App wird dadurch nicht umgestellt.
Die konkrete .2-Offlinemigration ist getrennt freigegeben und in Umsetzung.

GEN-05.1 wurde als `c4e3dc76` integriert. Der
[Compiler-Review](agent-tasks/GEN-05_1_DENSITY_COMPILER_REVIEW.md) ist CLEAN:
isolierter Snapshot auf `0276f3d4`,17/17 Files nach Probe-Restore bytegleich.
Debug/Release je177 Originaltests sowie3 eigene Probes grün; strict
Alltargetclippy und Workspace-fmt grün. Zusätzliche256 gemeinsame DAGs wurden
gegen ein unabhängiges Reachability-/Depthmodell geprüft. Root reproduzierte
die128 Original-JAR-Codecrecords normal und mit Python `-O`; Fixture und
Manifest bytegleich. Die Union misst3102 Vertices/3435 Edges/Depth87.
Dies ist Struktur-/Codecabnahme und aktiviert noch keine Terrainberechnung.

Diese beiden integrierten Runtimeänderungen sind im älteren CI-Lauf
`9c35e769` noch nicht enthalten. Gesamt-CI für `5ad5f655`,
[Lauf38079802426](https://github.com/jstrausd/ferrumc/actions/runs/38079802426),
scheiterte beim Workspace-Clippy: die bisher untypisierte leere Payload-
Assertion im UUID-Storageintegrationstest wurde durch das gemeinsame
serde_json-Feature mehrdeutig. Die Crate-Einzelprüfungen hatten diesen
kombinierten Fehler nicht erfasst. Root reproduzierte ihn auf detached5ad5,
Log `.codex-tmp/main-ci-repro-red.log`.

Der ausschließlich einzeilige Testfix **`9025c687`** typisiert die leere
RHS als `&[u8]`; keine Runtimeänderung oder Lintausnahme. Danach unabhängig
vollständiges Workspace-Alltargetclippy, Workspace-fmt und alle7 UUID-
Storageintegrationstests grün. Logs
`.codex-tmp/main-ci-repro-green-{clippy,fmt,uuid}.log`. Gesamt-CI für den
veröffentlichten Head **`9025c6876100d24d4508f3ca7fab223c0f2cedd1`** ist
erfolgreich: [Lauf38080572897](https://github.com/jstrausd/ferrumc/actions/runs/38080572897).
Rust-Workspace, Java-/Plugincontracts, Generator-/Originaldaten-/Claimchecks
und alle sieben Client-/Neustartgates grün. Die bewegten GEN-05.2-, SIM-03.3-
und STORAGE-ENTITY001.2-Implementierungen gehören noch nicht zu diesem Lauf.

## Item-State-Runtimeintegration

SIM-03.3/SRV-02.1 ist nach unabhängigem
[Rootreview](agent-tasks/SIM-03_3_ITEM_ENTITY_STATE_REVIEW.md) CLEAN als
`84967c54` integriert. Root Debug/Release je252 Tests grün:249 Autorentests
plus3 Probes mit65.536 Zählerfällen,2.048 unabhängigen Motionbitfällen und1.024
Slotinterleavings. Strict alltargetclippy/fmt grün; alle10 Main-/Snapshot-
Freezehashes nach vollständigem Entfernen der Probes gleich. Echte Items
besitzen sealed Zustand im bisherigen Ownerslot und native Capture-/Restore-
APIs; Drops/Pickup/Physics/Tracking/Apppersistenz werden dadurch noch nicht
ausgeführt. Die frühere Gesamt-CI9025 enthält diese Runtime noch nicht.

STORAGE-ENTITY001.2 ist nach unabhängigem
[Review](agent-tasks/STORAGE-ENTITY001_2_UUID_MIGRATION_REVIEW.md) CLEAN als
`34acc8e9` integriert. Eigene Reviewer-Debugprüfung und unabhängiges Rootrelease
jeweils243 Tests grün, zusätzlich fünf Runtimeprobes und drei negative
Authority-Compilerprobes. Strict Clippy/Fmt und aktueller13-Dateien-Freeze
bestätigt. Rootrelease aufd9feaa82 plus13 eingefrorenen Files, Log
`.codex-tmp/storage-entity001_2-root-release.log`. Der offline4→5-Producer
aktiviert keinen App-/Gameplayconsumer.

Gesamt-CI für `83ea2c6d2a1c251da956a94c06fe8a7891530260` ist erfolgreich:
[Lauf38081893560](https://github.com/jstrausd/ferrumc/actions/runs/38081893560).
Sie umfasst SIM-03.3, alle bisherigen Workspace-/Java-/Generator- und sieben
Client-/Neustartgates; STORAGE-ENTITY001.2 und GEN-05.2 gehören nicht zu diesem
Head. Ein neuer Gesamtlauf für `34acc8e9` ist angefordert.

## Laufende Folgearbeit

- `feature_catalog`: STORAGE-ENTITY001.2 unabhängig CLEAN abgeschlossen;
  jetzt read-only GEN-05.2-Codereview. Isolierter Snapshot folgt dem finalen
  Autorenfreeze. Root führt frische normal/-O Original-JAR-Reexports aus.
- `item_transfer_primitives`: GEN-05.2-Seed-/Scalar-/Climateumsetzung deckt
  sämtliche17.213 Oracle-Records lokal ab. Finale Debug-/Release-/strictchecks
  und Freeze laufen; nach CLEAN NoiseChunk/Interpolation, Aquifers,
  Material/Ore und Surface/Biome bis zu echter Chunkgeneration.
- `select_world_ticket`: konkreter DUR-03.1-API-/Schemaentwurf für atomare
  Player↔Entity-Ownership. Eigener versionierter Modus mit scoped Player,
  Revisionen/Tombstones und bounded idempotenten Receipts; Rootdesignreview
  vor Runtimecode. Danach DUR-04 Reservation/Pending/Ack und DUR-05 Appdriver.

Der Oracleagent `gen05_2_router_oracle` ist beendet und besitzt keine aktive
Runtimeaufgabe. Seine fünf Referenzdateien sind eingefroren:17.213 tatsächliche
Original-JAR-Records, normal/-O Fixture und Manifest bytegleich,27 originale
Class-CodeSources plus frischer Helper sowie acht Negativguards in beiden
Pythonmodi geprüft. Dies ist kein Terrain-/Paper-End-/Spielbarkeitclaim.

Drop/Pickup, Mining/Loot/Crafting, Physik/Tracking/Kampf/Mobs und sämtliche
Dimension-/Bossconsumer bleiben anhand des [Katalogs](MINECRAFT_FEATURE_CATALOG.md)
und der Survival-Gesamtabnahme offen. Dieses Dokument beendet den Arbeitsauftrag
nicht und verspricht keinen Hintergrunddienst nach beendeter Sitzung.
