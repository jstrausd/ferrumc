# GEN-05.5 — unabhängige Biomselector- und Quartarrayabnahme

Status: **CLEAN** für den builtin Worldcomputeconsumer. Autor
`item_transfer_primitives`, Reviewer Root. Root hat Selectoraufbau/-suche,
Endquelle, Arrayconsumer, NoiseChunkphase und die tatsächliche Originalquelle
gelesen. Kein offener Codebefund bleibt im eingefrorenen Umfang.

Die Original-RTreeordnung, wrapping Java-long-Distanzen, strikten Ties und
explizit besessene History bleiben erhalten. Overworld besitzt9112 Nodes/
7593 Leaves, Nether6/5. Die Biomquelle ist explizit; NoiseSettings wählen sie
nicht implizit. Quartarrays nutzen vollständige Accessorhöhen: Overworld-64/384,
Nether/End0/256. End verwendet tatsächliche cached f64-Erosion mit transformierten
Zentren und Originalschwellen. Admission und ausgeführte Fehler bleiben
unterschiedlich: Ausführungsfehler verwerfen Output, erhalten frühere History
und poisonen Quelle/Noise; Reset muss explizit erfolgen.

Die echte `createBiomes`-Oracle verwendet Original-ProtoChunk, RandomState,
StructureManager und Beardifier. Der begrenzte LevelAccessor erlaubt den
tatsächlichen Candidate bei STRUCTURE_REFERENCES; andere Aufrufe verweigern.
Kein nachgeschriebener Java-Fillloop liefert erwartete Biome.156 komplette
Arrays enthalten88 MultiNoise-,64 End- und vier Fixedfälle; sechs echte
Javaidentitätsprüfungen belegen gemeinsam verwendete ParameterList-/RTreeobjekte
über normal/large_biomes/amplified.24 uninstrumentierte Generatorbaselines
stimmen bytegleich, alle fünf Endbiome und27 genaue Grenzproben sind enthalten.
3287 Originalselectoraufrufe und ganze Treestrukturen sind geprüft.

Root hat ein frisches `18acd2be1a247328b2a1518a3ba067c0c5c33e71`-Archiv mit
ausschließlich184 World-/Briefdateien geprüft: Debug und Release jeweils262
Tests, striktes Alltargetclippy, Workspacefmt und Worldrustdoc mit
`-D warnings` bestanden ohne Ausnahmen.184 Source-,94 Evidence- und175
Originalinputpins wurden vor/nach frisch geprüft.154 alte Worlddateien,
einschließlich aller40 alten Materialfixtures, bleiben bytegleich. Die
notwendige zusätzliche `biome_phase`-Testinitialisierung ist einzeln geprüft.
Root prüfte acht Originalrunner-Refusalguards jeweils unter Python normal und
`-O`; alle verweigern vor Output. Originalexport und Manifeste beider Modi
sind bytegleich geprüft; keine identische neue Exportserie war erforderlich.

Sourcefreeze `.codex-tmp/gen05_5-author-world.freeze.json`:
`06ff3569ce8e410a0b13ee019e43d358a40a307d8c91bbdfe6d6725244f3b4ef`.
Evidencefreeze: `dabc868a18598816baac3d2bb73b731fdd7a71b8f667ae0905e501e6ccfa9fda`.
Inputinventory: `774e776ef2becdb535374f6b0089a9146912c2dfefc8753185593495653cb432`.
Volltrace: `81a50ffc2fb92ebdc695e671ecf493c72564e4cc09c16e259f99e895f19fe1ef`;
Compactprojection: `33c3cec9c0937b659cddd026fe89cbbabe4e91abc6095c48f4a9b6adfd8caa78`.
Commands/Logs stehen unter `.codex-tmp/gen05_5-root-review/checks.json`.

Die tatsächlichen Nativecapacitywerte sind für Overworld1181618 retained Bytes,
47190 scratch Bytes und10845171 Buildwork; Nether874/20/184. Node128/Tree96
Bytes, maximal angefragter Output6144 Nativebytes. Diese Zahlen schließen
Allocatoroverhead und RSS aus; sie belegen keinen Vergleich von TPS oder
Speicherverbrauch mit einem vollständigen Paper-Server.

Produktive App und Netzwerk bleiben Flat-v1/Plains. Biomregistry-/Palette-
Verdrahtung, Zoom, Oberfläche, Carver, Features, Strukturen, Licht und natürlicher
Survivalfortschritt bleiben offen. Diese Computeabnahme ist keine vollständige
Weltgenerierung oder End-/Enderdrachenabnahme.
