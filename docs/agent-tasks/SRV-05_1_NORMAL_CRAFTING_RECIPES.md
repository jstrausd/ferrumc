# SRV-05.1 — Normale Craftingrezepte und konservative Craftpläne

Status: Autor `/root/feature_catalog`; implementiert und geprüft, Freeze für
unabhängigen Rootreview. Integration/Onlineconsumer bleiben Rootaufgaben.
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

## Konkreter Implementierungsentwurf

Root hat am 2026-10-10 den vollständigen Entwurf mit SHA256
`efa83f86ce89d74df143370210fd6ed88559efc581b3c4cd0eae0ba852b401fa`
innerhalb der vorhandenen Nutzerautorisierung geprüft und intern zur konkreten
Runtime-/Originaldatenimplementierung freigegeben. Vor Integration folgen die
genannten vollständigen Checks und ein unabhängiger Review; dies ist noch
keine Abnahme des Matchers oder des Onlinecraftings.

Der geschlossene Builtinproducer liefert alle 968 normalen Rezepte aus dem
gepinnten Export, keine Caller-/Plugin-JSON-API. Ein frisch kompilierter
Java-21-Helper lädt ausschließlich das gepinnte Original-JAR plus seine 43
geprüften Libraries. Er bootstrapped die Builtinregistries, erzeugt den echten
Vanillapack über `ServerPacksSource.createVanillaPackSource`, lädt die Itemtags
mit `TagLoader.loadTagsForExistingRegistries` und bindet jedes `PendingTags`
mit `apply`. Anschließend decodiert `Recipe.CODEC` die unveränderten Rezepte mit
Original-RegistryOps. Exportiert werden tatsächlich decodierte getrimmte
Patterns, sämtliche Ingredientmitgliedschaften, Item-Craftingremainders und
die Original-Streamcodecbytes fester Resultpatches. Die 16 Stewresultate bleiben
geprüfte serverautorisierte Opaque-Komponenten. Python erzeugt daraus private
Rustdeskriptoren und Differentialfixtures; keine neuen Rustdependencies oder
Lockfileänderungen. Bundle/Inner/Mapping/Recipehash, Javaoptionen, Helper-shadow,
Original- und Helper-CodeSources sowie unveränderte Inputs werden geprüft.

Öffentliche API mit privaten Feldern: `NormalRecipes::builtin()` liefert einen
validierten unveränderlichen Katalog; `iter` und `recipe(name)` ermöglichen die
explizite Rezeptwahl. `CraftingGrid::new(side, borrowed_stacks)` akzeptiert genau
2×2 oder 3×3 und prüft alle Stacks ohne Kopien. `NormalRecipe::matches` prüft
getrimmtes shaped Matching samt horizontaler Spiegelung oder ein echtes
augmenting-path Matching mit höchstens neun Zutaten/Slots. `plan_one` liefert
bei Match einen vollständigen `SingleCraftPlan` mit festem Output,
ursprünglich indizierten `grid_after`, `crafting_remainders` und Verbrauchsmaske.
Kein Match liefert `None`; malformed Inputs/Bestand/Resourceverletzungen
liefern typisierte Fehler. Paper-Managerpriorität/Lastrecipecache werden nicht
durch eine unbelegte Firstmatch-API ersetzt. ExactChoice ist in diesem
geschlossenen normalen Katalog nicht darstellbar und bleibt separat.

Alle Eingabestacks werden verlustfrei über den bestehenden Persistencepreflight
admitted. Present-air wird ausdrücklich mit `PresentAirUnsupported` abgewiesen:
Vanillas `isEmpty` verwirft air, während das vorhandene Rustmodell einen
Present-air-Stack erhalten kann. Dadurch erfolgt keine versteckte
Normalisierung/Komponentenvernichtung. Komponenten beeinflussen normale
Ingredientmembership nicht; feste Resultate übernehmen keine Eingabepatches.
Jede belegte Position verliert genau ein Item. Verbleibende Patches behalten
ihre geordnete Repräsentation; count0 wird canonicalempty. Der vorhandene
`transfer::copy_at_count` wird nur crateintern wiederverwendet. Craftingreste
entstehen aus dem tatsächlichen Itemremainder mit Standardpatch und werden
separat gehalten; Sim entscheidet anschließend über Merge/Inventar/Drop.

Explizite Ressourcenlimits: maximal 1024 Rezepte, 1024 unterschiedliche
Ingredientsets, 2048 Mitglieder je Set, insgesamt 131072 Mitgliedseinträge;
höchstens neun Slots, höchstens neun Komponenten-Resultpatches und 8192 Bytes
je festem Komponentenpayload. Der tatsächliche Census wird im Manifest
festgehalten und muss innerhalb dieser Limits liegen. Eingabegrid maximal
512 KiB schema-1-Encodedbestand und 1 MiB logischer Clonebestand; kompletter
Plan maximal 1 MiB Encodedbestand und 2 MiB logischer Clonebestand. Der
bestehende bounded Persistencepreflight liefert die Encodedgrößen, ergänzt
durch ein crateinternes allokationsfreies Cloneaccounting über validierte
Component-/NBT-Bäume. Dieses zählt geklonte Element-/Node-/String-/Arraylängen
und Inlinewerte; es behauptet keine RSS-/Allocatorgarantie. Alle Planpostimages,
Resultate und Restitems werden vor der ersten Kopie insgesamt geprüft.
Der Katalog prüft Deskriptor-/Set-/Patch-/Namensgrenzen vor Materialisierung.

