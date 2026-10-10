# SIM-03.2 — unabhängige Entity-Ownership-Review

Status: **CLEAN für den finalen eingefrorenen SIM-03.2-Vertrag nach F1-Fix**.
Reviewer: `select_world_ticket`, 2026-10-10. Ein bestätigtes Finding wurde
unabhängig RED reproduziert, vom Implementer korrigiert und gegen denselben
Reproduktionstest GREEN geprüft. Keine weiteren bestätigten Findings.

Schreibbesitz im Hauptrepository ausschließlich dieses Reports. Der Reviewer
änderte weder Produktionscode, Producer, andere Dokumente, Git noch Lockfiles.
Eigene Rustprobes und Buildartefakte liegen nur im isolierten Reviewcheckout
beziehungsweise `.codex-tmp`. Root besitzt Integration; `feature_catalog`
besitzt die SIM-03.2-Implementation. Die nächste Storageimplementation benötigt
ein eigenes von Root freigegebenes Ticket.

## Geprüfter Stand und Isolation

Root stellte den Checkout
`.codex-tmp/sim032-review-checkout` auf Basis `62a5b74d` bereit und ersetzte nach
F1 genau die sieben freigegebenen Freeze-Dateien durch ihren finalen Stand.
Andere bewegliche App-/Worldänderungen wurden nicht in den Build aufgenommen.
Die unabhängigen Probes blieben außerhalb des Producerfreezes.

Vor und nach den finalen Prüfungen bestätigte `sha256sum --check` sowohl im
Hauptrepository als auch im isolierten Checkout **7/7 identische Dateien**.
Owned-Kopie des abschließenden Manifests:
`.codex-tmp/sim032-review-freeze.sha256`; Implementermanifest:
`/tmp/sim032-freeze.sha256`.

| Datei | Finaler SHA256 |
| --- | --- |
| `server/crates/ferrumc-sim/INVARIANTS.md` | `258dcd1b27c4d81757c3c3f7e7c1d1b83c90c983dbff3811bfcc2d5646e561dd` |
| `server/crates/ferrumc-sim/README.md` | `6d4b282862012ef30cd4e04489df45f6439937d3800bc5ccee20347aa8fd7f3f` |
| `server/crates/ferrumc-sim/src/lib.rs` | `ee5c228c9340e4eda65f73948b7ea1feafb7d5c23996f4da0ea165745b9715ec` |
| `server/crates/ferrumc-sim/src/shard.rs` | `f5160216368a4481271b576c4aee2d48e0daf6bd196e3b3e708caac88393173f` |
| `server/crates/ferrumc-sim/src/entity.rs` | `7a05e682c83caa8a058f10d9143fa8bb992546ac48dccd203c3487024b687894` |
| `server/crates/ferrumc-sim/tests/entity_ownership.rs` | `3d0b22c95f104d5a043681a7f5983ebe192ef39d5f934daf6f30ec32f04032b4` |
| `docs/agent-tasks/SIM-03_2_ENTITY_OWNERSHIP.md` | `c8c045c095ce18e22dee23c99c1b6f81ad6b75335d0412da4fef960eafa22b2a` |

Buildtarget ausschließlich `.codex-tmp/sim032-target`, mit
`/home/jstrauss/.cargo/bin/cargo`, `--offline --locked -j2` aus dem isolierten
`server/`. `server/target` wurde nicht verwendet. Dieser Reviewtarget ist nach
Abschluss der aufgeführten Läufe frei.

Produktionsmodul `entity.rs`, alle Entitytests, die geänderten Shardmethoden,
öffentlichen Exports und Dokumentationsverträge wurden vollständig gelesen.
Zusätzlich: kanonischer `ShardId`-/Regionvertrag, EntityUuid-/Wireidentitäten,
der geprüfte EntityTypeId-Producer, bestehender LoadedChunkMap-/Ticketvertrag
und Actor-Sharding-Architektur. Die App und der Shadowscheduler aktivieren
`initialize_entities` beziehungsweise `run_tick_with_entity_work` noch nicht.

## Bestätigtes Finding F1 und unabhängiges RED→GREEN

**F1: Ausgetauschte Chunkmaps konnten eine fremde Welt-/Dimensionsresidenz als
Admissionbeleg für Entities des ursprünglichen Owners verwenden.**

