package ferrumc.rootselector;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import com.mojang.datafixers.util.Pair;
import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;
import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.biome.Climate;
import net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterList;

/** Invokes original selectors and exports original tree fields; no search formulas. */
public final class BiomeSelectorOracle {
    private static Object field(Object object, String name) throws Exception {
        Class<?> type = object.getClass();
        while (type != null) {
            try {
                Field f = type.getDeclaredField(name);
                f.setAccessible(true);
                return f.get(object);
            } catch (NoSuchFieldException missing) {
                type = type.getSuperclass();
            }
        }
        throw new NoSuchFieldException(name);
    }

    private static void cold(Climate.ParameterList<?> list) throws Exception {
        ((ThreadLocal<?>)field(field(list, "index"), "lastResult")).remove();
    }

    private static int node(Object object, JsonArray nodes) throws Exception {
        int id = nodes.size();
        JsonObject row = new JsonObject();
        row.addProperty("index", id);
        nodes.add(row);
        JsonArray ranges = new JsonArray();
        for (Climate.Parameter parameter : (Climate.Parameter[])field(object, "parameterSpace")) {
            JsonArray range = new JsonArray();
            range.add(parameter.min());
            range.add(parameter.max());
            ranges.add(range);
        }
        row.add("axes", ranges);
        if (object.getClass().getSimpleName().equals("Leaf")) {
            row.addProperty("leaf", (Integer)field(object, "value"));
        } else {
            JsonArray children = new JsonArray();
            for (Object child : (Object[])field(object, "children")) children.add(node(child, nodes));
            row.add("children", children);
        }
        return id;
    }

    private static Climate.TargetPoint target(long[] q) {
        return new Climate.TargetPoint(q[0], q[1], q[2], q[3], q[4], q[5]);
    }

    private static long[] targetValues(Climate.TargetPoint point) {
        return new long[] {point.temperature(), point.humidity(), point.continentalness(), point.erosion(), point.depth(), point.weirdness()};
    }

    private static JsonObject query(String group, int sequence, long[] q, Climate.ParameterList<Integer> list) {
        JsonObject row = new JsonObject();
        row.addProperty("group", group);
        row.addProperty("sequence", sequence);
        JsonArray axes = new JsonArray();
        for (long value : q) axes.add(value);
        row.add("target", axes);
        Climate.TargetPoint target = target(q);
        try {
            row.addProperty("indexed", list.findValue(target));
        } catch (RuntimeException error) {
            row.addProperty("indexed_error", error.getClass().getName());
        }
        row.addProperty("brute_force", list.findValueBruteForce(target));
        return row;
    }

    private static List<long[]> probes(List<Pair<Climate.ParameterPoint, Integer>> values) {
        List<long[]> probes = new ArrayList<>();
        int selected = Math.min(32, values.size());
        for (int pick = 0; pick < selected; pick++) {
            Climate.ParameterPoint point = values.get(pick * (values.size() - 1) / Math.max(1, selected - 1)).getFirst();
            Climate.Parameter[] axes = {point.temperature(), point.humidity(), point.continentalness(), point.erosion(), point.depth(), point.weirdness()};
            long[] midpoint = new long[6];
            for (int axis = 0; axis < 6; axis++) midpoint[axis] = (axes[axis].min() + axes[axis].max()) / 2;
            probes.add(midpoint.clone());
            for (int axis = 0; axis < 6; axis++) {
                for (long boundary : new long[] {axes[axis].min() - 1, axes[axis].min(), axes[axis].max(), axes[axis].max() + 1}) {
                    long[] q = midpoint.clone();
                    q[axis] = boundary;
                    probes.add(q);
                }
            }
        }
        for (int bits : new int[] {0, 0x80000000, 0x3f000000, 0xbf000000, 0x3effffff, 0x3f000001, 0x3f800000, 0xbf800000, 0x40000000, 0xc0000000}) {
            float value = Float.intBitsToFloat(bits);
            for (int axis = 0; axis < 6; axis++) {
                float[] q = new float[6];
                q[axis] = value;
                probes.add(targetValues(Climate.target(q[0], q[1], q[2], q[3], q[4], q[5])));
            }
        }
        for (long value : new long[] {Long.MIN_VALUE, Long.MAX_VALUE, -40000, 40000}) {
            for (int axis = 0; axis < 6; axis++) {
                long[] q = new long[6];
                q[axis] = value;
                probes.add(q);
            }
        }
        return probes;
    }

