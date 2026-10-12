# GEN-03.1 Revision2 — geschlossener Registry- und Heightmapentwurf

Autorenkorrektur an Root, 12.10.2026. **DESIGN/TOOLS ZUR NEUEN UNABHÄNGIGEN
PRÜFUNG; KEINE RUNTIME- ODER CAPANNAHME.** Verbindlicher neuer Auftrag:
`GEN-03_1_BLOCKSTATE_HEIGHTMAP_REVISION2_TASK.md`, SHA256
`9ac0b0e87dd2552581e50ddc5f75ee48f4f3e953dc168df5b602040fda321766`.
Es gab keine früheren Autorenantworten. Die drei neuen r1-Findings werden unten
beantwortet; diese Revision erklärt sie nicht selbst unabhängig geschlossen.

## 1. Unveränderte Authority, Kontext und Korrekturmatrix

Readonly Basis ist ausschließlich main47eadc4c
`.codex-tmp/gen05_11-runtime-root-base-storage47e/archive`: 1.490 Dateien,
324 World, 93 Storage; `base.freeze.json`
`58ba13d6418133bb49a154682ba1d82533f63a5e6184c6c2a8b07c6c15a47a75`.
Akzeptierte Worldbytes bleiben a912b1c2. Shared World gehört CarverWorld;
OreRevision2 ist ein anderer, unangenommener Entwurf. Hier wurden weder Git,
Agenten, Cargo-/Sharedbuilds, Produktion noch Dependencies verändert.
Writes liegen ausschließlich in diesem Dokument und
`.codex-tmp/gen03_1-revision2/**` (nachfolgend `N/`). Root publiziert separat.

| Bindung | SHA256 / Umfang |
|---|---|
| vollständiger Originalvertrag, 346 Zeilen | `260ff459932d8f82f0c5c2bc30898b0de5b0e2a324939911ecafd155a45ee08f` |
| Originalauthorfreeze | `3d5211d260ebce102705f61d307cb992008d0d9dd470c3886f1a546f3165d48b`; 202 Dateien /36.215.670 B |
| vollständiger r1-Review, 317 Zeilen | `a45abd91edfad2c5fb5f7d33312e1e5a3b217daf9a09ecd52827e7b46b2a1213` |
| r1-Reviewfreeze | `62a2c62d5981b91d9d2aee1bd0f52d6983472efe2f2765a9431a52e05cce0275`; 741 Dateien /680.487.002 B |
| akzeptierter GEN-06.1-r2-Vertrag / Review | `24ab2da586620009f25bfbfd99e433215d7e8698ec76a62eaff03ec03ec74eb8` / `1b2220e923033a3bf22843a7de3a86f7f056850c2e9715f123501e8b5f100027` |
| akzeptierte GEN-06.1-r2-Freezes | `e44f7199a3f222007e9759a6fff69cf0ab030008e6adcbe003b8d03da81315a7` / `dd95a0be1baa4e176857a518978543d86427a2461e8cf796e00a365b1f6e63a4` |

Die vollen GEN-03.1-Aufträge/Verträge/Reviewtools, AGENTS, CLAUDE,
PAPER_REFERENCE, akzeptierte Registry-/World-Invarianten, Architektur/ADRs und
GEN-06.1-Korrekturkette/GEN-06.2 bleiben bindend. Tatsächliche Quellen,
Toolherkunft und readonly Referenzen sind in `N/reference-pins.json`,
`N/provenance/copied-tools.json` und den exakten Diffs gebunden. Das genaue kumulative Readscope und seine Grenzen stehen in
`N/provenance/read-scope.json`. Der ganze
Registry-`load_blocks`/`emit_blocks` und State-/Heightmap-/Packingcode wurden
gegen die akzeptierte Basis gelesen. Alte Helper/Originale/Fixtures/Freezes
werden weder repariert noch refrozen.

**Aktualisierter Kontext:** GEN-06.3 r1 ist **FINDINGS** (CR63-B1/B2/B3).
Sein neuer Compositionentwurf ist pending. Die angenommene/queued GEN-06.2
Single-key-Primitive mit **1..=1.048.576** Runallowance und ihrer sealed Lease
bleibt unverändert. Root-CI38260056240 ist grün, aber kein Beweis eines neuen
Registry-/Heightmap-/Oreproducers. Keine fremde Childauthority wird übernommen.

| Finding / genaue alte Lücke | konkrete Revision2-Antwort | neue Evidenz / verbleibendes Gate |
|---|---|---|
| **G31-R1-B1 HIGH**: kein Rawkernel; Updatecandidate/Query8/Reader/Cleanup ungeschlossen | §§4–7: vollständiger Rawimport, frisches Prime, sechs changed-word-Records, feste Reader, Operationstabellen und final infallible Commit | `N/work-*-closed-ledger/receipt.json`, `N/native-layout.{rs,json}`; echte Rust-N-/Stack-/Late-refusaltests weiterhin Pflicht |
| **G31-R1-B2 HIGH**: kostenlos angenommener sortierter Vollstateindex | §§2–3: neuer u16[1105]-Blockindex mit Sortierarbeit, voller Propertyjoin und checked Vorwärtsradix; Streamingemission nach kompletter Admission | `N/join-*-stream-complete/**`, unabhängiger inverser `N/census-*/**`; kein u16[27946]-Identityindex, Rustbuildpeak unbewiesen |
| **G31-R1-M1 MEDIUM**: zusätzliche Nested-/Emptydirectories zugelassen | §8: vollständige exakte sechs-Entry-Membership und lstat-Entrytypes vor Parsing, keine Symlinks/Specials | `N/membership-*-final/results.json`, `N/challenges-*/results.json`; beide echten Originalpositivkontrollen normal/−O |

## 2. Registry-only: Herkunft, vollständiger Join und endliche Arbeit

Minecraft1.21.8/772/4440, Paper Build60/Commit29c8822d bleiben fest.
Vanilla-inner `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`,
Paper-inner `c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`,
Mapping `e373ba569839e6f3c7bd4880247f3e85577d92cde2c605adfa2024eb9cae503e`,
JDK21.0.12.1+1. Runtime bleibt statisch, dependencyfrei und ohne Parsing/IO.

Bestehendes `load_blocks` besitzt normalisierte `Block`-/`Property`-Vecs mit
Strings und bool-Domains `[true,false]`. `emit_blocks` baut temporäre
Blocknamen-/Rangeindizes und einen wachsenden Source-String. **Keines dieser
Objekte ist ein Index vollständiger Stateidentitäten.** Die neue Phase läuft
nach write/drop dieses alten Emissionsstrings und seiner temporären Indizes.
Die normalisierten Blockmetadata werden immutable geliehen. Ihre tatsächlichen
Vec-/Stringcapacities, Header und der alte Loaderpeak werden separat inventarisiert;
die neue Phase erzeugt keine zweite JSON-DOM oder Metadatenkopie.
Vor dem bestehenden Loader muss das ursprüngliche blocks.json whole-file
hash-/längenadmitted sein. Diese Pinprüfung ist eine explizite Produceränderung.

Neue Admission: blocks.json≤2MiB, Census≤8MiB; exakt1.105 Blöcke/27.946 States,
sechs Types, 20 beobachtete Fluidklassen. Jede Censuszeile einschließlich LF≤512B,
Canonicalname≤64B, vollständiges Propertyfeld≤192B, höchstens7 Paare,
Propertyname/value≤32B, vollständige Fluididentität≤64B. Nur die originale
ASCII-TSV-Grammatik ist zugelassen: keine Escape-/Unicodealias-Dekodierung,
CR/NUL, signed−0, nichtkanonische Zahlen/Bools oder extra Felder.
Die gemessenen Domains haben maximal **26** Werte (insgesamt1.794 Properties/
5.911 Domainwerte); dieser neue geschlossene Domaincap gehört zur Rootprüfung.
Gemessen: Census4.690.854B, Name44B, Properties144B, sieben Paare,
Key20B/Value27B. Beim nächsten Profilwechsel ist neue Originalauthority nötig.

