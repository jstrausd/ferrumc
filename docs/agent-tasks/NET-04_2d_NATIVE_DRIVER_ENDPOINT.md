# NET-04.2d — konkreter nativer Driver- und Endpointvertrag

Status: **readonly Designvertrag, Runde 4 (Nachbesserung nach Review Runde 2,
Verdikt FINDINGS)**, 11.10.2026. Autor: Design-Subagent Runde 4. Schreibscope
ausschließlich dieser Brief und `.codex-tmp/net04_2d/**`. Keine Produktion,
Dependencies, Locks, Generated- oder historischen Edits, kein Git, keine weiteren
Agents. Dieser Vertrag behauptet **keine** Runtime-, Live-Survival-, Plugin-,
Paper-Differenzial- oder Performanceabnahme.

Grundlage: der eingefrorene Runde-3-Kandidat (Brief `900bc35c…c8db`, Freeze
`6c6ac688…fef0`, unverändert kopiert und gehasht unter
`.codex-tmp/net04_2d/round3-candidate/`), die unabhängige Prüfung Runde 2
`NET-04_2d_NATIVE_DRIVER_ENDPOINT_REVIEW_ROUND2.md` (`9453905f…1f9f`, Verdikt
FINDINGS: N1–N3, R2-A1…A6, R2-H1…H7; `final-assessment.json` `69df514c…4806`),
die Prüfung Runde 1 (B1–B3, A1–A10, H1–H4) und die Root-Vorgaben für Runde 4
(§3). Alle Root-Entscheidungen früherer Runden (G1–G5, L1/L2, B1/B2/B3, R3-1..3)
gelten weiter. Alle Änderungen gegenüber Runde 3 sind in §18 „Änderungen
Runde 4“ Befund für Befund zugeordnet; §17 („Änderungen Runde 3“) bleibt als
Verlauf stehen.

## 1. Basis, Pins und Leseumfang

| Gegenstand | Wert |
|---|---|
| Akzeptierte Basis | git archive `394d4c2794365560bbf7a65fe59c16f2ec20eca9`, 1274 Dateien, Manifest-SHA256 `4c964d711c1dd4499bf914327266134c734f8aa13aefe9010a5dab27d80f519b` |
| Frische Basisprüfung | Pfadmenge gleich, 0 Byteabweichungen, alle Dateien readonly (`.codex-tmp/net04_2d/round4/baseline.verify.json`) |
| SIM-03.4a | CLEAN_RUNTIME; Sourcefreeze `a47209ad…7c96` (56 Dateien), Evidencefreeze `9cfed099…e1eb`; alle 56 Simdateien der Basis bytegleich; Brief `a89b50cd…5d48` |
| NET-04.2c | Brief `3dff58ac5d1b9511862cf050018fb838ea5f8372b8768fd56a661ab949efd04c`, Review `401b7a55…f04c`; 25 Referenzpins frisch identisch |
| Nachbarverträge (frisch gehasht, identisch) | DUR-03.3 `2edfc588…42ab`, DUR-03.3-Review `3a8c0986…80f0d3`, DUR-04.1 `71f26af1…b63`, DUR-04.2 `a655af05…2e8`, DUR-05 `3206b075…a96` |
| Reviews | Runde 1 `7ece2449…0d16` / `e46c5da2…19f0`; Runde 2 `9453905f…1f9f` / `69df514c…4806` |
| Workorder | `NET-04_2d_NATIVE_DRIVER_ENDPOINT_TASK.md` `6919a9b7…5c3ceb5c` |
| Paper | MC 1.21.8 / 772 / Build 60, Checkout-HEAD `29c8822d90899c89d2689338e81a98f690bcba12` |

Vollständig gelesen in Runde 4: Workorder, Runde-3-Kandidat, Review Runde 2 samt
`final-assessment.json`, Review Runde 1, die Pflichtlektüre DUR-04.1, DUR-04.2
und DUR-03.3, SIM-03.4a-Brief (Grenzen), dazu die Root-Entscheidungen im
Runde-3-Thread (nur lesend). Aus Runde 2/3 übernommen: AGENTS, CLAUDE, App-/
Session-/Sim-INVARIANTS, Architektur, ADR-0002/0003/0009 (Kopf), PAPER_REFERENCE,
NET-04.2c samt Review, DUR-05.

Produktionscode wurde ausschließlich aus der Basis gelesen. Jede im Brief
zitierte Codestelle ist in `.codex-tmp/net04_2d/round4/producer-pins.json`
gepinnt (Pfad, Zeilen, Token auf der Startzeile maschinell geprüft,
Datei-SHA256); die 137 Runde-3-Anker sind unverändert enthalten. Zeilen beziehen
sich auf die Basis; „counters.rs“ meint `shard/player_ownership/counters.rs`,
„entity/counters.rs“ `entity/counters.rs` unter `server/crates/ferrumc-sim/src/`.
DUR-03.3-Format7-APIs existieren in der Basis noch nicht; sie werden aus dem
Vertrag zitiert, nicht aus Code.

## 2. Tatsächliche Producer und harte Befunde

1. **Einziger Receiverkonsument.** `server.rs:229-243` registriert den
   einzigen Owner als World-Coverage und gibt nur `into_receiver()` zurück;
   die **Lease wird verworfen**. `server.rs:366-382` bewegt Router, `SimShard`
   und Receiver in denselben `driver::run`-Task (`driver.rs:690-706`).
2. **Native Owner kann diesen Receiver nicht konsumieren.** `SimShard::enqueue`
   verweigert im Ownershipmode mit `OwnershipInputRequired`
   (`shard.rs:680-692`); `drain_shard_inputs` macht jede Enqueue-Refusal
   fatal (`driver.rs:1691-1711`).
3. **Owner-Schutz pro Apply.** `actual_player` prüft Weak-Owner und exakte
   Admission (`player_ownership.rs:601-635`); Refusal gibt den Input zurück und
   erzeugt `Resync` (`337-380`).
4. **Admission.** Restore/Rejoin publizieren sofort `PlayerAdmitted`
   (`780-827`); Bootstrap erst nach echter Completion in `publish_confirmed`
   (`1652-1743`); definitiver Abort gibt die Bootstrapzelle frei (`1787-1816`).
   Rejoin vergleicht Stamp und Record, Restore nicht (`795-806`). Ein
   laufendes Pending desselben Spielers verweigert Rejoin/Restore mit
   `Reserved` (`739`, `755`). Im Lifecycle-Modus reserviert und versiegelt jede
   Restore-/Rejoin-Admission eine eigene Lifecyclezelle (`807-822`).
5. **Native Bewegung** erzeugt `PlayerChanged{handle,generation}`
   (`apply_deferred`, `1976-2029`), außerhalb der Region `Ineligible`
   (`move_owned_player 1876-1917`, `require_player_region 906-921`, Ableitung
   über `EntityTransform::chunk`). Nach Disconnect verweigern Move/Mode/Select
   mit `Ineligible` (`1905`, `1939`, `1969`).
6. **Detach.** `apply_native_disconnect` setzt nur `connected=false` und
   versiegelt den Fact (`lifecycle.rs:178-206`); `PlayerDetached` trägt nur die
   `PlayerId` (`pending.rs:546-549`). Ein detached PlayerState wird nie entfernt
   und zählt gegen `MAX_OWNED_PLAYERS=128` (`player.rs:43`).
   `connected_owned_player_view` verweigert detached Spieler (`lifecycle.rs:59-75`).
7. **Autosave** verlangt weder `dirty` noch `connected`
   (`plan_owned_player_write 1010-1064`), verweigert bei laufendem Pending
   desselben Spielers (`1020`). Bestätigung erzeugt `PlayerChanged` (`1700`).
   Dispositionen in `complete_owned` (`1586-1650`): `Unresolved`/
   `ReceiptAbsent` → Uncertain, Busy/HistoryCapacity → Prepared,
   `Confirmed{Err}` → NeedsRefresh, `Confirmed{Ok}` → Veröffentlichung (`1594`,
   `1630`). **R2-H4:** Im Counter-Modus führt `Refused(CasConflict)` zu
   `Quarantined` (`1610-1614`); jede andere definitive Refusal ruft
   `release_pending` und `apply_deferred` auf, das Pending verschwindet, Sim
   erzeugt `Resync` (`1615-1619`, `1787`). Alle diese Zweige außer
   `Confirmed{Ok}` geben `Err` zurück, die Completion landet in
   `outcome.refusals` (`337-349`).
8. **Counter-Modus (B1).** `initialize_owned_item_counters(limits,
   last_completed)` nur auf leerem Owner (`counters.rs:75-94`) und liefert das
   einzige `OwnedItemCounterHandle` (Weak auf die Owner-Inkarnation,
   `item_counters.rs:180`). Danach verweigert `run_tick_with_player_ownership`
   mit `CounterTickMismatch` (`player_ownership.rs:290-297`); die legacy
   `run_tick`/`run_scheduled_tick` brechen per Assertion ab (`shard.rs:751-758`,
   `767`). Der echte Boundary ist `run_tick_with_player_ownership_at(tick)` mit
   strikt `last+1` (`counters.rs:155-233`, Mismatch `172`, Overflow `170`).
   **R2-H1 (korrigiert):** `counters.rs:174` prüft nur vorab, ob die Generation
   noch ein `next()` erlaubt, und verwirft den Wert; die shardweite lokale
   Generation wächst nur über `next_owned_generation`/`next_generation`
   (`player_ownership.rs:924-928`) bzw. beim Veröffentlichen
   (`player_ownership.rs:1718`), nicht je Boundary. Alle Fehler von `_at`
   entstehen vor jeder Queue- oder Zustandsmutation (`counters.rs:185-204`). Im
   Counter-Modus entstehen keine Legacy-`GameOutput`s (`player_ownership.rs:327-333`).
9. **Recovery-Boundary.** `recover_player_ownership_completions`
   (`counters.rs:237-296`) wendet nur Completion/Reconcile/Retry/Drain/
   RefillItemSaveIds an (`277-294`); alle anderen Inputs (Disconnect, Autosave,
   Stop, Gameplay, Restore/Rejoin/Bootstrap) bleiben mit Position und Charge in
   der Queue. Er claimt keinen Tick, führt keine Countersteps aus, setzt aber die
   Save-Auswahl fort. `Drain` ist auch im Draining-Zustand zulässig (Allow-Liste
   `player_ownership.rs:395-410`) und markiert bei jedem Apply alle Slots
   priorisiert (`567-576`, `entity/counters.rs:262-273`).
10. **Save-IDs.** `select_counter_save` (`counters.rs:1880-1972`) braucht eine
    ID aus dem Pool; leer → höchstens ein `SaveIdsNeeded` je Boundary (`1951`,
    `pending.rs:452-455`). Nachschub nur über
    `OwnedPlayerInput::refill_item_save_ids(owner, ids)` (`pending.rs:134`),
    geprüft gegen Owner, Poolgrenze, lokale Duplikate und Pendings
    (`counters.rs:96-136`).
11. **Stop im Counter-Modus (N2).** Reihenfolge im Normal-Boundary: Inputs
    (Completions zuerst, dann FIFO inkl. Drain/Stop) → Counter-Apply →
    Save-Auswahl (`counters.rs:212-231`). Stop wird gegen den Vorschrittzustand
    geplant: `remaining` zählt **jedes** Nicht-Completion-Input
    (`counters.rs:794-805`), Annahme nur bei `remaining == 0`, leerem Ledger,
    Zellen 0, keinem Pending, sauberen Spielern und allen Items
    `generation == saved && !due` (`896-909`); nur dann unterbleibt der
    Counterschritt (`666-679`). Jeder sonstige Normal-Boundary stept alle
    residenten, nicht fälligen Items und macht sie dirty (`2082-2113`); der
    Counter-Apply ändert `generation`/`due`, nie `saved`, und lässt `priority`
    stehen (`priority |= due`, `entity/counters.rs:161-186`, `counters.rs:1781-1806`).
    Eine Save-Bestätigung setzt `saved`/`saved_tick` und löscht `due`/
    `priority` (`entity/counters.rs:83-107`, `98-105`); die Priorität kommt nur
    zurück, wenn bei der Bestätigung `generation ≠ saved` gilt
    (`counters.rs:400-423`, `entity/counters.rs:254-260`). Die Normal-Lane
    wählt nur bei `tick − saved_tick ≥ save_period` (`counters.rs:1917-1923`).
    Der Stop-Apply prüft erneut `ownership_quiescent` (`player_ownership.rs:577-588`,
    Prädikate `701-720`, `pub(super)`; `is_tick_quiescent` `pub(crate)`,
    `shard.rs:557`). Lokale Generation beginnt je Initialisierung bei 1
    (`pending.rs:626`) und wrappt nie (`player.rs:224-230`, Fehler `player.rs:67-73`).
12. **Router.** Wrapping-Allocator ab 2 (`router.rs:751-757`), Self-ID 1
    (`world.rs:48`), Legacy-Join mit PlayerJoin vor Install (`635-748`), Leave
    vor Mapentfernung mit retained Pending (`1492-1599`), Leave-Order
    TabRemove→EntityRemove (`1622-1650`), Lineageprüfung je Send (`1903-1957`),
    Lane-Defaults 1024/16 (`router.rs:32`, `41`).
13. **App-Join/Teardown.** UUID-Lease vor Storage (`play.rs:68-89`), Legacy-
    Record und Connectioninventar (`95-176`), direkter JoinGame=1 aus JoinKit
    (`join.rs:90-96`), Stage1-Flush vor Chunks (`216`), eine Outboundmessage je
    Flush (`play.rs:494-577`), Teardown Save→Release→Leave-Ack→Quarantäne
    (`617-711`), Legacy-Leave-Save aus Connectioninventar (`669-680`),
    Modespiegel ins Connectioninventar (`1205-1212`), Quarantäne
    (`player_admission.rs:97`).
14. **Storage heute.** App öffnet Format4 (`world.rs:285`, Migration-CLI
    validiert Format4 `cli.rs:176`); Chunkticketfreigabe über den Legacy-Flush
    mit Overlay-/Journalwritern (`driver.rs:1511-1548`,
    `storage_worker.rs:365-400`), die Format7 verweigert (DUR-03.3 „One
    authority for every writer“). Work wird nur extern ausgeführt; ein falscher
    Store gibt die Work intakt zurück (`evidence.rs:329-372`). **R2-H2:** Die
    Klärung eines laufenden Pendings geht über `reconcile(query)` →
    `dispatch_retained(clarify = true)` (`player_ownership.rs:1373-1413`); wie
    `retry` verlangt sie eine freie Lease (`1390-1392`), `retry` zusätzlich den
    Status Prepared (`1393-1395`). `into_receipt_clarification`
    (`evidence.rs:316-323`) wandelt nur eine noch nicht ausgeführte Work um.
15. **Apply-Refusals sind nicht zuordenbar (N3, R2-A5).** `OwnedPlayerInput`
    ist opak, ohne Kind- oder Query-Accessor (`pending.rs:120-121`);
    `OwnedPlayerInputRefusal` liefert nur Grund und Input (`328-341`). Eine
    abgelehnte Completion hält die einzige Lease ihres Pendings, solange das
    Objekt lebt (`evidence.rs:280-284`; Retry/Reconcile → `Reserved`,
    `player_ownership.rs:1390-1392`). Öffentlich lesbar sind dagegen der Status
    je Query (`player_ownership_checkpoint`, `684-698`; `OwnershipPendingCheckpoint`
    mit `query`/`status`, `pending.rs:421-447`), die Pendingzahl
    (`player_ownership_pending_count`, `677-681`) sowie Intents und Work jedes
    Boundaries. Der Driver entscheidet jede Folgeaktion aus diesem Zustand, nie
    aus der Zuordnung einer Refusal (§6.3, §6.5).
16. **Grenzen und Durchsatz.** `inputs ≤ 32` und `≤ 8 MiB` gemeinsam für alle
    Inputarten (`player_ownership.rs:248-286`, Zählprüfung `258-260`;
    `player.rs:51`, `53`), Pending ≤ 32 (`player.rs:47`). Es gibt höchstens
    **eine** Entity-Operation zugleich (`counters.rs:1884-1886`), die
    Priority-Lane erhält 3 von 4 Auswahlturns (`1898`), Cursor und Turn rücken
    vor der ID-Prüfung weiter (`1929-1940`), `save_scan` ≤ 256, Default 64
    (`item_counters.rs:62`, DUR-04.2 Z. 192). Defer-Grenze 1024
    (`item_counters.rs:13`); `CounterRecoveryRequired` entsteht am Defer-Cap
    eines gebundenen Entity-Pendings (`counters.rs:2090-2095`). Der Plan zählt
    `entities + input.len() + 1` gegen `plan_entries` (`counters.rs:470-487`;
    Default/Maximum `65536 + 32 + 1`, `item_counters.rs:17-18`). Die Itemzahl
    ist lesbar über `SimShard::entities()` und `EntityStore::len()`
    (`shard.rs:491-493`, `entity.rs:475-477`).

## 3. Root-Entscheidungen (verbindlich)

Runde 2 (weiter gültig): **G1** eigene native Endpointart (§4); **G2**
sichtbare Regionsgrenze (§9); **G3** additiver Retirementzweig für nie gebundene
Admissions (§8); **G4** Aktivierung hinter DUR-03.3, DUR-03.3a, DUR-05 (§10);
**G5** SIM-03.4b Release vor Live-Aktivierung (§7.5); **L1/L2** getrennt (§7.4).

Runde 3 (weiter gültig): **B1** native Aktivierung nur im Counter-Modus (§6,
§10); **B2** genau ein Producer für Leave-Save und Release in der Session, Driver
nur mit Token (§7); **B3** vollständige G3-Race-Matrix (§14); **R3-1** Stop A/B/C
mit `owned_stop_ready()` (§10.4); **R3-2** Release ohne Caller-Query,
`detached_owned_player_status` rein lesend (§7.5); **R3-3** G3 über
`adopt_unbound_native_admission` mit vorreserviertem Platz (§8).

Runde 4, Root-Vorgaben (umgesetzt):
- **N1** Belegungsgrenze 16 für alle Inputs, die Recovery nicht anwendet,
  feste Reserve 16 für Completion/Reconcile/Retry/Refill/Drain, deterministischer
  Recovery-Austritt mit typisierter Eskalation (§6.1a, §6.2).
