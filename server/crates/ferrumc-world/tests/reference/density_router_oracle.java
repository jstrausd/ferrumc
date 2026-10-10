package ferrumc.reference;

import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.mojang.serialization.Codec;
import com.mojang.serialization.DataResult;
import com.mojang.serialization.DynamicOps;
import com.mojang.serialization.JsonOps;
import java.io.PrintWriter;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.TreeMap;

/** Actual original RandomState/visitor/scalar/climate projection; no copied formulas. */
final class DensityRouterOracle {
    private static final Gson GSON = new Gson();
    private static final String[] ROUTER_FIELDS = {
        "barrier", "fluid_level_floodedness", "fluid_level_spread", "lava",
        "temperature", "vegetation", "continents", "erosion", "depth", "ridges",
        "initial_density_without_jaggedness", "final_density", "vein_toggle", "vein_ridged", "vein_gap"
    };
    private static final String[] GEOMETRY_FIELDS = {"min_y", "height", "size_horizontal", "size_vertical"};
    private static DynamicOps<JsonElement> ops;
    private static Codec<Object> direct;
    private static Codec<Object> holderHelper;
    private static Codec<Object> settingsCodec;

    private static Class<?> type(String name) throws Exception { return Class.forName(name); }
    private static Object field(Object object, String name) throws Exception {
        Field field = object.getClass().getDeclaredField(name);
        field.setAccessible(true);
        return field.get(object);
    }
    private static Object invoke(Object object, String name) throws Exception {
        Method method = object.getClass().getDeclaredMethod(name);
        method.setAccessible(true);
        return method.invoke(object);
    }
    private static String bits(double value) { return String.format(Locale.ROOT, "%016x", Double.doubleToRawLongBits(value)); }
    private static String bits(float value) { return String.format(Locale.ROOT, "%08x", Float.floatToRawIntBits(value)); }
    private static JsonObject node(String kind) {
        JsonObject node = new JsonObject();
        node.addProperty("kind", kind);
        return node;
    }
    private static void doubleField(JsonObject result, String key, Object object, String field) throws Exception {
        result.addProperty(key + "_bits", bits((Double) field(object, field)));
    }
    private static String serialized(Object object) throws Exception { return (String) invoke(object, "c"); }

    private static JsonObject holder(Object holder, boolean density) throws Exception {
        // jl is the actual Holder interface: e=unwrapKey, a=value, g=getRegisteredName.
        Optional<?> key = (Optional<?>) type("jl").getMethod("e").invoke(holder);
        JsonObject result = new JsonObject();
        if (key.isPresent()) {
            result.addProperty("resource", (String) type("jl").getMethod("g").invoke(holder));
        } else {
            Object value = type("jl").getMethod("a").invoke(holder);
            if (density) {
                result.add("inline", density(value));
            } else {
                JsonObject parameters = new JsonObject();
                parameters.addProperty("first_octave", (Integer) field(value, "c"));
                JsonArray amplitudes = new JsonArray();
                for (Object amplitude : (List<?>) field(value, "d")) amplitudes.add(bits((Double) amplitude));
                parameters.add("amplitudes_bits", amplitudes);
                result.add("inline", parameters);
            }
        }
        return result;
    }
    private static JsonObject noiseHolder(Object noiseHolder) throws Exception { return holder(field(noiseHolder, "b"), false); }

    private static JsonObject spline(Object spline) throws Exception {
        JsonObject result;
        if (spline.getClass().getName().equals("bbb$c")) {
            result = node("constant");
            result.addProperty("value_bits", bits((Float) field(spline, "b")));
        } else {
            if (!spline.getClass().getName().equals("bbb$e")) throw new IllegalStateException("unknown original spline class");
            result = node("multipoint");
            result.add("coordinate", holder(field(field(spline, "b"), "c"), true));
            float[] locations = (float[]) field(spline, "c");
            List<?> values = (List<?>) field(spline, "d");
            float[] derivatives = (float[]) field(spline, "e");
            JsonArray points = new JsonArray();
            for (int i = 0; i < locations.length; i++) {
                JsonObject point = new JsonObject();
                point.addProperty("location_bits", bits(locations[i]));
                point.addProperty("derivative_bits", bits(derivatives[i]));
                point.add("value", spline(values.get(i)));
                points.add(point);
            }
            result.add("points", points);
        }
        result.addProperty("min_value_bits", bits((Float)type("bdh").getMethod("b").invoke(spline)));
        result.addProperty("max_value_bits", bits((Float)type("bdh").getMethod("c").invoke(spline)));
        return result;
    }

