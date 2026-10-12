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

Root hat DUR-03.2 unabhängig CLEAN geprüft und als `242c8f07` auf
`main` veröffentlicht. Debug/Release jeweils299 Tests und strictchecks bestehen
im frischen `b87d5b61`-Archiv mit exakt61 Storagefiles;62 Source-/36
Evidenzfiles und das vollständige Quellarchiv sind hashgeprüft.
[Prüfbericht](agent-tasks/DUR-03_2_OWNERSHIP_FORMAT6_MIGRATION_REVIEW.md).
Die Migration verwendet tatsächliche private Stage-CASreceipts und einen
atomaren Sourceimport; Appmigration und FCPS-Gameplayadmission bleiben offen.

`select_world_ticket` hat den konkreten
[NET-04.2c-Vertrag](agent-tasks/NET-04_2c_COLLECTOR_SESSION_BINDING.md) abgeschlossen.
Root hat alle fünf Freeze-, 25 Referenz-, 44 Source- und 53 akzeptierten
Simdateien ohne Drift geprüft und den Vertrag unabhängig CLEAN abgenommen.
[Vertragsreview](agent-tasks/NET-04_2c_COLLECTOR_SESSION_BINDING_REVIEW.md).
Der tatsächliche Simproducer
[SIM-03.4a](agent-tasks/SIM-03_4a_COLLECTOR_LIFECYCLE.md) implementiert borrowed
ConnectedPlayerView, bounded Lifecyclezellen, applied Detachfacts und
reserved/retained Takepins. Session-/App-Wiring und der native Join folgen
danach; heutiger Appjoin bleibt der Legacyplayerpfad mit eigener JoinGame-ID1.
SIM-03.4a ist unabhängig CLEAN_RUNTIME abgenommen und als `394d4c27` integriert.
Root hat die Kombination im echten `e772564c`-Archiv geprüft: Debug/Release
jeweils369, Workspace-Alltarget-Clippy, fmt und rustdoc grün;56 Source-/40
Evidence-Dateien sowie1215 Außendateien unverändert. Der neue unabhängige
Reviewer hat alle15 geänderten/neuen Dateien und ihre vollständigen
Produktionspfade geprüft; zwei eigene Tests mit vier Memory/redb-Szenarien
bestanden. Die erste unterbrochene Reviewrunde lieferte keine Abnahme.
[Runtimeabnahme](agent-tasks/SIM-03_4a_COLLECTOR_LIFECYCLE_REVIEW.md).
Sessionretirement, Consumerreferenzen und Appaktivierung bleiben eigene Verbraucher.

