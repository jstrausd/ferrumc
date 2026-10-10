# GEN-05 — Originale Densitydefinitionen, Seedrouter und Chunkinterpolation

Status: **GEN-05.1/.2 unabhängig CLEAN integriert; GEN-05.3 implementiert, lokale Abnahme grün, finale unabhängige Runtimeprüfung läuft**.
Compiler und echte Originalcodec-Strukturabnahme fertig; [Rootreview](GEN-05_1_DENSITY_COMPILER_REVIEW.md)
mit Debug/Release je177 Tests, drei eigenen Zusatzprobes und frischem
bytegleichem Original-JAR-Reexport grün. GEN-05.2 hat die genehmigte
Seedbindung und direkte Auswertung vollständig geliefert und unabhängig
CLEAN als `290e5343` integriert. GEN-05.3 folgt seinem freigegebenen privaten
Paper-Chunkvertrag. GEN-04.3 ist unabhängig CLEAN und als
`245134eb` integriert. REG-WORLDGEN001 ist CLEAN als `555ee2aa` integriert; sein unveränderter
APIproducer ist die geprüfte Basis dieser Worldcompilation.

## Ziel, Scope und notwendige Reihenfolge

Die vorhandenen Originalprimitives werden jetzt zum tatsächlichen Terrainpfad
verbunden. Genau drei aufeinander aufbauende Worldverträge sind nötig:

1. **GEN-05.1 Definitionparser/Compiler:** alle35 Originaldensitydefinitionen
   und sämtliche sieben Originalrouter schema- und referenzgerecht zu privaten,
   begrenzten Graphen kompilieren. Dieser Schritt bietet keine Samplemethode.
2. **GEN-05.2 Seedbindung und direkte Auswertung:** originale RandomState-
   Noisebindung, Factorylowering/Bounds, f64-Arithmetik und f32-Splineadapter;
   direkte Point-/Climateproben für alle sieben Router gegen das echte JAR.
3. **GEN-05.3 NoiseChunk-Auswertung:** originale Wrapper, Array-/Cellkontexte
   und Interpolation in einer exklusiv besessenen Chunk-Session. Direkte
   Routerproben bleiben ein anderer Kontext als interpolierte Chunkwerte.

Alle drei gehören ausschließlich `server/crates/ferrumc-world/**` und diesem
Taskbrief. Keine neue Crate, Eventschicht, Registryparser, Runtime-I/O, globale
mutable Registry, App-/Sim-/Storage-/Config-/Lockfileänderung. Root gibt jeden
konkreten Produktionsschritt frei und organisiert Review/Integration. Es gibt
keine zusätzlichen allgemeinen Frameworktickets zwischen diesen Schritten.
Definition- und Routercompiler sind kein Naturalgenerator und erzeugen keine
Fake-Nullnoise oder vermeintlich fertige deaktivierte Chunkwrapper.

## Freigegebener Registryproducer und konkrete Consumerbindung

Stabile `ferrumc_registry::worldgen`-APIs, unabhängig abgenommen:

- Opaque `NoiseResourceId`, `DensityFunctionResourceId`, `NoiseSettingsResourceId`:
  `.from_name(&str) -> Option<Self>`, `.resource_location() -> &'static str`,
  `.definition() -> &'static OriginalDefinition`. Kein Rawconstructor, Wire-ID
  oder Default; Familien sind verschiedene Typen.
- Noise zusätzlich `.parameters() -> &'static NoiseParameters` mit
  `.first_octave() -> i32`, `.amplitudes() -> &'static [f64]` aus `from_bits`.
- `noise_resources()`, `density_function_resources()`, `noise_settings_resources()`:
  allocfreie exact-size/double-ended/fused Iteration in canonical sort order.
- `OriginalDefinition.json() -> &'static str` enthält den exakten Fixture-
  Rawvalueslice. `.stats()` meldet Bytes/Nodes/Depth/Dependencyoccurrences;
  `.dependencies()` enthält unique sortierte typisierte Referenzen: Noise,
  DensityFunction, Block, Biome, DensityFunctionType, SurfaceRuleType,
  SurfaceConditionType und synthetischer RandomName bleiben getrennt.

Der Producer stammt aus unverändertem
`docs/fixtures/minecraft-1.21.8-original/worldgen.json`,60 Noiseparameter,
35 Densitydefinitionen und7 Settings; SHA256
`bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30`.
World verlässt sich nicht auf einen Rawjsonslice als Runtimegraph: es prüft
Shapes, Referenzrollen und kombinierte Graphstruktur selbst. Producerbytes/JSONnodes/-depth und die tatsächlich kompilierten
Dependencykategorien (Density/Noise/Densitytype) werden verglichen. Deferred
Surface-Noiseverweise erhalten einen eigenen bounded Refcensus; vollständige
Surface/Block/Biome/Spawnsemantik und Gesamtrefoccurrencecensus bleiben der
integrierten Producerprüfung und ihren späteren konkreten Consumern zugeordnet.
Settingsreferenzen zu Surface/Block/Biome sind keine Densitynodes.

## Tatsächlich vermessener Graphbestand

Die korrigierte Inventarisierung zählt die eigene private IRdarstellung aus
**echten Original-JAR-Codecprojektionen**: eine Vertex je Inlinevorkommen,
benannte Definitionen geteilt, beide Reference/Direct-`HolderHolder`-Arten
explizit sowie raw Reference/Direct-Splinecoordinateholders als separate IR-
Vertices. Koordinatenholder sind keine zusätzlichen `HolderHolder`-Objekte;
beide Rollen bleiben im IR verschieden. Splineconstants/-multipoints gehören
zu derselben joint Graphprüfung. Keine Factoryfolding/structural Deduplication.
Combineddepth zählt jede Vertex inklusive Holder, Coordinate und Spline; ein
Leaf hat Tiefe1. Dies ist keine Heap-/Sampling-/TPSmessung.

| Settingsrouter | Density inkl. Holder/Coordinate | Splinevertices | Gesamtvertices / Edges | Kombinierte Tiefe | Splinepunkte / Marker |
| --- | ---: | ---: | ---: | ---: | ---: |
| `overworld` | 658 | 433 | 1091 / 1217 | 87 | 430 / 30 |
| `large_biomes` | 658 | 433 | 1091 / 1217 | 87 | 430 / 30 |
| `amplified` | 658 | 433 | 1091 / 1217 | 87 | 430 / 30 |
| `nether` | 82 | 0 | 82 / 69 | 22 | 0 / 5 |
| `caves` | 82 | 0 | 82 / 69 | 22 | 0 / 5 |
| `floating_islands` | 82 | 0 | 82 / 69 | 22 | 0 / 5 |
| `end` | 100 | 0 | 100 / 85 | 24 | 0 / 3 |

Union aller35 Definitionen und7 Router: **3102 Vertices,3435 Edges, Tiefe87**;
1299 Splinevertices/1290 Punkte und64 Markeroccurrences. Die frühere syntaktische
Inventur2460/2793/52 ließ Direct-Holderhelper und raw Koordinatenholder aus.
Das war eine unzutreffende Codecannahme, keine akzeptierte Optimierung. Die
korrigierten echten Codecs zeigen außerdem: `derivative` ist REQUIRED; allein
der separate originale Splinebuilder bietet ein Default0. Root hat folglich die
combinedDepthgrenze96 statt64 ausdrücklich freigegeben. Die Rustabnahme prüft
96/97 an tatsächlich typgerechten Density/Spline/Coordinate/Holderketten.

Neun Terrain-Splineprofile: normal/large/amplified je offset254Nodes/253Punkte,
factor135/134, jaggedness44/43, lokale Splinetiefe5, höchstens11 Punkte je
Multipoint und4 verschiedene Coordinates; separate GEN-04.3-Caps bleiben gültig.

Reproduktion ab Reporoot:
`python3 server/crates/ferrumc-world/tests/reference/measure_density_graph.py`.
Dessen Eingang ist die gepinnte tatsächliche JARprojection; vollständige aktuelle
Messung in `.codex-tmp/gen05-codec-inventory-final.json`. Rust vergleicht die
reachable Routerzahlen unabhängig beim vollständigen IRprojectionstest.

## GEN-05.1 — freigegebener Compilerumfang

Freigegebene Oberfläche in `ferrumc_world::density`:

```text
RouterTemplate::compile_builtin(NoiseSettingsResourceId)
    -> Result<RouterTemplate, DensityCompileError>
DensityTemplate::compile_definition(DensityFunctionResourceId)
    -> Result<DensityTemplate, DensityCompileError>
```

Beide besitzen private, geprüfte IR und immutable `Arc`-Sharing. Öffentliche
Getter melden Originalresource, Nodes/Edges/Depth, benannte15 Routerfelder,
Noisereferenzen, Spline-/Wrappermengen und gültige Noisegeometrie. Es gibt keine
öffentlichen unchecked Nodefelder, frei konstruierbaren Programmhandles,
Defaultsettings, Samplemethode oder Nullnoise. Kein Inputslice von Caller-
Fixtures bleibt erhalten; Originaldefinitionen selbst sind producerseitig static.

Beide Compilerentries validieren alle35 Densityresources einschließlich ungenutzter
Teilgraphen; Routercompilation nimmt zusätzlich den ausgewählten Settingsrouter auf. Alle sieben Router werden
über dieselbe API abgenommen. Eine Namedreference ist ein Holder, kein sofort
ersetztes Inlineconstant; Marker-, Holder- und originale Factoryherkunft werden
für die nachfolgenden Seed-/Climate-/NoiseChunk-Visitorpässe erhalten.
Splinecoordinates sind echte Densityedges; Zyklen werden über die gemeinsame
Density-/Spline-/Ressourcenclosure erkannt, nicht in getrennten DAGprüfungen.

Runtimeparser ausschließlich World: Root hat den kleinen eigenen bounded
JSONlexer statt des ursprünglichen DeserializeSeed-Vorschlags freigegeben.
Originale Numeric-Tokens bleiben borrowed während der begrenzten Admission;
FLOAT/DOUBLE werden unabhängig direkt nach f32/f64 geparst. INT erhält exakte
Decimal-Truncation und wrapping32 ohne f64Zwischenschritt. Serde_json übernimmt
nur bounded String-/Escapedecoding; keine unbounded `Value`-Materialisierung
vor Admission. Bytecheck vor Parser, Nodes/Depth/Containers vor Descension/
Arenaerweiterung, decoded Duplicatekeys, volle JSONsyntax und trailing Reject.
Der128KiB-Inputcap begrenzt temporären escaped Decoderbedarf; der256Byte-
Decodedstringcap allein behauptet keine256Byte-Decoderheapobergrenze.

WorldCargo erhält nur das bereits gelockte `serde_json` (float_roundtrip);
unnötige direkte `serde`-Dependency wurde entfernt. Implementer editiert kein
Lockfile. Root nahm eine einzige bestehende `serde_json`-Dependencyzeile im
Worldpackage auf; Versionen/Sources/Checksums bleiben unverändert. RootlockSHA
`87a7baae3407eb4b7dcb60cb10096ed073cb66660dc3752a784ae8973d6a7da2`.

Compiler verarbeitet die tatsächlichen Originalcodecs:

- Densityholderhelper: Zahl/Object oder Resourcename werden IMMER zu einem
  `HolderHolder`; Inlineconstant ist kein sofort erkannter Factoryconstant.
  Splinecoordinate verwendet dagegen raw `DensityFunction.CODEC`-Holder.
- `clamp.input` ist ausdrücklich **DIRECT_CODEC**, also Zahl/Object und kein
  Resource-string. Shift `argument` ist **NoiseHolder**, kein Densityholder.
- Splinecoordinate ist Densityholder; Splinevalue ist f32-Constant oder
  verschachtelter Multipoint, nicht beliebige Densityfunction. Die originalen
  FLOAT-Codecs lesen Decimal direkt als f32; das ist getrennt vom späteren
  Compute-f64→f32-Cast der Densitycoordinate nachzuweisen.
- Bareconstant / `type:constant`, default namespace / explicit namespace und
  originale Zahlentyp-/Codecgrenzen werden bewusst unterschieden. Unknown-
  Felder/Typen/Shapes/Referenzen sind Errors, kein stiller Null-/Overworldfallback.
- Settings: exakt15 Routerfelder, `noise`-Geometrie, `legacy_random_source`,
  sea level und Aquifer/Oreveinflags. Defaultblock/-fluid, `surface_rule` und
  `spawn_target` werden als begrenzte separate Originalpayloads behalten;
  ihre vollständige Material-/Surface-/Spawncompilation bleibt offen.

| Compilergrenze | Freigegebener Wert / Motivation |
| --- | --- |
| Definitioninput | Producervertrag:128KiB,4096 JSONnodes,Depth32,256 Containerentries, Strings256Byte,2048 Refoccurrences/512 unique Dependencies. Worldprüfungen bleiben eigenständig. |
| Gesamtinput je Compilation | 2MiB über tatsächlich gelesene Definitionsslices, bevor Compilation; cold path ohne I/O. |
| IRvertices / Edges | 4096 /8192 einschließlich Density, Holder und Spline. Gemessene komplette Union3102/3435 hat Reserve; jede private Arenaappend ist checked. |
| Kombinierte Tiefe | 96 inklusive Definitionreferenzen, Direct-/Referenceholder und Splinecoordinatewechsel; gemessener Max87. Iterative Zyklus-/Depthprüfung, kein unchecked rekursives Aliasfollow. |
| Benannte Ressourcen | Ausschließlich35 gepinnte Densitys,60 gepinnte Noises,7 gepinnte Settings. IDs typisiert; keine Public-Raw-ID oder dynamische unbekannte Registry. |
| Splinegraphs | Bestehende1024Nodes/64Coords/64Punkte je Node/16384Punkte/Depth32; diese zusätzliche lokale Kappe ersetzt keine gemeinsame Cycle-/Depthprüfung. |

Private bounded Zwischenarenen können während Parsing/Compilation entstehen;
erst nach vollständiger Prüfung wird ein öffentlicher Immutablegraph freigegeben.
Errors enthalten Resource/Field bzw. lokale Edge-/Nodeposition: invalid JSON,
Duplicatefield, malformed number/name, unknown node/noise/density, wrong-family/
Direct-vs-Holder shape, Dependencydrift, invalid geometry/point order, Cycle,
Depth/Byte/Node/Edgecap. Keine partielle öffentliche Compilation escapes.

