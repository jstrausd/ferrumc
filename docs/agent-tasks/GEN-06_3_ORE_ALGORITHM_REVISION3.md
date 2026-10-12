# GEN-06.3 Revision3 — Query53, typisierter Proof und vollständige Payloadbindung

12. Oktober 2026. **Vollständiger neuer Autorenkandidat zur unabhängigen Prüfung.** Die drei konkreten Revision2-Gegenbeispiele sind im neuen Werkzeug-/Entwurfsstand korrigiert und normal/−O geprüft. Dies ist kein unabhängiges CLEAN-Urteil und keine Runtime- oder Limitannahme. Alle ursprünglichen Pflichten und ungelösten Producer-/Native-/Lifecyclegates bleiben bindend.

Writes beschränken sich auf diesen neuen Bericht und `.codex-tmp/gen06_3-revision3/` (N). Kein Git, Agent, World-/Registry-/Productionedit, Dependency-/Lockfilewechsel, Cargo/shared Build, Original-JAR-Wechsel oder historischer Refreeze. Rootauftrag SHA256 `b022d82cf3f7785165a350fab1fc4a18988dabdccdd43dc37a8b8b0489fc16ef`, unverändert publiziert unter b1798614e63a0353721d6c744fc7ee5e6b23ca54. Root übernimmt Veröffentlichung und einen neuen unabhängigen Review.

## 1. Bindender vollständiger Kontext und Integrität

Gelesen: ursprünglicher GEN-06.3-Auftrag/-kandidat, r1 Reviewauftrag/-bericht, Revision2-Korrekturauftrag/-antwort, Revision2-Reviewauftrag/-bericht und vollständiger late Heightmapnachtrag; ganze GEN-03.1 original/r1/Revision2/Reviewkette und separate Registry-only GEN-03.1a-Vergabe; ganze GEN-06.1 original/Korrektur/Reviewkette und queued GEN-06.2; AGENTS, CLAUDE, PAPER_REFERENCE, archivierte Architektur/Simulation/ADRs und vollständige immutable Registry-/Worldinvarianten. Tatsächliche Java-Kernels/Callees und relevante Source-/Featurepatchteile wurden gelesen: Ore/Heightmap/Bulk/Sectionsetter, Placement/ConfiguredFeature/Feature/Biomefilter/Tagmatch, Wrapper/BitRandom/Xoroshiro, Region-/Chunkheight und ProtoChunkupdate, Decoration und FEATURES-Tasks/Pyramid. Die ganzen Patchdateipins sind keine Behauptung, alle fremden Moonrisemechaniken reviewed zu haben.

| Unveränderte Authority | SHA256 |
| --- | --- |
| Originalkandidat / Autorenfreeze | `49746a75ef24eb952ea73198741a6b80e262de8bec9da325cdf7e6dfa3ee758d` / `a919bc57099d1791f1021cf03aca9716fb90be5f4cb700c4fb2f824370252593` |
| r1 Bericht / Reviewfreeze | `efdf02482cc4317014d7c4543bbee14fdcd0d5c8716c4672c6767f158b74d27e` / `9462b5aa4cf8343d5fab7e4b950fbd60506196b13a847dbfb8d7100358ba7ef7` |
| Revision2-Antwort / Freeze | `b4b38a4f453cd7235d8e178cb915d3326d47337d4164744865b8617fe51f7381` / `8e0ca5579560dd1d21f9fe39ebefa5f467a104735e8bb0cf8dd360b5aff714a6` |
| Revision2-Reviewbericht / final enclosing Freeze | `08d1fb263bbe5a1261d386f05767f196f76e52b486467c3687ef44bcf8323d9a` / `e97293eb8197052b0a4cec25f2c1d04630bc1f989a8ac227bc25d24776e34f9e` |
| Heightmap Revision2 / vollständiger Review | `469933e39a0cd41e6374ae9f66479cecb702cd56419d6c30f4e7d4d855bd292e` / `f9578c75df327866d7de9e9759fd6d012dacc1a93b0b39adff3f75ceb98906c0` |
| Heightmap finaler Reviewfreeze | `bdea7cfd73cb7bbf983bd60656a13578deaeb01e8ebb8db3efe97ff4699b7300` |

N/tools/integrity.py streamt vor/nach den Proben selbst den **finalen** enclosing Review12543 Dateien/21699636198 B, den vollständigen Autor5897/14045912630 B und alle deduplizierten27779 Inputpins/28125050686 B. Historische/Input-/JDK-/Archivmembership und legal resolved Links mit separat gepinnten vollständigen Zielbytes werden geprüft; der late Heightmapreview zusätzlich nach lstat-Typ/Mode/Target und Rootmode. Specials/Mutationlinks werden nicht geöffnet oder gefolgt. Rootintake `.codex-tmp/orchestration/gen06_3-revision2-review-intake.json` ist Kontext; eigene vollständige Prüfung steht in before-/after-normal-/after-optimized-integrity.json. Der frühere primäre Reviewfreeze bleibt als frühere Membership erhalten; nur der enclosing Freeze definiert die finale12543-Mitgliedschaft.

Runtimebasis bleibt ausschließlich main47eadc4c, `.codex-tmp/gen05_11-runtime-root-base-storage47e/archive`,1490 Dateien/324 World, base.freeze `58ba13d6418133bb49a154682ba1d82533f63a5e6184c6c2a8b07c6c15a47a75`. Gemeinsame mutable Registry und frozen-unaccepted Carver sind keine Producerauthority. Minecraft1.21.8/772/4440, PaperBuild60/29c8822d, JDK21.0.12.1+1 und ganze Original-JARs/CodeSources bleiben gepinnt. Zusätzliche historische Archive bleiben reine readonly Kontexte.