- **N2** Stop aus dem Basiscode hergeleitet. **Ergebnis der Prüfung: keine
  zusätzliche Sim-Funktion nötig.** Ein erneutes `drain()` als erstes Input von
  Phase B markiert alle Slots priorisiert; ohne Normal-Boundary gibt es keinen
  Counterschritt, der danach wieder Dirty ohne Priorität erzeugen könnte
  (Invariante P, §10.4). Ab Phase B ist das Einreihen nicht recovery-anwendbarer
  Inputs strukturell gesperrt; in Phase C ist Stop das einzige Input
  (`remaining == 0`). Durchgerechnet für 4096 Items (§10.4,
  `round4/n2_stop_walkthrough.md`).
- **N3** Beim Apply abgelehnte Completions werden nie wieder eingereiht, sondern
  verworfen; Retry bzw. Reconcile folgen aus dem Pendingstatus (§6.3, §6.5).
- **R2-A1…A6** und **R2-H1…H7** siehe §18.

**.2c-Entscheidungen und Ergänzungen (A6).** F1/F2, gemeinsamer checked
Wire-Allocator, SelfJoinGame als erstes FIFO-Paket, exakte Registry-/Mode-/
Pose-/Seedproducer, ursprüngliche Admission, Consumerrefs und alle Count-/Work-/
Scratchgrenzen bleiben unverändert. Folgende .2c-APIs werden geändert; jede ist
als **„.2c-Ergänzung“** markiert und schwächt keine .2c-Regel ab:

| .2c-Original | Ergänzung | Grund |
|---|---|---|
| `retire_native_session(key, fact)` (.2c Z. 540-545) | **.2c-Ergänzung R1:** `retire_detached_native_sessions(&mut \|h\| view)` (§7.2 D5) | Session besitzt das Handle; Fact wird über Closure aus der echten Sim-View gelesen statt vom Caller übergeben (Root G1(d)) |
| Session-Record frei nach Consumerrefs 0 | **.2c-Ergänzung R2:** `acknowledge_retired_native_records(&mut \|h\| view, &mut \|t\| ack)`; Record bleibt bis Release (§7.2 D7/D8) | B2: Record trägt das Original-Handle bis zum Release |
| — | **.2c-Ergänzung G3 (Runde 3 präzisiert):** `adopt_unbound_native_admission` + `retire_unbound_native_admission` (§8) | G3 + R3-3 |
| `admit_native_join(&reservation, admitted, &view, …)` | **.2c-Ergänzung R3:** zusätzlich `&NativeShardReceiver` (§4.3) | G1 Receiverbindung |
| `revalidate_native_endpoint(key, &lane, &view)` (Runde 3, nicht implementierbar ohne Driver-Handle) | **.2c-Ergänzung R4:** `revalidate_native_endpoints(&lane, &mut \|h\| view)` (§4.3) | R2-A2: Session besitzt die Handles, View nur über Closure |

## 4. G1 — native Endpointart und Receiverbindung

### 4.1 Directory und Exklusivität (Root-Auflage a)

`ShardEndpoint` erhält eine private Art: `Legacy{sender: Sender<GameInput>}`
oder `Native{sender: Sender<NativeShardInput>, lane: Arc<NativeLaneIdentity>,
generation: GenerationBinding}`. Beide verwenden dieselbe `ShardDirectory`,
dieselben Leases, Generationen und Lineage-IDs.

- Legacy- und Native-Coverage schließen sich **pro World/Dimension** aus:
  Registrierung verweigert typisiert (`EndpointKindConflict`), sobald irgendeine
  Coverage der anderen Art im selben Scope existiert.
- Legacy-Route auf native Coverage: `SessionError::NativeEndpointRequiresSession`,
  Input intakt zurück. Native Route auf Legacy-Coverage:
  `CollectorSessionError::LegacyEndpoint`. Keine UUID-Übersetzung.
- **H4:** e1 passt `ferrumc-session/INVARIANTS.md` an: Der Router tauscht
  zusätzlich `NativeShardInput` über die native Lane aus; die Aussage „nur
  GameInput/GameOutput/ClientboundPlayPacket“ gilt dann nur für Legacy-Coverage.

### 4.2 Typen (Session, NET-04.2e1)

```rust
pub struct NativeShardRegistration { /* home, lease, receiver */ }
impl NativeShardRegistration {
    pub fn shard_id(&self) -> ShardId;
    pub fn lease(&self) -> ShardLease;
    pub fn into_parts(self) -> (ShardId, ShardLease, NativeShardReceiver);
}
/// Nicht Clone, kein public constructor; Identität = Arc pro Generation.
pub struct NativeShardReceiver { /* identity, mpsc::Receiver<NativeShardInput> */ }
impl NativeShardReceiver {
    pub fn try_recv(&mut self) -> Result<NativeShardInput, NativeLaneRecv>;
}
/// Nur Session konstruiert: Sessionkey + aus dem gespeicherten echten Handle gebauter Input.
pub struct NativeShardInput { /* key, OwnedPlayerInput, lane class */ }
impl NativeShardInput {
    pub fn session(&self) -> &PlayerSessionKey;
    pub fn class(&self) -> NativeInputClass; // Data | Control
    /// R2-A5: der Key bleibt beim Driver, falls Sim das Enqueue verweigert.
    pub fn into_parts(self) -> (PlayerSessionKey, NativeInputClass, OwnedPlayerInput);
}
impl SessionRouter {
    pub fn register_native_world_shard(&mut self, home: ShardId, generation: GenerationBinding)
        -> Result<NativeShardRegistration, CollectorSessionError>;
    pub fn replace_native_shard_registration(&mut self, lease: &ShardLease, current: &NativeShardReceiver)
        -> Result<NativeShardRegistration, CollectorSessionError>;
    pub fn unregister_native_shard_registration(&mut self, lease: &ShardLease, receiver: NativeShardReceiver)
        -> Result<ShardId, CollectorSessionError>;
    /// .2c-Ergänzung R4 (§4.3).
    pub fn revalidate_native_endpoints(&mut self, current: &NativeShardReceiver,
        view: &mut dyn FnMut(&OwnedPlayerHandle) -> Option<ConnectedOwnedPlayerView<'_>>)
        -> Result<NativeRevalidation, CollectorSessionError>;
    /// R2-A5: entscheidet, ob ein gehaltener Lane-Input noch zugestellt werden darf.
    pub fn native_lane_input_live(&self, key: &PlayerSessionKey, class: NativeInputClass) -> bool;
}
```

`generation` muss der in `initialize_collector_sessions` validierte
Layout-/Generationscope sein (.2c FlatOverworld1_21_8); sonst `Unsupported`.
`PlayerSessionKey` ist Routingdatum ohne Autorität; der Driver darf ihn halten.

### 4.3 Bindung Receiver ↔ Sim-Owner

Der reale Hook ist die Single-Task-Ownership von `driver::run`. DUR-05a macht
daraus einen **App-privaten** Typ:

```rust
struct NativeShardSlot {                   // ferrumc-app, nicht pub
    shard: SimShard,                        // nativ + Counter-Modus initialisiert (§10.2)
    counter_owner: OwnedItemCounterHandle,  // nur aus initialize_owned_item_counters
    clock: NativeTickClock,                 // letzter abgeschlossener logischer Tick (§6.1)
    lane: NativeShardReceiver,              // nie herausgegeben, nie Clone
    draining: Option<NativeShardReceiver>,  // alter Receiver nach Replacement
    lease: ShardLease,                      // nicht mehr verworfen
    held: Option<HeldLaneInput>,            // höchstens ein von Sim beim Enqueue abgelehnter Lane-Input (R2-A5)
    gate: NonRecoveryGate,                  // Belegung nicht recovery-anwendbarer Inputs + Phase (N1/N2, §6.2)
    recovery: RecoveryState,                // Normal | Recovery{…} | Degraded{…} (N1, §6.1a)
    outstanding: OutstandingWork,           // ≤32 Queries mit Lease-Zustand (N3, §6.5)
    held_completions: CompletionFifo,       // ≤32, NUR Enqueue-Refusals (DUR-04.1 Z. 563-565)
    tokens: NativeRetentionTokens,          // L2-Tabelle, beim Start auf das Sessions-Limit vorreserviert (R2-A3)
    phase: NativeShardPhase,                // Running | StopA | StopB | StopC (§10.4)
}
struct HeldLaneInput { key: PlayerSessionKey, class: NativeInputClass, input: OwnedPlayerInput }
```

Kein Accessor gibt `lane`, `draining` oder `counter_owner` heraus. Session prüft
bei jeder Bindung die **Borrow des aktuellen Receivers** (Identität = aktuelle
Registrierung und exakte Lease-Generation) zusammen mit einer echten
`ConnectedOwnedPlayerView` desselben Handles. Sim prüft bei jedem Apply erneut
Weak-Owner und Admission (§2.3). Callerbool, Hash, Label oder Shardnummer
werden nirgends akzeptiert.

- **Erstbindung:** ausschließlich `admit_native_join(&reservation, &lane,
  &handle, &view, join_game, name, equipment)` (.2c-Ergänzung R3). Erfolg setzt
  die Session **Active** und `ever_active = true` (**A2**): ab diesem Moment
  gilt für die Session ausschließlich der normale Leave (§7), nie G3.
- **Replacement (gleiche Lineage):** verlangt aktuelle Lease **und** Borrow des
  aktuellen Receivers; neue Generation und Lane-Identität, gleiche
  registration_id/home. Alle nativen Sessions werden `EndpointSuspended`. Der
  Driver legt den alten Receiver in `draining` und leert ihn vollständig und
  vor dem neuen; alte Items behalten ihre eigenen Handles.
- **Revalidierung (A3, R2-A2, .2c-Ergänzung R4):** Nach dem Replacement ruft
  der Driver einmal je Iteration, bis der Bericht keine offene Session mehr
  meldet,
  `router.revalidate_native_endpoints(&slot.lane, &mut |h| slot.shard.connected_owned_player_view(h).ok())`.
  Die Session iteriert über ihre nativen Sessions dieser Registrierung im
  Zustand Active **oder** Leaving vor dem DetachFact (≤ Sessions-Limit
  Closure-Aufrufe je Aufruf), ruft die Closure mit dem **gespeicherten** Handle
  auf und prüft die gelieferte View gegen Handle, volle Generation und
  Owner-Lineage der Registrierung. Passt sie, wird die Session an die Identität
  von `current` gebunden, `EndpointSuspended` aufgehoben und ein retained
  Disconnect als erstes Lane-Item gesendet. Passt sie nicht, folgt typisiert
  `NativeEndpointRevalidationRefused` und ein kontrollierter Session-Leave.
  Liefert die Closure `None` (Sim-Spieler detached), braucht die Session keine
  Lane mehr: Leave-Save und Release laufen über Session-Closures (§7), sie bleibt
  suspendiert, ohne Fortschritt zu blockieren. Der Driver hält dabei kein
  Handle; die View lebt nur im Closure-Aufruf.
- **Routing während Suspension (A3):** Data-Events → typisiert
  `EndpointSuspended`, nicht gesendet (Coalescible verworfen, Authoritative →
  nach Revalidierung ein mandatory `SynchronizePlayerPosition` auf die echte
  Sim-Pose, §6.3). Disconnect (Control) wird in der Session dedupliziert
  **retained** und nach Revalidierung als erstes Lane-Item der Session
  gesendet. Ist keine Revalidierung möglich (Shard gestoppt), greift §10.4.
- **Unregister/Register:** `unregister_native_shard_registration` konsumiert den
  Receiver und verweigert `NativeEndpointInUse`, solange irgendeine Session,
  provisorische Reservation, Retention-Record (§7.1) oder adoptierte Admission
  (§8) diese registration_id referenziert.

### 4.4 Lane, Credits und Cancel (Root-Auflagen b, c, d)

- **(b) strukturell:** Native Startup ruft nie `register_world_owner` und nie
  `driver::run`. `driver::run_native(router, slot, …)` hat keinen
  `Receiver<GameInput>`-Parameter; `drain_shard_inputs`, `run_tick`,
  `run_tick_with_player_ownership` und Schedulerpfade liegen außerhalb des
  nativen Moduls und werden dort nie referenziert (Source-Scan, §4.5).
- **(c) bounded:** `mpsc::channel(native_lane_capacity)`, NonZero, ≤1024
  (= `DEFAULT_SHARD_INPUT_CAPACITY`, H2), Kontrollreserve 16 (=
  `DEFAULT_SHARD_CONTROL_RESERVE`, H2). Router ist alleiniger Sender; Data stoppt
  vor der Reserve, Control darf sie nutzen. Voll: bestehende Policy
  (Authoritative/Coalescible → kontrollierter Session-Leave; Disconnect →
  dedupliziert retained, Retry je Boundary). Der Driver entnimmt Lane-Items nur
  im Lane-Fenster der Iteration (§6.2), nur im Normal-Modus, nur solange das
  Gate offen ist und Platz meldet (`lane ≤ 16 − G`).
- **Gehaltener Lane-Input (R2-A5):** Der Driver zerlegt jedes Lane-Item mit
  `into_parts()` und behält Key und Klasse, während er `enqueue_owned_player_input`
  aufruft. Verweigert Sim das **Enqueue** (Count/Bytes/Allocation), gibt Sim nur
  das `OwnedPlayerInput` zurück; der Driver bildet
  `held = Some(HeldLaneInput{key, class, input})`. Vor jedem weiteren `try_recv`
  wird `held` zuerst erneut versucht, so bleibt FIFO je Session erhalten.
  Meldet `router.native_lane_input_live(&key, class)` `false` (Data einer nicht
  mehr aktiven Session, Control einer bereits detachten Session), verwirft der
  Driver `held` mit Metrik `native_lane_held_dropped`. Höchstens
  **Lanekapazität + 1** Items unterwegs.
- **Cancel:** Eine Leaving/retired Session sendet keine neuen Data-Items;
  bereits eingereihte werden von Sim mit `Ineligible("detached …")`/
  `StaleHandle` verweigert und vom Driver verworfen (§6.3). Disconnect ist
  idempotent.

### 4.5 Konstruktionsrechte (Root-Auflage d, B1, B2, N2, N3)

| Konstruktor `OwnedPlayerInput::*` | Einziger Producer | Quelle |
|---|---|---|
| `move_player`, `set_game_mode`, `select_hotbar` | Session (aktive Session) | gespeichertes Handle + Sessionkey |
| `disconnect` | Session (aktive Session über Lane; adoptierte Admission über §8) | gespeichertes Handle |
| `autosave` (Leave-Save) | Session, nur für Retention-Records (§7.3) | gespeichertes Original-Handle + Driver-ID |
| `restore`, `rejoin`, `bootstrap` | App-Driver | echter `OwnedPlayerRead` + Driver-ID (§6.4) |
| `complete` | App-Driver | echte `PlayerOwnershipCompletion` |
| `retry`, `reconcile` | App-Driver | Query aus `outstanding` (§6.5) |
| `drain`, `stop` | App-Driver | Stop-Protokoll (§10.4) |
| `refill_item_save_ids` | App-Driver (DUR-05a) | ausschließlich `slot.counter_owner.clone()` + Driver-IDs (§6.4) |
| alle übrigen (`drop_selected`, `pickup`, `normal_slot_click`, `save_item`, `restore_item`, `spawn_item`) | im Nativpfad keiner bis DUR-05b | — |

`SimShard::release_detached_owned_player` wird ausschließlich innerhalb des
Closures aufgerufen, den der Driver an `router.release_native_retention`
übergibt (§7.5); das Handle erreicht den Driver nur als Closure-Argument.

**Source-Scan-Test (NET-04.0b-Muster, angepasst):** im Crate `ferrumc-app`
(1) keine `OwnedPlayerInput::*`-Konstruktoren außer `restore`, `rejoin`,
`bootstrap`, `complete`, `retry`, `reconcile`, `drain`, `stop`,
`refill_item_save_ids`; (2) `refill_item_save_ids` genau an einer Stelle im
nativen Driver-Modul, erstes Argument textuell `self.counter_owner.clone()`
bzw. `slot.counter_owner.clone()`; (3) `release_detached_owned_player` genau an
einer Stelle, innerhalb eines `release_native_retention(`-Aufrufs; (4) kein
Struct-/Enum-Feld und keine Collection mit `OwnedPlayerHandle` (nur Borrows in
Signaturen und Closure-Parametern); (5) kein Aufruf von `run_tick(`,
`run_tick_with_player_ownership(` oder `drain_shard_inputs` im nativen Modul;
**(6) N1/N2:** `enqueue_owned_player_input` genau in zwei Funktionen des nativen
Moduls, `enqueue_recovery_class` und `enqueue_gated`; `enqueue_gated` verlangt
ein `GateTicket`, das nur `NonRecoveryGate::admit` bzw. `NonRecoveryGate::stop_ticket`
erzeugt; `stop()` wird nur über `enqueue_gated` eingereiht;
**(7) N3:** das Ergebnis von `take_refusals()` geht ausschließlich an
`dispose_apply_refusals`, und dort ist kein `enqueue_`-Aufruf erreichbar;
**(8) N3:** `held_completions` wird nur im `Err`-Zweig von
`enqueue_recovery_class` befüllt.
`OwnedItemCounterHandle` entsteht nur aus `initialize_owned_item_counters`
(`counters.rs:75-94`); ein Spieler-Handle kann es nicht ersetzen (anderer Typ).

## 5. Nativer Join: Schritte, Eigentum und Rollback

App-Kommandos: `SimCommand::NativeReadPermit{player, reply}`,
`SimCommand::NativeJoin{permit, read, name, lease: PlayerReservation, reply}`,
`SimCommand::NativeLeave{session_or_permit, lease, reply}`. Die Connection kennt
nie Handle, Shard oder Store-Mutationsrechte.

