# GEN-06.3 Revision2 — getrennte Regionkomposition und vollständige Originalproofs

12. Oktober 2026. **Autorenantwort zur neuen unabhängigen Prüfung; keine Runtimefreigabe.** Maßgeblich ist der vollständige neue Auftrag `GEN-06_3_ORE_ALGORITHM_REVISION2_TASK.md`, SHA256 `d913b56bdeb6ae14f639476c9c55e49afacd5d0da414bd6f3d719ba6c7ea541e`. CR63-B1/B2/B3 werden hier erstmals beantwortet. Kein CLEAN- oder Rooturteil wird vorweggenommen. Alle neuen Region-, Work-, Native-, Cleanup- und Schnittstellenzahlen sind **pending review**.

Schreibscope: ausschließlich dieses Dokument und `.codex-tmp/gen06_3-revision2/**` (nachfolgend A). Keine Gitaktion, Agenten, Produktions-/Dependencyänderung oder Sharedbuild. Carver bleibt exklusiver Worldwriter. Die parallele Heightmapvorbereitung ist ein unangenommener Schnittstellenvorschlag; kein fremder Child wird zur Authority. Die angenommenen GEN-04/05-Limits und der ganze queued GEN-06.2-Auftrag bleiben unverändert.

## 1. Unveränderter vollständiger Auftrag und Antwortmatrix

Der gesamte Originalauftrag, sein 185-Zeilen-Vertrag, der Reviewauftrag und 177-Zeilen-Review sowie die vollständige GEN-06.1-Original-/Revision2-/Reviewkette und GEN-06.2 bleiben bindend. Die Originalchronologie, Geometrie-/Semantikfälle, Producer-/Lifecycle-/Adoptionpflichten und künftige Runtimeprüfungen gelten weiter. Die nachstehend explizit ersetzten Aussagen des alten §2/4/6–8 dürfen nicht weiter als Garantie benutzt werden.

| Finding | Alte Behauptung / tatsächliches Gegenbeispiel | Neue Verpflichtung und eigene Antwort | verbleibende Entscheidung |
|---|---|---|---|
| CR63-B1, blocking | Ein 1Mi-Single-key-Meter finanziere eine ganze kopierte Region. Bereits old60-Clay braucht 1.232.620/1.226.688; kontrolliertes Full30 2.003.339/1.955.293, ohne vollständige Native-/Ownershipkosten. | §§3–7 definieren eine **separate** `ClosedCompositionOwner`-Authority, einen Core, eine immutable kumulative `RegionAllowance`, tatsächliches einmaliges Decoration- und kanonisches Featureseeding, vollständige Copy-/Query-/Cleanup-/Publicationabrechnung. Neue echte Full30- und fünf Biomplanläufe mit jedem Feature-Endzustand. | Separater Interfaceamendment- und Ceiling-/Nativeentscheid; keine Änderung von GEN-06.2. |
| CR63-B2, blocking | Entfernen beliebiger Markerblöcke beweise Preservation. Reviewer schreibt im echten `next(I)I` Originalslot1=0; Ergebnis 2.763.940→−160.476.802, alter Proof akzeptiert 126/134 Methoden. | §8 ersetzt diese Garantie durch exakte originalabgeleitete Templates mit geschütztem Local-/Stack-/CFG-/Throwablefluss. Die konkrete Reviewmutation wird tatsächlich erneut ausgeführt; acht weitere tatsächliche computational Mutantenklassen pro Profil werden geprüft. Alle legitimen elf Transformklassen bleiben positive Kontrollen. | Neuer unabhängiger Werkzeugreview; keine generelle JVM-/Allocationparität. |
| CR63-B3, high / Oracle-blockierend | Optionaler Census decke computational Payload ab. Branch3 true→false und Sphere0 `402efabc05b261b6`→0 werden akzeptiert; mit Census bleibt Sphere akzeptiert. | §9 verlangt ausdrücklich eine externe, vorher unabhängig gepinnte genuine Originalreferenz und vollständige Rawpayload-/Computationalmembershipgleichheit. CLI ohne Referenz verweigert. Beide Reviewmutanten plus Center/Y/Z/Radius/Cull/Branch/Core/Raw/Write/Mask/Release/Missing/Duplicate/Reorder werden konkret in beiden Profilen normal/−O angegriffen. | Neuer unabhängiger Oracle-/Referenzreview. Selbstgehashte neue Aufzeichnungen sind kein Expected. |

Keine vorherige Authorresponse wird erfunden. A enthält neue Quellen, tatsächliche Original-/Kontrollausgaben, Classmutanten, vollständige TSVmutanten und before/after-Pins. Alte Author-/Reviewerquellen, Helpers, JARs, Freezes und Fixtures bleiben bytegleich.

## 2. Readonly Authority und Provenanz

Einzige angenommene Runtimebasis: `.codex-tmp/gen05_11-runtime-root-base-storage47e/archive`, Main `47eadc4c14813c201184b552a87ca7b9c6f97db4`, 1.490 Dateien /324 Worlddateien; `base.freeze.json` SHA256 `58ba13d6418133bb49a154682ba1d82533f63a5e6184c6c2a8b07c6c15a47a75`. Diese komplette Membership und alle Bytes werden vor/nach geprüft. Shared World, Carver-WIP, Storage-/Sim-WIP und spätere docs/CI-Commits ersetzen sie nicht. Rustbuilds: null; Produktionsoverlay: leer. Root-CI38260056240 ist keine Oreprüfung.

| historisches readonly Objekt | SHA256 / Umfang |
|---|---|
| GEN-06.3 Originalvertrag | `49746a75ef24eb952ea73198741a6b80e262de8bec9da325cdf7e6dfa3ee758d` |
| Authorfreeze | `a919bc57099d1791f1021cf03aca9716fb90be5f4cb700c4fb2f824370252593`; 3.878 Dateien /599.591.972 B |
| Reviewbericht | `efdf02482cc4317014d7c4543bbee14fdcd0d5c8716c4672c6767f158b74d27e` |
| Reviewfreeze | `9462b5aa4cf8343d5fab7e4b950fbd60506196b13a847dbfb8d7100358ba7ef7`; 2.383 Dateien /2.773.901.576 B |
| GEN-06.1 angenommene Revision2 | `24ab2da586620009f25bfbfd99e433215d7e8698ec76a62eaff03ec03ec74eb8` |
| deren Author-/Reviewfreeze | `e44f7199a3f222007e9759a6fff69cf0ab030008e6adcbe003b8d03da81315a7` / `dd95a0be1baa4e176857a518978543d86427a2461e8cf796e00a365b1f6e63a4` |
| Vanilla 1.21.8 Inner | `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250` |
| Paper Build60 Inner | `c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507` |

Minecraft1.21.8/protocol772; Papercommit `29c8822d90899c89d2689338e81a98f690bcba12`; JDK21.0.12.1+1. A/input-freeze.json bindet die ganzen readonly Inputs des Reviews einschließlich seiner 8.415 externen Inputs, seiner 2.383 Files und der aktuellen akzeptierten Archivbasis; Überschneidungen werden dedupliziert. Vor Beginn: **12.291 einzigartige Dateien**. Vollständige Source-/Patchkopien gegen Origins, 894 Ressourcen gegen originale ZIPentries, 1.728 historische Inputpins, 454-Dateien-JDKmembership und die vier GEN-06.1-Freezes mit 1.147/164/366/494 Dateien bleiben geprüft. Zusätzliche tatsächlich gelesene pending-Kontexte/Pythoninputs stehen separat in `extra-inputs.json`.

