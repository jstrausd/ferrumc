# SIM-03.4c — Gewählter Hotbar-Slot in der verbundenen Spieler-View: unabhängiges Runtime-Review

**Verdikt: CLEAN_RUNTIME.** Es gibt keinen blockierenden, hohen, mittleren oder
niedrigen Befund, nur vier Hinweise (H1–H4) ohne Änderungsbedarf am Kandidaten.

Reviewer ist ein unabhängiger Review-Subagent, 12.10.2026; den Kandidaten hat er
nicht erstellt. Geprüft wurde die Kombination aus dem aktuellen `main`
`d97fcc5c49148d1f0713022a9a603119da1d44ef` (mit NET-04.2e2 und der
GEN-05.9-World-Runtime) und den 61 Sim-Dateien des Kandidaten. Es gab keine
Änderung an Kandidat, Evidenz oder Produktion, kein Git und keine weiteren
Agents. Geschrieben wurde nur unter `.codex-tmp/sim03_4c-runtime-root-review/`
(außer `combined/`) und dieses Dokument.

## Prüfbasis

| Gegenstand | Wert |
|---|---|
| `combined/` | readonly, 1388 Dateien; `combined.freeze.json` `c8a4d18f2f8db64df775e5f1102a53bc9d66dc3f3194a1614dba539704dfcea0`, vor und nach allen Läufen 0 Abweichungen, 0 Zusatzdateien |
| Kandidat | `candidate.manifest.json` `2df6e217646fda60a9be894a18a8d49e96a4dff9feec160cac12af0cbba10020`, 61 Dateien; gegen `combined/`, `work/` und den Arbeitsbaum je 0 Abweichungen (vor und nach den Checks) |
| Delivery | `SIM-03_4c_SELECTED_SLOT_VIEW_DELIVERY.md` `6c635ee942e48df4b854e68f65e08e62d9a8fe25619c546cfc4b55d310c4e7a1`; `delivery.freeze.json` `50a9b1a5f46d8a618255add0adfe556842004b9e778f77efa08b9f5fedf1e155`, alle 91 Dateihashes stimmen |
| Arbeitskopie | `work/` = Kopie von `combined/`; frisches `CARGO_TARGET_DIR=target-fresh` (vorher nicht vorhanden), `CARGO_INCREMENTAL=0` |
| Probe-Kopie | `probe-work/` = `combined/` + `counters/tests/review_probes.rs` + eine `mod`-Zeile; eigenes Target `target-probe` |
| Toolchain | cargo 1.99.0 (5f94df478 2026-08-27), `--jobs 2 --locked`, ausgeführt mit `setsid nohup` über `run-checks.sh` |

## Teil A — Ausführung

### A1 Frische Checks auf der Kombination

