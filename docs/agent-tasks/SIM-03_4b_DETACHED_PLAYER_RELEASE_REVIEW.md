# SIM-03.4b — Freigabe abgemeldeter nativer Spielerzustände: unabhängiger Runtime-Review

Reviewer-Subagent (hat den Kandidaten nicht erstellt), 11.10.2026. Kein Git,
keine Agents, keine Änderung an Kandidat, Evidenz oder Produktion. Eigene
Ablage: `.codex-tmp/sim03_4b-runtime-root-review/**` (außer `combined/`).

## Verdikt

**CLEAN_RUNTIME**

Keine blockierenden, hohen oder mittleren Befunde. Alle geforderten frischen Checks auf main `8da4c770` + Kandidat sind grün, ferrumc-session (mit NET-04.2e1) kompiliert und testet grün gegen das neue ferrumc-sim, `combined/` und die 60 Kandidatendateien sind danach unverändert. Fünf eigene Laufzeit-Probes (Memory und redb, Debug und Release) sowie zwei Compile-Probes bestätigen das Verhalten. Die Root-Entscheidungen 1–7 sind am Code bestätigt.

Einziger Befund **N1 (niedrig, Dokumentation)**: Moduldoku, INVARIANTS und Delivery beschreiben die Pending-Scans als einfach bzw. doppelt, tatsächlich sind es drei. Die Vertragsgrenze von 833 Eintragsbesuchen bleibt eingehalten (≈ 769). Nicht blockierend; Korrektur bei der Integration empfohlen. Dazu drei Hinweise (H1–H3) für e2/f.

## Prüfbasis

| Gegenstand | Wert |
|---|---|
| Kombination | `combined/` = `git archive main 8da4c770222d7d40b747ce2abb7a08ff9706fa25` (mit NET-04.2e1) + 60 Sim-Dateien + Delivery; 1318 Dateien |
| `combined.freeze.json` | `a124cbfc85a4f24f93ff06e516bb145871af1f86b688ca3ebc934bcdcde406f3`; vor und nach den Checks 0 Abweichungen, keine Zusatzdatei |
| Gegenprobe gegen Git | 1313 main-Dateien: genau die 9 geänderten Sim-Dateien weichen ab, 0 fehlen; Zusatz genau 4 neue Sim-Dateien + Delivery (`logs/combined-vs-main.json`) |
| Kandidat | `candidate.manifest.json` `03747d586401caf3804154e394cc4068628bd25c5788d6dee915c4ac7d8a1e27`; 60 Dateien gleich zu `combined/`, `candidate-src/` und Arbeitsbaum (`logs/verify-pre.json`, `logs/checks/verify-after.json`) |
| Delivery | `01dbe30bb1f582e4ab4b29bacb75b3468d9e184a6db9054bd85e828a56be3825` (Arbeitsbaum = `combined/`) |
| Delivery-Freeze | `9fe42b1bd9d0fe3c940a19b065231bd11269442444116df36af62c81e34fc6b9`; 139 Einträge frisch nachgehasht, 0 Abweichungen |
| Vertrag | `NET-04_2d_NATIVE_DRIVER_ENDPOINT.md` `e0530e0a522b8c2e57a06268206a7721b9ca512b2fd2af3f7f71126fcdbba14f` (frisch) |
| Sim-Änderung auf main seit d7f16b21 | keine (`git diff d7f16b21 8da4c770 -- server/crates/ferrumc-sim` leer); main änderte nur Session, Docs und 2 Lockfile-Zeilen |
| Toolchain | cargo/rustc 1.99.0, `--jobs 2 --locked`, frisches `CARGO_TARGET_DIR=target-fresh` |

## Teil A — Ausführung (frische Checks auf der Kombination)

Skript `tools/checks.sh`, detached, Logs unter `logs/checks/`. Arbeitskopie
`work/` = `combined/` (vor und nach den Checks bytegleich, `verify-before/after.json`).