Algorithmus nach vollständigem Hash-before-parse:

1. Die geliehenen Metadata vollständig validieren: Namen-/Property-/Valuelängen,
   eindeutige Namen und Property-/Domainwerte, bool-Reihenfolge, cardinality>0,
   min≤default≤max<27.946, checked u64-Cardinalityprodukt≤27.946 und genaue
   Rangebreite. Alle1.105 Blockrangepaare auf Disjunktheit prüfen; keine versteckte
   erneut sortierte Rangetabelle. Strings werden geliehen, nicht geklont.
2. Ein neues fixes `u16[1105]` mit Metadataordinals füllen. Deterministischer
   Insertion-sort über bounded padded64-B-Namensvergleiche. Höchstens609.960
   innere Vergleiche/Shifts, zusätzlich1.104 äußere Heads und die Endheads.
   Anschließend1.104 adjacent Uniquechecks. Kein sort_unstable mit ungezählter
   Bibliotheksarbeit/Stack und kein Vollstateprodukt.
3. Jede bounded Row einmal lesen/grammatisch scannen; sieben offsetbasierte
   Paarrecords referenzieren denselben512-B-Rowbuffer. Vollständige sortierte
   Schlüssel und keine Duplikate prüfen. Bounded Namens-binärsuche: höchstens11
   Schritte plus finaler Gleichheitscheck, insgesamt12 Slots à64 Byte.
4. In **ursprünglicher Metadata-Propertyreihenfolge** jeden vollständigen
   Rowkey gegen höchstens7 Propertynames (padded32) und seinen Wert gegen
   höchstens26 vorhandene Domainwerte (padded32) vergleichen. Genau ein Name-
   und Valuematch, genau so viele Rowpaare wie Properties; unbekannt/extra/missing
   verweigert. Keine Defaultinheritance im Censusjoin. Booldigit0=true/1=false;
   Enum-/Intdigits behalten tatsächliche vorhandene Reihenfolge.
5. Vorwärtsradix: `offset=offset*cardinality+digit`, `product*=cardinality`,
   jeweils checked u64; `sid=min+offset`, product=rangebreite, sid≤max.
   Compare sid gegen Original-ID und `sid==default` gegen Originaldefault.
   Damit ist die vollständige Name+Propertiesidentität bewiesen, keine bloße
   numerische Angleichung. Kein allgemeiner BTreeMap-Override-Resolver als Ersatz.
6. Jeder ID genau ein Bitmapbit, Duplikat vor table-write verweigert.
   Alle27.946 Bits und die sechs ungenutzten Endbits prüfen; jede ID vorhanden,
   jeder Blockrange genau ein richtiges Default. Jeder u16-Flagbit wird aus den
   genuine Originalflags gebildet, mit vollständiger Fluididentität gegen20
   bounded Slots. Keine Stringhash-/Collisionretryloops.

Neue Vorwärtsroute `join_model.py` bestätigt jede Originalidentität. Die kopierte
reviewseitige inverse Radixroute `census.py` bestätigt unabhängig denselben Join
und jedes zurückdekodierte Flag. Pythonmetadata und dynamische Prüfprodukte sind
**offline Modelle**, keine freigegebene Rustbuildinfrastruktur.

Buildwork ist ein neuer konservativer **u64**-Operationszensus; sein Maximum
überschreitet u32. Er ist keine Runtime-/Oreallowance und keine CPU-/Zeitmessung.
`L(n,b)=2+n*b` enthält Eintritt/terminierenden Head; Body enthält laufenden
Head und Advance. `C32=L(32,5)=162`, `C64=L(64,5)=322`: head, linker Byte-read,
rechter Byte-read, compare, advance. Grammarkonsum hat ebenfalls fünf Aktionen:
head, read, Grammar-/Offsettransition, Bufferstore, advance. Padded Slots führen
alle Positionen aus; ungenutzte skalare Prüfschritte sind feste NOPs.

Die mathematische Blueprint legt B=1105,P=7,D=26,S=27946 fest. Vollständige
Formeln und die gleiche Phasensumme stehen in `N/join_model.py` und
`N/join-*-stream-complete/receipt.json`. Metadata: je Block10 Scalarchecks
(count/name-length/name-ASCII/property-count/min/max/default/range/product/type),
C64 Name; je Property10 Scalarchecks (name/type/length/cardinality/domain/default/
boolorder/range/ordinal/overflow), C32 Name; je Domainwert5 Scalarchecks
(length/ASCII/canonical/value/ordinal), C32 Wert; alle Domainpaare C32+2 und
Propertypaare C32+2. Alle Rangepaare sechs Slots: head, beide Rangereads,
overlapcheck, pairadvance, advance. Sort hat je outer7 Slots und höchstens
609960 inner C64+4 Slots (head/index-read/shift/advance neben Compare).

Je Row: `L(16,L(512,5)+10)` ist ein absichtlich konservativer vollständiger
512-Byte-Feldslot für jedes der16 Felder plus zehn Scalarchecks; dazu
`L(12,C64+6)` für Binärsuche/finale Gleichheit, `L(21,C32+4)` für alle
Propertykeypaare, `L(7,L(7,C32+4)+L(26,C32+4)+16)` für vollständige
Key-/Domainmatches und checked Radix, `L(20,C64+4)` Fluidmatches,
`L(16,4)` Flags, und24 feste Row-/Bitmap-/Table-/Default-/ID-Abschlussaktionen.
Die16 Radix-/Matchslots umfassen Keyordinal, Valueordinal, Matchcounts,
Missing/Extra, cardinality, digit, offsetmul/add, productmul, overflowchecks,
minadd, maxcheck und finalidentity; kein Hash oder Bibliothekssort versteckt.
Membership `L(S,5)+L(B,6)` liest jedes Bit/Default in bounded ID/Blockorder.
Descriptor/Provenance hat `L(6,128)+L(20,256)+L(4,L(32,3)+2)+16` feste Slots.
Emission umfasst `L(4096+S*8,4)+L(S,12)+L(20,12)+L(6,12)+16`: bytehead,
format/copy, sinkstore, advance plus Recordformat-/Field-/Grenzchecks.

SHA-256 wird ausdrücklich als bounded Scalarblueprint gezählt. Pro maximal
163842 gepaddeten64-B-Blöcken gilt
`L(16,3)+L(64,128)+L(64,256)+L(8,4)+16`.
Die128 Scheduleslots umfassen head/advance, vier Wortreads, sechs feste
Shift-/Rotateoperationen, vier XORs, drei Additionen, Store und Kontroll-/NOPslots;
die256 Rundenplätze umfassen head/advance, acht State-/zwei Schedule/Konstantenreads,
zwölf Rotates/Shifts, zwölf XOR/AND/NOTs, acht wrapping Adds, acht Stateassignments
und feste Kontroll-/NOPslots. Padding/endian/length/finale acht Stateadds und
Outputbytes sind in den16 Abschluss-/L(8,4)- und Bytephasen enthalten. Das sind
konservative feste Platzbudgets, **keine gemessene instrumentierte Implementierung**.
Ein Rustbuilder muss exakt diese begrenzten Operationen oder eine ausdrücklich
neu reviewed äquivalente Blueprint benutzen; ein freier Libraryhash gilt nicht.

| Buildphase / konservatives Maximum | Einheiten |
|---|---:|
| vollständige drei Chunk-IOloops | 18.678 |
| alle Deskriptoren/Fluids/vier Digests/Version | 6.310 |
| vollständige bounded Streamingemission | 1.246.344 |
| beide Inputhashes: ≤10MiB, vollständige SHA-/Paddingblueprint | 4.075.405.722 |
| u16-Indexinitialisierung | 3.317 |
| Insertionsort incl.Heads/Shifts | 198.854.690 |
| vollständige Membership-/Defaultprüfung | 146.364 |
| vollständige bounded Metadata incl.Domains-/Propertypaare | 451.399.132 |
| adjacent Uniquechecks | 357.698 |
| feste File-/Publication-/Cleanupoperationen | 16 |
| sämtliche disjunkten Rangepaare | 3.659.762 |
| 27.946 vollständige Rows und Vorwärtsradix | 2.617.981.282 |
| Syntax/Rowbufferkonsum ≤8MiB | 41.943.042 |
| Workspaceinitialisierung und Bytecleanup, jeweils79.868 | 479.212 |
| **Summe** | **7.391.501.569** |

