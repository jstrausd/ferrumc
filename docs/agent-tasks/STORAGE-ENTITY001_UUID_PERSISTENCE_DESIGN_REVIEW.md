# STORAGE-ENTITY001 — Unabhängiger Designreview

Stand: 10.10.2026. Reviewer `feature_catalog`; ausschließlich dieses Dokument
geschrieben, keine Storage-/Sim-/App-/World-/Gitänderung im Reviewauftrag.

Reviewbasis: `STORAGE-ENTITY001_UUID_PERSISTENCE_DESIGN.md`, SHA256
`b4380b3699b34f8b7bd242e3afe79117523e8b3b847d6f835fe0e8cfae80ea83`.
Der Hash wurde vor und nach Quellenlektüre bestätigt. Dies ist eine
Designprüfung, kein ausgeführter Format-5-Test oder Dauerhaftigkeitsnachweis.

## Ergebnis

**Bedingt freigabefähig als begrenzter Storageproducer.** Das vorgeschlagene
Format5, die getrennten UUID-/Locator-/Chunkindizes, persistente Chunkeepochs,
atomare redb-/Memorymutationen und die ausdrückliche Verweigerung unbekannter
Legacyidentitäten sind geeignet. Keine Wire-ID→UUID-Ableitung und keine
pauschale Paper-/Crash-/Itemconservationzusage werden eingeführt.

Vor Produktionsbeginn müssen die vier folgenden konkreten Vertragslücken
festgelegt werden. Sie erfordern keine neue Backendarchitektur, zusätzliche
Gameplaytabellen oder vorgezogenen DUR-03-Transfer. Root kann die unten
vorgeschlagenen kleinen Regeln als Ergänzung des Implementierungsbriefs
reservieren. Kein bereits laufender Format-5-Code wurde als fehlerhaft geprüft.

## R1 — Online-Encountered-Prüfung und vollständige Offlinevalidierung trennen

Der APItext verspricht Key-/Envelope-/Indexkonsistenz und „bestätigtes None“.
Eine einzelne UUIDabfrage hat jedoch keinen unbekannten Chunkkey: Bei fehlendem
Record und Locator kann ein verwaister Chunkindexeintrag irgendwo in derselben
Welt stehen. Auch ein einzelner Pageprefix findet keine zweite Chunkzuordnung
außerhalb dieses Prefixes. Ein voller Weltscan pro Read/Save würde den
ausdrücklich begrenzten Producervertrag verletzen.

Kleinste Präzisierung:

- Onlineaufrufe prüfen sämtliche **angetroffenen** Rows und ihre direkt
  adressierbaren Record-/Locator-/Index-/Epochbeziehungen im selben Snapshot.
  Ein fehlender Record mit vorhandenem Locator oder angetroffenem Index ist
  Korruption, keine Abwesenheit, Reparatur oder Deleteautorisation.
- `None`/Deletefalse bestätigt den angefragten scoped Record unter dieser
  Encounteredprüfung. Es bescheinigt keinen weltweiten Orphancheck. Ein gültiger
  Locator auf eine andere Dimension ist zunächst eine andere lokale Adresse;
  seine direkt adressierbaren Beziehungen dürfen nicht als korruptes Miss
  verschwiegen werden. Save desselben World+UUID in dieser anderen Dimension
  bleibt der ausdrücklich definierte Duplicate-World-UUID-Fehler.
- Vollständige bidirektionale Entity↔Locator↔Chunkindexprüfung gehört in den
  streaming Offline-/Migrationsvalidator; dort werden alle drei Tabellen und
  Epochrows zeilenweise mit Pointgets gegengeprüft, ohne World-Vec. Jede echte
  Entitychunkzuordnung braucht positive Epoch und bestätigten Descriptor.
- Der normale Format-5-Opener muss klar festlegen, ob er diese Vollprüfung
  ebenfalls einmalig ausführt oder nur Format/Catalog/Tabletypen plus die
  vorhandenen begrenzten Startchecks. Selbst bei einer Startvollprüfung bleibt
  der Onlinevertrag ausdrücklich Encounteredprüfung. Auf keinen Fall dürfen
  alle APIs den Offlinevalidator bei jeder Mutation aufrufen.

