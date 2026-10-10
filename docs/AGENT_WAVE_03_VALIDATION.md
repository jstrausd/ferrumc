# Dritte Agentenwelle: geprüfte Grundlagen für Generation und Entities

Stand: 10.10.2026. Ziel: Minecraft Java 1.21.8 / Protokoll 772 /
DataVersion 4440. Ausgangspunkt `347080f4`, geprüfte Runtimebasis `6998a76e`.
Spätere Dokumentationsänderungen präzisieren nur Umfang und Vergaben.
Root orchestriert Reviews, Tests und Git; Runtimecode stammt von den Agents.
Arbeitsbranch bleibt `main`.

GitHub-CI ist für den veröffentlichten Dokumentationsstand **`fb0e96e0`**
erfolgreich: [CI-Lauf 38072169222](https://github.com/jstrausd/ferrumc/actions/runs/38072169222).
Dieser Commit hat dieselbe Runtime wie die unten eingefrorene Basis.
Rust-/Java-/Originaldaten-/Generator-/Claims-Gates und alle sieben
Clientharnesses bestanden. Dieser Lauf belegt keine später integrierten
Registry-, Noise- oder Format-4-Appänderungen; deren Prüfung ist Welle4.

Die [Survival-Gesamtabnahme](SURVIVAL_END_TO_END_ACCEPTANCE.md) bleibt in
allen sechs Etappen offen. Diese Welle liefert keine natürliche Welt, keine
aktiven Item-/Mob-Entities und keinen Nether-/Endfortschritt. Die produktive
App verwendet an dieser geprüften Basis weiterhin Format 3 und die Flatwelt.
Die Aktivierung von Format 4 ist das getrennte laufende Ticket WG-004.

## Integrierte Teilverträge

| Ticket / Commit | Nachweis | Grenze |
| --- | --- | --- |
| GEN-04.1, `e754254f` | Legacy48-/Xoroshiro-RNG, positional factories, Improved-/PerlinNoise. 144 Worldtests, sieben Release-Vertragstests, 16.556 bitgenaue Original-JAR-Records. Rootreview CLEAN, unabhängiger Python-`-O`-Reexport von Fixture und Manifest bytegleich. | Explizite FerrumC-Koordinaten-/Octavecaps; keine Densityrouter oder Terrainaktivierung. |
| CONFIG-WG001, `402df480` | Optionale numerische i64-Seed-/geschlossene Presetdeklaration; fehlende Werte bleiben fehlend. Alle 30 Configtests auch unabhängig grün, Review CLEAN. | Der Producer wählt keinen Default und aktiviert keinen Generator. |
| WG-002-MIG, `b299a5fc` | Explizite Format-3→4-Migration mit deklarierter Flat-v1-/Overworld-/Seed0-Basis, vollständiger Daten-/Scopevalidation, neuem geprüftem Backup und atomarem Formatwechsel. Alle 177 Storagetests auch unabhängig grün, Review CLEAN. | Keine historische Seedableitung, automatische Migration oder Reparatur. Transaktions-Cutpoints sind keine Prozesskill-/Stromausfallabnahme. |
| WG-003, `da61c638` | Scope-/Identitätsprüfung vor Chunkadmission; opaque, an dieselbe Arcinstanz gebundener Token für synchrone Dirtycapture. 204 Simtests, strict checks; Fremdreview CLEAN nach RED/GREEN-Reproduktion zweier Admissionlücken. | Producer-/Consumerverträge, noch keine produktive Appaktivierung. |
| SIM-03.1, `6998a76e` | Dauerhafte 128-Bit-EntityUuid als eigener Typ, getrennt von signed i32-Wire-ID und PlayerId. Default-/No-default-/Compile-fail-Checks; unabhängige öffentliche Tests6/4 grün, Review CLEAN. | Keine UUID-Storagekeys, Entityverwaltung, Physik, Tracking oder Mobs. |

Die [Taskdokumente](IMPLEMENTATION_BOARD.md) halten APIs, vollständige
Originalmethoden, Callees und relevante Paperpatches sowie Grenzen fest.
Quellenprüfung ist keine Live-Paper-Differenzialabnahme. Der gepinnte echte
Paper-1.21.8-Build60-JAR ist separat hash-/größengeprüft vorhanden; ein
gestarteter Vergleichslauf ist damit noch nicht belegt.

Das WG-003-Fremdreview reproduzierte eine Generatoränderung über die
Metadaten-Awaitgrenze: vor dem Fix erfolgte ein unzulässiger Worldread. Der
Implementierer prüft den Generator jetzt erneut am Anfang von compose_chunk.
Die unveränderte externe Probe ist danach 2/2 grün. Root prüfte zusätzlich
die Legacyadmission; beliebige Generatoren dürfen deren Flat-Seed0-Basis
nicht wechseln. Logs: `/tmp/wg003-independent-metadata-await-{red,green}.log`.

## Kombinierte Prüfung einer eingefrorenen Runtimebasis

Ein detached Checkout `.codex-tmp/wave03-checkout` auf `6998a76e` trennt die
geprüfte Basis von uncommitteten Änderungen der nächsten Agents. Er erstellt
keinen zusätzlichen Branch. Folgende Prüfungen sind mit Exitcode0 beendet:

| Prüfung | Ergebnis / lokales Log |
| --- | --- |
| `cargo test --workspace --all-targets --offline --locked -j2` | 2.271 bestanden, 0 fehlgeschlagen, 1 ignoriert, 126 Ergebnisgruppen; `/tmp/wave03-workspace-tests.log`. |
| `cargo test --workspace --doc --offline --locked -j2` | 36 bestanden, 0 fehlgeschlagen; `/tmp/wave03-workspace-doctests.log`. |
| `cargo clippy --workspace --all-targets --offline --locked -j2 -- -D warnings` | Bestanden; `/tmp/wave03-workspace-clippy.log`. |
| `cargo fmt --all --check` | Bestanden; `/tmp/wave03-workspace-fmt.log`. |
| `cargo xtask generate --check` | Generierter Protokollcode aktuell; `/tmp/wave03-generated.log`. |
| Maven `verify`, Java21 | Elf Unit- und acht Prozessintegrationstests bestanden; `/tmp/wave03-java.log`. |
| Ignorierter Rust→Java-Vertrag separat | 1 bestanden; `/tmp/wave03-rust-java.log`. Scheduler-Unsupported bleibt ein Fehlerpfad. |
| Originalexporthelper und Inventory | 17 beziehungsweise sieben Selftests und unveränderte Offlinepins; `/tmp/wave03-original-export.log`, `/tmp/wave03-root-inventory.log`. |
| Performance-Harness-Unittests | Sieben bestanden; `/tmp/wave03-performance-harness.log`. Kein Performancebenchmark. |
| Public claims am präzisierten Dokumentationsindex | 94 Selftests, clean149; `/tmp/wave03-root-index-claims.log`. |

`cargo nextest` ist lokal nicht installiert; der vollständige Cargo-Testlauf
ist der lokale Ersatz, CI verwendet nextest. Das ältere
`/tmp/wave03-claims.log` enthält vier unpräzise Dokumentationsformulierungen
auf `6998a76e`. Diese wurden in `0aec222b` korrigiert und erneut geprüft;
das alte Fail-Log wird nicht als bestandene Prüfung ausgegeben.
Crate-/Reviewzahlen sind Teilmengen oder Wiederholungen der Workspaceprüfung.

## Unabhängige Client- und Neustartprüfung

Nach dem erfolgreichen Build der eingefrorenen Basis verwendet jeder Prozess
dieselbe schreibgeschützte Binarykopie:

```text
Binary: .codex-tmp/wave03-bin/ferrumc
SHA256: 98f64d1b5273755c1fab8d6031131d39e52bf46f1f9f7d7482029f9fce21bcb6
Buildlog: /tmp/wave03-build.log
```

| Harness | Ergebnis / lokale Artefakte |
| --- | --- |
| smoke.mjs | Login, Streaming, Bewegung und 28 Musterblöcke einschließlich Glas/Fences/Torches nach Rejoin/Prozessneustart grün; `/tmp/wave03-smoke.log`, `/tmp/ferrumc-wave03-smoke-run`. |
| door-hinge.mjs | 14 echte Platzierungen, 28 Half-States bei Actor/Observer vor und nach SIGINT-Neustart grün; `/tmp/wave03-doors.log`, `/tmp/ferrumc-door-hinge-nNxdQC`. |
| chest-component-persistence.mjs | Komponenten und exakte NBT-NaN-/Zero-/Duplicate-Key-Bytes nach zwei Prozessneustarts erhalten; genau sieben Marker, grün; `/tmp/wave03-chest-components.log`, `/tmp/ferrumc-chest-components-WZRk6c`. |
| inventory-cursor.mjs | Volle Tasche/Cursor, stale Pakete, UUID-Admission, Rejoin und zwei Prozessneustarts grün; 13 Marker erhalten. `/tmp/wave03-inventory-cursor.log`, `/tmp/ferrumc-inventory-client-ot3k8c`. |
| player-inventory.mjs | Bestehende Survival-Left-/Rightclicks, Komponenten, State-/Windowgates, Equipment, Spectator und Close0 grün; `/tmp/wave03-player-inventory.log`, `/tmp/ferrumc-player-inventory-ePlX6W`. |
| operator-commands.mjs | Zwei Clients, live Grant/Revoke, Gamemode und CLI-Recovery nach zwei Neustarts grün; `/tmp/wave03-operator-commands.log`, `/tmp/ferrumc-operator-client-Y6z7jZ`. |
| java-plugin.mjs | Bestehender begrenzter Pluginvertrag, Commands/Permissions/Events, Unsupported-World-Fehler und Shutdown grün; `/tmp/wave03-java-plugin-client.log`, `/tmp/ferrumc-java-client-P45X9q`. |

Diese Abnahme prüft aktive bisherige Consumer und Format-3-Persistenz auf
Regressionen. Sie prüft keine produktive Format-4-Migration, neue
Entitysimulation oder vollständige Survivalrunde. Alle sieben Harnesses
endeten mit Exitcode0. Lokale Logs/Binaries sind
Arbeitsartefakte; Harnesses und reproduzierbare Befehle sind eingecheckt.

## Fortgesetzte Umsetzung

- WG-004: Appstart, Generatorbindung, synchrone Tickcapture, Workerretries,
  Unload-/Shutdownack und explizite Offline-Migrations-CLI.
- GEN-04.2: originale Normal-/Simplex-/BlendedNoise-Sampler; Rootreexport
  unter Python-`-O` bereits byteidentisch, sieben öffentliche Contracts grün.
  Fremdreview und Integration folgen separat.
- REG-ENTITY001: bisher fehlender Entitytypenproducer aus Originaldaten;
  danach SIM-03.2 Entityverwaltung mit UUID-/Wire-/Chunkindex.
- Anschließend natürliche Terrainconsumer und sichere dauerhafte
  Inventory↔Entity-Transfers mit Drop/Pickup, Physik, Tracking und Mobs.

Nether, End, Enderdrache, breite Plugin-API-/NMS-Verträge und die übrigen
214 Haupttickets bleiben anhand des [Katalogs](MINECRAFT_FEATURE_CATALOG.md)
abzuarbeiten. RAM-/CPU-Vorteile erfordern Vergleiche bei gleichem Spielumfang
einschließlich Java-Sidecar; diese Welle liefert keinen solchen Nachweis.
