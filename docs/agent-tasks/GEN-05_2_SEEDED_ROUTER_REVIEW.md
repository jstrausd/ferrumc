# GEN-05.2 — Unabhängiger Seedrouterreview

Reviewer: `feature_catalog`, 10.10.2026. Ergebnis **CLEAN** für die
freigegebene geschlossene Seedbindung und Scalar-/Climateauswertung der
originalen Minecraft-1.21.8-Builtins. Kein bestätigtes Finding und kein
erforderlicher Produktionsfix. GEN-05.3 kann nach Roots Integration beginnen.
Diese Abnahme aktiviert keine natürliche Welt und schließt Survival,
Paper-Endsemantik, Plugin-Kompatibilität oder Performanceabnahme nicht ab.

## Exakter Stand und unabhängige Prüfung

Root stellte den isolierten Snapshot `.codex-tmp/gen05_2-review-checkout`
aus dem read-only Gitarchive von
`e4650a51ee9f7dd01f179eac1d8e4e94edef5fc7` plus genau den 29 eingefrorenen
Dateien bereit. Reviewer nahm keine Git-, Worktree-, Lockfile- oder
Produktionsmutation vor. Die einzige neue Main-Datei ist dieser Bericht.

Autorfreeze `.codex-tmp/gen05_2-world.freeze.json`, SHA256
`a3fe491459a9c3fe90f7235cbb085bc329ab784b2d57102a766b2e2fdf33979d`;
Inputcheck `.codex-tmp/gen05_2-world-input-check.json`, SHA256
`a438212ade97d29c2d48b87217b6e25defdd6e0f2fadbeed2ca8647fbcea8187`.
Der Freeze umfasst den GEN-05-Brief, Runtime, private Tests, Dokumentation
und die fünf finalen Oracledateien. Alle 29 Längen und SHA256s wurden
unabhängig in Main und Snapshot vor den Tests und nach Probe-Restore geprüft.

Zusätzlich wurden die 20 bisherigen Referenzdateien und sechs
Compiler-/Abhängigkeitsinputs unabhängig gegen `c4e3dc76` verglichen: alle
26 bytegleich. Dazu gehören `Cargo.lock`, Worldmanifest, Registry-Sourcefixture,
`density/compile.rs`, dessen Tests und `density/json.rs`. Der Graphdiff ergänzt
nur `Clone` für `Point`/`NodeKind`; der bestehende Parser und Compiler wurden
nicht neu implementiert. Die vorherigen GEN-04- und GEN-05.1-Corpora bleiben
unverändert. Nach Restore sind Main und Snapshot wieder **29/29** und alle
alten Inputs **26/26** bytegleich. Integritätsrecords:
`.codex-tmp/gen05_2-review-integrity-{before,after}.json`.

Vollständig geprüft wurden der freigegebene
[Vertrag](GEN-05_2_SEEDED_ROUTER_APPROVAL.md), die archivierte
Herkunftspräzisierung im [GEN-05-Brief](GEN-05_DENSITY_COMPILER_ROUTER.md),
`density.rs`, `bind.rs`, `bind/origin.rs`, `seeded.rs`, `evaluate.rs`,
`sample_work.rs`, alle neuen privaten Seedroutertests, die betroffenen
Noise-/Splinepfade und ihre Callees sowie Export-/README-/Invariantendiffs.
Der vollständige Java-/Python-Oracle und seine finale Beschreibung wurden
ebenfalls geprüft. Keine fremden Crates wurden als zusätzliche Aufgabe geprüft.

## Originalquellen und tatsächliche JAR-Evidenz

Referenz: Minecraft **1.21.8**, Protokoll **772**, DataVersion **4440**;
Paper Build **60**, Commit `29c8822d90899c89d2689338e81a98f690bcba12`.
Gelesen wurden die vollständigen relevanten Originalklassen, darunter
DensityFunction/DensityFunctions, RandomState, NoiseRouter/NoiseRouterData,
Noises, TerrainProvider, CubicSpline und die Noise-/Randomprimitives. Die
zugehörigen Climate-, Quart-, Geometry-, Empty-Blender-, Mth- und
consumeCount-Pfade wurden einschließlich ihrer numerischen Callees geprüft.

Der Reviewer hat 21 relevante Base-/Applied-Quelldateien unabhängig gehasht
und bytegleich bestätigt: unter anderem RandomState, NoiseRouterData,
TerrainProvider, Climate, CubicSpline und alle geprüften Noise-/Randomklassen.
Record: `.codex-tmp/gen05_2-review-original-source-check.json`.
Der vollständige DensityFunctions-Diff enthält ausschließlich Papers
EndIsland-Distanzverbreiterung und dessen Cache; der angewendete Endhelper
wurde gesondert vollständig gelesen. Eine Quellenprüfung wird nicht als
tatsächlicher Paper-Serverboot oder numerischer JAR-Test ausgegeben.