### Explizite Policy und Codecgrenzen

Publicscope sind ausschließlich die gepinnten builtins; kein allgemein nutzbarer
Datapack-/JSONcompiler wird angeboten. Beide APIs veröffentlichen checked
Metadaten, keine Root-/Nodehandles und keine Seed-/Samplemethoden. Originale
Normal-noisehalter können auch Inlineparameter tragen; diese dynamischen
Holders sind außerhalb dieses geschlossenen Named-builtin-Scope und werden
abgewiesen, statt implizit einer Noiseresource gleichgesetzt zu werden.

Unknownfields, finite FLOAT/DOUBLE und streng aufsteigende finite f32locations
sind zusätzliche FerrumC-Admissionpolicy: echte Originalcodecs ignorieren Extras,
akzeptieren FloatInfinity und sogar unsortierte/duplicate Splinepoints. Die
separate Java-Buildervalidierung darf hier nicht als Codecguard ausgegeben
werden. Reversed Clamp-/Rangebounds und equal/reversed Gradientendpoints
bleiben originalzulässig. Spline-`derivative` ist nach echter Codecprobe required.

INT ist nicht auf syntaktische Integers tightened: tatsächliche Gson
`Number.intValue` truncated fractions und wrapped übergroße Integer **vor**
Codec-Rangeguards. `1e309`/`1e1000` werden als INT0 angenommen; globale
f64-finiteJSONchecks wären falsch und wurden entfernt. DOUBLE/FLOAT dagegen
prüfen finite erst im jeweiligen typedConsumer. Die gebundelten Gson2.11-
`NumberLimits` wurden vollständig per originalem `javap -p -c` geprüft:
BigDecimalfallback hat Tokenlänge≤10000 und abs(scale)<10000. Integer-/Longfastpaths
können in gültiger JSONsyntax diese Caps nicht überschreiten (keine leading
zeros, höchstens20 Integerzeichen, scale0); der begrenzte Decimalalgorithmus
liefert für sie dieselben wrapping32bits. Für Guardfehler meldet Rust typed
Numeric/LimitError statt Java-`NumberFormatException`. Looparbeit: jedes
Mantissadigit höchstens einmal; nach32 trailing Multiplikationsnullen ist modulo
2^32 bereits0. Exponent/Token/Scale werden vor diesem Loop geprüft.

Raw Defaultblock/-fluid, SurfaceRule und Spawntarget sind producergeprüfte,
byte-/treebegrenzte static Payloads. GEN-05.1 führt deren Verhalten nicht aus
und beansprucht keine vollständige `NoiseGeneratorSettings`-Consumerfähigkeit.
Factoryprobes zeigen directConstant→MulOrAdd separat; tatsächliche JSON-
Binaryinputs bleiben HolderHolder und daher Ap2 bis zum späteren Visitorpass.

## Alle tatsächlich benötigten Nodetypen

Die35 Densitys plus7 Router benutzen28 serialisierte Typen. Alle32 Original-
Codecformen werden in derselben Implementierung unterstützt; keine neuen
generischen Frameworkstufen für die vier zusätzlichen Formen.

| Gruppe | Originalformen und notwendiger Vertrag |
| --- | --- |
| Constant / Holder | Bare f64 oder `constant`, `HolderHolder`/Resourceref; finite Originalcodecwerte bis±1,000,000, Signedzero erhalten. |
| Zweifach | `add`, `mul`, `min`, `max`; originale Factory erzeugt Ap2 oder MulOrAdd, keine beliebigen Constantfolds/Reassociation. |
| Unary | `abs`, `square`, `cube`, `half_negative`, `quarter_negative`, `squeeze`; originale Diagnostics und Operationorder. |
| Auswahl / Gradient | `clamp`, `range_choice`, `y_clamped_gradient`; inclusive lower/exclusive upper und originale clampedMap/Lerpsemantik. |
| Noise | `noise`, `shift_a`, `shift_b`, `shift`, `shifted_noise`, `weird_scaled_sampler`, `old_blended_noise`, `end_islands`; echte Sampler und Seeds, keine Placeholdernoise. |
| Spline | `spline`, verschachtelte Cubicnodes und Densitycoordinates; f64→f32 Coordinate und f32→f64 Ergebnis. |
| Markers | `interpolated`, `flat_cache`, `cache_2d`, `cache_once`, `cache_all_in_cell`; immutable Descriptors bis GEN-05.3, keine finale No-op-Chunkimplementation. |
| Terrainkontext | `blend_alpha`, `blend_offset`, `blend_density`, `beardifier`; direkte Originalsinglepointprobe ist empty Blender/Beardifiermarker. Chunkkontext muss seine frische unblended/no-structure Voraussetzung explizit erklären. |

`constant`-Object, generisches `shift`, `cache_all_in_cell` und `beardifier`
sind die vier fehlenden serialisierten Formen im gemessenen Bestand; Bare-
constants kommen selbstverständlich vor. NoiseChunk erzeugt zusätzlich selbst
CacheAllInCell und Beardifieraddition, also braucht diese Unterstützung ohnehin
der unmittelbar folgende Consumer.

## GEN-05.2 — Seedbindung, Ownership und Sampling

`RouterTemplate::bind_seed(WorldSeed) -> Result<SeededRouter, DensityBindError>`
ist erst nach erfolgreicher .1-Abnahme verfügbar. `SeededRouter` besitzt einen
immutable Arcgraph, originale Bounds und immutable Noise-/Blended-/Endsampler.
Clone teilt diese Daten; jeder spätere Chunk hält ausschließlich seine eigenen
Wrapper-/Interpolationsarrays. Sampling verbraucht keine RNGdraws und allozieren
nur explizite Construction/Sessions bzw. vorab angelegte Batchscratchs.

RandomState wird nach Original verbunden:

1. Settingsalgorithm (Overworld/large/amplified xoroshiro, nether/caves/
   floating/end legacy) mit allen signed64 Seedbits erstellen und einmal
   `forkPositional`. Aquifer-/Orekeys heißen canonical `minecraft:aquifer` /
   `minecraft:ore`, jeweils name-derived stream, dann positional fork.
2. `Noises.instantiate` benutzt den **canonical Noise-Resourcename**, nicht
   einen Graphslot. Ein immutable Sampler je semantisch gleicher Noisbindung;
   benannte Ressourcen dürfen Graphsharing besitzen, keine allgemeine
   Memoization der ausgewerteten Densitywerte.
3. Legacy-Temperatur verwendet `Legacy(levelSeed+0)` und Legacyvegetation
   `Legacy(levelSeed+1)` mit originalen `(-7,[1,1])`; Addition wrappt Java-i64.
   Legacyshift (Noises.SHIFT heißt tatsächlich `minecraft:offset`) verwendet
   name-derived Stream und `Normal(0,[0])`. Das ist kein allgemeiner Nullnoise-
   Fallback, sondern genau der originale dokumentierte Specialcase.
4. Blendednoise wird bei legacy aus `Legacy(levelSeed)`, sonst aus canonical
   `minecraft:terrain` rekonstruiert. EndIsland verwendet unabhängiges Legacy
   mit17292 Consume +Simplex. Reorder der benannten Anfragen darf Ergebnisse
   nicht verändern; error Construction verändert keinen vorherigen Seedrouter.
5. Originaler NoiseWiringvisitor rekonstruiert Factory-/Splinebounds nach den
   echten gebundenen Samplerbounds. Climatevisitor strippt Holder/Marker im
   eigenen View. Originaler SurfaceSystem-Constructor lädt weitere Noises und
   Claybands, liefert aber keinen hier implementierten Surfaceconsumer;
   namederivation beeinflusst andere Noises nicht durch Requestorder.

Factory-/Scalarsemantik ist kritisch: `Ap2.mul` überspringt rechts bei±0 und
liefert+0. **MulOrAdd**, sobald eine originale konstante Seite erkannt wurde,
wertet sein input aus und rechnet `value*constant`/`value+constant`; ein pauschaler
Nullfold wäre falsch. MIN/MAX benutzen originale Boundshortcircuits und Java
min/max mit Signedzero/NaN, auch wenn eine Diagnose mathematisch nicht enger
optimiert werden könnte. RangeChoice wertet nur den gewählten Child. Kein
globales topologisches Evaluate-all mit zusätzlichen Childrequests.

Ein gemeinsamer eigener Workmeter zählt Densityentry, Noiseinvocation,
Splineentry/-callback/-binsearch und konkrete EndIsland-Looparbeit **vor**
Ausführung, auch bei Density→Spline→Density. Vorschlag Pointbudget
1..=1,048,576; gemeinsames maximales Batchbudget67,108,864 für vorab begrenzte
4096 Kontexte. Die genaue Unitdefinition bleibt öffentlich und wird mit
erfasster erfolgreicher/abgebrochener Requestspur geprüft. Ein separater
MAX_SPLINE-Budget je Innercall erfüllt diesen Gesamtvertrag nicht.
Dafür wird die bestehende Splineevaluation nach ihrer Integration um einen
crate-internen shared-state/metred Pfad erweitert; die bisherige öffentliche
SplineAPI und ihre Unit-/Bit-/Reihenfolgeverträge bleiben erhalten.

Dasselbe gilt für Noisebounds: Originaljaggednoise hat xzScale1500. Bei
legitimen±30,000,000 Blocks sind45,000,000,000 Noiseinput keine Fehlkonfiguration,
obwohl die öffentlichen GEN-04-CallerCaps enger sind. Der Worldconsumer braucht
einen privaten checked-finitedensity Samplerpfad mit originalem per-octave Wrap,
statt die öffentlichen Caps auf diese Zwischenwerte anzuwenden. Keine Public-
unchecked Freigabe: Rootcontext ist typed `BlockPos` im expliziten Weltbereich,
gesamte Scales/Bindings sind geprüfte Builtins, finite Zwischenwerte und
fixed/capped Octaveloops bleiben geprüft. Original-JAR-Proben an±30M plus
angrenzenden Wrapgrenzen sind Pflicht; bestehende öffentliche Noisecaps bleiben
als eigene APIverträge getestet. Kein kapbedingtes Geländeverschwinden.

Direkte API: `sample_field(RouterField, BlockPos, WorkBudget) -> Result<f64,...>`
und benannte Densityprobe; Climate liefert sechs Werte in Originalreihenfolge
und f64→f32→`coord*10000f32`→Java-long-Quantisierung, kein fertiger Biomesearch.
Plainpoint ist ausdrücklich der Originalsinglepoint/empty-Blenderkontext;
Markeridentity darin behauptet keine NoiseChunk-Parität.

## GEN-05.3 — originale Chunkwrapper und Interpolation

Die Session besitzt private Markerbinding-/Wrapperarrays und Grapharc,
gebunden an Seedrouter, `ChunkPos`, geprüfte Noisegeometrie und expliziten
fresh/unblended/no-structure Kontext. Nicht vorhandenes Blending oder Structure-
Beardifier darf in späterer Aktivierung kein stiller0/identity-Fallback werden.
Vorgeschlagene Caps:128 Wrapperbindings und8MiB eigene Sample-/Cachearrays je
Session; geometric preflight vor Allocation. Builtins benötigen höchstens30
serialisierte Wrappervorkommen plus die fest erzeugten Materialwrapper.
Keine globalen Locks oder geteilten mutable Caches.

| Wrapper / Originaltrigger | Konkreter Consumervertrag |
| --- | --- |
| Interpolated | Zwei `(cellCountXZ+1)*(cellCountY+1)` f64-Slices, originale Cornerorder. Normale Updates lerpen Y→X→Z; beim Cellfill gilt originale `Mth.lerp3`-Reihenfolge. Gleiche Mathematik allein beweist keine Bitgleichheit beider Pfade. |
| FlatCache | Quartgrid inklusive äußeren Rand:5×5 für16-Blockchunk, precompute bei y0. Negative Quartcoords über arithmetic shift; außerhalbGrid direkter Child im tatsächlichen Kontext. |
| Cache2D | Genau letzter x/z-Key, ignoriert y; `fillArray` delegiert direkt. Kein beliebiger Mehrfachkeycache oder globale y-Cacheannahme. |
| CacheOnce | Originale scalar-/arraycounter, `arrayIndex` und Arraylength; fremder Plaincontext delegiert. Counters und Fieldfill-Reihenfolge verändern Cachebedarf. |
| CacheAllInCell | Höchstens `cellWidth²*cellHeight`, Fillorder absteigend Y, dann X, dann Z. Fremder Kontext delegiert; in eigener Session außerhalbInterpolation Fehler. Out-of-cell delegiert. |

Origin-/Wrappergleichheit muss den OriginalmapAll-/HashMap-Vertrag respektieren,
einschließlich structural equality der Originalrecords und gesonderter Identität
von Array-/Blended-/Splineobjekten. Einzigartige IRoccurrences dürfen nicht
blind zu beliebigen globalen Caches verschmolzen werden. Das echte Java-
Wrapperoracle muss Sharing, Counter-/Fill-/Requesttraces mitprüfen.

Private Loopstate erzwingt initialize→advanceX→selectYZ→updateY→updateX→updateZ,
swapSlices und stop, mit original absteigendem CellY. LayoutOverworld:
cellWidth4,cellHeight8,4×48×4 Zellen; End/floating: width8,height4. Eine öffentliche
bounded `sample_chunk_densities`-Operation führt diese Reihenfolge intern aus;
kein beliebig manipulierbarer public Counter-/Cellstate. Fehler invalidiert die
Session bis explizitem Reset; kein Rollbackclaim für bereits veränderte private
Caches. Calleroutput wird erst nach vollständigem bounded Scratchsuccess
veröffentlicht. Für den Chunklauf vorgeschlagen maximal268,435,456 gemeinsame
Workunits; auf den echten goldenen Loops konkret zu prüfen, keine TPSgarantie.

Noch fehlen in dieser Session **Aquifer.computeSubstance/OreVeinifier/Material-
RuleList und Blockausgaben**. Preliminary-surface-densityscan mit Quartalignment,
cellHeightsteps, Threshold0.390625 und Integer.MAX_VALUE-Sentinel kann im gleichen
Consumer umgesetzt werden, da Aquifer/Surface ihn unmittelbar benötigen.
Damit sind originale interpolierte Densityarrays bereit, keine fertigen Chunks.

## Tatsächlich gelesene Originalquellen und Paperabweichungen

