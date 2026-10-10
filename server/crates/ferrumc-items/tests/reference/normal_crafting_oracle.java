package ferrumc.reference;

import com.google.gson.*;
import com.mojang.serialization.*;
import io.netty.buffer.*;
import java.io.PrintWriter;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.stream.Stream;

/** Calls the pinned original codecs, tag loader, recipes and stack stream codecs. */
final class NormalCraftingOracle {
    private static final Gson GSON = new Gson();
    private static Object access, items, componentTypes;
    private static DynamicOps<JsonElement> ops;
    private static Codec<Object> recipes, stacks;
    private static final Map<String, Object> itemByName = new TreeMap<>();
    private static final Map<String, Integer> itemIds = new TreeMap<>();
    private static final Map<String, Integer> itemMaximums = new TreeMap<>();
    private static final Map<String, Object> remainderByName = new TreeMap<>();
    private static final List<JsonObject> cases = new ArrayList<>();
    private static Class<?> type(String name) throws Exception { return Class.forName(name); }
    private static Object field(Class<?> owner, Object object, String name) throws Exception {
        Field f = owner.getDeclaredField(name); f.setAccessible(true); return f.get(object);
    }
    private static Object field(String owner, String name) throws Exception { return field(type(owner), null, name); }
    private static Object call(String owner, Object target, String name, Class<?>[] signature, Object... args) throws Exception {
        return type(owner).getMethod(name, signature).invoke(target, args);
    }
    private static void require(boolean value, String message) { if (!value) throw new IllegalStateException(message); }
    private static JsonArray array(Collection<?> values) { return GSON.toJsonTree(values).getAsJsonArray(); }
    private static String itemName(Object item) throws Exception {
        return call("jy", items, "b", new Class<?>[]{Object.class}, item).toString();
    }
    private static boolean empty(Object stack) throws Exception {
        return (boolean) call("dcv", stack, "f", new Class<?>[]{});
    }
    private static byte[] encoded(Object codec, Object value) throws Exception {
        ByteBuf raw = Unpooled.buffer();
        try {
            Object buf = type("wx").getConstructor(ByteBuf.class, type("jz")).newInstance(raw, access);
            call("zo", codec, "encode", new Class<?>[]{Object.class, Object.class}, buf, value);
            byte[] bytes = new byte[raw.readableBytes()]; raw.readBytes(bytes); return bytes;
        } finally { raw.release(); }
    }
    private static String slot(Object stack) throws Exception { return HexFormat.of().formatHex(encoded(field("dcv", "h"), stack)); }
    private static JsonObject stackRecord(Object stack) throws Exception {
        JsonObject out = new JsonObject(); out.addProperty("slot", slot(stack));
        if (empty(stack)) { out.addProperty("empty", true); return out; }
        Object item = call("dcv", stack, "h", new Class<?>[]{});
        out.addProperty("item", itemName(item));
        out.addProperty("count", (int) call("dcv", stack, "M", new Class<?>[]{}));
        Object patch = call("dcv", stack, "d", new Class<?>[]{});
        Set<?> entries = (Set<?>) call("ko", patch, "b", new Class<?>[]{});
        JsonArray additions = new JsonArray(), removals = new JsonArray();
        for (Object entry : entries) {
            Map.Entry<?, ?> e = (Map.Entry<?, ?>) entry;
            int id = (int) call("jy", componentTypes, "a", new Class<?>[]{Object.class}, e.getKey());
            Optional<?> value = (Optional<?>) e.getValue();
            if (value.isEmpty()) { removals.add(id); continue; }
            JsonObject c = new JsonObject(); c.addProperty("id", id);
            c.addProperty("name", call("jy", componentTypes, "b", new Class<?>[]{Object.class}, e.getKey()).toString());
            Object codec = call("kp", e.getKey(), "e", new Class<?>[]{});
            c.addProperty("payload", HexFormat.of().formatHex(encoded(codec, value.get()))); additions.add(c);
        }
        out.add("added", additions); out.add("removed", removals); return out;
    }
    private static Object stack(String name, int count, boolean components) throws Exception {
        if (name == null) return field("dcv", "l");
        JsonObject json = new JsonObject(); json.addProperty("id", name); json.addProperty("count", count);
        if (components) {
            JsonObject data = new JsonObject(); data.addProperty("probe", 1);
            JsonObject patch = new JsonObject(); patch.add("minecraft:custom_data", data); json.add("components", patch);
        }
        return stacks.parse(ops, json).getOrThrow();
    }
    private static List<String> members(Object ingredient) throws Exception {
        List<String> names = new ArrayList<>();
        try (Stream<?> holders = (Stream<?>) call("dhi", ingredient, "a", new Class<?>[]{})) {
            for (Object holder : holders.toList()) {
                Object item = call("jl", holder, "a", new Class<?>[]{}); names.add(itemName(item));
            }
        }
        Collections.sort(names); require(!names.isEmpty(), "empty actual ingredient"); return names;
    }
    private static void caseRecord(String name, Object recipe, String label, int side, List<String> names, boolean max, boolean components) throws Exception {
        List<Object> grid = new ArrayList<>(); JsonArray input = new JsonArray();
        for (String item : names) {
            int count = item == null ? 0 : (max ? itemMaximums.get(item) : 1);
            grid.add(stack(item, count, components));
            if (item == null) input.add(JsonNull.INSTANCE);
            else { JsonArray v = new JsonArray(); v.add(itemIds.get(item)); v.add(count); v.add(components); input.add(v); }
        }
        Object positioned = call("dha", null, "b", new Class<?>[]{int.class, int.class, List.class}, side, side, grid);
        Object crafting = call("dha$a", positioned, "a", new Class<?>[]{});
        boolean matches = (boolean) call("dhm", recipe, "a", new Class<?>[]{type("dhs"), type("dmu")}, crafting, null);
        JsonObject row = new JsonObject(); row.addProperty("kind", "case"); row.addProperty("recipe", name);
        row.addProperty("label", label); row.addProperty("side", side); row.add("input", input); row.addProperty("matches", matches);
        if (matches) {
            Object result = call("dhm", recipe, "a", new Class<?>[]{type("dhs"), type("jn$a")}, crafting, access);
            row.addProperty("output", slot(result));
            List<?> rests = (List<?>) call("dhb", recipe, "a", new Class<?>[]{type("dha")}, crafting);
            int width = (int) call("dha", crafting, "f", new Class<?>[]{});
            int left = (int) call("dha$a", positioned, "b", new Class<?>[]{});
            int top = (int) call("dha$a", positioned, "c", new Class<?>[]{});
            List<String> after = new ArrayList<>(), remaining = new ArrayList<>(Collections.nCopies(side * side, "00"));
            for (Object original : grid) {
                int count = empty(original) ? 0 : (int) call("dcv", original, "M", new Class<?>[]{});
                Object rest = call("dcv", original, "c", new Class<?>[]{int.class}, Math.max(0, count - 1));
                after.add(slot(rest));
            }
            for (int i = 0; i < rests.size(); i++) remaining.set((top + i / width) * side + left + i % width, slot(rests.get(i)));
            row.add("after", array(after)); row.add("remainders", array(remaining));
        }
        cases.add(row);
    }
    @SuppressWarnings("unchecked")
    public static void main(String[] args) throws Exception {
        require(args.length == 4, "expected fixture/output/inner/classes");
        Path inner = Path.of(args[2]).toRealPath(), classes = Path.of(args[3]).toRealPath();
        for (String name : new String[]{"ac", "amg", "mm", "jy", "jy$a", "jz", "jn$a", "jl", "jp", "ko", "kp", "wx", "amc", "axc", "axh", "axo", "awd", "baf", "dha", "dha$a", "dhb", "dhm", "dhi", "dia", "dib", "dic", "dcv", "dcr", "cuy", "cuz"}) {
            Class<?> original = Class.forName(name, false, NormalCraftingOracle.class.getClassLoader());
            Path origin = Path.of(original.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
            require(origin.equals(inner), "wrong original CodeSource: " + name + ": " + origin);
        }
        require(Path.of(NormalCraftingOracle.class.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(classes), "wrong helper CodeSource");
        call("ac", null, "a", new Class<?>[]{}); call("amg", null, "a", new Class<?>[]{});
        items = field("mm", "g"); componentTypes = field("mm", "am");
        access = call("jz", null, "a", new Class<?>[]{type("jy")}, field("mm", "aJ"));
        Object vanilla = call("axc", null, "b", new Class<?>[]{});
        Object manager = type("axh").getConstructor(type("awd"), List.class).newInstance(field("awd", "b"), List.of(vanilla));
        try {
            List<?> pending = (List<?>) call("baf", null, "a", new Class<?>[]{type("axo"), type("jz")}, manager, access);
            require(!pending.isEmpty(), "no actual tags loaded");
            for (Object tags : pending) call("jy$a", tags, "d", new Class<?>[]{});
            ops = (DynamicOps<JsonElement>) call("jn$a", access, "a", new Class<?>[]{DynamicOps.class}, JsonOps.INSTANCE);
            recipes = (Codec<Object>) field("dhm", "a"); stacks = (Codec<Object>) field("dcv", "b");
            List<JsonObject> records = new ArrayList<>();
            for (Object item : (Iterable<?>) items) {
                String name = itemName(item); itemByName.put(name, item);
                int id = (int) call("jy", items, "a", new Class<?>[]{Object.class}, item); itemIds.put(name, id);
                int maximum = (int) call("dcr", item, "g", new Class<?>[]{}); itemMaximums.put(name, maximum);
                Object remainder = call("dcr", item, "i", new Class<?>[]{}); remainderByName.put(name, remainder);
                JsonObject row = stackRecord(remainder); row.addProperty("kind", "item"); row.addProperty("source", name); row.addProperty("id", id); row.addProperty("max", maximum); records.add(row);
            }
            JsonObject fixture = JsonParser.parseString(Files.readString(Path.of(args[0]))).getAsJsonObject().getAsJsonObject("entries");
            int shaped = 0, shapeless = 0;
            for (Map.Entry<String, JsonElement> entry : new TreeMap<>(fixture.asMap()).entrySet()) {
                JsonObject json = entry.getValue().getAsJsonObject(); String serializer = json.get("type").getAsString();
                if (!serializer.equals("minecraft:crafting_shaped") && !serializer.equals("minecraft:crafting_shapeless")) continue;
                Object recipe = recipes.parse(ops, json).getOrThrow();
                boolean shapedRecipe = serializer.endsWith("shaped"); if (shapedRecipe) shaped++; else shapeless++;
                int width, height; List<List<String>> ingredientSets = new ArrayList<>();
                if (shapedRecipe) {
                    width = (int) call("dia", recipe, "k", new Class<?>[]{}); height = (int) call("dia", recipe, "l", new Class<?>[]{});
                    for (Object ingredient : (List<?>) call("dia", recipe, "f", new Class<?>[]{})) {
                        Optional<?> optional = (Optional<?>) ingredient;
                        ingredientSets.add(optional.isEmpty() ? List.of() : members(optional.get()));
                    }
                } else {
                    List<?> ingredientList = (List<?>) field(type("dic"), recipe, "g"); width = ingredientList.size(); height = 1;
                    for (Object ingredient : ingredientList) ingredientSets.add(members(ingredient));
                }
                Object result = field(type(shapedRecipe ? "dia" : "dic"), recipe, shapedRecipe ? "e" : "f");
                JsonObject row = new JsonObject(); row.addProperty("kind", "recipe"); row.addProperty("name", entry.getKey());
                row.addProperty("shaped", shapedRecipe); row.addProperty("width", width); row.addProperty("height", height);
                row.add("ingredients", GSON.toJsonTree(ingredientSets)); row.add("result", stackRecord(result)); records.add(row);
                List<String> baseline = new ArrayList<>(); for (List<String> set : ingredientSets) baseline.add(set.isEmpty() ? null : set.getFirst());
                for (int side : new int[]{2, 3}) {
                    if (shapedRecipe && (width > side || height > side) || !shapedRecipe && baseline.size() > side * side) continue;
                    int xCount = shapedRecipe ? side - width + 1 : 1, yCount = shapedRecipe ? side - height + 1 : 1;
                    for (int y = 0; y < yCount; y++) for (int x = 0; x < xCount; x++) for (int mirror = 0; mirror < 2; mirror++) {
                        List<String> grid = new ArrayList<>(Collections.nCopies(side * side, null));
                        for (int i = 0; i < baseline.size(); i++) {
                            int position = shapedRecipe ? (y + i / width) * side + x + (mirror == 0 ? i % width : width - 1 - i % width) : (mirror == 0 ? i : side * side - 1 - i);
                            grid.set(position, baseline.get(i));
                        }
                        caseRecord(entry.getKey(), recipe, "offset-" + x + "-" + y + "-mirror-" + mirror, side, grid, false, false);
                        if (x != 0 || y != 0 || mirror != 0) continue;
                        caseRecord(entry.getKey(), recipe, "max-count", side, grid, true, false);
                        caseRecord(entry.getKey(), recipe, "components", side, grid, true, true);
                        for (int i = 0; i < grid.size(); i++) if (grid.get(i) != null) {
                            List<String> missing = new ArrayList<>(grid); missing.set(i, null);
                            caseRecord(entry.getKey(), recipe, "missing-" + i, side, missing, true, false);
                        }
                        List<String> extra = new ArrayList<>(grid); int empty = extra.indexOf(null);
                        extra.set(empty < 0 ? 0 : empty, "minecraft:barrier");
                        caseRecord(entry.getKey(), recipe, "extra-or-wrong", side, extra, true, false);
                    }
                }
                List<String> grid = new ArrayList<>(Collections.nCopies(9, null));
                for (int i = 0; i < baseline.size(); i++) grid.set(shapedRecipe ? i / width * 3 + i % width : i, baseline.get(i));
                for (int i = 0; i < ingredientSets.size(); i++) for (String alternative : ingredientSets.get(i)) {
                    List<String> alternate = new ArrayList<>(grid); alternate.set(shapedRecipe ? i / width * 3 + i % width : i, alternative);
                    caseRecord(entry.getKey(), recipe, "member-" + i + "-" + itemIds.get(alternative), 3, alternate, false, false);
                }
            }
            require(shaped == 673 && shapeless == 295, "normal recipe census mismatch");
            records.addAll(cases);
            try (PrintWriter out = new PrintWriter(Files.newBufferedWriter(Path.of(args[1]), StandardCharsets.UTF_8))) {
                for (JsonObject record : records) out.println(GSON.toJson(record));
            }
            System.out.println("actual original: " + shaped + " shaped, " + shapeless + " shapeless, " + cases.size() + " cases");
        } finally { call("axh", manager, "close", new Class<?>[]{}); }
    }
}
