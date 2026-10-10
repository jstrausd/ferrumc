# REG-WORLDGEN001 — unabhängige Registryabnahme

Ergebnis: **CLEAN** für den begrenzten Originaldatenproducer. Reviewer: Root;
Implementer: `feature_catalog`. Kein Densitycompiler, Seedrouter, Terrain oder
Gameplay wird durch diese Abnahme freigegeben.

## Unveränderlicher Prüfstand

Isolierte Basis `3a5b4d62866d5f81d9820ec05dd3843389d32d90` unter
`.codex-tmp/reg-worldgen001-root-checkout`, ergänzt um exakt zwölf Dateien aus
`/tmp/reg-worldgen001-freeze.sha256`. Alle zwölf Hashes wurden vor den Prüfungen
und nach den zusätzlichen Reviewerproben in Hauptcheckout und Snapshot geprüft.
Keine offenen Storage-/Worldänderungen oder Lockänderung wurden übernommen.

Runtime-API-SHA256:
`173af81d7eace6620efa6366d6e21a9ac48b1ae146c8762d8e33105667ef0070`.
Die abschließende Änderung des Briefstatus/Reviewlinks ist rein dokumentarisch;
die übrigen elf Freeze-Dateien bleiben bytegleich.

## Quellen- und Ownershipprüfung

Die endgültigen Runtime-, Buildparser-, Schemavalidator-, Codegenerator- und
Testquellen wurden vollständig geprüft. Opaque Familien-IDs besitzen nur
geprüfte Lookups, getrennte Typen und statische Iteratoren. Es gibt keine
Runtime-JSONabhängigkeit, Init-I/O, mutable globale Registry, Seedwahl oder
Sampler. JSON, Stats und direkte typisierte Dependencies entstehen ausschließlich
aus dem unveränderten Fixture mit Produktions-SHA-/Byte-/Versionspins.

Die vollständigen relevanten Originalcodecs in `DensityFunction`,
`RegistryFileCodec`, `DensityFunctions`, `NoiseRouter`, `NoiseSettings`,
`CubicSpline.codec`, Surface-Rule-/Conditionquellen, `VerticalAnchor` und
`Climate.Parameter/ParameterPoint` wurden mit den bereits gelesenen Noise- und
Settingsquellen abgeglichen. Referenz: Minecraft 1.21.8/772/4440, angewandte
Paper-Build60-Quellen, Commit `29c8822d90899c89d2689338e81a98f690bcba12`.
Der vollständige DensityFunctions-Patch ändert die End-Insel-Auswertung und
deren Cache, keine hier erzeugten Daten oder Codecpfade.

Clamp verwendet Directcodec; gewöhnliche Densityarguments verwenden
Holderhelper; Shiftarguments verwenden Noiseholder. Splinecoordinates verwenden
den Densityholdercodec, Splinevalues den rekursiven Float-/Multipointcodec.
`derivative` ist ein Pflichtfeld. Benannte Referenzzyklen werden auch über
Splinecoordinates und ungenutzte Definitionen erfasst. Block-/Biome-/Surface-
und RNGnamen bleiben von Noise-/Densityidentitäten getrennt.

Die Admission ist absichtlich enger als die allgemeinen Minecraftcodecs:
gepinnte Builtinformen, finite Werte, eigene Ressourcenbounds und der
Bootstrapguard für aufsteigende Splinepunkte. Die Buildprüfung verwendet f64
und bei Splinelocations einen f32-Cast; sie ist kein allgemeiner Decimal→f32-
Runtimecodec. Der Worldcompiler erhält unveränderte Originalnumbertokens und
muss diese selbst direkt und originalgetreu als f32 lesen. Keine Schema- oder
Datapackparität außerhalb des gepinnten Bestands wird behauptet.

## Unabhängig ausgeführte Prüfungen

- Gesamte isolierte Registrydebugsuite: **69 bestanden**, keine Fehler/ignored;
  37 Unit-,21 Integration- und11 Doctests.
- Gesamte isolierte Registryreleasesuite: dieselben **69 bestanden**, keine
  Fehler/ignored; Log `.codex-tmp/reg-worldgen001-root-release.log`.
- Vier zusätzliche eigene Proben: **4 bestanden**. Indirekter Zyklus über
  verschachtelte Splinecoordinate und zweite benannte Ressource; Signedzero und
  benachbarte f64-Amplitudenbits bis zur Emission; ungültige Geometrie,
  Blockproperty und Surfacebiome; escaped Duplicatekey und ungültige JSONtails.
- Strict `cargo clippy --offline --locked -p ferrumc-registry --all-targets --
  -D warnings` und `cargo fmt --all --check`: bestanden auf wiederhergestelltem
  exakten Freeze.
- Originalhelper normal und unter Python `-O`: **102/102** Ressourcenpfade und
  Inhalte vollständig gleich, datagen/JAR-Rawbytes gleich, exakte Decimalinhalte
  und alle **195** Noiseamplituden-f64-Bits geprüft; drei Manifestindizes gleich.

Offizielles Bundle SHA256:
`2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7`;
innerer Server:
`2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`.
Fixture-/Runtime-Slices und JAR-Whitespace werden nicht als bytegleich
ausgegeben. Bytegleich sind die separat geprüften datagen/JAR-Resourcefiles.

Logs: `.codex-tmp/reg-worldgen001-root-{tests,extra-tests,clippy,fmt,original,
original-optimized}.log`. Eigene Proben blieben ausschließlich im isolierten
Reviewcheckout; archiviert als `.codex-tmp/reg-worldgen001-root-probes.rs`,
SHA256 `0b3b827d15674bfc180822ef18527431781416b64e426976a4b46f43df20d148`.
Die exakte Agenttestsuite wurde danach vor strictchecks wiederhergestellt.

Die Ressourcengleichheit ist unabhängig geprüft. Density-/Seed-/NoiseChunk-
Auswertung, Biome/Surface, natürliche Chunks und eine tatsächliche Paper-
Gameplay-Differenzialabnahme bleiben getrennte offene Consumer.