Gezielte Abnahme: bekanntes Missingrecord/Locator-/Indexproblem verweigert
online; unbekannter fremder Orphan wird vom Offlinevalidator erkannt, während
der lokale Read keine unerfüllbare globale Prüfbehauptung ausgibt. Same-world
und Crossdimensionrows, positive Epoch und Scopechecks einzeln prüfen.

## R2 — Pagebytes, Lookahead und Cursorabschluss genau definieren

1..64 Records und 1MiB sind sinnvolle Bounds. „Kodierte zurückgegebene Records“
muss aber die tatsächlichen 24Keybytes und 22Envelopebytes einschließen; ein
256KiB-Payload alleine ist kein vollständiger Record. Vier Maximalpayloads
passen mit Headern/Keys nicht in eine 1MiB-Page.

Empfohlener Vertrag:

- Bytezählung je Entry: `24 + 22 + payload_len`, checked summiert, höchstens
  1MiB. Recordcount und Bytebudget gleichzeitig einhalten. Page-/Cursorstruct-
  und Allocatoroverhead sind zusätzliche begrenzte Heapkosten, keine RSSparität.
- Prefixrange startet für Folgeseiten **exklusiv** nach der letzten ausgegebenen
  UUID und endet nach exakt diesem 16Byte-Chunkprefix. Falsche Prefixkeys,
  Lengths und angestoßene Beziehungen werden klassifiziert verweigert.
- Für End-/Byteentscheidung höchstens ein zusätzliches Indexentry/Record als
  Lookahead. Länge/Envelope prüfen, bevor dessen Payload kopiert wird; bevorzugt
  borrowed View im aktuellen redb-Readsnapshot. Keine Vorleseschleife über
  sämtliche restlichen Bewohner und kein Chunkvollscan für Pagecount.
- `next=None` bedeutet Prefix erschöpft; sonst Cursor mit letzter vollständig
  ausgegebener UUID. Eine nichtleere nächste Page kann immer mindestens einen
  gültigen Maximalrecord ausgeben. Nil-/Maximal-UUID sowie maximale Chunkkeys
  brauchen eigene Range-/Endtests, damit Prefixobergrenzen nicht wrappen.
- Incarnation, Chunkkey, volle erwartete WGidentity und persistenter Epoch
  werden vor Ergebnispublikation im selben Snapshot geprüft. Neuer Backend/
  Reopen verweigert alten Cursor; Arc-Kopien desselben Backendowners dürfen
  ihn verwenden. Last-delete und Recreate behalten/erhöhen Epoch; keine GC oder
  Wraparoundrücksetzung im Producer.

Das Dokument benennt diese Bausteine bereits weitgehend. Diese Präzisierung
schließt nur Bytezählungs-/Fortschrittslücken. Der spätere Simconsumer benötigt
zusätzlich ein begrenztes Gesamtbudget für die komplette Chunkloadreservation;
ein Pagebudget alleine begrenzt keinen materialisierten Gesamtchunkbestand.

## R3 — Same-key, Batchduplikate, Epoch und Rawno-op reservieren

`move` verlangt gleichzeitig vorhandene Quelle und freies Ziel. Bei gleichen
Keys ist das widersprüchlich; ein missverständlich implementierter Delete/
Insertpfad könnte trotzdem vorhandene Daten ersetzen. Außerdem ist bisher
nicht festgelegt, ob mehrere geänderte Rows desselben Chunks in einem Batch
seinen Epoch einmal oder mehrfach erhöhen.

Kleinste deterministische Regeln:

- Save ist derselbe scoped UUIDkey mit neuer Chunk-/Payloadzuordnung. Bewegung
  innerhalb einer Dimension, auch zwischen Chunks, erfolgt durch Save.
