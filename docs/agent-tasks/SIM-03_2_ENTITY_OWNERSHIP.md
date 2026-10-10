# SIM-03.2 — Begrenzte shardbesessene Entity-Verwaltung

Status: Implementierung und lokale Prüfung grün, Reviewfinding F1 behoben und
erneut eingefroren; unabhängiger Fixreview und Integration stehen aus. API und Consumerstart wurden von Root nach dem
separat geprüften Registryproducer freigegeben. SIM-03.1 liefert `EntityUuid`
als `6998a76e`; REG-ENTITY001 liefert den validierten `EntityTypeId` und alle
151 Originaltypen als `cd32fbd2`. Beide Producer bleiben unverändert.
Produktions-Crate ausschließlich `ferrumc-sim`; keine aktive App-Anbindung.

## Ziel und Besitz

Auf dem getrennten `EntityUuid`-Producer aus SIM-03.1 entsteht eine echte
besessene Entityverwaltung für den späteren Item-/Mobconsumer. Entityuuid,
Wire-ID und lokaler Handle erfüllen unterschiedliche Aufgaben. Kein Socket,
kein Raw-DB-Handle und keine globale Entitymap werden eingeführt.

Besitz bei Vergabe: `server/crates/ferrumc-sim/**` und dieses Dokument.
WG-003 ist als `da61c638` integriert; seine öffentlichen APIs und Tests bleiben
kompatibel. Keine Core-/Storage-/World-/Registry-/App-/Lock-/Gitänderung.
Root besitzt Board/Katalog. Vor Umsetzung AGENTS/CLAUDE, Sim-INVARIANTS,
Ownership-/Architektur-/ADR- und vollständige Referenzquellen lesen.

## Abgestimmter begrenzter Vertrag

- Owned bounded `EntityStore`, Entityuuid- und Wire-ID-Index sowie typisierter
  Chunk-/Spatialindex. Kein linearer Scan sämtlicher Entities pro Chunkquery.
  Initialisierung, zulässige Kapazität und voller Store haben konkrete,
  dokumentierte Fehler-/Backpressureverträge; keine Default-unbounded Maps.
- Registrierter Entitytyp und begrenzte endliche Basisposition/Transformation
  werden validiert. Nur Basisdaten und sichere Spawn/Remove/Move/Lookup-
  Operationen; keine vorgetäuschte KI, vollständige Metadata, Kollision,
  Living-/Itemkomponenten oder automatische Zufalls-/Wire-ID-Erzeugung.
- Ein opaquer generationaler lokaler Handle verhindert stale Zugriff nach
  Remove/Slotreuse. Overflow darf keinen alten Handle wieder gültig machen.
  Umgang mit fremdem Store/Owner und neuer Store-Inkarnation explizit entwerfen;
  Handles nicht als permanente UUID oder Wire-ID serialisieren.
- Alle Mutationen eines Owners synchron an dessen Tickgrenze. Fehlende,
  doppelte, malformed oder über der Kapazität liegende Spawns ändern keinen
  Index und keine schon vorhandene Entity. Move/Remove/Chunkquery bewahren
  sämtliche Indexkonsistenz einschließlich negativer Chunk-/Shardgrenzen.
- Store/Resident-Lifetime, scope-/shardgebundene Aufnahme und tatsächlicher
  Chunkload-/Tickstatus sind getrennt. Noch fehlende Cross-shard-Transfers,
  Persistenz, Tracking und global prozessweite Wire-ID-Allocation benennen.
  Keine Doppelinstanz als vollständige globale Duplicateuuidprüfung ausgeben.
- Konsistente SimShard-Anbindung mit eigenen Accessoren/Admissionmethoden
  und klarer Tickgrenze, soweit ohne fehlenden Storage-/Sessionconsumer möglich.
  Bereits vorhandene Spieleridentität/-zustände nicht in einen zweiten
  konkurrierenden Owner kopieren. Fehlende aktive Verbraucher bleiben offen.
