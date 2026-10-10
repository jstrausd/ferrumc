# FerrumC: vollständiger Minecraft-Zielkatalog und Agenten-Backlog

Stand: 10.10.2026. Zielversion: **Minecraft Java Edition 1.21.8, Protokoll 772,
DataVersion 4440**. Entwicklungsbranch: `main`. Dieser Katalog beschreibt den
**angeforderten Zielumfang**, nicht einen bereits fertigen Server. FerrumC
bleibt Rust; unveränderte Bukkit-/Paper-Plugins werden über den Java-21-Sidecar
gemäß [ADR-0009](adr/0009-java-plugin-compatibility.md) angebunden.

Der [Survival-Lieferplan](SURVIVAL_DELIVERY_PLAN.md) bestimmt Lieferphasen,
Aufwandsspannen und kritischen Pfad. Der [v2-Plan](V2_IMPLEMENTATION_PLAN.md)
dokumentiert den bisherigen technischen Stand und seine alten Ticket-IDs.
Das [Implementierungsboard](IMPLEMENTATION_BOARD.md) führt die tatsächlich
zugewiesenen Agenten, reservierten Dateibereiche und den aktuellen Lieferstand.
Dieser neue Katalog ist der **kanonische Gesamtbacklog**: neue Requests werden
hier eingeordnet, fehlende Inhaltszeilen ergänzt und Ticketstatus mit konkreter
Abnahme aktualisiert. Er ersetzt keine dort festgehaltenen Prüfergebnisse.

## Was „vollständig und spielbar“ hier bedeutet

Ein neues versionsfestes Survival-Spiel muss von natürlicher Weltgenerierung
über Holz, Werkzeuge, Ressourcen, Crafting, Nahrung, Kampf, Farmen und Handel
bis Nether, End und Bossfortschritt funktionieren. Alle vorgesehenen
Block-, Item-, Entity- und Datapack-Inhalte werden durch Inhaltsmatrizen erfasst.
Multiplayer, Tod/Respawn, Chunk-Lebensdauer, Save/Restart und Backup gehören zu
jedem betroffenen Feature. Creative, Adventure und Spectator werden ebenfalls
geprüft. Die Serverlogik orientiert sich an den vollständigen Originalquellen,
einschließlich Paper-Patches und festgehaltener Konfiguration; siehe
[Paper-Referenz](PAPER_REFERENCE.md).

Eine Themenliste allein beweist keine Vollständigkeit. Für das Abschlussgate
muss jede versionsfeste Inhaltszeile ein Verhaltensticket, Verbraucherpfade,
Persistenzvertrag und unabhängige Abnahme besitzen. Neue Vanilla-Inhalte dürfen
nicht als unbemerkte Defaultblöcke oder ignorierte Pakete verschwinden. Eigene
Terrain-Generierung ist ein früher Lieferumfang; **exakte Vanilla-Seed-Parität**
ist ein eigenes, späteres Pflichtgate. Eine Java-Fassade ohne funktionierende
Rust-Spielmechanik erfüllt den API-Vertrag nicht.

Das Pluginziel umfasst die vollständige deklarierte Bukkit-/Paper-1.21.8-API
und konkrete versionsfeste CraftBukkit-/NMS-Verträge. Die externe Pluginmatrix
führt unveränderte JARs, Versionen, Funktionen und Kombinationen. Eine Garantie
für beliebige JAR-Dateien jeder Minecraft-Version lässt sich daraus nicht
ableiten; Forge/Fabric-Mods besitzen andere APIs. Versionswechsel erhalten
einen neuen Katalogpin und neue Abnahme.

## Status, Ticketvertrag und bisheriger Nachweis

**O** = offen; **T** = teilweise vorhanden, weitere Arbeit/Abnahme offen;
**A** = konkret zugewiesen und in Umsetzung; **V** = implementiert, noch in
Verifikation; **F** = definierter Ticketumfang abgenommen. Abhängigkeiten sind
Voraussetzungen für Aktivierung, keine Aussage über einen aktuellen Blocker.
Ein vorbereitetes Datenmodell darf vorher entwickelt werden, wenn es nicht
unvollständige Laufzeitpfade aktiviert. Ein **F**-Unterticket schließt seinen
größeren Lieferblock erst nach dessen kompletter Abnahme.

Jede Tabellenzeile besitzt genau **einen Produktions-Owner**. Die Kürzel stehen
für `ferrumc-<kürzel>`; `app` bedeutet `server/app`, `java` bedeutet das getrennte
Modul `server/java-bridge`, `tools` bedeutet Generator-/Harness-/Dokumentarbeit
ohne Runtime-Crate. Optional ist `ferrumc-testkit` die zweite Crate. Eine
cross-lane Funktion erhält aufeinanderfolgende Producer-, Consumer- und
Wiring-Tickets. Änderungen an generierten Dateien erfolgen ausschließlich
durch den zugehörigen Generator und seinen geprüften Output.

Für jedes zugewiesene Ticket hält der Brief fest: Basiscommit, Dateibesitz,
Quellenpin/Dateien/Symbole, Trigger und Fehlerpfade, Events/Cancellation,
Ownership, Queue-/Arbeits-/Bytegrenzen, Abweichungen und Abnahmebefehle. Die
Tabellen nennen die konkrete zusätzliche Abnahme; die gemeinsamen Gates am
Dokumentende gelten immer. Die Crate-Invarianten werden vor Änderungen gelesen.

Die vorhandene Grundlage umfasst Offline-Login, Streaming, einige
Inventar-/Placement-Aktionen, Checked-Komponentenpersistenz, native Plugins und
eine kleine Java-API-Teilmenge. Details stehen im v2-Plan und
[V2_VALIDATION.md](V2_VALIDATION.md). Die einzelnen Torch-/Fence-Korrekturen
stehen im [Placement-Prüfbericht](D0_PLACEMENT_VALIDATION.md), native-only
Operator-Persistenz im [Operator-Prüfbericht](D0_OPERATOR_VALIDATION.md).
Die Glas-Korrektur deaktiviert absichtlich verändernde Demo-Plugins per Default;
explizites Opt-in bleibt erhalten. Diese Nachweise schließen weder allgemeine
Blockphysik noch Survival oder breite Java-Kompatibilität ab.

## Versionsfeste Inhaltsmatrizen

Der deterministische [Bestandsbericht](fixtures/minecraft-1.21.8-feature-inventory.json)
wird durch [minecraft-feature-inventory.py](../scripts/minecraft-feature-inventory.py)
aus vorhandenen vendorten Daten erzeugt. Er enthält stabile IDs, Bestandszahlen,
SHA-256 und Provenienz, **keinen erfundenen Implementierungsstatus**. Block-/Item-
IDs beweisen nur vorhandene Daten, nicht deren Spielmechanik. Noch nicht
erfasste Kategorien sind ausdrücklich als solche geführt; CAT-Tickets schließen
sie vor der Gesamtfreigabe. Der Bericht benötigt weder Netzwerk noch einen
lokalen Paper-Checkout.

Der erste Bestandslauf erfasst **1.105 Blocktypen / 27.946 Blockstates,
1.416 Items, 65 Biome und 245 gerichtete Pakete**. Die IDs sind Datenbestand;
CAT-02.1 ergänzt **151 Entitytypen, 22 Recipe-Serializer, 1.407 Rezepte,
1.246 Loot Tables, 952 Worldgen-JSONs in 14 Registries, vier Dimensiontypen
und sechs World Presets** aus dem gepinnten offiziellen Vanilla-Server.
Die [Exportnotiz](agent-tasks/CAT-02_1_ORIGINAL_CONTENT.md) beschreibt
Rohbyte-/Vollmengenvergleich, Pins und bewusst unerfasste Inhalte. Tags,
NBT-Strukturtemplates, weitere Registries, Commands, API/Events und NMS bleiben
offen; Worldgen/Structures und Loot/Predicates sind deshalb nur teilweise erfasst.
Die Herkunft des Item-Extraktionsschritts ist nicht im Protokollmanifest
festgehalten; sein vorhandener Snapshot wird mit eigenem SHA-Pin erfasst,
die weitergehende Provenienz bleibt CAT-02.

```sh
python3 scripts/minecraft-feature-inventory.py --self-test --check
python3 scripts/minecraft-original-export.py --self-test --check
# Nach einem bewussten Daten-/Versionupdate neu erzeugen:
python3 scripts/minecraft-feature-inventory.py
```

`--check` schreibt keine Dateien und schlägt bei Hash-/Version-/Schema-/IDdrift
oder verändertem Bericht fehl. Generierung hängt nur von Repoquellen ab und
enthält keine Uhrzeit, Rechnerpfade oder aus dem Arbeitsstand erfundene Status.

| Inhaltsmatrix | Versionsfester Bestand / erforderliche Spalten | Verhaltenstickets / Abschlussbedingung |
| --- | --- | --- |
| Blocks und Blockstates | Vendorte `blocks.json`; jede Resource-ID, alle Property-Kombinationen/State-IDs; Platzierung, Shape/Support, Interaktion, Drops, Tick/Redstone/Fluid, BE-Typ und Persistenz | CAT-01/CAT-03, SIM-04–08, BLK; null unerfasste oder implizit ersetzte States. |
| Items und Komponenten | Vendorte `items.json`; jede Resource-ID, Defaults/Removals, Stacklimit, Use/Consume/Equip, Durability, Rezepte/Enchant/Loot/Placement | CAT-01/CAT-03, SRV/BLK; jede Itemfamilie hat Verhalten, inklusive nicht platzierbarer Items. |
| Entities und Metadaten | Export aus gepinntem echten Server; jede Type-ID, Metadaten/Attribute, Spawn/AI/Use/Combat, Save und Netzwerk | CAT-02/CAT-03, SIM/SRV/MOB; Spieler, Mobs und alle Nicht-Mob-Entities getrennt erfassen. |
| Biome, Dimensionen und Worldgen | Vendorte Biome-IDs; echter Export aller Dimension-/Noise-/Density-/Carver-/Feature-/Structure-/Template-Pool- und Preset-Keys | CAT-01/CAT-02, GEN; IDs plus Ressourcen und Seed-/Fortschrittsabnahme in allen drei Dimensionen. |
| Recipes und Recipe-Serializer | Alle versionsgepinnten Recipe-Ressourcen und Serializer; Zutaten/Tags/Komponenten, Ergebnis/Rest, Book, Craft-/Process-Owner | CAT-02/CAT-03, SRV-05–08/16–19, BLK-14–17; spezielle Rezepte zusätzlich zu normalen JSON-Typen. |
| Loot und Predicate/Function | Alle Loot-Table-/Predicate-/Item-Modifier-Keys und Typen; Kontext, RNG, Tools/Enchants/Chance, Quellen, Consumer | CAT-02/CAT-03, SRV-04/MOB/GEN; Block-, Entity-, Chest-, Fishing- und Geschenk-Loot vollständig. |
| Commands und Argumenttypen | Echter Default-Dispatcher-Export; alle Literale, Arguments, Redirects, Requirements, Execute-Forks, Suggestions und Komponenten | CAT-02/CAT-04, CMD/NET; jede Route mit Operator/Normalspieler/Konsole getestet. |
| Pakete und Konfigurationsregistries | Vendorte `protocol.json`; State/Richtung/Packetname/-ID; dynamische Registry-, Tags-, Feature-Flag- und Component-Serializer | CAT-01/CAT-04, NET; modelliert, geroutet und tatsächlich sichtbar getrennt erfassen. |
| Gamerules, Effects, Enchants, Attributes, DamageTypes | Echter Registry-/Datapack-Export; Default, Typ/Limit, Consumer, Timer, Save, Commands/API und Events | CAT-02/CAT-03, SRV/DAT/CMD/API; keine Registryzeile ohne Consumer. |
| Advancements, Statistics, Tags und Datapacks | Alle versionsgepinnten Keys/Trigger/Typen und Load-/Reload-Pfade; Kriterien, Rewards, Player-Save und UI | CAT-02/CAT-03, DAT; vollständige Fortschritts- und Datenabnahme. |
| Bukkit-/Paper-API, Events und NMS | Gepinntes API-Artefakt und Servermapping; Klasse/Methode/Overload/Event, Thread-, Rückgabe-/Mutation-/Cancellationvertrag und externe Fixture | CAT-05/CAT-06, API/NMS; unsupported Surface bleibt sichtbar, Abdeckung pro Symbol statt JAR-Scan. |

