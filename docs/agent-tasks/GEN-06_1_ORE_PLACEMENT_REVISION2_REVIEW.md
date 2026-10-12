# GEN-06.1 — unabhängiger Korrekturreview, Runde 2

12.10.2026. Neuer Review der vollständigen [Originalworkorder](GEN-06_1_ORE_PLACEMENT_TASK.md), des [Originalvertrags](GEN-06_1_ORE_PLACEMENT.md), des [ersten Reviews](GEN-06_1_ORE_PLACEMENT_REVIEW.md), der [Root-Korrekturworkorder](GEN-06_1_ORE_PLACEMENT_REVISION2_TASK.md) und des vollständig eingefrorenen [Revision-2-Vertrags](GEN-06_1_ORE_PLACEMENT_REVISION2.md). Exklusiver Auftrag: [Reviewworkorder](GEN-06_1_ORE_PLACEMENT_REVISION2_REVIEW_TASK.md).

**Verdikt: CLEAN_CONTRACT / CLEAN_ORACLE ausschließlich für die kontrollierte Vorbereitung. CR61-B1/B2/B3 sind geschlossen.** Der Entwurf ist ausreichend konkret für einen separat von Root zu vergebenen World-only-Primitiveticket. Dieses Urteil autorisiert keine Runtimearbeit und nimmt keine native Konstruktion, Allocation, Stack-/RSS-Messung, CARVERS, vollständige Section-/Corechronologie, generierte FEATURES, Survival, Performance oder Appaktivierung ab. Root entscheidet separat über Vergabe und Capannahme.

Root nennt für diesen Review Main `4bf91462`, docs-only über akzeptiertem World `a912b1c2`; der ältere Mainname `c46077e1` im eingefrorenen Autorenvertrag ist dessen historische Basisangabe. Maßgeblich sind die frisch verifizierten 324 akzeptierten Worlddateien. Keine Gitaktion, Dependency, Produktionsänderung oder weitere Agenten. Mutable Storage und unangenommene private Simstände wurden weder gebaut noch als Authority verwendet. GEN-05.11 bleibt unangenommen und hat einen eigenen Reviewowner.

## 1. Unabhängige Integrität und genaue Authority

`audit.py` hat selbst jeden eingefrorenen Hash und die spezifizierte Membership geprüft, vor den Experimenten und erneut nach Fertigstellung dieses Berichts, normal und Python `-O`. Abschließende Worldauthority ist ausdrücklich die vor den Experimenten vollständig verifizierte private akzeptierte Kopie. Das sind eigene Rehashes; Roots vorherige Prüfung bestätigt keine Claims und ersetzt diese Prüfung nicht.

| Readonly Objekt | Dateien / Bytes | SHA-256 |
| --- | ---: | --- |
| Originalvertrag | einzeln | `a12b58d1bffbfd0ad918afe624fec7f32cefcc117d4a63a31c7ba431ab46b082` |
| Originalfreeze | 1.147 / 50.513.276 | `720c775e6defd843996de0229687a9fe9882fe3740ba0921368812f0ab73a269` |
| erster Reviewbericht | einzeln | `48737c693cf6bdcb6db68e62e069a7df9cd47d82591b5b109fb0357c90bc59a0` |
| erster Reviewfreeze | 164 / 108.222.925 | `336a8e7910acabfde0c7afcf29e70d84cb672b937f306b7df5652d231dfbb0fe` |
| Revision-2-Vertrag | einzeln | `24ab2da586620009f25bfbfd99e433215d7e8698ec76a62eaff03ec03ec74eb8` |
| Revision-2-Freeze | 366 / 73.591.041 | `e44f7199a3f222007e9759a6fff69cf0ab030008e6adcbe003b8d03da81315a7` |

Zusätzlich: sämtliche 1.728 Originalinputpins, vollständige 454-Dateien-JDKmembership, Source-/Helpermembership, 190 ganze Source-/Patchkopien gegen ihre tatsächlichen Herkunftsbytes, 894 Ressourcenkopien gegen die tatsächlichen ZIPentries und alle 324 akzeptierten Worlddateien samt Membership. Neue Werkzeugkopien werden gegen ihre gepinnten Ursprünge geprüft. Alle historischen Verträge, Reviews, Helpers, Fixtures und Freezes bleiben bytegleich. Neue Kontext-/Toolchaininputs sind in den eigenen Provenanzreceipts enumeriert.

**Finaler Outside-bytebefund:** Der geteilte Worldtree änderte sich während dieses Reviews. Die erste abschließende Liveprüfung verweigerte normal und `-O` am Pin `src/biome_selection.rs`; der Snapshot `shared-world-final-drift.json` enumeriert 27 geänderte bestehende Dateien und die neue `src/sample_work/cumulative.rs` (325 statt324 Mitglieder). Diese Bytes werden weder akzeptiert noch als Authority übernommen. Keine dieser Änderungen stammt von diesem Reviewer; sämtliche eigenen Cargoexperimente bauen ausschließlich den separaten Sketch. `audit-live-world.py` erhält den verweigernden Prüfpfad, `final-live-audit-refusals.json` den Befund. Der finale `audit.py` prüft transparent die frühere private akzeptierte Kopie gegen dieselbe accepted Freeze und deren exakte Membership. Root muss vor einer Runtimevergabe die tatsächlich gewählte accepted Basis neu festlegen; dieses Urteil akzeptiert keinen veränderten Live-Worldstand.

