# REG-ENTITY001 — Geprüfte originale Entitytypen

Status: Producer implementiert und lokal geprüft; Freeze für Rootreview.
Temporärer Besitz ausschließlich `server/crates/ferrumc-registry/**` und dieses
Dokument. Keine Sim-/Core-/World-/App-/Fixture-/Proto-/Lock-/Gitänderungen.
SIM-03.2 beginnt erst nach unabhängigem Review und Integration dieses Producers.

## Ziel und API

Der Sim-Entitystore braucht eine validierte Typidentität. Vorher exportierte
Registry nur Item-/Block-/Biome-/Dimensiondaten; frei erfundene Entitywiretypen
würden die Admissionprüfung umgehen. Dieses Ticket stellt ausschließlich den
vollständigen statischen Entitytypenproducer für **Minecraft Java 1.21.8,
Protokoll 772, DataVersion 4440** bereit.

`entity_type::EntityTypeId` ist ein privater i32-Wrapper mit Copy/Eq/Ord/Hash.
`try_from_protocol_id`, `from_name`, `lookup_entity_type` und
`lookup_entity_type_name` validieren die Identität; unbekannte, negative und
malformed Werte liefern `None`. Exakte bare oder `minecraft:`-Namen sind
zulässig. Keine Groß-/Kleinschreibungs-, Whitespace- oder Pfadnormalisierung.
Keine Default-, ungeprüfte oder Deserialisierungskonstruktion.
`protocol_id`, `resource_location`, `is_player` liefern Daten aus denselben
generierten Tabellen. `entity_types` iteriert in Wire-ID-Reihenfolge mit
ExactSize/DoubleEnded/Fused-Vertrag und `.take`/`.skip`-Budgetierung;
`ENTITY_TYPE_COUNT` beträgt 151. Jeder Runtimezugriff ist allocationfrei und
liest ausschließlich statische Daten. Keine globale mutable Registryinstanz.

Der Typ bezeichnet eine Entityart. Er ersetzt weder `EntityUuid`, den
protokollseitigen Instanz-`EntityId`, einen generationalen Storagehandle noch
die bestehende Playerownership. `is_player` ermöglicht dem nachfolgenden
Simconsumer, konkurrierende Playeradmission zu verweigern.

## Daten und reproduzierbarer Generator

Das bestehende `build.rs` liest direkt die **unveränderte**
[CAT-02.1-Entityfixture](../fixtures/minecraft-1.21.8-original/entity-types.json),
ohne handgeschriebene zweite Liste oder Kopie des vollständigen Reports.
`build/entity_type_codegen.rs` prüft Schema und denselben 151-Entry-Vertrag:
eindeutige kanonische Namen, eindeutige/dichte IDs 0..150, Registryname und
Registry-ID 5, Default `minecraft:pig`, Player vorhanden. Doppelte JSONkeys
werden verweigert, statt durch Mapüberschreiben zu verschwinden. Metadaten und
der tatsächliche begrenzte Read verhindern Eingaben über 64 KiB. Fehler sind
Io/Json/Oversized/Invalid/Format und scheitern vor Emission der neuen Tabelle.
Player-ID und i32-Iteratorgrenze werden aus diesem Input generiert. Keine neue
Runtime-/Builddependency und keine Cargo.lock-Änderung.

[Exporthelper](../../server/crates/ferrumc-registry/tests/reference/export_entity_types.py)
und [Oraclemanifest](../../server/crates/ferrumc-registry/tests/fixtures/entity_types_1_21_8.manifest.json)
liegen innerhalb Registry. Der Helper liest und prüft unabhängige feste Pins;
er startet keinen Server, Java oder Download. Schreiben fordert den vollständigen
Report und prüft semantische Gleichheit des gesamten `minecraft:entity_type`-
Subsets mit der bereits eingecheckten Fixture, einschließlich Default und
Registry-ID. `require` bleibt unter Python `-O` aktiv. Der exportierte TSV
liefert alle 151 Namen, IDs und Playerklassifikation als öffentliches Testoracle.

| Quelle | Bytes / SHA-256 |
| --- | --- |
| Originalsubset | 9.319 / `b65a61ac9f4346ad74ff159cb3f8be78ec96229ccfc3abe29a14a7e0063d4d6b` |
| Vollständiger Originalreport | 466.543 / `177a6ad99d7b6d28f27e49231d15a5ccdb262bed9ac07f7e66dab5d32cc20c27` |
| Originalexportmanifest | 7.061 / `d5b4506f5749d114d7b54c50231ee00f6f15d14b2826f8c39489df06932ed9ee` |
| Oracle-TSV | 4.566 / `e56e7322b59e20ef74447fc0a7237f7784fe3455b308a2c4ebe7a1277bafa15d` |
| Oraclemanifest | 2.332 / `08540db5110ea7cd7ad13d2a9f5d53bbf1bfe87747bef09563194aa58a4d6dab` |

Manifest bindet außerdem unverändertes offizielles Versionmanifest,
Serverbundle SHA-256 `2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7`
und inneres Original-JAR SHA-256
`2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`.
Mojang-/Microsoft-Daten werden dadurch nicht unter Repository-MIT umgelizenziert;
Tooling bleibt MIT. Keine JAR-/Klassenkopien eingecheckt.

Aus Repositoryroot reproduzieren/prüfen:

