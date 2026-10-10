# GEN-04.1 — originale Random-/Noise-Primitiven

Stand: 10.10.2026. Implementierung und isolierte Crateabnahme abgeschlossen;
bereit für unabhängige Prüfung und Integration durch den Orchestrator.
Keine Terrainaktivierung und keine Erweiterung der geschlossenen
`GenerationIdentity`. GEN-04 bleibt für natürliche Welten offen. Diese Abnahme
belegt die unten getesteten Primitiven, keine vollständige Welt-/Seedparität.

## Festgelegter Vertrag

Pure, exklusiv besessene Minecraft-Java-1.21.8-Primitiven: Legacy48 und
Xoroshiro128++, signed numeric Seed/128-bit-Upgrade, reseed/fork/positional
factory, int/bounded int/long/boolean/float/double/consumeCount. Positional
Koordinaten sind `BlockPos`; Legacy-Namen verwenden Java-UTF16-Hash, moderne
Namen MD5 von UTF-8 mit Big-Endian-Seedhälften. Vorhandenes `md-5` 0.10.6
vermeidet eine eigene Hashimplementierung. Keine UUID-/Zeit-/Textseedwahl.
Gaussian bleibt ausdrücklich offen; diese Noiseinitialisierung benötigt es
nicht. Java-Strings mit isolierten Surrogaten sind durch Rust-`str` nicht
darstellbar.

`ImprovedNoise` besitzt feste 256er Permutation und drei Offsets und bietet
Sampling, originales yScale/yMax-Verhalten sowie analytische Ableitungen.
`PerlinNoise` besitzt maximal 64 Octaves, moderne named-octave factories und
einen expliziten Legacykonstruktor mit originaler 262-step Gapconsumption,
wrap, fixedY und Amplituden-/Maxvalue-Faktoren. Keine approximate Noise,
Terrainfallbacks, Densityrouter, Biome- oder Chunkconsumer.

Koordinaten kommen als `Vec3`. f64-Reihenfolge folgt dem Original ohne
absichtliche FMA oder Reassoziierung. Sampling allokiert nichts und verändert
den Sampler nicht. Konfigurationsfehler werden vor Allokation und RNGverbrauch
klassifiziert. Diese bewusst frühere Fehlerprüfung sowie die folgenden
Grenzen sind FerrumC-Verträge, keine zusätzlichen Vanilla-Paritätszusagen.

## Öffentliche API und bewusste Grenzen

| Modul/Typ | Vertrag |
| --- | --- |
| `RandomAlgorithm` | Explizit `Legacy` (48-bit) oder `Xoroshiro128PlusPlus`; keine implizite Auswahl. |
| `Seed128` | `new`, `upgrade`, `low`, `high`; originale Stafford13-Seedmischung und All-zero-Fallback des Xoroshiro-Streams. |
| `WorldgenRandomSource` | `new`, `legacy`, `xoroshiro`, `xoroshiro_128`, `algorithm`, `set_seed`, `fork`, `fork_positional`; originaler Elternverbrauch. |
| Random-Ausgaben | `next_int`, `next_int_bounded`, `next_long`, `next_boolean`, `next_float`, `next_double`, `consume_count`. |
| `PositionalRandomFactory` | `legacy`, `xoroshiro`, `at(BlockPos)`, `from_hash_of`, `from_seed`; unveränderlich und unabhängig von Abfragereihenfolge. |
| `ImprovedNoise` | `new`, `offsets`, `permutation`, `sample`, `sample_with_y`, `sample_with_derivative`; volle 256er Permutation und originale Interpolation. |
| `NoiseSample` | `value`, `derivative`; Ableitung als `Vec3`. |
| `PerlinNoise` | `new` für moderne benannte Octaves, `legacy` für explizite Legacyinitialisierung; `first_octave`, `amplitudes`, `octave_noise`, `max_value`, `max_broken_value`, `sample`, `sample_with_y`. |
| `wrap_noise_coordinate` | Originale 33.554.432-Perioden-Wraprechnung innerhalb des unterstützten Koordinatenbereichs. |

- `next_int_bounded` akzeptiert positive `i32`-Bounds; ungültige Bounds liefern
  `RandomError::InvalidBound`, ohne den Stream zu verändern. Java-Überläufe,
  signed Legacy-`nextLong`-Summanden und die ursprüngliche Rejectionconsumption
  bleiben erhalten.