Originale: Minecraft Java 1.21.8 / Protokoll 772, Paper Build 60 / Commit `29c8822d90899c89d2689338e81a98f690bcba12`; Vanilla-Inner `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`, Paper-Inner `c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`, gepinnter JDK 21.0.12.1+1. AGENTS, CLAUDE, PAPER_REFERENCE, Worldinvarianten/-architektur und die akzeptierten Density-, Material-, Biome-, Stored-Biome-, Temperature- und Surfaceproducergrenzen wurden einbezogen.

Quellenprüfung umfasst die vollständigen relevanten Base-/Applied-Methoden und Callees für Decoration/FeatureSorter/indexlookup, WorldgenRandom/BitRandomSource/Xoroshiro, Placed-/ConfiguredFeature/Feature/context, Modifier/Provider/Anker, Ore/Scattered/Target-/Luftprüfung, Bulk-/Section-/ProtoChunk-/Regionpfade, Heightmaps und FEATURES-Pyramid/Tasks. Die relevanten Sourcepatches und Featurepatchschichten 0004/0012/0016 wurden dazu berücksichtigt. Ganze Patchdateipins belegen Integrität; daraus folgt keine Prüfung aller fremden Moonrisemechaniken. Registry-Namelookup, Properties/defaults/dense ranges, akzeptierter RNG und die vorhandenen Parser wurden tatsächlich gelesen. Sourcebefunde sind keine ausgeführten Serverstages.

## 2. CR61-B1: vollständiges Binding und konkreter Speicherentwurf

### Eigener Census direkt aus Original-ZIPentries

Der neue reviewer-eigene `census.py` benutzt die frisch ausgeführten possibleBiome-/Indexrows und die Original-ZIPentries. Er entdeckt über tatsächliche ConfiguredFeature-Typen; weder `ore_`-Präfix noch eine Fixtureinventur liefert die Closure. Sein Algorithmus ist vom Autorcensus getrennt. Normal und `-O` ergeben bytegleiche finale Receipts.

| Vollständige Closure | eigenes Ergebnis |
| --- | ---: |
| Placed / Configured / Tags | 30 / 24 / 3 = 57 Ressourcen |
| Vanilla Quellbytes / Maximum | 26.930 / 720 B |
| Paper Quellbytes / Maximum | 26.987 / 721 B |
| SHA-Kompressionsblöcke einschließlich Padding, Vanilla / Paper | 463 / 464 |
| Nodes / Keys / Edges | 1.684 / 727 / 1.627 |
| Strings / dekodierte Stringbytes / Duplicate-Keypaare | 1.070 / 12.356 / 364 |
| größte Quelle: Nodes / Tiefe / Entries / String | 41 / 7 / 6 / 36 B |
| Modifier / Countprovider / Heightprovider / Anker | 120 / 26 / 30 / 60 |
| Configured-Targetreferenzen / Placed-Targetreferenzen | 41 / 50, verschiedene Zählgrundlagen |
| eindeutige Zielstates / explizite Properties | 25 / 3 |
| Tagreferenzen / expandierte Statezugehörigkeiten | 12 / 16 = 8+4+4 |
| vollständige globale Indexrecords | 164; normal = large_biomes = amplified |

Jede der 57 Paperentries ist exakt die entsprechende Vanillaentry plus ein LF. Geparste JSONwerte sind identisch; Hashes bleiben profilgenau und werden nicht normalisiert. Paper ist die Produktionsquelle, Vanilla ausschließlich diskriminierende Kontrolle. Alle 30 PlacedFeatures dispatchen OreFeature, einschließlich der Nicht-Erzablagerungen; Scattered bleibt separate Netherkontrolle. Die 25 echten States einschließlich `lit=false` zweimal und `axis=y` einmal sind vollständig gebunden. TagMatch umfasst alle gültigen States der originalen Blockmitglieder, ohne RNGdraw und ohne vollständigen Produktionsscan über 27.946 States.

Der tatsächliche Registrylookup ist allocationfreie binäre Suche über 1.105 Einträge. Größter bare Blockname: 34 B. Der Reserveblock 12×64 pro Aufruf deckt Namespaceprüfung und die begrenzten Namensvergleiche. Der eigene Resolvercheck benutzt tatsächliche Registrydefaults, Min-/Maxranges und Propertycardinalities. Die drei Overrides betreffen jeweils eine Property mit höchstens drei Werten. Eine private bounded Berechnung kann ihre tatsächlichen Schleifen und checked Arithmetik mit 136 Einheiten abdecken. Der allgemeine allokierende BTreeMap-Resolver ist dafür kein erlaubter Ersatz.

### Jeder Debit und die tatsächliche Reihenfolge

