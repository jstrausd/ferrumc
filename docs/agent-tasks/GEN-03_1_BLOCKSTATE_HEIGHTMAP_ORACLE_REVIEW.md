# GEN-03.1 — unabhängiger Review GEN-03.1/Runde 1

12. Oktober 2026. **Verdict: FINDINGS.** Die kontrollierten echten Originalwerte
wurden unabhängig vollständig reproduziert. Der Producer-/Consumervertrag ist
wegen **G31-R1-B1** und **G31-R1-B2** noch nicht geschlossen; die Outputadmission
hat den bestätigten Membershipfehler **G31-R1-M1**. Kein pauschales
CLEAN_CONTRACT/CLEAN_ORACLE und keine Runtime-, Cap-, Stage- oder Appabnahme.
Es gab keinen früheren GEN-03.1-Review, keine Autorenantwort und keine Reparatur.

## 1. Bindende Authority und eigener Scope

| Input | SHA256 / Umfang |
|---|---|
| Reviewauftrag `GEN-03_1_BLOCKSTATE_HEIGHTMAP_ORACLE_REVIEW_TASK.md` | `6c9a95aa6ea51f4f3c1652364d51229ea4661f977ff847f6d9dd23fa0ae875f1` |
| Originalauftrag `GEN-03_1_BLOCKSTATE_HEIGHTMAP_ORACLE_TASK.md` | `4f41ecd24a86443db7ae823b960c78b7989e60f968230520c9f63184b7fa74e3` |
| Vollständiger 346-Zeilen-Kandidat `GEN-03_1_BLOCKSTATE_HEIGHTMAP_ORACLE.md` | `260ff459932d8f82f0c5c2bc30898b0de5b0e2a324939911ecafd155a45ee08f` |
| `.codex-tmp/gen03_1-preparation/freeze-manifest.json` | `3d5211d260ebce102705f61d307cb992008d0d9dd470c3886f1a546f3165d48b`, 202 Dateien / 36.215.670 B |
| akzeptierte Basis `gen05_11-runtime-root-base-storage47e/base.freeze.json` | `58ba13d6418133bb49a154682ba1d82533f63a5e6184c6c2a8b07c6c15a47a75`, 1.490 Dateien, darunter 324 World / 93 Storage |
| akzeptierter GEN-06.1-r2-Autorfreeze | `e44f7199a3f222007e9759a6fff69cf0ab030008e6adcbe003b8d03da81315a7`, 366 Dateien / 73.591.041 B |
| akzeptierter GEN-06.1-r2-Reviewfreeze | `dd95a0be1baa4e176857a518978543d86427a2461e8cf796e00a365b1f6e63a4`, 494 Dateien / 307.996.259 B |

Basis ist main `47eadc4c14813c201184b552a87ca7b9c6f97db4`, akzeptierte World
`a912b1c2aa1c0b3de7510156c1b195a506e2e65b`. Shared World und andere aktive
Children wurden weder gebaut noch als Authority übernommen. GEN-06.1 revision2
samt Korrekturen ist bestehende akzeptierte Authority; GEN-06.3/Ore bleibt pending
review. Root entscheidet Veröffentlichung separat.

Geschrieben wurde nur diese neue Reviewdatei und
`.codex-tmp/gen03_1-independent-review-r1/**` (`B/`). `A/` bezeichnet unverändert
`.codex-tmp/gen03_1-preparation`. Keine Gitoperation, Agentdelegation,
Produktions-/Dependency-/Authoränderung oder shared Build. Der native Probe ist
ein isoliertes einzelnes Rustmodell in B, ohne Cargo und ohne Repositorycrate.

Vollständig gelesen wurden beide GEN-03.1-Aufträge und der gesamte Kandidat,
AGENTS/CLAUDE/PAPER_REFERENCE, die archivierten Registry-/World-INVARIANTS,
Architekturüberblick, ADR-0001/0002/0009, der vollständige akzeptierte
GEN-06.1-r2-Vertrag und Review sowie die GEN-06.2/06.3-Aufträge. Geprüft wurden
alle neuen Autorhelper/-tools vollständig, inklusive Preparation, Runner,
Validator, Join, Packing, Mutation, Guard, Source-, Ore-, Bounds- und Freezetools.
Der Sourceaudit umfasst die vollständigen relevanten Methoden und Callees der
31 base/applied-Paare und die betroffenen Source-/Featurepatches. Exakte
Herkunft und Methodenscope stehen in `B/source-audit/scope.json`; ausgewählte
vollständige betroffene Feature-Dateidiffs und deren Herkunft liegen daneben.
Die gepinnte Patchmembership ist kein Gesamtaudit fremder Moonrise-Pfade.