Abnahme: frischer normal/-O-Originalexport; alle 968 Resultate und Remainders;
sämtliche zulässigen 2×2-/3×3-Offsets/Mirrors, Innenlücken, fehlende/zusätzliche
Zutaten, jedes tatsächlich alternative Tagmitglied, Count1/Maximum,
komponententragende Inputs, alle Stewoutputs sowie Present-air-Admisson.
Ein unabhängiges erschöpfendes Permutationsmodell prüft das bipartite Matching,
und eine getrennte Mengen-/Komponentenbilanz prüft Planverbrauch und Reste.
Negative Hash-/Herkunfts-/Descriptor-/Budgetproben und unveränderte Inputs bei
Refusal ergänzen die vollständigen Items-Debug-/Release-/Clippy-/Fmt-/Rustdoc-
Checks. Danach exakter Freeze für unabhängigen Review. Dieser reine Katalog
und Plan aktiviert weder GUI-/Outputklicks noch Shiftcraft/Recipebook/Durability.

## Implementierung und Autorenabnahme

Implementiert ausschließlich in `ferrumc-items`: geschlossener immutable
`NormalRecipes`-Katalog, expliziter `NormalRecipe`, borrowed `CraftingGrid`,
trim/mirror Matcher, vollständiges augmenting-path Shapelessmatching und
konservativer `SingleCraftPlan`. Originalslots, count−1, feste Outputs und
separate Remainders bleiben erhalten. Present-air ist eine typed Refusal.
Persistencegröße und logischer Clonebestand werden allokationsfrei geprüft;
`copy_at_count` wird crateintern wiederverwendet. Bestehende Transfer- und
Persistenceformen/-semantik bleiben unverändert. Für strict öffentliche
Rustdoc wurde außerdem ein bereits redundantes Linkziel in `component.rs`
entfernt; keine Runtimeänderung daran.

Der actual Original-JAR-Producer bindet echte Vanillapacktags über den
Original-TagLoader und `PendingTags.apply`, decodiert `Recipe.CODEC`, ruft
Match/Assembly/Remainder/Copy-Methoden auf und exportiert echte Streamcodecbytes.
Er erzeugt **968 Rezepte**, **479 Ingredientsets**, **1506 Mitgliedschaften**
(größtes Set44), **25 Itemtagreferenzen**, **1416 Registrypins**, alle fünf
Item-Craftingremainders und sämtliche16 komponententragenden Stewoutputs.
Es wurden keine Itemtagheuristiken, handgeschriebenen Rezeptlisten,
Rustdependencies, Lockfile-, Git- oder anderen Crateänderungen eingeführt.

Frische Exporte in `.codex-tmp/srv05_1-final-normal` und
`.codex-tmp/srv05_1-final-optimized` sind bytegleich: RawJSONL, Deskriptoren,
TSVcases und vollständiges Manifest. **21855 tatsächliche Fälle** ergeben
**15119 Matches** und **6736 Refusals**. Jeder normale Rezeptoutput und alle
Remainders werden tatsächlich geprüft; sämtliche shaped Offsets/Mirrors,
jede Ingredientalternative, fehlende/zusätzliche Zutaten, Count1/Maximum und
Komponenteninputs sind enthalten. Ein getrenntes erschöpfendes
Assignmentsuchmodell sowie Mengen-/Patchrepräsentations-/Slotbilanzen prüfen
die Rustpläne. Weitere Tests decken Descriptor-/Component-/Aggregategrenzen,
Present-air, ungültige Models, Greedyfallen, Floatbits/NBTduplikate und
unveränderte Inputs ab.

Rust/Cargo1.99.0, Edition2021, `--locked --offline`, Jobs2 und eigener Target
`.codex-tmp/srv05_1-author-target`: **84/84 Debug**, **84/84 Release**, jeweils
**3/3 Doctests**, strict all-target Clippy `-D warnings`, package Fmtcheck und
öffentliche Rustdoc `RUSTDOCFLAGS=-D warnings` bestehen. Logs und kompakte
Evidenz stehen in `.codex-tmp/srv05_1-checks`. Producernegativprüfungen bestehen
normal und `-O` jeweils8/8: Digest-/Symlink-/Shadow-/Descriptor-/Census-/Case-
Refusals, tatsächlich ausgeführte Original-/Helper-CodeSource-Refusals und alle
drei Javaoption-Injectionguards. Referenz-/Producer-/Fixturequellen und eine
19-Dateien-Original/Applied/Patchmatrix liegen in der Itemscrate.

Evidenzpins:

- RawJSONL SHA256
  `6110a3130294599a11d480e8eb0c3f74ff7854f208ece81514a22090fe1cbd7c`.
- CaseTSV SHA256
  `64f90f5b3cff75eb6885993f33cbcb6b8c9992e0f36a63f7a9ed55041fbd6649`.
- Deskriptoren SHA256
  `fbdc704830a75bcf457457fbc5a54be67a9e6b560ab1eeb1cdf3275ad5b70f3c`.
- Manifest SHA256
  `05c3bba8aacebc01af9fd58a2951c15c25fa714514574ce68b108e70a641d4a3`.

Der exakte Dateifreeze steht in `.codex-tmp/srv05_1-author-freeze.sha256`,
die separaten Check-/Reexportartefakte in
`.codex-tmp/srv05_1-author-evidence.sha256`. Nach der Übergabe keine
Autorenänderung an diesen produktiven Dateien ohne koordinierte Reviewkorrektur.
Unabhängiger Review und Rootintegration sind ausdrücklich noch offen.
