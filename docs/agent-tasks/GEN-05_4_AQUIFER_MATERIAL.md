# GEN-05.4 — Aquifer, große Erzadern und Noise-Materialausgabe

Status: GEN-05.3 unabhängig CLEAN und mit `e7565c38` integriert. Autor
`/root/item_transfer_primitives` besitzt exklusiv
`server/crates/ferrumc-world/**` und diesen Brief; keine neuen Dependencies,
App-/Sim-/Storage-/Registryänderungen oder Gitmutation durch den Autor.
Root hat den vollständigen konkreten Entwurf SHA256
`0e907adbae119f18485f0e3d31b7d5e01362fdcac848db39f997fbaa671d48cd`
und alle26 Base-/Appliedsourcepins unabhängig geprüft und zur gezielten
Runtimeumsetzung freigegeben. Runtime und vollständige Referenzconsumer sind
umgesetzt; alle finalen Wholechecks sind grün. Der Stand geht eingefroren an
das unabhängige Review. CLEAN und Rootintegration stehen aus.

## Konkretes Ergebnis

Der bereits besessene NoiseChunkdriver erhält einen tatsächlichen Materialsink:
originale Konstruktor-Materialdichte im selben eigenen FunctionContext,
Aquifer, bei `null` OreVeinifier, bei erneutem `null` Settingsdefaultblock.
Er liefert alle Blockstates und Fluid-Postprocessingmarkierungen eines
frischen vollständigen 16×16-Noisechunks für alle sieben Builtinsettings.
Das ist eine reine Worldausgabe vor Surface/Carvers/Features, noch kein aktiv
spielbarer Naturalgenerator. Keine neue allgemeine Material-Engine.

`NoiseChunkSession::sample_chunk_materials` und `ChunkMaterialSamples` sind
umgesetzt, mit dem unten konkretisierten öffentlichen Vertrag.
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

## Konkreter Implementierungsentwurf zur Rootprüfung

### Öffentlicher Vertrag und gemeinsamer Driver

`NoiseChunkSession::sample_chunk_materials(&mut self)` liefert
`Result<ChunkMaterialSamples, NoiseChunkError>`. Das Ergebnis besitzt ausschließlich
private Felder: `WorldSeed`, `NoiseSettingsResourceId`, `ChunkPos`, `NoiseGeometry`,
`Box<[BlockStateId]>` und `Box<[u64]>` für die Fluidmarkierungen. Accessors heißen
`seed`, `resource`, `chunk_pos`, `geometry`, `block_states`, `block_state_at`,
`fluid_update_bitmap` und `should_schedule_fluid_update`. Die States behalten
exakt den vorhandenen Index `((y-minY)*16+localX)*16+localZ`. Bit i bezeichnet
denselben Stateindex; unbenutzte hohe Bits im letzten Wort sind null. Die
Positionsaccessors geben außerhalb der eigenen Geometrie `None` zurück.

Der vorhandene vollständige Driver bekommt einen privaten konkreten Sinkenum
mit Density- und Materialvariante. Er behält initialize/firstslice/advance/
select/Y/X/Z/material/swap/stop und die vollständige originale Loopreihenfolge.
Der bestehende `sample_material` bleibt der checked Running/ZUpdated-Eintritt
für die Konstruktor-Materialdichte. Der neue Materialsink ruft unmittelbar
danach Aquifer, bei `None` den originalen Oreconsumer, bei erneutem `None` den
Settingsdefault auf. Seine letztendliche Fluidmarkierung prüft sowohl das echte
Aquiferflag als auch den tatsächlich ausgewählten Non-Airstate mit Fluid.
Keine zweite Densityfahrt, kein Callbackframework und keine pro-Blockallokation.

Material- und Densityfahrt teilen die vorhandene Freshauthority: jede beendet
den einen erlaubten vollständigen Lauf. Nach Complete verweigert auch die
jeweils andere Methode; Reset ermöglicht wieder genau eine Fahrt. Preliminary
und Climate bleiben gemäß .3 in Fresh/Complete verfügbar. Der Materialsink
besitzt seine Aquiferarrays über den gesamten Lauf; nach erfolgreicher Ausgabe
werden diese privaten temporären Materialcaches freigegeben. Das Ergebnis
behält nur States und Fluidbitmap. Eine öffentliche spätere Carver-/Aquifer-
Wiederverwendung ist außerhalb dieses Tickets.