| Schritt | Ergebnis auf der Kombination | Delivery (auf d7f16b21) |
|---|---|---|
| `cargo test -p ferrumc-sim` (Debug) | exit 0; 11 Läufe, **394** bestanden, 0 fehlgeschlagen, 0 ignoriert (54 s) | 394 / 0 / 0 |
| `cargo test -p ferrumc-sim --release` | exit 0; 11 Läufe, **394** / 0 / 0 (182 s) | 394 / 0 / 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, 0 Warnungen | exit 0 |
| `cargo fmt --all --check` | exit 0 | exit 0 |
| `RUSTDOCFLAGS=-Dwarnings cargo doc -p ferrumc-sim --no-deps` | exit 0 | exit 0 |
| zusätzlich `RUSTDOCFLAGS=-Dwarnings cargo doc -p ferrumc-session --no-deps` | exit 0 | — |
| `cargo build --workspace --all-targets` | exit 0 (216 s) | exit 0 |
| `cargo test -p ferrumc-session --no-fail-fast` | exit 0; 8 Läufe, **206** / 0 / 0 (lib 151, native_borrow 2, native_endpoint 2, native_join 11, native_lane 13, shard_directory 15, source_scan 2, Doctests 10) | 3 Läufe, 159 / 0 / 0 (vor NET-04.2e1) |
| `cargo test -p ferrumc-testkit --no-fail-fast` | exit 0; 8 Läufe, **100** / 0 / 0 | 100 |
| `cargo test -p ferrumc-bench --no-fail-fast` | exit 0; 4 Läufe, **5** / 0 / 0 | 5 |
| `cargo test -p ferrumc-app --no-fail-fast` | exit 0; 40 Läufe, **375** / 0 / **1 ignoriert** (`genuine_paper_api_plugin_…`, braucht Java/JAR-Pfade, vorbestehend) | 375 / 0 / 1 |

`multiplayer_visibility` bestand im ersten Lauf (2/2, 3,83 s); kein Einzellauf
nötig. ferrumc-session auf main nutzt die nativen Sim-APIs (NET-04.2e1) und ist
mit dem neuen ferrumc-sim kompilier- und testgrün. Delta der Sim-Tests zu main:
361 → 384 Testattribute, 9 → 11 `compile_fail`-Doctests; +25 = 394.

Nach den Checks: `combined/` 1318 Dateien, 0 Abweichungen; 60 Kandidatendateien
gleich zu Manifest, `combined/`, `candidate-src/` und Arbeitsbaum; `work/`
außerhalb `.codex-tmp/` bytegleich zu `combined/` (`logs/verify-final.json`).
App-Testnebenprodukte nur in `work/.codex-tmp/`:
`p57-plugin-testhost-2244256-artifacts/{libplugin_testhost_fixture.so,librenamed_fixture.so}`
(beide `f04e1839…`) und leeres `p52-abi-invalid-input/2242551`.

## Teil B — Eigene Probes

Getrennte Kopie `work-probe/` = `combined/` + nur diese Reviewerdateien:
`…/counters/tests/release.rs` (+2 Zeilen `#[cfg(test)] mod probe_review;`),
neu `…/counters/tests/release/probe_review.rs` (`d1a71c2e…`),
`ferrumc-session/src/lib.rs` (+`mod probe_reexport;`), neu
`ferrumc-session/src/probe_reexport.rs` (`1a288d32…`), neu
`ferrumc-sim/examples/probe_forge_{status,receipt}.rs`. Target per Reflink aus
`target-fresh` geseedet (`tools/probes.sh`, Logs `logs/probes/`). Alle
Sim-Probes laufen je Test auf **Memory und redb**, Counter-Modus, mit echter
Work/Completion.

