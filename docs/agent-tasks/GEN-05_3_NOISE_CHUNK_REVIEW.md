# GEN-05.3 — unabhängiger NoiseChunk-Runtime-Review

Status: **CLEAN — GEN-05.3 am unveränderten finalen Runtimefreeze abgenommen.**
Diese Abnahme betrifft den beschriebenen numerischen NoiseChunkvertrag;
Terrain-, Survival-, Plugin- oder Performanceabnahme bleiben separate Tickets.
Reviewownership: `/root/feature_catalog`; Änderungen
an der Worldruntime bleiben beim Autor `/root/item_transfer_primitives`.

## Maßstab und Quellen

Minecraft Java 1.21.8, Protokoll 772, DataVersion 4440; Paper Build 60,
Commit `29c8822d90899c89d2689338e81a98f690bcba12`. Maßstab sind der vollständige
GEN-05-Brief, das freigegebene .3-Proposal und dessen zwei ausdrücklich
genehmigten Änderungen: Cell-direct erhöht `arrayIndex` vor Childcompute;
die private Paper-Samplingbasis benutzt die tatsächliche f32-Rundung von
Legacy- und Xoroshiro-nextDouble. Die öffentliche .2-Vanillabasis bleibt bestehen.

Die unabhängigen tatsächlichen Java-/Paperproben bleiben eingefroren unter
`.codex-tmp/gen05_3-independent-oracle/`; Freeze SHA256
`0ece7c737a023744d6124aa5fb3cf5b7d6bf9c53f9af094cf9466ba21dcf4582`.
Die vollständige Quellen-/Calleematrix und 63 Pins dieses Oracles werden hier
weiterverwendet. Es wird kein weiterer Fullarray-/Materialexport erzeugt.

Gelesen: AGENTS.md, CLAUDE.md, World-INVARIANTS, Architekturübersicht,
Simulation-/Shardvertrag, PAPER_REFERENCE, vollständiger GEN-05-Brief und
Proposal. Vollständige NoiseChunk-Quelle einschließlich aller Constructors,
Wrapper, Provider, Driver und Preliminary-/Climatemethoden erneut verglichen;
RandomState und relevante DensityFunctions-/CubicSpline-/Noise-/Mthcallees
gehören zur bestehenden vollständigen Oracle-Quellenprüfung.

| Vertrag | Geprüfte Runtime |
| --- | --- |
| Eigene Session, Publicadmission, Poison/Reset, Surface-/Climatepublish | `density/chunk.rs` |
| Tatsächliche Coldvisitorreihenfolge, Wrapperregistrierung und zweite Material-/Climatewraps | `density/chunk/cold.rs` |
| Java-record-Gleichheit, primitive Bits, Noise-/Array-/Blended-/End-/Wrapperidentität | `density/chunk/cold/keys.rs` |
| Mapped Splinebounds, Plan-/Shapeprüfung und retained-byteadmission | `density/chunk/cold/spline.rs` |
| Fünf eigene scalar Wrapper, Contextidentität und Cachefailuretiming | `density/chunk/state.rs` |
| Branchspezifische Arraydispatch, eigene Provider, Prefix/Failure und Scratchleases | `density/chunk/fill.rs` |
| Slices, Cellregistration, beide Lerpfolgen und genau ein Outputwrite pro Block | `density/chunk/driver.rs` |
| Private Paperbasis bei unverändertem öffentlichem Vanillaevaluator | `density/evaluate.rs`, `noise.rs`, `noise/{normal,blended}.rs` |
| Ursprung-/Metadataownership und unveränderte .2-Views | `density/bind.rs`, `density/seeded.rs`, `spline.rs` |

## Reproduziertes Finding R1: private Driverphase fehlt

Priorität P2, vor Runtimefreeze an Autor und Root gemeldet. Am gesicherten
Vorfixstand akzeptieren `fill_slice`, `update_y`, `update_x` und `update_z`
eine vollständig neue Session außerhalb einer laufenden Interpolation und
verändern den Arbeitszähler beziehungsweise Cachefelder. `select_cell` prüft
nur den Bool und Zellindices; nach einzig `interpolating=true` akzeptiert es
noch nicht initialisierte Slices. Der genehmigte Vertrag verlangt typisierte
Lifecyclefehler für illegale private Folgen und stale selectedCell.

