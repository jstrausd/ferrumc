# ITEM-SIZE-01 — Stackgrößen und exakte Repräsentation ohne Bytebuffer

Status: Producer nach Rootfreigabe implementiert und vollständig geprüft;
Source-/Evidencefreeze zur unabhängigen Rootabnahme. Consumer noch getrennt.
Freigegebener Entwurf SHA-256:
`2c88bf26ef8e597d0ef04971b4a10fe2dfbff4f971d5ef0ce87bc1d562a9afce`.
Autor besitzt ausschließlich
`server/crates/ferrumc-items/**` und diesen Brief. Eigener Scratch unter
`.codex-tmp/item_size_01/`. Sim, Storage, Proto, Net, Registry, Session, App,
Dependencies und Rootboards readonly. NET-04.1b bleibt eingefroren.

## Konkreter Bedarf und Umfang

Die tatsächlichen readonly Consumer `InventoryPlan::new`, `pickup` und `same`
in `ferrumc-sim/src/player/plan.rs` serialisieren FCIS temporär, um nur Länge
oder eine vom Count unabhängige Repräsentation zu prüfen. Bei `same` werden
zwei Bytebuffer erzeugt und ausschließlich das feste Countbyte auf0 gesetzt.
Spawn-/Dropbudgetpfade verwenden ebenfalls `encode_persisted_stack(...).len()`.
Der Simautor bestätigt den Bedarf und hält seine Quellen bis Producerabnahme.

Dieses Ticket stellt nur die fehlenden öffentlichen borrowed Producer bereit.
Keine Consumerintegration, Stacknormalisierung, Komponenten-/NBTkopie, neue
Bytebuffer, Parser, Dependency, Queue, Storage-/Publicationauthority oder
Gameplayaktivierung. Bestehende gültige FCIS-, Slot- und Transferbytes bleiben
unverändert. Der spätere Simconsumer ersetzt ausschließlich seine temporären
Sizing-/Vergleichsencodes und behält Countplanung/Materialisierung getrennt.

## Konkrete öffentliche APIs

```rust
pub fn persisted_stack_size(
    stack: &ItemStack,
) -> Result<usize, ItemPersistenceError>;

pub fn same_stack_representation(
    source: &ItemStack,
    destination: &ItemStack,
) -> Result<bool, StackTransferError>;
```

`persisted_stack_size` liegt in `persistence.rs` und ist ein enger Wrapper über
das vorhandene `authoritative_encoded_size`; keine zweite Sizingimplementierung
und keine Änderung seiner bestehenden internen Craftingconsumer. Reexport aus
`lib.rs`. Ergebnis ist die **ganze schema1-FCIS-Recordgröße**, kein Slot-/NBT-
Payloadmaß, keine Heapgröße und keine Aggregatebudgetreservierung.

`same_stack_representation` liegt in `transfer.rs`; Reexport aus `lib.rs`.
Zuerst `validate_authoritative_stack(source)` mit `InvalidSource`, danach
`validate_authoritative_stack(destination)` mit `InvalidDestination`, jeweils
über den bestehenden lossless Preflight. Erst dann boolesche Entscheidung.
Kein früher `false` für verschiedene Item-IDs, leere Inputs oder verschiedene
Patches vor diesen Checks. Sourcefehler gewinnt, wenn beide Inputs ungültig
sind; bei gültiger Source darf auch ein inkompatibles/leeres Gegenstück keinen
Destinationfehler verstecken. Keine neue Errorabstraktion; der Simconsumer
besitzt bereits `From<StackTransferError>`.

Bei gültigen Inputs gilt:

- Identische `ItemId` plus bestehender privater `same_patch_representation`-
  Vergleich. Den vorhandenen `same_component_representation`-/
  `same_nbt_representation`-Pfad wiederverwenden, keinen neuen NBTvergleich.
- Gegenwärtige Counts werden vorab geprüft, beeinflussen dann das Bool nicht.
  Count1 vs64 kann `true` sein; ein Registrycount65 darf kein `true` liefern.
- Zwei canonical Empty sind gleich, Empty vs Present ungleich. Die öffentlichen
  Konstruktoren liefern Empty mit `None`/Count0/leerem Patch. Keine neue
  Canonicalization und kein veränderter Empty-Preflight. PresentAIR bleibt
  Present, einschließlich seiner gültigen Modellcounts.