- Keine Allocation beim erfolgreichen Lookup/Transformupdate; Spawn kann
  bounded lazy Growth nutzen. Queryergebnisse bevorzugt borrowed Iteratoren
  oder ausdrücklich bounded Paginierung, kein unbegrenzter materialisierter Vec.

Das konkrete API-/Handle-/Scope-/Kapazitätsdesign vor Produktionsänderungen
an Root melden. Falls fehlender Producer die korrekte Einbindung verhindert,
präzisen fehlenden Vertrag melden und das Ticket vor dieser Grenze teilen.

## Originalreferenz und Abnahme

MC1.21.8 / Protokoll772, Paper Build60 / Commit
`29c8822d90899c89d2689338e81a98f690bcba12`.
Vollständige relevante Entity-ID/UUID-/Removal-Methoden und Callees,
`EntityLookup`, `EntitySection`, `EntitySectionStorage`,
`PersistentEntitySectionManager` beziehungsweise die tatsächlich aktiven
Moonrise-Entitylookup-/Chunkconsumer lesen. Relevante Source-/Featurepatches
für Duplikate, Visibility, Tick/Tracking, Unload und Callbacks mitlesen.
Vanilla-/Paperverhalten, zusätzliche FerrumCcaps und fehlende Event-/
Trackingconsumer im Taskdoc trennen; Sourcelektüre ist kein Livevergleich.

- Öffentliche Tests für mehrere registrierte Entitytypen, UUID-/Wireduplikate,
  Scope-/Shard-/Positiongrenzen, volle Kapazität und unveränderte Fehlerzustände.
- Spawn/Move/Remove/Reuse mit konsistenten UUID-/Wire-/Chunkindizes;
  deterministische Interleavings, mehrere Entities im Chunk, negative Grenzen,
  Pagination-/Budget- und leere Queries; stale/fremde Handles verweigern.
- Private Slotgeneration-Max-/Retirementfälle, keine Wraparoundaliasierung.
  Snapshot-/Cloneverhalten ausdrücklich prüfen, falls es angeboten wird.
- Unterschied zwischen geladenem, tickendem, ausgeladenem und entfernten
  Zustand dokumentieren; kein Dauerhaftigkeits-Ack ohne noch fehlenden Store.
- Vorhandene Simtests inklusive WG-003; strict all-targets Clippy und fmt.
  Source-/Behavior-/Event-/Ownership-/Deviation-/Validationmatrix abschließen.
- Exaktes SHA256-Freeze aller geänderten Dateien und Logs an Root;
  keine Gitoperationen. Root organisiert Fremdreview und Integration.

Item-Drop/Pickup, atomare Ownershiptransfers, Entity-Storage/Restart,
Bewegungsphysik, Trackingpakete, Mobs/KI, Nether/End und Dragonphasen bleiben
anschließende Consumers. Dieses Ticket allein schließt keine Survival-Etappe.

## Umgesetzte API und Ressourcenvertrag

`src/entity.rs` implementiert `EntityStore`, `EntityHandle`, `EntitySpawn`,
`EntityData`, `EntityTransform`, `EntityChunkIter`, `EntityTickContext` und
klassifizierte `EntityStoreError`-Refusals; `lib.rs` exportiert diese Typen.
`EntitySpawn` benötigt ausdrücklich WorldId, DimensionId, EntityUuid, EntityId,
registrierten EntityTypeId und bereits validierte Transformation. Es gibt
keine UUID-/Wire-ID-Erzeugung, keinen zufälligen Fallback und keine ungeprüfte
öffentliche Record-/Handlekonstruktion.

