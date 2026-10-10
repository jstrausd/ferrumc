# FerrumC v2 — Umsetzungsplan für den Hosting-Fork

Stand: 10.10.2026. Ausgangspunkt: upstream `rework/v2-skeleton`, Commit
`8315da2c407a66ef591d2ef9de158aa74e7f6bc4`. Einziger Arbeitsbranch:
`main`. Der erste Hosting-PR wurde integriert; die anderen Fork-Branches
wurden nach Sicherung entfernt.

Dieser Plan basiert auf dem Quellcode und den vorhandenen Tests. Die älteren
[FEATURES.md](FEATURES.md) und [ROADMAP.md](ROADMAP.md) sind
überholt: Spieler-Persistenz, Anvil-Import, Paketbudgets und dynamische Plugins
sind inzwischen tatsächlich verdrahtet. Sie werden nicht erneut implementiert.
Die unten genannten Testdateien belegen vorhandene Abdeckung; der aktuelle
Prüfbericht und verbleibende Fehler stehen in
[V2_VALIDATION.md](V2_VALIDATION.md).

Die aktuelle Reihenfolge für Spielbarkeit, die vom Betreiber gemeldeten Fehler
und die grobe Aufwandsschätzung stehen im
[Survival-Lieferplan](SURVIVAL_DELIVERY_PLAN.md). Seine Pakete D0–D10 priorisieren
die untenstehenden technischen Aufgaben; die bisherige Kapitelnummerierung
ist keine Vorgabe, erst Minigame-Plugins und später Survival zu implementieren.

## Ziel und Reihenfolge

Das Betreiberziel ist ein speicherschonender, performanter Rust-Spielserver
mit vollständigem normalem Survival und unveränderten Java-Plugins.
Die derzeit geprüfte Grundlage spricht **Minecraft Java 1.21.8, Protokoll 772**,
einschließlich lokal geprüfter Docker-/Pterodactyl-Artefakte. Öffentliche
Anmeldung, Welt-/Inventarfunktionen und Java-Fassaden werden auf dieser
Grundlage schrittweise erweitert. Dafür ist
der Rust-Kern mit Java-Sidecar gemäß
[ADR-0009](adr/0009-java-plugin-compatibility.md) ausgewählt; diese Funktionen
sind nicht durch den bisherigen Hosting-PR erledigt. Der Auftrag umfasst
die weitere Implementierung; neue Arbeit bleibt auf `main` und wird jeweils
mit Vertrag, Fehlerpfaden und unabhängiger Abnahme veröffentlicht.

Jeder Implementierungsschritt verändert eine Produktions-Crate; bei Bedarf
kommt `ferrumc-testkit` für Testinfrastruktur hinzu. Änderungen über mehrere
Schichten werden als aufeinander aufbauende Schritte umgesetzt. Generierte
Protokolldateien werden ausschließlich über den Generator geändert.

Für Survival und Java-/Paper-Verhalten gilt die versionsfeste
[Paper-Implementierungsreferenz](PAPER_REFERENCE.md): echte API-Verträge,
Serverquellen, beide Patchschichten und relevante Vanilla-Basis prüfen;
Events, Grenzfälle und bewusste Abweichungen pro Feature festhalten.
Die erste Quellenprüfung verwendet Paper 1.21.8 Build 60. Vollständige
Basisquellenprüfung und Laufzeitvergleiche werden featureweise nachgeholt;
sie sind noch keine bestandene Paritätsabnahme.

## Was bereits vorhanden ist