## 2. Deutsche Antwortmatrix für alte und neue Befunde

| Befund / historischer Einwand | Konkrete Revision3-Antwort | Ergebnis und verbleibende Grenze |
| --- | --- | --- |
| CR63-B1 ursprünglich:1Mi reicht nicht für vollständige Region, fehlende Work-/Native-/Cleanupauthority | Eigenständiger Compositionentwurf aus R2 bleibt; echte Cursor/Core/Parentmeterparts, neun Candidates, vollständig nachgerechnete Copy-/Admin-/Cleanupreserven. Neuer konsistenter53+32 Querypfad unten. | Auf Entwurfsniveau konkret; Single1Mi unverändert. Compositecap/Native/Layout/Cleanup/Producer noch Root-/Runtimegates. |
| CR63-B1 HIGH / G31-R2-B1 HIGH: Getter53 versus eingefrorene8 und unbezahlter Adapter | Getter53 plus explizite Selection32 statt bisherigem Selectiongroup1; atomare85-Reservation am aktiven Parent. Alle12 Regionen/318 Featurepläne neu gestreamt. | Modell3904585 maxPP/289719 Margin; private Schnittstelle separat reviewbar, noch kein ausgeführter Rustgetter. |
| CR63-B2 ursprünglich: Markerstrip akzeptiert Originalslotmutation | Exaktes altes InverseMutationhelper frisch kompiliert/JVM-ausgeführt; alter Strip akzeptiert, neuer struktureller Proof verweigert. | Beide Profile normal/−O erhalten; keine Markerreceipt als computational Authority. |
| CR63-B2 BLOCKING neu: Type→String bootstrap arg0 trotz gleicher Druckdarstellung akzeptiert | Exakter Reviewer BootstrapMutation.java Count0-Control; alte Proofannahme reproduziert, neue typisierte Konstanten-/Operandtrees verweigern. | Valide definierte Klasse, echtes false→BootstrapMethodError/ClassCastException; komplette126/134 Methoden/elf Klassen/selected36 positiv. |
| CR63-B2 consequential Call/Field/Operand/Branch/Throwablekontrollen | Alle acht Autorenvarianten plus alle acht eigenen Reviewervarianten erneut tatsächlich JVM-ausgeführt; struktureller Proof verweigert jede. |32 Mutanten je Pythonmodus insgesamt, zusätzlich ursprünglicher Marker-/Bootstrapcontrol; echte Return/count/Throwablewirkung erforderlich. |
| CR63-B3 ursprünglich: optionale Structure/Censusprüfung verliert Sphären-/Branch-/Rawbits | Mandatory externe Original101-Authority und tatsächliche vollständige lossless Payload; alle34 alten ordinary Mutationen als wirklicher Candidate-Member erneut geprüft. |34 Payloadrefusals je Modus; optionaler Census ist keine Gleichheitsauthority. |
| CR63-B3 HIGH neu: plain+mutantgzip, hidden/nested mutantgzip, externer --trace | Exakt alle drei alten CLI-Erfolge reproduziert und durch eindeutige Representation, komplette Membership/Typen und membergebundenes --trace verweigert. | Beide Profile normal/−O:6 alte Falseacceptances/6 neue Refusals je Modus; alle zulässigen Payloadbytes vorher/nachher geprüft. |
| G31-R1-B1/B2/M1 aus getrenntem Heightmapreview | Dessen Präparations-/Design-/Validatorabschluss unverändert übernommen; Query53-Verbindung hier konkret korrigiert. | Keine Rustproducer-/IO-/Native-/TOCTOUannahme. Registry-only03.1a gewährt keinen Worldgetter. |
| CR61-B1/B2/B3, H1/H2 | CLEAN ausschließlich für alte kontrollierte Primitivevorbereitung bleibt. Equalitydiscovery→Identityindex, volle164 Indizes und Beobachtungsgrenzen unverändert. H1 war Reviewbeschreibungskorrektur: Originalguard bereits abs(block >>4). | Kein historischer Helperrepair; Nativeziele/volle Stage-/Core-/Section-/Producerabnahme bleiben getrennt. |

## 3. Query53 und konkrete Selection32-Schnittstelle

Der **gleiche reale checked Heightmapgetterkernel** ist später alleinige Implementierung für standalone QueryFirstAvailable/QueryHighestTaken/EmptyColumn und Ore. Seine feste Recipe bleibt genau `G24 +Kind6 +Index5 +Wordread1 +Extract3 +Range1 +Outputs3 +Return2 +Cleanup8 =53`. G24 folgt der vollständigen Heightmap-R2-Guardorder: Entry, minY, height, mask-range, mask-nonzero, Column-X/Z, Y-min/max, Owner, Seed, Settings, Source, Chunk, Version, Producer, Phase, Readergeometry/identity, Inputlength, retained/scratch/peak, permission; nichtbenötigte Slots feste NOPs. Kind6 prüft range/presence/state/statepermission/historypermission/phasepermission; Index5 worlddomain/mask X/Z, column, word, shift. Cleanup8 bleibt error-enum/ordinal/Resulttag/Returntoken/Reader-/Candidate-/oldownership-/operationborrowrelease. Dies ist eine Reservationrecipe, keine Behauptung53 tatsächlich ausgeführter nativer Aktionen. Kein State-/Readercallback, Prime oder Rebuild beim Getter.

