# NBT-WRITE-01 — geprüfte Network-NBT-Ausgabe in reservierten Callerbuffer

Status: NBT-only-Producer implementiert; alle isolierten Abnahmeprüfungen grün,
abschließender unabhängiger Rootreview und Gitintegration ausstehend.
Vorgelagerte Abhängigkeit von NET-04.2a. Root gab Produktion gegen Entwurfs-SHA
`ac1847449441bf012098262944fcb38ddda6d82d72c889f94e970d5ab3659258` frei.
Der Autor besitzt ausschließlich `server/crates/ferrumc-nbt/**` und diesen Brief;
Items/Proto/Net/Session/Sim/App, Dependencies und Rootboards bleiben readonly.
Nur Root integriert Git. Eigene Evidence unter `.codex-tmp/nbt_write_01/`.

## Tatsächliche Lücke und enger Lösungsweg

`write_network_root` erstellt ein eigenes Vec; `write_payload`, `write_string`
und `mutf8::encode` schreiben dagegen bereits direkt in einen vorhandenen Vec.
Der Items-Slotencoder kopiert dieses temporäre NBT anschließend in sein Vec.
Für genau eine fallible Metadatareservierung fehlt deshalb ein öffentlicher
geprüfter NBTproducer. Ein zweiter NBT-/MUTF8encoder in Items wird vermieden.
Die vorhandenen NBTwriter bleiben die einzige Wiresemantikauthority.

```rust
pub fn network_root_size(tag: &NbtTag, limits: &NbtLimits) -> Result<usize>;

pub fn write_network_root_into(
    out: &mut Vec<u8>,
    tag: &NbtTag,
    limits: &NbtLimits,
) -> Result<usize>;
```

`network_root_size` gibt die vollständige Networkrootgröße zurück, ohne
Encoding, Allocation oder Clone. Root ausschließlich Compound: Typ10 plus
Compoundpayload inklusive End0; EmptyCompound exakt2 Bytes. Kein Filename-
Prefix, Slot-/Packetmaß oder Heapbudget. Neue APIs in `write.rs`, Reexport aus
`lib.rs`; ein privater Sizehelper in `write/size.rs` besitzt die strikte
borrowed Ressourcenprüfung. Keine neue Dependency oder allgemeine Buffer-API.

## Strikte Admission und genaue Fehlerreihenfolge

Die neuen APIs prüfen Rootkind vor Traversal und übernehmen vorhandene
`UnexpectedRootTag`, `DepthExceeded`, `ListTooLong`, `StringTooLong`,
`HeterogeneousList` und `MaxBytesExceeded` mit ihren numerischen Feldern.
`max_bytes` zählt hier die ganze geplante Networkroot, ohne fremden
Callerprefix. Strings zählen MUTF8 inklusive NUL/astralen Surrogatpaaren und
bleiben zusätzlich durch u16 begrenzt. Sequenzen sind durch
`min(max_list_len, i32::MAX)` begrenzt. Compounds/Lists zählen dieselbe Tiefe
wie der bestehende Writer; die neue Checked-API hat als ausdrückliche lokale
Safetygrenze zusätzlich höchstens512, somit effektive Tiefe
`min(limits.max_depth(), NbtLimits::DEFAULT_MAX_DEPTH)`. Keine tiefere
allgemeine NBTfreigabe, keine Veränderung der alten Writerlimits.

Ein festes Sizebudget läuft zuerst über Roottyp, dann Containerdepth,
Sequenzcount und deklarierbaren Prefix, dann borrowed Kinder/Keys/Payloads
in vorhandener Reihenfolge. Homogenität wird nach dem begrenzten Listcount
vor dem jeweiligen Childpayload geprüft. Jede addition/multiplication ist
checked; `EncodedSizeOverflow` ist eine neue numerische NbtErrorvariante.
Ein enger neuer privater `mutf8::bounded_encoded_len` wiederverwendet den
vorhandenen `encoded_char_len` und stoppt sofort über der Stringgrenze.
Vorhandene `encode`, `encoded_len` und alle MUTF8encodealgorithmen bleiben
unverändert. Es werden keine Strings oder Sequenzen zur Größenprobe erzeugt.

