# GEN-03.1 — Original-Blockstates und Heightmaps: Oracle und getrennte Verträge

Lieferung an Root, 12. Oktober 2026. Dies ist eine writerfreie Vorbereitung zur
unabhängigen Prüfung. Sie liefert genuine Originalmethodenbeobachtungen und
konkrete Implementierungsentwürfe, keine akzeptierte Rust-Runtime und kein
vorweggenommenes CLEAN-Review. Verbindlicher Auftrag:
`GEN-03_1_BLOCKSTATE_HEIGHTMAP_ORACLE_TASK.md`, SHA256
`4f41ecd24a86443db7ae823b960c78b7989e60f968230520c9f63184b7fa74e3`.

## 1. Basis, Scope und Herkunft

Readonly Authority ist ausschließlich
`.codex-tmp/gen05_11-runtime-root-base-storage47e/archive`, main
`47eadc4c14813c201184b552a87ca7b9c6f97db4`, akzeptierte World
`a912b1c2aa1c0b3de7510156c1b195a506e2e65b`. Der Basisfreeze hat SHA256
`58ba13d6418133bb49a154682ba1d82533f63a5e6184c6c2a8b07c6c15a47a75`.
Alle 1.490 Dateien, darunter 324 World- und 93 Storage-Dateien, wurden vor den
Proben vollständig und nach den Läufen erneut bytegenau geprüft.

Gelesen wurden aus dieser Basis AGENTS, CLAUDE, PAPER_REFERENCE, Registry-/World-
INVARIANTS, Architekturüberblick, ADR-0001/0002/0009, GEN-06.1 revision2 und dessen
unabhängiger Review, GEN-06.2/06.3-Aufträge, Registry-State-/Buildgenerator,
World-Heightmap-/Statecode und Packingcaller. Shared World/GEN-05.11 sowie
Ore-/Carver-WIP haben keine Authority. Koordination verbleibt bei Root.

Geschrieben wurde ausschließlich diese neue Datei und
`.codex-tmp/gen03_1-preparation/**` (im Folgenden `A/`). Keine Gitoperation,
kein Agentstart, kein shared Build, keine produktive oder Dependencyänderung.
Original-JARs, Sources und historische Fixtures/Freezes wurden nur gelesen.

| Originalpin | Wert / SHA256 |
|---|---|
| Minecraft / Protocol / DataVersion | 1.21.8 / 772 / 4440; echte WorldVersion-Getter bestätigt |
| Paper | Build60, Commit `29c8822d90899c89d2689338e81a98f690bcba12` |
| Vanilla inner | `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250` |
| Paper inner | `c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507` |
| Paper outer | `8de7c52c3b02403503d16fac58003f1efef7dd7a0256786843927fa92ee57f1e` |
| Official mapping | `e373ba569839e6f3c7bd4880247f3e85577d92cde2c605adfa2024eb9cae503e` |
| inputs.json | `a4548b9cfa25a52c0761ef5004bad45fcf0617cc2e51bb5506437e1afab83d99` |
| tool-pins.json | `6321c9485e5130260b85f09109c3bee142bf1847d3e21c52c62f06fa48c32a22` |

`inputs.json` pinnt 4.438 ganze Inputs. Membership umfasst die 454 Dateien von
JDK21.0.12.1+1, Vanilla 45 Runtimeinputs (Bundle + inner + 43 Libraries;
Bundle nicht im Classpath), Paper 120 (inner +119 Libraries; outer separat),
918 vollständige Source-/Featurepatchdateien und historische GEN-06.1-r2-
Freezes mit 366 beziehungsweise 494 Dateien. Weitere 1.972 Pythoninputs
umfassen Interpreter, libpython und stdlib-Sources/Extensions; generierte
Bytecodecaches sind ausgeschlossen, siehe `python-toolchain.json`.

