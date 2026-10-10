# FerrumC v2 — Umsetzungsplan für den Hosting-Fork

Stand: 10.10.2026. Ausgangspunkt: upstream `rework/v2-skeleton`, Commit
`8315da2c407a66ef591d2ef9de158aa74e7f6bc4`. Arbeitsbranch:
`feat/v2-hosting-foundation`.

Dieser Plan basiert auf dem Quellcode und den vorhandenen Tests. Die älteren
[FEATURES.md](FEATURES.md), [ROADMAP.md](ROADMAP.md) und Teile des README sind
überholt: Spieler-Persistenz, Anvil-Import, Paketbudgets und dynamische Plugins
sind inzwischen tatsächlich verdrahtet. Sie werden nicht erneut implementiert.
Die unten genannten Testdateien belegen vorhandene Abdeckung; der aktuelle
Prüfbericht und verbleibende Fehler stehen in
[V2_VALIDATION.md](V2_VALIDATION.md).

## Ziel und Reihenfolge

Zuerst ein zuverlässig startbarer, administrierbarer Creative-/Minigame-Server
für **Minecraft Java 1.21.8, Protokoll 772**, einschließlich Pterodactyl.
Danach sichere öffentliche Anmeldung, brauchbare Plugin-Fassaden und mehr
Welt-/Inventarfunktionen. Survival und vollständige Vanilla-Parität sind ein
eigenes, wesentlich größeres Folgeprojekt.

Jeder Implementierungsschritt verändert eine Produktions-Crate; bei Bedarf
kommt `ferrumc-testkit` für Testinfrastruktur hinzu. Änderungen über mehrere
Schichten werden als aufeinander aufbauende Schritte umgesetzt. Generierte
Protokolldateien werden ausschließlich über den Generator geändert.

## Was bereits vorhanden ist

| Bereich | Quellcode und bestehende Abdeckung | Tatsächliche Grenze |
| --- | --- | --- |
| Offline-Anmeldung, Version, Spieleridentität | `app/src/connection/mod.rs`, `app/tests/login_boundaries.rs`, `canonical_identity.rs` | Keine Mojang-Anmeldung oder aktive Verschlüsselung. |
| Paketbudgets und Zugangskontrolle | `connection/serverbound_budget.rs`, `app/tests/ops_access.rs` | Gleichzeitige Verbindungen je IP begrenzt; zeitliche Verbindungsversuche noch nicht begrenzt. |
| Spielerzustand | `app/src/player_data.rs`, `app/tests/player_state_persistence.rs`, `server_teleport_persistence.rs` | Speichern bei Leave/Shutdown; Item-Komponenten werden nicht erhalten. |
| Welt-Persistenz | `app/src/storage_worker.rs`, `app/tests/leave_rejoin_persistence.rs`, `overlay_multi_section_persistence.rs` | Overlays, Journal, Retry und Flush-Barrieren vorhanden; Recovery-Verfahren bleibt zu prüfen. |
| Anvil-Import | `app/src/world.rs`, `ferrumc-anvil/tests/import.rs` | Blöcke, einfache Schilder, leere Truhen; kein Export, keine vollständige Weltkonvertierung. |
| Platzierung | `ferrumc-placement/src/lib.rs`, `app/tests/placement_rotation.rs` | Bereits Türen, Betten, Slab-Merge, Treppenecken, Verbindungen und Waterlogging; keine vollständige Blockphysik. |
| Truhen und Schilder | `app/src/window.rs`, `app/tests/signs.rs`, `block_entity_persistence.rs` | Truhen unterstützen einen Teil der Klickarten; Spielerfenster 0 synchronisiert bislang nur zurück. |
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
| H03 | P0 / offen | Reproduzierbare Toolchain-/CI-Basis festlegen; vorhandene Lintfehler einzeln beheben. | Format, Clippy mit `-D warnings`, Workspace-/Doc-Tests, Generator und Claim-Prüfung grün. Unabhängiger Client-Smoke wird CI-Gate. |
| H04 | P1 / offen | Fork-eigene CI-Branch-Regeln und Release-Pipeline für glibc Linux x86_64/aarch64. | Commit/Version im Artefakt, Prüfsummen, Image-Digest, native Plugin-Ziel dokumentiert; Egg verwendet veröffentlichtes, geprüftes Image. |
| H05 | P1 / offen | Backup-/Restore- und Upgrade-Verfahren. | Kopie nach sauberem Stop wird auf leerer Instanz wiederhergestellt; Welt, Spieler und unterstützte Plugin-Daten werden geprüft. Schemawechsel hat Test und Rückweg. |

