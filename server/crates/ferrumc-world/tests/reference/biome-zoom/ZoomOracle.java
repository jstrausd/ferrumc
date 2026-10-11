package ferrumc.zoomreference;

import java.lang.reflect.Constructor;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Properties;

/** Calls only the actual original selection, hashing, distance and LCG methods. */
public final class ZoomOracle {
    private static final class Trace implements InvocationHandler {
        private final Method expected;
        private int calls;
        private int[] quart;

        Trace(Method expected) { this.expected = expected; }
        void reset() { this.calls = 0; this.quart = null; }
        @Override
        public Object invoke(Object proxy, Method method, Object[] args) {
            if (!method.equals(this.expected) || args == null || args.length != 3) {
                throw new IllegalStateException("Unexpected original source callback " + method);
            }
            if (++this.calls != 1) throw new IllegalStateException("Extra original source callback");
            this.quart = new int[] {(Integer)args[0],(Integer)args[1],(Integer)args[2]};
            return null;
        }
        int[] checked() {
            if (this.calls != 1 || this.quart == null) throw new IllegalStateException("Missing original callback");
            return this.quart.clone();
        }
    }

    private record Instance(Object manager, Trace trace, Object replaced, Trace replacedTrace) {}

    private static Class<?> original(String name, Path expected, List<String> origins) throws Exception {
        Class<?> c = Class.forName(name,false,ZoomOracle.class.getClassLoader());
        if (c.getProtectionDomain().getCodeSource() == null) throw new IllegalStateException("No CodeSource " + name);
        Path actual = Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
        if (!Files.isSameFile(actual,expected)) throw new IllegalStateException("Wrong CodeSource " + name + ": " + actual);
        origins.add(name + "\t" + actual);
        return c;
    }

    private static Method method(Class<?> c, String name, Class<?>... args) throws Exception {
        Method result = c.getDeclaredMethod(name,args);
        result.setAccessible(true);
        return result;
    }

    private static Object source(Class<?> iface, Trace trace) {
        return Proxy.newProxyInstance(ZoomOracle.class.getClassLoader(),new Class<?>[] {iface},trace);
    }

    private static Instance instance(long seed, Class<?> iface, Method callback,
                                     Constructor<?> constructor, Method replace) throws Exception {
        Trace trace = new Trace(callback), second = new Trace(callback);
        Object manager = constructor.newInstance(source(iface,trace),seed);
        Object replaced = replace.invoke(manager,source(iface,second));
        return new Instance(manager,trace,replaced,second);
    }

    private static int[] select(Object manager, Trace trace, Object pos, Method select) throws Exception {
        trace.reset();
        if (select.invoke(manager,pos) != null) throw new IllegalStateException("Original changed callback result");
        return trace.checked();
    }

