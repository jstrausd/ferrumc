# SIM-03.5 — Review des Vertrags „quarantinierte Item-Pendings isolieren“

Unabhängiges Root-Review, 12.10.2026. Der Reviewer hat den Kandidaten nicht
erstellt. Die Produktion blieb readonly. Alle Proben liefen ausschließlich in
privaten Archivkopien unter `.codex-tmp/sim03_5-root-review/`. Es gab kein Git,
keine Agents und keine Änderung an Brief, Evidenz oder Produktion.

Dieses Review nimmt weder Aktivierung noch Survival, Paper-Parität, Plugins oder
Performance ab.

## Verdikt: FINDINGS

- **Die Wahl V2 ist tragfähig.** Die Abschirmung je Item ist die kleinste
  Variante, die alle geschützten Invarianten im Counter-Modus erhält. Alle drei
  heutigen Fehlwirkungen sind reproduziert und mit Code belegt. Der Prototyp
  bleibt in Sim, Session, Testkit und App grün.
- **Für CLEAN fehlt:**
  - A1: Die Referenzskizze gibt die Store-Reservierung auch im Legacy-Modus frei.
    Das widerspricht dem Vertrag; eigene Probe R1 zeigt dort eine neue CAS gegen
    die überholte Erwartung.
  - A2: Die verbindliche `ferrumc-sim/INVARIANTS.md` existiert. Der Brief
    bestreitet das, und vier ihrer Regeln ändert V2.
  - A3: Der DUR-04.2-Nachtrag (§8) nennt nicht alle Stellen, die er ändert. Nach
    wörtlicher Übernahme widerspräche DUR-04.2 sich selbst.
  - Dazu drei niedrige Punkte (A4–A6), darunter zwei Ehrlichkeitskorrekturen.
- **Empfehlung an Root:** Den Nachtrag noch nicht wörtlich übernehmen. Eine enge
  Runde 2 behebt A1–A6 im Brief; danach Runtime-Freigabe für V2. Einzelheiten
  stehen in §6.

## 1. Kandidat, Basis und eigene Läufe

| Gegenstand | Pin |
|---|---|
| Brief | `docs/agent-tasks/SIM-03_5_QUARANTINE_ISOLATION.md`, SHA-256 `a21d8fd2…3e0c0` (frisch geprüft) |
| Evidenz-Freeze | `.codex-tmp/sim03_5/evidence.freeze.json`, SHA-256 `6dbb05ef…427e5a` (frisch geprüft); alle 25 lokalen Einträge stimmen |
| Workorder | `SIM-03_5_QUARANTINE_ISOLATION_TASK.md` |
| Basis | Archiv f1e7511c unter `.codex-tmp/sim03_5-root-base/archive`, `base.freeze.json` `b715025e…4356`, 1390/1390 Dateien frisch identisch |
| Originalquellen | Paper-Checkout `.codex-tmp/paper-1.21.8-reference`, HEAD `29c8822d90899c89d2689338e81a98f690bcba12`; alle vier ItemEntity-/EAR-Hashes frisch identisch |

**Eigene Kopien** (je mit eigenem Target `.codex-tmp/sim03_5-root-review/target`,
`--jobs 2`, `--locked --offline`, Toolchain `~/.cargo/bin` 1.99.0):

- `base/`: Basis plus Probe-Module;
- `proto/`: Basis plus `make_proto.py` (14 Ersetzungen) plus Probe-Module;
- `proto-clean/`: Basis plus `make_proto.py`, ohne Probe-Modul, für Clippy,
  vollständige Sim-Suite und Verbraucher.

Mein Runner `run_probe.py` prüft jede Kopie gegen `base.freeze.json`. Außer den
Probe-Dateien, ihren `mod`-Zeilen und den zwei erlaubten Prototyp-Dateien weicht
nichts ab. Der Prototyp ist zeilengleich mit `v2-reference-sketch.diff` (+81/−6
bestätigt).

