# NET-04.1 — fehlende Item-Entity-Pakete für den echten Client

Status: vorbereitet, noch nicht vergeben. Vor DUR-05-Liveaktivierung nötig.
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
