# NET-04.2a — begrenzter Itemstack-Metadatabuilder

Status: Items-only implementiert; eigene fünf strikte Abschlussprüfungen grün,
unabhängige Rootabnahme ausstehend. Produktionsfreigabe gegen vollständig
gelesenen Entwurf-SHA
`76967bc062d865e8c1fd49b7fc4f04ea91900b3c1fae5eceeb6c166e278edd40`.
Die separat geprüfte Producerabhängigkeit
[NBT-WRITE-01](NBT-WRITE-01_BORROWED_NETWORK_ROOT.md) ist mit `579bed62`
integriert und hier readonly gepinnt. NET-04.1b/1c und ITEM-SIZE-01 bleiben
readonly. Der Autor besitzt ausschließlich `server/crates/ferrumc-items/**`
und diesen Brief. Proto, Net, Session, Sim, App, Storage und Rootboards bleiben
readonly. Keine neue Dependency, kein Proto-/Netimport in Items.

## Tatsächlicher Consumerbedarf

NET-04.1 stellt die opaque 263168-Byte-Hülle bereit; deren Constructor prüft
keinen Serializer oder Itemstack. Der neue Builder erzeugt innerhalb der
Items-Crate genau einen vollständigen DATA_ITEM-Eintrag mit wirklichem Slot
und Ende. Session baut daraus später SetEntityData mit echter Wire-ID;
unveröffentlichte oder nicht besessene Simpostimages erhalten hierdurch keine
Publicationauthority. Die Aufgabe aktiviert noch keine Item-Entities.

Referenz Minecraft1.21.8/772/4440, PaperBuild60/Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Tatsächlich gelesene Basis/Applied
SynchedEntityData.DataValue, EntityDataSerializers.ITEM_STACK, ItemEntity.DATA_ITEM,
ItemStack- und DataComponentPatchcodecs sowie Papers Sanitizerbindung verwenden.
Originalboot aus NET-04.1 belegt DATA_ITEM Index8, Serializer7 und OVERSIZED-
Kontext. Der Builder muss genaue mögliche Semantikgrenzen des vorhandenen
Itemmodels benennen; opaque trusted Componentbytes beweisen keine eigene
originale Componentadmission oder vollständige Sanitizerparität.

## Implementierter Vertragsentwurf

Der freigegebene API-/Fehler-/Dateientwurf und tatsächliche encoded-byte-
Preflight sind umgesetzt. Der vorhandene trusted `ItemStack::encode_slot` besitzt keine
umfassende gemeinsame Größenadmission. Eine nachträgliche Prüfung nach
unbounded Encoding reicht nicht. Lossless Modelvalidation und exakte komplette
Slot-/Metadata-Wiregröße werden vor Allocation geprüft, einschließlich typed
NBT, VarInts, Componentanzahlen, Removes und Index/Serializer/Endeoverhead.
Persistencegröße ist nicht pauschal identisch zur Slotwiregröße.

Keine Stack-/Componentclones, kein untrusted Normalizer, Clamping, stripping,
Sortieren oder generische leere Ersatzmetadata bei Refusal. Der vorhandene
Count-/Registryvertrag bleibt erhalten. Ein Checked-/Boundedoutput enthält
genau den einen Eintrag und das tatsächliche Endbyte, ohne Packet-/Lengthprefix.
Leerer Itemstack ist ein ausdrücklicher Slotfall; keine Phantomitemerzeugung.
Positive lokale Größenpolicy innerhalb der Proto263168-Byte-Hülle konkret
ableiten; Allocation ist fallible und checked. Lokale Policies sind keine
originalen Minecraft-/Pluginlimits.

Falls gemeinsame private Wirepreflighthelpers erforderlich sind, eng in
derselben Crate lösen und tatsächliche alte Slotconsumer bytegleich halten.
Keine allgemeine Item-/Komponenten-/Sanitizerneuentwicklung in diesem Ticket.
Ungeklärte Originalsemantik wird als Unsupported mit eigener Folgeabhängigkeit
sichtbar, nicht als geprüfte JAR-/Gameplayparität ausgewiesen.

## Meaningful Abnahme

Die bereits geprüften echten NET-04.1-Itemmetadatafixtures und deren Pin
`66b71e900d5e81fd42874ace58cb60a9ea309fec23158797d95d06e53ad753de` wiederverwenden:
EMPTY, stone1/64, tatsächliche CustomData/CustomName/RepairCost mit internen255
und Componentremove, soweit vom bestehenden Modell tatsächlich unterstützt.
Vollständiger unveränderter Originalslot inklusive Termination und ursprünglicher
Componentreihenfolge. Keine neue identische Javaexportserie ohne Evidenzlücke.

