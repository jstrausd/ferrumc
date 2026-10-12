# NET-04.2e2 — Unabhängiger Runtime-Review Runde 2: nativer Session-Lifecycle

**Verdikt: CLEAN_RUNTIME.** F1, F2 und V1 sind **behoben**. Alle frischen
Checks sind grün, und jede Zahl und jeder Hash der Delivery §11 ist
reproduziert. Die Original-Probes P4b, P5 und P7 kippen genau an den genannten
Assertions um (Z. 431, 559, 681). Meine angepassten Kopien und vier neue
Kreuzungsprobes laufen auf Memory **und** redb, jeweils in Debug und Release,
grün. Die Mutationskontrolle zeigt, dass sie jeden der drei Fixes einzeln
unterscheiden. Es gibt keinen Befund gegen eine e2-Auflage und keine
Regression. Offen bleiben vier Hinweise (R2-H1 bis R2-H4) und die korrekt
eingeordnete Driverpflicht für NET-04.2f (§6).

12.10.2026, Reviewer-Subagent Runde 2. Ich habe weder den Kandidaten noch den
Review von Runde 1 erstellt. Maschinenlesbar:
`.codex-tmp/net04_2e2-runtime-root-review-r2/final-assessment.json`.

## 1. Prüfbasis und Integrität

| Gegenstand | Wert |
|---|---|
| Prüfbasis | `combined/` = `git archive e18efd22` + 37 Session-Dateien + Delivery. 1334 Dateien, `combined.freeze.json` `3beaa1ea…`. Vor den Checks, nach den Checks und nach dem Scan jeweils 1334/1334, 0 geändert, 0 zusätzlich |
| Kandidat | Manifest `f180e54f…`, Aggregat `ad02a694…` mit der Formel aus `round2-manifest.py` nachgerechnet. Gegen die Basis 15 geändert, 8 neu, 14 unverändert; gegen Runde 1 8 geändert, 0 neu, 0 entfernt |
| Kandidatendateien | Nach den Checks 37/37 = Manifest in `combined/`, in der Arbeitskopie, im Arbeitsbaum und in `round2-candidate-src/`. `candidate-src/` (Runde 1) 37/37 = `631cc69e…` |
| Evidenz | `round2-evidence.freeze.json` `d1027049…`: 29/29 Dateien hash-gleich, Aggregat `6feea003…` nachgerechnet. Runde 1 `evidence.freeze.json` `a4ce2dcb…`: weiterhin 27/27 |
| Delivery | `31927f04…`, Vertrag `e0530e0a…`, Review Runde 1 `c8ba2276…`, Assessment Runde 1 `95d22835…` |
| Arbeitskopie | `work/` (privat, aus `combined/`), frisches `target/`, `CARGO_INCREMENTAL=0`, kein Wrapper, cargo/rustc 1.99.0, `--jobs 2 --locked` |
| Probe-Kopie | `probe/` = `combined/` + genau `tests/review_probes.rs` (unverändert, `4f977bbf…`) + `tests/review_probes_r2.rs` (eigene, `589f0b2c…`); geprüft, eigenes `target-probe/` |
| Mutationskopie | `mutation/` = `probe/`, eigenes `target-mut/`; jede Mutation wurde danach aus `combined/` zurückgesetzt und per `cmp` geprüft |

## 2. Ausführung (frisch)

Skript `scripts/run_checks.sh`, Exit-Codes in `logs/checks.txt`. Die Last lag
zwischen 0,8 und 13 (andere Agents).

| Check | Exit | Ergebnis |
|---|---|---|
| `cargo test -p ferrumc-session` Debug | 0 | **244** bestanden: Unit 154, audience 6, borrow 3, endpoint 2, join 11, lane 13, lifecycle 22, directory 15, source_scan 2, Doctests 16 |
| dito `--release` | 0 | **244** bestanden |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | grün |
| `cargo fmt --all --check` | 0 | grün (leeres Log) |
| `RUSTDOCFLAGS=-Dwarnings cargo doc -p ferrumc-session --no-deps` | 0 | grün (nur Session, keine Workspace-Aussage) |
| `cargo build --workspace --all-targets` | 0 | grün |
| `cargo test -p ferrumc-app --no-fail-fast` | 0 | **375** bestanden, 0 fehlgeschlagen, 1 ignoriert (Basis-`#[ignore]`), 40 Läufe. `multiplayer_visibility` im ersten Lauf 2/2 bei Last ~12, kein Einzellauf nötig |
| `cargo test -p ferrumc-testkit --no-fail-fast` | 0 | **100** bestanden, 8 Läufe |
| compile_fail eigenständig (gegen die rlibs des frischen Targets) | 0 | Z. 35 E0597, Z. 54 E0382, Z. 64 E0521, Z. 145 „cannot construct … due to private fields“, Z. 154 E0521, wie in Runde 1 |