    private static void same(int[] expected, int[] actual) {
        if (!Arrays.equals(expected,actual)) throw new IllegalStateException("Original source/history/fresh mismatch");
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 8) throw new IllegalArgumentException("mode requests output inner guava dfu helperdir symbols");
        if (!args[0].equals("vanilla") && !args[0].equals("paper")) throw new IllegalArgumentException("mode");
        Path input = Path.of(args[1]), output = Path.of(args[2]);
        Path inner = Path.of(args[3]).toRealPath(), guava = Path.of(args[4]).toRealPath();
        Path dfu = Path.of(args[5]).toRealPath(), helper = Path.of(args[6]).toRealPath();
        if (Files.exists(output)) throw new IllegalArgumentException("output must be fresh");
        Properties symbols = new Properties();
        try (var reader = Files.newBufferedReader(Path.of(args[7]),StandardCharsets.UTF_8)) { symbols.load(reader); }
        List<String> origins = new ArrayList<>();
        for (Class<?> c : List.of(ZoomOracle.class,Trace.class,Instance.class)) original(c.getName(),helper,origins);
        String prefix = args[0] + ".";
        Class<?> manager = original(symbols.getProperty(prefix+"manager"),inner,origins);
        Class<?> iface = original(symbols.getProperty(prefix+"source"),inner,origins);
        Class<?> pos = original(symbols.getProperty(prefix+"blockpos"),inner,origins);
        Class<?> lc = original(symbols.getProperty(prefix+"lcg"),inner,origins);
        for (String key : List.of("vec3i","quartpos","holder","mth")) original(symbols.getProperty(prefix+key),inner,origins);
        for (String name : List.of("com.google.common.hash.Hashing","com.google.common.hash.HashCode",
                "com.google.common.hash.HashFunction","com.google.common.hash.AbstractHasher",
                "com.google.common.hash.AbstractStreamingHasher","com.google.common.hash.PrimitiveSink")) original(name,guava,origins);
        for (String name : List.of("com.mojang.serialization.Codec","com.mojang.serialization.DataResult")) original(name,dfu,origins);
        Method hash = method(manager,symbols.getProperty(prefix+"hash"),long.class);
        Method fiddle = method(manager,symbols.getProperty(prefix+"fiddle"),long.class);
        Method distance = method(manager,symbols.getProperty(prefix+"distance"),long.class,int.class,int.class,int.class,
            double.class,double.class,double.class);
        Method choose = method(manager,symbols.getProperty(prefix+"choose"),pos);
        Method replace = method(manager,symbols.getProperty(prefix+"replace"),iface);
        Method callback = method(iface,"getNoiseBiome",int.class,int.class,int.class);
        Method next = method(lc,symbols.getProperty(prefix+"next"),long.class,long.class);
        Constructor<?> constructor = manager.getDeclaredConstructor(iface,long.class);
        Constructor<?> position = pos.getDeclaredConstructor(int.class,int.class,int.class);
        constructor.setAccessible(true); position.setAccessible(true);
        List<String> requests = Files.readAllLines(input,StandardCharsets.UTF_8);
        if (requests.size() > 100000) throw new IllegalArgumentException("oversized original probe corpus");
        Map<Long,Instance> instances = new LinkedHashMap<>();
        Map<String,int[]> selections = new LinkedHashMap<>();
        List<String> results = new ArrayList<>();
        for (String row : requests) {
            String[] f = row.split("\t",-1);
            if (f.length < 2 || !f[1].matches("[a-zA-Z0-9_.-]{1,100}")) throw new IllegalArgumentException("label");
            String result;
            switch (f[0]) {
                case "H" -> {
                    if (f.length != 3) throw new IllegalArgumentException("H arity");
                    result = Long.toString((Long)hash.invoke(null,Long.parseLong(f[2])));
                }
                case "F" -> {
                    if (f.length != 3) throw new IllegalArgumentException("F arity");
                    result = String.format("%016x",Double.doubleToRawLongBits((Double)fiddle.invoke(null,Long.parseLong(f[2]))));
                }
                case "L" -> {
                    if (f.length != 4) throw new IllegalArgumentException("L arity");
                    result = Long.toString((Long)next.invoke(null,Long.parseLong(f[2]),Long.parseLong(f[3])));
                }
                case "D" -> {
                    if (f.length != 9) throw new IllegalArgumentException("D arity");
                    double v = (Double)distance.invoke(null,Long.parseLong(f[2]),Integer.parseInt(f[3]),Integer.parseInt(f[4]),Integer.parseInt(f[5]),
                        Double.longBitsToDouble(Long.parseUnsignedLong(f[6],16)),Double.longBitsToDouble(Long.parseUnsignedLong(f[7],16)),
                        Double.longBitsToDouble(Long.parseUnsignedLong(f[8],16)));
                    if (!Double.isFinite(v)) throw new IllegalStateException("Unexpected nonfinite genuine distance");
                    result = String.format("%016x",Double.doubleToRawLongBits(v));
                }
                case "G" -> {
                    if (f.length != 7 || (!f[2].equals("world") && !f[2].equals("zoom"))) throw new IllegalArgumentException("G arity/seedkind");
                    long seed = Long.parseLong(f[3]);
                    if (f[2].equals("world")) seed = (Long)hash.invoke(null,seed);
                    Instance value = instances.get(seed);
                    if (value == null) {
                        if (instances.size() >= 4096) throw new IllegalArgumentException("too many managers");
                        value = instance(seed,iface,callback,constructor,replace); instances.put(seed,value);
                    }
                    Object block = position.newInstance(Integer.parseInt(f[4]),Integer.parseInt(f[5]),Integer.parseInt(f[6]));
                    int[] quart = select(value.manager(),value.trace(),block,choose);
                    same(quart,select(value.replaced(),value.replacedTrace(),block,choose));
                    Instance fresh = instance(seed,iface,callback,constructor,replace);
                    same(quart,select(fresh.manager(),fresh.trace(),block,choose));
                    if (selections.put(f[1],quart) != null) throw new IllegalArgumentException("duplicate G label");
                    result = seed+"\t"+quart[0]+"\t"+quart[1]+"\t"+quart[2]+"\t1\t1\t1";
                }
                default -> throw new IllegalArgumentException("unknown request type");
            }
            results.add(f[0]+"\t"+f[1]+"\t"+result);
        }
        Collections.reverse(requests);
        for (String row : requests) {
            String[] f = row.split("\t",-1);
            if (!f[0].equals("G")) continue;
            long seed = Long.parseLong(f[3]);
            if (f[2].equals("world")) seed = (Long)hash.invoke(null,seed);
            Instance value = instances.get(seed);
            Object block = position.newInstance(Integer.parseInt(f[4]),Integer.parseInt(f[5]),Integer.parseInt(f[6]));
            same(selections.get(f[1]),select(value.manager(),value.trace(),block,choose));
        }
        Files.write(output,results,StandardCharsets.UTF_8);
        Files.write(Path.of(output+".origins.tsv"),origins,StandardCharsets.UTF_8);
        System.out.println("Actual original " + args[0] + ": " + results.size()+" requests, "+selections.size()+" selected coordinates; replacement/fresh/reverse checked");
    }
}