**World-only Amendment nach angenommenen Voraussetzungen:** private konkrete `OreExecutionParts<'a>` mit sealed Varianten `Single(&mut SingleOwnerparts)` / `Composite(&mut ClosedCompositionOwnerparts)` ersetzt nur den internen Draw-/Chargecallee; beide borgen denselben echten exklusiven Cursor/Core/Workowner. Kein public Worktrait, Callbackmeter, cloned cursor/core oder zweite allowance. Im selben privaten Heightmapmodul wird `query_reserved(RealQueryView, QueryInput, SpentQuery53)` aus demselben Getter extrahiert; standalone reservation53 und Ore reservation85 liefern ausschließlich nichtclonebare ownergebundene spent footprints. `SpentQuery53` ist kein Producer-/Erfolgsreceipt und kein zweiter Zähler. Nur der konkrete Parentreservationcallee kann ihn privat erzeugen; die einzige Getterbody bleibt identisch. Keine externe bool-Autorisierung.

Ore nimmt einen echten einmaligen Pendingborrow. Der Caller borgt die bereits aktiven Parentparts und ruft private `ocean_floor_first_available(pending, real_region, world_x, world_z, execution_parts)`. **Vor Selection- und Mapreads** versucht genau dieser Parent checked atomar85 aus seiner unveränderten Runallowance zu reservieren. Accountingadmission gehört zur gleichen fest bounded Meteroperation wie sonstige Charges; kein rekursives Metering des Meters.85 besteht aus53 Getterfootprint und32 folgenden expliziten Adapterslots. Das alte im `CALLBACK height` gezählte Selectiongroup1 wird vollständig ersetzt; es bleibt kein zusätzliches1 stehen.

| Adapterreservation Slot | feste bounded Aktion |
| --- | --- |
| 1 | dispatch entry |
| 2 | Pending phase read |
| 3 | Pending phase compare |
| 4 | lease owner identity read |
| 5 | lease owner identity compare |
| 6 | catalog identity read |
| 7 | catalog identity compare |
| 8 | typed OCEAN_FLOOR_WG kind build |
| 9 | world X domain check |
| 10 | world Z domain check |
| 11 | chunk X arithmetic shift |
| 12 | chunk Z arithmetic shift |
| 13 | center X read |
| 14 | center Z read |
| 15 | checked delta X |
| 16 | checked delta Z |
| 17 | delta X interval check |
| 18 | delta Z interval check |
| 19 | nine-slot index expression |
| 20 | slot index bound |
| 21 | one fixed slot descriptor borrow |
| 22 | present descriptor check |
| 23 | descriptor chunk X read |
| 24 | descriptor chunk Z read |
| 25 | both descriptor coordinates compare |
| 26 | real map view borrow |
| 27 | fixed query authority reference pack |
| 28 | getter call entry |
| 29 | firstAvailable projection |
| 30 | query view borrow release |
| 31 | slot borrow release |
| 32 | return tag and fixed locals cleanup |

Slot19 ist genau `(chunk_z-center_z+1)*3+(chunk_x-center_x+1)`; keine Suche/Maphash-/neunfacher Scan. Slots9/10 prüfen die angenommene Worlddomain;15/16 checked Subtraktion,17/18 interval−1..1,20 range0..8. Slot25 vergleicht zwei feste i32Koordinaten, keine string-/payload-equality. Slot27 packt ausschließlich feste echte Authorityreferenzen; sämtliche Getterchecks bleiben separat in53. Gettercleanup beendet seine eigenen inneren Borrows; Adapter30/31 beendet die äußeren view/slot Borrows. Es wird kein identischer Borrow zweimal abgerechnet.32 ist fester Return-/Localcleanup, keine versteckte Destructoriteration; variable Ownertätigkeit gehört zu den vorhandenen vollständigen Cleanupreserven. Skipped Erfolgs-/Fehlerslots verbrauchen ihre schon reservierte Worstcasearbeit als NOPs.

Getterauthority bindet tatsächlichen Owner/Lifetime/Producer, ganzen Seed-/Settings-/Source-/Chunk-/Registry-/Mapversionsvektor, Geometry, Kind, Phase, Kindstate und History. Selection bindet zusätzlich Pendinglease/Katalog/Slots an denselben Parent und realen Regionowner. Fehlende WGmap oder stale/foreign history verweigert; keine missing→0/Air-Lücke. Immutable OCEAN_FLOOR_WG/Surface-WG-History stammt aus dem echten Stageproducer und wird nach direkten Ore-Sectionwrites **nicht** aus finalen States rekonstruiert. FirstAvailable ist `minY+relative`, empty−64 und Block319→320; ChunkAccess highest ist first−1, Regionhöhe first. Adapter29 projiziert den schon berechneten first-Wert und addiert kein weiteres1.

Fehlerpräzedenz bleibt: terminaler Owner/ungültiges Pending, Lease/Katalog/Sequenz und feste Header vor der allgemeinen Regionadmission;2048 Validationreservation, exakte neun Neighbor/Producerauthorities/Versions-/Capacities, Native/work/fallible reserve, Copy, wirkliche Biomefortsetzung. Am tatsächlichen Gatequery danach85-Reservation → Adapterguards in Slotorder → Getterguards/Kind/Index/Word/Range/Output in echter Heightmaporder → innerer/äußerer Cleanup → ursprüngliches Gatevergleichs-/Kernel-/Completion-/Publicationverfahren. Defensive Fehler nach85 sind terminale Oreerrors: Candidate verwerfen/Parent poisonieren, bereits belastete Arbeit nicht erstatten, keine weitere Candidate-/Planfortsetzung oder Publikation. Fehlgeschlagene gesamte85-Reservation führt keine Selection-/Map-/RNGoperation aus und erhält vorherige used/Core/publicationbytes; deren terminale Ownerfehlerpropagation ist zusätzlich unverändert Pflicht. Das Scalarreservationsmodell prüft den atomaren Reservationcallee selbst; es ist kein ausführbarer Poison-/Dropkernel.

