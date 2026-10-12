# GEN-05.11 — unabhängiges Vertrags-/Original-Oracle-Review

**Urteil: BLOCKING_CONTRACT.** Das gelieferte Oracle führt echte Originalmethoden auf echt erzeugten SURFACE-Chunks aus und ist unabhängig reproduziert. Der Vertrag ist noch nicht für eine Runtime freizugeben: C5(a) ersetzt Originalsemantik durch einen nicht angenommenen Stored-Biome-Pfad; außerdem fehlt eine konkret zugelassene, kumulative Budget-/History-Regel für den Aquifer und die nötige direkte Biomquelle. Das Urteil betrifft die Vorbereitung, keine vorhandene Carver-Runtime.

Review am 12.10.2026 unter Roots ausdrücklicher Delegation. Autorität: Roots Main `bfe580e3`, World unverändert gegenüber akzeptiertem GEN-05.10 (`a912b1c2` und dessen Source-Freeze). Keine Git-Aktion, kein weiterer Agent, keine Produktionsänderung und kein Storage-Baseline. Geschrieben wurden nur dieses neue Dokument und `.codex-tmp/gen05_11-independent-review/**`. AGENTS.md, CLAUDE.md, PAPER_REFERENCE.md, World-INVARIANTS, vollständige Workorder und vollständiger konkreter Vertrag wurden gelesen. Historische Original-/Runtime-Evidenz wurde nur gelesen.

## 1. Blockierende Befunde

### CR11-B1 — direkte Original-Biomquelle, Suchhistorie und Halo fehlen im Runtimevertrag

**Ort:** Vertrag §1, §3.1, §6, §8 C1/C5, §11, §13.1/3, §14 und §16; insbesondere `GEN-05_11_OVERWORLD_CARVERS.md:521` und `:687`. Original: `NoiseBasedChunkGenerator.applyCarvers:256,275–279`, `SurfaceRules.Context.updateY:317`, `Climate.RTree.search:388–389`. Oracle: `CarverOracle.java:806–823`.

Original `applyCarvers` erzeugt einen `BiomeManager.withDifferentSource` über `BiomeSource.getNoiseBiome(..., random.sampler())`. Der Zoom wählt eine Quartposition; diese wird direkt und ohne gespeicherte Y-Klemmung aufgelöst. Die laufende RTree-Suchhistorie beeinflusst Gleichstände. Die Source-Biomabfrage vor jedem Quell-Carver und die lazy Fix-up-Biomabfrage verändern dieselbe Historie. Die Stored-Arrays stammen aus der anderen `createBiomes`-Abfragefolge.

Die 54 gleichen Carver-Listen erlauben die **konstante Carverentscheidung**, aber nicht automatisch das Weglassen der Source-Biomabfragen, sobald Fix-ups die direkte Quelle verwenden. C1(a) und C5(b) sind deshalb nicht unabhängig kombinierbar. Im frischen Oracle wurden sämtliche 289 `carverBiome`-Caches kalt erzeugt; es erfolgen 289 Source-Abfragen in X-außen/Z-innen-Reihenfolge, mit dazwischenliegenden echten Fix-ups. Ein späterer warmer Nachbarcache kann eine Source-Abfrage auslassen und verlangt einen gesonderten History-/Cachevertrag vor Aktivierung.

Unabhängige Original-Grenzproben, beide Modi bytegleich, liefern konkrete Reproducer:

