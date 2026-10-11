# SIM-03.4 — unabhängige Publikationsabnahme

Ergebnis: **CLEAN für den reservierten Sim-Publikationsproducer**. Bestätigte
Itemänderungen und echte Native-Restores erzeugen unveränderliche, begrenzt
zurückgehaltene Fakten für den späteren Sessionconsumer. Der Appdriver bleibt
inaktiv.

Root las beide neuen vollständigen Produktionsmodule, alle bestehenden
Produktionsdiffs, tatsächliche Admission-/Confirm-/Release-/Countershadowpfade,
alle1441 neuen Testzeilen und die Invariants/README-Änderungen. Es bestehen keine
offenen Runtimefindings. Der ursprüngliche Brief `efbeb2be…`, beide Designfreezes
und die gesonderte enge Collector-Comparisonfreigabe bleiben unverändert.

Group-/Byte-/Sequenzcredit und unveränderte Itempostimage werden vor Workrequest
reserviert. Die komplette Outerqueue wird vor Aktivierung fallible reserviert.
Confirmation verschiebt den vorhandenen Body und seine Charge ohne weiteren
Deepclone; Scalars werden direkt nach Counterconfirmation und vor späteren
Intents/Counterticks versiegelt. Definitiver Abort gibt genau einmal frei;
Busy/Retry/Uncertainty/Refresh/Quarantine halten ihre Reservation.

Die Folge entsteht bei tatsächlichem Confirm/Restore. Checked
`last + reserved + 1` schützt MAX; Abort verbraucht keine Folge. Die tatsächliche
Entityauthority erlaubt derzeit nur eine Entitymutation gleichzeitig. Der
Reversecredit-Test prüft deshalb getrennt reine Ledgerarithmetik; er behauptet
keine unadmittierbare Zwei-Entity-Confirmationabnahme.

Facts binden echten Entitystore/Slotgeneration, volle Scope-/Generatorbindung,
UUID/Wire/Typ und ursprüngliche Collectoradmission. Alte Bodies bleiben über
weitere Ticks, Removal und Slotreuse unverändert. `matches_owned_player` prüft
nur ursprünglichen Owner/Player/Admission, keine aktuelle Verbindung. FIFOpeek
und das tatsächliche Headticket erlauben Ack erst nach vollständiger
Sessionadmission. Drain erlaubt Peek/Ack; Stop und Schedulerquiescence verlangen
auch leere Reserved-/Retainedledger. Countershadow verwendet denselben
Publicationpredicate in wirklicher Completionprefix-/FIFOordnung.

Root erzeugte ein frisches Archiv von
`242c8f07f3ea724538a099a3339d4c65b276e863`, also einschließlich der unabhängig
geprüften DUR-03.2-Migration. Ausschließlich53 Sourcefreezefiles wurden
überlagert:52 vollständige Simfiles plus der unveränderte eigene Brief.
Sourcefreeze-SHA256:
`dc309f6974442e414495a8ce1344a16aa34f9dbdd7857b615da837aa6cb0dca8`.
Alle61 integrierten Storagefiles stimmen mit deren geprüftem Kandidat überein;
1169 außerhalb liegende Archivfiles sind nach den Prüfungen unverändert.

Unabhängige Rootprüfungen auf genau dieser Kombination:

- Debug und Release jeweils231 Unit-,111 Integration- und fünf
  Compilefail-Dokutests: **347 bestanden**, null failed/ignored.
- Strict All-target-Clippy, Workspaceformat und Rustdoc `-D warnings` bestanden.
- 53 Source-/24 Evidenzfiles und alle fünf tatsächlichen Autorenlogs frisch
 geprüft;341 Kontrolldateien und1152 übrige Files des älteren Autorenarchivs
 bytegleich. Dessen ältere Storagebasis wird nicht als neue Kombinationsabnahme
 dargestellt.21 tatsächliche Paperpins und beide Designfreezes stimmen überein.

Die27 neuen Tests enthalten24 echte Runtimefälle und drei Ledger-/Limitfälle.
Sie prüfen Spawn/Restore/Drop/Partial-/Fullpickup/Delete, tatsächliche CAS-/
Receipt-/Reconciliationfehler, Quarantine, Limits, MAX, Owner-/Scope-/Rejoin-
Ablehnung, Drain/Stop, Immutablepostimages und echtes redb-Reopen. Faultwrapper
delegieren tatsächliche Commits/Receipts und injizieren erklärte Fehler;
es gibt keine Fakefact-/Receiptfactory.

Rootartefakte: `.codex-tmp/sim03_4-root-review/`, insbesondere
`source-review.json`, `delivery-hashcheck.json`, `assessment.json`,
`final-assessment.json` und die fünf tatsächlichen Logs. Autorenlieferung:
`.codex-tmp/sim03_4/delivery/AUTHOR_DELIVERY.md`.

Charge ist logische encoded Größe, weder Heap/RSS noch Wirebytes. Bestehende
bounded Item/NBT-Deepclones sowie Arc/BTreeMap behalten ihre infallible
Allocatorgrenze. Ephemeral Facts werden nicht im bestehenden Checkpoint
gespeichert. Session-Wirebindung, originale Tracking-/Lifecycledisposition,
Socketemission, Itemphysik/Merge, gemeinsame Player-/Blockdurability und
produktives Survival sind weiterhin eigene erforderliche Verbraucher.