### Bestand und Abdeckung (CAT)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| CAT-01 | T | tools | – | Repo-Bestände für Blocks/States, Items, Biome und Pakete deterministisch erfassen; Pins/Hashes prüfen, `--check` und Selftests. Fehlende Kategorien sichtbar lassen. |
| CAT-02 | T | tools | CAT-01 | Gepinnte Vanilla-/Paper-Registry- und Datapack-Exporte ergänzen: Entities, Worldgen/Structures, Recipes, Loot, Commands, Gamerules, Effects/Enchants/Attributes, Advances/Stats und Serializer; IDs/Hashes reproduzierbar, keine lokale Referenz still voraussetzen. |
| CAT-03 | O | tools | CAT-02 | Inhalt-ID→Verhaltensticket/Consumer/Save/Fixture-Matrix mit Status führen; absichtliche familienweite Verträge nur mit allen Randfällen; unbekannte/unzugewiesene IDs lassen Gesamtgate scheitern. |
| CAT-04 | O | tools | CAT-01, CAT-02 | Protokoll-/Command-Route-Matrix mit Parser, Routing, Wirkung und unabhängigen Wirefällen; bloß vorhandene Spec-/Handlernamen gelten nicht als Umsetzung. |
| CAT-05 | O | tools | CAT-02 | API-Methoden/Event-Surface samt Overloads/Thread-Vertrag exportieren und Bridge-Test zuordnen; Reflection und Defaultmethoden berücksichtigen. |
| CAT-06 | O | tools | CAT-05 | Versionsfeste externe JAR-/NMS-/Pluginkombinationsmatrix samt benötigten Internals und bestanden/offen; Artefakthashes, Quelle und tatsächlich ausgeführte Funktionen. |
| CAT-07 | O | tools | CAT-03–06 | CI-Gate für unzugewiesene Inhalte, neue Unsupported-Zweige und Daten-/Versionsdrift; positive/negative Fixtures, kein Status aus Testzahl ableiten. |
| CAT-08 | O | tools | CAT-07 | Vollständigkeitsreview des gepinnten Originals mit Inhalts- und Consumerdiff; Ergänzungen werden neue Tickets, Abweichungen bleiben öffentlich prüfbar. |

## Protokoll, Anmeldung und Sessions (NET; D0/D10, alte N/A/V11)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| NET-01 | T | proto-gen | CAT-01 | Fehlenden versionsfesten Packet-/Component-Surface erzeugen, einschließlich signed commands, Use/Interact/Respawn/Recipe/Chunk-Batch; unabhängige IDs und malformed Grenzfälle. |
| NET-02 | O | net | NET-01 | RSA/AES-CFB8-State und Umschaltpunkt; Fragmentierung, Verify-Token, fehlerhafte Schlüssel und bounded Crypto-Arbeit (A02). |
| NET-03 | O | app | NET-02 | Mojang-Session/Auth/Profile mit absoluten Fristen; echte Online-UUID/Skin übernehmen, Fehler/Replay/Disconnect und Auth-Ausfall prüfen (A03/A04). |
| NET-04 | O | net | – | Bounded IP-Versuchsrate und vollständige Byte-/Dekompressionsbudgets; Expiry, Burst, langsame/floodende Clients und faire Kapazität (N04). |
| NET-05 | O | app | NET-04 | Produktions-Ingress-/Bytebudget-Wiring und mutierende Handlerfristen; Lost-Reply/Timeout verliert keine Transaktion (N05/N06). |
| NET-06 | T | app | bestehende Teleport-Pakete | NET-06.1 (N03) liefert verbindungseigene pending Teleport-ID/Bestätigung, stale IDs, Replay, superseding `/tp` und einen endlichen Fence für bereits eingereihte Syncs. NET-06.2 bleibt offen: globale Bewegungszulassung für externe Teleports, die erst nach dem Queue-Snapshot eingereiht werden. Papers deaktivierter >20-Tick-Resend ist keine neue Ackdeadline; Connection-Liveness/Stop bleiben begrenzt/fair. |
| NET-07 | O | session | SIM-03, SIM-10 | Vollständige Player-/Entity-/Use-/Interact-/Inventory-/Respawn-Inputs und Stateoutputs routen; GameMode/Reichweite nur serverseitig, nie Socket-Weltmutation. |
| NET-08 | O | app | NET-01, NET-03 | Signed Chat/Sessions, Secure Profile, Acknowledgements und signed command arguments; Login-/Key-Lifetime/Replay/Filter/Systemmessage-Fälle mit Originalclient. |
| NET-09 | T | app | CAT-02 | Configuration mit vollständigen dynamischen Registries/Tags/Featureflags und Reconfiguration; falsche Reihenfolge/Registry-IDs abweisen, Client bestätigt alle Dimensionen. |
| NET-10 | O | session | NET-09 | Chunk-Batch-Acks, Sicht-/Simdistanz, Sendebudgets, Bundle/Entitytracking und langsame Clients; Unload/Rejoin und Backpressure ohne veraltete Entities. |
| NET-11 | O | app | NET-03 | Velocity-Forwarding mit MAC/Secret und getrenntem Identity-Migrationsvertrag; direkter Bypass/gefälschte UUID/IP scheitert (A05). |
| NET-12 | O | app | NET-09 | Resourcepack-Stack, URL/Hash, Required/Declined/Failed/Reload, Custom Payload/Plugin Messaging und Transfer; Clientzustände und bounded Bytes korrekt. |
| NET-13 | T | app | NET-03, NET-09 | Serverlist/MOTD/Favicon/Playerlist/Skins, Versionsablehnung, Login/Play-Disconnect und Restart-Liveness; vorhandene N01/N02-Fälle beibehalten. |

## Dauerhafte Welt-, Spieler- und Entity-Autorität (DUR; D1/D10, S/I/W)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| DUR-01 | T | world | – | Versionierte Chunk-/Dimension-/Block-Entity-/Entity-Datensätze und Generatorbasis/Overlay; unbekannte Versionen verhindern stille Datenersetzung. |
| DUR-02 | T | items | – | Autoritativer Inventory-/Itemkomponentenvertrag mit Stacklimits/Removals/Opaquebytes; vollständige Player-/Cursor-/Containerdarstellung, malformed Grenzen (I01/S02). |
| DUR-03 | O | storage | DUR-01, DUR-02 | Atomare Inventory↔World-/Container-/Entity-Ownership mit dauerhaften Transfer-IDs und Recovery; Cutpoints, Wiederholung, Konflikt und Lost-Reply exakt einmal (S03/V01). |
| DUR-04 | O | sim | DUR-03, SIM-03 | Sim-Consumer für sichere Transfers/Reservations und Entityidentität; zwei Pickups, Chunk-Unload, Disconnect/Retry weder Verlust noch Dupe. |
| DUR-05 | O | app | DUR-03, DUR-04 | Player-/Container-/Entity-Transfer in produktivem Driver aktivieren; Frameabbruch und Restart am Transfer-Cutpoint, alte Verbraucher erst danach entfernen. |
| DUR-06 | O | app | DUR-02 | Periodische Player-/World-Checkpoints und Flush-Barrieren; maximaler bestätigter Verlustzeitraum dokumentiert, Savefehler sichtbar (S01). |
| DUR-07 | O | storage | DUR-01 | Weltzeit, Wetter, Difficulty, Gamerules, Spawn, Border, Datapack/Generatorpin und globale gespeicherte Daten; Schema-/Migration-/Corruptiontests. |
| DUR-08 | O | sim | DUR-07 | Weltzustands-Load/Update/Save und Tickfortschritt; Neustart, mehrere Dimensionen und abgeleitete Clientsichten stimmen (W10). |
| DUR-09 | T | anvil | DUR-01, DUR-02 | Verlustfreier Importvertrag für Biome, Licht, volle BEs/Entities/Komponenten, unbekannte Inhalte und Reimport; unabhängige NBT-Fixtures (W01–03). |
| DUR-10 | O | anvil | DUR-09 | Vollständiger Inhaltsmatrix-Export, sichere `.mcc`/LZ4-/Pfad-/Dekompressionsgrenzen; Vanilla/Paper lädt Export ohne stille Änderungen (W04). |
| DUR-11 | O | storage | DUR-03, DUR-07 | Snapshot/Backup/Restore, Schema-Upgrade/Recovery, crash-/torn-write-/Disk-full-Pfade; wiederholbare Restore-Abnahme auf leerer Instanz. |
| DUR-12 | O | app | DUR-06, DUR-11 | Save-All/Stop/Crashreport/Recovery-Wiring, Plugin-Save und bind-/Startup-Rollback; keine erfolgreiche Flushmeldung bei Fehler (O04/H05). |

