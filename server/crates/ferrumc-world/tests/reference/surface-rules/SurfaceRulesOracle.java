package ferrumc.surfaceruleoracle;

import java.io.BufferedWriter;
import java.io.InputStream;
import java.lang.reflect.*;
import java.net.JarURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;
import java.util.concurrent.CompletableFuture;
import java.util.function.Function;
import java.util.stream.Stream;

/**
 * Original reflection/delegation only. Every rule outcome, condition result and
 * lazily computed context value is read from actual original objects. The helper
 * implements no surface condition, noise, random, anchor, temperature or zoom logic.
 */
public final class SurfaceRulesOracle {
    private static Properties symbols;
    private static String mode;
    private static Path inner, helperRoot;
    private static final Map<String, Class<?>> types = new TreeMap<>();
    private static final Map<String, String> origins = new TreeMap<>();
    private static final Set<String> paperOnly = new TreeSet<>();
    private static final Map<String, Object> holders = new TreeMap<>();
    private static final List<String> roster = new ArrayList<>();
    private static Object lookup, biomeRegistry, biomesStatus, referencesStatus, worldSurfaceWg;
    private static Object unsafe;
    private static Method allocate;
    private static BufferedWriter rows;
    private static MessageDigest allCalls;
    private static long rowCount, gCalls, gRecorded, cRows, crossCompared, crossMismatch, flatCompared, flatDiffer, dRows;
    private static String tryApplyName, testName;
    private static java.util.concurrent.ExecutorService originalBackground;

    /** Runs on the thread whose thread-local RTree history createBiomes actually uses. */
    private static <T> T generationLane(java.util.concurrent.Callable<T> operation) throws Exception {
        if (mode.equals("paper")) return operation.call();
        return originalBackground.submit(operation).get();
    }
    private static void coldHistory(Object generator) throws Exception {
        Object source = call("ChunkGenerator", "getBiomeSource", generator);
        if (!type("MultiNoise").isInstance(source)) return;
        Object tree = field("ParameterList", "index", call("MultiNoise", "parameters", source));
        ThreadLocal<?> last = (ThreadLocal<?>) field("RTree", "lastResult", tree);
        generationLane(() -> { last.remove(); return null; });
    }

