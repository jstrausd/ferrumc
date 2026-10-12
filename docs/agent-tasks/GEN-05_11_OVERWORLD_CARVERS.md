# GEN-05.11 — Overworld-Carver: konkreter Vertrag und echtes Original-Oracle

Vorbereitungslieferung vom 12.10.2026 zur [Workorder](GEN-05_11_OVERWORLD_CARVERS_TASK.md) (SHA-256
`1aaa00ea…097d`, Main `f57844d`). Das Review von Vertrag und Oracle gehört Root; dieses Dokument
autorisiert keine Runtime. Geschrieben wurden nur dieses Dokument und `.codex-tmp/gen05_11/**`. Kein Git,
keine Produktionsänderung, keine neue Abhängigkeit, kein weiterer Agent, keine Änderung an historischen
GEN-05.x-Artefakten. Die Design-Probe lief nur in einer privaten Archivkopie mit eigenem Target (`--jobs 2`).

Kennzeichnungen im ganzen Dokument: **Source-Review** (gelesen, nicht ausgeführt: §2.1, `sources/`),
**Differentialevidenz** (echte Original-Ausführung: §2.2, `final-*`), **Vertragsmodellprüfung (abgeleitet)**
(ein zweiter Weg, der den Vertragstext gegen echte Writes nachrechnet: §2.4) und **Design-Probe**
(FerrumC-Produzenten gegen echte Werte in einer privaten Kopie: §2.5) und **Grenzprobe** (echte
Originalmethoden mit skriptgesteuerten Extremziehungen, nie ein echter Chunkzustand: §2.6). Eine **FerrumC-Policy** (C*n*, oder
geerbt P*n*/D*n* aus GEN-05.9/05.10) wird nie als Originalverhalten ausgegeben.

## 1. Basis, Scope und konsumierte Produzenten

Basis ist die World-Crate im Arbeitsbaum auf Main `f57844d`. `oracle/prepare.py` vergleicht sie Datei für
Datei mit dem akzeptierten GEN-05.10-Runtime-Freeze (`.codex-tmp/gen05_10-runtime/source.freeze.json`,
324/324 gleich, Mitgliedschaft gleich; nur vier nicht versionierte `__pycache__`-Dateien liegen zusätzlich im
Baum und sind ausgenommen). Die laufenden Storage-Änderungen eines anderen Agents im gemeinsamen Baum wurden
weder gelesen noch berührt. Gepinnte Originalartefakte sind die der akzeptierten GEN-05.10-Pins
(`gen05_10/oracle/input-pins.json`), byteweise neu geprüft: Vanilla 1.21.8 Inner-JAR `2bee159e…0250` mit 44
Bibliotheken, Paper Build 60 (Commit `29c8822d`) Inner-JAR `c64fb7db…1507` mit 119 Bibliotheken, das komplette
JDK 21.0.12.1 (454 Dateien) und die offiziellen `server-mappings.txt`. Der akzeptierte GEN-05.10-Oracle-Freeze
(`ce970306…887d`, 321 Einträge) wird vor jedem Lauf vollständig nachgehasht; seine Fixed-Groups, die
Scan-Auswahl und die finalen Snapshots sind gepinnte Eingaben. `oracle/input-pins.json` enthält 1133
Eingaben, SHA-256 `27427e158bc05fa8ec6bf30577c54b0fdee2d38049cdbbd6db866fb3748e4837`.

Scope: der originale CARVERS-Schritt des Overworld-Generators — `NoiseBasedChunkGenerator.applyCarvers` mit
der 17×17-Quellschleife, dem Seeding, den drei konfigurierten Carvern (`cave`, `cave_extra_underground`,
`canyon`), `CarvingMask`, Ersetzungsregel, Lava-Level, Aquifer-Entscheidung, Fluid-Flag, Oberflächen-Fix-up
(`SurfaceSystem.topMaterial`), allen Writes mit Heightmap- und Post-Processing-Wirkung. Außerhalb
(Workorder): Features, Strukturen, Licht, Persistenz oder Aktivierung der natürlichen Welt, Nether- und
End-Carver, Survival, Java-Plugins.

Konsumierte akzeptierte Produzenten (alle unverändert; §2.5 zeigt, dass jeder die echte Carver-Eingabe für
jede zugelassene Oracle-Gruppe liefert):

| Produzent | Nutzung im Carver |
| --- | --- |
| GEN-05.10 `SeededSurfaceColumns::build_chunk` → `SurfaceChunkBuffer` (`Published`) | der Post-SURFACE-Chunk (Zustände, `WORLD_SURFACE_WG`, Fluid-Flags) als einzige Chunk-Eingabe; Identität (Chunk, Seed, Settings) aus `published()` |
| GEN-05.4 Aquifer (`density::chunk::material::aquifer::Aquifer::compute`) in einer frischen `NoiseChunkSession` desselben Chunks | `computeSubstance(pos, 0.0)` an jeder Carver-Position oberhalb des Lava-Levels und das `schedule`-Flag danach (Policy C2) |
| GEN-05.9 `SurfaceRuleSession` (`begin_column`, `evaluate`) mit `SeededSurfaceRules` | der Fix-up `topMaterial` an `(x, y−1, z)` mit Stone-Depth 1/1 und Wasserhöhe `Some(y)` oder `None` |
| GEN-05.8 `SurfaceBiomeResolver::biome_at_block` | das Biom der Fix-up-Regeln (Policy C5) |
| GEN-05.3 `PreliminarySurfaceCorners` (wie in GEN-05.10, A2 aufrufervertraut) | Mindest-Oberfläche der Fix-up-Regeln |
| GEN-04 `random::WorldgenRandomSource::legacy` | `LegacyRandomSource`/`WorldgenRandom`: Seeding, `nextInt`, `nextFloat`, `nextLong` |
| Registry `block_state::{block_metadata, compute_state_id}` | das geschlossene Zustandsalphabet (GEN-05.10 D1) und die Klassifikation nach Namen |

Nicht konsumiert: GEN-05.5/05.6/05.7 direkt (die Carver-Biomwahl entfällt nach Policy C1; GEN-05.7 erreicht
den Carver nur indirekt über die GEN-05.9-Regeln), keine Noise-Sampler über den Aquifer hinaus.

## 2. Originalfakten

### 2.1 Source-Review (gelesen, nicht ausgeführt)

`sources/collect.py` kopiert ganze Base-Dateien (paperweight mache = Vanilla 1.21.8 dekompiliert) und
Applied-Dateien (Paper bei `29c8822d`) der Closure, jeden Pro-Datei-Patch, die drei Feature-Patches, die die
Closure berühren (`0004-Anti-Xray`, `0012-Optimize-Bit-Operations-by-inlining`,
`0016-Moonrise-optimisation-patches`), Moonrises `ChunkUpgradeGenericStatusTask` und `NewChunkHolder` sowie
den Overworld-Ressourcenpatch: 131 Dateien, byteweise gepinnt in `sources/source-map.json`. Zeilennummern
beziehen sich auf `sources/copies/applied/…`.

1. **Unverändert.** `WorldCarver`, `CaveWorldCarver`, `CanyonWorldCarver`, `NetherWorldCarver`,
   `ConfiguredWorldCarver`, alle Carver-Konfigurationen, `CarverDebugSettings`, `CarvingMask`, `Aquifer`,
   `NoiseChunk`, `DensityFunction`, `WorldgenRandom`, `LegacyRandomSource`, `BitRandomSource`,
   `RandomSupport`, `RandomSource`, `UniformHeight`, `HeightProvider`, `VerticalAnchor`, alle Float-Provider,
   `Mth`, `QuartPos`, `BiomeGenerationSettings`, `MultiNoiseBiomeSource`, `Climate`, `SurfaceSystem`,
   `SurfaceRules`, `Heightmap`, `Blender` und `BlockTags` sind in Base und Applied byte-identisch.
2. **Paper-Änderungen in der Closure ändern die Carver-Semantik nicht.**
   - `CarvingContext` (Pro-Datei-Patch): ein zusätzlicher `@Nullable Level`-Parameter für den
     Flat-Bedrock-Träger (GEN-05.9 P2); `NoiseBasedChunkGenerator.applyCarvers` (263) übergibt
     `level.getMinecraftWorld()`.
   - `NoiseBasedChunkGenerator`: synchrone `Runnable::run`-Executors für Biome und Noise.
   - `ChunkStatusTasks.generateCarvers` (120-139): nur eine `instanceof ProtoChunk`-Prüfung vor
     `Blender.addAroundOldChunksCarvingMaskFilter`.
   - `WorldGenRegion`, `GenerationChunkHolder`, `ChunkPyramid`, `ChunkStep` (Moonrise): `getChunk(x, z,
     status, require)` (143-180) ist unverändert; `getChunkIfPresentUnchecked` delegiert an Moonrises
     `NewChunkHolder.chunkCompletions` (474-477).
   - `ChunkPos` (`longKey`), `SectionPos`, `BlockPos` (Bit-Inlining), `ProtoChunk`/`ChunkAccess`
     (Moonrise-Lichtfelder, Anti-Xray), `BiomeManager` (Moonrise-Zoom), `TagLoader` (Paper-Tag-Events),
     `BlockBehaviour`: keine dieser Änderungen berührt einen Wert, den der Carver liest oder schreibt.
3. **CARVERS-Schritt.** `ChunkPyramid` (31-34): CARVERS verlangt `STRUCTURE_STARTS` im Radius 8,
   Block-Write-Radius 0. `generateCarvers` baut eine `WorldGenRegion`, ruft
   `Blender.addAroundOldChunksCarvingMaskFilter` (setzt nur bei alten/blendenden Nachbarn eine Zusatzmaske;
   für frische Welten ein No-op) und dann `applyCarvers(region, level.getSeed(), randomState,
   level.getBiomeManager(), structureManager, chunk)`. Die persistierte Statusstufe während des Schritts
   ist SURFACE, deren `heightmapsAfter` die beiden WG-Heightmaps sind (`ChunkStatus` 17-27).
4. **`applyCarvers`** (`NoiseBasedChunkGenerator` 253-291): `biomeManager1 =
   biomeManager.withDifferentSource((x, y, z) -> biomeSource.getNoiseBiome(x, y, z, sampler))`; ein
   `WorldgenRandom` über `LegacyRandomSource`; der vom NOISE-Schritt zurückgehaltene `NoiseChunk`
   (`getOrCreateNoiseChunk`; nur wenn er fehlt, wird er mit `Blender.of(region)` neu erzeugt) und dessen
   `aquifer()`; ein `CarvingContext` mit `settings.surfaceRule()`; `((ProtoChunk) chunk).getOrCreateCarvingMask()`.
   Schleife `i1 = −8..8` (außen, X), `i2 = −8..8` (innen, Z): Quell-Chunk `(cx + i1, cz + i2)`,
   `region.getChunk(...).carverBiome(() -> getBiomeGenerationSettings(biomeSource.getNoiseBiome(
   QuartPos.fromBlock(minX), 0, QuartPos.fromBlock(minZ), sampler)))` (pro Quell-Chunk gecacht), dann für
   jeden Carver der Liste mit Index `i3`: `setLargeFeatureSeed(seed + i3, sx, sz)`, `isStartChunk`,
   bei Start `carve(context, chunk, biomeManager1::getBiome, random, aquifer, sourcePos, mask)`.