- `BiomeManager.getBiome`, Weltseed 0, Block `(1,-63,12)` fragt Quart `(0,-17,3)` an. Der echte Original-Zoom erreicht Quart-Y −17 in 60 der 256 getesteten Spalten bei Block-Y −63. GEN-05.8s Stored-Accessor klemmt dagegen auf −16. Dies ist eine kontrollierte Koordinatenprobe mit einem deklarierenden NoiseSource-Callback, **kein** natürlich erzeugter Gras-/Carverzustand und kein erfundener Oracle-Blockwert.
- Der echte eingebaute Overworld-RTree liefert für den quantisierten Target `[0,0,-10500,0,0,0]` kalt `minecraft:deep_ocean`, nach den ersten zwölf deklarierten Forward-Targets des akzeptierten GEN-05.5-Originalcorpus `minecraft:mushroom_fields`. Das ist eine echte Original-Tree-Probe mit kontrollierten Targets, **kein** Nachweis, dass ein bestimmter natürlicher Carver-Fix-up diesen Target trifft. Sie widerlegt jedoch eine allgemeine History-Unabhängigkeit dieser Quelle.

Die bisherigen `tops.tsv`-Diagnosen entfernen die RTree-Historie **nach** dem Carverlauf und fragen anschließend nur die Fix-up-Positionen ab. Sie sind keine Aufzeichnung der tatsächlich während `tryApply` verbrauchten Biome. Dieses Review ergänzt eine private transparente Beobachtung: während des originalen R-/D-`tryApply` wird ausschließlich `Context.biome` durch einen Supplier ersetzt, der beim tatsächlichen ersten `get()` den ursprünglichen memoisierten Supplier delegiert und dessen Ergebnis aufzeichnet. Keine vorgezogene oder zusätzliche Biomabfrage. Ergebnis: 710 Beobachtungen aus R und D, 355 echte Fix-ups, alle tatsächlichen Biome gleich der bisherigen Diagnose; alle 13 bestehenden Paper-TSVs bleiben bytegleich, U/R/D/F und Ereignisfolge bleiben gleich. Die Diagnose ist damit für diesen Corpus gestützt; sie beweist weiterhin keine all-seed Gleichheit zum Stored-Pfad.

**Erforderliche Korrektur vor Runtimefreigabe:** C5(b) als verbindlichen Originalpfad festlegen; C5(a) nicht stillschweigend annehmen. Einen additiven, geschlossenen World-Hook und den zugehörigen erweiterten Oraclevertrag konkretisieren:

1. Eine private `DirectCarverBiomes`-artige Run-Instanz verbindet GEN-05.6 `BiomeZoom` mit GEN-05.3s tatsächlichen sechs Paper-Climate-Wurzeln und GEN-05.5 `BiomeSelectionSession`/`BiomeClimateTarget::from_climate`. Seed, Settings, Overworld-Quellpreset, Chunk und Paper-Zoom werden geprüft. Kein Stored-Fallback, kein Firstmatch und keine öffentlichen Callback-/Rawarray-Eingänge.
2. Der bestehende öffentliche GEN-05.9 Stored-Pfad bleibt unverändert. Ein crate-privater `evaluate_carver`-/Provider-Einstieg benutzt die direkte Instanz **erst beim originalen lazy Biom-Memo-Zugriff**, höchstens einmal pro Fix-up. Vorab ein Biom zu berechnen und als Wert zu injizieren würde die Original-Historie bei Regeln ohne Biomzugriff verändern.
3. Historie beginnt für diesen frischen Run ausdrücklich kalt, wie U/R/D/F. Vor jedem Quell-Carverblock werden die originalen Source-Queries `(4*sx,0,4*sz)` in der originalen Reihenfolge ausgeführt; die Carverliste darf danach konstant sein. Fix-up-Queries werden in derselben Instanz dazwischen eingeordnet. Kein Reset je Carver, Quelle oder Fix-up. Ein warmer Server-/Cachepfad ist separat offen.
4. Die Source-Halo muss privat, abgeleitet und vollständig zugelassen werden. Bei den unveränderten Zentrumgrenzen `[-1874999,1874998]` sind Quellen `[-1875007,1875006]`, Quart-X/Z `[-7500028,7500024]`, Blockursprünge `[-30000112,30000096]` möglich. `sample_climate`/`ClimateQuartPos` erlauben heute öffentlich nur die ursprüngliche ±30M-Domäne. Ein origin-getreuer privater Halo-Hook kann dem akzeptierten Material-Halo-Muster folgen, muss aber seine eigene Domäne und Grenzen beweisen. Öffentliche Climate-/Quartcaps werden nicht stillschweigend erweitert; Randgruppen werden nicht nachträglich umklassifiziert.
5. Originalinstrumentierung muss direkt in der lazy Fix-up-Abfrage Position, ausgewähltes Quart, verwendetes Biom und geordnete Source-/Fix-up-Anfragen aufzeichnen. Das kann um den genuine `Context.biome`-/`biomeGetter`-Supplier geschehen, ohne zusätzliche Queries; jede Erweiterung behält U als Kontrolle und prüft R/D gegen U. Neuer eigener Freeze, historische 11-Dateien-Fixture unverändert. Die neue Evidenz muss auch Nichtverbrauch, History-Ties und die ungeklemmte untere Quartgrenze als kontrollierte Tests unterscheiden.