Der unabhängige Test verwendet den tatsächlichen Netherrouter, Seed 0, und
für jede Operation eine neue Session. Ergebnis:

| Ungültiger Aufruf | Ergebnis | Work vorher → nachher |
| --- | --- | --- |
| Fresh `fill_slice(true, 0)` | `Ok(())` | 49.856 → 53.068 |
| Fresh `update_y(0.5)` | `Ok(())` | 49.856 → 49.861 |
| Fresh `update_x(0.5)` | `Ok(())` | 49.856 → 49.859 |
| Fresh `update_z(0.5)` | `Ok(())` | 49.856 → 49.857 |
| Keine initialisierten Slices; lediglich `interpolating=true`; `select_cell(0, 0)` | `Ok(())` | 49.856 → 51.663 |

Reproduktion ausschließlich in der eigenen isolierten Kopie:
`.codex-tmp/gen05_3-independent-review/pre-lifecycle-fix/server/`.
Test `independent_review_pre_freeze_illegal_lifecycle_acceptance`, Log
`../pre-lifecycle-fix-test.log`, vor Testinjektion gehashte 86 Worlddateien in
`../pre-lifecycle-fix-files.json`. Die reduzierte Scratchworkspace benötigt
eine eigene Lockprojektion; Produktions-Cargo/Lockfiles wurden nicht geändert.
Der Test dokumentiert die damalige fehlerhafte Annahme ausdrücklich und wird
nicht als positiver Runtimevertrag übernommen. Autor hat die Lücke bestätigt
und vor Freeze korrigiert. Die abschließende unabhängige Fixgegenprobe auf dem
unveränderten Finalstand bestand; Details stehen unten.

Zusätzlich konkret an den Autor gemeldet: private Slice-X-/Index-/Deltadomains
sollen vor Work-/Cachemutation geprüft werden. Am Vorfixstand konnte ein
beliebiger Slice-X intern die Multiplikation überlaufen; `for_index` lud den
Meter vor Indexadmission. Diese Punkte werden zusammen mit der
Phasekorrektur bewertet.

Die aktuelle Previewkorrektur wurde unabhängig in einer zweiten isolierten
Kopie geprüft. Eigene Fresh-/Dormant-/FirstSlice-/NeedAdvance-/SlicesReady-/
Y-/X-/Swap-/Complete-/Poisoned-Gegenproben bestanden einschließlich falscher
Slice-X-Werte `i32::MIN/MAX`, Indexexactmax/`usize::MAX`, nichtendlicher Deltas,
falscher Arraylängen und der fillingCell-Zwischenphase. Der eigene Fingerprint
vergleicht Meter, Counter, Driverphase, Koordinaten, alle Wrapper-/Cachewerte,
Scratcharrays und sämtliche Surfacecacheslots. Log:
`.codex-tmp/gen05_3-independent-review/runtime-preview-independent.log`;
116 vor eigenen Testinjektionen gesicherte Worlddateien in
`runtime-preview-files.json`. Dieser Previewpass ersetzt keinen finalen Freeze.

Root präzisierte den Phasenvertrag: Die private Admission sichert vorhandene
Slices, Datenabhängigkeiten und verweigert stale selectedCell. Selektive
Original-/Wrapper-/Providerprobes bleiben zulässig; eine zusätzliche
exhaustive Cell-/Blockzählung im privaten Testpfad wird nicht verlangt.
Der öffentliche Driver muss weiterhin sämtliche Z/Y/X-Zellen in der
Originalreihenfolge abarbeiten und ausschließlich die vollständige Ausgabe
publizieren. `swap_slices` invalidiert die checked Session-Updates und den
checked Session-Materialzugriff.

## Review-Härtung R2: checked Materialeintritt

Im aktuellen Preview ist der bisherige Materialeintritt des Drivers eine
direkte `state.sample(material, own=true)`-Folge. Nach einem Slicewechsel gibt
derselbe rohe Eintritt weiterhin die zuvor ausgewählte Zellcachezahl aus:
Netherrouter, Seed 0, Ergebnis `Ok(0.4583333333333333)`, Work 58.099 → 58.100.
Eigener Scratchtest `independent_review_pre_guard_stale_material_acceptance`
im vorgenannten Log dokumentiert diesen vor Guard vorhandenen Zugang.

