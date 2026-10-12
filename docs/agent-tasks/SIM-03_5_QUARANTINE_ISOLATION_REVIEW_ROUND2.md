# SIM-03.5 — Review Runde 2 des Vertrags „quarantinierte Item-Pendings isolieren“

Unabhängiges Review, 12.10.2026. Der Reviewer hat weder den Kandidaten noch
das Review der Runde 1 erstellt. Die Produktion blieb readonly. Alle Proben
liefen ausschließlich in privaten Archivkopien unter
`.codex-tmp/sim03_5-root-review-r2/`, mit eigenem Target-Verzeichnis. Es gab
kein Git, keine Agents und keine Änderung an Brief, Evidenz, früherem Review
oder Produktion.

Dieses Review nimmt weder Aktivierung noch Survival, Paper-Parität, Plugins oder
Performance ab.

## Verdikt: FINDINGS (ein niedriger, rein textlicher Befund)

- **A1–A6 sind behoben** (Einstufung in §2).
  - Der Prototyp wirkt nur im Counter-Modus.
  - Im Legacy-Modus ist das Verhalten zeilengleich mit der Basis.
  - Außerhalb einer Abschirmung bleibt die Verweigerungsreihenfolge exakt gleich.
- **Der Nachtrag ist mechanisch übernehmbar:**
  - Mein eigenes, unabhängiges Skript erzeugt aus den Zeilenangaben des Briefs
    byte-gleiche Dokumente wie die Prüfung des Erstellers.
  - Inhaltlich fand ich keinen Widerspruch und keine verwaisten Verweise.
- **Neu, niedrig (N1):** Der neue DUR-04.2 §11 Punkt 3 behauptet, im
  quarantinierenden Boundary verweigere die Reservation auch den **Pickup** mit
  `Reserved`.
  - Tatsächlich wird der Pickup des Items dort immer mit
    `Ineligible("quarantined item")` verweigert, nur die Save mit `Reserved`.
  - Die Disposition (Verweigerung, 0 Work) und die Gleichheit von Plan und Apply
    sind nicht betroffen; falsch ist nur die normative Aussage.
- **Empfehlung an Root:**
  - Keine volle Runde 3.
  - Den Nachtrag mit der wörtlichen Korrektur aus §5 (drei Zeilen im Block
    `D2-S11`) übernehmen; danach ist er aus meiner Sicht sauber.
  - Runtime-Freigabe für V2 im Scope `ferrumc-sim/**` einschließlich
    `INVARIANTS.md`, mit den Pflichtzusätzen aus §6.

## 1. Kandidat, Basis und eigene Läufe

| Gegenstand | Pin / Prüfung |
|---|---|
| Brief | `docs/agent-tasks/SIM-03_5_QUARANTINE_ISOLATION.md`, SHA-256 `a7da63e0…26d55`, frisch geprüft |
| Freeze | `.codex-tmp/sim03_5/round2/evidence.freeze.json`, SHA-256 `b38e6be4…e06223`; alle 64 Einträge frisch identisch; `sketch_r2_sha256` stimmt |
| Basis | Archiv f1e7511c, `base.freeze.json` `b715025e…4356`; 1390/1390 Dateien frisch identisch |
| Toolchain | `~/.cargo/bin` 1.99.0, `--locked --offline`, `--jobs 2`, eigenes Target (per Reflink aus dem Runde-1-Review geseedet) |

**Eigene Kopien:**

- `base/`: Basis plus Probe-Module;
- `proto/`: Basis plus `round2/make_proto_r2.py` (14 Ersetzungen) plus
  Probe-Module. Mein Diff ist **byte-gleich** mit
  `v2-reference-sketch-r2.diff` (+83/−6);
- `proto-clean/`: wie `proto/`, ohne Probe-Module, für Clippy und die vollen
  Suiten;
- `proto-r1/`: Basis plus Runde-1-Skizze (`make_proto.py`), nur als
  Gegenkontrolle.

