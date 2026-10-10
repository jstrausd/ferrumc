# SIM-DOOR-GOLDEN — Türscharnier im Sim-Consumertest

Stand: 10.10.2026. **Status: Test-Golden korrigiert und lokal geprüft;
Integration durch Root.**

## Umfang

Nur der bestehende Test
`server/crates/ferrumc-sim/src/shard.rs::tests::place_door_writes_lower_and_upper_halves`
und seine Kommentare wurden geändert. Keine Produktionslogik, anderen Tests,
öffentlichen APIs oder Dependencies wurden verändert. Dies ist ein eigener
One-Crate-Testschritt nach [PLACE-DOOR-HINGE](PLACE-DOOR-HINGE.md), kein neues
Gameplayfeature und kein Abschluss der Tür-/Survival-Abnahme.

## Originalreferenz und tatsächlicher Cursor

Paper Java Edition **1.21.8 Build 60**, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Die vollständige angewandt gepatchte
Quelle unter
`.codex-tmp/paper-1.21.8-reference/paper-server/src/minecraft/java/net/minecraft/world/level/block/DoorBlock.java`
wurde gelesen: `getStateForPlacement`, `getHinge` und `setPlacedBy`.
Bei fehlenden Nachbartüren und Hindernissen entscheidet `getHinge` mit strikten
Vergleichen des lokalen Cursors. Südgerichtet ergibt nur `x < 0.5` ein rechtes
Scharnier; der exakte Mittelpunkt ergibt `left`. Die obere Hälfte übernimmt
Facing/Scharnier der unteren Hälfte.

Der vorhandene Sim-Testhelfer `place_block` setzt ausdrücklich
`cursor_position = Vec3::new(0.5, cursor_y, 0.5)`. Der betroffene Aufruf verwendet
`Direction::Up`, `cursor_y=0.5`, `yaw=0.0`; damit gelten x/z-Mitte ohne
horizontalen Face-Schritt und südliches Facing. Die geprüfte Erwartung ist
folglich `hinge=left`.

Der gepinnte 1.21.8-Blockbestand liefert für die geschlossene, unpowered
Eichentür südgerichtet die expliziten IDs **4713** für `half=lower` und **4705**
für `half=upper`. Geprüft wurden die Propertyreihenfolge/State-Range in
`server/crates/ferrumc-registry/data/blocks.json` sowie der unabhängige
Prismarine-1.21.8-Bestand. Diese Goldens werden nicht aus der zu prüfenden
Placementfunktion abgeleitet.

## Reproduktion und Änderung

Der vorangegangene Root-Workspace-Lauf unter
`/tmp/ferrumc-wave1-workspace-tests.log` enthielt im Sim-lib **157 bestandene
Tests und diesen einen Fehler**: tatsächlicher unterer State 4713, stale
Erwartung 4717. Der alte Test hatte Mittelpunktplatzierung pauschal als
`hinge=right` beschrieben und erwartet.

Korrigiert wurden die beiden Zustandserwartungen und die beiden erwarteten
`GameOutput::BlockChanged`-States: lower 4717 → 4713, upper 4709 → 4705.
Sequence, MutationCause und vorhandene Assertions bleiben erhalten. Der
Kommentar benennt den tatsächlichen Mittelpunkt-Cursor und dessen strikte
Tieentscheidung.

## Lokale Prüfung

Aus `server/`:

```sh
cargo test --locked -p ferrumc-sim --lib shard::tests::place_door_writes_lower_and_upper_halves
cargo clippy --locked -p ferrumc-sim --all-targets -- -D warnings
rustfmt --check --edition 2021 --config skip_children=true crates/ferrumc-sim/src/shard.rs
```

- Fokussierter Consumertest: **1 bestanden, 157 gefiltert**, Exitcode 0;
  Log `/tmp/ferrumc-sim-door-golden-test.log`.
- Strict Sim-Clippy inklusive Targets: Exitcode 0;
  Log `/tmp/ferrumc-sim-door-golden-clippy.log`.
- Owned-Datei-Formatprüfung: Exitcode 0. Read-only Diffprüfung bestätigt, dass
  ausschließlich der zugewiesene bestehende Test verändert wurde.

Diese Abnahme prüft den Sim-Consumerzustand und dessen GameOutputs. Sie startet
keinen Paper-Prozess und ist keine Netzwerk-/Mehrspieler-/Restart-Abnahme.
Der unabhängige Clienttest besitzt den getrennten Vertrag
[PLACE-DOOR-HINGE_CLIENT](PLACE-DOOR-HINGE_CLIENT.md). Ein vollständiger neuer
Workspace-Lauf sowie Commit/Push bleiben Orchestratorarbeit.