## Tick, Shapes, Bewegung, Licht und Entity-Grundlage (SIM; D1/D2)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| SIM-01 | T | sim | – | Autoritativer Tick-/Systemorder-Vertrag und dedicated Executor; Input/Storage/Plugin/System/Output/Save-Reihenfolge deterministisch, Lag ohne Catch-up. |
| SIM-02 | O | sim | SIM-01, DUR-01 | Persistente scheduled Block-/Fluid-Ticks, deduplizierte Neighbor-/Shapeupdates und Randomticks; Priority/Order/Restart/Budgetfortsetzung (W09). |
| SIM-03 | O | sim | DUR-01, SIM-01 | Allgemeine Entitytypen/-IDs, Komponenten, Spawn/Remove, Chunk-/Spatialindex, Activation/Tracking/Lifecycle; UUID-/ID-/Unload-/Restartgrenzen. |
| SIM-04 | O | registry | CAT-02 | Versionsfeste Shape-/Support-/Occlusion-/Collision-Fakten pro State statt grobem Vollwürfel; Generator/Goldens prüfen Partialshapes und Richtung. |
| SIM-05 | O | math | SIM-04 | Swept collision, Raytrace, Face-/Shapeoperationen, Step-/Pushauflösung; Randkontakt, kleine Hindernisse, negative Koordinaten und keine Tunnelingfälle. |
| SIM-06 | O | placement | SIM-04 | Statebasierte Support-/Attachment-/Replaceable-/CanSurvive- und Verbindungsregeln; alle Familien/Rotationen/Waterlogging, bisherige Torch/Fence-Fixes erhalten. |
| SIM-07 | O | world | SIM-04 | Korrekte Heightmaps und Block-/Skylightpropagation samt Sectiongrenzen; Light-Filter/Emission, Pflanzen/Fluids und Import (W05/W06). |
| SIM-08 | O | sim | SIM-02, SIM-06, SIM-07 | Allgemeine Neighbor-/Support-/Lighting-Consumers für sämtliche Mutationseinstiege; Place/Break/Command/Plugin/Explosion und Chunkgrenzen gleich. |
| SIM-09 | O | sim | SIM-03, SIM-05 | Autoritative Entitybewegung/Collision/Gravity, fallende Blöcke/Items und Transfergrenzen; keine globale Entitysuche pro Aktion. |
| SIM-10 | O | sim | SIM-09, NET-06 | Playerphysics: Walk/Sprint/Sneak/Jump/Step, Swim/Climb/Crawl, Flight/Elytra, Knockback/Vehicles; GameModes/Teleport/Abilities/Fallstatus konsistent. |
| SIM-11 | O | sim | SIM-01, SIM-03 | Tickende/geladene/gespeicherte Chunks, Tickets/Forcedloads/Spawnregion, Activation/Despawn; unload vs inaktiv vs offline explizit verschieden. |
| SIM-12 | O | sim | SIM-11, DUR-04 | Mehr-Shard-Ownership und tickgrenzige Entities-/Ticket-/Updateübergabe; Teleports/Edits/Explosions-/Fluidgrenzen deterministisch (M5). |
| SIM-13 | O | session | SIM-07, SIM-09 | Light-/Heightmap-/Block-/Entity-Metadaten und Attribute aus demselben autoritativen Zustand senden; zwei Betrachter/Chunkrejoin. |
| SIM-14 | O | app | SIM-01–13, NET-07 | Getrennte bereits abgenommene Producer-/Consumerpfade produktiv verdrahten; Referenzszenario und Shutdown über mehreren Shards. Kein Sammel-Runtime-Ticket über Crates. |
| SIM-15 | O | sim | SIM-02, SIM-03 | Welt-/Player-/Entity-Tick- und Pluginarbeitsbudgets mit fairem Fortschritt; überlaufende Queues führen definierte Fortsetzung/Fehler statt verlorener Mechanik. |

## Weltgenerierung, Dimensionen und Seed-Parität (GEN; D2/D7, W07/W08/V10)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| GEN-01 | T | world | bestehender Flat-Generator | GEN-01.1 (WG-001) liefert numerischen Seed, geschlossene/versionierte Generatoridentität, strikten Codec und reinen Flat-v1-ChunkGenerator. Persistenz, produktive Seedkonfiguration und neue Generatoren bleiben Folgetickets; Ladefolge, paralleler Abruf, negative Koordinaten und Restart müssen auch dort gleich bleiben. Flat bleibt explizites Preset (W07). |
| GEN-02 | O | world | GEN-01 | Versionierter spielbarer eigener Overworld-Generator mit Höhe/Land/Wasser/Bedrock; Chunkgrenzen und erreichbare Ressourcen; noch keine Vanilla-Seed-Zusage. |
| GEN-03 | O | registry | CAT-02 | Versionierte Worldgen-/Biome-/Dimension-/Feature-Daten und Tags vollständig generieren; keine fehlenden Ressourcen hinter Defaults verstecken. |
| GEN-04 | T | world | GEN-01, GEN-03 | GEN-04.1 liefert Legacy48/Xoroshiro128++, Positionalfactories, Improved-/Perlinnoise mit 16.556 bitgenauen Original-JAR-Records. Normal-/Blended-/Simplexnoise, Gaussian, WorldgenRandom-Wrapper und Density-Arithemetik bleiben offen; vollständige Seedparität ist nicht abgenommen. |
| GEN-05 | O | world | GEN-04 | Overworld Noise-Terrain, Klima/Biome, Surface, Aquifers und Caves/Carvers; Section-/Chunkgrenzen und Höhenbiome mit Referenzhashes. |
| GEN-06 | O | world | GEN-05 | Feature-Placements: Erze, Vegetation/Bäume, Seen, Geoden, Fossilien, disk-/patch-/multichunk Features; stabile Featureorder und Survival-Ressourcen. |
| GEN-07 | O | world | GEN-05, CAT-02 | Structure-Start/Locate/Reference/Jigsaw/Template-Pools/Lootmarkers; villages, strongholds, monuments, mansions, ruins, trails, trials und komplette Structurematrix. |
| GEN-08 | O | world | GEN-04, GEN-06, GEN-07 | Nether Terrain/Biome/Surface/Caves/Features/Strukturen inkl. Fortress/Bastion; eigener Dimensionkey/-höhe, Seed-/Ressourcenabnahme. |
| GEN-09 | O | world | GEN-04, GEN-06, GEN-07 | End-Inseln/Biome/Features/End-Cities/Ship/Gateways und Spawnplattform; referenzgeprüfte Chunks und Bossarena. |
| GEN-10 | O | sim | GEN-08, GEN-09, SIM-11 | Dimensionwechsel, Portalbau/Erkennung/Scaling/Search, Netherportal/Endportal/Gateway/Respawnanker/Betten; Fahrzeug-/Entitytransfer und Persistenz. |
| GEN-11 | T | storage | GEN-01, DUR-01 | Generatorpin, Chunkstatus/-Version und Worldgen-Savedata dauerhaft speichern; Overlay nie unter neuer Basis still umdeuten. |
| GEN-12 | O | app | GEN-01, GEN-11 | Seed/Preset/Dimensionkonfiguration und bounded/off-thread Generation aktivieren; neue vs alte Welt, interrupted Generate/Save/Load und Configabweichung. |
| GEN-13 | O | world | GEN-03–09 | Flat/Large Biomes/Amplified/Single Biome/Custom-Presets und Datapackdimensionen; Konfigurationsserialize/Seed/Featureflags versionieren. |
| GEN-14 | O | world | GEN-04–09 | Exakte Vanilla-Seed-Parität in sämtlichen Generationstufen; Goldens inkl. Höhe, Biome, Blocks/States, Structures, Loot/Feature-RNG und Grenzkoordinaten. |
| GEN-15 | O | tools | GEN-14, GEN-12 | Generieren in anderer Lade-/Threadreihenfolge und Restart gegen gepinnten echten Server vergleichen; festgehaltene Hashes/Seeds, keine aus FerrumC erzeugte Oracle. |
| GEN-16 | O | sim | GEN-10, DUR-07 | Spawnwahl, Bonuschest, Sleep/Difficulty/Dimensionweather/Worldborder, Structure-Locate und Exploration; komplette Fortschrittsstrecke spielbar. |
| GEN-17 | O | tools | GEN-15, DUR-09, DUR-10 | Vanilla/Paper-Welt importieren, weitergenerieren, exportieren und neu laden; Grenzen von Alt-/Neuchunks und unbekannte Generatorversionen überprüft. |

## Survival, Inventare, Items, Kampf und Fortschritt (SRV; D1/D3/D7)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| SRV-01 | T | items | DUR-02 | Vollständige Stack-/Slot-/Cursor-/Offhand-/Equipmentoperationen, Komponentenvergleich und Conservation; alle erlaubten Klickprimitiven serverautoritativ (I03). |
| SRV-02 | O | sim | DUR-04, SIM-03, SIM-09 | Item-Drop/Pickup/Merge/Despawn, Delay/Owner/Throwphysics/Animation; Komponenten, Teilaufnahme, volle Tasche, zwei Spieler und inaktive Entities (V01). |
| SRV-03 | O | sim | SRV-02, SIM-10 | Survival-Miningprogress, Werkzeuge/Speed/Attributes/Effects/Wasser/Luft/Adventurechecks, Abbruch/Reach; kein mehrfacher Abschluss (V02). |
| SRV-04 | O | sim | SRV-03, CAT-02 | Vollständiges block-/entity-/fishing Loot, Tool-/Enchantkontext, RNG, Silk/Fortune/Explosion, Erfahrung und Durability; Conservation/Eventreihenfolge. |
| SRV-05 | O | items | CAT-02, SRV-01 | Recipe-Datenmodell/Matcher: shaped/shapeless/special, Tags/Komponenten, Restitems und Zutatenmultiplikation; unabhängige Rezepte/Fehlergrenzen (V03). |
| SRV-06 | O | sim | SRV-05, DUR-04 | Crafting 2×2/3×3 und Repeated-/Shift-Craft transaktional, Rezeptunlock/Book; Output read/write, Close/Fullbag/Changedrecipes/Reentrancy. |
| SRV-07 | O | app | SRV-01, SRV-06 | Alle Inventory-Clickmodes und Resync: shift/number/offhand/drag/double/drop/creative/spectator, Craftscreen/Book; Paper-State-ID-Vertrag prüfen statt aktuelle Abweisung als Parität behaupten. |
| SRV-08 | O | app | DUR-04 | Live-Container-Viewerupdates und Konfliktversionen; zwei Spieler sehen Änderungen ohne Extra-Klick, Containerwechsel/Close/Disconnect (I04). |
| SRV-09 | O | sim | SIM-03, SIM-10 | Living health/Attributes/DamageTypes/Invulnerability, Rüstung/Toughness/Enchants/Resist/Absorption, Shield/Totem; Fall/Feuer/Lava/Drown/Freeze/Suffocation/Void/Starvation. |
| SRV-10 | O | sim | SRV-09 | Melee/PvP mit Cooldown/Crit/Sweep/Knockback/Sprint/Reach/Team/Difficulty; zwei Clients, Immunitätsfenster und Events (V04). |
| SRV-11 | O | sim | SRV-09, DUR-04 | Tod, Deathmessage/Combatlog, Item-/XPdrops, keepInventory, Hardcore, Respawn/Bed/Anchor; Disconnect und Restart am Toddupe-Cutpoint. |
| SRV-12 | O | sim | SRV-09, SRV-11 | Hunger/Saturation/Exhaustion, Food/Consume/Use-Duration/Restitems und Regeneration; sprint/mine/attack/swim/idle, Hunger-Difficulty und Save (V05). |
| SRV-13 | O | sim | SRV-09, CAT-02 | Sämtliche Effects/Attribute-Modifiers mit Dauer/Amplifier/Stackregeln, Instant/Wirkungsperioden, Curatives und death/save; alle Registryzeilen. |
| SRV-14 | O | sim | SRV-11 | XP-Orbs/Pickup/Merge, Level/Progress, Enchantkosten, Mending und Death-XP; Mengen/Delay/Restart unabhängig prüfen. |
| SRV-15 | O | sim | SIM-09, SRV-09 | Bögen/Crossbow/Arrows/Spectral/Tipped/Trident/Snowball/Egg/Pearl/Firework/Fishing und restliche Projektile; charge/ammo/physics/hit/shield/enchant/owner. |
| SRV-16 | O | sim | SRV-06, SRV-13, SRV-14 | Enchanttable/Lapis/Bookshelves, alle Enchant-Trigger/Conflicts/Curses, Anvil/Repair/Rename/Grindstone; Seed/Costs/TooExpensive und Conservation. |
| SRV-17 | O | sim | SRV-06, SRV-13 | Brewing/Fuel/Ingredients/Containerarten/Potiontransforms/Splash/Lingering; Timer, Recipe/Reste, Effekte und Restart. |
| SRV-18 | O | sim | SRV-06 | Stonecutter/Smithing/Loom/Cartography und alle übrigen Rezept-/Result-Screens; Templates, Bannerpattern, Map-Komponenten, Restitems und Klickmatrix. |
| SRV-19 | O | sim | SRV-04, SRV-15 | Fishing-Loot/Openwater/Timing/Lure/Luck/Rod-Durability sowie sämtliche Use-/Consume-/Equip-Itemfamilien aus Matrix. |
| SRV-20 | O | sim | SIM-10, SRV-09 | Elytra/Gliding/Firework, Riptide/Swimming, Vehicles/Passengers/Mountcontrol und fallende/teleportierende Spieler; client-independent Status. |
| SRV-21 | O | sim | SRV-01, SRV-09 | Armor/Offhand/Equippable/Binding/Unbreaking/Reparatur, Bed/Sleep/Spawn, Item-Cooldowns und Use-Interruptions; alle Inputpfade gleicher Zustand. |
| SRV-22 | O | session | SRV-02–21 | Sichtbare Health/Hunger/XP/Abilities/Use/Combat/Respawn/Recipe/Equipment-/Entitystatus-Ausgaben; Observer/rejoin, kein selbstbehaupteter Treffer. |
| SRV-23 | O | app | SRV-07, SRV-22 | Survival-/Adventure-/Creative-/Spectator-Spielpfade wiring, gespeicherte Playerstate-Migration und sichere Window/Death/Respawn-Verbindungen. |
| SRV-24 | O | tools | GEN-12, SRV-23 | Echte Mehrspieler-Survivalstrecke ohne Creativehilfen: Ressourcen→Werkzeuge→Craft/Food→Fight→Death/Respawn→Restart; Itemmengen/Komponenten und spätere Nether/End-Erweiterung. |

