# GEN-05.7 — unabhängige Runtimeabnahme

Stand: 11.10.2026. **CLEAN_RUNTIME für den World-Temperaturproducer.**
Die komplette finale Übergabe entspricht bytegenau dem überarbeiteten Kandidaten
`53f29051`, den Root auf einem eigenen Archiv unabhängig geprüft hat.

`BuiltinBiomeTemperatures` enthält alle 65 Originalklimadefinitionen und fünf
feste Simplexobjekte. Die Abfrage benutzt den tatsächlichen BiomeResourceId,
die originalen f32-Rechenschritte, Frozen-Zweige und Temperatur-/Niederschlags-
Prädikate. Sie hält keine veränderliche Abfragehistorie oder Worldseedbindung.
Der private bounded JSONcursor validiert vollständige Quellen einschließlich
übersprungener Felder, decoded Duplicate-Aliase und malformed/trailing Input.
Vorhandener RNG-/Noisecode, GEN-05.6, Registry und Dependencies blieben unverändert.

Root hat sämtliche neue Produktion, Parser-/Fehler-APIs, vollständige
Differential-/Boundary-/Allokationstests und mechanische Import-/Messwerkzeuge
gelesen. Keine offenen Codefindings. Die finale Nachprüfung liegt unter
`.codex-tmp/gen05_7-root-runtime-review/final-assessment.json`; der unabhängige
Prüflauf unter `revised-final/checks-assessment.json`.

| Nachweis | Ergebnis |
| --- | --- |
| Quelle | 234 Dateien: 233 komplette Worlddateien und RuntimeDelivery; SHA-256 `53f2905165fb5f72958d991e28f3fa2573152d5790d9c611cb8ecd1490cdc2d4` |
| Autoren-Evidence | 43 Artefakte; SHA-256 `e063258efb3632a9a297b0d6205ebbe6104d072975ac7c1183a250e7f30dade8` |
| Exakte Basis | echtes `d3fca66c78931864fd07104fa212d944c1a6778e`-Archiv, alle 1224 Dateien unabhängig bytegeprüft |
| Isolation | nur ganzes World plus Delivery überlagert; 1028 Außendateien vor/nach unverändert |
| Whole-World Debug/Release | jeweils 285 bestanden, null fehlgeschlagen, ein absichtlich ignorierter Instrumentierungstest in neun Suites |
| Instrumentierung | der ignorierte Test zusätzlich explizit im kalten Debug-/Release-Prozess ausgeführt, inklusive erstem Compile |
| Strictchecks | Alltarget-Clippy `-D warnings`, Workspace-fmt und rustdoc `-D warnings` bestanden |

Root reproduzierte den mechanischen Import in ein eigenes frisches Verzeichnis:
alle 27 Dateien bytegleich. Die echte Originalreproduktion und ihr unabhängiges
Oraclereview wurden zuvor separat abgenommen; dieser Import behauptet keinen
neuen Javaaufruf. 119 Oracle-, 64 Design- und 757 tatsächliche Inputpins wurden
frisch geprüft. Autoren-Reimporte normal/`-O` und alle 26 Originalausgaben der
echten Rootreproduktion blieben bytegleich.

Die Runtime vergleicht alle 129588 Originaltemperaturzeilen je Modus,
352 direkte Noiseproben, 98304 Erweiterungsbeobachtungen, 65 Metadatenzeilen,
fünf Permutationen und reale 262/786/262 Draws samt finalen Legacyzuständen.
Frozen deckt 19296 modified-, 13024 outerfalse- und 448 innerfalse-Fälle ab.
Originale Labels, Reihenfolge und tatsächliche mutable/cache-Zustände bleiben
erhalten. Die 15 Paper-Constructoroffset- und 57 Vanilla-Publiccacheunterschiede
sind ausdrücklich diagnostiziert; Rust verwendet offsets=false und bildet
den Vanilla-Publiccache nicht nach. Allgemeine Paper-Offsetbitparität wird
damit nicht behauptet.

Roots eigener exakter Debug-/Release-Binarylauf beobachtete jeweils drei
erfolgreiche und drei verweigerte Compiles, 100 Clones sowie 388764 Abfragen.
COMPILE/CLONE/QUERY zeigten jeweils null Allokationsaufrufe, angeforderte Bytes
und Frees; die positive Kontrolle genau einen Aufruf, 4096 Byte und einen Free.
Der erste Tabellenaufbau liegt tatsächlich innerhalb der Messphase.
Native `size_of`-Erfassung: 2232 Byte retained und konservativ 570216 Byte
simultaner Scratch. Das 1-MiB-Stack-Konstruktionstest besteht. Beobachtete
Compile-Stackwrites: Debug425864, Release72632 Byte; Clone/Query bleiben innerhalb
der geschützten 16384-Byte-Messmarge. Stackwrites, Objektkapazitäten und
unbeschriebene Stackreservationen sind getrennte Angaben. Die externe
Testinstrumentierung gilt für die gepinnten Linux/glibc-Binaries, ohne unsafe
Rust, Produktionsglobal oder neue Cargo-Dependency.

Der alte `521d2e48`-Harness hatte einen ungemessenen ersten Compile. Er wurde
vor Abnahme ausdrücklich verworfen; seine Logs und der alte Manifest bleiben
als `SUPERSEDED_BEFORE_ACCEPTANCE` erhalten. Der gesamte revidierte Worldumfang
wurde frisch geprüft. Produktion, Fixtures und Originalreferenzen änderten sich
bei dieser Harnesskorrektur nicht.

Diese Abnahme aktiviert keine natürliche Welt, Surface-/Schnee-/Eis-/Fluid-
Mutation, Mobs, Survival oder Java-Pluginmatrix. Eine Gesamtserver-RSS-/TPS-/
Latenzverbesserung ist damit nicht gemessen. Der nächste verbundene Baustein
ist GEN-05.8 stored biome resolver; danach folgen originale Surface-Regeln und
der vollständige SurfaceSystem-Treiber.