| Lauf | Ergebnis |
|---|---|
| Ersteller-Baseline B1–B9, debug, eigene Kopie | 9/9 grün; alle 26 Messzeilen identisch mit der Ersteller-Evidenz |
| Ersteller-Target T1–T13, debug, eigener Prototyp | 14/14 grün; alle 35 Messzeilen identisch |
| eigene Gegenproben R1–R4 (`probes/rev035.rs`), Prototyp debug | 4/4 grün, Messzeilen §3 |
| R1–R4 auf der Basis (Kontrolle) | R1 und R4 grün. R2 und R3 brechen auf der Basis schon beim Restore eines zweiten Items ab (`tests.rs:238`, Restore verweigert): das ist B4 |
| `cargo fmt --all --check` (proto-clean) | grün |
| `cargo clippy -p ferrumc-sim --all-targets -D warnings` nach `cargo clean -p ferrumc-sim` | grün (frisch, nicht aus Cache) |
| `cargo test -p ferrumc-sim` **alle Targets** | 404/404: Lib 266, Integrationstests 127 (darunter 46 Legacy-`player_entity_transfer`), Doctests 11 |
| `cargo test -p ferrumc-session` | 244/244, darunter `uncertain_pickup_counts_every_100_quarantines_and_renews_one_final_save` |
| `cargo test -p ferrumc-testkit` | 100/100 |
| `cargo test -p ferrumc-app --no-fail-fast` | siehe §1a |
| Release-Läufe der Proben | siehe §1a |
| Forbidden-Claims (`markdown_blocks` + `scan` aus `scripts/check-forbidden-claims.py`, mit und ohne erzwungenen Plugin-Scope) | 0 Funde in 266 Blöcken; Self-Test des Skripts 100/100 |

Der Ersteller hatte nur die Lib-Tests von `ferrumc-sim` ausgeführt. Die
Integrationstests von Sim sowie Session, Testkit und App liefen erst in diesem
Review.

### 1a. App-Tests und Release-Läufe

| Lauf | Ergebnis |
|---|---|
| `cargo test -p ferrumc-app --no-fail-fast` (proto-clean) | 375 bestanden, 0 fehlgeschlagen, 1 ignoriert (bestehend), 39 Testbinaries; kein Wiederholungslauf nötig |
| alle `counters::tests` im Prototyp mit Target- und Review-Proben, debug und release | je 107/107; die 35 Target-Messzeilen sind in debug und release identisch mit der Ersteller-Evidenz, die 8 Review-Zeilen in allen Läufen identisch |
| Ersteller-Baseline, release, eigene Basiskopie | 9/9; 26 Messzeilen identisch mit `baseline-release.json` des Erstellers |

Die App-Tests legen erwartungsgemäß `.codex-tmp/p52-*` und `.codex-tmp/p57-*`
in der Kopie `proto-clean/` an. Das sind Testartefakte, keine Quellen.

## 2. Reproduktion der drei Fehlwirkungen

Alle drei Fehlwirkungen sind auf Memory und redb reproduziert und am Code belegt
(Zeilen auf f1e7511c).

1. **R3-A5, nach `defer − claims` dauerhaft `CounterRecoveryRequired`:
   bestätigt.**
   - `plan_counter_boundary` bindet jedes Pending mit `counters` ohne
     Statusfilter (`counters.rs:495-501`).
   - `step_boundary_entry` verweigert bei `claims > defer` (`counters.rs:2090-2095`).
   - Ein Recovery-Boundary hilft nicht, denn ein Quarantined-Pending hat keine
     Completion (`validate_completion` verlangt `InFlight`,
     `player_ownership.rs:1438-1442`).
   - B1/B9 reproduziert: Nach dem Eintritt bleibt der Disconnect eines
     unbeteiligten Spielers in der Queue, und Release → `DetachedRelease(Connected)`.
2. **Neu: dauerhafte Reservierung des ganzen Entity-Stores: bestätigt.**
   - `install_candidate` reserviert (`player_ownership.rs:1303-1308`,
     `entity/ownership.rs:270-281`).
   - Freigabe gibt es nur über `publish_owned` (`entity/ownership.rs:348`) oder
     `release_pending`. Beides erreicht ein Quarantined-Pending nie.
   - `require_owner` verweigert dann jeden Owned-Writer (`entity/ownership.rs:89-100`).
     Der Plan rechnet `reserved` ohne Statusfilter (`counters.rs:712-715`), und
     die Save-Auswahl kehrt sofort zurück (`counters.rs:1884-1886`).
   - Reproduziert sind B4 und B8 sowie unabhängig meine R2/R3 auf der Basis:
     Der Restore eines zweiten Items wird verweigert.
   - Andere Pfade, die ein Item im Ownership-Modus verändern, gibt es nicht:
     `require_legacy_write` (`entity/ownership.rs:74-79`) verweigert Legacy-
     Writer schon wegen `ownership_mode`. Die Reservierung ist also tatsächlich
     die einzige Sperre, und ihr Ende macht genau die Owned-Pfade wieder frei.
