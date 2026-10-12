# SIM-03.4c — Gewählter Hotbar-Slot in der verbundenen Spieler-View: Autorenlieferung

Dies ist ein reviewbarer **Sim-only-Runtimekandidat**, keine unabhängige
Rootabnahme. Autor ist der Implementierungs-Subagent, 12.10.2026. Es gab kein
Git, keine weiteren Agents, keine Änderung an Dependencies, Lockfile oder
generierten Dateien und kein anderes Crate. Diese Lieferung behauptet **keine**
Aktivierung, keine Survival-, Live-, Paper- oder Performanceabnahme. Die
Held-Slot-Resync-Verdrahtung in Session und Driver bleibt bei NET-04.2f.

## Basis und Grundlagen

| Gegenstand | Wert |
|---|---|
| Basis | git archive `3f5ed9915c0da1359a5524622e43292de939f4a8` unter `.codex-tmp/sim03_4c-root-base/archive`, 1337 Dateien |
| `base.freeze.json` | `ae4dca9ad99e3519f5317b2141d553544629bf3faf9dd146a0a62be5d4ca4da4`, frisch geprüft: Pfadmenge gleich, 0 Byteabweichungen |
| Freigabe | `SIM-03_4c_SELECTED_SLOT_VIEW_TASK.md` `4247d73c8a2e88ef6a8c5baadd4753ab676ffbb6de4ddd6a2a4a0f904977ec85` (Arbeitsbaum = Basis) |
| Vertrag | `NET-04_2d_NATIVE_DRIVER_ENDPOINT.md` `e0530e0a522b8c2e57a06268206a7721b9ca512b2fd2af3f7f71126fcdbba14f`; Root-Entscheidungen `f9b9c82f737546c8e02d2e6cafaf33f5a06a3e9ffa480ac6bc526dfe8421bb52` |
| Gelesen | Freigabe; NET-04.2e2-Delivery §8.1, Review Runde 1 (A3) und Runde 2; Vertrag §5 Schritt 10/11; Root-Entscheidungen (Auflagen an NET-04.2f); SIM-03.4b-Review (N1); AGENTS, CLAUDE, Sim-INVARIANTS |
| Arbeitskopie | `.codex-tmp/sim03_4c-runtime/work` (Basis + Sim-Overlay), Target-Verzeichnisse `target-dev` (Entwicklung) und frisch `target-fresh` (finale Checks) |
| Gemeinsamer Baum | `server/crates/ferrumc-sim/**` vor der Übernahme bytegleich zur Basis (60 Dateien, 0 Abweichungen, keine Zusatzdatei), danach exakt der Kandidat (61 Dateien, 0 Abweichungen, keine Zusatzdatei) |

## Umgesetztes Verhalten

**Typ `HotbarSlot`** (`src/player.rs:347-371`, über `pub use player::*`
exportiert): ein `Copy`-Newtype mit privatem `u8`. `HotbarSlot::new(index)`
liefert `Some` genau für `0..=8` und `None` darüber, ohne Wrap oder Clamp.
`index()` gibt den Index in derselben Kodierung wie
`NativePlayerSnapshot::selected` zurück, `COUNT = 9`. Die Derives sind `Debug`,
`Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `PartialOrd` und `Ord`. Im Workspace
gab es bisher keinen typisierten Hotbar-Slot. Session nutzt nur den
Fehlernamen `CollectorSessionError::InvalidHotbarSlot`, daher gibt es keinen
Namenskonflikt.

**Accessor `ConnectedOwnedPlayerView::selected() -> HotbarSlot`**
(`src/collector_lifecycle.rs:114-125`): `const fn`, rein lesend, ohne
Allokation. Der Name folgt dem Vorschlag aus NET-04.2e2 §8.1.

**Herkunft, eine einzige Wahrheit.** Der Wert stammt ausschließlich aus dem
privaten Feld `OwnedPlayerInventory::selected` der betrachteten Admission. Der
Konstruktor `connected_owned_player_view` (`src/shard/player_ownership/lifecycle.rs:59-83`)
liest es unter derselben geteilten Shard-Leihe, prüft es einmal mit
`HotbarSlot::new` und legt den Wert in das neue `pub(crate)`-Feld `selected` der
View. Solange die View lebt (`'a`), kann der Shard nicht verändert werden; die
Kopie ist also immer gleich dem Feld. Sonst entsteht kein Zustand,
insbesondere keine Session-seitige oder zweite Sim-Kopie.
`CollectorLifecycleView::live()` reicht den Wert unverändert durch.

