# ITEMS-TRANSFER-001 — Checked Stack-Split und Teiltransfer

Stand: 10.10.2026. Produktions-Crate ausschließlich `ferrumc-items`.
Fundament für spätere DUR-03/SRV-02-Consumer; dieser Schritt aktiviert kein
Drop/Pickup und schreibt keine Items in Storage.

## Implementierter Vertrag

- `plan_split(&ItemStack, requested: u8)` und
  `plan_transfer(&ItemStack, &ItemStack, requested: u8, destination_capacity: u8)`
  liefern einen `StackTransferPlan` mit `source_after`, `destination_after`,
  `moved` und `into_stacks`. Borrowed Eingaben bleiben unverändert.
- `requested` ist eine Mengenobergrenze, kein Authoritativcount: 0 ist no-op;
  eine höhere Anfrage nimmt höchstens die vorhandene Source-Menge.
- Destinationcapacity ist die gesamte erlaubte Zielmenge. Alle `u8`-Werte
  sind zulässig; die sichere Obergrenze ist zusätzlich das Registrymaximum.
  Capacity 255 erweitert daher keinen Registrycount. Ein vorhandenes Ziel über
  der gelieferten Capacity wird als Fehler gemeldet und nicht verkleinert.
- Auch bei no-op, leerer Source, vollem oder inkompatiblem Ziel prüfen beide
  Modelle vor jedem Ergebnisclone dieselben Bounds wie die Itempersistenz.
  Ungültige Source-/Destinationdaten haben getrennte Fehlerklassen.
- Nur identische Item-IDs mit exakt gleicher geordneter Modellrepräsentation
  dürfen zusammengeführt werden. Kein Swap/Overwrite bei Inkompatibilität.
  Beide Ergebniscounts konservieren jede Item-/Patch-Identität; vollständig
  entleerte Ergebnisse haben kein Item, Count 0 und leeren Patch.
- Modellierte Varianten, opaque Bytes, signed Durabilitywerte, Removals,
  NBT-Duplikatnamen und Reihenfolge bleiben erhalten. Floatvergleich verwendet
  rekursiv exakte Bits: identische NaN-Bits passen, verschiedene Payloads und
  Signed-Zero-Bits passen nicht. Die bestehende derived `PartialEq` des NBT-
  Modells wird für floattragende Transferkompatibilität nicht verwendet.
- Minimaler gemeinsamer `pub(crate)`-Preflight in `persistence.rs` ersetzt die
  frühere kleine Payloadgrößen-Vec durch ein festes 64er Array. Storageformat,
  Schema, Encoderoutput und Decodervalidierung bleiben unverändert.

## Originalreferenz und konkrete Abweichungen

Minecraft Java **1.21.8 / Protokoll 772**, Paper **Build 60**, Commit
`29c8822d90899c89d2689338e81a98f690bcba12`.
Gelesen wurden die vollständigen Methoden in der angewandten Vanilla-/Paper-
Quelle unter `.codex-tmp/paper-1.21.8-reference/paper-server/src/minecraft/java/`,
einschließlich Callees und relevanter Source-/Feature-Patches gemäß
[PAPER_REFERENCE.md](../PAPER_REFERENCE.md).

| Vollständige Methode/Quelle unter `net/minecraft/` | Originalverhalten / Grenze dieses Schritts |
| --- | --- |
| `world/item/ItemStack.java`: `split`, `copyWithCount`, `copy`, `grow`, `shrink`, `getCount`, `isEmpty` | Split benutzt `min(requested, getCount())`, kopiert die Komponenten und reduziert die Source. `copyWithCount` kann einen Present-Stack mit Count 0 erzeugen; der primitive Rustvertrag stellt canonical Empty her. Paper betrachtet AIR als leer, während das unterstützte FerrumC-Persistenzmodell Present-AIR erhält. Negative Java-Mengen sind durch die Rust-`u8`-API nicht darstellbar. |
| `ItemStack.java`: `getMaxStackSize`, `isStackable`, `isDamageableItem`, `isDamaged`, `validateComponents` | Paper löst MAX_STACK_SIZE aus Komponenten, prüft Damage-/Unbreakable-Zustände und verbietet bestimmte damageable+stackable Kombinationen. Hier gelten die existierenden Modell-/Registrybounds; Consumer müssen wirksame Capacity und Eligibility zusätzlich liefern. Ein `max_stack_size` kann die Registrygrenze weiterhin nicht anheben. |
| `ItemStack.java`: `isSameItemSameComponents`; `core/component/PatchedDataComponentMap.java`: `fromPatch`, `applyPatch`, `set`, `remove`, `equals`, `copy` | Paper benutzt Prototype plus normalisierten Map-Patch; Defaultwerte oder das Entfernen abwesender Defaults können verschwinden. Map-Gleichheit ist nicht Reihenfolgengleichheit. FerrumCs ordered Modellrepräsentation bleibt unverändert und vergleicht deshalb strenger; expliziter Default vs. leerer Patch und anders geordnete Patches passen nicht. |
| `world/Container.java`: `getMaxStackSize(ItemStack)`; `world/entity/player/Inventory.java`: `hasRemainingSpaceForItem`, `canHold`, beide `addResource`, beide `add`, `getSlotWithRemainingSpace`, `placeItemBackInInventory`, `removeItem` | Originalcapacity ist min(Containerlimit, effektives Itemlimit), passende belegte Offhand ist berücksichtigt; Slotreihenfolge, Damagebranch und Creativeoverflow sind eigenständig. Hier wird nur eine explizit gewählte Source/Destination betrachtet; keine Inventarsuche oder Creativevernichtung. |
| `world/entity/item/ItemEntity.java`: `tryToMerge`, `areMergable`, alle `merge`, `playerTouch` | Item-Entitymerge hat Owner-/Delay-/Age-/Ganzmengen-Guards und ItemMergeEvent vor Mutation. Pickup hat Attempt-, Legacy-, Entity-Events, Cancellationrestauration, Stackersetzung, Pickupdelay/Owner, Inventaradd, Take/Discard und Stats. Diese Welt-/Plugin-/Paketreihenfolge liegt außerhalb des reinen Plans; allgemeiner Teiltransfer ist nicht automatisch ein zulässiger Item-Entitymerge. |
| `world/item/component/CustomData.java`: `equals`, `copyTag`; `nbt/CompoundTag.java`: Ladepfad, `put`, `copy`, `equals` | Paper verwendet eine NBT-Map, Duplicatekeys werden beim Laden ersetzt und Mapgleichheit ignoriert Reihenfolge. FerrumC erhält geordnete Duplicate-Namen; Transfer darf sie nicht kollabieren. |
| `nbt/FloatTag.java`, `nbt/DoubleTag.java`: Records, `valueOf`, `copy`, `write` | Java-Recordgleichheit hat andere NaN-Regeln; `valueOf` normalisiert Signed Zero auf ZERO und die Java-Ausgabe normalisiert NaN-Bits. Der Rustmodellvertrag erhält vorhandene Bits und verlangt deren Identität. Kein Versprechen derselben Java-NBT-Repräsentation. |
| `core/component/DataComponents.java`: MAX_STACK_SIZE/MAX_DAMAGE/DAMAGE Registrierung | Paper persistent begrenzt MAX_STACK_SIZE auf 1..99, MAX_DAMAGE positiv und DAMAGE nichtnegativ. Das bestehende FerrumC-Persistenzmodell erhält signed Damage/MaxDamage; dieses Ticket verschärft den Modellvertrag nicht nachträglich. Opaque für modellierte IDs bleibt opaque und wird nicht neu interpretiert. |

