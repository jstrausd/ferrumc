# D0: Placement-Korrekturen und Abnahme

Stand: 10.10.2026. Zielversion: Minecraft Java 1.21.8, Protokoll 772.
Ausgangscode: `780485865f2f2ea1a7f5705f9ea071f970005e08`.
Dieser Bericht ergänzt den [Survival-Lieferplan](SURVIVAL_DELIVERY_PLAN.md).
D0 und die vollständige Survival-/Paper-Kompatibilität bleiben offen.

## Torch-Itembasis

Der reale Inventarpfad lieferte für Torch, Soul Torch und Redstone Torch
die Wand-Defaults aus dem Reverse-Mapping. Die vorhandene Placement-Regel
erkennt hingegen die stehenden Varianten. Dadurch wurde ein echtes Torch-Item
auch auf einer oberen Steinfläche als Wandfackel gesetzt.

Der Registry-Generator normalisiert genau diese drei Familien auf ihre
stehenden Defaults. Item- und Blocknamen stammen aus den gepinnten Tabellen;
eine unerwartete ursprüngliche Zuordnung bricht den Build ab. Die Rohdaten
bleiben unverändert. Andere Items behalten ihre bisherigen Zuordnungen.

Referenz: Paper 1.21.8 Build 60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`, vollständig mit Java 21 und
`./gradlew --no-daemon applyPatches` erzeugte Minecraft-Quellen.
Gelesen: `Items`-Registrierungen der drei Familien,
`StandingAndWallBlockItem.getPlacementState/registerBlocks`,
`BlockPlaceContext.getNearestLookingDirections`, `BaseTorchBlock` und
`WallTorchBlock`. Sowohl Basis als auch CraftBukkit-/Paper-Patches wurden
einbezogen; Version und Wiederherstellung stehen in
[PAPER_REFERENCE.md](PAPER_REFERENCE.md).

Die Referenz verwendet eine stehende Basis und eine zusätzliche Wandvariante.
Sie prüft außerdem Support, Blickrichtungsreihenfolge, Unobstructed und
`BlockCanBuildEvent`. Diese zusätzlichen Regeln implementiert der Mapping-Fix
noch nicht. Auch Unterseiten/Fallback, Supportverlust, Redstone-Signalverhalten
und Java-Event-Parität bleiben offen. Es gab keinen Vergleichslauf gegen einen
laufenden Paper-Server.

## Tatsächlich bestandene Torch-Prüfungen

| Prüfung | Ergebnis und Umfang |
| --- | --- |
| Regression vor Fix | Registry-Test: Item 310 lieferte 2402 statt 2401. Socket-Test: obere Steinfläche lieferte denselben falschen State. Unabhängiger Node-Client: `wall_torch` statt `torch`. Alle drei reproduzierten den realen Fehler. |
| `cargo test --locked -p ferrumc-registry` | 29 Unit- und sechs Doc-Tests bestanden. Die gesamte übrige Itemtabelle behält ihre Driftprüfung. |
| `cargo clippy --locked -p ferrumc-registry --all-targets -- -D warnings` | Bestanden. |
| `cargo test --locked -p ferrumc-app --test torch_placement -- --nocapture` | Zwei Tests bestanden: echte Inventaritems, drei echte Steinträger, 15 Boden-/Wandplatzierungen, Actor und zweiter Beobachter; sauberer Shutdown und erneutes Öffnen desselben Redb-Speichers. Chunkpaletten werden unabhängig von FerrumCs Weltencoder gelesen. Zusätzlich malformed Palette-Daten geprüft. |
| App-Test-Clippy | `cargo clippy --locked -p ferrumc-app --test torch_placement -- -D warnings` bestanden. |
| Unabhängiger verwalteter Node-Smoke | Alle 15 Torch-Zustände plus Träger, bestehende Bau-/Abbauaktionen, Rejoin und vollständiger Prozessneustart bestanden. Prismarine liest Namen/Properties unabhängig. Beide Prozesse endeten mit Code 0 ohne Signal. |

Node-Prüfkonfiguration: frische Flat-Welt, `world_dir` im eigenen temporären
Run-Verzeichnis, `dashboard_enabled=false`, Keepalive 10 Sekunden. Der Test
startet das mit `cargo build --locked -p ferrumc-app` gebaute Debug-Binary.
Der erste erfolgreiche Torch-Lauf verwendete SHA-256
`b8b2e0f59db094cac06f6f4f7c2c5dd4abd66011233e044275907f363c958e1f`.
Das ist keine Ressourcenmessung und keine Abnahme eines Containerimages.

Der Smoke-Harness lehnt nun auch fehlende Prozesse, erfolglose Exits und
Shutdown-Timeouts ab. Ein notwendiger SIGKILL räumt auf, macht den Lauf aber
weiterhin fehlerhaft; er darf keine bestandene Restart-Abnahme erzeugen.

## Begrenzte Fence-Nachbaraktualisierung

Nach erfolgreichem regulärem Place/Break prüft Sim die vier residenten
horizontalen Nachbarzellen je tatsächlich geändertem Block. Bei jedem Zauntyp
wird ausschließlich das Verbindungsbit zum geänderten Block aktualisiert.
`waterlogged` und andere Arme bleiben unverändert. Die Extrazelle einer Tür
oder eines Betts wird vor dem Refresh geschrieben. Sekundäre Änderungen
gehen durch den vorhandenen Journal-/Persistenzpfad und werden übertragen,
ohne zusätzliche Spieler-ACKs. No-op-Zustände und abgelehnte Edits lösen keine
sekundäre Änderung aus; die Arbeit ist begrenzt und nicht rekursiv.

Die gepinnte vollständige `FenceBlock.updateShape`-Quelle belegt den Erhalt
aller übrigen Properties beim Ändern eines Arms. `FenceBlock.connectsTo`,
`FenceGateBlock.connectsToDirection`, `Block.isExceptionForConnection`,
`SupportType.FULL` und die Fence-Tags wurden ebenfalls gelesen. Ihr vollständiges
Verbindungsprädikat ist noch offen: Die Live-Query verwendet weiterhin die
grobe `boundingBox == "block"`-Metadatenquelle. Materialfamilien, Torachsen,
Ausnahmen und zustandsabhängige volle Anschlussflächen benötigen weitere
separate Änderungen. Das aktuelle Paket belegt keine allgemeine Vanilla-
Shape-Parität, Fluid-Ticks oder Java-Event-Parität.

Exact-write und Region-/Undo-Operationen behalten ihren bisherigen Vertrag
ohne Nachbarphysik. Es gibt keine Force-loads, keine shardübergreifende
Aktualisierung und keine zusätzliche Load-Reconciliation. Darstellbare
Nachbarpositionen werden mit `checked_add` gebildet; jenseits des i32-Bereichs
wird kein Wraparound oder Panic zugelassen.

| Prüfung | Tatsächliches Ergebnis |
| --- | --- |
| Regression vor Fix | Fünf der ersten sechs Sim-Fälle scheiterten an unveränderten Fence-States. Echter Actor-Client erhielt nach Stone-Placement keinen Fence-Update; ein später bestätigter Marker desselben World-FIFO belegt das ohne Timeout. Der Node-Client reproduzierte denselben fehlenden Refresh. |
| Placement | 38 Unit-, drei Determinism- und ein Doc-Test bestanden; striktes All-target-Clippy bestanden. Ein Property-Test prüft alle 32 States von 13 Fence-Familien, vier Richtungen und beide Verbindungswerte. Wasser-/Disconnect-Regression zunächst RED, danach GREEN. |
| Sim | Gesamte Sim-Suite und striktes All-target-Clippy bestanden. Sieben Refresh-Tests prüfen Wasser/andere Arme, Stone und anderen Holztyp, Place/Break in einem Tick, Ablehnung/No-op/Exact-write, sekundäre Journal-/Dirty-Einträge, residente Chunkgrenzen, unbekannte andere Richtungen und beide i32-Ränder auf beiden Achsen. Der Randtest reproduzierte vor seiner Korrektur einen Integer-Overflow. |
| `cargo test --locked -p ferrumc-app --test fence_neighbors --test torch_placement -- --nocapture` | Vier Tests bestanden: beide tatsächlichen Item-Flows, Actor/Viewer, ACKs, unabhängige Chunkdekodierung und sauberer Redb-Neustart. Fence-Flow: Oak, Stone setzen/abbauen, Spruce setzen/abbauen, Stone erneut setzen. |
| Unabhängiger Node-Smoke | Beide Placement-Familien, Fence-Refresh, Rejoin und Prozessneustart bestanden. Der finale persistierte Vergleich umfasst 25 Blöcke; Shutdown jeweils Code 0. Alte Blockbeobachtungen werden vor Edits gelöscht und können keine neue Aktion bestätigen. |

Der Socket-Neustartfall verwendet eine Sektion für Primär- und Sekundärblöcke;
er beweist deren Endzustand nach Restart. Den separaten Dirty-Vertrag der
sekundären Änderung über Chunkgrenzen prüft ausdrücklich der Sim-Test.
Die vollständige Workspace-Abnahme nach der ersten Integration bestand mit
2.117 Tests, einem separat auszuführenden ignorierten Java-Artefakttest,
striktem Workspace-Clippy, Formatprüfung und Generatorprüfung. Nach dem
zusätzlichen Integer-Randfix wurde die gesamte Sim-Suite erneut erfolgreich
geprüft. Bestehende Java-, Container- und Performanceergebnisse werden durch
diese Placement-Fälle nicht erweitert.
