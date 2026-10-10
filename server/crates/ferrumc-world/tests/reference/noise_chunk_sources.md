# GEN-05.3: unabhängige Quellen-/Calleematrix

Minecraft Java 1.21.8 / Protokoll 772 / DataVersion 4440; Paper Build 60, Commit `29c8822d90899c89d2689338e81a98f690bcba12`. Vollständige Pins stehen in `source-pins.json` und den beiden Runmanifesten. Die folgende Tabelle trennt rekonstruierte Quellen von tatsächlich ausgeführten JAR-Methoden.

| Quelle | Base SHA-256 | Applied SHA-256 |
| --- | --- | --- |
| `net/minecraft/core/Holder.java` | `a1492ca1ce858e5f38ecea76a5b0270293f0ad1ca7251d556410b33a48dd2cfe` | `039063ef0cf1bfab31e1b5827f68e97c4f69939184f5595effbce7f513523806` |
| `net/minecraft/core/QuartPos.java` | `2cc0674d3a548826735c737d63cdf7f89fc3afd3eb9e19a4dcf4bfc1f7ef27d7` | `2cc0674d3a548826735c737d63cdf7f89fc3afd3eb9e19a4dcf4bfc1f7ef27d7` |
| `net/minecraft/server/level/ColumnPos.java` | `178bbee74a280a74a6354110f3f254021d5e3baad7ddd82c575c5c44c3933686` | `178bbee74a280a74a6354110f3f254021d5e3baad7ddd82c575c5c44c3933686` |
| `net/minecraft/util/CubicSpline.java` | `834d1b5f4696161ca974fcf2b4ef8caa3ebec5e6a0b5bfcccb561ba4a6a63f97` | `834d1b5f4696161ca974fcf2b4ef8caa3ebec5e6a0b5bfcccb561ba4a6a63f97` |
| `net/minecraft/util/Mth.java` | `b16d2c4c6b1979c2999480952e4e4d624a67755204d96de909006dd99ff51a5c` | `b16d2c4c6b1979c2999480952e4e4d624a67755204d96de909006dd99ff51a5c` |
| `net/minecraft/world/level/ChunkPos.java` | `e2689106080af423f1f06640dff61b23d4a379c18bb3e6b56fb8423a7bb8869c` | `c3245aee1e796b9994806eda2890039b39ca9d763726a94ffb6d7e6e493a7866` |
| `net/minecraft/world/level/biome/Climate.java` | `1afa61089fbc72018cff7d09fc548892b9f636a7b30f38e14f316107d92e9d20` | `1afa61089fbc72018cff7d09fc548892b9f636a7b30f38e14f316107d92e9d20` |
| `net/minecraft/world/level/levelgen/BitRandomSource.java` | `726528fb348082c248218899112a2d6fba1a1897dad719aa41939eff643ff2c1` | `726528fb348082c248218899112a2d6fba1a1897dad719aa41939eff643ff2c1` |
| `net/minecraft/world/level/levelgen/DensityFunction.java` | `3a7c65bcefe09274d50a69e971858679273d2090a190202200cc2d6b8c7459f9` | `3a7c65bcefe09274d50a69e971858679273d2090a190202200cc2d6b8c7459f9` |
| `net/minecraft/world/level/levelgen/DensityFunctions.java` | `3f1002ec0d0d8047fec58fa256c1625c7291f8fa47b96a429ff8687f58e88693` | `62b4422976740e5ea5115b7675cfb404bec6887c15ca232e79bbeaf35480a749` |
| `net/minecraft/world/level/levelgen/LegacyRandomSource.java` | `f05787a1b23d609499ec99e403ff267fecbc500f71bcc15311fce14a3a6a6c02` | `f05787a1b23d609499ec99e403ff267fecbc500f71bcc15311fce14a3a6a6c02` |
| `net/minecraft/world/level/levelgen/NoiseBasedChunkGenerator.java` | `1aee1be9952d347975d2a464da06ee330dc0980aba1a979ceca9d18e6d89dc71` | `2b586e7fca9059b65d214f33f6b84fcc7ae3de90285ad7cd9c8a56edc319f2d8` |
| `net/minecraft/world/level/levelgen/NoiseChunk.java` | `e3c872cf3f6a15a063d255f1188296ea841ef012b01959d35ca7e0012d97c863` | `e3c872cf3f6a15a063d255f1188296ea841ef012b01959d35ca7e0012d97c863` |
| `net/minecraft/world/level/levelgen/NoiseGeneratorSettings.java` | `e132b2da8afc1648b98b9feee7a27a9ae7b30b44d473f6c9e49c704df98bb728` | `e132b2da8afc1648b98b9feee7a27a9ae7b30b44d473f6c9e49c704df98bb728` |
| `net/minecraft/world/level/levelgen/NoiseRouter.java` | `58ae86a1d35c0152b4bfb52a878711bef6ba56635b108a65230a3d3d2552c8b7` | `58ae86a1d35c0152b4bfb52a878711bef6ba56635b108a65230a3d3d2552c8b7` |
| `net/minecraft/world/level/levelgen/NoiseSettings.java` | `78db972efcb2f96c57cea2641d957954d8da791b51bfc176397b0666bfe865c7` | `78db972efcb2f96c57cea2641d957954d8da791b51bfc176397b0666bfe865c7` |
| `net/minecraft/world/level/levelgen/RandomState.java` | `95482aed87fb09b24ad4500cf03042aeebc808451c56b0299ae9ec6fd10d46d7` | `95482aed87fb09b24ad4500cf03042aeebc808451c56b0299ae9ec6fd10d46d7` |
| `net/minecraft/world/level/levelgen/Xoroshiro128PlusPlus.java` | `4ea97c1bb740c224ff0ce77fff27c7794454b72151ed4f8638dfc32aa7a84354` | `4ea97c1bb740c224ff0ce77fff27c7794454b72151ed4f8638dfc32aa7a84354` |
| `net/minecraft/world/level/levelgen/XoroshiroRandomSource.java` | `76a1af8439d57038d3e80ea41dc7b38ca3d9cbfe3a8db8bd87982b5aa363e70f` | `76a1af8439d57038d3e80ea41dc7b38ca3d9cbfe3a8db8bd87982b5aa363e70f` |
| `net/minecraft/world/level/levelgen/blending/Blender.java` | `1b0f51498d8f5e164b044d90bbef28b5a4c49762e6f6ae70b48ea4cf8c59bc0e` | `1b0f51498d8f5e164b044d90bbef28b5a4c49762e6f6ae70b48ea4cf8c59bc0e` |
| `net/minecraft/world/level/levelgen/material/MaterialRuleList.java` | `5b236d45b8910b4d9572a3c5e6a6cec9c9edd87eca1449939caf6d7a823e3cb0` | `5b236d45b8910b4d9572a3c5e6a6cec9c9edd87eca1449939caf6d7a823e3cb0` |
| `net/minecraft/world/level/levelgen/synth/BlendedNoise.java` | `7f66f06eb7ab25677842e3f042f185c728788d693faf6229078f6c829b11c967` | `7f66f06eb7ab25677842e3f042f185c728788d693faf6229078f6c829b11c967` |
| `net/minecraft/world/level/levelgen/synth/ImprovedNoise.java` | `f522c414939fe6ca27b94a1e758d2ab644d3ba43cbad201dd4368fb9fa0abc10` | `f522c414939fe6ca27b94a1e758d2ab644d3ba43cbad201dd4368fb9fa0abc10` |
| `net/minecraft/world/level/levelgen/synth/NormalNoise.java` | `70484ab5bb90f5971dc79428e9475edf0e4d0a2cf4e6c4cd2f30e5937a259c24` | `70484ab5bb90f5971dc79428e9475edf0e4d0a2cf4e6c4cd2f30e5937a259c24` |
| `net/minecraft/world/level/levelgen/synth/PerlinNoise.java` | `4966cfd8dc0623fbeca2f0e607d2763d1e9f146e067ce72a7de6406f96cc644e` | `4966cfd8dc0623fbeca2f0e607d2763d1e9f146e067ce72a7de6406f96cc644e` |
| `net/minecraft/world/level/levelgen/synth/SimplexNoise.java` | `d5e6944c34c6a58c95816fe103386d3254290a2eef347a4239a44eecd1f66280` | `d5e6944c34c6a58c95816fe103386d3254290a2eef347a4239a44eecd1f66280` |