GEN-05.6 ist unabhängig CLEAN als `34c8875e` integriert. Der konkrete
GEN-05.7-Temperaturvertrag ist CLEAN auf Vertragsumfang. Die finale genuine
Originalserie umfasst129588 Temperaturfälle je Original,65 Biome und alle
Frozenzweige. Root reproduzierte den gesamten Lauf und12 Guardrefusals;
26 Ausgabedateien sind bytegleich,119 Evidence-/64 Design-/757 tatsächliche
Inputdateien vor/nach hashgeprüft. Ein eigener T3-Reviewer hat die vollständige
Oraclequelle unabhängig CLEAN geprüft. Root hat daraufhin die Worldproduktion
an den T3-Agenten „GEN-05.7 originale Biomtemperatur in Rust“ vergeben.
[Oraclereview und Freigabe](agent-tasks/GEN-05_7_BIOME_TEMPERATURE_ORACLE_REVIEW.md).
Der finale 234-Datei-Umfang `53f29051` ist unabhängig CLEAN_RUNTIME abgenommen
und als `40488324` auf `main` integriert: Whole-World Debug/Release jeweils285
Tests, striktes Alltarget-Clippy, fmt, rustdoc und explizite kalte
Allokationsproben grün. Der absichtlich ignorierte Instrumentierungstest wurde
separat in beiden Modi ausgeführt. Alle43 Evidence-Dateien sind frisch gegen den
Freeze geprüft;1028 Außendateien und alle1224 Baseline-Dateien blieben unverändert.
Der alte Harnesskandidat ist ausdrücklich vor Abnahme verworfen.
[Runtimeabnahme](agent-tasks/GEN-05_7_BIOME_TEMPERATURE_RUNTIME_REVIEW.md).
Der freigewordene Worldslot bearbeitet jetzt
[GEN-05.8 stored biome resolver](agent-tasks/GEN-05_8_STORED_BIOME_RESOLVER_TASK.md):
konkreter Originalvertrag und echte ProtoChunk-/BiomeManager-Oracle, zunächst
ohne Productionänderungen. Danach folgen Surface-Regeln und der Columndriver.
Vertrag und Originaloracle sind unabhängig CLEAN_CONTRACT: Der Reviewer hat den
ganzen Oraclelauf normal und mit Python `-O` frisch reproduziert; alle28
Differentialdateien, beide Manifeste und alle15 Guardverweigerungen sind
bytegleich, alle211 Freezehashes stimmen weiterhin.
[Vertragsreview](agent-tasks/GEN-05_8_STORED_BIOME_RESOLVER_REVIEW.md).
Root hat die Worldruntime mit den Auflagen A1–A5 freigegeben:
[Runtimefreigabe](agent-tasks/GEN-05_8_STORED_BIOME_RESOLVER_RUNTIME_TASK.md).
Die Worldruntime `stored_biome` ist unabhängig CLEAN_RUNTIME abgenommen und
integriert: Whole-World Debug/Release jeweils320 Tests (2 ignorierte
Messharnesses separat ausgeführt), striktes Workspace-Clippy, fmt, rustdoc und
1195 Verbrauchertests grün. Alle220960 Queryzeilen und243 Arrays sind
unabhängig gegen die rohen Java-Ausgaben zurückverfolgt; Build/Clone/Query
allokieren nicht (positive Kontrolle4096 B). A3 war wörtlich unerfüllbar, weil
GEN-05.6 keine Vanilla/Paper-Differenzen hat; der test-only Ersatz tötet fest
verdrahtete Modi und falsche Seeds. Offene Auflage R-A1 für das nächste
Worldticket: ein Test, der die zweiphasige Halo-Refusalreihenfolge festlegt.
[Runtimeabnahme](agent-tasks/GEN-05_8_STORED_BIOME_RESOLVER_RUNTIME_REVIEW.md),
integriert als `251efaf9`. Der freie Worldslot bearbeitet jetzt
[GEN-05.9 Surface-Regeln](agent-tasks/GEN-05_9_SURFACE_RULES_TASK.md): originaler
Overworld-Regelbaum und Einzelblockauswertung, zuerst Vertrag und echte
Originaloracle ohne Productionänderungen. Der Columndriver folgt separat.
Nach drei Reviewrunden ist der Vertrag CLEAN_CONTRACT: Runde 1 fand einen
fabrizierten Flat-Bedrock-Probekontext (jetzt nur `Disabled`), Runde 2 einen
Konflikt der Replayregel mit der eigenen Höhen-/Stone-Depth-Policy (jetzt
eigene Klasse `refused`). Die Runtime ist
[freigegeben](agent-tasks/GEN-05_9_SURFACE_RULES_RUNTIME_TASK.md), mit
Pflicht zum direkten Bitvergleich von `clay_bands_offset` über eine erweiterte
Kopie des Originalhelpers. Die Worldruntime `surface_rules` ist unabhängig
CLEAN_RUNTIME integriert: World Debug/Release je360 und 1426 Verbrauchertests
grün; der Reviewer hat alle Replayklassen aus den 765.138 Rohzeilen neu
abgeleitet, alle 720 Arrays gegen die Originalindizes und die Clay-Bits über
20.480 Spalten selbst verglichen.
[Runtimeabnahme](agent-tasks/GEN-05_9_SURFACE_RULES_RUNTIME_REVIEW.md).
Als nächster Worldschritt ist der SurfaceSystem-Columndriver GEN-05.10 in Runde 1
unabhängig CLEAN_CONTRACT: echtes `buildSurface` mit und ohne aufzeichnenden
Proxy in 72/72 Gruppen gleich, alle 51 Overworld-Oberflächenbiome, Icebergs und
Badlands-Pfeiler. Die Runtime ist mit den Policies D1–D5 und den Auflagen A1–A6
sowie RR-H1/RR-H2 [freigegeben](agent-tasks/GEN-05_10_SURFACE_COLUMN_DRIVER_RUNTIME_TASK.md)
und nach der ersten Reviewrunde unabhängig CLEAN_RUNTIME integriert: Untermodul
`surface_rules::column` mit `build_chunk`, eine frische GEN-05.9-Session je
Chunk, Alles-oder-nichts-Veröffentlichung in einen wiederverwendbaren Puffer.
Der Replay trifft 70 Gruppen, 17.920 Spalten und 2.261.637 Aufrufe exakt;
`build_chunk` allokiert in Debug und Release nicht. Wegen A4 hat der Root
`W_col` 2698 und `R` 888.832 angenommen (vorher 2697 und 888.576; Draws bleiben
≤ 384). World Debug/Release je 383 und alle Worldverbraucher 1489 grün;
Testlückenhinweise RR10-H1–H5 gehen an das nächste Worldticket.
[Runtimeabnahme](agent-tasks/GEN-05_10_SURFACE_COLUMN_DRIVER_RUNTIME_REVIEW.md).
Der nächste Worldschritt sind die Overworld-Carver
([GEN-05.11](agent-tasks/GEN-05_11_OVERWORLD_CARVERS_TASK.md)), zuerst als Vertrag
mit echter Originaloracle.
Die aktive App bleibt Flat-v1/Plains; Surface-/Carver-/Featureconsumer fehlen.

### Weltgenerierung: Betreiberpriorität vom 12.10.2026

Der Betreiber hat die Weltgenerierung als nächsten Schwerpunkt festgelegt.
GEN-05.11 hat den exklusiven World-Runtimeauftrag abgeschlossen und den
vollständigen Kandidaten eingefroren. Der Runtime-Review ist FINDINGS wegen
RR11-M1; die [Korrekturlieferung09](agent-tasks/GEN-05_11_CARVER_PRECHARGE_CORRECTION_DELIVERY.md)
ist vollständig eingefroren. Der neue [unabhängige Runtimereview Runde2](agent-tasks/GEN-05_11_CARVER_PRECHARGE_CORRECTION_REVIEW_TASK.md)
prüft jetzt die tatsächliche Refusal-Reihenfolge auf einer isolierten aktuellen
main-Basis. GEN-06.2 wartet weiter auf Abnahme und Integration. Der unabhängige Vertragskorrekturreview
ist CLEAN_CONTRACT/CLEAN_ORACLE. Root nimmt den privaten unsigned192-Meter-
und exakten Fehlerpfadentwurf an; die Umsetzung und Runtimeabnahme folgen getrennt.