Mein Runner `run_probe.py` ist eine Kopie des Ersteller-Runners mit eigenem
Ausgabeverzeichnis. Er prüft jede Kopie gegen `base.freeze.json`; außer
Probe-Dateien, `mod`-Zeilen und den zwei erlaubten Prototyp-Dateien weicht
nichts ab.

| Lauf (Runde-2-Prototyp) | Ergebnis |
|---|---|
| `cargo fmt --all --check` | grün |
| `cargo clippy -p ferrumc-sim --all-targets -D warnings` nach `cargo clean -p ferrumc-sim` | sauber; das Log zeigt `Checking ferrumc-sim`, also frisch |
| `cargo test -p ferrumc-sim --all-targets` plus `--doc` | 404/404: Lib 266, Integration 127, Doctests 11 |
| `cargo test -p ferrumc-session` | 244/244 |
| `cargo test -p ferrumc-testkit` | 100/100 |
| `cargo test -p ferrumc-app --no-fail-fast` | 375 bestanden, 0 fehlgeschlagen, 1 ignoriert (Java/Maven-Fixture), 40 Testbinaries, ohne Wiederholungslauf |
| alle `counters::tests` mit T1–T13, T14, R1–R4 und meinen Proben A–C | 111/111 in debug und 111/111 in release; alle Messzeilen in debug und release identisch |
| T1–T13 | 35/35 Messzeilen identisch mit der Runde-1-Evidenz des Erstellers |
| Messzeilen des Erstellers, Runde 2 | 45/45 identisch, debug und release |
| T14 auf der Basis | grün; Messzeilen Basis = Runde 2 |
| R1 | Runde 2 = Basis (Reservierung `true`, `Reserved`, 0 Work) |
| R2–R4 | zeilengleich mit dem Prototyp des Runde-1-Reviewers |
| R2/R3 auf der Basis | brechen beim Restore eines zweiten Items ab; das ist B4 wie in Runde 1 und erwartet |

## 2. Einstufung der Runde-1-Befunde

| Befund | Einstufung | Beleg |
|---|---|---|
| **A1** Legacy-Freigabe der Reservierung | **behoben** | §3 |
| **A2** `INVARIANTS.md` | **behoben** | §4.1 |
| **A3** Nachtrag DUR-04.2/DUR-04.1 | **behoben**; dabei neuer Textfehler N1 im neuen §11 | §4.2, §5 |
| **A4** Ehrlichkeit | **behoben** | §4.4 |
| **A5** Kapazität | **behoben** | §4.3 |
| **A6** O1-Begründung | **behoben** | §4.4 |

## 3. A1: nur Counter-Modus, Legacy unverändert

### 3.1 Jeder V2-Pfad und sein Modus

Ich habe alle Aufrufer im Prototyp nachgeschlagen (`grep` auf alle neuen und
geänderten Funktionen, ohne Tests).

| V2-Pfad | Modus-Absicherung (Prototyp) |
|---|---|
| Plan: Bindungsschleife mit `fences_item` → `resident = false` | `plan_counter_boundary` hat genau einen Aufrufer, `run_tick_with_player_ownership_at` (`counters.rs:185`). Dieser verlangt `item_counters` (`162-166`); zusätzlich verlangt der Plan selbst `item_counters` (`508-512`) |
| Step | liegt im Plan (`733-738`), also derselbe Schutz |
| Plan: `reserved` ohne `Quarantined`, `EntitySave`-Arm, `Retry/Reconcile`-Arm | in `plan_counter_actions`, einziger Aufrufer der Plan (`719`) |
| Reservierungsfreigabe | `release_quarantined_reservation` kehrt ohne `item_counters` zurück (`446-448`). Beide Aufrufer (`231`, `295`) sind damit abgedeckt, auch die modusunabhängige Recovery |
| Save | `plan_counter_save` verlangt `item_counters` (`1872-1876`) |
| Save-Auswahl | `select_counter_save` kehrt ohne `item_counters` zurück |
| Pickup | `require_unfenced_item` → `item_fenced`, das `item_counters.is_some()` selbst prüft (`459`) |
| Reconcile-Verweigerung | `dispatch_retained` prüft `item_counters.is_some()` (`player_ownership.rs:1402-1407`) |

