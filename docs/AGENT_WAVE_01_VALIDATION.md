# Erste Agentenwelle: implementierte Teilverträge und Abnahme

Stand: 10.10.2026. Zielversion: Minecraft Java **1.21.8 / Protokoll 772**.
Runtime-/Harness-Basis: `090165bde119cf3f92e54202c1423f68aeb4b15b` auf `main`.
Der Schlussstand ergänzt ausschließlich Katalog, Board, diesen Prüfbericht und
die geplante WG-002-Spezifikation. Root orchestriert, prüft und integriert;
Gameplaycode stammt von den ausdrücklich zugewiesenen Agents.

Der [Katalog](MINECRAFT_FEATURE_CATALOG.md) enthält 214 eindeutige Haupttickets.
Das [Board](IMPLEMENTATION_BOARD.md) führt tatsächliche Vergaben und nächste
Schritte. Die lokale Abnahme dieser Welle betrifft die folgenden Teilverträge;
vollständige Survival-/Worldgen-/Paper-Freigabe bleibt offen.

## Implementierung und unabhängiges Review

| Teilvertrag | Commit | Ergebnis |
| --- | --- | --- |
| WG-001 / GEN-01.1 | `e69bfc71` | Numerischer Seed, geschlossene Generatoridentität, strikter 27-Byte-Codec und reiner ChunkGenerator-Vertrag; bestehender Flat-v1-Pfad bleibt reproduzierbar. 137 Tests, strict Clippy, Format und unabhängiges Review CLEAN. |
| PLACE-DOOR-HINGE / BLK-01.1 | `676a357e` | Originale Cursor-/Hindernis-/Nachbartürentscheidung, alle 21 Materialien, beide Hälften. Sechs ursprüngliche Regressionen, danach 56 Tests; strict Clippy/Format und Review CLEAN. |
| SIM-DOOR-GOLDEN | `d9cac2c7` | Ausschließlich vier alte State-Goldens und Kommentar im vorhandenen Sim-Consumertest an Mittelpunkt → hinge=left angepasst. Keine Produktionslogik geändert; fokussierter Test, Sim-Clippy und Review durch Root bestanden. |
| N03 / NET-06.1 | `dff6c824` | Verbindungseigene IDs/ACK-Gate, finite FIFO-Reihenfolge, stale/replay/Superseding-Teleports, erlaubte Rotation und persistente Serverdestination. Review fand fehlenden Antwortflush; echte Socketregression vorher RED, danach Fix und Re-Review CLEAN. 357 App-Tests, davon sechs Teleport-Sockettests. |
| PLACE-DOOR-HINGE-CLIENT / BLK-01.2 | `090165bd` | Unabhängiges PrismarineJS-Harness, zwei Clients, echte Itemplatzierungen und zwei saubere Serverprozesse; Review CLEAN. Syntax, Golden-Selftest und Live-Abnahme bestanden. Als eigenes CI-Gate aktiviert. |