Root führte den finalen Original-JAR-Oracle mit frisch kompiliertem Helper
unabhängig aus, normal und mit Python `-O`. Reviewer bestätigte die
Fixture-/Manifest-Bytes aller vier finalen Exporte, also Child und Root
jeweils normal und optimiert, gegen den eingefrorenen Stand. Beide Rootlogs
enthalten exakt **27 einzelne Original-CodeSourceprüfungen** gegen den
gepinten inneren Original-JAR sowie die gesonderte Prüfung des frischen
Helpers. Die Loghashes stimmen mit Roots Validierungsrecord überein.
Die 51 nach Ausführung erneut gehashten Inputs sind Roots Reexportevidenz;
der Reviewer beansprucht keinen zusätzlichen eigenen JAR-Reexport.

Rootrecord `.codex-tmp/gen05_2-root-oracle-validation.json`, SHA256
`30257cf53e7c8fd6335e1fb19f2d21948af6a2875871bbf34263bbb824c4c473`;
eigene Artefakt-/Originprüfung `.codex-tmp/gen05_2-review-oracle-check.json`.
Die acht tatsächlichen negativen Digest-/Classpath-/CodeSourceguards des
Oracle-Childs sind in beiden Modi erfolgreich protokolliert. Die Guardlogik
benutzt ausdrückliche Refusals und bleibt unter Python `-O` wirksam.

Finalfixture: **47.371.851 Bytes**, SHA256
`2012acc2b8256a9adbf10b54a3534e8d9cc5d0a79d5e481d5bc3ff55cd73430e`.
Finalmanifest: **6.891 Bytes**, SHA256
`f3dfc7e1614e26ab809996dea2928638e76b6c03da80b124c58f63c8b96c80f2`.
Unabhängig bestätigter Census: **17.213 Records** mit 49 Binding-, sieben
Projection-, 1.813 Point-, 294 Quart-, 14.700 Wrapped-noise-, 42 Quantize-
und 308 Tracerecords. Die Pointrecords enthalten genau **90.650** originale
Scalarwerte. Hinzu kommen 245 native Namedrootprojections, 2.940
Noise-Maximumdiagnosen und 686 Factorypositionen mit 2.058 Longdraws.
Alle sieben Settings und sieben Seeds einschließlich beider i64-Extrema
sind vertreten.

Die ursprünglichen **2.163 Records** wurden unabhängig in Eingabereihenfolge
rekursiv mit dem alten Corpus verglichen. Sämtliche alten Werte,
Arraylängen und Strukturen bleiben erhalten. Ausschließlich neue
Aquifer-/Ore-/Noise-Maximummetadata, native Namedrootprojections und
Spline-Min-/Max-Diagnostics kamen hinzu. Originalcorpus-SHA256
`70cded38014c5f4b6947c09da9588a578013ecef11fd53493fc52e889db2aae6`;
Record `.codex-tmp/gen05_2-review-baseline-records.json`.

## Vertragsprüfung

