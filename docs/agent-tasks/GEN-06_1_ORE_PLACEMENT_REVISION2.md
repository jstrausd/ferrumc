# GEN-06.1 — Erzplatzierung, Revision 2 vor writerfreier Primitive-Runtime

Autorenkorrektur vom 12.10.2026 zur [vollständigen ursprünglichen Workorder](GEN-06_1_ORE_PLACEMENT_TASK.md)
und [bindenden Revision-2-Workorder](GEN-06_1_ORE_PLACEMENT_REVISION2_TASK.md).
**Nur Vorbereitung; neuer unabhängiger Korrekturreview vor jeder Runtimevergabe.**
Der ursprüngliche Vertrag und das unabhängige BLOCKING_CONTRACT-Review bleiben
readonly. Diese Revision beantwortet CR61-B1/B2/B3 konkret und trägt H1/H2;
sie erklärt sich nicht selbst CLEAN_CONTRACT.

Rootbasis: aktuelles angenommenes Main `c46077e1` (docs-only), angenommene Worldbytes
`a912b1c2`. Alle324 Dateien/Membership der akzeptierten Worldfreeze werden unabhängig
vor/nach gehasht; Mutable Storage und private Simcandidates sind keine Baseline.
GEN-05.11 bleibt unangenommen; Carverrevision gehört einem anderen Owner.
Geschrieben werden ausschließlich dieser neue Vertrag und `.codex-tmp/gen06_1-r2/**`.
Keine Gitaktion, Dependency, Produktion, weitere Agenten oder alte Freezeänderung.

| Readonly Bindung | SHA-256 |
| --- | --- |
| Originalvertrag | `a12b58d1bffbfd0ad918afe624fec7f32cefcc117d4a63a31c7ba431ab46b082` |
| Autorenfreeze (1.147 Dateien) | `720c775e6defd843996de0229687a9fe9882fe3740ba0921368812f0ab73a269` |
| unabhängiger Reviewbericht | `48737c693cf6bdcb6db68e62e069a7df9cd47d82591b5b109fb0357c90bc59a0` |
| Reviewfreeze (164 Artefakte plus Bericht, 108.222.925 B) | `336a8e7910acabfde0c7afcf29e70d84cb672b937f306b7df5652d231dfbb0fe` |

Die1.728 Autoreninputpins, vollständigen190 Source-/Patchkopien,894 Ressourcenbytes,
454 JDK-Dateien und accepted Worldpins bleiben bindend und readonly. GEN-05.11s
historische Integrität wird geprüft, nicht als Runtimeauthority übernommen.
AGENTS/CLAUDE/PAPER_REFERENCE und Worldarchitektur/Producerpflichten gelten unverändert.

Kennzeichnung: **Quellenbefund**=vollständige relevante Originalmethoden/Callees aus
Base/Applied und beiden Patchschichten; **Originalprobe**=echte gepinnte JARmethoden;
**kontrollierter Input**=deklariert, kein generierter Chunk; **FerrumC-Policy**=neuer
bounded Entwurf, keine Originalparität von Refusals. Source-/Resourcepfade in §§1/8
bezeichnen historische readonly `.codex-tmp/gen06_1/`-Inputs. Neue Ausgaben und genaue
finale Names stehen in §13; diese werden separat eingefroren. Zur Selbstständigkeit
enthält die Revision alle originalen Ressourcen-, Algorithmus-, Producer-/Whole-
FEATURES- und späteren Abnahmeobligationen, mit den genannten Korrekturen.

## 1. Originale, geschlossene Ressourcen und Quellenpins

