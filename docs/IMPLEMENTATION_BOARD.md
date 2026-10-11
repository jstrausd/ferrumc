# Minecraft-Implementierung und Agent-Zuweisungen

Ziel: Minecraft Java Edition **1.21.8 / Protokoll 772** vollständig spielbar
in FerrumCs Rust-Simulation, einschließlich Survival, natürlicher Welten und
der angeforderten Java-/Paper-Kompatibilität. Arbeitsbranch: ausschließlich
`main`. Ausgangspunkt dieser Runde: `c3e14d0f`.

Ausgangspunkt der vierten Welle: `fb0e96e0`; dessen GitHub-CI ist erfolgreich.
Der Betreiber hat die fortgesetzte Umsetzung bis zu durchspielbarem Survival
einschließlich Entities, End und Enderdrache ausdrücklich beauftragt. Die
[Survival-Gesamtabnahme](SURVIVAL_END_TO_END_ACCEPTANCE.md) definiert die
Fortschrittsstrecke; derzeit ist keine ihrer Etappen vollständig abgenommen.

Dieser Umfang ist der Auftrag, keine Fertigmeldung. Der bestehende
[Survival-Lieferplan](SURVIVAL_DELIVERY_PLAN.md) beschreibt die tatsächlichen
Lücken und Aufwandsschätzung; der [technische Plan](V2_IMPLEMENTATION_PLAN.md)
enthält bestehende Verträge. Der [ausführliche Featurekatalog](MINECRAFT_FEATURE_CATALOG.md)
enthält 214 Haupttickets mit Status, Crate-Owner, Abhängigkeiten und Abnahmen.
Inhaltsabnahme verwendet versionsfeste Matrizen,
damit zusätzliche Block-, Item- und Entityvarianten nicht vergessen werden.

## Vollständiger Arbeitsumfang

| Bereich | Umzusetzender Umfang |
| --- | --- |
| Anmeldung und Protokoll | Alle Zustände und Pakete der Zielversion, Online-Authentifizierung, Verschlüsselung, Profile, Skins, Chat-Signaturen, Teleports, Resource-Packs und sichere Proxy-Anbindung. |
| Weltmodell und Speicherung | Welt-/Dimensionsidentität, Chunks, Biome, Heightmaps, Block-/Entity-/Spielerdaten, Generatorbasis, Journal-Recovery, atomare Transfers, Migrationen, Backup/Restore und Anvil-Import/Export. |
| Weltgenerierung | Dauerhafter Seed und Generatorversion; natürliche Overworld mit Terrain, Biomen, Höhlen, Erzen, Vegetation und allen Features/Strukturen; Nether und End; versionsgenaue Seed-Parität, Datapack-Generatoren und Flat-/Custom-Profile. |
| Simulation und Bewegung | Autoritative Ticks, Chunk-/Entity-Aktivität, deterministische Nachbar-/Block-/Random-Ticks, vollständige Shapes, Kollision, Bewegung, Flugrechte, Schwimmen, Klettern, Schleichen, Fall- und Portalverhalten. |
| Licht, Zeit und Umwelt | Block-/Skylight, Tagesablauf, Wetter, Gewitter, Schwierigkeit, Gamerules, World Border und Weltzustand über Neustarts. |
| Survival-Grundkreislauf | Mining, Werkzeuge, Abbauzeiten, Loot, Item-Entities, Drop/Pickup/Merge/Despawn, Haltbarkeit, Inventarerhaltung, Crafting, Rezeptbuch und alle Container-Klickarten. |
| Spieler und Kampf | Gesundheit, Hunger/Sättigung/Exhaustion, Nahrung, Regeneration, Schaden, PvP, Rüstung, Schilde, Knockback, Effekte, Erfahrung, Tod, Respawn, Betten und Respawn-Anker. |
| Items und Fortschritt | Alle Itemverwendungen/Komponenten, Verzauberungen, Reparatur, Amboss, Schmieden, Schleifstein, Brauen, Bücher, Karten, Kompass, Advancements, Statistiken und besondere Ausrüstung. |
| Blockphysik und Landwirtschaft | Alle Placement-/Support-/Verbindungsregeln, Türen und Trapdoors, Flüssigkeiten/Eimer/Waterlogging, Feuer, Schwerkraft, Wachstum, Pflanzen, Bäume, Farmen und alle Block-Entities. |
| Container und Verarbeitung | Truhen, Öfen, Hopper, Dispenser/Dropper, Brewing Stand, Crafter und alle übrigen Arbeitsblöcke; Mehrspieleransichten, Tickzeiten und atomare Itemtransfers. |
| Redstone und Explosionen | Wire-Formen/Power, Quellen, Repeater, Comparator, Observer, Kolben/Sticky-Blöcke, Rails, Sensoren, TNT, Explosionsphysik, Schaden, Drops und Kettenreaktionen. |
| Entities und Mobs | Alle registrierten Arten, Spawn-/Despawnregeln, KI, Navigation, Kampf, Loot, Zucht, Zähmung, Villager/Handel/Raids, Bosse, Vehicles, Passagiere, Projektile und dekorative Entities. |
| Dimensionen und Transport | Nether-/End-Portale, Endfortschritt, Dragon/Wither, Dimensionsreisen, Boote, Minecarts, Reiten, Elytra und sämtliche versionsabhängigen Transportmechaniken. |
| Daten und Verwaltung | Registries, Tags, Rezepte, Loot Tables, Datapacks, Functions, Predicates, Advancements, alle Commands/Selectors/Completion, Scoreboards, Teams, Bossbars und administrative Dienste. |
| Plugins | Native APIs sowie Bukkit-/Paper-Lifecycle, Loader/Dependencies, Permissions, Services, Rust-tickbasierter Scheduler, World/Entity/Inventory/ItemMeta-APIs, Events/Cancellation/Reentrancy und versionsgebundene CraftBukkit-/NMS-/Packet-Verträge. |
| Betrieb und Performance | Release-/Upgradeverfahren, Pterodactyl/Wings-Abnahme, Save/Stop/RCON, Diagnostik, Sicherheit, Langläufe und gemessene RAM-/CPU-/Tickzeitvergleiche mit Paper bei gleichem Gameplay-/Pluginumfang einschließlich JVM. |