N/adapter-{normal,optimized}.json prüfen mathematisch51 transaktionale Kontrollen und108 negative/asymmetrische signed Chunkseams; gleiche53/32/85footprints. Keine echte Registry/Heightmaphistory, Nativecleanup oder Rustoperation wird dadurch erzeugt. Kein Refund/Reset/Upgrade, Preparationmeter oder caller successreceipt. Single-key GEN-06.2 bleibt1.048.576; Whole Composite besteht als neuer getrennter Vorschlag.

## 4. Vollständiges neues Arbeitsmodell und unveränderte Nativegates

N/tools/models.py streamt komplette Originalregiontraces erneut normal/−O, prüft echte Stack-/Plan-/Queryzuordnung und rekonstruiert den gesamten alten finalen Ledger unabhängig. Alle318 perfeature Records je Profil/Modus enthalten unveränderten Key/Step/Index, Attempts, Coreentries und tatsächliche Queries sowie neue Query53-/Selection32units.13946 Queries und318 Pläne pro Profil/Modus sind harte requirements; beide Modeledgers sind vollständig gleich. Ein Attempt ist kein Query. Für jeden echten Query ersetzt53 die alte8 (+45) und32 die bisherige1 (+31): **76 zusätzliche Einheiten**,85 Gesamtreservation. Der reviewer53-only Zwischenvergleich3848413/345891 wird ausschließlich als historischer Gegenvergleich gehalten.

Vollständig enthalten bleiben: Copy1801845 (State1769481, packed Maps4050, Fluid27657, Counter441, PPterminalheads216), Validation2048, Fixed598119 (Cleanup524288, Nativeaktionen512, Header32769, Geometry2049, Auswahl352, Metadata1024, Biomegather36865, Publicationmetadata256, Constructor4). Volle PP adds262144. Ereigniskernel plus Mask/Handle/Sphereinitialisierung, Spherebody/native Setter, Air-/Release-Endheads, Primitiveemission/Dispatch/Completion, Planadmin und finaler Publicationdebit bleiben einzeln ausgewiesen. Selection32 ersetzt nur das tatsächliche Gatequerygroup1; die separate352 Katalogausschwahlreserve bleibt eigene Arbeit. Getter/Adapter53/32 schließt eigene Guards/Return/terminales Cleanup; Regionownerdestruktion bleibt eigene vorab ausgegebene reserve. Keine erfolgreiche unverändert eingefrorene Region-JVMworkload wurde allein für ein grünes Modell wiederholt.

| vollständiger Kontrollplan | tatsächliche Queries Seed0 /−1 | neues maxPP Seed0 /−1 | Margin zu pending2^22 Seed0 /−1 |
| --- | ---: | ---: | ---: |
| full30 | 1812 /1651 | 3904585 /3843015 | 289719 /351289 |
| plains | 962 /801 | 3413176 /3351820 | 781128 /842484 |
| badlands | 1012 /851 | 3456339 /3394653 | 737965 /799651 |
| dripstone_caves | 962 /801 | 3440081 /3377759 | 754223 /816545 |
| lush_caves | 1008 /847 | 3725315 /3664252 | 468989 /530052 |
| windswept_hills | 1700 /1539 | 3507351 /3445809 | 686953 /748495 |

**Usefulminimum für das größte vollständige deklarierte Modell:3.904.585; Margin289.719 zu weiterhin pending4.194.304.** Full30 Seed0 delta137712 (=1812×76), ohne vollePP3642441; Seed−1 delta125476. Alle zwölf erklärten Modelle passen rechnerisch. Kein neuer Cap wird angenommen, kein Seed-universelles Completion-/Performanceversprechen. Perfeaturecensus vollständig in models-*-complete.json; folgende Full30-Zellen werden von den echten fünf Biommengen ausgewählt, ohne neu nummerierte ore-only Indizes:

| echter Key | Step/Index | Queries Seed0 /−1 |
| --- | --- | ---: |
| minecraft:ore_dirt | 6/0 | 7 /7 |
| minecraft:ore_gravel | 6/1 | 14 /14 |
| minecraft:ore_granite_upper | 6/2 | 0 /0 |
| minecraft:ore_granite_lower | 6/3 | 2 /2 |
| minecraft:ore_diorite_upper | 6/4 | 0 /0 |
| minecraft:ore_diorite_lower | 6/5 | 2 /2 |
| minecraft:ore_andesite_upper | 6/6 | 1 /0 |
| minecraft:ore_andesite_lower | 6/7 | 2 /2 |
| minecraft:ore_tuff | 6/8 | 2 /2 |
| minecraft:ore_coal_upper | 6/9 | 30 /30 |
| minecraft:ore_coal_lower | 6/10 | 20 /20 |
| minecraft:ore_iron_upper | 6/11 | 810 /650 |
| minecraft:ore_iron_middle | 6/12 | 10 /10 |
| minecraft:ore_iron_small | 6/13 | 10 /10 |
| minecraft:ore_gold | 6/14 | 4 /4 |
| minecraft:ore_gold_lower | 6/15 | 1 /1 |
| minecraft:ore_redstone | 6/16 | 4 /4 |
| minecraft:ore_redstone_lower | 6/17 | 8 /8 |
| minecraft:ore_diamond | 6/18 | 7 /7 |
| minecraft:ore_diamond_medium | 6/19 | 2 /2 |
| minecraft:ore_diamond_large | 6/20 | 0 /0 |
| minecraft:ore_diamond_buried | 6/21 | 4 /4 |
| minecraft:ore_lapis | 6/22 | 2 /2 |
| minecraft:ore_lapis_buried | 6/23 | 4 /4 |
| minecraft:ore_copper_large | 6/24 | 16 /16 |
| minecraft:ore_copper | 6/25 | 16 /16 |
| minecraft:ore_clay | 6/27 | 46 /46 |
| minecraft:ore_gold_extra | 6/28 | 50 /50 |
| minecraft:ore_emerald | 6/33 | 724 /724 |
| minecraft:ore_infested | 7/4 | 14 /14 |