- Move ist ausschließlich explizite Dimensionverlagerung: World und UUIDbits
  identisch, unterschiedliche Dimension/Keys, Source vorhanden, Destination
  frei. Same-key/Same-scope-Move wird vor Writeränderung klar verweigert.
  Crossworld oder andere UUID bleibt Unsupported/Invalidrequest.
- Batchduplikate nach scoped Key **und** `(WorldId, UUID)` prüfen. Auch zwei
  identische Writes desselben Keys werden verweigert; zwei Dimensionwrites
  derselben Welt-UUID ebenfalls. Gleiche UUIDbits in verschiedenen Welten
  bleiben zulässig. Inputreihenfolge darf keinen Last-wins-Sonderfall erzeugen.
- Ein Commit erhöht jeden **distinct tatsächlich geänderten Chunk** genau um
  eins. Alter und neuer Chunk zählen je einmal; identischer Chunk ist ein Key.
  Der temporäre betroffene-Keybestand bleibt auf höchstens128 bei64 Writes
  begrenzt. U64MAX verweigert nur tatsächliche Änderung atomar; ein exact
  Rawno-op bleibt auch bei maximalem Epoch zulässig.
- Rawno-op bedeutet identische Key-/Envelope-/Payloadschemabytes und Bodybytes
  sowie bereits konsistente Indizes. Ein fehlender Index wird nicht durch
  einen „identischen Save“ repariert. Deletefalse erhält Epoch und alle Rows.
  Emptybatch ist im unterstützten Modus ein No-op; Unsupported Mode wird auch
  für leere Requests vor einem erfolgreichen Ergebnis verweigert.
- Diese Idempotenz gilt für dieselbe noch aktuelle Baseline. Unversionierter
  Save/Delete/Move ist **kein** replayfester Operation-ID-/CASvertrag. Retry nach
  zwischenzeitlichem neuerem Write/Delete/Recreate kann alte Daten überschreiben
  oder wiederherstellen; ein committed Move hat beim Retry keine Source mehr.
  Bis DUR-03 ist deshalb kein autonomer Gameplay-/Autosave-Retryconsumer
  freigegeben, der neuere Owneroperationen überholen kann.

Die Epochregel ist ein konkret vorgeschlagener Vertrag, kein behauptetes
Paperverhalten. Root kann alternativ bounded Mehrfachincrements reservieren,
muss dann ihre Count-/Overflow- und Samechunksemantik vor Code festlegen.

## R4 — Format-4-Validator und Format-5-Retryevidenz vollständig konkretisieren

Die vorhandene WG-002-MIG-Validation ist nicht ein generischer Format4checker:
sie verwendet `LegacyGenerationDeclaration`, eine explizite Flat-v1/Seed0-
Identität, höchstens9 Tabellen und feste12Byte-Entitykeys. Ihre Format4-
Retryprüfung verlangt exakt diese deklarierte Identität/Scopemenge; unscoped
Players/Journal erlauben dort nur einen deklarierten Scope.

Der neue 4→5-Schritt muss dagegen ein bereits gültiges Format4 mit seinen
**tatsächlich bestehenden**, gegebenenfalls verschiedenen Generatoridentitäten
und unveränderten unscoped Player-/Journalrows übernehmen können. Keine neue
Flat-/Seed0-Deklaration, nachträgliche Scopezuweisung oder gelockerte Altvalidation.
Der Vorschlag ist hier richtig versioniert, braucht aber folgende APIspezifikation:

- Eigener Format4/5-Validator mit eigenen12-Tabellen-Countern/Fingerprints und
  exakten Rowgrammatiken. Descriptorlookup gegen den bestehenden begrenzten
  Descriptorbestand; unveränderte Journal-/Receipt-/Watermarkchecks. Alte
  Prepare-API verweigert Format5 weiterhin.
- Im Format4-Quellzustand jede wire-keyed Entityrow verweigern. Vorhandene
  Chunk-/Overlay-/Player-/Plugin-/Generation-/Journal-/Receiptbytes bleiben
  exakt; Sourceprüfung/Backup dürfen keine decoderbedingte Normalisierung
  einführen. Leere Entitytabelle ist keine Erlaubnis, Descriptoren zu raten.