Die rohe Wrapperoperation ist eine ausdrücklich beibehaltene Originalseam;
deren Sentinel-/selektive Probes sollen keinen pauschalen Driverphaseguard
bekommen. Root und Autor vereinbarten stattdessen einen engen privaten checked
Session-Materialeintritt: Running/ZUpdated vor Work oder Statezugriff prüfen
und diesen im tatsächlichen Driver verwenden. Der Autor ergänzte diesen
Eintritt; nach `swap_slices` verweigert er stale Materialzugriffe ohne Mutation. Der
öffentliche Driver hatte bereits die gültige vollständige Schleifenfolge;
dieses Finding belegt keine bisherige Terrain- oder Gameplayabweichung.
Die eigene Fixgegenprobe auf dem finalen Stand steht unten.

## Abnahme des eingefrorenen Runtimekandidaten

Kandidatfreeze `.codex-tmp/gen05_3-runtime-candidate.freeze.json`, SHA256
`9198bbb58916e1a4d773dcd6a052be04f6b8c995489d6e17b0772dfb66db2bbb`:
sämtliche 118 Dateien vollständig gehasht, nach den Gegenproben erneut
unverändert geprüft. Eigene isolierte Kopie:
`.codex-tmp/gen05_3-independent-review/runtime-candidate/server/`.
Die 117 Worlddateien stimmen vor ausschließlich eigenen Testinjektionen
exakt mit dem Kandidaten überein; Sicherung `runtime-candidate-files.json`.
Der gemeinsame GEN-Brief ist die 118. Datei und wurde ausschließlich gelesen.

Eigene drei Tests bestanden:

- 143 verweigerte Lifecycle-/Domain-/Shape-/Materialzugriffe mit unverändertem
  erweitertem Statefingerprint einschließlich Arbeitszähler;
- dieselben 22 tatsächlichen Java-equals-Kontrollen auf dem Kandidaten;
- alle zehn Driverübergänge jeweils mit exakt notwendigem Budget N und
  mit korrekter Verweigerung bei N−1: Initialisierung, erster Slice, Advance,
  Cellselection, Y/X/Z, Materialsample, Swap und Stop.

Der checked Materialeintritt ist vor Y/X/Z und nach Swap verweigert, bevor
State oder Work mutieren; der gültige ZUpdated-Zugriff bleibt erfolgreich.
Der tatsächliche Driver und die 49 Driverdiagnosen verwenden denselben
Eintritt. R1 und der enge R2-Vertrag sind auf diesem Kandidaten unabhängig
gegengeprüft. Rohes Originalmarkersampling bleibt eine deklarierte Seam.
Log `runtime-candidate-independent.log`.

Zusätzlich die vollständigen zwölf Autorenrequesttests in der eigenen Kopie
reproduziert: alle 55 Wrapperkontrollen und zwölf Providerkontrollen sowie
sieben unveränderte endliche Fehlerprefixe, 16 Addendumfälle und die weiteren
Arraykontrollen bestanden. Bits, kompletter Prefix, Requests und Counterstate
werden direkt verglichen. Die cfg(test)-Leaves ersetzen ausschließlich
deklarierte Childleaves; checked Dispatch, Produktionsevaluator, Wrapper und
Provider bleiben der tatsächlich getestete Code. Der gesamte Hilfspfad
einschließlich Foreignprovider und Tracearrays ist cfg(test); es wird keine
Callback-/PublicAPI für Produktionscaller geöffnet. Log
`runtime-candidate-requests.log`.

Sieben Coldtests in derselben Kopie bestanden: vollständige tatsächliche
Constructor-Gleichheitspartitionen/Registrierungen, eigene Equalsprobe,
Keys-/Identity-/Map-/Wrapper-/Graph-/Depth-/Visitcaps, falsche Childshapes
und Scratchpreflight einer großen zugelassenen Geometrie. Separate Key-,
Graph-, Cold-, Cache-, Scratch- und Outputarenen mit checked Arithmetic und
Admission vor den jeweiligen Allocations/Growths wurden vollständig gelesen;
Surfacecache publiziert erst nach erfolgreichem Scan und erlaubt Hits an
Kapazität. Log `runtime-candidate-cold.log`.

Die Kandidatabnahme wurde nach Finaldelta- und vollständiger Pinprüfung auf
den finalen unveränderten Freeze übertragen. Es gibt kein weiteres
reproduziertes offenes Runtimefinding.