Minecraft Java 1.21.8/Protokoll 772; Paper Build 60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`.
Vanilla-Inner `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`,
Paper-Inner `c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`.
Der vollständige vorhandene JDK 21.0.12.1+1, offizielle Mappings, Runtime-JARs,
Bibliotheken und Werkzeugquellen werden vor/nach jedem Export gehasht.

`sources/source-map.json` pinnt 190 vollständige Quelldateien einschließlich
Base/Applied, vorhandener Source-Patches und der ganzen relevanten Feature-Patches
0004 Anti-Xray, 0012 Bit-Inlining und 0016 Moonrise. `sources/resource-map.json`
pinnt 894 ZIP-Ressourcenbytes: beide Originale, 54 Overworld-Biome, alle 164 von ihnen
referenzierten PlacedFeatures, sämtliche eingebauten ConfiguredFeatures einschließlich
vorhergehender/nachfolgender Dekorationen, die benutzte Tagclosure und Preset/Parameterliste.
Kein Patchausschnitt wird als vollständige Servermethode ausgegeben. Die Patchdateien
sind ganz gepinnt; gelesen wurden die in `feature_patch_targets` benannten Dateihunks,
keine Behauptung über jede andere Mechanik dieser großen Patchdateien.

Vollständig geprüft wurden die betroffenen Methoden von `ChunkGenerator` (Konstruktor,
FeatureSorter-Zugang und gesamte Decoration-Methode samt Paper-Wrapper), `FeatureSorter`,
`WorldgenRandom`, `BitRandomSource`, `XoroshiroRandomSource`, `PlacedFeature`,
`ConfiguredFeature`, `Feature.place`/Nachbarprüfung, `PlacementContext`, allen fünf
verwendeten Modifiern samt Eltern, beiden Höhenprovidern, `VerticalAnchor`, beiden
Countprovidern, `FeatureConfiguration`/`LevelWriter`-Defaults, `OreConfiguration`, `OreFeature` (einschließlich `doPlace`, Ziel- und
Luftprüfung), `ScatteredOreFeature`, Tag-/BlockMatchTest, BulkSectionAccess,
WorldGenRegion-Lesen/Schreiben, Section-Write, ProtoChunk-Write, Heightmap-Prime/Update,
FEATURES-Pyramid/Task sowie den benutzten Tag- und Datapack-Ladepfaden. Die vollständigen
Bootstrapdateien `OrePlacements`/`OreFeatures` dienen der Kontrolle; die Inventur kommt
von geladenen Originalressourcen, nicht aus einer geratenen Erzliste.

`sources/inventory.json` enthält den vollständigen maschinenlesbaren Vertrag: jeden Key,
Configured-Key, Größe, Discard-Float, geordnete Zielzustände/Properties, komplette
Modifierliste, alle Biommitgliedschaften mit Decoration-Step und biomlokalem Index,
drei expandierte Tags und die endliche Typclosure. Entdeckung: alle Originalbiome mit
`minecraft:cave`; unabhängig bestätigt durch die **54 tatsächlichen possibleBiomes des
original geladenen normal-Generators**. Es wurden alle verwendeten ConfiguredFeature-Typen
untersucht, nicht nur ein `ore_`-Namenspräfix ausgewählt.

Ergebnis: **30 PlacedFeatures → 24 unterschiedliche OreConfiguration, 25 Zielzustände**.
Alle 30 dispatchen `minecraft:ore`; alle Ziele verwenden `minecraft:tag_match`.
Gemeinsame Kette: Count oder Rarity → InSquare → HeightRange → BiomeFilter.
Closure: fünf Modifiertypen; ConstantInt/UniformInt; UniformHeight/TrapezoidHeight
(Plateau 0); absolute/above_bottom/below_top. Kein UniformFloat-Provider in dieser
Overworld-Closure. BlockMatchTest ist untersucht, aber kein konsumierter Overworld-Zieltest.

`OreFeature` platziert auch Nicht-Erzablagerungen: dirt, gravel, granite, diorite,
andesite, tuff, clay und infested. Sie sind Teil der originalen Reihenfolge und dürfen
nicht übersprungen werden. GEN-05.4s dichtegesteuerte OreVeinifier-Adern sind ein
anderer früherer Materialproduzent. **ScatteredOreFeature ist hier kein Overworld-
Consumer**: die tatsächlichen Ancient-Debris-Konfigurationen gehören zum Nether;
seine Quellen und kontrollierten Achsenproben bleiben separat, ohne Netheraktivierung.

## 2. Vollständige Inventur

Alle Namen haben Namespace `minecraft:`. Die folgende Tabelle ist mechanisch aus den
Originalressourcen erzeugt. `U` = inklusive UniformHeight, `T` = inklusive TrapezoidHeight
mit Plateau 0. Höhenanker sind für minY −64/genDepth 384 aufgelöst; außerhalb der
Bauhöhe liegende Originalhöhen werden **nicht** vorzeitig geklemmt. `C` = konstanter
Count; `CU` = UniformInt inklusive; `R` = RarityFilter chance.

| Placed-Key | Configured-Key | Step/Index normal | Count/Rarity | Höhe | Größe | Discard |
| --- | --- | --- | --- | --- | --- | --- |
| ore_dirt | ore_dirt | 6/0 | C7 | U[0,160] | 33 | 0.0 |
| ore_gravel | ore_gravel | 6/1 | C14 | U[-64,319] | 33 | 0.0 |
| ore_granite_upper | ore_granite | 6/2 | R6 | U[64,128] | 64 | 0.0 |
| ore_granite_lower | ore_granite | 6/3 | C2 | U[0,60] | 64 | 0.0 |
| ore_diorite_upper | ore_diorite | 6/4 | R6 | U[64,128] | 64 | 0.0 |
| ore_diorite_lower | ore_diorite | 6/5 | C2 | U[0,60] | 64 | 0.0 |
| ore_andesite_upper | ore_andesite | 6/6 | R6 | U[64,128] | 64 | 0.0 |
| ore_andesite_lower | ore_andesite | 6/7 | C2 | U[0,60] | 64 | 0.0 |
| ore_tuff | ore_tuff | 6/8 | C2 | U[-64,0] | 64 | 0.0 |
| ore_coal_upper | ore_coal | 6/9 | C30 | U[136,319] | 17 | 0.0 |
| ore_coal_lower | ore_coal_buried | 6/10 | C20 | T[0,192] | 17 | 0.5 |
| ore_iron_upper | ore_iron | 6/11 | C90 | T[80,384] | 9 | 0.0 |
| ore_iron_middle | ore_iron | 6/12 | C10 | T[-24,56] | 9 | 0.0 |
| ore_iron_small | ore_iron_small | 6/13 | C10 | U[-64,72] | 4 | 0.0 |
| ore_gold | ore_gold_buried | 6/14 | C4 | T[-64,32] | 9 | 0.5 |
| ore_gold_lower | ore_gold_buried | 6/15 | CU0..1 | U[-64,-48] | 9 | 0.5 |
| ore_redstone | ore_redstone | 6/16 | C4 | U[-64,15] | 8 | 0.0 |
| ore_redstone_lower | ore_redstone | 6/17 | C8 | T[-96,-32] | 8 | 0.0 |
| ore_diamond | ore_diamond_small | 6/18 | C7 | T[-144,16] | 4 | 0.5 |
| ore_diamond_medium | ore_diamond_medium | 6/19 | C2 | U[-64,-4] | 8 | 0.5 |
| ore_diamond_large | ore_diamond_large | 6/20 | R9 | T[-144,16] | 12 | 0.7 |
| ore_diamond_buried | ore_diamond_buried | 6/21 | C4 | T[-144,16] | 8 | 1.0 |
| ore_lapis | ore_lapis | 6/22 | C2 | T[-32,32] | 7 | 0.0 |
| ore_lapis_buried | ore_lapis_buried | 6/23 | C4 | U[-64,64] | 7 | 1.0 |
| ore_copper_large | ore_copper_large | 6/24 | C16 | T[-16,112] | 20 | 0.0 |
| ore_copper | ore_copper_small | 6/25 | C16 | T[-16,112] | 10 | 0.0 |
| ore_clay | ore_clay | 6/27 | C46 | U[-64,256] | 33 | 0.0 |
| ore_gold_extra | ore_gold | 6/28 | C50 | U[32,256] | 9 | 0.0 |
| ore_emerald | ore_emerald | 6/33 | C100 | T[-16,480] | 3 | 0.0 |
| ore_infested | ore_infested | 7/4 | C14 | U[-64,63] | 9 | 0.0 |

Zielgruppen, jeweils in genau dieser Reihenfolge: metallische Erze einschließlich
coal/emerald/infested nutzen zuerst stone_ore_replaceables → gewöhnlicher Zielzustand,
danach deepslate_ore_replaceables → Deepslate-Zielzustand. Nicht-Erzablagerungen nutzen
base_stone_overworld → ihren gleichnamigen Zustand. `redstone_ore` und
`deepslate_redstone_ore` tragen explizit `lit=false`; `infested_deepslate` explizit
`axis=y`. Die genauen 25 Zustandsdefinitionen stehen verlustlos in der Inventur;
Namensheuristik oder eine zusätzliche Deepslate-Konvertierung sind verboten.

Tagclosure, originale Blöcke in Ressourcenreihenfolge:

| Tag | Mitglieder |
| --- | --- |
| base_stone_overworld | stone, granite, diorite, andesite, tuff, deepslate |
| stone_ore_replaceables | stone, granite, diorite, andesite |
| deepslate_ore_replaceables | deepslate, tuff |

`TagMatchTest` testet Blockmitgliedschaft, alle gültigen Properties desselben Blocks;
es zieht nichts. Der erste passende Zieltest gewinnt nur dann, wenn auch die Luftregel
zulässt; schlägt die Luftregel fehl, wird das nächste Ziel gegen den **vor dem Ziel-
Loop gelesenen** Zustand getestet. Ein ersetzter Block kann identisch zum Ziel sein:
Original zählt den erfolgreichen Section-Write trotzdem; `true` ist kein Netto-Delta.


Vollständige25 Zielzustandsdefinitionen (fehlende Properties erben originale
Defaults, keine zusätzliche Konversion):

| State | explizite Properties |
| --- | --- |
| minecraft:andesite | keine |
| minecraft:clay | keine |
| minecraft:coal_ore | keine |
| minecraft:copper_ore | keine |
| minecraft:deepslate_coal_ore | keine |
| minecraft:deepslate_copper_ore | keine |
| minecraft:deepslate_diamond_ore | keine |
| minecraft:deepslate_emerald_ore | keine |
| minecraft:deepslate_gold_ore | keine |
| minecraft:deepslate_iron_ore | keine |
| minecraft:deepslate_lapis_ore | keine |
| minecraft:deepslate_redstone_ore | lit=false |
| minecraft:diamond_ore | keine |
| minecraft:diorite | keine |
| minecraft:dirt | keine |
| minecraft:emerald_ore | keine |
| minecraft:gold_ore | keine |
| minecraft:granite | keine |
| minecraft:gravel | keine |
| minecraft:infested_deepslate | axis=y |
| minecraft:infested_stone | keine |
| minecraft:iron_ore | keine |
| minecraft:lapis_ore | keine |
| minecraft:redstone_ore | lit=false |
| minecraft:tuff | keine |

## 3. Decoration-Reihenfolge, Biome und RNG — CR61-B2 korrigiert

`ChunkGenerator` memoisiert `FeatureSorter.buildFeaturesPerStep(List.copyOf(
biomeSource.possibleBiomes()), generationSettingsGetter.features, true)`. Der Sorter
assigniert Discovery-IDs über **equals/hashCode** (`Object2IntOpenHashMap`), nicht Objektidentität, baut einen durch Biomlisten bestimmten
Graph über (Step, discovery ID), führt DFS mit geordneter Adjazenz aus, reversiert das
Ergebnis und baut **pro Step** eine neue Identity-Indexmapping-Liste. Zyklen werfen
`IllegalStateException`, gegebenenfalls mit den beteiligten Quellen. Alphabetische
Sortierung, lokaler Biomindex oder eine auf Erze reduzierte Graphliste sind falsch.
Der Originalexport enthält alle 164 Step/Index-Zeilen und bestätigt normal = large_biomes
= amplified. Andere Sources/Packs brauchen einen neuen Indexvertrag.
Die genuine distinct-equal-record-Probe akzeptiert den Clone in `hasFeature`;
die originale Sorterprobe mit beiden gleichen Records im selben Step erzeugt
eine Equality-Selbstkante und `Feature order cycle found`. Erst der spätere
`Util.createIndexIdentityLookup` ist eine Identitätsmap. Die komplette globale
164er-Tabelle ist verbindlich, ohne ore-only-, alphabetischen oder biomlokalen Sorter.

FEATURES verarbeitet Decoration-Ordinals 0..10: RAW_GENERATION, LAKES,
LOCAL_MODIFICATIONS, UNDERGROUND_STRUCTURES, SURFACE_STRUCTURES, STRONGHOLDS,
UNDERGROUND_ORES (6), UNDERGROUND_DECORATION (7), FLUID_SPRINGS, VEGETAL_DECORATION,
TOP_LAYER_MODIFICATION. In Step 6 stehen **underwater_magma Index 26 vor ore_clay 27**
und **disk_grass/sand/clay/gravel Indizes 29..32 vor ore_emerald 33**. ore_infested
liegt in Step 7; seine tatsächliche Indexzelle steht in der Tabelle. Diese weiteren
Features sind keine ordinary OreConfiguration, ihre Mutation vor einem späteren Erz
bleibt dennoch kausal. Die vollständigen vorhergehenden Steps gehören zur späteren
FEATURES-Abnahme.

`applyBiomeDecoration`/Paper `addVanillaDecorations`:

1. Bei debugVoidTerrain kein Vanilla-Decorationlauf. Ansonsten `SectionPos.of(chunk,
   level.minSectionY).origin()`: (16cx, −64, 16cz), keine Chunkmitte.
2. Strukturregistry in Listen nach Decoration-Ordinal gruppieren; originale Reihenfolge
   innerhalb jeder Liste erhalten. `WorldgenRandom(new XoroshiroRandomSource(uniqueSeed))`;
   der uniqueSeed ist vor tatsächlichen Draws durch `setDecorationSeed` ersetzt.
3. Biommenge aus allen Sections der **3×3** Chunks in `ChunkPos.rangeClosed` sammeln,
   dann `retainAll(possibleBiomes)`. Das sind gespeicherte Biome, keine ersetzende
   direkte Source-Abfrage. Pro Step Strukturplacements zuerst (falls enabled), mit
   eigenem Index ab 0; danach Featureindizes aus allen aufgenommenen Biomlisten
   vereinigen, deduplizieren und numerisch aufsteigend sortieren.
4. Vor jedem ausgewählten PlacedFeature RNG neu seeden und
   `placeWithBiomeCheck(region, generator, random, origin)` ausführen. Kein kumulativer
   RNG-Drawoffset früherer Features; deren **Block-/Heightmap-/Biomkontext** bleibt relevant.
5. `setCurrentlyGenerating` steht vor jeder Ausführung; nach Erfolg aller Steps null.
   Featureausnahmen werden als `ReportedException` „Feature placement“, außen nochmals
   „Biome decoration“ mit Chunk/Decorationseed eingepackt. Originalwrites werden nicht
   transaktional zurückgerollt. Bei Fehler bleibt der aktuelle Generierungssupplier gesetzt.

Seeding mit Java-long-Wrap: `setDecorationSeed(s,bx,bz)` setzt s, zieht zwei originale
Wrapper-nextLong a/b, macht beide per `|1` ungerade, setzt und liefert
`(bx*a + bz*b) ^ s`. `setFeatureSeed(deco,index,step)` setzt
`deco + index + (int)(10000*step)`, hier endliche kleine Steps.

**Der Decoration-RNG ist kein direkter XoroshiroRandomSource.** WorldgenRandom erbt
BitRandomSource/Legacy-Methoden und delegiert jedes `next(bits)` an einen neuen
Xoroshiro-Corelong `>>> (64−bits)`. nextInt(n) nutzt den Java-31-bit-Pow2-/Modulo-
Rejectionpfad; nextLong verbraucht **zwei** Corelongs, signed High/Low-ints, kombiniert
`((long)high<<32)+low`; nextFloat einen Corelong/24 Bits; nextDouble zwei Corelongs/
26+27 Bits. `getCount` zählt tatsächliche next(bits), nicht öffentliche Drawmethoden.
Das bestehende FerrumC `random::WorldgenRandomSource::xoroshiro` implementiert den
**direkten** RNG (Lemire-bounded-int, ein Corelong pro long/double) und ist daher kein
fertiger Decoration-Adapter. Öffentliche GEN-04-Methoden bleiben unverändert; ein neuer
privater metered Wrapper muss diese Semantik eigenständig bereitstellen.

Vanilla nextDouble widenet 53 Bits nach f64; Paper-Build60-Bytecode rundet über
f32-Multiplikation und widenet danach. Quellen allein zeigen das hier nicht vollständig.
Die echte Maximal-Corebit-Probe liefert Vanilla `3fefffffffffffff`, Paper
**`3ff0000000000000` = 1.0**; zwei Corelongs. Somit darf keine geometrische oder
Admission-Schranke `Paper nextDouble < 1` voraussetzen. Die normalen Originaldraws und
beide Modi bleiben in der Fixture getrennt, auch wenn Writes zufällig gleich sind.

Paper-Spezialfälle (Quellenbefund, keine Serverprobe): `featureSeeds.features` ist eine
Reference2LongMap über **Holder<ConfiguredFeature>**, default −1, random generation
standardmäßig false. Ein anderer Wert ersetzt nur für dieses PlacedFeature den
Decorationseed durch `setDecorationSeed(configSeed,bx,bz)`; danach normaler Featureseed.
Mehrere PlacedFeatures desselben Configured-Holders teilen diese Konfigurationszuordnung.
Nach Vanilla-Decorations ruft Paper jeden Bukkit-Populator mit frischem **Legacy**-Wrapper,
Decorationseed über Chunk-x/z (keine Block-x/z), CraftLimitedRegion auf; danach
saveEntities/breakLink. Selbst `addVanillaDecorations=false` überspringt diesen
Populator-Nachlauf nicht. Dafür gibt es hier keinen ausgeführten Bridge-/Eventvertrag.

## 4. Lazy Placement und Höhensemantik

PlacedFeature baut die Modifierkette als geordnete `Stream.flatMap` auf und konsumiert
sie vollständig via `forEach`; mutableBoolean ORt Erfolge, kein Abbruch beim ersten
Treffer. **Keine Candidate-Vorberechnung:** nach Count/Rarity wird Candidate 1 durch
Square/Height/Biome und den vollständigen Algorithmus gefahren, erst danach Candidate 2.
Algorithmusdraws verändern deshalb die Position des nächsten Attempts.

- Count: `count.sample(random)` genau einmal pro ursprünglichem Origin. Constant ohne
  Draw; UniformInt(0,1) mit einem bounded-int-Draw einschließlich Rejections. Repeating
  liefert dieselbe Originreferenz count-mal; zero count keine nachfolgenden Draws.
- Rarity: exakt ein nextFloat, Zulassung streng `< 1.0f/chance`. Bei Ablehnung keine
  Square-, Height-, Biome- oder Algorithmusarbeit.
- InSquare: nextInt(16) zuerst X, danach Z; Y bleibt. Addition Java-i32.
- HeightRange ersetzt Y durch einen echten Providerwert, keine Höhenklemme.
  WorldGenerationContext: minGenY=max(level.minY,generator.minY),
  genDepth=min(level.height,generator.genDepth). AboveBottom=minGenY+offset;
  BelowTop=genDepth−1+minGenY−offset; Absolute unverändert.
- UniformHeight inklusive [lo,hi], `lo+nextInt(hi−lo+1)`. Bei lo>hi Originalwarnung
  (Uniform einmal pro Paar), Rückgabe lo ohne Draw; keine stille Normalisierung.
- TrapezoidHeight: d=hi−lo. Plateau≥d → Uniform; sonst b=(d−plateau)/2 (Java-int),
  a=d−b; `lo+nextInt(a+1)+nextInt(b+1)` in genau dieser Reihenfolge. Bei lo>hi
  Warnung und lo ohne Draw. Alle eingebauten ore-Plateaus sind 0.
- BiomeFilter holt zuerst `topFeature` oder wirft die originale IllegalStateException;
  dann `region.getBiome(candidate)` und `generator.getBiomeGenerationSettings(holder).
  hasFeature(canonicalPlacedFeature) als **Equalitymitgliedschaft** des PlacedFeature-records`. Keine RNG-Ziehung. `PlacedFeature.place` liefert
  Optional.empty und ist deshalb bei dieser Kette kein Ersatz für placeWithBiomeCheck.

Die Feature-Biome kommen aus **WorldGenRegions gespeicherter, gezoomter Quelle**;
der im Carver verwendete `withDifferentSource`-Manager geht hier nicht weiter.
GEN-05.8s `SurfaceBiomeResolver` beschränkt Queries auf die Centercolumn und ist deshalb
für allgemeine Nachbar-Candidates kein fertiger Consumer; sein StoredBiomeNeighborhood
ist die mögliche zugrundeliegende Leseprimitive. Übernahme muss die genaue selected-quart-
Halo-/Yclamp-/Zoomsemantik beweisen. Biomsammlung und Filteraufrufe dürfen weder
weggelassen noch durch direkte RTree-Samples ersetzt werden. Der Reviewhinweis zu
Carver-RTree-Historie bleibt offen und wird hier nicht als Äquivalenzannahme übernommen.

ConfiguredFeature delegiert an Feature.place(config,...); dessen ensureCanWrite(origin)
steht **vor** FeaturePlaceContext/Algorithmusdraws. Ein BiomeFilter-pass garantiert
keine erfolgreiche Erzplatzierung.

## 5. OreFeature: Geometrie, Prüfungen und Writes

Originalarithmetik bleibt Java-f32 an Floatoperationen, f64 an Doubleoperationen;
keine Reassoziation, FMA oder ersetzende Math.sin-Tabelle am ersten Winkel. `Mth.sin`
innerhalb der Sphären nutzt dagegen seine originale gepinnte Lookupsemantik.

1. Draws: f32-Winkel `nextFloat()*PI_f32`; f1=size/8f; c=ceil((size/16f*2f+1f)/2f).
   Enden X=originX±Math.sin(angle)*f1, Z=originZ±Math.cos(angle)*f1.
   Enden Y=originY+nextInt(3)−2, zweimal nacheinander. Boxstart
   X/Z=origin−ceil(f1)−c, Y=originY−2−c; width=2*(ceil(f1)+c), height=2*(2+c).
2. OceanFloor-Gate: X außen, Z innen, beide Boxintervalle **inklusive** bis start+width.
   Beim ersten `boxMinY <= region.getHeight(OCEAN_FLOOR_WG,x,z)` direkt doPlace;
   ohne Treffer false und keine nextDouble-/Zielarbeit. Region.getHeight ist
   Chunk.getHeight+1, nicht dessen highest-taken-Y. Gate bei einem Nachbarn ist relevant.
3. Scratch: BitSet(width*height*width), size*4 doubles. Für i=0..size−1:
   f=(float)i/size; lerp X/Y/Z; Draw d3=`nextDouble()*size/16.0`;
   radius=`((Mth.sin(PI_f32*f)+1f)*d3+1.0)/2.0`.
   Alle size Draws kommen **vor** Culling und Blockiteration.
4. Paarculling i außen, j>i innen: nur positive Radien vergleichen;
   (ri−rj)^2 > dx²+dy²+dz² entfernt die kleinere Sphäre mit Radius −1.
   Gleichheit entfernt nichts. Wird i entfernt, läuft sein innerer Loop original weiter.
5. Sphären in i-Reihenfolge, negative Radien überspringen. Minima max(floor(center−r),
   Boxstart), Maxima max(floor(center+r), Minimum); keine zusätzliche Box-Maxklemme.
   Schleifen X außen, Y innen, Z innerst, jeweils aufsteigend inklusive. Zentren der
   Blocktests `(coord+0.5−center)/r`; alle Abstandstests strikt `<1`. Bauhöhe wird vor
   BitSetmarkierung geprüft. Index `(x−startX)+(y−startY)*width+(z−startZ)*width*height`.
6. Bitset pro Attempt, nicht Feature/Chunk: beim ersten Besuch Bit **vor** ensureCanWrite
   setzen. Danach Section via BulkSectionAccess; null Section → nichts. Aktuellen
   Blockstate genau einmal lesen, Zieltests in Konfigurationsreihenfolge.
7. `canPlaceOre`: zuerst Target.test; bei false kein Discarddraw/Neighborread.
   chance≤0 → Luftcheck überspringen ohne Draw; chance≥1 → Luftcheck immer, ohne Draw;
   sonst nextFloat≥chance → überspringen, darunter Luftcheck. Luft bedeutet `state.isAir`,
   nicht Nicht-Solidität; Wasser ist keine Luft. Nachbarn DOWN, UP, NORTH, SOUTH, WEST,
   EAST; beim ersten Luftstate abbrechen. Ein lebender Zugriff sieht vorangegangene Writes.
8. Erfolgreiches Ziel: **direkt** `LevelChunkSection.setBlockState(localX,Y,Z,state,false)`;
   counter++, break Ziel-Loop. Am Ende return counter>0. BulkSectionAccess acquire/release
   aller tatsächlich berührten Sections im try-with-resources, auch bei Exception.

Keine WorldGenRegion.setBlock-/ProtoChunk.setBlockState-Aufrufe für diese Writes:
Sectionpalette und nonempty/random-ticking/fluid-Counter werden gepflegt, Paper zusätzlich
seine Sectionlisten/Kollisionszähler. **Keine Heightmap.update, POI, BlockEntity-Removal,
Postprocessingliste oder Nachbarphysik durch OreFeature.** Für die geschlossene gültige
Target-/Replaceable-Gruppe sind States solide, ohne Fluid/BlockEntity; dass die Top-
Prädikate unverändert bleiben, muss für jeden eingebundenen Zustand geprüft werden,
nicht über `is_solid_cube` geraten. Eine spätere allgemeine Feature-Schreibprimitive darf
nicht alle Algorithmuswrites über denselben ProtoChunk-Pfad vereinheitlichen.

BulkSectionAccess: Sectionindex gegen Bauhöhe; außerhalb AIR (nicht VOID_AIR).
Innerhalb getChunk(blockX>>4,blockZ>>4), Section acquire und lokales &15. Es cached eine
LastSection plus Map aller erworbenen Sections; eine Luft-Nachbarprüfung kann Nachbar-
Sections zusätzlich erwerben. close gibt jedes Handle frei. Das ist nicht eine globale
Shared-World-Freigabe, sondern ein begrenzter Scope über bereits verfügbare Chunkowner.

## 6. ScatteredOreFeature und Regionstatus (gesonderter Quellenbefund)

Scattered zieht nextInt(size+1), also 0..size Attempts, und liefert **immer true**, auch
bei null Attempts/keinem Write. Pro Attempt i magnitude=min(i,7); X, Y, Z je zwei
nextFloat und `Math.round((a−b)*magnitude)` (f32; negative Halbwerte nach +∞).
Auch magnitude0 verbraucht sechs Floatdraws. Es liest state(pos), führt dieselbe
OreFeature-Luft-/Zielregel aus und ruft WorldGenLevel.setBlock(pos,target,2), ignoriert
dessen Rückgabe. Der originale dreistellige LevelWriter-Default delegiert mit
Rekursionstiefe512 an den vierstelligen Aufruf. Keine visited-Maske; gleiche Position kann wieder geprüft werden.

WorldGenRegion.ensureCanWrite: Chunk-Chebyshev gegen FEATURES-writeRadius1; bei alten
upgrading Chunks zusätzlich Generationhöhe. Außerhalb false und Originalwarnung;
Paper puffert die Warnung pro Region. setBlock setzt zuerst diesen Guard, dann
Chunk.setBlockState, gegebenenfalls Level.updatePOI, BlockEntity-Behandlung (Paper
entfernt alte Entities vor DUMMY-Neuanlage nur beim Blockwechsel), state.hasPostProcess
und flag16, danach true. Dieser Pfad unterscheidet sich beobachtbar von OreFeature.
Die Scattered-Achsenproben belegen nur echte private Offsetmethoden, keine tatsächliche
Netherregion, kein WorldGenRegion.setBlock und keine Scattered-Wholeplacement-Abnahme.

FEATURES-Pyramid: STRUCTURE_STARTS Radius8 und **CARVERS Radius1**, BlockwriteRadius1.
WorldGenRegion.getChunk prüft die direkte Dependencyliste und tatsächliche Holder-
Completion; out-of-bounds/noch nicht verfügbar wirft **ReportedException**, auch bei
requireChunk=false. Kein AIR/Plains-/leerer-Nachbarfallback.
`generateFeatures` primet am Center vor Decoration vier finale Heightmaps:
MOTION_BLOCKING, MOTION_BLOCKING_NO_LEAVES, OCEAN_FLOOR, WORLD_SURFACE. Persisted Status
während der Arbeit ist CARVERS. Danach `Blender.generateBorderTicks`; dieser echte
Task wurde hier nicht ausgeführt. OCEAN_FLOOR_WG wird vom Ore-Gate trotzdem gelesen;
WG-Maps sind keine automatisch live durch alle vorherigen FEATURES-Writes aktualisierte
Sicht. Der Runtime-/Oracle-Treiber muss die tatsächlich vorhandenen Maps erhalten,
nicht aus Endstates bequem neu berechnen.

## 7. CR61-B1: verpflichtende Konstruktion, Bytes und alleinige Sessionauthority

Alle folgenden Einheiten/Caps sind **FerrumC-Policy und überprüfbare Entwurfsziele**,
keine ausgeführte Rustarbeit, kein natives Layout und keine Stack-/Allocator-/RSSmessung.
Konstruktion und Ausführung besitzen getrennte Meter. Der Bind-Meter darf den Run-Meter
nicht ersetzen; ein Plan ist erst nach vollständiger Bindung sichtbar.

### 7.1 Vollständiger Census und exakte Quellenprofile

`binding-final-v3/census.json` wird direkt aus den originalen ZIPentries abgeleitet:
die 54 echten `possible_biome`-Rows bestimmen sämtliche 164 PlacedKeys, anschließend
wird nach tatsächlichem Configured-Typ selektiert. Die historische Inventur dient
nur dem unabhängigen Gegenvergleich. Kein `ore_`-Präfix oder Fixturezustand bestimmt
Runtimezulassung. Die ganzen 57 Ressourcen sind in beiden Originalvarianten neu gepinnt.

| Gegenstand | Unabhängig abgeleiteter Wert |
| --- | ---: |
| Placed / Configured / Tags | 30 / 24 / 3 = 57 |
| Vanilla-Quellbytes / größte Ressource | 26.930 / 720 B |
| Paper-Quellbytes / größte Ressource | 26.987 / 721 B |
| JSON-Nodes (Root, jeder Key, jeder Value) | 1.684 |
| Keys / Edges / Strings / dekodierte Stringbytes | 727 / 1.627 / 1.070 / 12.356 |
| größte Nodezahl / Tiefe / Container / String | 41 / 7 / 6 Einträge / 36 B |
| geordnete Modifier / Countprovider / Heightprovider / Anker | 120 / 26 / 30 / 60 |
| Targetreferenzen der 24 Configs / eindeutige Zielstates / explizite Properties | 41 / 25 / 3 |
| Targetreferenzen über die 30 Placed-Records | 50 (andere Zählgrundlage!) |
| Tagmitgliedsreferenzen / expandierte Statezugehörigkeiten | 12 / 16 = 8+4+4 |
| SHA-256-Kompressionsblöcke incl. Padding, Vanilla / Paper | 463 / 464 |
| ungeordnete Keypaare innerhalb JSON-Objekten | 364 |

**Bytebefund:** Jede Paper-ZIPentry ist exakt die Vanilla-ZIPentry plus ein LF.
Die JSON-Werte sind gleich, die Digests sind verschieden; kein Digest wird normalisiert.
Der verlangte Reviewcensus 26.930/720 bleibt der Vanilla-Kontrollcensus.
Produktion bindet **exakte Paperbytes** mit 26.987/721; der Paper-Decorationwrapper bleibt
das einzige Produktionsprofil. Die zusätzlichen 57 LF sind eine offengelegte
Quellprofilpräzisierung, keine Änderung der ursprünglichen Ressourcenwerte.
Vanilla ist ausschließlich der diskriminierende Testkontrollpfad.

World besitzt neue private `PlacedId`, `ConfiguredId`, `TagId` und checked Pläne.
Sie haben keine raw/string/Wire-ID-Konstruktoren. Die eindeutigen Schlüssel werden
aus den tatsächlichen Originalressourcen in World-Code übernommen und dort geprüft;
nicht aus einer Laufzeitdatei unter `tests/reference` oder `.codex-tmp` geladen.
Die 164 original exportierten globalen Indexrecords werden als vollständige immutable
World-Metadaten eingebunden und im Originalvergleich geprüft. Sie ersetzen keinen
Datapacksorter, keine Biomgenerationmembership und keine Biomezulassung.
Die 57 JSON-Bytes und ihre Hashdeskriptoren sind readonly/static; Registry-
`BlockMetadata` und Properties werden ausschließlich aus bestehenden statischen
Registrytabellen geliehen. Mutable Storage und private Simdaten sind keine Inputs.

### 7.2 Vollständiger, obligatorischer Bind-Debit

`BindAllowance::new(u32)` erlaubt **1..=1.048.576**. Ein privater `BindWork` nimmt
jeden Debit *vor* dem jeweiligen Vorgang/der Schleifeniteration ab; `try_charge(k)`
ist checked und atomar: bei Verweigerung unveränderter used-Wert und kein Vorgang.
Es gibt keinen optionalen unmetered Vorlauf, keine Nachzählung nach einer Allocation
und keinen rekursionslokalen Reset. Folgender absichtlich einfacher, fester
Debitplan ist verbindlich, bis Root nach Review einen anderen Plan explizit annimmt.
Er ist ein Arbeitsprotokoll und keine CPU-Instruktionszählung.

Vergleichsslots sind immer vollständig gepolstert: 64 Byte für Keys/Resources/Typen,
32 Byte für Propertynamen/-werte; ein Bytevergleich ein Debit, kein kostenloser
Stringhash. Abweichende Slots werden weiter verglichen, bevor das Ergebnis gewählt
wird. Schema hat höchstens vier Keys je Objekt; Uniondispatch hat die angegebenen
5/2/3 endlichen Slots. Alle Nodes und Edges werden genau einmal durch den Parser/
Validator geführt; keine ungezählte zweite generische Baumtraversierung.
Containerloop-Eintritt/Buildaktionen sind durch Node-/Edge-/Builddebits abgedeckt,
Loopende ist konstante Kontrolle. Neue inhaltsabhängige Schleifen dürfen nur mit
angepasstem und neu reviewed Debitplan hinzukommen.

| Debitphase | Exakte Ableitung | Vanilla | Paper |
| --- | --- | ---: | ---: |
| Beginn | 1 | 1 | 1 |
| zwei Allocationadmissions + zwei Release-Reservierungen | 2+2 | 4 | 4 |
| initiale scratch/staging Byteinitialisierung | 32.768 | 32.768 | 32.768 |
| Quellheader/Ordinale | 57 | 57 | 57 |
| SHA-Inputbytes | B | 26.930 | 26.987 |
| SHA-Schedule + Kompressionsrunden | 128·H | 59.264 | 59.392 |
| Digestvergleich | 57·32 | 1.824 | 1.824 |
| exakter Vergleich aller Originalbytes | B | 26.930 | 26.987 |
| JSON-Bytekonsum | B | 26.930 | 26.987 |
| Nodeadmission / Edges | 1.684+1.627 | 3.311 | 3.311 |
| Stringdekodierung | 12.356 | 12.356 | 12.356 |
| Key-/Schemaslots | 727·4·64 | 186.112 | 186.112 |
| Duplicate-/escaped-duplicate-Keypaare | 364·64 | 23.296 | 23.296 |
| Stringvalue-Klassifikation | (1.070−727)·64 | 21.952 | 21.952 |
| Configured-Referenzbindung | 30·24·64 | 46.080 | 46.080 |
| Target→Tag-Referenzbindung | 41·3·64 | 7.872 | 7.872 |
| bestehender Registry-Namelookup, reservierte obere Arbeit | (41+12)·12·64 | 40.704 | 40.704 |
| drei Propertyoverrides, reservierte obere Arbeit | 3·(32+3·32+8) | 408 | 408 |
| Modifiertypen | 120·5·64 | 38.400 | 38.400 |
| Countprovidertypen | 26·2·64 | 3.328 | 3.328 |
| Heightprovidertypen | 30·2·64 | 3.840 | 3.840 |
| Ankertypen | 60·3·64 | 11.520 | 11.520 |
| Import aller globalen Indexrecords | 164·(64+1) | 10.660 | 10.660 |
| eindeutige (step,index)- und Keyprüfung | 2·164·163/2 | 26.732 | 26.732 |
| Plan→globaler Index, checked IDs | 30·164 | 4.920 | 4.920 |
| Tagrange-/Statemitgliedschaftsbuild | 12+16 | 28 | 28 |
| typed IR-/Planbuild | 57+120+41+30+24+3 | 275 | 275 |
| garantierter Resource-Cleanup | 57·4.096 | 233.472 | 233.472 |
| finaler Scratchcleanup | 16.384 | 16.384 | 16.384 |
| Ownershiprelease + letzte Publication | 1+1 | 2 | 2 |
| Zielstate-Paarprüfung / Abschlusscensus | 41·40/2 + (57+1.684+120+41+12+164) | 2.898 | 2.898 |
| **gesamter erfolgreicher Bind-Debit N** | **Summe jeder obigen Zeile** | **869.258** | **869.557** |

Für die bestehenden Registrycalls werden die festen oberen Debits als *Gruppen*
vor dem Aufruf abgenommen, nicht als angeblich gemessene innere Calls ausgegeben.
`block_metadata` sucht allocationfrei binär in den gepinnten **1.105** Einträgen;
12 Vergleichsslots à höchstens 64 Zeichen decken die komplette binäre Suche und
Namespaceprüfung ab. Das Maximum aller Registryblocknamen ist gesondert geprüft.
Nur geladene Ziel-/Tagblöcke dürfen weiterverwendet werden. Keine vollständige
27.946-State-Scanning-Allocation in Produktion: TagMatch benutzt die exakten
Blockmetadaten/Dense-Ranges, einschließlich aller Deepslate-Properties.
Die drei expliziten Overrides haben höchstens eine Property und höchstens drei
Werte. Der bounded private Resolver berechnet den State aus default/min/cardinality;
alle tatsächlichen Property-/Value-/Arithmetikloops sind durch die obige Reservierung
begrenzt. Er darf nicht unbemerkt den allgemeinen BTreeMap-Resolver oder einen
unbegrenzten Registryconsumer ausführen. Defaults bleiben echte Registrydefaults;
`lit=false` und `axis=y` werden ausdrücklich gebunden, nie entfernt.

Digestprüfung reserviert auch die festen SHA-Padding-/Blockarbeiten; Bytekonsum,
64 Schedule-/64 Kompressionsrunden pro Block und 32 Digestbytes sind einzeln
gezählt. Diese Zählung ist eine konkrete bounded SHA-Schnittstellenanforderung,
kein Nachweis eines schon vorhandenen kostenlosen Hashers. Decoderbytes sind
neben dem Syntaxbytekonsum gezählt, inklusive escaped Keys. Feste padded Slots
werden ohne Heapstring durchgeführt. Name-/Profilediscriminator dürfen den
beschriebenen Slot-/Byteumfang nicht überschreiten.

Cleanup wird **vor** Erzeugung des jeweils aufräumpflichtigen Zustands reserviert.
Reservation zählt als verbrauchter Bind-Debit und wird nicht zurückgebucht; auch
Exhaustion kann daher den begrenzten POD-Arenacleanup ausführen. Es gibt keine
rekursive JSON-Destruktorstruktur. Speicherfreigaben sind konstante zwei
Allocatoraktionen mit vorreservierten Debits (Scratch plus Staging bei Failure
oder Scratch plus alter Katalog beim erfolgreichen Ersatz; der neue Katalog bleibt
besessen). Die Ownershipreleasewirkung des alten Katalogs erfolgt erst nach dem
letzten admitted Publicationdebit, nie bei N−1. Alle Arbeiten einschließlich Release
werden vor der letzten Publication admitted; nach Publication kann nichts mehr
fallibel sein. N−1 scheitert am letzten Publicationdebit und erhält einen etwaigen
alten Katalog unverändert. N und N+1 veröffentlichen denselben Katalog mit used=N;
es gibt keine Pflicht, zusätzliche Allowance zu verbrauchen. Für Fehlpfade gilt
die tatsächlich erreichte Prefixsumme mit bereits reserviertem Cleanup.

### 7.3 Gleichzeitige Bytes und before-growth admission

Die früheren 128-KiB/2-MiB-/Depth32-Generalparserzahlen sind **keine** Zulassung für
GEN-06.2. Es gilt die geschlossene Quelle: 57 Ressourcen, exakt erwartete Bytes und
Digests; maximal 721 B/Quelle in Paper, 720 B in Vanilla. Der private Parser hat
64 POD-Nodes/Quelle, Tiefe8, höchstens8 Containerentries und 64 dekodierte Stringbytes.
Die vorhandenen maximal41 Nodes/Tiefe7/6 Entries/36 B passen mit Reserve hinein.
Der dekodierte Stringpool hat höchstens721 B je Quelle: UTF-8-/Escape-Dekodierung
kann nicht mehr Bytes erzeugen als der bounded Originalstringinput. Alle decoded
Keys/Strings werden dort einmal gespeichert; Duplicate-/Slotvergleiche scannen
die gespeicherten Bytes, keine ungezählte erneute Dekodierung.
Nur eine Quelle und ihr AST/IR sind gleichzeitig live; kein Aggregat-AST aller57.
Jeder byte/node/string/edge-/stackframe-Zuwachs prüft arithmetisch sein Limit und
seinen Debit *vor* dem Schreiben. Fehler sind bounded enums mit typed Ordinal/Offset,
requested/max; Fehlermessages werden nicht allokiert.

| Gleichzeitiges Objekt | Besitz / Entwurfsobergrenze |
| --- | --- |
| exakte Quellbytes | static/borrowed; 26.987 B Paper bzw. 26.930 B Vanilla, separat gezählt |
| Hashdeskriptoren, Key-/164-Indextabellen, bestehende Registry | immutable static/borrowed; eigene Byteinventur bei Runtime, kein owned-Heap-Abzug |
| neuer privater Katalog/Staging einschließlich aller Pläne, Configs, Targets, Tags/IDs | **16.384 B retained**, identisches Objekt wird publiziert, kein zweiter Finalkatalog |
| Parserarena/Frames/64-B-Decoderfenster | 4.096 B Scratchpartition; 64×32-B-Nodetarget=2.048 B +721-B-Pool aller dekodierten Strings +8×32-B-Frames=256 B +64-B-Decoderfenster +512-B-Parsermetadaten =3.601 B;495 B Alignmentreserve |
| ein typed Resource-IR | 4.096 B Scratchpartition |
| Referenz-/Index-/Tag-Bindworkspace | 4.096 B Scratchpartition; keine HashMap-growth |
| SHA-/Parser-/Compilerlocals und Alignmentreserve | 4.096 B Scratchpartition |
| **gesamter simultaner neuer Compiler-/Parser-/IRscratch** | **16.384 B** |
| **neuer Stagingkatalog + Scratch gleichzeitig** | **32.768 B** |
| vorhandener alter Katalog bei fehlerschonendem Ersatz | zusätzlich höchstens16.384 B; aggregate owned peak **49.152 B** |
| Primitivecursor/RNG/Meter einschließlich Pendinglease-Metadaten | **512 B** Ziel; separate Session, kein retained Candidate-Vec |

Die vier Scratchpartitionen können in einer vorher fallibel admitted 16-KiB-Arena
liegen; der Stagingkatalog in einer weiteren festen admitted Allocation. Keine
Arena auf dem Rustcallstack als scheinbar kostenloses großes by-value-Objekt,
keine nachträgliche unfallible Box-Konvertierung oder hidden Vec-Capacitydoubling.
Vor `try_reserve_exact` werden checked elementsize×capacity, alle simultanen
Partials/Alignment und der gemeinsame Peak geprüft. Tatsächlich größere Allocator-
Capacities zählen vollständig und verweigern vor Inhaltsschreiben; `AllocationFailed`
ist typisiert. Spätere Reservierung darf niemals mehr als diese admitted capacities
fordern. Der 512-B-Cursor kann erst nach einer tatsächlichen Runtime-Lifetime-/Layout-
Messung als eingehalten bezeichnet werden. Alte live Sessions leihen ihren alten
Katalog; Rust verhindert dessen gleichzeitigen Austausch über `&mut CatalogOwner`.

**Neuer Vorschlag zur Rootprüfung:** 16-KiB-Konstruktionsscratch und 32-KiB neuer
Compilerpeak (49.152 B mit altem Katalog). Zuvor existierte kein Bind-Scratchcap;
dies ist keine gemessene Reduktion/Erhöhung. Die 16-KiB-Katalog- und 512-B-Cursorziele
bleiben erhalten. Die **8 KiB ausschließlich Geometriescratch** des späteren
Algorithmus dürfen diesen Konstruktor nicht finanzieren. Verfehlt das konkrete
Rustlayout eine Partition, darf der Autor das Ziel nicht erhöhen: Root entscheidet
zwischen Layoutverdichtung innerhalb dieser Zahlen und einem separat quantifizierten
Capänderungsentwurf mit erneuter unabhängiger Prüfung.

### 7.4 Exakte Konstruktionsverweigerung

Präzedenz in Ausführungsreihenfolge:

1. `InvalidBindAllowance` (0/max+1), danach Produktionsprofil, Settings/Source und
   ausgeschlossene Overrides/Populators/Blending; statische Header-/Count-/Ordinal-
   und byte-length-Grenzen, dann simultane Nativezieladmission. Noch kein RNG.
2. Bindbeginn und Cleanup-/Allocation-/Initialisierungsdebits; fehlende Allowance
   stoppt vor der ersten nicht admitted Aktion, Allocationfailure vor Parserbytes.
3. Ressourcen in festem Ordinal (Placed lexikalisch30, Configured lexikalisch24,
   Tags lexikalisch3; nur Bindordnung, **nicht** Decorationreihenfolge): Digest und
   exakt erwartete Originalbytes, dann Syntax/UTF-8/trailing bytes, decoded duplicate
   Keys, Schema/UnknownKey/Types/finite ranges, Referenz-/State-/Tagbindung und IRbuild.
   Je Kategorie erster Sourceordinal/Byteoffset. Bindarbeit stoppt beim ersten Fehler.
4. Vollständiger Census, exakte Typ-/Referenzclosure, 164-Indextabelle/alle Paare,
   Planindizes, finale Katalogbytes, Cleanup und letzte Publication.

`WorkBudgetExceeded` hat Vorrang vor der *nächsten* Aktion, wenn deren Debit nicht
admitted ist, nicht rückwirkend vor schon gefundenen Fehlern. Keine ungezählte
„erst alle Fehler prüfen“-Traversal. Productionbytes mit malformed Mutation liefern
bei genügender Allowance zuerst `CatalogueDrift` beim Digest, nicht einen erfundenen
Java-Codecfehler. Die gesamte Parser-/Schemafehlermatrix wird zusätzlich ausschließlich
über `cfg(test)`-Inputs in denselben privaten Parser/Compilerkern geführt; ein
Testdeskriptor darf die neue Mutation zum Parser durchlassen, er ist keine
öffentliche Quelle-, Hash- oder Planconstructorauthority. Budget versus Malformed
wird am exakt letzten admitted Byte/Node/Slot und in beiden Reihenfolgen geprüft.

Konkrete private Fehler: `InvalidBindAllowance`, `UnsupportedProfile/Settings/Source`,
`UnsupportedFeatureSeedOverride/Populators/Blending`, `CatalogueDrift`,
`MalformedDefinition` (Syntax/UTF8/DuplicateKey/UnknownKey/Type/Trailing),
`UnknownResource/Modifier/HeightProvider/CountProvider/TargetRule/Tag`,
`InvalidBlockState/Anchor/ProviderRange`, `FeatureOrderCycle/InvalidGlobalIndex`,
`NativeLimit`, `AllocationFailed`, `WorkBudgetExceeded`. Kein AIR-/Plains-/falsefallback.

### 7.5 Sealed Session, Pending und alleiniger Stream

Der folgende private Rustentwurf ist verbindlich für GEN-06.2, **noch kein Code**:

```rust
// Alle Typen/Felder/Methoden nur im privaten World-placement-Modul.
struct Session<'catalog> { /* &Catalog, Identity, Rng, RunWork, State, Cursor */ }
enum State { Fresh, Running, Pending, Complete, Poisoned }
struct Pending<'run, 'catalog> { session: Option<&'run mut Session<'catalog>> }
enum Next<'run, 'catalog> { Pending(Pending<'run, 'catalog>), Complete }
// new(&Catalog, checked Identity, PlacedId, RunAllowance) -> Result<Session>
// begin(&mut self) -> Result<()>                    // ausschließlich Fresh
// next(&mut self) -> Result<Next<'_, 'catalog>>     // ausschließlich Running
// Pending::position(&self) -> BlockPos             // kein Biomepass
// Pending::dispatch(self, private ConsumerKind) -> Result<ContinuationReceipt>
// Pending::Drop: Pendingsession immer Poisoned, wenn nicht explizit abgeschlossen.
```

`RunAllowance` erlaubt **1..=2^20 =1.048.576**, unveränderlich über die ganze Session.
Der Katalogborrow bleibt über die Session erhalten; kein fremdes Plan-/Katalogalias.
Vor Begin: vorhandener checked Katalog, Paperdefaultprofil, checked Originalkey,
zugehöriger tatsächlicher globaler step/index (nicht Callerinput), WorldSeed,
Overworld-Settings (normal/large_biomes/amplified), explizite passende MultiNoise-
Sourceidentity, minY−64/height384 und admitted Chunkorigin/Originspalte. Centerorigin
und interpolation edge16c+16 werden in i64 in ±30.000.000 geprüft; Origin=(16cx,−64,16cz).
Neue private Testidentitäten dürfen nur den gemeinsamen Kernel unter `cfg(test)`
erreichen. Identity allein beweist weder CARVERS noch Biom-/Regioncompleteness.
Alle abgeleiteten X/Z-/Anchor-/bounded-range-Domains sind vor Begin admitted; Y-
Outputs dürfen die Bauhöhe über-/unterschreiten, wie die Originalprovider.

Genau ein Sessionowner besitzt RNG **und** Meter; beide private Felder sind weder
Clone noch austausch-/exportierbar. Die Fresh-RNG-Felder sind noch kein seeded
Core; erst Begin admitted die erste Seedmutation. Es gibt keinen zweiten RNG,
externen `RandomSource`, neuen Consumerseed, Fork, Reset, SetAllowance oder public
Successcallback. Seedmutationen sind atomare bounded Aktionen ohne Draw; zwei
long-Draws des Decorationseeds bleiben zusammen vier Coremutationen.

`next` läuft lazy bis zum nächsten unentschiedenen Biomfilter. Count/Rarity nur
beim ersten Origin, dann X vor Z, dann Höhenprovider. Der vierte Modifierentry
wird beim Übergang an die Biomefortsetzung charged; seine Query ist **deferred**.
Ein Emissionsdebit setzt genau eine Position/Sequenz und `Pending`. Das ist keine
Biomeentscheidung und kein Algorithmuserfolg. Es gibt keine Candidatevorberechnung.

Die Pendinglease hält exklusiv `&mut Session`, deshalb verhindert der Borrowchecker
ein zweites `next` oder paralleles Ziehen während der realen Fortsetzung. Ihre
`dispatch(self, ConsumerKind)` ist nur für konkrete interne, sealed Consumer-
Varianten zugänglich, keine generische Closure/trait object. GEN-06.2 enthält **keine
Produktionsvariante**, die eine Biomeentscheidung oder Orewrite vortäuscht.
`cfg(test)` fügt deklarierte Original-Consumerdrawscripts hinzu und benutzt dieselbe
Lease/Wrapper-/Meter-/Stateimplementierung. Ein späterer echter .3-Consumer bekommt
im privaten Dispatch exklusiv einen reborrow des Session-RNG/Meters; er erhält keine
zweite Authority. Nur dieses interne Dispatch kann nach einem tatsächlichen Return
den Receipt und den Übergang nach Running erstellen. Receipt enthält die gleiche
Identity/Candidatesequenz; keine frei konstruierbare bool-Erfolgsmeldung des Callers.

Erfolgreicher Abschluss konsumiert die Lease, nimmt den Completiondebit ab und
setzt Running (auch nach dem letzten Candidate; Complete folgt beim nächsten lazy
Endcheck). Failure, Panicunwind, verweigerte Consumerziehung oder Drop ohne Abschluss
setzen **terminal Poisoned**. `Option::take` beendet den Borrow ohne unsafe und
verhindert doppelten Drop/Abschluss. Rust-Drop ist bounded/fallible-free; Failclosed-
Statebit und Freigabe eines Borrows sind konstante administrative Arbeit, auch wenn
Allowance vollständig verbraucht ist. Kein RNG wird dabei weitergezogen.
`mem::forget(Pending)` kann Drop umgehen: Session bleibt Pending und ist dennoch
unwiederverwendbar; jede spätere Sessionoperation verweigert und poisoniert, statt
die alte Lease zu rekonstruieren. Keine Busy-Wait-/Lifecycle-Reparatur.

| Zustand / Operation | Admission und Charge vor Wirkung | Ergebnis / Verweigerung |
| --- | --- | --- |
| Fresh→Begin | komplette Identity/Domain, dann Begin1 | Running; nach Beginfailure Poisoned |
| initiales Seeden | Seed1, vier einzelne Core1, Decoration-ReSeed1, Feature-ReSeed1 | mit Begin insgesamt **8**; keine RNGmutation bei verweigertem nächsten Charge |
| Count/Rarityentry | Modifierentry1, danach eventuell Core1/Rejections | Count0 oder Rarity-false erreicht Ende ohne Square/Height/Biome |
| Candidatevorlauf | Squareentry1; X-Core; Z-Core; Heightentry1 und seine Coreiterations | kein veröffentlichter Prefixcandidate |
| Pendingemission | Biomeentry1; Emission1 vor Pending | genau eine unentschiedene Position; kein Biomepass |
| Pending→Consumerdispatch | Dispatch1 | derselbe exclusive RNG/Meter, kein hidden Reset |
| erste / verworfene / letzte Consumer-Coreziehung | jeweils Core1 *vor* `next(bits)` | Rejection zählt als erfolgreiche Mutation; verweigerter Versuch zählt keine |
| explizite Completion | Completion1 vor Receipt/Running | exakt ein Receipt; kein Caller-bool |
| Streamende | End1 vor Complete | original kein weiterer Draw; Count0/Rarity-false Complete |
| Consumerfailure / Drop / forgotten Lease | bounded Poisonbit/Borrowcleanup, keine erfolgreiche Aktion/kein Draw | terminal; keine erneute Continue/Begin/Next-Authority |

Der Run-Debit ist somit exakt
`Begin + Seedmutationen + erfolgreiche Coremutationen + tatsächliche Modifierentries
+ Pendingemissionen + Dispatchentries + erfolgreiche Completions + Streamende`.
Alle Drawmethods inklusive nextLong/nextDouble und jeder rejected bounded-int-
Iteration verwenden denselben checked Corepfad; kein Charge nur am äußeren nextInt.
Run-`used` und `successful_core` inkrementieren erst nach admitted Charge, vor dem
infalliblen Coreupdate. Refusal verändert weder Corestate noch successful_core;
frühere erfolgreiche Draws bleiben als diagnostischer Prefix erhalten, Session poisoned.

**Java-Diagnose separat:** Original `WorldgenRandom.next(bits)` erhöht getCount vor
Delegation. Bei einer testkontrollierten verweigerten Delegation kann getCount daher
successful_core+1 sein. Die vier reviewseitigen Reject-all-Controls liefern bei
Allowance0/1/2/3 Corecounts0/1/2/3, Java-getCount1/2/3/4. Diese Exceptiondiagnose
wird erhalten, aber niemals zum erfolgreichen FerrumC-Workcount umgedeutet.

Konkrete diskriminierende geplante Workgrenzen: Begin/Seeding benötigt8; jeder der
vier Coredraws, jeder ReSeed und der erste Countentry werden bei Prefix−1/Prefix/
Prefix+1 getrennt geprüft. UniformCount0 bzw. Rarity-false benötigen nach Begin
Count/Rarityentry1+Core1+End1: **N=11**. ore_dirt mit sieben uniform Candidates und
je einem deklarierten Consumerfloat benötigt **N=80**: Begin8+Countentry1+
7·(Square1+X1+Z1+Height1+Heightcore1+Biomeentry1+Emission1+Dispatch1+Floatcore1+
Completion1)+End1. Erfolgreiche Coremutationen=4+7·4=32. Der erwartete Drawoutput
stammt aus der echten lazy Originalprobe; die 80 sind ausschließlich der neue
FerrumC-Debitplan. Trapezoid zieht die beiden Heightdraws in Originalreihenfolge.
Jede Rejection verlängert die Summe um eine tatsächliche Coremutation; keine kleine
universelle Rejectionobergrenze wird behauptet.

Der frühere spätere Vorschlag **2^32** gilt ausdrücklich **nicht für diese Session**.
Eine neue Algorithmus-/Regionkomposition braucht einen eigenen genehmigten Gesamt-
vertrag einschließlich Originalchronologie, Metertransfer/Initialseed, Provenance,
Bytes und Publication. Diese API hat keinen Upgrade-/Transferpfad. Die Prüfung
von GEN-06.2 kann einen solchen späteren Vertrag weder akzeptieren noch zulassen.

### 7.6 Aufgeschobene Geometrie-/Regionpflichten und CR61-H2

Die bisherigen strukturellen Bounds bleiben für GEN-06.3-Präparation erhalten:
size3..64; Radius≤4,5 einschließlich Paper Double1; width26/height14;
9.464 Maskbits/1.184 u64-Bytes; 64×4 Doubles=2.048 B; ≤2.016 Paarchecks;
≤729 Gatehöhen; ≤704/7.744/85.184 X/Y/Z-Besuche; höchstens2 Targets und6
Luftnachbarn; Vereinigung aller30 Pläne ≤471 Attempts. Geometrydebit524.288 pro
Attempt deckt den früheren strukturellen Teilbound380.370, **keine** gesamte
Placement-/RNG-/Bind-/FEATURES-Arbeit. Reads relativ zum Center[-14,29], Writes
[-13,28] bleiben konservativer kontrollierter 3×3-Scope. Handle27 und Geometry8KiB
bleiben ungemessene spätere Ziele; keine Nativeruntimeautorität.

**CR61-H2 explizit getragen:** neun384-hohe Statepayloads umfassen
9·98.304·4=3.538.944 B. Erhaltener Input plus private Candidatekopie leben gleichzeitig:
**7.077.888 B allein für Statepayload**. Die frühere4-MiB-Regionzahl ist zurückgenommen;
hier gibt es **keinen angenommenen neuen Regioncap**. Maps/Counter/Fluidflags,
geordnete PP-Listen incl. Duplikaten (früheres separates Ziel131.072 Einträge/512KiB),
Borrowedbiomcapacities, Input-/Resultobjekte, Kopierarbeit, Reservations und Alignment
müssen im späteren Regionvertrag gemeinsam und zusätzlich admitted werden.

World darf dort nur gegen eine private, versionsgebundene ganze3×3-Region arbeiten.
Erfolg gibt ein einziges geschlossenes Result mit neun Puffern/Deltas und gemeinsamem
Commitmarker; Failure kein Prefixresult/kein Shared-Nachbarwrite. Sim reserviert
alle neun Owner über Shardgrenzen, prüft den ganzen Versionsvektor und installiert
an einer Tickgrenze atomar; Konflikt/defer installiert nichts. Center-/Neighbororder
muss den Originalscheduler einschließlich früherer Decorations abbilden. Disjunkte
Snapshots blind parallel mischen ist unzulässig. MissingChunk/Status/Map/
Classification/VersionConflict bleiben spätere typisierte Verweigerungen, keine
STONE-/AIR-/Plainsdefaults. GEN-06.2 implementiert keinen dieser Regionpfade.


## 8. Tatsächliche Originalevidenz und Reproduzierbarkeit — CR61-B3 korrigiert

`oracle/OreOracle.java` ist ein frisch kompilierter JDK-only Reflectionhelper, keine
nachgeschriebene Erz-/Placement-/RNG-Erwartungsformel. Vanilla-Namen kommen mechanisch
aus den offiziellen Mappings. Geladene Datapack-Registries folgen WorldLoader/TagLoader/
RegistryDataLoader einschließlich echten angewandten Tags; bootstrap-only DummyTags
sind keine Testauthority. Original-/Bibliotheks-/Helper-CodeSources werden im JVM vor
Samples geprüft. Erwartete Outputs kommen ausschließlich von den tatsächlichen Methoden.

Der Hauptcorpus jedes Originals enthält:

- 54 echte possibleBiomes; je164 echte FeatureSorter-Indizes für normal, large_biomes,
  amplified, mit tatsächlich identischer Tabelle;
- 48 echte setDecorationSeed/setFeatureSeed plus Wrapper-Drawfolgen: Seeds0/−1/i64-Min/
  Max, Origin und negative/asymmetrische/Weltrand-Blockkoordinaten, Steps6/7 und Indizes
  0/33; sämtliche echte getCount-Werte;
- eine skriptgesteuerte echte bounded-int-Rejection, corelongs −1 dann0, bound3:
  Ergebnis0, getCount2; Maximal-Corebit-nextDouble mit zwei Corelongs;
- 60 echte HeightProvider-Endpunktaufrufe und52 Count-Endpunkte des geladenen Katalogs,
  jeweils deklarierte positive bounded-int-Extrema; 400 originale Zielmitgliedschaftstests
  gegen stone/granite/diorite/andesite/deepslate/tuff/dirt/air ohne RNGdraw;
- 17 echte shouldSkipAirCheck-Grenzen (0,0.5,0.7,1; draw darunter/gleich/darüber soweit
  gültiger nextFloat); 16 canPlaceOre-Kombinationen: Zielmatch/-mismatch, Stein/Wasser/
  Luft/Luft nur UP, Draw0 oder0.5, mit vollständigen geordneten Neighborreads;
- 16 echte private Scattered-Achsenergebnisse magnitude0..7, positive/negative Extreme,
  mit exakter Ziehungsanzahl (gesondert, kein Overworld-Consumer);
- **60 komplette echte placeWithBiomeCheck/OreFeature-Aufrufe**, alle30 Keys × Seeds0/−1,
  an kontrollierter Region; außerdem die echte Ausnahme bei place ohne topFeature.

**Die 60 Aufrufe sind ausdrücklich kontrolliert:** neun echte ProtoChunks/Sections,
alle Blöcke als deklarierte STONE-Eingabe durch originale Sections gefüllt, ursprünglicher
ProtoChunk-Defaultstatus unverändert, keine erfundene CARVERS-/ServerLevel-Instanz.
WorldGenLevel ist ein beschränkter Proxy, Höhe320 explizit kontrollierter Gatewert,
getBiome gibt den ersten genuine possibleBiomes-Holder mit tatsächlicher Featuremitgliedschaft
zurück. Er implementiert keine Biomselektion. Die beiden kontrollierten RNGs führen
original setDecorationSeed aus, setzen danach für diese isolierte Probe bewusst
Featureseed `seed+0+60000`, **nicht** den echten Featureindex aus der Tabelle.
Jeder Fall beginnt wieder von identischem STONE-Inhalt; jeder Netto-Write im vollständigen
3×3-Statebereich wird nach echtem Aufruf gelesen und durch originale Sectionwrites
zurückgesetzt. Drawtrace delegiert transparent an original WorldgenRandom; **öffentlich zurückgekehrte
Randomcalls**, Heightgates, ensureCanWrite, Biomqueries und Chunkrequests sind aufgezeichnet.
**Keine Statereads sind in diesen60 Traces beobachtet:0 S-Events.** Direkte
`LevelChunkSection.getBlockState`- und `BulkSectionAccess.getBlockState`-Reads
(einschließlich Luftnachbarn), Sectionwrites und acquire/release liegen außerhalb
des Proxyhooks. Auch innere next(bits)-Rejections sind nicht vollständig in R erfasst.
Die16 separaten canPlaceOre-Proben beobachten echte deklarierte N-Callbacks;
sie belegen nicht die Sectionread-Chronologie der60 Placements. Je Original
enthalten deren aufgezeichnete Proxytraces19.216 R-,32.657 W-,2.413 H-,933 B-
und1.150 C-Events. Fehlende vollständige Section-/Corechronologie ist ein
GEN-06.3-Präparationgate, kein Hindernis der writerfreien GEN-06.2-Primitive.
Die Blockdelta-Reihenfolge ist **Snapshotordnung**, keine Behauptung eines aufgezeichneten
Sectionwrite-Eventtraces; Sectionwrites liegen außerhalb des Proxyhooks. Voller FEATURES-
Interfeature-/Nachbarhistory-/Postprocessingtrace ist nicht geliefert.

`oracle/input-pins.json` pinnt alle Originalruntimeinputs, den ganzen JDK, Mappings,
Werkzeuge, Quell-/Ressourcenbytes und ausdrücklich readonly historische Dokumente.
Kein von Carver-WIP abgeleiteter Runtime-baseline-Ersatz. `run.py` verweigert fünf
Java-Injektionsvariablen, Byte-/Membershipdrift, Helperschatten in Original-JARs,
nichtfrische/fremde Ausgabe und falsche frisch compilierte Helpermembership.
Pre-/Postchecks benutzen `require`, **keine Python-asserts**. Logs mit Originaltimestamps
sind diagnostisch; TSV und Manifest sind die reproduzierbare Evidenz.

`fixture.py` projiziert alle Originalrows verlustlos ohne Originzellen; beide Modi
bleiben erhalten. Es prüft auf einer zweiten Route komplette Ressourcenbytes direkt
gegen ursprüngliche ZIPentries und Quellenkopien gegen ursprüngliche Base/Applieddateien,
roster/counts, unique row keys, Presetindexgleichheit, erlaubte Traceevents, Blockdelta-
Domain und Erfolgskonsistenz. `validate` vergleicht jede Row zurück mit den originalen
Rohbytes. Sieben malformed Einzelmutationen müssen verweigern: Header, Fieldcount,
unknown Kind, fehlende Row, Duplicate, Doubleergebnis, Coretrace. Das sind echte
Validator-Refusals und keine bereits vorhandenen Rustparser-/Budgettests.

Normal und Python `-O` laufen frisch; normale/optimierte Fixtures und Originaloutputs
müssen bytegleich sein. Guards: fünf echte Environment-entrypoint-Refusals, declared
Digest-/Shadow-/JDKmembership-/Preparationmembership-Faults, Fresh-/Scopeguards und drei
**echte JVM-CodeSource-Verweigerungen vor jedem Sample** (Original, Helper, Bibliothek).
Declared fault injection und tatsächlich geänderte JVM-Erwartungspfade sind getrennt.
Keine Mutation eines historischen Artefakts oder Original-JARs.

Die maschinenlesbare Endauswertung `delivery/analysis.json` enthält die tatsächlich
ausgeführten Zählungen und alle Vanilla/Paper-Differenzen nach kind/case. Gleiche
Blockdeltas werden nicht als gleicher Zufallsstream ausgegeben. Die Reportdatei ist
Korpusbefund, kein source-parity-, allgemeiner Gameplay- oder Performancebeweis.

Der abschließende Corpus liefert **60/60 gleiche Return-/Netto-Blockdeltas** zwischen
Vanilla und Paper, insgesamt je31397 Nettozellen über die unabhängigen kontrollierten
Aufrufe. Dennoch unterscheiden sich **51/60 **aufgezeichnete Proxytraces** mit der oben exakt beschriebenen Oberfläche**; die
48/48 Seed-/Wrapper-Drawrows unterscheiden sich in den Doublebits, ebenso die
Maximal-Corebit-Doubleprobe. Gleiche Nettozellen beweisen damit keinen gleichen Stream.
Normal/-O-Original-TSVs, Manifeste und Helper-Class sowie sämtliche Fixturebytes sind
identisch; je14 Guards und je7 malformed Refusals bestehen in beiden Pythonmodi.
190 vollständige Quellen-/Patchkopien, 894 Ressourcen und1728 Inputpins sind geprüft;
324 Dateien der akzeptierten GEN-05.10-Worldbasis bleiben bytegleich.

## 9. Echte fehlende Produzenten und kleinster Folgeticket

| Voraussetzung | Tatsächlicher Stand / konkreter Consumer |
| --- | --- |
| Decoration-RNG-Wrapper | GEN-04 liefert Core-Xoroshiro, nicht BitRandomSource-Wrapper. Separater metered Adapter nötig, insbesondere Seed/long/int/double; bestehende öffentlichen RNG-Fixtures unverändert. |
| Geschlossener Placementkatalog | Registry-worldgen API liefert derzeit Noise/Density/Settings, keine Placed-/ConfiguredFeatureResourceIds. Ein World-only Builtinkatalog kann exakte Ressourcendaten in seiner eigenen Crate besitzen; allgemeine pack-reloadfähige Registrytypen wären ein separater Registryticket, kein Stub. |
| Targettags | Drei originale Blocktags und konkrete dense Registry-Statebereiche lassen sich World-only geschlossen binden. Generische Tags/Tagupdates sind kein vorhandener Producer. Kein UnknownTag→false oder AIRfallback. |
| CARVERS-Puffer | GEN-05.11 nicht angenommen; kein jetzt akzeptierter CARVERS-Producer. Surface allein ist kein FEATURES-Vorgänger. |
| Heightmaps | Surface veröffentlicht nur WORLD_SURFACE_WG und Fluidset. OCEAN_FLOOR_WG, die vier finalen Maps und genaue state.blocksMotion/leaves/fluid-Klassifikation fehlen als komplette admissible Eingabe. Registry is_solid_cube ist kein Ersatz; vorhandene Originalsnapshot-Zellen sind Fixtures, keine Laufzeitproducer. |
| Biomegeneration-Settings | Jeder der54 IDs braucht seine vollständige Featuremitgliedschaft und tatsächliche Graphindexliste. Der Featurefilter darf keine homogenisierten „in jedem Biom erlaubt“-Regel verwenden. |
| Nachbarschaft/Stageauthority | Neun vollständige CARVERS-Chunks mit erhaltenen Biomarrays, WG-/finalen Maps, Postprocessing und versionsgebundener exklusiver Mehrchunkreservation. STRUCTURE_STARTS Radius8 und tatsächliche Scheduler-Completion bei einem ganzen FEATURES-Lauf zusätzlich. |
| Vorhergehende Dekorationen | Steps0..5, Strukturen und die Nicht-Ore-Einträge zwischen Oreindizes; spätere Step7-ore_infested sieht auch Step6-Writes und sculk/dripstone. Kein erfolgreicher leerer FEATURES-Schritt. |
| Postprocessing/Adoption | Geordnete per-section Listen samt Duplikaten, echte BorderTicks/Blending und spätere Update/Lightingconsumer; World-Regionergebnis ist keine Live-Chunkinstallation. |

**Verbindlicher kleinster Briefentwurf: GEN-06.2 nach §§7/12**, World-only,
alle57 Originalressourcen, ganzer164-Indexexport, private Paperdefault-Decorationwrapper
und sealed lazy Pendinglease; kein Blockwrite, kein Region-/Biomeconsumer.
Vanilla bleibt Testkontrolle. Der vollständige Bind-Debit, Scratch und alleinige
Runmeter sind Pflicht, keine optionale Vorprüfung. Ein pending Candidate ist
unentschiedene Fortsetzung, kein Biomepass; cfg(test) benutzt denselben Handoffkern.

Danach ein separater World-only **GEN-06.3 OreFeature-Algorithmus** mit geschlossenem
Targettag-/Statebinding, kontrollierter **ehrlich deklarierter** Region und vollständiger
Section-/Draw-/Neighborchronologie, typisierten Status/Heightmap-Lücken und mehrchunk-
atomic result; kein Scattered-Netherconsumer und kein vollständiger FEATURES-Treiber.
Nicht vorhandene Stage-/Regionproducer müssen zuerst eigenständig abgeschlossen sein,
bevor .3 echte Producerinputtypen statt kontrollierter Kerneltests public benutzen kann.
Zuletzt ein eigener **vollständiger Decorationdriver** mit allen vorhergehenden
Dekorationen/Strukturen, echten Featureindizes und anschließend ein separates
Sim-Adoptionticket. Die beiden letzten Schritte passen sachlich nicht in den kleinen
Primitiveticket und werden nicht durch sein Bestehen automatisch autorisiert.

Runtimebedingungen für alle späteren Teile:

- Debug/Release vergleichen jeden übernommenen Originaltoken, einschließlich Drawbits
  trotz gleicher geometrischer Outcomes; Fixture darf nicht auf gleiche Blockdeltas
  reduziert werden. Sources/Patches/Mappings/Origin/normal/-O bleiben pinned.
- Arbeit N−1/N/N+1 für tatsächliche Core-Rejections, Candidate0, Count0, erster/letzter
  Heightdraw, Culling, Bitmark vor Guard, Targetmismatch ohne Draw, Airneighbor-
  Shortcircuit und finaler Write; budget0/max/max+1 separat. Exhaustion hinterlässt
  keine publizierte Hälfte und keinen wiederverwendbaren halbfortgeschrittenen Cursor.
- Retained-/Scratchcaps jeweils exakte **literal** Runtimebytes auf der gepinnten
  Toolchain, N−1/N/N+1 mit unabhängigen privaten Limitcontrols. Compiler-/Bindallocations
  zählen; Candidate-/Algorithmuspfad erst nach Messung als null allocations bezeichnen.
  Externe kalte Debug/Release-Allocatorprozesse mit positiver4096-B-Kontrolle, erster
  Erfolg und jede Verweigerungsart. Keine installierte Performancebehauptung.
- Parser malformed: duplicate/escaped duplicate, UnknownResource/Tag/Modifier/Rule,
  falsche Types, size −1/0/64/65, chance NaN/Inf/−ε/0/1/1+ε, Count−1/0/256/257,
  reversed Height-/Countbounds und Anchoroverflow; Original-Codecs und FerrumC-Policy
  werden getrennt getestet. Die originalen Codecgrenzen sind keine Builtinzulassung
  aller hypothetischen Konfigurationen.
- Regionpolicy: jeder fehlende/duplizierte/falsche Nachbar, mismatchSeed/Settings/
  Source/Geometry/Status/Version, fehltMap/Classification, writeRadiusrand, Bauhöhe−65/
  −64/319/320; falsche Ordnung/mehrere Fehler pinnen Präzedenz. Nicht zulässige Defaults
  niemals durch gültige STONE-/AIR-/Plainswerte ersetzen.
- Mehrchunkresult: mindestens ein Write über jede positive/negative Chunk-/Shardnaht,
  keine Einzelpublikation bei spätem Fehlermeter oder Versionskonflikt, Replay von
  identischem kompletten Vorgänger reproduziert gesamten Result/Trace. Nachbardaten
  dürfen nach Abschluss andere Owner haben; der private Result bleibt unabhängig.
- Original-Semantikcases: gleiches Ziel/kein Netto-Delta mit true; zwei Zieltests,
  Wasser gegen Luft, jede Luftnachbarrichtung, chance0/1 ohne Draw, chancegleichheit,
  Gate-equality, Entfernen einer inneren Sphäre inklusive tie, Maskbit trotz Guard-false,
  verschiedene Attemptmasken und all-size-doubles-vor-erstem-Write. Diese hier nicht
  alle ausgeführten Fälle sind verpflichtende **zusätzliche** Abnahme, keine vorhandene
  Wholechunkcoverage.

## 10. Whole-FEATURES-Oracle: konkrete ehrliche Lücke und Plan

Ein faithfully runnable WorldGenRegion mit tatsächlichem ServerLevel und vorherigen
Dekorationen liegt in dieser Lieferung **nicht** vor. Die bestehenden Surface-/Carver-
Libraryproben enthalten weder komplette Struktur-/Featureconsumer noch einen lebenden
Serverlevel mit Heightmap-/POI-/Biome-/Tick-/Featureseedkonfiguration und Scheduler-
Completions. Eine neunfach kopierte Surface-/Carverprobe mit erfundenem FEATURESstatus
wäre keine Lösung. Kein allocateInstance für ServerLevel/Regionstatus wurde hier benutzt.

Der nächste Whole-stage-Export muss einen **tatsächlich gestarteten** gepinnten Paper-
Server oder nachweislich gleichwertig vollständig initialisierten originalen Server-
Lifecycle benutzen. Isoliertes Weltverzeichnis unter diesem Ticketarea, config/seed/
WorldOptions/datapacks/plugins/gamerules/featureSeeds/Blending pins; Vanilla separat.
Generationstop/Recording muss den echten `generateFeatures`-Task erreichen, bevor Maps
geprimet/Decorations gestartet sind; alle Dependencyholder im radius8 und CARVERS-
Nachbarn dokumentieren. Kein Statussetter ohne tatsächlichen Vorgängeraufruf.

Declare-before-run Corpus: vier Seeds0/−1/i64-Min/Max, negative/positive Chunknähte,
normal/large_biomes/amplified, mountain-emerald/infested, badlands-extraGold,
dripstone-largeCopper, lush-clay, cave/air-exposure, top/buildheight und Weltkante.
Originalseitige Auswahl darf echte Outputs benutzen, absent Targets werden als Lücke
berichtet; keine synthetischen Blöcke als generierte Chunkerwartung.

Für jeden echten FEATURES-Call aufzeichnen:

1. Vorheriger **gesamter 3×3-CARVERS-Snapshot** mit aller Stage-/Generationidentity,
   echten Holdercompletions, gespeicherten Biomen/RTree-Schedulinghistory soweit der
   tatsächliche Producer sie konsumiert, beiden WG-Maps, vorhandenen finalen Maps,
   Blockentities und geordneten Postprocessinglisten.
2. Original primeHeightmaps-Delta; Biommenge; echter globaler Sorter-/Stepindex und
   Configured-Holder; Struktur-/Featureseed und alle RNGcore/Draws. Vollständige Steps,
   inklusive nicht in diesem Erzscope befindlicher Consumer und Regionreads/writes.
3. Für jeden Ore-Aufruf Original-Placementkandidaten, Biomholder/hasFeatureentscheidung,
   Heightgate, Sphären-/Cullingchronologie und Sectionwrites mit bisherigem/neuem State;
   ausschließlich transparente Aufzeichnung, kein nachgeschriebener Erwartungsalgorithmus.
4. Nachher alle neun States/Maps/Counter/Entities/Postprocessing, `currentlyGenerating`,
   Blender-BorderTicks und finaler Taskstatus. Snapshot auch bei echter Exception, ohne
   diese als erfolgreiche Ergebnisrow zu klassifizieren.
5. Zweite unbeobachtete Generation desselben Ausgangszustands; recorded/unrecorded
   volle Snapshots müssen gleich sein. Neighborcenter-FEATURES-Abfolge forward/reverse
   getrennt behalten, keine scheduleabhängigen Writes normalisieren.

Danach erst die Rustproducer vor ihrer Consumption gegen echte Vorgänger prüfen und
alle Write-/RNG-/Biom-/Statuszellen differentiell vergleichen. Dies schließt sowohl
GEN-05.11s offene Carverhistorie als auch den zusätzlichen FEATURES-Multichunkvertrag
nicht rückwirkend. Ganzserver-/Survival-/Java-Plugin-/Lighting-/Nether-/End- und
Performanceabnahmen bleiben eigene Arbeiten.


## 11. Konkrete Korrekturantworten, H1 und Abnahmeplan

| Einwand | Verbindliche Revision-2-Antwort | verbleibendes Gate |
| --- | --- | --- |
| CR61-B1 | §7 legt obligatorischen vollständigen Debit, BindAllowance, exakte zwei Byteprofile, simultane Compilerbytes und sealed alleinige RNG-/Meterlease fest. Fresh/Running/Pending/Complete/Poisoned, Completion/Drop/forget und N-Prefixe sind konkret. | unabhängiger Korrekturreview; danach tatsächliche Rustmeter-/Byte-/Allocatorabnahme |
| CR61-B2 | §3 korrigiert Discovery auf equals/hashCode; Identity gilt erst für den späteren Stepindexlookup. §4 nennt hasFeature als Equalitymitgliedschaft kanonischer Records. Alle164 globalen Originalindizes bleiben vollständig unverändert. | equal-clone-/Selbstzyklusrows bei Runtime erhalten, kein ore-only Sorter |
| CR61-B3 | §8 weist die60 Placementtraces ausdrücklich als öffentliche Returned-Draw-/Proxytraces mit0 S-Events und Nettosnapshots aus. Die16 N-Callbackproben sind separat. | vollständige Section-/Bulk-/Corechronologie vor GEN-06.3 |
| CR61-H1 | Neuer Helper `H1Guard.java` benutzt arithmetische `block>>4`-Chunks und signed Intervallchecks;18 X/Z/Diagonalproben der sechs verlangten Werte, normal/−O. Diskrepanz zum Reviewtext ausdrücklich unten. | keine ungeprüfte Wiederverwendung einer abs-before-shift-Policy |
| CR61-H2 | §7.6 nennt7.077.888 B gleichzeitig lebendes Input+Copy-Payload;4-MiB-Regionbehauptung zurückgenommen, kein neuer angenommener Regioncap. | vollständige simultane Region-/Result-/PP-/Borrowedbyte- und Arbeitsplanung später |

### H1: Quellenbefund und sechs konkrete Grenzen

Die erneute vollständige Pinprüfung bestätigt den unveränderten historischen Helper
SHA-256 `681cfa680f3a036b045051327eddca170f55cb69e1414ea47ab7ab63758146e7`.
Seine Zeile225 enthält tatsächlich
`Math.abs(bx(a[0])>>4)<=1&&Math.abs(bz(a[0])>>4)<=1`.
Damit steht die Klammer **nach** dem Shift; dieser gepinnte Ausdruck verwendet
arithmetische Chunkkoordinaten. Der H1-Reviewtext beschreibt dagegen
`abs(block)>>4`. Diese Beschreibung passt nicht zu diesen historischen Bytes.
Revision2 behauptet daher nicht, durch historische Mutation einen existierenden
Fehler korrigiert zu haben. Sie übernimmt die geforderte robuste neue Policy
`cx=x>>4; cz=z>>4; -1<=cx<=1 && -1<=cz<=1` ohne abs und diskriminiert die
im Review beschriebene falsche Variante als gesonderte negative Kontrolle.

| Blockwert | arithmetischer Chunk | neue Policy (andere Achse0) |
| ---: | ---: | --- |
| −33 | −3 | false |
| −32 | −2 | false |
| −17 | −2 | false |
| −16 | −1 | true |
| 31 | 1 | true |
| 32 | 2 | false |

Jeder Wert wird auf X, Z und beiden Achsen geprüft. Alle18 neue Ergebnisse sind
bitgleich zur echten gepinnten historischen Expression. Die abs-before-shift-
Negativkontrolle lässt −17 in drei Achsenkombinationen fälschlich zu und wird
hierdurch diskriminiert. `h1-normal-final/boundaries.tsv` und
`h1-optimized-final/boundaries.tsv` sind bytegleich; Helper-CodeSource und Membership
werden geprüft. Dies ist ein **deklarierter Guardpolicy-/Java-Arithmetiktest**,
kein neu gemessener WorldGenRegion- oder Minecraft-Stageaufruf. Die60 historische
Rows werden mit dem alten Helper nur zur Reproduktion bytegleich wiederholt;
kein alter Helper/Freeze wird editiert. Die Quelle-/Reviewdiskrepanz ist im
unabhängigen Korrekturreview anhand dieses konkreten Pins zu prüfen.

### Verpflichtende zukünftige Rusttestmatrix (hier noch nicht ausgeführt)

| Bereich | konkrete Fälle und erwartete Refusal-/Ownershipwirkung |
| --- | --- |
| Bindbudget | 0/1/max/max+1; Paper **869.556/869.557/869.558**, Vanilla **869.257/869.258/869.259**; N−1 verweigert letzte Publication, N/N+1 used=N; alter Katalog und alle Sources unverändert |
| jeder Bindprefix | vor/nach erstem/letztem Hashbyte, Kompressionsblock, JSONbyte, Node, Keyduplicatevergleich, Reference-/State-/Taglookup, IRbuild, vollständigem164-Indeximport, reserviertem Cleanup und Publication; bei gruppiertem Lookup reservation−1/N/+1 kein Teilcall |
| malformed Parser | leer/truncated/trailing/invalid UTF-8, falscher Root/Type, unknown und duplicate/escaped duplicate Keys; fehlende/zusätzliche/fremde Resource, Tag, Modifier, Provider, Rule; ungeordnetes/mehrfaches/fehlendes Indexrecord; production zuerst Digestdrift, cfg(test) anschließend derselbe Parserkern |
| konkrete Codec-/Policygrenzen | size−1/0/64/65, Count−1/0/256/257, Rarity0/1/6/9/i32-Max, DiscardNaN/Inf/−ε/0/1/1+ε, reversed Count-/Heightbounds, unbekannte Properties/Values/fehlendes Name, zwei Ankerkeys und Anchoroverflow; original leniente/reversed Methodenrows versus strikte geschlossene FerrumCzulassung getrennt |
| Source-/Parser-/IRbytes | exakte source length±1 (Paper721, Vanilla720), aggregate±1; nodes63/64/65, depth7/8/9, entries7/8/9, String63/64/65 über cfg(test); vier einzelne Scratchpartitionscaps, simultaner16-KiB-Scratch/16-KiB-Staging/32-KiB-neuer-Peak/49.152-B-alt+neu jeweils N−1/N/N+1 |
| allocations | erste kalte erfolgreiche Bindung und jede Budget-/Malformed-/Identity-/Native-/Allocationfailure separat Debug/Release; echte fallible-reserve-Verweigerung, kein hidden growth; unabhängige positive4096-B-Allocatorcontrol; tatsächliche capacities/Alignment/temporäre Resultbytes, keine RSSumdeutung |
| Runmeter / Seeding | 0/1/2^20/2^20+1; Beginprefix und Seed1/alle viercore/Decoreseed/Featureseed bei N−1/N/N+1; Falliblecharge verweigert vorMutation; used/corecount getrennt vom Java-getCount; erste/rejected/letzteboundeddraw und nextDouble-Secondcore einzeln |
| lazy Count0/Rarity | N11±1, CountUniform0 keine Square-/Height-/Biome-/Consumerziehung; chance6/9 nextDown/equal/nextUp des echten f32-Grenzwerts; square-X dann Z, danach uniform oder zwei trapezoiddraws |
| Pendingemission / Completion | ore_dirt-Skript N80±1, jede Pending-/Dispatch-/Completionprefixgrenze und letztes End1; consumerdraws verändern nächste X/Z/Heightfolgen; Emissionrefusal liefert keine Position, Completionrefusal keine neue Runningauthority |
| alleiniger Consumer | zweites next während Livelease compile_fail; keine externe RNG-/Clone-/Reseed-/Budgetupgrade-/bool-Erfolgseinspeisung; fremder Katalog/Key/Identity/Sequenz in cfg(test) verweigert; exactly-once Completion, Drop vor/nach einem Draw, Consumerfailure/Panicunwind und mem::forget terminal |
| Domain / Identity | seed/settings/source/geometry/center/key/stepindex einzeln falsch, mehrere Fehler in festgelegter Reihenfolge; negative/asymmetrische Chunknähte und ±30M-Edges; falsches Begin/next nach Complete/Poisoned mutiert keinen RNG; Candidate-Y außerhalbBauhöhe wird original belassen |
| B2 / Double | unveränderte164 Tabellen allerdrei Presets, distinct-equal-record equality/cycle Originalrows; beide Doublebitprofile und Paper1.0, kein directXoroshiro-Lemire-Ersatz |

Runtime muss alle48 Seed-/Wrapperrows,60 Height-/52 Count-Endpunkte, echten
Rejection-/Doublemax- und Reviewzusatzrows in Debug/Release prüfen. Die Corpus-
Malformedchecks dieser Vorbereitung beweisen ausschließlich die Checker, nicht
Rustbudget- oder Allocationeigenschaften. Workbudgettests dürfen eigene checked
Limitcontrols haben; sie dürfen keine bisherigen öffentlichen GEN-04/GEN-05-Caps
ändern. Tests benutzen den realen privaten Dispatch-/Pending-/Wrapperkern,
keinen separaten Testcursor, der die Productionhandoff umgeht.

Erhaltene **spätere** Algorithmus-/Regionpflichten: target-mismatch ohne Draw,
gleicher Zielstate/kein Nettodelta mit erfolgreichem Write/true, beide Targets,
Wasser versus Luft, jede Luftnachbarrichtung, chance0/1 ohne Draw und Gleichheit,
Gate-equality und fehlender Gate, alle-size-Doubles vorerstem Write,
Sphärenculling/tie, Bitmark auch bei Guardfalse, unterschiedliche Attemptmasken,
Sectionacquire/release bei Failure, Write über jede positive/negative Chunk- und
Shardnaht, Bauhöhe−65/−64/319/320. Jeder fehlende/duplizierte/falsche Nachbar,
Status/Map/Klassifikation/Versionfehler, spätes Exhaustion/Versionskonflikt müssen
atomic Refusal ohne Teilinstallation liefern. Keine Endstate-Neuberechnung von
WG-Maps. Das sind zusätzliche verpflichtende **unbewiesene** Abnahmen.

## 12. Kleinstes konkretes GEN-06.2 und ehrliche Folgegates

**Vergabefähiger Briefentwurf nach unabhängigem CLEAN_CONTRACT-Korrekturreview:**
`GEN-06.2 — World-only closed ore placement primitives`, ausschließlich
`server/crates/ferrumc-world/**` auf einer von Root eingefrorenen akzeptierten Worldbasis.
Der frühere akzeptierte Worldpin `a912b1c2` bleibt maßgeblich; die docs-only-Mainbasis
`c46077e1` ändert keine Producerbytes. Kein Registryreloadticket und keine Storage-/Sim-/
App-/Dependency-/GEN-04-/GEN-05-Änderung. Root muss den neuen16-KiB-Bindscratch und
32-KiB-Compilerpeak vor Runtimevergabebeginn ausdrücklich mitreviewen.

1. Ganze57-Quellenclosure mit exakten Paperbytes, Properties,3 Tags und allen164
   originalen globalen Indizes; private World-owned checked IDs. Verpflichtender
   BindAllowance1..=2^20, PaperN869.557, kein unmetered Parser/Build.
2. Privater originaler BitRandomSource-/WorldgenRandom-Decorationadapter über einen
   eigenen Core, original Java-long/f32-Arithmetik, PaperDoubleprofil; Vanilla nur
   testdiscriminator. Kein vorhandenes directXoroshiro-API zum Wrapper umbenennen.
3. Sealed lazy Session und Pendinglease nach §7; unveränderlicher gemeinsamer Runmeter
   1..=2^20, consumer-interleaved Draws, Count0/Rarityrefusal, keine Candidatevorberechnung.
   Keine Produktions-Biome-/Algorithmusconsumer-Variante in diesem kleinen Ticket;
   private cfg(test)-Controls üben die reale handoff, nicht einen Successcallback.
4. Debug/Release sämtliche echten Originalrows und geplanten Fehl-/Meter-/Ownership-
   Grenzen; tatsächliche native retained/scratch/Stack-/Allocationsmessungen mit
   independent positive4096-B-Control. Whole-World, strict all-target Clippy, fmt
   und warnings-as-errors rustdoc auf **akzeptierter Archivbasis plus test/Runtime-
   Overlay**, kein Test der geteilten Storage-WIP als akzeptierte Gesamtbasis.
5. Nur nach komplettem Freeze/Membership-/Outside-bytecheck eigener unabhängiger
   Runtime-Review. Eine grüne .2-Abnahme akzeptiert kein .3, keinen Regiondriver,
   Biomfilterersatz, CARVERS, FEATURES, Naturalworld/Survival/Performance oder Aktivierung.

**GEN-06.3 bleibt Vorbereitungsgate:** transparente vollständige Originalinstrumentierung
von direkten Section-/Bulkreads, Writes, acquires/releases und jedem Wrappercore/
Rejection; gesonderter unbeobachteter Kontrolllauf mit vollständigem identischem
Snapshot. Dann eigener geschlossener algorithmischer Regionkernel-/Meter-/Bytevertrag
unter H2; echte public Regioninputs erst nach angenommenen Producerdaten. Scattered
bleibt Netherkontrolle und erhält hier keinen Runtimeconsumer.

Danach eigenständiger vollständiger Decorationdriver (alle Steps, Strukturen und
Zwischenfeatures in Originalreihenfolge), genuine Whole-FEATURES-Lifecycle nach §10
und separates mehrchunk-atomic Simadoptionticket. Vollständig gespeicherte Biome und
echte frühere Stagecompletions dürfen nicht aus kontrollierten matching Holders oder
neun kopierten STONE-Inputs erzeugt werden. GEN-05.11 bleibt unangenommen.

## 13. Neue Reproduktion, Integritätsguards und Freeze

Eigener Area ausschließlich `.codex-tmp/gen06_1-r2/`; keine Gitaktion, Rustproduktion,
Dependency, weiteren Agenten oder historische Mutation. Keine private Rust-
Designprobe/Overlay; Java verwendet nur gepinnte originale JARs und JDK. Die frischen
Pythonwrapper ändern ausschließlich erlaubte Outputpaths und rufen readonly kanonische
Autorenhelpers auf. Reviewprobe/Secondchecker/Guards werden als neue bytegleiche
Werkzeugkopien ausgeführt, nicht als bereits ausgeführte frühere Ergebnisse ausgegeben.

Explizite finale Evidenz:

- `normal/`, `optimized/`: unveränderter Autorenhelper frisch compiliert; beide
  rohen Original-TSVs bytegleich zu Autor und Review. Alle kanonischen JDK-/Input-/
  Source-/Helper-/ZIPshadow-/JVM-CodeSourceguards vor/nach Samples.
- `fixture-normal/`, `fixture-optimized/`: alle1.218 Originalrows je Modus vollständig
  erhalten, eigene neue Projections bytegleich zur historischen Fixture.
- `probe-normal-complete/`, `probe-optimized-complete/`:128 zusätzliche genuine
  Originalrows einschließlich lazy/Count0/Rarity/reject-all/equality/cycle, separat
  gepinnt. Die unabhängige Replay-TSV bleibt bytegleich zum Originalhelper. Keine
  originale Klasse oder Erwartungsformel wurde ersetzt.
- `guards-normal/`, `guards-optimized/`:14 tatsächliche/deklarierte Refusalarten und
  drei positive Controls selbst ausgeführt. Private Byte-/ZIPshadow- und echte
  JVM-CodeSourcefehlpfade verweigern vor Samples; nur private Fehlermeldungspfade und
  Pythonmode werden für Guardvergleich normalisiert.
- `author-mutations-*`: sieben originale Projection-Malformedverweigerungen;
  `own-mutations-*-complete` und `tokens-*.json`: zusätzlicher Secondchecker akzeptiert
  jede unveränderte Row und verweigert elf semantische Einzelrowmutationen sowie fünf
  Framing-/Completenessmutationen normal/−O.0 S-Events bleibt exakter Befund.
- `bytecodes/`: frische Originaljavap-Kontrollen beider Doubleprofile, Wrapper,
  Ore/Scattered, BiomeSettings und StepFeatureData.
- `h1-*-final/`: die18 neuen ausdrücklich kontrollierten Grenzproben, separat von
  Originalplacementrows. Keine neue Minecraft-Sectionchronologie behauptet.
- `binding-final-v3/`, `binding-optimized-v3/`: sämtliche57 exakten Ressourcenbytes
  **je Original** samt ganzen164 exportierten Indizes und unabhängigem Source-/
  Workcensus; vollständige Membership und Bytes normal/−O identisch.
- `pins-audit.json`, `integrity-*.json`, `delivery-summary.json`, `input-pins.json`:
  alle historischen Autoren-/Reviewpins und akzeptierten Worldbytes vor/nach geprüft;
  Herkunft der neuen Werkzeuge/Originalinputs und Byte-/Count-/Modevergleich explizit.

Die ersten lokalen Ausgaben `binding/`, `binding-final/`, `binding-final-v2/` und
`h1-normal/` sind **nicht finale Evidenz** und werden nicht eingefroren: anfänglicher
Census erwartete fälschlich bytegleiche JARresources statt des tatsächlichen Paper-LF;
früher H1checker erwartete fälschlich sechs statt drei diskriminierte Achsenrows;
frühere Bindtabellen enthielten noch nicht die finalen expliziten Paare/Censusscans.
Ein erster Secondcheck lief vor der optimierten Projection und scheiterte deshalb
an deren fehlender Datei; final läuft er erst nach beiden vollständigen Projections.
Diese Entwicklungsfehler sind korrigiert, keine Acceptance und ändern keine
historischen Bytes. Endgültige Autoritätsnamen sind ausschließlich die oben genannten.

Reproduktion in frischen Namen innerhalb eines eigenen erlaubten r2-Areas:
`reproduce.py run OUT` normal/−O, anschließend `generate/validate/mutations`,
`probe.py OUT` normal/−O, `guards.py OUT` normal/−O, dann `check_tokens.py` nach beiden
Fixtures, `bytecodes.py OUT`, `h1.py OUT` normal/−O und `census.py` mit frischem
Ausgabeverzeichnis. Skripte behalten Fresh-/Scopeguards; vorhandene finale Inputs
werden nicht überschrieben. Zweitchecker und Integritychecker benutzen `require`,
keine deaktivierbaren Pythonasserts. Die Freezeprüfung enumeriert **nur** den neuen
Vertrag, neue Werkzeuge/Pins und die expliziten finalen Evidenzpfade; alte Freeze-
Manifeste und Sourcepins bleiben ausschließlich readonly Referenzinputs.

`freeze-manifest.json` pinnt jede finale Datei plus den Revision2-Vertrag. Manifest
und äußere `hashes.json` sind zur Vermeidung zirkulärer Hashes ausgeschlossen; die
Receipt nennt exakten Vertrag-/Manifest-/Summaryhash sowie Datei-/Bytecensus.
Der separate Integritychecker rehasht danach jede Datei, die Scopemembership und
alle historischen Pins erneut normal/−O. Diagnostische JVMlogs sind einzeln enumeriert,
aber ihre Timestamps sind keine bytegleiche Differentialerwartung.

**Status: AUTORENKORREKTUR ZUR UNABHÄNGIGEN PRÜFUNG.** B1/B2/B3 sind in diesem
Entwurf konkret beantwortet, kein Rooturteil vorweggenommen. Native Konstruktor-
und Cursorlayout/Stack/Allocator/RSS, all-seed Erfolg unter endlichem Meter,
volle Section-/Corechronologie, echte Biome-/CARVERS-/Region-/FEATURESproducer,
Schedulerhistory/Adoption und Survival-/Performance-/Appaktivierung sind unbewiesen.
Keine Runtimeautorisierung; ein neuer unabhängiger Korrekturreview ist Pflicht.