## Umsetzungstickets

| Ticket | Besitzer | Bereich | Status | Konkrete Abnahme |
| --- | --- | --- | --- | --- |
| CATALOG-001 | `feature_catalog` | Featurekatalog und versionsfester Inhaltsbestand | Erstbestand geprüft | 214 eindeutige Tickets ohne fehlende Abhängigkeits-IDs oder Zyklen; fünf Inventory-Selftests und `--check` bestanden. Fehlende Inhaltskategorien bleiben eigene CAT-Tickets. |
| WG-001 / GEN-01.1 | `select_world_ticket`, Review `feature_catalog` | `ferrumc-world` | Lokal geprüft und integriert, `e69bfc71` | 137 Tests, striktes Clippy und Formatprüfung bestanden; unabhängiges Review CLEAN. Numerischer Seed, geschlossene Generatoridentität, strikter 27-Byte-Codec und reproduzierbarer Flat-v1-Pfad. Keine natürliche Welt und keine persistierte Weltidentität. [Ticket](agent-tasks/WG-001_GENERATION_CONTRACT.md). |
| PLACE-DOOR-HINGE / BLK-01.1 | `select_survival_ticket`, Review `feature_catalog` | `ferrumc-placement` | Lokal geprüft und integriert, `676a357e` | Sechs Regressionen vor Fix; danach 56 Tests, striktes Clippy und Formatprüfung bestanden. Unabhängiges Review CLEAN. Cursorentscheidung, alle 21 Türmaterialien und beide Hälften; Shape-/Support-/Powergrenzen bleiben offen. [Ticket](agent-tasks/PLACE-DOOR-HINGE.md). |
| N03 / NET-06.1 | `teleport_confirmation`, Review `feature_catalog` | `ferrumc-app` | Lokal geprüft und integriert, `dff6c824` | Verbindungs-IDs/ACK-Gate, endlicher FIFO-Präfix und Persistenz; Review fand einen fehlenden Antwortflush, echte RED-Regression und Fix, anschließendes Review CLEAN. 357 App-Tests, davon sechs Teleport-Sockettests; Java-Test im App-Gesamtlauf ignoriert, anschließend separat erfolgreich ausgeführt (Wellenbericht). [Ticket](agent-tasks/N03_TELEPORT_CONFIRMATION.md). |
| PLACE-DOOR-HINGE-CLIENT / BLK-01.2 | `door_client_acceptance`, Review `select_world_ticket` | `tools` | Live geprüft und integriert, `090165bd` | 14 echte Itemplatzierungen, 28 Half-States bei Actor/Observer vor und nach SIGINT-Prozessneustart; zwei saubere Prozessläufe mit identischem Binary-SHA. Review CLEAN, CI-Gate ergänzt. [Bericht](agent-tasks/PLACE-DOOR-HINGE_CLIENT.md). |
| SIM-DOOR-GOLDEN | `select_world_ticket`, Review Root | `ferrumc-sim` (nur Test) | Lokal geprüft und integriert, `d9cac2c7` | Bestehender Consumertest erwartete die frühere Mittelpunkt-Scharnierseite; vier State-Goldens und Kommentar originalgetreu korrigiert. Keine Produktionslogik geändert. [Ticket](agent-tasks/SIM-DOOR-GOLDEN.md). |
| WG-002 / GEN-11.1 | `select_world_ticket`, Review `item_transfer_primitives` | `ferrumc-storage` | Lokal geprüft und integriert, `dd910fd4` | 146 Crate-Tests, striktes Clippy und Format; unabhängiges Review CLEAN und sieben öffentliche Contracttests separat grün. Eigene Generatoridentitätstabelle in explizitem Format 4, atomarer Freshness-/Savevertrag; produktives Format 3 unverändert. Migration ist ein eigenes Ticket. [Ticket](agent-tasks/WG-002_GENERATION_METADATA.md). |
| ITEMS-TRANSFER-001 / SRV-01.1 | `item_transfer_primitives`, Review `feature_catalog` | `ferrumc-items` | Lokal geprüft und integriert, `a7f7aa33` | 72 Unit-Tests und drei Doctests; Review CLEAN, unabhängige Probe mit 1.377.792 Mengen-/Capacityfällen und 3.364 Repräsentationsvergleichen. Reine Split-/Transferpläne, keine Gameplayanbindung oder dauerhafte Transfertransaktion. [Ticket](agent-tasks/ITEMS-TRANSFER-001.md). |
| CAT-02.1 | `feature_catalog`, Review `select_world_ticket` | `scripts`, Originaldaten-Fixtures | Lokal geprüft und integriert, `37c037b9` | 17 Helper- und sieben Inventorytests, Reexport aus frischem Tempdir/kaltem Cache und unabhängiger Reexport/Vollmengenvergleich; Review CLEAN nach ZIP64-/UTF-8-Grenzfix. Vanilla-1.21.8-Referenzdaten und Offline-CI-Gate; weitere Kategorien und Gameplayconsumer offen. [Ticket](agent-tasks/CAT-02_1_ORIGINAL_CONTENT.md). |
| NET-06.2 | Noch nicht vergeben | Producer/Consumer getrennt vor Vergabe | Offen | Gemeinsame Epoch-/Move-Admission-Autorität für externe Teleports nach dem endlichen Queue-Snapshot. NET-06 bleibt insgesamt T; bestehendes App-ACK-Gate ist keine globale atomare Tickreihenfolge. |

WG-001 wird anschließend durch getrennte Storage-, Sim- und App-Tickets
aktiviert; erst danach wird natürliche Generierung eingeführt. Mining und
Survival bauen auf autoritativen Inventaren, Entity-Ownership, Blockfakten,
Shapes und atomarer Speicherung auf. Die Türkorrektur ist ein einzelner
Placement-Vertrag; sie schließt keine Survival-Etappe ab.

Der [Prüfbericht dieser Welle](AGENT_WAVE_01_VALIDATION.md) hält kombinierte
lokale Ergebnisse, unabhängige Clientartefakte und die verbleibenden Grenzen
fest. GitHub-CI prüft den veröffentlichten Commit auf `main`; ein älterer
Workflowlauf ist kein Nachweis für diesen neuen Stand.