**Ergebnis:** Jeder V2-Pfad wirkt ausschließlich im Counter-Modus.

### 3.2 Läufe im Legacy-Modus

- **T14:** grün auf Basis und Runde 2, Messzeilen identisch. Auf der
  Runde-1-Skizze schlägt der Test erwartungsgemäß fehl.
- **Eigene R1 (Probe A, `probes/r2rev.rs`):** Ich habe R1 verschärft.
  - Ablauf: Legacy-Owner, echte Superseded-Pickup-Quarantäne, danach **dreimal**
    die Completion-only-Recovery samt Legacy-Tick.
  - Danach versuchen alle konkurrierenden Store-Writer zu schreiben: veralteter
    Pickup des quarantinierten Items, frischer Pickup eines **anderen** Items,
    Drop und Restore eines dritten Items.
  - **Basis und Runde 2:** Die Reservierung ist nach jedem der sechs Schritte
    `true`; alle vier Writer erhalten `Reserved`; 0 Work; 2 Entities.
  - **Runde-1-Skizze:** Reservierung sechsmal `false`, der veraltete Pickup
    erzeugt 1 Work. Der Test fällt durch.

### 3.3 Geänderte Fence-Prüfung: Verweigerungsreihenfolge

**Analyse:** `require_unfenced_item` ist eine reine Leseoperation ohne
Seiteneffekt.

- Ohne Abschirmung liefert sie immer `Ok(())`, auch bei unbekanntem Handle.
- Danach läuft der Aufrufer exakt wie auf der Basis.
- Im Legacy-Modus gibt es nie eine Abschirmung.

**Beleg, eigener Differentialtest gegen die Basis** (Probe B, auf Memory und
redb):

- Item 2 wird zuvor vollständig aufgenommen; sein Handle ist damit veraltet.
- Danach folgen Eingaben, die auf zwei Arten falsch sind.
- Die erwarteten Werte sind auf die Basis gepinnt.

| Fall | Basis | Runde 2 | Runde-1-Skizze |
|---|---|---|---|
| Counter: Pickup, veralteter Handle **und** veralteter Spieler-Read | `StaleSnapshot` | `StaleSnapshot` | `EntityStore(StaleHandle)` |
| Counter: Pickup, veralteter Handle, frischer Read | `EntityStore(StaleHandle)` | gleich | gleich |
| Counter: Save, veralteter Handle | `Entity(Store(StaleHandle))` | gleich | `EntityStore(StaleHandle)` |
| Counter: Save, falsche Generation | `StaleGeneration` | gleich | gleich |
| Counter: Pickup, lebendes Item, veralteter Read | `StaleSnapshot` | gleich | gleich |
| Legacy: Pickup, veralteter Handle und veralteter Read | `StaleSnapshot` | gleich | `EntityStore(StaleHandle)` |
| Legacy: Pickup, veralteter Handle, frischer Read | `EntityStore(StaleHandle)` | gleich | gleich |

- In allen Fällen entsteht 0 Work.
- Runde 2 ist in jedem Fall identisch mit der Basis.
- Die Runde-1-Skizze hätte die Reihenfolge in zwei Fällen verändert.

**Innerhalb einer Abschirmung** (Probe C, nur Messzeilen):

- Ein Pickup mit veraltetem Read ergibt `Ineligible("quarantined item")`
  statt `StaleSnapshot`.
- Eine Save mit falscher Generation ergibt ebenfalls
  `Ineligible("quarantined item")`.
