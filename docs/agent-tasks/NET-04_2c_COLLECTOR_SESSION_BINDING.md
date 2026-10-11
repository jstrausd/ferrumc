# NET-04.2c — tatsächliche Collector-/Sessionbindung

Status: an `select_world_ticket` vergeben, **ausschließlich konkreter readonly
Vertragsentwurf**. Schreibscope ist dieser Brief und `.codex-tmp/net04_2c/**`.
Auch Sessionruntime, App, Sim, Storage, Items, Net, Proto, Dependencies,
Rootboards und Gitmutationen bleiben außerhalb dieser Vergabe. Produktion
benötigt eine ausdrückliche separate Rootfreigabe. SIM-03.4 ist inzwischen
unabhängig CLEAN und auf lokalem main als `b9f15db8` integriert: tatsächlicher
Sourcefreeze `dc309f6974442e414495a8ce1344a16aa34f9dbdd7857b615da837aa6cb0dca8`,
Root-Kombinationsarchiv `242c8f07f3ea724538a099a3339d4c65b276e863`, Debug/Release
je347 sowie Strictchecks. Dessen wirkliche Group-/Ticket-/FIFO-/Matcher-APIs
sind erneut gelesen und gepinnt. Der weiter unveränderte genehmigte
Entwurf mit SHA-256 `efbeb2beb26198f6b6d5ae3904388fa91cfe8f91ced42f44ece012549df986fa`
und dessen Rootreview einschließlich des engen `matches_owned_player`-Vergleichs
bleiben ergänzende Vertragsgrundlage.

Die angewandte **Paper**-Methode `LivingEntity.take` benutzt Collectortracking
einschließlich Collector selbst als Audience. Die Vanilla-Basismethode benutzt
dagegen das aufgenommene Entity als Broadcastquelle; diese reale Abweichung
bleibt ausdrücklich erhalten. Das Itemviewer-Set und PlayerId allein reichen
dafür nicht. Session besitzt heute den tatsächlichen Remoteplayer-Wirewert;
Sim liefert einen versiegelten `ItemCollectorBinding` mit echter Owner-Weak,
Admission und voller Generationbindung. Dessen genehmigter readonly Vergleich
`matches_owned_player` kann eine vorhandene echte Actoradmission vergleichen,
beweist allein aber weder aktuelle Session noch Connected-/Scopezustand.

Root hat zusätzlich eine konkrete Grenze gelesen: `router.rs` vergibt
Remoteplayer-WireIDs ab2 und erhöht derzeit wrapping. `app/src/world.rs`
verwendet `PLAYER_ENTITY_ID=1` im tatsächlichen JoinGame für jeden Client.
Der Collector hat daher bei sich selbst momentan einen anderen Wirewert als
bei Observern. Eine globale Takecollector-ID darf nicht an alle Viewer
weitergereicht werden. Die App-/JoinGamebindung bleibt getrennte Folgearbeit;
ein Sessionconstructor darf callerlieferbare Zahlen nicht als Socketreceipt
oder tatsächlichen JoinGameproof ausgeben.

Der Entwurf liest alle wirklichen Join-/Rejoin-/Move-/Deliveredbaseline-/
Leave-/Endpointrotationpfade sowie Originaltracker-/Take-/Removalcode und
Source-/Featurepatches der gepinnten Minecraft1.21.8/PaperBuild60-Referenz.
Er legt APIs, tatsächliche Router-/Sessionincarnation, volle World-/Dimension-/
Generator-/Owner-/Admissionbindung, positive Count-/Scratch-/Arbeitscaps und
jede Fehlerdisposition vor. Keine öffentlich konstruierbare Fakebinding,
Mutationshandle aus einem Fact, Weakadresse als Identität oder neue
Socket-/DB-/SimShardcapability.

Audience verwendet die wirklich admitted Collector-Trackerbaselines und den
Collector selbst, mit viewerspezifischem tatsächlichem Wirewert. Farviewer,
gleiche PlayerId in fremdem Owner, Endpointreplacement, abgewickelter oder
noch pending Disconnect, Rejoin, Dimensions-/Scopewechsel und veraltete
Admission brauchen explizite Zustandsfolgen. Ein alter Collector darf nicht
auf die neue Session desselben Spielers umgebunden werden. Fehlende Authority
darf keinen erfundenen Take erzeugen; die ganze bestätigte Itemgruppe und deren
weitere Changed-/Removephasen bleiben erhalten. Ausschließlich der unten
definierte genuine terminale Retirementproof erlaubt die beweisbar leere
Takephase und danach normale vollständige Changed-/Removeadmission.

Wire-Allocatorüberlauf wird vor PlayerJoin/Sessioninstallation überprüft;
keine wiederverwendete kollidierende ID nach wrapping. Die gemeinsame
Namespaceabstimmung mit zukünftigen Item-WireIDs ist eine ausdrücklich zu
benennende Grenze. Neue Refusals behalten die wirklichen Mapping-/Leave-
Retryverträge und unverändert die bestehende Mandatory-Lane.

Meaningful Tests benutzen echte bounded Routerkanäle, tatsächlich erzeugte
Sim-Admissions-/Collectorbindings und Decoderframes: Collector/Observer mit
verschiedenen Self-/Remotewerten, echte Trackingaudience, stale/foreign/
Rejoin-/Scopefälle, MAX-/N/N+1-Grenzen und voller/geschlossener Control-/
Outboundlane mit retained Leave. Ein eigener Calleradapter ersetzt keine
Original- oder Authorityabnahme. WholeSession Debug/Release und strictchecks,
exakter Freeze und unabhängiger Rootreview folgen vor Integration.

Dieses Ticket liefert die geprüfte Sessionbindung für NET-04.2b. Es aktiviert
keine Itemphysik, automatische Aufnahme, produktiven Format6driver oder
Javaevents und schließt keine Survivaletappe ab.

## Gelesene tatsächliche Pfade und entscheidende Grenzen

`SessionEntry` enthält heute PlayerId, stabilen Shardregistrierungsstamm,
Remote-Wire-ID, Outboundsender, gecachte Pose und `delivered: BTreeMap<i32, Vec3>`.
Es fehlen tatsächliche Router-/Sessioninkarnation, native Admission und volle
Generationbindung. `is_player_connected` bedeutet ausschließlich Mapmitgliedschaft;
auch ein zurückgehaltener Leave bleibt dort sichtbar. Dies ist kein Liveproof.

Der wirkliche Join in `router.rs:635` prüft zunächst Scopeauflösung, Duplicate
und Joinerburst, trennt dann langsame Viewer, sendet PlayerJoin, installiert die
Session und sendet gegenseitige Add-/Spawnpakete. Der wrapping Allocator liegt
heute **nach** PlayerJoin. Seine Refusal muss künftig schon **vor dem ersten
Slowviewer-Disconnect** feststehen. `send_mandatory_spawn` setzt die Baseline erst
bei erfolgreichem Layer-A-Enqueue; droppable Move verändert sie nur bei Erfolg.

`broadcast_move` betrachtet aktuell nur Viewer in Reichweite der neuen Pose.
Es entfernt kein früheres Pairing bei Bewegung aus Reichweite und aktualisiert
bei der Bewegung eines Viewers nicht dessen Sicht auf alle anderen Subjects.
Diese alten Baselines dürfen nicht als vollständiges aktuelles Collectortracking
ausgegeben werden. Der neue native Pfad muss Ein- und Austritt in **beiden**
Richtungen abgleichen; bestehende Legacyaufrufe liefern keinen nativen Proof.