## Tatsächliche Callverträge

| Bereich | Vollständig relevante Quellen/Symbole | Direkte JAR-Beobachtung |
| --- | --- | --- |
| Constructor/Sharing | NoiseChunk Constructor, wrap/wrapNew, material lambda; RandomState visitor/mapAll; NoiseRouter alle 15 Roots; Holder.Direct/Reference | 98 Fälle pro Backend; decoded/native und 7 Seeds; tatsächliche HashMap-keyclasses, Java-equals- und Objektidentitätspartitionen, bestehende und zweite Wrapperregistrierungsreihenfolge, Material-/Climate-Roots |
| Wrapper | Alle fünf NoiseChunk Wrapper einschließlich Constructors, compute, fillArray, boundsdelegation und Arrays | Sentinelhit, Yänderung, foreign/own, scalar-/arrayEpoch, Prefix/Länge, Failuretiming, nested/shared Inputs, cell/in-out, inclusive Flatgrid, echte Interpolatorupdates und Mth.lerp3 |
| Provider/Arrays | NoiseChunk sliceFillingContextProvider, forIndex, fillAllDirectly, selectCellYZ; DensityFunctions Ap2, MulOrAdd, RangeChoice, PureTransformer, TransformerWithContext, SimpleFunction | Childrequests, tatsächliche Arrayprefixe bei Fehler, Slot-/Countertiming, Registrierung, ADD/MUL/MIN/MAX/Range, contextual Blend/Weird und direkte Constant/Marker/Gradient-Fills |
| Preliminary | NoiseChunk preliminarySurfaceLevel und private computePreliminarySurfaceLevel; QuartPos/ColumnPos | Alle 49 setting/seed-Paare, 25 Grid + 32 Offgridspalten; Miss/Hit/Reordering; originaler strikter Threshold mit Nextafter, NaN, top/minY, MAX_SENTINEL und fehlendem Cachepublish bei Fehler |
| Climate | NoiseChunk.cachedClimateSampler und Climate.Sampler.sample/target/quantizeCoord | Originale 294 Quartfälle in tatsächlichen Sessions plus 784 weitere Grid/Offgrid/Yextremafälle; 6 reale Roots und f64→f32-Bits/quantisierte long-Achsen |
| Driverdiagnostik | NoiseBasedChunkGenerator doFill als Callerreferenz; NoiseChunk initialize/advance/select/update/swap/stop; separate Initial-/Finalsessions | 49 tatsächliche Sessiondiagnosen: Slices, Cornerfelder, Cellarrays, ausgewählte Driverwerte und Counter; bestehende 126 vollständige Paperarrays separat und unverändert |
| Equality | Holder.Direct/Reference; DensityFunctions Constant/Marker/NoiseHolder/End, CubicSpline Constant/Multipoint/mapAll und BlendedNoise | ±0/NaN/f64/f32, Direct/NativeReference, moderne gemeinsame und frische LegacyNoiseidentitäten, gleicher Splinearrayinhalt bei gleicher/verschiedener Arrayidentität, End/Blended identity |
| Random/Noise | BitRandomSource/XoroshiroRandomSource.nextDouble; Improved Constructor/Offsets/Shuffle; Perlin/Normal/Blended/Simplex Klassen | Direkte nextDouble-Methoden beider JARs, 14 Algorithmus/Seedfälle, tatsächliche Offsets/Permutation/Consumeorder und 7 vollständige boundNormalnoise-Offsetinventare |