Unveränderter kompletter simultaner Nativevorschlag15.042.707 B/Margin1.734.509 zu pending16MiB; packed Maps31968 B, Geometry2048+1184+864+4096=8192. Beide Input/CandidateStates, Fluid-/Countersets, volle Tickvalues, beide PPsets, geborgte Biome/Zoom/Membership/Classification/Sources, echte Cursor/Core/Meter, Entity- und zusätzliche Tickreverse-/Hashindexcapacities, Headers/Stack/Temps/Alignment bleiben vollständig inventarisiert.53/32 benötigt ausschließlich feste PODlocals innerhalb derselben Partitionen; das ist ein Layoutziel, keine native Messung. Ursprüngliche volle ShortListindices7078752 B werden durch Borrowvalues nicht unsichtbar und verletzen weiterhin das neue2Mi-indexziel. Cleanup131072 Nodes/depth32/524288 units und512 Nativeaktionen brauchen echte Producerdescriptors für sämtliche Input-/Candidate-/Sidecar-/Container-/String-/last-refcountdropverpflichtungen, ownergebundene precharged Cleanuppermits, echte Reservefault-/Unwind-/Lifetime-/Finalcommitproben. Keine implizit kostenlose alte Chunkdestruktion.

## 5. CR63-B2: tatsächlich typisierte computational Gleichheit

N/tools/ProofVerify.java behält **alle** ursprünglichen Templates/Selectedmethoden und ersetzt ausschließlich verlorene Stringequality durch strukturelle immutable `ConstantKey`, `InstructionKey`, `HandlerKey`-Trees. Gleichheit arbeitet auf Typ und Feldern, nicht auf Drucktext; `Arrays.equals` wird nur für wirkliche ZIPbyteidentität benutzt. Diagnostischer Recorddruck ist keine computational Gleichheit.

| Operand/Konstante | gebundene Identität |
| --- | --- |
| Integer /Long /String | eigenständiger Kindtag und genauer Wert |
| Float /Double | Kindtag und floatToRawIntBits/doubleToRawLongBits; NaNpayloads/±0 bleiben verschieden |
| ASM Type | Kindtag, genauer sort und Descriptor |
| Handle | Kindtag, tag/owner/name/descriptor/interfacebit |
| ConstantDynamic | Kindtag, name/descriptor, typisierter BootstrapHandle und rekursiv geordnete vollständige Args |
| LDC /invokedynamic | dieselbe typisierte constantKey-Implementation; indy zusätzlich Name/Descriptor/Bootstrap/Args |
| executable Instruktionen | opcode, eindeutiger Nodekind, sämtliche lokale slots/Integeroperanden/Owner/Name/Descriptor/interfacebit; Switchkeys/ranges/geordnete Labels; Multiarraydescriptor/dims; originale Jumpziele |
| Exceptionedges | originale priorisierte start/end/handlerinstructiontargets und genauer Throwabletyp |
| Field ConstantValue | ebenfalls derselbe typisierte Konstantenvergleich |

Classinput≤16777216 B vor ClassReader, Methodinstructionnodes≤262144, recursive constantdepth≤32 (root0),≤65536 Constantvisits pro Methode,≤65535 bootstrapargs, String-/Type-descriptor≤65535 Java chars; Klassendatei-UTF8-/Constantpool-/Operandgrenzen bleiben zusätzlich JVMformatgrenzen. ConstantDynamic wird gegen aktive Identitätszyklen geschützt. Unsupported Byte/Boolean/Object/array/null oder executable unbekannter Nodekind verweigert explizit; keine Ersatzstringrepräsentation. Metadaten-/Debug-/Frame-/maxstack-Daten sind weiterhin keine computational Equality; echte JVMdefinition/Ausführung ist zusätzlich nötig. Diese Bounds garantieren keine adversariale ClassReader-Ressourcen-/TOCTOU-Abwehr. Nur private unveränderliche, vorab gepinnte Klasseneingänge sind vorgesehen.

Templates bleiben: originale maxlocals bestimmen disjoint frame/tempwide Slots; originale Operanden werden für DASTORE/BitSet/Return genau spilled/reloaded; computational calls/fields/branches/targets bleiben erhalten; Handlerpriorität bleibt original plus letzter catch/rethrow **derselben** Throwableidentität. Keine gelöscht-markierte Sequenz, Agentimport, Klassenreceipt oder eingeschränkte Templateauswahl ersetzt die Prüfung. Erfolgreiche vollständige Observerruns werden mit unbeobachteten Runs verglichen. Logger/Boxing/IO/OOM/Instrumentationfailure können Fehler-/Ressourcenchronologie ändern; keine allgemeine Exception-/OOMparitätsbehauptung.

