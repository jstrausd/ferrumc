# GEN-05.4 — Aquifer, große Erzadern und Noise-Materialausgabe

Status: abhängiges Folgeticket vorbereitet, noch nicht vergeben oder
implementiert. Voraussetzung ist ein unabhängig CLEAN integrierter
GEN-05.3-NoiseChunkconsumer. Genau ein Autor besitzt anschließend
`server/crates/ferrumc-world/**` und diesen Brief; keine neuen Dependencies,
App-/Sim-/Storage-/Registryänderungen oder Gitmutation durch den Autor.

## Konkretes Ergebnis

Der bereits besessene NoiseChunkdriver erhält einen tatsächlichen Materialsink:
originale Konstruktor-Materialdichte im selben eigenen FunctionContext,
Aquifer, bei `null` OreVeinifier, bei erneutem `null` Settingsdefaultblock.
Er liefert alle Blockstates und Fluid-Postprocessingmarkierungen eines
frischen vollständigen 16×16-Noisechunks für alle sieben Builtinsettings.
Das ist eine reine Worldausgabe vor Surface/Carvers/Features, noch kein aktiv
spielbarer Naturalgenerator. Keine neue allgemeine Material-Engine.

Öffentliche Methode und Ergebnisnamen werden vor Umsetzung konkretisiert.
Der Consumer bleibt synchron und ausschließlich besessen, mit privaten
Feldern. Material- und Dichtedriver dürfen denselben Sessionlauf nicht zweimal
verwenden; dieselbe Fresh/Running/Complete/Poisoned-/Resetauthority und derselbe
Workmeter gelten. Kein budgetloser Aquifer-, Noise- oder RNGunterpfad, keine
pro-Blockallokation und keine Ausgabe eines unvollständigen Chunks.

## Referenz und Rootbefunde

Minecraft1.21.8/772, PaperBuild60 Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Neue Chunkpfade behalten GEN-05.3s
ausdrückliche Paper-Endpolicy und private PaperBuild60-Samplingbasis.
Tatsächliche Paper-`nextDouble`-Methoden in XoroshiroRandomSource und
BitRandomSource runden über `l2f/fmul/f2d`; Vanilla verwendet `l2d/dmul`.
Die private .3-RNG-/Noiseseam wird für den Materialsink weiterverwendet,
einschließlich tatsächlich benötigter Aquifer-/Ore-Zufallsdraws. Keine globale
Rundung fertiger Noiseergebnisse. GEN-05.2s öffentliche Vanilla-Scalar-/Climate-
und Random-/Noiseverträge sowie bisherige Fixtures bleiben unverändert.

Root hat die vollständigen angewandten `Aquifer`- und `OreVeinifier`-Quellen,
die tatsächlichen Materialconstructor-/getInterpolatedStatepfade,
`NoiseBasedChunkGenerator.createFluidPicker/doFill`, den zugehörigen
Generator-Sourcepatch sowie die relevanten `OverworldBiomeBuilder`,
`DimensionType`- und `Mth`-Callees gelesen. Der Autor liest Originalbasis,
angewandte Methoden und relevante Source-/Featurepatches vollständig und
führt eine genaue Source-/Calleematrix mit Hashes. Die bisherigen Quellen-
befunde sind keine Rust-, Material- oder gestartete Paper-Serverabnahme.

Verbindliche Befunde für den konkreten Producer:

- Globalfluidpicker: Lava bis `y < min(-54, seaLevel)`, ansonsten das genaue
  Settingsfluid mit seinem Sealevel. `FluidStatus.at(y)` verwendet streng
  `y < fluidLevel`, ansonsten originales Air. Flags stammen aus den echten
  Settings; Nether/End nicht anhand des Ressourcennamens besonders behandeln.
- Disabled-Aquifer: positive Substance ergibt `null`, ansonsten Globalfluid;
  Fluidupdate immer false. Enabled-Aquifer führt die tatsächlichen zwölf
  Centerkandidaten mit X außen, Y dann Z aus. Gleichheit benutzt die originale
  `>=`-Tieordnung für vier nächste Centers, nicht eine andere Distanzsortierung.
- Eigene bounded Location-/Fluidstatusarrays mit originaler Grid-/Indexform,
  initialem `Long.MAX_VALUE`-Locationsentinel und absent Fluidstatus. Tatsächliche
  Factory-at-Streams und die drei bounded-int-Draws10/9/10 bleiben erhalten.
  Cachemiss-/Hit- und Requestordnung sind Teil der Abnahme.
- Pressurebranches prüfen Originalfluidstate, Höhen und Similarity; Barrier-
  Noise wird nur in den tatsächlich verlangten Branches und höchstens einmal
  pro Pressureauswertung über den originalen MutableDouble-Sentinel gelesen.
  Eigener Context und fremde SinglePointcontexts bleiben unterschieden.
