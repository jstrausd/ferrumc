# CAT-02.1 — Reproduzierbarer Originaldatenexport

Stand: 10.10.2026. Basis: `main` / `d96228b8`. Zielversion: Minecraft Java
**1.21.8, Protokoll 772, DataVersion 4440**. Tools-/Datenarbeit ohne Runtimecrate.
Dieses Ticket erfasst die unten definierte Vanilla-Datenmenge. CAT-02 insgesamt
bleibt teilweise umgesetzt; Datenexport aktiviert kein Gameplay.

## Implementierter Umfang und Dateibesitz

- [minecraft-original-export.py](../../scripts/minecraft-original-export.py):
  Download-/Archiv-/JSON-Prüfung, offizielle Datenpipeline, Export und Offlinecheck.
- [minecraft-feature-inventory.py](../../scripts/minecraft-feature-inventory.py):
  bestehende Vendorpins erhalten; unabhängig gepinnte Originalfixtures ergänzen
  den Bestand. Inventoryschema jetzt 2.
- [Bestandsbericht](../fixtures/minecraft-1.21.8-feature-inventory.json):
  regeneriert durch das Inventoryscript; keine Spielmechanikstatus aus Daten abgeleitet.
- Acht erzeugte Dateien im eigenen Namespace
  [minecraft-1.21.8-original](../fixtures/minecraft-1.21.8-original/manifest.json)
  sowie diese Tasknotiz. Keine Server-JARs oder ausgepackten Klassen versioniert.

Board, Katalog, CI und Gitintegration gehören Root. Bestehende Daten unter
`fixtures/protocol/1_21_8` und `ferrumc-registry/data` bleiben unverändert.

## Tatsächlicher Originalpin

