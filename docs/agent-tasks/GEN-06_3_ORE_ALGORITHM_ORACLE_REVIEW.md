# GEN-06.3 — unabhängige Review r1

12. Oktober 2026. **Verdikt: FINDINGS.** CR63-B1 und CR63-B2 sind blockierend; CR63-B3 ist eine hohe Oracle-Validierungslücke und blockiert CLEAN_ORACLE. Der Kandidat kann in diesem Stand weder als geschlossener feasible Regionvertrag noch mit der behaupteten vollständigen Inverse-/Bitvalidierung angenommen werden. Die positiv reproduzierte kontrollierte Originalchronologie bleibt als eigenständiger Befund erhalten. Keine Runtime-, Nativecap-, natürliche FEATURES-, World-/Survival-/App- oder Performanceabnahme.

## 1. Auftrag, unveränderte Authority und Prüfumfang

Bindend ist der vollständige neue Root-Auftrag `GEN-06_3_ORE_ALGORITHM_ORACLE_REVIEW_TASK.md`, SHA256 `c765f6b7e0d33bf73b4e997f632256fafd2ccb06b37d15e4f0a0f261c30d4828`, einschließlich Originalauftrag, GEN-06.1-Korrekturkette und konkreter Root-Objection zum verbleibenden 1Mi-Meter. Dieser Auftrag wurde nicht geändert. Zusätzlich wurden der ganze originale GEN-06.3-Auftrag, alle 185 Kandidatzeilen, alle neuen Author-Helper/Transformer/Logger/Disassembler/Inverse-/Validator-/Control-/Mutation-/Guard-/Audittools und die bindenden Vorverträge gelesen.

| geprüftes unverändertes Objekt | SHA256 / Umfang |
| --- | --- |
| `GEN-06_3_ORE_ALGORITHM_ORACLE.md` | `49746a75ef24eb952ea73198741a6b80e262de8bec9da325cdf7e6dfa3ee758d`, 185 Zeilen |
| Author `gen06_3-preparation/freeze-manifest.json` | `a919bc57099d1791f1021cf03aca9716fb90be5f4cb700c4fb2f824370252593`, 3878 Dateien / 599591972 B |
| immutable `gen06_3-root-base/base.freeze.json` | `5d3d971efacee11e6f89d6f695e38066474b8401fe5609563d3f4d68fd919b69`, 1450 Dateien / 324 Worlddateien |
| angenommenes GEN-06.1 Revision2 | `24ab2da586620009f25bfbfd99e433215d7e8698ec76a62eaff03ec03ec74eb8` |
| Revision2 Authorfreeze | `e44f7199a3f222007e9759a6fff69cf0ab030008e6adcbe003b8d03da81315a7`, 366 Dateien |
| angenommener Revision2 Reviewfreeze | `dd95a0be1baa4e176857a518978543d86427a2461e8cf796e00a365b1f6e63a4`, 494 Dateien |
| Vanilla 1.21.8 Inner-JAR | `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250` |
| Paper 1.21.8 Build60 Inner-JAR | `c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507` |

Authority ist das readonly Archiv zu Main `18d510e1023f2422b3dc65e9acbf5189523824af`, akzeptierter Worldruntime `a912b1c2aa1c0b3de7510156c1b195a506e2e65b`. Aktuelles Main und concurrent World-WIP ersetzen diese Authority nicht. Keine Gitaktion, Agentendelegation, Dependency-/Author-/Produktionsänderung oder gemeinsamer Build wurde durchgeführt. Eigene Writes liegen ausschließlich in diesem neuen Bericht und `.codex-tmp/gen06_3-independent-review-r1/**`. Die anderen World-/Registryheightmap-Arbeiten wurden nicht übernommen.

AGENTS/CLAUDE/PAPER_REFERENCE, World-/Registryinvarianten, Architektur/Simulation/ADR0002, angenommene RNG-/Density-/Material-/Biome-/Zoom-/Stored-Biome-/Temperature-/Surface-/Columndriververträge sowie die vollständigen ursprünglichen und korrigierten GEN-06.1-Aufträge/Verträge/Reviews und der queued GEN-06.2-Auftrag sind Kontext. Historisches old60 ohne S-Ereignisse bleibt unverändert. **GEN-06.1 H1 war eine Korrektur der Reviewbeschreibung:** der Originalhelper enthält bereits `abs(block >> 4)`; kein historischer Codefix wurde verlangt oder ausgeführt. H2 bleibt eine neu zu entscheidende gleichzeitige Nativecap, deren Eingang plus Kandidat allein 7077888 B Statepayload benötigt.

