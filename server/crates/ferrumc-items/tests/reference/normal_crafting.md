# Normal crafting original reference

Minecraft **1.21.8**, protocol **772**, DataVersion **4440**. Paper reference:
build60, commit `29c8822d90899c89d2689338e81a98f690bcba12` (see
`docs/PAPER_REFERENCE.md`). Only original shaped/shapeless builtins are produced.

The complete original and applied Paper crafting sources were read:
`ShapedRecipe`, `ShapedRecipePattern`, `ShapelessRecipe`, `Ingredient`,
`CraftingInput`, `CraftingRecipe`, `Recipe`, `RecipeManager`, `RecipeMap`,
`PlacementInfo`, `StackedContents`, `StackedItemContents`, and `ResultSlot`.
Their sourcepatches and featurepatch0020 (exact ingredients) were read in full.
The applied `ItemOrExact` and `StackedContentsExtrasMap` confirm semantic
resolved-component matching for ExactChoice. Relevant complete `ItemStack`
codec/constructor/emptiness/copy/component methods and `Item.getCraftingRemainder`
were read, together with the entire ItemStack sourcepatch. There is no Item
sourcepatch at this pin; its remainder method remains unchanged.
`TagLoader`, `ServerPacksSource`, registry-context and component-stream-codec
callees were checked for the actual producer. The source matrix records hashes
of original and applied files and associated patches, without vendoring Java.

Original input and pattern trimming preserve internal holes. A shaped recipe
accepts the exact trimmed rectangle and horizontal mirror. `CraftingInput`
accounts at most one item per occupied position. Normal ingredients test
item/tag membership; shapeless recipes use complete assignment. Assembly copies
the fixed result; default rests come from `Item.getCraftingRemainder` and not
the input patch. Paper's recipe manager prioritizes the last matching recipe
unless its last-recipe cache already matches. This pure API therefore requires
explicit selection and does not assert first-match or manager behavior.

The helper calls original code without booting a Minecraft server:

1. Verify original/own CodeSources, then initialize SharedConstants/Bootstrap.
2. Obtain original builtins and `RegistryAccess.fromRegistryOfRegistries`.
3. Create the real vanilla resource pack and `MultiPackResourceManager`.
4. Run `TagLoader.loadTagsForExistingRegistries` and each `PendingTags.apply`.
5. Decode every normal exported recipe with original `Recipe.CODEC` and
   registry-aware JsonOps; export actual patterns and ingredient members.
6. Call actual `matches`, `assemble`, `getRemainingItems`, item remainder and
   count-copy methods, and actual result/component stream codecs.

The original item ceiling comes from `Item.getDefaultMaxStackSize`. In
particular, an air default ItemStack is semantically empty and reports a stack
ceiling of1; the air Item's actual default component ceiling is64. Runtime
crafting rejects present-air explicitly and leaves the existing Rust model intact.
Original JsonOps-to-NBT produces TAG_Byte for the custom-data probe value1.
The differential Rust inputs use that actual representation, and retained
patches are independently compared through lossless persistence bytes.

Reproduce offline, supplying the pinned mappings file in each output directory:

```text
python3 tests/reference/generate_normal_crafting.py ORIGINAL_CACHE JAVA21_HOME OUTPUT
python3 -O tests/reference/generate_normal_crafting.py ORIGINAL_CACHE JAVA21_HOME OUTPUT_OPTIMIZED
```

Run from the Itemscrate, or adjust script paths. Use fresh output directories.
The generator verifies bundle/inner/recipe/mapping hashes, all43 original
libraries, Java21, forbidden option injection, helper classpath shadowing,
30 original class CodeSources, fresh helper origin, and all inputs again after
execution. It compiles with annotation processing disabled. Descriptors and
TSV rows are derived from actual results; Python contains no tag expansion,
recipe matching, output assembly, or remainder implementation.

Current census:968 recipes,479 unique sets,1506 memberships, largest set44,
25 tag references,1416 item pins, five remainder items,16 component outputs,
21855 actual cases. Cases cover every allowable shaped 2×2/3×3 offset/mirror,
shapeless forward/reverse positions, every member of every ingredient position,
count1/default maximum, components, missing ingredients and extra/wrong items.
All968 recipes have actual successful assembly and remainder cases. Rust adds
an independently formulated exhaustive assignment model and per-item quantity,
mask, component-representation and original-slot balance checks.

No live Paper/client survival, output-click, event, recipebook, persistence
transaction, plugin or special-recipe acceptance is implied by this corpus.
