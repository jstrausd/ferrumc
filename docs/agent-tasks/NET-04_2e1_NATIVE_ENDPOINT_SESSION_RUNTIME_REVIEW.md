# NET-04.2e1 — Unabhängiger Runtime-Review: native Endpoint-Art in der Session

**Verdikt: FINDINGS.** Ein kleiner blockierender Befund (B1), sieben Auflagen
für e2/f (A1–A7) und zwölf Hinweise (H1–H12).

Die Runtime selbst ist solide. Alle frischen Checks sind grün. Es gibt kein
Autoritätsleck, und F1, F2 und F4 sind eingehalten. Blockierend ist nur dies:
Eine e1-Matrixzeile und die F4-Coalescing-Tests laufen nur auf Memory statt auf
Memory **und** redb, und die Delivery behauptet das Gegenteil. Behoben ist das
mit `for store in STORES` in fünf bis sieben Tests und einer korrigierten
Delivery-Zeile. Meine eigenen Probes zeigen das Verhalten auch auf redb (P2, P5).

Review-Subagent, 11.10.2026. Den Kandidaten habe ich nicht erstellt und nicht
verändert. Ich treffe keine Aktivierungs-, Survival-, Performance- oder
Live-Client-Aussage.

## 1. Prüfbasis und Integrität

| Gegenstand | Wert |
|---|---|
| Kombinationsarchiv | `.codex-tmp/net04_2e1-runtime-root-review/combined`, `combined.freeze.json` SHA-256 `ff5137cf3a9a4f555360b79e41ab8489ea2cd2c13fad6d7ae4c7d7c68fd3509f`: vor und nach allen Läufen 1311/1311 bytegleich, keine Zusatzdatei |
| Kandidatenmanifest | `0f7f4b9989afe33ca899e0b218fff9e62a37d1944d2792f8c48886e23d5fd1c1`, 30 Einträge: 30/30 identisch im Arbeitsbaum und in `combined/` (nach den Checks erneut geprüft) |
| Evidence-Freeze | `0018cba417e06c9c25db25825f0115cb2b555d731e7531d661ebb0c0fca46cab`: 24/24 Dateien hashgleich |
| Delivery | `3969dc4c28ed34f6a000b1656e73a938cb1dc56fb23823ade5a98289a49d00a4` |
| Vertrag / .2c / Root-Fragen | `e0530e0a…` / `3dff58ac…` / `2c8c5ebb…`, alle frisch nachgerechnet |
| Private Arbeitskopie | `.codex-tmp/net04_2e1-runtime-root-review/work` (Kopie von `combined/`), eigenes Target `target-review` (frisch), `CARGO_INCREMENTAL=0` |
| Toolchain | cargo/rustc 1.99.0 (`~/.cargo/bin`), alle Läufe mit `--jobs 2 --locked` |

Ergebnisse: `final-assessment.json` und `evidence/` im Review-Ordner. Die Logs
liegen unter `logs/`, mit SHA-256 in `final-assessment.json`.

## 2. Ausführung (frisch)

| Check | Exit | Ergebnis |
|---|---|---|
| `cargo test -p ferrumc-session` (Debug) | 0 | **205** bestanden: 150 Unit, `native_borrow` 2, `native_endpoint` 2, `native_join` 11, `native_lane` 13, `shard_directory` 15, `source_scan` 2, Doctests 10 |
| dito `--release` | 0 | **205** bestanden |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | grün |
| `cargo fmt --all --check` | 0 | leeres Log |
| `RUSTDOCFLAGS=-Dwarnings cargo doc -p ferrumc-session --no-deps` | 0 | grün |
| `RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps` (+ `--keep-going`) | 101 | Fehler in **9 Crates, die gegenüber `main` unverändert sind**: app, plugin-sdk-builtin, dashboard, bench, plugin-greeter, plugin-api, placement, observability, codec. `ferrumc-session` ist dabei fehlerfrei. Das ist eine vorbestehende Baseline, nicht vom Kandidaten verursacht (H9) |
| `cargo build --workspace --all-targets` | 0 | grün |
| `cargo test -p ferrumc-app` | 0 | **375** bestanden, 1 ignoriert (Basis-`#[ignore]`: echter Java/Maven-Bridge-Test) |
| `cargo test -p ferrumc-testkit` | 0 | **100** bestanden |
| Eigene Probes `tests/review_probes.rs` (nur in der Kopie) | 0 | 7/7 in Debug und Release, Clippy `-D warnings` grün; danach wieder entfernt |
| F2-Compile-Gate: `use ferrumc_storage` in `lib.rs` injiziert, `cargo check --lib` | 101 (erwartet) | `E0432 no external crate ferrumc_storage`; `lib.rs` danach bytegleich wiederhergestellt |

