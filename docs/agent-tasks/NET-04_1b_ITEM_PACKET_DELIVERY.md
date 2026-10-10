# NET-04.1b — echte Netklassifizierung der neuen Entitypakete

Status: konkreter Entwurf nach NET-04.1-Protofreeze vom Root intern freigegeben;
Netproduktion umgesetzt, isolierte Whole-Net-Abnahme bestanden und für den
abschließenden unabhängigen Rootreview eingefroren. Keine Commit-/Pushintegration
durch den Autor.
Ein Autor besitzt ausschließlich `server/crates/ferrumc-net/**` und diesen
Brief. Proto-/Spec-/Generator-/Session-/Sim-/App-/Items-/Registrydateien sind
readonly. Nur Root integriert Git; .1 und .1b werden gemeinsam veröffentlicht.

## Konkrete Integrationslücke

Die beiden vollständigen Matches in `src/play/priority.rs`,
`OutboundPriority::for_packet` und `Criticality::for_packet`, behandeln jede
vorhandene `ClientboundPlayPacket`-Variante explizit. Die drei neuen Varianten
aus NET-04.1 brauchen deshalb einen eigenen Netconsumer, bevor der Workspace
wieder kompiliert. Keine pauschale Wildcard oder verteilte Handeditänderung.

Vor Code liefert der Autor den konkreten Klassifizierungsentwurf und liest
die vorhandenen Layer-A-/Layer-B-Verträge sowie die tatsächlichen Paper-
Sendepfade für Entitypairing, dirty Metadata, Motion und Take/Remove. Gepinnte
Referenz: Minecraft1.21.8/PaperBuild60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`.

SetEntityData muss den bestätigten Stack-/Countzustand zuverlässig übertragen;
ein verlorenes Partialpickupupdate darf keine dauerhafte falsche Itemanzeige
erzeugen. Spawn→Metadata und Take→Remove benötigen gemeinsame konkrete
Prioritäts-/FIFO-Regeln. Der quantisierte Motionwirevertrag ist vom
Bewegungsconsumer und dessen Healing-/Coalescingvertrag zu unterscheiden.
Ein Standardklassenwert allein beweist keine korrekte gesamte Paketfolge.

Der enge Consumer ergänzt die fehlenden Matcharme und sinnvoll notwendige
Klassifizierungs-/Writer-/Backpressuretests innerhalb der Netcrate. Keine
Socket-/Session-/Gameplayaktivierung, keine neue Queue oder Dependency und
kein Sonderweg zum Umgehen vorhandener Byte-/Packetbudgets. Calleroverride
bleibt möglich und muss gegen das tatsächlich beobachtbare Writerordering
geprüft werden. Falls für zusammengehörige Pakete ein neuer Bundlevertrag
benötigt wird, diesen vor Runtime separat konkretisieren.

## Abnahme

WholeNet Debug/Release, striktes Alltargetclippy/Fmt/Rustdoc. Tests müssen reale
Writerqueues und deren Drainreihenfolge verwenden: State/Worldverkehr,
voller Writer, mandatory refusal und droppable outcome, sowie tatsächliche
initiale Spawn/Metadata- und Take/Remove-Reihenfolgen. Keine bloßen Tests, die
nur denselben neuen Matcharm wiederholen. Positiv/negativ/N/N+1 passend zu den
vorhandenen Queue-/Bytecaps; keine Wanduhrschläfe oder echten Unit-Testsockets.

Exakter Net-/Brief-Filefreeze und unabhängiger Rootreview. Anschließend beide
getrennten Cratecommits gemeinsam pushen und vollständige CI für diesen Head
auslösen. NET-04.2 übernimmt danach Itemmetadatabuilder und Sessiontracking;
DUR-05 aktiviert den echten Format6-Driver. Dieses Netconsumer-Ticket allein
liefert keine Itemphysik, Pickupanimation im Livebetrieb, Mobs oder Survival.

## Konkreter Klassifizierungs- und Writerentwurf

Nur die zwei vorhandenen vollständigen Matches in `src/play/priority.rs`
um je drei explizite Varianten ergänzen. Kein Wildcardfallback.

| Neue Variante | Defaultpriority | Defaultcriticality | Begründung |
| --- | --- | --- | --- |
| SetEntityData | State | Mandatory | Ein verlorener Stack-/Countzustand kann ohne neues Dirtyevent dauerhaft falsch bleiben. |
| SetEntityMotion | State | Mandatory | Vorerst kein geprüfter outbound Healing-/Resendconsumer; eine einzelne Impuls-/Stopänderung darf nicht still verschwinden. |
| TakeItemEntity | State | Mandatory | Dieselbe Lane/FIFO wie vorhandenes RemoveEntities; ursprünglicher Count unverändert. |

Vorhandene SpawnEntity und RemoveEntities bleiben State/Mandatory. Alle
zusammengehörigen fünf Varianten teilen damit bei Defaultadmission dieselbe
State-FIFO. Motion wird konservativ ebenfalls dort zuverlässig geführt; ein
späterer ausdrücklich geprüfter Movementconsumer kann den Kontext mit
Supersession-/Healingregeln anders klassifizieren. Der vorhandene
MovementCoalescer betrifft ausschließlich serverbound Spielerpositionen und
beweist keine outbound Velocityheilung. Dies ist FerrumCs lokale
Transportpolicy, keine Übernahme von Papers eigener Queuepolitik.

**Layer A:** Die Defaultcriticality wird durch OutboundMessage transportiert;
Sessionchannel-Mandatory bedeutet liefern oder Disconnect bei Full/Closed.
Dieses Ticket ändert oder testet Sessionchannel/App nicht. Ihre vorhandenen
Methoden sind readonly geprüft, keine neue Liveaktivierung daraus behauptet.

**Layer B:** PlayWriter besitzt keine Criticality und ändert sich funktional
nicht. `enqueue[_classified]` prüft seine vorhandene Paketcap und liefert bei
Full weiterhin EnqueueOutcome::Dropped, lässt vorherige FIFOeinträge intakt
und zählt den Drop. Die tatsächliche App queue_outbound trägt Priority und
Criticality des OutboundMessage weiter und eskaliert diesen Dropped über
`is_mandatory_overflow`; nur der Caller bewirkt den Disconnect.
OutboundPriority::is_drop_fatal bleibt allein Critical und ersetzt diesen
Callercheck nicht. Der Nettest beweist echten Writerrefusal mit unverändertem
Backlog und zurückgegebenem Drop; keinen App-Disconnect.

**Ordering:** Bei gemeinsamer Defaultlane und erfolgreicher Einzeladmission
gilt FIFO Spawn→Data→Motion sowie Take→Remove über Batch-/Frame-/Bytegrenzen.
Criticalverkehr kann davor entwichen, World-/Cosmeticverkehr danach; keiner
der relativen Stateeinträge wird untereinander umgeordnet. Die Gruppe ist
dabei nicht atomar: bei verweigerter Mandatory-Folge muss der bestehende
Caller den Client schließen. Paket-/Byte-/Framecaps werden nicht umgangen.
Es gibt keinen neuen Bundle-, Reservation- oder Writer-Queuevertrag.

Callerpriorityoverride bleibt möglich. Konkrete Negativprobe stellt
Metadata explizit auf Critical bei Spawn auf State; der echte Writer drainet
Metadata davor. Zweite Probe stellt Take auf World bei Remove auf State;
Remove kommt zuerst. Der spätere Viewerconsumer muss eine gemeinsame
Lane für diese abhängigen Folgen binden und Layer-A/Layer-B-Refusal korrekt
behandeln; Defaultwerte allein machen freie Overrides nicht sicher.

## Tatsächliche Originalquellen und klare Grenze

Applied ServerEntity komplett gelesen; Vanilla-Basis der vollständigen
relevanten Sendemethoden mitgelesen. sendPairingData liefert getAddEntityPacket
vor initialem SetEntityData. addPairing sendet die Liste als echten
ClientboundBundlePacket; removePairing sendet RemoveEntities. Dieser
Netconsumer führt kein Protokollbundle ein und beansprucht dessen Atomicität
nicht. sendDirtyEntityData packt dirty Values vor Broadcast, aktualisiert
trackedDataValues. Motion aus sendChanges ist delta-/Impulse-/hurtMarked-
abhängig, kein Beweis periodischer Wiederholung identischer Velocity.
Die tatsächlichen Sourcepatches und Featurepatch0017 (Trackerresync) sowie
den vollständigen ServerEntity-Hunk in Moonrise0016 mitgelesen.

Vollständige relevante ItemEntity.playerTouch-Methoden Basis/Applied und
Applied LivingEntity.take gelesen. Der Takecount verwendet den ursprünglichen
Count; Paper kann die Animation sogar im cancelled Attempt/Legacyevent
flyAtPlayer-Zweig senden. Normaler erfolgreicher voller Pickup ruft take vor
discard auf; der tatsächliche allgemeine Remove-/Viewer-/Tickconsumer und
Eventparität gehören zur späteren Simulation-/Session-/App-Abnahme.
Sourcepins/Reviewcoverage `.codex-tmp/net04_1b/source-design-pins.json`.

## Konkrete Dateiliste und sinnvolle Abnahme

Produktion nach Freigabe ausschließlich priority.rs (sechs neue explizite
Variantenpatterns in den zwei exhaustive Matches), Net-README/INVARIANTS für
obige Grenzen, ein enges Writer-Integrationtestfile sowie dieses Taskdoc.
Zusätzlich intern freigegebene reine Net-Rustdockorrekturen: writer.rs-
Moduldoku für den Caller-Mandatoryvertrag und vier bestehende private
Doclinks in lib.rs, login.rs und writer.rs. Keine Sichtbarkeitsausweitung oder
Lintausnahme. Keine Writer-Runtimeänderung, neue
Dependencies, Queue, Bundles, Socket-, Session-, App-, Proto-, Generator-,
Registry-, Sim-, Items- oder Storagemutation.

Tests verwenden tatsächliche PlayWriterqueues und kodierten Drain samt
Frame-/Clientbound-Protodecoding: initiale Spawn→Data→Motion; ursprünglicher
Takecount→Remove; Partialmetadataupdate; gemischter Critical/State/World/
Cosmeticverkehr; Batchframescap1 und Bytecap1 sowie maximale bereits
zugelassene263168-Rawtail; PaketcapN/N+1/0, Counter/Backlog unverändert
bei Refusal; bestehender droppable Worldverkehr mit eigenem Dropoutcome;
beide expliziten negativen Priorityoverrides. Ein enger Framecap verweigert
eine größere Metadata und hält den Frontpacket unverändert für den Caller.
Keine bloßen Matcharmtests als Hauptbeweis, keine Wanduhrschläfe oder
Unit-Testsockets. Whole-Net Debug/Release und strict Clippy/Fmt/Rustdoc,
exakter eigener Freeze und unabhängiger Rootreview.

## Bestandene isolierte Ticketabnahme

Lokale Reviewbasis `c3d75914cd64df2f20dc4de0960b356c8556a476` enthält den
separaten NET-04.1-Protocommit `aaa8fdc3`. Exakter eigener Netoverlay in
`.codex-tmp/net04_1b/net-review-c3d75914/`, separater Cargo-Targetpfad
`.codex-tmp/net04_1b/net-review-target/`. Fremde parallele Registry-/Sim-WIP
sind kein Bestandteil dieser Probe. Alle155 gepinnten Dependency-/Spec-/
Generatorsourcedateien dieser Archivebasis und alle79 einzigartigen Dateien
aus NET-04.1s Source-/Evidencefreezes vor/nach unverändert geprüft. Keine
zusätzlichen Paperboots für diesen reinen Transportconsumer.

| Check | Tatsächlich bestanden |
| --- | --- |
| Whole Net Debug | 196 Tests:136 Unit +60 Integration, davon11 neue Writer-Tests;0 fehlgeschlagen. |
| Whole Net Release | Dieselben196 Tests;0 fehlgeschlagen. |
| Alltargetclippy | `cargo clippy --locked -p ferrumc-net --all-targets -- -D warnings`, ohne Lintausnahme. |
| Fmt | `cargo fmt -p ferrumc-net -- --check`. |
| Net-Rustdoc | `cargo rustdoc --locked -p ferrumc-net -- -D warnings`, sämtliche Netdocs ohne Warnunterdrückung. |

Nach den bestandenen Runtime-/Testläufen ausschließlich die vier reinen
Doclinkkorrekturen vorgenommen; Fmt/Clippy/Net-Rustdoc für die endgültigen
Docquellen wiederholt. Writercode nach Entfernen der Rustdoc-Zeilen gegenüber
der Archivebasis exakt unverändert; keine Queue-/Drain-/Encodeänderung.

Die zusätzliche `cargo doc`-Probe mit Dokumentation aller Dependencies ist
**nicht bestanden**: im unveränderten Codec verlinkt `blob.rs:41` eine
öffentliche `new`-Dokumentation auf die private Methode `max_encodable`.
Dieser fremde Baselinebefund ist separat aufgezeichnet und nicht durch
`--document-private-items`, Sichtbarkeitsänderung oder Lintausnahme verdeckt.
Die erforderliche Net-only-Rustdocabnahme besteht.

`tests/item_entity_delivery.rs` drainet echte Writerqueues, löst alle
Längenframes auf, dekomprimiert bei Bedarf und decodiert echte Clientbound-
Pakete. Die11 Fälle prüfen initiale Spawn/Data/Motion/Stop-FIFO, unveränderte
Partialmetadata und ursprüngliche Takecounts, gemischte vier Prioritäten
über normale/Framescap1/Bytecap1-Batches bei aus/Threshold0/Threshold256-
Kompression, maximale263168-Rawtail mit Fortschritt, Statepaketcap2/N+1
und0 samt exakten Dropcountern/FIFO, bestehenden droppable Worldrefusal,
beide expliziten negativen Priorityoverrides, harte Framecap9/10 mit
unverändertem Front/Backlog sowie opake leere/malformed/trailing Metadaten
und signed Takecounts. Die originalen Stonecount1/64-Tails readonly gegen
NET-04.1s tatsächliche Paperfixture geprüft; Item-Type69 readonly aus
minecraft-data3.111.0 für Protokoll772 nachgeschlagen und Dataset gepinnt.
Das beweist Transport, keine Gameplay-/Client-/Slot-/Pluginparität.

Finale Filehashes unter `.codex-tmp/net04_1b/source.freeze.json`; eigene
Source-/Caller-/Fixture-/Baselinepins, Preservationchecks und finale Checklogs
unter `.codex-tmp/net04_1b/evidence.freeze.json`. Nach diesem Freeze keine
weitere Net-/Briefmutation durch den Autor; Rootreview und gemeinsame
Veröffentlichung mit NET-04.1 bleiben bei Root.

Offen bleiben echter Viewerconsumer, Session/App-Refusalabnahme und der
fehlende Bundlevertrag. Vor dem getrennten Metadata-/Slotbuilder wird nach
Rootauftrag eine enge Items-Größenauthority/borrowed Repräsentationsprüfung
geplant; dieses Ticket aktiviert weiterhin keine Itemphysik oder Survival.