| # | Schritt (Producer) | Eigentümer danach | Refusal / Rollback |
|---|---|---|---|
| 1 | Connection `PlayerAdmissions::reserve` (L1) | Connection | `AlreadyReserved/Quarantined/AtCapacity` → `PlayDisconnect`, keine Storage-/Sim-Wirkung |
| 2 | Driver `NativeReadPermit` (§7.4): Rejoin-Permit bei bestehendem L2-Token, sonst Restore-Permit | Driver (Permittabelle) | offener Permit / globale Cap / Shutdown / `NativeShardDegraded` → typisiert, L1 bleibt bei Connection |
| 3 | Connection `OwnershipStoreBinding::read_player` (async, Blocking-Pool) | Connection hält sealed `OwnedPlayerRead` | Lese-/Scopefehler → Abbruch, Permit verfällt, L1 frei |
| 4 | Connection: deklarierte Position aus `NativePlayerSnapshot::try_from_record` bzw. Spawn bei echtem Miss; Regionsvorprüfung (§9) | Connection | außerhalb → `NativeAdmissionOutsideRegion`, keine Simwirkung |
| 5 | `NativeJoin` an Driver; L1 **wandert in den Driver** | Driver | Reply bereits geschlossen → Permit/L1 freigeben, keine Simwirkung |
| 6 | Driver: Permitprüfung, `router.reserve_native_join(player, generation, position)` (.2c, alle Count-/Counter-/Wire-Refusals vor Sideeffects). Die Reservation alloziert fallibel den Session-Tabelleneintrag, der später die aktive Session oder den adoptierten Leaving-Record trägt; er zählt in das Sessions-Limit (R3-3) | Driver hält `NativeJoinReservation` (`PendingNativeJoin`) | Refusal (inkl. `AllocationFailed`) → Reply typisiert, L1 zurück |
| 7 | Driver baut aus dem Read `restore`/`rejoin` bzw. `bootstrap(read, spawn, [0,0], id)` mit Driver-ID (§6.4) und reiht ihn **gated** ein (§6.2: höchstens 2 Join-Inputs in der Sim-Queue, nur im Normal-Modus und bei offenem Gate); sonst wartet `PendingNativeJoin` in Driver-FIFO (≤ Sessions-Limit) | Sim (Input), Driver (Reservation, L1, Reply) | Enqueue-Refusal `Capacity`/`AllocationFailed` → Input intakt zurück an den Kopf des Driver-FIFO, nächste Iteration erneut; `Draining` oder anderer Grund → `cancel_native_join` (gibt Tabelleneintrag deterministisch frei), Reply, L1 zurück |
| 8 | Boundary: Restore/Rejoin → `PlayerAdmitted`; Bootstrap → Work → Completion → `PlayerAdmitted` | Sim (Admission, Lifecyclezelle) | **Apply-Refusal (N3, §2.15):** nach einem erfolgreichen Normal-Boundary ohne `PlayerAdmitted` dieser PlayerId (Restore/Rejoin) bzw. ohne Work mit der Bootstrap-ID (`work.query().id()`) war der Join abgelehnt → `cancel_native_join`, Reply; das abgelehnte Input ist bereits verworfen (§6.3). Definitiver Bootstrap-Abort gibt die Sim-Zelle frei, dann gleich |
| 9 | Driver im selben synchronen Schritt wie das Intent: Connection erreichbar → `connected_owned_player_view(&handle)`, `admit_native_join(…)`; sonst oder bei Refusal → `adopt_unbound_native_admission` (§8) | Session (Admission + Handle) | Das Handle wird in beiden Zweigen an die Session übergeben; der Driver speichert es nie |
| 10 | Driver liest im selben Schritt `owned_player_slots/carried`, Selected, Mode, kodiert Window-0-Inhalt, Held-Slot, Abilities; Reply `{PlayerSessionHandle, NativeJoinKit, L1}` | Connection | Reply-Send scheitert → Session ist bereits Active (A2) → normaler Leave §7 mit L1 im Driver (D1), nicht G3 |
| 11 | Connection: **erste** Outboundmessage exakt der Router-JoinGame → Stage1; danach Self-Tabadd, GameEvent13, Center, Default-Spawn, Commands, Spawn-Sync via `PendingTeleport::issue` (erste ID = 1), Abilities/Container/Held-Slot aus `NativeJoinKit`; Stage1-Flush | Connection | anderer erster Typ → Invariantenfehler → Teardown (§7) |
| 12 | Stage2: Spawnchunks `desired ∩ Region` über `acquire_owned_chunk` (§10.3) | Connection hält bestätigte Tickets | Ticketuncertainty → Quarantäne wie heute |

Ein `_at`-Fehler lässt eingereihte Join-Inputs unverändert in der Queue
(§2.8); entschieden wird erst nach dem nächsten erfolgreichen Normal-Boundary.

`JoinGame` wird aus dem echten Native-Mode gebaut (Survival 0 bei frischem
Bootstrap). **Keine zweite Inventarautorität (A4):** Im Nativpfad entfallen
Connection-`PlayerInventory`, Creative-JoinKit, `SimCommand::SetGameMode` nach
Join, `mirror_game_mode_change` (`play.rs:1205-1212`) und der Legacy-Leave-Save
aus dem Connectioninventar (`play.rs:669-680`); Inventar, Mode und Selection
liest die Connection nur aus `NativeJoinKit` und späteren Router-Paketen.
**Credit-Retention:** Reservation, Tabelleneintrag, Lifecyclezelle und L1
haben in jedem Schritt genau einen Eigentümer; ein Fehler der provisorischen
Netzwerkreservation löscht nie eine admitted Sim-Admission oder pending
Inventory-Mutation.

## 6. Driver-Boundary im Counter-Modus (B1) und native Ein-/Ausgabe

### 6.1 Tick-Identität und Boundary-Arten

`NativeTickClock{last: Tick}` gehört dem Slot. Start: `Tick::ZERO`, übergeben an
`initialize_owned_item_counters(limits, Tick::ZERO)` (DUR-04.2 Z. 225-231).
Ticks werden nicht persistiert; jeder Start beginnt bei 0.

Jede Driver-Iteration (Wall-Clock-Takt wie heute, 50 ms) führt **genau einen**
Boundary aus:

- **Normal:** `slot.shard.run_tick_with_player_ownership_at(clock.last.next())`.
  Nur bei `Ok` wird `clock.last` fortgeschrieben. Die Legacy-Methode
  `run_tick_with_player_ownership` ist im nativen Modul nicht referenziert
  (§4.5), ebenso `run_tick` (Assertion `shard.rs:751-758`).
- **Recovery:** `slot.shard.recover_player_ownership_completions()`; claimt
  keinen Tick, zählt keinen Gameplaytick, kein Catch-up (DUR-05 Z. 42).

Disposition eines Normal-Boundary-Fehlers (alle vor Mutation, Queue intakt,
`counters.rs:185-204`):

| Fehler | Bedeutung | Driver |
|---|---|---|
| `CounterTickMismatch` | Driver hat einen falschen Tick gereicht | Invariantenbruch → Fail-Stop des Drivers (§10.4 Ergebnis `Unresolved`) |
| `CounterRecoveryRequired`, `Capacity("counter plan…")`, `Capacity("future item slots")`, `AllocationFailed` | DUR-04.2 completion-only Recovery nötig | **Recovery-Modus** (§6.1a) |
| `GenerationExhausted`, `CounterTickOverflow` | terminal | `Degraded{terminal}`: nur Recovery-Boundaries, keine Normal-Versuche, keine Joins; Stop direkt ab Phase B, Ergebnis `Unresolved` (§10.4); dokumentierte Grenze §15 |
| `Draining` (nach Stopped) | Shard gestoppt | Driver endet regulär |
| andere `PlayerOwnershipError` | Owner/Scope/Snapshot | Invariantenbruch → Fail-Stop |

### 6.1a Recovery-Modus mit deterministischem Austritt (N1)

- **Je Iteration genau ein Boundary.** Ein **Normal-Versuch** mit demselben Tick
  läuft, wenn (i) seit dem letzten fehlgeschlagenen Versuch Fortschritt
  beobachtet wurde — `player_ownership_pending_count()` ist gesunken
  (`player_ownership.rs:677-681`) oder der Checkpoint einer eingereihten
  Completion ist verschwunden (§6.5) —, oder (ii) **spätestens nach K = 20
  Recovery-Boundaries** seit dem letzten Normal-Versuch, unabhängig von jeder
  Completion. Sonst läuft ein Recovery-Boundary.
- **Intake:** nur Klasse 1 (§6.2: Completion, Retry, Reconcile, Refill, Drain).
  Das Gate nimmt keine neuen nicht recovery-anwendbaren Inputs an; ihre
  Belegung `G` bleibt unverändert ≤ 16.
- **Austritt:** der erste `Ok` eines Normal-Versuchs beendet den Modus;
  `G := 0` (der Normal-Boundary hat die ganze Queue angewandt,
  `player_ownership.rs:299-335`).
- **Typisierte Eskalation:** nach **F_max = 60** aufeinanderfolgenden
  fehlgeschlagenen Normal-Versuchen (bei reiner K-Taktung 1200 Recovery-Boundaries
  ≈ 60 s) geht der
  Slot in `Degraded{reason, since}`: Metrik `native_shard_degraded{reason}`,
  Log-Warnung, neue Read-Permits und Joins typisiert `NativeShardDegraded`.
  Der Slot macht weiter K-getaktete Normal-Versuche und verlässt `Degraded`
  beim ersten `Ok`. Ein Stop in `Degraded` läuft nach §10.4 und endet typisiert.
- **Terminierung je Ursache:**
  - `CounterRecoveryRequired` verschwindet genau dann, wenn das Entity-Pending
    mit vollem Defer-Ledger aufgelöst ist (`counters.rs:2090-2095`). Dessen
    Completion, Retry oder Reconcile passt immer in die Reserve (§6.2) und wird
    vom nächsten Boundary angewandt; der nächste Normal-Versuch folgt dann über
    Regel (i), sonst spätestens nach K Boundaries. Bleibt Storage dauerhaft
    unklar, bleibt das Pending → nach F_max `Degraded`.
  - `Capacity("counter plan")`: `needed = entities + input.len() + 1`
    (`counters.rs:470-487`). Nach jedem Recovery-Boundary liegen höchstens die
    `G ≤ 16` gated Inputs in der Queue. Bleibt `needed > plan_entries`
    (Betreiber hat `plan_entries` gesenkt), ist das eine dauerhafte
    Konfigurationsgrenze → nach F_max `Degraded`, kein blindes Kreisen.
  - `AllocationFailed`/`Capacity("future item slots")`: transient bzw.
    praktisch unerreichbar; K-getaktete Wiederholung.
  - Jede Iteration ist begrenzt (ein Boundary, ≤ 32 Inputs, ≤ 32
    Checkpoint-Lesungen); der Modus endet also entweder mit `Ok`
    (≤ K + 1 Iterationen nach Wegfall der Ursache) oder bleibt sichtbar
    typisiert `Degraded`, bis ein Stop typisiert endet.

K = 20 und F_max = 60 sind gewählt, **nicht gemessen**.

**Praktische Erreichbarkeit der terminalen Grenze (Root R3-1, R2-H1):** Tick ist
u64 mit strikt +1 je Normal-Boundary; bei 20 Boundaries/s sind 2^64 Ticks ≈
2,9·10^10 Jahre. Die shardweite lokale Generation wächst nur bei
Veröffentlichungen, Dispatches und Admissions (`player_ownership.rs:924-928`,
`1718`), je Boundary höchstens um die Zahl der angewandten Inputs (≤32) plus
Deferred-Veröffentlichungen (≤128 Spieler) plus eine Save-Admission; selbst bei
200 Schritten × 20/s ≈ 1,5·10^8 Jahre (konservativ). Beide Werte werden **nicht
persistiert**: Jeder Start initialisiert Tick 0 und Generation 1
(`pending.rs:626`), neue lokale Handles (DUR-04.2 Z. 132-134). Ein beschädigter
Durable-Zustand kann daher keine fast erschöpften Tick-/Generationswerte
einschleusen. Persistierte Revisionen nahe MAX verweigert Storage typisiert
(DUR-03.3 Refusal-Order „Revision/Sequence exhaustion“, Reopen-Validierung);
der Driver behandelt das als definitive Completion-Refusal eines einzelnen
Requests, nicht als Shard-Terminalfall. Die terminale Grenze bleibt typisiert
spezifiziert, nicht als Abnahme behauptet.

### 6.2 Sim-Input-Belegung, Reserve und Priorität (A1, N1)

`enqueue_owned_player_input` prüft eine gemeinsame Grenze für alle Inputarten
(`inputs ≤ 32`, `≤ 8 MiB`, `player_ownership.rs:248-286`). Native Konfiguration
verlangt `inputs = 32` (`MAX_PLAYER_OWNERSHIP_INPUTS`) und `input_bytes = 8 MiB`.

**Zwei Klassen nach Recovery-Anwendbarkeit** (`counters.rs:277-294`):
- **Klasse 1, recovery-anwendbar:** Completion, Retry, Reconcile,
  RefillItemSaveIds, Drain. Jeder Boundary (Normal wie Recovery) wendet sie an
  und entfernt sie aus der Queue.
- **Gated, nicht recovery-anwendbar:** Lane-Items (Move/Mode/Select/
  Disconnect), Join-Inputs (Restore/Rejoin/Bootstrap), Session-gebaute
  Retention-Submits (Leave-Save-Autosave, G3-Disconnect) und Stop. Nur ein
  erfolgreicher Normal-Boundary wendet sie an.

**Feste Zahlen (gewählt, nicht gemessen):**

| Größe | Wert | Durchsetzung |
|---|---|---|
| Belegung `G` gated Inputs in der Sim-Queue | **≤ 16** zu jedem Zeitpunkt | `NonRecoveryGate::admit` |
| Reserve für Klasse 1 | **32 − 16 = 16** Plätze, nie unterschritten | folgt aus `G ≤ 16` |
| Retention-Submits | ≤ 4 je Iteration | Gate |
| Join-Inputs in der Sim-Queue | ≤ 2 | Gate |
| Lane-Items | ≤ 16 − G je Iteration (≥ 10 im Normal-Modus, da Retention + Join ≤ 6) | Gate |
| Stop | nur in Phase C, als einziges Input (`G = 0` davor) | `NonRecoveryGate::stop_ticket` |
| Gated Inputs in Recovery-Modus, Phase B, Phase C (außer Stop) | 0 neue | Gate geschlossen |

**Belegungsbuchhaltung:** `G` steigt um 1 je erfolgreich eingereihtem gated
Input, wird nach jedem erfolgreichen Normal-Boundary 0 (ganze Queue angewandt,
`player_ownership.rs:299-335`) und bleibt nach Recovery-Boundaries und nach
`_at`-Fehlern unverändert (dort bleiben gated Inputs liegen). Weil Klasse-1-
Inputs von jedem Boundary verbraucht werden, sind zu Beginn jeder Iteration
mindestens 16 Plätze frei oder mit Klasse-1-Inputs belegt, die genau dieser
Boundary anwendet. **Nicht recovery-anwendbare Inputs können die Recovery damit
nie blockieren** (Review-Szenario N1 a ist ausgeschlossen).

**Bytes:** Ein Join-Input belastet `Recordlänge + 512` (`pending.rs:280-323`),
Recordlänge ≤ 1 MiB (DUR-04.1 §6), also ≤ 1 049 088 Bytes; Lane-, Retention-
und Stop-Inputs je 512. Gated höchstens `2 × 1 049 088 + 14 × 512 = 2 105 344`
Bytes. Für Klasse 1 bleiben ≥ `8 388 608 − 2 105 344 = 6 283 264` Bytes; eine
maximale Completion belastet `512 + 1 MiB + 256 KiB = 1 311 232` Bytes
(`evidence.rs:290-309`; Player ≤ 1 MiB, Item ≤ 256 KiB), es passen also
stets mindestens 4 maximale Completions. Retry/Reconcile/Drain belasten 512,
ein Refill `32 × 16 + 512 = 1024`.

**Reihenfolge je Iteration (vor dem Boundary):**
1. **Klasse 1:** zuerst Refill (≤1, §6.4) und Drain (Stop-Protokoll), dann
   `held_completions` (Driver-FIFO, ≤32, nur Enqueue-Refusals, Lease bleibt beim
   Driver, DUR-04.1 Z. 563-565), dann `try_recv` vom Completion-Kanal, dann
   Retry/Reconcile aus `outstanding` (§6.5) — bis Sim verweigert. Eine beim
   **Enqueue** verweigerte Completion kommt intakt an das Ende von
   `held_completions`.
2. **Gated Klasse 2** (nur Normal-Modus, Gate offen): Retention-Submits (≤4),
   Join-Inputs (Queue ≤2).
3. **Gated Klasse 3** (nur Normal-Modus, Gate offen): Lane ≤ 16 − G.

**Verhungern ausgeschlossen:** Ein erfolgreicher Normal-Boundary leert die
Sim-Queue vollständig; in der nächsten Iteration stehen alle verfügbaren
Completions vor jedem gated Input. Leave-Save-Inputs laufen als gated Klasse 2
vor den Lane-Items. Pending ≤ 32, also höchstens 32 Completions; im
ungünstigsten Fall warten 16 eine Iteration.

**Vereinbarkeit mit den Nachbargrenzen:** DUR-04.1/DUR-04.2 bleiben unverändert
(32 Inputs, 8 MiB, 32 Pendings, eine Entity-Operation, DUR-04.2 Z. 188-195).
`plan_entries` (Default `65536 + 32 + 1`) deckt jede Queuebelegung ≤ 32.
SIM-03.4a: Lifecyclezellen ≤ 128 Spieler + 256 Publicationgruppen (SIM-03.4a
Z. 25-27) werden bei jeder Admission geprüft, eine Refusal führt zu
`cancel_native_join`; Lifecycle-Acks (D7/D8) und Releases sind direkte
Aufrufe zwischen Boundaries, keine Queue-Inputs; Publicationpins entstehen nur
mit Completions. Keine SIM-03.4a- oder DUR-04.2-Grenze wird geändert.

### 6.3 Intents und Refusals nach jedem Boundary

Verarbeitung in Intent-Reihenfolge:
- `PlayerAdmitted` → §5 Schritt 9 bzw. §8.
- `PlayerChanged{handle}` / `Resync{handle}`: Driver versucht
  `connected_owned_player_view(&handle)`. Mit View → `router.native_player_changed(&view)`
  bzw. `router.native_resync(&view)` (mandatory `SynchronizePlayerPosition` auf
  die echte Pose). **Ohne View (detached, A5):** kein Fehler, kein Driverstopp;
  `router.native_detached_intent(&handle)` übergibt das Handle transient an die
  Session, die es gegen ihre Retention-Records vergleicht (Owner, UUID,
  Admission) und als Retry-Trigger für den Leave-Save nutzt (§7.3). Kein
  Treffer → Metrik `native_stale_intent`, verworfen.
- `PlayerDetached{player}`: nur Trigger, nie Autorität: Driver ruft
  `router.native_detached(player)` → Session prüft im Closure die echte
  Lifecycle-View (§7.2 D3).
- `SaveIdsNeeded` → §6.4. `ItemSaveBlocked{item, reason}` → Metrik; Dirty bleibt.
- `Draining`/`Stopped` → §10.4.
- `ItemSpawned/Changed/Removed/Take/Stat/ItemSpawnRefused` treten im Nativpfad
  vor DUR-05b nicht auf (keine Drop/Pickup-Producer); NET-04.2b übernimmt sie.

