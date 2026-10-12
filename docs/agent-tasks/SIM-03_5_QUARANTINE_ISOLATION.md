# SIM-03.5 — quarantinierte Item-Pendings vom Shard isolieren: konkreter Vertrag

Design-Subagent, 12.10.2026, **Runde 2**. Auftrag:
[SIM-03_5_QUARANTINE_ISOLATION_TASK.md](SIM-03_5_QUARANTINE_ISOLATION_TASK.md).
Ursprung: R3-A5 in [NET-04.2d Runde 3](NET-04_2d_NATIVE_DRIVER_ENDPOINT_REVIEW_ROUND3.md)
und [Root-Entscheidungen](NET-04_2d_NATIVE_DRIVER_ENDPOINT_ROOT_DECISIONS.md),
Befund §8.3 der [NET-04.2e2-Delivery](NET-04_2e2_NATIVE_SESSION_LIFECYCLE_DELIVERY.md).
Runde 2 setzt die Befunde A1–A6 des
[unabhängigen Reviews](SIM-03_5_QUARANTINE_ISOLATION_REVIEW.md) und die
Root-Entscheidungen dazu um. §12 ordnet jede Änderung ihrer Stelle zu. Der
Runde-1-Kandidat liegt unverändert unter `.codex-tmp/sim03_5/round1-candidate/`.
Die Produktion blieb readonly. Alle Proben liefen ausschließlich in privaten
Archivkopien unter `.codex-tmp/sim03_5/`.

Dieser Vertrag nimmt weder Aktivierung noch Survival, Paper-Parität, Plugins
oder Performance ab.

## 0. Ergebnis in Kürze

- **Heutige Fehlwirkung, eigene Probe belegt (B1–B9, Memory und redb):** Ein
  quarantiniertes Entity-Pending hat **zwei** shardweite Folgen.
  - Es zählt weiter `claims` und führt nach genau `defer − claims` weiteren
    Normal-Boundaries dauerhaft zu `CounterRecoveryRequired` (R3-A5).
  - Schon vorher hält es dauerhaft die **ganze Store-Reservierung**. Dann sind
    kein Item-Save, kein Pickup, kein Spawn und kein Restore im Shard mehr
    möglich, und Phase B kann kein anderes Item flushen.
  - Zusätzlich stuft ein `Reconcile` eine CasConflict-Quarantäne heute zu
    `Uncertain` herab (B7).
- **Begründung der heutigen Regel:** Die Projektion schützt bei *lösbaren*
  Pendings die einmalige Tickzählung und die ehrliche Weltzeit („kein stilles
  Pausieren“). Ein quarantiniertes Pending hat im Prozess aber keinen
  Veröffentlichungs- oder Freigabepfad mehr. Die Projektion hat damit keinen
  Konsumenten, und ihr einziger Effekt ist die Cap-Eskalation. Vanilla und Paper
  kennen keine Projektion (Source-Review, §3.3).
- **Gewählt: V2, typisierte Per-Item-Abschirmung („Fence“), nur im
  Counter-Modus** (`item_counters.is_some()`). Im DUR-04.1-Legacymodus ändert
  sich nichts (Root-Entscheidung 2, §5.1).
  - Der Plan bindet das Pending nicht mehr: keine Claims, kein Schritt, die
    Projektion bleibt eingefroren.
  - Am Ende des Quarantäne-Boundaries endet die Store-Reservierung.
  - Das betroffene Item ist für Save, Pickup und Auswahl gesperrt.
  - `Reconcile` auf `Quarantined` wird typisiert verweigert.
  - Die Änderung umfasst nur `ferrumc-sim`: zwei Quelldateien (+83/−6 Zeilen in
    der Runde-2-Skizze) und `INVARIANTS.md` (§5.4). Es gibt keinen neuen Status,
    keine neue Fehlervariante und kein neues Feld; Storage und Session bleiben
    unberührt.
- **Prototyp in privater Kopie (Runde 2):**
  - T1–T13 (14 Tests) auf Memory und redb grün; alle 35 Messzeilen sind
    zeilengleich mit Runde 1, in debug und release.
  - Neuer Legacy-Pflichttest T14 (Muster R1): Die Reservierung bleibt
    bestehen, der veraltete Pickup wird mit `Reserved` verweigert, 0 Work. Der
    Test besteht auf der Basis und auf Runde 2; die Runde-1-Skizze fällt
    erwartungsgemäß durch.
  - Die Gegenproben R1–R4 des Reviewers laufen auf Runde 2 grün. R1 zeigt jetzt
    das Basisverhalten, R2–R4 sind zeilengleich mit dem Reviewer-Prototyp.
  - Volle Suiten ohne Probe-Module: `ferrumc-sim` alle Targets 404/404,
    `ferrumc-session` 244/244, `ferrumc-testkit` 100/100, `ferrumc-app`
    375 bestanden, 0 fehlgeschlagen, 1 ignoriert (bestehend; braucht Java und
    Maven-Fixtures), 40 Testbinaries, ohne Wiederholungslauf; Clippy `-D warnings` frisch nach `cargo clean -p ferrumc-sim` sauber;
    `cargo fmt --all --check` grün.
- **Auflösung:** nur durch Neustart. Stop endet typisiert `Unresolved`, danach
  gilt der Durable-Stand (T13). Bis dahin bleiben die in §6.1 aufgezählten
  Ressourcen gebunden. Eine In-Prozess-Übernahme bleibt ein optionales
  Folgeticket.
- **Nicht gelöst und bewusst abgegrenzt:** der dauerhaft unsichere Pickup
  (`Uncertain`, NET-04.2e2 §8.3). Dafür legt Root ein eigenes Ticket mit eigener
  Aktivierungsgrenze an; V2 wird nicht auf `Uncertain` ausgedehnt (§11).

## 1. Basis, Pins und Leseumfang

- **Code:** Root-Archiv des Main-Commits `f1e7511cb109f67bb52ca93f3a04a4e5cfce6b13`
  unter `.codex-tmp/sim03_5-root-base/archive` (1390 Dateien). Das Manifest
  `base.freeze.json` hat SHA-256
  `b715025e50153abc6281e3c6d88188f4dd9233e9fe499fb2db6ce12dd9094356` und wurde vor
  jeder Probe gegen die private Kopie geprüft. Alle Zeilenanker unten beziehen
  sich auf dieses Archiv.
  - **Hinweis:** Die R3-A5-Anker der Runde 3 (`player_ownership.rs:1610-1614`)
    liegen auf f1e7511c bei `1618-1622`. Die übrigen genannten Anker stimmen.
- **Verträge vollständig gelesen:**
  - DUR-04.1 ([Brief](DUR-04_1_PLAYER_ENTITY_CONSUMER.md), Review) und DUR-04.2
    (Brief, Review), insbesondere DUR-04.2 §3, §7 „Strukturreservation“,
    „Duegrenze“, „Savefortsetzung“, §8 und §9 Z. 595–615 sowie DUR-04.1 §3 und
    §6;
  - SIM-03.4a/b/c (Delivery, Reviews);
  - NET-04.2d §6.1/§6.1a/§6.5/§7.3/§7.4/§10.4/§15 samt Root-Entscheidungen;
  - NET-04.2e2 (Delivery §5/§8, Review §7);
  - AGENTS.md, CLAUDE.md.
- **`server/crates/ferrumc-sim/INVARIANTS.md` existiert und ist verbindlich.**
  Sie hat 560 Zeilen; AGENTS.md Z. 31 verlangt, sie zu lesen, Z. 126, sie bei
  neuen Regeln zu aktualisieren. In Runde 1 hatte ich das Gegenteil behauptet;
  das war falsch (Review A2). V2 ändert vier ihrer Regeln; den genauen neuen
  Wortlaut gibt §5.4 vor.
  - Die richtigen Anker auf f1e7511c sind Z. 290–294, 347–351, 366–372 und
    384–389. Das Review nennt für die zweite und vierte Stelle Z. 349–350 und
    391–392; dort steht jeweils der Folgetext derselben Regel.
- **Runtime gelesen:**
  - `shard/player_ownership.rs`, `shard/player_ownership/{counters,release,publications}.rs`;
  - `entity/{counters,ownership}.rs`, `entity.rs` (`reserve_spawn_slot`),
    `item_counters.rs`, `item_publication.rs`, `collector_lifecycle.rs`;
  - `player/{pending,evidence,detached}.rs`;
  - `ferrumc-storage/src/ownership/engine.rs` (Receipt-Replay vor CAS:
    `prepare` 524–536, `check_cas` 374–389);
  - `ferrumc-session/src/native/retention.rs` (exhaustives Match auf
    `OwnershipPendingStatus`, 583–589).
- **Originalquellen:** Paper 1.21.8 Build 60, Commit
  `29c8822d90899c89d2689338e81a98f690bcba12`, dieselben Pins wie DUR-04.2.
  `.codex-tmp/dur04_2-reference.sha256` wurde für alle drei ItemEntity-Dateien
  erneut mit `OK` geprüft:

  | Datei | SHA-256 |
  |---|---|
  | Vanilla 1.21.8, dekompiliert (mache base): `.gradle/caches/paperweight/mache/base/sources/net/minecraft/world/entity/item/ItemEntity.java` | `64e95726…69d4` |
  | Paper angewandt: `src/minecraft/java/net/minecraft/world/entity/item/ItemEntity.java` | `0e01331a…3bad` |
  | Paper-Patch: `patches/sources/net/minecraft/world/entity/item/ItemEntity.java.patch` | `877b1038…85b6` |
  | `patches/features/0003-Entity-Activation-Range-2.0.patch` | `64845ef6…d64b` |

  Ergänzend wurde das Vanilla-`server.jar` gepinnt: SHA-1
  `6bce4ef400e4efaa63a13d5e6f6b500be969ef81` (gleich `version.json`
  `downloads.server.sha1`), SHA-256 `2349d9a8…fbbe7`.

### 1.1 Was Runde 1 belegt hatte und was nicht (Review A4)

| Aussage in Runde 1 | Stand |
|---|---|
| B1–B9, T1–T13 auf Memory und redb, debug und release | belegt (Runde-1-Freeze; vom Reviewer zeilengleich reproduziert) |
| 266 `ferrumc-sim`-Lib-Tests grün, Clippy sauber | belegt, aber **nur** die Lib-Tests |
| „Storage und Session sind nicht betroffen. Das ist geprüft“ | **nicht belegt.** Ich hatte weder Session noch die 127 Sim-Integrationstests ausgeführt. Die Aussage war richtig; belegt haben sie erst das Review (Session 244/244, Sim 404/404) und jetzt Runde 2 (§7, M14) |
| „Die Scope-Frage zu Uncertain wurde Root früh gemeldet“ | **falsch.** Meine einzige Meldung war ein `SendMessage` an `main` (Runde-1-Thread, Position 90). Er endete mit `"success": false` („You are the main conversation“). Die Frage erreichte Root erst mit dem Abschlussbericht |
| „`ferrumc-sim` hat keine eigene `INVARIANTS.md`“ | **falsch** (siehe oben) |
| Die Skizze „gilt nur im Counter-Modus“ | **für einen Pfad falsch** (Review A1, §5.1 Punkt 4) |

