package ferrumc.rootbiomes;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Comparator;
import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.biome.Climate;
import net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterList;

/** Exports actual builtin parameter order and quantized values; no biome formulas. */
public final class BiomeParameterCensus {
    public static void main(String[] args) throws Exception {
        Path output = Path.of(args[0]).toRealPath();
        Path inner = Path.of(args[1]).toRealPath();
        Path classes = output.resolve("classes").toRealPath();
        if (!Files.isSameFile(Path.of(BiomeParameterCensus.class.getProtectionDomain().getCodeSource().getLocation().toURI()), classes)) {
            throw new IllegalStateException("wrong helper origin");
        }
        String[] names = {
            "net.minecraft.SharedConstants", "net.minecraft.server.Bootstrap", "net.minecraft.util.Mth",
            "net.minecraft.resources.ResourceKey", "net.minecraft.resources.ResourceLocation",
            "net.minecraft.world.level.biome.Biomes", "net.minecraft.world.level.biome.OverworldBiomeBuilder",
            "net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterList",
            "net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterList$Preset",
            "net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterList$Preset$1",
            "net.minecraft.world.level.biome.MultiNoiseBiomeSourceParameterList$Preset$2",
            "net.minecraft.world.level.biome.Climate", "net.minecraft.world.level.biome.Climate$Parameter",
            "net.minecraft.world.level.biome.Climate$ParameterPoint", "net.minecraft.world.level.biome.Climate$ParameterList",
            "net.minecraft.world.level.biome.Climate$RTree", "net.minecraft.world.level.biome.Climate$RTree$Node",
            "net.minecraft.world.level.biome.Climate$RTree$Leaf", "net.minecraft.world.level.biome.Climate$RTree$SubTree"
        };
        for (String name : names) {
            Class<?> type = Class.forName(name, false, BiomeParameterCensus.class.getClassLoader());
            if (!Files.isSameFile(Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()), inner)) {
                throw new IllegalStateException("wrong original class origin " + name);
            }
        }
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        if (SharedConstants.debugGenerateSquareTerrainWithoutNoise) throw new IllegalStateException("debug biome preset enabled");
        var presets = MultiNoiseBiomeSourceParameterList.knownPresets();
        JsonObject result = new JsonObject();
        for (var preset : presets.keySet().stream().sorted(Comparator.comparing(value -> value.id().toString())).toList()) {
            JsonArray rows = new JsonArray();
            for (var pair : presets.get(preset).values()) {
                var point = pair.getFirst();
                JsonObject row = new JsonObject();
                row.addProperty("index", rows.size());
                row.addProperty("biome", pair.getSecond().location().toString());
                JsonArray axes = new JsonArray();
                for (Climate.Parameter axis : new Climate.Parameter[] {point.temperature(), point.humidity(), point.continentalness(), point.erosion(), point.depth(), point.weirdness()}) {
                    JsonArray range = new JsonArray();
                    range.add(axis.min());
                    range.add(axis.max());
                    axes.add(range);
                }
                row.add("axes", axes);
                row.addProperty("offset", point.offset());
                rows.add(row);
            }
            result.add(preset.id().toString(), rows);
        }
        Files.writeString(output.resolve("parameters.json"), result.toString() + "\n");
        System.out.println("Verified " + names.length + " actual class origins and fresh helper origin; exported " + presets.size() + " presets");
    }
}
