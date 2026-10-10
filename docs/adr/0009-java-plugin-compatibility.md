# ADR-0009: Java-Plugins und Paper-Verhalten

**Status:** Angenommen — Rust-Simulation mit Java-Sidecar
**Datum:** 10.10.2026
**Geltungsbereich:** Hosting-Fork `jstrausd/ferrumc`, Arbeitsbranch `main`

## Angefordertes Ziel

Der Fork soll als normaler Minecraft-Server mit den üblichen Funktionen und
unveränderten Bukkit-/Paper-Java-Plugins nutzbar werden. Das erweitert die alte
Creative-/Minigame-Scope ausdrücklich. Der derzeitige Rust-Server erfüllt
dieses Ziel noch nicht; seine native C-ABI ist keine Java-Plugin-API.

Ein JAR ist ein Archiv mit Java-Bytecode. Ein Dateiscan, ein `ClassLoader` oder
eine gestartete JVM belegt deshalb keine funktionierende Plugin-Kompatibilität.
Das Plugin benötigt zusätzlich die API und deren Serververhalten.

## Zwei mögliche Architekturen

| Weg | Ergebnis und Konsequenz |
| --- | --- |
| Echter Paper-Betrieb | Ein eigener Paper-Java-Prozess liefert Spielmechanik und Plugin-API. Er verwendet Paper-Welten und ein eigenes `plugins/`-Verzeichnis. FerrumCs Rust-Simulation führt diese Plugins dabei nicht aus. |
| Java-Kompatibilität im Rust-Server | JVM im Prozess oder Java-Sidecar plus eine implementierte Bukkit-/Paper-Server-API. Aufrufe müssen konsistent in FerrumCs Welt-, Tick-, Entity- und Inventarmodell überführt werden. Zunächst lässt sich nur eine getestete Teilmenge anbieten. |

Der Betreiber hat ausdrücklich den Rust-Server mit neu entwickelter
Java-/Paper-Kompatibilität gewählt. Ein echter Paper-Server ersetzt FerrumC
nicht. Die erste Umsetzung verwendet einen Java-21-Sidecar, damit der
Rust-Kern kein JNI und keine zusätzliche Unsafe-Grenze benötigt.

## Erste Implementierungsstufe

`server/java-bridge/` verwendet die echte Paper-API für 1.21.8 und lädt
unveränderte klassische `plugin.yml`-JARs mit `JavaPlugin`. Die App besitzt den
Java-Prozess und bindet Lifecycle, deklarierte Commands sowie Join-/Quit-Events
an ihren bestehenden Laufzeitpfad an. Die Welt und Netzwerkverbindungen bleiben
in Rust; es läuft kein Paper-Server neben der Simulation.

Requests und Responses verwenden ein vier Byte langes Big-Endian-Längenfeld
und UTF-8-JSON mit maximal 65.536 Bytes pro Frame. Rust vergibt eindeutige IDs,
prüft Metadaten, Antworten, Nachrichten und Permissions und verwendet eine
begrenzte Prozesswarteschlange mit absoluten RPC-Fristen. Bei einem verlorenen
Protokollzustand darf der Sidecar nicht mit der nächsten Anfrage weiterarbeiten.
Plugin-stdout darf die Frames nicht verändern; Logs gehen nach stderr.

Die erste API-Teilmenge umfasst Plugin-Lifecycle, deklarierte Commands,
Spielernamen/UUID, serverseitige Rechte, Textnachrichten und Join-/Quit-Listener.
Der Rechte-Snapshot enthält erlaubte, in Commands deklarierte Nodes und die
tatsächliche Mitgliedschaft in Rusts konfigurierter `ops`-Liste. Bukkit-Defaults,
Children, Attachments und allgemeine Permission-Plugin-Integration sind noch
nicht implementiert. Native Grants ohne Command-Deklaration werden nicht als
vollständiger Bukkit-Rechtebaum übertragen. Root-/Alias-Suggestions verwenden
den Rust-Commandbaum; Plugin-TabCompleter werden noch nicht aufgerufen.
Nachrichten richten sich in dieser Stufe an den Spieler der aktuellen Anfrage
oder die Konsole. Broadcasts und Nachrichten an andere Spieler fehlen;
veränderte Join-/Quit-Broadcasttexte melden deshalb einen konkreten Fehler.
World-, Inventory-, Scheduler-, NMS- und weitere Paper-APIs bleiben eigenständige
Implementierungsschritte. Nicht implementierte Aufrufe werfen konkrete Fehler.
Eine neue Loader-/Command-Fixture wird nicht als breite Plugin-Abnahme ausgegeben.

Java-Plugins sind vom Betreiber als ausführbarer Code zu behandeln. Der
separate Prozess verspricht keine Betriebssystem-Berechtigungsgrenze und kann
mit den Rechten des Serverbenutzers Dateien, Netzwerk und andere Prozesse nutzen.

## Verträge für einen Rust-/Java-Pfad

