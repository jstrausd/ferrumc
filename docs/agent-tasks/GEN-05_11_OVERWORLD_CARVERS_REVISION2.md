# GEN-05.11 — Vorbereitung Runde 2: direkte Biomquelle und kumulative Zulassung

Lieferung vom 12.10.2026 zur bindenden [Root-Workorder Runde 2](GEN-05_11_OVERWORLD_CARVERS_REVISION2_TASK.md), von Root auf `c0b78c23` festgelegt. **Vorbereitung zur unabhängigen Prüfung; keine Runtimefreigabe.** Das [erste unabhängige Review](GEN-05_11_OVERWORLD_CARVERS_REVIEW.md) bleibt `BLOCKING_CONTRACT`. Dieses Dokument ersetzt dessen beanstandete Vertragsstellen konkret; es erklärt sich nicht selbst für `CLEAN_CONTRACT` oder `CLEAN_ORACLE`.

Schreibownership ausschließlich dieses neue Dokument und `.codex-tmp/gen05_11-r2/**`. Kein Git, kein weiterer Agent, keine Dependency, keine Produktions- oder historische Änderung. Storage- und Erz-Reviewarbeit im gemeinsamen Baum wurden nicht als Baseline verwendet. Die native Probe verwendet ausschließlich Roots akzeptiertes Archiv mit einem eng begrenzten Testoverlay. AGENTS.md, CLAUDE.md, PAPER_REFERENCE.md, World-INVARIANTS, Architektur-/Simulationslane und ADR-0009 gelten. Das Betreiberziel bleibt ein vollständig spielbarer Rust-Server mit ursprünglicher Minecraft-Logik; diese Vorbereitung liefert nur den nächsten World-Generierungsschritt.

## 1. Unveränderte Autoritäten und genaue Ersetzung

Alle Hashes sind SHA-256 über vollständige Bytes, nicht gekürzte Symbolpins.

| Autorität | Hash |
| --- | --- |
| [Runde-1-Vertrag](GEN-05_11_OVERWORLD_CARVERS.md) | `3f6ca7387ab19d0fcf063b9cb4bfbe0c534e7cedafa5a87e61f6107bb3f137ff` |
| `.codex-tmp/gen05_11/delivery/freeze-manifest.json`, 427 Dateien | `6451131f336c011486edcfc91d1a43a38274f4e5e390b48e3b5ba72f88b29863` |
| Runde-1-`oracle/input-pins.json`, 1133 Inputs | `27427e158bc05fa8ec6bf30577c54b0fdee2d38049cdbbd6db866fb3748e4837` |
| `.codex-tmp/gen05_11-independent-review/freeze-manifest.json`, 433 Dateien | `69bbb587222da52015d01480bf56aed48f7bc716f12afc703fd8ebcb388a1ced` |
| `.codex-tmp/gen05_10-runtime-review-base/base.freeze.json`, 1431 Archivdateien | `106ad3f3aaea282e4dc658d8c834162fc77e6a52882967391a6b45989496442e` |

`tools/final_check.py` prüft beide historischen Filefreezes vollständig, alle 1133 Inputs über die originale kanonische Preflight-/Membership-Prüfung, das akzeptierte Archiv und die 324 unveränderten World-Dateien. Der gesamte historische Elf-Dateien-Fixture bleibt unverändert. Neue Instrumentierung ist zusätzliche Evidenz, keine ersetzte Erwartung.

Runde 1 §§3–5 bleiben der vollständige Geometrie-, Seeding-, Ersetzungs-, Masken- und Schreibvertrag, einschließlich der echten Float-Grenzprobe `count = 112`. §§7/9/12/16 gelten weiter, soweit die folgenden präzisen Ersetzungen erforderlich sind:

- C1: konstante Liste **nach** jeder echten Source-Biomabfrage; das Weglassen der 289 Abfragen entfällt.
- C5(b): echte direkte Quelle mit Paper-Zoom, sechs Paper-Climate-Wurzeln und originalem Selector; Stored-Ersatz, Y-Klemme und Stored-Fallback entfallen.
- C2/C8: ein frischer, laufeigener Aquifer; kein fremder mutable Noise-Session-Eingang und keine Flagübernahme aus NOISE.
- C7(b): statische genuine SIN-Rawbits; die `f64::sin`-Herstellung entfällt.
- C9(a): unabhängiger Ausgabepuffer mit berechneter Kopier-/Resetarbeit.
- C10(a): strukturelle Carver-Zulassung plus separate kumulative Producer-Zulassungen; die alte Beobachtung 36293 ist kein strukturelles Versprechen.
- Eigenes `carver`-Modul. Alle Ergänzungen an akzeptierten Produzenten sind World-private und additiv. Öffentliche Budget-/Koordinatencaps und der bestehende Stored-Surface-Pfad behalten ihren Vertrag.

C3 (52 replaceable Blocknamen gegen das geschlossene Alphabet), C4 (Fluidmarken als Menge), C6 (alles oder nichts), C8, C11 und geerbte P1/P2/P3/P4/P6 sowie D1/D4/D5 bleiben deklarierte FerrumC-Policies. C11 bezeichnet ausschließlich den unmetered Legacy-Geometrie-RNG; der metered Aquifer-RNG wird in §5 separat begrenzt. Keine Policy wird als Java-Fehler-/Publikationssemantik ausgegeben.

## 2. Originalmethoden, Patchschichten und zusätzliche Beobachtung

Referenz: Minecraft Java 1.21.8, Protokoll 772, DataVersion 4440; Paper Build 60, Commit `29c8822d90899c89d2689338e81a98f690bcba12`; Temurin 21.0.12.1+1. Originale Inner-JARs, sämtliche Libraries, offizielle Mappings und das vollständige 454-Dateien-JDK werden durch die unveränderte Preflight-Prüfung vor/nach den Läufen gehasht. Vanilla Inner `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`, Paper Inner `c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`.