## 2. Findings und konkrete Korrekturen

### G31-R1-B1 — HIGH: Work-/Admission-/Atomizitätsvertrag des Consumers ungeschlossen

**Stellen:** Kandidat Zeilen 287–293 und 302–322; `A/bounds.py::main`,
insbesondere `input_admission`, `prime_bound`, `update_bound`, `query_bound`.

Der Vertrag verlangt vollständige Rawmapprüfung, konservative Reservation vor
Mutation und geprüfte Candidatepublication. Die Worktabelle enthält jedoch
keinen Rawmap-Importkernel. Die 393.216 Einheiten gelten ausdrücklich der
Admission von 98.304 **StateIDs**, nicht der Range-/Paddingprüfung gepackter
Maps. Ein voller sechs-Kind-Rawimport muss mindestens 1.536 Werte, 222 Wörter,
Länge, Padding, Kind-/Phasen-/Geometrieauthority und seine Publication zuordnen.

Die Updateformel `1 + 6*(8 + 4*383 + 3) + 1 = 9.260` beschreibt Rescans,
definiert aber nicht, wie der geprüfte Kandidat entsteht. Eine volle Mapsetkopie
benötigt 222 Wortkopien; die im Primeentwurf angesetzte volle Outputprüfung
allein 1.536*4 = 6.144 Einheiten. Kopie, Paddingprüfung, Cleanup und Commit
sind im Updateentwurf keinem konkreten Kernel zugeordnet. Auch Query8,
Fehlerpriorität und die Kosten des konkreten Statezugriffs sind nicht ausbuchstabiert.
Ein variabler Callback darf diese Kosten nicht beliebig erweitern.

**Probe:** `B/census-{normal,optimized}/receipt.json` reproduziert die Formeln
unabhängig. `B/feasibility.json` und `B/native-layout.json` zeigen einen möglichen
sechs-Wort-Deltakandidaten mit 96 B Scratch. Damit wird **nicht** behauptet,
9.260 sei mathematisch unmöglich: ein Delta mit geprüften geänderten Wörtern,
erhaltener Invariante und anschließend infallible Commit könnte Kopien vermeiden.
Gerade diese entscheidende Wahl und ihre Kosten fehlen im verbindlichen Entwurf.

**Korrektur / Rootbedarf:** Vor Zahlenübernahme Delta oder volle Candidatekopie
festlegen. Für Rawimport, Prime, Update, Query und verweigerte Pfade vollständige
Operation-/Debit-/Reservationstabellen einschließlich Copy, Hash/Validation,
Padding, Cleanup, Maskpflege und Commit liefern; konkrete bounded Reader und
Fehlervorrang festlegen. Anschließend native old/candidate/scratch/stack und
wirkliche N−1/N/N+1 sowie late-refusal oldbytes/Workremaining nachweisen.
Kein stilles Erhöhen bestehender Caps. Mapvorbereitung bleibt ein separater
Producerabschnitt; jede Orequery belastet dessen **bestehenden einzigen** Meter.
Die neue Allowance 4.194.304 erweitert keinen akzeptierten GEN-06.2-Runscope.

### G31-R1-B2 — HIGH: angenommener sortierter Vollstateindex ist keine bestehende Infrastruktur

**Stellen:** Kandidat Zeilen 252–269, besonders „bereits geprüfte sortierte
Identitäten“, höchstens15 Vergleiche/State und die 81.882-B-Buildrechnung.
Akzeptierter `server/crates/ferrumc-registry/build.rs::load_blocks` ab Zeile289
und `::emit_blocks` ab Zeile388 im Basisarchiv.

Die akzeptierte Infrastruktur besitzt Blockmetadata mit Propertydomains und
min/max/default-IDs. `emit_blocks` sortiert 1.105 Blocknamen und Blockranges;
sie besitzt keinen sortierten Index über 27.946 vollständige
Canonicalname+Propertyidentitäten. Der Pythonvalidator erzeugt seine Produkte
selbst; dessen dynamische Datenstrukturen sind keine kostenlos vorhandene
bounded Buildinfrastruktur für den neuen Rustproducer.