| Reihenfolge | Nächster Lieferumfang | Aktueller Stand / Freigabekriterium |
| --- | --- | --- |
| 1 | GEN-05.11: Höhlen, zusätzliche unterirdische Höhlen und Schluchten, Masken, Wasser/Lava und Oberflächenkorrektur | [Korrekturreview CLEAN_CONTRACT/CLEAN_ORACLE](agent-tasks/GEN-05_11_OVERWORLD_CARVERS_REVISION2_REVIEW.md), CR11-B1/B2 auf Vertragsebene geschlossen. [Runtime-Review044 FINDINGS](agent-tasks/GEN-05_11_OVERWORLD_CARVERS_RUNTIME_REVIEW.md): Klima-f64→f32 läuft vor ihrer Budgetreservierung. [Korrekturkandidat09 eingefroren](agent-tasks/GEN-05_11_CARVER_PRECHARGE_CORRECTION_DELIVERY.md), World Debug/Release jeweils418 bestanden/7ignored; betroffene Kaltstartproben separat bestanden, alte Reihenfolge in beiden Profilen assertion-killed. Root hat945 Paket-/7Seal-/364Source-/577Evidence-Mitglieder und tatsächliche Nachweise geprüft. [Unabhängiger Runtimereview Runde2](agent-tasks/GEN-05_11_CARVER_PRECHARGE_CORRECTION_REVIEW.md) ist CLEAN_RUNTIME_REVIEW; RR11-M1 geschlossen. Root prüfte483 Reviewpaket-/4Seal-Mitglieder vollständig und nimmt den bounded kalten Producer an. Der vollständige364-Dateien-Worldstand wird integriert; frische Whole-CI folgt. Appaktivierung bleibt offen. |
| 2 | GEN-06.1: gewöhnliche Erzfeatures und ihre Platzierung; danach Vegetation und weitere Features | [Revision2](agent-tasks/GEN-06_1_ORE_PLACEMENT_REVISION2.md) ist unabhängig [CLEAN_CONTRACT/CLEAN_ORACLE](agent-tasks/GEN-06_1_ORE_PLACEMENT_REVISION2_REVIEW.md), B1/B2/B3 auf Vorbereitungsebene geschlossen. Root nimmt 16-KiB-Katalog/16-KiB-Scratch/32-KiB-neuen Peak/49.152-B-mit-altem-Katalog und 512-B-Cursor als messpflichtige Designlimits an. [GEN-06.2 Rust-Primitive](agent-tasks/GEN-06_2_CLOSED_ORE_PLACEMENT_PRIMITIVES_RUNTIME_TASK.md) wartet auf den exklusiven Worldslot und frische akzeptierte Basis. [GEN-06.3 Review Revision2](agent-tasks/GEN-06_3_ORE_ALGORITHM_REVISION2_REVIEW.md) ist FINDINGS: Query53-Abgleich, typisierte Bootstrapkonstanten und ganze Trace-Membership wurden in der [Autorenantwort Revision3](agent-tasks/GEN-06_3_ORE_ALGORITHM_REVISION3.md) korrigiert; der abgeschlossene [unabhängige Review Revision3](agent-tasks/GEN-06_3_ORE_ALGORITHM_REVISION3_REVIEW.md) ist CLEAN_CONTRACT/CLEAN_ORACLE im kontrollierten Präparationsumfang. Root hat vollständige Bytes/Membership geprüft und nimmt nur diese Vorbereitung an. Neue Regionschnittstellen/-limits bleiben unangenommen; H2 und whole-FEATURES offen. |
| 3 | Natürlichen Generator vollständig mit Chunkmodell, Biomen/Heightmaps, Licht und Generatorpersistenz verbinden (GEN-03/GEN-11/GEN-12) | Getrennte Producer-/Consumer-/App-Tickets mit Generation außerhalb des Netzwerk- und Tickpfads. Bestehende Flat-Welten behalten ihre gespeicherte Generatoridentität. Noch keine Naturweltaktivierung. |
| 4 | Neue Naturwelt am echten Client und nach Save/Restart prüfen; Strukturen und weitere Dimensionen fortsetzen | Keine Spielbarkeits- oder vollständige Seedparitätsabnahme aus Bibliothekstests ableiten. Vollständige Features/Strukturen, Nether und End bleiben im Gesamtbacklog. |

Die Direct-source-Biome des originalen Carver-Fix-ups und der vorgeschlagene
Stored-biome-Resolver müssen im Review getrennt bewertet werden: gleiche
Corpuswerte allein belegen keine Gleichheit bei allen Seeds/RTree-Gleichständen.
Diese Prüfung und RR10-H1–H5 bleiben Bedingungen des nächsten Worldtickets.

