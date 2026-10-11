# SIM-03.4a — genuine Collector-Liveview, Detach und Publicationpins

**Rootfreigabe: Sim-Produktion innerhalb des folgenden Vertrags.** Exklusiv
`server/crates/ferrumc-sim/**`, `.codex-tmp/sim03_4a/**` und ein neuer
`SIM-03_4a_COLLECTOR_LIFECYCLE_DELIVERY.md`. Keine andere Produktionscrate,
Dependency-/Lock-/Generated-/App-/Session-/Storageänderung; kein Git oder
zusätzlicher Agent. Root integriert und prüft unabhängig.

Verbindliche vollständig zu lesende Grundlage ist NET-04.2c, Brief-SHA256
`3dff58ac5d1b9511862cf050018fb838ea5f8372b8768fd56a661ab949efd04c`, plus seine
Rootprüfung. F1/F2 und beide Countpräzisierungen sind geschlossen. Die aktuelle
abgenommene Basis ist main `d3fca66c`, mit DUR-03.2 und SIM-03.4 `b9f15db8`.
Der ursprüngliche SIM-03.4-Brief und alle alten Design-/Delivery-/Rootartefakte
bleiben immutable. Neue Änderungen sind ein eigenständiges Addendum.

Implementiere den gesamten dort festgelegten **Simteil**, keine bloße Factory:

- `connected_owned_player_view` borrows tatsächlichen Player-/Ownershipzustand,
  prüft private Owneridentity, ursprüngliche Admission, ganze aktuelle Generation,
  active Ownership, connected und nicht stopped. Drain erlaubt den connected
  Outputverbrauch. SnapshotOnly/foreign/stale/detached verweigern. Native
  Position/Look/GameMode und bestehender `BiomeZoomSeed`-Hashproducer liefern die
  tatsächlichen Werte; kein Callerbool/Hash oder neuer Worldseedalgorithmus.
- Vor jeder echten Restore-/Bootstrapadmission reservierte begrenzte Lifecycle-
  Zellen; neue Rejoinadmission erhält eine neue Zelle. Cap ist checked
  owned-player-limit + Publicationgruppenlimit mit positivem höchstens1MiB-
  Scalarbytecap und fallible vorreserviertem tatsächlichem Layout. Keine neue
  FCIS-/NBTkopie oder unbounded historische Liste. Explizite Opt-ininitialisierung
  erfolgt auf leerem tatsächlichem Owner vor Admissions.
- Ein sealed `NativePlayerDetachFact` entsteht direkt beim wirklichen Apply von
  `OwnedPlayerInput::disconnect`, nachdem genau diese Admission disconnected ist.
  Wiederholung liefert denselben behaltenen Fact; alte Disconnects mutieren nach
  Rejoin niemals den neuen Spieler. Bestehende UUID-only Intents sind kein Proof.
- Jeder tatsächlich vorbereitete `take:Some` hält genau einen Pin seiner originalen
  Lifecyclezelle: staged/reserved/retained Übergänge und Release gemäß NET-04.2c.
  Confirm bewegt, definitive Abort und echter FIFOheadack releasen genau einmal;
  Busy/Uncertain/Refresh/Quarantine/Detach halten vollständig. Reserved plus
  retained Pins überschreiten das bestehende Groupcap nicht. Pinadmission liegt
  vor Bodycopy/Workrequest. Der wirkliche Countershadow verwendet dieselben
  Übergänge und preflightet vor weiteren Ressourcen/Mutation.
- Historische `collector_lifecycle_view` borrows echte Ledgerzellen auch nach
  Rejoin. Kein public Constructor/Clone/Weakgetter/Mutationshandle aus Facts.
  Detached + sourcepins0 erzeugt das genaue Lifecycle-Ticket; Ack revalidiert
  owner/admission/generation/Zellenidentität und 0pins. Sessionretirement und
  Consumerrefs0 sind separat vom tatsächlichen Driver vor Ack zu beweisen;
  das Simticket behauptet keinen Socketreceipt. Stop/Quiescence berücksichtigt
  ungelöste Lifecyclecredits; Clone/SnapshotOnly enthält keine runnable Facts.

Lies AGENTS/CLAUDE, Sim-INVARIANTS, Architektur und PAPER_REFERENCE sowie die
vollständigen betroffenen heutigen Restore-/Candidate-/Release-/Confirm-/Ack-/
Countershadowpfade. Bestehende Conservation, Gen-/Tick-/FIFO-/Creditverträge
bleiben erhalten. Neue APIs bekommen Rustdoc und klassifizierte Fehler.

Meaningful Tests verwenden tatsächliche Memory/redb-Bootstrap-/Restore- und
Pickupadmissions, echte Handles/Groups und Work-/Completionpfade. Prüfe native
Survival/Creative/Pose/Seedbits, foreign/same-UUID/different-owner/snapshot/stale,
connected während Drain, wirklichen Detach/Rejoin und alte Facts; reserved
Pickup→Detach→Confirm→Headack, definitive Abort, Busy/Uncertain/Quarantine,
stale/foreign/doppelte/reordered Acks, exakte N-1/N/N+1-Zellen-/Pin-/Bytegrenzen,
Countershadow und wirklichen Drain/Stop. Keine testbare arbitrary Faktfactory.

Benutze für unabhängige finale Autorenchecks eine private Kopie des echten
Rootarchivs `.codex-tmp/gen05_7-root-runtime-base/archive` (1224 Dateien,
Manifest `c148936b29cc58d7ac25b2904618523c3b3a163476a2bf380b1d00fadf07c7b0`),
überlagere ausschließlich den ganzen finalen Simstand und prüfe Outsidebytes.
Shared World-WIP ist keine neue abgenommene Dependency. Rootarchiv readonly.
WholeSim Debug/Release, alltargetclippy -Dwarnings, Workspacefmt --check und
Rustdoc -Dwarnings; Source-/Evidencefreeze und präzise Delivery danach. Vor
unlösbarem Vertragsproblem an Root melden; keine stillen Scope-/Authority-
Erweiterungen. Dies aktiviert keine App, Itemphysik, Plugins oder Live-Survival.