3. **Neu: Reconcile stuft eine CasConflict-Quarantäne zu `Uncertain` herab:
   bestätigt, aber nur über die Sim-API erreichbar.**
   - `dispatch_retained` lässt `clarify` für jeden Status zu und setzt
     `InFlight` (`player_ownership.rs:1381-1421`). Für die Query existiert nie
     ein Receipt, denn Storage prüft den Replay vor der CAS
     (`ferrumc-storage/src/ownership/engine.rs:533-536`, `check_cas` 374-389).
     Die Completion ist dann `ReceiptAbsent` → `Uncertain`
     (`player_ownership.rs:1607-1610`).
   - Die heutige Session sendet nie Reconcile auf `Quarantined`
     (`ferrumc-session/src/native/retention.rs:583-585`; NET-04.2d §6.5 „kein
     Input“). Erreichbar ist es also nur über einen fehlerhaften Driver.
   - **Zusatz:** Unter V2 trägt die Verweigerung Last. Ohne sie würde ein
     Reconcile den Status auf `InFlight` setzen, während die Reservierung schon
     frei ist. Dann bände der Plan die Projektion wieder und rechnete `reserved`
     wahr, die Anwendung sähe aber keine Reservierung: Plan und Apply liefen
     auseinander (DUR-04.2 §8). T7 deckt das ab.

Die Brief-Anker (§1 Hinweis 1610→1618, `validate_completion` 1423, Pickup-Arm
519-533, `counters.rs` 494-501/523-536/675-685/712-715/986/1051/1529-1534/
1781-1807/1884-1886/2082-2113, `engine.rs` 524-536/374-389, `retention.rs`
583-589) habe ich einzeln nachgeschlagen; sie stimmen.

## 3. Invarianten und eigene Gegenproben

### 3.1 Was die DUR-04.2-Regel schützt

Die Herleitung in Brief §3.1/§3.2 ist richtig. Die Projektion schützt drei
Dinge für ein **lösbares** Pending, das später veröffentlicht oder freigegeben
wird:

- einmalige Tickzählung: `first/last`, keine Doppelzählung;
- die Grenze unbestätigten Fortschritts: `claims ≤ defer`, DUR-04.2 Z. 187/205-207;
- ehrliche Weltzeit: kein erfolgreicher Tick bei still stehenden Items,
  Z. 320-328.

Für `Quarantined` gibt es keinen solchen Abnehmer mehr, denn Completion, Retry
und Restore lesen die Projektion nicht. Die Regel hat dort keine Schutzwirkung,
nur die Eskalation.

### 3.2 Verletzt V2 eine Invariante in einem erreichbaren Fall?

**Im Counter-Modus: nein.** Geprüft:

- **Einmalige Tickzählung:**
  - Ein abgeschirmtes Item wird im Plan nie gebunden (`resident = false`, Skizze
    `counters.rs:543-549`), nie gestept und nie angewandt.
  - Im Quarantäne-Boundary zählt es höchstens einen Claim. Danach schreibt nur
    `apply_counter_boundary` einen Wert in eine Projektion, die nie gelesen
    wird.
- **Begrenzte unbestätigte Fortschrittsmenge:** Nicht quarantinierte Pendings
  behalten `claims ≤ defer`. Abgeschirmte zählen nicht mehr.
- **Ehrliche Weltzeit:**
  - Das Item wird nicht als getickt gemeldet (`counter_steps` filtert `resident`).
  - Die Abweichung ist deklariert. Ihre Sichtbarkeit hängt aber an einer noch
    nicht implementierten Metrik (Info I1).
- **Plan gleich Apply:**
  - Der Plan leitet Fence und `reserved` aus dem Status bei Planbeginn ab.
  - Im quarantinierenden Boundary rechnet der Plan `reserved` (Status noch
    `InFlight`), und die Apply-Seite hält die Reservierung bis Boundaryende.
    Beide verweigern.
  - Die Begründungen können abweichen: Die Apply-Seite meldet für das Item
    selbst `Ineligible("quarantined item")` beim Pickup bzw. `Reserved` beim
    Save. Die Disposition ist gleich. R3 bestätigt den gemeinsamen Boundary:
    CasConflict-Refusal plus `Reserved` für den Restore, kein Panic.

**Im Legacy-Modus: ja, durch die Skizze** (Befund A1).

### 3.3 Lebenszyklus des abgeschirmten Items

