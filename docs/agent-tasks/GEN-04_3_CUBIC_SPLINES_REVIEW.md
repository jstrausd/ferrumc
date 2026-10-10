# GEN-04.3 — Unabhängige Splineprüfung

Stand: 10.10.2026. Implementer `item_transfer_primitives`, Reviewer `/root`.
**CLEAN nach F1-Dokumentationskorrektur.** Dieser Bericht prüft ausschließlich
den begrenzten World-Splinevertrag, keine natürliche Welt oder Survivaletappe.
Keine Produktionsänderung durch den Reviewer.

## Isolierte Basis und tatsächliche Quellenlektüre

Detached Checkout `.codex-tmp/gen04_3-root-checkout` auf `916251da`, ergänzt um
exakt zwölf Implementer-Freeze-Dateien und private zusätzliche Reviewtests.
Laufende Registry-/Storageänderungen sind nicht Bestandteil dieses Checkouts.
Finales Reviewfreeze `.codex-tmp/gen04_3-root-reviewed.sha256`, SHA256
`b37ffb530a14afa0ad35d65521f25cdca8a7a018957367f3493371f263daae40`.
Alle zwölf Hashes stimmen am Reviewabschluss im isolierten und Hauptcheckout
überein; eine anschließende Briefstatusaktualisierung ändert keinen Code/Test.
33 vorherige World-Quell-/Test-/Oracle-/Fixturedateien bleiben erhalten.

Integrationsfreeze nach der reinen Briefstatus-/Reviewlinkaktualisierung:
`.codex-tmp/gen04_3-freeze.sha256`, SHA256
`62050b97538f84a061418462207b3e5d63734a69f74594f6c7f7602f88433719`.
Alle elf Code-/Test-/Fixture-/README-/Invarianten-Dateien bleiben zum geprüften
Snapshot identisch. Finaler BriefSHA256
`83f7ea57f1739d9a2a839b33b638993c986c8a7f8d429b1d7159d9538347eb72`.

Vollständig gelesen: öffentliche Spline-/Fehler-/Ownershipoberfläche,
`spline/validate.rs` und `spline/evaluate.rs`, neue Publiccontracts,
Javaoracle und Pythonexporter, README/Invarianten sowie AGENTS/CLAUDE und
relevante Architektur-/ADR-/Referenzverträge. Originalbasis: Minecraft1.21.8,
Protokoll772; PaperBuild60/Commit `29c8822d90899c89d2689338e81a98f690bcba12`.
Vollständige angewandte `CubicSpline` und `ToFloatFunction`, vollständige
`Mth.binarySearch`-/f32-/f64-Lerpcallpaths, gesamte `TerrainProvider`-Klasse,
vollständiger DensityFunctions-Spline-/Coordinate-/Pointconsumer und dessen
angewandter Sourcepatch gelesen. Die EndIsland-Paperänderungen betreffen nicht
die hier geprüfte Splinearithmetik. Codec/Visitor/Profile/Densityadapter bleiben
Consumer; ihre Referenzlektüre aktiviert sie nicht.

## Geprüfter Vertrag und F1

Opaque private Felder, borrowed checked Specs, eigene immutable Graphdaten
und Arc-Clones erfüllen den vereinbarten Ownershipvertrag. Vor Graphallocation
werden alle Nodes, einschließlich ungenutzter Teilgraphen, validiert. Die
iterative fixed-capacity DFS prüft IDs/Zyklen vor Tiefe; die exakte Nodekappe
erzeugt keinen zusätzlichen unvalidierten Stackeintrag. Tiefe32 inklusive
Constantleaf begrenzt die spätere Rekursion. Sampling kostet Nodeentry,
Callback und Binsearchvergleich je eine Einheit vor Ausführung; es gibt
keine interne Heapallocation, Memoization oder mutable Samplerdaten.

Die Sourceprüfung bestätigt die originale f32-Reihenfolge, erste strikt
größere Binsearchlocation, Endpoint-/Derivativezero-Verhalten, linke vor
rechter Childauswertung und wiederholte Shared-Child-Anfragen. Diagnostik
bewahrt Java-Minmax mit NaN und Signedzero; sie clampet Samples nicht.
Graphlokale IDs sind Deklarationsindizes und beweisen keine Provenienz.
FerrumCs Ressourcenlimits und nonfinite Fehlerpolicy sind ausdrücklich eigene
Verträge; Callbackarbeit/-effekte liegen außerhalb des eigenen Workbudgets.

F1 war ein tatsächlich fehlgeschlagener strict Alltargetclippy-Lauf:
`README.md:130`, `doc_markdown`, unformatierter Produktname im neuen Spline-
Abschnitt. Der Implementer ergänzt ausschließlich Backticks. Keine Allow-
oder Lintlockerung, kein Runtimefix. Derselbe isolierte Clippyaufruf ist danach
grün. Das vorher gelieferte Clippylog lag vor der finalen READMEfassung.
RED bleibt unter `.codex-tmp/gen04_3-root-clippy-red.log` erhalten;
GREEN unter `.codex-tmp/gen04_3-root-clippy-green.log`.

