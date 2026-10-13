package ferrumc.vegetationoracle;

import java.io.BufferedWriter;
import java.lang.reflect.*;
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.atomic.AtomicReferenceArray;
import java.util.function.Function;
import java.util.function.Supplier;
import java.util.stream.Stream;

/** Genuine loaded registry objects and original methods; region and scripted draws are controlled probes, not FEATURES chunks. */
public final class VegetationOracle {
    private static Properties symbols;
    private static String mode;
    private static Path inner, helperRoot;
    private static final Map<String, Class<?>> types = new TreeMap<>();
    private static final Map<String, String> origins = new TreeMap<>();
    private static final Set<String> paperOnly = new TreeSet<>();
    private static final Map<String, Object> holders = new TreeMap<>();
    private static final List<String> roster = new ArrayList<>();
    private static Object lookup, biomeRegistry, registryAccess, biomesStatus, noiseStatus, surfaceStatus, startsStatus, referencesStatus;
    private static Object worldSurfaceWg, oceanFloorWg, carversStep, replaceableTag;
    private static Object airBlock, waterBlock, lavaBlock, stoneBlock, grassBlock, myceliumBlock, dirtBlock;
    private static final List<Object> configured = new ArrayList<>();
    private static final List<String> configuredKeys = new ArrayList<>();
    private static Object unsafe;
    private static Method allocate, shortGet, shortSize;
    private static String tryApplyName, computeName, scheduleName, applyName;
    private static java.util.concurrent.ExecutorService originalBackground;

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
            result = Class.forName(property("class." + alias), false, VegetationOracle.class.getClassLoader());
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
        String actual = paperOnly.contains(alias) ? name : property(symbol);
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
    /** A Paper-added field on a Mojang class (Paper mode only; it has no official mapping). */
    private static Field paperField(String alias, String name) throws Exception {
        require(mode.equals("paper"), "Paper-added field outside Paper mode");
        Field result = type(alias).getDeclaredField(name);
        result.setAccessible(true);
        return result;
    }
    private static Object field(String alias, String name, Object receiver) throws Exception {
        return fieldRef(alias, name).get(receiver);
    }
    private static int intField(String alias, String name, Object receiver) throws Exception {
        return fieldRef(alias, name).getInt(receiver);
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
    private static Object blockPos(int x, int y, int z) throws Exception {
        return type("BlockPos").getConstructor(int.class, int.class, int.class).newInstance(x, y, z);
    }
    private static Object proxy(Class<?> iface, InvocationHandler handler) {
        return Proxy.newProxyInstance(VegetationOracle.class.getClassLoader(), new Class<?>[]{iface}, handler);
    }

    private static void loadRegistries() throws Exception {
        Object pack = call("ServerPacksSource", "createVanillaPackSource", null);
        actualOrigin(pack, "vanilla-pack");
        Object resources = type("MultiPackResourceManager").getConstructor(type("PackType"), List.class).newInstance(field("PackType", "SERVER_DATA", null), List.of(pack));
        Object layered = call("RegistryLayer", "createRegistryAccess", null);
        Object staticAccess = method("LayeredRegistryAccess", "getLayer", Object.class).invoke(layered, field("RegistryLayer", "STATIC", null));
        List<?> pending = (List<?>) method("TagLoader", "loadTagsForExistingRegistries", type("ResourceManager"), type("RegistryAccess")).invoke(null, resources, staticAccess);
        Object loading = method("LayeredRegistryAccess", "getAccessForLoading", Object.class).invoke(layered, field("RegistryLayer", "WORLDGEN", null));
        List<?> lookups = (List<?>) method("TagLoader", "buildUpdatedLookups", type("Frozen"), List.class).invoke(null, loading, pending);
        Object worldgen = method("RegistryDataLoader", "load", type("ResourceManager"), List.class, List.class).invoke(null, resources, lookups, field("RegistryDataLoader", "WORLDGEN_REGISTRIES", null));
        List<Object> all = new ArrayList<>(lookups);
        all.addAll(((Stream<?>) call("Provider", "listRegistries", worldgen)).toList());
        lookup = method("Provider", "create", Stream.class).invoke(null, all.stream());
        actualOrigin(lookup, "registry-provider");
        for (Object p : pending) call("PendingTags", "apply", p);
        registryAccess = worldgen;
        biomeRegistry = method("RegistryAccess", "lookupOrThrow", type("ResourceKey")).invoke(worldgen, field("Registries", "BIOME", null));
        actualOrigin(biomeRegistry, "biome-registry");
    }

    public static void main(String[] args) throws Exception {
        mode="paper"; inner=Path.of(args[0]).toRealPath();helperRoot=Path.of(args[1]).toRealPath();symbols=new Properties();
        try(var r=Files.newBufferedReader(Path.of(args[2]))){symbols.load(r);}
        origin(VegetationOracle.class,helperRoot,"helper/main");
        call("Shared","tryDetectVersion",null);call("Bootstrap","bootStrap",null);loadRegistries();
        Object preset=method("Getter","getOrThrow",type("ResourceKey")).invoke(registryLookup("WORLD_PRESET"),resourceKey("WORLD_PRESET","minecraft:normal"));
        Map<?,?> dims=(Map<?,?>)call("Dimensions","dimensions",call("Preset","createWorldDimensions",call("Holder","value",preset)));
        var generator=(net.minecraft.world.level.chunk.ChunkGenerator)call("Stem","generator",dims.get(resourceKey("LEVEL_STEM","minecraft:overworld")));
        for(var t:List.of(net.minecraft.world.level.levelgen.feature.RandomPatchFeature.class,net.minecraft.world.level.levelgen.feature.SimpleBlockFeature.class,net.minecraft.world.level.block.VegetationBlock.class,net.minecraft.world.level.levelgen.placement.NoiseThresholdCountPlacement.class))origin(t,inner,"kernel/"+t.getName());
        try(var out=Files.newBufferedWriter(Path.of(args[3]))) {
            line(out,"case","feature","seed","cx","cz","support","slope","count","result","writes","draws");
            int belowCoordinate=0;boolean found=false;for(int c=-256;c<=256;c++){if(net.minecraft.world.level.biome.Biome.BIOME_INFO_NOISE.getValue(c*16/200.0,-c*16/200.0,false)<-.8){belowCoordinate=c;found=true;break;}}require(found,"original low-noise count probe");
            for(String feature:List.of("patch_grass_plain","patch_grass_forest"))for(long seed:new long[]{0,-1,Long.MIN_VALUE,Long.MAX_VALUE})for(int coordinate:new int[]{0,-1,17,belowCoordinate})for(String support:List.of("grass_block","dirt","farmland","podzol","dirt_path","water","stone","grass_block@desert"))for(boolean slope:new boolean[]{false,true}) {
                var pf=(net.minecraft.world.level.levelgen.placement.PlacedFeature)call("Holder","value",method("Getter","getOrThrow",type("ResourceKey")).invoke(registryLookup("PLACED_FEATURE"),resourceKey("PLACED_FEATURE","minecraft:"+feature)));
                var biome=(net.minecraft.core.Holder<net.minecraft.world.level.biome.Biome>)method("Getter","getOrThrow",type("ResourceKey")).invoke(registryLookup("BIOME"),resourceKey("BIOME",support.contains("@")?"minecraft:desert":feature.endsWith("plain")?"minecraft:plains":"minecraft:forest"));
                int cx=coordinate,cz=-coordinate,bx=cx*16,bz=cz*16;
                var block=net.minecraft.core.registries.BuiltInRegistries.BLOCK.getValue(net.minecraft.resources.ResourceLocation.parse("minecraft:"+support.split("@")[0]));
                var state=block.defaultBlockState();
                var writes=new LinkedHashMap<net.minecraft.core.BlockPos,net.minecraft.world.level.block.state.BlockState>();
                var draws=new ArrayList<String>();
                var real=new net.minecraft.world.level.levelgen.WorldgenRandom(new net.minecraft.world.level.levelgen.XoroshiroRandomSource(seed));
                long deco=real.setDecorationSeed(seed,bx,bz);real.setFeatureSeed(deco,feature.endsWith("plain")?54:59,9);
                var rng=(net.minecraft.util.RandomSource)proxy(net.minecraft.util.RandomSource.class,(p,m,a)->{Object v=m.invoke(real,a);require(m.getName().equals("nextInt"),"unexpected RNG "+m);draws.add(a[0]+":"+v);return v;});
                java.util.function.ToIntFunction<net.minecraft.core.BlockPos> baseHeight=p->64+(slope?Math.floorMod(p.getX()+p.getZ(),5):0);
                var level=(net.minecraft.world.level.WorldGenLevel)proxy(net.minecraft.world.level.WorldGenLevel.class,(p,m,a)->{
                    switch(m.getName()) {
                        case "getMinY":return -64;
                        case "getHeight":if(a==null||a.length==0)return 384;var pos=new net.minecraft.core.BlockPos((int)a[1],0,(int)a[2]);int h=baseHeight.applyAsInt(pos)+1;for(var q:writes.keySet())if(q.getX()==pos.getX()&&q.getZ()==pos.getZ())h=Math.max(h,q.getY()+1);return h;
                        case "getBiome":return biome;
                        case "getBlockState":var q=(net.minecraft.core.BlockPos)a[0];return writes.getOrDefault(q,q.getY()<=baseHeight.applyAsInt(q)?state:net.minecraft.world.level.block.Blocks.AIR.defaultBlockState());
                        case "setBlock":var q2=((net.minecraft.core.BlockPos)a[0]).immutable();require(q2.getY()>=-64&&q2.getY()<320,"write height");writes.put(q2,(net.minecraft.world.level.block.state.BlockState)a[1]);return true;
                        case "ensureCanWrite":return true;
                        case "getLevel":return null;
                        default:if(m.isDefault())return InvocationHandler.invokeDefault(p,m,a);throw new IllegalStateException("undeclared callback "+m);
                    }
                });
                boolean result=pf.placeWithBiomeCheck(level,generator,rng,new net.minecraft.core.BlockPos(bx,-64,bz));
                var encoded=new StringJoiner(";");for(var q:writes.keySet())encoded.add(q.getX()+","+q.getY()+","+q.getZ());
                double noise=net.minecraft.world.level.biome.Biome.BIOME_INFO_NOISE.getValue(bx/200.0,bz/200.0,false);
                line(out,feature+"/"+seed+"/"+coordinate+"/"+support+"/"+slope,feature,seed,cx,cz,support,slope,feature.endsWith("plain")?(noise<-.8?5:10):2,result,encoded,String.join(";",draws));
            }
        }
        try(var out=Files.newBufferedWriter(Path.of(args[3]+".origins"))){for(var e:origins.entrySet())line(out,e.getKey(),e.getValue().replace("\t","|"));}
        System.out.println("VEGETATION_DONE");
    }
}