Verbraucher von `ferrumc-session` ist nur `ferrumc-app`. `ferrumc-testkit`
wurde laut Auftrag zusätzlich geprüft.

**Testnebenprodukte:** Der Testkit-/App-Test `plugin_testhost` erzeugte zwei
Dateien in der privaten Kopie:
`work/.codex-tmp/p57-plugin-testhost-1919920-artifacts/{libplugin_testhost_fixture.so,librenamed_fixture.so}`.
Sonst ist keine Datei der Kopie verändert oder hinzugekommen.

**Delivery-Zahlen:** 205/205, app 375 + 1 ignoriert und testkit 100 stimmen
exakt. Ebenso stimmen 143 Basis-Unit-Tests, 15 `shard_directory` (bytegleich)
und 1 Basis-Doctest. Alle genannten Hashes stimmen.

## 3. F2-Grenze

- **Lock-Diff:** Es gibt genau `+ "ferrumc-storage"` und `+ "ferrumc-world"`
  in der Dependency-Liste von `ferrumc-session`. Es kommt kein Paket hinzu, und
  keine Version ändert sich. Der Diff-Körper ist identisch mit
  `evidence/cargo-lock.diff`.
- **Produktionscode:** Keine Erwähnung unter `src/` außerhalb
  `router/native_tests.rs` (`#[cfg(test)] #[path]`). Kein `build.rs`, kein Makro,
  kein `cfg(not(test))`-Pfad. `ferrumc-sim` re-exportiert nichts aus
  `ferrumc_storage`. Aus `ferrumc_world` re-exportiert es nur die schon in der
  Basis genutzten `BlockStateId`/`Sign*`. Die Produktion liest
  `GenerationBinding` über Sim-Getter (`key()`, `identity().encode()`). Das ist
  vertragsgemäß (§4.2) und kein Raw-Zugriff.
- **Robustheit:** Die eigentliche Durchsetzung leistet der Compiler. Dev-Dependencies
  werden nicht in das `--lib`-Target gelinkt, eine Produktionserwähnung bricht
  den Build mit E0432 (belegt). Der Source-Scan ist eine textuelle Heuristik und
  umgehbar (H1, Probe P6). Er ist Zusatz, nicht Grenze.

## 4. Source-Review gegen den Vertrag (e1-Umfang)

Geprüft wurden `src/native/{mod,config,error,lane,revalidation}.rs`,
`src/router/native.rs`, der Diff von `router.rs`/`error.rs`/`lib.rs` gegen die
Basis, INVARIANTS, README und alle neuen Tests.

**Ohne Befund:**

- **Endpoint-Art und Exklusivität:**
  - Die private `EndpointKind::{Legacy, Native}` ist umgesetzt.
  - `EndpointKindConflict` greift in beide Richtungen (`router.rs:628-634`,
    `native.rs:266-271`).
  - Legacy-Join, -Replace, -Unregister und -Route auf native Coverage ergeben
    `NativeEndpointRequiresSession` mit unversehrtem Input (`router.rs:742-746`,
    `2047-2058`).
  - Es gibt nur eine `shards.register`-Stelle für Legacy.
- **Konstruktionsrechte:**
  - `NativeShardInput`, `NativeShardReceiver`, `NativeJoinReservation`,
    `NativeRevalidationProbe` und `NativeRevalidationVerdict` haben
    `pub(crate)`-Felder, keinen Konstruktor und kein `Clone`.
  - `PlayerSessionKey` ist `Clone`, aber nur Routingdatum. `PartialEq` prüft
    `Weak::ptr_eq` + Spieler + Inkarnation, `RouterIdentity` ist eine ZST hinter
    `Arc`. Die Allokation bleibt durch `Weak` belegt, deshalb gibt es keine
    Adresswiederverwendung.
  - `compile_fail`: 10/10 aus dem richtigen Grund (`compile_fail.json`; Clone →
    E0308 über `&T: Clone`, private Felder).
