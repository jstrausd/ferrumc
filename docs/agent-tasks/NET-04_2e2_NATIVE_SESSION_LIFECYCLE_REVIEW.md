# NET-04.2e2 — Unabhängiger Runtime-Review: nativer Session-Lifecycle

**Verdikt: FINDINGS.** Alle frischen Checks sind grün. Jede Zahl und jeder Hash
der Delivery ist reproduziert. Zwei Befunde richten sich gegen e2-Auflagen:
**F1 (mittel)** und **F2 (niedrig)**. Dazu kommt eine Vertragsunstimmigkeit
**V1**, über die Root entscheiden muss, sowie die Hinweise H1–H6.

Nicht gefunden wurden: Autoritätslecks, Retargeting, unbegrenzte Strukturen,
Locks über `.await`, `unwrap`/`expect` in Produktion, verlorene oder doppelte
Leave-Saves bzw. Releases und Deadlocks zwischen L1/L2, Retention und Release.

12.10.2026, Reviewer-Subagent, nicht Ersteller. Maschinenlesbar:
`.codex-tmp/net04_2e2-runtime-root-review/final-assessment.json`.

## 1. Prüfbasis und Integrität

| Gegenstand | Wert |
|---|---|
| Prüfbasis | `combined/` = `main` `e18efd22` + 37 Kandidatendateien + Delivery. 1334 Dateien, `combined.freeze.json` `606b3112…`, vorher und nachher 1334/1334 |
| Kandidat | Manifest `631cc69e…` (14 geändert, 8 neu, 15 unverändert, 0 entfernt). Aggregat `4a5b3653…` mit der Formel aus `manifest.py` nachgerechnet |
| Kandidatendateien | Vor und nach den Checks 37/37 = Manifest, und zwar in `combined/`, in der Arbeitskopie, im Arbeitsbaum und in `candidate-src/` |
| Evidenz | `evidence.freeze.json` `a4ce2dcb…`: 27/27 Dateien hash-gleich, Aggregat `33cc5eb3…` nachgerechnet. `session.diff` `10c76e1f…`, `e1-tests.diff` `f082b41c…` |
| Delivery | `a04af896…` |
| Arbeitskopie | `work/` (privat), frisches `target/`, `CARGO_INCREMENTAL=0`, kein Wrapper/Cache, cargo/rustc 1.99.0 |
| Probe-Kopie | `probe/` = `combined/` + genau `server/crates/ferrumc-session/tests/review_probes.rs` (geprüft), eigenes `target-probe/` |

## 2. Ausführung (frisch, `--jobs 2 --locked`)

Skript `scripts/run_checks.sh`, Exit-Codes in `logs/checks.txt`, Hashes in
`evidence/review-files.sha256`.

| Check | Exit | Ergebnis |
|---|---|---|
| `cargo test -p ferrumc-session` Debug | 0 | **241** bestanden: Unit 154, audience 6, borrow 3, endpoint 2, join 11, lane 13, lifecycle 19, directory 15, source_scan 2, Doctests 16 |
| dito `--release` | 0 | **241** bestanden |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | grün |
| `cargo fmt --all --check` | 0 | grün (leeres Log) |
| `RUSTDOCFLAGS=-Dwarnings cargo doc -p ferrumc-session --no-deps` | 0 | grün (nur Session, keine Workspace-Aussage) |
| `cargo build --workspace --all-targets` | 0 | grün |
| `cargo test -p ferrumc-app --no-fail-fast` | 0 | **375** bestanden, 0 fehlgeschlagen, 1 ignoriert (Basis-`#[ignore]`, Java/Maven), 40 Läufe. `multiplayer_visibility` im ersten Lauf 2/2, kein Einzellauf nötig |
| `cargo test -p ferrumc-testkit --no-fail-fast` | 0 | **100** bestanden, 8 Läufe |
| compile_fail eigenständig (gegen die rlibs des frischen Targets, stderr protokolliert) | 0 | Z. 35 E0597, Z. 54 E0382, Z. 64 E0521, Z. 145 „cannot construct … due to private fields“, Z. 154 E0521. Alle fünf scheitern aus dem beabsichtigten Grund |

