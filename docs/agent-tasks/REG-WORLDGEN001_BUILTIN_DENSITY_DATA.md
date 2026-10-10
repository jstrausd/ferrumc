# REG-WORLDGEN001 — Originaldaten für Noise- und Densityconsumer

Status: implementiert durch `feature_catalog`, unabhängiges Rootreview
**CLEAN**; zur Integration auf `main` freigegeben. Registryprüfungen grün;
[Reviewbericht](REG-WORLDGEN001_BUILTIN_DENSITY_DATA_REVIEW.md).
Produktion ausschließlich `ferrumc-registry`; keine Weltgeneration allein
durch diesen Datenproducer.

## Freigegebener Implementierungsvertrag

Drei getrennte opaque Resource-IDs für Noise, Densityfunctions und Noise-Settings
mit exakten canonical-/bare-Lookups und allocfreier statischer Iteration. Noise-
Parameter entstehen als `f64::from_bits`; alle 102 Resources erhalten immutable
`OriginalDefinition`-Slices aus dem vorhandenen Export, Stats und typisierte
Dependencies. Originaldefinition bedeutet hier exakt den Fixture-Slice, nicht
die abweichende Whitespace-Serialisierung des inneren Original-JARs.

Build-only `serde_json`-Features `raw_value`/`float_roundtrip` und zusätzliches
build-only `sha2` 0.10 sind freigegeben; kein Runtime-Dependency oder Lockedit.
Vor Übernahme der Rawslices: duplicate-key-/Shapeprüfung, vollständige schema-
directed Referenzauflösung und Limits während der JSONmaterialisierung. Whole-
fixture: 4 MiB, 131072 Nodes, Depth32, 1024 Containerentries, Strings256 Bytes.
Pro ausgewählter Definition: 128 KiB, 4096 Nodes, Depth32, 256 Containerentries,
2048 Dependencyoccurrences und 512 deduplizierte Dependencies. Manifest16KiB,
Versionmanifest64KiB. Noise: 1..16 Amplituden, firstOctave -16..-1, finite Werte
0..2; diese Grenzen betreffen ausschließlich den gepinnten Builtinbestand.

Density-/Noiseverweise lösen vollständig auf geprüfte eigene IDs auf; Block,
Biome, Densityfunctiontype, Surfaceruletype, Surfaceconditiontype und synthetischer
RandomName bleiben getrennte Kategorien. Clamp-Directcodec, Shift-NoiseHolder
und unary-DensityHolder werden getrennt geprüft. Keine fehlende Referenz erhält
einen Fallback. Compiler, Sampler, Seedbindung und Chunkgeneration bleiben World.

## Ziel und Besitz

Die natürliche Terrainpipeline braucht versionsfeste Originalparameter statt
handkopierter Noise-/Routerdefaults. CAT-02.1 exportiert bereits unveränderte
Inhalte in `docs/fixtures/minecraft-1.21.8-original/worldgen.json`: **60 Noise-
Parameter, 35 Densityfunctionressourcen und sieben Noise-Settings**. Mache diese
aus dem bestehenden gepinnten Bestand für einen getrennten Worldconsumer
zugänglich. Nicht benötigte weitere Worldgenfamilien bleiben separate Tickets.

Exklusiv bei Vergabe: `server/crates/ferrumc-registry/**` und dieses Taskdoc.
Keine Originalfixture-, Scripts-, World-/Sim-/Storage-/App-/Lock-/Gitänderungen.
Vor Produktion konkrete API, Codegeneratorvalidation, Ressourcenbounds und
den erwarteten Density-/Noiseconsumervertrag an Root melden.

## Begrenzter Producervertrag

- Buildzeitgenerierung aus dem gepinnten Export; kein neuer Fetchschritt und
  keine manuell gepflegte Runtime-Parametertabelle. Fixture-/Manifest-/Version-
  und Mengenabweichungen verweigern. Duplicate JSONkeys und Resource-IDs,
  malformed Names, nichtendliche oder nicht passende Parameter klassifizieren.
- Geprüfte opaque Resource-/Familienidentität, deterministische Originalnames
  und vollständige allocfreie Lookups/Iteration. Unbekannt bleibt `None` oder
  klassifizierter Fehler; kein Defaultseed, Pig-/Overworldfallback oder
  normalisierendes Umschreiben fremder Namespacewerte.
- Noiseparameter können als immutable statische firstOctave-/Amplitudenwerte
  angeboten werden. Finite Werte, Längen-/Octavegrenzen und exakte Original-
  f64-Bits prüfen. RNGwahl, Namehash, Seedbindung und Samplerinstanz bleiben World.
