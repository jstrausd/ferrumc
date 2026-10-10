# WG-002 / GEN-11.1 — Dauerhafte Generatoridentität

Stand: 10.10.2026. **Status: lokal implementiert/geprüft, unabhängiges Review CLEAN; integriert mit `dd910fd4`.**
Produktions-Crate: `ferrumc-storage`. WG-001 liefert den numerischen Seed, die
geschlossene `GenerationIdentity` und deren festen 27-Byte-Codec. WG-002 speichert
diesen Vertrag dauerhaft und atomar im ausdrücklich gewählten Metadatamodus.
Der produktive Format-3-Pfad bleibt aktiv. Migration, App-/Sim-Wiring und
natürliche Weltgenerierung sind weiterhin nicht umgesetzt.

## Ausgangslage und Ziel

`RedbStore` verwendet heute Datenbankformat 3. `ferrumc:meta` speichert
`&str -> u64`; der vorhandene Tabellenkatalog wird strikt geprüft. Eine neue
Generationstabelle benötigt deshalb ein ausdrücklich anderes Datenbankformat.
`WorldStore` besitzt bisher keinen Generator-Metadatenvertrag. `InMemoryStore`
verwendet separate Maplocks für die Datenkategorien; eine Leerheitsprüfung über
diese Kategorien wäre ohne zusätzliche gemeinsame Mutationsgrenze racy.

Eine vorhandene Overlaybasis darf niemals durch eine neue Konfiguration
stillschweigend anders generiert werden. Fehlende Metadaten sind bei vorhandenem
Weltzustand kein Nachweis einer frischen Welt. Aus Blockinhalt, Dateialter oder
einer Defaultkonfiguration darf kein historischer Seed geraten werden.

## Öffentliche API

Implementierte Form; öffentliche Felder bleiben privat und öffentliche Items
erhalten Rustdoc:

```rust
pub struct GenerationKey {
    // private WorldId und DimensionId
}

pub enum GenerationInitialization {
    Initialized,
    AlreadyPresent,
}

// Neue WorldStore-Methoden mit default Unsupported:
async fn load_generation_identity(
    &self,
    key: GenerationKey,
) -> Result<Option<GenerationIdentity>>;

async fn ensure_generation_identity(
    &self,
    key: GenerationKey,
    requested: GenerationIdentity,
) -> Result<GenerationInitialization>;
```

`GenerationKey` erhält einen Constructor sowie Getter für die typisierten
Welt-/Dimensionshandles. Eine Dimensionsart in `GenerationIdentity` ersetzt
diese Handles nicht. Andere `WorldStore`-Implementierungen und der alte
Speicherpfad liefern bei den neuen Methoden `Unsupported`; fehlende
Unterstützung darf nicht durch `Ok(None)` als Storage-Miss erscheinen.

## Stufenweise Aktivierung und Speicherlayout

- Neue Tabelle `ferrumc:generation`: acht Schlüsselbytes (`WorldId u32` und
  `DimensionId u32`, big-endian) auf genau 27 Wertbytes aus WG-001.
- Keine ungewöhnliche Aufteilung in mehrere `u64`-Metadatenschlüssel und keine
  Verwendung eines Plugin-Namespaces für Weltmetadaten.
- Höchstens 4096 Identitäten pro Store. Ein weiterer neuer Schlüssel scheitert
  vor Mutation; Compare eines vorhandenen Schlüssels bleibt möglich.
- Das neue Tabellenlayout heißt **Datenbankformat 4**. Datenbankformat,
  `FCGI`-Envelope-Schema und Generatorrevision bleiben voneinander unabhängig.
  Bestehende Chunk-/Overlayrecordbytes benötigen durch die neue Tabelle allein
  keine Umdeutung oder Normalisierung.
- `RedbStore::open` bleibt zunächst der vorhandene Format-3-Pfad einschließlich
  seines bisherigen Tabellenkatalogs. Bestehende produktive App-Aufrufer werden
  durch WG-002 nicht auf Format 4 umgestellt.
- Eine neue explizite Methode `RedbStore::open_with_generation_metadata`
  erstellt oder öffnet ausschließlich Format 4 mit neun Tabellen. Format 3
  liefert einen klassifizierten `GenerationMigrationRequired`-Fehler; die
  Methode migriert nicht und trägt keinen Descriptor nach.
- `InMemoryStore::new/default` behält das bisherige Verhalten. Ein ausdrücklich
  gewählter neuer Metadatamodus bildet denselben Admission-/Identitätsvertrag
  wie Format 4 ab.
