# FerrumC: Spielbarkeit, Survival und Paper-Kompatibilität

Stand: 10.10.2026. Geprüfter Ausgangspunkt: `0616402019fe2a3134e2cc3a46c4f3f88582aeed`.
Arbeitsbranch: `main`. Dies ist der priorisierte Lieferplan zum Betreiberbericht
über fehlende Weltgenerierung, Mobs, Wasserfluss, Redstone, TNT, Survival,
Bogen/PvP, Befehle und fehlerhafte Blockplatzierung. Der technische Einzelbacklog
bleibt im [v2-Implementierungsplan](V2_IMPLEMENTATION_PLAN.md).

Der neue [Minecraft-Gesamtfeaturekatalog](MINECRAFT_FEATURE_CATALOG.md) ist der
kanonische Inhalts-/Featurebacklog mit versionsfesten Matrizen, one-crate Tickets,
Abhängigkeiten, Abnahme und Agentenboard. Dieser Lieferplan bleibt die Grundlage
für Phasen, kritischen Pfad und die bisherige grobe Aufwandsschätzung.

## Einordnung des aktuellen Stands

FerrumC ist derzeit eine Entwicklungsgrundlage mit Anmeldung, Chunk-Streaming,
einigen Bau-/Inventaraktionen, Persistenz und einer kleinen Java-API-Teilmenge.
Ein normal spielbarer Survival-Server oder ein Paper-Ersatz ist damit nicht
erreicht. Bestandene Codec-, Socket- und Fixture-Tests belegen jeweils ihren
begrenzten Vertrag; sie belegen keinen vollständigen Minecraft-Spielablauf.

Die ursprüngliche Planung ist durch erste D0-Implementierung ergänzt:
Torch-Items verwenden nun ihre stehende Placement-Basis. Echte Verbindungen
prüfen Boden und vier Wandrichtungen für alle drei Familien samt Beobachter
und Redb-Neustart. Bestehende Zäune aktualisieren nach regulärem Place/Break
das betroffene Verbindungsbit und behalten Wasser/andere Arme. Der [D0-Prüfbericht](D0_PLACEMENT_VALIDATION.md) hält
Artefakt, Umfang und offene Physik fest. Die weitere Migration prüft jetzt
Holz-/Nether-Familien, Gate-Achsen und 33 Verbindungsausnahmen. Native-only
persistente `/op`-/`/deop`-Einzelziele samt laufenden Commandgraphs und
lokaler CLI-Wiederherstellung sind im [Operator-Prüfbericht](D0_OPERATOR_VALIDATION.md)
beschrieben. Java-Mutation und volle Supportflächen bleiben offen.
Die übrigen Laufzeitprobleme und
vollständige Spielbarkeit bleiben offen.