| Fall | Ergebnis | Beleg |
|---|---|---|
| Rejoin des Collectors | `Reserved` (Pending blockiert `shadow_player_admission`, `counters.rs:1757-1763`) | T3 |
| Release | Collector `DetachedRelease(ItemReservation)` (`release.rs:197-199`, Gate je Spieler); Unbeteiligte frei | T3, T10 |
| Stop | dauerhaft `NotQuiescent` (`resources.pending_count == 0` verfehlt, `counters.rs:901`); Phase B endet über den Wachhund | T9 |
| Neustart | Durable-Stand; keine Pendings | T13 |
| Chunk-Unload/Reload | Körper bleibt registriert (`release_owned_chunk` entfernt nichts), 0 Schritte, nach Reload weiter abgeschirmt, `defer + 16` Boundaries `Ok`; auf der Basis nach 6 Boundaries `CounterRecoveryRequired` | **R4** |
| Restore derselben UUID (frischer echter Read des Fremd-Records) | `EntityStore(DuplicateUuid)`; Plan verweigert ebenso (`shadow_spawn_admitted` prüft UUID, `counters.rs:1683-1685`) | **R3** |
| Restore im Quarantäne-Boundary | `Reserved`, kein Panic | **R3** |
| Despawn-Quarantäne (fälliges Item, Delete-CAS kollidiert) | Pending `EntityDespawn`, Item bleibt `due` und sichtbar, 24/24 Boundaries `Ok`, 0 `ItemSaveBlocked`, ein anderes Item wird gespeichert | **R2** (vom Brief §7 als Pflicht genannt, hier vorab belegt) |

### 3.4 Sichtbarkeit und Interaktion

- **Sichtbar** bleibt das Item (deklariert, O6).
- **Pickup durch andere Spieler:**
  - Der Plan verweigert über `!entry.resident` (`counters.rs:1051`), die
    Anwendung über `require_unfenced_item` vor `plan_owned_pickup`.
  - T3b belegt den gefährlichsten Fall, einen vor dem Konflikt gelesenen,
    passenden Pair-Read: 0 Work.
- **Merge und Despawn:**
  - Im Ownership-Modus gibt es keinen Merge-Producer (DUR-04.2 §6 `.2e`), und
    Legacy-Writer sind gesperrt.
  - Despawn läuft nur über die Save-Auswahl, die den Slot überspringt (R2).
- **Andere Pfade auf die überholte Erwartung:**
  - Store-Writer sind nur `prepare_owned_*` und `reserve_owned`
    (`entity/ownership.rs:118/163/235/275`), dazu `plan_counter_save` und
    `select_counter_save`.
  - Alle sind für das Item abgeschirmt. Spawn oder Drop derselben UUID scheitert
    am frischen Read und an der UUID-Prüfung.
  - **„Reservierung endet am Boundaryende“ ist im Counter-Modus sicher.**

### 3.5 Eigene Messzeilen (Prototyp, debug, Memory = redb)

- **R1 (Legacy-Modus):**
  - Pickup-Quarantäne über Superseded (echter Commit, verlorene Antwort,
    Fremdschreiber, Reconcile), danach einmal `recover_player_ownership_completions()`.
  - **Prototyp:** Reservierung vorher `true`, danach `false`. Ein vor dem
    Konflikt gelesener Pair-Read eines zweiten Spielers erzeugt **1 Work**
    (CAS gegen die überholte Erwartung).
  - **Basis:** Reservierung bleibt `true`, derselbe Pickup → `Reserved`, 0 Work.
- **R2:** `EntityDespawn`, CasConflict `expected rev 1 / actual rev 2`, Item
  `due`, 24/24 `Ok`, `first_error = none`, Item 2 gespeichert, Status bleibt
  `Quarantined`.
- **R3:** Restore derselben UUID später `DuplicateUuid`, im selben Boundary
  `CasConflict | Reserved`; 24/24 `Ok`; Reservierung frei.
- **R4:** Unload → `Unloaded`, 4/4 `Ok` entladen, Reload, 24/24 `Ok`, Counter
  unverändert; Basis: 4 entladen `Ok`, nach Reload 6 `Ok`, dann
  `CounterRecoveryRequired`.

## 4. Varianten

- **V1 abgelehnt, tragfähig:** Die Reservierung bliebe, also der Item-Pfad des
  Shards gesperrt (B4/B8). Das R3-A5-Ziel „ein Item degradiert nicht den Shard“
  wäre verfehlt.
- **V3 abgelehnt, tragfähig:**
  - Für Superseded ist der Pair-Commit bereits erfolgt, eine Freigabe würde
    Mengen doppeln.
  - Für CasConflict entspräche V3 dem Legacy-Verhalten. Im Counter-Modus
    hinterließe es aber ein freies Item mit bekannt überholter Erwartung; jede
    weitere Save wäre eine CAS gegen eine bekannt veraltete Erwartung
    (DUR-04.2 Z. 282).
- **V4 abgelehnt, tragfähig:** Obermenge von V2 mit zusätzlichen
  Session-sichtbaren Pfaden, ohne Sicherheitsgewinn.
- **Kleinere Variante:** keine gefunden.
  - V2 ohne Fence würde Save und Pickup gegen die überholte Erwartung zulassen.
  - V2 ohne Reconcile-Verweigerung bricht Plan gleich Apply (§2 Punkt 3).
  - V2 ohne Apply-Fence beim Pickup bräche die Gleichheit zum Plan.