Diese neuen APIs sind strikter als die alten owned Writer: bisher erzwingen
diese nur Format-/Depthregeln, nicht alle Outputressourcencaps. Bestehende
owned Writer, ihre gültigen Bytes und ihre bestehende Fehlerreihenfolge
bleiben vollständig unverändert. Die neue strikte Fehlervorrangordnung ist
kein rückwirkender Vertrag für diese alten APIs.

## Capacity, direkte Ausgabe und Partialerrorvertrag

`write_network_root_into` führt zuerst die vollständige neue Size-/Modelprobe
aus. Erst danach vergleicht es die genaue Rootgröße mit
`out.capacity() - out.len()`. Ein zu kleiner Callerbuffer liefert die neue
`OutputCapacityExceeded { needed, remaining }`-Variante, ohne Reserveversuch,
Mutation oder Allocation. Ein ungültiges Model gewinnt deshalb auch bei
zu wenig Capacity. Der Caller muss den Buffer selbst fallible reservieren.

Bei ausreichender Capacity werden ausschließlich der vorhandene Roottyp-
Append und der unveränderte `write_payload(out, tag, 1, limits)` verwendet.
NBTstring- und MUTF8ausgabe bleiben vollständig im vorhandenen Writerpfad.
Der immutable Tag hat nach der Probe identische strukturelle Voraussetzungen;
die exakte vorgeprüfte Größe garantiert, dass keine Vecoperation wächst oder
reserviert. Kein Zwischenbuffer, keine Baum-/Component-/Stringkopie.
Ergebnis ist die Zahl tatsächlich angehängter Bytes, kein gesamter Callerlen.

Erfolgreiche Ausgabe erhält alle ursprünglichen Prefixbytes und die Capacity.
Jeder Fehler erhält Prefix, Len und Capacity. Präflight-/Capacityfehler
geschehen vor dem ersten Append; ein unerwarteter späterer Writerfehler
führt über `truncate(start)` den gesamten Append zurück. Eine defensive
`OutputSizeMismatch { expected, actual }`-Variante verweigert bei abweichender
Endlänge ebenfalls mit Rollback. Die Noallocationaussage beruht auf der
exakten vollständigen Preflight-/Writerprüfung; ein solcher Mismatch wäre
ein Implementierungsdefekt, keine zulässige partielle Ausgabe.

Keine `try_reserve`, `resize`, Shrink-/Boxkonvertierung oder eigene
Outputownership in der neuen Borrowed-API. Keine Token-/Revision-/Publication-
authority, kein Parser, allgemeiner Serializer oder Sanitizer.

## Konkrete Dateien, Preservation und ausgeführte Abnahme

Dateien: `src/write.rs` (nur neue APIs/private Sizeanbindung),
`src/write/size.rs`, `src/mutf8.rs` (nur neuer bounded Sizewrapper),
`src/error.rs` (drei additive numerische Varianten), `src/lib.rs`, README,
INVARIANTS, ein enges `tests/borrowed_network_root.rs` sowie dieser Brief.
Die gesamten ursprünglichen Writer-/MUTF8algorithmen bleiben byteidentisch
nach Entfernen ausschließlich der neuen API-/Sizeanbindungsblöcke.

