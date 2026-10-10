# NET-04.0 — begrenzte Bytehülle ohne Längenpräfix

Status: **Implementierung und abgegrenzte lokale Prüfungen abgeschlossen;
exakter Freeze zur unabhängigen Root-Abnahme. Noch kein CLEAN/Integration.**
Autor: `/root/feature_catalog`. Exklusive Ownership:
`server/crates/ferrumc-proto-gen/**`, dieser Brief und eigener Scratch unter
`.codex-tmp/net04_0/`. Produktions-Protospec, Proto-/Codec-/Itemsruntime,
generierte Produktionsdateien, Cargo-/Lockfiles und Git bleiben read-only.
NET-04.1 beginnt erst nach separater .0-Abnahme und Integration.

## Problem und begrenztes Ergebnis

Die aktuelle Grammatik bietet `remaining_bytes` als `Vec<u8>` und
`prefixed_bytes(N)` als begrenzte, VarInt-längenpräfixierte Hülle. Die
Item-Entity-Metadaten werden dagegen ohne Byte-Längenpräfix geschrieben.
Die vorhandene unbounded Variante kann den ausdrücklich verlangten
Speichervertrag von NET-04.1 nicht darstellen.

Neuer Typ: `bounded_remaining_bytes(N)`. Er trägt ausschließlich die
verbleibenden Rohbytes eines Packetbodys, höchstens N Bytes. Er dekodiert
keine Metadataeinträge, Serializer, Slots, NBT oder Terminatoren. Leere
oder semantisch fehlerhafte Metadatabytes innerhalb N bleiben auf dieser
allgemeinen Hüllenebene opaque. Der spätere Builder muss seine eigene
Metadataadmission und tatsächliche Byte255-Terminierung nachweisen.

## Optionen und Empfehlung

Empfohlen wird `BoundedRemainingBytes(usize)` in der internen `WireType`-
Grammatik und `ferrumc_codec::BoundedBytes<N>` ausschließlich als begrenzter
Speichertyp. `new` und `as_slice` liefern bereits checked Construction und
unveränderlichen Zugriff. Dessen `read`/`write` dürfen hier niemals aufgerufen
werden, weil sie einen Wire-Längenpräfix erwarten bzw. schreiben. Der neue
Generatordispatch liest die Rawtail und schreibt `as_slice` unverändert.
Damit sind keine Dependencies oder Runtimeprimitivänderungen notwendig.

Alternative wäre ein eigener Unprefixed-Blobtyp im Codec oder in Proto.
Das erfordert ein separates Runtime-/Crate-Ticket und ist für den hier
begrenzten Vertrag unnötig. Eine vollständige Metadata-Wiregrammatik ist
ein größeres Folgeticket; eine neue Bytehülle beweist keine solche Parität.

## Konstante Cap und zulässige Position

Vorgeschlagener fester Grammatikbereich: **0..=2.097.152 Bytes**. Null ist
ein gültiges Empty-only-Bytefeld; Metadata selbst braucht später mindestens
seinen Terminator. Die Obergrenze entspricht der bereits in der Protospec
genutzten 2-MiB-Blobhülle und liegt sicher innerhalb der nichtnegativen
VarInt-Domäne, die `BoundedBytes::new` ebenfalls verlangt. Sie ist eine
FerrumC-Speicherpolitik, keine originale Metadata-Limitbehauptung.

N ist ein ASCII-Dezimalliteral; äußere/Argument-Whitespace ist erlaubt,
Ausdrücke, negative/positive Vorzeichen, Dezimalpunkt, Hexnotation,
fehlendes Argument, overflowende Zahlen und Werte über 2.097.152 verweigert
der Generator als `GenError::PacketsInvalid`. Kein stilles Clampen.

Der neue Typ ist nur als **direktes letztes Feld eines Packets** zulässig:

- kein nachfolgendes Feld und keine zweite bounded Tail;
- keine Verwendung in einem `[[struct]]`, selbst als dessen letztes Feld;
- keine Verschachtelung in `optional`, `prefixed_array` oder Kombinationen;
- deshalb auch keine indirekt in einem Composite versteckte Tail.

Bei einem Packet mit dem neuen Typ wird außerdem ein vorheriges Feld mit
explizitem `remaining_bytes` verweigert, einschließlich dessen Optional-/
Arrayverschachtelung in derselben Typexpression: die alte Rawtail würde
sonst bereits alle Bytes konsumieren. Dies ist das eng begrenzte Ergebnis
von Roots Mischfallreview; Schemas ohne neuen Typ bleiben unverändert.

Die Placementprüfung läuft bei vollständiger Specadmission vor Emission
oder Dateiausgabe und nennt Eigentümer/Feld im Fehler. Die bestehenden
`remaining_bytes`-/`prefixed_bytes`-Typen und Runtimeverträge werden hier
nicht geändert. Alte Rawtails in benannten Structs werden nicht rekursiv
auditiert; deren bestehende Admission und Konsumption bleiben erhalten.
Ein vorheriges solches Composite kann deshalb Bytes vor dem bounded Feld
konsumieren. Der neue bounded Typ selbst kann wegen der universellen
Structverweigerung nicht in einem Composite versteckt werden.

## Erzeugte APIs und Wireverhalten

Der Packetconstructor nimmt einen bereits gültigen `BoundedBytes<N>` und
bleibt wie bestehende Constructoren infallible. Der Getter gibt eine
unveränderliche Referenz auf diesen Speichertyp zurück; `as_slice` liefert
die Rawbytes. Keine öffentlichen Felder oder mutable Veczugriffe.

Reader:

1. `reader.remaining()` ermittelt die tatsächliche Rawtaillänge.
2. Bei Länge > N: `CodecError::BytesTooLong { length, max: N }` über
   `ProtoError::Codec`, vor `read_bytes`, Kopie oder Allokation.
3. Sonst sämtliche verbleibenden Bytes lesen, begrenzt kopieren und mit
   `BoundedBytes<N>::new` übernehmen. Kein Prefix und keine Payloadprüfung.

Das Caprefusal lässt den Reader an der **Feldeintrittsposition** unverändert.
Vorherige Packetfelder können bereits konsumiert sein; dies führt keinen
neuen transaktionalen Packetdecodervertrag ein. Nach erfolgreicher Capprüfung
können `read_bytes(remaining)` und Wrapperadmission bei derselben Länge nicht
wegen Shape/Cap fehlschlagen. OOM ist keine als recoverable Error behauptete
Runtimesemantik.

Writer schreibt ausschließlich `wire::write_raw(buf, value.as_slice())`.
Die begrenzte Tail selbst kann beim Encoding nicht fehlschlagen und fügt
keinen Count-/Längenpräfix oder Terminator hinzu. Packet-ID und vorherige
Felder behalten ihre vorhandene Reihenfolge. Etwaige Fehler vorheriger
NBT-/Compositefelder erhalten deren bisherigen Vertrag; es wird keine
pauschale Packetwriter-Rollbackgarantie eingeführt.

Truncation/VarIntoverflow in vorherigen synthetischen Skalaren bleiben
die typisierten existierenden Codecfehler. Eine gekürzte opaque Tail
innerhalb N ist kein semantisch erkennbarer Truncationfehler. Bytes nach
einem vermeintlichen Metadata-EOF sind auf dieser Ebene ebenfalls Teil
der Tail; ein pauschales Scannen auf 0xff wäre wegen beliebiger Payload-
Bytes falsch. Der spätere Metadata-Builder/Parser benötigt eigene Tests.

## Dateien und Verifikation

Geplante Änderungen ausschließlich im Generator:
`src/packets.rs` für Typ/Cap-/Placementadmission, `src/emit.rs` für Typ-/
Decode-/Encodeverwendung, `README.md` für den tatsächlichen Vertrag und
abgegrenzte Generatortests/Fixtures innerhalb derselben Crate. Keine
Produktions-Protospec- oder generierten Runtimeänderung in NET-04.0.