**Apply-Refusals (`outcome.take_refusals()`, N3):** Jede wird nach Metrik
`native_apply_refusal{reason}` in `dispose_apply_refusals` **verworfen und nie
erneut eingereiht**. Das Verwerfen gibt bei Completions die Lease frei
(DUR-04.1 Z. 475-477, DUR-04.2 Z. 606-609). Die Folgeaktion bestimmt der Driver
ausschließlich aus Sim-Zustand (§2.15):
- Completion, Retry, Reconcile → Auswertung von `outstanding` über
  `player_ownership_checkpoint` (§6.5);
- Join-Input → §5 Schritt 8 (fehlendes `PlayerAdmitted` bzw. fehlende Work);
- Refill → §6.4 (erneutes `SaveIdsNeeded`);
- Session-gebaute Retention-Inputs → die Session liest das Ergebnis aus dem
  Status (§7.3), die ID bleibt unverbraucht;
- Lane-Data/Disconnect → Sim hat für einen noch besessenen Spieler bereits
  `Resync` erzeugt (`player_ownership.rs:351-380`); für detached Sessions nichts.
  Ein nicht angewandter Disconnect wird über den Status erkannt und erneut
  gesendet (§7.3 Punkt 1).

**Enqueue-Refusals** (synchroner `Err` von `enqueue_owned_player_input`) sind
davon getrennt: Completion → `held_completions` (Lease beim Driver); Lane →
`held` mit Key (§4.4); Retention-Submit → `native_retention_unsent`; Join →
zurück in das Driver-FIFO bzw. `cancel_native_join` (§5 Schritt 7); Refill →
unsent, nächster Versuch bei `SaveIdsNeeded`. Ein beim Enqueue verweigertes
gated Input erhöht `G` nicht.

### 6.4 Save-IDs aus der Driver-UUID-Autorität

`NativeOperationIds` (ferrumc-app, DUR-05a) erzeugt jede `OwnershipTransferId`
für Bootstrap, Leave-Save und Item-Save-Pool aus `uuid::Uuid::new_v4()`
(bestehende App-Dependency, `operators.rs:359`). Globale Eindeutigkeit
entscheidet die echte Storage-Receipt-ID/SHA-Prüfung (DUR-04.2 Z. 557-558);
eine Kollision wird zur definitiven Refusal des einzelnen Requests.

- **Init:** direkt nach `initialize_owned_item_counters` ein Refill mit
  `limits.save_ids` (NonZero ≤32) IDs, vor dem ersten Boundary
  (DUR-04.2 Z. 562-563 „DUR-05 liefert vorher IDs“).
- **SaveIdsNeeded:** Sim meldet höchstens einen je Boundary (`counters.rs:1951`).
  Der Driver hält höchstens **einen Refill in flight** (`refill_pending`); er
  wird bei Erfolg des Enqueues gesetzt und nach jedem Boundary-`Ok` (Normal und
  Recovery wenden Refill an) gelöscht. Größe = `limits.save_ids`.
- **Apply-Refusal (N3, ohne Zuordnung):** Sim wendet den Refill vor der
  Save-Auswahl desselben Boundaries an (`counters.rs:212-231`, `277-294`).
  Meldet genau der Boundary, der den Refill angewandt hat, `SaveIdsNeeded`, war
  der Refill abgelehnt (bei leerem Pool kann `Capacity("save ID pool")` nicht
  greifen). Dann neuer Refill mit neuen IDs; nach **3** aufeinanderfolgenden so
  erkannten Ablehnungen Fail-Stop `NativeRefillRefused` (deckt `StaleHandle` =
  `counter_owner` gehört nicht zur Shard-Inkarnation).
- Eine leere ID-Quelle quittiert nie Dirty/Priority (DUR-05 Z. 46); Counterticks
  laufen weiter.

### 6.5 Work-Executor und Completion-Disposition (A9, N3)

`take_work()` je Boundary; jede Work auf einem Tokio-Task mit
`execute(&store)`; Completion über bounded Kanal (Kapazität 32 = Pending-Limit).
Kein DB-Zugriff im Driver-Task.

**`OutstandingWork` (≤32 Einträge, Schlüssel `query.id()`):** Zustand je Query
`Executing` (Task hält Work/Lease) | `CompletionHeld` (in `held_completions`) |
`CompletionQueued` (in diesem Boundary eingereiht) | `Idle{status, since,
clarifications}` (Driver hält weder Work noch Completion, Lease frei). Einträge
entstehen bei `take_work()`; sie verschwinden, sobald
`player_ownership_checkpoint(query)` `CompletionMismatch` meldet (kein Pending).

**Nach jedem erfolgreichen Boundary** (Normal oder Recovery): für jeden Eintrag
in `CompletionQueued` liest der Driver `player_ownership_checkpoint(query)`
(`player_ownership.rs:684-698`). Kein Pending → Eintrag entfernt (veröffentlicht
oder definitiv freigegeben). Sonst → `Idle{status}`; die zugehörige abgelehnte
Completion wurde bereits verworfen (§6.3), die Lease ist frei.

**Je Iteration für `Idle`-Einträge (Klasse 1):**

| Status nach Apply-Refusal | Ursache in `complete_owned` (`1586-1650`) | Driver |
|---|---|---|
| kein Pending | `Confirmed{Ok}` veröffentlicht, oder definitive Refusal → `release_pending` + Resync (`1615-1619`) | nichts; Item bleibt ggf. dirty und wird neu gewählt; Leave-Save → `Due` (§7.3) |
| `Prepared` | `Refused(Busy|HistoryCapacity)` | `retry(query)` (`dispatch_retained`, verlangt Prepared und freie Lease, `1390-1395`) |
| `Uncertain` | `Unresolved`, `ReceiptAbsent` | `reconcile(query)` (`clarify = true`) |
| `NeedsRefresh` | `Confirmed{Err}` | `reconcile(query)` |
| `InFlight` | Validierung oder Veröffentlichung scheiterte vor jeder Mutation (z. B. `GenerationExhausted`) | `reconcile(query)` |
| `Quarantined` | `Refused(CasConflict)` im Counter-Modus (`1610-1614`) oder `confirm_current` scheitert | **kein** Input; Metrik `native_pending_quarantined`; bleibt bis Stop (`Unresolved`); Leave-Save → Quarantäne (§7.3) |
| Validierungsfehler (`CompletionMismatch`, fremder Owner, stale) | doppelte/alte Completion, kein Zustandswechsel | nichts, falls kein Pending; liegt ein Pending mit lebender Lease vor (`Executing`), wartet der Driver auf dessen eigene Completion |

Ein Retry/Reconcile führt nach dem Apply zu einer neuen Work (neue Lease,
`Executing`). Wird er selbst beim Apply abgelehnt (z. B. `Reserved`,
`UnprovenWorkerLifetime`), wird er verworfen; die nächste Checkpoint-Lesung
entscheidet erneut. Bleibt ein Eintrag nach zwei Klärungen im selben
nichtfinalen Status (`Uncertain`/`NeedsRefresh`/`InFlight`), folgt die nächste
Klärung frühestens 20 Boundaries später (begrenzte Rate, kein Abbruch: Storage
kann zurückkehren; ein Stop endet dann `Unresolved`).

**Damit sind §6.3, §6.5 und DUR-04.1/DUR-04.2 widerspruchsfrei:**
`held_completions` enthält ausschließlich beim Enqueue verweigerte Completions
(DUR-04.1 Z. 563-565); beim Apply verweigerte werden verworfen, wodurch die Lease
frei wird und Retry bzw. Reconcile möglich sind (DUR-04.1 Z. 475-477, DUR-04.2
Z. 606-609).

- **Refusal von `execute` (`evidence.rs:329-339`, falscher Store, Work kommt
  zurück):** Invariantenbruch der Slot-Verdrahtung → Work wird im Driver
  gehalten (Lease bleibt, kein Drop als „Abbruch“), Driver Fail-Stop mit
  Ergebnis `Unresolved`.
- **Verlorene Antwort/abgebrochener Task:** endet der Task ohne Completion, ist
  die Work samt Lease gedroppt → `Idle`, Checkpoint-Lesung → `reconcile(query)`;
  `ReceiptAbsent` bleibt Uncertain, nie Abbruch.
- **Dauerhaftes Checkpoint-Wiring:** DUR-04.1 Z. 296-301 weist es DUR-05 zu.
  NET-04.2f persistiert **keine** Checkpoints. Nach Neustart liest Restore den
  aktuellen Durable-Stand; ein unsicherer Autosave/Transfer war atomar (Player-
  CAS bzw. Pair-Commit), also gilt genau einer der beiden konsistenten Stände.
  Shutdown mit Uncertain bleibt „melden und L1 quarantinieren“ (§10.4).

### 6.6 Inbound nach dem Join

Connection sendet `SimCommand::Event` mit dem Sessionkey. Router prüft exakten
Key, Active, Region (§9) und baut `NativeShardInput`. Unbekannter/alter Key →
`StaleSession`, nie Retarget. Block-, Command-, Container- und Itemaktionen
haben im Nativmode noch keinen Consumer und werden typisiert verweigert.

## 7. Disconnect, Retention, Leave-Save und Release (B2)

### 7.1 Retention-Record in der Session (einziger Producer)

Ab dem Disconnect wird die Session zu einem **Retention-Record** in der
Session-Tabelle (derselbe Eintrag, keine Neuallokation):

```rust
// ferrumc-session, privat (e2)
struct NativeRetentionRecord {
    original: OwnedPlayerHandle,      // das in admit/adopt übergebene Handle
    token: NativeRetentionToken,      // dem Driver herausgegeben
    phase: Leaving | Detached | Retired { acked: bool } | Superseded,
    leave_save: LeaveSaveState,       // §7.3
    bound: bool,                      // false = adoptierte G3-Admission
}
/// Opak, Copy; enthält nur PlayerId und Admission-Lineage, keine Autorität.
pub struct NativeRetentionToken { player: PlayerId, lineage: u64 }
```

**Lineage (R2-A3):** Die Session vergibt sie aus einem routerlokalen monotonen
`u64`-Zähler (checked; Überlauf → typisiert `LineageExhausted` vor jeder
Wirkung, bei 2^64 praktisch unerreichbar) in `disconnect_native` bzw.
`adopt_unbound_native_admission` und gibt sie im Token zurück. Der Driver hält
für L2 **kein** Handle, nur das Token in seiner beim Start vorreservierten
Token-Tabelle (§7.4, §10.2). Der Record wird erst nach erfolgreichem Release
(§7.5) oder als `Superseded` nach Lifecycle-Ack entfernt.
**Grenzen:** Records teilen den Session-Tabelleneintrag; Sessions + Reservationen
+ Records ≤ Sessions-Limit (.2c), Records ≤ Sim-`players` (≤128).

Session-API (e2):
```rust
impl SessionRouter {
    pub fn disconnect_native(&mut self, key: &PlayerSessionKey) -> Result<NativeRetentionToken, CollectorSessionError>;
    pub fn native_detached(&mut self, player: PlayerId, view: &mut dyn FnMut(&OwnedPlayerHandle) -> Option<CollectorLifecycleView<'_>>);
    pub fn native_detached_intent(&mut self, handle: &OwnedPlayerHandle);
    /// `applied` = Zahl der bisher erfolgreich angewandten Normal-Boundaries (Driver-Zähler).
    /// `submit_allowed` = das Gate erlaubt in dieser Iteration noch einen gated Submit (§6.2).
    pub fn service_native_retention(&mut self, token: NativeRetentionToken, applied: u64, submit_allowed: bool,
        status: &mut dyn FnMut(&OwnedPlayerHandle) -> Result<DetachedOwnedPlayerStatus, PlayerOwnershipError>,
        ids: &mut dyn FnMut() -> OwnershipTransferId,
    ) -> Result<NativeRetentionStep, CollectorSessionError>;
    /// Sim-Enqueue des zuletzt gelieferten Submit scheiterte; Record zurück, kein Versuch gezählt.
    pub fn native_retention_unsent(&mut self, token: NativeRetentionToken);
    pub fn release_native_retention(&mut self, token: NativeRetentionToken,
        release: &mut dyn FnMut(&OwnedPlayerHandle) -> Result<ReleasedOwnedPlayer, PlayerOwnershipError>,
    ) -> Result<(), CollectorSessionError>;
}
pub enum NativeRetentionStep {
    Submit(NativeShardInput),          // Session-gebauter Leave-Save oder (G3/erneut) Disconnect, gated Klasse 2
    Waiting,                           // nichts zu tun in diesem Boundary
    SaveConfirmed,                     // saved_after_detach dieser Admission liegt vor → D9
    Quarantined(LeaveSaveQuarantine),  // Retries erschöpft oder Invariante → §7.3
}
```

`OwnershipTransferId`, `DetachedOwnedPlayerStatus`, `ReleasedOwnedPlayer` und
`PlayerOwnershipError` erreicht die Session ausschließlich über `ferrumc-sim`
(R2-A4, §7.5); ferrumc-session erhält keine Storage-Dependency.

### 7.2 Gebundene Session: Kette D1–D10

| # | Cutpoint | Wirkung | Fehler/Uncertainty |
|---|---|---|---|
| D1 | Connection-Ende, `NativeLeave{session, L1}` | L1 wandert in den Driver | Driver weg → L1 Quarantäne |
| D2 | `router.disconnect_native(key)`: Session → `Leaving`, baut einmal `OwnedPlayerInput::disconnect(original)` (Lane, Control); gibt `NativeRetentionToken` zurück | Lane-Item; Driver trägt das Token ohne Allokation ein, L2 beginnt (Permit-Epoche +1, §7.4) | Lane voll → retained, Retry je Boundary; Suspension → §4.3 |
| D3 | Sim-Apply `apply_native_disconnect` → Fact; Intent `PlayerDetached` → `router.native_detached(player, &mut \|h\| shard.collector_lifecycle_view(h))`: Session prüft, dass der Fact exakt auf das gespeicherte Original-Handle passt → `Detached` | Sim, Session | wiederholt → selber Fact |
| D4 | Ab `Detached`: Leave-Save über `service_native_retention` (§7.3), gated Klasse 2 | Sim pending Work | Retry/Quarantäne nach §7.3 |
| D5 | **.2c-Ergänzung R1:** `router.retire_detached_native_sessions(&mut \|h\| shard.collector_lifecycle_view(h))`: nur `ever_active`, `Detached`, View-Fact passt auf Original-Handle und volle Generation; entfernt das Key-Mapping, mandatory **EntityRemove, dann TabRemove** an Viewer mit Baseline; Record → `Retired{acked:false}` | Session | Pending Leave, stale Lease → kein Proof |
| D6 | Late Pickup: bereits reservierte Pins bestätigen; NET-04.2b nutzt `Retired(proof)`, ganzer FIFO-Headack gibt genau den Pin frei | Sim-Pins → 0 | Uncertain → retained |
| D7 | **.2c-Ergänzung R2:** `router.acknowledge_retired_native_records(&mut \|h\| view, &mut \|t\| shard.acknowledge_collector_lifecycle(t))`: nur bei Consumerrefs 0 und `retirement_ticket()` Some | Record `Retired{acked:true}`, **bleibt bestehen** | sonst unverändert |
| D8 | Im selben synchronen Closure: `acknowledge_collector_lifecycle(&ticket)` | Sim-Zelle frei (`collector_lifecycle.rs:525-526`), SIM-03.4b setzt das Admissionsbit `lifecycle_acked` (§7.5) | Fehler = Invariantenbruch → Fail-Stop |
| D9 | L1 frei, sobald D5 **und** `SaveConfirmed` für dieses Token vorliegen | Connection-Task endet | Retries erschöpft → §7.3 Quarantäne |
| D10 | L2 (Token + Record + Sim-Platz) bis Release (§7.5) | Session-Record, Driver-Token | — |

Alte Connection-Events nach Rejoin tragen den alten Key und treffen nie die
neue Session (`StaleSession`).

### 7.3 Leave-Save mit begrenzten Retries

`LeaveSaveState` im Record: `AwaitDetach{sent_at, resends}`, `Due`,
`Submitted{id, at}`, `Pending{id, since}`, `Waiting{since, blocking}`,
`Confirmed`, `Quarantined{reason}`, Zähler `attempts`. `at`/`since`/`sent_at`
sind Werte des Driver-Zählers `applied`. Je Iteration bedient der Driver
höchstens **8 Records** im Round-Robin über seine Token-Tabelle
(`service_native_retention`, Closure liefert
`detached_owned_player_status(original)`, §7.5); höchstens **4** davon dürfen
einen `Submit` liefern (`submit_allowed`, §6.2). Die Session entscheidet in
dieser Reihenfolge:

1. **Status-Fehler (R2-A1):**
   - `Ineligible("connected player")`: der Disconnect ist noch nicht
     angewandt. Gebundener Record → `AwaitDetach` ohne Versuch; ist seit
     `sent_at` mindestens ein Normal-Boundary angewandt (`applied > sent_at`)
     und der Spieler weiter verbunden, sendet die Session den Disconnect erneut
     (idempotent; gebunden über die Lane, G3 als `Submit`), höchstens 8-mal,
     danach `Quarantined{DisconnectNotApplied}`.
   - `StaleHandle`: das Original-Handle zeigt nicht mehr auf den echten
     Spieler, obwohl der Record weder `Superseded` noch freigegeben ist →
     `Quarantined{StatusInvariant}` (L1-Quarantäne, Metrik), der Record bleibt
     bis zum Stop.
   - Foreign Owner, `SnapshotOnly`, `NotInitialized`: Verdrahtungsbruch →
     Session gibt `CollectorSessionError::NativeStatusInvariant` zurück, Driver
     Fail-Stop.
2. `status.saved_after_detach()` = Some → `Confirmed`, Schritt `SaveConfirmed`.
3. `Due`:
   - `status.blocking()` = None und `submit_allowed` →
     `Submit(autosave(original, ids()))`, `attempts += 1`, → `Submitted{id, at}`.
     Scheitert schon das Sim-Enqueue, ruft der Driver `native_retention_unsent`
     (zurück zu `Due`, Versuch zurückgenommen). Ohne `submit_allowed` bleibt
     `Due` ohne Versuch.
   - **`status.blocking()` = Some(fremd) → `Waiting{since: applied, blocking}`,
     kein Versuch (R2-A1).**