| Ausgeführte Probe | Konkreter Nachweis |
| --- | --- |
| Ganze Größe/Bytes | Exact Size und direkte Ausgabe gegen unveränderte owned Writer und vorhandene NBTgoldens; alle12 Tagfamilien in Compoundroot, beide Floatbits, ordered/duplicate Keys, homogene/empty Lists, MUTF8/NUL/astral. Keine nachgeschriebene zweite Encoderreferenz. |
| Unabhängige Wirefixture | Bestehende NET-04.1-Paperfixture `metadata_item_components`: tatsächliche CustomData-Networkroot mit inneren255 unverändert wiederverwenden, ohne Protoimport oder neue Javaexportserie. CustomName-Stringroot gehört nicht zum Compoundrootvertrag. |
| Capacity N-1/N/N+1 | Tatsächliche freie Vec-Capacity gezielt einstellen; ein Byte zu wenig unverändert verweigern, exact und ein Byte mehr bytegleich anhängen. Prefix/Suffixposition, Pointer und Capacity vorher/nachher kontrollieren. Keine Annahme über Allocatorrundung. |
| Neue Ressourcencaps | Ganze Networkrootbytes N-1/N/N+1, Arrays/Lists, StringMUTF8caps und u16-Grenze, Depth512/513 bzw. kleinere konfigurierte Tiefe; Heterogeneous/rootkind und numeric Fehler. Source und Prefix bleiben unverändert. |
| Errorprecedence/Legacy | Ungültiges Model vor Capacityerror; neue caps dürfen owned Writerbytes/-Errors nicht rückwirkend ändern. Eine über neuer lokaler Bytecap liegende, ansonsten gültige Legacyfixture bleibt beim alten Writer accepted. |
| Ressourcen-/Regression | Vollständiger Callgraph mit bestehender direkter Emission; kein Globalallocator, neue Dependency oder Testframework. WholeNBT Debug/Release, strict Alltargetclippy/Fmt/NBT-Rustdoc und exakte Source-/Evidencefreezes. |

Originale Paper-/Vanilla-Wirebelege werden aus der bestehenden gepinnten
NET-04.1-Fixture übernommen; der neue Producer ist nur lokale NBTadmission.
Keine Survival-, Plugin-, Rootkind-Erweiterungs- oder Performanceparität daraus.
NET-04.2a bleibt ein separater Consumerauftrag nach Rootreview und Integration
dieses Producers; keine Consumeraktivierung in diesem Ticket.

## Endstand und prüfbare Evidence

Geprüfte isolierte Basis:
`18acd2be1a247328b2a1518a3ba067c0c5c33e71`. Das Archiv enthält ausschließlich
das vollständige NBT-Overlay und diesen Brief; andere Agentänderungen sind
nicht Teil dieser Abnahme. Der gemeinsame NBT-Quellstand ist byteidentisch
mit dem abschließend geprüften Archiv.

- WholeNBT Debug und Release: jeweils105 bestanden, keine fehlgeschlagenen
  oder ignorierten Tests. Aufteilung:75 Unittests,16 neue öffentliche
  Integrationstests,10 bestehende Fuzztests,1 Fuzzcorpustest,2 Roundtriptests
  und1 Doctest. Die beiden neuen privaten Tests prüfen arithmetische
  Sizeoverflow-/Sequenzprefixgrenzen ohne riesige Testallokationen.
- Alltarget-Clippy mit `-D warnings`, `cargo fmt -- --check` und NBT-Rustdoc
  mit `-D warnings`: bestanden. Vollständige Commands, CWD, Exitcodes und Logs
  unter `.codex-tmp/nbt_write_01/checks/`. Frühere Testhelfer-Stilhinweise
  bleiben in `checks-first-helper-lints/` und `checks-second-helper-lint/`
  erhalten; keine Lintausnahmen und keine Runtimekorrektur dafür.
- Alle10 unberührten ursprünglichen NBT-Dateien bleiben vollständig
  byteidentisch zur Basis. Entfernt man ausschließlich die neuen Additionen
  aus `write.rs`, `mutf8.rs` und `error.rs`, erhält man jeweils den vollständigen
  ursprünglichen Quelltext einschließlich Tests. Sämtliche Abhängigkeiten,
  Lockfiles und die vier Originalfixturedateien bleiben unverändert.
- Ressourcenbeleg: gelesener vollständiger Callgraph, vollständiges borrowed
  Sizepreflight, tatsächliche freie Capacity sowie Pointer-/Prefix-/Capacity-
  Prüfungen bei Erfolg und Fehlern. Keine Allocatorinstrumentierung, kein
  Laufzeit-/RSSbenchmark und kein Nachweis eines vollständigen Itemsanitizers.
- `source.freeze.json` bindet alle18 NBT-Dateien plus diesen Brief;
  `evidence.freeze.json` bindet Entwurfsnapshot, Pins, Preservation-/Callgraph-
  Audits und alle drei Checkserien. Beides liegt ausschließlich unter
  `.codex-tmp/nbt_write_01/`; Target- und Archivduplikate sind nicht mitgepinnt.
