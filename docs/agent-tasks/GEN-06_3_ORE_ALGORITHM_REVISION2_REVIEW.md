# GEN-06.3 — unabhängiger Revision2-Review

12. Oktober 2026. **Verdikt: FINDINGS.** Die ursprünglichen IDs CR63-B1/B2/B3 bleiben erhalten. Der separate Compositionentwurf behebt die ursprüngliche 1Mi-Scopeverwechslung weitgehend; sein Heightmapvertrag ist inzwischen konkret überholt. Zwei unabhängige neue Gegenbeispiele verhindern die behauptete vollständige Proof-/Payloadabnahme. Dies ist ausschließlich eine Vorbereitungskontrolle. Root entscheidet alle neuen Caps und privaten Schnittstellen separat.

Kein Git, Agent, Rustbuild, Productionoverlay, Dependency-/Lockfilewechsel, Authorrepair oder historische Mutation. Eigene Writes liegen nur in diesem Bericht und `.codex-tmp/gen06_3-revision2-independent-review/` (V). A bezeichnet `.codex-tmp/gen06_3-revision2/`. Der Carverwriter und die pending Heightmapvorbereitung bleiben getrennte, unangenommene Authority.

## 1. Vollständige bindende Eingänge und unabhängige Intake

Gelesen wurden vollständiger Originalauftrag/-kandidat, erster Reviewauftrag/-befund, Revision2-Korrekturauftrag/-antwort und dieser neue Reviewauftrag; die gesamte ursprüngliche/korrigierte GEN-06.1-Auftrags-/Vertrags-/Reviewkette, queued GEN-06.2, AGENTS/CLAUDE/PAPER_REFERENCE, relevante Architektur/ADRs und archivierte Registry-/Worldinvarianten/Producerinterfaces. Tatsächliche Implementierungen/Callees umfassen Decoration/FeatureSorter/GenerationSettings, RNGwrapper/Core/Provider, Placement/Feature/ConfiguredFeature/Biomefilter/Tags, Ore/Bulk/Section/Paletten/ShortList, Region/ProtoChunk/ChunkAccess und Heightmaps/FEATURES-Tasks/Pyramid. Beide relevanten Source-/Featurepatchlagen und tatsächliche Originalclassbytes wurden geprüft. Ganze Moonrisepins behaupten keine Prüfung sämtlicher fremder Patchmechaniken.

| Authority | SHA256 / Umfang |
| --- | --- |
| Originalkandidat | `49746a75ef24eb952ea73198741a6b80e262de8bec9da325cdf7e6dfa3ee758d` |
| Originalfreeze | `a919bc57099d1791f1021cf03aca9716fb90be5f4cb700c4fb2f824370252593`, 3878 Dateien |
| erster Befund | `efdf02482cc4317014d7c4543bbee14fdcd0d5c8716c4672c6767f158b74d27e` |
| erster Reviewfreeze | `9462b5aa4cf8343d5fab7e4b950fbd60506196b13a847dbfb8d7100358ba7ef7`, 2383 Dateien |
| Korrekturauftrag | `d913b56bdeb6ae14f639476c9c55e49afacd5d0da414bd6f3d719ba6c7ea541e` |
| Revision2-Antwort | `b4b38a4f453cd7235d8e178cb915d3326d47337d4164744865b8617fe51f7381` |
| Revision2-Freeze | `8e0ca5579560dd1d21f9fe39ebefa5f467a104735e8bb0cf8dd360b5aff714a6`, 5897 Dateien /14045912630 B |
| Revision2-Inputfreeze / Zusatzinputs | `66bb3da4ce73d039388f3e24686dffc631bb1d86b9b1f9757b231d99349d160d` / `1e63e6a2fb3f06502431a46536a00d9da32423e708d10ed5135e39963df91d88` |
| neue pending Heightmapantwort | `469933e39a0cd41e6374ae9f66479cecb702cd56419d6c30f4e7d4d855bd292e` |
| deren pending Freeze | `f49d4664fdb31f6d690e233250591912b0b71494d90b0878a5a99ce930144e5f` |

V/tools/integrity.py streamt selbst alle5897 Authorbytes und alle12293 gemergten externen Pins /5041635179 B; kontrolliert tatsächliche Author-/Historical-/JDK-/Archivmembership und jeden legal-Symlink gegen separat enumerierte gepinnte Zielbytes. Roots Intake ist keine Claimannahme. Die korrigierte volle Vorprüfung bestand; die erste Verifierfassung mit falsch interpretierten digest-only Archivrecords bleibt als verweigerter Versuch erhalten. `source-resource-audit.json` überprüft zusätzlich190 vollständige Herkunftskopien,894 originale ZIPressourcen,1728 Originalpins und die vollständige454-Dateien-JDKmembership.

