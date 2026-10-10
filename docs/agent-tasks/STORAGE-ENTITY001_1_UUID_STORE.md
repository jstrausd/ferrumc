# STORAGE-ENTITY001.1 — Begrenzter UUID-Entity-Speicher

Status: implementiert durch `select_world_ticket`, unabhängig **CLEAN** geprüft
durch `feature_catalog`, durch Root zur Integration freigegeben.
[Implementationreview](STORAGE-ENTITY001_1_UUID_STORE_REVIEW.md): Debug/Release
je212 Tests und neun eigene Probes, strict Clippy/fmt grün; 16/16 Freezehashes
bestätigt. API-/Mode-/Validatorplan einschließlich R1–R4 erfüllt.
Produktion ausschließlich `ferrumc-storage`, keine Gameplay-/Appaktivierung.

## Verbindlicher Umfang

Implementiere den kleinsten fresh/strict Format5-Producer aus dem
[Schemaentwurf](STORAGE-ENTITY001_UUID_PERSISTENCE_DESIGN.md) und dem
[unabhängigen Designreview](STORAGE-ENTITY001_UUID_PERSISTENCE_DESIGN_REVIEW.md).
Root akzeptiert die vier Reviewpräzisierungen R1–R4; diese Ergänzung hat bei
abweichenden alten Formulierungen Vorrang. Der vollständige 4→5-Migrationspfad
wird als **STORAGE-ENTITY001.2 separat** umgesetzt. Dieses Ticket öffnet
vorhandenes Format4 ausschließlich mit Migration-required-Refusal und verändert
es nicht. Die alte 3→4-Migration und ihre strikten Validatoren bleiben erhalten.

Exklusiver Dateibesitz: `server/crates/ferrumc-storage/**` und
dieser Brief. Keine alten Design-/Reviewdocs, andere Crates, Originaldaten,
Lockfile oder Git ändern. Der vor Production gemeldete Vertrag ist freigegeben:
separater dyn-kompatibler UUIDtrait mit privaten Value-/Cursortypen, explizite
UUIDmodekonstruktoren, gemeinsame begrenzte Codec-/Preflightlogik, Memory mit
einem owned UUIDstatelock innerhalb der bestehenden Mutationsgrenze und redb
mit einer Write-Transaktion pro Mutation. Cursor binden eine schwache Referenz
auf die Backendauthority; Clones teilen diese, Reopen erzeugt eine neue. Der
strict Format5-Opener validiert einmalig alle zwölf Tabellen mit begrenztem
Descriptorbestand und eigenem temporären Receiptindex (16MiB Cache). Erste
Page beginnt einschließlich des rohen Chunkprefixes, damit auch zu kurze
Prefixkeys auffallen. Runtimeoperationen prüfen nur angetroffene Beziehungen.

## Reservierte Regeln

- Format5 hat exakt zwölf Tabellen und die im Entwurf beschriebenen UUIDkey24-,
  Locator20-/12-, Chunkindex32-/0-, Epoch16-/8- und Envelope22-Grammatiken.
  Generatoridentity, Payloadschema und Format sind verschiedene Versionen.
- Neues dyn-kompatibles `PersistentEntityStore`, explizite UUID-Modusopener
  für fresh/strict redb und Memory; Legacy-/nicht adoptierte Producer geben
  Unsupported. Alle vier wire-keyed Entitymethoden verweigern im UUIDmodus
  auch leere Requests. Sonstige bestehende Storageverträge bleiben kompatibel.
- Atomare Savebatch/Delete/Dimensionmove mit exakt erwarteter Generatoridentity
  in derselben Mutationsgrenze. Keine Rawhandles oder IO auf Tokio-/Simworker.
- Online prüft alle angetroffenen und direkt adressierbaren Beziehungen,
  behauptet keinen globalen Orphanscan. Normaler redb-Format5-Start führt
  einmalig den vollständigen streaming Format5-Validator aus; kein Vollscan
  pro Runtimeoperation. Validator prüft alle zwölf Tabellen, beide Index-
  Richtungen, Descriptoren und unveränderte Journal-/Receiptinvarianten.