`source-review/review.json` bindet 62 vollständige Sources (31 base/applied-
Paare), 24 vollständige javap-Exporte sowie alle relevanten Patchpins. Geprüft
sind die vollständigen betroffenen Methoden samt relevanten Callees; die
Membership aller Featurepatches behauptet kein Gesamtaudit fremder Moonrise-
Pfade. Paperoptimierungen werden durch echte Paperaufrufe beobachtet.

## 2. Genuine Census und Identität

`HeightmapOracle.java` ist ein JDK-only Reflectionhelper mit Official-Mapping-
Symbolprojektion. Er bootstrapped die echte Registry und echte Biome-/Tag-
Registries aus Originalpacks. Vor Bootstrap/TSVöffnung prüft er CodeSource der
Hilfsklasse, 47 deklarierter Originalklassen und relevanter Libraries; danach
auch die tatsächlichen Objektklassen. Die 341 Herkunftszeilen je Original
bezeichnen insbesondere alle sechs tatsächlichen Predicateobjekte.
Nur JVM-Lambdaadresssuffixe `/0x…` werden aus Herkunftslabels entfernt.

Jeder State stammt aus echten StateDefinitions und wird gegen die vollständige
`Block.BLOCK_STATE_REGISTRY`-Membership geprüft. ID, Defaultstate, Canonicalname,
Property.getName(value), isAir, blocksMotion, getFluidState, Fluidtype/-properties
und alle `Heightmap.Types.isOpaque().test(state)` sind originale Methoden.
Leavesklassifikation benutzt den tatsächlichen LeavesBlock-Typ. Kein
nachgeschriebener Java-Predicate-/Heightmapalgorithmus liefert Expectedwerte.

Je Vanilla/Paper: **1.105 Blöcke, 27.946 States, sechs Types, 167.676 echte
Heightmap-Predicatecalls**. Vollständiger FerrumC-Join über Canonicalname und
sortierte vollständige Properties: **27.946 Joins, null ID-, Default- oder
Identitätsabweichungen**. Vanilla/Paper haben null State-/Predicateabweichungen.
Numerische Alignmentannahmen allein wären hierfür unzureichend.

| Originalfact / Type (tatsächliche Enumreihenfolge) | Truecount |
|---|---:|
| default | 1.105 |
| air | 3 |
| motion | 23.659 |
| leaves | 308 |
| fluid_empty | 18.254 |
| 0 WORLD_SURFACE_WG | 27.943 |
| 1 WORLD_SURFACE | 27.943 |
| 2 OCEAN_FLOOR_WG | 23.659 |
| 3 OCEAN_FLOOR | 23.659 |
| 4 MOTION_BLOCKING | 24.154 |
| 5 MOTION_BLOCKING_NO_LEAVES | 23.846 |

Beobachtet sind 20 vollständige Fluididentitäten, keine Enumeration der gesamten
Fluidstate-Registry. Lexikalisch lokale Fluid-Slots sind keine originalen Wire-
IDs. Air ist ID0 `minecraft:air`, ID14013 `minecraft:void_air`, ID14014
`minecraft:cave_air`. Alle acht Snowlayers liefern blocksMotion=false; daraus
folgt keine Collision-/Support-/Light-/Occlusionklassifikation.

Die akzeptierte Flatregel `id != 0` weicht über alle States bei Surface/WG in
2, OceanFloor/WG in je 4.286, MotionBlocking in 3.791 und NoLeaves in 4.099
States ab. Registry boundingBox=`block` weicht in **2.862 States** vom echten
blocksMotion ab. Vollständige verlustlose Differenzlisten liegen in `analysis`.
Die fünf akzeptierten Flatstates air/bedrock/stone/dirt/grass bleiben bezüglich
der beiden aktuellen Maps korrekt; beliebige Pflanzen-/Caveair-/Fluidstates
dürfen diese eingeschränkte Aussage nicht erben.

## 3. Kontrollierte echte Chunks und Originalsemantik