    private static JsonObject densityShape(Object density) throws Exception {
        JsonObject result;
        switch (density.getClass().getName()) {
            case "eju$a":
                result = node("binary");
                result.addProperty("operation", serialized(field(density, "e")));
                result.add("argument1", density(field(density, "f")));
                result.add("argument2", density(field(density, "g")));
                return result;
            case "eju$b": return node("beardifier");
            case "eju$d": return node("blend_alpha");
            case "eju$e":
                result = node("blend_density");
                result.add("argument", density(field(density, "a")));
                return result;
            case "eju$f": return node("blend_offset");
            case "eju$g":
                result = node("clamp");
                result.add("input", density(field(density, "e")));
                doubleField(result, "min", density, "f");
                doubleField(result, "max", density, "g");
                return result;
            case "eju$h":
                result = node("constant");
                doubleField(result, "value", density, "a");
                return result;
            case "eju$i": return node("end_islands");
            case "eju$j":
                result = node("holder");
                for (Map.Entry<String, JsonElement> entry : holder(field(density, "a"), true).entrySet()) result.add(entry.getKey(), entry.getValue());
                return result;
            case "eju$k":
                result = node("mapped");
                result.addProperty("operation", serialized(field(density, "a")));
                result.add("argument", density(field(density, "e")));
                return result;
            case "eju$l":
                result = node("marker");
                result.addProperty("operation", serialized(field(density, "a")));
                result.add("argument", density(field(density, "e")));
                return result;
            case "eju$n":
                result = node("mul_or_add");
                result.addProperty("operation", ((Enum<?>) field(density, "e")).name().toLowerCase(Locale.ROOT));
                result.add("argument", density(field(density, "f")));
                doubleField(result, "mapped_argument", density, "i");
                return result;
            case "eju$o":
                result = node("noise");
                result.add("noise", noiseHolder(field(density, "f")));
                doubleField(result, "xz_scale", density, "g");
                doubleField(result, "y_scale", density, "h");
                return result;
            case "eju$q":
                result = node("range_choice");
                result.add("input", density(field(density, "f")));
                doubleField(result, "min_inclusive", density, "g");
                doubleField(result, "max_exclusive", density, "h");
                result.add("when_in_range", density(field(density, "i")));
                result.add("when_out_of_range", density(field(density, "j")));
                return result;
            case "eju$r", "eju$s", "eju$t":
                result = node(switch (density.getClass().getName()) { case "eju$r" -> "shift"; case "eju$s" -> "shift_a"; default -> "shift_b"; });
                result.add("argument", noiseHolder(field(density, "a")));
                return result;
            case "eju$v":
                result = node("shifted_noise");
                result.add("shift_x", density(field(density, "e")));
                result.add("shift_y", density(field(density, "f")));
                result.add("shift_z", density(field(density, "g")));
                doubleField(result, "xz_scale", density, "h");
                doubleField(result, "y_scale", density, "i");
                result.add("noise", noiseHolder(field(density, "j")));
                return result;
            case "eju$w":
                result = node("spline");
                result.add("spline", spline(field(density, "e")));
                return result;
            case "eju$z":
                result = node("weird_scaled_sampler");
                result.add("input", density(field(density, "e")));
                result.add("noise", noiseHolder(field(density, "f")));
                result.addProperty("rarity_value_mapper", serialized(field(density, "g")));
                return result;
            case "eju$aa":
                result = node("y_clamped_gradient");
                result.addProperty("from_y", (Integer) field(density, "e"));
                result.addProperty("to_y", (Integer) field(density, "f"));
                doubleField(result, "from_value", density, "g");
                doubleField(result, "to_value", density, "h");
                return result;
            case "ezl":
                result = node("old_blended_noise");
                for (String[] pair : new String[][]{{"xz_scale", "p"}, {"y_scale", "q"}, {"xz_factor", "l"}, {"y_factor", "m"}, {"smear_scale_multiplier", "n"}}) doubleField(result, pair[0], density, pair[1]);
                return result;
            default: throw new IllegalStateException("unknown original density class: " + density.getClass().getName());
        }
    }

    private static JsonObject density(Object density) throws Exception {
        JsonObject result = densityShape(density);
        result.addProperty("min_value_bits", bits((Double)type("ejt").getMethod("a").invoke(density)));
        result.addProperty("max_value_bits", bits((Double)type("ejt").getMethod("b").invoke(density)));
        return result;
    }

    private static JsonObject settings(Object settings) throws Exception {
        JsonObject result = new JsonObject();
        Object geometry = field(settings, "j");
        JsonObject noise = new JsonObject();
        for (int i = 0; i < GEOMETRY_FIELDS.length; i++) noise.addProperty(GEOMETRY_FIELDS[i], (Integer) field(geometry, String.valueOf((char) ('g' + i))));
        noise.addProperty("cell_height", (Integer) invoke(geometry, "a"));
        noise.addProperty("cell_width", (Integer) invoke(geometry, "b"));
        result.add("noise", noise);
        Object router = field(settings, "m");
        JsonObject routerResult = new JsonObject();
        for (int i = 0; i < ROUTER_FIELDS.length; i++) routerResult.add(ROUTER_FIELDS[i], density(field(router, String.valueOf((char) ('b' + i)))));
        result.add("noise_router", routerResult);
        result.addProperty("sea_level", (Integer) field(settings, "p"));
        String[] flags = {"disable_mob_generation", "aquifers_enabled", "ore_veins_enabled", "legacy_random_source"};
        for (int i = 0; i < flags.length; i++) result.addProperty(flags[i], (Boolean) field(settings, String.valueOf((char) ('q' + i))));
        return result;
    }

    private static JsonElement canonical(JsonElement value) {
        if (value.isJsonObject()) {
            JsonObject result = new JsonObject();
            TreeMap<String, JsonElement> sorted = new TreeMap<>();
            value.getAsJsonObject().entrySet().forEach(entry -> sorted.put(entry.getKey(), entry.getValue()));
            sorted.forEach((key, child) -> result.add(key, canonical(child)));
            return result;
        }
        if (value.isJsonArray()) {
            JsonArray result = new JsonArray();
            value.getAsJsonArray().forEach(child -> result.add(canonical(child)));
            return result;
        }
        return value;
    }