```sh
python3 server/crates/ferrumc-registry/tests/reference/export_entity_types.py --self-test --check
python3 -O server/crates/ferrumc-registry/tests/reference/export_entity_types.py --self-test --check
python3 server/crates/ferrumc-registry/tests/reference/export_entity_types.py --export --report .codex-tmp/original-1.21.8/generated/reports/registries.json --check
```

CAT-02.1 besitzt den Originaldownload-/Datagenvertrag. Kein beschädigter oder
fehlender Report wird durch Netzwerk, Paper oder einen anderen Release ersetzt.

## Quellen-/Verhaltens-/Event-/Ownershipmatrix

Referenzlektüre: [Paper Build60 / Commitpin](../PAPER_REFERENCE.md)
`29c8822d90899c89d2689338e81a98f690bcba12`, vollständige erzeugte Quellen mit
Source-/Featurepatches. Oraclequelle ist das ungepatchte offizielle Original-JAR.

| Quelle/Symbol und Umfang | Verhalten / FerrumC-Vertrag | Events, Ownership, Grenze |
| --- | --- | --- |
| Vollständiger `RegistryDumpReport.run/dumpRegistry` | `BuiltInRegistries.REGISTRY` liefert Registry-ID; `registry.getId(holder.value())` liefert Entry-ID; Default separat. Vollständige Reportsubsetprüfung ausgeführt. | Datenexport; kein Entityspawn und kein Live-Paper-Vergleich. |
| Vollständige `EntityType.register`-Overloads, `vanillaEntityId`, `getKey`, `byString`; komplette `PLAYER`-Registrierung | Player ist getrennte registrierte Art, noSave/noSummon/createNothing in Original. FerrumC liefert nur deren Identität; keine Factoryflags aus einer ID ableiten. | Diese Getter/Registrierung erzeugen hier keine Playerinstanz; bestehende Playerownership bleibt beim Consumer. |
| `BuiltInRegistries.ENTITY_TYPE` samt vollständigem relevantem Sourcepatch | Default pig; Paper ergänzt APIregistry-Registrierung, Freeze-/Holderlisteners. Statische Original-151-Menge bleibt expliziter Pin. | Paperregistry-Lifecycle/-Listener und dynamische Erweiterungen bleiben offen. |
| Vollständige `DefaultedMappedRegistry` | `getValue/byId/getId` können auf Default fallen; `getOptional` verweigert unbekannte Namen. FerrumC wählt für Admission ausdrücklich checked None statt Pigfallback. | Bewusster strikter Admissionvertrag; keine vollständige DefaultedRegistry-/Bukkit-APIparität. |
| Relevanter EntityType-Sourcepatch und zuvor gelesene vollständige Spawn-/Lookup-/Removalconsumer | Original hat Spawn-/Remove-/Tracking-/Tickconsumer und Events. Keiner davon wird durch Typenlookup ausgelöst. | Metadata, Dimensionen, Kollision, Spawn-Eignung, Tracking, AI, Storage, Cross-shard und Pluginconsumer bleiben eigenständige Tickets. |

Ein bekannter Typ ist kein bereits spielbarer Mob. Datenvollständigkeit schließt
weder SIM-03.2 noch eine Survivaletappe oder JavaPlugin-Kompatibilität ab.

## Ausgeführte Abnahme

Finaler lokaler Lauf vollständig grün; Rootreview und Integration folgen:

| Prüfung | Ergebnis / Log |
| --- | --- |
| Registry default tests | 37 bestehende Unittests, 3 echte Buildparsercontracts, 5 öffentliche Identitycontracts, 8 Doctests inkl. opaquem Konstruktor-Compilefail; `/tmp/reg-entity001-tests.log`. |
| Exhaustiver öffentlicher Oraclevergleich | Alle 151 Originalnamen/Wire-IDs, reverse lookup, bare/name-ID-Gleichheit, unique Player, vollständige geordnete Iteration. Kein selbst erfundener Typ als Abnahme. |
| Negative und Buildparserfälle | Negative/unknown IDs, malformed Names, keine Pigdefaults; truncate/duplicate keys/extra fields, invalid registry/default/names/ids/counts, bool/float/overflow, >64KiB, fehlender Player; unveränderte Originalfixture. |
| Strict Clippy all-targets | Grüne Registry samt Buildcode/Tests; `/tmp/reg-entity001-clippy.log`. Früher index-Cast unter Concurrent-Clippy gefunden und vor Freeze durch generated i32range beseitigt. |
| Rustfmt | Finale Registryprüfung grün; `/tmp/reg-entity001-fmt.log`. |
| Helper mit vollständigem Originalreport | 6 Selftests; complete151-Subsetgleichheit und unveränderte Exportoutputs; `/tmp/reg-entity001-helper.log`. |
| Helper mit Python `-O`, offline | Dieselben 6 Selftests inkl. require-/Provenienz-/Malformationrefusal; `/tmp/reg-entity001-helper-optimized.log`. |

Statische allocationfreie Lookupimplementierung ist strukturell geprüft;
keine allgemeine Minecraft-CPU-/RAMbenchmarkbehauptung. Kein gestarteter Paper-
oder echter Clientvergleich für Entityverhalten. Root organisiert Fremdreview,
separaten Registrycommit und erst danach den Simconsumer.