Pin: Minecraft1.21.8/protocol772/DataVersion4440;
PaperBuild60/Commit `29c8822d90899c89d2689338e81a98f690bcba12`.
Vollständig gelesen: DensityFunction(119Zeilen), DensityFunctions(1365),
RandomState(154), NoiseRouter(68), NoiseRouterData(527), NoiseGeneratorSettings
(181), NoiseSettings(59), NoiseChunk(790), NoiseBasedChunkGenerator(428), Noises
(82), WorldgenRandom(89), SurfaceSystem(324), Beardifier(168), Blender(387)
und QuartPos. Vollständige verwendete Mth-Sqrt/Clamp/ClampedLerp/InverseLerp/
Lerp2/Lerp3/ClampedMap-Callees und Climate.Sampler-/Target-/Quantisierungspfade
gelesen; bestehende Noise-/Splinecallees bereits in GEN-04 vollständig geprüft.

Appliedquellen liegen unter
`.codex-tmp/paper-1.21.8-reference/paper-server/src/minecraft/java/`, Basequellen
unter `.gradle/caches/paperweight/mache/base/sources/` desselben Serverroots.
Vollständige DensityFunctions- und NoiseBasedChunkGenerator-Sourcepatches sowie
die vollständigen relevanten Filehunks aus `0013-Remove-streams-from-hot-code`
und `0016-Moonrise-optimisation-patches` gelesen. Letztere ändern Beardifier-
Iteration bzw. die Ausführungsdomain des Chunkgenerators, keinen vorgespielten
Rust-Tokio-/Worldthreadvertrag. Surface/Carver-Context bindet Paperbedrocksettings.

**EndIsland getrennt belegen:** Vanilla führt `x*x+z*z` zuerst als overflowing
i32 aus; Paper widenet diese erste Distanz nach i64 und ergänzt einen8192-Slot
Threadlocalcache. Das kann distant Endoutputs ändern. Vanilla-JARgoldens
belegen diesen Paperpatch ausdrücklich nicht. GEN-05.2 benennt seine Original-
Minecraft-Semantik; vollständiger Paper-Drop-in-Anspruch verlangt zusätzlich
die tatsächlich gepinnte Paper-Referenzabnahme dieses Branches. Der Cache kann
in Rust ausschließlich owned session-local oder gar nicht übernommen werden,
ohne eine mutable globale Map zu kopieren. Die Entscheidung betrifft keinen
Overworldcompiler und verzögert diesen nicht.

## Konkrete Differential- und Fehlerabnahme

GEN-05.1-Oracle lädt Originalresources mit den echten Vanilla-Codecs und
Registryholdern, exportiert alle35+7 semantischen Nodeformen/Fields/Referenzen,
f64-/f32-Rawbits und Geometryflags. Für jeden public Compilerentry Vergleich
des checked IR und vollständiger Closure, keine Rust-gegen-sich-selbst-Fixture.
Synthetic malformed JSON, duplicate/escaped duplicatekeys, truncation/trailing,
unknown/wrong-family names, Direct-vs-Holder, finite/codec boundaries,
pointorder±0, combined coordinate/resourcecycles und alle Kappen erhalten
vorhandene Templates; Shader-/Wrapper-/Noiseevaluation gehört nicht .1.

GEN-05.2-Oracle erstellt echte Original-RandomState aus gepinnten registrierten
Settings und Seeds0/±1/i64 extrema/mehreren asymmetrischen Seeds. Alle15 Fields
aller7 Router, alle35 Densitys, Bounds, active Noisepins und sechs Climatewerte
werden bei negativen Koordinaten, vertikalen Limits, repräsentativen Chunk-
grenzen,±30M/scaled wrap boundaries und branch-adjacent Contexts geprüft.
Zusätzliche echte Synthetic-Ap2/MulOrAdd/RangeChoice/Splineproben liefern
Shortcircuit-/Fehlerrequestspuren und Signedzero/NaNpolicy-Beispiele. Original-
Densityprofile niemals aus Tabellen von Hand kopieren.

GEN-05.3-Oracle verwendet echte Original-NoiseChunkinstanzen, ihre Wrapper-
Constructors und Loopmethoden auf explizit fresh/unblended/no-structure Fixtures.
Mehrere vollständige angrenzende Chunks je modern/legacy Settings vergleichen
Cellcorners, slices, Y-/X-/Z-Lerps, arrayFillorder, wrapper requests/counters,
interpolierte finale/initiale Density und preliminary surfacelevel exakt.
Sharing/reordered chunk sessions/clone sampling, illegale Stateabfolgen,
Budget-/Arraycaperrors, Sessionreset und unveränderte Calleroutputs prüfen.

Alle Oracles verifizieren dieselben Bundle-/Inner-/Mappings-/43-Librarypins
wie GEN-04, kompilieren frische Helper zuletzt im Classpath und laufen normal
und Python-`-O`; negative Pinguards werden erneut konkret geprüft. Numerical
bitgoldens sind Debug/Release und toolchaingebunden. Abschließend volle World-
Suite, unveränderte frühere Oracles, strict all-targets Clippy, fmt, Source-
Matrix und Dateifreeze; Root organisiert unabhängige Zusatzproben.

## Unmittelbar anschließend zu echten natürlichen Chunks

Nach .3 folgen keine weiteren allgemeinen Primitiveetappen: **Aquifer und
Material/Orevein → Climate-/Biome-Resolver und SurfaceRules/Bedrock → konkrete
Original-Noisechunkblockausgabe**. Diese Arbeit kann durch denselben Worldagent
an der freigegebenen Pipeline fortgesetzt werden; Producer für fehlende
Surface-/Biome-/Blockparameter werden nur nach konkretem Consumerbedarf ergänzt.
Carvers, Features, Vegetation, normale Ores, Structures und Blending/Retrogen
bleiben anschließend reale fehlende Generationsstages, nicht bereits fertige
Naturalwelt. Generatoridentität und App-/Sim-/Storageadoption müssen vor Aktiv-
schaltung einen separaten tatsächlichen vollständigen Vertragsstand schützen.

Die ersten sichtbaren neuen Chunks dürfen explizit als nachgewiesener Terrain-
Zwischenstand angeboten werden; vollständige originale Welt-/Survival-/Paper-
Pluginfähigkeit und gemessener gleichwertiger RAM/TPSvorteil entstehen dadurch
nicht automatisch. Dieser Brief beansprucht keine Produktionscompletion.

## GEN-05.1 — gelieferter Stand und Abnahmematrix

Nur Worldproduktion: `src/density.rs`, `density/compile.rs`, `density/json.rs`,
`density/graph.rs`; private Compiler-/Graph-/JSONtests. `lib.rs` exportiert nur
`density`. README/INVARIANTS dokumentieren exactscope/checkedPolicy. Cargo nutzt
nur genehmigtes gelocktes `serde_json`; Root besitzt den einzelnen Lockeintrag.
Alle älteren Noise-/Splineproduktionsfiles und ihre Goldens bleiben unverändert.

| Tatsächliche Originalquelle | Vollständiger Pfad/Vertrag | Rust- und echte JARabnahme |
| --- | --- | --- |
| `DensityFunction.java`, `RegistryFileCodec.java` | Direct vs registry Holder vs Holderhelper; beide Helperorigins erzeugen HolderHolder, Noise ist eine eigene Holderfamilie. | Alle35Definitionen und7Router complete recursiveProjection; direct/ref/inline/signed0/wrongfamily/unknown/direct-ref Proben. |
| `DensityFunctions.java` | Alle32 serialisierten Formen; originaler Binaryfactoryclass, Mapped-/Markerdescriptors, clamp DirectCodec, Range/Gradientbounds, Named-noise und Blendedparameters. |32Formen durch echte Codecs; privateIRbit-/kind-/field-/referencegleichheit;6direkte Originalfactoryprobes getrennte nachfolgendeVisitor-Evidence. Kein künstlicher Nullsampler. |
| `CubicSpline.java`, `Spline.Coordinate` | Floatcodec direkt Decimal→f32; derivative required; coordinate rawDensityHolder; nestedSpline keine freieDensityvalue. |3verschiedene FLOATfelder bei1.0000000596046448 ergeben3f800001; missingderivative echt rejected; eigene finite/ascending-Caps gesondert tested. |
| `NoiseSettings.java`, `NoiseGeneratorSettings.java`, `NoiseRouter.java` | geometryguard,15Felder, sea level/4flags; die übrigen Settingsfelder bleiben deferred. |Alle7 realen Settingsdecoder erfolgreich;8 echte Geometrycases inclfractions/wrap; originalCellwidth/-height. Keine vollständigeSurface-/Materialconsumerbehauptung. |
| Bundled Gson2.11 `NumberLimits` (vollständiges `javap -p -c`) | JSONops INT truncates/wraps vor Range; BigDecimalfallback≤10000Zeichen und abs(scale)<10000. Fastpaths sind withinbounds. |OriginalIntegerfraction/negative/wrapping/i64overflow und1e309/1e1000/1e-1000 Proben; zusätzlich private10000/10001Digits,9999/10000scale,integer-lowbits tests. |
| Eigene boundedAdmission | JSONgrammar,decodedUnicode/escapedDuplicates,trailing;Source/JSON/IR/Splinecaps; jointCycle/depth includingunused. |Genau128KiB/+1,4096/4097JSONnodes,32/33JSONdepth,256/257entries+UTF8bytes,4096V/+1,8192E/+1,2MiB/+1,64/65points;SplineNode/Point/Coord/Depthcaps; validcombined96/97 und echteDensity→Spline→Coordinate→namedresourcecycle. |

Vollständig gelesene Sourcehashes (angewendete gepinnte Paperreference, getrennt
vom actual Original-JARcodecbeweis):

| Datei | SHA256 |
| --- | --- |
| DensityFunction.java | `3a7c65bcefe09274d50a69e971858679273d2090a190202200cc2d6b8c7459f9` |
| DensityFunctions.java | `62b4422976740e5ea5115b7675cfb404bec6887c15ca232e79bbeaf35480a749` |
| RandomState.java | `95482aed87fb09b24ad4500cf03042aeebc808451c56b0299ae9ec6fd10d46d7` |
| NoiseRouter.java | `58ae86a1d35c0152b4bfb52a878711bef6ba56635b108a65230a3d3d2552c8b7` |
| NoiseGeneratorSettings.java | `e132b2da8afc1648b98b9feee7a27a9ae7b30b44d473f6c9e49c704df98bb728` |
| NoiseSettings.java | `78db972efcb2f96c57cea2641d957954d8da791b51bfc176397b0666bfe865c7` |
| CubicSpline.java | `834d1b5f4696161ca974fcf2b4ef8caa3ebec5e6a0b5bfcccb561ba4a6a63f97` |
| RegistryFileCodec.java | `030a5ad3d560f6f1e2d52b4698a8cdb0bd832ae11be4cff9e644f91820f90f3a` |
| VanillaRegistries.java | `69965842728b8a5ff4a829cabf16ffde977513fef6cef5fdb30854a84189344c` |

Eigenes finales Originaloracle: **128 Records**,35Definitionen,7Router,
72Densitycodec-/8Settingsproben,6direkteFactories. Alle32Formen erfolgreich;
80Codecproben71ok/9error. Finalfixture278269Bytes SHA
`856bd9f9d25642864945a11bd2ea907fa3c90b4fda8bd7b1153017b74da65a73`;
Manifest SHA `57f4adec761f15dfb6b26605d51bba1872a1621589595479b28491bc30fe257d`.

Die Childlieferung5Files/125Records (Fixtureb2a83d75..., Manifest9c9015...)
war zunächst eingefroren. Der Worldimplementer hat danach **bewusst erneut
regeneriert**:3zusätzliche tatsächliche INTexponent-/fractionwrap-Proben,
redundante strukturierte `input`-Objekte entfernt,32formcensus auf exakte
`input_json`-Sourcetrings umgestellt, entsprechende OracleREADMEpolicy angepasst.
Damit werden OriginalacceptedInfinityTokens niemals über generisches f64
voreingelesen. `input_json` enthält unveränderte originale Codecinputs; Antworten
stammen weiter vollständig aus echten JARobjects. Keine erneute Childinstanz.

Explizite old→new5Fileshashes:
`.codex-tmp/gen05_1-codec-child-to-final-provenance.json`; minimale Java-/Python-
Diffs `.codex-tmp/gen05_1-child-to-final-java.diff` und `...-py.diff`. Deren old
Sources wurden gegen den ursprünglichen Childfreezehash83c7631e.../01bc7c7d...
verifiziert. Alte Datafiles liegen unverändert in Oracle-a. Der neue vollständige
5Filesfreeze ist `.codex-tmp/gen05_1-codec-oracle-final.freeze.json`; Root prüft
bewusst den neuen Stand und behandelt ihn nicht als unveränderte Childlieferung.

Normales und Python-`-O`-Oracle liefern bytegleiche finalfixture **und** manifest:
`.codex-tmp/gen05_1-codec-oracle-d[.log]` / `...-e[.log]`. Alle5negativen
Pinguards gegenfinalGenerator normal/-O erneutgrün:
`.codex-tmp/gen05_1-final-codec-pin-guards[ -o].log`. Bundle/Inner/officialMappings/
sourcefixture/alle43libraries folgen den bestehenden GEN-04pins; neuerHelper
wird frisch kompiliert und zuletzt imClasspath ausgeführt. Manifest enthält
Source-/Helper-/Generator-/Libraryhashes und JDK21.0.12.1+1/Python-/Plattformpins.

Eigene Checks mit Rust1.99.0/b940084d7/LLVM23.1.1/x86_64Linux und2jobs,
`--offline --locked --target-dir ../.codex-tmp/gen04_3-target` ab `server/`:

- `cargo test -p ferrumc-world`: **177 passed** (134lib+43integration),14neue
  Densitytests; alle bisherigen Originalnoise-/Splinebitcorpora unverändertgrün.
- `cargo test -p ferrumc-world --release`: **177 passed** bei selben Fällen.
- `cargo clippy -p ferrumc-world --all-targets -- -D warnings`: grün.
- `cargo fmt -p ferrumc-world -- --check`: grün.

