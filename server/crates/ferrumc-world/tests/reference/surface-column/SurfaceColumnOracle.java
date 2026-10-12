package ferrumc.surfacecolumnoracle;

import java.io.BufferedWriter;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;
import java.util.concurrent.CompletableFuture;
import java.util.function.Function;
import java.util.stream.Stream;

/**
 * Original reflection/delegation only. Every column input, block write, heightmap, post-processing
 * mark and rule outcome is read from actual original objects after genuine original execution. The
 * helper implements no surface rule, column loop, extension, noise, random, temperature or zoom logic;
 * it only snapshots chunks and forwards genuine tryApply calls through a transparent RuleSource proxy.
 */
public final class SurfaceColumnOracle {
    private static Properties symbols;
    private static String mode;
    private static Path inner, helperRoot;
    private static final Map<String, Class<?>> types = new TreeMap<>();
    private static final Map<String, String> origins = new TreeMap<>();
    private static final Set<String> paperOnly = new TreeSet<>();
    private static final Map<String, Object> holders = new TreeMap<>();
    private static final List<String> roster = new ArrayList<>();
    private static Object lookup, biomeRegistry, biomesStatus, noiseStatus, referencesStatus, worldSurfaceWg, oceanFloorWg;
    private static Object airBlock, waterBlock, lavaBlock, iceBlock, snowBlock, stoneBlock;
    private static Object unsafe;
    private static Method allocate, shortGet, shortSize;
    private static String tryApplyName;
    private static java.util.concurrent.ExecutorService originalBackground;

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
            result = Class.forName(property("class." + alias), false, SurfaceColumnOracle.class.getClassLoader());
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
        while (t instanceof InvocationTargetException || t instanceof java.util.concurrent.CompletionException
            || t instanceof java.util.concurrent.ExecutionException || t instanceof UndeclaredThrowableException) t = t.getCause();
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
    private static String doubleText(double value) {
        return Double.toString(value) + "/" + Long.toHexString(Double.doubleToRawLongBits(value));
    }
    private static Object blockPos(int x, int y, int z) throws Exception {
        return type("BlockPos").getConstructor(int.class, int.class, int.class).newInstance(x, y, z);
    }

    // ---------------------------------------------------------------- original states
    private static final IdentityHashMap<Object, Integer> stateIds = new IdentityHashMap<>();
    private static final List<Object> stateList = new ArrayList<>();
    private static final List<String> stateRows = new ArrayList<>();
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
    private static boolean is(Object state, Object block) throws Exception {
        return (boolean) method("StateBase", "is", type("Block")).invoke(state, block);
    }
    private static int stateId(Object state) throws Exception {
        if (state == null) return -1;
        Integer known = stateIds.get(state);
        if (known != null) return known;
        int id = stateList.size();
        stateIds.put(state, id);
        stateList.add(state);
        boolean air = (boolean) call("StateBase", "isAir", state);
        boolean fluid = !(boolean) call("FluidState", "isEmpty", call("StateBase", "getFluidState", state));
        stateRows.add(cells(id, stateText(state), air ? 1 : 0, fluid ? 1 : 0, is(state, stoneBlock) ? 1 : 0,
            call("Block", "defaultBlockState", stoneBlock) == state ? 1 : 0, is(state, waterBlock) ? 1 : 0, is(state, lavaBlock) ? 1 : 0,
            (boolean) call("StateBase", "blocksMotion", state) ? 1 : 0, (boolean) call("StateBase", "hasBlockEntity", state) ? 1 : 0));
        return id;
    }
    private static String rle(int[] column) {
        StringJoiner runs = new StringJoiner(".");
        int i = 0;
        while (i < column.length) {
            int j = i;
            while (j < column.length && column[j] == column[i]) j++;
            runs.add(column[i] + "*" + (j - i));
            i = j;
        }
        return runs.toString();
    }

