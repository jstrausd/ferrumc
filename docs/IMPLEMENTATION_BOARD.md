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
| STORAGE-ENTITY001.2 | `select_world_ticket`, Designreview Root | `server/crates/ferrumc-storage/**`, eigener .2-Brief | Konkrete explizite4→5-Migration durch Root freigegeben und in Umsetzung: voller Validator, unverändertes hashgebundenes Backup, immutable344-Byte-Checkpoint, bounded Prüfkopien und exakte Delta/Resume-Abnahme. Keine automatische Appmigration. |
| REG-WORLDGEN001 | `feature_catalog`, Review Root | `server/crates/ferrumc-registry/**`, eigenes Taskdoc | CLEAN geprüft und auf main integriert555ee2aa:60 Noiseparameter,35 Densityresources und7 Noisesettings. Root Debug/Release jeweils69,4 eigene negative/Bitproben, strictchecks und102 Originalressourcen normal/-O grün. Statische Daten; Worldcompiler/Terrainaktivierung bleiben offen. |
| GEN-05.1 | `item_transfer_primitives`, Review Root | `server/crates/ferrumc-world/**`, eigener GEN-05-Brief; Root besitzt Lockresolution | Unabhängig CLEAN und integriert `c4e3dc76`: alle35 Definitionen/7 Router,128 tatsächliche Original-JAR-Records, Debug/Release je177 und3 eigene Probes grün. Frischer Originalreexport normal/-O bytegleich, Freeze17/17 und strictchecks grün. [Review](agent-tasks/GEN-05_1_DENSITY_COMPILER_REVIEW.md); Seed-/Sample-/Chunkconsumer folgen. |
| GEN-05.2 | `item_transfer_primitives`, Vertragsreview Root | `server/crates/ferrumc-world/**`, gemeinsamer GEN-05-Brief | [Konkreter Seed-/Scalar-/Climatevertrag](agent-tasks/GEN-05_2_SEEDED_ROUTER_APPROVAL.md) vollständig geprüft und freigegeben; Runtimeumsetzung begonnen. Zwei originale Visitorviews, shared Workmeter, alle7 Settings/35 Definitionen gegen echtes JAR. Chunkwrapper/Terrainaktivierung bleiben .3 und weiteren Consumern zugeordnet. |
| SIM-03.3 / SRV-02.1 | `feature_catalog`, Vertragsreview Root | `server/crates/ferrumc-sim/**`, eigener Itembrief | [Konkreter ItemEntity-Vertrag](agent-tasks/SIM-03_3_ITEM_ENTITY_STATE.md) vollständig geprüft und freigegeben; Runtimeumsetzung begonnen. Sealed Slotpayload, owned Refusals/Removal, FCIE144-Byte-Header/262144-Byte-Body,16MiB Aggregatebudget, checked Capture/Restore und originale Zählerregeln. Noch keine App-/Drop-/Pickup-/Physikaktivierung. |

Aktuell arbeiten neben Root der Weltagent `item_transfer_primitives`,
der Simagent `feature_catalog` und der Storageagent
`select_world_ticket`. Der separate `gen05_1_codec_oracle`-Auftrag ist beendet.
Der Storageimplementer setzt nach .1-Integration die freigegebene explizite
.2-Migration um. Der bisherige Reviewer implementiert den vollständig geprüften
Item-Entity-Vertrag in Sim; der Weltagent implementiert nach CLEAN-Integration
des Compilers die freigegebene Seedberechnung. Die Vergabe überschreitet die
drei Childslots nicht; Root überwacht, prüft und integriert die Ergebnisse.

Root hat die integrierte Runtime `62a5b74d` in einem detached Checkout
geprüft: 2.304 Tests, 38 Doctests, strict Clippy/fmt und Generatorcheck grün,
getrennt von aktiven Agentänderungen. Gesamt-CI für `9025c687`, einschließlich
UUID-Storageproducer und Densitycompiler, ist erfolgreich:
[Lauf38080572897](https://github.com/jstrausd/ferrumc/actions/runs/38080572897).
Die drei laufenden Folgeconsumer sind darin noch nicht enthalten.
Ergebnisse der vierten Welle stehen im
[Prüfbericht](AGENT_WAVE_04_VALIDATION.md); eine ältere grüne CI ist kein
Nachweis für diese neue Runtime.

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