**Testnebenprodukte:** `plugin_testhost` legte in `work/`
`.codex-tmp/p57-plugin-testhost-2512240-artifacts/{libplugin_testhost_fixture.so,librenamed_fixture.so}`
ab (je 8 222 560 Byte, SHA-256 `f42d775d…`), dazu das leere Verzeichnis
`.codex-tmp/p52-abi-invalid-input/2510720`. Die `.so`-Dateien liegen jetzt unter
`test-byproducts/` (mit `SHA256SUMS` und `SIZES`), die leeren Verzeichnisse sind
entfernt. Danach ist `work/` wieder 1334/1334 = Freeze.

## 3. Probes (Ausführung)

### 3.1 Original P1–P8 unverändert gegen Runde 2

Log `logs/probes-review_probes-debug.log` (dazu Release). 5 bestanden, 3
erwartungsgemäß umgekehrt, genau an den vom Ersteller genannten Stellen:

| Probe | Ergebnis |
|---|---|
| P1, P2, P3, P6 | bestanden; Werte gleich wie in Runde 1 (M+R) |
| P4 | bestanden; weiterhin 800/1400 Boundaries, 9 Versuche, Release „leave-save not confirmed“ (H1 unverändert) |
| P4b | scheitert in **Z. 431**: `left: None`, `right: Some(DisconnectNotApplied)`. Die Lane wird nie geleert, deshalb gibt es keine Quarantäne (V1) |
| P5 | scheitert in **Z. 559**: `left: 0`, `right: 1`. Beobachtet: `retry_series=[0,0,0,0,0]`, 39/40 `Waiting`, `Pending`, 1 Versuch, nicht verbunden, Control nicht live (F2) |
| P7 | scheitert in **Z. 681:49** (`unwrap` auf `None`). Keine Quarantäne, echter Detach in Iteration 13, Endzustand `Pending`/`Detached` (V1) |

Alle drei brechen im Memory-Lauf ab; ihr redb-Zweig kommt nicht an die Reihe.

### 3.2 Eigene Kopien auf das neue Soll (Memory **und** redb)

`tests/review_probes_r2.rs`, Debug 7/7 und Release 7/7 (Logs
`logs/probes-review_probes_r2-{debug,release}.log`). Kein Memory-Assert kann
den redb-Lauf verhindern: Jede Assertion kodiert das neue Soll.

| Probe | Beobachtung (beide Backends gleich) |
|---|---|
| **r2-P4b** (1) | Unveränderter P4b-Aufbau: 20/20 `Waiting`, `AwaitDetach`, `renew` → `Ok(false)`, der nächste Service → `Waiting`, kein Fehler |
| **r2-P4b** (2) | Echtes `DisconnectNotApplied` durch verlorene Übergaben: Quarantäne nach **9** entnommenen Disconnects in Iteration 18. `renew` → `Ok(true)`, der nächste Service → `Ok(Quarantined{DisconnectNotApplied, attempts 0})`, **kein** `NativeStatusInvariant`, kein Submit. Danach `renew` → `Ok(false)`, 5 weitere Iterationen ohne Control-Item (kein Resend), retry 0, Spieler verbunden |
| **r2-P5** | Wie P5, aber Bees Detach-Fakt wird an die Session gemeldet. Retry-Serie `[0,0,0,0,0]`, Ay `RevalidationRefused` + `Sent`. Echter Detach, genau 1 Submit, `Confirmed` nach 4 Services, 1 Versuch, Control nicht live. Danach D5 (2), D7/D8 (2), Release von Ay und Bee, 0 Records |
| **r2-P7** | 48 Hotbar-Items, Fenster 4: keine Quarantäne, Detach in Iteration 13, genau 1 Control-Item entnommen, Leave-Save `Confirmed` in Iteration 15 mit 1 Versuch, Release. Variante **sustained** (der Spammer füllt den Datenteil der Lane jede Iteration bis zur Reserve nach): identische Werte, FIFO hält die Position |

