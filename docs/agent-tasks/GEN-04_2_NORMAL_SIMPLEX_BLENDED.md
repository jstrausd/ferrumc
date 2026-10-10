# GEN-04.2 — Originale Normal-, Simplex- und Blendednoise

Status: vorbereitet; Vergabe nach Abschluss des Simreviewfixes.
Produktions-Crate ausschließlich `ferrumc-world`.

## Ziel und Besitz

Die unabhängig geprüften GEN-04.1-Primitiven sind als `e754254f` integriert.
Die folgenden reinen Sampler sind Voraussetzung für Overworld-/Netherdensity
und Endinseln. Sie verändern noch keine aktive Welt oder Generatoridentität.
Besitz bei Vergabe: `server/crates/ferrumc-world/**` und dieses Dokument.
Keine Sim-/Storage-/App-/Core-/Config-/Generatoridentity-/Lockfileänderung;
keine neuen Dependencies ohne begründete Abstimmung und keine Gitoperationen.
Bestehende GEN-04.1-Fixtures bleiben unverändert; neue Referenzfälle erhalten
einen getrennten Oraclehelper, Fixture und Manifest.

## Vertrag

- `NormalNoise`: originale zwei Perlinstacks, moderne/explicit legacy-
  Initialisierung, Amplituden-/Spanwertfaktoren, originale zweite
  Koordinatenskalierung, maxValue und reines Sampling.
- `SimplexNoise`: originale Offsets/Permutation, originale 2D- und 3D-Samples,
  Gradienten-/Gleichheits-/Negativkoordinatenfälle und Parent-RNGverbrauch.
  Keine ungeprüfte Ersatzbibliothek und kein approximativer Endfallback.
- `BlendedNoise`: ursprüngliche min-/max-/main-noise stacks, Faktoren,
  acht-/sechzehn-Octaveiteration, clamped interpolation und bounds. Originale
  `withNewRandom`-/Konstruktionssemantik auf das owned Rust-RNGmodell abbilden.
- Nur tatsächlich implementierte Varianten exportieren; API und bewusste
  Eingabe-/Ressourcengrenzen vor Umsetzung kurz an Root melden. Keine
  unbegründete Annahme, vorhandene Perlingrenzen seien Vanilla-Defaults.
- Konfiguration vor RNGmutation/Allocation prüfen; bei vorhersagbaren Fehlern
  RNGzustand erhalten. Numerische Reihenfolge aus Original beibehalten.
  Nichtendliche/unsupported Eingaben klassifizieren, Sampling allokiert nicht,
  keine I/O-/Thread-/Storage-/Packetabhängigkeit.

## Originalreferenz und differenzielle Abnahme

Gepinnte vollständige Quellen unter `.codex-tmp/paper-1.21.8-reference/`,
MC1.21.8 / Protokoll772, Paper Build60 / Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Vollständige `NormalNoise`,
`SimplexNoise`, `BlendedNoise` und benötigte `Mth`-/Perlincallees samt relevanten
Patches lesen. Angewandte Consumer `DensityFunctions`/`NoiseRouterData`
betrachten, um End-/Netheranforderungen und noch fehlende Consumers zu erkennen.

Neue Java-Oracle ruft mittels Reflection die tatsächlichen obfuskierten
Klassen aus dem gepinnten offiziellen Original-JAR auf; keine nachimplementierte
Javaformel. Sicherheits-/Reproduktionsvertrag wie GEN-04.1: Bundle/inneres
JAR/mappings/libraries fest prüfen, keine abschaltbaren Assertions, frisches
temporäres Oraclebytecode, gepinnte Originalklassen zuerst im Classpath.
Große Targets/Exports unter `.codex-tmp` auf Disk, nicht im knappen `/tmp`.

- Bitgenaue Offset-/Permutation-/Sample-/Maxwert- und Parentconsumptiongoldens
  für beide RNGalgorithmen, mehrere Seeds einschließlich signed extrema,
  Octavegaps/Nullamplituden, negative/Border-/Zellgrenzkoordinaten.
- Simplex 2D/3D Gleichheitszweige, Normal aktuelle und legacy Konstruktoren,
  Blendednoise Clamps und originale Constructor-/Samplergrenzen abdecken.
- Fehlkonfiguration/NAN/infinity/zu große Größen, reine Samplingreihenfolge,
  Dokumentation zusätzlicher FerrumCcaps, Defaultfreie Konstruktion.
- Bestehende komplette Worldtests und neue öffentliche Contracttests;
  Originalgoldens im Debug und Release, strict all-targets Clippy und fmt.
- Frischer Originalreexport einschließlich Python `-O` bytegleich; Manifest
  mit Toolchain/Java/Hashes/Recordzahlen; keine pauschale Crossplatformzusage.
- Finaler Quellen-/Verhaltens-/Abweichungs-/Abnahmebericht und exaktes
  Dateifreeze-SHA256-Manifest an Root. Root organisiert Fremdreview und Git.

Gaussian, `WorldgenRandom`-Wrapper, Densityrouter, Klima-/Biomeauswahl,
Terrain/Features/Structures und vollständige Weltseedparität bleiben getrennte
Folgetickets. Das Ticket erzeugt keine natürliche Welt und keine neuen Mobs.