4. `Submitted`, erst wenn `applied > at` (mindestens ein Normal-Boundary hat
   alle Queue-Inputs angewandt, `player_ownership.rs:299-335`):
   - `blocking` zeigt die eigene ID → `Pending{id, since}`;
   - `blocking` zeigt ein fremdes Pending → die Einreichung wurde mit
     `Reserved` verweigert → `Waiting` und `attempts -= 1` (Warten zählt nicht);
   - sonst (keine eigene ID, kein Nachweis) → verweigert aus anderem Grund
     (z. B. `Capacity`) → `Due` frühestens 20 Boundaries später (Versuch zählt).
5. `Pending`: Busy-Retry, Uncertain-Klärung und NeedsRefresh betreibt der
   Driver generisch für alle Pendings (§6.5); dank N3 werden sie tatsächlich
   aufgelöst. Je volle 100 angewandte Boundaries ohne Bestätigung zählt ein
   Versuch. Eigenes Pending `Quarantined` (CasConflict/Superseded) → sofort
   Quarantäne. Verschwindet das eigene Pending ohne `saved_after_detach`
   (definitive Refusal) → `Due` frühestens 20 Boundaries später (Versuch zählt).
6. `Waiting`: Wiederaufnahme (→ `Due`) sobald `status.blocking()` = None, sofort
   geprüft bei einem `native_detached_intent` für dieses Handle. Ist das fremde
   blockierende Pending `Uncertain`, zählt je volle 100 Boundaries ein Versuch;
   bei `Quarantined` sofort Quarantäne; bei `Prepared`/`InFlight`/`NeedsRefresh`
   zählt erst nach 1200 Boundaries Wartezeit je volle 100 Boundaries ein Versuch.
7. `attempts > 8` → `Quarantined{reason}`.

Wird ein Record erst einige Boundaries nach der Auflösung eines fremden
Pendings bedient, kann Punkt 4 eine `Reserved`-Refusal als „anderer Grund“
einordnen; das kostet höchstens einen Versuch und 20 Boundaries.

Im Recovery-Modus und ab Phase B ist `submit_allowed` immer `false` (§6.2,
§10.4); die Zähler laufen nur über `applied`, also nur in Normal-Boundaries.

**Reserved bei laufendem Pickup (Root-Auflage, R2-A1):** Ein detached Spieler
kann kein neues Pending erzeugen (Move/Mode/Select/Pickup verweigern detached,
`1905`/`1939`/`1969`; Pickup ist im Nativpfad bis DUR-05b ohnehin nicht
erreichbar). Das blockierende Pending ist daher genau der vor dem Detach
gestartete Pickup bzw. Autosave. Ablauf: Record `Due`, Status meldet
`blocking = (P, InFlight)` → Punkt 3 → `Waiting` ohne Versuch. Die Auflösung von
P (Confirmed → Veröffentlichung, `PlayerChanged`; definitive Refusal → Release
+ `Resync`; Busy → Driver-Retry nach N3; Uncertain → Reconcile nach N3)
entfernt das Pending bzw. führt dorthin und erzeugt ein Intent für dieses
Handle. Das Intent bzw. spätestens der nächste Status ohne `blocking` setzt den
Record auf `Due`; der nächste Leave-Save wird zugelassen, weil
`plan_owned_player_write` (`1010-1064`) dann kein Pending mehr findet und weder
`dirty` noch `connected` verlangt. Ein dauerhaft `Uncertain` bleibender Pickup
blockiert bis zur Klärung und zählt (Punkt 6).

**Quarantäne (typisiert, sichtbar):** `LeaveSaveQuarantine{player, reason,
attempts}`. Folgen: L1 wird mit `PlayerReservation::quarantine()`
(`player_admission.rs:97`) gesperrt, die UUID liefert bis zum Neustart
`AdmissionError::Quarantined`; Log-Warnung und Metrik
`native_leave_save_quarantined`; L2-Token, Record und Sim-Platz bleiben bis zum
Final-Save im Stop-Protokoll (§10.4 Phase A erneuert genau einen Versuch). Kein
stilles Dauerhängen, kein Release ohne `saved_after_detach`. Bestätigt eine
spätere generische Klärung doch noch einen Leave-Save dieser Admission, darf der
Release (§7.5) laufen und L2 freigeben; die L1-Quarantäne bleibt bis zum
Neustart bestehen.

Die Werte 8 Records/Iteration, 4 Submits/Iteration, 8 Versuche, 8
Disconnect-Wiederholungen, 20 Boundaries Wiedereinreichungsabstand, 100
Boundaries Zählintervall und 1200 Boundaries Wartezeit sind gewählt, **nicht
gemessen** (bei 20 Boundaries/s: Quarantäne frühestens nach etwa 40 s
unbestätigtem Pending).

### 7.4 L1 und L2

- **L1** = bestehende `PlayerReservation`: frei nach D5 + `SaveConfirmed`
  (D9); erschöpfte Retries oder Shutdown-Uncertainty → `quarantine()`.
- **L2** = Driver-Token-Tabelle `PlayerId → NativeRetentionToken` ab D2 bis
  Release; dazu der Session-Record mit dem Original-Handle. **R2-A3:** Die
  Tabelle wird beim Start fallibel auf das Sessions-Limit vorreserviert
  (§10.2); Eintragen nach D2 bzw. nach der Adoption alloziert nie und kann nicht
  an Kapazität scheitern, weil jeder Record einen Session-Tabelleneintrag
  belegt (≤ Sessions-Limit). Solange L2 besteht: Join nur über Rejoin-Permit;
  Restore-Permits verweigert.
- **Read-Permits:** höchstens einer pro UUID; global ≤ Sessions-Limit
  (NonZero ≤1024, H2: gewählt, nicht gemessen). Permit = `Arc`-Token bei der
  Connection, Driver hält Weak; tote Permits werden zu Beginn jeder Iteration
  deterministisch abgeräumt. Ein Permit ist an die **Permit-Epoche** der UUID
  gebunden: ein driverglobaler monotoner `u64`-Zähler (getrennt von der
  Session-Lineage im Token); jeder L2-Beginn und jedes L2-Ende erhöht ihn und
  markiert den offenen Permit der UUID als stale. Release verweigert bei offenem
  Permit. Release und Permit-Ausgabe laufen beide nur im seriellen
  Driver-Loop, ein Release ist innerhalb eines synchronen Schritts
  abgeschlossen (**A10:** es gibt keinen Zustand „laufender Release“).
  `NativeJoin` mit stale/fremdem Permit → `StaleSnapshot` vor jeder Simwirkung.
  Restliche Annahme: Restore-Aktualität nach bestätigtem Commit (redb-MVCC-
  Monotonie bleibt Bibliotheksannahme).
- **Rejoin vor Release:** Sim-Rejoin verlangt `!connected`, kein Pending,
  `!dirty`, Stamp- und Recordgleichheit. Ein in-flight Leave-Save blockiert ihn
  in Sim (`Reserved`). Der Rejoin publiziert eine **neue** Admission; im selben
  Schritt: Session setzt den alten Record auf `Superseded` (kein Leave-Save,
  kein Release mehr; er wartet nur noch auf D7/D8 der alten Lifecyclezelle und
  wird dann entfernt), Driver beendet das alte L2-Token (Permit-Epoche +1).
  `saved_after_detach` wird durch `publish_player` gelöscht.

### 7.5 SIM-03.4b — Release detached PlayerState (G5, R3-2)

```rust
impl SimShard {
    /// Rein lesend, ohne Allokation, nur mit dem Original-Handle.
    pub fn detached_owned_player_status(&self, original: &OwnedPlayerHandle)
        -> Result<DetachedOwnedPlayerStatus, PlayerOwnershipError>;
    pub fn release_detached_owned_player(&mut self, original: &OwnedPlayerHandle)
        -> Result<ReleasedOwnedPlayer, PlayerOwnershipError>;
    /// Rein lesend, ohne Allokation; derselbe interne Helfer wie ownership_quiescent.
    pub fn owned_stop_ready(&self) -> Result<bool, PlayerOwnershipError>;
}
/// Private Felder, nur Getter; reine Daten ohne Autorität.
pub struct DetachedOwnedPlayerStatus { /* … */ }
impl DetachedOwnedPlayerStatus {
    pub fn saved_after_detach(&self) -> Option<OwnershipTransferId>;
    pub fn blocking(&self) -> Option<(OwnershipTransferId, OwnershipPendingStatus)>;
    pub fn dirty(&self) -> bool;
}
/// R2-H5: Quittung ohne Autorität: kein Handle, keine Query, kein öffentlicher Konstruktor.
pub struct ReleasedOwnedPlayer { /* player: PlayerId, freed_player_bytes: usize */ }
impl ReleasedOwnedPlayer {
    pub fn player(&self) -> PlayerId;
    pub fn freed_player_bytes(&self) -> usize;
}
// R2-A4: in ferrumc-sim/src/lib.rs
pub use ferrumc_storage::{OwnershipReceiptQuery, OwnershipTransferId};
```

- **R2-A4 (Crate-Graph):** `OwnershipTransferId` und `OwnershipReceiptQuery`
  sind in ferrumc-storage definiert (`ferrumc-storage/src/ownership.rs:207`,
  `605`); ferrumc-sim exportiert sie heute nicht weiter (`lib.rs:136-140`,
  `player.rs:25-30`), und ferrumc-session hängt von ferrumc-sim, nicht von
  Storage ab (`ferrumc-session/Cargo.toml:33`; CLAUDE.md: Session „NEVER raw
  storage“). SIM-03.4b ergänzt deshalb den obigen Re-Export in ferrumc-sim
  (reine Typnamen, keine Storage-Funktion, keine Backendhandles). Die Session
  nennt die Typen als `ferrumc_sim::OwnershipTransferId`; eine neue
  Session→Storage-Dependency entsteht nicht.
- **`saved_after_detach`** setzt ausschließlich Sim beim Anwenden der echten
  `Confirmed{current: Ok}`-Completion eines **Autosave**-Pendings genau dieser
  Admission, wenn der Spieler zu diesem Zeitpunkt `!connected` ist. Bei
  mehreren Retries gilt die zuletzt bestätigte. Zurückgesetzt auf `None` wird es
  durch jede spätere Mutation dieses PlayerStates (Deferred-Apply,
  Rejoin-Publish, Dirty) und durch jedes neu zugelassene Pending dieses Spielers;
  ein späteres unsicheres Ergebnis lässt das Pending bestehen und blockiert den
  Release typisiert.
- **`detached_owned_player_status`:** `StaleHandle`/Foreign/`SnapshotOnly`
  typisiert; verbundener Spieler → `Ineligible("connected player")`
  (Disposition in §7.3 Punkt 1).
- **Lifecycle-Ack beobachtbar (R2-H6):** Eine acknowledgte Zelle ist in der
  Sim nur als „keine Zelle mehr“ sichtbar (`collector_lifecycle.rs:425-438`,
  `525-526`). Der Nativpfad initialisiert den Lifecycle verpflichtend (§10.2
  Schritt 5); dort reserviert und versiegelt jede Restore-/Rejoin-Admission eine
  Zelle (`player_ownership.rs:807-822`), Bootstrap ebenso (`437`). SIM-03.4b
  setzt zusätzlich beim erfolgreichen `acknowledge_collector_lifecycle` ein
  privates Bit `lifecycle_acked` im PlayerState genau dieser Admission
  (zurückgesetzt nur durch eine neue Admission). Das Release-Gate verlangt
  Lifecycle-Modus initialisiert **und** `lifecycle_acked`; „nie eine Zelle“
  (Lifecycle nicht initialisiert) ist damit eine typisierte Refusal, nicht
  „acknowledged“.
- **Release-Gate** (alles vor Mutation, typisierte Refusal, keine Teilmutation):
  echter Owner, nicht SnapshotOnly/Stopped, `actual_player(original)`,
  `!connected`, `lifecycle_acked` dieser Admission, `saved_after_detach` Some,
  kein Pending/Deferred/Dirty, kein Pending/Bootstrap/Restore und **kein Input
  in der Sim-Queue** für dieselbe PlayerId, keine Publication-Pins/Item- oder
  Chunkreservierung, keine Collector-Bindung. Wirkung: entfernt PlayerState,
  `player_bytes` und die zugehörigen Zähler im selben Schritt.
- **Verhältnis zum DUR-04.2a-Boundary-Plan (A7, R2-H3 korrigiert):** Release
  läuft ausschließlich **zwischen** Boundaries (nie innerhalb `_at`/Recovery).
  Der Plan modelliert Player-Admissions skalar aus dem Zustand bei Planbeginn
  (DUR-04.2 Z. 588-593) und kopiert die `LifecycleCredits` je Plan frisch aus
  dem Live-Ledger (`counters.rs:510`); es gibt keine dauerhafte Countershadow-
  Kopie, die der Release bereinigen müsste. Die Zelle dieser Admission ist schon
  beim Ack frei (`collector_lifecycle.rs:525-526`). Ein nach dem Release
  geplanter Boundary sieht nur die verringerte Spielerzahl. Da das Gate jeden
  Queue-Input dieser PlayerId verweigert, kann kein geplanter Input auf einen
  entfernten Spieler zeigen.
- **Arbeitsgrenze (A7):** je Aufruf ≤ 128 Spieler + 384 Lifecyclezellen + 32
  Pendings + 32 Queue-Inputs + 256 Publication-Gruppen
  (`MAX_ITEM_PUBLICATION_GROUPS`, `item_publication.rs:21`) + 1
  Entity-Reservation = 833 Eintragsbesuche; keine Allokation; höchstens
  **4 Releases je Iteration** (gewählt, nicht gemessen).
- **Keine automatische Verdrängung.** Bis SIM-03.4b existiert, ist
  `Capacity("player count")` die typisierte Aktivierungsgrenze.
- **`owned_stop_ready`:** exakt die Prädikate von `ownership_quiescent`
  (`player_ownership.rs:701-720`) über einen gemeinsamen privaten Helfer, keine
  Kopie, keine Allokation.

Release-Reihenfolge: D5 → D6–D8 → `SaveConfirmed` → im Driver
`router.release_native_retention(token, &mut |h| slot.shard.release_detached_owned_player(h))`
→ Session entfernt den Record → Driver entfernt das Token (Permit-Epoche +1) → L2 frei.

## 8. .2c-Ergänzung G3 (Runde 3 präzisiert) — nie gebundene echte Admission

Additiv; der Zweig „ehemals Active“ (§7.2) bleibt unverändert.

```rust
impl SessionRouter {
    /// Konsumiert die Reservation genau einmal; alloziert nicht (Platz seit Schritt 6 reserviert).
    pub fn adopt_unbound_native_admission(&mut self, reservation: NativeJoinReservation,
        handle: &OwnedPlayerHandle, view: &ConnectedOwnedPlayerView<'_>)
        -> Result<NativeRetentionToken, AdoptionInvariant>;
    pub fn retire_unbound_native_admission(&mut self, token: NativeRetentionToken,
        lifecycle: &mut dyn FnMut(&OwnedPlayerHandle) -> Option<CollectorLifecycleView<'_>>)
        -> Result<(), CollectorSessionError>;
}
```

- **Wann:** `PlayerAdmitted` für eine `PendingNativeJoin` ohne erreichbare
  Connection, oder `admit_native_join` hat verweigert. Der Driver übergibt das
  Handle aus dem Intent im **selben synchronen Schritt**; er speichert es nie.
- **Kapazitätsunfehlbar Ende zu Ende (R3-3, R2-A3):** Der Session-Tabelleneintrag
  wurde in Schritt 6 fallibel reserviert und in das Sessions-Limit gezählt;
  `cancel_native_join` gibt ihn bei Abbruch vor der Admission deterministisch
  frei. Die Adoption verschiebt nur Handle und Zustand in diesen Eintrag und
  vergibt die Lineage aus dem Session-Zähler (§7.1); der Driver trägt das Token
  in seine vorreservierte Tabelle ein. Beides kann weder an Count noch an
  Allokation scheitern.
- **Einzige mögliche Fehler** (`AdoptionInvariant`): Reservation stammt nicht
  von diesem Router; Spieler von Reservation und Handle verschieden; View
  gehört nicht zu diesem Handle/dieser Generation/Owner-Lineage der
  Registrierung; `LineageExhausted`. Jeder ist ein Invariantenbruch: **fatal
  und typisiert**. Die Session markiert den reservierten Eintrag als
  `Poisoned{player, kind}` (sie weiß also vom Handle-Fall); der Driver geht in
  Fail-Stop (keine Joins, Stop-Protokoll, Ergebnis `Unresolved` mit dieser
  UUID, L1 quarantiniert). Das Handle wird weder behalten noch unbemerkt
  verworfen; der Sim-Spieler bleibt admitted, deshalb kann Stop nicht quiescent
  werden, und das wird gemeldet.
- **Ablauf nach Adoption:** Record `bound=false`, `Leaving`. Die Session liefert
  über `service_native_retention` zuerst `Submit(disconnect(original))`
  (gated Klasse 2; solange der Status `Ineligible("connected player")` meldet,
  nach §7.3 Punkt 1 erneut, Disconnect ist idempotent), danach dieselbe Kette
  D3, D4 (Leave-Save §7.3), dann `retire_unbound_native_admission` statt D5:
  prüft im eigenen Router-State, dass der Record nie Active war, der Fact exakt
  auf Owner/Admission/Generation des gespeicherten Handles passt und keine
  Session, kein anderer Record, kein Collector-Record und keine Reservation auf
  diese Admission verweist (Scan ≤ Sessions-Limit). Keine Tracking-Pakete, da
  nie gebunden. Dann D7/D8 (keine Pins, da ohne Session kein Pickup), D9 für L1,
  D10/§7.5 für L2.
- **Bootstrap-Uncertainty:** Zelle, Reservation, L1 und Pending bleiben beim
  Driver; nur echte Completion löst auf (Confirm → Admission → Bindung oder
  Adoption; definitiver Abort → Cancel). Stop bleibt nicht quiescent; Shutdown
  meldet `Unresolved` und quarantiniert L1.

Race-Regeln (jede mit Matrixzeile in §14, **B3**):

- **Join-Enqueue in-flight, Reply geschlossen** (Restore, Rejoin, Bootstrap):
  Reply wird erst nach Admission/Refusal bewertet; Admission → Adoption;
  Refusal → `cancel_native_join`.
- **Connection endet zwischen Schritt 5 und 10:** `NativeLeave` trägt den
  Permit statt eines Sessionkeys; Driver markiert `PendingNativeJoin` als
  `reply_closed`; weiter wie oben.