- `consume_count` nimmt `u32` mit maximal 1.048.576 Steps an. Null konsumiert
  nichts; eine Überschreitung liefert `RandomError::ConsumeCountTooLarge`, ohne Mutation.
  Java kann einen negativen signed Count entgegennehmen; die Rust-API stellt
  diesen nicht dar.
- Legacy-Koordinatenhashing multipliziert x zuerst mit `i32`-Overflow und z mit
  `i64`-Overflow. Legacy-Namen verwenden Java-UTF16, Xoroshiro-Namen MD5 von
  UTF-8 mit Big-Endian-Hälften. Nicht gepaarte Java-Surrogate sind mit `str`
  nicht darstellbar. Der Legacy-Factory-Seed beeinflusst `from_seed` wie im
  Original nicht; Xoroshiro XORt beide Factoryhälften mit dem Eingabeseed.
- Perlin akzeptiert 1..64 Amplituden und erste/letzte Exponenten -64..32.
  Alle Amplituden und die errechnete Maxvalue-Diagnose müssen endlich sein.
  Negative Amplituden und signed zero bleiben erhalten. `max_value` ist die
  originale Diagnoseformel und kein neuer nichtnegativer Absolutwert-Bound.
- Legacy-Perlin lehnt jeden positiven letzten Exponenten ab, auch bei einer
  Nullamplitude dort. Er erzeugt immer das anfängliche Noiseobjekt für Octave
  null. Untere Gaps und Octaves außerhalb der gespeicherten Spanne konsumieren
  exakt 262 `next_int`-Aufrufe. Moderne Initialisierung konsumiert immer einen
  Positionalfork; ihre Nullgaps erzeugen keine Noiseobjekte.
- `octave_noise` zählt rückwärts: Index null bezeichnet den höchsten
  Exponenten. Gaps und ungültige Indizes, einschließlich `usize::MAX`, ergeben
  `None`.
- Koordinaten sowie yScale/yMax müssen endlich sein und einen Betrag von
  maximal 2.147.483.136 besitzen. Das gilt auch für die öffentliche Wrapfunktion.
  `max_broken_value` verlangt einen endlichen Multiplikator. Nichtendliche
  Rechenergebnisse werden als `NoiseError::NonFiniteResult` zurückgewiesen.
- ImprovedNoise übernimmt die originale f32-Epsilonkonstante, die vor der
  yScale-Quantisierung zu f64 erweitert wird. Die Java-Derivativemethode addiert
  in ein vom Aufrufer übergebenes Array; Rust liefert einen reinen `NoiseSample`.
  Die Verantwortung für additive Akkumulation bleibt beim späteren Consumer.
- `WorldgenRandomSource` ist ein exklusiv besessener primitiver Stream. Sein
  Name behauptet keine Implementierung des Java-`WorldgenRandom`-Wrappers oder
  dessen Verbrauchszähler, Bitadapter und Feature-/Decorationseedmethoden.

## Originalreferenz

