# DUR-03.3 — gemeinsame Player- und Blockdurability

Status: konkrete Storageabhängigkeit identifiziert, noch nicht vergeben oder
runtimefreigegeben. Ein Autor besitzt ausschließlich `ferrumc-storage` und
diesen Brief. DUR-03.2 besitzt derzeit exklusiv Storage; kein Parallelwriter.
Simplacement und Appverdrahtung folgen als getrennte Consumer.

## Tatsächlicher fehlender Vertrag

Root hat den vorhandenen WorldStore-,PlayerStore-,Ownership-,Chunk-/Overlay-
und BlockMutationLogRecordvertrag gelesen. Player/Entity-CAS ist gemeinsam
atomar. Chunk-/Overlaywrites, Journalappend und Playerwrites sind dagegen
getrennte öffentliche Aufrufe. ChunkRecord.schema_version und Overlay.updated_at_tick
sind keine vom Backend ausgestellte CASrevision. UUID-Entitychunkepochs
beschreiben Entityindexänderungen, nicht sämtliche Block-/Containerwrites.

Beim Survivalplacement muss derselbe bestätigte Vorgang das Item verbrauchen
und den tatsächlichen Blockdelta dauerhaft machen. Zwei einzeln erfolgreiche
Aufrufe lassen an Crash-/Lostreply-Cutpoints Verlust oder Dupe zu. Ein Save
nach sichtbarer Mutation oder ein freier Callerhash behebt diese Lücke nicht.
Die produktive Format6-Umschaltung in DUR-05 darf dafür keine Legacyauthority
zurücköffnen.

## Konkreter Entwurf vor Umsetzung

Der Storageautor prüft echte bestehende Memory-/redb-Writer und erstellt eine
kleine versionierte API-/On-disk-/Migration-/Receiptgrammatik für gemeinsame
Player-plus-Blockmutation. Genuine expectations müssen die actual Backendowner-,
Generation-,Player- und Chunkbaselines binden. Auch normale Block-/Container-
Writer müssen dieselbe Chunkrevision respektieren; ein separater CASpfad neben
unbeschränktem Overwrite wäre kein vollständiger Vertrag.

Der Request besitzt eine echte Operationidentity, begrenzte Originalpostimages
und die gesamte betroffene Chunkunion, einschließlich Türhälften/Nachbarupdates.
Ein Scope darf nicht aus einem unscoped Journalrecord erschlossen werden:
der derzeitige BlockMutationLogRecord führt BlockPos,Actor,Cause und Tick,
aber keine World-/Dimensionidentity. Dieser Unterschied braucht ausdrücklich
geprüfte Darstellung oder separate Producerabhängigkeit vor Dimensionsbetrieb.

Eine tatsächliche Writertransaction bestätigt alle Beforeimages, schreibt
Playerpostimage plus genaue Block-/Blockentity-/Journalwirkung und bindende
Receipt atomar. No-op, Konflikt, IDcollision, History-/Revisionexhaustion,
Request-/Batch-/Bytecaps, QueueBusy und CommitUncertain sind typisiert.
Lostreply/Receiptclarification/Retry behält dieselbe Operation und kann keine
zweite Blockwirkung oder zweiten Itemverbrauch auslösen. Neue permanente
Grammatik benötigt echte Fresh-/Reopenvalidation und explizite Bestandsmigration;
vorhandenes Format6 wird nicht stillschweigend umgedeutet.

Die konkrete Darstellung und Limits sind noch zu prüfen. Dieser Brief erklärt
keine neue Receiptkind,Revisionauthority oder Formatnummer für freigegeben.
Er verhindert, dass Appwiring eine fehlende gemeinsame Transaktion verdeckt.

## Abnahme und Consumer

Reale Memory-/redb-Prepare/Commit/Reopen-/Lostreplyfälle, konkurrierende Spieler,
mehrere betroffene Chunks und vollständige Komponenten. Alle tatsächlichen
Pre-/Postcommit-Cutpoints erhalten entweder den ganzen alten oder ganzen neuen
Bestand; exakt eine Receipt bindet Itemmenge,Blockstates,Blockentities und
Journalwirkung. Unabhängiger Erwartungsbestand, N/N+1-Caps und Drift-/Foreign-
Backend-/Generationfälle. WholeStorage Debug/Release und strictchecks.

Danach folgen eigener Simconsumer für Owned Block-/Container-/Commandaktionen
und zuletzt der Appdriver. Miningloot, ItemEntities, Fluids und Redstone erweitern
die Transaktionsunion über eigene konkrete Tickets. Keine Survival-/Plugin-
Fertigmeldung aus einem Storageproducer.
