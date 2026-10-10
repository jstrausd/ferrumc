# SIM-03.3 / SRV-02.1 — Besessener ItemEntity-Zustand und nativer Codec

Status: **vollständiger Vertrag durch Root geprüft und zur Sim-Runtimeumsetzung
freigegeben; Implementierung läuft**. Freigegebener Designstand SHA256:
`0f48834af0195c42a52f89638905014e35d2d8297cbd4088352d06ec1e53a9f0`.
Dieser Brief selbst aktiviert noch keinen Gameplay-/Appconsumer.
Parent `/root` besitzt Board, Integration und die anschließende Fremdabnahme.
Agent `/root/feature_catalog` besitzt nun ausschließlich `ferrumc-sim` und
dieses Dokument. Keine Änderung an Items, Registry, Core, Storage, App,
Cargo.lock oder Git. Bestehende Producer bleiben unverändert; Root besitzt
Docs-/Runtimeintegration und unabhängige Abnahme.

## Ziel und ausdrücklich bestätigte Grundsätze

Gedroppte Items erhalten einen konkreten, move-owned Zustand **direkt im
besitzenden Slot des existierenden `EntityStore`**. Kein zweiter Owner, Sidecar,
allgemeines Components-/Genericsframework oder konkurrierendes Playerinventar.
SIM-03.1/03.2 liefern UUID/Handle/Store, REG-ENTITY001 den registrierten Typ,
ITEM-PERSIST den verlustfreien FCIS-Stackcodec und STORAGE-ENTITY001.1 die
UUID-Key-/Recordwerte. Dieser Consumer führt kein Storage-I/O aus.

Root hat bestätigt: Base-only `minecraft:item`-Registrierungen der bisherigen
API bleiben ausdrücklich **Placeholder**. Echte Itemconsumer verweigern sie
als `MissingItemState`; kein EMPTY-/Defaultfallback. `EntityStore::new` behält
Itembudget null; ein neuer Konstruktor verlangt ein explizites Budget. Legacy
Remove verweigert Payloadslots atomar; owned Remove gibt Basis und gesamten
Stack zurück. Alle Itemfelder/Patches bleiben sealed, checked Encoding liefert
den gecachten Längenwert, Snapshot/Capture prüfen vor jedem Payloadclone.
Signed-zero-Motion- und Bobbits bleiben erhalten. Native Ownersnapshots sind
keine Nachbildung von Java `ItemEntity.copy()`.

Root hat zusätzlich die unten beschriebene Clone-Vorprüfung mit eng begrenztem
fail-stop für den infallible Legacyclone sowie den Tickcapabilityguard der
direkten Legacy-Transform-API bestätigt. Methodennamen, 144-Byte-Grammatik,
16-MiB-Hardcap und konkrete Counter-/Restoreregeln sind im oben genannten
Designfreeze ebenfalls vollständig freigegeben.

## Daten, Owner und Grenzen

`Slot` erhält eine private `item: Option<ItemEntityState>`. Nur ein lebender
`minecraft:item`-Slot darf `Some` tragen; freie/retired Slots tragen `None`.
Der Zustand enthält genau einen owned `ItemStack`, private geprüfte
`ItemEntityProperties` und eine private gecachte FCIS-Länge. Es wird kein
zweites dauerhaftes FCIS-Bytearray neben dem Stack gehalten.

`ItemEntityProperties` ist ein sealed Copywert: Motion `Vec3` (drei `f64`),
Boboffset `f32`, `ItemEntityCounters`, Pickup-Owner/Target
`Option<EntityUuid>` und getrennt Thrower `Option<EntityUuid>`.
`ItemEntityCounters` enthält Age, PickupDelay und Health jeweils als **`i32`**;
sämtliche signed Werte sind gültig. Konstruktion der Properties prüft nur
Endlichkeit von Motion/Bob. Kein Zufalls-/UUID-/Wire-ID-Generator, implizites
Default, Motionclamp, Healthclamp oder eingeschränkter Zählerbereich.
Live State/Stack/Properties haben ausschließlich immutable Getter; keine
`&mut ItemStack`, Patchmutation oder ungeprüfte Deserialisierung.