Disconnect sendet PlayerLeave vor Mapentfernung. Full/Closed/stale-Lineage
behält das Mapping und den deduplizierten pending Leave; Cascade und Retry
bleiben iterativ. Erfolgreicher Leave räumt nur tatsächlich zugestellte
Playerbaselines mit Mandatory Tabremove/Entityremove auf. Directoryreplacement
erhält registration_id/home, ändert aber die exakte Leasegeneration.
Unregister/Register erhält eine neue Lineage; alte Spieler dürfen ihr weder
folgen noch auf Worldfallback umgebunden werden.

Der aktuelle Appablauf ist UUID-Reservation → Legacyrecordload und lokales
Inventory → `SimCommand::Join` → Router-PlayerJoin → **anschließend** direkter
JoinGame=1 aus `JoinKit` in `send_join_kit`. Stage1 wird vor Chunks geflusht;
später verarbeitet die App je Outboundmessage genau einen Socketflush.
Teardown speichert zuerst, released bestätigte Chunktickets und wartet auf
Leaveack; ein Fehlschlag quarantined die UUID. Kein dieser Schritte produziert
heute einen nativen Connected-/Owner-/Admissionproof für Session.

Im native Ownershipmode ignoriert `SimShard::run_tick` sogar sämtliche alten
GameInputs (`shard.rs:819`). Ein gewöhnliches PlayerJoin/PlayerLeave darf deshalb
nicht als native Admission/Detachbestätigung gelten. Der App-/Nativecontrol-
Anschluss ist eine echte Produktionsvoraussetzung, kein Sessionadapterdetail.

## Originalreferenz: Audience, Tracking und Removal

Pin: Minecraft Java 1.21.8 / 772, Paper Build60, tatsächlicher Referenz-HEAD
`29c8822d90899c89d2689338e81a98f690bcba12`. Vollständige relevante Methoden
und Callees aus Basis und angewandter Referenz sowie relevante Source-/Feature-
Hunks wurden gezielt gelesen; Hash-/Lesescopes stehen im eigenen Evidenzrecord.

| Tatsächliche Quelle | Schluss für diesen Vertrag |
|---|---|
| Vanilla `LivingEntity.take:3025` | `broadcast(entity, packet)`: aufgenommenes Entity als Quelle; keine Behauptung gleicher Paperaudience |
| Applied `LivingEntity.take:3792`, Sourcepatch1605–1612 | `broadcastAndSend(this, packet)`: Collectortracker plus Collector selbst; Originalcount im Take |
| Applied `ServerChunkCache:723`, `ChunkMap:1067` | Fehlt der Collectortracker, wird **auch kein Selfpaket** gesendet |
| `TrackedEntity.broadcast:1247`, `broadcastAndSend:1261` | Collector-seenBy zuerst, danach Self bei ServerPlayer; keine Itemviewer-Schnittmenge |
| `updatePlayer:1281`, Moonrise tick1152/clear1212 | Aktuelles Pairing berücksichtigt XZ-Range, ggf. Y, broadcastToPlayer, tatsächlich getrackten Chunk, canSee und Trackevent; Wegfall unpaired |
| `removePlayer:1274`, `ChunkMap.removeEntity:984` | Connection erst aus Tracking entfernen, dann Entityremove; Playerconnection aus allen Trackern entfernen |
| `ServerEntity.addPairing:327`, `sendPairingData:334` | Spawn vor Metadaten/weiteren Daten; tatsächliche Pairingdaten und Removal getrennt |
| `ItemEntity.playerTouch:414` und synchroner Removalcallback | Tatsächlicher Transfer → optional Take → Empty-discard/Trackerremove; Partialrestmetadata folgt später |

Paper fügt seenBy **vor** einem cancellable PlayerTrackEntityEvent ein. seenBy
allein ist deshalb kein genereller Spawnreceipt. Dieser Sessionvertrag verwendet
wirklich admitted FerrumC-Baselines. Rangeform, Chunktracking und Java-Vanish-/
Trackevents des vollständigen Papertrackers sind eigene sichtbare
Integrationsgrenzen; weder der heutige Chunkquadrat-Test noch fehlende
Javaevents werden als Originalparität ausgegeben.

## Eine konkrete Bindungsform

Der Router erhält eine private `Arc<RouterIdentity>` und einen checked,
monotonen NonZero-u64-Sessionzähler. `PlayerSessionKey` enthält ausschließlich
Weak-Routeridentity, PlayerId und tatsächliche Sessioninkarnation. Felder,
Constructor und Deserializefactory sind privat. Eq prüft `Weak::ptr_eq` und
beide Scalars; keine Weakadresse wird zum Key, in Zahlen konvertiert oder
serialisiert. Ein Routerclone, falls später eingeführt, besitzt frische Identität
und keine aktiven Sessions oder Nativebindungen.

`PlayerSessionHandle::session_key()` liefert nur einen cheap readonly Key,
keinen neuen Sender, Socket, Actor oder DBhandle. Jeder native Inbound-, Move-,
Chunk-, Detach- und Outboundfolger führt diesen Key mit. Eine alte Connection
kann dadurch nach Rejoin weder neue Inputs noch Disconnects an die neue Session
derselben UUID adressieren. Alte UUID-only Legacy-APIs behalten ihre bisherige
Semantik und dürfen keinen nativen Collectorbinding-Eintrag erzeugen.

Ein privater `NativeCollectorSession` in der wirklichen Session enthält:

- die **wirklich erhaltene** `OwnedPlayerHandle`-Admission, niemals aus einem
  Fact neu erzeugt oder aus PlayerId rekonstruiert;
- die vollständige `GenerationBinding`, also World/Dimension und Minecraft-
  release, Generatorart, Revision, Seed und Dimensionart;
- die bei Bindung aktuelle echte Directorylease einschließlich fremder
  Directoryidentity, Coverage, registration_id, home und exakter Generation;
- den tatsächlichen Sessionkey, Remote-Wire-ID und die intern aus der wirklichen
  Self-JoinGame-Admisson gewonnene Self-Wire-ID;
- Lifecycle `SelfJoinPending`, `Active`, `EndpointSuspended`, `Leaving` und
  danach die getrennte genuine `Retired`-Recordform; `ever_active` ist privat.

Es gibt **keine** öffentliche `bind(player, wire, generation)`-, arbitrary-
parts-, `from_fact`- oder Setter-API. Die einmalige Installation geschieht nur
im unten beschriebenen echten nativen Join. Admission, Scope und Self-ID werden
in derselben Session niemals nachträglich auf einen neuen Spielerstand gesetzt.

Jede Collectorprüfung verlangt gemeinsam: exakten Sessionkey im eigenen Router,
Active-Lifecycle, offenen Outbound, keinen pending Disconnect, aktuelle Lease,
`collector.matches_owned_player(stored_handle)`, vollständige Gleichheit von
Collector-, Session- und tatsächlicher Liveproof-Generation sowie die
Liveproof-Gleichheit mit **demselben** stored Handle. Der enge Matchingvergleich
beweist Owner/Player/Admissiongleichheit; seine heutige Implementierung prüft
selbst keine Generation oder connected-State. Diese Checks bleiben getrennt.

## Enger fehlender Simproducer

Diese API ist **vorgeschlagene Folgearbeit**, keine vorhandene Funktion:

```rust
impl SimShard {
    pub fn connected_owned_player_view<'a>(
        &'a self,
        handle: &OwnedPlayerHandle,
    ) -> Result<ConnectedOwnedPlayerView<'a>, PlayerOwnershipError>;
}
pub struct ConnectedOwnedPlayerView<'a> { /* private borrowed actual state */ }
impl ConnectedOwnedPlayerView<'_> {
    pub fn player(&self) -> PlayerId;
    pub fn generation(&self) -> GenerationBinding;
    pub fn position(&self) -> Vec3;
    pub fn look(&self) -> [f32; 2];
    pub fn game_mode(&self) -> GameMode;
    pub fn hashed_seed(&self) -> i64;
    pub fn matches_owned_player(&self, handle: &OwnedPlayerHandle) -> bool;
}
```