5. **Carver** (`WorldCarver` 61-221, `CaveWorldCarver` 27-238, `CanyonWorldCarver` 26-138): §3-§5 geben den
   vollständigen Ablauf als Vertrag wieder. `isStartChunk` ist `nextFloat() <= probability`.
   `carveEllipsoid` iteriert lokale X außen, Z innen, Y absteigend; `carveBlock` setzt `reachedSurface`
   bei `grass_block` oder `mycelium`, ersetzt nur Tag-Zustände, wählt Lava bis `lavaLevel`, sonst
   `aquifer.computeSubstance(SinglePointContext(x, y, z), 0.0)`; auf `null` folgt kein Write. Nach dem Write
   liest es `aquifer.shouldScheduleFluidUpdate()` und markiert Fluid-Writes für das Post-Processing; der
   Fix-up läuft, wenn `reachedSurface` gesetzt ist und unter der Position `dirt` liegt.
6. **Writes** (`ProtoChunk` 136-190, 264-286): Section-Write; für jede Heightmap von
   `getPersistedStatus().heightmapsAfter()` (bei CARVERS: SURFACE → `WORLD_SURFACE_WG`, `OCEAN_FLOOR_WG`)
   `Heightmap.update` (81-108); `markPosForPostprocessing` hängt `x | (y & 15) << 4 | z << 8` an die Liste
   der Section `(y >> 4) + 4` an, ohne Duplikatprüfung. Kein Block-Entity wird erzeugt oder entfernt.
7. **Fix-up** (`SurfaceSystem.topMaterial` 183-203): ein frischer `SurfaceRules.Context` pro Aufruf,
   `updateXZ(x, z)`, `updateY(1, 1, hasFluid ? y + 1 : Integer.MIN_VALUE, x, y, z)`, `tryApply`; das Biom
   kommt aus `biomeManager1::getBiome` (Direktquelle mit Zoom), die Steep-Höhen live aus dem Chunk.
8. **Registrypfad.** `VanillaRegistries.createLookup` liefert für den `replaceable`-Tag der Carver nur einen
   Konstruktions-Platzhalter (`RegistrySetBuilder` 306-317, `HolderSet.emptyNamed`), der beim
   Dereferenzieren wirft. Ein echter Server lädt die Worldgen-Registries aus dem Datapack
   (`WorldLoader.load` 38-48, `ReloadableServerResources.updateStaticRegistryTags`). Das Oracle folgt
   deshalb genau diesem Pfad (§2.2); GEN-05.9/05.10 nutzten `createLookup`, was für Oberflächenregeln ohne
   Tags genügt. §2.2 belegt, dass beide Pfade dieselben Post-SURFACE-Chunks liefern.

### 2.2 Differentialevidenz (echte Original-Ausführung)

Der JDK-only-Helfer `oracle/CarverOracle.java` steuert echte Originalobjekte per Reflection (Vanilla über die
gepinnten offiziellen Mappings, Paper über Mojang-Namen) und prüft im JVM die CodeSource jeder Originalklasse,
jeder Bibliothek und des frisch kompilierten Helfers. Er implementiert keine Carver-, Random-, Aquifer-,
Masken-, Regel-, Biom- oder Zoom-Logik. Ein Lauf hat drei JVM-Stufen:

1. **scan (Paper, nur Auswahl):** alle 72 Basisgruppen und Kandidaten in Chebyshev-Ringen 1..3 um deklarierte
   Anker für jedes der 23 deklarierten Selektionsprädikate (`design/features.tsv`); 263 echte Auswertungen,
   0 Fehler. Ausgewählt wurde eine neue Gruppe (`feature/top_fluid`, Chunk (3, 19)). Die Auswahl nutzt
   Originalausgaben; jede Erwartung bleibt ein Ergebnis der Stufen 2 und 3.
2. **vanilla** und 3. **paper:** 73 Gruppen = die 72 akzeptierten GEN-05.10-Gruppen (16 feste, 51 Biom-,
   3 Feature-, 2 Naht-Gruppen, unverändert) + 1 Scan-Gruppe. Registrypfad: `ServerPacksSource.
   createVanillaPackSource()`, `MultiPackResourceManager(SERVER_DATA)`, `RegistryLayer.createRegistryAccess()`,
   `TagLoader.loadTagsForExistingRegistries`, `buildUpdatedLookups`, `RegistryDataLoader.load(…,
   WORLDGEN_REGISTRIES)`, danach `PendingTags.apply()` — wörtlich `WorldLoader` 38-48. Pro Gruppe **vier
   unabhängige echte Generierungen** bis SURFACE nach der GEN-05.10-Disziplin (`WorldPreset`-Generator,
   `RandomState.create`, neun `ProtoChunk`s (−64, 384), kalte RTree-Historie, `createBiomes` zeilenweise,
   echtes `fillFromNoise`, der akzeptierte GEN-05.10-U-Pfad `buildSurface`, Status SURFACE). Die vier
   Post-SURFACE-Snapshots müssen gleich sein. Region: eine allozierte `WorldGenRegion` mit genau den vier
   Feldern, die `applyCarvers` liest (`center`, `generatingStep = GENERATION_PYRAMID.getStepTo(CARVERS)`,
   `cache = StaticCache2D.create(cx, cz, 8, …)`, `level`), über echten `ChunkHolder`-Objekten (Vanilla:
   `GenerationChunkHolder.futures`, Paper: `NewChunkHolder.chunkCompletions`); sie liefern das
   Post-SURFACE-Zentrum, die acht BIOMES-Nachbarn und 280 frische `ProtoChunk`s mit Status `STRUCTURE_STARTS`
   (bei `generateStructures = false` lässt dieser Schritt einen Chunk unberührt). Level-Träger: ein
   alloziertes `ServerLevel` mit `registryAccess` = geladene Worldgen-Registries, in Paper zusätzlich
   `generateFlatBedrock = false` (P2). Jeder Carver-Lauf beginnt mit kalter RTree-Historie im aufrufenden
   Thread.
   - **U:** echtes `applyCarvers` mit dem zurückgehaltenen NOISE-`NoiseChunk` und dessen Aquifer.
   - **F:** echtes `applyCarvers`, nachdem der zurückgehaltene `NoiseChunk` durch einen echten frischen
     `createNoiseChunk(chunk, structures, Blender.empty(), randomState)` ersetzt wurde (Reload-Pfad).
   - **R:** echtes `applyCarvers` mit drei transparenten Proxys: der Aquifer des `NoiseChunk` (zeichnet
     jedes `computeSubstance` mit Position, Dichte, Ergebnis und Flag danach sowie jedes
     `shouldScheduleFluidUpdate` auf), die `surfaceRule` der Settings (eine Recordkopie mit genau einer
     ersetzten Komponente; zeichnet jedes Fix-up-`tryApply` mit Kontextfeldern auf) und der
     `generationSettingsGetter` des Generators (zeichnet jede Carver-Biomauflösung auf). Der
     Preset-Generator ist zwischen den Generierungen geteilt; seine Felder werden nach jedem
     aufgezeichneten Lauf wiederhergestellt und vor dem nächsten geprüft.
   - **D:** die deklarierte 17×17-Schleife als Folge echter Aufrufe (`WorldGenRegion.getChunk`,
     `ChunkAccess.carverBiome`, `setLargeFeatureSeed`, `isStartChunk`, `ConfiguredWorldCarver.carve` mit
     echtem `CarvingContext` und `withDifferentSource`-Accessor) unter denselben Proxys; pro gestartetem
     Carver werden neue Maskenbits und Netto-Writes zugeordnet. D muss U bitgleich reproduzieren, und die
     Ereignisfolge von D (ohne Carver-Marker) muss der von R gleichen.
   - **Diagnostik** (nebenwirkungsfrei, danach): pro Fix-up das direkt gezoomte und das über die
     gespeicherten Arrays gezoomte Biom; die echte Tag-Expansion; die echten Carver-Konfigurationen (Floats
     als Bits, Anker über das echte `resolveY`); die Carver-Liste jedes der 65 Biome; die echte
     `Mth.SIN`-Tabelle.

Welcher Originalcode jede Carver-Eingabe erzeugte:

| Eingabe | Originalproduzent | Aufgezeichnet |
| --- | --- | --- |
| Post-SURFACE-Chunk, beide WG-Heightmaps, Post-Processing | `fillFromNoise` + GEN-05.10-U-Pfad `buildSurface` | voller Snapshot S1 von U, R, D, F (gleich) |
| Carver-Liste pro Quell-Chunk | `ChunkAccess.carverBiome` → `getBiomeGenerationSettings(getNoiseBiome(quart(minX), 0, quart(minZ)))` | R-Ereignis `B` (Biomindex) pro Quell-Chunk; gecachte Einstellung jedes der 289 Chunks nach U |
| Konfiguration (Wahrscheinlichkeit, Höhen, Provider, Lava-Level, Tag) | datapack-geladenes `ConfiguredWorldCarver` | `carvers.tsv` (Floats als Bits) |
| Replaceable-Tag | `TagLoader` über das Vanilla-Datapack | `tag.tsv` (52 Blöcke) und `is(tag)` pro beobachtetem Zustand |
| Start pro (Quell-Chunk, Index) | `setLargeFeatureSeed` + `isStartChunk` | D-Marker; `starts.tsv` (867 Flags pro Gruppe) |
| Aquifer-Ergebnis und Flag | `NoiseBasedAquifer.computeSubstance` | R-Ereignis `A x,y,z,density,state,flag` |
| Flag-Lesung nach jedem Write | `shouldScheduleFluidUpdate` | R-Ereignis `S` |
| Fix-up-Ergebnis | `SurfaceSystem.topMaterial` → genuine Regel | R-Ereignis `T x,y,z,water,depth,above,below,state` |
| Carving-Mask | `CarvingMask` | `toArray()` nach U/R/D/F |
| Zuordnung pro Carver | Folge echter `carve`-Aufrufe | D (`carves.tsv`) |
| finaler Chunk, Heightmaps, Post-Processing, Block-Entities | `ProtoChunk.setBlockState`, `Heightmap.update`, `markPosForPostprocessing` | voller Snapshot FINAL |

**Ergebnisse** (frischer Lauf `final-normal`; `final-optimized` (`python3 -O`) ist bytegleich, §17):