Logs `.codex-tmp/gen05_1-world-debug.log`, `...-world-release.log`,
`...-clippy.log`. Dateifreeze `.codex-tmp/gen05_1-world.freeze.json`; Root wird
separat unveränderteDependency-/Sourcepins und alle tatsächlich gelieferten
Files unabhängig prüfen. Kein Implementercommit. Nach sauberer .1-Abnahme
ist **GEN-05.2 echte Seedbindung/Scalar-/Climateauswertung** das konkrete nächste
Ticket; alle Samples/Chunkwrapper/Aquifer/material/biome/surface und aktive
Naturalwelt bleiben bis zu ihrer eigenen Umsetzung offen.


## GEN-05.2 geprüfte Herkunftspräzisierung (2026-10-10)

Root hat die vollständige sparse-native Herkunftskorrektur intern genehmigt.
Unveränderter Vorschlag SHA256 `2e91cc9b346bf0168036033e185179f059fc3801f7ad7dd21c18f3de733277e5`;
der konkrete geprüfte Inhalt ist nachfolgend archiviert. Diese Freigabe erweitert
keine öffentliche API und aktiviert keine Chunk-/Paper-Terrainausgabe.

# GEN-05.2 amendment: actual registered target origin

Internal approval request to Root, no user approval required. Parent has made
no runtime change for this issue yet. Only a private dry-run census/projection
test was added. Existing oracle outputs remain authoritative and unchanged.

## Concrete original behavior and uncovered mismatch

Read complete base RandomState.java constructor/inner NoiseWiringHelper/anonymous
climatevisitor and full NoiseRouterData.java.bootstrap plus all relevant helpers:
`.codex-tmp/paper-1.21.8-reference/paper-server/.gradle/caches/paperweight/mache/base/sources/net/minecraft/world/level/levelgen/`.
RandomState lines35..130 wires actual settings graphs and returns actual registry
values inside mapAll. HolderHolder.mapAll (DensityFunctions576..578) retains a
Direct holder after mapping its **actual value**. NoiseRouterData.bootstrap
(69..156) builds registry graphs through native factories, with explicit named
HolderHolder only in registerAndWrap/getFunction (219..225). Inline native
factory child inputs do not have the codec's HOLDER_HELPER_CODEC wrapper.
Native spline coordinates use raw named Holder (registerTerrainNoises lines175ff).
Native MulOrAdd arises through TwoArgumentSimpleFunction.create (1155..1214).
MulOrAdd.mapAll (749..770) owns its specific reconstruction formula rather than
replaying Ap2.create. Actual Vanilla-JAR recursive projections already show this.

Example original decoded router temperature -> named shift_x Holder -> native
flat_cache -> native cache_2d -> ShiftA. Current Rust wrongly has a direct
CodecHolder between flat_cache and cache_2d because .1 graph roots are decoded
from exported JSON. .1 typed metadata/codecprojection remains valid: resource
projections stop at the typed ResourceId; it did not claim actual registered
closurefactory evaluation. .2 now needs to represent both actual origins.

## Proposed bounded preparation before existing two visitors

No public API or .1compile changes. In bind.rs create one private PreparedGraph
from the admitted .1 graph. Preserve source vertices at original indices for
explicit decoded named requests/router roots. Create a `native_map` for source
nodes reachable from all35named roots, in admitted child-first order.

- A decoded ReferenceHolder/rawReferenceCoordinate points to actual native_map
  of its registered resource target. A decoded DirectHolder retains its source
  decoded child. Decoded inputs/Namedroots do not blanket strip their holders.
- Native inlineDirectHolder is only a codec-representation artifact and maps
  directly to the native child without allocating a runtime vertex. Native
  ReferenceHolder remains an actual factory and points to native_map(target).
- All other native child references use native_map(child). If those child IDs
  and source factory shape match the already prepared decoded counterpart,
  share that immutable node. Otherwise append one tagged native variant for that
  source index. No repeated reference occurrence expands a target tree.
- Native direct-child Constant means its Binary source shape actually came
  from MulOrAdd before NoiseWiring. First noisevisitor therefore uses original
  MulOrAdd.mapAll endpoint formula for that tagged node. Ordinary decoded Ap2
  follows actual Ap2.mapAll/create; actual climate secondvisitor consumes prior
  seeded factory as already implemented. No arithmetic constantfolding.
- Boundnodes retain `{source_index, representation:Decoded|Registered,
  holder_origin:Option<HolderOrigin>}` privately; BoundView tags SeedRouter or
  Climate. Actual factory remains BoundKind (Original/actualAlias/MulOrAdd).
  Source-origin tags and private retained aquifer/ore factories may carry an
  explicit dead_code expectation in non-test compilation because they are
  required audit/following-consumer metadata. No fake public API or consumer.

PreparedGraph stores owned nodes, roots/fields/noises metadata and checked stats;
root/.1 graph never mutates. BoundSpline construction uses prepared remapped
coordinate targets and the same existing spline bounds/evaluator.

## Exact measured variant census and bounds

Private dry-run tested childremapping/native omission against all7settings and
ran existing full graph::validate on its result. Log
`.codex-tmp/gen05_2-native-census.log` (Rustdebug test inspect_registered_target_census).
Both visitorviews retain the same admitted vertex slots; climate Alias nodes
are uncharged and actualprojection skips them. Thus counts below apply to EACH
SeedRouter/Climate view before/after secondvisitor:

| Settings | Source vertices | Native variants | View vertices | View edges | View depth |
| --- | ---: | ---: | ---: | ---: | ---: |
| amplified |2398|162|2560|3032|71|
| caves |2271|162|2433|2887|35|
| end |2295|162|2457|2907|35|
| floating_islands |2271|162|2433|2887|35|
| large_biomes |2398|162|2560|3032|71|
| nether |2271|162|2433|2887|35|
| overworld |2398|162|2560|3032|71|

This preserves existing4096vertices/8192edges/combineddepth96 perview and
8192/16384 combinedviews. Native fullcopy would incorrectly require4233..4360
vertices and is explicitly rejected. Check cloneappend vertices/edges BEFORE
growth, run original graph::validate for shape/cycle/depth before publishing
prepared/bound graph. Construction temporary mapping/order/pending lists have
original4096caps. Invalid shape typed InvalidGraph; limits typed BindError.Limit,
no incomplete public binding. Existing noise60/splinecaps unchanged.

## Work/evaluation and tests

Actual native codec-artifact holders have no compute operation and no charge;
actual native Referenceholders remain compute entries after noisevisitor.
Actual climate-removed Holder/Marker aliases remain nocharge. No workbudget
reset across Density/Spline/Noise and no valuecache. Context/Worldborder/Y and
publiccaps remain unchanged. Native MulOrAdd still evaluates its input at0;
Ap2 mul ±0 still skips right and yields+0.

Acceptance:
1. Replace temporary census with exact production PreparedGraph sourceorigin,
variantcounts/edges/depth assertions for all7settings and bothvisitorviews.
2. Compare ALL recursive actual7projection trees, every Factory/input/bit/bounds,
including nested source coordinate/view, against unmodifiedactualJARhelper.
3. Preserve49binding/1813point/90650scalar/294quart matches; add private builtin
native-vs-directdecoded Namedtarget test proving distinct wrapper/factory/work
origins and sourceconst/MulOrAdd secondvisitor diagnostics.
4. Complete separate Oraclechild additional actual noise/RNG/branchtrace/quantize
corpus, sharedmeter N/N-1 tests and typed finite/errorpolicy.
5. Full oldpublicoracles, debug/release, strictclippy/fmt, source matrix/provenance,
final freeze and independent Rootreview. No .3chunks or Paperterrain claim.

## GEN-05.2 — gelieferter Stand und eingefrorene Abnahme (2026-10-10)

Die freigegebene Seedbindung ist implementiert. Der ursprüngliche vollständige
Aktivierungsvorschlag `.codex-tmp/gen05_2-activation-proposal.md` hat SHA256
`9b92ed683d495d5e8ddea964b046f44a1d90e7c8ac089cb811c24b94bf84b177`.
Die oben archivierte Herkunftskorrektur ist ebenfalls umgesetzt und geprüft.
Dieser Abschnitt dokumentiert die eigene Abnahme; er ersetzt kein unabhängiges
Rootreview und behauptet keine aktive Terrain- oder Survivalfähigkeit.

Öffentlicher Vertrag in `ferrumc_world::density`:

- `RouterTemplate::bind_seed(WorldSeed)` liefert einen immutable `SeededRouter`.
- `seed`, `resource`, `field_bounds`, `definition_bounds`, `climate_bounds`
  melden expliziten Ursprung und originale Diagnostik ohne Sample-Clamping.
- `sample_field` und `sample_definition` werten den tatsächlichen direkten
  Blockkontext aus; `sample_climate_at_block` behält exakt diesen Block,
  `sample_climate(ClimateQuartPos)` verwendet den originalen Quartursprung.
- Point/Climate teilen je einen geprüften Budgetwert von 1 bis 1.048.576.
  `sample_batch` nimmt 1 bis 4096 Requests und ein gemeinsames Budget von
  höchstens 67.108.864; erst vollständiger Erfolg veröffentlicht das Ergebnis.
- Klassifizierte Compile-/Bind-/Samplefehler behalten Resource, Feld, Position,
  Batchindex und die jeweiligen Limit-/Nonfiniteinformationen. Kein Sample
  führt I/O, RNGmutation, Locks oder eine Valuecachemutation aus. Nur der
  begrenzte Batchoutput wird beim Sampling allokiert.

Die sparse-native Vorbereitung ergänzt pro Router exakt 162 private Varianten.
Beide tatsächlichen Visitorviews halten für Overworld/Large/Amplified jeweils
2560 Vertices, 3032 Edges und Tiefe 71, für Caves/Floating/Nether jeweils
2433/2887/35 und für End 2457/2907/35. Alle behalten die freigegebenen Kappen
4096/8192/96; Vertex-/Edgeprüfungen passieren vor Wachstum, danach läuft der
vollständige bestehende gemeinsame Graphvalidator. Decoderorigins und actual
registrierte Factoryprodukte bleiben getrennt; unveränderte private Nodes
werden immutable geteilt. Das belegt noch keine Java-HashMap-/Objektgleichheit
oder Wrapper-Sharingsemantik einer späteren NoiseChunk-Session.

Die echte Original-JAR-Abnahme umfasst **17.213 Records**:

| Abnahme | Tatsächlich geprüfter Umfang |
| --- | --- |
| Seeds und direkte Werte | 49 Settings-/Seedbindungen, 1813 Blockpositionen, 90.650 Densitywerte; alle 15 Fields und 35 decoded Namedroots. |
| Tatsächliche Visitorstruktur | 637 vollständige rekursive Roots: je sieben Settings × (15 Fields + 35 decoded Named + 35 native Registered + sechs Climateaxes); sämtliche Factory-/Bounds-/Referenzbits. |
| Spline und Climate | 53.628 verschachtelte Spline-Treeoccurrences mit vollständiger Diagnostik; sechs Climateaxes an allen Blockpositionen, 294 zusätzliche Quartproben und 42 echte Quantisierungsfälle. |
| Noise und Namen | 14.700 Normal-/Perlinproben einschließlich großer legitimer Inputs und falscher Namen als Kontrollen; 2940 originale Maximalwertpins für alle 60 Noiseresourcen. Runtimepool wird nur gegen aktive Ressourcen geprüft. Unbenutzte kanonische Testproben verwenden separat denselben privaten Binder. |
| Factory und Requests | 308 tatsächliche Branch-/Factorytraces aus 77 Cases × Factory/Identity/Wiring/Climate, einschließlich Shortcircuit-/Nonfinite-/Failureprefixnachweis. |
| Spätere Material-RNG-Basis | 686 echte Aquifer-/Ore-Factorypositionen und 2058 Longdraws; noch keine Aquifer-, Ore- oder Blockausgabe. |

Elf zusätzliche Rusttests prüfen unter anderem native Herkunft und alle Kappen,
gemeinsame Density→Spline→Density/Noise-Meter, ausgewählte Branches,
Quart-/Worldborder-/i32-Y-Grenzen, große Noiseinputs, Endoverflow, atomare
Batchfehler und die ausdrücklich strengere finite Policy. Jede tatsächlich
angefragte fertig berechnete Nonfinite-Node wird verweigert: Java kann danach
noch einen endlichen outside-RangeChoice-Child wählen. Java ProbeFailure wird
im privaten Rusttrace durch einen Nonfinite-Childsentinel modelliert; geprüft
wird die Requestfolge beziehungsweise der Refusalprefix, keine Gleichheit
von Javaexceptions oder Pluginverhalten.

Vanilla End behält den ersten i32-Radiusoverflow und den vollständigen
625-Kandidatenloop. Ein resultierendes NaN wird als typisierter Fehler
verweigert. Paper widenet diese Distanz und benötigt eine getrennte
Numericalabnahme; die Vanilla-JAR-Fixture beweist diese Abweichung nicht.

Oraclechild besitzt die fünf neuen Router-Fixture-/Referencefiles exklusiv.
Sein vollständiger Freeze `.codex-tmp/gen05_2-child-oracle-final.freeze.json`
hat SHA256 `bae903ce56acdef17ac8d28901f359c351a5958fc08eb340a731ba28843ffac8`.
Fixture: 47.371.851 Bytes, SHA256
`2012acc2b8256a9adbf10b54a3534e8d9cc5d0a79d5e481d5bc3ff55cd73430e`;
Manifest SHA256
`f3dfc7e1614e26ab809996dea2928638e76b6c03da80b124c58f63c8b96c80f2`.
Normaler und Python-`-O`-Reexport ergeben bytegleiche Fixture **und** Manifest;
acht negative Pinguards laufen auch unter `-O`. 28 tatsächliche CodeSources
(27 Originalklassen und Helper) sowie 51 Inputs wurden rehashed. Die ursprüngliche
2163-Record-Baseline ist separat bytegenau rekonstruiert. Vollständige
Source-/Calleematrix und Policygrenzen stehen in
`tests/reference/density_router.md` der Worldcrate. Root hat zusätzlich eigene
frische normale und optimierte Reexports geprüft.

Eigene finale Checks mit dem gepinnten Rusttoolchain, zwei Jobs,
`--offline --locked --target-dir ../.codex-tmp/gen04_3-target` ab `server/`:

- World-Debug: **188 passed** (145 Lib + 43 Integration).
- World-Release: **188 passed** bei denselben Fällen.
- Strict all-targets Clippy: grün.
- World `cargo fmt -- --check` und Scope-`git diff --check`: grün.
- Alle 20 vorher getrackten Fixture-/Referencefiles, World-Cargo.toml,
  Cargo.lock, Compiler-/JSONfiles und ursprüngliche `worldgen.json` bleiben
  unverändert; alle fünf Oraclechildfiles stimmen mit dessen Freeze überein.