Runtimebasis bleibt ausschließlich Main47eadc4c,1490/324-Dateien-Archiv mit Freeze `58ba13d6418133bb49a154682ba1d82533f63a5e6184c6c2a8b07c6c15a47a75`; zusätzlich das1499-Dateien-356670f7-Archiv mit Freeze `3788f6be81effe6d4d06db3bf4380319844ed3738dbf80dc90e62a119041d6ae`. Originale: Minecraft1.21.8/protocol772, PaperBuild60/29c8822d, JDK21.0.12.1+1; Vanilla-Inner `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`, Paper-Inner `c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`. Shared World/Carver sind keine Oreproducerbasis. H1 bleibt ausschließlich die historische Reviewbeschreibungskorrektur: Originalguard bereits `abs(block >> 4)`. Keine rückwirkende Helperänderung.

## 2. CR63-B1 — HIGH, Vertragskorrektur nötig: Query8 passt nicht zum neuen Query53-Consumer

**Exakte Stellen:** A-Antwort §5/Zeile125, §11/Zeile264 und A/tools-extra/work_model.py rechnen8 je Heightquery; Root hat inzwischen die pending Heightmaprevision2 §§5–7 mit53 vollständigen Queryunits benannt. Beide eingefrorenen Kandidaten sind readonly und keiner genehmigt seine Caps oder Interfaceauthority.

Die neue Queryreservation ist unabhängig nachgerechnet: `24 Guard +6 Kind +5 Column/Index +1 Wordread +3 Extract +1 Range +3 Outputs +2 Return +8 Cleanup =53`. Der Regionadapter/Chunkselectiongroup1 bleibt zusätzlich bereits im Ore-Ereigniscensus. Deshalb wird **45 je tatsächlicher Query**, nicht53 zusätzlich und nicht nur je Attempt, addiert. Die früh aus dem Gate ausgeschlossenen Eisen-/Smaragdversuche fragen oft mehrere Columns; Attempts dürfen den Querycensus nicht ersetzen.

V/tools/models.py streamt jeden Originalregiontrace und zählt jede `CALLBACK height` unter dem tatsächlichen `declared.plan`-Key/Step/Index. Es rekonstruiert das vollständige finale Autorenmodell eigenständig, einschließlich positiver und terminaler Copyheads, Initialization, Spherebody/Setter, Air-/Releaseheads, Primitivehandoff, Admin, Biomecorners und Publication. Keine importierte Autoren-Gesamtsumme oder nachgeschriebene computational Oracle wird Expected. Normale/−O Receipts sind bis auf Pythonmode gleich; beide Originalprofile haben dieselben Censuses. `model-equality.json` prüft zusätzlich jede Biomplanquery gegen ihre genuine Full30-Seedzelle. Alle318 Perfeaturecensuses je Profil stehen in `models-{normal,optimized}.json`.

| Plan | Queries Seed0 / −1 | Query53 maxPP total Seed0 / −1 | Margin Seed0 / −1 |
| --- | ---: | ---: | ---: |
| full30 | 1812 / 1651 | 3848413 / 3791834 | 345891 / 402470 |
| plains | 962 / 801 | 3383354 / 3326989 | 810950 / 867315 |
| badlands | 1012 / 851 | 3424967 / 3368272 | 769337 / 826032 |
| dripstone_caves | 962 / 801 | 3410259 / 3352928 | 784045 / 841376 |
| lush_caves | 1008 / 847 | 3694067 / 3637995 | 500237 / 556309 |
| windswept_hills | 1700 / 1539 | 3454651 / 3398100 | 739653 / 796204 |


Das frozen größte Modell3766873 mit427431 Margin wird bei Full30 Seed0 durch1812×45=81540 zu **3848413**, Margin **345891** unter proposed4194304. Ohne maxPP beträgt dieser Pfad3586269. Full30 Seed−1 addiert1651×45=74295. **Alle zwölf erklärten vollständigen Kontrollpläne passen weiterhin rechnerisch.** Der nützliche kleinste Allowancewert für das größte vollständig deklarierte Modell ist jetzt3848413; dies nimmt weder ihn noch2^22 an. Kein weiterer Ceilingwechsel ist durch diese Querykorrektur numerisch nötig. Arbitrary Seedcompletion oder natürliche FEATURES sind daraus nicht ableitbar.

Vollständiger Full30-Querycensus; die fünf Biompläne wählen ihre echten Teilmengen derselben Seedzellen:

| Original Key | Step/Index | Queries Seed0 / −1 |
| --- | --- | ---: |
| `minecraft:ore_dirt` | 6/0 | 7 / 7 |
| `minecraft:ore_gravel` | 6/1 | 14 / 14 |
| `minecraft:ore_granite_upper` | 6/2 | 0 / 0 |
| `minecraft:ore_granite_lower` | 6/3 | 2 / 2 |
| `minecraft:ore_diorite_upper` | 6/4 | 0 / 0 |
| `minecraft:ore_diorite_lower` | 6/5 | 2 / 2 |
| `minecraft:ore_andesite_upper` | 6/6 | 1 / 0 |
| `minecraft:ore_andesite_lower` | 6/7 | 2 / 2 |
| `minecraft:ore_tuff` | 6/8 | 2 / 2 |
| `minecraft:ore_coal_upper` | 6/9 | 30 / 30 |
| `minecraft:ore_coal_lower` | 6/10 | 20 / 20 |
| `minecraft:ore_iron_upper` | 6/11 | 810 / 650 |
| `minecraft:ore_iron_middle` | 6/12 | 10 / 10 |
| `minecraft:ore_iron_small` | 6/13 | 10 / 10 |
| `minecraft:ore_gold` | 6/14 | 4 / 4 |
| `minecraft:ore_gold_lower` | 6/15 | 1 / 1 |
| `minecraft:ore_redstone` | 6/16 | 4 / 4 |
| `minecraft:ore_redstone_lower` | 6/17 | 8 / 8 |
| `minecraft:ore_diamond` | 6/18 | 7 / 7 |
| `minecraft:ore_diamond_medium` | 6/19 | 2 / 2 |
| `minecraft:ore_diamond_large` | 6/20 | 0 / 0 |
| `minecraft:ore_diamond_buried` | 6/21 | 4 / 4 |
| `minecraft:ore_lapis` | 6/22 | 2 / 2 |
| `minecraft:ore_lapis_buried` | 6/23 | 4 / 4 |
| `minecraft:ore_copper_large` | 6/24 | 16 / 16 |
| `minecraft:ore_copper` | 6/25 | 16 / 16 |
| `minecraft:ore_clay` | 6/27 | 46 / 46 |
| `minecraft:ore_gold_extra` | 6/28 | 50 / 50 |
| `minecraft:ore_emerald` | 6/33 | 724 / 724 |
| `minecraft:ore_infested` | 7/4 | 14 / 14 |


**Erforderliche konkrete Korrektur:** neuer Authorresponse/-freeze muss Query53, alle Perplan-/Regionzahlen, Usefulminimum und Margin durchgängig übernehmen. Die bereits genannten vier privaten Primitiveamendments reichen nicht für Heightmaps: zusätzlich braucht es den separat geprüften sealed Queryadapter, der vor jedem Mapread direkt53 am **aktiven tatsächlichen Single/Composite-Oreowner** reserviert, neben dem separaten Chunkselectiondebit. Kein neuer HeightPreparationmeter, Reset, Upgrade, frei implementierbares Worktrait oder Testproxy als Producer. Adapter bindet echten Map-/History-/Owner-/Version-/Geometry-/Kind-/Phasebeweis und behält original firstAvailable/Chunk-highestTaken+1. Reservationrefusal führt keinen Mapread aus und mutiert weder RNG/Meter noch veröffentlichte Ownerbytes. Queryfehler nach Admission folgen der terminalen Orepolicy; Cleanup ist in53 enthalten. GEN-06.2 hat derzeit keinen solchen Consumerdispatch. Root und der eigene Heightmapreview müssen diese konkrete Schnittstelle zuerst disponieren. Ohne angenommenen Producer/Getter bleibt Publicadmission verweigert.

### Bereits sinnvoll korrigierte Composition, noch keine Runtimeabnahme

Der neue separate ClosedCompositionOwner besitzt einen Core/Wrapper, immutable RegionAllowance, echte Cursorstate und neun Candidates. Ein sealed geliehener Executionowner mit Single-/Compositevarianten ist ein implementierbarer Weg für dieselben Draw-/Modifier-/Emission-/Dispatch-/Completionoperationen. Ein Feldborrow über kurzlebige disjunkte Cursor-/Core-/Workparts braucht keinen zweiten Cursor oder Meter. Original Single1Mi bleibt unverändert und kann weder übertragen noch upgraded werden. Die vier expliziten privaten Deltas (Cursorstate trennen; echte RNG/Chargecallee erweitern; Pending/Receiptidentity variieren; ausschließlich Composite-BetweenPlans reseeden) sind noch zu reviewende Interfaceänderungen, keine bestehende Primitiveauthority.

Exclusive Pendingborrow verhindert zweites next/Planwechsel. Owner/Katalog/Plan/Candidatesequenz muss echt gebunden sein; interne Biomeentscheidung und Consumer erzeugen den Receipt erst nach admitted Completion. Option::take bleibt bis nach jeder falliblen/panikfähigen Aktion armed. Drop/failure/unwind sind terminal; forget lässt Pending zurück und spätere Wiederverwendung verweigert ohne Core-/Workmutation. Result übernimmt dieselben Candidatebuffer, kein dritter Payload. Letzter Versionscheck und ganzer Commitpermit vor einem einzigen infallible Ownermove; neun Einzelinstallationen wären Vertragsbruch. Vergessene ganze Owner können Ressourcen leaken, erhalten aber keine neue Publicationauthority; Cleanupgarantie kann mem::forget nicht erzwingen.