`SimShard::loaded_chunks_mut()` erlaubt den Austausch der kompletten
`LoadedChunkMap`. Nach Initialisierung des EntityStores in world7/dimension13
konnte eine Map aus world8 oder dimension14 eingesetzt und Chunk `(0,0)` darin
geladen werden. Der vorherige Entitytickentry verglich Mapscope und Storeowner
nicht. Ein gültiger world7/dimension13-Spawn wurde deshalb angenommen; auch
die gewöhnliche PlayerJoin-Inbox wurde verarbeitet.

Der eigene unveränderte Reproduktionstest
`sim032_review_scope::replacing_the_public_chunk_map_cannot_admit_entities_against_a_foreign_scope`
verlangt Ablehnung am Tickentry sowie leeren EntityStore, erhaltene Inbox und
unveränderte Playerzahl. RED lieferte tatsächlich ein erfolgreiches EntityHandle
zusammen mit `PlayerSpawned`. Log:
`.codex-tmp/sim032-review-scope-red.log`.

Der Implementer ergänzt im finalen `run_tick_with_entity_work` einen Vergleich
der aktuellen Mapworld/-dimension mit dem unveränderlichen Storeowner,
**vor Contextkonstruktion, Closure und gewöhnlichem Inboxdrain**. Abweichung
liefert `ScopeMismatch { owner, world, dimension }` mit dem tatsächlichen
Mapscope; der Store wird weder ersetzt noch reinitialisiert. Während der
Closure verhindern die beiden Borrows einen erneuten Map-/Storeaustausch.

Derselbe unabhängige Reproduktionstest besteht gegen diesen Freeze GREEN.
Log: `.codex-tmp/sim032-review-scope-green.log`. Probe-SHA256:
`208098a1e48408264d319981a1e83e5677914a0207e72539f6abfc0e995ee8bf`.

Die zusätzliche eigene Probe `sim032_review_scope_preservation` verwendet eine
bereits registrierte Cow, einen gültigen Livehandle, einen negativen Owner und
signed Wire-Minimum. Welt-, Dimensions- und kombinierte Scopeabweichungen
verweigern Remove-Arbeit sowie einen bloßen Residencyquery jeweils, ohne die
Closure auszuführen. Daten, UUID-/Wire-/Chunkindex, Handle, Chunkticket, Inbox
und Playerzahl bleiben unverändert. Nach Wiederherstellung der Originalmap
funktioniert die weitere Arbeit mit demselben Handle und verarbeitet die
erhaltene Inbox. Auch die beiden neuen Producerregressionen mit Livehandle,
Never-Closure und Inboxerhalt wurden vollständig gelesen und bestanden.

## Referenz- und Verhaltensmatrix

Referenz: Minecraft Java **1.21.8**, Protokoll772, Paper Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Der Gitstand wurde erneut geprüft.
Basissourcen und angewandte Quellen aus `.codex-tmp/paper-1.21.8-reference`
wurden gelesen. SHA256-Nachweise der angewandten Quellen und relevanten Patches:
`.codex-tmp/sim032-review-reference.sha256`. Die Tabelle unterscheidet
vollständig gelesene Dateien von relevanten vollständigen Methoden/Hunks;
eine vollständige Review der gesamten Papercodebasis wird nicht behauptet.