| Schritt | Ergebnis |
|---|---|
| `cargo fmt --all --check` | exit 0, leere Ausgabe |
| `cargo test -p ferrumc-sim` (Debug) | exit 0; 11 Läufe, **404** / 0 / 0 (unittests 266, inkl. der 10 neuen) |
| `cargo test -p ferrumc-sim --release` | exit 0; 11 Läufe, **404** / 0 / 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-sim --no-deps` | exit 0, 0 Warnungen |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, 0 Warnungen |
| `cargo build --workspace` | exit 0, 0 Warnungen |
| `cargo test -p ferrumc-session` | exit 0; 10 Läufe, 244 / 0 / 0 (inkl. `native_lifecycle`, `native_borrow`, `native_endpoint` mit NET-04.2e2-Revalidierung und -Probe) |
| `cargo test -p ferrumc-testkit` | exit 0; 8 Läufe, 100 / 0 / 0 |
| `cargo test -p ferrumc-bench` | exit 0; 4 Läufe, 5 / 0 / 0 |
| `cargo test -p ferrumc-app --no-fail-fast` | exit 0; 40 Läufe, 375 bestanden, 0 fehlgeschlagen, 1 ignoriert (vorbestehend: `genuine_paper_api_plugin_…`, braucht explizite Java- und JAR-Pfade); `multiplayer_visibility` im Gesamtlauf 2 / 0 / 0, daher kein Einzellauf nötig |

Die Zahlen stimmen mit der Delivery überein, obwohl sie dort auf `3f5ed991`
erhoben wurden. Zwischen `3f5ed991` und `d97fcc5c` liegen nur die zwei
World-Commits `ffd7009b` und `d97fcc5c`.

Danach waren `combined/` (1388/1388) und die 61 Kandidatendateien in
`combined/`, `work/` und im Arbeitsbaum unverändert. In `work/` sind alle 1388
Dateien bytegleich. Als Zusatzdateien liegen dort nur die App-Test-Nebenprodukte
`work/.codex-tmp/p57-plugin-testhost-2616078-artifacts/libplugin_testhost_fixture.so`
und `librenamed_fixture.so` (je 8 221 176 Bytes, SHA-256 `8b3dcc05…`) sowie das
Verzeichnis `work/.codex-tmp/p52-abi-invalid-input/`, alles außerhalb jedes
Quellpfads.

### A2 Eigene Probes (`probes/review_probes.rs`, 8 Tests, alle grün)

Die Probes laufen in der Counter-`Env` mit echter Work und Completion gegen den
echten Store. P1, P2 und P4 laufen je auf Memory **und** redb.

| Probe | Ergebnis |
|---|---|
| **P1** `select_hotbar(6)`, `disconnect` und Leave-Autosave in **einer** Boundary, danach Completion und Rejoin | keine Refusal; Feld `(6, None, detached)`; persistiertes FCPS3-Byte **roh** an Offset 38 = 6, dekodiert = 6; die Rejoin-View und `collector_lifecycle_view(q).live()` liefern `HotbarSlot(6)` |
| **P2** Auswahl 5 hinter einem offenen Autosave zurückgestellt, dann `disconnect` und erst danach die Completion | Disconnect angenommen, Feld `(1, Some(5), detached)`. Die Completion bestätigt die erfasste 1 und wendet danach die Zurückstellung an: `(5, None, detached)`. Keine View; `detached_owned_player_status().dirty()` = true (Release bleibt blockiert). Der Leave-Save persistiert 5, Rejoin zeigt 5. Siehe H1 |
| **P3** verweigerte Auswahl eines detached Spielers (6 und 9) | `Ineligible("detached selection")` bzw. `Malformed("selection")`, jeweils **mit** `Resync` und der unveränderten Generation; Feld bleibt 2. Damit ist die nur aus dem Code abgeleitete Aussage der Delivery belegt |
| **P4** gültiger Record, Byte 38 von 4 auf 9 gesetzt (am Encoder vorbei), per CAS gespeichert, dann Restore | `try_from_record` → `Malformed("header admission")`; Restore verweigert mit demselben Fehler, kein `PlayerAdmitted`, kein `OwnedPlayerInventory` |
| **P5** in einer Boundary `select(2)`, `select(9)`, `select(255)` | 2 angewandt, zwei Refusals mit je einem `Resync`; dessen Generation ist die nach dem Apply, die View zeigt 2 |

### A3 Mutanten (Probe-Kopie, danach zurückgesetzt und per `cmp` gegen `combined/` bestätigt)

| Mutant | Erkannt durch |
|---|---|
| M1: Clamp `HotbarSlot::new(owned.selected.min(8))` in der View-Erzeugung | `out_of_range_owner_field_refuses_view_creation` (1 von 97 im Filter `counters::tests::`) |
| M2: `>8`-Prüfung in `select_owned_player` entfernt | `applied_refused_and_deferred_selection_{memory,redb}` |

Der erste M1-Lauf scheiterte an der CLI, weil zwei Testfilter angegeben waren
(rc=1, kein Testlauf). Er wurde mit einem Filter wiederholt; beide Läufe stehen
in `logs/mutants.log`. Die Mutanten des Erstellers (deferred statt angewandt,
konstant 0) sind in seinem `logs/dev/mutants.log` nachvollziehbar.

### A4 Forbidden-Claims-Scan

`tools/claims_scan.py` lädt `scripts/check-forbidden-claims.py` aus `combined/`
und nutzt dessen `markdown_blocks`, `rustdoc_blocks` und `scan`, jeweils mit und
ohne `force_plugin`.

- Delivery: 75 Blöcke, 0 Treffer.
- Rustdoc der geänderten Rust-Dateien: 0 Treffer.
- `INVARIANTS.md`: 2 Treffer der Regel gegen unqualifizierte Verträglichkeitsaussagen
  (Regelname in `logs/claims-scan.json`), Zeilen 87 und 321.
  Beide bestehen auf `main` unverändert, der neue SIM-03.4c-Abschnitt und die
  N1-Zeilen tragen 0 bei. Der CI-Scan (`--check-index`) erfasst
  `server/crates/*/INVARIANTS.md` ohnehin nicht. Siehe H4.

## Teil B — Source-Review

### B1 `HotbarSlot` (`player.rs:347-371`)

- Newtype mit privatem `u8`. Ohne Zugriff auf das Feld gibt es keinen
  Konstruktor außer `HotbarSlot::new`, und der liefert `Some` genau für
  `index < 9`. Kein Wrapping, kein Clamping, kein `From<u8>`, kein `Default`.
  Der Test deckt 0–8, 9, 10, 36 und 255 ab.
- Rustdoc ist an Typ, `COUNT`, `new` und `index` vorhanden. Der Hinweis auf das
  Window-0-Slot `36 + index` stimmt mit `plan.rs:96` überein. Der Typ ist über
  `pub use player::*` exportiert.

### B2 `selected()` (`collector_lifecycle.rs:114-125`)

`const fn` gibt eine `Copy`-Kopie zurück, rein lesend und ohne Allokation. Der
Wert entsteht einmal in `connected_owned_player_view` aus `owned.selected`,
unter derselben geteilten Leihe `'a`, die `player`/`state` hält. Solange die
View lebt, kann der Shard nicht mutiert werden. `PlayerState` und
`OwnedPlayerInventory` haben keine innere Mutabilität. `live()` kopiert den
Wert der gespeicherten View derselben Leihe. Es gibt nur diese zwei
Konstruktionsstellen von `ConnectedOwnedPlayerView`; der Typ hat keinen
öffentlichen Konstruktor (`pub(crate)`-Felder, `compile_fail`-Doctest gegen
`clone`).

### B3 Eine einzige Quelle und das Verhältnis zu `NativePlayerSnapshot::selected` und FCPS3

`OwnedPlayerInventory` wird nur in `publish_player` gebaut
(`player_ownership.rs:1852`). Vollständig erfasste Schreibpfade auf `selected`:

1. `publish_player` (`:1855`) übernimmt `snapshot.selected`. Aufrufer:
   - `restore_owned_player` (Restore/Rejoin, `:820`): Der Snapshot stammt aus
     `try_from_record`. Der Decoder verweigert `selected >= 9` vor jeder
     Allokation (`codec.rs:167`); belegt durch P4.
   - `publish_confirmed` (`:1697`): `pending.after_player`. Alle vier
     Candidate-Pfade (Bootstrap `:451` mit `empty_survival` = 0, Autosave,
     Leave-Save und Click `:1060`, Drop `:1130`, Pickup `:1240`) rufen vor dem
     Anlegen des Pendings `try_to_record()` auf, also `validate`
     (`codec.rs:27`) und `encode_parts` (`:57`). Ein Snapshot mit Wert über 8
     kann kein Pending werden. Der Counter-Pfad (`counters.rs:1853`) setzt
     `after_player: None`.
2. `apply_deferred` (`:2034-2036`) übernimmt `deferred_selected`. Das wird nur in
   `select_owned_player` (`:1987`) gesetzt, und zwar nach der Prüfung `> 8`
   (`:1976`), oder in `publish_player` vom selben Inventar übernommen
   (`:1874`).
3. `counters.rs:1383` schreibt nur in den `ShadowPlayer` der Vorplanung (ebenfalls
   hinter `> 8`), nicht ins Inventar.

`NativePlayerSnapshot::new` ist öffentlich und validiert nicht. Die
Validierung liegt jedoch an jeder Grenze zum Inventar (Decode, beziehungsweise
Encode vor dem Pending). Die defensive Ablehnung
`Malformed("selected hotbar slot")` ist daher für Produktion unerreichbar. M2
zeigt, dass die Tests den Fall bemerken würden, wenn die Select-Prüfung
entfiele. Das Verhältnis „View = angewandter Live-Wert, Store = zuletzt
bestätigter Wert, gleich nach bestätigtem Save“ ist korrekt dokumentiert und
durch die Tests sowie P1 und P2 (Rohbyte) belegt.

**Root-Entscheidung (Prüfung bei der View-Erzeugung).** Der Zustand ist von
keinem Schreiber erzeugbar (siehe oben). Die zusätzliche Ablehnung bricht
keinen Aufrufer:

- Produktion in ferrumc-session reicht die View nur als
  `&ConnectedOwnedPlayerView` durch (`router/native.rs`, `audience.rs`,
  `retention.rs`) oder erhält sie als `Option` (`NativeRevalidationProbe::submit`).
- Die Session-Tests nutzen `.ok()`, `.unwrap()` oder
  `Err(PlayerOwnershipError::Ineligible(_))` für detached
  (`native_lane.rs:571`, `native_endpoint.rs:250`). `PlayerOwnershipError` wird
  nirgends erschöpfend auf das Ergebnis von `connected_owned_player_view`
  gematcht.
- app, testkit und bench verwenden die View nicht. Die Session-Suite inklusive
  NET-04.2e2-Revalidierung ist grün (244/0/0). Siehe H2.

Die Umstellung der Detached-Prüfung von `is_none_or(|o| !o.connected)` auf
`match Some(owned) if owned.connected` ist semantisch gleich.

### B4 Zurückgestellte und verweigerte Auswahl

Zurückgestellte Werte (`deferred_selected`) liest die View nie. Das ist dieselbe
Semantik wie bei `position`, `look` und `game_mode`, die direkt aus
`PlayerState` kommen und ebenfalls erst in `apply_deferred` geschrieben werden.
Verweigerte Auswahlen ändern das Feld nicht: `> 8`, detached, stale und foreign
sind durch Tests belegt, detached zusätzlich durch P3. Den Resync für `> 8` und
detached liefert die bestehende `resync_refused_input`-Logik. Bei stale oder
foreign entsteht keiner, weil `owned_player_generation` scheitert; das belegen
die Tests.

### B5 Keine View für detached, stale, foreign

Die Reihenfolge ist: `stopped` (Draining), dann `actual_player` (Owner und
Admission, also StaleHandle bzw. SnapshotOnly), dann `connected`, erst danach
die Slot-Prüfung. Die neue Prüfung liegt hinter allen bestehenden Gates und
verändert deren Fehler nicht. Tests: detached (`Ineligible("detached
player")`), stale nach Rejoin, foreign (zweiter Owner mit derselben PlayerId),
jeweils auch über `collector_lifecycle_view(..).live()`.

### B6 N1 (`release.rs`)

`logs/release-rs.diff` (16 Zeilen) ändert nur `//!`-Zeilen. Gegen `main`
stimmen alle Nicht-Kommentarzeilen überein. `check_detached_release` iteriert
die Pending-Map genau dreimal: Take-Pins, Itemreservierung und Pending, letzteres
kurzgeschlossen über `owned.pending`. Rechnung: 1 + 32 + 3 × 32 + 256 + 384 =
**769 ≤ 833**. Der Vertrag (NET-04_2d §7.5 A7) rechnet 128 Spieler + 384 + 32
+ 32 + 256 + 1 = 833. Die Implementierung besucht nur einen Map-Eintrag statt
128 Spielern, dafür 3 × 32 Pendings. Selbst wenn man die je Queue-Input
höchstens eine Schlüsselsuche für `Retry`/`Reconcile` als Besuch zählt, sind es
769 + 32 = 801 ≤ 833. Moduldoku und `INVARIANTS.md:532-536` sind korrekt.

### B7 Tests des Kandidaten gegen die Freigabe

| Freigabe | Abdeckung |
|---|---|
| Memory **und** redb | jede Szenariofunktion zweifach. Der White-Box-Wächter läuft nur auf Memory; er ist storageunabhängig |
| Counter-Modus | `Env::new` initialisiert `initialize_owned_item_counters`, Publication- und Lifecycle-Modus |
| echte Work/Completion | `finish` führt die echte Work gegen den Store aus und reicht die Completion als Input ein; Bootstrap wird erst nach Completion zugelassen |
| nach Bootstrap und Restore | 0 bzw. 8/0/5, View = Feld = Store |
| nach angewandtem `select_hotbar` | 5→7, `PlayerChanged`, Store erst nach Save |
| verweigert / noch nicht angewandt | 9 mit Resync; 2 und 3 zurückgestellt, nach Completion 3 |
| nach Rejoin | neues Handle, 4 = 4 = 4, `live()` |
| keine View für detached/stale/foreign | ja, inklusive Lifecycle-View und Refusal ohne Resync für stale/foreign |

Die Temp-Pfade für redb sind je Test und Prozess eindeutig und werden per Drop
entfernt.

### B8 Delivery

- Alle Zeilenangaben stimmen auf der Kombination (`player.rs:347-371`,
  `collector_lifecycle.rs:114-125`, `lifecycle.rs:59-83`,
  `player_ownership.rs:1855/1961-1989/2034-2036`, `counters.rs:1376-1384`,
  `codec.rs:27/57/167`, `release.rs:3-10`, `INVARIANTS.md:532-536/544-560`).
- `selected.rs` hat 389 Zeilen. Die Hashes stimmen: `sim-diff.patch` `6bf5a116…`
  (594 Zeilen), `release-rs.diff` `6a3c51e6…` (16 Zeilen), Freigabe `4247d73c…`,
  Vertrag `e0530e0a…`, Root-Entscheidungen `f9b9c82f…`, `base.freeze.json`
  `ae4dca9a…`, `summary.log` `c774e0fb…`, `checks.sh` `62f22920…`, `verify.py`
  `cc53ce5b…`.
- Die Testzahlen sind auf der Kombination reproduziert (siehe A1).
- Die Nicht-Behauptungen sind vollständig und zutreffend: keine Aktivierung,
  keine Live-, Survival-, Paper- oder Performanceabnahme, Session- und
  Driver-Verdrahtung bleiben bei NET-04.2f, R3-A5 ausgenommen. Die Aussage
  „detached erzeugt laut Code einen Resync, nicht eigens getestet“ ist korrekt
  gekennzeichnet und jetzt durch P3 belegt.

## Befunde nach Schwere

Keine blockierenden, hohen, mittleren oder niedrigen Befunde.

**H1 (Hinweis) — Zurückstellung über einen Disconnect hinweg.** Eine
angenommene, aber zurückgestellte Auswahl wird beim Abschluss des blockierenden
Pendings auch dann angewandt, wenn der Spieler inzwischen detached ist (P2).
Das entspricht dem vorbestehenden Verhalten von Position, Look und Mode in
`apply_deferred`. Es ist weder sichtbar (keine View) noch verloren: Der Status
ist dirty, Release bleibt blockiert, der Leave-Save persistiert den Wert, und
Rejoin zeigt ihn. Die Delivery beschreibt den Fall nicht; für NET-04.2f ist er
relevant, falls der Held-Slot-Resync nach Rejoin geplant wird.

**H2 (Hinweis) — Revalidierung bildet jeden Fehler auf „Detached“ ab.**
`NativeRevalidationProbe::submit(view.ok())` würde die neue, unerreichbare
`Malformed`-Ablehnung als `Detached` werten. Heute ohne Wirkung. NET-04.2f
sollte den Held-Slot über die View lesen und keine eigene Kopie halten.

**H3 (Hinweis) — Formulierung „every native save materializes … from it“.**
Das Bootstrap-Pending materialisiert `selected = 0` über `empty_survival`, nicht
aus dem Feld, denn vor der Admission existiert keines. Danach seedet es das
Feld. Sachlich korrekt, nur unscharf formuliert; keine Änderung nötig.

**H4 (Hinweis) — Vorbestehende Scan-Treffer in Sim-`INVARIANTS.md`.**
Die Treffer in den Zeilen 87 und 321 (Regelname in `logs/claims-scan.json`)
bestehen auf `main`. Sie
stammen nicht von diesem Kandidaten und liegen außerhalb des CI-Scanbereichs.

## Artefakte

Unter `.codex-tmp/sim03_4c-runtime-root-review/`:

- `final-assessment.json`
- `run-checks.sh`, `verify.py`, `tools/claims_scan.py`, `tools/assess.py`
- `logs/` mit `status.txt`, je einem Log pro Check, `probes-run1.log`,
  `mutants.log`, `claims-scan.json` und `INVARIANTS.main.md`
- `probes/review_probes.rs`
- `work/`, `probe-work/`, `target-fresh/`, `target-probe/`