    // ---------------------------------------------------------------- reflection
    private static void require(boolean condition, String message) {
        if (!condition) throw new IllegalStateException(message);
    }
    private static String property(String name) {
        String result = symbols.getProperty(mode + "." + name);
        require(result != null, "Missing exact official symbol " + name);
        return result;
    }
    private static Class<?> type(String alias) throws Exception {
        Class<?> result = types.get(alias);
        if (result == null) {
            result = Class.forName(property("class." + alias), false, SurfaceRulesOracle.class.getClassLoader());
            origin(result, inner, "original/" + alias);
            types.put(alias, result);
        }
        return result;
    }
    private static String canonical(Class<?> t) {
        for (String alias : types.keySet()) if (types.get(alias) == t) return symbols.getProperty("paper.class." + alias);
        return t.getName();
    }
    private static final Map<String, Method> methods = new HashMap<>();
    private static final Map<String, Field> fields = new HashMap<>();
    private static Method method(String alias, String name, Class<?>... parameters) throws Exception {
        StringJoiner args = new StringJoiner(",");
        for (Class<?> p : parameters) args.add(canonical(p));
        String symbol = "method." + alias + "." + name + "(" + args + ")";
        Method cached = methods.get(symbol);
        if (cached != null) return cached;
        Class<?> owner = type(alias);
        String actual = property(symbol);
        Method result;
        try { result = owner.getDeclaredMethod(actual, parameters); }
        catch (NoSuchMethodException absent) { result = owner.getMethod(actual, parameters); }
        result.setAccessible(true);
        methods.put(symbol, result);
        return result;
    }
    private static Object call(String alias, String name, Object receiver) throws Exception {
        return method(alias, name).invoke(receiver);
    }
    private static Field fieldRef(String alias, String name) throws Exception {
        String symbol = alias + "." + name;
        Field cached = fields.get(symbol);
        if (cached != null) return cached;
        String actual = paperOnly.contains(alias) ? name : property("field." + alias + "." + name);
        Field result = type(alias).getDeclaredField(actual);
        result.setAccessible(true);
        fields.put(symbol, result);
        return result;
    }
    private static Object field(String alias, String name, Object receiver) throws Exception {
        return fieldRef(alias, name).get(receiver);
    }
    private static int intField(String alias, String name, Object receiver) throws Exception {
        return fieldRef(alias, name).getInt(receiver);
    }
    private static long longField(String alias, String name, Object receiver) throws Exception {
        return fieldRef(alias, name).getLong(receiver);
    }
    private static void origin(Class<?> t, Path expected, String label) throws Exception {
        require(t.getProtectionDomain().getCodeSource() != null, "Missing actual CodeSource " + label);
        Path actual = Path.of(t.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
        require(Files.isSameFile(actual, expected), "Wrong actual CodeSource " + label + ": " + t.getName());
        String previous = origins.putIfAbsent(label, t.getName() + "\t" + (expected.equals(inner) ? "$original-inner" : expected.equals(helperRoot) ? "$fresh-helper" : expected.toString()));
        require(previous == null || previous.equals(origins.get(label)), "Origin label changed");
    }
    private static void actualOrigin(Object object, String label) throws Exception {
        require(object != null, "Missing actual object " + label);
        origin(object.getClass(), inner, "actual/" + label);
    }
    private static String key(Object holder) throws Exception {
        return call("ResourceKey", "location", call("Reference", "key", holder)).toString();
    }
    private static Object resourceKey(String registryName, String resource) throws Exception {
        Object location = method("ResourceLocation", "parse", String.class).invoke(null, resource);
        return method("ResourceKey", "create", type("ResourceKey"), type("ResourceLocation")).invoke(null, field("Registries", registryName, null), location);
    }
    private static Object registryLookup(String registryName) throws Exception {
        return method("Provider", "lookupOrThrow", type("ResourceKey")).invoke(lookup, field("Registries", registryName, null));
    }
    private static Throwable unwrap(Throwable t) {
        while (t instanceof InvocationTargetException || t instanceof java.util.concurrent.CompletionException || t instanceof java.lang.reflect.UndeclaredThrowableException) t = t.getCause();
        return t;
    }
    private static String exception(Throwable t) {
        t = unwrap(t);
        StringBuilder result = new StringBuilder();
        for (int depth = 0; t != null && depth < 6; depth++, t = t.getCause()) {
            if (depth > 0) result.append(" <- ");
            String name = t.getClass().getName();
            for (String alias : types.keySet()) if (types.get(alias) == t.getClass()) name = symbols.getProperty("paper.class." + alias);
            result.append(name).append(": ").append(String.valueOf(t.getMessage()).replace('\n', ' ').replace('\r', ' ').replace('\t', ' '));
        }
        return result.toString();
    }
    private static String cells(Object... values) {
        StringJoiner result = new StringJoiner("\t");
        for (Object value : values) {
            String text = String.valueOf(value);
            require(!text.contains("\t") && !text.contains("\n") && !text.contains("\r"), "Unframed output cell");
            result.add(text);
        }
        return result.toString();
    }
    private static void line(BufferedWriter out, Object... values) throws Exception {
        out.write(cells(values));
        out.newLine();
    }
    private static List<String[]> lines(Path path) throws Exception {
        List<String[]> result = new ArrayList<>();
        List<String> input = Files.readAllLines(path);
        require(!input.isEmpty(), "Missing input header " + path);
        for (int i = 1; i < input.size(); i++) result.add(input.get(i).split("\t", -1));
        return result;
    }
    private static Object allocate(Class<?> t) throws Exception {
        return allocate.invoke(unsafe, t);
    }

    // ---------------------------------------------------------------- original data
    private static String stateText(Object state) throws Exception {
        Object block = call("StateBase", "getBlock", state);
        Object name = method("Registry", "getKey", Object.class).invoke(field("BuiltInRegistries", "BLOCK", null), block);
        Map<?, ?> values = (Map<?, ?>) call("StateHolder", "getValues", state);
        TreeMap<String, String> sorted = new TreeMap<>();
        for (Map.Entry<?, ?> e : values.entrySet()) {
            String property = (String) call("Property", "getName", e.getKey());
            String value = (String) method("Property", "getName", Comparable.class).invoke(e.getKey(), e.getValue());
            sorted.put(property, value);
        }
        StringJoiner props = new StringJoiner(",", "[", "]");
        for (Map.Entry<String, String> e : sorted.entrySet()) props.add(e.getKey() + "=" + e.getValue());
        boolean isDefault = call("Block", "defaultBlockState", block) == state;
        return name + (sorted.isEmpty() ? "" : props.toString()) + "|" + method("Block", "getId", type("BlockState")).invoke(null, state) + "|" + (isDefault ? "default" : "nondefault");
    }
    private static String anchorText(Object anchor) throws Exception {
        if (type("Absolute").isInstance(anchor)) return "absolute:" + intField("Absolute", "y", anchor);
        if (type("AboveBottom").isInstance(anchor)) return "above_bottom:" + intField("AboveBottom", "offset", anchor);
        if (type("BelowTop").isInstance(anchor)) return "below_top:" + intField("BelowTop", "offset", anchor);
        throw new IllegalStateException("Unknown original anchor " + anchor.getClass());
    }
    private static String doubleText(double value) {
        return Double.toString(value) + "/" + Long.toHexString(Double.doubleToRawLongBits(value));
    }

    /** Canonical preorder census of a genuine original RuleSource object graph. */
    private static final class TreeIndex {
        final List<String> lines = new ArrayList<>();
        final IdentityHashMap<Object, Integer> first = new IdentityHashMap<>();
        final Map<Integer, List<Object>> anchors = new TreeMap<>();
        int add(Object source, int parent, int depth, String kind, String attrs) {
            int id = lines.size();
            Integer shared = first.putIfAbsent(source, id);
            lines.add(id + "\t" + parent + "\t" + depth + "\t" + kind + "\t" + attrs + "\t" + (shared == null ? "-" : shared.toString()));
            return id;
        }
    }
    private static void walkRule(TreeIndex t, Object rs, int parent, int depth) throws Exception {
        if (type("SequenceSrc").isInstance(rs)) {
            List<?> children = (List<?>) field("SequenceSrc", "sequence", rs);
            int id = t.add(rs, parent, depth, "sequence", "n=" + children.size());
            for (Object child : children) walkRule(t, child, id, depth + 1);
        } else if (type("TestSrc").isInstance(rs)) {
            int id = t.add(rs, parent, depth, "condition", "-");
            walkCondition(t, field("TestSrc", "ifTrue", rs), id, depth + 1);
            walkRule(t, field("TestSrc", "thenRun", rs), id, depth + 1);
        } else if (type("BlockSrc").isInstance(rs)) {
            t.add(rs, parent, depth, "block", stateText(field("BlockSrc", "resultState", rs)));
        } else if (type("BandlandsSrc").isInstance(rs)) {
            t.add(rs, parent, depth, "bandlands", "-");
        } else throw new IllegalStateException("Unknown original rule source " + rs.getClass());
    }
    private static void walkCondition(TreeIndex t, Object cs, int parent, int depth) throws Exception {
        if (type("NotCond").isInstance(cs)) {
            int id = t.add(cs, parent, depth, "not", "-");
            walkCondition(t, field("NotCond", "target", cs), id, depth + 1);
            return;
        }
        String kind, attrs;
        if (type("BiomeCond").isInstance(cs)) {
            StringJoiner names = new StringJoiner(",");
            for (Object k : (List<?>) field("BiomeCond", "biomes", cs)) names.add(call("ResourceKey", "location", k).toString());
            kind = "biome"; attrs = names.toString();
        } else if (type("NoiseCond").isInstance(cs)) {
            kind = "noise_threshold";
            attrs = call("ResourceKey", "location", field("NoiseCond", "noise", cs)) + ";" + doubleText((double) field("NoiseCond", "minThreshold", cs)) + ";" + doubleText((double) field("NoiseCond", "maxThreshold", cs));
        } else if (type("GradientCond").isInstance(cs)) {
            kind = "vertical_gradient";
            attrs = field("GradientCond", "randomName", cs) + ";" + anchorText(field("GradientCond", "trueAtAndBelow", cs)) + ";" + anchorText(field("GradientCond", "falseAtAndAbove", cs));
        } else if (mode.equals("paper") && type("FlatBedrockCond").isInstance(cs)) {
            kind = "paper_optionally_flat_bedrock";
            attrs = field("FlatBedrockCond", "randomName", cs) + ";" + anchorText(field("FlatBedrockCond", "trueAtAndBelow", cs)) + ";" + anchorText(field("FlatBedrockCond", "falseAtAndAbove", cs)) + ";is_roof=" + field("FlatBedrockCond", "isRoof", cs);
        } else if (type("YCond").isInstance(cs)) {
            kind = "y_above";
            attrs = anchorText(field("YCond", "anchor", cs)) + ";" + field("YCond", "surfaceDepthMultiplier", cs) + ";" + field("YCond", "addStoneDepth", cs);
        } else if (type("WaterCond").isInstance(cs)) {
            kind = "water";
            attrs = field("WaterCond", "offset", cs) + ";" + field("WaterCond", "surfaceDepthMultiplier", cs) + ";" + field("WaterCond", "addStoneDepth", cs);
        } else if (type("StoneCond").isInstance(cs)) {
            kind = "stone_depth";
            attrs = field("StoneCond", "offset", cs) + ";" + field("StoneCond", "addSurfaceDepth", cs) + ";" + field("StoneCond", "secondaryDepthRange", cs) + ";" + call("CaveSurface", "getSerializedName", field("StoneCond", "surfaceType", cs));
        } else if (type("SteepCond").isInstance(cs)) { kind = "steep"; attrs = "-"; }
        else if (type("HoleCond").isInstance(cs)) { kind = "hole"; attrs = "-"; }
        else if (type("AboveCond").isInstance(cs)) { kind = "above_preliminary_surface"; attrs = "-"; }
        else if (type("TemperatureCond").isInstance(cs)) { kind = "temperature"; attrs = "-"; }
        else throw new IllegalStateException("Unknown original condition source " + cs.getClass());
        int id = t.add(cs, parent, depth, kind, attrs);
        if (kind.equals("vertical_gradient")) t.anchors.put(id, List.of(field("GradientCond", "trueAtAndBelow", cs), field("GradientCond", "falseAtAndAbove", cs)));
        if (kind.equals("paper_optionally_flat_bedrock")) t.anchors.put(id, List.of(field("FlatBedrockCond", "trueAtAndBelow", cs), field("FlatBedrockCond", "falseAtAndAbove", cs)));
        if (kind.equals("y_above")) t.anchors.put(id, List.of(field("YCond", "anchor", cs)));
    }
    private static TreeIndex index(Object tree) throws Exception {
        TreeIndex result = new TreeIndex();
        walkRule(result, tree, -1, 0);
        return result;
    }
    private static Object decodeRule(Object element) throws Exception {
        Object ops = Class.forName("com.mojang.serialization.JsonOps").getField("INSTANCE").get(null);
        Object codec = field("RuleSource", "CODEC", null);
        Method parse = Class.forName("com.mojang.serialization.Decoder").getMethod("parse", Class.forName("com.mojang.serialization.DynamicOps"), Object.class);
        Object result = parse.invoke(codec, ops, element);
        Optional<?> value = (Optional<?>) result.getClass().getMethod("result").invoke(result);
        require(value.isPresent(), "Original codec refused surface rule: " + result);
        return value.get();
    }
    private static Object json(String text) throws Exception {
        return Class.forName("com.google.gson.JsonParser").getMethod("parseString", String.class).invoke(null, text);
    }
    private static Object member(Object element, String name) throws Exception {
        Object object = element.getClass().getMethod("getAsJsonObject").invoke(element);
        Object result = object.getClass().getMethod("get", String.class).invoke(object, name);
        require(result != null, "Missing JSON member " + name);
        return result;
    }
    private static String jarResource(String name) throws Exception {
        URL url = SurfaceRulesOracle.class.getClassLoader().getResource(name);
        require(url != null, "Missing original JAR resource " + name);
        JarURLConnection connection = (JarURLConnection) url.openConnection();
        require(Files.isSameFile(Path.of(connection.getJarFileURL().toURI()), inner), "Resource not from the pinned original inner JAR: " + url);
        try (InputStream in = connection.getInputStream()) { return new String(in.readAllBytes(), StandardCharsets.UTF_8); }
    }

    // ---------------------------------------------------------------- genuine context mapping
    /** Binds canonical node ids to the actual Condition objects that one apply(context) built. */
    private static final class Probe {
        final Object ctx, rule;
        final Object[] lazy;
        final String[] singleton;
        final Object temperature, steep, hole;
        final Object[] noise, random;
        long lastColumnUpdate = Long.MIN_VALUE, columnSeen = Long.MIN_VALUE;
        int calls, top;
        Probe(Object ctx, Object rule, int nodes) throws Exception {
            this.ctx = ctx; this.rule = rule;
            lazy = new Object[nodes]; singleton = new String[nodes]; noise = new Object[nodes]; random = new Object[nodes];
            temperature = field("Context", "temperature", ctx);
            steep = field("Context", "steep", ctx);
            hole = field("Context", "hole", ctx);
        }
    }
    private static int mapRule(Object rs, Object rule, int id, Probe p) throws Exception {
        if (type("SequenceSrc").isInstance(rs)) {
            List<?> children = (List<?>) field("SequenceSrc", "sequence", rs);
            int next = id + 1;
            if (children.size() == 1) return mapRule(children.get(0), rule, next, p);
            require(type("SequenceRule").isInstance(rule), "Original sequence rule shape");
            List<?> applied = (List<?>) field("SequenceRule", "rules", rule);
            require(applied.size() == children.size(), "Original sequence arity");
            for (int i = 0; i < children.size(); i++) next = mapRule(children.get(i), applied.get(i), next, p);
            return next;
        }
        if (type("TestSrc").isInstance(rs)) {
            require(type("TestRule").isInstance(rule), "Original test rule shape");
            int next = mapCondition(field("TestSrc", "ifTrue", rs), field("TestRule", "condition", rule), id + 1, p);
            return mapRule(field("TestSrc", "thenRun", rs), field("TestRule", "followup", rule), next, p);
        }
        if (type("BlockSrc").isInstance(rs)) {
            require(type("StateRule").isInstance(rule) && field("StateRule", "state", rule) == field("BlockSrc", "resultState", rs), "Original state rule identity");
            return id + 1;
        }
        require(type("BandlandsSrc").isInstance(rs) && !type("TestRule").isInstance(rule) && !type("SequenceRule").isInstance(rule) && !type("StateRule").isInstance(rule), "Original bandlands rule shape");
        return id + 1;
    }
    private static int mapCondition(Object cs, Object condition, int id, Probe p) throws Exception {
        if (type("NotCond").isInstance(cs)) {
            require(type("NotCondition").isInstance(condition), "Original not condition shape");
            return mapCondition(field("NotCond", "target", cs), field("NotCondition", "target", condition), id + 1, p);
        }
        if (type("SteepCond").isInstance(cs)) { require(condition == p.steep, "Shared steep"); p.singleton[id] = "steep"; return id + 1; }
        if (type("HoleCond").isInstance(cs)) { require(condition == p.hole, "Shared hole"); p.singleton[id] = "hole"; return id + 1; }
        if (type("TemperatureCond").isInstance(cs)) { require(condition == p.temperature, "Shared temperature"); p.singleton[id] = "temperature"; return id + 1; }
        if (type("AboveCond").isInstance(cs)) { require(condition == field("Context", "abovePreliminarySurface", p.ctx), "Shared above"); p.singleton[id] = "above"; return id + 1; }
        require(type("LazyCondition").isInstance(condition), "Original lazy condition expected for " + cs.getClass());
        p.lazy[id] = condition;
        // The exact sampler objects captured by the genuine condition; read-only diagnostics.
        for (Field f : condition.getClass().getDeclaredFields()) {
            if (type("NormalNoise").isAssignableFrom(f.getType())) { f.setAccessible(true); p.noise[id] = f.get(condition); }
            if (type("PositionalRandomFactory").isAssignableFrom(f.getType())) { f.setAccessible(true); p.random[id] = f.get(condition); }
        }
        return id + 1;
    }
    private static Probe probe(Object tree, TreeIndex index, Object ctx) throws Exception {
        Object rule = ((Function<Object, Object>) tree).apply(ctx);
        Probe p = new Probe(ctx, rule, index.lines.size());
        require(mapRule(tree, rule, 0, p) == index.lines.size(), "Incomplete original rule binding");
        return p;
    }
    private static Object memo(Object ctx) throws Exception {
        Object supplier = field("Context", "biome", ctx);
        if (supplier == null) return null;
        for (Class<?> c = supplier.getClass(); c != null; c = c.getSuperclass()) {
            try { Field f = c.getDeclaredField("value"); f.setAccessible(true); return f.get(supplier); }
            catch (NoSuchFieldException absent) { }
        }
        throw new IllegalStateException("Unknown original memoizing supplier " + supplier.getClass());
    }
    private static int height(Object chunk, int lx, int lz) throws Exception {
        return (int) method("Chunk", "getHeight", type("HeightmapTypes"), int.class, int.class).invoke(chunk, worldSurfaceWg, lx, lz);
    }

    /** Executes one original tryApply on a genuine context and returns the row text plus raw outcome. */
    private static final class Outcome { String text; Object state; Throwable failure; String result; }
    private static Outcome observe(Probe p, int x, int y, int z, boolean trace) throws Exception {
        Object ctx = p.ctx;
        long[] before = new long[p.lazy.length];
        if (trace) for (int i = 0; i < p.lazy.length; i++) if (p.lazy[i] != null) before[i] = longField("LazyCondition", "lastUpdate", p.lazy[i]);
        long steepBefore = longField("LazyCondition", "lastUpdate", p.steep), holeBefore = longField("LazyCondition", "lastUpdate", p.hole), tempBefore = longField("LazyCondition", "lastUpdate", p.temperature);
        long secondaryBefore = longField("Context", "lastSurfaceDepth2Update", ctx), minBefore = longField("Context", "lastMinSurfaceLevelUpdate", ctx);
        Object memoBefore = memo(ctx);
        Outcome out = new Outcome();
        try { out.state = method("SurfaceRule", "tryApply", int.class, int.class, int.class).invoke(p.rule, x, y, z); }
        catch (Throwable failure) { out.failure = unwrap(failure); }
        out.result = out.failure != null ? "exception" : out.state == null ? "none" : stateText(out.state);
        if (!trace) return out;
        StringJoiner computed = new StringJoiner(",");
        for (int i = 0; i < p.lazy.length; i++) {
            if (p.lazy[i] == null) continue;
            if (longField("LazyCondition", "lastUpdate", p.lazy[i]) == before[i]) continue;
            String entry = i + ":" + (((Boolean) field("LazyCondition", "result", p.lazy[i])) ? 1 : 0);
            int bx = intField("Context", "blockX", ctx), by = intField("Context", "blockY", ctx), bz = intField("Context", "blockZ", ctx);
            if (p.noise[i] != null) entry += ":" + Long.toHexString(Double.doubleToRawLongBits((double) method("NormalNoise", "getValue", double.class, double.class, double.class).invoke(p.noise[i], (double) bx, 0.0, (double) bz)));
            if (p.random[i] != null) entry += ":" + Integer.toHexString(Float.floatToRawIntBits((float) method("RandomSource", "nextFloat").invoke(method("PositionalRandomFactory", "at", int.class, int.class, int.class).invoke(p.random[i], bx, by, bz))));
            computed.add(entry);
        }
        StringJoiner singles = new StringJoiner(",");
        String steepHeights = "-";
        if (longField("LazyCondition", "lastUpdate", p.steep) != steepBefore) {
            singles.add("steep:" + ((Boolean) field("LazyCondition", "result", p.steep) ? 1 : 0));
            Object chunk = field("Context", "chunk", ctx);
            int lx = intField("Context", "blockX", ctx) & 15, lz = intField("Context", "blockZ", ctx) & 15;
            steepHeights = height(chunk, lx, Math.max(lz - 1, 0)) + "," + height(chunk, lx, Math.min(lz + 1, 15)) + "," + height(chunk, Math.max(lx - 1, 0), lz) + "," + height(chunk, Math.min(lx + 1, 15), lz);
        }
        if (longField("LazyCondition", "lastUpdate", p.hole) != holeBefore) singles.add("hole:" + ((Boolean) field("LazyCondition", "result", p.hole) ? 1 : 0));
        if (longField("LazyCondition", "lastUpdate", p.temperature) != tempBefore) singles.add("temperature:" + ((Boolean) field("LazyCondition", "result", p.temperature) ? 1 : 0));
        // Only a cached minimum surface makes the original above-condition test side-effect free.
        if (longField("Context", "lastMinSurfaceLevelUpdate", ctx) == longField("Context", "lastUpdateXZ", ctx))
            singles.add("above:" + ((boolean) method("Condition", "test").invoke(field("Context", "abovePreliminarySurface", ctx)) ? 1 : 0));
        String column = "-";
        if (longField("Context", "lastUpdateXZ", ctx) != p.columnSeen) {
            p.columnSeen = longField("Context", "lastUpdateXZ", ctx);
            Object system = field("Context", "system", ctx);
            int bx = intField("Context", "blockX", ctx), bz = intField("Context", "blockZ", ctx);
            double value = (double) method("NormalNoise", "getValue", double.class, double.class, double.class).invoke(field("SurfaceSystem", "surfaceNoise", system), (double) bx, 0.0, (double) bz);
            double draw = (double) method("RandomSource", "nextDouble").invoke(method("PositionalRandomFactory", "at", int.class, int.class, int.class).invoke(field("SurfaceSystem", "noiseRandom", system), bx, 0, bz));
            column = doubleText(value) + ";" + doubleText(draw);
        }
        String minSurface = "-";
        if (longField("Context", "lastMinSurfaceLevelUpdate", ctx) != minBefore) {
            int[] cache = (int[]) field("Context", "preliminarySurfaceCache", ctx);
            minSurface = intField("Context", "minSurfaceLevel", ctx) + ";" + cache[0] + "," + cache[1] + "," + cache[2] + "," + cache[3] + ";" + longField("Context", "lastPreliminarySurfaceCellOrigin", ctx);
        }
        String secondary = longField("Context", "lastSurfaceDepth2Update", ctx) != secondaryBefore ? doubleText((double) field("Context", "surfaceSecondary", ctx)) : "-";
        Object memoAfter = memo(ctx);
        String memoText = memoBefore == null && memoAfter != null ? key(memoAfter) : "-";
        out.text = cells(x, y, z, intField("Context", "blockX", ctx), intField("Context", "blockY", ctx), intField("Context", "blockZ", ctx),
            intField("Context", "surfaceDepth", ctx), intField("Context", "stoneDepthAbove", ctx), intField("Context", "stoneDepthBelow", ctx),
            intField("Context", "waterHeight", ctx), longField("Context", "lastUpdateXZ", ctx), longField("Context", "lastUpdateY", ctx),
            out.result, out.failure == null ? "none" : exception(out.failure), computed.length() == 0 ? "-" : computed.toString(),
            singles.length() == 0 ? "-" : singles.toString(), steepHeights, minSurface, secondary, memoText, column);
        return out;
    }

    // ---------------------------------------------------------------- groups
    private static final class Group {
        String id, preset, recording, origin, target;
        long seed;
        int cx, cz, holeX, holeZ;
        boolean sweep;
        Object generator, settings, randomState, manager, chunk, noiseChunk, system, wg, wgFlat;
        final Map<String, Object> chunks = new LinkedHashMap<>();
    }
    private static Object protoChunk(int cx, int cz) throws Exception {
        Object pos = type("ChunkPos").getConstructor(int.class, int.class).newInstance(cx, cz);
        Object accessor = method("Height", "create", int.class, int.class).invoke(null, -64, 384);
        Object result = type("ProtoChunk").getConstructor(type("ChunkPos"), type("Upgrade"), type("Height"), type("Registry"), type("BlendingData"))
            .newInstance(pos, field("Upgrade", "EMPTY", null), accessor, biomeRegistry, null);
        actualOrigin(result, "proto-chunk");
        return result;
    }
    private static Object structureManager(Group g, Object center) throws Exception {
        Object accessor = Proxy.newProxyInstance(SurfaceRulesOracle.class.getClassLoader(), new Class<?>[]{type("Accessor")}, (proxy, m, args) -> {
            Method get = method("Reader", "getChunk", int.class, int.class, type("Status"));
            require(m.getName().equals(get.getName()) && args != null && args.length == 3 && args[2] == referencesStatus, "Unexpected original structure callback " + m);
            Object c = g.chunks.get(args[0] + "," + args[1]);
            require(c != null, "Structure callback outside the declared 3x3 group");
            return c;
        });
        Object options = type("WorldOptions").getConstructor(long.class, boolean.class, boolean.class).newInstance(g.seed, false, false);
        return type("StructureManager").getConstructor(type("Accessor"), type("WorldOptions"), type("StructureCheck")).newInstance(accessor, options, null);
    }
    private static Object manager(Group g) throws Exception {
        Method chunkMethod = method("Reader", "getChunk", int.class, int.class, type("Status"), boolean.class);
        Object reader = Proxy.newProxyInstance(SurfaceRulesOracle.class.getClassLoader(), new Class<?>[]{type("Reader")}, (proxy, m, args) -> {
            if (m.isDefault()) return InvocationHandler.invokeDefault(proxy, m, args);
            if (m.getName().equals(chunkMethod.getName()) && Arrays.equals(m.getParameterTypes(), chunkMethod.getParameterTypes())) {
                require(args[2] == biomesStatus && Boolean.FALSE.equals(args[3]), "Actual LevelReader request must be BIOMES,false");
                Object c = g.chunks.get(args[0] + "," + args[1]);
                // WorldGenRegion refuses chunks outside the SURFACE step's BIOMES radius; never substitute.
                require(c != null, "Biome lookup outside the genuine 3x3 BIOMES neighbourhood");
                return c;
            }
            throw new IllegalStateException("Unexpected LevelReader callback " + m);
        });
        long zoom = (long) method("Manager", "obfuscateSeed", long.class).invoke(null, g.seed);
        return type("Manager").getConstructor(type("NoiseSource"), long.class).newInstance(reader, zoom);
    }
    private static Object carrier(boolean flat) throws Exception {
        Object environment = allocate(type("Environment"));
        fieldRef("Environment", "generateFlatBedrock").setBoolean(environment, flat);
        Object configuration = allocate(type("WorldConfiguration"));
        fieldRef("WorldConfiguration", "environment").set(configuration, environment);
        Object level = allocate(type("ServerLevel"));
        Field paper = type("Level").getDeclaredField("paperConfig");
        paper.setAccessible(true);
        paper.set(level, configuration);
        return level;
    }
    private static Object worldContext(Group g, Object flatCarrier) throws Exception {
        if (flatCarrier == null) return type("WGContext").getConstructor(type("ChunkGenerator"), type("Height")).newInstance(g.generator, g.chunk);
        return type("WGContext").getConstructor(type("ChunkGenerator"), type("Height"), type("Level")).newInstance(g.generator, g.chunk, flatCarrier);
    }
    private static void prepareGroup(Group g, BufferedWriter arrays) throws Exception {
        Object worldPreset = method("Getter", "getOrThrow", type("ResourceKey")).invoke(registryLookup("WORLD_PRESET"), resourceKey("WORLD_PRESET", "minecraft:" + g.preset));
        Map<?, ?> dims = (Map<?, ?>) call("Dimensions", "dimensions", call("Preset", "createWorldDimensions", call("Holder", "value", worldPreset)));
        Object stem = dims.get(resourceKey("LEVEL_STEM", "minecraft:overworld"));
        require(stem != null, "Missing actual overworld stem");
        g.generator = call("Stem", "generator", stem);
        actualOrigin(g.generator, "generator");
        g.settings = call("Holder", "value", call("Generator", "generatorSettings", g.generator));
        g.randomState = method("RandomState", "create", type("Settings"), type("Getter"), long.class).invoke(null, g.settings, registryLookup("NOISE"), g.seed);
        for (int x = g.cx - 1; x <= g.cx + 1; x++) for (int z = g.cz - 1; z <= g.cz + 1; z++) g.chunks.put(x + "," + z, protoChunk(x, z));
        // Each group starts from a cold RTree history; its nine chunks then share it in row-major order.
        coldHistory(g.generator);
        for (Map.Entry<String, Object> e : g.chunks.entrySet()) {
            Object future = method("Generator", "createBiomes", type("RandomState"), type("Blender"), type("StructureManager"), type("Chunk"))
                .invoke(g.generator, g.randomState, call("Blender", "empty", null), structureManager(g, e.getValue()), e.getValue());
            require(((CompletableFuture<?>) future).join() == e.getValue(), "createBiomes did not return the real chunk");
            method("ProtoChunk", "setPersistedStatus", type("Status")).invoke(e.getValue(), biomesStatus);
            String[] xz = e.getKey().split(",");
            StringJoiner values = new StringJoiner(",");
            int cx = Integer.parseInt(xz[0]), cz = Integer.parseInt(xz[1]);
            for (int qy = -16; qy < 80; qy++) for (int qx = 0; qx < 4; qx++) for (int qz = 0; qz < 4; qz++)
                values.add(Integer.toString(roster.indexOf(key(method("Chunk", "getNoiseBiome", int.class, int.class, int.class).invoke(e.getValue(), cx * 4 + qx, qy, cz * 4 + qz)))));
            line(arrays, g.id, cx, cz, values);
        }
        g.chunk = g.chunks.get(g.cx + "," + g.cz);
        Object filled = ((CompletableFuture<?>) method("Generator", "fillFromNoise", type("Blender"), type("RandomState"), type("StructureManager"), type("Chunk"))
            .invoke(g.generator, call("Blender", "empty", null), g.randomState, structureManager(g, g.chunk), g.chunk)).join();
        require(filled == g.chunk, "fillFromNoise did not return the real chunk");
        g.noiseChunk = field("Chunk", "noiseChunk", g.chunk);
        require(g.noiseChunk != null, "Original NOISE step retained no NoiseChunk");
        g.manager = manager(g);
        g.system = call("RandomState", "surfaceSystem", g.randomState);
        g.wg = worldContext(g, mode.equals("paper") ? carrier(false) : null);
        if (mode.equals("paper")) g.wgFlat = worldContext(g, carrier(true));
    }
    private static Object context(Group g, Object wg) throws Exception {
        Object manager = g.manager;
        Method getBiome = method("Manager", "getBiome", type("BlockPos"));
        // Identical delegation to SurfaceSystem.buildSurface's biomeManager::getBiome method reference.
        Function<Object, Object> getter = pos -> { try { return getBiome.invoke(manager, pos); } catch (Exception e) { throw new IllegalStateException(e); } };
        Constructor<?> c = type("Context").getDeclaredConstructor(type("SurfaceSystem"), type("RandomState"), type("Chunk"), type("NoiseChunk"), Function.class, type("Registry"), type("WGContext"));
        c.setAccessible(true);
        return c.newInstance(g.system, g.randomState, g.chunk, g.noiseChunk, getter, biomeRegistry, wg);
    }
    private static String directBiome(Group g, int x, int y, int z) throws Exception {
        Object pos = type("BlockPos").getConstructor(int.class, int.class, int.class).newInstance(x, y, z);
        try { return key(method("Manager", "getBiome", type("BlockPos")).invoke(g.manager, pos)); }
        catch (Throwable t) { return "exception:" + exception(t); }
    }

    // ---------------------------------------------------------------- tiers
    private static Object primary, bootstrap, registryTree;
    private static TreeIndex primaryIndex, bootstrapIndex, registryIndex;
    private static void genuineSurface(Group g) throws Exception {
        Probe[] holder = new Probe[1];
        Object recorder = Proxy.newProxyInstance(SurfaceRulesOracle.class.getClassLoader(), new Class<?>[]{type("RuleSource")}, (proxy, m, args) -> {
            if (m.getName().equals("apply") && args != null && args.length == 1) {
                require(holder[0] == null, "buildSurface applied the rule source more than once");
                Probe p = probe(primary, primaryIndex, args[0]);
                holder[0] = p;
                return Proxy.newProxyInstance(SurfaceRulesOracle.class.getClassLoader(), new Class<?>[]{type("SurfaceRule")}, (rp, rm, ra) -> {
                    if (rm.getName().equals(tryApplyName) && ra != null && ra.length == 3) {
                        long column = longField("Context", "lastUpdateXZ", p.ctx);
                        if (column != p.lastColumnUpdate) {
                            p.lastColumnUpdate = column; p.calls = 0;
                            p.top = height(g.chunk, ((int) ra[0]) & 15, ((int) ra[2]) & 15) + 1;
                        }
                        int x = (int) ra[0], y = (int) ra[1], z = (int) ra[2], n = p.calls++;
                        Outcome o = observe(p, x, y, z, true);
                        String text = cells("G", g.id, "c" + ((x & 15) * 16 + (z & 15)) + "/" + n + "/top" + p.top) + "\t" + o.text + "\t" + cells(directBiome(g, x, y, z), "-", "-");
                        allCalls.update((text + "\n").getBytes(StandardCharsets.UTF_8));
                        gCalls++;
                        boolean keep = g.recording.equals("window") ? y >= p.top - 24 : g.recording.equals("top") ? n < 8
                            : g.recording.equals("column") && x == g.holeX && z == g.holeZ;
                        if (keep) { rows.write(text); rows.newLine(); rowCount++; gRecorded++; }
                        if (o.failure != null) throw o.failure;
                        return o.state;
                    }
                    if (rm.isDefault()) return InvocationHandler.invokeDefault(rp, rm, ra);
                    throw new IllegalStateException("Unexpected original SurfaceRule call " + rm);
                });
            }
            if (m.isDefault()) return InvocationHandler.invokeDefault(proxy, m, args);
            throw new IllegalStateException("Unexpected original RuleSource call " + m);
        });
        method("SurfaceSystem", "buildSurface", type("RandomState"), type("Manager"), type("Registry"), boolean.class, type("WGContext"), type("Chunk"), type("NoiseChunk"), type("RuleSource"))
            .invoke(g.system, g.randomState, g.manager, biomeRegistry, call("Settings", "useLegacyRandomSource", g.settings), g.wg, g.chunk, g.noiseChunk, recorder);
        require(holder[0] != null, "buildSurface never applied the rule source");
    }

    private static final class Controlled {
        final Probe jar, boot, reg, flat;
        long column = Long.MIN_VALUE;
        Controlled(Group g, Object chunkOverride) throws Exception {
            Object saved = g.chunk;
            if (chunkOverride != null) g.chunk = chunkOverride;
            jar = probe(primary, primaryIndex, context(g, g.wg));
            boot = probe(bootstrap, bootstrapIndex, context(g, g.wg));
            reg = probe(registryTree, registryIndex, context(g, g.wg));
            flat = mode.equals("paper") ? probe(primary, primaryIndex, context(g, g.wgFlat)) : null;
            g.chunk = saved;
        }
    }
    private static int water(String token, int y) {
        if (token.equals("none")) return Integer.MIN_VALUE;
        if (token.equals("y")) return y;
        if (token.startsWith("y+")) return y + Integer.parseInt(token.substring(2));
        if (token.startsWith("y-")) return y - Integer.parseInt(token.substring(2));
        return Integer.parseInt(token);
    }
    private static void updateBoth(Probe p, long columnKey, Controlled c, int sa, int sb, int w, int x, int y, int z) throws Exception {
        Method xz = method("Context", "updateXZ", int.class, int.class);
        Method yy = method("Context", "updateY", int.class, int.class, int.class, int.class, int.class, int.class);
        if (c.column != columnKey) xz.invoke(p.ctx, x, z);
        yy.invoke(p.ctx, sa, sb, w, x, y, z);
    }
    private static void controlledRow(Group g, Controlled c, String label, int x, int y, int z, int sa, int sb, int w) throws Exception {
        long columnKey = ((long) x << 32) ^ (z & 0xffffffffL);
        updateBoth(c.jar, columnKey, c, sa, sb, w, x, y, z);
        Outcome o = observe(c.jar, x, y, z, true);
        String cross = "=";
        for (Probe other : new Probe[]{c.boot, c.reg}) {
            updateBoth(other, columnKey, c, sa, sb, w, x, y, z);
            Outcome q = observe(other, x, y, z, false);
            crossCompared++;
            if (!q.result.equals(o.result)) { crossMismatch++; cross = (other == c.boot ? "bootstrap=" : "registry=") + q.result; }
        }
        String flat = "-";
        if (c.flat != null && y <= 10) {
            updateBoth(c.flat, columnKey, c, sa, sb, w, x, y, z);
            Outcome q = observe(c.flat, x, y, z, false);
            flatCompared++;
            flat = q.result;
            if (!q.result.equals(o.result)) flatDiffer++;
        }
        c.column = columnKey;
        rows.write(cells("C", g.id, label) + "\t" + o.text + "\t" + cells(directBiome(g, x, y, z), cross, flat));
        rows.newLine();
        rowCount++; cRows++;
    }
    @SuppressWarnings("unchecked")
    private static void controlled(Group g, Map<String, Object> cases) throws Exception {
        Controlled c = new Controlled(g, null);
        int bx = g.cx * 16, bz = g.cz * 16;
        List<List<Object>> columns = (List<List<Object>>) cases.get("c_columns");
        for (List<Object> col : columns) {
            int lx = ((Number) col.get(0)).intValue(), lz = ((Number) col.get(1)).intValue(), x = bx + lx, z = bz + lz;
            int top = height(g.chunk, lx, lz) + 1;
            for (Object off : (List<Object>) cases.get("c_surface_offsets"))
                for (List<Object> v : (List<List<Object>>) cases.get("c_surface_variants")) {
                    int y = top - 1 + ((Number) off).intValue();
                    controlledRow(g, c, "surface/" + lx + "," + lz + "/" + off + "/" + v, x, y, z, ((Number) v.get(0)).intValue(), ((Number) v.get(1)).intValue(), water((String) v.get(2), y));
                }
            for (Object y : (List<Object>) cases.get("c_gradient_y"))
                controlledRow(g, c, "gradient/" + lx + "," + lz + "/" + y, x, ((Number) y).intValue(), z, 1, 1, Integer.MIN_VALUE);
            for (Object y : (List<Object>) cases.get("c_anchor_y"))
                for (List<Object> v : (List<List<Object>>) cases.get("c_anchor_variants")) {
                    int yy = ((Number) y).intValue();
                    controlledRow(g, c, "anchor/" + lx + "," + lz + "/" + y + "/" + v, x, yy, z, ((Number) v.get(0)).intValue(), ((Number) v.get(1)).intValue(), water((String) v.get(2), yy));
                }
            for (Object y : (List<Object>) cases.get("c_end_y"))
                controlledRow(g, c, "end/" + lx + "," + lz + "/" + y, x, ((Number) y).intValue(), z, 1, 1, Integer.MIN_VALUE);
        }
        for (List<Object> col : (List<List<Object>>) cases.get("c_stone_columns")) {
            int lx = ((Number) col.get(0)).intValue(), lz = ((Number) col.get(1)).intValue(), x = bx + lx, z = bz + lz, y = height(g.chunk, lx, lz);
            for (Object d : (List<Object>) cases.get("c_stone_depths")) {
                int depth = ((Number) d).intValue();
                controlledRow(g, c, "stone_above/" + lx + "," + lz + "/" + depth, x, y, z, depth, 1, Integer.MIN_VALUE);
                controlledRow(g, c, "stone_below/" + lx + "," + lz + "/" + depth, x, y, z, 1, depth, Integer.MIN_VALUE);
            }
        }
        for (List<Object> col : (List<List<Object>>) cases.get("c_water_columns")) {
            int lx = ((Number) col.get(0)).intValue(), lz = ((Number) col.get(1)).intValue(), x = bx + lx, z = bz + lz, y = height(g.chunk, lx, lz);
            for (Object s : (List<Object>) cases.get("c_water_stone")) {
                int stone = ((Number) s).intValue();
                controlledRow(g, c, "water/" + lx + "," + lz + "/" + stone + "/none", x, y, z, stone, 1, Integer.MIN_VALUE);
                for (Object off : (List<Object>) cases.get("c_water_offsets"))
                    controlledRow(g, c, "water/" + lx + "," + lz + "/" + stone + "/" + off, x, y, z, stone, 1, y + ((Number) off).intValue());
            }
        }
    }
    @SuppressWarnings("unchecked")
    private static void sweep(Group g, Map<String, Object> cases) throws Exception {
        Controlled c = new Controlled(g, null);
        int bx = g.cx * 16, bz = g.cz * 16;
        String[][] variants = {{"1", "1", "none"}, {"1", "2", "y+1"}, {"3", "1", "y+20"}};
        String[][] deeper = {{"2", "1", "none"}, {"4", "2", "none"}, {"1", "1", "y+5"}, {"1", "2", "y+20"}, {"2", "1", "y+3"}, {"1", "1", "y+20"}, {"2", "1", "y+20"}, {"2", "2", "none"}};
        for (int lx = 0; lx < 16; lx++) for (int lz = 0; lz < 16; lz++) {
            int x = bx + lx, z = bz + lz, top = height(g.chunk, lx, lz) + 1;
            Map<Integer, String> ys = new LinkedHashMap<>();
            ys.putIfAbsent(top - 1, "top-1"); ys.putIfAbsent(top + 2, "top+2"); ys.putIfAbsent(100, "100"); ys.putIfAbsent(110, "110"); ys.putIfAbsent(50, "50");
            for (Map.Entry<Integer, String> e : ys.entrySet()) {
                int y = e.getKey();
                if (!directBiome(g, x, y, z).equals(g.target)) continue;
                if (!e.getValue().equals("50")) for (String[] v : variants)
                    controlledRow(g, c, "sweep/" + lx + "," + lz + "/" + e.getValue() + "/" + String.join(",", v),
                        x, y, z, Integer.parseInt(v[0]), Integer.parseInt(v[1]), water(v[2], y));
                if (!e.getValue().equals("top-1") && !e.getValue().equals("100") && !e.getValue().equals("50")) continue;
                for (String[] v : deeper)
                    controlledRow(g, c, "sweep/" + lx + "," + lz + "/" + e.getValue() + "/" + String.join(",", v),
                        x, y, z, Integer.parseInt(v[0]), Integer.parseInt(v[1]), water(v[2], y));
            }
        }
        if (g.recording.equals("column")) {
            int lx = g.holeX & 15, lz = g.holeZ & 15, top = height(g.chunk, lx, lz) + 1;
            for (int off = -12; off <= 12; off++) for (String[] v : concat(variants, deeper))
                controlledRow(g, c, "holecolumn/" + off + "/" + String.join(",", v), g.holeX, top - 1 + off, g.holeZ, Integer.parseInt(v[0]), Integer.parseInt(v[1]), water(v[2], top - 1 + off));
        }
    }
    private static String[][] concat(String[][] a, String[][] b) {
        String[][] out = Arrays.copyOf(a, a.length + b.length);
        System.arraycopy(b, 0, out, a.length, b.length);
        return out;
    }
    private static void steepProbe(Group g) throws Exception {
        int base = height(g.chunk, 7, 7);
        Object chunk = protoChunk(g.cx, g.cz);
        Object stone = call("Block", "defaultBlockState", field("Blocks", "STONE", null));
        Method set = method("ProtoChunk", "setBlockState", type("BlockPos"), type("BlockState"), int.class);
        Constructor<?> pos = type("BlockPos").getConstructor(int.class, int.class, int.class);
        int bx = g.cx * 16, bz = g.cz * 16;
        int[][] h = new int[16][16];
        for (int lx = 0; lx < 16; lx++) for (int lz = 0; lz < 16; lz++) {
            h[lx][lz] = base + 3 * ((lx * 7 + lz * 3) % 5) + ((lx + lz) % 2);
            for (int y = -64; y <= h[lx][lz]; y++) set.invoke(chunk, pos.newInstance(bx + lx, y, bz + lz), stone, 0);
            require(height(chunk, lx, lz) == h[lx][lz], "Original heightmap did not follow placed stone");
        }
        Controlled c = new Controlled(g, chunk);
        for (int lx = 0; lx < 16; lx++) for (int lz = 0; lz < 16; lz++) {
            controlledRow(g, c, "steep/" + lx + "," + lz + "/top", bx + lx, h[lx][lz], bz + lz, 1, 1, Integer.MIN_VALUE);
            controlledRow(g, c, "steep/" + lx + "," + lz + "/below", bx + lx, h[lx][lz] - 1, bz + lz, 2, 1, Integer.MIN_VALUE);
        }
    }
    private static void diagnostic(String label, String outcome) throws Exception {
        rows.write(cells("D", label, outcome));
        rows.newLine();
        rowCount++; dRows++;
    }
    private static void diagnostics(Group g) throws Exception {
        if (mode.equals("paper")) {
            try { ((Function<Object, Object>) primary).apply(context(g, worldContext(g, null))); diagnostic("paper-jar-tree-without-level", "applied"); }
            catch (Throwable t) { diagnostic("paper-jar-tree-without-level", exception(t)); }
        }
        Probe fresh = probe(bootstrap, bootstrapIndex, context(g, g.wg));
        Outcome o = observe(fresh, g.cx * 16, 64, g.cz * 16, false);
        diagnostic("tryApply-before-update", o.result + "|" + (o.failure == null ? "none" : exception(o.failure)));
        Controlled c = new Controlled(g, null);
        int x = g.cx * 16 + 7, z = g.cz * 16 + 7, y = height(g.chunk, 7, 7);
        int[][] extremes = {{Integer.MAX_VALUE, 1, Integer.MIN_VALUE}, {Integer.MIN_VALUE, 1, Integer.MIN_VALUE}, {1, Integer.MAX_VALUE, Integer.MIN_VALUE},
            {1, Integer.MIN_VALUE, Integer.MIN_VALUE}, {1, 1, Integer.MAX_VALUE}, {1, 1, Integer.MIN_VALUE + 1}, {0, 0, Integer.MIN_VALUE}, {-1, -1, Integer.MIN_VALUE}};
        for (int[] e : extremes) controlledRow(g, c, "extreme/" + e[0] + "," + e[1] + "," + e[2], x, y, z, e[0], e[1], e[2]);
    }

    // ---------------------------------------------------------------- main
    private static void setup(String[] args) throws Exception {
        Path output = Path.of(args[1]).toRealPath();
        inner = Path.of(args[2]).toRealPath();
        helperRoot = Path.of(args[3]).toRealPath();
        symbols = new Properties();
        try (var in = Files.newBufferedReader(Path.of(args[4]))) { symbols.load(in); }
        origin(SurfaceRulesOracle.class, helperRoot, "helper/main");
        for (String alias : Files.readAllLines(Path.of(args[7]))) type(alias);
        if (mode.equals("paper")) for (String alias : Files.readAllLines(Path.of(args[8]))) { paperOnly.add(alias); type(alias); }
        Properties libraries = new Properties();
        try (var in = Files.newBufferedReader(Path.of(args[6]))) { libraries.load(in); }
        for (String name : new TreeSet<>(libraries.stringPropertyNames())) origin(Class.forName(name), Path.of(libraries.getProperty(name)), "library/" + name);
        call("Shared", "tryDetectVersion", null);
        call("Bootstrap", "bootStrap", null);
        require(!(boolean) field("Shared", "debugGenerateSquareTerrainWithoutNoise", null), "Debug terrain preset enabled");
        lookup = call("VanillaRegistries", "createLookup", null);
        actualOrigin(lookup, "registry-provider");
        Class<?> lifecycle = Class.forName("com.mojang.serialization.Lifecycle");
        biomeRegistry = type("MappedRegistry").getConstructor(type("ResourceKey"), lifecycle).newInstance(field("Registries", "BIOME", null), lifecycle.getMethod("stable").invoke(null));
        List<?> all = ((Stream<?>) call("Lookup", "listElements", registryLookup("BIOME"))).sorted(Comparator.comparing(h -> { try { return key(h); } catch (Exception e) { throw new IllegalStateException(e); } })).toList();
        require(all.size() == 65, "Complete original builtin biome registry");
        for (Object h : all) {
            Object registered = method("MappedRegistry", "register", type("ResourceKey"), Object.class, type("RegistrationInfo")).invoke(biomeRegistry, call("Reference", "key", h), call("Holder", "value", h), field("RegistrationInfo", "BUILT_IN", null));
            require(holders.putIfAbsent(key(registered), registered) == null, "Duplicate original biome");
        }
        call("MappedRegistry", "freeze", biomeRegistry);
        roster.addAll(holders.keySet());
        Path design = Path.of(args[5]).toRealPath();
        require(roster.equals(Files.readAllLines(design.resolve("roster.tsv"))), "Actual biome roster differs from declared inputs");
        biomesStatus = field("Status", "BIOMES", null);
        referencesStatus = field("Status", "STRUCTURE_REFERENCES", null);
        worldSurfaceWg = field("HeightmapTypes", "WORLD_SURFACE_WG", null);
        tryApplyName = method("SurfaceRule", "tryApply", int.class, int.class, int.class).getName();
        testName = method("Condition", "test").getName();
        Class<?> u = Class.forName("sun.misc.Unsafe");
        Field theUnsafe = u.getDeclaredField("theUnsafe");
        theUnsafe.setAccessible(true);
        unsafe = theUnsafe.get(null);
        allocate = u.getMethod("allocateInstance", Class.class);
        if (mode.equals("vanilla")) {
            Object executor = call("Util", "backgroundExecutor", null);
            originalBackground = (java.util.concurrent.ExecutorService) call("TracingExecutor", "service", executor);
            require(originalBackground instanceof java.util.concurrent.ForkJoinPool && ((java.util.concurrent.ForkJoinPool) originalBackground).getParallelism() == 1,
                "Actual Vanilla background pool must have original max.bg.threads=1");
        }
    }
    private static Object overworldSettings(String name) throws Exception {
        Object holder = method("Getter", "getOrThrow", type("ResourceKey")).invoke(registryLookup("NOISE_SETTINGS"), resourceKey("NOISE_SETTINGS", "minecraft:" + name));
        return call("Holder", "value", holder);
    }
    private static void writeTree(Path output, String name, TreeIndex t) throws Exception {
        try (BufferedWriter out = Files.newBufferedWriter(output.resolve("tree-" + name + ".tsv"))) {
            line(out, "id", "parent", "depth", "kind", "attributes", "shared_with");
            for (String l : t.lines) { out.write(l); out.newLine(); }
        }
    }
    @SuppressWarnings("unchecked")
    private static void scan(String[] args) throws Exception {
        Path output = Path.of(args[1]).toRealPath();
        Path design = Path.of(args[5]).toRealPath();
        Group g = new Group();
        g.preset = "normal"; g.seed = 0;
        Object worldPreset = method("Getter", "getOrThrow", type("ResourceKey")).invoke(registryLookup("WORLD_PRESET"), resourceKey("WORLD_PRESET", "minecraft:normal"));
        Map<?, ?> dims = (Map<?, ?>) call("Dimensions", "dimensions", call("Preset", "createWorldDimensions", call("Holder", "value", worldPreset)));
        Object generator = call("Stem", "generator", dims.get(resourceKey("LEVEL_STEM", "minecraft:overworld")));
        Object settings = call("Holder", "value", call("Generator", "generatorSettings", generator));
        Object random = method("RandomState", "create", type("Settings"), type("Getter"), long.class).invoke(null, settings, registryLookup("NOISE"), 0L);
        Object source = call("ChunkGenerator", "getBiomeSource", generator);
        Object sampler = call("RandomState", "sampler", random);
        Method noise = method("BiomeSource", "getNoiseBiome", int.class, int.class, int.class, type("ClimateSampler"));
        List<String> targets = new ArrayList<>();
        for (String[] r : lines(design.resolve("targets.tsv"))) targets.add(r[0]);
        int step = 256, radius = 49152;
        int[] levels = {64, 120, 176, 0, -40};
        Map<String, Object> cases = parseJson(Files.readString(design.resolve("cases.json")));
        List<Object> sweep = (List<Object>) cases.get("sweep_targets"), holes = (List<Object>) cases.get("hole_targets");
        int extra = ((Number) ((Map<String, Object>) cases.get("cases")).get("sweep_extra_hits")).intValue();
        Map<String, List<int[]>> found = new LinkedHashMap<>();
        Set<String> used = new HashSet<>();
        for (int ring = 0; ring * step <= radius; ring++) {
            for (int ix = -ring; ix <= ring; ix++) for (int iz = -ring; iz <= ring; iz++) {
                if (Math.max(Math.abs(ix), Math.abs(iz)) != ring) continue;
                int x = ix * step, z = iz * step;
                for (int y : levels) {
                    String biome = key(noise.invoke(source, x >> 2, y >> 2, z >> 2, sampler));
                    if (!targets.contains(biome)) continue;
                    List<int[]> hits = found.computeIfAbsent(biome, k -> new ArrayList<>());
                    int limit = sweep.contains(biome.substring("minecraft:".length())) ? 1 + extra : 1;
                    if (hits.size() < limit && used.add(biome + "@" + (x >> 4) + "," + (z >> 4))) hits.add(new int[]{x, y, z, ring});
                }
            }
        }
        Object system = call("RandomState", "surfaceSystem", random);
        Method depth = method("SurfaceSystem", "getSurfaceDepth", int.class, int.class);
        try (BufferedWriter out = Files.newBufferedWriter(output.resolve("centers.tsv"))) {
            line(out, "label", "biome", "kind", "x", "y", "z", "cx", "cz", "ring");
            for (String t : targets) {
                List<int[]> hits = found.getOrDefault(t, List.of());
                if (hits.isEmpty()) line(out, t, t, "absent", "-", "-", "-", "-", "-", "-");
                for (int i = 0; i < hits.size(); i++) {
                    int[] h = hits.get(i);
                    line(out, i == 0 ? t : t + "#" + i, t, i == 0 ? "first" : "extra", h[0], h[1], h[2], h[0] >> 4, h[2] >> 4, h[3]);
                }
                if (!hits.isEmpty() && holes.contains(t.substring("minecraft:".length()))) {
                    int[] h = hits.get(0);
                    int[] hole = null;
                    for (int r = 0; r <= 384 && hole == null; r++) for (int ix = -r; ix <= r && hole == null; ix++) for (int iz = -r; iz <= r && hole == null; iz++) {
                        if (Math.max(Math.abs(ix), Math.abs(iz)) != r) continue;
                        int x = h[0] + ix * 4, z = h[2] + iz * 4;
                        if ((int) depth.invoke(system, x, z) > 0) continue;
                        for (int y : new int[]{40, 64}) if (hole == null && key(noise.invoke(source, x >> 2, y >> 2, z >> 2, sampler)).equals(t)) hole = new int[]{x, y, z, r};
                    }
                    if (hole == null) line(out, t + "#hole", t, "hole-absent", "-", "-", "-", "-", "-", "-");
                    else line(out, t + "#hole", t, "hole", hole[0], hole[1], hole[2], hole[0] >> 4, hole[2] >> 4, hole[3]);
                }
                List<Object> bandList = (List<Object>) ((Map<String, Object>) cases.get("cases")).get("band_searches");
                for (int b = 0; b < bandList.size(); b++) {
                    List<Object> band = (List<Object>) bandList.get(b);
                    if (hits.isEmpty() || !t.equals("minecraft:" + band.get(0))) continue;
                    Object noiseObject = method("RandomState", "getOrCreateNoise", type("ResourceKey")).invoke(random, resourceKey("NOISE", (String) band.get(1)));
                    Method value = method("NormalNoise", "getValue", double.class, double.class, double.class);
                    double lo = ((Number) band.get(2)).doubleValue(), hi = ((Number) band.get(3)).doubleValue();
                    int[] h = hits.get(0), hit = null;
                    for (int r = 0; r <= 384 && hit == null; r++) for (int ix = -r; ix <= r && hit == null; ix++) for (int iz = -r; iz <= r && hit == null; iz++) {
                        if (Math.max(Math.abs(ix), Math.abs(iz)) != r) continue;
                        int x = h[0] + ix * 4, z = h[2] + iz * 4;
                        double v = (double) value.invoke(noiseObject, (double) x, 0.0, (double) z);
                        if (v < lo || v > hi) continue;
                        for (Object y : (List<Object>) band.get(4)) {
                            int yy = ((Number) y).intValue();
                            if (hit == null && key(noise.invoke(source, x >> 2, yy >> 2, z >> 2, sampler)).equals(t)) hit = new int[]{x, yy, z, r};
                        }
                    }
                    if (hit == null) line(out, t + "#band" + b, t, "band-absent", "-", "-", "-", "-", "-", "-");
                    else line(out, t + "#band" + b, t, "band", hit[0], hit[1], hit[2], hit[0] >> 4, hit[2] >> 4, hit[3]);
                }
            }
        }
        System.out.println("Scanned genuine MultiNoiseBiomeSource lattice and genuine surface depths: " + found.size() + "/" + targets.size());
    }
    @SuppressWarnings("unchecked")
    public static void main(String[] args) throws Exception {
        require(args.length == 10, "MODE OUT INNER HELPER SYMBOLS DESIGN LIBRARIES ALIASES PAPER_ALIASES CENTERS");
        String requested = args[0];
        require(requested.equals("vanilla") || requested.equals("paper") || requested.equals("scan"), "Explicit original mode");
        mode = requested.equals("scan") ? "paper" : requested;
        setup(args);
        if (requested.equals("scan")) { scan(args); return; }
        Path output = Path.of(args[1]).toRealPath();
        Path design = Path.of(args[5]).toRealPath();
        Map<String, Object> cases = (Map<String, Object>) parseJson(Files.readString(design.resolve("cases.json"))).get("cases");
        // Three genuine trees: bootstrap objects, this artifact's JAR data via the original codec,
        // and the FerrumC registry's vendored export slice via the same original codec.
        bootstrap = call("Settings", "surfaceRule", overworldSettings("overworld"));
        primary = decodeRule(member(json(jarResource("data/minecraft/worldgen/noise_settings/overworld.json")), "surface_rule"));
        registryTree = decodeRule(json(Files.readString(design.resolve("registry-overworld-surface-rule.json"))));
        primaryIndex = index(primary); bootstrapIndex = index(bootstrap); registryIndex = index(registryTree);
        writeTree(output, "jar", primaryIndex); writeTree(output, "bootstrap", bootstrapIndex); writeTree(output, "registry", registryIndex);
        try (BufferedWriter out = Files.newBufferedWriter(output.resolve("tree-variants.tsv"))) {
            line(out, "settings", "source", "nodes", "canonical_equal_to_jar_overworld");
            for (String name : new String[]{"overworld", "large_biomes", "amplified"}) {
                TreeIndex b = index(call("Settings", "surfaceRule", overworldSettings(name)));
                TreeIndex j = index(decodeRule(member(json(jarResource("data/minecraft/worldgen/noise_settings/" + name + ".json")), "surface_rule")));
                TreeIndex r = index(decodeRule(json(Files.readString(design.resolve("registry-" + name + "-surface-rule.json")))));
                line(out, name, "bootstrap", b.lines.size(), canonicalEqual(b, primaryIndex));
                line(out, name, "jar", j.lines.size(), canonicalEqual(j, primaryIndex));
                line(out, name, "registry", r.lines.size(), canonicalEqual(r, primaryIndex));
            }
        }
        allCalls = MessageDigest.getInstance("SHA-256");
        List<Group> groups = new ArrayList<>();
        for (String[] r : lines(design.resolve("fixed-groups.tsv"))) {
            Group g = new Group(); g.id = r[0]; g.preset = r[1]; g.seed = Long.parseLong(r[2]); g.cx = Integer.parseInt(r[3]); g.cz = Integer.parseInt(r[4]); g.recording = r[5]; g.origin = "fixed";
            groups.add(g);
        }
        List<Object> windowTargets = (List<Object>) parseJson(Files.readString(design.resolve("cases.json"))).get("window_targets");
        List<Object> sweepTargets = (List<Object>) parseJson(Files.readString(design.resolve("cases.json"))).get("sweep_targets");
        List<Object> holeTargets = (List<Object>) parseJson(Files.readString(design.resolve("cases.json"))).get("hole_targets");
        List<Object> firstOnly = (List<Object>) parseJson(Files.readString(design.resolve("cases.json"))).get("sweep_first_only");
        for (String[] r : lines(Path.of(args[9]))) {
            if (r[2].endsWith("absent")) continue;
            String bare = r[1].substring("minecraft:".length());
            Group g = new Group(); g.preset = "normal"; g.seed = 0; g.target = r[1];
            g.cx = Integer.parseInt(r[6]); g.cz = Integer.parseInt(r[7]); g.origin = "scan:" + r[2] + ":" + r[3] + "," + r[4] + "," + r[5];
            g.sweep = sweepTargets.contains(bare) || holeTargets.contains(bare) || firstOnly.contains(bare);
            if (r[2].equals("first")) { g.id = "biome/" + bare; g.recording = windowTargets.contains(bare) ? "window" : "top"; }
            else if (r[2].equals("extra")) { g.id = "sweep/" + bare + "/" + r[0].substring(r[0].indexOf('#') + 1); g.recording = "none"; }
            else if (r[2].equals("hole")) { g.id = "hole/" + bare; g.recording = "column"; g.holeX = Integer.parseInt(r[3]); g.holeZ = Integer.parseInt(r[5]); }
            else { g.id = "band/" + bare + "/" + r[0].substring(r[0].indexOf('#') + 1); g.recording = "column"; g.holeX = Integer.parseInt(r[3]); g.holeZ = Integer.parseInt(r[5]); g.sweep = true; }
            groups.add(g);
        }
        require(groups.size() <= 96, "Declared group cap");
        try (BufferedWriter r = Files.newBufferedWriter(output.resolve("rows.tsv"));
             BufferedWriter arrays = Files.newBufferedWriter(output.resolve("arrays.tsv"));
             BufferedWriter columns = Files.newBufferedWriter(output.resolve("groups.tsv"))) {
            rows = r;
            line(r, "tier", "group", "case", "x", "y", "z", "ctx_x", "ctx_y", "ctx_z", "surface_depth", "stone_above", "stone_below", "water_height",
                "update_xz", "update_y", "result", "exception", "computed", "singletons", "steep_heights", "min_surface", "surface_secondary", "biome_memo", "column_surface_noise_and_draw", "biome_direct", "cross_trees", "paper_flat_bedrock");
            line(arrays, "group", "cx", "cz", "roster_indices_qy_qx_qz");
            line(columns, "group", "preset", "seed", "cx", "cz", "recording", "origin", "preliminary_corners", "final_world_surface_wg", "sea_level", "min_gen_y", "gen_depth", "clay_bands", "resolved_anchors");
            for (Group g : groups) {
                prepareGroup(g, arrays);
                genuineSurface(g);
                StringJoiner corners = new StringJoiner(",");
                for (int[] d : new int[][]{{0, 0}, {1, 0}, {0, 1}, {1, 1}})
                    corners.add(Integer.toString((int) method("NoiseChunk", "preliminarySurfaceLevel", int.class, int.class).invoke(g.noiseChunk, (g.cx + d[0]) * 16, (g.cz + d[1]) * 16)));
                StringJoiner heights = new StringJoiner(",");
                for (int lx = 0; lx < 16; lx++) for (int lz = 0; lz < 16; lz++) heights.add(Integer.toString(height(g.chunk, lx, lz)));
                StringJoiner bands = new StringJoiner(",");
                for (Object state : (Object[]) field("SurfaceSystem", "clayBands", g.system)) bands.add(stateText(state).split("\\|")[0]);
                StringJoiner resolved = new StringJoiner(",");
                for (Map.Entry<Integer, List<Object>> e : primaryIndex.anchors.entrySet()) {
                    StringJoiner ys = new StringJoiner("/");
                    for (Object a : e.getValue()) ys.add(Integer.toString((int) method("Anchor", "resolveY", type("WGContext")).invoke(a, g.wg)));
                    resolved.add(e.getKey() + "=" + ys);
                }
                line(columns, g.id, g.preset, g.seed, g.cx, g.cz, g.recording, g.origin, corners, heights,
                    call("Settings", "seaLevel", g.settings), call("WGContext", "getMinGenY", g.wg), call("WGContext", "getGenDepth", g.wg), bands, resolved);
                if (!g.recording.equals("none") && !g.recording.equals("column")) controlled(g, cases);
                if (g.sweep) sweep(g, cases);
                if (g.id.equals("biome/frozen_peaks") || g.id.equals("biome/snowy_slopes") || g.id.equals("biome/jagged_peaks")) steepProbe(g);
                if (g.id.equals("origin/normal/0")) diagnostics(g);
            }
        }
        require(rowCount <= 1_000_000, "Declared per-mode row cap");
        try (BufferedWriter out = Files.newBufferedWriter(output.resolve("origins.tsv"))) { for (var e : origins.entrySet()) { out.write(e.getKey() + "\t" + e.getValue()); out.newLine(); } }
        Files.writeString(output.resolve("counts.tsv"), cells("rows", rowCount) + "\n" + cells("genuine_surface_calls", gCalls) + "\n" + cells("genuine_surface_recorded", gRecorded) + "\n"
            + cells("genuine_surface_all_calls_sha256", HexFormat.of().formatHex(allCalls.digest())) + "\n" + cells("controlled_rows", cRows) + "\n"
            + cells("cross_tree_compared", crossCompared) + "\n" + cells("cross_tree_mismatch", crossMismatch) + "\n"
            + cells("paper_flat_compared", flatCompared) + "\n" + cells("paper_flat_differ", flatDiffer) + "\n" + cells("diagnostic_rows", dRows) + "\n" + cells("groups", groups.size()) + "\n");
        System.out.println("Completed genuine original surface-rule probes; no WorldGenRegion, carver or server execution");
    }
    private static String canonicalEqual(TreeIndex a, TreeIndex b) {
        if (a.lines.size() != b.lines.size()) return "false";
        for (int i = 0; i < a.lines.size(); i++) {
            String x = a.lines.get(i), y = b.lines.get(i);
            if (!x.substring(0, x.lastIndexOf('\t')).equals(y.substring(0, y.lastIndexOf('\t')))) return "false@" + i;
        }
        return "true";
    }
    @SuppressWarnings("unchecked")
    private static Map<String, Object> parseJson(String text) throws Exception {
        Object element = json(text);
        return (Map<String, Object>) toJava(element);
    }
    private static Object toJava(Object element) throws Exception {
        Class<?> c = element.getClass();
        if ((boolean) c.getMethod("isJsonObject").invoke(element)) {
            Map<String, Object> out = new LinkedHashMap<>();
            Object object = c.getMethod("getAsJsonObject").invoke(element);
            for (Object e : (Set<?>) object.getClass().getMethod("entrySet").invoke(object)) {
                Map.Entry<?, ?> entry = (Map.Entry<?, ?>) e;
                out.put((String) entry.getKey(), toJava(entry.getValue()));
            }
            return out;
        }
        if ((boolean) c.getMethod("isJsonArray").invoke(element)) {
            List<Object> out = new ArrayList<>();
            for (Object e : (Iterable<?>) c.getMethod("getAsJsonArray").invoke(element)) out.add(toJava(e));
            return out;
        }
        Object primitive = c.getMethod("getAsJsonPrimitive").invoke(element);
        if ((boolean) primitive.getClass().getMethod("isNumber").invoke(primitive)) {
            String token = primitive.getClass().getMethod("getAsString").invoke(primitive).toString();
            return token.matches("-?[0-9]+") ? (Object) Long.parseLong(token) : (Object) Double.parseDouble(token);
        }
        return primitive.getClass().getMethod("getAsString").invoke(primitive);
    }
}