Das lokale Egg ist eine Entwicklungsgrundlage. Die echte Panel-/Wings-Abnahme
und Veröffentlichung gehören zu H04; ein lokaler Container-Test ersetzt sie
nicht. Ein v1-Release darf nicht als v2-Artefakt installiert werden.

## M1 — Verbindungen und Zustandskorrektheit

| ID | Priorität / Status | Arbeit und Zuständigkeit | Abnahme |
| --- | --- | --- | --- |
| N01 | P0 / umgesetzt (Play) | In `ferrumc-app` vorhandene `ConnectionLiveness` für Play-Keep-alive verdrahtet; zuvor wurden Antworten in `connection/handlers.rs` ignoriert. | Genau eine offene ID; richtige Antwort akzeptiert, falsche/unangeforderte/wiederholte Antwort abgewiesen. Absolute Antwortfrist trotz Timer-/Outbound-Aktivität; abbrechbare Socket-/Chunk-Waits begrenzt. Tests mit angehaltener Tokio-Zeit plus echtem Socket. |
| N02 | P0 / offen | Absolute Handshake-/Login-/Configuration-/Teilframe-Fristen verdrahten (`ferrumc-app`; vorhandene Net-Policy wiederverwenden). | Slowloris, minimale Teilbytes und unbekannte Pakete verlängern keine Deadline; gültige Fortschritte und Netzwerk-Fragmentierung bleiben erlaubt. |
| N03 | P1 / offen | Teleport-IDs und ausstehende Bestätigung serverseitig verfolgen (`ferrumc-app`). | Fremde/veraltete Bestätigung akzeptiert keine neue Position; Timeout und `/tp` während offener Bestätigung definiert. |
| N04 | P1 / offen | Begrenzung neuer Verbindungsversuche je IP in `ferrumc-net`, dann separater App-Wiring-Schritt. | Bounded IP-Tabelle mit Ablauf, Burst-/Refill-Tests, genaue Ablehnungszähler; bestehendes Gleichzeitigkeit-Limit bleibt separat. |
| N05 | P1 / offen | Vorhandene strengere `PlayIngress`-/Bytebudget-Regeln in den App-Laufzeitpfad integrieren. | Große Frames, komprimierte Daten und unbekannte Pakete beanspruchen begrenzte Bytes/CPU; Paketanzahl-Budget bleibt zusätzlich aktiv. |
| N06 | P1 / offen | Fristen und einen sicheren Abbruchvertrag für zustandsändernde Handler-/Container-Waits definieren (`ferrumc-app`). | Verzögerte Simulation oder Speicherung blockiert eine Verbindung nicht unbegrenzt; Abbruch verliert oder dupliziert keine Items und lässt keine halb abgeschlossene Transaktion zurück. |
| I01 | P0 / offen | Cursor-Stack bei voller Spielertasche erhalten (`ferrumc-app`). In `play.rs` und `handlers.rs` werden Reste derzeit nur geloggt und verworfen. | Anzahl und Inhalt der Items bleiben bei Fenster-Schließen, Disconnect und Rejoin erhalten, auch bei voller Tasche. Kein Duplizieren durch wiederholtes Schließen. |
| I02 | P0 / offen | Truhen-Klick-State-ID und Fensterzustand prüfen (`ferrumc-app`). | Veraltete/falsche Fenster-ID oder State-ID mutiert nichts und synchronisiert den autoritativen Zustand zurück. Zwei Betrachter erhalten konsistente Versionen. |
| I03 | P1 / offen | Normale Links-/Rechtsklicks für Spielerfenster 0; danach Shift-, Zahlentasten-, Drag-/Split-Klicks in einzelnen Schritten (`ferrumc-app`). | Item-Erhaltung für jedes Klickmodell, manipulierte clientseitige Änderungen zurückweisen, begrenzte Verarbeitung. |
| I04 | P1 / offen | Offene Truhenfenster aller Betrachter nach einer Mutation aktualisieren. | Zwei Spieler sehen dieselben Slots/Versionen ohne zusätzlichen Klick; konkurrierende alte Klicks werden zurückgewiesen. |
| S01 | P1 / offen | Periodische Spieler-Checkpoints (`ferrumc-app`). | Bounded/off-thread Speicherung mit definiertem maximalem Verlustfenster; Fehler werden sichtbar; alter Datensatz bleibt bei fehlgeschlagenem Laden unverändert. |
| S02 | P1 / offen | Item-Komponenten verlustfrei speichern: zuerst Welt-Codec (`ferrumc-world`), dann Spielerdaten (`ferrumc-app`). | Namen, Verzauberungen und unterstützte Komponenten bleiben nach Restart identisch; Schema-/Längen-/Malformed-Tests und Migration. |
| S03 | P1 / offen | Journal-Recovery-Audit und Wiederanlaufpfad (`ferrumc-storage`, danach App-Wiring separat). | Commit-/Overlay-/Journal-Cutpoints deterministisch prüfen, Recovery mehrfach gleiches Ergebnis; keine unbewiesene Crash-Durability-Garantie. |

