# GEN-04.2 — Originale Normal-, Simplex- und Blendednoise

Stand: 10.10.2026. Implementierung und isolierte Crateabnahme abgeschlossen;
bereit für unabhängige Prüfung und Integration durch Root. Produktionscode
liegt ausschließlich in `ferrumc-world`. Dieses Ticket aktiviert keine natürliche
Welt, verändert keine Generatoridentität und belegt keine vollständige
Weltseed-/Terrainparität. GEN-04.1 ist als `e754254f` integriert; dessen Oracle
und Fixtures bleiben unverändert.

## Vertrag und öffentliche API

| API | Umgesetztes Verhalten |
| --- | --- |
| `NormalNoise::new/legacy(&mut WorldgenRandomSource, i32, &[f64])` | Zwei originale moderne named-octave bzw. explizite Legacy-Netherbiome-Perlinstacks. |
| `NormalNoise::first/second/value_factor/max_value/sample(Vec3)` | Originale Stackdiagnostik, nonzero-Amplitudenspanne, zweite Koordinatenskalierung und reine Samples. |
| `SimplexNoise::new(&mut WorldgenRandomSource)` | Drei originale Offsets und vollständiger ursprünglicher Shuffle einschließlich Parentverbrauch. |
| `SimplexNoise::offsets/permutation/sample_2d(f64,f64)/sample(Vec3)` | Originale 2D-/3D-Sampler, strikte/equal Branches, Gradienten und Java-i32-Overflow. |
| `BlendedNoiseParameters::new(f64,f64,f64,f64,f64)` | Private, vollständig vorab validierte originale CODEC-Konfigurationswerte; benannte Getter. |
| `BlendedNoise::new(&mut WorldgenRandomSource, parameters)/with_new_random` | Originale min/max/main-Perlinstacks mit 16/16/8 Legacyoctaves; Reconstruction nur aus gespeicherten Parametern. |
| `BlendedNoise::parameters/min_limit_noise/max_limit_noise/main_noise/min_value/max_value/sample(BlockPos)` | Originale Skalierung, acht-/sechzehn-Iteration, Clampflags, strikte clamped interpolation, Bounds und Diagnostik. |

Keine Defaults oder implizite Seed-/Algorithmus-/Terrainwahl. Die originale
`createUnseeded`-Convenience wird bewusst nicht angeboten. Aufrufer können
explizit einen Xoroshirostream mit numerischem Seed 0 konstruieren. Sampler und
Parameter besitzen ihre Daten exklusiv; Kopien sind unabhängige Werte.

Konfiguration wird vor RNGmutation und Allocation geprüft. Normal validiert
nicht nur den Perlinmaxwert, sondern auch die Addition beider Maxwerte und
nachfolgende Normalisierung. Die Konstruktoren übernehmen den aktualisierten
Parentzustand erst nach vollständigem Erfolg aus einem lokalen Snapshot.
Sampling allokiert nichts, verändert weder Sampler noch RNG und enthält keine
I/O-, Thread-, Packet-, Event- oder Storageabhängigkeit. F64-Reihenfolge bleibt
wie im Original; weder bewusste FMA noch algebraische Umordnung.

## Bewusste Grenzen und Abweichungen

- Normal übernimmt ausdrücklich die GEN-04.1-Konfigurationscaps: 1..64 gespeicherte
  Amplituden, erste/letzte Octave -64..32, endliche Amplituden und endliche
  Diagnosen; legacy zusätzlich letzter Exponent höchstens 0. Das sind
  zusätzliche FerrumC-Ressourcengrenzen, keine angenommenen Vanilla-Defaults.
- Normal verlangt endliche Eingänge mit Betrag höchstens
  `MAX_NORMAL_NOISE_COORDINATE = 2147483136.0 / 1.0181268882175227`.
  Die tatsächlich multiplizierten zweiten Koordinaten werden vor beiden
  Stacks nochmals gegen den öffentlichen Perlingrenzwert geprüft.
  Grenzwert und nächstkleinerer Float besitzen Originalgoldens;
  nächstgrößere Floats und der größere Perlingrenzwert werden explizit verworfen.
- Simplex akzeptiert endliche Koordinaten mit Betrag höchstens 2147483136.
  Seine Offsets sind original vorhandene Diagnosen; die originalen Sampler
  addieren sie nicht. Nur die 256 tatsächlich adressierten Permutationswerte
  werden gespeichert. Die Java-512er-int-Liste enthält darüber hinaus nur
  unadressierte Nullen; die Oracle prüft diese Annahme ausdrücklich.
  Skew-Floors bleiben Java-int-Casts mit ursprünglicher Sättigung und
  Overflowkorrektur; Flooradditionen wrappen als i32.
