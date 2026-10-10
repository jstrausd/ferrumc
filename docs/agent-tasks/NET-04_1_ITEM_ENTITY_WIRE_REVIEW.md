# NET-04.1 — unabhängiger Proto-/Originalevidenzreview

Ergebnis: **CLEAN**, Autor `feature_catalog`, Reviewer Root. Geprüft gegen
veröffentlichtes main `2a4a6cd4418b3da03bccd280b5590a360cd4e0a4` mit exakt
überlagerten Spec-/Proto-Dateien, getrennt von Registry-/Sim-WIP.
Keine Gameplayaktivierung oder Whole-Workspace-Abnahme dieses Producers.

## Geprüfter Stand

28 Source-/51 Evidencepins im Checkout und im eigenen frischen Archiv geprüft;
95 veröffentlichte Dependency-/Generator-/Fixturepins unabhängig kontrolliert.
Sourcefreeze SHA256
`643164606c08308384bead538a62a9487be0e1afcb1a9af121fe50ce5b138adf`,
Evidencefreeze SHA256
`1fc56daa788460e2f9f8dba3d0781139f2331edc622c36b966d139583e246a29`.
Nach den Checks bleiben alle Autorenpins unverändert. Spec ist der Producer;
Generateddateien sind tatsächlicher xtaskoutput, ohne Generatoränderung.

Rootfrischlauf: Debug/Release jeweils54 Tests, strict Alltargetclippy,
Cratefmt, Rustdoc mit `-D warnings` ohne Ausnahme und tatsächlicher
`cargo xtask generate --check` erfolgreich. Autorenbeleg für zwei bytegleiche
Generationen aller sechs Outputdateien ebenfalls gepinnt. Artefakte:
`.codex-tmp/net04_1-root-review/`, Runner
`.codex-tmp/net04_1-root-review.py`. Root schreibt keinen Runtimecode.

## Originalbelege und Vertragsprüfung

Finale echte normale/-O-Paperexports unabhängig unter Python `-O` geprüft:
121 aktuelle Artifactpins,42 Sourcepins, tatsächliche geladene Originalklassen,
frisch kompilierte/geladene Helperklasse und Helper-JAR, Inputkonfigurationen
und vollständige semantische Beobachtungen stimmen überein. Ausschließlich
Helperoriginpfade dokumentarisch entfernt. Fünf tatsächlich vorhandene
Packet-TSVs inklusive unabhängiger Prismarinebytes und beider adoptierter
Fixtures bytegleich:30 Pakete,14 Metadata/8 Motion/8 Take; SHA256
`66b71e900d5e81fd42874ace58cb60a9ea309fec23158797d95d06e53ad753de`.
Rootcheck: `.codex-tmp/net04_1-root-reference-check.json`.

Die finalen vollständigen Helper-/Runnerpfade wurden gelesen; Stream- und
GameProtocols-Dispatchcodecs sind Originale. DATA_ITEM hat tatsächlich Index8,
ITEM_STACK Serializer7, Obfuscationbinding OVERSIZED. Originale negative
Metadatakontrollen und alle Cursorpositionen bleiben erhalten. Die separat
gehashten Postconfigs unterscheiden sich in server.properties und spigot.yml;
keine bytegleiche Postconfigbehauptung oder semantische Packetnormalisierung.

Spec und tatsächliche Generated-APIs geprüft:0x5c/0x5e/0x75, unpräfixierte
Rawtail, signed VarInts und bereits quantisierte i16. Wiretests verwenden
Originalbytes direkt und über Dispatch, prüfen Scalartruncation, vorhandene
VarIntfehler, Nachlauf und tatsächliche N/N+1-Cap-/Readerposition. Das
263168-Byte-Limit wird vor Tailread/Kopie durchgesetzt. Leere, abgeschnittene,
unbekannte Serializer-/Slot- und Nachlauftails sind bewusst opaque erhalten;
diese Hülle beansprucht keine originale semantische Metadataadmission.

## Integrationsgrenze

Der getrennte Netconsumer NET-04.1b muss die beiden exhaustiven Matches und
tatsächliche Writer-/Backpressureordnung schließen. .1 und .1b werden als
getrennte Cratecommits gemeinsam veröffentlicht, damit main kompilierbar
bleibt. Itemslot-/Sanitizerbuilder, Viewer-/Sessiontracking, Appdriver,
Physics/Merge/Javaevents und Survival S1–S6 bleiben offen. Dieser Review
belegt Paketbytes und lokale Admission; keine Performanceverbesserung.