**Testnebenprodukte, separat ausgewiesen:** `plugin_testhost` legte in der
Arbeitskopie
`.codex-tmp/p57-plugin-testhost-2374986-artifacts/{libplugin_testhost_fixture.so,librenamed_fixture.so}`
ab (je 8 222 048 Byte, SHA-256 `9c519b4a…`), dazu das leere Verzeichnis
`.codex-tmp/p52-abi-invalid-input/2373889`. Die `.so`-Dateien liegen jetzt unter
`test-byproducts/` (mit `SHA256SUMS` und `SIZES`), die leeren Verzeichnisse sind
entfernt. Danach ist die Arbeitskopie wieder 1334/1334 = Freeze.

## 3. Auflagen

| Auflage | Bewertung | Beleg |
|---|---|---|
| **A1** Reservation alloziert den übernehmbaren Eintrag, Adoption kapazitätsunfehlbar | **erfüllt** | `NativeTable` (`router/native.rs:96-195`): Nur `reserve_slot` wächst, und zwar fallibel per `try_reserve(1)`, vor dem Verbrennen von Inkarnation und Wire-ID (`:713-726`). Admission (`:890-893`), Retirement (`router/native/retention.rs:738-757`) und Adoption (`:641-650`) ersetzen den Zustand im selben Slot. Probe P3: Lane voll und Limit 2/2 erschöpft, die dritte Reservation endet mit `SessionCapacity`, die Adoption gelingt |
| **A2** Refused-Revalidierung endet definiert, `retry` erreicht 0 | **teilweise** | Normalfall erfüllt: Der Disconnect aus dem gespeicherten Handle geht über die aktuelle Lane, bei `StaleHandle` folgt Quarantäne, kein Resend, kein Rebind. **F2**: Hat die Refused-Session einen zurückgehaltenen Disconnect und folgt ein zweites Replacement, endet sie nie |
| **A3** Pose/Mode aus aktueller Sim-View, Follow-up-Sync | **teilweise, korrekt offengelegt** | `native_pose` (`router/native/audience.rs:112-165`) nimmt Position, Look und Mode nur aus einer View, die zum gespeicherten Handle und zur vollen Generation passt. Follow-up-Sync über `sync_target`, Korrekturen nutzen `session.pose`. **Held-Slot offen** (Delivery §8.1): `ConnectedOwnedPlayerView` hat keinen Selected-Getter (`collector_lifecycle.rs`), `selected()` gibt es nur auf dem dauerhaften `NativePlayerSnapshot` (`player.rs:280/323`). Die Begründung stimmt |
| **A4** Control nach DetachFact nicht live | **erfüllt** | `native_lane_input_live` (`router/native.rs:1049-1065`) verlangt `!session.detached`. Nach D5 gibt es kein Key-Mapping mehr |
| **A5** `disconnect_native` → Token | **erfüllt** | `router/native/retention.rs:56-77`: idempotent mit gleichem Token. `LineageExhausted` kommt vor jeder Wirkung |
| **A6** Session-Teil der Teleport-IDs | **erfüllt** | Der einzige native `SynchronizePlayerPosition`-Producer ist `send_correction` (`router/native.rs:1415-1446`), und er schreibt nur in den Session-Outbound. In der App läuft jede Nachricht dieses Outbounds durch `mirror_outbound` mit `pending_teleport.issue` (`app/src/connection/play.rs:1119/1132/1145-1158/1179`). Ein Direktversand mit ID 0 ist im heutigen Code nicht möglich. Für f bleibt die Auflage aus Delivery §8.2 |
| **A7** Decision-Log | **erfüllt** | Delivery §5 Punkt 1 nennt alle vier e1-Abweichungen |
| **H1** (SIM-03.4b) | **erfüllt** | `session_api_works_against_the_sim_reexports_without_a_storage_type` (M+R) läuft echt. Storage und World sind nur Dev-Dependencies (`Cargo.toml`), `src/` nennt Storage nur in `native_tests.rs` (`cfg(test)`) |
| Stale/Foreign → Quarantäne | **erfüllt** | `native/retention.rs:312-317` |
| Leave-Save-Zustandsmaschine | **erfüllt, mit F1 und V1** | 8 Versuche (Quarantäne bei `attempts > 8`, P4: nach 800 Boundaries mit 9), Quarantäne typisiert. Pickup in Reserved wartet ohne Zählung (P1, R2-A1-Test). R2-A1: Due + fremdes Pending → Waiting, Rücknahme bei `Reserved` (`:444-455`, `:483-491`) |
| Release nur über Status und `release_detached_owned_player(original)` | **erfüllt** | Typisierte Vorprüfung (`router/native/retention.rs:535-559`). Der Closure-Receipt muss den Spieler nennen. `ReleasedOwnedPlayer` hat keinen öffentlichen Konstruktor. Das Sim-Gate prüft `saved_after_detach` erneut |
| Driver hält nur ein opakes Token | **erfüllt** | `NativeRetentionToken` hat `pub(crate)`-Felder und keinen Konstruktor |
| .2c-Ergänzungen G3, R1, R2, R4 | **erfüllt** | `adopt/retire_unbound` (§8), `retire_detached_native_sessions`, `acknowledge_retired_native_records` (Probe-Form, D7/D8 als eine Closure), R4 unverändert aus e1 |
| Audience/Reconcile mit Grenzen vor dem ersten Send | **erfüllt** | `plan_reconcile` (`router/native/audience.rs:209-345`) prüft Work, Scratch, Baselines pro Viewer und gesamt sowie die Joiner-Outbound-Kapazität, bevor ein Delta angewandt wird. Der Plan wird fallibel vorreserviert und kann nicht reallozieren (≤ 2(P−1) Deltas). Viewer-seitige Mandatory-Fehler setzen den Viewer gemäß Slow-Client-Policy auf Leaving |