- **Späte Events einer nie gebundenen Connection:** Sie besitzt keinen
  Sessionkey; `SimCommand::Event` ohne Key wird vom Driver typisiert
  verworfen (`NoNativeSession`), keine Sim- oder Recordwirkung.
- **Rejoin derselben UUID während G3:** L1 liegt beim Driver bis D9 → neue
  Connection `AlreadyReserved`. Nach D9 und vor Release: Rejoin-Permit, Sim-Rejoin,
  alter Record → `Superseded` (§7.4).
- **Reply-Fehler in Schritt 10:** Session ist Active (A2) → normaler Leave, nicht G3.
- **Adoption-Invariantenbruch:** fatal wie oben.
- **Stop während PendingNativeJoin:** neue Joins → `ShuttingDown`; eingereihte
  Join-Inputs werden in Phase A angewandt oder verweigert (Drain verweigert
  Joins, §10.4); Admission → Adoption → G3-Kette in Phase A, Refusal → Cancel.
- **Endpoint-Replacement während G3:** Record hängt nicht an der Lane;
  Unregister verweigert `NativeEndpointInUse`.

## 9. G2 — Regionsgrenze, sichtbar

Region = 8×8 Chunks des Owner-Home `ShardId::region()` (128×128 Blöcke).

- **Start:** native Konfiguration verweigert typisiert
  (`NativeSpawnOutsideRegion`), wenn Spawn oder ein Spawn-Ticketchunk außerhalb
  liegt. Kein Clamp, kein Verschieben.
- **Admission:** Restore außerhalb → Schritt 4 `NativeAdmissionOutsideRegion`
  (stabiler `PlayDisconnect`-Text), kein Reset auf Spawn.
- **Chunks:** JoinGame-Viewdistance bleibt der konfigurierte Wert; der
  Chunkstream fordert nur `desired_chunks ∩ Region` an. Beim ersten Erreichen von
  ≤ Viewdistance zum Rand einmal je Annäherung eine droppable Action-Bar
  „Native Simulation: Regionsgrenze (128×128 Blöcke)“.
- **Bewegung (A10):** Session prüft jede Positionsänderung mit **derselben
  Chunkableitung wie Sim**: `EntityTransform::try_new(position, 0, 0)?.chunk()`
  (`require_player_region 906-921`), keine eigene Rundung. Außerhalb → kein
  Sim-Input, `NativeRegionBoundary`, mandatory `SynchronizePlayerPosition` auf die
  letzte echte Sim-Pose plus droppable Action-Bar. Paper-Pin: `handleMovePlayer`
  teleportiert bei verworfener Bewegung auf die alte Position (Vanilla
  991-1126). Ungültige Positionen (`try_new`-Fehler) → dieselbe Korrektur.
- WorldBorder-Pakete sind in `ferrumc-proto` nicht generiert (eigenes
  Generator-Ticket). Multi-Shard/Transfer: eigenes späteres Ticket.

## 10. G4 — Format, Initialisierung, Persistenz und Stop

### 10.1 Format7 (A8)

Native Modus ist explizit konfiguriert. DUR-05a öffnet den Store ausschließlich
über die DUR-03.3-API (Vertrag Z. 55-57):
`RedbStore::open_with_player_block_durability(path, config, scratch_directory)`
für redb, `InMemoryStore::with_player_block_durability(config)` nur für Tests.
Fresh `create_new` schreibt Format7; Existing7 wird validiert; Format3..6 →
`MigrationRequired` → typisierte Startrefusal `NativeStoreFormatUnsupported{found}`
**vor** Listener-Bind; kein Legacy-Fallback, keine automatische Migration
(DUR-03.3a offline). Dasselbe Store-Objekt dient als
`Arc<dyn PlayerEntityOwnershipStore>` (alle sieben Methoden in Format7
unterstützt) für `OwnershipStoreBinding` und als `&dyn WorldStore`-Leser für
`acquire_owned_chunk`.

### 10.2 Initialisierungsreihenfolge (B1, R2-A3)

Auf leerem Owner, jeder Fehler beendet den Start vor Clientannahme:
1. Store öffnen (10.1), Generation-Identity und Scope prüfen.
2. `SimShard` mit dieser Generation; Entities mit Itembudget.
3. `initialize_player_ownership(binding, limits)` mit `inputs = 32`,
   `input_bytes = 8 MiB`.
4. `initialize_item_publications(…)`.
5. `initialize_collector_lifecycle(…)` (verpflichtend, §7.5 R2-H6).
6. `initialize_owned_item_counters(limits, Tick::ZERO)` → `counter_owner`.
7. Initialer Save-ID-Refill (§6.4) über `enqueue_owned_player_input`.
8. Session `initialize_collector_sessions(ConfigRegistries)`.
9. `register_native_world_shard` → `NativeShardSlot`; dabei fallibel
   vorreserviert: Token-Tabelle auf das Sessions-Limit (R2-A3),
   `OutstandingWork` und `held_completions` auf je 32, `PendingNativeJoin`-FIFO
   auf das Sessions-Limit. Fehler → typisiert `NativeStartupAllocation`.
10. Erst dann Listener-Bind.

### 10.3 Persistenzbarriere (A8)

Die Legacy-Freigabe „wie heute“ (`release_chunks_acked`, `StorageFlushRequest`,
`save_chunk_overlays`/`append_block_mutation_batch`, `driver.rs:1511-1548`,
`storage_worker.rs:365-400`) ist im nativen Modul nicht erreichbar (Source-Scan
wie §4.5). Im Nativpfad vor DUR-05b gibt es keinen Blockwriter (alle
Blockaktionen verweigert, §6.6); Chunks sind Generatorausgabe oder gelesene
Format7-Bilder. Daher:
- **Chunkticket-Freigabe:** `release_owned_chunk` (`player_ownership.rs:2083`)
  ohne Storage-Write; Sim markiert Items im entladenen Chunk zum
  Prioritätsflush; deren Save läuft als Entity-CAS-Work.
- **Einzige Durable-Barriere** ist die receipt-gestützte Player-/Entity-CAS mit
  Current-Read (`Confirmed{current: Ok}`); jede Commit-Transaktion ist bei redb
  `Durability::Immediate` (DUR-03.3 Z. 292).
- **Stop-Barriere:** `Stopped` (§10.4) → alle Executor-Tasks gejoint
  (begrenzt) → letztes `Arc` auf den Store gedroppt → redb geschlossen.
- DUR-05b führt Blockmutationen nur über `commit_player_blocks`/`commit_blocks`
  ein und legt dort die Chunk-Entladebarriere auf Block-Receipts fest.

### 10.4 Stop-Protokoll A/B/C (R3-1, N2)

Phasenwechsel und Gate gehören dem Slot (`phase`, `gate`). Ab Phase B ist
`NonRecoveryGate::admit` für jedes gated Input `Err(NativeShutdownPhase)`;
Retention-Service, Lane-Fenster und Join-Einreihung bekommen dann keinen
Ticket. In Phase C liefert ausschließlich `stop_ticket()` einen Ticket, und nur
bei `G = 0`.

**Phase A (Normal-Boundaries, ≤ 1200 Iterationen, nicht gemessen):**
- Erste Iteration: neue Read-Permits und Joins `ShuttingDown`;
  `disconnect_native` für alle Active-Sessions (Client `PlayDisconnect` „Server
  stopping“); `drain()` als Klasse-1-Input. Drain wird vor den gated Inputs
  derselben Iteration angewandt; Disconnect, Autosave, Retry und Reconcile
  bleiben unter Draining zulässig (`player_ownership.rs:395-410`), Joins und
  Lane-Data werden verweigert (Join → `cancel_native_join`, Data verworfen).
- D3–D8 und Leave-Saves laufen; je quarantiniertem Record genau ein erneuter
  Leave-Save-Versuch („Final-Save“). G3-Adoptionen laufender Admissions folgen
  §8.
- **Übergang zu B** erst, wenn: alle Records `Retired{acked}`/`Superseded`/
  `Poisoned`, alle Leave-Saves bestätigt oder quarantiniert, keine
  `PendingNativeJoin`, `held` leer, die Lane leer (`try_recv` → leer) und der
  **letzte Boundary ein erfolgreicher Normal-Boundary** war, nach dem kein gated
  Input mehr eingereiht wurde. Dann ist `G = 0` und die Sim-Queue enthält kein
  nicht recovery-anwendbares Input.
- Ablauf der 1200 Iterationen: Wechsel nach B trotzdem (um Completions
  abzuschließen), aber mit Merker `phase_a_incomplete`; liegt dann noch ein gated
  Input in der Queue, kann `owned_stop_ready()` nie wahr werden, und Phase B endet
  über ihren Wachhund mit `Unresolved`.

**Phase B (nur Recovery-Boundaries):**
- **Phase-B-Drain (N2):** In der ersten Phase-B-Iteration ist `drain()` das
  **erste** Input der Queue, vor allen Completions. Der Recovery-Boundary wendet
  es in Queue-Reihenfolge an (`counters.rs:277-294`); es ist unter Draining
  zulässig und markiert alle Slots priorisiert (`player_ownership.rs:567-576`,
  `entity/counters.rs:262-273`).
- Danach je Iteration nur Klasse 1: Completions, Retry, Reconcile, Refill;
  Releases zwischen den Boundaries (≤4 je Iteration).
- **Invariante P:** Nach dem Phase-B-Drain gilt bis zum Ende von Phase B für
  jeden vorhandenen Slot `priority ∨ (generation = saved ∧ ¬due)`.
  Begründung aus dem Code: (1) Der Drain setzt `priority` für alle Slots.
  (2) `priority` löscht nur die Save-Bestätigung (`install_confirmed`,
  `entity/counters.rs:98-105`), die dabei `saved = generation` setzt; im selben
  Schritt kann nur die gebundene Projektion `generation` ändern
  (`apply_counter_publication`, Teil von `publish_counter_confirmation`,
  `counters.rs:400-423`), und dann setzt `restore_counter_flush` die Priorität
  wieder, weil sie vorher gesetzt war (`entity/counters.rs:254-260`).
  (3) Sonst ändert sich `generation` in Phase B nur über
  `release_counter_projection` bzw. Projektion, beide ohne `priority`
  zu löschen. (4) Counterschritte, die `generation` ohne Bestätigung erhöhen,
  gibt es nur in Normal-Boundaries (`counters.rs:2082-2113`, `1781-1806`), und
  Phase B hat keinen. (5) Drop/Pickup/Restore-Item/Autosave werden in
  Recovery nicht angewandt (`277-294`), neue Pendings entstehen nur als
  Counter-Saves. Daher findet die Priority-Lane (`d.priority || d.due`,
  `counters.rs:1917-1923`) jeden noch nicht sauberen Slot; die Normal-Lane, die
  ohne Tickfortschritt nichts wählt, wird nicht gebraucht.
- **Terminierung:** Höchstens eine Entity-Operation zugleich
  (`counters.rs:1884-1886`); jede bestätigte Save macht genau einen Slot sauber,
  und kein Slot wird ohne Priorität dirty (P). Nötig sind höchstens
  `N_B + 33` Bestätigungen (N_B Items, ≤32 Pendings bei Phase-B-Beginn, ≤1
  Neuspeicherung durch eine in Phase A zugelassene Projektion).
- **Grenze (R2-A6, gewählt, nicht gemessen):** Phase B endet mit `Unresolved`,
  wenn (a) 1200 aufeinanderfolgende Recovery-Boundaries ohne Fortschritt
  vergehen (Fortschritt = Pendingzahl gesunken oder ein `outstanding`-Eintrag
  verschwunden, §6.5) oder (b) die Gesamtzahl `B_max = 12 000 + 8 · N_B`
  überschritten ist, mit `N_B = entities().map_or(0, len)` bei Phase-B-Beginn
  (`shard.rs:491-493`, `entity.rs:475-477`). Vor DUR-05b ist `N_B = 0`, also
  `B_max = 12 000` (≈ 10 min bei 50 ms). Der Faktor 8 deckt im Modell eine
  mittlere Save-Latenz bis 6 Iterationen (300 ms) ab (unten).
  **DUR-05b-Auflage:** Save-Latenz und Stopdauer mit echten Items messen und
  `B_max` sowie den Wachhund danach festlegen.
- Phase B endet bei `owned_stop_ready() == true` (§7.5) → Phase C. Dann gilt:
  Sim-Queue leer, kein Pending, Zellen 0, Ledger leer, Spieler sauber,
  `counter_quiescent`.

**Phase C:**
- Der Driver reiht **genau ein** Input ein: `stop()` über `stop_ticket()`
  (`G = 0`). Kein Refill, keine Retention, kein Lane-Item, kein Join. Eine jetzt
  eintreffende Completion kann zu keinem Pending gehören (es gibt keines) und
  wird verworfen.
- Dann **genau ein** Normal-Boundary `_at(clock.last.next())`. Der Plan sieht
  `remaining == 0`, weil Stop das einzige Nicht-Completion-Input ist
  (`counters.rs:794-805`), und denselben quiescenten Vorschrittzustand
  (`896-909`): `stopped = true`, alle Einträge `resident = false`, kein
  Counterschritt (`666-679`). Der Stop-Apply prüft `ownership_quiescent` auf
  demselben Zustand (Queue genommen, `boundary_remaining = 0`,
  `player_ownership.rs:577-588`, `701-720`) → `Stopped`; die Save-Auswahl kehrt
  bei `stopped` sofort zurück (`counters.rs:1880-1972`).
- `_at` scheitert vor jeder Mutation (z. B. `AllocationFailed`): Stop bleibt in
  der Queue; Wiederholung nach der K-Regel (§6.1a). Dazwischenliegende
  Recovery-Boundaries ändern nichts (kein Input anwendbar, alle Slots sauber,
  keine Auswahl), der nächste Normal-Versuch nimmt Stop an. Nach 3
  fehlgeschlagenen Versuchen → `Unresolved{reason: StopBoundaryFailed}`.
- Wird Stop trotzdem verweigert (durch den obigen Zustand ausgeschlossen, nur
  Testinjektion): der Normal-Boundary hat gestept → zurück zu Phase B mit neuem
  Phase-B-Drain, angerechnet auf `B_max`; höchstens 3 Phase-C-Anläufe.

**Durchrechnung 4096 Items** (Quellmodell `round4/n2_stop_model.py`, Ergebnis
`round4/n2_stop_model.json`, Schritte in `round4/n2_stop_walkthrough.md`; keine
Runtime-Messung). Annahmen: alle Items resident und durch den letzten
Phase-A-Boundary dirty, `save_scan = 64`, `save_period = 100`, Pool 32,
Completion L Iterationen nach der Work verfügbar, Normal-Lane findet nichts.

| L | Recovery-Boundaries bis `owned_stop_ready` | Bestätigte Saves | Refills | ≈ Zeit bei 50 ms | in `B_max = 44 768` |
|---|---:|---:|---:|---:|---|
| 1 | 5 696 | 4 096 | 127 | 285 s | ja |
| 2 | 9 792 | 4 096 | 127 | 490 s | ja |
| 3 | 13 888 | 4 096 | 127 | 694 s | ja (Runde-3-Grenze 12 000 wäre überschritten) |
| 6 | 26 176 | 4 096 | 127 | 1 309 s | ja |
| 1, ohne Phase-B-Drain (Runde 3) | nie (0 Saves in 12 000) | 0 | 0 | — | Gegenprobe N2(a) |

Die untere Schranke ist `4096 · 4/3 ≈ 5 462` (Priority-Lane 3 von 4 Turns,
`counters.rs:1898`); die Differenz zu 5 696 entsteht, weil ein leerer Pool den
Turn verbraucht und den Cursor vor der ID-Prüfung weiterrückt
(`counters.rs:1929-1940`), sodass das übersprungene Item erst im nächsten
Umlauf gespeichert wird. Danach Phase C: ein Normal-Boundary, Stop angenommen.

**Phase B und laufende Transfers (Root-Auflage):** Pickups, die vor `Drain`
admitted wurden, sind Pair-Pendings; ihre Completion, Retry und Klärung werden
in Recovery-Boundaries angewandt, die Veröffentlichung wendet die gebundene
Counterprojektion an, ohne neuen Counterschritt; Menge und Reservierung bleiben
bis zum Receipt im Owner (DUR-04.2 §7 „Strukturreservation“). Nach `Drain`
eingereihte Drop/Pickup-Inputs verweigert Sim mit `Draining`, Input intakt,
Items unverändert. **Merges gibt es nicht**: der native Merge (.2e) hat keinen
Storage-Producer (DUR-04.2 §6). Im Nativpfad vor DUR-05b entstehen ohnehin keine
Drop/Pickup-Inputs.

**Ergebnis:** `NativeShutdownOutcome::Stopped` oder
`Unresolved{players: ≤128 PlayerIds, pending: usize, reason}` bei Phasenablauf,
Wachhund, `Degraded{terminal}`, Fail-Stop oder Adoption-Poison. Bei
`Unresolved` werden alle betroffenen UUIDs L1-quarantiniert, der zuletzt
bestätigte Durable-Stand bleibt maßgeblich, es wird kein Erfolg und kein
Teilzustand erfunden; kein `Stopped` ohne Sim-Intent.

Terminaler Fall (`GenerationExhausted`/`CounterTickOverflow`): Phase A ist
unmöglich (Disconnect/Autosave werden nur in Normal-Boundaries angewandt),
direkt Phase B, Ergebnis `Unresolved`.

## 11. Grenzen und Dispositionen

„nicht gemessen“ = gewählte Vertragsgrenze ohne Messung (H2).