Fragen melde ich künftig nicht per `SendMessage`. Ich beende den Lauf mit der
Frage als Ergebnis; nur das erreicht Root zuverlässig.

## 2. Heutige Mechanik (f1e7511c)

1. **Quarantäne entsteht** in `complete_owned` (`player_ownership.rs:1594-1658`):
   - Im Counter-Modus setzt `Refused(CasConflict)` den Status `Quarantined`
     (`1618-1622`, `quarantine_counter_conflict` `counters.rs:437-449` setzt die
     Projektion auf die Original-Counter zurück).
   - Ein echter Receipt, dessen Current-Read `confirm_current` verfehlt
     (Superseded, Mismatch, Epochen), führt in **beiden** Modi zu `Quarantined`
     (`1645-1647`). Dann bleibt die Projektion unverändert.
   - Ein `CasConflict` ist definitiv: Storage prüft den Receipt vor der CAS
     (`engine.rs:533-536`) und verweigert vor jedem Commit. Für diese Query gibt es
     also nie einen Receipt.
2. **Kein Ausgang im Prozess:**
   - Eine Completion verlangt `InFlight` und die eigene Store-Reservierung
     (`validate_completion` `1423-1464`).
   - Ein Retry verlangt `Prepared` (`dispatch_retained` `1401-1403`).
   - Ein Reconcile ist zugelassen (`1381-1421`). Er liefert für einen CasConflict
     `ReceiptAbsent` und damit `Uncertain` (B7), für Superseded erneut
     `Quarantined` (hergeleitet aus `confirm_current`, nicht geprobt).
   - Die Projektion wird nicht persistiert; Restore liest den Durable-Stand
     (DUR-04.2 Z. 132–134).
3. **Projektion zählt weiter:**
   - `plan_counter_boundary` bindet jedes Pending mit `counters` ohne
     Statusfilter (`counters.rs:494-501`).
   - `step_boundary_entry` erhöht `claims` je residentem Normal-Boundary und
     verweigert bei `claims > defer` mit `CounterRecoveryRequired`
     (`2082-2113`, Rückschrieb `1781-1807`).
4. **Store-Reservierung bleibt:**
   - `install_candidate` reserviert den ganzen Entity-Store (`1303-1308`,
     `entity/ownership.rs:270-281`). Freigegeben wird nur durch Veröffentlichung
     oder `release_pending`.
   - Solange eine Reservierung besteht, gilt:
     - `require_owner` liefert `Reserved` (`entity/ownership.rs:89-100`);
     - der Plan nimmt keinen Pickup, Spawn, Drop, Save oder Restore an
       (`counters.rs:712-715`);
     - die Save-Auswahl kehrt sofort zurück (`1884-1886`).
5. **Modus der Einstiege:** `run_tick_with_player_ownership_at` verlangt Counter
   (`counters.rs:155-170`), `run_tick_with_player_ownership` verlangt das
   Gegenteil (`player_ownership.rs:291-298`). `recover_player_ownership_completions`
   (`counters.rs:237-296`) prüft den Modus **nicht** und läuft auch auf einem
   Legacy-Owner. Jeder V2-Haken dort braucht deshalb einen eigenen
   Counter-Modus-Guard (Review A1).

**Eigene Probe** (`.codex-tmp/sim03_5/probes/baseline_probe.rs`, nur als
`cfg(test)`-Modul in der Kopie). Alle Zeilen gelten für Memory und redb
identisch, in debug und release; die Werte sind gemessen.

| ID | Fall | Ergebnis heute |
|---|---|---|
| B1 | CasConflict-Save, `defer` 8 und 1024 | Nach dem Quarantäne-Boundary `claims` = 2, danach genau 6 bzw. 1022 Normal-Boundaries `Ok`, dann `CounterRecoveryRequired`. 3 Zyklen Recovery + Normal bleiben `Err`. Sichtbare Counter eingefroren. |
| B2 | Superseded (Commit, verlorene Antwort, Fremdschreiber, Reconcile) | `claims` = 3, danach 5 `Ok`, dann `CounterRecoveryRequired` |
| B3a | Pair-Pickup-Quarantäne, Disconnect vor Recovery | Disconnect angewandt, `blocking = (id, Quarantined)`; danach 5 `Ok`, dann Recovery |
| B3b | dasselbe, Disconnect nach Recovery | Disconnect bleibt 20 Zyklen in der Queue, Status `Ineligible("connected player")`: Leave-Save-Quarantäne unerreichbar |
| B4 | Store-Reservierung, `defer` 1024 | 200 Boundaries ohne Save-Work für ein anderes dirty Item. Dessen Durable-Stamp bleibt unverändert, expliziter Save → `Reserved`, Restore eines neuen Items → `Reserved` |
| B5 | Rand `defer` = 8, Completion mit 6/7/8 Claims | 6 → Quarantäne bei 7, noch 1 `Ok`. 7 → bei 8, 0 `Ok`. 8 → der Completion-Boundary verweigert vor dem Drain (Status `InFlight`, Completion bleibt), Recovery quarantiniert, danach 0 `Ok`. Immer dauerhaft Recovery |
| B6 | Kontrolle Uncertain (`ReceiptAbsent`) | `claims` = 3, 5 `Ok`, dann `CounterRecoveryRequired` (der e2-Fall). Ausgelöst durch **eine** verworfene Work plus Reconcile |
| B7 | Reconcile auf CasConflict-Quarantäne | Work wird erzeugt, Status `InFlight`, nach der Completion `Uncertain` (Fehlklassifikation) |
| B8 | Phase-B-Modell (Drain plus 40 Recovery-Boundaries) | 0 Save-Work, anderes Item bleibt dirty, `owned_stop_ready` = false |
| B9 | unbeteiligter Spieler nach Recovery | Disconnect bleibt in der Queue, Release → `DetachedRelease(Connected)` |

## 3. Begründung der heutigen Regel (Frage 1)

### 3.1 Warum DUR-04.2 die Projektion weiterrechnet

DUR-04.2 baut die Regel in drei Schritten auf:

- **§3 Z. 83–91:** Eine dauerhafte Pause während eines DB-Pendings wäre eine
  „explizite Gameplayabweichung, keine native Timerparität“. Deshalb laufen
  Timer reservierter Items privat weiter.
- **§7 „Strukturreservation“ Z. 268–282:** `DeferredOwnedCounters` projiziert
  aus dem gefrorenen Postimage. Bei Bestätigung wird die Projektion auf den Rest
  angewandt, bei einer definitiven Ablehnung auf das Original-Live-Item.
  „Conflict/Superseded bleibt quarantined und ermächtigt keine neue CAS gegen
  erfundene Erwartungen.“
- **§9 Z. 604–606** präzisiert für CasConflict: Pending und sichtbarer Körper
  bleiben strikt unverändert, „nur private ursprüngliche Counterprojection wird
  weitergerechnet“, ohne neue Retry-Autorität und ohne Refund.

Die Projektion hält also bereit, was das Original-Item ohne Reservierung gezählt
hätte, für den Fall einer späteren Auflösung. Die Cap `defer ≤ 1024` begrenzt diese
unbestätigte Fortschrittsautorität (§7 Tabelle, Z. 205–207). Darüber hinaus wird
der Shard bewusst recovery-required statt still zu stehen (§7 Z. 320–328).

### 3.2 Geschützte Invarianten und ihre Erreichbarkeit bei `Quarantined`

| Invariante | Schützt die Projektion sie bei einem *quarantinierten* Pending? |
|---|---|
| I1 einmalige Tickzählung (Tickidentität, `first/last`, keine Doppelzählung) | Nein. Die Projektion wird nie veröffentlicht (§2 Punkt 2) und nach dem Neustart nicht gelesen. Ohne weitere Claims ist I1 trivial erfüllt. |
| I2 reservierter Postimage und sichtbarer Körper unverändert | Unabhängig von der Projektion; der sichtbare Körper wird nie gestept |
| I3 kein erfundener Zustand, keine CAS gegen eine überholte Erwartung | Unabhängig; verlangt nur, dass weder Save noch Pickup noch Reconcile neue Autorität schafft |
| I4 begrenzte unbestätigte Fortschrittsautorität (≤ `defer`) | Die Projektion *erzeugt* hier die Verletzung, die die Cap dann eskaliert. Ohne Zählung bleibt sie trivial begrenzt. |
| I5 ehrliche Weltzeit („kein stilles Pausieren“, §7 Z. 326–328) | Der einzige echte Inhalt. Er verlangt, dass ein erfolgreicher Boundary nicht behauptet, ein Item getickt zu haben, das nicht tickt. Eine **typisierte, sichtbare** Abschirmung erfüllt das ebenso (§5.2). |

Fazit: Für `Uncertain`, `NeedsRefresh`, `InFlight` und `Prepared` hat die
Projektion einen erreichbaren Konsumenten, nämlich die spätere Veröffentlichung
oder Freigabe. Dort bleibt die Regel unverändert. Für `Quarantined` hat sie keinen
Konsumenten mehr.

### 3.3 Originalsemantik (Source-Review mit Pins, kein Differenziallauf)

- **Vanilla 1.21.8** (mache base, `64e95726…`):
  - `ItemEntity.tick()` ab Z. 117: `age++` außer `-32768` (Z. 172–173);
    Despawn bei `age >= 6000` durch `discard()` (Z. 184–185).
  - Persistenz schreibt den Live-Wert: `addAdditionalSaveData` `putShort("Age")`
    Z. 309; Lesen in Z. 321.
- **Paper Build 60** (angewandt, `0e01331a…`; Patch `877b1038…`; EAR
  `64845ef6…`):
  - `inactiveTick()` Z. 134–153: `pickupDelay--`, `age++`, Despawn bei
    `age >= despawnRate` mit `ItemDespawnEvent`; ein Cancel setzt `age = 0`.
  - `tick()` Z. 156–237 mit `age++` (Z. 215–216) und Despawn Z. 227–235.
  - Speichern `addAdditionalSaveData` Z. 375–378 (`Age` als `short`), Lesen
    Z. 392–394.
  - Ob ein Entity aktiv oder inaktiv tickt, entscheidet
    `ActivationRange.checkIfActive` (EAR-Patch).
- **Folgerung:**
  - Das Original kennt keine Persistenz-Pendings und keine Projektion. Ein
    getickter Item-Körper altert einmal je Tick und wird bei der Schwelle
    entfernt. Gespeichert wird, was der Live-Körper beim Save hat.
  - Für ein quarantiniertes Item würde die Originalsemantik Weiteraltern **und
    Despawn** verlangen. Im Prozess ohne frischen Read bräuchte der Despawn eine
    Delete-CAS gegen die überholte Erwartung, die DUR-04.2 Z. 282 und 606
    verbietet. Die Alternative wäre die In-Prozess-Übernahme (§11 O4).
  - Weder die heutige Regel noch V2 liefert daher Originalparität. Die heutige
    Projektion nähert sie nicht einmal an, weil sie nie veröffentlicht wird.
  - Die Originalsemantik verlangt die Projektion also **nicht**. Die Abweichung
    wird in beiden Fällen deklariert.