## Abgenommene zweite Welle

Ausgangspunkt ist `d96228b8`; die CI der ersten Welle ist erfolgreich.
Die drei obigen Agents waren tatsächlich beauftragt. WG-002 besaß nur die
Storage-Crate und sein Taskdokument; ITEMS-TRANSFER-001 besitzt nur die
Items-Crate und sein Taskdokument. CAT-02.1 besaß Export-/Inventory-Tooling,
zugehörige Originaldaten und sein Taskdokument. Root besitzt dieses Board,
den Featurekatalog und den gemeinsamen Prüfbericht. Nur Root verändert Git.

Die Agents prüften jeweils fremde Implementierungen; alle drei Reviews sind
CLEAN. Der [Prüfbericht der zweiten Welle](AGENT_WAVE_02_VALIDATION.md)
hält gemeinsame Rust-/Java-/Toolprüfungen und vier unabhängige Client- und
Neustartharnesses fest. Formatmigration, produktive Descriptor-Anbindung, natürliche
Generierung und dauerhafte Inventory↔Entity-Transfers bleiben eigene Tickets.

## Nächste abhängigkeitstreue Vergaben

Nach Integration und CI dieser Welle werden freie Slots anhand dieser Reihenfolge
vergeben. Diese Tabelle ist Planung, keine Behauptung laufender Agents.

| Spur | Nächster begrenzter Schritt | Danach |
| --- | --- | --- |
| Welt | WG-002-MIG: explizite Format-3→4-Migration mit deklarierter Legacybasis; WG-002-Metadatenspeicher ist integriert | WG-003: Sim verlangt Descriptor vor Regeneration/Overlay; WG-004: App aktiviert Seed/Preset erst nach Migration. Danach GEN-02 Terrain und GEN-04 Original-RNG/Noise. |
| Originalinhalte und Shapes | CAT-02.1-Referenzdaten sind integriert; nächste getrennte Vergabe SIM-04 für konkrete Shape-/Supportfamilien, weitere Tags/Registries über CAT-02-Folgetickets | SIM-06 Placement, SIM-02 persistente Block-/Fluidticks und SIM-08 Neighborconsumer; darauf Wasser, Wachstum und Redstone. |
| Survival | DUR-03-Unterticket: atomarer Inventory↔Entity-Transfer mit Transfer-ID und Recovery; allgemeine Entitygrundlage separat als SIM-03-Unterticket | DUR-04 Simconsumer, DUR-05 App-Wiring, SRV-02 Drop/Pickup; danach Mining/Loot. Recipe-Matcher SRV-05 kann mit vorhandenem Rezeptbestand separat folgen. |
| Teleportautorität | NET-06.2 zuerst in konkrete Producer-/Consumer-/Wiringtickets zerlegen; deterministische Race-Reproduktion für einen nach Snapshot angenommenen Fremdteleport | Gemeinsame Epochprüfung bei tatsächlicher Sim-Move-Admission; Command-, Plugin-, Korrektur- und Disconnectpfade prüfen. |

Die jeweils nächste konkrete Vergabe erhält einen eigenen Dateibesitz und
Akzeptanzvertrag. Fehlende aktive Verbraucher werden nicht durch ungeprüfte
Mehr-Crate-Sammeländerungen ersetzt.

## Dritte Welle: integrierte Grundlagen

| Ticket | Agent | Exklusiver Dateibesitz | Status und Abnahme |
| --- | --- | --- | --- |
| WG-002-MIG / GEN-11.2 | `select_world_ticket`, Review Root | `server/crates/ferrumc-storage/**`, `docs/agent-tasks/WG-002-MIG.md` | Geprüft und integriert `b299a5fc`: 177 Tests auch unabhängig grün, strict Clippy/fmt; Review CLEAN. Explizite Offline-Migration mit deklarierter Legacybasis, Prepare/Backup, Daten-/Scopevalidation und atomarem Formatwechsel. Kein automatisches Upgrade und kein Prozesskillnachweis. |
| WG-003 / GEN-01.2 | `item_transfer_primitives`, Review `feature_catalog` | `server/crates/ferrumc-sim/**`, `docs/agent-tasks/WG-003_SIM_GENERATION.md` | Geprüft und integriert `da61c638`: 204 Tests und strictchecks grün, Review CLEAN nach unabhängiger Await-RED/GREEN-Repro; Fremdharness2/2 grün. Opaque Storebindung und Legacy-Seed0-Abgrenzung. Appaktivierung folgt in WG-004. |
| GEN-04.1 | `feature_catalog`, Review Root | `server/crates/ferrumc-world/**`, `docs/agent-tasks/GEN-04_1_RANDOM_NOISE.md` | Geprüft und integriert `e754254f`: 144 Debugtests, sieben Release-Vertragstests und 16.556 bitgenaue Original-JAR-Records; unabhängiger Rootreexport unter Python `-O` bytegleich, Contract7 grün, Review CLEAN. Kein natürliches Terrain allein durch diese Grundlagen. |
| CONFIG-WG001 / GEN-12.0 | `item_transfer_primitives`, Review Root | `server/crates/ferrumc-config/**`, `docs/agent-tasks/CONFIG-WG001_GENERATION.md` | Geprüft und integriert `402df480`: optionale signed numerische Seed-/geschlossene Presetdeklaration ohne implizite Auswahl; 30 Tests auch unabhängig grün, Review CLEAN. App aktiviert den Producer erst in WG-004. |
| SIM-03.1 | `feature_catalog`, Review Root | `server/crates/ferrumc-core/**`, `docs/agent-tasks/SIM-03_1_CORE_ENTITY_UUID.md` | Geprüft und integriert `6998a76e`: dauerhafte Entity-UUID als eigener Typ, getrennt von signed i32-Wire-ID; Default-/No-default-Tests und strictchecks grün, unabhängige Contracttests6/4 grün, Review CLEAN. Noch keine Entitysimulation oder UUID-basierte Storagekeys. |