- Reconcile ergibt `Ineligible("quarantined pending")`.
- 0 Work, Reservierung frei, Status bleibt `Quarantined`.
- Das entspricht Brief §5.1 Punkt 9 („vor der Stamp-Prüfung“); siehe Info I-R2-3.

## 4. A2, A3, A5, A4/A6

### 4.1 A2: `INVARIANTS.md`

- **Anker:** Die korrigierten Zeilen stimmen auf f1e7511c.
  - Z. 347 beginnt die Regel „Every eligible resident live item …“.
  - Z. 386–387 enthalten „freeze while Entity pending exists“ innerhalb der Regel
    ab Z. 384.
  - Die Zeilenangaben 349–350 und 391–392 des Runde-1-Reviews lagen daneben.
- **Vollständig:** Die vier Blöcke decken alle vier von V2 geänderten Regeln ab.
  - Nach der Übernahme steht kein überholter Satz mehr.
  - Die vier Abschnitte, in denen sie stehen, sind passend: INV-290 im
    DUR-04.1-Abschnitt mit ausdrücklicher Modusangabe; INV-347/366/386 im
    Counter-Abschnitt ab Z. 329.
- **Gegen den Rest der Datei gelesen**, alle Treffer zu quarant/reserv/eligible/
  freeze/reconcil. Unverändert gültig bleiben:
  - Z. 300 „historical superseded receipt quarantines/resyncs“;
  - Z. 438, 489, 501 (Quarantäne behält Publikations-, Zellen- und Pin-Kredite;
    passt zu INV-290 „charge and credits stay bound“);
  - SIM-03.4b Z. 535–550 (Release-Gates, `owned_stop_ready`).
- **Kein Widerspruch gefunden.** Nur eine Begriffsunschärfe in SIM-03.4b
  (Info I-R2-1).

### 4.2 A3: Nachtrag DUR-04.2 und DUR-04.1

**Mechanisch:**

- `round2/amend_check.py` als Kopie mit umgeleitetem Ausgabepfad: rc 0. Die
  Ausgaben sind byte-gleich mit dem eingefrorenen `amendment-check/`.
- **Eigenes Skript** `amend/own_apply.py`:
  - Es übernimmt die Alt-Texte **nicht** aus dem Skript des Erstellers. Es
    ersetzt nur die Zeilenbereiche, die die Überschriften des Briefs nennen
    (10 + 2 + 4 Bereiche, von unten nach oben, ohne Überlappung).
  - Den Ersatz liest es aus den Blöcken des eingefrorenen Briefs; §11 hängt es an.
  - **Ergebnis:** Alle drei Dokumente sind byte-gleich mit denen des Erstellers
    (DUR-04.2 `9d5ab7a9…`, DUR-04.1 `9a8f8616…`, INVARIANTS `85c5d6e4…`).
  - Anfangs- und Endwörter jedes Ersatzes schließen grammatisch an den
    unveränderten Kontext an.

**Inhaltlich** habe ich beide Dokumente im Zusammenhang gelesen (Diff mit fünf
Kontextzeilen sowie die vollständigen Abschnitte DUR-04.2 §3, §7, §8, §9, §11
und DUR-04.1 §3, §4, §6 und den Implementierungsteil):

- **„max1/Shard“ gegen „limits.pending“:** aufgelöst.
  - Z. 188 lautet jetzt „max1 aktiv gebunden/Shard, dazu eingefrorene
    quarantinierte bis zu den §11-Grenzen“.
  - Z. 260 und Z. 360 sprechen von der „aktiven“ Entityoperation.
  - DUR-04.1 Z. 229ff. und Z. 377ff. unterscheiden `.1`-Modus und Countermodus.
- **Verweise:** Alle Verweise des §11 zeigen auf vorhandene Abschnitte, auch §5
  („wie beim Crash“, DUR-04.2 Z. 135). DUR-04.1 §3/§6 verweisen auf DUR-04.2
  §11. Kein verwaister Verweis.