Tatsächliche vollständige Base-/Appliedquellen und relevante ganze Source-/Featurepatchdateien wurden gegen Herkunft überprüft: OreFeature, BulkSectionAccess, Section/PalettedContainer/ShortList, RNGwrapper/Core, Placement/ConfiguredFeature/Feature/context, Biomefilter/GenerationSettings/FeatureSorter, Tag-/Targetclosure, Height-/Countprovider/Anker, Heightmap/LevelHeightAccessor, WorldGenRegion/ProtoChunk/ChunkAccess sowie Decoration/ChunkStatusTasks/ChunkPyramid. Anti-Xray, Bit-Inlining und die relevanten Moonrise-Section-/Paletteänderungen wurden einbezogen; das behauptet keine Prüfung sämtlicher fremder Moonrisefunktionen. Tatsächliche Originaldisassemblies und alle elf Transformklassensätze je Profil wurden geprüft; Quellen allein ersetzen nicht die geladenen Bytes.

## 2. CR63-B1 — BLOCKING: useful Full30-Completion passt nicht zur unveränderten Session-/Meterauthority

**Ort:** Kandidat Zeilen 93, 99, 103–105, 127–133, 175–179; `tools/work.py`, Author `census-*.json/declared_work_model`; queued GEN-06.2 sealed Single-key-Session/RunAllowance/Pendinglease. Root hat weder neue Debits noch Reserve oder Capänderung angenommen.

Der Kandidat verlangt denselben kumulativen Meter für Copy, Validation, Primitive, Kern und Publication, maximal 1048576; Reset/Upgrade/Cloning sind ausgeschlossen. Gleichzeitig kostet sein eigenes gepaartes Read+Store-Modell:

| obligatorischer Copyteil | Einheiten |
| --- | ---: |
| neun Statebuffer, inklusive Endheads | 884745 |
| sechs Maps pro Chunk | 13878 |
| Fluidwörter | 13833 |
| Counterrecords | 225 |
| leere PP-Abschnittheads | 216 |
| Copy gesamt | **912897** |
| vorgeschlagene Validationreserve | **2048** |
| vor Primitive/Kern mindestens ausgegeben | **914945** |
| verbleibend bis 1Mi | **133631** |

Die Censusereignisse wurden unabhängig gestreamt und aus tatsächlichen branchordinals/Methodenframes neu gezählt, nicht aus Author-Summen übernommen. Zusätzlich zum partiellen Ereignismodell wurden alle ausdrücklich separat geforderten Initialisierungsstores/-heads für Sphere/Mask/Handles, Spherebodygroups, native Settergroups, uninstrumentierte Luftloopheads/-predicates, Releaseheads, Primitive-Emission/Dispatch/Completion, Begin/Seedadministration/End und einmalige Regionpublication zugeordnet. Echte Raster-/Cull-/Targetheads und Entscheidungen, Section-/Bulkreads/-writes/acquire/release und genuine `next(bits)`-Coreentries sind im partiellen Teil enthalten; kein Snapshot wird als Writefolge gezählt.

Das ist ein **vollständiger konservativer Layer-Lower-bound für die enumerierten Vertragsoperationen**, keine exakte implementierte native Ledger-/Allocator-/Ownershipmessung. Noch nicht festgelegte native Admission-/Owner-/Reserve-/Cleanupkosten können das Ergebnis erhöhen. Es wird weder die Allowance angehoben noch Copy weggelassen. Die neuen Debitdefinitionen bleiben Rootentscheidungen.

### Historischer kontrollierter old60-Corpus, keine kumulative Generation

Alle 30 alten Cases pro Seed setzen STONE jeweils neu und verwenden ausdrücklich Index0/Step6. Für eine rechnerische Summe dieser 30 isolierten Kernarbeiten mit nur einem Copy/Validation und einer Publication ergeben sich in beiden Originalprofilen:

| Seed | partielles Ereignismodell | Summe mit zusätzlichen Gruppen, einem Copy/Validation/Publication | Überschreitung 1Mi |
| --- | ---: | ---: | ---: |
| 0 | 962459 | **1991517** | **942941** |
| −1 | 935596 | **1964816** | **916240** |

Diese Summe ist ausdrücklich kein Trace einer gemeinsamen Region und keine natürliche Decoration. Bereits **ein einzelner** vollständiger old60-Clayplan benötigt Seed0 `292419+25256+914945 = 1232620`, Seed−1 `286472+25271+914945 = 1226688`; jeweils 46 Attempts/1518 Sphären. Damit scheitert sogar diese erklärte vollständige Einzelplanarbeit nach dem notwendigen Regioncopy, lange vor einer nützlichen 30-Planabnahme. Schon der Author-Partialcensus liegt weit über dem verbleibenden 133631-Rest.

