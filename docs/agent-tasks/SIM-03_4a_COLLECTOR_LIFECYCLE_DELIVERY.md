# SIM-03.4a — Collector-Lifecycle-Addendum: Autorenlieferung

Reviewbarer **Sim-only Runtimekandidat**, keine unabhängige Rootabnahme.
Basis aller Autorenchecks ist ausdrücklich **d3fca66c78931864fd07104fa212d944c1a6778e**.
NET-04.2c-Brief vollständig gelesen und frisch geprüft:
`3dff58ac5d1b9511862cf050018fb838ea5f8372b8768fd56a661ab949efd04c`;
Rootprüfung `NET-04_2c_COLLECTOR_SESSION_BINDING_REVIEW.md` und Assessment
`CLEAN_CONTRACT` bleiben Grundlage. Die historischen SIM-03.4-Briefe,
Deliveries/Freezes und Rootboards wurden nicht verändert. Kein Git und kein
zusätzlicher Agent; kein Schreiben außerhalb des genehmigten Sim-/Artefakt-/
neuen Deliveryscopes.

## Konkretes Verhalten und tatsächliche Producer

`initialize_collector_lifecycle(CollectorLifecycleLimits)` wählt den Producer
explizit einmal auf dem leeren tatsächlichen Ownershipowner, nach tatsächlicher
Publicationinitialisierung. Zelllimit ist checked **Playerlimit + Groupcap**
(höchstens128+256=384), Pinlimit genau die echte Groupcap. Der positive
Scalarbytecap ist höchstens1MiB. Vor Aktivierung werden exakt Vecheader plus
`count * size_of::<Option<CollectorLifecycleCell>>()` checked admitted und
fallible vorreserviert. Zellen besitzen ausschließlich scalare Admission-/
Generation-/Weak-/Detach-/Pinmetadaten; kein FCIS-/FCIE-/NBTbody oder Storehandle.
Diese logische Outercharge ist keine tatsächliche Heap-/RSS-/OOMgarantie.

`connected_owned_player_view(&OwnedPlayerHandle)` borrows echte PlayerState
und den Ownershipowner: private Owneridentity, ursprüngliche Admission,
volle aktuelle Chunk-/Entityscope-/Generationbindung, connected und nicht
stopped. Sie liefert tatsächliche Position-/Lookbits und GameMode; Seedhash
kommt unmittelbar aus dem bestehenden
`BiomeZoomSeed::from_world_seed(state.binding.generation().identity().seed()).bits()`.
Fresh Bootstrap meldet Survival; genuine Mode-/Poseänderungen und Restore
werden übernommen. Drain entwertet connected Outputauthority nicht.
SnapshotOnly, foreign/stale/detached und Scopeabweichung verweigern.

Bootstrap/Restore/Rejoin prüfen Zellcredit vor Bodymaterialisierung. Beim echten
Pendinginstall reserviert Bootstrap seine eigene Zelle; echte Confirmation
versiegelt diese mit dem dann admitted Handle. Definitiver Abort gibt genau
diesen Credit frei, Uncertainty/Refresh/Quarantine behalten ihn ohne erfundene
Admission. Restore/Rejoin reservieren und versiegeln nach vollständig erfolgreichem
Preflight vor ihrem Admissionintent. Rejoin erhält stets eine neue Zelle;
pinlose alte, noch nicht acknowledged Detachfacts bleiben gegen die Cap gezählt.

`OwnedPlayerInput::disconnect` mintet den sealed `NativePlayerDetachFact` direkt
im wirklichen Apply nach `connected=false`. Er bindet Originalowner, Player,
Admission und volle Generation. Wiederholung borrows denselben behaltenen Fact;
historischer Disconnect nach Rejoin verändert den neuen PlayerState nicht.
Die bestehenden UUID-only `PlayerDetached`-Intents werden weiter ausgegeben,
beweisen aber keine Detachmentauthority. Native Facts/Views besitzen weder
public Constructor/Clone noch Weakgetter oder herausgebbares Mutationshandle.

Jedes echte `take:Some` in Lifecyclemode trägt genau einen originalen Zellpin.
Pinpreflight liegt vor den Candidate-/Bodykopien und Workrequest. Die private
Preparedpublication bewegt den scalaren Pin beim Pendinginstall in reserved,
beim Confirm in retained, beim definitiven `release_pending`-Abort oder echten
FIFOheadack genau einmal aus dem Ledger. Busy/Historyrefusal/Lostreply/
ReceiptAbsent/NeedsRefresh/Quarantine/Detach behalten ihn; Rejoin retargetet
weder Gruppe noch Pin. Gruppen ohne Take tragen keinen Collectorpin.

