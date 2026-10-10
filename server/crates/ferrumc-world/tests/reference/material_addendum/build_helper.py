"""Build independent actual-method helper from the already frozen basic scaffold."""
from pathlib import Path

work = Path(__file__).resolve().parent
source = (work.parent / 'material_branch' / 'MaterialBranchOracle.java').read_text()
source = source.replace('package ferrumc.branchoracle;', 'package ferrumc.branchaddendum;')
source = source.replace('MaterialBranchOracle', 'MaterialBranchAddendum')
source = source.replace('import net.minecraft.core.registries.Registries;', 'import net.minecraft.core.registries.Registries;\nimport net.minecraft.core.BlockPos;\nimport net.minecraft.server.level.ColumnPos;\nimport it.unimi.dsi.fastutil.longs.Long2IntMap;')
source = source.replace('final Trace trace;\n        Leaf(', 'final Trace trace;\n        final JsonObject inputs;\n        Leaf(')
source = source.replace('this.value = f64(inputs.getAsJsonObject("leaves").get(label).getAsString());', 'this.inputs = inputs;\n            this.value = f64(label.equals("preliminary") ? inputs.getAsJsonObject("preliminary").get("default_f64_bits").getAsString() : inputs.getAsJsonObject("leaves").get(label).getAsString());')
source = source.replace('trace.density(label, context, value);\n            return value;', '''double observed = value;
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
            return observed;''')
prefix = source[:source.index('    public static void main(')]
origin = source[source.index('        Path inner ='):source.index('        NoiseChunk own =')]
origin = origin.replace('"net.minecraft.SharedConstants",', '"net.minecraft.core.BlockPos", "net.minecraft.core.QuartPos", "net.minecraft.server.level.ColumnPos", "net.minecraft.SharedConstants",')
methods = r'''
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
'''
main = r'''
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
'''
(work / 'MaterialBranchAddendum.java').write_text(prefix + methods + '    public static void main(String[] args) throws Exception {\n' + origin + main)