- Blended unterstützt den originalen serialisierten CODEC-Bereich: vier
  Faktoren inklusive Endpunkten 0.001..1000, Smearmultiplier 1..8. Der originale
  Java-Testingconstructor validiert diesen Bereich nicht und akzeptiert auch
  außerhalb liegende/unendliche Werte; diese zusätzliche Rustprüfung und ihr
  früherer Fehlerzeitpunkt sind ausdrücklich keine Parität für diesen
  weiter gefassten Constructor. Fehler nennen den konkreten Parameter.
- Blended akzeptiert den ganzen i32-`BlockPos`-Bereich. Öffentliche Improved-
  oder Perlin-Koordinatencaps werden nicht auf seine originalen internen
  skalierten/wrapped Koordinaten oder unwrapped yMax übertragen. Die
  validated CODEC-Faktoren erzeugen endliche Intermediatewerte; nichtendliche
  Endergebnisse werden klassifiziert, niemals in Ersatzterrain umgewandelt.
- `max_value` folgt der originalen Diagnose, einschließlich signed amplitudes
  bei Normal und Legacyeinheiten bei Blended. Dies ist keine neue absolute
  mathematische Schranke oder zugesicherte Terrainhöhe.
- Sampling/Reconstruction erzeugt keine Pluginereignisse. Gaussian,
  `WorldgenRandom`-Wrapper, EndIslanddensity samt Paperoptimierungen,
  Densityrouter, Klima/Biome, Terrain/Features/Structures und natürliche
  Chunkconsumer bleiben eigenständige Folgetickets.

## Quellenreferenz

Minecraft Java 1.21.8 / Protokoll 772, Paper Build 60 / Commit
`29c8822d90899c89d2689338e81a98f690bcba12`; angewandte Quellen unter
`.codex-tmp/paper-1.21.8-reference/paper-server/src/minecraft/java/`.
Vollständige `NormalNoise`, `SimplexNoise`, `BlendedNoise`, `PerlinNoise` und
`ImprovedNoise` sowie die benötigten `Mth.floor/lfloor/clampedLerp/lerp`-
Callees wurden gelesen. Keine direkten Paperpatches verändern diese drei
Sampler. Quellenreview ist vom numerischen Abnahmenachweis getrennt.

Angewandte `DensityFunctions.EndIslandDensityFunction` einschließlich des
vollständigen Islandheightconsumer und `NoiseRouterData`-Registrierung/Nether-
Endrouter wurden betrachtet. Originale Routerbaselinekonfigurationen sind
Overworld `(0.25,0.125,80,160,8)`, Nether `(0.25,0.375,80,60,8)` und End
`(0.25,0.25,80,160,4)`. Alle sind in der Fixture enthalten. EndIsland startet
Legacy mit 17.292 Verbrauchsschritten vor Simplexconstruction; entsprechende
Samplerinitialisierung ist als Primitive getestet, der Islandconsumer bleibt
unimplementiert. Papers `DensityFunctions.java.patch` fügt Islandcaching und
eine long-Multiplikation gegen originalen Distanzoverflow hinzu; diese
Consumeränderungen werden nicht stillschweigend in Simplex übernommen.

## Direkte Original-JAR-Oracle

Neue, getrennte `tests/reference/CompositeNoiseOracle.java` und
`generate_composite_noise.py` führen Reflexionsaufrufe gegen die tatsächlichen
obfuskierten Originalklassen `ezr`, `ezo`/`ezo$a`, `ezl`, `ezp`, `ezm`,
RNG `bck`/`ekb`/`elb`, `ejt$b`/`ejt$e` und `Mth` `bcb` aus.
Die Oracle enthält keine nachimplementierte numerische Noise-/RNGformel.
Originale SharedConstants-Versionsdetektion und Bootstrap werden zuerst
aufgerufen; Klassen werden zunächst ohne Initialisierung geladen, damit
Blendeds originale Densityfunction-CODEC-Klassen registriert starten.

Sicherheits-/Reproduktionsvertrag: offizielles Bundle, inneres JAR, offizielle
Mappings und jede Bundledlibrary werden hart gehasht; explizite `require`-
Prüfungen bleiben unter Python `-O` aktiv. Frisch kompilierter temporärer
Bytecode, `javac -proc:none`, Original-JAR und gepinnte Libraries zuerst im
Classpath. Kein altes Probeclass-Verzeichnis kann Originalklassen ersetzen.
Alle Targets/Exports liegen auf Disk unter `.codex-tmp`, nicht im knappen `/tmp`.

| Artefakt | SHA256 |
| --- | --- |
| Offizielles Bundle | `2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7` |
| Inneres Server-JAR | `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250` |
| Offizielle Mappings | `e373ba569839e6f3c7bd4880247f3e85577d92cde2c605adfa2024eb9cae503e` |
| Neue Compositefixture | `af477c940f8a807334686d1aa2ebfcc69618130f856baf6ee93fac12c31baa6d` |

