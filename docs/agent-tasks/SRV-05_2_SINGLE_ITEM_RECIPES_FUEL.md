# SRV-05.2 — Koch-/Steinschneiderrezepte und originale Brennstoffe

Status: konkretes Folgeticket vorbereitet, noch nicht vergeben. Ein Autor
besitzt ausschließlich `server/crates/ferrumc-items/**`, diesen Brief und seine
benannten Rezeptproducer-/Fixtures unter `tools/` und `docs/fixtures/`.
Voraussetzung ist CLEAN SRV-05.1 (`572083d1`). Keine Änderungen an Sim, World,
Storage, App, Java, Cargo/Lock oder Git. Kein neues allgemeines Rezeptframework.
Root prüft den konkreten API-/Datei-/Ressourcenentwurf vor Runtimecode.

## Ergebnis

Die vorhandenen normalen Rezepte werden um zwei konkrete Producer ergänzt:
alle114 originalen Kochrezepte (72Smelting,24Blasting,9Smoking,9Campfire) und
alle254 Stonecuttingrezepte. Dazu der vollständige originale FuelValues-Katalog
bei den gepinnten Vanilla-Featureflags. Geschlossene Recipekind- und Ressourcen-
identität, genaue Ingredientmitgliedschaften, Group/Category, fixed Results,
originale f32-Experiencebits und Cookingticks, allocfreier Lookup/Match.
Kein Namens-/Suffix-/Materialheuristikersatz für Tags oder Brennstoffe.

Ein synchroner bounded Single-operationplan konsumiert genau ein Inputitem
und kopiert das feste Rezeptresultat. Borrowed Inputadmission und vollständige
encoded-/logical-clone-Budgets erfolgen vor jedem Clone. Inputpatches bleiben
auf verbleibendem Input erhalten und gelangen nicht in das fixed Resultat.
Inputverbrauch und mögliche fuel-/slot-/containerbedingte Remainderwirkung
sind verschieden: keine normalen Craftingremainders in diese Rezepte erfinden.
Owned Resultate überleben Input-/Katalogdrop, besitzen aber keine Commitauthority.

Fuelabfrage gibt originale Basisticks und tatsächliche Item-Craftingremainders
für den späteren Verbraucher. Die konkreten Furnace/Smoker/Blast-overrides
werden aus der vollständigen Implementierung festgehalten. Ofentick, Slotmerge,
Cookprogress, XP-Orbs, Blockstate, Fuelverbrauch, Campfire-Drops, GUIklicks,
Pluginänderungen und Persistenz gehören anschließend ihren benannten Consumern.
Ein Datenkatalog wird nicht als spielbarer Ofen bezeichnet.

## Referenzpflicht und bisherige Rootbefunde

Minecraft1.21.8/772, DataVersion4440; Paper1.21.8 Build60 Commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Der Autor liest Vanilla-Basis,
vollständige Appliedmethoden, Callees und alle relevanten Source-/Featurepatches
und pinnt eine genaue Quellenmatrix. Root hat Applied SingleItemRecipe,
AbstractCookingRecipe inklusive Serializer, vier konkrete Cookingklassen,
StonecutterRecipe, vollständige FuelValues inklusive Builder sowie vollständige
Furnace serverTick/canBurn/burn/getBurnDuration/getTotalCookTime/XP- und
Campfire cookTick/cooldownTick/placeFood-Pfade gelesen. Das ist Quellenlektüre,
noch kein neuer Original-JAR-/Rust-/Serverpass.

Verbindliche Befunde:

- SingleItemRecipe.matches liest tatsächliche Ingredientmitgliedschaft;
  assemble kopiert ausschließlich das fixed Resultat. Stonecutting verwendet
  STRICT_CODEC; Cooking STRICT_SINGLE_ITEM_CODEC. Serializerdefaults und
  Streamcodec-Type-/Feldreihenfolge aus den echten Klassen exportieren.
- FuelValues.vanillaBurnTimes verwendet echte aufgelöste Itemtags, sequenzierte
  Put-/Overwriteordnung, Featureflags und abschließendes remove(NON_FLAMMABLE_WOOD).
  FuelisFuel und burnDuration sind verschiedene Abfragen; empty hat0Ticks.
  Z.B. carpet67, driedkelpblock4001 und bamboo50 sind Integerarithmetik,
  keine gerundeten idealisierten Schmelzquoten.