Der GEN-05.11 RR11-M1-Runtimekorrekturreview ist CLEAN_RUNTIME_REVIEW; Root hat die finale Lieferung geprüft und nimmt den bounded kalten Producer an. Der vollständige Worldstand wird integriert; neue Whole-CI folgt.
GEN-06.3 Revision3 ist unabhängig CLEAN_CONTRACT/CLEAN_ORACLE im kontrollierten Präparationsumfang. Der unabhängige
Storage-Review DUR-03.3b ist CLEAN_RUNTIME_REVIEW abgeschlossen. GEN-06.1 Review Runde 2
ist fertig; Root hat alle 494 Dateien/307.996.259 Bytes erneut geprüft und nimmt
nur die kontrollierte Vorbereitung an. Im Review erkannte shared-Worldänderungen
gehören dem aktiven Carverwriter und sind keine akzeptierte Basis. GEN-06.3 nutzt
ein frisches readonly `18d510e1`-Archiv mit denselben 324 akzeptierten Worlddateien.
Der eingefrorene Storage-Kandidat Runde 6 zählt Block-Entities mit ihrer
tatsächlichen Kodierlänge; Autorenchecks Debug/Release jeweils 354 grün.
Root prüfte alle 91 Quelldateien bytegleich und stellte ein readonly Archiv
des echten `c0b78c23` plus Kandidat bereit. Das ersetzt keine unabhängige
Runtimeabnahme. Der [Storage-Review Runde 3](agent-tasks/DUR-03_3_PLAYER_BLOCK_DURABILITY_RUNTIME_REVIEW_ROUND3.md)
bestätigt B1 und 23 gezielte Tests, bleibt aber wegen R6-B2 FINDINGS: ein gültiger
1024-Effekt-Commit lässt sich mit derselben 1-GiB-Scanallowance nicht wieder öffnen.
Root verlangt die Korrektur vor Integration, ohne Grenzerhöhung oder Ausnahme.
[DUR-03.3b](agent-tasks/DUR-03_3b_STARTUP_RECEIPT_REUSE_TASK.md) hält höchstens
einen Receipt während seines zusammenhängenden Journalbereichs und beseitigt
Mehrfachlesen/Hashen/Parsen bei vollständiger Zeilen- und Relationsprüfung.
Die [DUR-03.3b-Delivery](agent-tasks/DUR-03_3b_STARTUP_RECEIPT_REUSE_DELIVERY.md)
ist eingefroren: 93 Quelldateien, 42 Evidenzartefakte; Autorenchecks
Debug/Release jeweils 360 bestanden. Die ursprüngliche 1-GiB-Datenbank öffnet
laut Autorenprobe wieder, drei unabhängig hergeleitete Scan-Grenzen sind geprüft.
Root hat Kandidat, readonly Kopien und Evidenz vollständig bytegeprüft und
stellt echtes `4225c21c` mit vollständigem Storageoverlay bereit. Außerhalb
Storage unterscheiden sich Autorenbasis und aktuelles main nur in 11 Docpfaden.
Der [neue unabhängige Runtime-Review](agent-tasks/DUR-03_3b_STARTUP_RECEIPT_REUSE_RUNTIME_REVIEW.md)
ist CLEAN_RUNTIME_REVIEW: R6-B2 geschlossen, 25 eigene Prüfungen grün,
ursprüngliche 1-GiB-Datenbank real wieder geöffnet. Root hat alle 1.544
Reviewartefakte sowie 93 Sourcebytes erneut geprüft und nimmt die
[Format7-Bibliothek zur Integration an](agent-tasks/DUR-03_3_ROOT_RUNTIME_ACCEPTANCE.md).
Der frische aktuelle Mainvergleich enthält außerhalb Storage nur Docänderungen;
alle Runtime-/Dependencybytes stimmen mit der geprüften Kombination überein.
Keine Appaktivierung; DUR-03.3a bleibt vor DUR-05 erforderlich.

Die vollständige Format7-Bibliothek ist als `47eadc4c` auf main integriert und
gepusht. Der CI-Lauf `38254637982` erreichte 3.067 von 3.070 Tests ohne gemeldeten
Testfehler und wurde am 30-Minuten-Joblimit abgebrochen. Das Joblimit wird auf
60 Minuten erhöht; Tests, Debugprofil und anschließende Clientprüfungen bleiben
vollständig erhalten. Dieser abgebrochene Lauf ist keine grüne CI-Abnahme.
Der neue vollständige CI-Lauf `38260056240` auf `26d8bf8b` ist erfolgreich:
Rust-, Java-, Daten-/Text- und alle unabhängigen Clientprüfungen sind grün.
Der spätere Commit `b9f6335e` enthält ausschließlich Dokumentationsänderungen.

[GEN-06.3](agent-tasks/GEN-06_3_ORE_ALGORITHM_ORACLE.md) hat einen eingefrorenen
Originalchronologie-/Kernelvertragskandidaten geliefert: 101 kontrollierte Fälle
je Vanilla/Paper-Profil und drei zusätzliche Wrapper-Endzustandskontrollen.
Root prüfte alle 3.878 Dateien/599.591.972 Bytes ohne Abweichung. Der
[unabhängige Review](agent-tasks/GEN-06_3_ORE_ALGORITHM_ORACLE_REVIEW_TASK.md)
prüft Instrumentierung, Schreib-/RNG-/Sectionreihenfolge und den vorgeschlagenen
16-MiB-Regioncap. Besonders offen ist die Nutzbarkeit des unveränderten
1-Mi-Arbeitsbudgets nach mindestens 914.945 Einheiten für Kopie und Validation.
Der [Review Runde 1](agent-tasks/GEN-06_3_ORE_ALGORITHM_ORACLE_REVIEW.md) ist
FINDINGS: CR63-B1 blockiert die Zusammensetzung im 1-Mi-Meter (Full30 benötigt
mindestens 2.003.339/1.955.293 Einheiten; schon ein vollständiger Clayplan passt
nicht). CR63-B2 zeigt eine akzeptierte wirksame Rechenmutation im Inverseprüfer;
CR63-B3 zeigt nicht erkannte Branch-/Sphärenbitmutationen im Validator.
Root hat alle 2.383 Reviewartefakte/2.773.901.576 Bytes frisch geprüft und
verlangt die [Korrektur Revision 2](agent-tasks/GEN-06_3_ORE_ALGORITHM_REVISION2_TASK.md)
vor Runtime. Getrennte neue Compositionauthority und fehlerfeste Prüfwerkzeuge
werden vorbereitet; vorhandene Primitive-/GEN-Limits bleiben unverändert.
Noch keine Kernel-Runtimefreigabe oder Whole-FEATURES-Abnahme.