- **Schreiber:** Ein angenommenes `select_hotbar` schreibt das Feld beim Apply
  (`apply_deferred`, `shard/player_ownership.rs:2034-2036`; der Counter-Shadow
  spiegelt das in `counters.rs:1376-1384`). Bootstrap, Restore und Rejoin setzen
  es aus dem zugelassenen dauerhaften Snapshot (`publish_player`,
  `shard/player_ownership.rs:1855`).
- **Verhältnis zu `NativePlayerSnapshot::selected`:** Jedes native
  Spieler-Pending (Autosave, Leave-Save, Drop, Pickup, Click) materialisiert den
  Snapshot aus genau diesem Feld (`plan.materialize(..., owned.selected)`), und
  FCPS3 persistiert dieses Byte (`codec.rs:27`, `:57`, `:167` verweigern `≥ 9`).
  Nach bestätigter Completion übernimmt `publish_player` denselben Wert zurück.
  `HotbarSlot::index()` und `NativePlayerSnapshot::selected()` sind deshalb nach
  einem bestätigten Save gleich. Dazwischen zeigt die View den angewandten
  Live-Wert, der Store den zuletzt bestätigten.
- **Nicht angewandt oder verweigert:** Während ein Pending des Spielers offen
  ist, wird `select_hotbar` nach `deferred_selected` zurückgestellt
  (`select_owned_player`, `shard/player_ownership.rs:1961-1989`). Die View zeigt
  dann weiter den angewandten Wert, genau wie bei Position, Look und Mode. Eine
  verweigerte Auswahl (`> 8`: `Malformed("selection")`; detached:
  `Ineligible("detached selection")`; stale oder foreign: `StaleHandle`)
  verändert das Feld nicht. Bei `> 8` erzeugt die bestehende
  `resync_refused_input`-Logik einen `Resync`-Intent für die Admission
  (getestet). Daraus kann NET-04.2f den Held-Slot-Resync speisen. Bei detached
  entsteht laut Code ebenfalls einer (`owned_player_generation` prüft nicht auf
  `connected`); das ist nicht eigens getestet. Stale und foreign erzeugen keinen
  (getestet).
- **Defensive Verweigerung:** Kein Schreiber kann einen Wert über 8 ablegen.
  Wäre es doch einmal so, verweigert die View-Erzeugung typisiert mit
  `Malformed("selected hotbar slot")`, statt zu wrappen oder zu clampen. So
  bleibt der Accessor ohne `unwrap` unfehlbar. Für alle erreichbaren Zustände
  ändert sich das Verhalten von `connected_owned_player_view` nicht (siehe
  Entscheidung 2).
- **Keine View** gibt es weiterhin für detached (`Ineligible("detached player")`),
  stale oder foreign (`StaleHandle`), stopped (`Draining`) und `SnapshotOnly`.

**N1 (SIM-03.4b-Review), nur Kommentare und Doku.** In der Moduldoku
`release.rs:3-10` und in `INVARIANTS.md:532-536` steht jetzt: drei Durchläufe
über höchstens 32 Pendings (Take-Pins, Itemreservierung, übriges Pending),
1 + 32 + 3 × 32 + 256 + 384 = 769 Eintragsbesuche innerhalb der Vertragsgrenze
von 833, dazu ein Schlüsselzugriff je Retry/Reconcile in der Queue.
`logs/release-rs.diff` (16 Zeilen) zeigt nur `//!`-Zeilen; 0 geänderte
Nicht-Kommentarzeilen. Die historische SIM-03.4b-Delivery korrigiert Root
selbst; sie ist unberührt.

**INVARIANTS.** Neuer Abschnitt „Selected hotbar slot in the connected view
(SIM-03.4c)“ (`INVARIANTS.md:544-560`).

## Tests (Memory **und** redb, Counter-Modus, echte Work/Completion)

Neues Modul `src/shard/player_ownership/counters/tests/selected.rs` (389 Zeilen,
eingebunden in `counters/tests.rs`). Die Umgebung ist die bestehende
Counter-`Env` mit Publikations- und Lifecycle-Modus. Jede Admission, jeder Save
und jede Completion läuft als echte Work gegen den echten Store. Geprüft wird
jeweils der View-Wert, das private Feld (`selected`, `deferred_selected`) und
das `selected`-Byte des aktuellen dauerhaften FCPS3-Records.