## Alle Mobs, Nicht-Mob-Entities, KI und Population (MOB; D5/D7)

Die Familienzeilen sind keine Behauptung einer fertigen Artenliste: CAT-03
ordnet **jede** gepinnte Entity-ID einschließlich neuer 1.21.8-Inhalte zu.
Jede Art erhält innerhalb ihrer Zeile eigene Verhalten-/Save-/Eventfixtures.

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| MOB-01 | O | sim | SIM-03, SIM-09, GEN-05 | Spawnplacement/Light/Biome/Difficulty/Chunkpopulation/Caps/Persistence/Despawn, Spawner/Summon/SpawnEggs; Dimensionen/Spielerabstand und Workbudget. |
| MOB-02 | O | sim | SIM-05, SIM-09 | Navigation/Pathfinding/Doors/Wasser/Lava/Climb/Fly/Swim, Ziele/Senses/Memory/Brain; bounded Search, Hindernisse und Chunkgrenzen. |
| MOB-03 | O | sim | MOB-01, MOB-02, SRV-09 | Basis-LivingAI: Wander/Look/Flee/Target/Attack, Damage/Loot/Equipment/Attributes; Activation führt belegte inaktive Arbeit aus. |
| MOB-04 | O | sim | MOB-03, SRV-12 | Nutz-/passive Tiere: sheep/cow/pig/chicken/rabbit/goat/sniffer/turtle/armadillo und alle Matrixarten; Tempt/Breed/Age/Food/Drops/Farms. |
| MOB-05 | O | sim | MOB-03, SRV-20 | Tame-/Ride-Arten: wolves/cats/horse/donkey/mule/llama/camel/parrot; Owner/Trust/Chest/Saddle/Breed/Variant/Mount und Restart. |
| MOB-06 | O | sim | MOB-03, SRV-15 | Overworldhostiles: zombies/skeletons/creepers/spiders/endermen/slimes/witches/phantoms/silverfish/endermites und übrige Arten; Spezialattacken/Transform/Sun/Explosion. |
| MOB-07 | O | sim | MOB-03, SRV-13, GEN-08 | Netherhostiles/-neutral: piglins/brutes/zombified/hoglins/zoglins/ghasts/blazes/magmacubes/striders; Barter/Anger/Transform/Fly/Ride und Netherfarms. |
| MOB-08 | O | sim | MOB-03, GEN-09 | Endermen/Shulker und übrige End-Mechaniken: Teleport/Bullet/Levitation/Attachment/Split; Entity-/Block-/Chunkgrenzen. |
| MOB-09 | O | sim | MOB-03 | Aquatisch/fliegend: fish/squid/glowsquid/dolphins/axolotls/frogs/tadpoles/bats/bees/allays und alle Matrixarten; buckets/hive/trust/metamorphosis/delivery. |
| MOB-10 | O | sim | MOB-02, MOB-03, SRV-08 | Villager/WanderingTrader: Brain/Schedule/POI/Home/Profession/Trades/Restock/Prices/Reputation/Breed/Gossip/Zombify/Cure; handelbare Stacks konserviert. |
| MOB-11 | O | sim | MOB-03, MOB-10 | Pillager/Vindicator/Evoker/Illusioner/Ravager/Vex, Patrols/Raids/Omen/Waves/Village/Hero; Save und alle Events/Farmfälle. |
| MOB-12 | O | sim | MOB-03, SRV-13, GEN-07 | Warden/Sculk/Vibrations/Darkness und Trial-Spawner/Breeze/Bogged/Vault/Ominous-Trials; scoped Spezialticks/Loot/Playertracking und Restart. |
| MOB-13 | O | sim | MOB-03, SIM-08 | Golems/Snowgolem/Creaking/Heart und weitere spezielle Spawn-/Linkedblock-/AI-Arten der Zielversion; Bauform/Tag/Nacht/Link-/Despawnsemantik. |
| MOB-14 | O | sim | MOB-03, GEN-09, SRV-15 | Enderdragon: Parts/Phases/Crystals/Death/XP/Portal/Respawn/Gateway und Wither: Summon/Invulnerability/Skulls/Explosion/Bossbar/Loot. |
| MOB-15 | O | sim | SIM-03, SRV-15, SRV-20 | Nicht-Mob-Entitymatrix: boats/minecarts/fallingblocks/TNT/XP/items/paintings/frames/armorstands/displays/interactions/lightning/areaeffects/markers; Life/Use/Save/Metadata. |
| MOB-16 | O | session | MOB-04–15 | Sämtliche Metadaten/Attributes/Equipment/Sounds/Status/Passengers und Entityvisibility; Join/Unload/Dimensionwechsel mit zwei echten Clients. |
| MOB-17 | O | tools | MOB-16, DUR-05 | Artenweise Referenz-/Farm-/Combat-/Breed-/Trade-/Boss-/Saveabnahme, voller Entity-ID-Consumerdiff; aktive Populationen und Pathkosten messen. |