## 4. Source-Review

- **Autorität.** Tokens, Verdicts, Probes, Audience, Proof und
  `AdoptionInvariant` haben nur `pub(crate)`-Felder; Probes und Tickets können
  den Closure-Aufruf nicht verlassen (compile_fail).
  - Probe-Sequenzen sind checked; ein fremdes Verdict beweist nichts
    (`native_tests.rs`).
  - Handles verlassen die Session nur als Closure-Argument.
  - Ein im Verdict gehaltenes, nicht quittiertes `CollectorLifecycleTicket` ist
    reines Datum (`collector_lifecycle.rs:166-225`). Es fällt folgenlos weg und
    wird beim nächsten Aufruf neu erzeugt.
- **Retargeting.** Keine Stelle zielt auf eine neue Session um:
  - `native_session` vergleicht den vollen Key (Router-Weak, Spieler,
    Inkarnation). Ein wiederverwendeter Slot mit neuer Inkarnation liefert
    `StaleSession`.
  - D3, Pose, Intents und `collector_delivery` vergleichen gegen das
    gespeicherte Handle (Owner-Weak, Spieler, Admission) und die volle
    Generation.
  - P2b: Nach einem Sim-Rejoin, aber vor der Übergabe an die Session, liefert
    der alte Release `NativeReleaseStale`. Der Record bleibt erhalten; der
    Admit setzt ihn auf Superseded und entfernt ihn.
- **Grenzen.** Die Tabellenlänge ist durch das Sessions-Limit begrenzt, weil
  freie Slots zuerst wiederverwendet werden. Alle Worklists sind per
  `try_reserve_exact(slots.len())` begrenzt. Kein Channel ohne Kapazität.
- **Locks, Panics, async.** Im geänderten Produktionscode gibt es kein
  `.await`, kein `Mutex`/`RwLock`, kein `unwrap`/`expect`/`panic!` und keine
  Indizierung. Die Zähler `baselines` sind konsistent gepflegt: Jedes Inkrement
  in `apply_enter` hat genau ein Dekrement in `begin_native_leave` oder
  `retire_session`.