| Test | Prüft |
|---|---|
| `hotbar_slot_accepts_exactly_zero_to_eight` | `new` für 0–8, `None` für 9, 10, 36, 255; `COUNT`, Ordnung |
| `bootstrap_and_restore_seed_the_view_from_the_durable_record_{memory,redb}` | Bootstrap erst nach echter Completion zugelassen, dann 0/0. Restore von Records mit 8, 0 und 5 liefert View = Feld = Store; die Bootstrap-Admission bleibt unberührt |
| `applied_refused_and_deferred_selection_{memory,redb}` | Angewandt 5→7: `PlayerChanged`, View 7, Store 5; nach Autosave Store 7. Verweigert 9: `Malformed("selection")`, `Resync` für die Admission, Wert 7. Nicht angewandt: Auswahl 2 und dann 3 hinter einem offenen Autosave, View 7, Feld `(7, Some(3))`; nach der echten Completion Store 7 (erfasster Wert), View 3; nach weiterem Save Store 3 |
| `detach_and_rejoin_selection_{memory,redb}` | Auswahl 4 angewandt, Disconnect, danach keine View (`Ineligible("detached player")`), `live()` ist `None`; detached-Auswahl 6 verweigert, Feld bleibt 4. Leave-Save persistiert 4, Rejoin liefert neues Handle mit 4 = 4 = 4, `collector_lifecycle_view(q).live().selected()` = 4. Altes Handle: `StaleHandle`, `live()` `None`, Auswahl 8 verweigert ohne Resync, neuer Wert unberührt. Neue Auswahl 0 wird angewandt |
| `foreign_handle_produces_no_view_{memory,redb}` | Echtes Handle eines zweiten Owners mit derselben PlayerId: View und Lifecycle-View `StaleHandle`, `matches_owned_player` false. Auswahl 7 wird beim Apply mit `StaleHandle` ohne Resync verweigert; beide Owner behalten 2 bzw. 6 |
| `out_of_range_owner_field_refuses_view_creation` | White-Box: Feld künstlich 9 ergibt `Malformed("selected hotbar slot")` und `live()` `None`; zurück auf 3 ergibt wieder 3 |

**Mutantenprobe** (separate Kopie `probe-work`, eigenes Target, danach
zurückgesetzt; `logs/dev/mutants.log`):

- Die View zeigt den zurückgestellten statt des angewandten Werts: 2 von 10
  Tests schlagen fehl (deferred, Memory und redb).
- Konstant 0: 9 von 10 Tests schlagen fehl.

Beide Mutanten werden also erkannt.

## Kandidat, Manifest und Checks

| Gegenstand | Wert |
|---|---|
| Kandidat | 61 Sim-Dateien; geändert 6 (`INVARIANTS.md`, `src/collector_lifecycle.rs`, `src/player.rs`, `src/shard/player_ownership/counters/tests.rs`, `src/shard/player_ownership/lifecycle.rs`, `src/shard/player_ownership/release.rs`), neu 1 (`counters/tests/selected.rs`), entfernt 0 |
| `candidate.manifest.json` | `2df6e217646fda60a9be894a18a8d49e96a4dff9feec160cac12af0cbba10020`, **vor** den Checks abgelegt; readonly Kopie `candidate-src/` und `candidate.report.md` |
| Diff | `logs/sim-diff.patch` `6bf5a1162542e3dacc275013dbbcc63e5f91d734b26fdb1e946a8d2574ccaa3a` (594 Zeilen) |
| Toolchain | cargo 1.99.0 (5f94df478 2026-08-27), rustc 1.99.0 (b940084d7 2026-09-28), `--jobs 2 --locked`, frisches `CARGO_TARGET_DIR=target-fresh` (vorher nicht vorhanden, protokolliert) |
| Quellstand während der Checks | Sim-Crate in `work/` schreibgeschützt; Abgleich vor und nach den Checks: Kandidat 0 Abweichungen, außerhalb der Sim 1277 Basisdateien bytegleich |

Die Checks liefen über `tools/checks.sh`, detached, mit Logs unter
`logs/checks/`; ohne Source-Änderung, in dieser Reihenfolge:

| Schritt | Ergebnis |
|---|---|
| `cargo test -p ferrumc-sim` (Debug, `--no-fail-fast`) | exit 0; 11 Läufe, **404** bestanden, 0 fehlgeschlagen, 0 ignoriert (58 s); Basis laut SIM-03.4b-Review 394, also +10 neue |
| `cargo test -p ferrumc-sim --release` | exit 0; 11 Läufe, **404** / 0 / 0 (180 s) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, 0 Warnungen (65 s) |
| `cargo fmt --all --check` | exit 0, leere Ausgabe |
| `RUSTDOCFLAGS=-Dwarnings cargo doc -p ferrumc-sim --no-deps` | exit 0, 0 Warnungen |
| `cargo build --workspace` | exit 0, 0 Warnungen (57 s) |
| `cargo test -p ferrumc-session` | exit 0; 10 Läufe, 244 / 0 / 0 |
| `cargo test -p ferrumc-testkit` | exit 0; 8 Läufe, 100 / 0 / 0 |
| `cargo test -p ferrumc-bench` | exit 0; 4 Läufe, 5 / 0 / 0 |
| `cargo test -p ferrumc-app --no-fail-fast` | exit 0; 40 Läufe, 375 bestanden, 0 fehlgeschlagen, 1 ignoriert (vorbestehend: `genuine_paper_api_plugin_…`, braucht explizite Java- und JAR-Pfade); `multiplayer_visibility` 2 / 0 / 0 im Gesamtlauf, also kein Einzellauf nötig |