## 4. Varianten (Frage 2)

| | V1 Projektion einfrieren | **V2 typisierter Per-Item-Fence (gewählt)** | V3 Pending auflösen oder Körper verwerfen | V4 eigene Quarantänetabelle |
|---|---|---|---|---|
| Kern | Plan bindet `Quarantined` nicht, keine Claims, Körper nicht gestept; Reconcile-Verweigerung nötig, sonst bindet `Uncertain` wieder | V1 plus Reservierungsende am Boundaryende plus Fence für Save, Pickup und Auswahl, alles nur im Counter-Modus | wie eine definitive Ablehnung freigeben oder den Live-Körper entfernen | V2 plus Umzug in eine getrennte Tabelle mit eigener Cap |
| `CounterRecoveryRequired` durch Quarantäne | weg | weg | weg | weg |
| Item-Pfad des Shards | **bleibt gesperrt** (B4/B8: kein Save, Pickup, Spawn oder Restore; Phase B flusht nichts) | frei für alle anderen Items (T4, T9, T11, T12) | frei | frei |
| Mehrere Quarantänen | höchstens eine (Store gesperrt) | bis zur ersten erschöpften Grenze aus §6.1: `limits.pending` (T6), Pending-Ladung, bei Pair-Pickups Spielerplätze und Publikationskredite | — | eigene Cap |
| I3 kein erfundener Zustand | ja | ja | **nein**: neue CAS-Autorität bzw. Refund gegen eine überholte Erwartung; bei Superseded ist der Pair-Commit bereits erfolgt | ja |
| Umfang | ca. 25 Zeilen | Skizze +83/−6 Zeilen, 2 Dateien, plus `INVARIANTS.md` | gering, aber vertragswidrig | Checkpoint-, Stop-, Release- und Session-`blocking`-Pfade ändern sich |

**Wahl V2** (Root-Entscheidung 1 bestätigt sie).

- V1 erfüllt nur den Wortlaut „kein `CounterRecoveryRequired`“. Ein einzelnes
  Item würde den Item-Pfad des ganzen Shards aber dauerhaft sperren, also genau
  das, was R3-A5 ausschließen soll.
- V3 verletzt DUR-04.2 Z. 282 und 606.
- V4 bringt nur einen Kapazitätsgewinn, verändert aber von der Session gelesene
  Pfade. Sie bleibt eine Option (§11 O2).

## 5. Konkreter Vertrag V2

### 5.1 Funktionen und Felder

**Alle V2-Pfade gelten ausschließlich im Counter-Modus
(`item_counters.is_some()`). Im DUR-04.1-Legacymodus ändert sich nichts**,
auch nicht über die modusunabhängige `recover_player_ownership_completions`
(Root-Entscheidung 2, Review A1). Anker beziehen sich auf f1e7511c. Die
Referenzskizze `.codex-tmp/sim03_5/round2/v2-reference-sketch-r2.diff` (aus
`round2/make_proto_r2.py`) zeigt genau diese Punkte; sie ist Evidenz, kein
Produktionscode. Gegenüber Runde 1 ändern sich nur die Punkte 3 und 4.

| V2-Pfad | Warum er nur im Counter-Modus wirkt |
|---|---|
| `plan_counter_boundary`, `plan_counter_actions` (Punkte 5, 6) | nur aus `run_tick_with_player_ownership_at` erreichbar, das ohne Counter `NotInitialized` liefert (`counters.rs:162-166`) |
| `plan_counter_save` (Punkt 7) | verlangt `item_counters` (`counters.rs:1814-1818`) |
| `select_counter_save` (Punkt 8) | kehrt ohne `item_counters` zurück (`counters.rs:1887-1893`) |
| `item_fenced`, `require_unfenced_item` (Punkte 2, 3, 9) | `item_fenced` prüft `item_counters.is_some()` selbst; `require_unfenced_item` meldet ohne Abschirmung nie einen eigenen Fehler |
| `dispatch_retained`, Reconcile-Verweigerung (Punkt 10) | prüft `item_counters.is_some()` |
| `release_quarantined_reservation` (Punkt 4) | **Runde 2:** kehrt ohne `item_counters` zurück. Nötig, weil eine Aufrufstelle die modusunabhängige Recovery ist |

1. **`fn fences_item(p: &PendingPlayerMutation) -> bool`** (neu, privat, `counters.rs`):
   `p.status == Quarantined` und `p.after_entity` ist `Replace` oder `Remove`,
   benennt also ein vorhandenes Item.
2. **`SimShard::item_fenced(&self, uuid: EntityUuid) -> bool`** (neu,
   `pub(super)`): Counter-Modus und ein Pending mit `fences_item` und
   `entity_expected.key().uuid() == uuid`. Je Aufruf werden höchstens
   `limits.pending` (≤ 32) Einträge durchsucht, in der Save-Auswahl also höchstens
   256 × 32 Pending-Besuche je Boundary. Kein neuer Speicher; die Abschirmung wird
   aus dem einzigen Statusfeld abgeleitet.
3. **`SimShard::require_unfenced_item(&self, handle) -> Result<(), PlayerOwnershipError>`**
   (neu): `Err(Ineligible("quarantined item"))` für ein abgeschirmtes Item.
   - **Runde 2:** Der Handle wird nur aufgelöst, um nach einer Abschirmung zu
     suchen. Ein unbekannter Handle ergibt `Ok(())`. Der Aufrufer meldet dann
     seinen eigenen Fehler, und die bisherige Verweigerungsreihenfolge bleibt
     außerhalb einer Abschirmung unverändert. In Runde 1 lieferte schon der
     Lookup einen `EntityStore`-Fehler, auch im Legacymodus.
4. **`SimShard::release_quarantined_reservation(&mut self)`** (neu): ruft für jedes
   Pending mit `Quarantined` und `after_entity.is_some()`
   `EntityStore::release_owned(id)` auf (No-op, falls es nicht der Halter ist).
   - **Runde 2: Ohne `item_counters` kehrt die Funktion sofort zurück.** Ein
     Legacy-Owner behält seine Reservierung bis zu passender Evidenz, also bis
     zum Neustart (T14).
   - Aufruf am Ende von `run_tick_with_player_ownership_at`, nach
     `apply_counter_boundary` und `restore_counter_scratch`, vor
     `select_counter_save` (`counters.rs:221-231`).
   - Aufruf am Ende von `recover_player_ownership_completions`, vor
     `select_counter_save` (`294`). Diese Stelle ist auch im Legacymodus
     erreichbar; der Guard in der Funktion deckt sie ab. Ein Guard nur an den
     Aufrufstellen wäre gleichwertig, aber für künftige Aufrufer fehleranfällig.
   - **Nicht** sofort in `complete_owned`. Sonst würde die FIFO-Anwendung
     desselben Boundaries Aktionen zulassen, die der vorab berechnete Plan als
     reserviert verweigert hat (DUR-04.2 §8, gleiche Eligibility in Plan und
     Apply). T11 belegt die Übereinstimmung.
5. **`plan_counter_boundary`**, Bindungsschleife (`494-501`): Für `fences_item(p)`
   setzt der Plan `entry.resident = false`, statt `bind_projection` aufzurufen.
   - `resident` ist das vorhandene Eligibility-Bit des Plans. Gelesen wird es nur
     von der Step-Schleife (`680-685`), dem Schrittzähler (`213-220`), dem
     Pickup-Schatten (`1051`) und `apply_counter_boundary` (`1782`). Stop löscht
     es heute schon für alle Einträge (`675-679`).
   - **Kein neues Feld:** Ein zusätzliches `bool` in `CounterBoundaryEntry`
     vergrößert den Eintrag nachgemessen auf 144 B. Der bestehende Test
     `actual_scalar_shape_fits_positive_nine_mib_cap` schlägt dann fehl
     (9 451 272 B > 9 437 184 B). DUR-04.2 Z. 542–544 verlangt dafür eine
     Root-Meldung; V2 vermeidet das.
   - Der Quarantäne-Boundary selbst bleibt wie heute: höchstens ein Claim
     (`523-536`). Überschreitet er `defer`, verweigert der Normal-Boundary vor dem
     Drain, und der folgende Recovery-Boundary wendet die Completion an (T5).
6. **`plan_counter_actions`:**
   - `reserved` (`712-715`) zählt Pendings mit `Quarantined` nicht.
   - Der `EntitySave`-Arm (`986`) verweigert bei `self.item_fenced(entry.uuid)`.
   - Der `Retry`/`Reconcile`-Arm (`1529-1534`) verweigert bei `Quarantined`.
   - Der Pickup-Schatten verweigert bereits über `!entry.resident`.
7. **`plan_counter_save`** (`1809ff`): `require_unfenced_item(handle)?` nach der
   Reservierungsprüfung und vor der Durability-Prüfung (gilt für den Input
   `EntitySave` und die Auswahl). Im quarantinierenden Boundary selbst greift
   deshalb noch `Reserved`, ab dem folgenden Boundary `Ineligible("quarantined item")`.
8. **`select_counter_save`** (`1880ff`): Ein abgeschirmter Slot wird nach dem
   Durability-Lookup übersprungen. Er gilt nicht als `needs`, der Cursor läuft
   weiter, es entsteht kein `ItemSaveBlocked`-Intent.
9. **`apply_owned_kind`**, Pickup-Arm (`player_ownership.rs:519-533`):
   `self.require_unfenced_item(item)?` vor `plan_owned_pickup`.
   - Bewusst nicht *in* `plan_owned_pickup`: Diese Funktion hat genau 100 Zeilen,
     und jede zusätzliche Zeile scheitert an `clippy::too_many_lines`
     (nachgemessen).
   - Der Fence greift vor der Stamp-Prüfung. Ein veralteter, vor dem Konflikt
     gelesener Pair-Read, der zur alten Erwartung passt, wird deshalb typisiert
     verweigert statt eine zweite CAS-Kollision zu erzeugen (T3b).
   - Dieser Arm ist auch im Legacymodus erreichbar. Dort gibt es keine
     Abschirmung, und der Aufruf ist wirkungslos (Punkt 3, T14).
10. **`dispatch_retained`** (`1381-1421`): nach der Lease-Prüfung
    `clarify && counter mode && Quarantined` → `Err(Ineligible("quarantined pending"))`.
    - Das gilt für **jede** Quarantäne im Counter-Modus, auch für reine
      Player-Quarantänen (CasConflict einer Player-Autosave oder eines
      Bootstraps, `player_ownership.rs:1618-1622`; Review I3).
    - Retry bleibt unverändert über das `Prepared`-Gate verweigert
      (`UnprovenWorkerLifetime`). Der bestehende Test
      `real_cas_conflict_retains_quarantined_native_body_and_never_creates_retry_authority`
      bleibt deshalb unverändert.
    - Im Legacymodus bleibt Reconcile auf `Quarantined` zugelassen (T14:
      1 Work, Status `InFlight`, wie auf der Basis).