| Grenze | Konkreter Vertrag |
| --- | --- |
| Slotkapazität | Bestehende explizite `NonZeroUsize`, höchstens 65.536; retired Slots zählen weiter. |
| `MAX_NATIVE_ITEM_ENTITY_BYTES` | **262.144 Byte**, gleich Storage `MAX_ENTITY_DATA_LEN`, einschließlich sämtlicher FCIE-Header- und FCIS-Bytes. |
| `NATIVE_ITEM_ENTITY_HEADER_BYTES` | **144 Byte**, daher FCIS höchstens **262.000 Byte**. Kein Abschneiden von Komponenten, Count oder Metadaten. |
| `MAX_ITEM_ENTITY_BYTES_PER_SHARD` | Vorschlag **16.777.216 Byte**. Explizites `NonZeroUsize`-Budget darf kleiner sein; größer wird typed verweigert. Auch ein Budget unter der kleinsten Itembodygröße ist gültig und verweigert jede echte Aufnahme. |
| Aggregate Accounting | `item_bytes_used = Summe(144 + checked FCISlen)` aller lebenden Payloadslots. Placeholder verbrauchen null. Spawn/Restore addieren, Replace tauscht die exakte Differenz, owned Remove subtrahiert, Move/Counter/Props ändern die Summe nicht. Keine saturierende Reparatur. |
| Scratch | Öffentlicher FCIS-Encoder allokiert nach seiner eigenen semantischen Vorprüfung höchstens `MAX_PERSISTED_STACK_BYTES = 263.168` Byte. Erst danach wird der strengere native Bodycap geprüft. Je Operation höchstens ein solcher FCIS-Scratch gleichzeitig; native Output separat höchstens 262.144 Byte. |

Die 16-MiB-Grenze begrenzt serialisierte Itempayloads und erlaubt beispielsweise
64 maximale Bodies; sie ist **kein RSS-/Allocatorbudget**. Slots, Maps,
Component-/NBT-Objektstruktur, temporäre Encodingbuffers und ausdrücklich
angeforderte Snapshots verbrauchen zusätzlichen, separat strukturell begrenzten
Speicher. Ein Clone darf nicht als kostenlose Kopie gelten. OOM in den
bestehenden Standardcontainern bzw. FCIS-Encoder ist fail-stop, keine
recoverable Refusal; recoverable Slot-/Bodyreservation erfolgt vor Commit.

## Konkrete API zur Freigabe

Alle neuen öffentlichen Typen haben private Felder und Rustdoc. Keine
öffentliche `Clone`-Implementierung für rohe Spawn-/Restoreproposals oder den
move-owned `ItemEntityState`; interne geprüfte Kopien gehören dem Store.