| Verhalten | Gelesene echte Implementierung | Urteil für dieses Ticket |
| --- | --- | --- |
| Identität und Lookup | Vollständige aktive Moonrise `EntityLookup` und `server/ServerEntityLookup`; Vanilla/Base- und angewandte `world/level/entity/EntityLookup`, vollständiger zugehöriger Sourcepatch; Entity-ID-/UUIDgetter und Setter. | UUID, signed Wire-ID und registrierter Typ haben unterschiedliche Aufgaben. Moonrise preflightet ID/UUID und nimmt bei UUIDkollision die IDzuordnung zurück; Entfernung prüft die konkrete Entityreferenz. FerrumCs eigene Incarnation-/Generationhandles verhindern veraltete oder fremde Objektzugriffe. Prozessweite UUID-/Wireallocation gehört noch nicht zum Producer. |
| Räumliche Zuordnung | Vollständige Moonrise `ChunkEntitySlices` und `EntityLookup`, vollständige Vanilla/Paper `EntitySection` und `EntitySectionStorage` samt SectionStoragepatch. `Entity.setPosRaw`, `Mth.floor`, `SectionPos.blockToSectionCoord` und passende Original-/Moonrisehunks. | Original leitet Chunks aus Floorpositionen ab und führt räumliche Indizes. FerrumC verwendet genau 64 feste Buckets innerhalb seines kanonischen 8×8-Owners und verweigert Cross-Ownerbewegung. Die 8×8-Grenze ist FerrumCs Architektur; Moonrises 32×32-Indexregion ist kein Beleg für diese Ownergröße. FerrumCs Chunkquery scannt keine vollständige Slotliste. |
| Registrierung, Zugriff und Ticking | Vollständige Vanilla/Base- und angewandte `TransientEntitySectionManager`, `PersistentEntitySectionManager`, `EntityTickList`, `Visibility`, `EntityInLevelCallback` und `LevelCallback`; Managerpatch; vollständiger Moonrise Status-/Callbackpfad; vollständige `ServerLevel.EntityCallbacks`. | Inaccessible Registrierung ist von öffentlichem Zugriff, Tracking und Entitytickliste getrennt. Upgrade trackt vor Ticking; downgrade stoppt Ticking vor Tracking, mit Reentrancyguards. Der Producer bietet nur Registrierung und ticketbezogene Zieladmission, ohne Aktivierungs-/Tracking-/Tickstatus. |
| Add/Remove und Events | Vollständige `ServerLevel.addFreshEntity`, `addWithUUID`, `addDuringTeleport`, `addPlayer`, `addEntity`, `tryAddFreshEntityWithPassengers`; `Entity.kill`, `discard`, `remove`, `setRemoved`, `unsetRemoved`, `shouldBeSaved`; Moonrise add/remove/move; `PaperHooks.screenEntity`/`unloadEntity`; `CraftEventFactory.doEntityAddEventCalling` und `callEntityRemoveEvent`; relevante Entity- und Moonrisepatchhunks sowie kompletter EntityScheduleroptimierungspatch. | Paper besitzt Spawn-/Removeevents, Passagiere, Player-Sonderpfade, Visibilitycallbacks und Scheduler-Lifecycle. Rustplayer werden bewusst nicht in diesem Non-playerstore dupliziert. Remove liefert Basisdaten und keinen Persistenceack. Events, Passagiere und Entityscheduler wurden nicht als umgesetzt gewertet. |
| Load/Unload und Save | Vollständige `ChunkEntitySlices` save/read/unload/move- und Indexpfade; Manager load/save/unload einschließlich Sourcepatch; vollständige `NewChunkHolder.loadInEntityChunk` und `unloadStage1/2/3`; `PaperHooks.unloadEntity`; kompletter Entity-load/save-limit-Patch. | Diskload, Deserialisierung, Registrierung und transiente Retention unterscheiden sich. FerrumCs Registrierung überlebt das letzte Ticket; residenter Zielspawn/-move wird danach verweigert, Cleanup bleibt möglich. Es werden keine UUIDrecords gespeichert, gelöscht oder automatisch zurückgeladen. |
| Position und Rotation | Vollständige originale/relevante angewandte `Entity.setRot`, `setYRot`, `setXRot`, `checkPosition`, `setPosRaw` und deren Source-/Moonrisehunks; direkte Floor-/Sectioncallee. | Für endliche Inputs stimmen yaw `%360` und pitch `clamp(pitch%360,-90,90)` mit dem gelesenen Pfad überein. Nichtendliche Inputs werden explizit verweigert; Papers `setRot` repariert einige davon. Horizontales ±30M, vertikales ±20M sowie der Slotdeckel sind dokumentierte FerrumC-Admissionalimits, keine Original-Minecraft-Limits. Andere Setterpfade werden nicht gleichgesetzt. |

## Unabhängige Prüfnachweise

Alle finalen Läufe verwenden die sieben abschließenden Freeze-Dateien.

| Prüfung | Ergebnis | Eigenes Log |
| --- | --- | --- |
| ursprüngliche F1-Probe | RED vor Korrektur, derselbe Test GREEN danach | `.codex-tmp/sim032-review-scope-{red,green}.log` |
| kompletter Sim-Debuglauf einschließlich eigener Probes | **164 Unit + 56 Producerintegration + 4 eigene Integration + 3 Doc = 227 bestanden**, keine fehlgeschlagenen/ignorierten Tests | `.codex-tmp/sim032-review-final-tests.log` |
| kompletter Sim-Releaselauf einschließlich eigener Probes | **dieselben 227 bestanden** | `.codex-tmp/sim032-review-release.log` |
| `cargo clippy -p ferrumc-sim --all-targets -- -D warnings` | bestanden, einschließlich der eigenen Probes und Abhängigkeiten | `.codex-tmp/sim032-review-clippy.log` |
| `cargo fmt -p ferrumc-sim -- --check` | kompletter Producer-/Paketbestand bestanden; eigene Scratchprobes während dieser reinen Formatprüfung außerhalb der Paketdiscovery, danach zurückgestellt | `.codex-tmp/sim032-review-fmt.log` |
| Main- und Reviewcheckoutfreeze | vor/nach **7/7 identisch** | `.codex-tmp/sim032-review-freeze.sha256` |