| Meldung | Am Ausgangscode belegter Stand | Geplante Abnahme |
| --- | --- | --- |
| Keine normale Weltgenerierung | [world.rs](../server/app/src/world.rs) verwendet ausschließlich `FlatWorldGenerator`; [generator.rs](../server/crates/ferrumc-world/src/generator.rs) erzeugt feste horizontale Schichten. Importierte Chunks ersetzen keine natürliche Generierung. | Neue Welt liefert Terrain, Biome, Ressourcen und Höhlen; derselbe Seed bleibt bei anderer Ladefolge und Neustart gleich. |
| Fackeln immer an der Wand | Der reale Item-Pfad führt über [item_id.rs](../server/crates/ferrumc-items/src/item_id.rs) zum [Registry-Mapping](../server/crates/ferrumc-registry/data/item_to_block_mapping.json): Torch-Items liefern Wandvarianten. [placement/lib.rs](../server/crates/ferrumc-placement/src/lib.rs) klassifiziert dagegen Bodenvarianten als Torch. Bestehende Torch-Tests verwenden direkt den Boden-Blockstate. Über echte Inventaritems reproduziert und durch generierte Normalisierung der drei Familien behoben; Boden-/Vier-Wandmatrix und Restart bestanden. Support-/Fallback-/Abbauphysik bleibt offen. | Normale Torch-, Soul-Torch- und Redstone-Torch-Items auf Boden und Wand, Tragblock entfernen, zwei Betrachter und Neustart prüfen. |
| Zäune und andere Verbindungen fehlerhaft | [shard.rs](../server/crates/ferrumc-sim/src/shard.rs) aktualisiert Nachbarn nur nach FenceLike-Platzierung und nur desselben Zaunnamens, nicht allgemein nach Abbau oder anderen Edits. Recompute in [placement/lib.rs](../server/crates/ferrumc-placement/src/lib.rs) verliert Waterlogging. [Registry-Build](../server/crates/ferrumc-registry/build.rs) setzt grobe `boundingBox=block`-Metadaten mit Vollwürfeln gleich. Begrenzter Refresh nach regulärem Place/Break und Erhalt bestehender Properties implementiert und über echte Clients/Restart geprüft; Holz-/Nether-Familien, Gate-Achsen und 33 Ausnahmen inzwischen in beide Live-Consumer migriert. Volle State-/Face-Flächen, übrige Mutationseinstiege und andere Blockfamilien bleiben offen. | Tatsächliche Items nacheinander setzen/entfernen, Verbindungen beider Seiten, verschiedene Holz-/Nether-Zäune, Torachsen, Tragflächen, Waterlogging, Chunk-Grenzen und mehrere Spieler prüfen. |
| Redstone verbindet/schaltet nicht | Kein vollständiges Wire-, Power- oder Redstone-Ticksystem im produktiven Simulationspfad. Darstellung und Signalverhalten sind separate Lücken. | Staubformen und Signalstärke, Schalter, Repeater, Comparator, Observer und Kolben anhand festgelegter Schaltungen vergleichen. |
| Wasser fließt nicht | Waterlogging beim Platzieren ist teilweise vorhanden; es gibt keinen produktiven Fluid-Tick-Pfad. | Quellen, Gefälle, Ausbreitung, Entfernen, Eimer, Wasser/Lava-Reaktionen, Chunk-Grenzen und Fortsetzung nach Restart. |
| TNT funktioniert nicht | Kein produktiver Primed-TNT-/Fuse-/Explosionspfad. | Zündung, Fuse, Bewegung, Explosion, Block-/Entityschaden, Kettenreaktion, Drops und Neustart. |
| `/op` und Befehle fehlen/gehen nicht | [command.rs](../server/app/src/command.rs) registriert unter anderem `/gamemode`, `/tp` und `/time`, aber kein `/op`/`/deop` und keinen vollständigen Vanilla-Befehlssatz. Rechte kommen aus konfigurierten `ops` in [plugins.rs](../server/app/src/plugins.rs). Nur unsigned `ChatCommand` ist in [packets.toml](protocol/1_21_8/packets.toml) modelliert; die unabhängige Protokollfixture enthält zusätzlich signed commands. Das erklärt nicht automatisch jede gemeldete Command-Störung. Native-only persistente Offline-Einzelziele für `/op`/`/deop`, Live-Graph-Refresh und lokale CLI-Recovery sind inzwischen implementiert; Java-Mutation und breite Vanilla-Befehle bleiben offen. | Echter Client: Operator und normaler Spieler, Ausführung, Fehlermeldung, Argumente, Completion, Namespaces, dynamische Rechte und Restart; Paketpfade abhängig vom tatsächlichen Client prüfen. |
| Survival geht nicht | Der GameMode kann gespeichert/gesetzt werden; [translate.rs](../server/crates/ferrumc-session/src/translate.rs) behandelt Abbau überwiegend als Creative-Startaktion. Mining-Fortschritt, Loot, Hunger und Gesundheit bilden keinen vollständigen Survival-Ablauf. | Neue Welt ohne Creative-Hilfen: Holz, Werkzeug, Ressourcen, Crafting, Nahrung, Schaden, Tod, Respawn und Restart. |
| Keine Mobs | Im produktiven [SimShard](../server/crates/ferrumc-sim/src/shard.rs) fehlt eine allgemeine Mob-Laufzeit mit Spawn, KI und Navigation. | Spawn-/Despawnregeln, Tageslicht, Kollision, Verfolgung, Kampf, Drops, Farmen, Chunk-Unload und Restart. |
| Bogen und PvP offen | Kein vollständiger Item-Use-/Projectile-/Damage-Pfad. Sichtbare Remote-Spieler und Ausrüstung belegen keinen Kampf. | Zwei echte Clients: Bogen laden/schießen, Pfeilverbrauch, Treffer, Rüstung, Schild, Knockback, Immunitätsfenster, Tod/Respawn; PvP-Schalter prüfen. |