Fehlerfälle für tatsächlichen Count-/ID-/Patch-/NBTvertrag und konkrete
Byte-/Entry-/Depthgrenzen N/N-1 prüfen, insbesondere große trusted typed NBT
und complete metadataoverhead. Refusals müssen vor Outputallocation liegen
und den Input unverändert erhalten. Bestehende Slot-, Persistence-, Transfer-
und Rezeptverträge bestehen weiterhin. WholeItems Debug/Release, strict
Alltargetclippy/Fmt/Rustdoc, exakter Freeze und unabhängiger Rootreview.

NET-04.2b besitzt anschließend Sessionviewertracking und gemeinsame Lane/
Refusalordnung. DUR-05 aktiviert den Storage-/Sim-/Appdriver nach seinen
wirklichen Inventar-/Block-/Command-/Durabilityabhängigkeiten. Physik, Merge,
automatischer Pickup, Mobmetadata, Javaevents und Survival S1–S6 bleiben offen.

## Konkrete öffentliche Producer, Errors und Output

```rust
pub const MAX_ITEM_ENTITY_METADATA_BYTES: usize = MAX_COMPONENTS_TOTAL_BYTES;

pub fn encode_item_entity_metadata(
    stack: &ItemStack,
) -> Result<ItemEntityMetadata, ItemMetadataError>;

impl ItemEntityMetadata {
    pub fn as_bytes(&self) -> &[u8];
    pub fn into_bytes(self) -> Vec<u8>;
}
```

Das private Outputfeld ist ein Vec mit genau einem DATA_ITEM-Eintrag:
`08 07 <vollständiger trusted Slot> ff`, ohne Entity-ID-/Packet-/Frameprefix.
Keine öffentliche Konstruktion oder mutable Bytes; `into_bytes` verschiebt
den bereits geprüften Vec ohne Clone/Boxkonvertierung. Keine automatisch
abgeleitete Clone-API. Dieser Typ garantiert die deklarierte begrenzte
trusted Modellrepräsentation, keine originale Komponenteninnerschemaadmission
oder gameplaytaugliche Stackability. Der spätere obere Layer baut den echten
SetEntityData-Packet mit der aktuellen besessenen Wire-ID.

`ItemMetadataError` ist non-exhaustive: `InvalidStack(ItemPersistenceError)`,
`UnsupportedPresentAir`, `MetadataTooLarge { size, max }`, `SizeOverflow`,
`AllocationFailed { requested, source: TryReserveError }`, `Nbt(NbtError)` und
defensive `OutputSizeMismatch { expected, actual }`. Rustdoc auf Typen, APIs
und Fehlerfeldern. Keine Normalisierung, generischer Ersatz oder Stringerror.

Zuerst der bestehende unveränderte lossless Modelpreflight; anschließend
PresentAIR als ausdrücklich Unsupported, bevor irgendein Output entsteht.
Paper `isEmpty()` behandelt AIR als leer, das bestehende FerrumCmodell bewahrt
PresentAIR dagegen auch in FCIS/Transfer. Eine stillschweigende Umwandlung
hier würde Repräsentation verlieren. Ungültiger Count/Patch gewinnt vor AIR-
Unsupported. Canonical Empty ist ein echter Slot0 und ergibt `080700ff`
mit4 Bytes, keine leere Metadataliste `ff` und kein Phantomitem.

## Exakte Wiregröße und positive lokale Policy

Ein neuer privater `metadata.rs`-Preflight rechnet nach der Modelprüfung:

```text
Empty: metadata_size = 1(index8) + 1(serializer7) + 1(slot0) + 1(end255) = 4
Present:
  slot_size = varint_len(count) + varint_len(item_id)
            + varint_len(added_count) + varint_len(removed_count)
            + sum(varint_len(component_id) + actual_wire_payload_size)
            + sum(varint_len(removal_id))
  metadata_size = 3 + slot_size
```

Count/IDs/Entrycounts bleiben originale skalare Werte; keine VarInt-
Längenprefixe vor trusted Payloads. MaxStackSize als tatsächlicher VarInt,
Damage/MaxDamage als signed VarInt mit1..5 Bytes, Unbreakable ohne Payload,
Opaque exakt raw.len. Beide typed NBTvarianten benutzen den separat geprüften
`ferrumc_nbt::network_root_size` mit lokalen8192-Byte-/Sequence-/Stringlimits
und Tiefe512. WholeModel-FCISadmission bleibt zusätzlich unverändert: dadurch
ist zulässiges typed Network-NBT derzeit höchstens8190 Bytes, weil sein
File-NBT-Prefix zwei Bytes mehr trägt. Persistedsize wird niemals als Slotmaß
übernommen. Alle Sizeadditionen und usize/i32-Konvertierungen sind checked.