Chunk-IO ist separat `L(2562,4)+L(2049,4)+L(57,4)=18678`: head,
ein Read/Writeversuch, exakte Resultprüfung, advance. Hash hat≤2562 4KiBreads
inklusive zwei EOFversuchen, Parser≤2049 inklusive EOF, Sink≤57 flushwrites.
Kurzes IOresult verweigert ohne Retry. IOlatency/OSimplementation bleibt außerhalb
eines CPU-/Zeitclaims. Die16 festen Fileoperationsslots sind stat/admit der zwei
Inputs, zwei hash-open/close, parser-open/close, part-create, flush, part-close,
final-admit, rename, failure-close, failure-unlink, ownership-release; nicht
benutzte Erfolgs-/Fehlerslots bleiben reserviert. Kein post-Rename-Failurestep.
Bestehender Loader/Generator und sein JSONparse bleiben **zusätzliche** alte
Buildphasen; die neue Joinblueprint gibt sie nicht als kostenlos aus.

## 3. Registry-only: Memory, Fehler und vollständige Streamingemission

Tablebits:0..5 sechs Originalprädikate;6 air;7 motion;8 leaves;9 fluid_empty;
10..14 lokale lexikalische Fluidklasse0..19;15 default. Die20 Slots sind keine
Wire-/Bootstrapfluid-IDs und keine vollständige Fluidregistry. Keine Shape-,
Support-,Collision-,Light-,Occlusion- oder Connectionauthority folgt daraus.

| Gleichzeitiges neues Buildobjekt | Bytes, Modell |
|---|---:|
| einzige u16-Candidatefacttabelle | 55.892 |
| IDbitmap | 3.494 |
| neuer u16-Blocknamenindex | 2.210 |
| Hash-/Inputfenster | 4.096 |
| einzelner Rowbuffer / sieben8-B-Offsetpaare | 512 /56 |
|20×64-B-Fluidrecords /20 u16-Längen | 1.280 /40 |
| einziges begrenztes Emissionsfenster | 4.096 |
| sämtliche Parser/Hasher/Sort/IOheaders/Callstack/temporären Locals + Alignmentpartition | 8.192 |
| **neues repr(C)-Layoutmodell, align2** | **79.868** |

Dies sind neue eigene Modellstrukturen, kein übernommener84.092-B-Reviewpeak.
Ein zusätzlicher u16[27946]-Identityindex ist verboten. Rowstrings werden nicht
nochmals dekodiert/kopiert; Pairrecords enthalten nur Offsets/Längen.
Hashstate/schedule, File-/BufWriterheaders, Error/Resulttemporaries und
Allocatorcapacities müssen im neuen **131.072-B-Peakziel** liegen; ungenutzter
Modellraum51.204B rechtfertigt keinen kostenlosen Buffer. Größere reale
Capacity als angefragt zählt vollständig; checked Before-growth/try_reserve_exact
vor jedem owned Objekt. Überschreitung/Allocationfailure veröffentlicht nichts.
Die8.192-B-Partition umfasst Locals; keine8192-B-Arena plus externe Locals.
Keine Behauptung über gesamten Buildheap/RSS. Der geliehene vorhandene Bestand
mit1.105 Blocks/1.794 Properties/5.911 Strings und seinen tatsächlichen
Capacities wird als zusätzlicher simultaner Bestand vermessen; frühere
`load_blocks` raw JSON-/RawBlock-/ranges/seen_names- und emitter-Stringpeaks
bleiben eigene vorhandene Phasen und dürfen nicht mit der neuen Phase überlappen.

Static:55.892 Tabelle +320 &str-Headers +831 Fluidstringbytes +144
Deskriptorbytes +94 Typnamen +144 Provenance/Versionbytes =57.425B;
eigenes align8-Layoutmodell **57.432B**. Neues Staticziel **65.536B** einschließlich
aller tatsächlich emitten Objekte/Alignment. Vier32-B-Digests binden Census,
blocks, Originalprovenance und Tool-/Schemaidentity; Versionrecord16B bindet
1.21.8/772/4440/Build60. Keine weiteren ungezählten Strings/Deskriptoren.

**Publication:** Erst nach ganzem Hash-/Schema-/Join-/Bitmap-/Default-/Factcensus
ein exklusiv neu erzeugtes `.part` im vertrauenswürdigen OUT_DIR öffnen.
Ganze Tabelle aus Candidate in IDreihenfolge streamen; ein bounded Formatrecord
≤512B, ein4KiB-Sinkbuffer, Tabellenwerte als acht ASCIIbytes `0xhhhh,\n`.
Types/Fluids/Deskriptoren/Provenance haben zusammen≤4.096 feste Outputbytes:
total≤227.664B. Keine weitere volle Source-Stringallocation, kein append an
den alten `emit_blocks`-String. `join-*-stream-complete/predicates.rs` ist die ausgeführte vollständige
Streamingmodellprobe (225.871B; davon2.303B Header/Metadata, größter Record163B),
einschließlich sechs Deskriptoren, vier32-B-Digests und16-B-Version. Keine
zweite TYPES-Headerarray: Deskriptoren tragen die sechs Namen. Die vier Digests
binden States, blocks, Originaloutputpins und byteidentischen Helper/Schema.
Flush/close prüfen, dann atomarer Rename genau einer vollständigen Generatordatei.
Vor jedem Fehler alter published Output unverändert; Candidate und `.part`
bounded schließen/unlinken, keine kaputte Datei als Result. Cleanupadmission
wird vor Fileerzeugung reserviert, kein unbounded IOretry. Nach Rename kein
fallibler Computationalstep. IO-Crashdurability ist keine Storageabnahme;
Cargo kann nur den vollständig benannten Output lesen. Late-write/close/rename-
Refusal, kurzes Schreiben, Native-/Allocationfailure und Cleanupfailure müssen
mit alten Outputbytes künftig tatsächlich geprüft werden.

Fehlerpräzedenz: version/profile/length/nativeadmission → vollständige Hashpins
vor parse → Metadataordinal und dort name/property/domain/range → Censusheader,
erster Rowordinal/Byteoffset, framing/fields → name → keys/values → radix/range →
ID/default → duplicate/flag/fluid → finale Missing/Extra/Defaultmembership →
Emission/IO → finaler Rename. Unknown hat niemals Air-/Stonefallback.
Productionmutation liefert `CatalogueDrift`; private cfg(test)-Descriptors
führen denselben Parser mit neuem Testpin zu Malformedfehlern. Sie sind keine
öffentliche Sourceauthority. Fehler sind bounded typed enums/Ordinals/Offsets;
kein dynamisch allokierter Fehlermeldungsstring in Runtime/Builderkern.

Neue Registry-`CheckedStateId`-Felder privat, fallible u32-Konversion, keine
Default/From<u32>/Serde/raw/unchecked-Constructorauthority. Vollständige Property-
Identityadmission bleibt Buildarbeit. Factsgetters sind feste table-reads und
allocation-/IOfrei; sechs geschlossene Kindwerte, unknown verweigert.
Bestehende World-`BlockStateId::new/From<u32>/AIRdefault` bleiben **raw** und
sind kein Checkednachweis. Konversion wird ausdrücklich gecharged; bestehende
Flatconsumers werden im Registryticket nicht still geändert.

## 4. World-only: Reader, Stateauthority, Maps und Lifecycle