Die neue [GEN-06.3-Antwort Revision2](agent-tasks/GEN-06_3_ORE_ALGORITHM_REVISION2.md)
ist eingefroren. Root hat alle5897 Dateien/14.045.912.630 Bytes und12293 externe
Inputs/5.041.635.179 Bytes vollständig geprüft. Neue Originalkontrollen liefern318
Feature-Endstates je Profil; die finalen Tools prüfen exakte Computational-
Templates und verpflichtende externe verlustlose Referenzen. Die finale
Autorenmodellrechnung zählt Copyheads separat:3.766.873 Einheiten,427.431
unter dem vorgeschlagenen neuen2^22-Regionceiling; native Modellsumme15.042.707 B.
Diese Zahlen sind keine Rust-/Nativeabnahme. Der
[abgeschlossene unabhängige Review Runde2](agent-tasks/GEN-06_3_ORE_ALGORITHM_REVISION2_REVIEW.md)
bleibt FINDINGS. CR63-B1 HIGH verlangt denselben Query53-Kernel am aktiven
Parentmeter und explizit bounded Chunkselection. Die unabhängig geprüfte
Querykorrektur ergibt maxPP3.848.413/Margin345.891 zu weiterhin pending2^22.
CR63-B2 BLOCKING belegt eine gültige Bootstrap-Type→String-Mutation:
Originalreturnfalse wird BootstrapMethodError, der Proof akzeptiert sie dennoch.
CR63-B3 HIGH belegt drei vollständige CLI-Falseacceptances durch zusätzliche
mutierte gzip-/nested-Traces beziehungsweise eine externe --trace-Substitution.
Root hat sämtliche12543 finalen Reviewdateien/21.699.636.198 Bytes sowie
Author-/Outside-/Historical-/JDK-/Archiv-/späte Heightmapauthorities vollständig
erneut geprüft. Der frühere primäre Freeze bleibt als historischer Snapshot
erhalten; maßgeblich ist der vollständige enclosing Freeze e97293eb.
Root verlangt [gezielte Korrektur Revision3](agent-tasks/GEN-06_3_ORE_ALGORITHM_REVISION3_TASK.md):
typisierte executable Konstanten, genaue rekursive Payloadmembership und
konsistenter Query-/Adapter-/Debitentwurf. Keine historische Freezeänderung.
Die vollständige [Antwort Revision3](agent-tasks/GEN-06_3_ORE_ALGORITHM_REVISION3.md)
ist geliefert: Query53 plus explizite Selection32 am selben Parent, Modellmaximum
3.904.585/Margin289.719 zu weiterhin pending2^22, typisierte Konstanten und
vollständige membergebundene Payloadprüfung. Der Autorenfreeze umfasst68900
Dateien/31.815.362.417 Bytes; mathematische Modelle und echte JVMkontrollen
bleiben getrennt von fehlenden Rustproducern. Der neue
[unabhängige Reviewauftrag](agent-tasks/GEN-06_3_ORE_ALGORITHM_REVISION3_REVIEW_TASK.md)
hat die gesamte ursprüngliche Pflicht-/Befund-/Antwortkette geprüft. Der
[abgeschlossene Bericht](agent-tasks/GEN-06_3_ORE_ALGORITHM_REVISION3_REVIEW.md)
ist CLEAN_CONTRACT/CLEAN_ORACLE: CR63-B1/B2/B3 sind auf Design-/Tool-/Originaloracle-Ebene geschlossen.
Root hat alle55.628 regulären Reviewdateien/34.229.518.250 Bytes und55.971 Entries
sowie die vollständigen Author-/Input-/Historical-Pins geprüft und nimmt nur
diese kontrollierte Vorbereitung an. Tatsächliche Rustproducer, Native-/Lifecycleabnahme
und neue Regionlimits bleiben offen.
GEN-06.2s Single-key1Mi bleibt unverändert.
Alle neuen Region-/Kapazitäts-/Interfacevorschläge bleiben bis zu Review und
Rootentscheidung unangenommen.

