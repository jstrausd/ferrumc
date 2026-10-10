# GEN-05.5 — originale Biomeauswahl und vollständige Quartarrays

Status: vorbereitet; Worldproduktion nach unabhängig geprüfter REG-BIOME-01-
Integration und konkretem Autorentwurf. Ein Autor besitzt ausschließlich
`server/crates/ferrumc-world/**` und diesen Brief. Registry, Sim, Storage,
App, Protokoll, bestehende Reviews und Rootboards bleiben readonly.

## Tatsächlicher Ausgangspunkt

GEN-05.4 ist auf main `2a4a6cd4` integriert und vollständig CI-grün. Der
Density-/NoiseChunkconsumer liefert geprüfte Climatewerte und Materialarrays,
noch keine originale Biomeauswahl. REG-BIOME-01 liefert vollständige65
Ressourcendefinitionen und die tatsächlichen7593 Overworld-/5 Netherparameter.
Seine opaque lokalen Ressourcen-IDs sind ausdrücklich keine Wire-, Bootstrap-
oder Persistenzindizes. Die aktive App sendet weiterhin ausschließlich Plains.

Referenz: Minecraft1.21.8/772/4440, PaperBuild60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Tatsächliche Vanilla-Basis,
angewandte Quellen und relevante Feature-/Sourcepatches gemeinsam lesen:
Climate.ParameterList/RTree, MultiNoiseBiomeSource, TheEndBiomeSource,
NoiseBasedChunkGenerator.createBiomes, ChunkAccess/LevelChunkSection.fillBiomesFromNoise
und die wirkliche Preset-/BiomeSourcebindung. NoiseSettings allein bestimmen
keine BiomeSource. Der Autor grenzt unterstützte Builtins und fehlende
Custom-/Datapackproducer vor Code ausdrücklich ab.

## Bereits vorhandene unabhängige Originalevidenz

Root hat den tatsächlichen Originalparametercensus und RTree normal/-O
exportiert. Die bestehende Evidenz unverändert weiterverwenden; identische
Javaexports ohne neue Evidenzlücke sind nicht erforderlich.

- Parameterdatei `.codex-tmp/gen05_5-root-biome-census-normal/parameters.json`,
  SHA256 `4e9ee584071de6822b9e0e8030eb0666814a3c61b77f8e0ce611f9757508f971`.
- Censusfreeze `.codex-tmp/gen05_5-root-biome-census.freeze.json`,
  SHA256 `95deae4a95856a4a80cb3ec71c1c251e1852b6ecc0a1506d6aa811433e28729b`.
- Selectorfreeze `.codex-tmp/gen05_5-root-selector-oracle.freeze.json`,
  SHA256 `c7533ffed81a4b76e3ecca7108c67e09973315621745c5659800fbbb1df74cf4`.
  Die tatsächlichen Trees haben9112Nodes/7593Leaves und6/5;3287 Originalcalls.

Im Corpus unterscheiden sich177 Overworldbiomes von linearer Brutesuche,
41 zwischen Cold-/Forwardhistory und57 zwischen Cold-/Reversehistory.
Beim Target `[0,0,-10500,0,0,0]` ist das Coldresult deep_ocean, das Forwardresult
mushroom_fields. Stabile Originalsortierung, strikte Ties und behaltene
History sind Teil des Vertrags. Ein firstmatch oder beliebiger Tie-Breaker
ersetzt ihn nicht. Originale long-Arithmetik darf nicht versehentlich durch
Debugoverflowpanics oder saturierende Distanz verändert werden.

## Konkreter Runtimeentwurf vor Implementierung

Autor legt überprüfbare öffentliche APIs, Sourcepins, konkrete Daten-/Work-/
Bytecaps und tatsächliche Traversal-/Historyzustände vor. History wird explizit
besessen, ohne globale mutable Caches oder erfundene zeit-/scheduleunabhängige
Parität. Fehler-/Budgetverhalten muss den beobachtbaren Folgezustand eindeutig
definieren. Kein Parser, IO, neuer Dependency oder eine Allokation je Quart.
Treeaufbau und Scratch benötigen vor Allocation geprüfte fallible Grenzen.

Quartarrays enthalten vollständige lokale Ressourcenidentitäten. Originale
Abfolge: SectionY aufsteigend, danach localQuartX, localQuartY, localQuartZ.
Arrays und Historyfolge müssen aus dem tatsächlichen createBiomes-Consumer
stammen; separat sortierte Scalartargets beweisen keine Chunkparität. Negative
Chunk-/Sectionkoordinaten und alle tatsächlichen Höhenbereiche prüfen.
Ein partieller Arrayfehler darf keinen vollständig gültigen Output vortäuschen.

Endquelle und Builtinbindungen ausschließlich nach tatsächlicher Referenz
aufnehmen; fehlt dafür ein eigenständiger Producer, diesen konkret benennen.
Keine Plainsfallbacks für unbekannte Quellen, IDs, Nichtfinite oder Grenzen.
Der neue Computeconsumer verändert weder aktive Generatoridentität noch
Storage-/Clientpalette. Diese anschließende Aktivierung erhält eigene Tickets.

## Abnahme und Integration

Vollständige tatsächliche RTree-Strukturen und alle bestehenden Originalcalls
prüfen, einschließlich retained History und ursprünglicher Parameterordinale.
Neue vollständige originale Quartarrays über mehrere Seeds, negative/nachbar-
Chunks, Builtinbindungen und Höhenbereiche. Originalhelpers frisch kompilieren,
Artifact-/Source-/Helperorigins pinnen und neue Outputdaten normal/-O vergleichen.
Kein nachgeschriebener Referenzselector oder Rustroundtrip als Oracle.

Meaningful fehlerhafte/unsupported Inputs sowie konkrete Node-/Byte-/Workcaps
mit N/N-1 prüfen. Vorhandene Material-/Density-/Noise-/RNG-Fixtures bleiben
bytegleich. WholeWorld Debug/Release, strict Alltargetclippy/Fmt/Rustdoc,
exakter Filefreeze und unabhängiger Review. Surface, Carvers, Features,
Strukturen, natürliche Appwelt und Survival S1–S6 bleiben eigene Abnahmen.