### 3.3 Neue Probes, die die geänderten Pfade kreuzen (Memory **und** redb)

| Probe | Beobachtung |
|---|---|
| **N1** Rejoin während eines F1-Records in Quarantäne | Nach `renew` und Rückkehr in `DisconnectNotApplied` (0 Versuche): Session-Reservation → `DuplicatePlayer`; Sim-Rejoin wird mit „player is already owned by this shard“ verweigert; D5 retiriert 0. Späte echte Übergabe des letzten verlorenen Disconnects → `Confirmed`-Detach. Der Record bleibt `Quarantined{DisconnectNotApplied}` (Final-Save verbraucht, `renew` → `Ok(false)`). D5/D7/D8 je 1, die Reservation geht jetzt durch. Release → `RetentionNotReleasable("leave-save not confirmed")`, der Record bleibt, Cancel gibt die Reservation frei, retry 0 |
| **N2** Stop-Phase A mit V1-Rückstau und F1-Record | X ist vorab `DisconnectNotApplied`. 48 Data-Items des Spammers, dann `disconnect_native` für Spammer, Ann und Ben, `renew` für alle: `[Ok(true), Ok(false), Ok(false), Ok(false)]`. Fenster 4: alle drei Detaches in Iteration 13, alle Leave-Saves `Confirmed` in Iteration 15, 0 Quarantänen, genau 3 Control-Items, retry max. 0. X liefert in 15/15 Services `Quarantined{DisconnectNotApplied}` mit 0 Versuchen, nie einen Fehler. D5/D7/D8/Release 3/3, danach `renew` → `StaleRetention`. X bleibt `Leaving` und verbunden, Release „not retired“ |
| **N3a** Replacement mit verlorenen Übergaben, alte Lane geleert | Ein Resend liegt beim Replacement in der alten Lane; der Driver leert sie zuerst (und verliert ihn). Nach der Revalidierung läuft die Zählung auf der neuen Lane weiter: Quarantäne nach **insgesamt 9** entnommenen Disconnects über beide Lane-Generationen. Kein Reset, keine Doppelzählung |
| **N3b** Resend fällt in das Suspensionsfenster | Nach dem Replacement löst die Zählung einen Resend aus, solange die Session suspendiert ist: `Retained`, retry `[1,1,1]`, nur `Waiting`. Die Revalidierung sendet ihn als erstes Item der neuen Lane. Quarantäne nach insgesamt 9, retry danach 0 |
| **N3c** Rückstau beim Replacement, Drain nach §4.3 | Spammer-Rückstau plus Disconnect in der alten Lane. Der Driver leert die alte Lane zuerst mit Fenster 4. Detach in Iteration 13, 1 Control-Item, kein Resend, beide Sessions revalidiert |
| **N3d** Driverfehler: alte Lane ungeleert verworfen | Der Disconnect wird nie entnommen. 100/100 `Waiting`, `AwaitDetach`, 0 Control-Items auf der neuen Lane, Spieler bleibt verbunden, retry 0, `renew` → `Ok(false)` (§6) |
| **N4** Gehaltener Disconnect (R2-A5, Enqueue verweigert) | Entnommen, 30 Boundaries nicht übergeben: 30/30 `Waiting`. Genau **ein** Resend wird gezählt und liegt in der Lane. Nach Übergabe des gehaltenen Inputs folgt der Detach. Der zusätzliche Resend ist nicht mehr live (`native_lane_input_live` false) und wird verworfen. Leave-Save, D5, D7/D8 und Release folgen |

### 3.4 Mutationskontrolle

`scripts/run_mutations.sh`, Log `logs/mutations.txt`. Jede Mutation nimmt
genau einen Fix zurück:

| Mutation | Scheiternde eigene Probes | Grund |
|---|---|---|
| M1: F1-Zweig wirkungslos (Muster auf `StatusInvariant`) | r2-P4b, N1, N2 | `NativeStatusInvariant` (Driver-Unwrap bzw. `next`) |
| M2: Refused folgt der neuen Lane nicht | r2-P5 | Retry-Serie `[1,1,1,1,1]` |
| M3: `Queued` zählt wie `Taken` | r2-P4b, r2-P7, N2, N3, N4 | Resends im Rückstau (N2 12 statt 3 Control-Items, r2-P4b 16 statt 20 `Waiting`, N3 4 statt 9). r2-P7 scheitert an der eigenen Belegungsrechnung (Überlauf), weil die Mutation zusätzliche Resends einreiht |

