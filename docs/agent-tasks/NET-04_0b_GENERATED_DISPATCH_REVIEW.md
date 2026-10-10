# NET-04.0b / .0c — unabhängiger Review und Generatoradoption

Ergebnis: **CLEAN für die methodenspezifische Dispatch-Lintausnahme**.
Autor `.0b`: `feature_catalog`; unabhängiger Review und getrennte
Generatedadoption `.0c`: Root. Neue Entitypakete folgen in NET-04.1.

Die drei tatsächlichen neuen Packetarme überschreiten die bisherige
100-Zeilen-Grenze der generierten ClientboundPlayPacket-Decodefunktion.
Die Änderung ergänzt ausschließlich in `render_dispatch` ein begründetes
`allow(clippy::too_many_lines)` direkt an nichtleere generierte
`decode(id, reader)`-Dispatchmethoden. Andere Decoder, Encoder, Module,
handgeschriebener Code und sämtliche übrigen Lints behalten ihre Regeln.

Root hat alle neun Source- und43 Evidencepins unabhängig nachgerechnet,
ebenso35 read-only Produktionspins. Der Sourcefreeze hat SHA256
`0d1620a0b41e4f720833373bf28f9c0dcabeeda46cd9b2eb2c93e05308d27cc8`,
der Evidencefreeze
`46d860d4ef44ee2f4b46d2a289cc61f867039bcd6194ae50370305fb4f8ce5a3`.
Die Runtimeänderung umfasst drei Emitterzeilen; neun READMEzeilen erklären
die genaue Grenze. Spec, Proto und Cargo-/Lockfiles wurden vom Autor nicht
verändert.

Ein eigener frischer Rootarchivlauf auf
`4f7d50371a8a12560a9d93957a70ac009dc6b9bb` mit separatem Target besteht
29 Whole-Generatortests, striktes All-target-Clippy/Fmt und uneingeschränktes
Generatorrustdoc. Der echte xtask erzeugt genau neun lokale Attribute in
fünf Statedateien. Nach Entfernung ausschließlich des normalisierten
Attributs sind alle sechs generierten Dateien bytegleich zur Baseline.
Wiederholte Generierung und `--check` bestehen.

Die neu erzeugte unveränderte Produktionsspec besteht48 Prototests und
striktes All-target-Clippy. Ein ausschließlich im Rootarchiv ergänztes
Drei-Packet-Schema mit den tatsächlichen Namen/IDs/Shapen besteht zusätzlich
xtask/`--check`,48 Prototests, All-target-Clippy und Fmt. Es dient hier nur
der Größen-/Generierungsprüfung; tatsächliche Wire-/Metadata-/Motion-/Take-
Abnahme gehört weiterhin NET-04.1. Die Probe wird nicht in Produktion adoptiert.

Root adoptiert ausschließlich die Ausgabe der unveränderten Produktionsspec
mit dem tatsächlichen `cargo xtask generate`, als getrennten `.0c`-Commit.
Alle sechs Produktionsdateien müssen der unabhängig erzeugten Ausgabe exakt
entsprechen. `.0b` und `.0c` werden erst gemeinsam bytekonsistent gepusht;
kein Head mit unabgenommenem Generateddrift wird veröffentlicht.

Der25-Dateien-Rootfreeze unter `.codex-tmp/net04_0b-root-review/` hat SHA256
`cdefe6dbfa49c3665d3b4347f0f512d41e4be3e59eb5bb7cf4ec89fc13fe52de`.
Die bekannte unveränderte private Rustdoclink-Baseline in Proto wird durch
die lokale Dispatchausnahme nicht behoben; NET-04.1 besitzt diesen eigenen
Scope. Keine Gameplay-, Plugin-, Performance- oder Drop-in-Abnahme folgt
aus diesem Generatorpass. GitHub-CI wird für den gemeinsamen Head ausgelöst.