Die **ganze Metadata** wird lokal auf256KiB begrenzt, inklusive Index,
Serializer, Komponenten-IDs und Endbyte. Das liegt unter der bereits
abgenommenen Proto263168-Byte-Hülle und benötigt keinen Proto-/Netimport.
Ein FCISgültiger Stack kann deshalb wegen Metadataenvelope ausdrücklich
`MetadataTooLarge` liefern. Diese strengere positive Ressourcenpolicy ist
kein Minecraft-/Pluginlimit und kein RSS-/Aggregatebudget.

Alle Prüfungen geschehen vor Outputallocation. Erst danach:
`Vec::new()` und genau ein fallibles `try_reserve_exact(metadata_size)`.
Bei Reservefehler bleibt das Source unverändert und kein Output wird
veröffentlicht. Das Vec wird nicht im Voraus mit Nullen gefüllt; jeder normale
Append prüft seinen Platz gegen die exakte geplante Endlänge, bevor kopiert
wird. VarInts verwenden den bestehenden Codecwriter über einen festen
5-Byte-Stackbuffer, keine zweite VarIntimplementierung und kein Heapbuffer.

Typed NBT wird direkt über den neuen geprüften
`ferrumc_nbt::write_network_root_into(&mut out, tag, &limits)` angehängt.
Diese API reserviert nicht selbst und besitzt weiterhin die alleinige
NBT-/MUTF8wiresemantik. Ihre vollständige erneute NBTprobe benutzt nur
borrowed Daten. Der Metadatawriter vergleicht die angehängte NBTgröße und
abschließend die ganze Endlänge gegen die vorgeprüften Maße; jeder Fehler
verwirft ausschließlich den privaten Output. Keine alten `encode_slot`-Calls
mit ihren ZwischenVecs und kein privater NBT-/MUTF8zweiter Encoder in Items.

## Originalreferenz und tatsächliche semantische Grenze

Basis/Applied vollständig relevante ItemStack-Optionalcodec- und
DataComponentPatch-Encode/Decode-/encodeComponentmethoden, SynchedEntityData-
DataValue.write und tatsächliches ClientboundSetEntityDataPacket.pack gelesen.
ItemEntity.DATA_ITEM/defineSynchedData und Papers EntityDataSerializers-
OVERSIZED_ITEM_CODEC sowie Sanitizerbinding/-session/-component/-oversized-
Klassen und relevante Sourcepatches gelesen. Narrow Featuresearch findet
keinen zusätzlichen Serializer-/Itemobfuscationhunk; Activation-Range-
SynchedEntityData-Konstruktorkontext ist kein neuer Wirevertrag.

Papers Itemmetadata downgradet einen aktiven ALL-Kontext auf OVERSIZED und
erhält damit Count/CustomName; Nested Container/Bundle/Projectile-Sanitizer
bleiben eigene Originalzweige. Diese Rustrepräsentation implementiert keine
Originalprototype-/Textcomponent-/Nesteditem-/Sanitizeradmission. Root hat
den bestehenden checked **trusted opaque** Passthrough in diesem Ticket
ausdrücklich erlaubt; dessen bytes werden weder geparst noch normalisiert.
Auch opaque modeled IDs bleiben ihre ausdrücklich trusted Repräsentation.
Unverifizierte Innerschemas können daher keine allgemeine Client-/Plugin-
Kompatibilitätsbehauptung begründen; solche Admission ist Folgearbeit.

Die tatsächliche Originalfixture `metadata_item_components` enthält CustomName
als NBT-Stringroot. Typed CustomName und die vorhandene FCISauthority akzeptieren
dagegen Compoundroots. Die Fixture wird exakt mit typed CustomData sowie
checked opaque CustomName5 und RepairCost16 repräsentiert. Kein Rootkindumbau,
keine Ersatz-Compoundfixture und keine neue Sanitizerparitätsbehauptung.
Die tatsächlichen internal255 bleiben Payloadbytes; nur das wirkliche finale
Metadata-Endbyte terminiert die Datenliste.

## Enge Dateien und überprüfbarer Consumerstand

Die freigegebene Itemsproduktion umfasst nur neuen `src/metadata.rs`, Reexports
in `src/lib.rs`, README/INVARIANTS, enges `tests/item_entity_metadata.rs` und
diesen Brief. `persistence.rs`, `persistence/nbt_size.rs`, `stack.rs`, `wire.rs`,
Transfer/Crafting und alle alten privaten Algorithmen bleiben byteidentisch.
NBT-WRITE-01 wurde als getrenntes NBT-only-Ticket vor dieser Adoption
unabhängig abgenommen; sein vollständiger finaler 19-Dateien-Freeze bleibt
unverändert. Keine neue Dependency.