Die Produktionsänderungen sind jeweils auf eine Crate beschränkt. Der
Sim-Nachtrag verändert nur einen bestehenden Test. Die vollständigen gepinnten
Paper-Methoden sowie Basis-/Source-/Featurepatches wurden geprüft; genaue
Dateien, Verträge, Events und Grenzen stehen in den jeweiligen
[Agenttickets](IMPLEMENTATION_BOARD.md#umsetzungstickets).
Eine Quellenprüfung ist kein ausgeführter Paper-Laufzeitvergleich.

## Kombinierte lokale Prüfungen

Nach dem Flushfix und der Sim-Golden-Korrektur wurde der gemeinsame Stand erneut
geprüft. Alle folgenden Befehle endeten mit Exitcode 0:

| Prüfung | Ergebnis / lokales Log |
| --- | --- |
| `cargo fmt --all --check` | Bestanden; `/tmp/ferrumc-wave1-fmt-final.log`. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Bestanden; `/tmp/ferrumc-wave1-workspace-clippy-final.log`. |
| `cargo test --workspace --all-targets --locked` | **2160 bestanden, 0 fehlgeschlagen, 1 ignoriert**, 120 Ergebnisgruppen; `/tmp/ferrumc-wave1-workspace-tests-final.log`. |
| `cargo test --workspace --doc --locked` | **30 bestanden**, keine Fehler; `/tmp/ferrumc-wave1-workspace-doc-tests-final.log`. |
| `cargo xtask generate --check` | Protokollcode aktuell; `/tmp/ferrumc-wave1-generate-final.log`. |
| Maven `verify`, Java 21 | **19 Tests bestanden**: 11 Unit- und 8 Prozessintegrationstests; `/tmp/ferrumc-wave1-java.log`. |
| Ignorierter Rust→Java-Vertrag separat ausgeführt | **1 bestanden**; `/tmp/ferrumc-wave1-rust-java-final.log`. Die bewusst getestete nicht unterstützte Scheduler-API bleibt ein Fehlerpfad, keine implementierte Schedulerfunktion. |
| Inventory `--self-test --check` | **5 Selftests**, Bericht unverändert: 1105 Blocktypen / 27946 States, 1416 Items, 65 Biome, 245 Pakete. Datenbestand belegt keine Spielmechanik. |
| Performance-Harness-Unittests | **7 bestanden**; Eigentum/Cleanup geprüft, keine neue Performance-Messung. |
| Forbidden-claims `--self-test --check-index` | **94 Selftests**, Indexprüfung vor jedem Integrationscommit. |

Die sechs Teleport-Sockettests sind Teil der 357 App-Tests und der 2160
Workspace-Tests; sie werden nicht als zusätzliche unabhängige Testzahl addiert.
Die normale Workspace-Prüfung ignoriert den Java-Vertrag; der separate Lauf
prüft ihn tatsächlich mit den durch Maven gebauten JARs.

## Unabhängige Clients und Artefakt

Vor den Clientläufen wurde das Binary in eine eigene schreibgeschützte Kopie
übernommen. Beide Starts jedes Tests verwenden diese Kopie. Nach den letzten
Workspacechecks wurde `cargo build --locked -p ferrumc-app` erneut auf obigem
Runtime-/Harness-Commit ausgeführt: sein SHA ist identisch mit dem getesteten
Binary. Eine zwischenzeitliche Änderung des Cargo-Testbinaries wird damit nicht
als getestetes Artefakt ausgegeben.

```text
SHA-256: 364cd3b81aa114f0000ca217318b929c3cabf45f83d7202e3efb83d1e57f4878
Testkopie: .codex-tmp/wave01-bin/ferrumc
Finale identische Kopie: .codex-tmp/wave01-final-bin/ferrumc
```

- `smoke.mjs`: Anmeldung, Streaming, Bewegung, echte Glas-/Fence-/Torch-/andere
  Placementpfade und alle 28 Musterblöcke nach Rejoin/Prozessneustart bestanden.
  Log `/tmp/ferrumc-wave1-blackbox-smoke.log`.
- `door-hinge.mjs`: **14 echte Door-Itemplatzierungen**, vier Yaws × drei
  Cursorwerte und eine gemischte Birch/Oak-Nachbartür. **28 Half-States pro
  Client**, jeweils Actor und Observer vor und nach Restart: 56 Prüfungen je
  Phase. Zwei SIGINT-Shutdowns mit Exitcode 0; derselbe Binary-SHA und dieselbe
  World-DB. Result `/tmp/ferrumc-door-hinge-rfW1Sv/result.json`, Prozesslogs und
  DB im selben Ordner; Clientlog `/tmp/ferrumc-door-client-live.log`.

Diese Logs und Binaries sind lokale Arbeitsartefakte. Das reproduzierbare
Harness und die Befehle sind eingecheckt. GitHub-CI führt Rust-, Java-,
Generator- und unabhängige Clientgates für den veröffentlichten Head aus;
[Workflow](https://github.com/jstrausd/ferrumc/actions/workflows/ci.yml).
Ein früherer grüner CI-Lauf wird nicht als Ergebnis dieser Welle ausgegeben.

## Offen für die nächsten Agents

- WG-001 ist ein Daten-/Generatorvertrag. Natürliche Welt, persistierter
  Descriptor, Seedkonfiguration und Sim-/App-Aktivierung fehlen weiterhin.
  [WG-002](agent-tasks/WG-002_GENERATION_METADATA.md) ist nur spezifiziert:
  explizites Format 4, atomare Freshness/Saves, separate Offline-Migration ohne
  geratenen historischen Seed.
- BLK-01 bleibt T: stateabhängige Shapes/Support, Power/Open, Nachbarphysik,
  Gegenhälften-Abbau und Plugin-Eventparität sind nicht abgeschlossen.
- NET-06 bleibt T: **NET-06.2** muss die gemeinsame Epoch-/Move-Admission für
  externe Teleports absichern, die erst nach dem App-Queue-Snapshot eingereiht
  werden. ACK-Gate und captured FIFO-Fence begründen keine globale atomare
  Welt-/Tickreihenfolge. Vollständige Bewegung/Dimension-/Relative-Teleports
  und Java-Teleportevents fehlen weiterhin.
- Inhaltsmatrix-Exporte für Entities, Recipes/Loot/Worldgen/Commands und API
  sowie ihr Verhalten bleiben CAT-Folgetickets. Survival, Mobs, Wasserfluss,
  Redstone/TNT, Dimensionen und breite Java-/NMS-Kompatibilität bleiben die
  offenen Hauptpakete; dieser Bericht schließt keines davon ab.
- Es wurde kein Paper-Prozess für eine Differenzialabnahme gestartet und kein
  vollständiger Vanilla-GUI-Survivalablauf geprüft. Es gibt keine neue Aussage
  zu RAM-/CPU-Vorteilen bei gleichem Gameplay-/Pluginumfang.

Der [Survival-Lieferplan](SURVIVAL_DELIVERY_PLAN.md) bleibt die grobe
Aufwandsgrundlage. Das Board hält die nächsten abhängigkeitstreuen Vergaben
fest; eine abgeschlossene Sitzung ist kein zugesicherter Hintergrunddienst.