## Aufwand und Annahmen

Die folgenden Spannen sind eine frühe Engineering-Schätzung aus dem geprüften
Umfang, keine empirisch kalibrierte Lieferzusage. Es gibt noch keine gemessene
Umsetzungsgeschwindigkeit für diese neuen Gameplay-Systeme. Nach den ersten
zwei bis vier Arbeitswochen wird mit tatsächlichem Fortschritt neu geschätzt.

Kalenderszenario: **drei erfahrene Vollzeit-Kernentwickler** mit Rust-, Minecraft-
und Java-API-Erfahrung, dazu etwa **eine halbe QA-Stelle** für echte Clients,
Paper-Vergleiche und Langläufe. Keine andere Vollzeitpriorität, festgehaltene
Minecraft-Version, kontinuierliche Integration und verfügbare Testumgebung.
Ein Agent oder eine einzelne Chatsitzung entspricht nicht diesem Personalplan.

Eine Personenwoche bedeutet eine Arbeitswoche eines Entwicklers. Die Schätzung
enthält Feature-Recherche, Implementierung, Tests und Integration des jeweiligen
Pakets. Zusätzlich werden **25–40 % Reserve** für unerwartete Verhaltenslücken,
Rework und Stabilisierung eingeplant. Parallele Teilaufgaben verkürzen die
Kalenderzeit nur, wenn ihre Grundlagen und Schnittstellen bereits stehen.