- **Lane:**
  - Die Lane ist bounded (`lane_channel(shard_input_capacity)` ≤ 1024).
  - Data stoppt bei `capacity() <= reserve`, Control nutzt die Reserve
    (`native.rs:1118-1125`).
  - Voll heißt kontrollierter Leave, und der Disconnect wird dedupliziert
    zurückgehalten.
  - `into_parts()` erhält Key und Klasse.
- **Replacement und Unregister:**
  - Replacement verlangt die aktuelle Lease und `Arc::ptr_eq` des aktuellen
    Receivers. Alle `Bound`-Sessions der Registrierung werden `Suspended`, die
    Zelle wird geschlossen (`native.rs:338-348`).
  - Unregister zählt Sessions und Reservationen und gibt den Receiver bei einer
    Verweigerung zurück.
- **Revalidierung (F1/R3-A1):**
  - Die Signatur entspricht der Root-Entscheidung, die Worklist ist per
    `try_reserve_exact` reserviert.
  - Die Probe-Sequenz ist geprüft. Ein Verdict mit fremder Sequenz gilt als
    Mismatch.
  - `submit` vergleicht Spieler, `matches_owned_player` (Owner-`Weak`, Spieler,
    Admission) und die volle Generation.
  - Der Treiber erhält kein Handle über den Aufruf hinaus. Der geteilte und der
    exklusive Borrow sind belegt (`native_borrow.rs`, beide Backends).
- **Region (A10):** `region_contains` nutzt
  `EntityTransform::try_new(p,0,0)?.chunk()`. Session- und Sim-Entscheidung sind
  an 13 Kanten identisch, auf beiden Backends.
- **Allocator:** `next_entity_id: Option<i32>`, `checked_add`, `MAX` genau
  einmal, kein Wrapping. Im Legacy-Join wird die Verfügbarkeit vor
  Slowviewer-Disconnect und `PlayerJoin` geprüft, vergeben wird an der
  historischen Stelle (`router.rs:788-790`, `822-824`).
- **reserve/admit/cancel:** Alle Refusals kommen vor Seiteneffekten. `admit`
  prüft Reservation, aktuellen Receiver, Handle/View/Generation, die
  bitgenaue Position und 21 JoinGame-Felder. JoinGame ist das erste
  Mandatory-Paket.
- **disconnect_native:** idempotent, genau ein Control-Item, Eintrag bleibt
  `Leaving`.
- **Kein Lock über `.await`:** Im nativen Code gibt es keine `async fn`.
- **Kein `unwrap`/`expect`/`panic!` außerhalb von Tests** im neuen Code; die
  Treffer liegen nur in vorbestehenden Testmodulen.
- **Rustdoc:** auf allen neuen öffentlichen Items vorhanden. Session-Rustdoc ist
  unter `-D warnings` grün.

**Gezielte Suche:**

- **Autoritätslecks:** Keine gefunden. Ein Key öffnet keine fremde Session,
  ein Verdict lässt sich nicht fälschen oder wiederverwenden, ein Input lässt
  sich nicht bauen. Receiver und Reservation haben kein `Clone`.
- **Retargeting:** Keines. Alte Keys anderer Inkarnationen oder Router ergeben
  `StaleSession`. Ein Rejoin ist in e1 ohnehin durch `DuplicatePlayer`
  gesperrt (P4).
- **Unbounded:** Die Tabellen sind durch das Sessions-Limit begrenzt. Ein
  Restpunkt steht in H3/H7.

## 5. F4-Coalescing

**Semantik:**

- `session.movement` zeigt nur dann auf eine Zelle, wenn deren Lane-Item das
  neueste der Session ist. Jeder Barrierepfad setzt `None`:
  - Nicht-Bewegung: `native.rs:1013`;
  - Teleport-Confirm: `754`;
  - Korrektur/Resync: `1135`;
  - Rebind: `937`;
  - Leave: `1057`;
  - Replacement: `346`;
  - `Taken`/`Poisoned`: `985`.
- Merge nur bei `!taken` unter der Sperre. `take` setzt `taken` unter der
  Sperre und kopiert nur Felder und einen `Weak`-Clone. Kein Lock wird zusammen
  mit einem anderen oder über `.await` gehalten. Es gibt keine globale Zelle.

**Ergebnis:**

- **Überholen** ist unmöglich, weil Merges nur in das neueste Item der Session
  gehen.
- **Duplikate** sind unmöglich, weil das Item einmal konsumiert und `taken`
  atomar gesetzt wird.