**Exakter gültiger Bootstrapregressionstest:** N/tools/BootstrapMutation.java ist ein unveränderter ganzer Bytecopy des Reviewers. Die computational PlacedFeature-FlatMaplambda arg0 wird von ASM MethodType auf einen String **desselben** `(Ljava/lang/Object;)Ljava/lang/Object;` Textes gesetzt. Fresh javac und PrivateLoader definieren den tatsächlichen JVMmutanten; CountPlacement.of(0), echte Oreconfiguration/Holder/Bootstrap erzeugen Original `return:false`, Mutant `BootstrapMethodError cause:ClassCastException`. Frisch kompilierter **alter** R2-Proof akzeptiert diese Mutante126/134 Methoden/selected36; neuer Proof verweigert am typisierten Type/String-Operand. Beide Profile normal/−O, ganze11 Original-/Transformklassen, tatsächliche JVMlogs und alte/neue Proofs unter proof-{normal,optimized}-final/.

Der zusätzliche neue ProofCallhelper leert nach dem exakten unveränderten BootstrapMutation-Aufruf LogManager.shutdown; dies betrifft nur die vollständige Logausgabe. Die ersten vollständigen Count0-Läufe hatten Exit0, aber Papers asynchroner Konsolenlogger verlor die letzte Ergebniszeile. Initiale Freezeadmission verweigerte deshalb; alle alten Sources/Logs/Classes bleiben als superseded/failed Belege erhalten. Finale Proofläufe wurden mit dieser Logabschlusskontrolle normal/−O wiederholt.

Exakter alter Marker-/Originalslotangriff liefert weiterhin2763940→−160476802; alter StripVerify akzeptiert, neuer Proof verweigert. Alle acht alten Autorenmutationen und alle acht alternativen Reviewermutationen (original-slot, computational-call, field-write, return-operand, branch-predicate, branch-target, replace-throwable, swallow-throwable) wurden frisch kompiliert, valide definiert und wirklich gegen Original JVM-Ausgänge/Counts/Throwableidentität ausgeführt. Keine inconsequential oder malformed Kontrolle zählt. proof-*-final und reviewer-mutants-{normal,optimized} halten32 consequential Mutanten je Modus plus exakte Marker-/Bootstrapkontrollen; volle echte126 Vanilla/134 Paper method positives, selected36 und elf Klassen bleiben erhalten.

N/tools/TypedConstantChallenges.java prüft19 typed Mehrdeutigkeiten jeweils durch Constantkey **und** beide LDC-/indy-Operandwege,57 Positivkontrollen/63 Distinctions-und-Refusals je Modus. Dazu **sechs tatsächlich valide ausgeführte synthetische JVMklassen**: unterschiedliche Float-/Double-NaNrawbits und nested ConstantDynamic mit tatsächlichem String.class versus DescriptorString. Diese sechs sind ausdrücklich synthetische Konstantencontrols, keine Minecraftalgorithmen/oracle. Unsupportedkind-/Depth-/Nodeboundrefusals werden separat gezählt. typed-normal-final und typed-optimized-final enthalten ganze Source/Class-/JVMartifacts. Keine Minecraftberechnung wurde neu implementiert.

## 6. CR63-B3: vollständige computational Membership vor jeder Payload

**Verbindliche CLI:** `python -B [-O] N/tools/lossless.py CANDIDATE --reference EXTERNES_DECLARATION [--trace ADMITTED_CANDIDATE_MEMBER] [--structure]`. Für den101-Corpus bleiben nur Original preparation/final-normal/{vanilla,paper} Referenzroots zugelassen; unabhängiger Originalfreeze `a919…` und r1 Reviewfreeze `9462…` sind mandatory fest. Jeder deklarierte Filepin muss im gepinnten Originalfreeze liegen. Kandidat als Referenz, selfminted freeze, fehlender Reviewpin, falsche Pins oder unvollständige Referenzmembership verweigern. Referenzdeclarations werden nicht aus Candidates erzeugt. `--structure` ist ausschließlich eine zusätzliche unveränderte Strukturprüfung nach erfolgreicher vollständiger lossless Equality; kein optionaler Census als Authority.

**Eine Representation:** tatsächlicher Candidate und Referenz haben je genau eine `trace.tsv` **oder** `trace.tsv.gz`. Zwei Repräsentationen werden selbst bei identischen Bytes verweigert. Kandidat und Referenz sind echte lstat-Directories; jedes Entry ist regulär. Der computational101-Layout ist vollständig flach. **Jedes** Unterdirectory, auch empty/hidden, und jeder Link/Special wird vor dem Öffnen abgewiesen. Diese flache Policy bindet die vollständige rekursive Membership, weil kein descendant zugelassen wird; keine rglob-Auslassung versteckter Unterdateien. Zusätzliche hidden regular Files werden durch exakte Namensmembership verweigert. Own FIFOs/Symlinks sind Typstimuli, werden ausschließlich mit lstat/readlink inventarisiert, niemals geöffnet/gefolgt.

Reihenfolge: candidate lstat Membership → reference lstat/exakte independiente Freeze-/Namensbindung → beide eindeutigen Trace-/vollen Namensmengen und ggf --trace-Memberadmission → erst dann alle computational Referenzpayloadbytes und Candidatebytes lesen. Authorityfreeze-/Declarationbytes sind Metadaten, keine computational Payload. Kein Referenzsnapshot wird vor vollständiger Candidateadmission gelesen. Nonraw Dateien werden alle SHA/Byte-exakt mit der externen Authority verglichen. Der **wirkliche admitted** Trace wird vollständig dekomprimiert und zeilenbytegenau einschließlich Header, originaler Case/seq/Kind/frame/Symbol/valuefolge und EOF verglichen; keine Sortierung, Filterung, Normalisierung oder Repräsentationsauswahl nach angenehmem Inhalt. Vorher-/nachher volle Names/Types/Modes und jedes computational Filebyte für beide Seiten werden erneut geprüft. Dies setzt immutable private Inputs voraus; keine lockbasierte/adversariale TOCTOUgarantie.

