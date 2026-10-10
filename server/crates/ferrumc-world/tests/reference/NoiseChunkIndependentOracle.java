package ferrumc.independent;

import com.google.gson.*;
import com.mojang.serialization.*;
import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;

/**
 * Calls the actual pinned server methods through official semantic mappings.
 * Synthetic children only return declared values or failures and log requests.
 * This helper contains no density/cache/noise/lerp reference implementation.
 */
public final class NoiseChunkIndependentOracle {
    static final Gson GSON = new Gson();
    static final String LEVEL = "net.minecraft.world.level.levelgen.";
    static final long[] SEEDS = {0, 1, -1, Long.MIN_VALUE, Long.MAX_VALUE, 0x0123456789abcdefL, -0x1234567890abcdeL};
    static final String[] ROOTS = {"barrierNoise","fluidLevelFloodednessNoise","fluidLevelSpreadNoise","lavaNoise","temperature","vegetation","continents","erosion","depth","ridges","initialDensityWithoutJaggedness","finalDensity","veinToggle","veinRidged","veinGap"};
    static final String[] CLIMATE = {"temperature","humidity","continentalness","erosion","depth","weirdness"};
    static JsonObject mappings, origins;
    static boolean paper;
    static PrintWriter output;
    static DynamicOps<JsonElement> ops;
    static Codec<Object> settingsCodec;
    static Object lookup, noiseGetter, settingsGetter, emptyBlender, beard;
    static final Map<String,String> reverse = new HashMap<>();
    static final Map<String,Method> methods = new HashMap<>();
    static Path inner;
    static String name(String shortName) {
        if (shortName.contains(".") || shortName.endsWith("[]")) return shortName;
        return switch(shortName) {
            case "Holder","Holder$Direct","Holder$Reference","HolderGetter","HolderGetter$Provider","HolderLookup$Provider","QuartPos" -> "net.minecraft.core."+shortName;
            case "Registries" -> "net.minecraft.core.registries.Registries";
            case "ResourceKey","ResourceLocation","RegistryOps" -> "net.minecraft.resources."+shortName;
            case "Climate","Climate$Sampler","Climate$TargetPoint" -> "net.minecraft.world.level.biome."+shortName;
            case "Blender" -> LEVEL+"blending.Blender";
            case "MaterialRuleList" -> LEVEL+"material.MaterialRuleList";
            case "ImprovedNoise","NormalNoise","PerlinNoise","BlendedNoise","SimplexNoise" -> LEVEL+"synth."+shortName;
            case "CubicSpline","CubicSpline$Constant","CubicSpline$Multipoint","CubicSpline$CoordinateVisitor","ToFloatFunction","RandomSource","Mth" -> "net.minecraft.util."+shortName;
            case "SharedConstants" -> "net.minecraft.SharedConstants";
            case "Bootstrap" -> "net.minecraft.server.Bootstrap";
            case "VanillaRegistries" -> "net.minecraft.data.registries.VanillaRegistries";
            case "ChunkPos" -> "net.minecraft.world.level.ChunkPos";
            default -> LEVEL+shortName;
        };
    }
    static Class<?> type(String semantic) throws Exception {
        if (semantic.endsWith("[]")) return Array.newInstance(type(semantic.substring(0,semantic.length()-2)),0).getClass();
        return switch(semantic) {
            case "int" -> int.class; case "long" -> long.class; case "double" -> double.class;
            case "float" -> float.class; case "boolean" -> boolean.class; case "byte" -> byte.class;
            case "void" -> void.class;
            default -> {
                semantic=name(semantic);
                JsonObject map=mappings.getAsJsonObject(semantic);
                Class<?> resolved=Class.forName(paper || map==null ? semantic : map.get("class").getAsString(),false,NoiseChunkIndependentOracle.class.getClassLoader());
                if(semantic.startsWith("net.minecraft.")&&origins!=null&&!origins.has(semantic(resolved)))requireOrigin(resolved,inner);
                yield resolved;
            }
        };
    }
    static String semantic(Class<?> clazz) {
        String original=reverse.getOrDefault(clazz.getName(),clazz.getName());
        return original.startsWith(LEVEL) ? original.substring(LEVEL.length()) : original;
    }
    static String symbol(String owner,String group,String key) {
        if(paper)return key.contains("(")?key.substring(0,key.indexOf('(')):key;
        JsonElement item=mappings.getAsJsonObject(name(owner)).getAsJsonObject(group).get(key);
        if(item==null)throw new IllegalArgumentException("missing mapping "+owner+" "+key);
        // Official covariant-return mappings list the implementation before its bridge.
        if(item.isJsonArray())return item.getAsJsonArray().get(0).getAsString();
        return item.getAsString();
    }
    static Field slot(String owner,String field) throws Exception {
        Field result=type(owner).getDeclaredField(symbol(owner,"fields",field)); result.setAccessible(true); return result;
    }
    static Object get(Object instance,String owner,String field) throws Exception {return slot(owner,field).get(instance);}
    static void set(Object instance,String owner,String field,Object value) throws Exception {slot(owner,field).set(instance,value);}
    static Method method(String owner,String label,String...arguments) throws Exception {
        String[] full=Arrays.stream(arguments).map(a->switch(a){case "int","long","double","float","boolean","byte","void"->a;default->name(a);}).toArray(String[]::new);
        String key=name(owner)+"."+label+"("+String.join(",",full)+")";
        Method cached=methods.get(key);if(cached!=null)return cached;
        Class<?>[] args=new Class<?>[arguments.length];for(int i=0;i<args.length;i++)args[i]=type(arguments[i]);
        Method result=type(owner).getDeclaredMethod(symbol(owner,"methods",label+"("+String.join(",",full)+")"),args);
        result.setAccessible(true);methods.put(key,result);return result;
    }
    static Object call(Object object,String owner,String label,String[] args,Object...values) throws Exception {
        return method(owner,label,args).invoke(object,values);
    }
    static Object call(Object object,String owner,String label) throws Exception {return call(object,owner,label,new String[0]);}
    static Object create(String owner,String[] args,Object...values) throws Exception {
        Class<?>[] types=new Class<?>[args.length];for(int i=0;i<args.length;i++)types[i]=type(args[i]);
        Constructor<?> constructor=type(owner).getDeclaredConstructor(types);constructor.setAccessible(true);return constructor.newInstance(values);
    }
    static String bits(double value){return String.format(Locale.ROOT,"%016x",Double.doubleToRawLongBits(value));}
    static String bits(float value){return String.format(Locale.ROOT,"%08x",Float.floatToRawIntBits(value));}
    static JsonArray doubles(double[] array){JsonArray result=new JsonArray();for(double value:array)result.add(bits(value));return result;}
    static JsonArray doubles2(double[][] array){JsonArray result=new JsonArray();for(double[] row:array)result.add(doubles(row));return result;}
    static JsonObject row(String kind,String caseName){JsonObject result=new JsonObject();result.addProperty("kind",kind);result.addProperty("case",caseName);return result;}
    static void write(JsonObject row){output.println(GSON.toJson(row));if(output.checkError())throw new IllegalStateException("fixture write failed");}
    static JsonObject contextRecord(Object context,Object chunk) throws Exception {
        JsonObject result=new JsonObject();result.addProperty("own",context==chunk);
        for(String axis:List.of("X","Y","Z"))result.addProperty(axis.toLowerCase(Locale.ROOT),(Integer)call(context,"DensityFunction$FunctionContext","block"+axis));
        if(chunk!=null)for(String field:List.of("interpolationCounter","arrayInterpolationCounter","arrayIndex"))result.add(field,GSON.toJsonTree(get(chunk,"NoiseChunk",field)));
        return result;
    }
    static Object point(int x,int y,int z) throws Exception{return create("DensityFunction$SinglePointContext",new String[]{"int","int","int"},x,y,z);}
    static double compute(Object function,Object context) throws Exception{return (Double)call(function,"DensityFunction","compute",new String[]{"DensityFunction$FunctionContext"},context);}
    static void fill(Object function,double[] array,Object provider) throws Exception{call(function,"DensityFunction","fillArray",new String[]{"double[]","DensityFunction$ContextProvider"},array,provider);}
    static Object wrap(Object chunk,Object function) throws Exception{return call(chunk,"NoiseChunk","wrap",new String[]{"DensityFunction"},function);}
    static Object marker(String factory,Object child) throws Exception{return call(null,"DensityFunctions",factory,new String[]{"DensityFunction"},child);}
    static Object constant(double value) throws Exception{return call(null,"DensityFunctions","constant",new String[]{"double"},value);}
    static Object enumValue(String owner,String label) throws Exception {
        for(Object value:type(owner).getEnumConstants())if(((Enum<?>)value).name().equals(label))return value;
        throw new IllegalArgumentException("unknown enum "+owner+" "+label);
    }
    static Throwable cause(Throwable value){while(value instanceof InvocationTargetException || value instanceof UndeclaredThrowableException)value=value.getCause();return value;}
    static class ProbeFailure extends RuntimeException {}
    static final class Probe implements InvocationHandler {
        final String label;final JsonArray trace; final double[] values;final double min,max;final Object chunk;
        Object function;int scalarRequests,failAt=-1;boolean failFill;Map<Integer,Double> byY;
        Probe(String label,JsonArray trace,Object chunk,double min,double max,double...values) throws Exception {
            this.label=label;this.trace=trace;this.chunk=chunk;this.min=min;this.max=max;this.values=values;
            function=Proxy.newProxyInstance(NoiseChunkIndependentOracle.class.getClassLoader(),new Class<?>[]{type("DensityFunction")},this);
        }
        JsonObject request(String operation){JsonObject result=row("request",label);result.addProperty("operation",operation);return result;}
        public Object invoke(Object proxy,Method called,Object[] args) throws Throwable {
            if(called.getName().equals("hashCode"))return System.identityHashCode(proxy);
            if(called.getName().equals("equals"))return proxy==args[0];
            if(called.getName().equals("toString"))return "independent-probe:"+label;
            if(called.equals(method("DensityFunction","minValue")))return min;
            if(called.equals(method("DensityFunction","maxValue")))return max;
            if(called.equals(method("DensityFunction","compute","DensityFunction$FunctionContext"))) {
                JsonObject request=request("compute");request.add("context",contextRecord(args[0],chunk));trace.add(request);
                if(scalarRequests++==failAt)throw new ProbeFailure();
                if(byY!=null)return byY.getOrDefault((Integer)call(args[0],"DensityFunction$FunctionContext","blockY"),0.0);
                int x=(Integer)call(args[0],"DensityFunction$FunctionContext","blockX");
                return values.length==1?values[0]:values[Math.floorMod(x,values.length)];
            }
            if(called.equals(method("DensityFunction","fillArray","double[]","DensityFunction$ContextProvider"))) {
                double[] array=(double[])args[0];JsonObject request=request("fillArray");request.addProperty("length",array.length);trace.add(request);
                for(int i=0;i<array.length;i++){if(failFill&&i==1)throw new ProbeFailure();array[i]=values[i%values.length];}
                return null;
            }
            if(called.equals(method("DensityFunction","mapAll","DensityFunction$Visitor")))return call(args[0],"DensityFunction$Visitor","apply",new String[]{"DensityFunction"},proxy);
            if(called.isDefault())return InvocationHandler.invokeDefault(proxy,called,args);
            throw new UnsupportedOperationException("unexpected probe call "+called);
        }
    }
    static Object provider(JsonArray trace) throws Exception {
        return Proxy.newProxyInstance(NoiseChunkIndependentOracle.class.getClassLoader(),new Class<?>[]{type("DensityFunction$ContextProvider")},(proxy,method,args)->{
            if(method.equals(method("DensityFunction$ContextProvider","forIndex","int"))){
                int i=(Integer)args[0];JsonObject r=row("provider","foreign");r.addProperty("index",i);trace.add(r);return point(i,100+i,-i);
            }
            if(method.equals(method("DensityFunction$ContextProvider","fillAllDirectly","double[]","DensityFunction"))){
                double[] array=(double[])args[0];JsonObject r=row("provider","foreign");r.addProperty("direct_length",array.length);trace.add(r);
                for(int i=0;i<array.length;i++)array[i]=compute(args[1],call(proxy,"DensityFunction$ContextProvider","forIndex",new String[]{"int"},i));
                return null;
            }
            throw new UnsupportedOperationException("unexpected provider call "+method);
        });
    }
    static Object registry(Object getter,String registry,String resource) throws Exception {
        Object location=call(null,"ResourceLocation","parse",new String[]{"java.lang.String"},resource);
        Object key=call(null,"ResourceKey","create",new String[]{"ResourceKey","ResourceLocation"},get(null,"Registries",registry),location);
        Object holder=call(getter,"HolderGetter","getOrThrow",new String[]{"ResourceKey"},key);
        return call(holder,"Holder","value");
    }
    static Object random(Object settings,long seed) throws Exception {
        return call(null,"RandomState","create",new String[]{"NoiseGeneratorSettings","HolderGetter","long"},settings,noiseGetter,seed);
    }
    static Object chunk(Object settings,Object random,int x,int z) throws Exception {
        Object geometry=call(settings,"NoiseGeneratorSettings","noiseSettings");
        int width=(Integer)call(geometry,"NoiseSettings","getCellWidth");
        Object fluid=Proxy.newProxyInstance(NoiseChunkIndependentOracle.class.getClassLoader(),new Class<?>[]{type("Aquifer$FluidPicker")},(p,m,a)->{throw new IllegalStateException("unexpected actual fluid/material request");});
        return create("NoiseChunk",new String[]{"int","RandomState","int","int","NoiseSettings","DensityFunctions$BeardifierOrMarker","NoiseGeneratorSettings","Aquifer$FluidPicker","Blender"},16/width,random,x,z,geometry,beard,settings,fluid,emptyBlender);
    }
    static Object isolated(Object settings,long seed) throws Exception {
        Object chunk=chunk(settings,random(settings,seed),0,0);
        ((List<?>)get(chunk,"NoiseChunk","interpolators")).clear();((List<?>)get(chunk,"NoiseChunk","cellCaches")).clear();
        ((Map<?,?>)get(chunk,"NoiseChunk","wrapped")).clear();return chunk;
    }
    static Object material(Object chunk) throws Exception {
        Object rules=get(chunk,"NoiseChunk","blockStateRule");
        Object[] fillers=(Object[])get(rules,"MaterialRuleList","materialRuleList");
        Object density=null;
        for(Field field:fillers[0].getClass().getDeclaredFields())if(type("DensityFunction").isAssignableFrom(field.getType())){
            if(density!=null)throw new IllegalStateException("ambiguous actual material capture");field.setAccessible(true);density=field.get(fillers[0]);
        }
        if(density==null)throw new IllegalStateException("actual material capture missing");return density;
    }
    static long arrayStorage(Object value,Set<Object> arrays) {
        if(value==null||!value.getClass().isArray()||!arrays.add(value))return 0;
        if(value instanceof double[] d)return d.length*8L;
        long bytes=0;if(value instanceof Object[] all)for(Object child:all)bytes+=arrayStorage(child,arrays);return bytes;
    }
    static JsonObject census(Object chunk) throws Exception {
        JsonObject result=new JsonObject();Map<?,?> wrapped=(Map<?,?>)get(chunk,"NoiseChunk","wrapped");
        TreeMap<String,Integer> keys=new TreeMap<>(),wrappers=new TreeMap<>();Set<Object> seen=Collections.newSetFromMap(new IdentityHashMap<>());
        for(Object key:wrapped.keySet())keys.merge(semantic(key.getClass()),1,Integer::sum);
        for(Object value:wrapped.values())if(semantic(value.getClass()).startsWith("NoiseChunk$")&&seen.add(value))wrappers.merge(semantic(value.getClass()),1,Integer::sum);
        for(String field:List.of("blendAlpha","blendOffset")){Object value=get(chunk,"NoiseChunk",field);if(seen.add(value))wrappers.merge(semantic(value.getClass()),1,Integer::sum);}
        result.add("keyclasses",GSON.toJsonTree(keys));result.add("wrapperclasses",GSON.toJsonTree(wrappers));result.addProperty("map_entries",wrapped.size());
        for(String field:List.of("interpolators","cellCaches"))result.addProperty(field,((List<?>)get(chunk,"NoiseChunk",field)).size());
        Set<Object> arrays=Collections.newSetFromMap(new IdentityHashMap<>());long bytes=0;int maxLastArray=0;
        for(Object wrapper:seen)for(Field field:wrapper.getClass().getDeclaredFields())if(field.getType().isArray()){
            field.setAccessible(true);Object array=field.get(wrapper);bytes+=arrayStorage(array,arrays);
            if(semantic(wrapper.getClass()).equals("NoiseChunk$CacheOnce")&&array instanceof double[] d)maxLastArray=Math.max(maxLastArray,d.length);
        }
        result.addProperty("sample_array_bytes",bytes);result.addProperty("cache_once_max_array",maxLastArray);
        return result;
    }
    static int identity(IdentityHashMap<Object,Integer> ids,Object object){return ids.computeIfAbsent(object,k->ids.size());}
    static JsonArray identities(List<?> objects,IdentityHashMap<Object,Integer> ids){JsonArray result=new JsonArray();for(Object object:objects)result.add(identity(ids,object));return result;}
    static void constructors(String settingName,Object settings,long seed,String origin) throws Exception {
        Object random=random(settings,seed),chunk=chunk(settings,random,0,0),router=call(random,"RandomState","router");
        JsonObject result=row("constructor",settingName+"|"+origin+"|"+seed);result.add("before_second_wrap",census(chunk));
        IdentityHashMap<Object,Integer> ids=new IdentityHashMap<>();JsonArray registration=new JsonArray();
        for(String field:List.of("interpolators","cellCaches")){
            for(Object value:(List<?>)get(chunk,"NoiseChunk",field)){JsonObject entry=row("registration",field);entry.addProperty("id",identity(ids,value));entry.addProperty("class",semantic(value.getClass()));registration.add(entry);}
        }
        JsonArray visits=new JsonArray();List<Object> equalKeys=new ArrayList<>();
        Object visitor=Proxy.newProxyInstance(NoiseChunkIndependentOracle.class.getClassLoader(),new Class<?>[]{type("DensityFunction$Visitor")},(proxy,m,args)->{
            if(m.equals(method("DensityFunction$Visitor","apply","DensityFunction"))){
                Object key=args[0];int partition=-1;for(int i=0;i<equalKeys.size();i++)if(equalKeys.get(i).equals(key)){partition=i;break;}
                if(partition<0){partition=equalKeys.size();equalKeys.add(key);}
                Object value=wrap(chunk,key);JsonObject v=row("visit",semantic(key.getClass()));v.addProperty("equal_partition",partition);v.addProperty("key_identity",identity(ids,key));v.addProperty("value_identity",identity(ids,value));v.addProperty("value_class",semantic(value.getClass()));visits.add(v);return value;
            }
            if(m.equals(method("DensityFunction$Visitor","visitNoise","DensityFunction$NoiseHolder")))return args[0];
            throw new UnsupportedOperationException("unexpected actual wrapping visitor call "+m);
        });
        Object wrappedRouter=call(router,"NoiseRouter","mapAll",new String[]{"DensityFunction$Visitor"},visitor);
        JsonArray roots=new JsonArray();
        for(String field:ROOTS){Object value=call(wrappedRouter,"NoiseRouter",field);JsonObject r=row("root",field);r.addProperty("id",identity(ids,value));r.addProperty("class",semantic(value.getClass()));roots.add(r);}
        Object density=material(chunk);result.addProperty("material_id",identity(ids,density));
        Object sampler=call(chunk,"NoiseChunk","cachedClimateSampler",new String[]{"NoiseRouter","java.util.List"},router,call(settings,"NoiseGeneratorSettings","spawnTarget"));
        JsonArray climate=new JsonArray();for(String field:CLIMATE)climate.add(identity(ids,call(sampler,"Climate$Sampler",field)));
        result.add("constructor_registration",registration);result.add("second_wrap_visits",visits);result.add("roots",roots);result.add("climate_roots",climate);
        result.add("interpolator_identity_order",identities((List<?>)get(chunk,"NoiseChunk","interpolators"),ids));result.add("cellcache_identity_order",identities((List<?>)get(chunk,"NoiseChunk","cellCaches"),ids));
        result.add("after_climate",census(chunk));write(result);
    }
    static void randomControls() throws Exception {
        for(String algorithm:List.of("XoroshiroRandomSource","LegacyRandomSource"))for(long seed:SEEDS){
            Object rng=create(algorithm,new String[]{"long"},seed);JsonObject result=row("random",algorithm+"|"+seed);
            JsonArray values=new JsonArray();for(int i=0;i<12;i++)values.add(bits((Double)call(rng,"RandomSource","nextDouble")));result.add("next_double_bits",values);
            rng=create(algorithm,new String[]{"long"},seed);Object noise=create("ImprovedNoise",new String[]{"RandomSource"},rng);
            JsonArray offsets=new JsonArray();for(String axis:List.of("xo","yo","zo"))offsets.add(bits((Double)get(noise,"ImprovedNoise",axis)));
            result.add("offset_bits",offsets);byte[] permutation=(byte[])get(noise,"ImprovedNoise","p");
            JsonArray bytes=new JsonArray();for(byte value:permutation)bytes.add(value&255);result.add("permutation",bytes);
            long after=(Long)call(rng,"RandomSource","nextLong");result.addProperty("post_improved_next_long",Long.toUnsignedString(after));
            Object control=create(algorithm,new String[]{"long"},seed);for(int i=0;i<3;i++)call(control,"RandomSource","nextDouble");
            for(int i=0;i<256;i++)call(control,"RandomSource","nextInt",new String[]{"int"},256-i);
            long replay=(Long)call(control,"RandomSource","nextLong");if(after!=replay)throw new IllegalStateException("actual ImprovedNoise consume order differs");
            result.addProperty("consume_order_checked",true);
            JsonArray samples=new JsonArray();for(double[] p:new double[][]{{0,0,0},{13,-56,-29},{-1.25,73.5,255.75}})samples.add(bits((Double)call(noise,"ImprovedNoise","noise",new String[]{"double","double","double"},p[0],p[1],p[2])));result.add("noise_samples",samples);write(result);
        }
    }
    static void requireOrigin(Class<?> clazz,Path expected) throws Exception {
        var source=clazz.getProtectionDomain().getCodeSource();
        if(source==null||!Files.isSameFile(Path.of(source.getLocation().toURI()),expected))throw new IllegalStateException("wrong actual CodeSource "+clazz.getName());
        System.out.println("verified actual CodeSource "+clazz.getName()+" "+expected.toRealPath());
        origins.addProperty(semantic(clazz),clazz==NoiseChunkIndependentOracle.class?"$fresh-helper-classes":expected.toRealPath().toString());
    }
    @SuppressWarnings("unchecked")
    public static void main(String[] args) throws Exception {
        if(args.length!=7)throw new IllegalArgumentException("MODE WORLDGEN MAPPINGS OUTPUT INNER CLASSES LIBRARY_ORIGINS required");
        paper=args[0].equals("paper");inner=Path.of(args[4]);mappings=JsonParser.parseString(Files.readString(Path.of(args[2]))).getAsJsonObject();origins=new JsonObject();
        for(var entry:mappings.entrySet())reverse.put(paper?entry.getKey():entry.getValue().getAsJsonObject().get("class").getAsString(),entry.getKey());
        for(String cls:List.of("SharedConstants","Bootstrap","VanillaRegistries","RegistryOps","HolderGetter$Provider","HolderLookup$Provider","Holder","Holder$Direct","Holder$Reference","Registries","ResourceKey","ResourceLocation","DensityFunction","DensityFunction$FunctionContext","DensityFunction$ContextProvider","DensityFunction$Visitor","DensityFunction$SinglePointContext","DensityFunction$NoiseHolder","DensityFunctions","DensityFunctions$Marker","DensityFunctions$Ap2","DensityFunctions$MulOrAdd","DensityFunctions$RangeChoice","DensityFunctions$PureTransformer","DensityFunctions$TransformerWithContext","DensityFunctions$Constant","DensityFunctions$Spline","DensityFunctions$Spline$Coordinate","DensityFunctions$EndIslandDensityFunction","NoiseChunk","NoiseChunk$1","NoiseChunk$NoiseInterpolator","NoiseChunk$FlatCache","NoiseChunk$Cache2D","NoiseChunk$CacheOnce","NoiseChunk$CacheAllInCell","RandomState","RandomState$1NoiseWiringHelper","NoiseRouter","NoiseSettings","NoiseGeneratorSettings","Blender","Climate","Climate$Sampler","Climate$TargetPoint","MaterialRuleList","Aquifer","Aquifer$FluidPicker","XoroshiroRandomSource","Xoroshiro128PlusPlus","LegacyRandomSource","BitRandomSource","RandomSource","ImprovedNoise","PerlinNoise","NormalNoise","BlendedNoise","SimplexNoise","CubicSpline","CubicSpline$Constant","CubicSpline$Multipoint","Mth","QuartPos","ChunkPos"))requireOrigin(type(cls),inner);
        requireOrigin(NoiseChunkIndependentOracle.class,Path.of(args[5]));
        JsonObject libraryOrigins=JsonParser.parseString(Files.readString(Path.of(args[6]))).getAsJsonObject();
        for(var entry:libraryOrigins.entrySet())requireOrigin(Class.forName(entry.getKey()),Path.of(entry.getValue().getAsString()));
        call(null,"SharedConstants","tryDetectVersion");call(null,"Bootstrap","bootStrap");lookup=call(null,"VanillaRegistries","createLookup");
        ops=(DynamicOps<JsonElement>)call(null,"RegistryOps","create",new String[]{"com.mojang.serialization.DynamicOps","HolderLookup$Provider"},JsonOps.INSTANCE,lookup);
        settingsCodec=(Codec<Object>)get(null,"NoiseGeneratorSettings","CODEC");
        noiseGetter=call(lookup,"HolderGetter$Provider","lookupOrThrow",new String[]{"ResourceKey"},get(null,"Registries","NOISE"));
        settingsGetter=call(lookup,"HolderGetter$Provider","lookupOrThrow",new String[]{"ResourceKey"},get(null,"Registries","NOISE_SETTINGS"));
        emptyBlender=call(null,"Blender","empty");beard=get(null,"DensityFunctions$BeardifierMarker","INSTANCE");
        JsonObject definitions=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject().getAsJsonObject("registries").getAsJsonObject("minecraft:worldgen/noise_settings").getAsJsonObject("entries");
        Map<String,Object> settings=new TreeMap<>();for(String key:definitions.keySet()){Object holder=settingsCodec.parse(ops,definitions.get(key)).getOrThrow();settings.put(key,call(holder,"Holder","value"));}
        try(PrintWriter stream=new PrintWriter(Files.newBufferedWriter(Path.of(args[3])))){
            output=stream;JsonObject originRow=row("origins","actual");originRow.add("code_sources",origins);write(originRow);randomControls();
            for(var entry:settings.entrySet())for(String origin:List.of("decoded","native"))for(long seed:SEEDS)constructors(entry.getKey(),origin.equals("decoded")?entry.getValue():registry(settingsGetter,"NOISE_SETTINGS",entry.getKey()),seed,origin);
            wrappers(settings.get("minecraft:overworld"));arrays(settings.get("minecraft:overworld"));equalControls(settings.get("minecraft:overworld"));
            for(var entry:settings.entrySet())for(long seed:SEEDS){preliminary(entry.getKey(),entry.getValue(),seed);climate(entry.getKey(),entry.getValue(),seed);diagnostics(entry.getKey(),entry.getValue(),seed);}
            JsonObject actualOrigins=row("origins","after-all-actual-calls");actualOrigins.add("code_sources",origins);write(actualOrigins);
        }
    }
    interface Action {Object run() throws Exception;}
    static void observation(String kind,String label,Object chunk,JsonArray trace,Action action) throws Exception {
        JsonObject result=row(kind,label);
        try{
            Object value=action.run();
            if(value instanceof Double d)result.addProperty("result_bits",bits(d));
            else if(value instanceof double[] array)result.add("array_bits",doubles(array));
            else if(value!=null)result.add("result",GSON.toJsonTree(value));
        }catch(Throwable error){
            Throwable actual=cause(error);
            if(!(actual instanceof ProbeFailure||actual instanceof IllegalStateException||actual instanceof IndexOutOfBoundsException))throw error;
            result.addProperty("error",semantic(actual.getClass()));
        }
        if(chunk!=null){
            JsonObject state=new JsonObject();
            for(String field:List.of("interpolationCounter","arrayInterpolationCounter","arrayIndex","interpolating","fillingCell","inCellX","inCellY","inCellZ"))state.add(field,GSON.toJsonTree(get(chunk,"NoiseChunk",field)));
            result.add("state",state);
            List<JsonObject> states=new ArrayList<>();
            for(Object wrapper:((Map<?,?>)get(chunk,"NoiseChunk","wrapped")).values()){
                String cls=semantic(wrapper.getClass());JsonObject ws=row("wrapper-state",cls);
                if(cls.equals("NoiseChunk$CacheOnce")){
                    for(String field:List.of("lastCounter","lastArrayCounter","lastValue","lastArray")){
                        Object value=get(wrapper,cls,field);
                        if(value instanceof Double d)ws.addProperty(field,bits(d));
                        else if(value instanceof double[] d)ws.add(field,doubles(d));
                        else ws.add(field,GSON.toJsonTree(value));
                    }
                    states.add(ws);
                }else if(cls.equals("NoiseChunk$Cache2D")){
                    ws.addProperty("lastPos2D",Long.toUnsignedString((Long)get(wrapper,cls,"lastPos2D")));
                    ws.addProperty("lastValue",bits((Double)get(wrapper,cls,"lastValue")));states.add(ws);
                }
            }
            states.sort(Comparator.comparing(GSON::toJson));result.add("actual_wrapper_fields",GSON.toJsonTree(states));
        }
        result.add("trace",trace.deepCopy());write(result);trace.asList().clear();
    }
    static void wrappers(Object settings) throws Exception {
        JsonArray trace=new JsonArray();Object chunk=isolated(settings,0);
        Probe probe=new Probe("cache2d-child",trace,chunk,-20,20,13);
        Object wrapped=wrap(chunk,marker("cache2d",probe.function));
        for(int[] p:new int[][]{{1875066,0,1875066},{1875066,79,1875066},{1,0,2},{1,100,2},{2,-3,1}})
            observation("wrapper","cache2d|"+Arrays.toString(p),chunk,trace,()->compute(wrapped,point(p[0],p[1],p[2])));
        observation("wrapper","cache2d-fill-bypass",chunk,trace,()->{double[] a=new double[3];fill(wrapped,a,provider(trace));return a;});
        Object fail2d=wrap(chunk,marker("cache2d",new Probe("poison2d",trace,chunk,-20,20,17).function));
        Probe failProbe=new Probe("poison2d-child",trace,chunk,-20,20,17);failProbe.failAt=0;fail2d=wrap(chunk,marker("cache2d",failProbe.function));
        final Object poisoned2d=fail2d;
        observation("wrapper","cache2d-child-error",chunk,trace,()->compute(poisoned2d,point(73,9,-29)));
        observation("wrapper","cache2d-poisoned-key-hit",chunk,trace,()->compute(poisoned2d,point(73,79,-29)));
        chunk=isolated(settings,0);final Object onceChunk=chunk;
        Probe onceProbe=new Probe("once-child",trace,chunk,-100,100,11,22,33,44);
        Object once=wrap(chunk,marker("cacheOnce",onceProbe.function));
        observation("wrapper","once-initial-counter-zero",chunk,trace,()->compute(once,onceChunk));
        set(chunk,"NoiseChunk","interpolationCounter",1L);
        observation("wrapper","once-scalar-miss",chunk,trace,()->compute(once,onceChunk));
        observation("wrapper","once-scalar-hit",chunk,trace,()->compute(once,onceChunk));
        observation("wrapper","once-array-miss",chunk,trace,()->{double[] a=new double[4];fill(once,a,provider(trace));return a;});
        set(chunk,"NoiseChunk","arrayIndex",2);
        observation("wrapper","once-array-before-scalar",chunk,trace,()->compute(once,onceChunk));
        observation("wrapper","once-array-prefix",chunk,trace,()->{double[] a=new double[2];fill(once,a,provider(trace));return a;});
        observation("wrapper","once-array-longer-same-epoch",chunk,trace,()->{double[] a=new double[5];fill(once,a,provider(trace));return a;});
        set(chunk,"NoiseChunk","arrayInterpolationCounter",1L);
        observation("wrapper","once-changed-logical-length",chunk,trace,()->{double[] a=new double[2];fill(once,a,provider(trace));return a;});
        set(chunk,"NoiseChunk","arrayIndex",1);
        observation("wrapper","once-new-array-last-index",chunk,trace,()->compute(once,onceChunk));
        observation("wrapper","once-foreign-context",chunk,trace,()->compute(once,point(3,7,11)));
        set(chunk,"NoiseChunk","arrayInterpolationCounter",2L);onceProbe.failFill=true;
        observation("wrapper","once-array-failure",chunk,trace,()->{double[] a={-99,-98,-97};try{fill(once,a,provider(trace));}finally{JsonObject r=row("failure-prefix","array");r.add("bits",doubles(a));r.addProperty("last_epoch",(Long)get(once,"NoiseChunk$CacheOnce","lastArrayCounter"));r.add("last_array",doubles((double[])get(once,"NoiseChunk$CacheOnce","lastArray")));trace.add(r);}return a;});
        set(chunk,"NoiseChunk","arrayInterpolationCounter",Long.MAX_VALUE);
        onceProbe.failFill=false;
        observation("wrapper","once-near-wrap-array",chunk,trace,()->{double[] a=new double[3];fill(once,a,provider(trace));return a;});
        set(chunk,"NoiseChunk","arrayInterpolationCounter",Long.MIN_VALUE);
        observation("wrapper","once-wrapped-array-counter",chunk,trace,()->{double[] a=new double[1];fill(once,a,provider(trace));return a;});
        Object nested=wrap(chunk,marker("cacheOnce",wrap(chunk,marker("cache2d",onceProbe.function))));
        set(chunk,"NoiseChunk","interpolationCounter",Long.MAX_VALUE);
        observation("wrapper","once2d-nested-scalar-max",chunk,trace,()->compute(nested,onceChunk));
        set(chunk,"NoiseChunk","interpolationCounter",Long.MIN_VALUE);
        observation("wrapper","once2d-nested-scalar-min",chunk,trace,()->compute(nested,onceChunk));
        chunk=isolated(settings,0);final Object flatChunk=chunk;
        Probe flatProbe=new Probe("flat-child",trace,chunk,-100,100,7,11,13,17,19);
        Object flat=wrap(chunk,marker("flatCache",flatProbe.function));
        observation("wrapper","flat-constructor-inclusive-grid",chunk,trace,()->doubles2((double[][])get(flat,"NoiseChunk$FlatCache","values")));
        for(int[] p:new int[][]{{0,79,0},{16,-63,16},{19,12,19},{20,12,0},{-1,12,0},{0,-50,-1}})
            observation("wrapper","flat|"+Arrays.toString(p),chunk,trace,()->compute(flat,point(p[0],p[1],p[2])));
        observation("wrapper","flat-fill-provider-direct",chunk,trace,()->{double[] a=new double[3];fill(flat,a,provider(trace));return a;});
        Object flatOnce=wrap(chunk,marker("cacheOnce",flat));
        observation("wrapper","flat-once-foreign",chunk,trace,()->compute(flatOnce,point(20,13,-3)));
        chunk=isolated(settings,0);final Object cellChunk=chunk;
        Probe cellProbe=new Probe("cell-child",trace,chunk,-5000,5000,37);
        Object cell=wrap(chunk,marker("cacheAllInCell",cellProbe.function));
        double[] cellValues=(double[])get(cell,"NoiseChunk$CacheAllInCell","values");
        for(int i=0;i<cellValues.length;i++)cellValues[i]=1000+i;
        observation("wrapper","cell-before-interpolation",chunk,trace,()->compute(cell,cellChunk));
        observation("wrapper","cell-foreign",chunk,trace,()->compute(cell,point(1,2,3)));
        set(chunk,"NoiseChunk","interpolating",true);
        for(int[] p:new int[][]{{0,0,0},{3,7,3},{1,3,2},{-1,0,0},{4,0,0},{0,8,0}}){
            set(chunk,"NoiseChunk","inCellX",p[0]);set(chunk,"NoiseChunk","inCellY",p[1]);set(chunk,"NoiseChunk","inCellZ",p[2]);
            observation("wrapper","cell|"+Arrays.toString(p),chunk,trace,()->compute(cell,cellChunk));
        }
        observation("wrapper","cell-fill-foreign-provider",chunk,trace,()->{double[] a=new double[3];fill(cell,a,provider(trace));return a;});
        chunk=isolated(settings,0);final Object interpChunk=chunk;
        Probe interpProbe=new Probe("interpolator-child",trace,chunk,-1e30,1e30,23);
        Object interp=wrap(chunk,marker("interpolated",interpProbe.function));
        observation("wrapper","interpolator-own-before-init",chunk,trace,()->compute(interp,interpChunk));
        observation("wrapper","interpolator-foreign-fallback",chunk,trace,()->compute(interp,point(2,3,1)));
        String[] corners={"noise000","noise100","noise010","noise110","noise001","noise101","noise011","noise111"};
        double[] declared={1e16,1,-1e16,3.25,Math.nextUp(1.0),-1e-20,17,-37};
        for(int i=0;i<corners.length;i++)set(interp,"NoiseChunk$NoiseInterpolator",corners[i],declared[i]);
        set(chunk,"NoiseChunk","interpolating",true);set(chunk,"NoiseChunk","inCellX",1);set(chunk,"NoiseChunk","inCellY",3);set(chunk,"NoiseChunk","inCellZ",2);
        set(chunk,"NoiseChunk","fillingCell",true);
        observation("wrapper","interpolator-fillingCell-actual-lerp3",chunk,trace,()->compute(interp,interpChunk));
        observation("wrapper","interpolator-fillingCell-direct-fill",chunk,trace,()->{double[] a=new double[3];fill(interp,a,provider(trace));return a;});
        set(chunk,"NoiseChunk","fillingCell",false);
        for(double fraction:new double[]{Math.nextDown(0.0),0,Math.nextUp(0.0),0.25,Math.nextDown(1.0),1,Math.nextUp(1.0)}){
            call(interp,"NoiseChunk$NoiseInterpolator","updateForY",new String[]{"double"},0.375);
            call(interp,"NoiseChunk$NoiseInterpolator","updateForX",new String[]{"double"},fraction);
            call(interp,"NoiseChunk$NoiseInterpolator","updateForZ",new String[]{"double"},0.5);
            observation("wrapper","interpolator-normal-update|"+bits(fraction),chunk,trace,()->compute(interp,interpChunk));
        }
        observation("wrapper","interpolator-normal-fill-child",chunk,trace,()->{double[] a=new double[3];fill(interp,a,provider(trace));return a;});
        providers(settings);
    }
    static void providers(Object settings) throws Exception {
        JsonArray trace=new JsonArray();Object chunk=isolated(settings,0);
        Probe child=new Probe("provider-child",trace,chunk,-100,100,13);
        Object slice=get(chunk,"NoiseChunk","sliceFillingContextProvider");
        for(int i:new int[]{0,1,(Integer)get(chunk,"NoiseChunk","cellCountY")}){
            final int index=i;
            observation("provider","slice-forIndex|"+i,chunk,trace,()->contextRecord(call(slice,"DensityFunction$ContextProvider","forIndex",new String[]{"int"},index),chunk));
        }
        observation("provider","slice-direct-fill",chunk,trace,()->{
            double[] a=new double[(Integer)get(chunk,"NoiseChunk","cellCountY")+1];
            call(slice,"DensityFunction$ContextProvider","fillAllDirectly",new String[]{"double[]","DensityFunction"},a,child.function);return a;
        });
        for(int i:new int[]{0,1,15,16,127}){
            final int index=i;
            observation("provider","cell-forIndex|"+i,chunk,trace,()->contextRecord(call(chunk,"NoiseChunk","forIndex",new String[]{"int"},index),chunk));
        }
        observation("provider","cell-direct-fill",chunk,trace,()->{double[] a=new double[128];call(chunk,"NoiseChunk","fillAllDirectly",new String[]{"double[]","DensityFunction"},a,child.function);return a;});
        Probe left=new Probe("first-cell-registration",trace,chunk,-100,100,19),right=new Probe("second-cell-registration",trace,chunk,-100,100,23);
        wrap(chunk,marker("cacheAllInCell",left.function));wrap(chunk,marker("cacheAllInCell",right.function));
        call(chunk,"NoiseChunk","initializeForFirstCellX");call(chunk,"NoiseChunk","advanceCellX",new String[]{"int"},0);
        observation("provider","actual-cellcache-registration-fill",chunk,trace,()->{call(chunk,"NoiseChunk","selectCellYZ",new String[]{"int","int"},0,0);return null;});
        set(chunk,"NoiseChunk","interpolationCounter",Long.MAX_VALUE);
        observation("provider","slice-scalar-counter-wrap",chunk,trace,()->contextRecord(call(slice,"DensityFunction$ContextProvider","forIndex",new String[]{"int"},0),chunk));
    }
    static void arrays(Object settings) throws Exception {
        double[] inputs={0,-0.0,-3,Math.nextDown(2.0),2,Math.nextUp(2.0),5,Double.NaN};
        for(String operation:List.of("add","mul","min","max"))for(String failure:List.of("none","left","right")){
            JsonArray trace=new JsonArray();Probe left=new Probe("left",trace,null,-4,6,inputs),right=new Probe("right",trace,null,2,2,11);
            if(failure.equals("left"))left.failFill=true;if(failure.equals("right"))right.failAt=1;
            Object function=call(null,"DensityFunctions",operation,new String[]{"DensityFunction","DensityFunction"},left.function,right.function);
            observation("array",operation+"|"+failure,null,trace,()->{
                double[] a=new double[inputs.length];Arrays.fill(a,-99);
                try{fill(function,a,provider(trace));}finally{JsonObject r=row("array-prefix","actual");r.add("bits",doubles(a));trace.add(r);}return a;
            });
        }
        for(double zero:new double[]{0,-0.0})for(boolean fail:new boolean[]{false,true}){
            JsonArray trace=new JsonArray();Probe input=new Probe("input",trace,null,-5,5,inputs);input.failFill=fail;
            Object function=call(null,"DensityFunctions","mul",new String[]{"DensityFunction","DensityFunction"},constant(zero),input.function);
            observation("array","mulOrAdd-zero|"+bits(zero)+"|"+fail,null,trace,()->{double[] a=new double[inputs.length];try{fill(function,a,provider(trace));}finally{JsonObject r=row("array-prefix","actual");r.add("bits",doubles(a));trace.add(r);}return a;});
        }
        for(String failure:List.of("none","inside","outside","input")){
            JsonArray trace=new JsonArray();Probe input=new Probe("input",trace,null,-3,5,inputs),inside=new Probe("inside",trace,null,17,17,17),outside=new Probe("outside",trace,null,-19,-19,-19);
            if(failure.equals("input"))input.failFill=true;if(failure.equals("inside"))inside.failAt=0;if(failure.equals("outside"))outside.failAt=0;
            Object function=call(null,"DensityFunctions","rangeChoice",new String[]{"DensityFunction","double","double","DensityFunction","DensityFunction"},input.function,0.0,2.0,inside.function,outside.function);
            observation("array","rangeChoice|"+failure,null,trace,()->{double[] a=new double[inputs.length];try{fill(function,a,provider(trace));}finally{JsonObject r=row("array-prefix","actual");r.add("bits",doubles(a));trace.add(r);}return a;});
        }
        for(String operation:List.of("blendDensity","abs","clamp")){
            JsonArray trace=new JsonArray();Probe input=new Probe("input",trace,null,-5,5,inputs);
            Object function=operation.equals("blendDensity")?call(null,"DensityFunctions","blendDensity",new String[]{"DensityFunction"},input.function):
                operation.equals("abs")?call(input.function,"DensityFunction","abs"):call(input.function,"DensityFunction","clamp",new String[]{"double","double"},-1.0,1.0);
            observation("array",operation+"|context-order",null,trace,()->{double[] a=new double[inputs.length];fill(function,a,provider(trace));return a;});
        }
        JsonArray trace=new JsonArray();Object chunk=isolated(settings,0);
        Probe probe=new Probe("shared-child",trace,chunk,-10,10,2);
        Object once=wrap(chunk,marker("cacheOnce",probe.function));
        Object add=call(null,"DensityFunctions","add",new String[]{"DensityFunction","DensityFunction"},once,once);
        observation("array","shared-once-add-array",chunk,trace,()->{double[] a=new double[4];fill(add,a,provider(trace));return a;});
        for(String operation:List.of("constant","blendAlpha","blendOffset","beard","gradient","weirdScaledSampler")){
            JsonArray requests=new JsonArray();Object function;
            if(operation.equals("constant"))function=constant(-0.0);
            else if(operation.equals("beard"))function=beard;
            else if(operation.equals("gradient"))function=call(null,"DensityFunctions","yClampedGradient",new String[]{"int","int","double","double"},100,107,-1.0,1.0);
            else if(operation.equals("weirdScaledSampler")){
                Probe input=new Probe("weird-input",requests,null,-3,3,-1,0,1,2);
                Object holder=call(noiseGetter,"HolderGetter","getOrThrow",new String[]{"ResourceKey"},get(null,"Noises","TEMPERATURE"));
                function=call(null,"DensityFunctions","weirdScaledSampler",new String[]{"DensityFunction","Holder","DensityFunctions$WeirdScaledSampler$RarityValueMapper"},input.function,holder,enumValue("DensityFunctions$WeirdScaledSampler$RarityValueMapper","TYPE1"));
            }else function=call(null,"DensityFunctions",operation);
            final Object actual=function;
            observation("array",operation+"|actual-fill-contract",null,requests,()->{double[] a=new double[8];fill(actual,a,provider(requests));return a;});
        }
        JsonArray nestedTrace=new JsonArray();Object nestedSession=isolated(settings,0);
        Probe nestedProbe=new Probe("nested-once-child",nestedTrace,nestedSession,-20,20,7,11,13,17);
        Object nestedOnce=wrap(nestedSession,marker("cacheOnce",nestedProbe.function));
        call(nestedSession,"NoiseChunk","initializeForFirstCellX");call(nestedSession,"NoiseChunk","advanceCellX",new String[]{"int"},0);
        set(nestedSession,"NoiseChunk","arrayInterpolationCounter",41L);
        observation("array","cacheonce-own-array-before-direct-provider",nestedSession,nestedTrace,()->{double[] a=new double[128];fill(nestedOnce,a,nestedSession);return a;});
        observation("array","cacheonce-own-cell-direct-arrayIndex-contract",nestedSession,nestedTrace,()->{double[] a=new double[128];try{call(nestedSession,"NoiseChunk","fillAllDirectly",new String[]{"double[]","DensityFunction"},a,nestedOnce);}finally{JsonObject r=row("array-prefix","actual");r.add("bits",doubles(a));nestedTrace.add(r);}return a;});
    }
    static void equality(String label,Object first,Object second) {
        JsonObject result=row("equals",label);result.addProperty("first_class",semantic(first.getClass()));result.addProperty("second_class",semantic(second.getClass()));
        result.addProperty("identity",first==second);result.addProperty("equals_forward",first.equals(second));result.addProperty("equals_reverse",second.equals(first));
        result.addProperty("hash_equal",first.hashCode()==second.hashCode());write(result);
    }
    static void equalControls(Object settings) throws Exception {
        for(double[] pair:new double[][]{{0,-0.0},{Double.longBitsToDouble(0x7ff8000000001234L),Double.longBitsToDouble(0x7ff8000000009876L)},{1,1}}){
            Object first=constant(pair[0]),second=constant(pair[1]);equality("constant-f64|"+bits(pair[0])+"|"+bits(pair[1]),first,second);
            Object d1=call(null,"Holder","direct",new String[]{"java.lang.Object"},first),d2=call(null,"Holder","direct",new String[]{"java.lang.Object"},second);
            equality("direct-holder-f64|"+bits(pair[0])+"|"+bits(pair[1]),d1,d2);
            equality("marker-equal-children|"+bits(pair[0])+"|"+bits(pair[1]),marker("cache2d",first),marker("cache2d",second));
        }
        for(float[] pair:new float[][]{{0,-0.0f},{Float.intBitsToFloat(0x7fc01234),Float.intBitsToFloat(0x7fc09876)},{1,1}}){
            Object first=call(null,"CubicSpline","constant",new String[]{"float"},pair[0]),second=call(null,"CubicSpline","constant",new String[]{"float"},pair[1]);
            equality("spline-constant-f32|"+bits(pair[0])+"|"+bits(pair[1]),first,second);
        }
        Object noiseKey=get(null,"Noises","TEMPERATURE");
        Object holder=call(noiseGetter,"HolderGetter","getOrThrow",new String[]{"ResourceKey"},noiseKey);
        equality("native-reference-repeated",holder,call(noiseGetter,"HolderGetter","getOrThrow",new String[]{"ResourceKey"},noiseKey));
        Object normalHolder=create("DensityFunction$NoiseHolder",new String[]{"Holder"},holder);
        Object random=random(settings,0);
        for(boolean legacy:new boolean[]{false,true}){
            Object helper=create("RandomState$1NoiseWiringHelper",new String[]{"RandomState","long","boolean"},random,0L,legacy);
            Object a=call(helper,"DensityFunction$Visitor","visitNoise",new String[]{"DensityFunction$NoiseHolder"},normalHolder);
            Object b=call(helper,"DensityFunction$Visitor","visitNoise",new String[]{"DensityFunction$NoiseHolder"},normalHolder);
            equality("noiseholder-modern-legacy|"+legacy,a,b);
            equality("normalnoise-modern-legacy|"+legacy,get(a,"DensityFunction$NoiseHolder","noise"),get(b,"DensityFunction$NoiseHolder","noise"));
        }
        Object coordinate=Proxy.newProxyInstance(NoiseChunkIndependentOracle.class.getClassLoader(),new Class<?>[]{type("ToFloatFunction")},(p,m,a)->{
            if(m.equals(method("ToFloatFunction","minValue")))return -1f;if(m.equals(method("ToFloatFunction","maxValue")))return 1f;
            if(m.equals(method("ToFloatFunction","apply","java.lang.Object")))return 0f;
            if(m.getName().equals("equals"))return p==a[0];if(m.getName().equals("hashCode"))return System.identityHashCode(p);
            throw new UnsupportedOperationException("unexpected coordinate probe "+m);
        });
        float[] locations={-1,1},derivatives={0,0};List<Object> values=List.of(call(null,"CubicSpline","constant",new String[]{"float"},0f),call(null,"CubicSpline","constant",new String[]{"float"},1f));
        String[] params={"ToFloatFunction","float[]","java.util.List","float[]","float","float"};
        Object spline1=create("CubicSpline$Multipoint",params,coordinate,locations,values,derivatives,0f,1f);
        Object spline2=create("CubicSpline$Multipoint",params,coordinate,locations,values,derivatives,0f,1f);
        Object spline3=create("CubicSpline$Multipoint",params,coordinate,locations.clone(),values,derivatives.clone(),0f,1f);
        equality("spline-same-array-objects",spline1,spline2);equality("spline-same-bytes-new-arrays",spline1,spline3);
        equality("spline-wrapper-same-arrays",call(null,"DensityFunctions","spline",new String[]{"CubicSpline"},spline1),call(null,"DensityFunctions","spline",new String[]{"CubicSpline"},spline2));
        equality("end-fresh-identity",call(null,"DensityFunctions","endIslands",new String[]{"long"},0L),call(null,"DensityFunctions","endIslands",new String[]{"long"},0L));
        equality("blended-fresh-identity",call(null,"BlendedNoise","createUnseeded",new String[]{"double","double","double","double","double"},0.25,0.125,80.0,160.0,8.0),call(null,"BlendedNoise","createUnseeded",new String[]{"double","double","double","double","double"},0.25,0.125,80.0,160.0,8.0));
        Object chunk=isolated(settings,0),a=marker("cacheOnce",constant(7)),b=marker("cacheOnce",constant(7));
        equality("wrap-equal-markers-share-result",wrap(chunk,a),wrap(chunk,b));
        JsonArray trace=new JsonArray();Probe fail=new Probe("once-poison",trace,chunk,-20,20,17);fail.failAt=0;Object once=wrap(chunk,marker("cacheOnce",fail.function));
        set(chunk,"NoiseChunk","interpolationCounter",1L);
        observation("wrapper","once-scalar-failure",chunk,trace,()->compute(once,chunk));
        observation("wrapper","once-poisoned-scalar-hit",chunk,trace,()->compute(once,chunk));
    }
    static List<int[]> columns() {
        List<int[]> result=new ArrayList<>();for(int x=0;x<=16;x+=4)for(int z=0;z<=16;z+=4)result.add(new int[]{x,z});
        for(int[] p:new int[][]{{-1,0},{0,-1},{-4,-4},{-5,3},{-16,0},{0,-16},{-17,-17},{-19,19},{20,0},{0,20},{20,20},{31,31},{32,-1},{-1,32},{-32,-32},{-33,17},{-5,20},{20,-5},{19,20},{20,19},{-8,8},{8,-8},{-24,24},{24,-24},{-48,0},{0,-48},{48,0},{0,48},{30000000,-30000000},{-30000000,30000000},{-65,-67},{65,67}})result.add(p);
        return result;
    }
    static void preliminary(String name,Object settings,long seed) throws Exception {
        Object random=random(settings,seed),chunk=chunk(settings,random,0,0);JsonObject result=row("preliminary",name+"|"+seed);JsonArray values=new JsonArray();
        for(int[] p:columns()){
            int a=(Integer)call(chunk,"NoiseChunk","preliminarySurfaceLevel",new String[]{"int","int"},p[0],p[1]);
            int b=(Integer)call(chunk,"NoiseChunk","preliminarySurfaceLevel",new String[]{"int","int"},p[0],p[1]);
            if(a!=b)throw new IllegalStateException("actual preliminary hit differs");
            JsonObject r=row("column",Arrays.toString(p));r.addProperty("x",p[0]);r.addProperty("z",p[1]);r.addProperty("surface",a);r.addProperty("cache_size",((Map<?,?>)get(chunk,"NoiseChunk","preliminarySurfaceLevel")).size());values.add(r);
        }
        Object fresh=chunk(settings,random,0,0);List<int[]> reversed=columns();Collections.reverse(reversed);
        for(int[] p:reversed){
            int a=(Integer)call(fresh,"NoiseChunk","preliminarySurfaceLevel",new String[]{"int","int"},p[0],p[1]);
            int b=(Integer)call(chunk,"NoiseChunk","preliminarySurfaceLevel",new String[]{"int","int"},p[0],p[1]);if(a!=b)throw new IllegalStateException("preliminary request order differs");
        }
        result.add("columns",values);result.addProperty("fresh_reordered_checked",true);write(result);
        if(seed!=0)return;
        for(double threshold:new double[]{Math.nextDown(0.390625),0.390625,Math.nextUp(0.390625),Double.NaN}){
            Object synthetic=isolated(settings,seed);JsonArray trace=new JsonArray();Probe probe=new Probe("initial-threshold",trace,synthetic,-1,1,threshold);
            set(synthetic,"NoiseChunk","initialDensityNoJaggedness",probe.function);
            observation("preliminary-trace",name+"|threshold|"+bits(threshold),synthetic,trace,()->call(synthetic,"NoiseChunk","preliminarySurfaceLevel",new String[]{"int","int"},-1,19));
            observation("preliminary-trace",name+"|threshold-hit|"+bits(threshold),synthetic,trace,()->call(synthetic,"NoiseChunk","preliminarySurfaceLevel",new String[]{"int","int"},-4,16));
        }
        Object synthetic=isolated(settings,seed);JsonArray trace=new JsonArray();Probe fail=new Probe("initial-failure",trace,synthetic,-1,1,0);fail.failAt=1;
        set(synthetic,"NoiseChunk","initialDensityNoJaggedness",fail.function);
        observation("preliminary-trace",name+"|child-error",synthetic,trace,()->call(synthetic,"NoiseChunk","preliminarySurfaceLevel",new String[]{"int","int"},-1,19));
        observation("preliminary-trace",name+"|error-entry-not-published",synthetic,trace,()->((Map<?,?>)get(synthetic,"NoiseChunk","preliminarySurfaceLevel")).size());
        fail.failAt=-1;
        observation("preliminary-trace",name+"|retry-after-error",synthetic,trace,()->call(synthetic,"NoiseChunk","preliminarySurfaceLevel",new String[]{"int","int"},-1,19));
        Object minSession=isolated(settings,seed);JsonArray minTrace=new JsonArray();Probe minProbe=new Probe("initial-min-only",minTrace,minSession,-1,1,0);
        int minY=(Integer)call(call(settings,"NoiseGeneratorSettings","noiseSettings"),"NoiseSettings","minY");minProbe.byY=Map.of(minY,Math.nextUp(0.390625));
        set(minSession,"NoiseChunk","initialDensityNoJaggedness",minProbe.function);
        observation("preliminary-trace",name+"|threshold-at-minY",minSession,minTrace,()->call(minSession,"NoiseChunk","preliminarySurfaceLevel",new String[]{"int","int"},-1,19));
    }
    static void climate(String name,Object settings,long seed) throws Exception {
        Object random=random(settings,seed),chunk=chunk(settings,random,0,0),router=call(random,"RandomState","router");
        Object sampler=call(chunk,"NoiseChunk","cachedClimateSampler",new String[]{"NoiseRouter","java.util.List"},router,call(settings,"NoiseGeneratorSettings","spawnTarget"));
        int min=(Integer)call(call(settings,"NoiseGeneratorSettings","noiseSettings"),"NoiseSettings","minY");
        int top=min+(Integer)call(call(settings,"NoiseGeneratorSettings","noiseSettings"),"NoiseSettings","height");
        List<int[]> positions=new ArrayList<>(List.of(new int[]{0,0,0},new int[]{-1,-1,-1},new int[]{-4,16,-5},new int[]{1,-16,4},new int[]{7500000,16,-7500000},new int[]{-7500000,-16,7500000}));
        for(int y:new int[]{min/4-1,min/4,min/4+1,top/4-1,top/4,top/4+1,-536870912,536870911})for(int[] xz:new int[][]{{4,4},{5,-1}})positions.add(new int[]{xz[0],y,xz[1]});
        JsonObject result=row("climate",name+"|"+seed);JsonArray samples=new JsonArray();
        for(int[] p:positions){
            Object target=call(sampler,"Climate$Sampler","sample",new String[]{"int","int","int"},p[0],p[1],p[2]);
            JsonObject r=row("quart",Arrays.toString(p));r.add("position",GSON.toJsonTree(p));JsonArray quantized=new JsonArray(),scalar=new JsonArray();
            String[] targetFields={"temperature","humidity","continentalness","erosion","depth","weirdness"};
            for(String field:targetFields)quantized.add((Long)call(target,"Climate$TargetPoint",field));
            for(String field:CLIMATE)scalar.add(bits((float)compute(call(sampler,"Climate$Sampler",field),point(p[0]*4,p[1]*4,p[2]*4))));
            r.add("quantized",quantized);r.add("f32_bits",scalar);samples.add(r);
        }
        result.add("samples",samples);result.add("with_cached_climate",census(chunk));write(result);
    }
    static JsonArray snapshots(Object chunk,String part) throws Exception {
        JsonArray result=new JsonArray();
        if(part.equals("cellCaches")){
            for(Object wrapper:(List<?>)get(chunk,"NoiseChunk",part))result.add(doubles((double[])get(wrapper,"NoiseChunk$CacheAllInCell","values")));
        }else for(Object wrapper:(List<?>)get(chunk,"NoiseChunk","interpolators"))result.add(doubles2((double[][])get(wrapper,"NoiseChunk$NoiseInterpolator",part)));
        return result;
    }
    static void diagnostics(String name,Object settings,long seed) throws Exception {
        Object random=random(settings,seed),router=call(random,"RandomState","router"),geometry=call(settings,"NoiseGeneratorSettings","noiseSettings");
        int min=(Integer)call(geometry,"NoiseSettings","minY"),height=(Integer)call(geometry,"NoiseSettings","height"),w=(Integer)call(geometry,"NoiseSettings","getCellWidth"),h=(Integer)call(geometry,"NoiseSettings","getCellHeight");
        JsonObject result=row("diagnostic",name+"|"+seed);
        Object initialSession=chunk(settings,random,0,0);Object initial=get(initialSession,"NoiseChunk","initialDensityNoJaggedness");
        JsonArray initialSamples=new JsonArray(),finalSamples=new JsonArray();
        for(int[] p:new int[][]{{0,min,0},{0,min+h,0},{0,-56,0},{-4,min,16},{20,min+height,20},{-17,73,31}}){
            JsonObject sample=row("point",Arrays.toString(p));sample.addProperty("bits",bits(compute(initial,point(p[0],p[1],p[2]))));initialSamples.add(sample);
        }
        Object finalSession=chunk(settings,random,0,0);Object finalRoot=call(router,"NoiseRouter","finalDensity");
        for(int[] p:new int[][]{{0,min,0},{0,min+h,0},{0,-56,0},{-4,min,16},{20,min+height,20},{-17,73,31}}){
            JsonObject sample=row("point",Arrays.toString(p));sample.addProperty("bits",bits(compute(finalRoot,point(p[0],p[1],p[2]))));finalSamples.add(sample);
        }
        result.add("initial_foreign",initialSamples);result.add("bare_final_foreign",finalSamples);
        Object session=chunk(settings,random,0,0);
        call(session,"NoiseChunk","initializeForFirstCellX");result.add("initial_slice0",snapshots(session,"slice0"));
        call(session,"NoiseChunk","advanceCellX",new String[]{"int"},0);result.add("advanced_slice1",snapshots(session,"slice1"));
        call(session,"NoiseChunk","selectCellYZ",new String[]{"int","int"},0,0);result.add("first_cell_arrays",snapshots(session,"cellCaches"));
        JsonArray corners=new JsonArray();
        for(Object wrapper:(List<?>)get(session,"NoiseChunk","interpolators")){
            JsonArray values=new JsonArray();for(String field:List.of("noise000","noise100","noise010","noise110","noise001","noise101","noise011","noise111"))values.add(bits((Double)get(wrapper,"NoiseChunk$NoiseInterpolator",field)));corners.add(values);
        }
        result.add("first_cell_corners",corners);JsonArray samples=new JsonArray();Object material=material(session);
        for(int[] p:new int[][]{{0,0,0},{w-1,h-1,w-1},{1,1,1},{w-1,0,0}}){
            call(session,"NoiseChunk","updateForY",new String[]{"int","double"},min+p[1],(double)p[1]/h);
            call(session,"NoiseChunk","updateForX",new String[]{"int","double"},p[0],(double)p[0]/w);
            call(session,"NoiseChunk","updateForZ",new String[]{"int","double"},p[2],(double)p[2]/w);
            JsonObject sample=row("driver-point",Arrays.toString(p));sample.addProperty("bits",bits(compute(material,session)));sample.add("context",contextRecord(session,session));samples.add(sample);
        }
        result.add("driver_samples",samples);call(session,"NoiseChunk","swapSlices");result.add("after_swap_slice0",snapshots(session,"slice0"));result.add("after_swap_slice1",snapshots(session,"slice1"));
        result.addProperty("scalar_counter",(Long)get(session,"NoiseChunk","interpolationCounter"));result.addProperty("array_counter",(Long)get(session,"NoiseChunk","arrayInterpolationCounter"));
        call(session,"NoiseChunk","stopInterpolation");write(result);
        if(seed==0)noiseOffsets(name,random);
    }
    static void noiseOffsets(String name,Object random) throws Exception {
        Map<?,?> noises=(Map<?,?>)get(random,"RandomState","noiseIntances");TreeMap<String,Object> sorted=new TreeMap<>();
        for(var entry:noises.entrySet())sorted.put(call(entry.getKey(),"ResourceKey","location").toString(),entry.getValue());
        JsonObject result=row("bound-noise-offsets",name);JsonArray records=new JsonArray();
        for(var entry:sorted.entrySet()){
            JsonObject normal=row("normal",entry.getKey());
            for(String field:List.of("first","second")){
                Object perlin=get(entry.getValue(),"NormalNoise",field);JsonArray octaves=new JsonArray();
                for(Object improved:(Object[])get(perlin,"PerlinNoise","noiseLevels")){
                    if(improved==null){octaves.add(JsonNull.INSTANCE);continue;}
                    JsonObject octave=new JsonObject();JsonArray offsets=new JsonArray();for(String axis:List.of("xo","yo","zo"))offsets.add(bits((Double)get(improved,"ImprovedNoise",axis)));octave.add("offset_bits",offsets);
                    byte[] perm=(byte[])get(improved,"ImprovedNoise","p");octave.addProperty("permutation_sha256",HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(perm)));octaves.add(octave);
                }
                normal.add(field,octaves);
            }
            records.add(normal);
        }
        result.add("noises",records);write(result);
    }
}