## Blockmechanik, Weltleben, Redstone und Transport (BLK; D0/D4/D6/D7)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| BLK-01 | T | placement | SIM-04, SIM-06 | Alle Placement-/Rotate-/Replace-/Shapefamilien: torch/fence/pane/wall/stairs/slab/door/trapdoor/bed/rail/sign/hanging/sign/banners/heads und Waterlogging; keine groben Supportdefaults. |
| BLK-02 | O | sim | SIM-02, SIM-08 | Support-/Multiblock-/Neighbor-/Gravityphysik, sand/gravel/anvil/concrete/dripstone/scaffolding; entfernte Stützen, aktive/vorher entladene Nachbarn. |
| BLK-03 | O | sim | SIM-02, SIM-08 | Wasser-/Lava-Fluidticks, Quellen/Gefälle/Level/Spread/Slope/Reaction und Chunkgrenzen; remove/restart/scheduled-order exakt nachvollziehbar. |
| BLK-04 | O | sim | BLK-03, DUR-04 | Buckets/Empty/Fill/Waterlogging/Cauldron/Dispenser/Fish/EntityinFluid; Nether-Wasser, Lava-/Cobble-/Obsidian-Reaktionen und Conservation. |
| BLK-05 | O | sim | SIM-02, SRV-09 | Fire/Soulfire/Lava-Ignition, Spread/Fuel/Burn/Rain/Lightning/Extinguish und gamerules; sichtbarer/physischer Schaden und Restart. |
| BLK-06 | O | sim | SIM-02, SRV-09 | Weather/Time/Rain/Thunder/Snow/Ice/Melt/Freeze/Sleep/Lightning und Difficulty; Biome-/Dimensionbedingungen, Dauer-Save und Pakete. |
| BLK-07 | O | sim | SIM-02, GEN-05 | Farmland/Moisture/Crops/Seeds/BoneMeal/Stem/Fruit/Saplings/Trees/Mushroom/Bamboo/Sugarcane/Cactus/Kelp und vollständige Growmatrix; Randomticks/Light/Soil. |
| BLK-08 | O | sim | BLK-07 | Grass/Mycelium/Moss/Vines/Leaves/Decay/Flowers/Coral/Dripleaf/Dripstone/Oxidation/Wax/Weathering/Sculk und alle Transformfamilien; Loot/Updateorder. |
| BLK-09 | O | sim | SIM-02, SIM-08 | Redstone-States/Verbindung und autoritative Power/Weak/Strong/Quasi-Connectivity-Referenz; Paper-Redstoneimplementierung/Config pinnen, Updateorder nicht nur Darstellung. |
| BLK-10 | O | sim | BLK-09 | Buttons/Levers/Pressure/Tripwire/Daylightsensors/Detectors/Redstonetorch/Repeaters/Comparators/Observers/Targets/Calibratedsculk; Pulse/Delay/Lock/Input. |
| BLK-11 | O | sim | BLK-09, BLK-10, DUR-04 | Pistons/Sticky/Movingblocks/Slime/Honey/Pushlimits/Blockentities; Extend/retract/order/quasi/connectivity und item-/entity-conserving Bewegung. |
| BLK-12 | O | sim | BLK-09, SRV-15 | Dispenser/Dropper/NoteBlocks/Lamps/CopperBulb/Crafter/Doors/Rails und alle redstone-reactive Consumers; aktive Schaltungsfixtures plus Restart. |
| BLK-13 | O | sim | SIM-09, SRV-09, SRV-04 | TNT/Ignition/Fuse/Physics/Explosion/Raycast/Blastresistance/Knockback/Fire/Drops/Chains sowie Creeper/Bed/Anchor/Wither explosions; Saved TNT. |
| BLK-14 | T | sim | DUR-04, SRV-08 | Chest/DoubleChest/Trapped/Ender/Shulker/Barrel/Bundle/Lock/Loot/Viewer/Openphysics; linkedcontainers/Chunkgrenzen/Save und sämtliche Klickmodi. |
| BLK-15 | O | sim | SIM-02, SRV-05, DUR-04 | Furnace/Blast/Smoker/Campfire mit fuel/smelt/timers/XP/recipes/rain/restart; Inputs/Output/Reste konservieren (V07). |
| BLK-16 | O | sim | BLK-15, DUR-04 | Hopper/MinecartHopper/AutoCrafter/Dropper-Transfers, Cooldown/Sideaccess/Redstone/Fullcontainer/Lootgen; Konflikt/Event/Crashmatrix. |
| BLK-17 | O | sim | SRV-17, SRV-18 | Brewing-/Enchant-/Anvil-/Smithing-/Stonecut-/Loom-/Cartography-Blockentities und Screens, Bookshelves/Lectern/Composter; echte Item-/Screenpfade. |
| BLK-18 | O | sim | SIM-09, SRV-20 | Rail-shape/Powered/Detector/Activator, alle Minecarts und Boats/Chestboats; Passenger/collision/speed/portals/chunks/save/commands. |
| BLK-19 | T | sim | SIM-08, DUR-01 | Schilder/Hängeschilder Front/Back/Edit/Wax/Glow/Dye, Banners/Heads/Pots/Frames/Paintings/Books/Lecterns; Komponenten und GUI-/Saveabnahme. |
| BLK-20 | O | sim | SRV-13, MOB-01, DUR-04 | Beacon/Conduit/RespawnAnchor/Beds/Bell/Jukebox/Flowerpot/Decoratedpot/Chiseledbookshelf/Brush/Suspiciousblocks/Archaeology; alle Spezialinteraktionen. |
| BLK-21 | O | sim | MOB-12, DUR-01 | Spawner/TrialSpawner/Vault/Endgateway/Portal/SculkSensor/Catalyst/Shrieker/Hives und verbleibende ticker-BEs; Timer, Loot, Offline/Unload getrennt. |
| BLK-22 | O | sim | SIM-02, CMD-09 | Commandblocks/Repeating/Chain, Functions, Structureblocks/Jigsaws/Test/Debug-Blocks der Zielversion; Rechte, begrenzte Forks und versionierte Daten. |
| BLK-23 | O | sim | SIM-10, SRV-09 | Welt-/Blockinteraktionen mit Entities: honey/slime/powdersnow/soulsoil/magma/campfires/cobweb/berrybush/cactus/bubblecolumns/portals; Hitbox/Effects/Move. |
| BLK-24 | O | session | BLK-03–23 | Blockactions/Events/BE-Daten/Breakanimations/Sound/Particles/Lighting und Observerstates korrekt übertragen; tatsächliche Clients plus Restart. |
| BLK-25 | O | app | BLK-24 | Abgenommene Blockuse-/Placement-/Redstone-/Containerpfade schrittweise produktiv aktivieren; unimplementierte Interaktionen bleiben konkrete Fehler. |
| BLK-26 | O | tools | BLK-25, CAT-03 | Jede Block-/State-ID nach Behavior/Support/Use/Update/Loot/Save abnehmen, explizite Inert-Familien eingeschlossen; Matrix darf keine undefinierten Rows haben. |
| BLK-27 | O | tools | BLK-09–13 | Versionsfeste Schaltungs-/Piston-/TNT-Suite mit Originalserver; tickgleiche Observables, Chunkgrenzen/Restart und zugelassene Paper-Konfiguration. |
| BLK-28 | O | tools | BLK-03–08, BLK-14–21 | Farms/Fluid-/Feuer-/Crop-/Hopper-/Containerlanglauf mit gleichem Paperumfang; tatsächliche Mechanik/Ertrag/Progress plus Speicher-/Tickkosten. |

## Datapacks, Datenfunktionen, Karten und Spielerfortschritt (DAT; D7)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| DAT-01 | O | registry | CAT-02 | Vollständige dynamische Registries/Tags/Featureflags/Registry-References und Data-Driven-Serializer; Unknown/cycle/oversized Daten sicher behandeln. |
| DAT-02 | O | app | DAT-01, NET-09 | Packdiscover/order/enable/disable/required/version/reload und atomarer erfolgreicher Snapshot; fehlerhafter Reload erhält alten aktiven Zustand. |
| DAT-03 | O | sim | DAT-01, SRV-05, SRV-04 | Recipe/Loot/Predicate/Modifier-Reload und Consumerinvalidierung; stale Screens/Tags/Enchants/Worldgen bleiben versions-/transaktionsklar. |
| DAT-04 | O | sim | CAT-02, SRV-23 | Sämtliche Advancement-Trigger/Criteria/Requirements/Rewards/Progress und Recipeunlock; Tod/Disconnect/Restart/Revoke und Ereignisorder. |
| DAT-05 | O | storage | DAT-04 | Stats/Advances/Recipebook/Exploration-/Map-/Raid-/POI-/Boss-Savedata persistieren; Player-/Worldscopes, Migration und Crashlimits. |
| DAT-06 | T | sim | CAT-02 | Scoreboard Objectives/Criteria/Scores/Teams/Display/Death-/Kill-/Trigger-/Statintegration und Bossbars; vorhandene Displayfeatures erweitern, Defaults/Save prüfen. |
| DAT-07 | O | sim | GEN-12, DAT-05 | Map-Items/Create/Scale/Lock/Clone/Exploration/Color/Icons/Banner/Frames/Playerupdates und Cartography; Pixel-/ID-/Dimension-/Savevertrag. |
| DAT-08 | O | command | CAT-02 | Text-/Adventure-Components, selectors/hover/click/style/NBT/translation/JSON/SNBT/ItemComponents und number formats; bounded parsers/commands/API. |
| DAT-09 | O | sim | DAT-03, CMD-09 | Functions/Macros/Tags/ScheduledFunctions/Predicates/Execute/Storagecommands mit Tickorder/Result/Forkbudget; reload/persistence/limitfällen. |
| DAT-10 | O | sim | CAT-02, DUR-08 | Jede Gamerule mit typed Default/Set/Query/Consumer und Difficulty/Hardcore; immediate vs tick effect, Dimension-/Player-/Savevertrag. |
| DAT-11 | O | session | DAT-04, DAT-06, DAT-07, DAT-08 | Advancement-/Stats-/Recipe-/Map-/Score-/Bossbar-/Textclientsynchronisierung, Sounds/Particles/Worldevents und Recipebook-Screens. |
| DAT-12 | O | tools | DAT-02–11 | Versionierter vollständiger Datapack-Content-/Reload-/Command-/Survivalvergleich mit gepinntem Original; default + gezielte custom packs, keine fehlenden Serializer. |

## Vollständiger Command-/Permission-Surface (CMD; D0/D7/D8)

Jede Default-Dispatcherroute wird durch CAT-04 erfasst. Die folgenden Tickets
verteilen die Familien; sie ersetzen den maschinellen Routenvergleich nicht.

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| CMD-01 | T | command | CAT-02, DAT-08 | Alle Argumenttypen/Selectors/Coordinates/Rotations/Resources/Ranges/NBT/Components, Redirect/Forks/Results/Errors/Suggestions; begrenzte Komplexität. |
| CMD-02 | T | permission | CAT-05 | Vanilla Operatorlevel plus Bukkit Defaults/Children/Attachments/Negation/Recalc/Subscriptions; Onlinestate und konsole/offline getrennt. |
| CMD-03 | T | app | CMD-02 | `/op`, `/deop`, whitelist/ban/ban-ip/pardon/list/kick und persistente Zugangsverwaltung; bisherige native-only Fixes erhalten, Java-Operator-Mutationen separat aktivieren. |
| CMD-04 | T | sim | SRV-09, SRV-11 | gamemode/difficulty/defaultgamemode/spawnpoint/setworldspawn/tp/teleport/spectate/kill/attribute/damage/effect/experience; Grenzen/Tod/Save und Results. |
| CMD-05 | T | sim | SIM-08, DUR-04 | setblock/fill/clone/fillbiome/place/locate/worldborder/time/weather/gamerule/forceload; chunk-/dimension-/schematische Änderungen und begrenzte Workjobs. |
| CMD-06 | O | sim | SRV-04, SRV-06, DAT-03 | give/clear/item/loot/recipe/enchant/summon/data mit Komponenten/NBT/Records; Itemconservation, unknown refs und Eventvertrag. |
| CMD-07 | T | sim | DAT-06, DAT-08 | scoreboard/team/bossbar/title/tellraw/say/msg/tell/w/teammsg/playsound/stopsound/particle; criteria/components/targets/results/restart. |
| CMD-08 | O | sim | DAT-04, DAT-05, DAT-10 | advancement/trigger/random/seed/ride/return und weitere Matrixrouten der Zielversion; RNG/Result/Fork/Playerfortschritt. |
| CMD-09 | O | command | CMD-01 | execute-Bedingungen/Contexts/Store/Return/Functions/Macros/Schedule/Datapack-/Debug-/Tick-Surface, bounded recursion/forks; eigene Simconsumer über DAT-09. |
| CMD-10 | O | app | DUR-12, DAT-02 | save-all/save-off/save-on/stop/reload/help/plugins/status/perf und restliche Operator-/Betriebsrouten; Konsole und Gameclient gleiche autorisierte Ausführung. |
| CMD-11 | O | app | CMD-01–10, NET-08 | Live-Commandgraph/Namespaces/Aliases/Completion bei Join/op/deop/plugin enable/disable/reload; volle Routen/UTF16/ranges/signed paths. |
| CMD-12 | O | tools | CAT-04, CMD-11 | Jede Originaldispatcherroute gegen Spielerlevel/Konsole/Namespaced execution prüfen, negative Arguments/Targets/Forks/Results und Tabmatrix inklusive Plugins. |

## Native und Java Bukkit-/Paper-API (API; D8, alte P/J)