**Quellenlektüre** und **genuine Ausführung** bleiben getrennt. Ganze Base-/Applied-Dateien und Source-/Feature-Patchdateien der ursprünglichen 131-Dateien-Closure bleiben in Runde 1 gepinnt. Maßgebliche Methoden: `NoiseBasedChunkGenerator.applyCarvers`, `CarvingContext`, `WorldCarver.carveBlock/carveEllipsoid`, komplette Cave-/Canyon-Carver, `CarvingMask`, `WorldgenRandom.setLargeFeatureSeed`, Aquifer samt Fluidpicker, `SurfaceSystem.topMaterial`, `SurfaceRules.Context.updateXZ/updateY` und lazy Bedingungen, `BiomeManager`, `MultiNoiseBiomeSource`, `Climate.ParameterList/RTree`, `NoiseChunk`, Heightmaps und ProtoChunk-Writes. Gelesene relevante Patchschichten: Generator-/CarvingContext-Sourcepatches, BiomeManager-Moonrise-Zoom, WorldGenRegion/Holder-Carrier, Bit-Inlining, Anti-Xray und die Paper-Overworld-Ressourcenbedingung. Die übrigen Mechaniken der ganzen Moonrise-Patchdatei sind keine hier geprüfte Serverabnahme.

Zusätzlich werden ganze Base-/Applied-`Xoroshiro128PlusPlus` und `XoroshiroRandomSource` eingefroren; ihre Zustandsübergänge/Zero-Normalisierung bzw. `nextInt(bound)` begründen §5. Der Noise-Basisconstructor kopiert vorhandene Permutationen und rundet Offsets; dessen metered 259 Kopierschritte sind von den unmetered ursprünglichen Shuffle-Ziehungen der vorgelagerten Routerbindung zu unterscheiden. Diese Vorbereitung verspricht keine CPU-Zeitgrenze für diese Ziehungen.

`tools/prepare_observer.py` leitet einen **neuen** Helfer mechanisch vom unveränderten Round-1-Helper ab; das Script enthält die exakten Ersetzungen, `observer-derivation.json` deren Source-/Ergebnishashes und Beobachtungsscope. Er führt die echten U/F/R/D-Pfade auf genuine bis SURFACE erzeugten Chunks aus. U und F bleiben uninstrumentiert. Source-Abfragen werden am tatsächlichen `generationSettingsGetter` erfasst; ihre Quartposition folgt dem verifizierten originalen kalten Source-Ordinal. Bei originalem R/D-`SurfaceRule.tryApply` wird der originale memoized Supplier transparent delegiert und der originale BiomeManager durch eine transparente NoiseSource-Beobachtung geführt. Nur der tatsächlich verbrauchte erste `get()` protokolliert das vom echten Zoom ausgewählte Quart und das echte Biom. Kein zweiter Lookup und kein vorgezogener Zugriff. Die Getter/Settings werden anschließend wiederhergestellt.

Neue getrennte Dateien pro Backend: `biome-queries.tsv` (geordnete Source/Fix-up-Anfragen), `lazy-consumption.tsv` (null/ein konsumierter Source-Request), `controlled-nonconsumption.tsv`. Alle 28 historischen Original-TSVs der beiden Backends bleiben bytegleich. R und D besitzen dieselbe neue Query-/Consumption-Folge; normale und Python-`-O`-Läufe sind bytegleich.

| Neue genuine Beobachtung, je Backend und Phase R oder D | Ergebnis |
| --- | --- |
| vollständige Gruppen | 73 |
| kalte Source-Abfragen | 73 × 289 = 21097 |
| echte Fix-ups / konsumierte Biomabfragen | 355 / 355 |
| tatsächlich nicht konsumierte natürliche Fix-ups im Corpus | 0 |
| kontrollierte originale `BlockRuleSource(stone)` | `stone`, Supplier-`get()` genau 0 |

Der tokenbasierte unabhängige Checker rekonstruiert die interleavte Folge aus den originalen B-/T-Ereignissen, Source-Reihenfolge und echten T-Positionen. Fünf Mutationen ausschließlich neuer Evidenz (Source-Reihenfolge, Quart-Y, Biom, fehlende lazy Zeile, zusätzliche Query) werden normal/`-O` verweigert. Die alten `tops.tsv`-Nachlaufdiagnosen bleiben Diagnosen; nur die neue Aufzeichnung belegt den tatsächlichen Verbrauch.

Das unveränderte Reviewer-`EdgeProbe.java` wurde frisch mit beiden echten Backends normal/`-O` wiederholt: Original-Zoom bei Seed 0, Block `(1,-63,12)` → Quart `(0,-17,3)`; 60/256 kontrollierte Spalten erreichen −17. Echter builtin RTree, Target `[0,0,-10500,0,0,0]`: kalt `deep_ocean`, nach den zwölf gepinnten Forward-Targets `mushroom_fields`. Gemeinsamer TSV-Hash `96f656e305ce97c93875e1439c1571aa78ccfba6fe7bc65204f52c90b81721c4`. Das sind **kontrollierte Originalmethodenproben**, keine erfundenen natürlichen Gras-/Carverchunks und keine Aussage, dass ein natürlicher Corpus-Fix-up dieses Tie-Target erreicht.

## 3. Geschlossener laufeigener Producer und lazy World-Hook (CR11-B1)

Der Lauf besitzt genau eine frische Paper-NoiseChunk-Instanz für das Ziel, einen frischen Aquifer mit `schedule=false`, einen kalten `BiomeSelectionSession` auf dem immutable originalen `minecraft:overworld`-RTree und einen Paper-`BiomeZoom` mit einmal aus dem identischen WorldSeed abgeleitetem ZoomSeed. Settings: nur Overworld/large_biomes/amplified mit ursprünglicher Geometry `(-64,384)`, `legacy_random_source=false`, FlatBedrock disabled. Die explizite Quelle ist `BiomeSourceIdentity::MultiNoise(minecraft:overworld)`; sie wird geprüft, nicht aus NoiseSettings erraten. Es gibt keinen öffentlichen Rawarray-, Callback-, Fixture- oder frei zusammengesetzten Climate-Eingang.

**Chronologie:** History beginnt unmittelbar vor der ersten Source kalt. Für `dx=-8..8` außen und `dz=-8..8` innen wird `(qx,qy,qz)=(4·sx,0,4·sz)` über sechs tatsächliche Paper-Climate-Wurzeln → `BiomeClimateTarget::from_climate` → originalen Selector aufgelöst. Die konstante Liste `[cave,cave_extra_underground,canyon]` folgt danach. Carverindex 0/1/2, `setLargeFeatureSeed(seed+index,sx,sz)`, Starttest und Geometrie bleiben Runde 1. Echte lazy Fix-up-Queries werden genau an ihrem Zugriff zwischen diese Source-Abfragen eingeschoben. Kein Reset bei Quelle, Carver, Fix-up oder Segmentwechsel; keine Vorabberechnung aller Sources.

