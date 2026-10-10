# NET-04.1 — fehlende Item-Entity-Pakete für den echten Client

Status: Protoproducer implementiert und eigener isolierter Abnahme bestanden;
Rootreview/Netconsumer vor gemeinsamer Publikation ausständig.
NET-04.0b/.0c auf main f6eb7111ad11569fba51209f74f24b9321d0c66c.
Isolierter echter Paper-Referenzboot für Wirefixtures autorisiert.
Vor DUR-05-Liveaktivierung nötig.
Ein Autor besitzt ausschließlich die Protokollspec
`docs/protocol/1_21_8/packets.toml`, `server/crates/ferrumc-proto/**`, diesen
Brief und eigene Wirefixtures. Generierte Dateien ausschließlich durch
`cargo xtask generate` ändern. Ein erforderlicher Generatorumbau erhält ein
getrenntes vorheriges Ticket; kein Handedit oder stilles zweites Crateticket.

## Tatsächliche Lücke und Ergebnis

`SpawnEntity`, dessen `EntityVelocity`-Composite und `RemoveEntities` sind
vorhanden. Es fehlen dagegen die eigenständigen Clientboundpakete für
Metadata, laufende Bewegungsgeschwindigkeit und Pickupanimation. Der neue
DUR-04.1-Consumer kann deshalb noch keine vollständig sichtbare Item-Entity
an einen echten Client anbinden. Keine App-/Sim-/Session-/Items-/Storage-
Änderung in diesem Producer und keine Behauptung, der Wirevertrag allein
aktiviere Drop/Pickup.

Gepinnte 1.21.8-Spec, Protokoll772/DataVersion4440:

| Originalsymbol | Packet-ID | Felder in Wire-Reihenfolge |
| --- | --- | --- |
| ClientboundSetEntityDataPacket | 0x5c | VarInt Entity-ID, Metadataeinträge, Byte255 als Ende |
| ClientboundSetEntityMotionPacket | 0x5e | VarInt Entity-ID, drei big-endian i16 |
| ClientboundTakeItemEntityPacket | 0x75 | VarInt Itemwire-ID, Collectorwire-ID, ursprünglicher Pickupcount |

Namen nicht mit dem bestehenden `EntityVelocity`-Composite kollidieren lassen.
Packet-ID-/Feldbelege aus `fixtures/protocol/1_21_8/protocol.json` und den
tatsächlichen Original-Streamcodecs unabhängig prüfen. Die Metadata umfasst
Serializer-ID und deren vollständiges tatsächliches Feldformat; ein unbounded
`remaining_bytes`-Feld ersetzt keine geprüfte Metadataadmission. Der konkrete
Entwurf darf eine ausdrücklich opaque, bounded Wirehülle mit einem nachfolgenden
Items-/Session-Builder vorschlagen, muss dabei unverifiziertes semantisches
Decoding deutlich trennen und darf keine Parserparität beanspruchen.

## Originalreferenz und Producergrenze

Root hat die vollständigen angewandten drei Packetklassen gelesen unter
`.codex-tmp/paper-1.21.8-reference/paper-server/src/minecraft/java/net/minecraft/network/protocol/game/`.
Pin: Paper1.21.8 Build60, Commit `29c8822d90899c89d2689338e81a98f690bcba12`.
Autor ergänzt Vanilla-Basis, Callees, SynchedEntityData.DataValue,
EntityDataSerializers, ItemEntity.DATA_ITEM und Source-/Featurepatchmatrix.
Insbesondere die Item-Sanitizer-Session im Paper-Metadatapacker festhalten;
ein Itemwireencoder allein ist kein Sanitizer-/Plugineventnachweis.

Motionconstructor clampet jede tatsächliche Komponente auf ±3.9, multipliziert
mit8000.0 und castet zu Javaint; geschrieben werden i16. Die Wirehülle darf
bereits quantisierte Werte verwenden. Pose-/Physics-/Quantisierungconsumer
bleiben getrennt, sofern kein ausdrücklich geprüfter Producer hinzukommt.
Takecount ist ein VarInt und darf nicht stillschweigend als aktueller
Entitycount oder wirklich eingelagerter Teilcount umgedeutet werden.