- **Verlust** gibt es nur wie vorgesehen:
  - Zwischenstände werden zusammengefasst;
  - Bewegung während einer Suspension wird verworfen (Vertrag A3);
  - eine abgelehnte Regionsbewegung wird verworfen;
  - eine vergiftete Zelle wird typisiert verworfen (praktisch unerreichbar, H12).
- Teleport-Confirm, Resync und Regionskorrektur trennen korrekt; das ist getestet.
- Die Entnahme nach dem Dequeue, aber vor dem Lock liefert korrekt den neueren
  Stand.

**Race-Test** (`native_lane.rs:236-284`): Er ist aussagekräftig. Echte Threads
gehen über die öffentliche Router-API, 2000 Schritte, die Werte steigen streng
und der letzte Stand kommt an. Er belegt aber nicht, dass beide Zweige
(`Taken` nebenläufig, `Merged` nach dem Dequeue) tatsächlich getroffen wurden
(H2).

**Eigene Probes** (`evidence/review_probes.rs`, echte bounded Lanes, echte
Admissions im Counter-Modus):

| Probe | Backends | Ergebnis |
|---|---|---|
| P1 Replacement während gefüllter, nicht entnommener Zelle | Memory + redb | Alte Lane liefert genau 1 Item mit dem neuesten Stand vor dem Replacement (x=12), danach `Disconnected`. Kein Merge in die alte Zelle während der Suspension. Nach der Revalidierung neues Item nur auf der neuen Lane. Sim-Pose 12, dann 15. Grün |
| P2 Disconnect mit ausstehender Bewegung, Lane bis zur Reserve voll | Memory + redb | Data stoppt vor der Reserve (C → Leave). A koalesziert weiter, der A-Disconnect nutzt die Reserve **hinter** der A-Bewegung (x=11). B wird genau einmal zurückgehalten, nach dem Leeren genau einmal gesendet, kein Duplikat. Grün |
| P2b ausstehende Bewegung vor Disconnect in der Sim angewandt | Memory + redb | Pose (40, 41) angewandt, danach detached. Grün |
| P3 Refused/Detached nach Revalidierung | Memory + redb | Beide bleiben dauerhaft `Retained`, `retry_native_disconnects() == 2` für immer. Der Sim-Spieler von E bleibt **verbunden** (→ A2). Beobachtung grün |
| P4 e1-Übergabezustand | Memory | UUID nach Leave `DuplicatePlayer`, Unregister `NativeEndpointInUse{sessions:1}`. Grün (wie dokumentiert) |
| P5 gehaltenes Data-Item der alten Lane | redb | Live während Suspension; nach `RevalidationRefused` nicht mehr live. Grün |
| P6 Source-Scan-Heuristik | — | `'{'` in einem Testmodul verschluckt eine folgende Produktionszeile; `#[cfg(test)] #[path="router.rs"] mod x;` markiert eine Produktionsdatei als Testdatei. Grün (= umgehbar) |

## 6. Befunde

### Blockierend

**B1 — e1-Matrix nicht vollständig auf Memory und redb; Delivery-Aussage
unzutreffend.**

- **Belege:**
  - `tests/native_lane.rs:511, 540` (`full_lane_…`) und `:597` (`held_lane_…`)
    decken die §14-Zeile „Lane voll / Sim-Enqueue verweigert“ (e1/f) ab, laufen
    aber nur mit `Store::Memory`.
  - Ebenso nur auf Memory laufen die F4-Pflichttests `:129` (Spam), `:157`
    (Barriere), `:182` (Confirm/Korrektur), `:214` (Take vs. Replace) und
    `:237` (Race).
  - Vertrag §14, Kopf: „Alle Tests mit echten bounded Routerkanälen, echter
    Memory- **und** redb-Ownershipadmission“.
  - Delivery `:241-244`: „Alle Integrationstests laufen … auf **Memory und
    redb**“.
- **Szenario:** Root nimmt die Matrix als vollständig belegt ab, obwohl die
  Zeile Lane-voll/held und die F4-Auflagen nie gegen redb liefen.
- **Laufzeitrisiko:** gering. Session-Logik und Sim-`Capacity`-Refusal hängen
  nicht vom Backend ab, und P2/P5 zeigen dasselbe Verhalten auf redb. Es bleibt
  aber eine unzutreffende Abnahmebehauptung.
- **Behebung:** Die genannten Tests mit `for store in STORES` ausführen und die
  Delivery-Zeile korrigieren. Andernfalls nimmt Root die Ausnahme ausdrücklich
  auf.