Relevante Sourcepatches `ItemStack.java.patch`, `Inventory.java.patch`,
`ItemEntity.java.patch`, `CompoundTag.java.patch` und
`DataComponentPatch.java.patch` wurden mitgelesen. Featurepatch
`0028-Optimize-Hoppers.patch` ergänzt `copy(originalItem)`/Copy-On-Write-
Verwendung und Hopper-Event-/Restaurationpfade; der reine Rustplan aktiviert
diese Consumer nicht. Quellenlektüre ist keine bestandene Live-Paper-
Differenzialabnahme.

## Bounds, Ownership und nächste Consumer

Vor Resultclones: maximal 64 Patchentries, 8192 **encoded** Payloadbytes je
Komponente, 256 KiB gesamt, gültige IDs/Typedwerte und passende NBT-Struktur/
Depth/Sequence-/Stringbudgets. Preflight ist allocation-free mit festem
64er Größenarray und 96er ID-Set. Erfolgreiche Ergebnisse besitzen höchstens
zwei logische Modellkopien; decoded Heapkosten hängen zusätzlich von NBT-
Enum-/String-/Container- und Allocatoroverhead ab. Encodedbudget ist kein gleich
großes Heapbudget. Gleichzeitige Plans/Inventare benötigen eigene Aggregatlimits.

Der Besitzer muss beide Outputs unter exklusiver Authority veröffentlichen
und Gameplay-/Slotpolicy prüfen. Plan enthält weder Versions-/CAS-Guard noch
Transaktions-ID, Exactly-Once-Zusage, Replayprotection, Storagecommit oder
Crashrecovery. DUR-03/SRV-02 müssen diese Verträge getrennt umsetzen; Socket-
oder Restarttests gegen zukünftige Consumer wären hier noch kein Nachweis.

## Abnahme

Eigene deterministische Tests prüfen Mengen-/Bytekonservierung mit einer
unabhängigen Persistenzrepräsentationsbilanz, alle `u8`-Anfragen für sämtliche
Stonecounts, eine Capacity-/Restmatrix, alle registrierten Itemceilings,
Inkompatibilität/Defaults/Reihenfolgen, opaque/NaN/Signed-Zero, Fehlerklassifikation
und unveränderte Eingaben bei ungültigem Count/Patch/NBT/Budget.
Die vollständigen bestehenden Itempersistenztests prüfen die unveränderte
Schema-/Encoderrepräsentation zusätzlich.

Abgeschlossen: **72 Unit-Tests und 3 Doctests grün**, darunter 13 neue
Transfercontract-Tests. Die Splitmatrix umfasst 16.640 Count-/Requestfälle,
die Capacitymatrix 82.944 Fälle; alle Registryitems werden zusätzlich geprüft.
Exakte 8192-Byte-/256-KiB-Grenzen und Überschreitung der kombinierten Payload-
und Entrybudgets haben eigene Fälle.

`cargo clippy -p ferrumc-items --all-targets -- -D warnings` und der
Rustfmtcheck sämtlicher geänderter Rustdateien sind grün. Logs:
`/tmp/ferrumc-items-transfer-tests-final.log`,
`/tmp/ferrumc-items-transfer-clippy-final.log`,
`/tmp/ferrumc-items-transfer-fmt-final.log`.
Keine neue Socket-/Restart-/Javaevent- oder Live-Paper-Abnahme für diesen reinen
Modellschritt; dafür existiert noch kein integrierter Produktionsconsumer.
