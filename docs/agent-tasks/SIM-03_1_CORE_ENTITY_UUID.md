# SIM-03.1 — Dauerhafte Entity-UUID getrennt von Wire-ID

Status: Implementierung und isolierte Abnahme abgeschlossen, bereit für
unabhängige Prüfung und Integration. Stand: 10.10.2026.
Produktions-Crate: ausschließlich `ferrumc-core`.

## Ziel und Besitz

`EntityId` ist bereits eine signed i32-Protokoll-ID. Sie darf nicht als
dauerhafte Mob-/Itemidentität verwendet werden. Ein eigener `EntityUuid`
Newtype bildet die im Original gespeicherte Entity-UUID ab, ohne den
bestehenden Wiretyp oder seine Verbraucher umzudeuten. Die aktuelle Storage-
`EntityKey` verwendet noch `EntityId`; ihre getrennte Migration und die
Entitysimulation bleiben anschließende Tickets.

Besitz bei Vergabe: `server/crates/ferrumc-core/**` und dieses Dokument.
Keine Storage-/Sim-/App-/Protokoll-/Lockfileänderung und keine Gitoperationen.
Vor Umsetzung AGENTS, CLAUDE, Core-INVARIANTS und die relevanten Architektur-
und Referenzdokumente lesen.

## Vertrag

- Opaquer immutable UUID-Newtype mit `from_uuid`, `as_uuid`, verlustfreien
  16-Byte-Konstruktor-/Zugriffsmethoden, `Display`, `FromStr`, Equality, Hash,
  Order, Clone/Copy und optionaler Serde wie die bestehenden Coretypen.
  Vorhandenes `uuid` verwenden; keine zusätzliche Dependency.
- Alle 128 Bit bleiben erhalten, auch nil, all-one und nicht-v4 Altbestand.
  Die generische Identität behauptet keine UUID-Version-/Uniquenessvalidation.
  Keine automatische Konvertierung aus `EntityId`, `PlayerId`, Integern oder
  unbekanntem Storagebestand. UUID↔Newtype darf ausdrücklich verlustfrei sein.
- Keine versteckte Zufalls-/Zeitwahl, keine globale Allocation und kein
  IDcounter. Erzeugung und Duplikat-/Lifetime-/Scopeprüfung gehören später
  dem exklusiven Entityowner. Der Typ trägt keine Welt-/Dimensions-/Wire-ID.
- Dokumentierte Byteordnung folgt `uuid` canonical/network byte sequence;
  kein behaupteter NBT-/VarInt-/Storagecodec. Die originale UUID-Intarray-
  Darstellung erhält später ihren eigenen Codecvertrag.
- `EntityId` API und bestehende Verbraucher bleiben kompatibel. Dokumentation
  grenzt den Laufzeitbezug von dauerhafter UUID und einem generationalen
  internen Entityhandle ab.

## Originalreferenz

