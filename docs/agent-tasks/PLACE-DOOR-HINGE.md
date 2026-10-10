# Task: PLACE-DOOR-HINGE — BLK-01.1

## Goal

Die Scharnierentscheidung gesetzter Türen verwendet den Cursor und untere
Nachbartüren beliebigen Materials im bestehenden FerrumC-Placementpfad.
Dies ist ein Teilvertrag von BLK-01, keine vollständige Tür-/Survival-Abnahme.

## Crate(s)

- `ferrumc-placement`: `src/lib.rs`, `src/door_hinge_tests.rs`, `INVARIANTS.md`.
- Keine Änderungen an App, Sim, Session, World, Registry oder generierten Daten.

## Context

- [Featurekatalog](../MINECRAFT_FEATURE_CATALOG.md), BLK-01.
- [Implementierungsboard](../IMPLEMENTATION_BOARD.md).
- [Simulation](../architecture/simulation.md).
- [Placement-Invarianten](../../server/crates/ferrumc-placement/INVARIANTS.md).
- [Paper-Referenzvertrag](../PAPER_REFERENCE.md).

Referenz: Paper 1.21.8 Build 60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Gelesen wurde die vollständige,
angewandte `DoorBlock.getHinge`-Methode einschließlich
`getStateForPlacement` und `setPlacedBy` unter
`.codex-tmp/paper-1.21.8-reference/paper-server/src/minecraft/java/net/minecraft/world/level/block/DoorBlock.java`.
SHA-256 der vollständigen angewandten Datei:
`344e6f4f34f7bea34f59fa33fc9b8096dfe1e90c6070bd64561ebe1898bc46fd`.
Die vollständige Vanilla-Basismethode wurde ebenfalls verglichen. Der
[DoorBlock-Sourcepatch](https://github.com/PaperMC/Paper/blob/29c8822d90899c89d2689338e81a98f690bcba12/paper-server/patches/sources/net/minecraft/world/level/block/DoorBlock.java.patch)
verändert Redstone-Nachbarverhalten; die Entscheidungsmethode bleibt gleich.
In den Featurepatches wurde keine zusätzliche Hinge-Änderung gefunden.

## Required Behavior

Das Gewicht zählt linke untere/obere volle Hindernisse mit jeweils −1 und
rechte mit jeweils +1. Pairing-Flags erkennen alle 21 gepinnten Türfamilien,
ausschließlich deren untere Hälfte. Die Bool-/Gewichtzweige behalten die
Reihenfolge der Referenz. Bei Gleichstand entscheiden strikte Vergleiche:

| Facing | Scharnier `right`, wenn … | Sonst |
| --- | --- | --- |
| West | lokales `z < 0.5` | `left` |
| Ost | lokales `z > 0.5` | `left` |
| Nord | lokales `x > 0.5` | `left` |
| Süd | lokales `x < 0.5` | `left` |

Exakt 0.5 ergibt `left`. Beide erzeugten Hälften besitzen dieselben
Scharnier-/Facingwerte; ihre vorhandenen `open=false`/`powered=false`-Werte
bleiben bestehen. Es gibt keine neue öffentliche API und keine zusätzlichen
Abhängigkeiten. Die Berechnung bleibt synchron und begrenzt auf vier
Hinderniszellen und zwei Tür-Flags.

Der tatsächliche Consumer wurde im Quellcode geprüft:
`handle_use_item_on` → `SimCommand::PlaceBlock` → `GameInput::BlockPlace` →
`SimShard::refine_placement` → `compute_placement` → `place_door` → vorhandener
Mutations-/Journalpfad. Session `use_item_on_target` versetzt die Position
bereits um die angeklickte Face; App reicht die Cursorwerte innerhalb der
angeklickten Zelle unverändert weiter. Daher subtrahiert die Hinge-Regel den
horizontalen Face-Schritt, um den Punkt relativ zur Placementzelle zu erhalten.
Diese Rechnung benötigt keine Weltkoordinaten in Fließkomma.

`getHinge` selbst ist eine Beobachtung ohne Events oder Mutation. In Paper
prüft `getStateForPlacement` davor Höhe/Replaceability und setzt Power/Open;
`setPlacedBy` schreibt später die obere Hälfte. Diese zusätzlichen Verträge
sind hier nicht umgesetzt. FerrumCs existierende Interaction-/Place-Hooks
und sein vor dem Commit berechneter After-Hook bleiben unverändert; eine
Paper-gleiche Event-/Commit-Reihenfolge wurde damit nicht nachgewiesen.

## Non-Goals

State-spezifische Collision-/Support-Shapes, Tragblockprüfung, Replaceability,
Redstone/Öffnen, Nachbarphysik, Abbau der Gegenhälfte und Plugin-Eventparität
bleiben weitere Tickets. Das vorhandene `is_solid_cube`-Hindernisproxy wird
weder erweitert noch als vollständige Shape-Parität dargestellt. Exakte
Plugin-/Command-Schreibpfade umgehen Placement weiterhin bewusst.

## Acceptance Criteria

Aus `server/`:

```sh
cargo test --locked -p ferrumc-placement
cargo clippy --locked -p ferrumc-placement --all-targets -- -D warnings
rustfmt --edition 2021 --check crates/ferrumc-placement/src/lib.rs crates/ferrumc-placement/src/door_hinge_tests.rs
cargo doc --locked -p ferrumc-placement --no-deps
```

Die neuen Tests wurden vor dem Produktionsfix ausgeführt:
`cargo test --locked -p ferrumc-placement door_hinge_tests` endete mit Code 101,
sechs fehlgeschlagenen Tests; Log `/tmp/ferrumc-door-red.log`.
Die anschließende vollständige Crate-Prüfung besteht mit **52 Unit-, drei
Integrationstests und einem Doctest**, Log `/tmp/ferrumc-door-green.log`.
Strict-Clippy besteht, Log `/tmp/ferrumc-door-clippy.log`.
Format-/Rustdoc-Logs: `/tmp/ferrumc-door-fmt.log`, `/tmp/ferrumc-door-doc.log`.
Rustdoc endet mit Code 0 und meldet den bestehenden privaten Link
`compute_pane_connection_state` → `pane_connects`; dieser unveränderte
Pane-Dokumentationshinweis gehört nicht zum Hinge-Ticket.
Diese lokalen Logs sind Arbeitsartefakte, keine eingecheckten CI-Artefakte.

## Invariants

Pure Placementberechnung, unveränderte öffentliche Typen, keine Weltmutation,
kein I/O, keine Tickplanung oder zusätzlichen Events. Keine neue unbounded
Allocation oder Produktions-`unwrap`/`expect`. Kein neuer Parser; dessen
Malformed-Input-Abnahme ist hier nicht anwendbar.

## Test Requirements

Unabhängige explizite Goldenmatrizen prüfen alle vier Ausrichtungen bei
Cursor 0.25/0.5/0.75 und positive/negative Weltkoordinaten, echte Hitpunkte
auf vier Seitenfaces mit dem Positionsschritt, alle 21 Materialien als
untere und obere Nachbartürhälfte, Gewicht −2…2 und alle vier Pairing-Flags
mit den realisierbaren oberen Hindernissen. Alle Fälle prüfen beide Hälften.
Einzelne Pairing-Flags und alle Gewichte −2…2 können nicht beliebig kombiniert
werden, weil eine untere Tür selbst keinen vollen Hinderniswürfel darstellt.

## Done Means

Implementiert und lokal im genannten Scope geprüft; unabhängiges Review und
Integration auf `main` führt Root aus. Die Quellenprüfung und deterministischen
Placementtests sind **keine laufende Paper-Differentialabnahme**, kein echter
Mehrspieler-/Restart-Test und kein Abschluss von BLK-01.