| Größe | Vanilla | Paper |
| --- | --- | --- |
| Gruppen / Ausnahmen (Generierung, U, R, D, F) | 73 / 0 | 73 / 0 |
| S1 gleich in U, R, D, F | 73 / 73 | 73 / 73 |
| FINAL U = R = D | 73 / 73 | 73 / 73 |
| FINAL U = F (frischer NoiseChunk) | 73 / 73 | 73 / 73 |
| Ereignisfolge R = D | 73 / 73 | 73 / 73 |
| Nachbar-Chunks unverändert (Sections, Post-Processing, keine Maske) | 73 / 73 | 73 / 73 |
| Aquifer-Flag am Ende von NOISE / im frischen NoiseChunk | `false` 73 / `false` 73 | `false` 73 / `false` 73 |
| Starttests / Starts (cave, extra, canyon) | 63 291 / 4807 (2943, 1655, 209) | gleich |
| Carver mit Writes im Ziel (cave, extra, canyon) | 148 (89, 53, 6) | gleich |
| Maskenbits / Gruppen mit Maske | 65 198 / 67 | gleich |
| Aquifer-Aufrufe: Luft / Wasser / Lava / `null` | 26 628 / 14 306 / 115 / 1597 | gleich |
| Lava-Level-Writes / davon mit veraltetem Flag markiert | 500 / 4 | gleich |
| Flag-Lesungen / Writes / davon Zustandsänderungen | 41 549 / 41 904 / 38 940 | gleich |
| Fix-ups (ohne / mit Fluid) / Ergebnis `grass_block` / `dirt` / leer | 355 (322 / 33) / 322 / 33 / 0 | gleich |
| Post-Processing-Marken des Carvers / Gruppen mit Duplikaten | 618 / 6 | gleich |
| Zustände / Tag-Blöcke | 32 / 52 | 32 / 52 |

Abdeckung (Paper): `hit_cave` 53, `hit_cave_extra_underground` 44, `hit_canyon` 6 Gruppen; Quellen aus
Nachbar-Chunks 62, aus Chebyshev-Abstand ≥ 5 2, mit negativen Koordinaten 52; Aquifer Luft 58, Wasser 38,
Lava 8, Barriere 27; Lava-Level 9; veraltetes Flag 1; Wasser im Carver-Bereich (unter Ozeanen/Flüssen) 13;
Gras erreicht 6; Fix-up trocken 6, mit Fluid 1; Oberfläche gesenkt 8; Carver-Marken 27; Duplikat-Marken 6;
Maskenbits auf nicht ersetzbaren Zuständen 53 Gruppen. 6 Gruppen haben keinen Carver-Write im Ziel (der
Quelllauf mit Starts, Tunnelschritten und Ellipsoiden findet trotzdem statt). Presets normal, large_biomes,
amplified und single_biome_surface; Seeds 0, −1, `i64::MIN`, `i64::MAX`; negative Koordinaten in 50 Gruppen;
Nähte: in 6 benachbarten Gruppenpaaren schreiben dieselben (Quell-Chunk, Carver) in beide Chunks.
Deklariert, aber nach der deklarierten Suche absent (ehrliche Lücke, §15): `mycelium_carved` (48
Kandidaten), `top_empty` (96 Kandidaten).

**Registrypfad.** Die Post-SURFACE-Chunks des Datapack-Pfads sind in allen 72 gemeinsamen Gruppen gleich
den akzeptierten GEN-05.10-FINAL-Snapshots (Zustände nach Name, beide Heightmaps, Post-Processing;
`report.s1_equals_gen05_10_final`).

**Carver-Listen.** Alle 54 Overworld-Biome (51 Oberflächen- und 3 Höhlenbiome) listen genau
`[minecraft:cave, minecraft:cave_extra_underground, minecraft:canyon]` in dieser Reihenfolge; 5 Nether-Biome
`nether_cave`, 6 End/Void-Biome nichts. Jede der 73 Gruppen löste für alle 289 Quell-Chunks genau diese Liste
auf.

### 2.3 Vanilla gegen Paper

Ergebnis (`delivery/analysis-normal.json`): **Der Carver-Schritt ist in Vanilla und Paper identisch.**
Starts, Ereignisfolgen (jeder Aquifer-Aufruf mit Ergebnis und Flag, jede Flag-Lesung, jeder Fix-up), die
Zuordnung pro Carver, die Diagnostik, Konfigurationen, Tag und Sinustabelle sind bytegleich; das
Carver-Delta (Positionen, an denen FINAL von S1 abweicht, mit Endzustand) und die Masken sind in 73/73
Gruppen gleich. Die Chunk-Zeilen unterscheiden sich in 25 Gruppen ausschließlich, weil S1 abweicht — das sind
die bekannten NOISE/SURFACE-Unterschiede aus GEN-05.10 §2.2. Die f32-Rundung des Paper-`nextDouble`
(GEN-05.9/05.10) betrifft den Carver nicht: er zieht nur `nextInt`, `nextFloat` und `nextLong`.

Deshalb sind **keine Tests gerechtfertigt, die einen Vanilla/Paper-Unterschied im Carver voraussetzen.**
Paper-Zellen sind die Erwartung (P1); Vanilla bleibt Kontrolle.

### 2.4 Vertragsmodellprüfung (abgeleitet)

`oracle/project_originals.py` spielt den Vertragstext (§3-§7) auf jeder Gruppe beider Originale nach und
vergleicht mit den echten Writes. Es liefert nie einen Oracle-Wert: Aquifer- und Fix-up-Ergebnisse nimmt
es aus den echten Aufrufen, aber erst nachdem es geprüft hat, dass es an genau dieser Stelle der Folge genau
diese Position anfragt. Nachgerechnet werden: die 17×17-Schleife, das Seeding, alle 63 291 Starttests,
die Höhlen- und Canyon-Geometrie mit Java-`float`/`double`/`int`-Semantik und der echten Sinustabelle, die
Maske, die Ersetzungsregel nach Blocknamen aus `tag.tsv`, das Lava-Level, Position und Reihenfolge jedes
Aquifer-Aufrufs, jede Flag-Lesung (Wert = Flag des letzten Aquifer-Aufrufs, initial `false`), der
Fix-up-Auslöser, alle Writes, beide WG-Heightmaps über die originale `update`-Regel, die geordneten
Post-Processing-Listen und die Zuordnung pro Carver. **0 Abweichungen** in beiden Originalen (73 Gruppen;
Paper: 205 820 Tunnelschritte, 143 454 Ellipsoid-Aufrufe, 65 198 Maskenbits, 42 646 Aquifer-Aufrufe,
41 549 Flag-Lesungen, 355 Fix-ups). Die Namensklassifikation (Luft, Fluid, Tag) stimmt mit den echten Flags
auf allen 32 beobachteten Zuständen überein.

### 2.5 Design-Probe (private Archivkopie; nur Design)

`probe/run_probe.py` kopiert Roots eingefrorenes Archiv `.codex-tmp/gen05_10-runtime-review-base/archive`
(`base.freeze.json` `106ad3f3…442e`, 1431 Dateien; seine World-Crate ist 324/324 gleich der aktuellen und
dem akzeptierten Runtime-Freeze), fügt genau ein `#[cfg(test)]`-Modul `carver_design_probe.rs` unter
`density/chunk/material/` und dessen Registrierung in `material.rs` hinzu, verweigert jede andere Abweichung
und führt Debug und Release mit eigenem Target aus. Beide Profile liefern dieselben Werte
(`probe/final/receipt.json`).

| Prüfung | gleich / verglichen |
| --- | --- |
| GEN-05.4 `Aquifer::compute(…, 0.0)` in frischer Session, Aufrufe in echter Reihenfolge: Zustand | 42 646 / 42 646 |
| dasselbe: `schedule`-Flag nach jedem Aufruf | 42 646 / 42 646 |
| davon Luft / Wasser / Lava / `null` | 26 628 / 14 306 / 115 / 1597, je alle gleich |
| `WorldgenRandomSource::legacy`: `setLargeFeatureSeed` + `next_float() <= p` gegen jeden echten Starttest | 63 291 / 63 291 |
| `(f64::sin(i · π · 2 / 65536)) as f32` gegen die echte `Mth.SIN`-Tabelle | 65 536 / 65 536 |

Designzahlen: maximale GEN-05.4-Arbeit eines Aquifer-Aufrufs 36 293 Einheiten (Cache-Fehlgriffe mit
Fluidstatus-Berechnung); `size_of::<Aquifer>()` 80 B (plus Caches ≤ 12 KiB, GEN-05.4).

Ohne Probe, aber durch Komposition belegt: der GEN-05.10-Treiber reproduziert die akzeptierten
GEN-05.10-FINAL-Snapshots (Runtime-Review CLEAN_RUNTIME, 70/70), und diese sind S1 dieses Oracles (72/72).
Die GEN-05.9-Regeln an den 355 Fix-up-Eingaben sind nicht in der Probe ausgeführt; sie sind Bedingung C2 der
Runtime (§16). Korpusbefund (keine allgemeine Parität): an allen 355 echten Fix-ups dieses Korpus gleicht
das direkt gezoomte Biom dem über die gespeicherten Arrays gezoomten (`tops.tsv`); die offene
Abweichungsdomäne steht bei C5 (§8).

### 2.6 Grenzprobe (skriptgesteuerte Ziehungen; kein echter Chunkzustand)

`boundary/run_boundary.py` ruft in Vanilla und Paper echte Originalmethoden mit einer skriptgesteuerten
`RandomSource` auf, die nur deklarierte Ziehungen liefert (jede andere Ziehung verweigert sie):
`UniformFloat.of(lo, hi).sample`, `TrapezoidFloat.of(0, 6, 2).sample`, `Mth.randomBetween(0.75f, 1.0f)`,
`CaveWorldCarver.getThickness` am registrierten `cave`-Carver und `Mth.sin((float)(π/2))`; dazu den
Bytecode von `CanyonWorldCarver.carve` aus beiden Inner-JARs. Ergebnisse (`boundary/final/boundary.json`,
Vanilla = Paper), jeweils bei `nextFloat = (2²⁴ − 1)/2²⁴` (`3f7fffff`):