`declarations.tsv` deklariert 222 Operationen in zehn vollständig konstruierten
echten ProtoChunks. Der Originalconstructor erhält echte BiomeRegistry,
UpgradeData.EMPTY und originale Geometrie; der unveränderte Defaultstatus EMPTY
wird geprüft. Kein Unsafe, Statussetter, uninitialisiertes Objekt oder Fake-
ServerLevel. Kontrollierte Sectionwrites, manuelle echte Heightmap.update-
Aufrufe und echte ProtoChunksetter/Lazygetter bleiben als verschiedene Ereignisse
erkennbar. Diese Chunks sind keine erzeugten natürlichen Stageinputs.

40 deklarierte Statevarianten decken Air/Caveair/Voidair, Stone, Pflanzen,
Cobweb, Bamboosapling, Water/Lava levels0/7/8/15, wet/dry Leaves persistent±,
wet/dry Slabs top/bottom/double, acht Snowlayers, Tallgrasshälften und wet/dry
Ladder ab. Hinzu kommen Placement ober-/unterhalb, Replacement, Removal mit
Topdownrescan, min/max-Edges, Rawcontrols und Geometrien −128/256, −64/16, 0/16.

Je Original enthält der Trace **836 Zeilen**: C10 Chunkdeklarationen, D222
Operationen, W194 Sectionwrites mit Previous-ID und tatsächlichem Readback,
U114 originale updatereturns, P8 originale Protosetterreturns, Q6 echte Lazy-
Chunkheightqueries, S282 vollständige Mapsnapshots. Jeder Snapshot enthält
alle 256 Spaltenwerte first/highest und den kompletten Rawvektor:
**72.192 Snapshotspalten, 144.384 first/highest-Werte je Original**.

| Befund durch Originalmethoden | Konsequenz |
|---|---|
| firstAvailable = gespeicherte relative Höhe + minY; highestTaken = first−1 | Gettertypen dürfen nicht verwechselt werden |
| Leer bei minY−64: first−64, highest−65, raw0 | Leer ist kein y0 und keine beliebige Air-ID |
| Oberster Block y319: first320, highest319, relative384 | relative384 ist zulässig |
| 9 Bits, sieben Werte pro Long, 37 Longs je Map | nonspanning Packing; z×16+x |
| Original setRawData prüft Länge; akzeptiert relative511 sowie gesetztes Padding | kontrolliert first447/highest446; keine zulässige Worldadmission |
| falsche Rawlänge führt zu originalem Primefallback | zukünftiger Inputparser soll explizit verweigern |
| hasPrimedHeightmap prüft Mappräsenz | kein Freshness-/Lifecyclebeweis |
| reprime_stale: Stone y0 primen, direkt entfernen, erneut primen | alle sechs Maps behalten first1/highest0 |

Originalprime startet am höchsten gefüllten Sectionbereich, überspringt Air und
arbeitet mit Mapreferenzliste/Iterator. Nicht gefundene Referenzen können über
Spalten bestehen bleiben und anwachsen. Ein zukünftiger fixed-mask Scanner
liefert bounded Outputsemantik auf frischen Maps; er behauptet weder identische
Javaarbeit noch einen allgemeinen „stale repair“. Update übergeht Änderungen
weit unterhalb der Spitze, hebt passende Placements oberhalb an und rescant beim
Entfernen/Ersetzen der Spitze nach unten. True/false-Returns sind mit eingefroren.

## 4. Lifecycle: Sourcebefund, Teilbeobachtung und offene Abnahme

Echte ChunkStatus-Getter liefern EMPTY bis SURFACE die beiden WG-Maps;
CARVERS bis FULL die finalen vier. Echte EMPTY-Protosetter aktualisieren nur WG;
bereits angelegte finale Maps bleiben im kontrollierten Versuch stale.
Lazy ChunkAccess.getHeight wurde echt ausgeführt, einschließlich Maskierung
von x/z; er liefert highestTaken. WorldGenRegion.getHeight addiert laut
vollständig gelesenem Caller eins und liefert firstAvailable.