Eigene Ableitung aus den einzeln gezählten Ressourcen, nicht Übernahme des Autor-Gesamtergebnisses:

| Phase | Formel | Vanilla | Paper |
| --- | --- | ---: | ---: |
| Beginn | 1 | 1 | 1 |
| Allocationadmission / Releasereservation | 2+2 | 4 | 4 |
| Initialisierung | 32.768 | 32.768 | 32.768 |
| Quellheader | 57 | 57 | 57 |
| SHA-Input | B | 26.930 | 26.987 |
| SHA-Schedule/-Runden | 128H | 59.264 | 59.392 |
| Digestvergleich | 57×32 | 1.824 | 1.824 |
| exakter Originalbytevergleich | B | 26.930 | 26.987 |
| Syntaxbytekonsum | B | 26.930 | 26.987 |
| Node-/Edgeadmission | 1.684+1.627 | 3.311 | 3.311 |
| einmalige Stringdekodierung | 12.356 | 12.356 | 12.356 |
| padded Schema-/Keyslots | 727×4×64 | 186.112 | 186.112 |
| decoded Duplicate-/escaped-Duplicatepaare | 364×64 | 23.296 | 23.296 |
| Stringvalueklassifikation | (1.070−727)×64 | 21.952 | 21.952 |
| Configuredreferenzen | 30×24×64 | 46.080 | 46.080 |
| Target→Tag | 41×3×64 | 7.872 | 7.872 |
| gruppierter Registryreserve | (41+12)×12×64 | 40.704 | 40.704 |
| gruppierte Propertyreserve | 3×136 | 408 | 408 |
| Modifierdispatch | 120×5×64 | 38.400 | 38.400 |
| Countdispatch | 26×2×64 | 3.328 | 3.328 |
| Heightdispatch | 30×2×64 | 3.840 | 3.840 |
| Ankerdispatch | 60×3×64 | 11.520 | 11.520 |
| vollständiger Indeximport | 164×65 | 10.660 | 10.660 |
| beide Index-/Keypaarprüfungen | 2×164×163/2 | 26.732 | 26.732 |
| Planindexbindung | 30×164 | 4.920 | 4.920 |
| Tagranges/-States | 12+16 | 28 | 28 |
| typed IR-/Planbuild | 57+120+41+30+24+3 | 275 | 275 |
| vorreservierter Resourcecleanup | 57×4.096 | 233.472 | 233.472 |
| vorreservierter Scratchcleanup | 16.384 | 16.384 | 16.384 |
| Releaseadmission / letzte Publication | 1+1 | 2 | 2 |
| Targetpaare / Abschlusscensus | 820+2.078 | 2.898 | 2.898 |
| **N** | gesamte Summe | **869.258** | **869.557** |

`accounting_model.py` modelliert die konkreten per-resource Prefixe: finaler Cleanupreserve vor Allocation; Resourcecleanupreserve vor ihrem Parserzustand; Header, SHA, Bytevergleich, Syntax/Nodes/Edges/decoded Strings, padded Vergleiche und Typ-/Referenz-/State-/Tag-/IRbuild; danach vollständiger 164-Import, alle Paarprüfungen, Planindizes, Abschlusscensus, alte Releaseadmission und zuletzt Publication. Die Tabellenposition der Censuszeile ist keine Anweisung, sie nach Publication auszuführen.

1.840 Erst-/Letztgrenzkontrollen je Profil prüfen Einzelschritte und Gruppenadmission. Reserve 768 bzw. 136 wird atomar **vor** dem tatsächlichen Registry-/Overridecall verbraucht. Verweigerung verändert used nicht und ruft keine innere Arbeit auf; ungenutzte Reserve bleibt absichtlich ausgegeben. Diese Reserve ist eine Policyobergrenze, keine behauptete Messung des Bibliothekscalls. SHA-Padding/Schedule/Runden sind bounded; ein beliebiger kostenloser Hashlibrarycall wäre keine Umsetzung dieses Vertrags. Dekodierte Strings werden einmal im Pool gespeichert, danach aus gespeicherten Bytes verglichen. Die paired Keyprüfung kann nach dem gepolsterten Import kanonische Ordinale vergleichen; ein zusätzlicher ungezählter Stringvergleich ist damit nicht erlaubt. Kein hidden second parse, HashMapgrowth, Endcensus nach Publication oder nachträglich zugelassener Cleanup.

| eigenes Policy-Modell | N−1 | N / N+1 |
| --- | --- | --- |
| Paper | 869.556, Refusal bei FINAL_PUBLICATION, alter Katalog erhalten | used 869.557, gleicher neuer Katalog |
| Vanilla | 869.257, Refusal bei FINAL_PUBLICATION, alter Katalog erhalten | used 869.258, gleicher neuer Katalog |

