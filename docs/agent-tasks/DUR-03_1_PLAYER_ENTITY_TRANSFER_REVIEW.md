# DUR-03.1 — unabhängiger Rootreview

Ergebnis: **CLEAN**, zur Integration auf `main` freigegeben. Eine Produktionscrate:
`ferrumc-storage`. Der Runtimeproducer liefert explizite Format6-/CAS-/Receipt-
Authority; er aktiviert noch keinen Drop-/Pickup-/Appconsumer.

## Eingefrorener Stand

Autorenfreeze:21 Dateien gemäß `.codex-tmp/dur03_1-owned-files.txt`, Manifest
`.codex-tmp/dur03_1-author.sha256`, SHA256
`3b641f626b7d61ba3ada963d299d362e6591b052661d121b15a75cba1a3379d3`.
Alle21 Hashes vor Integration unabhängig geprüft. Alle20 Storagefiles stimmen
bytegleich mit dem isolierten Rootprüfcheckout überein. Autoren-Ergebnisreport
und sämtliche sechs Logpins wurden zusätzlich unabhängig nachgehasht.

Rootprüfcheckout `.codex-tmp/dur03_1-root-review-checkout` enthält Archivbaseline
`f92b58cf` plus ausschließlich den eingefrorenen Storagecrate und einen neuen
Rootcontracttest. Laufende Worldänderungen und andere Produktionscrates waren
nicht enthalten. Rust1.99.0/Edition2021, zwei Jobs, separates Roottarget.

## Code- und Vertragsprüfung

Root hat vollständige öffentliche Authority-/Request-/Error-/Capacitytypen,
CanonicalSHA-/Header-/Receiptgrammatik, gemeinsames Read/CAS/Planmodell, beide
realen Backendproducer, den eigenständigen vollständigen Format6validator und
sämtliche Mode-/Readonly-/Legacywritergates geprüft. Kein bestätigtes Runtimefinding.

Matching Receiptlookup precedes live Owner/CAS/Historycapacity; historische
Afterstamps bleiben historische Stamps. Revisionen/Tombstones behalten Scope
und verhindern ABA. Memory publiziert alle Pair-/UUID-/Header-/Receipt-/Counter-
Rows unter demselben Lock; redb benutzt eine tatsächliche Immediate-Transaction.
26 reale Precommit-Cutpoints, Postcommit-Lostreply/Workerpanic und echte
Commit-sync-I/O-Fehler sind konkret geprüft. Ungewissheit behauptet keinen Abort.
Worker behält seinen Permit nach verlorenem Caller; keine unbegrenzte Retryqueue.

Die vollständigen Format6scans prüfen bidirektionale UUID-/Ownerrelationen,
Lastchanging-/Historicalreceipts, genaue Catalog-/Metagrammatiken und echte
Journalranges. Bestehender Format5validator und beide Migrationen unverändert.
Typed Raw-/Codec-/Engineparser werden wiederverwendet, keine Klassifikation
anhand von ServerErrorstrings. Tatsächliche Scan-/Scratch-/Requestgrenzen,
MAXrevisionen/-epochs und readonly UUID-Lookahead sind abgenommen.

Die16 Referenzpins umfassen11 vollständige Appliedquellen und5 Sourcepatches
für tatsächliche Drop-/Pickup-/Inventory-/Save-/Moonrise-Pfade. Alle Hashes
unabhängig geprüft; Source-/Eventbefunde bleiben Quellenlektüre, kein gestarteter
Papervergleich und keine Plugin- oder Itemkonservationsabnahme dieses Bytestores.

## Unabhängige Prüfungen

Root Wholecrate Debug/Release jeweils **280 bestanden**:214 Unit,54 Integration
inklusive zwei zusätzlicher Rootprobes,12 Rustdoctests. Alle278 Autorenfälle
bestanden in beiden Profilen. Die final strukturierten Rootprobes wurden danach
nochmals gezielt in Debug/Release geprüft. Strict all-target Clippy, Wholeworkspace-
Fmt und strict Rustdoc ebenfalls erfolgreich. Ein erster Clippylauf meldete nur
die Länge des neuen Roottesthelpers; Aufteilung in konkrete Testphasen behob ihn
ohne Suppression oder Runtimeänderung. Alte und finale Logs bleiben erhalten.

Neuer committeter PublicAPItest `tests/root_ownership_contract.rs`, SHA256
`a29445c2fae6d2d4c94198fe50edea1ae41ff30983ceca2085ac522a162be4e4`:

- Identische Canonicalanfrage eines inzwischen zerstörten fremden Backends
  replayt Originalevidence vor Owner/CAS, nach späteren Writes und bei voller
  Receiptcapacity; geänderte SHA kollidiert vor Ownercheck.
- Eine verweigerte fremde Anfrage konsumiert ihre ID nicht. Der echte Owner
  kann anschließend diese ID mit einem gültigen aktuellen Request verwenden.
- Alte Replayexpectations überschreiben keinen aktuellen Playerzustand;
  verweigertes stale CAS hinterlässt keine Receipt. Exakt aktuelle Authority
  verwendet danach dieselbe ID erfolgreich.
- redbReopen bewahrt Payloads/Revisionen/Chunkeepoch und alle historischen
  Receiptantworten, ohne eine neue Menge oder eine alte Postimage zu publizieren.

Rootlogs unter `.codex-tmp/dur03_1-root-review/logs/`, separater Reviewfreeze
`.codex-tmp/dur03_1-root-review.freeze.json`. Keine zusätzlichen Tests oder
Performancebehauptungen aus wiederholten bereits bestandenen Gesamtläufen.

## Unmittelbare Verbraucher

DUR04.1 übernimmt jetzt die tatsächliche46-Slot-/Cursor-Playerauthority in Sim
mit Owner-/Clone-/Rejoin-/Scopegates, bounded Reservation/Pendingauftrag und
receiptgematchter gemeinsamer Veröffentlichung. DUR05 aktiviert anschließend
Appdriver und explizite Bestandsmigration. Capacityänderungen/Receiptretirement,
Crossdimension-/Mehrressourcentransfers, wirkliche Clients/Plugins, Processkill/
Powerloss und vollständige Survivalprogression bleiben eigene Folgearbeit.
