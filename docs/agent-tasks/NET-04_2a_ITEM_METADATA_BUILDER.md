# NET-04.2a — begrenzter Itemstack-Metadatabuilder

Status: vorbereitet; Vergabe nach NET-04.1b-Freeze. Ein Autor besitzt
ausschließlich `server/crates/ferrumc-items/**` und diesen Brief. Proto, Net,
Session, Sim, App, Storage und Rootboards bleiben readonly. Keine neue
Dependency, kein Proto-/Netimport in die untergeordnete Items-Crate.

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

## Konkreter Entwurf vor Code

Autor liefert API-/Fehler-/Dateientwurf und tatsächlichen encoded-byte-
Preflight. Der vorhandene trusted `ItemStack::encode_slot` besitzt keine
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