M1 ist fertig, wenn diese P0-Lücken geschlossen sind, die Regressionen grün
sind und der unabhängige Client Join, Bauen, Rejoin und Restart bestätigt.
N01 unterbricht nur nachweislich abbrechbare Socket-/Chunk-Waits. Zustandsändernde
Handler und Containertransaktionen dürfen zunächst fertiglaufen, damit die
Inventar-Persistenz konsistent bleibt; ihre begrenzte Abwicklung ist N06.

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
Für diesen Plan gilt [ADR-0008](adr/0008-trusted-native-plugins-c-abi.md):
zielabhängige C-ABI-Artefakte, kein Hot-Reload und kein zusätzlicher WASM-Runtime.
Bukkit-/Spigot-/Paper-Binärplugins gehören nicht zu dieser API.

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

W06–W09 sind bewusst spätere Arbeiten; sie ändern die heutige CreativeCore-Scope
und dürfen nicht nebenbei in die Hosting-Grundlage aufgenommen werden.

## M5 — Skalierung erst nach Messung

1. Referenzmessungen: Startup, Join-Sturm, Bewegung, aktive Builder, Plugin-Hooks,
   Chunk-Import und langsamer Storage. Hardware, Commit, Config, TPS/Tick-p95/p99,
   RAM und Queue-Tiefen festhalten; keine Spielerzahl versprechen ohne Messung.
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

## M6 — Survival / weitere Vanilla-Parität

Diese Bereiche sind überwiegend noch nicht vorhanden und setzen M1, Welt-Ticks,
Item-Komponenten und ein belastbares Entity-/Inventory-Modell voraus:

- Survival-Inventar vollständig, Crafting/Rezepte, Furnace/Smelting, Hopper und
  weitere Container; Transaktionen erhalten Items und erlauben keine Dupes.
- Item-Entities, Pickup, Despawn, Projektile, Kollision und Entity-Metadaten.
- Gesundheit, Schaden, Angriff/Knockback, Hunger, Nahrung, Erfahrung,
  Tod/Respawn und serverseitig durchgesetzte Bewegungs-/Flugfähigkeiten.
- Mob-Spawn, KI, Navigation und Loot; anschließend weitere Entity-Typen.
- Redstone, Tick-Reihenfolge, Kolben und komplexe Blockphysik.
- Vollständige Biome/Features/Caves/Structures und Licht-Parität, falls das
  Produktziel später Vanilla-Parität wird.
- Signed Chat/Chat-Sessions, Resource-Pack-Antwortpfad, Skins/Profil-Properties,
  Maps und restlicher Command-/Packet-Surface für genau Protokoll 772.

Für jedes Thema wird vor Implementierung ein eigener Umfang mit unabhängigen
Client-/Welt-Fixtures definiert. Ein großer Sammel-PR wäre weder prüfbar noch
mit den Crate-Regeln vereinbar.

## Abnahme eines Arbeitsschritts

1. Invarianten und Abhängigkeiten der betroffenen Crate lesen; Verhalten und
   Fehlerpfade vor der Änderung benennen.
2. Deterministische Tests, bei Parsern auch fehlerhafte/maximale Eingaben.
   Uhren kontrollieren statt in Unit-Tests zu schlafen.
3. Betroffene Tests, Format und Clippy; danach die vorgeschriebenen Workspace-,
   Doc-, Generator- und Claim-Prüfungen. Bestehende Fehler getrennt festhalten.
4. Bei Protokolländerungen unabhängiges `tests/blackbox`-Smoke; echte Vanilla-
   Client-Abnahme zusätzlich dokumentieren, nicht aus Fake-Client-Tests ableiten.
5. Dokumentation und Statuszeile aktualisieren; konventioneller Commit,
   kleiner Review-Schritt. Erst geprüfte Artefakte als Release veröffentlichen.