    private static void write(PrintWriter output, JsonObject record) {
        output.println(GSON.toJson(canonical(record)));
    }

    private static JsonObject record(String kind, String settings, long seed) {
        JsonObject result = new JsonObject();
        result.addProperty("kind", kind);
        result.addProperty("settings", settings);
        result.addProperty("seed", seed);
        return result;
    }

    private static JsonArray bounds(Object function) throws Exception {
        JsonArray result = new JsonArray();
        result.add(bits((Double)type("ejt").getMethod("a").invoke(function)));
        result.add(bits((Double)type("ejt").getMethod("b").invoke(function)));
        return result;
    }

    private static Object context(int[] xyz) throws Exception {
        return type("ejt$e").getConstructor(int.class, int.class, int.class)
            .newInstance(xyz[0], xyz[1], xyz[2]);
    }

    private static double sample(Object function, Object context) throws Exception {
        return (Double)type("ejt").getMethod("a", type("ejt$b")).invoke(function, context);
    }

    private static void position(JsonObject record, int[] xyz) {
        JsonArray result = new JsonArray();
        for (int value : xyz) result.add(value);
        record.add("position", result);
    }

    private static java.util.List<int[]> positions(Object settings) throws Exception {
        java.util.Map<String, int[]> points = new java.util.LinkedHashMap<>();
        java.util.function.Consumer<int[]> add = p -> points.put(p[0] + "," + p[1] + "," + p[2], p);
        for (int[] point : new int[][]{
            {0,0,0}, {1,1,1}, {-1,-1,-1}, {13,73,-29}, {-127,-63,255},
            {-17,16,17}, {-16,15,16}, {-15,17,15}, {15,-17,-15}, {16,-16,-16}, {17,-15,-17},
            {30000000,64,30000000}, {-30000000,64,-30000000},
            {30000000,-64,-30000000}, {-30000000,319,30000000},
            {29999999,65,-29999999}, {-29999999,-65,29999999},
            {11184,80,-11184}, {11185,80,-11185}, {-11185,80,11185},
            {22369,0,22370}, {-22370,0,-22369},
            {370720,64,0}, {370728,64,0}, {370736,64,0},
            {-370728,64,0}, {262136,64,262136}, {262144,64,262144}, {262152,64,262152},
            {0,Integer.MIN_VALUE,0}, {0,Integer.MAX_VALUE,0}
        }) add.accept(point);
        Object geometry = field(settings, "j");
        int min = (Integer)field(geometry, "g");
        int max = min + (Integer)field(geometry, "h");
        for (int y : new int[]{min-1, min, min+1, max-1, max, max+1}) add.accept(new int[]{31,y,-17});
        return new java.util.ArrayList<>(points.values());
    }

    private static JsonArray climateValues(Object[] climate, Object context) throws Exception {
        JsonArray result = new JsonArray();
        for (Object function : climate) result.add(bits((float)sample(function, context)));
        return result;
    }

    private static JsonArray quantized(JsonArray values) throws Exception {
        JsonArray result = new JsonArray();
        for (JsonElement value : values) {
            float coordinate = Float.intBitsToFloat((int)Long.parseLong(value.getAsString(),16));
            result.add((Long)type("dog").getMethod("a", float.class).invoke(null, coordinate));
        }
        return result;
    }

    private static Object map(Object function, Object visitor) throws Exception {
        return type("ejt").getMethod("a", type("ejt$f")).invoke(function, visitor);
    }

    private static Object registryValue(Object getter, String registryField, String name) throws Exception {
        Object location = type("ame").getMethod("a",String.class).invoke(null,name);
        Object key = type("amd").getMethod("a",type("amd"),type("ame"))
            .invoke(null,type("mn").getField(registryField).get(null),location);
        Object holder = type("jm").getMethod("b",type("amd")).invoke(getter,key);
        return type("jl").getMethod("a").invoke(holder);
    }

    private static JsonArray positionalDraws(Object factory) throws Exception {
        int[][] positions = {{0,0,0},{-1,-1,-1},{-17,16,17},{16,-16,-16},
            {30000000,Integer.MIN_VALUE,-30000000},{-30000000,Integer.MAX_VALUE,30000000},{29999999,319,-29999999}};
        JsonArray result = new JsonArray();
        for (int[] xyz : positions) {
            JsonObject draw = new JsonObject();
            position(draw,xyz);
            draw.add("next_longs",threeLongs(type("ekn").getMethod("a",int.class,int.class,int.class)
                .invoke(factory,xyz[0],xyz[1],xyz[2])));
            result.add(draw);
        }
        for (int i=positions.length-1;i>=0;i--) {
            int[] xyz = positions[i];
            JsonArray reverse = threeLongs(type("ekn").getMethod("a",int.class,int.class,int.class)
                .invoke(factory,xyz[0],xyz[1],xyz[2]));
            if (!reverse.equals(result.get(i).getAsJsonObject().get("next_longs")))
                throw new IllegalStateException("original positional draws depend on request order");
        }
        return result;
    }

    private static JsonArray threeLongs(Object random) throws Exception {
        JsonArray result = new JsonArray();
        for (int i=0;i<3;i++) result.add((Long)type("bck").getMethod("g").invoke(random));
        return result;
    }