Root besitzt Board, Katalog, Abnahme- und gemeinsame Prüfberichte sowie
Gitintegration. Die vorhandene Generatoridentität wird vom Noiseagenten nicht
ohne Abstimmung erweitert. Fremdreviews erhalten frei gewordene Slots.
Der [dritte Wellenbericht](AGENT_WAVE_03_VALIDATION.md) hält 2.271 normale
Workspace-Tests, 36 Doctests, sieben unabhängige Clientharnesses und die
erfolgreiche CI für `fb0e96e0` fest. Diese Prüfung betrifft die damalige
Format-3-App; die Format-4-App folgt in der vierten Welle.

## Vierte Welle: Integration und aktive Folgetickets

| Ticket | Agent / Review | Exklusiver Dateibesitz | Status und Abnahme |
| --- | --- | --- | --- |
| REG-ENTITY001 / SIM-03.2a | `feature_catalog`, Review Root | `server/crates/ferrumc-registry/**`, eigenes Taskdoc | Geprüft und integriert `cd32fbd2`: generierter opaquer EntityTypeId für alle 151 Originaltypen, allocfreie Lookups und strikte Generatorvalidation. Rootcontract8 und unabhängiger Originalreexport grün, Review CLEAN. Keine aktiven Mobs. |
| GEN-04.2 | `item_transfer_primitives`, Review `select_world_ticket` und Root | `server/crates/ferrumc-world/**`, eigenes Taskdoc | Geprüft und integriert `09fd6657`: Normal-/Simplex-/Blendednoise, 153 Worldtests und 16.974 Original-JAR-Records. Fremdreview CLEAN mit zusätzlich 25.950 Records aus neuen Seeds/Parametern; Rootreexport bytegleich. Kein Terrainrouter oder natürliche Chunkgenerierung. |
| WG-004 / GEN-12.1 | `select_world_ticket`, Review `item_transfer_primitives` und Root | `server/app/**`, eigenes Taskdoc | Geprüft und integriert `62a5b74d`: produktive Format-4-/Seed-/Generatorbindung, verifizierte Tickcapture, Retry-/Unload-/Stopbarrieren und Offline-Migrations-CLI. Fremdreview CLEAN; sieben Rootclientharnesses und zusätzliche Migration/Read-only-Restartprobe grün. Flat-v1 bleibt der aktive Generator. |
| SIM-03.2 | `feature_catalog`, Review `select_world_ticket` und Root | `server/crates/ferrumc-sim/**`, eigenes Taskdoc | Geprüft und integriert `916251da`: begrenzter UUID-/Wire-/Handle-/Chunkindex und Tickadmission. Fremdreview CLEAN nach F1-RED→GREEN; unabhängige Debug-/Releaseprüfung jeweils227 und12.000 Modellinterleavings grün. Keine aktivierten Mobs oder Itemconsumer. |
| GEN-04.3 | `item_transfer_primitives`, Review Root | `server/crates/ferrumc-world/**`, eigenes Taskdoc | CLEAN geprüft und integriert245134eb:163 Worlddebugtests, Rootrelease20 und bytegleicher Originalreexport. Zusätzlich32 Graphen/4294 Originalrecords einschließlich3462Samples/160Sourceexceptions; strictchecks nach dokumentarischem F1 grün. CI des Gesamtstands3a5b4d62 erfolgreich; Terrainaktivierung bleibt offen. |
| STORAGE-ENTITY001.1 | `select_world_ticket`, Review `feature_catalog` und Root | `server/crates/ferrumc-storage/**`, eigenes Taskdoc | CLEAN geprüft und integriert `a5d11819`: unabhängige Debug-/Releaseprüfung je212 Tests plus9 eigene Probes,39 echte Writeabortcutpoints und Lostreply/Reopen; strictchecks grün. Fresh/strict Format5-UUIDproducer mit bounded Pages und Indizes; vorhandenes Format4 unverändert verweigert. Migration/Gameplaytransfers bleiben getrennt. |
| STORAGE-ENTITY001.2 | Autor `select_world_ticket`, Fremdreview `feature_catalog` und Root | `server/crates/ferrumc-storage/**`, eigener .2-Brief | Unabhängig CLEAN und integriert `34acc8e9`: Debug/Release je243 Tests, fünf zusätzliche Runtimeprobes und drei Authority-Compilerprobes grün. Voller Validator, geschlossenes Backup, immutable344-Byte-Checkpoint, bounded Prüfkopien und exakte Delta/Resume. [Review](agent-tasks/STORAGE-ENTITY001_2_UUID_MIGRATION_REVIEW.md); keine automatische Appmigration. |
| REG-WORLDGEN001 | `feature_catalog`, Review Root | `server/crates/ferrumc-registry/**`, eigenes Taskdoc | CLEAN geprüft und auf main integriert555ee2aa:60 Noiseparameter,35 Densityresources und7 Noisesettings. Root Debug/Release jeweils69,4 eigene negative/Bitproben, strictchecks und102 Originalressourcen normal/-O grün. Statische Daten; Worldcompiler/Terrainaktivierung bleiben offen. |
| GEN-05.1 | `item_transfer_primitives`, Review Root | `server/crates/ferrumc-world/**`, eigener GEN-05-Brief; Root besitzt Lockresolution | Unabhängig CLEAN und integriert `c4e3dc76`: alle35 Definitionen/7 Router,128 tatsächliche Original-JAR-Records, Debug/Release je177 und3 eigene Probes grün. Frischer Originalreexport normal/-O bytegleich, Freeze17/17 und strictchecks grün. [Review](agent-tasks/GEN-05_1_DENSITY_COMPILER_REVIEW.md); Seed-/Sample-/Chunkconsumer folgen. |
| GEN-05.2 | `item_transfer_primitives`, Vertragsreview Root, Codereview `feature_catalog` | `server/crates/ferrumc-world/**`, gemeinsamer GEN-05-Brief | Unabhängig CLEAN und integriert `290e5343`: Debug/Release je188, drei zusätzliche unabhängige Debug-/Releaseprobes, strictchecks und29/29 Freezehashes grün. Frischer normal/-O Originalreexport bytegleich;17.213 tatsächliche JAR-Records. [Review](agent-tasks/GEN-05_2_SEEDED_ROUTER_REVIEW.md). Chunk-/Terrainconsumer bleiben offen. |
| GEN-05.3 | `item_transfer_primitives`, Runtime-Review `feature_catalog` und Root | `server/crates/ferrumc-world/**`, gemeinsamer GEN-05-Brief | Unabhängig CLEAN integriert `e7565c38`:126 Chunks/7.077.888 tatsächliche Paperdichten, Wrapper/Provider/Constructor/Climate; Root Debug/Release je232 Tests und strictchecks,118/118 Autorenpins geprüft. Zwei Lifecyclefindings korrigiert und unabhängig gegengetestet. [Review](agent-tasks/GEN-05_3_NOISE_CHUNK_REVIEW.md); Aquifer-/Blockausgabe und aktive natürliche Welt folgen. |
| GEN-05.4 | `item_transfer_primitives`, unabhängiger Review Root | `server/crates/ferrumc-world/**`, eigener .4-Brief | Unabhängig CLEAN und als `2a4a6cd4` integriert, vollständige CI erfolgreich: frische Archivprüfung Debug/Release je251, strictchecks ohne Ausnahmen;160 Autorenpins unverändert.126 vollständige Paper-Materialarrays/7.077.888 States+Flags,499+42 finite Methodenfälle inkl Cache-/Request-/RNGchronologie und15 zusätzliche Nonfinitepolicyfälle. [Review](agent-tasks/GEN-05_4_AQUIFER_MATERIAL_REVIEW.md). Naturweltaktivierung und Surface folgen. |
| REG-BIOME-01 | `item_transfer_primitives`, unabhängiger Review Root | `server/crates/ferrumc-registry/**`, eigener Brief | Unabhängig CLEAN lokal als `c3d75914` integriert:120 Autorenpins/48 Overlaydateien und160 alte Worldpins unverändert; Root Debug/Release je70+14 Doctests und strictchecks.67 komplette Originaldefinitionen/7598 geordnete Parameter, alle67 JARressourcen und vier Negativeguards unter Python-O; vier alte Generatedtabellen bytegleich, neue Tabelle in vier Modi gleich. [Review](agent-tasks/REG-BIOME-01_ORIGINAL_BIOME_DATA_REVIEW.md). Worldselector/Clientpalette folgen. Veröffentlichung gemeinsam mit fehlendem Netconsumer. |
| NET-04.1 | `feature_catalog`, unabhängiger Review Root | Proto/Spec und eigener Brief | Unabhängig CLEAN lokal als `aaa8fdc3` integriert:28 Source-/51 Evidence-/95 Publishedbaselinepins geprüft, Root Debug/Release je54 und strictchecks/Generatorcheck ohne Ausnahmen.30 echte Original-/Prismarine-Paketfixtures,121 Artifact-/42 Sourcepins, finale normal/-O-Helferklasse/JAR/semantische Beobachtungen unabhängig geprüft. [Review](agent-tasks/NET-04_1_ITEM_ENTITY_WIRE_REVIEW.md). Veröffentlichung gemeinsam mit .1b, keine Itemgameplayaktivierung. |
| NET-04.1b | `feature_catalog`, unabhängiger Review Root | ausschließlich Net und eigener Brief | Unabhängig CLEAN als `e88dced0` integriert: frischer Rootarchivlauf Debug/Release je196, strictchecks ohne Ausnahmen;36 Source-/15 Evidence-/79 unveränderte .1-Dateien und acht Paperpins geprüft. Elf reale Writer-/Frame-/FIFO-/Refusaltests, zwei negative Overrides und Framefrontretention; Writerfunktion bytegleich. [Review](agent-tasks/NET-04_1b_ITEM_PACKET_DELIVERY_REVIEW.md). Keine Itemgameplayaktivierung; Gesamt-CI für gemeinsamen Head folgt. |
| NET-04.0 | `feature_catalog`, unabhängiger Review Root | `server/crates/ferrumc-proto-gen/**`, eigener Brief | Unabhängig CLEAN integriert `e563b971`:27 Unit/2 WholeGeneratorchecks, frische Archivcopy,6 eigene Rawwireproben Debug/Release,54 WholeProtoDebug; strictchecks und9/55 Freeze-Dateien unabhängig geprüft. Produktionsspec/Generatedfiles bytegleich; ProtoRustdoc mit dokumentierter einzelner Baselinelintausnahme. [Review](agent-tasks/NET-04_0_BOUNDED_REMAINING_BYTES_REVIEW.md). |
| NET-04.0b | `feature_catalog`, unabhängiger Review Root | `server/crates/ferrumc-proto-gen/**`, eigener Brief | Unabhängig CLEAN als `cc3503e5` integriert: ausschließlich begründete methodenspezifische Dispatchdecode-Zeilenlintausnahme;29 Generatortests und strictchecks. Root9/43/35 Source-/Evidence-/Readonlypins geprüft; frischer echter xtask mit unveränderter Spec und separater Drei-Packet-Probe, jeweils48 Prototests und strictclippy grün. [Review](agent-tasks/NET-04_0b_GENERATED_DISPATCH_REVIEW.md). |
| NET-04.0c | Root | ausschließlich tatsächlich erzeugte Proto-Generatedfiles und Integrationsdocs | CLEAN adoptiert durch echten Produktionsxtask; sechs Dateien bytegleich zum unabhängigen Rootoutput, genau neun neue Methodattribute in fünf States. `--check` grün, Spec unverändert. .0b/.0c sind gemeinsam auf main `f6eb7111` veröffentlicht; CI38092928296 wurde durch den neueren Lauf ersetzt; vollständige CI für `2a4a6cd4` einschließlich dieser Adoption ist erfolgreich. Tatsächliche Entitypakete folgen .1. |
| DUR-03.1 | `select_world_ticket`, unabhängiger Review Root | `server/crates/ferrumc-storage/**`, eigener [Transferbrief](agent-tasks/DUR-03_1_PLAYER_ENTITY_TRANSFER.md) | Unabhängig CLEAN und integriert `eaedabd2`: Debug/Release jeweils280 Tests einschließlich zwei zusätzlicher Rootprobes, strictchecks und21/21 Freezehashes grün. Tatsächliche joint redb-Transaction, Memoryauthority, CAS/Tombstone/Receipts, Cutpoints/Uncertainty und Grenzen. [Review](agent-tasks/DUR-03_1_PLAYER_ENTITY_TRANSFER_REVIEW.md); Onlineconsumer folgt. |
| DUR-04.1 | `select_world_ticket`, unabhängiger Review Root | `server/crates/ferrumc-sim/**`, eigener [Consumerbrief](agent-tasks/DUR-04_1_PLAYER_ENTITY_CONSUMER.md) | Unabhängig CLEAN als `4f7d5037` integriert und CI-grün: frischer Archivlauf, WholeSim Debug/Release je290 einschließlich drei eigener Rootproben auf Memory/redb; strictchecks ohne Ausnahmen,43 Runtime-/22 Referenz-/3 Appfixture-/44 Ownedpins und fünf Autorenlogs geprüft. Versiegelte echte Store-/Receipt-/Currentpostimageevidence, SnapshotOnly-Clone und volle Playerreservation. [Review](agent-tasks/DUR-04_1_PLAYER_ENTITY_CONSUMER_REVIEW.md). Timer-/Dirtyconsumer und Appdriver folgen. |
| DUR-04.2a / SRV-02.2 | `select_world_ticket`, unabhängiger Review Root | `server/crates/ferrumc-sim/**`, eigener Counterbrief | CLEAN geprüft und als `d3c84252` integriert: unabhängige Debug-/Releaseprüfung jeweils320 Tests und strictchecks ohne Ausnahmen. Der historische FCIS-Allokationsbefund bleibt erhalten; sieben Größenstellen und ein Repräsentationsvergleich verwenden jetzt die separat geprüften borrowed Items-APIs. Alle residenten Items jeTick,64-Slot-Saveauswahl getrennt. [Review](agent-tasks/DUR-04_2_OWNED_ITEM_COUNTERS_REVIEW.md); Physik, Merge, Javaevents und Appaktivierung bleiben offen. |
| GEN-05.5 | `item_transfer_primitives`, unabhängiger Review Root | `server/crates/ferrumc-world/**`, eigener Biomebrief | CLEAN geprüft und als `65f050fb` integriert: Debug/Release je262 Tests, strictchecks ohne Ausnahmen,184 Source-/94 Evidence-/175 Inputpins frisch geprüft.156 echte createBiomes-Arrays,27 End-Grenzproben,3287 Selectoraufrufe; acht Runnerguards jeweils normal/-O grün. [Review](agent-tasks/GEN-05_5_BIOME_SELECTION_REVIEW.md); Zoom, Surface, Carver und aktive Naturwelt bleiben offen. |
| NBT-WRITE-01 | `feature_catalog`, unabhängiger Review Root | ausschließlich NBT und eigener Brief | CLEAN geprüft und als `579bed62` integriert: frisches f753e312-Archiv, Debug/Release je105, strictchecks ohne Ausnahmen.19 Source-/27 Evidence-/4 Fixturepins vor/nach geprüft;1161 übrige Archivdateien und alter Writer-/MUTF8code unverändert. [Review](agent-tasks/NBT-WRITE-01_BORROWED_NETWORK_ROOT_REVIEW.md); separater Itemsconsumer folgt. |
| GEN-05.6 | `item_transfer_primitives`, unabhängiger Review Root | ausschließlich World und eigener Brief; Root besitzt Lockresolution | CLEAN als `34c8875e` integriert: frisches65064bc7-Archiv, Debug/Release je270 und strictchecks ohne Ausnahmen.198 Overlay-/137 Source-/103 Evidence-/185 Inputpins,999 Outside/179 alte World-/44 Fixturedateien geprüft.63.443 tatsächliche Originalrecords je Modus, neun Rootguards normal/-O grün. [Review](agent-tasks/GEN-05_6_BIOME_ZOOM_REVIEW.md); Temperatur/Resolver/Surface folgen. |
| NET-04.2a | `feature_catalog`, unabhängiger Review Root | ausschließlich Items und eigener Brief | CLEAN als `67a9aa94` integriert: frisches65064bc7-Archiv, Debug/Release je120 und strictchecks ohne Ausnahmen.30 Source-/24 Evidence-/20 Paperpins;1155 Outside/24 alte Items-/19 NBT-/4 Fixturedateien plus Codecwriter unverändert. Fünf ganze Originalrows bytegleich, bounded direkte Metadataausgabe ohne Sourceclone. [Review](agent-tasks/NET-04_2a_ITEM_METADATA_BUILDER_REVIEW.md); bestätigte Simfakten und Sessiontracking folgen. |
| SRV-05.1 | `feature_catalog`, Review Root | `server/crates/ferrumc-items/**`, eigener [Rezeptbrief](agent-tasks/SRV-05_1_NORMAL_CRAFTING_RECIPES.md) | Unabhängig CLEAN und integriert `572083d1`: alle968 normalen Rezepte und21.855 Original-JAR-Fälle; Debug/Release je84 plus3 unabhängige Rootprobes mit301.510 Matchmodellvergleichen grün, strictchecks und frischer bytegleicher Reexport. [Review](agent-tasks/SRV-05_1_NORMAL_CRAFTING_REVIEW.md). Specials und Onlinecrafting bleiben Folgeconsumer. |
| SIM-03.3 / SRV-02.1 | `feature_catalog`, Review Root | `server/crates/ferrumc-sim/**`, eigener Itembrief | Unabhängig CLEAN geprüft und integriert `84967c54`: sealed Slotpayload, owned Refusals/Removal, lossless FCIE/FCIS, checked Capture/Restore und originale Zähler. Root Debug/Release je252 Tests einschließlich3 eigener Probes grün; strictchecks und10/10Freezehashes gleich. [Review](agent-tasks/SIM-03_3_ITEM_ENTITY_STATE_REVIEW.md). App-/Drop-/Pickup-/Physikaktivierung folgt. |

