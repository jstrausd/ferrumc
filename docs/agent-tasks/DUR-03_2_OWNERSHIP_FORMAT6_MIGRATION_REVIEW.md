# DUR-03.2 — unabhängige Storageabnahme

Ergebnis: **CLEAN für den Offline-Storageproducer**. Bestehende Format5-Daten
können mit ausdrücklich deklarierter Player-Scopezuordnung auf Format6
migriert werden. Der produktive Appstart bleibt ein getrenntes Ticket.

Root prüfte alle elf neuen Produktionsmodule, die Änderungen an bestehenden
Einstiegen und Dateihelfern sowie die neuen Fehler-/Recoverytests vollständig.
Die drei zuletzt geänderten Produktionsmodule wurden erneut gelesen; die
übrigen acht geprüften Produktionshashes waren unverändert. Es bestehen keine
offenen Reviewfindings auf diesem Umfang.

Der Prepareeinstieg verlangt bereits das ausdrücklich gewählte
Scratchverzeichnis. Die eigene private Format6-Stage verwendet tatsächliche
Fresh→Live-CASreceipts. Ein einziger Source-Writertransaction importiert die
geprüften Headers/Receipts und bestätigt den Sourcefingerprint erneut.
Entitypayloads, Locator, UUIDindex und Journal bleiben erhalten; Chunkepochs
steigen checked um die tatsächliche Entityzahl. Die vollständige
15-Table-Grammar und der erwartete Targethash werden vor Commit geprüft.

Immutable Stage/Backup werden ausschließlich über eigene begrenzte Prüfkopien
geöffnet. Identitygebundenes Cleanup erhält fremde Ersatzdateien. Hash und
Copy binden die Anfangslänge und den Physicalcap; nach höchstens einem Byte
Lookahead erfolgen erneute Identity-/Längenprüfungen. Fehler nach tatsächlichem
Sourcecommit liefern Uncertain und können als AlreadyCommitted fortgesetzt
werden. Die beiden neuen tatsächlichen Postcommit-Stagecopytests prüfen dabei
das vollständige Target und alle fünf unveränderten Evidenzdateien.

Root erzeugte ein frisches Archiv von
`b87d5b61ad71e722602f66b7a7dc1e614f35dc1b` und überlagerte ausschließlich den
vollständigen61-Datei-Storagekandidat mit Manifest-SHA256
`a602d69bd775b59d4c9c19d3e07dca2e76d14eb9966bf17fe0139888d3c04959`.
1152 außerhalb liegende Archivdateien und sämtliche geprüften Quellen wurden
vor/nach kontrolliert. Zwölf der14 Originalproducer sind bytegleich; die
interne Freshreader-Sichtbarkeit und die bounded Dateihelperänderung wurden
gesondert geprüft. Die übrigen43 alten Storagedateien sind unverändert.

Unabhängige Rootprüfungen auf genau diesem Archiv:

- Debug und Release jeweils233 Unit-,54 Integration- und12 Dokutests:
  **299 bestanden**, keine fehlgeschlagenen oder übersprungenen Fälle.
- Strict All-target-Clippy und Rustdoc mit `-D warnings` bestanden.
- Workspaceformatprüfung bestanden.
- Finaler62-Datei-Sourcefreeze,36-Datei-Evidenzfreeze und das vollständige
 62-Datei-Quellarchiv frisch gehasht; keine Abweichung. Alle zehn zusätzlichen
 Deliveryartefakthashes stimmen überein.

Rootartefakte stehen unter `.codex-tmp/dur03_2-root-review/`, insbesondere
`final-source-review.json`, `final-delivery-hashcheck.json`,
`checks-assessment.json`, `final-assessment.json` und die fünf tatsächlichen
Prüflogs. Autorenprüfungen auf ihrer älteren Basis bleiben getrennt erhalten.
Der ursprüngliche Designhash `4e66fd1b…` und alle Originalproducerpins bleiben
unverändert; der Designhash bleibt die tatsächliche Parservertragbindung.

Die zwei gleichzeitig möglichen Scratchindices besitzen zusammen eine
physische Obergrenze von2×Scratchbytes; Plan und höchstens eine Prüfkopie haben
eigene Grenzen. Das belegt weder Gesamt-RSS noch Filesystemallocatedbytes.
Die Migration aktiviert keine Itemphysik, gewöhnlichen Blockconsumer,
Survival-, Java- oder Dimensionsfunktionen. Gemeinsame Player-/Blockdurability,
Sessionadmission und die Format6-Appumschaltung bleiben erforderlich.