### Frischer echter kumulativer Original-Kontrolllauf mit allen 30 Plänen

Zur direkten Auflösung der Root-Objection wurden zusätzlich neue kontrollierte JVM-Läufe vorab deklariert und frisch unabhängig kompiliert: initial neun echte Defaultstatus-ProtoChunks mit STONE, Gate320, Seeds0/−1; alle 30 tatsächlichen Original-PlacedFeatureobjekte in der echten globalen Step-/Indexreihenfolge. Vorherige Writes bleiben über alle Pläne erhalten. Jeder Plan erhält seinen erklärten passenden Originalbiomeholder-Proxy und tatsächlich originale WorldgenRandomwrapper/Core mit originalem Featureseedcell. Alle 30 Callbacks wurden gegen den unveränderten vollständigen **164-Indizes-Originalexport** geprüft, inklusive Lücken und Stepwechsel von infested.

Dies ist eine **deklarierte kumulative kontrollierte 30-Planunion**, kein natürlicher FEATURES-Lauf: nicht alle diese Pläne gehören zu einem einzigen tatsächlichen Biome; stored Snapshotbiome bleibt plains; Strukturen, andere Decorations und non-ore Interleaving fehlen. Per-feature Originalwrapper-Neuseeding ist Teil des Kontrollinputs und erteilt keiner angenommenen FerrumC-Session eine Reseed-/Resetauthority. Aufgenommen ist nur der RNG-Endzustand des letzten Features, nicht jedes Feature-Endstate; alle einzelnen Coreentries sind dagegen beobachtet.

Normal und Python −O, jeweils beobachtet/unbeobachtet und Vanilla/Paper, schließen je Profil 60 originale Placements (2 Regionen ×30) ab. Pro Profil/Pythonmodus sind **10 Computationaldateien** einschließlich acht vollständiger State-/Metadatasnapshots bytegleich; normale/−O Computational-/Class-/Trace-Membership und Bytes stimmen ebenfalls überein. Rückgaben sind Seed0 `calls=30;success=27`, Seed−1 `calls=30;success=25`. PP ist im gesamten frischen Kontrolllauf null.

| Seed, beide Profile | doPlace-Attempts | partielles Modell | zusätzliche notwendige Gruppen | Copy/Validation + ganzer Kern, Mindestbedarf | Überschreitung 1Mi |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0 | 433 | 974516 | 113878 | **2003339** | **954763** |
| −1 | 434 | 927566 | 112782 | **1955293** | **906717** |

Die Vanilla-Traces enthalten 1260424/1203007 Ereignisse für Seed0/−1; Paper 1260454/1203037. Papers zusätzliche originale `getLevel`-Callbacks sind keine Kosten-/Resultnormalisierung. Der reused frozen Wrapperrunner schreibt in `execution.json` geerbte Drei-Wrapper-Texte. **Diese Texte sind für Full30 falsch beschriftet.** Die vorab geschriebenen `full30-*-declaration.json`, tatsächlichen Results, 30 Plan-Callbacks und `full30-model-*.json` berichtigen den Umfang ausdrücklich; daraus wurde keine neue per-feature Endstatebehauptung abgeleitet.

**Auswirkung:** Checked exhaustion kann die Refusalpolitik testen, erfüllt aber keine useful vollständige Regiongeneration. Zusätzlich gibt GEN-06.2 nur die sealed Single-key-Session/Pendingleaseauthority; der Kandidat definiert keine zugelassene Zusammensetzung von 30 Keyläufen mit gemeinsamem unverändertem Meter und einmaliger Regioncopy/publication. Einzelne neue Sessions mit frischem Meter oder unmetered vorgeschaltete Copy würden diesen Vertrag nicht erfüllen.

**Erforderliche Korrektur:** Vor GEN-06.4 einen gesonderten konkreten Composition-/Allowancevertrag vorlegen, der Begin/Planordnung, originär zulässiges interplan Featureseeding, sealed Leases, gemeinsamen kumulativen Meter, Copy und End/publication geschlossen beschreibt. Die bestehende GEN-06.2-Primitiveauthority bleibt bis einer expliziten neuen Rootentscheidung unverändert. Für die leere-PP-Kontrollunion liegt die belegte useful Untergrenze bei **mindestens 2003339**, zuzüglich noch zuzuordnender tatsächlicher native Owner-/Reserve-/Cleanupkosten. `2^21=2097152` ließe bei Seed0 nur **93813** übrig; es ist **nicht als ausreichend bewiesen oder genehmigt**. Als reine Gegenrechnung würde die ebenfalls unangenommene PP-Admissionalternative mit 131072 Einträgen bei gleicher übriger Arbeit mindestens **2134411** verlangen, bereits 37259 über `2^21`; dies wurde nicht als Originalinput ausgeführt. Eine alternative Copy-/Compositionslösung braucht dieselbe numerische vollständige Nachweispflicht und ausdrückliche Rootentscheidung. Kein stiller neuer Cap, Partoutput oder Scopewechsel.

