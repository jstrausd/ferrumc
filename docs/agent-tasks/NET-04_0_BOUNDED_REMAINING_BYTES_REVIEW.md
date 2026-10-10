# NET-04.0 — unabhängige Root-Abnahme

Ergebnis: **CLEAN für den definierten opaque bounded Rawtailvertrag**.
Basis `e7565c386b74af78503be1ff31bb7d0d9b956a5f`. Kein Metadata-/Itembuilder,
Paketsemantik-, Sanitizer-, Drop/Pickup- oder vollständiger Survivalpass.

Root hat den vollständigen Generatordiff und die Grammar-/Admission-/Emitter-
Tests gelesen. Der neue Typ verlangt ein direktes letztes Packetfeld und
ASCII-Dezimalcap0..=2.097.152. Der Reader prüft die gesamte übrige Länge vor
Read/Kopie/Allokation; der Writer schreibt nur `as_slice` ohne Prefix.
Sichtbare alte Rawtailausdrücke werden in neuen bounded-tail-Paketen verweigert.
Die ausdrücklich dokumentierte Legacystructgrenze ist kein rekursiver Audit.

Alle9 Sourcefreeze- und55 Evidencefreeze-Dateien wurden unabhängig nach
Größe/SHA geprüft; Autorenfreeze unverändert:
`1e8a76bdb59a7c5afa8e3d8a1b78c9ab37dbe9a39bdd848fbec1892d815c19bf`,
Evidence `7746944d0d4862205919eedea91f7dfa87e9c6eef773511b5e2d1345f4953a0b`.

Root erstellte eine frische `git archive`-Kopie mit ausschließlich eingefrorenem
Generatoroverlay.27 Unit- und2 Whole-Generatortests, strict Alltargetclippy/
Fmt/Rustdoc und Produktionsgeneratorcheck sind grün. Die unveränderte
Produktionsspec erzeugt bytegleiches Output; keine generierten Handedits.

Nur im Archiv erzeugte Root drei zusätzliche eigene Packettypen mit Caps7,0
und2.097.152 durch echten `cargo xtask generate`. Zweite Generation bytegleich,
Check grün. **6 eigene Rawwireproben** vergleichen unabhängig festgelegte
Bytes und Readerpositionen: signed/multibyte Entity-ID und BEi16, alle Tail-
Längen0..7, N+1-Refusal am Feldeintritt, Empty-only, Maximum/Maximum+1,
interne0xff und Bytes danach, unveränderte vorherige Scalarfehlersprache.
Die synthetischen IDs0x73/0x74 sind keine Minecraft-Packet-ID-Belege.

WholeProto Debug: **54 Tests**, einschließlich6 Rootproben; Rootproben Release
6/6; Proto-Alltargetclippy und Fmt grün. ProtoRustdoc hat den unveränderten
Baselinefehler des öffentlichen Links auf das private `wire`-Modul. Es wurde
nur mit dokumentierter Ausnahme `-A rustdoc::private_intra_doc_links` bei
weiterhin `-D warnings` geprüft; keine uneingeschränkte ProtoRustdocabnahme.
GeneratorRustdoc ist uneingeschränkt grün.

Artefakte `.codex-tmp/net04_0-root-review/`, genaues Checkmanifest
`results.json`. Rootreviewfreeze 21 Dateien, SHA256 `1280e1a4d686892f86a2b934be0bd3cd910f90fd6e50d33e5396996336ba641d`.
Der tatsächliche Autorenbrief liegt bei SHA256
`c22798de4d45f653f0c48ef96acc2f17c68994be1b41d0579fd5e3c1fbe5201c`.

Nach Integration folgen die getrennte Dispatchgeneratoranpassung und echte
SetEntityData-/SetEntityMotion-/TakeItemEntity-Pakete aus NET-04.1. Sim-/App-
Aktivierung, Itemtimer/Physik und Client-/Restartabnahme bleiben erforderlich.