| Methode | Ergebnis | Bezug zur Obergrenze |
| --- | --- | --- |
| `UniformFloat(0.75f, 1.0f)` (Canyon `distance_factor`, `horizontal_radius_factor`) | `1.0` (`3f800000`) | **erreicht** die Obergrenze |
| `Mth.randomBetween(0.75f, 1.0f)` (Canyon `updateVerticalRadius`) | `1.0` | **erreicht** |
| `UniformFloat(0.8f, 1.3f)` (`vertical_radius_multiplier`) | `1.3f` (`3fa66666`) | **erreicht** |
| `UniformFloat(0.7f, 1.4f)` (`horizontal_radius_multiplier`) | `1.3999999` (`3fb33332`) | darunter |
| `UniformFloat(0.1f, 0.9f)` (`yScale`) | `0.8999999` (`3f666665`) | darunter |
| `UniformFloat(−1.0f, −0.4f)` (`floor_level`) | `−0.40000004` | darunter |
| `UniformFloat(−0.125f, 0.125f)` (`vertical_rotation`) | `0.124999985` | darunter |
| `TrapezoidFloat(0, 6, 2)` (Canyon `thickness`, zwei Maximalziehungen) | `5.9999995` (`40bfffff`) | darunter |
| `getThickness` ohne / mit Bonus (`nextInt(10)` = 1 / 0) | `2.9999998` / `11.999998` | darunter 3 / 12 |
| `Mth.sin((float)(π/2))` | `1.0` | — |

Der Canyon-Bytecode enthält genau einmal `i2f; aload_2; getfield shape; getfield distanceFactor; aload;
invokevirtual FloatProvider.sample; fmul; f2i`, also `count = (int)((float) i · factor)` mit `i = 112`.
Folge: **`count` kann 112 erreichen**, nicht nur 111 (Root-Vorprüfung, hier am Original reproduziert).
Die Echtläufe zeigen den Rand nicht; er ist eine Eigenschaft der Ziehungsfunktion. Alle anderen
§10-Schranken, die auf `<` beruhen, sind damit am Original geprüft (§10).

## 3. Carver-Schleife und Seeding (Vertrag)

Für einen zugelassenen Ziel-Chunk `(cx, cz)` mit Weltseed `s` (Java-`long`, wrapping):

1. `random = WorldgenRandom(LegacyRandomSource)`; der Anfangsseed ist irrelevant, weil vor jedem Test neu
   gesetzt wird. `fluidFlag = false` (Policy C2/C8). Die Maske ist leer.
2. Für `i1 = −8..=8` (außen), `i2 = −8..=8` (innen): Quell-Chunk `(sx, sz) = (cx + i1, cz + i2)`. Die
   Carver-Liste ist `[cave, cave_extra_underground, canyon]` (Policy C1; keine Biomabfrage). Für jeden
   Carver mit Index `k` (0, 1, 2) in dieser Reihenfolge:
   1. `setLargeFeatureSeed(s + k, sx, sz)`: `setSeed(base)`; `a = nextLong()`; `b = nextLong()`;
      `setSeed((sx · a) ^ (sz · b) ^ base)` mit `long`-Wrapping, `sx`/`sz` auf `long` erweitert.
   2. Start, wenn `nextFloat() <= p` (`float`-Vergleich; `p` = 0.15f, 0.07f, 0.01f; Bits
      `3e19999a`, `3d8f5c29`, `3c23d70a`).
   3. Bei Start läuft der Carver mit **demselben** `random` weiter (§4); alle Writes landen nur im Ziel-Chunk.
3. Eine Quelle erreicht das Ziel nur über die Geometrie: Ellipsoide außerhalb `|x − (16cx + 8)| ≤ 16 + 2hr`
   und `|z − (16cz + 8)| ≤ 16 + 2hr` tun nichts; Tunnel enden über `canReach` (§4.4). Die 289 Quellen und
   867 Starttests werden immer vollständig durchlaufen.

`LegacyRandomSource`: `setSeed(v) = (v ^ 0x5DEECE66D) & (2⁴⁸ − 1)`; `next(b)`: Zustand
`(z · 0x5DEECE66D + 0xB) & (2⁴⁸ − 1)`, Ergebnis `(int)(z >> (48 − b))`; `nextInt(n)` nach
`BitRandomSource` (Zweierpotenz: `(int)((long) n · next(31) >> 31)`, sonst Ablehnungsschleife
`i − i % n + (n − 1) < 0` in `int`); `nextLong() = ((long) next(32) << 32) + next(32)` (beide vorzeichenbehaftet);
`nextFloat() = next(24) · 2⁻²⁴`. `RandomSource.create(seed)` ist `LegacyRandomSource(seed)`.

## 4. Geometrie (Vertrag; Java-Arithmetik)

Notation: `f32(·)` rundet jede `float`-Operation einzeln (Java `float`), `double` ist IEEE-`f64`,
`(int)` trunkiert gegen null und saturiert, `Mth.floor(d) = (int) d − (d < (int) d)`. `sin(v) =
SIN[(int) f32(v · 10430.378f) & 65535]`, `cos(v) = SIN[(int) f32(f32(v · 10430.378f) + 16384f) & 65535]`;
`SIN[i] = (float) Math.sin(i · π · 2.0 / 65536.0)` (links assoziativ; Policy C7). `PI_F = (float) π`,
`TAU_F = (float)(2π)`, `HALF_PI_F = (float)(π/2)`. Float-Provider: `uniform(lo, hi) = f32(f32(nextFloat() ·
f32(hi − lo)) + lo)`; `trapezoid(lo, hi, pl)`: `f = f32(hi − lo)`, `f1 = f32(f32(f − pl) / 2)`, `f2 = f32(f − f1)`,
`f32(f32(lo + f32(nextFloat() · f2)) + f32(nextFloat() · f1))` (zwei Ziehungen in dieser Reihenfolge);
`constant(v) = v` ohne Ziehung. `UniformHeight(lo, hi) = nextInt(hi − lo + 1) + lo`.
Generierungsgrenzen: `minGenY = −64`, `genDepth = 384`, oberste Carver-Zeile `−64 + 384 − 1 − 7 = 312`
(`isUpgrading()` ist für frische Chunks `false`), unterste `−62`.

Konfiguration (echt, `carvers.tsv`): `cave` y `[−56, 180]`, `cave_extra_underground` y `[−56, 47]`; beide
`horizontal_radius_multiplier` uniform(0.7f, 1.4f), `vertical_radius_multiplier` uniform(0.8f, 1.3f),
`floor_level` uniform(−1.0f, −0.4f), `yScale` uniform(0.1f, 0.9f); `canyon` y `[10, 67]`, `yScale`
constant(3.0f), `vertical_rotation` uniform(−0.125f, 0.125f), `distance_factor` und
`horizontal_radius_factor` uniform(0.75f, 1.0f), `thickness` trapezoid(0, 6, 2), `width_smoothness` 3,
`vertical_radius_default_factor` 1.0f, `vertical_radius_center_factor` 0.0f; alle `lava_level` −56,
`debug` aus, `replaceable` `#minecraft:overworld_carver_replaceables`. Wegen der `f32`-Rundung von
`uniform` ist die Obergrenze **nicht exklusiv**: `uniform(0.75f, 1.0f)` und `uniform(0.8f, 1.3f)` liefern bei
der größten Ziehung exakt die Obergrenze (§2.6); Schranken dürfen deshalb nie `sample < hi` annehmen.

### 4.1 Höhle (`cave`, `cave_extra_underground`)

1. `n = nextInt(nextInt(nextInt(15) + 1) + 1)` (innerste Ziehung zuerst; `n ≤ 14`).
2. Für jede Höhle: `x = 16sx + nextInt(16)`, `y = UniformHeight`, `z = 16sz + nextInt(16)` (als `double`),
   `hm = horizontal_radius_multiplier`, `vm = vertical_radius_multiplier`, `fl = floor_level`. Skip-Test
   `relY ≤ fl || relX² + relY² + relZ² ≥ 1` (`double`, links assoziativ). `tunnels = 1`.
3. Wenn `nextInt(4) == 0`: Raum. `ratio = yScale`, `r = f32(1 + f32(nextFloat() · 6))`,
   `d = 1.5 + f32(sin(HALF_PI_F) · r)`, Ellipsoid bei `(x + 1, y, z)` mit `hr = d`, `vr = d · ratio`;
   `tunnels += nextInt(4)`.
4. Für jeden Tunnel: `yaw = f32(nextFloat() · TAU_F)`, `pitch = f32(f32(nextFloat() − 0.5f) / 4)`,
   `thickness = f32(f32(nextFloat() · 2) + nextFloat())`; wenn `nextInt(10) == 0`: zwei Ziehungen `a, b` und
   `thickness = f32(thickness · f32(f32(f32(a · b) · 3) + 1))`; `count = 112 − nextInt(28)`;
   `tunnel(nextLong(), x, y, z, hm, vm, thickness, yaw, pitch, 0, count, ratio = 1.0)`.

### 4.2 Tunnel (`createTunnel`)

`rs = LegacyRandomSource(seed)`; `branch = rs.nextInt(count / 2) + count / 4`; `steep = rs.nextInt(6) == 0`;
`f = f1 = 0f`. Für `i = start..count−1`:
`d = 1.5 + f32(sin(f32(f32(PI_F · i) / count)) · thickness)`, `d1 = d · ratio`; `c = cos(pitch)`;
`x += f32(cos(yaw) · c)`, `y += sin(pitch)`, `z += f32(sin(yaw) · c)`; `pitch = f32(pitch · (steep ? 0.92f :
0.7f))`, `pitch = f32(pitch + f32(f1 · 0.1f))`, `yaw = f32(yaw + f32(f · 0.1f))`, `f1 = f32(f1 · 0.9f)`,
`f = f32(f · 0.75f)`; `f1 = f32(f1 + f32(f32(f32(a − b) · c') · 2))` und danach
`f = f32(f + f32(f32(f32(a' − b') · c'') · 4))` mit je drei Ziehungen in Lesereihenfolge.
Wenn `i == branch && thickness > 1`: zwei Kindtunnel, jeweils Argumente in dieser Reihenfolge
`rs.nextLong()`, dann `thickness' = f32(f32(rs.nextFloat() · 0.5) + 0.5)`, mit `yaw ∓ HALF_PI_F`
(zuerst minus), `pitch' = f32(pitch / 3)`, `start = i`, `ratio = 1.0`; danach endet der Tunnel. Sonst, wenn
`rs.nextInt(4) != 0`: endet bei `!canReach(target, x, z, i, count, thickness)`, sonst Ellipsoid
`(x, y, z, d · hm, d1 · vm)`. Kindtunnel verzweigen nie: `thickness' ≤ 1.0` (bei der größten Ziehung rundet
`f32(0.49999997 + 0.5)` auf exakt 1.0), die Verzweigung verlangt aber `thickness > 1`; die Rekursionstiefe ist 2.

### 4.3 Canyon

