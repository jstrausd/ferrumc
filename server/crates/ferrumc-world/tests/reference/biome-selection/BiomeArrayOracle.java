package ferrumc.biomearray;

import com.google.gson.*;
import com.mojang.serialization.*;
import java.io.PrintWriter;
import java.lang.reflect.*;
import java.nio.file.*;
import java.util.*;
import java.util.stream.Stream;
import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.core.*;
import net.minecraft.core.registries.Registries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.resources.*;
import net.minecraft.util.KeyDispatchDataCodec;
import net.minecraft.world.level.*;
import net.minecraft.world.level.biome.*;
import net.minecraft.world.level.chunk.*;
import net.minecraft.world.level.chunk.status.ChunkStatus;
import net.minecraft.world.level.dimension.*;
import net.minecraft.world.level.levelgen.*;
import net.minecraft.world.level.levelgen.blending.Blender;
import net.minecraft.world.level.levelgen.presets.*;

/** Real createBiomes and section getters; traces delegate every call once. */
public final class BiomeArrayOracle {
    static HolderLookup.Provider lookup;
    static MappedRegistry<Biome> registry;
    static JsonObject trees;
    static PrintWriter output;
    static JsonObject origins = new JsonObject();
    static int proxyCalls;
    static Climate.ParameterList<Holder<Biome>> sharedParameters;
    static int sharedIdentityChecks;