Die tatsächliche Outerchronologie ist jetzt richtig abgegrenzt: Decorationseed einmal, vier anfängliche Wrappercores, originale canonical Step/Indexfeatureseeds, kumulativer Java-int-count trotz setSeed/setFeatureSeed; ausgelassene Nicht-Oreconsumer sind keine erfolgreich gefahrenen Steps. Sorterdiscovery benutzt equals/hashCode, spätere Stepindexlookup Identity, hasFeature Equality. Original Paperfeatureseedmap verwendet default−1; ein echter Override würde zusätzliche Decorationseeds und Corecalls erzeugen und bleibt außerhalb dieses default-only Vorschlags. Strukturen und vorangehende/interleavte Features fehlen. Original count++ vor delegiertem Core, inklusive Overflow/Throw, bleibt getrennt vom FerrumC successful-core/debit.

### Finale Work-/Nativearithmetik und echte Lebenszeitgates

Eigene finale Copyrechnung: State1769481 +Maps4050 +Fluid27657 +Counter441 +leerePPheads216 = **1801845**; Validation2048; vorab ausgegebene Zusatzreserven **598119**, davon Cleanup524288, Reserve/Release512, Header32769, Geometry2049, Auswahl352, Metadata1024, Biomevorbereitung36865, Publicationmetadata256, Constructor4. Volle PP adds262144. Ersetzte A/work-normal/-optimized.json sind keine finalen Belege. Copy/Libraryconversion/zero-fill/reserve/error/cleanup dürfen in einer späteren Umsetzung keine zusätzliche unbezahlte Arbeit verbergen: fallible reserve vor push innerhalb echter Capacity ist darstellbar; zusätzliche Initialisierung oder Box-/Shrinkcopy braucht weitere Debits und Bytes.

Eigenständig addierter simultaner Nativevorschlag: **15042707 B**, Margin1734509 zu16Mi; darunter Stateinput+candidate7077888, packed Maps31968, beide Fluidsets221184, beide Counter/Flags6912, volle Tickvalues1769472, beide PPsets1048576, Borrowbiome18936, Zoom512/Membership4096, Classification223568, Source26987, Katalog16384, Cursor/Core/Meter512, Geometry8192, Entities inklusive Indizes2097152, Tickreverse-/Hashindizes zusätzlich2097152, Header131072, übriger Stack/Temps/Alignment262144. Query53 benutzt feste PODlocals; die alte gleichzeitige Byteaddition wird dadurch nicht automatisch erhöht oder angenommen. Die aktualisierte Heightmaprepresentation/-scratch muss tatsächlich in diesen Partitionen landen.

Geometry2048+1184+864+4096=8192; size≤64, Radius≤4.5 einschließlich Paper1.0, Mask9464/148words,2016 Paare und27 Handles sind strukturelle Produktionsdomainbounds, keine nativen Messungen. Original full4096 Tickvalues in216 Sections benötigen allein **7078752 B** Hashkey/-valuearrays (8192+1 pro Array); das verletzt das neue2Mi-indexziel und wird korrekt verweigert. Kein Borrow der Values darf diese gleichzeitig lebenden Indizes verschwinden lassen.

Cleanup131072 Nodes/depth32/524288units und512 Reserve-/Releaseaktionen sind **neue Produceradmission**, keine Fähigkeit bestehender Chunk/BTreeMap/NBT/Vec/Stringowner. Ein echter Producerdescriptor muss alle gleichzeitig gehaltenen Input-/Candidate-/Sidecar-/Index-/String-/Container-/Refcountdropverpflichtungen erfassen; Flat-PODbuffer/Arenen sind unter solchen Bounds darstellbar, beliebig viele einzelallokierte NBTstrings oder pro-SectionVecs nicht automatisch. Shared old Chunkdrop kann nicht implizit kostenlos werden. Destructorrelevant loops müssen dem Nodeplan entsprechen; alle Allocations/Deallocations und tatsächlichen Capacities gehören zur Admission. Precharged, ownergebundene, nichtclonebare/nonrefundable Cleanuppermits stammen ausschließlich aus demselben Meter, werden mit dem Candidate/Result übernommen und dürfen nach Originalownerdrop nicht auf freigegebene Sidecars zeigen. Native reservefault/unwind/last-refcountdrop und Resultlifetime sind noch echte Runtimepflichten. Leere Javaentities/maps beweisen diese Producer nicht.

## 3. CR63-B2 — BLOCKING: typed Bootstrapkonstante ändert echte Berechnung, ExactProof akzeptiert

**Ort:** A/tools/ProofVerify.java, `code(MethodNode)`, InvokeDynamiczweig mit `Arrays.toString(d.bsmArgs)` (whole-file SHA `43dec508cdcac4bfe3374e1ced3356aeefe661f14774f2d4ef3f0030c8b3a820`). Die Serialisierung verliert Bootstrapkonstantentypen. Dies widerspricht §8s Constants/Types/InvokeDynamic- und vollständiger computational-identity-Zusage.

V/tools/BootstrapMutation.java ändert in der tatsächlichen computational `PlacedFeature.placeWithContext(PlacementContext,RandomSource,BlockPos)Z`-FlatMaplambda genau Bootstraparg0: Original ASM Type mit Descriptor `(Ljava/lang/Object;)Ljava/lang/Object;` wird zu einer Java Stringkonstante mit **identischem Text**. ClassWriter(0) erhält Code/Frames; tatsächlicher privater JVMloader definiert die Klasse erfolgreich. Danach wird die echte Originalmethode beziehungsweise Mutante mit genuine Oreconfiguration/Holder und `CountPlacement.of(0)` aufgerufen. Die erfolgreiche Originalkontrolle emittiert keine Position und gibt false zurück; die Mutante erreicht denselben invokedynamic und wirft BootstrapMethodError, verursacht durch ClassCastException String→MethodType.