| Bereich | Quellcode und bestehende Abdeckung | Tatsächliche Grenze |
| --- | --- | --- |
| Offline-Anmeldung, Version, Spieleridentität | `app/src/connection/mod.rs`, `app/tests/login_boundaries.rs`, `canonical_identity.rs` | Keine Mojang-Anmeldung oder aktive Verschlüsselung. |
| Paketbudgets und Zugangskontrolle | `connection/serverbound_budget.rs`, `app/tests/ops_access.rs` | Gleichzeitige Verbindungen je IP begrenzt; zeitliche Verbindungsversuche noch nicht begrenzt. |
| Spielerzustand | `app/src/player_data.rs`, `player_data/codec.rs`, `app/tests/player_component_persistence.rs`, `server_teleport_persistence.rs` | Schema 3 erhält alle 46 Slots plus Cursor einschließlich unterstützter Komponenten; Speichern bei Leave/Shutdown, periodische Checkpoints bleiben S01. |
| Welt-Persistenz | `app/src/storage_worker.rs`, `ferrumc-sim/src/loaded.rs`, `ferrumc-storage/src/record.rs`, `app/tests/leave_rejoin_persistence.rs` | Produktive Checked-Capture-/Apply-Pfade erhalten Komponenten; Journal, Retry und Flush-Barrieren vorhanden. Crash-Recovery über mehrere Records bleibt S03; experimenteller Scheduler verwendet noch Legacy-Capture. |
| Anvil-Import | `app/src/world.rs`, `ferrumc-anvil/tests/import.rs` | Blöcke, einfache Schilder, leere Truhen; kein Export, keine vollständige Weltkonvertierung. |
| Platzierung | `ferrumc-placement/src/lib.rs`, `app/tests/placement_rotation.rs` | Bereits Türen, Betten, Slab-Merge, Treppenecken, Verbindungen und Waterlogging; keine vollständige Blockphysik. |
| Truhen und Schilder | `app/src/window.rs`, `app/tests/signs.rs`, `block_entity_persistence.rs` | Truhen unterstützen Linksklicks; Spielerfenster 0 unterstützt normale Links-/Rechtsklicks. Rezepte, Shift-/Drag-/Zahlentasten-Klicks fehlen. |
| Multiplayer und Ausrüstung | `app/tests/multiplayer_visibility.rs`, `app/src/inventory.rs` | Spieler sichtbar, Rotation, Haupthand, Rüstung/Offhand vorhanden; übrige Entity-Systeme fehlen. |
| Befehle und Darstellung | `app/src/command.rs`, `ferrumc-sim/src/shard.rs` | Unter anderem `/tp`, `/time`, `/weather`, `/fill`, `/replace`, `/undo`, Titel, Sound, Partikel, Scoreboard, Teams, Bossbar vorhanden; kein kompletter Vanilla-Befehlssatz. |
| Trusted native plugins | `app/src/plugins.rs`, `app/tests/trusted_native_plugin.rs`, `ferrumc-plugin-loader` | Versionierte C-ABI, Manifest-/Hash-/Zielprüfung, Events, Intents und Block-Veto vorhanden; weitere Fassaden nicht im App-Laufzeitpfad angeboten. |
| Shutdown und Beobachtung | `app/src/main.rs`, `app/src/server.rs`, `ferrumc-dashboard` | SIGINT/SIGTERM/SIGHUP flushen; lokales Dashboard vorhanden; kein Prometheus-Exporter oder Admin-Konsole. |
| Shard-Infrastruktur | `ferrumc-sim/src/scheduler.rs`, `app/src/server.rs` | App verwendet derzeit einen weltweiten Simulationsbesitzer; kein Nachweis einer produktiven parallelen Mehr-Shard-Laufzeit. |

## M0 — Bau- und Hosting-Grundlage