Der Producer benutzt die vorhandenen wirklichen `ownership_state`- und
`actual_player`-Checks: active Owner, kein SnapshotOnly, echte Chunk-/Entityscope-
Generation, private Ownerweak und exakte Admission, vorhandener nativer Player
und dessen aktuelles `owned.connected`. Er prüft zusätzlich nicht stopped.
Während Drain darf eine noch verbundene echte Session vorhandene bestätigte
Gruppen abarbeiten; Drain allein entfernt keine Authority. Reserved/dirty
Inventoryzustand entwertet die Identität nicht und erzeugt keine Postimage.

View hält einen kurzen Borrow des tatsächlichen SimShardzustands, besitzt kein
Clone, public constructor, Weakgetter oder herausgebbares Mutationshandle.
Der Driver hält diesen Borrow während readonly Audienceplanung und unmittelbarer
Sessionannahme. Die Simulation kann in diesem Intervall nicht gleichzeitig
Rejoin, Detach oder Scopewechsel mutieren. Ein deskriptiver connected-Bool und
`owned_player_generation`/Slotslookup sind kein Ersatz. Native Poseupdates für
Tracking werden aus dieser tatsächlichen View oder einem gesondert echten
identitätsgebundenen Producer übernommen; UUID-only GameOutput wird nicht auf
die native Session umgedeutet.

`game_mode` stammt aus demselben tatsächlichen PlayerState; `hashed_seed` wird
im Simproducer durch den bestehenden Worldproducer
`BiomeZoomSeed::from_world_seed(state.binding.generation().identity().seed()).bits()`
berechnet: SHA256 der acht little-endian Seedbytes, erste acht Digestbytes
wieder little-endian als i64. Sim besitzt bereits die Worlddependency; Session
benötigt weder neue World-/Registrydependency noch Sha2. Eine callerlieferbare
Hashzahl oder bereits gehashte Seedzahl ist kein Ersatz dieses Producers.

SIM-03.4 selbst ist akzeptiert. Dieser **zusätzliche** Liveview-/Lifecycleproducer
muss separat geprüft und auf seinen tatsächlichen späteren Sourcehash gebunden
werden; sein Entwurf behauptet keine bereits vorhandene Funktion.

## Enger tatsächlicher App-/JoinGameproducer

Die gewählte Form ist ein **tatsächlicher Layer-A-JoinGameproducer im Router**,
kein nachträglicher callerlieferbarer IDbeleg. Vorgeschlagene APIs:

```rust
pub struct NativeJoinReservation { /* sealed router/key/wire/declared scope */ }
impl SessionRouter {
    pub fn reserve_native_join(
        &mut self,
        player: PlayerId,
        declared_scope: GenerationBinding,
        declared_position: Vec3,
    ) -> Result<NativeJoinReservation, CollectorSessionError>;

    pub fn admit_native_join(
        &mut self,
        reservation: &NativeJoinReservation,
        admitted: &OwnedPlayerHandle,
        current: &ConnectedOwnedPlayerView<'_>,
        join_game: JoinGame,
        name: &str,
        equipment: Vec<u8>,
    ) -> Result<PlayerSessionHandle, CollectorSessionError>;
}
```

Reservation ist ein begrenzter provisorischer Netzwerkslot, **kein** Simadmission-
oder Generationproof. Declaredposition dient dem reinen vorherigen Peerburst-/
Workpreflight, stammt im Driver aus dem echten Native-Read/Bootstrapplan und
muss später bitgenau zur tatsächlichen Liveviewposition passen. Sie erzeugt
keinen Pose-/Ownerproof. Der Driver reserviert nach echter UUIDreservation,
vor native Restore/Bootstrap/Slowviewerteardown. Pure count-/counter-/wire-
Refusals liegen vor seinen Sideeffects. Nach echter `PlayerAdmitted`-Produktion
legt der Driver die Beziehung genau dieser Reservation zu genau diesem echten
Handle an; Join kann nur einmal fertiggestellt werden. Keine Legacy-PlayerJoin-
Erzeugung in diesem native Zweig und keine zweite connectionseitige Inventory-
authority. Legacyjoins prüfen dieselben neuen Countergrenzen vor ihrem alten
PlayerJoin und vor Slowviewer-Disconnect.

`admit_native_join` vergleicht aktuelle View, Handle und deklarierte Scope mit
der wirklichen Registrierungslease. Die konkrete Registry-/SpawnInfovalidierung
ist unten festgelegt, einschließlich echter Mode-/Hash-/Poseproducer. Es
validiert den **tatsächlichen typed** JoinGame; Self-ID ist in dieser Stufe1.
Anschließend queued es genau dieses JoinGame als **erstes Mandatory/State-
Paket in den tatsächlichen Outbound dieser Session**. Nur dieser erfolgreiche
interne Enqueue setzt die private Selfbinding. Die callerlieferbare Packet-ID
allein, ein Constructor oder `admit_native_join` ohne tatsächlichen Enqueue
setzen keine Selfbinding. Capacity für Self plus Peerburst wird vorher checked.
Danach folgen die Mandatory Add-/Spawnpaare und ihre wirklichen Baselines.

Die zwingende Appfolgeänderung in `join_simulation`/`send_join_kit` konsumiert
dieses **exakte erste** JoinGame-Outboundpaket, entfernt den direkten erneuten
`ctx.join_kit.join_game()`-Versand, sendet den verbleibenden Stage1-Kit und
flusht Stage1 vor Chunks und normalem Outboundpump. Kein beliebiges späteres
Packet kann eine fertige Session umbiegen. Dadurch liegt tatsächlicher JoinGame
auf demselben realen FIFO-Pfad vor jedem Take an Self. Layer-A-Enqueue bleibt
ausdrücklich **kein Socketreceipt**; Encode-/Writer-/Socketfehler benutzen den
wirklichen Appteardown, Quarantäne und retained native Detach.

Native Detach braucht den tatsächlichen `OwnedPlayerInput::disconnect` mit
vorhandenem genuine Handle und dessen echte Admission/Applybestätigung im
Driver. Der heutige PlayerLeave-GameInput wird im Nativeowner ignoriert und
genügt nicht. Bis diese genaue Controlbridge umgesetzt/geprüft ist, bleibt
native Collectorbinding unaktivierbar. Vorläufige Reservationrefusal/drop
gibt nur deren Netzwerkslot frei; bereits admitted Actoren und ihre Pending/
Publikationsgruppen dürfen dadurch nicht verschwinden. Der Driver bleibt
Eigentümer ihres echten Cleanup/Retry. Kein Sessiontyp erhält einen SimShard.

## Exakte Registry-, SpawnInfo- und Poseprüfungen

Der echte Appregistryproducer ist `ConfigRegistries::build` in `registries.rs`:
KnownPack `(minecraft, core, 1.21.8)` und `minecraft:dimension_type` mit genau
einem Entry `minecraft:overworld` an Index0, Data=None. Der wirkliche Sender
`Connection::send_registries_and_finish` sendet genau die Records der in
`ConnContext.config` gehaltenen Arc. Session erhält beim einmaligen Startup
den **tatsächlichen typed** Dimension-RegistryDatarecord und KnownPack dieses
gleichen Appobjekts; `ferrumc-proto` ist bereits eine Sessiondependency.

