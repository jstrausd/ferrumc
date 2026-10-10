# Zweite Agentenwelle: Generator-Metadaten, Item-Pläne und Originaldaten

Stand: 10.10.2026. Zielversion: Minecraft Java **1.21.8 / Protokoll 772 /
DataVersion 4440**. Ausgangspunkt: `d96228b8`; integrierte Runtime-/Toolingbasis:
`a7f7aa33`. Der Schlusscommit ergänzt nur Dokumentation. Root koordinierte
Dateibesitz, Reviews, Prüfungen und Gitintegration; Produktionscode stammt
von den zugewiesenen Agents. Arbeitsbranch bleibt ausschließlich `main`.

Der [Katalog](MINECRAFT_FEATURE_CATALOG.md) enthält weiterhin 214 Haupttickets;
das [Board](IMPLEMENTATION_BOARD.md) führt die begrenzten Unterverträge und
nächsten Schritte. Diese Welle liefert Grundlagen für Weltgenerierung und
Survival; produktive Generatoraktivierung und Drop/Pickup folgen separat.

## Implementierung und unabhängige Reviews

| Ticket / Commit | Implementierung und Prüfung | Verbleibende Grenze |
| --- | --- | --- |
| WG-002 / GEN-11.1, `dd910fd4` | `ferrumc-storage`: private typisierte 8-Byte-Keys, 27-Byte-WG-001-Identität, immutable Load/Ensure, 4096-Cap; explizites Format 4/neun Tabellen und atomare Freshness-/Save-Admission. 146 Crate-Tests, strict Clippy/Format; fremdes Review CLEAN und sieben öffentliche Contracttests separat grün. | Format 3 bleibt produktiv. Keine automatische Migration oder Seedvermutung; App-/Sim-Anbindung und natürliche Generierung offen. Player/Journal verlangen nur irgendeine Identität, weil ihre Records noch keinen Scope tragen. |
| ITEMS-TRANSFER-001 / SRV-01.1, `a7f7aa33` | `ferrumc-items`: reine geprüfte Split-/Transferpläne, Mengen-/Komponentenerhaltung, explizite Zielkapazität, exakte Patch-/NBT-/Floatbitrepräsentation. Gemeinsamer allocation-free Persistenzpreflight, unveränderte FCIS-Bytes. 72 Unit- und drei Doctests, strict Clippy/Format; fremdes Review CLEAN. | Keine Consumeraktivierung, Slot-/Gameplaypolicy, effektive Paper-Defaultauflösung, Versions-/CAS-Guards oder dauerhafte Ownershiptransaktion. Inkompatible Stacks bleiben unverändert; Vergleich ist bewusst strenger als Papers normalisierte Maps. |
| CAT-02.1, `37c037b9` | Tooling und acht Originalfixtures: SHA-/Längen-geprüftes Mojang-Bundle, Java-21-Datagen, reproduzierbarer Vollmengen-/Rohbytevergleich für 3609 ausgewählte Ressourcen und Offline-CI-Gates. 17 Helper- und sieben Inventorytests; fremdes Review CLEAN nach zwei Grenzfixes. | Nur ausgewählte Standard-Vanilla-Daten; weitere Registries/Tags, NBT-Templates, Commands/API und Consumer fehlen. Original-Minecraft-Daten erhalten keine MIT-Lizenz. |

Reviewrotation: `item_transfer_primitives` prüfte WG-002,
`select_world_ticket` prüfte CAT-02.1, `feature_catalog` prüfte die Item-Pläne.
Jeder Agent prüfte fremden Code. Root führte allein Gitoperationen aus.
Die Produktionsänderungen sind in getrennten Commits mit jeweils einer Crate.

Das Item-Review führte außerdem eine eigene Public-API-Probe aus:
**1.377.792 Mengen-/Capacityentscheidungen** gegen eine One-item-Referenz und
**3.364 Persistenzrepräsentationsvergleiche**, einschließlich NaNs, Signed Zero,
Duplicate-/ordered NBT, Removals und typed/opaque Varianten. Logs:
`/tmp/ferrumc-items-transfer-independent-{tests,probe}.log`. Diese Fälle sind
keine zusätzliche Cargo-Testanzahl und keine aktive Gameplayabnahme.

