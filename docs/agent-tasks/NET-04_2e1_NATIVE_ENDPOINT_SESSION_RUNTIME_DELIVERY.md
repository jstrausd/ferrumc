# NET-04.2e1 — Delivery: native Endpoint-Art in der Session

Status: **Implementierung abgeschlossen, Checks grün, wartet auf unabhängigen
Root-Review.** 11.10.2026. Implementierungs-Subagent, exklusiver Owner von
`server/crates/ferrumc-session/**`. Keine Aktivierungs-, Survival-,
Performance-, Paper- oder Live-Client-Aussage; es gibt in diesem Ticket keinen
App-Driver, keinen echten Client und keine Messung.

## 1. Basis, Pins, Schreibscope

| Gegenstand | Wert |
|---|---|
| Basis | `main` `ce39d817aa7216beacc2db9bca236d3942910b98`, Archiv `.codex-tmp/net04_2e1-root-base/archive`, 1297 Dateien, `base.freeze.json` SHA-256 `4c7d0bd3d79702110a413e9ba2949cb010e0a5b81a3cc877ea32f091f593ee85` |
| Vertrag | `NET-04_2d_NATIVE_DRIVER_ENDPOINT.md` `e0530e0a522b8c2e57a06268206a7721b9ca512b2fd2af3f7f71126fcdbba14f` |
| .2c-Vertrag | `NET-04_2c_COLLECTOR_SESSION_BINDING.md` `3dff58ac5d1b9511862cf050018fb838ea5f8372b8768fd56a661ab949efd04c` |
| Rückfragen Runde 1 | `.codex-tmp/net04_2e1-runtime/ROOT_QUESTIONS.md` `2c8c5ebb1469c9da5809ce785b8b07fa29daf9bd1ceb6c9cb652ef91a343fbb8` |
| Root-Entscheidungen | `NET-04_2d_NATIVE_DRIVER_ENDPOINT_ROOT_DECISIONS.md` (R3-A1, R3-A2) und die Antworten F1–F4 dieser Vergabe |
| Arbeitskopie | `.codex-tmp/net04_2e1-runtime/work` (Archiv nie bearbeitet, frisch verifiziert 1297/1297) |

Geändert wurden ausschließlich `server/crates/ferrumc-session/**` (9 Dateien
geändert, 13 neu) und `server/Cargo.lock` (nur die zwei F2-Einträge), dazu
`.codex-tmp/net04_2e1-runtime/**` und dieses Dokument. Kein anderes Crate, keine
externe Dependency, kein Git.

## 2. Umsetzung der Root-Entscheidungen

### F1 — Umsetzung von R3-A1 (Revalidierungs-Signatur)

```rust
pub fn revalidate_native_endpoints(&mut self, current: &NativeShardReceiver,
    probe: &mut dyn FnMut(&OwnedPlayerHandle, NativeRevalidationProbe<'_>) -> NativeRevalidationVerdict,
) -> Result<NativeRevalidation, CollectorSessionError>;
impl NativeRevalidationProbe<'_> {
    pub fn submit(self, view: Option<ConnectedOwnedPlayerView<'_>>) -> NativeRevalidationVerdict;
}
```

- `NativeRevalidationProbe` hat keinen öffentlichen Konstruktor und kein
  `Clone`; `submit` verbraucht ihn und ist der einzige Weg zu einem
  `NativeRevalidationVerdict` (versiegelt, private Felder). `submit` vergleicht
  die View gegen das gespeicherte Handle (Spieler, exakter Owner/Admission über
  `matches_owned_player`) und die volle Generation der Registrierung; nur
  skalare Pose-Daten verlassen den Aufruf. Der Router prüft zusätzlich die
  routerlokale Probe-Sequenz (checked `u64`); ein Verdict einer anderen Probe
  gilt als `NativeEndpointRevalidationRefused`
  (`router::native::tests::a_verdict_of_another_probe_refuses_revalidation`).
- **Geteilter und exklusiver Shard-Borrow, getrennt von `&lane`:**
  `tests/native_borrow.rs` baut eine slotähnliche Struktur
  `{ shard: SimShard, lane: NativeShardReceiver }` und ruft
  `router.revalidate_native_endpoints(&slot.lane, …)` einmal mit
  `slot.shard.connected_owned_player_view(h)` (shared) und einmal mit
  `let shard = &mut slot.shard` (exklusiv) auf; nach `submit` mutiert dieselbe
  exklusive Leihe den Shard (`enqueue_owned_player_input(drain())`), die
  Mutation ist im nächsten echten Boundary als `OwnershipIntent::Draining`
  sichtbar. Beide Backends (Memory, redb).
- **`compile_fail`-Belege** (Doctests, zusätzlich standalone gegen die gebauten
  rlibs kompiliert, `evidence/compile_fail.json`):

| Beleg | Ort | tatsächlicher Compilerfehler |
|---|---|---|
| Vertragsform `FnMut(&H) -> Option<View<'_>>` | `native/revalidation.rs` Moduldoku | **E0597** `shard` does not live long enough |
| explizite `'s`-Variante mit exklusivem Capture | ebenda | „captured variable cannot escape `FnMut` closure body“ |
| Probe zweimal einreichen | `NativeRevalidationProbe` | **E0382** use of moved value `probe` |
| Probe aus der Closure herausschmuggeln | `NativeRevalidationProbe` | **E0521** borrowed data escapes outside of closure |
| Verdict fremd konstruieren | `NativeRevalidationVerdict` | private Felder (struct literal verweigert) |
| `NativeShardInput` konstruieren | `NativeShardInput` | **E0451** private Felder |
| `NativeShardReceiver`/`NativeJoinReservation` klonen | jeweilige Typdoku | **E0308** (kein `Clone`) |
| `PlayerSessionKey` fälschen | `PlayerSessionKey` | private Felder |

Dieselbe Form (Handle + einmalige Probe, Borrow am Shard, nicht am Handle) ist
für D7/D8 in e2 vorgesehen.

### F2 — echte Sim-Admissions in Tests (Option A)

- `ferrumc-storage` und `ferrumc-world` stehen **nur** unter
  `[dev-dependencies]` von `ferrumc-session`, mit Kommentar im Muster der
  bestehenden Einträge (warum nur Tests, Produktion nutzt sie nie).
