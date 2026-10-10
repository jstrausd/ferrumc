# GEN-05.3 — unabhängiger tatsächlicher NoiseChunk-Oracle

Stand: 10.10.2026. Die Java-Referenzexporte sind vollständig erstellt;
der Rust-Code erhält eine getrennte Prüfung. Dieser Brief erklärt die
Numerical-/Cache-/Array-/Identitybelege. Er schließt keine aktive
Weltgenerierung oder Survivalfunktion ab.

## Umfang und unveränderte Basis

Minecraft Java 1.21.8, Protokoll 772, DataVersion 4440; Paper Build 60,
Commit `29c8822d90899c89d2689338e81a98f690bcba12`; Temurin 21.0.12.1+1.
Die Helpers verwenden tatsächliche Serverklassen beider JARs, offizielle
gehashte Mappings und frisch kompilierte eigene Klassen an letzter
Classpathposition. Synthetische DensityFunction-Kinder liefern deklarierte
Werte/Arrays/Fehler und protokollieren Requests. Cache-, Density-, Noise-
und Lerpformeln wurden nicht als Groundtruth nachgeschrieben.

Die bereits eingefrorenen **126 vollständigen Paper-Materialdensitychunks**
werden weiterverwendet: 7.077.888 Werte, 56.623.104 Bytes, Bits-SHA256
`9dd6827609a0a6b65968b239a588fd60e21121097520501ff5828eb19655a9db`.
Der bestehende Freeze bleibt
`80e17003c7229cf769bda08a3b90094f27fc9bded97032d1991a294813f6bb20`.
Es wurde keine zweite Paper-Materialbitdatei erzeugt. GEN-05.2s ursprünglicher
Vanilla-Punktvertrag und seine Referenzen wurden nicht verändert.

## Bestandene tatsächliche JAR-Matrix

Pro Backend enthält der Kernexport **474 Zeilen**:

| Bereich | Tatsächliche Fälle |
| --- | --- |
| Constructor/Sharing | 98: 7 Settings × decoded/native × 7 Seeds; Keyclasses, Equals-/Identitypartitionen, 15 Routerroots, Material-/Climate-Roots, Registrierung und zweite Wraptraversal |
| Wrapper | 55: alle fünf Wrapper, Sentinel/counter0, own/foreign, In-/Outcell, Flatgrid, Prefix/Längen, Near-wrap, Failuretiming, nested/shared Inputs und echte Interpolatorupdates |
| Arraydispatch | 32: ADD, MUL ±0-Skip, MulOrAddzero mit Inputrequest, strikte MIN/MAX-Bounds, RangeChoice, Pure-/Contexttransformer, Weird, Constant/Blend/Beard/Gradient und eigene CacheOnce-Providergrenze |
| Provider | 12: Slice-/Cell-forIndex/direct-fill, Counter/Arrayindextiming und Cellcache-Registrierungsreihenfolge |
| Java Equality | 23: f64/f32 ±0/NaNs, Direct/Referenceholder, modern shared/legacy fresh Noise, Splinearrayidentität, End/Blended identity |
| Preliminary | 49 × 57 Spalten: 25 inclusive Grid und 32 Offgrid/unaligned/negative/halo; Hits und frische Reverseorder; zusätzlich 84 Threshold-/minY-/topY-/MAX-/NaN-/Failuretraces |
| CachedClimate | 49 × 22 Quartproben = 1.078: ursprüngliche 294 plus 784 Grid/Offgrid/Yextremafälle, f32-Bits und quantisierte long-Werte |
| Getrennte Diagnostik | 49: Initial- und bare-Final-Punkte in getrennten Sessions; Slice-/Corner-/Cellarrays, Driverwerte, Swap und Counter |
| Random/Noise | 14 direkte Algorithmus/Seed-Controls mit nextDouble/Improved-Offsets/Permutation/Consumeorder; 7 boundNormalnoise-Offsetinventare |
| Provenienz | 2 Zeilen vor/nach den Calls; 77 tatsächlich geprüfte CodeSources einschließlich Bibliotheks-/Helperursprüngen |

Constructor-, Wrapper-, Array-, Provider- und Equalityresultate sind zwischen
den Backends gleich. Numerische Abweichungen bleiben erhalten: 14 Random-,
7 Offset-, 42 Climate- und 49 Diagnostikzeilen unterscheiden sich. Alle
14 Improved-Permutationen und RNG-Verbrauchsfolgen sind backendgleich;
die Offsets unterscheiden sich.

Der ergänzende End-/Chunk-Export enthält:

- **658 Interpolator-Foreignfallbacks je Backend**, exakt gegen die vorhandenen
  tatsächlichen Root-Endwerte; frische Sessions und Reverseorder ebenfalls
  geprüft. Die bekannten 266 abweichenden Paare bleiben erhalten.
- **42 entfernte End-Corner-/Slice-/Lerpfälle je Backend**; 21 unterscheiden
  sich. Die tatsächlichen Vanilla-NaN-Bits bleiben als Referenzdaten erhalten.
- **84 vollständige tatsächliche Vanilla-Chunks** gegen die eingefrorenen
  Paperbits: 5.701.632 Werte, 4.023.840 unterschiedliche Bits, kein nichtendlicher
  Vanilla-Wert in dieser nahen Matrix; alle 84 Chunkhashes unterscheiden sich.
  Vanillaarrays bleiben private Vergleichspuffer; ausgegeben werden Hashes,
  Counts, Counter, maximale endliche Differenz und Beispiele.

## Zwei konkrete Vertragskorrekturen