Alle drei Dateien wurden danach zurückgesetzt (`restored=yes`), und
`mutation/` stimmt wieder mit `probe/` überein.

## 4. Source-Review

### 4.1 Diff Runde 1 → Runde 2

Er umfasst 8 Dateien mit 711 Diff-Zeilen und ist inhaltlich identisch mit
`round2-vs-round1.diff` (`85c292e8…`; nur die Kopfzeilen unterscheiden sich).
Geändert ist **nur das Nötige**:

- `native/lane.rs`: In `NativeLaneIdentity` kommen die Zähler `sent`/`taken`
  (`AtomicU64`, `Relaxed`) hinzu, außerdem `LaneMark` und
  `record_take` in `try_recv`.
- `native/mod.rs`: Re-Exporte.
- `native/retention.rs`:
  - `LaneDisconnect` ersetzt `Lane{sent}`;
  - `FinalSave` ersetzt `final_save_used`;
  - F1-Zweig in `await_detach`;
  - Lane-Resend mit `sent_at: None`;
  - `lane_resend_retained` entfällt.
- `router/native.rs`:
  - F2-Arm im Replacement;
  - `disconnect_mark`;
  - `send_lane` liefert die Position;
  - `flush_disconnect` setzt die Marke.
- `router/native/retention.rs`: Pfadableitung `Retained`/`Queued`/`Taken`.
- `INVARIANTS.md`.
- `native_tests.rs`: nur der Konstruktor.
- `tests/native_lifecycle.rs`:
  - neuer Kopfkommentar und neue Imports;
  - geändert ist nur Teil 1 von `status_closure_refusals_are_typed` (V1
    erzwingt es);
  - dazu die Helfer und drei neue Tests.

Keine weitere Assertion wurde geändert. Die öffentliche API bleibt unverändert,
es kommt keine Dependency hinzu, und `Cargo.lock` ist bytegleich.

### 4.2 F1 — **behoben**

- `Retention::final_save: Option<FinalSave{reason, attempts}>`
  (`native/retention.rs:258-286`).
- `renew_final_save` erneuert genau einmal je quarantiniertem Record
  (`:648-662`).
- `await_detach` (`:381-402`): Ein `Due`-Record mit
  `FinalSave{DisconnectNotApplied}` und weiterhin verbundenem Spieler kehrt mit
  den Versuchen von vor der Erneuerung in die Quarantäne zurück. Er sendet weder
  Submit noch Resend und meldet keinen Fehler.
- Jeder andere Record außerhalb von `AwaitDetach` mit `Ineligible(connected)`
  bleibt `StatusInvariant`. Das ist korrekt, weil er `AwaitDetach` nur über
  einen detached Status verlassen hat.
- Ist der Disconnect inzwischen angewandt, nimmt der Record den normalen
  `Due`-Pfad. Das ist genau ein echter Versuch (`attempts := 7`, Submit
  → 8).
- Das entspricht der Root-Vorgabe („in der Quarantäne halten, bestätigter
  Final-Save als Fakt, ohne neuen Versuch“).
- Belege: r2-P4b(2), N1, N2 und der neue Ersteller-Test; Mutation M1.

### 4.3 F2 — **behoben**

- Das Replacement setzt für `Suspension::Refused` derselben Registrierung
  `session.lane` auf die neue Identität (`router/native.rs:565-569`).
- Die Session bleibt `Refused`:
  - Die Revalidierung erfasst nur `Suspended` (`:1129-1137`).
  - Data verlangt `Bound` (`:1224`, `:1284`).
  - `native_lane_input_live(Data)` schließt `Refused` aus (`:1069`).
- `send_lane` verlangt weiterhin gleiche `registration_id` **und**
  `ptr_eq(native.lane, session.lane)` (`:1401-1405`). Die `registration_id`
  bleibt über Replacements stabil (`directory.rs:139`). Ein zurückgehaltener
  Disconnect geht deshalb beim nächsten Retry über die neueste Lane.
- Für die Endbehandlung gibt es zwei Wege:
  - Apply → Detach-Kette;
  - Apply-Refusal → `StaleHandle` → `Quarantined{StatusInvariant}`, ohne
    Resend (Ersteller-Test (b)).
