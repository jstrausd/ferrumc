# NET-04.2e2 — Delivery: nativer Session-Lifecycle

Status: **Implementierung abgeschlossen, Checks grün, wartet auf unabhängigen
Root-Review.** 12.10.2026. Implementierungs-Subagent, exklusiver Owner von
`server/crates/ferrumc-session/**`. Keine Aktivierungs-, Survival-,
Performance-, Paper- oder Live-Client-Aussage; es gibt in diesem Ticket keinen
App-Driver, keinen echten Client und keine Messung.

## 1. Basis, Pins, Schreibscope

| Gegenstand | Wert |
|---|---|
| Basis | `main` `e18efd22c9e062190b023575ac53fb0da04c2eb8`, Archiv `.codex-tmp/net04_2e2-root-base/archive`, 1325 Dateien, `base.freeze.json` SHA-256 `2deaad94edf10780dd4f5431631408f4a883410257565058cd4c5d889b0eb3f3` (frisch geprüft 1325/1325, readonly) |
| Freigabe | `NET-04_2e2_NATIVE_SESSION_LIFECYCLE_TASK.md` |
| Vertrag | `NET-04_2d_NATIVE_DRIVER_ENDPOINT.md` `e0530e0a…` (§4–§8, §13 e2-Zeile, §14 e2-Zeilen) |
| Root-Entscheidungen | `…_ROOT_DECISIONS.md` (R3-A1 Form), e1 F1–F4, SIM-03.4b-Integrationsvermerk (H1, H3), Root-Antwort auf die Frühmeldung (1, 2(a)–(e)) |
| Arbeitskopie | `.codex-tmp/net04_2e2-runtime/work` (Archiv nie bearbeitet), Dev-Target `target-dev` |
| Checks | eigene Prüfkopie (Archiv + Kandidat) im Ordner `.codex-tmp/net04_2e2-runtime/`, frisches `target-final` |

Geändert wurden ausschließlich `server/crates/ferrumc-session/**` (14 Dateien
geändert, 8 neu, 0 entfernt), dazu `.codex-tmp/net04_2e2-runtime/**` und dieses
Dokument. `Cargo.lock`, `Cargo.toml` und jedes andere Crate sind bytegleich zur
Basis. Kein Git, keine weiteren Agents, keine Dependency.

## 2. Frühmeldung und Root-Antwort

Vor der Implementierung habe ich dem Root eine Lücke außerhalb der Session
gemeldet (Held-Slot-Resync ohne Sim-Producer) und fünf Entscheidungen
vorgelegt. Root hat alle angenommen, (b) und (d) mit Auflagen. Umsetzung:

| Root | Umsetzung |
|---|---|
| 1 Held-Slot-Resync offen | §8, Punkt 1 |
| 2(a) Lifecycle-Closures in F1-Probe-Form, D7/D8 eine Closure | `NativeLifecycleProbe::submit` → versiegeltes `NativeLifecycleVerdict`, das das owned Ticket hält; `NativeLifecycleVerdict::acknowledge(|t| shard.acknowledge_collector_lifecycle(t))` leiht es genau einmal. `compile_fail`: Vertragsform E0597, Probe doppelt E0382, Probe-Flucht E0521, Verdict-Fälschung (private Felder), Ticket-Schmuggel E0521 (`evidence/compile_fail.json`). Positiver Nachweis: Doctest (`no_run`, kompiliert) und `native_borrow::lifecycle_closures_take_shared_and_exclusive_shard_borrows_beside_the_lane` (Memory+redb, echter Slot mit `shard`+`lane`, geteilter Borrow für D3/Status, exklusiver für D7/D8 und Release neben lebendem `&slot.lane`) |
| 2(b) Teleport-ID in der App, kein Direktversand mit ID 0 | Der Router erzeugt `SynchronizePlayerPosition` nur in `send_correction` (`router/native.rs:1415`) und legt sie ausschließlich in den Outbound der Session; die App stempelt jede Sync aus diesem Outbound in `mirror_outbound` (`app/src/connection/play.rs:1145-1154`, `pending_teleport.issue`) und hält Client-Positionen bis zum Confirm zurück (`handlers.rs:152-159`). Die 0 ist Platzhalter, kein Versandwert. Auflage für f in §8 |
| 2(c) Stale und Foreign → Quarantäne | Status-Closure `StaleHandle` (Sim meldet Foreign ebenso) → `Quarantined{StatusInvariant}` (`native/retention.rs` `step`) |
| 2(d) Refused-Revalidierung | §3 A2 |
| 2(e) `disconnect_native` → Token | §3 A5; e1-Testdiff §6.3 |

## 3. Auflagen der Freigabe

- **A1 (R3-3, §5 Schritt 6).** Eine einheitliche Tabelle `NativeTable`
  (`router/native.rs:96ff.`) ersetzt die zwei infalliblen BTreeMaps aus e1.
  `reserve_native_join` alloziert den Eintrag fallibel (`reserve_slot`,
  `try_reserve(1)`, freigegebene Slots zuerst) **vor** dem Verbrennen von
  Inkarnation und Wire-ID; `AllocationFailed` liegt vor jeder Wirkung. Admission,
  `disconnect_native`, Retirement (Session → Record) und G3-Adoption ersetzen den
  Zustand im selben Slot; keine spätere Transition alloziert oder scheitert an
  Kapazität. Reservationen, Sessions, Records und Poisoned zählen gemeinsam ins
  Sessions-Limit. Belege: `table_entries_are_taken_over_in_place_without_growth`
  (intern: Länge und Kapazität des Vektors über alle Übergänge konstant, Slot-
  Wiederverwendung), `adoption_is_capacity_infallible_with_the_session_limit_exhausted_by_reservations`
  (Limit 3: N-1/N gelingen, N+1 `SessionCapacity`, Cancel gibt frei, alle drei
  Adoptionen gelingen bei voll ausgeschöpftem Limit).
- **A2.** Eine verweigerte Revalidierung bindet nie um: die Session wird
  `RevalidationRefused`/`Leaving`, und ihr Disconnect aus dem **gespeicherten**
  Handle geht über die aktuelle Lane (`Suspension::Refused` darf nur Control
  senden). Wendet der Sim-Owner ihn an, folgt der normale Detach-Pfad; lehnt er
  beim Apply ab (`StaleHandle`), liefert die Status-Closure `StaleHandle` →
  `Quarantined{StatusInvariant}`, kein erneutes Senden, keine Umbindung.
  Revalidiert-als-detached hält keinen Disconnect mehr (`NativeDisconnect::NotNeeded`).
  `retry_native_disconnects()` zählt nur noch gebundene/refused Sessions mit
  wirklich zurückgehaltenem Disconnect und alloziert nicht mehr (H3 aus e1).
  Belege: `refused_revalidation_disconnect_ends_in_detach_or_typed_quarantine_never_rebound`
  (beide Ausgänge, Memory+redb, retry = 0, zweite Revalidierung ohne Arbeit),
  e1-Endpointtest (E jetzt `Sent`, retry 0, E detacht genuin).
- **A3 (teilweise, siehe §8 Punkt 1).** Pose-Producer `native_player_changed`/
  `native_resync` (`router/native/audience.rs:90-165`) übernehmen Position, Look
  und Mode nur aus einer echten `ConnectedOwnedPlayerView` der gespeicherten
  Admission (Owner, Spieler, Admission, volle Generation). Korrekturen nutzen
  immer die jüngste solche Pose. Gegen veraltete Korrekturen: Ist nach einer
  Korrektur/einem Resync noch keine neuere Client-Bewegung geroutet und liefert
  eine echte Pose einen anderen Stand (ältere Lane-Items nach der Korrektur
  angewandt), folgt genau ein mandatory Follow-up-Sync. Mode-Wechsel → mandatory
  `GameEvent(3, mode)`. Beleg: `pose_producer_uses_the_current_sim_pose_with_follow_up_sync_and_mode_events`.
  **Held-Slot-Resync: offen** (kein Sim-Producer).
