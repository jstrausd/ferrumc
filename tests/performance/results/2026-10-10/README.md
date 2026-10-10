# FerrumC Release-Prozessbasis — 10.10.2026

Bei acht stehenden Clients lag der gemessene Rust-Prozess ohne JVM bei
21,48–21,70 MiB residentem Speicher. Mit dem Java-Host und der Bukkit-Fixture
lagen Rust plus JVM bei 108,41–108,72 MiB. Beide Modi hielten im Test ungefähr
20 TPS. Diese Zahlen bilden eine lokale Ausgangsbasis für spätere Änderungen;
sie messen die derzeitige Teilmenge von FerrumC und keinen vollständigen
Survival- oder Paper-Workload.

Die [vollständigen Messdaten](results.json) enthalten alle 372 Prozess-/Dashboard-
Samples, Hardwaredaten, exakte Configs, Latenzen und Zusammenfassungen. SHA-256
der unveränderten Datei:
`e8ae443e59824c410299c5f702b81b37a2545de9fba46a3ee87cf74a81386b29`.

## Gemessene Software und Rechner

- Release-Buildrevision: `7333f404f2957cfd27345a82daa3d814982f6c5d`.
  Spätere Änderungen an `main` liefen nicht in diesen Prozessen.
- Eingefrorenes Docker-Image:
  `sha256:b11b4e3a7ee06c3348427c84e20e1ea2a9c08dc3c8ba9b84188ea27a43ec2b39`.
- Rust-Binary SHA-256:
  `e2d8137be016e3fa47b16a5bedbd05eb23100c5495e64f7defcf51d239f42f2b`.
- Java-Bridge SHA-256:
  `53e13cdf0e4e836f52f2717f4cc325482eae4c091cd5a43978b9e3de142c213b`.
- Unveränderte Standard-Bukkit-Fixture-JAR SHA-256:
  `d2debfa435550790eccf826217e0d6f0f7f81b9c81f087855b40995152841e91`.
- Runtime: Temurin Java `21.0.12.1+1-LTS`, Standard-JVM-Flags; kein Paper-Server.
  Unabhängiger Client: `minecraft-protocol` 1.66.2, Minecraft 1.21.8 / Protokoll 772.
- Intel Core i5-7500T @ 2,70 GHz, vier logische CPUs, 15,50 GiB nutzbarer RAM;
  Linux `7.2.4-3-cachyos`, CPU-Governor `powersave`, Frequenz nicht fixiert.
- Docker 29.8.2, Node 26.10.0, Python 3.14.7. Keine parallelen Builds oder anderen
  Server-Clienttests im Messfenster. Normale Anwendungen liefen weiter: mittlere
  Host-CPU-Auslastung je Fenster 9,97–13,86 % aller vier CPUs. Zu Beginn waren
  8,61 GiB RAM verfügbar; andere Prozesse belegten bereits 8,82 GiB Swap.
  Die gemessenen Rust-/JVM-Prozesse hatten durchgehend **0 MiB VmSwap**.

Gesamter Lauf: **11:14:11–11:22:30 UTC**, entsprechend 13:14:11–13:22:30 in Wien.
Runde 1: nativ, dann Java; Runde 2: Java, dann nativ. Jede der vier Instanzen
verwendete eine neue flache Welt und durchlief 0 → 4 → 8 Clients. Zwischen den
Instanzen wurden Dateisystem-/JRE-Caches nicht geleert.

## Konfiguration und Ablauf

Gemeinsam: 20 TPS, View-/Simulation-Distance 4, Spawn `[8, 64, 8]`, Spawn-Radius
1, Chunk-Stream-Intervall 50 ms, maximal 16 Verbindungen, kein Per-IP-Limit,
keine Paketkompression, redb-Persistenz und die gleichen eingebauten nativen
Plugins. Alle Clients standen am gleichen Spawn; jeder erhielt 81 Chunk-Spalten.
Das Dashboard war in beiden Modi aktiviert und blieb im Container an Loopback
gebunden. Der Sampler fragte es mit `nsenter` ab, ohne seinen Port freizugeben.

Der Java-Modus ergänzte ausschließlich den Java-Host und eine kopierte,
vorher eingefrorene Fixture-JAR. Sie führte Enable/Disable und Join-/Quit-
Listener aus. Während der Messfenster gab es keine aktiven Plugin-Commands,
Tasks, Weltänderungen oder Bewegungen.

Vor jedem Fenster: zehn Sekunden Stabilisierung nach vollständiger Client-
Anmeldung und View-Auslieferung. Dann 30 Sekunden Messung mit einem Sample
pro Sekunde plus Startsample, also 31 Samples je Fenster. CPU ist die gemessene
User-/Systemzeit der Rust-/JVM-Prozesse, geteilt durch tatsächliche Fensterdauer;
**100 % entsprechen einer logischen CPU**. Clients, Docker, `curl` und der
Sampler gehören nicht zum gemessenen Prozessbaum.

## Messwerte je Runde

RSS-Werte sind p50 innerhalb des jeweiligen Fensters, in MiB. Die CPU-Spalte
enthält Mittelwerte in Prozent, jeweils **Rust / JVM / Gesamt**. Die letzte
Spalte zeigt den vorhandenen Driver-Tick-Timer am Fensterende in Millisekunden,
jeweils **p50 / p95 / p99**.