## Exakter finaler Runtimefreeze

Finaler Autorenfreeze `.codex-tmp/gen05_3-world.freeze.json`, SHA256
`cbe9d07652b85427589e610da2baf6c796ff002f6f9eee5dd5dd1256c8c48392`:
alle 118 Größen-/SHA256pins vor dem Kopieren und nach der Prüfung verifiziert.
Zwischen geprüftem Kandidaten und Finalstand liegen genau zwei vollständig
gelesene formale Änderungen: der cfg(test)-Requests-None-Guard verwendet
`self.requests.as_ref()?` statt des gleichwertigen expliziten Returns;
ein Rustdoclink auf die private Singlevaluecontainerfunktion ist Klartext.
Produktionsnumerik, Driver, Materialguard und Caps sind unverändert.

Eigene isolierte Finalkopie:
`.codex-tmp/gen05_3-independent-review/runtime-final/server/`.
Vor eigenen Testinjektionen stimmen sowohl Inhalt als auch vollständiges
117-Worlddateien-Set exakt mit dem finalen Freeze überein; Sicherung
`runtime-final-files.json`. Eigene drei Tests nochmals auf dem exakten
Finalstand bestanden: 143 atomare Admissionverweigerungen, 22 tatsächliche
Java-equals-Kontrollen und zehn Budget-N/N−1-Gegenproben. Der gültige
Materialsampleeintritt gelingt erst nach ZUpdated; der stale Eintritt nach
Swap wird typisiert und ohne State-/Meteränderung verweigert. R1 und R2 sind
damit auf dem endgültigen Runtimefreeze unabhängig abgenommen. Log
`runtime-final-independent.log`.

Die zwölf direkten Requesttests wurden wegen des geänderten cfg(test)-Guards
ebenfalls vollständig auf dem Finalstand wiederholt und bestanden. Die
unveränderten Caps-/Constructormethoden und -tests des Kandidaten wurden
hashgleich auf den Finalstand übertragen; Root prüft zusätzlich die gesamte
Worldcrate auf einer frischen unabhängigen Kopie. Log
`runtime-final-requests.log`.

Die acht Autorenchecklogs/-dateien in `gen05_3-final-checks.json` wurden
unabhängig vollständig nach ihren Größen-/SHA256pins verifiziert;
Manifest-SHA256
`09ed60b39f463a7b0f2832359054c754363d6db5c87cb7f421676388254bda59`.
Autorenprüfungen: je 229 Tests in Debug und Release, ohne ignored Tests;
Clippy und Rustdoc mit `-D warnings`, Formatcheck, zwölf finale Requesttests
und 27 Negativguards je normal/`-O` bestanden. Diese gelesenen/verifizierten
Autorenlogs wurden zusätzlich zu den unabhängigen Root-Gesamtchecks geprüft.

## Eigene Gleichheitsgegenprobe

In der erhaltenen Vorfixkopie wurden 22 tatsächliche Java-equals-Kontrollen
direkt gegen die Rustkeys und deren Identityinternierung ausgeführt; alle
22 bestanden. Geprüft wurden f64-/f32-NaNs und ±0, Directholder-/Markerchild-
Gleichheit, moderne geteilte und Legacy-frische Noisetokens, gleiche und neue
Splinearrayidentitäten, Splinewrapper, frische End-/Blendedidentitäten und
gemeinsam verwendete Ergebnisse gleichwertiger Marker. Die 23. Javakontrolle
`native-reference-repeated` gehört zur vollständigen Constructorpartition;
sie wurde nicht durch einen künstlichen Keysvergleich ersetzt.

Eigener Test: `independent_review_actual_java_equals_controls`; Log
`.codex-tmp/gen05_3-independent-review/equals-test.log`, Referenzextract
`equals-actual-paper.jsonl`, Provenance `equals-source.json`. Quelle ist der
unveränderte tatsächliche Paperhauptcorpus mit SHA256
`892f86c29b5202f0e6cafd57a4d4c4b5fbe049609fd80222f1c39ea8a6ab2cc4`.
Die Gegenprobe wurde auf dem endgültigen Runtimefreeze erfolgreich wiederholt.

## Eingefrorenes endliches Arrayaddendum