- Density-/Settingsdarstellung vor Code entscheiden: checked generierter
  Strukturvertrag oder klar begrenzte immutable Originaldefinitionen. Keine
  ungeprüften public Nodefelder und kein Runtime-JSONparser innerhalb Registry.
  Falls World die Definitionen später checked kompiliert, Byte-/Depth-/Node-
  und Referenzlimits explizit festhalten; fehlender Compiler bleibt offen.
- Verweise zwischen den ausgewählten Density-/Noisebeständen vollständig
  auflösen oder ihre externe Registry-/Tag-/Biome-/Blockabhängigkeit typisiert
  kennzeichnen. Ein unbekannter Verweis darf nicht stillschweigend Nullnoise
  erzeugen. Surface/Biome/Feature-/Structureconsumer nicht als mitgeliefert ausgeben.
- Kein Init-I/O, keine global mutable Registry, kein unbounded Builder und
  keine Sampling-/Densityevaluation in dieser Crate. Resources und Localslots
  sind keine prozessweiten Entity- oder Worldidentitäten.

## Referenz und Abnahme

Minecraft Java1.21.8 / Protokoll772 / DataVersion4440. Paper Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Vollständige relevante Bootstrap- und
Codecpfadmethoden für NoiseData, NoiseRouterData, NoiseGeneratorSettings,
DensityFunctions und deren Registrylookup-/Holderconsumer samt Callees und
Paperpatches lesen. Tatsächliche Samplerlogik bleibt GEN-04 in World; Referenz-
lektüre und reine Ressourcengleichheit sind keine Terrain-/Seedparitätsabnahme.

Bestehendes Fixture-SHA256:
`bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30`,
2.530.249 Bytes. Bestehende CAT-02.1-Pins und Export-/Inventory-Selftests erhalten.

- Alle 60/35/7 Originalresources gegen tatsächlichen hashgeprüften Original-
  JAR-/Reportbestand vollständig vergleichen, keine Stichprobe als Vollbeleg.
- Öffentliche Vertragstests für Lookup/Iteration/Typtrennung, Unknown/Malformed,
  exakte Zahlen-/Definitionserhaltung und Crossreferences. Generatornegative
  Tests für Duplicatekeys, Drift, malformed Daten, Oversize und fremde Familie.
- Gesamte Registrytests, strict Alltargetclippy und fmt; exakte Source-/Consumer-/
 Ownership-/Bounds-/Abweichungsmatrix, Dateifreeze und Logs an Root.
- Root organisiert unabhängiges Review und Gitintegration. Densitygraphcompiler,
  NoiseRouter, NoiseChunk, Aquifers, Surface/Biome und aktive Chunkgeneration
  bleiben anschließend getrennte World-/Sim-/Appconsumer.

## Implementierungs- und Referenzmatrix

Referenz: angewandte Paperquellen unter
`.codex-tmp/paper-1.21.8-reference/paper-server/src/minecraft/java/net/minecraft/`;
Vanillabasis unter `paper-server/.gradle/caches/paperweight/mache/base/sources/`.
Relevante Bootstrap-/Codec-/Holdermethoden und die genannten Callees wurden
gelesen; die Matrix beschreibt ausschließlich Datenadmission und deren Consumer.