Ein neu anzulegender Vollstateindex braucht zusätzlich seine Erzeugung,
Sortierung, Ownership und simultane Kapazität. Bereits ein kompakter
`u16[27946]`-Index benötigt 55.892 B: zusammen mit den angeführten 81.882 B
**137.774 B > 131.072 B**. Ein `usize`-Index benötigt 223.568 B, zusammen
305.450 B. Vollständige Schlüsselstrings/DOMkopien würden weitere Kosten
verursachen. Eine bewusste Lifetimewiederverwendung könnte passen, ist aber
nicht spezifiziert. Das beweist keine generelle Unmöglichkeit des Buildziels.

**Probes:** `B/census.py` prüft jeden State unabhängig durch inverse gemischte
Radixzerlegung der akzeptierten Propertydomains und vollständigen Vergleich;
27.946 Joins, null Identitäts-/Default-/IDabweichungen.
`B/feasibility.json` und der echte Standalone-Rustlayoutprobe zeigen als mögliche
Alternative einen `u16[1105]`-Blocknamenindex (2.210 B), insgesamt84.092 B
Modellbuildbuffer einschließlich der 8.192-B-Localreserve.

**Korrektur / Rootbedarf:** Den Joinalgorithmus schließen: beispielsweise
bounded Blocknamensuche plus vollständige Propertydomainprüfung und sichere
Radix-ID-Berechnung; oder explizit einen Vollstateindex mit Erzeugungs-/Sortier-
Work und belegter Lifetimekapazität. Die gemessenen Originalidentitäten bleiben
Authority. Neue Index-/String-/Produktkosten dürfen nicht als „bestehend“
ausgebucht werden. Das bestehende `load_blocks` nutzt Strings/Serde-DOM und
`emit_blocks` einen wachsenden Ausgabestring; borrowed Bestand darf separat
bilanziert werden, neu verursachte Arbeit und Kopien müssen hinzukommen.
Hash-vor-Parse, Streaminglimits und complete emission bleiben Pflicht.

### G31-R1-M1 — MEDIUM: Outputvalidator ignoriert zusätzliche Verzeichnisse und deren Dateien

**Stelle:** `A/validate.py::parse`, Zeile22:
`{p.name for p in folder.iterdir() if p.is_file()} == six_names`.
Kandidat §5 beschreibt vollständige Outputmembership; der Originalauftrag
verlangt Verweigerung unvollständiger Membership.

**Echte unabhängige Probes in beiden Modi:**
`B/challenges-{normal,optimized}/extra-unlisted-directory/` enthält alle sechs
unveränderten Original-TSVs **plus `unlisted/extra.tsv`**. Der unveränderte
Autorvalidator akzeptiert die gesamte Kandidatdirectory.
`B/supplement-{normal-v2,optimized-v2}/extra-empty-dir/` bestätigt dasselbe für
ein zusätzliches leeres Verzeichnis. Ein zusätzlicher Top-level-File wird
korrekt verweigert; das vollständige Original wird korrekt akzeptiert.
Die Mutanten und die vollständigen Resultate sind erhalten und eingefroren.

**Korrektur:** Exakte komplette Directorymembership vor Parsing prüfen;
zusätzliche Dateien, Verzeichnisse und unerlaubte Symlinks explizit verweigern.
Nested-file- und Empty-directory-Mutanten plus Originalpositivkontrollen frisch
normal/−O ausführen. Der vollständige **Authorfreeze** wird separat geprüft und
hat keinen beobachteten Drift. Dieses Finding verändert nicht die erfolgreich
reproduzierten kanonischen sechs TSVs und behauptet keine falschen Stateflags.

## 3. Unabhängige echte Originalreproduktion und Join

`B/reproduce.py` rehashed vor/nach jedem Lauf den ganzen Authorfreeze, alle4.438
Inputpins, Directorymembership, vollständiges1.490-Dateien-Basisarchiv und
Helperpackage-Shadows in den echten Runtime-JARs. Der unveränderte gepinnte
`A/HeightmapOracle.java` wurde mit dem gepinnten JDK getrennt frisch kompiliert.
Der reviewer-eigene Runner startete die echten Vanilla-/Paper-inner-JARs direkt;
keine Expectedprädikate oder Updateformeln wurden als Javaoracle nachgeschrieben.
Vollständige Commands und JVMlogs liegen in `B/{normal,optimized}/`.

