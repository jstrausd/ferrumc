package ferrumc.branchoracle;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.mojang.serialization.JsonOps;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.SharedConstants;
import net.minecraft.core.registries.Registries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.resources.RegistryOps;
import net.minecraft.server.Bootstrap;
import net.minecraft.util.KeyDispatchDataCodec;
import net.minecraft.util.RandomSource;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Aquifer;
import net.minecraft.world.level.levelgen.DensityFunction;
import net.minecraft.world.level.levelgen.DensityFunctions;
import net.minecraft.world.level.levelgen.NoiseBasedChunkGenerator;
import net.minecraft.world.level.levelgen.NoiseChunk;
import net.minecraft.world.level.levelgen.NoiseGeneratorSettings;
import net.minecraft.world.level.levelgen.NoiseRouter;
import net.minecraft.world.level.levelgen.NoiseSettings;
import net.minecraft.world.level.levelgen.OreVeinifier;
import net.minecraft.world.level.levelgen.PositionalRandomFactory;
import net.minecraft.world.level.levelgen.RandomState;
import net.minecraft.world.level.levelgen.blending.Blender;
import org.apache.commons.lang3.mutable.MutableDouble;

/** Invokes pinned actual methods. Leaves and float arrays are declared inputs only. */
public final class MaterialBranchOracle {
    private static double f64(String bits) {
        return Double.longBitsToDouble(Long.parseUnsignedLong(bits, 16));
    }
    private static String bits(double value) {
        return String.format(java.util.Locale.ROOT, "%016x", Double.doubleToRawLongBits(value));
    }
    private static String bits(float value) {
        return String.format(java.util.Locale.ROOT, "%08x", Float.floatToRawIntBits(value));
    }
    private static BlockState fluid(String name) {
        return switch (name) {
            case "water" -> Blocks.WATER.defaultBlockState();
            case "lava" -> Blocks.LAVA.defaultBlockState();
            case "air" -> Blocks.AIR.defaultBlockState();
            default -> throw new IllegalArgumentException("undeclared fluid " + name);
        };
    }
    private static Method method(Class<?> owner, String name, Class<?>... parameters) throws Exception {
        Method method = owner.getDeclaredMethod(name, parameters);
        method.setAccessible(true);
        return method;
    }
    private static final class Trace {
        final NoiseChunk own;
        final JsonArray events = new JsonArray();
        Trace(NoiseChunk own) { this.own = own; }
        JsonObject event(String kind) {
            JsonObject event = new JsonObject();
            event.addProperty("kind", kind);
            events.add(event);
            return event;
        }
        void density(String label, DensityFunction.FunctionContext context, double value) {
            JsonObject event = event("density");
            event.addProperty("label", label);
            event.addProperty("x", context.blockX());
            event.addProperty("y", context.blockY());
            event.addProperty("z", context.blockZ());
            if (context == own) event.addProperty("own_context", "NoiseChunk");
            else if (context instanceof DensityFunction.SinglePointContext) event.addProperty("own_context", "SinglePoint");
            else throw new IllegalStateException("unexpected function context " + context.getClass());
            event.addProperty("value_f64_bits", bits(value));
        }
    }
    private static final class Leaf implements DensityFunction.SimpleFunction {
        final String label;
        final double value;
        final Trace trace;
        Leaf(String label, JsonObject inputs, Trace trace) {
            this.label = label;
            this.value = f64(inputs.getAsJsonObject("leaves").get(label).getAsString());
            this.trace = trace;
        }
        public double compute(DensityFunction.FunctionContext context) {
            trace.density(label, context, value);
            return value;
        }
        public double minValue() { return value; }
        public double maxValue() { return value; }
        public KeyDispatchDataCodec<? extends DensityFunction> codec() {
            throw new UnsupportedOperationException("declared leaf is not a serialized density graph");
        }
    }
    private static final class TraceFactory implements PositionalRandomFactory {
        final Trace trace;
        final float[] declared;
        final PositionalRandomFactory delegate;
        TraceFactory(Trace trace, JsonObject inputs, PositionalRandomFactory delegate) {
            this.trace = trace;
            this.delegate = delegate;
            JsonArray array = inputs.has("float_bits") ? inputs.getAsJsonArray("float_bits") : null;
            declared = array == null ? null : new float[array.size()];
            if (declared != null) {
                if (declared.length > 3) throw new IllegalArgumentException("at most three declared floats");
                for (int i = 0; i < declared.length; i++) {
                    declared[i] = Float.intBitsToFloat(Integer.parseUnsignedInt(array.get(i).getAsString(), 16));
                    if (!Float.isFinite(declared[i]) || declared[i] < 0.0F || declared[i] >= 1.0F)
                        throw new IllegalArgumentException("declared float outside nextFloat domain");
                }
            }
        }
        public RandomSource at(int x, int y, int z) {
            JsonObject event = trace.event("factory_at");
            event.addProperty("x", x); event.addProperty("y", y); event.addProperty("z", z);
            event.addProperty("mode", declared == null ? "actual" : "declared_floats");
            return new TraceRandom(trace, declared, declared == null ? delegate.at(x, y, z) : null);
        }
        public RandomSource fromHashOf(String name) { throw new UnsupportedOperationException("not an endpoint call"); }
        public RandomSource fromSeed(long seed) { throw new UnsupportedOperationException("not an endpoint call"); }
        public void parityConfigString(StringBuilder builder) { throw new UnsupportedOperationException("not an endpoint call"); }
    }
    private static final class TraceRandom implements RandomSource {
        final Trace trace;
        final float[] declared;
        final RandomSource delegate;
        int index;
        TraceRandom(Trace trace, float[] declared, RandomSource delegate) {
            this.trace = trace; this.declared = declared; this.delegate = delegate;
        }
        public float nextFloat() {
            float value;
            if (declared != null) {
                if (index == declared.length) throw new IllegalStateException("undeclared extra float draw");
                value = declared[index++];
            } else { value = delegate.nextFloat(); }
            JsonObject event = trace.event("next_float");
            event.addProperty("f32_bits", bits(value));
            return value;
        }
        public int nextInt(int bound) {
            if (delegate == null) throw new UnsupportedOperationException("undeclared bounded-int call");
            int value = delegate.nextInt(bound);
            JsonObject event = trace.event("next_int");
            event.addProperty("bound", bound); event.addProperty("value", value);
            return value;
        }
        public RandomSource fork() { throw new UnsupportedOperationException("not an endpoint call"); }
        public PositionalRandomFactory forkPositional() { throw new UnsupportedOperationException("not an endpoint call"); }
        public void setSeed(long seed) { throw new UnsupportedOperationException("not an endpoint call"); }
        public int nextInt() { throw new UnsupportedOperationException("not an endpoint call"); }
        public long nextLong() { throw new UnsupportedOperationException("not an endpoint call"); }
        public boolean nextBoolean() { throw new UnsupportedOperationException("not an endpoint call"); }
        public double nextDouble() { throw new UnsupportedOperationException("not an endpoint call"); }
        public double nextGaussian() { throw new UnsupportedOperationException("not an endpoint call"); }
    }
    private static NoiseRouter router(JsonObject inputs, Trace trace) {
        DensityFunction zero = DensityFunctions.zero();
        return new NoiseRouter(new Leaf("barrier", inputs, trace), new Leaf("floodedness", inputs, trace),
                new Leaf("spread", inputs, trace), new Leaf("lava", inputs, trace), zero, zero, zero,
                new Leaf("erosion", inputs, trace), new Leaf("depth", inputs, trace), zero, zero, zero,
                new Leaf("toggle", inputs, trace), new Leaf("ridged", inputs, trace), new Leaf("gap", inputs, trace));
    }
    private static Aquifer.FluidStatus status(JsonObject value) {
        return new Aquifer.FluidStatus(value.get("height").getAsInt(), fluid(value.get("fluid").getAsString()));
    }
    private static int stateId(BlockState state) {
        if (state == null) return -1;
        int id = Block.getId(state);
        if (id < 0) throw new IllegalStateException("unregistered state");
        return id;
    }
    private static DensityFunction.FunctionContext context(JsonObject inputs, NoiseChunk own, NoiseSettings geometry) {
        int x = inputs.get("x").getAsInt(), y = inputs.get("y").getAsInt(), z = inputs.get("z").getAsInt();
        if (inputs.get("context").getAsString().equals("SinglePoint")) return new DensityFunction.SinglePointContext(x,y,z);
        if (x < 0 || x >= geometry.getCellWidth() || z < 0 || z >= 16 || y < geometry.minY() || y >= geometry.minY()+geometry.height())
            throw new IllegalArgumentException("own context must fit initialized first X slice");
        int cy = Math.floorDiv(y - geometry.minY(), geometry.getCellHeight());
        int cz = Math.floorDiv(z, geometry.getCellWidth());
        own.selectCellYZ(cy, cz);
        own.updateForY(y, (double)Math.floorMod(y - geometry.minY(),geometry.getCellHeight())/geometry.getCellHeight());
        own.updateForX(x, (double)x/geometry.getCellWidth());
        own.updateForZ(z, (double)Math.floorMod(z,geometry.getCellWidth())/geometry.getCellWidth());
        if (own.blockX()!=x || own.blockY()!=y || own.blockZ()!=z) throw new IllegalStateException("wrong actual context position");
        return own;
    }
    public static void main(String[] args) throws Exception {
        Path inner = Path.of(args[3]).toRealPath();
        Path output = Path.of(args[2]).toRealPath();
        Path classes = output.resolve("classes");
        String[] origins = {
            "net.minecraft.SharedConstants", "net.minecraft.server.Bootstrap", "net.minecraft.data.registries.VanillaRegistries",
            "net.minecraft.world.level.block.Block", "net.minecraft.world.level.block.Blocks", "net.minecraft.world.level.block.state.BlockState",
            "net.minecraft.util.Mth", "net.minecraft.util.RandomSource", "net.minecraft.world.level.biome.OverworldBiomeBuilder",
            "net.minecraft.world.level.dimension.DimensionType", "net.minecraft.world.level.levelgen.Aquifer",
            "net.minecraft.world.level.levelgen.Aquifer$NoiseBasedAquifer", "net.minecraft.world.level.levelgen.Aquifer$FluidStatus",
            "net.minecraft.world.level.levelgen.OreVeinifier", "net.minecraft.world.level.levelgen.NoiseChunk",
            "net.minecraft.world.level.levelgen.NoiseChunk$BlockStateFiller", "net.minecraft.world.level.levelgen.NoiseChunk$NoiseInterpolator",
            "net.minecraft.world.level.levelgen.DensityFunction", "net.minecraft.world.level.levelgen.DensityFunction$SinglePointContext",
            "net.minecraft.world.level.levelgen.DensityFunctions", "net.minecraft.world.level.levelgen.RandomState",
            "net.minecraft.world.level.levelgen.PositionalRandomFactory", "net.minecraft.world.level.levelgen.NoiseGeneratorSettings",
            "net.minecraft.world.level.levelgen.NoiseSettings", "net.minecraft.world.level.levelgen.NoiseRouter",
            "net.minecraft.world.level.levelgen.NoiseBasedChunkGenerator", "net.minecraft.world.level.levelgen.blending.Blender",
            "net.minecraft.world.level.levelgen.XoroshiroRandomSource", "net.minecraft.world.level.levelgen.XoroshiroRandomSource$XoroshiroPositionalRandomFactory"
        };
        JsonArray originRecords = new JsonArray();
        for (String name : origins) {
            Class<?> type = Class.forName(name, false, MaterialBranchOracle.class.getClassLoader());
            Path actual = Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
            if (!Files.isSameFile(actual, inner)) throw new IllegalStateException("wrong original origin " + name + ": " + actual);
            JsonObject record = new JsonObject(); record.addProperty("class",name);record.addProperty("origin",actual.toString());originRecords.add(record);
        }
        for (Class<?> type : new Class<?>[]{MaterialBranchOracle.class, Trace.class, Leaf.class, TraceFactory.class, TraceRandom.class}) {
            if (!Files.isSameFile(Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()),classes))
                throw new IllegalStateException("wrong fresh helper origin " + type);
        }
        Files.writeString(output.resolve("origins.json"), originRecords.toString()+"\n");
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        var lookup = VanillaRegistries.createLookup();
        var ops = RegistryOps.create(JsonOps.INSTANCE, lookup);
        var definitions = JsonParser.parseString(Files.readString(Path.of(args[0]))).getAsJsonObject()
                .getAsJsonObject("registries").getAsJsonObject("minecraft:worldgen/noise_settings").getAsJsonObject("entries");
        NoiseGeneratorSettings settings = NoiseGeneratorSettings.CODEC.parse(ops,definitions.get("minecraft:overworld")).getOrThrow().value();
        NoiseSettings geometry = settings.noiseSettings();
        RandomState random = RandomState.create(settings,lookup.lookupOrThrow(Registries.NOISE),0L);
        Field beardField = Class.forName("net.minecraft.world.level.levelgen.DensityFunctions$BeardifierMarker").getDeclaredField("INSTANCE");
        beardField.setAccessible(true);
        var beard = (DensityFunctions.BeardifierOrMarker)beardField.get(null);
        Method pickerMethod = method(NoiseBasedChunkGenerator.class,"createFluidPicker",NoiseGeneratorSettings.class);
        Aquifer.FluidPicker picker = (Aquifer.FluidPicker)pickerMethod.invoke(null,settings);
        NoiseChunk own = new NoiseChunk(16/geometry.getCellWidth(),random,0,0,geometry,beard,settings,picker,Blender.empty());
        own.initializeForFirstCellX();own.advanceCellX(0);
        Method pressure = method(Aquifer.NoiseBasedAquifer.class,"calculatePressure",DensityFunction.FunctionContext.class,MutableDouble.class,Aquifer.FluidStatus.class,Aquifer.FluidStatus.class);
        Method surface = method(Aquifer.NoiseBasedAquifer.class,"computeSurfaceLevel",int.class,int.class,int.class,Aquifer.FluidStatus.class,int.class,boolean.class);
        Method spread = method(Aquifer.NoiseBasedAquifer.class,"computeRandomizedFluidSurfaceLevel",int.class,int.class,int.class,int.class);
        Method lava = method(Aquifer.NoiseBasedAquifer.class,"computeFluidType",int.class,int.class,int.class,Aquifer.FluidStatus.class,int.class);
        Method ore = method(OreVeinifier.class,"create",DensityFunction.class,DensityFunction.class,DensityFunction.class,PositionalRandomFactory.class);
        List<String> rows = new ArrayList<>();
        for (String line : Files.readAllLines(Path.of(args[1]))) {
            if (line.isBlank()) continue;
            JsonObject row = JsonParser.parseString(line).getAsJsonObject();
            JsonObject inputs = row.getAsJsonObject("inputs");
            int x=inputs.get("x").getAsInt(),y=inputs.get("y").getAsInt(),z=inputs.get("z").getAsInt();
            DensityFunction.FunctionContext point = context(inputs,own,geometry);
            Trace trace = new Trace(own);
            NoiseRouter declaredRouter = router(inputs,trace);
            TraceFactory factory = new TraceFactory(trace,inputs,random.aquiferRandom());
            Aquifer.FluidStatus defaultStatus = new Aquifer.FluidStatus(63,Blocks.WATER.defaultBlockState());
            Aquifer aq = Aquifer.create(own,new ChunkPos(0,0),declaredRouter,factory,geometry.minY(),geometry.height(),(a,b,c)->defaultStatus);
            JsonObject result = new JsonObject();
            switch (row.get("kind").getAsString()) {
                case "pressure" -> {
                    MutableDouble sentinel = new MutableDouble(f64(inputs.get("sentinel_f64_bits").getAsString()));
                    JsonArray values = new JsonArray(), sentinels = new JsonArray();
                    for (int call=0;call<inputs.get("calls").getAsInt();call++) {
                        values.add(bits((double)pressure.invoke(aq,point,sentinel,status(inputs.getAsJsonObject("first")),status(inputs.getAsJsonObject("second")))));
                        sentinels.add(bits(sentinel.getValue()));
                    }
                    result.add("value_f64_bits",values);result.add("sentinel_after_calls_f64_bits",sentinels);
                }
                case "surface_level" -> result.addProperty("surface_level",(int)surface.invoke(aq,x,y,z,status(inputs.getAsJsonObject("fluid")),inputs.get("max_surface").getAsInt(),inputs.get("fluid_present").getAsBoolean()));
                case "random_surface" -> result.addProperty("surface_level",(int)spread.invoke(aq,x,y,z,inputs.get("max_surface").getAsInt()));
                case "fluid_type" -> result.addProperty("raw_state_id",stateId((BlockState)lava.invoke(aq,x,y,z,status(inputs.getAsJsonObject("fluid")),inputs.get("surface_level").getAsInt())));
                case "ore" -> {
                    PositionalRandomFactory actual = inputs.has("actual_seed") ? RandomState.create(settings,lookup.lookupOrThrow(Registries.NOISE),Long.parseLong(inputs.get("actual_seed").getAsString())).oreRandom() : random.oreRandom();
                    TraceFactory oreFactory = new TraceFactory(trace,inputs,actual);
                    NoiseChunk.BlockStateFiller filler = (NoiseChunk.BlockStateFiller)ore.invoke(null,declaredRouter.veinToggle(),declaredRouter.veinRidged(),declaredRouter.veinGap(),oreFactory);
                    result.addProperty("raw_state_id",stateId(filler.calculate(point)));
                }
                case "disabled_substance" -> {
                    Aquifer disabled = Aquifer.createDisabled((a,b,c)->status(inputs.getAsJsonObject("fluid")));
                    result.addProperty("raw_state_id",stateId(disabled.computeSubstance(point,f64(inputs.get("substance_f64_bits").getAsString()))));
                    result.addProperty("schedule",disabled.shouldScheduleFluidUpdate());
                }
                default -> throw new IllegalArgumentException("unknown kind " + row.get("kind"));
            }
            JsonArray requests = new JsonArray(), rng = new JsonArray();
            for (JsonElement value : trace.events) {
                if (value.getAsJsonObject().get("kind").getAsString().equals("density")) requests.add(value);
                else rng.add(value);
            }
            row.add("result",result);row.add("requests",requests);row.add("rng_events",rng);row.add("events",trace.events);
            rows.add(row.toString());
        }
        own.stopInterpolation();
        Files.write(output.resolve("branches.jsonl"),rows);
        System.out.println("actual Paper branch endpoints="+rows.size()+" origins="+origins.length);
    }
}