**Nachweise:** `.codex-tmp/gen06_3-independent-review-r1/work-final2-{normal,optimized}.json`, `full30-model-{normal,optimized}.json`, vier vorab geschriebene `full30-*-declaration.json`, `full30-final-{observed,control}/**`, `full30-optimized-{observed,control}/**`; eigene `tools/work.py`, `Full30.java.txt`, `full30.py`, `full30_checks.py`.

## 3. CR63-B2 — BLOCKING: inverse Rückprojektion akzeptiert eine tatsächliche Rechenmutation

**Ort:** `.codex-tmp/gen06_3-preparation/tools/StripVerify.java:21`, `strip`, Zeile22 Blocklöschung und Zeile54 Vergleich; Kandidat Zeile41 behauptet Preservation sämtlicher Originalrecheninstruktionen/CFG.

Die markierten Blöcke erlauben beliebige `ISTORE/LSTORE/FSTORE/DSTORE/ASTORE`, ohne Originalargumente/-locals von neu eingeführten Temp-Slots per Datenfluss zu trennen. Der Prüfer entfernt anschließend den ganzen Block. Gleichheit des Restprogramms beweist deshalb nicht die Unschädlichkeit der entfernten Stores.

Frische unabhängige **Classfilemutation** in beiden tatsächlichen transformierten WorldgenRandomklassen: innerhalb `GEN063_BEGIN:entry` in `WorldgenRandom.next(I)I` (`ekz.c(I)I` Vanilla) wurden `ICONST_0; ISTORE 1` eingefügt. Slot1 ist das ursprüngliche computational bits-Argument. Tatsächliche Original- und Mutantmethoden wurden separat ausgeführt:

| Kontrolle | tatsächliches next(24)-Resultat |
| --- | ---: |
| Original | **2763940** |
| Mutant mit überschriebenem bits-Argument | **−160476802** |

Trotz dieser wirksamen Rechenänderung meldet der unveränderte Authorprüfer **`STRIP_VERIFIED 126`** Vanilla und **`STRIP_VERIFIED 134`** Paper. Genuine positive Transformkontrollen bestehen ebenfalls. Normal/−O reproduzieren dieselbe Annahme und tatsächliche Resultdifferenz.

Dies ist kein Befund einer solchen Mutation im tatsächlich gelieferten Authortransformer. Positive Originalkontrollen in §5 bleiben gültig. Es widerlegt jedoch die behauptete unabhängige mechanische Preservationgarantie: ein angreifbarer Prüfer plus Marker-/Opcodeallowlist ist keine vollständige Noninterferenceprüfung.

**Erforderliche Korrektur:** Exakte observertemplates oder ein gleichwertiger lokaler Datenfluss-/Stack-/CFGbeweis müssen ursprüngliche Argumente/Locals vor Veränderung schützen, neue Temps eindeutig nachweisen, konkrete Observe-/Boxingsignaturen und Operanden sowie Branchprädikate/-ziele, ursprüngliche Exceptionedges und Throwableidentity prüfen. Diese tatsächliche computational Classmutation muss bei weiterhin gültigen Positivkontrollen abgewiesen werden, normal und −O. Kein bloßes Entfernen weiterer Opcodes ohne Prüfung legitimer Storetemplates.

**Nachweise:** `inverse-{normal,optimized}/results.json`, tatsächliche mutierte Classfiles, Original-/Mutant-Ausführungslogs, positive Inverseproofs und vom Mutanten akzeptierte Proofs; `tools/InverseMutation.java`, `tools/inverse.py`. Die negative Mutation bleibt ausschließlich im privaten Reviewbereich.

## 4. CR63-B3 — HIGH, CLEAN_ORACLE-blockierend: relevante Tracebits werden nicht vollständig validiert

**Ort:** `.codex-tmp/gen06_3-preparation/tools/validate.py:82–87`, `validate`; optionaler Censusvergleich Zeile108 und CLI ohne `expected` Zeile120. BRANCH prüft Framing, Format und Ordinalmembership. Initiale SPHERE_STOREs prüfen Format und Indexfolge, nicht den echten ursprünglichen f64-Wert.