| API/Struktur | Umgesetzter Vertrag |
| --- | --- |
| `EntityStore::new` | Canonical `ShardId`, explizite `NonZeroUsize`, höchstens 65.536 Slots; leerer Store reserviert keine Slotliste und hat keinen unbounded/default Modus. |
| Spawn | Scope, Spieler-Typ, Ownerregion, UUID-/Wireduplikate und Kapazität vollständig vorprüfen. `Full` versus durch Retirement ausgeschöpfte `SlotsExhausted` bleiben unterscheidbar. |
| Slotliste | Bei Bedarf `try_reserve_exact`, begrenztes verdoppelndes Wachstum; Freelist-Reuse benötigt keine weitere Slotreservation. Auch dauerhaft retired Slots zählen zur Grenze. |
| UUID-/Wireindizes | Separate `BTreeMap`s auf Slotindex; höchstens ein Eintrag je lebender Registrierung. Prozessweite Duplicateprüfung ist nicht Bestandteil dieser Maps. |
| Chunkindex | Genau 64 feste Buckets innerhalb der 8×8-Region, intrusive Previous-/Next-Slots und Count; Query läuft nur über die verlinkten Bewohner ihres Chunks. |
| Handle | Opaquer Weak-Incarnation/Owner/Slot/u64-Generation-Vertrag. Fremder Store wird vor Slotzugriff verweigert. Remove erhöht checked; u64MAX wird für immer retired. |
| Lookup/Update/Query | Erfolgreiches Handle-/UUID-/Wirelookup, Transformupdate und borrowed Iteration allokieren keinen Heapgegenstand. Keine materialisierte globale Queryliste; Budgets über `take`/`skip`. |
| Iteratorordnung | Spawn hängt an; gleicher Chunk bewahrt Position, Umzug hängt im Ziel an. ExactSize/Fused, Mutation während borrowed Iteration durch Rust ausgeschlossen. |
| Transformation | Endliche xyz/yaw/pitch, x/z ±30.000.000 und y ±20.000.000 als FerrumC-Rejectioncaps; yaw `%360`, pitch `(pitch%360).clamp(-90,90)`. Chunkkoordinaten werden gefloort. |
| Snapshot | Store-/SimShard-Clone kopiert begrenzte Daten mit neuer Incarnation. Quellhandles werden verweigert; eine Kopie darf ohne externen Ownervertrag nicht als zweite aktive globale Instanz laufen. |

`SimShard` bewahrt seine bestehenden Konstruktoren und Player-/WG-003-Verträge.
`initialize_entities` initialisiert einmal optional, verweigert falsche
Kapazität/ungültige Ownerregion ohne Storeaufnahme und ersetzt keinen bereits
initialisierten Store. `entities()` ist ausschließlich read-only.

`run_tick_with_entity_work` erzeugt innerhalb seines Aufrufs die nicht öffentlich
konstruierbare `EntityTickContext` mit `&mut EntityStore` und
`&LoadedChunkMap`. Die Closure läuft vor dem bestehenden gewöhnlichen Tick.
Spawn/Move prüfen zuerst Scope/Owner beziehungsweise Handle und danach einen
residenten Zielchunk. Fehlender Store verweigert vor Closure und Inboxdrain.
Ein Closure-Result-Fehler ist keine Transaktion und rollt frühere erfolgreiche
Operationen nicht zurück; der gewöhnliche Tick läuft trotzdem. Panic ist
fail-stop, kein Retrysignal. Ticknummerierung bleibt beim externen Coordinator.

Eine einzelne zurückgegebene Operationserror erhält sämtliche Registrierungen,
Indizes und Handle-Gültigkeit. Slotreservation ist vor Commit fallible.
Standardbibliothek-BTreeMap-Allocationexhaustion ist Prozessfailure, kein
recoverable Result; das Ticket behauptet keine Crashtransaktion. Releasing
des letzten Chunktickets erhält die Registrierung, blockiert weiteres Spawn/
Move in den nun nichtresidenten Zielchunk und erlaubt Cleanup-Remove oder Move
zu einem anderen residenten Chunk. Es gibt keinen Force-Load. `minecraft:player`
wird verweigert; vorhandener `PlayerState` bleibt alleiniger Spielerowner.

Jeder Entity-Tickentry vergleicht zusätzlich den aktuellen LoadedChunkMap-
Scope mit dem unveränderlichen EntityStoreowner. Die öffentliche
`loaded_chunks_mut()`-API kann die ganze Map ersetzen; eine anders scoped Map
legitimiert daher auch bei identischen geladenen Chunkkoordinaten keinen
Entity-Tick. Mismatch wird als `ScopeMismatch` mit tatsächlicher Mapworld/-
dimension vor Closure, Contextkonstruktion und gewöhnlichem Inboxdrain
verweigert. Es wird kein Store ersetzt, Chunk repariert oder Scope geraten.