- `Cargo.lock`: einzige Änderung sind genau diese zwei Einträge in der
  Dependency-Liste von `ferrumc-session`; keine neuen Pakete, keine
  Versionsänderung (`evidence/cargo-lock.diff`, SHA-256
  `0e7807a110ea5b61b48e69cdc4c320ee7417e6231e4a6b6f80bc99f8c27ceaa6`):

```diff
@@ -701,7 +701,9 @@
  "ferrumc-net",
  "ferrumc-proto",
  "ferrumc-sim",
+ "ferrumc-storage",
  "ferrumc-testkit",
+ "ferrumc-world",
  "thiserror",
```

- Source-Scan `tests/source_scan.rs`: kein Nicht-Test-Code unter `src/` nennt
  `ferrumc_storage`/`ferrumc_world` (auch nicht in Bindestrich-Schreibweise);
  ausgenommen sind nur `#[cfg(test)]`-Module (inline per Klammertiefe, Dateimodule
  inkl. `#[path]`) und `tests/`. Ein zweiter Test belegt, dass der Scanner eine
  Produktionserwähnung findet und gegatete Items überspringt.
- Session-INVARIANTS: neuer Abschnitt „Dependencies“ dokumentiert die
  Ausnahme; die CLAUDE.md-Regel (Session hängt nicht von Raw-Storage/-World ab)
  gilt für Produktion weiter. Die Generations-/Layoutprüfung der Produktion
  liest `GenerationBinding` nur über `ferrumc-sim`-Getter und das dokumentierte
  feste schema-1-Identity-Envelope (`encode()`), ohne World-Typen zu benennen.

### F3 — .2d-Ticketschnitt e1/e2 präzisiert

In e1 umgesetzt: versiegelter `PlayerSessionKey` (Weak-Routeridentität,
`PlayerId`, checked `NonZeroU64`-Inkarnation, `MAX` genau einmal); **ein**
checked Wire-Allocator `[2, i32::MAX]` ohne Wrapping (Legacy- und native Joins,
Verweigerung vor Slowviewer-Disconnect/`PlayerJoin`); `CollectorSessionLimits`
(6 × NonZero, Hardmaxima 1024/1023/262144/1024/4194304/1048576, Querregeln)
und `initialize_collector_sessions` mit `FlatOverworld1_21_8`;
`reserve_native_join`/`admit_native_join`/`cancel_native_join`; minimales
`disconnect_native`; native Endpointart, Exklusivität, Register/Replace/
Unregister, Suspension, Revalidierung, Lane mit `into_parts`,
`native_lane_input_live`, Regionsvorprüfung, Coalescing, INVARIANTS (H4).

In e2 verbleiben: Retention-Record/Token/Lineage, Leave-Save, Adoption,
Retirement (R1/R2), Audience/Reconcile/Collector-Baselines, Pose-/Resync-
Producer. **Folge:** Native Sessions sehen sich in e1 gegenseitig noch nicht
(keine Tab-/Spawn-Pakete zwischen nativen Sessions, kein Peer-Burst nach
`JoinGame`).

**Legacy-Auflage:** Alle bestehenden Legacy-Tests sind unverändert grün
(143 Unit + 15 `shard_directory` + 1 Doctest der Basis, Dateien bytegleich).
Die Legacy-ID wird weiterhin an der historischen Stelle nach `PlayerJoin`
vergeben; die Verfügbarkeit wird nur vorher geprüft, damit bleiben IDs im nicht
erschöpften Bereich identisch (`legacy_refused_join_consumes_no_wire_id`). Neu
ist nur `SessionError::EntityIdsExhausted` bei Erschöpfung
(`legacy_wire_allocator_issues_max_once_then_refuses_before_slow_viewer_and_player_join`:
`MAX-1`, `MAX`, dann Verweigerung ohne Slowviewer-Disconnect, ohne `PlayerJoin`,
ohne retained Leave; Kontrolllauf mit freier ID trennt denselben langsamen
Viewer). Das Sessions-Limit greift für Legacy nur opt-in nach
`initialize_collector_sessions` (`SessionError::SessionCapacity`).

**Übergabezustand `disconnect_native` → e2:** Der Eintrag bleibt in der
Session-Tabelle (zählt ins Sessions-Limit, blockiert seine UUID und die
Unregistrierung), Phase `Leaving`, eigenes Outbound geschlossen,
Bewegungszelle geschlossen, gespeichertes Original-Handle und `ever_active`
unverändert, Disconnect genau einmal `Sent` oder dedupliziert `Retained`
(Retry über `retry_native_disconnects` bzw. als erstes Item nach
Revalidierung). Es gibt weder Token noch Lineage noch Leave-Save; e2 erweitert
die Rückgabe zu `NativeRetentionToken` und wandelt denselben Eintrag ohne
Neuallokation in den Retention-Record. Es entsteht kein Zustand, den e2 nicht
übernehmen kann: es gibt keine teilweise entfernten Einträge, keine
herausgegebenen Handles und keine zweite Disconnect-Quelle.

### F4 — Coalescing (Umsetzung von R3-A2)

- Ein Bewegungs-Lane-Item trägt eine Zelle `Arc<MovementCell>` mit
  `std::sync::Mutex` je Session, geteilt von Router (Merge) und Receiver
  (Entnahme). **Warum ein Lock statt reiner Ownership:** ein `mpsc`-Eintrag ist
  nach `try_send` nicht mehr ersetzbar; ohne geteilte Zelle könnte der Router
  nur vor dem Senden puffern, was entweder keinen Lane-Platz spart oder die
  Zustellung bis zum nächsten Routeraufruf verzögert.
- Kritische Abschnitte sind O(1) ohne Allokation (Feldkopie bzw. `Weak`-Clone
  des Handles), nie über `.await`, nie zusammen mit einer anderen Sperre; die
  Zelle gehört genau einem Session-Eintrag (keine globale Zelle) und wird mit
  ihm bzw. mit dem Lane-Item freigegeben. Vergiftung: Merge verweigert
  (`MergeOutcome::Poisoned` → neuer Eintrag), Entnahme meldet typisiert
  `NativeLaneRecv::MovementCellPoisoned{session}` ohne `unwrap`.
- Schließen der Zelle: jedes Nicht-Bewegungs-Item, Teleport-Confirm,
  Regionskorrektur/Resync, Replacement, Revalidierung und Leave.