- Save bewegt innerhalb derselben Dimension. Move verlangt gleiche World/UUID,
  verschiedene Dimensionkeys, vorhandene Source und freies Ziel. Same-key/
  Same-scope und Crossworld verweigern. Batchduplikate nach Key und World+UUID
  verweigern; gleiche UUIDbits in verschiedenen Welten sind zulässig.
- Höchstens64 Writes/4MiB. Jeder distinct tatsächlich geänderte Chunk erhält
  pro Commit genau Epoch+1, höchstens128 betroffene Keys. Exact Rawno-op bleibt
  bei u64MAX zulässig, repariert keine Indizes; Änderung bei MAX verweigert.
  Deletefalse und Emptybatch im unterstützten Modus ändern nichts.
- Pages: Limit1..64 und exakt `24 + 22 + payload_len` je Entry, höchstens1MiB;
  maximal ein bounded borrowed Lookahead, kein Restchunk- oder Weltscan.
  Exklusive UUID-Fortsetzung im genauen Chunkprefix, Null-/Max-UUID-/Endranges
  ohne Wraparound. `next=None` beweist Prefixende unter Encounteredprüfung.
- Cursor privat: Backendincarnation, Chunkkey, volle expected Identity, Epoch,
  letzte UUID. Clones derselben Backendauthority akzeptieren; anderer Backend/
  Reopen/Chunk/Identity/Epoch verweigert vor Ergebnispublikation. Leere historische
  Chunks behalten Epoch; keine GC oder Rücksetzung. Epoch ist kein Ownership-CAS.
- Memory mutiert alle UUIDdaten/Indizes/Epochs gemeinsam unter der vorhandenen
  owned Mutationsgrenze und exakter Generationadmission; keine Guards über
  Await. redb preflight/commit derselben Operation wirklich atomar.
- Globale Generationfreshness umfasst neue Records/Locator/Index/Epochs;
  historische Epochs erlauben keine frische Worldbaseline. Opaque Entitybody
  bleibt uninterpretiert; Simcodec/Chunkloadreservation folgen getrennt.
- Save/Delete/Move sind noch keine replayfesten CAS-/Transferoperationen.
  Keine autonomen Gameplay-/Autosaveretries oder Player↔Entity-Transferack
  aktivieren. DUR-03 braucht eigene Versionen/Tombstones/Receipts und alle Writer.

## Quellen und Abnahme

AGENTS/CLAUDE, Storage-INVARIANTS, Architektur/ADRs, vollständige Referenzquellen
und beide Designdokumente lesen. MC1.21.8/772, PaperBuild60/Commit
`29c8822d90899c89d2689338e81a98f690bcba12`; Quellenmatrix und zusätzliche
FerrumC-Policies erfassen. Keine Live-/Powerloss-/Itemconservationbehauptung aus
reinem Source- oder Transactionaborttest.

Abnahmematrix des Designbriefs gilt für **API/Codec, Writes/Index, Pages,
Memory/redb, Journalregression und Werkzeuge** einschließlich R1–R3-Negativfällen.
Migrationfälle dieses Tickets: fresh Format5, strict Reopen/Fullvalidator,
Format3/4-/fremder Catalog-/malformed Index-/Descriptorrefusal mit erhaltenen
Rawrows. Keine Implementierung oder Fertigmarkierung der 4→5-Migration.
Producer-Cutpoints/Lostreply und parallele konsistente Reads mit deterministischem
Harness; Header-/Count-/Byte-/Epoch-/Cursorgrenzen und Fehler-No-op prüfen.

Ganze Storagecrate plus strict Alltargetclippy/fmt; genaue Logs und Dateifreeze
an Root. Keine fremden bewegten Zwischenstände als finales Dependencyfinding
verwenden: Root stellt bei Bedarf isolierte Basissourcen bereit. Root organisiert
Fremdreview, Integration und das anschließende Migrationticket.

## Implementierter Producervertrag