Finale Logs: `.codex-tmp/gen05_2-world-debug.log`,
`...-world-release.log`, `...-clippy.log`, `...-world-fmt.log`,
`...-world-diff-check.log`. Der komplette eigene Runtime-/Docfreeze liegt in
`.codex-tmp/gen05_2-world.freeze.json`; er enthält sämtliche tatsächlich
geänderten/neuen Worldfiles und diesen Brief einschließlich der separat
gefrorenen Oraclechildfiles. Unveränderte Inputs sind separat in
`.codex-tmp/gen05_2-world-input-check.json` festgehalten. Kein Implementercommit.

Nach Übergabe bleiben alle .2-Produktionsfiles und dieser gemeinsame Brief
für unabhängiges Review unverändert. Die nächste konkrete Arbeit ist der
separate vollständige .3-Vorschlag
`.codex-tmp/gen05_3-noise-chunk-proposal.md`: tatsächliche fünf Wrapper,
Originalsharing, Array-/Cellkontexte, Lifecycle, Interpolation, Ressourcen- und
Arbeitskappen sowie echte NoiseChunk-JAR-Abnahme. Kein .3-Runtimecode wird vor
Rootfreigabe geschrieben. Aquifer/Material/Orevein, Biome/Surface/Bedrock und
konkrete Noisechunkblöcke folgen danach; Carvers, Features und Structures
bleiben weiterhin eigenständige reale Generationsstages.


## GEN-05.3 — konkrete interne Produktionsfreigabe (2026-10-10)

GEN-05.2 ist nach unabhängigem CLEAN-Review als `290e5343` integriert. Root hat
den folgenden vollständigen Proposal gelesen und zur Runtimeimplementierung
freigegeben, SHA256 `4d943e796734ffe1231b9d95271205deec5ab457b45b753445517e88731f8cff`. Der .2-Dateifreeze wird ausschließlich für
diese dokumentierten .3-Änderungen freigegeben. Die öffentlichen .2-Scalar-/
Climate-Vanillaverträge bleiben byte-/bitgleich; PaperBuild60 ist die explizite
neue Chunkpolicy, ohne globalen Endcache. Scope bleibt Worldcrate und dieser
Brief; kein Git, Cargo/Lock oder andere Crates. Root prüft den tatsächlichen
Consumer unabhängig vor Integration. Widersprechende Sharing-/Array-/Capdaten
werden konkret gemeldet, nicht semantisch vereinfacht. Nach Abnahme folgt
unmittelbar Aquifer/Material. Das nachfolgende vollständige Originalproposal
ist unverändert archiviert; geplante Acceptance ist noch keine Abnahme.

# GEN-05.3 — konkrete Paper-NoiseChunk-Session

Stand: 2026-10-10. **Vollständiger Implementierungsvorschlag an Root; keine
.3-Rustproduktion geschrieben.** GEN-05.2 bleibt während seines unabhängigen
Reviews unverändert. Der gemeinsame GEN-05-Brief gehört zu dessen Freeze und
wird durch diesen separaten Vorschlag nicht nachträglich geändert.

## Ergebnis und exakter Scope

Ein ausschließlich besessener, synchroner Worldconsumer führt die tatsächliche
NoiseChunk-Wrapperbindung, Arrayfüllung und vollständige Interpolationsschleife
für alle sieben gepinnten Settings aus. Er liefert die originale Dichte, die der
NoiseChunk-Konstruktor seinem Aquifer übergibt:
`cacheAllInCell(add(wrappedRouter.finalDensity(), BeardifierMarker)).mapAll(wrap)`.
Im ausdrücklich frischen Kontext bleibt der Beardifier der echte Nullmarker.
Die Addition und das zweite `mapAll` werden trotzdem ausgeführt: `finalDensity`
und `finalDensity + +0.0` sind keine allgemein austauschbaren Bitverträge.

Zusätzlich liefert derselbe Consumer die originale Preliminary-Surface-Abfrage
und die sechs gecachten Climateachsen. Dies ermöglicht danach unmittelbar den
Aquifer-/Ore-/Materialconsumer und Biome/Surface/Bedrock. Dieser Schritt erzeugt
noch keine Blöcke, keine Flüssigkeit, keine Erze, keine Biome und keinen fertigen
Naturalgenerator. Keine weitere allgemeine Frameworkrunde wird vorgeschaltet.

Scope ausschließlich `server/crates/ferrumc-world/**` und nach Rootfreigabe der
gemeinsame GEN-05-Brief. Keine neue Crate, keine Dependencies/Lockmutation,
kein App-/Storage-/Sim-/Configumbau, kein Generatoridentitätswechsel und keine
Gitmutation durch den Implementer. Root besitzt unabhängiges Review/Integration.

## Explizite Referenz- und Kontextentscheidung

Root hat für neue Chunkauswertung **Paper 1.21.8 Build 60** ausgewählt, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Öffentlicher Konstruktor heißt
ausdrücklich `fresh_paper_noise_chunk`. Der einzige zulässige Kontext ist:

- frisch generiertes, vollständiges 16×16-Noisechunk;
- `Blender.empty()`, keine alten Blendingdaten, kein Retrogen;
- keine Structure-Beardifierbeiträge;
- ursprüngliche vollständige Settingsnoisegeometrie, kein HeightAccessorclamp;
- die bestehenden geprüften, JSON-decodierten Builtinsettings aus GEN-05.1/.2
  mit ihren tatsächlichen nativen registrierten Densityreferenzzielen.

Der Name und die Rustdocs stellen diese Voraussetzungen klar. Es gibt keine
öffentlichen Blender-/Structurehandles oder einen stillen Fallback für deren
spätere aktive Consumer. Native registrierte Settings sind zusätzliche
Source-/Sharingkontrollen des Oracles, nicht heimlich dieselbe Factoryherkunft.
Der spätere Appgenerator muss seine HeightAccessor-/Structure-/Blendinginputs
vor Aktivierung ausdrücklich an einen dann implementierten Vertrag binden.

GEN-05.2s öffentliche Scalar-/Named-/Climateproben behalten **exakt** ihren
Vanilla-Pointvertrag. Dieselbe immutable Simplexpermutation ist als Zahlenbasis
nutzbar; der neue Chunkconsumer wählt privat die Paper-Endradialdistanz. Alle
neuen Chunkpfade einschließlich SinglePoint-Fallbacks, Preliminary Surface,
Arraycorners und Climate verwenden diese Paperpolicy. Keine bekannte endliche
Vanilla-Endabweichung wird als Paperterrain angeboten.

Minimale private numerische Seam in `density/evaluate.rs`:

```text
enum EndRadialReference { Vanilla1_21_8, Paper1_21_8Build60 }
end_islands(...): existierender Einstieg -> Vanilla1_21_8
end_islands_with_reference(..., reference): gemeinsam geprüfter Rechenkern
```

Nur die erste radiale Summe unterscheidet sich: Vanilla wrapping-i32,
Paper i64-Multiplikation und i64-Summe. Paper konvertiert diese Summe zuerst
nach f32 für `Mth.sqrt(float)`; danach double-sqrt und f32-Cast. Der übrige
625-Kandidatenloop, signed Div/Rem, f32-Arithmetik und Seed/RNG bleiben erhalten.
Die Chunkpolicy ist kein öffentlicher frei konfigurierbarer Bool und verändert
keine bestehenden Defaults. Leafarithmetik/Noise-/Splinehelper werden privat
geteilt, während Wrapper und echte Arraydispatch im Chunkconsumer liegen.

Papers Threadlocal-/WeakHashMap-Endcache wird in dieser Stufe **nicht** kopiert.
Der Paper-Numericalpfad wird ohne zusätzlichen Endcache berechnet; sämtliche
625 Kandidaten und ausgeführten Simplexaufrufe bleiben im gemeinsamen Meter.
Die eigene Workmenge darf daher von Papers Optimierung abweichen. Es gibt
keinen behaupteten Cachetrace-/CPU-/TPS-Gleichheitsvertrag für diesen Patch.
Root hat bereits echte fresh/reverse Paper-Endbitstabilität nachgewiesen.

Root-Endbasis unverändert: `.codex-tmp/gen05_2-root-paper-end/`, Freeze SHA256
`d27d4bac9ec5870eba58eabf4919cc6382a1846c928d1790b720f7d714bb10ad`.
658 Paare ergeben 392 gleiche und 266 unterschiedliche Antworten, darunter
210 Vanilla-NaNs und 56 endliche Drifts. Paperinner SHA256
`c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`.
Die neue Abnahme ruft zusätzlich tatsächliche Paper-NoiseChunkmethoden auf.

## Öffentliche API und Lebensdauer

Alle Felder privat, jedes öffentliche Item mit Rustdoc. Konkrete Oberfläche:

```text
NoiseChunkWorkBudget::new(u32) -> Result<Self, NoiseChunkError>
    // 1..=268_435_456
SeededRouter::fresh_paper_noise_chunk(ChunkPos, NoiseChunkWorkBudget)
    -> Result<NoiseChunkSession, NoiseChunkError>

NoiseChunkSession::{seed, resource, chunk_pos, geometry, work_used}
NoiseChunkSession::sample_chunk_densities(&mut self)
    -> Result<ChunkDensitySamples, NoiseChunkError>
NoiseChunkSession::preliminary_surface_level(&mut self, BlockPos)
    -> Result<i32, NoiseChunkError> // y ignored, original MAX_VALUE sentinel
NoiseChunkSession::sample_climate(&mut self, ClimateQuartPos)
    -> Result<ClimateSample, NoiseChunkError>
NoiseChunkSession::reset(&mut self, NoiseChunkWorkBudget)
    -> Result<(), NoiseChunkError>

ChunkDensitySamples::{seed, resource, chunk_pos, geometry}
ChunkDensitySamples::density_at(BlockPos) -> Option<f64>
ChunkDensitySamples::values() -> &[f64]
```

`NoiseChunkSession` hat weder `Clone` noch öffentliches Cell-/Counter-/Node-
Manipulieren. Sie teilt ausschließlich immutable Router-/Samplerarcs; ihre
Wrapperwerte, slices, Scratcharrays und Epochs gehören genau einer Session.
`ChunkDensitySamples` besitzt die vollständig erfolgreiche Ausgabe. Es ist
keine `Chunk`, `ChunkGenerator`-Implementierung oder neue GenerationIdentity.

Die Konstruktion setzt genau einen Workmeter. Eager FlatCacheprecompute,
Coldvisitor, Driver, spätere Preliminary-/Climateabfragen benutzen diesen
Meter bis zum Reset; keine lokale budgetlose Arbeit und keine Budgetaddition
bei einer späteren Methode. Shape-/Koordinatenadmission wird vor Berechnung
beziehungsweise Ausgabeallocation abgeschlossen.

Lifecycle: Fresh → Running → Complete. `sample_chunk_densities` ist genau einmal
je erfolgreichem Reset erlaubt. Preliminary/Climate sind in Fresh und Complete
zulässig. Ein Ausführungsfehler führt nach Poisoned; dann erlaubt ausschließlich
`reset` eine weitere Auswertung. Reine vorab abgewiesene falsche Positions-/
Lifecycleanfragen verändern keine Caches. Reset baut eine vollständig geprüfte
Candidate-Session und tauscht erst bei Erfolg; fehlgeschlagener Reset lässt
den bisherigen Zustand unverändert. Bereits ausgegebene Ergebnisse bleiben
owned und werden durch Reset nicht verändert. Kein Rollbackclaim für vorher
geänderte private Caches bei einem Driverfehler.

Horizontaler Chunkcheck mit checked i64: alle 16×16 Innenblöcke und der positive
Corner-/Quartgridrand `origin+16` müssen in ±30.000.000 liegen. Zulässige
Chunkkoordinaten je Achse deshalb -1.875.000 bis 1.874.999. Kein shift-/multiply-
Wrap für Chunks. Y kommt aus der geprüften vollständigen Settingsgeometrie.
Einzelne Preliminary-/Climate-Singlepoints behalten .2s geprüftes X/Z- und
i32-Y-Domain; es gibt keinen unbemerkten Clamp zum Chunkrand.

## Datenbereich, Ausgabe und spätere direkte Consumer

Der Driver erstellt genau **eine** f64-Ausgabe für die tatsächliche
Konstruktor-Materialdichte. Keine zusätzlichen initial-/climate-/vein-
Fieldrequests werden in dessen hot loop eingefügt. Originale bare finalDensity
und initialDensity werden in getrennten privaten Acceptance-Sessions geprüft;
deren Observationsrequests verändern nicht die Produktionsrequestspur.

Ausgabe index = `((y-minY)*16 + localX)*16 + localZ`, also aufsteigendes Y,
X, Z. Berechnung erfolgt trotzdem im originalen Cell-/Blockloop. Der aktuelle
Ausgabeindex wird vor Schreiben aus der tatsächlichen absoluten Blockposition
geprüft; kompletter Driver muss jeden Slot genau einmal schreiben. Anzahl:
256×height, bei Overworld 98.304 f64 = 786.432 Bytes. `density_at` verweigert
fremde Chunkpositionen/ungeprüfte Y mit `None`. Height, MinY, Seed und Resource
begleiten jede Ausgabe; bestehende Flat-Overworldgeometrie wird nicht vorausgesetzt.

Private für den nächsten konkreten Materialconsumer behalten:

- alle 15 vollständig gewrappten Routerroots in originaler Reihenfolge;
- actual Konstruktor-Materialroot mit zweitem Wrap;
- die initialDensityroot und sechs gecachte Climatesamplerroots;
- Noisegeometrie/sea level/Aquifer-/Oref flags und bestehende immutable
  Aquifer-/Ore-Positionalfactories;
- aktuellen own FunctionContext und einen einzigen Meter.

Der anschließende Materialdriver ersetzt am **selben** Blockpunkt nur den
Ausgabesink: erst materialDensity im own Context, dann Aquifer.computeSubstance;
bei null weiter OreVeinifier, bei null Settingsdefaultblock. Ore konsumiert
VeinToggle, VeinRidged, VeinGap in seinen tatsächlich gelesenen Branches;
es wird keine spekulative Evaluate-all-Auswertung dieser Fields eingeführt.
Aquifer kann in seinen Druckbranches denselben aktuellen own Context für
BarrierNoise und in seinen Fluid-/Surfacebranches die originalen fremden
SinglePointcontexts verwenden. Die .3-Arraysamplingmethode liefert hierfür
keine öffentliche frei manipulierbare Zwischenstate-API. Der konkrete neue
Aquifer-/Materialcode benötigt anschließend eigene Source-/JAR-Abnahme.