### Constructorchronologie, Meter und Fehler

Die Materialvorbereitung wird erst vom neuen öffentlichen Materialaufruf
ausgeführt. Die bestehende .3-Sessionkonstruktion und ihr Dichte-/Climatevertrag
werden nicht nachträglich mit Materialallokationen oder RNGdraws verändert.
Originaler Aquiferconstructor initialisiert/sealed nur feste Arrays; er zieht
keine Centerstreams. Orecreate hält Functions/Factory, zieht ebenfalls nichts.
Die zeitlich verschobene Vorbereitung verändert daher keine tatsächliche
RNG-/Densityrequestchronologie. Lazy Center-/Fluidstatusfüllung erfolgt weiterhin
erst am tatsächlich verlangten Originalbranch, in Originalreihenfolge.

Ein Lifecyclefehler bei verweigertem Fresh-Eintritt lässt Session und Meter
unverändert. Nach erfolgreichem Eintritt setzt der Materialaufruf Running;
sämtliche Preparation-/Ausführungsfehler setzen anschließend Poisoned. Das gilt
auch für Resolver-, Halo-, Cache-, Output- und Budgetfehler während Preparation.
Alle Shapes/Counts/Bytes/Domains werden vor der zugehörigen Allokation und vor
Driverstart geprüft. Ein Fehler veröffentlicht weder States noch Bitmap.
Atomic Reset baut weiterhin einen vollständigen .3-Kandidaten; bei Fehler bleiben
alte Wrapper/Counters/Surface/Meter/Lifecycle vollständig erhalten. Bei Erfolg
ist Materialpreparation wieder ungeschehen und Fresh.

Der vorhandene Meter läuft ohne lokale Budgets weiter. Zusätzliche Charges
liegen vor: Vorbereitungsschritt, bounded Resolverinputscan/Lookup/Properties,
jeder Cacheinitialisierung, Aquiferentry, Globalfluidrequest, jedem der zwölf
Centerkandidaten, Location-/Statuslookup und Cachepublish, Factory-at, jedem
tatsächlich ausgeführten RNGdraw einschließlich jeder rejection-loop-Iteration,
Surfaceoffset, DeepDark/Pressure/Surface-/Fluidtypebranch und finalem State-/
Fluidbitwrite. Bestehende Density-/Surface-/Noiseroots verwenden unverändert
denselben Meter. Barrier wird innerhalb des originalen gemeinsamen Pressure-
Sentinels höchstens einmal angefordert. Kein Vorabberechnen von übersprungenen
Erosion/Depth/Ridged/Gap/RNGbranches. N/N−1 prüft auch Cachepublish nach einem
vollständig berechneten Wert und finalen Outputwrite.

Neue Fehler sind klassifizierte `NoiseChunkErrorKind`-Varianten für ungültigen
Materialstate und eine konkrete private Materialadresse außerhalb ihrer
admittierten Domain; neue `NoiseChunkLimit`-Varianten trennen Materialresolver,
Aquiferslots/-bytes, States und Fluidbitmap. Vorhandene Work-, Graph-, Lifecycle-
und Nonfinitefehler bleiben erhalten. Keine ungeprüften Unknownstate-/AIR-
Fallbacks oder erfolgreiche Fortsetzung nach nichtendlichem Densitychild.

### Factoryherkunft und numerische Originaldetails