Root und Runtimeautor autorisierten 16 zusätzliche tatsächliche Javafälle,
weil der letzte NaN des ursprünglichen Arrayinputvektors bei FerrumCs endlicher
Policy einige später eintretende Branches und Fehlerprefixe verdeckt. Das
Addendum verändert ausschließlich diesen letzten Input von NaN auf `6.0`;
actual-JAR-Factories und `fillArray` entscheiden weiterhin die Ergebnisse und
Requestfolgen. Die Helpermatrix enthält keine nachgebildeten Density-, Cache-,
Noise- oder Lerpformeln. Sie kompiliert und verwendet die eingefrorenen
Reflection-/Provider-/Observationmethoden des ursprünglichen Oracles erneut.

23 frische Kontrollen pro Backend liefern 16 fehlende Varianten. Sieben
bereits endliche Fehlerprefixe werden aus dem Hauptcorpus unverändert
weiterverwendet und im Runner frisch byte-/objektgleich verglichen:
`add|left`, `mul|left`, `min|left`, `max|left`, beide
`mulOrAdd-zero|…|true`-Varianten und `rangeChoice|input`.

Addendumverzeichnis:
`.codex-tmp/gen05_3-independent-review/finite-array-addendum/`.
42 Dateien sind eingefroren; Freeze SHA256
`a111bf5647885c643e8cbdde4b950f6342430ef5191daeca6f02b25a014f163f`.
Die 16-Record-Fixture enthält 12.159 Bytes, SHA256
`615f6313ec15be94ca86c2d488db6522b5c47779f632c28a37a0ab3f51438d82`;
Vanilla und Paper sind exakt bytegleich. Manifest-SHA256 beider Pythonmodi:
`7781715eefd8c4f9a7fa8897ec461b2b10563717ca0acaf4116d7b662857102e`.

Checks: 15 Dateien je normal/`-O` byteverglichen, 28 echte CodeSources geprüft,
acht Negativguards je Modus bestanden, Python-Compile bestanden. Darunter
verweigert ein tatsächlich falscher frischer Helper-JVM-Origin vor dem
Fixtureoutput. Der ursprüngliche 117-Dateien-Freeze wurde danach vollständig
unverändert nachgeprüft; weder bestehende Oracledateien noch die ursprüngliche
Fixture wurden geändert. Keine zusätzlichen Fullchunk-/Materialexports.

## Abschließende Reviewbewertung

Root prüfte eine frische `git archive`-Kopie von `58db2977` mit exakt dem
gefrorenen World-/GEN-Briefstand und drei eigenen PublicAPIproben gegen
tatsächlich eingefrorene Paperbits. Die ganze Worldcrate bestand in Debug und
Release jeweils 229 vorhandene Tests plus drei Rootproben, ohne ignored
Tests. Striktes Alltargetclippy, Formatcheck und Rustdoc mit `-D warnings`
bestanden. Ein erster Clippyfehler betraf ausschließlich 18 Hexliterale in
Roots eigenen Zusatzproben; nach reiner Separatorformatierung bestanden
Clippy sowie die drei Rootproben in Debug/Release erneut. Ergebnisse:
`.codex-tmp/gen05_3-root-review/final-logs/results.json`.

Nach diesen Ergebnissen wurden sämtliche 118 finalen Runtimepins, 117
Hauptoraclepins, 42 Addendumpins und elf eigene Beweisdateien nochmals
vollständig nach Größe/SHA256 geprüft: unverändert. Der eigene Beweisfreeze
liegt in `final-review-evidence.freeze.json`, SHA256
`6533fb2ac1e9539c42f6f3625f06bbd21a8b45a0601871cec33c5fa2280666b8`.
Root prüfte diese elf Dateien ebenfalls unabhängig.

**CLEAN für GEN-05.3 am angegebenen finalen Runtimefreeze.** Beide konkreten
Reviewfindings sind korrigiert und unabhängig gegengeprüft; kein
reproduziertes Runtimefinding bleibt offen. Die konkrete Runtime, alle
Änderungen, Phase-/Shapegegenproben, Ressourcenadmission und Metergrenzen
sind abgeschlossen. Der Vertrag umfasst ausschließlich frische numerische
NoiseChunk-Interpolation und die geprüften Cache-/Surface-/Climatefolgen.
Aquifer, Materialblöcke/-fluids, Biome, vollständige Weltgenerierung,
Survival, Java-/Paperplugins und RAM/TPS-Vorteile sind dadurch nicht abgenommen.