- Die beiden Mess-Fallen des Erstellers (9-MiB-Plan-Grenze bei neuem Feld,
  100-Zeilen-Grenze von `plan_owned_pickup`) habe ich nicht erneut gemessen. Der
  Prototyp kommt ohne beide aus, und der Shape-Test bleibt grün.

## 5. Originalsemantik (Source-Review, kein Differenziallauf)

Gegen die gepinnten Quellen (HEAD `29c8822d…`, vier SHA-256 frisch identisch)
nachgeschlagen:

- **Vanilla:** `tick()` ab Z. 117, `age++` außer -32768 bei Z. 172-173,
  `age >= 6000` → `discard()` Z. 184-185, `putShort("Age")` Z. 309, Lesen Z. 321.
- **Paper:**
  - `inactiveTick()` Z. 134ff mit `age++` (140) und Despawn ab 143
    (Cancel → `age = 0`, 146);
  - `tick()` ab 156 mit `age++` (216) und Despawn ab 227 (Cancel 230);
  - Speichern 377, Lesen 394;
  - EAR `checkIfActive` (Patch Z. 311-313; Items jeden vierten Tick aktiv).

Die Aussage des Briefs stimmt: Das Original altert jeden getickten Körper und
entfernt ihn an der Schwelle. Eine Projektion kennt es nicht. Die Präzisierung
„der Despawn bräuchte einen Delete gegen die überholte Erwartung“ gilt für den
Prozess ohne frischen Read. Die Alternative wäre die In-Prozess-Übernahme nach
O4 (SIM-03.5b), was der Brief als Folgeticket führt. Weder heute noch V2
liefern Parität; das ist deklariert.

## 6. Befunde nach Schwere

Keine hohen Befunde.

### A1 (mittel) — Skizze gibt die Reservierung im Legacy-Modus frei

- **Stelle:** `v2-reference-sketch.diff`, Hunk `@@ -291,6 +292,7 @@`; im
  Prototyp `counters.rs:295` (Aufruf in `recover_player_ownership_completions`)
  und `counters.rs:441-450` (`release_quarantined_reservation` ohne
  Counter-Modus-Prüfung).
- **Ursache:**
  - `recover_player_ownership_completions` (`counters.rs:237-296` Basis)
    prüft den Counter-Modus nicht und ist auch auf einem DUR-04.1-Owner ohne
    Counter aufrufbar.
  - Eine Superseded-Quarantäne entsteht in beiden Modi
    (`player_ownership.rs:1645-1647`).
- **Szenario (R1):** Legacy-Owner, Pickup-Quarantäne über Superseded, ein Aufruf
  der Completion-only-Recovery.
  - Danach ist die Reservierung frei.
  - Der Pickup des quarantinierten Items mit einem Pair-Read von vor dem
    Konflikt erzeugt eine Work, also eine CAS gegen die überholte Erwartung.
    Die Fence-Prüfung `item_fenced` greift nur im Counter-Modus.
  - Auf der Basis: `Reserved`, 0 Work.
- **Widerspruch zu:**
  - Brief §5.1 („Gilt nur im Counter-Modus“), §7 Pflichttest „Legacy-Modus: kein
    Reservierungsende“ und §8 Nachtrag („der DUR-04.1-Legacypfad bleibt
    unverändert“);
  - DUR-04.1 §3 Z. 229-237 („während der Entityreservation verweigern alle
    konkurrierenden Storewriter“) und Sim-`INVARIANTS.md` Z. 290-291.
- **Folge:** Storage verweigert diese CAS (Durable ist weitergezogen), der
  Legacy-Pfad gibt dann definitiv frei. Mengen gehen nicht verloren, aber die
  Legacy-Garantie „ganzer Entity-Owner reserviert bis zu passender Evidenz“ ist
  gebrochen. Ein Runtime-Autor, der die Skizze übernimmt, würde den Pflichttest
  verfehlen.
- **Korrektur:**
  - Brief §5.1 Punkt 4 präzisieren: `release_quarantined_reservation` wirkt nur
    bei `item_counters.is_some()`; Guard in der Funktion oder an beiden
    Aufrufstellen.
  - Skizze anpassen.
  - Der Legacy-Pflichttest muss ausdrücklich
    `recover_player_ownership_completions` auf einem Legacy-Owner mit
    Quarantäne aufrufen.

### A2 (mittel) — `ferrumc-sim/INVARIANTS.md` existiert und wird geändert