[GEN-03.1](agent-tasks/GEN-03_1_BLOCKSTATE_HEIGHTMAP_ORACLE.md) hat die
writerfreie Originalvorbereitung geliefert: je Original 27.946 States,
1.105 Blöcke, sechs Heightmap-Prädikate sowie 836 Tracezeilen aus zehn echten
kontrollierten EMPTY-ProtoChunks. Der vollständige Identitätsjoin meldet keine
FerrumC-/Vanilla-/Paper-Abweichungen. Root prüfte alle 202 eingefrorenen
Dateien/36.215.670 Bytes ohne Abweichung. Der
[unabhängige Review](agent-tasks/GEN-03_1_BLOCKSTATE_HEIGHTMAP_ORACLE_REVIEW_TASK.md)
ist FINDINGS: G31-R1-B1 verlangt vollständige Rawimport-/Update-/Queryarbeit
und konkrete Atomizität; G31-R1-B2 schließt den fehlenden Vollstateindex nicht
in die Buildbilanz ein; G31-R1-M1 bestätigt akzeptierte zusätzliche Output-
Verzeichnisse. Die echten Originaldaten wurden vollständig unabhängig reproduziert.
Root hat alle 741 Reviewdateien/680.487.002 Bytes ohne Abweichung geprüft und
beauftragt [Revision 2](agent-tasks/GEN-03_1_BLOCKSTATE_HEIGHTMAP_REVISION2_TASK.md):
bounded Update-Delta, vollständige Work-/Readerverträge, Blocknamen-/Radixjoin
ohne großen Vollstateindex und exakte Outputtreeadmission. Die vorgeschlagenen
Kapazitäts-/Workgrenzen bleiben unangenommen. Statischer Registryproducer und
Worldconsumer folgen als getrennte Runtimeaufträge nach erneutem Review.
Die [GEN-03.1-Korrektur Revision2](agent-tasks/GEN-03_1_BLOCKSTATE_HEIGHTMAP_REVISION2.md)
ist jetzt eingefroren:688 reguläre Dateien/474.448.752 Bytes, insgesamt879
Entries einschließlich167 Verzeichnissen,18 absichtlichen Symlinkmutanten und
sechs FIFOs. Root hat Bytes, vollständige Membership, lstat-Typen/-Modi und
Linktargets geprüft; FIFOs wurden nicht geöffnet. Alle4438 Originalpins,
175 Referenzpins,1490 Archivdateien und202/741/366/494 historische Freeze-Dateien
stimmen weiterhin. Der
[abgeschlossene unabhängige Korrekturreview](agent-tasks/GEN-03_1_BLOCKSTATE_HEIGHTMAP_REVISION2_REVIEW.md)
schließt G31-R1-B1/B2 auf Designniveau und M1 im tatsächlichen Validator.
Root hat alle1404 regulären Reviewdateien/1.116.541.431 Bytes und1817 Entries
einschließlich Typen/Modi/Linktargets sowie sämtliche Eingangsauthorities geprüft.
Das Gesamturteil bleibt FINDINGS wegen G31-R2-B1: Query53 widerspricht Ore Query8.
Die unabhängig gezählten1812 Queries im Full30-Seed0-Plan erhöhen den
Modelltotal auf3.848.413; Margin345.891 zu weiterhin pending2^22. Bounded
Chunkselection und ein gemeinsam geprüfter Kernel-/Parentmeteradapter bleiben offen.
Neue Reservationsmodelle36/491559/14163/4259119/14419/Query53 und eigene
Preparation-Allowance2^23 bleiben unangenommen. Der historische Erz-Entwurf Revision2 rechnete
Query8; die neue Revision3 schlägt53+32 und eine atomare85-Reservation am
aktiven Parent vor. Der abgeschlossene Revision3-Review bestätigt diese Verbindung
als Blueprint/Modell; der tatsächliche Getter-/Parent-/WGhistoryproducer bleibt offen. Keine stille Änderung eines Freeze oder zweiten Ore-Meters.
Root nimmt separat nur den Registryentwurf mit Static65536/Build131072,
Domain26 und bounded u64-Buildwork7391501569 als messpflichtige Limits an.
Der [Registry-only Runtimeauftrag](agent-tasks/GEN-03_1a_REGISTRY_STATE_FACTS_RUNTIME_TASK.md)
implementiert vollständigen Property-/Radixjoin, original gepinnte Prädikate,
statische Facts und CheckedStateId ohne World-/Flatänderung. Die private
Prüfbasis ist ein echtes main65c7f682-Archiv mit1505 Dateien; Shared Carver-WIP
gehört nicht dazu. Registry-Runtimeabnahme und sämtliche Worldlimits bleiben offen.
Der Carver-Runtimekorrekturreview und Ore-Review Revision3 sind abgeschlossen.
[GEN-03.1a Registry-Round3](agent-tasks/GEN-03_1a_REGISTRY_ONE_SHOT_OPEN_CORRECTION_TASK.md)
läuft exklusiv in Registry. Der Heightmap-Korrekturreview ist abgeschlossen.
GEN-06.2 bleibt bis zur Carverintegration ohne Worldwriter. Naturwelt-/Licht-/Appaktivierung
bleibt offen. Root stellte für den Carverreview ein echtes readonly `356670f7`-
Archiv bereit:1499 Dateien, alle Runtime-/Dependencybytes gleich der finalen
Autorenbasis `47eadc4c`. Die vollständige erwartete Kombination enthält1538
Dateien einschließlich362 eingefrorener Worlddateien und Delivery. Alle202
bisherigen World-Fixture-/Referenzdateien und1166 Outside-Dateien der Autorenbasis
sind unverändert. Die beobachteten null Allokationen/Deallokationen im kalten
vorbereiteten Aufruf sind Autorenmessungen; unabhängige Runtimeabnahme und
Live-Performance bleiben offen. Dies beschreibt die historische044-Reviewbasis;
Root hat inzwischen die vollständige korrigierte World09-Lieferung angenommen.

Root hat den vollständigen392-member Carverreviewfreeze, den9-member finalen
Verifikationsseal, Referenz-/Bindingpins, unveränderte362 Kandidatenquellen und
den restaurierten1538-file Privatstand geprüft. RR11-M1 ist medium: die
Klimaumwandlung wird vor ihrer Charge ausgeführt, echt in Debug/Release bei
Allowance39/59 beobachtet. Die erfolgreiche71-Gruppen-Paperreplay-/Nativeprüfung
ist davon getrennt; keine erfolgreiche numerische Drift wurde gefunden.
Die Korrekturbasis ist ein echtes mainb1798614-Archiv1509 Dateien plus ganzer
frozen044 World/Delivery als explizit unangenommener1547-Dateien-Korrekturstand.
Neue ganze Worldquelle09 mit364 Dateien und tatsächlicher Refusal-Regressions-/
Mutationstest sind jetzt eingefroren. Root hat das vollständige neue945-Paket,
den7-member Seal, alle577 Evidenzdateien, den exakten5-Dateien-Delta und
Autorenlogs/Binäridentitäten geprüft. Debug/Release jeweils418/0failed/7ignored;
betroffene kalte Carver-/Native-/71-Gruppen-Proben erfolgreich; alte Reihenfolge
kompiliert und scheitert in beiden Profilen an der tatsächlichen Cast-Assertion.
Der neue unabhängige Runde2-Review nutzt ein echtes main5602-Archiv1515 plus
ganze World09/Delivery als explizit unangenommenen1556-Dateien-Reviewstand.
Autorenbasisb179 und alte044-/Review-/f305-Nachweise bleiben getrennt erhalten.
Der abgeschlossene [Runde2-Review](agent-tasks/GEN-05_11_CARVER_PRECHARGE_CORRECTION_REVIEW.md)
schließt RR11-M1. Root hat483 Reviewpaket- und4 Seal-Mitglieder exakt geprüft,
die364 Worlddateien gegen Author/Review und Gitindex bytegleich verifiziert
und nimmt den bounded kalten Carverproducer an. Frische Kombinations-CI folgt;
App-/Naturweltaktivierung und Live-Performance bleiben offen.

