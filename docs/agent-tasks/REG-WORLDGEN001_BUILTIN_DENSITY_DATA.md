# REG-WORLDGEN001 — Originaldaten für Noise- und Densityconsumer

Status: vergeben an `feature_catalog`, in Umsetzung. API- und Boundsentwurf
durch Root freigegeben. Produktion ausschließlich `ferrumc-registry`; keine
Weltgeneration allein durch diesen Datenproducer.

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