- **A4.** Nach bestätigtem DetachFact liefert `native_lane_input_live(Control)`
  `false` (Bound-Chain-Test, e1-Endpointtest für E).
- **A5.** `disconnect_native` liefert `NativeRetentionToken` (idempotent, gleicher
  Token); Lineage aus checked `NonZeroU64`-Zähler, Erschöpfung vor jeder Wirkung
  (`exhausted_lineage_refuses_before_any_effect_and_poisons_an_adoption`).
- **A6 (Session-Teil).** Siehe Root 2(b). Die Vergabe bleibt in der App, wie der
  Vertrag sie über `PendingTeleport` verortet (§5 Schritt 11) und `play.rs:1154`
  sie heute ausführt.
- **A7.** Decision-Log §5.
- **H1 (SIM-03.4b).** `session_api_works_against_the_sim_reexports_without_a_storage_type`:
  die Session-API nimmt die Leave-Save-ID als `ferrumc_sim::OwnershipTransferId`,
  die Sim meldet dieselbe ID, die echte Work-Query ist ein
  `ferrumc_sim::OwnershipReceiptQuery`. Produktion nennt Storage nicht
  (`source_scan`, Compiler: Storage nur Dev-Dependency).
- **Leave-Save-Zustandsmaschine (§7.3, R2-A1).** `native/retention.rs`:
  8 Versuche, 8 Disconnect-Wiederholungen, 20 Boundaries Wiedereinreichung,
  100 Boundaries Zählintervall, 1200 Boundaries Wartezeit, alles nur über
  `applied`. Fremdes `blocking` → `Waiting` ohne Versuch; Reserved-Refusal
  durch fremdes Pending nimmt den Versuch zurück; eigenes `Quarantined` →
  sofortige Quarantäne; Enqueue-Refusal → `native_retention_unsent`, Versuch
  zurück. Quarantäne ist typisiert (`LeaveSaveQuarantine{player, reason, attempts}`),
  eine spätere echte Bestätigung erlaubt trotzdem den Release; Stop-Phase-A-
  Final-Save über `renew_native_leave_save` genau einmal.
- **Pickup in Reserved wartet ohne Versuchszählung:** `r2_a1_due_with_a_foreign_pickup_waits_without_attempts_then_saves`.
- **Release** nur über `detached_owned_player_status` (Service) und
  `release_detached_owned_player(original)` im Release-Closure; `lifecycle_acked`
  über `acknowledge_collector_lifecycle` in D8.
- **Driver hält nur ein opakes Token** (`NativeRetentionToken`: Copy, PlayerId +
  Lineage, kein Handle, keine Query).

## 4. Öffentliche API (neu oder geändert)

Neu: `disconnect_native → NativeRetentionToken`, `native_retention_status`,
`native_detached`, `native_detached_intent`, `service_native_retention`,
`native_retention_unsent`, `renew_native_leave_save`,
`retire_detached_native_sessions`, `acknowledge_retired_native_records`,
`release_native_retention`, `adopt_unbound_native_admission`,
`retire_unbound_native_admission`, `native_player_changed`, `native_resync`,
`collector_delivery`, `native_record_count`, `native_poisoned`,
`native_baseline_count`; Typen `NativeRetentionToken`, `NativeRetentionStep`,
`NativeRetentionStatus`, `NativeRetentionPhase`, `NativeLeaveSaveState`,
`LeaveSaveQuarantine`, `LeaveSaveQuarantineReason`, `NativeLifecycleProbe`,
`NativeLifecycleVerdict`, `NativeDetachOutcome`, `NativeRetirementReport`,
`NativeAcknowledgeReport`, `NativePoseOutcome`, `AdoptionInvariant`,
`AdoptionInvariantKind`, `CollectorDeliveryDisposition`, `CollectorAudience`,
`CollectorViewerBinding`, `CollectorRetirementProof`; Konstanten
`MAX_LEAVE_SAVE_ATTEMPTS`, `MAX_DISCONNECT_RESENDS`, `LEAVE_SAVE_*`,
`MAX_NATIVE_EQUIPMENT_BYTES`; `CollectorSessionError`-Varianten (§5).
Alle öffentlichen Items haben Rustdoc, keine öffentlichen Felder, keine
öffentlichen Konstruktoren für Token, Verdict, Probe, Audience, Proof.

## 5. Decision-Log (Präzisierungen gegenüber dem Vertragstext)

Keine Regel wird abgeschwächt.

1. (A7, e1) `unregister_native_shard_registration` liefert bei Verweigerung den
   Receiver zurück (`NativeUnregisterError`); `PlayerSessionHandle::session_key()`
   ist `Option`; §6.6 ist als `route_native_play`/`route_native_game_mode`
   konkretisiert; die Revalidierungsverweigerung ist typisiert über
   `NativeEndpointState::RevalidationRefused` und `NativeRevalidation::refused`
   statt einer Fehlervariante.
2. Alle Lifecycle-Closures (`native_detached`, `retire_detached_native_sessions`,
   `acknowledge_retired_native_records`, `retire_unbound_native_admission`) in
   der F1-Probe-Form statt `FnMut(&H) -> Option<View<'_>>` (R3-A1, Root 2(a)).
3. `NativeRetentionStep` ist `#[non_exhaustive]` und hat zusätzlich
   `Superseded` (der Driver beendet dann das alte Token, §7.4).
4. `native_retention_unsent` und `renew_native_leave_save` liefern `Result`
   (`StaleRetention`, `NoRetentionSubmit`), statt still zu ignorieren.
5. `native_detached` liefert `NativeDetachOutcome` (Confirmed/NotDetached/
   NoRecord); `native_detached_intent` liefert das passende Token (`None` =
   stale, Metrik beim Driver).
6. `release_native_retention` verweigert typisiert vor dem Closure:
   `RetentionSuperseded`, `RetentionNotReleasable("not retired" |
   "lifecycle not acknowledged" | "leave-save not confirmed")`; Sim-Refusals
   werden `NativeReleaseStale`, `NativeReleaseRefused(DetachedReleaseRefusal)`,
   `NativeReleaseFailed`. Der Record bleibt bei jeder Verweigerung.
7. `NativeEndpointInUse.sessions` zählt jetzt Sessions, Retention-Records,
   adoptierte Admissions und Poisoned-Einträge (Feldtyp unverändert).
8. `NativeDisconnect::NotNeeded` (A2). Neue Fehler `ProbesExhausted` (statt der
   in e1 semantisch unpassenden `SessionIdsExhausted`, H4; die e1-Revalidierung
   bleibt unverändert), `LineageExhausted`, `NativeStatusInvariant`,
   `NativeLifecycleAckFailed`, `RetirementRefused`, `InvalidJoinMetadata`,
   `PeerBurstCapacity`, `BaselineCapacity`, `WorkCapacity`, `ScratchCapacity`,
   `AudienceCapacity`, `CollectorBindingUnavailable`, `CollectorAdmissionMismatch`,
   `CollectorNotCurrent`, `TrackingNotCurrent`.
9. H7 (e1): `admit_native_join` begrenzt `name` auf 1–16 Zeichen und `equipment`
   auf `MAX_NATIVE_EQUIPMENT_BYTES` = 64 KiB (gewählt, nicht gemessen).
10. Peer-Burst bei Admission: ein geprüfter Reconcile; Work-, Scratch-,
    Baseline- und Joiner-Outbound-Grenzen vor jedem Send; eine Verweigerung
    stellt die Reservation wieder her (Abbruch oder Adoption beim Driver).