- `retry_native_disconnects()` erreicht 0. Belege: r2-P5, N3b; Mutation M2.

### 4.4 V1 — **behoben**, vertragskonform zum §7.3-Nachtrag

**Zählung über Lane-Generation, gesendet und entnommen:**

- **Ein Sendepfad.** Es gibt genau einen Lane-Sendepfad, `send_lane`
  (`router/native.rs:1410-1414`). Er ruft nach erfolgreichem `try_send`
  `record_send()` auf. Andere `try_send`-Stellen gehen nur an Outbounds bzw.
  den Nicht-Native-Pfad (`router.rs:2071`). Ein Sendebypass, der Positionen zu
  niedrig ansetzen und eine Marke vorzeitig „entnommen“ melden würde, existiert
  in Produktion nicht. Nur der Test `native_tests.rs:459` sendet direkt, mit
  eigener Identität.
- **Ein Empfangspfad.** Es gibt genau einen Empfangspfad,
  `NativeShardReceiver::try_recv` (`native/lane.rs:194-215`). Er zählt jede
  entnommene Position, auch eine vergiftete Bewegung, und hält so die
  FIFO-Positionen konsistent. Einen anderen Receive-Zugang gibt es nicht.
- **Eine Markenstelle.** Die einzige Stelle, die `DisconnectState::Sent` setzt,
  setzt auch die Marke (`:1373-1377`). Position und Marke stammen garantiert aus
  derselben Generation, weil `send_lane` `ptr_eq(native.lane, session.lane)`
  erzwingt.
- **Ohne Lock, ohne Allokation.** Zwei monotone `AtomicU64` und ein `Arc`-Clone
  je gesendetem Disconnect. Router und Receiver laufen in einem Task;
  `Relaxed` kann höchstens zu spät „entnommen“ melden, nie zu früh.
- **Gegentest.** Lässt man `Queued` als `Taken` gelten (Mutation M3), schlagen
  die Rückstau-Probes an.

**Gilt ein Disconnect als entnommen, ohne an die Sim gegangen zu sein?** Ja,
in drei Fällen, alle begrenzt:

- **Gehaltener Input (R2-A5, Enqueue verweigert).** Solange er gehalten wird,
  ruft der Driver nach §4.4 kein weiteres `try_recv` auf. Es zählt also genau
  ein Resend, der folgende bleibt `Queued` (N4). Offengelegt in Delivery
  §11.4.
- **Verlorene Übergabe (Driverfehler).** Sie zählt, und das ist der einzige
  Weg zu `DisconnectNotApplied` (Delivery §11.4/§11.8.3).
- **Vergiftete Bewegung.** Sie ist kein Disconnect und verschiebt nur die
  Position korrekt.

**Kann ein Replacement die Zählung zurücksetzen oder verdoppeln?** Nein:

- `resends` liegt im Record, nicht in der Lane.
- Je Zählung wird `sent_at` auf `None` gesetzt, und erst eine **neue** Marke,
  die entnommen ist, öffnet die nächste Wartezeit.
- Die Marke hält ihre eigene Generation (`Arc`) und überlebt das Replacement.
  Die alte Lane wird nach §4.3 zuerst geleert.
- Beleg: N3a/N3b, je genau 9 über zwei Generationen.

**Beim Apply verweigerter gegenüber verlorenem Disconnect:**

- **Verweigert.** Die Sim lehnt nur ein fremdes oder veraltetes Handle ab
  (`apply_native_disconnect`, `lifecycle.rs:191-219`; einen Draining-Ausschluss
  gibt es für Disconnect nicht, `player_ownership.rs:396-410`). Der Status
  meldet dann `StaleHandle` → sofort `Quarantined{StatusInvariant}`, ohne
  Resend.
- **Verloren.** Der Status bleibt `Ineligible(connected)`. Je entnommenem und
  verlorenem Disconnect zählt nach einem Boundary ein Resend; nach dem neunten
  folgt `Quarantined{DisconnectNotApplied}`.
- **Gar nicht entnommen.** Es wird nie gezählt (Rückstau, und auch der
  Driverfehler aus §6).

**Gegentest „nach 9 entnommenen Disconnects folgt die Quarantäne“:**
**vertragskonform.**

- §7.3 Punkt 1 sagt: „sendet den Disconnect erneut …, höchstens 8-mal, danach
  `Quarantined{DisconnectNotApplied}`“. Das ergibt 1 Sendung + 8
  Wiederholungen.