**Unverändert bleiben:**

- `OwnershipPendingStatus`: die Session matcht exhaustiv; nicht
  `#[non_exhaustive]`;
- `PlayerOwnershipError` (nutzt das vorhandene `Ineligible(&'static str)`);
- `CounterBoundaryEntry`, `DeferredOwnedCounters`;
- `quarantine_counter_conflict`, `complete_owned`, `owned_state_quiescent`,
  `check_detached_release`;
- der gesamte Legacypfad;
- Storage, Session, App, Cargo.

### 5.2 Erhaltene Invarianten

- **I1:** Ein abgeschirmtes Item claimt 0 weitere Ticks. Seine Projektion wird
  nie angewandt (T1, T2: `claims` und Projektion bleiben gleich). Nach dem
  Neustart gilt der Durable-Stand (T13).
- **I2:** Der sichtbare Körper wird nie gestept (T1: Counter unverändert), und das
  Pending-Postimage bleibt unberührt.
- **I3:** Keine neue CAS gegen die überholte Erwartung. Save und Auswahl werden
  verweigert (T4), ebenso Pickup (T3b) und Reconcile (T7); Retry bleibt
  verweigert (T7). Es gibt keinen Refund und kein Verwerfen. Der vorbereitete
  `used`/`bucket`-Schnappschuss des Pendings wird nie angewandt:
  `validate_completion` verlangt `InFlight` und `check_reserved(id)`, was nach
  der Freigabe scheitert.
- **I4:** Für alle nicht quarantinierten Pendings bleibt `claims ≤ defer`
  unverändert (T8 Kontrolle). Quarantänen sind durch die Grenzen aus §6.1
  begrenzt (T6).
- **I5:** Ehrliche Weltzeit. Für das abgeschirmte Item meldet der Boundary keinen
  Schritt (`counter_steps` zählt es nicht). Die Abweichung ist deklariert und
  typisiert per Checkpoint abfragbar (§6). Die Metrik
  `native_pending_quarantined` ist in NET-04.2d vertraglich, im Code aber noch
  nicht vorhanden (Review I1, DUR-05a).
- **I6:** Höchstens eine **aktive** Entity-Operation je Shard, also ein nicht
  quarantiniertes Pending mit Store-Reservierung.
- **I7:** Plan und Apply bleiben gleich (DUR-04.2 §8): Fence und `reserved`
  werden aus dem Status bei Planbeginn abgeleitet, die Freigabe erfolgt erst am
  Boundaryende (T11). Ohne die Reconcile-Verweigerung liefen beide auseinander
  (Review §2 Punkt 3). Alle bestehenden Plan- und Grenztests bleiben unverändert
  grün.
- **I8:** Mengenerhaltung. Ein quarantiniertes Pending veröffentlicht und
  entfernt nichts. Nach dem Neustart enthält das Spielerinventar genau den
  Durable-Stand (T13: 0 Items, weil der Pair-Commit nie stattfand).
- **I9 (Runde 2):** Der Legacymodus ist unverändert. T14 liefert auf Basis und
  Runde-2-Prototyp dieselben Messzeilen: Reservierung nach Recovery und
  Legacy-Tick weiter `true`, veralteter Pickup `Reserved` mit 0 Work, Reconcile
  zugelassen. DUR-04.1 §3 („während der Entityreservation verweigern alle
  konkurrierenden Storewriter“) und Sim-`INVARIANTS.md` Z. 290–291 gelten dort
  wörtlich weiter.

### 5.3 Wirkung auf die angrenzenden Verträge

- **`owned_stop_ready` (SIM-03.4b):** Das Prädikat bleibt unverändert. Mit
  mindestens einem quarantinierten Pending ist es dauerhaft `false`, weil
  `pending` nicht leer ist. Ohne Quarantäne gilt das bisherige Verhalten
  (T9-Kontrolle: `true`, Stop angenommen).
- **Release (SIM-03.4b):**
  - Unbeteiligte Spieler lassen sich nach Detach und Save freigeben, auch
    `defer + 16` Boundaries nach der Quarantäne (T10). Heute ist das nach dem
    Recovery-Eintritt unmöglich (B9).
  - Der Collector eines quarantinierten Pickups bleibt typisiert gesperrt
    (`DetachedRelease(ItemReservation)`, bei animiertem Take vorher
    `PublicationPin`; T3). Die Gates sind unverändert. Die Bezeichnung
    `ItemReservation` passt nicht mehr wörtlich, weil das Pending nach dem
    Quarantäne-Boundary keine Store-Reservierung mehr hält; das Gate prüft aber
    `after_entity` und nicht die Reservierung.
- **Rejoin:** Der Collector wird mit `Reserved` verweigert (T3); andere Spieler
  sind unberührt.
- **Leave-Save-Quarantäne (NET-04.2e2, §7.3):**
  - Normal-Boundaries laufen weiter, also wird der Disconnect des Collectors
    angewandt, und der Status meldet `blocking = (id, Quarantined)` (T3, nach
    `defer + 16` Boundaries). Die Session quarantiniert dann sofort
    (§7.3 Punkte 5/6). Heute ist das nach dem Recovery-Eintritt unerreichbar (B3b).
  - Leave-Saves anderer Spieler laufen normal.
  - Für einen dauerhaft *unsicheren* Pickup ändert SIM-03.5 nichts (§11 O1).
- **Recovery-Modus und Degraded (NET-04.2d §6.1a):**
  - Eine Quarantäne löst kein `CounterRecoveryRequired` mehr aus.
  - Höchstens ein Recovery-Boundary ist nötig, wenn schon der Completion-Boundary
    `defer` überschritten hätte (T5, Fall N). Der Austritt folgt der K-Regel bzw.
    Regel (i), weil die Completion angewandt wird.
  - Der Pfad Quarantäne → `Degraded` entfällt. `Degraded` bleibt für Uncertain,
    Storage-Ausfall und Kapazitätsursachen.
  - Die §6.5-Zeile `Quarantined` („kein Input“) bleibt. Ein fälschlich gesendeter
    Reconcile wird jetzt typisiert verweigert, statt den Status herabzustufen.
- **Stop-Phasen (NET-04.2d §10.4):**
  - **Phase A:** Normal-Boundaries laufen, Disconnects und Leave-Saves aller
    Spieler werden angewandt. Der Record des Collectors wird quarantiniert; der
    Final-Save wird am Pending verweigert, der Record bleibt quarantiniert. Der
    Übergang nach B erfolgt wie vertraglich vorgesehen.
  - **Phase B:**
    - Drain markiert alle Slots mit Priorität. Andere Items werden geflusht, weil
      die Reservierung frei ist und der abgeschirmte Slot übersprungen wird
      (T9: Item sauber). Heute flusht Phase B nichts (B8).
    - Invariante P gilt weiter (der abgeschirmte Slot trägt Priorität). Das
      Terminierungsargument schließt abgeschirmte Slots aus.
    - `owned_stop_ready` wird nie wahr. Phase B endet über den Wachhund
      (1200 Boundaries ohne Fortschritt) mit `Unresolved`.
  - **Phase C:** wird bei einer Quarantäne nicht erreicht.
  - **Ergebnis:** `Unresolved{pending ≥ k, players}`. Betroffene UUIDs werden
    L1-quarantiniert, der Durable-Stand bleibt maßgeblich.

### 5.4 Änderungen an `ferrumc-sim/INVARIANTS.md` (Review A2)

Die Runtime-Lieferung ersetzt die folgenden Stellen wörtlich. Die alten Texte
sind die Zeilen auf f1e7511c; `round2/amend_check.py` wendet genau diese Blöcke
auf eine Kopie an und prüft, dass jeder alte Text genau einmal vorkommt und kein
überholter Satz stehen bleibt (§8.4).

**Z. 290–294** (DUR-04.1-Abschnitt; bisher „One player and the whole entity
owner remain reserved until genuine matching evidence“ ohne Ausnahme, und
„Expired buffer permits readonly reconciliation“ für jeden Status). Ersetzen
durch:

<!-- INV-290 -->
```text
- One player and the whole entity owner remain reserved until genuine matching
  evidence. Only with owned item counters (SIM-03.5) a quarantined pending ends
  its whole-owner reservation at the end of the boundary that quarantined it;
  its player, charge and credits stay bound. A legacy owner keeps the
  reservation, also across completion-only recovery. Every pending mutation has
  at most one outstanding move-owned Work/Completion buffer. Its weak lease
  proves buffer lifetime only. Retained refused completions block another
  dispatch. Expired buffer permits readonly reconciliation, except of a
  counter-mode quarantine; receipt None never proves worker abort or releases
  items.
```

**Z. 347–349** (Beginn der Regel „Every eligible resident live item …“; die
Regel endet in Z. 351, die Zeilen 350–351 bleiben). Ersetzen durch:

<!-- INV-347 -->
```text
- Every eligible resident live item receives exactly one scalar step in each
  accepted Tick, up to 65,536 items. Unloaded items and items fenced by a
  quarantined pending receive zero and no offline catch-up. A fully accepted
  Stop has no subsequent counter steps. Save scanning
```

**Z. 366–372** (Regel „At most one Entity operation reserves the whole owner
structure … CasConflict quarantines with the original scalar projection“).
Ersetzen durch:

<!-- INV-366 -->
```text
- At most one active Entity operation reserves the whole owner structure. Its
  visible body and request postimage stay frozen; unrelated items may change
  only scalar counters, leaving indexes, stack lengths and Prepared.used
  unchanged. Each bound private original/postimage projection records at most
  1024 actual logical claims. Claim 1025 refuses the entire next Tick with
  CounterRecoveryRequired. Lost reply or receipt None retains pending authority
  and its bound projection. CasConflict, or a genuine receipt failing the
  current read, quarantines without publishing into the visible body. A
  quarantine is terminal in-process: no completion, retry or reconcile. From the
  next boundary its projection stays frozen and unclaimed, it reserves nothing,
  and its existing item is fenced: no counter step, new due mark, pickup, save or
  selection until restart. A quarantine never causes CounterRecoveryRequired;
  Stop stays refused and restart restores the last confirmed durable state.
```

**Z. 386–387** (in der Regel „Save selection admits at most one job …“; bisher
„freeze while Entity pending exists“). Ersetzen durch:

<!-- INV-386 -->
```text
  Two slot cursors advance after each examined slot, three priority turns to one
  normal turn, and freeze while an active Entity pending reserves the owner. A
  quarantined Entity pending does not freeze them: selection skips only its
  fenced slot, which keeps its priority mark. The identical query survives
```

## 6. Auflösungsweg (Frage 3)