- Dokumentiert in INVARIANTS und in der Moduldoku `native/lane.rs`.

| Auflage | Test |
|---|---|
| Spam belegt einen Platz, neuester Stand | `movement_spam_uses_one_lane_slot_and_delivers_the_newest_state` (201 Bewegungen + Rotation → 1 Item, echte Sim-Pose/Look) |
| Bewegung hinter Nicht-Bewegung nicht vorgezogen | `movement_behind_a_non_movement_input_is_never_pulled_forward` |
| Teleport-Confirm, Resync/Regionskorrektur schließen | `teleport_confirm_resync_and_region_correction_close_the_cell`; Revalidierungs-Resync im Endpoint-Test |
| Entnahme vs. Ersetzung deterministisch | `take_before_replace_yields_old_then_new_and_replace_before_take_yields_new`; nebenläufig mit echten Threads `concurrent_take_and_replace_never_lose_or_reorder_the_newest_state` (2000 Schritte, streng steigend, letzter Stand angekommen) |
| Lane voll → `held ≤ 1` mit Key, ohne Verlust | `held_lane_input_keeps_its_key_and_fifo_and_is_dropped_once_not_live` (echte Sim-Enqueue-Refusal `Capacity`, Key/Klasse gehalten, FIFO, alle 3 Inputs angewandt) |
| Vergiftete Zelle | `poisoned_movement_cell_is_reported_typed_and_never_merged_again` |

## 3. Öffentliche API (neu)

`CollectorSessionLimits`, `CollectorSessionError`, `PlayerSessionKey`,
`NativeShardRegistration`, `NativeShardReceiver` (`try_recv`),
`NativeShardInput` (`session`, `class`, `into_parts`), `NativeInputClass`,
`NativeLaneRecv`, `NativeJoinReservation`, `NativeSpawnProjection`,
`NativeRevalidationProbe`, `NativeRevalidationVerdict`, `NativeRevalidation`,
`NativeRouted`, `NativeDisconnect`, `NativeSessionStatus`,
`NativeSessionPhase`, `NativeEndpointState`, `NativeUnregisterError`, die
`MAX_*`-Konstanten; `SessionRouter::{initialize_collector_sessions,
collector_session_limits, register_native_world_shard,
replace_native_shard_registration, unregister_native_shard_registration,
reserve_native_join, admit_native_join, cancel_native_join, disconnect_native,
retry_native_disconnects, route_native_play, route_native_game_mode,
native_lane_input_live, revalidate_native_endpoints, native_session_status,
native_session_count, native_reservation_count}`;
`PlayerSessionHandle::{session_key, native_spawn_projection}`; neue
`SessionError`-Varianten `NativeEndpointRequiresSession`,
`EndpointKindConflict`, `EntityIdsExhausted`, `SessionCapacity`. Alle
öffentlichen Items haben Rustdoc, keine öffentlichen Felder.

**Präzisierungen gegenüber dem Vertragstext** (keine abgeschwächte Regel):
1. `unregister_native_shard_registration` liefert im Fehlerfall
   `NativeUnregisterError` mit dem **unversehrten Receiver** zurück
   (`into_parts`), statt nur `CollectorSessionError`; sonst ginge die Lane bei
   `NativeEndpointInUse` verloren.
2. `PlayerSessionHandle::session_key()` liefert `Option`: Legacy-Handles
   minten keinen Key (Legacy-APIs unverändert, .2c).
3. `disconnect_native` liefert in e1 `NativeDisconnect::{Sent, Retained}`; e2
   erweitert auf `NativeRetentionToken`.
4. §6.6 „SimCommand::Event mit Sessionkey“ ist als
   `route_native_play(&key, &ServerboundPlayPacket)` und
   `route_native_game_mode(&key, GameMode)` konkretisiert.
5. Korrektur-/Resync-Pakete nutzen wie die bestehenden Legacy-Korrekturen
   Teleport-ID 0; die App-eigene `PendingTeleport`-ID-Vergabe ist NET-04.2f.
6. Die Session kennt in e1 als „letzte echte Sim-Pose“ nur die Pose aus der
   Admission- bzw. Revalidierungs-View; der laufende Pose-Producer
   (`native_player_changed`/`native_resync`) ist e2.

## 4. Abnahmematrix §14, e1-Zeilen

| Zeile | Tests |
|---|---|
| Native Registrierung; Legacy-Exact im selben Scope → `EndpointKindConflict`; Legacy-Route auf native → Input intakt | `legacy_and_native_coverage_never_share_a_scope` (beide Reihenfolgen, Legacy-Join/-Replace/-Unregister → `NativeEndpointRequiresSession`, native Lease-APIs auf Legacy → `LegacyEndpoint`), `a_legacy_input_resolved_to_a_native_endpoint_is_returned_intact` |
| Replacement mit alter Lease / ohne aktuellen Receiver | `replacement_suspends_and_closure_revalidation_rebinds_active_and_leaving_sessions` (fremde Lease, fremder Receiver, alte Lease nach Replacement; Endpoint unverändert) |
| Replacement korrekt mit Active- und Leaving-Session (e1-Teil) | ebenda, je Backend: alle suspendiert; alte Items vollständig vor neuen (alter Receiver bis `Disconnected`); Revalidierung per Closure: Active A + Leaving B mit View gebunden, Detached D ohne View bleibt suspendiert/`Detached`, falsche View E → `RevalidationRefused` + Leave; Disconnect C während Suspension retained und danach **erstes** Item der neuen Lane; Authoritative während Suspension → mandatory Sync auf die echte Pose |
| Unregister mit Session/Reservation (Record/Adoption: e2) | Endpoint-Test (`NativeEndpointInUse{sessions: 5}`, Receiver zurück und weiter live), `reservation_refusals_precede_every_side_effect` (`reservations: 2`, Erfolg nach Cancel), `replacement_without_sessions_and_unregister_success` (neue Lineage) |
| Lane voll / Sim-Enqueue verweigert (e1-Teil) | `full_lane_turns_data_into_a_controlled_leave_and_retains_the_disconnect`, `held_lane_input_keeps_its_key_and_fifo_and_is_dropped_once_not_live`, `native_lane_input_live` in Endpoint-/Lane-Tests |
| Bewegung über Regionsgrenze; Chunkgrenze ±ε; Restore außerhalb (e1-Teil) | `region_decisions_match_the_simulation_derivation_on_both_backends` (13 Kantenpositionen inkl. `-0.0`, `±1e-9`, `128.0`; Session- und echte Sim-Entscheidung je Position identisch; Korrektur mandatory + Action-Bar droppable), `invalid_transforms_are_corrected_without_a_lane_item`, Reservation außerhalb → `NativeAdmissionOutsideRegion` |