**Additiver konkreter Hook:** Der öffentliche `SurfaceRuleSession::new/evaluate` bleibt auf seinem Stored-Resolver. Gemeinsame private Auswertung erhält intern ein konkretes `BiomeProvider<'call>`-Enum mit `Stored(&SurfaceBiomeResolver)` bzw. `DirectCarver(&mut DirectCarverBiomes)`; kein Traitobjekt oder externer Callback. Der Provider wird durch die vorhandene Auswertung weitergereicht und ausschließlich im bestehenden `biome()`-Memo-Miss aufgerufen. `temperature` benutzt dasselbe Memo. `NoRule` bzw. eine Regel ohne Biom-/Temperaturzugriff konsumiert **null** Climate-/Selector-Queries. Wiederholte Biom-/Temperaturzugriffe innerhalb eines Fix-ups konsumieren höchstens eine.

`evaluate_carver` ist crate-privat. Sein Kontext hält dieselben per-column/per-block Caches und nutzt dieselben Regeln, Noise-/Temperaturmethoden, P4-Prüfungen und Live-Höhen. Für jeden `topMaterial`-Fix-up wird `begin_column` erneut ausgeführt und danach genau ein `evaluate` mit `(y-1,1,1,hasFluid?Some(y):None)`. Damit entsteht wie im originalen frischen `SurfaceRules.Context` pro Fix-up kein übernommener XZ-/Y-Regelcache, während die direkte Selector-Historie und Producer-Caches weiterleben. Private Fehlerauswertung unterscheidet Surface- und Direct-Fehler; der öffentliche Stored-Pfad erhält keine neue öffentliche Direct-Fehler-/Provider-API.

Der vorbereitete Lauf hält Noise/Aquifer/Selector/Zoom und den separaten Regel-Kontextzustand als Felder. Ein temporärer Direct-Provider borrowt bei der Auswertung getrennte mutable Felder; kein selbstreferentielles Struct. Der private Aquifer-Einstieg setzt ausschließlich den tatsächlichen Blockcontext und ruft `compute(...,0.0)` auf; `schedule` und Locations/Statuses werden nie ersetzt. Unterhalb/einschließlich Lava-Level −56 wird der Aquifer wie im Original übersprungen; die anschließende Flaglesung kann daher **stale** sein. Sie bleibt über alle Source-/Carvergrenzen erhalten.

**Private Halo, keine öffentliche Erweiterung:** Aus unveränderten Zentren `[-1874999,1874998]` plus Source-Offset `[-8,8]` folgen Source-Chunks `[-1875007,1875006]`, Source-Quart-X/Z `[-7500028,7500024]`, physische Source-Ursprünge `[-30000112,30000096]`. i64-checked Rechnung precedes Cast/Arbeit. Source-Y ist 0. Fix-up-Block-Y `[-63,311]` erreicht durch `(y-2)>>2` plus Eckauswahl Quart-Y `[-17,78]`, physisches Climate-Y `[-68,312]`. Für Fix-up-X/Z im Ziel ergeben sich `4c-1..4c+4`, vollständig innerhalb des bereits größeren Source-Halos. Keine gespeicherte Y-Klemmung.

Nur ein geschlossener `CarverClimateQuart`-Eingang darf diese abgeleiteten Source-/Zoom-Adressen in der privaten NoiseChunk-Methode `sample_carver_climate` verwenden. Er prüft Requestart, Run-Zentrum, Source-Offset oder tatsächliche Zoomausgabe, X vor Z und Y, danach alle Produkte. Der allgemeine öffentliche `ClimateQuartPos`-/Climate-/Blockcap bleibt unverändert. Negative floor-/shift-Semantik wird original erhalten. `meter-model-*` prüft beide Seiten der genannten Grenzen bei N−1/N/N+1 als **Designarithmetik**; echte Rust-Halo-/Publiccaptests bleiben Runtimepflicht. Randgruppen behalten `full`.

## 4. API, Zulassung und Veröffentlichung (CR11-B2)

Eigener World-Modul `carver`, private Felder, Rustdoc auf allen öffentlichen Items. Kandidat:

- `SeededCarvers::bind(&SeededSurfaceColumns, CarverBindBudget)` erbt Seed/Settings/Alphabet; Zensus 35+52+3=90, Budget `1..=65536`, Tests 89/90/91. SIN wird statisch referenziert und nie in eine Bindung kopiert.
- `prepare_run(router, rules, temperatures, explicit_source, chunk, corners)` erstellt einen **frischen vollständig zugelassenen** `PreparedCarverRun`. Er trägt keinen öffentlichen Query-/Reset-/Budgetsetter. Corners behalten GEN-05.10 A2: sie müssen aus GEN-05.3 für identische Seed/Settings/Chunk stammen; diese Herkunft ist aufrufervertraut und durch Producer-Replay zu prüfen.
- `carve_chunk(CarverInput { surface: &SurfaceChunkBuffer, run: &mut PreparedCarverRun }, CarverWorkBudget, &mut CarvedChunkBuffer)` erlaubt genau einen Lauf des Prepared-Objekts. `Ready → Running → Complete/Poisoned`; keine Wiederaufnahme. Drop/Deallokation des Prepared-Objekts liegt außerhalb des gemessenen Carver-Aufrufs. Private Testkonstruktoren dürfen N-Grenzen/Fehler injizieren; Produktion erhält keine fremde mutable Noise-/Selector-/Surface-Session.
- `CarverReport` nennt tatsächliche Carver-/Surface-Arbeit, Starts/Writes/Masken/Fix-ups sowie exakte private Noise-/Selector-Gesamtzähler als drei u64-Limbs. Keine truncating `usize`-Ausgabe privater großer Zähler.
- `CarvedChunkBuffer` besitzt States, Fluidbitmap, Live-`WORLD_SURFACE_WG`, Maske und optionale Published-Identität. Seine Accessoren geben nur veröffentlichte vollständige Inhalte frei. `SurfaceChunkBuffer` wird nur gelesen und bleibt Published.

`prepare_run` prüft vor einem Noiseconstructor oder einer Source-Abfrage: Bindungsidentitäten (Seed, Settings), explizite Quelle, Geometry, Paper-Zoomprofil, Ziel-/Haloarithmetik, laufeigene Native-Caps; dann vollständige Allowances in der Reihenfolge Noise, Selector, Surface. Danach die bereits zugelassene immutable RTree-Konstruktion/Bindung, Noiseconstructor, Materialstate-Auflösung und Aquiferconstructor. Fehlgeschlagene Vorbereitung veröffentlicht kein Ready-Objekt. Die private Constructor-Allowance wird dabei tatsächlich benutzt, nicht zunächst mit einem öffentlichen zu kleinen Budget gebaut und nachträglich aufgefüllt.

