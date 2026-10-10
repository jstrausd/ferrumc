package ferrumc.branchaddendum;

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
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ColumnPos;
import it.unimi.dsi.fastutil.longs.Long2IntMap;
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
public final class MaterialBranchAddendum {
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
        final JsonObject inputs;
        Leaf(String label, JsonObject inputs, Trace trace) {
            this.label = label;
            this.inputs = inputs;
            this.value = f64(label.equals("preliminary") ? inputs.getAsJsonObject("preliminary").get("default_f64_bits").getAsString() : inputs.getAsJsonObject("leaves").get(label).getAsString());
            this.trace = trace;
        }
        public double compute(DensityFunction.FunctionContext context) {
            double observed = value;
            if (label.equals("preliminary")) {
                JsonArray overrides = inputs.getAsJsonObject("preliminary").getAsJsonArray("overrides");
                if (overrides.size() > 128) throw new IllegalArgumentException("too many point overrides");
                for (JsonElement element : overrides) {
                    JsonObject row = element.getAsJsonObject();
                    if (row.get("x").getAsInt()==context.blockX() && row.get("y").getAsInt()==context.blockY() && row.get("z").getAsInt()==context.blockZ()) {
                        observed = f64(row.get("value_f64_bits").getAsString()); break;
                    }
                }
            }
            trace.density(label, context, observed);
            return observed;
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

    private static Field field(Class<?> owner, String name) throws Exception {
        Field field=owner.getDeclaredField(name); field.setAccessible(true); return field;
    }
    private static Aquifer.FluidPicker picker(JsonObject inputs, Trace trace) {
        JsonObject declared=inputs.getAsJsonObject("picker");
        JsonArray overrides=declared.getAsJsonArray("overrides");
        if (overrides.size()>16) throw new IllegalArgumentException("too many picker overrides");
        return (x,y,z)-> {
            JsonObject chosen=declared.getAsJsonObject("default");
            for(JsonElement element:overrides) {
                JsonObject row=element.getAsJsonObject();
                if(row.get("x").getAsInt()==x && row.get("y").getAsInt()==y && row.get("z").getAsInt()==z) {chosen=row.getAsJsonObject("status");break;}
            }
            Aquifer.FluidStatus value=status(chosen);
            JsonObject event=trace.event("global_fluid");
            event.addProperty("x",x);event.addProperty("y",y);event.addProperty("z",z);
            event.addProperty("height",value.fluidLevel());event.addProperty("fluid_raw_state_id",stateId(value.fluidType()));
            return value;
        };
    }
    private static JsonObject snapshot(Aquifer aq, NoiseChunk own) throws Exception {
        long[] locations=(long[])field(Aquifer.NoiseBasedAquifer.class,"aquiferLocationCache").get(aq);
        Aquifer.FluidStatus[] statuses=(Aquifer.FluidStatus[])field(Aquifer.NoiseBasedAquifer.class,"aquiferCache").get(aq);
        Long2IntMap preliminary=(Long2IntMap)field(NoiseChunk.class,"preliminarySurfaceLevel").get(own);
        JsonObject out=new JsonObject();
        out.addProperty("location_capacity",locations.length);out.addProperty("status_capacity",statuses.length);
        JsonArray locationEntries=new JsonArray(),statusEntries=new JsonArray(),preliminaryEntries=new JsonArray();
        for(int i=0;i<locations.length;i++) {
            if(locations[i]!=Long.MAX_VALUE) {
                JsonObject item=new JsonObject();item.addProperty("index",i);item.addProperty("packed_i64",Long.toString(locations[i]));
                item.addProperty("x",BlockPos.getX(locations[i]));item.addProperty("y",BlockPos.getY(locations[i]));item.addProperty("z",BlockPos.getZ(locations[i]));locationEntries.add(item);
            }
            if(statuses[i]!=null) {
                JsonObject item=new JsonObject();item.addProperty("index",i);item.addProperty("height",statuses[i].fluidLevel());item.addProperty("fluid_raw_state_id",stateId(statuses[i].fluidType()));statusEntries.add(item);
            }
        }
        long[] columns=preliminary.keySet().toLongArray();java.util.Arrays.sort(columns);
        for(long column:columns) {
            JsonObject item=new JsonObject();item.addProperty("packed_i64",Long.toString(column));item.addProperty("x",ColumnPos.getX(column));item.addProperty("z",ColumnPos.getZ(column));item.addProperty("surface_level",preliminary.get(column));preliminaryEntries.add(item);
        }
        out.add("locations",locationEntries);out.add("statuses",statusEntries);out.add("preliminary",preliminaryEntries);return out;
    }
    private static void locations(Aquifer aq, JsonObject inputs) throws Exception {
        if(!inputs.has("centers"))return;
        JsonArray centers=inputs.getAsJsonArray("centers");
        if(centers.size()!=12)throw new IllegalArgumentException("exactly twelve declared candidate centers required");
        long[] locations=(long[])field(Aquifer.NoiseBasedAquifer.class,"aquiferLocationCache").get(aq);
        Method index=method(Aquifer.NoiseBasedAquifer.class,"getIndex",int.class,int.class,int.class);
        java.util.HashSet<Integer> used=new java.util.HashSet<>();
        for(JsonElement element:centers) {
            JsonObject center=element.getAsJsonObject();JsonArray grid=center.getAsJsonArray("grid"),point=center.getAsJsonArray("point");
            int slot=(int)index.invoke(aq,grid.get(0).getAsInt(),grid.get(1).getAsInt(),grid.get(2).getAsInt());
            if(!used.add(slot))throw new IllegalArgumentException("duplicate location slot");
            locations[slot]=BlockPos.asLong(point.get(0).getAsInt(),point.get(1).getAsInt(),point.get(2).getAsInt());
        }
    }
    public static void main(String[] args) throws Exception {
        Path inner = Path.of(args[3]).toRealPath();
        Path output = Path.of(args[2]).toRealPath();
        Path classes = output.resolve("classes");
        String[] origins = {
            "net.minecraft.core.BlockPos", "net.minecraft.core.QuartPos", "net.minecraft.server.level.ColumnPos", "net.minecraft.SharedConstants", "net.minecraft.server.Bootstrap", "net.minecraft.data.registries.VanillaRegistries",
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
            Class<?> type = Class.forName(name, false, MaterialBranchAddendum.class.getClassLoader());
            Path actual = Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
            if (!Files.isSameFile(actual, inner)) throw new IllegalStateException("wrong original origin " + name + ": " + actual);
            JsonObject record = new JsonObject(); record.addProperty("class",name);record.addProperty("origin",actual.toString());originRecords.add(record);
        }
        for (Class<?> type : new Class<?>[]{MaterialBranchAddendum.class, Trace.class, Leaf.class, TraceFactory.class, TraceRandom.class}) {
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

        Method computeFluid=method(Aquifer.NoiseBasedAquifer.class,"computeFluid",int.class,int.class,int.class);
        Method aquiferStatus=method(Aquifer.NoiseBasedAquifer.class,"getAquiferStatus",long.class);
        Method pressure=method(Aquifer.NoiseBasedAquifer.class,"calculatePressure",DensityFunction.FunctionContext.class,MutableDouble.class,Aquifer.FluidStatus.class,Aquifer.FluidStatus.class);
        List<String> rows=new ArrayList<>();
        for(String line:Files.readAllLines(Path.of(args[1]))) {
            if(line.isBlank())continue;
            JsonObject row=JsonParser.parseString(line).getAsJsonObject(),inputs=row.getAsJsonObject("inputs");
            long seed=Long.parseLong(inputs.get("actual_seed").getAsString());
            RandomState actualRandom=RandomState.create(settings,lookup.lookupOrThrow(Registries.NOISE),seed);
            NoiseChunk own=new NoiseChunk(16/geometry.getCellWidth(),actualRandom,0,0,geometry,beard,settings,picker,Blender.empty());
            own.initializeForFirstCellX();own.advanceCellX(0);
            DensityFunction.FunctionContext point=context(inputs,own,geometry);
            Trace trace=new Trace(own);
            field(NoiseChunk.class,"initialDensityNoJaggedness").set(own,new Leaf("preliminary",inputs,trace));
            ((Long2IntMap)field(NoiseChunk.class,"preliminarySurfaceLevel").get(own)).clear();
            Aquifer aq=Aquifer.create(own,new ChunkPos(0,0),router(inputs,trace),new TraceFactory(trace,inputs,actualRandom.aquiferRandom()),geometry.minY(),geometry.height(),picker(inputs,trace));
            locations(aq,inputs);
            JsonObject result=new JsonObject();result.add("cache_before",snapshot(aq,own));
            JsonArray calls=new JsonArray(),snapshots=new JsonArray();
            for(JsonElement element:inputs.getAsJsonArray("calls")) {
                JsonObject call=element.getAsJsonObject(),answer=new JsonObject();
                answer.addProperty("events_start",trace.events.size());
                int x=inputs.get("x").getAsInt(),y=inputs.get("y").getAsInt(),z=inputs.get("z").getAsInt();
                switch(inputs.get("endpoint").getAsString()) {
                    case "enabled_substance" -> {
                        answer.addProperty("raw_state_id",stateId(aq.computeSubstance(point,f64(call.get("substance_f64_bits").getAsString()))));
                        answer.addProperty("schedule",aq.shouldScheduleFluidUpdate());
                    }
                    case "fluid_scan" -> {
                        Aquifer.FluidStatus value=(Aquifer.FluidStatus)computeFluid.invoke(aq,x,y,z);
                        answer.addProperty("height",value.fluidLevel());answer.addProperty("fluid_raw_state_id",stateId(value.fluidType()));
                    }
                    case "fluid_cache" -> {
                        Aquifer.FluidStatus value=(Aquifer.FluidStatus)aquiferStatus.invoke(aq,BlockPos.asLong(x,y,z));
                        answer.addProperty("height",value.fluidLevel());answer.addProperty("fluid_raw_state_id",stateId(value.fluidType()));
                    }
                    case "pressure" -> {
                        MutableDouble sentinel=new MutableDouble(f64(call.get("sentinel_f64_bits").getAsString()));
                        answer.addProperty("value_f64_bits",bits((double)pressure.invoke(aq,point,sentinel,status(inputs.getAsJsonObject("first")),status(inputs.getAsJsonObject("second")))));
                        answer.addProperty("sentinel_after_f64_bits",bits(sentinel.getValue()));
                    }
                    default ->throw new IllegalArgumentException("unknown endpoint");
                }
                answer.addProperty("events_end",trace.events.size());calls.add(answer);snapshots.add(snapshot(aq,own));
            }
            own.stopInterpolation();result.add("calls",calls);result.add("cache_after_calls",snapshots);
            JsonArray requests=new JsonArray(),rng=new JsonArray(),global=new JsonArray();
            for(JsonElement value:trace.events) {
                switch(value.getAsJsonObject().get("kind").getAsString()) {
                    case "density" ->requests.add(value);
                    case "global_fluid" ->global.add(value);
                    default ->rng.add(value);
                }
            }
            row.add("result",result);row.add("requests",requests);row.add("rng_events",rng);row.add("global_requests",global);row.add("events",trace.events);rows.add(row.toString());
        }
        Files.write(output.resolve("addendum.jsonl"),rows);
        System.out.println("actual Paper branch addendum="+rows.size()+" origins="+origins.length);
    }
}