| tatsächlich ausgeführt, beide Profile normal/−O | Resultat |
| --- | --- |
| genuine Original Count0-placement | `return:false` |
| valide definierte computational Mutante | `throw:java.lang.BootstrapMethodError cause:java.lang.ClassCastException` |
| neuer whole-method Proof über dieselbe Mutante | **ACCEPTED:126 Vanilla /134 Paper, selected36** |

Kein malformed-class-Reject wird als computational Negativkontrolle gezählt. Keine solche Mutation im gelieferten Agent wird behauptet. Dies ist ein tatsächliches Gegenbeispiel zur **Prüfergarantie**, mit positiven ganzen legitimen Klassen und erfolgreichem Originalcall.

V/proof-success-{normal,optimized}/results.json, tatsächliche `.class`-Mutanten, Original-/Mutantausführungslogs und angenommene Proofs sind vollständige Evidenz. Der frühere null-holder-Control bleibt als schwächere zusätzliche genuine NPE→BootstrapMethodError-Differenz erhalten; der erste ambiguous-Type-Compileversuch ist fehlgeschlagen und als solcher gefroren.

Die ursprüngliche Reviewmutation wurde unverändert frisch reproduziert: ICONST_0/ISTORE1 im marker-contained WorldgenRandom.next verändert next24 **2763940→−160476802**, alter StripVerify akzeptiert126/134, neuer Proof verweigert. Zusätzlich alle acht tatsächlichen original-slot/call/field/return-operand/branch-predicate/branch-target/replace-throwable/swallow-throwable-Mutanten pro Profil/Modus verändern wirklich Return/count/Exceptionidentität und werden verweigert. Zusätzlich schrieb der Reviewer V/tools/OwnComputationalMutants.java unabhängig: bits23 statt24, setSeed41, count−7, originaler Returnoperand XOR1, IFEQ→IFLT, echter GOTOreturn−123, neue IllegalStateException statt derselben Sentinelthrowable und swallowed Return−321. Diese acht eigenen Varianten werden auf den frisch selbst kompilierten normalen/−O-Klassen wirklich JVM-ausgeführt; jede verändert Return/count/Exceptionidentität und jeder Proof verweigert. V/own-mutants-{normal,optimized}-complete/results.json enthält alle32 eigenen consequential Kontrollen. Die zuerst ausschließlich am unbenutzten Legacyreturn eingefügte XORmutation war inconsequential und wurde vom eigenen Harness verweigert; source/log/classes bleiben im failed own-mutants-normal erhalten. Vollständige Positivproofs aller elf Klassen126/134 Methoden bestehen. Diese Erfolge schließen das Bootstrapgegenbeispiel nicht.

**Erforderliche Korrektur:** typisierte strukturelle Gleichheit aller executable Operanden, besonders Bootstrapargs; kein Arrays.toString/toString als computational Equality. Erhalten: Constantpoolkind; Type-sort/Descriptor; Handle-tag/owner/name/descriptor/interface; exakt rohe Float-/Doublebits; rekursive ConstantDynamic-Namen/-Descriptor/-Bootstrap/-Argumente. Gleiches Prinzip für LDCs. Eigene gültige bootstrap-type-Regression muss normal/−O verweigern bei unverändert vollständigen legitimen Positivkontrollen und tatsächlichen Originalreturns. Danach neue Proofquelle, alle Klassen und Belege separat einfrieren/reviewen. Bytepin einer falsch akzeptierten neuen Klasse ersetzt keine Noninterferencegarantie.

Die konkreten Templates sind sonst nachvollziehbar: original maxlocals L, disjoint new frame L/L+1 und Tempbereich, unveränderte computational DASTORE/BitSetcalls mit Operandspill/reload, typed Returnreload/Boxing, originale Predikate/Targets/Ordinals, ursprüngliche Handlerpriorität plus letzter catch/rethrow derselben Throwableidentität. Debug/Frames/maxstack bleiben aus computational Equality ausgeschlossen und brauchen echte JVMvalidität. Observer-Entry vor dem zusätzlichen Handler und Logger/Boxing/IO/OOM können Ressourcen-/Fehlerchronologie ändern; die Antwort begrenzt Preservation ausdrücklich auf vollständige erfolgreiche beobachtete Runs mit unbeobachteten Controls. Es gibt keine allgemeine Ressourcen-/instrumentation-failure-Exceptionparität.

## 4. CR63-B3 — HIGH, CLEAN_ORACLE-blockierend: Candidatevollständigkeit durch ignorierte Tracefiles umgehbar