`x = 16sx + nextInt(16)`, `y = UniformHeight` (als `double`), `z = 16sz + nextInt(16)`,
`yaw = f32(nextFloat() · TAU_F)`, `pitch = vertical_rotation`, `ratio = yScale` (3.0, keine Ziehung),
`thickness = trapezoid`, `count = (int) f32(112 · distance_factor)`; `rs = LegacyRandomSource(nextLong())`.
Breitenfaktoren `w[0..384)`: `w0 = 1f`; für jedes `i`: wenn `i == 0 || rs.nextInt(3) == 0` dann
`w0 = f32(1 + f32(rs.nextFloat() · rs.nextFloat()))`; `w[i] = f32(w0 · w0)`. Für `i = 0..count−1`:
`d = 1.5 + f32(sin(f32(f32(i · PI_F) / count)) · thickness)`, `d1 = d · ratio`,
`d = d · horizontal_radius_factor(rs)`; `g = f32(1 − f32(|f32(0.5f − f32(i / count))| · 2))`,
`fv = f32(1.0f + f32(0.0f · g))`, `d1 = fv · d1 · f32(f32(rs.nextFloat() · 0.25f) + 0.75f)` (`double`);
`c = cos(pitch)`, `s = sin(pitch)`; `x += f32(cos(yaw) · c)`, `y += s`, `z += f32(sin(yaw) · c)`;
`pitch = f32(f32(pitch · 0.7f) + f32(f1 · 0.05f))`, `yaw = f32(yaw + f32(f · 0.05f))`, `f1 = f32(f1 · 0.8f)`,
`f = f32(f · 0.5f)`, dann die zwei Drei-Ziehungs-Updates wie in §4.2; wenn `rs.nextInt(4) != 0`: Ende bei
`!canReach`, sonst Ellipsoid `(x, y, z, d, d1)` mit Skip-Test
`(relX² + relZ²) · w[y − minGenY − 1] + relY² / 6 ≥ 1`.

### 4.4 Ellipsoid und `canReach`

`canReach`: `dx = x − (16cx + 8)`, `dz = z − (16cz + 8)`, `rem = (double)(count − i)`,
`lim = f32(f32(width + 2) + 16)`; erreichbar, wenn `dx² + dz² − rem² ≤ lim²` (`double`; Zielchunk, nicht
Quellchunk).

Ellipsoid `(x, y, z, hr, vr, skip)`: nichts, wenn `|x − (16cx + 8)| > 16 + 2hr` oder `|z − (16cz + 8)| >
16 + 2hr`. Sonst lokale Grenzen `x0 = max(floor(x − hr) − 16cx − 1, 0)`, `x1 = min(floor(x + hr) − 16cx, 15)`,
ebenso `z0`, `z1`; `y0 = max(floor(y − vr) − 1, −63)`, `y1 = min(floor(y + vr) + 1, 312)`. Für `lx = x0..x1`
(außen), `lz = z0..z1`, `relX = (16cx + lx + 0.5 − x) / hr`, `relZ` analog: nur wenn `relX² + relZ² < 1`;
`reached = false` **pro Spalte und Ellipsoid**; für `y = y1` absteigend bis `y0 + 1`: `relY = (y − 0.5 −
y_c) / vr`; wenn `!skip(relX, relY, relZ, y)` und das Maskenbit `(lx, y, lz)` nicht gesetzt ist: Bit setzen,
dann `carveBlock` (§5).

## 5. Ersetzung, Aquifer, Flag, Maske und Fix-up (Vertrag)

`carveBlock(lx, y, lz, reached)` an `p = (16cx + lx, y, 16cz + lz)`, gegen den **aktuellen** Chunkinhalt:

1. `s = state(p)`. Ist der Block von `s` `grass_block` oder `mycelium`: `reached = true` (auch wenn danach
   nichts geschrieben wird).
2. Ersetzbar ist `s` genau dann, wenn sein Block in der Tag-Expansion steht (§9 C3: 52 Blöcke,
   `tag.tsv`). Sonst Ende (Maskenbit bleibt gesetzt). Nicht ersetzbar sind genau die Alphabetzustände, deren
   Block nicht in der Liste steht; beobachtet: `air`, `bedrock`, `lava`, `ice`.
3. Carve-Zustand: wenn `y ≤ −56` (Lava-Level): `lava` **ohne** Aquifer-Aufruf; sonst
   `r = aquifer.computeSubstance(p, 0.0)`, danach `fluidFlag = aquifer.schedule`; `r = null` → Ende ohne Write
   (Barriere). Der GEN-05.4-Aquifer liefert nur `air`, `water` (Default-Fluid) oder `lava`.
4. Write des Carve-Zustands (§7). Danach wird `fluidFlag` gelesen (auch nach Lava-Level-Writes, deren Flag aus
   dem letzten Aquifer-Aufruf irgendeines früheren Carvers dieses Chunks stammt): ist der Carve-Zustand ein
   Fluid und `fluidFlag` gesetzt, wird `p` für das Post-Processing markiert (4 echte Fälle markieren Lava über
   ein veraltetes Flag).
5. Fix-up: wenn `reached` und `state(x, y − 1, z)` (unmittelbar darunter, aktueller Inhalt) genau `dirt` ist:
   `topMaterial((x, y − 1, z), hasFluid = Carve-Zustand ist Fluid)` (§6); ein Ergebnis wird an `y − 1`
   geschrieben und, falls Fluid, markiert. Kein Ergebnis → kein Write.

**Maske.** 16 × 16 × 384 Bits, Index `lx | lz << 4 | (y + 64) << 8`; ein Bit wird **vor** `carveBlock`
gesetzt und nie gelöscht, also wird jede Position höchstens einmal pro Ziel-Chunk geprüft — auch über Carver
und Quell-Chunks hinweg; Positionen mit nicht ersetzbarem Zustand oder Aquifer-`null` behalten ihr Bit. Die
Zusatzmaske (`setAdditionalMask`) ist für frische Welten immer `false` (§2.1.3) und entfällt.

**Fluid-Flag.** Ein einziger Zustand pro Ziel-Chunk: das `schedule`-Flag des zuletzt ausgeführten
Aquifer-Aufrufs dieses CARVERS-Laufs, initial `false`. Er wird weder pro Carver noch pro Quell-Chunk
zurückgesetzt.

## 6. Eingaben und Produzenten

| Eingabe | Herleitung | Zeitpunkt | Quelle in FerrumC |
| --- | --- | --- | --- |
| Post-SURFACE-Zustände, `WORLD_SURFACE_WG`, Fluid-Flags | veröffentlichter GEN-05.10-Puffer, einmal kopiert | vor Quelle (−8, −8) | `SurfaceChunkBuffer` (`Published` für Chunk, Seed, Settings) |
| Carver-Liste, Konfiguration, Tag | Konstanten (C1, C3) | Bindung | Vertrag §4, gepinnt in `carvers.tsv`/`tag.tsv` |
| Starts, Ziehungen | §3 | pro Quelle/Carver | GEN-04 `WorldgenRandomSource::legacy` |
| Sinustabelle | §4 | Bindung oder pro Lookup (C7) | `f64::sin` |
| Aquifer-Ergebnis und Flag | `compute(p, 0.0)` | pro Carve oberhalb −56 | GEN-05.4 `Aquifer` in frischer `NoiseChunkSession` des Ziel-Chunks (C2) |
| Fix-up-Zustand | `topMaterial` | nach einem Write mit `reached` über `dirt` | GEN-05.9 `SurfaceRuleSession`: `begin_column((x, z))`, `evaluate(SurfaceBlockInput::new(y−1, 1, 1, water))`, `water = Some(y)` bei Fluid, sonst `None` |
| Fix-up-Biom | Zoom über die 3×3-Arrays | lazy in `evaluate` | GEN-05.8-Resolver (C5) |
| Fix-up-Steep-Höhen | `WORLD_SURFACE_WG` live | lazy in `evaluate` | Live-Höhen des Carver-Puffers |
| Fix-up-Mindest-Oberfläche | GEN-05.3-Ecken | lazy | wie GEN-05.10 (A2: aufrufervertraut) |

Alle echten Fix-up-Eingaben liegen in den GEN-05.9-P4-Domänen: `y − 1 ∈ [−63, 311]`, Stone-Depth 1/1,
Wasser `None` oder `Some(v)` mit `v ≤ 312`. **Aquifer: teilen oder neu erzeugen?** Das Original teilt im
Speicher den NOISE-Aquifer (`getOrCreateNoiseChunk`) und erzeugt ihn nach einem Reload neu; beide Pfade sind
ergebnisgleich, weil die Aquifer-Caches reine Funktionen der Gitterposition sind und das Flag am Ende von
NOISE in allen 73 Gruppen `false` ist (F = U 73/73). FerrumC erzeugt neu (C2); die Probe (§2.5) zeigt
42 646/42 646 gleiche Ergebnisse und Flags. Gelesen wird kein weiterer Noise-Zustand.

## 7. Chunk-Mutationsvertrag

- **Schreibbare Positionen (Original):** nur der Ziel-Chunk, `y ∈ [−63, 312]`; Carve-Writes nur auf
  Maskenpositionen mit ersetzbarem Zustand, Fix-up-Writes nur direkt darunter auf `dirt`. Nachbarn,
  Biome, Block-Entities, Licht, Entities und Strukturen bleiben unberührt (73/73).
- **Schreibreihenfolge:** Quellen in Schleifenreihenfolge (§3), darin Carver 0, 1, 2, darin die
  Geometriereihenfolge (§4), darin pro Position: Carve-Write, Flag-Lesung und Marke, Fix-up-Write und Marke.
- **Heightmaps:** jeder Write aktualisiert `WORLD_SURFACE_WG` live nach der originalen `update`-Regel
  (Anheben bei Nicht-Luft an oder über der Spitze; bei Luft genau an der Spitze Rescan abwärts bis −65; sonst
  nichts). Die Fix-up-Regeln lesen diese Live-Sicht (Steep). `OCEAN_FLOOR_WG` wird nicht geführt und nicht
  veröffentlicht (D5 fortgeführt; das Modell prüft ihn nur als abgeleitete Kontrolle). 8 echte Gruppen senken
  `WORLD_SURFACE_WG`.
- **Post-Processing (Policy C4):** die veröffentlichte Flagmenge ist die GEN-05.10-Flagmenge ODER die
  Carver-Marken. Das Original hängt an Section-Listen an und erzeugt dabei Duplikate (6 Gruppen: eine
  NOISE-Wassermarke und eine Carver-Marke auf derselben Position); Reihenfolge und Duplikate bleiben in der
  Fixture für einen späteren Verbraucher, nicht in der Runtime-Ausgabe.
- **Maske:** wird mit dem Ergebnis veröffentlicht (1536 Wörter).
- **Keine Teilergebnisse (Policy C6, wie D3):** jeder Aufruf markiert zuerst den Ausgabepuffer `Empty`; alle
  Verweigerungen von §8 liegen vor der ersten Quelle; ein defensiver Fehler in der Schleife lässt den Puffer
  `Empty`. Veröffentlicht wird nur ein vollständig durchlaufener Chunk (`Published` mit Chunk, Seed,
  Settings). Das Original ist nicht transaktional; alles-oder-nichts ist FerrumC-Policy.