    public static void main(String[] args) throws Exception {
        Path output = Path.of(args[0]).toRealPath();
        Path inner = Path.of(args[1]).toRealPath();
        if (!Files.isSameFile(Path.of(BiomeSelectorOracle.class.getProtectionDomain().getCodeSource().getLocation().toURI()), output.resolve("classes").toRealPath())) throw new IllegalStateException("wrong helper origin");
        String[] originals = {
            "net.minecraft.SharedConstants", "net.minecraft.server.Bootstrap", "net.minecraft.util.Mth",
            "net.minecraft.resources.ResourceKey", "net.minecraft.resources.ResourceLocation",
            "net.minecraft.world.level.biome.Biomes", "net.minecraft.world.level.biome.OverworldBiomeBuilder",
            "net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterList",
            "net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterList$Preset",
            "net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterList$Preset$1",
            "net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterList$Preset$2",
            "net.minecraft.world.level.biome.Climate", "net.minecraft.world.level.biome.Climate$TargetPoint",
            "net.minecraft.world.level.biome.Climate$Parameter", "net.minecraft.world.level.biome.Climate$ParameterPoint",
            "net.minecraft.world.level.biome.Climate$ParameterList", "net.minecraft.world.level.biome.Climate$RTree",
            "net.minecraft.world.level.biome.Climate$RTree$Node", "net.minecraft.world.level.biome.Climate$RTree$Leaf",
            "net.minecraft.world.level.biome.Climate$RTree$SubTree"
        };
        for (String name : originals) {
            Class<?> type = Class.forName(name, false, BiomeSelectorOracle.class.getClassLoader());
            if (!Files.isSameFile(Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()), inner)) throw new IllegalStateException("wrong original origin " + name);
        }
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        if (SharedConstants.debugGenerateSquareTerrainWithoutNoise) throw new IllegalStateException("debug biome preset enabled");
        var presets = MultiNoiseBiomeSourceParameterList.knownPresets();
        JsonObject trees = new JsonObject();
        StringBuilder records = new StringBuilder();
        for (var preset : presets.keySet().stream().sorted(Comparator.comparing(value -> value.id().toString())).toList()) {
            List<Pair<Climate.ParameterPoint, Integer>> values = new ArrayList<>();
            for (var pair : presets.get(preset).values()) values.add(Pair.of(pair.getFirst(), values.size()));
            Climate.ParameterList<Integer> list = new Climate.ParameterList<>(values);
            JsonArray nodes = new JsonArray();
            node(field(field(list, "index"), "root"), nodes);
            trees.add(preset.id().toString(), nodes);
            List<long[]> probes = probes(values);
            int sequence = 0;
            for (long[] q : probes) {
                cold(list);
                JsonObject row = query("cold", sequence++, q, list);
                row.addProperty("preset", preset.id().toString());
                records.append(row).append('\n');
            }
            for (boolean reverse : new boolean[] {false, true}) {
                List<long[]> ordered = new ArrayList<>(probes);
                if (reverse) Collections.reverse(ordered);
                cold(list);
                sequence = 0;
                for (long[] q : ordered) {
                    JsonObject row = query(reverse ? "reverse" : "forward", sequence++, q, list);
                    row.addProperty("preset", preset.id().toString());
                    records.append(row).append('\n');
                }
            }
        }
        Climate.Parameter zero = new Climate.Parameter(0, 0);
        List<Pair<Climate.ParameterPoint, Integer>> ties = List.of(
            Pair.of(new Climate.ParameterPoint(new Climate.Parameter(-10000, -10000), zero, zero, zero, zero, zero, 0), 0),
            Pair.of(new Climate.ParameterPoint(new Climate.Parameter(10000, 10000), zero, zero, zero, zero, zero, 0), 1));
        Climate.ParameterList<Integer> tieList = new Climate.ParameterList<>(ties);
        for (String group : new String[] {"tie-forward", "tie-reverse"}) {
            cold(tieList);
            int sequence = 0;
            for (long temperature : group.equals("tie-forward") ? new long[] {-10000, 0, 10000, 0} : new long[] {10000, 0, -10000, 0}) {
                JsonObject row = query(group, sequence++, new long[] {temperature, 0, 0, 0, 0, 0}, tieList);
                row.addProperty("preset", "declared-two-point-tie");
                records.append(row).append('\n');
            }
        }
        Files.writeString(output.resolve("trees.json"), trees + "\n");
        Files.writeString(output.resolve("queries.jsonl"), records.toString());
        System.out.println("Verified " + originals.length + " actual original origins and fresh helper; exported original trees and selector results");
    }
}