- Furnace decremented Litremaining läuft vor neuer Fueladmission. BurnEvent
  kann cancellation/burntime/burning/consumeFuel verändern; Craftingremainder
  ersetzt nur tatsächlich leer gewordenen Fuelstack. StartSmelt vor Timerstart,
  SmeltEvent vor Output-/Inputänderung; das Resultat wird danach neu gelesen.
- canBurn verwendet same-item-and-components und originale Kapazitätsbranches.
  Paper burn prüft nach Pluginresultänderung Bukkit-isSimilar. Bereits vorhandene
  Rustrepresentation-Gleichheit ist keine bewiesene effektive Componentgleichheit.
  Keine neue Default-/Componentnormalisierung als Nebenänderung dieser .2.
- WetSponge plus vorhandener Bucket im Fuelslot erzeugt nach erfolgreichem
  Smelten WaterBucket, bevor Input schrumpft. Das ist Verbraucherlogik,
  kein Craftingremainder des Rezeptinputs.
- XP verwendet f32-Produkt/floor/fract plus Zufallsentscheidung und
  FurnaceExtract-/BlockExpEvent. Katalogexperience ist weder sofortige XP-
  Veröffentlichung noch eine globale Math.random-Nachbildung.
- Campfire verarbeitet vier Slots in Originalreihenfolge, Paper stopCooking,
  fehlendes Rezeptfallback und enabledFeatures. BlockCook-Cancellation kehrt
  aus dem ganzen Tick zurück. Paper dropt geteilte Resultatitems mit eigener
  Motion; placeFood konsumiert nur ohne Creative-Bypass gemäß tatsächlichem
  consumeAndReturn. Diese Tick-/Entityconsumer bleiben Folgearbeit.

## Tatsächlicher Daten-/Testproducer

Vor Runtime einen exakten Census aus den echten Original-JAR-Methoden liefern.
Bestehende gepinnte Recipe-/Itemressourcen und TagLoader/PendingTags-Verträge
wiederverwenden, nicht die bereits geprüften .1-Fixtures verändern. Neuer Helper
ruft tatsächliche Recipe.CODEC/Ingredient.matches/SingleItemRecipe.assemble,
Cookingmetadata, Streamcodecs und FuelValues.vanillaBurnTimes/isFuel/burnDuration
auf. Keine nachgeschriebenen Java-Match-/Tag-/Fuel-/Planformeln als Oracle.

Alle1416 Originalitems gegen alle368 Rezipienten matchen; Ergebnisse/Metadata/
Streams und alle Fuelitems/ticks/remainders vollständig exportieren. Zusätzlich
mehrere Counts/patches, gemischte Kochtypen, signedzero/NaN/opaque/NBT-Daten,
empty/present-air sowie konkret falsche Rezept-/Tag-/Itempins testen.
Verbraucherreihenfolge und selectionpriority nicht aus Catalogsortierung ableiten.
Normal/-O-Reexport bytegleich; alle Originalartefakte vor/nach gehasht, frischer
Helper-/OriginalClassorigin und negative Herkunfts-/Hash-/Shadow-/Envguards.

Konkrete Caps vor Allocation aus gemessenem Catalog-/Membership-/Fixedoutput-
Census und maximalem Plan begründen. Keine pro-Match-Allokation, globalen Caches,
IO-/Thread-/DBhandles oder Erweiterung alter .1-Caps als Nebenänderung. Typed
unknown-kind/resource/malformed/present-air/budget errors, N/N−1/+1 an allen
Caps; Refusal verändert oder normalisiert kein Calleritem.

Whole-items Debug/Release, strict all-target Clippy/Fmt/Rustdoc und unveränderte
.1-Abnahmen. Exakter owned File-/Evidenzfreeze und unabhängiger Rootreview vor
Integration. Danach tatsächlicher Ofen-/Campfireconsumer in BLK-14/SRV-17 und
Stonecutterscreen SRV-18 innerhalb der neuen DUR-04/05-Playerauthority.
Specialcrafting/Transmute/Smithing, wiederholtes Crafting, Recipebook, Java-
Recipe-/Cookevents und Survivalprogression bleiben eigene Folgearbeit.