- **Stelle:** Brief §1, „`ferrumc-sim` hat keine eigene `INVARIANTS.md`“.
- **Tatsache:**
  - `server/crates/ferrumc-sim/INVARIANTS.md` (560 Zeilen) liegt in Basis und
    Arbeitsbaum, inhaltlich identisch.
  - AGENTS.md Z. 31 verlangt, sie zu lesen; Z. 126 verlangt, sie bei neuen
    Regeln zu aktualisieren. Sie ist im Auftrag als „Sim-INVARIANTS“ verbindlich
    genannt.
- **V2 ändert vier dort festgeschriebene Regeln:**
  - Z. 290-291: „the whole entity owner remain reserved until genuine matching
    evidence“;
  - Z. 366-372: „At most one Entity operation reserves the whole owner
    structure … CasConflict quarantines with the original scalar projection“;
  - Z. 349-350: „Every eligible resident live item receives exactly one scalar
    step“. Bleibt wahr, wenn „eligible“ die Abschirmung umfasst; das sollte dort
    stehen.
  - Z. 391-392: Save-Cursor „freeze while Entity pending exists“. Unter V2
    laufen die Cursor trotz quarantiniertem Entity-Pending.
- **Korrektur:**
  - Brief §1 berichtigen.
  - §10 Ticketzuschnitt um die Aktualisierung von `INVARIANTS.md` erweitern; sie
    liegt im Runtime-Scope `ferrumc-sim/**`.
  - Den Wortlaut der vier Stellen im Brief vorgeben.

### A3 (mittel) — Nachtrag §8 ist nicht vollständig verankert

- **Stelle:** Brief §8, Kopf „Ersetzt … Z. 604–605. Präzisiert … Z. 279–282 …
  und … Z. 320–328“.
- **Nicht genannt, von V2 aber geändert:**
  - DUR-04.2 §7 Tabelle Z. 188: „Private counter ledger entries | 1 pro
    Entitypending, max1/Shard | keine zweite Entityoperation“. Unter V2 bestehen
    bis `limits.pending` Projektionen fort, neben einer weiteren aktiven
    Entity-Operation.
  - DUR-04.2 Z. 260-261: „weiterhin höchstens **eine** Entityoperation pro
    Shard“. Punkt 4 des Nachtrags sagt das im Text, der Kopf nennt die Stelle
    nicht.
  - DUR-04.2 Z. 360-361: „Wenn schon Entitypending existiert: **keinen** neuen
    Savecandidate encodieren, keine Cursor-/Turnfortschritte“.
  - DUR-04.1 §3 Z. 229-232 und Z. 373 („Eine Entityreservation pro Shard“, die
    „während Pending“ gilt). DUR-04.2 verfeinert DUR-04.1 im Counter-Modus; der
    Nachtrag sollte die Abweichung für `Quarantined` ausdrücklich nennen.
- **Folge:** Nach wörtlicher Übernahme stünde in DUR-04.2 Z. 188 „max1/Shard“
  neben §11 „bis `limits.pending`“. Das ist nicht widerspruchsfrei.
- **Kleinere Ungenauigkeiten im Text:**
  - Punkt 2, „behält ihren Wert zum Quarantänezeitpunkt“: Der Wert wird am Ende
    des quarantinierenden Boundaries noch einmal geschrieben
    (`apply_counter_boundary`, `counters.rs:1781-1802`). Besser: „ihren Wert am
    Ende des quarantinierenden Boundaries“.
  - Punkt 3, „Pickup und Save … mit `Ineligible("quarantined item")`
    verweigert“: Im quarantinierenden Boundary selbst ist die Begründung beim
    Save `Reserved` (`plan_counter_save` prüft die Reservierung zuerst). Besser:
    „ab dem folgenden Boundary“.
- **Verträglichkeit mit den Nachbarverträgen:**
  - **SIM-03.4b:** Gates unverändert, Release je Spieler, keine Kollision.
  - **NET-04.2d:** §6.5 „kein Input“ bleibt. §6.1a „Terminierung je Ursache“ und
    §10.4 „Höchstens eine Entity-Operation zugleich“ brauchen die in Brief §9
    vorgeschlagenen Notizen; diese sind richtig.
  - **DUR-04.1:** siehe oben.
- **Korrektur:** Den Kopf des Nachtrags um Z. 188, 260-261 und 360-361 sowie die
  DUR-04.1-Stellen ergänzen. Die zwei Formulierungen schärfen.

### A4 (niedrig) — Ehrlichkeit: frühe Meldung und Verbraucherprüfung

- **Frühe Meldung:**
  - Brief §10 „Die Scope-Frage zu Uncertain wurde Root früh gemeldet“ und §11 O1
    „(früh gemeldet)“: Die einzige Meldung im Ersteller-Thread (Position 90,
    `SendMessage` an `main`) endete mit `"success": false`.
  - Eine andere Meldung an Root ist nicht erkennbar. Die Frage erreichte Root
    erst mit dem Abschlussbericht.