Der Late-census-Mutant publiziert bei N−1 und verweigert erst danach: eigener Checker erkennt genau diesen Vertragsbruch. Zusammen mit split-Registry-, Censusfeld-, Summe- und Globalindexmutationen: **19 neue Modell-/Censusnegativkontrollen**. Das sind eigene Designmodelltests, kein ausgeführter nativer Konstruktor. Native Umsetzung muss jeden echten content-dependent Loop dem entsprechenden Debit zuordnen; fixed administrative Branches und vorreservierter Cleanup dürfen keine neue Inhaltsarbeit verstecken. Nach Publication verbleiben nur infallible, bereits admitted Ownershipwirkungen; alter Katalog wird erst dann freigegeben.

### Native Feasibility und separate Empfehlung an Root

**Empfehlung: die vorgeschlagenen 16 KiB Konstruktionsscratch / 32 KiB neuer Peak beibehalten**, gemeinsam mit 16 KiB retained Katalog, 49.152 B bei gleichzeitigem altem Katalog und 512 B Cursorziel. Der Census liefert keinen Grund, einen kleineren Cap zu erzwingen. Diese Zahlen sind umsetzbare Entwurfsziele, keine gemessenen Layouts oder Laufzeitfreigabe.

Vier 4-KiB-Scratchbudgets erlauben eine einzelne bounded Quelle: Parserarena mit 64 POD-Nodes à Ziel32 B, 721-B-Stringpool, acht Ziel32-B-Frames, 64-B-Decoderfenster und Ziel512-B-Metadaten = 3.601 B vor der 495-B-Reserve; getrennt typed Resource-IR, Referenz-/Index-/Tagworkspace und SHA-/Compilerlocals/Alignment. Kein 57-Quellen-Aggregatbaum. Kleine IDs, offsetbasierte String-/Nodebezüge, feste typed Records und disjunkte ranges sind ohne neue Dependency und ohne unsafe möglich. Das veröffentlichte Stagingobjekt bleibt derselbe Katalog; kein zweiter Finalkatalog und kein retained Candidate-Vec.

**16 KiB Arena plus externe Locals wären mehr als 16 KiB Scratch.** Arenaheaders, Vec-/Ownerheaders, tatsächliche Capacities, Return-/Error-/IRtemporaries, SHAstate/-schedule, Compilerlocals, Callstack und Alignment müssen innerhalb der vier Budgets liegen. Der Autor kann Localsplatz vom Arenaanteil abziehen oder Partitionsdaten im selben sicher adressierten Speicher halten; kein by-value Großarray auf dem Stack und keine unfallible Boxkonversion. Native Umsetzung muss die feste Initialisierungs-/Reservationspolicy konkret abbilden und tatsächliche simultane Bytes nachweisen. Die rein algebraischen 3.601 B sind keine `size_of`-Messung. Zwei vorab fallibel admitted Allocations sind für kompakte fixed Slots möglich; Headerbytes gehören trotzdem zum Cap. Größere Allocatorcapacities zählen vollständig und müssen vor Inhaltsschreiben verweigern. Jede spätere Growthanforderung ist an bereits admitted capacities gebunden.

Die vorhandene Density-JSON-DOM allokiert rekursive Container/Strings; der Climateparser dekodiert Strings wiederholt. Unveränderte Wiederverwendung dieser Pfade erfüllt den neuen POD-/Einmaldecode-/Debitvertrag nicht. Der kleine private Parser/Compiler muss die angegebenen Bounds und denselben cfg(test)-Kern tatsächlich implementieren. Root entscheidet über Verdichtung oder einen neuen separat quantifizierten Capänderungsentwurf, falls konkrete Messungen die Ziele verfehlen. Kein neuer Dependencyentscheid ist durch diesen Review implizit angenommen.

## 3. CR61-B1: sealed Pending, ein RNG und ein Runmeter

Die private Session besitzt genau einen RNG und einen unveränderlichen RunAllowance `1..=2^20`; BindAllowance ist separat. Fresh→Running→Pending→Running/Complete und terminal Poisoned sind konkret. `next` bleibt lazy: Count/Rarity, X vor Z, Height, deferred Biomefortsetzung. Pendingposition ist weder Biompass noch Oresuccess. Kein Consumerseed, RNGexport/-clone, Reset, Allowanceupgrade oder öffentlicher Callback/bool kann authority liefern. Die spätere 2^32-Regionidee besitzt hier keinen Transfer-/Upgradepfad.

Der Katalogborrow lebt über die Session; ein alter geborgter Katalog kann nicht gleichzeitig über `&mut CatalogOwner` ersetzt werden. Pending leiht exklusiv `&mut Session`, nimmt sich genau einmal beim privaten Dispatch und erzeugt den Receipt mit internem Identity-/Sequenzwert. Fremde Caller können diese Werte nicht konstruieren. GEN-06.2 enthält keinen produktiven Biom-/Writerconsumer. Eine leere private Produktions-Consumerenum und ein separater privater `consume`-Callee sind Rust-kompilierbar; cfg(test)-Varianten können **denselben** Dispatch-/Borrow-/Meter-/Completionkern ausführen. Eine erfundene produktive Successvariante ist unnötig.