Zwei unabhängig deklarierte **Einzelzeilenmutationen eines echten Vanilla-TSV** wurden über den vollständigen 101-Fallvalidator gestreamt; alle anderen Fixturebytes und Eingänge bleiben unverändert:

| tatsächlicher Fall/Seq/Frame/Symbol | Original → Mutation | normal/−O |
| --- | --- | --- |
| `old60-ore_andesite_lower-0`, seq43, frame21, `BitRandomSource.nextInt(int)` | Branch `3:true` → `3:false` | **ACCEPTED** |
| derselbe Fall, seq134, frame46, `OreFeature.doPlace`, Sphereindex0 | `f64:402efabc05b261b6` → `f64:0000000000000000` | **ACCEPTED** |

Positive 101-Fallvalidation beider Profile und alle 41 Semantikfälle pro Profil bestehen normal/−O; die vorhandenen 13 Author-Negativmutationen werden abgewiesen. Diese Positivergebnisse schließen die beiden neu belegten Lücken nicht.

Zusätzlich wurde der echte positive erwartete Census ausdrücklich an `validate(..., expected=positive)` gegeben, bei jeweils exakt einem gematchten ursprünglichen Datensatz. Dann wird die Branchmutation als `fixture completeness census` verweigert; **die f64-Sphärenmutation wird weiterhin ACCEPTED**. Ein verpflichtender vorhandener Count-/Branchcensus allein schließt die Bitlücke somit nicht. Die neuen Bitmutationen wurden konkret am Vanilla-Stream durchgeführt; daraus wird kein zusätzlich durchgeführter Paper-Mutationstest behauptet.

**Erforderliche Korrektur:** Relevante computational Payloads müssen durch unabhängige originale Beziehungen oder verpflichtenden verlustfreien Vergleich gegen die gepinnte originale Referenz bewiesen werden. Dabei weder eine nachgeschriebene Oracle als Original ausgeben noch nur die momentan akzeptierte Datei gegen ihren eigenen frisch erzeugten Hash prüfen. Branch-Taken- und initiale Spherecenterbits als konkrete Negativkontrollen ergänzen und positive genuine Originalausführung normal/−O erhalten. Der Freeze erkennt Veränderungen seiner festen Bytes; er beweist keine semantische Vollständigkeit des Validators oder einer künftig neu gefrorenen Aufzeichnung.

**Nachweise:** `fixtures-{normal,optimized}/additional-mutations.json`, zwei jeweilige `.tsv`-Vorher/Nachherbelege, positive Validation-/Semantik-/Author-Mutationreceipts, `expected-mutations-{normal,optimized}.json`; `tools/fixtures.py`, `tools/expected_checks.py`. Die frozen Originalfixture wurde nicht verändert.

## 5. Positive unabhängige Originalkontrollen und getrennte Source-/Chronologieverdicts

**Sourceverdict:** Kein zusätzlicher konkreter Fehler in der beschriebenen Ore-/Bulk-/Section-/Wrapper-Originalchronologie gefunden. Die neuen Methoden sind wirklich originale JARmethoden mit deklarierter Instrumentierung und kontrollierten Eingängen. Die Originalsource-/bytecode-, CodeSource- und Membershippins sowie positive beobachtet/unbeobachtet-Vergleiche tragen diesen engeren Befund. B2 verhindert die weitergehende mechanische Preservationabnahme.

**Kontrollierte Chronologie:** Frisch unabhängig kompiliert und ausgeführt wurden je Profil/Pythonmodus **43 Originalfälle**: old60 dirt/gravel Seed0 und alle 41 zusätzlichen consequential Edges, jeweils beobachtet und ohne javaagent/aktive Loggeraufnahme. Es stimmen **174 Computationaldateien** inklusive **172 vollständiger State-/Metadatasnapshots** überein. Alle 43 einzelnen Rawtraces gleichen exakt den entsprechenden frozen Authorfällen. Vanilla **359509**, Paper **359513** Ereignisse. Normale/−O Membership und Computational-/Class-/Fixturebytes: focused observed402, control356 Dateien. Das ist eine ausdrücklich fokussierte unabhängige Reproduktion, keine Behauptung nochmals vollständig ausgeführter old60/101 JVMläufe. Der gesamte eingefrorene 101-Corpus wurde dagegen vollständig gehasht, validiert, semantisch geprüft und gezählt.

