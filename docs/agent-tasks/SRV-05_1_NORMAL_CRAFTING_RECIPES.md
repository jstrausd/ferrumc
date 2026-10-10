# SRV-05.1 — Normale Craftingrezepte und konservative Craftpläne

Status: nächste unabhängige Itemsvergabe nach einem freien Childslot.
Root orchestriert; der zugewiesene Autor besitzt genau `ferrumc-items` und
diesen Brief. Kein Sim-/App-/Storage-/Registry-/Coreedit, kein Gitcommit.

## Ziel und konkrete vorhandene Daten

Normale 2×2-/3×3-Craftingrezepte müssen vor ihrem Simconsumer tatsächlich
matchen und einen begrenzten vollständigen Craftplan liefern. Der gepinnte
Originalexport enthält 1.407 Rezepte, darunter **673 shaped und 295 shapeless**.
Alle 968 normalen Rezepte gehören zum gewünschten Producerumfang, einschließlich
der 16 normalen Resultate mit Suspicious-Stew-Komponenten. Andere Serializer
bleiben ausdrücklich separat; SRV-05 ist dadurch noch nicht vollständig.

Vor Runtimecode einen konkreten umsetzbaren API-/Daten-/Ressourcenentwurf in
diesem Brief liefern. Root prüft ihn innerhalb der vorhandenen Autorisierung.
Keine allgemeine Recipeengine oder erneute Grundsatzrunde: anschließend den
normalen Recipebestand, Matcher und genau einen Craftplan implementieren.

## Quellen und Producerbesitz

- AGENTS/CLAUDE, Items-INVARIANTS/README, Architektur und PAPER_REFERENCE lesen.
- Original `recipes.json` SHA256
  `023cc012b9e01b138831c302bdfb956e4179203b447a9e1826fc0939f285a232` und
  seinen Manifest-/Versionpin beibehalten; keine handkopierten Rezeptlisten.
- Vollständige originale und angewandte Paper-Methoden einschließlich Callees:
  ShapedRecipe, ShapedRecipePattern, ShapelessRecipe, Ingredient, CraftingInput,
  CraftingRecipe, Recipe/RecipeManager/RecipeMap, StackedContents,
  StackedItemContents, Item.getCraftingRemainder und ResultSlot.onTake.
  Zugehörige Sourcepatches sowie Featurepatch0020 vollständig prüfen.
- Tags fehlen im bisherigen Originalexport ausdrücklich. Benötigte echte
  Builtintags und Remainders durch einen konkreten reproduzierbaren
  Original-JAR-/Datagenproducer in der Itemscrate erfassen; keine Logs-/Planks-
  Suffixheuristik oder leere Ersatzsets. Referenz-/Generator-/Fixturefiles
  innerhalb derselben Crate gehören zum Ticket.
- Ein konkret notwendiger anderer Crateproducer wird zuerst Root gemeldet;
  nicht ohne Vergabe andere Dateien verändern. Cargo.lock besitzt Root.

## Matcher und Plan

Private Felder und kontrollierte Konstruktion; höchstens neun Gridslots,
drei Reihen/Spalten und explizit bounded Rezept-/Ingredientsets. Originale
Input-/Patterntrimmung und Positionoffsets, gespiegeltes shaped Matching,
Innenlücken und genaue Anzahl belegter Slots prüfen. Normale Zutaten beachten
Item-/Tagzugehörigkeit; Count64 in einem Slot ersetzt keine zwei belegten
Zutatenpositionen. Überlappende shapeless Alternativen benötigen eine echte
vollständige Zuordnung, keine fehlerhafte greedy Auswahl.

Ein erfolgreicher Singlecraftplan besitzt Output, alle Gridremainder und
separat ausgewiesene Craftingreste. Jede verwendete Position verbraucht genau
ein Item; unverbrauchte Stack-/Komponentenrepräsentation bleibt erhalten.
Assembly kopiert das tatsächliche feste Resultat, übernimmt keine beliebigen
Inputkomponenten. Buckets und übrige tatsächliche Remainders gehen nie verloren
und werden nicht ungeprüft in inkompatible Reststacks geschrieben. Overflow-
oder Drop-/Inventarplatzierung bleibt Aufgabe des autoritativen Simconsumers.

Alle Inputs und Resultate vor Clone/Allocation durch bestehende verlustfreie
Itemspreflights prüfen; aggregate Grid-/Planbytes und Clonebestand begrenzen.
Kein Creative-normalize, kein stilles Clamp/Stripping, kein In-place-Halbplan.
Present-air und andere bislang erlaubte Modellformen brauchen eine explizite
Originalemptiness-/Admissionpolicy. Liefere typed Refusals und keinen
ersetzenden leeren Craftoutput. Bereits vorhandene Itemcodecs/-Transfers nicht
heimlich semantisch umdeuten.

Paper ExactChoice verlangt aufgelöste Default-/Componentgleichheit und den
tatsächlichen Extrasmap-Vertrag. Diese Eigenschaft nicht aus ordered Patch-
Representationgleichheit ableiten. Der normale erste Producer darf ExactChoice
ausdrücklich Unsupported melden; Pluginrezepte und Specials folgen separat.
Keine Behauptung, dass dieser reine Plan schon Outputklicks, Recipebook,
Shiftcraft, Autosave/CAS, Javaevents oder Onlinecrafting aktiviert.

## Abnahme

Konkrete echte Original-JAR-Matches, Resultate und Remainders für sämtliche
968 normalen Rezepte, inklusive aller erlaubten Offsets/Mirrors, fehlenden/
zusätzlichen Zutaten, Tags/Alternativen, Countgrenzen, vollen Remainderstacks
und komponententragenden Inputs/Outputs. Frische gepinnte Helper/CodeSources,
normal/-O-Reexport und negative Herkunfts-/Hashguards. Parser-/Konstruktor-
Grenzen und unmodifizierte Callerinputs auch bei Refusal prüfen.

Zusätzlich ein unabhängig formuliertes kleines bipartites Matchingmodell und
Mengen-/Komponentenbilanz, nicht ausschließlich Codecroundtrips. Debug/Release
der vollständigen Itemscrate, strict all-target Clippy/Fmt und public Rustdoc.
Exakter Dateifreeze und unabhängiger Review vor Integration.

Nach CLEAN folgt SRV-06s tatsächlicher 2×2-/3×3-Craftingconsumer unter der
verfügbaren durable Ownerauthority sowie SRV-07s Output-/Shift-/Bookwiring.
Die übrigen Serializer und speziellen Komponentenumformungen erhalten
konkrete Folgeuntertickets derselben Survivalroute.
