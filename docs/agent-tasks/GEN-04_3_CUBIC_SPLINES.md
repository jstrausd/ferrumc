# GEN-04.3 — Originale kubische Splines für Terrain-Density

Status: **zugewiesen an `item_transfer_primitives`; Design durch Root freigegeben,
in Umsetzung; noch keine Abnahme**. WG-004-Fremdreview ist CLEAN, GEN-04.2 wurde
als `09fd6657` integriert. Produktions-Crate ausschließlich ferrumc-world;
eigener Buildtarget `.codex-tmp/gen04_3-target`, kein zusätzlicher Agentslot.

## Ziel und Besitz

Originale Minecraft-1.21.8-Splines bilden die nächste Grundlage für die
Overworld-Densityrouter. Implementiere einen begrenzten, besessenen Vertrag
für konstante und verschachtelte Multipoint-Splines mit originaler f32-
Interpolation, Endpunkten, Ableitungen und Extrapolation. Keine neue
natürliche Welt allein durch dieses Primitive behaupten.

Besitz bei Vergabe: server/crates/ferrumc-world/** und dieses Taskdokument.
Keine Identity-/Generator-/Registry-/Sim-/App-/Storage-/Lock-/Gitänderungen.
Vor Code das konkrete Node-/Coordinate-/Builder-/Sampling-/Fehler-/Capdesign
an Root melden. Fehlende Density-/Registryconsumer als separate Abhängigkeit
benennen, keine handkopierte Terrainkonfiguration als Ersatz.

## Begrenzter Vertrag

- Immutable owned Splines, explizite typisierte Koordinatenbindung und
  verschachtelte Werte. Keine globale mutable Map, I/O oder Pluginereignisse.
  Daten werden vor Speicherung/Allocation so weit möglich geprüft.
- Originale Auswertungsreihenfolge, Float32-Rundung, Binärsuche und
  Endpunkt-/Derivativlogik erhalten. Keine algebraischen Umordnungen oder
  beabsichtigte FMA. Koordinaten-Callback und tatsächlicher Bedarf an
  verschachtelten Werten müssen einen klaren Reihenfolge-/Fehlervertrag haben.
- Explizite maximale Knotenzahl, Punkte pro Knoten, Tiefe und Arbeitsbudget;
  beliebig tiefe JSON-/Graphdaten oder zyklische Referenzen nicht zulassen.
  Caps sind zusätzliche FerrumC-Ressourcengrenzen, keine Vanilla-Defaults.
- Reine erfolgreiche Auswertung ohne Heapallocation. Wiederverwendung und
  Clone sind unabhängige Werte oder immutable Sharing mit dokumentiertem
  Ownershipvertrag. Unveränderter Zustand nach Construction-/Samplingfehler.
- Originale min/max-Diagnostik, soweit angeboten, vollständig lesen und
  portieren; Diagnose nicht als neue absolute Schranke für beliebige
  Koordinatenverträge ausgeben. Nicht implementierte Visitor-/Codec-/Router-
  Consumer ausdrücklich offen lassen.
- Kein unbounded Builder und keine generischen unchecked public Nodefelder.
  Malformed IDs, unbekannte Koordinaten, unsortierte/doppelte Stützstellen,
  nichtendliche Werte und Größen-/Tiefenüberschreitungen klassifizieren.

## Quellen und Abnahme

Minecraft Java1.21.8 / Protokoll772; Paper Build60 / Commit
29c8822d90899c89d2689338e81a98f690bcba12. Vollständige relevante
net.minecraft.util.CubicSpline samt Builder/Multipoint/Constant/Visitor und
ToFloatFunction sowie Mth-Binärsuche-/Lerp-Callees lesen. TerrainProvider-
und DensityFunctions-Splineconsumer mit relevanten Paperpatches lesen,
damit f32/f64-Konvertierungen und Koordinatenbounds korrekt abgegrenzt sind.

Unabhängiges Oracle ruft tatsächliche gepinnte Original-JAR-Methoden auf;
keine nachgeschriebene Interpolationsformel als alleiniger Vergleich.
Original-JAR-/Mappings-/Libraryhashes prüfen, frischen Probe-Bytecode
verwenden und reproduzierbaren Helper im eigenen Scope einchecken.

- Bitgenaue f32-Goldens für konstante, einfache und verschachtelte Splines,
  alle Intervalle, Endpunkte und angrenzende Floats, negative Werte,
  Signedzero und unterschiedliche Derivatives.
- Fehler-/Cap-/Depth-/Cycle-/Koordinatenfälle erhalten den Zustand;
  Construction kann keinen teilweise gültigen public Graph freigeben.
- Deterministische Samplingreihenfolge, gleicher Wert nach Clone und
  unabhängiger Node-/Koordinatenanfragefolge; Budget-/Grenzfehler prüfen.
- Bestehende Worldtests und beide Original-Noisegoldens erhalten;
  Debug-/Releasecontracts, strict all-targets Clippy und fmt.
- Vollständige Quellen-/Ownership-/Abweichungs-/Validierungsmatrix, exaktes
  Dateifreeze und Logs an Root; Root organisiert Fremdreview und Integration.

TerrainProviderprofile, Noise-/Densityregistry-Auflösung, NoiseRouter,
NoiseChunkinterpolation, Aquifers, Surface/Biome und tatsächliche natürliche
Chunkgenerierung bleiben nach diesem Untervertrag eigene Consumer.