## Tatsächlich gemessener Paper-Census als Kappenbasis

Planninghelper `.codex-tmp/gen05_3-planning-census/NoiseChunkPlanningCensus.java`
konstruiert **echte** gepinnte Paper-NoiseChunkobjekte. 98 Constructorfälle:
7 Settings × 2 tatsächliche Origins (decoded/registered) × 7 Seeds. Danach
actual `cachedClimateSampler`; bei Seed0 je Origin/Setting ein kompletter
originaler Cell-/Blockloop, insgesamt **14 vollständige Materialdensityloops**.
Der Helper liest die Dichte aus dem tatsächlichen ersten MaterialRuleList-
Lambda-Capture, nicht aus einer nachträglich von Hand gebauten Root.
FluidPicker verweigert tatsächliche Fluidberechnung; es werden keine Blöcke,
Aquifercompute- oder Orepfade ausgeführt. Die Materialroot kommt dennoch aus
dem vollständigen echten Konstruktor mit zweitem `mapAll(wrap)`.

| Settings | Interpolator / Flat / 2D / Once / Cell | Insgesamt inkl. 2 Blendflatcaches | Actual wrapped Map: decoded / registered | Eigene f64-Cachearrays |
| --- | --- | ---: | ---: | ---: |
| overworld, large_biomes, amplified | 8 / 10 / 6 / 6 / 1 | 31 | 254 / 150 | 34.384 Bytes |
| caves | 1 / 6 / 4 / 0 / 1 | 12 | 60 / 30 | 4.224 Bytes |
| floating_islands | 1 / 6 / 4 / 0 / 1 | 12 | 60 / 30 | 6.368 Bytes |
| nether | 1 / 6 / 4 / 0 / 1 | 12 | 60 / 30 | 3.584 Bytes |
| end | 1 / 2 / 1 / 0 / 1 | 5 | 63 / 19 | 4.032 Bytes |

Alle sieben Seeds behalten diese Counts. Der gecachte Climatepass ergänzt im
Corpus weder Wrapper noch Arrays. In allen 14 vollständigen Builtinloops
bleiben CacheOnce.lastArray null; tatsächliche Synthetic-arraycases sind
deshalb Pflicht. Dieser Census ist eine konkrete Ressourcenmessung des echten
JARs, keine .3-Rust-, Terrain-/Gameplay-/Paper-Server-Abnahme.

Normale und optimierte hardened Reproduktion:

```text
python3 .codex-tmp/gen05_3-planning-census/run_census.py NEW_OUTPUT
python3 -O .codex-tmp/gen05_3-planning-census/run_census.py OTHER_NEW_OUTPUT
```

Die jeweiligen JSONL **und** Manifests müssen bytegleich sein. Der Runner
prüft OriginalworldgenSHA, alle 120 gehashten Paperclasspathartefakte gegen
Roots Inputpins, elf actual Classorigins, den frisch kompilierten Helperorigin,
Classpathshadow und Javaagent-/Classpathenvinjection, Inputs vor/nachher.
Genaue finale Hashes stehen im separaten Planningfreeze, der zusammen mit
diesem Proposal übergeben wird.

## Getrennte Ressourcen- und Arbeitsgrenzen

Alle Rechnungen checked, alle Arena-/Array-/Map-Erweiterungen prüfen vor
Wachstum. Keine totalRSS-Zusage aus Arraybytes.

| Bereich | Konkrete Grenze und Admission |
| --- | --- |
| Sourcegraph/Visitor | Bestehende private source/prepared Graphcaps 4096 Vertices, 8192 Edges, depth96. Coldvisitor maximal 131.072 tatsächliche Density-/Splinevisitsteps; vor Descension charge/admit. |
| Chunkgraph | Höchstens 4096 Runtimevertices, 8192 Edges, depth96 einschließlich wrapper/materialhelper. Joint validator vor Veröffentlichung. Childindices/Shape/Lifecycle nicht nur nach Zahlen prüfen. |
| Wrapperbindings | Höchstens128 einschließlich Blendflatcaches und materialhelper. Tatsächlicher Max31. Append/Listenregistrierung checked und original chronologisch. |
| Equality/Hashkeys | Höchstens4096 Equalitykeys, höchstens4096 visitor-map Entries und4096 identity tokens. Vor Append checked. Eigene Key-/Child-/Identityarenen höchstens2MiB; keine rekursiven unbounded String-/Treekeys. Recordtags/typed resource IDs und primitive/child IDs. |
| Graphmetadaten | Eigene zusätzliche Runtimegraph-/Spline-/Origin-/Listarenen höchstens2MiB retained; getrennt von vorhandenen immutable Router/noise Arcs. Cold temporäre Maps/Orders/Keys/Stacks höchstens4MiB, vor Allocation preflight. Beide Limits messen tatsächliche element counts × size_of plus owned slices. |
| Cachearrays | Eigene f64-Slices, Flatgrids, Cellcache und vorreservierte CacheOnce-array storage zusammen höchstens8MiB. Sliceeach `(countXZ+1)*(countY+1)`; Cell `width²*height`; Flat `(noiseSizeXZ+1)²`. CacheOnce reserve max(cellvolume, slicelength) je realem Binding, valid length ist zunächst None. |
| Driverausgabe | Genau256×height f64; höchstens1MiB undhöchstens131.072 Samples. Builtinmax786.432B. Separate Ausgabe/Scratchresult wird erst vollständig zurückgegeben; dieser Cap ist kein Cachearraycap. |
| fillArray temporär | Eigener nach maximal gleichzeitig lebenden ADD-right-arrays vermessener Scratchpool, höchstens4MiB. Länge exakt aktueller Providerarray; checked take/release, DAGdepthbound. Keine pro-Blockallocation; Fehler gibt alle Leases wieder frei oder poisoned Session besitzt sie bis Reset/drop. |
| Preliminarycache | Höchstens2048 einzigartige quartaligned Columns; feste vorreservierte Key-/Value-/Occupancyarrays, höchstens128KiB. Vor neuem Schlüssel checked. Hits bleiben zulässig bei voller Map; kein Evict, Sentineloverwrite oder Nullfallback. Für späteren Aquifer tatsächliche Offsets einschließlich -3Chunk X ausdrücklich abgedeckt. |
| Meter | Ein gemeinsamer1..=268.435.456 Workwert. Kein Reset in Spline/Noise/Arraybranch. Alle echten NoiseOctaves und Endkandidaten wie .2; Wrapperentry auch bei Hit, fillArrayentry, Contextselection, Bufferwrite/-copy pro Slot, ausgeführte Lerp-/Slice-/Celloperationen vor Ausführung charge. Cold keycompare/visitsteps ebenfalls metered. |

Die 8MiB beziehen sich nur auf eigene Sample-/Cachearrays. Keymetadata,
Ausgabe, temporärer Fillpool und vorhandener Router/Samplerbestand zählen
separat. Bei Construction/Reset kann gleichzeitig die alte und die Candidate-
Session leben; das wird dokumentiert und nicht als einfacher Sessionpeak
ausgegeben. Coldtemporaries werden vor Freigabe zurückgegeben. Tatsächliche
Builtin-Rustcounts/Array-/Key-/Graphbytes und Workused aus kompletten Goldenloops
werden im Ergebnisbericht aufgeführt; reicht eine Kappe nicht, gibt es einen
typisierten Fehler und eine konkrete Datenbegründung für Root, keine stille
Capaufhebung oder Referenzvereinfachung.

## Tatsächlicher Coldvisitor und Java-Gleichheit

.2Noisepool ist eine Zahlenbasis, kein Beweis gleicher Javaobjektidentität.
.3 ergänzt privat im BoundRouter ein immutable `Arc` des bereits geprüften
PreparedGraph/Fabrikursprungs und eine Kopie der vorhandenen RouterMetadata.
Der Konstruktor zieht keine JSON-/I/O-/Registrydaten nach. Diese Ergänzung
ändert keine .2-public Sample-/Budget-/Boundsverträge. Sie wird erst nach
.2-CLEAN und Rootfreigabe implementiert, nicht in dessen eingefrorenen Stand.

Der konkrete Consumer rekonstruiert die tatsächlich relevanten Seedvisitor-
Objektgleichheiten aus diesem admitted Ursprung, anschließend NoiseChunkwrap
in originaler Reihenfolge. Maximal begrenzte Factory-/Equal-IDinternierung
vermeidet expanded string/tree keys. Sie ist keine beliebige mathematische
Densitydeduplication und kein Runtimevaluecache.

1. Alle 15 Seedrouterroots werden wie NoiseRouter.mapAll von Barrier bis
   VeinGap traversiert. Children zuerst, originale Builderbounds/actual
   MulOrAdd-Form verwenden. Clamp, Mapped und MulOrAdd rufen in ihrem echten
   mapAll **keinen** visitor.apply auf; diese Ausnahme bleibt erhalten.
2. Java-record Equality benutzt Class, primitive Komponenten und rekursive
   Child-Equality, inklusive tatsächlicher Diagnosefelder. f32/f64 behandeln
   NaNs canonical wie Float/Double.compare und unterscheiden ±0. Arrays
   vergleichen Identität, Lists ihre Elemente. Resource- und enum-Familien
   werden nicht anhand zufällig gleicher Zahlen verschmolzen.
3. Holder.Reference bleibt tatsächliche Registryidentität; mapAll erzeugt
   Holder.Direct mit Value-Equality. HolderHolder wird erst durch actual wrap
   entfernt. Keine pauschale Herkunfts-/Wrapperstripregel auf Native/Decoded.
4. NoiseHolder trägt echte Noiseparemeterholderidentität **und** Sampleridentität.
   Modern getOrCreateNoise teilt resourceweise. Legacy Temperatur/Vegetation/
   Offset konstruiert je tatsächlich ausgeführtem visitNoise eigene immutable
   virtuelle Identity, selbst wenn der Zahlenpool dieselben Bits liefert.
5. Blended und End sind identity-bearing Java-Klassen. Sourcefactorytokens
   unterscheiden actual Nativeinstanzen und actual Codec-unit singletons;
   wiederholter Seedvisitor.apply folgt dessen Map. Sourceindex allein wird
   nicht als Gleichheitsbeweis eingesetzt. Originales Source-/Identitycensus
   aus dem neuen JARhelper prüft diese Tokens explizit.
6. CubicSpline.Multipoint ist ein record, aber locations/derivatives sind
   identitätsgleiche Arrays. mapAll behält deren Originalarrays; gleiche
   Floatinhalte von zwei verschiedenen Sourcearrays dürfen nicht zu einem
   Gleichheitskey werden. Values-List vergleicht Element-Equality und Bounds
   sind Komponenten. Nestedarrays/coordinates vollständig berücksichtigen.
7. NoiseChunk.wrapped canonicalisiert nur tatsächliche visitor.apply Keys.
   Jeder echte mutable Wrapper besitzt eine eigene identity ID. Seine
   registration in interpolators/cellCaches passiert bei erster Neuerzeugung.
   Re-mapAll von NoiseChunk-Wrappern baut Marker aus deren wrapped child,
   sodass der zweite Materialwrap vorhandene richtige Wrapper wiederfindet.
8. Danach den tatsächlichen materialhelper bauen und erneut mapAll/wrap;
   danach cachedClimateSampler über dieselbe Map. Immutable numerische
   Recorddaten dürfen intern geteilt werden, ohne einen behaupteten Java-==
   Vertrag für nicht gecachte Records. Relevant sind die originale equals-
   Relation, identity-bearing Leaves/Arrays und tatsächliche mutable Wrapper-
   Sharingpartition samt Reihenfolge/Requests.

Die komplette Gegenprobe vergleicht echte Keyclasses/Equalrelation und
Wrapperidentitypartition; der bloße Zahlen-Census31/12/5 genügt nicht.

## Die fünf Wrapperverträge

| Wrapper | compute / fillArray / Kontext |
| --- | --- |
| NoiseInterpolator | Eigenes Context ohne laufende Interpolation -> Fehler. Fremder SinglePoint -> noiseFiller.compute ohne Lerp. Bei fillingCell original Mth.lerp3 mit X→Y→Z-rechenordnung; normal gespeichertes Y→X→Z-Updatevalue. Zwei Slices, acht originale Cornerfelder; `fillArray` bei fillingCell provider direct, sonst wrapped.fillArray. |
| FlatCache | Eager inclusive Quartgrid bei Y0, X außen/Z innen. Für16×16 exakt5×5. Compute arithmetic>>2 und relative Quartindices; in-grid Value ignoriert Y, out-grid echter Childcontext. fillArray provider direct. Zwei Blendflatcaches werden auch bei empty Blender genau als Konstruktorarrays angelegt/befüllt, obwohl BlendAlpha/Offsetenum dann nicht zu diesen Wrappern ersetzt wird. |
| Cache2D | Genau letzter gepackter Block-X/Z-Key. Anfang ist **ChunkPos.INVALID_CHUNK_POS = asLong(1875066,1875066)** und lastValue+0. Ein erster Compute an diesen gültigen Koordinaten gibt daher tatsächlich+0 ohne Child. Kein Option-Schlüssel als vermeintliche Korrektur. Key wird vor Child gesetzt; Childfehler poisoned Session. fillArray delegiert direkt an function und hat keinen eigenen Mehrkeycache. |
| CacheOnce | Own Context: erst lastArray && gleiche arrayCounter, dann gleiche scalarCounter, sonst Child. Fremder Context delegiert. Initialcounter0/lastValue+0 beibehalten. fillArray: bei gleicher Arrayepoch copy Originalprefix, sonst vollständig wrapped.fillArray, gleiche Länge copy oder neue logical storage length, danach epoch. Vorreservierte Kapazität darf None/valid length nicht voreilig verändern. Counterassignment-/Failuretiming beachten. |
| CacheAllInCell | Array width²×height, registriert bei Konstruktion. Fremder Context delegiert. Eigen ohneInterpolation Fehler. Eigene in-cell coordinates lesen `((height-1-inY)*width+inX)*width+inZ`; außerhalb Cell delegiert. fillArray provider direct. Cellfill läuft für cellCaches in echter registration order; kein paralleles oder dependency-sortiertes Valuecachefill. |

