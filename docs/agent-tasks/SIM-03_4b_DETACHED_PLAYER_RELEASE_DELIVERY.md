# SIM-03.4b — Freigabe abgemeldeter nativer Spielerzustände: Autorenlieferung

Reviewbarer **Sim-only-Runtimekandidat**, keine unabhängige Rootabnahme. Autor:
Implementierungs-Subagent, 11.10.2026. Kein Git, keine weiteren Agents, keine
Dependency-, Lockfile- oder Generated-Änderung, kein anderes Crate. Diese
Lieferung behauptet **keine** Aktivierung, keine Survival-, Live-, Paper- oder
Performanceabnahme.

## Basis und Grundlagen

| Gegenstand | Wert |
|---|---|
| Basis | git archive `d7f16b21a6baf311801e6bef4918498178fde28d`, 1298 Dateien |
| `base.freeze.json` | `98306c845fdcaba5f01e6d062bc4cf02dcd9aa83d6b471024724217e34e816b8`, frisch geprüft: Pfadmenge gleich, 0 Byteabweichungen, alle Dateien readonly |
| Vertrag | `NET-04_2d_NATIVE_DRIVER_ENDPOINT.md` `e0530e0a522b8c2e57a06268206a7721b9ca512b2fd2af3f7f71126fcdbba14f` (frisch gehasht) |
| Gelesen | Freigabe `SIM-03_4b_DETACHED_PLAYER_RELEASE_TASK.md`; Vertrag §2, §3, §6.2–6.5, §7 (inkl. D8, §7.3, §7.5), §10.4, §11, §13, §14, §17/§18; Root-Entscheidungen; Reviews Runde 1–3 (SIM-03.4b-Stellen); SIM-03.4a-Delivery/-Review; DUR-04.2-Brief/-Review; AGENTS, CLAUDE, Sim-INVARIANTS |
| Arbeitskopie | `.codex-tmp/sim03_4b-runtime/work` (Basis + Sim-Overlay), eigene Target-Verzeichnisse `target-dev` und frisch `target-final` |
| Gemeinsamer Baum | `server/crates/ferrumc-sim/**` vor der Übernahme bytegleich zur Basis (56 Dateien, 0 Abweichungen, keine Zusatzdatei); danach exakt der Kandidat (60 Dateien, 0 Abweichungen, keine Zusatzdatei) |

## Umgesetztes Verhalten

**Admissionsdaten.** `OwnedPlayerInventory` trägt zwei private Felder:
`saved_after_detach: Option<OwnershipTransferId>` und `lifecycle_acked: bool`.

- `saved_after_detach` setzt ausschließlich `publish_confirmed`, wenn es die
  echte `Confirmed{current: Ok}`-Completion eines **Autosave**-Pendings anwendet
  und der Spieler danach `!connected` ist (`record_detached_save`). Die zuletzt
  bestätigte gilt. Zurückgesetzt wird es bei jedem neu zugelassenen Pending des
  Spielers (`install_candidate`), bei jedem Deferred-Apply/Dirty
  (`apply_deferred`) und bei jeder Veröffentlichung über `publish_player`
  (Rejoin/Restore/Bootstrap und jede andere bestätigte Spielerveröffentlichung
  beginnen mit `None`). Ein späteres unsicheres Ergebnis lässt das Pending
  bestehen; `blocking()` zeigt es, der Release verweigert typisiert.
- `lifecycle_acked` (D8) setzt `acknowledge_collector_lifecycle` nach
  erfolgreicher Ack, nur wenn die aktuelle PlayerState-Admission der Admission
  des Tickets entspricht; eine Ack der Zelle einer älteren Admission markiert
  nie einen Rejoin. Nicht-admittierende Veröffentlichungen übernehmen das Bit,
  nur eine neue Admission setzt es zurück. Ohne Lifecycle-Modus gibt es keine
  Zelle; der Release verweigert dann `LifecycleNotInitialized` statt „acked“.

