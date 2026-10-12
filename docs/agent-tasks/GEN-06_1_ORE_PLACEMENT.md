# GEN-06.1 — Vorbereitung gewöhnlicher Overworld-Erzplatzierungen

Lieferung vom 12.10.2026 zur verbindlichen [Workorder](GEN-06_1_ORE_PLACEMENT_TASK.md),
Workorder-Commit `04dcffebbe8037edc21e21fe3ae4b06280d68a08`. Zuweisungsbasis laut Root:
Main `bfe580e3`; akzeptierter Surface-Treiber `a912b1c2`. Dieser deutsche Vertrag und
`.codex-tmp/gen06_1/**` sind die einzigen eigenen Dateien. Keine Gitaktion,
Produktionsänderung, Dependency oder weiterer Agent. Keine private Rust-Archivprobe.
Gemeinsame Storage-/Sim-Arbeitsstände sind keine Authority.

GEN-05.11 ist **nicht angenommen**. Sein Vertrag bleibt readonly, SHA-256
`3f6ca7387ab19d0fcf063b9cb4bfbe0c534e7cedafa5a87e61f6107bb3f137ff`; sein Freeze
`6451131f336c011486edcfc91d1a43a38274f4e5e390b48e3b5ba72f88b29863`.
Die Registry-/Reflection-/Guard-Methodik wurde gelesen und teilweise als deklarierte
Werkzeugbasis übernommen; kein Carver-Ergebnis wird hier Runtimeauthority.

Die Kennzeichnungen sind verbindlich: **Quellenbefund** = komplette Originalmethoden
und Callees gelesen; **Originalprobe** = tatsächliche Methoden aus den gepinnten JARs;
**kontrollierte Region/Ziehung** = deklarierte Eingaben, kein generierter Chunk;
**FerrumC-Policy** = vorgeschlagene strengere Zulassung/Transaktion, kein Originalverhalten.
Diese Vorbereitung autorisiert keine Runtime und akzeptiert keinen FEATURES-Schritt.

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

## 3. Decoration-Reihenfolge, Biome und RNG