- **Verbraucherprüfung:**
  - Brief §10 „Das ist geprüft: … das exhaustive Session-Match bleibt gültig,
    und der e2-Uncertain-Test bleibt unverändert“: Der Ersteller hat weder
    Session noch Sim-Integrationstests ausgeführt.
  - In diesem Review sind beide grün (§1). Die Aussage ist also richtig, war
    aber nicht belegt.
- **Korrektur:** Formulierungen berichtigen und diese Review-Läufe als Beleg
  nennen.

### A5 (niedrig) — Kapazität unvollständig beschrieben

- **Stelle:** Brief §4 Tabelle („bis `limits.pending`“) und Nachtrag Punkt 7.
- **Zusätzlich gebunden bis zum Neustart:**
  - die gemeinsame Pending-Ladung (8 MiB): Eine maximale Entity-Save hat etwa
    1,84 MiB Ladung (DUR-04.2 Z. 216-217), also erschöpfen etwa vier solcher
    Quarantänen die Ladung vor `limits.pending`;
  - beim Pair-Pickup der `PlayerState` des Collectors samt Spieler-Bytes und
    Lifecycle-Zelle (Release `ItemReservation`, Rejoin `Reserved`), was
    `limits.players` belastet.
- **Korrektur:** In Punkt 7 alle gebundenen Ressourcen und die jeweiligen
  typisierten Verweigerungen nennen. Die Eskalationsschwelle bleibt Root bzw.
  DUR-05a vorbehalten (O2).

### A6 (niedrig) — Begründung zu O1 trägt nicht

- **Stelle:** §11 O1, „dauerhafte Uncertainty ist meist Storage-Verfügbarkeit,
  also shardweit“.
- **Gegenbeleg:**
  - Die eigene Kontrolle B6 erzeugt dauerhaftes `Uncertain` durch **eine**
    verworfene Work plus Reconcile (`ReceiptAbsent`). Ebenso der e2-Test.
  - Das ist ein Ereignis an einem einzelnen Item: Ein verlorener Work-Puffer
    reicht, um den Shard nach `defer` Boundaries in Recovery zu bringen.
- **Die Empfehlung bleibt richtig, die Begründung nicht:**
  - V2 ist auf `Uncertain` nicht übertragbar. Ein später erscheinender Receipt
    braucht Reservierung und Projektion; eine Freigabe ermöglichte Writes gegen
    eine womöglich schon eigene, überholte Erwartung.
  - Eine Lösung braucht einen Abbruchbeweis aus dem Storage (z. B. die Query
    storageseitig sperren) oder eine Driver-Politik. Das ist ein eigenes Ticket.
- **Korrektur:** Die Begründung ersetzen. Die e2-Review-Auflage ausdrücklich
  nennen und einem Ticket zuweisen (siehe O1 unten).

### Info

- **I1:** „Typisiert sichtbar, keine stille Pause“ stützt sich auf
  `player_ownership_checkpoint(query).status()`. Die Metrik
  `native_pending_quarantined` ist in NET-04.2d Z. 721 vertraglich, im Code aber
  noch nicht vorhanden (DUR-05a). Bis dahin ist die Sichtbarkeit nur per
  Abfrage gegeben.
- **I2:** Ein dauerhaft sichtbares, nicht aufnehmbares Item erzeugt bei jedem
  Pickup-Versuch einen Pair-Read außerhalb des Ticks und eine Verweigerung. Ein
  künftiger Driver (NET-04.2b/DUR-05a) sollte bei `Ineligible("quarantined item")`
  zurückweichen. Heute ist dasselbe unter der Store-Sperre ebenfalls so, also
  nicht neu.
- **I3:** Im Counter-Modus verweigert V2 Reconcile auch für **nur den Spieler
  betreffende** Quarantänen (CasConflict einer Player-Autosave quarantiniert im
  Counter-Modus ebenfalls, `player_ownership.rs:1618-1622`). Das ist konsistent
  und unschädlich, sollte im Nachtrag Punkt 1 aber ausdrücklich stehen.
- **I4:** `item_fenced` wird in der Save-Auswahl je geprüftem Slot aufgerufen:
  höchstens 256 × 32 Pending-Besuche je Boundary. Das ist begrenzt; die
  Brief-Angabe „je Aufruf ≤ 32“ stimmt.

## 7. Offene Punkte O1–O6: Empfehlung an Root