Es werden ausschließlich die bereits besessenen `BoundRouter.aquifer_random`
und `.ore_random` verwendet. Sie stammen aus RandomStates originalem
Positionalfactoryfork und den Namen `minecraft:aquifer`/`minecraft:ore`.
Aquifer leitet `at(gridX,gridY,gridZ)` ab und zieht10/9/10; Ore leitet am tatsächlichen
Block ab und zieht seine1..3 Floats ausschließlich in ihren Originalbranches.
Ein crate-private `WorldgenRandomSource`-Meterpfad charged jede tatsächliche
bounded-positive rejectioniteration vor RNGmutation; öffentliche Randommethoden
und sämtliche bisherigen Vanilla-Fixtures bleiben unverändert. Aquifer und Ore
benötigen keine neuen `nextDouble`-Draws. Ihre normalen Noisefunktionen nutzen
weiterhin .3s private tatsächlich geprüfte Paper-offsetbasis.

Alle F32konstanten werden zunächst als f32 dargestellt und dann nach f64
gewidened, wenn das Original sie im Doubleausdruck verwendet: insbesondere
DeepDark−0.225F/0.9F, Vein0.4F/0.6F/0.1F/0.3F und Gap−0.3F. Floatdrawvergleiche
bleiben f32. Double−0.3/0.8/−0.8/0.4 in Aquifer behalten ihre Doublebits.
`clampedMap` bleibt `clampedLerp(outputMin,outputMax,inverseLerp(...))`,
`quantize` bleibt Java-floor(value/factor) mit anschließender i32-Multiplikation.
MAX-Preliminarysentinel+8 und anschließende i32-Arithmetik wrappen ausdrücklich.
Centerdistanz-/Similaritytieordnung bleibt das originale `>=`; PackedBlockPos
behält26/12/26 Bits und den ursprünglichen Long.MAX_VALUE-Locationsentinel.

### Private Stateauflösung und zulässige Daten

Keine neue Registrylane und keine eigene IDtabelle: die vorhandene vollständige
`ferrumc_registry::block_state::{block_metadata,block_default_state,compute_state_id}`
API liefert die gepinnten gültigen States. Der nur fünf Namen unterstützende
Rootwrapper `default_block_state_id` wird hier nicht verwendet. Der private
Worldresolver liest die statischen `RouterMetadata.default_block/default_fluid`
Slices mit dem vorhandenen bounded JSONlexer und verlangt `Name` sowie optional
`Properties`; unbekannte Felder/Blocks/Properties/Values oder falsche JSONtypen
verweigern. Ausgelassene Properties erben den originalen Defaultstate.
Inputcap4096Bytes,32Properties und die vorhandenen String-/JSONcaps gelten vor
Produktion; die Inputs sind bereits bounded Originaldefinition-Slices.
Die temporäre Parserallokation ist durch4096Sourcebytes und vorhandene128KiB/
4096Node-Lexercaps begrenzt und wird vor Parsing durch den Inputscan metered.
Unknown-/malformed-Properties werden strenger als das originale
`lenientOptionalFieldOf("Properties")` verweigert; dies ist checked Policy,
keine behauptete Fehlereingabeparität mit Java.

AIR/WATER/LAVA, CopperOre/RawCopper/Granite und DeepslateIronOre/RawIron/Tuff
werden einmal während Preparation über dieselbe bestehende Registry aufgelöst.
Die sieben echten Settingsdefaultblöcke und -fluids werden exakt geprüft.
Die geschlossene Materialausgabe kann nur die aufgelösten Settingsdefaults und
diese festen Originalstates auswählen. Fluidklassifikation wird für die
wirklich möglichen Air/Water/Lava/defaultfluidstates versiegelt, nicht aus
Namensfragmenten oder Blocksolidity erraten. Unbekannte hypothetische Fluids
verweigern Preparation. Originaldefaults/Propertycodec und die RawstateIDs
werden mit tatsächlichen gepinnten JARwerten geprüft.

### Halo und konkrete Foreigndomains

Sei p ein ausgegebener Block. Aus `floorDiv(p−5,16)+{0,1}` und Jitter0..9
folgt für Center X/Z genau eine sichere Deltaextent[−20,+20]; aus
`floorDiv(y+1,12)+{−1,0,1}` und Jitter0..8 folgt[−22,+21]. Die13 Surfaceoffsets
liegen in X[−48,+16]/Z[−16,+16]; Quartfloor fügt[−3,0] hinzu. Deshalb können
Preliminaryadressen relativ p in X[−71,+36]/Z[−39,+36] liegen. Die private
symmetrische128-Extent ist größer als jede dieser tatsächlich hergeleiteten
Grenzen. Vor Allocation/Driver werden Chunkorigin, positive Corner, alle
Extremadressen und Geometrie durch i64checked Ableitung geprüft.