Aktuell arbeiten neben Root drei Agents parallel. Der Weltagent
`item_transfer_primitives` hat GEN-05.6 unabhängig CLEAN geliefert; Root hat
den allocationfreien Biom-Zoomproducer als `34c8875e` integriert. Der Agent
konkretisiert jetzt readonly GEN-05.7: originale feste Biomtemperatur-Noises
und Paper-Temperatur-/Niederschlagsprädikate.65 echte Biomdefinitionen und
zwei Presetdefinitionen bleiben getrennt. Runtimefreigabe folgt auf den
konkreten Parser-/RNG-/Ressourcen-/Originaloraclevertrag.
Oberflächen benötigen außerdem originale Temperatur-/Surface-Regeln; Carver
benötigen eigene Tag-/Konfigurationsproducer. Die aktive App bleibt Flat-v1/Plains.

`select_world_ticket` implementiert nach der CLEAN-Simintegration die konkret
geprüfte [DUR-03.2-Migration](agent-tasks/DUR-03_2_OWNERSHIP_FORMAT6_MIGRATION.md).
Die intern gewählte Variante A benutzt echte private Format6-Stage-CASreceipts
und genau einen atomaren Sourceimport. Originalpayloads bleiben erhalten;
Chunkepochs steigen checked um die tatsächliche importierte Entityzahl.
Immutable Stage/Backup werden nach Checkpointseal nur über begrenzte eigene
Prüfkopien geöffnet. Exklusiver Besitz ist Storage plus eigener Aufgabenbrief;
Appmigration und FCPS-Gameplayadmission folgen separat.