Jede der vier frischen Profil-/Moduskombinationen hat exakt sechs Outputdateien,
keine versteckten Unterverzeichnisse oder Symlinks. Alle sechs TSVs sind je
Profil bytegleich mit beiden Autorläufen. Die frisch erzeugte Helperclass ist
in allen vier reviewer/author-Moduskombinationen bytegleich:
`5483fb5cf2bbba21abb1d5a4120095d7776c324a73e4656865e2d88e6e6a4eeb`.
Vanilla/Paper states/types/census/statuses/trace sind bytegleich; Origins tragen
korrekt die jeweiligen Originalklassennamen. JAR-/Library-/Helper-CodeSource,
Officialmapping und 454-Dateien-JDKmembership wurden frisch geprüft.

| Eigenständig bestätigtes Ergebnis je Profil | Wert |
|---|---:|
| echte Blöcke / States / Heightmap.Types | 1.105 / 27.946 / 6 |
| direkte echte Heightmap-Prädikatcalls | 167.676 |
| Canonicalname+sortierte Properties+Default+ID-Joins | 27.946, null Abweichungen |
| vollständige beobachtete Fluididentitäten | 20 |
| C / D / W / U / P / Q / S | 10 / 222 / 194 / 114 / 8 / 6 / 282 |
| Tracezeilen / überprüfte Snapshotspalten | 836 / 72.192 |

`B/census.py` benutzt die akzeptierte inverse State-ID-Domainzerlegung als
anders aufgebauten Vollidentitätsjoin. Bloße Counts oder IDgleichheit werden
nicht als Identitybeweis verwendet. Jeder Default wurde geprüft; jeder
u16-Flagbit und Fluid-/Defaultslot wurde aus den **echten** Flags zurückdekodiert.
Keine Collision und kein Informationsverlust. Lexikalische Fluid-Slots0..19
sind weiterhin lokale Klassifikationen, keine originalen WireIDs.

Truecounts bestätigt: default1.105, air3, motion23.659, leaves308,
fluid_empty18.254, Surface/WG27.943, Floor/WG23.659, Motion24.154,
NoLeaves23.846. Die festgehaltenen Flat-/boundingBox-Differenzlisten sind aus
Originalflags abgeleitet; Shape-/Support-/Collision-/Lightauthority folgt daraus
nicht. Die fünf bisherigen Flatstates bleiben in ihrem engen Scope korrekt.

| Resultat | SHA256 |
|---|---|
| frisches states.tsv | `078215ee6f0ee1cca461b280a6d4666330c0b042db21e542c1ffd6c3b9719eb7` |
| frisches trace.tsv | `f936c3fcfc58cb1a9b08a025023dfa10f1e324c574e01a24854047f3505d1e62` |
| unabhängiger kompletter identity-joins.tsv | `adb062807b6481acf813bd7358ba2863f04d3df6af7d8bfe00ba91fa4a14e62b` |
| unabhängige predicate-table.u16le | `95e27539aff80d384a7d9841454210da95ceb2ea8c56353d37b19c7d9cbb2701` |

## 4. Kontrollierte Chunks, Quellen und Lifecyclegrenzen

Alle zehn Chunks sind echte vollständig konstruierte EMPTY-ProtoChunks mit
originaler BiomeRegistry und originalen Konstruktoren. Der Helper verwendet
keinen Statussetter, Unsafe, uninitialisierten Chunk oder Fake-ServerLevel.
Alle222 Inputs,836 Events und vollständigen256-Spalten-Snapshots wurden gegen
die echten frischen Ergebnisse validiert. Der unabhängige Traceaudit verfolgt
alle194 Previous-/Readback-States aus den kontrollierten Writes.

Bestätigt sind alle sechs Typen und deklarierte Air/Caveair/Voidair-, Fluid-,
Waterlogged-, Leaf-, Plant-, Snowlayer- und Slabvarianten; Placement/Removal/
Replacement/Rescan sowie unterschiedliche Minima und Extrema. `firstAvailable`
ist min+relative, `highestTaken` first−1; leer bei min−64 ergibt−64/−65/raw0,
oberster Block319 ergibt first320/relative384. Das Packing ist nonspanning,
neun Bits, sieben Werte/Wort,37 Wörter, Index z*16+x. Rawrange511 und gesetztes
Padding werden vom richtigen Javaarray lenient übernommen; falsche Länge führt
zum echten Primefallback. Neue Range0..384-/Zeropadding-/Lengthrefusal ist klar
strengere FerrumC-Policy, keine Fehlerinputparität.