## fillArray und Provider: eigene vollständige Dispatch

Keine Ersetzung von originalem fillArray durch einen universellen Pointloop.
Auf Scalarparität allein lässt sich die Arrayrequestfolge nicht schließen.

- Constant/BlendAlpha/BlendOffset/Beardifier füllen das gesamte Array direkt.
- Holder/ursprünglicher Marker delegieren; gewrappte Formen wie oben.
- PureTransformer füllt zuerst den Childarray vollständig und transformiert
  danach slots aufsteigend. MulOrAdd verwendet diesen Pfad und überspringt
  sein Input bei argument0 ausdrücklich nicht.
- Ap2.ADD: linkes Array vollständig, rechtes vollständiges Scratcharray,
  danach slotweise Addition. Ap2.MUL/MIN/MAX: links ganz, dann nur für tatsächlich
  erforderliche Slots rechts `compute(provider.forIndex(i))`. Rechtes fillArray
  ist dabei falsch. Signedzero/strict bounds wie .2.
- RangeChoice: Inputarray ganz, dann pro Slot genau ausgewählten Childcompute
  über forIndex; keine zwei Candidatearrays.
- TransformerWithContext: Childarray ganz, danach forIndex je transformiertem
  Slot; dies betrifft BlendDensity und WeirdScaledSampler.
- Noise/Shift/ShiftedNoise/Spline/YGradient/End und SimpleFunction fallback:
  provider.fillAllDirectly mit der tatsächlichen Function. Finitepolicy gilt
  auch jedem berechneten/geschriebenen Arraychild; refusal stoppt den nächsten
  Slot statt einen stillen endlichen Ersatz zu schreiben.

Sliceprovider forIndex/directfill setzt Y `(i+cellNoiseMinY)*cellHeight`, erhöht
scalar interpolationCounter je Index, setzt inY0 und arrayIndex=i. Cellprovider
forIndex: Z=i%width, X=(i/width)%width, Y=height-1-i/(width²), setzt arrayIndex=i
und erhöht den Scalarcounter **nicht**. Cell fillAllDirectly beginnt arrayIndex0
und loopt Y absteigend, X aufsteigend, Z aufsteigend. Java evaluiert in
`values[this.arrayIndex++] = function.compute(this)` den Arrayindex und dessen
Postincrement **vor** dem RHS-compute: der Child sieht i+1, nicht i. Slice-direct
und Cell-forIndex sehen weiterhin i. Dies ist das durch actual-JAR-Providertrace
bestätigte, von Root freigegebene interne Evaluation-order-Amendment; der
frühere Proposaltext mit compute vor Postincrement war falsch.
Current provider object identity entscheidet own vs foreign.
Arraylength/-indexlimits werden vor Dispatch geprüft. Provider ist privat,
kein Callercallback kann Counter oder Contextidentity einschleusen.

## Interpolationsdriver: genaue private Statefolge

1. `initializeForFirstCellX`: interpolating=true, scalarCounter=0; slice0 bei
   firstCellX füllen. Für jede Zcornerspalte arrayCounter++ vor allen
   InterpolatorfillArrays, anschließend nochmals arrayCounter++.
2. CellX aufsteigend: advanceX füllt slice1 bei firstCellX+cellX+1 mit derselben
   Sliceproviderfolge; dann cellStartX auf tatsächliches linkes Cell setzen.
3. CellZ aufsteigend, CellY absteigend: selectYZ kopiert zuerst die acht
   Cornerwerte sämtlicher Interpolatoren. fillingCell=true, cellStartY/Z setzen,
   arrayCounter++; registrierte Cellcaches durch ihren noiseFiller.fillArray
   füllen; arrayCounter++; fillingCell=false.
4. inCellY absteigend: updateY setzt inY und vier Lerps; inCellX aufsteigend:
   updateX setzt inX und zwei Lerps; inCellZ aufsteigend: updateZ setzt inZ,
   scalarCounter++ und finalen Lerp. Jetzt materialdensity.compute(ownContext)
   und an überprüftem Ausgabeindex schreiben.
5. Nach sämtlichen Z/Y-Zellen swapSlices nur einmal für diese Xsäule; nach
   letzter Xsäule stopInterpolation. Alle illegalen private Sequences erhalten
   typed Lifecyclefehler. Kein zweites initialize oder stale selectedCell.

Originaler normaler Updatepfad ist Y→X→Z, originaler fillingCell Mth.lerp3
rechnet X→Y→Z. Beide werden separat bitweise geprüft; algebraische Gleichheit
erlaubt keine gemeinsame umsortierte Lerpimplementierung. Counters sind private
i64 mit Java-wrap semantics; im realen capped Driver sind deren Overflows
unreachable. Near-overflow ist eine private Syntheticoraclekontrolle, keine
öffentliche Manipulationsmöglichkeit.

## Preliminary Surface und gecachtes Climate

Preliminary x/z arithmetic>>2 und checked×4 zum Quartursprung. Lookup per
originalem ColumnPos32/32-Packing. On miss: von minY+height **inklusive** abwärts
in cellHeight-Schritten bis minY; tatsächliche wrapped initialDensityroot mit
fremdem SinglePointcontext. Erste density >0.390625 ergibt dieses Y, andernfalls
Integer.MAX_VALUE. Gleichheit0.390625 genügt nicht. Erst erfolgreicher kompletter
Scan veröffentlicht den cache Entry. Out-grid Flatcachefallback und letzte
Cache2D-Keys sind deshalb für diese Abfrage relevant.

CachedClimate bildet originale sechs Seedrouterfields nochmals mapAll(wrap)
über die bestehende session Map ab. Es benutzt ausdrücklich nicht den .2-
Climateview, dessen separater Visitor Holder/Marker entfernt. sample(Quart)
ist ein fremder SinglePoint am checked Quartursprung, sechs Fields in originaler
Reihenfolge, f64→f32→originale 10.000f32-Quantisierung. Keine Biome-/Spawnsuche;
deferred spawnTarget wird nicht als fertiger Resolver ausgegeben.

## Original-JAR-Oracle und konkrete Acceptance

Neue Files `tests/reference/noise_chunk_oracle.java`,
`generate_noise_chunk.py`, `noise_chunk.md`,
`tests/fixtures/noise_chunk_1_21_8.jsonl`, `...manifest.json` und ein begrenztes
Bigendian-u64-Datafile mit vollständigen Paper-Materialdensitybits. Keine neue
Rustdependency: erwartete f64-Bits unmittelbar mit acht Fixturebytes vergleichen.
Fixtureoffsets/-lengths vor Zugriff prüfen; Manifest pinnt SHA256 und Bytegröße.

Der Helper ruft tatsächliche Constructor-/wrap-/fillArray-/initialize-/advance-/
select-/update-/swap-/stop-/Preliminary-/Climate-Methoden aus Vanilla und Paper
auf. Keine kopierte Wrapper-/Noise-/Lerpformel. Reference-only synthetische
DensityFunction-Proxies loggen Inputs/Arrayrequests und liefern deklarierte
Werte oder Fehler; Javaoriginal entscheidet sämtliche Branches und Caches.
 reflection liest actual MaterialRuleList-Lambdacapture und tatsächliche
Wrapper-/Counter-/Slice-/Cachefelder. Traceprobes und vollständige Numerical-
Runs laufen getrennt, damit zusätzliche Observations keine Hotlooprequests
verändern. Counter-/wrapper IDs sind traversalgebundene fixturelokale Ordinals.

Konkrete Mindestmatrix:

- 98 Constructor-/sharing/resource Fälle wie Planningcensus, beide Origins
  gegen actual constructor/mapAll, sämtliche15roots, materialhelper und cached
  climate roots. Produktions-Rustabnahme verwendet den freigegebenen decoded
  Ursprung; native Registryfälle kontrollieren Herkunft separat.
- Vollständige Materialdensityarrays für7Settings ×2Seeds (0 und asymmetrischer
  0x0123456789abcdef) ×6Chunks `(0,0),(1,0),(0,1),(-1,0),(0,-1),(-1,-1)`:
  **84 vollständige Chunks**, alle Blockbits, actual ownDriver counters,
  wrapperslices vor/nach jedem Xswap und Cellcachearrays. Vanilla/Paper dieser
  nahen Numericalfälle zusätzlich gegeneinander festhalten.
- Zusätzlich tatsächlicher Paper-End: alle7Seeds ×6 ferne Chunkfälle bei
  sign-/i32overflowgrenzen und ±30M, darunter der Rootgegenbeleg(-524289,0,0):
  **42 vollständige Endchunks**. Endcorner-/SinglePointfallbackseam gegen alle
  658 vorhandenen Rootpositionspaare; Vanillaabweichungen ausdrücklich erhalten.
- Datafilegröße für84+42Chunks:7.077.888 Werte /56.623.104Bytes; nur ein
  festgelegter Materialdensitywert pro Block. Trace/corner/root diagnostics
  separat und bounded. Keine doppelte Komplettausgabe je Observationsfeld.
- Alle7Seeds/7Settings: ausgewählte Cellcorner, Boundary-/Nextafter-Lerps,
  sliceProviderforIndex und directfill, cellProviderforIndex und directfill,
  initial/bare-final Values in separaten Acceptance-Sessions; tatsächliche
  wrapped Bounds und request order. Alle kompletten Arrays zusätzlich bei
  fresh Session und reordered Chunkrequests, reset/clone Seedrouter/droporigin.
- Preliminary: alle25 inclusive Quartgridcolumns plus mindestens32 outside-
  grid/Aquifer-offset columns je7Settings/7Seeds, negative unaligned Adressen,
  Grenze0.390625 mit Nextafter, min/topY und MAX_VALUE; miss/hit/poison/refusal.
- CachedClimate: alle ursprünglichen294 Quartproben in echten NoiseChunk-
  Samplern, mindestens je Setting Grid-/Outgridfälle und Yextrema; actual
  resource/key equality und quantized longs, keine Biomesuche.
- Jeder Wrapper isoliert und verschachtelt: Hit/Miss, foreign/own, in/outcell,
  inclusive/outgrid, repeated/shared child und material secondwrap. Cache2D
  erster Sentinelhit mit +0 und ohne Child, Yänderung und fillArraybypass.
- CacheOnce tatsächliche arraycounterhit/miss, prefixcopy, gleiche/geänderte
  logical Länge, forIndexindex, counter0/scalarcache, near wrap und Childfailure;
  builtinlastArray=null ist kein Ersatz für diese Abnahme.
- Actual Java equals controls für f64/f32±0 undNaNs, Holderdirect/reference,
  moderne shared gegenüber frischen LegacyNoiseidentitäten, gleiche/verschiedene
  Splinearrayobjekte bei gleichen Bytes, Blended-/Endidentity, gleichwertige
  getrennte Marker versus wiederholte Native Namedreferences.
- Vollständige fillArray-Branchrequestspuren: ADD-links-all/rechts-all,
  MUL-±0-skip gegenüber MulOrAddzero, MIN/MAX strict Bounds, RangeChoice-
  selectedchild, contextual TransformforIndex, OriginalLerp3 gegenüber normalen
  Updates. Failureprefix/nonfinite Policy separat von Javaexceptionsemantik.
- Rust N/N-1 Meter an construction, slicefill, cellfill, Noise/Splinecallbacks,
  Driveroutput, preliminary und climate; kein lokaler allowance Reset.
  Vertex/edge/depth/visit/key/wrapper/array/output/temp/surfacecaps genau an
  Grenze/+1. Callerresult bleibt bei failure unveröffentlicht, bestehender
  Router und getrennte Session bleiben unverändert. Fehler/reset Lifecycle.

Finalgenerator übernimmt die bestehenden Vanilla Bundle/Inner/43-Library-
Mappingspins und Paper Bundle/Inner/119-Librarypins. Jeder actual verwendete
Originalcallee bekommt einen geprüften CodeSource. Frischer Helper zuletzt
im Classpath, shadow/envinjection refusal, Inputs vor/nach, normale/-O
byteidentische Fixture **und** Manifest. Negativeguards für falsche Bundle/
Inner/Library/Mapping/source/helper-origin/shadow/env auch unter `-O`; keine
Pythonassert-Pinguards. Serverboot/Client/Gameplay bleiben separate Gates.

Eigene Abschlusschecks: vollständige World-Debug und-Release, strict
all-targets Clippy, fmt, Scope diffcheck, unveränderte alte Fixtures/Oracles und
Dependencyinputs, Source-/Calleematrix, vollständiger Filefreeze. Root führt
isoliertes unabhängiges Runtime-/API-/Errorreview und Zusatzprobes aus.

## Gelesene Quellen und Paperänderungen

Vollständig gelesen: NoiseChunk790, NoiseBasedChunkGenerator428,
DensityFunction119, DensityFunctions1365, RandomState154, NoiseSettings59,
NoiseRouter68, Holder einschließlich Gleichheit und der tatsächlichen
Paperänderung, CubicSpline-Multipoint mapAll/create/apply und Constant;
Aquifer komplett einschließlich realer PreliminarySurface-Offsets. Mth.sqrt,
lerp/lerp2/lerp3, clamp und floorDiv sowie QuartPos, ColumnPos und
Climate.Sampler vollständig in ihrem relevanten Vertrag gelesen.
Blender/Beardifier-/Noise-/Splinecallees wurden bereits vollständig in .2
gelesen; der neue Consumer bleibt bewusst empty/no-structure.

NoiseChunk Base/Applied bytegleich SHA256
`e3c872cf3f6a15a063d255f1188296ea841ef012b01959d35ca7e0012d97c863`.
DensityFunctions Base
`3f1002ec0d0d8047fec58fa256c1625c7291f8fa47b96a429ff8687f58e88693`,
Applied `62b4422976740e5ea5115b7675cfb404bec6887c15ca232e79bbeaf35480a749`:
vollständiger Endpatch oben berücksichtigt.
NoiseBasedChunkGenerator Base
`1aee1be9952d347975d2a464da06ee330dc0980aba1a979ceca9d18e6d89dc71`,
Applied `2b586e7fca9059b65d214f33f6b84fcc7ae3de90285ad7cd9c8a56edc319f2d8`.
Sourcepatch verändert spätere Bedrock-/Carvercontexts; vollständiger relevanter
Moonrisehunk macht Executors Runnable::run und verändert nicht die geprüfte
Interpolation. Kein Rust-Thread-/Schedulingclaim daraus. Streams-Beardifier-
Patches betreffen den hier explizit ausgeschlossenen Structurebeitrag.
Holder-Paperänderung benutzt für Tagkeys reference Set, verändert weder
Directrecord-Equality noch Referenceobjektidentität des konkreten Consumers.