| Lieferpaket | Geplante Arbeit und Reihenfolge | Entwicklungsaufwand | Fertig, wenn … |
| --- | --- | --- | --- |
| D0: gemeldete Fehler und Bedienbarkeit | Reproduktion mit genauem Build/Config; echte Item-Mappings korrigieren; Torch/Fence-/Placement-Pfade prüfen; vorhandene Commands zugänglich machen; `/op`/`/deop`, persistente Rechte, Commandbaum-Updates und administrativen Zugang ergänzen. N03/N06/I04 nach Bedarf zuerst. | 4–8 Personenwochen | Jeder gemeldete Fix hat einen tatsächlichen Client-/Item-Test; Berechtigungen und Rechteentzug funktionieren nach Rejoin/Restart. Noch kein Survival-Release. |
| D1: gemeinsame Simulationsgrundlage | Autoritative Inventare und Entities, eindeutige dauerhafte Identität, Chunk-Entity-Index, crash-atomare Item-Transfers, Collision-Shapes, Bewegung/Spatial-Index, Entity-Ticking und begrenzte Block-/Nachbar-/Random-Ticks. Grundlagen zuerst, Aktivierung erst nach Verbraucher-Migration. | 18–30 Personenwochen | Drops/Pickup/Merge/Despawn konservieren Items; Lost-Reply, zwei konkurrierende Spieler, Disconnect, Crash und Unload sind abgefangen. Tick-Arbeit und Heap sind begrenzt. |
| D2: Terrain und Licht | Versionierter Seed-/Generatorvertrag, natürliche Overworld mit Biomen, Höhlen, Bäumen, Erzen und Features; Heightmaps; echtes Block-/Skylight. Zunächst spielbare eigene Generierung, exakte Vanilla-Seed-Parität zusätzlich in D7. Kann nach den Datenverträgen teilweise parallel zu D3 laufen. | 24–44 Personenwochen | Neue Welt enthält erreichbare Survival-Ressourcen; Chunk-Grenzen und Ladefolge verändern den Seed nicht; Licht nach Blockänderungen und Restart stimmt. |
| D3: Survival-Spielkreislauf und Kampf | Mining/Tools/Loot/Durability; Rezepte und vollständige Klickmodi; Health/Hunger/Food/Effects/XP; Tod/Respawn; Movement-/Fallregeln; Melee/PvP, Rüstung/Schild, Bogen und Pfeile. Teilaufgaben nacheinander auf D1; Pluginschnittstellen vor Aktivierung festlegen. | 26–48 Personenwochen | Mehrere echte Spieler können ohne Creative-Items abbauen, herstellen, essen, kämpfen und nach Tod/Restart weiterspielen. Mengen und Komponenten bleiben korrekt. |
| D4: lebende Welt und Container | Wasser/Lava und Eimer; Wasserlogging-Interaktionen; Feuer, Wachstum und Landwirtschaft; Furnace/Smelting, Hopper und weitere Container. Erste Wasser-/Ofen-Teilmenge für die Alpha vorziehen. | 14–28 Personenwochen | Fluss, Wachstum, Brennstoff und Transfers folgen festgelegten Ticks; Farms überstehen Chunk-Grenzen und Restart ohne Itemverlust. |
| D5: Mobs | Zunächst wenige passive und feindliche Arten; Spawn-/Despawncaps, Navigation, KI, Kampf/Loot. Danach Zucht, weitere Arten, Dorfbewohner/Handel, komplexere KI und Bosse. Erste Arten für die Alpha vorziehen. | 24–48 Personenwochen | Spawn und Verhalten sind mit der festgelegten Version geprüft; Farmen, Kampf und Persistenz bestehen. Mobzahl/Navigation haben gemessene Arbeitsgrenzen. |
| D6: Redstone, TNT und Blockmechanik | Wire-Verbindungen und Power, Tick-Reihenfolge, Schalter, Repeater/Comparator/Observer, Kolben und relevante Blöcke; TNT-Zündung/Fuse/Explosion/Kettenreaktion und weiterer Explosionsschaden. TNT-Grundpfad kann nach D1/D3 vor breiter Redstone-Abdeckung kommen. Paper-Redstone-Implementierung/Konfiguration für die Referenz ausdrücklich festlegen. | 20–40 Personenwochen | Referenzschaltungen und TNT-Fälle entsprechen dem vereinbarten Paper-Verhalten, einschließlich Chunk-Grenzen, Reihenfolge und Restart. Ein leuchtender Wire ist kein Abschluss. |
| D7: breiter Vanilla-Umfang | Nether/End, Portale, Strukturen und genaue Generator-Parität, restliche Block-/Entity-/Itemfamilien, Vehicles/Projectiles, Enchanting/Brewing, Maps/Datapacks/Gamerules und vollständiger versionsfester Command-Surface. Inventar der offenen Klassen/Blöcke/Commands führen. | 24–48 Personenwochen, besonders optimistische Grobschätzung; vor Lieferzusage zerlegen und neu schätzen | Die Inhaltsmatrix ist abgearbeitet, Dimensionen und Fortschritt bis Nether/End/Bossen funktionieren; fehlende Zweige sind nicht hinter Defaultverhalten versteckt. |
| D8: Java-/Paper-API | Scheduler/Thread-Vertrag, Permissions/Services, World/Block/Chunk, Player/Entity/Inventory/ItemMeta/Recipes und Events. Mutation/Cancellation/Reentrancy auf Rust-Tickgrenzen abbilden. Contracts parallel vorbereiten; die konkrete API erst nach ihrem Gameplay-Owner aktivieren. | 60–120 Personenwochen | Unveränderte, versionsgepinnt ausgewählte externe Plugins verschiedener Kategorien laufen mit echten Clients, Weltänderungen, Inventaren und Restart. Loader-Fixtures genügen nicht. |
| D9: Internals und restliche Pluginoberfläche | `paper-plugin.yml`, Bootstrap, Libraries/Dependencies, Mapping; CraftBukkit-/NMS-Klassen/Handles, Reflection, Packet-Hooks und verbleibende API-Verträge. Umfang pro Plugin/Version erfassen. | 40–100+ Personenwochen | Die deklarierte externe Kompatibilitätsmatrix besteht; konkret benötigte Internals sind implementiert und versionsgenau getestet. Kein universeller Nachweis für beliebige JARs. |
| D10: produktiver Betrieb und Performance | Online-Auth/Verschlüsselung, Spieleridentitätsmigration, Backup/Restore/Recovery, Release-/Upgradepfad und echte Panel/Wings-Abnahme. Metriken/Profiling, Longruns, aktive Gameplay-/Plugin-Last und gleichwertige Paper-Vergleiche. Gates von Beginn an, abschließende Stabilisierung nach Integration. | 20–36 Personenwochen | Identität, Daten und Stop/Recovery sind geprüft; aktive Workloads halten das erklärte TPS-/Ressourcenbudget; veröffentlichte Artefakte sind exakt zugeordnet. |