Die 56 Producerintegrationstests teilen sich auf: Chunkloading10,
Entityownership14, Generationbinding19, Shardownership6 und Tickharness7.
Die vier eigenen Tests liegen in drei eigenen Scratchtargets:
`sim032_review_model`2, `sim032_review_scope`1,
`sim032_review_scope_preservation`1.

Der eigene modellbasierte Test wählt **12.000 deterministische Interleavings**
über drei positive/negative Owner. Er belegt zu Beginn jedes Owners alle 64
Buckets und hält als unabhängige Erwartung eine einfache UUID-BTreeMap mit
Wire-ID, Typ, Position, Rotation und mathematisch abgeleitetem Chunk. Er kopiert
weder Intrusivelinks noch Slot-/Freelist-/Generationimplementation. Spawn,
Move, Remove, Duplicate- und Scopeablehnung, alte Handles, Clone und
Store-Neuerzeugung werden verschränkt; nach jedem Schritt werden alle Buckets,
ExactSize/Fusediteration, UUID-/Wirelookup, Handlelookup und Gesamtinhalt
verglichen. Fehlgeschlagene Operationen erhalten den vorherigen Datensnapshot.

Eine weitere eigene Probe behandelt sämtliche **151 Originalregistriertypen**:
150 Non-playertypen werden unverändert angenommen, Player wird atomar
verweigert. Signed Wire-Minimum/-Maximum und semantisches ±0 für Position und
Rotation werden geprüft. Ihr Erfolg bedeutet Typidentität, nicht implementierte
Verhaltensklassen oder gültige natürliche Spawnbedingungen aller Typen.

Eigenes Modellprobe-SHA256:
`a53d002f5c630e224a8fa49a479e240e571e07d63d4e715c7b1582938b260c7f`.
Eigene Livehandle-/Scopeprobe-SHA256:
`e12d051a583e146de7c06a141162c08e473299f3852bbe07878195d5d0d7dc8a`.

Private Retirementtests und Produktionspfad wurden gesondert gelesen:
`u64::MAX-1` erlaubt eine letzte Reusegeneration, `u64::MAX` retiert dauerhaft;
retired Slots zählen zur begrenzten Kapazität, laufen nie um und verhindern kein
anderes freies Slot. Ihre Tests bestehen in beiden Profilen. Keine Milliarden
von Removeoperationen oder künstliche öffentliche Handlefabrikation waren nötig.
Slots wachsen lazy; Free-Reuse, Lookup, Chunkquery und Transformupdate benötigen
keine neue Heapreservation. BTreeMap-Prozessallokationsfehler sind ausdrücklich
kein recoverable `Result`, weshalb dafür keine No-op-Rückgabe behauptet wird.

## Grenzen des CLEAN-Urteils

CLEAN betrifft **SIM-03.2 als bounded owner-local Entityproducer und seinen
expliziten Entitytickentry**. Es bestätigt keine aktivierte Mob-/Entitysimulation,
Netzwerktracking, Velocity-/Kollisionsphysik, AI, natürlichen Spawn, Passagiere,
Item-/Inventarownership, persistente UUIDrecords, Cross-shard-/Dimensiontransfer,
Java-/Paperevents, vollständiges Survival oder vollständigen Drop-in-Betrieb.
Auch automatische Entityarbeit im App-/Shadowscheduler wurde nicht aktiviert.

Der Entitywork-Closurevertrag ist ausdrücklich keine Transaktion: vorherige
erfolgreiche Operationen und der gewöhnliche Tick laufen bei einem als Wert
zurückgegebenen Closurefehler weiter. Panic bleibt fail-stop; das Review
behauptet weder Rollback noch sichere Wiederholung nach partieller Arbeit.

Kein tatsächlicher Paper60-Server wurde live als Entityoracle ausgeführt und
kein neuer CPU-/RSSbenchmark erhoben. Die Referenzmatrix ist eine unabhängige
Quellen-/Vertragsprüfung; echte Client- oder JARdifferentialakzeptanz späterer
Consumer ist weiterhin erforderlich. Nach dem bestätigten F1-Fix verbleibt für
den hier eingefrorenen Vertrag kein bestätigtes offenes Finding.
