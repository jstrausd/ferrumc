# WG-001 — Versionsfester Generatorvertrag

Stand: 10.10.2026. Produktions-Crate: `ferrumc-world`. Dieser Schritt liefert
einen numerischen Seed-/Identitätsvertrag und einen reinen Generatorpfad;
natürliche Weltgenerierung und dauerhaft gespeicherte Weltidentität sind noch
nicht aktiviert.

## Implementierter Umfang

- `WorldSeed` erhält jeden `i64`-Wert unverändert. Zufallsauswahl und Textseed-
  Interpretation liegen außerhalb des reinen Weltmodells.
- Die geschlossene `GenerationIdentity` enthält Minecraft-Version,
  Generatorart, Generatorrevision, Seed und Dimensionsart. Ausführbar ist heute
  ausschließlich Minecraft 1.21.8 / FerrumC flat revision 1 / Overworld.
- Feste, allocationfreie Kodierung mit genau 27 Bytes. Das `FCGI`-Envelope hat
  Schema 1; die Generatorrevision ist ein davon getrenntes Feld. Mehrbytewerte
  sind big-endian, der Seed ist signed `i64`.
- Der Decoder prüft die vollständige Länge vor jedem Feldzugriff und lehnt
  fremde Magic, Schema, Minecraft-Version, Generatorart, Generatorrevision und
  Dimension mit klassifizierten Fehlern ab. Es gibt keinen stillen Fallback.
- `ensure_compatible` verlangt vollständige Identitätsgleichheit. Wegen des
  geschlossenen heutigen Vertrags können nur unterschiedliche Seeds zwei
  unterstützte, inkompatible Identitäten bilden. Andere Varianten scheitern
  bereits am Decoder; die Tests behaupten keine weiteren ausführbaren Generatoren.
- Objektfähiger synchroner `ChunkGenerator` mit `identity` und
  `generate_chunk(ChunkPos)`. Generierung ist unabhängig von Aufrufreihenfolge
  und vorausgegangenen Chunks; die Identität bleibt unverändert.
- `FlatWorldGenerator::generate` delegiert intern an diesen Produktionspfad.
  Die existierenden Aufrufer erhalten unveränderte Flat-Blöcke. Default wählt
  ausdrücklich Seed 0; `with_seed` ändert die Identität. Flat-v1 verwendet keine
  Zufallszahlen, weshalb verschiedene Seeds trotzdem gleiche Blöcke liefern.

## Gepinnte Originalreferenz

Paper Java Edition **1.21.8 Build 60**, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Die vollständige Vanilla-Basis mit
angewandten Source-/Feature-Patches wurde unter
`.codex-tmp/paper-1.21.8-reference/paper-server/src/minecraft/java/` gelesen.
Grundlage ist [PAPER_REFERENCE.md](../PAPER_REFERENCE.md); Quellenprüfung ist
kein Laufzeitvergleich gegen einen gestarteten Paper-Server.

| Vollständige Quelle unter `net/minecraft/` | Gelesenes Verhalten / Konsequenz |
| --- | --- |
| `world/level/levelgen/WorldOptions.java` | Numerischer Seed ist Java `long`; Struktur- und Bonuschestoptionen sind eigenständig. `parseSeed` trimmt einen Textwert, akzeptiert `long` oder verwendet Java-Stringhash; leerer Text liefert keine Auswahl. WG-001 implementiert nur den bereits ausgewählten numerischen Wert. |
| `server/dedicated/DedicatedServerProperties.java` | Konfiguration liest `level-seed`, wählt den numerischen Seed und setzt Weltoptionen vor Generierung. Zufall/Textauswertung gehört zum separaten Config/App-Schritt. |
| `world/level/levelgen/WorldDimensions.java` | Dimension-Stems binden Dimensiontyp und Generator; fehlende Overworld scheitert ausdrücklich. WG-001 bleibt auf die vorhandene Overworld-Geometrie begrenzt. |
| `world/level/levelgen/RandomSupport.java` | Seedmischung/-erweiterung und Hashableitung haben ausdrückliche Algorithmen. WG-001 implementiert keine Noise-/RNG-Algorithmen und behauptet keine Vanilla-Seed-Parität. |