    private static Map<String,double[]> wrappedInputs() throws Exception {
        Map<String,double[]> result = new LinkedHashMap<>();
        result.put("origin",new double[]{0,0,0});
        result.put("signed_zero",new double[]{-0.0,0.0,-0.0});
        result.put("mixed",new double[]{13.25,-73.5,29.75});
        result.put("worldborder",new double[]{30000000,64,-30000000});
        result.put("jagged_positive_45b",new double[]{45000000000.0,0,-45000000000.0});
        result.put("jagged_negative_45b",new double[]{-45000000000.0,0,45000000000.0});
        double half = 16777216.0;
        for (String direction : new String[]{"positive","negative"}) {
            double value = direction.equals("positive") ? half : -half;
            result.put("half_"+direction+"_below",new double[]{Math.nextDown(value),1,0});
            result.put("half_"+direction+"_equal",new double[]{value,1,0});
            result.put("half_"+direction+"_above",new double[]{Math.nextUp(value),1,0});
        }
        result.put("multiple_periods",new double[]{117440512.0,-83886080.0,150994944.0});
        // The actual retained NormalNoise constant determines these probe inputs;
        // sample answers and per-octave wrapping remain original method calls.
        Field scale = type("ezo").getDeclaredField("a");
        scale.setAccessible(true);
        double secondBoundary = half / (Double)scale.get(null);
        result.put("second_half_below",new double[]{Math.nextDown(secondBoundary),0,-secondBoundary});
        result.put("second_half_equal",new double[]{secondBoundary,0,-secondBoundary});
        result.put("second_half_above",new double[]{Math.nextUp(secondBoundary),0,-secondBoundary});
        return result;
    }

    private static Object boundNoise(Object visitor, Object getter, String name) throws Exception {
        Object location = type("ame").getMethod("a",String.class).invoke(null,name);
        Object key = type("amd").getMethod("a",type("amd"),type("ame"))
            .invoke(null,type("mn").getField("bd").get(null),location);
        Object holder = type("jm").getMethod("b",type("amd")).invoke(getter,key);
        Object unbound = type("ejt$c").getConstructor(type("jl")).newInstance(holder);
        Object bound = type("ejt$f").getMethod("a",type("ejt$c")).invoke(visitor,unbound);
        return type("ejt$c").getMethod("c").invoke(bound);
    }

    private static JsonObject wrappedRecord(String settings,long seed,String noise,String mode,
            String stack,String name,double[] xyz,Object sampler) throws Exception {
        JsonObject result = record("wrapped_noise",settings,seed);
        result.addProperty("case",name);
        result.addProperty("noise",noise);
        result.addProperty("stack",stack);
        result.addProperty("name_mode",mode);
        JsonArray inputs = new JsonArray();
        for (double value:xyz) inputs.add(bits(value));
        result.add("input_bits",inputs);
        Method sample = sampler.getClass().getMethod("a",double.class,double.class,double.class);
        result.addProperty("result_bits",bits((Double)sample.invoke(sampler,xyz[0],xyz[1],xyz[2])));
        Method maximum = sampler.getClass().getDeclaredMethod("a");
        maximum.setAccessible(true);
        result.addProperty("max_value_bits",bits((Double)maximum.invoke(sampler)));
        return result;
    }

    private static void exportWrapped(PrintWriter output,String settings,long seed,Object randomState,
            Object getter,Object visitor) throws Exception {
        Map<String,double[]> inputs = wrappedInputs();
        String[] noises = {"minecraft:jagged","minecraft:temperature","minecraft:vegetation","minecraft:offset","minecraft:continentalness"};
        for (int slot=0;slot<noises.length;slot++) {
            String name = noises[slot];
            for (String mode:new String[]{"canonical","shortname","slot"}) {
                Object normal;
                if (mode.equals("canonical")) normal = boundNoise(visitor,getter,name);
                else {
                    String randomName = mode.equals("shortname") ? name.substring("minecraft:".length()) : "slot_"+slot;
                    Object rng = type("ekn").getMethod("a",String.class).invoke(field(randomState,"a"),randomName);
                    normal = type("ezo").getMethod("b",type("bck"),type("ezo$a"))
                        .invoke(null,rng,registryValue(getter,"bd",name));
                }
                Object[] samplers = {normal,field(normal,"d"),field(normal,"e")};
                String[] stacks = {"normal","first","second"};
                List<JsonObject> records = new ArrayList<>();
                for (Map.Entry<String,double[]> input:inputs.entrySet()) {
                    if (!mode.equals("canonical") && !input.getKey().equals("mixed") && !input.getKey().equals("jagged_positive_45b")) continue;
                    for (int i=0;i<3;i++) {
                        JsonObject record = wrappedRecord(settings,seed,name,mode,stacks[i],input.getKey(),input.getValue(),samplers[i]);
                        records.add(record);
                        write(output,record);
                    }
                }
                Collections.reverse(records);
                for (JsonObject original:records) {
                    int index = List.of(stacks).indexOf(original.get("stack").getAsString());
                    JsonObject reverse = wrappedRecord(settings,seed,name,mode,stacks[index],original.get("case").getAsString(),
                        inputs.get(original.get("case").getAsString()),samplers[index]);
                    if (!reverse.equals(original)) throw new IllegalStateException("original wrapped noise depends on request order");
                }
            }
        }
    }