- Nach dem Nachtrag zählt jede Wiederholung erst, wenn der vorige Disconnect
  entnommen ist, danach mindestens ein Normal-Boundary angewandt wurde und der
  Spieler weiter verbunden ist.
- Die Quarantäne fällt, sobald der neunte entnommene Disconnect nach einem
  Boundary nicht angewandt ist (`resends >= 8`, `native/retention.rs:428-434`).
- `sent_at` setzt der erste Service nach der Entnahme. Das ist konservativ und
  kostet höchstens einen Boundary je Wiederholung (Iteration 18 bei 9
  Disconnects).

## 5. Regressionen (Auflagen aus Runde 1)

Der Produktionscode außerhalb der in §4.1 genannten Stellen ist bytegleich zu
Runde 1. Unverändert sind unter anderem `audience.rs`, `lifecycle.rs`, die Tabelle und
Release.

| Auflage | Bewertung | Beleg |
|---|---|---|
| A1 in-place-Tabelle, Adoption kapazitätsunfehlbar | weiter erfüllt | P3 gleich, Session-Tests |
| A2 Refused endet definiert, retry 0 | **jetzt vollständig erfüllt** | r2-P5, N3b, Ersteller-Test (a)/(b) |
| A4 Control nach DetachFact nicht live | weiter erfüllt | N4 (zusätzlicher Resend nicht live, verworfen), r2-P5 |
| A5 Token, idempotent | weiter erfüllt | N2: `renew` nach Release → `StaleRetention` |
| A6, A7, H1 (SIM-03.4b) | unverändert erfüllt | kein Diff in den betroffenen Pfaden, Session-Tests |
| Release-Pfad | weiter erfüllt | N1 (Release ohne Bestätigung verweigert, Record bleibt), N2, r2-P5, r2-P7 |
| Token opak | unverändert | kein API-Diff |
| G3/R1/R2/R4 | weiter erfüllt | N1/N2/r2-P5 (D5, D7/D8 als eine Closure), P3, Submit-Pfad unverändert |
| Audience | unverändert | kein Diff, Session-Tests |
| Stop-Phase A (Final-Save genau einmal) | erfüllt | N2: `renew` nur für den quarantinierten Record `Ok(true)`, für nicht quarantinierte `Ok(false)` ohne Verbrauch |

## 6. Auflage für NET-04.2f: ungeleerter alter Receiver

**Einordnung korrekt:** Das ist eine Driverpflicht nach §4.3. „Der Driver legt
den alten Receiver in `draining` und leert ihn vollständig und vor dem neuen.“
Delivery §11.8 Punkt 2 benennt sie offen als Auflage für NET-04.2f, ebenso den
Unterschied zu Runde 1.

**Folgen über „bis zum Stop“ hinaus (N3d):** Der Record bleibt in
`AwaitDetach`, aber nicht quarantiniert. Daraus folgt:

- **Kein Final-Save:** `renew` liefert `Ok(false)`.
- **Phase A kann nicht übergehen** und endet über `phase_a_incomplete` mit
  `Unresolved`.
- **Der Sim-Spieler bleibt verbunden:** Es gibt keinen Detach-Fakt.

Siehe R2-H2.

## 7. Delivery

- **Zahlen reproduziert:**
  - Session 244 (241 + 3) mit der genannten Aufschlüsselung;
  - App 375 + 1 ignoriert in 40 Läufen;
  - Testkit 100 in 8 Läufen;
  - compile_fail 5/5.

  Die Dauern weichen lastbedingt ab.
- **Hashes reproduziert:**
  - Manifest `f180e54f…`, Aggregat `ad02a694…`;
  - Diffs `85c292e8…` und `351f053e…`;
  - Skript `2b04983f…`, `checks.txt` `dc615930…`;
  - alle Log-Hashes der Tabelle §11.7: `e38fd664`, `e2739343`, `66a86c4b`,
    `e3b0c442`, `da5481b6`, `d30d2915`, `9068a408`, `ff3e1b30`, `c624ba1d`,
    `f9825d16`;
  - compile_fail-JSON `cf087ae9…`;
  - Freeze `d1027049…`, 29/29, Aggregat `6feea003…`;
  - Runde 1 `a4ce2dcb…`, 27/27;
  - `server/Cargo.lock` `183450e7…`.

  Probe-Datei im Ersteller-Lauf `4f977bbf…` = Original.