GEN-03.1a Round1 lieferte [G31A-CLOSE-1](agent-tasks/GEN-03_1a_REGISTRY_STATE_FACTS_RUNTIME_DELIVERY.md)
vor Implementierung: safe std Fileclose meldet keinen Fehler. Root prüfte1552
reguläre Evidenzdateien/1884 Entries und unveränderte Registry-/Archivbytes.
Der [Fortsetzungsauftrag](agent-tasks/GEN-03_1a_REGISTRY_CHECKED_CLOSE_RESUME_TASK.md)
erlaubt exakt Build-only nix0.31.3 für Unix und io-close0.3.7 ausschließlich für
Windows-FileCloseHandle, plus notwendige Lockfileauflösung. Der Unix-io-close-
Pfad ist wegen EINTR-Normalisierung ausgeschlossen; kein lokales unsafe oder
Runtimeabhängigkeit. Adapter-, IO-, Plattform- und vollständige Registrychecks
sind noch nicht abgenommen. Die originalen Byte-/Worklimits bleiben bestehen.

GEN-03.1a [Round2-Lieferung](agent-tasks/GEN-03_1a_REGISTRY_STATE_FACTS_RUNTIME_ROUND2_DELIVERY.md)
enthält den konkreten Registryproducer und geprüften consuming Closeadapter.
Autorenchecks Registry Debug/Release72+14rustdoc, Producer22 je Profil und
strikte Checks sind grün. G31A-OPEN-1 bleibt offen: std Unix open64 wiederholt
EINTR intern; beide Producerprofile zeigen17 injizierte EINTR plus EIO als18
native Versuche. Root hat den ganzen3745-Entry-/3075-Regular-/1.352.274.403-B-
Freeze geprüft und alle66 Registryquellen plus Lock/Delivery readonly gesichert.
Der neue [Round3-Korrekturauftrag](agent-tasks/GEN-03_1a_REGISTRY_ONE_SHOT_OPEN_CORRECTION_TASK.md)
autorisiert ausschließlich nix0.31.3s Build-only-fs-Feature für eine sichere
einmalige Unix-Dateiöffnung. Keine Budgeterhöhung oder Runtimeabnahme; Windows,
native Attribution und unabhängiger Runtimereview bleiben offen.

`feature_catalog` hat NBT-WRITE-01 und NET-04.2a unabhängig CLEAN geliefert;
Root integrierte sie als `579bed62` und `67a9aa94`. SIM-03.4 ist nach unabhängigem Rootreview und frischem Kombinationsarchiv mit
DUR-03.2 als `b9f15db8` integriert. Debug/Release jeweils347 Prüfungen und alle
strictchecks bestehen;53 Source-/24 Evidenzfiles sind gebunden. Vor Workrequest
reservierte immutable Gruppen und retained Outbox sind implementiert.
[Runtimeprüfung](agent-tasks/SIM-03_4_ITEM_PUBLICATION_REVIEW.md).
Sessiontracking, tatsächlicher native Join/Detach und vollständige terminale
Ausgabedisposition sind noch offene Verbraucher.
Die alten NBT-/MUTF8writer bleiben die Wireauthority. Anschließend folgen
versiegelte Sim-Publikationsfakten und Sessionviewertracking. Die neuen
Metadata-/Motion-/TakeItempakete, ihre Netklassifikation und der Apptracefix
sind integriert. Das aktiviert noch keine Live-Item-Entities.