    // ---------------------------------------------------------------- generation
    private static final class Group { String id, preset, origin; long seed; int cx, cz; }
    private static final class Gen {
        Object generator, settings, randomState, chunk, noiseChunk, manager, wg, system, structures;
        final Map<String, Object> chunks = new LinkedHashMap<>();
        final Map<String, String> arrays = new LinkedHashMap<>();
        int minX, minZ, seaLevel;
        long seed;
    }
    private static Object protoChunk(int cx, int cz) throws Exception {
        Object pos = type("ChunkPos").getConstructor(int.class, int.class).newInstance(cx, cz);
        Object accessor = method("Height", "create", int.class, int.class).invoke(null, -64, 384);
        Object result = type("ProtoChunk").getConstructor(type("ChunkPos"), type("Upgrade"), type("Height"), type("Registry"), type("BlendingData"))
            .newInstance(pos, field("Upgrade", "EMPTY", null), accessor, biomeRegistry, null);
        actualOrigin(result, "proto-chunk");
        return result;
    }
    private static Object structureManager(Gen g) throws Exception {
        Object accessor = Proxy.newProxyInstance(SurfaceColumnOracle.class.getClassLoader(), new Class<?>[]{type("Accessor")}, (proxy, m, args) -> {
            Method get = method("Reader", "getChunk", int.class, int.class, type("Status"));
            require(m.getName().equals(get.getName()) && args != null && args.length == 3 && args[2] == referencesStatus, "Unexpected original structure callback " + m);
            Object c = g.chunks.get(args[0] + "," + args[1]);
            require(c != null, "Structure callback outside the declared 3x3 group");
            return c;
        });
        Object options = type("WorldOptions").getConstructor(long.class, boolean.class, boolean.class).newInstance(g.seed, false, false);
        return type("StructureManager").getConstructor(type("Accessor"), type("WorldOptions"), type("StructureCheck")).newInstance(accessor, options, null);
    }
    private static Object manager(Gen g) throws Exception {
        Method chunkMethod = method("Reader", "getChunk", int.class, int.class, type("Status"), boolean.class);
        Object reader = Proxy.newProxyInstance(SurfaceColumnOracle.class.getClassLoader(), new Class<?>[]{type("Reader")}, (proxy, m, args) -> {
            if (m.isDefault()) return InvocationHandler.invokeDefault(proxy, m, args);
            if (m.getName().equals(chunkMethod.getName()) && Arrays.equals(m.getParameterTypes(), chunkMethod.getParameterTypes())) {
                require(args[2] == biomesStatus && Boolean.FALSE.equals(args[3]), "Actual LevelReader request must be BIOMES,false");
                Object c = g.chunks.get(args[0] + "," + args[1]);
                // WorldGenRegion serves only the SURFACE step's BIOMES radius 1; never substitute.
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
    private static String arrayText(Object chunk, int cx, int cz) throws Exception {
        StringJoiner values = new StringJoiner(",");
        for (int qy = -16; qy < 80; qy++) for (int qx = 0; qx < 4; qx++) for (int qz = 0; qz < 4; qz++)
            values.add(Integer.toString(roster.indexOf(key(method("Chunk", "getNoiseBiome", int.class, int.class, int.class).invoke(chunk, cx * 4 + qx, qy, cz * 4 + qz)))));
        return values.toString();
    }
    private static Gen generate(Group group) throws Exception {
        Gen g = new Gen();
        g.seed = group.seed;
        Object worldPreset = method("Getter", "getOrThrow", type("ResourceKey")).invoke(registryLookup("WORLD_PRESET"), resourceKey("WORLD_PRESET", "minecraft:" + group.preset));
        Map<?, ?> dims = (Map<?, ?>) call("Dimensions", "dimensions", call("Preset", "createWorldDimensions", call("Holder", "value", worldPreset)));
        Object stem = dims.get(resourceKey("LEVEL_STEM", "minecraft:overworld"));
        require(stem != null, "Missing actual overworld stem");
        g.generator = call("Stem", "generator", stem);
        actualOrigin(g.generator, "generator");
        require(type("Generator").isInstance(g.generator), "Overworld generator must be NoiseBasedChunkGenerator");
        g.settings = call("Holder", "value", call("Generator", "generatorSettings", g.generator));
        g.seaLevel = (int) call("Settings", "seaLevel", g.settings);
        require(call("Settings", "defaultBlock", g.settings) == call("Block", "defaultBlockState", stoneBlock), "Overworld default block must be stone");
        require(!(boolean) call("Settings", "useLegacyRandomSource", g.settings), "Overworld settings use the Xoroshiro random source");
        g.randomState = method("RandomState", "create", type("Settings"), type("Getter"), long.class).invoke(null, g.settings, registryLookup("NOISE"), g.seed);
        for (int x = group.cx - 1; x <= group.cx + 1; x++) for (int z = group.cz - 1; z <= group.cz + 1; z++) g.chunks.put(x + "," + z, protoChunk(x, z));
        g.structures = structureManager(g);
        // Each generation starts from a cold RTree history; its nine chunks then share it in row-major order.
        coldHistory(g.generator);
        for (Map.Entry<String, Object> e : g.chunks.entrySet()) {
            Object future = method("Generator", "createBiomes", type("RandomState"), type("Blender"), type("StructureManager"), type("Chunk"))
                .invoke(g.generator, g.randomState, call("Blender", "empty", null), g.structures, e.getValue());
            require(((CompletableFuture<?>) future).join() == e.getValue(), "createBiomes did not return the real chunk");
            method("ProtoChunk", "setPersistedStatus", type("Status")).invoke(e.getValue(), biomesStatus);
            String[] xz = e.getKey().split(",");
            g.arrays.put(e.getKey(), arrayText(e.getValue(), Integer.parseInt(xz[0]), Integer.parseInt(xz[1])));
        }
        g.chunk = g.chunks.get(group.cx + "," + group.cz);
        Object filled = ((CompletableFuture<?>) method("Generator", "fillFromNoise", type("Blender"), type("RandomState"), type("StructureManager"), type("Chunk"))
            .invoke(g.generator, call("Blender", "empty", null), g.randomState, g.structures, g.chunk)).join();
        require(filled == g.chunk, "fillFromNoise did not return the real chunk");
        method("ProtoChunk", "setPersistedStatus", type("Status")).invoke(g.chunk, noiseStatus);
        g.noiseChunk = field("Chunk", "noiseChunk", g.chunk);
        require(g.noiseChunk != null, "Original NOISE step retained no NoiseChunk");
        g.manager = manager(g);
        g.system = call("RandomState", "surfaceSystem", g.randomState);
        Object accessor = method("Height", "create", int.class, int.class).invoke(null, -64, 384);
        g.wg = mode.equals("paper")
            ? type("WGContext").getConstructor(type("ChunkGenerator"), type("Height"), type("Level")).newInstance(g.generator, accessor, carrier(false))
            : type("WGContext").getConstructor(type("ChunkGenerator"), type("Height")).newInstance(g.generator, accessor);
        g.minX = group.cx * 16; g.minZ = group.cz * 16;
        return g;
    }

    // ---------------------------------------------------------------- snapshots
    private static final class Snap {
        final int[][] states = new int[256][384];
        final int[] wswg = new int[256], ofwg = new int[256];
        String heightmaps, post;
        int blockEntities, pending;
    }
    private static int height(Object chunk, Object type, int lx, int lz) throws Exception {
        return (int) method("Chunk", "getHeight", type("HeightmapTypes"), int.class, int.class).invoke(chunk, type, lx, lz);
    }
    private static Object blockAt(Object chunk, int x, int y, int z) throws Exception {
        return method("ProtoChunk", "getBlockState", type("BlockPos")).invoke(chunk, blockPos(x, y, z));
    }
    private static int[] column(Gen g, Object chunk, int lx, int lz) throws Exception {
        int[] result = new int[384];
        for (int y = -64; y < 320; y++) result[y + 64] = stateId(blockAt(chunk, g.minX + lx, y, g.minZ + lz));
        return result;
    }
    private static Snap snap(Gen g, Object chunk) throws Exception {
        Snap s = new Snap();
        for (int lx = 0; lx < 16; lx++) for (int lz = 0; lz < 16; lz++) s.states[lx * 16 + lz] = column(g, chunk, lx, lz);
        Map<?, ?> maps = (Map<?, ?>) field("Chunk", "heightmaps", chunk);
        boolean ws = maps.containsKey(worldSurfaceWg), of = maps.containsKey(oceanFloorWg);
        s.heightmaps = (ws ? "WORLD_SURFACE_WG" : "") + "," + (of ? "OCEAN_FLOOR_WG" : "") + ",n=" + maps.size();
        // getHeight primes a missing heightmap (a side effect); read only present maps.
        for (int lx = 0; lx < 16; lx++) for (int lz = 0; lz < 16; lz++) {
            s.wswg[lx * 16 + lz] = ws ? height(chunk, worldSurfaceWg, lx, lz) : Integer.MIN_VALUE;
            s.ofwg[lx * 16 + lz] = of ? height(chunk, oceanFloorWg, lx, lz) : Integer.MIN_VALUE;
        }
        Object[] lists = (Object[]) call("Chunk", "getPostProcessing", chunk);
        StringJoiner post = new StringJoiner(";");
        for (int i = 0; i < lists.length; i++) {
            if (lists[i] == null) continue;
            StringJoiner entries = new StringJoiner(",");
            int n = (int) shortSize.invoke(lists[i]);
            for (int k = 0; k < n; k++) entries.add(Integer.toString(((short) shortGet.invoke(lists[i], k)) & 0xffff));
            post.add(i + ":" + entries);
        }
        s.post = post.length() == 0 ? "-" : post.toString();
        s.blockEntities = ((Map<?, ?>) call("ProtoChunk", "getBlockEntities", chunk)).size();
        s.pending = ((Map<?, ?>) call("ProtoChunk", "getBlockEntityNbts", chunk)).size();
        return s;
    }
    private static boolean same(Snap a, Snap b) {
        return Arrays.deepEquals(a.states, b.states) && Arrays.equals(a.wswg, b.wswg) && Arrays.equals(a.ofwg, b.ofwg)
            && a.heightmaps.equals(b.heightmaps) && a.post.equals(b.post) && a.blockEntities == b.blockEntities && a.pending == b.pending;
    }
    private static String ints(int[] values) {
        StringJoiner out = new StringJoiner(",");
        for (int v : values) out.add(Integer.toString(v));
        return out.toString();
    }
    private static String neighbours(Gen g) throws Exception {
        StringJoiner out = new StringJoiner(";");
        for (Map.Entry<String, Object> e : g.chunks.entrySet()) {
            if (e.getValue() == g.chunk) continue;
            out.add(e.getKey() + "=" + call("Chunk", "getHighestFilledSectionIndex", e.getValue()));
        }
        return out.toString();
    }

    // ---------------------------------------------------------------- recorded SURFACE run
    private static final class ColRec {
        int lx, lz, firstHeight, surfaceDepth;
        long xz;
        int[] after;
        final List<int[]> calls = new ArrayList<>();
        String steep = "-";
        int steepCall = -1;
        int[] steepHeights;
        String minSurface = "-";
    }
    private static final class Recording {
        Object ctx;
        final List<ColRec> order = new ArrayList<>();
        final Map<Long, ColRec> byXz = new HashMap<>();
        long firstXz = Long.MIN_VALUE;
        ColRec current;
        final List<String> orphanMinSurface = new ArrayList<>();
    }
    private static void closeMinSurface(Recording r) throws Exception {
        // A column's getMinSurfaceLevel (rule tree or frozen-ocean extension) stays in the context until a later
        // column recomputes it; this read happens before the next column's first tryApply can.
        long at = longField("Context", "lastMinSurfaceLevelUpdate", r.ctx);
        if (r.firstXz == Long.MIN_VALUE || at < r.firstXz) return;
        String value = Integer.toString(intField("Context", "minSurfaceLevel", r.ctx));
        ColRec owner = r.byXz.get(at);
        if (owner == null) {
            String orphan = (at - r.firstXz) + ":" + value;
            if (!r.orphanMinSurface.contains(orphan)) r.orphanMinSurface.add(orphan);
        } else if (owner.minSurface.equals("-")) owner.minSurface = value;
        else require(owner.minSurface.equals(value), "Cached minimum surface changed within one column");
    }
    private static Recording recordedSurface(Gen g) throws Exception {
        Recording r = new Recording();
        Object genuine = call("Settings", "surfaceRule", g.settings);
        Object steepHolder[] = new Object[1];
        Object recorder = Proxy.newProxyInstance(SurfaceColumnOracle.class.getClassLoader(), new Class<?>[]{type("RuleSource")}, (proxy, m, args) -> {
            if (m.getName().equals("apply") && args != null && args.length == 1) {
                require(r.ctx == null, "buildSurface applied the rule source more than once");
                r.ctx = args[0];
                Object rule = ((Function<Object, Object>) genuine).apply(args[0]);
                steepHolder[0] = field("Context", "steep", r.ctx);
                return Proxy.newProxyInstance(SurfaceColumnOracle.class.getClassLoader(), new Class<?>[]{type("SurfaceRule")}, (rp, rm, ra) -> {
                    if (rm.getName().equals(tryApplyName) && ra != null && ra.length == 3) {
                        int x = (int) ra[0], y = (int) ra[1], z = (int) ra[2];
                        long xz = longField("Context", "lastUpdateXZ", r.ctx);
                        if (r.current == null || r.current.xz != xz) {
                            if (r.current != null) closeMinSurface(r);
                            ColRec c = new ColRec();
                            c.lx = x & 15; c.lz = z & 15; c.xz = xz;
                            if (r.firstXz == Long.MIN_VALUE) r.firstXz = xz;
                            require(intField("Context", "blockX", r.ctx) == x && intField("Context", "blockZ", r.ctx) == z, "Context column differs from tryApply column");
                            // Taken before this column's first rule write: the column after the eroded-badlands extension.
                            c.after = column(g, g.chunk, c.lx, c.lz);
                            c.firstHeight = height(g.chunk, worldSurfaceWg, c.lx, c.lz);
                            c.surfaceDepth = intField("Context", "surfaceDepth", r.ctx);
                            r.current = c; r.order.add(c); r.byXz.put(xz, c);
                        }
                        ColRec c = r.current;
                        require(intField("Context", "blockY", r.ctx) == y, "Context blockY differs from tryApply y");
                        long steepBefore = longField("LazyCondition", "lastUpdate", steepHolder[0]);
                        Object state;
                        try { state = method("SurfaceRule", "tryApply", int.class, int.class, int.class).invoke(rule, x, y, z); }
                        catch (Throwable failure) { throw unwrap(failure); }
                        if (longField("LazyCondition", "lastUpdate", steepHolder[0]) != steepBefore) {
                            require(c.steepCall < 0, "Steep computed twice in one column");
                            c.steepCall = c.calls.size();
                            int lx = c.lx, lz = c.lz;
                            c.steepHeights = new int[]{height(g.chunk, worldSurfaceWg, lx, Math.max(lz - 1, 0)), height(g.chunk, worldSurfaceWg, lx, Math.min(lz + 1, 15)),
                                height(g.chunk, worldSurfaceWg, Math.max(lx - 1, 0), lz), height(g.chunk, worldSurfaceWg, Math.min(lx + 1, 15), lz)};
                            c.steep = c.steepCall + ":" + (((Boolean) field("LazyCondition", "result", steepHolder[0])) ? 1 : 0) + ":" + ints(c.steepHeights);
                        }
                        c.calls.add(new int[]{y, intField("Context", "stoneDepthAbove", r.ctx), intField("Context", "stoneDepthBelow", r.ctx),
                            intField("Context", "waterHeight", r.ctx), stateId(state)});
                        return state;
                    }
                    if (rm.isDefault()) return InvocationHandler.invokeDefault(rp, rm, ra);
                    throw new IllegalStateException("Unexpected original SurfaceRule call " + rm);
                });
            }
            if (m.isDefault()) return InvocationHandler.invokeDefault(proxy, m, args);
            throw new IllegalStateException("Unexpected original RuleSource call " + m);
        });
        // Exactly the arguments NoiseBasedChunkGenerator.buildSurface passes: chunk.getOrCreateNoiseChunk (the
        // retained NOISE-step object), settings.useLegacyRandomSource(), randomState.surfaceSystem().
        method("SurfaceSystem", "buildSurface", type("RandomState"), type("Manager"), type("Registry"), boolean.class, type("WGContext"), type("Chunk"), type("NoiseChunk"), type("RuleSource"))
            .invoke(g.system, g.randomState, g.manager, biomeRegistry, call("Settings", "useLegacyRandomSource", g.settings), g.wg, g.chunk, g.noiseChunk, recorder);
        require(r.ctx != null, "buildSurface never applied the rule source");
        if (r.current != null) closeMinSurface(r);
        return r;
    }
    private static void genuineSurface(Gen g) throws Exception {
        method("Generator", "buildSurface", type("Chunk"), type("WGContext"), type("RandomState"), type("StructureManager"), type("Manager"), type("Registry"), type("Blender"))
            .invoke(g.generator, g.chunk, g.wg, g.randomState, g.structures, g.manager, biomeRegistry, call("Blender", "empty", null));
    }
    private static String runs(List<int[]> calls) {
        StringJoiner out = new StringJoiner(";");
        int i = 0;
        while (i < calls.size()) {
            int[] s = calls.get(i);
            int n = 1;
            while (i + n < calls.size()) {
                int[] t = calls.get(i + n);
                if (t[0] != s[0] - n || t[1] != s[1] + n || t[2] != s[2] - n || t[3] != s[3]) break;
                n++;
            }
            out.add(s[0] + ":" + s[1] + ":" + s[2] + ":" + (s[3] == Integer.MIN_VALUE ? "-" : Integer.toString(s[3])) + ":" + n);
            i += n;
        }
        return calls.isEmpty() ? "-" : out.toString();
    }
    private static String outcomes(List<int[]> calls) {
        int[] values = new int[calls.size()];
        for (int i = 0; i < values.length; i++) values[i] = calls.get(i)[4];
        return values.length == 0 ? "-" : rle(values).replace("-1*", "n*");
    }

    // ---------------------------------------------------------------- diagnostics and features
    private static final class Column {
        String biome;
        boolean melt;
        int erodedWrites, frozenWrites, frozenSnow, frozenFromWater;
        boolean hole, holeWrite, airWrite, swampWater, lavaWater, multiRun, steepChanged;
    }
    private static Column[] analyse(Gen g, Snap s0, Snap fin, Recording r, BufferedWriter diag, String id) throws Exception {
        Column[] cols = new Column[256];
        Object system = g.system;
        Method value = method("NormalNoise", "getValue", double.class, double.class, double.class);
        Object random = field("SurfaceSystem", "noiseRandom", system);
        Method at = method("PositionalRandomFactory", "at", int.class, int.class, int.class);
        for (int lx = 0; lx < 16; lx++) for (int lz = 0; lz < 16; lz++) {
            int i = lx * 16 + lz, x = g.minX + lx, z = g.minZ + lz;
            Column c = cols[i] = new Column();
            Object holder = method("Manager", "getBiome", type("BlockPos")).invoke(g.manager, blockPos(x, s0.wswg[i] + 1, z));
            c.biome = key(holder);
            c.melt = (boolean) method("Biome", "shouldMeltFrozenOceanIcebergSlightly", type("BlockPos"), int.class)
                .invoke(call("Holder", "value", holder), blockPos(x, g.seaLevel, z), g.seaLevel);
            if (diag == null) continue;
            Object rs = at.invoke(random, x, 0, z);
            int a = (int) method("RandomSource", "nextInt", int.class).invoke(rs, 4);
            int b = (int) method("RandomSource", "nextInt", int.class).invoke(rs, 10);
            StringJoiner doubles = new StringJoiner(",");
            for (int k = 0; k < 4; k++) doubles.add(Long.toHexString(Double.doubleToRawLongBits((double) call("RandomSource", "nextDouble", rs))));
            line(diag, id, lx, lz, x, z, c.biome, c.melt ? 1 : 0,
                doubleText((double) value.invoke(field("SurfaceSystem", "badlandsSurfaceNoise", system), (double) x, 0.0, (double) z)),
                doubleText((double) value.invoke(field("SurfaceSystem", "badlandsPillarNoise", system), x * 0.2, 0.0, z * 0.2)),
                doubleText((double) value.invoke(field("SurfaceSystem", "badlandsPillarRoofNoise", system), x * 0.75, 0.0, z * 0.75)),
                doubleText((double) value.invoke(field("SurfaceSystem", "icebergSurfaceNoise", system), (double) x, 0.0, (double) z)),
                doubleText((double) value.invoke(field("SurfaceSystem", "icebergPillarNoise", system), x * 1.28, 0.0, z * 1.28)),
                doubleText((double) value.invoke(field("SurfaceSystem", "icebergPillarRoofNoise", system), x * 1.17, 0.0, z * 1.17)),
                a, b, doubles);
        }
        if (r == null) return cols;
        int air = stateId(call("Block", "defaultBlockState", airBlock)), ice = stateId(call("Block", "defaultBlockState", iceBlock));
        int water = stateId(call("Block", "defaultBlockState", waterBlock)), snow = stateId(call("Block", "defaultBlockState", snowBlock));
        for (ColRec rec : r.order) {
            int i = rec.lx * 16 + rec.lz;
            Column c = cols[i];
            int[] before = s0.states[i], loop = rec.after.clone();
            for (int k = 0; k < 384; k++) if (before[k] != rec.after[k]) c.erodedWrites++;
            Set<Integer> bottoms = new HashSet<>();
            for (int[] call : rec.calls) {
                if (call[4] >= 0) loop[call[0] + 64] = call[4];
                if (call[4] == air || call[4] == ice || call[4] == water) c.holeWrite = true;
                if (call[4] == air) c.airWrite = true;
                if (call[4] == water && (c.biome.equals("minecraft:swamp") || c.biome.equals("minecraft:mangrove_swamp"))) c.swampWater = true;
                if (call[3] != Integer.MIN_VALUE && call[3] - 1 >= -64 && call[3] - 1 < 320 && is(stateList.get(rec.after[call[3] - 1 + 64]), lavaBlock)) c.lavaWater = true;
                bottoms.add(call[0] - call[2] + 1);
            }
            c.multiRun = bottoms.size() >= 2;
            c.hole = rec.surfaceDepth <= 0;
            for (int k = 0; k < 384; k++) if (fin.states[i][k] != loop[k]) {
                c.frozenWrites++;
                if (fin.states[i][k] == snow) c.frozenSnow++;
                if (is(stateList.get(loop[k]), waterBlock)) c.frozenFromWater++;
            }
            if (rec.steepHeights != null) {
                int lx = rec.lx, lz = rec.lz;
                int[] pre = {s0.wswg[lx * 16 + Math.max(lz - 1, 0)], s0.wswg[lx * 16 + Math.min(lz + 1, 15)], s0.wswg[Math.max(lx - 1, 0) * 16 + lz], s0.wswg[Math.min(lx + 1, 15) * 16 + lz]};
                c.steepChanged = !Arrays.equals(pre, rec.steepHeights);
            }
        }
        return cols;
    }
    private static boolean frozen(String biome) {
        return biome.equals("minecraft:frozen_ocean") || biome.equals("minecraft:deep_frozen_ocean");
    }
    private static Set<String> features(Snap s0, Column[] cols) {
        Set<String> out = new TreeSet<>();
        for (int i = 0; i < 256; i++) {
            Column c = cols[i];
            if (c.erodedWrites > 0) out.add("eroded_pillar");
            if (frozen(c.biome) && c.frozenSnow > 0) out.add("iceberg_snow");
            if (frozen(c.biome) && c.frozenFromWater > 0) out.add("iceberg_from_water");
            if (frozen(c.biome) && c.frozenWrites > 0) out.add(c.melt ? "iceberg_melt_true" : "iceberg_melt_false");
            if (frozen(c.biome) && c.hole && c.holeWrite) out.add("frozen_hole");
            if (c.airWrite) out.add("rule_air_write");
            if (c.swampWater) out.add("swamp_water");
            if (c.lavaWater) out.add("lava_water_height");
            if (c.multiRun) out.add("multi_run");
            if (s0.wswg[i] == 319) out.add("top_320");
            if (c.steepChanged) out.add("steep_changed");
        }
        return out;
    }
    private static Map<String, Integer> biomeCounts(Column[] cols) {
        Map<String, Integer> out = new TreeMap<>();
        for (Column c : cols) out.merge(c.biome, 1, Integer::sum);
        return out;
    }

    // ---------------------------------------------------------------- main run per group
    private static void runGroup(Group group, BufferedWriter groups, BufferedWriter chunks, BufferedWriter columns, BufferedWriter diag, BufferedWriter arrays) throws Exception {
        Gen u, rg;
        try { u = generate(group); rg = generate(group); }
        catch (Throwable failure) {
            line(groups, group.id, group.preset, group.seed, group.cx, group.cz, group.origin, "generation-exception:" + exception(failure),
                "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-", "-");
            return;
        }
        Snap s0 = snap(u, u.chunk), s0r = snap(rg, rg.chunk);
        boolean s0Equal = same(s0, s0r) && u.arrays.equals(rg.arrays);
        for (Map.Entry<String, String> e : u.arrays.entrySet()) line(arrays, group.id, e.getKey(), e.getValue());
        String neighboursBefore = neighbours(u);
        String uException = "none", rException = "none";
        try { genuineSurface(u); } catch (Throwable failure) { uException = exception(failure); }
        Recording rec = null;
        try { rec = recordedSurface(rg); } catch (Throwable failure) { rException = exception(failure); }
        Snap fu = snap(u, u.chunk), fr = snap(rg, rg.chunk);
        boolean finalEqual = same(fu, fr);
        boolean neighboursUnchanged = neighboursBefore.equals(neighbours(u));
        StringJoiner corners = new StringJoiner(",");
        for (int[] d : new int[][]{{0, 0}, {1, 0}, {0, 1}, {1, 1}})
            corners.add(Integer.toString((int) method("NoiseChunk", "preliminarySurfaceLevel", int.class, int.class).invoke(u.noiseChunk, (group.cx + d[0]) * 16, (group.cz + d[1]) * 16)));
        Object factory = field("SurfaceSystem", "noiseRandom", u.system);
        require(type("XoroPositional").isInstance(factory), "Overworld SurfaceSystem random must be the Xoroshiro positional factory");
        Column[] cols = analyse(u, s0, fu, rec, diag, group.id);
        String order = "-", orphans = "-";
        if (rec != null) {
            StringJoiner o = new StringJoiner(",");
            for (ColRec c : rec.order) o.add((c.lx * 16 + c.lz) + "@" + (c.xz - rec.firstXz));
            order = o.toString();
            orphans = rec.orphanMinSurface.isEmpty() ? "-" : String.join(",", rec.orphanMinSurface);
        }
        line(groups, group.id, group.preset, group.seed, group.cx, group.cz, group.origin, "none", uException, rException,
            s0Equal ? 1 : 0, finalEqual ? 1 : 0, neighboursUnchanged ? 1 : 0, neighboursBefore, corners, u.seaLevel,
            call("WGContext", "getMinGenY", u.wg) + "," + call("WGContext", "getGenDepth", u.wg),
            longField("XoroPositional", "seedLo", factory) + "," + longField("XoroPositional", "seedHi", factory),
            String.join(",", features(s0, cols)), order, orphans);
        for (Object[] phase : new Object[][]{{"S0", s0}, {"FINAL", fu}}) {
            Snap s = (Snap) phase[1];
            StringJoiner states = new StringJoiner(";");
            for (int i = 0; i < 256; i++) states.add(rle(s.states[i]));
            line(chunks, group.id, phase[0], s.heightmaps, ints(s.wswg), ints(s.ofwg), s.post, s.blockEntities, s.pending, states);
        }
        if (rec != null) for (ColRec c : rec.order) {
            int i = c.lx * 16 + c.lz;
            line(columns, group.id, c.lx, c.lz, u.minX + c.lx, u.minZ + c.lz, cols[i].biome, s0.wswg[i], c.firstHeight, c.surfaceDepth,
                rle(c.after), runs(c.calls), outcomes(c.calls), c.calls.size(), c.steep, c.minSurface);
        }
    }

    // ---------------------------------------------------------------- scan (Paper): selection only
    private static final class Evaluation { Group group; Map<String, Integer> biomes; Set<String> features; String failure; }
    private static final Map<String, Evaluation> evaluated = new LinkedHashMap<>();
    private static Evaluation evaluate(Group group, BufferedWriter log) throws Exception {
        String k = group.preset + "/" + group.seed + "@" + group.cx + "," + group.cz;
        Evaluation cached = evaluated.get(k);
        if (cached != null) return cached;
        Evaluation e = new Evaluation();
        e.group = group;
        try {
            Gen g = generate(group);
            Snap s0 = snap(g, g.chunk);
            Recording r = recordedSurface(g);
            Snap fin = snap(g, g.chunk);
            Column[] cols = analyse(g, s0, fin, r, null, group.id);
            e.biomes = biomeCounts(cols);
            e.features = features(s0, cols);
        } catch (Throwable failure) {
            e.failure = exception(failure);
            e.biomes = Map.of();
            e.features = Set.of();
        }
        evaluated.put(k, e);
        line(log, group.id, group.cx, group.cz, e.failure == null ? "none" : e.failure, e.biomes.toString().replace(" ", ""), String.join(",", e.features));
        return e;
    }
    /** Genuine surface-depth spiral (as the accepted GEN-05.9 hole search); selection only. */
    private static List<int[]> holeCandidates(Object random, Object source, Object sampler, Method noise, String biome, int[] hit, int max) throws Exception {
        Object system = call("RandomState", "surfaceSystem", random);
        Method depth = method("SurfaceSystem", "getSurfaceDepth", int.class, int.class);
        List<int[]> out = new ArrayList<>();
        Set<String> seen = new HashSet<>();
        int bx = hit[0] * 16, bz = hit[1] * 16;
        for (int r = 0; r <= 384 && out.size() < max; r++) for (int ix = -r; ix <= r && out.size() < max; ix++) for (int iz = -r; iz <= r && out.size() < max; iz++) {
            if (Math.max(Math.abs(ix), Math.abs(iz)) != r) continue;
            int x = bx + ix * 4, z = bz + iz * 4;
            if ((int) depth.invoke(system, x, z) > 0) continue;
            for (int y : new int[]{40, 64}) {
                if (!key(noise.invoke(source, x >> 2, y >> 2, z >> 2, sampler)).equals(biome)) continue;
                if (seen.add((x >> 4) + "," + (z >> 4))) out.add(new int[]{x >> 4, z >> 4, r, y});
                break;
            }
        }
        return out;
    }
    @SuppressWarnings("unchecked")
    private static void scan(Path output, Path design) throws Exception {
        Map<String, Object> cases = parseJson(Files.readString(design.resolve("cases.json")));
        Map<String, Object> declared = (Map<String, Object>) cases.get("scan");
        int step = ((Number) declared.get("step_blocks")).intValue(), radius = ((Number) declared.get("radius_blocks")).intValue();
        int hitsPer = ((Number) declared.get("hits_per_biome")).intValue(), candidates = ((Number) declared.get("biome_candidates")).intValue();
        int accept = ((Number) declared.get("biome_accept_columns")).intValue();
        List<Object> levels = (List<Object>) declared.get("block_y_levels");
        Group probe = new Group(); probe.preset = "normal"; probe.seed = 0;
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
        Set<String> wanted = new HashSet<>(targets);
        for (String[] f : lines(design.resolve("features.tsv"))) wanted.addAll(Arrays.asList(f[1].split(",")));
        Map<String, List<int[]>> hits = new TreeMap<>();
        for (int ring = 0; ring * step <= radius; ring++) {
            for (int ix = -ring; ix <= ring; ix++) for (int iz = -ring; iz <= ring; iz++) {
                if (Math.max(Math.abs(ix), Math.abs(iz)) != ring) continue;
                int x = ix * step, z = iz * step;
                for (Object level : levels) {
                    int y = ((Number) level).intValue();
                    String biome = key(noise.invoke(source, x >> 2, y >> 2, z >> 2, sampler));
                    if (!wanted.contains(biome)) continue;
                    List<int[]> list = hits.computeIfAbsent(biome, b -> new ArrayList<>());
                    boolean seen = false;
                    for (int[] h : list) seen |= h[0] == (x >> 4) && h[1] == (z >> 4);
                    if (!seen && list.size() < hitsPer) list.add(new int[]{x >> 4, z >> 4, ring, y});
                }
            }
        }
        List<Group> selected = new ArrayList<>();
        Set<String> used = new HashSet<>();
        try (BufferedWriter log = Files.newBufferedWriter(output.resolve("evaluations.tsv"));
             BufferedWriter out = Files.newBufferedWriter(output.resolve("selected.tsv"))) {
            line(log, "group", "cx", "cz", "failure", "column_biomes", "features");
            line(out, "group", "preset", "seed", "cx", "cz", "origin");
            for (String t : targets) {
                List<int[]> list = hits.getOrDefault(t, List.of());
                Evaluation best = null; int bestCount = 0;
                for (int i = 0; i < Math.min(candidates, list.size()); i++) {
                    Group g = new Group(); g.preset = "normal"; g.seed = 0; g.cx = list.get(i)[0]; g.cz = list.get(i)[1];
                    g.id = "biome/" + t.substring("minecraft:".length());
                    g.origin = "scan:" + t + ":hit" + i + ":ring" + list.get(i)[2] + ":y" + list.get(i)[3];
                    if (used.contains(g.cx + "," + g.cz)) continue;
                    Evaluation e = evaluate(g, log);
                    int count = e.biomes.getOrDefault(t, 0);
                    if (count > bestCount) { best = e; bestCount = count; }
                    if (count >= accept) break;
                }
                if (best == null) { line(out, "biome/" + t.substring("minecraft:".length()), "absent", "-", "-", "-", "scan:" + t + ":absent:hits" + list.size()); continue; }
                Group g = new Group(); g.preset = "normal"; g.seed = 0; g.cx = best.group.cx; g.cz = best.group.cz; g.id = "biome/" + t.substring("minecraft:".length());
                g.origin = best.group.origin + ":columns" + bestCount;
                used.add(g.cx + "," + g.cz); selected.add(g);
                line(out, g.id, g.preset, g.seed, g.cx, g.cz, g.origin);
            }
            Map<String, Group> featureGroups = new HashMap<>();
            for (String[] f : lines(design.resolve("features.tsv"))) {
                String feature = f[0];
                Group found = null;
                for (Group s : selected) {
                    Evaluation e = evaluated.get(s.preset + "/" + s.seed + "@" + s.cx + "," + s.cz);
                    if (e != null && e.features.contains(feature)) { found = s; break; }
                }
                if (found != null) { featureGroups.put(feature, found); line(out, "feature/" + feature, "covered", "-", found.cx, found.cz, "by:" + found.id); continue; }
                int max = Integer.parseInt(f[2]), tried = 0;
                List<Object[]> stream = new ArrayList<>();
                for (String biome : f[1].split(",")) {
                    if (f[5].equals("hole")) {
                        List<int[]> list = hits.getOrDefault(biome, List.of());
                        for (int h = 0; h < Math.min(6, list.size()); h++) for (int[] c : holeCandidates(random, source, sampler, noise, biome, list.get(h), max)) stream.add(new Object[]{biome, c});
                    } else for (int[] h : hits.getOrDefault(biome, List.of())) stream.add(new Object[]{biome, h});
                }
                for (Object[] s : stream) {
                    if (tried >= max) break;
                    int[] h = (int[]) s[1];
                    if (used.contains(h[0] + "," + h[1])) continue;
                    Group g = new Group(); g.preset = f[4]; g.seed = 0; g.cx = h[0]; g.cz = h[1]; g.id = "feature/" + feature;
                    g.origin = "scan:feature:" + feature + ":" + s[0] + ":" + f[5] + ":ring" + h[2] + ":y" + h[3] + ":preset" + f[4];
                    tried++;
                    if (evaluate(g, log).features.contains(feature)) { found = g; break; }
                }
                if (found == null) { line(out, "feature/" + feature, "absent", "-", "-", "-", "scan:feature:" + feature + ":absent:tried" + tried); continue; }
                used.add(found.cx + "," + found.cz); selected.add(found); featureGroups.put(feature, found);
                line(out, found.id, found.preset, found.seed, found.cx, found.cz, found.origin);
            }
            for (String[] s : lines(design.resolve("seam-partners.tsv"))) {
                Group base = featureGroups.get(s[0]);
                if (base == null) { line(out, "seam/" + s[0], "absent", "-", "-", "-", "seam:" + s[0] + ":no-base"); continue; }
                Group g = new Group(); g.preset = "normal"; g.seed = 0; g.cx = base.cx + Integer.parseInt(s[1]); g.cz = base.cz + Integer.parseInt(s[2]);
                g.id = "seam/" + s[0]; g.origin = "seam:" + base.id + ":" + s[1] + "," + s[2];
                if (!used.add(g.cx + "," + g.cz)) { line(out, g.id, "covered", "-", g.cx, g.cz, "seam:duplicate"); continue; }
                selected.add(g);
                line(out, g.id, g.preset, g.seed, g.cx, g.cz, g.origin);
            }
        }
        System.out.println("Scan selected " + selected.size() + " groups after " + evaluated.size() + " genuine Paper evaluations");
    }

    // ---------------------------------------------------------------- main
    private static void setup(String[] args) throws Exception {
        inner = Path.of(args[2]).toRealPath();
        helperRoot = Path.of(args[3]).toRealPath();
        symbols = new Properties();
        try (var in = Files.newBufferedReader(Path.of(args[4]))) { symbols.load(in); }
        origin(SurfaceColumnOracle.class, helperRoot, "helper/main");
        for (String alias : Files.readAllLines(Path.of(args[7]))) type(alias);
        if (mode.equals("paper")) for (String alias : Files.readAllLines(Path.of(args[8]))) { paperOnly.add(alias); type(alias); }
        Properties libraries = new Properties();
        try (var in = Files.newBufferedReader(Path.of(args[6]))) { libraries.load(in); }
        for (String name : new TreeSet<>(libraries.stringPropertyNames())) origin(Class.forName(name), Path.of(libraries.getProperty(name)), "library/" + name);
        Class<?> shortList = Class.forName("it.unimi.dsi.fastutil.shorts.ShortList");
        shortGet = shortList.getMethod("getShort", int.class);
        shortSize = Class.forName("java.util.Collection").getMethod("size");
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
        noiseStatus = field("Status", "NOISE", null);
        referencesStatus = field("Status", "STRUCTURE_REFERENCES", null);
        worldSurfaceWg = field("HeightmapTypes", "WORLD_SURFACE_WG", null);
        oceanFloorWg = field("HeightmapTypes", "OCEAN_FLOOR_WG", null);
        airBlock = field("Blocks", "AIR", null); waterBlock = field("Blocks", "WATER", null); lavaBlock = field("Blocks", "LAVA", null);
        iceBlock = field("Blocks", "ICE", null); snowBlock = field("Blocks", "SNOW_BLOCK", null); stoneBlock = field("Blocks", "STONE", null);
        tryApplyName = method("SurfaceRule", "tryApply", int.class, int.class, int.class).getName();
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
    public static void main(String[] args) throws Exception {
        require(args.length == 10, "MODE OUT INNER HELPER SYMBOLS DESIGN LIBRARIES ALIASES PAPER_ALIASES SELECTED");
        String requested = args[0];
        require(requested.equals("vanilla") || requested.equals("paper") || requested.equals("scan"), "Explicit original mode");
        mode = requested.equals("scan") ? "paper" : requested;
        setup(args);
        Path output = Path.of(args[1]).toRealPath();
        Path design = Path.of(args[5]).toRealPath();
        if (requested.equals("scan")) { scan(output, design); writeTables(output); return; }
        List<Group> groups = new ArrayList<>();
        for (String[] r : lines(design.resolve("fixed-groups.tsv"))) {
            Group g = new Group(); g.id = r[0]; g.preset = r[1]; g.seed = Long.parseLong(r[2]); g.cx = Integer.parseInt(r[3]); g.cz = Integer.parseInt(r[4]); g.origin = "fixed";
            groups.add(g);
        }
        for (String[] r : lines(Path.of(args[9]))) {
            if (r[1].equals("absent") || r[1].equals("covered")) continue;
            Group g = new Group(); g.id = r[0]; g.preset = r[1]; g.seed = Long.parseLong(r[2]); g.cx = Integer.parseInt(r[3]); g.cz = Integer.parseInt(r[4]); g.origin = r[5];
            groups.add(g);
        }
        require(groups.size() <= 96, "Declared group cap");
        try (BufferedWriter groupsOut = Files.newBufferedWriter(output.resolve("groups.tsv"));
             BufferedWriter chunks = Files.newBufferedWriter(output.resolve("chunks.tsv"));
             BufferedWriter columns = Files.newBufferedWriter(output.resolve("columns.tsv"));
             BufferedWriter diag = Files.newBufferedWriter(output.resolve("diagnostics.tsv"));
             BufferedWriter arrays = Files.newBufferedWriter(output.resolve("arrays.tsv"))) {
            line(groupsOut, "group", "preset", "seed", "cx", "cz", "origin", "generation", "u_exception", "r_exception", "s0_equal_u_r", "final_equal_u_r",
                "neighbours_unchanged", "neighbour_highest_filled_section", "preliminary_corners", "sea_level", "min_gen_y_gen_depth", "noise_random_seed_lo_hi",
                "features", "column_order_at_update_xz", "orphan_min_surface");
            line(chunks, "group", "phase", "heightmaps", "world_surface_wg", "ocean_floor_wg", "post_processing", "block_entities", "pending_block_entities", "columns_rle");
            line(columns, "group", "lx", "lz", "x", "z", "biome_at_height_plus_1", "s0_world_surface_wg", "first_call_world_surface_wg", "surface_depth",
                "column_at_first_call_rle", "call_runs", "call_outcomes", "calls", "steep", "min_surface");
            line(diag, "group", "lx", "lz", "x", "z", "biome", "melt_slightly", "badlands_surface", "badlands_pillar", "badlands_pillar_roof",
                "iceberg_surface", "iceberg_pillar", "iceberg_pillar_roof", "next_int_4", "next_int_10", "next_double_bits");
            line(arrays, "group", "chunk", "roster_indices_qy_qx_qz");
            for (Group g : groups) runGroup(g, groupsOut, chunks, columns, diag, arrays);
        }
        writeTables(output);
        Files.writeString(output.resolve("counts.tsv"), cells("groups", groups.size()) + "\n" + cells("states", stateList.size()) + "\n");
        System.out.println("Completed genuine original SURFACE runs for " + groups.size() + " groups; no carver, feature or server execution");
    }
    private static void writeTables(Path output) throws Exception {
        try (BufferedWriter out = Files.newBufferedWriter(output.resolve("states.tsv"))) {
            line(out, "index", "state", "is_air", "has_fluid", "is_stone_block", "is_default_stone", "is_water_block", "is_lava_block", "blocks_motion", "has_block_entity");
            for (String r : stateRows) { out.write(r); out.newLine(); }
        }
        try (BufferedWriter out = Files.newBufferedWriter(output.resolve("origins.tsv"))) { for (var e : origins.entrySet()) { out.write(e.getKey() + "\t" + e.getValue()); out.newLine(); } }
    }
    @SuppressWarnings("unchecked")
    private static Map<String, Object> parseJson(String text) throws Exception {
        Class<?> parser = Class.forName("com.google.gson.JsonParser");
        Object element = parser.getMethod("parseString", String.class).invoke(null, text);
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
