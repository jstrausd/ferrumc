# REG-BIOME-01 — unabhängige Daten-/Generatorabnahme

Ergebnis: **CLEAN**, Autor `item_transfer_primitives`, Reviewer Root.
Frisches eigenes Archiv von veröffentlichtem main `2a4a6cd4` mit ausschließlich
geprüftem Registry-/Brief-Overlay, getrennt von Proto-/Net-/Sim-WIP.

120 Autorenpins unabhängig geprüft,48 eigene Overlaydateien bytegleich in
das Archiv übernommen. Alle160 früheren GEN-05.4-Pins bleiben unverändert.
Autorfreeze `.codex-tmp/reg-biome-01-author-registry.freeze.json`, SHA256
`8e287e49352f60c68e5ad6c9fb5b29a21ecf7cba6c6f1755dfffae6ad4616433`.
Alle Checkout-/Archiv-/Evidenzpins nach Rootchecks nochmals unverändert.
Rootartefakte `.codex-tmp/reg-biome-01-root-review/`, eigener Runner
`.codex-tmp/reg-biome-01-root-review.py`.

WholeRegistry Debug/Release jeweils70 Tests und14 Doctests ohne Fehler oder
ignorierte Fälle. Strict Alltargetclippy und Rustdoc mit `-D warnings` ohne
Ausnahmen sowie isolierter vollständiger Workspacefmtcheck erfolgreich.
Sharedworkspace-Sim-WIP wird weder formatiert noch als Teil dieser Abnahme
ausgegeben. Cargoänderung ist ausschließlich eine Kommentarpräzisierung;
keine Runtime-/Builddependencyänderung.

Root hat neuen Generator/API-/Censusvisitor-/Test-/Verifiercode gelesen.
Alle67 Runtime-RawValues und ihre Syntaxstatistiken stimmen mit unveränderten
Originalexporten überein. Alle7598 Ordinale, sechs quantisierten Spans,
Offsets und aufgelösten Biomekeys exakt, keine Sortierung oder Deduplication.
Private lokale Biome-/Preset-IDs bleiben typgetrennt, ohne raw/default/
Serialisierungskonstruktoren. Lookup-/Borrow-/Iteratorpfade allokieren nicht.

Der build-only typed Censusvisitor prüft tatsächliche2/7593/5/4/6/2-
Containerformen, lehnt zusätzliche Childpayloads vor Materialisierung ab
und berechnet node/depth Charges vor ihren Childpfaden. Duplicate decoded
keys, Fremdreferenzen, Gaps, Floats/Overflow und konkrete Domain-/Bytegrenzen
besitzen meaningful Refusaltests. Die bestehende bounded JSONimplementierung
wird einmalig geteilt; geändert ist ausschließlich ihre private Sichtbarkeit,
keine Admissionsemantik oder alte Worldgenlimits.

Zusätzliche Rootprüfungen unter Python `-O`: kompletter Census-/7-Adoptionspin-
Verifier, vier Negativeguardtests sowie alle65+2 actual Original-JARressourcen.
Offizieller Bundle/Inner verifiziert; sämtliche Datagen/JARbytes, exakte
Decimalinhalte und Originalreceiptindizes stimmen überein. Der bestehende
unabhängige normale/-O-Census wird unverändert adoptiert; kein unnötiger
identischer Javaexport und kein nachgeschriebener Referenzselektor.

Alle vier alten Generatedtabellen gegen den eigenen frischen Root-Proto-
Baselinebuild unabhängig bytegleich. Die neue Generateddatei ist in allen
vier Rootmodi bytegleich:3671918Bytes, SHA256
`1ac42f8397ad7da9b8ee9191381a2f24f8ef6c5c90472081e46809aabffb0ff7`.
7598 native Rows benötigen höchstens850976 statische Bytes. Das ist keine
gemessene Ressourcendifferenz gegenüber einem gleich vollständigen Paper.

Der Producer liefert vollständige Ressourcendaten, keine Biomeauswahl,
RTreehistory, Endquelle, NETWORK-NBT-/dynamische Registryadmission oder
aktive Naturwelt. GEN-05.5, Surface/Carvers/Features/Strukturen und tatsächliche
Palette-/Appconsumer folgen getrennt. Flat-v1 und Survival S1–S6 bleiben offen.