- **GEN-05.10-Puffer:** wird nur gelesen (Kopie in den Carver-Puffer; Policy C9); er bleibt `Published` und
  unverändert.

## 8. Fehler, Verweigerungsreihenfolge und Policies

`CarverErrorKind` (optionale Chunk/Position/Grenze/Zähler-Nutzlast, keine Nachrichtenallokation):
`InvalidBudget`, `WorkBudgetExceeded`, `NativeLimit`, `InconsistentBinding(BindingMismatch::{Seed,
Settings})`, `InconsistentSurface(SurfaceMismatch::{NotPublished, Seed, Settings, Chunk})`,
`InconsistentAquifer(AquiferMismatch::{Seed, Settings, Chunk, Lifecycle})`, `Rules(SurfaceRuleErrorKind)`,
`Aquifer(NoiseChunkErrorKind)`, `UnclassifiedState`. Rustdoc nennt pro Art Policy, Grenze, Konsistenz oder
Defensive.

Reihenfolge eines Carver-Aufrufs (alles vor der ersten Quelle; Puffer bereits `Empty`):

1. `InconsistentBinding`: die Carver-Bindung gehört zu den gegebenen `SeededSurfaceRules` (Seed, Settings).
2. `InconsistentSurface`: der GEN-05.10-Puffer ist `Published`, mit gleichem Seed, Settings und Chunk =
   Resolver-Zentrum.
3. `InconsistentAquifer`: die Noise-Session gehört zu Seed, Settings und Chunk und ist abfragebereit.
4. Session-Zulassung der GEN-05.9-Regeln (GEN-05.9-Reihenfolge) → `Rules(kind)`.
5. `WorkBudgetExceeded`, wenn die Carver-Allowance unter `R` liegt (§10, C10).
6. Schleife. `UnclassifiedState` (Zustand außerhalb des Alphabets), `Aquifer(kind)` aus GEN-05.4 und
   `Rules(kind)` aus der Session sind defensiv (für zugelassene Eingaben unerreichbar) → `Empty`.

FerrumC-Policies dieses Tickets (kein Originalverhalten):

- **C1 Konstante Carver-Liste.** Keine Biomabfrage pro Quell-Chunk; die Liste ist für jede zugelassene Quelle
  `[cave, cave_extra_underground, canyon]`. Begründung: alle 54 Biome, die `MultiNoise(minecraft:overworld)`
  (P6) liefern kann, listen genau diese drei; 73 × 289 echte Auflösungen bestätigen es. Folge: die
  RTree-Historie kann die Liste nicht beeinflussen. Root-Option §13.
- **C2 Frischer Aquifer.** Ein frischer GEN-05.4-Aquifer in einer Noise-Session des Ziel-Chunks, Flag initial
  `false` (= Reload-Pfad, ergebnisgleich zum In-Memory-Pfad, §6). Root-Option §13.
- **C3 Replaceable nach Namen.** Die 52 Blöcke der echten Tag-Expansion als gepinnte Namensliste; geprüft
  wird über das GEN-05.10-Alphabet (35 Zustände) per Blockname, nie über Fixture-Flags (A6). Der Tag ist
  nicht in der Registry; eine Tag-Tabelle ist Voraussetzung für Features, nicht für diesen Carver.
- **C4 Post-Processing als Menge** (D5 fortgeführt).
- **C5 Fix-up-Biom über die gespeicherten Arrays** (GEN-05.8) statt der originalen Direktquelle
  (`biomeManager1::getBiome` über `BiomeSource.getNoiseBiome`). Das ist eine **Abweichung vom Original, keine
  belegte Parität**: die 355 gleichen Werte sind ein Korpusbefund, kein Seed- oder Weltbeweis. **Offene
  Abweichungsdomäne (für das Review ausdrücklich offen):** (i) RTree-Gleichstände, bei denen die Direktquelle
  im Carver mit einer anderen Suchhistorie auflöst als `createBiomes` beim Speichern der Arrays; (ii) die
  Quart-Y-Klemmung der gespeicherten Arrays (−16..79) gegenüber der ungeklemmten Direktquelle, erreichbar,
  wenn der Zoom einer Fix-up-Position bei `y − 1 ≤ −62` Quart −17 abtastet. Für (ii) ist im Korpus kein Gras
  so tief; quellbasiert ausgeschlossen ist es nicht. Root-Option §13.
- **C6 Alles oder nichts** (D3 fortgeführt).
- **C7 Sinustabelle aus `f64::sin`** mit einem erschöpfenden Test aller 65 536 Einträge gegen den gepinnten
  Digest `cdfaec68…627d`. Root-Option §13.
- **C8 Kein In-Memory-Flag.** Der Carver übernimmt kein Flag aus NOISE.
- **C9 Eigener Carver-Puffer** (Kopie des GEN-05.10-Puffers plus Maske). Root-Option §13.
- **C10 Chunk-Vorprüfung** der Carver-Allowance gegen die strukturelle Schranke `R` (wie D2). Root-Option §13.
- **C11 Ziehungen ohne Arbeitsgrenze.** `nextInt`-Ablehnungsschleifen sind ergebnisrelevant und laufen
  vollständig; sie zählen nicht gegen die Allowance (§10).
- Geerbt: P1, P2, P3, P4, P6 (GEN-05.9), D1, D4, D5 (GEN-05.10).

## 9. Replay-Klassen

Die Klasse folgt mechanisch aus der Policy-Tabelle und wird auf drei Wegen abgeleitet (§12):

| Policy (Reihenfolge) | Domäne | Oracle-Gruppen außerhalb | Erwartet |
| --- | --- | --- | --- |
| P6 Resolverquelle (geerbt) | Presets normal, large_biomes, amplified | `policy/single_biome_surface/0` | kein veröffentlichter GEN-05.10-Puffer (`Rules(InconsistentResolver(Source))` beim GEN-05.10-Aufruf); der Carver ist unerreichbar |
| D4 Zentrumsdomäne (geerbt) | `cx, cz ∈ [−1874999, 1874998]` | `outside/normal/0/+x` | kein Puffer (GEN-05.10 A1: `ChunkOutOfRange`/`CoordinateDomain`); Carver unerreichbar |
| P4 pro Fix-up | `y ∈ [−64, 319]`, Stone 1, Wasser ≠ `Some(i32::MIN)` | keine (0 von 355) | — |
| C1 Carver-Liste | Biome mit Liste `[cave, extra, canyon]` | keine (0 von 73 × 289) | — |

Ergebnis: 71 `full`, 1 `refused:CentreOutsideDomain`, 1 `refused:InconsistentResolver(Source)`. Die
Quell-Chunks einer Randgruppe (Zentrum 1874998) liegen bis 1875006 außerhalb der Chunk-Domäne; das ist für
den Carver unkritisch, weil Quellen nur Seeds und Geometrie liefern, keine Chunkdaten.

## 10. Kapazitäten, Arbeit und Allokation

**Carver-Arbeitseinheiten** (getrennt von den GEN-05.4-Aquifer- und GEN-05.9-Session-Einheiten): 1 pro
Starttest, Tunnel- oder Canyon-Schritt, Breitenfaktor-Iteration, Ellipsoid-Aufruf, Spaltentest, Y-Schritt,
`carveBlock`, Aquifer-Aufruf, Flag-Lesung, Fix-up und Write; ein Rescan zählt 1 pro gelesener Zeile.
Ziehungen zählen nicht (C11).

| Teil | Schranke (Herleitung aus §4) |
| --- | --- |
| Höhlen pro Start | 14 (`nextInt(nextInt(nextInt(15)+1)+1)`) |
| Tunnel pro Höhle | 4 (1 + `nextInt(4)` nur mit Raum) |
| Schritte pro Tunnel inkl. Kinder | 197 (`count ≤ 112`, `branch ≥ count/4`: `branch + 1 + 2(count − branch)`) |
| Ellipsoide pro Tunnel | 196; pro Höhle 1 + 4 · 196 = 785 |
| Höhlen-Carver pro Start | 10 990 Ellipsoide, 11 032 Schritte |
| Canyon pro Start | 112 Schritte und Ellipsoide (`(int)((float) 112 · f)` mit `f ≤ 1.0`, §2.6), 384 Breitenfaktoren |
| Radien (Grenzprobe §2.6) | Höhle: `d ≤ 1.5 + 11.999998 = 13.499998`, `hr ≤ d · 1.3999999 < 18.9`, `vr ≤ d · 1.3f = 17.549997 < 17.55`; Raum: `r ≤ 6.9999995`, `vr ≤ 8.4999995 · 0.8999999 < 7.65`; Canyon: `hr ≤ (1.5 + 5.9999995) · 1.0 = 7.4999995`, `vr ≤ 22.4999985 · 1.0 < 22.5` |
| Iteration pro Ellipsoid (Höhle / Canyon) | 1 + 256 Spalten + 256 · 38 bzw. 256 · 47 Y-Schritte = 9985 / 12 289 (`floor(y + vr) − floor(y − vr) + 2` ≤ 38 bzw. 47) |
| pro Quell-Chunk | 3 + 2 · (10 990 · 9985 + 11 032) + (112 · 12 289 + 112 + 384) = 220 869 231 |
| Geometrie pro Ziel-Chunk `G` | 289 · 220 869 231 = **63 831 207 759** |
| Block-Arbeit pro Ziel-Chunk `B` | Positionen ≤ 256 · 375 = 96 000 → `carveBlock`, Aquifer, Flag, Fix-up je ≤ 96 000; Writes ≤ 192 000 zu je ≤ 1 + 384: 4 · 96 000 + 192 000 · 385 = **74 304 000** |
| **`R = G + B`** | **63 905 511 759** |

Beobachtet (Paper, Maximum pro Ziel-Chunk, `delivery/analysis-normal.json`): 4728 Tunnelschritte,
3344 Ellipsoide, 6618 Spalten, 30 215 Y-Schritte, 5065 Maskenbits, 3815 Aquifer-Aufrufe, 3718 Writes,
168 Fix-ups; größtes `hr` 10.3, größtes `vr` 19.27 (Canyon), größte Y-Spanne 41. Die Schranke ist strukturell
und um Größenordnungen über dem Beobachteten, weil `canReach` ferne Tunnel früh beendet; sie ist keine
Laufzeitaussage.