Die echten Edges bestätigen Wrapperdelegation/rejected bounded-int/Coreexception/PaperDouble1; target mismatch ohne Draw/Neighbor, Targetgleichheit mit Write/true trotz Nullnettdelta und zweites DEEPSLATE-Target; sechs originale Luftdirektionen mit Shortcircuit und Wasserpfad sowohl standalone als auch Bulk/Section; chance0/1 ohne Draw und chance0.5-Gleichheit; Gategrenzen; size-Doubles vor Zugriff/Write, Cull-/Tieverhalten, getrennte Masken und Mark vor guardfalse; Bauhöhen und positive/negative Chunk-/Shardnähte; Consumerthrow mit wirklichem release/close; Biomefilterrefusal und missing-topFeature-Exception. Releasefolge ist originale Bulkmap-Iteratorfolge, nicht Snapshotordnung.

Drei zusätzliche echte Wrapper-/Scriptkontrollen wurden separat frisch wiederholt: **14 gleiche Computationaldateien** pro Profil/Pythonmodus. Endzustände `count=2;calls=2;remaining=[]`, `count=2;calls=2;remaining=constant:-1`, `count=1;calls=1;remaining=always-throws`; keine erfundene Xoroshirostate für Scriptcores. Normale/−O Wrappermembership/Class-/Fixturebytes observed82/control36 stimmen überein. Full30 aus B1 ist ein weiterer eigener frischer Kontrolllauf, kein Ersatz dieser Edgekontrollen.

Alle neun before/after Chunks werden vollständig als States und verfügbare Metadata inventarisiert: 24 Sections ×4096 States, Sectioncounter, Paperflags und gespeicherte ticking-location-Reihenfolge; vorhandene Maps, Fluidwörter, Entity-/pendingEntity-/PP- und mutable Wrapper-/Scriptdaten. Defaultstatus-/leere Map-/Entity-/PP-Felder sind kontrollierte Inputeigenschaften, keine Producererzeugung. Die null-PP-Full30-Snapshots und die deklarierten Extras mit doppeltem PP-Append wurden auseinandergehalten. Ein Nettsnapshot beweist nicht die tatsächliche Writefolge; dafür wurden echte Sectionwrite-Events geprüft.

Direkte OreFeaturewrites benutzen den echten Sectionsetter, keinen WorldGenRegion-/ProtoChunksetter. Damit werden keine Heightmap-, POI-, Entity- oder PP-Nebenwirkungen erfunden. Vanilla nonempty-fluid und Paper random-ticking-fluid Counterregeln sowie Papercallback/Specialcollision-/ticking-membership bleiben profilabhängig. Identischer State überspringt Papers callback, nicht den davor liegenden Counterpfad. Die allgemeine admissiongeprüfte Klassifikation vorhandener Eingangstates bleibt erforderlich.

**Provenanz-/Guardkontrollen:** Je normal/−O **14 Refusals und drei Positivkontrollen**: fünf JVMenv-Einstiegspunkte, echte private Bytemutation und ZIP-Packageshadow, JDK-/Quellmembership, Fresh-/Scopesperren sowie drei tatsächliche JVM-CodeSourceverweigerungen (Helper, Library, Original). Guardnegative benutzen den accepted historischen Helper als klar bezeichnete Kontrolle; die unabhängigen focused Positivläufe kompilieren den neuen abgeleiteten Helper frisch. Keine Refusal wird aus einem bloßen Mockreturn abgeleitet.

**Nachweise:** `comparison-final.json`, `normal-{observed,control}/**`, `optimized-{observed,control}/**`, vier `wrapper-*/**`, `fixtures-*/**`, `guards-*/**`, private Compile-/Execution-/Transform-/Inverse-/CodeSourcebelege und alle reviewer Tools. Instrumentierungsallocations sind keine native Memory- oder Allocationparität.

## 6. Native Vorschläge, Geometry und noch offene Authority

Die Nativearithmetik wurde unabhängig nachgerechnet. Der **unangenommene** 16MiB-Vorschlag ergibt **12968883 B** Subtotal und **3808333 B** Reserve. Dabei sind beide Statepayloads **7077888**, beide Mapsets55296, Fluidsets221184, Counter/Flags6912, immutable ticking-values1769472, beide PPsets1048576, Borrowbiomes18936, Zoomresolver512, membership4096, classification223568, Sourcebytes26987, Katalog16384, Session512, geometry8192, native Entitycapacities2097152, übrige Header131072 und Stack/Temps/Alignment262144 B berücksichtigt. Diese Rechnung ist eine Designprüfung, keine tatsächliche Vec-/Hashmap-/Stack-/Allocatorinventur.