- Alle13 Preliminary-Surfaceoffsets einschließlich -3Chunks X bleiben erhalten.
  DeepDark liest Erosion vor Depth mit originalem Shortcircuit. Floodedness,
  Spread und Lava benutzen ihre tatsächlichen skalierten SinglePointadressen,
  Float-/Doublekonstanten, clampedMap-/quantizeordnung und strengen Grenzen.
- `Integer.MAX_VALUE` ist ein tatsächlicher Preliminarysentinel. Dessen `+8`
  und nachfolgende originale i32-Arithmetik wrappen wie Java; keine stille
  checked-overflow-Verweigerung oder ersetzende Nullhöhe an dieser Stelle.
- Materialabfragen am Weltrand benötigen einen kleinen privaten Noisehalo:
  Centerjitter und Surfaceoffsets können öffentliche ±30M überschreiten.
  Der Autor leitet aus den originalen Zugriffen eine konkrete bounded
  Haloextent ab, prüft sie vor Allocation/Driverstart und testet alle vier
  Randrichtungen. Öffentliche .2/.3-Domains werden dabei nicht aufgeweicht.
  Eine symmetrische128-Blockextent ist eine zu prüfende sichere Ausgangsbasis,
  keine bereits abgenommene APIentscheidung.
- OreVeinifier: tatsächlicher Toggle bestimmt Copper/Ironsign, jeweilige
  Ybereiche0..50/-60..-8, Randabfall, drei Floatdraws in ihren echten Branches,
  Ridged-/Gapshortcircuit sowie ursprüngliche Floatthresholds. Keine normalen
  Oreplacements oder Deepslatekonvertierung aus Namensheuristiken ergänzen.
- Endgültiger Zustand entsteht nach Aquifer→Ore→Default. Fluidmarkierung
  entspricht erst dann `shouldScheduleFluidUpdate && !fluidState.isEmpty()`
  für tatsächlich geschriebenen Non-Airstate. Stale Flag-/Fluidannahmen
  dürfen nicht als Postprocessingbitmap veröffentlicht werden.

## Daten, Ressourcen und Fehler

Defaultblock/-fluid und alle festen Aquifer-/Oreblocks werden über originale
Blockstatecodec-/Registrydaten exakt auf gültige `BlockStateId` abgebildet;
kein stiller AIRfallback bei unbekanntem Namen/Property/State. Der Autor
konkretisiert vor Produktion die kleine private Daten-/Resolverseam innerhalb
der vorhandenen World-/Registryabhängigkeiten. Kein anderer Crateproducer
wird ohne ausdrücklichen Besitzabgleich mitimplementiert.

Materialausgabe enthält Seed, Settingsresource, konkrete Noisegeometrie und
ChunkPos; ursprüngliche Y/X/Z-Sampleindexordnung bleibt erhalten. Eigene
Aquiferarrays, Outputstates, Bitmap, Scratch und vorhandene .3Caches werden
getrennt checked budgetiert. Der tatsächliche Constructor-/Runtimecensus aus
dem gepinnten JAR begründet alle Caps vor Umsetzung. Keine globalen Caches,
IO-/Thread-/DBhandles, ganzen Weltclones oder erfundenen RSS-/TPS-Zusagen.

Typisierte Shape-/Position-/State-/Budget-/Nonfinite-/Lifecyclefehler vor
Veröffentlichung; echte Ausführungsfehler poisonen die Session. Ein fehlerhafter
Reset ersetzt die vorherige Session nicht. Keine Semantikvereinfachung, um
einen Ressourcenfehler zu umgehen. Zusätzliche Produktiondateien werden mit
Root abgeglichen, bestehende öffentliche numerical Verträge bleiben erhalten.

## Unabhängiger Planningcensus vor Vergabe

Root hat einen getrennten Java21-Helper ausgeführt, der originale Paper-
Konstruktoren, `createFluidPicker`, den vollständigen NoiseChunkdriver und
tatsächlich `getInterpolatedState` aufruft. Keine nachgeschriebene Material-
oder Aquiferformel. **49 vollständige Materialloops:7Settings×7Seeds** bei
Chunk0/0, zusammen3.325.952 States mit originalen Fluidflags. Alle120 Paper-
Artefakte vor/nach gehasht,16 benutzte Original-Class-CodeSources und der
frisch kompilierte Helperorigin geprüft. Normale/-O-Runs liefern bytegleiche
JSONL und Manifests.

Ergebnisse: Overworld/LargeBiomes/Amplified verwenden je315 Location- und
Statusslots; höchstens291 Locations und184 Statusslots waren belegt.
Preliminarycache maximal185 Columns. Die übrigen vier Settings verwenden
den tatsächlichen Disabled-Aquifer und hatten keine solchen Slots/Columns.
Die neuen Semantik-/Halo-/Branchtests bleiben erforderlich;49 Nearorigin-
Loops sind keine vollständige Materialparität oder Runtimeabnahme von Rust.