| Runde | Modus | Clients | Rust RSS | JVM RSS | Gesamt RSS | CPU Rust / JVM / Gesamt | Effektive TPS | Driver p50 / p95 / p99 ms |
| --- | --- | ---: | ---: | ---: | ---: | --- | ---: | --- |
| 1 | Nativ | 0 | 10,51 | 0,00 | 10,51 | 0,23 / 0,00 / 0,23 | 20,032 | 0,004 / 0,005 / 0,006 |
| 1 | Nativ | 4 | 17,96 | 0,00 | 17,96 | 1,07 / 0,00 / 1,07 | 19,999 | 0,004 / 0,011 / 0,022 |
| 1 | Nativ | 8 | 21,70 | 0,00 | 21,70 | 1,90 / 0,00 / 1,90 | 19,966 | 0,005 / 0,009 / 0,021 |
| 1 | Java | 0 | 10,39 | 78,24 | 88,62 | 0,27 / 0,10 / 0,37 | 20,030 | 0,004 / 0,005 / 0,006 |
| 1 | Java | 4 | 22,64 | 85,95 | 108,59 | 1,07 / 0,07 / 1,13 | 20,030 | 0,005 / 0,007 / 0,022 |
| 1 | Java | 8 | 22,77 | 85,95 | 108,72 | 1,83 / 0,10 / 1,93 | 19,964 | 0,005 / 0,014 / 0,025 |
| 2 | Java | 0 | 10,25 | 78,14 | 88,39 | 0,23 / 0,10 / 0,33 | 19,999 | 0,004 / 0,005 / 0,012 |
| 2 | Java | 4 | 22,98 | 84,80 | 107,78 | 1,03 / 0,10 / 1,13 | 20,031 | 0,004 / 0,012 / 0,023 |
| 2 | Java | 8 | 23,61 | 84,80 | 108,41 | 1,83 / 0,07 / 1,90 | 19,997 | 0,004 / 0,016 / 0,026 |
| 2 | Nativ | 0 | 10,08 | 0,00 | 10,08 | 0,23 / 0,00 / 0,23 | 19,999 | 0,005 / 0,005 / 0,018 |
| 2 | Nativ | 4 | 19,04 | 0,00 | 19,04 | 1,23 / 0,00 / 1,23 | 19,966 | 0,004 / 0,008 / 0,023 |
| 2 | Nativ | 8 | 21,48 | 0,00 | 21,48 | 1,90 / 0,00 / 1,90 | 20,032 | 0,004 / 0,018 / 0,027 |

Die effektiven TPS werden aus Tickfortschritt und Fensterdauer abgeleitet.
Bei nur 30 Sekunden ergeben ein Tick mehr oder weniger und Snapshot-Abfrage-
zeitpunkte bereits ungefähr 0,033 TPS Abweichung von 20.

## Start und Anmeldung

Readiness wird von `docker run` bis zum beobachteten Bereitschaftslog gemessen;
Docker-/Polling-Overhead und warme Caches gehören dazu. Client-Latenzen laufen
von der Client-Erstellung bis zur ersten Position beziehungsweise zur kompletten
View und schließen Prismarine-Protokollinitialisierung, lokale Auswertung und
vier gleichzeitig erstellte Clients je Anmeldewelle ein. Sie sind keine
isolierten Messungen der Server-Anmelderoutine. p50/p95 verwenden Nearest Rank
über die 16 Anmeldungen je Modus.

| Modus | Readiness Runde 1 / 2 ms | Erste Position p50 / p95 ms | Komplette View p50 / p95 ms |
| --- | --- | --- | --- |
| Nativ | 598,68 / 596,97 | 655,30 / 1290,32 | 876,04 / 1467,44 |
| Java | 938,59 / 998,02 | 665,90 / 1298,94 | 948,79 / 1616,58 |

## Bedeutung und Grenzen

Der Test zeigt einen kleinen Rust-Grundbedarf und einen zusätzlichen JVM-
Grundbedarf für die tatsächlich implementierte Java-API-Teilmenge. RSS misst
residenten Prozessspeicher; die Summe kann gemeinsam genutzte Seiten mehrfach
zählen. Dateisystem-Cache, Docker-Overhead und Client-Speicher sind ausgeschlossen.
Peaks beim Start oder während der Anmeldung liegen außerhalb der stabilisierten
Messfenster. Die kleinen CPU-Werte unterliegen der `/proc`-Zeitauflösung und
normaler Streuung auf einem gemeinsam genutzten Rechner.

Die Tick-Perzentile erfassen den bestehenden Instrumentierungsabschnitt des
Drivers über die letzten 600 Ticks. Netzwerk-/Chunk-Encoding, Java-RPCs,
Storage und Snapshot-Arbeit außerhalb dieses Abschnitts werden damit nicht
vollständig erfasst. **Diese Werte sind keine vollständige Server-MSPT.**

Es wurden keine Mobs, Survival-Mechaniken, Redstone, normale Terrain-Generierung,
aktive Plugin-Last oder umfassende Paper-Kompatibilität gemessen. Der Test liefert
weder eine maximale Spielerzahl noch einen Leistungs- oder Speichervorteil
gegenüber Paper. Solche Aussagen benötigen später passende Funktionen, Welten,
Server-/Plugin-Versionen und aktive, reproduzierbare Workloads.

Alle vier Instanzen beendeten sich mit Exit-Code 0. Beide Java-Instanzen
deaktivierten die Fixture jeweils genau einmal. Alle erfassten Rust-/JVM-
Prozesse waren beendet, alle veröffentlichten Minecraft-Ports geschlossen;
keine Paket-Decoding-Fehler in den Messfenstern.

Reproduktion: [Harness und Anleitung](../../README.md). Die ursprünglichen
lokalen Rohartefakte einschließlich Logs und eingefrorener Dateien liegen unter
`/tmp/ferrumc-process-baseline-pzciuuf6`.