11. Baselines: Tab-Eintrag bleibt beim Verlassen der Sicht, nur EntityRemove;
    Wiedereintritt nur Spawn. D5 sendet EntityRemove (falls gespawnt) und dann
    TabRemove. Die Audience-Abweichung H3 (nur Viewer mit Baseline) bleibt.
12. Konsumentenreferenzen (.2c) existieren erst mit NET-04.2b; in e2 sind sie 0,
    D7 prüft daher nur Retirement, Detach-Fakt und Ticket (Pins 0).
13. Stop-Phase-A-Final-Save als eigener Aufruf `renew_native_leave_save` (genau
    ein erneuter Versuch je Record).

## 6. Tests

### 6.1 e2-Matrixzeilen (§14)

Alle Tests laufen über echte bounded Router-Kanäle und eine echte native
Admission (Restore, Rejoin oder receipt-gestützter Bootstrap) im Counter-Modus
(Init-Reihenfolge §10.2, `run_tick_with_player_ownership_at`). Work und
Completion sind echt, ebenso jede Storage-Ablehnung (Receipt-Kapazität, CAS
eines zweiten Schreibers, Receipt-Kollision, verlorene Antwort mit
Reconcile). „M+R“ heißt: Der Test läuft in `for store in STORES` auf Memory
**und** redb.

| §14-Zeile (e2) | Test (Datei) | Backends |
|---|---|---|
| G3 (a): Restore- bzw. Rejoin-Admission mit geschlossener Reply → Adoption, G3-Kette, L1 nach Save, Release | `g3_closed_reply_restore_adoption_runs_the_unbound_chain_and_releases`, `g3_closed_reply_rejoin_adoption_supersedes_the_old_record` (`native_lifecycle.rs`) | M+R |
| G3 (c): Rejoin während G3-Retirement (vor D9 / nach D9 vor Release) | vor D9: `DuplicatePlayer` (Session-Seite; `AlreadyReserved` der L1 ist f) im Restore-G3-Test; nach D9: Rejoin-Zweig, alter Record `Superseded`, alter Release `RetentionSuperseded`, Sim-Release des alten Handles `StaleHandle` im Rejoin-G3-Test | M+R |
| Adoption kapazitätsunfehlbar (Limit durch Reservationen ausgeschöpft) | `adoption_is_capacity_infallible_with_the_session_limit_exhausted_by_reservations`; intern `table_entries_are_taken_over_in_place_without_growth` | M+R / Memory |
| Adoption-Invariantenbruch (falsche View) | `adoption_invariant_breaks_are_fatal_typed_and_poison_the_entry` (ViewMismatch → Poisoned, UUID blockiert, Sim-Spieler bleibt; PlayerMismatch; fremde Reservation); intern `exhausted_lineage_…` (LineageExhausted → Poisoned) | M+R |
| Disconnect mit reserved Pickup → Confirm → Retired(proof) → Headack → Lifecycle-Ack | `collector_delivery_tracks_the_audience_and_proves_retirement_after_a_reserved_pickup` (`native_audience.rs`): vor D5 `CollectorNotCurrent`, nach D5 `Retired(proof)`, ohne Headack kein Ticket, Headack → Pins 0, D7/D8 genau ein Ticket, Record bleibt bis Release, Release-Gate passiert (`lifecycle_acked`) | M+R |
| R2-A1: `Due` + fremdes `blocking` → `Waiting` ohne Versuch → Pickup-Confirm → Intent → `Due` → Leave-Save bestätigt | `r2_a1_due_with_a_foreign_pickup_waits_without_attempts_then_saves`; Variante Autosave: `foreign_autosave_confirmed_after_detach_is_the_leave_save` | M+R |
| Leave-Save bei Busy-Pickup | Session-Teil: fremdes `Prepared` (echte `HistoryCapacity`) wartet 1200 Boundaries ohne Versuch, dann je 100 einer (N-1/N/N+1 bei 1299/1300), Quarantäne nach 2100: `prepared_foreign_pending_counts_only_after_1200_boundaries_then_quarantines`; Auflösung → `Due` → Leave-Save wie R2-A1. Ein transientes Busy → Driver-Retry (N3) → Bestätigung ist f (siehe §8 Punkt 4) | M+R |
| Leave-Save blockiert durch dauerhaft Uncertain-Pickup | `uncertain_pickup_counts_every_100_quarantines_and_renews_one_final_save` (Versuche je 100, Quarantäne nach 900, L2/Sim-Platz bleiben, Final-Save genau einmal; mit `defer` = 1024, siehe §8 Punkt 3) | M+R |
| Leave-Save Busy / Uncertain / CasConflict / Capacity | eigenes InFlight bzw. Uncertain je 100: `own_inflight_leave_save_…`, `own_uncertain_leave_save_counts_every_100_boundaries`; CasConflict: `own_leave_save_cas_conflict_quarantines_immediately`; Capacity und definitive Refusal → Wiedereinreichung nach 20: `capacity_refusal_resubmits_after_20_and_an_enqueue_refusal_counts_nothing`, `definitive_leave_save_refusal_resubmits_after_20_applied_boundaries` | M+R |
| Status-Closure-Fehler (`Ineligible` connected, `StaleHandle`, Foreign) | `status_closure_refusals_are_typed`: 8 Resends, dann `DisconnectNotApplied` (9 Control-Items, retry 0); fremder Owner → `StatusInvariant`; `SnapshotOnly`/`NotInitialized` → `NativeStatusInvariant`, Record unverändert | M+R |
| Rejoin nach Save, vor Release | `rejoin_after_save_before_release_supersedes_and_the_old_release_is_stale` (Ack vor Rejoin: Record beim Supersede entfernt), Rejoin-G3-Test (Ack danach) | M+R |
| Re-Export (R2-A4) / H1 | `session_api_works_against_the_sim_reexports_without_a_storage_type` | M+R |

Weitere e2-relevante Zeilen anderer Tickets: Unregister mit Record/Adoption →
`unregister_refuses_while_records_adopted_or_poisoned_entries_exist` (deckt
nur den adoptierten Record ab, nicht einen Poisoned-Eintrag; der Name sagt mehr,
siehe §8 Punkt 7); Replacement mit Leaving-Session und Control nach DetachFact
(A4) → e1-Endpointtest und `bound_leave_runs_d2_to_d10_…`; D1–D10 gebunden →
`bound_leave_runs_d2_to_d10_and_releases_on_both_backends`.

### 6.2 Weitere getrennte Refusal-, N-1/N/N+1- und Reihenfolge-Tests

- **Audience/Reconcile** (`native_audience.rs`, alle M+R):
  `native_sessions_track_each_other_in_both_directions_with_reentry` (Eintritt
  mit Tab-Add + Spawn in beide Richtungen, Wire-IDs, droppable Bewegung, Austritt
  mit EntityRemove in beide Richtungen, Tab bleibt, Wiedereintritt nur Spawn,
  Viewer-Bewegung); `peer_burst_caps_refuse_the_admission_before_any_send_and_keep_the_reservation`
  (je N/N+1: Baselines je Viewer 1/2, gesamt 5/6, Work 64/65, Joiner-Outbound
  4/5; Verweigerung ohne Send, Reservation bleibt pending);
  `a_viewer_that_cannot_take_a_mandatory_spawn_is_set_leaving`;
  `collector_delivery_refusals_are_typed` (Audience-Limit 1, fremder Router).
- **Pose/Resync (A3)**: `pose_producer_uses_the_current_sim_pose_with_follow_up_sync_and_mode_events`
  (Korrektur auf aktuelle Sim-Pose, Sync-ID 0 als Platzhalter, Follow-up-Sync nach
  später angewandtem Lane-Item, kein zweiter Sync bei gleicher Pose, neue
  Bewegung schließt das Fenster, Mode → `GameEvent(3, 1.0)` mandatory, Resync
  mandatory, fremde Admission `Stale`).