**APIs** (Signaturen wie §7.5):

- `SimShard::detached_owned_player_status(&self, original)` →
  `DetachedOwnedPlayerStatus` (Copy, private Felder, Getter
  `saved_after_detach()`, `blocking()`, `dirty()`; `dirty` schließt noch nicht
  angewandte Deferred-Werte ein). Refusals: `NotInitialized`, `SnapshotOnly`,
  `ScopeMismatch` (über `ownership_state`), `StaleHandle` (fremder Owner,
  alte Admission, freigegeben), `Ineligible("connected player")`.
- `SimShard::release_detached_owned_player(&mut self, original)` →
  `ReleasedOwnedPlayer` (Getter `player()`, `freed_player_bytes()`, kein
  öffentlicher Konstruktor, kein Handle, keine Query). Keine Caller-Query.
- `SimShard::owned_stop_ready(&self)` → `Result<bool, _>`; gestoppter Owner →
  `Draining`, Snapshot → `SnapshotOnly`.
- `pub use ferrumc_storage::{OwnershipReceiptQuery, OwnershipTransferId}` in
  `ferrumc-sim/src/lib.rs` mit Rustdoc (R2-A4).
- R3-H2 (optional, umgesetzt): `OwnedPlayerInputRefusal::query()` liefert rein
  diagnostisch die Query einer abgelehnten Completion/Retry/Reconcile, sonst
  `None`. Keine Korrektheit hängt daran.

**Gemeinsames Stop-Prädikat (R3-1).** Der bisherige Rumpf von
`ownership_quiescent` ist unverändert in den privaten Helfer
`owned_state_quiescent(&self, &PlayerOwnershipState)` verschoben;
`ownership_quiescent` (Stop-Apply, `is_tick_quiescent`) und `owned_stop_ready`
rufen ihn beide auf. Keine Kopie, nur geborgte Iteratoren, keine Allokation.

**Release-Gate (A7, G5, R3-2).** Alle Prüfungen laufen auf geborgtem Zustand
vor der einzigen Mutation, in dieser Reihenfolge:

| # | Prüfung | Refusal |
|---|---|---|
| 1 | Owner initialisiert, kein Snapshot, Scope | `NotInitialized` / `SnapshotOnly` / `ScopeMismatch` |
| 2 | nicht gestoppt | `Draining` |
| 3 | `actual_player(original)` | `StaleHandle` (fremd, alt, wiederholter Release) |
| 4 | `!connected` | `DetachedRelease(Connected)` |
| 5 | Lifecycle-Modus initialisiert | `DetachedRelease(LifecycleNotInitialized)` |
| 6 | kein Queue-Input nennt die PlayerId (Handle, Restore/Rejoin/Bootstrap-Read, Completion-Keys, Retry/Reconcile auf ein Pending des Spielers) | `DetachedRelease(QueuedInput)` |
| 7 | kein reservierter (Pending) oder retained (FIFO) Take mit diesem Spieler als Collector | `DetachedRelease(PublicationPin)` |
| 8 | kein Item-/Pair-Pending des Spielers (hält die Entity-Owner-Reservierung inkl. betroffener Chunks) | `DetachedRelease(ItemReservation)` |
| 9 | kein sonstiges Pending (inkl. Bootstrap) des Spielers | `DetachedRelease(Pending)` |
| 10 | `lifecycle_acked` dieser Admission | `DetachedRelease(LifecycleNotAcknowledged)` |
| 11 | keine weitere Lifecyclezelle derselben PlayerId (ältere Admission) | `DetachedRelease(CollectorBinding)` |
| 12 | nicht dirty, keine Deferred-Werte | `DetachedRelease(Unsaved)` |
| 13 | `saved_after_detach` Some | `DetachedRelease(NotSaved)` |

