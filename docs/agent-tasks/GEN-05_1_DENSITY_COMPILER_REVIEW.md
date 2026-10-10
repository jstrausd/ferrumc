# GEN-05.1 — Unabhängiger Compilerreview

Reviewer: Root, 10.10.2026. Ergebnis **CLEAN** für den geschlossenen,
begrenzten Compiler der originalen Minecraft-1.21.8-Densitydefinitionen und
Router. Kein bestätigtes Runtimefinding. Sampling, Seedbindung, Chunkwrapper
und natürliche Blockausgabe sind weitere Consumer und bleiben offen.

## Geprüfter Snapshot und Quellen

Isolierte detached Basis `0276f3d4`, Reviewcheckout
`.codex-tmp/gen05_1-root-checkout`. Genau die 16 eingefrorenen World-/Ticketfiles
plus Roots einzelne Worldpackage-Dependencyzeile in Cargo.lock wurden kopiert.
Aktive Storage-Migrationsänderungen gehören nicht zur Reviewbasis.

Agentmanifest `.codex-tmp/gen05_1-world.freeze.json`, SHA256
`1506d86ea59a6f9c6a950e76bfc5b761d0fdce94e063ec3c2dcef4c67c06eaca`.
Rootinput `.codex-tmp/gen05_1-root-input.freeze.json` erfasst alle17 Files;
Lock-SHA256 `87a7baae3407eb4b7dcb60cb10096ed073cb66660dc3752a784ae8973d6a7da2`.
Keine Dependencyversion, Source oder Checksum wurde geändert.
Vor Originalabnahme und nach Restore der eigenen Probes waren Main und
Reviewcheckout jeweils **17/17 bytegleich**. Nur der Briefstatus wird bei
Integration um diesen Review ergänzt; Runtime und Originalcorpus bleiben gleich.

Vollständig geprüft: `density.rs`, `density/compile.rs`, `density/graph.rs`,
`density/json.rs`, alle neuen Compiler-/Graph-/JSONtests und der vollständige
Java-/Python-Oracle samt Messskript. Bestehende Noise-/Splinequellen und
Goldens wurden nicht verändert. Die Source-/Patch-/Codecmatrix des
[Taskbriefs](GEN-05_DENSITY_COMPILER_ROUTER.md) bleibt die fachliche Referenz:
Minecraft1.21.8/Protokoll772/DataVersion4440, PaperBuild60/Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Vollständige Density-, Router-,
RandomState-, NoiseChunk-, CubicSpline-, Registryholder- und Numericcodec-
Pfade wurden geprüft; Quellenreview ist keine Live-Paper-Spielabnahme.

## Tatsächlicher Original-JAR-Nachweis

Root hat den finalen Oracle unabhängig mit frisch kompiliertem Helper aus
hashgeprüftem Original-JAR, offiziellen Mappings und allen43 Librarypins
erneut ausgeführt, normal und mit Python `-O`. Beide Läufe liefern **Fixture
und Manifest bytegleich** zum finalen Snapshot:

- `.codex-tmp/gen05_1-root-original[.log]`;
- `.codex-tmp/gen05_1-root-original-optimized[.log]`.

128 Records,278269 Bytes:35 Definitionen,7 Router,72 Densitycodecproben,
8 Settingsproben und6 Factoryproben. Alle32 serialisierten Formen sind durch
tatsächliche Originalcodecs erfolgreich dekodiert. 80 Codecproben enthalten
71 erfolgreiche und9 verweigerte Antworten. Die Factoryproben sind getrennte
Evidenz für den folgenden Visitorconsumer und keine fertige Auswertung.

Fixture-SHA256
`856bd9f9d25642864945a11bd2ea907fa3c90b4fda8bd7b1153017b74da65a73`;
Manifest-SHA256
`57f4adec761f15dfb6b26605d51bba1872a1621589595479b28491bc30fe257d`.
Die ursprüngliche125-Record-Childlieferung wurde ausdrücklich erneut generiert;
Root hat die fünf finalen Artefakte, INT-Zusatzproben und minimalen Java-/Python-
Diffs samt `.codex-tmp/gen05_1-codec-child-to-final-provenance.json` geprüft.
Der alte Childfreeze wird nicht als unveränderte finale Lieferung bezeichnet.

Unabhängige Graphmessung `.codex-tmp/gen05_1-root-measured-graph.json`:
Union3102 Vertices/3435 Edges/Depth87, darunter1299 Splinevertices,
1290 Splinepunkte und64 Marker. Overworldrouter1091/1217/87,
End100/85/24, die übrigen legacy Router82/69/22. Beide Holderhelperorigins,
raw Splinecoordinateholder, shared named Targets und Splineedges werden
tatsächlich gezählt. Die genehmigte combinedDepthcap96 wird an96/97 geprüft.

