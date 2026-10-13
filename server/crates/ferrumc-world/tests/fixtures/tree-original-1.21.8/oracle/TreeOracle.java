package ferrumc.treeoracle;

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
public final class TreeOracle {
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
            result = Class.forName(property("class." + alias), false, TreeOracle.class.getClassLoader());
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
        return Proxy.newProxyInstance(TreeOracle.class.getClassLoader(), new Class<?>[]{iface}, handler);
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
        mode="paper";inner=Path.of(args[0]).toRealPath();helperRoot=Path.of(args[1]).toRealPath();symbols=new Properties();try(var r=Files.newBufferedReader(Path.of(args[2]))){symbols.load(r);}origin(TreeOracle.class,helperRoot,"helper/main");
        call("Shared","tryDetectVersion",null);call("Bootstrap","bootStrap",null);loadRegistries();
        for(String species:List.of("oak","birch")) {
        var cf=(net.minecraft.world.level.levelgen.feature.ConfiguredFeature<?,?>)call("Holder","value",method("Getter","getOrThrow",type("ResourceKey")).invoke(registryLookup("CONFIGURED_FEATURE"),resourceKey("CONFIGURED_FEATURE","minecraft:"+species)));
        var preset=method("Getter","getOrThrow",type("ResourceKey")).invoke(registryLookup("WORLD_PRESET"),resourceKey("WORLD_PRESET","minecraft:normal"));
        var dims=(Map<?,?>)call("Dimensions","dimensions",call("Preset","createWorldDimensions",call("Holder","value",preset)));
        var generator=(net.minecraft.world.level.chunk.ChunkGenerator)call("Stem","generator",dims.get(resourceKey("LEVEL_STEM","minecraft:overworld")));
        if(species.equals("oak"))runPlacement(generator,Path.of(args[3]+".placement"));
        for(var c:List.of(net.minecraft.world.level.levelgen.feature.TreeFeature.class,net.minecraft.world.level.levelgen.feature.trunkplacers.StraightTrunkPlacer.class,net.minecraft.world.level.levelgen.feature.foliageplacers.BlobFoliagePlacer.class,net.minecraft.world.level.levelgen.feature.featuresize.TwoLayersFeatureSize.class))origin(c,inner,"kernel/"+c.getName());
        try(var out=Files.newBufferedWriter(Path.of(args[3]+"."+species))) {
            line(out,"seed","x","y","z","scenario","result","draws","writes","final","ticks");
            for(long seed:new long[]{0,1,-1,17,Long.MIN_VALUE,Long.MAX_VALUE})for(int y:new int[]{64,-64,-63,311,314})for(String scenario:List.of("grass","dirt","stone_floor","blocked","adjacent_log","persistent_leaf","leaf_distance","grass_in_trunk","water_in_trunk","mixed_leaf","mixed_log","coarse_dirt","water_in_crown","podzol","sand","snow_in_trunk","snowy_grass","mycelium","snowy_mycelium","farmland","floating_grass")) {
                int x=seed==17?-16:0,z=seed==-1?15:0;
                var origin=new net.minecraft.core.BlockPos(x,y,z);var states=new HashMap<net.minecraft.core.BlockPos,net.minecraft.world.level.block.state.BlockState>();var changes=new ArrayList<String>();var ticks=new TreeSet<String>();var draws=new ArrayList<String>();
                java.util.function.Function<net.minecraft.core.BlockPos,net.minecraft.world.level.block.state.BlockState> initial=p->{
                    if(scenario.equals("floating_grass")&&p.equals(origin.above(3).east(3)))return net.minecraft.world.level.block.Blocks.SHORT_GRASS.defaultBlockState();
                    if(scenario.equals("water_in_crown")&&p.equals(origin.above(4).east()))return net.minecraft.world.level.block.Blocks.WATER.defaultBlockState();
                    if(scenario.equals("snow_in_trunk")&&p.equals(origin))return net.minecraft.world.level.block.Blocks.SNOW.defaultBlockState();
                    if(p.getY()<y&&scenario.equals("podzol"))return net.minecraft.world.level.block.Blocks.PODZOL.defaultBlockState();
                    if(p.getY()<y&&scenario.equals("sand"))return net.minecraft.world.level.block.Blocks.SAND.defaultBlockState();
                    if(p.getY()<y&&scenario.equals("snowy_grass"))return net.minecraft.world.level.block.Blocks.GRASS_BLOCK.defaultBlockState().setValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.SNOWY,true);
                    if(p.getY()<y&&scenario.equals("mycelium"))return net.minecraft.world.level.block.Blocks.MYCELIUM.defaultBlockState();
                    if(p.getY()<y&&scenario.equals("snowy_mycelium"))return net.minecraft.world.level.block.Blocks.MYCELIUM.defaultBlockState().setValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.SNOWY,true);
                    if(p.getY()<y&&scenario.equals("farmland"))return net.minecraft.world.level.block.Blocks.FARMLAND.defaultBlockState();
                    if(p.getY()<y&&scenario.equals("coarse_dirt"))return net.minecraft.world.level.block.Blocks.COARSE_DIRT.defaultBlockState();
                    if(scenario.equals("grass_in_trunk")&&p.equals(origin))return net.minecraft.world.level.block.Blocks.SHORT_GRASS.defaultBlockState();
                    if(scenario.equals("water_in_trunk")&&p.equals(origin))return net.minecraft.world.level.block.Blocks.WATER.defaultBlockState();
                    if(scenario.equals("mixed_leaf")&&p.equals(origin.above(4).east()))return net.minecraft.world.level.block.Blocks.OAK_LEAVES.defaultBlockState().setValue(net.minecraft.world.level.block.LeavesBlock.DISTANCE,2);
                    if(scenario.equals("mixed_log")&&p.equals(origin.above(3).east()))return net.minecraft.world.level.block.Blocks.OAK_LOG.defaultBlockState();
                    if(p.getY()<y)return scenario.equals("dirt")?net.minecraft.world.level.block.Blocks.DIRT.defaultBlockState():scenario.equals("stone_floor")?net.minecraft.world.level.block.Blocks.STONE.defaultBlockState():net.minecraft.world.level.block.Blocks.GRASS_BLOCK.defaultBlockState();
                    if(scenario.equals("blocked")&&p.equals(origin.above(3)))return net.minecraft.world.level.block.Blocks.STONE.defaultBlockState();
                    if(scenario.equals("adjacent_log")&&p.getX()==x+1&&p.getZ()==z&&p.getY()>=y+1&&p.getY()<=y+6)return (species.equals("birch")?net.minecraft.world.level.block.Blocks.BIRCH_LOG:net.minecraft.world.level.block.Blocks.OAK_LOG).defaultBlockState();
                    if(scenario.equals("persistent_leaf")&&p.getX()==x+2&&p.getZ()==z+2&&p.getY()==y+4)return (species.equals("birch")?net.minecraft.world.level.block.Blocks.BIRCH_LEAVES:net.minecraft.world.level.block.Blocks.OAK_LEAVES).defaultBlockState().setValue(net.minecraft.world.level.block.LeavesBlock.PERSISTENT,true);
                    if(scenario.equals("leaf_distance")&&p.getX()==x+1&&p.getZ()==z&&p.getY()==y+4)return (species.equals("birch")?net.minecraft.world.level.block.Blocks.BIRCH_LEAVES:net.minecraft.world.level.block.Blocks.OAK_LEAVES).defaultBlockState().setValue(net.minecraft.world.level.block.LeavesBlock.DISTANCE,2);
                    return net.minecraft.world.level.block.Blocks.AIR.defaultBlockState();
                };
                java.util.function.Function<net.minecraft.core.BlockPos,net.minecraft.world.level.block.state.BlockState> state=p->states.getOrDefault(p,initial.apply(p));
                var real=new net.minecraft.world.level.levelgen.WorldgenRandom(new net.minecraft.world.level.levelgen.XoroshiroRandomSource(seed));
                var rng=(net.minecraft.util.RandomSource)proxy(net.minecraft.util.RandomSource.class,(p,m,a)->{Object v=m.invoke(real,a);require(m.getName().equals("nextInt"),"tree unexpected RNG "+m);draws.add(a[0]+":"+v);return v;});
                var level=(net.minecraft.world.level.WorldGenLevel)proxy(net.minecraft.world.level.WorldGenLevel.class,(p,m,a)->{
                    switch(m.getName()) {
                        case "getMinY":return -64;
                        case "getHeight":return 384;
                        case "getBlockState":return state.apply((net.minecraft.core.BlockPos)a[0]);
                        case "getFluidState":return state.apply((net.minecraft.core.BlockPos)a[0]).getFluidState();
                        case "isStateAtPosition":return ((java.util.function.Predicate<net.minecraft.world.level.block.state.BlockState>)a[1]).test(state.apply((net.minecraft.core.BlockPos)a[0]));
                        case "isFluidAtPosition":return ((java.util.function.Predicate<net.minecraft.world.level.material.FluidState>)a[1]).test(state.apply((net.minecraft.core.BlockPos)a[0]).getFluidState());
                        case "setBlock":var q=((net.minecraft.core.BlockPos)a[0]).immutable();var v=(net.minecraft.world.level.block.state.BlockState)a[1];states.put(q,v);changes.add(q.getX()+","+q.getY()+","+q.getZ()+"="+net.minecraft.world.level.block.Block.getId(v)+":"+a[2]);return true;
                        case "ensureCanWrite":return true;
                        case "getRandom":return real;
                        case "getLevel":return null;
                        case "scheduleTick":var q2=(net.minecraft.core.BlockPos)a[0];if(!(a[1] instanceof net.minecraft.world.level.block.LeavesBlock))return null;ticks.add(q2.getX()+","+q2.getY()+","+q2.getZ()+"="+net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey((net.minecraft.world.level.block.Block)a[1])+":"+a[2]);return null;
                        default:if(m.isDefault())return InvocationHandler.invokeDefault(p,m,a);throw new IllegalStateException("undeclared tree callback "+m);
                    }
                });
                boolean result=cf.place(level,generator,rng,origin);var finals=new TreeSet<String>();for(var e:states.entrySet()){var p=e.getKey();finals.add(p.getX()+","+p.getY()+","+p.getZ()+"="+net.minecraft.world.level.block.Block.getId(e.getValue()));}
                line(out,seed,x,y,z,scenario,result,String.join(";",draws),String.join(";",changes),String.join(";",finals),String.join(";",ticks));
            }
        }
        }
        try(var facts=Files.newBufferedWriter(Path.of(args[3]+".facts"))) {
            line(facts,"id","block","air","motion","replaceable","log","leaf","dirt","sapling","persistent","distance","distance1to7","waterSource","saplingBirch","snowSupport","snowTag");
            var stable=new HashSet<String>();for(var v:com.google.gson.JsonParser.parseString(Files.readString(Path.of(args[3]).getParent().getParent().getParent().getParent().resolve("src/tree_resources/admitted_blocks.json"))).getAsJsonArray())stable.add(v.getAsString());
            for(var block:net.minecraft.core.registries.BuiltInRegistries.BLOCK)for(var state:block.getStateDefinition().getPossibleStates()) {
                var level=(net.minecraft.world.level.LevelReader)proxy(net.minecraft.world.level.LevelReader.class,(p,m,a)->{if(m.getName().equals("getBlockState"))return state;if(m.isDefault())return InvocationHandler.invokeDefault(p,m,a);throw new IllegalStateException("undeclared support "+m);});
                boolean leaf=state.is(net.minecraft.tags.BlockTags.LEAVES);
                var variants=new ArrayList<String>();if(state.hasProperty(net.minecraft.world.level.block.state.properties.BlockStateProperties.DISTANCE))for(int d=1;d<=7;d++)variants.add(""+net.minecraft.world.level.block.Block.getId(state.setValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.DISTANCE,d)));
                line(facts,net.minecraft.world.level.block.Block.getId(state),net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(block),state.isAir(),state.blocksMotion(),state.is(net.minecraft.tags.BlockTags.REPLACEABLE_BY_TREES),state.is(net.minecraft.tags.BlockTags.LOGS),leaf,net.minecraft.world.level.levelgen.feature.Feature.isDirt(state),net.minecraft.world.level.block.Blocks.OAK_SAPLING.defaultBlockState().canSurvive(level,new net.minecraft.core.BlockPos(0,64,0)),leaf&&state.getValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.PERSISTENT),leaf?state.getValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.DISTANCE):0,String.join(",",variants),state.getFluidState().isSourceOfType(net.minecraft.world.level.material.Fluids.WATER),net.minecraft.world.level.block.Blocks.BIRCH_SAPLING.defaultBlockState().canSurvive(level,new net.minecraft.core.BlockPos(0,64,0)),(stable.contains(net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(block).toString())||state.is(net.minecraft.tags.BlockTags.LOGS)||leaf)&&net.minecraft.world.level.block.Blocks.SNOW.defaultBlockState().canSurvive(level,new net.minecraft.core.BlockPos(0,64,0)),state.is(net.minecraft.tags.BlockTags.SNOW));
            }
        }
        try(var out=Files.newBufferedWriter(Path.of(args[3]+".origins"))){for(var e:origins.entrySet())line(out,e.getKey(),e.getValue().replace("\t","|"));}
        Files.writeString(Path.of(args[3]+".done"),"TREE_DONE\n");
        System.out.println("TREE_DONE");
    }
    private static net.minecraft.world.level.levelgen.placement.PlacedFeature subsetLeaf(String species,boolean checked) throws Exception {
        var holder=(net.minecraft.core.Holder<net.minecraft.world.level.levelgen.feature.ConfiguredFeature<?,?>>)method("Getter","getOrThrow",type("ResourceKey")).invoke(registryLookup("CONFIGURED_FEATURE"),resourceKey("CONFIGURED_FEATURE","minecraft:"+species));
        return new net.minecraft.world.level.levelgen.placement.PlacedFeature(holder,checked?List.of(net.minecraft.data.worldgen.placement.PlacementUtils.filteredByBlockSurvival(species.equals("birch")?net.minecraft.world.level.block.Blocks.BIRCH_SAPLING:net.minecraft.world.level.block.Blocks.OAK_SAPLING)):List.of());
    }
    private static void runPlacement(net.minecraft.world.level.chunk.ChunkGenerator generator,Path output) throws Exception {
        var oak=subsetLeaf("oak",true);var birch=subsetLeaf("birch",true);var inline=subsetLeaf("oak",false);
        var noop=new net.minecraft.world.level.levelgen.placement.PlacedFeature(net.minecraft.core.Holder.direct(new net.minecraft.world.level.levelgen.feature.ConfiguredFeature<>(net.minecraft.world.level.levelgen.feature.Feature.NO_OP,net.minecraft.world.level.levelgen.feature.configurations.NoneFeatureConfiguration.INSTANCE)),List.of());
        var no=net.minecraft.core.Holder.direct(noop);var o=net.minecraft.core.Holder.direct(oak);var b=net.minecraft.core.Holder.direct(birch);var i=net.minecraft.core.Holder.direct(inline);
        var pf=new ArrayList<net.minecraft.world.level.levelgen.placement.PlacedFeature>();var original=new ArrayList<net.minecraft.world.level.levelgen.placement.PlacedFeature>();
        var configurations=List.of(
            new net.minecraft.world.level.levelgen.feature.configurations.RandomFeatureConfiguration(List.of(new net.minecraft.world.level.levelgen.feature.WeightedPlacedFeature(no,.0125f)),b),
            new net.minecraft.world.level.levelgen.feature.configurations.RandomFeatureConfiguration(List.of(new net.minecraft.world.level.levelgen.feature.WeightedPlacedFeature(no,.0025f),new net.minecraft.world.level.levelgen.feature.WeightedPlacedFeature(b,.2f),new net.minecraft.world.level.levelgen.feature.WeightedPlacedFeature(no,.1f),new net.minecraft.world.level.levelgen.feature.WeightedPlacedFeature(no,.0125f)),o),
            new net.minecraft.world.level.levelgen.feature.configurations.RandomFeatureConfiguration(List.of(new net.minecraft.world.level.levelgen.feature.WeightedPlacedFeature(no,.33333334f),new net.minecraft.world.level.levelgen.feature.WeightedPlacedFeature(no,.0125f)),i));
        var names=List.of("trees_birch","trees_birch_and_oak_leaf_litter","trees_plains");
        for(int f=0;f<3;f++) {
            var actual=(net.minecraft.world.level.levelgen.placement.PlacedFeature)call("Holder","value",method("Getter","getOrThrow",type("ResourceKey")).invoke(registryLookup("PLACED_FEATURE"),resourceKey("PLACED_FEATURE","minecraft:"+names.get(f))));original.add(actual);
            pf.add(new net.minecraft.world.level.levelgen.placement.PlacedFeature(net.minecraft.core.Holder.direct(new net.minecraft.world.level.levelgen.feature.ConfiguredFeature<>(net.minecraft.world.level.levelgen.feature.Feature.RANDOM_SELECTOR,configurations.get(f))),actual.placement()));
        }
        var place=net.minecraft.world.level.levelgen.placement.PlacedFeature.class.getDeclaredMethod("placeWithContext",net.minecraft.world.level.levelgen.placement.PlacementContext.class,net.minecraft.util.RandomSource.class,net.minecraft.core.BlockPos.class);place.setAccessible(true);
        try(var out=Files.newBufferedWriter(output)) {
            line(out,"seed","cx","cz","biome","support","slope","draws","final");
            for(long seed:new long[]{0,-1,Long.MIN_VALUE,Long.MAX_VALUE})for(int coordinate:new int[]{0,-1,17})for(String biomeName:List.of("plains","forest","birch_forest","desert"))for(String support:List.of("grass_block","dirt","podzol","stone","water","short_grass"))for(boolean slope:new boolean[]{false,true}) {
                int cx=coordinate,cz=-coordinate;var states=new HashMap<net.minecraft.core.BlockPos,net.minecraft.world.level.block.state.BlockState>();var draws=new ArrayList<String>();
                var biome=(net.minecraft.core.Holder<net.minecraft.world.level.biome.Biome>)method("Getter","getOrThrow",type("ResourceKey")).invoke(registryLookup("BIOME"),resourceKey("BIOME","minecraft:"+biomeName));
                var floor=net.minecraft.core.registries.BuiltInRegistries.BLOCK.getValue(net.minecraft.resources.ResourceLocation.parse("minecraft:"+(support.equals("short_grass")?"grass_block":support))).defaultBlockState();
                java.util.function.ToIntFunction<net.minecraft.core.BlockPos> height=p->64+(slope?Math.floorMod(p.getX()+p.getZ(),5):0);
                java.util.function.Function<net.minecraft.core.BlockPos,net.minecraft.world.level.block.state.BlockState> initial=p->p.getY()<=height.applyAsInt(p)?floor:support.equals("short_grass")&&p.getY()==height.applyAsInt(p)+1?net.minecraft.world.level.block.Blocks.SHORT_GRASS.defaultBlockState():net.minecraft.world.level.block.Blocks.AIR.defaultBlockState();
                java.util.function.Function<net.minecraft.core.BlockPos,net.minecraft.world.level.block.state.BlockState> state=p->states.getOrDefault(p,initial.apply(p));
                var real=new net.minecraft.world.level.levelgen.WorldgenRandom(new net.minecraft.world.level.levelgen.XoroshiroRandomSource(seed));
                var rng=(net.minecraft.util.RandomSource)proxy(net.minecraft.util.RandomSource.class,(p,m,a)->{Object v=m.invoke(real,a);require(m.getName().equals("nextInt")||m.getName().equals("nextFloat"),"placement unexpected RNG "+m);draws.add(m.getName().equals("nextInt")?a[0]+":"+v:"F:"+Integer.toUnsignedString(Float.floatToRawIntBits((float)v)));return v;});
                var level=(net.minecraft.world.level.WorldGenLevel)proxy(net.minecraft.world.level.WorldGenLevel.class,(p,m,a)->{
                    switch(m.getName()) {
                        case "getMinY":return -64;
                        case "getHeight":if(a==null||a.length==0)return 384;var t=(net.minecraft.world.level.levelgen.Heightmap.Types)a[0];for(int y=319;y>=-64;y--){var s=state.apply(new net.minecraft.core.BlockPos((int)a[1],y,(int)a[2]));if(t.isOpaque().test(s))return y+1;}return -64;
                        case "getBiome":return biome;
                        case "getBlockState":return state.apply((net.minecraft.core.BlockPos)a[0]);
                        case "getFluidState":return state.apply((net.minecraft.core.BlockPos)a[0]).getFluidState();
                        case "isStateAtPosition":return ((java.util.function.Predicate<net.minecraft.world.level.block.state.BlockState>)a[1]).test(state.apply((net.minecraft.core.BlockPos)a[0]));
                        case "isFluidAtPosition":return ((java.util.function.Predicate<net.minecraft.world.level.material.FluidState>)a[1]).test(state.apply((net.minecraft.core.BlockPos)a[0]).getFluidState());
                        case "setBlock":states.put(((net.minecraft.core.BlockPos)a[0]).immutable(),(net.minecraft.world.level.block.state.BlockState)a[1]);return true;
                        case "ensureCanWrite":return true;
                        case "getRandom":return real;
                        case "getLevel":return null;
                        case "scheduleTick":return null;
                        default:if(m.isDefault())return InvocationHandler.invokeDefault(p,m,a);throw new IllegalStateException("undeclared placement callback "+m);
                    }
                });
                for(int f=0;f<3;f++) {long deco=real.setDecorationSeed(seed,cx*16,cz*16);real.setFeatureSeed(deco,new int[]{24,25,52}[f],9);place.invoke(pf.get(f),new net.minecraft.world.level.levelgen.placement.PlacementContext(level,generator,Optional.of(original.get(f))),rng,new net.minecraft.core.BlockPos(cx*16,-64,cz*16));}
                var finals=new TreeSet<String>();for(var e:states.entrySet())if(e.getValue()!=initial.apply(e.getKey())){var p=e.getKey();finals.add(p.getX()+","+p.getY()+","+p.getZ()+"="+net.minecraft.world.level.block.Block.getId(e.getValue()));}
                line(out,seed,cx,cz,biomeName,support,slope,String.join(";",draws),String.join(";",finals));
            }
        }
    }

}