- **Unverändert stehen und weiter gültig:**
  - DUR-04.1 Z. 241: „während der Entityreservation verweigern alle
    konkurrierenden Storewriter“;
  - DUR-04.1 Z. 262ff.: definitive Refusals; ein CasConflict im Countermodus
    galt schon vor SIM-03.5 nicht als definitiv;
  - DUR-04.2 Z. 187 (Claims „pro gebundenem Item/Pending“);
  - DUR-04.2 §8 („höchstens eine … Entityspawnreservation“; eine quarantinierte
    Spawn-Pending hat keine zulässige Completion).
- **Gegen den Code geprüft:**
  - D2-279: CasConflict ist im Countermodus nie definitiv; der Wegfall von
    „Conflictrefusal“ ist richtig.
  - D2-604: eingefroren am Boundaryende (`apply_counter_boundary`).
  - §11 Punkte 1, 2, 4–7 stimmen.
  - **Punkt 3 enthält den Fehler N1** (§5).

**Nachbarverträge:**

- **SIM-03.4b:**
  - `owned_stop_ready` und die Release-Gates sind unverändert.
  - Das Gate `ItemReservation` prüft `after_entity`
    (`release.rs:194-199`), greift also weiter für den Collector.
  - Vereinbar; Info I-R2-1.
- **NET-04.2d §6.1a:**
  - Im Fall N wendet ein Recovery-Boundary die Completion an. Danach bindet der
    Plan die Projektion nicht mehr, `CounterRecoveryRequired` entfällt.
  - Weil die Quarantäne nach §6.5 nur zu `Idle{Quarantined}` führt und die
    Pendingzahl nicht sinkt, greift für den Austritt Regel (ii) (≤ K = 20). Der
    Brief sagt „K-Regel bzw. Regel (i)“; das ist richtig (Info I-R2-2).
  - Die vorgeschlagene Notiz ist richtig.
- **NET-04.2d §10.4:**
  - Invariante P bleibt, weil der abgeschirmte Slot seine Prioritätsmarke behält
    (INV-386) und V2 keine Generationsänderung hinzufügt.
  - Die Terminierung schließt abgeschirmte Slots aus. Phase B endet über den
    Wachhund mit `Unresolved`.
  - Die Notiz „Höchstens eine Entity-Operation zugleich meint eine aktive“ ist
    nötig und richtig.
- **NET-04.2e2:**
  - Die Leave-Save-Quarantäne ist wieder erreichbar (T3).
  - Der dauerhaft unsichere Pickup ist ausdrücklich abgegrenzt.
  - Der e2-Session-Test ist grün.

### 4.3 A5: Ressourcen, nachgerechnet

**Konstanten im Code:**

| Konstante | Wert | Stelle |
|---|---|---|
| `MAX_PLAYER_OWNERSHIP_PENDING` | 32 | `player.rs:49` |
| `MAX_PLAYER_OWNERSHIP_PENDING_BYTES` | 8 MiB | `player.rs:51` |
| `MAX_ENTITY_DATA_LEN` | 256 KiB | `ferrumc-storage/src/record.rs:58` |
| `MAX_OWNED_ENTITY_SAVE_CHARGE` | 2 MiB | |
| `MAX_OWNED_PLAYERS` | 128 | |
| `MAX_OWNED_PLAYER_BYTES` | 32 MiB | |
| Live-/Confirmed-Bytes | je 16 MiB | |
| Publikationsladung | 256 B | `ITEM_PUBLICATION_SCALAR_CHARGE` |

- **Ladung:** Die Ladungsformel steht in `pending_charge`
  (`player_ownership.rs:938-957`): erst Anzahl, dann Bytes, jeweils mit
  `Capacity(...)`.