- Reihenfolge, typed/opaque Varianten, in-range Removals, NBT-Key-/Duplicate-
  Reihenfolge und rohe Floatbits zählen. Identische NaNbits sind gleich;
  andere NaNpayloads und +0/-0 ungleich, auch rekursiv in Listen/Compounds.
  Eine typed MaxStackSize und wireähnliche opaque Form bleiben verschieden.

Das ist Repräsentationsgleichheit des unterstützten Persistenz-/Transfermodells,
keine Minecraft-Stackability oder Papers semantische Prototype-/Componentmap-
Gleichheit. Die bestehende Abweichung ist in [ITEMS-TRANSFER-001](ITEMS-TRANSFER-001.md)
und Items-README dokumentiert; kein neuer Paperboot oder Gameplaynachweis.

## Korrektheit, Ressourcen und Fehler

Die bestehende Größenauthority preflightet Registrycount, insgesamt64 Patch-
Einträge,96 gültige/konfliktfreie IDs, typed Werte,8192 Bytes pro Payload,
256KiB insgesamt, kanonische Itemnamen und begrenztes File-NBT. NBT umfasst
Modified-UTF8, homogene Listen, Sequenz-/Bytecaps und Tiefe bis512. Sie benutzt
ein festes64er usize-Array und96er IDset, keine Vec-/String-/Componentclones.
Die öffentlichen Wrapper erben diese Checks und Fehler unverändert.

Empty hat exakt7 Bytes. Present hat
`12 + name_utf8_len + 9*added + 4*removed + payload_sum` Bytes. Das feste
Countbyte verändert die Länge für gültige Presentcounts nicht; Count0 in einer
späteren scalar Countplanung wird als canonical Empty mit7 Bytes gerechnet.
Nach dem Preflight gilt `name_len<=256`, `added+removed<=64` und
`payload_sum<=262144`; das Envelope ist höchstens844 Bytes, somit der Record
höchstens262988 und unter `MAX_PERSISTED_STACK_BYTES=263168`. Die vorhandene
usize-Arithmetik ist dadurch ohne Overflow; keine Änderung gültiger Bounds.

Der borrowed Repräsentationsvergleich läuft erst nach beiden Preflights und
über deren begrenzte Bäume/Slices. Keine temporäre Serialisierung oder
NBTnormalisierung. Fehlerpayloads des bestehenden Preflights enthalten
numerische Klassifizierung; kein unbekannter Namensdecode oder Stringclone.
Inputkapazitäten/-inhalte werden weder verändert noch als Heapbudget gewertet.
Allokationsfreiheit wird am vollständigen Callgraph und beim unabhängigen
Rootreview geprüft; kein unsicherer Globalallocator oder neues Testframework.
Keine allgemeine FPS/TPS-/RSS-Verbesserung aus diesem Producer allein behaupten.

## Enge Dateien und tatsächliche Abnahme

Umgesetzt: `src/persistence.rs`, `src/transfer.rs`, `src/lib.rs`, neues enges
Public-API-Testfile `tests/borrowed_stack_accounting.rs`,
Items-README/INVARIANTS und dieser Brief. Private Vergleicher und Preflight-
Algorithmen funktional unverändert; kein Encoder-/Decoder-/NBT-/Crafting-
Runtimeumbau. Ein zulässiger neuer öffentlichen Producer ist noch kein
geprüfter Consumer oder persistenter Commit.

| Probe | Erwartete echte Abnahme |
| --- | --- |
| Ganze FCIS-Größe | Neue Public-API gegen echte Encoderlänge und vorhandene goldene FCIS-Bytes: Empty7, komponentenlos, alle sieben Modellvarianten, signed Damage, MUTF8/NUL/astral, ordered/duplicate NBT und beide Floatbreiten. |
| Größen-N/N+1 | Countmax/übermax,64/65 Patches,8192/8193 File-NBT-Payloadbytes,256KiB/+1 Gesamtsumme, Tiefe512/513 und heterogene Listen; genaue bestehende Fehler. Keine neue unkonstruierbare Opaque-Übercapfixture. |
| Count-/Item-/Emptyregel | Gleiche gültige Presentrepr bei verschiedenen Counts; verschiedene Item-IDs; beide Empty; Empty vs Present; PresentAIR vs Empty. Ungültige Counts bleiben Fehler. |
| Exakte Repräsentation | Typed vs opaque, addition-/removalorder, doppelte NBT-Keys/Order, nested Lists/Compounds, identische/distinkte NaNbits und signed zero in f32/f64. |
| Beide Fehlerseiten | InvalidSource/InvalidDestination vor Bool bei gleichen/verschiedenen IDs, Emptygegenstück und verschiedenen Patches; Sourceprecedence bei zwei Fehlern. Keine Inputmutation. |
| Unabhängiger Repr-Orakel | In Tests ausschließlich gültige originale FCIS-Encoderbytes vergleichen, mit nur dem festen Countbyte maskiert; Empty separat. Das Produktions-API encodiert niemals. |
| Regression/Ressourcen | Whole Items Debug/Release, strict Alltargetclippy/Fmt/Items-Rustdoc; vorhandene Persistence-/Transfer-/Craftingtests unverändert bestehen; Callgraph ohne Encoding, Allokation oder Clone vor Ergebnis. |

