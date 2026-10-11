# NET-04.2a — unabhängige Abnahme der Itemmetadata

Status: **CLEAN**. Autor `feature_catalog`, Reviewer Root. Root hat den
vollständigen neuen Producer und alle17 Tests gelesen. Die abschließenden
Rustdoc-/VarInt-/Testhelferkorrekturen sind im geprüften Freeze enthalten.
Kein offener Befund bleibt im Items-only-Umfang.

Der Producer prüft zuerst das unveränderte lossless Modell, danach PresentAIR,
exakte ganze Wiregröße und die256KiB-Metadatapolicy. Erst dann reserviert er
einmal fallible den privaten Outputbuffer. VarInts verwenden den bisherigen
Codecwriter mit festem5-Byte-Scratch; typed NBT verwendet den geprüften direkten
NBTappend. Kein Item-/Componentclone, temporärer NBTbuffer, Normalize oder
Sortieren ist enthalten. CanonicalEmpty ist `080700ff`; PresentAIR verweigert
ausdrücklich, statt die vorhandene Modellrepräsentation zu verlieren.

Root erzeugte ein frisches vollständiges Archiv von
`65064bc7a54d60660263eae5c96c33ee5ebef28c` und übernahm ausschließlich die30
eingefrorenen Items-/Briefdateien. Debug und Release bestehen jeweils**120**
Tests (84 Unit,13 Borrowedaccounting,17 neue Metadata,3 Rootcrafting,3
Rustdoctests). Strict Alltargetclippy, Workspacefmt und Rustdoc mit
`-D warnings` bestehen ohne Ausnahmen.30 Source-/24 Evidencepins wurden
vor/nach geprüft;20 tatsächliche Paperquellen frisch gehasht.

1155 übrige Archivdateien und24 ganze alte Itemsdateien bleiben byteidentisch.
Nach Entfernen ausschließlich der neuen Additionen sind ganze lib-, README-
und INVARIANTSdateien originalgleich. Alle19 eingefrorenen NBTdateien, vier
Originalfixturedateien und der gemeinsame Codecwriter sind in Sharedtree und
Prüfarchiv unverändert. Keine Dependency-/Lockänderung gehört zum Ticket.

Die fünf vorhandenen ganzen Paper-/Prismarine-Metadatarows sind bytegleich:
Empty, Stone1/64, tatsächliche Komponenten und Removal. Die echte
CustomName-Stringroot wird über checked trusted opaque5 wiederverwendet;
RepairCost bleibt opaque16. Typed CustomData benutzt die shared NBTauthority.
Alle sieben Modellvarianten, ursprüngliche Patchorder, signed VarInts,
Mehrbyte-ItemIDs, NBTfamilien/-Bits/-Reihenfolge und MUTF8 werden außerdem
gegen den vollständig unveränderten bisherigen Slotencoder verglichen.

Negative Abnahme trennt PresentAIR, Registrycounts, IDs, Duplicate/Conflicts,
typed Values,64/65 Entries, ganze Metadata256KiB−1/N/+1, Modelpayloadcaps,
File-/Network-NBTgrößen und Tiefe512/513. Sourcebytes und ursprüngliche
Fehlerreihenfolge bleiben erhalten. Opaque Innerschemas und Paper-Sanitizer
werden ausdrücklich nicht neu validiert. Die lokalen encoded-byte-Policies
sind keine originalen Minecraftlimits oder RSSbudgets. Allocationfehler wurden
nicht injiziert; der vollständig gelesene Callgraph belegt die eine fallible
Reservation nach dem borrowed Preflight.

Freezes unter `.codex-tmp/net04_2a/`:

- Source: `143848f8ed22d59dcdd06bf37a24f4b6cee8dfc451693ad4501f02dc7c1db40c`.
- Evidence: `e42c49a8e9cbfc566123f2df400427183b4fff4d3dd12ff2962e28fa791469fc`.

Rootcommands, volle Logs und separates CLEANassessment stehen unter
`.codex-tmp/net04_2a-root-review/`. Der eingefrorene Autorbrief bewahrt den
Zustand vor der unabhängigen Abnahme. Seine erste Test-IDannahme und der
behobene Strict-Testhelferbefund sind erhalten; keine Lintausnahme wurde
eingeführt. Eine neue identische Originalexportserie war nicht erforderlich.

Der Output gewährt keine Entity-/Durability-/Publicationauthority. Bestätigte
Simfakten, Player-Wirebindung, Sessiontracking und produktive Itemadoption
folgen getrennt. Physik, Pickup, Merge, Survival S1–S6 und allgemeine
Java-/Paper-JAR-Kompatibilität bleiben offen; keine Performanceabnahme folgt.