Neue Toolkopien haben ganze Originpins und Deltas in `tools/copy-provenance.json` und `tool-deltas.json`. Agent/Observe/ProbeExtra sind zunächst exakte Kopien; die Computationaloriginale werden nicht umgeschrieben. Neue Helpers werden frisch kompiliert. `regions-*-runner.py.txt`, Javaquellen, Helperdiffs, javac/JAR/JVMlogs, CodeSourcebelege, Original-/Transformbytes und Proofreceipts sind vollständige neue Evidenz. Privat kopierte oder geladene Mutanten heißen ausdrücklich Mutanten.

H1 ist weiterhin ausschließlich die korrigierte historische Reviewbeschreibung: der unveränderte Helper enthält bereits `abs(block >> 4)`. H2 bleibt offen; Input+Candidate-States allein sind **7.077.888 B**. Keine alte4-MiB-Behauptung wird übernommen.

## 3. Tatsächlicher äußerer Originaldriver und neue kontrollierte Läufe

Vollständige Base/Applied-Methoden und relevante Patchschichten bestimmen den Vertrag: `ChunkGenerator.addVanillaDecorations/applyBiomeDecoration`, `WorldgenRandom.setDecorationSeed/setFeatureSeed/next`, `FeatureSorter`, Placement/Feature/Biomefilter, Ore/Bulk/Section und Heightmapcaller. Die komplette Originaltabelle enthält **164** Indizes; die Discoverypartition ist equals/hashCode, erst der Stepindexlookup Identity. hasFeature ist Equalitymitgliedschaft kanonischer Records.

Original: Origin=(16cx,minY,16cz); ein WorldgenRandom über einem Xoroshirocore für den äußeren Decorationlauf. `setDecorationSeed` setzt den Seed, zieht zwei originale Wrapperlongs = vier Coremutationen, macht beide `|1`, berechnet Java-long-wrapping `(bx*a+bz*b)^worldSeed`, setzt diesen Decorationseed und liefert ihn. Danach erhalten ausgewählte Features in numerischer Step-/Indexordnung jeweils `setFeatureSeed(deco,index,step)`: Java-long-Addition mit vorheriger Java-int-Multiplikation `10000*step`. Kein Carryover der Drawposition zwischen Featureseedzellen; vorherige Worldwrites bleiben sichtbar.

**Wrappercount wird durch setSeed nicht zurückgesetzt.** `next(bits)` inkrementiert den Java-int-Diagnosecount vor Delegation, mit originalem int-overflow. A/overflow-{normal,optimized}/ führt in beiden Profilen echte unveränderte Originalmethoden auf einem ausdrücklich injizierten Diagnosecount2147483647 aus: next24=2763940, Count danach−2147483648; setSeed und setFeatureSeed behalten ihn; setDecorationSeed zieht vier Wrappercores und endet bei−2147483644. Das ist eine kontrollierte Countgrenze, keine admitted Region oder Seedoverridefreigabe. Regiondiagnose führt außerdem einen checked erfolgreichen Corecount; er ist bei einer geworfenen Delegation nicht mit getCount identisch. Seeding/Featurewechsel resetten weder Diagnosechronologie noch Regionwork. Ein Regioncore darf nicht exportiert, geforkt oder geklont werden.

Original Paper: `featureSeeds.features` benutzt Configured-Holderidentität; default−1. Ein Override berechnet mit demselben Wrapper erneut `setDecorationSeed(configSeed,bx,bz)` und danach normales Featureseeding. Dessen zusätzliche vier Coremutationen und zwei Seedaktionen müssten kumulativ bezahlt werden; getCount läuft weiter. Erste geschlossene Productionadmission bleibt **Paperdefault-only ohne Overrides/Populators/Blending**, wie GEN-06.2. Ein Override benötigt eine weitere echte kontrollierte Ausführung und neue explizite Scopeannahme. Kein callergewählter Seedparameter an einer Pendinglease. Bukkit-Populators mit eigenem Legacywrapper und Chunk-x/z-Seeding liegen außerhalb dieser Komposition.