Die zusätzliche nonfinite Policy ist separat geprüft: Ein überlaufender
linker Child stoppt Rust vor dem rechten Callback; Java darf weiterrechnen.
Nur erfolgreiche finite Samples beanspruchen originale Ergebnis-/Traceparität.
Typed Callbackfehler und frühere Callbackeffekte werden nicht rückgängig gemacht;
Graphdaten bleiben nach Fehlern unverändert.

## Ausgeführte unabhängige Abnahme

- Isolierte vollständige Worlddebugsuite: **163 bestanden**,0 fehlgeschlagen,
  0 ignored; sieben Ergebnisgruppen einschließlich leerem Rustdocbestand.
- Eigenes frisches Python-`-O`-Original-JAR-Reexport:38 Graphen/8260 Records,
  Fixture und Manifest bytegleich mit dem eingefrorenen Producerbestand.
  Bundle, inneres JAR, offizielle Mappings und alle43 Libraries hashgeprüft;
  frisch kompilierter Helper, Originalklassen zuerst im Classpath.
- Eigene zusätzliche32 deterministische Shared-DAG-Graphen, vier Koordinaten,
  vier Multipointlevels, unterschiedliche Locations/Derivatives, Sharedchildren,
  96 Kontexte je Graph und angrenzende Floats an jeder Rootlocation. Spec-RNG
  `java.util.Random(0x6d6353504c494e45L)` erzeugt nur Eingaben; Antworten stammen
  aus realen Original-JAR-Spline-/Diagnostikmethoden, ohne kopierte Formel.
- Erweiterter Corpus:70 Graphen/12554 Records, gegenüber dem Producer
  **4294 neue Records**:128 Koordinaten,512 Nodes,32 Diagnosen,
  **3462 finite Samples** und **160 echte Sourceexceptiontraces**.
  Die öffentliche Readerprobe vergleicht Ergebnisbits und Callbacktraces exakt,
  einschließlich aller zusätzlichen Diagnosen und Fehlerprefixe; Debug2 grün.
- Release: **20 bestanden**,0 fehlgeschlagen/ignored;10 ursprüngliche
  Splineverträge plus10 Publiccontracts mit erweitertem Originalcorpus.
  Die wiederverwendeten Grenztests zählen nicht als neue unabhängige Testideen.
- Finales strict Alltargetclippy einschließlich Dependencies mit `-D warnings`
  und scoped `cargo fmt --check` grün. Scoped Whitespacecheck grün.

Rootläufe verwenden offline/locked, höchstens2 Cargojobs und den vorübergehend
exklusiven Implementertarget `.codex-tmp/gen04_3-target`. Keine veränderten
Noise-/RNG-Fixtures, Registry-/Storageabhängigkeiten, Runtimekonfiguration oder
Generatoridentität im geprüften Snapshot. Toolchains entsprechen dem Brief.

Zusätzliche Reviewartefakte, SHA256:

| Artefakt | SHA256 |
| --- | --- |
| Neuer Javahelper `.codex-tmp/gen04_3-root-extra/CubicSplineOracle.java` | `a1b9e313b3f31331b74e7697acdd8a7d90ce17f3c878e042c08b87d837860b31` |
| Erweiterte Originalfixture | `8764aca589210f934cf1dbe36c97bd0f78d0e36729fe68ade9da494c76710e7c` |
| Privater Publicreader `tests/root_spline_extra.rs` im Reviewcheckout | `5ba16bd10ef1e9aaf47740bb36d5056147ebf2a90e852956ea8059f5318370d0` |

Logs relativ zu `.codex-tmp/`: `gen04_3-root-tests.log`,
`gen04_3-root-oracle.log`, `gen04_3-root-extra-oracle.log`,
`gen04_3-root-extra-tests.log`, `gen04_3-root-release.log`,
`gen04_3-root-clippy-red.log`, `gen04_3-root-clippy-green.log`,
`gen04_3-root-fmt.log`. Private Reviewprobes sind nicht Teil der Runtime.

## Verbleibende Consumerscope

Keine Densitydefinition-/Holder-/Noisesettingscompilation, kein RandomState-
Router, NoiseChunk, Aquifer, Surface/Biome oder aktiver Naturalgenerator in
diesem Ticket. Flat-v1 bleibt aktiv. Keine neue App-/Entity-/Mob-/Nether-/End-
Mechanik, Paper-Livedifferenzialabnahme, Ressourcenvergleich oder Survival-
Etappenfreigabe wird aus diesem numerischen Vertrag abgeleitet.