### Auflagen (e2/f; in e1 ohne Live-Wirkung, da es keinen Driver gibt)

1. **A1 — R3-3/§5 Schritt 6 nicht umgesetzt und nicht offengelegt (e2).**
   - **Befund:** Die Reservation zählt in das Limit, alloziert aber keinen
     Tabelleneintrag, den Session oder Adoption später übernehmen.
     `native_reservations.insert` (`native.rs:499`) und
     `native_sessions.insert` (`:605`) sind zwei infallible BTreeMap-Inserts.
     Es gibt weder `AllocationFailed` noch denselben Eintrag.
   - **Folge:** Die Adoption in e2 wäre so nicht „kapazitätsunfehlbar ohne
     Allokation“.
   - **Auflage:** e2 braucht eine einheitliche, vorreservierte Tabelle
     (Reservation → Session/Adoption → Retention ohne Neuallokation). Das
     Delivery sollte die Abweichung nennen.
2. **A2 — `RevalidationRefused` lässt den Sim-Spieler verbunden (e2/f).**
   - **Befund:** Nach einem Mismatch wird die Session `Leaving`, ihr
     Disconnect ist aber für immer `Retained`, weil `flush_disconnect` nur bei
     `Bound` sendet (`native.rs:1075`). Auch `native_lane_input_live(Control)`
     liefert `false`. Der Sim-Spieler bleibt admitted, und Stop wird nie
     quiescent (P3).
   - **Zusätzlich:** `retry_native_disconnects()` zählt Refused- und
     Detached-Sessions dauerhaft als „retained“ (`:688-699`). Ein Driver, der
     bis 0 wiederholt, terminiert nie.
   - **Auflage:** e2/f müssen die Disposition festlegen. Möglich sind ein
     Disconnect aus dem gespeicherten Handle über die aktuelle Lane, weil Sim
     beim Apply ohnehin prüft, oder ein typisierter Fail-Stop. Detached sollte
     nicht als „retained“ zählen.
3. **A3 — Pose-Quelle für Korrekturen und Resync (e2, vor jeder Aktivierung).**
   - **Befund:** Korrekturen synchronisieren auf `session.pose`
     (`native.rs:1139`). Das ist in e1 die Admission- bzw. Revalidierungs-Pose.
     Nach legitimer Bewegung teleportiert eine Regionskorrektur den Client also
     auf einen veralteten Punkt (offengelegt, Abweichung 6).
   - **Zusätzlich:** Der Revalidierungs-Resync liest die Pose, bevor die
     geleerten Alt-Lane-Items angewandt sind. `native_endpoint.rs:197-201`
     verankert „genuine pose before the move“ (8,5), danach wendet Sim 9,5 an.
     Client und Sim weichen voneinander ab.
   - **Lücke im Vertrag selbst:** Ein verworfener Hotbar-Wechsel wird nur mit
     einem Positions-Sync beantwortet, der Held-Slot des Clients bleibt
     abweichend.
   - **Auflage:** e2 liefert den Pose-Producer; f revalidiert nach dem Boundary,
     der die Alt-Items anwendet. Ein passender Held-Slot-Resync gehört in e2/f.
4. **A4 — `native_lane_input_live(Control)` nach DetachFact (e2).** In e1 ist
   Control für jede `Bound`/`Suspended`-Session live (`native.rs:801-813`). R2-A5
   verlangt `false` für „Control einer bereits detachten Session“. Das setzt den
   DetachFact aus e2 voraus.
5. **A5 — `disconnect_native` → `NativeRetentionToken` (e2).** Das ist eine
   API-Änderung auf demselben Eintrag. Der Status `disconnect = Retained` für
   Detached-Sessions ist irreführend.
6. **A6 — Teleport-ID 0 (f).** Korrekturen brauchen IDs aus `PendingTeleport`,
   und Bewegungen müssen bis zum passenden Confirm verworfen werden (Vanilla
   `awaitingPositionFromClient`). Sonst wenden sich veraltete Bewegungen nach
   einer Korrektur an.
7. **A7 — Vertragsnachtrag (Root).** Die Signaturabweichungen sollten im
   Decision-Log festgehalten werden: Unregister-Fehler mit Receiver,
   `session_key() -> Option`, `route_native_play`/`route_native_game_mode` und
   das Fehlen einer Fehlervariante `NativeEndpointRevalidationRefused`
   (typisiert über `NativeEndpointState::RevalidationRefused` und
   `NativeRevalidation::refused`).