| Probe | Inhalt | Debug | Release |
|---|---|---|---|
| P1 `probe_refused_rejoin_same_boundary_then_release_leaves_no_residue` | (a) Rejoin mit wirklich veraltetem Read → `StaleSnapshot`, Zustand bis auf den Tick unverändert; Release danach erfolgreich, `player_bytes`/`pending_bytes`/`input_bytes` 0, Generation nicht verbraucht; Restore derselben UUID: neue, größere Admission, `lifecycle_acked`/`saved_after_detach` frisch, altes Handle überall `StaleHandle`, Autosave mit altem Handle verweigert. (b) Autosave vor Rejoin im selben Boundary: Rejoin `Reserved`, Nachweis zurückgesetzt, Release `Pending`, nach Bestätigung Release. (c) Rejoin vor Autosave im selben Boundary: neue Admission, Autosave `StaleHandle`, keine Work; alter Release `StaleHandle`, neuer `Connected`. | ok | ok |
| P2 `probe_older_admission_proof_never_releases_newer_admission` | Nachweis von Admission A überträgt sich nicht auf B: B nach Detach + Ack → `NotSaved`; erst eigener Save gibt frei. Autosave von A in flight → Rejoin `Reserved`; Nachweis bleibt bei A; nach Rejoin startet B mit `None` und nicht geackt. | ok | ok |
| P3 `probe_queued_completion_blocks_and_stale_retry_after_release_is_harmless` | Eingereihte echte Completion des Spielers → `QueuedInput` ohne Mutation; ein Retry auf ein bereits aufgelöstes Pending blockiert nicht, wird nach dem Release typisiert verweigert, erzeugt keine Work und keinen Spielerzustand. | ok | ok |
| P4 `probe_old_cell_with_retained_pin_after_rejoin_acks_and_unblocks_release` (Root-Frage 5) | Alte Zelle mit retained Take-Pin nach echtem Rejoin: `retirement_ticket` None → Release B `PublicationPin` → FIFO-Headack → `CollectorBinding` → Ack der alten Zelle über das alte Handle → Release B ok, Zellen/Pins 0, `owned_stop_ready` true. Kein Deadlock. | ok | ok |
| P5 `probe_release_and_stop_readiness_under_drain` (Root-Frage 6) | Unter Drain: zwei verbundene Spieler → `owned_stop_ready` false; Disconnect, Autosave, Ack zulässig → true; Release unter Drain ok; Stop als einziges Input → `Stopped`, 0 Counterschritte; danach Release `Draining`. | ok | ok |
| P6 Session-Lib Re-Export | `cargo check -p ferrumc-session --lib` (Lib ohne Storage-Dependency; Storage nur Dev-Dependency) mit Funktion über `ferrumc_sim::{OwnershipTransferId, OwnershipReceiptQuery, DetachedOwnedPlayerStatus, ReleasedOwnedPlayer, DetachedReleaseRefusal, PlayerOwnershipError}` | exit 0 | — |
| P7 compile_fail-Grund | Die beiden Doctest-Snippets als externe Beispiele: `DetachedOwnedPlayerStatus` → E0451 (`saved_after_detach`, `blocking`, `dirty` privat), `ReleasedOwnedPlayer` → E0451 (`player`, `freed_player_bytes` privat) | exit 101 wie erwartet | — |

Offenlegung zu den Probe-Läufen: Versuch 1 kompilierte nicht (mein Vergleich
`PlayerLocalGeneration > …`; korrigiert über `.get()`), Logs in
`logs/probes-attempt1/`. Versuch 2 schlug in P1 an meiner falschen Annahme fehl:
Ein inhaltsgleicher Autosave lässt den Durable-Stamp unverändert (No-op-CAS), der
„alte“ Read war daher aktuell und der Rejoin korrekt zugelassen; Logs in
`logs/probes-attempt2/`. P1 hält diese Beobachtung jetzt als eigene Assertion fest
und erzeugt den veralteten Read über eine echte Posenänderung. Kein Kandidatenfehler.
Der zusätzliche `cargo clippy -p ferrumc-sim --tests` in `work-probe` meldet nur
Stilbefunde in meiner Probe-Datei (`probe_review.rs:16,29,51`), nichts im
Kandidaten.