`carve_chunk` setzt zuerst den Zielpuffer Empty. Präzedenz vor jeder Source:

1. `InconsistentBinding(Seed, dann Settings)` zwischen Bindung und Prepared-Regeln.
2. `InconsistentSurface(NotPublished, Seed, Settings, Chunk)` gegen das Prepared-Zentrum.
3. `InconsistentProducer(Seed, Settings, Source, Geometry, ZoomSemantics, Chunk, Lifecycle)`; nur Ready ist zulässig.
4. Private Regel-Kontextzulassung gemäß GEN-05.9-Identitätsreihenfolge; typisiert `Rules(kind)`.
5. Noch vorhandene reservierte Noise-/Selector-/Surface-Allowances gegen ihre **Restanforderung**; nur Ready ohne öffentliche Konsumption. Der Constructor hat aus derselben Noise-Gesamt-Allowance bereits abgebucht. `bound_prepare=117744161352+16384+631=117744178367`; Noise-Restanforderung = `N_noise−117744178367`, Selector `18716174375`, Surface `45120000`, nicht gesamte Noise-Schranke noch einmal. Nachweis `used_prepare ≤ bound_prepare` ist Teil der Ready-Publikation.
6. `WorkBudgetExceeded` bei Carver-Allowance unter `63912717682`.
7. Kopie/Maskenreset, Running, Source-/Carverlauf; erst nach vollständigem Erfolg Published und Complete.

Budgets sind zuvor validierte Typen; InvalidBudget precedes Konstruktion. NativeLimit precedes die betreffende Allocation. Erweiterte typisierte Fehler: Producer-/Provider-Mismatch, `ProducerWorkExceeded(Noise/Selector/Surface)` mit exakten Limbs, `Producer(NoiseChunkErrorKind)`, `Biome(BiomeErrorKind)`, vorhandene `Rules`, `UnclassifiedState`; keine Heap-Nachrichten. Private Meterfehler haben eigene exakte Payload, keine truncierte Umdeutung in öffentliche `usize`-Limits. Im Loop auftretende defensive Fehler lassen den Puffer Empty und den Prepared-Lauf Poisoned. Keine Fortsetzung, kein AIR/Plains-Fallback. Das Wort „unerreichbar“ gilt für Work erst nach unabhängig angenommener Ableitung und tatsächlichem Runtime-Nachweis der Charges, nicht allein wegen der Messwerte.

## 5. Strukturelle kumulative Producer-Allowances

`bounds-normal.json`/`bounds-optimized.json` sind identische **abgeleitete Vertragsmodelle**, keine Rust-N-Tests und keine Java-Budgeterwartungen. Jede Charge liegt vor der gezählten Operation. Meterbuchhaltung selbst ist feste checked Integerarbeit; sie erzeugt keine rekursiven Zusatzcharges. Immutable Router-/Regel-/Treebindung hat ihre vorhandenen getrennten Constructorbudgets; hier werden die laufeigenen Noise-/Aquiferconstructor eingeschlossen.

### 5.1 Noise-/Splinewurzelkosten und amortisierte Caches

Die drei geschlossenen Settings haben identische strukturelle Form, unabhängig vom Seed. `carver_r2_probe.rs::cost` bildet über den akzeptierten Graph die Charge-Rekurrenz: eine Densityentry; Binary beide Children als sichere Schranke, Range input+max(branch), Alias ohne Entry; Normalnoise Samplerentry plus sämtliche Oktavslots; Spline Entry/Koordinaten/Binärsuche und höchstens zwei benachbarte Children; foreign Cachewrapper delegiert höchstens den Input. Kein erwarteter State oder Climatewert entsteht durch diese Rekurrenz.

Wurzel-Schranken (15 Felder, dann sechs Climatefelder in Originalreihenfolge):

```text
4,4,4,4,39,39,46,38,564,40,1108,3706,7,18,4 | 39,39,46,38,564,40
```

Climatequery inklusive sechs f64→f32-Charges kostet ≤772. Maximal Q=289+96000=96289 Queries, also 74335108. Der Selector wird dabei nie ausgelassen. Das Corpus beobachtet teils Flatcache-Hits; die Schranke setzt keine solchen Hits voraus.

Maximal 96000 Carvepositionen sind maskeneinmalig. Aquifer: höchstens 315 Location- und 315 Status-Misses ohne Eviction, Constructor 1+2·315=631. Hit/Lookup/Globalfluid/Pressureentry kosten konservativ ≤40 pro Aufruf; gemeinsamer Barrierroot wird höchstens einmal angefordert (≤4). Statusmiss: ≤64 branch-/lookup-/publish-Charges plus Erosion38, Depth564, Floodedness4, Spread4, Lava4, Sampleentry4, also ≤678 pro Miss. Factory-at und Locationpublish kosten zusätzlich 2 pro Locationmiss.

Ein unabhängiger vollständiger Jitter-/Grid-/Offset-Census enumeriert Grid-X/Z −1..1, Centerjitter 0..9, die tatsächlichen 13 Offsets und Quartfloor: **297** mögliche Preliminary-Spalten. Die alte Planning-/Beobachtungszahl 185 ist keine strukturelle Schranke. ≤315·13=4095 Requests zu je ≤297 Cachevergleichen; jeder der ≤297 einmaligen Misses scannt höchstens 49 Höhen (320 bis −64, Schritt8), je Entry+Wurzel1108, dann Publish. Die isolierten vollständigen Spalten stehen im bounds-JSON.

### 5.2 Constructor-Schranke

Geschlossene Builtins, keine beliebigen/custom Graphen. Konservativer Constructorcost `117744161352`, aus den unveränderten vorab geprüften Counts V=131072 Coldvisits, K=4096 Keys/Vertices, 8192 Edges, 128 Wrapper, 64 Oktavslots und Splinecaps. Teile vollständig in `noise_constructor_parts`:

| Chargeklasse | Obergrenze/Formel |
| --- | --- |
| kopierte Normalnoise-Paperbasis | 4096·2·64·260 |
| Coldvisitorentries | V |
| Identityvergleiche/Publikation | 5·V·(K+1) |
| Equalityhash, Kollisionen, Publish | 3·V·(70+K·71+1) |
| Multipoint-Boundsrefresh | V·64 |
| Splineplan/build | K·(16385+1024·64+16384+64+1024+16384+1024) |
| Blended-Paperbasis | K·40·260 |
| Flatcachefill | 128·25·(3706+1) |
| Validator/Edges/22 Roots/Wrapper/Scratch | 2·K+8192+22+128 |
| Empty-Blender-Grids | 50 |