| Ressource | Grenze | Herkunft | Voll / Fehler |
|---|---|---|---|
| Native Lane | NonZero ≤1024, Control-Reserve 16, +1 `held` (mit Key) | = Router-Defaults; nicht gemessen | Data → Overload-Policy; Disconnect retained; `held` einer inaktiven Session verworfen |
| Sim-Inputs | `inputs` = 32, Bytes 8 MiB | DUR-04.1 | — |
| Gated Belegung `G` | ≤16 (Lane + Join + Retention + Stop) | nicht gemessen | Gate verweigert, Input wartet beim Producer |
| Reserve Klasse 1 | 16 Plätze, ≥ 6 283 264 Bytes (≥ 4 maximale Completions) | folgt aus G ≤ 16, Join ≤ 2 | Enqueue-Refusal → `held_completions` |
| Lane je Iteration | ≤ 16 − G (≥10 im Normal-Modus); Recovery/Phase B/C 0 | nicht gemessen | Rest wartet in Lane |
| Join-Inputs in Queue | ≤2 | nicht gemessen | `PendingNativeJoin` wartet |
| Retention | 8 Records/Iteration, ≤4 Submits | nicht gemessen | Round-Robin |
| `held_completions` | ≤32, nur Enqueue-Refusals | = Pending-Limit | nie mehr als Pendings |
| `OutstandingWork` | ≤32 | = Pending-Limit | vorreserviert |
| Pending/Work, Completion-Kanal | ≤32 | DUR-04.1 | nie voll |
| Klärung je Query | ab der 3. Klärung im selben Status ≥20 Boundaries Abstand | nicht gemessen | — |
| Save-ID-Pool, Refill in flight | ≤32 IDs, 1 Refill; 3 erkannte Apply-Refusals → Fail-Stop | DUR-04.2; Zahlen gewählt | §6.4 |
| Recovery-Modus | K = 20, F_max = 60 → `Degraded` | nicht gemessen | §6.1a |
| Pending Joins/Reservations | ≤ Sessions-Limit inkl. reserviertem Tabelleneintrag | .2c + R3-3 | typisiert vor Sideeffects |
| Retention-Records, Token-Tabelle | ≤ Sessions-Limit und ≤128; Tabelle beim Start vorreserviert | .2c / Sim / R2-A3 | folgt Reservation |
| Read-Permits | 1/UUID, global ≤ Sessions-Limit | nicht gemessen | typisiert |
| Leave-Save | 8 Versuche, 8 Disconnect-Wiederholungen, 20 Boundaries Wiedereinreichung, 100 Boundaries Zählintervall, 1200 Boundaries Wartezeit | nicht gemessen | Quarantäne §7.3 |
| Releases | ≤4/Iteration, ≤833 Eintragsbesuche, keine Allokation | nicht gemessen | Rest nächste Iteration |
| Stop Phase A | ≤1200 Iterationen | nicht gemessen | Merker, weiter nach B |
| Stop Phase B | Wachhund 1200 ohne Fortschritt; `B_max = 12 000 + 8·N_B` | nicht gemessen; DUR-05b-Auflage | `Unresolved` |
| Stop Phase C | ≤3 Anläufe | nicht gemessen | `Unresolved` |
| .2c Work/Scratch | unverändert | .2c | unverändert |

## 12. Originalreferenz (Quellenprüfung, kein Differenziallauf)

Lesepins in `.codex-tmp/net04_2d/round4/reference-pins.json` (unverändert aus
Runde 2/3 übernommen und frisch gehasht); 25 .2c-Pins frisch identisch.

| Quelle | Befund für FerrumC |
|---|---|
| Vanilla `PlayerList.placeNewPlayer` 139-235 | Login(eigene Entity-ID) → Difficulty → Abilities → HeldSlot → Recipes → … → `sendLevelInfo` → `addNewPlayer` → `initInventoryMenu` |
| Applied Paper `placeNewPlayer` 151-~400 | zusätzlich `PlayerSpawnLocationEvent`, `supressTrackerForLogin`, `PlayerJoinEvent`, `canSee`-Filter, `chunkMap.addEntity` danach |
| Vanilla `sendLevelInfo` 666-679 | GameEvent13 nach Spawnposition |
| Vanilla `remove` 334-367 / Paper 505-600 | Save → `removePlayerImmediately` (EntityRemove) → `players.remove` → PlayerInfoRemove (`broadcastAll` bzw. Paper alle mit `canSee`) |
| `onDisconnect`/`removePlayerFromWorld` V1349-1363, P2170-2208 | Disconnect → Quitmeldung → `PlayerList.remove` |
| `ServerLevel.addPlayer` V921-931, P1516-1525 | doppelte UUID wird entfernt (FerrumC verweigert) |
| `handleMovePlayer` V991-1126 | verworfene Bewegung → `teleport` auf alte Position |

Bekannte, bewusst erhaltene Abweichungen:
1. Self-ID 1 vs. Remote ≥2 (Vanilla gleiche ID).
2. Stage1-Reihenfolge FerrumC-eigen; nur JoinGame-zuerst gebunden.
3. Keine Java-Join/Quit/SpawnLocation-Events im Nativpfad.
4. Carried-Item wird beim Leave nicht gedroppt (DUR-05).
5. Regionsgrenze und Restore-Refusal statt Spawn-Reset.
6. **TabRemove-/TabAdd-Audience (H3).** FerrumC sendet TabRemove nur an
   Viewer mit Baseline (TabAdd ebenso nur `within_view`, `router.rs:635-748`);
   Vanilla `broadcastAll` an alle Spieler, Paper an alle mit `canSee`.
   Angeglichen ist nur die Reihenfolge EntityRemove vor TabRemove; die Audience
   bleibt eine bekannte Abweichung, ein späteres Ticket muss sie gemeinsam mit
   TabAdd angleichen.
7. Kein Checkpoint-Wiring über Neustart (§6.5).
8. **Neu:** Shutdown mit vielen Items kann deutlich länger dauern als bei
   Paper, weil höchstens eine Entity-Save zugleich läuft (DUR-04.2 Z. 339-342,
   §10.4); Grenze und Messung sind DUR-05b-Auflage.

## 13. Ticketfolge (eine Production-Crate je Ticket) mit Auflagen

| Ticket | Crate | Voraussetzung | Inhalt | Gebundene Auflagen |
|---|---|---|---|---|
| NET-04.2e1 | ferrumc-session | .2c CLEAN | .2c-Kern + native Endpointart, Lane (`into_parts`), Exklusivität, Replacement/Unregister, Suspension, **Revalidierung per Closure (.2c-Ergänzung R4)**, `native_lane_input_live`, Regionsvorprüfung, INVARIANTS | A3, A10 (Regionsableitung), H4, R2-A2 (Signatur), R2-A5 (Session-Teil) |
| SIM-03.4b | ferrumc-sim | SIM-03.4a, DUR-04.2a | `saved_after_detach`, `detached_owned_player_status` (Getter-Typ), `release_detached_owned_player(original)`, `ReleasedOwnedPlayer` ohne Autorität, `owned_stop_ready`, Admissionsbit `lifecycle_acked`, **Re-Export `OwnershipTransferId`/`OwnershipReceiptQuery`** | A7, G5, R3-1, R3-2, R2-A4, R2-H3, R2-H5, R2-H6 |
| NET-04.2e2 | ferrumc-session | e1, **SIM-03.4b** | Join (admit, Reservation mit Tabelleneintrag), Active-Übergang, Retention-Record, Lineage-Vergabe, Leave-Save-Zustandsmaschine inkl. R2-A1, `service/release_native_retention`, Detached-Intents, `adopt/retire_unbound`, R1/R2-Retirement, Audience/Reconcile, Pose/Resync | A2, A5 (Session-Teil), A6, B2, B3, R3-3, R2-A1, R2-A3 (Lineage) |
| DUR-03.3 | ferrumc-storage | läuft | Format7-Runtime inkl. Open-API | — |
| DUR-03.3a | ferrumc-storage (+ App-CLI-Ticket) | DUR-03.3 | Offline Format6→7 | — |
| DUR-05a | ferrumc-app | DUR-03.3/3a, e1, SIM-03.4b | Format7-Open + Formatgate, Init-Reihenfolge §10.2 inkl. Counter-Modus und Vorreservierungen, `NativeShardSlot`, `NativeTickClock`, `run_native` mit `_at`/Recovery-Disposition und **Recovery-Modus K/F_max/Degraded**, **`NonRecoveryGate`** (G ≤ 16, Reserve 16), `NativeOperationIds`, Save-ID-Refill, Work-Executor mit **`OutstandingWork` und Drop-Disposition**, Klasse 1, Persistenz-/Stop-Barriere, **Stop A/B/C mit Phase-B-Drain** ohne Spieler, Source-Scan §4.5 (1)–(8) | A1 (Klasse 1), A8, A9, B1, N1, N2, N3 |
| NET-04.2f | ferrumc-app | DUR-05a, e2 | Permits/Permit-Epoche, NativeJoin/Leave, **L2-Token-Tabelle vorreserviert**, gated Klasse 2/3 über das Gate (Join ≤2, Retention ≤4, Lane ≤16−G), `held` mit Key, Retention-Service, Releases, Detached-Intent-Routing, Revalidierungsaufruf, Join-Kit ohne Connectioninventar, Regionschnitt, Teardown, Stop Phase A mit Sessions | A1 (gated), A4, A5 (Driver-Teil), B2 (Driver-Teil), B3, R2-A3, R2-A5 |
| NET-04.2b | ferrumc-session | e2 | Itemconsumer (unverändert .2c) | — |
| DUR-05b… | ferrumc-app | f | Inventory/Block/Command-Consumer, Checkpoint-Wiring, **Messung Save-Latenz/Stopdauer, Festlegung `B_max`** | A9 (Folge), R2-A6 |
| ACC | ferrumc-testkit | alle | echter Client, Paper-Vergleich, Restart (Primary-Crate testkit; Paper-Vergleichswerkzeuge ggf. eigenes Ticket) | R2-H7 |
| (später) | sim/session | — | Multi-Shard/Transfer; TabAdd/TabRemove-Audience | H3 |

Abhängigkeitsgraph (zyklenfrei): .2c → e1 → e2 → {NET-04.2b, f};
SIM-03.4a, DUR-04.2a → SIM-03.4b → {e2, DUR-05a}; DUR-03.3 → DUR-03.3a → DUR-05a;
e1 → DUR-05a; DUR-05a, e2 → f → DUR-05b → ACC.
Live-Aktivierung zusätzlich nach SIM-03.4b und ACC.

## 14. Meaningful Abnahmematrix

Alle Tests mit echten bounded Routerkanälen, echter Memory- **und**
redb-Ownershipadmission im Counter-Modus, echter Work/Completion, echten
Decodern; keine Fake-Facts oder Testbinder. Getrennt: isolierte Contracttests je
Crate, App-Integration mit `FakeClient`, echter Client und Paper-Vergleich (ACC).

| Fall | Erwartung | Ticket |
|---|---|---|
| Native Registrierung; Legacy-Exact im selben Scope | `EndpointKindConflict`; Legacy-Route auf native → Input intakt | e1 |
| Replacement mit alter Lease / ohne aktuellen Receiver | verweigert, Endpoint unverändert | e1 |
| Replacement korrekt mit Active- und Leaving-Session | alle suspendiert; alte Items vollständig vor neuen; **Revalidierung per Closure** (Active + Leaving mit View, Detached ohne View bleibt suspendiert); Disconnect während Suspension retained und danach zuerst gesendet; falsche View → `NativeEndpointRevalidationRefused` | e1/f |
| Unregister mit Session/Reservation/Record/adoptierter Admission | `NativeEndpointInUse` | e1 |
| Lane voll / Sim-Enqueue verweigert | kein Verlust, `held` ≤1 **mit Key**, FIFO je Session; Session inzwischen detached → `held` verworfen | e1/f |
| **Bewegungsspam (32 Spieler) + Leave-Save-Completion** | Completion im nächsten Boundary vor jedem gated Input; Lane ≤ 16 − G | DUR-05a/f |
| **N1 (a): 16 gated Inputs liegen, Recovery-Modus** | `G` bleibt 16; ≥16 Klasse-1-Plätze; Completion/Reconcile eingereiht und angewandt; Normal-Versuch nach Fortschritt bzw. nach K | DUR-05a/f |
| **N1 (b): `AllocationFailed`/`Capacity("counter plan")` ohne Pending** (Testinjektion bzw. gesenktes `plan_entries`) | Normal-Versuch alle K Recovery-Boundaries; nach F_max `Degraded`, Joins `NativeShardDegraded`; Ursache entfällt → `Ok` binnen K+1 Iterationen | DUR-05a |
| **N1: Belegungseigenschaft** unter zufälliger Last (Lane, Joins, Retention, Fehler) | nie `G > 16`, nie Join > 2 in der Queue, Klasse-1-Bytes ≥ 6 283 264 | DUR-05a/f |
| Foreign Owner, gleiche UUID anderer Router, Snapshot, stale Admission | `StaleHandle`/Foreign, keine Mutation | f |
| **Counter-Init**: Reihenfolge §10.2; zweiter Init | `AlreadyInitialized`; Refill vor erstem Boundary; Vorreservierungen fallibel | DUR-05a |
| **Tick**: 40 Normal-Boundaries mit Delay40-Item; Recovery dazwischen | Delay 0 nach genau 40 Normal-Boundaries; Recovery zählt keinen Tick | DUR-05a |
| **Falscher Tick** (Testinjektion im Driver) | `CounterTickMismatch` → Fail-Stop, Queue intakt | DUR-05a |
| **CounterRecoveryRequired** | Recovery-Modus, Completion angewandt, danach derselbe Tick erfolgreich | DUR-05a |
| **SaveIdsNeeded** mit leerem Pool | genau ein Refill in flight; Dirty bleibt bis Save; kein zweiter Refill im selben Boundary | DUR-05a |
| **Refill mit fremdem/stale Counter-Handle** | dreimal `SaveIdsNeeded` im anwendenden Boundary → `NativeRefillRefused` Fail-Stop; Source-Scan verweigert Konstruktion aus Spieler-Handle | DUR-05a |
| **N3: Busy → Drop → Retry** | Completion-Refusal verworfen, nie wieder eingereiht; Checkpoint Prepared; `retry` nicht `Reserved`; zweite Completion bestätigt | DUR-05a |
| **N3: Uncertain / ReceiptAbsent / Confirmed{Err}** | verworfen; `reconcile`; Klärungs-Completion löst auf bzw. bleibt Uncertain mit Ratenbegrenzung | DUR-05a |
| **N3: CasConflict / confirm_current-Fehler** | verworfen; Status `Quarantined`; kein Retry/Reconcile; Stop → `Unresolved` | DUR-05a |
| **N3: definitive Refusal** | verworfen; Pending weg; Item bleibt dirty und wird neu gewählt; kein Requeue | DUR-05a |
| **N3: Sim-Queue voll beim Enqueue einer Completion** | Completion in `held_completions`, Lease beim Driver, später eingereiht (DUR-04.1 Z. 563-565) | DUR-05a |
| **N3: Source-Scan** | `take_refusals()` nur an `dispose_apply_refusals`, dort kein `enqueue_`; `held_completions` nur aus Enqueue-`Err` | DUR-05a |
| Work-Refusal falscher Store / verlorene Antwort | Fail-Stop mit gehaltener Work / `Idle` → Reconcile | DUR-05a |
| Join: Restore, Rejoin, Bootstrap (Survival 0, Seedbits, Pose-Bits) | JoinGame erstes Outbound, Stage1 vor Chunks, Teleport-ID 1; kein Connectioninventar, kein SetGameMode, kein Modespiegel | f (A4) |
| Join-Apply-Refusal (Restore `Reserved`, Bootstrap `Capacity`) | kein `PlayerAdmitted` bzw. keine Work mit Bootstrap-ID → `cancel_native_join`, Tabelleneintrag frei | f |
| Bootstrap Busy / Uncertain / Abort / Confirm nach Verbindungsverlust | Retry / Retention / Cancel / Adoption → G3 | f |
| **G3 (a)**: Restore- bzw. Rejoin-Admission mit geschlossener Reply | Adoption, G3-Kette, L1 nach Save, Release | e2/f |
| **G3**: Connection endet zwischen Schritt 5 und 10 | `reply_closed`, Adoption bzw. Cancel | f |
| **G3 (b)**: späte Events einer nie gebundenen Connection | `NoNativeSession`, keine Sim-/Recordwirkung | f |
| **G3 (c)**: Rejoin derselben UUID während G3-Retirement (vor D9 / nach D9 vor Release) | `AlreadyReserved` / Rejoin-Zweig, alter Record `Superseded`, alter Release `StaleHandle` | e2/f |
| **G3 (d)**: Reply-Send-Fehler in Schritt 10 | Session Active, normaler Leave D1–D10 | f |
| **Adoption kapazitätsunfehlbar**: Sessions-Limit voll ausgeschöpft durch Reservationen | Adoption und Token-Eintrag ohne Allokation (Tabelle vorreserviert); Abbruch vor Admission gibt Eintrag frei | e2/f |
| **Adoption-Invariantenbruch** (Testinjektion: falsche View) | `AdoptionInvariant` fatal, Eintrag `Poisoned`, Fail-Stop, `Unresolved` mit UUID, L1 quarantiniert | e2/f |
| Stop mit `PendingNativeJoin` | `ShuttingDown`; eingereihter Join in Phase A angewandt (→ Adoption → G3) oder verweigert (→ Cancel) | f |
| Duplicate UUID / Rejoin mit alter Connection | `AlreadyReserved`; alte Events `StaleSession` | f |
| Bewegung über Regionsgrenze; Position auf Chunkgrenze ±ε; Restore außerhalb; Spawn außerhalb | Korrektur+Action-Bar, Session- und Sim-Entscheidung identisch / Disconnect / Startrefusal | e1/f |
| Disconnect mit reserved Pickup → Confirm → Retired(proof) → Headack → Lifecycle-Ack | Pins exakt 0, Ticket genau einmal, `lifecycle_acked` gesetzt, Record bleibt bis Release | e2/b |
| **R2-A1: Leave-Save, Status `Due` + fremdes `blocking`** | → `Waiting` ohne Versuch → Pickup-Confirm → Intent → `Due` → Leave-Save bestätigt → L1 frei | e2/f |
| **Leave-Save bei Busy-Pickup** | Pickup Busy → Drop → Retry (N3) → bestätigt → `Due` → Leave-Save bestätigt | e2/f |
| **Leave-Save blockiert durch dauerhaft Uncertain-Pickup** | Versuche zählen; nach 8 Quarantäne; UUID `Quarantined`; L2/Sim-Platz bleiben; Stop Phase A Final-Save | e2/f |
| Leave-Save Busy / Uncertain / CasConflict / Capacity | Retry / Klärung, je 100 Boundaries ein Versuch / sofortige Quarantäne / Wiedereinreichung nach 20 | e2/f |
| **Status-Closure-Fehler** (`Ineligible` connected, `StaleHandle`, Foreign) | `AwaitDetach` + Disconnect-Wiederholung ≤8 / `Quarantined{StatusInvariant}` / Fail-Stop | e2/f |
| **Detached `PlayerChanged`/`Resync`** | kein Driverstopp, Session-Trigger; fremdes Handle → verworfen | f (A5) |
| Rejoin vor Save-Bestätigung | L1 `AlreadyReserved`; Sim zusätzlich `Reserved` | f |
| Rejoin nach Save, vor Release | Rejoin-Zweig, neue Admission, alter Record `Superseded`, späterer Release alt → `StaleHandle` | e2/f |
| Rejoin gegen Release (Permit zuerst / Release zuerst) | Release verweigert / Permit stale (Permit-Epoche), danach Restore-Permit | f |
| Restore nach Release mit vorher gelesenem Stand | `StaleSnapshot` vor Simwirkung | f |
| **Release-Refusals (A7)**: verbunden; Zelle nicht acked; Lifecycle nicht initialisiert; kein `saved_after_detach`; Pending/Bootstrap/Restore/Queue-Input derselben PlayerId; Publication-Pin; Item-/Chunkreservierung; Collector-Bindung; Snapshot-, Foreign-, Stale-Handle; wiederholter Release | jeweils typisiert, keine Teilmutation | SIM-03.4b |
| **`saved_after_detach`**: zwei Retries, zweiter bestätigt; späteres Pending | zuletzt bestätigte gilt; neues Pending setzt zurück, Release blockiert | SIM-03.4b |
| **`detached_owned_player_status`** mit verbundenem/Stale/Snapshot-Handle | typisierte Refusal, keine Allokation | SIM-03.4b |
| **Re-Export (R2-A4)** | Session kompiliert ohne Storage-Dependency gegen `ferrumc_sim::OwnershipTransferId`/`OwnershipReceiptQuery` | SIM-03.4b/e2 |
| **`owned_stop_ready`** vs. tatsächlicher Stop | true ⇔ Stop als einziges Input im nächsten Normal-Boundary angenommen | SIM-03.4b/DUR-05a |
| **N2 (a): Stop mit 4096 residenten Items**, Drain in Phase A, weitere Normal-Boundaries vor B | Phase-B-Drain → alle Slots priorisiert → `owned_stop_ready` ohne Counterstep → Phase C ein Boundary → `Stopped`; Neustart Restore liefert gespeicherten Stand | DUR-05a/f |
| **N2 (a) Negativkontrolle** ohne Phase-B-Drain (Testvariante) | nach erneut gesteptem Save kein Fortschritt, `owned_stop_ready` bleibt false → belegt, dass der Phase-B-Drain nötig ist | DUR-05a |
| **N2 (b): gated Input in Phase B** (Retention-Submit, Join, Lane) | `NativeShutdownPhase`, Sim-Queue unverändert; Source-Scan (6) | DUR-05a/f |
| **N2 (c): Phase C** | nur Stop eingereiht; eintreffende Completion verworfen; `_at`-Fehler → Wiederholung → `Stopped`; injizierte Stop-Verweigerung → Phase B mit neuem Drain, ≤3 Anläufe | DUR-05a |
| **R2-A6: Phase-B-Grenze** | `B_max = 12 000 + 8·N_B` aus `entities().len()`; Wachhund 1200 ohne Fortschritt → `Unresolved` | DUR-05a/b |
| **Stop mit vor Drain admittiertem Pickup** | Completion in Phase B, Projektion angewandt, Menge erhalten, Priorität bleibt (Invariante P) | DUR-05a/f |
| **Stop-Phasenablauf** (Uncertain bleibt) | `Unresolved`, UUIDs quarantiniert, kein `Stopped` | DUR-05a/f |
| **Terminal `GenerationExhausted`** (Testinjektion intern) | `Degraded{terminal}`, keine Normal-Versuche, `Unresolved` | DUR-05a |
| 129. distinkte UUID ohne SIM-03.4b | `Capacity("player count")` | f |
| Format4/6 bei nativer Konfiguration | Startrefusal vor Bind | DUR-05a |
| **Kein Legacy-Writer im Nativpfad** (Source-Scan + Laufzeit: Chunk-Release) | kein `release_chunks_acked`/Overlay-/Journal-Write; Format7 würde verweigern | DUR-05a (A8) |
| Neustart nach sauberem Leave | Restore aus Storage, bitgleiche Pose/Inventar | f/ACC |
| Echter Client (ACC) | Join, Bewegung, Grenze, Leave, Rejoin, Restart; Paper-Vergleich getrennt | ACC |