Nur private Materialqueries dürfen physische X/Z in
`[−30_000_128,+30_000_128]` verwenden. Public .2/.3 Positionchecks bleiben
unverändert. Center-Y wird zusätzlich auf `[minY−22,topY+20]` begrenzt, da
der letzte Outputblock topY−1 ist; diese admitted Centers müssen vor dem Lauf
in PackedBlockPos' unverfälschte Yrange[−2048,+2047] passen. Alle echten sieben
Settings erfüllen diese Voraussetzung. Private hypothetische Geometrymutationen
außerhalb der packedrange verweigern statt falsche Centerpositionen zu cachen.
Preliminary scannt weiterhin seine vollständige ursprüngliche Geometry inkltop.

Erosion/Depth/Floodedness verwenden den checked physischen Centercontext.
Spread verwendet ausschließlich daraus abgeleitet
`(floorDiv(x,16),floorDiv(y,40),floorDiv(z,16))`: X/Z
`[−1_875_008,+1_875_008]`, Y die floordiv40-Endpunkte der admittierten Center-
Yrange. Lava verwendet `(floorDiv(x,64),floorDiv(y,40),floorDiv(z,64))`:
X/Z `[−468_752,+468_752]`, dieselbe Yrange. Diese skalierten Foreignadressen
werden unabhängig von einer relativen Chunkhalo geprüft; ein far-chunk geteilter
Wert ist kein physischer Nachbar. Barrier/Ore verwenden den echten owncontext.
MAX-Surfacewrap beeinflusst Globalfluidhöhen und ursprüngliche Arithmetik,
ändert aber keine fremde Noiseadresse in eine ungeprüfte i32-Extremaquery.

### Separate Counts und Bytes

| Kategorie | Checked Admission vor Allokation |
| --- | --- |
| Materialstates | Höchstens131072×4Bytes =512KiB; die originale height×256-Shape und vollständige Y/X/Z-Ordnung. |
| Fluidbitmap | Höchstens2048×8Bytes =16KiB; unabhängig von States und Writebitmap. |
| Exactly-once-Writebitmap | Vorhandene höchstens2048×8Bytes =16KiB; niemals als Fluidflag wiederverwenden. |
| AquiferLocation/Status | Originale Gridindexform, höchstens512Slots; tatsächliche Builtins315. Getrennt maximal12KiB für `i64`-Locations und `Option<FluidStatus>` mit tatsächlichem `size_of`; beide Counts identisch. |
| Preparationresolver | Höchstens4096Bytes je statischem Settingsstateinput,32Properties; zusätzliche Parserlimits oben. Keine RetainedJSONtrees, keine Blockloopallokation. |
| Vorhandene .3Resourcen | Key/Graph/Wrapper/8MiBCache/4MiBScratch/2048Surfacecolumns unverändert und getrennt; alle Workpfade teilen268435456 maxUnits. |

Materialsinkmetadata und vier nächste Centers/Pressure-Sentinel sind feste kleine
Werte; keine per-BlockVecs. Keine neuen Dependencies, globalen Caches oder
Performancezusagen. Neue Runtimefiles: `density/chunk/material.rs` und private
`material/{aquifer,ore,blocks}.rs`; Driver/Chunkexports und der private RNGpfad
werden gezielt erweitert. Tests/Originalrunners/Fixtures liegen im Worldscope.

### Tatsächlich gelesene Quellen und noch ausstehende Abnahme