Sinnvolle Tests:

- Grammatik: 0, 1, Maximum; malformed Argumente, Maximum+1, usizeoverflow.
- Shape: final direkt gültig; vor anderem Feld, in Struct, optional,
  Array und geschachtelten Optional-/Arraykombinationen verweigert.
- Zweimal synthetisch generieren: vollständige Ausgabe bytegleich;
  `--check` auf dieser Ausgabe grün; invalid Spec schreibt keine Dateien.
- In eigener `git archive`-Kopie synthetischen Packettyp ergänzen, durch
  denselben echten Generator erzeugen und die entstandene Proto-Runtime
  kompilieren. Dort keine generierten Handedits und keine Abhängigkeiten.
- Kompilierte Wireprobe gegen eigenständig festgelegte Rawbytes: kein
  Längenpräfix, genaue Packet-ID-/Scalar-/Tailfolge, Getter/Construction,
  Empty-only, N und N+1, typed Caprefusal mit unverändertem Fieldreader,
  fixed-width Truncation, malformed/overflowende VarIntprefixe vor Tail.
- Opaqueboundary: interne 0xff-Bytes und Bytes danach unverändert erhalten;
  keine fälschlich behauptete Metadata-/Terminator-/Trailingadmission.
- Generator-Wholechecks, Alltargetclippy/Fmt/Rustdoc und deterministischer
  Check der unveränderten Produktionsspec. Exakter Filefreeze mit
  abgegrenzter unabhängiger Rootprüfung vor Integration.

Für dieses generische Format gibt es keine Java-Spiellogikformel nachzubilden.
Die tatsächliche Motivation wurde anhand der vollständig gelesenen
angewandten `ClientboundSetEntityDataPacket`-Quelle und
`SynchedEntityData.DataValue` geprüft: Entity-ID-VarInt, unprefixed Einträge
und Byte255-Ende. Die vollständige originale Packet-/Callee-/Featurematrix
und actual-JAR-/Prismarine-Wirefixtures gehören anschließend zu NET-04.1.

## Freigabegate und Folge

Root bestätigte vor Code diesen konkreten Cap-/Placement-/Atomicityvertrag
am Designbrief SHA256
`61b92ec881204fecc3d6965263ab20492ef044cd244b26148a31b87fb98905bd`.
Nach .0-CLEAN und Integration folgt .1 mit SetEntityData 0x5c, SetEntityMotion
0x5e und TakeItemEntity 0x75. .1 hält opaque Speicheradmission getrennt von
einem späteren Items-/Session-Builder und behauptet keine vollständige
Entitymetadata-, Sanitizer-, Plugin- oder Gameplayabnahme allein aus der Hülle.

## Implementierung und tatsächliche Ergebnisse

`WireType::BoundedRemainingBytes`, ASCII-Capadmission und direkte Finalfeld-
Prüfung sind implementiert. Der Emitter verwendet ausschließlich `new`/
`as_slice`: vor jeglichem Tailread/-copy wird `remaining > N` als typisiertes
`BytesTooLong` verweigert, beim Schreiben `wire::write_raw` aufgerufen. Root
hat zusätzlich den eng auf neue Typnutzung begrenzten sichtbaren Legacytail-
Mischfallfix freigegeben; keine rekursive Legacystructadmission eingeführt.

Der endgültige isolierte Stand stammt aus `git archive` von
`e7565c386b74af78503be1ff31bb7d0d9b956a5f` mit ausschließlich überlagertem
Generator. Vier synthetische Proben ergänzen nur dort die Spec: N=4 in Play,
N=0/1/2.097.152 in Login. Die Test-IDs 0x7ffc..0x7fff sind ausdrücklich keine
Minecraft-Packet-ID-Belege. Der echte `cargo xtask generate` erzeugte die
Runtime; ein zweiter Lauf war bytegleich, `--check` grün. Keine generierten
Handedits, keine Cargo-/Lockänderung und keine Produktionsspecänderung.