## Teil C — Source-Review

Diff gegen main: 9 geänderte + 4 neue Sim-Dateien (`logs/sim-diff.patch`).
Produktive Änderungen außerhalb der neuen Module sind klein und lokal:
`collector_lifecycle.rs` +1 lesender Helfer, `item_publication.rs` +1 lesender
Helfer, `lib.rs` +Re-Export, `player.rs` +Modul/Export/Fehlervariante,
`pending.rs` +2 private Felder und +1 Accessor, `lifecycle.rs` +Bitsetzung nach
erfolgreicher Ack, `player_ownership.rs` Helfer-Extraktion und 4 Einzeiler.
`counters/tests.rs` hat genau `+mod release;`; bestehende Tests sind unverändert
(361 → 384 Testattribute, 9 → 11 `compile_fail`; +23 Tests +2 Doctests = 394).

### Befunde nach Schwere

**Blockierend / hoch / mittel: keine.**

**N1 (niedrig, Dokumentation, nicht blockierend) — Pending-Scans sind dreifach,
nicht doppelt beschrieben.**
`server/crates/ferrumc-sim/src/shard/player_ownership/release.rs:5-7` („32
pendings (twice)“), Delivery Abschnitt „Release-Gate“ („2×≤32 Pendings“) und
`INVARIANTS.md:532` („32 pendings“). Tatsächlich iteriert
`check_detached_release` die Pending-Map dreimal (`release.rs:177-183` Pins,
`release.rs:189-195` Itemreservierung, `release.rs:196` Pending), dazu je
Queue-Input `Retry`/`Reconcile` ein `BTreeMap::get` (`release.rs:46-49`).
Szenario: 32 Pendings, keines des Spielers → 96 Pending-Besuche statt der
angegebenen 32/64. Beleg: Quelltext. Folge: keine; die Summe (1 Map-Eintrag + 32
Inputs + 3×32 Pendings + ≤256 Gruppen + ≤384 Zellen ≈ 769) liegt weiter unter
der Vertragsgrenze von 833 Eintragsbesuchen (§7.5 A7), alle Scans sind geborgt
und allokationsfrei. Empfehlung: Zahl in Moduldoku/INVARIANTS/Delivery bei der
Integration auf „3×≤32“ korrigieren oder als Hinweis übernehmen.

**H1 (Hinweis) — Matrixzeile Re-Export liegt im Sim-Crate.**
`tests/detached_release_api.rs:9-25` belegt Typidentität, aber im Sim-Crate, das
selbst von Storage abhängt. Die Vertragszeile („Session kompiliert ohne
Storage-Dependency“) ist SIM-03.4b/e2 zugeordnet; Probe P6 belegt sie auf der
Kombination (Session-Lib ohne Storage-Dependency kompiliert gegen
`ferrumc_sim::{OwnershipTransferId, OwnershipReceiptQuery, DetachedOwnedPlayerStatus, ReleasedOwnedPlayer, DetachedReleaseRefusal}`).
Für e2 ein echter Session-Test empfohlen.

**H2 (Hinweis) — Fingerprint der Refusal-Tests.**
`counters/tests/release.rs:151-226` vergleicht Spielerdaten (inkl. beider neuer
Bits), Bytes, Pendings mit Status, Queue, Generation, Lifecycle-Zellen/Pins,
Publicationgruppen, Entityzahl, Tick, Draining/Stopped. Nicht enthalten sind
Entity-Owner-Reservierungen, Counter-Einträge und Detach-Fakten der Zellen. Da
`release_detached_owned_player` genau eine Mutationsstelle nach dem Gate hat
(`release.rs:121-139`) und das Gate nur liest (`&self`, `release.rs:151`), ist
das keine Lücke in der Garantie, nur in der Beobachtung.

