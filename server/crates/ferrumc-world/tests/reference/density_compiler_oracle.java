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
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Optional;
import java.util.TreeMap;

/** Structural projection of actual original codec products; no density formulas. */
final class DensityCompilerOracle {
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
        if (spline.getClass().getName().equals("bbb$c")) {
            JsonObject result = node("constant");
            result.addProperty("value_bits", bits((Float) field(spline, "b")));
            return result;
        }
        if (!spline.getClass().getName().equals("bbb$e")) throw new IllegalStateException("unknown original spline class");
        JsonObject result = node("multipoint");
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
        return result;
    }

    private static JsonObject density(Object density) throws Exception {
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
    private static void write(PrintWriter output, String kind, String id, JsonElement input, JsonElement projection, String status) {
        write(output, kind, id, input, projection, status, input == null ? null : input.toString());
    }
    private static void write(PrintWriter output, String kind, String id, JsonElement input, JsonElement projection, String status, String inputJson) {
        JsonObject record = new JsonObject();
        record.addProperty("kind", kind);
        record.addProperty("id", id);
        if (inputJson != null) record.addProperty("input_json", inputJson);
        if (projection != null) record.add("projection", projection);
        if (status != null) record.addProperty("status", status);
        output.println(GSON.toJson(canonical(record)));
    }
    private static void probe(PrintWriter output, String id, String input, Codec<Object> codec) throws Exception {
        JsonElement json = JsonParser.parseString(input);
        DataResult<Object> decoded = codec.parse(ops, json);
        Optional<Object> result = decoded.result();
        write(output, "probe", id, json, result.isPresent() ? density(result.get()) : null, result.isPresent() ? "ok" : "error", input);
    }

    private static void probes(PrintWriter output) throws Exception {
        for (String value : new String[]{"-0", "0", "-0.0", "0.0", "1000000", "-1000000", "1000000.0000000001"}) probe(output, "constant_" + value, value, direct);
        probe(output, "constant_object", "{\"type\":\"constant\",\"argument\":-0.0}", direct);
        probe(output, "holder_bare_namespace", "\"zero\"", holderHelper);
        probe(output, "holder_explicit_namespace", "\"minecraft:zero\"", holderHelper);
        probe(output, "holder_inline", "-0.0", holderHelper);
        probe(output, "direct_ref_rejected", "\"minecraft:zero\"", direct);
        for (String form : new String[]{"abs", "square", "cube", "half_negative", "quarter_negative", "squeeze", "interpolated", "flat_cache", "cache_2d", "cache_once", "cache_all_in_cell", "blend_density"}) probe(output, "form_" + form, "{\"type\":\"" + form + "\",\"argument\":-0.0}", direct);
        for (String operation : new String[]{"add", "mul", "min", "max"}) {
            probe(output, "form_" + operation, "{\"type\":\"" + operation + "\",\"argument1\":-0.0,\"argument2\":{\"type\":\"blend_alpha\"}}", direct);
            probe(output, "named_" + operation, "{\"type\":\"" + operation + "\",\"argument1\":\"zero\",\"argument2\":{\"type\":\"blend_alpha\"}}", direct);
        }
        for (String form : new String[]{"blend_alpha", "blend_offset", "beardifier", "end_islands"}) probe(output, "form_" + form, "{\"type\":\"" + form + "\"}", direct);
        for (String form : new String[]{"shift", "shift_a", "shift_b"}) probe(output, "form_" + form, "{\"type\":\"" + form + "\",\"argument\":\"offset\"}", direct);
        probe(output, "form_noise", "{\"type\":\"noise\",\"noise\":\"temperature\",\"xz_scale\":-0.0,\"y_scale\":1.25}", direct);
        probe(output, "form_shifted_noise", "{\"type\":\"shifted_noise\",\"noise\":\"temperature\",\"xz_scale\":1,\"y_scale\":0,\"shift_x\":\"zero\",\"shift_y\":-0.0,\"shift_z\":0}", direct);
        probe(output, "form_old_blended_noise", "{\"type\":\"old_blended_noise\",\"xz_scale\":0.001,\"y_scale\":1000,\"xz_factor\":80,\"y_factor\":160,\"smear_scale_multiplier\":8}", direct);
        for (String mapper : new String[]{"type_1", "type_2"}) probe(output, "form_weird_" + mapper, "{\"type\":\"weird_scaled_sampler\",\"noise\":\"temperature\",\"input\":\"zero\",\"rarity_value_mapper\":\"" + mapper + "\"}", direct);
        probe(output, "form_clamp", "{\"type\":\"clamp\",\"input\":-0.0,\"min\":3,\"max\":-2}", direct);
        probe(output, "form_range_choice", "{\"type\":\"range_choice\",\"input\":\"zero\",\"min_inclusive\":3,\"max_exclusive\":-2,\"when_in_range\":-0.0,\"when_out_of_range\":0.0}", direct);
        probe(output, "form_y_clamped_gradient", "{\"type\":\"y_clamped_gradient\",\"from_y\":64,\"to_y\":-64,\"from_value\":-0.0,\"to_value\":0.0}", direct);
        probe(output, "gradient_equal_endpoints", "{\"type\":\"y_clamped_gradient\",\"from_y\":0,\"to_y\":0,\"from_value\":1,\"to_value\":-1}", direct);
        probe(output, "gradient_fractional_integer", "{\"type\":\"y_clamped_gradient\",\"from_y\":1.5,\"to_y\":2.9,\"from_value\":1,\"to_value\":-1}", direct);
        probe(output, "gradient_wrapping_integer", "{\"type\":\"y_clamped_gradient\",\"from_y\":4294967297,\"to_y\":4294967298,\"from_value\":1,\"to_value\":-1}", direct);
        probe(output, "gradient_negative_fraction", "{\"type\":\"y_clamped_gradient\",\"from_y\":-1.9,\"to_y\":-2.9,\"from_value\":1,\"to_value\":-1}", direct);
        probe(output, "gradient_extreme_integer", "{\"type\":\"y_clamped_gradient\",\"from_y\":9223372036854775808,\"to_y\":9223372036854775809,\"from_value\":1,\"to_value\":-1}", direct);
        probe(output, "gradient_huge_exponent", "{\"type\":\"y_clamped_gradient\",\"from_y\":1e309,\"to_y\":1e1000,\"from_value\":1,\"to_value\":-1}", direct);
        probe(output, "gradient_decimal_exponent", "{\"type\":\"y_clamped_gradient\",\"from_y\":4294967297.9,\"to_y\":-4294967298.9,\"from_value\":1,\"to_value\":-1}", direct);
        probe(output, "gradient_exponent_underflow", "{\"type\":\"y_clamped_gradient\",\"from_y\":1e-1000,\"to_y\":-1e-1000,\"from_value\":1,\"to_value\":-1}", direct);
        probe(output, "form_spline_constant", "{\"type\":\"spline\",\"spline\":-0.0}", direct);
        String spline = "{\"type\":\"spline\",\"spline\":{\"coordinate\":%s,\"points\":[{\"location\":%s,\"value\":%s%s}]}}";
        probe(output, "spline_named_decimal", String.format(spline, "\"zero\"", "0.100000001490116119384765625", "-0.0", ",\"derivative\":-0.0"), direct);
        probe(output, "spline_inline_decimal", String.format(spline, "-0.0", "1.0000000596046448", "1.0000000596046448", ",\"derivative\":1.0000000596046448"), direct);
        probe(output, "spline_missing_derivative", String.format(spline, "\"zero\"", "0", "0", ""), direct);
        probe(output, "spline_infinite_float", String.format(spline, "\"zero\"", "1e39", "1e39", ",\"derivative\":1e39"), direct);
        probe(output, "spline_duplicate_signed_zero", "{\"type\":\"spline\",\"spline\":{\"coordinate\":\"zero\",\"points\":[{\"location\":-0.0,\"value\":1,\"derivative\":0},{\"location\":0.0,\"value\":2,\"derivative\":0}]}}", direct);
        probe(output, "spline_reversed_locations", "{\"type\":\"spline\",\"spline\":{\"coordinate\":\"zero\",\"points\":[{\"location\":1,\"value\":1,\"derivative\":0},{\"location\":-1,\"value\":2,\"derivative\":0}]}}", direct);
        probe(output, "spline_empty_points", "{\"type\":\"spline\",\"spline\":{\"coordinate\":\"zero\",\"points\":[]}}", direct);
        probe(output, "clamp_holder_rejected", "{\"type\":\"clamp\",\"input\":\"zero\",\"min\":-1,\"max\":1}", direct);
        probe(output, "noise_missing_scale", "{\"type\":\"noise\",\"noise\":\"temperature\",\"xz_scale\":1}", direct);
        probe(output, "unknown_density", "\"minecraft:missing_density\"", holderHelper);
        probe(output, "wrong_noise_family", "{\"type\":\"shift\",\"argument\":\"zero\"}", direct);
        probe(output, "unknown_type", "{\"type\":\"minecraft:missing_type\"}", direct);
        probe(output, "unknown_field_ignored", "{\"type\":\"constant\",\"argument\":1,\"extra\":true}", direct);
        probe(output, "noise_unrestricted_scale", "{\"type\":\"noise\",\"noise\":\"temperature\",\"xz_scale\":1000001,\"y_scale\":-1000001}", direct);
        probe(output, "noise_infinite_scale", "{\"type\":\"noise\",\"noise\":\"temperature\",\"xz_scale\":1e309,\"y_scale\":0}", direct);
        probe(output, "noise_inline_parameters", "{\"type\":\"noise\",\"noise\":{\"firstOctave\":-7,\"amplitudes\":[1,-0.0]},\"xz_scale\":1,\"y_scale\":1}", direct);
        // Call the actual factory as well as codecs: helper holders can prevent factory lowering.
        Class<?> densityType = type("ejt");
        Object constant = direct.parse(ops, JsonParser.parseString("-0.0")).getOrThrow();
        Object named = holderHelper.parse(ops, JsonParser.parseString("\"zero\"")).getOrThrow();
        Object alpha = direct.parse(ops, JsonParser.parseString("{\"type\":\"blend_alpha\"}")).getOrThrow();
        for (String[] operation : new String[][]{{"add", "a"}, {"mul", "b"}}) {
            Method factory = type("eju").getMethod(operation[1], densityType, densityType);
            write(output, "factory", "factory_" + operation[0] + "_constant_left", null, density(factory.invoke(null, constant, alpha)), "ok");
            write(output, "factory", "factory_" + operation[0] + "_constant_right", null, density(factory.invoke(null, alpha, constant)), "ok");
            write(output, "factory", "factory_" + operation[0] + "_named_left", null, density(factory.invoke(null, named, alpha)), "ok");
        }
    }

    private static void settingsProbes(PrintWriter output, JsonObject original) throws Exception {
        JsonObject template = original.deepCopy();
        JsonObject router = new JsonObject();
        for (String key : ROUTER_FIELDS) router.addProperty(key, 0);
        template.add("noise_router", router);
        template.add("spawn_target", new JsonArray());
        template.add("surface_rule", JsonParser.parseString("{\"type\":\"minecraft:block\",\"result_state\":{\"Name\":\"minecraft:stone\"}}"));
        for (String[] item : new String[][]{
            {"geometry_fractional_min_y", "min_y", "-64.5"},
            {"geometry_fractional_height", "height", "128.9"},
            {"geometry_fractional_horizontal", "size_horizontal", "1.9"},
            {"geometry_fractional_vertical", "size_vertical", "2.9"},
            {"geometry_wrapping_min_y", "min_y", "4294967296"},
            {"geometry_wrapping_height", "height", "4294967424"},
            {"geometry_wrapping_horizontal", "size_horizontal", "4294967297"},
            {"geometry_extreme_min_y", "min_y", "9223372036854775808"}
        }) {
            JsonObject input = template.deepCopy();
            input.getAsJsonObject("noise").add(item[1], JsonParser.parseString(item[2]));
            DataResult<Object> decoded = settingsCodec.parse(ops, input);
            Optional<Object> result = decoded.result();
            write(output, "settings_probe", item[0], input, result.isPresent() ? settings(result.get()) : null, result.isPresent() ? "ok" : "error");
        }
    }

    @SuppressWarnings("unchecked")
    public static void main(String[] args) throws Exception {
        if (args.length != 2) throw new IllegalArgumentException("expected SOURCE_FIXTURE OUTPUT_JSONL");
        type("ac").getMethod("a").invoke(null);
        type("amg").getMethod("a").invoke(null);
        Object lookup = type("pc").getMethod("a").invoke(null);
        Method create = type("amc").getMethod("a", DynamicOps.class, type("jn$a"));
        ops = (DynamicOps<JsonElement>) create.invoke(null, JsonOps.INSTANCE, lookup);
        direct = (Codec<Object>) type("ejt").getField("b").get(null);
        holderHelper = (Codec<Object>) type("ejt").getField("d").get(null);
        settingsCodec = (Codec<Object>) type("ekf").getField("a").get(null);
        JsonObject registries = JsonParser.parseString(Files.readString(Path.of(args[0]), StandardCharsets.UTF_8)).getAsJsonObject().getAsJsonObject("registries");
        try (PrintWriter output = new PrintWriter(Files.newBufferedWriter(Path.of(args[1]), StandardCharsets.UTF_8))) {
            for (String[] family : new String[][]{{"minecraft:worldgen/density_function", "density"}, {"minecraft:worldgen/noise_settings", "router"}}) {
                TreeMap<String, JsonElement> entries = new TreeMap<>();
                registries.getAsJsonObject(family[0]).getAsJsonObject("entries").entrySet().forEach(entry -> entries.put(entry.getKey(), entry.getValue()));
                for (Map.Entry<String, JsonElement> entry : entries.entrySet()) {
                    Codec<Object> codec = family[1].equals("density") ? direct : settingsCodec;
                    Object value = codec.parse(ops, entry.getValue()).getOrThrow();
                    write(output, family[1], entry.getKey(), null, family[1].equals("density") ? density(value) : settings(value), null);
                }
            }
            probes(output);
            settingsProbes(output, registries.getAsJsonObject("minecraft:worldgen/noise_settings").getAsJsonObject("entries").getAsJsonObject("minecraft:end"));
        }
    }
}