- **Leave-Save und Release ohne Verlust oder Duplikation.**
  - Ein zweiter Service im selben `applied` bleibt `Waiting`.
  - Submit-Rückgaben laufen über `last_submit` und `unsent`.
  - Ein Release gibt den Slot nur bei passendem Receipt frei; ein zweiter
    Release liefert `StaleRetention` (P1, P2, Matrixtests).
- **Deadlocks.** Kein Zyklus: D9 braucht D5 und `SaveConfirmed`, Release braucht
  Ack und Confirmed, und keine dieser Bedingungen hängt am Release. Der Rejoin
  wird bis D5 von der Session verweigert (`DuplicatePlayer`, P1) und von der Sim
  so lange, wie das Leave-Save-Pending läuft (`Reserved`, P1).
- **Record, der nie endet.** Zwei Fälle, beide unten als Befunde:
  - F2: Refused-Session über ein zweites Replacement.
  - V1: Ein durch Rückstau quarantinierter Disconnect hält Record und L1 bis
    zum Stop.

## 5. Eigene Probes (Memory und redb, echte Work und Completion)

`probe/server/crates/ferrumc-session/tests/review_probes.rs` (`4f977bbf…`),
Log `logs/probes-debug.log`: 8/8 bestanden. Jede Assertion kodiert den
beobachteten Wert.

| Probe | Beobachtung (beide Backends gleich) |
|---|---|
| P1 Rejoin während laufendem Leave-Save | Vor D5 `DuplicatePlayer`. Nach D5 lehnt die Sim den Rejoin ab („ownership is reserved“). Der Driver cancelt; der Leave-Save bestätigt mit 1 Versuch, der Release läuft genau einmal |
| P2 Release gegen Rejoin, beide Reihenfolgen | (a) Release zuerst: Der Rejoin mit dem alten Read ist „foreign or stale“, ein frischer Restore bindet normal. (b) Sim-Rejoin zuerst: Der Release liefert `NativeReleaseStale`, der Record bleibt (`Retired{acked}`), die Sim ist unverändert. Der Admit entfernt den Record, ein erneuter Release liefert `StaleRetention` |
| P3 G3-Adoption bei voller Lane und erschöpftem Limit | Die dritte Reservation endet mit `SessionCapacity{2}`. Die Adoption gelingt, der Disconnect ist ein gated Submit unabhängig von der Lane. Die Kette läuft bis zum Release; der freie Slot nimmt die nächste Reservation an |
| P4 eigenes `Prepared` (echte `HistoryCapacity`) → 8 Versuche → Stop | Quarantäne nach 800 Boundaries (`AttemptsExhausted`, 9). Der Final-Save wertet das eigene alte Pending als fremdes Blocking (Grace 1200) und quarantiniert erneut nach **1400** Boundaries (H1). Der Release liefert `RetentionNotReleasable("leave-save not confirmed")` |
| P4b Final-Save eines `DisconnectNotApplied`-Records | `renew` → `Ok(true)`, danach liefert der Service `Err(NativeStatusInvariant)` → **F1** |
| P5 Refused + zurückgehaltener Disconnect + zweites Replacement | `retry_native_disconnects()` liefert dauerhaft 1. In 40 von 40 Services `Waiting`, `AwaitDetach`, 0 Versuche. Der Sim-Spieler bleibt verbunden, Control bleibt live → **F2** |
| P6 Poisoned blockiert Unregister | `NativeEndpointInUse{sessions:1}`. Das schließt die Belegslücke aus Delivery §8.7 |
| P7 Lane-Rückstau (48 Data-Items, Fenster 4 pro Iteration) | Quarantäne `DisconnectNotApplied` in Iteration 10, obwohl der Disconnect noch in der Lane liegt. Der echte Detach folgt in Iteration 13. Der Record bleibt quarantiniert und `Detached`, kein Leave-Save bis zum Stop → **V1** |