**Vertraglich: Neustart.** Ein quarantiniertes Pending endet nur mit dem Prozess.

- Die Quarantäne belegt, dass das Owner-Modell überholt ist: Storage hat definitiv
  verweigert, oder ein Fremdschreiber hat einen echten Commit überholt.
- Deshalb gibt es keinen erfundenen Weg zurück. Kein Reconcile (er würde einen
  CasConflict als `Uncertain` umdeuten, B7), kein Retry, kein Refund, kein
  Entfernen.
- Stop endet typisiert `Unresolved`. Danach lädt Restore den zuletzt bestätigten
  Durable-Stand:
  - das Item mit dem Record des Fremdschreibers (T13: Alter 99, nach dem ersten
    Restore-Schritt 100);
  - den Spieler unverändert, weil der Pair-Commit nie stattfand (T13: 0 Items).
- Unbestätigter Timer- und Transformfortschritt des abgeschirmten Items geht wie
  beim Crash verloren (DUR-04.2 §5). Mengen verdoppeln sich nicht und gehen nicht
  verloren.
- Bei einer Spawn- oder Drop-Quarantäne existiert kein Körper. Der Stack bleibt
  im Durable-Inventar des Spielers (Pair nicht committed), der Durable-Record der
  UUID gehört dem Fremdschreiber (T12).

**Was Spieler sehen:**

- **Das Item** bleibt an seiner Stelle sichtbar und eingefroren: kein Altern, kein
  Despawn, keine Aufnahme bis zum Neustart. Ein Pickup-Versuch wird verweigert,
  der Client erhält den bestehenden `Resync`-Intent der verweigerten Eingabe. Es
  gibt keine erfundene Client-Entfernung.
- **Der Collector** eines Pair-Pickups:
  - Er erhält das Item nicht.
  - Inventaraktionen werden mit Resync verweigert (sein Pending blockiert).
  - Bewegung, Modus und Slot laufen als Deferred.
  - Beim Verlassen greift die Leave-Save-Quarantäne (L1); ein erneuter Join ist
    bis zum Neustart nicht möglich (`AdmissionError::Quarantined`, §7.3). Nach dem
    Neustart joint er mit dem Durable-Stand.
- **Alle anderen Spieler** sind unberührt.

**Was Betreiber sehen:**

- `player_ownership_checkpoint(query).status() == Quarantined`;
- die Metrik `native_pending_quarantined` (NET-04.2d §6.5), sobald DUR-05a sie
  implementiert (Review I1);
- Log und Metrik `native_leave_save_quarantined`;
- **kein** `Degraded` aus dieser Ursache;
- beim Stop `Unresolved{pending, players}` mit L1-UUIDs.

Mehrere Quarantänen deuten auf einen systemischen Fremdschreiber hin. Die
Eskalationsschwelle legt DUR-05a fest (§11 O2).

### 6.1 Bis zum Neustart gebundene Ressourcen (Review A5)

Gemeinsam für **jede** Quarantäne:

| Ressource | Grenze | Typisierte Verweigerung bei Erschöpfung |
|---|---|---|
| Pendingplatz | `limits.pending`, Maximum 32 (`MAX_PLAYER_OWNERSHIP_PENDING`) | `Capacity("pending count")` |
| Pendingladung `661 + 2·before + 5·after` | gemeinsame `limits.pending_bytes`, Maximum 8 MiB = 8 388 608 B | `Capacity("pending bytes")` |

Zusätzlich je Art (nur Counter-Modus, Store-Reservierung jeweils frei):

| Quarantäne-Art | zusätzlich gebunden | Folge |
|---|---|---|
| Entity-Autosave (`Replace`) | abgeschirmter Körper: ein Entity-Slot, seine Live-Bytes (gemeinsam ≤ 16 MiB) und Confirmed-Bytes (gemeinsam ≤ 16 MiB), Dirty-/Prioritätsmarke; eine verbrauchte Save-ID | Restore/Spawn neuer Items scheitern früher an Slot- oder Byte-Cap |
| Entity-Despawn (`Remove`, R2) | wie Autosave, das Item bleibt `due`; dazu ein Publikationskredit (eine Gruppe, Ladung 256 B) | wie oben plus Publikationskapazität |
| Pair-Pickup (`Replace`/`Remove`) | abgeschirmter Körper wie oben; der `PlayerState` des Collectors (ein Platz von `limits.players` ≤ 128, seine Bytes in `limits.player_bytes` ≤ 32 MiB) und seine Lifecycle-Zelle; ein Publikationskredit (Gruppe, Ladung 256 B + Stackgröße bzw. 256 B bei Entfernung); bei animiertem Take ein Lifecycle-Pin | Release `PublicationPin` bzw. `ItemReservation`, Rejoin `Reserved`, Stop und Scheduler-Ruhe blockiert; neue Spieler `Capacity("player count")`; Publikations- und Pin-Kapazität |
| Drop (Spawn, kein Körper) | `PlayerState` und Lifecycle-Zelle des Werfers; ein Publikationskredit (256 B + Stackgröße). Kein Entity-Slot: `reserve_spawn_slot` reserviert nur Vec-Kapazität | Release `ItemReservation`, Rejoin `Reserved`; Spieler- und Publikationskapazität |
| Item-Spawn (Spawn, kein Spieler) | ein Publikationskredit | Publikationskapazität |
| reine Player-Quarantäne (Autosave, Bootstrap) | `PlayerState` bzw. beim Bootstrap die gestagte Lifecycle-Zelle; V2 ändert daran nichts | Release `Pending`; Spielerkapazität |

Die Projektion `DeferredOwnedCounters` liegt im Pendingeintrag und belegt nichts
zusätzlich. Publikationskredite entstehen nur mit aktivem Publikationsmodus
(SIM-03.4), Lifecycle-Zellen und -Pins nur mit aktivem Lifecycle-Modus
(SIM-03.4a). Ein Spieler belegt seinen Platz auch ohne Quarantäne; die
Quarantäne verhindert nur, dass Release ihn freigibt.

**Rechnung:**

- **Pendingladung:** Eine maximale Entity-Save hat bei je 256 KiB Before und
  After die Ladung 661 + 2·262 144 + 5·262 144 = 1 835 669 B (DUR-04.2 Z. 216–217).
  ⌊8 388 608 / 1 835 669⌋ = 4. Vier solche Quarantänen binden 7 342 676 B. Die
  fünfte gleich große Save-Admission wird mit `Capacity("pending bytes")`
  verweigert, obwohl erst 4 von 32 Pendingplätzen belegt sind. Für kleinere
  Pendings bleiben 1 045 932 B.
- **Allgemein** gilt für k gleich große Quarantänen mit Ladung c:
  k_max = min(`limits.pending`, ⌊`limits.pending_bytes` / c⌋), dazu für
  Pair-Arten min(`limits.players`, Publikationsgruppen). Bei c > 4 MiB, etwa
  einem Pair mit großem Spielerrecord, verhindert schon eine Quarantäne jede
  zweite gleich große Pending.
- **Erschöpfte Pendingladung blockiert jeden Schreibweg** des Shards, der eine
  Pending braucht: Item-Saves, Pickups, Drops, Player-Autosaves und Leave-Saves.
  Die Item-Isolation aus V2 hilft dann nicht mehr. Ab welcher Belegung typisiert
  eskaliert wird, entscheidet DUR-05a (§11 O2).

## 7. Testmatrix (Frage 4)

Alle Zeilen laufen auf `InMemoryStore` und auf frisch geöffnetem redb, im
Counter-Modus mit echter Work/Completion (Fremdschreiber über echte Store-CAS)
und ohne injizierten Zustand; M15 läuft im Legacymodus. „Heute“ bezeichnet die
Baseline-Probe auf f1e7511c, „Ziel“ den Runde-2-Prototyp. Alle Probe-Zeilen
liefen in debug und release.

| Zeile | Inhalt | Heute | Ziel |
|---|---|---|---|
| M1 R3-A5-Repro | CasConflict-Save, `defer` 8 und 1024 | B1: Recovery nach `defer − 2` | T1: `defer + 64` `Ok`, `claims` eingefroren bei 2, Reservierung frei, `owned_stop_ready` = false |
| M2 Superseded | echter Commit, Fremdschreiber, Reconcile | B2: Recovery | T2: `defer + 64` `Ok`, eingefroren |
| M3 e2-Interplay | Pair-Pickup, Detach, Leave-Save-Blocker | B3a/B3b: Detach nach Recovery unmöglich | T3: Detach nach `defer + 16` angewandt, `blocking = Quarantined`, Release `ItemReservation`, Rejoin `Reserved`, frischer Fremd-Read `StaleSnapshot` |
| M3b veralteter Read | Pair-Read vor dem Konflikt | — | T3b: `Ineligible("quarantined item")`, 0 Work |
| M4 Isolation | anderes Item speichern, neues Item restaurieren | B4: alles `Reserved`, 0 Saves | T4: 3 bestätigte Saves, Durable-Stamp fortgeschrieben, Restore zugelassen, Save des abgeschirmten Items `Ineligible` |
| M5 N−1/N/N+1 | Completion mit `defer − 2`/`defer − 1`/`defer` Claims | B5: immer dauerhaft Recovery | T5: eingefroren bei 7/8/8; im Fall N genau 1 Recovery-Boundary, danach `defer + 64` `Ok` |
| M6 mehrere Quarantänen | 4 nacheinander, `limits.pending` = 4 | heute höchstens eine (B4) | T6: 4 quarantiniert, nächster Save `Capacity("pending count")`, Boundaries `Ok` |
| M7 Retry/Reconcile | auf `Quarantined` | B7: Reconcile führt zu `Uncertain` | T7: Reconcile `Ineligible("quarantined pending")`, Retry `UnprovenWorkerLifetime`, 0 Work, Status bleibt |
| M8 Kontrolle Uncertain | `ReceiptAbsent` | B6: Recovery | T8: unverändert Recovery (Scope-Grenze, §11 O1) |
| M9 Stop | Drain plus Recovery-Boundaries (Phase-B-Modell), danach Stop | B8: 0 Saves, anderes Item dirty | T9: anderes Item geflusht und sauber, `owned_stop_ready` = false, Stop `NotQuiescent`; Kontrolle ohne Quarantäne: `true`, Stop angenommen |
| M10 Release | unbeteiligter Spieler nach `defer + 16` | B9: Detach unmöglich | T10: Detach, Save, Ack, Release ok |
| M11 Quarantäne-Boundary | Completion plus Save eines anderen Items im selben Boundary | — | T11: im selben Boundary `Reserved` in Plan und Apply, anderes Item gestept, kein Panic; nächster Boundary lässt den Save zu |
| M12 Spawn-Quarantäne | UUID nach frischem Read von Fremdschreiber belegt | — | T12: Reservierung frei, kein Körper, `defer + 32` `Ok`, Save eines anderen Items zugelassen |
| M13 Neustart | redb schließen, wieder öffnen, Restore | — | T13: Items mit Fremd-Record, Collector mit 0 Items, keine Pendings |
| M14 Regression | volle Suiten auf dem Prototyp ohne Probe-Module | — | `ferrumc-sim` alle Targets 404/404 (Lib 266, Integration 127, darunter 46 Legacy-`player_entity_transfer`, Doctests 11); `ferrumc-session` 244/244; `ferrumc-testkit` 100/100; `ferrumc-app` (`--no-fail-fast`) 375 bestanden, 0 fehlgeschlagen, 1 ignoriert (bestehend), 40 Testbinaries, ohne Wiederholungslauf; Clippy `-p ferrumc-sim --all-targets -D warnings` frisch sauber; `fmt --all --check` grün |
| **M15 Legacy (Runde 2)** | DUR-04.1-Owner ohne Counter: Superseded-Pickup-Quarantäne, `recover_player_ownership_completions`, Legacy-Tick, veralteter Pair-Read eines zweiten Spielers, Reconcile | T14 auf der Basis: Reservierung bleibt `true`, `Reserved`, 0 Work, Reconcile 1 Work | T14: **zeilengleich mit der Basis**. Die Runde-1-Skizze fällt durch (Reservierung `false`, 1 Work gegen die überholte Erwartung) |

