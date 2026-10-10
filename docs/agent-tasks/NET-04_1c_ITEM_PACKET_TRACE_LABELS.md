# NET-04.1c — vollständige App-Trace-Namen für Entitypakete

Status: enge Rootzuweisung nach tatsächlicher CI-Rejection auf
`765147ca3935cd69260a4bf682d756299cce4aed`; Labelmapping ergänzt, isolierte
App-Build-/Lintabnahme bestanden und für Rootreview eingefroren.
Autor besitzt nur `server/app/src/observe.rs`
und diesen Brief; sämtliche anderen Runtime-/Test-/Dependencydateien sowie
ITEM-SIZE-01 und NET-04.1b bleiben readonly. Nur Root integriert Git.

## Konkreter Fehler und Umsetzung

`clientbound_play_name` besitzt einen vollständigen Match über
`ClientboundPlayPacket`. NET-04.1 ergänzt SetEntityData, SetEntityMotion und
TakeItemEntity; fehlende App-Matcharme verursachen E0004 im Workspacebuild.
Die drei Varianten erhalten ihre expliziten stabilen snake_case-Namen
`set_entity_data`, `set_entity_motion` und `take_item_entity`.

Die existente Funktion und alle bisherigen Labels bleiben unverändert;
kein Wildcard, Bodyencode, Bytebuffer, Parser, Queue-, Writer-, Session-,
Gameplay- oder Persistenzumbau. Traces behalten ihren bestehenden
`&'static str`-/Packet-ID-Vertrag und die unveränderte Play-Size0-Regel.
AGENTS, App-INVARIANTS und Netzwerk-/Architekturverträge gelesen. Es handelt
sich um lokales Observabilitymapping, keinen neuen Paper-/Survivalvertrag.
Keine neuen Tests für die triviale Namenszuordnung; echter App-Build und
striktes Alltargetclippy prüfen den tatsächlichen exhaustive Consumer.

## Isolierte Abnahme

Archiv von `765147ca` ausschließlich mit `observe.rs` und diesem Brief
überlagern; parallele fremde WIP und ITEM-SIZE-01 werden nicht eingebunden.
Mit eigenem Targetdir `cargo build --locked -p ferrumc-app`,
`cargo clippy --locked -p ferrumc-app --all-targets -- -D warnings` sowie
`cargo fmt -p ferrumc-app -- --check` prüfen.

Alle drei Prüfungen sind mit Exit0 bestanden, einschließlich aller vorhandenen
Apptargets unter Clippy mit `-D warnings`; keine Lintausnahme und keine neuen
Tests für dieses reine Labelmapping. Build und Clippy verwenden lediglich
`--jobs 2` zur Begrenzung gleichzeitiger Compilerjobs. Vollständige Logs und
exakte Command-/Exit-/Zeitrecords liegen in `.codex-tmp/net04_1c/checks/`.
Die1140 sonstigen Originalarchivdateien sind vor/nach SHA-256-identisch;
nach Entfernen nur der drei Labelarme ist die gesamte `observe.rs` exakt
byteidentisch mit `765147ca`. ITEM-SIZE-01- und NET-04.1b-Source-/Evidencefreezes
wurden samt jedem referenzierten Filehash unverändert kontrolliert.

Sourcefreeze `.codex-tmp/net04_1c/source.freeze.json` enthält ausschließlich
`observe.rs` und diesen Brief. Der zugehörige Evidencefreeze pinnt das
unveränderte Archiv, Preservationchecks und finale Checklogs. Nach dem
Codecheck wurde nur dieser Taskbrief ergänzt. Kein Appconsumer aktiviert
Itemmetadata oder macht dadurch Survival oder Plugins vollständig; unabhängiger
Rootreview, Commit und erneute Gesamt-CI bleiben bei Root.