**Ort:** A/tools-extra/lossless.py, `equality()` top-level `iterdir`, beide `actual.discard('trace.tsv[.gz]')`, externe `--trace`-Auswahl. Der aktuelle mandatory Expectedpfad ist deutlich besser: Originalfreeze/-root und unabhängiger Reviewpin sind fest; candidate-as-reference/self-minted freeze/falscher Authorpin/fehlender Reviewpin/unvollständige Expectedmembership und missing --reference werden wirklich verweigert. Die gewöhnliche gewählte Raweventfolge wird bytegenau ohne Sortierung/Normalisierung verglichen.

Der Kandidatinhalt wird dennoch nicht vollständig gebunden. Eigene vollständige Mutantstreams ändern die exakte alte Sphere0-Zeile `f64:402efabc05b261b6→f64:0000000000000000`. Nur eigene Outputpfade wurden von den deklarierten Mutationsdefinitionen abgeleitet; provenance und vollständige gzip-Bytes sind gefroren. Alle übrigen Candidatefiles bleiben exakt original. Tatsächliche StandaloneCLI-Ergebnisse in **beiden Profilen und Pythonmodi**:

| eigener Candidate | tatsächlicher Prüferbefund |
| --- | --- |
| korrekte trace.tsv plus sphärenmutierte trace.tsv.gz | **ACCEPTED**, mutierte alternative Payload vollständig ignoriert |
| zusätzliches hidden/trace.tsv.gz mit denselben mutierten Bits | **ACCEPTED**, rekursive Computationalmembership ignoriert |
| sphärenmutierte Candidate-trace.tsv; --trace zeigt auf korrekte externe Author-trace.tsv | **ACCEPTED**, Candidatepayload nicht verglichen |

Dabei behauptet jeder CLIoutput `membership_and_all_payload_bytes:true`,406 Computationalfiles und2644242/2644304 lossless lines. V/payload-{normal,optimized}/results.json, sechs CLIlogs pro Modus, vollständige Candidate-/Mutantbytes und Commandarrays belegen die Annahmen. Hardlinks halten unveränderte Kopien effizient; mutierte Files wurden ausschließlich im eigenen Area neu erstellt. Kein Authorfile wurde geändert. Das ist ein Vollständigkeits-/Auswahlfehler, keine Behauptung einer tatsächlich mutierten Authorrecording.

Die exakten alten Structuralfindingzeilen wurden ebenfalls unverändert reproduziert: Branch3 true→false und Sphere0 werden ohne Census akzeptiert; echte positive Censusreferenz verweigert Branch und akzeptiert weiterhin Sphere. Neuer lossless selected-stream-Vergleich verweigert dagegen alle17 Mutationen je Profil/Pythonmode: beide alten Zeilen, CenterY/Z, Radius, Cull, Branchrichtung, signed/Corebit, rawf32/f64, WriteState, Mask, Release, Missing, Duplicate, Reorder. Alle34 Refusals je Modus und echte positive externe Expectedkontrollen stehen im Reviewarea. Die −O-Challenges verwenden die vollständig bytegleichen frozen Normalstreams; die unabhängigen fresh −O-Originalruns prüfen diese Bytegleichheit zusätzlich. Optionaler Census bleibt keine Ersatzauthority.

**Erforderliche Korrektur:** genaue vollständige rekursive Candidate-/Referencecomputational-Membership binden. Genau eine zugelassene Rawrepresentation, oder alle vorhandenen plain/gzip-Repräsentationen vollständig dekomprimiert vergleichen und mehrdeutige/abweichende zweite Payload verweigern. Keine unerwarteten nested computational Dateien. Externes --trace darf keine Ganzcandidate-Erfolgsaussage erzeugen: remove/restrict auf den eindeutig admitted tatsächlichen Candidate-Member, oder klar separater Einzeltrace-Testmodus mit eigener Ergebnisauthority und weiterhin vollständiger Candidateprüfung. Positive historische Expectedpins bleiben mandatory. Alle drei konkreten Membership-/Substitution-Gegenbeispiele normal/−O verweigern; alle bisherigen positiven und34 gewöhnlichen Mutationskontrollen erhalten.

## 5. Frische eigene Originalkontrollen, getrennte Chronologieabnahme

Die eigenen frischen vollständigen Originalruns führen je Profil/Pythonmode101 declared Fälle beobachtet und unbeobachtet aus;406 Computationaldateien inklusive404 vollständiger before/after State-/Metadatasnapshots sind jeweils bytegleich. Ganze Rawevents matchen die mandatory externe ursprüngliche101-Referenz verlustlos:2644242 Vanilla-/2644304 Paperzeilen einschließlich Header. Strukturchecker und alle41 Semantikedges bestehen. Keine lediglich focused Reproduktion wird als full101 ausgegeben.