- **Intern** (`router/native_tests.rs`, Memory): In-place-Tabelle,
  Lineage-Erschöpfung (MAX genau einmal, danach `LineageExhausted` vor jeder
  Wirkung, Adoption → Poisoned), Lifecycle-Verdict einer fremden Probe beweist
  nichts, erschöpfte Probe-Sequenz → `ProbesExhausted` vor dem Closure.
- **Borrow** (`native_borrow.rs`, M+R): D3/Status geteilt, D7/D8 und Release
  exklusiv neben `&slot.lane`.
- **compile_fail** (Doctests, zusätzlich eigenständig gegen die rlibs aus
  `target-final` kompiliert, `evidence/compile_fail.json` `5c2acf88…`):
  `lifecycle.rs` Z. 35 E0597, Z. 54 E0382, Z. 64 E0521, Z. 145 private Felder,
  Z. 154 E0521.
- **Reihenfolge-Races** (deterministisch, der Router ist single-threaded):
  Rejoin vor bzw. nach dem Lifecycle-Ack der alten Zelle; Detach-Fakt vor dem
  Leave-Save; Korrektur vor bzw. nach dem Anwenden älterer Lane-Items;
  Refused-Revalidierung mit echtem bzw. fremdem Owner. Ein nebenläufiger Race
  kommt in e2 nicht hinzu; der e1-Race-Test der Bewegungszelle bleibt
  unverändert.

### 6.3 Anpassungen bestehender e1-Tests

`evidence/e1-tests.diff` (`f082b41c…`) zeigt jede Änderung. Keine e1-Assertion
ist abgeschwächt; geändert ist nur, was A5, Root 2(d) und der e2-Peer-Burst
erzwingen:

- `disconnect_native` liefert das Token (A5): `Ok(NativeDisconnect::Sent/Retained)`
  wird zu `disconnect(&mut router, &key).1 == Some(Sent/Retained)` (Helfer in
  `native_support`, liest den Status). Die Idempotenz prüft jetzt zusätzlich den
  gleichen Token.
- `native_endpoint.rs` (Root 2(d)): E (Revalidierung verweigert) ist jetzt
  `Sent` statt `Retained`; zusätzlich `retry_native_disconnects() == 0`; die neue
  Lane führt C, dann E (Control), dann A; E wird beim Apply genuin detacht und
  `native_detached` bestätigt; die Detach-Prüfung umfasst jetzt `[1, 2, 3, 4]`.
  Der Peer-Burst nach den fünf Joins wird typgeprüft geleert (je 8 mandatory
  Tab-Add/Spawn).
- `native_join.rs`: Statt „no peer burst in e1“ wird genau der Peer-Burst (2
  mandatory Pakete) geprüft, danach nichts.
- `native_support/mod.rs`: `EnvLimits` (Inputs, Pending, Receipts, Defer),
  `fresh_id` über `u128`, Helfer `disconnect` und `drain_peer_burst`.
- `router/native_tests.rs`: Zugriff auf die Tabelle statt auf die zwei Maps (A1).

Testanzahl: 206 bestehend (alle grün) + 35 neu = **241**.

## 7. Checks

Eigene Prüfkopie = Basisarchiv + Kandidat, vorher geprüft (1296
Außendateien bytegleich, 37 Kandidatendateien = Manifest). Frisches
`CARGO_TARGET_DIR=target-final`, `CARGO_INCREMENTAL=0`, cargo/rustc 1.99.0,
jeder Schritt `--jobs 2 --locked`, Skript `evidence/run_checks.sh`
(`c0fd6702…`), Exit-Codes `logs/checks.txt` (`1f0010bf…`). Keine Source-Änderung
während der Checks.

| Check | Exit | Ergebnis | Log SHA-256 |
|---|---|---|---|
| `cargo test -p ferrumc-session` (Debug) | 0 | **241 bestanden**, 0 fehlgeschlagen: Unit 154, `native_audience` 6, `native_borrow` 3, `native_endpoint` 2, `native_join` 11, `native_lane` 13, `native_lifecycle` 19, `shard_directory` 15, `source_scan` 2, Doctests 16 | `7e16c0c0…` |
| dito `--release` | 0 | **241 bestanden**, 0 fehlgeschlagen | `bee39e28…` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | grün | `de3ddd97…` |
| `cargo fmt --all --check` | 0 | grün (leeres Log) | `e3b0c442…` |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-session --no-deps` | 0 | grün (nur Session, keine Workspace-Aussage) | `4f107fe3…` |
| `cargo build --workspace --all-targets` | 0 | grün | `547a7460…` |
| `cargo test -p ferrumc-app --no-fail-fast` | 0 | **375 bestanden**, 0 fehlgeschlagen, 1 ignoriert (Basis-`#[ignore]`), 40 Läufe; `multiplayer_visibility` im ersten Lauf 2/2 grün, kein Einzellauf nötig | `5250a47c…` |
| `cargo test -p ferrumc-testkit --no-fail-fast` | 0 | **100 bestanden**, 0 fehlgeschlagen, 8 Läufe | `cddd74a6…` |
| compile_fail eigenständig | 0 | 5/5 scheitern aus dem erwarteten Grund | `c624ba1d…` |

**Nach den Checks:** Kandidat in der Prüfkopie und in `candidate-src/` = Manifest;
Außendateien 1296/1296 bytegleich zur Basis; Basisarchiv 1325/1325 bytegleich.

**Testnebenprodukte, separat ausgewiesen:** Der App-Test `plugin_testhost`
(p57) legte in der Kopie
`.codex-tmp/p57-plugin-testhost-2343274-artifacts/{libplugin_testhost_fixture.so,librenamed_fixture.so}`
ab (je 8 221 192 Byte, beide SHA-256 `25d0b838c5adc94261e5d3ac42fd858bb33d91951ab77d984252a4de75c8de33`),
dazu das leere Verzeichnis `.codex-tmp/p52-abi-invalid-input/2342192`. Die
`.so`-Dateien liegen jetzt unter `.codex-tmp/net04_2e2-runtime/test-byproducts/`
(mit `SHA256SUMS`), die leeren Verzeichnisse sind entfernt. Danach stimmt der
Dateibestand der Kopie wieder mit dem Stand vor den Checks überein
(Dateiliste vor den Checks unter `evidence/`). Kein Source, außerhalb der Zählung. Die
redb-Tests legen temporäre Dateien `ferrumc-net042e1-*.redb` in
`std::env::temp_dir()` an und entfernen sie.

## 8. Offene Punkte und bekannte Einschränkungen

1. **A3 teilweise: Pose/Mode erfüllt, Held-Slot offen.** Grund: kein
   Sim-Producer. `ConnectedOwnedPlayerView` liefert nur Spieler, Generation,
   Position, Look, Mode, Seed-Hash und den Admission-Vergleich
   (`ferrumc-sim/src/collector_lifecycle.rs:83-125`); `SimShard` bietet nur
   `owned_player_slots` und `owned_player_carried` (`shard/player_ownership.rs:652`,
   `:665`), keinen Selected-Getter. Ein verweigerter Hotbar-Wechsel erhält daher
   nur den Positions-Resync (`router/native/audience.rs:105`), keinen
   Held-Slot-Resync. Root legt ein Sim-Ticket an (z. B.
   `ConnectedOwnedPlayerView::selected()`) plus Verdrahtung in NET-04.2f; das ist
   Voraussetzung für die Live-Aktivierung.