Abgegrenzte grüne Checks im frischen v3-Archiv:

- Generator: **27 Unit- und 2 Whole-Generator-Tests**, striktes Alltarget-
  Clippy, Fmt und Rustdoc uneingeschränkt mit `-D warnings`.
- Grammatik: 5 gültige Cap-/Whitespaceformen, 19 malformed/excessive Formen
  plus exakter usizeoverflow; 11 verbotene Placements; 5 Mixedtailrefusals
  und deren 5 unveränderte Legacy-only-Kontrollen; explizite Legacystructgrenze.
- Whole-Generator: vollständige Ausgabe zweimal bytegleich, wiederholter
  Write ohne Änderung, beide Checkmodi grün. 6 Invalidspecs werden in beiden
  Modi vor Output verweigert, ohne Verzeichniscreation oder Sentineloverwrite.
- Tatsächlich generierte Proto-Runtime: **58 Whole-Debugtests** (48 bestehende
  plus 10 unabhängige Rawbyteproben), finale 10 Proben Debug und Release;
  striktes Alltarget-Clippy und Fmt grün.
- Rawproben: Packet-ID/VarInt/i16/unprefixed Tail exakt, append ohne Prefix/
  Terminator, typed N+1 bei unverändertem Tailentryreader, checked Storage/
  Getter, Empty-only, Grenzen von 1 Byte und 2 MiB, vorherige Scalartruncation/
  überlange Continuation, vorhandene Fifth-byte-Lenience und opaque 0xff-
  Bytes mit beliebigen nachfolgenden Bytes/kurzen Tails.
- Produktions-`cargo xtask generate --check` grün; **34 read-only Dateien**
  SHA256-identisch zur Baseline, einschließlich Spec, generierter Runtime,
  Proto/Codec und Cargo-/Lockfiles. Auch Archivmanifest/-lock unverändert.

Die öffentliche Proto-Rustdocprüfung mit uneingeschränktem `-D warnings`
trifft einen **vorhandenen Baselinefehler** in `ferrumc-proto/src/lib.rs:7`:
der öffentliche Cratedoclink zeigt auf das private `wire`-Modul. Auch
`--document-private-items` verweigert Rust1.99 diesen Link. Die Datei ist
hashidentisch zur Baseline und außerhalb des .0-Schreibscopes. Eine zusätzliche
Proto-Dokumentationsprüfung ist mit genau
`-D warnings -A rustdoc::private_intra_doc_links` und
`--document-private-items` grün; das ist keine uneingeschränkte strikte
Proto-Rustdocabnahme. Generator-Rustdoc benötigt diese Ausnahme nicht.

Evidence liegt unter `.codex-tmp/net04_0/`: `baseline.json`, endgültiger
`runtime-archive-v3/`, getrennte `archive-target-v3/`/`production-target/`,
`logs-v3/`, `checks.json`, `runtime_probe.rs`, `synthetic-packets.toml`,
`generator-source.freeze.json`, `synthetic-output.freeze.json`,
`readonly-check.json` sowie abschließende Source-/Evidencefreezes. Erst Root
entscheidet anhand dieses unveränderten Stands über CLEAN und Integration.

Die gespeicherte Prüfungshistorie enthält bewusst auch reparierte eigene
Harnessfehler: anfänglich gemeinsam genutztes Target wiederverwendete xtasks
eingebetteten Produktionspfad; danach überschritt eine rein synthetische
Vier-Play-Packet-Dispatch die bestehende Clippy-Zeilengrenze. Der finale
Archivstand trennt Targets und verteilt die Proben auf Play/Login. Eine
Probeassertion wurde für den neuen `assert_is_empty`-Lint korrigiert und
mit der tatsächlichen Workspaceedition2021 formatiert. Die Fehlerversuche
begründen keine Produktionsdrift und sind keine ausgeblendeten Abnahmen.