Diese Komposition ist mit den akzeptierten World-Algorithmen möglich. Die heutige öffentliche SurfaceRuleSession borrows allerdings ausschließlich `SurfaceBiomeResolver`; ihr `biome()` ruft ihn fest auf. Deshalb fehlt ein konkreter additiver Consumer-Hook. Zusätzlich fehlen privater Climate-Halo und die Arbeits-/Fehleradmission aus CR11-B2. §14s derzeitige Aussage, alle Eingaben seien bereits vorhanden und nur Aquifer-/Buffer-Hooks fehlten, ist für den nun erforderlichen Originalpfad unvollständig.

### CR11-B2 — kumulative Producer-Allowance und Vorabzulassung sind nicht festgelegt

**Ort:** Vertrag §8.3–6 und §10 (`:587`), §11/§16 C2/C6; akzeptiertes `density/chunk.rs:28,142`, `biome_selection.rs:23,308`, `sample_work.rs:30`; Designprobe `carver_design_probe.rs:119`.

Carver-`R` ist ein anderer Meter als Noise-/Aquifer- und Selector-Arbeit. §10 nennt 36.293 Einheiten pro Aquifer-Aufruf als Designzahl und kündigt eine Ableitung aus ≤96.000 Aufrufen sowie ≤315 Location-/Status-Misses an. Die gelieferte Probe setzt aber **vor jedem Aufruf** `session.work = SampleWork::new(1_000_000)`. Sie beweist Zustand/Flag und einen beobachteten einzelnen Maximalverbrauch, weder den strukturellen Maximalverbrauch noch die gesamte zugelassene Session-Allowance. Die akzeptierte `NoiseChunkWorkBudget`- und `BiomeWorkBudget`-Obergrenze ist jeweils 268.435.456; die naive Multiplikation mit dem beobachteten Einzelmaximum ergäbe bereits 3.484.128.000. Das ist **kein Unmöglichkeitsbeweis**: eine richtige amortisierte Cache-Miss-Ableitung kann wesentlich kleiner sein. Diese Ableitung fehlt derzeit.

Hinzu kommt eine tatsächliche Vertragslücke: `CarverInput` nimmt eine fremde mutable NoiseChunkSession an. Fresh/Complete, korrekte Identität und Abfragebereitschaft sagen nichts über ihren verbleibenden Meter. Vorab ausgeführte Climate-/Preliminary-Queries können ihn verbrauchen, ohne Lifecycle oder Identität zu ändern. Eine passend identifizierte, abfragebereite Session mit geringer Rest-Allowance wird nach §8.3 zugelassen; der Carver kann später wegen `WorkExceeded` im Aquifer scheitern. §8.6 nennt dies für zugelassene Eingaben unerreichbar, obwohl eine Restbudget-Prüfung fehlt. Mit C5(b) entstehen zusätzlich Climate- und Selector-Meter; `BiomeSelectionSession::reset` setzt zugleich die Original-Historie zurück und darf nicht als unsichtbare Budgeterneuerung dienen.