Vor Code konkreten Spec-/API-/Bytecap-/Dateientwurf liefern. Keine neuen
Dependencies, globalen Caches, Net-/DB-/Simulationhandles oder handgeschriebenen
generierten Pakete. Fehlende Typunterstützung zuerst eng im Generator lösen,
danach dieses Runtime-/Spec-Ticket fortsetzen.

## Abnahme und nächste Verbraucher

Unabhängige Original-/Prismarine-Wirefixtures für positive/mehrbyteige/signed
VarInts, i16-Extrema und echte Itemstackmetadata. Exakte Packet-ID/Feld-/
Terminator-/Trailing-/Truncation-/Overflow-/Bytecap-N/N+1-Abnahme, passend zum
tatsächlich implementierten opaque oder semantischen Contract. Keine bloßen
Encoder-Decoderrundreisen oder kopierten Java-Wireformeln als Oracle.

Whole-Protochecks, strict Alltargetclippy/Fmt/Rustdoc, deterministischer
Generatorcheck, exakter Filefreeze und unabhängiger Review. Danach
NET-04.2/Session-Trackingbuilder und DUR-05-Appdriver mit tatsächlichen
Spawn→Metadata-/Motion-/Take→Remove-/Viewer-/Reconnect-/Restartfolgen.
Physik, automatischer Pickup, Merge/Despawn, vollständige Entitymetadata und
allgemeine Mob-/Dragon-/Paperkompatibilität bleiben konkrete Folgeconsumer.

## Konkreter Spec-/API-/Speicherentwurf

Ausschließlich drei neue Play/Clientbound-Definitionen:

| Rustsymbol | ID | Constructorfelder in Wire-Reihenfolge |
| --- | --- | --- |
| SetEntityData |0x5c| entity_id:varint, metadata_raw:bounded_remaining_bytes(263168) |
| SetEntityMotion |0x5e| entity_id:varint, velocity_x:i16, velocity_y:i16, velocity_z:i16 |
| TakeItemEntity |0x75| item_entity_id:varint, collector_entity_id:varint, pickup_count:varint |

SetEntityData nimmt einen bereits gültigen `BoundedBytes<263168>` entgegen;
`metadata_raw()` gibt ausschließlich eine unveränderliche Referenz zurück.
Die Tail enthält bei gültigen Originalpaketen Einträge einschließlich des
Byte255-Endes. Diese Wirehülle fügt kein Ende hinzu und prüft weder Ende noch
Einträge, Serializer, Slot, Itemkomponenten oder Nachlauf. Auch leere oder
semantisch fehlerhafte Tails innerhalb der Cap sind auf dieser Ebene zulässig.
Sie bleiben vollständig erhalten, einschließlich interner255-Bytes und Bytes
nach einem vermeintlichen Ende. Es wird kein Terminatorparser eingeführt.

263168 Bytes entsprechen der vorhandenen Items-Komponentenpolicy von256KiB
plus1KiB für begrenzten Slot-/Metadataoverhead. Gegenüber einer pauschalen
2MiB-Hülle ist damit die maximale einzelne Rawtailallokation deutlich enger.
Dies ist eine lokale FerrumC-Admissionpolicy für den geplanten einzelnen
Itemstackconsumer, kein originales Metadata-Limit und keine Zusage, beliebige
große Vanilla-/Pluginmetadaten zu akzeptieren. Ein späterer Builder muss seinen
vollständigen konkreten Overhead und Bytebedarf innerhalb dieser Cap prüfen;
mehrere Itemwerte oder allgemeine Entitymetadata benötigen eigene Admission.

Decoder verweigert N+1 vor Tailread/-copy/-allocation als typisiertes
CodecError::BytesTooLong und lässt den Reader an der Tail-Feldeintrittsposition.
Entity-ID kann bereits konsumiert sein. Getter/Writer verwenden die vorhandene
begrenzte Hülle unverändert; kein Längenpräfix. Motion nimmt bereits quantisierte
i16-Werte entgegen, inklusive des vollständigen i16-Wirebereichs; keine neue
Vec3-/Physicsquantisierung. Signed VarInts bleiben reine Wirewerte. Takecount
wird weder positiv geclampet noch als Rest-/Einlagerungsmenge interpretiert.