Weitere getrennte Refusal-/Malformed-/Grenz-/Race-Tests: Limits N-1/N/N+1 je
Hardmaximum und Querregeln; Registry-Layout (8 malformed Varianten),
Simulationsdistanz `-1/0/32/33/MIN/MAX`, Lane-Kapazität 1024/1025, zweite
Initialisierung, `NotEmpty`; jedes der 20 JoinGame-Felder einzeln falsch →
`JoinGameMismatch` ohne Seiteneffekt, danach genau einmal erfolgreich;
Admission-Mismatches (fremde Reservation, fremder/ersetzter Receiver,
Spieler, deklarierte Position bitgenau, Handle eines Rejoins); Reservation-
Refusals (Scope, Seed, Region, NaN/Inf/3.1e7, Duplicate, Sessions-Limit
N-1/N/N+1), Cancel verbrennt IDs; Session-/Wire-Zähler `MAX` einmal, dann
erschöpft; geteilter monotoner Allocator Legacy/Native; Hotbar `-1/9/MIN/MAX`
vs. `0/8`; nicht unterstützte Aktionen typisiert; fremde/Leaving-Keys.
Erfolgsfälle mit Decoderframe: erstes Outbound ist exakt der Router-
`JoinGame` (Mandatory/State, `assert_wire_frame`), Self-ID 1, Remote-ID aus
dem Allocator, Survival 0, Seed-Hash-Fixture `8828412754801553711`,
Spawn-Projektion bitgenau inkl. `-0.0`.

Alle Integrationstests laufen gegen echte bounded Router-Kanäle. Tests mit
Sim-Admission nutzen echte Admissions (Restore und receipt-gestützter
Bootstrap) im Counter-Modus (Init-Reihenfolge §10.2, Refill vor dem ersten
Boundary, `run_tick_with_player_ownership_at`), ohne Fake-Facts.

**Korrektur (Runde 3, Review-Befund B1):** In Runde 2 stand hier, alle
Integrationstests liefen auf **Memory und redb**. Das traf nicht zu. Die Zeile
„Lane voll / Sim-Enqueue verweigert“, die fünf F4-Pflichttests und
`invalid_transforms_…` liefen nur auf Memory. Seit Runde 3 laufen alle Tests
der e1-Matrixzeilen, die eine Sim-Admission brauchen, auf Memory **und** redb.
Das gilt auch für alle F4-Pflichttests. Welche Tests nur Memory oder gar keine
Admission nutzen, steht vollständig in §9.2/§9.3.

## 5. Grenzen und Backpressure

| Ressource | Grenze | Voll / Fehler |
|---|---|---|
| Native Lane | Router-Shard-Input-Kapazität, NonZero, ≤ 1024 (sonst `InvalidLimit`), Control-Reserve = Router-Reserve | Data aktiver Session → kontrollierter Leave (`NativeLaneFull`/`NativeLaneClosed`); Disconnect dedupliziert retained, Retry |
| Bewegungszellen | ≤ 1 offene je Session | Merge statt neuem Slot |
| Sessions + Reservationen | `CollectorSessionLimits::sessions` (≤ 1024), Legacy opt-in mitgezählt | `SessionCapacity` vor Seiteneffekten |
| Wire-IDs | `[2, i32::MAX]`, `MAX` einmal | `EntityIdsExhausted` vor Seiteneffekten |
| Session-Inkarnation | `NonZeroU64`, `MAX` einmal | `SessionIdsExhausted` |
| Revalidierung | ≤ eine Closure je suspendierter Session und Aufruf; Worklist fallibel reserviert | `AllocationFailed` ohne Zustandsänderung |

Alle Werte sind gewählte Vertragsgrenzen, nicht gemessen.

## 6. Checks (ohne Source-Änderung nach dem Kandidaten-Freeze)

Toolchain 1.99.0, alle Läufe `--jobs 2 --locked`, `CARGO_TARGET_DIR`
privat, Arbeitskopie `work/server`; Skript `evidence/run_checks.sh`, Exit-Codes
`logs/checks.txt` (alle 0).

| Check | Ergebnis | Log SHA-256 |
|---|---|---|
| `cargo test -p ferrumc-session` (Debug) | **205 bestanden**, 0 fehlgeschlagen: 150 Unit (143 Basis + 7 neu), `native_join` 11, `native_lane` 13, `native_endpoint` 2, `native_borrow` 2, `source_scan` 2, `shard_directory` 15 (Basis), 10 Doctests (1 Basis + 9 neue `compile_fail`) | `aff1fb89…` |
| `cargo test -p ferrumc-session --release` | **205 bestanden**, 0 fehlgeschlagen | `deae156a…` |
| `cargo clippy --workspace --all-targets -- -D warnings` | grün | `a178c790…` |
| `cargo fmt --all --check` | grün (leeres Log) | `e3b0c442…` |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-session --no-deps` | grün, uneingeschränkt (Basis: Exit 101, 9 Fundstellen, siehe §8) | `d598805f…` |
| `cargo build --workspace --all-targets` | grün | `765c69ba…` |
| `cargo test -p ferrumc-app` | **375 bestanden**, 0 fehlgeschlagen, 1 ignoriert (Basis-`#[ignore]`) | `f90efbf6…` |
| `cargo test -p ferrumc-testkit` | **100 bestanden**, 0 fehlgeschlagen | `62ebcc33…` |
| `compile_fail`-Beleg standalone | 10/10 Blöcke kompilieren nicht, erwartete Codes E0597/E0382 getroffen (`evidence/compile_fail.json` `cc8b8975…`) | `4cbb90d8…` |
| Basis-/Außendateien (`evidence/verify_base.py`) | Archiv 1297/1297 bytegleich, keine fehlenden/zusätzlichen Dateien; Arbeitskopie außerhalb des Scopes unverändert; Arbeitsbaum-`ferrumc-session/**` (16) und `Cargo.lock` vor der Übernahme bytegleich zur Basis (`evidence/verify_base.json` `32b14e76…`) | — |