    private static void exportQuantize(PrintWriter output) throws Exception {
        java.util.Set<Integer> raw = new java.util.LinkedHashSet<>();
        for (int value:new int[]{0,0x80000000,0x7f800000,0xff800000,0x7fc00000,0xffc00000,
                0x7fc12345,0xffc12345,1,0x80000001,0x7f7fffff,0xff7fffff}) raw.add(value);
        for (float center:new float[]{1,-1,0.0001f,-0.0001f,0.00015f,-0.00015f,
                1677.7216f,-1677.7216f,(float)(Long.MAX_VALUE/10000.0),(float)(Long.MIN_VALUE/10000.0)}) {
            raw.add(Float.floatToRawIntBits(Math.nextDown(center)));
            raw.add(Float.floatToRawIntBits(center));
            raw.add(Float.floatToRawIntBits(Math.nextUp(center)));
        }
        for (int value:raw) {
            float input=Float.intBitsToFloat(value);
            JsonObject result=node("quantize");
            result.addProperty("case","f32_"+bits(input));
            result.addProperty("input_bits",bits(input));
            result.addProperty("quantized",(Long)type("dog").getMethod("a",float.class).invoke(null,input));
            write(output,result);
        }
    }

    private static final class ProbeFailure extends RuntimeException {}

    private static Object probe(String label,double value,double min,double max,List<String> trace,String fail) throws Exception {
        return Proxy.newProxyInstance(DensityRouterOracle.class.getClassLoader(),new Class<?>[]{type("ejt")},(proxy,method,args)->{
            if (method.getName().equals("toString")) return "original-oracle-child:"+label;
            if (method.getName().equals("hashCode")) return System.identityHashCode(proxy);
            if (method.getName().equals("equals")) return proxy==args[0];
            if (method.getName().equals("a") && method.getParameterCount()==0) return min;
            if (method.getName().equals("b") && method.getParameterCount()==0) return max;
            if (method.getName().equals("a") && method.getParameterCount()==1 && method.getParameterTypes()[0]==type("ejt$b")) {
                trace.add(label);
                if (label.equals(fail)) throw new ProbeFailure();
                return value;
            }
            if (method.getName().equals("a") && method.getParameterCount()==1 && method.getParameterTypes()[0]==type("ejt$f"))
                return type("ejt$f").getMethod("apply",type("ejt")).invoke(args[0],proxy);
            throw new UnsupportedOperationException("unsupported original probe call:"+method);
        });
    }

    private static Object identityVisitor() throws Exception {
        return Proxy.newProxyInstance(DensityRouterOracle.class.getClassLoader(),new Class<?>[]{type("ejt$f")},(proxy,method,args)->{
            if (method.getName().equals("apply") || method.getName().equals("a")) return args[0];
            if (method.getName().equals("toString")) return "original-oracle-identity-visitor";
            if (method.getName().equals("hashCode")) return System.identityHashCode(proxy);
            if (method.getName().equals("equals")) return proxy==args[0];
            throw new UnsupportedOperationException("unsupported original visitor call:"+method);
        });
    }

    private static JsonArray bitBounds(double min,double max) {
        JsonArray result=new JsonArray(); result.add(bits(min)); result.add(bits(max)); return result;
    }