- Der neue Backupsnapshot muss auch die bestehende Generationtabelle kopieren.
  Der heutige `copy_snapshot` für 3→4 kopiert diese absichtlich nicht, da sie im
  dortigen Sourceformat nicht existiert. Gleiches gilt für Format4-Catalog und
  vorhandenen Formatmarker im logischen Fingerprint.
- Nach Lostreply oder Neustart braucht der checked Format5-Migrationretry
  ausdrücklich die vorher festgehaltene Pre-Migrationreport-/Backupidentity/
  Physicalhash-/Logicalsnapshot-Evidenz als Eingang. Ein erneutes
  `prepare_uuid_entity_migration(path)` ohne diese Evidenz kann exakte
  Vorher/Nachhergleichheit nicht aus dem Marker rekonstruieren.
- Minimal entweder separate `verify_completed_uuid_entity_migration(path,
  expected_report, backup_path, expected_backup_hash)` oder eine gleichwertige
  explicit checked Retryvariante. Sie vergleicht den vollständig validierten
  Format5-Source nach **exakter** Migrationdelta: Marker4→5 und genau drei leere
  neue Tabellen. Bereits gespielte Entity-/Epoch-/Indexdaten verweigern diesen
  identischen Migrationretry. Generisches gültiges Format5-Open ist etwas anderes.

Die create_new-/Lock-/Fileidentity-/Recoveryguard-/Backuphash-/fsync-/Typestate-
Grundlage kann wiederverwendet werden. Kein automatisches Rollback nach
mehrdeutigem Commit; keine Powerlosszusage nur aus Transactionabort/Reopen.

## DUR-03 bleibt eine eigene Versionierungs- und Writergrenze

Die Trennung im Vorschlag ist richtig. Chunkeepoch schützt Pages, nicht
Player-/Entityownership. UUIDkey und lokale Simhandle-Generation liefern
keinen langlebigen CAS. Der erste Transfer muss Player, Entity/Deletion,
Indizes, Ownershiprevisionen und Transferreceipt in einer echten Backend-
Writertransaction behandeln, und Receiptlookup erfolgt vor aktuellem CAS.

Ein zukünftiges `ExpectedAbsent` darf einen bekannten Tombstone nicht ohne
dessen Revision ignorieren: ein altes Absentread vor Create/Delete/Recreate
könnte sonst erneut passen. Benötigter Vertrag ist Fresh-never-seen-Abwesenheit
oder versionierte Tombstoneabwesenheit. U64Revisionen und Tombstones dürfen
nicht durch Delete/GC auf0 zurückfallen. Das ist eine abhängige DUR-03-Lücke,
kein Anlass, Format5 jetzt bereits mit unbekannten Transfergrammatiken zu erweitern.

Alle betroffenen Writer müssen im später aktivierten Ownershipmodus
versionsgeführt oder Unsupported werden, einschließlich neuem UUIDblindsave,
Player-save/delete und später Containeroverlay-Autosave. Increment allein
verhindert ein nachfolgendes blindes Überschreiben nicht. Receiptidentität/
Retention, Scopeführung bisher unscoped Players, gemeinsame Backendauthority,
Reservations, Lostreply/Disconnect und erst nach Ack sichtbare Take-/Remove-
Outputs bleiben explizite DUR-03/04/05-Consumerbedingungen.

## Kleinster sicher freigebbarer Producer

Nach Reservierung R1–R4: ausschließlich `ferrumc-storage`, typed UUIDkey/
checked Chunkheader/opaque EntityRecord, expliziter fresh/strict Format5-Modus,
Unsupported auf allen vier wire-keyed Entitymethoden in diesem Modus,
atomare bounded Save/Delete/Crossdimensionmove, Locator/Chunkindex/persistenter
Epoch und bounded Pagecursor mit gleicher Memory-/redb-Semantik. Erwartete
WGidentity wird in derselben Backend-Mutationsgrenze wirklich verglichen,
nicht nur auf Descriptorexistenz geprüft. Es gibt keine Rawhandles. Dass zwei
separat gelieferte WorldStore-/PersistentEntityStore-Dyn-Arcs dieselbe
Backendallocation vertreten, muss der spätere Consumer ausdrücklich binden;
das Entitytrait alleine beweist diese gemeinsame Authority noch nicht.