**Erforderliche Korrektur:** Eine numerische, strukturell hergeleitete Run-Allowance pro Producer festlegen, einschließlich Constructor-/Wrapperkosten, Hit-Pfad, maximal einmaligen Location-/Status-/Preliminary-Misses, benötigter Noise-Samples, maximal 289 Source- und ≤96.000 lazy Fix-up-Biomqueries sowie Selector-Traversal. Aufzeigen, wie sie unter den unveränderten akzeptierten Caps zugelassen wird. Falls eine private begrenzte Meterfortsetzung nötig ist, muss sie als eigener Vertrag Historie, Aquifer-Flag und Caches erhalten und einen endlichen Gesamtzähler haben; kein öffentlicher Capwechsel oder stilles Sessionreset. Rest-Allowances werden in der dokumentierten Fehlerpräzedenz **vor der ersten Quelle** geprüft, oder der Lauf besitzt eine frische, bereits vollständig zugelassene Producer-Instanz. Laufzeitbedingungen erhalten exakte N−1/N/N+1-Tests für diesen kumulativen Anspruch sowie Success-/Refusal-Allokationsproben. Eine tatsächliche laufende Probe ohne Einzelaufruf-Meterreset ist erforderlich. Die Beobachtungszahl 36.293 bleibt als solche beschriftet.

## 2. Bestandene unabhängige Prüfung

Der Reviewer rief die unveränderte, gepinnte `CarverOracle.java` mit dem gepinnten JDK und echten Inner-JARs in einem privaten Wrapper auf. Kanonische Source-/Input-/Membership-/CodeSource-Prüfungen wurden beibehalten; nur das Ausgabeverzeichnis wurde in die eigene Review-Area gelegt. Normal und Python `-O` führten jeweils Scan, Vanilla und Paper frisch aus: alle 28 Original-TSVs sind bytegleich zum Autor und zueinander. Beide Läufe verwenden development_limit 0. Das abgeleitete `project_originals` dient nur der Vertragsmodellprüfung, nie zur Erzeugung von Erwartungen.

Die realen Erzeugungspfade wurden im Helper geprüft: Datapack-Laden über `TagLoader`/`RegistryDataLoader`/PendingTags, echtes Preset/RandomState, neun genuine `createBiomes`, tatsächliches `fillFromNoise`, akzeptierter originaler `buildSurface`-Callee, Status SURFACE. U hat den gehaltenen NOISE-Aquifer; F erzeugt ihn über originalen `createNoiseChunk` frisch; R delegiert transparente Aquifer-/Regel-/GenerationSettings-Proxys; D ruft die tatsächlichen Original-Carver auf. Der einzige D-Loop ist eine überprüfte Zerlegung, nicht die Erwartungsquelle. Settings werden nach R/D zurückgesetzt. Originchecks betreffen originale Klassen, Libraries und frische Helperklassen.

Unsafe-Carrier sind eng begrenzt und zutreffend beschriftet: WorldGenRegion liest center/generatingStep/cache/level; echte Vanilla GenerationChunkHolder-Futures bzw. Paper NewChunkHolder-Completions beliefern `getChunk`. 280 frische STRUCTURE_STARTS-Chunks sind bei `generateStructures=false` ausreichend; acht BIOMES-Nachbarn und echtes SURFACE-Zentrum bleiben vorhanden. Der ServerLevel-Carrier liefert Worldgen-Registry und Papers `generateFlatBedrock=false`. Kein voll konstruiertes ServerLevel oder ChunkStatusTasks-Lauf wurde vorgetäuscht. Original-Methodenparität innerhalb dieses Geltungsbereichs ist genuine; voller Serverbetrieb folgt nicht daraus.

Aus meinen frischen Läufen wurden die elf kompakten Dateien normal und `-O` regeneriert, vollständig bytegleich zum Autorenfixture. Ein eigens geschriebener zweiter Token-Checker rekonstruiert Klassifikation, Start-/Quellenreihenfolge, S1/FINAL-Digests, beide Heightmaps in beiden Phasen, Maskencensus, Aquifer-Position/Zustand, Flagkontinuität, Fix-up-Fluidhöhe, per-Carver-Zähler und behaltene Post-Processing-Präfixe. Normal = `-O`:

| Prüfung | Ergebnis |
| --- | --- |
| Gruppen / Replay | 73 / 71 full, 2 refused |
| Starttests / Starts / Carver mit Netto-Writes | 63.291 / 4.807 / 148 |
| Maskenbits | 65.198 |
| Aquifer Luft / Wasser / Lava / null | 26.628 / 14.306 / 115 / 1.597 |
| Aquifer-Aufrufe / Flag-Lesungen / Fix-ups | 42.646 / 41.549 / 355 |
| rekonstruierte FINAL-Digests / Heightmap-Arrays | 73 / 292 |
| angehängte Fluid-Marken | 618 |
| Vanilla-FINAL ganz gleich Paper | 48; 25 geerbte S1-Unterschiede |

Die 16 Autor-Guardarten wurden jeweils normal und `-O` selbst ausgeführt, mit eigenen frischen Klassen für die drei tatsächlichen JVM-Originverweigerungen. Fünf Environmentfälle liefen zusätzlich über den echten privaten Einstieg mit beibehaltener kanonischer Preflight-Prüfung. Alle verweigern vor Samples/Output. Elf selbst erzeugte Einzelzellen-Mutationen (die zehn verlangten Kategorien plus veraltetes S-Flag) scheitern im unabhängigen Checker identisch normal/`-O`. Autor-Guard-/Mutationsberichte wurden zusätzlich geprüft und werden nicht als eigene Tests ausgegeben.

Post-Processing-Reihenfolge nach dem S1-Präfix lässt sich aus dem kompakten Trace allein nicht vollständig rekonstruieren, weil Lava-Level-Schreibpositionen darin nicht einzeln stehen. Bytegleiche Regeneration und die Original-/Modellpfade behalten jedoch die genaue Liste. Die publizierte Runtime bleibt gemäß C4 eine Menge; das ist ausdrücklich FerrumC-Policy. Ein privater Entwicklungsversuch, nur angehängte Fluid-Marken umzuordnen, wird deshalb vom reinen Token-Checker nicht diskriminiert; der endgültige Präfixkontrollfall verändert bestehende S1-Reihenfolge und verweigert. Beide privaten Versuche bleiben im Evidenzinventar.

Die scripted-draw BoundaryProbe wurde neu mit Originalmethoden ausgeführt; beide TSVs und Canyon-bytecodes gleichen dem Autor. Zusätzlich sind die tatsächlichen Cave-bytecodes abgelegt. `uniform(0.75f,1.0f)` und `uniform(0.8f,1.3f)` erreichen ihre oberen f32-Werte. Canyon castet `i2f → fmul → f2i` und kann 112 Schritte haben. Kinddicke kann auf 1.0 runden, `thickness > 1.0` verhindert die Rekursion. Die korrigierten Radien/Y-Spannen und geometrischen Formeln ergeben `G=63.831.207.759`, `B=74.304.000`, `R=63.905.511.759`; R liegt unter 2³⁶. Diese Schranke zählt erklärte Geometrie-/Blockeinheiten; Java-nextInt-Rejections sind nach C11 nicht begrenzt, daher kein harter CPU-/Zeitdeckel.

Die native Autorenprobe wurde mit Source/Inputs/Receipt sowie Debug-/Release-Gleichheit geprüft; sie ist weiterhin **Design-Evidenz**, keine Carver-Runtime-Abnahme und wurde hier nicht als eigener Rust-Lauf umgedeutet. Die fehlende tatsächliche GEN-05.9-Fix-up-Komposition bleibt eine Runtimeauflage.

## 3. Empfehlungen für Roots §13-Entscheidungen