1. `values[this.arrayIndex++] = function.compute(this)` erhöht den Index
   **vor** dem Childcompute. Tatsächliches Vanilla und Paper geben im
   128er Cell-direct-fill Childindices **1..128** aus; der Scalarcounter
   bleibt konstant. Cell-forIndex und Sliceprovider sind getrennte Verträge.
   Die ursprüngliche Proposalformulierung „compute vor Postincrement“ war
   falsch; der Worldautor wurde informiert und korrigierte seinen Consumer.
2. Rekonstruierte Base/Applied-Quellen enthalten in XoroshiroRandomSource und
   BitRandomSource Floatliterale. Tatsächliches Vanilla-Bytecode verwendet
   `l2d/dmul`, Paper Build60 `l2f/fmul/f2d`. Direkte JAR-Methoden bestätigen
   die Offset-/Numericaldrifts. Quellenbytegleichheit beweist hier keine
   Vanilla-Laufzeitgleichheit. Die private Paperbasis von .3 ist getrennt
   vom unveränderten öffentlichen Vanilla-Punktvertrag zu prüfen.

CacheOnce mit eigener 128er Arrayepoch und anschließendem Cell-direct-fill
zeigt den originalen Zugriff am Index 128 als `ArrayIndexOutOfBoundsException`.
Java-Childfehler veröffentlichen bei Cache2D/CacheOnce ihre Keys/Scalarcounter
vor dem Value. FerrumCs zusätzliche Poison-/Reset- und Refusalpolicy gehört
zur Rustprüfung. Im JSON ausgelassene `lastArray`-Felder bedeuten tatsächlich
Java `null`; leere Arrays und veröffentlichte Epochen bleiben erhalten.

## Artefakte, Pins und Reproduktion

Alle Artefakte liegen unter `.codex-tmp/gen05_3-independent-oracle/`:

| Artefakt | Inhalt |
| --- | --- |
| `NoiseChunkIndependentOracle.java`, `run_oracle.py` | Kernhelper und gehärteter Freshproducer |
| `EndAndChunkSeams.java`, `run_seams.py` | direkte Endseams und Vanilla/Paper-Vergleich ohne doppelte Paperbits |
| `semantic-mappings.json`, `source-pins.json`, `source_matrix.md` | offizielle Symbolprojektion, 63 Source-/Patchevidencepins und konkrete Callee-/Abweichungsmatrix |
| `final-normal/`, `final-optimized/` | vollständige Kernfixtures samt identischen Manifesten |
| `seams-normal/`, `seams-optimized/` | End-/Chunkvergleichsfixtures samt identischen Manifesten |
| `compact-final-normal/`, `compact-final-optimized/` | verlustfrei gepackte Constructorvisits; 24 Klassen, 4.794.097 Bytes Constructor-only je Backend |
| `negative-normal/`, `negative-optimized/` | je 27 Bundle-/Inner-/Library-/Mapping-/Source-/Helper-/JDK-/Shadow-/Env-/actual-CodeSource-Refusals |
| `validation.json`, `final.freeze.json` | abschließende Konsistenzprüfung und Filefreeze |

Compactvisits sind
`[keyclassid,equal_partition,key_identity,value_identity,valueclassid]`,
mit `classes.json` als Dictionary. Der Roundtrip wurde vollständig gegen
Rawvisits geprüft. Java-IDs unveränderlicher Records sind Diagnosen; sie
begründen keine Rust-Objektidentitätspflicht. Mutable Wrapper-Sharing,
Equals-Partitionen und Registrierungsreihenfolge bleiben erhalten.

Normal und Python `-O` erzeugen **bytegleiche Kern-, End- und Compactfixtures
einschließlich ihrer Manifeste**. Die 27 Negativguards verhalten sich ebenfalls
gleich; ausschließlich die absoluten Pfade der eigenen frischen Shadow-Test-
JARs werden beim Fehlermeldungsvergleich ersetzt. Werte, Counts, Requestorder,
Exceptions und Cachezustände werden nicht normalisiert.

Vanilla-Bundle/Inner/43 Libraries sowie Paper-Bundle/Inner/119 Libraries,
offizielle Mappings, Weltgenquelle, relevante Base-/Applied-/Patchquellen,
JDK-Executables/Module und Helpers werden gehasht. Verwendete Klassenursprünge
werden gegen die jeweiligen Inner-/Bibliotheks-JARs geprüft; falsche Herkunft
verweigert vor Fixtureveröffentlichung. Keine Python-asserts sichern Pins.

Neue Ausgabeordner müssen fehlen:

```sh
python3 .codex-tmp/gen05_3-independent-oracle/run_oracle.py /absolute/fresh/core
python3 -O .codex-tmp/gen05_3-independent-oracle/run_oracle.py /absolute/fresh/core-optimized
python3 .codex-tmp/gen05_3-independent-oracle/run_seams.py /absolute/fresh/seams
python3 -O .codex-tmp/gen05_3-independent-oracle/run_seams.py /absolute/fresh/seams-optimized
```

## Getrennte Runtimeabnahme

Dieser Agent änderte ausschließlich seinen Scratchbereich und diesen Brief.
Rustimplementation, gemeinsame GEN05-Dokumente, Dependencies, Lockfiles,
Git/Branches und das eingefrorene SRV-05.1-Paket blieben außerhalb seiner
Schreibownership. Worldautor und Root übernehmen Ressourcen-/Meter-/Finite-/
Position-/Failure-/Poison-/Reset-/Publicationabnahme und Wholecratechecks.

Aquifer/Ore/Materialblockausgabe, Biome/Surface/Bedrock, Carvers/Features/
Structures, natürliche Generatoridentität, aktive Adoption, Licht, Survival,
Clients und Java-/Paper-Plugins bleiben eigene offene Tickets. Diese
Numericalbelege enthalten keine RAM-/TPS-/Spielbarkeitsbehauptung.