Original Steps0..10, Strukturen zuerst je Step, dann sortierte Vereinigungsindizes gespeicherter 3×3-Biome. underwater_magma6/26, disks6/29..32 und die nicht-Ore-Einträge vor infested7/4 bleiben kausal. A/regions-*/** führt **keine** dieser fehlenden Consumer als erfolgreichen Leerstep aus.

Frisch vorab deklariert je Profil/Pythonmodus, observed und ohne Agent/aktive Loggeraufnahme:

- Die gesamten **101** historischen controlled Originalfälle: old60 alle30×Seeds0/−1 plus41 konsequential Edges; originaler isolated Index0/Step6 bleibt deren erklärte Eingabe. Alle Rawevents und vollständigen States/Metadaten/Returns/RNGdiagnosen werden verlustlos mit der unabhängig zuvor gefrorenen Originalreferenz verglichen.
- Drei zusätzliche echte Wrapper-/Scriptcontrols mit vollständigem count/calls/remaining-Endzustand: Rejection2/2/[], Doublemax2/2/constant−1, Exception1/1/always-throws.
- **12 kumulative Ore-only-Regionen**: Full30, plains, badlands, dripstone_caves, lush_caves, windswept_hills, jeweils Seed0/−1. Jede Region beginnt mit neun echten defaultstatus-ProtoChunks/STONE, Gate320 und PP=null. Derselbe einzelne Core/Wrapper lebt durch alle Calls; Decorationseeding einmal, genuine Featureseeding je tatsächlicher globaler Zelle. Frühere Writes bleiben erhalten.

Full30 ist die kontrollierte Vereinigung aller30 mit passend ausgewähltem Originalbiomeholder je Plan. Die fünf anderen Fälle verwenden die komplette tatsächliche Ore-Mitgliedschaft eines einzigen echten Biomeholders, jeweils über die vollständige 164er-Tabelle ausgewählt: **25/26/25/26/27 Pläne**. Das ergibt **318 tatsächliche PlacedFeaturecalls und318 Feature-Endstates je Profil/Pythonmodus**. Bei den kontrollierten Snapshots bleiben die gespeicherten Biome plains; der Holderproxy wird explizit erklärt. Das sind vollständige genuine **Biom-Orepläne auf kontrolliertem Eingang**, keine natürlich erzeugten biomegerechten FEATURES-Regionen.

`feature-endstates.tsv` hält nach jedem Call Ordinal/Key/Step/Index/Holder/Return und sämtliche primitiven Corestatefelder plus kumulativen Javawrappercount fest. Diese diagnostischen Reads finden zwischen abgeschlossenen Calls statt und ziehen nichts. Originalcorewerte werden nicht aus Drawformeln berechnet. Der ganze Frame-/Coretrace, jeder Endstate und der finale State werden observed/control und normal/−O vollständig verglichen; `compare.py` prüft count=wirkliche Coreentries und genau eine Decorationseedinvocation. Der erste Feature-Endcount enthält die anfänglichen vier Coredraws; spätere zählen sie nicht erneut.

Neue Executionreceipts deklarieren diesen wirklichen Umfang. Der falsche geerbte Drei-Wrapper-Text des Review-Full30-Runners wird nicht übernommen. Alte Reviewoutputs bleiben unverändert und werden nicht als neue Feature-Endstateevidenz ausgegeben.

## 4. Separater ClosedCompositionOwner — exakter zusätzlicher Scopevertrag

**Diese Schnittstelle ist ein neuer privater Amendmentvorschlag. Die unveränderte GEN-06.2-Session gewährt sie nicht.** Öffentliche beziehungsweise zugelassene Single-key-API, `RunAllowance1..=2^20`, BindAllowance, BindingN869557/869258, sealed Pending, Fehler-/Drop-/forgetregeln und alle Nativeziele bleiben genau wie queued. Es gibt keinen Transfer/Upgrade einer aktiven Single-key-Session.

Zusätzlicher World-only Scope nach eigener Prüfung:

```rust
// Entwurf, nicht implementiert. Alle Typen/Konstruktoren privat/sealed.
struct ClosedCompositionOwner<'catalog> {
    // genau ein &Catalog, eine admitted Regionidentity, ein Core/Wrapper,
    // eine immutable RegionAllowance + kumulative RegionWork,
    // ein eingebetteter echter Cursorframe, Plan-/Candidatesequenz,
    // neun private Candidateowner und ihre precharged Cleanuppermits
}
enum CompositionState { Fresh, Validating, Copying, BetweenPlans,
                        RunningPlan, Pending, Completing, Complete, Poisoned }
struct CompositeAllowance; // new: checked 1..=4194304, PENDING REVIEW
struct CompositionPending<'run, 'catalog> {
    owner: Option<&'run mut ClosedCompositionOwner<'catalog>>
}
// admit(&Catalog, sealed Inputs, CompositeAllowance) -> Result<Owner>
// begin(&mut self) -> Result<()>  // einmalige Validation/Copy/Decorationseed
// enter_next_plan(&mut self) -> Result<()> // nur BetweenPlans, canonical cell
// next(&mut self) -> Result<Pending|PlanComplete> // echter lazy Primitivekernel
// Pending::dispatch(self, sealed OreConsumerKind) -> Result<InternalReceipt>
// finish(self) -> Result<ClosedRegionResult> // alle neun Owner, ein Marker
```

Kleinste private Wiederverwendung: die echten GEN-06.2-Modifier-/Wrapper-/Handoffoperationen erhalten intern einen **sealed borrowed Executionowner** mit genau zwei implementierungsinternen Varianten `Single(&mut SingleSession)` und `Composite(&mut ClosedCompositionOwner)`. Es gibt keine generische externe Charge-/Randomtraitauthority. `draw_bits`, Modifierentry, Emission, Dispatch, Completion und End verwenden dieselben Operationen/Refusalstellen; der Borrow leitet `charge` immer zum einzigen echten Owner und dessen Meter. Der Primitivecursorframe erhält keinen eigenen Meter/Core. Standalone-Aufrufe benutzen ausschließlich Single; kein Compositeconstructor ist über die Single-key-API erreichbar.

Erforderliche konkrete private Deltas: (1) primitive Cursorstate von seinem Single-owner-Borrow trennen, ohne zweite Cursorimplementation; (2) interne RNG-/Chargecallee auf den sealed Ownerborrow ausweiten; (3) interne Pendingstorage/Dispatchreceipt mit Owner-/Katalog-/Plan-/Candidatesequenz für beide Owner variieren; (4) **nur** Composite-BetweenPlans einen kanonischen `enter_next_plan`-Seedingcallee anbieten. Derselbe Completion-/Option::take-/Dropguard bleibt bis nach allen falliblen/panikfähigen Aktionen armed. Diese vier Deltas verlangen eine separat geprüfte Änderung des GEN-06.2-Privatinterfaces; sie sind kein bereits genehmigtes Implementationdetail.

Eine andere Verbindung ist ausgeschlossen: keine frische Single-session pro Key, keine exportierte RNGkopie, Cursorfork, frei vorgeschaltete Copy, ersetzte Primitive, privates Gegenbudget, uncharged Constructor/Reseed/Consumer oder Reset. Der Planframe kann zwischen vollständig geschlossenen Plänen in seinen initialen Count-/Iteratorzustand wechseln; Regionused/Core/Wrapperdiagnose/Identity bleiben derselbe Owner. Planwechsel ist kein Wiederaufleben eines Poisoned/Pendingowners. Auswahl erfolgt aus sealed genuine Membership plus allen164 kanonischen Records, niemals ore-only-Sorter oder Callerindex.

Pending hält exklusiv `&mut Owner`; zweites next/Planwechsel ist während Borrow unmöglich. Interner Dispatch nimmt reale Stored-Biomentscheidung und danach echten Oreconsumer mit demselben exklusiven Core/Meter vor. Biomefalse ist ein tatsächliches geprüftes Ergebnis, kein Caller-bool. Receipt ist nur intern nach erfolgreichem Completiondebit erzeugbar. Consumerfailure/Drop/Unwind/Completionrefusal sind terminal. Forgotten Pending bleibt Pending; nächster Aufruf poisoniert/refusiert ohne RNG-/Workänderung. Nicht abgeschlossene Owner veröffentlichen nichts.

Der ganze Run hält eine immutable Allowance. Failed charge verändert weder used noch den verweigerten Core/State/Counter/Write/Publicationvorgang. Vorher erfolgreich admitted Prefixe bleiben allein diagnostisch. Cleanuppermits sind vorab kumulativ **ausgegeben**, ownergebunden, nicht clonebar, nicht rückbuchbar und keine zweite Allowance. Der Resultowner übernimmt dieselben Candidatebuffer und die precharged letzten Destruktorpflichten; nach finaler Publication ist keine fallible Arbeit übrig. Resultdrop installiert nichts.

## 5. Vollständiger neuer Workentwurf und praktische Feasibility

Einheit: ein explizit begrenzter skalarer Group ohne datenabhängigen Innenloop (höchstens64 skalare Operationen); jeder tatsächliche Headtest inklusive Endtest, Store, Query, Entscheidung, Lookup, Setter-, acquire/release-/Reserve-/Copy-/Publicationlayer separat wie unten. NativeLibrarycalls mit variablem Innenloop sind nicht solche Groups. Befund ist ein **Originalevent-abgeleitetes Policymodell**, kein ausgeführter Rustmeter, keine CPU-/Native-/TPSmessung.

Die vollständigen bisherigen Sphere-/Cull-/Raster-/Target-/Air-/Mask-/Guard-/Bulk-/Section-/Coredebits bleiben erhalten, einschließlich aller rejected bounded-int-Coremutationen und PaperDouble1. Tatsächliche Branchordinale stammen aus originalem Bytecode, profilgenau. Initiale Spherearray-, Mask- und Handle-Stores/Heads werden zusätzlich gezählt; jedes Spherebodygroup und jeder tatsächliche native Settergroup kostet1. Airloopheads/predicates und release-Endheads werden nicht in einem kostenlosen Call versteckt. Emission/Dispatch/Completion/End bleiben echte Primitivekosten. `work_model.py` benutzt den unabhängig geschriebenen Reviewcensus mit ausdrücklich einzig geändertem neuen Caseprefix und enumeriert alle zusätzlichen Deltas.

**Explizite alte→neue Proposaländerung:** sechs Maps werden als echte nonspanning 37-u64-Mapsets kopiert, nicht als sechs ungepackte256-u16-Tabellen. Producer muss sie bereits echt validiert/versiegelt liefern; keine kostenlos nachträglich geparsten Rawbytes. Bestehende angenommene Flat-/Surfacecaps ändern sich nicht. Beim neuen Handoff werden keine WG-Maps aus Endstates rekonstruiert.

| Copyteil, einmal je ganzer Region | Units |
|---|---:|
| neun98.304-Statebuffer: paired read+store **und jeweils eigener Head**, neun Endheads | 1.769.481 |
| neun×sechs×37 genuine packed words: Body+Head plus54 Endheads | 4.050 |
| neun×1.536 Fluidwörter: Body+Head plus9 Endheads | 27.657 |
| 216 Counterrecords: Body+Head plus9 Endheads | 441 |
| leere PP-Abschnittheads | 216 |
| **neue vollständige Copy** | **1.801.845** |
| aggregate PP bis131.072 erhaltene Records: Body+Head zusätzlich | **262.144** |
| konstante Validationreserve vor Inhaltsloops | **2.048** |

Hier wird die geerbte Copyformel ausdrücklich zusätzlich korrigiert: ein paired Read+Store bleibt Body1, sein nichtterminaler Loophead kostet ebenfalls1. Die alte Copyregel hatte trotz allgemeiner Headpflicht lediglich die Endheads addiert. Der finale neue Copycensus ist deshalb gegenüber dem ersten privaten901071-Modell um900774 erhöht; maximal PP kostet zusätzlich262144 statt131072. `work-normal.json`/`work-optimized.json` sind nur **vorläufige, ersetzte Modelle** und bleiben als Diagnose erhalten. Ausschließlich `work-final-normal.json`/`work-final-optimized.json` gelten als neue Autorenrechnung. Weder der alte Vertrag noch sein Reviewfreeze werden geändert.

Ein safe Rust-Vec kann nach fallible Vorreservation PODwerte einzeln pushen; die paid paired Copy schreibt dadurch unmittelbar in admitted Capacity. Keine uninitialisierte unsafe Arena, zusätzliche kostenlose Zero-fill+Copy oder unfallible shrink-/Boxkonversion. Falls das konkrete Layout initialisiert **und** kopiert, sind beide wirklichen Schleifen zusätzlich zu bezahlen. Der Result behält dieselben Vecs/Capacities; kein dritter voller Payload.

**Heightquery konkret:** der pending GEN-03.1-Worldentwurf schlägt einen vollständig checked Heightgetter mit **8 Units** vor. Jede echte Ore-Gatequery charged zusätzlich zu ihrem Regionadaptergroup1 diesen gesamten8-Reserveblock am **Regionmeter**. Kein Heightmap-Standalone-Meter wird innerhalb der aktiven Komposition konstruiert. Ein späterer privater Amendmentcallee `sealed_height_query(..., &mut CompositeExecutionBorrow)` muss denselben tatsächlichen Getterkern mit dieser Parentcharge benutzen; gecharge wird vor dem Call. Das ist eine weitere separat zu prüfende Schnittstellenvoraussetzung, keine Authority aus der laufenden Heightmapreview. Fehlt angenommener Producer/Getter, verweigert Publicadmission. Der kontrollierte Gate320-Proxy bleibt allein Originalcontrolinput.

Stored-Biomquery verwendet den angenommenen stateless Zoom/Storedlookup mit acht separat admitted fixed Cornergroups (inklusive ihrer acht LCGsteps/Distance-/first-minimum-Arithmetik); Biomefilterentry/membership und Lookup behalten eigene Debits. Vollständige gathered-biome/membership-Vorbereitung hat eine separate feste obere Reservation: ein flatten Head+Body je höchstens18.432 gespeicherten Werten plus Endhead. Body liest einen checked Biomewert und ORt die drei festen164-Bit-Membershipwörter; keine Stringhash-/RTree-/Parserarbeit. Jeder später tatsächlich zusätzliche Loop muss neu abgebildet werden.

| zusätzliche einmalige vorab ausgegebene Reserve/Workphase | Units |
|---|---:|
| vollständiger Owner-/Sidecar-/Entity-Dropcleanup | 524.288 |
| native Allocation-/Reserve-/Releasegroups | 512 |
| bis128KiB neue Headerinitialisierung, u64 Stores+Heads | 32.769 |
| erste gesamte8KiB-Geometryinitialisierung, u64 Stores+Heads | 2.049 |
| alle164 Recordbodies/Heads und11 Stepbodies/Heads | 352 |
| Plan-/Identity-/Owner-Metadatabounds | 1.024 |
| gathered Storedbiome/Membership-Vorbereitung | 36.865 |
| Result-/Versions-/Completion-/Publicationmetadaten | 256 |
| echte Core-/Wrapperconstructor-Groups | 4 |
| **Summe** | **598.119** |

Die1024-Metadatenreserve umfasst zusätzlich die realen äußeren Copyloops: zehn Chunkheads,63 Mapheads,225 PP-Sectionheads, bis432 konstante Chunk-/Owner-/Headergroups (48 je Chunk), bis240 Planidentitygroups (8 je30 Plänen),54 weitere konstante Abschlussgroups. Summe1024; die inneren positiven/Endheads und alle Inhaltsschritte stehen bereits separat in der Copytabelle. Keine PP-/Map-/Chunk-Controlschleife wird kostenlos verborgen. Dieses konkrete Layout ist eine künftige Runtimebeweispflicht, nicht angenommener Producerbestand.

Native Cleanup muss durch einen tatsächlichen versiegelten Producerdescriptor bewiesen sein: höchstens131.072 destruktorrelevante Nodes, Depth≤32 und höchstens4 feste Group/Headaktionen je Node; sämtliche Entitystrings/NBTcontainer/Index-/Sidecar-/Ownerdrops zusammen ≤524.288 Units. Höchstens512 konstante fallible Reserve-/Deallocation-/Owneraktionen sind separat admitted. **Dies sind neue zusätzliche Regionadmissionvorschläge**, keine vorhandenen Producerfähigkeiten und keine Änderung encoded Storagecaps. Ohne echten Descriptor verweigern; kein kostenloser rekursiver Drop. POD-Vecs brauchen keine elementweisen Destruktorloops, ihre tatsächlichen Deallocations zählen trotzdem. Zusätzliche variable Cleanup-/Reservearbeit, die diese Vorschläge verfehlt, benötigt eine konkret bezifferte neue Rootkorrektur vor Runtimefreeze.

Regionbegin/Initialseed/Decorationreseed kosten zusammen3 administrative Units, die vier initialen Coremutationen stehen bereits im wirklichen Eventcensus. Je Plan Begin/Featureseed/End zusammen3; übrige Modifier-/Dispatch-/Completionkosten bleiben dynamisch. Anders als Review-Full30 werden anfängliches Decorationseeding und seine vier Cores nicht30-mal verlangt. Das ändert die Originalfeaturebits nicht; die neue genuine gemeinsame Wrapperchronologie zeigt es ausdrücklich.

Vollständige Vorschlagsrechnung: tatsächlicher vollständiger Kernelcensus mit neuer Administration +1801845 Copy +2048 Validation +598119 Zusatzreserven +8 je Heightquery; maximal PP zusätzlich262144. Jede Reservation bleibt ausgegeben, auch wenn unbenutzt. Keine Workrückzahlung beim Planwechsel oder Cleanup.

| kompletter controlled Originalplan | Pläne | Attempts Seed0/−1 | Gesamt, leere PP, Seed0/−1 | Gegenrechnung max PP, Seed0/−1 |
|---|---:|---|---|---|
| Full30 Union | 30 | 433 /434 | 3.504.729 /3.455.395 | **3.766.873 /3.717.539** |
| plains | 25 | 233 /234 | 3.077.920 /3.028.800 | 3.340.064 /3.290.944 |
| badlands | 26 | 283 /284 | 3.117.283 /3.067.833 | 3.379.427 /3.329.977 |
| dripstone_caves | 25 | 233 /234 | 3.104.825 /3.054.739 | 3.366.969 /3.316.883 |
| lush_caves | 26 | 279 /280 | 3.386.563 /3.337.736 | 3.648.707 /3.599.880 |
| windswept_hills | 27 | 321 /322 | 3.116.007 /3.066.701 | 3.378.151 /3.328.845 |

Beide Originalprofile haben in diesem Corpus dieselben Workmodellsummen; Rawbits/Originalevents bleiben profilgetrennt. Alle Originalcalls schließen wirklich ab; **Completion bei vorgeschlagener Allowance ist eine überprüfte Modellrechnung über diesen kompletten Eventpfad mit expliziten Zusatzreserven**, keine Rustexecution. PP131072 ist hier ausschließlich Gegenrechnung, nicht ausgeführter solcher Originalinput. Native capacities und nichtleere Entities/Maps bleiben eigene Runtime-/Producergates.

Kleinste numerisch useful Allowance für den erklärten kompletten Vorschlagscensus einschließlich maximaler PP-Gegenrechnung: **3.766.873**. Das ist keine allgemein minimale Implementierungsarbeit. `2^21` unterschreitet bereits diesen Entwurf um1.669.721. Empfehlung: neues `CompositeAllowance1..=2^22=4.194.304`, gewöhnliche erklärte Regionen mit max allowance; Margin zum größten hier kompletten Modell **427.431**. Kein Ceiling über dem Root-Ausgangspunkt wird benötigt oder genehmigt. GEN-06.2 bleibt1Mi. Weitere Seeds mit dynamischen Rejections oder andere admitted Inhalte können bounded `RegionWorkExceeded` liefern; kein all-seed-Erfolgsversprechen. Die Laufzeit muss gerade useful ganze Regionen, nicht allein einen erfolgreichen Attempt, prüfen.

Fehlerpräzedenz: invalid/terminal Owner/Pending → Katalog/Owner/Key/Plan-/Candidatesequenz → konstante Profil/Seed/Settings/Source/Geometry/Center/Allowanceheader → Validation-/Cleanup-/Reserveadmission → fehlende/duplizierte/fremde Neighbor → tatsächlicher Producerstatus/Maps/Classification/StoredBiome/Membership → vollständiger Versionsvektor → native capacities/Dropdescriptor → nächste Workadmission → fallible Reserve → Inhaltscopy → kanonischer Plan-/Seedwechsel → lazy Biome/Gate/Kernel → Completion/End → letzte ganze Regionpublication. Workexhaustion steht vor der verweigerten nächsten Aktion, nicht rückwirkend vor einem schon gefundenen Fehler. Fehler im admitted Run poisonieren; keine Präfixpublication. Konstant bounded Failclosed-Statebits/Borrowrelease brauchen auch bei vollständig verbrauchter Allowance kein neues Budget. Variable Cleanup ist bereits reserviert.

## 6. Gesamtes simultanes Nativeziel und Ownerleben

**16.777.216 B bleibt allein pending Regionziel.** Alle Inputs/borrowed Capacities, Candidates, volle Entity-/Tickindizes, Stack/Temps und Header zählen gleichzeitig; shared immutable Allocations einmal, sämtliche eigenen Referenz-/Ownerheader separat. Keine RSS-, Allocator- oder measured-native-Behauptung.

| Bestandteil | neuer vorgeschlagener Betrag B |
|---|---:|
| neun Input- plus neun Candidate-Statepayloads | 7.077.888 |
| Input+Candidate sechs packed Maps je Chunk | 31.968 |
| Input+Candidate Fluidsets | 221.184 |
| Input+Candidate Counter/Flags | 6.912 |
| vollständige immutable Tick-valuecapacities | 1.769.472 |
| Input+Candidate PP-capacities, je aggregate131072×u32 | 1.048.576 |
| geliehene Storedbiomecapacities/Borrowheader | 18.936 |
| Zoom / Generationmembership | 512 /4.096 |
| echte Stateklassifikation, konservatives Borrowziel | 223.568 |
| Paper57-Sourcebytes / retained Katalog / Cursor+Core+Meterziel | 26.987 /16.384 /512 |
| gesamte Geometry einschließlich Handles/Locals/Stack/Alignment | 8.192 |
| gesamte native Entity-/pendingEntitycapacities inklusive Indizes | 2.097.152 |
| **zusätzlich vollständig native Tick-Reverse-/Hashindizes** | **2.097.152** |
| alle übrigen Owner-/Buffer-/Map-/PP-/Index-/Result-/Reservationheader | 131.072 |
| übriger Stack, Error-/Admission-/Returntemporaries, Alignment | 262.144 |
| **gesamt / Margin bis16MiB** | **15.042.707 /1.734.509** |

Explizite old→new Änderungen: ungepackte Mapzeile55296→31968 B; **zusätzliche** Tickindexzeile2097152 B. Frühere subtotal12968883→15042707 B. Keine genehmigte Cap wird geändert; auch die PP-/Entity-/Tickindex-/Cleanupzulassungen sind neue Proposals. GEN-03.1s pending2KiB-Mapset/512B-Scratchziele gewähren hier keine Runtimeauthority; tatsächliche Mapsetheader/Scratch gehören zu den vorhandenen Regionheader-/Stackpartitionen.

Paper-ShortList besitzt `byIndex` **und** Short2ShortOpenHashMap. Ein Borrow der Valuefolge beseitigt nicht den gleichzeitig lebenden Reverse-/Hashindex. Alle tatsächlichen capacities werden erhoben; Original full4096-Values in216 Sections können allein für Hash-key/valuearrays bei n8192+1 etwa **7.078.752 B** verlangen. Dieser Shape würde das neue2MiB-Indexadmissionziel verweigern; er wird nicht als 16MiB-feasible ausgegeben. Kein Truncation und kein Encodedbytes-Ersatz. Producer muss das tatsächliche native Inventar anbieten. Der geschlossene Ore-/Tagstatebeweis erlaubt immutable Tick-/Specialcollisionmembership, aber keinen ungezählten Index oder einen resultlebenslang ungültigen Borrow.

Sidecarowner werden echt verschoben oder als bereits admitted immutable Refcountowner übernommen. Alle gleichzeitig lebenden Input-/Resultreferences samt Indexpayloads zählen weiter. Result nimmt dieselben neun Candidate-State-/Map-/Fluid-/Counter-/PPbuffer; alte Chunkfreigabe darf seine Sidecars nicht zerstören. Letzter Refcountdrop hat einen echten vorreservierten Cleanupdescriptor. Keine neue Entität wird über ein kostenloses tiefes Clone/NBTdecode hergestellt. Shared World-BTreeMap-/Palettelayout ist keine accepted regionfähige Darstellung; tatsächlicher Ownerhandoff und Dropcap bleiben ein eigenes Producerticket.

8192 Geometry umfasst2048 Sphärendoubles,1184 Maskbytes,864 B für27 feste32-B-Handles und4096 B Headers/Locals/Stack/Temporaries/Alignment. Kein zusätzlich volles8KiB-Arenaobjekt plus externe Locals. 27 wird für alle admitted Origins/vertikalen Slots bewiesen, nicht aus arbitrary direct-doPlace-Controls generalisiert. size≤64, Radius≤4.5 inklusive Paper1.0, Mask26×14×26=9464,148 Wörter und2016 Paare bleiben strukturelle Bounds. Sämtliche Java-i32/i64-Wraps, f32-Rundungen/Operationreihenfolge und Originalstrictness bleiben erhalten.

## 7. Erhaltene Algorithmus-, Producer- und Adoptionpflichten

Originalkernel bleibt lazy: Count/Rarity → X vor Z → Height → unentschiedene Biomfortsetzung → vollständiger Consumer → nächster Candidate. Feature.guard steht vor Angle/Heightdraws. Gate fragt OCEAN_FLOOR_WG als firstAvailable (Chunk-highestTaken+1) in X→Z bis erstem Erfolg. Alle size-Doubles und Sphere-DASTOREs vor Culling/Writes; f32-i/size vor f64-Lerp; striktes Containment, Ties erhalten, Cullradius−1. Sphere→X→Y→Z; buildheight vor Masklookup; Mark vor guardfalse; eigene Maske je Attempt. Targets originalgeordnet; mismatch ohne Draw; chance0/1 ohne Draw, zwischen0/1 skip bei float≥chance; DOWN/UP/NORTH/SOUTH/WEST/EAST mit erstem Airshortcircuit, Wasser keine Luft. Gleiches Target kann tatsächlichen Write/true ohne Nettodelta liefern. Bulk releases wirklich erworbene Sections auch beim Throw; tatsächliche Iteratorreleaseordnung bleibt im Trace.

Direkte Sectionwrites erfinden keine WorldGenRegion-/ProtoChunksetter-Nebenwirkungen. Vanilla fluid/nonempty und Paper random-ticking-fluid-/Specialcollision-/Tickmembership-Regeln bleiben profilgenau; identischer State überspringt bei Paper nur den Callback. Heightmap-, POI-, Entity-/PP-Neuberechnung wird nicht ergänzt. Echte Classification einschließlich aller zugelassenen Eingangstates muss vorher independently producergeprüft sein.

Public Regionauthority fehlt bis angenommen: neun echte CARVERSowner; WG-/finale Heightmaps mit tatsächlicher vollständiger State-/Fluid-/Leavesclassification; stored featurebiomes/Zoom/full Generationmembership; volle Neighbor-/Stage-/Versionsauthority; vorherige Structures/Decorations und non-ore Interleaving. Concurrent Carver und pending Heightmapauthor/-review sind keine angenommenen Producer. Defaultstatus-STONE, Gate320 und chosen Holder ersetzen diese Felder nicht.

Whole-FEATURES bleibt ein eigenes Gate mit tatsächlich gestartetem oder gleichwertig vollständig initialisiertem Original-ServerLevel; originalen Vorstages/Structures/alle11 Steps und configs/packs/WorldOptions/seeds/featureSeedPolicy/Blending, radius8 STRUCTURE_STARTS/radius1 CARVERS, echten Holdercompletions sowie vollständigen before/after State/Core/Section/Map/Counter/Entity/PP/BorderTickdaten und unbeobachteter Kontrollgeneration. Keine allocateInstance-ServerLevelattrappe, Statussetter oder erfolgreich leere Consumer. Forward/reverse Neighborcenterhistory bleibt separat.

Regionresult wird einmal vollständig veröffentlicht; späte Refusal verwirft alle neun Candidates. Sim muss später alle neun Owner über positive/negative Shardnähte reservieren, ganzen Versionsvektor erneut prüfen und atomar an Tickgrenze installieren. Resultdrop ohne Adoption hat keine Weltwirkung. Kein Sim-/Storage-/Appconsumer in dieser Lieferung.

## 8. Neuer exact instrumentation-/Noninterferenceproof

`tools/ProofVerify.java` importiert **keinen Agent**, löscht **keinen** Markerblock und benutzt **keine** Transformreceipt als computational Authority. Es liest jede tatsächliche Originalklasse und verlangt Bytegleichheit zum originalen gepinnten ZIPentry. Aliase/Selection/semantische Labels entstehen aus originalen Symbols/Descriptors. Unselected Methoden, ursprüngliche Felder/Shape und alle ausführbaren Originalinstruktionen werden unverändert verglichen. Selected Methoden erhalten einen unabhängig formulierten endlichen Templatesatz; das vollständige erwartete executable Programm wird gegen das tatsächlich transformierte verglichen.

Der Vergleich umfasst Opcodes, originale Slots, Constants/Typen, Field-/Methodowner/names/descriptors/interfaceflags, InvokeDynamicbootstrap/args, Switchkeys/-ziele, Branchprädikate/-ziele und **sämtliche** ursprünglichen/zusätzlichen Exceptionedges. Debuglines/Frames werden nur als nichtcomputational Metadaten aus der Gleichheit ausgeschlossen; sie sind in ganzen Classbytes gepinnt und werden tatsächlich von der JVM geladen/verifiziert. Originale Instruktionsordinale benutzen alle originalen Nodes einschließlich Debugnodes, wie die ausgeführte Instrumentierung.

Templatebeweis je Methode mit Original-maxLocals L:

| Template | geschützter Daten-/Stackfluss |
|---|---|
| Entry | ausschließlich originale Argument/Receiverloads; genau neues Object[] mit exakt descriptorpassenden Boxingcalls; Arraywrites nur in dieser neuen Arrayreferenz. Observe.enter mit exakter Signatur; frame-LSTORE nach SlotL/L+1. Kein Originalslotstore. Operandstackprefix bleibt erhalten. |
| DASTORE-before/-after | ursprünglicher `(array,index,double)`-Stack wird in neue SlotsL+2/L+4/L+5 gespillt und in derselben Reihenfolge/Identität wieder geladen. Der **originale** DASTORE bleibt an gleicher computational Stelle. Observe.store lädt exakt dieselben Index/Doubletemps und Frame; kein originaler Store wird ersetzt. |
| BitSet get/set | ursprüngliche Setreferenz/Index disjoint spill/reload; echter Originalcall bleibt. get-Resultat wird ausschließlich in neues int-Temp gespeichert, genau dieses Resultat boxed/beobachtet und danach unmittelbar mit ILOAD aus demselben Temp für den Originalcaller geladen. set beobachtet den festgelegten `marked`-Token. |
| Return | originales typed Resultat wird disjoint gespillt, exakt beobachtet und unverändert für ursprünglichen typed Return geladen. void erzeugt nur null. Boxingowners und Descriptors sind vollständig festgelegt. |
| Conditionaltrampoline | ursprüngliches Predicate bleibt exakt. True/falseziel führt jeweils nur zum richtigen Observe.branch(ordinal,taken), dann zu ursprünglichem Ziel beziehungsweise Fallthrough. Vollständiger CFGvergleich erlaubt keinen dritten Branch/Targetwechsel. |
| Catch/rethrow | zusätzliche letzte Throwablehandleredge schützt genau den originalen Methodenbereich nach Entry; ursprüngliche Handlerpriorität bleibt erhalten. ASTORE in neues Temp; Observe.exit bekommt **dieselbe** Throwableidentität; exakt ALOAD dieses Temps/ATHROW. Kein return/suppress/replace. |

Jeder originale Localzugriff einschließlich wide L/D und IINC wird mechanisch gegen originales L geprüft. Alle neuen Slotreferenzen/Loads/Stores werden durch den exakten ganzen Templatevergleich nachgewiesen; Marker sind selbst lediglich erwartete Literale/POPs und keine Erlaubnis. Originalobjekte werden nicht durch neue Fieldwrites oder computational Calls verändert. `Observe` ist ebenfalls whole-file gepinnt und gelesen; es ruft keine RNG-/Worldmethoden. Loggingfehler/OOM/IO/Transformfailure werden nicht als successful Originalcontrol akzeptiert. Noninterference gilt für die vollständig erfolgreichen beobachteten Runs, mit tatsächlicher unbeobachteter Vergleichsausführung; keine Ressourcen-/Exceptionparität bei versagender Instrumentierungsumgebung.

Positive ganze Transformklassen: je normal/−O **126 Vanilla-/134 Paper-Methoden**, davon36 selected je Profil. Alle elf tatsächlich geladenen Original-/Transformklassensätze sind ZIP-/CodeSource-/Membershipgeprüft. Der neue Proof besteht bei allen legitimen Stores, Returns, Boxing- und Throwabletemplates, statt dafür nötige Opcodes pauschal zu verbieten.

A/inverse-complete-{normal,optimized}/ reproduziert zuerst exakt die readonly Reviewmutation: genuine Originalnext24=2763940, genuine Mutantnext24=−160476802; alter StripVerify akzeptiert weiterhin126/134, **neuer Proof verweigert**. Zusätzlich acht tatsächlich JVM-ausgeführte Mutanten je Profil: Originalslotstore, ursprüngliches setSeed-Computationalcall, count-Fieldwrite, Returnoperandload aus falschem Slot, ursprünglicher Branchpredicatewechsel, Branch/GOTOzielwechsel im Entrytemplate, ersetzte Throwableidentität und geschluckte Exception. Jede Mutation ändert wirklich Return/count/Exceptionidentität und wird verweigert. Die echte Corethrowkontrolle benutzt denselben Sentinelthrowable; Original gibt ihn identisch weiter. Keine ungültige JVMklasse wird als consequential computational Mutation gezählt.

Frühere private Entwicklungsversuche mit einem allein observerseitigen Returnload und einer unverifizierbaren Jumpklasse sind ausdrücklich **fehlgeschlagene Entwicklung**, keine Negativabnahme. Ihre Logs/Bytes bleiben im neuen vollständigen Freeze, als solche klassifiziert. Final gelten ausschließlich inverse-complete-* mit tatsächlichen Original-/Mutantausführungen und positiven ganzen Proofs.

## 9. Verpflichtende vollständige Payloadreferenz statt optionalem Census

`tools/validate.py` bleibt als **structural checker** getrennt: Frames/Kinds/Header, vollständige States, Read-/Write-Replay, lifetimes, masks, cores/rejections/float/double, existing completenesschecks. Seine optionalen Censuschecks werden nicht als vollständiger computational Bitbeweis ausgegeben.

`tools-extra/lossless.py` ist die neue zwingende Ebene: `--reference` ist required. Die Referenzdeklaration verlangt genaue externe Freezeidentity, vollständige Membership und jede erwartete SHA256/Bytezahl aus diesem bereits unabhängig gepinnten Freeze. Sie kann die Candidateaufzeichnung nicht zum Expected erklären. Referenz `reference-{vanilla,paper}.json` bindet den echten historischen101-Originalcorpus im unveränderten Authorfreeze; dieser wurde vom unabhängigen Review whole-byte und in43 focused genuine Calls zusätzlich reproduziert und vor diesen neuen Calls nochmals komplett gepinnt. Erwartungserzeugung ruft weder FerrumC noch einen kopierten Ore-/RNG-/Spherealgorithmus auf.

Jede neue101-Aufzeichnung wird vollständig Rawbyte/zeilengetreu gegen diese **andere, vorher gefrorene Originalausführung** verglichen. Keine Sortierung, Deduplizierung, Bit-/Sign-/Identity-/Exception-/Ordernormalisierung. Gzip ist nur ein reversibler Container. Zusätzlich müssen sämtliche Computationaldateien mit voller Membership und Bytes gleich sein: alle neun before/after State-/Metadatasnapshots, Stateinventory und gesamte Results/Exceptions/RNGdiagnosen. Der strukturale Checker bleibt anschließend eigenständig ausgeführt; er kann die zwingende Referenzgleichheit nicht optional machen. Ein Count-/Branchcensus genügt nicht.

Die neuen regionalen Payloads und Perfeatureendstates werden separat mit fresh independently compiled unobserved Controls und dem zweiten normal/−O Originalrun **verlustlos** verglichen, nach exact Templateproofs; sie sind neue genuine Evidenz, keine durch ihren eigenen Hash bewiesene Oracle. Für künftig konsumierte regionale Fixtures muss unabhängiger Review diese neuen Originalreferences pinnen, bevor die normale Standalone-Reference CLI sie als Expected verwenden darf. Der aktuelle externe CLIreferencepfad lässt nur bereits unabhängig gefrorenen101-Corpus zu.

Neue tatsächliche TSVmutationen: **17 je Profil/Pythonmodus**, insgesamt34 Refusals je Pythonmodus. Beide Reviewzeilen (Branch3 und Sphere0) werden exakt getroffen; zusätzliche erste CenterY/CenterZ/Radius/Cullstorebits, Orebranchrichtung, signed/Corebit, rawf32/rawf64, Setterstate, Maskwert, Releaseidentity, Missing/Duplicate/Reorder. Vollständige mutierte gzip-Streams und genaue Before/Afterzeilen bleiben eigene Fixtures. Der letzte verschärfte Prüfer wird in `lossless-final-{normal,optimized}/results.json` nochmals mit exakt gepinnter finaler Werkzeugquelle geprüft:34 vorhandene vollständige Mutanten, vier echte positive Equality-/CLIcontrols, zehn Refusals für self-minted Freeze/falschen Authorpin/fehlenden Reviewpin/Candidatereference/unvollständige Membership sowie zwei CLIrefusals ohneExpected je Pythonmodus. Frühere payload-*/results.json dokumentieren die Erstellung; nur die finalen Losslessreceipts belegen diese letzte Prüferfassung. Das alte Sphere-accept-with-original-census wird mit readonly Reviewerfixture und echtem positivem Census frisch reproduziert; kein alter Input wird geändert.