| Entscheidung | Empfehlung und Grenze |
| --- | --- |
| C1 | Konstante Carverentscheidung zulassen; bei C5(b) alle history-wirksamen originalen Source-Abfragen trotzdem ausführen. Warmcache-/Serverhistory offen halten. |
| C2 | (a) frischer run-eigener Aquifer, Flag einmal false; CR11-B2 zuerst schließen. Keine Rücksetzung an Source-/Carvergrenzen. |
| C5 | Verbindlich (b) mit CR11-B1s lazy direktem World-Hook und expliziter History/Halo; (a) ist nicht angenommen. |
| C7 | (b) originale 65.536 Rawbits als unveränderliche statische Tabelle bevorzugen. Die native `f64::sin`-Probe belegt nur die geprüfte Plattform/Toolchain. Bei (a) müssten alle unterstützten Targets bitgleich validiert bzw. typisiert verweigert werden; kein stilles Vertrauen in libm-Portabilität. Statische 256 KiB gesondert ausweisen. |
| C9 | (a) eigener Puffer. Copy-Payload ist 406.528 B Zustände/Flagbitmap/Höhen plus eigener Maskenreset; gleichzeitig bleiben Surface-Puffer, Carver-Puffer und Producer-Caches erhalten. Exakte neue Retained-Bytes und Kopier-/Resetarbeit literal berichten, ohne RSS-/Speedclaim. |
| C10 | (a) strukturelle Vorprüfung beibehalten, ergänzt um Producer-Allowances. Eine praktische Kappe wäre eine neue mögliche Original-Verweigerung und bedarf eigener Rootentscheidung. |
| Modul | Eigenes `carver`-Modul; additive crate-private Hooks für Aquifer und direkte/lazy Biomquelle. |
| C3/C4/C6/C8/C11 | Im beschriebenen frischen Scope annehmbar; Mengen-PP, Atomic-Publication und unmeterte RNG-Rejections bleiben explizite Policies. D1–D5/P1–P6 nicht stillschweigend erweitern. |

C2s initiales false hat zusätzlich einen Quellenbeweis im hier zugelassenen vollen Heightsetting: die letzte NOISE-Zeile ist y −64; Aquifer ist der erste Materialfiller. Positive Dichte setzt das Flag false, ansonsten liefert der globale Fluidpicker dort Lava und setzt ebenfalls false. SURFACE fragt den Aquifer nicht erneut ab. Das gilt für vollständig erzeugte, ungeklemmte Builtins ohne Exception; nicht für beliebige anders hoch erzeugte oder manipulierte Chunks. F=U sowie 42.646 Zustand-/Flag-Paare stützen die konkrete Reload-Komposition. Die vier stale Lava-Level-Marken müssen im Runtime-Trace erhalten bleiben.

## 4. Präzise Runtime-Abnahme nach Korrektur

Die Vorbereitung erhält erst nach CR11-B1/B2 und neuem Freeze ein CLEAN_CONTRACT. Danach braucht ein separat autorisiertes World-Runtime-Ticket:

- Unverändertes Replay aller 71 full/2 refused-Gruppen, Produktions-S1 aus akzeptierten GEN-05.3/.4/.5/.8/.9/.10-Produzenten, sämtliche FINAL-Zustände nach Namen, Masken, Live-WORLD_SURFACE_WG, Fluidmenge, Starts, Zuordnungen und geordneten cfg(test)-A/S/T-Traces. Keine Klassenlockerung.
- Die 355 echten GEN-05.9-Fix-ups samt neu aufgezeichneten tatsächlich verbrauchten direkten Biomen/Quartqueries; Source-/Fix-up-Historyfolge und kalter Start. Kontrollierte untere Quartgrenze, Tiehistorie, lazy Nichtverbrauch und Rand-Halo mit N−1/N/N+1, ohne neue natürliche Chunk-Erwartungen zu erfinden.
- Sämtliche bestehenden §16-C3–C8-Grenzen, originale Float-/Canyon-Bytecodesemantik, beide Carverarten, Barrier-Masken, veraltetes Flag über Source-/Carvergrenzen, Mycelium/empty-top als kontrollierte Quellfälle und Live-Steep-Höhen nach eigenen Writes.
- Literal gepinnte neue Retained-/Scratchwerte, Bindungszensus und N−1/N/N+1-Caps; **zusätzlich** die exakt hergeleiteten kumulativen Noise/Selector/Climate-Allowances, Restmeter-/Lifecycle-/Identitäts-/Fehlerpräzedenz und ein Published→Empty-Test je Verweigerungsfamilie. Jeder Erfolgs- und Refusalpfad allokiert während des Calls null, geprüft separat in Debug/Release mit positiver 4096-B-Kontrolle; Konstruktion außerhalb des Calls separat zählen.
- RR10-H1–H5 unverändert: Produktionsscan bei y319, Lava/Wasser in beiden Extensions, literal 10.816/406.592 B, Binding-/Material-/Session-/UnclassifiedState-Allokationsverweigerungen sowie A1 mit bereits publiziertem Eingabepuffer.

