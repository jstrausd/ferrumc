package ferrumc.independent;

import com.google.gson.*;
import com.mojang.serialization.*;
import java.io.*;
import java.lang.reflect.*;
import java.nio.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;

/** Actual End wrapper fallback/corners and full Vanilla-vs-frozen-Paper driver comparison. */
public final class EndAndChunkSeams {
    static JsonObject definitions;
    static Map<String,Object> settings=new TreeMap<>();
    static final class Session {
        final Object chunk,wrapper;
        Session(Object endSettings,long seed) throws Exception {
            chunk=NoiseChunkIndependentOracle.isolated(endSettings,seed);
            wrapper=NoiseChunkIndependentOracle.wrap(chunk,NoiseChunkIndependentOracle.marker("interpolated",
                NoiseChunkIndependentOracle.call(null,"DensityFunctions","endIslands",new String[]{"long"},seed)));
        }
    }
    @SuppressWarnings("unchecked")
    static void bootstrap(String[] args) throws Exception {
        NoiseChunkIndependentOracle.paper=args[0].equals("paper");
        NoiseChunkIndependentOracle.inner=Path.of(args[4]);
        NoiseChunkIndependentOracle.mappings=JsonParser.parseString(Files.readString(Path.of(args[2]))).getAsJsonObject();
        NoiseChunkIndependentOracle.origins=new JsonObject();
        for(var entry:NoiseChunkIndependentOracle.mappings.entrySet())
            NoiseChunkIndependentOracle.reverse.put(NoiseChunkIndependentOracle.paper?entry.getKey():entry.getValue().getAsJsonObject().get("class").getAsString(),entry.getKey());
        NoiseChunkIndependentOracle.requireOrigin(NoiseChunkIndependentOracle.class,Path.of(args[5]));
        NoiseChunkIndependentOracle.requireOrigin(EndAndChunkSeams.class,Path.of(args[5]));
        NoiseChunkIndependentOracle.origins.addProperty(EndAndChunkSeams.class.getName(),"$fresh-helper-classes");
        JsonObject libraryOrigins=JsonParser.parseString(Files.readString(Path.of(args[6]))).getAsJsonObject();
        for(var entry:libraryOrigins.entrySet())NoiseChunkIndependentOracle.requireOrigin(Class.forName(entry.getKey()),Path.of(entry.getValue().getAsString()));
        NoiseChunkIndependentOracle.call(null,"SharedConstants","tryDetectVersion");
        NoiseChunkIndependentOracle.call(null,"Bootstrap","bootStrap");
        NoiseChunkIndependentOracle.lookup=NoiseChunkIndependentOracle.call(null,"VanillaRegistries","createLookup");
        NoiseChunkIndependentOracle.ops=(DynamicOps<JsonElement>)NoiseChunkIndependentOracle.call(null,"RegistryOps","create",
            new String[]{"com.mojang.serialization.DynamicOps","HolderLookup$Provider"},JsonOps.INSTANCE,NoiseChunkIndependentOracle.lookup);
        NoiseChunkIndependentOracle.settingsCodec=(Codec<Object>)NoiseChunkIndependentOracle.get(null,"NoiseGeneratorSettings","CODEC");
        NoiseChunkIndependentOracle.noiseGetter=NoiseChunkIndependentOracle.call(NoiseChunkIndependentOracle.lookup,"HolderGetter$Provider","lookupOrThrow",
            new String[]{"ResourceKey"},NoiseChunkIndependentOracle.get(null,"Registries","NOISE"));
        NoiseChunkIndependentOracle.emptyBlender=NoiseChunkIndependentOracle.call(null,"Blender","empty");
        NoiseChunkIndependentOracle.beard=NoiseChunkIndependentOracle.get(null,"DensityFunctions$BeardifierMarker","INSTANCE");
        definitions=JsonParser.parseString(Files.readString(Path.of(args[1]))).getAsJsonObject().getAsJsonObject("registries").getAsJsonObject("minecraft:worldgen/noise_settings").getAsJsonObject("entries");
        for(String key:definitions.keySet()){
            Object holder=NoiseChunkIndependentOracle.settingsCodec.parse(NoiseChunkIndependentOracle.ops,definitions.get(key)).getOrThrow();
            settings.put(key,NoiseChunkIndependentOracle.call(holder,"Holder","value"));
        }
    }
    static List<JsonObject> records(Path path) throws Exception {
        List<JsonObject> result=new ArrayList<>();for(String line:Files.readAllLines(path))result.add(JsonParser.parseString(line).getAsJsonObject());return result;
    }
    static void endFallback(Path source) throws Exception {
        List<JsonObject> requests=records(source);Map<Long,Session> sessions=new LinkedHashMap<>(),fresh=new LinkedHashMap<>();
        Object end=settings.get("minecraft:end");
        for(JsonObject request:requests){
            long seed=Long.parseLong(request.get("seed").getAsString());
            if(!sessions.containsKey(seed)){sessions.put(seed,new Session(end,seed));fresh.put(seed,new Session(end,seed));}
            Session session=sessions.get(seed);Object context=NoiseChunkIndependentOracle.point(request.get("x").getAsInt(),request.get("y").getAsInt(),request.get("z").getAsInt());
            String value=NoiseChunkIndependentOracle.bits(NoiseChunkIndependentOracle.compute(session.wrapper,context));
            if(!value.equals(request.get("bits").getAsString()))throw new IllegalStateException("actual End foreign wrapper differs from frozen root actual-JAR value");
            JsonObject result=NoiseChunkIndependentOracle.row("end-foreign-fallback",seed+"|"+request.get("x")+"|"+request.get("y")+"|"+request.get("z"));
            result.add("frozen_request",request);result.addProperty("wrapper_bits",value);result.addProperty("interpolating",(Boolean)NoiseChunkIndependentOracle.get(session.chunk,"NoiseChunk","interpolating"));
            NoiseChunkIndependentOracle.write(result);
        }
        Collections.reverse(requests);
        for(JsonObject request:requests){
            Session session=fresh.get(Long.parseLong(request.get("seed").getAsString()));
            String value=NoiseChunkIndependentOracle.bits(NoiseChunkIndependentOracle.compute(session.wrapper,NoiseChunkIndependentOracle.point(request.get("x").getAsInt(),request.get("y").getAsInt(),request.get("z").getAsInt())));
            if(!value.equals(request.get("bits").getAsString()))throw new IllegalStateException("actual End fallback request order/fresh session drift");
        }
    }
    static void endCorners(List<JsonObject> chunks) throws Exception {
        Object end=settings.get("minecraft:end");
        for(JsonObject request:chunks)if(request.get("far_end").getAsBoolean()){
            long seed=Long.parseLong(request.get("seed").getAsString());
            int x=Math.multiplyExact(request.get("chunk_x").getAsInt(),16),z=Math.multiplyExact(request.get("chunk_z").getAsInt(),16);
            Object random=NoiseChunkIndependentOracle.random(end,seed),chunk=NoiseChunkIndependentOracle.chunk(end,random,x,z);
            ((List<?>)NoiseChunkIndependentOracle.get(chunk,"NoiseChunk","interpolators")).clear();
            ((List<?>)NoiseChunkIndependentOracle.get(chunk,"NoiseChunk","cellCaches")).clear();
            ((Map<?,?>)NoiseChunkIndependentOracle.get(chunk,"NoiseChunk","wrapped")).clear();
            Object wrapper=NoiseChunkIndependentOracle.wrap(chunk,NoiseChunkIndependentOracle.marker("interpolated",
                NoiseChunkIndependentOracle.call(null,"DensityFunctions","endIslands",new String[]{"long"},seed)));
            NoiseChunkIndependentOracle.call(chunk,"NoiseChunk","initializeForFirstCellX");
            NoiseChunkIndependentOracle.call(chunk,"NoiseChunk","advanceCellX",new String[]{"int"},0);
            NoiseChunkIndependentOracle.call(chunk,"NoiseChunk","selectCellYZ",new String[]{"int","int"},0,0);
            JsonObject result=NoiseChunkIndependentOracle.row("end-far-corners",seed+"|"+request.get("chunk_x")+"|"+request.get("chunk_z"));
            result.add("slice0",NoiseChunkIndependentOracle.doubles2((double[][])NoiseChunkIndependentOracle.get(wrapper,"NoiseChunk$NoiseInterpolator","slice0")));
            result.add("slice1",NoiseChunkIndependentOracle.doubles2((double[][])NoiseChunkIndependentOracle.get(wrapper,"NoiseChunk$NoiseInterpolator","slice1")));
            JsonArray corners=new JsonArray();for(String field:List.of("noise000","noise100","noise010","noise110","noise001","noise101","noise011","noise111"))corners.add(NoiseChunkIndependentOracle.bits((Double)NoiseChunkIndependentOracle.get(wrapper,"NoiseChunk$NoiseInterpolator",field)));
            result.add("corners",corners);JsonArray samples=new JsonArray();
            int w=(Integer)NoiseChunkIndependentOracle.get(chunk,"NoiseChunk","cellWidth"),h=(Integer)NoiseChunkIndependentOracle.get(chunk,"NoiseChunk","cellHeight");
            for(int[] p:new int[][]{{0,0,0},{w-1,h-1,w-1},{1,1,1}}){
                NoiseChunkIndependentOracle.call(chunk,"NoiseChunk","updateForY",new String[]{"int","double"},p[1],(double)p[1]/h);
                NoiseChunkIndependentOracle.call(chunk,"NoiseChunk","updateForX",new String[]{"int","double"},x+p[0],(double)p[0]/w);
                NoiseChunkIndependentOracle.call(chunk,"NoiseChunk","updateForZ",new String[]{"int","double"},z+p[2],(double)p[2]/w);
                JsonObject r=NoiseChunkIndependentOracle.row("seam-point",Arrays.toString(p));r.addProperty("bits",NoiseChunkIndependentOracle.bits(NoiseChunkIndependentOracle.compute(wrapper,chunk)));samples.add(r);
            }
            result.add("samples",samples);
            result.addProperty("scalar_counter",(Long)NoiseChunkIndependentOracle.get(chunk,"NoiseChunk","interpolationCounter"));
            result.addProperty("array_counter",(Long)NoiseChunkIndependentOracle.get(chunk,"NoiseChunk","arrayInterpolationCounter"));
            NoiseChunkIndependentOracle.call(chunk,"NoiseChunk","stopInterpolation");NoiseChunkIndependentOracle.write(result);
        }
    }
    static void compareFullVanilla(List<JsonObject> chunks,Path paperBits) throws Exception {
        Method initialize=NoiseChunkIndependentOracle.method("NoiseChunk","initializeForFirstCellX"),
            advance=NoiseChunkIndependentOracle.method("NoiseChunk","advanceCellX","int"),
            select=NoiseChunkIndependentOracle.method("NoiseChunk","selectCellYZ","int","int"),
            updateY=NoiseChunkIndependentOracle.method("NoiseChunk","updateForY","int","double"),
            updateX=NoiseChunkIndependentOracle.method("NoiseChunk","updateForX","int","double"),
            updateZ=NoiseChunkIndependentOracle.method("NoiseChunk","updateForZ","int","double"),
            swap=NoiseChunkIndependentOracle.method("NoiseChunk","swapSlices"),
            stop=NoiseChunkIndependentOracle.method("NoiseChunk","stopInterpolation"),
            compute=NoiseChunkIndependentOracle.method("DensityFunction","compute","DensityFunction$FunctionContext");
        Map<String,Object> randomStates=new HashMap<>();
        try(RandomAccessFile data=new RandomAccessFile(paperBits.toFile(),"r")){
            for(JsonObject request:chunks)if(!request.get("far_end").getAsBoolean()){
                String setting=request.get("settings").getAsString();long seed=Long.parseLong(request.get("seed").getAsString());
                String key=setting+"|"+seed;Object definition=settings.get(setting);
                if(!randomStates.containsKey(key))randomStates.put(key,NoiseChunkIndependentOracle.random(definition,seed));
                int x=Math.multiplyExact(request.get("chunk_x").getAsInt(),16),z=Math.multiplyExact(request.get("chunk_z").getAsInt(),16);
                int min=request.get("min_y").getAsInt(),height=request.get("height").getAsInt(),w=request.get("cell_width").getAsInt(),h=request.get("cell_height").getAsInt();
                Object chunk=NoiseChunkIndependentOracle.chunk(definition,randomStates.get(key),x,z),material=NoiseChunkIndependentOracle.material(chunk);
                int bytes=Math.multiplyExact(Math.multiplyExact(height,256),8);
                if(bytes!=request.get("bytes").getAsInt())throw new IllegalStateException("root contiguous chunk geometry differs");
                byte[] expected=new byte[bytes];data.seek(request.get("offset").getAsLong());data.readFully(expected);
                if(!HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(expected)).equals(request.get("sha256").getAsString()))throw new IllegalStateException("frozen root chunk hash differs");
                ByteBuffer vanilla=ByteBuffer.allocate(bytes).order(ByteOrder.BIG_ENDIAN),paper=ByteBuffer.wrap(expected).order(ByteOrder.BIG_ENDIAN);
                int count=0,different=0,nonfinite=0;double maximum=0;JsonArray firstDifferences=new JsonArray();
                initialize.invoke(chunk);
                for(int cx=0;cx<16/w;cx++){
                    advance.invoke(chunk,cx);
                    for(int cz=0;cz<16/w;cz++)for(int cy=height/h-1;cy>=0;cy--){
                        select.invoke(chunk,cy,cz);
                        for(int ly=h-1;ly>=0;ly--){
                            int yy=min+cy*h+ly;updateY.invoke(chunk,yy,(double)ly/h);
                            for(int lx=0;lx<w;lx++){
                                int xx=x+cx*w+lx;updateX.invoke(chunk,xx,(double)lx/w);
                                for(int lz=0;lz<w;lz++){
                                    int zz=z+cz*w+lz;updateZ.invoke(chunk,zz,(double)lz/w);
                                    double value=(Double)compute.invoke(material,chunk);long raw=Double.doubleToRawLongBits(value);
                                    int offset=Math.multiplyExact(((yy-min)*16+xx-x)*16+zz-z,8);vanilla.putLong(offset,raw);count++;
                                    long other=paper.getLong(offset);if(!Double.isFinite(value))nonfinite++;
                                    if(raw!=other){
                                        different++;double delta=Math.abs(value-Double.longBitsToDouble(other));if(Double.isFinite(delta))maximum=Math.max(maximum,delta);
                                        if(firstDifferences.size()<3){JsonObject r=NoiseChunkIndependentOracle.row("different-point",xx+"|"+yy+"|"+zz);r.addProperty("vanilla_bits",NoiseChunkIndependentOracle.bits(value));r.addProperty("paper_bits",String.format(Locale.ROOT,"%016x",other));firstDifferences.add(r);}
                                    }
                                }
                            }
                        }
                    }
                    swap.invoke(chunk);
                }
                stop.invoke(chunk);
                if(count!=bytes/8)throw new IllegalStateException("incomplete actual Vanilla chunk");
                JsonObject result=NoiseChunkIndependentOracle.row("full-vanilla-paper-comparison",key+"|"+request.get("chunk_x")+"|"+request.get("chunk_z"));
                result.add("paper_chunk",request);result.addProperty("samples",count);result.addProperty("different_bits",different);
                result.addProperty("vanilla_nonfinite",nonfinite);result.addProperty("max_finite_absolute_difference_bits",NoiseChunkIndependentOracle.bits(maximum));
                result.addProperty("vanilla_sha256",HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(vanilla.array())));
                result.addProperty("scalar_counter",(Long)NoiseChunkIndependentOracle.get(chunk,"NoiseChunk","interpolationCounter"));
                result.addProperty("array_counter",(Long)NoiseChunkIndependentOracle.get(chunk,"NoiseChunk","arrayInterpolationCounter"));
                result.add("first_differences",firstDifferences);NoiseChunkIndependentOracle.write(result);
            }
        }
    }
    public static void main(String[] args) throws Exception {
        if(args.length!=10)throw new IllegalArgumentException("MODE WORLDGEN MAPPINGS OUTPUT INNER CLASSES LIB_ORIGINS END_ROWS CHUNK_ROWS CHUNK_BITS required");
        bootstrap(args);List<JsonObject> chunks=records(Path.of(args[8]));
        try(PrintWriter output=new PrintWriter(Files.newBufferedWriter(Path.of(args[3])))){
            NoiseChunkIndependentOracle.output=output;endFallback(Path.of(args[7]));endCorners(chunks);
            if(!NoiseChunkIndependentOracle.paper)compareFullVanilla(chunks,Path.of(args[9]));
            JsonObject origins=NoiseChunkIndependentOracle.row("origins","actual-end-chunk-seams");origins.add("code_sources",NoiseChunkIndependentOracle.origins);NoiseChunkIndependentOracle.write(origins);
        }
    }
}