```rust
impl CollectorSessionLimits {
    pub fn new(
        sessions: NonZeroUsize, baselines_per_viewer: NonZeroUsize,
        total_baselines: NonZeroUsize, audience: NonZeroUsize,
        work: NonZeroUsize, scratch: NonZeroUsize,
    ) -> Result<Self, CollectorSessionError>;
}
impl SessionRouter {
    pub fn initialize_collector_sessions(
        &mut self, limits: CollectorSessionLimits,
        dimension_registry: &RegistryData, core_pack: &KnownPack,
        simulation_distance: i32,
    ) -> Result<(), CollectorSessionError>;
}
```

Initialization prüft vor Enumaration exact Entrycount1, Registryidentifier,
Entryname/Data und Packnamespace/ID/version; daraus entsteht privat
`NativeRegistryLayout::FlatOverworld1_21_8` mit Dimensionindex0. Dieses Layout
ist **Konfigurationsdatenvalidation**, kein erfundener Konfigurations-/Socket-
Receipt. Appfolgearbeit muss beide Daten aus demselben tatsächlichen
ConfigRegistriesobjekt nehmen, das die Connection verwendet. Tests prüfen den
wirklichen Registryproducer und dessen Decoderframes; ein Callerlabel reicht
nicht. Andere Registrylayouts sind in dieser Stufe explicit Unsupported.

Der Join vergleicht über vorhandene typed Getter vollständig:

- Die echte Livegeneration hat WorldId0/DimensionId0, release `(1,21,8)`,
  Generator `FerrumcFlat` revision1 und Overworlddimension; darüber hinausgehende
  Generation bleibt Unsupported, bis echte Registry-/Joinproducer dafür stehen.
- `JoinGame.entity_id()==1`; `world_names` hat genau einen Entry und dieser
  heißt `minecraft:overworld`; `world_state.dimension_type()==0` entspricht
  dem tatsächlich geprüften Layout, `dimension_name` entspricht demselben Entry.
- `world_state.hashed_seed()==current.hashed_seed()` aus dem oben benannten
  World/Simgenuine Producer; niemals bloß Vergleich mit der Callerhashzahl.
- `world_state.gamemode()==current.game_mode().as_id()`, previous_gamemode255,
  is_debugfalse, is_flattrue, death_locationNone, portal_cooldown0, sea_level63.
  Der heutige stets Creative-JoinKit passt zu Survival **nicht**: App muss
  SpawnInfo aus der realen native Mode bauen, statt nachträglich durch ein
  Creativeadvertisement zu korrigieren. Freshnative Survival meldet Mode0.
- Für diese bisherige reale Konfiguration: hardcorefalse, max_players32,
  reduced_debug_infofalse, enable_respawn_screentrue, limited_craftingfalse,
  enforces_secure_chatfalse. Viewdistance entspricht dem konfigurierten Router;
  simulation_distance ist der am Startup aus tatsächlichem AppConfig übernommene
  und wie dort in0..=32 validierte Wert;0 ist tatsächlich erlaubt, kein Clamp.
  Andere Configpolicies verlangen explizite Layoutaufnahme.

JoinGame enthält **keine Pose**. Session mintet deshalb daneben eine private,
readonly `NativeSpawnProjection` im tatsächlichen PlayerSessionHandle aus
`current.position/look/game_mode`: jede f64-Positionsachse finite und
abs≤3.0e7, Look-f32 finite, keinerlei Winkelnormalisierung. Sessioncached Pose
und Projection übernehmen identische floatbits. App nutzt ausschließlich diese
Projection für das tatsächliche `spawn_sync`/`PendingTeleport::issue` in Stage1:
Pos-/Lookbits stimmen, dx/dy/dz=+0.0, flags0 und echte neu issued positive
Teleport-ID. Sie benutzt keine Legacyloadposition oder Spawnkitdefaults.
Diese konkrete Übergabe und Framevergleich gehören zur Appfolgeprüfung;
Posegetter allein behauptet keine Socketzustellung. Abilities und Inventory
müssen ebenfalls aus derselben native Mode/Admission kommen, bevor dieser
Pfad produktiv aktiviert wird.

## Genuine Retirement, Publikationspins und terminale leere Takephase

Fehlender Tracker allein ist fehlende Authority und bleibt Refusal. Eine
**wirklich retired ursprüngliche Session** bekommt dagegen eine neue sealed
Disposition. Der zusätzliche Simproducer muss dafür pro **realer nativer
Admission** einen private Lifecycleeintrag erhalten; keine Callerbooleans:

```rust
impl SimShard {
    pub fn collector_lifecycle_view<'a>(
        &'a self, original: &OwnedPlayerHandle,
    ) -> Result<CollectorLifecycleView<'a>, PlayerOwnershipError>;
    pub fn acknowledge_collector_lifecycle(
        &mut self, ticket: &CollectorLifecycleTicket,
    ) -> Result<(), PlayerOwnershipError>;
}
pub struct NativePlayerDetachFact { /* sealed original owner/admission/generation */ }
pub struct CollectorLifecycleView<'a> { /* borrowed genuine lifecycle ledger */ }
impl CollectorLifecycleView<'_> {
    pub fn matches_owned_player(&self, original: &OwnedPlayerHandle) -> bool;
    pub fn generation(&self) -> GenerationBinding;
    pub fn live(&self) -> Option<ConnectedOwnedPlayerView<'_>>;
    pub fn detach(&self) -> Option<&NativePlayerDetachFact>;
    pub fn reserved_take_pins(&self) -> usize;
    pub fn retained_take_pins(&self) -> usize;
    pub fn retirement_ticket(&self) -> Option<CollectorLifecycleTicket>;
}
```

Lifecyclezelle wird vor realer Restore-/Bootstrapadmission als Scalarcredit
reserviert und erst mit dem tatsächlich admitted Handle/Ownerweak und voller
Generation versiegelt. Vorbootstraprefusal/definitiver Bootstrapabort gibt den
Credit frei; Bootstrap-Uncertainty hält ihn, ohne native Admission zu erfinden.
Zelle bleibt bei echter Admission bis zur terminalen Bestätigung erhalten.
Native-Rejoin erhält eine **neue** Zelle/Admission; alte Zellen bleiben separat.
Der private Lookup prüft Weakowner und exakte Admission der ursprünglichen
Handle, auch wenn PlayerState inzwischen tatsächlich rejoined ist.

`NativePlayerDetachFact` entsteht **einmal** direkt beim tatsächlichen Apply
des echten `OwnedPlayerInput::disconnect`, nachdem genau dieser Spieler
connectedfalse wurde. Er bindet ursprüngliche private Ownerweak, PlayerId,
Admission und ganze Generation. Wiederholter Disconnect derselben Admission
liefert denselben bestehenden Fact. Bare `PlayerDetached{player}`, enqueued
Disconnect, dead Weak, Scope-/Endpointwechsel und ein Callerreceipt reichen
nicht. Seine Getter/Matcher geben weder Ownerweak noch Mutationshandle heraus.
Der recordgedeckte Fact ist bereits scalarvorbereitet, sodass Mandatory Detach
nicht erst nach Mutation an zusätzlicher Allocation/Capacity scheitert.

Für jedes vor Workrequest tatsächlich vorbereitete `take:Some` hält die
Originaladmission **genau einen Publicationpin**, zusammen mit den bereits
wirklichen SIM-03.4-Group-/Byte-/Sequencecredits. Die echte private
`PreparedItemPublication` bekommt einen scalaren zugehörigen Ledgerpin; weder
Caller noch Session können ihn erzeugen. Gesamtzahl reserved+retained Takepins
≤konfiguriertes SIM-03.4-Gruppenlimit, heute höchstens256. Pinledgergröße ist
vor Bodycopy/Workrequest checked, und der vorhandene Countershadow benutzt
genau dieselben scalaren Übergänge vor seinen weiteren Admissionresources.