Grob summiert: **etwa 170–330 Personenwochen für Gameplay/Betrieb** und
**weitere 100–220+ für Java-/Paper-Kompatibilität**, jeweils vor der Reserve.
Dies ist eine Größenordnung; sie macht den erheblichen Neuentwicklungsanteil
sichtbar. Die Spannen einzelner Pakete sind keine statistischen Konfidenzintervalle.
Rechnerisch ergeben der gesamte Backlog und die Reserve etwa **340–770+
Personenwochen**. Selbst bei idealer Aufteilung auf drei Entwickler wären das
etwa **26–60+ Monate**; Abhängigkeiten und besonders unsichere Teile von D7/D9
kommen hinzu. Exakte Worldgen, vollständige Datapacks und interne Plugin-Hacks
können den oberen Korridor deutlich überschreiten.

## Wann welche Nutzung realistisch wird

Die Zeiten sind **kumulativ ab kontinuierlichem Arbeitsbeginn**, unter dem
genannten Personalszenario. Sie sind nicht zu addieren. Ein Startdatum oder
tatsächlich besetztes Team wird damit nicht behauptet.

| Ergebnis | Grobe Kalenderzeit | Konkreter Lieferumfang |
| --- | --- | --- |
| Erste sichtbare Korrekturen | **1–3 Wochen**, bei tiefen Ursachen/Integration **4–6 Wochen** | Die ersten nachvollziehbar behobenen Placement-/Command-Probleme; D0 als Ganzes kann länger dauern. |
| Begrenzte Survival-Alpha | **6–12 Monate** | D1 plus erste Teile von D2–D5: eigene generierte Overworld, Mining/Crafting, Gesundheit/Nahrung, Items, erste Mobs, Wasser/Öfen, Melee/Bogen und Restart. Noch offene Arten, Schaltungen, Dimensionen und Plugins werden ausdrücklich benannt. |
| Breiter, stabiler Survival-Betrieb | **18–30+ Monate** | D0–D6, spielrelevante D7-Teile und Betriebsgates: Dimensionen, Mobvielfalt, Farms, Redstone/TNT und weitere Spielinhalte mit belastbarer Abnahme. Exakte Worldgen-/Redstone-Parität und restlicher D7-Umfang können die obere Grenze überschreiten. |
| Breite Paper-Kompatibilität mit Produktivabnahme | **24–48+ Monate** | D8/D9/D10 auf derselben Spielgrundlage, zahlreiche unveränderte externe Plugins und konkret geprüfte NMS-Nutzung. Das bleibt eine Zielspanne mit sehr großer Unsicherheit. |
| Vollständiger derzeit geplanter Zielumfang | **30–60+ Monate** | Gesamte D0–D10-Inhalts-/API-Matrix einschließlich exakter Generator-Parität und übriger Internals. Die breite Plugin-Stufe davor prüft eine Teilmenge. Obergrenze offen bei neuen Versionen oder nicht erfassten Sonderfällen. |

Bei nur **einem erfahrenen Vollzeitentwickler** bedeutet bereits der Gameplay-
Umfang grob **4–9+ Jahre** einschließlich Reserve; der gesamte API-/Internals-
Umfang kann das Gesamtprojekt auf **7–15+ Jahre** bringen. Teilzeitarbeit
verlängert die Kalenderzeit entsprechend. Mehr Personen beseitigen die
Abhängigkeiten nicht; einfach durch Kopfzahl zu teilen wäre irreführend.
KI-Unterstützung kann Recherche und einzelne Implementierungen beschleunigen,
ersetzt aber keinen Nachweis für Integration, Spielverhalten und Langzeitstabilität.

## Kritischer Pfad und verbindliche Qualität

Zuerst D0 reproduzierbar beheben und D1 aufbauen. D2 kann anschließend mit
D3 parallel laufen; D4 und erste D5-Arten folgen den Tick-/Entity-Verträgen.
D6 setzt Nachbar-Ticks, Kollision und Damage voraus. D7 erweitert den bestätigten
Spielkreislauf. D8-Schnittstellen und Eventstellen werden schon während D1–D7
mitgeplant, damit nachträgliche Cancellation keine Architektur-Neufassung
erzwingt. Die heute fehlende Welt-/Inventarlogik kann nicht durch eine
Java-Fassade vorgezogen werden. D9 und die breite externe Abnahme kommen auf
eine stabile Basis. D10 begleitet jeden Schritt.