| Typ / Methode | Vertrag |
| --- | --- |
| `ItemEntityCounters::new(age: i32, pickup_delay: i32, health: i32)` | Infallible Copywert, Getter erhalten alle 32 Bit; keine Sentinelnormalisierung. |
| `ItemEntityProperties::try_new(motion: Vec3, bob_offset: f32, counters: ItemEntityCounters, owner_target: Option<EntityUuid>, thrower: Option<EntityUuid>)` | `Result<Self, ItemEntityError>`; finite prüfen, ansonsten alle Floatbits und UUIDs unverändert übernehmen. |
| `ItemEntitySpawn::new(base: EntitySpawn, stack: ItemStack, properties: ItemEntityProperties)` | Infallibles owned Proposal, noch keine Aufnahme. `into_parts` gibt alle drei Eingaben zurück. Auch ein ungültiger Stack bleibt bis zur checked Admission owned. |
| `EntityStore::with_item_byte_budget(owner: ShardId, capacity: NonZeroUsize, item_bytes: NonZeroUsize)` | `Result<Self, ItemEntityError>`; leer/lazy, explizite Grenzen. Der bisherige Konstruktor erzeugt denselben Store mit Itembudget null. Keine spätere Budgetvergrößerung/-ersetzung. |
| `SimShard::initialize_entities_with_item_budget(capacity: NonZeroUsize, item_bytes: NonZeroUsize)` | Einmalige Konfiguration mit den bisherigen Owner-/AlreadyInitialized-Regeln. Eine Refusal initialisiert/ersetzt nichts. Alter Initializer bleibt sourcekompatibel und Itembudget-null. |
| `EntityStore::{item_byte_budget,item_bytes_used}` | Read-only `usize`, keine Reservierung oder Admissionpermission. |
| `EntityStore::item_state(&EntityHandle)` | `Result<&ItemEntityState, ItemEntityError>`; Handle → Typ → vorhandener Payload. Nicht-Item liefert `WrongEntityType`, Itemplaceholder `MissingItemState`. |
| `EntityTickContext::spawn_item(ItemEntitySpawn)` | `Result<EntityHandle, ItemEntitySpawnRefusal>`; Refusal enthält Reason und **komplettes ursprüngliches Proposal**. Scope/Typ/Region/Duplikate/Slotcap → Residency → Stack/Body/Budget → Reservation; erst danach ein gemeinsamer Commit. |
| `EntityTickContext::replace_item_stack(&EntityHandle, ItemStack)` | `Result<ItemStack, ItemStackRefusal>`; Erfolg gibt den bisherigen Stack move-owned zurück. Refusal gibt den neuen Eingabestack unverändert zurück und bewahrt den bisherigen Stack. Handle/Typ/Payload/Residency, current Stack+cache und neuer Stack+Body sowie Aggregatebudget vollständig vor Commit prüfen. |
| `EntityTickContext::set_item_properties(&EntityHandle, ItemEntityProperties)` | `Result<ItemEntityProperties, ItemEntityError>`; gibt alte Props zurück. Handle/Typ/Payload/Residency prüfen; geprüfte finite Copyprops ersetzen atomar, keine Stackmutation oder Eventwirkung. |
| `EntityTickContext::remove_owned(&EntityHandle)` | `Result<RemovedEntity, ItemEntityError>`; `RemovedEntity` gibt `EntityData` und `Option<ItemEntityState>` mit `into_parts` move-owned zurück. Auch Base-only/Nicht-Item zulässig. Cleanup funktioniert nach Ticketunload; kein Storageack oder Disposeevent. |
| `EntityStore::snapshot_item(&EntityHandle)` | `Result<ItemEntitySnapshot, ItemEntityError>`; Basis+Props+Stack vor dem ersten Stackclone vollständig semantisch und als nativer Body prüfen. Snapshot hat immutable Getter und owned `into_parts`, keine Admissionpermission. Residency nicht nötig. |
| `EntityStore::try_snapshot()` | `Result<EntityStore, ItemEntityError>`; gesamten Store und Aggregateaccounting vor dem ersten Itempayloadclone prüfen, dann bounded Snapshot mit frischer Incarnation. Keine zweite aktive globale Ownershippermission. |
| `EntityStore::capture_item(&EntityHandle)` | `Result<(PersistentEntityKey, PersistentEntityRecord), ItemEntityError>`; aktuelle Basis+Props+Stack erneut vollständig prüfen, dann nativen Body encodieren. Kein ItemStackclone, IO, Dirtyclear oder Saveack; Capture auch nach Ticketunload erlaubt. |
| `decode_item_entity(key: PersistentEntityKey, record: &PersistentEntityRecord)` | `Result<RestoredItemEntity, ItemEntityError>`; checked Native-/FCISdecode unten. Nur borrowed Rohbytes, keine Änderung am Input/Owner. Ergebnis hat **keine Wire-ID**, keine öffentliche Rawkonstruktion. |
| `EntityTickContext::restore_item(restored: RestoredItemEntity, new_wire_id: EntityId)` | `Result<EntityHandle, ItemEntityRestoreRefusal>`; bewahrt UUID und gesamten Zustand, prüft wie Spawn Scope/Region/Resident/Duplikate/Stack/Body/Budget, vergebenen Wirewert verwendet unverändert. Refusal gibt das vollständige decoded Proposal und den neuen Wirewert zurück. |

`ItemEntitySpawnRefusal`, `ItemStackRefusal`, `ItemEntityRestoreRefusal` sind
konkrete owned Structs mit `reason()` und `into_parts()`, keine generischen
Transferframeworktypen. `RestoredItemEntity::into_parts()` gibt
`(PersistentEntityKey, EntityTypeId, EntityTransform, ItemStack,
ItemEntityProperties)` zurück. ItemState/Snapshot haben entsprechende
vollständige owned Zerlegung. Kein Errorweg klont, verwirft oder repariert die
Eingabe; JSON/Creative-Normalisierung kommt nicht zum Einsatz.

`ItemEntityError` klassifiziert die konkreten Refusals: `Store(EntityStoreError)`,
`Stack(ItemPersistenceError)` und `StorageRecord(StorageError)` behalten den
Producerreason; `WrongEntityType { actual: EntityTypeId }`, `MissingItemState`,
`EmptyStack`, `NonFiniteProperty { component: ItemPropertyComponent }`,
`InvalidItemByteBudget { requested: usize, max: usize }`,
`ItemByteBudgetExceeded { required: usize, limit: usize }`,
`NativeBodyTooLarge { size: usize, max: usize }`,
`CachedItemLengthMismatch { cached: usize, actual: usize }` und
`AggregateItemBytesMismatch { cached: usize, actual: usize }` unterscheiden
Admission und interne Invariantprüfung. Kein freier Stringreason.