## Tatsächlich gelesene Referenzquellen

Alle Pfade dieser Liste sind relativ zum gepinnten Checkout
`.codex-tmp/paper-1.21.8-reference/`; der Pin bleibt Paper 1.21.8 Build60,
Commit `29c8822d90899c89d2689338e81a98f690bcba12`. Die vollständigen angewandten
Quellen unter `paper-server/src/minecraft/java/` wurden zusammen mit Vanilla-
Basis unter `.gradle/caches/paperweight/mache/base/sources/`, relevanten
`patches/sources/`-Abschnitten und Featurepatch
`patches/features/0016-Moonrise-optimisation-patches.patch` geprüft.

- **Vollständige Dateien:** Vanilla/applied
  `net/minecraft/world/level/entity/{EntityLookup,EntitySection,EntitySectionStorage,
  PersistentEntitySectionManager,TransientEntitySectionManager,Visibility,
  EntityTickList,EntityInLevelCallback,LevelCallback}.java`; aktive Moonrise-
  `ca/spottedleaf/moonrise/patches/chunk_system/level/entity/EntityLookup.java`
  (1.006 Zeilen), `server/ServerEntityLookup.java` (167) und
  `ChunkEntitySlices.java` (602).
- **Vollständige relevante Entity-Methoden:** Konstruktor/ID-/UUID-
  Initialisierung sowie save/load/getId/setId/UUID aus SIM-03.1; für dieses
  Ticket kill/discard/remove/setRemoved/unsetRemoved/shouldBeSaved,
  Schedulerretirement, setRot/setYRot/setXRot/checkPosition/setPosRaw.
- **Vollständige ServerLevel-Pfade:** addFreshEntity, addWithUUID,
  addDuringTeleport, addPlayer, addEntity, tryAddFreshEntityWithPassengers und
  die vollständige EntityCallbacks-Klasse einschließlich Tick/Tracking/Remove.
- **Vollständige relevante Callees:** `PaperHooks.screenEntity`,
  modifySavedEntities, unloadEntity; `NewChunkHolder.loadInEntityChunk` und
  unloadStage2 mitsamt NBTload-/Deserialisierungs-/Registrierungsgrenzen;
  `CraftEventFactory.doEntityAddEventCalling` und callEntityRemoveEvent.
- **Producerquerprüfung aus REG-ENTITY001:** vollständige
  DefaultedMappedRegistry und RegistryDumpReport; relevante EntityType-
  Registrierungen/get/byString/Player sowie BuiltInRegistries-Entityfeld und
  Sourcepatch. Der vollständige originale Registriesreport und sein gepinntes
  151-Typen-Subset sind im separaten Registrytask geprüft.

Einzelne Event-Untercallees wie callItemSpawnEvent wurden nicht zusätzlich als
vollständige Spawnmechanik untersucht; diese API ruft überhaupt keine Events
auf und ist nicht als Parität dieser noch fehlenden Consumers ausgegeben.

## Source-/Behavior-/Event-/Ownership-/Deviation-/Validationmatrix