**`Option::take` darf den Dropguard erst nach allen falliblen oder panikfähigen Consumer-/Completionaktionen disarmen.** Ein frühes take wäre ein Fehler. Failure, verweigerte Draw-/Completionadmission, Panicunwind und unvollständiger Drop poisonieren mit konstantem infallible Cleanup. Vergessene Lease bleibt Pending; der nächste Begin/next verweigert und poisoniert ohne RNG-/Workmutation. Kein Rekonstruieren oder neuer Versuch. Bei abort gibt es keine fortsetzbare lebende Session; der geprüfte Guardfall ist Unwind.

Jede tatsächliche `next(bits)`-Coremutation, einschließlich **jeder rejected bounded-int-Iteration** und des zweiten nextDoublecores, charged vorher denselben Meter. Verweigerter charge verändert weder RNG noch successful_core; erfolgreiche frühere Prefixe bleiben diagnostisch. Original-Java-getCount inkrementiert vor Delegation und kann bei kontrollierter Delegationexception successful_core+1 sein. Die genuine Reject-all-Rows mit Core0/1/2/3 versus getCount1/2/3/4 bleiben getrennt vom FerrumC-Debit.

Eigene Policysummen: Begin/Seeding8; UniformCount0 bzw. Rarityfalse11; sieben uniforme ore_dirt-Candidates mit jeweils interleavtem Consumerfloat80 und32 erfolgreichen Coremutationen. Dies sind neue FerrumC-Workanforderungen. Die zugehörigen erwarteten Drawbits stammen aus echter lazy Originalausführung, nicht aus diesem Modell.

### Eigene private Rust-Designprobe, keine FerrumC-Abnahme

Alle 324 akzeptierten Worlddateien wurden vor der Probe bytegenau in `accepted-world-copy/` kopiert; Provenanzreceipt enumeriert sie, **kein Overlay auf kopierte Worlddateien**. Gebaut wurde ausschließlich `design-probe/`, ein separates dependencyfreies Reviewerpackage mit synthetischem Corecounter. Es implementiert weder FerrumC-RNG noch native Binding oder Layoutmessung. Outside inputs: gepinnte Rust/Cargo1.99.0-Toolchain und enumerierte Targetlibraries. Kein Storage/Sim/Worldcratebuild.

Mit `/home/jstrauss/.cargo/bin/cargo`, `--locked --offline -j 2`, `RUSTFLAGS='-D warnings'`: jeweils **3 Tests Debug und Release bestanden**. Sie prüfen Runallowances1..81/finales End1/N80/Core32, Drop/Failure/Panic vor und nach Draw/forget sowie unveränderten erfolgreichen Corecount bei verweigertem charge. Compilecontrols verweigern zweites next (E0499), doppelte Completion (E0382) und Katalogaustausch während Borrow (E0506). Drei eigene Verhaltensmutanten — frühes Option::take, Core vor charge, Consumermeterreset — scheitern an erwarteten Tests.

Diese Probe belegt die strukturelle Rustfeasibility. Sie testet keine echten Drawbits, Rejectionimplementation, gefälschten Worldidentitäten, nativen Kapazitäten oder reale Worldhandoff. GEN-06.2 muss gerade diese offenen Fälle samt Consumerreseed/zweitem RNG/reset/upgrade, fremdem Katalog/Key/Identity/Sequenz, jeder Seed-/Core-/Candidate-/Dispatch-/Completiongrenze und Debug/Release im **wirklichen** privaten Implementierungskern diskriminieren. Die Contractclosure wird durch concrete ownership und genuine Originalerwartungen begründet, nicht durch die synthetische Probe ersetzt.

## 4. CR61-B2/B3 und H1/H2

**B2 geschlossen:** FeatureSorter-Discovery verwendet Object2IntOpenHashMap und equals/hashCode. Erst der spätere per-step `Util.createIndexIdentityLookup` verwendet Identität. BiomeGenerationSettings.hasFeature ist equality-Mitgliedschaft kanonischer Records. Die frisch ausgeführten distinct-equal-clone- und self-cycle-Controls bestätigen beides in beiden Originalen. Alle 164 Indexrecords bleiben unverändert, einschließlich underwater_magma6/26, ore_clay6/27, disks6/29..32, emerald6/33 und infested7/4. Kein ore-only-, alphabetischer oder biomlokaler Ersatzsorter.

**B3 geschlossen:** Revision2 beschreibt die tatsächliche Beobachtungsoberfläche korrekt. Je Original 60 controlled placements, 933 Candidates, 31.397 Nettosnapshotzellen; **0 S-Events**. Beobachtet: 19.216 öffentliche erfolgreich zurückgekehrte R-Drawcalls, 32.657 W-Guards, 2.413 H-Gates, 933 B-Biomqueries und 1.150 C-Chunkrequests. Direkte Section-/Bulkreads, Sectionwrites, acquire/release und volle innere Core-/Rejectionchronologie liegen außerhalb des Hooks. Die separaten16 Neighborcallbackproben füllen diese Lücke nicht. 51 unterschiedliche aufgezeichnete Proxytraces sind keine 51 vollständig beobachteten Algorithmuszugriffstraces. Snapshotdeltaordnung ist keine Writechronologie. Das fehlende Instrumentieren bleibt GEN-06.3-Präparationgate und wird nicht zum falschen writerfreien Primitiverequisit erklärt.