| Tatsächlicher Weg | Originaladmission-Pin / Lifecyclezelle |
|---|---|
| Takepreparationrefusal vor Pending | Nur staged Credit zurück; keine übernommene Mutation oder Fact |
| Pendinginstall | reserved Takepin genau einmal übernehmen |
| Confirm | reserved→retained ohne Gesamtwachstum; Fact/Gruppe gehört weiter derselben Admission |
| Busy/Retry/Lostreply/ReceiptAbsent/Refresh/Quarantine | Pin und Zelle vollständig erhalten; kein zweiter Pin, kein terminaler Zeroproof |
| Tatsächlicher definitiver `release_pending`-Abort | Genau den reserved Pin freigeben, gemeinsam mit existierenden Publicationcredits |
| Wirkliches native Detach | Fact einmal versiegeln; **alle** reserved/retained Pins erhalten |
| Vollständige normale oder terminale ganze Gruppenannahme plus gültiger echter FIFOheadack | Genau deren retained Pin freigeben |
| Refusal, stale/foreign/doppelter Ack oder halbe Aufnahme | Kein Pin-/Zellrelease |
| Detached, beide Pinzahlen0, Sessionretirement und Consumerreferenzen abgeschlossen | Genuines Lifecycle-Ticket erlaubt Freigabe genau dieser Zelle |

Sim-Lifecyclecountcap ist positive, checked
`owned_player_limit + item_publication_group_limit`; jeder zusätzlich behaltene
alte Admissioneintrag muss mindestens einen Pin oder einen noch nicht
acknowledged Detachfact besitzen. Noch nicht acknowledged, pinlose alte Facts
zählen **weiter** gegen dieses Cap; neue Restore/Bootstrap/Rejoin verweigern
vor Mutation bei Capacity. Lifecycle-Outercharge ist exakt
`size_of::<Vec<Option<CollectorLifecycleCell>>>() + cells * size_of::<Option<CollectorLifecycleCell>>()`;
alle Operationen checked, zusätzlicher positiver Scalarbytecap höchstens1MiB,
initialisiert fallible vor Aktivierung und vor der ersten nativen Admission,
ohne neue Item-/NBTbodys. Gewährte Allocatorcapacity/-overhead ist keine
behauptete Heapmessung. Keine
unbounded Rejoinhistorie. Ownershipclone enthält wie bisher keine runnable
Ledgerzellen/Detachfacts/Pins; SnapshotOnly kann keinen Proof erzeugen.

Session reserviert **bei jedem NativeJoin** bereits genau einen Retirementslot.
`P_installed + native_provisional + retired_records <= sessions_limit` gilt
gemeinsam. P_installed enthält **alle** Legacy- und Native-Mappings einschließlich
pending Leave. Aktive Nativecollectorrecords sind bereits in P enthalten und
werden **nicht noch einmal** gezählt; retired Records zählen erst nach tatsächlicher
Mapentfernung an ihrer Stelle. Ein tatsächlicher native Owner/Admission darf innerhalb des Routers
nur an einen Sessionkey gebunden werden; der echte Handlevergleich prüft dies
vor Join. Retiredrecords verhindern den erneuten Gebrauch derselben Admission;
neuer Rejoin braucht genuine neue Admission. Kein zusätzlicher Retirement-
Allocate/Countrefusal erst nach Mandatorydisconnect.

Der echte Appdriver liefert den sealed Detachfact der tatsächlich applied
ursprünglichen Admission an `SessionRouter::retire_native_session(key, fact)`.
Session prüft vorhandene **ehemals Active** Originalbindung, exakten eigenen
Sessionkey, Fact/Storedhandle/Generation und vorher bereits Leaving. Erst
nach tatsächlichem Native-Detachapply entfernt sie das native Mapping und
reconciled alle zugehörigen Collectorbaselines: Mandatory Remove an verbleibende
Viewer; Full/Closed markiert diese Leaving und hält deren echten Cleanup.
Selfoutbound ist geschlossen/aus dem Router entfernt. Kein Activeviewer behält
eine zugelassene Originalcollectorbaseline. Ein kontrolliertes Registryretirement
kann so privat `CollectorRetirementProof` minten, gebunden an **genau**
Originalowner/Admission/Generation/Sessionkey und echte Baselineentfernung.
Pending Leave, stale Lease, nie gebundene Selfsession oder bloß missing Tracker
minten keinen solchen Proof. Alte volatile Actoren ohne genuine Detachbestätigung
werden weiter als echte nicht erledigte Cleanup behandelt.

`CollectorDeliveryDisposition::Retired(proof)` bedeutet: dieser ursprüngliche
Collectortracker ist tatsächlich beendet; seine historische Takeaudience ist
für spätere Ausgabe **beweisbar leer**. Es wird kein Take/Selfpaket erzeugt.
NET-04.2b führt anschließend die **normalen vollständigen** Changed-/Removed-
Phasen derselben Gruppe für ihre wirklichen Itemviewer aus. Der echte Headack
ist erst danach erlaubt und released exakt den retained Sourcepin. Es handelt
sich weder um Groupdiscard noch um Überspringen ihrer Itemänderung. Auch nach
Detach noch bestätigende vorher reserved Pickups benutzen denselben old Pin/
Retirementproof und können FIFO/Stop deshalb bis zur wirklichen Klärung drainen.

Session hält zusätzlich Consumerreferenzen pro Retiredrecord für tatsächlich
admitted, noch nicht vollständig quittierte Gruppenpläne; deren Gesamtcount
≤Publicationgruppenlimit, vor Planaufnahme geprüft. Nach echter Gruppenquittung
werden diese exakt released. Erst genuine Lifecycleview mit detached, beide
Sourcepinzahlen0 und private Consumerrefs0 erlaubt Sessionrecordrelease; danach
bestätigt der Driver das exakt owner/admissiongebundene Lifecycle-Ticket in Sim.
Ticket ist kein Socketreceipt und mintbar nur aus einer solchen realen Zelle;
Ack revalidiert detach/0pins und konkrete Zellenidentität. Stale/reordered/
foreign/doppelter Release verweigert unverändert. Neue Process-/Scope-/Owner-
incarnation kann alte Tickets nicht verwenden. Drain erlaubt Gruppen-/Lifecycle-
ack, Stop verlangt keine reserved/retained Gruppen und keine ungelösten
Lifecycle-/Retirementcredits. Uncertain-Mutationen werden weiter durch die
bisherige echte Clarification aufgelöst, niemals durch einen Retirementabort.

## Tatsächliches Collectortracking und readonly Audience-API

Native Playerbaselines werden nach tatsächlicher Mandatory Spawnadmission
unter der **Sessioninkarnation des Subjects** gespeichert. Jede enthält
Subjectkey, dessen native Generation, wirklichen Viewer-Wirewert und letzte
zugestellte Pose. Keine UUID-/i32-only Baseline aus einem alten Join wird
übernommen. Der numerische BTreeMapindex ist der innerhalb des Routers checked
vergebene Sessioncounter, keine Weakadresse; die vollständige Bindung bleibt
im Value und wird bei Lookup überprüft.

Die Audience enthält exakt alle aktuellen Active-/nicht Leaving-Viewer, deren
wirkliche Collectorbaseline zu dieser Subjectsession passt, plus Collector-
Self **nur bei vorhandener Active-Collectortrackerbindung**. Self erhält1;
Observer erhalten den Remote-Wirewert ihrer tatsächlichen Collectorbaseline.
Es wird weder eine neue Audience aus Positionen geschätzt noch das Itemviewer-
Set als Filter benutzt. Ein Collectorobserver ohne Itembaseline bekommt gemäß
Paper trotzdem Take; für dessen Item-ID ist die unten genannte echte Mapping-
admission erforderlich. Ein Itemviewer ohne Collectorbaseline bekommt kein Take.