Quellen und frische javap-Ausgaben bestätigen: Prime startet am höchsten
gefüllten Sectionbereich, überspringt literal Blocks.AIR und setzt vorhandene
Rawmaps nicht zurück. Nicht aufgelöste Listenreferenzen können bestehen bleiben;
fixedmask Priming frischer Maps ist eine neue bounded Kernelwahl. Stale reprime
repariert die entfernte Topstate nicht. `hasPrimedHeightmap` prüft nur Präsenz.
Update überspringt y≤first−2, hebt passende neue Tops an und rescant bei
Topentfernung downward. Returnwerte stammen aus den Originalcalls.

`ChunkAccess.getHeight` maskiert x/z und liefert **highestTaken**;
`WorldGenRegion.getHeight` addiert eins und liefert **firstAvailable**. Das sind
unterschiedliche Consumer-/Adaptertypen. EMPTY-Protosetter aktualisieren WG;
vorhandene finale Maps bleiben im Probe stale. Sourceauthority zu Statussets
ist getrennt: EMPTY bis SURFACE WG; CARVERS bis FULL finale vier. NOISE.doFill
schreibt Sections und aktualisiert anschließend zwei WG-Maps. Surface verwendet
Protosetter und SurfaceWG+1, einschließlich eroded/frozen Callerchronologie.
FEATURES primet finalfour vor Decorations und Borderticks. Live-LevelChunksetter
hat zusätzlich originale Light-/Entity-/onPlace-Effekte; Proto→Level übernimmt
die nach FULL behaltenen vier Maps. Diese Quellenbefunde sind keine ausgeführten
natürlichen Stagecompletions.

Der ganze OreFeature wurde geprüft: OCEAN_FLOOR_WG-Gate über WorldGenRegion,
anschließend direkte Sectionwrites; keine erfundenen Heightmap-/POI-/Entity-/PP-
Updates. `B/ore_audit.py` liest die echten27 JARresources je Profil (24 Configured,
drei Tags),30 akzeptierte GEN-06.1-r2-Pläne und **frische Originalstates**. Die82
Factszeilen sind bytegleich zum Autor;25 Targets und Tagstatecounts8/4/4.
Alle admitted Targets/Replaceables sind notair/motion/!leaves/emptyfluid und für
alle sechs Prädikate true. Das beweist nur deren enge Prädikaterhaltung, keine
pending GEN-06.3-Abnahme oder tatsächliche Region-/Bulk-/FEATURES-Chronologie.
Historische WGmaps dürfen weiterhin nicht aus finalen Endstates rekonstruiert werden.

## 5. Parser-/Guardaudit und native/work Feasibility

Frisch normal/−O je **49 abschließend bewertete Fälle**:42 Refusals, fünf echte
beabsichtigte Positivkontrollen und zwei unerwartete Membershipadmissions (M1).
Erhalten sind späte IDlücken, Default-/Predicate-/Propertymutanten, unbekannte
Fluidproperties/Typeusage, Origin-/Statusfehler, signed−0, Previous-/Readback-
Änderung, Sentinel-/Paddingfehler; außerdem fehlende/duplizierte letzte
States/Types/Origins/Traceevents, Header-/Bool-/Canonicalfehler und Truncation.
Der Validator verwendet explizite Refusals unter−O und unveränderte unabhängige
Originaloutputpins. Seine dynamischen Produktmengen sind offline Prüfwerkzeuge;
sie gelten nicht als bounded Runtimegenerator.

Fünf echte Envrefusals und sechs echte JVM-CodeSourcefehler (Original/Helper/
Library je Profil) verweigerten vor TSVausgabe. Hinzu kommen tatsächliche
private JAR-, Mapping- und JDKbyteänderungen gegen unveränderte Originalhashes,
echte Directorymemberänderung mit erfolgreicher Positivkontrolle und ein gültiger
ZIP mit eingefügtem Helperpackage gegen den ZIPshadowguard. Originale blieben
unangetastet. Resultate: `challenges-*`, `guards-*`, `supplement-*-v2/results.json`.
Frühere `supplement-normal/optimized` hatten in der Directorypositivkontrolle
fehlende Probeconfiguration (`jdk`, KeyError); diese Harnessfehlläufe bleiben
als Diagnose erhalten und sind aus den49 finalen Fällen ausgeschlossen. Beide
korrigierten v2-Läufe bestehen die Positivkontrolle. Kein Finding daraus.