Die ersten Arbeitspakete werden in dieser Reihenfolge konkretisiert:

1. Exaktes Testartefakt/Config, echte Clientfälle für Torch/Fence/Commands;
   vorhandene Tests um den tatsächlich verwendeten Item-/Packet-Pfad ergänzen.
2. Torch-/Item-Mapping als eigener Registry-/Generator-Schritt korrigieren,
   anschließend Placement-/Nachbarfälle getrennt schließen.
3. Admin-Zugang, persistente `/op`-/`/deop`-Mutationen und nachgeführte
   Command-/Permission-Sichten; Signatur-/Paketpfade getrennt bearbeiten.
4. D1 in kleine Crate-Aufgaben teilen: Inventarmodell, Storage-Transfervertrag,
   Sim-Autorität, App-Migration, Shapes, Entities/Protokoll/Visibility und
   tickbasierte Weltmechanik. Keine Drop-/Pickup-Aktivierung vor atomarer Ownership.

Jeder Schritt verändert eine Produktions-Crate; Cross-Lane-Verbraucher werden
danach separat verdrahtet. Keine Handänderungen an generiertem Protokoll oder
Registries. Die [Paper-Referenzmatrix](PAPER_REFERENCE.md) verlangt den passenden
API-/Basis-/Patch-Code, Events, Konfiguration und festgehaltene Abweichungen.
Vollständige Basisquellenprüfung und laufende Paper-Vergleichsfixtures müssen
für die jeweiligen Feature-Gates ergänzt werden; sie sind heute noch offen.

Abnahme besteht aus Verhalten, Fehlerpfaden, zwei echten Clients, unabhängigen
Packet-/Item-Fixtures, Crash-/Restart-Cutpoints und messbarer Ressourcenarbeit.
Zwei Spieler müssen Änderungen gleichzeitig sehen. Ein Einzeltest oder ein
umschaltbarer GameMode erfüllt kein Meilenstein-Gate. Ein Bug bleibt offen bis
Reproduktion und tatsächlicher Fix-Abnahme; große Testzahlen ändern das nicht.

Performance wird fortlaufend bei aktiven Systemen gemessen: Rust und JVM
gemeinsam, RSS, vollständige Tickzeit-Verteilung, Queues, residente Chunks und
Entities. Die bestehende stehende Ausgangsmessung ist kein Paper-Speedup.
Auch Budgets müssen fairen Fortschritt gewährleisten; langsamer oder gar nicht
ausgeführte Spielmechanik darf keinen vermeintlichen Geschwindigkeitsvorteil liefern.

## Versions- und Pluginziel

Die Entwicklung referenziert zunächst Minecraft/Paper **1.21.8**. Diese alte
Referenzversion ist kein langfristiger Maintenanceplan. Vor öffentlicher
Produktivfreigabe wird ein eigener Upgrade-/Support-Meilenstein festgelegt;
neue Protokolle, Daten, Java- und NMS-Versionen können die Schätzung erhöhen.

Das Betreiberziel bleibt normaler Survival und unveränderte Java-Plugins.
Ein überprüfbares Releaseversprechen nennt aber immer Serverversion,
Pluginversion und bestandene Funktionen. **Alle beliebigen `.jar`-Dateien**
über sämtliche Versionen, interne Hacks und Pluginkombinationen lassen sich
nicht mit einem endlichen Testplan garantieren; selbst Paper bietet keine
solche versionsübergreifende Zusage. Bukkit/Paper-Verträge werden systematisch
erweitert, NMS-/Reflection-/Packet-Abhängigkeiten ausdrücklich geführt.
Fehlende Methoden und bewusste Verhaltensabweichungen bleiben sichtbar.

Alle Pakete D0–D10 bleiben **offen**. D0 hat einzeln abgenommene
Placement-Korrekturen und native-only Operator-Persistenz; das gesamte Paket und der Survival-Umfang sind damit
noch nicht abgeschlossen.