**Gegenproben des Reviewers auf dem Runde-2-Prototyp** (`rev035.rs`,
SHA-gleich mit `.codex-tmp/sim03_5-root-review/probes/rev035.rs`):

| Probe | Ergebnis Runde 2 |
|---|---|
| R1 Legacy-Recovery | Reservierung nach Recovery `true`, veralteter Pickup `Reserved`, 0 Work, `stale_cas = none`; gleich der Basis. Auf dem Runde-1-Prototyp hatte der Reviewer `false` und 1 Work gemessen |
| R2 Despawn-Quarantäne | zeilengleich mit dem Reviewer-Prototyp: Item `due`, 24/24 `Ok`, Item 2 gespeichert |
| R3 Restore derselben UUID | zeilengleich: später `DuplicateUuid`, im Quarantäne-Boundary `Reserved`, kein Panic |
| R4 Unload/Reload | zeilengleich: 0 Schritte, nach Reload weiter abgeschirmt, `defer + 16` `Ok` |

**Pflicht für die Runtime-Abnahme** (alle als echte Tests im Ticket, auf Memory
und redb):

- M1–M15, also T1–T13 einschließlich T3b und T14;
- **R1–R4 des Reviewers** als echte Tests mit Assertions statt Messzeilen
  (Root-Entscheidung 9);
- **EntityDespawn-Quarantäne** (fälliges Item, Delete-CAS kollidiert): Der
  abgeschirmte Slot bleibt `due`, wird nicht ausgewählt, und die Shard-Boundaries
  bleiben `Ok` (durch R2 vorab belegt);
- **Quarantäne in einem Recovery-Boundary** (Phase B) mit anschließender
  Save-Auswahl im selben Recovery-Boundary;
- **SnapshotOnly-Kopie** mit Quarantäne: Die Abschirmung leitet sich aus der
  kopierten Pendingtabelle ab;
- **Legacy-Modus**, ausdrücklich mit `recover_player_ownership_completions` auf
  einem Legacy-Owner mit Quarantäne (T14/R1): Reservierung bleibt, keine Work
  gegen die überholte Erwartung, Reconcile zugelassen;
- **Pickup durch einen zweiten Spieler** mit einem vor dem Konflikt gelesenen,
  zur alten Erwartung passenden Read (T3b);
- **Pendingladung:** vier maximale Entity-Save-Quarantänen, dann
  `Capacity("pending bytes")` für die fünfte (§6.1);
- `INVARIANTS.md` enthält die vier Blöcke aus §5.4 wörtlich;
- Verbraucher Session, Testkit und App laufen erneut, App mit `--no-fail-fast`.

## 8. Exakter Änderungstext für DUR-04.2 und DUR-04.1 (Frage 5, Review A3)

Root übernimmt den Nachtrag in drei Teilen: die Stellenersetzungen in DUR-04.2
(§8.1), den neuen DUR-04.2-Abschnitt §11 (§8.2) und die Stellenersetzungen in
DUR-04.1 (§8.3). Die alten Texte sind die Zeilen auf f1e7511c; Arbeitsbaum und
Archiv sind für beide Dateien identisch. Jeder Block ersetzt genau die genannten
Zeilen. `round2/amend_check.py` wendet alle Blöcke wörtlich aus diesem Brief an
und belegt, dass danach kein überholter Satz stehen bleibt (§8.4).

### 8.1 Stellenersetzungen in DUR-04.2

**§3, Z. 89–90** (Ergänzung; bisher „Eine dauerhafte Pause während DB-Pending
wäre eine explizite Gameplayabweichung, keine native Timerparität.“). Ersetzen
durch:

<!-- D2-089 -->
```text
konkreten Backpressure/Drainpfad erhalten. Eine dauerhafte Pause während DB-
Pending wäre eine explizite Gameplayabweichung, keine native Timerparität; für
quarantinierte Pendings ist genau diese Abweichung in §11 deklariert.
```

**§7 Tabelle, Z. 188** (bisher „1 pro Entitypending, max1/Shard | keine zweite
Entityoperation“). Ersetzen durch:

<!-- D2-188 -->
```text
| Private counter ledger entries |1 pro Entitypending; max1 aktiv gebunden/Shard, dazu eingefrorene quarantinierte bis zu den §11-Grenzen|keine zweite aktive Entityoperation|
```

**§7 „Strukturreservation“, Z. 260–261** (bisher „weiterhin höchstens **eine**
Entityoperation pro Shard“). Ersetzen durch:

<!-- D2-260 -->
```text
Keine breitere per-slot Reservation in `.2a`: weiterhin höchstens **eine**
aktive Entityoperation pro Shard; eine quarantinierte gibt die Reservation am
Ende ihres Quarantäneboundaries frei und zählt danach nicht mehr (§11). Die
bestehende ganze Storestrukturreservation bleibt
```

**§7 „Strukturreservation“, Z. 279–282** (bisher „Definitiver No-start- oder
Conflictrefusal wendet Projection … an“; im Countermodus ist CasConflict nie
definitiv, sondern quarantiniert). Ersetzen durch:

<!-- D2-279 -->
```text
Doppelspawn noch eine nachträgliche zweite Itemquelle. Definitiver No-start-
Refusal wendet Projection auf das ursprüngliche tatsächliche Liveitem an und
behält seine passende bestätigte Recordauthority. Conflict/Superseded bleibt
quarantined und ermächtigt keine neue CAS gegen erfundene Erwartungen; die
Projection wird eingefroren und nie angewandt, das Item abgeschirmt (§11).
```

**§7 „Duegrenze“, Z. 326–328** (Ergänzung nach „Dauerhafte Uncertainty macht
den Shard ausdrücklich recovery-required“). Ersetzen durch:

<!-- D2-326 -->
```text
kein Autoabort, Itemdroprefund oder frisches IDreplan. Dauerhafte Uncertainty
macht den Shard ausdrücklich recovery-required, nicht still stehengebliebene
Items bei behaupteter erfolgreicher Weltzeit. Keine Wallclocktimeoutauthority.
Eine Quarantäne ist keine Uncertainty: Sie bindet nach ihrem Quarantäneboundary
keine Claims und führt nicht in diesen Modus; ihr Item ist typisiert
abgeschirmt statt still stehengeblieben (§11).
```

**§7 „Savefortsetzung“, Z. 360–361** (bisher „Wenn schon Entitypending
existiert: **keinen** neuen Savecandidate encodieren, keine
Cursor-/Turnfortschritte“). Ersetzen durch:

<!-- D2-360 -->
```text
Wenn schon ein aktives Entitypending die Storestruktur reserviert: **keinen**
neuen Savecandidate encodieren, keine Cursor-/Turnfortschritte, nur tatsächliche
Counter-/Deferphase. Ein quarantiniertes Entitypending reserviert ab seinem
Folgeboundary nicht; die Auswahl läuft weiter und überspringt nur den
abgeschirmten Slot (§11). Nach genuine
```

**§8, Z. 436** (bisher „Unloaded/offline erhalten0“). Ersetzen durch:

<!-- D2-436 -->
```text
Unloaded/offline und quarantäneabgeschirmte Slots (§11) erhalten0;
RecoveryRequired meldet keinen erfolgreichen Tick.
```

**§8, Z. 439–440** (bisher „alle4096Delay40-Werte0, abgesehen vom explizit
definierten späteren ersten Spawn-Tick“). Ersetzen durch:

<!-- D2-439 -->
```text
Items werden in40erfolgreichen Ticks alle4096Delay40-Werte0, abgesehen vom
explizit definierten späteren ersten Spawn-Tick und abgeschirmten Slots (§11).
Die Kosten sind bounded O(N)
```

**§8, Z. 472–474** (Ergänzung: „uncertain/completionfehlende Items bleiben an
die entsprechende private Projection gebunden“ gilt nicht für eine Quarantäne).
Ersetzen durch:

<!-- D2-472 -->
```text
Spawn außerhalb der Prüfmenge auftauchen. Definitive Refusals projizieren auf
den erhaltenen ursprünglichen Body; uncertain/completionfehlende Items bleiben
an die entsprechende private Projection gebunden. Ein bei Planbeginn
quarantiniertes Pending bindet keine Projection; sein Item ist nicht eligible
(§11).
```

**§9, Z. 604–605** (bisher „nur private ursprüngliche Counterprojection wird
weitergerechnet“). Ersetzen durch:

<!-- D2-604 -->
```text
CasConflict hält Quarantined Pending und sichtbaren Body strikt unverändert;
die private ursprüngliche Counterprojection wird am Ende dieses Boundaries
eingefroren und nicht weitergerechnet, die Reservation endet dort (§11).
```

### 8.2 Neuer Abschnitt §11 am Ende von DUR-04.2