Zusätzlich eigene zwölf kumulative Regionen (Full30 und fünf echte Biomplanmengen, Seeds0/−1) beobachtet/unbeobachtet und normal/−O:51 Computationaldateien je Profil,48 Snapshotfiles plus Inventory/Results/Featureendstates. Alle318 vollständigen Perfeatureendstates je Profil, jeder genuine Key/Step/Index/Holder/Return/Corestate und kumulative count stimmen bytegleich; komplette Rawtraces normal=−O. compare prüft alle164 originalen Exportrecords, tatsächliche chronologische Planorder, genau eine Decorationseedinvocation je Region und count=wirkliche WorldgenRandom.next-Entries. Diese kontrollierten Inputs behalten STONE/Gate320/chosen Holder und fehlende Nicht-Oreconsumer ehrlich bei.

Drei zusätzliche Wrappercontrols je Profil/Pythonmode beobachtet/unbeobachtet:14 Computationaldateien einschließlich vollständiger mutable Scriptcore-/Wrappercounts matchen; rejection count2/calls2/remaining[], Doublemax count2/calls2/constant−1, Throw count1/calls1/always-throws. Separate echte unmodifizierte Overflowcontrols injizieren nur den Diagnosecount MAX: next24 überschreitet zuMIN; setSeed/setFeatureSeed behalten ihn, Decorationseed erhöht durch vier Originalcores zuMIN+4.

Normale/−O Computational-/Rawtrace-/Helperclass-Membership und ganze Bytes stimmen in allen drei Familien; jeweils22 actual Original-/Transformclassdateien pro Profil/Familie sind ebenfalls exakt bytegleich. Alle originalen Klassen werden frisch gegen ZIPentries und tatsächliche CodeSources geprüft, Agent/Observer/Probe frisch aus enumerierten Quellen kompiliert. Alle16 echten Guardrefusals/3 Positivkontrollen bestehen selbst normal/−O (fünf JVMenv-Entrypoints, sechs Profile-CodeSourcefaults, private byte-/ZIPshadow-/JDK-/helpermembershipfaults und missing-referenceCLI). Geerbte Scope-/Fresh-/tool-membership-Guards sind im Runner weiter aktiv. Vergleichs-/Validation-/Semantik-/Guard-/Overflowreceipts liegen vollständig unter V/harness/. Die Probehelper-/Tooldeltas sind ausdrücklich nur private Ausgabe-/ROOTpfade, originale deklarierten Observer-/Region-/Wrapperoverlays und Referenzadaption; computational Originalmethoden sind nicht nachgeschrieben.

Keine neue konkrete Abweichung der **gelieferten** Originalorechronologie gefunden. Featureguard vor Angle/Ydraws; Heightgate X→Z bis firstsuccess; alle originalen size-Doubles/Spherewrites vor Culling, f32 ratio vor f64-Lerp, striktes Containment/Ties/−1-Cullradius; Sphere→X→Y→Z, buildheight vor Mask, mark vor Guard; geordnete Targets, mismatch ohne RNG; chance0/1 ohne Draw, dazwischen float≥chance skip; DOWN/UP/NORTH/SOUTH/WEST/EAST mit erstem Airshortcircuit, Wasser kein Air; echter equal-state-Write/true ohne Nettodelta; acquired Sections wirklich release/close beim Throw. Die41 consequential Edges und drei Wrappercontrols bleiben bindend. Direkte Sectionwrites erzeugen keine Region-/ProtoChunk-Heightmap-/POI-/Entity-/PP-Nebenwirkung. Vanilla nonempty-fluid versus Paper randomly-ticking-fluid und Tick-/Specialcollisioncallback bleiben getrennt. Snapshotordnung wird nie als Writeordnung ausgegeben.

Neue genuine Regionreferenzen werden in V/region-reference-{vanilla,paper}.json nach eigenen vollständigen beobachtet/unbeobachtet-/normal/−O-Vergleichen separat gepinnt. Dies deklariert überprüfte Originalinputs/-outputs; es macht weder eine neue Hashliste noch den fehlerhaften Proof zum computational Expectedgenerator. Aktuelle author losslessCLI erlaubt weiterhin nur den vorab unabhängig gefrorenen101-Corpus. Jede künftige regional Expectedzulassung braucht explizite unabhängige Referenzauthority und einen korrigierten vollständigen Prüfer.

## 6. Rootdisposition und kleinster nützlicher Folgescope

CR63-B1: separater Owner/ein Meter/echter Primitiveborrow ist als neuer Entwurf tragfähig; old1Mi-Fullregionbehauptung ist aufgegeben. Query53-/Adapterkorrektur bleibt zwingend. CR63-B2: ursprünglicher marker-slot-Angriff abgewiesen, typed-bootstrap-Falseacceptance hält den Befund blockierend. CR63-B3: mandatory unabhängige Expectedauthority und die ursprünglichen Bitmutationen behandelt, tatsächliche Candidatevollständigkeit bleibt offen. Daher **weder CLEAN_CONTRACT noch CLEAN_ORACLE**.