`PersistentEntityStore` hat sechs dyn-kompatible asynchrone Methoden:
`load_persistent_entity`, `save_persistent_entity`,
`save_persistent_entities`, `delete_persistent_entity`,
`move_persistent_entity` und `load_persistent_entity_page`. Alle Defaults
verweigern Unsupported; `with_uuid_entities`/`open_with_uuid_entities` sind
explizite Moduseinstiege. Öffentliche Key-/Record-/Write-/Entry-/Epoch-/Page-/
Cursortypen besitzen private Felder und Getter. Kein Cursor-Rawkonstruktor,
kein Datenbankhandle und keine implizite Konvertierung von EntityId/PlayerId
werden veröffentlicht. Der komplette GenerationIdentity-Wert ist erforderlich.

Die gemeinsame Engine hält bisherige Payloads als Borrow/AccessGuard, prüft
anliegende Beziehungen und erstellt einen begrenzten Mutationsplan. Memory
wendet diesen erst nach vollständigem Preflight unter dem owned UUIDstatelock
an; nach Beginn gibt es keinen falliblen Returned-error-Schritt. redb nutzt
Preflight, alle Entity-/Locator-/Index-/Epochwrites und Commit innerhalb einer
Write-Transaktion auf `spawn_blocking`. Private Testcutpoints liegen nach jeder
wirklichen Rowmutation. Jeder distinct geänderte Chunk erhält genau eine neue
Epoche; ganze No-ops haben keinen Commit und historische Epochen bleiben stehen.

Paging nutzt einschließlich Chunkprefix16 beim ersten Scan und eine exklusive
UUID-Fortsetzung, prüft längenbegrenzte Beziehungen vor der Payloadkopie und
liest maximal Limit+1 Indexrows sowie einen borrowed Lookahead. Cursor bindet
Weak-Backendincarnation, kompletten ChunkKey, volle Identity, Epoche und letzte
vollständig emittierte UUID. `PersistentEntityCursorRejection` klassifiziert
Backend, Chunk, GenerationIdentity und Epoch. Scopefehler scheitern vor
Rowabfragen; Epochfehler vor dem Indexscan und jeder Ergebnispublikation.

Der neue vollständige Format5-Validator besitzt eigene zwölf Zähler und eine
versionierte logische SHA-256-Fingerprintgrammatik. Er verwendet keine alte
Caller-Deklaration, keine Defaultidentity und keine Ganzweltcollection. Der
bestehende neun-Tabellen-Migrationsvalidator wurde nicht verändert. Descriptoren
werden begrenzt gestreamt und scoped Rows per Punktlookup geprüft. Journal und
Receipts behalten Grammatik/Range-/Replayverträge; unscoped State verlangt
mindestens einen tatsächlichen Descriptor. Ein privater diskbasierter
Receiptindex sortiert nichtleere Ranges mit 16 MiB Cache. Er besitzt seine
Dateiidentität und entfernt nur die eigene unveränderte reguläre Datei nach
Freigabe der Handles; Replacement wird erhalten. Nichtleere Receiptprüfung
verlangt aktuell Unix-Fileidentity und verweigert andernorts ausdrücklich.

Haupt-redb-Cache: 64 MiB; Receiptindex-Cache zusätzlich 16 MiB. Eine begrenzte
decodierte Row, Input-/Encoding-/Plan-/Pagepuffer, redb-Interna und Map-/
Objektoverhead kommen dazu. Die 4 MiB/1 MiB beziehen sich exakt auf
`24 + 22 + payload_len` pro Entry, nicht auf den gesamten Prozessheap. Memory
hält seinen vollständigen gespeicherten Datenbestand. Keine RSS-/TPS-Messung
oder Gameplayperformance wird aus diesen Grenzen abgeleitet.