## Tatsächliche Quellenbefunde und Referenzfixtureentwurf

Basis und Applied der drei vollständigen Packetklassen sind gelesen. Motion
und Take sind in diesen Quellen unverändert. Metadata-Papersourcepatch startet
eine ItemObfuscationSession gemäß gebundener Konfiguration vor sämtlichen
DataValue.write-Aufrufen und schließt sie vor dem EOF-Marker. SynchedEntityData
schreibt U8-Index, VarInt-Serializer-ID und echten Serializercodec. Die reale
Serializerregistrierung setzt ITEM_STACK auf7. Die Originalaufnahme endet beim
U8-Marker255; unbekannte Serializer und abgeschnittene Werte verweigert dieser
semantische Originaldecoder. Unsere opaque Rawtail beansprucht dessen Vertrag
ausdrücklich nicht.

Basis/Applied EntityDataSerializers und SynchedEntityData sowie die vollständigen
ItemObfuscationSession, ItemObfuscationBinding, ItemComponentSanitizer und
OversizedItemComponentSanitizer sind gelesen. Papers ITEM_STACK-Serializer
reduziert innerhalb aktiver Obfuscation den Context auf OVERSIZED, damit Count
und sichtbare normale Komponenten erhalten bleiben. Oversized Container-/
Bundle-/Projectilekomponenten haben weiter eigene Sanitizer. Nichtleeres
ItemStack-Encoding führt über den tatsächlichen sanitizerabhängigen Count-/
Componentpatchpfad; dessen statischer Initializer benötigt echte Serverregistry.
Ein bloßer Registrybootstrap ist deshalb kein vollständiger Paper-Itemcodec-
Kontext und wird hier nicht dafür ausgegeben.

Der autorisierte Referenzexport bootet den gepinnten tatsächlichen Paper-Server
isoliert im eigenen Scratch mit eigener flacher Welt, lokalen Bindadresse/
Port, dokumentierten/gehashten Konfigurationen und kontrolliertem Shutdown.
Ein kleiner eigener JavaPlugin-Helper läuft nach Serverinitialisierung und
verwendet den echten MinecraftServer-/Registry-/GlobalConfiguration-Kontext.
Er ruft tatsächliche Packet-STREAM_CODECs und die tatsächliche GameProtocols-
Dispatchcodec auf, die auch originale Packet-IDs liefert. ItemEntity.DATA_ITEM
wird aus der echten Klasse bezogen. Keine nachgebauten Serializer, keine
übernommene Javaquantisierungsformel und keine Umgehung von Klasseninitializern.
Alle Original-Class-CodeSources, frisch gebauter Helperorigin und Artifact-/
Source-/Helper-/Inputpins werden vor/nach kontrolliert.

Prismarine-Referenz entsteht daneben mit der vorhandenen gepinnten
minecraft-protocol1.66.2/minecraft-data3.111.0-Toolchain gegen die vendorte
1.21.8-Protokollschemafixture. Die Fixtures erfassen positive/mehrbyteige/
signed VarInts, Original-Motionconstructor-Grenzen und codecdekodierte
i16-Extrema, alle drei Takefelder, Empty-/Primitive-/tatsächliche Itemstack-
Metadata samt normalen Komponenten und internen255-Bytes. Gegenläufige
Originaldecoderkontrollen dokumentieren Ende/Unknownserializer/Truncation/
Nachlauf, ohne daraus eine solche Admission für unsere opaque Tail abzuleiten.

## Geplante Produktionsdateien und finale Abnahme

Nach ausdrücklicher Freigabe nur Spec sowie durch tatsächlichen xtask erzeugtes
Play-Modul, Proto-README/INVARIANTS und sinnvolle Prototests/Wirefixtures samt
Reexporthelper/Provenance in derselben Crate. Keine Generator-, Registry-, Items-,
App-, Sim-, Storage-, Dependency- oder Lockfileänderung.