Vollständiger Sourcebefund: NOISE.doFill schreibt direkt Sections und aktualisiert
beide WG-Maps; Surface-BlockColumn benutzt den Protosetter und liest SurfaceWG
mit +1. generateFeatures primet die finalen vier vor Decoration/border ticks.
WG-Maps sind historische Generationsdaten und dürfen nicht bequem aus finalen
Endstates neu berechnet werden. Ore.place/doPlace liest OCEAN_FLOOR_WG über
WorldGenRegion und schreibt unmittelbar Sections ohne Heightmap.update.

Begrenzter zusätzlicher Oreabgleich: 30 **akzeptierte** GEN-06.1-r2-Pläne,
24 tatsächliche Configured-JARresources, drei tatsächliche Tags; 25 Targetstates
und Tagstatecounts 8/4/4. `ore-facts` enthält 82 Zeilen über beide Originale,
41 je Profil. Alle Targets/Replaceables sind not-air, motion=true, leaves=false,
fluid_empty=true und für alle sechs Maps true. Direkter Austausch zwischen
diesen admitted States erhält deshalb die Predicatebelegung. Das beweist keinen
kompletten Ore-/Region-/FEATURES-Lifecycle und verwendet kein Ore-WIP als Authority.

**Offene echte Callerfähigkeit:** Kein vollständig initialisierter ServerLevel,
Lightengine oder echter Chunkdependencyholder-Lifecycle wurde bereitgestellt.
LevelChunkkonversion/Liveupdates, INITIALIZE_LIGHT/LIGHT, WorldGenRegioncaller
und tatsächliche NOISE/SURFACE/CARVERS/FEATURES-Stagecompletion wurden daher
nicht ausgeführt. Source-/Statussets sowie kontrollierte EMPTY-Methoden ersetzen
diese Abnahme nicht. Survival, Natural-World, whole-FEATURES, Lighting, Plugins,
Performance und Appaktivierung sind weiterhin gesonderte Aufgaben.

## 5. Mechanik, Negativkontrollen und Freeze

Frische Normal- und Python-`-O`-Läufe kompilieren den Helper getrennt und führen
Vanilla/Paper tatsächlich erneut aus. Je Profil sind alle sechs TSVs, Helperclass
und Manifest bytegleich; ebenso alle sieben Analysefiles, Policy- und Oreoutputs.
Vanilla/Paper sind states/types/census/statuses/trace bytegleich; Herkunftsklassen
unterscheiden sich erwartbar durch Obfuscation. JVMdiagnostik hat Zeit/Pfade,
ZIPstimuli nur unterschiedliche Erzeugungsmetadaten; Membership/Payload wurden
verglichen. Keine Original-TSVnormalisierung. Receipt:
`55abcdf6e1bdf83a2e145e89f855413451b7c372fc5e0cd3a3c218110b5cb493`.

Der unabhängige Validator prüft exakte Felder/Framing, Integer/Boolformate,
vollständige State-/Type-/Originmembership, Propertyprodukte, eindeutige IDs,
Defaults, Trace-/Declarationzuordnung, Reads und Rawpacking. Expectedpredicate-
und Updateresultate stammen ausschließlich aus vollständig verglichenen,
unabhängig gepinnten Originalausgaben, niemals FerrumC-/Formelresultaten.

Je Modus bestehen 24 Mutantenverweigerungen und zwei vollständige Original-
Positivkontrollen: fehlende/doppelte States, IDs, Canonicalnames, fehlende/doppelte/
geänderte Properties, Predicates, Bools, Defaults, Types, Herkunft, Truncation,
Trace-/Spaltenmembership, Packing und updatereturns. Außerdem 16 Guardrefusals
und drei Positivkontrollen: fünf echte Javaenv-Entrypointrefusals, drei deklarierte
Digestfaults, JDKmembership, ZIPshadow sowie sechs echte JVM-CodeSourcefaults
(Original/Helper/Library je Profil), letztere vor jeglicher TSVausgabe.
Alle Checks benutzen explizite Verweigerung, keine unter `-O` entfallenden asserts.