Der wirkliche Countershadow verwendet dieselben fixed-size `LifecycleCredits`-
Prädikate und Übergänge: echte Completionprefixe bestätigen/behalten oder
releasen Credits; neue Restore-/Bootstrap-/Pickupadmission prüft danach die
zukünftige Cap vor weiteren Shadowresources. Sein bestehender Scratchcharge
zählt den tatsächlichen neuen `ShadowResources`-Layoutanteil mit.

`collector_lifecycle_view(original)` borrows die echte historische Zelle auch
nach Rejoin. Detached + reserved0 + retained0 mintet ein genaues
`CollectorLifecycleTicket`; Ack revalidiert konkrete Zellslot-/Creditidentity,
private Originalowner-/Admission-/Generationbindung und beide Pinzahlen0.
Foreign/stale/repeated Release verändert nichts. **Driver muss zuvor genuine
Originalsessionretirement und Consumerrefs0 separat beweisen**; das Ticket
belegt ausschließlich Simzustand und ist kein Netzwerk-/Socketreceipt.
Drain erlaubt Gruppen-/Lifecycleacks. Jede ungelöste Zelle blockiert Stop und
wirkliche Schedulerquiescence. Snapshotclone enthält weder runnable Ledger
noch Detachfacts/Pins; der unveränderte durable Checkpoint enthält diese
flüchtigen Producer nicht.

Die tatsächlichen neuen Producer-/Integrationspins stehen in
`.codex-tmp/sim03_4a/producer-pins.json` (8 Dateien). Der ganze finale Simstand
steht in `.codex-tmp/sim03_4a/source.freeze.json` (56 Dateien), SHA256
`a47209ad338893f42d62ecd9c3cf4a24f724d5ba63809fbe216729641c7f7c96`.
Es sind15 gegenüber dem vor Arbeit frisch aufgenommenen Simstand geänderte/
neue Dateien, vollständig im Simscope. Dependencies/Cargo.lock bleiben
unverändert.

## Meaningful Abnahme auf Basis d3fca66c

18 neue Runtime-Tests:12 im tatsächlichen `player_entity_transfer`-Integrationstest
und6 beim echten Counterconsumer; dazu4 neue compile-fail Rustdoctests für
fehlende View-/Fact-Clone-/Constructorauthority. Memory und redb werden durch
echte Backendadmission, echte Workrequests/Completions, tatsächliche Handles
und retained Gruppen benutzt. Fehleradapter liefern gezielte Backendfehler;
sie konstruieren keine Receipt-/Detach-/Live-/Completionfacts.

| Tatsächlicher geprüfter Pfad | Befund |
|---|---|
| Fresh Memory/redb-Bootstrap, gespeicherter Restore/Rejoin | echte Survival/Creative-, Position-/Lookbits und unabhängiges Seed37-Hashfixture `8828412754801553711` |
| Gleiche UUID in zwei tatsächlichen Ownern, SnapshotOnly, alte Admission | kein foreign/stale/snapshot Liveproof; Drain behält noch connected Liveview |
| Nur enqueued Disconnect, echter Apply, wiederholter alter Disconnect nach Rejoin | Fact erst nach Apply; gleiche behaltene Factadresse; neuer Player bleibt connected |
| Reserved Pickup → Detach → Confirm → Rejoin → tatsächlicher FIFOheadack | originaler Pin reserved→retained→0; alter Collector wird nie auf neue Admission umgebunden |
| Tatsächlicher Partial-/Fullpickup; Animation aus/an | Changedbody mit Originalcount5/moved1 bzw. Fullremove; genau ein Pin nur bei `take:Some` |
| Busy, Commit-/Executionuncertainty, ReceiptAbsent, Currentreadfailure | originaler Pin und Zelle bleiben; tatsächliche Retry/Clarification bestätigt ohne zweiten Pin |
| Echte CASrefusal ohne Countermode; echte ReceiptCollision im Countermode | definitiver Abort gibt genau reserved Pin/Bootstrapzelle frei |
| Completionabort + neuer Pickup/Bootstrap in derselben Counterboundary an voller Cap | zukünftiger Shadowcredit stimmt mit tatsächlicher Ressourcenadmission überein |
| Superseded Receipt und tatsächliche Counter-CASquarantine | kein Inventory-/Zell-/Pinrelease und kein terminaler Zeroproof |
| N−1/N/N+1 für Zellen/Pins und exakte Outerbytes | neue Admission vor Mutation verweigert; pinlose alte Facts zählen weiterhin |
| Foreign/stale/repeated Zellack, wiederverwendeter Zellslot; reordered/doppelter Gruppenack | kein unberechtigter Credit-/Pinrelease; bestehende SIM-03.4-Foreign-/FIFOregressionen weiterhin grün |
| Counter-Scratch-/Scopeverweigerung und vorhandene Tick-/Generationoverflowregressionen | Inputs, Tick und Lifecyclecredits erhalten |
| Actual Drain/Stop/Schedulerquiescence | letzte Gruppen- und Lifecyclecredits müssen genuinely erledigt sein; stopped Countertick verweigert |