## 6. Befunde nach Schwere

### F1 (mittel) — Final-Save eines `DisconnectNotApplied`-Records meldet einen Verdrahtungsbruch

- **Ort:** `native/retention.rs:611-619` (`renew_final_save` erneuert jeden
  quarantinierten Record) mit `:351-362`: `Ineligible("connected player")`
  ergibt außerhalb von `AwaitDetach` `Decision::StatusInvariant`. Die Abbildung
  liegt in `router/native/retention.rs:304-306` und `:342-353`.
- **Szenario:**
  1. Ein Disconnect wird nach 8 Resends noch nicht angewandt, zum Beispiel
     wegen V1. Der Record geht in `Quarantined{DisconnectNotApplied}`.
  2. Stop-Phase A erneuert „je quarantiniertem Record genau einen Versuch“
     (§10.4). Der Record wird `Due`.
  3. Der nächste Service sieht den noch verbundenen Spieler und liefert
     `CollectorSessionError::NativeStatusInvariant`.
  4. Der Vertrag (§7.3 Punkt 1) behandelt das als Verdrahtungsbruch, der Driver
     macht Fail-Stop. Die Leave-Saves aller übrigen Records der Phase A laufen
     dann nicht mehr.
- **Beleg:** P4b auf Memory und redb.
- **Behebung im Session-Crate:**
  - Entweder erneuert `renew` nur Records, deren Spieler detached ist bzw.
    nicht `DisconnectNotApplied` sind;
  - oder `Ineligible` führt einen erneuerten Record zurück nach `AwaitDetach`
    statt `StatusInvariant`.
  - Dazu gehört ein Regressionstest.

### F2 (niedrig) — Refused-Session mit zurückgehaltenem Disconnect über ein zweites Replacement

- **Ort:**
  - `router/native.rs:553-565`: Ein Replacement stellt nur `Bound` auf
    `Suspended`; `Refused` behält die Lane-Identität der Verweigerung.
  - `:1123-1126`: Die Revalidierung nimmt nur `Suspended` in ihre Worklist.
  - `:1390-1393`: `send_lane` verlangt die eigene Lane-Identität und liefert
    sonst `Superseded`, was als `Retained` gezählt wird.
  - `:937-949`: `retry_native_disconnects` zählt diese Session.
  - `native/retention.rs:366`: `Lane{sent:false}` ergibt `Waiting` ohne Zählung.
- **Szenario:**
  1. Eine Revalidierung wird verweigert (falsche View).
  2. Der Disconnect findet die neue Lane voll vor und wird zurückgehalten.
  3. Vor dem nächsten Retry wird die Registrierung erneut ersetzt.
  4. Danach scheitert jeder Flush. `retry_native_disconnects()` erreicht nie 0,
     der Record zählt nie und quarantiniert nie, der Sim-Spieler bleibt
     verbunden.
- **Abweichung:** Das widerspricht A2 („definierte Endbehandlung, retry erreicht
  0“) und der INVARIANTS-Aussage dazu.
- **Erreichbarkeit:** Sie ist gering. Eine nicht passende View setzt nach
  meiner Analyse einen Driverfehler voraus (vgl. e1-H11). Hinzukommen müssen
  eine volle Lane und ein zweites Replacement.
- **Beleg:** P5 auf Memory und redb.
- **Behebung:** Das Replacement führt die Lane-Identität von Refused-Sessions
  mit, oder `flush_disconnect` nutzt für `Refused` die aktuelle Lane der eigenen
  Registrierung (kein Rebind, nur der Disconnect des gespeicherten Handles).

### V1 (Vertrag, Root-Entscheidung) — Resend-Zählung ignoriert das Lane-Fenster

- **Ort:** §7.3 Punkt 1 zählt Resends ab `applied > sent_at`. Nach §4.4/§6.2
  nimmt der Driver aber höchstens `16 − G` Lane-Items pro Iteration. e2 setzt
  die Regel wörtlich um (`native/retention.rs:385-391`).