**H3 (Hinweis für NET-04.2f) — Chunk-Tickets gehören dem Aufrufer.**
`acquire_owned_chunk`/`release_owned_chunk` (`player_ownership.rs:2076-2111`)
nehmen das Ticket vom Aufrufer; Tickets tragen keine PlayerId
(`TicketReason::Player` ohne Id) und native Admissions nehmen keine Tickets
(`require_player_region`, `player_ownership.rs:913-929`, prüft nur). Der Release
kann daher keine Tickets sehen oder freigeben; der Driver muss für einen
Spieler erworbene Tickets selbst freigeben.

### Prüfpunkte des Auftrags

**saved_after_detach.** Einziger Setzer `record_detached_save`
(`release.rs:142-149`), einziger Aufrufer `publish_confirmed`
(`player_ownership.rs:1697-1700`) nach `publish_player` und nur für
`PendingKind::Autosave`; `publish_confirmed` hat genau einen Aufrufer, den
`Confirmed{current: Ok}`-Zweig von `complete_owned` nach `confirm_current`
(`player_ownership.rs:1637-1655`). `Unresolved`/`ReceiptAbsent` → `Uncertain`,
`Busy` → `Prepared`, `Confirmed{Err}` → `NeedsRefresh`: Pending bleibt, setzt
nichts. Gilt nur, wenn danach `!connected`. „Genau diese Admission“: ein
Pending kann nicht über eine Admissiongrenze leben (Rejoin verlangt kein Pending,
`player_ownership.rs:745-747,762-764`; Release verlangt kein Pending), und
`publish_player` behält die Admission bei Nicht-Admission (`1839-1843`).
Rücksetzen: jedes echte neue Pending (`install_candidate`,
`player_ownership.rs:1312`; die drei `p.pending = Some` in `counters.rs:1169/1286/1330`
sind Schattenplanung), jede Veröffentlichung (`publish_player` startet mit
`None`, `1867`), jedes Deferred-Apply = einzige echte `dirty = true`-Stelle
(`2037-2039`; die übrigen `dirty`-Schreibstellen in `counters.rs` sind
Schattenansichten). Deferred-Werte, die eine Autosave-Bestätigung überdauern,
werden im selben Schritt angewandt (`publish_confirmed` endet mit
`apply_deferred`, `1752`) und löschen den Nachweis sofort wieder. Uncertain
blockiert über `Pending` (Test und Probe).

**Release ohne Caller-Query, 13 Gates gegen A7.** Jede A7-Zeile hat eine
typisierte Refusal: verbunden → `Connected`; Zelle nicht acked →
`LifecycleNotAcknowledged`; Lifecycle nicht initialisiert →
`LifecycleNotInitialized`; kein Nachweis → `NotSaved`; Pending/Bootstrap →
`Pending`; Restore/Rejoin/Bootstrap-Read und jeder Queue-Input → `QueuedInput`
(Match ohne Wildcard, `release.rs:28-56`, neue Input-Varianten erzwingen eine
Entscheidung); Publication-Pin (reserviert und retained) → `PublicationPin`;
Item-/Chunkreservierung → `ItemReservation`; Collector-Bindung →
`CollectorBinding`; Snapshot → `SnapshotOnly`; Foreign/Stale/wiederholt →
`StaleHandle`; gestoppt → `Draining`; zusätzlich dirty/deferred → `Unsaved`.
Alle Gates laufen in `check_detached_release(&self)`; danach genau eine
Mutation: `players.remove` + `player_bytes = remaining` (vorher `checked_sub`),
dazwischen nichts Fehlbares (`release.rs:121-139`). Zellcredits sind seit der
Ack frei, Pending-/Input-/Publication-Bytes sind per Gate null für diesen
Spieler, Spielerzahl ist `players.len()`, Generation wird nicht verbraucht
(Probe P1). Ein späterer Restore derselben UUID bekommt eine neue, strikt
größere Admission aus dem monotonen `state.generation` (`pending.rs:672-677`),
das alte Handle bleibt überall `StaleHandle` (Probe P1). Legacy-`undo_history`
(nach PlayerId) wird nicht berührt; im Counter-Modus ist Legacy-`run_tick`
gesperrt (`shard.rs:751-757`), sie kann für native Spieler nicht entstehen.