| Punkt | Einordnung | Empfehlung |
|---|---|---|
| **O1** Uncertain | **Eigenes Ticket und eigene Aktivierungsgrenze**, kein SIM-03.5-Blocker | V2 nicht auf `Uncertain` ausdehnen (I3-Verletzung). Die e2-Review-Auflage („Leave-Save- und L1-Ausgang im Recovery/Degraded-Pfad festlegen“, NET-04.2e2-Review Z. 231) DUR-05a zuweisen. Ein dauerhaft `Uncertain` gebliebenes Item-Pending (verlorene Work) als eigenen Aktivierungsblocker neben R3-A5 führen; Lösung über einen storageseitigen Abbruchbeweis oder eine Driver-Politik. Das R3-A5-Gate gilt mit SIM-03.5 nur für `Quarantined` als erfüllt. |
| **O2** Kapazität | zulässige Folgearbeit (DUR-05a) | Vor Aktivierung eine typisierte Schwelle festlegen (z. B. `Degraded{QuarantineLimit}`) und dabei alle gebundenen Ressourcen aus A5 zählen |
| **O3** Phase B nur über Wachhund | zulässige Folgearbeit (DUR-05a) | Folge im Betrieb: Stop dauert mit Quarantäne mindestens 1200 Recovery-Boundaries ohne Fortschritt (NET-04.2d Z. 1265). Ein lesendes Prädikat „nur Quarantänen blockieren“ ist sinnvoll, wenn Phase B sonst sauber ist |
| **O4** In-Prozess-Auflösung (SIM-03.5b) | optional | Erst nach DUR-05a; braucht frischen Read beider Ressourcen bei Pair-Quarantänen |
| **O5** Restore einer Spawn-UUID | zulässig | R3 bestätigt die Grenze: Bei vorhandenem Körper verweigert `DuplicateUuid`, bei Spawn-Quarantäne (kein Körper) ist ein Restore aus dem Durable-Stand nichts Erfundenes |
| **O6** Client-Sicht | NET-04.2b | plus Rückzugsregel aus I2 für den Driver |

## 8. Empfehlung an Root

1. **CLEAN_CONTRACT:** noch nicht. Eine enge Runde 2 des Erstellers sollte
   beheben:
   - A1: Brief §5.1 Punkt 4 präzisieren, Skizze mit Guard, Legacy-Pflichttest
     mit `recover_player_ownership_completions`;
   - A2: Brief §1 berichtigen, `INVARIANTS.md` in §10 und Wortlaut für Z.
     290-291, 349-350, 366-372, 391-392;
   - A3: Kopf des Nachtrags vervollständigen, die zwei Formulierungen schärfen;
   - A4–A6: Textkorrekturen.

   Der Prototyp-Code braucht außer dem A1-Guard keine Änderung. Prüfen in
   Runde 2: R1 muss auf der korrigierten Skizze „Reservierung bleibt `true`,
   `Reserved`“ liefern.
2. **DUR-04.2-Nachtrag:** inhaltlich richtig (Punkte 1–6), aber erst nach A3
   (und A5 in Punkt 7) übernehmen. Sonst widerspricht er Z. 188, 260-261 und
   360-361. Die NET-04.2d-Notizen aus Brief §9 sind richtig und können mit
   übernommen werden.
3. **Runtime-Freigabe:** nach Runde 2, Scope `server/crates/ferrumc-sim/**`
   einschließlich `INVARIANTS.md`.
   - Pflicht zusätzlich zu M1–M14 und den fünf Brief-§7-Pflichtzeilen: R1
     (Legacy) und R2–R4 dieses Reviews als echte Tests.
   - Die Verbraucher Session, Testkit und App werden erneut ausgeführt.
4. **Aktivierung:** bleibt unabhängig davon an DUR-05a gebunden: O1-Ticket,
   O2-Schwelle und die Metrik aus I1.

## 9. Evidenz

Unter `.codex-tmp/sim03_5-root-review/`; `review.freeze.json` bindet dieses
Dokument und alle Artefakte. Die SHA-Werte werden außerhalb des Dokuments an
Root gemeldet.

| Artefakt | Inhalt |
|---|---|
| `run_probe.py`, `run_repro.sh`, `run_checks.sh` | Runner (Freeze-Prüfung der Kopien), Reproduktion, Gesamtchecks |
| `probes/baseline_probe.rs`, `probes/target_probe.rs`, `probes/make_proto.py` | Kopien der Ersteller-Artefakte (SHA-gleich mit dessen Freeze) |
| `probes/rev035.rs` | eigene Gegenproben R1–R4 |
| `proto.diff` | eigener Diff Basis → Prototyp (gleich der Ersteller-Skizze) |
| `results/*.json`, `logs/*` | Kommandos, Exit-Codes, Logs, Messzeilen |
| `scan_claims.py`, `results/forbidden-claims.json` | Forbidden-Claims-Scan |
| `final-assessment.json` | Verdikt und Befunde maschinenlesbar |