Root hat zusätzlich die erforderliche Format5-spezifische Pluginnamespace-
Admission von maximal 4096 UTF-8-Bytes freigegeben. Put/Get/Delete/Keys prüfen
sie vor Keyallokation und IO mit `UuidStorePluginIdTooLong`/Capacity. Validator
und Producer verwenden dieselbe Format5grenze. Generischer PluginId sowie
Legacy-/Metadata-Methoden bleiben unverändert; keine vermeintliche bisherige
allgemeine PluginIdgrenze wird behauptet. Alle anderen Format5-Schreiber
(Chunks/Overlays, Spieler, Journal/Receipts) behalten ihre geprüften Envelopes,
Längen und Descriptoradmission, die der neue Startupvalidator akzeptiert.

## Referenzmatrix und Abweichungen

Pin: Minecraft 1.21.8/772, Paper Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Die zwölf tatsächlich verwendeten
Basis-/angewandten Source-/Sourcepatchdateien sind mit SHA-256 eingefroren in
`.codex-tmp/storage-entity001-reference.sha256` (Manifest-SHA-256
`61d43cf3e4af7ba1bc1b451ba67943b17faa8ad3b8804d436a662d33ac7d0e89`).

| Quelle/Symbole | Tatsächlich gelesener Vertrag | FerrumC-Abgrenzung |
| --- | --- | --- |
| vollständige Basis und angewandte `EntityStorage`, `loadEntities`/`storeEntities` | Chunkorientierte Region-IO, Chunkpositionsprüfung, DataVersion und Entities-NBT; leere Entitychunks besitzen eigene Savebehandlung | UUID-Datenbanklayout, Lokator, Epochen und Pagecursor sind eigenständige FerrumC-Policies. Kein Vanilla-NBT-/Anvil-Import und kein Regionfile-Drop-in. |
| `Entity` UUID-/Savepfade; vollständige Basis, angewandte `ItemEntity` und Sourcepatch, Save/Load/Pickup/Merge | Persistente UUID und Itemdaten unterscheiden sich von Laufzeit-/Wirehandles; Timer/Owner-/Merge-/Pluginregeln liegen in der Simulation | Storage interpretiert den begrenzten Body nicht. Keine ItemEntity-Codec-, Pickup-/Merge-/Eventaktivierung oder Item-Conservationabnahme. |
| vollständige Basis und angewandte `PlayerDataStorage` plus Sourcepatch, `save`/`load`; `Util.safeReplaceFile`/`safeReplaceOrMoveFile` und Callees | Spielerdateien verwenden separaten Save-/Ladepfad und Dateiersetzung mit Backup-/Retrylogik | Kein Player↔Entity-Commit, keine Übernahme von Vanilla-Retries und kein Durabilitybeleg für FerrumCs eigene Entitytransfers. |
| Moonrise `ChunkEntitySlices.save/read/saveEntityChunk0`; `NewChunkHolder.saveEntities` und vollständiges `save` inklusive Shutdownzweig | Chunkentitysnapshot und Region-IO stehen unter Chunk-/Tickownership; Shutdown führt ausstehende Unloads aus und behandelt transient Entitychunks gezielt | Nur Storageproducer; kein Chunkloadreservation-, Dirtyack-, Autosave-/Unload-/Restartconsumer oder Simwriter-CAS. |

Dies ist eine Quellenprüfung plus FerrumC-Vertragstest, keine laufende
Paper-Differenzialabnahme. Keine Java-Events werden von diesem Purestorage-
Producer erzeugt; Bridge-/Gameplayereignisse verbleiben beim späteren Consumer.

## Abnahme und Grenzen

Die Implementierungstests umfassen:

- Opaque Compilefail-Verträge für Wire-/Player-ID, vollständige Identity und
  privaten Cursor; Bigendian-/Nil-/MaxUUID, exakte Envelopes, malformed/trailing/
  oversized Rows und volle Chunkorigin-/Offsetgrenzen.
- Memory/redb Save-/Delete-/Moveparity, WorldUUID-Duplikate einschließlich
  verschiedener Dimensionen im Batch, unabhängige Welten, fehlende/falsche
  Identity, Fehler-No-op, 64/65 Writes und exakte 4 MiB/+1-Byte-Admission.
