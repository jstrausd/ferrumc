# ITEM-SIZE-01 — unabhängige Producerabnahme

Status: **CLEAN**. Autor `feature_catalog`, unabhängiger Reviewer Root.
Geprüfter Ausgangspunkt `765147ca3935cd69260a4bf682d756299cce4aed`;
Scope ausschließlich Items und eigener Aufgabenbrief. Consumerintegration
erfolgt separat in DUR-04.2a.

## Geprüfter Code und Vertrag

Root hat beide öffentlichen Wrapper, den vollständigen erreichbaren
Preflight-/NBT-/Repräsentationsvergleich und die 13 neuen APItests gelesen.
`persisted_stack_size` verwendet die bestehende exakte FCIS-Größenauthority.
`same_stack_representation` prüft Source und Destination in dieser Reihenfolge
vor jeder booleschen Entscheidung und verwendet den vorhandenen geordneten,
bitgenauen Patch-/NBTvergleich. Gültige Counts beeinflussen die Identität nicht;
Empty und PresentAIR bleiben getrennt. Kein Encoder, Allocator oder Clone ist
in diesen beiden Produktionspfaden erreichbar.

Die ursprünglichen Persistence-/Transferdateien sind nach Entfernen allein
der neuen dokumentierten Wrapper bytegleich zum Ausgangscommit. 21 weitere
vollständige ursprüngliche Itemsdateien bleiben unverändert. Die bounded
FCIS-Größe ist kein Slot-, Heap- oder Aggregatebudget. Der Vergleich belegt
Modellrepräsentation und keine vollständige Paper-Stackability.

## Unabhängige Prüfung

Root erzeugte ein frisches Gitarchiv des genannten Ausgangsstands und kopierte
ausschließlich die 28 eingefrorenen Items-/Briefdateien hinein. 13 Evidence-
und 116 readonly Dependency-/Generated-/Spec-/Konfigurationspins wurden geprüft;
Overlay und Baseline blieben vor/nach allen Prüfungen unverändert. Parallele
Sim-/World-/Appänderungen waren nicht Teil des Archivs.

| Tatsächlicher Check | Ergebnis |
| --- | --- |
| Whole Items Debug | 103 bestanden: 84 Unit, 13 neue Integration, 3 Craftingintegration, 3 Doctests. |
| Whole Items Release | Dieselben 103 bestanden. |
| Alltargetclippy | `--locked --offline -D warnings`, bestanden. |
| Workspacefmt im isolierten Archiv | Bestanden. |
| Items-Rustdoc | `--no-deps`, `RUSTDOCFLAGS=-D warnings`, bestanden. |

Die neuen Tests verwenden den unveränderten FCIS-Encoder als Größen-/Identitäts-
orakel: alle sieben Modellkomponenten, 324 Repräsentationspaare, Counts1..64,
ordered/duplicate NBT, beide Floatbreiten/signed zero/NaNbits, beide Fehlerseiten
mit Sourceprecedence und tatsächliche N/N+1-Byte-/Entry-/Depthgrenzen.

Sourcefreeze `.codex-tmp/item_size_01/source.freeze.json`:
`a2191dbc36cd92c2e6851d31e14d4311ec1f79456db8e6749bb439ff035a95b6`.
Evidencefreeze `.codex-tmp/item_size_01/evidence.freeze.json`:
`6c11bdff0ef26a422a83a9c833607d85dbf2f0c787f83ac3ab1cd6fea66d9bcf`.
Rootrunner `.codex-tmp/item_size_01-root-review.py`, unabhängige vollständige
Logs und Records `.codex-tmp/item_size_01-root-review/checks.json`.

Diese Abnahme erlaubt die getrennte Simadoption. Sie aktiviert weder Item-
Entities noch eine Spielmechanik und belegt keine RAM-/TPS-, Paperplugin-
oder vollständige Survivalabnahme.