| Artefakt | SHA256 |
|---|---|
| states.tsv (beide Originale) | `078215ee6f0ee1cca461b280a6d4666330c0b042db21e542c1ffd6c3b9719eb7` |
| trace.tsv (beide Originale) | `f936c3fcfc58cb1a9b08a025023dfa10f1e324c574e01a24854047f3505d1e62` |
| identity-joins.tsv | `adb062807b6481acf813bd7358ba2863f04d3df6af7d8bfe00ba91fa4a14e62b` |
| predicate-table.u16le | `95e27539aff80d384a7d9841454210da95ceb2ea8c56353d37b19c7d9cbb2701` |
| fluid-identities.tsv | `e6ad80a42eb37d4e23fe9897e63f66494f05773cf2c9af26e4b1327fbb9edb0c` |
| bbox-motion-differences.tsv | `04e87a8607ce139470b46ecc7837a22633ace71d33b1b8e0060e1d456f2f53be` |
| flat-predicate-differences.tsv | `5559631b50095a365230d654f5d283f81b0d58c78733c428530701d66b36755f` |

`A/README.md` gibt vollständige Reproduktions-/Prüfkommandos. `hashes.json`
bindet diesen Vertrag und den vollständigen `freeze-manifest.json` ohne
zirkuläres Selbsthashing. `verify.py` rehashed normal/-O finale Membership,
alle Inputpins, vollständige akzeptierte Basis und historischen Freezes,
Sourcekopien, Tool- und Originaloutputpins. Entwicklungs-/Fehlläufe sind explizit
aus Acceptance ausgeschlossen. Neue Reviewoutputs müssen eigene frische Ordner
nutzen und dürfen den Freeze nicht ersetzen.

## 6. Entwurf A: Registry-only Producer

Rootentscheidung/anschließende unabhängige Prüfung erforderlich; alle folgenden
Kapazitäten sind **neue Zielvorschläge**, keine nativen Messungen und keine
Änderung bisher akzeptierter GEN-04/05/06.2-Limits.

1. Einen gepinnten Censusinput mit Version-/Provenanceheader übernehmen; Hash
   und Längengrenze vor Parsing prüfen. Keine JVM oder externe IO in Rust-Runtime.
2. Bounded Streamingparser, vollständiger Canonicalname+Properties-Join gegen
   akzeptierte Registrydaten; exakte ID-/Default-/Type-/Membershipbestätigung.
   Jede ID genau einmal über festen Bitmapnachweis; unknown/extra/missing oder
   falsch sortierte Properties sind Buildfehler, nie Air-/Stone-/Solidfallback.
3. Erst nach vollständiger Prüfung eine geschlossene statische u16-Tabelle
   emittieren: bits0..5 Originalmapflags, 6 air, 7 motion, 8 leaves, 9 fluid_empty,
   10..14 lokaler Fluididentitätsslot0..19, 15 default. Mapping/Slots separat
   unveränderlich dokumentieren. Tabelle enthält 27.946×2 = **55.892 Bytes**.
4. Opaquer geprüfter StateID-Typ mit fallible Admission; keine öffentlichen
   Felder, Default-/unchecked-/Serde-Abkürzung. Allocation-/IOfreie Facts-
   Getter und Typenum; explizite Konversion aus bestehenden WorldIDs.
   Bestehende Flatdefaults bleiben eigene bisherige Verträge.