**H1 historischer Quellenbefund korrigiert:** Ganzer unveränderter Helper SHA `681cfa680f3a036b045051327eddca170f55cb69e1414ea47ab7ab63758146e7` enthält tatsächlich `Math.abs(bx(a[0])>>4)<=1&&Math.abs(bz(a[0])>>4)<=1`. Die Erstreviewbeschreibung `abs(block)>>4` ist unzutreffend. Hier wurde kein historischer Helperfehler durch Mutation behoben. Der alte Review bleibt readonly; die Korrektur steht ausschließlich in diesem neuen Bericht.

| Blockwert | arithmetischer Chunk | Ergebnis |
| ---: | ---: | --- |
| −33 | −3 | false |
| −32 | −2 | false |
| −17 | −2 | false |
| −16 | −1 | true |
| 31 | 1 | true |
| 32 | 2 | false |

18 frisch ausgeführte X-/Z-/Diagonalcontrols sind normal/−O bytegleich, neue signed-Intervallpolicy = historische Expression. Der separat bezeichnete falsche abs-before-shift-Control lässt −17 in drei Rows zu und wird erkannt. Dies ist Guardpolicy-/Javaarithmetikevidenz, kein ausgeführter Minecraft-Stageguard.

**H2 bleibt spätere offene Pflicht:** Input+private Copy allein = 7.077.888 B Statepayload. Die alte4-MiB-Regionbehauptung ist zurückgenommen. Kein neuer Regioncap ist angenommen; Maps/Counter/Fluidflags, geordnete Postprocessinglisten mit Duplikaten, Borrowedbiomcapacities, Input-/Result-/Reservationobjekte, Alignment und Kopierarbeit müssen später gemeinsam hinzugerechnet und admitted werden. Das ist außerhalb dieser writerfreien Primitive und wird nicht als geschlossen ausgegeben.

## 5. Frische Originalausführung und eigene Kontrollen

Byteidentische Kopien der gepinnten Revision-2-Executiontools stehen mit Herkunft in `tool-provenance.json`. Sie wurden **hier neu ausgeführt**; ihre vorhandenen Autorlogs wurden nicht als eigene Tests gezählt. Der Secondchecker ist ebenfalls eine geprüfte Kopie des vorhandenen unabhängigen Checkers, keine neue Erfindung dieses Reviewers. Neue eigene Programme sind Census, Accountingmodell, Rustsketch/-mutanten, Summary-/Integrity-/Freezetools.

| hier frisch ausgeführter Umfang | Ergebnis |
| --- | --- |
| Originalhelper und vollständige Projection, normal/−O | 1.218 Rows je Original, bytegleich zum historischen Corpus |
| Seed-/Wrapper / Height-/Count / Rejection / Doublemax | 48 / 60 / 52 / 1 / 1 genuine Originalrows |
| Tagrule / Discard / canPlaceOre / Scattered / topFeatureexception | 400 / 17 / 16 / 16 / 1; deklarierte kontrollierte Inputs |
| Extras je Original/Pythonmodus | 128 genuine Rows, unverändertes Originalreplay |
| Guardreihe je Pythonmodus | 14 Refusals, 3 positive Controls |
| Autorenprojection-Mutationen je Pythonmodus | 7 Refusals |
| Secondchecker-Mutationen je Pythonmodus | 11 semantische +5 Framingrefusals; jede unmutierte Row akzeptiert |
| eigene direkte ZIP-/Modellkontrollen | Census normal=−O, Bindmodell normal=−O, 19 Negativkontrollen |
| eigene H1-Grenzen | 18, normal=−O |
| eigene Rustsketchtests | 3 Debug /3 Release, 3 Compile-/3 Verhaltensrefusals |

Extras: dispatch30, target50, membership30, census1, loaded_tag3, lazy_sequence1, count_rarity_control7, controlled_exhaustion4, equality_control1, sorter_equality_control1. Tagcontrols benutzen echte geladene Originalstates; Production muss keinen solchen globalen Stateexport konsumieren. Lazyfolge interleavt echten Consumerfloat vor dem nächsten X/Z/Height. Count0 und f32-Rarityschwellen nextDown/equal/nextUp sowie echte reject-all Delegationexceptions bleiben deklarierte Kontrollfälle.

Alle60 controlled Return-/Nettoergebnisse stimmen Vanilla/Paper überein,51 Proxytraces unterscheiden sich; alle48 Seed-/Wrapperrows und Doublemax diskriminieren das Doubleprofil. Frisches javap bestätigt Vanilla `l2d/dmul`, Paper `l2f/fmul/f2d`; echter Paper-Wrapper erreicht1.0 mit zwei Corelongs. Kein direct-Xoroshiro-Lemirepfad ersetzt den BitRandomSource-Decorationwrapper.