`feature_catalog` hat NBT-WRITE-01 und NET-04.2a unabhängig CLEAN geliefert;
Root integrierte sie als `579bed62` und `67a9aa94`. Der vollständige konkrete
SIM-03.4-Entwurf `efbeb2be` ist durch Root und einen separaten readonly Reviewer
CLEAN auf Vertragsumfang geprüft. Der Agent übernimmt jetzt ausschließlich
Simproduktion mit vor Workrequest reservierten immutable Publicationgruppen
und retained Outbox. [Vertragsreview](agent-tasks/SIM-03_4_ITEM_PUBLICATION_CONTRACT_REVIEW.md);
Runtimeabnahme und Integration folgen separat.
Die alten NBT-/MUTF8writer bleiben die Wireauthority. Anschließend folgen
versiegelte Sim-Publikationsfakten und Sessionviewertracking. Die neuen
Metadata-/Motion-/TakeItempakete, ihre Netklassifikation und der Apptracefix
sind integriert. Das aktiviert noch keine Live-Item-Entities.

ITEM-SIZE-01 ist unabhängig CLEAN als `b5551395` integriert. Seine Größen- und
Repräsentations-APIs vermeiden die zuvor gefundenen temporären FCIS-Buffer;
DUR-04.2a adoptiert diese APIs vollständig im geprüften Umfang. Physik, Merge,
automatischer Pickup, Ordinary-Block-/Command-Ownedconsumer und gemeinsame
Player-/Chunkdurability müssen vor der produktiven Format6-Umschaltung folgen.
Keine Survivaletappe ist vollständig abgenommen. Drei Childslots bleiben die Grenze.