Wirkung: `players.remove` und `player_bytes -= confirmed.len()` (checked, vor
der Mutation berechnet) im selben Schritt. Die Zelle ist seit der Ack frei;
Pendingbytes, Inputbytes, Publication- und Lifecyclecredits betreffen den
Spieler nachweislich nicht mehr (Gates 6–11). Arbeit je Aufruf: ein
Map-Eintrag, ≤32 Inputs, 2×≤32 Pendings, ≤256 retained Gruppen, ≤384 Zellen;
keine Allokation; keine automatische Verdrängung. Release ist `&mut self`
zwischen Boundaries, der nächste Plan sieht nur die verringerte Spielerzahl.

Neu: `DetachedReleaseRefusal` (`#[non_exhaustive]`, Copy, thiserror) und die
additive Variante `PlayerOwnershipError::DetachedRelease(#[from] …)`. Alle
Release-Refusals sind damit typisiert und unterscheidbar.

## Tests (Memory **und** redb, Counter-Modus, echte Work/Completion)

19 neue Tests in `src/shard/player_ownership/counters/tests/release.rs`
(je Test beide Backends über `for redb in [false, true]`, echte
`InMemoryStore`/`RedbStore`-Ownershipadmission, `initialize_owned_item_counters`,
`run_tick_with_player_ownership_at`, echte `execute(&store)`-Completions; keine
Fake-Facts, keine Testbinder). Jede Refusal prüft per Fingerprint (Spielerdaten
inkl. beider neuen Bits, `player_bytes`, Pendings mit Status, Pending-/Input-
Bytes, Generation, Lifecycle-Zellen/Pins, Publicationgruppen, Entities, Tick,
Draining/Stopped), dass nichts verändert wurde.

| Matrixzeile §14 | Tests |
|---|---|
| Release-Refusals (A7) | `release_refuses_connected_snapshot_foreign_stale_and_repeated_handles_without_mutation`, `release_without_lifecycle_mode_is_a_typed_refusal_not_an_acknowledgement`, `release_gates_flip_one_at_a_time_unacked_unsaved_pending_then_success` (N-1/N/N+1, wiederholte Ack), `queued_inputs_naming_the_player_refuse_while_unrelated_queued_inputs_do_not` (Disconnect, Autosave, Move, Rejoin, Restore, Bootstrap; fremder Input und volle 32er-Queue blockieren nicht), `pinned_reserved_and_retained_take_and_item_reservation_refuse_typed`, `older_unacknowledged_admission_cell_is_a_collector_binding_refusal`, `stopped_owner_refuses_release_and_stop_readiness_typed`, `deferred_apply_after_detached_confirm_clears_proof_and_marks_unsaved` |
| `saved_after_detach` | `saved_after_detach_latest_confirmed_wins_new_pending_resets_and_uncertainty_blocks` (zwei bestätigte → letzte; echte ReceiptCollision-Refusal, dann zweiter Retry bestätigt; späteres Pending setzt zurück, `Pending`-Refusal; Lost-Reply → echte Reconcile → Uncertain blockiert; späterer echter Commit + Reconcile bestätigt), `autosave_admitted_connected_and_confirmed_detached_counts_but_connected_confirm_does_not`, `rejoin_publication_clears_proof_and_acknowledgement_of_the_new_admission` |
| `detached_owned_player_status` | `detached_status_refuses_connected_stale_foreign_snapshot_and_uninitialized_without_mutation` |
| Re-Export (R2-A4) | `tests/detached_release_api.rs` (4 Tests: Typidentität mit `ferrumc_storage`, typisierte Refusal, Getter- und Einstiegssignaturen ohne Query); 2 `compile_fail`-Doctests (kein Konstruktor für Status/Quittung) |
| `owned_stop_ready` vs. Stop | `owned_stop_ready_is_true_exactly_when_stop_alone_is_accepted_next_boundary` (8 Zustände: leer, dirty Spieler, Pending, ungeackte Zelle, dirty Item, retained Gruppe, alles geklärt inkl. Release, verbundener Spieler mit Zelle; je `ready == Stopped`, bei true `counter_steps == 0`), `owned_stop_ready_is_false_with_any_queued_input_and_refuses_snapshots` |
| Erfolg | `release_removes_state_bytes_and_counters_after_save_then_ack_on_both_backends`, `ack_before_save_keeps_lifecycle_acked_through_confirmed_autosave_publication` |
| Races | `release_versus_rejoin_in_both_orders_is_deterministic`, `player_cap_n_minus_one_n_n_plus_one_has_no_automatic_eviction` |
| R3-H2 | `refusal_query_accessor_names_completion_and_retry_inputs_only` |