```rust
impl PlayerSessionHandle {
    pub fn session_key(&self) -> PlayerSessionKey;
}
impl SessionRouter {
    pub fn collector_delivery<'a>(
        &'a self,
        collector: &'a ItemCollectorBinding,
        lifecycle: &'a CollectorLifecycleView<'a>,
    ) -> Result<CollectorDeliveryDisposition<'a>, CollectorSessionError>;
    pub fn retire_native_session(
        &mut self, session: &PlayerSessionKey, applied: &NativePlayerDetachFact,
    ) -> Result<(), CollectorSessionError>;
    pub fn revalidate_native_endpoint(
        &mut self, session: &PlayerSessionKey, current: &ConnectedOwnedPlayerView<'_>,
    ) -> Result<(), CollectorSessionError>;
}
pub enum CollectorDeliveryDisposition<'a> {
    Tracked(CollectorAudience<'a>),
    Retired(CollectorRetirementProof<'a>),
}
pub struct CollectorAudience<'a> { /* private router borrow + bounded entries */ }
pub struct CollectorRetirementProof<'a> { /* private exact retired record borrow */ }
impl CollectorAudience<'_> {
    pub fn len(&self) -> usize;
    pub fn viewers(&self) -> impl ExactSizeIterator<Item = &CollectorViewerBinding>;
}
pub struct CollectorViewerBinding { /* private actual session key + wire */ }
impl CollectorViewerBinding {
    pub fn session(&self) -> &PlayerSessionKey;
    pub fn collector_wire_id(&self) -> i32;
    pub fn is_collector_self(&self) -> bool;
}
```

Beide Dispositionspayloads haben keine public factory/Clone, Packetbytes, Sender
oder Ack-/Detachauthority. Retired kann nur der private echte Retirementpfad
erzeugen; Tracked verlangt die tatsächliche `lifecycle.live()`-View.
Der Borrow verhindert Routermutation während ihres Lesens. NET-04.2b
bereitet daraus intern den vollständigen Gruppe-/Viewerversandplan vor und
prüft nach Ende des Borrows, unmittelbar vor Mutation, alle exakten Session-
keys, Lifecycle und Bindungen erneut. Ein kopierter Viewerkey ist kein Liveproof.
`revalidate_native_endpoint` darf ausschließlich exakte aktuelle Leasegeneration
für gleichen Owner/Admission/Generation und registration_id/home aktualisieren;
es ersetzt weder Handle noch Sessionkey. Die tatsächliche Appnative Service-
wiring muss die View aus dem **wirklich an den Replacementreceiver angeschlossenen**
Owner liefern. Ein gültiger alter Actorview und eine passende Shardnummer allein
beweisen nicht, welches Ziel den neuen Receiver konsumiert. Ohne diesen echten
Endpointproducer bleibt EndpointSuspended; es gibt keinen callerzahlbasierten
Revalidateadapter. Gleiches gilt bei der ersten Nativebindung.

Beim tatsächlichen Pose-, Viewdistance-, Chunktracking- oder Scopestatuswechsel
werden betroffene Pairings **vor der nächsten Audienceabfrage** abgeglichen.
Bewegung betrifft Subject→Observer und alle bisherigen Subjects→bewegten Viewer.
Verlassen invalidiert die Collectorbaseline und queued Mandatory Entityremove;
Reentry erfordert neues wirklich admitted Spawnpairing und neue Posebaseline.
Full/Closed sperrt den Viewer sofort als Leaving und ruft den bisherigen
iterativen echten Leavevertrag auf. Ein zurückgehaltener Leave erzeugt niemals
erneut Takeeligibilität. Bloßes Viewerunpairing ist kein ItemInstanceRemovefact.

Die tatsächliche Chunk-/Policyeligibilität des künftigen Papertrackers muss aus
den echten bestätigten Appchunkadmissions und tatsächlichen Sim-/Java-
Visibilityentscheidungen kommen. Ohne diesen zusätzlichen Producer bleibt die
existing native Range-/Pairingstufe ausdrücklich begrenzt; diese Audience-API
macht keine fehlenden Chunktickets oder Pluginentscheidungen aus Callerlisten.

## Positive Caps, Admission und Allokationsgrenze

Opt-in `CollectorSessionLimits::new` nimmt sechs **NonZeroUsize**-Werte;
keine 0-Clamps oder unbounded Defaults. Einmalige Initialisierung erfolgt vor
Sessions/provisorischen Reservations/Baselines. Kein Reset in derselben Router-
inkarnation. Vorgeschlagene überprüfbare Hardmaxima:

| Neues Limit | Hardmaximum | Zählt / Zeitpunkt |
|---|---:|---|
| Sessions | 1024 | sämtliche installed Mappings inkl. Legacy/pending Leave + native provisional + retired Records; aktive Nativecollectorrecords nicht doppelt; vor jedem Join |
| Playerbaselines je Viewer | 1023 | tatsächliche Pairings; vor Add-/Spawnadmission |
| Playerbaselines insgesamt | 262144 | Summe aller Viewerpairings; checked vor Baselinewachstum |
| Collector-Audience | 1024 | Observer plus genau einmal Self; vor Snapshot-/Packetvorbereitung |
| Readonlyarbeit | 4194304 Eintragsbesuche | upper bound vor neuem Scan/Reconcile, kein stiller Teilprefix |
| Scratch | 1048576 Bytes | checked Outerarraycharge vor `try_reserve_exact`, keine Bodykopie je Viewer |

Config prüft per-viewer < Sessions und Audience ≤ Sessions. Bestehende Session-
Outbound- und Controlreservegrenzen bleiben unverändert. Große Configs sind
keine erlaubte Speicher-/Performancebehauptung.

Seien `P=players.len()`, `C=collector_records.len()` einschließlich aller active/
provisional/retired Records. Für P==0 gilt R=0; andernfalls wird zuerst
`P.checked_sub(1)` und danach `checked_mul(2)` ausgeführt: R ist genau
`2*(P-1)`, kein Saturating-/Underflowersatz. Beide Counts sind vor
dem Scan bekannt und ≤Sessionslimit. Für Disposition/Audience wird vor Scan
checked Worst-casearbeit `C + 2*P + 2` admitted: Bindungsrecords durchsuchen,
Countscan und Fillscan der wirklichen Viewermap. Der Countscan validiert
baselines/lifecycle, ohne Daten zu kopieren oder zu mutieren. Erst nach dessen
Audiencecapprüfung wird die folgende **tatsächliche** Typsizecharge checked
gegen Scratch admitted und fallible exakt reserviert:

```text
S_audience = size_of::<CollectorAudience<'_>>()
           + count * size_of::<CollectorViewerBinding>()
S_retired  = size_of::<CollectorRetirementProof<'_>>()
S_reconcile = size_of::<ReconcilePlan>()
            + R * size_of::<ReconcileDelta>()
            + 2 * P * size_of::<PlayerSessionKey>()
W_reconcile = P + 3*R + 2*P*P + 8*P + 8
```

