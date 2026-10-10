# DUR-03.1 — Atomare Player↔Item-Entity-Ownership

Status: **nächste Survivalvergabe geplant; konkreter API-/Schemaentwurf vor
Produktionscode erforderlich**. Root vergibt diesen Auftrag erst nach einem
freien Childslot. Dies ist der verbindliche Auftrag für den Entwurf, keine
Freigabe ungeprüfter Tabellen oder öffentlicher APIs.

## Ziel

Eine begrenzte Storageoperation überträgt einen Player- und einen UUID-
Entitysnapshot atomar mit dauerhaften Revisionen und Transferreceipt. Drop,
Teilpickup und Fullpickup müssen danach einen sicheren Sim-/Appconsumer
erhalten können. Der Storageproducer behauptet keine semantische Itembilanz.

## Crate und Dateibesitz

Ein später zugewiesener Autor besitzt ausschließlich `ferrumc-storage` und
diesen Brief. Root besitzt Git, Board, Katalog und Integration. Bestehende
STORAGE-ENTITY001.1/.2-Files bleiben bis zu ihrer Abnahme eingefroren.
Keine Items-/Sim-/App-/Core-/Lockfileänderung und kein generisches
Transaktionsframework. Neue Identitätswerte dürfen zunächst konkrete private-
field Storagewerte sein; ein anderer Crateproducer erhält bei Bedarf ein
eigenes Ticket.

## Kontext

- [Storagearchitektur](../architecture/storage.md), [Backendentscheidung](../adr/0004-storage-backend.md)
  und `server/crates/ferrumc-storage/INVARIANTS.md`.
