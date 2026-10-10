# GEN-05.2 — freigegebene Seed-, Scalar- und Climateberechnung

Stand 2026-10-10. **Konkrete Produktionsfreigabe durch Root nach vollständiger
Vertragsprüfung**, Agent `item_transfer_primitives` implementiert. GEN-05.1 ist
unabhängig CLEAN und auf `main` als `c4e3dc76` integriert, siehe
[Compilerreview](GEN-05_1_DENSITY_COMPILER_REVIEW.md).

Freigegebener Vorschlag `.codex-tmp/gen05_2-activation-proposal.md`, SHA256
`9b92ed683d495d5e8ddea964b046f44a1d90e7c8ac089cb811c24b94bf84b177`.
Dieses Dokument archiviert den vollständigen geprüften Consumervertrag.
Root besitzt diesen Freigaberecord; der Implementer besitzt die Worldcrate und
den gemeinsamen [GEN-05-Brief](GEN-05_DENSITY_COMPILER_ROUTER.md).
Die folgenden APIs, Grenzen und Abnahmen sind verbindlich. Produktionscode,
Seed-/Scalar-/Climate-Goldens und ihre unabhängige Abnahme sind noch offen.

Nach der Freigabe hat Root die getrennte tatsächliche Vanilla-/Paper-
EndIsland-Helferprobe abgeschlossen:658 Paare,266 Unterschiede, normal/-O
bytegleich. Der [Referenznachweis](../PAPER_REFERENCE.md#tatsächliche-endisland-helferprobe)
ergänzt die folgende ursprünglich offene Paper-Evidenz; er verändert den
freigegebenen Vanilla-Consumeralgorithmus oder seine Finitepolicy nicht.

## Ergebnis und begrenzter Scope

Die echten Minecraft-1.21.8-Definitionen werden erstmals mit einem expliziten
Seed verbunden und im originalen `SinglePointContext` ausgewertet. Alle sieben
Settings, ihre 15 Routerfelder und alle 35 benannten Densitydefinitionen werden
gegen echte Original-JAR-Ergebnisse abgenommen. Ein eigener Climate-View folgt
dem zweiten tatsächlichen `RandomState`-Visitor. Keine Chunks, Wrappercaches,
Blockausgabe, Aquifer-, Surface-, Biome- oder Spawnauflösung werden damit
aktiviert. Die bestehenden Flat-Generator-/Persistenzidentitäten ändern sich
nicht. Änderungen bleiben in `ferrumc-world/**` und dem GEN-05-Taskbrief; Root
besitzt Git, Lockfile, Integration und unabhängige Abnahme.

Numerische Referenz für .2 ist ausdrücklich **Vanilla 1.21.8**. Paper Build 60,
Commit `29c8822d90899c89d2689338e81a98f690bcba12`, ist die gelesene zusätzliche
Quellreferenz. Sein abweichender End-Island-Pfad benötigt eine eigene tatsächliche
Paper-JAR-Abnahme vor einer späteren Terrainaktivierung mit Paper-Paritätsanspruch.

## Freigegebene exakte öffentliche API

Alle neuen Typen liegen in `ferrumc_world::density`. `WorldSeed` und der absolute
`ferrumc_math::BlockPos` werden unverändert wiederverwendet. Es gibt keine
öffentlichen Nodehandles, Caller-JSONs oder Caller-Noises.

```rust
RouterTemplate::bind_seed(&self, seed: WorldSeed)
    -> Result<SeededRouter, DensityBindError>;

DensityWorkBudget::new(units: usize)
    -> Result<DensityWorkBudget, DensitySampleError>;
DensityBatchBudget::new(units: usize)
    -> Result<DensityBatchBudget, DensitySampleError>;

SeededRouter::seed(&self) -> WorldSeed;
SeededRouter::resource(&self) -> NoiseSettingsResourceId;
SeededRouter::field_bounds(&self, field: RouterField) -> DensityBounds;
SeededRouter::definition_bounds(&self, id: DensityFunctionResourceId)
    -> DensityBounds;
SeededRouter::climate_bounds(&self, axis: ClimateAxis) -> DensityBounds;

SeededRouter::sample_field(&self, field: RouterField, pos: BlockPos,
    work: DensityWorkBudget) -> Result<f64, DensitySampleError>;
SeededRouter::sample_definition(&self, id: DensityFunctionResourceId,
    pos: BlockPos, work: DensityWorkBudget) -> Result<f64, DensitySampleError>;
SeededRouter::sample_climate_at_block(&self, pos: BlockPos,
    work: DensityWorkBudget) -> Result<ClimateSample, DensitySampleError>;
SeededRouter::sample_climate(&self, pos: ClimateQuartPos,
    work: DensityWorkBudget) -> Result<ClimateSample, DensitySampleError>;

ClimateQuartPos::new(x: i32, y: i32, z: i32)
    -> Result<ClimateQuartPos, DensitySampleError>;
ClimateQuartPos::from_block(pos: BlockPos)
    -> Result<ClimateQuartPos, DensitySampleError>;
ClimateQuartPos::block_origin(self) -> BlockPos;

DensityRequest::field(field: RouterField, pos: BlockPos) -> DensityRequest;
DensityRequest::definition(id: DensityFunctionResourceId,
    pos: BlockPos) -> DensityRequest;
SeededRouter::sample_batch(&self, requests: &[DensityRequest],
    work: DensityBatchBudget) -> Result<Box<[f64]>, DensitySampleError>;
```

`SeededRouter` ist `Clone` und besitzt ein immutable `Arc` mit gebundenen
Graphen und Samplern. Sampling benötigt weder RNGmutation, I/O noch Locks.
Die einfachen Point-/Climateoperationen allozieren nicht. Batch allokiert nur
sein vorab begrenztes Resultatscratch und veröffentlicht es ausschließlich
nach vollständigem Erfolg. Es gibt keine neue öffentliche mutable Session in
.2 und keine neuen Defaultsettings/-seeds.

`DensityBounds` ist ein nicht frei konstruierbarer `Copy`-Metadatenwert mit
`min_value()/max_value() -> f64`; beide Originalbitmuster bleiben erhalten.
Bounds sind die **originalen Diagnostics**, kein garantierter mathematischer
Hull und keine Admission-/Sampleclamps. Auch ±Infinity, NaN und reversed Bounds
werden in dieser Metadatenrolle nicht normalisiert oder abgewiesen. Die private
Testprojection prüft zusätzlich tatsächliche Factoryklasse und Originalherkunft.

`ClimateAxis` enthält genau Temperature, Humidity, Continentalness, Erosion,
Depth, Weirdness. `ClimateSample` besitzt in dieser Reihenfolge sechs originale
f32-Werte und sechs quantisierte i64-Werte, nur Getter/Arraygetter. Es enthält
weder Biome-ID noch Spawnposition. f64-Rohwerte werden durch die normale
Fieldprobe nachgewiesen; die Climatewerte kommen aus dem Climate-View.

Alle absoluten Samples prüfen x/z **inklusive ±30_000_000**; y unterstützt den
vollständigen i32-Blockbereich, damit Pointproben außerhalb einer gewählten
Noisegeometrie nicht still auf deren Yrand geklemmt werden. Settingsgeometry
bleibt Metadatum und bindet erst .3 die tatsächliche Chunkdomain. Tests decken
die jeweiligen Geometrylimits und i32-Yextrema gesondert ab.

`ClimateQuartPos` ist World-owned und typed; es erweitert keine fremde Crate.
Seine Konstruktion prüft die Umrechnung mit i64-Zwischenrechnung **vor** einem
Cast: x/z *4 müssen im obigen Blockbereich und y *4 im i32-Bereich liegen.
Für angenommene Werte entspricht das exakt `QuartPos.toBlock(axis) == axis<<2`.
`from_block` verwendet arithmetic `axis>>2` und liefert den zugehörigen
Quartorigin, auch bei -1/-4/-5; es behauptet nicht, am eingegebenen unaligned
Block zu samplen. `sample_climate_at_block` sampelt dagegen genau diesen Block.
Java-Quart-Shiftwrap außerhalb dieser explizit begrenzten Consumerdomain wird
typed refused, kein modulo umgebogener Weltpunkt.

Kein eigenständiges `DensityTemplate::bind_seed(seed)` ohne Settingskontext:
benannte Densitys hängen ebenfalls vom selected legacy/modern Wiring ab.
Alle 35 sind bereits im Routertemplate geprüft und werden deshalb über dessen
konkretes `sample_definition` gebunden. Ein späterer standalone Consumer müsste
denselben Settingskontext ausdrücklich erhalten.

## Seedbindung und Factory-/Bound-Views

1. Das Settingsflag wählt original `Legacy` oder `Xoroshiro128PlusPlus`; alle
   signed64 Seedbits bleiben erhalten. Der settings RNG wird einmal mit
   `fork_positional` in den RandomState-Fabrikseed überführt. Canonical
   `minecraft:aquifer` und `minecraft:ore` erzeugen jeweils name-derived RNG
   und anschließend eine eigene Positionalfactory. Diese immutable Fabriken
   bleiben private für die unmittelbar folgenden Materialconsumer; sie werden
   gegen echte JARdraws an repräsentativen Positionen geprüft.
2. NormalNoisebindungen werden nach **canonical `NoiseResourceId`-Name**
   konstruiert, nie nach Nodeindex, Definitionsindex, Shortname oder Requestorder.
   Der originale name-derived RNG versorgt beide originalen Normalstack-Forks.
   Ein immutable Sampler je gleicher tatsächlich gewählter Noiseresource und
   Wiringart; Noiseaufrufe memoizieren keine Densitywerte.
3. Bei legacy gelten die echten drei Specialcases: Temperature aus unabhängiger
   `Legacy(level_seed.wrapping_add(0))`, Vegetation aus
   `Legacy(level_seed.wrapping_add(1))`, jeweils originale `(-7,[1,1])`
   Legacy-Nether-Biome-Normalnoise. `Noises.SHIFT` ist **`minecraft:offset`**;
   sein name-derived RNG konstruiert die originale moderne `Normal(0,[0])`.
   Dies ist ein nachgewiesener spezieller Bindingzweig, kein Errorfallback.
4. BlendedNoise wird mit seinen vorhandenen Codecparametern erneut gebaut:
   legacy aus unabhängigem `Legacy(level_seed)`, modern aus canonical
   `minecraft:terrain`. EndIsland bekommt unabhängig `Legacy(level_seed)`,
   exakt 17292 `consume_count`-Schritte, dann Simplex. Bereits gebundene
   Router/Clones bleiben bei jedem Constructionerror unverändert.
5. Ein echter NoiseWiring-View rekonstruiert die originalen `mapAll`-Factories
   bottom-up mit den echten Samplerbounds. Er behält HolderHolder und Marker;
   seine Child-/Factoryklassen folgen dem JAR, nicht beliebigen algebraischen
   Optimierungen. Ein gesonderter Climate-Visitor läuft danach über die sechs
   Seedrouter-Climatefelder, entfernt dort Holder/Marker wie `RandomState` und
   **rekonstruiert die Ancestors erneut**.

Konkretes Beispiel: Codec-`mul` mit zwei inline Zahlinputs ist zunächst Ap2,
weil beide echte HolderHolder sind. Der erste Wiring-Visitor behält diese
Holders und damit Ap2. Der Climate-Visitor kann sie entfernen; eine echte
Constantchild kann dadurch `MulOrAdd` erzeugen. Es darf weder vorab in .1
gefaltet noch im Seedrouter pauschal zu MulOrAdd gemacht werden. Eine bereits
echte MulOrAdd-Factory behält dagegen beim eigenen `mapAll` ihre spezifische
Factory und berechnet Bounds nach genau diesem Originalpfad neu.

Private Lowerednodes besitzen source-origin, visitor-view und tatsächliche
Factoryherkunft; Referenz-/Directholder und raw Splinecoordinateholder bleiben
unterschiedene Ursprünge. Die normale Auswertung hat kein Evaluate-all und
keinen globalen Valuecache. .2 fügt noch keine Behauptung hinzu, dass beliebige
IRoccurrences bereits die vollständige Original-HashMap-/Objectgleichheit für
.3-Wrappersharing ersetzen; dessen tatsächliche Gleichheitsabnahme bleibt .3.
Unveränderte Origins dürfen immutable Daten teilen, nicht zukünftige mutable
Wrappercaches erfinden.

Bounds benutzen die originalen Factoryformeln und Reihenfolge, einschließlich
der originalen Ap2-Multiplikationsdiagnose und `Mapped.create`-Diagnostics.
Es gibt keine "verbesserten" tighter Bounds, Reassociation, FMA oder pauschale
Constantfolds. Java `Math.min/max` für Signedzero/NaN wird explizit umgesetzt;
Rusts NaN-ignorierendes `f64::min/max` ist kein Ersatz. Clamp/RangeChoice behalten
reversed Endpunkte, Gradient behält equal/reversed Endpunkte. Ungebundene
Noiseholder werden niemals durch den originalen prebinding Nullsamplepfad als
erfolgreich gebundene Nodes ausgegeben.

Splinecoordinates erhalten ihre **neu gebundenen** Densitydiagnostics, exakt
f64→f32. Splinebounds werden mit dem originalen `Multipoint.create/mapAll`
rekonstruiert und als f32→f64 exportiert. Bestehende GEN-04.3-Factory-/Bitlogik
wird weiterverwendet. Wo der öffentliche `SplineCoordinateRange::bounded`
absichtlich engere finite/ascending Diagnosticadmission hat, darf das nicht
die originalen privaten Densitydiagnostics beschneiden: erforderlichenfalls
ein ausschließlich crate-interner checked-graph Builderpfad, weiterhin mit
den vollständigen Struktur-/Point-/Depthcaps; keine neue unchecked PublicAPI.
Die bestehende öffentliche Splineadmission bleibt unverändert und tested.

Preflight: Jeder der beiden immutable Views bleibt bei höchstens 4096 Vertices,
8192 Edges und combined Depth96; das Lowering ersetzt/entfernt vorhandene Nodes
statt zusätzlicher Runtimegraphformen mit unbounded Expansion. Zusammen höchstens
8192 Viewvertices/16384 Viewedeges; Noisepool höchstens die 60 gepinnten Ressourcen,
Blended-/Endsampler höchstens ihre bereits begrenzten Originaloccurrences.
Keine wiederholte Expansion benannter Definitions-DAGs in vollständige Bäume.
Bei unerwarteter Überschreitung typed BindError vor Veröffentlichung statt
stiller vereinfachter Factory. Originalsurface-Noises/Claybands werden in .2
nicht ausgeführt; deren kanonische Namedfabriken ändern Routerseeds nicht durch
Requestorder. Das Oracle darf den echten vollständigen Java-RandomState bauen.

## Scalarreihenfolge und gemeinsamer Workmeter

Die Auswertung folgt der originalen Anfragefolge. Ap2 `mul` liest links zuerst;
bei ±0 überspringt es rechts und liefert **+0**. MulOrAdd liest sein input auch
bei Faktor0 und rechnet original `value*constant`/`value+constant`. Ap2 MIN/MAX
benutzen originale right-child Bounds für ihre strikten Shortcircuitchecks.
RangeChoice fragt nur den gewählten Child an. ShiftedNoise berechnet Xshift,
Yshift, Zshift in Originalreihenfolge. Spline evaluiert zuerst die Koordinate,
dann die originale binäre Suche und genau den ausgewählten Endpoint bzw. low
vor high. Direktmarker delegieren nur in diesem Plainpointkontext; BlendAlpha1,
BlendOffset0, Beardifiermarker0 und EmptyBlender-Density folgen genau diesem
originalen Kontext, keiner vorgetäuschten NoiseChunkimplementation.

Ein kleiner privater Budgetcounter wird von **demselben Sample** durch
Density→Spline→Density und alle Noise-/Endcalls weitergereicht. Ein neues inneres
Splinebudget oder ein Callback, der ein frisches Densitybudget erzeugt, ist
unzulässig. Keine allgemeine Evaluationframework-Zwischenetappe.

| Charge vor Ausführung | Eine Unit |
| --- | --- |
| Eintritt in einen ausgeführten Densitynode | Einschließlich actual Holder, Marker, BlendDensity, Constant, Ap2/MulOrAdd, EndIsland und Spline-Densitywrapper. Kein Charge für einen übersprungenen Child. |
| Eintritt in einen ausgeführten Splinenode | Constant oder Multipoint; unveränderter öffentlicher Splineunitvertrag. |
| Splinecoordinatecallback | Vor dem Densitycallback. Der raw Coordinateholder ist dieser Callback, kein zusätzlicher Density-HolderHolder. |
| Ein Binärsuchloop im Multipoint | Vor dem Locationvergleich; unveränderter öffentlicher Splineunitvertrag. |
| Invocation eines Normal-/Blended-/Simplexsamplers | Vor der jeweiligen Samplingarbeit, auch bei tatsächlich originaler Offset-Zeronormalnoise. |
| Eine feste Octaveloopiteration | Vor dem Slotcheck, inklusive Nullamplitudenslot, in jedem ausgeführten Perlinstack bzw. Blended-main/limitloop. Beide Normalstacks zählen; beim Blended-limitloop ein Charge je Index unabhängig von aktiven Min-/Maxstacks. Innere Improvedcalls erhalten keinen zusätzlichen Invocationcharge. |
| Eine EndIsland-Kandidateniteration | Genau outer X -12..12, inner Z -12..12, vor Radiuscheck; eventuelle Simplexinvocation zählt zusätzlich. |

Noiseconstructor/RNGdraws, bounded Admission, O(1)-Positionschecks, f32-Casts
und sechs Climatequantisierungen sind Construction bzw. fixed overhead und
keine Sampleunits. Noiseloops sind dennoch explizit charged; keine versteckte
unbounded Schleife. Originales EndIsland enthält genau 625 Kandidaten ohne Cache.
Der bestehende öffentliche CubicSplinepfad zählt weiterhin ausschließlich seine
drei vorherigen Unitarten; zusätzliche Density-/Noisecharges gibt es erst durch
seinen actual privaten Densitycallback, nicht für normale fremde Callbacks.

Pointbudget **1..=1_048_576**; Climate teilt dieses Budget über alle sechs Achsen,
anstatt es sechsmal neu zu starten. Batch nimmt **1..=4096 Requests** und ein
einziges Budget **1..=67_108_864**, das über Requests in Eingabereihenfolge
weiterläuft. Alle Shape-/Positions-/Lengthchecks erfolgen vor Samplearbeit und
Batchscratchallocation. Keine späteren Requests nach erstem Fehler. Die Grenze
ist eine checked Arbeitsschranke, kein Benchmark oder TPSversprechen.

Abbruch bei Charge auf exhausted Meter: Die angekündigte Operation läuft nicht
mehr; Errors enthalten Origin/Field bzw. named Density, Position, Batchindex,
Kind und limit/used. Point/Climate liefern keinen Teilwert, Batch keinen
teilweise gefüllten Outputsliceverweis. Die immutable Seedbindung bleibt nach
Errors voll verwendbar. Privat erfasste erfolgreich/abgebrochenen Requests
prüfen genau budget N versus N-1 auch über mehrere Density↔Splinewechsel und
Batchgrenzen. Testtraces sind bounded und keine öffentliche Hotpathallocation.

Finite Consumerpolicy: Input-/scaled Noiseparameter und alle ausgeführten
Density-/Spline-Resultate müssen finite sein. **Erst ein fertig berechneter
tatsächlich angefragter Node** wird auf Nonfinite geprüft; Diagnostics werden
nicht als Sampleclamp genutzt und unangefragte Branches nicht ausgewertet.
NaN/Infinity, falls das echte Originalsample sie erzeugt, erhalten einen
typed NonFiniteResult einschließlich raw bits, keine Ersatz0/-100-Density.
Das ist eine ausdrückliche FerrumC-Consumerabweichung, keine Behauptung, das
Original-JAR sei dort finite. Spline-f64→f32-overflow bekommt die bestehende
NonFiniteCoordinatepolicy. Budget-/Finiteerrors stoppen Folgeanfragen sichtbar.

## Legitime große Noisezwischenwerte

Ein Originaljagged-Sample bei x=30_000_000 und xzScale1500 hat bereits
45_000_000_000 als Normalnoiseinput. Die öffentliche GEN-04-Normal-/Perlincap
darauf anzuwenden wäre ein tatsächlicher Consumerbug.

.2 ergänzt ausschließlich private/crate-interne actual Consumerpfade von
Normal/Perlinnoise: Rootposition ist bereits checked, Resource/Scales stammen
aus geschlossenen geprüften Builtins. Positionsprodukte, die zusätzliche
Normal-second Skalierung und per-octave Produkte werden vor Sampling auf
finite geprüft; das originale Wrap passiert **je Octave** in derselben
Arithmeticorder wie Java. Auf wrapped Improvedinputs und vertikale Derivedwerte
gelten die begründeten internen Sicherheitsprüfungen, kein ungeprüfter
Callerzugang. Kein einmaliges Vorwrappen von Normalinput; das wäre numerisch
eine andere Rechnung. Die vorhandenen öffentlichen CallerCaps bleiben exakt.

Die originalen Mth-lfloor-/Wrapconversionannahmen werden für den tatsächlich
erreichbaren Builtindomainbereich geprüft; bei den legitimen Worldborder-/Y-
Extremproben darf keine durch die alten Publiccaps verursachte Refusal bestehen.
Blendednoise besitzt schon einen actual Blockconsumer mit internen Wrapped-
Loops; dieser wird in den shared Workmeter aufgenommen, ohne seinen numerischen
Algorithmus oder seine public Blockcoordinateadmission zu verändern. EndSimplex
erhält nur seine konkreten Integerderived Inputs; kein beliebiges großes
PublicSimplexsample wird hier freigegeben.

## Climate und End-Island-Policy

Die sechs Climate-Viewfields werden in Originalreihenfolge sampled. Jeder Wert
wird direkt von f64 nach f32 gecastet; anschließend **f32**-Multiplikation mit
10000.0f32, danach Java-long-Truncation/Saturation. Kein f64-Zwischenschritt,
Round-to-nearest, Vorclamp auf [-1,1] oder vorzeitiges Ganzzahlprodukt. Die
private Quantisierungsfunktion hat echte Java-Bit-/Longproben auch für ±0,
adjacent f32-Rundungsfälle, NaN und ±Infinity; die öffentliche Sampleschnittstelle
bleibt unter ihrer gerade genannten finite Consumerpolicy. Der echte JAR-
Climate-Sampler wird bei Quartproben direkt aufgerufen; exact-block Climate-
Proben verwenden seine tatsächlichen sechs bereits transformierten Felder und
`Climate.target`, nicht den Seedrouter als vermeintlich gleichen Factoryview.

Vanilla EndIsland berechnet den ersten `x*x+z*z`-Term als Java-wrapping i32;
`x/z` sind vorher truncating block/8, im Heighthelper folgt truncating /2 und
signed `%2`, kein floorDivide. Dies ist .2s bewiesener Algorithmus. Sein
anfänglicher radialer sqrt kann nach Overflow NaN erzeugen; der ganze originale
625-Kandidatenloop und Java-Math.max-NaNverhalten bleiben erhalten, bevor der
fertige Densitynode gemäß FerrumCpolicy typed refused wird. Oracle zeichnet
die **Originalraw-NaNbits** und davon getrennt die erwartete Rust-Refusal auf.
Nicht jedes entfernte Endsample ist NaN; auch endlich bleibende, gegenüber
Paper falsche Vanilla-Wrapresultate brauchen konkrete distant Probefälle.

Paper widenet den anfänglichen Distanzterm nach i64 und verwendet den
8192-Slot-Threadlocalcache. Vanilla-JARfixtures belegen diesen Pfad nicht.
Kein Terrain mit bekannt falschen Enddistanzresultaten wird als Paper-Drop-in
aktiviert. Nach .2s konkretem bound/eval-Vertrag kann Root einen actual gepinnten
Paper-JARhelper aus den inzwischen tatsächlich vorbereiteten Artefakten bauen;
dessen Samples/Bits müssen den widened Zweig
unabhängig prüfen. Rust benötigt hierfür später keinen globalen mutable Cache;
owned Sessioncache oder kein Cache kann dieselbe widened Numericalsemantik
tragen. Kein ad-hoc Semantikschalter oder hypothetisches Papergolden in .2.

Root hat inzwischen `.codex-tmp/paper-runtime-1.21.8-60/verified-classpath.txt`
und `verified-artifacts.json` bereitgestellt: der tatsächlich Paperclip-gepatchte
inner `versions/1.21.8/paper-1.21.8.jar` hat SHA256
`c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`;
119 Libraries wurden von Root gegen die Bundle-META-INF-Liste gehasht. Dieser
Stand entstand durch patch-only exit0, keinen Serverboot. Read-only `javap`
auf genau diesem Classpath bestätigt die tatsächlich öffentlichen Klassen
`DensityFunctions$EndIslandDensityFunction(long)` und
`DensityFunction$SinglePointContext(int,int,int)`, sowie tatsächliche
`compute(FunctionContext)`, `minValue/maxValue` und das private Threadlocal-
NoiseCachefeld. Ein separater Helper kann somit den **actual Paperconstructor
und compute** direkt aufrufen; keine nachgebaute widened Formel ist nötig.
Diese Klasseninspektion ist noch kein numerical Paperhelpergolden und kein
Gameplaygate. Sein Manifest muss den eigenen Paperbundle-/inner-/119-Library-
Satz separat vom Vanilla-/43-Librarysatz pinnen.

## Actual-JAR-Goldens und Traceabnahme

Neue getrennte Artefakte, die .1-Fixtures/Generatoren unverändert lassen:

- `tests/reference/density_router_oracle.java`
- `tests/reference/generate_density_router.py`
- `tests/reference/density_router.md`
- `tests/fixtures/density_router_1_21_8.jsonl`
- `tests/fixtures/density_router_1_21_8.manifest.json`
- Integration-/private Tests für Seedbinding, Factory-/Boundprojection,
  Scalar/Climate, Budget-/Abbruchtraces und große wrapped Inputs.

Generatorpins sind identisch zu .1/GEN-04: Bundle
`2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7`, inner JAR
`2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`, mappings
`e373ba569839e6f3c7bd4880247f3e85577d92cde2c605adfa2024eb9cae503e`, Original-
Worldgenfixture `bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30`,
alle 43 tatsächlichen LibrarySHA256s. Explizite `require`-Guards, kein Python
`assert` als Pinguard. Frische Helperclass zuletzt im classpath; frische Output-
directory je Run, normal und Python `-O`, fixture **und manifest** byte-identisch.
Bundle/inner/mappings/library/helper-shadow-Negativeguards erneut konkret testen.

Der Helper baut die tatsächlichen VanillaRegistries/RegistryOps und tatsächliche
`RandomState.create` aus original registrierten NoiseGeneratorSettings. Er
ruft reale Routerfields `compute` und `minValue/maxValue`, den echten
Climate-Sampler sowie echte Factory-/Splineprodukte auf. **Keine Javakopie der
Rustformeln oder der originalen Wiringlogik.** Für alle 35 Nameddefinitions
kann die tatsächliche originale innere NoiseWiringHelperclass per gepinntem
Mapping konstruiert werden: `eko$a` mit `(eko,long,boolean)`; ihre tatsächliche
`mapAll`-Visitorimplementation bindet die bereits wirklich decodierten Named-
Densitys. Der actual Climatevisitor `eko$1(eko)` liefert gesonderte Viewproben.
Das ist Reflection auf Originalcode, kein nachgebauter Visitor.

Testmatrix und Pflichtcensus:

| Gruppe | Tatsächlicher Nachweis |
| --- | --- |
| Settings/Namedclosure | Alle 7 Router ×15 Fields; alle 35 named Densitys unter allen 7 expliziten Settingswirings, inklusive ansonsten unused modern/legacy branches. Bound-/Factoryprojected Bits vor Sampleproben. |
| Seeds | 0, 1, -1, i64::MIN, i64::MAX, 0x0123456789abcdef und -0x1234567890abcde; legacy Vegetation an i64::MAX beweist wrapping+1. Mindestens zwei ungleichsymmetrische Seeds. |
| Pointdomain | Negative x/y/z; 0/±1, -17/-16/-15/15/16/17 Chunkseams; jeweilige minY-1/minY/minY+1/maxY-1/maxY/maxY+1; i32-Yextrema; x/z ±30M, ±(30M-1), beide Vorzeichen und mixed-sign. |
| Wrap/Jagged | Actual built-in jaggedness bei ±30M für Overworld/large/amplified. Original direkte Normal/Perlinproben mit skalierten 45b Inputs; ±Wrap-halfperiod und unmittelbar benachbarte f64bits, mehrere Perioden, second-stack scaled boundary, wirkliche per-octave Wrapcases. |
| Seeds/RNGdiagnostics | Canonical Noise vs falscher Shortname/slot eindeutig unterscheidende Proben; alle bound Noiseparameters/actual maxValuebits; legacy Temperature/Vegetation, offset-zero, modern vs legacy Blended, End17292; actual Aquifer-/Orefactory positional draws. Reversed Bindingrequestorder und unabhängige Clones ändern nichts. |
| Factories/Bounds | Actual Ap2 vs MulOrAdd vor/nach beiden Visitors; original Mapped-/Clamp-/Range-/Gradient-/Splinebounds und Boundshortcircuit traces. Signedzero/NaN/Infinity-Diagnostics werden nicht umgedeutet. |
| Branches | Actual Ap2/MulOrAdd mit instrumentierten child Densityfunctions: ±0-skip vs mul-by0-evaluate, right MIN/MAX strikt bound-adjacent, selected-only RangeChoice am unteren/oberen Rand, Spline endpoint/equality/rightinterval/low-before-high; actual field/density branch-adjacent Contexts mit Census. |
| Climate | Alle 6 Achsen, rohe f32bits und i64quantized Werte; typed Quart→Blockalignment einschließlich -1/-4/-5; actual direct-block View und actual SamplerQuartprobe; original quantizeCoord mit adjacent float-Rundungs-/Castgrenzen. |
| End | Inselzentrum/negative remainder/Radius4096-adjacent, i32-Radialoverflow an und neben der block/8-Distanzgrenze, diagonale Overflowcases, legitime ±30M. Originalfinite Outputs vs OriginalNaNbits/FerrumCtyped Refusal getrennt. Paperwidendrift ausdrücklich als unbelegter separater Pfad. |
| Work/Errors | Rust actual shared-meter traces für N/N-1, invalid 0/max+1, Density→Spline→Density mit mehrfachen Übergängen, Noiseoctaves, Endloop, batch request boundary, Branchthrow/nonfinite; nichts nach erstem Fehler, keine Teiloutputs, Seedrouter weiter benutzbar. |

Instrumentierte Javachilds delegieren Numericalbehavior an echte originale
Factories und zeichnen nur Requestfolge/gezielte Throwpunkte auf. Für Rusts
eigene Budgetabbruchpolicy werden erfolgreiche Java-Requestspuren und die
sichtbaren Rustcharge-/Abbruchspuren getrennt verglichen; Java besitzt diesen
Workbudgetvertrag nicht. Es gibt keinen vorgetäuschten Java-Budgetbeweis.
Bei NaNs klassifizieren Tests OriginalNaNbits und Consumerrefusal, statt einen
arithmetischen Plattform-NaNpayload außerhalb des belegten Toolchainpins als
allgemeine Bitgarantie auszugeben.

## Freigegebene Umsetzung

1. Separate actual-JAR-Seed/Factory/Bounds/Scalar/Climate-Goldens herstellen;
   Coveragecounts und Negativepinguards zuerst belegen. Kein Zusatzagentspawn
   ohne freien durch Root zugewiesenen Slot.
2. Private Seedbinding-/Factory-/Boundviews und checked API implementieren;
   originale Namednoise-/legacy-/Blended-/Endseeds und Diagnostics vergleichen.
3. Gemeinsamen actual Density/Spline/Noise-Workpfad implementieren; bestehende
   Publicprimitives und ihre Goldens vollständig erhalten; plain Scalar und
   Climate an tatsächlichen JARbits/Requesttraces abnehmen.
4. Failure-/Caps-/Bordercases und unveränderte Outputs/Clones prüfen; Source-
   Matrix ergänzen, volle Worlddebug/-release, strict all-targets Clippy, fmt,
   whitespacecheck und Freeze. Cargo nur offline/locked, max2 Jobs, vereinbartes
   Agenttarget; keine Lockmutation. Rootreview und Integration abwarten.

Danach unmittelbar GEN-05.3 actual NoiseChunkwrapper/Interpolation; danach
Aquifer/Material/Ore→Biome/Surface→konkrete Blockchunks. .2 allein beansprucht
weder vollständige Survivalfähigkeit noch Paperplugin-Kompatibilität oder einen
gemessenen RAM-/TPSvorteil.