Parserrefusals im selben enum: `UnsupportedPayloadSchema { found: u32 }`,
`InvalidMagic`, `UnsupportedNativeVersion { found: u16 }`,
`TruncatedNative { offset: usize, needed: usize }`,
`InvalidNativeEntityType { raw: i32 }`,
`NativeKeyMismatch { field: ItemNativeKeyField }` (World/Dimension/Uuid),
`NativeChunkMismatch { expected: ChunkPos, actual: ChunkPos }`,
`NonCanonicalTransform { component: EntityTransformComponent }`,
`InvalidUuidPresence { field: ItemReferenceField, value: u8 }`,
`NonZeroAbsentUuid { field: ItemReferenceField }`,
`InvalidFcisLength { declared: u32, remaining: usize }` und
`NonCanonicalFcis`. Die exakte Endprüfung verweigert Trailingbytes als
`InvalidFcisLength`; FCIS-interne Malformation bleibt `Stack(...)`.
`ItemReferenceField` unterscheidet OwnerTarget/Thrower; Propertycomponent
MotionX/MotionY/MotionZ/Boboffset. Recoverable Reservation kann den bestehenden
`Store(AllocationFailed)` verwenden. Kein Result übernimmt einen teilweise
geänderten Index oder kompensiert Itemverlust durch synthetische Ersatzitems.

### Legacy, Tickcapability, Removal und Clone

- Bestehendes `EntityStore::spawn` und Contextspawn bleiben Base-only;
  `minecraft:item` dort ist ein Placeholder. Echte Spawn-/Restore-/Itemsetter
  sind ausschließlich über `EntityTickContext` öffentlich. Private Storehelper
  sind keine zweite mutable Capability.
- Legacy `EntityStore::remove` und Contextremove behalten ihre Signatur
  `Result<EntityData, EntityStoreError>`, liefern bei Payloadslots jedoch
  `PayloadRequiresOwnedRemove` **vor** Detach, Index-, Budget- oder
  Generationänderung. Keine stille Stackfreigabe.
- Öffentliches Legacy `EntityStore::update_transform` muss echte Payloadslots
  als `ItemMutationRequiresTickContext` verweigern; sonst wäre Tick-only
  umgehbar. Bestehendes `EntityTickContext::update_transform` verwendet einen
  privaten checked Helper und bewegt Basis plus Payload im selben Slot, mit
  den bisherigen Target-resident-/Ownerregeln. Base-only API bleibt kompatibel.
- Removeowned verschiebt Basis+State aus demselben Slot, bereinigt exakt beide
  Identitätsindizes und Chunklinks, entlastet das Budget einmalig und
  invalidiert/retired die bisherige Generation. Reuse startet payloadfrei;
  u64MAX wird nicht wrapped. Foreign/Stale/Incarnation werden vor Payloadzugriff
  geprüft. Storeclone führt zu neuer Incarnation; Sourcehandles bleiben fremd.
- `EntityStore::Clone` und dadurch `SimShard::Clone` bleiben infallible und
  sourcekompatibel. Sie benutzen dieselbe vollständige Vorprüfung wie
  `try_snapshot`, **bevor** irgendein Stack kopiert wird. Ein durch private
  Korruption unmöglich gewordener Invariantbruch ist dort explizit fail-stop
  (`match` + `panic!`, kein `unwrap`, EMPTY oder lossy fallback); `try_snapshot`
  liefert stattdessen den typed Reason. Alle normalen öffentlichen Setter
  erhalten das sealed Invariant. Ein Kopierergebnis darf ohne externen
  Ownervertrag nicht zugleich mit der Quelle aktiviert werden.

Scalar-/Countersetter kopieren keine Stacks und ändern ihre FCISrepräsentation
nicht; validated Properties plus der sealed admitted State erhalten das
Body-/Budgetinvariant ohne vollständiges Stackencoding pro Timerstep.
Payloadaufnahme, Stackreplacement, Snapshot und Capture prüfen dagegen mit
dem **öffentlichen** FCIS-Encoder, nicht nur `ItemStack::is_valid`. Snapshot des
gesamten Stores prüft auch, dass die Summe der erneut gemessenen Bodies dem
gecachten Aggregatewert entspricht. Kein Itemsproducer wird hierfür geöffnet.

## Native Body FCIE, Version 1

Storage `EntityRecord::schema_version` ist **SchemaVersion(1)**. Die eigene
FCIEversion ist separat `u16(1)`. FCUE-/Storageformat, FCISversion,
Generatoridentity und Storeepoch sind andere Domains. Der Codec ist konkret
für den gepinnten MC1.21.8-`minecraft:item`-Typ; Registryupdates brauchen eine
ausdrückliche Migration. Alle Zahlen sind Big Endian, Floatfelder werden über
ihre IEEE754-Bits geschrieben, UUIDs als sämtliche 16 Bytes.