## Vertragsprüfung

| Grenze | Ergebnis |
| --- | --- |
| API und Ownership | Nur typisierte geschlossene Registryresources, private immutable Arcgraphen, kein Caller-JSON/unchecked Handle/Sample-/Seedfallback. Beide Entries validieren alle35 Definitionen einschließlich ungenutzter Closure. |
| Originalcodecstruktur | HolderHolder für beide Densityhelperorigins; raw Coordinateholder gesondert; Clamp DIRECT_CODEC, Shift NoiseHolder, required Spline-derivative. Alle7 Router mit15 Feldern und checked Geometry; deferred Material-/Surface-/Spawnpayloads bleiben separat. |
| Zahlen und Parser | Numeric-Tokens bleiben erhalten; f32 direkt aus Decimal, f64 getrennt; Gson INT truncation/wrapping vor Range. Vollständige Grammar, Unicode/escaped Duplicatekeys, trailing Reject und alle Admissioncaps. |
| Joint Graph | Density/Spline/Coordinate/Holder gemeinsam; alle Edges typgeprüft, keine getrennten Cycleprüfungen. Shared Targets erhalten Depth/Reachability; Graph-/Splinecaps gelten vor Veröffentlichung. |
| Eigene Admissionpolicy | Finite DOUBLE/FLOAT, aufsteigende f32-Splinepunkte und Unknownfieldreject ausdrücklich enger als Originalcodec. Inline-Noiseholderobjects außerhalb des geschlossenen Builtinscopes verweigert, keine vermeintliche vollständige Datapack-API. |
| Ressourcen | Input128KiB, aggregate2MiB, IR4096Vertices/8192Edges/Depth96, separate JSON-/Splinecaps. Encoded-/Graphcaps sind keine RSS-/TPSmessung. Keine Runtime-I/O, Threads, globale mutable Maps oder unbounded Queues. |

## Unabhängig ausgeführte Prüfungen

Originalsnapshot, `--offline --locked -j2`, eigenes
`.codex-tmp/gen05_1-root-target`:

- Debug und Release jeweils **177 bestanden**, 0 fehlgeschlagen/ignoriert:
  134 Unit +43 Integration. Logs `.codex-tmp/gen05_1-root-{debug,release}.log`.
- Strict `cargo clippy -p ferrumc-world --all-targets -- -D warnings`: grün,
  `.codex-tmp/gen05_1-root-clippy.log`.
- `cargo fmt --all --check` im isolierten Workspace: grün,
  `.codex-tmp/gen05_1-root-fmt.log`.

Drei eigene zusätzliche Probes, jeweils Debug/Release grün:

1. 256 deterministische gemeinsame Density-/Spline-/Holder-DAGs gegen ein
   unabhängig konstruiertes Edge-/Depth-/Reachabilitymodell, shared Edges und
   mehrere Roots.
2. Ungenutzter Namedresource↔Splinecoordinatecycle sowie dangling und
   rollenwidrige Edges werden verweigert.
3. Escaped Schemafelder behalten Originalnumeric-Tokens: direkter f32-Midpoint
   `3f800001`, negative Nullen und escaped Duplicatekeyreject.

Probequelle ausschließlich `.codex-tmp/gen05_1-root-probes.rs`, SHA256
`4c0c81e06a5e1fbf9998dc46b810214272ce6c4d60b4553c6fd8359170e36f33`.
Logs `.codex-tmp/gen05_1-root-extra-{debug,release,clippy,fmt}.log`.
Strict Alltargetclippy und Fmt ebenfalls grün. Root korrigierte ausschließlich
Literalformatierung und Helperaufteilung seiner eigenen Probe; dies ist kein
Produktionsfinding. Die Probe wurde nur dem isolierten Testfile angehängt und
danach vollständig aus dessen gespeicherten Originalbytes entfernt.
Abschließende Main-/Snapshot-Freezeprüfung17/17 grün.

## Nächster konkreter Consumer

GEN-05.2 bindet tatsächliche Seeds, Noise-/Factory-/Bounds-Views und originale
Scalar-/Climateauswertung; GEN-05.3 folgt mit echten NoiseChunkwrappern und
Interpolation. Danach Aquifer/Material/Ore, Biome/Surface und konkrete Chunks.
Vanillas overflowing EndIslanddistanz und Papers widened Distanz sind getrennte
Semantiken und benötigen getrennte tatsächliche JAR-Proben. Das bereits
vorbereitete gepatchte Paper-JAR wurde noch nicht als Server gebootet.

Flat-v1 bleibt der aktive Generator. Keine Survival-Gesamtabnahme,
vollständige Java-/Paper-Pluginfähigkeit oder gemessene Performanceparität
wird durch diesen Compilerreview abgeschlossen.