Vollständig gelesen: Aquifer einschließlich constructor/computeSubstance/
pressure/getStatus/computeFluid/surface/spread/lava, OreVeinifier einschließlich
aller Branches und VeinTypes; NoiseChunkconstructor/MaterialRuleList/
getInterpolatedState und vollständiger Driver; GeneratorcreateFluidPicker/
doFill sowie der gesamte Generator-Sourcepatch; DeepDarkcallee; Mthfloor/
inverseLerp/map/clampedMap/clampedLerp/quantize, DimensionTypekonstanten und
BlockPospack/unpack; StateHoldercodec/BlockStatecodec sowie Propertydefault-
auflösung; tatsächliche beide Factory-at und bounded-int/Floatmethoden.
Die Sourcepatches für Generator und BlockState und die relevanten Feature-
patchabschnitte0012(BlockPos) und0016(StateHolderoptimierung) werden zusammen
mit Basis und angewandtem Code geprüft. Keine Aquifer-/Ore-/NoiseChunk-
Source-/Featurepatchänderung wurde im gepinnten Checkout gefunden.

Base liegt unter Paper-`paper-server/.gradle/caches/paperweight/mache/base/sources/`,
Applied unter `paper-server/src/minecraft/java/`; Präfix `net/minecraft/`:

| Originaldatei | Base SHA256 | Applied SHA256 |
| --- | --- | --- |
| world/level/levelgen/Aquifer.java | `6c55f04707514c491173a408f448025e06a9db08c7e9dd7c04377d9ac5e1315a` | gleich |
| world/level/levelgen/OreVeinifier.java | `f68ed59d78a1621791de4bcd187cbb7ea4ed0a89a11bd6c293282d021d2371f1` | gleich |
| world/level/levelgen/NoiseChunk.java | `e3c872cf3f6a15a063d255f1188296ea841ef012b01959d35ca7e0012d97c863` | gleich |
| world/level/levelgen/NoiseBasedChunkGenerator.java | `1aee1be9952d347975d2a464da06ee330dc0980aba1a979ceca9d18e6d89dc71` | `2b586e7fca9059b65d214f33f6b84fcc7ae3de90285ad7cd9c8a56edc319f2d8` |
| world/level/biome/OverworldBiomeBuilder.java | `cf35eb3b0e9ebc945e9199a641d1d87919725eada4cdc38a8171c0417aeb13b3` | gleich |
| world/level/dimension/DimensionType.java | `627b841fdedb30b0ea35576baa67dadcc884180c1f083fda4cf42701a474ec92` | gleich |
| util/Mth.java | `b16d2c4c6b1979c2999480952e4e4d624a67755204d96de909006dd99ff51a5c` | gleich |
| core/BlockPos.java | `5c7bae8a163f077e1c4016cc86ce500e5f4a50ec73f434e8886b6a2e6e91a4c7` | `3721abe51c6206f9bf9d98cb1df543578e7f8a250f25fa4562fb42f2ca034843` |
| world/level/block/state/BlockState.java | `82d7b1ff7ae2f76dbe595da8c55be5bb3f22f16ed2a2c09cf62463eb0e05c37b` | `9f47781fe7e32828d00d55de58586bcca792ccc7fab6b122dbb81a622ebb2c33` |
| world/level/block/state/StateHolder.java | `950c134ea03ad199db2b9287ed48f5af8155c2c1cbd516bb24d2629146c8cc65` | `c3decaef0dbb20fa69e152bc2d2dfe976c59a8cdc07137afc3d38375dfb198cf` |
| world/level/levelgen/XoroshiroRandomSource.java | `76a1af8439d57038d3e80ea41dc7b38ca3d9cbfe3a8db8bd87982b5aa363e70f` | gleich |
| world/level/levelgen/LegacyRandomSource.java | `f05787a1b23d609499ec99e403ff267fecbc500f71bcc15311fce14a3a6a6c02` | gleich |
| world/level/levelgen/BitRandomSource.java | `726528fb348082c248218899112a2d6fba1a1897dad719aa41939eff643ff2c1` | gleich |