- **Rechnung:**
  - 661 + 2·262 144 + 5·262 144 = **1 835 669 B**, also ≤ 2 MiB;
  - ⌊8 388 608 / 1 835 669⌋ = 4;
  - 4 · 1 835 669 = **7 342 676 B**, Rest 1 045 932 B;
  - die fünfte Save braucht 1 835 669 > 1 045 932 B → `Capacity("pending bytes")`
    bei 4 von 32 Plätzen.
  - **Bestätigt.**
- **Mechanisch** (Probe D, Memory und redb), skaliert mit kleinen Items
  (c = 1 858 B = 661 + 7·171):
  - Vier echte CasConflict-Quarantänen binden genau 4c.
  - Die Reservierung ist jeweils frei.
  - Grenze 5c − 1: Die fünfte Save erhält `Capacity("pending bytes")` bei 4/32.
  - Grenze 5c: Die fünfte wird zugelassen.
  - Damit sind exakt/+1 und die Bindung der Ladung durch Quarantänen belegt.
- **Übrige Tabelle §6.1:**
  - `reserve_spawn_slot` reserviert nur Vec-Kapazität (`entity.rs:606-614`);
  - der Despawn trägt eine Publikationsladung (`publication_charge(None)` im
    `EntitySave`-Arm bei `due`);
  - Release `ItemReservation` und Rejoin `Reserved` stimmen;
  - die allgemeine Formel k_max = min(pending, ⌊pending_bytes / c⌋) und die
    Aussage zu c > 4 MiB stimmen.

### 4.4 A4 und A6: Ehrlichkeit und Abgrenzung zu `Uncertain`

- **A4:**
  - Brief §1.1 führt ehrlich auf, was Runde 1 belegt hatte und was nicht (Session
    und Integrationstests nicht ausgeführt, SendMessage mit `success: false`,
    falsche INVARIANTS-Aussage, Legacy-Pfad).
  - Die Testzahlen in §0, §7 M14 und §10 habe ich reproduziert.
  - Der Ersteller-Thread Runde 2 (nur gelesen) enthält keine Frage an Root und
    keine abweichende Zahl.
- **A6:**
  - Die falsche Begründung ist zurückgenommen, B6 als Gegenbeleg genannt.
  - V2 wird im Code nicht auf `Uncertain` ausgedehnt: `fences_item`, der
    Plan-Filter und die Reconcile-Verweigerung verlangen alle `Quarantined`.
  - T8 und der e2-Test sind unverändert.
  - §11 Punkt 5 des Nachtrags sagt das ausdrücklich.
  - Das R3-A5-Gate gilt laut Brief nur für `Quarantined` als erfüllt; das eigene
    Ticket mit Aktivierungsgrenze liegt bei Root.
- **Keine Aktivierungs-, Survival- oder Performance-Aussagen.**
  - Forbidden-Claims-Scan (`markdown_blocks` + `scan` aus
    `scripts/check-forbidden-claims.py`, mit und ohne erzwungenen Plugin-Scope):
    Brief 0 Funde in 415 Blöcken, geändertes DUR-04.2 0 von 132, DUR-04.1 0 von
    126.
  - `INVARIANTS.md` hat 2 Treffer (Z. 87, 321 → 326). Sie bestehen identisch
    schon auf der Basis und stammen nicht aus dem Nachtrag.
  - Self-Test des Skripts: 100/100.

## 5. Neuer Befund

### N1 (niedrig) — §11 Punkt 3: falsche Begründung für den Pickup im quarantinierenden Boundary

- **Stelle:** Brief §8.2, Block `D2-S11`, Punkt 3, Brief-Z. 862–864; nach der
  Übernahme DUR-04.2 §11 Punkt 3.
- **Text:** „Pickup und Save dieses Items werden … mit
  `Ineligible("quarantined item")` verweigert; im quarantinierenden Boundary
  selbst verweigert sie noch die bestehende Reservation (`Reserved`).“