Die Pflicht gilt für **alle** Branch/Sphere/raw/core/state/order/lifetime-Payloads, auch jenseits der einzeln angegriffenen Felder, weil alle Rawbytes einer anderen gepinnten Originalexecution verglichen werden. Sie beweist diese erklärten Inputs; arbitrary neue Inputs benötigen zuerst ihre eigene independently geprüfte Originalreferenz. Neue selbstdeklarierte Counter-/Hashlisten ersetzen das nicht.

## 10. Tatsächliche neue Checks und zukünftige Runtimepflichten

Fresh observed/control normal/−O und beide Profile: volle101, dreiWrappercontrols und zwölf kumulative Regionen. Whole-Class/ZIP-/CodeSourceproofs und exakte normale/−O Class-/Fixturebytes; vollständige Computationalmembership und before/after-Gleichheit. Die41 Semantikedges bleiben geprüft, einschließlich echter Consumerexception/release, Targetequal/zero-delta, beide Targets, sechs Luftshortcircuits/Wasser, chance0/1/equality, Gates, Cull/tie, Maskmark trotz guardfalse, alle-size-Doubles, buildheight und positive/negative Chunk-/Shardnähte. Snapshotordnung wird weiterhin nicht als Writeordnung ausgegeben.

Guardreihe je normal/−O: fünf reale JVMenv-Entrypointrefusals, sechs tatsächliche fresh-helper-JVM-CodeSourcefaults (Original/Helper/Library in beiden Profilen), echte private Bytemutation und derived-ZIPshadow, JDK-/Helpermembershipfaults sowie mandatory-reference-CLIrefusal; **16 Refusals/3 positive Controls**. Die historischen Fresh-/Scopesperren bleiben in echten reused Runnern erhalten. Compile-/Run-/Nachpinfehler sind keine erfolgreichen Samples.