### Hinweise

1. **H1:** Der Source-Scan ist umgehbar, siehe P6 (`tests/source_scan.rs:28-93`).
   Das ist unkritisch, weil der Compiler die Grenze hält. Robuster wäre ein Test
   auf `cargo metadata` (nur `kind = "dev"` für storage/world).
2. **H2:** Der Race-Test sollte zählen, dass `Coalesced` und `Queued` nebenläufig
   beide auftreten. Bei 2000 Schritten ist das nicht garantiert belegt.
3. **H3:** `retry_native_disconnects` sammelt in einen infalliblen `Vec`
   (`native.rs:689`). Er ist begrenzt durch die Tabelle, aber inkonsistent zum
   `try_reserve` der Revalidierung.
4. **H4:** Eine erschöpfte Probe-Sequenz wird als `SessionIdsExhausted`
   gemeldet (`native.rs:875`); der Name passt semantisch nicht.
5. **H5:** INVARIANTS `:42` sagt, die Zelle gehöre genau einem Session-Eintrag.
   Tatsächlich gibt es eine Zelle je Lane-Item, höchstens eine offene je Session.
   `config.rs:101` „including pending leaves“: Legacy-`pending_disconnects`
   zählen nicht ins Limit.
6. **H6:** Das Coalescing ist strenger als R3-A2: Ein entnommenes, aber vom Sim
   abgelehntes (gehaltenes) Item wird nicht mehr gemergt. Das ist zulässig und
   kostet nur Lane-Plätze.
7. **H7:** `name`/`equipment` werden ungeprüft gespeichert
   (`native.rs:622-623`). Sie stammen vom Driver; der Konsument in e2 sollte sie
   begrenzen.
8. **H8:** On-Ground aus R3-A2 lässt sich nicht abbilden, weil
   `OwnedPlayerInput::move_player` kein On-Ground-Feld hat. Das ist kein Defekt.
9. **H9:** Rustdoc `-D warnings` scheitert auf Workspace-Ebene an 9 unveränderten
   Crates. Die Delivery behauptet zu Recht nur Session-Rustdoc grün. Root sollte
   das nicht als Workspace-Aussage lesen.
10. **H10:** Die 9 Doc-Link-Korrekturen (presentation 6, router 2, text 1) sind
    zulässig. Es sind reine Doku-Änderungen, im eigenen Crate, ohne
    Verhaltensänderung. Die Navigation verliert drei Modul-Links.
11. **H11:** Ein Driverfehler mit falschem Verdict führt zum Leave der Session.
    Das ist fail-safe und akzeptabel.
12. **H12:** Wird ein Merge `Poisoned` und das Folge-Item ist reine Rotation,
    geht die Position der vergifteten Zelle verloren. Praktisch ist das
    unerreichbar, weil die kritischen Abschnitte nicht paniken können.

## 7. Bewertung der offengelegten Abweichungen

| Abweichung | Bewertung |
|---|---|
| Unregister liefert bei Verweigerung den Receiver (`NativeUnregisterError`) | **zulässig**, strenger als der Vertrag (keine Lane geht verloren); Nachtrag A7 |
| `session_key()` liefert `Option` | **zulässig** (Legacy-Handles minten keinen Key) |
| `disconnect_native` liefert in e1 `Sent`/`Retained` | **zulässig** nach F3; **Auflage A5** |
| Benennung `route_native_play`/`route_native_game_mode` | **zulässig** (§6.6 ist abstrakt); A7 |
| Teleport-ID 0 | **zulässig in e1**; **Auflage A6** |
| Pose nur aus Admission/Revalidierung | **zulässig in e1**; **Auflage A3**, Aktivierungsvoraussetzung |
| 9 Rustdoc-Links außerhalb des Tickets | **zulässig** (H10) |
| *nicht offengelegt:* Reservation ohne übernehmbaren Tabelleneintrag | **Auflage A1** |

## 8. Matrix §14, e1-Zeilen