Artefakte unter `.codex-tmp/gen05_4-root-planning{,-normal,-optimized}/`,
sechs-Dateien-Freeze `.codex-tmp/gen05_4-root-planning.freeze.json`.
JSONL27.226Bytes, SHA256
`c6171590ddd59008fb1afcc6fb45954930a5d931693a2b48188d0afbb93a24ec`;
Manifest29.283Bytes, SHA256
`6096f558f46139acb61ea4eb26734083e992040ecf714354f119b340d0573a01`.
Dieser Census begründet zukünftige Ressourcenadmission und beansprucht weder
Rust-/Gameplay-/Serverboot-/Drop-in- noch Performanceabnahme.

## Tatsächliche Abnahme

Root hat vor Vergabe zusätzlich die vollständige .3-Matrix als tatsächliche
Paper-Materialreferenz ausgeführt:126 Chunks/7.077.888 Records mit originalen
Blockstate-IDs und Fluidflags.84 Nearcases (7Settings×2Seeds×6Chunks) plus42
ferne Endcases (7Seeds×6Chunks). Echte `createFluidPicker`-/
`getInterpolatedState`-Methoden und vollständiger NoiseChunkdriver; weder
Aquifer noch Ore-/Materialformeln wurden nachgeschrieben. Originale25
Class-CodeSources, frischer Helperorigin und alle120 Artefaktpins vor/nach
geprüft. Normal/-O-Reexport vollständig bytegleich, separate genaue
Matrixprüfung und lückenlose Recordbereiche.

Artefakte `.codex-tmp/gen05_4-root-material-goldens{,-normal,-optimized}/`.
State-/Flagdatei35.389.440Bytes, SHA256
`e27a497fbbed2adb8060e3bc9556f6341a0ecb6ab5eb0ef565e70570113ed384`;
JSONL32.562Bytes, SHA256
`7782ac995e1adaa7c58c23478101521d633764babb4b5ec46e7fa4124fcd3455`.
Acht-Dateien-Freeze `.codex-tmp/gen05_4-root-material-goldens.freeze.json`,
SHA256 `bb9f9f888d11e93c17ff099ac8e5452c6d76fc9c4c2a42ba4319fdf6541b0f3f`.
Dies ist eine Paperreferenz, noch kein Rustmaterial- oder Gameplaypass.

Der neue gepinnte Helper ruft echte `NoiseChunk.getInterpolatedState`,
Aquifer-/Ore-/Globalfluidmethoden aus dem Original-/Paper-JAR auf. Keine
nachgeschriebene Javareferenzformel. Vollständige Materialstate-/Fluidflag-
Arrays für alle7Settings, asymmetrische Seeds, positive/negative Nachbarchunks
und entfernte Endchunks; dieselben .3Numericalfälle sind die Ausgangsmatrix.
Actual-oracle Cache-/RNG-/Noisebranchtraces ergänzen die vollständigen Arrays.

Eigene Grenzfälle erfassen Substance±0, Fluidheightgleichheit, Centerdistanz-
ties, Similarity-/Barriergrenzen, Water/Lavacontact, DeepDarkshortcircuit,
MAX-Surfacewrap, alle Orethresholds mit nextafter, RNGconsumption und false/
true Postprocessingflags. Erfolgreiche komplette Chunks werden in frischen,
resetten und umgeordneten Sessions erneut verglichen; alle Source-/Origin-
und Klassenpins, normal/-O-Reexport und negative Herkunfts-/Hashguards gelten.

Rustprüfungen: exakte Budgetgrenze/N−1 an Constructor, Aquifermiss/hit,
Preliminary/Pressure, Orebranches und Blockoutput; getrennte Cache-/Output-
Grenzen, malformed Settingsstate, unveröffentlichter Teiloutput und Poison/
Reset. Ganze Worldcrate Debug/Release, strict Alltargetclippy/Fmt/Rustdoc,
unveränderte alte Inputs und exakter Freeze. Ein unabhängiger Reviewer prüft
den tatsächlichen Producer vor Rootintegration.

## Unmittelbare Folgearbeit

Danach folgen tatsächliche Biome/Surface/Bedrock und die Worldgeometrie-/
Chunkadoption mit neuer ausdrücklicher Generatoridentität. Carvers, normale
Feature-/Oreplacements, Vegetation, Structures und Licht bleiben eigene
Verbraucher, ebenso aktive Nether-/Enddimensionen und sämtliche Mobs/Bosse.
Weder eine Materialarrayausgabe noch eine Numericalabnahme schließt die
Survivalstrecke oder den angeforderten Drop-in-Vertrag ab.