Exakte Original-/Prismarinebytevergleiche an direkten APIs und Playdispatch;
keine bloßen Rustroundtrips. Constructor/Getter/Append-/unprefixed Tailvertrag,
N/N+1 und typed Readerposition, scalar Truncation und bestehende malformed-
VarIntklassifikation. Opaque Tailgrenzen bewusst separat: Empty, fehlendes Ende,
unbekannter Serializer, interne255 und Nachlauf bleiben innerhalb N erhalten.
Whole-Proto Debug/Release und strict Alltarget-Clippy/Fmt/Rustdoc; unveränderte
vorhandene APIs/Fixtures, bytegleiche wiederholte Generation und --check.
Der autorisierte public→private-wire Rustdoclink wird in normalen Pfadtext
korrigiert; Rustdoc wird strikt ohne Baselineausnahme geprüft.
Exakter Source-/Evidencefreeze und unabhängige Rootprüfung vor Integration.

## Getrennte Net-Integration NET-04.1b

`ferrumc-net/src/play/priority.rs` matcht `ClientboundPlayPacket` zweimal
vollständig. Die drei neuen Varianten benötigen dort echte Priority-/
Criticalityklassifizierung und Writer-/Backpressure-Sequenzabnahme in einem
getrennten Net-Ticket. Dieser Producer friert ausschließlich Proto ein und
ändert Net nicht. .1 und .1b werden gemeinsam publiziert; eine Whole-Workspace-
Kompilierbarkeit ist vor diesem getrennten Consumer nicht zugesagt.

## Bestandene Producerabnahme und exakter Reviewumfang

Drei Specdefinitionen durch echte xtaskgeneration übernommen. 54 Whole-Proto-
Tests jeweils Debug/Release (12 Unit, 6 neue unabhängige Wire-/Policytests,
6 Disconnect, 30 vorhandene). Strict Alltargetclippy/Fmt/Rustdoc ohne Ausnahme;
public→private-wire-Link in normalen Pfadtext korrigiert. Zwei wiederholte
Generationen über alle sechs Dateien bytegleich, --check grün. Keine
Generator-/Dependency-/Lockfileänderung.

Finales Original normal und Python -O: genuine Paperboot,121 Artifact- und42
Sourcepins vorher/nachher, identische frische Helperklasse/-JAR, Konfigurations-
inputs, Originalobservations nach ausschließlich Helperpfadnormalisierung und
30 Packetfixtures. Exakter TSV-SHA256 für Paper und separat semantisch
konstruierte Prismarinebytes:
`66b71e900d5e81fd42874ace58cb60a9ea309fec23158797d95d06e53ad753de`.
Originalsanitizer-Level tatsächlich OVERSIZED; DATA_ITEM Index8/Serializer7.
Postbootserver.properties-Datumscomment und Spigot-YAML-Mapreihenfolge
unterscheiden sich; beide Originalconfigs/Diffs sind gehasht aufbewahrt,
keine bytegleiche Postconfigbehauptung. Keine Packet-/Semantiknormalisierung.

Die finale Codeprüfung verwendet eine separate veröffentlichte main-Kopie
2a4a6cd4 mit exakt überlagerten Proto-/Specbytes und eigenem Cargo-Target,
weil gleichzeitig fremde Registry-Biome-WIPdateien im gemeinsamen Workspace
stehen. Diese abhängige Publishedbaseline ist separat gepinnt und unverändert
geblieben. Der vorhandene gemeinsame Registry-OUT_DIR-Zustand und spätere
Biome-Buildscript-Clippylints wurden weder im Producer repariert noch mit
Ausnahmen unterdrückt. Keine Whole-Workspace-Abnahme vor .1b behauptet.

Reviewartefakte ausschließlich `.codex-tmp/net04_1/`: `source.freeze.json`
(owned Spec/Proto/Brief), `evidence.freeze.json`, `published-baseline-pins.json`,
`isolated-checks/verification.json`, echte `paper-final-normal-pinned/` und
`paper-final-optimized/` Provenances sowie `prismarine-final/`.

Rawtailgrenzen sind lokale Admission, kein Java-Metadataparitätsnachweis.
Die Items256KiB stammen aus untrusted/persistence Policy; ein arbitrary
trusted Slot hat damit noch kein vollständiges Preflight. Motion bekommt
bereits quantisierte Werte; Takecount bleibt ursprünglicher Wirewert.
Writerordering/mandatory Refusal folgen in NET-04.1b; Slotsanitizerbuilder,
Session/App, Event-, Physics-/Gameplay-/Performanceabnahme bleiben getrennt.