| Byteoffset | Feld / Länge |
| --- | --- |
| 0 | Magic `FCIE`, 4 Byte |
| 4 | FCIEversion `u16`, 2 Byte |
| 6 | Registry Entitytype `i32`, 4 Byte; lookup muss exakt `minecraft:item` ergeben, kein Hardcoding/Fallback auf numerischen Default |
| 10 / 14 | WorldId `u32` / DimensionId `u32`, je 4 Byte |
| 18 | EntityUuid, 16 Byte |
| 34 | Position x/y/z, 3 × `f64`, 24 Byte |
| 58 / 62 | Yaw / Pitch, je `f32`, 8 Byte zusammen |
| 66 | Motion x/y/z, 3 × `f64`, 24 Byte |
| 90 | Boboffset `f32`, 4 Byte |
| 94 / 98 / 102 | Age / PickupDelay / Health, je `i32`, 12 Byte zusammen |
| 106 / 107 | OwnerTarget-Presence `u8` / UUID, 1 + 16 Byte |
| 123 / 124 | Thrower-Presence `u8` / UUID, 1 + 16 Byte |
| 140 | FCIS-Länge `u32`, 4 Byte |
| 144 | Exakt die vollständig encodierten FCIS-Bytes; keine weiteren Bytes |

Presence ist 0 oder 1. Bei 0 müssen die zugehörigen 16 Reservedbytes null sein;
bei 1 sind **alle** UUIDs einschließlich Nil zulässig. Die beiden Referenzen
werden niemals zusammengelegt. Body enthält keine Wire-ID, Slot-/Handle- oder
Incarnationbits, Generation, Dirtyfact, Despawnrate oder letztes Tickdatum.

Decode prüft in dieser Reihenfolge, **bevor FCIS einen Stack allokiert**:

1. Outer schema, Bodycap, Mindestheader, Magic und FCIEversion.
2. Registrierten exakten Itemtyp; World/Dimension/UUID stimmen bytegenau mit
   dem äußeren `PersistentEntityKey` überein.
3. Position ist finite und innerhalb bestehender Simcaps; daraus gefloorter
   Chunk stimmt mit `PersistentEntityRecord::chunk()` überein. Die Rotation
   muss bereits canonical sein: checked `EntityTransform::try_new` darf kein
   Yaw-/Pitchbit verändern. Malformed/future Records werden verweigert, nicht
   beim Laden normalisiert.
4. Motion und Bob finite, Presence/Reservedbytes korrekt; signed Zähler
   ungeändert lesen. FCISlen ≤262.000 und exakt Bodylen−144; truncated,
   überlaufende oder trailing Längen verweigern.
5. `decode_persisted_stack` übernimmt die vollständige FCISsemantik. Leer wird
   als `EmptyStack` verweigert. Anschließendes checked Encoding misst den
   Zustand erneut; Länge und Bytes müssen exakt mit dem FCISinput stimmen,
   andernfalls typed noncanonical Refusal. Erst danach ein sealed Restorewert.

Capture prüft aktuelle Basis/State vor Outputpublish, benutzt exakt die
gemessenen FCISbytes und erzeugt Key+Chunk+Body aus **einer** Entityaufnahme.
Restore setzt eine explizite neue, vom Caller allokierte Wire-ID ein;
Chunk/Scope werden nie geraten. Storagegeneration, UUID-Storewrite,
Durability-/Deleteack, Retry und Reconciliation bleiben beim späteren
Storage-/Appconsumer.

## Reine Countersteps und begrenzte Despawnentscheidung

`step_active_item_counters(counters: ItemEntityCounters, despawn_rate: i32)`
und `step_inactive_item_counters(...)` liefern einen Copywert
`ItemCounterStep` mit neuen Countern und
`ItemDespawnDecision::{Keep, Due}`. Die endliche Projektion beider Pfade ist
bei vorhandenem **nichtleerem** Stack identisch:

1. `pickupDelay > 0 && pickupDelay != 32767`: minus eins, ansonsten unverändert.
2. `age != -32768`: `age.wrapping_add(1)` wie Java `int`, ansonsten unverändert.
3. `new_age >= despawn_rate`: `Due`, sonst `Keep`. Kein zusätzlicher
   Infinite-Lifetime-Guard, kein Positivconstraint für die übergebene Rate.
4. Health, Motion, Bob, Referenzen und Stack sind unverändert.