World folgt ausschließlich nach angenommenem Registryproducer. Zunächst nur
minY−64/height384/24 Sections. Datenreihenfolge `relativeY*256+z*16+x`.
Keine Callback-/Traitobject-/Palette-/Nachbarsearch wird hinter einem konstanten
Stateread versteckt. `DenseStates<'a>` leiht exakt `&'a [u32;98304]` immutable;
sein privater Admissionproof bindet die komplette geprüfte Range, Registryrevision
und Produceridentity. Alle Roh-IDs werden einmal gescannt. Rustborrow verhindert
Änderung dieser Arraybytes während Proof/Readerleben; weder mutable alias noch
Default-/Serde-/public constructor kann den Proof erzeugen. Ein read berechnet
einen bounded Index, liest ein u32 und einen Registry-u16; kein Name-/Propertyjoin.
Registrytypkonversion bleibt checked/konstant oder benutzt einen explizit privaten
Registryproofgetter, der denselben Admissionproof verlangt; kein World-rawcast.

Ein bereits konkreter admitted Producer darf einen solchen Borrow liefern,
erst nach Nachweis seines gesamten ID-/Array-/Lifetimecontracts. Andernfalls
gilt die unten bezifferte Denseadmission. Der Proof ist kein neues Statearray,
sondern ein kleines borrowed Wrapperobjekt innerhalb Scratch/Headers. Seine
genaue native Größe bleibt Runtimegate. Ein neuer u16-Densearray wäre196.608B
und benötigt zusätzlich Conversion/copy/nativeadmission; er wird hier nicht gebaut.

Mapset:sechs feste37-u64-Maps,1.776B Payload. Metadaten enthalten private
Kindmaske/Mapzustände/Phase und Producer-/Owner-/Versionreferenzen. Presence,
PresentUninitialized, FreshPrimed, HistoricalWG, CurrentFinal und Stale sind
verschiedene Authority. Ein Textstatus/Epochcount/Digest oder checked Statearray
allein kann niemals Stagecompletion oder historische WGmaps erzeugen.

Rawimport braucht einen echten sealed `HistoryAuthority`-Borrow aus einem
admitted Producer/Owner, passend zu Seed/Settings/Source/Chunk/Geometry/Stage/
Versionsvektor. Hier existiert kein neuer natürlicher Producer dieses Tokens.
Private Kontrolltypes dürfen unter cfg(test) dieselben Kernel aufrufen, ihre
Outputs bleiben ausdrücklich controlled. Öffentliche Rawimports ohne Token
verweigern auch bei richtigen Packedbytes. Rawbytes beweisen lediglich Storage-
Range/Padding, nicht Gleichheit zu States oder zu vergangener Generationsgeschichte.

Prime admitted nur einen kompletten frischen Kandidaten, ausgewählte Maps
Absent/PresentUninitialized und passende FreshInputauthority. Alle unselektierten
Maps sind Absent/zero. Schon FreshPrimed/HistoricalWG/CurrentFinal/Stale in einem
zu primenden Slot → `ReprimeRefused`; kein stilles Stalerepair. Ein späteres
Merge von partial existing Maps ist eine zusätzliche Copy-/Work-/Ownershipaufgabe.
Ein importiertes komplettes Set darf selektierte und absent Slots haben, wobei
alle1536 Values/222 Wörter geprüft werden und absent Slots vollständig zero sind.

Update braucht einen sealed Einzeländerungs-/Poststateproof: dieselbe Producer-
Generation und genau diese Column/Y/old/new-Stateänderung. Dieser Proof stammt
vom tatsächlichen Writer eines privaten noch unveröffentlichten Chunkcandidates,
nicht von einer Callerbool. Er erlaubt nur die für diesen Writepfad betroffenen
Kindslots. EMPTY-Protosetter betreffen WG, Live-Levelsetter ihre vier finalen Maps;
Ore-direct-Sectionwrites dürfen keinen erfundenen Heightmapupdate erwerben.
Ein Mapkernel allein installiert keine Block-/Sectionänderung. Der spätere
Stagewriter muss Block- und Mapcommit gemeinsam vorreservieren und nach letzter
Admission infallibly unter exklusivem Owner installieren. Failure verwirft seinen
unveröffentlichten Poststatecandidate und erhält alte publizierte States/Maps.
Diese Writerkopierarbeit liegt zusätzlich in seinem eigenen geprüften Stagevertrag.

Kein allgemeiner stale-map Queryfallback oder lazy Prime beim Query. Historische
WGqueries lesen die behaltene Originalchronologie; finale States ersetzen sie nicht.
`QueryFirstAvailable`, `QueryHighestTaken`, `EmptyColumn` sind explizite Typen.
Der Worldkoordinatenadapter maskiert nur nach admitted Worlddomain/Chunkauswahl
`x&15,z&15`; strikter `LocalColumn` prüft0..15. Beide Adapteraktionen sind
im Queryplan enthalten; Missing/Uninitialized/Stale ist ein Fehler, kein0/Air.

## 5. World-only: vollständige Reservation und genaue Operationen

Arbeitseinheit bezeichnet eine deklarierte bounded primitive Aktion, keine CPU-
Instruktion/Zeit. `L(n,b)=2+n*b`: Eintritt und terminating Head, je Body der
genannte Head/Advance plus Einzelaktionen. Keine ungezählte Inhaltsloop oder
generische variabel teure Bibliotheksmethode ist zulässig. Alle Worstcases
werden **einmal vor content-dependent Arbeit** checked reserviert.
Fehlende Reserve verändert weder remaining noch alte Mapbytes; N−1 stoppt vor
dem Kernel. Bei erfolgreicher Reserve bleibt das ganze N verbraucht, auch bei
frühem Fehler/Shortcircuit. Es gibt keine Rückbuchung, Nachzählung oder zweite
Allowancereservation im Kernel. Preflight/Cleanup/finaler Commit sind enthalten.

`G=24` feste guard-Slots: Entry; minY; height; mask-range; mask-nonzero;
Column-X; Column-Z; Y-min; Y-max; Owner; Seed; Settings; Source; Chunk;
Version; Producer; Phase; Readergeometry; Readeridentity; Inputlength;
retainedbytes; scratchbytes; simultaneouspeak; operationpermission.
Nichtbenötigte Slots sind feste NOPs; string-/State-/Arrayprüfungen sind hier
nicht versteckt. Identityvergleiche sind private fixed-size checked IDs/Tokens.
Kindzustandchecks zählen gesondert in Kindmetadata/Metadata. `C=8` feste Slots:
error-enum, error-ordinal, Resulttag, Returntoken, Readerborrow-release,
Candidateborrow-release, oldownership-release, operationownership-release;
Erfolg/Refusal führt nicht benötigte Slots als NOP. Keine Bytewipe-/rekursive
Destructorloop, kein Heaperrorstring. Native Umsetzung muss tatsächliche Locals
und Initialisierungen diesen Slots zuordnen; keine separate zeroed Scratcharena.

| Recipe | vollständige enthaltene Einzelaktionen |
|---|---|
| Densebody5 | head, raw-u32-read, Registryrangecheck, checked-IDadmission, advance |
| Rawword8 | head, inputword-read, mask-index/shift/mask-build3, paddingcheck, candidateword-store, advance |
| Raw/Primevalue8 | head, word-index, word-read, shift-expression, extract, rangecheck, absent-zero-check, advance |
| Deltavalue6 (Word bereits gehalten) | head, slot-index/shift/extract3, range oder unused-zero-check, advance |
| Zeroword3 / Heightinit3 | head, zero-store, advance |
| Rawkind7 | head, presence-test, requested-state-read, statepermission, historykindpermission, candidate-state-store, advance |
| Primekindmeta6 | head, selection-test, old-state-read, reprime-statecheck, candidate-state-store, advance |
| Primeread5 | head, dense-index-expression, raw-state-read, fixed Registryfact-read, advance |
| Primekind6 | head, unresolved-bit-test, predicate-bit-test, relative-height-store, clear unresolved bit, advance |
| Pack7 | head, candidateword-read, shift/clear/new-bit-expressions3, candidateword-store, advance |
| Primewordvalidate6 | head, candidateword-read, mask-index/mask-build2, padding/unused-mask-check, advance |
| Rescan6 | head, bounded dense-index-expression, state-read, Registryfact-read, kindbit-test, downwardadvance |
| Deltabuild8 | oldword-read, clear expressions2, shifted-new-bits2, ORjoin, recordword-store, changedflag-store |
| Deltapadding4 | mask-index, bounded mask-build, changedword-padcheck, unused-slot policy |
| Deltainit11 | head, neun Stores:word/kind/index/changed/fünf Paddingbytes, advance |
| Commitkind6 | head, changedflag-read, changed-test, recordword-read, admitted fixed word-store, advance |
| Querykind6 | kind-range, presence-test, state-read, statepermission, historykindpermission, phasekindpermission |
| Queryindex5 | X-domain/mask, Z-domain/mask, column=z*16+x, word=column/7, shift=9*(column%7) |

