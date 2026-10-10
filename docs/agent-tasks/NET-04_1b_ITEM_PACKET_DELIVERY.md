# NET-04.1b — echte Netklassifizierung der neuen Entitypakete

Status: vorbereitet; unmittelbar nach dem NET-04.1-Protofreeze vergeben.
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