Ganze relevante Patchdateipins (gelesene Abschnitte oben, nicht behauptet alle
übrigen Moonrisemechaniken geprüft): Generator-Sourcepatch
`031f165c23c23c603320cde05ecee4ce5656a5762a4e21abda2bdff9fb31e04a`,
BlockState-Sourcepatch
`41cb04435d2d247c92d833997305cd3787db12839cea0b9618697cb50ca76e6e`,
Feature0012
`05917edf3c780e02fbea9b667afbeaf0bb09100aea10baf53da7f92909371dd0`,
Feature0016
`267e01eaeb0873893c09a903bae3d9880de7a98c757b59fca03bc827de06297b`.

Diese Sourceprüfung und dieser Entwurf sind keine Runtimeabnahme. Die bereits
unabhängig eingefrorenen126 vollständigen Paperarrays werden ohne neue Formel-
oracle oder Reexport des alten .3Bitscorpus wiederverwendet. Dazu kommen echte
JARbranch-/Requesttraces, Guard-/Origin-/normal/−O-Nachweise sowie die oben
definierten malformed/halo/threshold/tie/sentinel/N/N−1/Poison/Resettests.
Erst nach erfolgreicher kompletter State-/Fluidflagmatrix und unabhängigem
Review wird GEN-05.4 als CLEAN zur Rootintegration gemeldet.

## Eingefrorener Autorstand für das unabhängige Review

Umgesetzt sind die vier privaten Runtimefiles `material.rs` und
`material/{aquifer,ore,blocks}.rs`, der eine konkrete Density-/Materialsink im
bestehenden Driver, das besessene öffentliche `ChunkMaterialSamples` und der
crate-private metered bounded-int-Pfad. Materialvorbereitung ist aufgeschoben,
die alte .3-Sessionkonstruktion bleibt ohne zusätzliche Materialdraws. Das
Productiondispatch verwendet konkrete Factory-/Fluid-/Statewerte; sämtliche
deklarierten Float-/Point-/Picker-/Context-/Tracehooks sind `cfg(test)`.

Tatsächlich bestanden:

| Prüfung | Eingefrorener Umfang |
| --- | --- |
| Komplette Paper-Materialausgabe | Alle126 Chunks und7.077.888 States/Flags exakt; sieben Settings, asymmetrische Seeds und signed Near-/Far-Endmatrix. |
| Unabhängige finite Endpunkte |499:Pressure69/Surface151/Spread70/Fluid66/Ore107/Disabled36; Results/Rawbits/Requestcontexts/Factory-/Floatchronologie exakt. |
| Aquifer-Addendum |42 finite Enabled27/Ties6/Fluidscan6/Fluidcache3; vollständige Resultate, Cache-Snapshots vor/nach jedem Call und Density/RNG/Globalfluid-Interleaving exakt. |
| Zusätzliche Nonfinitepolicy |15 echte Java-Ergebnisproben mit NaN/+Inf/−Inf für Substance/Floodedness/Spread/Preliminary/Barrier. Rust verweigert typisiert mit ursprünglichen Rawbits und unveröffentlichtem Status/Surfacecache; keine Java-Ergebnisparität behauptet. |
| Endpunkt-Workgrenzen |Alle541 finiten Fälle erfolgreich mit exaktN, typed Fehler beiN−1 und unveränderter vollständiger bzw. Prefix-Tracechronologie. |
| Session-/Publikationsverträge |VolloutputN/N−1; Preparation/State-/Bitmapwrites getrennt; Poison/atomarer Reset; Resolvermalformed/Unknownfluids; vier Weltkanten/private scaled-/packedDomains; Aquifermiss/hit/failed Cachepublish; metered RNGrejections. |
| Ergebnisbesitz und Reihenfolge |Für alle sieben Settings zwei gleichzeitig bestehende Sessions desselben Routers, Neighborlauf zwischen erstem Lauf und Reset, rückwärts angeforderte Chunks desselben Routers, Ergebnis nach Drop von Router/Session gültig. |
| WholeWorld Debug und Release |Je205 Unit +46 Integration =251 Tests;0 Fehler/0 Ignored. |
| Werkzeuge |Strict Alltargetclippy mit `-D warnings`, `cargo fmt -p ferrumc-world --check`, Rustdoc ohne Dependencies mit `RUSTDOCFLAGS=-D warnings`, alle grün. Offline/locked, `-j 2`, eigener Scratchtarget. |