`EntityTickContext::step_item_counters(&EntityHandle, ItemCounterMode::{Active,
Inactive}, despawn_rate: i32) -> Result<ItemDespawnDecision, ItemEntityError>`
wendet genau eine solche Projektion auf den vorhandenen residenten Itemslot
an. Foreign/Stale/WrongType/MissingItemState/Nonresident verweigern atomar.
Der Context hat keine globale Tick-ID; zweimaliger Aufruf bedeutet zwei
Countersteps, keine behauptete Exactly-once-Automatik.

Dies ist **kein vollständiger Active-Entitytick**: Im Original liegt die
Delayänderung vor Physics/Merge, Ageänderung nach diesen Phasen; ein künftiger
vollständiger Consumer muss diese Phasen korrekt trennen und kann nicht beide
Counteränderungen über eine dazwischenliegende Mergeoperation hinwegziehen.
Der originale Activepfad entfernt leere Stacks vor den Countern; EARinactive
enthält diesen Emptyguard nicht. Hier werden leere States schon bei Admission
verweigert und Placeholder typed behandelt, daher entsteht keine Ersatzlogik.

`Due` entfernt nichts und dispatcht kein Event. Späterer Caller muss
ItemDespawnEvent aufrufen, dessen veränderbare Daten und weiterhin gültigen
Handle erneut prüfen und bei Cancellation ausdrücklich
`EntityTickContext::reset_item_despawn_age(&EntityHandle)` verwenden: nur Age
wird auf null gesetzt, kein zweiter Delaystep. Bei erlaubtem Despawn ruft er
separat Removeowned auf und verantwortet den Destroy-/Persistpfad. Das
Resetmethod prüft ebenso Handle/Typ/Payload/Residency. Keine Eventtoken,
Reentrancy-, Cancellation-, Remove- oder Java-Brücke wird jetzt behauptet.

Grenzfälle der Referenz: Delay 32768 fällt auf 32767 und friert dann ein;
Age −32769 fällt auf −32768 und friert dann ein; i32MAX wird i32MIN.
SentinelAge kann bei einer Rate ≤−32768 trotzdem `Due` liefern. Null/negative
Health löst keinen Timerdeath aus. Aktivierung-inaktiv ist nicht unloaded
oder offline: nach Ticketunload gibt es keinen Step/Wallclockcatchup, lediglich
Capture/Cleanup bleibt möglich. Die spätere Activationrange und tatsächliche
Tickstatusentscheidung liefert der Caller, nicht Chunkresidency allein.

## Geprüfte Originalquellen, Verhalten und Abweichungen

