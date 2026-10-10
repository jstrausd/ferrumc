# STORAGE-ENTITY001.1 — Begrenzter UUID-Entity-Speicher

Status: vergeben und in Umsetzung an `select_world_ticket`; das abgeschlossene
SIM-03.2-Fremdreview ist CLEAN. Root hat den präzisierten API-/Mode-/Validatorplan
einschließlich R1–R4 freigegeben. Produktion ausschließlich `ferrumc-storage`,
keine Gameplay-/Appaktivierung.

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