- Pro Commit einmalige Epochen einschließlich genau 128 betroffener Chunks,
  Raw-no-op bei MAX und atomare Refusals bei realer Änderung, historische leere
  Chunks, Pagecount und exakter 1 MiB-Grenzfall mit vollständigen Header-/Keybytes.
- Ein borrowed Lookahead und höchstens Limit+1 Indexrows, Indexscan-Workcounter,
  Nil-/MaxUUID und all-FF-Prefix ohne Wrap, Backend-/Chunk-/Identity-/Epoch-
  Refusalklassen ohne Publish, Arcclone/Reopen/Weak-Lifetime.
- Gezielt angetroffene Memorykorruption und bekannte Orphanziele ohne Repair;
  unbekannte fremde Orphans bleiben dem Fullvalidator vorbehalten. Ein zunächst
  falsch global formulierter negativer Test wurde auf diese explizite Grenze
  korrigiert; das implementierte Pointverfahren wurde dafür nicht erweitert.
- Acht reale redb-Move-Schreibcutpoints mit unverändertem Rawdatensatz, verworfene
  Writer-Transaktion und strikter Reopen, verlorene Moveantwort und ausdrücklich
  fehlender allgemeiner Retry-/CASvertrag, deterministische konsistente Memoryreads.
- 25 Row-/Reverseindex-/Metadatakorruptionsfälle über alle zwölf Tabellen,
  fehlende/fremde/Multimap-/Typ-Catalogs, Descriptorcap, unscoped Player-/Empty-
  Receiptadmission, Receipt-Ranges/Nonoverlap/Cleanup/Replacements und leere
  historische Epochen nach Reopen.
- Legacy3/Metadata4 UUIDopener-Refusal mit allen Rawrows/Catalog erhalten;
  unveränderte gesamte 3→4-Migrations- und Journalregressionssuite.
- UUIDmode Pluginnamespace ASCII/Multibyte4096/4097 für alle vier Methoden,
  Fehler-No-publish, Memory/redb/Restartparity und unveränderte ältere Modi.

Finale Prüfung läuft ausschließlich auf Roots isolierter Basis `3a5b4d62`
unter `.codex-tmp/storage-entity001-isolated/`, mit eigenem externen Target
`.codex-tmp/storage-entity001-target`, `--offline --locked -j2`. Keine bewegten
World-/Registryzwischenstände oder Lockfileänderungen gehören zum Ergebnis.
Root organisiert das anschließende unabhängige Review und alleinige Gitintegration.

Finaler Producer-Nachweis auf isoliertem Commit `3a5b4d62`:
**212 bestanden, 0 fehlgeschlagen, 0 ignoriert**: 167 Unit-, 38 Integrations-
und 7 Doc-/Compilefailtests. Die Integrationsgruppen bestehen aus 7 Generation-,
2 Migrationboundary-, 13 redb-Roundtrip-, 9 WorldStorage-Vertrags- und 7 neuen
UUID-/Pluginparitytests. Die gesamten bisherigen Migration-/Journaltests sind
enthalten; die neun-Tabellen-Migrationsvalidierung bleibt unverändert.

Logs: `.codex-tmp/storage-entity001-final-tests.log`,
`.codex-tmp/storage-entity001-final-clippy.log` und
`.codex-tmp/storage-entity001-final-fmt.log`.
Test/Alltargetclippy laufen mit `--offline --locked -j2`, Clippy zusätzlich
`-- -D warnings`; Fmt ist auf `ferrumc-storage` einschließlich neuer Module
begrenzt. Der endgültige 16-Datei-Snapshot samt allen Filename-/SHA-256-Werten
liegt in `.codex-tmp/storage-entity001-freeze.sha256`; Main und isolierte
Abnahmebasis müssen vor/nach den Checks exakt übereinstimmen.

Noch offen: explizite STORAGE-ENTITY001.2-Migration und alle
Gameplay-/App-/Transferconsumer. Keine
Powerloss-, Item-Conservation- oder Drop-in-Survivalabnahme wird abgeschlossen.
