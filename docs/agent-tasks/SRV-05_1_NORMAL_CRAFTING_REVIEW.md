# SRV-05.1 — unabhängige Abnahme des normalen Rezeptproducers

Ergebnis: **CLEAN**, geprüft durch Root am 2026-10-10. Der Autor
`feature_catalog` besitzt nur die Itemscrate und den Rezeptbrief; Root besitzt
diesen Review und den zusätzlichen öffentlichen Contracttest. Onlinecrafting,
Recipebook, Specials, Pluginrezepte und dauerhafte Inventartransaktionen sind
weiterhin Folgeconsumer, keine durch diesen Review aktivierten Funktionen.

Der geprüfte Autorenfreeze enthält26 Dateien und ist vollständig unverändert:
`.codex-tmp/srv05_1-author-freeze.sha256`, SHA256
`319a49d89f51ee36aa3479406fef2c996323440e73a80388e44409e30f687f0a`.
Die16 separaten Autorenartefakte sind in
`.codex-tmp/srv05_1-author-evidence.sha256` festgehalten, SHA256
`4e22b39fa18f4d15cb36a8aac3b09a20389c619b0c92e5192637ef085ca9f74b`.

## Geprüfter Vertrag

Root hat die gesamte neue Runtime, Cloneaccounting, Katalogvalidation,
Originalproducer und die Änderungen bestehender Itemmethoden geprüft. Der
lossless Persistencepreflight wird wiederverwendet; alte Codec-/Transferformen
bleiben unverändert. Kein neues Cargo-/Lockfile-/Dependencyverhalten.

Alle968 normalen Rezepte verwenden echte Originalingredientsets und feste
Resultate. Trimming/Offsets/Mirror und inner holes sind korrekt; das vollständige
bipartite Matching berücksichtigt belegte Positionen. Ein Count64 ersetzt
keine zweite Zutat. Ein vollständiger Plan verbraucht genau ein Item je
belegtem Slot, bewahrt überlebende Komponenten und weist sämtliche echten
Remainders separat aus. Keine ungeprüfte Remainderüberschreibung oder
Inputpatchübernahme in das feste Resultat. Present-air wird ausdrücklich
verweigert. Grid-/Planbudgets werden vor Resultkopien geprüft; das logische
Cloneaccounting behauptet keine RSS-Garantie.

Die normalen Vanilla-Methoden und angewandten Paper-Methoden/Patches wurden
für diese Semantik geprüft; ExactChoice und RecipeManagerpriorität werden
bewusst nicht aus dem geschlossenen Producer abgeleitet. Root hat alle52
Dateipins der19-Quellen-/Applied-/Patch-/Exacthelpermatrix unabhängig validiert.
Das ist Quellenprüfung und keine Live-Paper- oder Java-Pluginabnahme.

## Unabhängige Prüfungen

Root verwendete einen frischen `git archive` des integrierten Stands mit
ausschließlich den26 gehashten Autorendateien. Laufende World-/Storageänderungen
gingen nicht in diesen Prüfcheckout ein. Rust/Cargo1.99.0, Edition2021,
`--locked --offline`, Jobs2 und eigener Target:

- Alle84 Debugtests und3 Doctests bestanden unabhängig.
- Alle84 Releasetests,3 Doctests und3 zusätzliche Rootcontracttests bestanden.
- Die3 Rootcontracttests bestanden außerdem separat in Debug.
- Strict all-target Clippy, package Fmtcheck und Rustdoc mit `-D warnings`
  bestanden auch mit den zusätzlichen Contracttests.

Der zusätzliche PublicAPItest `tests/root_crafting_contract.rs` verwendet
konkrete, unabhängig aus Originalressourcen festgelegte Slotmasken:

- Sämtliche19683 3×3-Belegungen mit leer/Planks/Stick prüfen Sticks und
  Holzäxte, alle vier Axtplatzierungen/Mirrors, gemischte Holzarten und Counts.
- Sämtliche262144 3×3-Belegungen mit leer/Paper/Leather/Stick prüfen das
  shapeless Buchrezept, drei separate Paperpositionen und zusätzliche Items.
- Ein überlebender Craftplan bleibt nach Drop von Eingabe und Katalog gültig;
  doppelte NBTnamen, NaNpayload und signed zero bleiben bitweise erhalten.

Damit bestehen301510 zusätzliche explizite Matchmodellvergleiche, ergänzt
durch Planverbrauch und unveränderte Callerinputs. Der Roottest ist SHA256
`7a9f6e1e3b6c4d29a79e2a39955b15ab1463f4e0a790c3c109f9ef0c067de565`.

Root hat den tatsächlichen gepinnten Originalproducer aus einem frischen
Outputverzeichnis unter Python `-O` erneut ausgeführt. RawJSONL, Deskriptoren,
CaseTSV und vollständiges Manifest sind bytegleich mit beiden frischen
Autorenexporten. Alle21855 Original-JAR-Fälle bleiben enthalten; kein
gekopierter Matcher dient als Originalgroundtruth. Auch die8 Producer-/Hash-/
CodeSource-/Javaoptionnegativtests bestanden unabhängig unter `-O`.

Reexportpins:

- RawJSONL: `6110a3130294599a11d480e8eb0c3f74ff7854f208ece81514a22090fe1cbd7c`.
- TSV: `64f90f5b3cff75eb6885993f33cbcb6b8c9992e0f36a63f7a9ed55041fbd6649`.
- Deskriptoren: `fbdc704830a75bcf457457fbc5a54be67a9e6b560ab1eeb1cdf3275ad5b70f3c`.
- Manifest: `05c3bba8aacebc01af9fd58a2951c15c25fa714514574ce68b108e70a641d4a3`.

## Restliche Survivalarbeit

Das Ergebnis ist der normale Recipeproducer. DUR-04/05 führen die tatsächliche
Playerinventoryauthority und bestätigte Storagejobs ein; SRV-06/07 konsumieren
diese Pläne für normale Outputklicks/Shiftcraft/Recipebook. Weitere Serializer,
Specialkomponenten und Pluginrezepte folgen separat. Kein vollständiges
Survival, aktive natürliche Welt oder Endfortschritt wird hiermit abgenommen.