- **Tatsache:**
  - `run_player_ownership_boundary` wendet alle Completions als Präfix vor den
    übrigen Inputs an (`player_ownership.rs:312-318`). Die Quarantäne steht also
    fest, bevor ein Pickup desselben Boundaries angewandt wird.
  - Der Pickup-Arm prüft die Abschirmung vor jeder Reservierungsprüfung
    (Prototyp `player_ownership.rs:531`, vor `plan_owned_pickup`).
  - Ergebnis: Ein Pickup des Items wird im quarantinierenden Boundary **immer**
    mit `Ineligible("quarantined item")` verweigert. Nur die Save prüft die
    Reservierung zuerst und erhält `Reserved` (Brief §5.1 Punkt 7 sagt das für
    die Save richtig).
- **Beleg** (Probe E, `probes/r2rev_cap.rs`, Memory und redb): Completion,
  Pickup und Save im selben Boundary, in beiden Eingabereihenfolgen. Die
  Verweigerungen sind jeweils `CasConflict | Ineligible("quarantined item") |
  Reserved`, dazu 0 Work und nach dem Boundary eine freie Reservierung.
- **Folge:**
  - Keine Sicherheitsfolge; der Plan verweigert denselben Pickup über `reserved`,
    die Disposition ist gleich.
  - Der Nachtrag ist aber zur wörtlichen Übernahme bestimmt. Ein Runtime-Autor,
    der Punkt 3 als Testerwartung übernimmt, bekäme für den Pickup einen roten
    Test oder würde den Code ohne Not umsortieren.
  - Das Runde-1-Review hatte den Unterschied in §3.2 festgehalten. A3 hatte nur
    für die **Save** „ab dem folgenden Boundary“ verlangt.
- **Korrektur, wörtlich:** Im Block `D2-S11` die Zeilen

  ```text
     Save dieses Items werden vor jeder Kandidaten- oder Bodykopie mit
     `Ineligible("quarantined item")` verweigert; im quarantinierenden Boundary
     selbst verweigert sie noch die bestehende Reservation (`Reserved`). Eine
  ```

  ersetzen durch

  ```text
     Save dieses Items werden vor jeder Kandidaten- oder Bodykopie mit
     `Ineligible("quarantined item")` verweigert. Das gilt für Pickup schon im
     quarantinierenden Boundary, weil Completions dort vor allen übrigen Inputs
     angewandt werden; eine Save verweigert dort noch die bestehende Reservation
     (`Reserved`). Eine
  ```

  Die übrigen Zeilen des Punkts bleiben unverändert. Zusätzlich empfehle ich
  eine Runtime-Pflichtzeile: Completion, Pickup und Save desselben Items im
  quarantinierenden Boundary ergeben `CasConflict`,
  `Ineligible("quarantined item")`, `Reserved` und 0 Work.

### Info

- **I-R2-1:** Der SIM-03.4b-Block in `INVARIANTS.md` (nach der Übernahme
  Z. 539–547) nennt das Gate `ItemReservation` „item/chunk (entity owner)
  reservation“.
  - Unter V2 hält eine quarantinierte Pair-Pending diese Reservierung nicht mehr.
  - Das Gate (`after_entity`) und „no other pending“ verweigern aber weiter.
  - Kein Widerspruch im Ergebnis. Eine Klarstellung im Runtime-Ticket ist
    optional; der Brief §5.3 benennt die Unschärfe bereits.
- **I-R2-2:** Für die NET-04.2d-Notiz zu §6.1a:
  - Nach einer Quarantäne im Recovery-Boundary meldet §6.5 `Idle{Quarantined}`,
    und die Pendingzahl sinkt nicht.
  - Der Austritt kommt deshalb über Regel (ii) nach ≤ 20 Recovery-Boundaries.
  - Das sollte die Notiz ausdrücklich sagen.
- **I-R2-3:** Im Counter-Modus geht die Abschirmung der Pair-Validierung vor
  (Probe C). Das ist vertragsgemäß und für Testautoren relevant.
- **I-R2-4:** In D2-279 wird der Zeilenumbruch „No-start-“ / „Refusal“ in
  Markdown als „No-start- Refusal“ dargestellt; rein kosmetisch.
