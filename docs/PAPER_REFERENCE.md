# Paper als Implementierungsreferenz

Stand: 10.10.2026. Diese Referenz ergänzt den
[v2-Plan](V2_IMPLEMENTATION_PLAN.md) und
[ADR-0009](adr/0009-java-plugin-compatibility.md). FerrumC bleibt ein
Rust-Spielserver mit optionaler Java-API-Brücke. Quellenprüfung und geplante
Tests bedeuten nicht, dass die entsprechenden Funktionen bereits laufen.

## Festgelegte Versionen

| Referenz | Festlegung und Nachweis |
| --- | --- |
| Minecraft | Java Edition 1.21.8, Protokoll 772; dieselbe Version wie FerrumC. |
| Paper-Server | **1.21.8 Build 60**, Commit `29c8822d90899c89d2689338e81a98f690bcba12`. Die [offiziellen Build-Metadaten](https://fill.papermc.io/v3/projects/paper/versions/1.21.8/builds/60) ordnen diesen Commit dem Build zu. |
| Paper-Serverartefakt | Laut Build-Metadaten `paper-1.21.8-60.jar`, SHA-256 `8de7c52c3b02403503d16fac58003f1efef7dd7a0256786843927fa92ee57f1e`. Das JAR wurde in dieser Prüfung weder heruntergeladen noch gestartet; dies ist der veröffentlichte Sollwert. |
| Bridge-API | `io.papermc.paper:paper-api:1.21.8-R0.1-20250906.215025-55`, festgelegt in [pom.xml](../server/java-bridge/pom.xml). Der API-Pin ersetzt keinen Server-Pin. |
| Bridge-API-Quellen | [Offizielles Sources-JAR](https://repo.papermc.io/repository/maven-public/io/papermc/paper/paper-api/1.21.8-R0.1-SNAPSHOT/paper-api-1.21.8-R0.1-20250906.215025-55-sources.jar), lokal geprüfte SHA-256 `f9ecfe790b21408eb0c108bc27daab63b104dbb5eb644776f4ca5f270c19050d`. |
| Basis und Werkzeugkette | Im gepinnten [Server-Build](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/build.gradle.kts): Mache `1.21.8+build.2`, Spigot BuildData `436eac9815c211be1a2a6ca0702615f995e81c44`; Java 21. |

Der offizielle Checkout liegt lokal unter
`.codex-tmp/paper-1.21.8-reference/` und ist nicht Bestandteil
des FerrumC-Repositorys. Eine neue Referenzkopie lässt sich so festlegen:

```sh
git init .codex-tmp/paper-1.21.8-reference
git -C .codex-tmp/paper-1.21.8-reference remote add origin https://github.com/PaperMC/Paper.git
git -C .codex-tmp/paper-1.21.8-reference fetch --depth 1 origin 29c8822d90899c89d2689338e81a98f690bcba12
git -C .codex-tmp/paper-1.21.8-reference checkout --detach FETCH_HEAD
git -C .codex-tmp/paper-1.21.8-reference rev-parse HEAD
```

Ein Versionswechsel benötigt neue Pins und eine erneute Prüfung der betroffenen
Verträge. Der bewegliche Branch `ver/1.21.8` allein reicht nicht als Nachweis.

## Welche Quellen was belegen

- **API-Quellen** belegen öffentliche Methoden, dokumentierte Verträge und
  tatsächlich enthaltene Implementierungen. Die Bridge verwendet echte
  `JavaPlugin`, `PluginCommand`, `YamlConfiguration`, `MemoryConfiguration`,
  `PluginDescriptionFile`, `HandlerList` und `RegisteredListener`. Diese sieben
  eingebetteten Klassen wurden bytegenau mit dem offiziellen lokalen
  API-Artefakt verglichen. `Server`, `Player`, `PluginManager` und weitere
  Serverfassaden sind dagegen FerrumCs begrenzte Adapter.
- **CraftBukkit-/Paper-Serverquellen und Patches** zeigen die wirkliche
  Eventeinbindung, Inventarverarbeitung, Scheduler und Konfigurationsabhängigkeiten.
  `patches/sources/` und `patches/features/` müssen zusammen gelesen werden.
- **Die vollständige Vanilla-Basis samt angewandten Patches** ist für unveränderte
  Mechanik nötig. Ein Patch enthält nur Änderungen und Kontext. Papers
  [Build-Anleitung](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/README.md)
  beschreibt `./gradlew applyPatches` mit Java 21 zum Erzeugen der Quellen.
  `applyPatches` wurde am 10.10.2026 mit Java 21 erfolgreich ausgeführt;
  vollständige Basis und beide Patchschichten liegen lokal zur Prüfung vor.
  Das ist kein Laufzeitvergleich mit einem gestarteten Paper-Server.
- **Protokoll-Fixtures und Laufzeitbeobachtung** prüfen die sichtbaren Folgen
  unabhängig von FerrumCs eigener Implementierung. Prismarine-Fixtures belegen
  Wire-Formate; sie belegen keine Mining-, Pickup- oder Tick-Regeln.

## Tatsächlich gelesene Serverreferenzen und offene Abnahme

Alle Links in dieser Tabelle sind auf denselben Server-Commit festgelegt.
Die Befunde beschreiben Paper, keine neuen FerrumC-Funktionen.

| Planbereich | Gelesene Originalquelle | Konkreter Befund und nötige Prüfung |
| --- | --- | --- |
| V01: Drop | [LivingEntity-Patch](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/patches/sources/net/minecraft/world/entity/LivingEntity.java.patch), `drop`, `createItemStackToDrop`; [ServerPlayer-Patch](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/patches/sources/net/minecraft/server/level/ServerPlayer.java.patch) | Consumer und Drop-Event laufen vor World-Add. Cancellation versucht Rückgabe in Mainhand bzw. Inventar; der Fallback beweist keine allgemeine Conservation bei beliebigen Pluginänderungen. Der gelesene LivingEntity-Konstruktorweg setzt Pickup-Delay 40; andere Drop-Herkünfte brauchen eigene Prüfung. Statistiken verwenden den tatsächlich erzeugten Stack. |
| V01: Eingaben/Kapazität | [ServerGamePacketListenerImpl-Patch](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/patches/sources/net/minecraft/server/network/ServerGamePacketListenerImpl.java.patch), `handlePlayerAction`; [Inventory-Patch](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/patches/sources/net/minecraft/world/entity/player/Inventory.java.patch), `canHold` | Einzel-/Ganzstack-Drop sind getrennt; Einzel-Drop hat ein zusätzliches tickbasiertes Spamlimit. GUI-Drop läuft durch Container-Events. Kapazitätsprüfung berücksichtigt Komponenten, Stacklimits und einen bereits belegten passenden Offhandstack. Numerische Wire-Statuswerte brauchen einen unabhängigen Protokollbeleg. |
| V01: Pickup | [ItemEntity-Patch](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/patches/sources/net/minecraft/world/entity/item/ItemEntity.java.patch), `playerTouch`; [CraftItem](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/src/main/java/org/bukkit/craftbukkit/entity/CraftItem.java) | Teilaufnahme verwendet `canHold`/`remaining`. Attempt-, Legacy- und Entity-Pickup-Events haben eine Reihenfolge; Cancellation stellt den Count wieder her. Plugins können den Stack ersetzen und Pickup-Animation beeinflussen. Owner-Prüfung und tatsächliche Einlagerung zusätzlich in vollständiger Basis und Laufzeit prüfen. |
| V01: Merge/Despawn | [ItemEntity-Patch](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/patches/sources/net/minecraft/world/entity/item/ItemEntity.java.patch), `merge`, `tick`, `setItem`; [WorldConfiguration](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/src/main/java/io/papermc/paper/configuration/WorldConfiguration.java) | Merge ist cancellable; Ziel übernimmt größeres Pickup-Delay und kleineres Alter. Radius, Wandprüfung und Despawn-Zeit sind konfigurationsabhängig. Abgebrochenes Despawn setzt Alter auf 0. Vanilla-Konstanten allein reichen nicht als Paper-Vertrag. |
| V01/W09: aktive und inaktive Entities | [Entity Activation Range Feature-Patch](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/patches/features/0003-Entity-Activation-Range-2.0.patch), `ItemEntity.inactiveTick`, `ActivationRange`, `ServerLevel` | Auch activation-inaktive Items altern und verringern Pickup-Delay; Sentinel-Werte sind ausgenommen. Das unterscheidet sich von ausgeladenen Chunks oder Offline-Zeit. Eine pauschale Pause aller inaktiven Items wäre keine belegte Parität. Chunk-Tick-/Unload-/Restart-Semantik bleibt separat zu prüfen. |
| I03/V03/J05/J06: Klicks | [ServerGamePacketListenerImpl-Patch](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/patches/sources/net/minecraft/server/network/ServerGamePacketListenerImpl.java.patch), `handleContainerClick` | Container-ID und Gültigkeit, Spectator, Slot-Rechte, Komponenten und Stacklimits beeinflussen die Aktion. Nach dem Event wird die Containeridentität erneut geprüft; `DENY` verhindert die Standardaktion. State-ID-Abweichung ist im gelesenen Abschnitt kein unmittelbarer DENY-Guard. FerrumCs derzeitige strikte Ablehnung veralteter IDs ist deshalb nicht als gleiche Paper-Semantik belegt. |
| V03/J05/J06: Drag/Open/Close | [AbstractContainerMenu-Patch](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/patches/sources/net/minecraft/world/inventory/AbstractContainerMenu.java.patch); [CraftEventFactory](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/src/main/java/org/bukkit/craftbukkit/event/CraftEventFactory.java), `callInventoryOpenEvent` | Drag bereitet Slots vor und reduziert den Cursor vor dem Event als Schutz beim Schließen durch ein Plugin; DENY stellt den Cursor wieder her. Open kann das vorherige Fenster schließen und transferiert bei Cancellation zurück. Cancellation nimmt nicht beliebige direkte Pluginänderungen zurück. Reentrancy, Screenwechsel und Resync gehören zur Abnahme. |
| J03/J07: Scheduler/Lifecycle | [CraftScheduler](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/src/main/java/org/bukkit/craftbukkit/scheduler/CraftScheduler.java), `mainThreadHeartbeat`, `handle`, `cancelTasks`; [PaperPluginInstanceManager](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/src/main/java/io/papermc/paper/plugin/manager/PaperPluginInstanceManager.java), `disablePlugin` | Ausführung und Wiederholung folgen Ticks; gleiche Fälligkeit verwendet FIFO. Disable beendet Plugin-Tasks. FerrumC muss diesen beobachtbaren Vertrag mit seiner autoritativen Rust-Simulation verbinden; eine separate Java-Wanduhr reicht nicht. Scheduler und vollständiger Thread-Vertrag fehlen derzeit. |

Für D0 wurden zusätzlich die vollständig erzeugten `Items`,
`StandingAndWallBlockItem`, `BaseTorchBlock`, `WallTorchBlock`, `FenceBlock`,
`FenceGateBlock` und `Block` sowie die Fence-Tags gelesen. `Items` registriert
jede der drei Torch-Familien mit stehender Basis, Wandvariante und
Attachment-Richtung `DOWN`. `StandingAndWallBlockItem.registerBlocks` trägt
beide Blöcke für dasselbe Item ein: Ein Reverse-Mapping kann deshalb die
Wandvariante behalten, ohne damit eine gültige Placement-Basis zu bestimmen.
Die vollständige Placement-Methode prüft Blickrichtungsreihenfolge,
`canSurvive` und Unobstructed/`BlockCanBuildEvent`. FerrumCs erste Korrektur
behebt ausschließlich die Item-Basis und die vorhandenen Boden-/Wandfälle;
Support, Fallback, Nachbarphysik und dieses Java-Event sind damit nicht umgesetzt.
Die bestandene FerrumC-Abnahme steht im [D0-Prüfbericht](D0_PLACEMENT_VALIDATION.md).

Die Fence-Folgeschritte verwenden die eingebauten `FENCES`/`WOODEN_FENCES`-
Familien, die expliziten `FenceGateBlock`-Registrierungen und die 33 Ausnahmen
aus `Block.isExceptionForConnection`. Die reine Connection-Formel und die
beiden Sim-Consumer sind deterministisch geprüft; tatsächliche FULL-Flächen
gewöhnlicher Blockstates bleiben offen. Die gepinnten vier Tagfixtures samt
Hashes liegen in `server/crates/ferrumc-registry/data/connection-tags/`.

Für die native-only Operator-Erweiterung wurden außerdem die vollständigen
`OpCommand`, `DeOpCommands`, `Commands.hasPermission`,
`CommandSourceStack.hasPermission`, `PlayerList.op/deop/sendPlayerPermissionLevel`
und `CraftPlayer.isOp/setOp` mit relevanten Source-/Feature-Patches gelesen.
Paper verbindet nominal Level 3 mit Bukkit-Checks, sendet Permission-
EntityEvents und recalculiert Bukkit-Permissions/Commands; Deop überprüft
Whitelist-Zugang erneut. FerrumCs strengere Offline-Identitätsprüfung,
eigene Recordform und fehlende Zweige sind im
[Operator-Prüfbericht](D0_OPERATOR_VALIDATION.md) benannt. Der getrennte
Java-`player_state`-RPC aktualisiert retained Proxies; seine aktive Rust-
Mutationseinbindung bleibt gesperrt. Auch diese Quellenlektüre und Tests
belegen keinen laufenden Paper-Vergleich.

Dies ist eine erste Quellenprüfung, keine vollständige Serverprüfung und keine
bestandene Paper-Differenzialabnahme. Auch die bestehenden I03-Klicks haben
noch keine solche Abnahme. Mining/Loot, Rezepte, Kampf, Hunger, Mobs,
Weltgenerierung, Licht, Fluids, Redstone, übrige Block-Entities, Permissions,
weitere Events und NMS bleiben anhand ihrer Planzeilen offen. Weitere
unveränderte Basiszweige und Itemphysik müssen vor V01-Aktivierung zusätzlich
gelesen und getestet werden.

## Pflicht vor jeder betroffenen Implementierung

Für jeden Survival- oder Java-API-Schritt wird im Implementierungsbrief und
anschließend im Prüfbericht folgende Matrix geführt:

| Eintrag | Erforderlicher Inhalt |
| --- | --- |
| Version und Quelle | Server-/API-Pin, Datei und Symbol; API, Basiscode, Source-Patch und Feature-Patch unterscheiden. Relevante Callees und Konfiguration mitlesen. |
| Verhalten | Trigger, Voraussetzungen, normale Wirkung, Grenzfälle, Tick-/Thread-Reihenfolge und sichtbare Pakete/Rückgaben. |
| Plugins | Events vor/nach Mutation, Reihenfolge, Cancellation, veränderte Daten, Reentrancy und Plugin-Disable; fehlende Bridge-Methoden ausdrücklich nennen. |
| Ownership und Fehler | Item-Erhaltung, gleichzeitige Aktionen, volle Queues, verlorene Antworten, Disconnect, Crash/Restart und Chunk-Lebensdauer. Paper allein beweist FerrumCs eigene Storage-Transaktionen nicht. |
| Abweichung | Bewusste Abweichungen und noch nicht belegte Zweige markieren; deren Auswirkung auf Vanilla-Verhalten und Plugins benennen. Ressourcenlimits dürfen nicht stillschweigend als Parität gelten. |
| Abnahme | Deterministische Contract-Tests und unabhängige Client-/Plugin-Fixtures; tatsächliche Paper-Vergleichsläufe mit Versions-/Konfigurations-/Plugin-Pins und aufgezeichneten Ergebnissen. Geplante und bestandene Fälle getrennt führen. |

Für die Vergleichsläufe werden dieselben Aktionen auf Paper und FerrumC
wiedergegeben und Itemmengen/-komponenten, Inventare, Eventprotokolle und
Clientzustände verglichen. Normalisierung darf nur transportbedingte Unterschiede
wie neu vergebene Entity-IDs entfernen. Reihenfolge, Cancellation und
Komponenten dürfen nicht durch Normalisierung verschwinden. Paper-Defaults,
Gamerules und relevante Konfiguration gehören zur Fixture. GUI-Darstellung
braucht zusätzlich echte Client-Abnahme. Lastmessungen folgen dem bestehenden
Performancevertrag bei gleichwertigem Funktionsumfang.

Quellenlektüre schützt nicht allein vor Auslassungen. Jede Planzeile bleibt
offen, bis ihr definierter Umfang, Fehlerpfade und unabhängige Abnahme erfüllt
sind. Für bestehende Funktionen wird fehlende Referenzabnahme nachgetragen;
eine bestandene FerrumC-Fixture wird nicht nachträglich als Paper-Vergleich
umgedeutet.

## Umgang mit Referenzcode

Paper erbt laut [Lizenzhinweis](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/LICENSE.md)
GPLv3, mit MIT-Freigaben genannter Mitwirkender. Die Rust-Umsetzung wird eigenständig
anhand dokumentierter Verhaltensverträge und Tests entwickelt; Paper-/Vanilla-
Implementierungscode wird nicht pauschal in den MIT-Rust-Kern übernommen.
Die tatsächlich eingebetteten API-Klassen behalten ihre Quellen und
[Distributionshinweise](../server/java-bridge/README.md#dependency-licensing-and-sources).