Vor Code Rootfreigabe des Entwurfs; danach exact Items-/Brief-Sourcefreeze,
Evidencefreeze und unabhängiger Rootreview. Nur Root integriert Git. Simautor
integriert erst den abgenommenen Producer in seinem separaten Consumer;
NET-04.2a-Metadatabuilder folgt danach in einem eigenen Items-Ticket.

## Tatsächlicher Producerstand und Prüfevidence

Die Runtimeänderung besteht aus den beiden dokumentierten öffentlichen
Wrappern und ihren Reexports. Die gesamten ursprünglichen `persistence.rs`
und `transfer.rs` sind nach Entfernen ausschließlich der neuen APIblöcke
byteidentisch mit dem veröffentlichten Ausgangsstand
`765147ca3935cd69260a4bf682d756299cce4aed`;21 weitere vorhandene Itemsdateien
sind vollständig unverändert. Damit bleiben alte Preflight-/Vergleichs-
Algorithmen und alle bestehenden Encoder-/Decoder-/Transferpfade unverändert.

13 neue Integrationstests prüfen die echte FCIS-Encoderlänge, vorhandene
goldene Empty-/Stonebytes, alle7 Modellkomponenten und NBTtagfamilien,
gültige Counts1..64, PresentAIR/Empty, sowie die Größen-/Tiefe-N/N+1-Grenzen.
Ein18er gültiger Repräsentationskorpus erzeugt324 Paarvergleiche gegen die
unveränderten FCIS-Encoderbytes mit ausschließlich maskiertem Countbyte.
Zusätzliche verschachtelte f32-/f64-Tests prüfen signed zero und gleiche/
verschiedene NaNbits. Beidseitig ungültige Counts/Patches werden auch bei
Empty, verschiedenen IDs und verschiedenen Patches vor Bool geprüft;
Sourcefehler gewinnt bei zwei Fehlern. Encoding/Cloning ist nur Testorakel.

Alle fünf erforderlichen Checks liefen im eigenen isolierten Archiv des
obigen Ausgangsstands mit ausschließlich dem Items-/Brief-Overlay und einem
eigenen Targetdir.116 readonly Dependency-/Generated-/Spec-/Konfigurations-
Dateien wurden vor und nach den Checks SHA-256-gepinnt. Resultate:

| Check | Tatsächliches Ergebnis |
| --- | --- |
| Whole Items Debug |103 bestanden:84 Unit,13 neue Integration,3 vorhandene Craftingintegration,3 Doctests. |
| Whole Items Release | Dieselben103 bestanden. |
| Clippy | `cargo clippy --locked -p ferrumc-items --all-targets -- -D warnings` bestanden, keine Ausnahmen. |
| Fmt | `cargo fmt -p ferrumc-items -- --check` bestanden. |
| Items-Rustdoc | `cargo rustdoc --locked -p ferrumc-items -- -D warnings` bestanden. |

Aufruf-/Exit-/Zeitrecords und vollständige Logs liegen in
`.codex-tmp/item_size_01/checks/`. Die endgültigen SHA-256-Dateimaps liegen in
`.codex-tmp/item_size_01/source.freeze.json` (gesamtes Items-Subtree und dieser
Brief) und `evidence.freeze.json` (Pins, Preservation-/Callgraphnachweis und
Checkrecords/-logs). Die Allokationsaussage beruht auf dem vollständigen
unveränderten Callgraph, nicht auf Allocatorinstrumentierung oder einem
Performancebenchmark. Kein Consumer, Slotbudget, Gameplay, Net-/Sessionpfad
oder Persistenzpublish wurde hier aktiviert; daraus folgt kein Survival-,
Paperplugin- oder allgemeiner Performancevollständigkeitsnachweis.