A/validation-*.json, *-semantics.json, payload-*/results.json, **lossless-final-*/results.json**, inverse-complete-*/results.json, guards-*/results.json, overflow-*/results.json, **comparison-final-{normal,optimized}.json** und **work-final-{normal,optimized}.json** sind die maßgeblichen nachprüfbaren Receipts. Der finale Vergleich prüft zusätzlich jede der22 actual Original-/Transformklassendateien je Profil/Runfamilie normal/−O bytegleich. Absolute Pfade oder JVMtimestamps in Diagnosefiles sind einzeln gepinnt; keine computational Payload wird dafür normalisiert. Vorläufige Workmodelle und frühere Losslessprüferfassungen sind ausdrücklich keine finalen Abnahmereceipts. Modellwerte, genuine Originalcalls und künftige Rust-/Nativepflichten sind getrennt.

Vor irgendeiner Runtimeabnahme: wirklicher GEN-06.2-Kern plus separat reviewed Interfaceamendment; echte N−1/N/N+1 an sämtlichen Region-/Plan-/Seed-/Core-Rejection-/Emission-/Biome-/Gate-/Sphere-/Cull-/Mask-/Read-/Write-/Reserve-/Cleanup-/Completion-/Publicationgrenzen. Allowance0/1/2^22/2^22+1, old Single1Mi unverändert; refused Core/used/data-/Ownerbytes unverändert; terminal Drop/failure/unwind/forget; exklusive Compileborrow-/Movecontrols und keine RNG-/Meter-/Seedinjektion. Vollständige useful Full30-/Biomplancompletion auf diesen echten nativen Modellen mit ganzer Regionkopie und allen Endstates wird verlangt.