Alle Gameplaymethoden setzen ihren Rust-Owner voraus. Eventstellen und
Cancellation werden bereits im Gameplayticket geplant, nicht erst nach einer
fertigen Mutation aufgesetzt. Der Sidecar nutzt die echte gepinnte API;
unsupported Methoden müssen konkrete Fehler liefern.

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| API-01 | T | java | CAT-05 | Vollständiger klassischer Lifecycle/Classloader/Configs/Services/Dependencies/Exceptions/PluginIdentity; echte Enable/Disable-once, Klassen-/Libraryisolation. |
| API-02 | O | java | API-01 | `paper-plugin.yml`, Bootstrap/Loader/Libraries/Dependencies/Remapping/Lifecycle; cycles/optional/load-before/sharedclasses/missing deps und reproduce Pins (J07). |
| API-03 | O | java | SIM-01 | Autoritativer synchroner API-/Scheduler-Threadvertrag, tickbasierte Delay/Repeat/Cancel und Async-Limits; gleichfälliges FIFO, Disable, Lag und deadlockfreie bidir Calls (J03). |
| API-04 | O | app | API-03 | Rusttick↔Java-Wiring, sichere reentrante Read/Mutate-/Commitpfade und Failure-State; RPC-Timeouts/Processloss/Reload verlieren keine Ownership. |
| API-05 | T | java | CMD-02, API-01 | Vollständige Commands/Tabfallback/Permissions/Services/Playeridentity/Console/Messaging/Broadcast/Adventure/Pluginmessaging; retained Playerproxies und dynamische Operatorstate. |
| API-06 | O | app | API-05, CMD-11 | Rust-Command-/Permission-/Player-/Messaging-Consumer und vollständige Snapshot-/Mutationantworten; Java-on Operatorpfad erst nach Admission-Vertrag aktivieren. |
| API-07 | O | java | SIM-11, GEN-10 | World/Chunk/Block/BlockState/BlockData/Biome/Dimension/Generation/Tickets/Time/Weather/Border und Rückgaben; methodgenaue API-Matrix (J04). |
| API-08 | O | sim | API-07, SIM-08 | Autoritative Welt-/Chunk-/Block-Read-/Mutatecapabilities, BlockPhysics/CanBuild/Place/Break/Fluid/Grow/Explosion-Eventstellen; cancellation/resync/recursive changes. |
| API-09 | O | java | SRV-23, DUR-02 | Player/Living/Entity/Attribute/Inventory/ItemStack/ItemMeta/Components/Recipe/Container/Map-/Score-/Advancement-API vollständig; reference/clone/null/value semantics. |
| API-10 | O | sim | API-09, DUR-04 | Transaktionale Inventar-/Entity-/Player-/Damage-/Teleport-/Recipecapabilities mit synchroner Antwort; Menge/Komponenten/cancellation/Screenwechsel/Reentrancy (J05). |
| API-11 | O | java | CAT-05 | Sämtliche Events/Prioritäten/ignoreCancelled/veränderliche Daten/Listener- und Plugin-Order; MONITORvertrag, HandlerLists und Disable; Familienweise Fixtures (J06). |
| API-12 | O | app | API-11, API-08, API-10 | Javaevent-Admission vor Rustmutation, Rückantworten/Timeout/Fehler und konkrete fehlende Events; Chat/Login/Join/Quit/Move/Block/Inventory/Combat/Mobs/Betrieb separat testen. |
| API-13 | T | plugin-api | Gameplayowner | Native Capabilities für stabile World/Player/Entity/Inventory/Commands/Scheduler/Storage/Events definieren; ABI-/Snapshotlebensdauer, keine rohen Internals (P02–06). |
| API-14 | T | plugin-host | API-13 | Dispatch/Lifecycle/Disable/Isolation/Taskbudget/Permission-/Eventorder mit Conservation-Vertrag; panic/cancel/pluginerror/queued-disable (P08). |
| API-15 | O | app | API-14 | Native Livecommand-/Worldsnapshot-/Player-/Task-/KV-Consumer erst nach Hostcapability verdrahten; keine angebotene Nullfassade als volle API (P01–06). |
| API-16 | O | storage | API-13, DUR-11 | Native Plugin-KV/Namespaced PersistentData/Java-Pluginzustand langlebig, Key-/Bytegrenzen und Versionsmigration (P04). |
| API-17 | O | tools | API-04–16, CAT-06 | Unveränderte externe Plugins für Permissions, Chat, Claims, Worldedit, GUI/Economy/Items, Minigames, Protection/Inventory/Combat, Scheduler und Observability; tatsächliche Funktionen/Restart. |
| API-18 | O | tools | API-17 | Kombinations-/Stress-/Failure-/Upgrade-/Disableabnahme samt kompletter API-/Eventmatrix; extra JVM/RPCkosten und Pluginreaktionen messen (J09). |

## CraftBukkit, NMS, Reflection und Packet-Plugins (NMS; D9/J08)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| NMS-01 | O | tools | CAT-06 | Exakte benötigte Klassen/Fields/Methods/Descriptors/Handles/Mojang-/Spigotmapping pro unveränderter externer Pluginversion inventarisieren. |
| NMS-02 | O | java | NMS-01, API-09 | CraftBukkit-/NMS-Fassaden, `getHandle`, Reflection und Typidentität nach tatsächlichen Verträgen; Fehler statt semantisch leerer Stubs. |
| NMS-03 | O | java | NMS-02 | Remapping/Classloader/Bytecodeverträge/Service hooks mit exakter Zielversion; Library-/Reflection-/Obfuscationrandfälle. |
| NMS-04 | O | session | NMS-01, NET-01 | Autorisierte Packet-read/rewrite/cancel/inject-Verträge und Input-/Outputorder; malformed/recursive/backpressure können Autorität nicht umgehen. |
| NMS-05 | O | app | NMS-03, NMS-04 | Java-Packet-/Internals-Wiring ohne Netzwerk-Weltmutation, Retain/Disconnect/Plugin-Disable/Failure und synchronous results. |
| NMS-06 | O | tools | NMS-05 | Unveränderte versionsgepinnt ausgewählte Protocol-/NPC-/AntiCheat-/NMS-Plugins tatsächlich ausführen; Matrix nennt jede verbleibende interne Abhängigkeit. |

## Betrieb, Pterodactyl, Sicherheit und Wartung (OPS; D10/H/O)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| OPS-01 | T | app | NET-03, DUR-12 | Startup-/Shutdown-/Readiness-/Error-/Exitcodevertrag, Signals/Config/Ports/Identity, sichere Rollen; existierende lokale Container-/Login-Fälle erhalten. |
| OPS-02 | O | dashboard | SIM-01, DUR-12 | Bounded Prometheus-Metriken für vollständige Tickzeiten/Queues/Chunks/Entities/Storage/Auth/Plugins; stabile Namen, bounded Labels (O01). |
| OPS-03 | O | app | CMD-10 | Autorisierte stdio-Admin-Konsole mit Hilfe/Save/Stop/Status; lokaler Recoverypfad, noninteractive shutdown und keine doppelte Commandautorität (O02). |
| OPS-04 | O | net | CMD-10 | RCON/Query als getrennte begrenzte Protokolle: auth/frames/limits/timeout/disable; keine unautorisierte Konsole. |
| OPS-05 | O | app | OPS-04 | RCON-/Query-Wiring samt Konfig/Permission/logging; Shutdown/Saveflush und Port-/Secretfehlkonfigtests. |
| OPS-06 | O | tools | OPS-01, API-02 | Gepinnte Linux x86_64/aarch64 Release-/Container-/Javaartefakte, SBOM/Lizenzen/Checksums/Image-Digests/CPU ABI; frischer Checkout baut reproduzierbar (H04). |
| OPS-07 | T | tools | OPS-06 | Echtes Pterodactyl Panel/Wings/Egg-End-to-End: install, allocation, start/readiness, signalstop, reinstall, Java/Plugins, backup/restore und upgrade ohne Datenüberschreiben (H02). |
| OPS-08 | O | tools | DUR-11, OPS-07 | Betreiberhandbuch/Backuprestore-/Schema-/Offline→Online-/Versionmigration mit Recoveryrunbook und realer leerer Restoreinstanz (H05). |
| OPS-09 | O | codec | CAT-04 | Vollständige hostile-input/parser-/Byte-/String-/Depth-/NBT-/Componentgrenzen, fuzz/property/replay; keine Panic/OOM durch Input. |
| OPS-10 | O | app | NET-05, API-04, DUR-12 | End-to-End Flood/Slowclient/Storagefailure/Plugincrash/Processloss/Diskfull-Abnahme; Ressourcen bounded und Fehler sichtbar, legitime Spieler kommen voran. |
| OPS-11 | O | observability | OPS-02 | Strukturierte bounded Logs/Crash-/Lagdiagnose/Tracing ohne unbounded Player/IPlabels oder sensible Sessions; Ergebnisse exaktem Commit zuordnen. |
| OPS-12 | O | tools | CAT-08, QA-09 | Versions-/Support-/Security-/Dependencies-Updateprozess und neuer Pin/Matrixdiff; 1.21.8 ist die Referenz, langfristiger Support braucht eigenes Upgradegate. |
| OPS-13 | O | tools | OPS-07–12, PERF-09 | Produktivfreigabe: veröffentlichte geprüfte Artefakte, Runbook, echte Survival-/Plugin-/Restoreresultate und bekannte Versionsgrenzen. |

## Speicher, CPU und gleichwertige Performance (PERF; M5/D10)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| PERF-01 | T | tools | – | Historische stehende Releasebasis erhalten; neue Szenarien/Commit/Hash/Hardware/Config/World/Client-/Pluginpins, CPU/RSS/PSS/volle Tickverteilung erfassen. |
| PERF-02 | O | sim | SIM-11, SIM-15 | Speicher-/Arbeit je residentem Chunk/Entity/Blocktick/Mob-/Path/Container und Freigabe nach Unload; bounded Pools/Caches ohne Gameplayverlust. |
| PERF-03 | O | net | NET-10 | Chunkstream/Kompression/Batching/Queues bei schnellen und langsamen Clients messen; Priorität/Throughput/Fairness bei gleicher Welt/Sichtdistanz. |
| PERF-04 | O | storage | DUR-11 | Writebatch/Journal/Saveflush/Loadlatency/RAM/Disk/Recovery unter tatsächlicher Mutation; gleiche Durability, kein schnellere Ergebnis durch fehlende Saves. |
| PERF-05 | O | app | API-18 | Rust/JVM getrennt und gesamte Prozessgruppe messen; RPC-/Kopier-/GC-/Pluginarbeit, synchrone APIs nur vertragskonform bündeln. Native-only startet keine JVM. |
| PERF-06 | O | sim | SIM-12 | Worker-/Shardscaling unter realer Bewegung/Updates/Transfer/AI und ungleichmäßiger Last; keine globalen Lock-/Cross-shardorderabweichungen. |
| PERF-07 | O | tools | SRV-24, BLK-28, MOB-17 | Gleichwertige aktive Paper-/FerrumC-Workloads: Generate/Explore/Build/Mine/Combat/Farms/Redstone/Mobs/Plugins; gleicher Content/Ertrag/TPS und Clientumfang. |
| PERF-08 | O | tools | PERF-07 | Langlauf/Joinstorm/Lag/slowstorage/disconnect/restart auf gepinntem Release; Heap/RSS/Queue/Chunk-/Entitywachstum und Fortschritt über Zeit. |
| PERF-09 | O | tools | PERF-01–08 | Tatsächlich gemessene CPU-/RAM-/Tickziele/Regressiongate veröffentlichen; Rust ist kein Messergebnis, fehlende Mechanik kein Speedup. |

## Integrations- und Vollständigkeitsgates (QA)