Hinweis: Der Testkit-Test `plugin_testhost` legt unter `<repo>/.codex-tmp/`
zwei Fixture-`.so` ab; in der Arbeitskopie entstanden sie unter
`work/.codex-tmp/` und wurden als Testnebenprodukt nach
`evidence/test-byproducts/` verschoben (kein Source, außerhalb der Zählung).
Nach allen Checks: Kandidatenmanifest erneut 30/30 identisch zur Arbeitskopie.

## 7. Freeze

| Artefakt | Wert |
|---|---|
| Kandidat `.codex-tmp/net04_2e1-runtime/candidate/` | 30 Dateien (`ferrumc-session/**` 29 + `Cargo.lock`), Aggregat `8c2eed2cda8c4ce2f9f5acf7d7269ec98632ba15b29ebc1879049ab77c80605e` |
| Kandidatenmanifest `evidence/candidate.manifest.json` | `0f7f4b9989afe33ca899e0b218fff9e62a37d1944d2792f8c48886e23d5fd1c1` (vor den Checks abgelegt) |
| Evidence-Freeze `.codex-tmp/net04_2e1-runtime/evidence.freeze.json` | `0018cba417e06c9c25db25825f0115cb2b555d731e7531d661ebb0c0fca46cab`, 24 Dateien, Aggregat `143e126bacd89142f2343273100adb87707c6219c64761af23a16e74b1c08d31` |
| `Cargo.lock`-Diff | `0e7807a110ea5b61b48e69cdc4c320ee7417e6231e4a6b6f80bc99f8c27ceaa6` |
| Übernahme in den Arbeitsbaum | `server/crates/ferrumc-session/**` und `server/Cargo.lock` nach geprüfter Bytegleichheit zur Basis kopiert; danach 30/30 identisch zum Kandidatenmanifest |

Git, Board und Integration übernimmt Root.

## 8. Offene Punkte und bekannte Einschränkungen

- e2: Retention/Token/Lineage, Leave-Save, Adoption, Retirement, Audience/
  Reconcile, Pose-/Resync-Producer; bis dahin sehen sich native Sessions nicht
  und Leaving-Einträge werden nicht entfernt.
- NET-04.2f: Driver (`held`, Gate, Revalidierungsaufruf, Teleport-IDs,
  JoinKit), Foreign-Owner-Fälle am echten Apply.
- Vorbestehende Rustdoc-Baseline: Die Basis scheitert unter
  `RUSTDOCFLAGS="-D warnings"` an 9 privaten Intra-Doc-Links in
  `presentation.rs`, `router.rs`, `text.rs` (`logs/base-rustdoc-session.log`,
  Exit 101). Da das Crate im Scope liegt, wurden diese Links als reine
  Doku-Korrektur in Code-Formatierung umgewandelt; damit ist Session-Rustdoc
  uneingeschränkt grün. Keine Verhaltensänderung.
- Keine Aktivierungs-, Survival-, Performance- oder Live-Client-Abnahme.

## 9. Runde 3 — Nachbesserung nach dem Root-Review (B1, H2)

11.10.2026, Implementierungs-Subagent, exklusiver Owner von
`server/crates/ferrumc-session/**`. Grundlage:

- Review `NET-04_2e1_NATIVE_ENDPOINT_SESSION_RUNTIME_REVIEW.md`
  (`aa59539912f6f488fa65c64f17298572ae08193913ad949935ba9a1559a7bbd2`);
- `final-assessment.json` (`7c833ff7ec4400c73856cd3d96f7062f56bb8cc545515844f475ecabe83fff74`);
- dieses Delivery im Stand von Runde 2
  (`3969dc4c28ed34f6a000b1656e73a938cb1dc56fb23823ade5a98289a49d00a4`);
- Runde-2-Kandidat `evidence/candidate.manifest.json` (`0f7f4b99…`).

Vertrag, Root-Entscheidungen und F1–F4 gelten unverändert. Umgesetzt sind nur
B1 und Hinweis H2. A1–A7 sind **nicht** umgesetzt (§9.7).

### 9.1 Änderungen

Nur zwei Testdateien sind geändert (`round3-session.diff` gegen
`round2-candidate-src`, SHA-256 `f80c2b09…`). Unverändert bleiben
Produktionscode, `Cargo.toml` und `Cargo.lock`; letzteres ist bytegleich zu
Runde 2. Es gibt keine Verhaltensänderung.

- `tests/native_lane.rs`:
  - Diese Tests laufen jetzt über `for store in STORES` (Memory **und** redb),
    jeweils mit echter Admission über `fixture(store, …)`:
    - `full_lane_turns_data_into_a_controlled_leave_and_retains_the_disconnect`;
    - `held_lane_input_keeps_its_key_and_fifo_and_is_dropped_once_not_live`;
    - die fünf F4-Pflichttests `movement_spam_…`, `movement_behind_…`,
      `teleport_confirm_resync_…`, `take_before_replace_…` und
      `concurrent_take_and_replace_…`;
    - zusätzlich `invalid_transforms_are_corrected_without_a_lane_item`. Er
      hatte dieselbe Lücke in Matrixzeile 6; der Review nennt ihn nicht.
  - Der Race-Test hat jetzt Zweigzähler (§9.4).
- `src/router/native_tests.rs` (`#[cfg(test)] #[path]`): neuer
  deterministischer Test
  `each_take_and_replace_branch_of_a_movement_cell_is_decided_deterministically`
  (§9.4) und angepasste Moduldoku.

### 9.2 e1-Matrix §14: Testfunktionen und Backends

„M+R“ heißt: Der Test läuft in einer Schleife über `STORES`, mit echter
Admission auf Memory und redb. „keine Admission“ heißt: Der Test prüft reine
Router- und Directory-Logik vor jeder Sim-Admission, ein Backend ist dafür
nicht nötig.