Der Offlinevalidator und der eigene strikte 4→5-Backup-/Migrationretryvertrag
gehören vor Übernahme vorhandener Welten zur Abnahme. Falls Root die Umsetzung
teilen will, kann der erste Producer ausschließlich fresh Format5 und
Migration-required-Refusal liefern; der explizite Migrationconsumer folgt
sequenziell in derselben Crate. Das ändert keine Daten eines vorhandenen
Format4-Stores und wird nicht als abgeschlossene 4→5-Migration ausgegeben.
Im ursprünglichen ungeteilten Ticket müssen beide Teile vollständig geprüft sein.

Keine Type-/UUID-/Transforminterpretation des opaque Body im Storage. Der
spätere Simcodec prüft inneren Type, UUID, Position↔Headerchunk, Simscope/caps,
Living-/Itempayload und vollständige Admission vor Publish; ungültige Bodies
werden nicht in Schweigen oder fehlende Entity übersetzt. SIM-03.2 besitzt nur
Basisregistrierungen und keine Disk-/Item-/Globaluuidauthority. Es ersetzt
keine Durability- oder Chunkloadreservation.

## Tatsächlich geprüfte Evidenz

Storage: vollständige production API-/Entitykey-/Record-/Generation-/Memory-
Verträge; konkrete redb Catalog/initialize/require_generation/Freshness und
vollständige Entity-/Playerread/write/delete-Pfade; vollständiger679-Zeilen-
Migrationsvalidator; vollständige Prepare/Backup/Reverify/Commit/Copy-/Staging-
Methoden und typisierte Report-/Counter-/Fingerprintverträge. Geprüfte Dateien:

- `server/crates/ferrumc-storage/{INVARIANTS.md,README.md}` und
  `src/{key.rs,record.rs,generation.rs,store.rs,memory.rs,codec.rs,redb_store.rs}`.
- `src/redb_store/migration.rs`, `migration/validation.rs`, öffentliche
  `tests/generation_migration.rs` und relevante Metadata-Contracts.
- SIM-03.2-Owner-/Read-only-/Capacity-/Incarnation-/Residencyvertrag aus Taskdoc
  und eigener eingefrorener Production; WG-003 bleibt separate Storebinding.

Pinned Originalreferenz: MC1.21.8/772, PaperBuild60,
`29c8822d90899c89d2689338e81a98f690bcba12`. Vollständige Vanilla-Basis und
angewandte EntityStorage/PlayerDataStorage; vollständiger PlayerDataStorage-
Sourcepatch; vollständige relevante Entity.saveAsPassenger/save/saveWithoutId/
setUUID/getUUID-Methoden; aktive Moonrise ChunkEntitySlices.save/saveEntityChunk0,
NewChunkHolder.save/saveEntities, PaperHooks.modifySavedEntities/unloadEntity,
zugehöriger vollständiger Featurepatchabschnitt; Util.safeReplaceFile/
safeReplaceOrMoveFile und alle verwendeten Retry-/Rename-/Delete-/Checkcallees.

Die Referenz bestätigt getrennte chunkweise Entityregiondaten und UUIDweise
Playerdateien, Saveflags/Passengers und Paper-spezifische Save-Limits. Sie
beweist keine gemeinsame Player↔Entity-Disktransaction. FerrumCs strikte
Corruptionrefusals, Ressourcenlimits, CAS und Receiptvertrag sind eigene
Verträge. Keine neu ausgeführten Storage-/Format5-/Paper-Live-/Crash-/Lasttests
in diesem reinen Designreview; das im Entwurf benannte unabhängige Testpaket
bleibt vor Integration Pflicht und wird um R1–R4-Fälle ergänzt.