Cold::intern publiziert höchstens K neue Nodes; höchstens drei Internschritte und fünf Identityschritte pro Visit decken die Seed-/Climate-/Wrapwege inklusive Splinearray-/Wrapperidentität. Keylänge höchstens70, K Collisioncandidates zu je Längencharge+70 Wordcharges. Splineplan besucht höchstens16385 Pendingentries und16384 Points; höchstens1024 Koordinatensuchen zu64, Buildlists/specs und64 Koordinaten. Die 3706 Flatfill-Schranke ist die größte tatsächlich strukturell abgeleitete Overworld-Wurzel, kein beobachteter Einzelcall-Maximalwert. Die Runtime muss diese Methodenzuordnung erneut gegen alle eigenen Charges prüfen; Änderungen an der Constructorimplementation invalidieren diese Summe.

### 5.3 Metered Aquifer-RNG: endliche echte Periodenbegründung

10/9/10-bounded-int zieht im akzeptierten Aquifer mit Xoroshiro128++. Jede Rejectioniteration ist metered. Eine „genau drei Draws pro Location“ wäre falsch. Die echten `Xoroshiro128PlusPlus.nextLong`-Zustandsübergänge aller 128 Basiszustände und die Constructor-Nullnormalisierung werden frisch aufgerufen (`PeriodProbe.java`, actual CodeSource geprüft). Die Quelle zeigt einen linearen GF(2)-Zustandsübergang; die Ausgabe ist getrennt.

`check_period.py` prüft über diese echten Übergänge P=2^128−1, vollständige Primfaktorisierung, T^P=I sowie Rang128 von T^(P/p)−I für jeden Primfaktor p. Deshalb hat jeder nichtnull Zustand genau Periode P. Constructor(0,0) normalisiert tatsächlich auf nonzero. Der echte Basiszustand `(lo=1,hi=0)` liefert Output-low32=131073; dieses Output erfüllt die originale Annahmebedingung für9 und10. Jede Bahn erreicht daher binnen höchstens P Draws einen angenommenen Wert. Der Beweis ist kontrollierte Algebra über Originalmethoden, keine natürliche Chunkerwartung und kein empirischer All-seed-Test.

P=`340282366920938463463374607431768211455`; ≤315·3=945 bounded Calls ergeben RNG-Allowance
`321566836740286847972889004023020959824975`.
Diese sehr große endliche Schranke begrenzt den **Arbeitszähler**, keine brauchbare Wartezeit. C11 lässt unmetered Legacy-Carver-Rejections origin-getreu weiterlaufen; weder C11 noch diese Periodenbegründung verspricht CPU/TPS oder eine praktische Seedzeitgrenze.

### 5.4 Literale Run-Summen und notwendige private Erweiterung

| Noise-Teil | Einheiten |
| --- | ---: |
| Noiseconstructor | 117744161352 |
| Materialblocks-Auflösung | 16384 |
| Aquiferconstructor | 631 |
| 96000 Hit-/Barrierpfade | 4224000 |
| Locationfactory/publish | 630 |
| Statusmisses ohne Preliminary | 213570 |
| Preliminaryvergleiche | 1216215 |
| Preliminaryscan/publish | 16139574 |
| alle Climatequeries | 74335108 |
| maximaler metered RNG-Verbrauch | 321566836740286847972889004023020959824975 |
| **Noise gesamt N_noise** | **321566836740286847972889004023138800132439** |

Selector: echter Tree 9112 Nodes, 7593 Leaves, 1519 Branches, 9111 Edges. Jeder Node wird höchstens einmal betreten, jeder Edge höchstens einmal versucht. Distanz kostet1+7=8, Prior-Distanz pro Branch ≤8, Edgeprüfung plus Child-/Selecteddistanz plus Vergleiche ≤19. Pro Query ≤2+9112+8·1519+19·9111=194375; Q Queries ergeben **N_selector=18716174375**. Immutable Treebuild: unverändertes öffentliches Maximum67108864; tatsächlich10845171, getrennt vom Runmeter.

Beide Run-Schranken überschreiten den unveränderten öffentlichen268435456-Cap. **Konkreter Erweiterungsvorschlag für Root, noch nicht implementiert/freigegeben:** Der interne SampleWork-/Selector-Chargepfad erhält einen ausschließlich privat konstruierten `CarverCumulativeMeter`-Modus; öffentliche `new`/Budgettypen bleiben im bestehenden Modus. Privater Modus besitzt `limit` und `used` als checked unsigned192, drei little-endian u64-Limbs; Overflow/limit refusal precedes Operation und ändert keinen Counter. `charge()` prüft `used < limit`, inkrementiert checked genau1 und führt einen Segmentrest von höchstens268435456. Am Segmentende wird ausschließlich der Segmentrest aus `limit-used` fortgesetzt; das unveränderte kumulative `used` ist immer die Autorität. Ein Segmentwechsel allokiert nicht und resetet kein SampleWork-Objekt, keine Noise-/Selector-Session, keine History, Caches, Epochen oder Flag.

Ein privates `new_with_carver_meter` speist den bereits zugelassenen Meter vor der ersten Constructorcharge in denselben Noiseconstructor ein; öffentliches `fresh_paper_noise_chunk` bleibt auf dem öffentlichen Modus. Analog erhält der Selector einen geschlossenen privaten Constructor, kein öffentliches Budgetupgrade. Private Ausführung verwendet ausschließlich exakte private Counter-/Fehleraccessoren. Bestehendes öffentliches `work_used()->usize` und öffentliche SampleWorkExceeded-Payload werden nur vom öffentlichen Modus erreicht; es gibt keinen truncating Cast/Overflow-Fallback des privaten Counters in diese API. Diese Trennung ist Teil der Erweiterung, nicht eine spätere Reparatur. Keine neue Dependency oder unsafe Integerlibrary: checked Add/Sub/Compare über drei Limbs.