| Grenze | Geprüftes Ergebnis |
| --- | --- |
| API und Ownership | Nur `RouterTemplate.bind_seed`, geschlossene typisierte Ressourcen, unveränderliche Arcbindung und Clones. Keine eigenständige DensityTemplate-Seedbindung, fremdes JSON, fabricated Budget-/Quartfelder oder öffentlicher Graphzugriff. Point-/Climatepfade besitzen keinen eigenen Heap-/I/O-/RNG-/Threadpfad. |
| Seedherkunft | Originales modern/legacy Settingswiring, kanonische Ressourcennamen, Legacy-Temperatur/Vegetation einschließlich wrapping seed+1, originaler Offset-Zeronoise, getrennte Terrain-/Aquifer-/Orefabriken und End-Legacy mit 17.292 konsumierten Schritten. Sampling mutiert keinen RNG. |
| Native und decoded Graphen | Sparse native Varianten unterscheiden tatsächliche registrierte Factorygraphen von Codecgraphen. Native DirectHolderartefakte entfallen; echte Referenzholder und raw Splinecoordinates bleiben korrekt. Keine vollständige Baumexpansion oder pauschales Holderentfernen. Sourceindex, Construction-/Holderorigin und Visitorpass bleiben privat erhalten. |
| Graphcaps | Vor Wachstum 4.096 Vertices/8.192 Edges, joint Shape-/Cycle-/Depthprüfung mit Maximum 96. Je Settings 162 native Varianten; Overworld/large/amplified 2.560/3.032/71, End 2.457/2.907/35, die übrigen 2.433/2.887/35. Beide Views verwenden begrenzte gemeinsame Slots; keine neue unbeschränkte PublicAPI. |
| Originale Visitorpässe | Seedvisitor behält tatsächliche Holder/Marker; Climatevisitor entfernt diese und rekonstruiert Ancestors. Ap2 gegen tatsächliches MulOrAdd und dessen eigener mapAll-Boundsformel werden unterschieden. Kein algebraisches Vorfalten oder Ersatzdiagnostik. |
| Bounds und Splines | Originale f64-/f32-Diagnostikbits einschließlich NaN, Infinity, reversed Endpoints und Signedzero. Diagnostics sind keine Clamps. Neu gebundene f64-Coordinates werden original nach f32 adaptiert; private Densityrekonstruktion erhält alle bestehenden Spline-Strukturcaps und lockert die öffentliche Splineadmission nicht. |
| Scalaranfragen | Links vor rechts; Ap2 ±0 überspringt rechts und liefert +0, MulOrAdd Faktor0 fragt sein Input an. MIN/MAX überspringen nur bei strikter originaler Boundsentscheidung. RangeChoice/ShiftedNoise/Spline folgen tatsächlicher ausgewählter Anfragefolge; Marker gelten nur im leeren Plainpointkontext. |
| Gemeinsamer Workmeter | Ein Counter über Density→Spline→Density, Callback/Binärsuche und Noise. Charge vor jedem ausgeführten Density-/Splineentry, Callback, Vergleich, Sampler, jeder Octaveslotiteration einschließlich Zeroamplituden und jedem der 625 Endkandidaten; eligible Simplex zusätzlich. Kein inneres Budgetreset oder Valuecache. |
| Domain und Zulassung | X/Z inklusive ±30.000.000, Y alle i32. Quartkonversion prüft i64×4 vor Cast; negative Blockkoordinaten verwenden arithmetic >>2. Point/Climate 1..=1.048.576, Batch 1..=4.096 Requests und 1..=67.108.864 gemeinsame Units. |
| Große Noisezwischenwerte | Legitime 45-Milliarden-Jaggedinputs gelangen durch private finitegeprüfte Normal-/Perlinpfade. Originales Wrap je Octave, second-stack scaling und Longkonversionsdomain werden erhalten. Die öffentlichen Primitivecaps bleiben unverändert. |
| Finitepolicy und Fehler | Erst fertig berechnete tatsächlich angefragte Nodes werden geprüft. Nichtfinite Kinder, f64→f32-Overflow und exhausted Work stoppen unmittelbar, auch wenn Java einen später wieder finiten Parent herstellen könnte. Typisierte Source-/Field-/Position-/Batch-/limit-/used-Diagnostik; keine Ersatzdensity. |
| Climate und Batch | Eigener Climateview, Originalachsenfolge, direkt f64→f32, f32×10.000 und Java-long-Cast. Exact-block und echte Quartorigins sind getrennt. Batch prüft sämtliche Shape-/Positionen vor Auswertung und Outputallokation; ein gemeinsamer Meter und keine sichtbaren Teiloutputs. |

## Eigene Laufzeit- und Compilerprüfungen

Rust/Cargo **1.99.0**, Edition **2021**, offline/locked, maximal zwei Jobs,
eigener Target `.codex-tmp/gen05_2-review-target`, ausschließlich im isolierten
Snapshot:

- Vollständige Worlddebugsuite: **188 bestanden**, 0 fehlgeschlagen oder
  ignoriert, also 145 Unit- und 43 Integrationstests. Darin alle alten
  unveränderten Noise-/Spline-/Compiler-Corpora und die neue JARmatrix.
- Striktes `clippy -p ferrumc-world --all-targets -- -D warnings`: bestanden.
- `fmt -p ferrumc-world --check`: bestanden.

Logs `.codex-tmp/gen05_2-review-{debug,clippy,fmt}.log`. Der Autor hat gegen
denselben Freeze zusätzlich die komplette Release-Suite mit **188** Tests,
Clippy/Fmt und Whitespacecheck bestanden; seine Release-Evidenz ist
`.codex-tmp/gen05_2-world-release.log` und wird ausdrücklich als Autorprüfung
geführt. Ein zweiter eigener vollständiger Release-Lauf war nicht nötig.