## Implementierungsreihenfolge nach Rootfreigabe

### Freigegebenes internes Paper-Build-60-Amendment: nextDouble

Der erste vollständige actual-Paper-Dichtevergleich hat einen zusätzlichen
numerischen Unterschied konkret nachgewiesen. Paper Build 60 berechnet
`XoroshiroRandomSource.nextDouble` **und** `BitRandomSource.nextDouble`
(damit auch LegacyRandomSource) mit `l2f`, `1.110223E-16F`, `fmul`, `f2d`.
Vanilla verwendet `l2d`, double-Multiplikation. Die vollständigen Paper-Base-
und Appliedquellen sind jeweils bytegleich: Xoroshiro SHA256
`76a1af8439d57038d3e80ea41dc7b38ca3d9cbfe3a8db8bd87982b5aa363e70f`,
BitRandomSource SHA256
`726528fb348082c248218899112a2d6fba1a1897dad719aa41939eff643ff2c1`.
Actual JAR-Bytecode wurde von Root unabhängig geprüft: Paper-Dump SHA256
`d6a7cb7499dc0bed7c8519ec0d5d3eefc6f0e5af75dadd748af2f2d48d1743e4`,
Vanilla-Dump
`430611872a9bca2b68e115264103f962c30c81903b04d9df1c3de74e777b8e5e`.
ImprovedNoise Base/Applied SHA256
`f522c414939fe6ca27b94a1e758d2ab644d3ba43cbad201dd4368fb9fa0abc10`.

Root hat ausdrücklich ausschließlich für die neuen .3-Pfade die konkrete
Paper-Zahlenbasis freigegeben. `density/evaluate.rs::MarkerEvaluator` wählt
privat den Session-Samplerpool; `noise.rs` und `noise/{normal,blended}.rs`
kopieren die bestehenden Immutable-Permutationen und runden die drei
Improved-Offsets über f32. Dies ist exakt zur tatsächlichen Java-Konstruktion:
nextDouble-Skalierung und Offsetmultiplikation mit 256 sind Zweierpotenzen,
Draworder und alle 256 Shufflewerte bleiben erhalten. Ein durch Rundung
entstandener Offset 256 wird ausdrücklich nicht geclampt. Auch Legacy-
Normal- und Blended-Stacks verwenden diese private Basis. End-Simplexoffsets
sind im tatsächlichen sampler irrelevant; seine unveränderte Permutation
wird weiterhin mit der freigegebenen Paper-i64-Radialpolitik verwendet.

Alle Offset-/Permutationkopien verbrauchen denselben .3-Meter und werden vor
Allocation im zusätzlichen 2MiB-Runtime-Metadatencap zugelassen. Keine globale
RNG-Option, keine Änderung an öffentlichem RNG/Noise/Scalar/Climate-Verhalten
von .2, keine Vermischung von Vanilla-/Paper-Sampleridentitäten. Unabhängige
actual-noise-/Legacy-/Materialgoldens und sämtliche .2-Regressionen bleiben
Abnahmepflicht. Die Beobachtung impliziert keine zusätzliche Gameplay- oder
Performance-Abnahme.

1. Neue echte Oracleconstructor-/Identity-/Wrapper-/Array-/Driverfälle und Pins
   exportieren; tatsächliche Key-/visit-/resource Counts gegen freigegebene
   Caps aufnehmen. Bestehende .2-Fixtures bleiben unverändert.
2. Metadata-/Sourcearc ergänzen, kleine private Paper-Endnumericalseam mit
   unverändertem .2-Nachweis; bounded Coldvisitor und five-wrapper binding.
3. Originale fillArraydispatch und Provider implementieren, synthetic actual
   Request-/Failureprefixnachweise zuerst. Danach kompletter Driver/Sink.
4. Preliminarycache und gecachtes Climate; vollständige84+42Numericalarrays,
   Caps/Meter/poison/reset. Concrete bytes/Workused im Bericht festhalten.
5. Debug/Release/Clippy/fmt, unveränderte Inputs, Freeze und unabhängiges Review.
   Danach direkt Aquifer/Ore/Material → Biome/Surface/Bedrock → echte Noise-
   Blockausgabe mit passender Generatoridentität und späterer aktiver Adoption.

Carvers, Features, normale Oreplacements, Vegetation, Structures, Licht,
Gameplay/Survival und vollständige Paper-/Java-Pluginfähigkeit werden durch
dieses Numericalticket nicht als erledigt markiert. RAM-/TPS-Vorteile werden
erst bei gleicher aktiver Funktionalität gemessen.

## GEN-05.3 — gelieferter Worldconsumer und konkrete Abnahmematrix

Der neue `SeededRouter::fresh_paper_noise_chunk` besitzt seine mutable Session
exklusiv. Seine öffentliche Ausgabe ist ein vollständiges endliches
Materialdichtearray **vor** Aquifer oder Block-/Fluidauswahl. Die Runtime
implementiert originale seed/mapAll/wrap-Chronologie, alle fünf Wrapper,
Arraydispatch, Slice-/Cellprovider, originalen vollständigen Driver, Preliminary
und cached Climate; immutable Quellen und der öffentliche .2-Vanillavertrag
bleiben erhalten. Die beiden vorstehenden konkreten Amendments sind umgesetzt.
Papers Threadlocal-/WeakHashMap-Endcache bleibt ausdrücklich außerhalb dieser
Stufe; alle625 Kandidaten und tatsächlich ausgeführten Simplexproben sind metered.

| Direkte Runtimeabnahme | Tatsächlich verglichener Umfang |
| --- | --- |
| Vollständige Paperdichten | Alle126 eingefrorenen vollständigen Chunks /7.077.888f64-Werte bitgenau; sieben Settings, signed Seeds,84 nahe und42 entfernte Endchunks. Ein Datafile56.623.104B, SHA`9dd6827609a0a6b65968b239a588fd60e21121097520501ff5828eb19655a9db`. |
| Constructor/Sharing | Alle49 decoded Settings/Seedfälle: vollständige zweite Traversallänge/-reihenfolge, Keyclasses und Java-equals-Partition, Valueclasses, mutable Wrapperidentitäten/Registrierungsreihenfolge, alle15 Roots, Materialroot und6 shared Climateroots. Native49 Oraclefälle bleiben Originaldiagnosen; die Worldsession konsumiert decoded Settings mit tatsächlichen registered Densitytargets. |
| Getrennte Sessiondiagnostik |49 Fälle: echte Initial-/bare-Final-Fremdpunkte, vollständige Slice0/1-, Cellcache- und Cornerarrays, vier ausgewählte Driverproben samt tatsächlichem Kontext, getauschte Slices und tatsächliche Counter. Keine erfundene Counterformel für komplette Chunks. |
| Alle fünf Wrapper | Sämtliche55 tatsächlichen Controls direkt mit Ergebnisbits/Fehler, Sessionzustand und exakter Child-/Providertrace; isolierte, nested und shared Pfade, Sentinel, counter0, logical Arraylängen/Prefixe, scalar/array-overflow und Failuretiming. |
| Provider | Alle12 originalen Slice-/Cell-forIndex/directfill-, Cellregistrierungs- und scalarCounterwrapcontrols; exakte Computecontexts/-order und Arrays. Cell-direct i+1 und one-past-end CacheOnceprefix separat geprüft. |
| Arraydispatch | Vollständige23-Familienkontrollen mit7 unveränderten endlichen Original-Failureprefixen und16 neuen tatsächlichen endlichen Addendumrows, dazu9 originale SharedOnce-/Unit-/Gradient-/Weird-/own-directcontrols. Child-/Providerrequests, ±0-Bits, strict MIN/MAX, selected RangeChoice, context-transformers und Failureprefixe stimmen. |
| Preliminary und Climate |49×57 Preliminaryspalten inklinclusive25grid/offgrid32 und cached Hits;49×22 Quartproben: alle6f32-Bits und quantized longs. Grenzwerte/Min-/Top-/Sentinel/2048cap/fehlendes Cachepublish/finite f64→infinite f32 sind zusätzliche gezielte Rustprobes. |
| Random/Noisebasis |14 actual-JAR RNGcases: private f32-rounded nextDouble-/Improvedoffsets und alle256Permutationbytes. Alle erreichbaren named-noise-Stacks aus7 tatsächlichen Normaloffsetinventaren bitgenau; private legacy Temperature/Vegetation/zerooffsetsampler fehlen im Java-named-Mapinventar und werden durch die vollständigen Dichte-/Climatediagnosen geprüft. |
| Paper Endseam | Alle658 tatsächlichen Interpolator-Foreignfallbackwerte in frischen pro-Seed-Sessions und umgekehrter Reihenfolge bitgenau. Die42 Far-End-Cornercases sind zusätzlich in vollständigen Dichtechunks abgedeckt; Original-Vanilladrifts bleiben unverändert erhalten. |
| Eigene checked Policy | N/N−1 für Constructor, Surface/Climate, Slice/Cell und Driver mit einem Meter; public Poison/Reset und keine Teilpublikation; volle-/übervolle Key-/Identity-/Graph-/Wrapper-/Visit-/Depth-/Bytesarenen; child-before/type/cycle; private Lifecycle/shape/index/x/deltaadmission vor Mutation. |

Die ursprünglichen Java-NaN-Fälle bleiben unverändert im Corpus. Rust stoppt
bereits am ersten angeforderten nichtendlichen Child/Arraywrite; spätere
Java-Requests und Exceptions sind deshalb **kein** erfolgreicher Tracevergleich.
Das endliche Addendum ersetzt ausschließlich den letzten deklarierten NaN-Wert
mit6.0 und ruft weiterhin dieselben tatsächlichen Originalfactories/methods auf.
Immutable Java-record-Identitydiagnosen und23 originale equalsControls sind
retained; gleiche Rustallocationidentity wird nicht behauptet. Die tatsächliche
vollständige Constructor-equals-Partition und mutable Sharing sind verglichen.

Die Originalquellen/Callees sind in
`server/crates/ferrumc-world/tests/reference/noise_chunk_sources.md` samt vollständigen
Base/Appliedpins dokumentiert: tatsächliche NoiseChunkconstructor/wrap/wrapNew,
alle Wrappercompute/fillArray, initialize/advance/select/update/swap/stop,
Preliminary/Climate sowie DensityFunctions Ap2/MulOrAdd/RangeChoice/
PureTransformer/TransformerWithContext, Mth.lerp3 und originale RNG/noises.
`source-pins.json` und die Manifeste pinnen zusätzlich Bibliotheken, JDKmodule,
offizielle Mappings und vollständige reconstructed sources/featurepatches.
Quellenbytegleichheit ersetzt wegen des konkret nachgewiesenen nextDouble-
Bytecodedrifts keine tatsächliche JARabnahme.

Das unabhängige Review fand zwei private Lifecyclelücken, die gezielt korrigiert
wurden: Update-/Selectaufrufe konnten vor initialisierten Slices laufen; der
Materialeintritt konnte nach swap stale Cellwerte lesen. Private DriverPhase/
Columntracking verweigern illegale Übergänge vor Arbeit/Cachemutation. Der enge
`sample_material`-Eintritt verlangt Running/ZUpdated, und drive verwendet ihn.
Rohe Originalmarker-/Providerseams bleiben in ihren Sentinel-/selektiven Probes
unverändert; es wird keine exhaustive private Cellzählung erfunden. Publicdrive
führt vollständig X/Z/descending-Y/descending-inY/inX/inZ aus und prüft genau
eine Publikation je Outputslot über ein separates maximal16KiB-Bitmap.

Die Budget-/Speicherkategorien entsprechen der freigegebenen Tabelle. Beispiel
Seed0/Origin im Messharness vor test-only Tracefeldern: Overworld822Vertices,
31Wrapper,254Visitorentries,40.528Cachebytes,9.216Scratchbytes und570.916
Constructorworkunits; Amplified825Vertices und563.000Units, End105Vertices/
5Wrapper/63Entries/4.032Cachebytes/10.240Scratchbytes/47.196Units. Retained
Graph-/samplermetadaten werden als tatsächliche `size_of`-Elementbytes mit
getrennten Limits geprüft. Test-only DeclaredLeaf/Foreignproviderfields erhöhen
Messharnessmetadaten; daraus folgt keine Prozess-RSS- oder TPSmessung.

Owned Dateien liegen unter `tests/fixtures/noise_chunk_1_21_8/`, Runner und
Reproduktionsbeschreibung unter `tests/reference/`. Die immutable Exporte sind
vollständig gehasht: Hauptoracle117files SHA
`0ece7c737a023744d6124aa5fb3cf5b7d6bf9c53f9af094cf9466ba21dcf4582`,
Root126 SHA`80e17003c7229cf769bda08a3b90094f27fc9bded97032d1991a294813f6bb20`,
finiteAddendum42files SHA
`a111bf5647885c643e8cbdde4b950f6342430ef5191daeca6f02b25a014f163f`.
Die umgezogenen eigenen Originalrunners bestehen normal und Python−O je27
konkrete ENV/Bundle/Inner/Library/Source/Mapping/Helper/JDK/Shadow/CodeSource-
Refusals; das finite Addendum separat je8. Alle26 alten Producer-/Referenceinputs,
alle5 ursprünglichen .2Oraclefiles und sämtlichen gepinnten Dependencyinputs
wurden unverändert gehasht (`.codex-tmp/gen05_3-input-check-final.json`).

Wholecrate Debug/Release bestehen jeweils229Tests ohne ignoredTests; Strict
Clippy und Formatierung bestehen. Die abschließende unabhängige Runtimeprüfung
und Rootintegration bleiben bis zu ihrer ausdrücklichen CLEAN-Meldung offen.
Aquifer/materialblocks/fluids/ores folgen erst mit GEN-05.4; Biome/Surface/
Carvers/Features/Structures, natürliche Generatoridentität/aktive Adoption,
Licht, Survival/Clients und Java-/Paperplugins bleiben eigene offene Tickets.
Keine Spielbarkeits-, Vollständigkeits-, RAM- oder TPSverbesserung wird aus
GEN-05.3 abgeleitet.