Die Erweiterung umfasst ausdrücklich die verschachtelte Fehlerkette: interner `ChargeFault::{Public{limit:usize,used:usize},Carver{limit:[u64;3],used:[u64;3]}}` und privater Core-Resultadapter durch Noisebasis, RNG, Spline, Densityevaluate, Chunk-/Aquifer und Selector. Öffentliche Adapter bilden ausschließlich Publicfaults auf die unveränderten bestehenden öffentlichen Fehler ab; der private Carveradapter erhält den Widefault bis zum Runerror. Insbesondere darf `density/evaluate.rs`' heutige Spline-`BudgetExceeded`-Umsetzung mit `work.used()` nicht vom Widepfad benutzt werden. Dafür müssen die gemeinsamen privaten Samplingkerne Widefaults vor jeder öffentlich schmalen Fehlerumsetzung durchreichen. Keine duplizierte Noise-/Splineformel, kein öffentlicher Callback oder anders metered Erfolgsweg; Root muss diesen erweiterten internen Refactorumfang zusammen mit dem Meter vor Runtime prüfen. Die native Designprobe testet diese noch fehlende Fehleradaption ausdrücklich nicht.

Noise höchstens `1197929817215676784414384529080759` Segmente; Selector höchstens70. Limit/used je24Bytes, Segmentremaining4Bytes plus konkreter Enum-/Alignmentoverhead, zur Runtime literal zu messen. Alle Summen passen in192Bits. N−1 verweigert **vor Konstruktion/erster Source**, N und N+1 lassen zu. `meter-model-*` prüft Admission, beide64-/128-Bit-Carrys,192-Bit-Limit und wirkungslose refused Charge als Designmodell. Es beweist keinen implementierten Rustmeter. Bis Roots unabhängiges Review die Erweiterung und diese strukturellen Summen annimmt, bleibt CR11-B2s Runtime-Gate offen; die vorhandenen öffentlichen Caps werden nicht für „genug“ erklärt.

## 6. Carver-Arbeit, SIN und Native-Kapazitäten

Runde-1-Geometrie bleibt G=63831207759, Blockarbeit B=74304000, **R_alt=63905511759**. `count≤112`, 197 Tunnelschritte inklusive Kinder, 10990 Caveellipsoide und112 Canyonellipsoide pro Start bleiben mit der originalen Boundaryprobe belegt. Zusätzlich tatsächlich auszuführen/zu zählen:

- Kopie98304 States +1536 Fluidwörter +256 Höhen; Reset1536 Maskenwörter +Empty/Published2 = **101634** Charges. Kopierpayload **406528Bytes**, Maske separat12288Bytes. Keine erneute Initialhöhe-Suche erforderlich.
- Sourceproviderentry289; höchstens96000 lazy Fix-ups mit64 LCG-Schritten +8 Ecken +1 Memo +1 privater Entry: **7104289** Charges. Climate-/Selector-/Regelcharges bleiben eigene Meter.
- **R_neu=63912717682** (`R_alt+101634+7104289`), weiterhin <2^36. `CarverWorkBudget` unverändert `1..=2^36`; N−1/N/N+1 am neuen R. Shape-/Schreib-/Heightmaparbeit bleibt im alten G/B enthalten.

Surface-Regelallowance ≤96000·(2+1+467)=**45120000**; vorhandenes `SURFACE_RULE_OVERWORLD_CHUNK_WORK=45908736` reicht. Jeder Fix-up beginnt seinen eigenen Regel-XZ-Kontext; direkte Climate-/Selectorarbeit wird zusätzlich in deren Runmetern abgerechnet. Ein frischer vollständig zugelassener Regel-Kontext benötigt keinen unsichtbaren Regelbudgetreset.

`tools/import_sin.py` liest den unveränderten genuine Original-Export aller65536 SIN-Rawbits, prüft vollständige Indizes und Digest und erzeugt mechanisch eine statische `[u32;65536]` sowie little-endian Binärbytes. Keine Sinusformel oder Handabschrift wird Erwartung. Ganzer Rawbit-Roundtrip normal/`-O` identisch; originale sin.tsv SHA `6e16407bd7721398a296fb2a247ed2cb779e81fa997e1272d8d501c0e836d8ba`; RawLE SHA `cdfaec6870788e193dff3f1ddfa3ca7d3f1a897042d78365a1fe3a433be3627d`. **Statische262144Bytes** separat, unveränderlich und ohne globale mutable Initialisierung. Unterstützte Targetbits bleiben Original-f32; `from_bits` ist der Zugriff.

Native, x86-64/Rust1.99.0, Debug=Release; neue Zahlen sind **Designlayout**, noch keine Runtime-/Allocatorabnahme:

| Objekt | Wert/retained bzw. Cap |
| --- | --- |
| CandidateCarvedChunkBuffer | Wert1104 +393216 States +12288 Fluid +12288 Maske = **418896Bytes**, vorgeschlagener unveränderter448KiB-Cap |
| gleichzeitig geborgter SurfaceChunkBuffer | akzeptiert **406592Bytes**; zusammen mit eigener Candidateausgabe **825488Bytes** |
| SurfaceColumns-Bindung | akzeptiert **10816Bytes**, zusätzlich separat zur neuen Carverbindung |
| Carverbindung | Cap4KiB; 90-Zensus, Actual-literal erst Runtime; SIN nicht eingerechnet |
| NoiseChunkSession/Aquifer/Selector | Wert1192/80/64Bytes vor vorgeschlagener Metererweiterung; dynamische Producerbytes separat |
| immutable Overworldtree | **1181618Bytes** vor eventuellem Arc-Header; shared/borrowed Bindung, keine Run-Kopie |
| bestehende Noise-Caps | Graph2MiB, Keys2MiB, Coldtemporary4MiB, Caches8MiB, simultaner Scratch4MiB; Preliminary2048 Einträge, Aquifer≤315 belegbare Slots, separater12KiB-Cap |
| Carverscratch | Canyon384·4=1536Bytes; Tunneltiefe2; Regelstate bisher400Bytes, Direct-Enum/Meter-Alignment zusätzlich zu messen |

Die Summe825488 enthält keine Producer-/Tree-/Bindungs-/Meterbytes und ist keine vollständige Prozesssumme. Runtime muss jede retained Capacity, gleichzeitige owned/borrowed Kategorie, Konstruktorpeak, Arc-Overhead und Scratch literal berichten. Publiccaps bleiben unverändert; Änderungen an Nativecaps erfordern Rootentscheidung. Vorbereitung/Bufferkonstruktion dürfen begrenzt allozieren und werden separat gezählt. Der bereits vorbereitete `carve_chunk`-Aufruf muss null Heapallocationen und keine Producer-Deallokation zeigen, kalte Debug-/Release-Prozesse mit4096Bytes Positivkontrolle, Erfolg und jede Refusal. Diese Vorbereitung enthält keine solche Carver-Allokationsmessung.