Ein expression-Slot ist ein **explizit fester** bounded Integerausdruck, keine
variable Subroutine: denseindex=y*256+column, shift=9*(column%7), mask9/word63/
last36 aus admitted Ordinals. Token-/permissionvergleiche sind fixed private
Enums/IDs; keine Strings oder Inhaltsiteration. Updateprep22 je Kind ist genau:
head, selection-test, old-state-read, statepermission, phasekindpermission,
historytransitionpermission, word-index, oldword-read, shift-expression,
oldvalue-extract, oldvalue-rangecheck, resultheight-init, changed-init, lowguard,
newpredicate-test, raisecompare, removecompare, rescan-Yinit, destination-reference,
shiftmask-prepare, changed-result-aggregate, advance. Skipped Fälle belegen dieselben
reservierten Slots. Die sechs Zielreferenzen und Wordwerte werden vor Commit
admitted; die96-B-Localspartition inventarisiert sie. Alle Fieldreads/Stores,
Maskpflege und terminalen Heads sind ausdrücklich gezählt.

Bei skipped Branches können tatsächliche Actions geringer sein; reservierte
Worstcasearbeit wird trotzdem konsumiert. Heights sind relative0..384; arithmetic
ist innerhalb admitted geometry/index bounds total. Metadata8 bei Update bedeutet
vier private Candidate-/Versionchecks vor finaler Admission und vier feste
Metadatastores danach; kein old-Metadatum wird vorher verändert. Raw/Prime-Metadata8 ist phase-check, lineage-check, kindmask-check, version-check,
phase-store, lineage-store, mask-store, version-store im privaten Kandidaten.
Update benutzt die ersten vier als private Prüfungen und die vier Stores nach
finaler Admission. Candidateconstruction8 ist der feste Constructorplan:
owner-store, producer-store, version-store, phase-store, mask-store,
candidate-borrow, output-token-init, changed-result-init. Einzelne zusammengesetzte
lineage-Token sind fixed private IDs; keine ungezählten Strings/Objektkopien.
StateID range/token4 ist raw-read, domaincompare, revisioncompare, checked-tokenbuild;
Denseproof4 ist borrowbuild, Registryrevisionstore, Producerstore, immutableprooftag.
Publish2 ist admitted Ownermove und oldownershiprelease. Primecolumn3 ist
head, unresolvedmask-init, advance; Heightinit und alle kind/read Heads sind
separat als L erfasst. Finale Admission1 ist der einzige
private fixed-size Commitpermit-Check, nicht ein beliebiger Callback.

| Kernel | vollständiger Plan | N |
|---|---|---:|
| einzelnes raw→CheckedStateId | G+4 range/token-Aktionen+C | **36** |
| ganze Dense-IDadmission | G+L(98304,5)+4 Proofbuild+1 final+C | **491.559** |
| kompletter Rawimport | G+8 Candidate+L(222,8)+L(1536,8)+L(6,7)+8 Metadata+1 final+2 swap/release+C | **14.163** |
| frisches sechs-Kind-Prime | G+8 Candidate+L(222,3)+L(6,6)+L(256,3+L(6,3)+L(384,5+L(6,6))+L(6,7))+L(222,6)+L(1536,8)+8 Metadata+1 final+2 swap/release+C | **4.259.119** |
| sechs-Kind-Einzelupdate | G+2 Newfact+L(6,11)+L(6,22+L(383,6)+8+L(7,6)+4)+8 Metadata+1 final+L(6,6)+C | **14.419** |
| eine checked Query | G+6 kindpolicy+5 column/index+1 wordread+3 extract+1 range+3 first/highest/empty+2 return+C | **53** |
| neue Raw-IDadmission + frisches Prime |491559+4259119 | **4.750.678** |

Priming scannt in Columnorder0..255 und Y383..0; ein read/fact pro State,
sechs feste kind-Slots pro read. Fixed unresolvedmask, keine Java-Referenzliste.
Ein unresolved passendes Bit setzt height=y+1; leere Maps bleiben zero.
Kein Sectionemptiness-/Topsectionsearch; kein literal-Air-Schritt nötig, weil
die genuine Surface-/Motionflags der drei Air-IDs false sind. Danach sechs
bounded Packwrites je Column und **vollständige**222word-/1536value-Validation.
Diese Outputprüfung ist zusätzliche Arbeit, keine „bereits validen Inputs“-Abkürzung.

Rawimport: exact222 words (=sechs×37), jede inputword-paddingprüfung und einmalige
private Copy, dann alle1536 Werte auf0..384. Wörter0..35 haben nur63 Nutzbits,
word36 hat vier Nutzslots=36Bits; bit63 beziehungsweise alle oberen28Bits müssen
zero sein. Keine von Java erlaubte511/Paddingmutation oder Wronglength-Primefallback.
Kind-/Absent-/Phase-/Geometry-/Historyauthority und Nativeadmission sind Pflicht.
Candidate ist genau das nachher published Objekt, kein dritter Finalcopy.

Update: pro selected Kind alte erste Höhe aus einem alten packed Word; original
lowguard `y<=first−2`, passende Anhebung bei `y>=first`, andernfalls nur bei
entfernter nichtpassender Topposition downwardbismin. Max383 rescanned States je
Kind. Rückgabechanged ist die genuine Updatewirkungsentscheidung, auch bei
gleichbleibendem Wort darf sie nicht in ein Nettdeltabool umgedeutet werden.
Unselected Records werden als unverändert markiert. Newfact2 ist checked
Newstate-/fact-read aus dem vorab gebundenen Transitionproof.

Query53 führt **keinen Stateread** und keinen Readercallback aus. Sie prüft
Kindpresence/freshness/Historyauthority, berechnet und liest exakt ein Word,
extrahiert und rangeprüft genau einen Wert. first=min+relative,
highest=first−1, empty=(relative==0). Leer−64/−65 und oberster Block319→320/319
bleiben exakt. Finaltypen unterscheiden ChunkAccess highest von Region first.

**Neue gesonderte Producer-Policy zur Rootprüfung:** HeightPreparationAllowance
1..=**8.388.608 (2^23)**, der nächste Zweiercap oberhalb4.750.678 für whole
raw-ID+Prime. Gegenüber dem alten unangenommenen4.194.304-Vorschlag ist dies
eine ausdrückliche neue Entscheidung; kein bestehendes GENlimit wird erhöht.
8.388.608 bedeutet keine Garantie beliebig vieler Updates/Chunks pro Stage.
Ein späterer echter Producer kann diese Reservationen in seinen bereits
genehmigten Meter integrieren oder braucht einen neuen geprüften Stagevertrag.

**Ore:** Query53 muss durch einen konkreten sealed Adapter unmittelbar den
bereits aktiven einzigen Ore-Meter belasten, vor irgendeinem Mapread. Kein
HeightPreparationMeter, Upgrade, Reset oder frei implementierbares Worktrait
mitten im Ore-Run. GEN-06.2 besitzt derzeit keinen solchen Consumerdispatch;
eine neue private Dispatch-/Queryadapteränderung braucht eigenen Review.
Der pending GEN-06.3-Compositionowner muss53 je tatsächlicher Gatequery plus
seine Chunkselectionarbeit neu einrechnen. Diese Revision akzeptiert seine
Allowance/Nativecaps nicht und verleiht ihm keine Compositionauthority.