2. **Auflage für NET-04.2f (Root 2(b)):** `mirror_outbound`
   (`app/src/connection/play.rs:1145-1158`) stempelt jede Router-Sync über
   `PendingTeleport` – das muss bleiben –, ruft für jedes `GameEvent` aber auch
   `mirror_game_mode_change(event, inventory)` auf (`play.rs:1158`, `:1205`). Das
   ist Connection-Inventar-Autorität, die nach Vertrags-Review A4 im Nativpfad
   ausgeschlossen ist; e2 erzeugt genau solche `GameEvent(3, mode)` aus dem
   Pose-Producer. f muss die Spiegelung für native Sessions abschalten, ohne die
   Teleport-Stempelung zu verlieren, und alle nativen Outbound-Nachrichten über
   diesen Pfad leiten (kein Direktversand mit ID 0).
3. **R3-A5-Wechselwirkung (Befund).** Ein dauerhaft unsicherer Item-Pickup hält
   seine gebundene Counterprojektion; nach `defer` (Default im Testenv 64, Maximum
   1024) Normal-Boundaries scheitert `run_tick_with_player_ownership_at` mit
   `CounterRecoveryRequired` (im Test bei 1024 Claims). Danach gibt es keine
   applied Boundaries mehr; die Leave-Save-Zähler stehen still, und auch der
   Final-Save der Stop-Phase A braucht Normal-Boundaries. Die Quarantäne nach
   900 Boundaries ist nur mit `defer` > 900 erreichbar; sonst bestimmt der
   Recovery-/Degraded-Pfad des Drivers (DUR-05a, R3-A5-Sim-Ticket) den Verlauf.
   Der Test macht beides sichtbar.
4. **Transientes Busy.** Auf `InMemoryStore` schreibt der Store synchron; ein
   echtes `Busy` ist ohne Testbinder nicht deterministisch erzeugbar. Die
   Session-Seite (`Prepared` blockiert, Zählregeln) ist mit echter
   `HistoryCapacity` belegt; die Kette Busy → Driver-Retry → Bestätigung ist N3
   im Driver (DUR-05a/f).
5. **Konsumentenreferenzen** (.2c) entstehen erst mit NET-04.2b; e2 zählt sie als
   0. NET-04.2b muss D7 um diese Bedingung ergänzen.
6. **Audience-Abweichung H3** bleibt (Tab nur an Viewer in Sicht); Range ist der
   bestehende Chunk-Quadrat-Test mit der Router-Viewdistance, kein Paper-Tracker.
7. **Testname:** `unregister_refuses_while_records_adopted_or_poisoned_entries_exist`
   prüft nur einen adoptierten Record. Dass ein Poisoned-Eintrag die
   Unregistrierung blockiert, ist Code (`check_native_unregister`), aber nicht
   eigens getestet. Da der Name nach dem Kandidaten-Freeze aufgefallen ist, habe
   ich ihn nicht mehr geändert.
8. Werte (8/8/20/100/1200, 64 KiB Equipment, 16 Zeichen) sind gewählte
   Vertragsgrenzen, nicht gemessen. Workspace-Rustdoc bleibt wie in e1 (H9)
   außerhalb dieser Aussage.
9. NET-04.2f: Driver, Gate, Permits, L2-Token-Tabelle, Teleport-Stempelung,
   Stop A/B/C mit Sessions, Revalidierungsaufruf nach dem Boundary der
   Alt-Items. DUR-05a: Recovery/Degraded.

## 9. Freeze

| Artefakt | Wert |
|---|---|
| Kandidat `candidate-src/` (readonly, vor den Checks) | 37 Dateien, Aggregat `4a5b36539b368969036253973cea6c97bdcafe4732529d16f538626c173d47f1` |
| `evidence/candidate.manifest.json` | `631cc69e84fe5e5e1c2df4b895916fc5910000f3499950bccd3dab2792080cee` (vor den Checks abgelegt und dem Root gemeldet) |
| Diff gegen Basis `evidence/session.diff` | `10c76e1f9769e87df1b2b734642d1bfacab48f13eddf342ea6728bfde6c8f4b5` |
| e1-Testdiff `evidence/e1-tests.diff` | `f082b41c0ae594730ae936e3da57c180abf4217286f6a2578734706768bfe98b` |
| Evidence-Freeze `.codex-tmp/net04_2e2-runtime/evidence.freeze.json` | `a4ce2dcbe1948c03f2abbd7e9b97b86976588e7983bf0b90f1a43b71bec905fb`, 27 Dateien (Evidenz, Logs, Testnebenprodukte), Aggregat `33cc5eb327cda0f378215bb16b1c0a7926f2b860c22633269d83dcddaa42c9ac` |
| Übernahme in den Arbeitsbaum | vorher geprüft: `server/crates/ferrumc-session/**` und `server/Cargo.lock` bytegleich zur Basis; danach 37/37 = Manifest, keine Zusatzdatei; `Cargo.lock` unverändert |

Git, Board und Integration übernimmt Root. Im Arbeitsbaum ändern andere Agents
`ferrumc-storage` (und bald `ferrumc-world`); die Checks oben decken nur
`main e18efd22` plus diesen Kandidaten ab.

## 10. Nicht-Aussagen

Keine Aktivierungs-, Survival-, Performance-, Paper- oder Live-Client-Aussage.
Alle Zahlen in §3/§5 sind gewählte Vertragsgrenzen, nicht gemessen.

## 11. Runde 2 — Nachbesserung F1, F2, V1

Status Runde 2: **Nachbesserung abgeschlossen, Checks grün, Probes P1–P8
gelaufen; wartet auf den gezielten Nachreview.** 12.10.2026,
Implementierungs-Subagent, exklusiver Owner von `server/crates/ferrumc-session/**`.
Die Abschnitte 1–10 beschreiben unverändert den Stand von Runde 1; wo Runde 2
eine Aussage dort ändert, steht es hier.

### 11.1 Auftrag und Basis

| Gegenstand | Wert |
|---|---|
| Review | `NET-04_2e2_NATIVE_SESSION_LIFECYCLE_REVIEW.md` `c8ba2276…`, `final-assessment.json` `95d22835…` (Verdikt FINDINGS: F1, F2, V1, H1–H6) |
| Root-Entscheidungen Runde 2 | F1 und F2 beheben, V1 als Vertragsnachtrag „§7.3-Nachtrag V1 (Root)“ in der Session umsetzen, H1 nur dokumentieren, keine weiteren Umbauten |
| Basis | `main` `e18efd22c9e062190b023575ac53fb0da04c2eb8` (`git archive`), Runde-1-Kandidat `candidate.manifest.json` `631cc69e…` (= Arbeitsbaum vor Runde 2, geprüft) |
| Vertrag | `NET-04_2d_NATIVE_DRIVER_ENDPOINT.md` `e0530e0a…`, §7.3 Z. 840–860 |
| Arbeitskopie | `.codex-tmp/net04_2e2-runtime/round2-work` (Archiv + Runde-1-Overlay), Dev-Target `round2-target-dev` |
| Prüfkopie | `.codex-tmp/net04_2e2-runtime/round2-isolated` (frisches Archiv + eingefrorener Runde-2-Overlay), frisches `round2-target-final` |

Geändert wurden nur Dateien unter `server/crates/ferrumc-session/**` (8 Dateien
gegenüber Runde 1, keine neu, keine entfernt), dazu neue Dateien mit Präfix
`round2-` unter `.codex-tmp/net04_2e2-runtime/` und dieser Abschnitt.
`candidate-src/` und die Runde-1-Evidenz sind unverändert. In der Prüfkopie
sind `Cargo.lock`, `Cargo.toml` und alle anderen Crates bytegleich zur Basis.
Im Arbeitsbaum ist `server/Cargo.lock` unverändert. Kein schreibendes Git: Es
liefen nur `git archive` für die Basis und lesend `rev-parse`/`status`. Keine
Agents, keine Dependency.