Audience-/Reconciletypen halten nur echte Schlüssel, Scalarposes/-wire und
Borrowmetadaten; keine Name-/Equipment-/Packetbodykopien im Plan.
`ReconcileDelta` besitzt Enter/Leave-Tag und echte Viewer-/Subjectkeys, Wire
und Pose. Die zwei P-Keyarrays sind begrenzte Cascadeworklist und attempted-
IDs. `2*P*P + 8*P +8` begrenzt konservativ die notwendigen Lookup-/Leave-
Cascadeeintragsbesuche; R-Prefixscan/Count/Fill benötigt höchstens3R.
Das sind positive explizite Operationcounts, keine CPUzeitmessung.
Jede Multiplikation/Addition checked; keine Fixedheaderzahl aus Vermutung.
Fill verwendet nur cheap sealed Bindungen und Scalars; keine Item/NBTkopie.
Neue Reconcilearbeit nimmt höchstens R Beziehungen auf; tatsächlicher Leave-
Count L und neuer Entercount E werden ohne Mutation ermittelt. Vor zusätzlichem
Spawn/Insert gilt checked `total_baselines - L + E <= total_limit` und je
Viewer dasselbe mit dessen echten Delta. Ganzes Reconcile nimmt W/S-Credit
**vor** Pose-/Baselineänderung, Peerpacketaufbau, neuen Paarungen und Sends auf.
Bestehender Name-/Equipment-/Packetaufbau folgt erst nach Admission unter
seinen bereits vorhandenen bounded Produzenten und der genannten infallible
Allocatorgrenze; er wird nicht als scratchfreie physische Heapkopie behauptet.
Ein überschrittener Cap behält den ganzen
Consumerinput; kein halb abgeschlossener Reconcile wird als fertig gemeldet.

Disconnect/retained-Leave-Recovery erhält einen getrennten, durch Sessions und
Gesamtbaselines begrenzten Lifecycleworklistpfad; Collector-Audience-/Scratch-
Refusal darf Mandatory Cleanup nicht blockieren. Keine Rekursion. Das heutige
BTreeMap-/String-/Packetallocationverhalten bleibt eine infallible Allocator-
grenze; hier wird keine all-OOM-Resultgarantie erfunden. Neue Outerarrays sind
fallible und neue logische Insertionscounts werden vor Sideeffects geprüft.

## Gemeinsamer Entity-Wirenamespace und checked Erschöpfung

Ein **einziger Routerallocator** vergibt zukünftig sämtliche Remoteplayer-,
Item- und übrigen Entity-Wire-IDs monoton aus `[2, i32::MAX]`.0 wird nicht
vergeben,1 bleibt die tatsächliche Self-ID dieser Appstufe. Kein wrapping,
keine negative ID, Freelist oder Wiedervergabe nach Leave/Scopewechsel.
Der letzte gültige Wert MAX kann einmal benutzt werden; danach ist der
Allocator explizit erschöpft. Sessioncounter MAX ist ebenfalls einmal gültig.

Vor einem Join werden beide Counter, neue Count-/Work-/Burst-/Baselinelimits
und Scope readonly geprüft. **Vor jedem Slowviewer-Disconnect, PlayerJoin oder
Nativeadmissionsideeffect** wird die konkrete verfügbare Counterreservation
festgelegt. Reservationen verbrennen ihre Werte auch bei späterer Refusal;
das verhindert ABA und benötigt keine Rollbackfreelist. Erschöpfung verändert
weder Mapping, pending Leave, Channels noch Viewer und erzeugt keinen PlayerJoin.

NET-04.2b darf Sim-Item-Wirewerte nicht einfach parallel in denselben
Clientnamespace übernehmen. Bei der ersten tatsächlichen Iteminstanceadmission
mintet es eine echte Router-Wirezuordnung aus diesem gemeinsamen Allocator,
gebunden an die vollständige sealed `ItemInstanceBinding`. Alle Spawn-/Metadata-/
Motion-/Take-/Removepakete verwenden diese Zuordnung. Gleicher Simwirewert in
anderem Scope/Incarnation bekommt eine andere Router-ID. Zuordnung erst nach
vollständiger Removegruppenannahme freigeben, numerische ID trotzdem nie
wiederverwenden. Fehlt die Zuordnung für Take, wird nichts erfunden und die
gesamte Gruppe bleibt retained. Separate Iteminstanz-/Payloadcaps und die
tatsächliche neue Mappingproduktion bleiben NET-04.2b; die Namespaceentscheidung
ist damit festgelegt, ohne Sim IDs oder Plugins getEntityId bereits zu migrieren.

## Ordering, Admission und vollständige Refusalmatrix

Alle Take-/Changed-/Removephasen bleiben auf der geprüften Mandatory/State-Lane.
NET-04.2c versendet oder acknowledged keine Itemgruppe. NET-04.2b muss zunächst
**die gesamte bestätigte Gruppe** planen: gültige Instance-/Wirezuordnung,
genuine Collectorbindung bei Take, Itemviewerfolger, vollständige Packet-/Byte-
admission und alle genauen Viewerkeys. Erst dann können gruppierte Channelslots
oder ausdrücklich idempotenter Fortschritt aufgenommen werden. Take liegt bei
jedem betroffenen Viewer vor dessen Changed-/Removefolger. Keine Changed-/
Removeaufnahme, kein Ack und keine Bytefreigabe, solange der frühere Take der
Gruppe an fehlender Binding scheitert. Die Collector- und Itemaudiences dürfen
verschieden sein; der Consumer plant ihre tatsächliche Union ohne neue Viewer-
oder Itemauthority. Channelannahme ist keine Socketbestätigung.

| Wirklicher Zustand / Fehler | Sessiondisposition | Bestätigte ganze Itemgruppe |
|---|---|---|
| Aktuelle reale Admission, Selfjoin und Collectortracker | Viewer IDs aus admitted Collectorbaselines plus Self; normale Gesamtadmission | Ack erst nach vollständigem NET-04.2b-Vertrag |
| Legacy-/SelfJoinPending-/ungebundene Session, fehlender native Producer | `CollectorBindingUnavailable`; keine Fake-ID, keine Umwandlung UUID-only | Head/Body/Sequence/Charge inklusive Changed/Remove erhalten |
| Foreign Router/Owner oder andere native Admission bei gleicher UUID | `ForeignSession`/`CollectorAdmissionMismatch`; kein Retarget | Unverändert retained |
| Veraltete Admission, bloß enqueued Detach, alter UUID-only Leave oder Rejoin ohne Originalretirementproof | `CollectorNotCurrent`; neue Session ersetzt alten Fact nie | Unverändert retained; kein Take/Changed/Remove ohne vollständigen Vertrag |
| Genau ursprünglicher native Detach **applied**, ehemals Active Session tatsächlich retired, sealed Retirementproof unter Sourcepins vorhanden | `Retired(proof)`; ursprüngliche Takeaudience beweisbar leer, kein Take/Selfpaket | Normale vollständige Changed-/Removedadmission, danach echter Headack und genau ein retained Pinrelease |
| Pending Leave oder geschlossenes Collectoroutbound | Sofort Leaving/nicht eligible; echte retained Cleanup-/Retryordnung | Kein Take/Changed/Remove der Gruppe; retained |
| Viewer full/closed bei Mandatory Gruppenadmission | Exakte Viewerinkarnation Leaving; echter Leave/cascade, Full/Closed/stale behält Mapping/Leave | Kein Ack bei unvollständigem Gruppenvertrag; bereits zugelassener Fortschritt nur nach geprüfter Idempotenz |
| Farviewer hat nie Collectorpairing / bereits echt unpaired | Nicht in Audience, unabhängig vom Itemview | Kein erfundener Empfänger; übriger echter Gruppenvertrag bleibt |
| Subject-/Viewerbewegung mit noch ausstehendem Reconcile | `TrackingNotCurrent`, kein Scan alter Baselines als fertige Audience | Ganze Gruppe retained bis tatsächlicher Reconcile |
| Authorized Endpointrotation, gleiche Lineage/home | Native Binding `EndpointSuspended`; alter Data-/Leavepfad folgt weiterhin aktueller Senderrotation | Retained bis echte neue Liveview gleiche Admission/volle Generation auf neuer Lease nachweist |
| Rotation durch denselben genuine Owner/Generation bestätigt | Nur Leasegeneration aktualisieren; keine Admission/Session-/Wire-/Baselineumdeutung | Alte Facts derselben tatsächlichen Admission können wieder admitted werden |
| Andere Owner/Generation/Scope beim Replacement | Kein stiller Rebind; `CollectorNotCurrent`; alte Cleanup-/Mappingpflicht bleibt | Unverändert retained |
| Unregister/Register oder fremde Directorylease | `StaleShardBinding`/Foreign; kein Worldfallback/Lineage-ABA | Unverändert retained; echtes Leave bleibt retrybar repräsentiert |
| Dimensions-/World-/Seed-/Generator-/Revisionwechsel | Alte native Sessionbindung invalidieren; vor neuem native Join alte Pairings Mandatory entfernen | Alter Fact nie an neuen Scope liefern; retained |
| Kein Collectortracker ohne echten Retirementproof / kein Itemwire-Mapping | `CollectorTrackerUnavailable`/`ItemWireBindingUnavailable`; kein Self-only oder leer erfundener Fallback | Ganze Gruppe einschließlich Folgern retained |
| Count-/Work-/Scratchcap, checked Overflow, neue Outerallocationrefusal | Klassifizierte Refusal vor Scanallocation/Mutation/Sends, vorhandene Lifecyclepflege erhalten | Input/FIFOhead unverändert retrybar |
| Wire-/Sessioncounter erschöpft | Neuer Join/erste Entityadmission vor allen Sideeffects verweigert | Kein wrapping/Rebind; bereits vorhandene Zuordnungen bleiben gültig |
| Drain, aktuelle Session noch connected | Bestehende Gruppen/Ack/Leavepfade zulässig; keine neue Gameplayadmission | Vorhandene echte Gruppen abarbeiten |
| Shutdown mit unresolved Mutation, pending Detach oder fehlender originaler Retirementauthority | Echte Clarification/Controlretry; kein erfundener Erfolg | Retained bis tatsächlicher Confirm/definitiver Abort bzw. genuine Retirementdisposition |
| Drain nach applied originalem Detach/Retirement und vollständiger Ausgabe aller alten Confirmgruppen | Source- und Consumerpins0 echt nachweisen, dann genaue Lifecycle-/Sessioncredits freigeben | FIFO und Retirementledger leeren sich; Stop kann tatsächliche Quiescence erreichen |