Der ShortList-Originaltyp hat neben der geordneten Valuecapacity auch Hashmap-/byIndexzustand. Er darf in einer generischen mutable Umsetzung nicht ausgelassen werden. Das vorgeschlagene einmalige immutable Borrow der gespeicherten Reihenfolge ist nur mit geschlossener echter Klassifikation zulässig, die unveränderte random-ticking-/Specialcollisionmitgliedschaft für sämtliche admitted Ore-/Tagstates beweist. Sidecars brauchen tatsächlichen Ownermove oder bereits admitted Refcount-Owneradoption; ein Resultborrow aus dem später freigegebenen alten Chunk ist unzulässig. Der Resultowner muss dieselben neun Candidatebuffer übernehmen; eine dritte volle Payload ist nicht eingerechnet oder genehmigt.

Geometry: maximaler Radius4.5 einschließlich PaperDouble1; mask26×14×26=9464 Zellen,148 Wörter1184 B; size64 Spherearray2048 B;2016 Paare; konservativ11 Zellen/Achse, X/Y/Z704/7744/85184 Bodies und768/8448/92928 Heads. Mit27 festen32-B-Handles864 B und4096 B Rest ergibt sich8192 B. Kein zusätzlich externes8192-B-Scratch plus ungezählte Locals. Der vollständige Produktionsorigindomain-/vertikale Slotnachweis für27 Handles bleibt vor Runtime verbindlich; arbitrary direkte doPlace-Testorigins ersetzen ihn nicht. Javaoverflow, f32-Operationen und originale float/Branchreihenfolge dürfen innerhalb der admitted Domain nicht umgeschrieben werden.

Die proposed Validationreserve2048 enthält rechnerisch den fixed Baum1163 sowie die erklärte extension bound1322. Konstante Headeradmission muss vor Inhaltsloops erfolgen; fehlende Klassifikations-/StoredBiom-/Membershipauthority darf nicht durch kostenlosen Vollscan, Hashing oder Parsing ersetzt werden. Jeder wirkliche before-work-/Growth-/Reserve-/Cleanup-/Setter-/Completion-/Publicationlayer braucht später eine native Zuordnung. Cleanupreserve muss vor variablem Release/Drop gesichert sein; Latepublicationrefusal darf keine Präfixadoption hinterlassen. N−1/N/N+1 und Reservefailure/Unwind sind künftig echte Runtimekontrollen, keine hier bestandenen Tests.

**Rootentscheidungen bleiben offen:** 16MiB Gesamtcap, aggregate PP131072 und native Entityinventur/admission,8192/27 Geometryziel,2048 Validationreserve, vollständige neue Kerneldebits sowie die B1-Composition-/Workkorrektur. Keine dieser Zahlen wird durch diesen Review angenommen; bestehende GEN-04/05/06.2-Caps bleiben unverändert.

**Fehlende reale Producer:** angenommene CARVERS für alle neun versionierten Chunkowner; OCEAN_FLOOR_WG plus vier finale Heightmapproducer mit echter air/motion/leaves/fluid-Klassifikation; vollständige stored featurebiomes/Zoom-/Generationmembership; Stage-/Neighbor-/voller Versionsvektor; prior structures/decorations und non-ore Interleaving. Surfacebuffer, Defaultstatus-ProtoChunk, Gate320, STONE und passend ausgewählte Holder erzeugen keine Public Regionauthority. Concurrent Carver-WIP bleibt ausgeschlossen.

**Whole-FEATURES und Adoption:** tatsächlicher initialisierter ServerLevel und configs/packs/worldoptions/seeds/featureseedpolicy/blending, originale Vorstages/Strukturen/Decorations, STRUCTURE_STARTS radius8 und CARVERS radius1 sowie echte Completion, vollständige state/core/section/map/counter/entity/PP/tick before/after und unbeobachtete Kontrollgeneration fehlen. Keine allocateInstance-ServerLevelattrappe oder Statussetterabnahme. Späterer Simconsumer benötigt Reservation aller neun Owner über Shardnähte, erneuten vollen Versioncheck und atomare Installation an einer Tickgrenze; Resultdrop hat keine Weltwirkung. Diese Arbeiten bleiben getrennte Tickets und Abnahmen.

Kleinste Folge: queued GEN-06.2 erhalten; zuerst eine getrennte Authorcorrection mit B1/B2/B3 und konkreten Rootentscheidungen. Danach erst ein privater World-only GEN-06.4 mit ControlledOreInputs, demselben geschlossenen Dispatch/Pending/Core/Work-/Completionpfad und tatsächlichen Candidatebuffers. Echte Producer-/Region-/Featuredriver, Whole-FEATURES und Simadoption bleiben separat. Ein privater successful Einzelattempt ist keine useful Full30-Completion.