Die fünf wirklichen Originalrows EMPTY/stone1/stone64/components/removed
werden als ganze Metadata byteidentisch mit dem bestehenden Paper-/Prismarine-
Fixturepin verglichen. Lokale zusätzliche Tests vergleichen alle7 Varianten,
signed VarIntbreiten, typed/opaque Formen, removal/addition Order, nested
NBTtagfamilien, NUL/astral, duplicate Keys und float bits zusätzlich gegen den
unveränderten trusted Slotencoder. PresentAIR ist ausdrückliche Unsupported-
Abnahme; invalidCount/IDs/duplicate/conflict/typedValues und NBTgrenzen
verweigern vor Output und bewahren die Source.

Für ganze Metadata-N-1/N/N+1 werden32 checked opaque Einträge verwendet,
deren lokale raw-Längen die genaue Endlänge256KiB-1/256KiB/256KiB+1 ergeben,
während alle vorhandenen Modelpayloadcaps noch eingehalten sind. Das prüft
wirkliche Overheadadmission; diese synthetischen trusted Payloads sind keine
Originalcomponent-Decodeabnahme. Entries64/65, NBTfile8192/8193 bzw.
Network8190, totalModel256KiB/+1 und Tiefe512/513 separat prüfen. Der Source-
Callgraph belegt vor Allocation keine Encode-/Cloneprobe und danach nur die
einzige eigene fallible Reservation plus geprüften direkten NBTappend.

WholeItems Debug/Release, strict Alltargetclippy/Fmt/Items-Rustdoc und exact
Items-/Brief-Source-/Evidencefreezes sowie unabhängiger Rootreview.
NBTproducerfreeze als readonly Baseline pinnen. Kein Sessionviewer, keine
Itementity-Aktivierung oder Ownership-/Durabilitypublication aus diesem Builder.


## Eigene abgeschlossene Abnahme und Grenzen

Exaktes isoliertes Archiv mit Baseline
`65064bc7a54d60660263eae5c96c33ee5ebef28c`, darüber ausschließlich eigene
Itemsdateien und dieser Brief. Alle fünf finalen Checks Exit0: WholeItems
Debug/Release jeweils **120** Tests (84 Unit,13 vorhandene Borrowedaccounting,
17 neue Itemmetadata,3 vorhandene Rootcrafting,3 Rustdoctests), Alltargetclippy
`-D warnings`, Fmtcheck und Items-Rustdoc `-D warnings`. Locked Dependencies,
Jobs2; genaue Commands/Cwd/Exits/Laufzeiten und volle Logs im Evidencefreeze.

Die tatsächlichen fünf gepinnten Paper-/Prismarine-Rows stimmen als komplette
Metadata bytegleich. Zusätzlich alle7 Varianten, ursprüngliche Patchorder,
signed VarIntgoldens, Item-ID128/895/1173, NBTtagfamilien/duplicateKeys/NUL/astral/
Floatbits, Modellrefusals, presentAIR, Entries64/65, Wholemetadata256KiB-1/N/+1,
Modelpayload256KiB/+1, FileNBT8191/8192/8193 und Tiefe512/513 geprüft.
Synthetische trusted Opaquepayloads prüfen Größen-/Passthroughverträge; daraus
folgt keine originale Componentdecode-/Sanitizeradmission.

24 ganze alte Itemsdateien einschließlich aller bisherigen Algorithmen und
Tests bleiben byteidentisch. Der Originalinhalt von lib/README/INVARIANTS ist
nach Entfernung ausschließlich eigener Additionen bytegleich. Alle Baseline-
Archivdateien außerhalb Items und diesem Brief, vier Originalfixtures, der
shared Codecwriter, Dependencies/Lockfiles und der finale 19-Dateien-NBTfreeze
bleiben unverändert. Source-/Evidencefreezes werden zur Rootprüfung übergeben;
kein Gitcommit/-push durch diesen Autor.

Die erste Strictserie enthielt nur einen Clippyfehler im neuen Testhelfer
(unnötiges owned Errorargument); originale Logs sind erhalten. Der Helfer
borrowt nun den Expectederror ohne globale lintallow. Die erste gezielte
Testserie korrigierte eine Fixtureannahme: netherite_block ist Item-ID94,
während der Mehrbytefall jetzt echtes Item128 verwendet. Keine dieser
Korrekturen erweitert die Runtime-/Scopeverträge.

Der Sourcecallgraph belegt borrowed Preflight vor genau einer falliblen
Outputreservation, festen5Byte-VarIntscratch und geprüften direkten NBTappend.
Keine Allocatorfehlerinjektion, RSS-/Performancemessung oder Heapbudgetabnahme
wird behauptet. Sessionconsumer und tatsächliche besessene Simpostimages
benötigen weiterhin eigene Freigabe; Item-Entities, Physik, Pickup, Merge,
Survival und Java-/Paper-Pluginvollständigkeit sind hier nicht aktiviert.