Bestehende Sim-Tests sind unverändert (einzige Änderung in `counters/tests.rs`:
`mod release;`) und grün.

## Kandidat, Manifest und Checks

Kandidat und Manifest wurden **vor** den Checks abgelegt; readonly Kopie in
`.codex-tmp/sim03_4b-runtime/candidate-src/` (60 Dateien, alle readonly,
0 Abweichungen).

- `candidate.manifest.json` `03747d586401caf3804154e394cc4068628bd25c5788d6dee915c4ac7d8a1e27`:
  60 Simdateien; geändert 9, neu 4, entfernt 0; außerhalb des Sim-Crates 1242
  Dateien, 0 Abweichungen zur Basis.
- Geändert: `INVARIANTS.md` (`96d32577…`), `src/collector_lifecycle.rs`
  (`862e1d5c…`), `src/item_publication.rs` (`f929ab7a…`), `src/lib.rs`
  (`d3bd55d7…`), `src/player.rs` (`1c9cc275…`), `src/player/pending.rs`
  (`252160cc…`), `src/shard/player_ownership.rs` (`f452aa81…`),
  `src/shard/player_ownership/counters/tests.rs` (`fca18419…`),
  `src/shard/player_ownership/lifecycle.rs` (`a4d3fc1d…`).
- Neu: `src/player/detached.rs` (`f1a552d1…`),
  `src/shard/player_ownership/release.rs` (`cb180e70…`),
  `src/shard/player_ownership/counters/tests/release.rs` (`1bdac41f…`),
  `tests/detached_release_api.rs` (`b0db7863…`). Volle Hashes im Manifest.

Abschlusschecks `final-checks.sh` (`6f394905…`), detached, frisches
`CARGO_TARGET_DIR=target-final`, jeweils `--jobs 2 --locked`, cargo/rustc
1.99.0, ohne Source-Änderung während der Checks:

| Schritt | Ergebnis |
|---|---|
| `cargo test -p ferrumc-sim` (Debug) | exit 0; 11 Testläufe (inkl. Doctests), 394 bestanden, 0 fehlgeschlagen, 0 ignoriert |
| `cargo test -p ferrumc-sim --release` | exit 0; 394 / 0 / 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all --check` | exit 0 |
| `RUSTDOCFLAGS=-Dwarnings cargo doc -p ferrumc-sim --no-deps` | exit 0 |
| `cargo build --workspace --all-targets` | exit 0 |
| `cargo test -p ferrumc-session --no-fail-fast` | exit 0; 3 Testläufe, 159 / 0 / 0 |
| `cargo test -p ferrumc-testkit --no-fail-fast` | exit 0; 8 Testläufe, 100 / 0 / 0 |
| `cargo test -p ferrumc-bench --no-fail-fast` | exit 0; 4 Testläufe, 5 / 0 / 0 |
| `cargo test -p ferrumc-app --no-fail-fast` | exit 0; 40 Testläufe, 375 / 0 / 1 ignoriert (vorbestehend); `multiplayer_visibility` im ersten Lauf bestanden, kein Einzellauf nötig |

Sim-Verbraucher laut Manifesten: `ferrumc-session`, `ferrumc-app`,
`ferrumc-bench`; `ferrumc-testkit` zusätzlich wie gefordert.

**Nach den Checks:** `post.manifest.json` `415f719fe0cd7b13baa829c28aabacd6cb49bf2a01e09f1cf3e4e79c9a213736`:
Simdateien identisch zum Kandidaten, 1242 Außendateien unverändert zur Basis.
Einzige Zusatzdateien in der Kopie sind App-Testnebenprodukte unter
`work/.codex-tmp/`: `p57-plugin-testhost-2170394-artifacts/libplugin_testhost_fixture.so`
und `…/librenamed_fixture.so` (beide `60bddac251afa0441cb0216f34314166adbd74599b3ee7e73f5498ec32bf6ed8`)
sowie das leere Verzeichnis `p52-abi-invalid-input/2168718`. Die redb-Tests
schreiben temporäre Dateien `ferrumc-sim034b-*.redb` nach `std::env::temp_dir()`
und entfernen sie (wie die bestehenden DUR-04.2-Tests). Logs und Summary unter
`.codex-tmp/sim03_4b-runtime/final-logs/`; Freeze `delivery.freeze.json`.

## Entscheidungen und Fragen an Root

1. **Foreign vs. Stale.** `actual_player` meldet einen fremden Owner wie bisher
   als `StaleHandle` (Dokumentation „foreign or stale“, bestehende Tests). §7.3
   unterscheidet Foreign (Fail-Stop) von `StaleHandle` (Quarantäne); mit dem
   bestehenden Fehlertyp ist das nicht unterscheidbar. Ich habe keine neue
   Variante eingeführt, um bestehende Pfade nicht zu ändern. e2 muss beide als
   `StaleHandle` behandeln (Quarantäne, kein Release), sofern Root keine eigene
   Variante anordnet.
2. **Refusaltyp.** Neue Enum `DetachedReleaseRefusal` plus additive Variante
   `PlayerOwnershipError::DetachedRelease`. Der Status meldet „verbunden“ wie
   vertraglich als `Ineligible("connected player")`, der Release als
   `DetachedRelease(Connected)`.
3. **„Chunkreservierung“.** Chunk-Tickets tragen in Sim keine PlayerId
   (`TicketReason::Player` ohne Id). Die einzige spielerbezogene
   Chunkreservierung ist die Entity-Owner-Reservierung eines Item-/Pair-Pendings
   mit seinen betroffenen Chunks; sie ist `ItemReservation`.
4. **Bootstrap-Pending derselben PlayerId** kann nicht neben einer PlayerState
   existieren (`AlreadyOwned`); für ein releasefähiges Handle ist es
   unerreichbar. Bootstrap-/Restore-/Rejoin-**Inputs** derselben PlayerId
   verweigern als `QueuedInput` (getestet). Das Gate prüft Pendings dennoch
   generisch über `pending.player`.
5. **„Collector-Bindung“** ist jede weitere retained Lifecyclezelle derselben
   PlayerId (ältere, noch nicht geackte Admission). Das ist strenger als nötig
   und blockiert den Release bis zur Ack der alten Zelle (D7/D8 des
   `Superseded`-Records).
6. **Verbundene Spieler und Stop.** Im Lifecycle-Modus hält jede verbundene
   Admission ihre Zelle; `owned_stop_ready` ist dann wie der Stop-Apply `false`
   (getestet). Ein gestoppter Owner liefert `Err(Draining)`.
7. **Status nach Stop** ist nicht verweigert (Vertrag schweigt; rein lesend).
8. **Keine Allokation** ist strukturell begründet (nur geborgte Scans, Copy-
   Rückgaben); ein zählender Allocator ist wegen `unsafe_code = "forbid"` im
   Workspace nicht möglich.

## Nicht Teil dieser Lieferung

Session-Retention und Release-Caller (NET-04.2e2), App-Driver und Stop-Deadline
(DUR-05a, R3-A3), Isolierung quarantinierter Pendings (R3-A5), Live-Aktivierung,
Survival-, Paper- und Performanceaussagen.