- **Folge:** Ein Data-Rückstau, etwa Hotbar-Spam eines anderen Clients (nicht
  koalesziert, bis Lane-Kapazität minus Reserve), quarantiniert den Leave eines
  unbeteiligten Spielers als `DisconnectNotApplied`. Damit ist sein L1 bis zum
  Neustart gesperrt, und der Leave-Save unterbleibt bis zur Stop-Phase A,
  obwohl der Disconnect später echt angewandt wird.
- **Beleg:** P7.
- **Vorschlag:** f/Root legen fest, dass Resends erst nach Leerung des
  Lane-Rückstands der Session zählen, mit einem Driver-Signal oder einem
  „gesendet und entnommen“-Nachweis. Alternativ darf eine spätere echte
  Detach-Bestätigung die `DisconnectNotApplied`-Quarantäne zurück auf `Due`
  setzen. Das gehört als Auflage nach NET-04.2f bzw. als Vertragsnachtrag; die
  zweite Variante wäre in e2 selbst umsetzbar.

### Hinweise

1. **H1 (P4):** Nach `renew` wertet `due()` das eigene alte, noch bestehende
   Pending als fremdes Blocking (`native/retention.rs:611-619`, `:444-455`,
   `:553-557`). Die Grace von 1200 plus 2 Intervalle ergibt 1400 Boundaries.
   Das ist länger als Phase A (1200 Iterationen), sodass Phase A mit
   `phase_a_incomplete` endet. Ergebnis ist vertragsgemäß `Unresolved`, ohne
   Verlust.
2. **H2:** Ändert sich in `Waiting` die Phase des Blockings von
   Prepared/InFlight zu Uncertain, wird ab `since` gezählt (`:547-557`). Das
   kann mehrere Versuche auf einmal und eine sofortige Quarantäne ergeben. Der
   Vertragstext ist hier mehrdeutig.
3. **H3:** Poisoned-Einträge speichern keine Registrierung und blockieren
   deshalb das Unregister **jeder** nativen Registrierung (`router/native.rs:629`).
   Das ist konservativ und unkritisch, weil Poisoned zum Fail-Stop führt.
4. **H4:** `Confirmed` bleibt in der Session stehen, auch wenn die Sim
   `saved_after_detach` später zurücksetzt (`native/retention.rs:323-329`). Das
   Sim-Release-Gate bleibt maßgeblich (typisierte Verweigerung, kein Verlust).
5. **H5:** `viewed.insert` (BTreeMap) und `equipment.clone()` allozieren
   infallibel (`router/native/audience.rs:360-386`). Mengen und Größen sind
   begrenzt; A1 betrifft nur die Tabelle.
6. **H6:** compile_fail Z. 145 trägt keinen Fehlercode. Er scheitert am
   Privacy-Fehler und damit aus dem richtigen Grund.

## 7. Offengelegte Punkte

| Punkt | Bewertung |
|---|---|
| A3 Held-Slot offen | **Zulässige Auflage** für Sim-Ticket und f. Korrekt offengelegt, Begründung im Code bestätigt. Voraussetzung für die Live-Aktivierung |
| R3-A5: dauerhaft unsicherer Pickup → `CounterRecoveryRequired`; Quarantäne nur bei `defer > 900` erreichbar | **Zulässig, kein Befund gegen e2.** Die Session zählt vertragsgemäß nur `applied` (§7.3), und der Test zeigt beide Seiten. Auflage an das R3-A5-Sim-Ticket bzw. DUR-05a: Leave-Save- und L1-Ausgang im Recovery/Degraded-Pfad festlegen |
| Busy auf Memory nicht deterministisch, Prepared über `HistoryCapacity` | **Zulässig.** Die Session behandelt `Prepared` unabhängig von der Ursache gleich (P4 bestätigt). Busy → Retry ist N3 im Driver |
| Zu weit gefasster Testname (Poisoned) | **Zulässiger Hinweis.** Das Verhalten ist jetzt durch P6 belegt; eine Umbenennung kann bei der F1/F2-Runde mitlaufen |