Beim Export fand Root einen gemischten ZIP64-/Legacy-EOCD-Bypass der
Vorprüfung. Der Implementierer ergänzte die explizite Locatorablehnung vor
`ZipFile`-Konstruktion und eine Regression. Das fremde Review fand danach
ein Zeichen-/UTF-8-Bytecap-Mismatch; Byteprüfung und Mehrbyte-Grenztest folgten.
Beide Fixes wurden unabhängig erneut geprüft; Daten/Pins blieben identisch.

Die vollständigen gepinnten Paper-/Vanilla-Methoden, relevanten Callees und
Patches wurden gelesen. Die [Taskdokumente](IMPLEMENTATION_BOARD.md#umsetzungstickets)
halten Verhalten, bewusste Abweichungen und Fehlergrenzen fest.
Quellenreview und offizieller Datagen sind keine Live-Paper-Differenzialabnahme.

## Kombinierte lokale Prüfungen

Nach Runtime-Freeze und den Exportfixes endeten alle folgenden Prüfungen mit
Exitcode 0. Der veränderte Exporthelper wurde nach dem letzten Fix erneut geprüft.

| Prüfung | Ergebnis / Log |
| --- | --- |
| `cargo fmt --all --check` | Bestanden; `/tmp/ferrumc-wave2-fmt.log`. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Bestanden; `/tmp/ferrumc-wave2-clippy.log`. |
| `cargo test --workspace --all-targets --locked` | **2198 bestanden, 0 fehlgeschlagen, 1 ignoriert**, 121 Ergebnisgruppen; `/tmp/ferrumc-wave2-tests.log`. |
| `cargo test --workspace --doc --locked` | **31 bestanden**, keine Fehler; `/tmp/ferrumc-wave2-doctests.log`. |
| `cargo xtask generate --check` | Protokollcode aktuell; `/tmp/ferrumc-wave2-generated.log`. |
| Maven `verify`, Java 21 | **19 bestanden**: elf Unit- und acht Prozessintegrationstests; `/tmp/ferrumc-wave2-java.log`. |
| Ignorierter Rust→Java-Vertrag separat | **1 bestanden**; `/tmp/ferrumc-wave2-rust-java.log`. Scheduler-Unsupported bleibt ein geprüfter Fehlerpfad, keine neue Schedulerfunktion. |
| Originalhelper `--self-test --check` | **17 bestanden**, alle acht Originalfixtures unverändert; `/tmp/ferrumc-wave2-original-data.log`. |
| Inventory `--self-test --check` | **7 bestanden**, Report/Pins unverändert; `/tmp/ferrumc-wave2-inventory.log`. |
| Performance-Harness-Unittests | **7 bestanden**; `/tmp/ferrumc-wave2-performance-harness.log`. Keine neue Performance-Messung. |
| Public-claims `--self-test --check-index` und `git diff --cached --check` | Vor jedem Integrationscommit bestanden; 94 Claims-Selftests. Logs `/tmp/ferrumc-wave2-{storage,export,items,docs}-index-claims.log`. |

Die Crate-/Reviewer-Testzahlen sind Teilmengen oder Wiederholungen und werden
nicht zur Workspacezahl addiert. Der separat ausgeführte Java-Vertrag entspricht
dem im Workspace bewusst ignorierten Test.

## Originaldaten und reproduzierbarer Export

Der Bestand ergänzt **151 Entitytypen, 22 Recipe-Serializer, 1407 Rezepte,
1246 Loot Tables, 952 Worldgen-Werte in 14 Registries, vier Dimensiontypen
und sechs Presets mit 18 Dimensionstems**. Acht Fixtures: **4.469.047 Bytes**.

Der Implementierer prüfte einen frischen Export aus verifizierten Downloads
sowie einen kalten Cache mit echten Downloads. Der Reviewer führte einen
weiteren frischen Datagenlauf aus einem separaten SHA-geprüften Cache durch:
alle acht Outputs byteidentisch; `/tmp/ferrumc-cat021-independent-export.log`.
Der Providerlog im separaten Cache dokumentiert old count 0 und 6113 neu
geschriebene Dateien. Die gewählte Fixturemenge umfasst 3609 Originalressourcen.
Zusätzlich verglich der Reviewer deren ID-Mengen/JSON-Werte unabhängig vom
Export-Assemblierungshelper vollständig mit dem eingebetteten Original-JAR.

Die [Exportnotiz](agent-tasks/CAT-02_1_ORIGINAL_CONTENT.md) und das
[Manifest](fixtures/minecraft-1.21.8-original/manifest.json) dokumentieren
Herkunft, Normalisierung, Limits, reproduzierbare Befehle und unerfasste Daten.
Das originale Server-Bundle, Bibliotheken und Klassen bleiben im ignorierten
Arbeitscache. Das neue CI-Gate prüft die eingecheckten Daten vollständig offline.

## Unabhängige Client-/Neustartprüfung

Nach den Cargo-Prüfungen wurde `cargo build --locked -p ferrumc-app` ausgeführt
und das Binary als schreibgeschützte Kopie eingefroren. Jeder Prozessstart
der folgenden Harnesses verwendet diese feste Kopie:

```text
SHA-256: 0125339c5d00189a9072f8be4f773044ab63dcfd69ebdce5b11dc5256636b145
Binary: .codex-tmp/wave02-bin/ferrumc
Buildlog: /tmp/ferrumc-wave2-build.log
```

| Harness | Tatsächlich geprüft / Artefakt |
| --- | --- |
| `smoke.mjs` | Login, Streaming, Bewegung, aktuelle Glas-/Fence-/Torch-/Placementpfade und alle 28 Musterblöcke nach Rejoin/Prozessneustart; `/tmp/ferrumc-wave2-smoke.log`, Run `/tmp/ferrumc-wave2-smoke-run`. |
| `door-hinge.mjs` | 14 echte Itemplatzierungen, 28 Half-States je Actor/Observer vor und nach SIGINT-Neustart; beide Prozesse Exitcode 0. `/tmp/ferrumc-wave2-doors.log`, Result `/tmp/ferrumc-door-hinge-FCKS6B/result.json`. |
| `inventory-cursor.mjs` | Volle Tasche/Cursor, Fensterwechsel, stale Pakete, UUID-Admission, Disconnect und Komponenten nach zwei Prozessneustarts; 13 Marker erhalten. `/tmp/ferrumc-wave2-inventory-cursor.log`, Run `/tmp/ferrumc-inventory-client-CpHbJx`. |
| `chest-component-persistence.mjs` | Name/Damage/Removals und exakte NBT-NaN-/Zero-/Duplicate-Key-Bytes; zwei Prozessneustarts und genau sieben vollständige Marker. `/tmp/ferrumc-wave2-chest-components.log`, Run `/tmp/ferrumc-chest-components-2OzTFm`. |

Diese Läufe prüfen die aktiven bisherigen Consumer und Format-3-Persistenz
auf Regressionen nach der Preflightextraktion. Sie aktivieren weder den neuen
Format-4-Pfad noch die neuen reinen Transferpläne. WG-002 prüft Backend-Reopen,
atomare Transaktionsabbrüche und Retry nach Commit; das sind keine neuen
Stromausfall-/Prozesscrashtests.

Logs/Binaries sind lokale Arbeitsartefakte; Harnesses und Prüfkommandos sind
eingecheckt. Der [GitHub-Workflow](https://github.com/jstrausd/ferrumc/actions/workflows/ci.yml)
prüft den veröffentlichten Schlussstand zusätzlich. CI der ersten Welle ist
ältere Evidenz und ersetzt den Lauf für den neuen Head nicht.

## Nächste Schritte

- WG-002-MIG: explizite geprüfte Format-3→4-Migration mit deklarierter Legacybasis;
  anschließend WG-003 Sim-Descriptorconsumer und WG-004 App-Aktivierung.
- DUR-03: dauerhafte Inventory↔Entity-/Container-Ownershiptransaktion;
  SIM-03 Entitygrundlage, DUR-04/05 Consumer/Wiring und SRV-02 Drop/Pickup.
- SIM-04 Shapes/Support und weitere CAT-02-Daten; Tagauflösung und spätere
  Recipe-/Loot-/Worldgenconsumer benötigen eigene Abnahmen.
- Natürliche Welt, vollständiger Survival, Mobs, Fluids, Redstone/TNT,
  Dimensionsfortschritt und breite Java-/NMS-Kompatibilität bleiben offen.
  Es gibt keinen neuen Nachweis von CPU-/RAM-Vorteilen bei gleichem Spielumfang.

Die abgeschlossene Welle ist kein zugesicherter Hintergrunddienst. Das Board
hält den Arbeitsstand für die nächste Umsetzung fest.