Die unabhängigen Originalhelpers enthalten keine nachgeschriebene Aquifer-/
Pressure-/Ore-/Surface-/Center-Ranking-/RNG-Erwartungsformel. Adoptierte Java-
Helpers und deklarierte Inputs bleiben bytegleich. Die beiden Rootfreezes für
Endpunkte und Addendum haben SHA256
`ad582f936bb33dfb507504f38b86249f979ea695ca8ab91e1d638ef038aee714`
bzw.
`3e7529d90389b90332729b67bbb45d287622ce9e22787186ba23ae3ff2c4b41c`.
JSONL499:475.091Bytes, SHA256
`4ae88195e5fb1008794f0f4ac2b4ee69b6fc1a63f0902b52f00baf9ad21ff4ab`.
JSONL57:873.181Bytes, SHA256
`850ccb051798c73317bd26bc22d40ee60b8c29ef6eb1c1403fd40feeeb022d87`.

Owned-Reexport normal/−O reproduziert Rootmaterialarrays, beide Rootbranch-
JSONLs, tatsächliche ClassOrigins und alle Helperclasses bytegleich. Die
jeweiligen Ownedmanifests sind normal/−O identisch; ihre SHA256s sind
`57b46d9f74305d7263c230cd646fef3780933eca81a4dcdd29c2ddce2acc4f66`
(Arrays), `e6441992c50ca861ee2dcfd4a554e6cfa38027de38e6152ec2b236ba73f0342c`
(499) und `04cbfdb7f5654a8959c88188a7908084a4a15d6e8a3e8dc29656b4a9175598f0`
(57). Alle19/8/10 Ownedguards bestehen in beiden Pythonmodi. Feste Helper-/
Input-/Builder-/JDKpins,26 Base-/Appliedsourcepins,120 Paperartefaktpins,
Pre-/Posthashes, tatsächliche CodeSources und Helper-shadow-/Environment-
Verweigerungen gelten. Nichtendliche Javaergebnisse werden nicht normalisiert.

Gegen `.codex-tmp/gen05_3-world.freeze.json` bleiben109/118 Dateien bytegleich,
einschließlich aller24 alten Fixtures und28 Referencefiles. Nur neun zuvor
konkretisierte Runtime-/Testhook-/Dokudateien sind geändert; es gibt keine
unerwartete Änderung oder Dependency-/Cargo.toml-/Cargo.lockmutation.
Vergleich: `.codex-tmp/gen05_4-prior-contract-compare.json`.
Referenzadoption und normal/−O-Vergleiche:
`.codex-tmp/gen05_4-frozen-reference-adoption-compare.json`,
`.codex-tmp/gen05_4-owned-final-reexport-compare.json`,
`.codex-tmp/gen05_4-owned-branches-final-compare.json`,
`.codex-tmp/gen05_4-owned-addendum-final-compare.json`.

Finale Logs heißen `.codex-tmp/gen05_4-final-{debug,release,clippy,fmt,rustdoc}.log`;
World-/Brief-Authorfreeze und zugehörige Checknachweise liegen unter
`.codex-tmp/gen05_4-author-world.freeze.json` und
`.codex-tmp/gen05_4-author-finalchecks.json`. Nach Freeze editiert der Autor nur
auf konkrete Reviewfindings. Root verwaltet unabhängige Reviewentscheidung,
Integration und Push; der Autor hat kein Git verwendet.

Diese Abnahme liefert ausschließlich frische Materialarrays vor
Surface/Carvers/Features. Aktive Naturwelt, Biome-/Surface-/Chunkadoption,
Nether/Endbetrieb, Survival, Java-/Paperplugins, komplette Vanilla-/Paper-
Parität und RAM/TPSverbesserungen werden damit nicht behauptet. Der separate
GEN-05.5-Scratchentwurf konkretisiert als nächste Producerstrecke vollständige
Biomeparameter → originalen historyhaltigen RTree-Selector → Quartarray;
keine .5Runtime ist durch dieses Ticket aktiviert.