| Zeile | Testfunktion (Datei) | Backends |
|---|---|---|
| Native Registrierung; Legacy-Exact → `EndpointKindConflict`; Legacy-Route → Input intakt | `legacy_and_native_coverage_never_share_a_scope` (`native_join.rs`) | keine Admission |
| | `native_registration_requires_initialization_and_the_layout_scope` (`native_join.rs`) | keine Admission |
| | `a_legacy_input_resolved_to_a_native_endpoint_is_returned_intact` (`router/native_tests.rs`) | keine Admission |
| Replacement mit alter Lease / ohne aktuellen Receiver | `replacement_suspends_and_closure_revalidation_rebinds_active_and_leaving_sessions` (`native_endpoint.rs`) | M+R |
| | `admission_mismatches_are_refused_on_both_backends` (`native_join.rs`; Admission über den ersetzten Receiver → `StaleReceiver`) | M+R |
| | `replacement_without_sessions_and_unregister_success` (`native_endpoint.rs`) | keine Admission |
| Replacement korrekt mit Active + Leaving (e1-Teil) | `replacement_suspends_…` (`native_endpoint.rs`) | M+R |
| | `revalidation_closure_takes_a_shared_shard_borrow_beside_the_lane_borrow`, `revalidation_closure_takes_an_exclusive_shard_borrow_and_mutates_after_submit` (`native_borrow.rs`) | M+R |
| Unregister mit Session/Reservation (Record/Adoption: e2) | `replacement_suspends_…` (`NativeEndpointInUse{sessions: 5}`) | M+R |
| | `reservation_refusals_precede_every_side_effect` (`reservations: 2`) (`native_join.rs`) | keine Admission |
| | `replacement_without_sessions_and_unregister_success` | keine Admission |
| Lane voll / Sim-Enqueue verweigert (e1-Teil) | `full_lane_turns_data_into_a_controlled_leave_and_retains_the_disconnect` (`native_lane.rs`) | **M+R (neu in Runde 3)** |
| | `held_lane_input_keeps_its_key_and_fifo_and_is_dropped_once_not_live` (`native_lane.rs`; echte Sim-Refusal `Capacity`) | **M+R (neu in Runde 3)** |
| | `native_lane_input_live` in `replacement_suspends_…` | M+R |
| Bewegung über Regionsgrenze; Chunkgrenze ±ε; Restore außerhalb (e1-Teil) | `region_decisions_match_the_simulation_derivation_on_both_backends` (`native_lane.rs`) | M+R |
| | `invalid_transforms_are_corrected_without_a_lane_item` (`native_lane.rs`) | **M+R (neu in Runde 3)** |
| | Reservation außerhalb → `NativeAdmissionOutsideRegion` in `reservation_refusals_…` | keine Admission (Refusal vor der Sim-Admission) |

Einschränkung wie im Review (§8): Für „Session detached → held verworfen“ ist
in e1 nur „Leaving → Data nicht mehr live“ belegt. „Control nach DetachFact“
ist e2 (A4).

**F4-Pflichttests** (Tabelle in §2):

- `movement_spam_…`, `movement_behind_…`, `teleport_confirm_resync_…`,
  `take_before_replace_…`, `concurrent_take_and_replace_…` und `held_lane_…`:
  M+R, alle neu in Runde 3.
- Crate-intern und nur auf Memory: `poisoned_movement_cell_…` und der neue
  `each_take_and_replace_branch_…`. Beide prüfen reine Zellen- und
  Router-Logik und verwenden die Memory-Admission aus `native_tests.rs`.

### 9.3 Tests nur auf Memory oder ohne Admission (keine Matrixzeile)

Aussagen über die Backends gelten nur so, wie sie hier aufgeschlüsselt sind.

- **M+R (16 Tests):**
  - `native_borrow` 2;
  - `native_endpoint` 1 (`replacement_suspends_…`);
  - `native_join` 2 (`restore_and_bootstrap_…`, `admission_mismatches_…`);
  - `native_lane` 11 (alle außer den zwei unten genannten).
- **Nur Memory, mit echter Admission:**
  - `malformed_and_unsupported_client_events_are_refused_typed`;
  - `foreign_stale_and_leaving_keys_never_address_a_session`, der u. a.
    `native_lane_input_live` für einen fremden Key prüft;
  - `every_join_game_field_mismatch_is_refused_before_any_side_effect`;
  - `cancelled_reservations_never_reissue_wire_ids_or_incarnations`;
  - crate-intern: `poisoned_movement_cell_…`, `each_take_and_replace_branch_…`
    und `a_verdict_of_another_probe_refuses_revalidation`.
- **Ohne Admission:**
  - `limits_*` (2);
  - `initialization_validates_…`;
  - `native_registration_requires_…`;
  - `legacy_and_native_coverage_…`;
  - `reservation_refusals_…`;
  - `legacy_joins_count_…`;
  - `replacement_without_sessions_…`;
  - die vier crate-internen Allocator-Tests und `a_legacy_input_resolved_…`;
  - `source_scan` (2), `shard_directory` (15, Basis) und die 10 Doctests.

### 9.4 Hinweis H2: Beide Race-Zweige belegt

Die Lösung ändert kein Produktionsverhalten. Die Zähler leiten sich aus dem
öffentlichen Ergebnis von `route_native_play` ab, ohne cfg(test)-Hook im Lib-Code.

- **`concurrent_take_and_replace_never_lose_or_reorder_the_newest_state`**
  (M+R, echte Threads, öffentliche Router-API):
  - **Zähler:**
    - `Queued` nach dem ersten Schritt heißt „Entnahme vor Ersetzung“: Die
      Zelle war genommen (`MergeOutcome::Taken`), es entsteht ein neuer
      Eintrag.
    - `Coalesced` heißt „Ersetzung vor Entnahme“ (`Merged`).
  - **Abbruch:**
    - Der Producer läuft mindestens 2000 Schritte, danach so lange, bis beide
      Zähler > 0 sind, höchstens bis `MAX_STEPS = 100_000`.
    - Die Positionen sind exakt (Schrittweite 2⁻¹⁰) und bleiben in der Region.
    - Fehlt nach 2000 Schritten noch ein Zweig, gibt der Producer zwischen den
      Schritten per `yield_now()` ab. Der Wettlauf selbst bleibt
      unsynchronisiert.
  - **Zusicherungen:**
    - beide Zähler > 0;
    - Zähler + 1 = Schritte;
    - Zahl der entnommenen Einträge = Zahl der `Queued`, also jeder Eintrag
      genau einmal entnommen und kein Slot für `Coalesced`;
    - entnommene Werte streng steigend;
    - der letzte Stand kommt an.
  - Die Zähler stehen je Lauf in der Ausgabe.