| ID | Priorität / Status | Arbeit | Abnahme |
| --- | --- | --- | --- |
| H01 | P0 / umgesetzt | Build-Anleitung, auditierter Plan, dokumentierte Prüfergebnisse. | Frischer Checkout wird aus `server/` mit `--locked` gebaut; Plan nennt vorhandene Funktionen und echte Lücken. |
| H02 | P0 / lokal geprüft | Mehrstufiges Container-Image, separates Installer-Image und importierbares Pterodactyl-Egg. | Nicht-root-Laufzeit, `/home/container`, konfigurierter Allocation-Port, persistente Daten, Ready-Erkennung, SIGINT/SIGTERM und Reinstall ohne Datenüberschreiben lokal geprüft; Installer ohne Entrypoint-/User-Override wie bei Wings gestartet. |
| H03 | P0 / lokal und auf GitHub geprüft (Basisstand) | Rust 1.99.0 fest gepinnt; vorhandene Lintfehler crateweise behoben; CI für `main` mit Java- und unabhängigen Client-Gates verdrahtet. | GitHub-Lauf [38046184757](https://github.com/jstrausd/ferrumc/actions/runs/38046184757) für `7333f404` vollständig bestanden. Inventarcursor- und Java-Gates sind verpflichtend; die abschließenden Gates der späteren N02/I03/S02-Arbeit werden im Prüfbericht separat festgehalten. |
| H04 | P1 / offen | Fork-eigene CI-Branch-Regeln und Release-Pipeline für glibc Linux x86_64/aarch64. | Commit/Version im Artefakt, Prüfsummen, Image-Digest, native Plugin-Ziel dokumentiert; Egg verwendet veröffentlichtes, geprüftes Image. |
| H05 | P1 / offen | Backup-/Restore- und Upgrade-Verfahren. | Kopie nach sauberem Stop wird auf leerer Instanz wiederhergestellt; Welt, Spieler und unterstützte Plugin-Daten werden geprüft. Schemawechsel hat Test und Rückweg. |

Das lokale Egg ist eine Entwicklungsgrundlage. Die echte Panel-/Wings-Abnahme
und Veröffentlichung gehören zu H04; ein lokaler Container-Test ersetzt sie
nicht. Ein v1-Release darf nicht als v2-Artefakt installiert werden.

## M1 — Verbindungen und Zustandskorrektheit

| ID | Priorität / Status | Arbeit und Zuständigkeit | Abnahme |
| --- | --- | --- | --- |
| N01 | P0 / umgesetzt (Play) | In `ferrumc-app` vorhandene `ConnectionLiveness` für Play-Keep-alive verdrahtet; zuvor wurden Antworten in `connection/handlers.rs` ignoriert. | Genau eine offene ID; richtige Antwort akzeptiert, falsche/unangeforderte/wiederholte Antwort abgewiesen. Absolute Antwortfrist trotz Timer-/Outbound-Aktivität; abbrechbare Socket-/Chunk-Waits begrenzt. Tests mit angehaltener Tokio-Zeit plus echtem Socket. |
| N02 | P0 / umgesetzt | Absolute Handshake-/Login-/Configuration-/Teilframe-Fristen (`ferrumc-app`; vorhandene Net-Policy wiederverwendet). | Nur semantischer Phasenwechsel erneuert das konfigurierte Fenster. First-Byte-Teilframefrist maximal 10 Sekunden; gepipelinete Reste behalten den Empfangszeitpunkt. Paused-Clock- und echte Socket-Abnahme prüfen Stalls, Unknown-ID-Flood, Shutdown und freie Verbindungskapazität. |
| N03 | P1 / offen | Teleport-IDs und ausstehende Bestätigung serverseitig verfolgen (`ferrumc-app`). | Fremde/veraltete Bestätigung akzeptiert keine neue Position; Timeout und `/tp` während offener Bestätigung definiert. |
| N04 | P1 / offen | Begrenzung neuer Verbindungsversuche je IP in `ferrumc-net`, dann separater App-Wiring-Schritt. | Bounded IP-Tabelle mit Ablauf, Burst-/Refill-Tests, genaue Ablehnungszähler; bestehendes Gleichzeitigkeit-Limit bleibt separat. |
| N05 | P1 / offen | Vorhandene strengere `PlayIngress`-/Bytebudget-Regeln in den App-Laufzeitpfad integrieren. | Große Frames, komprimierte Daten und unbekannte Pakete beanspruchen begrenzte Bytes/CPU; Paketanzahl-Budget bleibt zusätzlich aktiv. |
| N06 | P1 / offen | Fristen und einen sicheren Abbruchvertrag für zustandsändernde Handler-/Container-Waits definieren (`ferrumc-app`). | Verzögerte Simulation oder Speicherung blockiert eine Verbindung nicht unbegrenzt; Abbruch verliert oder dupliziert keine Items und lässt keine halb abgeschlossene Transaktion zurück. |
| I01 | P0 / umgesetzt und geprüft | `PlayerInventory` besitzt den einzigen Cursor; Fenster-Schließen und Disconnect behalten Einzahlungsreste. S02 erweitert die bisherigen ID-/Anzahl-Saves auf Schema 3 einschließlich Komponenten; JSON 1/2 migrieren ausdrücklich. UUID-Reserve vor dem Load bis nach Save/Leave-Ack; fehlgeschlagene Saves/unklare Freigaben sperren die UUID bis zum Neustart. | Unabhängiger Client bestätigt volle Tasche, falsche/veraltete IDs, Fensterwechsel, doppelte Anmeldung, Rejoin, zwei Neustarts und exakte Gesamtmenge am Basisstand. Spawn-Tickets werden nach tatsächlicher Erwerbsantwort erfasst. Die zusätzliche Komponenten-/Restart-Abnahme und aktuellen Gates sind unter S02 und im Prüfbericht dokumentiert. |
| I02 | P0 / umgesetzt | Truhen-Klick-State-ID und Fensterzustand prüfen (`ferrumc-app`); Chest-Snapshot im Driver atomar vergleichen. | Veraltete/falsche Fenster-ID oder State-ID mutiert nichts und synchronisiert Slots/Cursor zurück. Bei Konflikt bleibt der Cursor unverändert; echte Zwei-Client-Regressionsabnahme bestanden. Proaktive Viewer-Updates bleiben I04. |
| I03 | P1 / erste Stufe umgesetzt | Serverautoritative Links-/Rechtsklicks für Spielerfenster 0 inklusive Komponenten, Offhand und Cursor. | Unit-/Socket-/unabhängige Client-Abnahme prüft Split/Merge/Swap, State-ID und volle Tasche. Spectator verändert weder Spieler- noch Truhenslots. Crafting-Ergebnis bleibt read-only, Armor-Inserts abgewiesen; Shift-, Zahlentasten-, Drag-/Drop-Klicks, Rezepte und Equippable-/Binding-Regeln bleiben offen. |
| I04 | P1 / offen | Offene Truhenfenster aller Betrachter nach einer Mutation aktualisieren. | Zwei Spieler sehen dieselben Slots/Versionen ohne zusätzlichen Klick; konkurrierende alte Klicks werden zurückgewiesen. |
| S01 | P1 / offen | Periodische Spieler-Checkpoints (`ferrumc-app`). | Bounded/off-thread Speicherung mit definiertem maximalem Verlustfenster; Fehler werden sichtbar; alter Datensatz bleibt bei fehlgeschlagenem Laden unverändert. |
| S02 | P1 / umgesetzt und lokal geprüft | Eigenständiger bounded Item-Codec, versionierter Welt-Codec und produktive Checked-Verbraucher in Storage/Sim/App verdrahtet. Spieler-Schema 3 enthält 46 Slots plus Cursor mit vollständiger unterstützter Darstellung. | Schema-/Migration-/Malformed-/Grenzwerttests, Redb-Reopen und Socket-Restart-Fixtures prüfen Komponenten, opaque Payloads, Removals, NBT-Reihenfolge und Floatbits. Fehler behalten Dirty-/Journal-/Ticket-Verantwortung. Elternrecords und Queues begrenzen kodierte Bytes; Lokale Workspace-, Socket- und unabhängige Client-Abnahme bestanden; CI wird commitbezogen im Prüfbericht verlinkt. |
| S03 | P1 / offen | Journal-Recovery-Audit und Wiederanlaufpfad (`ferrumc-storage`, danach App-Wiring separat). | Commit-/Overlay-/Journal-Cutpoints deterministisch prüfen, Recovery mehrfach gleiches Ergebnis; keine unbewiesene Crash-Durability-Garantie. |

M1 ist fertig, wenn diese P0-Lücken geschlossen sind, die Regressionen grün
sind und der unabhängige Client Join, Bauen, Rejoin und Restart bestätigt.
N01 unterbricht nur nachweislich abbrechbare Socket-/Chunk-Waits. Zustandsändernde
Handler und Containertransaktionen dürfen zunächst fertiglaufen, damit die
Inventar-Persistenz konsistent bleibt; ihre begrenzte Abwicklung ist N06.

Der S02-Speichervertrag verwendet für Spieler ein `FCPS`-Schema-3-Envelope
mit genau 47 `FCIS`-Itemrecords (46 Slots plus Cursor) und höchstens 1 MiB
Gesamtpayload. JSON-Schemas 1/2 migrieren nur im Speicher mit ihrer bisherigen
Normalisierung; Laden schreibt nichts zurück. Alte Saves enthalten keine
Komponenten, die durch Migration wiedergewonnen werden könnten. Nur ein
bestätigter Storage-Miss erlaubt frische Spielerdaten; kaputte, zu große oder
unbekannte Datensätze werden abgewiesen.

Eine Block-Entity teilt höchstens 16 KiB auf ihre Inhalte auf. Alle Entities
eines Chunks teilen einen 2-MiB-Tail einschließlich Positionen und Längen;
ein vollständiger Chunk-/Overlayrecord hat höchstens 3 MiB. Der Driver
erfasst höchstens vier Overlays je Übergabe; die Queue besitzt acht Slots,
der Worker schreibt ab 8 MiB an wartenden kodierten Overlaybytes oder seinen
bestehenden Count-/Zeitgrenzen. Fehlerhafte späte Captures lassen die ganze
ausgewählte Dirty-Gruppe, Journal und ID-Zähler unverändert. Release-Barrieren
bestätigen alle Teilbatches vor dem letzten Ticket, Shutdown drainiert alle
verbliebenen Batches. Diese Grenzen beschreiben kodierte Bytes und keine
RSS-Obergrenze. Der experimentelle Scheduler verwendet weiterhin den
Legacy-Capture; Crash-Atomizität zwischen Spieler-, Overlay- und Journalrecords
ist S03 und gehört nicht zu diesem Implementierungsschritt.

## M2 — Öffentliche Anmeldung und Betrieb

Offline-Modus vertraut Spielernamen; dadurch sind auch `ops`, Bypass-Rechte und
gespeicherte Offline-UUIDs namensbasiert. Ein öffentliches Hosting-Angebot braucht
einen festgelegten Authentifizierungspfad, bevor diese Rechte belastbar sind.

| ID | Arbeit, in getrennten Crate-Schritten | Abnahme |
| --- | --- | --- |
| A01 | Encryption Request/Response und strikt begrenzte Decoder über `ferrumc-proto-gen`; generierte Dateien prüfen. | Unabhängige Wire-Fixtures, falsche Schlüssellängen, Truncation und falsche Tokens getestet. |
| A02 | RSA/AES-CFB8-Verbindungszustand in `ferrumc-net` nutzbar machen. | Encryption-Umschaltpunkt bytegenau, Fragmentierung, falscher Verify-Token und Crypto-Fehler getestet. |
| A03 | Mojang-Session-Prüfung mit Timeouts/Fehlerklassen; Konfiguration in `ferrumc-config`, App-Wiring separat. | Gültiger Account erreicht Play; abgelaufene/gefälschte Session wird abgewiesen. Session-ID/UUID/Profil übernehmen statt Offline-Identität. |
| A04 | Offline-zu-Online-Spielerdaten-Migration und Rechtezuordnung. | Keine Übernahme fremder Daten allein wegen eines Namens; dokumentierter, getesteter Betreiber-Migrationsschritt. |
| A05 | Optional modern Velocity forwarding mit Shared-Secret-/MAC-Prüfung. | Direkte Umgehung und falsche MAC abgewiesen; erst nach Identitätsmodell, kein blindes Vertrauen in IP/UUID. |
| O01 | Prometheus-Endpoint im Dashboard oder separater Exporter, mit festem Metriknamen-/Label-Satz. | Tickzeit, Spieler, Queues, Disconnect-Gründe, Storage-Retries und Plugin-Fehler scrape-bar; keine unbeschränkten Spieler-/IP-Labels. |
| O02 | Autorisierte Admin-Konsole mit Stop/Save/Status; RCON als separater Schritt. | Befehle durch bestehenden Command-Pfad, Berechtigungen und Begrenzungen; Stop wartet auf Flush. |
| O03 | Begrenzte Diagnostik, klare Startup-/Shutdown-/Storage-Fehler. | Keine Erfolgsmeldung bei fehlgeschlagenem Flush; Trace-/Log-Volumen bei Join-Sturm begrenzt. |
| O04 | Startup-Rollback: Task-Start erst nach falliblen Voraussetzungen oder jeden gestarteten Task bei fehlgeschlagenem Bind/Zugangskontrolle aufräumen (`ferrumc-app`). | Belegter Port/fehlerhafte Konfiguration hinterlassen keinen Driver/Storage-Worker oder offenen redb-Handle; sofortiger zweiter Start möglich. |

## M3 — Plugins für reale Minigames

Das heutige Modell heißt **trusted native plugins**. Es führt Bibliotheken im
Serverprozess aus. Manifest-Capabilities beschreiben angebotene Host-Fassaden.
Für den heutigen nativen Pfad gilt [ADR-0008](adr/0008-trusted-native-plugins-c-abi.md):
zielabhängige C-ABI-Artefakte, kein Hot-Reload und kein zusätzlicher WASM-Runtime.
Bukkit-/Spigot-/Paper-JARs gehören nicht zu dieser nativen API. Der neu
angeforderte Rust-/Java-Pfad wird separat gemäß
[ADR-0009](adr/0009-java-plugin-compatibility.md) implementiert und geprüft.

| ID | Arbeit | Abnahme |
| --- | --- | --- |
| P01 | Eingebaute Plugin-Befehle aus dem Host in den Live-Command-Baum integrieren (`ferrumc-app`). | Ausführung und Tab-Vervollständigung gehen durch dieselben Permission-Prüfungen; Namenskonflikte scheitern klar beim Start. |
| P02 | Native Command-/Permission-Fassaden schrittweise anbieten. | Neue Capability erst nach verdrahtetem App-Pfad; ABI-Major/Minor und alte Plugin-Fixtures prüfen. |
| P03 | Begrenzte autoritative World-Reads statt der heutigen Null-WorldView. | Snapshot-Lebensdauer und Shard-Kontext stimmen; kein DB-/Socket-/mutable Chunk-Zugriff. |
| P04 | Plugin-KV-Storage an langlebigen Backend anbinden. | Namespaces, Bytes-/Key-Limits, Fehlerpfad; Plugin-Daten überstehen Restart. |
| P05 | Player-/Chat-/Interaction-Veto und PlayerApi-Fassaden ausbauen. | Unterstützte Capability-Matrix entspricht Verhalten; Rejections resynchronisieren Client und haben deterministische Reihenfolge. |
| P06 | Tick-basierte, begrenzte Scheduler-Tasks verdrahten. | Kosten-/Tasklimits, deterministische Tick-Reihenfolge, deaktivierte Plugins erhalten keine weiteren Calls. |
| P07 | Beispiel-Minispiel und Plugin-Distribution. | Ein extern gebautes, hash-gepinntes Bundle funktioniert im Container: Join/Command, Countdown, Teleport, Scoreboard und Restart-Persistenz. |
| P08 | Explizite Disable-/Shutdown-Lifecycle für eingebaute Plugins verdrahten. | `on_disable` wird beim normalen Stop genau einmal ausgeführt; fehlgeschlagene/nie gestartete Plugins haben einen definierten Zustand. |

Jede ABI-Erweiterung braucht zuerst einen isolierten Contract-/Loader-/Host-Test
und anschließend einen eigenen App-Schritt. Kein einzelner PR über alle Crates.

## M4 — Bestehende Welten und belastbares Weltmodell

| ID | Arbeit | Abnahme |
| --- | --- | --- |
| W01 | Importvertrag schließen: unbekannte Blockstates/Block-Entities und Wiederimport vorhandener Welten ausdrücklich behandeln. | Keine unbemerkte Ersetzung wertvoller Daten; Testwelt mit importiertem Chunk, Änderung, Shutdown und erneutem Start bleibt konsistent. |
| W02 | Import mit Biomen und befüllten Truhen; Komponentenvertrag voraussetzen. | Abschnittsweise unabhängige NBT-Fixtures; Limits und nicht unterstützte Inhalte klar gemeldet. |
| W03 | `.mcc` und LZ4-Anvil-Kompression nach Bedarf hinzufügen. | Begrenzte Dekompression, sichere Dateipfade, Fragment-/Header-/Bomben-Tests. |
| W04 | Anvil-Export mit dokumentierter Inhaltsmatrix. | Native Daten → Export → unabhängiger Leser/Vanilla-Testwelt; nicht darstellbare Daten führen zu klarer Ablehnung. |
| W05 | Heightmap-Semantik korrigieren (`ferrumc-world`): `MOTION_BLOCKING` ist derzeit höchstes Nicht-Air. | Pflanzen, Flüssigkeiten und nicht blockierende Zustände unterscheiden sich korrekt von `WORLD_SURFACE`. |
| W06 | Echte Block-/Skylight-Berechnung als eigener späterer Meilenstein. | Lichtänderung nach Platzieren/Entfernen, Chunk-Grenzen, Import und Restart getestet; heutiges Full-Bright ersetzen. |
| W07 | Seed-/Generator-Version, zunächst einfacher deterministischer Terrain-Generator. | Unabhängig von Ladefolge identische Chunks, sauber versionierte Overlay-Basis. Vanilla-Terrain-Parität bleibt separat. |
| W08 | World Border, mehr Dimensionen, Dimension-Wechsel und weitere Block-Entity-Typen. | Eindeutige World/Dimension-IDs durch Routing und Speicherung; Grenz-/Wechsel-/Restart-Tests. |
| W09 | Block-/Random-Ticks, Fluids und Nachbarphysik in begrenzten Einzelaufgaben. | Reproduzierbare Reihenfolge, Arbeitsbudget, grenzüberschreitende Updates und Persistenz. |
| W10 | Weltzeit, Wetterdauer und Gamerules speichern. | `/time`-/`weather`-Zustand übersteht Restart; Tagesfortschritt und Pakete entsprechen der gespeicherten Basis. |

W06–W09 gehören zum ausdrücklich angeforderten Survival-Ziel. Sie folgen
den Inventar-, Komponenten- und Tick-Grundlagen in eigenen geprüften Schritten.

## Performancevertrag

Rust ist eine technische Grundlage, kein gemessener Vorteil gegenüber Paper.
Jede Messung nennt Commit/Binary-Hash, Release-Build, Hardware, Konfiguration,
Weltumfang, Clientanzahl und Arbeitslast. Native-only und Java-Bridge werden
getrennt und gemeinsam gemessen: RSS des Rust-Prozesses, RSS der JVM, gesamte
Prozessgruppe, CPU-Zeit, Startup/Join und Tickverteilung. Eine kleinere Heapgrenze
ist keine Messung des realen Speicherverbrauchs. Summiertes RSS kann gemeinsam
benutzte Seiten mehrfach zählen; die Messmethode muss dies ausweisen.

Ziel sind stabile 20 TPS im jeweils festgelegten Szenario, begrenzte Queues und
Cache-/Chunk-Lebensdauern sowie wenig Leerlaufarbeit. Der instrumentierte
Driverabschnitt ist kein vollständiger MSPT-Nachweis. Leistungsvergleiche sind
erst bei gleichem Funktionsumfang, Welt, Sichtweite, Plugins und Arbeitslast
belastbar. Fehlende Survival-Systeme dürfen nicht als Geschwindigkeitsgewinn
gezählt werden. Ausgangsmessungen sind noch keine Kapazitäts- oder
Produktionszusagen.

Java-APIs sollen gebündelte, begrenzte Datenübertragungen verwenden, wenn der
synchrone API-Vertrag dies erlaubt. Asynchrone Plugin-Aufgaben bekommen eigene
Arbeitsgrenzen; wiederholte Java/Rust-Kopien und ein RPC pro Block sind explizite
Messpunkte. Native-only Betrieb startet weiterhin keine JVM. Survival und
Kompatibilität werden zusammen mit den Ressourcenmessungen ausgebaut.

Die [Release-Ausgangsmessung vom 10.10.2026](../tests/performance/results/2026-10-10/README.md)
verwendet den eingefrorenen Basisstand `7333f404`, vor N02/I03/S02.
Bei acht stehenden Clients: Rust-RSS nativ 21,48–21,70 MiB,
Rust plus JVM mit Bukkit-Fixture 108,41–108,72 MiB; CPU der Serverprozesse
1,90–1,93 % eines logischen Kerns. Über die Messfenster lagen die effektiven
TPS bei 19,964–20,032. Es gab keine aktive Bewegung, Survival-Systeme oder
Plugin-Arbeitslast. Diese Werte sind eine reproduzierbare Regressionsbasis;
die aktuelle Implementierung und ein gleichwertiger Paper-/Survival-Workload
müssen getrennt gemessen werden.

## M5 — Skalierung erst nach Messung

1. Referenzmessungen: Startup, Join-Sturm, Bewegung, aktive Builder, Plugin-Hooks,
   Chunk-Import und langsamer Storage. Hardware, Commit, Config, TPS/Tick-p95/p99,
   RAM und Queue-Tiefen festhalten; keine Spielerzahl versprechen ohne Messung.
   Die stehende Release-Ausgangsmessung ist vorhanden; aktive Szenarien und
   Messungen der neuen S02-Verbraucher bleiben offen.
2. Bestehenden `TickScheduler` prüfen und Simulation auf den vorgesehenen eigenen
   Executor bringen. Heute wird eine weltweite Simulationsinstanz vom App-Driver
   bedient. Vor einer Mehr-Shard-Verdrahtung Inputs, Outputs, Storage und
   Plugin-Snapshot-Grenzen vollständig in diesem Executor abbilden.
3. Mehrere Shard-Besitzer in separaten Schritten integrieren; Entity-/Ticket-
   Transfer nur an Tickgrenzen. Grenzübertritte, Teleports, Nachbarsicht,
   gleichzeitig betroffene Edits und Shutdown deterministisch testen.
4. Chunk-Batch-Acknowledgements und klientgesteuerte Flusskontrolle implementieren.
   Kompression, Sendebudgets und Queue-Prioritäten am langsamen Client messen.
5. Langläufe mit vielen Chunks/Spielern, Backpressure und Storage-Fehlern.
   Kein Catch-up-Tick und kein unbegrenztes Wachstum. Distributed Multi-Server
   bleibt außerhalb dieses Plans.

## M6 — Survival / weitere Vanilla-Parität (angefordertes Ziel)

Diese Bereiche sind noch nicht vollständig vorhanden und setzen M1, Welt-Ticks,
Item-Komponenten und ein belastbares Entity-/Inventory-Modell voraus.
Jede Zeile wird vor der Umsetzung in einzelne Crate-Aufgaben geteilt.

| ID | Arbeit / Voraussetzung | Verhaltens- und Ressourcenabnahme |
| --- | --- | --- |
| V01 | Item-Entities, Drop/Pickup/Despawn, Kollision und Entity-Metadaten; nach S02. | Items bleiben über Inventar/Welt/Restart erhalten; gleichzeitiger Pickup zählt genau einmal. Entityzahl und Arbeitsmenge je Tick begrenzt, Heap je Entity gemessen. |
| V02 | Mining, Tools, Abbauzeiten, Haltbarkeit und Loot; nach V01. | Server bestimmt Werkzeug, Fortschritt und Drops; Abbruch, falsche Reichweite und wiederholter Abschluss erzeugen keine Items. |
| V03 | Rezeptregistrierung, Crafting-Ergebnis und sämtliche Inventar-Klickarten; nach I03/S02. | Ingredient-/Result-Mengen und Komponenten, Repeated-/Shift-Crafting, Recipe-Book sowie Abbruch/Close/Restart mit unabhängigen Rezept-Fixtures geprüft. |
| V04 | Gesundheit, Schaden, Angriff/Knockback, Tod/Respawn und Erfahrung. | Tick-/Damage-Reihenfolge, Rüstung, Immunitätsfenster, XP-/Inventarerhalt und Respawn mit echten Clients geprüft; kein unbegrenzter Entity-Scan je Treffer. |
| V05 | Hunger, Sättigung, Exhaustion, Essen, Regeneration und Effekte; nach V04. | Vanillanahe tickbasierte Abläufe, Item-Verbrauch, Tod und Persistenz. Kosten werden bei stehenden und aktiven Spielern getrennt gemessen. |
| V06 | Serverseitige Bewegungs-/Flugfähigkeiten, Flüssigkeits-/Fallverhalten und Projektile. | GameMode, Teleports, Kollision und Schaden verwenden denselben autoritativen Zustand; keine Client-Behauptung erzeugt Flugrechte oder Treffer. |
| V07 | Furnace/Smelting, Hopper und weitere Container; nach W09/V03. | Tickzeit, Brennstoff, Inputs/Outputs, Nachbartransfer und Restart konservieren Items; begrenzte aktive Containerlisten statt Scans aller geladenen Blöcke. |
| V08 | Mob-Spawn, KI, Navigation und Loot; nach V01/V04/W06/W07. | Arten-, Spawn-/Despawn- und Pfadbudget, Sichtbarkeit, Kampf und Restart; Ressourcen bei gleicher Mobzahl und aktivem Terrain messen. |
| V09 | Redstone, Tick-Reihenfolge, Kolben und komplexe Blockphysik; nach W09. | Kleine unabhängige Schaltungs-Fixtures, Chunk-Grenzen und Wiederanlauf; Update-Floods haben Arbeitsgrenzen und definierte Fortsetzung. |
| V10 | Vollständige Biome, Caves, Features, Structures und Licht-Parität; W06–W08. | Versionierter Seed-/Generatorvertrag, Dimensionen und unabhängige Welt-Fixtures; Generate/Load/Unload-RAM und CPU getrennt messen. |
| V11 | Signed Chat/Sessions, Resource-Packs, Skins/Profile, Maps und restlicher Command-/Packet-Surface. | Für Protokoll 772 eigene Client-/Authentifizierungs-Fixtures; unsupported Inhalte bleiben sichtbar statt stillschweigend zu verschwinden. |

Ein Survival-Meilenstein erfordert einen echten Ablauf von neuer Welt über
Ressourcenabbau, Crafting, Nahrung, Kampf und Tod/Respawn bis zum Neustart.
Seine RAM-/CPU-/Tickmessung nennt die aktive Welt, Chunks, Entities und Plugins;
ein bestandener Einzeltest ersetzt diesen vollständigen Ablauf nicht.

Für jedes Thema wird vor Implementierung ein eigener Umfang mit unabhängigen
Client-/Welt-Fixtures definiert. Ein großer Sammel-PR wäre weder prüfbar noch
mit den Crate-Regeln vereinbar.

## M7 — Java-/Paper-Kompatibilität im Rust-Server

Dieser Pfad wurde vom Betreiber ausdrücklich ausgewählt. Es läuft eine
Java-API-Brücke zur Rust-Simulation, kein Paper-Spielserver. M1–M6 bleiben
Voraussetzungen für APIs, deren Spielmechanik FerrumC noch nicht besitzt.

| ID | Status / Arbeit | Abnahme |
| --- | --- | --- |
| J01 | Umgesetzt und lokal geprüft: Java-21-Sidecar mit echter Paper-1.21.8-API, klassischem `plugin.yml`-Loader, Lifecycle/Konfiguration, Commands/Aliases/Namespaces und Join-/Quit-Listenern; Rust-App-Wiring. | Gewöhnliches, ohne FerrumC-API gebautes JAR aktiviert sich und verarbeitet Client-Commands/Events. Java-Prozess-, Rust-Socket- und unabhängige Zwei-Client-Abnahme bestanden; Rechte, UTF-8, Listener-Reihenfolge, Startup-Rollback, Disable-once und Restart geprüft. Frames/IDs/Metadaten/Queues/Fristen begrenzt; Prozessverlust, Cancel und fehlerhafte Antworten geprüft. |
| J02 | Teilweise umgesetzt: Plugin-TabCompleter mit realem `PluginCommand`-Fallback und `minecraft:ask_server`-Ankündigung. Bukkit-Permissions samt Defaults/Attachments und Plugin-Services bleiben offen. | Java-Prozesstests und unabhängiger Client prüfen Rechte, originale Alias-Schreibweise, leere Argumente, UTF-16-Bereiche, Unicode, Grenzen und Fehler-Recovery. Standard-Spielernamen-Fallback bleibt offen; Completion-Waits beachten Play-Frist und Shutdown. Permission-Plugins brauchen anschließend eigene versionsgenaue Abnahme. |
| J03 | Offen: Bukkit-Scheduler und synchroner API-Thread-Vertrag. | Tasks folgen autoritativen Rust-Ticks statt eigener Java-Wanduhr; Delay/Repeat/Cancel, Lag und Plugin-Disable geprüft. Bidir-RPC kann keine Java-/Simulations-Deadlocks erzeugen. |
| J04 | Offen: World-/Chunk-/Block-/Biome-Fassaden und Rückgaben. | Reads stammen aus autoritativem Zustand; synchrone Mutation hat definierte Commit-Grenze. Block-Edit-Plugins mit unabhängiger Welt-/Client-Abnahme, Chunk-Lebensdauer und Failure-Tests. |
| J05 | Offen: Player-/Entity-/Inventory-/ItemMeta-/Recipe-APIs. | Item-Erhaltung, Komponenten und Transaktionen stimmen mit M1/M6 überein; GUI-, Inventory- und Survival-Plugins geprüft. |
| J06 | Offen: übrige Bukkit-/Paper-Events, Mutationen und Cancellation. | Priority/ignoreCancelled und veränderte Ergebnisse wirken vor der Aktion. Block, Inventory, Damage, Movement, Chat und weitere Events jeweils mit echten Vorher-/Nachher-Belegen. |
| J07 | Offen: Paper-Bootstrap/Lifecycle, `paper-plugin.yml`, Library-/Dependency-Loader und Mapping. | Abhängigkeiten, Zyklen, geteilte Klassen, Library-Auflösung und Versionen reproduzierbar; Metadaten allein gelten nicht als erfolgreiche Aktivierung. |
| J08 | Offen: NMS-/CraftBukkit- und Packet-Verträge. | Konkrete Klassen/Handles und versionsabhängiges Remapping für ausgewählte externe Plugins; unsupported Aufrufe werden klar benannt. Kein universeller Nachweis aus Bukkit-Interfaces ableiten. |
| J09 | Offen: breite externe Plugin- und Gameplay-Matrix, Betrieb und Releases. | Unveränderte Plugins für die festgehaltene Serverversion, echte Client-/Survival-Abnahme, Neustart-/Backup-Tests und Langlauf. Jede noch fehlende API/Funktion bleibt sichtbar; erst dann normaler produktiver Serverumfang. |

J01 ist eine funktionale erste API-Teilmenge. Ein JAR-Loader mit bestandener
Fixture erledigt weder M1–M6 noch J02–J09. Siehe den API-Vertrag in
[ADR-0009](adr/0009-java-plugin-compatibility.md) und die genauen Grenzen in
[`server/java-bridge/README.md`](../server/java-bridge/README.md). Die erste
Rechteübertragung umfasst erlaubte Command-Nodes und konfigurierte Operators;
allgemeine Bukkit-Permissions, Standard-Spielernamen-Completion und Broadcasts
bleiben offen.

## Abnahme eines Arbeitsschritts

1. Invarianten und Abhängigkeiten der betroffenen Crate lesen; Verhalten und
   Fehlerpfade vor der Änderung benennen. Bei Survival-/Java-API-Arbeit zusätzlich
   die Quellen- und Verhaltensmatrix aus [PAPER_REFERENCE.md](PAPER_REFERENCE.md)
   führen; fehlende Basiszweige und Referenzabnahmen sichtbar lassen.
2. Deterministische Tests, bei Parsern auch fehlerhafte/maximale Eingaben.
   Uhren kontrollieren statt in Unit-Tests zu schlafen.
3. Betroffene Tests, Format und Clippy; danach die vorgeschriebenen Workspace-,
   Doc-, Generator- und Claim-Prüfungen. Bestehende Fehler getrennt festhalten.
4. Bei Protokolländerungen unabhängiges `tests/blackbox`-Smoke; echte Vanilla-
   Client-Abnahme zusätzlich dokumentieren, nicht aus Fake-Client-Tests ableiten.
5. Dokumentation und Statuszeile aktualisieren; konventioneller Commit,
   kleiner Review-Schritt. Erst geprüfte Artefakte als Release veröffentlichen.
