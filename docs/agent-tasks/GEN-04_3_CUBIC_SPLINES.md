# GEN-04.3 — Originale kubische Splines für Terrain-Density

Status: **durch `item_transfer_primitives` implementiert, unabhängig CLEAN
geprüft und zur Integration freigegeben**. Design durch Root
vor Produktionsänderungen freigegeben. WG-004-Fremdreview ist CLEAN, GEN-04.2 wurde
als `09fd6657` integriert. Produktions-Crate ausschließlich ferrumc-world;
eigener Buildtarget `.codex-tmp/gen04_3-target`, kein zusätzlicher Agentslot.

## Ziel und Besitz

Originale Minecraft-1.21.8-Splines bilden die nächste Grundlage für die
Overworld-Densityrouter. Implementiere einen begrenzten, besessenen Vertrag
für konstante und verschachtelte Multipoint-Splines mit originaler f32-
Interpolation, Endpunkten, Ableitungen und Extrapolation. Keine neue
natürliche Welt allein durch dieses Primitive behaupten.

Besitz bei Vergabe: server/crates/ferrumc-world/** und dieses Taskdokument.
Keine Identity-/Generator-/Registry-/Sim-/App-/Storage-/Lock-/Gitänderungen.
Vor Code das konkrete Node-/Coordinate-/Builder-/Sampling-/Fehler-/Capdesign
an Root melden. Fehlende Density-/Registryconsumer als separate Abhängigkeit
benennen, keine handkopierte Terrainkonfiguration als Ersatz.

## Begrenzter Vertrag

- Immutable owned Splines, explizite typisierte Koordinatenbindung und
  verschachtelte Werte. Keine globale mutable Map, I/O oder Pluginereignisse.
  Daten werden vor Speicherung/Allocation so weit möglich geprüft.
- Originale Auswertungsreihenfolge, Float32-Rundung, Binärsuche und
  Endpunkt-/Derivativlogik erhalten. Keine algebraischen Umordnungen oder
  beabsichtigte FMA. Koordinaten-Callback und tatsächlicher Bedarf an
  verschachtelten Werten müssen einen klaren Reihenfolge-/Fehlervertrag haben.
- Explizite maximale Knotenzahl, Punkte pro Knoten, Tiefe und Arbeitsbudget;
  beliebig tiefe JSON-/Graphdaten oder zyklische Referenzen nicht zulassen.
  Caps sind zusätzliche FerrumC-Ressourcengrenzen, keine Vanilla-Defaults.
- Reine erfolgreiche Auswertung ohne Heapallocation. Wiederverwendung und
  Clone sind unabhängige Werte oder immutable Sharing mit dokumentiertem
  Ownershipvertrag. Unveränderter Zustand nach Construction-/Samplingfehler.
- Originale min/max-Diagnostik, soweit angeboten, vollständig lesen und
  portieren; Diagnose nicht als neue absolute Schranke für beliebige
  Koordinatenverträge ausgeben. Nicht implementierte Visitor-/Codec-/Router-
  Consumer ausdrücklich offen lassen.
- Kein unbounded Builder und keine generischen unchecked public Nodefelder.
  Malformed IDs, unbekannte Koordinaten, unsortierte/doppelte Stützstellen,
  nichtendliche Werte und Größen-/Tiefenüberschreitungen klassifizieren.

## Quellen und Abnahme

Minecraft Java1.21.8 / Protokoll772; Paper Build60 / Commit
29c8822d90899c89d2689338e81a98f690bcba12. Vollständige relevante
net.minecraft.util.CubicSpline samt Builder/Multipoint/Constant/Visitor und
ToFloatFunction sowie Mth-Binärsuche-/Lerp-Callees lesen. TerrainProvider-
und DensityFunctions-Splineconsumer mit relevanten Paperpatches lesen,
damit f32/f64-Konvertierungen und Koordinatenbounds korrekt abgegrenzt sind.

Unabhängiges Oracle ruft tatsächliche gepinnte Original-JAR-Methoden auf;
keine nachgeschriebene Interpolationsformel als alleiniger Vergleich.
Original-JAR-/Mappings-/Libraryhashes prüfen, frischen Probe-Bytecode
verwenden und reproduzierbaren Helper im eigenen Scope einchecken.

- Bitgenaue f32-Goldens für konstante, einfache und verschachtelte Splines,
  alle Intervalle, Endpunkte und angrenzende Floats, negative Werte,
  Signedzero und unterschiedliche Derivatives.
- Fehler-/Cap-/Depth-/Cycle-/Koordinatenfälle erhalten den Zustand;
  Construction kann keinen teilweise gültigen public Graph freigeben.
- Deterministische Samplingreihenfolge, gleicher Wert nach Clone und
  unabhängiger Node-/Koordinatenanfragefolge; Budget-/Grenzfehler prüfen.
- Bestehende Worldtests und beide Original-Noisegoldens erhalten;
  Debug-/Releasecontracts, strict all-targets Clippy und fmt.
- Vollständige Quellen-/Ownership-/Abweichungs-/Validierungsmatrix, exaktes
  Dateifreeze und Logs an Root; Root organisiert Fremdreview und Integration.

TerrainProviderprofile, Noise-/Densityregistry-Auflösung, NoiseRouter,
NoiseChunkinterpolation, Aquifers, Surface/Biome und tatsächliche natürliche
Chunkgenerierung bleiben nach diesem Untervertrag eigene Consumer.

## Implementiertes und freigegebenes Design

`spline` ist ein eigener öffentlicher World-Untervertrag. `SplineNodeId` und
`SplineCoordinateId` haben private `u32`-Felder und deklarieren Indizes in den
übergebenen Node-/Koordinatenslices; `new/index` sind keine globalen oder
persistierten Handles. Die Admission lehnt unbekannte darstellbare Indizes ab.
Gleiche Zahlen aus verschiedenen Konstruktionen enthalten ausdrücklich keine
Provenienz. Der Koordinaten-Callback bekommt genau diesen lokalen Index, nicht
eine globale Registry-ID oder eine Position.

Private-field `SplineCoordinateRange`, `SplinePoint` und die borrowed
`SplineNodeSpec` erlauben nur geprüfte Eingaben. Konstanten, Locations und
Derivatives sind finite. Diagnosticranges sind entweder finite geordnete
Endpunkte oder explizit `unlimited()`; einseitig unendliche Bereiche werden
zusätzlich von FerrumC abgelehnt. Punkte sind strikt steigend, inklusive
Reject gleicher positiver/negativer Null. Die Specs behalten keine unchecked
Runtimefelder und allozieren selbst nicht.

`CubicSpline::new` prüft sämtliche Specs einschließlich nicht erreichbarer
Teilgraphen, bevor eine eigene Graphallocation erfolgt. Root-, Child- und
Coordinateindizes sowie die Gesamtkappen werden zuerst geprüft, dann findet
eine iterative DFS mit fixed-capacity Arrays alle Zyklen. Erst danach werden
child-before-parent Tiefe und Originaldiagnostik berechnet. Nach erfolgreicher
Preflight werden die Nodes, Punkte und Ranges besessen; ein `Arc` teilt nur
immutable Graphdaten. Borrowed Inputs bleiben nicht erhalten. Sampling hat
keinen Cache oder mutable RNG; nur eventuelle Caller-Callbacks können allozieren.

| Grenze | Wert und Einheiten | Nachweis / Fehlervertrag |
| --- | --- | --- |
| Nodes | 1024 inklusive Konstanten und ungenutzter Specs | Exakte Kappe akzeptiert; 1025 als `TooManyNodes` abgelehnt. |
| Coordinates | 64 deklarierte lokale Ranges | 64 inklusive Callbackindex63 akzeptiert; 65 als `TooManyCoordinates` abgelehnt. |
| Punkte je Multipoint | 1..=64 | Leer / 65 klassifiziert; Duplicate, Signedzero und umgekehrte Reihenfolge abgelehnt. |
| Gesamtpunkte | 16,384 über alle Specs | Exakte Summe akzeptiert, nächster Multipoint vor Allocation abgelehnt. |
| Tiefe | 32, Constantleaf zählt1 | Tiefe32 akzeptiert,33 `DepthExceeded`; auch ungenutzte Nodes geprüft. 1024-langer Zyklus wird ohne Rekursion erkannt. |
| Samplingbudget | 1..=1,048,576 Arbeitseinheiten | Vor Nodeentry, Callback und jeder Binsearchcomparison wird je1 Einheit berechnet. 0 / Overmax / usize-Max vor Callback abgelehnt. Shared-DAG-Explosion stoppt deterministisch. |

Die Caps sind eigene FerrumC-Ressourcengrenzen und keine Vanilla-Defaults.
Die Workunits begrenzen ausschließlich eigene Auswertung, nicht die Laufzeit,
Nebenwirkungen oder Allokationen eines externen Callbacks. Budget12 für die
geprüfte repeated-child Probe genügt exakt;11 und1 scheitern an definierten
Operationen ohne nachfolgende Arbeit. Alle Bounds und Einheiten sind zusätzlich
im öffentlichen Rustdoc dokumentiert.

## Quellen-, Reihenfolge- und Abweichungsmatrix

Gelesen wurden vollständige `CubicSpline.java` samt Codec, Builder,
Valuetransformation, Constant, Multipoint, CoordinateVisitor, create/apply/find-
Interval/mapAll/Diagnostik und vollständige `ToFloatFunction.java` mit
createUnlimited/comap. `Mth.binarySearch` und sämtliche verwendeten f32/f64-
Lerpmethoden wurden mit ihrem vollen Methodenkontext gelesen. Der vollständige
`TerrainProvider` und die vollständigen DensityFunctions-Spline-, Coordinate-
und Pointconsumer wurden gelesen. Die vollständige zugehörige DensityFunctions-
Sourcepatch und die relevanten Patchsuchen ergeben keine Splineänderung:
die dortigen EndIsland-Cache/Overflowänderungen betreffen diesen Vertrag nicht.
Applied `CubicSpline` und `ToFloatFunction` sind bytegleich mit den Basequellen.

Referenzroot: `.codex-tmp/paper-1.21.8-reference/paper-server/`.
Applied Java liegt unter `src/minecraft/java/`, Base unter
`.gradle/caches/paperweight/mache/base/sources/`.

| Originalquelle / beobachteter Vertrag | Implementierung, Ownership und Grenze |
| --- | --- |
| `util/CubicSpline.Constant`: f32-Wert, identische min/max, keine Coordinateanfrage | Finite Konstanten; bitgenauer Signedzero/Subnormal/Extrema-Vertrag. Kein unbound testing constant. |
| `util/CubicSpline.Multipoint.findIntervalStart` → `Mth.binarySearch`: erste Location strikt größer als x, danach Index−1 | Gleiche Binsearch und f32-Vergleiche. Exakte innere Location liegt im Intervall rechts und wertet beide Children aus. Exakte letzte Location verwendet nur den letzten Child. |
| Multipoint `apply` / `linearExtend`: außerhalb nur Endpointchild, Derivative±0 liefert Child ohne Arithmetik | Originaler f32-Ablauf, Negativezero bleibt erhalten. Kein Clamping auf deklarierte Coordinatebounds. |
| Multipoint `apply` innen: Coordinate einmal, linker Child vor rechtem Child, auch bei t=0; shared Children werden erneut angefordert | Kein Memoizing oder veränderte Callbackzahl. Original t, Differenzen, Derivativfaktoren, zwei Lerps und t·(1−t) in unveränderter Reihenfolge. Keine deliberate FMA/Reassociation. |
| Multipoint `create` berechnet Minmax mit Extrapolation und Quarter-Slopemargin; Java `Math.min/max` propagieren NaN und unterscheiden die Nullen | Gleiche Diagnoseoperationen und eigene Java-min/max-Semantik. NaN, Unendlichkeit und Signedzero bleiben bitgenau erhalten. Keine Behauptung absoluter Bounds für beliebige Coordinatecallbacks. |
| `util/ToFloatFunction` kann Minmax deklarieren, unlimited meldet ±inf; Originalcallbacks können Exceptions werfen | Lokaler typed callback `Result<f32,E>`; originale Sourceexceptiontraces separat geprüft. Graph immutable, frühere Callbackeffekte können nicht rückgängig gemacht werden. |
| Java darf nonfinite Coordinates/Nodewerte weiterrechnen | FerrumC lehnt nonfinite Eingaben und reale Coordinatewerte ab; jedes nonfinite Child-/Rootresult stoppt sofort mit Node/Rawbits. Explizite zusätzliche Policy; keine pauschale nonfinite Originaltraceparität. |
| Childoverflow-Probe: original linker Child +inf, danach rechter Callback und RootNaN | Originaltrace `[0,1,2]`; Rust `NonFiniteResult(node1,+inf)` mit `[0,1]`. Folgender rechter Callback entfällt; gleicher Graph liefert danach finite1.0 unverändert. |
| `DensityFunctions.Spline.Coordinate` castet original compute-f64 und Diagnostics nach f32, Spline widenet f32-Sample nach f64 | Hier explizite f32-Calleroberfläche. Künftiger Densityadapter muss beide originalen Casts übernehmen; noch kein Densityconsumer implementiert. |
| `data/worldgen/TerrainProvider`: Builderprofile und Floattransformer für original Overworldoffset/factor/jaggedness | Vollständig als Consumerreferenz gelesen; keine Werte handkopiert. Profiles müssen mit registrierter Density-/Splineauflösung implementiert und unabhängig geprüft werden. |
| CubicSpline Codec, Buildertransformation, Coordinatevisitor/mapAll | Außerhalb dieser Primitive-API. Keine JSONdecodierung, dynamische Registry oder vermeintlich vollständige Terraincodec-Kompatibilität. |

SHA256 der angewandten gelesenen Quellen:

| Datei relativ zu Applied-Java | SHA256 |
| --- | --- |
| `net/minecraft/util/CubicSpline.java` | `834d1b5f4696161ca974fcf2b4ef8caa3ebec5e6a0b5bfcccb561ba4a6a63f97` |
| `net/minecraft/util/ToFloatFunction.java` | `b80fa43345e91c3fb7e2e074d82fe40db0a3fdd6e21bd5f16b4d1e98ddc4a316` |
| `net/minecraft/util/Mth.java` | `b16d2c4c6b1979c2999480952e4e4d624a67755204d96de909006dd99ff51a5c` |
| `net/minecraft/data/worldgen/TerrainProvider.java` | `a7ce45f74ce7f14d424fdae508d9e43f41171844a18c39c9dc3a1ba5c178c076` |
| `net/minecraft/world/level/levelgen/DensityFunctions.java` | `62b4422976740e5ea5115b7675cfb404bec6887c15ca232e79bbeaf35480a749` |

## Unabhängiges Original-JAR-Oracle

`tests/reference/CubicSplineOracle.java` implementiert keine Interpolation oder
Rangeformel. Reflection ruft die realen originalen Constant-/Multipointfactorys
und Sample-/Minmaxmethoden auf (`bbb`, `bbb$e`, `bdh`). `ToFloatFunction`-Proxies
liefern nur deklarierte Ranges, f32-Koordinaten und protokollierte Requestindizes.
Originale Versionsinitialisierung und Registrybootstrap erfolgen vor Mth-/
Splinestaticinitialisierung; frischer Helperbytecode liegt nur im jeweiligen
Outputtemp. Original-JAR und sämtliche43 Originalbibliotheken stehen zuerst
im Classpath, Helperbytecode zuletzt; `javac -proc:none` lädt keine Prozessoren.

`generate_cubic_spline.py` prüft vor Ausführung Bundle, inneres JAR, Größe und
Hash der offiziellen Mappings und alle gebündelten Libraryhashes. Guards sind
normale `require`-Aufrufe, keine durch Python `-O` entfernbaren Assertions.
Der Manifest enthält alle43 Libraries, Toolchains, Helper-/Generatorhashes
und Fixturezähler. JDK21 ist erforderlich. Zwei frische Exporte mit normalem
Python und `-O` sind inklusive Manifest byteidentisch.

- BundleSHA256: `2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7`.
- InnerJARSHA256: `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`.
- Mappings: 8,186,232 Bytes; SHA256
  `e373ba569839e6f3c7bd4880247f3e85577d92cde2c605adfa2024eb9cae503e`,
  MojangobjektSHA1 `eb1e1eb47cb740012fc82eacc394859463684132`.
- JavahelperSHA256: `eec82ed37d16eb7512ad27e972f12ef50b3597b33590b994bb3ad8391c708482`.
- PythonhelperSHA256: `4bd6c09a0d92d4224fe5feae618cd81f30f78a8e148c91ec581c1e8fca703856`.
- FixtureSHA256: `3d5e9adfa7742bbb280633b70353cb0999a249178c5f258c9260d9ab865fcd35`.

Der synthetische Corpus enthält38 Graphen und8260 Records /364,310 Bytes:
40 Koordinatendeklarationen (`C`),196 Nodes (`N`),38 Rootdiagnosen (`D`),
7588 finite Samples (`S`),353 originale nonfinite Ergebnisse (`X`) und45
originale Sourceexceptionproben (`E`). S-/X-Zeilen enthalten Koordinatenbits,
Resultbits und Requesttraces; E enthält die gewählte Fehlerrequestnummer und
den realen Originalprefixtrace. Kein `TerrainProvider`-Profil ist kopiert.

Finite S-Samples und Originaldiagnostik werden ohne Toleranz bitgenau geprüft.
X-Fälle klassifizieren die extra Rustpolicy; die besondere Childoverflowprobe
prüft ausdrücklich unterschiedliche Java-/Rusttraces. E-Proben stammen von
finit erfolgreichen Contexts und werfen an der ersten, bis vierten und gegebenen-
falls letzten tatsächlichen Anfrage einer repräsentativen maximalen Requestspur.
Der Rust-typed Fehler behält dieselben Originalrequests bis zum Fehler.

## Prüfungen und konkretes Dateifreeze

Alle neuen zehn Public-Splinecontracts bestehen in Debug und Release:
Originalgoldens/Diagnostik/Reihenfolge, echte Sourceexceptiontraces, gesonderter
Childoverflow, malformed Floats/Ranges/Locations/IDs, exakte alle Größenkappen,
unused-/longcycles, Depth32/33, workbounds und repeated-DAG-Abbruch, owned
Inputs/Send+Sync/Clone/Requestorder, nonfinite Coordinate-/Rootresulterrors
sowie nicht clampende Diagnosticranges. Nach fehlgeschlagenem Sample oder
fehlgeschlagener Folgeconstruction bleiben vorhandene Graphen unverändert.

- Vollständige Worlddebugsuite:163 bestanden,0 fehlgeschlagen,0 ignored;
  120 Unit +7 Composite +10 Spline +5 Determinism +14 Generation +7 Randomnoise.
  Rustdoc hat0 Tests; kein Rustdoctest wird als eigener Nachweis gezählt.
- Release:24 Publiccontracts,7 Composite +10 Spline +7 Randomnoise, alle grün.
- Strict all-targets Clippy einschließlich Dependencies mit `-D warnings` grün;
  kein neues Blanketallow oder gelockerter Lint. Der Implementerlauf lag vor der
  finalen README. Roots isolierter finaler Lauf fand und bestätigte F1:
  `doc_markdown` am unformatierten Produktnamen wurde ausschließlich durch
  Backticks behoben; der wiederholte finale Lauf ist grün. RED/GREEN liegen
  getrennt in `.codex-tmp/gen04_3-root-clippy-{red,green}.log`.
  `cargo fmt -p ferrumc-world
  -- --check` und scoped Diffwhitespacecheck grün.
- Normal-/`-O`-Oracleexport und Manifest byteidentisch. Separate negative
  `-O`-Proben ändern eigene Tempkopien/ersetzen Symlinks, niemals Originals:
  Bundle-/Inner-/Mappings-/Librarymismatch jeweils exit1 vor Fixture-/Manifest-
  export und mit passender Pinfehlermeldung.
- 33 zuvor integrierte World-Rust-/Reference-/Fixturedateien bleiben bytegleich
  zu `09fd6657`; `lib.rs` erhält ausschließlich den neuen Splineexport.
  Insbesondere beide bestehenden Noiseoracle-Fixtures/Helper bleiben erhalten.
- Es gibt keine Locks, Scheduler, Events, I/O, Generatoraktivierung, Registry-,
  Sim-, Storage-, App-, Config- oder Lockfileänderung dieses Tickets.
  Speicher-/TPS-Vergleiche und Live-Paperterrain werden hier nicht behauptet.

Toolchain: Rust1.99.0 (`b940084d7`,2026-09-28), LLVM23.1.1,
x86_64-unknown-linux-gnu; Python3.14.7; Temurin OpenJDK21.0.12.1+1,
Linux7.2.4-3-cachyos/x86_64/glibc2.44. Diese goldenen Vergleiche belegen die
getesteten Fälle auf dieser Toolchain, keinen allgemeinen Plattformgleichstand.

Reproduktion relativ zum Repo (Cargoaufrufe in `server/`, keine Standardtarget-
verwendung; die Reproduktionsbefehle nutzen den eigenen Target und maximal2 Jobs):

```bash
python3 server/crates/ferrumc-world/tests/reference/generate_cubic_spline.py .codex-tmp/original-1.21.8 /home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1 .codex-tmp/gen04_3-oracle-a
python3 -O server/crates/ferrumc-world/tests/reference/generate_cubic_spline.py .codex-tmp/original-1.21.8 /home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1 .codex-tmp/gen04_3-oracle-b
# in server/:
/home/jstrauss/.cargo/bin/cargo test --offline --locked -j2 -p ferrumc-world --target-dir ../.codex-tmp/gen04_3-target
/home/jstrauss/.cargo/bin/cargo test --offline --locked -j2 -p ferrumc-world --release --test cubic_spline_contract --test composite_noise_contract --test random_noise_contract --target-dir ../.codex-tmp/gen04_3-target
/home/jstrauss/.cargo/bin/cargo clippy --offline --locked -j2 -p ferrumc-world --all-targets --target-dir ../.codex-tmp/gen04_3-target -- -D warnings
/home/jstrauss/.cargo/bin/cargo fmt -p ferrumc-world -- --check
```

Die finale Debugsuite wurde mit demselben Target ohne `--offline --locked -j2`
aufgerufen; obige Reproduktion fügt nur Network-/Lock-/Jobguards hinzu.
Die finalen Release-/Clippyaufrufe verwenden diese Guards ausdrücklich.

Artefakte relativ zum Repo:

- `.codex-tmp/gen04_3-freeze.sha256`: exakte eigene Quellen, Tests, Fixture,
  Manifest, README, Invarianten, `lib.rs` und dieses Taskdokument.
- `.codex-tmp/gen04_3-preserved.sha256`: 33 unveränderte frühere Worlddateien.
- `.codex-tmp/gen04_3-world-tests.log`, `gen04_3-release.log`,
  `gen04_3-clippy.log`, `gen04_3-fmt.log`, `gen04_3-diffcheck.log`.
- `.codex-tmp/gen04_3-oracle-a.log`, `gen04_3-oracle-b.log`,
  `gen04_3-oracle-equality.log`, `gen04_3-pin-negative.log`.
- `.codex-tmp/gen04_3-pin-negative.py`: reproduzierbare negative Tempcacheproben.
- `.codex-tmp/gen04_3-validation-artifacts.sha256`: Hashes der Logs und
  temporären Reviewartefakte, getrennt vom getrackten Dateifreeze.

## Konkrete noch fehlende Consumer

1. **Densitydaten und Registryauflösung:** versionierte echte Builtin-
   Density-/Noiseparameterdaten, Holderreferenzen, bounded Codec-/Graphadmission
   und typed Noise-/Splinecoordinatebinding; kein handkopiertes Zahlenprofil.
2. **DensityFunctions/Router:** ursprüngliche f64-Consumer samt f32-Splinecast,
   TerrainProviderprofile/Transformers, Clamp/Arithmetic/RangeChoice/Shift/Y-
   Gradient/Blended/EndIsland und Wrappersemantik. Registrybindung, Builder-
   transformer und Coordinatevisitor/mapAll fehlen bewusst in diesem Primitive.
3. **NoiseChunk:** originale Cellinterpolation und Sampling-/Cachewrapper,
   NoiseSettings, vertikale/regionale Geometrie und stabile Densitykontexte.
4. **Aquifer/Material/Surface:** original Flüssigkeitsdruck, Blockentscheidung,
   SurfaceRules, Bedrock und dimensionale Ausgaben; danach Biome-/Climate-
   Resolver, Carvers, Features, Ores und Structures mit originalen Randomseeds.
5. **Tatsächliche Generatoren und Abnahme:** neue executable Identität vor
   Aktivierung von Overworld/Nether/End, vollständiger kompatibler Storage- und
   Apppfad und echte Originalchunk-/Biome-/Seedgoldens. Erst diese Consumer
   können natürliche Chunks erzeugen. Vollständiges Survival und Java-/Paper-
   Plugins sowie vergleichbare RAM-/TPS-Messungen bleiben weitere Verträge.

Root hat das unabhängige Review mit **CLEAN nach F1-Dokumentationskorrektur**
abgeschlossen: isoliert163 Worldtests, frischer bytegleicher `-O`-Reexport,
32 zusätzliche Original-JAR-Graphen mit4294 neuen Records (3462 finite Samples,
160 echte Sourceexceptiontraces), zusätzliche Debug2 und Release20 sowie
finales strict Alltargetclippy/Format grün. Der vollständige
[unabhängige Prüfbericht](GEN-04_3_CUBIC_SPLINES_REVIEW.md) ist Root-owned.
Root übernimmt die Integration; die Abnahme gilt ausschließlich diesem
Splineprimitive und beansprucht keine Natural-/Survivaletappe.
