# NBT-WRITE-01 — unabhängige Abnahme der direkten Network-NBT-Ausgabe

Status: **CLEAN**. Autor `feature_catalog`, Reviewer Root. Die neue Größenprobe
und der Appendproducer wurden vollständig gelesen; Root prüfte außerdem den
bisherigen Writer-/MUTF8pfad, alle16 neuen öffentlichen Tests und beide privaten
Arithmetiktests. Kein offener Befund bleibt im geprüften NBT-only-Umfang.

`network_root_size` prüft borrowed Compoundroots, vollständige Wiregröße,
Sequenz-/Stringgrenzen und die effektive Containertiefe bis512.
`write_network_root_into` prüft anschließend die tatsächlich freie Capacity
und schreibt mit den vorhandenen NBT-/MUTF8algorithmen direkt in den
Callerbuffer. Keine eigene Reservation, Baumkopie oder temporäre Ausgabe;
Modelrefusals haben Vorrang vor Capacityrefusals. Prefix, Len und Capacity
bleiben bei Refusal erhalten. Die bisherigen owned Writer behalten ihre
historischen Limits, Bytes und Fehlerreihenfolge.

Root erzeugte ein frisches Archiv von
`f753e31220fef4d18db1c7d689da00d17629f867` und übernahm ausschließlich die19
eingefrorenen NBT-/Briefdateien. Debug und Release bestehen jeweils105 Tests
(75+16+10+1+2+1), striktes Alltargetclippy, Workspacefmt und Rustdoc mit
`-D warnings` ebenfalls ohne Ausnahmen. Alle19 Source-,27 Evidence- und vier
Originalfixturepins wurden vor/nach geprüft.1161 übrige Archivdateien und zehn
ganze alte NBTdateien sind unverändert. Nach Entfernen ausschließlich der
neuen Additionsblöcke sind die vollständigen ursprünglichen Writer-, MUTF8-
und Errorquellen bytegleich. Keine Dependency- oder Lockänderung gehört dazu.

Die tatsächliche vorhandene Paper-CustomData-Networkroot wurde bytegleich
wiederverwendet, einschließlich innerer255. Weitere Tests decken alle12
Tagfamilien, Reihenfolge/duplicate Keys, Floatbits, NUL/astrale MUTF8zeichen,
die tatsächliche freie Capacity N−1/N/N+1, konfigurierte Byte-/Count-/String-
und Depthgrenzen sowie Legacyrefusals ab. CustomName-Stringroots gehören nicht
zu dieser Compoundroot-API. Eine neue identische Originalexportserie war
für diesen Producer nicht erforderlich.

Sourcefreeze `.codex-tmp/nbt_write_01/source.freeze.json`:
`f5eeafb0bc6fe344861f18aeef97767af01dc9b59aa694a4d978469778f6900a`.
Evidencefreeze:
`f86567df03dccf0704e1a8b807bcabfe1c8cd1a87e8e1f9088e05f3c5e3f8a04`.
Rootcommands/Logs und separates CLEANassessment stehen unter
`.codex-tmp/nbt_write_01-root-review/`. Der eingefrorene Autorbrief hält den
Zustand vor dieser unabhängigen Abnahme fest.

Der Producer ist eine Voraussetzung für NET-04.2a. Itemsadoption,
Sessiontracking, aktive ItemEntities und allgemeine Java-/Paperkompatibilität
bleiben getrennte Arbeit. Diese Abnahme belegt keine vollständig spielbare
Survivalwelt oder gemessene TPS-/RSSverbesserung.