## Kritische Abweichungen und Korrekturen

1. `values[this.arrayIndex++] = function.compute(this)` erhöht in beiden tatsächlichen JARs den Index **vor** dem Childcompute. Der erste Child sieht 1, der letzte einer 128er Zelle 128. `forIndex` setzt dagegen den angefragten Index ohne diesen Schritt; Sliceprovider erhöht seinen Scalarcounter. Das ist ein Java-Auswertungsordnungsbefund, keine Rustformel.
2. Rekonstruierte Base/Applied-Quellen zeigen in beiden Random-Implementierungen Floatliterale. Tatsächliches Vanilla-Bytecode verwendet `l2d/dmul`, Paper Build60 `l2f/fmul/f2d`. Quellenbytegleichheit beweist hier keine Laufzeitbitgleichheit. Die direkten Random-/Improvedproben bewahren beide Ergebnisse; GEN-05.2s ursprüngliche Vanilla-Punktbasis wird dadurch nicht geändert.
3. Der DensityFunctions-Sourcepatch erweitert den ersten Endradius nach long und fügt den tatsächlichen Paper-Endcache ein. Die vorhandenen 658 Originalpaare bleiben mit 210 Vanilla-NaNs und 56 endlichen Drifts erhalten; kein NaN-Ersatz wird als Gleichheit ausgegeben.
4. NoiseBasedChunkGenerator Sourcepatch reicht Weltkontext für spätere Surface/Carvingarbeit weiter. Die beiden relevanten Moonrisehunks ändern Executor-Aufrufe zu Runnable::run. Daraus folgt keine Rust-Thread-/Schedulingabnahme.
5. Holder-Sourceänderung verwendet ein Referenceset für Tags. Sie verändert die hier geprüfte Direct-record-Equality und Referenceobjektidentität nicht.

## Grenzen

Die Helpers rechnen keine Density-/Cache-/Noise-/Lerpformeln nach. Synthetische Kinder liefern deklarierte Werte, Arrays oder Fehler und protokollieren Requests; tatsächliche Servermethoden treffen die Entscheidungen. Numerische Diagnosen und Requestprobes verwenden getrennte Sessions. Fixturelokale IDs beruhen auf tatsächlicher Traversal-/Registrierungsreihenfolge; absolute Java-Identitätshashwerte werden nicht exportiert.

Original-JAR-Ausnahmen und FerrumCs zusätzliche finite-/Position-/Ressourcen-/Budgetrefusals sind unterschiedliche Verträge. FerrumCs Budgets, Atomizität, Poison/Reset und private Provideradmission sind Rust-Review-/Testarbeit beim Worldautor. Diese Javaartefakte belegen keine Aquifer-, Materialblock-, Biome-, Struktur-, Gameplay-, Client-, Survival- oder Pluginfunktion.