`CarverWorkBudget::new` lässt `1..=2³⁶` zu (Tests 0 / 1 / 2³⁶ / 2³⁶ + 1); der Carver verweigert
`WorkBudgetExceeded` bei `R − 1` und lässt `R` und `R + 1` zu (N−1/N/N+1). Die Runtime berechnet die
Teilschranken aus diesen Formeln, pinnt sie als Konstanten, prüft die tatsächliche Nutzung jeder
Fixture-Gruppe `≤ R` und berichtet das Maximum. **Aquifer-Arbeit** läuft über die `SampleWork` der
Noise-Session (GEN-05.4): Designzahl 36 293 Einheiten pro Aufruf im Maximum; die Runtime leitet die
Session-Allowance aus ≤ 96 000 Aufrufen plus ≤ 315 Positions- und 315 Status-Fehlgriffen ab und testet sie
N−1/N/N+1. **Fix-up-Arbeit:** eine GEN-05.9-Session pro Chunk mit `SURFACE_RULE_OVERWORLD_CHUNK_WORK`
(45 908 736) reicht: ≤ 96 000 Fix-ups zu je ≤ 2 + 1 + 467 Einheiten = 45 120 000.

Bindungszensus (Designzahl, Runtime pinnt): Alphabet 35 + Tag-Namen 52 + Carver 3 = **90** (Tests
89 / 90 / 91); `CarverBindBudget` `1..=65 536`.

Kapazitäten (Design-Caps; die Runtime misst exakte Retained-Bytes, pinnt sie **literal** (RR10-H3) und testet
`NativeLimit` N−1/N/N+1 über private Cap-Konstruktoren; eine Cap-Änderung ist Root-Entscheidung):

| Element | Cap | Designzahl |
| --- | --- | --- |
| Carver-Bindung retained | 4 KiB | Konfigurationskonstanten + 35 Alphabet-Einträge mit Ersetzbarkeit; keine Sinustabelle (C7) |
| Carver-Puffer retained (C9) | 448 KiB | 393 216 B Zustände + 12 288 B Fluid-Flags + 12 288 B Maske + 1024 B Höhen + Kopf |
| Scratch pro Chunk | nur Stack | Canyon-Breitenfaktoren 384 · 4 = 1536 B, Rekursionstiefe 2, Tunnelzustand, GEN-05.9-Session (400 B) |
| Noise-Session mit Aquifer | GEN-05.4-Caps | `Aquifer` 80 B + Caches ≤ 12 KiB; die Session selbst nach GEN-05.3/05.4 |

**Allokation:** Bindung und Pufferkonstruktion allozieren (gezählt, nicht als frei behauptet). Der
Carver-Aufruf allokiert **null** Mal; gemessen in einem separaten kalten Debug-/Release-Prozess mit
positiver Kontrolle (4096 B), für Erfolg und jede Verweigerungsart (RR10-H4-Muster). Die Erzeugung der
Noise-Session und ihres Aquifers ist außerhalb des Aufrufs und wird getrennt gezählt. Keine TPS-, RSS- oder
Geschwindigkeitsaussage folgt.

## 11. API-Kandidat und Runtime-Ownership

Ein neues Modul `carver` in `ferrumc-world` (eigener `pub mod` in `lib.rs` oder Untermodul von
`surface_rules`; Root-Option §13), private Felder, Rustdoc auf allem:

- `CarverBindBudget`, `CarverWorkBudget` (`new(…) -> Result`, §10);
- `SeededCarvers::bind(&SeededSurfaceColumns, CarverBindBudget)` (erbt Seed, Settings, Alphabet);
- `CarvedChunkBuffer::new()`, Accessoren `state(..)`, `world_surface_heights()`, `fluid_flag(..)`,
  `carving_mask(..)` nur bei `Published`, `published() -> Option<(ChunkPos, WorldSeed,
  NoiseSettingsResourceId)>`;
- `SeededCarvers::carve_chunk(&self, CarverInput<'_>, CarverWorkBudget, &mut CarvedChunkBuffer) ->
  Result<CarverReport, CarverError>` mit `CarverInput { surface: &SurfaceChunkBuffer, rules, resolver,
  corners, noise: &mut NoiseChunkSession }`;
- `CarverReport`: `work_used`, Starts, Treffer, Maskenbits, Aquifer-Aufrufe, Writes, Fix-ups.

Crate-private Ergänzungen, rein additiv, ohne Verhaltensänderung bestehender Pfade: in
`density::chunk::material` ein `pub(crate)`-Einstieg „Aquifer an Blockposition, Dichte 0“, der in der
gegebenen Session einen run-eigenen `Aquifer` hält und `compute` an der gesetzten Position aufruft (genau der
Aufruf der Probe); ein Lesezugriff auf die veröffentlichten Inhalte des GEN-05.10-Puffers. Keine
Registry-Änderung, keine Abhängigkeit. Fixture-Pfad:
`server/crates/ferrumc-world/tests/fixtures/carver-original-1.21.8/` (die 11 Dateien von §12, unverändert);
Referenzwerkzeuge unter `server/crates/ferrumc-world/tests/reference/carver/`. `INVARIANTS.md` und `README.md`
erhalten einen Carver-Abschnitt.

## 12. Oracle-Protokoll, Reproduzierbarkeit und Fixture

Pins und Guards: `oracle/prepare.py` schreibt `input-pins.json` (1133 Eingaben: World-Crate gegen den
Runtime-Freeze, Artefakte, JDK, Mappings, akzeptierte GEN-05.10-Referenzen und -Snapshots, Dokumente,
Quellen, Design, Oracle). `oracle/run_oracle.py OUT` verweigert die fünf Java-Umgebungsvariablen,
Helfer-Paket-Schatten in den Original-JARs, einen nicht kanonischen Helfer, Byte- oder Mitgliedsdrift
(vorher und nachher), nicht frische oder bereichsfremde Ausgabe und eine abweichende Inventur der frisch
kompilierten Helferklassen (7). Ein optionales Entwicklungslimit steht als `development_limit` im Manifest;
beide autoritativen Läufe haben 0. `oracle/check_guards.py OUT REF` übt 16 Guards im
Python-Modus des Aufrufers mit explizitem `require` (5 Umgebungsguards über den echten Einstieg, kanonische
Quelle, Digest, Schatten, frische Ausgabe, Schreibbereich, JDK-/World-/Design-Mitgliedschaft, 3 echte
JVM-CodeSource-Abweichungen vor jeder Stichprobe). Normal- und `-O`-`guards.json` unterscheiden sich nur in
`entrypoint_python_optimize`. Zeitgestempelte Logs stehen nur in `diagnostics-manifest.json`.

**Kompakte Fixture** (`fixture/carver_fixture.py generate RAW OUT`, 11 Dateien, 4,2 MB):
`states.tsv` (32 beobachtete Zustände mit Original-Flags nur zur Durchsicht), `tag.tsv` (52 Blöcke),
`carvers.tsv` (drei Konfigurationen, Biomlisten), `groups.tsv` (Klasse, Herkunft, Startflag des Aquifers,
Carver-Biome, 867 Startflags, S1- und FINAL-Digest nach Zustandsnamen in Materialreihenfolge, Zähler),
`chunks.tsv` (S1 und FINAL: RLE, beide Heightmaps, geordnetes Post-Processing, Maske), `carves.tsv`
(Zuordnung pro gestartetem Carver), `trace.tsv` (geordnete Aquifer-, Flag- und Fix-up-Ereignisse),
`tops.tsv` (Fix-up-Eingaben, Ergebnis, Diagnosebiome), `sin.tsv` (echte Tabelle), `vanilla.tsv`
(Vanilla-Kontrolle) und `manifest.json` (Policy-Tabelle, Replay-Regel, Labels, Datei-Digests).

`validate RAW FIX` regeneriert bytegleich und prüft auf **Route 2** aus den Fixture-Token allein: Klassen aus
der Policy-Tabelle; Ersetzbarkeitsflags gegen die Tag-Namen; die 54-Biom-Liste; 867 Startflags gegen die
Zuordnungszeilen; S1- und FINAL-Digests; Maskenbits gegen die Summe der neuen Bits pro Carver; jede
Aquifer-Position auf Maske, Ersetzbarkeit, `y ∈ (−56, 312]` und Endzustand; Ereigniszähler gegen die
Zuordnung; beide Heightmaps als Top-Prädikate der Endzustände; Post-Processing als S1-Präfix plus Marken auf
Fluid-Endzuständen. Normal = `-O`: 73 Gruppen (71 `full`, 1 + 1 `refused`), 63 291 Starttests, 4807 Starts,
65 198 Maskenbits, 42 646 Aquifer-Aufrufe, 41 549 Flag-Lesungen, 355 Fix-ups, 73 Digests, Vanilla gleich in
48 Gruppen (25 S1-Unterschiede, §2.3). **Route 3** (`delivery/freeze.py`) leitet die Klassen direkt aus den
rohen Paper-Gruppen ab und zählt Aquifer-Aufrufe, Maskenbits und Starts aus den Rohdaten nach. Zehn
Einzelzellen-Mutationen von Fixture-Kopien (Aquifer-Ergebnis, Aquifer-Position, Klasse, Startflag,
Maskenwort, FINAL-RLE, Heightmap, Post-Processing-Reihenfolge, Zuordnungszeile, Ersetzbarkeitsflag) werden
von Route 2 jeweils mit dem erwarteten Grund verweigert (`delivery/mutations/`, `final-compare.json`).

Runtime-Replay-Regel: für jede `full`-Gruppe den Post-SURFACE-Puffer aus den FerrumC-Produzenten bauen
(GEN-05.10 `build_chunk`; er muss den S1-Token gleichen), den Carver laufen lassen und FINAL-Zustände
(Namen), Maskenwörter, `WORLD_SURFACE_WG`, die Fluid-Flagmenge (= FINAL-Post-Processing-Positionen), die
Startflags, die Zuordnungszeilen und den `cfg(test)`-Trace (Aquifer-Aufrufe mit Position, Ergebnis und Flag;
Flag-Lesungen; Fix-ups mit Eingaben und Ergebnis) exakt vergleichen; für jede `refused`-Gruppe nur die
typisierte Verweigerung erwarten; nie Zeilen hinzufügen, keine Klasse lockern, keine Werte einer
verweigerten Gruppe vergleichen.

## 13. Policy-Entscheidungen für Root

1. **C1 Carver-Liste.** (a) Konstante Liste ohne Biomabfrage (empfohlen; kein Klimasampling für 289
   Quellen, unabhängig von der RTree-Historie, durch 54/54 Biome und 21 097 echte Auflösungen gestützt);
   (b) Biom pro Quell-Chunk über GEN-05.5 und eine Carver-Liste pro Biom (originalnäher, aber ohne
   Ergebniswirkung und mit Kosten).
2. **C2 Aquifer.** (a) frischer Aquifer in einer Noise-Session über einen additiven crate-privaten Einstieg
   (empfohlen; Probe 42 646/42 646); (b) den Aquifer des GEN-05.4-Materiallaufs aufheben und weiterreichen
   (In-Memory-Pfad; verlangt eine Änderung an `MaterialRun`/`sample_chunk_materials`, ohne Ergebnisgewinn).