**ReleasedOwnedPlayer / DetachedOwnedPlayerStatus / DetachedReleaseRefusal.**
`pub(crate)`-Felder, nur `const`-Getter, kein öffentlicher Konstruktor
(`player/detached.rs`), `Copy`-Daten ohne Handle, Query, Lease oder Owner-Weak.
Probe P7 belegt, dass Fremdkonstruktion an den privaten Feldern scheitert
(E0451). Die Refusal-Enum ist `#[non_exhaustive]`, die neue Variante von
`PlayerOwnershipError` (bereits `#[non_exhaustive]`) ist additiv; kein
Workspace-Verbraucher matcht erschöpfend (Build/Clippy grün).

**lifecycle_acked.** Gesetzt nur nach erfolgreichem
`CollectorLifecycleState::acknowledge` und nur bei Admissiongleichheit
(`lifecycle.rs:131-141`); die Zelle ist owner-gebunden
(`ItemCollectorBinding::eq` vergleicht den Owner-Weak, `item_publication.rs:296-302`).
Zurückgesetzt nur durch eine neue Admission (`player_ownership.rs:1851`).
Rejoin-Probe und Ersteller-Test zeigen: Ack der alten Zelle markiert die neue
Admission nie.

**owned_stop_ready / ownership_quiescent.** Der alte Rumpf ist ohne
Prädikatänderung (gleiche Terme, gleiche Reihenfolge) in
`owned_state_quiescent` verschoben (`player_ownership.rs:702-726`);
`ownership_quiescent` behält `is_none_or` für fehlenden Ownership-Zustand, Stop-
Apply (`578-588`) und `is_tick_quiescent` (`shard.rs:557-562`) rufen ihn
unverändert. `owned_stop_ready` (`release.rs:99-105`) ruft denselben Helfer
nach `ownership_state()` und gibt `Err(Draining)` nur nach `stopped`. Unter
`draining` liefert er `Ok(bool)` (Probe P5). Konsistenz mit §10.4: In Phase A
halten verbundene Spieler ihre Zelle (`cells > 0` → false), erst Detach + Ack
machen ihn wahr; in Phase B entscheidet er den Übergang zu C; nach Stop
typisiert `Draining`. Root-Entscheidung 6 ist damit konsistent.

**Re-Export, Rustdoc, Hygiene.** `pub use ferrumc_storage::{OwnershipReceiptQuery, OwnershipTransferId}`
mit Rustdoc (`lib.rs:145-149`); Sim-rustdoc mit `-Dwarnings` grün. Kein
`unwrap`/`expect`/`panic` in den neuen Produktionsmodulen; das vorhandene
`panic!("bootstrap player disappeared")` (`player_ownership.rs:1309`) ist
unverändert aus main. `forbid(unsafe_code)` in Crate und Workspace.

**R3-H2-Accessor.** `OwnedPlayerInputRefusal::query()` (`pending.rs:341-350`)
liest nur; in der Sim gibt es keinen produktiven Aufrufer.

### Root-Fragen 1–7

1. **Foreign vs. Stale:** Fremder Owner scheitert in `actual_player` an
   `same_owner` vor jeder Mutation (`player_ownership.rs:602-622`); Status und
   Release liefern `StaleHandle`, Fingerprint unverändert (Ersteller-Test), auch
   bei gleicher UUID und im fremden Shard voll releasebarem Spieler. Nichts geht
   still durch.