- [UUID-Designreview](STORAGE-ENTITY001_UUID_PERSISTENCE_DESIGN_REVIEW.md#dur-03-bleibt-eine-eigene-versionierungs--und-writergrenze)
  und [UUID-Design](STORAGE-ENTITY001_UUID_PERSISTENCE_DESIGN.md#dur-03-als-eigenes-späteres-producerpaket).
- UUIDproducer `.1`, explizite Format4→5-Migration `.2`, SIM-03.3s sealed
  Itemstate/FCIE und ITEMS-TRANSFER-001s verlustfreie Transferpläne.
- [Paperreferenz](../PAPER_REFERENCE.md): vollständige ItemEntity.playerTouch,
  Player.drop und Inventory-Aufnahme-/Dropcallees sowie Savepfade/Patches lesen.
  Event-/Cancellation-/Threadordnung für spätere Consumer dokumentieren.
  Originaldateisaves sind keine atomare redb-Transferreferenz.

## Verbindliche Verhaltenseckpunkte für den Entwurf

1. Genau ein Player und eine UUID-Entity im ausdrücklich geprüften gleichen
   World-/Dimensionscope. Ein tatsächlicher Backendowner führt Receiptlookup,
   beide CAS-Prüfungen, Player-/Entitywrite oder Deletion, alle UUIDindizes,
   Chunkeepochs, Ownershiprevisionen und Receipt in einer realen gemeinsamen
   Writertransaction aus. Memorybackend besitzt dieselbe atomare Grenze.
2. Dauerhafte Revisionen werden vom Storage vergeben, sind positiv und wrappen
   nicht. Delete hinterlässt einen revisionierten Tombstone. Never-seen absence
   und versionierte Tombstoneabsence müssen verschieden sein; kein ABA nach
   Create/Delete/Recreate und keine stille Rücksetzung durch GC.
3. Receiptlookup geschieht vor live CAS. Ein identischer bereits committed
   Retry liefert die ursprüngliche Outcomeevidenz, auch nach späteren Writes.
   Dieselbe Transfer-ID mit anderer Requestidentität verweigert vollständig.
   Der Entwurf muss exakte bounded Canonicalbytes oder ausdrücklich benannte
   kryptografische SHA256-Identität wählen und den Ressourcenvertrag begründen.
4. Receipts dürfen nicht still ablaufen oder gelöscht werden, sodass alte IDs
   erneut ausführbar werden. Expliziter bounded Admission-/Retentionvertrag
   mit typisiertem Capacityfehler; sichere Retirement-/Replaygrenzen bei Bedarf
   separat. Ein Lostreply ist kein Abortbeweis.
5. Jeder Writer, der die beiden Ressourcen überschreiben könnte, ist im
   neuen Ownershipmodus versionsgeführt oder Unsupported. Insbesondere keine
   blinden UUIDsave/delete/move-, Player-save/delete- oder Autosavepfade neben
   CAS. Revisionsincrement allein verhindert konkurrierendes Überschreiben
   nicht. Alte Modi behalten ihre vollständigen bisherigen Verträge.
6. Playerrecords sind heute unscoped. Neue dauerhafte Playerownership führt
   den Scope ausdrücklich; mehrere vorhandene Descriptoren erlauben keine
   stille Weltwahl. Eigener versionierter Layout-/Modevertrag, keine nebenbei
   hinzugefügte Format5-Tabelle. Bestehende Daten verweigern mit expliziter
   Migrationanforderung; nötige Scope-Migration wird getrennt spezifiziert.
7. Explizite positive Request-/Payload-/Batch-/Receipt-/Scanlimits vor
   Allocation oder Write. Wiederverwenden der vorhandenen bounded Record-
   grammatik und vollständigen Generationidentityprüfung in derselben
   Mutationsgrenze. Neue API veröffentlicht keine DB-/Mutex-/Transactionhandles.
8. Konkrete Error- und Outcomevarianten unterscheiden unsupported mode,
   malformed data, identity mismatch, CAS conflict, Tombstone-/Freshnessfehler,
   Receiptcollision, Capacity, Revision/Epoch exhaustion, Backendfehler und
   ungewisses Commit. Keine Normalisierung oder ersetzenden leeren Inventare.

## Begrenzung

Container-/Overlay-/Hoppertransfers, Itemmerge, mehrere Player/Entities,
Crossdimension-/Crossshardtransfer, Javaeventbridge und App-/Simaktivierung
folgen separat. Der erste Producer darf sie nicht durch unfertige allgemeine
APIs vorwegnehmen. Kein automatisches Upgrade einer Welt und kein Claim von
Itemconservation allein aus opaquen Storagebytes.

## Konkreter Entwurf vor Umsetzung

Der Autor muss die vorhandenen Writer/Backends vollständig inventarisieren und
ein umsetzbares Proposal mit exakten Methoden/privaten Typen, Tablecatalog/
Key-/Value-/Receiptgrammatiken, Read-/CAS-/Commit-/Retryordnung, Default-
Unsupportedverhalten, Modeadmission, Migrationgrenzen, Byte-/Cachelimits,
Tombstone-/Receiptretention und Event-/Consumerseams liefern. Dazu konkrete
Testfälle und Dateiliste. Root prüft diesen vollständigen Entwurf und gibt
ihn innerhalb der bestehenden Nutzerautorisierung frei; keine erneute
Betreibergenehmigung für Routineimplementierungsentscheidungen.

## Abnahme und Invarianten

- Keine Teilwrites bei Konflikt/Returned-error; keine fehlende Tombstone-
  History; identischer Retry vor/nach Lostreply und Reopen, kollidierende ID,
  späterer CAS, zwei Pickups und Delete/Recreate mit alten Expectations.
- Reale Producer-Cutpoints vor/nach jedem Tablewrite und tatsächlichem
  Commit; vollständige Rawrelations-/Revision-/Receiptprüfung. Ungewisses
  Commit als solches berichten. Prozesskill-/Powerlossabnahme separat benennen.
- Beide Backends gleich; vollständiger Catalog-/Parser-/Malformed-/Boundary-
  Scan einschließlich maximaller Payloads, unknown schemas, fremden Scopes,
  fehlenden Descriptors, Revision/EpochMAX und allen blindwriterrefusals.
- Meaningful independent model/probes statt ausschließlich
  Encoder↔Decoder-Selbsterwartungen. Semantische Itembilanz gehört zusätzlich
  zum nachfolgenden Simconsumer mit ITEMS-TRANSFER-001.
- Vollständige Storage-Regression Debug/Release, strict all-target Clippy
  `-D warnings`, owned fmt und Rustdoc. Exact SHA256-Freeze und unabhängiges
  Fremdreview vor Rootintegration.

Nach CLEAN folgen DUR-04s Reservations-/Pending-/Ackconsumer und DUR-05s
produktiver Driver: sichtbare Pickup-/Take-/Removeoutputs erst nach bestätigtem
Commit; Disconnect/Unload/Shutdown behalten die Pendingauthority bis Klärung.
Dieses Ticket schließt die verbleibenden DUR-03-Erweiterungen nicht ab.