Minecraft 1.21.8 / Protokoll 772, Paper Build60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`, angewendete vollständige Quellen
unter `.codex-tmp/paper-1.21.8-reference/`.
Komplette Entitykonstruktion, `getId/setId`, `saveWithoutId`, `load`,
`getUUID/setUUID` einschließlich einschlägiger Callees/Source-/Featurepatches
lesen. `Mth.createInsecureUUID` und `UUIDUtil` vollständig lesen, insbesondere
die Grenze zwischen Erzeugung und gespeicherter Identität. Persistentes
Entitymanagement und Duplicateuuidverhalten werden nur als offene Consumer-
Anforderung dokumentiert; der Newtype allein implementiert sie nicht.

## Abnahme und Handoff

- Öffentliche roundtrip Tests für asymmetrische bekannte Byte-/UUIDgoldens,
  nil/all-one, signed-high-bit Werte, Display/Parsing und Hash/Ordering.
- Fehlparsefälle leer/ungültig/truncated/zu lang; dokumentierte vom bestehenden
  `uuid` akzeptierte Schreibweisen bleiben exakt. Keine unnötige neue
  Parsergrammatik neben `uuid`.
- Feature-enabled Serde roundtrip und malformed JSON; `--no-default-features`
  funktioniert ohne Serde. Compile-fail Beispiele verhindern Verwechslung
  mit Wire-ID und PlayerId an einer typed EntityUuid-Grenze.
- Bestehende Coretests, Doctests, strikte all-targets Clippy und fmt grün;
  keine pauschalen Lintsuppressions oder Produktions-unwraps.
- Quelle/Verhalten/Abweichung/Events/Ownership/Abnahme im finalen Taskdoc.
  Keine Gameplayevents in diesem wertbasierten Producer. Genaues Freeze-
  SHA256-Manifest aller geänderten Dateien und Logpfade an Root senden.

Root organisiert unabhängiges Review und Integration. Entityspawn, Tracking,
Speicherung, sichere Itemtransfers und sämtliche Mobs bleiben nach diesem
Producer offen.

## Implementiertes API

`ferrumc_core::EntityUuid` ist ein opaquer immutable `uuid::Uuid`-Newtype mit
`Debug`, `Clone`, `Copy`, `PartialEq/Eq`, `Hash`, `PartialOrd/Ord`.

| API | Verhalten |
| --- | --- |
| `const from_uuid(Uuid) -> EntityUuid` | Explizit supplied UUID vollständig übernehmen. |
| `const as_uuid(self) -> Uuid` | Unveränderte UUID zurückgeben. |
| `const from_bytes([u8; 16]) -> EntityUuid` | Canonical/network UUID-Reihenfolge; keine Endianfeldswap-/Version-/Variantanpassung. |
| `const to_bytes(self) -> [u8; 16]` | Alle Bytes in derselben Reihenfolge zurückgeben. |
| `From<Uuid>`, `From<EntityUuid> for Uuid` | Explizite, verlustfreie UUID/Newtype-Konvertierung. |
| `FromStr<Err = uuid::Error>` | Unverändert `Uuid::parse_str`; Fehler nicht als Default/Nil behandeln. |
| `Display` | Lowercase hyphenated UUIDtext. |
| Feature `serde` | Transparente Serde des vorhandenen UUIDtyps. |

Kein `Default`, keine random-/Zeit-/Countererzeugung, keine Conversion aus
`EntityId`, `PlayerId` oder Integern. Alle nil/all-one/non-v4/non-RFC-Bits werden
bewahrt. Equality/Hash und unsigned canonical-byte Ordering folgen dem bestehenden
`uuid`-Typ; das ist keine Zusage über Java-Collectionordering.

Die Textgrammatik ist ausdrücklich die des vorhandenen `uuid` 1.23.4: 32 Hexzeichen
(simple), 36 Bytes mit den üblichen Hyphens, `{hyphenated}` oder
`urn:uuid:hyphenated`; Hexzeichen dürfen Uppercase sein, der URNprefix ist
lowercase. Es gibt kein Trimming, NUL-/Whitespaceentfernung oder Auffüllen
verkürzter Felder. Diese Rustgrammatik behauptet keine Identität mit Javas
`UUID.fromString` oder dem Original-NBT-Codec. Serde stellt ebenfalls keinen
NBT-/Packet-/Storagevertrag dar.

## Gelesene Originalmethoden und Verhaltensmatrix

Pin: Minecraft Java 1.21.8, Protokoll 772; Paper Build 60, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Die folgenden vollständigen Methoden
und einschlägigen Callees wurden in den angewendeten Quellen unter
`.codex-tmp/paper-1.21.8-reference/paper-server/src/minecraft/java/` gelesen.
Die Basis-/Source-/Featurepatchkontexte wurden getrennt geprüft. Das
Quellenlesen ersetzt keine Gameplay-/Paper-Differenzialabnahme.

| Quelle / vollständiges Symbol | Originalverhalten und Grenze dieses Tickets |
| --- | --- |
| `net/minecraft/world/entity/Entity.java`, Feldinitialisierung und Konstruktor, `getId/setId` | Laufende signed int-ID aus `ENTITY_COUNTER`, getrennte UUIDinitialisierung; der Konstruktor setzt Entitydaten/Level/SynchedData. Newtype übernimmt weder Counter noch Konstruktor/Lifecycle. |
| `Entity.saveWithoutId` beide Overloads, `load` | Normaler Save enthält `UUID` über `UUIDUtil.CODEC`; Paper setzt das bei der Rawserialization hinter `includeAll`. Load ersetzt UUID/String nur bei erfolgreich gelesenem UUIDwert; ohne UUID bleibt die Konstruktoridentität erhalten. Fehlende Identität wird vom neuen Typ nicht erraten. Kein Save-/Loadconsumer geändert. |
| `Entity.getUUID/setUUID/getStringUUID` | UUID und cached canonical Text bleiben zusammen; keine Version- oder Uniquenessprüfung in diesen Accessors. Der immutable Rustwert hat keinen Stringcache oder mutable Entitysetter. |
| `net/minecraft/util/Mth.java`, beide `createInsecureUUID`-Overloads | Zwei Randomlongs, explizite v4-/Variantstempel. Das ist Erzeugung, keine Validierung aller gespeicherten UUIDs. Keine UUIDerzeugung in diesem Ticket. |
| `net/minecraft/core/UUIDUtil.java`, komplette Klasse | Originalcodec verwendet genau vier signed Ints mit vollständigen 128 Bits; `uuidToByteArray` schreibt MSB/LSB Big-Endian. String-/Authlib-/Lenientcodec und Offlineplayerderivation sind getrennte Verträge. Der neue Bytezugriff bewahrt canonical bytes, implementiert aber keinen dieser Codecs. |
| `Entity.java.patch`, relevante vollständige UUID/save/load-Konktexte | Rawserialization, `includeAll`, WorldUUID, OriginWorld und Bukkitdaten gehören zum Entitysaveconsumer; separate Weltidentität wird nicht in `EntityUuid` hineingemischt. |
| `Entity` Sharedrandomklasse und Source-/Moonrisepatch | Paper verwendet Sharedrandom und ersetzt dessen RNG; der ursprüngliche Erzeugungsconsumer bleibt offen und wird hier nicht pauschal übernommen. |
| Moonrise `EntityLookup.addEntity/removeEntity`, `ChunkStatusTasks.checkDupeUUID` | Separate ID-/UUIDindizes, owner-/thread-/removal-Voraussetzungen und konfigurationsabhängige DuplicateUUIDbehandlung. Nur als offene Consumeranforderung geprüft; keine Indizes, Duplikatbehandlung oder Ownershipclaims durch den Werttyp. |

Keine Gameplayevents, Cancellation, Pakete, Threads, Queues oder Storage-I/O
werden durch Konstruktor/Bytezugriff aktiviert. UUIDtext-/Serdeparsefehler
werden zurückgegeben; der Typ ersetzt ungültige Eingaben nicht durch Nil.
Entityspawn, Duplicateuuidregeln, generational handles, Scopezuordnung,
Shardtransfer, Tracking und sichere durable/retry Storageconsumer bleiben
explizit offen. Die bestehende `EntityId`-API und deren Verbraucher bleiben
kompatibel; ihre Dokumentation benennt nun die dauerhafte UUID als separaten
Typ.

## Bestandene isolierte Abnahme

Target: `.codex-tmp/gen04-target` (vorhandener isolierter Targetcache dieses
Agents), keine Workspacechecks oder Gitoperationen. Aus `server/`:

```sh
CARGO_TARGET_DIR=../.codex-tmp/gen04-target cargo test --offline --locked -p ferrumc-core
CARGO_TARGET_DIR=../.codex-tmp/gen04-target cargo test --offline --locked -p ferrumc-core --no-default-features
CARGO_TARGET_DIR=../.codex-tmp/gen04-target cargo clippy --offline --locked -p ferrumc-core --all-targets -- -D warnings
CARGO_TARGET_DIR=../.codex-tmp/gen04-target cargo clippy --offline --locked -p ferrumc-core --all-targets --no-default-features -- -D warnings
cargo fmt -p ferrumc-core -- --check
```

- Defaultfeatures: 35 bestehende Unittests, sechs neue öffentliche
  Entityuuid-Contracttests und vier Compile-fail-Doctests erfolgreich.
- Ohne Defaultfeatures: 29 bestehende Unittests, vier öffentliche Contracttests
  und dieselben vier Compile-fail-Doctests erfolgreich; kein Serde erforderlich.
- Beide strikten all-targets Clippychecks und die Coreformatprüfung erfolgreich.
- Asymmetrische feste Byte-/Textgoldens, nil/all-one und signed-high-bit Werte
  prüfen Erhaltung und Byteordnung, auch ohne v4-/Variantstempel. Konstante
  Konstruktor-/Accessorbenutzung wird beim Kompilieren geprüft.
- Bestehende UUIDschreibweisen und malformed/empty/truncated/oversized/Unicode/
  Whitespace/NUL-Eingaben geprüft; Parserfehler entsprechen `uuid`.
- Feature-enabled JSONroundtrips sind transparent, including nicht-v4-Werte;
  falsche Typen, malformed/truncated Texte, kaputte JSON und trailing JSON
  verweigern ohne Defaulting. Equality/Hashsets und unsigned ordering bewahren
  Unterschiede. Wire-/Player-IDs passen weder direkt noch über `.into()` an
  eine `EntityUuid`-Grenze.

Logs: `/tmp/sim031-coreuuid-tests.log`,
`/tmp/sim031-coreuuid-tests-no-default.log`,
`/tmp/sim031-coreuuid-clippy.log`,
`/tmp/sim031-coreuuid-clippy-no-default.log`,
`/tmp/sim031-coreuuid-fmt.log`.
Finales geänderte-Dateien-Hashmanifest: `/tmp/sim031-coreuuid-freeze.sha256`.
Keine Dependency-/Lock-/Storage-/Sim-/World-/App-/Protokolländerung.