## 6. Deltainduktion, Ownership und Atomizität

Genau sechs feste Records: `{word:u64, kind:u8, word_index:u8,
changed:u8, reserved:[u8;5]}`. Eigenes repr(C)-Modell16B/Record,96B Records;
Scratch mit heights/masks/index/Y/Meter/Error/96-B-Localsmodell **240B**.
96B ist die Recordmenge, ausdrücklich **nicht** der gesamte Scratch.
Update kopiert keine222 Wörter und baut keinen vollen Mapsetcandidate.

Induktionsvoraussetzung ist ein durch admitted Raw/Prime/previous Delta
unveränderlich bewiesenes Mapset: alle Werte0..384, sämtliches Padding zero,
passende Metadata-/Historyauthority. Für Columnc gilt word=c/7,shift=9(c%7).
`new=(old & ~(511<<shift)) | (relative<<shift)` ersetzt nur neun Nutzbits.
relative wird vor Bau0..384 geprüft; old-range/padding ist sealed.
Alle sieben Slots des changed Word werden nochmals geprüft, einschließlich
dreier ungenutzter Endslots bei word36; Paddingmaske ebenfalls. Somit bleiben
alle unberührten Worte/Slots invariant, das einzige geänderte Feld ist admitted,
und der komplette Set bleibt gültig. Keine neue full-outputvalidation beim
Update; ihre überflüssige Wiederholung wäre zusätzliche Work/native Pflicht.

Vor Publication erfolgen alle array-bound-/range-/history-/version-/native-
und changedwordprüfungen, panic-capable Helpers sowie Aufbau aller sechs
Zielreferenzen. Exclusive `&mut Owner` und immutable Readerborrow schließen
konkurrierende Veröffentlichung aus. Danach letzte Commitadmission; Refusal
liefert alten Owner bytegleich, Candidate/Records werden POD-gedroppt.
Erst danach sechs bounded stores und Metadata4, ausschließlich total fixed
Indizes bzw.vorher erzeugte sichere mutable Wordreferenzen. Sechs verschiedene Arrays
liefern disjunkte referenzen mit sicheren Borrows; eine nach Commit erneut
indexierende/fallible Zugriffsfunktion ist nicht zugelassen. Kein fallibler
Lookup/Allokation/Hash/Callback, indexingpanic oder Destructor nach dem ersten
Store. Observer/Plugin/IO sind ausgeschlossen. Administrative Ownershiprelease
ist bereits admitted/infallible. Panic **vor** Commit erhält alte Bytes;
nach Beginn darf der Kernel nicht unwindfähig sein. Kein abort/crash-atomicityclaim.

Raw/Prime ersetzen einen vorbereiteten besessenen Candidate durch einen
infallible Ownermove/swap unter derselben Exclusiveauthority. Caller liefert
einen vorab admitted freien Kandidatenslot; Allocationadmission gehört in
Native/G. Kein implizites Copy/Clone/Box::new eines dritten großen by-value Sets.
Reborrow verhindert Katalogaustausch oder Lesermutation während prepare/commit.
Ergebnisse überleben Writer/Reader-Drop, solange ihre echte Historyowner-
referenz gültig bleibt; geliehene Mapviews dürfen nicht in freigegebene Owner zeigen.

## 7. World-only: alle Fehlpfade und simultane native Ziele

Allowanceconstructor0/max+1 → `InvalidAllowance`, noch kein Kernel/Meter.
Jede Kerneloperation reserviert ihr ganzes N **vor** G. Deshalb gewinnt
`WorkBudgetExceeded` gegen gleichzeitig falsche Geometrie/Inhalt, wenn N fehlt;
remaining und oldbytes bleiben exakt beim Eintritt. Nach erfolgreicher Reserve
gilt Guardslotorder aus§5. Erste Fehlerklasse/Ordinal gewinnt; keine zusätzliche
„alle Fehler zuerst“-Traversal. Kindzustände werden in Enumorder0..5 geprüft.
Rawwordpadding wird vor value-range geprüft, dann Values kind/column order.
Updatefehler kommen kind0..5/scanYdescending/wordslot0..6, dann finalauthority.

| Refusalklasse | Arbeit / alte Bytes / Ownership |
|---|---|
| invalid allowance, kein Meter | constructor bounded; keine Mutation |
| N fehlt, auch bei gleichzeitig malformed Input | keine Kernelarbeit/Reserve, remaining unverändert, keine Candidate-/Recordinitialisierung |
| Geometry/Mask/Column/Y/Identity/Readerlength/Native/permission | N vollständig verbraucht; oldbytes unverändert, keine Inhaltsloop |
| UnknownStateID bei Dense/raw Stateconversion | N vollständig verbraucht; erster linearer Stateordinal; kein Proof/Output |
| Missing/Uninitialized/Stale/Reprime/Phase/History | N vollständig verbraucht; kein fallback/repair/Publication |
| RawLength/Padding/Range/AbsentNonzero | N verbraucht, Candidate vollständig privat; old erhalten |
| DefensiveFact/DeltaRange/DeltaPadding/Version/FinalAuthority | N verbraucht, keine partielle Stores/Metadata; PODcleanup≤C |
| private präcommit Panicunwind | alte Bytes erhalten; Drop bounded; keine fortsetzbare halbpublizierte Map |
| Erfolg | N verbraucht; ein vollständiges Result; kein später fallibler Step |

Kein aus unbekannten IDs gebautes AIRdefault, keine Nullhöhe bei MissingMap,
kein automatisches Erhöhen/Reset. Fehler payloads sind private bounded enums mit
typed kind/column/Y/ordinal/requested/max; keine Runtimefehlermeldungsstrings.
Denseproof aus untrusted rawIDs hat Conversionkosten491.559 und bindet Borrow-
Lifetime; checked States aus existierendem admitted Producer benötigen keinen
erneuten Vollscan, aber dessen komplette kostenpflichtige Admissionauthority.
Rawmapadmission14.163 ist eine andere Operation, kein Ersatz der IDadmission.

| Objekt / simultaner Scope | eigene Modellbytes / vorgeschlagenes Rootziel |
|---|---:|
| ein Mapset inkl.24B lineage und Zuständen |1.808 / retained≤2.048 |
| sechs DeltaRecords / ganzer Scratch inklusive temporärer Locals |96 /240; Gesamtziel≤512 |
| Update:old+Scratch, kein voller candidate |2.048 Modellbytes |
| Raw/Prime:old+ein owned candidate+Scratch |3.856 Modellbytes; standalone owned Ziel≤4.608 |
| geliehener Rawmapinput |1.776B separat, plus Slice/Ownerheaders in Scratch |
| Raw Gesamt mit geliehenem Payload |3.856+1.776=5.632B vor weiterer Ownerumgebung |
| geliehener Dense-u32-Input |393.216B separat; Proof-/Referenzheaders zählen im Scratch |
| Prime Gesamt mit Densepayload |3.856+393.216=397.072B vor Registry/Stageowner |
| Update Gesamt mit Densepayload |2.048+393.216=395.264B vor Registry/Stageowner |

Die2.048/512/4.608-Ziele bleiben **neue unangenommene** Grenzen. Das4.608-B-Ziel
betrifft nur standalone owned Mapobjekte plus Scratch; es ist kein Gesamt-
Input/Region/RSScap. Encoded Inputcapacities werden nicht als free ausgegeben.
Bei größerer tatsächlicher Inputcapacity zählt sie vollständig im separaten
Borrowbestand. Registryfacts/static Metadata, Biomes, neun Regionowner,
Postprocessing, Counter/Entities, Reservations und Stagebuffer sind zusätzlich.
H2 bleibt offen: neun Inputs plus private Candidatepayload allein7.077.888B.
Keine dritte Map-/State-/Regioncopy, keine ungezählte by-value Returntemporary.
Rust-literal sizeof/align, reale capacities, Heapheader, Callstack/Compiler-Moves
und cold Allocation/refusal müssen diese simultane Inventur später ersetzen.