<!-- D2-S11 -->
```text
## 11. Nachtrag SIM-03.5 (Root): quarantinierte Item-Pendings sind abgeschirmt

Gilt nur im Countermodus (`item_counters.is_some()`), in jedem normalen und
completion-only Recovery-Boundary. Der DUR-04.1-Legacypfad ohne Counter bleibt
unverändert: Dort hält ein Quarantined Pending die ganze Storereservation bis
zum Neustart, und Reconcile bleibt zugelassen. Dafür angepasst sind §3
(Abweichung deklariert), §7 Tabelle (Private counter ledger entries), §7
„Strukturreservation“ (eine aktive Entityoperation; Conflict/Superseded), §7
„Duegrenze“ (Quarantäne ist keine Uncertainty), §7 „Savefortsetzung“ (aktives
Entitypending), §8 (Counterphase, Plan) und §9 (CasConflict). DUR-04.1 §3 und
§6 verweisen hierher.

1. **Quarantäne ist im Prozess terminal.** Ein Pending wird `Quarantined` durch
   `Refused(CasConflict)` (Storage verweigert vor jedem Commit; es existiert kein
   Receipt) oder durch einen echten Receipt, dessen Current-Read
   `confirm_current` verfehlt. Danach gibt es weder Veröffentlichung noch
   Freigabe: keine Completion (Status ≠ `InFlight`), kein Retry (Status ≠
   `Prepared`). `Reconcile` wird mit `Ineligible("quarantined pending")`
   verweigert, weil eine Receiptklärung einen CasConflict fälschlich als
   `Uncertain` einstufen und Plan und Anwendung trennen würde. Das gilt für jede
   Quarantäne im Countermodus, auch für reine Player-Pendings.
2. **Projection eingefroren.** Die private `DeferredOwnedCounters` eines
   quarantinierten Pendings behält ihren Wert vom Ende des quarantinierenden
   Boundaries. Ab dem folgenden Boundary bindet der Plan sie nicht mehr; sie
   zählt keine Claims und wird nie angewandt. Der quarantinierende Boundary zählt
   wie bisher höchstens einen Claim. Überschreitet dieser `defer`, verweigert der
   Normal-Boundary vor dem Drain, und der completion-only Recovery-Boundary
   wendet die Completion an.
3. **Item abgeschirmt.** Hält ein quarantiniertes Pending das Replace- oder
   Remove-Postimage eines vorhandenen Items, ist dieses Item ab dem folgenden
   Boundary nicht eligible: kein Counterstep und damit keine neue Duemarkierung,
   keine Saveauswahl (der Cursor läuft weiter, kein `ItemSaveBlocked`). Pickup und
   Save dieses Items werden vor jeder Kandidaten- oder Bodykopie mit
   `Ineligible("quarantined item")` verweigert. Das gilt für Pickup schon im
   quarantinierenden Boundary, weil Completions dort vor allen übrigen Inputs
   angewandt werden; eine Save verweigert dort noch die bestehende Reservation
   (`Reserved`). Eine
   bereits gesetzte Duemarkierung bleibt ohne neuen Delete. Sichtbarer Körper,
   Pending-Postimage und zuletzt bestätigte Recordauthority bleiben unverändert;
   es entsteht keine CAS gegen die überholte Erwartung. Kein Altern, kein Despawn
   und keine Aufnahme bis zum Neustart sind eine deklarierte Gameplayabweichung,
   keine stille Pause: Der Boundary meldet für das Item keinen Schritt, und der
   Status ist per Checkpoint abfragbar.
4. **Reservation endet.** Am Ende des Boundaries (normal oder completion-only),
   der das Pending quarantiniert hat, endet seine ganze Storereservation. Ab dem
   folgenden Boundary zählt es im Plan nicht als reservierende Entityoperation.
   „Höchstens eine Entityoperation pro Shard“ gilt für aktive, nicht
   quarantinierte Pendings. Ein quarantiniertes Spawn- oder Drop-Pending (kein
   vorhandener Körper) verliert ebenso die Reservation und schirmt nichts ab.
   Checkpoint, Stop-Prädikat und Release-Gates bleiben unverändert.
5. **Unverändert:** `Uncertain`, `NeedsRefresh`, `InFlight` und `Prepared` binden
   ihre Projection und halten ihre Reservation weiter. Dauerhafte Uncertainty
   führt nach `defer` Claims wie bisher zu `CounterRecoveryRequired` (§7). Diese
   Abschirmung wird nicht auf `Uncertain` ausgedehnt.
6. **Auflösung.** Ein quarantiniertes Pending endet nur mit dem Prozess. Der Shard
   wird mit Quarantäne nie quiescent, Stop endet `Unresolved`, der zuletzt
   bestätigte Durable-Stand bleibt maßgeblich, und ein Neustart lädt ihn über
   Restore. Unbestätigter Timer- und Transformfortschritt des abgeschirmten Items
   geht wie beim Crash verloren (§5). Eine In-Prozess-Übernahme des Durable-Stands
   ist nicht Teil dieses Nachtrags.
7. **Gebundene Ressourcen.** Jede Quarantäne behält bis zum Neustart ihren
   Pendingplatz (`limits.pending`, höchstens 32) und ihre Pendingladung
   `661 + 2*before + 5*after` in der gemeinsamen Ladung (`limits.pending_bytes`,
   höchstens 8 MiB). Eine maximale Entitysaveladung von 1,835,669 Bytes passt
   viermal hinein; die fünfte wird mit `Capacity("pending bytes")` verweigert.
   Je nach Art bleiben außerdem gebunden: der abgeschirmte Körper mit Slot,
   Live- und Confirmed-Bytes; bei Pickup und Drop der Spieler (Spielerplatz,
   Spielerbytes, Lifecycle-Zelle; Release und Rejoin verweigert); bei Pickup,
   Drop, Spawn und Despawn ein Publikationskredit und bei animiertem Take ein
   Lifecycle-Pin; bei einer Save oder einem Despawn die verbrauchte Save-ID.
   Erschöpfte Grenzen verweigern neue Pendings typisiert (`Capacity("pending
   count")`, `Capacity("pending bytes")`, `Capacity("player count")`,
   Publikations- oder Lifecyclekapazität). Eine Eskalationsschwelle legt DUR-05a
   fest.

Originalsemantik: Vanilla und Paper altern einen getickten Item-Körper je Tick
und entfernen ihn an der Schwelle; eine Persistenz-Projection kennen sie nicht
(Source-Review SIM-03.5 §3.3, kein Differenziallauf). Abnahme: SIM-03.5-Matrix
M1–M15 und die Reviewproben R1–R4 auf Memory und redb mit echter
Work/Completion.
```

### 8.3 Stellenersetzungen in DUR-04.1

**§3, Z. 229–231** (bisher „Für `.1` wird konservativ höchstens **eine**
Entityoperation pro Shard reserviert …“ ohne Ausnahme). Ersetzen durch:

<!-- D1-229 -->
```text
Für `.1` wird konservativ höchstens **eine** Entityoperation pro Shard
reserviert: Drop/Pickup/Itemspawn besitzen während Pending die tatsächliche
EntityStore-Mutationsgrenze einschließlich Slotcapacity, IDs und Bytebudget.
Im `.1`-Modus gilt das auch für ein Quarantined Pending bis zum Neustart. Nur im
DUR-04.2-Countermodus endet die Reservation eines Quarantined Pending am Ende
seines Quarantäneboundaries; sein vorhandenes Item bleibt danach für alle
Storewriter abgeschirmt (DUR-04.2 §11).
```

Der Satz in Z. 236–237 („während der Entityreservation verweigern alle
konkurrierenden Storewriter“) bleibt wörtlich richtig und unverändert.

**§6, Z. 373–374** (bisher „Eine Entityreservation pro Shard“ ohne Ausnahme).
Ersetzen durch:

<!-- D1-373 -->
```text
- Eine Entityreservation pro Shard, ein coalescter Move pro zugelassenem
  Player; im DUR-04.2-Countermodus gilt das für aktive Pendings, quarantinierte
  reservieren nach ihrem Quarantäneboundary nicht mehr (DUR-04.2 §11). Alle
  Tabellen/Buckets/IDs bleiben durch bestehende Entityslotcaps