Forbidden-Claims-Scan (Funktionen aus `scripts/check-forbidden-claims.py`, über
den Wrapper des Reviews, Arbeitsbaum nach der Übernahme): Delivery, INVARIANTS,
README 0 Treffer. In der Session-Rustdoc gibt es dieselben 3 Treffer wie im
Review (`router.rs:83/195/512`). Das ist unveränderter Basistext.

### 11.2 F1 — Final-Save eines `DisconnectNotApplied`-Records

**Ursache (Review):** `renew_final_save` setzte jeden quarantinierten Record auf
`Due`. Meldete der nächste Status für einen noch verbundenen Spieler
`Ineligible("connected player")`, lieferte `await_detach` außerhalb von
`AwaitDetach` `Decision::StatusInvariant` und damit
`CollectorSessionError::NativeStatusInvariant` (Driver-Fail-Stop in Phase A).

**Umsetzung (`native/retention.rs`):**
- `Retention::final_save: Option<FinalSave>` ersetzt `final_save_used: bool`.
  `FinalSave` hält den erneuerten Quarantänegrund und die Versuche davor.
- `renew_final_save` bleibt „genau ein erneuerter Versuch je quarantiniertem
  Record, einmal“ (`Ok(true)`, danach `Ok(false)`).
- Sieht der nächste Schritt eines aus `DisconnectNotApplied` erneuerten Records
  (`Due`) einen noch verbundenen Spieler, kehrt der Record typisiert nach
  `Quarantined{DisconnectNotApplied}` zurück: Versuche wie vor der Erneuerung,
  kein Submit, kein Resend, kein Fehler. Der Driver bricht nicht ab, die
  übrigen Records laufen weiter.
- Ist der Disconnect inzwischen angewandt (Status detached), ist der erneuerte
  Versuch ein echter Leave-Save (normaler `Due`-Pfad).
- `NativeStatusInvariant` bleibt den echten Verdrahtungsbrüchen vorbehalten
  (Foreign/Stale über die Status-Closure → typisierte Quarantäne wie bisher;
  `SnapshotOnly`, `NotInitialized`, Scope → Fehler). Für jeden anderen Record
  gilt weiter: Er verlässt `AwaitDetach` nur über einen detached Status, und
  eine detached Admission wird nie wieder verbunden.

### 11.3 F2 — Refused-Session über weitere Replacements

**Ursache (Review):** `replace_native_shard_registration` stellte nur `Bound`
auf `Suspended`. Eine `Refused`-Session behielt die Lane-Identität ihrer
Verweigerung; nach einem weiteren Replacement scheiterte jeder Flush mit
`Superseded`, `retry_native_disconnects()` blieb 1.

**Umsetzung (`router/native.rs`):** Beim Replacement übernimmt eine
`Refused`-Session derselben Registrierung die neue Lane-Identität. Sie bleibt
`Refused`, wird nie umgebunden (keine Revalidierung, kein Data, nur der
Disconnect aus dem gespeicherten Handle). Ihr zurückgehaltener Disconnect geht
beim nächsten `retry_native_disconnects()` über die jeweils neueste Lane. Die
Sim wendet ihn an (normaler Detach-Pfad D3–D10) oder verweigert ihn beim Apply
(`StaleHandle` → `Quarantined{StatusInvariant}`, kein Resend, keine Umbindung).

**Aussage „retry erreicht 0“ (INVARIANTS, §3 A2):** Sie gilt nach der
Behebung wieder ohne die F2-Ausnahme. Jede gebundene oder verweigerte Session
sendet über die aktuelle Lane ihrer Registrierung, jede suspendierte nach ihrer
Revalidierung. Vorausgesetzt sind nur die Driverpflichten des Vertrags: Lane
leeren, nach einem Replacement revalidieren. Eine Ausnahme ohne Driverfehler
kenne ich nicht. Die Aussage blieb deshalb stehen; INVARIANTS nennt jetzt
zusätzlich „auch nach jedem weiteren Replacement (F2)“.

### 11.4 V1 — §7.3-Nachtrag V1 (Root): Resends zählen erst nach Entnahme

**Regel (Root):** Die höchstens 8 Disconnect-Wiederholungen (danach
`Quarantined{DisconnectNotApplied}`) zählen nicht mehr pro angewandtem
Normal-Boundary seit dem Senden (`applied > sent_at`). Eine Wiederholung zählt
nur, wenn der zuvor gesendete Disconnect tatsächlich aus der Lane entnommen
(und damit nach §6.2 der Sim übergeben) wurde, danach mindestens ein
Normal-Boundary angewandt ist und der Spieler weiter verbunden ist. Ein
Rückstau in der Lane verbraucht keinen Versuch und quarantiniert nie einen
unbeteiligten Spieler. Im Code ist jede Stelle als „§7.3 amendment V1 (Root)“
markiert; den Eintrag in die Vertragsentscheidungen macht Root.

**Umsetzung:**
- `native/lane.rs`: `NativeLaneIdentity` zählt je Lane-Generation die Sends
  (der Router ist einziger Sender) und die Entnahmen (`NativeShardReceiver::try_recv`,
  jedes entnommene Element, auch eine vergiftete Bewegung). Beides sind
  monotone `AtomicU64`, kein Lock, keine Allokation. Da der Kanal FIFO ist, hat
  das n-te gesendete Element die Lane genau dann verlassen, wenn der
  Entnahmezähler n erreicht. `LaneMark{lane, position}` merkt sich die Position
  eines Disconnects auf seiner Lane-Generation, auch über Replacements hinweg.
- `router/native.rs`: `send_lane` liefert die Position;
  `flush_disconnect` legt die Marke ab (`NativeSession::disconnect_mark`).
- `router/native/retention.rs`: Der Service leitet für eine gemappte Session
  `LaneDisconnect::{Retained, Queued, Taken}` ab (Taken = Marke erreicht oder
  kein Disconnect mehr nötig).
- `native/retention.rs` (`await_detach`): `Retained` und `Queued` warten ohne
  Zählung. Bei `Taken` merkt sich der Record den ersten `applied`-Wert der
  Beobachtung (`sent_at`). Erst bei `applied > sent_at` und weiter verbundenem
  Spieler folgt die nächste Wiederholung (`resends + 1`, `sent_at` wieder offen)
  bzw. nach der achten die Quarantäne. Die Wartezeit beginnt also erst mit der
  Beobachtung der Entnahme, nicht mit dem Senden. Das ist konservativ: Es kostet
  höchstens einen Boundary je Wiederholung. Der G3-Pfad (`Submit`) ist
  unverändert, ein Submit liegt direkt in der Sim-Queue.
- `lane_resend_retained` entfällt: Eine nicht eingereihte Wiederholung bleibt
  `Retained`, `retry_native_disconnects` sendet sie, und erst ihre Entnahme
  startet die nächste Wartezeit.
- Gehaltener Lane-Input (R2-A5): Verweigert die Sim das Enqueue eines
  entnommenen Disconnects, hält der Driver ihn und versucht ihn vor jedem
  weiteren `try_recv` zuerst. Dabei kann höchstens eine Wiederholung zählen; die
  neue liegt dann hinter dem gehaltenen Input in der Lane und zählt erst nach
  ihrer eigenen Entnahme.

**Folge für `DisconnectNotApplied`:** Die Sim wendet jeden übergebenen
Disconnect einer verbundenen Admission an (`apply_native_disconnect`). Die
Quarantäne setzt daher voraus, dass neun entnommene Disconnects nicht
angewandt wurden, also eine verlorene Übergabe im Driver. Ein reiner Rückstau
reicht nicht mehr.

### 11.5 Tests

Neu in `tests/native_lifecycle.rs` (alle Memory **und** redb, echte Lane,
echte Admission, echte Work/Completion):

