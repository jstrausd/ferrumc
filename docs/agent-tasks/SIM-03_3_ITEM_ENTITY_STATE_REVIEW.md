# SIM-03.3 / SRV-02.1 — Unabhängige Runtimeprüfung

Status: **CLEAN für den begrenzten Item-State-/Codecvertrag**. Root hat den
zehn Dateien umfassenden Autorenfreeze unabhängig geprüft; Gameplay- und
Appconsumer bleiben offen. Autor `feature_catalog`, Review Root, 2026-10-10.

## Eingabe und Quellen

Basis `9025c687`, isoliertes Checkout `.codex-tmp/sim033-checkout`.
Freeze `.codex-tmp/sim033-freeze.sha256`, SHA256
`b5f1bc574d72901e91c9bdaa00bb2023ae6b7bca0e57adf07120440ae3197f9f`.
Brief-SHA256 `87334d5d49aae69678c3960720f4651212ac43851bbb134e91c8f3c7f2b0505e`.
Alle zehn Dateien waren vor und nach der Prüfung in Main und isoliertem
Checkout bytegleich. Keine Producer-, Lockfile-, App- oder Fremdcrateänderung.

Root las den vollständigen Vertrag, die neuen Produktionsmodule und die
Änderungen an Slotaufnahme, Removal, Transformguard, Clone und Initialisierung.
Die privaten Korruptionsproben und öffentlichen unabhängigen Bytefixtures
wurden geprüft. Gegen den gepinnten Paper-1.21.8-Build60 wurden zusätzlich die
vollständigen angewandten Konstruktor-, tick-/inactiveTick-, Merge-/Damage-,
Item-save/load- und Copy-Pfade nachgelesen. Die bewussten nativen Abweichungen
und weitere Quellen/Callees stehen im [Autorenbrief](SIM-03_3_ITEM_ENTITY_STATE.md).
Dies ist kein laufender Paper-/Clientvergleich.

## Prüfung des Vertrags

- Stack und Properties liegen direkt im bestehenden privaten Ownerslot.
  Tick-only Aufnahme/Restore/Replacement und owned Removal besitzen vor dem
  gemeinsamen Commit alle falliblen Admissionen. Owned Refusals bewahren ihre
  vollständigen Eingaben; private Boxfelder kopieren keinen Stack.
- Legacy Remove und direkte Transformmutation verweigern echte Payloadslots.
  Placeholder, Typ, Scope, Residency, Handleincarnation und Generation bleiben
  getrennte geprüfte Refusals. Cleanup nach Unload benötigt keine neue Ladung.
- Bodycap umfasst Header und FCIS; Aggregatebudget und alte Budget-null-
  Konstruktoren sind ausdrücklich. Snapshot prüft vor Stackclone, Capture vor
  Publication. Infallible Clone ist bei unmöglicher privater Korruption
  fail-stop; öffentliche checked APIs behalten konkrete Fehler.
- FCIE prüft Outer-/Nativeidentität, endliche/canonical Werte, Referenzen und
  exakte FCIS-Länge vor Stackdecode. Native i32-/Bob-/Motionbits bleiben
  erhalten. Lossless FCIS-Reencode verweigert Normalisierung.
- Zähler entsprechen dem signed Originalpfad einschließlich Sentinels,
  Java-int-Wrapping und negativer Raten. Due, Cancellationreset und Entfernung
  sind getrennt. Der kombinierte Counterhelper ersetzt keinen Active-Tick mit
  dazwischenliegender Physik/Mergephase.

## Unabhängige Ausführung

Toolchain Rust1.99.0/Edition2021, offline/locked, höchstens zwei Compilejobs.
Root ergänzte ausschließlich im isolierten Checkout drei temporäre Probes:

1. 65.536 Zählerkombinationen gegen ein unabhängiges i64-Modell mit expliziter
   32-Bit-Ringkorrektur, signed Distanzvergleich und separaten Sentinelfällen.
2. 2.048 direkt in die festen Headeroffsets geschriebene finite Motion-
   Bitkombinationen einschließlich beider Nullvorzeichen; exakte Decodebits
   und unveränderte unabhängige FCIS-Bytes geprüft.
3. 1.024 deterministische Slot-Aufnahme-/Replacement-/Removalinterleavings
   gegen einen unabhängigen Livebestand: Counts, stale Handles, vollständige
   Capturelängen und exakte Aggregatebilanz nach jedem Schritt.

Debug und Release jeweils **252 Tests grün**: die 249 Autorentests plus drei
Rootprobes. Strict all-target Clippy `-D warnings` und fmt grün. Die Probes
wurden vollständig entfernt; danach erneut zehn von zehn Freezehashes gleich.
Eigene Probe-Compile-/Stilfehler wurden vor der grünen Abnahme korrigiert;
keine Produktionsfindings oder Lintausnahmen.

| Artefakt | SHA256 |
| --- | --- |
| `.codex-tmp/sim033-root-probes.rs` | `2e98917d963d076580497c908d5ab78f47422a71369bb90649eb06b3fdc17372` |
| `.codex-tmp/sim033-root-debug.log` | `0926662279f73cb877192bdac9502b89cf81dacfe169238df32e5a56a3ea0f2d` |
| `.codex-tmp/sim033-root-release.log` | `b20207b79328559f701b3e29d005403d36e1d286c7bdb440e13236eb474d9d8d` |
| `.codex-tmp/sim033-root-clippy.log` | `fa4ccbb40dc6f15a844cec1e32faa0e5db9502cc11fa3c7d3132203e74abd5c6` |
| `.codex-tmp/sim033-root-fmt.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Ergebnis und verbleibende Consumer

Keine offenen Implementierungsfindings für diese erste konkrete Stateetappe.
Physik, Aktivierung, Merge/Pickup, Inventardrop, Tracking, UUID-Storagewriter,
Ownership-CAS/Transferreceipts, Chunk-Reload und Appbetrieb folgen separat.
Keine neue Survival-, Plugin-, End-/Dragon- oder Performanceabnahme.