Buildinputlimits: Census≤8MiB, akzeptierte blocks.json≤2MiB, Datensatz≤512B,
exakt27.946 States/1.105 Blöcke/sechs Types, ≤7 Properties, Canonicalname≤64B,
sortierte Properties≤192B, Propertyname/value≤32B, Fluididentität≤64B.
Gemessen: states.tsv4.690.854B, maxName44B, maxProperties144B, max7Pairs,
maxKey20B/maxValue27B, insgesamt124.892 Paare. Keine neue vollständige DOM-
oder Namenduplizierung; Generatorbuffer und Parserloops müssen begrenzt sein.
Buildarbeit ist ebenfalls endlich zu schließen: höchstens10MiB Inputbytes,
27.946 Statezeilen, 195.622 Propertypaarprüfungen (27.946×7), 20 Fluidslots,
55.892 Tabellenoutputbytes; ein binärer Join gegen bereits geprüfte sortierte
Identitäten braucht höchstens15 Vergleiche je State, jeder über höchstens256B
Name+Properties. Keine Hashcollisionloops, frei wachsenden Produkte oder
unbegrenzten Retrypfade. Diese oberen Iterationsgrenzen sind ein Entwurf;
der spätere Generator muss Parser-/Hash-/Emissionarbeit vollständig zuordnen.

Zusätzliche statische Rechnung auf 64bit: Tabelle55.892 +20 &str320
+Fluidstrings831 +sechs vorgeschlagene 24B-Deskriptoren144 +Mapnamen94
= **57.281B**, Ziel≤**65.536B** einschließlich Alignment/Provenance.
Vorhandene Registrymetadata sind separat; kein Gesamtregistry-/RSSclaim.

Zusätzliche Buildkapazitäten: Kandidat55.892, Bitmap3.494, Hashbuffer4.096,
Row512, sieben Paare×32=224, 20 Fluidslots×64=1.280, Generator8.192,
Parser-/Hash-/Joinlocals/Alignmentziel8.192 = **81.882B**;
inkrementelles Peakziel≤**131.072B**. Bereits geparste Inputs und bestehende
Metadata müssen zusätzlich nach ihren tatsächlichen Kapazitäten gemessen werden.
Diese Rechnung ist weder totale Buildmemory noch JVM-/Rustnative-Messung.

## 7. Entwurf B: World-only Consumer nach akzeptiertem Producer

Separate Worldaufgabe nach Registryreview; keine Registrybuildänderung im selben
Ticket. Pure Data, bestehende Dependencyrichtung, keine Threads/IO/Packets.
Produktionsgeometrie zunächst allein min−64/height384; weitere Geometrien sind
hier kontrollierte Originalproben, kein freigegebener dynamischer Vertrag.

Sechs feste gepackte Maps: je 37×8=296B, zusammen **1.776B Payload**.
Retainedziel gesamtes Mapset inkl Metadaten≤**2.048B**, zwei gleichzeitig
≤**4.096B**, alle zusätzlichen Scratch-/Stack-/Temporaries≤**512B**,
standalone old+candidate+scratch≤**4.608B**. Keine Vec pro Spalte, kein
Option<i32>-Heightcache und keine versteckte dritte volle Mapsetkopie.
Chunkinput-/Kandidatenbuffer, Registry, Biomes, PP und Region bleiben separat;
die Ziele schließen weder H2 noch einen ganzen Regioncap.

Consumeradmission verlangt geprüfte StateIDs, typed LocalColumn und geprüfte
Geometrie/Kindmask. Rawimports exakt37words, relative0..384 und null Padding
(auch ungenutzte letzte Slots) prüfen. Strengere Range-/Paddingverweigerung ist
explizite neue FerrumC-Policy gegenüber lenienten Originalmethoden.
Getter unterscheiden FirstAvailable, HighestTaken und Empty; invalid/missing
ist ein Fehler, keine Nullhöhe. Originalmaskierung gehört in einen ausdrücklich
typisierten Worldkoordinatenadapter, nicht unchecked LocalColumnadmission.