| Zeile | Echte Tests | Memory | redb | Bewertung |
|---|---|---|---|---|
| Native Registrierung; Legacy-Exact → `EndpointKindConflict`; Legacy-Route → Input intakt | `legacy_and_native_coverage_never_share_a_scope`, `a_legacy_input_resolved_…` | — | — | vollständig (keine Admission nötig) |
| Replacement mit alter Lease / ohne aktuellen Receiver | `replacement_suspends_…` | ✓ | ✓ | vollständig (Hinweis: alter Receiver derselben Registrierung mit aktueller Lease nicht separat geprüft) |
| Replacement korrekt mit Active + Leaving (e1-Teil) | ebenda + P1 | ✓ | ✓ | vollständig für e1 |
| Unregister mit Session/Reservation (Record/Adoption: e2) | ebenda, `reservation_refusals_…` | ✓ | ✓ / — | vollständig für e1 |
| **Lane voll / Sim-Enqueue verweigert** (e1-Teil) | `full_lane_…`, `held_lane_…` | ✓ | **✗** | **B1**; „Session detached → held verworfen“ nur als *Leaving → Data* belegt, Control nach DetachFact ist e2 (A4) |
| Bewegung über Regionsgrenze; Chunkgrenze ±ε; Restore außerhalb (e1-Teil) | `region_decisions_…`, `invalid_transforms_…`, Reservation-Refusal | ✓ | ✓ (Region) | vollständig für e1; „Spawn außerhalb“ ist f |

Alle Tests laufen über echte bounded Router-Kanäle und echte Admissions
(Restore und receipt-gestützter Bootstrap) im Counter-Modus
(`initialize_owned_item_counters`, Refill vor dem ersten Boundary,
`run_tick_with_player_ownership_at`). Fake-Facts habe ich nicht gefunden.

## 9. Delivery

- Die Zahlen stimmen: 205/205 Debug und Release, app 375 + 1 ignoriert,
  testkit 100, Aufschlüsselung je Test-Binary.
- Die Hashes stimmen: Manifest, Evidence-Freeze, Lock-Diff, `compile_fail.json`,
  Delivery, Vertrag und Root-Fragen.
- Die Nicht-Behauptungen sind vorhanden: keine Aktivierungs-, Survival-,
  Performance-, Paper- oder Live-Client-Aussage.
- Unzutreffend ist allein die Backend-Aussage in §4 (B1). Die
  Reservation/R3-3-Abweichung fehlt in der Liste (A1).
- **Forbidden-Claims-Scan** (`markdown_blocks` + `scan` aus
  `scripts/check-forbidden-claims.py`, direkt angewandt):
  - Delivery: 0 Treffer (114 Blöcke);
  - INVARIANTS: 0 (26);
  - README: 0 (4).
  - Rustdoc der Session-Quellen: 3 Treffer der Regel für unqualifizierte
    Kompatibilitätsaussagen in `router.rs:83/195/513`. Sie sind textgleich in der Basis vorhanden
    (`:73/166/451`) und liegen außerhalb des CI-Scan-Umfangs.
  - Belege: `evidence/forbidden_scan.json`.

## 10. Empfehlung an Root

CLEAN_RUNTIME nach Behebung von B1: Tests auf `STORES` erweitern und die
Delivery-Zeile korrigieren, oder die Ausnahme ausdrücklich aufnehmen. Danach
ist eine Integration vertretbar. A1–A6 gehören als gebundene Auflagen in die
Tickets e2/f, A7 in das Decision-Log. Für die Integration ist dabei zu
beachten, dass im Arbeitsbaum ein anderer Agent `ferrumc-storage` ändert. Der
Review deckt ausschließlich `main ce39d817` + Kandidat ab.

## Root-Abnahme nach Runde 3

Root, 11.10.2026. Der einzige blockierende Befund B1 ist in Runde 3 behoben.
Gegenüber dem geprüften Runde-2-Kandidaten ändern sich nur
`src/router/native_tests.rs` (nur unter `#[cfg(test)]` eingebunden) und
`tests/native_lane.rs`; Produktionscode, `Cargo.toml` und `Cargo.lock` sind
bytegleich (Manifest Runde 3 `665feb86`). Root hat auf einer frischen Kopie von
`ce39d817` plus Kandidat selbst ausgeführt: Session-Tests Debug und Release je
206/206, `clippy -p ferrumc-session --all-targets -D warnings` und
`fmt --all --check` grün. Damit ist NET-04.2e1 CLEAN_RUNTIME; die Auflagen
A1–A7 gehen an NET-04.2e2 bzw. NET-04.2f. Dieser Bericht hatte beim Abschluss
des Reviews SHA-256 `aa59539912f6f488fa65c64f17298572ae08193913ad949935ba9a1559a7bbd2`.