Native Pflicht: actual literal size/capacity/headers/stack/temps/alignment, jede Partition und aggregate±1; volle ShortList-/Entityindexinventur und Dropdescriptor; echte fallible-reserve-Faults; kalte Debug/Release-Allocatorcontrols mit unabhängiger positiver4096-B-Kontrolle. Auch bei letzter Publicationrefusal kein Einzelownerinstall. Whole-World fmt/strict all-target Clippy/rustdoc und Debug/Release auf exakt Root-eingefrorenem accepted Archive plus enumeriertem privaten Overlay. Keine dieser Rustprüfungen ist hier ausgeführt oder aus Java-/Modellchecks abgeleitet.

## 11. Rootentscheidungsbedarf und kleinster nächster Split

Root entscheidet erst nach neuer unabhängiger Reviewrunde über:

1. Getrennte ClosedCompositionauthority und die exakt vier privaten Primitiveamendments aus §4; GEN-06.2 public Single-key1Mi unverändert.
2. Neues proposed Regionceiling2^22 und den kompletten neuen Debitplan einschließlich separat gezählter Copyheads,8-Unit-Heightquery, packed Mapcopy, constructor/admin sowie598119 vorab ausgegebenen Zusatzreserven. Die kleinste hier numerisch useful allowance3766873 ist ein Modellbefund.
3. Pending16MiB-Regionziel mit voll gezählten Tickindizes, aggregatePP131072, native Entity-/Tickindexziel je2MiB,131072 Cleanupnodes/depth32/524288 Dropunits,512 Reserve-/Releaseaktionen,8192/27 Geometry und2048 Validation. Producerownerlife und tatsächliche Capacities sind zusätzliche Gates.
4. Separaten Heightgetter-Handoff ohne zweiten Meter nach angenommenem GEN-03.1-Producer/Worldconsumer. Sein derzeitiges Review erzeugt keine Freigabe hier.