| Quelle und gelesener Pfad | Registryimplementation | Besitz/Bounds | Abweichung oder fehlender Consumer |
| --- | --- | --- | --- |
| `data/worldgen/NoiseData.bootstrap`, `registerBiomeNoises`, `register`; `NormalNoise.NoiseParameters.DIRECT_CODEC/CODEC` | Alle 60 Originalressourcen, firstOctave und 195 Amplituden; Runtimeparameter durch `f64::from_bits` | Build-only, 1..16 Amplituden/-16..-1/finite0..2/mindestens ein Nonzero; statische Runtimeborrows | Restriktiver gepinnter Builtinvertrag; RNG, Namenhash, Seed, Sampler bleiben World |
| `NoiseRouterData.bootstrap`, `registerTerrainNoises`, `registerAndWrap`, `getFunction`, Peaks/Caves/Underground/Postprocess/Slides und alle sieben Routerfactories | 35 Originaldensityresources und vollständige direkte Noise-/Densitydependencies; Zyklen über alle DensityHolder/Splinecoordinates abgewiesen | Opaque getrennte private Slots, exakt bekannte Namensmengen | Kein Bootstrapgraph oder Router wird in Registry evaluiert |
| `DensityFunctions.bootstrap`, DIRECT/NOISE_VALUE/Single-/Doubleargumentcodecs, Marker/Mapped/Noise/RangeChoice/ShiftA/ShiftB/ShiftedNoise/WeirdScaledSampler/YClampedGradient/BlendedNoise/Spline/Clamp/HolderHolder | Alle Formen/Schlüssel im gepinnten 35/7-Bestand geprüft; codec types separat typisiert; Inlineconstants und Holderstrings getrennt | Definitions- und Referenzlimits während Parse/Insertion; bounded CycleDFS über 35 Resources | Absente generische Formen, InlineNoiseparameter und dynamische Datapacks nicht freigegeben; fehlt nicht im gepinnten Resourcebestand |
| `DensityFunction.DIRECT_CODEC/CODEC/HOLDER_HELPER_CODEC/NoiseHolder`; `RegistryFileCodec.decode/encode`; `HolderGetter.getOrThrow` | Clamp.input nur number/object, Shift*.argument nur geprüfte Noisenamen, andere unary.argument DensityHolder; unknown/fremd/tag verweigert | Noiseref/Densityref sind unterschiedliche opaque IDtypen | Originale unseeded NoiseHolder-Nullnoise wird nicht als Unknownfallback angeboten |
| `CubicSpline.codec`, Multipoint/Builder; `DensityFunctions.Spline.Coordinate.CODEC`; vollständiger `TerrainProvider` | Verschachtelte Splinewerte/Koordinaten geprüft, finite f32-Werte und streng steigende f32-Locations | 4096Nodes/Depth32/Container256 je Definition, keine Expansion in Registry | World besitzt CubicSplineauswertung und gesamte Graph-/Worklimits |
| `NoiseGeneratorSettings.DIRECT_CODEC/CODEC/bootstrap`, end/nether/overworld/caves/floatingIslands; `NoiseSettings.CODEC/guardY`; `NoiseRouter.CODEC/field` | Alle sieben Settings, Geometrieguards, alle 15 Routerfelder, Defaults, Flags, SeaLevel, SpawnTarget und SurfaceRuleoriginale | Statische immutable vollständige Definitionslices; 128KiB pro Definition | Keine Dimensionaktivierung, NoiseChunk, Aquifers, OreVeinifier, Chunkgenerierung |
| Vollständiger `SurfaceRuleData`; `SurfaceRules.RuleSource/ConditionSource` Bootstrapcodecs samt Block/Biome/NoiseThreshold/Not/Sequence/TestRule/StoneDepth/VerticalGradient/Water/Ycodecs; `VerticalAnchor.CODEC`; `Climate.Parameter/ParameterPoint.CODEC`, `OverworldBiomeBuilder.spawnTarget` | Surface/Conditioncodec types, Block-/Biomeverweise und synthetische positional RNGnames getrennt; Blockeigenschaften gegen bestehenden Blockproducer geprüft, Biomes gegen gepinnte Originalfamilie | 2048 Occurrences/512 distinct Dependencies vor Insertion; externe Kategorien explizit | Daten-/Shapeprüfung implementiert; Surface/Biomebehavior und RNGfactory bleiben externe Worldconsumer; keine Tags im gepinnten Bestand |
| `DensityFunctions.java.patch`, Vergleich Vanillabasis/angewandte Klasse; Moonrisefeaturepatch nach ausgewählten Klassen durchsucht | Keine Änderung an Resource-/Bootstrap-/Codecpins | Paper Build60/Commit `29c8822d90899c89d2689338e81a98f690bcba12` | Einziger DensityFunctions-Diff: EndIslandcache/Overflowcast; Samplerconsumer prüft diese getrennt |

Die übrigen genannten Klassen sind in Basis/angewandter Quelle bytegleich;
DensityFunctions-Baseline/angewandter Sourcevergleich wurde vollständig geprüft.
Dies ist Sourcelektüre, keine Messung von Paper-/Terrain-/Seedparität.
Die Admission ist bewusst enger als generische Minecraftcodecs: nur gepinnte
Builtinformen/Namensräume, keine optionalen oder neuen Datapackformen, finite
f32-Splinewerte und der originale Bootstrap-Builderguard für steigende Locations.
Alle vorhandenen Originaldefinitionen erfüllen diese Guards; Rawdaten bleiben
unverändert. Eigene Caps sind keine Behauptung allgemeiner Codecgleichheit.

## Producerdaten und Ownership

- Runtime `worldgen`: `NoiseResourceId`, `DensityFunctionResourceId`,
  `NoiseSettingsResourceId`; jeweils `from_name`, `resource_location`,
  `definition`. Noise zusätzlich `parameters` mit `first_octave`/`amplitudes`.
  Keine unchecked/default/deserialize Konstruktion, keine numerische Slotexposure.
- `noise_resources`, `density_function_resources`, `noise_settings_resources`:
  allocfreie ExactSize-/DoubleEnded-/Fusediteratoren in canonical-name Reihenfolge.
  Exact bare/canonical Lookups, fremde Namespacewerte bleiben `None`.
- `OriginalDefinition`: private immutable fields mit `json`, `stats`,
  `dependencies`; Stats byte/node/depth/occurrence Getter, sortierte unique
  `ResourceDependency`-Slices. Perdefinitionbounds ersetzen keine World-
  Compiler-/Graph-/Work-/Seedlimits. JSON enthält Zahlenlexeme des Exports.