| Ticket | Status | Owner | Voraussetzung | Umfang und konkrete Abnahme |
| --- | --- | --- | --- | --- |
| QA-01 | T | testkit | – | Deterministischer Clock/Input/Storage-/Tick-/Clientharness und Fehler-Cutpoints; unabhängige Fixtures nicht aus Implementierung abgeleitet. |
| QA-02 | O | tools | QA-01, CAT-03 | Gepinnte Vanilla-/Paper-Differenzfixtures pro Ticket: Source vs Laufzeitbeleg getrennt, Config/Events/Reihenfolge und tatsächlich verglichene Observables. |
| QA-03 | O | tools | SRV-24, GEN-12 | Begrenzte Survivalalpha abnehmen und Restmatrix veröffentlichen; eigene Welt ohne Vanilla-Seed-/breite Pluginzusage. |
| QA-04 | O | tools | BLK-26–28, MOB-17, GEN-16 | Breiter Survival mit Farmen/Handel/Redstone/TNT/Portalen/Dimensionen/Bossen, Mehrspieler und Restart; vollständige Gameplaystrecke. |
| QA-05 | O | tools | GEN-17, DAT-12, CMD-12 | Vollständiger versionsfester Vanilla-Inhalts-/Worldgen-/Datapack-/Commandumfang; null fehlende Inhaltszeilen. |
| QA-06 | O | tools | API-18, NMS-06 | Deklarierte Java-/Paper-/NMS-Kompatibilitätsmatrix vollständig bestanden, Fehler/Disable/RPC-/Scheduler-/Event-/Pluginkombinationen. |
| QA-07 | O | tools | DUR-11, OPS-08 | Import/export/backuprestore/crash/schema-/identitymigration an produktiven Artefakten; Checkpoints, Cutpoints und Conservationsummen. |
| QA-08 | O | tools | QA-04–07 | Echte Vanilla-Clients für Survival/Creative/Adventure/Spectator samt Accessibility-/GUI-/Observerpfaden; Protokollbots allein belegen keine GUIdarstellung. |
| QA-09 | O | tools | CAT-08, QA-05–08, PERF-09 | Gesamtkatalogreview mit Registry/API/Consumerdiff, vollständigem CI und exact-head Artefakten; jede bewusste Abweichung/unerfasste Zeile offen benennen. |
| QA-10 | O | tools | QA-09, OPS-13 | Endgültige vollständige Spiel-/Betriebsfreigabe für deklarierte Version/Konfiguration/Pluginmatrix; Spielerfortschritt, Durability und Ressourcen gemeinsam nachgewiesen. |

## Abgenommene Teilverträge und nächste Untertickets

Die 214 Haupttickets bleiben der Gesamtumfang. Die bisherigen Wellen schließen nur
die folgenden ausdrücklich begrenzten Teilverträge; lokale kombinierte Prüfungen,
Artefakte und verbleibende Grenzen stehen im
[ersten Wellenbericht](AGENT_WAVE_01_VALIDATION.md),
[zweiten Wellenbericht](AGENT_WAVE_02_VALIDATION.md),
[dritten Wellenbericht](AGENT_WAVE_03_VALIDATION.md) und
[vierten Wellenbericht](AGENT_WAVE_04_VALIDATION.md). CI bewertet den veröffentlichten
`main`-Commit zusätzlich. Kein lokaler Teilnachweis ersetzt QA-02–10.

| Untervertrag | Status | Konkreter Umfang / Rest |
| --- | --- | --- |
| GEN-01.1 (WG-001) | F | `ferrumc-world`: Seed-/Identitäts-/Codec-/ChunkGenerator-Vertrag, Flat-v1; Quellenreview und 137 Tests. GEN-01 bleibt T bis Consumer und Persistenz aktiviert sind. |
| BLK-01.1 (PLACE-DOOR-HINGE) | F | `ferrumc-placement`: originale Cursor-/Nachbartür-/Hindernisentscheidung, 21 Materialien und beide Hälften; 56 Tests. Shapes, Support, Power und Türphysik bleiben offen. |
| BLK-01.2 (PLACE-DOOR-HINGE-CLIENT) | F | Unabhängiger Client: 14 echte Itemplatzierungen, 28 Half-States bei zwei Clients und erneut nach Prozessneustart; keine Live-Paperparität. |
| NET-06.1 (N03) | F | App-eigene IDs/ACK-Gate und endlicher FIFO-Fence für captured Syncs, Replay/KeepAlive/Stop/Persistenz und sofortiger Antwortflush; 357 App-Tests, davon sechs Teleport-Sockettests. |
| NET-06.2 (Folgeticket) | O | Gemeinsamer Epoch-/Move-Admission-Vertrag für parallel später eingereihte externe Teleports. Vor Vergabe in Producer-, Consumer- und Wiringtickets mit je einer Crate aufteilen; echter Race-/Ticktest nötig. |
| GEN-11.1 (WG-002) | F | [Storage-Ticket](agent-tasks/WG-002_GENERATION_METADATA.md), `dd910fd4`: eigene Tabelle, explizites Format 4, atomare Freshness/Saves; 146 Crate-Tests und Review CLEAN. Formatmigration und App-/Sim-Wiring bleiben getrennt; GEN-11 bleibt T. |
| SRV-01.1 (ITEMS-TRANSFER-001) | F | [Reine Split-/Transferpläne](agent-tasks/ITEMS-TRANSFER-001.md), `a7f7aa33`: Mengen-/Komponentenerhaltung, 72 Unit-/drei Doctests und Review CLEAN mit unabhängiger Probe. Gameplayconsumer, effektive Paper-Komponentenpolicy und dauerhafte Storage-Ownershiptransaktion bleiben offen; SRV-01 bleibt T. |
| CAT-02.1 | F | [Originaldatenexport](agent-tasks/CAT-02_1_ORIGINAL_CONTENT.md), `37c037b9`: ausgewählte 3.609 Originalressourcen vollständig/bytegenau geprüft, 17 Helper- und sieben Inventorytests, drei frische Reexporte und Review CLEAN. Weitere CAT-02-Kategorien und Laufzeitconsumer bleiben offen. |
| GEN-04.1 | F | [Random-/Noiseprimitiven](agent-tasks/GEN-04_1_RANDOM_NOISE.md), `e754254f`: 144 Worldtests, sieben Release-Vertragstests, unabhängiger Rootreexport unter Python `-O` byteidentisch und Review CLEAN. Diese Sampler allein erzeugen kein Terrain; GEN-04 bleibt T. |
| GEN-12.0 (CONFIG-WG001) | F | [Konfigproducer](agent-tasks/CONFIG-WG001_GENERATION.md), `402df480`: private optionale i64-Seed-/geschlossene Presetdeklaration, erhaltene Auslassung und unabhängiger Import; 30 Tests auch unabhängig grün, Review CLEAN. Produktive Flat-v1-Anbindung ist getrennt in WG-004 geprüft; vollständiger GEN-12-Umfang bleibt offen. |
| GEN-11.2 (WG-002-MIG) | F | [Explizite Migration](agent-tasks/WG-002-MIG.md), `b299a5fc`: 177 Tests auch unabhängig grün, strictchecks und Review CLEAN; typisierte Legacydeklaration, logisch raw-bytegleiches neues Backup, vollständiger Scan und atomarer Format3→4-Commit. Keine Startautomatik oder Prozesskill-/Stromausfallabnahme. |
| GEN-01.2 (WG-003) | F | [Sim-Generatorbindung](agent-tasks/WG-003_SIM_GENERATION.md), `da61c638`: opaque Weak-Arc-/Scope-/Identitätstoken, validiertes Chunkladen und all-or-nothing Dirtycapture; 204 Tests, strikte Checks, Review CLEAN nach unabhängiger Await-RED/GREEN-Repro. Appaktivierung ist getrennt in WG-004 geprüft. |
| SIM-03.1 | F | [Core EntityUuid](agent-tasks/SIM-03_1_CORE_ENTITY_UUID.md), `6998a76e`: dauerhafte 128-Bit-UUID getrennt von Wire-ID/PlayerId; Default-/No-default-/Compile-fail-Checks und unabhängige Contracttests6/4 grün, Review CLEAN. Entityownership, Tracking, UUID-Storagekeys und Mobs bleiben offen. |
| SIM-03.2a (REG-ENTITY001) | F | [Entitytypenproducer](agent-tasks/REG-ENTITY001_ENTITY_TYPES.md), `cd32fbd2`: alle 151 Entitytypen generiert, opaquer geprüfter Typ und allocfreie Lookup-/Iterationsverträge; Rootcontracts8 und Originalreexport grün, Review CLEAN. Keine aktive Entitysimulation. |
| GEN-04.2 | F | [Normal-/Simplex-/Blendednoise](agent-tasks/GEN-04_2_NORMAL_SIMPLEX_BLENDED.md), `09fd6657`: 153 Worldtests, 16.974 bitgenaue Original-JAR-Records, unabhängiger Rootreexport und zusätzliche Fremdreviewfälle grün. Review CLEAN; Densityrouter und Terrainconsumer bleiben offen. |
| GEN-12.1 (WG-004) | F | [Generation-Appanbindung](agent-tasks/WG-004_APP_GENERATION.md), `62a5b74d`: gespeicherter Seed/Format4 vor Import/Spawn/Netzwerk, verifizierte Capture-/Writer-/Stopverträge und explizite Offline-CLI. Fremdreview CLEAN, sieben Rootclientharnesses und Migration/Read-only-Restartprobe grün. Nur Flat-v1 aktiv; natürliche Welt und vollständige GEN-12-Abnahme bleiben offen. |
| SIM-03.2 | F | [Entityownership](agent-tasks/SIM-03_2_ENTITY_OWNERSHIP.md), `916251da`: shardbesessener begrenzter UUID-/Wire-/Chunkindex und generationaler Handle; Review CLEAN nach F1-RED→GREEN. Unabhängig Debug/Release jeweils227 und12.000 Modellinterleavings grün. Keine aktive App-/Mobsimulation; persistente Komponenten/Physik/Tracking folgen separat. |
| GEN-04.3 | F | [Kubische Splines](agent-tasks/GEN-04_3_CUBIC_SPLINES.md),245134eb: unabhängiger Review CLEAN nach Dokumentations-F1; Root163Worlddebug/20Release, bytegleicher Originalreexport und4294 zusätzliche Originalrecords grün. Density-/Terrainconsumer bleiben getrennt. |
| STORAGE-ENTITY001.1 | F | [UUID-Storageproducer](agent-tasks/STORAGE-ENTITY001_1_UUID_STORE.md), `a5d11819`: unabhängiges Review CLEAN, Debug/Release je212 Tests plus9 eigene Probes,39 Writecutpoints und Lostreply/Reopen grün. Fresh/strict Format5; Migration/Gameplay-/CASconsumer bleiben getrennt. |
| STORAGE-ENTITY001.2 | F | [Explizite4→5-Migration](agent-tasks/STORAGE-ENTITY001_2_UUID_MIGRATION.md), `34acc8e9`: unabhängiges Review CLEAN, Debug/Release je243 Tests plus fünf Runtime-/drei Authorityprobes grün. Geschlossenes Backup/immutable Checkpoint und vollständige Delta-/Retryvalidation. Keine Appaktivierung. |
| REG-WORLDGEN001 | F | [Originale Density-/Noisedaten](agent-tasks/REG-WORLDGEN001_BUILTIN_DENSITY_DATA.md),555ee2aa: unabhängiges Rootreview CLEAN, Debug/Release jeweils69 und4 Reviewerproben grün;102 tatsächliche Originalressourcen und195 Amplitudenbits vollständig geprüft. Keine aktive Terrainpipeline. |
| GEN-05.1 | F | [Densitycompiler](agent-tasks/GEN-05_DENSITY_COMPILER_ROUTER.md), `c4e3dc76`: unabhängiger Rootreview CLEAN, Debug/Release je177 plus3 eigene Probes und strictchecks grün.128 tatsächliche Original-JAR-Records, frischer normal/-O-Reexport bytegleich;35 Definitionen/7 Router. Seed-/Terrainconsumer bleiben offen. |
| GEN-05.2 | F | [Seed-/Scalar-/Climateconsumer](agent-tasks/GEN-05_2_SEEDED_ROUTER_REVIEW.md), `290e5343`: unabhängiger Review CLEAN, Debug/Release je188 und drei unabhängige Zusatzprobes grün. Alle17.213 Original-JAR-Records geprüft; frischer normal/-O Reexport bytegleich. Keine aktiven Chunks. |
| GEN-05.3 | A | NoiseChunk/Interpolation: vollständiger konkreter Entwurf durch Root intern freigegeben; `item_transfer_primitives` implementiert originale Wrapper-/Array-/Driversemantik, Preliminary und Climate. PaperBuild60-Policy nur im neuen Chunkpfad; Material-/Terrainaktivierung folgen. |
| DUR-03.1 | A | [Player↔Entity-Transferproducer](agent-tasks/DUR-03_1_PLAYER_ENTITY_TRANSFER.md): vollständiger Format6-/CAS-/Receiptentwurf intern freigegeben, `select_world_ticket` implementiert tatsächliche Storageauthority. Drop/Pickup-/Appconsumer und DUR-03-Gesamtumfang bleiben offen. |
| SRV-05.1 | A | [Normale Craftingrezepte](agent-tasks/SRV-05_1_NORMAL_CRAFTING_RECIPES.md): `feature_catalog` konkretisiert Daten/API für alle968 shaped/shapeless Rezepte mit Originaltags, Komponenten und Craftingresten. Rootreview vor Runtime; Onlinecrafting und weitere Rezepttypen folgen. |
| SIM-03.3 / SRV-02.1 | F | [Besessene ItemEntities](agent-tasks/SIM-03_3_ITEM_ENTITY_STATE.md), `84967c54`: Rootreview CLEAN, Debug/Release je252 Tests samt3 zusätzlichen Probes und strictchecks grün; sealed Slotstate, lossless FCIE/FCIS, owned Refusals/Removal, checked Restore und originale Counter. Drop/Pickup/Appaktivierung folgen; SIM-03/SRV-02 bleiben offen. |