Kleinster Runtimeweg: queued GEN-06.2 unverändert separat abschließen; danach ein **eigenes kleines World-only Interfaceamendment** für sealed Compositeborrow/echte Handoff und einmaliges Outer-/Planseeding, ohne Publicregionproducer. Anschließend GEN-06.4 privater ControlledOreInputs-Kernel mit echten neun Candidatebuffers, demselben Produktionshandoff, allen Sources/Fixturebits und complete Regionwork-/Native-/late-publicationtests. Ein paralleler cloned Testcursor ist unzulässig. Echte Stage-/Map-/Classification-/Storedmembership-/Sidecarownerproducer und vollständiger non-ore/FEATURESdriver bleiben separate Tickets; Simadoption anschließend separat. Root muss die tatsächliche accepted Dispatchbasis dann frisch liefern.

Keine natürliche World-/FEATURES-/Survival-/Plugin-/Lighting-/Nether-/End-/Performance-/App- oder native-memory-Abnahme. Diese Lieferung nimmt keine neue Produktionsauthority in Anspruch.

## 12. Vollständiger neuer Freeze und Reproduktion

A/freeze-manifest.json enthält dieses Dokument und sämtliche **neuen** Tools/Quellen/Inputs-/Provenanzreceipts, compiled Classes/JARs, Original-/Transform-/Mutantenbytes, complete normal/−O observed/control Outputs, Lossless-/Mutation-/State-/Perfeature-Endstatefixtures und finale Prüfungen. `tool-deltas.json`, `generated-helper-deltas.json` und `source-deltas*/**` nennen genaue Originpins und vollständige Textdeltas jeder kopierten oder daraus geänderten Werkzeug-/Helperquelle; neue Tools sind ausdrücklich eigenständig beschrieben. Entwicklungsfehler und ersetzte vorläufige Workmodelle bleiben als explizite Diagnose klassifiziert; sie gelten nicht als bestandene finale Tests. Es wird kein alter Freeze neu geschrieben. Einzige äußere Selfhash-Ausnahmen: freeze-manifest.json, hashes.json und abschließende verify-normal/verify-optimized.json zur Vermeidung zirkulärer Hashes; keine ausgelassenen Entwicklungsfiles/Caches.