- **Deterministisch je Zweig** (`router/native_tests.rs`):
  - Ersetzung vor Entnahme: `Coalesced`, der neuere Stand wird geliefert;
  - Entnahme vor Ersetzung: `Queued`, ein neuer Eintrag entsteht;
  - zusätzlich der vom Review genannte Fall „Dequeue vor dem Zell-Lock,
    Ersetzung dazwischen“: Der Eintrag ist aus dem mpsc entnommen, aber die
    Zelle noch nicht genommen. Der Merge landet trotzdem darin (`Coalesced`,
    kein zweiter Eintrag). Die Entnahme über den Receiver-Pfad liefert den
    neueren Stand (x = 15). Die nächste Bewegung ist wieder `Queued`.
- **Belege** (`round3-logs/race-repeat-{debug,release}.log`): je 25 Wiederholungen
  in Debug und Release nach dem Freeze, jede mit Memory und redb, alle grün.

| Profil | Backend | Schritte | Entnahme vor Ersetzung | Ersetzung vor Entnahme |
|---|---|---|---|---|
| Debug | Memory | 2000–2001 | 1–1053 | 946–1999 |
| Debug | redb | 2000–2001 | 1–1271 | 728–1999 |
| Release | Memory | 2000–2003 | 1–79 | 1920–2001 |
| Release | redb | 2000–2004 | 1–97 | 1902–2002 |

**Offengelegt: Zwischenstand v1.** Der erste Runde-3-Freeze
(`round3-v1-candidate.manifest.json`, `3d68182e…`) hatte noch keinen Yield.

- Im Release-Build unter Last brauchte der Producer dort bis zu **43 005**
  Schritte, bis „Entnahme vor Ersetzung“ auftrat. Die Last lag bei etwa 9–10
  auf 4 Kernen, mit parallelen Fremdprozessen. Der Abstand zur Grenze war zu
  knapp.
- Deshalb kam der Yield hinzu. Danach wurde neu eingefroren, und alle Checks
  liefen erneut in einem frischen Target.
- Die v1-Artefakte liegen unter `round3-v1-*` und sind im Evidence-Freeze
  enthalten.

### 9.5 Checks (finaler Kandidat, ohne Source-Änderung nach dem Freeze)

**Durchführung:**

- private Kopie `.codex-tmp/net04_2e1-runtime/round3-isolated` aus
  `git archive` `main ce39d817aa7216beacc2db9bca236d3942910b98` plus
  Session-Overlay;
- frisches `CARGO_TARGET_DIR` `round3-target-final`, `CARGO_INCREMENTAL=0`;
- cargo/rustc 1.99.0, alle Läufe mit `--jobs 2 --locked`;
- Skript `round3-run_checks.sh` (`4438c149…`), Exit-Codes in
  `round3-logs/checks.txt` (`8f99a743…`).

**Integrität der Kopie:**

- Gegen `base.freeze.json` (`4c7d0bd3…`) sind 1280 Dateien bytegleich, 17 vom
  Kandidaten überlagert und 13 neu. Es gibt keine Abweichung und keine
  Zusatzdatei (`round3-verify-base.json`).
- Nach den Checks: Kandidat 30/30 identisch.

| Check | Exit | Ergebnis | Log SHA-256 |
|---|---|---|---|
| `cargo test -p ferrumc-session` (Debug) | 0 | **206 bestanden**, 0 fehlgeschlagen: Unit 151 (Runde 2: 150, +1 `each_take_…`), `native_borrow` 2, `native_endpoint` 2, `native_join` 11, `native_lane` 13, `shard_directory` 15, `source_scan` 2, Doctests 10 | `ecd79080…` |
| dito `--release` | 0 | **206 bestanden**, 0 fehlgeschlagen, gleiche Aufschlüsselung | `4b0db4ca…` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | grün | `c9c09bf7…` |
| `cargo fmt --all --check` | 0 | grün (leeres Log) | `e3b0c442…` |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-session --no-deps` | 0 | grün; nur Session, keine Workspace-Aussage (Review H9) | `6893ab35…` |
| `cargo build --workspace --all-targets` | 0 | grün | `42cb942e…` |
| `cargo test -p ferrumc-app --no-fail-fast` | 0 | **375 bestanden**, 0 fehlgeschlagen, 1 ignoriert (Basis-`#[ignore]`) | `35262d75…` |
| `cargo test -p ferrumc-testkit --no-fail-fast` | 0 | **100 bestanden**, 0 fehlgeschlagen | `984479a3…` |
| Race-Wiederholung Debug / Release (je 25×) | 0 / 0 | alle grün, Zähler siehe §9.4 | `768d641e…` / `b50744d9…` |
| `each_take_and_replace_branch_…` einzeln | 0 | grün | `a4b3aca3…` |

**Offengelegt: App-Test im v1-Lauf.**

- Im ersten App-Lauf (v1, `round3-v1-logs/app-test.log`, `cb4dcaff…`) endete
  `cargo test -p ferrumc-app` mit Exit 101.
- Ursache: Beide Tests in `app/tests/multiplayer_visibility.rs` liefen in ihren
  10-s-Wall-Clock-Guard (`Elapsed`). Weil kein `--no-fail-fast` gesetzt war,
  liefen die folgenden App-Test-Binaries nicht.
- Wiederholung ohne Source-Änderung mit `--no-fail-fast`: 375 bestanden,
  1 ignoriert (`app-test-rerun1.log`, `20d2dc86…`).
- Fünf Einzelläufe dieses Binaries unter Last 9–10: 1× gescheitert, 4× grün,
  Laufzeiten 4,0–9,2 s (`multiplayer-visibility-repeat.log`, `c6c850b9…`).
- Die Eingaben von `ferrumc-app` sind gegenüber Runde 2 bytegleich. Geändert
  sind nur ein `#[cfg(test)]`-Modul und ein Integrationstest von Session; der
  App-Test liegt außerhalb des Scopes.
- Bewertung: lastabhängige Timing-Flakiness eines unveränderten Tests, nicht
  durch Runde 3 verursacht. Im finalen Lauf war er grün.

**Testnebenprodukte, separat ausgewiesen:** Im finalen Lauf entstanden in der
Kopie:

- `.codex-tmp/p57-plugin-testhost-2043284-artifacts/{libplugin_testhost_fixture.so,librenamed_fixture.so}`
  (je 8 222 552 Byte, SHA-256 `c9d833b4…`), aus dem p57-Test
  `plugin_testhost`;
- leere Verzeichnisse `.codex-tmp/p52-abi-invalid-input/2042158`.

Die `.so`-Dateien liegen jetzt unter `round3-test-byproducts/` (`SHA256SUMS`
`75c52c72…`), die leeren Verzeichnisse sind entfernt. Für v1 gilt dasselbe
unter `round3-v1-test-byproducts/`. Sie sind kein Source und außerhalb der
Zählung. Danach entspricht der Dateibestand der Kopie wieder dem Stand vor den
Checks. Die Dateilisten vor und nach den Checks liegen im Evidence-Freeze
(§9.6).

### 9.6 Freeze Runde 3

| Artefakt | Wert |
|---|---|
| Kandidat `round3-candidate-src/` (schreibgeschützt) | 30 Dateien (`ferrumc-session/**` 29 + `Cargo.lock`), Aggregat `e7ae6949439bcdfffac33f2870cfb33028d8ed305011f20359cfb41743a9dc43` |
| `round3-candidate.manifest.json` (vor den Checks abgelegt) | `665feb869536df999d544433cd73227f05db2a6e9089ee764e9863cad54d3c56` |
| Diff Runde 2 → 3 `round3-session.diff` | `f80c2b09e1462535ed902008cffca0568735805d011a518bff09094f41adbd8b` (2 Dateien) |
| Ergebnisse `round3-results.json` | `35bfa4a4194f96380478d173a953b548ddddf83cd0b22cea4ff786d6bf75142f` |
| Evidence-Freeze `round3-evidence.freeze.json` | `7793945ec6e760c50d6beffd185b4a48964f011846602019bb1328659f9b5dfa`, 52 Dateien, Aggregat `60cc2d616ead36af3501be00ebdc3cfdd611b358428658cc83184b3b423edb9a` |
| Übernahme in den Arbeitsbaum | Vorher frisch geprüft: Arbeitsbaum 30/30 = Runde-2-Manifest. Danach nur die zwei geänderten Dateien kopiert. Arbeitsbaum jetzt 30/30 identisch zum Runde-3-Manifest; `server/Cargo.lock` unverändert |

`round2-candidate-src/` und die Evidenz von Runde 2 sind unverändert. Git,
Board und Integration übernimmt Root. Im Arbeitsbaum ändert ein anderer Agent
`ferrumc-storage`. Die Checks oben decken ausschließlich `main ce39d817` plus
diesen Kandidaten ab.

### 9.7 Offene Auflagen für e2/f

Diese Auflagen gehören zu e2/f und sind in Runde 3 bewusst **nicht**
umgesetzt. Die Verweise beziehen sich auf den Review, §6 „Auflagen“.

1. **A1 (e2): nicht offengelegte Abweichung von R3-3 bzw. Vertrag §5
   Schritt 6.**
   - In e1 zählt die Reservation zwar in das Sessions-Limit, alloziert aber
     keinen Tabelleneintrag, den Session oder Adoption später übernehmen.
     `native_reservations.insert` (`router/native.rs:499`) und
     `native_sessions.insert` (`:605`) sind zwei getrennte, infallible
     `BTreeMap`-Inserts. Es gibt weder `AllocationFailed` noch denselben
     Eintrag.
   - Diese Abweichung fehlte in den Präzisierungen in §3. Damit ist sie
     hiermit offengelegt.
   - e2 braucht eine einheitliche, vorreservierte Tabelle (Reservation →
     Session/Adoption → Retention ohne Neuallokation).
   - Die Aussage in §2 (F3), e2 übernehme „denselben Eintrag ohne
     Neuallokation“, gilt erst nach dieser Umstellung.
   - Review §6, Auflage 1.
2. **A2 (e2/f): Disposition nach `RevalidationRefused`.**
   - Bei `RevalidationRefused` bleibt der Sim-Spieler verbunden. Der
     Disconnect bleibt dauerhaft `Retained` (`flush_disconnect` sendet nur bei
     `Bound`), Stop wird nie quiescent.
   - `retry_native_disconnects()` zählt Refused- und Detached-Sessions für
     immer als „retained“.
   - Review §6, Auflage 2, Probe P3.
3. **A3 (e2, vor jeder Aktivierung): Pose-Quelle für Korrekturen und Resync.**
   - Korrekturen synchronisieren auf die Admissions- bzw. Revalidierungs-Pose.
   - Der Revalidierungs-Resync liest vor dem Anwenden geleerter Alt-Lane-Items.
   - Ein verworfener Hotbar-Wechsel bekommt keinen Held-Slot-Resync.
   - Review §6, Auflage 3.
4. **A4 (e2): `native_lane_input_live(Control)` nach dem DetachFact** muss
   `false` liefern (R2-A5). Das setzt den DetachFact aus e2 voraus. Review §6,
   Auflage 4.
5. **A5 (e2): `disconnect_native` → `NativeRetentionToken`** auf demselben
   Eintrag. `Retained` für Detached-Sessions ist irreführend. Review §6,
   Auflage 5.
6. **A6 (f): Teleport-IDs aus `PendingTeleport`** statt ID 0. Bis zum
   passenden Confirm werden Bewegungen verworfen. Review §6, Auflage 6.
7. **A7 (Root): Vertragsnachtrag im Decision-Log** für folgende Punkte:
   - Unregister-Fehler mit Receiver;
   - `session_key() -> Option`;
   - `route_native_play` und `route_native_game_mode`;
   - typisierte Revalidierungs-Verweigerung über
     `NativeEndpointState::RevalidationRefused` und
     `NativeRevalidation::refused` statt einer Fehlervariante.

   Review §6, Auflage 7.

Die übrigen Hinweise des Reviews (H1, H3–H12) sind in Runde 3 nicht bearbeitet
und bleiben wie im Review beschrieben offen.

### 9.8 Nicht-Aussagen

Es gibt keine Aktivierungs-, Survival-, Performance-, Paper- oder
Live-Client-Aussage. Die Race-Zählerwerte sind Belege für das Auftreten der
Zweige, keine Messung. Die Workspace-Rustdoc-Baseline (Review H9) ist
unverändert und nicht Teil dieser Aussage.