## Agentenrollen, Board-Snapshot und Orchestrationsregeln

Es gibt derzeit **vier gleichzeitige Slots: einen Orchestrator und drei
Implementierungsagenten**. Der Orchestrator nimmt Betreiberrequests entgegen,
priorisiert, vergibt Tickets/Dateibesitz, verwaltet Abhängigkeiten, veranlasst
Reviews/Integration und führt als einziger Gitoperationen auf `main` aus.
Er implementiert keine Runtimefeatures. Ein Reviewer verwendet einen frei
gewordenen Umsetzungsslot; vor neuer Vergabe wird bestehende Arbeit beendet
oder ausdrücklich umpriorisiert. Keine unbounded Agentenschlange.

| Rolle / Agent | Ticket | Aktueller Status, Welle 4 | Exklusiver Dateibesitz / nächste Entscheidung |
| --- | --- | --- | --- |
| Orchestrator `/root` | Board/Integration/Betreiberrequests | aktiv | Vergaben, Git, finaler Prüflauf und Status; keine Runtimeimplementierung. |
| Rezeptagent `feature_catalog` | SRV-05.1 | Konkreter API-/Datenentwurf | Items-Crate und eigener Rezeptbrief; GEN-05.2-Fremdreview abgeschlossen und CLEAN integriert. |
| Storageagent `select_world_ticket` | DUR-03.1 | Runtimeimplementierung freigegeben und begonnen | Storage-Crate und Transferbrief; .2-Migration CLEAN integriert, eigene Format6authority. |
| Weltagent `item_transfer_primitives` | GEN-05.3 | Runtimeimplementierung freigegeben und begonnen | World-Crate und GEN-05-Brief; .2-Seedrouter CLEAN als `290e5343` integriert. |

Diese Rollenliste erfasst die Aufträge der laufenden Welle; die tatsächlichen
aktiven Slots stehen im Implementierungsboard. Reviews nutzen frei gewordene
Slots. Root aktualisiert Agentname,
genaue Ticketuntermenge, Dateien und Status nach tatsächlicher Vergabe.
Wenn ein Feature an fehlenden Verbrauchern hängt, erhält der nächste freie
Agent das abhängige Producer-/Consumer-/Wiringticket, statt beliebig andere
Runtimepfade mit unvollständigem Zustand zu aktivieren.

Die erste Welle ist der erste Lieferabschnitt des Gesamtspiels. Nach WG-001
folgen WG-002 (Storage-Descriptor, DUR-01/GEN-11), WG-003 (Sim-Consumer, GEN-01)
und WG-004 (App-Wiring, GEN-12) als getrennte Tickets; natürlicher Terrainbau
folgt über GEN-02. Survival beginnt mit DUR-/SRV-Grundlagen nach dem jeweiligen
Ownershipvertrag. Kein erster Foundation-/Placement-/Teleportfix schließt das
vollständige Spielziel. Der Orchestrationsvertrag gilt für spätere Requests und
Wellen weiter; persistierter Status bedeutet keinen zugesicherten Hintergrundlauf
nach beendeter Sitzung. Das Implementierungsboard ist für tatsächliche
Zuweisungen maßgeblich, die Katalogzeilen für Zielumfang und Restabnahme.

Ein Ticket wird nur mit klar begrenztem Umfang zugewiesen. Große Tabellenzeilen
werden vor Vergabe als `<ID>.1`, `<ID>.2` usw. in weitere **one-crate** Untertickets
geteilt: Familien-/Arten-/API-/Routeuntermenge, Dateibesitz, Vorbedingungen,
Abnahme und Restumfang stehen im Brief/Board. Ein einzelnes Teilfeature kann
geschlossen werden; seine Tabellenzeile bleibt T bis alle zugeordneten Inhalte
abgenommen sind. Shared-Dateien (`Cargo.toml`, Lockfile, zentrale Driver-/Shard-
Module) werden exklusiv reserviert oder nacheinander editiert. Keine Agenten-
Gitcommits, keine Branches, kein Reformat fremder offener Änderungen.

Die Rollenliste oben ist der Initialsnapshot. Laufende Zuordnung/Dateibesitz
werden ausschließlich im [Implementierungsboard](IMPLEMENTATION_BOARD.md)
nachgeführt; hier bleibt der Gesamtumfang bestehen. Haupttickets erhalten nach
Teilfortschritt höchstens T; F ist an den konkreten Unterticketumfang und seinen
Nachweis gebunden, nicht bloß an eine Vergabe. Root schreibt langfristig keinen Gameplaycode und nimmt neue
Betreiberrequests/Befehle für Priorisierung und Agentenvergabe weiter entgegen.

### Ablauf je Ticket und Integration

1. Agent liest AGENTS/Crate-Invarianten und relevante Architektur/ADR; prüft
   gepinnte vollständige Basis und beide Paper-Patchschichten. Quellen-/Event-
   /Ownershipmatrix aus der Paper-Referenz vor der Änderung festhalten.
2. Reproduktion/Vertrag und unabhängige erwartete Zustände festlegen; dann
   kleinsten Producer oder Consumer in genau einer Crate implementieren.
   Keine Laufzeitaktivierung vor erfüllten Ownership-/Durability-Voraussetzungen.
3. Gezielte deterministische Tests und sinnvolle hostile/error/boundary-Fälle;
   echte Socket-/Client-/Plugin-/Paperfixtures für ihren konkreten Umfang.
   Format, striktes Clippy und relevante Generator-/Claim-/Docchecks.
4. Agent liefert Dateien, konkrete Änderungen, Quellen, Befehle/Ergebnisse,
   Grenzen und Consumerrest zurück. Freier Reviewer prüft unabhängig,
   einschließlich Fehler-/Restart-/Eventreihenfolge, ohne gleichzeitige Edits.
5. Root integriert sequenziell, testet die zusammengeführte Version und
   aktualisiert Board/Matrix mit exact-head Nachweis; nur Root committet/pusht
   kleine reviewbare Schritte auf `main`. CI für diesen Head gehört zur Abnahme.
6. Nächsten bereiten Abhängigkeitsschritt vergeben; Betreiberbugs erhalten
   Reproduktion/Regression und werden in passende Ticket-/Inhaltszeilen geführt.

### Verbindliche gemeinsame Abnahme

Für mutierende Gameplayfeatures: serverautoritatives Verhalten, vollständige
Komponenten-/Mengenconservation, zwei konkurrierende Spieler/Betrachter,
Cancellation/Pluginänderungen/Reentrancy, volle Queues, verlorene Antworten,
Disconnect, Chunk-Unload, Crash/Restart und passende originalgetreue Tickorder.
Einfache reine Grundlagen erhalten entsprechende Contract-/Malformed-/Seed-
Tests; Runtime-/Clientgates werden erst beim tatsächlichen Consumer erfüllt.

Für Originalparität: dieselbe Version, Paperkonfiguration/Gamerules/Plugins und
dieselbe Eingabefolge. Nur transportbedingte Identitäten normalisieren; keine
Eventorder, Komponenten, Mengen, Timer oder Fehler entfernen. Quellenlesen,
eigene Unittests, unabhängige Wirefixtures und ein wirklich ausgeführter
Papervergleich sind verschiedene Nachweise. Bewusste Abweichungen werden
im Featurebericht und Inhaltsmatrix sichtbar geführt.

Für Ressourcen: vollständige Tickzeiten/Progress, aktive Chunks/Entities,
Queue-/Byte-/Heapgrenzen, Rust und JVM gemeinsam sowie getrennt. Aufgeschobene
Arbeit muss fair fortschreiten. Die historischen acht stehenden Clients
belegen keine Geschwindigkeit bei vollständig gleichem Survival-/Pluginumfang.

Die bisherigen Aufwandsspannen im Survival-Lieferplan bleiben eine frühe
Größenordnung. Parallel laufende Agenten sind kein Ersatz für Entwicklerjahre,
Originalverifikation und Integration; sie begründen keine neue feste Dauer.
Nach ersten abgenommenen Tickets wird die Planung anhand tatsächlicher
Durchsatzdaten und neu erfasster Matrixzeilen aktualisiert.