Drei unabhängige zusätzliche Runtimeprobes wurden nur temporär dem
isolierten privaten Testmodul angehängt. **Alle drei bestehen in Debug und
gezielt in Release**:

1. Deterministische Permutation sämtlicher Sourcevertices mit vollständigem
   Child-/Rootremap sowie umgekehrter Noisepool. Unter allen sieben Settings,
   drei Seeds und vier repräsentativen Positionen einschließlich Worldborder
   und Y-Extrema: **4.368** Scalar-/Climatepaare, **1.176** Diagnosticpaare,
   **4.200** exakte Workkostenpaare und **336** Aquifer-/Oredraws identisch.
2. **28** synchronisierte Clone-Threads nach Drop von Originalrouter und
   Template. **84** Wiederholungen nach tatsächlichen Admission-/Workfehlern,
   dabei **8.736** Scalar-/Climate-Samples und **1.260** geordnete Batchwerte
   unverändert. Synchronisierung durch Barrier, keine Sleeps.
3. Endliche Sourceformen mit wiederholtem Cube erzeugen Overflow erst im
   angefragten Zwischenkind. Clamp, Squeeze und RangeChoice stoppen an diesem
   Kind, obwohl Java den Parent wieder endlich machen könnte. N/N−1 prüfen
   genaue Entry-/Abbruchfolge und fehlende Folgeanfragen.

Probequelle `.codex-tmp/gen05_2-review-probes.rs`, SHA256
`21eadf2c762a94cf1ca34d68d3651eb134383d0a2ecf069c5296ff6961d343ef`;
Logs `.codex-tmp/gen05_2-review-probes-{debug,release}.log`. Danach wurde das
private Testfile aus den eingefrorenen Originalbytes wiederhergestellt.

Eine positive externe Rustcompiler-/Runtimeprobe bestätigt Send+Sync der
Templates und Bindung sowie das Sampling nach Template-Drop. Vier negative
Compilerprobes bestätigen fehlende standalone Densitybindung (**E0599**),
private Routerdaten (**E0616**), geschützten Budgetconstructor (**E0423**)
und geschützte Quartfelder (**E0451**). Diese Probes verlinken vorhandene
Snapshotartefakte und mutieren weder Cargo noch Dependencies.
Quellen/Logs `.codex-tmp/gen05_2-review-api/`, Runner
`gen05_2-review-api-probes.py`, Record `gen05_2-review-api.json`.

## Endsemantik und nächste Abnahmegrenze

.2 implementiert ausdrücklich die originale Vanilla-Endarithmetik mit
wrapping i32-Radius, truncating Division/Remainder und vollständigem
Kandidatenloop. Originale NaNs werden gemäß FerrumCs geprüfter Finitepolicy
verweigert. Die dokumentierte Differenz zu Paper ist deshalb kein Finding
gegen diesen freigegebenen .2-Vertrag.

Roots separater tatsächlicher Vanilla-/Paper-JARhelper enthält **658**
Samplepaare: 392 gleich, 266 verschieden, davon 210 originale Vanilla-NaNs
und 56 endliche Differenzen. Alle Paperantworten sind endlich. Der Reviewer
hat Ergebnisrecord und angewendeten Sourcepfad geprüft; er beansprucht keine
eigene numerische Paperabnahme. Beispiel Seed0, Block(-524289,0,0): Vanilla
0,5625 gegenüber Paper −0,440873384475708. Beleg
`.codex-tmp/gen05_2-root-paper-end/{results,differences}.json`.
Ein aktiver Paper-Endgenerator benötigt eine gesonderte Implementierung und
Abnahme dieses widened Pfads; erfolgreiche Vanilla-Scalarproben reichen nicht.

Der nächste Worldconsumer ist **GEN-05.3 NoiseChunkwrapper/Interpolation**,
einschließlich tatsächlicher Originalgleichheit/Sharingidentität und
Sessionkontext. Danach Aquifer/Material/Ore, Biome/Surface und konkrete
Blockchunks. Der aktive Servergenerator bleibt Flat-v1. Keine Live-Spiel-
oder vollständige Survival-/Java-Paper-Pluginabnahme und kein gemessener
RAM-/TPSvorteil folgen aus diesem Review.

Finales Evidenzmanifest `.codex-tmp/gen05_2-review-evidence.sha256` erfasst
38 eigene bzw. klar zugeordnete Autor-/Rootartefakte. SHA256
`0f6e66c65d96aab8e8e989f424b0f2db7a75db362e9cbec22c529596c45e0176`.