3. **C5 Fix-up-Biom.** (a) GEN-05.8-Resolver über die gespeicherten Arrays (billig, gleiche Werte im Korpus,
   aber eine bewusste Abweichung mit der offenen Domäne aus §8); (b) direkte Biomquelle mit Zoom
   (originalgetreu einschließlich Klemmung; braucht Klimasampling im Carver und für Historie-Gleichheit eine
   eigene RTree-Historie-Policy). Keine Empfehlung ohne Review der offenen Domäne; vorläufig (a) nur unter
   ausdrücklicher Root-Annahme der Abweichung.
4. **C7 Sinustabelle.** (a) `f64::sin` pro Lookup oder einmalig bei der Bindung, mit erschöpfendem Test
   gegen den Digest (empfohlen; 65 536/65 536); (b) die 256-KiB-Tabelle als eingebettete Fixture-Daten.
5. **C9 Puffer.** (a) eigener `CarvedChunkBuffer` als Kopie des GEN-05.10-Puffers (empfohlen; keine Änderung
   an GEN-05.10); (b) In-place-Übergang des `SurfaceChunkBuffer` (spart 406 KiB, ändert aber die
   GEN-05.10-Veröffentlichung).
6. **C10 Arbeit.** (a) Vorprüfung gegen die strukturelle Schranke `R` = 63 905 511 759 (empfohlen; verweigert
   nie einen zugelassenen Chunk); (b) eine praktische Kappe (z. B. 2²⁴) mit typisierter Verweigerung
   (begrenzt CPU pro Chunk härter, kann aber theoretisch einen echten Chunk verweigern).
7. **Modulort.** (a) eigenes Modul `carver` (empfohlen; der Carver ist ein eigener Generierungsschritt);
   (b) Untermodul `surface_rules::carver` (Zugriff auf `pub(super)`-Internas ohne neue Sichtbarkeit).
8. D1-D5 und P1-P6 bleiben unverändert; C3, C4, C6, C8, C11 sind ohne Alternative mit Ergebniswirkung und werden
   zur Annahme empfohlen.

## 14. Fehlende Daten und Voraussetzungen

Für diesen Carver ist **kein Voraussetzungsticket außerhalb von `ferrumc-world` nötig:** alle Eingaben
kommen aus akzeptierten World-Produzenten; Tag und Konfiguration sind als gepinnte Konstanten genug, weil
das Alphabet geschlossen ist (C3). Echte offene Lücken, ausdrücklich:

- **Producer-API (World, additiv):** GEN-05.4 hat keinen Einstieg „Aquifer an beliebiger Blockposition,
  Dichte 0“; der Aquifer ist an den Materiallauf gebunden (`MaterialRun`). Die Runtime braucht den
  crate-privaten Einstieg aus §11 (C2). Die Probe nutzt dafür nur `cfg(test)`-Zugriffe auf Session-Felder.
- **Consumer-API (World, additiv):** `SurfaceChunkBuffer` bietet die veröffentlichten Daten nur über
  Einzel-Accessoren; die Kopie in den Carver-Puffer (C9) braucht einen crate-privaten Massenlesezugriff.
- **Produzentenkette für Fix-ups:** die GEN-05.9-Session wurde an den 355 Fix-up-Eingaben nicht in der Probe
  ausgeführt (§2.5); C2 der Runtime schließt das.
- **C5:** offene Abweichungsdomäne der Fix-up-Biome (§8), bis Root entscheidet.
- **Spätere Verbraucher** (nicht dieses Ticket): eine Block-Tag-Tabelle in der Registry (Features), eine
  `blocksMotion`-Tabelle für `OCEAN_FLOOR_WG` (GEN-05.10 H6) und eine Fluid-Tick-Semantik für die
  Post-Processing-Duplikate (C4).

## 15. Offene Punkte und Grenzen

- `mycelium_carved` und `top_empty` sind nach der deklarierten Suche absent. `mycelium` teilt mit
  `grass_block` dieselbe Bedingung (§5.1, quellbasiert); ein leerer Fix-up schreibt nichts (quellbasiert).
  Runtime-Bedingung C5 deckt beide synthetisch ab.
- Nur 6 Canyon-Treffer und 1 Gruppe mit veraltetem Flag; beide sind exakt nachgerechnet, aber dünn.
- Fix-ups nur in 6 Gruppen und nur in den Biomen plains, forest, dark_forest, jungle und snowy_taiga; ob eine
  Fix-up-Regel dabei Steep liest, ist nicht aufgezeichnet (die Live-Sicht ist quellbasiert, C5).
- Biomdiagnose: direkt = gespeichert in 355/355 (Korpus); die Abweichungsdomäne von C5 (RTree-Gleichstand,
  Quart-Y-Klemmung) ist nicht beobachtet und bleibt offen.
- Der Canyon-Rand `count = 112` (§2.6) und die exakten Provider-Maxima sind nur über die Grenzprobe belegt,
  nicht in einem echten Chunk.
- Nicht ausgeführt: ein echtes `ServerLevel`, `ChunkStatusTasks.generateCarvers` selbst (die Region ist
  alloziert, §2.2), Blending alter Chunks (Zusatzmaske), Strukturen (`generateStructures = false`), Features.
- Die strukturelle Schranke `R` ist sehr grob; die echte Last ist um Größenordnungen kleiner.
- Nicht autoritativ, vom Freeze ausgenommen: `scratch/` (Entwicklungsläufe), `probe/archive/`,
  `probe/target/`.

## 16. Bedingungen für das Runtime-Ticket

- **C1 — Replay** exakt wie §12 für 71 `full` und 2 `refused` Gruppen; keine Klasse gelockert.
- **C2 — Produzenten:** der Runtime-Test baut jede Eingabe aus GEN-05.3/.4/.5/.8/.9/.10 und prüft sie vor dem
  Carver gegen die Fixture-Token (S1-RLE, Startflag des Aquifers); die 355 Fix-up-Eingaben laufen über die
  GEN-05.9-Session und müssen die echten Ergebnisse liefern.
- **C3 — Direkte Bits:** Sinustabelle (65 536 Einträge, Digest), alle 63 291 Starttests und die
  Ziehungsfolgen über `WorldgenRandomSource::legacy`; jede Zeile von `boundary/final/*.tsv` als
  Grenztest (insbesondere `uniform(0.75f, 1.0f)` → `1.0` und Canyon `count = 112`), als Grenzprobe gelabelt.
- **C4 — Aquifer:** jede Aquifer-Zeile des Trace (Position, Ergebnis, Flag) über den crate-privaten Einstieg;
  ein Test, der zeigt, dass das Flag über Carver- und Quellgrenzen hinweg erhalten bleibt (veraltete
  Lava-Marke).
- **C5 — Quellbasierte Kanten** über `cfg(test)`-Routinen, als Vertragstests gelabelt: `mycelium` löst den
  Fix-up aus; leerer Fix-up schreibt nichts; Fix-up mit Fluid (`water = Some(y)`); Skip bei gesetztem
  Maskenbit über Carver hinweg; Lava-Level −56 gegen −55; Ober- und Untergrenze 312 / −62; Kindtunnel
  verzweigen nicht; Luft-Write an der Spitze mit Rescan; Barriere ohne Write mit gesetztem Maskenbit.
- **C6 — Arbeit und Kapazitäten:** §10-Konstanten, Budgetdomänen, `R − 1 / R / R + 1`, Zensus
  89 / 90 / 91, Retained-Byte-Caps N−1/N/N+1, null Allokationen mit positiver Kontrolle.
- **C7 — Verweigerungen:** jede §8-Art in der genannten Reihenfolge, Puffer danach `Empty`; Kombinationen
  pinnen die Präzedenz.
- **C8 — Ownership/Doku** wie §11.
- **Übernommene Testauflagen aus dem GEN-05.10-Runtime-Review** (in der World-Crate zu ergänzen):
  - **RR10-H1:** ein Test über den Produktionspfad von `build_chunk` mit einem Material, das bei y 319
    einen Block hat (`with_states`), prüft `i4 = 320`, den ersten Aufruf bei y 319 und die veröffentlichte
    Höhe; tötet Mutant M13 (Ableitung ab 318).
  - **RR10-H2:** Säulen mit Lava in beiden Extensions: Eroded bricht nur an Wasser ab, die
    Frozen-Wasserklausel akzeptiert keine Lava; tötet M14 und M15.
  - **RR10-H3:** die Retained-Bytes literal pinnen: 10 816 (`SeededSurfaceColumns`) und 406 592
    (`SurfaceChunkBuffer`), x86-64 mit dieser Toolchain; ebenso die neuen Carver-Werte.
  - **RR10-H4:** die Allokationsprobe misst zusätzlich die Verweigerungen Binding, Material, Session und
    `UnclassifiedState` (erwartet 0/0/0) mit positiver Kontrolle; ebenso jede neue Carver-Verweigerung.
  - **RR10-H5:** der A1-Puffertest startet von einem bereits `Published` Puffer (nicht von einem frischen),
    sodass „bleibt/wird `Empty`“ diskriminiert.

## 17. Frische Läufe und Freeze

```text
python3    sources/collect.py                      # einmalig; Quellen sind danach eingefroren
python3    oracle/prepare.py
python3    oracle/run_oracle.py final-normal
python3 -O oracle/run_oracle.py final-optimized
python3    oracle/check_guards.py guards-normal    final-normal
python3 -O oracle/check_guards.py guards-optimized final-normal
python3    fixture/carver_fixture.py generate final-normal    fixture/carver-original-1.21.8
python3 -O fixture/carver_fixture.py generate final-normal    fixture/generate-optimized
python3    fixture/carver_fixture.py generate final-optimized fixture/generate-from-oracle-optimized
python3    fixture/carver_fixture.py validate final-normal fixture/carver-original-1.21.8
python3 -O fixture/carver_fixture.py validate final-normal fixture/carver-original-1.21.8
python3    delivery/analysis.py final-normal > delivery/analysis-normal.json
python3    probe/extract.py final-normal probe/input && (cd probe && python3 run_probe.py input final)
python3    boundary/run_boundary.py boundary/final          # Grenzprobe, skriptgesteuerte Ziehungen
python3    delivery/forbidden_claims_check.py > delivery/forbidden-claims.json
python3    delivery/freeze.py
```

`delivery/freeze.py` prüft die Pins erneut, vergleicht beide frischen Läufe (jede Ausgabe und das Manifest
bytegleich), die Guards, die drei Fixture-Generierungen, beide Validierungen, Route 3, die zehn
Mutationskontrollen, die Auswertung, die Grenzprobe, die Probe-Quittung und den Forbidden-Claims-Scan
(0 Befunde) und schreibt
`delivery/freeze-manifest.json` über den Vertrag und alle Evidenz. Vertrags- und Freeze-SHA-256 werden Root
getrennt gemeldet.