`ChunkGenerator` memoisiert `FeatureSorter.buildFeaturesPerStep(List.copyOf(
biomeSource.possibleBiomes()), generationSettingsGetter.features, true)`. Der Sorter
assigniert discovery IDs über Objektidentität, baut einen durch Biomlisten bestimmten
Graph über (Step, discovery ID), führt DFS mit geordneter Adjazenz aus, reversiert das
Ergebnis und baut **pro Step** eine neue Identity-Indexmapping-Liste. Zyklen werfen
`IllegalStateException`, gegebenenfalls mit den beteiligten Quellen. Alphabetische
Sortierung, lokaler Biomindex oder eine auf Erze reduzierte Graphliste sind falsch.
Der Originalexport enthält alle 164 Step/Index-Zeilen und bestätigt normal = large_biomes
= amplified. Andere Sources/Packs brauchen einen neuen Indexvertrag.

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
  hasFeature(exactPlacedFeature)`. Keine RNG-Ziehung. `PlacedFeature.place` liefert
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

## 7. Bounded Ownership, Arbeit, Bytes und Fehler

Diese Grenzen sind **FerrumC-Policy/Designwerte**; keine gemessene Rust-Allokations- oder
Performanceabnahme. Builtin-Admission: nur drei Overworld-Settings/Multinoise-Quelle,
Paper Build60-Profil, genau die geschlossene 30/24-Katalogclosure; keine Plugins,
Featureseed-Overrides, Blending/Upgrading, Structures oder arbitrary Datapacks in einer
ersten isolierten Erzalgorithmusprobe. Ein kompletter FEATURES-Treiber darf solche
fehlenden Consumer nicht durch erfolgreiche Leersteps ersetzen.

**Geometrische endliche Schranken**, ohne RNG-Rejectionarbeit:

| Gegenstand | Ableitung / Maximum |
| --- | --- |
| size | 3..64 im echten Katalog; originale Codecdomäne 0..64 separat |
| Radius | ≤ size/16 + 0.5 ≤ 4.5, ausdrücklich einschließlich Paper nextDouble=1 |
| width/height | ≤26/14; Bitset-logische Größe ≤9464 Bits, 148 u64 =1184 Bytes |
| Sphärenarray | ≤64×4 doubles =2048 Bytes |
| Paarchecks | ≤64×63/2 =2016 |
| Gatehöhen | ≤(26+1)² =729 |
| konservative Loopbreite pro Sphäre | ≤11 pro Achse; ≤704 X-, 7744 Y-, 85184 Z-Besuche |
| unterschiedliche markierte Positionen | ≤9464 je Attempt; keinesfalls Chunk-global |
| Targets / Luftnachbarn | ≤2 / höchstens6 je tatsächlich verlangtem Check |
| Alle 30 Platzierungen vereint | ≤471 Algorithmus-Attempts vor Biome-/Höhengates; kein einzelnes Biom hat notwendigerweise diese Vereinigung |
| Horizontaler kontrollierter Erzscope | origin im Center0..15; konservativ Reads inkl. Nachbar [-14,29], Writes [-13,28] relativ zum Center; vollständig im 3×3 |

Ein späterer Algorithmusmeter charged vor jeder Gatehöhe, Sphäre, Paarprüfung,
X-/Y-/Z-Iteration, Bitlookup/-mark, ensureCanWrite, Sectionlookup, Stateread, Zieltest,
Luftentscheidung, jedem Nachbarread und Write; Schlussentscheidung 1. Konservatives
Geometrylimit pro Attempt **524288** Einheiten: aus 96441 Gate/Sphären/Paar/Loops,
85184 Bitlookups, 5×9464 mark/guard/section/read/write, 2×18928 target/airdecisions,
113568 Neighborreads und Schluss1 ergibt sich ≤380370. Native Handle-Aufbereitung,
Placement, Bindung und RNG werden zusätzlich gezählt, nicht kostenlos versteckt.
Das ist eine strukturelle Herleitung; die Runtime muss Instrumentierung und Schranke
gegen Originaltraces prüfen. Keine Behauptung einer Zeit-/TPS-Grenze.

**Rejections haben keine hier belegte kleine endliche Drawschranke.** Jede tatsächliche
WorldgenRandom.next(bits)-Iteration wird vor Coremutation gecharged, auch jede verworfene
bounded-int-Ziehung. Kein „ein Charge pro nextInt“ und kein unmetered Unterpfad. Eine
endliche Sessionallowance `1..=2^32` ist vorgeschlagen; exhaustion verweigert typisiert,
auch wenn ein Original mit längerem Stream noch weiterliefe. Dies ist bewusste Policy,
keine Garantie, dass jeder originale Seed erfolgreich admitted wäre. Schon die reine
Geometrieschranke 471×524288=246939648 enthält **keine** komplette FEATURES-Arbeit.
Erneutes Beginnen benutzt denselben Eingangsnapshot und Initialseed; kein Resume mit
halb weitergedrehtem RNG oder privaten Prefixwrites.

Primitive-Nachfolgeticket: WorkBudget `1..=1048576`; dynamisch ein Charge pro Modifier-
Entry, konkret geliefertem Candidate und tatsächlichem Core-next(bits), vor Änderung.
Read-only bind/build kann seine feste Katalogarbeit vorprüfen. Ein Candidate liefert
sich nur vollständig oder eine terminale Executionverweigerung; kein partieller Output.
Domain-/Identityfehler vor Begin lassen eine alte veröffentlichte Ausgabe unverändert.
Nach admitted Begin ist ein Lauf Fresh→Running→Complete/Poisoned. Kein Callback darf
nach exhaustion weitergezogen werden. Diese Policies ersetzen keine Originalausnahme.

**Native Designcaps**, vor Allokation checked und getrennt von Quellebytes, Stack und
RSS: Katalog einschließlich Konfiguration/Modifier/Tagmitgliedschaften ≤16 KiB retained;
Primitivecursor/RNG/Meter ≤512 Bytes, kein retained Candidate-Vec; Geometryscratch
≤8 KiB Stack (Sphären, Bitset, bis27 konservative Sectionhandles und Schleifenlocals).
Handlecap27 ist bewusst grob (3×3 horizontal und bis3 vertikale Sections einschließlich
Airneighborhood), nicht ein Original-Mapkapazitätsgesetz.

Regiontransaction für ein späteres Erzalgorithmenticket: genau neun vollständige
besessene Chunkstatepuffer, minY−64/height384, je98304 States; **3538944 Bytes**
Statepayload bei u32-IDs. Zusätzlich je sechs Heightmaps als rohe gleiche Semantik,
Sectioncounts, Fluidflags, geordnete Postprocessinglisten und Metadaten. Vorschlag:
Region retained cap **4 MiB**, Scratch8KiB; Postprocessing-Eingangslisten eigenes Cap
131072 Einträge und 512 KiB nach tatsächlichem Rust-Elementlayout, niemals als
Bitset normalisiert. Caps werden vor Copy geprüft; zu große echte Vorgänger sind
`NativeLimit`, kein Truncation. Bei zu großem Gesamtlayout muss Root den Cap separat
entscheiden. Erzausführung appendet selbst nichts. Borrowed Biomearrays werden als
borrowed capacities gezählt, nicht nochmals als retained Heap.

**Mehrchunkpublikation:** World nimmt ein bereits admitted, versionsgebundenes 3×3-
Eingangsobjekt exklusiv entgegen, führt gegen eine private Candidatekopie aus und
liefert ein **einziges geschlossenes Regionergebnis** mit Center/Seed/Settings,
Predecessorstatus/Versionsvektor, neun vollständigen Chunkdeltas bzw. Puffern und
nur nach Erfolg einem gemeinsamen Commitmarker. Kein Nachbarwrite in den Shared-Store.
Ein späterer Simkoordinator reserviert alle neun Besitzer auch über Shardgrenzen,
prüft den kompletten Versionsvektor und publiziert an einer Tickgrenze atomar; bei
Konflikt/defer keine Einzelchunkinstallation. Deterministische serialisierte Centers
und Konfliktreihenfolge müssen im Driver-/Adoptionticket festgelegt und gegen die
Original-Schedulerreihenfolge geprüft werden. Disjunkte Snapshots blind parallel zu
mischen wäre falsch: ein Nachbar kann schon frühere Decorationwrites enthalten.
Das aktuelle Vorbereitungsticket implementiert keinen dieser Sim-/Storagehooks.

Typisierte vorgeschlagene Fehler (private Payloads, keine Nachrichtenallokation):
`InvalidBudget`, `UnsupportedProfile`, `UnsupportedSettings`, `UnsupportedSource`,
`UnsupportedFeatureSeedOverride`, `UnsupportedBlending`, `MalformedDefinition`,
`UnknownModifier`, `UnknownHeightProvider`, `UnknownCountProvider`, `UnknownTargetRule`,
`UnknownResource`, `InvalidBlockState`, `InvalidAnchor`, `InvalidProviderRange`,
`CatalogueDrift`, `FeatureOrderCycle`, `NativeLimit`, `CoordinateDomain`,
`InconsistentIdentity`, `MissingChunk`, `WrongPredecessorStatus`, `MissingHeightmap`,
`MissingBlockClassification`, `MissingTopFeature`, `RegionVersionConflict`,
`WorkBudgetExceeded`, `Poisoned` und `OriginalFailure` mit deklarierter Klasse/Kontext.

Präzedenz: gültiger Budgettyp → Profil/Settings/Source/Overrides/Blending → Quellebytes
und JSONstruktur → Ressourcen/Codecclosure → Katalogcensus/Nativecaps → Regionidentität
(Seed, Settings, Source, Geometrie, Center) → vollständige Membership/Duplikate → Status
und Heightmaps/Klassifikation → Versionsreservation → Begin/Executionmeter. Exaktes
Fehlerpayload umfasst typed Resource/Chunk/Block, Inputordinal und Limit requested/max.
Laufzeitfehler nach Begin poisonen und veröffentlichen nichts; Domainfehler vorher
mutieren keinen RNG und keinen alten Snapshot. Originale leere HeightRange-Rückgaben,
BiomeFilter-IllegalStateException und Write-false sind explizit unterschiedliche
Verhalten, keine als „false“ zusammengeworfenen Fehler.

Sourcebytes-Policy für die kleine World-only-Katalogbindung: ≤128 KiB pro JSON, ≤2 MiB
aggregiert, Tiefe32, ≤4096 Nodes/Ressource, ≤256 Containerentries/Stringbytes;
genau30/24 Resources, vier Modifier pro Placement, maximal2 Ziele/config, drei Tags,
12 Mitgliedsreferenzen vor dedup. Unknown/duplicate keys, falsche Typen, unendliche
Discardwerte, ungültige Stateproperties, unknown Tags und übergroße provider values
verweigern. Das ist strenger als Vanilla-Codecs mit optionalen/promotePartial-Feldern;
keine malformed-input-Parität wird behauptet.

## 8. Tatsächliche Originalevidenz und Reproduzierbarkeit

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
zurückgesetzt. Drawtrace delegiert transparent an original WorldgenRandom; Statereads,
Heightgates, ensureCanWrite, Biomqueries und Chunkrequests sind aufgezeichnet.
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
Aufrufe. Dennoch unterscheiden sich **51/60 vollständige Draw-/Zugriffstraces**; die
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

**Empfehlung: GEN-06.2 „bounded Decoration-RNG und builtin ore placement primitives“,
ausschließlich ferrumc-world.** Umfang: eigener privater WorldgenRandom-Xoroshiro-
Wrapper; echte setDecorationSeed/setFeatureSeed; checked30-key Placementpläne mit
Count/Rarity/InSquare/Uniform-/TrapezoidHeight und Ankern; lazy Candidatecursor mit
explizitem terminalem „Biomecheck erforderlich“-Token. Dieser Token ist keine bereits
bestandene Biomeentscheidung. Der spätere Consumer führt den echten Stored-Biomfilter
und Algorithmus **zwischen** Candidateaufrufen mit derselben RNGauthority aus.
Cursor/APIs private Felder, checked IDs statt Strings/wire IDs; kein Region-/Server-
oder false-accept-Biomecallback und kein Blockwrite. Sealed Erfolgsobjekte bewahren
Original-Inputidentity; Testcontrols bleiben cfg(test).

Die originale Prüfliste dieses Folgetickets ist bereits konkret: alle48 Seed-/Draw-
Rows, beide Doubleprofile, exakte Core-Rejection,60 Height- und52 Count-Endpunkte,
Raritygleichheit/nextafter, Square-X/Z-Reihenfolge und Count0-Laziness, Java-Anker-
Arithmetik sowie eine Primitive-Sequenz mit deklarierter Feature-Draw-Unterbrechung.
Paper-RNG-Profil ist der Produktionserwartungswert; Vanilla bleibt discriminierende
Kontrolle. Eine RawSourceType darf nicht anstelle des Wrappers accepted werden.
Builtinkatalogbindung prüft ihren echten bytes/nodes/count-Census und Source-Digests;
kein Parser, der nur die gerade benötigten Keys toleriert und Restdaten ignoriert.

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

## 11. Ausführung, Freeze und offene Rootentscheidungen

Reproduktion ab einem frischen Area aus dem Workspace:

```text
python3 .codex-tmp/gen06_1/sources/collect.py
python3 .codex-tmp/gen06_1/sources/supplement.py
python3 .codex-tmp/gen06_1/oracle/prepare.py
python3 .codex-tmp/gen06_1/oracle/run.py .codex-tmp/gen06_1/final-normal
python3 -O .codex-tmp/gen06_1/oracle/run.py .codex-tmp/gen06_1/final-optimized
python3 .codex-tmp/gen06_1/oracle/check_guards.py .codex-tmp/gen06_1/guards-normal .codex-tmp/gen06_1/final-normal
python3 -O .codex-tmp/gen06_1/oracle/check_guards.py .codex-tmp/gen06_1/guards-optimized .codex-tmp/gen06_1/final-normal
python3 .codex-tmp/gen06_1/oracle/fixture.py generate .codex-tmp/gen06_1/final-normal .codex-tmp/gen06_1/fixture-normal
python3 -O .codex-tmp/gen06_1/oracle/fixture.py generate .codex-tmp/gen06_1/final-optimized .codex-tmp/gen06_1/fixture-optimized
python3 .codex-tmp/gen06_1/oracle/fixture.py validate .codex-tmp/gen06_1/final-normal .codex-tmp/gen06_1/fixture-normal > .codex-tmp/gen06_1/delivery/validate-normal.json
python3 -O .codex-tmp/gen06_1/oracle/fixture.py validate .codex-tmp/gen06_1/final-normal .codex-tmp/gen06_1/fixture-normal > .codex-tmp/gen06_1/delivery/validate-optimized.json
python3 .codex-tmp/gen06_1/oracle/fixture.py mutations .codex-tmp/gen06_1/final-normal .codex-tmp/gen06_1/fixture-normal .codex-tmp/gen06_1/mutations-normal
python3 -O .codex-tmp/gen06_1/oracle/fixture.py mutations .codex-tmp/gen06_1/final-normal .codex-tmp/gen06_1/fixture-normal .codex-tmp/gen06_1/mutations-optimized
python3 .codex-tmp/gen06_1/delivery/analysis.py
python3 .codex-tmp/gen06_1/delivery/freeze.py
```

Keine frische Ausgabe überschreibt alte. `scratch01..scratch13`, `scratch-guards*`, `scratch-fixture*` und `scratch-mutations*` bleiben ausdrücklich
nichtautoritative Entwicklungsläufe und sind aus dem Freeze ausgeschlossen.
`delivery/freeze-manifest.json` enumeriert nur Vertrag, Quellen/Ressourcen/Pins/Werkzeuge,
beide frischen Originalexports einschließlich explizit diagnostischer Logs, beide
Fixture-Exporte, Validierungen, Guards, Mutationsrefusals und Endauswertung. SHA-256
für Vertrag/Manifest werden Root getrennt übergeben. Die Manifestdatei hasht sich
nicht selbst. Freezeprüfer verlangt Inputs erneut unverändert, normalen/-O-Export
bytegleich, Fixturebytegleich, gleiche Guards bis auf Pythonmode und sämtliche
malformed Refusals. Keine Hashliste über pauschal alles im Area und keine stale Sourcepins.

Offene Rootentscheidungen nach unabhängigem Contract-/Oracle-Review:

1. Den empfohlenen **Primitiveticket** als nächsten World-only Schritt wählen;
   Carverannahme ist dafür keine notwendige Runtimevoraussetzung, aber bleibt Gate für
   spätere echte Regioninputtypen/Stageconsumer.
2. World-eigener geschlossener Builtinkatalog oder zunächst getrennte generische
   Registry-/Tagproducer. Empfehlung: kleiner Worldkatalog für den endlichen Scope,
   keine neue Registrylane im Primitiveticket.
3. Proposed Budget-/Bytecaps, dynamische RNG-exhaustion und Poison-/Publikationspolicy
   ausdrücklich übernehmen oder vor Vergabe konkret ändern. Die hier genannten Native-
   Caps sind ungemessene Designwerte, keine bereits erwiesene Nullallokation.
4. Paper-default-only initial profile, ohne Featureseed-Overrides/Populators/Blending;
   unterstützte Alternativen benötigen neue echte Originalevidenz und klaren Scope.
5. Originalserver-Harness für whole FEATURES und Determinismus-/Mehrchunk-Adoption
   separat beauftragen. Keine automatische Runtimeautorisierung aus diesem Freeze.

Ausgeführt wurden keine Rustproduktionstests, da keine Produktion geändert wurde.
Geliefert ist ausschließlich die reproduzierbare, kontrollierte Original-
Vorbereitung; die unabhängige Rootannahme steht aus. Die aktive App bleibt Flat;
Naturalworld, Survival, Strukturen, Beleuchtung und Gameplay sind dadurch nicht aktiviert.