`before.json`, `after-normal.json` und `after-optimized.json` rehashen alle readonly Inputpins und die accepted Archiv-/historischen Memberships. `outside-byte-comparison.json` einschließlich letzter `outside-final-live.json` enumeriert das ganze eigene Vorher-/Nachherfenster außerhalb des Schreibscopes: alle geänderten/fehlenden/neuen Sharedpfade mit vollständigen Hashes/Bytes. Vier Änderungen bis zum letzten Capture sind vollständig benannt: IMPLEMENTATION_BOARD, neuer GEN-03.1-Revision2-Auftrag/-Vertrag und neue Carverdelivery. In den erfassten Productiondateien besteht kein Drift in diesem eigenen Zeitfenster. Diese concurrent Änderungen werden weder als eigene Änderung noch als Authority angenommen. `final-checks.json` prüft die unveränderten pre-run Tooldeclarations, finalen Sourceidentitäten/Mutanten-/Guardreceipts und vollständige Modell-/Vergleichsgleichheit normal/−O. Final zählen Productionoverlays/Sharedbuilds null.

`hashes.json` nennt exakte Dokument-/Freeze-/Inputfreezehashes sowie Datei-/Bytecensus. Finale Verifier prüfen every frozen byte, actual ganze neue Membership und externe ganze Pins/Memberships normal/−O. Root kann das Ergebnis ohne irgendeine historische Refreeze publizieren. Exakte Counts/Digests werden erst nach finaler Dokumentfertigstellung erzeugt und separat geliefert.

Reproduktion ausschließlich in frischen Pfaden unter eigenem zulässigem Area, exakte Flags und Sources stehen in neuen Runnern:

```text
python -B A/tools/run.py FRESH observed|control
python -B A/tools-extra/regions.py FRESH observed|control
python -B A/tools-extra/wrapper.py FRESH observed|control
python -B A/tools-extra/lossless.py CANDIDATE --reference INDEPENDENT_REFERENCE --structure
python -B A/tools-extra/validate_runs.py FRESH_RECEIPT RUN
python -B A/tools-extra/inverse_mutants.py FRESH
python -B A/tools-extra/payload_mutants.py FRESH
python -B A/tools-extra/guards.py FRESH
python -B A/tools-extra/work_model.py FRESH_RECEIPT
python -B A/tools-extra/retest_lossless.py FRESH
python -B A/tools-extra/overflow.py FRESH
python -B A/tools-extra/final_integrity.py FRESH_RECEIPT_PREFIX
python -B A/tools-extra/freeze.py verify-normal
python -B -O A/tools-extra/freeze.py verify-optimized
```

Jeweils zusätzlich Python−O; dort payload_mutants_optimized.py greift die echte frische optimized Originalaufzeichnung an. `compare.py` prüft beide ganz vorgegebenen Outputpaare und jede Feature-Endstate-/Core-/Seed-/164-Indexbeziehung. Kopierte Tools behalten Fresh-/Scope-/Source-/JDK-/JAR-/CodeSourceguards. Neues Werkzeug- oder Domaindelta erfordert vor erneutem101-Lauf eine neue Declaration mit vollständigen Toolpins. Keine optional fehlende Erwartung und keine Authority durch eine eigene neue Hashliste.