Root hat die integrierte Runtime `62a5b74d` in einem detached Checkout
geprüft: 2.304 Tests, 38 Doctests, strict Clippy/fmt und Generatorcheck grün,
getrennt von aktiven Agentänderungen. Gesamt-CI für `9025c687`, einschließlich
UUID-Storageproducer und Densitycompiler, ist erfolgreich:
[Lauf38080572897](https://github.com/jstrausd/ferrumc/actions/runs/38080572897).
Der neuere Gesamtstand `83ea2c6d` mit SIM-03.3 ist ebenfalls vollständig grün:
[Lauf38081893560](https://github.com/jstrausd/ferrumc/actions/runs/38081893560).
Gesamt-CI für `34acc8e9`, einschließlich STORAGE-ENTITY001.2, ist erfolgreich:
[Lauf38082651584](https://github.com/jstrausd/ferrumc/actions/runs/38082651584).
Die neue Gesamt-CI für `290e5343`, einschließlich GEN-05.2, ist erfolgreich:
[Lauf38083747393](https://github.com/jstrausd/ferrumc/actions/runs/38083747393).
Gesamt-CI für `572083d1`, einschließlich aller normalen Craftingrezepte, ist
ebenfalls erfolgreich:
[Lauf38085964215](https://github.com/jstrausd/ferrumc/actions/runs/38085964215).
Gesamt-CI für `eaedabd2`, einschließlich DUR-03.1, ist erfolgreich:
[Lauf38087498562](https://github.com/jstrausd/ferrumc/actions/runs/38087498562).
Gesamt-CI für `e7565c38`, einschließlich GEN-05.3, ist erfolgreich:
[Lauf38089465752](https://github.com/jstrausd/ferrumc/actions/runs/38089465752).
Gesamt-CI für `e563b971`, einschließlich NET-04.0, ist erfolgreich:
[Lauf38090777379](https://github.com/jstrausd/ferrumc/actions/runs/38090777379).
Gesamt-CI für `4f7d5037`, einschließlich DUR-04.1, ist erfolgreich:
[Lauf38091723034](https://github.com/jstrausd/ferrumc/actions/runs/38091723034).
Gesamt-CI für `2a4a6cd4`, einschließlich GEN-05.4 und NET-04.0b/.0c, ist
erfolgreich: [Lauf38093337400](https://github.com/jstrausd/ferrumc/actions/runs/38093337400).
Der ältere Lauf38092928296 für `f6eb7111` wurde durch diesen neueren Lauf
abgebrochen; er wird nicht als bestandene Abnahme geführt.
Gesamt-CI für `18acd2be`, einschließlich Registry-/Item-Wire-/Netproducer,
ITEM-SIZE-01 und Apptracefix, ist vollständig erfolgreich:
[Lauf38096358915](https://github.com/jstrausd/ferrumc/actions/runs/38096358915).
Der ältere Lauf38095460007 scheiterte mit dem konkret behobenen App-E0004 und
wird nicht als bestanden geführt. Die gemeinsame CI für Biome und Counter auf
`f753e312` ist vollständig erfolgreich:
[Lauf38097220739](https://github.com/jstrausd/ferrumc/actions/runs/38097220739).
Auch die Gesamt-CI für `65064bc7`, einschließlich NBT-WRITE-01, ist
vollständig erfolgreich:
[Lauf38098204450](https://github.com/jstrausd/ferrumc/actions/runs/38098204450).
GEN-05.6 und NET-04.2a sind lokal unabhängig CLEAN integriert; gemeinsame
Gesamt-CI für den neuen veröffentlichten Head folgt.
Ergebnisse der vierten Welle stehen im
[Prüfbericht](AGENT_WAVE_04_VALIDATION.md); eine ältere grüne CI ist kein
Nachweis für diese neue Runtime.

## Vorbereitete unmittelbare Verbraucher

GEN-05.6, NET-04.2a und DUR-04.2a sind unabhängig CLEAN integriert.
Unmittelbar folgen Biomtemperatur-/Oberflächenabhängigkeiten sowie
Storage-Format6migration, versiegelte Simpostimages und Sessiontracking.
Der echte Format6-Appdriver folgt nach
seinen Storage-/Wire-/Session-/Ordinary-Block-/Commandabhängigkeiten.
Vor dem Live-Drop-/Pickupdriver ist
[NET-04.1](agent-tasks/NET-04_1_ITEM_ENTITY_WIRE.md) konkret vorbereitet:
Metadata0x5c, Motion0x5e und TakeItem0x75 sowie ihre echte Netklassifikation
sind mit NET-04.1/.1b/.1c integriert und Gesamt-CI-grün.
NBT-WRITE-01 und Itemmetadata sind integriert; versiegelte Simpostimages,
Player-Wirebindung und Sessiontracking fehlen noch für den Liveconsumer.
Zusätzlich ist [SRV-05.2](agent-tasks/SRV-05_2_SINGLE_ITEM_RECIPES_FUEL.md)
für alle114 Koch-/254 Steinschneiderrezepte und originale Brennstoffe vorbereitet;
Vergabe folgt nach dem Wireproducer. Er aktiviert noch keinen Ofen.
Root hat zusätzlich49 echte Paper-Materialloops mit3.325.952 States zur
Ressourcenplanung ausgeführt; normal/-O bytegleich. Das ist keine Rust- oder
Spielabnahme. DUR-04.1 berücksichtigt die tatsächliche verbleibende Lücke:
das aktive Inventar gehört derzeit der Appconnection und muss für bestätigte
Player↔Entitytransfers in dieselbe besessene Simauthority gelangen.

Für bestehende Daten ist außerdem die ausdrücklich getrennte
[DUR-03.2-Format6migration](agent-tasks/DUR-03_2_OWNERSHIP_FORMAT6_MIGRATION.md)
vorbereitet. Sie besitzt zunächst nur den Offline-Storageproducer mit
unveränderlicher Player-Scopezuordnung und vollständigen Bootstrap-/Receipt-
Belegen. Der jetzige Format4start wird nicht durch einen Markerwechsel ersetzt.

Für die anschließende Biomephase liegt außerdem ein tatsächlicher Paper-
Parameterexport vor:7593 geordnete Overworldbereiche/54Biomes und5 Netherbereiche.
Originale quantisierte long-Grenzen und Reihenfolge erhalten;19 tatsächliche
Klassenursprünge, frischer Helper und120 JAR-Artefakte geprüft. Normal/-O liefert
bytegleiche Parameter und vollständige Manifeste. DatenSHA
`4e9ee584071de6822b9e0e8030eb0666814a3c61b77f8e0ce611f9757508f971`,
10-Dateienfreeze `.codex-tmp/gen05_5-root-biome-census.freeze.json`, SHA
`95deae4a95856a4a80cb3ec71c1c251e1852b6ecc0a1506d6aa811433e28729b`.
Der separate Rustselector ist mit GEN-05.5 abgenommen; dieser Census allein
belegt weiterhin keinen Surface- oder Clientpass.

Der getrennte [DUR-05-Appbrief](agent-tasks/DUR-05_FORMAT6_APP_DRIVER.md)
konkretisiert die tatsächliche Umschaltung auf Format6 und FCPS3. Vor dieser
Umschaltung benötigen gewöhnliche Block-/Commandaktionen eigene Ownedconsumer;
Placement mit Itemverbrauch verlangt gemeinsame Player-/Chunkdurability.
Der Appdriver ist vorbereitet, noch nicht vergeben oder aktiviert.
[NET-04.2a](agent-tasks/NET-04_2a_ITEM_METADATA_BUILDER_REVIEW.md) ist als eigener
Items-only-Metadatabuilder unabhängig CLEAN integriert: trusted Slotwirepreflight
vor Allocation und vollständiger Eintrag innerhalb der opaque Protohülle.
[NET-04.2b](agent-tasks/NET-04_2b_ITEM_VIEWER_TRACKING.md) konkretisiert das
Sessionviewertracking, bevor DUR-05 Liveentities aktiviert. Fehlende echte
Simpublicationfacts benötigen zuerst einen eigenen Producer; die Session
erzeugt keine dauerhafte Mutationsauthority.

## Orchestrierung

- Root priorisiert, nimmt Nutzerbefehle an, vergibt Tickets, organisiert Review
  und Tests und integriert auf `main`; Gameplaycode schreiben die Agents.
- Höchstens drei aktive Child-Agents gleichzeitig. Recherche/Katalog/Review
  beanspruchen ebenfalls einen Slot. Freie Slots erhalten abhängigkeitstreue
  Folgetickets.
- Jeder Agent besitzt ausdrücklich zugewiesene Dateien. Ein Umsetzungsticket
  verändert eine Produktions-Crate; schichtübergreifende Features erhalten
  aufeinander aufbauende Tickets. Gemeinsame Dateien werden nicht parallel editiert.
- Nur Root führt Git-Commits und Pushes aus. Agents melden Änderungen,
  Quellenreferenz, Tests und verbleibende Grenzen; ein anderer Agent prüft sie.
- Status folgt der Evidenz: geplant, in Arbeit, implementiert/noch ungeprüft,
  integriert/geprüft oder mit konkreter Abhängigkeit wartend. Ein Hilfsvertrag
  gilt nicht als aktiviertes Gameplayfeature.
- Kein Agent markiert das Gesamtspiel als fertig, solange die Inhaltsmatrix,
  echte Mehrspieler-Survival-/Dimensionsläufe, Neustart-/Recovery-Tests,
  Paper-Vergleiche und deklarierte Pluginmatrix offene Fälle enthalten.
- Dieses Board ist persistierter Arbeitsstand. Ein beendeter Agentlauf oder
  eine beendete Sitzung ist kein zugesicherter Hintergrunddienst.