Danach je Ticket Debug/Release, strict Clippy, fmt, rustdoc `-Dwarnings`,
exakter Freeze und unabhängiger Rootreview.

## 15. Offene Einwände und Grenzen

1. WorldBorder-Sichtbarkeit fehlt (Proto-Generator); nur Korrektur + Action-Bar.
2. Restore-Aktualität: durch Post-Commit-Current-Read gedeckt; redb-MVCC-
   Monotonie neuer Lesetransaktionen bleibt Bibliotheksannahme.
3. Stage1-Paketreihenfolge FerrumC-eigen; TabAdd/TabRemove-Audience abweichend (H3).
4. Native Block-/Command-/Container-/Itemaktionen bis DUR-05b verweigert.
5. Java-Join/Quit/SpawnLocation-Events fehlen im Nativpfad.
6. Alle in §11 als „nicht gemessen“ markierten Werte sind gewählte Grenzen.
7. Terminale Counter-/Generationsgrenze: kein `_at` mehr möglich, Disconnect/
   Autosave unangewendet, Stop nie quiescent → `Unresolved` (§6.1, §10.4);
   praktisch nicht erreichbar, nicht persistiert.
8. Kein dauerhaftes Checkpoint-Wiring (DUR-05-Folge).
9. DUR-03.3-Format7-APIs stammen aus dem CLEAN-Vertrag; die Storage-Runtime ist
   noch nicht abgenommen.
10. Keine Speicher-/RSS-/Performanceaussage.
11. **Neu (N3, §2.15):** Apply-Refusals sind ohne Sim-Accessor nicht einzeln
    zuordenbar. Der Vertrag leitet jede Folgeaktion aus öffentlichem Sim-Zustand
    ab (Checkpoint-Status, Pendingzahl, Intents, Work). Ein späterer rein
    lesender Accessor (z. B. Refusal-Art) würde Tests vereinfachen, ist aber für
    die Korrektheit nicht nötig und wird hier nicht verlangt.
12. **Neu (R2-A6):** Die Stopdauer mit vielen Items ist durch eine
    Entity-Operation zugleich bestimmt; mit 4096 Items im Modell 285 s bei
    einer Iteration Save-Latenz, bis `B_max` ≈ 37 min. Messung und endgültige
    Grenze sind DUR-05b-Auflage.
13. **Neu (N1):** `Degraded` hält den Shard ohne Gameplay-Fortschritt, bis die
    Ursache entfällt oder ein Stop typisiert endet; eine automatische Ablösung
    eines dauerhaft degradierten Shards ist nicht Teil dieses Vertrags.

## 16. Evidenz

Runde 4 unter `.codex-tmp/net04_2d/round4/`: `pins.py` (alle 137 Runde-3-Anker
unverändert plus neue Anker für N1–N3, R2-A*, R2-H*), `freeze.py`,
`baseline.verify.json`, `producer-pins.json`, `reference-pins.json`,
`forbidden-scan.json`, `citation_coverage.py` und `citation-coverage.json`
(jede `datei.rs:Zeile`-Angabe liegt in einem Anker), `n2_stop_model.py` und
`n2_stop_model.json` (Quellmodell
des Stop-Ablaufs, keine Runtime), `n2_stop_walkthrough.md` (Schritt-für-Schritt-
Rechnung mit 4096 Items gegen den Code), `EVIDENCE.md`. Der Freeze
`.codex-tmp/net04_2d/design.freeze.json` pinnt diesen Brief, die Runde-4-
Evidenz, die unveränderte Runde-3- und Runde-2-Evidenz sowie die Kopien beider
Vorkandidaten (`round3-candidate/`, `round2-candidate/`); er pinnt sich nicht
selbst.

## 17. Änderungen Runde 3

| Befund | Geänderte Stelle | Änderung |
|---|---|---|
| **B1** Counter-Modus | §2.8–2.11, §3, §4.3 (`counter_owner`, `clock`), §4.5, §6.1, §6.4, §10.2, §10.4, §13 (DUR-05a), §14 | Counter-Modus Pflicht; Init mit `initialize_owned_item_counters(…, Tick::ZERO)`; jeder Boundary `run_tick_with_player_ownership_at` mit `NativeTickClock`, Legacy-Methoden per Source-Scan unerreichbar; Recovery-Boundary und Fehlerdisposition (`CounterRecoveryRequired` u. a.); `SaveIdsNeeded` → Refill aus `NativeOperationIds`, ≤1 Refill in flight; `refill_item_save_ids` nur aus `counter_owner`; Stop A/B/C |
| **B2** Producer Leave-Save/Release | §4.5, §7.1–7.5, §8, §13 (e2, f, SIM-03.4b) | Einziger Producer Session; Retention-Record mit Original-Handle bis Release; Driver nur `NativeRetentionToken`; Leave-Save-Zustandsmaschine mit 8 Versuchen, typisierte Quarantäne; Pickup-`Reserved`-Ablauf; Release über Session-Closure; SIM-03.4b ohne Caller-Query |
| **B3** G3-Matrix | §8 Race-Regeln, §14 Zeilen „G3 (a)–(d)“, „Connection endet zwischen 5 und 10“, „Adoption …“, „Stop mit PendingNativeJoin“ | alle G3-Races mit Zeile |
| **A1** Completion vs. Lane | §6.2, §11, §14 | Completions zuerst + Lane-Anteil, Recovery-Lane 0 |
| **A2** Active-Übergang | §4.3 Erstbindung, §5 Schritt 10 | `admit_native_join` setzt Active/`ever_active` |
| **A3** EndpointSuspended | §4.3 Revalidierung/Routing | Active + Leaving revalidiert; Data typisiert, Disconnect retained |
| **A4** Keine zweite Inventarautorität | §5 (nach Tabelle), §14 Join-Zeile | Connectioninventar, Modespiegel, Legacy-Leave-Save entfallen; Test |
| **A5** Retry + detached Intents | §6.3, §7.3, §14 | begrenzte Retries/Quarantäne; detached `PlayerChanged`/`Resync` an Session |
| **A6** .2c-API-Abweichung | §3 Tabelle „.2c-Ergänzung“ R1–R3, G3 | gekennzeichnet; Aussage „unverändert“ korrigiert |
| **A7** SIM-03.4b | §7.5, §14 Release-Refusals | Verhältnis zum Boundary-Plan, Arbeitsgrenze 833, Refusal-Zeilen |
| **A8** Format7 | §10.1, §10.3, §14 | Open-API aus DUR-03.3, Persistenz- und Stop-Barriere, Legacy-Flush unerreichbar |
| **A9** Work-Executor | §6.5 | Refusal-Pfad, Klärung, kein Checkpoint-Wiring (DUR-05-Folge) |
| **A10** Region/Release | §9 Bewegung, §7.4 | identische Chunkableitung; „laufender Release“ gestrichen |
| **H1** | §1, §2.5, Pins | `apply_deferred` 1976-2029 gepinnt |
| **H2** | §4.4, §7.4, §11 | Werte als „nicht gemessen“ bzw. Herkunft gekennzeichnet; Reserve 16 |
| **H3** | §12 Abweichung 6, §13 | TabRemove-/TabAdd-Audience als Abweichung + späteres Ticket |
| **H4** | §4.1, §13 e1 | INVARIANTS-Anpassung in e1 |
| **R3-1/2/3** | §7.5, §8, §10.4, §11, §14 | Root-Annahmen der frühen Lückenmeldung mit Auflagen |

## 18. Änderungen Runde 4

| Befund | Geänderte Stelle | Änderung |
|---|---|---|
| **N1** Recovery-Selbstblockade | §2.9, §2.16, §3, §4.3 (`gate`, `recovery`), §4.5 Scan (6), §5 Schritt 2/7, §6.1, **§6.1a** (neu), **§6.2** (neu gefasst), §7.3 (`submit_allowed`), §11, §13 (DUR-05a, f), §14 Zeilen „N1 (a)“, „N1 (b)“, „Belegungseigenschaft“, §15.13 | Zwei Klassen nach Recovery-Anwendbarkeit; Belegung `G ≤ 16` für Lane + Join + Retention + Stop über `NonRecoveryGate`; feste Reserve 16 Plätze und ≥ 6 283 264 Bytes (≥4 maximale Completions); Join ≤2 in der Queue, Retention ≤4, Lane ≤16−G; im Recovery-Modus nur Klasse 1; Normal-Versuch nach Fortschritt oder spätestens nach K = 20 Recovery-Boundaries, unabhängig von Completions; F_max = 60 → typisiert `Degraded`; Terminierung je Ursache; Vereinbarkeit mit DUR-04.1/.2 und SIM-03.4a; alle Werte nicht gemessen |
| **N2** Stop-Quiescence | §2.9, §2.11, §3 (Ergebnis: keine neue Sim-Funktion), §4.5 Scan (6), §8 (Stop mit Join), **§10.4** (neu gefasst), §11, §12 Abweichung 8, §14 Zeilen „N2 (a)“, „Negativkontrolle“, „N2 (b)“, „N2 (c)“, „owned_stop_ready“ | Hergeleitet aus `counters.rs:212-231`, `794-805`, `896-909`, `1917-1923`, `2082-2113`, `entity/counters.rs:98-105`, `254-260`, `262-273`, `player_ownership.rs:395-410`, `567-588`: Phase-B-Drain als erstes Input von Phase B, Invariante P mit Beweis, Terminierung; ab Phase B Gate strukturell geschlossen (`NativeShutdownPhase`); Phase C nur `stop()` (`remaining == 0`), Retention/Lane/Join vorher abgeschlossen bzw. verweigert; `_at`-Fehler in C und Stop-Verweigerung typisiert; 4096-Item-Rechnung in `round4/n2_stop_walkthrough.md` und Modell `n2_stop_model.py/json` |
| **N3** abgelehnte Completions | §2.7, §2.14, **§2.15** (neu), §4.3 (`outstanding`, `held_completions`), §4.5 Scan (7)/(8), §5 Schritt 8, **§6.3** (Apply- vs. Enqueue-Refusals), §6.4 (Refill), **§6.5** (neu gefasst, Dispositionstabelle), §7.3 Punkt 5, §11, §14 Zeilen „N3: …“, §15.11 | Apply-Refusals werden immer verworfen, nie eingereiht (Lease frei, DUR-04.1 Z. 475-477, DUR-04.2 Z. 606-609); `held_completions` nur für Enqueue-Refusals (DUR-04.1 Z. 563-565); Disposition je Refusal-Art aus `player_ownership_checkpoint`: Prepared → Retry, Uncertain/NeedsRefresh/InFlight → Reconcile, Quarantined → keiner, kein Pending → nichts; Join-, Refill- und Lane-Dispositionen ohne Zuordnung aus Intents/Work/Status; §6.5 und die frühere Z. 453-454 widerspruchsfrei |
| **R2-A1** Due + fremdes Pending | §7.3 Punkte 1 und 3, Pickup-Absatz, §14 Zeilen „R2-A1“, „Status-Closure-Fehler“ | `Due` + `blocking = Some(fremd)` → `Waiting` ohne Versuch; typisierte Disposition der Status-Closure (`Ineligible` → `AwaitDetach` + begrenzte Disconnect-Wiederholung; `StaleHandle` → Quarantäne; Foreign/Snapshot → Fail-Stop) |
| **R2-A2** Revalidierung ohne Driver-Handle | §3 Tabelle (**.2c-Ergänzung R4**), §4.2, §4.3, §13 e1, §14 Replacement-Zeile | `revalidate_native_endpoints(&lane, &mut \|h\| view)`; Session iteriert ihre Handles, View nur im Closure |
| **R2-A3** Token-Tabelle/Lineage | §4.3 (`tokens`), §7.1 (Lineage durch Session), §7.2 D2, §7.4, §8, §10.2 Schritt 9, §11, §13 e2/f, §14 Adoption-Zeile | L2-Tabelle beim Start fallibel auf Sessions-Limit vorreserviert; Eintragen alloziert nie; Lineage vergibt die Session (checked u64), Permit-Epoche gehört getrennt dem Driver |
| **R2-A4** Storage-Typen in der Session-API | §7.1, §7.5 (Re-Export, Getter-Typ), §13 (SIM-03.4b-Inhalt, Kante **e2 → SIM-03.4b**, Graph), §14 Re-Export-Zeile | `pub use ferrumc_storage::{OwnershipReceiptQuery, OwnershipTransferId}` in ferrumc-sim (SIM-03.4b); keine Session→Storage-Dependency; e2 hängt von SIM-03.4b ab |
| **R2-A5** Rückweg abgelehnter Lane-Inputs | §4.2 (`into_parts`, `native_lane_input_live`), §4.3 (`HeldLaneInput`), §4.4, §6.3, §13 e1/f, §14 Lane-Zeile | `held` trägt Key und Klasse; Retry vor jedem `try_recv`; inaktive Session → verworfen |
| **R2-A6** Phase-B-Grenze vs. Save-Durchsatz | §2.16, §10.4 (Grenze, Rechnung), §11, §12 Abweichung 8, §13 DUR-05b, §14 Zeile „R2-A6“, §15.12 | `B_max = 12 000 + 8·N_B` plus Wachhund 1200 ohne Fortschritt; Modell zeigt die alte Grenze 12 000 ab L = 3 als zu knapp; Messung und Festlegung als DUR-05b-Auflage |
| **R2-H1** | §2.8, §6.1a (Erreichbarkeit), Pins | falsche Aussage zu `counters.rs:174` korrigiert (nur Vorabprüfung); Generation wächst über `player_ownership.rs:924-928`, `1718` |
| **R2-H2** | §2.14, §6.5 | Reconcile über `dispatch_retained(clarify = true)` mit freier Lease; `into_receipt_clarification` nur für unausgeführte Work |
| **R2-H3** | §7.5 (Verhältnis zum Plan) | Countershadow-Aussage ersetzt: Credits je Plan kopiert (`counters.rs:510`), Zelle beim Ack frei |
| **R2-H4** | §2.7, §6.5 Tabelle | CasConflict → Quarantined und definitive Refusal → `release_pending` + Resync aufgenommen |
| **R2-H5** | §7.5 (`ReleasedOwnedPlayer`) | Inhalt festgelegt: PlayerId und freigegebene Bytes, nur Getter, keine Autorität |
| **R2-H6** | §2.4, §7.2 D8, §7.5, §10.2 Schritt 5, §14 Release-Zeile | Lifecycle verpflichtend; Admissionsbit `lifecycle_acked` beim Ack; „nie eine Zelle“ ist typisierte Refusal |
| **R2-H7** | §13 ACC | Primary-Crate ferrumc-testkit; Paper-Vergleichswerkzeuge ggf. eigenes Ticket |