Der offizielle [Versionindex](https://piston-meta.mojang.com/mc/game/version_manifest_v2.json)
enthielt bei der Recherche für 1.21.8 den folgenden Packagepin. Der bewegliche
Index ist nur die Entdeckung; Reexport verwendet direkt die festgelegte
Package-URL und prüft ihre Bytes. Ein API-JAR oder ein lokaler Paper-Checkout
wird weder ausgeführt noch als Exportdatenquelle verwendet.

| Artefakt | Bytes | Integrität |
| --- | ---: | --- |
| [Versionmanifest](https://piston-meta.mojang.com/v1/packages/b1bb44ad769609925328838694503a8dcac7f529/1.21.8.json) | 43.719 | Mojang SHA-1 `b1bb44ad769609925328838694503a8dcac7f529`; SHA-256 `3bf5cdd444dfa89199fe4f8a92b6c79f4e4fa8977ff52029c3acd99378506fad` |
| [Offizielles Server-Bundle](https://piston-data.mojang.com/v1/objects/6bce4ef400e4efaa63a13d5e6f6b500be969ef81/server.jar) | 57.555.044 | Mojang SHA-1 `6bce4ef400e4efaa63a13d5e6f6b500be969ef81`; SHA-256 `2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7` |
| Inneres Server-JAR `META-INF/versions/1.21.8/server-1.21.8.jar` | 18.900.451 | SHA-256 `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`, auch vom gepinnten Bundle `versions.list` bestätigt |

Manifest und Bundle werden vor Verwendung anhand der im Helper festgelegten
Hashes/Längen geprüft. Das Versionmanifest muss zusätzlich Version, Java 21
und das genaue Serverdownloadobjekt bestätigen. Cachetreffer werden erneut
vollständig geprüft; beschädigte Cachedateien werden abgewiesen.
Die eigene `version.json` im gepinnten inneren JAR bestätigt zusätzlich
1.21.8, Protokoll 772, DataVersion 4440, Java 21 und den Release-Status.

## Export, Vollmengenvergleich und Normalisierung

Der Helper startet das unveränderte offizielle Bundle mit Java 21 und
`-DbundlerMainClass=net.minecraft.data.Main`, `--reports --server --output generated`.
Jeder Aufruf verwendet ein neues temporäres Arbeitsverzeichnis; ein älterer
Datagenoutput kann fehlende Dateien nicht ersetzen. Das Bundle bringt seine
eigenen Laufzeitbibliotheken mit. Es wird kein Minecraft-Spielserver geöffnet.

Aus `reports/registries.json` werden `minecraft:entity_type` und
`minecraft:recipe_serializer` einschließlich ihrer echten `protocol_id`-Werte
übernommen. Gültige vollständige IDs, Default und Zahl werden geprüft.
Der vollständige erzeugte Registryreport hat **466.543 Bytes**, SHA-256
`177a6ad99d7b6d28f27e49231d15a5ccdb262bed9ac07f7e66dab5d32cc20c27`;
sein Pin steht im Exportmanifest. Übrige Reportregistries bleiben unerfasst.

Datapackauswahl: ausschließlich die Standard-Vanilla-Ressourcen unter
`data/minecraft/recipe/`, `loot_table/`, `worldgen/` und `dimension_type/`.
**3.609 ausgewählte Originaldateien / 3.448.497 Bytes**. Diese genaue Menge
wird mit den eingebetteten Ressourcen desselben gepinnten inneren Server-JARs
verglichen: gleiche Dateinamenmenge und jede Datei **bytegenau gleich**.
Ein fehlender, zusätzlicher oder veränderter Export scheitert. Der Vergleich
gilt für diese Auswahl und erweitert sie nicht auf das gesamte Minecraft.
Er fängt auch fehlende Ausgaben trotz lediglich geloggter DataProvider-IOfehler ab.

Die eingecheckten Fixtures gruppieren vollständige JSON-Werte nach Registry
und Resource-ID. Schlüssel/Dateien werden sortiert; Arrays bleiben geordnet.
Dezimalzahlen werden exakt als `Decimal` eingelesen und ohne Float-Rundung
wiedergegeben, einschließlich `-0.0`. Density-Function-Werte können auch
Literalzahlen oder Referenzstrings sein und werden erhalten. Originaldateibytes
werden im Manifest durch je einen sortierten Path-/Bytes-/SHA-256-Indexhash
pro Kategorie gebunden. Das offizielle Versionmanifest bleibt byteidentisch.

Die acht fertigen Fixturehashes **einschließlich manifest.json** sind separat
im Helper festgelegt. Offlineprüfung liest ihre Pins nicht aus dem bearbeitbaren
Manifest. Änderungen an Daten und zugehörigen Manifestangaben können sich
dadurch nicht selbst legitimieren.

## Erfasste Daten und Größen

| Erzeugte Datei | Datenumfang | Bytes |
| --- | --- | ---: |
| [entity-types.json](../fixtures/minecraft-1.21.8-original/entity-types.json) | 151 Entitytypen, Default und originale Protokoll-IDs; keine Entityeigenschaften/KI | 9.319 |
| [recipe-serializers.json](../fixtures/minecraft-1.21.8-original/recipe-serializers.json) | 22 Serializertypen und originale Registry-IDs | 1.681 |
| [recipes.json](../fixtures/minecraft-1.21.8-original/recipes.json) | 1.407 vollständige Standard-Vanilla-Rezept-JSONs | 501.688 |
| [loot-tables.json](../fixtures/minecraft-1.21.8-original/loot-tables.json) | 1.246 vollständige Standard-Vanilla-Loot Tables, inklusive Inlinebedingungen | 1.366.578 |
| [worldgen.json](../fixtures/minecraft-1.21.8-original/worldgen.json) | 952 JSON-Werte in 14 Worldgen-Registries | 2.530.249 |
| [dimensions.json](../fixtures/minecraft-1.21.8-original/dimensions.json) | 4 Dimensiontypen; jeweils Overworld/Nether/End-Stems aus 6 World Presets | 8.752 |
| [version-manifest.json](../fixtures/minecraft-1.21.8-original/version-manifest.json) | Unveränderte offizielle Versionsmetadaten | 43.719 |
| [manifest.json](../fixtures/minecraft-1.21.8-original/manifest.json) | Provenienz, Input-/Outputpins, Kategorien und Grenzen | 7.061 |

Gesamt: **4.469.047 Bytes**. Manifest-SHA-256:
`d5b4506f5749d114d7b54c50231ee00f6f15d14b2826f8c39489df06932ed9ee`.

Worldgenfamilien: Biome 65, configured_carver 4, configured_feature 224,
density_function 35, flat_level_generator_preset 9,
multi_noise_biome_source_parameter_list 2, noise 60, noise_settings 7,
placed_feature 258, processor_list 40, structure 34, structure_set 20,
template_pool 188, world_preset 6. Dimensiontypen sind eine separate Registry;
Vanilla datagen erzeugt keine einzelnen `dimension/*.json`-Ressourcen.
Dimensionstems werden aus den World Presets erfasst, insgesamt 18.

## Reproduzieren und offline prüfen

Aus dem Repositoryroot, Python 3.11 oder neuer:

```sh
python3 scripts/minecraft-original-export.py --self-test --check
python3 scripts/minecraft-feature-inventory.py --self-test --check
```

Diese Befehle benötigen weder Java noch Netzwerk oder Paper-Checkout.
Ein fehlendes Originalfixture wird als Fehler behandelt, ohne Netzwerkfallback.

Ein frischer Originalexport mit Downloads und Vergleich:

```sh
python3 scripts/minecraft-original-export.py --export --check --java /path/to/java21
```

Der Helper verwendet standardmäßig den ignorierten Cache
`.codex-tmp/original-1.21.8`. `--cache /another/directory` prüft auch einen
kalten Downloadpfad. `--offline` bei `--export` fordert vorhandene, erneut
verifizierte Downloads; die Datenpipeline läuft trotzdem neu. Ohne `--check`
schreibt `--export` ausschließlich die acht bereits gepinnten Outputs atomar
pro Datei. Ein Versions-/Datenwechsel benötigt eine ausdrückliche neue Pinprüfung.
Ein normales Inventoryregenerieren erfolgt mit
`python3 scripts/minecraft-feature-inventory.py`.

## Limits und Fehlerpfade

| Grenze | Verhalten |
| --- | --- |
| Download | Nur die beiden festgelegten HTTPS-URLs, keine Redirects; genau gepinnte Länge/Hash; maximal 64 MiB. Downloaddeadline 120 s zwischen `read1`-Chunks, IOtimeout 20 s begrenzt einen gerade laufenden Read. Temporärdatei wird nur nach Integritätsprüfung als Cachedatei übernommen. |
| ZIP | EOCD-Prüfung vor Central-Directory-Allokation: höchstens 32.768 Einträge und 8 MiB Directory; kein ZIP64/Multidisk. Pfade, Duplicate, Symlink, Encryption und Compression werden vor Decompression geprüft. Je Eintrag höchstens 64 MiB, 384 MiB expandiert insgesamt, Ratio höchstens 512. Kein `extractall` im Helper. |
| Pfade | ASCII, keine absoluten/Drive-/Backslashpfade, leeren/dot/dotdot-Segmente; höchstens 512 Zeichen und 32 Segmente. Fixturedateien sind eine feste Acht-Dateien-Liste. |
| JSON | Pro Quelle/Fixture höchstens 16 MiB; eindeutige Keys, gültiges UTF-8, keine unpaired Surrogates/NaN/Infinity, begrenzte Zahlen; Tiefe 96, eine Million Nodes, 65.536 Array-/Objecteinträge, 1 MiB UTF-8-Bytes je String/Key. |
| Aggregate | Höchstens 16 MiB gewählte Datapackquellen, 64 MiB gesamter erzeugter Datagenoutput, 32 MiB sämtliche eingecheckten Originalfixtures. |
| Java | Java-21-Probe und Datagen mit eigener Frist, Datagen maximal 180 s und `-Xmx1G`; neue temporäre Arbeitsverzeichnisse werden auch bei Fehler entfernt. |

Byte-/Countcaps begrenzen Eingaben vor Übernahme; JSON-Treecaps werden zusätzlich
nach dem durch die Bytegrenze begrenzten Parsing geprüft. Datagen-Datei-/Gesamtcaps
werden vor Fixtureübernahme am erzeugten Baum geprüft. Das ausgeführte Programm
ist das vorher festgelegte SHA-verifizierte Mojang-Artefakt.

## Gelesene Originalreferenz

[Paper-Referenz](../PAPER_REFERENCE.md): 1.21.8 Build 60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`, vollständige Basis mit angewandten
Source-/Featurepatches. Der Checkout wurde nur zur Prüfung der Exportverträge
gelesen; reproduzierbare Daten stammen ausschließlich aus Mojangs Server-JAR.

| Vollständig gelesener Exportpfad unter `net/minecraft/` | Befund |
| --- | --- |
| `data/Main`, `DataGenerator` | `--reports` und `--server` wählen Report-/Vanillaprovider; Builtin-Experimentpacks erhalten eigene Unterverzeichnisse. |
| `data/info/RegistryDumpReport`, `DatapackStructureReport` | Report liefert statische Registry-/Entry-IDs und Default; Datapackpfade und dynamische Registries sind getrennt. |
| `data/registries/VanillaRegistries`, `RegistriesDatapackGenerator`, `RegistryPatchGenerator` | Registrylookup aus Bootstraps; registrierte Werte werden mit ihren echten Codecs unter Registry-/Resourcepfaden serialisiert. |
| `data/PackOutput`, `DataProvider`, `CachedOutput` | Pfadbildung und stabile JSON-Ausgabe. IOException in saveStable wird geloggt; eigener Vollmengen-/Bytevergleich ist daher erforderlich. |
| `data/recipes/RecipeProvider.Runner`, einschließlich vollständigem `run` | Doppelte Rezept-IDs scheitern; `Recipe.CODEC` erzeugt Rezeptwerte, Advancements getrennt. Kein Matcher implementiert. |
| `data/loot/LootTableProvider`, `packs/VanillaLootTableProvider` | Vollständige Providerliste, Required-Table-/Contextvalidierung, anschließend `LootTable.DIRECT_CODEC`. |
| `data/worldgen/DimensionTypes`, `world/level/levelgen/presets/WorldPresets` | Vier Dimensiontypen; sechs Presets mit je drei Stems. Kein Terrainalgorithmus in FerrumC aktiviert. |

Relevante Sourcepatchverzeichnisse und Featurepatchbezüge wurden geprüft.
Der einzige Data-Sourcepatch betrifft `VanillaChestLoot` und entfernt eine
Compass-Damage-Funktion. Das illustriert den Versions-/Vanilla-/Paperunterschied:
dieser Export folgt den ungepatchten offiziellen Vanillaressourcen. Es wird
keine Gleichheit zu sämtlichen Paperdaten oder Plugins behauptet.

## Prüfung und verbleibende Grenzen

Tatsächlich ausgeführte Nachweise, jeweils Exitcode 0:

| Prüfung | Ergebnis / Log |
| --- | --- |
| Originalhelper `--self-test --check` | **17 Offline-Selftests bestanden**: Parser-/Bounds-/Provenienz-/Fixtureintegrität; `/tmp/ferrumc-cat02_1-helper-tests.log`. |
| Inventory `--self-test --check` | **7 Selftests bestanden**: alte Vendorpins, neue Originalpins, Scopegrenzen und Determinismus; `/tmp/ferrumc-cat02_1-inventory-tests.log`. |
| Reexport aus verifizierten Downloads, neues Datagen-Tempdir | **Alle acht Outputs byteidentisch**, kompletter gewählter Original-Dateimengen-/Bytevergleich bestanden; `/tmp/ferrumc-cat02_1-reexport.log`; native Providerlog `.codex-tmp/original-1.21.8/last-export.log`. |
| Reexport aus kaltem Cache mit echten Downloads | **Alle acht Outputs byteidentisch** nach neuem Download und eigenem Datagen-Tempdir; `/tmp/ferrumc-cat02_1-online-reexport.log`; native Providerlog `.codex-tmp/original-1.21.8-online/last-export.log`. |
| Python-Syntax / lokale Doclinks / begrenzter Diffcheck | Bestanden. Keine Cargo-/Maven-/Runtimechecks für diesen Toolsumfang neu benötigt oder ausgeführt. |

Root fand in einer frühen Review einen echten ZIP64-Bypass: Python befolgt
einen ZIP64-Locator auch mit unveränderten, nicht-sentinel Legacy-EOCD-Feldern.
Die Korrektur verweigert diesen Locator **vor** `ZipFile`-Konstruktion. Eine
Regression verwendet genau das gemischte Archiv und prüft, dass der Directory-
Parser gar nicht aufgerufen wird; dadurch können ZIP64-Werte die Vorlimits
nicht nachträglich ersetzen. Diese frühe Korrektur ersetzt kein unabhängiges
Abschlussreview.

Das unabhängige Review fand außerdem eine Einheitenabweichung: die erste
Stringprüfung begrenzte Unicode-Zeichen statt UTF-8-Bytes. Jetzt prüft sie
zusätzlich die tatsächliche UTF-8-Bytelänge für Werte und Objektkeys.
Die Regression akzeptiert exakt 1 MiB Emoji-UTF-8 und verweigert die nächste
vier Byte lange Codepointfolge. Die Originalfixtures bleiben byteidentisch.

Bewusst unerfasst bleiben experimentelle Packs, Tags und Tagauflösung,
NBT-Strukturtemplates, weitere statische/dynamische Registries, Commands,
Advancements/Statistiken und Bukkit/Paper-/NMS-Matrizen. Inline-Lootbedingungen
sind Daten; Predicate-/Functionregistries und ihre Interpretation bleiben offen.
Worldgen-JSONs enthalten Referenzen auf Templates/Tags und implementieren weder
Noise/RNG noch Terrain/Seed-Parität. Rezept-IDs umfassen auch Special Recipes,
deren Serializeralgorithmen gesondert umgesetzt werden müssen.

Die neue Inventoryaufnahme markiert Worldgen/Structures und Loot/Predicates
entsprechend als **partially_captured_data_only**. Vollständiger Survivalablauf,
Entitysimulation, Crafting, Lootverbrauch, Dimensionreisen und Pluginverhalten
benötigen eigene Consumer-/Implementierungs- und Abnahmetickets.

Pythonwerkzeuge bleiben MIT. Die Original-Minecraft-Daten bleiben
Mojang-/Microsoft-Inhalte und erhalten durch ihre Aufnahme keine MIT-Lizenz.
Versionmanifest, offizielle Downloadlinks, Hashes, [EULA](https://www.minecraft.net/en-us/eula)
und [Usage Guidelines](https://www.minecraft.net/en-us/usage-guidelines) stehen
im Exportmanifest; keine pauschale Lizenzfreigabe für Minecraft-Inhalte abgeleitet.