Minecraft Java 1.21.8, Protokoll 772; Paper Build 60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`.
Alle folgenden Klassen wurden in der angewendeten Original-/Paperreferenz
vollständig gelesen; bei Callees wurden die angegebenen vollständigen Methoden
gelesen. Das Quellenlesen ist getrennt vom unten beschriebenen numerischen
Differenzialnachweis.

| Referenz | Übernommener oder abgegrenzter Vertrag |
| --- | --- |
| `LegacyRandomSource`, `BitRandomSource` | 48-bit-LCG, Bits, Boundrejection, signed long, float/double, fork/factory/reseed. |
| `XoroshiroRandomSource`, `Xoroshiro128PlusPlus` | 128-bit-Zustand, ++-Ausgabe, zero-state-Fallback, unsigned bounded multiplication/rejection, fork/factory. |
| `RandomSupport`, `PositionalRandomFactory`, `RandomSource` | Seedupgrade, UTF8-MD5/UTF16, Koordinaten- und Namensfabriken; Gaussian/ranged/triangle bleiben offen. |
| `SingleThreadedRandomSource`, `ThreadSafeLegacyRandomSource` | Gegenprüfung der Legacy-Bitsemantik; kein Thread-/Atomicmodell übernommen. |
| `MarsagliaPolarGaussian` | Gelesen; Cache/Gaussian ist nicht umgesetzt. |
| `ImprovedNoise` | Offsets, Shuffle, Gradienten, Interpolation, yScale/yMax und analytische Ableitungen. |
| `PerlinNoise` | Explizite Octaves/Amplituden, moderne Namen, Legacy-Gaps, Frequenz-/Wertfaktoren, wrap/fixedY/max-Diagnosen. |
| `WorldgenRandom` | Gelesen; Wrappercounter, Bits und Decoration-/Feature-/Slime-Seeding bleiben offen. |
| `Mth` | floor, lfloor, getSeed, lerp/lerp2/lerp3, smoothstep und Ableitung. |
| `SimplexNoise` | Gradienten-/dot-Callees geprüft; kein Simplexsampler implementiert. |

Die RNG-/Noise-Primitiven besitzen keine direkten Paper-Sourcepatches.
Consumerpatches und Randomersetzungen sind spätere Arbeit. Die relevanten
`DensityFunctions.java.patch`-Änderungen zu EndIsland/Simplex sowie der kleine
`NoiseBasedChunkGenerator.java.patch` zu Flatbedrock betreffen noch offene
Consumer. Dieser Modellschritt löst keine Events oder Pakete aus und enthält
keine Storage- oder Threadhandles.

## Differenzialreferenz aus dem echten Original-JAR

`tests/reference/RandomNoiseOracle.java` enthält nur Reflexionsaufrufe und
Testfälle, keine nachgebauten RNG-/Noisealgorithmen. Es ruft die tatsächlichen
obfuskierten Klassen aus dem offiziellen Minecraft-1.21.8-Server-JAR auf:
`bck`, `ekb`, `elb`, `ekn`, `ekp`/`ekp$a`, `ezm`, `ezp`. Das eigene Paket
`ferrumc.reference` verhindert einen Signermismatch mit signierten
Defaultpackage-Originalklassen.

| Artefakt | Pin |
| --- | --- |
| Offizielles Serverbundle SHA256 | `2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7` |
| Inneres Server-JAR SHA256 | `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250` |
| Offizielle Servermappings SHA1 | `eb1e1eb47cb740012fc82eacc394859463684132` |
| Offizielle Servermappings SHA256 | `e373ba569839e6f3c7bd4880247f3e85577d92cde2c605adfa2024eb9cae503e` |
| Mappinggröße | 8.186.232 Bytes |
| Fixture SHA256 | `b75e8d2caeb4ba7863c04a33554ffeace160ac2ded6c912bb43ad373362592a8` |
| Fixturegröße / Records | 1.555.442 Bytes / 16.556 Records |

Die Mappingquelle lautet
`https://piston-data.mojang.com/v1/objects/eb1e1eb47cb740012fc82eacc394859463684132/server.txt`.
Das eingecheckte Manifest enthält zusätzlich sämtliche Librarypins sowie
Generator-/Oraclehashes und die exakte Java-Version. Der Generator validiert
Bundle, inneres JAR, Mappinggröße/-hash und alle Library-SHA256 aus dem
gepinnten `META-INF/libraries.list`, bevor Java aufgerufen wird. Die Prüfung
verwendet explizite Fehler, keine mit Python `-O` abschaltbaren Assertions.
Frische temporäre Oracleclasses werden pro Lauf verwendet; die Original-JARs
stehen zuerst im Classpath. Es werden keine eingecheckten Fixtures überschrieben.

Reproduktion ab Repositoryroot mit dem vorhandenen Cache aus
`minecraft-original-export.py`:

```sh
python3 server/crates/ferrumc-world/tests/reference/generate_random_noise.py \
  .codex-tmp/original-1.21.8 \
  /home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1 \
  .codex-tmp/gen04-reference/reproduced
```

Die Cache-, Java- und Ausgabeordner sind CLI-Argumente. Java 21 wird explizit
geprüft; die Abnahme nutzte Temurin 21.0.12.1+1. Zwei frische Exporte mit normalem
Python und Python `-O` erzeugten byteidentische Fixture und Manifest, auch zur
eingecheckten Fixture. Ein separater korrumpierter Bundleprobe unter `-O`
scheiterte mit `ValueError: official bundle SHA256 mismatch`, bevor das
präparierte Java-Markerscript ausgeführt oder eine Fixture erzeugt wurde.