**Nebenprodukte (.so).** Die App-Tests erzeugen unter
`work/.codex-tmp/p57-plugin-testhost-2579937-artifacts/` die Dateien
`libplugin_testhost_fixture.so` und `librenamed_fixture.so` (je 8 450 024 Bytes,
SHA-256 `cded17d9f4a85d374bf4328802e2effe539ccf3eec610cd519cc0afa59e3e1af`) sowie
das leere Verzeichnis `work/.codex-tmp/p52-abi-invalid-input`. Sie liegen nur in
der privaten Arbeitskopie, außerhalb jedes Quellpfads, und stehen in
`verify-after` als einzige Zusatzdateien außerhalb der Sim. Es gibt keine
Basisabweichung.

**Übernahme.** Vor dem Kopieren war `server/crates/ferrumc-sim/**` im
Arbeitsbaum bytegleich zur Basis (60/60, keine Zusatzdatei). Kopiert wurden
genau die 7 geänderten oder neuen Dateien aus `candidate-src/`. Danach stimmt
der Arbeitsbaum mit dem Kandidatenmanifest überein (61 Dateien, 0 Abweichungen).
Außerhalb der Sim habe ich nur diese Delivery angelegt.

## Entscheidungen und Fragen an Root

1. **Neuer öffentlicher Typ `HotbarSlot`.** „Typisierter Slot-Wert“ verlangt
   einen Typ, und keiner existierte. Ich habe ihn minimal gehalten (`new`,
   `index`, `COUNT`), ohne Window-Slot-Helfer. Name und API kann Root für
   NET-04.2f festlegen; die Änderung wäre lokal.
2. **Prüfung bei der View-Erzeugung statt Feldtyp-Umbau.** Die Alternative wäre
   gewesen, `OwnedPlayerInventory::selected` selbst als `HotbarSlot` zu führen.
   Dann bräuchte `publish_player` eine fehlbare Umwandlung, und die liefe in
   `publish_confirmed` erst nach `remove_pending`. Das hätte eine Teilmutation
   im Fehlerfall bedeutet und den Counter-Shadow-Gleichlauf berührt. Ich habe
   daher die einmalige Prüfung im lesenden Konstruktor gewählt. Sie fügt
   `connected_owned_player_view` eine unerreichbare `Malformed`-Verweigerung
   hinzu; `CollectorLifecycleView::live()` zeigt sie über das bestehende `.ok()`
   als `None`. Für alle erreichbaren Zustände bleibt das Verhalten gleich.
3. **Semantik „angewandt“.** Der Accessor liefert den angewandten Wert und nie
   `deferred_selected`, konsistent mit `position`, `look` und `game_mode` derselben
   View und mit dem, was der nächste Save materialisiert. Für den Held-Slot-Resync
   nach einer Verweigerung ist das der richtige Wert (Test „deferred“).
4. **Kein Vertragskonflikt gefunden**; keine Rückfrage war nötig.

## Nicht Teil dieser Lieferung

Nicht enthalten sind: Session- und Driver-Verdrahtung des Held-Slot-Resyncs
und das Join-Kit (NET-04.2f), die Isolation quarantänierter Pendings (R3-A5),
Live-Aktivierung, Survival-, Plugin- und Performanceaussagen, die Korrektur der
historischen SIM-03.4b-Delivery (Root) sowie Git, Board und Integration.

## Artefakte

Alle Artefakte liegen unter `.codex-tmp/sim03_4c-runtime/`:

- `candidate-src/`, `candidate.manifest.json`, `candidate.report.md` (readonly)
- `tools/verify.py` (`cc53ce5b…`), `tools/checks.sh` (`62f22920…`)
- `logs/checks/`: Step-Logs, `summary.log` `c774e0fb5aaae4e68cc1f7aca0584f2342c9108d904b3f394e28def3ce67b53d`, `verify-before/after.json`
- `logs/sim-diff.patch`, `logs/release-rs.diff`
  (`6a3c51e65947e1d2ec6adcb6e5459c60e02a82429b871f7d94babeb9bb99ecea`),
  `logs/dev/` (Entwicklungsabgleich, Mutanten)
- `work/`, `probe-work/`, `target-dev/`, `target-fresh/`
- `delivery.freeze.json`: Hashes von Kandidat, Logs, Tools und dieser Delivery,
  nach Fertigstellung der Delivery erzeugt
