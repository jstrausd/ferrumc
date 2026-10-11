package ferrumc.temperatureoracle;

import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Properties;
import java.util.TreeMap;
import java.util.concurrent.atomic.AtomicLong;
import java.util.stream.Stream;

/** Reflection calls to genuine originals; no temperature/noise implementation. */
public final class TemperatureOracle {
    static final Properties symbols = new Properties();
    static final TreeMap<String,String> origins = new TreeMap<>();
    static final Map<String,Class<?>> types = new HashMap<>();
    static final TreeMap<String,Object> biomes = new TreeMap<>();
    static String mode;
    static Path inner;
    static Constructor<?> positionConstructor, biomeConstructor;
    static Method privateTemperature, publicTemperature, warm, cold, melt, precipitation, baseTemperature, modifierCall;
    static Field climateField, effectsField, generationField, mobsField, cacheField;
    static final String[] NOISE_NAMES = {"temperature", "frozen", "info"};
    static final Object[] noises = new Object[3];
    static Method noiseValue;

    static void require(boolean value,String message) {
        if (!value) throw new IllegalStateException(message);
    }
    static String symbol(String key) {
        String result = symbols.getProperty(mode + "." + key);
        require(result != null, "Missing checked symbol " + key);
        return result;
    }
    static Class<?> type(String key) throws Exception {
        Class<?> result = types.get(key);
        if (result == null) {
            result = Class.forName(symbol("class." + key), false, TemperatureOracle.class.getClassLoader());
            origin(result, inner, "$original-inner", key);
            types.put(key,result);
        }
        return result;
    }
    static Method method(String owner,String name,Class<?>... args) throws Exception {
        Method result = type(owner).getDeclaredMethod(symbol("method." + owner + "." + name),args);
        result.setAccessible(true);
        return result;
    }
    static Field field(String owner,String name) throws Exception {
        Field result = type(owner).getDeclaredField(symbol("field." + owner + "." + name));
        result.setAccessible(true);
        return result;
    }
    static void origin(Class<?> original,Path expected,String label,String semantic) throws Exception {
        require(original.getProtectionDomain().getCodeSource() != null,"Missing CodeSource " + original.getName());
        Path actual = Path.of(original.getProtectionDomain().getCodeSource().getLocation().toURI());
        require(Files.isSameFile(actual, expected),"Wrong CodeSource " + semantic);
        String record = original.getName() + "\t" + label;
        String previous = origins.putIfAbsent(semantic,record);
        require(previous == null || previous.equals(record),"CodeSource identity drift " + semantic);
    }
    static String floatBits(Object value) { return String.format("%08x", Float.floatToRawIntBits((Float)value)); }
    static String doubleBits(Object value) { return String.format("%016x", Double.doubleToRawLongBits((Double)value)); }
    static BufferedWriter output(Path directory,String name) throws Exception {
        return Files.newBufferedWriter(directory.resolve(name), StandardOpenOption.CREATE_NEW,StandardOpenOption.WRITE);
    }
    static void row(BufferedWriter out,Object... values) throws Exception {
        for(int i=0;i<values.length;i++) {
            if (i != 0) out.write('\t');
            out.write(String.valueOf(values[i]));
        }
        out.write('\n');
    }
    static Object freshBiome(Object original) throws Exception {
        Object result = biomeConstructor.newInstance(climateField.get(original),effectsField.get(original),generationField.get(original),mobsField.get(original));
        require(climateField.get(result) == climateField.get(original),"Original constructor changed climate identity");
        origin(result.getClass(),inner,"$original-inner","fresh-biome");
        return result;
    }
    static String keys(Object biome,Path fastutil) throws Exception {
        Object cache = ((ThreadLocal<?>)cacheField.get(biome)).get();
        origin(cache.getClass(), inner,"$original-inner","original-cache-subclass");
        Class<?> mapType = Class.forName("it.unimi.dsi.fastutil.longs.Long2FloatLinkedOpenHashMap");
        origin(mapType,fastutil,"$pinned-fastutil","fastutil-map");
        Object set = mapType.getMethod("keySet").invoke(cache);
        Class<?> collection = Class.forName("it.unimi.dsi.fastutil.longs.LongCollection");
        origin(collection,fastutil,"$pinned-fastutil","fastutil-long-collection");
        long[] values = (long[])collection.getMethod("toLongArray").invoke(set);
        StringBuilder result = new StringBuilder();
        for(long key:values) {
            if (!result.isEmpty()) result.append(',');
            result.append(key);
        }
        return result.toString();
    }
    static void bootstrap() throws Exception {
        method("SharedConstants","tryDetectVersion").invoke(null);
        method("Bootstrap","bootStrap").invoke(null);
        require(!(Boolean)field("SharedConstants","debugGenerateSquareTerrainWithoutNoise").get(null),"Debug terrain flag active");
        Object provider = method("VanillaRegistries","createLookup").invoke(null);
        origin(provider.getClass(),inner,"$original-inner","actual-registry-provider");
        Object lookup = method("Provider","lookupOrThrow",type("ResourceKey")).invoke(provider,field("Registries","BIOME").get(null));
        origin(lookup.getClass(),inner,"$original-inner","actual-biome-lookup");
        Method referenceKey = method("Reference","key"), referenceValue = method("Reference","value");
        Method location = method("ResourceKey","location");
        try(Stream<?> entries=(Stream<?>)method("HolderLookup","listElements").invoke(lookup)) {
            for(Object holder:entries.toList()) {
                origin(holder.getClass(),inner,"$original-inner","actual-biome-holder");
                String key = location.invoke(referenceKey.invoke(holder)).toString();
                require(biomes.put(key,referenceValue.invoke(holder)) == null,"Duplicate actual builtin biome");
            }
        }
        require(biomes.size() == 65,"Actual builtin biome count changed");
    }
    static void metadata(Path directory) throws Exception {
        Method precipFlag = method("Biome","hasPrecipitation");
        Method climateModifier = method("Climate","temperatureModifier"), climateTemperature = method("Climate","temperature");
        try(BufferedWriter out=output(directory,"metadata.tsv")) {
            for(var entry:biomes.entrySet()) {
                Object climate = climateField.get(entry.getValue());
                Object modifier = climateModifier.invoke(climate);
                origin(modifier.getClass(),inner,"$original-inner","modifier-" + ((Enum<?>)modifier).name());
                row(out,entry.getKey(),floatBits(baseTemperature.invoke(entry.getValue())),precipFlag.invoke(entry.getValue()),
                    ((Enum<?>)modifier).name(),floatBits(climateTemperature.invoke(climate)));
            }
        }
    }
    static void constructorProbes(Path directory) throws Exception {
        Class<?> random = type("Random"), legacy = type("Legacy"), worldgen = type("Worldgen"), perlin = type("Perlin");
        Constructor<?> legacyCtor = legacy.getConstructor(long.class), countedCtor = worldgen.getConstructor(random);
        Constructor<?> noiseCtor = perlin.getConstructor(random,List.class);
        Method count = method("Worldgen","getCount");
        Field levels = field("Perlin","noiseLevels"), inputFactor = field("Perlin","highestFreqInputFactor"), valueFactor = field("Perlin","highestFreqValueFactor");
        Field permutation = field("Simplex","p"), xo=field("Simplex","xo"), yo=field("Simplex","yo"), zo=field("Simplex","zo");
        try(BufferedWriter out=output(directory,"constructors.tsv")) {
            for(int i=0;i<3;i++) {
                long seed = new long[]{1234,3456,2345}[i];
                Object legacyObject = legacyCtor.newInstance(seed), counted = countedCtor.newInstance(legacyObject);
                Object constructed = noiseCtor.newInstance(counted,i == 1 ? List.of(-2,-1,0) : List.of(0));
                noises[i] = field("Biome",new String[]{"TEMPERATURE_NOISE","FROZEN_TEMPERATURE_NOISE","BIOME_INFO_NOISE"}[i]).get(null);
                Object[] originalLevels = (Object[])levels.get(noises[i]), independentLevels=(Object[])levels.get(constructed);
                require(originalLevels.length == independentLevels.length,"Actual original static level count changed");
                require(doubleBits(inputFactor.get(noises[i])).equals(doubleBits(inputFactor.get(constructed))) &&
                        doubleBits(valueFactor.get(noises[i])).equals(doubleBits(valueFactor.get(constructed))),"Independent original constructor factors differ");
                row(out,"N",NOISE_NAMES[i],seed,count.invoke(counted),((AtomicLong)field("Legacy","seed").get(legacyObject)).get(),
                    doubleBits(inputFactor.get(constructed)),doubleBits(valueFactor.get(constructed)),originalLevels.length);
                for(int level=0;level<originalLevels.length;level++) {
                    Object actual = originalLevels[level], independent = independentLevels[level];
                    origin(actual.getClass(),inner,"$original-inner","actual-simplex-" + i + "-" + level);
                    require(Arrays.equals((int[])permutation.get(actual),(int[])permutation.get(independent)),"Independent actual permutation differs");
                    for(Field offset:List.of(xo,yo,zo)) require(doubleBits(offset.get(actual)).equals(doubleBits(offset.get(independent))),"Independent actual offset differs");
                    StringBuilder p = new StringBuilder();
                    for(int value:(int[])permutation.get(actual)) {
                        if (!p.isEmpty()) p.append(',');
                        p.append(value);
                    }
                    row(out,"P",NOISE_NAMES[i],level,doubleBits(xo.get(actual)),doubleBits(yo.get(actual)),doubleBits(zo.get(actual)),p);
                }
            }
        }
    }
    static void noiseProbes(Path directory,Path inputs) throws Exception {
        try(BufferedReader input=Files.newBufferedReader(inputs);BufferedWriter out=output(directory,"noise.tsv")) {
            require(input.readLine().equals("id\tx\tz"),"Noise input header changed");
            for(String line;(line=input.readLine()) != null;) {
                String[] r = line.split("\t");int x=Integer.parseInt(r[1]),z=Integer.parseInt(r[2]);
                double[] xs = {(double)(x/8.0F),x*0.05,x*0.2,x*0.09};
                double[] zs = {(double)(z/8.0F),z*0.05,z*0.2,z*0.09};
                for(int i=0;i<4;i++) row(out,r[0] + "/" + i,x,z,doubleBits(xs[i]),doubleBits(zs[i]),
                        doubleBits(noiseValue.invoke(noises[new int[]{0,1,2,2}[i]],xs[i],zs[i],false)));
            }
        }
    }
    static String predicates(Object biome,Object position,int sea) throws Exception {
        return warm.invoke(biome,position,sea) + "\t" + cold.invoke(biome,position,sea) + "\t" + melt.invoke(biome,position,sea)
            + "\t" + ((Enum<?>)precipitation.invoke(biome,position,sea)).name();
    }
    static void scalar(Path directory,Path inputs,String filename,boolean frozenNoiseEvidence) throws Exception {
        Method climateModifier=method("Climate","temperatureModifier");
        try(BufferedReader input=Files.newBufferedReader(inputs);BufferedWriter out=output(directory,filename)) {
            require(input.readLine().equals("id\tbiome\tx\ty\tz\tsea"),"Scalar input header changed");
            for(String line;(line=input.readLine()) != null;) {
                String[] r=line.split("\t");Object biome=biomes.get(r[1]);require(biome != null,"Unknown builtin requested");
                Object position=positionConstructor.newInstance(Integer.parseInt(r[2]),Integer.parseInt(r[3]),Integer.parseInt(r[4]));int sea=Integer.parseInt(r[5]);
                Object adjusted=privateTemperature.invoke(biome,position,sea);
                Object modified=modifierCall.invoke(climateModifier.invoke(climateField.get(biome)),position,baseTemperature.invoke(biome));
                String observations = "";
                if (frozenNoiseEvidence) {
                    int x=Integer.parseInt(r[2]),z=Integer.parseInt(r[4]);
                    observations = "\t" + doubleBits(noiseValue.invoke(noises[1],x*0.05,z*0.05,false))
                        + "\t" + doubleBits(noiseValue.invoke(noises[2],x*0.2,z*0.2,false))
                        + "\t" + doubleBits(noiseValue.invoke(noises[2],x*0.09,z*0.09,false));
                }
                row(out,line,floatBits(adjusted),floatBits(modified),mode.equals("paper") ? floatBits(publicTemperature.invoke(biome,position,sea)) : "-",
                    (mode.equals("paper") ? predicates(biome,position,sea) : "-\t-\t-\t-") + observations);
            }
        }
    }
    static void controls(Path directory,Path inputs,Path fastutil) throws Exception {
        Object mutable=type("Mutable").getConstructor().newInstance();
        Method set=method("Mutable","set",int.class,int.class,int.class), packed=method("BlockPos","asLong");
        Object retained=null;String currentSequence=null;
        try(BufferedReader input=Files.newBufferedReader(inputs);BufferedWriter reuse=output(directory,"mutable.tsv");BufferedWriter cache=output(directory,"cache.tsv")) {
            require(input.readLine().equals("kind\tid\tbiome\tx\ty\tz\tsea\tsequence\treset"),"Control input header changed");
            for(String line;(line=input.readLine()) != null;) {
                String[] r=line.split("\t");Object builtin=biomes.get(r[2]);require(builtin != null,"Unknown control biome");
                int x=Integer.parseInt(r[3]),y=Integer.parseInt(r[4]),z=Integer.parseInt(r[5]),sea=Integer.parseInt(r[6]);
                Object immutable=positionConstructor.newInstance(x,y,z);
                if(r[0].equals("M")) {
                    require(set.invoke(mutable,x,y,z) == mutable,"Original MutableBlockPos.set changed object identity");
                    row(reuse,r[1],r[2],x,y,z,sea,floatBits(privateTemperature.invoke(builtin,mutable,sea)),
                        floatBits(privateTemperature.invoke(builtin,immutable,sea)),
                        mode.equals("paper") ? floatBits(publicTemperature.invoke(builtin,mutable,sea)) : "-",
                        mode.equals("paper") ? floatBits(publicTemperature.invoke(builtin,immutable,sea)) : "-");
                } else {
                    require(r[0].equals("C"),"Unknown control kind");
                    String sequence = r[2] + "/" + r[7];
                    if(r[8].equals("true")) {retained=freshBiome(builtin);currentSequence=sequence;}
                    require(retained != null && sequence.equals(currentSequence),"Control reset boundary changed");
                    Object fresh=freshBiome(builtin);
                    Object privateValue=privateTemperature.invoke(builtin,immutable,sea);
                    Object retainedValue=publicTemperature.invoke(retained,immutable,sea);
                    Object coldValue=publicTemperature.invoke(fresh,immutable,sea);
                    row(cache,r[1],r[2],x,y,z,sea,packed.invoke(immutable),floatBits(privateValue),floatBits(retainedValue),floatBits(coldValue),keys(retained,fastutil));
                }
            }
        }
    }
    public static void main(String[] args) throws Exception {
        require(args.length == 10,"MODE OUT INNER CLASSES SYMBOLS SCALAR CONTROLS NOISE LIBRARY_ORIGINS FROZEN_EXTENSION required");
        mode=args[0];require(mode.equals("vanilla") || mode.equals("paper"),"Unknown original mode");
        Path directory=Path.of(args[1]),classes=Path.of(args[3]);inner=Path.of(args[2]);
        try(var input=Files.newBufferedReader(Path.of(args[4]))) {symbols.load(input);}
        origin(TemperatureOracle.class,classes,"$fresh-helper-classes","TemperatureOracle");
        for(String key:symbols.stringPropertyNames()) if(key.startsWith(mode + ".class.")) type(key.substring((mode + ".class.").length()));
        Properties libraries=new Properties();try(var input=Files.newBufferedReader(Path.of(args[8]))) {libraries.load(input);}
        for(String name:libraries.stringPropertyNames()) origin(Class.forName(name,false,TemperatureOracle.class.getClassLoader()),Path.of(libraries.getProperty(name)),"$pinned-library",name);
        bootstrap();
        Class<?> pos=type("BlockPos");positionConstructor=pos.getConstructor(int.class,int.class,int.class);
        biomeConstructor=type("Biome").getDeclaredConstructor(type("Climate"),type("Effects"),type("Generation"),type("Mobs"));biomeConstructor.setAccessible(true);
        climateField=field("Biome","climateSettings");effectsField=field("Biome","specialEffects");generationField=field("Biome","generationSettings");mobsField=field("Biome","mobSettings");cacheField=field("Biome","temperatureCache");
        privateTemperature=method("Biome","getHeightAdjustedTemperature",pos,int.class);publicTemperature=method("Biome","getTemperature",pos,int.class);
        warm=method("Biome","warmEnoughToRain",pos,int.class);cold=method("Biome","coldEnoughToSnow",pos,int.class);melt=method("Biome","shouldMeltFrozenOceanIcebergSlightly",pos,int.class);precipitation=method("Biome","getPrecipitationAt",pos,int.class);
        baseTemperature=method("Biome","getBaseTemperature");modifierCall=method("Modifier","modifyTemperature",pos,float.class);noiseValue=method("Perlin","getValue",double.class,double.class,boolean.class);
        metadata(directory);constructorProbes(directory);noiseProbes(directory,Path.of(args[7]));scalar(directory,Path.of(args[5]),"scalar.tsv",false);
        scalar(directory,Path.of(args[9]),"frozen-extension.tsv",true);
        controls(directory,Path.of(args[6]),Path.of(libraries.getProperty("it.unimi.dsi.fastutil.longs.Long2FloatLinkedOpenHashMap")));
        try(BufferedWriter out=output(directory,"origins.tsv")) {for(var entry:origins.entrySet()) row(out,entry.getKey(),entry.getValue());}
    }
}