Damit blockiert ein **wirklich nachgewiesener** normaler Disconnect die
Publikations-FIFO/Stop nicht dauerhaft: schon reserved Pickupmutationen können
noch bestätigen, ihre genuine terminale Takephase bleibt leer und ihre übrige
Gruppe wird vollständig ausgegeben/acknowledged. Uncertainty/fehlende Apply-
oder Retirementauthority benötigt weiterhin echte Clarification/Retry und darf
nicht als Erfolg erfunden werden. Produktive Aktivierung setzt die beschriebenen
zusätzlichen Sim-/App-/Sessionproducer und deren wirkliche Abnahme voraus.

## Meaningful spätere Abnahme und nächste Produktionsentscheidung

Es wurden für diesen readonly Entwurf **keine Runtime-/Fakefacttests** erstellt
oder ausgeführt. Nach getrennten Producerfreigaben sind erforderlich:

- Echte Routerchannels, Memory/redb-Nativebootstrap/-restore/-pickup und
  original admitted Handle/Collectorgruppe; Frameencode und Decoder prüfen
  Observercollector-ID ≥2, Selfcollector-ID1, unveränderten Originalcount.
- Drei unterschiedliche echte Audiences: Item-only Viewer, Collector-only
  Viewer, beide; genau Collectortracker plus Self bekommt Take. Fehlender
  Collectortracker ergibt kein Self-only Paket. Kein arbitrary testbinder.
- Wirklicher Eintritt/Austritt/Reentry durch Subject **und** Viewerbewegung,
  reale Spawnadmission, mandatory Removal und aktualisierte Deliveredpose.
  Far-nevertracked sowie ehemals-tracked-far erhalten kein altes Take.
- Tatsächliche foreign Owner, gleiche UUID in anderem Router/Scope/Generation,
  native Detach, echte Rejoinadmission und alter Connectionkey: alte Fakten,
  Move-/Disconnectrequests oder baselines treffen niemals die neue Session.
- Echte gleiche-Lineage-Senderrotation, Suspend/Revalidate; neuer Owner bei
  gleicher Scopezahl; unregister/register; voller/geschlossener Controlkanal
  mit unverändertem Mapping, retained Leave und tatsächlichem Retry.
- Neue Limits jeweils 0/invalid und N-1/N/N+1, checked Scratch/Work und
  Metadata-/Gruppencaps. MAX-1/MAX/Erschöpfung mit echten bounded Channels;
  erschöpfter Join trennt selbst einen schon slowen Viewer **nicht** und
  hinterlässt keinen PlayerJoin oder veränderten pending Leave.
- Gemeinsamer Routerallocator mit zwei echten Iteminstances gleichen Simwire-
  Scalars in verschiedenen Scopes plus Remoteplayer: Decoder IDs kollidieren
  nicht; sämtliche Packetfolger verwenden die tatsächlich erzeugte Zuordnung.
- Fehlende Collector-/Self-/Livebindung bei bestätigtem Partial-/Fullpickup
  behält exakt **die ganze** Producergruppe einschließlich Changed/Remove.
  Wiederholte Refusal erzeugt keine Doppelanimation oder unberechtigten Ack.
- Echter applied Disconnect mit altem reserved oder retained Pickup, dann
  genuine Rejoin: sealed Originalretirement ergibt null Take an Old/New Self,
  aber normale Changed/Remove an echte Itemviewer und vollständigen Ack.
  Busy/Uncertain/Quarantine hält Pins; tatsächlicher Abort/Ack released genau
  einmal. N/N+1-Lifecycle-/Record-/Consumerrefs verweigern vor neuer Mutation;
  pending/stale/foreign/niegebundenes Retirement verweigert. Echter Drain/Stop
  wird nach letzten Gruppen-/Lifecycleack quiescent, ohne Groupdiscard.
- Wirklicher App-ConfigRegistriesproducer, original BiomeZoomSeedhash und
  tatsächliche native Mode-/Posebits: Survival0 statt Creative1, Seedbits und
  tatsächlicher Overworldindex0 stimmen in JoinGame/Stage1-Decoderframes.
  Falsche Mode/Hash/Name/Index/Pose sowie neu unsupported Layouts verweigern.
- Tatsächlicher Appjoin konsumiert exakt ersten Router-JoinGame, Stage1 vor
  Outboundpump; realer Mandatory Writer-/Socketfehler erreicht genuine native
  Detach und UUIDquarantäne. Grouporder bleibt Take→Changed/Remove.

Danach WholeSession Debug/Release, strict Alltargetclippy, package Fmt und
Rustdoc `-Dwarnings`, exakte Source-/Evidencefreeze und unabhängiger Rootreview.
Checks anderer Tickets sind keine Abnahme dieses unimplementierten Vertrags.

Konkrete nächste Entscheidung: Root prüft **diesen einen** Vertrag. Session-
Produktion kann anschließend checked gemeinsame Counter, sealed Sessionkeys,
Lifecycle und tatsächlich admitted Playerbaselines implementieren. Echte
Collectoraktivierung setzt davor/daneben den engen Sim-Liveviewproducer und
den beschriebenen App-Nativejoin/Selfjoin-/Detachpfad samt genuine Retirement-
und Publicationpinproducer voraus. NET-04.2b bindet die **bereits** unabhängig
abgenommene SIM-03.4-Runtime und separat geprüftes Lifecycleaddendum daran.
Keine Behauptung
fertiger Collector-, Survival-, Java-/Paperplugin- oder Performancefunktion.