| Test | Muster | Belegt |
|---|---|---|
| `final_save_of_a_disconnect_not_applied_record_never_breaks_the_driver` | P4b | (a) Disconnect bleibt unangewandt: `renew` → `Ok(true)`, dann `Ok(false)`; der nächste Service liefert `Quarantined{DisconnectNotApplied}` mit 0 Versuchen, **kein** `NativeStatusInvariant` (der Test-Driver entpackt das Ergebnis); kein Resend, Spieler bleibt verbunden; der zweite Record (Other) läuft im selben Service-Durchgang bis Leave-Save, D5, D7/D8 und Release. (b) Der zuletzt entnommene Disconnect wird doch noch übergeben: echter Detach, der erneuerte Versuch ist ein echter Leave-Save, beide Records werden released |
| `refused_retained_disconnect_survives_every_replacement_and_ends` | P5 | Einslot-Lane: Ay verweigert, Disconnect `Retained` (retry 1), **zwei** weitere Replacements; danach retry 0, Ay `RevalidationRefused` + `Sent`. (a) Echter Owner: Detach, Leave-Save, D5 (Ay und Bee), D7/D8, Release, retry 0. (b) Fremder Owner derselben UUID auf der neuesten Lane: Apply-Refusal, `Quarantined{StatusInvariant}`, nie erneut gesendet, retry 0, Original-Admission unberührt |
| `lane_backlog_of_another_client_never_quarantines_a_queued_disconnect` | P7 | 48 Hotbar-Items eines anderen Clients, Fenster 4 je Iteration: In den Iterationen 1–12 `Waiting`, `AwaitDetach`, keine Wiederholung (genau ein Control-Item unter 49 entnommenen). Der Disconnect verlässt die Lane in Iteration 13, echter Detach, keine Quarantäne, danach Leave-Save mit 1 Versuch, D5, D7/D8, Release |

Geändert (V1 erzwingt es): Der erste Teil von `status_closure_refusals_are_typed`
kodierte die alte Zählung. Die Lane wurde nie geleert, nach 9 Boundaries folgte
die Quarantäne. Jetzt:
- 100 Boundaries mit nie entnommenem Disconnect → stets `Waiting`/`AwaitDetach`;
- **Gegentest:** Jeder Disconnect wird entnommen, die Übergabe geht verloren →
  `DisconnectNotApplied` nach genau 9 entnommenen Disconnects (Original + 8
  Wiederholungen) in Iteration 18; retry 0, Spieler bleibt verbunden.

Die Teile StaleHandle/Foreign und SnapshotOnly/NotInitialized sind unverändert.
Keine andere Assertion wurde geändert. Dazu kommen
`NativeLaneIdentity::new(…)` in `router/native_tests.rs` (Konstruktor statt
Struct-Literal) und die Hilfen `lose_lane_items` und
`quarantine_by_lost_disconnects`.

**Mutationskontrolle (Dev-Kopie, danach zurückgesetzt, md5 geprüft):**
- F1-Zweig unwirksam (Muster auf einen anderen Grund gesetzt) → nur der
  F1-Test scheitert.
- F2-Zuweisung entfernt → nur der F2-Test scheitert.
- „Queued“ wie „Taken“ behandelt → der Backlog-Test und
  `status_closure_refusals_are_typed` scheitern.

Testanzahl Session: 241 + 3 = **244**.

### 11.6 Review-Probes P1–P8, unverändert gegen Runde 2

`review_probes.rs` (`4f977bbf…`, unverändert kopiert, Hash im Log) in eigener
Kopie `round2-probe/` (= `round2-isolated` + genau diese Datei), eigenes
`round2-target-probe`. Log `round2-logs/probes-debug.log`.

| Probe | Runde 1 (Review) | Runde 2 | Grund |
|---|---|---|---|
| P1 Rejoin während Leave-Save | bestanden | bestanden, gleiche Werte (`Pending`, `DuplicatePlayer` vor D5, Sim „reserved“, 1 Versuch, ein Release), M+R | unverändert |
| P2 Release gegen Rejoin | bestanden | bestanden, gleiche Werte in (a) und (b), M+R | unverändert |
| P3 G3 bei voller Lane, Limit 2/2 | bestanden | bestanden (`SessionCapacity{2}`, Adoption, Kette bis Release), M+R | unverändert |
| P4 eigenes `Prepared` | bestanden | bestanden: Quarantäne nach 800 Boundaries mit 9 Versuchen, erneut nach 1400 (H1), Release verweigert, M+R | unverändert |
| P4b Final-Save `DisconnectNotApplied` | bestanden (kodierte F1) | **scheitert erwartungsgemäß** in Z. 431 `assert_eq!(reason, Some(DisconnectNotApplied))`: `left: None`. Die Probe leert die Lane nie. Nach V1 zählt ein nie entnommener Disconnect keine Wiederholung, also entsteht in 20 Boundaries keine Quarantäne. Die F1-Assertion dahinter (Z. 446/447: `renew` → `Ok(true)`, danach `NativeStatusInvariant`) wird nicht mehr erreicht; das korrigierte F1-Verhalten belegt der neue Test mit verlorener Übergabe (§11.5) | Umkehr durch V1 |
| P5 Refused + zweites Replacement | bestanden (kodierte F2) | **scheitert erwartungsgemäß** in Z. 559 `assert_eq!(retry_native_disconnects(), 1)`: `left: 0`. Beobachtet: `retry_series=[0, 0, 0, 0, 0]` direkt nach dem zweiten Replacement; 39 von 40 Services `Waiting`, der übrige ist der Leave-Save-Submit; danach `leave_save=Pending`, 1 Versuch, Ay nicht mehr verbunden, Control nicht mehr live. Damit kehren sich auch Z. 560–562 um (40/40 `Waiting`, verbunden, `AwaitDetach`) | Umkehr durch F2 |
| P6 Poisoned blockiert Unregister | bestanden | bestanden (`NativeEndpointInUse{sessions: 1}`), M+R | unverändert |
| P7 Lane-Rückstau | bestanden (kodierte V1) | **scheitert erwartungsgemäß** in Z. 681 `quarantine_at.unwrap()` auf `None`. Beobachtet: keine Quarantäne, echter Detach in Iteration 13, Endzustand `Pending`/`Detached` (der Leave-Save läuft; die Probe führt dessen Work nicht aus). Damit kehren sich auch Z. 682–693 um (Grund `DisconnectNotApplied`, Detach erst nach der Quarantäne, quarantinierter Endzustand) | Umkehr durch V1 |

Die drei Probes kodieren das alte, fehlerhafte Verhalten als Assertion. Ihre
Schleife läuft Memory vor redb; die erste umgekehrte Assertion beendet den Test
daher im Memory-Lauf, und ihr redb-Lauf kommt nicht mehr an die Reihe. Das
korrigierte Verhalten auf redb belegen die drei neuen Regressionstests (§11.5).
P1, P2, P3, P4 und P6 liefern dieselben Werte wie im Review, P4 also weiterhin
800/1400 Boundaries (H1, §11.8).

### 11.7 Checks (Runde 2)

Skript `round2-run_checks.sh` (`2b04983f…`), frisches
`CARGO_TARGET_DIR=round2-target-final`, `CARGO_INCREMENTAL=0`, cargo/rustc
1.99.0, jeder Schritt `--jobs 2 --locked`, Exit-Codes `round2-logs/checks.txt`
(`dc615930…`). Keine Source-Änderung während der Checks; die Probes liefen
danach in `round2-probe/` mit eigenem `round2-target-probe`. Last beim Start
etwa 4,7–6,8.