Mappings SHA1 `eb1e1eb47cb740012fc82eacc394859463684132`.
Das getrennte Manifest enthält alle Librarypins, Oracle-/Helper-/Fixturehashes,
Java/Javac-, Python-/Plattformversionen, Dateigröße und alle Recordzahlen.
Java 21 ist verlangt; Abnahme mit Temurin 21.0.12.1+1, Python 3.14,
Rust 1.99.0 (b940084d7, LLVM23.1.1), x86_64-unknown-linux-gnu.
Die Bitgleichheit gilt für diese geprüfte Werkzeugkette/Fälle; keine
pauschale CPU-/OS-/Crossplatformzusage.

16.974 neue Records / 4.681.344 Bytes:

| Records | Anzahl | Umfang |
| --- | ---: | --- |
| S/S2/S3 | 32/736/736 | Beide RNGs, acht Seeds einschließlich i64extrema, normaler oder 17.292-Vorlauf, komplette Permutationen/Offsets/Parentlong; sechs 3D-Ordnungen, Gleichheiten, signedzero/subnormal, negative/Zell-/Skewoverflowkoordinaten. |
| N/NO/NV | 224/2.976/4.928 | Beide RNGs, modern/legacy, Gaps/Nullamplituden/signedzero/negative amplitudes/64span/-64/+32; beide kompletten Stacks, Span-/Maxfaktoren, Parentlong und Normalcallergrenzen. |
| B/BO/BV/BA | 128/5.120/2.048/16 | Drei originale Routerkonfigurationen, CODEC-Endpunkte, alle 40 Octaves, Crossalgorithm-withNewRandom, Parentverbrauch beider Streams, unveränderte Altsamples und ganze i32-Koordinaten. |
| CL | 30 | Tatsächliches originales Mth.clampedLerp mit Signedzero, direkt angrenzenden 0-/1-Grenzfloats, Innen-/Außenwerten und großen benachbarten Endpunkten. |

Die Clampbranchcoverage der tatsächlichen Rustsampler gegen Original-BV-
Goldens belegt Minimumclamp, Inneninterpolation und Maximumclamp. Separate
private CL-Goldentests prüfen strikte Clampgrenzen und ursprüngliche
Lerp-Rundung. Öffentliche Contracttests prüfen zudem Fehlkonfigurationen,
NaN/infinity, Größen-/Exponentcaps, jede Blendedparametergrenze,
Parentzustanderhaltung, Eingangsfehler und reine order-independent Samples.

## Abnahme und Reproduktion

- Gesamte Worldcrate: **153 Tests grün**, 120 unit +7 neue öffentliche
  Compositecontracts +5 Generationidentity +14 Blockentity +7 GEN-04.1;
  `/tmp/gen04_2-world-test.log`.
- Release: **134 Tests grün**, 120 unit einschließlich Clampgoldens +beide
  sieben öffentlichen Noisecontracts; `/tmp/gen04_2-release.log`.
- Strict all-targets Clippy, inklusive Dependencies: exit0,
  `/tmp/gen04_2-clippy.log`; keine Lints unterdrückt.
- Frischer normaler und Python-`-O`-Reexport sind bytegleich mit eingecheckter
  Fixture **und Manifest**, einschließlich temporär neu kompiliertem Bytecode;
  `/tmp/gen04_2-oracle-a.log`, `/tmp/gen04_2-oracle-b.log`.
- Negativehelperchecks unter Python `-O`: falsches Bundle/inneres JAR/Mappings/
  Library werden klassifiziert vor jedem Fixtureexport abgelehnt;
  `/tmp/gen04_2-oracle-negative.log`.
- `cargo fmt -p ferrumc-world -- --check`: exit0,
  `/tmp/gen04_2-fmt.log`. Finales 12-Datei-SHA256-Freeze:
  `/tmp/gen04_2-freeze.sha256`, vollständig gegen die Dateien geprüft.
  Root übernimmt unabhängige Review und Gitintegration; keine eigenen
  Git-/Lockfile-/fremden-Crateänderungen.

```bash
python3 server/crates/ferrumc-world/tests/reference/generate_composite_noise.py \
  .codex-tmp/original-1.21.8 \
  /home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1 \
  .codex-tmp/gen04_2-reference/fresh-export
cd server
cargo test --offline --locked -p ferrumc-world --target-dir ../.codex-tmp/gen04-target
cargo test --release --offline --locked -p ferrumc-world --lib \
  --test composite_noise_contract --test random_noise_contract \
  --target-dir ../.codex-tmp/gen04-target
cargo clippy --offline --locked -p ferrumc-world --all-targets \
  --target-dir ../.codex-tmp/gen04-target -- -D warnings
cargo fmt -p ferrumc-world -- --check
```

Cache-, Java- und Outputpfade sind explizite CLI-Argumente. Ein frischer
Outputordner lädt und prüft die offiziellen Mappings; die vorhandene verifizierte
Mappingdatei kann zuvor kopiert werden. Helper überschreibt keine eingecheckte
Fixture automatisch. Fertig getestete Primitiven erlauben noch keine vollständig
spielbare Survivalwelt, Mobs oder Java/Paper-Plugins.