## 7. Echte Ganzlauf-Designprobe ohne Einzelcallreset

`tools/prepare_native.py` prüft Roots komplettes1431-Dateien-Archiv und kopiert es ausschließlich in die neue Area. `native-overlay.json` und `native-overlay-files/` enthalten exakt fünf Änderungen: cfg(test)-Oktavcensus in `noise.rs`/`noise/normal.rs`, cfg(test)-Splinecharge-Rekurrenz in `spline.rs`, Testmodulregistrierung in `material.rs`, neues `carver_r2_probe.rs`. Alle übrigen Archivbytes/-Mitgliedschaften bleiben gleich. Build offline/locked, eigener Target, `--jobs2`, kein gemeinsamer WIP-Build.

`extract_native.py` projiziert echte R-Events und die neue tatsächliche Querybeobachtung **in originaler Interleavingfolge**. 71 zugelassene Gruppen werden gefahren; beide original verweigerten Gruppen bleiben als solche erhalten. Nativeinput SHA `8e093fd16d53b4cf2ecc639f24c9d2ab1928ca2418da756beddd26761c1d3418`.

Eine frische NoiseChunk-/Aquifer-/Selector-Instanz je Gruppe; Konstruktor, Climate- und Aquiferarbeit teilen tatsächlich denselben weiterlaufenden akzeptierten Noise-Meter. Kein per-call `SampleWork::new`, Sessionreset oder Historyreset innerhalb einer Gruppe. Der cfg(test)-Climate-Einstieg umgeht ausschließlich die öffentliche Halo-Zulassung für die in §3 abgeleiteten privaten Adressen. Die Probe implementiert noch keinen vorgeschlagenen192-Bit-Meter; der echte Corpusverbrauch passt in den alten öffentlichen Cap.

| Ganzlaufmessung, normale/`-O`-Orchestrierung, Debug/Release bytegleich | Ergebnis |
| --- | ---: |
| gefahrene vollständige Gruppen | 71 |
| genuine Aquiferpositionen, States und Flags verglichen | 42117 |
| tatsächliche Biomqueries/Zoomqueries verglichen | 20874 /355 |
| davon Sourcequeries | 20519 =71·289 |
| Noiseconstructor min/max | 563000 /571824 |
| vorbereiteter Noise-Meter max | 572652 |
| gesamter Noise-Verbrauch max pro Lauf | 905681 |
| Selector-Verbrauch max pro Lauf | 974373 |
| beobachteter Einzelcallverbrauch max | 36293 |
| Summe Aquifer-/Climate-/Noise-/Selectorwork über71 Läufe | 6785422 /6223487 /53547408 /47183452 |

Finaler Native-TSV-Hash **`e66f2c96d2640cbfa8a749488cf832e2442f3b3d08217457ed7fe8401337d95e`**. Ältere Probeoutputs mit/ohne Layoutcensus bleiben im Inventar als frühere Designläufe. Messwerte ersetzen keine strukturelle Zulassung; keine Multiplikation36293·96000 als „Beweis“.

Nicht in dieser nativen Probe ausgeführt: vollständige GEN-05.9-`topMaterial`-Komposition, Produktions-Carver, vollständiges native GEN-05.10→Carver-Replay,192-Bit-Erweiterung und native Allokations-/Boundarytests. Die echte Originalbeobachtung ersetzt diese Runtimepflichten nicht.

## 8. Unveränderte Fixtureklassen und bindende Runtimepflichten

Alle **71 full /2 refused** Gruppen und sämtliche Zeilen des historischen Elf-Dateien-Fixture bleiben unverändert. `policy/single_biome_surface/0` bleibt `refused:InconsistentResolver(Source)`, `outside/normal/0/+x` bleibt `refused:CentreOutsideDomain`; deren Originalwerte werden niemals Runtimeerwartung. Edgegruppen bleiben full, einschließlich privat über±30M hinausgehender Sources. Keine neue natürliche Chunkzeile wird aus kontrollierten Tree-/Zoom-/Nonconsumptionproben erzeugt.

**Gesamte Runde-1-§16-Auflagen werden fortgeführt**, mit den folgenden additiven Präzisierungen:

1. Replay: komplette71 Paperchunks samt FINALdigest, Livehöhen, Fluidmenge, Maske und vollständigem `cfg(test)`-Trace aus echten akzeptierten Produzenten; beide Refusals typisiert/Empty. S1-Token vor Carver prüfen. Vanilla bleibt Kontrolle; keine Formel-/Rust-Erwartung.
2. Produzenten: echte .3/.4/.5/.6/.7/.8/.9/.10-Komposition, alle355 originalen Fix-ups mit echter lazy Directregel, ursprüngliche Ergebnisse und Actualquart/Biom/Queryfolge. Sourcequeries289 je Lauf, gleiche History mit consumed Fix-ups. Warmcache bleibt eigene spätere Aktivierung.
3. Direkte Bits: vollständige65536-SIN-Rawbits/Digest, sämtliche63291 Starttests, RNG-/Seedingfolge und jede ursprüngliche Boundaryzeile, ausdrücklich `uniform`-f32-Oberende und Canyon112.
4. Aquifer: jede ursprüngliche Tracezeile; Flags über Source-/Carvergrenzen einschließlich stale Lava. Kein per-call reset; kumulative Reports stimmen mit Chargereplay überein.
5. Alle ursprünglichen quellbasierten Kanten: mycelium-Fix-up, NoRule ohne Write, Fluidfix-up, Maskenskip über Carver, −56/−55,312/−62, keine Kinderrekursion, Luftspitzenrescan, Barriere ohne Write bei gesetztem Maskenbit. Zusätzlich echter lowerQuart−17, kontrollierter History-Tie, Nonconsumption und temperaturgeteiltes Memo, gleiche konkrete Evaluatorfunktion in Stored/Directpfad.
6. Arbeit/Kapazität: neue R-/Noise-/Selector-/Surface-N−1/N/N+1-Vorprüfung; Constructor- und Remainingcharges; Public0/1/cap/cap+1; native Bytecaps;90-Zensus; carried64/128/192-Carries; Segmentboundary direkt vor Source und in lazy Aquifer-/Fix-up-Branch. Jeder Vergleich erfasst vollständige Cache-/History-/Flag-/Epochzustände und unveränderte Randomfolge. Tatsächlich implementierter private Meter darf keine Boundary durch Reset bestehen lassen.
7. Jede Refusal gemäß §4 einzeln und kombiniert; Präzedenz; Output bereits Published vor Refusal →Empty. Private after-admission defensive Fehler poisonen Readyrun und publizieren nichts; Inputs bleiben unverändert.
8. Eigentum/Doku: eigenes `carver`-Modul, World-private Hooks; README/INVARIANTS dokumentieren Directprovider, Coldscope, private Halo-/Meterdomäne, getrennte Counters und alle Policies. Neue Fixtureinstrumentierung separat vom unveränderten historischen Fixture mit vollständigem Manifestvalidator übernehmen.