Der `FCGI`-Codec ist ein eigener FerrumC-Metadatenvertrag. Er kopiert weder
Vanilla-NBT noch Paper-Implementierungscode. Flat-v1 bleibt FerrumCs bisheriger
Schichtgenerator, kein Vanilla-Flat-Profil. Es entstehen in diesem Schritt keine
neuen Plugin-/Worldgen-Events; die reine Generierung berührt keine Sockets,
Storagehandles oder Tick-Reihenfolge.

## Tests und Referenzfixture

`server/crates/ferrumc-world/tests/fixtures/generation_identity_v1.hex` ist eine
unabhängig deklarierte Goldenfolge mit nichtuniformem negativem Seed
`-0x0102030405060708`, keine Ausgabe des Rust-Encoders. Der Test pinnt zusätzlich
dieselbe Grammatik mit einer expliziten Bytefolge. Eigene Fälle prüfen die
signed-i64-Grenzen samt erwarteten Bytes, alle Truncations, trailing und
überlange Daten sowie jeden nicht unterstützten Metadatentyp. Schemafehler und
Generatorrevisionsfehler haben getrennte Abnahmen.

Deterministische Tests vergleichen konkrete und `dyn ChunkGenerator`-Aufrufe
mit wiederholten und umgekehrt angeforderten positiven/negativen Chunkpositionen
und verschiedenen Seeds. Ein unabhängiger, vollständig enumerierter Flat-
Profiltest prüft sämtliche 98.304 Blockpositionen sowie alle Dirty-Masken.
Generation setzt ausschließlich Netzwerk-dirty 0..7; Gameplay-persist-dirty
und kumulative persist-edited bleiben leer.

Diese Tests prüfen FerrumCs Metadatenvertrag und die unveränderte Flatbasis.
Sie sind weder echte Clientabnahme noch Terrainvergleich mit Paper.

## Konkrete Verbraucher und Folgetickets

1. **WG-002, `ferrumc-storage`:** Welt-/Dimensionsmetadaten dauerhaft einmalig
   speichern; atomare Erstinitialisierung und klare Mismatch-/Unknown-Abweisung.
   Legacy-Welten benötigen einen expliziten, geprüften Vertrag. Seed 0 ist hier
   nur der Flat-Default, kein Beweis einer Storage-Migration.
2. **WG-003, `ferrumc-sim`:** `loaded.rs::load_or_generate` und
   `LoadedChunkMap::acquire/acquire_spawn` vom konkreten Flattyp auf den
   Generatorvertrag umstellen. Metadaten prüfen, bevor regenerierte Basis und
   gespeicherte Overlays kombiniert werden.
3. **WG-004, `ferrumc-app`:** `world.rs::build_world`, `WorldSetup::generator` und
   Driver aus gespeicherter Identität verdrahten; bestehende Welt darf durch
   einen Konfigurationswechsel nicht neu geseedet werden. Text-/Zufallsseed-
   Auswahl und Fehlermeldungen separat testen.
4. **Natürliche Terrainstufe:** explizite neue Generatoridentität sowie Biome,
   Terrain, Höhlen, Ressourcen und Features erst nach diesen Aktivierungen
   einführen. Dieses Ticket erzeugt weiterhin ausschließlich feste Schichten.

Der heutige App-/Sim-Aufruf nutzt über `FlatWorldGenerator::generate` bereits
den gemeinsamen Generatorcode. Er liest oder prüft noch keine gespeicherte
Identität. Restart-Sicherheit, Dimensionswechsel und konfigurierbare Terrain-
Auswahl sind dadurch nicht nachgewiesen.