2. **Refusaltyp:** vollständig und eindeutig je A7-Zeile (siehe oben); Pin vor
   Itemreservierung ist dokumentiert (ein animierter Pickup hat beides). Keine
   Autorität im Typ.
3. **Chunkreservierung:** Außer Entity-Owner-Reservierung (Item-/Pair-Pending)
   gibt es keinen spielerzuordenbaren Chunk-/Ticket-Zustand in der Sim
   (Tickets ohne PlayerId, vom Aufrufer gehalten; H3).
4. **Bootstrap-Pending derselben PlayerId:** unerreichbar. Bootstrap und
   Restore laufen beide über `check_player_admission`
   (`player_ownership.rs:436`, `801`): bestehender Spieler → `AlreadyOwned`,
   fremdes Pending derselben Id → `Reserved` (`762-764`). Das Gate prüft
   trotzdem generisch.
5. **Strengere Collector-Regel:** Kein Deadlock in der Sim. Die alte Zelle ist
   nach Rejoin über das alte Handle erreichbar (`collector_lifecycle_view`,
   `lifecycle.rs:79-95`), ihr Detach-Fakt existiert immer (Rejoin verlangt
   `!connected`, das setzt `detach`), ihre Pins werden per FIFO-Headack frei.
   Probe P4 zeigt den Ablauf mit retained Pin: `PublicationPin` →
   Headack → `CollectorBinding` → Ack der alten Zelle → Release. Abhängigkeit:
   e2 muss D7/D8 auch für `Superseded`-Records ausführen (Vertrag §7.4 sieht das
   vor).
6. **owned_stop_ready false bei verbundenen Spielern:** konsistent (siehe oben).
7. **Keine Allokation:** Delivery und INVARIANTS begründen strukturell; keine
   gemessene Behauptung. Code: nur geborgte Iteratoren, `Copy`-Rückgaben,
   statische Fehlertexte; `Arc::downgrade` in `same_owner` zählt nur hoch.

### Testabdeckung der SIM-03.4b-Matrixzeilen

Alle 19 Tests in `counters/tests/release.rs` laufen `for redb in [false, true]`
über echte `InMemoryStore`/`RedbStore`-Ownershipadmission im Counter-Modus
(`initialize_owned_item_counters`, `run_tick_with_player_ownership_at`), mit
echten `execute(&store)`-Completions, echter ReceiptCollision, echtem Lost-Reply
und echter Reconcile-Klärung; keine Fake-Facts.

| Matrixzeile | Abdeckung | Bewertung |
|---|---|---|
| Release-Refusals (A7) | je Fall eigener Assert mit Fingerprint vor/nach (`assert_refused`, `release.rs:128-135`): Connected, LifecycleNotAcknowledged, LifecycleNotInitialized, NotSaved, Pending, QueuedInput (Disconnect, Autosave, Move, Rejoin, Restore, Bootstrap), PublicationPin (reserviert, retained), ItemReservation, CollectorBinding, Unsaved; Snapshot/Foreign/Stale/wiederholt/Draining mit Fingerprintvergleich | echt; Completion/Retry als Queue-Input ergänzt durch P3 |
| `saved_after_detach` | zwei bestätigte → letzte; ReceiptCollision dann bestätigter Retry; späteres Pending setzt zurück; Lost-Reply → Uncertain blockiert → späterer Commit + Reconcile bestätigt; Confirm-Zeitpunkt verbunden/abgemeldet; Deferred; Rejoin | echt; ältere Admission ergänzt durch P2 |
| `detached_owned_player_status` | verbunden, Stale (Rejoin), Foreign (zweiter Shard, gleiche UUID), Snapshot, NotInitialized; Fingerprint unverändert | echt; „keine Allokation“ nur strukturell (Root-Entscheidung 7) |
| Re-Export (R2-A4) | Typidentität im Sim-Crate | Session-Seite durch P6 auf der Kombination belegt (H1) |
| `owned_stop_ready` vs. Stop | 8 Zustände je Backend, `ready == Stopped`, bei true 0 Counterschritte, sonst `NotQuiescent`; Queue-Input und Snapshot | echt; Drain-Phase ergänzt durch P5 |