- Der alte Format-3-Pfad verweigert Format 4. So kann ein älterer Startpfad
  unbekannte Generator-Metadaten nicht übergehen.

## Initialisierung, Freshness und fehlende Metadaten

`ensure_generation_identity` besitzt genau diese Entscheidungen:

| Vorhandener Zustand | Ergebnis |
| --- | --- |
| Gültige Identität ist vollständig gleich | `AlreadyPresent`; keine Änderung. |
| Gültige Identität weicht ab | Klassifizierter Mismatch; keine Änderung. |
| Identität ist malformed oder unsupported | Klassifizierter Decodefehler; keine Änderung und kein Fallback. |
| Identität fehlt und der gesamte Weltzustand ist leer | Atomare erstmalige Initialisierung. |
| Identität fehlt und Weltzustand existiert | Ausdrückliche Ablehnung; keine automatische Legacybindung. |

Die zunächst **globale** Leerheitsprüfung umfasst Chunks, Overlays, Entities,
Spieler, Journal, Journalreceipts und den Journalwatermark. Auch ein leeres gespeichertes Overlay
oder ein Receipt eines leeren Batches ist vorhandener Weltzustand. Plugin-KV-
Daten allein belegen dagegen keine Terrainbasis.

Diese konservative Grenze ist erforderlich: Spielerrecords und Journal tragen
heute keine belastbare Welt-/Dimensionszuordnung. Daraus darf keine freie
Zieldimension abgeleitet werden. Mehrere Identitäten können vor dem ersten
Gameplayzustand initialisiert werden; zusätzliche Namespaces nach Beginn des
Spielbetriebs benötigen später einen eigenen Lifecycle-/Scopevertrag.

Eine leere Datenbank darf einen ausdrücklich angeforderten Seed erhalten.
Vorhandene Flat-Blöcke, importierte Chunks oder Overlays rechtfertigen keine
automatische Bindung an Flat-v1/Seed 0. Der numerische WG-001-Default 0 ist kein
gespeicherter historischer Seed und keine Storage-Migration.

## Save-Admission und atomare Konkurrenz

Im ausdrücklich gewählten Format-4-/InMemory-Metadatamodus gilt:

- Chunk-, Overlay- und Entity-Saves verlangen eine bestätigte Identität für
  ihren Welt-/Dimensionsscope, bevor Daten geschrieben werden.
- Journal- und Player-Saves verlangen mindestens eine bestätigte Identität im
  Store. Das schützt vor komplett ungebundenen Writes, ersetzt aber keine
  fehlende Welt-/Dimensionszuordnung dieser Records. Diese verbleibende Grenze
  muss in API-Dokumentation und Abnahme sichtbar bleiben.
- Identitäten können weder überschrieben noch gelöscht werden.
- Vollständige Save-Batches werden vor dem ersten Commit geprüft. Ein später
  Fehler lässt frühere Einträge desselben Batches unverändert.

Bei redb laufen Metadatenprüfung, Leerheitsprüfung und erstmaliger Insert in
**derselben Write-Transaction**. Erst der erfolgreiche Commit erlaubt die
Antwort `Initialized`. Save-Admission erfolgt ebenfalls unter der jeweiligen
Write-Transaction; konkurrierende Writer können die Prüfung nicht zwischen
Check und Insert passieren. Alle Datenbankarbeit bleibt in `spawn_blocking`.

InMemory erhält eine storeeigene `RwLock<()>`-Mutationsgrenze. Initialisierung
nimmt exklusiven Zugriff; betroffene Save-/Delete-/Journal-/Player-Mutationen
nehmen gemeinsamen Zugriff vor den vorhandenen Maplocks. Ein Guard darf keinen
`.await` überschreiten. Die Grenze ist Storagezustand innerhalb einer Instanz,
kein globaler Simulationslock. Auch Deletes müssen teilnehmen, damit die
Freshnessentscheidung einen konsistenten Zustand sieht.

## Klassifizierte Fehler

Vorgesehene unterscheidbare Ursachen:

- Metadatenvertrag auf dem alten Backendpfad nicht unterstützt.
- Explizite Migration von Format 3 erforderlich.
- Fehlende Identität bei vorhandenem Weltzustand oder abgewiesener Save.
- Inkompatible angeforderte und gespeicherte Identität, einschließlich Seed.
- Malformed Generation-Schlüssel oder fehlerhafter/unsupported WG-001-Wert.
- Metadatenobergrenze erreicht.
- Operativer Backend-/Lock-/Commitfehler.