    private static void exportTrace(PrintWriter output,Object wiring,Object climate,String name,String operation,String requestedFactory,
            double left,double right,double leftMin,double leftMax,double rightMin,double rightMax,
            double argument,double minimum,double maximum,String mapped,String fail) throws Exception {
        for (String view:new String[]{"factory","identity","wiring","climate"}) {
            List<String> trace=new ArrayList<>();
            JsonObject result=node("trace");
            result.addProperty("case",name+"_"+view);
            result.addProperty("view",view);
            result.addProperty("operation",operation);
            Object function;
            if (operation.equals("range_choice")) {
                result.addProperty("input_bits",bits(left));
                result.add("input_bounds",bitBounds(leftMin,leftMax));
                result.addProperty("min_bits",bits(minimum));
                result.addProperty("max_bits",bits(maximum));
                result.addProperty("inside_bits",bits(right));
                result.addProperty("outside_bits",bits(argument));
                result.add("inside_bounds",bitBounds(rightMin,rightMax));
                result.add("outside_bounds",bitBounds(argument,argument));
                function=type("eju").getMethod("a",type("ejt"),double.class,double.class,type("ejt"),type("ejt"))
                    .invoke(null,probe("input",left,leftMin,leftMax,trace,fail),minimum,maximum,
                        probe("inside",right,rightMin,rightMax,trace,fail),probe("outside",argument,argument,argument,trace,fail));
            } else if (operation.equals("mapped")) {
                result.addProperty("input_bits",bits(left));
                result.add("input_bounds",bitBounds(leftMin,leftMax));
                result.addProperty("mapped_operation",mapped);
                Object selected=null;
                for (Object choice:type("eju$k$a").getEnumConstants()) if (((Enum<?>)choice).name().equalsIgnoreCase(mapped)) selected=choice;
                if (selected==null) throw new IllegalStateException("unknown original mapped enum");
                function=type("eju").getMethod("a",type("ejt"),type("eju$k$a"))
                    .invoke(null,probe("input",left,leftMin,leftMax,trace,fail),selected);
            } else {
                String symbol=switch(operation){case "add"->"a";case "mul"->"b";case "min"->"c";case "max"->"d";default->throw new IllegalArgumentException(operation);};
                if (requestedFactory.equals("mul_or_add")) {
                    result.addProperty("input_bits",bits(left));
                    result.add("input_bounds",bitBounds(leftMin,leftMax));
                    result.addProperty("argument_bits",bits(argument));
                    function=type("eju").getMethod(symbol,type("ejt"),type("ejt"))
                        .invoke(null,type("eju").getMethod("a",double.class).invoke(null,argument),
                            probe("input",left,leftMin,leftMax,trace,fail));
                } else {
                    result.addProperty("left_bits",bits(left)); result.addProperty("right_bits",bits(right));
                    result.add("left_bounds",bitBounds(leftMin,leftMax)); result.add("right_bounds",bitBounds(rightMin,rightMax));
                    function=type("eju").getMethod(symbol,type("ejt"),type("ejt"))
                        .invoke(null,probe("left",left,leftMin,leftMax,trace,fail),probe("right",right,rightMin,rightMax,trace,fail));
                }
            }
            result.addProperty("factory_before",function.getClass().getName());
            if (view.equals("identity")) function=map(function,identityVisitor());
            if (view.equals("wiring") || view.equals("climate")) function=map(function,wiring);
            if (view.equals("climate")) function=map(function,climate);
            String actual=switch(function.getClass().getName()){case "eju$a"->"binary";case "eju$n"->"mul_or_add";case "eju$q"->"range_choice";case "eju$k"->"mapped";default->throw new IllegalStateException("unexpected original factory");};
            if (!actual.equals(requestedFactory)) throw new IllegalStateException("actual original factory differs from requested probe");
            result.addProperty("factory",actual);
            result.addProperty("factory_after",function.getClass().getName());
            JsonArray bounds=bounds(function);
            result.addProperty("min_value_bits",bounds.get(0).getAsString());
            result.addProperty("max_value_bits",bounds.get(1).getAsString());
            if (fail!=null) result.addProperty("throw_child",fail);
            try { result.addProperty("result_bits",bits(sample(function,context(new int[]{0,0,0})))); }
            catch (InvocationTargetException error) {
                Throwable cause=error;
                while (cause instanceof InvocationTargetException) cause=cause.getCause();
                if (!(cause instanceof ProbeFailure)) throw error;
                result.addProperty("error","original_probe_failure");
            }
            JsonArray requests=new JsonArray(); for (String child:trace) requests.add(child);
            result.add("trace",requests);
            write(output,result);
        }
    }

    private static void exportTraces(PrintWriter output,Object settings,Object noiseGetter) throws Exception {
        Object randomState=type("eko").getMethod("a",type("ekf"),type("jm"),long.class)
            .invoke(null,settings,noiseGetter,0L);
        var wiringConstructor=type("eko$a").getDeclaredConstructor(type("eko"),long.class,boolean.class);
        wiringConstructor.setAccessible(true);
        Object wiring=wiringConstructor.newInstance(randomState,0L,(Boolean)field(settings,"t"));
        var climateConstructor=type("eko$1").getDeclaredConstructor(type("eko"));
        climateConstructor.setAccessible(true);
        Object climate=climateConstructor.newInstance(randomState);
        for (double zero:new double[]{0.0,-0.0}) {
            exportTrace(output,wiring,climate,"mul_skip_"+bits(zero),"mul","binary",zero,7,-1,1,-2,8,0,0,0,null,"right");
            exportTrace(output,wiring,climate,"mul_or_add_zero_"+bits(zero),"mul","mul_or_add",-3,0,-4,4,0,0,zero,0,0,null,null);
            exportTrace(output,wiring,climate,"mul_or_add_zero_throw_"+bits(zero),"mul","mul_or_add",-3,0,-4,4,0,0,zero,0,0,null,"input");
        }
        for (String operation:new String[]{"add","mul","min","max"}) {
            exportTrace(output,wiring,climate,operation+"_ordinary",operation,"binary",-2,3,-4,4,-4,4,0,0,0,null,null);
            exportTrace(output,wiring,climate,operation+"_throw_left",operation,"binary",-2,3,-4,4,-4,4,0,0,0,null,"left");
            exportTrace(output,wiring,climate,operation+"_throw_right",operation,"binary",-2,3,-4,4,-4,4,0,0,0,null,"right");
            exportTrace(output,wiring,climate,operation+"_zeros",operation,"binary",-0.0,0.0,-0.0,0.0,-0.0,0.0,0,0,0,null,null);
            exportTrace(output,wiring,climate,operation+"_nan_bounds",operation,"binary",2,-1,Double.longBitsToDouble(0x7ff8000000001234L),4,-4,Double.POSITIVE_INFINITY,0,0,0,null,null);
            exportTrace(output,wiring,climate,operation+"_reversed_bounds",operation,"binary",-2,3,4,-4,5,-5,0,0,0,null,null);
        }
        for (String operation:new String[]{"min","max"}) {
            for (double value:new double[]{Math.nextDown(2.0),2.0,Math.nextUp(2.0)}) {
                exportTrace(output,wiring,climate,operation+"_bound_"+bits(value),operation,"binary",value,3,-4,4,2,2,0,0,0,null,null);
                exportTrace(output,wiring,climate,operation+"_bound_throw_"+bits(value),operation,"binary",value,3,-4,4,2,2,0,0,0,null,"right");
            }
        }
        for (double value:new double[]{Math.nextDown(-1.0),-1,Math.nextUp(-1.0),Math.nextDown(1.0),1,Math.nextUp(1.0),Double.NaN})
            exportTrace(output,wiring,climate,"range_"+bits(value),"range_choice","range_choice",value,17,-2,2,16,18,-19,-1,1,null,null);
        exportTrace(output,wiring,climate,"range_throw_input","range_choice","range_choice",0,17,-2,2,16,18,-19,-1,1,null,"input");
        exportTrace(output,wiring,climate,"range_throw_inside","range_choice","range_choice",0,17,-2,2,16,18,-19,-1,1,null,"inside");
        exportTrace(output,wiring,climate,"range_throw_outside","range_choice","range_choice",2,17,-2,2,16,18,-19,-1,1,null,"outside");
        exportTrace(output,wiring,climate,"range_reversed","range_choice","range_choice",0,17,-2,2,16,18,-19,1,-1,null,null);
        for (String mapped:new String[]{"abs","square","cube","half_negative","quarter_negative","squeeze"}) {
            exportTrace(output,wiring,climate,"mapped_"+mapped,"mapped","mapped",-0.0,0,-3,4,0,0,0,0,0,mapped,null);
            exportTrace(output,wiring,climate,"mapped_"+mapped+"_nan","mapped","mapped",Double.longBitsToDouble(0xfff8000000001234L),0,Double.NEGATIVE_INFINITY,Double.POSITIVE_INFINITY,0,0,0,0,0,mapped,null);
            exportTrace(output,wiring,climate,"mapped_"+mapped+"_reversed","mapped","mapped",-2,0,4,-4,0,0,0,0,0,mapped,null);
            exportTrace(output,wiring,climate,"mapped_"+mapped+"_throw","mapped","mapped",-2,0,-3,4,0,0,0,0,0,mapped,"input");
        }
    }