Ohne diese Laufzeitabnahme gibt es keine Carver-Parität, natürliche Generatoraktivierung, Survival-/Plugin-, Persistenz-, Lighting-, Blending-/Struktur- oder Performancefreigabe. Gleichheit von Vanilla/Paper im echten Carver-Delta ist ausreichend; ein erfundener Unterschied ist kein Testziel.

## 5. Vollständige Artefaktpins und Grenzen

Alle 427 Autoren-Evidenzdateien und Vertrags-/Workorder-Pins wurden nach dem Review erneut gehasht; ebenso sämtliche 1.133 Input-Pins und 324 World-Dateien mit unveränderter Mitgliedschaft. Keine historischen Bytes geändert. Das vollständige private Reviewinventar umfasst 433 Dateien einschließlich Programme, originaler frischer Runs, Beobachtung, eigener Kontrollen, Diagnostik und gescheiterter privater Entwicklungsversuche. Es steht in `.codex-tmp/gen05_11-independent-review/freeze-manifest.json`; das Review-Markdown wird separat gehasht, um keinen zirkulären Hash zu erzeugen.

| Artefakt | SHA-256 |
| --- | --- |
| geprüfter konkreter Vertrag | `3f6ca7387ab19d0fcf063b9cb4bfbe0c534e7cedafa5a87e61f6107bb3f137ff` |
| Autoren-Freeze, 427 Dateien plus Vertrags-/Workorderpin | `6451131f336c011486edcfc91d1a43a38274f4e5e390b48e3b5ba72f88b29863` |
| 1.133 Original-/Producer-Inputpins | `27427e158bc05fa8ec6bf30577c54b0fdee2d38049cdbbd6db866fb3748e4837` |
| komplettes privates Reviewinventar | `69bbb587222da52015d01480bf56aed48f7bc716f12afc703fd8ebcb388a1ced` |
| private maschinelle Reviewzusammenfassung | `55a6da4d428f668db390bb91dd58efb0027793d9723b1f7628d8f7e18ae4244b` |
| tatsächlich lazy verbrauchte Paper-Biome, R/D | `ab237622de88292345812ded6b5305c8a0db54730db3c5bddc518dd9cdcb6a02` |
| unabhängige Original-Zoom-/Tree-Grenzprobe, beide Modi | `96f656e305ce97c93875e1439c1571aa78ccfba6fe7bc65204f52c90b81721c4` |

Grenzen: keine Produktions-Carverimplementierung, kein Cargo-Test des geteilten mutable Workspace, keine Review-Ausführung der nativen Autoren-Designprobe, kein voll gestarteter Server, keine nachgewiesene natürliche Chunkabweichung für C5s Ties/untere Quartgrenze, keine globale In-Memory-History-/Warmcacheparität, keine alten/blendenden Chunks, Strukturen, Features oder TPS/RSS-Messung. Die unabhängigen kontrollierten Proben belegen Originalmethoden und semantische Risiken; sie ersetzen keine genuine Chunkfixture.