| Fixturegruppe | Records | Abgedeckte Fälle |
| --- | ---: | --- |
| `R`, `U`, `Z`, `P`, `E` | 6.488 | Randomstreams, Seedupgrade, raw Xoroshiro/zero-state, factories, originale Boundfehler und Folgestream. |
| `I`, `N`, `D` | 976 | Vollständige Permutationen/Offsets/Folgestream, Samples mit yScale/yMax, analytische Ableitungen. |
| `C`, `O`, `B`, `V`, `W` | 9.092 | Moderne/Legacykonstruktoren und Gaps, volle Octavekonfigurationen, maxBrokenValue, fixedY, Wrapgrenzen. |

Seeds umfassen 0, 1, -1, 42, `i64::MIN/MAX` und weitere große positive/negative
Werte. Koordinaten umfassen negative Grenzen, Blockhöhen, Weltborder,
Wrap-Halbperioden und den exakten unterstützten Höchstbetrag. Namen umfassen
UTF8/UTF16-Emoji, Combiningmarks, NUL und 63/64/65-Byte-MD5-Grenzen.
Noiseprüfungen umfassen subnormalen yScale, signed zero, negative Amplituden,
64 Octaves, moderne Exponenten -64 und 32 und komplette Nullamplituden.

Alle Integer und f32/f64-Bits der 16.556 Referenzrecords stimmen exakt,
ohne Toleranz, sowohl im Debug- als auch im optimierten Releasebuild überein.
Werkzeugkette: `rustc 1.99.0 (b940084d7 2026-09-28)`, LLVM 23.1.1,
`x86_64-unknown-linux-gnu`. Der Nachweis gilt für die geprüften Fälle auf dieser
Werkzeugkette; keine pauschale Crossplatform- oder komplette Weltseedzusage.

## Abnahme

Isolierter Targetordner: `.codex-tmp/gen04-target`. Aus `server/` bestanden:

```sh
CARGO_TARGET_DIR=../.codex-tmp/gen04-target cargo test --offline --locked -p ferrumc-world
CARGO_TARGET_DIR=../.codex-tmp/gen04-target cargo test --release --offline --locked -p ferrumc-world --test random_noise_contract
CARGO_TARGET_DIR=../.codex-tmp/gen04-target cargo clippy --offline --locked -p ferrumc-world --all-targets -- -D warnings
rustfmt --edition 2021 --check crates/ferrumc-world/src/random.rs crates/ferrumc-world/src/noise.rs crates/ferrumc-world/tests/random_noise_contract.rs
```

- World-Tests: 118 Unit-, 5 Blockentity-, 14 Generation- und 7 neue
  Random-/Noise-Contracttests; insgesamt 144 erfolgreich, keine Doctests.
- Release-Differenzialtests: 7/7 erfolgreich, einschließlich aller
  Referenzbitvergleiche.
- Strikte Crate-Clippy und Formatprüfung erfolgreich. Diese isolierte Abnahme
  ersetzt keine anschließende Workspaceintegration durch den Orchestrator.
- Zusätzliche Contracttests prüfen Fehlereingaben ohne RNGmutation,
  finite/extreme Bounds, reine Samplingreihenfolge und unveränderliche
  Factoryabfragen unabhängig von Reihenfolge.
- Neue direkte Dependency: bestehendes `md-5` 0.10.6. Die freigegebene
  Lockänderung ist nur dessen Eintrag in den Worlddependencies; keine neuen
  Packageversionen durch dieses Ticket.

Lokale Logs: `/tmp/ferrumc-gen04-tests.log`,
`/tmp/ferrumc-gen04-tests-release.log`, `/tmp/ferrumc-gen04-clippy.log`,
`/tmp/ferrumc-gen04-fmt.log`, `/tmp/ferrumc-gen04-java-oracle-final-a.log`,
`/tmp/ferrumc-gen04-java-oracle-final-b.log`,
`/tmp/ferrumc-gen04-oracle-negative.log`.

## Offene Folgetickets

Gaussian inklusive Cache-/Reseedvertrag, weitere RandomSource-Helfer
(ranges/triangle), `WorldgenRandom`-Wrapper-/Verbrauchszählung und Decoration-/
Feature-/Slime-Seeding, Normal-/Blended-/Simplexnoise, Densityrouter und deren
Sampler, Klima-/Biomeauswahl, natürliche Overworld-/Nether-/Endgeneratoren,
Feature-/Structureplacement und vollständige Weltseedparität bleiben offen.
`generation.rs`, GenerationIdentity und der aktive Flatgenerator wurden nicht
geändert. Es existiert kein Terrainconsumer und kein approximativer Fallback
aus diesem Ticket.