Pin: Minecraft Java1.21.8 / Protokoll772, PaperBuild60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`, siehe
[PAPER_REFERENCE.md](../PAPER_REFERENCE.md). Referenzpfade unten sind relativ
zu `.codex-tmp/paper-1.21.8-reference/paper-server/`.

Vollständig gelesen: Base/applied `ItemEntity.java`, dessen vollständiger
Sourcepatch und EAR2-Itemhunk; applied `EntityReference.java`. Vollständige
relevante Base/applied Entity-save/saveAsPassenger/saveWithoutId/load-Methoden
mit Sourcepatchhunks, UUID/ID-/Konstruktorpfade, Motionsetter sowie
`copyPosition` → `snapTo`; EAR2 ServerLevel-Branches und vollständige applied
tickNonPassenger/tickPassenger sowie ActivationRange.checkIfActive.
Konfigurationscallees: vollständige AltItemDespawnRate-Klasse und Spigot-
itemDespawnRate-Getter; vollständiger callItemDespawnEvent und MCUtil-
sanitizeNanInf. Kein Claim vollständiger Activation-/Physik-/Eventabnahme.

| Quelle / Trigger | Geprüftes Verhalten | FerrumC-Vertrag / offene Grenze |
| --- | --- | --- |
| ItemEntity Felder/Konstruktor/setItem | Age/Delay/Health `int`, Initialwerte 0/0/5. Bob wird zufällig gewählt; Itemsetter bestimmt Paper-Raten aus Konfiguration. | Caller liefert sämtliche initialen Props und die Rate explizit; kein RNG/Configconsumer. i32 bleiben vollständig erhalten. |
| ItemEntity.tick / EARinactive / ServerLevel | Beide Counterpfade mit Sentinels und signed Vergleich; Active: Physics/Merge dazwischen. EAR-inactive ist ein tatsächlicher Entitytick, nicht Unloadpause. | Reine begrenzte Counterprojektion; kein Ticking-/Activation-/Merge-/Physikabschluss. Explizite Callerordnung bleibt offen. |
| CraftEventFactory.callItemDespawnEvent / ItemEntity.tick | Event vor Discard; Cancellation setzt Age auf 0, Return. Healthdeath ist separater damage/onDestroyed-Pfad. | Nur `Due` + separater Reset/Remove. Kein Eventdispatch, Schadenssystem oder Destroyparität. |
| ItemEntity Save/Load | Age/Delay/Health werden zu signed `short` gekürzt; Owner und Thrower getrennt. Bob wird nicht gespeichert; fehlender Itemstack kann EMPTY werden und wird entfernt. | Native Codec bewahrt i32/Bob und vollständige FCISrepräsentation. Corruption/Empty wird typed verweigert. **Bewusste native Persistenzpolitik**, kein Vanilla-NBTcodec/Import. |
| EntityReference / getOwner vs playerTouch | Thrower löst den Traceable-/Damageowner auf; Target beschränkt Pickup und ist eigenständig. | Zwei UUIDfelder ohne Rawentitypointer; Resolver/Pickup/Eventconsumer fehlen. |
| Entity.setDeltaMovement / save/load / MCUtil | Setter nimmt rohe Vec3; Paper Save/Load sanitizen NaN/Inf; Load setzt einzelne Motionkomponenten mit Betrag >10 auf 0. Positionload clamp x/z ±30.000.512, y ±20M. | Native Aufnahme verweigert nonfinite, erhält **alle finiten** Motionbits einschließlich >10 und −0; Position hat bestehende strengere ±30M-Rejectioncaps. Keine stille Sanitisierung/Clamp. Spätere Vanilla-NBTmigration muss ihre eigene Policy haben. |
| ItemEntity.copy / Entity.copyPosition → snapTo | Java Copy konstruiert neue ID/UUID, kopiert Stack/Position/Rotation/Age/Bob, setzt andere Felder durch Konstruktor/default. Applied snapTo setzt Motion ohne preserveMotion auf null. | Native Ownersnapshot erhält sämtlichen modellierten Zustand samt UUID und frischer Storeincarnation; keine Java-Copy-API oder identische Sondersemantik. |
| FCIS / nativeBody / Storebudget | FCIS hat größeren eigenen Stackcap; Paper hat keinen identischen 16-MiB-/8×8-Vertrag. | Schwerer gültiger Stack wird unverändert typed verweigert. Diese Ressourcenlimits sind keine Gameplay-/RAMbenchmarkparität. |

Referenzdateihashes (SHA256; Ganzdateihash belegt Identität, nicht zusätzliche
Lektüre oder bestandene Laufzeitabnahme):

| Referenzdatei | SHA256 |
| --- | --- |
| `.gradle/caches/paperweight/mache/base/sources/net/minecraft/world/entity/item/ItemEntity.java` | `64e95726b7f323d634caec4c2e1944c5e385f38d6579052c4fbb0213f86a69d4` |
| `src/minecraft/java/net/minecraft/world/entity/item/ItemEntity.java` | `0e01331a2b7c21ddc9753b3a3b44350cdce0af46b62694d91f57dd037ef13bad` |
| `patches/sources/net/minecraft/world/entity/item/ItemEntity.java.patch` | `877b103800edfb06b277bbfe11803c5f26b27ff12cea747ca04be1969d998e05` |
| `.gradle/caches/paperweight/mache/base/sources/net/minecraft/world/entity/Entity.java` | `c16ef162b80ff320458297852c26bed3f70185bb488b8406f430f04b64719cd0` |
| `src/minecraft/java/net/minecraft/world/entity/Entity.java` | `eef66010adc42ecfe5da442c9c747f0f7a93ec76a19a7d2911e9dc13952b8b37` |
| `patches/sources/net/minecraft/world/entity/Entity.java.patch` | `93008a3fe5c1cbb5d7980b5660c026a484f3ebd11867e09952c679ad9292c295` |
| `src/minecraft/java/net/minecraft/world/entity/EntityReference.java` | `a97932020a27e727e5598b50716a6791468df227f5f6c5992b6207fb2f344a2d` |
| `patches/features/0003-Entity-Activation-Range-2.0.patch` | `64845ef65af69996e21d85eac1d5f1925e4dc94e41bde5348e0198b5e2237d0b` |
| `src/minecraft/java/net/minecraft/server/level/ServerLevel.java` | `47394ff3c2bfd64882f04f3e318a5557ed0e6a4d1fe68df760db8c1afcbeb5fc` |
| `src/minecraft/java/io/papermc/paper/entity/activation/ActivationRange.java` | `1037474eba483d5e8942f907f45780bc2565d243049644c7b902ca0b0adcbbcc` |
| `src/main/java/org/bukkit/craftbukkit/event/CraftEventFactory.java` | `baaf7cd5b53cd7074725927d30ae815a863fd55965a4d684130bf31e3cd49959` |
| `src/main/java/io/papermc/paper/configuration/WorldConfiguration.java` | `3b78257059b28b95e47edd6d6f4c854058180bf09cdb998eeba15ce6446ada8f` |
| `src/main/java/org/spigotmc/SpigotWorldConfig.java` | `e77256b969a12664af338470d5babaaabc84fd6f227181ef0504f8650ce1baeb` |
| `src/main/java/io/papermc/paper/util/MCUtil.java` | `9907608da7d5a2fe9d20a3a3fc2a9afa44e46574429325ea6f18d73c51619c31` |

## Geplante Abnahme der ersten Runtimeetappe

- Öffentliche Tests für echte Itemaufnahme, Placeholder-WrongType-/Missing-
  Refusals, alle Admissionfehlgründe inklusive slotfull/retired/resident/
  Scope/dupliziert, komplette owned Rückgabe und unveränderte Ausgangsdaten.
- Aggregatebudget exact/plusone, schwerer **gültiger** FCISstack über dem
  nativen Bodycap, Replacewachstum/-schrumpfung/-refusal, Cleanup nach Unload,
  Slotreuse und alle UUID-/Wire-/Chunkindizes. Komponenten nicht normalisieren;
  typed/opaque/removals/patchorder/NBTduplicatekeys/Floatbits prüfen.
- Private Grenzprobes für generationMAX/Retirement, Cachedlen-/Aggregatemismatch
  vor Clone/Capture und ohne Publication. Keine Stackclones vor negativer
  Semantik-/Bodyprüfung. Storeclone/try_snapshot/SimShardclone weisen
  Quellhandles ab und bewahren sämtliche Bits/Payload-/Budgetwerte.
- Parsermalformation an jedem Header-/FCIScutpoint, Bodycap exact/plusone,
  falscher outerSchema/Magic/Version/type/scope/uuid/chunk, uncanonical
  Rotation, NaN/Inf je Motion-/Bobfeld, Presence/Reservedbytes, FCISlen/
  truncated/trailing/semantic invalid/empty. Dekodierte Bits separat mit
  originalen Bytes vergleichen, kein reines encoder↔decoder-Selbstorakel.
- Nativecapture → UUID-Key-/Chunkrecord → Decode → Restore mit **neuer**
  Wire-ID, identischer UUID/Scope/Position/Stack/Props; Refusal bewahrt
  Restoreproposal. Kein DBwrite/Restartclaim ohne späteren Consumer.
- Unabhängige handgeprüfte Originalfälle für Age/Delay: 0,1,−1,
  32766/32767/32768, −32769/−32768/−32767, i32MIN/MAX sowie positive,
  null und negative Raten; Active-/Inactiveprojection, Due ohne Remove,
  Cancellationreset ohne Delaystep, Healthunverändert, NonresidentkeinStep.
- Compilefail für rohe mutable State-/Patch-/Storeitemaufnahme und
  entweichende Tickcapability. Legacy Payloadremove/-transform müssen
  typed verweigern, Contexttransform muss denselben Slot bewahren.
- Bestehende vollständige Simsuite einschließlich WG-003/SIM-03.2,
  gezielte Debug- und Releaseprüfung der wrapping-/Overflowgrenzen,
  strict `cargo clippy -p ferrumc-sim --all-targets -- -D warnings`, fmt.
  Root organisiert anschließend einen unabhängigen Review am exakten
  SHA256-Freeze. Keine Gitoperationen durch diesen Agent.

Bislang durchgeführt: Quellenlektüre und Design, **keine neue Runtime-,
Differenzial-, Client-, Plugin-, Survival- oder Performanceabnahme**. Physics,
Kollision/Gravity/Buoyancy, Merge/Pickup, Dropinventartransfer, Activationrange,
canMobPickup/frictionState und allgemeine Entityflags (onGround/noGravity/fire,
Passengers/Tags/PDC usw.), Tracking, Javaevents, Dirty-/Save-/Deleteconsumer,
Un-/Reload und Appbetrieb bleiben ausdrücklich nachfolgende Tickets. Diese
noch unmodellierten Felder haben keinen stillen Ersatz oder Codecfallback;
der Body bildet nur den oben vollständig definierten nativen Zustand ab.