## 8. e1-Testanpassungen (`e1-tests.diff`)

Keine Assertion ist abgeschwächt.

- `disconnect(...)` entpackt das `Result` (Fehler → Panic) und liest die
  Lane-Disposition aus dem Status. Die Idempotenzprüfung vergleicht zusätzlich
  das Token.
- In `native_endpoint.rs` wechselt E von `Retained` auf `Sent`, wie von Root 2(d)
  verlangt. Neu hinzu kommen `retry == 0`, die Lane-Reihenfolge C → E → A, der
  echte Detach von E und die Detach-Prüfung `[1,2,3,4]` statt `[1,2,3]`.
- In `native_join.rs` ersetzt die exakte Peer-Burst-Prüfung (2 mandatory
  Pakete, danach nichts) das frühere „kein Burst“.
- `native_tests.rs` greift jetzt auf die Tabelle zu statt auf die Maps.
- `fresh_id` nutzt einen getrennten ID-Raum.

## 9. Delivery

- **Zahlen:** Session 241 (Basis 196 Tests + 10 Doctests = 206, neu 29 + 6 = 35),
  App 375 + 1 ignoriert in 40 Läufen, Testkit 100 in 8 Läufen. Alles
  reproduziert.
- **Hashes:** Manifest, Diffs, Evidence-Freeze, alle 27 Evidenzdateien und
  beide Aggregate reproduziert.
- **Forbidden-Claims-Scan** (`scripts/forbidden_claims_scan.py`, Funktionen
  `markdown_blocks`, `rustdoc_blocks`, `scan` aus
  `scripts/check-forbidden-claims.py`):
  - Delivery, INVARIANTS, README: **0 Treffer**.
  - Zusätzlich gescannte Session-Rustdoc: 3 Treffer
    der Scannerregel für unqualifizierte Kompatibilitätsaussagen in
    `router.rs:83/195/512`. Das ist unveränderter
    Basistext und liegt außerhalb des Index-Check-Bereichs (README, `docs/`,
    Plugin-Quellen).
- **Nicht-Aussagen** eingehalten: keine Aktivierungs-, Survival-, Performance-,
  Paper- oder Live-Client-Aussage.
- **Ungenauigkeiten:** INVARIANTS (A2-Absatz) behauptet ohne Einschränkung, dass
  `retry_native_disconnects` 0 erreicht und Refused-Disconnects angewandt oder
  quarantiniert enden; F2 ist ein Gegenbeispiel. Delivery §3 A2 („retry = 0“)
  gilt entsprechend nur ohne zweites Replacement.

## 10. Empfehlung an Root

Statt CLEAN_RUNTIME empfehle ich eine kurze e2-Fixrunde, nur
`ferrumc-session`:

- F1 und F2 beheben, mit Regressionstests nach P4b und P5 auf Memory und redb;
- INVARIANTS und Delivery für A2 nachziehen;
- optional den Testnamen korrigieren und P6 übernehmen.

Root entscheidet V1, entweder als Vertragsnachtrag oder als NET-04.2f-Auflage,
gegebenenfalls mit der in e2 umsetzbaren Rückkehr von `DisconnectNotApplied` zu
`Due` nach echtem Detach.

Die übrigen Auflagen sind erfüllt bzw. als zulässige Folgeauflagen korrekt
ausgewiesen. Nach der Fixrunde genügt ein gezielter Nachreview: F1, F2,
Regressionstests und Session-Checks.

## Root-Integrationsvermerk

Root hat bei der Integration am 12.10.2026 nur die Fundstellenzeile im
Abschnitt zum Forbidden-Claims-Scan umformuliert: Der zitierte englische
Regelname wurde beschrieben statt zitiert, weil der CI-Scanner das Zitat sonst
als Aussage wertet. Der vom Reviewer gelieferte Bericht hatte SHA-256
`c8ba22766080a61aa47a5ce7fe9e6d1f5c7203f4adaf0c1371246bfe33de1618`.