N-1/N/N+1 (Gates einzeln, Spielerkappe ohne Verdrängung) und Races (Release vs.
Rejoin in beiden Reihenfolgen) sind getrennte Tests. „Keine Teilmutation“ wird
per Zustandsvergleich geprüft; zur Reichweite des Fingerprints siehe H2.

### Delivery

Zahlen der Delivery stimmen mit den Ersteller-Logs auf d7f16b21 überein (Sim
394/394, Session 159 in 3 Läufen, testkit 100, bench 5, app 375 + 1 ignoriert in
40 Läufen). Auf der Kombination gelten die Zahlen aus Teil A. Nicht-Behauptungen
(keine Aktivierung, Survival, Live, Paper, Performance; Allokation nur
strukturell) sind eingehalten. Forbidden-Claims-Scan mit `markdown_blocks`,
`rustdoc_blocks` und `scan` aus `scripts/check-forbidden-claims.py` über
Delivery, Freigabe, Sim-INVARIANTS und 7 geänderte/neue Sim-Quellen
(`logs/claims-scan.json`): 0 Treffer in Delivery, Freigabe, Rustdoc und dem neuen
INVARIANTS-Abschnitt; 2 Treffer der Regel „unqualified“ (Kategorie
Kompatibilitätsaussage ohne Umfang) in `INVARIANTS.md:87` und `:321`, beide in
den Zeilen 1–500, die bytegleich zu main sind (vorbestehend, nicht von diesem
Kandidaten). Dieses Review-Dokument selbst: 0 Treffer. Einzige Ungenauigkeit: N1.

## Offenlegung zum Ablauf

Beim Beenden meines eigenen Log-Monitors habe ich `pkill -x tail` ausgeführt. Das beendet alle `tail`-Prozesse dieses Benutzers und kann `tail -F`-Monitore anderer Sitzungen beendet haben. Dateien, Builds oder Tests anderer Agents sind davon nicht betroffen, deren Monitore müssen aber ggf. neu gestartet werden. Beim Neustart der Probes habe ich nur eigene Prozesse beendet (PID-genau), insbesondere nicht den parallel laufenden fremden `ferrumc-world`-Test.

## Artefakte

`.codex-tmp/sim03_4b-runtime-root-review/`: `final-assessment.json`, `tools/` (checks.sh, probes.sh, verify.py, claims_scan.py, assess.py), `logs/checks/`, `logs/probes/`, `logs/probes-attempt1/`, `logs/probes-attempt2/`, `logs/sim-diff.patch`, `logs/combined-vs-main.json`, `logs/claims-scan.json`, `logs/verify-pre.json`, `logs/verify-final.json`, `work/`, `work-probe/`, `target-fresh/`, `target-probe/`.

## Root-Integrationsvermerk

Root, 11.10.2026: SIM-03.4b ist mit den exakt geprüften 60 Sim-Dateien
(Manifest `03747d58`) integriert. Der niedrige Dokumentationsbefund N1 (drei
statt ein bis zwei Durchläufe der Pending-Map in `release.rs:5-7`,
`INVARIANTS.md:532` und der Delivery; die Grenze von 833 Besuchen bleibt mit
etwa 769 eingehalten) wird im nächsten Ticket korrigiert, das `ferrumc-sim`
berührt, damit nach dem Review kein Quellbyte geändert wird. Die Hinweise H1
(echter Session-Test für den Re-Export) und H3 (Chunk-Ticket-Freigabe durch den
Driver) gehen an NET-04.2e2 bzw. NET-04.2f. Dieser Bericht hatte beim Abschluss
des Reviews SHA-256
`f355e6de45593dd29aca5e349b2ef66b015fcea2e0971475a676a312711ff82c`.