## 8. Frische Originalevidenz und vollständige sechs-OutputsTreeAdmission

Der byteidentische alte `HeightmapOracle.java` wurde in N getrennt neu kompiliert.
Alle vier frischen Vanilla/Paper×normal/−O-Läufe rufen echte bootstrapped
Originalmethoden auf, CodeSources vor Output, ganze Pins vor/nach.
Kein kopierter Java-Heightmap-/Predicatealgorithmus als Oracle, keine Unsafe-
Konstruktion/Statussetter. Vollständige Commands/Logs/Helperclasses liegen in
`N/{normal,optimized}/`. Je Profil exakt sechs reguläre TSVs; alle Bytes gleich
zu Autor/reviewer und beiden Pythonmodi. Origins profilgetreu verschieden.

| eigener neuer genuine Befund je Profil | Ergebnis |
|---|---:|
| Blocks/States/Types /Originalpredicatecalls |1.105/27.946/6 /167.676 |
| alle Canonicalname/Properties/Default/ID-Identitäten |27.946, keine Abweichung |
| beobachtete vollständige Fluididentitäten |20 |
| C/D/W/U/P/Q/S Events |10/222/194/114/8/6/282 |
| Tracezeilen /vollständige Snapshotspalten |836 /72.192 |
| default/air/motion/leaves/fluid_empty |1.105/3/23.659/308/18.254 |
| Surface/WG, Floor/WG, Motion, NoLeaves true |27.943,23.659,24.154,23.846 |

Neue Admission ruft `lstat` für Root und **jeden** Topentry auf, bevor irgendeine
TSV geöffnet wird. Root muss echtes Directory, jeder Entry reguläres File sein,
Namenmembership exakt `states.tsv/types.tsv/census.tsv/statuses.tsv/origins.tsv/
trace.tsv`, count6. Da keine Directories zugelassen sind, ist damit die ganze
OutputsTreeMembership vollständig geschlossen: auch extra leeres/nested
Directory oder Directory unter erlaubtem Namen verweigert. Alle Symlinks,
einschließlich allowed-name, broken/escaping, Root-/Directorylinks und FIFOs/
sonstige Specials werden abgewiesen, niemals vorher dereferenziert. Trusted
Parent/exklusive immutable Outputdirectory über Admission+Parsing ist Pflicht;
der offline Validator verspricht keine TOCTOU-Abwehr gegen einen gleichzeitig
mutierenden fremden Filesystemowner. Der echte Runner besitzt seine fresh Outputs.
Authoritydirectory wird derselben Entrytypeprüfung unterzogen; whole-file/
Originalpins/CodeSource/input/ZIPshadowguards bleiben aktiv unter−O.

Je Modus insgesamt48 Probes:42 Refusals/6 Positivkontrollen; zusätzliche
mathematische Modellkontrollen stehen separat. Je Modus: `membership-*-final` **19 Fälle:16 Refusals/3 Positivkontrollen**,
einschließlich aller genannten Treearten und unknown-block/property cases.
`challenges-*` **15 Fälle:14 Refusals/1 positive**, zusätzlich beide frischen
Profilvalidierungen. Late-ID/default/predicate/fluid/type/origin/status/
Previous-/readback-/sentinel-/padding-Refusals bleiben erhalten.
`guards-*` **14 Fälle:12 Refusals/2 genuine positive**; fünf echte Javaenv-
Entrypointrefusals, sechs echte JVM-Original/Helper/Library-CodeSourcefehler vor
TSV und tatsächliche private JARbytemutation gegen unveränderten Originalpin.
Alle Membershipmutanten/Regularbytes/Linktargets/Entrytypes sind als neue
Stimuli eingefroren; Specials werden beim Freeze nicht geöffnet/Links nicht verfolgt.

Originalsemantik bleibt vollständig erhalten: min−64 leerfirst−64/highest−65;
Top319→first320/relative384; nonspanning9bits/seven slots/37words,z*16+x.
Genuine Raw511/padding bleiben als Java-Leniencecontrols in Trace erhalten;
Wronglength löst echtes Primefallback aus. Die neue Worldrefusal ist strengere
FerrumC-Policy, keine Errorinputparität. Stale-reprime bleibt im genuine Trace
stale und in der neuen Fresh-API ausdrückliche Refusal; keine erfundene Reparatur.
Weitere genuine kontrollierte Geometrien−128/256,−64/16,0/16 sind Oracleinputs,
keine freigegebene variable Produktionsgeometrie.

Die zehn echte EMPTY-ProtoChunks/222 Inputs enthalten Air/Caveair/Voidair,
Water/Lava levels0/7/8/15, wet/dry/persistent Leaves, Plants/Cobweb/Bamboo,
alle Snowlayers, Slabs top/bottom/double, wet/dry Ladder, Placement/Removal/
Replacement/Rescan/Extrema/Lazy/Protosetter. Keine natürliche Stagecompletion.
Sourcebefund: EMPTY..SURFACE WG, CARVERS..FULL finale vier; NOISE-doFill WGupdate,
Surface-Protosetter und SurfaceWG+1; FEATURES primet finalfour vor Decoration/
BorderTicks. Live-LevelChunksetter/Proto→Level bleiben genuine Caller-/Lightgates.
Die82 ursprünglichen Ore-Factrows/25 Targets/Tags8/4/4 bleiben readonly evidence:
ihre enge Predicateerhaltung ist kein allgemeiner Featurewritebypass.
Flatstates air/bedrock/stone/dirt/grass bleiben korrekt im alten Scope;
id≠0/boundingBox sind kein Ersatz der neuen Flags für alle anderen States.

| neuer unveränderter Computationaloutput | SHA256 |
|---|---|
| states.tsv | `078215ee6f0ee1cca461b280a6d4666330c0b042db21e542c1ffd6c3b9719eb7` |
| trace.tsv | `f936c3fcfc58cb1a9b08a025023dfa10f1e324c574e01a24854047f3505d1e62` |
| vorwärts- und inverser identity-joins.tsv | `adb062807b6481acf813bd7358ba2863f04d3df6af7d8bfe00ba91fa4a14e62b` |
| predicate-table.u16le | `95e27539aff80d384a7d9841454210da95ceb2ea8c56353d37b19c7d9cbb2701` |

## 9. Modellbelege, spätere Abnahme und kleinstes implementierbares Split

`work-*-closed-ledger` führt für jeden Kernel N−1/N/N+1 der **ganzen Reservation** aus,
mit unverändertem old/remaining bei N−1, vollständig konsumiertem N bei Erfolg
und finaler Authorityrefusal. Die generischen Reservationsprobes testen den
Transaktionsrahmen; tatsächliche Raw-/Dense-/Prime-/Delta-/Querymodelle werden
zusätzlich in getrennten Kernelkontrollen ausgeführt. Dazu1.024 changedword-Column/value-Edges, top384,
383-rescan-bound und vier length/range/padding/unused-slot-Mutanten, neun zusätzliche echte
mathematische Kernelrefusals (letzte Dense-ID, masks0/64, Querykind/column/missing,
Absent-nonzero) und empty/top Querykontrollen.
Dies sind Pythonwork-/Induktionsmodelle; sie sind keine instrumentierten Rust-
Debits, echten Rust-N-Grenzen oder nativen Allocationbeweise. `native-layout.rs`
ist ausschließlich ein dependencyfreies Standalone-size_of/align_of-Modell,
rustc1.99.0/x86_64, keine World-/Registrycrateimplementation. Die dynamischen
offline Checker dürfen nicht als null-Allocationkernel ausgegeben werden.

Vor Runtimeannahme alle folgenden **echten** Debug/Release-Nachweise:

1. Registryparser: hash-before-parse; jede grammar/field/row/wholeinput-N−1/N/N+1;
   duplicate/missing/extra/unknown IDs/names/properties/domains/defaults/fluid/
   types; radixoverflow/domainorder/booltruefalse; vollständige genuine all-ID-
   Vergleiche, keine copied oracle. Late IO/Native/Allocationrefusal mit altem
   Sourceoutput unverändert. File-/Partmembership und no-symlink/fresh controls.
2. Tatsächliche private Consumerkernel: allowance0/max/max+1, jeder plan-N−1/N/N+1,
   multifehler-Präzedenz, selected masks0/1/63/64, Absent/Uninitialized/Primed/
   Historical/Stale, Reader-/History-/Version-/Ownerfehler, Querykind/column/Y-
   Grenzfälle und echte late refusal/unwind-oldbytes. Jede Head/read/fact/mask/
   extract/word-validation/commit-Aktion gegen den konkreten Ledger prüfen.
3. Raw exact222±1, all1536 range0/384/385/511 und Paddingbit63/lastunused,
   jede letzte Kind-/Columnmutation. Delta je wordboundary, selected/unselected,
   raise/nochange/belowtop/remove/rescanempty/full-depth, stale-reprime refusal.
   Fresh-Prime alle genuine kontrollierten admissiblen Snapshotwerte; die
   lenienten Originalcontrols bleiben getrennte FerrumC-Refusals.
4. Native literal sizes/capacities/stack/Compiler-Moves/temporaries/old+new;
   every capN−1/N/N+1 mit privaten controls, erste kalte Allocation und jede
   Refusalklasse, independent positive4096-B-Allocatorcontrol. Vor Null-
   Allocationclaim actual external Debug/Release processes. Keine RSS/TPS-
   Behauptung aus sizeof. Exclusiveborrow/move/foreign-token Compilecontrols.
5. Vollständige betroffene Cratechecks auf neuer von Root eingefrorener accepted
   Basis, eigenes Target, fmt/strict all-target Clippy/rustdoc-Dwarnings und
   unveränderte Flat-/historische Fixtures. Kein Test shared Carver/Storage WIP
   als neue Acceptance. Neuer unabhängiger Runtime-Review nach complete freeze.

**Kleinstes Split:** zuerst Registry-only Producer mit eigenem bounded Join/
Parser/Streamemitter/static Facts/CheckedStateId, exact Native-/Malformedbeweisen.
Danach World-only Raw/Dense/FreshPrime/Delta/Query gegen akzeptierte Registry,
vorerst private kontrollierte Kerneltypes; keine erfundene natürliche authority.
World bleibt bis separater Vergabe bei seinem exklusiven Writer. Anschließend
separates echtes NOISE/SURFACE/CARVERS-WGhistory-Handoffticket und ein konkreter
Region/Orequeryadapter mit demselben Meter. Ganze FEATURES, vorige Structures/
Decorations, genuine ServerLevel/WorldGenRegion/Chunkdependencyholders, radius8/
radius1-Completions, INITIALIZE_LIGHT/LIGHT, LevelChunkconversion/liveupdates,
Session/Owner/Simadoption brauchen ihre eigenen echten Producer/Lifecyclegates.
Kein Naturalworld/Survival/Lighting/Plugin/Performance-/Appacceptanceclaim.

Root muss nach dem neuen unabhängigen Review entscheiden über Static65.536,
Build131.072, Domain26, World2.048/512/4.608, die sechs neuen Reservationswerte
und den eigenständigen HeightPreparation2^23-Vorschlag; alternativ konkret neu
quantifizieren und reviewen. Weiterhin muss Root den echten Historytokenproducer
und späteren genau abgegrenzten Queryadapter/Compositionvertrag separat vergeben.
Weder die ungeänderte GEN-06.2-Primitive noch dieser Designfreeze autorisiert ihn.

## 10. Freeze, Quellen-/Outsideintegrität und Reproduktion

Neue Helper-/Toolkopien besitzen whole-file-Ursprungshashes und sämtliche exakten
Deltas unter `N/provenance/`. Alle neue Methoden-/Reference-/Toolchaininputs
bleiben gepinnt; der Verifier prüft sourcecopy==sourceorigin und alte Tools/
Originaloutputpins weiterhin. Vor/nach echten Aufrufen und abschließend werden
die ganze1490-Dateienaccepted Basis, alle4438 Originalinputs/JDK454/Source-/Patch-
Membership, der202-Fileauthorfreeze,741-Filereviewfreeze sowie366/494-Files-
GEN-06.1-r2-Freezes vollständig readonly gehasht. Foreign mutable Children sind
keine Baseline; deren Änderungen würden hier niemals als eigene Authority gelten.

`N/freeze-manifest.json` enthält ausschließlich diesen Vertrag und alle neuen
expliziten Sources/evidence/inputreceipts, mit vollständigen Directory-/Entrytype-
Memberships. Nur outer selfhash/post-freeze receipts sind explizit ausgeschlossen.
Mutant-Symlinks/FIFOs werden durch lstat/type/linktarget gebunden, niemals als
externe Inputfiles geöffnet. `N/hashes.json` nennt exakte Contract-/Manifest-Hashes,
regular-file-/Entry-/Directory-/Bytecounts. Kein zirkulärer Hash im Vertrag.
Post-freeze normal/−O prüfen alle Bytes/Memberships und unveränderte historical
Inputs erneut; genaue neue Freezehashes werden Root separat übermittelt.

Entwicklungsdiagnostik bleibt sichtbar und ist keine Acceptance: ein anfänglicher
Copy-Patch enthielt zwei literal `+` vor Python-def; vor allen finalen Validator-
Probes korrigiert. Der erste Membershiplauf stoppte an einem alten impliziten
KeyError für unknown Stateidentity; die neue Kopie verweigert nun dort ausdrücklich.
`membership-normal/` ist dieser unvollständige Diagnoselauf; nur `*-final` zählt.
`join-normal/` und `work-{normal,optimized}/` sind frühere Modellreceipts;
nur finale Namen `join-*-stream-complete`/`work-*-closed-ledger` zählen. Auch
`*-final`, `*-closed`, `*-closed2`, `*-complete` und `work-*-sealed` sind frühere
Modellstände. Ein kurzer Pythonquotingfehler bei Descriptor-Emission wurde vor
den finalen Streamingläufen korrigiert; er erzeugte keine Outputs. Kein alter Source/Freeze
wurde geändert. `freeze-attempt1/` bewahrt den ersten neuen Freezeversuch:
beide Postfreezeprüfungen verweigerten, weil Countkeys die gleichnamigen
Membershiplisten überschrieben. Getrennte `entry_count`/`directory_count`
korrigieren diese neue Toolkopie; keine historische Refreeze/Byteänderung.
`freeze-attempt2/` bewahrt den zweiten Versuch: Membership/Outsidechecks
bestanden, danach verursachte fehlende Klammerung im Comparisonreceipt-Pfad
TypeError. Die Pfadbildung wurde vor dem abschließenden Freeze korrigiert.
Nur der abschließende Rootfreeze und seine beiden erfolgreichen Receipts zählen.
Diagnoseartefakte werden separat im neuen Manifest gekennzeichnet.

Reproduktion nur mit fresh privaten Namen: `reproduce.py MODE`, `census.py MODE`,
`join_model.py MODE`, `work_model.py MODE`, `membership.py MODE`, `challenges.py
MODE`, `guards.py MODE`, normal und `python3 -O` separat. Freshguards verhindern
Overwrite; Reviewer adaptiert allein neue Outputpfade in eigener Scopekopie,
zeichnet diese Deltas auf und behält alle historischen Inputguards. Danach
`audit.py FRESH.json`, finaler Vergleich, `freeze.py build`, `freeze.py verify
normal|optimized`. README nennt konkrete Commands; unabhängiger Review mit
Originalfindings und dieser Responsematrix folgt **vor** Runtime/Capfreigabe.