`StorageError` behält die konkrete Ursache und den betroffenen Schlüssel.
Konvertierung in den bestehenden `ServerError`-Vertrag darf Ablehnungen nicht
als erfolgreiche Misses verschleiern. Unknown-/Malformed-Fälle werden vor
Einsetzen einer neuen Basis beendet.

## One-Crate-Dateibesitz der Umsetzung

| Datei | Aufgabe |
| --- | --- |
| `server/crates/ferrumc-storage/src/generation.rs` — neu | Schlüssel, Ergebnis und gemeinsame Prüfhelfer. |
| `server/crates/ferrumc-storage/src/store.rs` | Default-Unsupported-Methoden und öffentlicher Vertrag. |
| `server/crates/ferrumc-storage/src/error.rs` | Klassifizierte Fehler und Konvertierung. |
| `server/crates/ferrumc-storage/src/lib.rs` | Öffentliche Exports. |
| `server/crates/ferrumc-storage/src/redb_store.rs` | Explizites Format 4, neue Tabelle, atomare Operationen und Save-Admission. |
| `server/crates/ferrumc-storage/src/memory.rs` | Expliziter Metadatamodus und gemeinsame Mutationsgrenze. |
| `server/crates/ferrumc-storage/src/redb_store/generation_tests.rs` — neu | Raw-DB-, Cutpoint- und Backendtests. |
| `server/crates/ferrumc-storage/src/memory/generation_tests.rs` — neu | Lock-/Admission-/Parallelitätsfälle. |
| `server/crates/ferrumc-storage/tests/generation_metadata.rs` — neu | Öffentlicher Backendvertrag und Parität. |
| `server/crates/ferrumc-storage/INVARIANTS.md` | Neue, ausdrücklich opt-in geltende Regeln. |
| `server/crates/ferrumc-storage/README.md` | API, Aktivierungsgrenzen und Migration. |

Diese Tasknotiz darf den Umsetzungsbefund aufnehmen. Cargo-/Dependency- und
Fremdcrate-Änderungen sind für diesen Umfang nicht vorgesehen. Board und
Featurekatalog verwaltet der Orchestrator.

## Implementierte Abnahmetests

1. Fresh initialize, dauerhaft speichern, schließen, Format-4-Wiederöffnung und
   identischen Compare prüfen. Numerische Seedgrenzen aus WG-001 erhalten.
2. Format-3-Öffnung und bestehende Vertragsfälle unverändert erhalten. Der neue
   Pfad verweigert Format 3 ohne Änderung von Header, Katalog oder logischen
   Daten; der alte Pfad verweigert Format 4.
3. Fehlende/fremde Tabellen, falsche Tabletypen, malformed acht-Byte-Schlüssel,
   truncated/überlange 27-Byte-Werte und alle WG-001-Unsupported-Fälle prüfen.
4. Mismatch überschreibt niemals die gespeicherte Identität. Ein Missing-Record
   mit vorhandenem Zustand erhält keinen Seed-0-Default.
5. Jeden welttragenden Datentyp separat für Missing/Freshness prüfen: Chunk,
   leeres/nichtleeres Overlay, Entity, Player, Journal und leeren Batchreceipt.
   Plugin-KV allein darf den klar definierten Freshfall nicht blockieren.
6. Obergrenze 4096; neuer Insert scheitert atomar, vorhandener Compare bleibt
   möglich. Keine inputabhängige unbeschränkte Metadatenallokation.
7. Parallel gleiche Initialisierung: genau einmal `Initialized`, übrige Aufrufe
   `AlreadyPresent`. Parallel verschiedene Seeds: genau ein erfolgreicher
   Gewinner; anderer Mismatch. Keine Sleeps als Synchronisation.
8. Deterministische Konkurrenz mit Saves: eine Mutation passiert die gemeinsame
   Grenze nicht zwischen Freshnesscheck und Insert; ungebundene Saves im neuen
   Modus scheitern. Ganze Batches bleiben bei spätem Admissionfehler unverändert.
9. Cutpoint nach Insert/vor Commit über denselben privaten Transaktionshelper
   wie die Produktion: Abort/Drop hinterlässt keine Identität. Daten, Journal,
   Header und Katalog bleiben logisch unverändert.
10. Cutpoint nach Commit/vor Antwort: bestätigter Datensatz bleibt bestehen;
    erneute gleiche Anfrage liefert `AlreadyPresent`, keinen zweiten Insert.
    Eine andere Anfrage bleibt Mismatch.
11. Nach fehlgeschlagenem Compare/Decode/Freshnesscheck bleibt der vollständige
    logische Snapshot unverändert; späterer gültiger Read/Compare funktioniert.