`--trace` bleibt aus Kompatibilität erhalten, darf aber ausschließlich exakt den absoluten lexikalischen Pfad des zugelassenen tatsächlichen Candidatemembers bezeichnen. Externe korrekte Authorplain oder Aliaspath liefert **keinen** Ganzcandidateerfolg. Es gibt keinen bypassenden Einzeltracemodus. Erfolgsreport nennt407 tatsächlich gebundene Files **einschließlich** Trace und `membership_and_all_payload_bytes:true`, `complete_before_after_files_and_bytes:true`, genaue lossless2644242/2644304 Zeilen und raw_member. Der vorherige406-Erfolg zählte den tatsächlich behaupteten Trace nicht als Computationalmember; diese unvollständige Zählung wird ersetzt.

| tatsächlicher alter CLIangriff, beide Profile normal/−O | alte CLI | Revision3 CLI |
| --- | --- | --- |
| korrektes plain plus mutated gzip | behauptet wholecandidate true | refuses exactly-one raw representation |
| hidden/trace.tsv.gz mit echter Spherebitmutation | behauptet wholecandidate true | refuses Directorytyp vor Payload |
| mutiertes tatsächliches plain, --trace korrekt extern | behauptet wholecandidate true | refuses membergebundenes --trace |

Alle6 alten Falseacceptances und6 neuen Refusals **je Modus** stehen als echte Commandarrays/Exit-/stdout-/stderrlogs und vollständige eigene Candidate-/Mutantbytes unter payload-{normal,optimized}-complete. Dazu34 gewöhnliche echte Payloadmutanten je Modus (17 pro Profil): Branch/Sphere0/CenterY/Z/Radius/Cull/Branchrichtung/signedCore/rawf32/f64/WriteState/Mask/Release/Missing/Duplicate/Reorder; jedes actual Candidategzip wird dekomprimiert bis zur geänderten Zeile und als `lossless original payload at line…` verweigert, nicht nur nach Auswahlmetadaten.6 echte Positives je Modus (externes frozen gzip, vollständiges Authorplain, admitted-member --trace),12 weitere Typ-/extra Membershiprefusals,10 authorityforgeries und2 missing-referenceCLIrefusals bleiben erhalten. Alle Fixtures sind vollständig in N; unveränderte Hardlinks werden nie mutiert. Regionalreferenzen werden hier **nicht** neu in die101-CLI aufgenommen: unabhängig gepinnte Reviewerregions dienen nur unveränderter kompletter chronologischer/perfeature Evidenz. Eine Erweiterung müsste deren exakte externe Pins/Provenanz separat zulassen.

## 7. Frische fokussierte JVMcalls und transparenter historischer Reuse

N/tools/focused.py kompiliert Agent/Observe/neuen Proof/helper frisch und guardet ganze Original-JARs/JDK/CodeSources/Helpermembership vor/nach Ausführung. Einzige zusätzliche Harnessänderung ist Skip der58 unaffected old60 Samples; Setup und alle wirklichen ausgewählten Actions bleiben original. Je Profil/Modus/observed-or-control **43 echte Aufrufe**, dirt0/gravel0 und alle41 Semantikpflichten. Alte originale Javaalgorithmen/Callees und Transformclassbytes sind unverändert. Whole tools-before Membershipbytes werden am Runende geprüft; später ergänzte Quellen werden erst nach Abschluss hinzugefügt und sind separat gepinnt.

N/check_focused.py normal/−O vergleicht alle174 Nonrawcomputationalfiles je Profil, einschließlich172 vollständiger before/after State-/MetadataSnapshots, volle Ergebnis-/finalRNG-/Publicdraw-/Guardfolge gegen unveränderten externen Original101-Corpus, nicht bloß ausgewählte Snapshotcells. Jede ausgewählte vollständige Raweventfolge matcht die ursprüngliche Originalfolge bytegenau.43 Rawcasehashes/Eventcounts je Profil stehen im Receipt. Ganze normal/−O Class-/Payloadmembership und Bytes gleich:406 files observed/360 control. Original ZIPentries aller elf tatsächlich geladenen Klassen, gleiche Transformation und volle126/134 Proofs werden frisch geprüft. Keine focused Ausführung wird als frischer101-Komplettlauf ausgegeben.

Unchanged komplette101-Fälle/404Snapshots,41 Semantikchecks,12 cumulative Regionen/318 Perfeature-RNGendstates, beobachtete/unbeobachtete Controls, Wrapper/Overflow/Guards und prior full normal/−O-Class-/Payloadgleichheit bleiben vollständige immutable Reviewer-/Authorauthorities. Deren erfolgreiche JVMworkload wird nicht nur für Grün wiederholt. Die geänderte Payloadgarantie liest den vollständigen101-stream tatsächlich neu; die geänderte Proofgarantie prüft ganze Klassen und gültige consequential Gegenbeispiele neu. Modelle streamen alle12 Regionen vollständig neu. Rootintake/whole outside verification bindet sämtliche geerbten Bytes, nicht bloß die Receipts.

Chronologie bleibt: Featureguard vor angle/Ydraws, Gate X→Z bis erster Erfolg; f32 ratio vor f64 lerp, alle Sphären vor striktem Cull; Sphere→X→Y→Z, buildheight vor maskget, mark vor Guard; target mismatch ohne Draw, Chance0/1 ohne Draw, sonst float≥chance; DOWN/UP/NORTH/SOUTH/WEST/EAST shortcircuit, Wasser kein Air; equal-state Write/true ohne Nettodelta; acquire/release/Throwchronologie. Direkte Sectionwrites erzeugen keine Region-/ProtoChunk-Map-/POI-/Entity-/PPupdates. Vanilla nonempty Fluidcounter/Paper randomly-ticking Counter/Callback bleiben getrennt. Outer Decorationseed genau einmal, komplette164 Stepindices/equalitydiscovery/Identityindex, Java-int-count vor Core inklusive Throw/Overflow, featureseed default−1 und kumulative Originalendstates bleiben Originalpflichten. Nicht-Ore-/Structures-/featureseedoverride-Consumer sind weiterhin fehlende Stages.