Die60 Placements sind echte Methodenaufrufe gegen neun kontrollierte STONE-ProtoChunks mit unverändertem Defaultstatus, Proxy-Gate320 und matching genuine Biomeholder. Featureseed ist bewusst index0/step6, auch für infested; keine natürliche6/7-Decorationgenerierung. Diese Inputs bleiben explizit kontrolliert. Keine erfundene ServerLevelinstanz, Stagecompletion oder Biomezulassung.

| finales Ergebnisartefakt | SHA-256 |
| --- | --- |
| rohe Vanilla-TSV | `f420aa782636f9893a41a98beb0db20a1f6e66d5c71c2a2c81f81f3357bf11b0` |
| rohe Paper-TSV | `e4367e2dd26755eceb78be4571db4b12d620bb3ebea8161fe1e9ff881fae740c` |
| kompakte Vanilla-TSV | `73a5dfc12866e69bbec3c8c5387d7b84563629045ee591eed81efea3f54c4910` |
| kompakte Paper-TSV | `968fa576b34fa232cac3a58898fc2dd3a9623d2c423cf276db67af81b26f33ec` |
| Extras, beide Originale/Pythonmodi | `23b9d8718358b3f7f352d20be8d5dbefeac62693c8e4cd13965f5400bcfc702f` |
| H1-boundaries | `83f49a2d4a2f7ac5e137cc78d9d51d18f81f101c77a05c5c2c7a46b9503a9e33` |

Eigene Entwicklungsdiagnostik ist getrennt: erste Censusannahme zu Rohspalten wurde vor Ausgabe korrigiert; vorläufige Censusreceipts hatten nondeterministische Targetlistenordnung, finale Receipts sind nach sorted Discovery bytegleich bei identischen Werten. Eine erste Inline-empty-enum-Sketchform erzeugte eine unreachable-Warnung; der finale private Callee kompiliert warnings-as-errors ohne produktive Successvariante. Vorläufige Accounting-/Designcontrolausgaben sind keine finalen Nachweise. Nichts davon verändert historische Bytes.

## 6. Konkreter Folgeticket und unveränderte offene Gates

Root kann den **Entwurf** GEN-06.2 separat vergeben: nur akzeptierte Worldbasis, 57 exakte Paperquellen/Properties/Tags, vollständige164-Indizes/private checked IDs, strikt bounded eigener Parser/Compiler, vollständiger Bind-Debit, privater Paper-Decorationadapter und lazy sealed Session mit kumulativem Runmeter. Vanilla bleibt Testdiscriminator. Keine Produktions-Biome-/Orewrite-Consumer, keine Publicsuccess-Attrappe, kein Registryreload, neue Dependencies, Storage-/Sim-/App- oder bestehende GEN-04/GEN-05-Capänderung.

Runtimepflichten bleiben vollständig aus Revision2 §§7/9/11/12 erhalten: Debug/Release alle Originaltokens/Extras; Parser-/Schema-/UTF8-/duplicate-/escaped-duplicate-/resource-/property-/provider-/anchorfehler im selben privaten Kern; Productiondigestdrift vor Parserfehler; genaue Fehlerpräzedenz; budget0/max/max+1 und N−1/N/N+1 an Bind/Seeding/Rejections/Emission/Dispatch/Completion/Ende; Drop/Failure/Unwind/forget und Compileownership; tatsächliche source/static/borrowed/old/new/native bytes, Kapazitäten/Stack/Temporaries und cold success/refusal Allocation mit unabhängiger positiver4096-B-Kontrolle. Whole-World, fmt, strict all-target Clippy und warnings-as-errors rustdoc erst auf eingefrorener akzeptierter Archivbasis plus enumeriertem Runtimeoverlay. Dieser Review führt keine unrelated Wholeworkspacechecks gegen Storage-WIP aus.

Unverändert offen und separat zu lösen:

1. GEN-06.3-Präparation: transparente komplette Section-/Bulkreads, Writes, acquire/release und Wrappercore-/Rejectionchronologie; unbeobachteter Originalkontrolllauf mit gleichen ganzen Snapshots. Geometry524.288/Attempt,8-KiB-Geometryscratch und27 Handles sind spätere strukturelle/ungemessene Ziele, keine vollständige Work- oder Nativeabnahme. Targetmismatch ohne Draw, gleicher Zielstate/true ohne Nettodelta, beide Targets, Wasser/Luft, alle Nachbarrichtungen/Shortcircuit, chance0/1/equality, Gateequality/-absence, alle-size-Doubles vor Write, Culling/tie und Maskmark trotz Guardfalse bleiben weitere erforderliche Kontrollen.
2. Echte Producer: CARVERS nicht angenommen; vollständige erhaltene Biomearrays/-membership, OCEAN_FLOOR_WG und alle finalen Maps samt tatsächlicher State-/Fluid-/Leavesklassifikation, Nachbarstages/-versionen und vorhergehende Decorations/Strukturen. Stored-gezoomter Featurebiompfad bleibt getrennt vom Direct-Carver-Biomesourcepfad. Surface oder fixturegewählte Holder ersetzen keinen Producer.
3. Vollständiger Decorationdriver/Whole-FEATURES: alle11 Steps und kausale Nicht-Ore-Einträge, echte globale Indizes, CARVERSradius1/STRUCTURE_STARTSradius8 und tatsächliche Schedulercompletions. Genuine gestarteter bzw. gleichwertig vollständig initialisierter Originalserver-Lifecycle, declare-before-run Seeds/Presets/Nähte/Edges/gezielte Biome, gepinnte Config/WorldOptions/Packs/Plugins/Featureseed-/Blendingdaten, vollständige3×3 vorher/nachher Snapshots, Prime-/Feature-/Core-/Section-/Map-/Counter-/Entity-/PP-/BorderTicktrace und unbeobachtete Kontrollgeneration. Forward/reverse Neighborcenterhistory bleibt separat; keine Statussetter oder erfolgreichen Leersteps.
4. Region/adoption: vollständiger H2-Cap samt Work, neun private versionsgebundene Puffer, ein geschlossenes Result/Commitmarker. Sim reserviert alle neun Owner über Shardgrenzen, prüft gesamten Versionsvektor und adoptiert atomar an Tickgrenze. Missing/duplicate/foreign neighbor, seed/settings/source/geometry/status/map/classification/version und spätes Exhaustion/Konflikt liefern kein Teilresult; alle positiven/negativen Chunk-/Shardnähte und Bauhöhe−65/−64/319/320 bleiben Tests. WG-Maps werden nicht aus Endstates bequem neu berechnet; geordnete Postprocessingduplikate bleiben erhalten.
5. Keine Scattered-Nether-/End-, Lighting-, Survival-, Java-Plugin-, Persistenz-, Performance-/RSS-/TPS- oder Appaktivierungsabnahme. Ein endlicher Runmeter garantiert keinen Erfolg für alle Seeds; genuine bounded Exhaustion ist Teil der checked Policy.

## 7. Finaler eigener Freeze und Reproduktion

Alle neuen Dateien liegen ausschließlich unter `.codex-tmp/gen06_1-r2-independent-review/` plus diesem Bericht. Finale Evidenz: `normal/`, `optimized/`, beide `fixture-*`, `probe-*-complete`, `guards-*`, `author-mutations-*`, `own-mutations-*-complete`, `bytecodes/`, `h1-*-final`, `tokens-*`, `census-*`, **`accounting-final-*`**, **`design-controls-final*`**, finaler Sketch samt Lock und `debug-final.log`/`release-final.log`, akzeptierte Worldkopie, Quellen-/Input-/Toolchainprovenanz, finale Summary-/Audit-/Freezetools/-receipts. Werkzeuge/Programme sind enumeriert, keine optionalen Outsideoverlays. Finale Mitgliedschaft steht vollständig im Manifest.

Reproduktion nur in frischen Namen im eigenen erlaubten Area: kopierte `reproduce.py run/generate/validate/mutations`, `probe.py`, `guards.py`, `check_tokens.py` nach vollständigen Projections, `bytecodes.py` und `h1.py`, normal/−O; eigener `census.py OUT` und `accounting_model.py OUT` normal/−O; dependencyfreier Sketch mit der oben genannten Cargozeile. Canonical Source-/Input-/Membership-/ZIPshadow-/JVM-CodeSourceguards bleiben aktiv. Require-basierte Pythonchecks bleiben unter `-O` wirksam. Timestamped JVMlogs sind einzeln gepinnt, keine differentialen Erwartungsbytes.

`audit-final-normal.json`/`audit-final-optimized.json` sind frische abschließende historische Rehash-/Membershipreceipts mit expliziter privater accepted-Worldauthority; die Live-World-Verweigerungen stehen separat wie oben beschrieben. Der eigene `freeze-manifest.json` enumeriert diesen Bericht und finale Evidenz mit jedem Hash/Byteumfang. Cargo `target/**`, Pythoncaches und vorläufige Entwicklungsdiagnostik sind explizit ausgeschlossen; ihre Pfade/Status bleiben im Freezeprotokoll bezeichnet. Manifest und äußere `hashes.json` sind zur Vermeidung eines zirkulären Hashes ausgeschlossen. Die äußere Receipt nennt Bericht-/Manifest-/Summaryhash und Datei-/Bytecensus; der abschließende Verifier prüft tatsächliche Membership, alle Frozenbytes, alle Provenanzinputs und historischen Pins nochmals normal/−O. Der volle Manifesthash wird Root separat mitgeteilt, ohne diesen Bericht nach dem Freeze umzuschreiben.

**Disposition:** keine neue Blocking-Contract-Fundstelle. B1/B2/B3 geschlossen, H1-Beschreibung ausdrücklich korrigiert, H2 und alle Runtime-/Producer-/Whole-stagegates erhalten. Empfohlene Scratch-/Peakziele sind Rootentscheidungen und spätere messpflichtige Grenzen. CLEAN_CONTRACT/CLEAN_ORACLE bleibt ausschließlich das Urteil über diese kontrollierte Vorbereitung.