1. **JVM und Lifecycle:** passender Java-Bytecode, Plugin-Metadaten,
   Abhängigkeiten und Bibliotheken, ClassLoader, Load/Enable/Disable,
   Fehlerbehandlung, Services und Konfiguration.
2. **Bukkit-/Paper-Server:** echte `Server`-Implementierung hinter `Bukkit`,
   Commands, Permissions, Spieleridentität, Messaging und Player-API.
3. **Tick-/Thread-Vertrag:** synchrone APIs und Tasks auf dem autoritativen
   Spiel-Thread, tickbasierte Verzögerungen, definierte asynchrone Aufrufe.
   Eine wartende RPC-Anfrage darf keinen Simulationsthread gegenseitig blockieren.
4. **Events:** Prioritäten `LOWEST` bis `MONITOR`, `ignoreCancelled`, veränderliche
   Eventdaten und Cancellation vor der Aktion; Listener-Reihenfolge und
   Rückgaben müssen unabhängig geprüft werden.
5. **Welt und Spielmechanik:** World/Chunk/Block/Biome, Dimensionen,
   Generierung, Lighting, Redstone, Entities, Inventare, ItemMeta-Komponenten,
   Rezepte, Survival, Persistence und Transaktionen mit korrekten Rückgaben.
6. **NMS/CraftBukkit:** Plugins mit `getHandle()`, Reflection, internen
   Minecraft-Klassen oder Packet-Hooks brauchen konkrete Implementierungen
   und versionsabhängiges Mapping. Bukkit-Interfaces allein reichen nicht.
7. **Betrieb:** authentifizierte Anmeldung, Save/Stop, Backup/Restore,
   Plugin-Shutdown, Ressourcenlimits, Diagnose und reproduzierbare Artefakte.

Ein eigener Schritt verändert eine Produktions-Crate und hat Contract- und
Fehlerpfadtests. Cross-Lane-Arbeit wird in aufeinander aufbauende Schritte
geteilt. Eine JVM-Brücke ersetzt keine fehlende Spielmechanik.

## Versionen und Weltdaten

FerrumC spricht derzeit Java Edition **1.21.8 / Protokoll 772**.
Die offizielle Paper-API nennt für Paper 1.21.8 **Java 21**, Build 60 und
Supportstatus **UNSUPPORTED**, Supportende 14.11.2025.
Aktuelle Paper-Versionen ab 26.1 benötigen laut Paper-Dokumentation Java 25.
Ein produktiver Paper-Pfad muss deshalb eine unterstützte Serverversion
festlegen und seine Plugins auf genau dieser Version prüfen.

FerrumCs redb-Datenbank ist kein Paper-Weltverzeichnis. Ein Wechsel verwendet
getrennte Datenordner; eine Weltmigration benötigt einen geprüften Export- und
Importvertrag und darf die bestehenden Daten nicht überschreiben.

## Abnahme und Kompatibilitätsmatrix

Für jede unterstützte Kombination werden Serverversion, Java-Version,
Pluginname/-version und tatsächlich bestandene Funktionen festgehalten.
Eine normale Paper-Plugin-JAR muss ohne Neuübersetzung installiert, aktiviert,
benutzt und sauber deaktiviert werden können. Eine reine Loader-Fixture ist
dafür kein ausreichender Nachweis.

Die Abnahme umfasst unter anderem einen echten Client, Commands/Permissions,
Join-/Quit-/Chat-Events, Scheduler, Weltänderungen, Inventartransaktionen,
Survival, Neustart-Persistenz sowie mindestens ein unverändertes externes
Plugin mit festgehaltener Version. NMS-Plugins erhalten eigene versionsabhängige
Fälle. Nicht unterstützte Aufrufe werden konkret benannt.

Auch im echten Paper-Betrieb bleibt die ausgewählte Server-/Plugin-Version
entscheidend: veraltete NMS-Plugins, Forge-/Fabric-Mods und JARs für andere
APIs sind kein Beleg für eine unterstützte Paper-Kombination.

## Quellen

- [Paper: Funktionsweise von Plugins](https://docs.papermc.io/paper/dev/how-do-plugins-work/)
- [Paper: Scheduler](https://docs.papermc.io/paper/dev/scheduler/)
- [Paper: Entwicklung mit Server-Internals](https://docs.papermc.io/paper/dev/userdev/)
- [Paper: Java-Voraussetzungen](https://docs.papermc.io/paper/getting-started/)
- [Offizielle Versionsdaten für 1.21.8](https://fill.papermc.io/v3/projects/paper/versions/1.21.8)
- [JavaPlugin auf dem 1.21.8-Branch](https://github.com/PaperMC/Paper/blob/ver/1.21.8/paper-api/src/main/java/org/bukkit/plugin/java/JavaPlugin.java)
- [InventoryClickEvent-Vertrag](https://jd.papermc.io/paper/1.21.8/org/bukkit/event/inventory/InventoryClickEvent.html)