- **I-R2-5:** Die Forbidden-Claims-Treffer in `INVARIANTS.md` bestehen schon auf
  der Basis (§4.4).

## 6. Empfehlung an Root

1. **CLEAN_CONTRACT:** nach der Korrektur N1. Sie ist rein textlich, umfasst drei
   Zeilen im Block `D2-S11`, und der Wortlaut steht in §5. Weitere Änderungen
   braucht der Vertrag aus meiner Sicht nicht. Ob Root N1 selbst bei der
   Übernahme korrigiert oder den Ersteller bittet, entscheidet Root.
2. **Nachtrag:** die 10 + 2 Stellenersetzungen und §11 (mit N1-Korrektur)
   wörtlich übernehmen. Die NET-04.2d-Notizen aus Brief §9 sind richtig;
   I-R2-2 als Präzisierung.
3. **Runtime-Freigabe V2**, Scope `server/crates/ferrumc-sim/**` einschließlich
   `INVARIANTS.md`. Pflicht zusätzlich zur Brief-Matrix (M1–M15, R1–R4 und die
   weiteren Brief-§7-Zeilen):
   - die Zeile aus N1;
   - meine Probe B (Verweigerungsreihenfolge außerhalb einer Abschirmung =
     Basis) als echter Test;
   - Probe A (Legacy: mehrfache Recovery, alle vier Writer `Reserved`);
   - Probe D (Pendingladung exakt/+1).
4. **Aktivierung:** bleibt an DUR-05a gebunden (Metrik
   `native_pending_quarantined`, Schwelle O2, e2-Auflage) und an das eigene
   `Uncertain`-Ticket.

## 7. Fragen an Root

Keine blockierenden Fragen. Root entscheidet, ob N1 bei der Übernahme selbst
korrigiert oder vom Ersteller nachgeliefert wird.

## 8. Evidenz

Alle Artefakte liegen unter `.codex-tmp/sim03_5-root-review-r2/`;
`review.freeze.json` bindet dieses Dokument und alle Artefakte. Die SHA-Werte
werden außerhalb dieses Dokuments an Root gemeldet.

| Artefakt | Inhalt |
|---|---|
| `run_probe.py` | Kopie des Ersteller-Runners mit eigenem Ausgabepfad und Tag `R2REV` |
| `run_suites.sh`, `logs/suites.status`, `logs/{fmt,clippy-sim,test-*,doctest-sim}.log` | volle Suiten auf `proto-clean/` |
| `run_probes.sh` | alle `counters::tests` mit T1–T14, R1–R4 und eigenen Proben, debug und release |
| `probes/r2rev.rs` | eigene Proben A (Legacy-R1), B (Differential der Verweigerungsreihenfolge), C (innerhalb der Abschirmung) |
| `probes/r2rev_cap.rs` | eigene Proben D (Pendingladung) und E (Begründung im quarantinierenden Boundary), nur Runde-2-Prototyp |
| `results/*.json`, `logs/*.log` | Kommandos, Exit-Codes, Logs mit SHA-256, Messzeilen. `d-proto-debug` ist durch `e-proto-debug` ersetzt (enthält D und E). `b-proto-r1-debug` und `rev-base-debug` scheitern erwartungsgemäß (Gegenkontrolle) |
| `amend/amend_check_author_copy.py`, `amend/author-run/` | Lauf des Ersteller-Skripts mit umgeleiteter Ausgabe |
| `amend/own_apply.py`, `amend/own/` | eigene, zeilenbasierte Übernahme und Byte-Vergleich |
| `scan_claims.py`, `results/forbidden-claims.json` | Forbidden-Claims-Scan |
| `proto.diff` | eigener Diff Basis → Prototyp (byte-gleich mit der Skizze) |
| `final-assessment.json` | Verdikt und Befunde maschinenlesbar |