Staticpayload57.281 B und Buildkomponentensumme81.882 B sind algebraisch korrekt.
Die echte **Standalone-Modellmessung** mit rustc1.99.0/x86_64 bestätigt
16-B-&str-Header,24-B-Deskriptor, Mapset1.792 B/align8, Scratch96 B/align8,
zwei Sets+Scratch3.680 B; Modellbuildbuffer mit kompaktem Blockindex84.092 B/align2.
Static auf8 gerundet57.288 B lässt8.248 B für Provenance/weitere Alignmentkosten.
Das sind konkret machbare **Modelllayouts**, keine Messung künftiger
Produktionsstructs, Stackframes, Compiler-Moves, Allocatorpeaks oder RSS.
Ein drittes vollständiges Set, ein Option<i32>-Cache oder String/DOMkopien
können die Ziele verbrauchen. B1/B2 schließen die verbleibenden Work-/Indexfragen.
Die vorgeschlagenen Ziele65.536/131.072/2.048/512/4.608 B sind nicht akzeptiert.

Der bestehende World-`BlockStateId::new`, `From<u32>` und AIRdefault lassen
ungeprüfte Rohwerte zu. Der Kandidat fordert korrekt einen **neuen** opaken
Registrytyp mit fallible Konversion. Diese Gatepflicht wird nicht als bereits
vorhandene checked Stateauthority behandelt: spätere Implementierung muss alle
Getter/Conversions sowie Default/Serde/unchecked-Bypässe prüfen und unbekannte
IDs/Properties verweigern. Source-/Inputlimits und Hash-vor-Parse sind passende
Entwurfsziele; vollständige Build-/Consumerfehler und echte Native-/Workgrenzen
bleiben getrennte Implementierungsbelege.

## 6. Rootentscheidungen, Lieferung und verbleibende Gates

Root benötigt eine korrigierte Vertragsfassung zu B1/B2 und korrigierte frische
Membershipbelege zu M1, jeweils eigenständig eingefroren und erneut unabhängig
reviewt. Die erfolgreichen kanonischen Census-/Tracewerte können als genau
begrenzte Originalbeobachtungen weiterverwendet werden; dieser Review gibt
keine pauschale Evidenceadmission oder Runtimefreigabe.

Der kleinste sinnvolle Split bleibt: zuerst Registry-only Producer/API mit
vollständigem Join-/Parser-/Nativebeweis; danach World-only sechs-Kind-Consumer
gegen akzeptierte Registryauthority, inklusive Packing/Sentinels, Work und
atomic late refusal. Anschließend separater echter NOISE/SURFACE/CARVERS-
Producer und WGhandoff, echte WorldGenRegion/Oreconsumerprüfung. World hat
weiterhin nur den bestehenden exklusiven Writer.

Residualgates: genuine ServerLevel und Chunkdependencyholder, Originalresources/
Session-/Owneradoption, INITIALIZE_LIGHT/LIGHT, echte LevelChunkkonversion und
Liveupdates, NOISE/SURFACE/CARVERS/FEATURES-Completion sowie native Caps/Stack/
Work/Allokation. Survival, Natural-World, Plugins, Performance und App sind
keine Ergebnisse dieses Reviews.

`B/review-inputs.json` bindet die vollständigen Inputpins und Zusatzinputs;
`B/integrity-{normal,optimized}.json` bindet source/output/tool-Provenance und
rekonstruierte historische Membership. Vor/nach den Originalcalls und abschließend
sind Candidate,Authorfreeze, alle4.438 Inputpins,62 Sourcekopien/Origins und
historische/baseline Freezes unverändert. `B/freeze-manifest.json` enumeriert
**diesen Bericht und sämtliche neuen Reviewerdateien/Directorymembership**;
`B/hashes.json` trägt exakte Report-/Freezehashes und Counts ohne zirkuläres
Selbsthashing. `B/post-freeze-{normal,optimized}.json` dokumentiert die frische
abschließende readonly Verifikation. Veröffentlichung und Rootentscheidungen
folgen separat; Author-/Original-/historische Freezes wurden nicht refrozen.