Mappräsenz, PresentUninitialized, Primed und frische Phase sind unterschiedliche
Zustände. Priming nur frischer/fehlender Maps; historische WGmaps erfordern den
echten NOISE/SURFACEproducer. Kontrollierte Epoch-/Statuslabels autorisieren
keinen natürlichen Stagehandoff. Directwrites müssen Mapinvalidierung oder den
ursprünglichen Updatevertrag ausdrücklich besitzen. Predicateerhalt wie bei den
geprüften Oretargets ist begrenzt nachweisbar, kein genereller Bypass.

Vor Mutation vollständige konservative Workreservation, bounded Kernel,
Candidatepublication erst nach vollständiger Prüfung. Fehler erhält alte Maps;
keine Teilpublication, Meterreset, automatische Allowanceerhöhung oder neuer
zweiter Ore-Meter. Entwurfsallowance1..4.194.304:

| Kernel | Vollreservierung |
|---|---:|
| vollständige Raw-ID-Inputadmission (4×98.304) | 393.216 |
| Prime aller sechs frischen Maps | 1.584.103 |
| Update aller sechs (max383 Rescanstates je Map) | 9.260 |
| geprüfte einzelne Heightquery | 8 |
| frische Admission + Prime | 1.977.319 |

Primebound: Begin1 +Candidate1 +222wordinit +6kind +256columns
+98.304×(4+2×6) scan/read/fact/air/predicate +1.536×3writes
+1.536×4outputvalidation +publication1. Updatebound:
1+6×(8+4×383+3)+1. Alle fixed loops, Mapmaskpflege, IDs und Arrayzugriffe
müssen sich auf diese deklarierten Einheiten abbilden; falls reale kernels mehr
brauchen, neue Rootprüfung statt stiller Erweiterung. `bounds.py`/Policyreceipt
zeigt N−1/N/N+1 nur als **Policymodell**, nicht als bestandene Rustgrenztests.
Künftig reale verweigerte Pfade müssen oldbytes/Workremaining nachweisen.

Ore erhält historische WGmaps erst nach independently geprüftem Producerhandoff.
Mapvorbereitung darf nicht mitten in einer bestehenden Oresession eine zweite
Budgetquelle schaffen. Die tatsächliche Querycharge wird im separat geprüften
Oreconsumer an dessen bestehenden Meter gebunden. Keine in diesem Dokument
vorgeschlagene Zahl überschreibt dessen akzeptierte Limits.

## 8. Kleinster Split, Review und konkreter Rootentscheid

1. Unabhängiger Review dieser writerfreien Evidenz, Pins, Parser und Lücken.
2. Registry-only Ticket: Producer/API, vollständige Identitätstests, unbekannte
   IDs/Properties, Buildadmissiongrenzen, native Static-/Buildkapazitätsnachweis.
3. World-only Ticket: sechs-Kindconsumer, Packing/Sentinels, genuine kontrollierte
   Originaldifferentialtests, Work-N−1/N/N+1 und native old/candidate/scratch-
   Kapazitäten, atomic Fehlernachweis. Nur akzeptierte Registryauthority nutzen.
4. Separates echtes NOISE/SURFACE/CARVERS-Lifecycle-/WGhandoffticket und echte
   Region/Oreconsumerprüfung; FEATURES/Lighting/App bleiben eigene Acceptance.

Root muss die neuen Static65.536B-/Build131.072B-Ziele sowie World2.048B
retained/512B scratch/4.608B standalonepeak und deklarierte Workreservationen
annehmen oder mit einem neuen geprüften Vertrag ersetzen. Darüber hinaus muss
Root den echten ServerLevel-/Dependency-/Light-Lifecycle für natürliche Caller-
Abnahme einem separaten Producer zuweisen. Hierfür wird nicht gepollt: Das
vorliegende Ergebnis ist vollständig reviewbar; Runtimefreigabe bleibt offen.