    private static void exportBinding(PrintWriter output, String name, Object settings,
            Object noiseGetter, Map<String,Object> definitions, Map<String,Object> registeredDefinitions,
            List<String> noiseNames,long seed) throws Exception {
        Object randomState = type("eko").getMethod("a",type("ekf"),type("jm"),long.class)
            .invoke(null, settings, noiseGetter, seed);
        Object router = type("eko").getMethod("a").invoke(randomState);
        Object sampler = type("eko").getMethod("b").invoke(randomState);
        Object[] fields = new Object[15];
        Object[] climate = new Object[6];
        for (int i=0; i<15; i++) fields[i] = field(router, String.valueOf((char)('b'+i)));
        for (int i=0; i<6; i++) climate[i] = field(sampler, String.valueOf((char)('a'+i)));
        var constructor = type("eko$a").getDeclaredConstructor(type("eko"),long.class,boolean.class);
        constructor.setAccessible(true);
        Object visitor = constructor.newInstance(randomState, seed, (Boolean)field(settings,"t"));
        Map<String,Object> named = new TreeMap<>();
        for (Map.Entry<String,Object> entry : definitions.entrySet()) named.put(entry.getKey(), map(entry.getValue(),visitor));

        JsonObject binding = record("binding",name,seed);
        JsonObject fieldBounds = new JsonObject();
        JsonObject namedBounds = new JsonObject();
        JsonArray climateBounds = new JsonArray();
        for (int i=0; i<15; i++) fieldBounds.add(ROUTER_FIELDS[i], bounds(fields[i]));
        for (Map.Entry<String,Object> entry : named.entrySet()) namedBounds.add(entry.getKey(),bounds(entry.getValue()));
        for (Object function : climate) climateBounds.add(bounds(function));
        binding.add("field_bounds",fieldBounds);
        binding.add("named_bounds",namedBounds);
        binding.add("climate_bounds",climateBounds);
        binding.add("aquifer_draws",positionalDraws(type("eko").getMethod("d").invoke(randomState)));
        binding.add("ore_draws",positionalDraws(type("eko").getMethod("e").invoke(randomState)));
        JsonObject noiseMaxima=new JsonObject();
        for (String noiseName:noiseNames)
            noiseMaxima.addProperty(noiseName,bits((Double)type("ezo").getMethod("a").invoke(boundNoise(visitor,noiseGetter,noiseName))));
        binding.add("noise_max_values",noiseMaxima);
        write(output,binding);
        if (seed == 0) {
            JsonObject projection = record("projection",name,seed);
            JsonObject fieldProjection = new JsonObject();
            JsonObject namedProjection = new JsonObject();
            JsonArray climateProjection = new JsonArray();
            for (int i=0; i<15; i++) fieldProjection.add(ROUTER_FIELDS[i],density(fields[i]));
            for (Map.Entry<String,Object> entry : named.entrySet()) namedProjection.add(entry.getKey(),density(entry.getValue()));
            for (Object function : climate) climateProjection.add(density(function));
            projection.add("fields",fieldProjection);
            projection.add("named",namedProjection);
            JsonObject registeredProjection=new JsonObject();
            for (Map.Entry<String,Object> entry:registeredDefinitions.entrySet())
                registeredProjection.add(entry.getKey(),density(map(entry.getValue(),visitor)));
            projection.add("registered_named",registeredProjection);
            projection.add("climate",climateProjection);
            write(output,projection);
        }
        for (int[] xyz : positions(settings)) {
            Object context = context(xyz);
            JsonObject samples = record("point",name,seed);
            position(samples,xyz);
            JsonObject fieldSamples = new JsonObject();
            JsonObject namedSamples = new JsonObject();
            for (int i=0; i<15; i++) fieldSamples.addProperty(ROUTER_FIELDS[i],bits(sample(fields[i],context)));
            for (Map.Entry<String,Object> entry : named.entrySet()) namedSamples.addProperty(entry.getKey(),bits(sample(entry.getValue(),context)));
            samples.add("fields",fieldSamples);
            samples.add("named",namedSamples);
            JsonArray values = climateValues(climate,context);
            samples.add("climate_f32",values);
            samples.add("climate_quantized",quantized(values));
            write(output,samples);
        }
        for (int[] quart : new int[][]{{0,0,0},{-1,-1,-1},{-4,16,-5},{1,-16,4},{7500000,16,-7500000},{-7500000,-16,7500000}}) {
            JsonObject sample = record("quart",name,seed);
            position(sample,quart);
            Object target = type("dog$f").getMethod("a",int.class,int.class,int.class)
                .invoke(sampler,quart[0],quart[1],quart[2]);
            JsonArray quantized = new JsonArray();
            for (int i=0; i<6; i++) quantized.add((Long)field(target,String.valueOf((char)('a'+i))));
            sample.add("climate_quantized",quantized);
            sample.add("climate_f32",climateValues(climate,context(new int[]{quart[0]<<2,quart[1]<<2,quart[2]<<2})));
            write(output,sample);
        }
        exportWrapped(output,name,seed,randomState,noiseGetter,visitor);
    }