```

### 8.4 Widerspruchsfreiheit

`round2/amend_check.py` wendet alle 12 Stellenersetzungen, die vier
`INVARIANTS.md`-Blöcke aus §5.4 und den neuen §11 wörtlich aus diesem Brief auf
Kopien der Archivdateien an. Ergebnis (`round2/amendment-check/result.json`):

- Jeder alte Text kommt genau einmal vor und beginnt in der genannten Zeile.
- Keiner dieser überholten Sätze bleibt stehen: „max1/Shard“, „Counterprojection wird weitergerechnet“,
  „No-start- oder Conflictrefusal“, „Wenn schon Entitypending existiert“,
  „höchstens **eine** Entityoperation pro Shard.“, „sichtbaren reservierten
  Body“, „At most one Entity operation reserves“, „freeze while Entity pending
  exists“, „CasConflict quarantines with the original scalar projection“,
  „Unloaded items receive zero and no offline“.
- `report.txt` listet nach der Übernahme jede Zeile mit Quarantäne-,
  Entityoperation-, Entityreservation-, Entitypending- oder Abschirmungsbezug
  (67 Zeilen). Ich habe jede Zeile gelesen; keine widerspricht dem Nachtrag.
  Außer den neuen Texten stehen dort unverändert und weiter gültig: DUR-04.1
  Z. 226 (Statusliste), Z. 236–237 (Storewriter während der Reservation) und
  Z. 244–245 („superseded Postimages führen zu Quarantined/Resync“);
  `INVARIANTS.md` Z. 300 („a historical superseded receipt quarantines/resyncs“)
  sowie Z. 424, 475 und 487 (Quarantäne behält Publikations-, Zellen- und
  Pin-Kredite, passend zu §6.1).
- Zusätzlich ohne Treffer im Bericht geprüft und unverändert gültig:
  - DUR-04.2 §8 „plus höchstens eine tatsächlich vorbereitete
    Entityspawnreservation“ (Planumfang): Eine quarantinierte Spawn-Pending hat
    keine zulässige Completion und kommt im Plan nicht vor.
  - DUR-04.2 Z. 187 (1024 Claims „pro gebundenem Item/Pending“) und §9
    „Receipt None bleibt Uncertain“.
  - `INVARIANTS.md` Z. 316 (Stop verlangt keine Pendings).
- Zeilenangaben nach der Übernahme verschieben sich um die eingefügten Zeilen;
  die Verweise in §11 nennen deshalb Abschnitte, keine Zeilen.

## 9. Vorschlag für Root-Notizen zu NET-04.2d (nicht verlangt, zur Konsistenz)

- **R3-A5:** „Durch SIM-03.5 für `Quarantined` umgesetzt: Eine Quarantäne löst
  kein `CounterRecoveryRequired` und kein `Degraded` mehr aus. Das Item ist
  abgeschirmt, Stop endet `Unresolved`. Ein dauerhaft `Uncertain` gebliebenes
  Pending ist nicht erfasst (eigenes Ticket).“
- **§6.1a, „Terminierung je Ursache“:** ergänzen um „Ein `Quarantined`-Pending
  bindet keine Projektion (DUR-04.2 §11); trifft seine Completion bei vollem
  Ledger ein, wendet genau ein Recovery-Boundary sie an.“
- **§10.4, Phase B:** Das Terminierungsargument schließt abgeschirmte Slots aus.
  Mit Quarantäne wird `owned_stop_ready` nie wahr, das Ende kommt über den
  Wachhund mit `Unresolved`. „Höchstens eine Entity-Operation zugleich“ meint
  eine aktive.

## 10. Ticketzuschnitt (Frage 6)

- **SIM-03.5 Runtime:** eine Production-Crate, `server/crates/ferrumc-sim/**`
  **einschließlich `INVARIANTS.md`** (Root-Entscheidung 3).
  - Geändert werden `src/shard/player_ownership/counters.rs`,
    `src/shard/player_ownership.rs` und `INVARIANTS.md` (die vier Blöcke aus §5.4).
  - Neu kommt ein Testmodul `src/shard/player_ownership/counters/tests/quarantine.rs`
    plus eine `mod`-Zeile in `counters/tests.rs`. Es enthält T1–T14 und R1–R4
    (§7).
  - Keine Dependency, keine Cargo-Änderung, keine öffentliche API-Änderung.
- **Storage und Session sind nicht betroffen.** Belegt durch Läufe auf dem
  Runde-2-Prototyp: kein neuer Status, keine neue Fehlervariante,
  `ferrumc-session` 244/244 einschließlich
  `uncertain_pickup_counts_every_100_quarantines_and_renews_one_final_save`,
  Testkit 100/100, App 375 bestanden, 0 fehlgeschlagen, 1 ignoriert. In Runde 1 war diese Aussage nicht belegt (§1.1).
- **Root:** übernimmt den Nachtrag aus §8 (DUR-04.2 und DUR-04.1) und optional
  die NET-04.2d-Notizen (§9); prüft vor der Runtime-Freigabe unabhängig.

## 11. Abgrenzung `Uncertain` und Folgearbeit

### O1: dauerhaft unsicherer Pickup, nicht Teil von SIM-03.5 (Review A6, Root-Entscheidung 7)

- **Befund:** Der NET-04.2e2-Befund §8.3 betrifft einen dauerhaft *Uncertain*
  gebliebenen Pickup, keine Quarantäne. Er führt nach `defer` Claims ebenso zu
  `CounterRecoveryRequired`.
- **Meine Runde-1-Begründung war falsch.** Ich hatte geschrieben, dauerhafte
  Uncertainty sei „meist Storage-Verfügbarkeit, also shardweit“. Meine eigene
  Kontrolle B6 widerlegt das: **eine einzige verworfene Work** plus Reconcile
  (`ReceiptAbsent`) macht ein Item dauerhaft `Uncertain` und bringt den Shard
  nach `defer` Boundaries in Recovery. Der e2-Test zeigt dasselbe. Das ist ein
  Ereignis an einem einzelnen Item, kein shardweiter Storage-Ausfall.
- **Warum V2 nicht auf `Uncertain` übertragbar ist:** Ein später erscheinender
  Receipt braucht Reservierung und Projektion. Eine Freigabe ermöglichte Writes
  gegen eine womöglich schon eigene, überholte Erwartung (I3). Eine Lösung
  braucht einen Abbruchbeweis aus dem Storage, etwa eine storageseitig gesperrte
  Query, oder eine Driver-Politik.
- **Vertragliche Abgrenzung:** SIM-03.5 löst diesen Fall nicht. V2 wird nicht auf
  `Uncertain` ausgedehnt. M8/T8 und der e2-Test dokumentieren das unveränderte
  Verhalten. Root legt dafür ein eigenes Ticket mit eigener Aktivierungsgrenze an.
  Das R3-A5-Gate gilt mit SIM-03.5 nur für `Quarantined` als erfüllt.

### Folgearbeit (Root-Entscheidung 8, nur benannt)

| Punkt | Inhalt | Zuweisung |
|---|---|---|
| O2 Kapazität | typisierte Schwelle (z. B. `Degraded{QuarantineLimit}`) und Zählung **aller** gebundenen Ressourcen aus §6.1 | DUR-05a |
| e2-Review-Auflage | Leave-Save- und L1-Ausgang im Recovery- bzw. Degraded-Pfad festlegen (NET-04.2e2-Review §7) | DUR-05a |
| O3 Phase B nur über Wachhund | Stop dauert mit Quarantäne mindestens 1200 Recovery-Boundaries ohne Fortschritt; ein lesendes Prädikat „nur Quarantänen blockieren“ für ein früheres `Unresolved` | zulässige Folgearbeit |
| O4 In-Prozess-Auflösung (SIM-03.5b) | Recovery-Input mit frischem echten Read dieser UUID (bei Pair-Quarantänen beider Ressourcen) übernimmt den Durable-Stand; eigener Vertrag und Driver-Verdrahtung | optional, nach DUR-05a |
| O5 Spawn-Quarantäne | Ein späterer Restore derselben UUID aus dem Durable-Stand wird zugelassen, solange kein Körper existiert (R3: mit Körper `DuplicateUuid`) | zulässig, kein Handlungsbedarf |
| O6 Client-Sicht | Das abgeschirmte Item bleibt sichtbar; ob Viewer es anders sehen, entscheidet NET-04.2b | NET-04.2b |
| Driver-Rückzug | Bei `Ineligible("quarantined item")` zurückweichen statt pro Tick einen Pair-Read zu machen (Review I2) | Auflage an NET-04.2f |

## 12. Änderungen Runde 2

| Befund / Entscheidung | Umsetzung | Stelle |
|---|---|---|
| Entscheidung 1: V2 bleibt | unverändert gewählt | §0, §4 |
| **A1** / Entscheidung 2: Legacy unverändert | Guard `item_counters.is_none() → return` in `release_quarantined_reservation`; `require_unfenced_item` meldet keinen eigenen Lookup-Fehler; Tabelle aller V2-Pfade mit Modusbegründung; Legacy-Pflichttest T14 (Muster R1) besteht auf Basis und Runde 2 mit gleichen Messzeilen, die Runde-1-Skizze fällt durch; volle Sim-Suite mit allen Targets plus Verbraucher | §2 Punkt 5, §5.1 Punkte 3/4/9/10, §5.2 I9, §7 M14/M15, §13 |
| **A2** / Entscheidung 3: `INVARIANTS.md` | Brief berichtigt; korrigierte Anker; vier Ersatzblöcke wörtlich; Ticketzuschnitt umfasst `INVARIANTS.md` | §1, §5.4, §10 |
| **A3** / Entscheidung 4: Nachtrag vollständig | 10 Stellenersetzungen in DUR-04.2 (§3, §7 Z. 188/260/279/326/360, §8 Z. 436/439/472, §9 Z. 604), 2 in DUR-04.1 (§3 Z. 229, §6 Z. 373); §11 neu mit geschärften Punkten 2 („Wert vom Ende des quarantinierenden Boundaries“) und 3 („ab dem folgenden Boundary“, `Reserved` im Quarantäne-Boundary); maschinelle Übernahmeprüfung | §8, §8.4 |
| **A4** / Entscheidung 5: Ehrlichkeit | Tabelle „belegt / nicht belegt“; SendMessage-Fehlschlag benannt; Fragen künftig als Laufergebnis | §1.1, §10 |
| **A5** / Entscheidung 6: Ressourcen | vollständige Liste je Quarantäne-Art mit Grenzen und Verweigerungen; Rechnung 4 × 1 835 669 B; Nachtrag Punkt 7 | §6.1, §8.2 Punkt 7 |
| **A6** / Entscheidung 7: O1 | falsche Begründung ersetzt; Abgrenzung; V2 nicht auf `Uncertain`; eigenes Ticket bei Root | §0, §11 O1, §8.2 Punkt 5 |
| Entscheidung 8: Folgearbeit | O2 und e2-Auflage → DUR-05a, O3 zulässig, O6 → NET-04.2b, Driver-Rückzug → NET-04.2f | §11 |
| Entscheidung 9: R1–R4 | als Pflicht-Tests der Runtime-Matrix; auf Runde 2 bereits gelaufen | §7 |
| Review I1–I4 | I1 Metrik offen (§5.2 I5, §6); I2 → NET-04.2f; I3 Reconcile-Verweigerung auch für Player-Quarantänen (§5.1 Punkt 10, §8.2 Punkt 1); I4 Aufwandsgrenze (§5.1 Punkt 2) | wie angegeben |

## 13. Evidenz

Unter `.codex-tmp/sim03_5/`. Der neue Freeze
`round2/evidence.freeze.json` bindet diesen Brief, den unveränderten
Runde-1-Kandidaten, alle Runde-1-Artefakte und alle Runde-2-Artefakte. Private
Kopien und Target-Verzeichnisse sind ausgenommen; ihr relevanter Inhalt ist über
`results/*.json` gepinnt. Brief-SHA und Freeze-SHA werden außerhalb dieses
Dokuments an Root gemeldet.

| Artefakt | Inhalt |
|---|---|
| `round1-candidate/` | Runde-1-Brief (`a21d8fd2…`), Runde-1-Freeze (`6dbb05ef…`), `SHA256SUMS`; schreibgeschützt |
| `probes/`, `results/`, `logs/`, `make_proto.py`, `v2-reference-sketch.diff`, `run_probe.py`, `run_all.sh`, `evidence.freeze.json` | Runde-1-Evidenz, unverändert |
| `round2/make_proto_r2.py`, `round2/v2-reference-sketch-r2.diff` | Runde-2-Skizze (+83/−6), jede Ersetzung genau einmal |
| `round2/probes/legacy_probe.rs` | T14 Legacy-Pflichttest (Muster R1, mit Assertions) |
| `round2/probes/target_probe.rs`, `round2/probes/rev035.rs` | T1–T13 (SHA-gleich mit Runde 1), R1–R4 (SHA-gleich mit dem Reviewer) |
| `round2/run_probe_r2.py` | installiert Probe-Module plus `mod`-Zeilen, prüft alle 1390 Dateien gegen `base.freeze.json`, eigenes Target `round2/target`, `--jobs 2`, `--test-threads 1` |
| `round2/run_all_r2.sh`, `round2/logs/all.status` | Gesamtlauf: vier Kopien neu, fmt, T14 auf Basis und Runde-1-Skizze (`--expect-fail`), alle `counters::tests` mit Proben debug, Clippy frisch, Sim alle Targets, Session, Testkit, App, Proben release |
| `round2/amend_check.py`, `round2/amendment-check/` | wörtliche Übernahmeprüfung der Texte aus §5.4 und §8 |
| `round2/results/*.json`, `round2/logs/*` | Kommandos, Exit-Codes, Logs mit SHA-256, Messzeilen |
| `round2/freeze_r2.py` | schreibt und prüft den Freeze |

Toolchain: `~/.cargo/bin` (rustc/cargo 1.99.0 laut `rust-toolchain.toml`),
`--locked --offline`. Kein Git, keine Agents, keine Dependencies, keine
Produktions- oder Historienänderung.

## Root-Integrationsvermerk (12.10.2026)

Root hat bei der Übernahme den niedrigen Textbefund N1 aus
[Review Runde 2](SIM-03_5_QUARANTINE_ISOLATION_REVIEW_ROUND2.md) §5 wörtlich
korrigiert: Block `D2-S11` Punkt 3 nennt jetzt für Pickup im quarantinierenden
Boundary `Ineligible("quarantined item")` und nur für die Save `Reserved`.
Sonst ist der Brief unverändert. Original-SHA-256 vor der Korrektur:
`a7da63e0cae935d4dad4c64d1492ca7438eb9d6b0732f0951ec181750fa26d55`.