Diese Cutpoints prüfen Transaktions-/Retryverhalten. Sie sind kein Nachweis
gegen Prozessabbruch oder Stromausfall; solche externen Recoverytests folgen
getrennt. Die Abnahme liegt in `tests/generation_metadata.rs` sowie den privaten
`memory/generation_tests.rs` und `redb_store/generation_tests.rs`. Parallelitäts-
prüfungen verwenden Barrieren ohne Sleeps. Die Save-/Init-Grenztests halten
den tatsächlichen Memory-Mutationsguard beziehungsweise eine redb-Write-
Transaction und modellieren einen bereits zugelassenen In-flight-Save vor der
wartenden Initialisierung; sie behaupten keine vollständige Schedule-Exploration.

## Separates Ticket WG-002-MIG — Explizite Offline-Migration

Vor produktivem Format-4-Wiring muss eine getrennte, reviewbare Migration von
Format 3 nach Format 4 fertig sein:

- Vorhandenen Format-3-Katalog und alle relevanten Daten prüfen. Ein laufender
  Server darf nicht parallel migriert werden.
- Eine ausdrücklich erklärte Legacybasis und zugehörige Welt-/Dimensionskeys
  verwenden. Keine Ableitung aus Blockinhalt und kein geratener historischer
  Seed. Der bisherige FerrumC-Flat-v1-Fall braucht einen belegten Vertrag;
  eine Zuordnung zu numerisch 0 wird als explizite Legacyzuordnung ausgewiesen.
- Mehrdeutige Scopezuordnung aufgrund Journal/Spielerdaten ausdrücklich
  ablehnen. Keine scheinbare Vollständigkeit aus allein vorhandenen Chunkkeys.
- Vorbereitung/Report und Backup-/Rollbackvertrag vor der Migration liefern.
  Migration erfolgt ausdrücklich, nicht automatisch beim normalen Start.
- Neue Tabelle, sämtliche geprüften Descriptoren und Formatmarker atomar
  zusammen schreiben. Vorhandene Recordbytes und Journalreihenfolge erhalten;
  nicht durch Decode/Encode still normalisieren.
- Cutpoints vor/nach Marker und vor Commit dürfen ausschließlich vollständiges
  Format 3 oder vollständiges Format 4 hinterlassen. Unvollständiger Katalog
  oder fehlende Descriptoren werden nicht nachgetragen.

WG-002 selbst bietet diese Migration nicht an. Es darf deshalb keinen
produktiven App-Aufrufer auf Format 4 umstellen.

## Gepinnte Originalreferenz

Paper Java Edition **1.21.8 Build 60**, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Gelesen wurde die vollständige
Vanilla-Basis mit angewandten Source-/Feature-Patches unter
`.codex-tmp/paper-1.21.8-reference/paper-server/src/minecraft/java/`.
Siehe [PAPER_REFERENCE.md](../PAPER_REFERENCE.md) und
[WG-001](WG-001_GENERATION_CONTRACT.md).

| Vollständige Quelle unter `net/minecraft/` | Befund für den Vertrag |
| --- | --- |
| `world/level/levelgen/WorldOptions.java` | Numerischer Seed und Weltoptionen sind ausdrücklicher Zustand; ein Flat-Default beweist keinen historischen Seed. |
| `world/level/levelgen/WorldDimensions.java` | Dimension-Stems verbinden Dimensiontyp und Generator. Missing Overworld scheitert ausdrücklich. |
| `world/level/storage/PrimaryLevelData.java`, `createTag` / `setTagData` | Speichert Version/DataVersion sowie Weltoptionen und Dimensiongeneratoren als `WorldGenSettings`; Metadaten stehen neben weiterem Weltzustand. |
| `world/level/storage/LevelStorageSource.java`, `getLevelDataAndDimensions` | Rekonstruiert gespeicherte `WorldGenSettings`, Dimensionen und Optionen beim Laden. |
| `world/level/storage/LevelStorageSource.java`, `saveDataTag` / `saveLevelData` | Metadatensave schreibt eine temporäre Datei und verwendet sicheren Austausch mit alter Datei. |

FerrumCs redb-Tabelle und Transaktionsvertrag sind eigenständig. Diese
Quellenprüfung behauptet weder Vanilla-NBT-Formatparität noch bestandene
Differenzial- oder Runtimeabnahme gegen Paper. In WG-002 werden keine
Plugin-Worldgen-Events oder neuen Terrainalgorithmen implementiert.

## Nötige Folgetickets und Grenzen

