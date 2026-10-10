# SIM-03.2 — Begrenzte shardbesessene Entity-Verwaltung

Status: an `feature_catalog` vergeben, API abgestimmt; Produktionsbeginn nach
separatem REG-ENTITY001-Registryproducer. Der bestehende Registryexport enthält
noch keinen EntityTypeId oder Entitytypenlookup. Agent implementiert zuerst
diesen Producer in einem eigenen Ticket und einer eigenen Crate.
Core-UUID ist unabhängig geprüft: sechs Default-/vier No-default-Contracttests
im Rootlauf grün, Freeze7/7 bestätigt und Review CLEAN.
Produktions-Crate ausschließlich `ferrumc-sim`.

## Ziel und Besitz

Auf dem getrennten `EntityUuid`-Producer aus SIM-03.1 entsteht eine echte
besessene Entityverwaltung für den späteren Item-/Mobconsumer. Entityuuid,
Wire-ID und lokaler Handle erfüllen unterschiedliche Aufgaben. Kein Socket,
kein Raw-DB-Handle und keine globale Entitymap werden eingeführt.

Besitz bei Vergabe: `server/crates/ferrumc-sim/**` und dieses Dokument.
WG-003 ist als `da61c638` integriert; seine öffentlichen APIs und Tests bleiben
kompatibel. Keine Core-/Storage-/World-/Registry-/App-/Lock-/Gitänderung.
Root besitzt Board/Katalog. Vor Umsetzung AGENTS/CLAUDE, Sim-INVARIANTS,
Ownership-/Architektur-/ADR- und vollständige Referenzquellen lesen.

## Geplanter begrenzter Vertrag

- Owned bounded `EntityStore`, Entityuuid- und Wire-ID-Index sowie typisierter
  Chunk-/Spatialindex. Kein linearer Scan sämtlicher Entities pro Chunkquery.
  Initialisierung, zulässige Kapazität und voller Store haben konkrete,
  dokumentierte Fehler-/Backpressureverträge; keine Default-unbounded Maps.
- Registrierter Entitytyp und begrenzte endliche Basisposition/Transformation
  werden validiert. Nur Basisdaten und sichere Spawn/Remove/Move/Lookup-
  Operationen; keine vorgetäuschte KI, vollständige Metadata, Kollision,
  Living-/Itemkomponenten oder automatische Zufalls-/Wire-ID-Erzeugung.
- Ein opaquer generationaler lokaler Handle verhindert stale Zugriff nach
  Remove/Slotreuse. Overflow darf keinen alten Handle wieder gültig machen.
  Umgang mit fremdem Store/Owner und neuer Store-Inkarnation explizit entwerfen;
  Handles nicht als permanente UUID oder Wire-ID serialisieren.
- Alle Mutationen eines Owners synchron an dessen Tickgrenze. Fehlende,
  doppelte, malformed oder über der Kapazität liegende Spawns ändern keinen
  Index und keine schon vorhandene Entity. Move/Remove/Chunkquery bewahren
  sämtliche Indexkonsistenz einschließlich negativer Chunk-/Shardgrenzen.
- Store/Resident-Lifetime, scope-/shardgebundene Aufnahme und tatsächlicher
  Chunkload-/Tickstatus sind getrennt. Noch fehlende Cross-shard-Transfers,
  Persistenz, Tracking und global prozessweite Wire-ID-Allocation benennen.
  Keine Doppelinstanz als vollständige globale Duplicateuuidprüfung ausgeben.
- Konsistente SimShard-Anbindung mit eigenen Accessoren/Admissionmethoden
  und klarer Tickgrenze, soweit ohne fehlenden Storage-/Sessionconsumer möglich.
  Bereits vorhandene Spieleridentität/-zustände nicht in einen zweiten
  konkurrierenden Owner kopieren. Fehlende aktive Verbraucher bleiben offen.
- Keine Allocation beim erfolgreichen Lookup/Transformupdate; Spawn kann
  bounded lazy Growth nutzen. Queryergebnisse bevorzugt borrowed Iteratoren
  oder ausdrücklich bounded Paginierung, kein unbegrenzter materialisierter Vec.

Das konkrete API-/Handle-/Scope-/Kapazitätsdesign vor Produktionsänderungen
an Root melden. Falls fehlender Producer die korrekte Einbindung verhindert,
präzisen fehlenden Vertrag melden und das Ticket vor dieser Grenze teilen.

## Originalreferenz und Abnahme

MC1.21.8 / Protokoll772, Paper Build60 / Commit
`29c8822d90899c89d2689338e81a98f690bcba12`.
Vollständige relevante Entity-ID/UUID-/Removal-Methoden und Callees,
`EntityLookup`, `EntitySection`, `EntitySectionStorage`,
`PersistentEntitySectionManager` beziehungsweise die tatsächlich aktiven
Moonrise-Entitylookup-/Chunkconsumer lesen. Relevante Source-/Featurepatches
für Duplikate, Visibility, Tick/Tracking, Unload und Callbacks mitlesen.
Vanilla-/Paperverhalten, zusätzliche FerrumCcaps und fehlende Event-/
Trackingconsumer im Taskdoc trennen; Sourcelektüre ist kein Livevergleich.

- Öffentliche Tests für mehrere registrierte Entitytypen, UUID-/Wireduplikate,
  Scope-/Shard-/Positiongrenzen, volle Kapazität und unveränderte Fehlerzustände.
- Spawn/Move/Remove/Reuse mit konsistenten UUID-/Wire-/Chunkindizes;
  deterministische Interleavings, mehrere Entities im Chunk, negative Grenzen,
  Pagination-/Budget- und leere Queries; stale/fremde Handles verweigern.
- Private Slotgeneration-Max-/Retirementfälle, keine Wraparoundaliasierung.
  Snapshot-/Cloneverhalten ausdrücklich prüfen, falls es angeboten wird.
- Unterschied zwischen geladenem, tickendem, ausgeladenem und entfernten
  Zustand dokumentieren; kein Dauerhaftigkeits-Ack ohne noch fehlenden Store.
- Vorhandene Simtests inklusive WG-003; strict all-targets Clippy und fmt.
  Source-/Behavior-/Event-/Ownership-/Deviation-/Validationmatrix abschließen.
- Exaktes SHA256-Freeze aller geänderten Dateien und Logs an Root;
  keine Gitoperationen. Root organisiert Fremdreview und Integration.

Item-Drop/Pickup, atomare Ownershiptransfers, Entity-Storage/Restart,
Bewegungsphysik, Trackingpakete, Mobs/KI, Nether/End und Dragonphasen bleiben
anschließende Consumers. Dieses Ticket allein schließt keine Survival-Etappe.