WholeSim Debug und Release: jeweils **360 Unit-/Integrationstests + 9
Rustdoctests = 369 bestanden**, ohne Fehler, ignorierte oder gefilterte Tests.
Davon sind 237 Unit- und 123 Integrationstests; die acht Integrationssuites
haben 10/14/19/18/46/3/6/7 Tests.
Alltargetclippy `-D warnings`, Workspacefmt `--check` und Rustdoc
`RUSTDOCFLAGS='-D warnings'` sind auf dem finalen Source grün.
Exakte Befehle, Exitcodes, Source-/Loghashes stehen je Check in
`.codex-tmp/sim03_4a/{debug,release,clippy,fmt,rustdoc}.json` und `.log`.
Alle fünf Records pinnen denselben oben genannten finalen Sourcefreeze.
Das finale Autorenassessment steht in `.codex-tmp/sim03_4a/assessment.json`;
`.codex-tmp/sim03_4a/final-audit.json` hält die frische vollständige Prüfung
von Source, privater Kopie, Archiv, Outsidebytes, Referenzpins und Testcounts fest.
`.codex-tmp/sim03_4a/evidence.freeze.json` pinnt diese Delivery und sämtliche
Autorenrecords einschließlich Logs; der Freeze pinnt sich selbst nicht.

Die private Kopie `.codex-tmp/sim03_4a/validation` stammt ausschließlich aus dem
**readonly** Rootarchiv `.codex-tmp/gen05_7-root-runtime-base/archive`.
Das gesamte Archiv mit1224 Pfaden/Dateihashes und Manifest-SHA256
`c148936b29cc58d7ac25b2904618523c3b3a163476a2bf380b1d00fadf07c7b0`
wurde vor Kopie und erneut vor/zwischen Checks frisch vollständig geprüft.
Nur der ganze finale Simstand wurde überlagert. **1172 Outside-Dateien**
werden mit exakter Pfadmenge und Hashes vor/nach jedem Check unverändert geprüft;
keine World-/Storage-WIPbytes wurden aufgenommen. Reproduktion:
`python3 .codex-tmp/sim03_4a/checks.py debug clippy fmt rustdoc release`,
Cargo `/home/jstrauss/.cargo/bin/cargo`, Jobs2, eigener Targetdir.
`outside.pre.json` und `outside.post.json` dokumentieren die vollständigen
Vor-/Nachprüfungen. Der abschließende Audit prüft zusätzlich die exakte
56-Dateien-Pfadmenge des Shared- und Private-Simstands gegen den Sourcefreeze.
Root muss später die tatsächliche neue Kombination mit World-/Storageintegration
unabhängig prüfen; diese Basis-d3fca66c-Checks behaupten keine solche Abnahme.
Root meldete inzwischen den veröffentlichten Head
`e772564c06445b9309a9294460c8988e7269916a` mit GEN05.7 und Docsintegration.
Dieser neue Head wurde hier nicht als Autorencheckbasis verwendet; die
Kombinationsprüfung muss den finalen Simstand auf Roots tatsächlichem
Integrationsstand prüfen. Sim bleibt exklusiver, nicht commiteter Runtime-WIP.

## Referenz, Grenzen und Folgepflichten

Minecraft1.21.8/772, Paper Build60,
`29c8822d90899c89d2689338e81a98f690bcba12`. Alle25 eingefrorenen NET-Referenzpins
wurden frisch geprüft. `reference-pins.json` trennt diese wiederverwendeten
Rootvertragsnachweise von9 eigenen genauen Source-/Referenzlesescopes und Hashes:
volle Vanilla-/applied Take-Methode samt Collectorpatch, tatsächlich vollständige
ChunkMap-Broadcast-/Removalcallees, vollständiger Trackerdesyncfeaturepatch,
Vanilla-/Paper-`BiomeManager.obfuscateSeed` und tatsächlicher Rust-BiomeZoomSeed.
Vanilla Take broadcastet vom Item, applied Paper vom Collector einschließlich
Self. Diese Audienceabweichung bleibt im späteren Sessionconsumer erhalten.

Diese Tests sind Sim-Contractabnahme, **kein gestarteter Paper-Differenziallauf**.
Es gibt keine Sessionretirement-/Consumerref-/JoinGame-/Endpoint-/Writerreceipt-
Implementierung in diesem Ticket. Die tatsächliche Driver-/Sessionbrücke muss
originale admitted Handles und den borrowed Fact verwenden und vor Lifecycleack
Retirement/Consumerrefs0 beweisen. Native Appaktivierung, Itemphysik, Javaevents,
Plugins, End/Dragon, Live-Survival und Performance sind damit nicht abgenommen.
Bestehende infallible Item-/NBTdeepclone-, Arc-/BTreeMap-Allocatorgrenzen bleiben;
neue scalar Outerallocation ist fallible, keine all-OOM-/RSSgarantie.