    static Object field(Object object, String name) throws Exception {
        for (Class<?> type=object.getClass();type!=null;type=type.getSuperclass()) {
            try { Field f=type.getDeclaredField(name);f.setAccessible(true);return f.get(object); }
            catch(NoSuchFieldException missing) { }
        }
        throw new NoSuchFieldException(name);
    }
    static void require(boolean value,String message) {if(!value)throw new IllegalStateException(message);}
    static void origin(Class<?> type, Path expected, String label) throws Exception {
        require(type.getProtectionDomain().getCodeSource()!=null,"missing CodeSource "+type);
        require(Files.isSameFile(Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()),expected),"wrong CodeSource "+type);
        origins.addProperty(type.getName(),label);
    }
    static JsonArray numbers(long... values) { JsonArray a=new JsonArray();for(long v:values)a.add(v);return a; }
    static String key(Holder<Biome> holder) {return holder.unwrapKey().orElseThrow().location().toString();}

    static final class History {
        final Climate.ParameterList<Holder<Biome>> list;
        final ThreadLocal<?> local;
        final IdentityHashMap<Object,Integer> ordinals=new IdentityHashMap<>();
        History(MultiNoiseBiomeSource source) throws Exception {
            Method method=MultiNoiseBiomeSource.class.getDeclaredMethod("parameters");method.setAccessible(true);
            @SuppressWarnings("unchecked") Climate.ParameterList<Holder<Biome>> values=(Climate.ParameterList<Holder<Biome>>)method.invoke(source);
            list=values;Object index=field(list,"index");local=(ThreadLocal<?>)field(index,"lastResult");
            String preset=list.values().size()==7593?"minecraft:overworld":"minecraft:nether";
            require(list.values().size()==7593||list.values().size()==5,"unexpected original parameter count");
            JsonArray nodes=trees.getAsJsonArray(preset);int[] cursor={0};verifyNode(field(index,"root"),nodes,cursor);
            require(cursor[0]==nodes.size(),"incomplete original tree verification");
        }
        void verifyNode(Object node,JsonArray nodes,int[] cursor) throws Exception {
            JsonObject expected=nodes.get(cursor[0]++).getAsJsonObject();
            Climate.Parameter[] axes=(Climate.Parameter[])field(node,"parameterSpace");
            require(axes.length==7,"wrong original axes");
            for(int i=0;i<7;i++){JsonArray span=expected.getAsJsonArray("axes").get(i).getAsJsonArray();require(span.get(0).getAsLong()==axes[i].min()&&span.get(1).getAsLong()==axes[i].max(),"tree axis mismatch");}
            if(expected.has("leaf")) {
                int ordinal=expected.get("leaf").getAsInt();require(node.getClass().getSimpleName().equals("Leaf"),"wrong leaf kind");
                require(field(node,"value")==list.values().get(ordinal).getSecond(),"original leaf value/ordinal mismatch");ordinals.put(node,ordinal);
            } else {
                Object[] children=(Object[])field(node,"children");require(children.length==expected.getAsJsonArray("children").size(),"wrong child count");
                for(int i=0;i<children.length;i++){require(cursor[0]==expected.getAsJsonArray("children").get(i).getAsInt(),"wrong child order");verifyNode(children[i],nodes,cursor);}
            }
        }
        Integer last() {Object node=local.get();if(node==null)return null;Integer ordinal=ordinals.get(node);require(ordinal!=null,"unknown retained leaf");return ordinal;}
        void cold(){local.remove();}
    }

    static final class TraceDensity implements DensityFunction {
        final DensityFunction delegate;final int axis;final JsonArray calls;
        TraceDensity(DensityFunction delegate,int axis,JsonArray calls){this.delegate=delegate;this.axis=axis;this.calls=calls;}
        @Override public double compute(FunctionContext context){double value=delegate.compute(context);JsonArray a=numbers(axis,context.blockX(),context.blockY(),context.blockZ());a.add(Long.toUnsignedString(Double.doubleToRawLongBits(value),16));calls.add(a);return value;}
        @Override public void fillArray(double[] a,ContextProvider p){throw new IllegalStateException("unexpected trace fillArray");}
        @Override public DensityFunction mapAll(Visitor v){throw new IllegalStateException("unexpected trace mapAll");}
        @Override public double minValue(){throw new IllegalStateException("unexpected trace minValue");}
        @Override public double maxValue(){throw new IllegalStateException("unexpected trace maxValue");}
        @Override public KeyDispatchDataCodec<? extends DensityFunction> codec(){throw new IllegalStateException("unexpected trace codec");}
    }
    static final class TraceSource extends BiomeSource {
        final BiomeSource delegate;final History history;final JsonArray trace=new JsonArray();
        TraceSource(BiomeSource delegate,History history){this.delegate=delegate;this.history=history;}
        @Override protected MapCodec<? extends BiomeSource> codec(){throw new IllegalStateException("unexpected source codec");}
        @Override protected Stream<Holder<Biome>> collectPossibleBiomes(){return delegate.possibleBiomes().stream();}
        @Override public Holder<Biome> getNoiseBiome(int x,int y,int z,Climate.Sampler sampler){
            JsonArray calls=new JsonArray();
            Climate.Sampler wrapped=new Climate.Sampler(new TraceDensity(sampler.temperature(),0,calls),new TraceDensity(sampler.humidity(),1,calls),new TraceDensity(sampler.continentalness(),2,calls),new TraceDensity(sampler.erosion(),3,calls),new TraceDensity(sampler.depth(),4,calls),new TraceDensity(sampler.weirdness(),5,calls),sampler.spawnTarget());
            Holder<Biome> result=delegate.getNoiseBiome(x,y,z,wrapped);
            JsonObject row=new JsonObject();row.add("quart",numbers(x,y,z));row.addProperty("biome",key(result));row.add("density_calls",calls);
            if(history!=null)row.add("ordinal",new Gson().toJsonTree(history.last()));
            trace.add(row);return result;
        }
    }

    static ProtoChunk chunk(LevelStem stem,ChunkPos position) {
        DimensionType type=stem.type().value();
        return new ProtoChunk(position,UpgradeData.EMPTY,LevelHeightAccessor.create(type.minY(),type.height()),registry,null);
    }
    static void fill(NoiseBasedChunkGenerator generator,RandomState random,ProtoChunk chunk,long seed) {
        ChunkPos position=chunk.getPos();
        LevelAccessor level=(LevelAccessor)Proxy.newProxyInstance(BiomeArrayOracle.class.getClassLoader(),new Class[]{LevelAccessor.class},(proxy,method,args)->{
            require(method.getName().equals("getChunk")&&args!=null&&args.length==3&&(int)args[0]==position.x&&(int)args[1]==position.z&&args[2]==ChunkStatus.STRUCTURE_REFERENCES,"unexpected original LevelAccessor call "+method);
            proxyCalls++;return chunk;
        });
        StructureManager manager=new StructureManager(level,new WorldOptions(seed,false,false),null);
        require(generator.createBiomes(random,Blender.empty(),manager,chunk).join()==chunk,"wrong original result chunk");
    }
    static JsonArray biomes(ProtoChunk chunk) {
        JsonArray array=new JsonArray();
        for(LevelChunkSection section:chunk.getSections())for(int y=0;y<4;y++)for(int x=0;x<4;x++)for(int z=0;z<4;z++)array.add(key(section.getNoiseBiome(x,y,z)));
        return array;
    }
    static void job(String preset,String dimension,long seed,ChunkPos position,String group,int sequence,boolean cold,boolean baseline) throws Exception {
        ResourceKey<WorldPreset> presetKey=ResourceKey.create(Registries.WORLD_PRESET,ResourceLocation.withDefaultNamespace(preset));
        WorldPreset world=lookup.lookupOrThrow(Registries.WORLD_PRESET).getOrThrow(presetKey).value();
        ResourceKey<LevelStem> stemKey=ResourceKey.create(Registries.LEVEL_STEM,ResourceLocation.withDefaultNamespace(dimension));
        LevelStem stem=world.createWorldDimensions().dimensions().get(stemKey);require(stem!=null,"missing original binding");
        NoiseBasedChunkGenerator original=(NoiseBasedChunkGenerator)stem.generator();BiomeSource source=original.getBiomeSource();
        History history=source instanceof MultiNoiseBiomeSource multi?new History(multi):null;
        if(group.startsWith("shared-")) {
            require(history!=null,"shared sequence requires multinoise");
            if(sequence==0)sharedParameters=history.list;
            else {
                require(history.list==sharedParameters,"original world presets did not share the parameter list");
                require(field(history.list,"index")==field(sharedParameters,"index"),"original world presets did not share the RTree object");
                sharedIdentityChecks++;
            }
        }
        if(history!=null&&cold)history.cold();Integer initial=history==null?null:history.last();
        RandomState random=RandomState.create(original.generatorSettings().value(),lookup.lookupOrThrow(Registries.NOISE),seed);
        JsonArray baselineBiomes=null;
        if(baseline) {require(cold,"baseline must be cold");ProtoChunk plain=chunk(stem,position);fill(original,random,plain,seed);baselineBiomes=biomes(plain);if(history!=null)history.cold();}
        TraceSource trace=new TraceSource(source,history);
        NoiseBasedChunkGenerator traced=new NoiseBasedChunkGenerator(trace,original.generatorSettings());
        ProtoChunk chunk=chunk(stem,position);int before=proxyCalls;fill(traced,random,chunk,seed);JsonArray array=biomes(chunk);
        require(proxyCalls-before==1,"expected one original empty-reference query");
        if(baseline)require(array.equals(baselineBiomes),"instrumentation changed original output");
        JsonObject row=new JsonObject();row.addProperty("world_preset","minecraft:"+preset);row.addProperty("dimension","minecraft:"+dimension);row.addProperty("seed",seed);row.add("chunk",numbers(position.x,position.z));row.addProperty("group",group);row.addProperty("sequence",sequence);row.addProperty("cold",cold);row.addProperty("baseline_verified",baseline);
        row.addProperty("noise_settings",original.generatorSettings().unwrapKey().orElseThrow().location().toString());row.addProperty("source_class",source.getClass().getName());row.addProperty("min_y",stem.type().value().minY());row.addProperty("height",stem.type().value().height());row.add("initial_ordinal",new Gson().toJsonTree(initial));row.add("final_ordinal",new Gson().toJsonTree(history==null?null:history.last()));row.add("biomes",array);row.add("trace",trace.trace);output.println(row);
    }

    public static void main(String[] args) throws Exception {
        require(args.length==4,"OUTPUT INNER TREES LIBRARY_ORIGINS required");Path out=Path.of(args[0]).toRealPath(),inner=Path.of(args[1]).toRealPath();
        for(Class<?> helper:List.of(BiomeArrayOracle.class,History.class,TraceDensity.class,TraceSource.class))origin(helper,out.resolve("classes"),"$fresh-helper-classes");
        String[] classes={"net.minecraft.SharedConstants","net.minecraft.server.Bootstrap","net.minecraft.data.registries.VanillaRegistries","net.minecraft.core.MappedRegistry","net.minecraft.core.RegistrationInfo","net.minecraft.core.Holder","net.minecraft.core.Holder$Reference","net.minecraft.core.HolderLookup$Provider","net.minecraft.core.RegistrySetBuilder","net.minecraft.world.level.LevelAccessor","net.minecraft.world.level.LevelHeightAccessor","net.minecraft.world.level.ChunkPos","net.minecraft.world.level.StructureManager","net.minecraft.world.level.chunk.ProtoChunk","net.minecraft.world.level.chunk.ChunkAccess","net.minecraft.world.level.chunk.LevelChunkSection","net.minecraft.world.level.chunk.PalettedContainer","net.minecraft.world.level.chunk.status.ChunkStatus","net.minecraft.world.level.biome.Climate","net.minecraft.world.level.biome.Climate$Sampler","net.minecraft.world.level.biome.Climate$ParameterList","net.minecraft.world.level.biome.Climate$RTree","net.minecraft.world.level.biome.Climate$RTree$Leaf","net.minecraft.world.level.biome.MultiNoiseBiomeSource","net.minecraft.world.level.biome.FixedBiomeSource","net.minecraft.world.level.biome.TheEndBiomeSource","net.minecraft.world.level.biome.BiomeSource","net.minecraft.world.level.dimension.DimensionType","net.minecraft.world.level.dimension.LevelStem","net.minecraft.world.level.levelgen.NoiseBasedChunkGenerator","net.minecraft.world.level.levelgen.NoiseGeneratorSettings","net.minecraft.world.level.levelgen.NoiseSettings","net.minecraft.world.level.levelgen.NoiseChunk","net.minecraft.world.level.levelgen.RandomState","net.minecraft.world.level.levelgen.Beardifier","net.minecraft.world.level.levelgen.DensityFunction","net.minecraft.world.level.levelgen.DensityFunction$SinglePointContext","net.minecraft.world.level.levelgen.WorldOptions","net.minecraft.world.level.levelgen.WorldDimensions","net.minecraft.world.level.levelgen.presets.WorldPresets","net.minecraft.world.level.levelgen.presets.WorldPreset","net.minecraft.world.level.levelgen.blending.Blender","net.minecraft.world.level.levelgen.BelowZeroRetrogen","net.minecraft.core.QuartPos","net.minecraft.core.SectionPos"};
        for(String name:classes)origin(Class.forName(name,false,BiomeArrayOracle.class.getClassLoader()),inner,"$original-paper-inner");
        JsonObject libraries=JsonParser.parseString(Files.readString(Path.of(args[3]))).getAsJsonObject();for(var entry:libraries.entrySet())origin(Class.forName(entry.getKey()),Path.of(entry.getValue().getAsString()),"$pinned-library");
        SharedConstants.tryDetectVersion();Bootstrap.bootStrap();require(!SharedConstants.debugGenerateSquareTerrainWithoutNoise,"debug preset active");lookup=VanillaRegistries.createLookup();
        registry=new MappedRegistry<>(Registries.BIOME,Lifecycle.stable());lookup.lookupOrThrow(Registries.BIOME).listElements().sorted(Comparator.comparing(h->h.key().location().toString())).forEach(h->registry.register(h.key(),h.value(),RegistrationInfo.BUILT_IN));require(registry.size()==65,"original biome bootstrap count");registry.freeze();
        trees=JsonParser.parseString(Files.readString(Path.of(args[2]))).getAsJsonObject();
        long[] seeds={0,-1,Long.MIN_VALUE,Long.MAX_VALUE};ChunkPos[] positions={new ChunkPos(0,0),new ChunkPos(-1,-1),new ChunkPos(1,0),new ChunkPos(-17,9)};
        try(PrintWriter stream=new PrintWriter(Files.newBufferedWriter(out.resolve("arrays.jsonl")))) {
            output=stream;int sequence=0;
            for(String preset:List.of("normal","large_biomes","amplified"))for(long seed:seeds)for(ChunkPos position:positions)job(preset,"overworld",seed,position,"cold",sequence++,true,position==positions[0]);
            for(long seed:seeds)for(ChunkPos position:positions)job("normal","the_nether",seed,position,"cold",sequence++,true,position==positions[0]);
            for(long seed:seeds)for(ChunkPos position:new ChunkPos[]{new ChunkPos(0,0),new ChunkPos(64,0),new ChunkPos(65,0),new ChunkPos(-65,0),new ChunkPos(100,-100),new ChunkPos(900,-800),new ChunkPos(1874999,0),new ChunkPos(-1875000,0),new ChunkPos(100,100),new ChunkPos(200,100),new ChunkPos(100,200),new ChunkPos(110,90),new ChunkPos(110,91),new ChunkPos(121,20),new ChunkPos(150,0),new ChunkPos(-150,0)})job("normal","the_end",seed,position,"end",sequence++,true,position.x==0);
            for(long seed:seeds)job("single_biome_surface","overworld",seed,new ChunkPos(-17,9),"fixed",sequence++,true,true);
            for(String dimension:List.of("overworld","the_nether"))for(String group:List.of("forward","reverse"))for(int i=0;i<positions.length;i++)job("normal",dimension,0,positions[group.equals("forward")?i:positions.length-1-i],group,i,i==0,false);
            String[] sharedPresets={"normal","large_biomes","amplified","normal"};
            for(String group:List.of("shared-forward","shared-reverse"))for(int i=0;i<positions.length;i++) {
                int pick=group.equals("shared-forward")?i:positions.length-1-i;
                job(sharedPresets[pick],"overworld",0,positions[pick],group,i,i==0,false);
            }
        }
        BiomeSource endSource=lookup.lookupOrThrow(Registries.WORLD_PRESET).getOrThrow(WorldPresets.NORMAL).value().createWorldDimensions().dimensions().get(LevelStem.END).generator().getBiomeSource();
        try(PrintWriter stream=new PrintWriter(Files.newBufferedWriter(out.resolve("end-probes.jsonl")))) {
            for(double value:new double[]{Math.nextDown(0.25),0.25,Math.nextUp(0.25),Math.nextDown(-0.0625),-0.0625,Math.nextUp(-0.0625),Math.nextDown(-0.21875),-0.21875,Math.nextUp(-0.21875)})for(int x:new int[]{256,260,-260}) {
                JsonArray calls=new JsonArray();DensityFunction zero=DensityFunctions.zero();
                Climate.Sampler sampler=new Climate.Sampler(zero,zero,zero,new TraceDensity(DensityFunctions.constant(value),3,calls),zero,zero,List.of());
                Holder<Biome> result=endSource.getNoiseBiome(x,-16,0,sampler);JsonObject row=new JsonObject();row.add("quart",numbers(x,-16,0));row.addProperty("input_bits",Long.toUnsignedString(Double.doubleToRawLongBits(value),16));row.addProperty("biome",key(result));row.add("density_calls",calls);stream.println(row);
            }
        }
        require(sharedIdentityChecks==6,"incomplete shared object identity checks");
        JsonObject receipt=new JsonObject();receipt.add("code_sources",origins);receipt.addProperty("original_structure_reference_calls",proxyCalls);receipt.addProperty("original_bootstrap_biomes",registry.size());receipt.addProperty("shared_original_parameter_list_and_rtree_identity_checks",sharedIdentityChecks);Files.writeString(out.resolve("origins.json"),receipt+"\n");System.out.println("Complete original createBiomes arrays exported with fail-closed structure context");
    }
}