Der konkrete DUR-03.3-Vertrag ist unabhängig CLEAN abgenommen:
1224 Archiv-, 461 Dependency-, 45 redb-, 82 Paper- und 16 Freeze-Dateien ohne
Drift;216 unabhängige Grammatik-Arithmetikfälle bestanden.
[Vertragsreview](agent-tasks/DUR-03_3_PLAYER_BLOCK_DURABILITY_REVIEW.md).
Storage-Format7 mit gemeinsamer Player-/Chunkrevision, echten Snapshotautoritäten
und atomarer Inventory-/Blockwirkung ist nach unabhängiger Runtimeabnahme
angenommen. Keine getrennten Legacywrites dürfen die gemeinsame Wirkung umgehen;
der produktive App-/Sim-Verbraucher bleibt separat. DUR-03.3a, die explizite Offline-Migration
Format6→7, folgt separat vor DUR-05.
Die aktiven Aufträge wurden nach Providerabbrüchen mit ihrer bestehenden Arbeit
wieder aufgenommen: GEN-05.8-Vertrag/Originaloracle, DUR-03.3-
Storageimplementierung und NET-04.2d. Kein abgebrochener Auftrag wurde als
abgenommen gewertet.
Der [native Driver-/Endpointworkorder NET-04.2d](agent-tasks/NET-04_2d_NATIVE_DRIVER_ENDPOINT_TASK.md)
hat einen eingefrorenen Vertragskandidaten geliefert. Root hat vor dem Freeze
fünf belegte Lücken entschieden: eigene native Endpointart statt
GameInput-Receiver, 8x8-Regionsgrenze als sichtbare Aktivierungsgrenze,
Retirement für zugelassene, nie gebundene Joins, Formatgates hinter
DUR-03.3/03.3a/DUR-05 und das neue Simticket SIM-03.4b zur Freigabe abgemeldeter
Spielerzustände nach bestätigtem Save. Nach drei unabhängigen Reviewrunden
(FINDINGS B1–B3, dann N1–N3) ist der Vertrag CLEAN_CONTRACT; der dritte Reviewer
hat alle202 Anker reproduziert und die Stopdurchrechnung für4096 Items
unabhängig nachgerechnet. Root hat die sechs Auflagen verbindlich entschieden:
kompilierbare Revalidierungssignatur, Bewegungscoalescing je Session,
konfigurierbare Stopfrist mit `Unresolved`, periodischer Player-Autosave vor
Liveaktivierung, eigenes Simticket gegen dauerhaft `Degraded` durch quarantinierte
Pendings und typisierte Disposition fehlschlagender Recoveryboundaries.
[Vertrag](agent-tasks/NET-04_2d_NATIVE_DRIVER_ENDPOINT.md),
[Rootentscheidungen](agent-tasks/NET-04_2d_NATIVE_DRIVER_ENDPOINT_ROOT_DECISIONS.md).
Das erste Runtimeticket NET-04.2e1 (nur `ferrumc-session`) ist CLEAN_RUNTIME
integriert: native Endpointart, bounded Lane mit Bewegungscoalescing je Session,
Revalidierung per Probe, Join-Reservierung/-Admission und minimales
`disconnect_native`. Nach dem Reviewbefund B1 laufen alle Pflichttests auf
Memory und redb; Root hat Session Debug/Release je206 Tests, Clippy und fmt
selbst bestätigt. Storage und World sind nur Testabhängigkeiten der Session.
[Delivery](agent-tasks/NET-04_2e1_NATIVE_ENDPOINT_SESSION_RUNTIME_DELIVERY.md),
[Review](agent-tasks/NET-04_2e1_NATIVE_ENDPOINT_SESSION_RUNTIME_REVIEW.md);
die Auflagen A1–A7 gehen an e2/f. Auch [SIM-03.4b](agent-tasks/SIM-03_4b_DETACHED_PLAYER_RELEASE_REVIEW.md)
(nur `ferrumc-sim`) ist unabhängig CLEAN_RUNTIME integriert: Freigabe abgemeldeter
nativer Spielerzustände nur nach bestätigtem Save, rein lesender Status,
`owned_stop_ready` mit dem gemeinsamen Stopprädikat, `lifecycle_acked` und der
Storage-Re-Export für die Session. Sim Debug/Release je394 und Session206 gegen
das neue Sim grün; fünf eigene Reviewerprobes auf Memory und redb. Ein fremder
Owner meldet weiter `StaleHandle` (offengelegt; e2 behandelt beides als
Quarantäne). [NET-04.2e2](agent-tasks/NET-04_2e2_NATIVE_SESSION_LIFECYCLE_REVIEW_ROUND2.md)
(nur `ferrumc-session`) ist nach zwei Reviewrunden CLEAN_RUNTIME integriert:
Join mit übernehmbarem Tabelleneintrag, Retention-Record mit Original-Handle bis
zum Release, Leave-Save-Zustandsmaschine mit typisierter Quarantäne, Release nur
über die SIM-03.4b-APIs, G3-Adoption, Audience/Reconcile und Pose-/Mode-Resync.
Runde 1 fand einen Driverabbruch beim Stop-Final-Save, einen nie endenden
zurückgehaltenen Disconnect und einen Vertragsfehler in der Wiederholungszählung;
der Root hat §7.3 dazu um den Nachtrag V1 ergänzt. Session Debug/Release je244,
App375 und Testkit100 grün. Den fehlenden Sim-Producer für den Held-Slot-Resync
liefert das unabhängig CLEAN_RUNTIME integrierte
[SIM-03.4c](agent-tasks/SIM-03_4c_SELECTED_SLOT_VIEW_REVIEW.md); die Verdrahtung
folgt in NET-04.2f. Für R3-A5 ist der Vertrag
[SIM-03.5](agent-tasks/SIM-03_5_QUARANTINE_ISOLATION.md) nach zwei unabhängigen
Reviewrunden angenommen: Ein quarantiniertes Item-Pending friert im
Countermodus seine Projektion ein, schirmt sein Item ab und gibt die
Storereservation frei, statt den Shard nach `defer` Claims in
`CounterRecoveryRequired` zu treiben. Der Root hat den Nachtrag in DUR-04.2
(§11) und DUR-04.1 übernommen; die
[Simruntime](agent-tasks/SIM-03_5_QUARANTINE_ISOLATION_RUNTIME_TASK.md) ist
freigegeben. Ein dauerhaft `Uncertain` gebliebenes Pending bleibt ein eigenes
Ticket mit eigener Aktivierungsgrenze.
DUR-05a, NET-04.2f und DUR-05b folgen der Ticketfolge; die f-Auflagen stehen in
den [Rootentscheidungen](agent-tasks/NET-04_2d_NATIVE_DRIVER_ENDPOINT_ROOT_DECISIONS.md).

Der veröffentlichte Ausgangsstand `d3fca66c` mit DUR-03.2 und SIM-03.4 hat
vollständig erfolgreiche CI:
[Lauf38101579432](https://github.com/jstrausd/ferrumc/actions/runs/38101579432).
Auch die vollständige CI für den veröffentlichten Integrationsstand
`e772564c` mit GEN-05.7 ist erfolgreich:
[Lauf38103410294](https://github.com/jstrausd/ferrumc/actions/runs/38103410294).
Noch ungeprüfte Runtimeänderungen gehören nicht zu diesem CI-Nachweis.

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
Die gemeinsame Gesamt-CI für GEN-05.6 und NET-04.2a auf exakt `b87d5b61`
ist vollständig erfolgreich, einschließlich unabhängiger Client-/Inventar-/
Operator-/Neustartabnahmen:
[Lauf38099105046](https://github.com/jstrausd/ferrumc/actions/runs/38099105046).
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