## 8. Ganze neue Lieferung, Reproduktion und genaue Restentscheidungen

N/source-provenance.json enthält ganze Herkunfts-/neue Sourcehashes und **exakte** unified Deltas für jeden kopierten/angepassten Tool/helper; neue Quellen sind explizit so deklariert. Die frisch kopierten proof helper haben zusätzlich source-provenance.json im jeweiligen Run. Ganze generated Helperdeltas/Commands, Sources, Classes/JARs, positive/negative JVMlogs/Proofs, vollständige actual Candidate-/Mutationbytes, Declarations, mathematische Work-/Native-/Adapterreceipts, komplette Inputs und Ergebnisse werden gefroren. N/attempts.json hält failed ambiguous-Type-javac und null synthetic handler Versuch samt Quellen/Logs, erfolgreichen aber superseded ersten Proof-/Payload-/Modellstand. Kein Fehlversuch zählt als Positivkontrolle; keine Entwicklungsordner werden ausgelassen.

Reproduktion mit **frischem** LABEL ausschließlich innerhalb N: tools/proof_challenges.py, tools/reviewer_mutants.py, tools/typed_constants.py, tools/payload_challenges.py, tools/models.py, tools/adapter_blueprint.py, tools/focused.py LABEL observed|control, check_focused.py und tools/integrity.py; jeweils Python -B normal/−O. Class-/Javaflags und echte JAR/JDK/CodeSourceguards stehen vollständig in den Quellen/Commandreceipts. Freeze und verify prüfen reguläre vollständige Bytes und exakte lstat Membership/Modes/Linktargets auch für eigene negative Entrytypen, ohne Specials/Links zu öffnen.

N/freeze-manifest.json bindet **alle** N-Dateien/-Entrytypen und diesen externen Bericht nach erfolgreichen vollen checks/immutable outside verification. N/hashes.json hält exakte Quellen-/Input-/Berichts-/Manifestpins, File-/Byte-/Entrycounts. Ausschließlich vier äußere Selfhashreceipts freeze-manifest.json, hashes.json, verify-normal.json, verify-optimized.json sind zur Vermeidung zirkulärer Pins ausgeschlossen. Beide finalen Verifier prüfen die komplette Lieferung plus ganze immutable Outsideauthority erneut. Livevorher-/nachherdifferenzen stehen ausdrücklich im outside-byte-comparison.json; konkurrierende Root-/Registryänderungen werden weder mutiert noch als akzeptierte Authority übernommen. Historische freezes/receipts bleiben unverändert.

**Exakte Restentscheidungen:** Root vergibt den neuen unabhängigen Review über diese komplette Korrekturkette. Getter53/Selection32/85 und private Single/Compositeborrow-/Pending-/Receipt-/BetweenPlansseed-Amendments sind konkrete reviewed-blueprint-Vorschläge, keine implementierten Producerschnittstellen. Vor dem kleinsten späteren World-only Amendment müssen wirkliche Registry/feste Classification, Heightmapkernel/immutable WGhistory, Version/Owner/Phase/Geometryproducer und primitive actual Cursor/Core/Parentmeterbasis angenommen sein. GEN-03.1a grants ausschließlich Registry; HeightPreparation2^23, Worldreservations, Composite2^22/16Mi, PP131072/Entity-/Tickindex2Mi, Cleanup/Native-/Geometry-/Validationziele bleiben unangenommen. GEN-06.2 Single1Mi bleibt unverändert.

Danach tatsächliche kontrollierte native Getter-/Oretests:53/32/85 N−1/N/N+1 im gleichen Single/Compositeparent, alle Guard-/history-/late-errors ohne Publication/RNGmutation, vollständige useful Full30-/Biomcompletion und318 perfeature RNGendstates; tatsächliche Debug/Release capacity/layout/stack/alignment,4096-B positive allocatorcontrol, fallible reservefault, terminal cleanup/unwind/last-refcountdrop, compileborrow/move/drop/forget, letzte Completion-/Publicationrefusal und einziger atomarer Commit. Hier **null Rust-/Cargo-/Runtimechecks**. Mathematische Scalar-/Native-/Workmodelle schaffen keine eigenen Producenten.

Neun versionierte CARVERSowner, stored Featurebiomes/Zoom/full Generationmembership, echte Neighbor-/Stage-/Sidecar-/Cleanupauthority und vorausgehende Structures/interleavte Nicht-Ore fehlen weiterhin. Whole FEATURES benötigt tatsächlichen vollständig initialisierten OriginalServerLevel/configs/packs/options/seeds/blending/featureseedpolicy, echte radius8 STRUCTURE_STARTS/radius1 CARVERS, ganze States/Maps/Counters/Entities/PP/Ticks/Corehistories und separate Forward/Reversecenterhistory. Keine erfolgreiche leere Stage oder gesetzter Status ersetzt das. Spätere Simadoption reserviert alle neun Owner, revalidiert ganzen Versionsvektor und installiert einmal atomar an Tickgrenze; Resultdrop hat keine Weltwirkung. **Natural-world activation, Survival, Lighting, Java plugins, ganze FEATURES und Performance bleiben außerhalb dieser Lieferung.**