**RR10-H1–H5 unverändert und nicht als durch die Vorbereitung erledigt ausgegeben:**

- H1: Produktions-`build_chunk` mit Materialblock bei y319 (`with_states`), i4=320, erster Aufruf y319 und published Höhe; MutantM13.
- H2: Lava in eroded und frozen Extensions; eroded bricht nur an Wasser ab, frozen-Wasserklausel akzeptiert keine Lava; M14/M15.
- H3: literal10816 SurfaceColumns und406592 SurfaceChunkBuffer auf dieser Toolchain plus echte neue Carver-/Provider-/Metergrößen.
- H4: externe Allokationsprobe zusätzlich Binding-/Material-/Session-/UnclassifiedState-Refusals, erwartet0/0/0 mit Positivkontrolle, sämtliche neue Carverrefusals; Producerkonstruktion separat.
- H5: A1-Puffertest beginnt mit tatsächlich Published, danach Empty bei Refusal.

Vollständige World-Debug/Release-, all-target-Clippy-`-D warnings`, Fmt und Rustdocchecks gehören zur späteren autorisierten Runtime auf Roots exakter Baseline+Worldoverlay; die fokussierte private Designprobe ist keine bestandene Wholecrateabnahme dieser noch fehlenden Runtime.

## 9. Reproduktion, neuer Freeze und verbleibende Gates

Alle Tools und Outputs liegen unter `.codex-tmp/gen05_11-r2/`. Runner verweigern bestehende Outputs; Review verwendet eigene neue Labels. Der Originalrunner akzeptiert `original-[a-z0-9-]+` innerhalb der Area. Nach dieser reinen Freshlabel-Ergänzung wurden zusätzliche `original-final-normal`/`original-final-optimized`-Läufe gestartet; deren abschließender byteweiser Vergleich gehört zwingend zum Freeze. Andere Originalalgorithmen und historische Auswahl wurden nicht geändert. `development_limit=0`; die originale263-Evaluation-Scanauswahl bleibt gepinnter Input und wurde nicht neu ausgesucht.

```text
python3    tools/run_original.py original-review-normal
python3 -O tools/run_original.py original-review-optimized
python3    tools/run_edges.py edges-review-normal
python3 -O tools/run_edges.py edges-review-optimized
python3    tools/run_period.py period-review-normal
python3 -O tools/run_period.py period-review-optimized
python3    tools/check_period.py period-proof-review.json
python3    tools/bounds.py bounds-review.json
python3    tools/meter_model.py meter-model-review.json
python3    tools/import_sin.py sin-review-normal
python3 -O tools/import_sin.py sin-review-optimized
python3    tools/run_native.py native-review-normal
python3 -O tools/run_native.py native-review-optimized
python3    tools/check_observer.py observer-review
python3    tools/guards.py guards-review-normal
python3 -O tools/guards.py guards-review-optimized
python3    tools/final_check.py final-check-review.json
```

Vom Repo-CWD sind die vollständigen `.codex-tmp/gen05_11-r2/`-Präfixe nötig. Die Checker lesen die eingefrorenen gelieferten Standardoutputs; die neuen Freshoutputs werden zusätzlich vollständig gegen diese verglichen. Der Native-Reproducer benötigt das verifizierte private Archiv+Testoverlay oder einen frischen durch `prepare_native.py` aufgebauten Kandidaten; weder gemeinsame Storagequellen noch neue Dependencies sind dafür erlaubt. Auch der Edge-Runner akzeptiert neue `edges-[a-z0-9-]+`-Labels; finale Freshreplays und Guards nach dieser reinen Labelergänzung sind separat inventarisiert.

Zusätzliche Guards: fünf tatsächliche Java-Environmentrefusals über den realen neuen Entrypoint, zwei tatsächliche Periodenhelper-JVM-CodeSource-Refusals vor Samples/Output, deklarierte Shadow-/Foreignoutput-Faults. Die ursprünglichen16 Guardarten und der Reviewer-Observer bleiben unveränderte frühere unabhängige Evidenz, keine neu behauptete eigene Ausführung aller16. Logs/Fehlversuche bleiben sichtbar; sie sind keine Erwartungen. Normale/`-O`-Guardresultate unterscheiden allein die explizite Interpretermodusangabe.

`input-pins.json` pinnt zusätzlich die neue Source-/Toolautorität. `final-check-{normal,optimized}.json` dokumentiert unabhängige vollständige Byte-/Membership-/Freezeprüfungen; `freeze-manifest.json` pinnt diesen Vertrag und jede maßgebliche neue Datei ohne sich selbst zu hashen. Private `native-archive/` und `native-target/` sind explizit ausgenommen; Archivauthority und alle fünf Overlaybytes sind stattdessen vollständig gepinnt. Logs und frühere Fehlversuche bleiben inventarisiert, mit klarer Design-/Diagnosekennzeichnung. Exakte Vertrags-/Input-/Freezehashes werden Root separat gemeldet, ohne zirkulären Hash im Vertragsinhalt.

Verbleibend: unabhängiges Rootreview von Vertrag/Originalbeobachtung,192-Bit-Erweiterung und groben Constructor-/RNG-/Selector-Schranken; danach erst gesonderte Runtimeworkorder und sämtliche §8-Abnahmen. Coldscope ist ausdrücklich ein neuer Carverlauf mit289 kalten SourcecarverBiome-Caches und kalter RTree-History. Ein echter warmer Nachbarcache kann Sourcequeries auslassen; Threadlocalhistorie kann aus früheren Jobs stammen. Live-/Warmcacheparität braucht vor Aktivierung einen getrennten Cache-/Scheduling-/Historyvertrag und echte Serverevidenz. Kein All-seed-Paritäts-, vollständiger Server-, Survival-, Plugin-, alte/blendende Chunks-, Struktur-, Feature-, Persistenz-, Licht-, TPS/RSS- oder Performanceabschluss wird aus diesen Proben abgeleitet.