    private static void requireOrigin(Class<?> clazz,Path expected) throws Exception {
        var source=clazz.getProtectionDomain().getCodeSource();
        if (source==null || !Path.of(source.getLocation().toURI()).toRealPath().equals(expected.toRealPath()))
            throw new IllegalStateException("actual class CodeSource mismatch:"+clazz.getName());
        System.out.println("verified CodeSource class="+clazz.getName()+" origin="+Path.of(source.getLocation().toURI()).toRealPath());
    }

    @SuppressWarnings("unchecked")
    public static void main(String[] args) throws Exception {
        if (args.length != 4) throw new IllegalArgumentException("expected SOURCE_FIXTURE OUTPUT_JSONL INNER_JAR HELPER_CLASSES");
        for (String name:new String[]{"ac","amg","pc","amc","ejt","eju","eko","ezo","ezp","dog","bbb","bdh","jl","jm","amd","ame","mn","bck","ekn","eko$a","eko$1","eju$a","eju$n","eju$q","eju$k","bbb$c","bbb$e"})
            requireOrigin(Class.forName(name,false,DensityRouterOracle.class.getClassLoader()),Path.of(args[2]));
        requireOrigin(DensityRouterOracle.class,Path.of(args[3]));
        type("ac").getMethod("a").invoke(null);
        type("amg").getMethod("a").invoke(null);
        Object lookup = type("pc").getMethod("a").invoke(null);
        ops = (DynamicOps<JsonElement>)type("amc").getMethod("a",DynamicOps.class,type("jn$a"))
            .invoke(null,JsonOps.INSTANCE,lookup);
        direct = (Codec<Object>)type("ejt").getField("b").get(null);
        settingsCodec = (Codec<Object>)type("ekf").getField("a").get(null);
        Object noiseGetter = type("jm$a").getMethod("b",type("amd"))
            .invoke(lookup,type("mn").getField("bd").get(null));
        Object densityGetter = type("jm$a").getMethod("b",type("amd"))
            .invoke(lookup,type("mn").getField("aS").get(null));
        JsonObject registries = JsonParser.parseString(Files.readString(Path.of(args[0]),StandardCharsets.UTF_8))
            .getAsJsonObject().getAsJsonObject("registries");
        Map<String,Object> definitions = new TreeMap<>();
        Map<String,Object> registeredDefinitions = new TreeMap<>();
        for (Map.Entry<String,JsonElement> entry : registries.getAsJsonObject("minecraft:worldgen/density_function")
                .getAsJsonObject("entries").entrySet()) {
            definitions.put(entry.getKey(),direct.parse(ops,entry.getValue()).getOrThrow());
            registeredDefinitions.put(entry.getKey(),registryValue(densityGetter,"aS",entry.getKey()));
        }
        Map<String,Object> settings = new TreeMap<>();
        for (Map.Entry<String,JsonElement> entry : registries.getAsJsonObject("minecraft:worldgen/noise_settings")
                .getAsJsonObject("entries").entrySet()) settings.put(entry.getKey(),settingsCodec.parse(ops,entry.getValue()).getOrThrow());
        List<String> noiseNames=new ArrayList<>(registries.getAsJsonObject("minecraft:worldgen/noise")
            .getAsJsonObject("entries").keySet());
        Collections.sort(noiseNames);
        try (PrintWriter output = new PrintWriter(Files.newBufferedWriter(Path.of(args[1]),StandardCharsets.UTF_8))) {
            for (Map.Entry<String,Object> entry : settings.entrySet()) {
                for (long seed : new long[]{0,1,-1,Long.MIN_VALUE,Long.MAX_VALUE,0x0123456789abcdefL,-0x1234567890abcdeL})
                    exportBinding(output,entry.getKey(),entry.getValue(),noiseGetter,definitions,registeredDefinitions,noiseNames,seed);
            }
            exportQuantize(output);
            exportTraces(output,settings.get("minecraft:overworld"),noiseGetter);
        }
    }
}