| Bereich/Quelle | Geprüftes Original-/Paperverhalten | FerrumC-Vertrag, Abweichung und Abnahme |
| --- | --- | --- |
| Vanilla EntityLookup/SectionStorage/Persistent-/Transientmanager | knownUuids, sichtbare Lookups, Sectionzuordnung und Ticklisten sind verschiedene Zustände. | UUID-/Wire-/Chunkindizes decken nur Registrierung ab; Publictests prüfen deren Konsistenz, nicht Visibility-/Tickparität. |
| Aktive Moonrise EntityLookup, ServerEntityLookup | Paper ersetzt den Vanilla Persistentmanager. Get(id/uuid) maskiert inaccessible, getAllMapped schließt Registrierungen ein. UUIDduplikat rollt zuvor eingetragenen Wireindex zurück; Remove schützt exakte Objektidentität. | FerrumC prüft beide Duplikate vor Commit und benutzt Incarnation+Slotgeneration für exakte lokale Identität. Keine Accessiblemaskierung; Queries geben ausdrücklich Registrierungen. Tests für Duplikate, Foreign/Stale, Snapshot/Recreation und Maximumgeneration. |
| Moonrise Status-/Callbackpfad; ServerLevel.EntityCallbacks | Statusupgrade lädt/trackt vor Ticking; downgrade stoppt Ticking vor Tracking. Reentrancyguards schützen Callbackwechsel. | Kein Statuswechsel, Callback, Activation oder Ticking in diesem Ticket. Sim-Tickgrenze ist Mutationsadmission und nicht bereits Original-Entitytick. Tick-Kontext-/Read-only-Compilefailchecks und reale Inbox-/Residenttests. |
| ChunkEntitySlices/NewChunkHolder/PaperHooks | NBTload, Deserialisieren, Registrieren, Residency und Accessible/Ticking sind getrennt; Unload speichert savable Entities und kann transient Entities behalten. | Ticketunload ändert Registrierung nicht. Remove bleibt möglich. Keine Entitycodec-/Saveflag-/Restart-/Tombstoneimplementation und kein Dauerhaftigkeits-Ack. Publictest unload→refused Move→resident migration→Remove. |
| Entity.setPosRaw/checkPosition/load | Der rohe Setter schützt Endlichkeit; Load clamped x/z ±30.000.512 und y ±20.000.000. | FerrumC verweigert bereits x/z ±30.000.000 und y ±20.000.000 statt Clamp. Das ist eine ausdrücklich strengere Ressourcenpolitik, nicht Setterparität. Grenz-/Nonfinite-/negativeFloor-Tests; kein Livevergleich. |
| Entity.setRot/setYRot/setXRot | Endliche Winkel verwenden Modulo360; XRot wird nach Modulo auf ±90 begrenzt. CraftBukkit-Rotationspfade können ungültige Werte reparieren. | Endliche Normalisierung entspricht dem gelesenen Pfad; Nonfinite wird atomar verweigert. Vertragstests decken positive/negative Modulo-/Clampfälle ab, keine vollständige Client-/Paper-Rotationsabnahme. |
| ServerLevel addFreshEntity/addWithUUID/addEntity/addPlayer | Unterschiedliche Spawnherkünfte, Playerowner und Passengers; Paper kann vor Add abbrechen. | Ausschließlich explizite Copy-Basisregistrierung, player-Type verweigert. Keine Spawnreason-/Passenger-/globale Wireallokation. Mehrtypentests sind Registry-/Ownershipprüfung, kein Mobspawn. |
| CraftEventFactory.doEntityAddEventCalling/remove; EntityCallbacks | Typabhängige Spawn-/Remove-/Tracking-/Weltevents, Cancellation und veränderbare Pluginzustände beeinflussen die Aufnahme. | Keine Java-/native Eventdispatch für Entities; keine Cancellation, Reentrancy oder Disableparität behauptet. Bridge-/Eventconsumer bleibt vor Gameplayactivation umzusetzen. |
| FerrumC-Ressourcen-/Ownervertrag | Paper hat keinen identischen 65.536-pro-8×8-Limitvertrag. | Capacity/refused no-op, bounded lazy slots/index sizes, keine globalen Maps/DB-/Socketbesitzer. Private Retirement-/Growthtests, öffentliche all64-Bucket-Interleavings und Rückweisungs-Snapshots. Keine behauptete CPU/RAM-Verbesserung ohne äquivalenten Benchmark. |

## Prüfstand und Freeze

Alle folgenden Läufe sind erfolgreich abgeschlossen, mit isoliertem
`CARGO_TARGET_DIR=../.codex-tmp/sim032-fix-target`, `--offline --locked -j2`:

Die abschließenden normalen Tests/Clippy/fmt laufen auf Roots isoliertem
`.codex-tmp/sim032-fix-checkout`, Basis `62a5b74d` plus genau diesen sieben
gespeicherten Ticketdateien. Das relative Target oben wurde dort absolut auf
`.codex-tmp/sim032-fix-target` gesetzt. Runtime-/Cratedocs/Testdateien stimmen
mit `.codex-tmp/sim032-fix-snapshot.sha256` überein; ausschließlich dieses
Taskdokument wird nach bestandenem Lauf um die finale Evidenz ergänzt. Damit
bleiben fremde gleichzeitig entstehende World-/Appänderungen außerhalb dieser
Fixabnahme.

| Prüfung | Tatsächliches Ergebnis |
| --- | --- |
| `cargo test -p ferrumc-sim` | 164 Unit + 56 Integration + 3 Doctests grün; 0 failed/ignored. |
| Neue private Contracts | 3 Tests: u64MAX-Retirement; letzte gültige Reusegeneration; bounded lazy growth/reuse. |
| Neue öffentliche Contracts | 14 Tests für mehrere Registrytypen, UUID/Wire/Scope/Owner/Capacity, endliche Transformcaps, negative Grenzen, Move/Remove/Reuse/Budgeting, Clone/Recreation/Foreign, alle64 Buckets, init/missing refusal, geordnete Contextpreflight, residency/unload, separaten Playerowner und ersetzte world-/dimension-scoped Chunkmaps. |
| Bestehende Integrationscontracts | Chunkloading10, Generationbinding19 (inkl. WG-003 Identity-Recheck), Shardownership6 und Tickharness7 grün. |
| Compilefail | Vorhandener typedpartition-Vertrag plus zwei neue Fälle: Contextreferenz kann nicht aus Closure entweichen; read-only Shardregistrierungen geben keinen mutablen Removezugriff. |
| `cargo clippy -p ferrumc-sim --all-targets -- -D warnings` | Grün, keine Allow-Ausnahme für neue Implementation. |
| `cargo fmt -p ferrumc-sim --check` | Grün. |

Logs: `/tmp/sim032-tests.log`, `/tmp/sim032-clippy.log`,
`/tmp/sim032-fmt.log`. Exakter sieben-Dateien-Freeze:
`/tmp/sim032-freeze.sha256`; alle Änderungen bleiben auf
`server/crates/ferrumc-sim/{INVARIANTS.md,README.md,src/lib.rs,src/shard.rs,
src/entity.rs,tests/entity_ownership.rs}` und diesem Taskdoc beschränkt.
Das isolierte Target ist nach Freeze für Review frei. Keine Gitoperationen
durch den Ticketagent. Root organisiert den unabhängigen Review; dieses Ticket
ist vor dieser Abnahme nicht als integriert/abgeschlossen markiert.

### Unabhängiges Finding F1 und Korrektur

Reviewer `select_world_ticket` bestätigte am ersten unveränderten Freeze:
Nach Initialize Owner7/13 erlaubt ein ganzer öffentlicher Mapersatz auf World8
oder Dimension14 bisher Entityadmission und den gewöhnlichen Inboxdrain. Der
erste Contextentry prüfte nur die Zielkoordinaten in der residenten Map und
vergaß deren aktuellen Scope. Unabhängige unveränderte RED-Probe:
`.codex-tmp/sim032-review-checkout/server/crates/ferrumc-sim/tests/sim032_review_scope.rs`;
Log `.codex-tmp/sim032-review-scope-red.log`.

Die eigenen zwei stärkeren Regressionen wurden vor Guardänderung ebenfalls
RED (`/tmp/sim032-scope-red.log`). Sie sichern zusätzlich vorhandene Live-
Registrierungen/Handles, Indexsnapshot, pending PlayerJoin, niemals ausgeführte
Closure, Tickets und ausdrücklich wiederhergestellten korrekten Scope. Nach
der oben beschriebenen entry-first Scopeprüfung sind beide und die gesamte
Simcrate grün. Die unabhängige unveränderte Probe wird gegen diesen neuen
Freeze erneut geprüft; das Finden und Beheben ersetzt diese Fremdabnahme nicht.

Es wurde kein Paper-Liveserver gestartet, kein Protokoll-/Plugin-Differenziallauf
und kein GUI-/Survival-/Performancevergleich für diesen Producer durchgeführt.