- Strikter bounded JSONvisitor lädt Werte/Keys nur innerhalb der freigegebenen
  Grenzen, verweigert decoded duplicate Keys vor next value, Containerüberschreitung
  vor weiterem Child und Depth-/Nodeüberschreitung vor Materialisierung.
  Wholefixture 4MiB ist zusätzliche obere Input-/Allocationgrenze; Manifest16KiB,
  Versionmanifest64KiB. RawValueübernahme folgt bounded duplicate-/Shapeprüfung;
  keine RuntimeJSONabhängigkeit. Build/dev raw_value/float_roundtrip +build sha2,
  alle schon gelockt, `server/Cargo.lock` unverändert.
- Alle 14 Fixturefamiliennamen/Mengen/Resourcenamen werden geprüft; ausschließlich
  60/35/7 ausgewählte Definitionen gelangen in Runtime. Wholefixture: 89285Nodes,
  Depth27, maxContainer258; ausgewählte maxima 350/72361/129853Bytes,
  21/2046/3411Nodes, Depth2/18/23. Runtimeoriginals total 1064169Bytes.
- Exklusive Änderungen: Registry und dieser Brief. Keine World-/Sim-/Storage-/
  Core-/App-/Scripts-/Originalfixture-/Lock-/Gitmutationen durch diesen Agent.

## Abnahmeevidenz

`tests/worldgen_resources.rs`: Alle 102 Namen/Rawexport-Slices/Iteratorlookups,
alle 195 Noisef64Bits gegen unabhängigen `str::parse::<f64>`, und für jede
Definition unabhängiger Originaltree-/Type-/Refcensus (Nodes/Depth/Occurrences/
vollständige typisierte direkte Dependencies) stimmen. Dieser Census ruft
Generatorparser und Schemavalidator nicht auf. Die Runtimegeneratorprüfung
kontrolliert zusätzlich vollständige Ausgabe/Stats/Refsets und Determinismus.

Generatornegative: decoded/escaped Duplicatekeys und doppelte Resource-ID,
Mengen-/Familien-/Namensdrift, unknown/foreign/tag References, Unknownforms und
Schlüssel, Noiseparameter, Densityzyklen, ClampDirect/ShiftNoise/unaryDensity-
Typverwechslung, Byte-/String-/Array-/Object-/Node-/Depthlimits und Occurrence- /
Distinctrefcaps. Positive entsprechende Codecpfade werden gesondert geprüft.
Das Selected-Rawbytecap wird mit legalem Padding ausschließlich innerhalb der
Noisefamilie geprüft; Wholefixturebytecap ist unabhängig davon.

`tests/reference/verify_builtin_worldgen.py`: ausschließlich offline aus
bestehendem Originalcache. Offizieller BundleSHA256
`2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7`,
InnerSHA256 `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`.
Alle 60/35/7 Resourcepaths und Originaldatagen/JAR-Rawbytes gleich; alle Inhalte
und exakten Decimalwerte stimmen mit dem unveränderten Export; alle 195
Noiseamplituden haben gleiche gerundete f64Bits. Drei vollständige Path/Byte/
SHA256indizes stimmen mit Manifest. Normaler und Python-`-O`-Lauf grün;
`require` bleibt auch unter Optimierung aktiv. Export/JAR-Whitespace und
Numbertokenspelling werden ausdrücklich nicht als rawbytegleich ausgegeben.

Isolierter Target: `.codex-tmp/reg-worldgen001-target`. Registry Debugtests:
37 Unit +21 Integration (3 bestehende Generator/5 Entity/13 Worldgen) +11
Doctests =69, alle grün; identische 69 Registrytests auch in Release grün.
Strict Alltargetclippy (`-D warnings`) und fmt grün, inklusive finalem RegistryREADME.
Finaler Freeze: `/tmp/reg-worldgen001-freeze.sha256`, zwölf Ticketdateien; Basis
bei Übergabe `3a5b4d62866d5f81d9820ec05dd3843389d32d90`. Bestehender Originalexporter unverändert:17 Selftests und
vollständiger Offline-Pincheck grün. Keine live Terrain-/Seed-/Gameplayabnahme.

Logs: `/tmp/reg-worldgen001-tests.log`, `/tmp/reg-worldgen001-clippy.log`,
`/tmp/reg-worldgen001-fmt.log`, `/tmp/reg-worldgen001-release-tests.log`,
`/tmp/reg-worldgen001-original-check.log`,
`/tmp/reg-worldgen001-original-check-optimized.log`,
`/tmp/reg-worldgen001-cat-check.log`.

`server/Cargo.lock` SHA256 unverändert:
`3b19aa0df48b04b1e6362c6ba06cb2c6e196232254fe0faf64b284c1a7c98b73`.