Root kann aus diesem Befund eine reine neue Authorcorrection zu diesen exakten drei Restpunkten vergeben; keine historische Refreeze. Public GEN-06.2 bleibt queued und unverändert, einschließlich1Mi und angenommenen Bind-/Nativezielen. Nach eigener Review der tatsächlichen Primitivebasis: eigenes kleines World-only Interfaceamendment für dieselbe Cursorstate/sealed Compositeborrow/Pending/Seed-/Completionauthority und separat den Query53-Adapter nach angenommener Heightmapauthority. Danach kontrollierter Ore-Kernel mit echten neun Candidates und vollständiger nützlicher Full30-/Biomcompletion, echtem Parentmeter und actual native/error/late-publication-Tests. Kein cloned Testcursor, keine fake production-successvariant.

2^22,16Mi, PP131072, Entities-/Tickindexziele je2Mi, Cleanup131072/depth32/524288,512 Nativeactions,8192/27 Geometry und2048 Validation bleiben neue Rootentscheidungen. Tatsächliche Debug/Release N−1/N/N+1, capacities/layout/stack/alignment/positive4096-B allocatorcontrols, fallible reservefaults, lifecycle compileborrow/moves/forget/unwind und ganzer atomarer Commit müssen später am produktiven Kern geprüft werden. Hier wurden null Rustchecks ausgeführt.

Reale Producer fehlen: neun angenommene versionierte CARVERSowner, richtige immutable WG/finale Maps plus vollständige Classification/History, Storedfeaturebiomes/Zoom/full Generationmembership, echte Neighbor-/Stage-/Versions-/Sidecar-/Cleanupauthority und prior Structures/Decorations/non-oreconsumer. Jede fehlende Authority verweigert Admission. Defaultstatus-STONE, Gate320 und chosen Holder sind explizite Kontrollinputs. Whole FEATURES verlangt weiterhin echten gestarteten/vollständig initialisierten OriginalServerLevel, configs/packs/options/seeds/featureseedpolicy/blending, echte radius8 STRUCTURE_STARTS/radius1 CARVERS-Completions und vollständige before/after/Core/Section/Map/Counter/Entity/PP/Tickkontrollen, Forward/Reversecenterhistory separat. Kein Statussetter oder erfolgreich leerer Consumer. Spätere Simadoption muss alle neun Owner reservieren, vollen Versionsvektor erneut prüfen und atomar an Tickgrenze installieren; Resultdrop hat keine Weltwirkung. Keine Naturalworld-, Survival-, Lighting-, Plugin-, Nether/End-, Performance- oder Appannahme.

## 7. Abschließende Integrität und vollständiger neuer Freeze

Nach allen Proben prüfen after-normal und after-optimized erneut alle5897 Authorfiles /14045912630 B,12293 gemergte externe Pins /5041635179 B, originalen JAR/JDK/Mapping/Source-/Resourcebytes und Historical-/Archivmemberships. supplement-input-freeze.json bindet die später angekommenen beiden Heightmappins und konkrete Reviewer-Pythonbytes separat, ohne nachträgliche initiale Intakebehauptung. 1 geänderte,0 fehlende,2 neue Livepfade im eigenen Vorher-/Nachherfenster sind in outside-byte-comparison.json exakt mit Hashes/Bytes benannt; 0 server-Pfade. Sie werden weder als Revieweränderung noch als accepted Authority ausgegeben. Jede pinned Authority bleibt unverändert.

Failed/superseded Versuche: erste archive-record-Intakeverweigerung, ambiguous-Type-javac, inconsequential erste own-return-Mutation, schwächerer null-holder-Bootstrapcontrol und beiläufige readonly Status-/Pfadabfragefehler sind in attempts.json und ihren Sources/Logs gehalten. Die volle korrigierte Intake und successful Count0-bootstrap sind maßgeblich. Kein failed Versuch wird als positive Abnahme gezählt.

V/freeze-manifest.json enumeriert diesen Bericht und **alle** neuen Tools, Sources, Copy-/Textdeltas, Declarations, compiled Classes/JARs, Computational-/Mutantenpayloads, Referenzen, Input-/Outside-/Model-/Proof-/Guard-/Vergleichsreceipts einschließlich failed/superseded Attempts. Es gibt keine ausgelassenen Entwicklungsordner. Nur äußere Selfhashreceipts `freeze-manifest.json`, `hashes.json`, `verify-normal.json`, `verify-optimized.json` sind zur Vermeidung zirkulärer Hashes ausgenommen. Exakte File-/Bytecounts und Bericht-/Freeze-/Inputhashes stehen in hashes.json; beide finalen Verifier streamen jede Membership und jedes Byte erneut. Der Manifesthash kann nicht im selbst gepinnten Bericht stehen; Root erhält ihn separat ohne Änderung eines Author-/Historicalfreeze.

Reproduktion ausschließlich in frischen Namen innerhalb eigener Reviewpaths: V/tools/proof_challenges.py, payload_challenges.py, models.py; V/harness/tools/run.py; V/harness/tools-extra/regions.py, wrapper.py, overflow.py, validate_runs.py, guards.py; V/tools/compare_full.py, integrity.py und freeze_review.py. Alle Javaflags/CodeSource-/JAR-/JDK-/Mappingguards stehen vollständig im frozen source. Keine kopierte computational Implementierung wird zur erwarteten Oracle.