## 7. Abschließender Freeze, outside bytes und Reproduktion

Authorcandidate und sämtliche enumerierten 3878 Dateien /599591972 B wurden unabhängig vor und abschließend nach den Proben normal/−O vollständig gehasht und auf tatsächliche declared Membership geprüft. Die vier historischen Freezes mit1147/164/366/494 Dateien,1728 originale Inputpins, exakte454-Dateien-JDKmembership (21.0.12.1+1),190 ganze Source-/Patchkopien gegen tatsächliche Origins und894 Ressourcen gegen originale ZIPentries sind unverändert. Alle elf Original-/Transformclasspins, ganze zusätzlichen ShortList-/Kontext-/Pythonbytes, immutable1450-Baseline und324 angenommene Worlddateien samt accepted review copy wurden überprüft. Ganze Inputs werden nicht nur mit verkürzten Metadaten benannt.

`outside-byte-comparison.json` enumeriert separat das Reviewer-Vorher-/Nachherfenster:1450 live Baselinepfade, drei Byteänderungen (`docs/IMPLEMENTATION_BOARD.md`, World `surface_rules/session.rs`, `surface_rules/tests.rs`); Live-World366→366, drei Byteänderungen (die zwei Worldfiles sowie `tests/carver_runtime_layout.rs`). Normal/−O Endinventare sind gleich. Exakte Vorher/Nachher-Hashes und Byteumfänge stehen im Receipt; diese concurrent Änderungen wurden nicht als Authority, Revieweränderung oder Reviewabnahme behandelt. Worldbuilds0, Produktionsoverlay leer.

Der eigene vollständige Freeze liegt in `.codex-tmp/gen06_3-independent-review-r1/freeze-manifest.json`. Er umfasst diesen Bericht, sämtliche eigenen Tools, frisch erzeugte Sources/Classfiles/Mutanten, finale positive/negative Outputs, Inputs-/Provenanz-/Census-/Vergleichsbelege. `input-freeze.json` enumeriert zusätzlich alle ganzen readonly externen Candidate-/historischen-/Original-JAR-/Sourceorigin-/JDK-/Archiv-/Kontext-/Pythoninputs mit Bytes, SHA256 und Kategorien. Der finale Verifier prüft alle diese Bytes und die exakte Review-/Candidate-/JDK-/Archivmembership normal und −O.

Bewusst ausgeschlossene eigene Zwischenstände: `full30-observed/**` (Entwicklung, späte Declaration und doppeltes Seeding; keine Evidenzauthority), `work-normal.json`, `work-final-normal.json`, `work-final-optimized.json` (durch `work-final2-*` ersetzt), Pythoncaches. Outer Selfhashdateien `freeze-manifest.json`, `hashes.json`, `verify-normal.json`, `verify-optimized.json` werden zur Vermeidung eines zirkulären Hashes nicht in den eigenen Manifestinhalt aufgenommen. Die exakten Bericht-/Freeze-/Inputfreezehashes und Counts stehen in `hashes.json`, abschließende whole-byte-/Membershipreceipts in `verify-{normal,optimized}.json`; Root kann sie ohne Authorrefreeze separat publizieren.

Reproduktion erfolgt ausschließlich in neuen privaten Pfaden. Die Tools enthalten die genauen originalen compile/JVM/JAR-/CodeSourceflags und deklarierte Inputs. Relative Aufrufe aus der Reporoot:

```text
python -B .codex-tmp/gen06_3-independent-review-r1/tools/focused.py FRESH observed|control
python -B .codex-tmp/gen06_3-independent-review-r1/tools/wrapper.py FRESH observed|control
python -B .codex-tmp/gen06_3-independent-review-r1/tools/full30.py FRESH observed|control
python -B .codex-tmp/gen06_3-independent-review-r1/tools/fixtures.py FRESH
python -B .codex-tmp/gen06_3-independent-review-r1/tools/inverse.py FRESH
python -B .codex-tmp/gen06_3-independent-review-r1/tools/guards.py FRESH
```

Python −O wurde jeweils zusätzlich benutzt; bestehende Freshoutputs werden nicht überschrieben. `compare.py`, `full30_checks.py`, `work.py` und `expected_checks.py` dokumentieren die finalen Vergleiche/Modelle und ergänzten Mutationen. `integrity.py` rehasht readonly Candidate/Historie/Origins. `freeze_review.py` prüft Reviewerfreeze und externe Inputs mit expliziten Fehlerchecks statt entfernbaren Pythonasserts. Der Bericht erteilt keine weitere Schreib-, Runtime- oder Produktionsauthority.
