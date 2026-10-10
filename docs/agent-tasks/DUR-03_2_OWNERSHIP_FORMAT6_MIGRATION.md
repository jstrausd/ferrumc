# DUR-03.2 — explizite Bestandsmigration in die Format6-Ownershipauthority

Status: vorbereitet, noch nicht vergeben. Voraussetzung vor produktiver
DUR-05-Adoption vorhandener Daten. Ein Autor besitzt ausschließlich
`server/crates/ferrumc-storage/**` und diesen Brief. App-/Sim-/Config-/Items-
Änderungen, vorhandene generierte Daten und Git bleiben ausgeschlossen.

## Tatsächliche Lücke

Die produktive App startet Format4. Der geprüfte UUID-Entityproducer besitzt
Format5, der Player-/Entity-Ownershipproducer benötigt Format6.
`RedbStore::open_with_player_entity_ownership` verweigert Format3..5 ausdrücklich.
Die vorhandene UUID-Migration4→5 bewahrt Player-/Journal-/Chunkbytes, erzeugt
aber keine Player-Scopezuordnung, Ownershipheader oder Ownershipreceipts.
Ein Formatmarkerwechsel oder ein leerer neuer Store würde vorhandene
Inventare, Entities und ihre dauerhafte Autorität nicht korrekt übernehmen.

## Konkreter geforderter Producer

Vor Runtime einen vollständigen bounded API-/Datei-/Row-/Commit-/Resumeentwurf
liefern. Ein enger erster Producer kann ausschließlich gültiges Format5→6
besitzen; Format4 erreicht Format5 über den bereits geprüften eigenen
Offlineproducer. Kein automatisches Upgrade oder Mehrformat-Sammelumbau.

Jeder vorhandene PlayerUUID braucht eine ausdrückliche unveränderliche
GenerationKey-Zuordnung aus einer begrenzten Mappingquelle. Der UUID-keyed
Playerrecord allein enthält diese welt-/dimensionsgebundene Authority nicht.
Missing-, doppelte-, fremde-, widersprüchliche- und ungenutzte Zuordnungen
erhalten typisierte Regeln; keine stillschweigende globale Defaultdimension.
Die zugehörige vollständige GenerationIdentity muss tatsächlich persistiert
sein. Bereits UUID-identifizierte Entities haben ihren tatsächlichen Scope
und Chunk aus den gültigen Format5-Record-/Locator-/Indexdaten.

Das vollständige Format6-Ergebnis braucht seine exakte15-Table-Grammatik,
persistierte Capacityagreement, Player-/Entityheader, Revision-/Receiptcounts
und passende echte historische Bootstrapbelege. Die vorhandene Grammar
verknüpft Header mit Receipts; freie erfundene Revision1header ohne diesen
Zusammenhang sind kein zulässiges Ergebnis. Ein Migrationreceipt muss seine
aus dem eingefrorenen Sourcepayload hergeleitete Requestidentity, Scope,
Generation, Resourcekeys, before/after Stamps und Epochregeln exakt belegen.
Solche Offlinebootstrapbelege sind von einem bereits laufenden Simtransfer
zu unterscheiden. Eine neue Receiptkind-/On-disk-Grammatikänderung benötigt
vorher einen eigenen begründeten Vertrag und unabhängige Abnahme.

Playerpayload-/GameMode-/Schema- und vollständige FCUE/Entitybytes bleiben
verlustfrei; Chunks/Overlays, Generatoridentitäten, Plugins, Journalreceipts
und deren Max-ID behalten ihren geprüften Inhalt. Die Storage-Crate besitzt
keine FCPS1/2→3-Gameplaymigration; deren konkreter nachfolgender Appconsumer
muss native Payloadadmission vor Liveplayerzulassung übernehmen. Nicht
unterstützte Bestände werden vor Commit typisiert verweigert.

## Offlineauthority und Recovery

Die bereits unabhängig geprüften4→5-Verträge dienen als konkrete Grundlage:
exklusive Quelle, Pfad-/Inode-/Alias-/Symlinkprüfungen, echte closed Backupcopy,
unveränderlicher Checkpoint vor Sourcecommit, explizite positive Scratchcaps,
vollständige bounded Sourcevalidation, atomarer Gesamtdelta und exakte
postcommit/resume Validation. Keine In-place-Handedits, automatisches
Reparieren fremder Daten oder Ersetzen/Überschreiben vorhandener Evidenz.

Alle Mapping-/Player-/Entity-/Bootstrap-/Receipt-/Revision-/Bytecounts werden
vor Allocation und Commit gegen konkrete positive Operatorlimits geprüft.
Mapping und Quellenfingerprint sind im Checkpoint gebunden. Kein vollständiger
unbounded Payloadclonebestand und kein DB-I/O auf dem Simulationthread.
CommitUncertain bleibt ungewiss bis zum tatsächlichen readonly Resumecheck;
ein Fehler nach erfolgreichem Commit darf kein erneutes Bootstrap auslösen.

## Meaningful Abnahme und Verbraucher

Reale redb-Prepare/Commit/Reopen/Resumefälle, echte Sourcebytes vor/nach und
unabhängige vollständige Format6-Open-/Ownershipreads. Nichtleere Player und
Entities mit Komponenten, verschiedene Scopes/Generationen, vorhandene
Chunkepochs/Journal-/Plugin-/Overlaydaten und alle Mappingfehlformen.
Cap N/N+1, Header-/Receipt-/Count-/Indexfehlformen, source/mapping drift,
Backup-/Checkpointänderung, fremde/resumed/bereits migrierte Dateien,
Cutpoints vor jedem Write und vor/nach tatsächlichem Commit. Der genaue
zugelassene Delta muss independently herleitbar bleiben.

Whole-Storage Debug/Release, strict All-target-Clippy/Fmt/Rustdoc, exakter
Freeze und unabhängiger Review. App-CLI/Startflag/Driver erhält danach sein
eigenes App-Ticket. Dieses Storage-Ticket allein aktiviert weder Drop/Pickup
noch vollständiges Survival, Paperplugins oder Nether/End/Dragon.