| Check | Exit | Dauer | Ergebnis | Log SHA-256 |
|---|---|---|---|---|
| `cargo test -p ferrumc-session` (Debug) | 0 | 84 s | **244 bestanden**, 0 fehlgeschlagen: Unit 154, `native_audience` 6, `native_borrow` 3, `native_endpoint` 2, `native_join` 11, `native_lane` 13, `native_lifecycle` 22, `shard_directory` 15, `source_scan` 2, Doctests 16 | `e38fd664…` |
| dito `--release` | 0 | 245 s | **244 bestanden**, 0 fehlgeschlagen | `e2739343…` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 59 s | grün | `66a86c4b…` |
| `cargo fmt --all --check` | 0 | 3 s | grün (leeres Log) | `e3b0c442…` |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-session --no-deps` | 0 | 12 s | grün (nur Session, keine Workspace-Aussage) | `da5481b6…` |
| `cargo build --workspace --all-targets` | 0 | 171 s | grün | `d30d2915…` |
| `cargo test -p ferrumc-app --no-fail-fast` | 0 | 167 s | **375 bestanden**, 0 fehlgeschlagen, 1 ignoriert (Basis-`#[ignore]`), 40 Läufe; `multiplayer_visibility` im ersten Lauf 2/2, kein Einzellauf nötig | `9068a408…` |
| `cargo test -p ferrumc-testkit --no-fail-fast` | 0 | 33 s | **100 bestanden**, 0 fehlgeschlagen, 8 Läufe (die Zeile „requested fixture panic“ ist der beabsichtigte Panic eines bestandenen Tests, wie in Runde 1) | `ff3e1b30…` |
| compile_fail eigenständig (`evidence/compile_fail_check.py` gegen die rlibs aus `round2-target-final`) | 0 | < 1 s | 5/5 scheitern aus dem erwarteten Grund, wie in Runde 1: Z. 35 E0597, Z. 54 E0382, Z. 64 E0521, Z. 145 private Felder, Z. 154 E0521 (`round2-evidence/round2-compile_fail.json` `cf087ae9…`) | `c624ba1d…` |
| Review-Probes P1–P8 (unverändert, Debug, `--test-threads 1`) | 101 | 39 s | 5 bestanden, 3 erwartungsgemäß umgekehrt (§11.6) | `f9825d16…` |

**Vor und nach den Checks:** `round2-isolated` vorher frisch aus `git archive`
(1325/1325 = `base.freeze.json`), dann Overlay; Außendateien 1296/1296
bytegleich zur Basis, 37 Kandidatendateien = Runde-2-Manifest, 1333 Dateien.
Nach den Checks identisch, nachdem die Testnebenprodukte verschoben waren.

**Testnebenprodukte, separat ausgewiesen:** `plugin_testhost` (p57) legte in der
Prüfkopie
`.codex-tmp/p57-plugin-testhost-2454793-artifacts/{libplugin_testhost_fixture.so,librenamed_fixture.so}`
ab (je 8 221 424 Byte, beide SHA-256
`810277e274e33beb400ad1c43d4a08ebc39e6e4ea3ddb585f104311918f8fc11`), dazu das
leere Verzeichnis `.codex-tmp/p52-abi-invalid-input/2453999`. Die `.so`-Dateien
liegen jetzt unter `.codex-tmp/net04_2e2-runtime/round2-test-byproducts/` (mit
`SHA256SUMS` und `SIZES`), die leeren Verzeichnisse sind entfernt.

### 11.8 Hinweise und Einschränkungen

1. **H1 (nur dokumentiert, nicht geändert):** Nach `renew_native_leave_save`
   wertet der Status das eigene alte, noch bestehende Pending als fremdes
   Blocking. Die Wartezeit von 1200 plus zwei Zählintervalle ergibt 1400
   Boundaries bis zur erneuten Quarantäne. Das liegt über der Phase A von
   1200 Iterationen (§10.4), sodass Phase A dann mit `phase_a_incomplete` endet.
   Ergebnis bleibt vertragsgemäß `Unresolved`, ohne Verlust und ohne Release
   (P4, Runde 2 unverändert).
2. **V1, Driverpflicht:** Die Zählung setzt voraus, dass der Driver jeden alten
   Receiver vor dem neuen vollständig leert (§4.3 „Replacement“: alten Receiver
   in `draining`, vollständig und vor dem neuen leeren). Ein Driver, der
   einen Receiver ungeleert verwirft, verliert die enthaltenen Disconnects ohne
   Entnahme. Der Record bleibt dann ohne Wiederholung in `AwaitDetach` bis zum
   Stop. Runde 1 hätte dort nach einem Boundary eine Wiederholung angestoßen,
   die nach der Revalidierung über die neue Lane gegangen wäre.
   Das ist ein Driverfehler, kein Session-Pfad. Ich nenne ihn als Auflage für
   NET-04.2f: Replacement erst nach vollständigem Drain des alten Receivers.
3. **V1, Erreichbarkeit der Quarantäne:** `DisconnectNotApplied` entsteht nur
   noch nach verlorener Übergabe (§11.4). Der Gegentest erzeugt sie durch einen
   Test-Driver, der entnommene Items verwirft. Das ist der einzige Weg, weil die
   Sim jeden übergebenen Disconnect einer verbundenen Admission anwendet.
4. H2–H6 und die übrigen Punkte aus §8 sind unverändert offen bzw. ausgewiesen,
   auch die Testnamen-Ungenauigkeit (§8 Punkt 7, Verhalten durch P6 belegt).
   Root hat keine weiteren Umbauten beauftragt.

### 11.9 Freeze Runde 2

| Artefakt | Wert |
|---|---|
| Kandidat `round2-candidate-src/` (readonly, vor den Checks) | 37 Dateien, Aggregat `ad02a694180bbcbdd3899b727a45b6ae3f79c6bd975744ba4ec224130b2ffee2` |
| `round2-evidence/round2-candidate.manifest.json` | `f180e54f897e2699711b5eef2b550091d72d87793619c07c5d67d2a53047abd9` (vor den Checks abgelegt). Gegen die Basis: 15 geändert, 8 neu, 14 unverändert, 0 entfernt (`native/lane.rs` ist neu geändert). Gegen Runde 1: 8 geändert (`INVARIANTS.md`, `native/lane.rs`, `native/mod.rs`, `native/retention.rs`, `router/native.rs`, `router/native/retention.rs`, `router/native_tests.rs`, `tests/native_lifecycle.rs`), 0 neu, 0 entfernt |
| Diff Runde 2 gegen Runde 1 `round2-evidence/round2-vs-round1.diff` | `85c292e898a481572674e9cbe48f29d8c97cede7daca02f3d3f0fb26f35032ad` |
| Diff gegen Basis `round2-evidence/round2-session.diff` | `351f053e2fae09cfb2d1dfeb544dd98226be15a5eed4fd8393fb3bc1b7191469` |
| Evidence-Freeze `round2-evidence.freeze.json` | `d10270493ea0bec03b29c92a954ff0b388f4c9c9d4b4438e2cec3e5f41156a1b`, 29 Dateien (Logs, Evidenz, Skripte, Testnebenprodukte), Aggregat `6feea003a2812c6de52986d4e9112b66e4b47d1a01dd72305453094b930dc6a9` |
| Runde 1 unverändert | `candidate-src/` = Manifest `631cc69e…` (37/37); `evidence.freeze.json` `a4ce2dcb…` 27/27 nachgeprüft |
| Übernahme in den Arbeitsbaum | vorher geprüft: `server/crates/ferrumc-session/**` = Runde-1-Manifest (37/37); danach 37/37 = Runde-2-Manifest (`round2-evidence/round2-worktree-verify.json`), keine Zusatzdatei; `server/Cargo.lock` unverändert (`183450e7…`) |

### 11.10 Nicht-Aussagen (Runde 2)

Keine Aktivierungs-, Survival-, Performance-, Paper- oder Live-Client-Aussage.
Alle Grenzen (8 Wiederholungen, 8 Versuche, 20/100/1200 Boundaries) sind
gewählte Vertragsgrenzen, nicht gemessen. Git und Integration übernimmt Root.