- **Forbidden-Claims-Scan** (`scripts/forbidden_claims_scan.py`, Funktionen
  `markdown_blocks`, `rustdoc_blocks`, `scan` aus
  `scripts/check-forbidden-claims.py`; auf `combined/` und auf dem Arbeitsbaum):
  - Delivery, INVARIANTS, README: **0 Treffer**.
  - Session-Rustdoc: dieselben 3 Treffer der Scan-Regel für unbelegte Kompatibilitätsaussagen in
    `router.rs:83/195/512`. Das ist unveränderter Basistext außerhalb des
    Index-Check-Bereichs.
- **Aussagen §11** stimmen mit Code und Probes überein:
  - Umkehrstellen;
  - Werte von P1–P4/P6;
  - „retry erreicht 0“ jetzt ohne F2-Ausnahme;
  - Folge für `DisconnectNotApplied`.

  Die Nicht-Aussagen sind eingehalten.

## 8. Befunde nach Schwere

**Keine Befunde.**

### Hinweise

1. **R2-H1 (V1, Annäherung offengelegt):** „Entnommen“ wird an `try_recv`
   gemessen, nicht am erfolgreichen Sim-Enqueue. Ein beim Enqueue verweigerter,
   gehaltener Disconnect (R2-A5) zählt deshalb als entnommen. Die Abweichung
   vom Wortlaut „entnommen **und an die Sim übergeben**“ ist auf höchstens
   einen gezählten Resend je Haltephase begrenzt (N4) und in Delivery §11.4
   offengelegt. Empfehlung: Root nimmt diese Präzisierung in den Text des
   §7.3-Nachtrags V1 auf.
2. **R2-H2 (Auflage f, Wortlaut):** Delivery §11.8.2 schließt mit „Replacement
   erst nach vollständigem Drain des alten Receivers“. §4.3 verlangt die
   umgekehrte Reihenfolge: Replacement, dann den alten Receiver vollständig
   **vor** dem neuen leeren. Die Folgen eines Verstoßes sind größer als „bis
   zum Stop“ (§6).

   Empfehlung: Die f-Auflage nach dem Wortlaut von §4.3 formulieren und mit
   einem f-Test belegen (alter Receiver mit Disconnect → vollständig geleert).
   Optionale spätere Härtung: Die Session könnte eine Marke auf einer
   verworfenen Lane-Generation erkennen und den Disconnect wieder
   zurückhalten.
3. **R2-H3 (F1-Folge, informativ):** Der Final-Save eines noch verbundenen
   `DisconnectNotApplied`-Records unternimmt keinen neuen Disconnect-Versuch.
   Wird der Disconnect danach doch angewandt, folgt kein Leave-Save mehr, weil
   der Final-Save verbraucht ist. Release bleibt verweigert, L1 bleibt bis zum
   Neustart gesperrt (N1). Das entspricht der Root-Vorgabe und betrifft nach V1
   nur noch Driververluste.
4. **R2-H4 (vorbestehend, nur Code-Lesung, nicht geprobt):** Ein erneuerter
   `StatusInvariant`-Record (StaleHandle) kehrt mit `attempts = 7` in die
   Quarantäne zurück, ohne dass ein Versuch stattfand. Die
   Versuchs-Wiederherstellung gilt nur für `DisconnectNotApplied`
   (`native/retention.rs:336-340`, `:659`). Das ist ein kosmetischer
   Metrikwert und nicht in Runde 2 entstanden.

Die Hinweise H1–H6 aus Runde 1 bleiben wie dort bewertet (H1 dokumentiert in
§11.8.1).

## 9. Empfehlung an Root

CLEAN_RUNTIME für NET-04.2e2 Runde 2, Integration möglich. Für die
Vertragsentscheidungen und NET-04.2f:

- den §7.3-Nachtrag V1 eintragen, mit der Präzisierung „entnommen = aus der
  Lane genommen; ein gehaltener Input zählt höchstens einmal“ (R2-H1);
- die Drain-Auflage nach §4.3 formulieren, Folgen nach §6 (R2-H2);
- die offenen Punkte aus Runde 1 (Held-Slot-Resync, Teleport-Stempelung,
  R3-A5, Konsumentenreferenzen) bleiben wie ausgewiesen.