1. **WG-002-MIG:** explizite geprüfte Format-3-Migration vor produktiver
   Aktivierung, wie oben beschrieben.
2. **WG-003 / `ferrumc-sim`:** Descriptor vor Regeneration und Overlayanwendung
   verlangen; `loaded.rs::load_or_generate` und LoadedChunkMap auf
   `ChunkGenerator` umstellen. Ein Metadatenfehler darf keine neue Basis erzeugen.
3. **WG-004 / `ferrumc-app`:** erst nach Migration den produktiven Öffnungspfad
   wechseln. Descriptor vor Anvilimport, Spawn-Acquire und ersten Saves bestätigen.
   Gespeicherte Identität steuert die Generatorwahl; Konfigurationsabweichungen
   scheitern. Text-/Zufallsseed-Auswahl getrennt prüfen.
4. **Weitere DUR-Tickets:** stabile Zuordnung von Welt-/Dimensionshandles,
   scopbare Journal-/Playerrecords, dynamischer Namespace-Lifecycle und breitere
   Recoveryabnahme schließen. Die aktuelle globale Freshnessprüfung wird erst
   mit diesen belastbaren Verträgen verfeinert.
5. **Natürliche Generierung:** explizite neue Generatoridentitäten und Terrain
   folgen nach Persistenz und Consumeraktivierung über GEN-02 und weitere
   Generatorfeaturetickets.

## Umsetzungsbefund und Nachweis

- Neues öffentliches `GenerationKey` (private Felder, typisierte Getter),
  `GenerationInitialization` und Limit `MAX_GENERATION_IDENTITIES = 4096`.
- Traitdefaults und Legacyinstanzen liefern `GenerationMetadataUnsupported`.
  Neuer InMemory-Constructor: `with_generation_metadata`; neuer redb-Opener:
  `open_with_generation_metadata`. Der alte Opener ist weiterhin Format 3.
- Admission umfasst `save_chunk`, `save_chunks`, `save_chunk_overlays`,
  `save_entity`, `save_entities`, `save_player`, `append_block_mutations` und
  `append_block_mutation_batch`, einschließlich leerer Journalanfragen/Receipts.
  Memory koordiniert auch `delete_chunk`, `delete_entity`, `delete_player`.
  redb Deletes bleiben durch die gemeinsamen Write-Transactions serialisiert.
- Generationstabelle prüft Typ/Katalog, exakt acht Schlüsselbytes und exakt
  27 Wertbytes, Decoderfehler jeder WG-001-Familie und den Descriptorcap. Kein
  Record wird beim Öffnen normalisiert oder zur Rekonstruktion eines Seeds benutzt.
- Fehlende Identität/Mismatch/Migration -> `ServerError::InvalidState`;
  Metadatencap -> `Capacity`; alter Modus -> `Unsupported`; malformed Key und
  Decoderfehler -> `Internal` mit genauer `StorageError`-Ursache/Key im Diagnostic.
  Falscher Versionmarker/Katalog/Tabellentyp behält die vorhandene
  `IncompatiblePreAlphaData`-/`InvalidState`-Klassifikation. Operative Fehler
  bleiben Backendfehler. Der vorhandene `ServerError`-Vertrag transportiert
  Diagnostik, keinen neuen typisierten Storage-Source-Downcast.
- RED: der zuerst hinzugefügte öffentliche Test konnte ohne neue API nicht
  kompilieren. Dies ist Compile-RED, kein behaupteter Laufzeit-Verhaltens-RED.
- `/home/jstrauss/.cargo/bin/cargo test -p ferrumc-storage`: **146 grün**
  (116 Unit, 7 Metadatenintegration, 13 redb-Roundtrip, 9 Memory-Roundtrip,
  1 Rustdoc; keine ignorierten Tests).
- `/home/jstrauss/.cargo/bin/cargo clippy -p ferrumc-storage --all-targets -- -D warnings`:
  bestanden. Owned-file-Rustfmt mit Edition 2021 und `skip_children=true`:
  bestanden. Keine Dependencies/Cargo-, Fremdcrate- oder generierten Änderungen.

Diese Umsetzung aktiviert keine neue produktive Weltbasis. Separate Migration
und Verbraucheraktivierung bleiben zwingende Folgetickets. Source-Referenzprüfung
und Transaktions-/Retrytests sind kein Vanilla-/Paper-Runtimeabnahmebeleg.
Die ursprüngliche Schätzung für WG-002 allein war 3–5 Personentage einschließlich
Review; Migration und Sim-/App-Wiring werden weiterhin separat geschätzt.
