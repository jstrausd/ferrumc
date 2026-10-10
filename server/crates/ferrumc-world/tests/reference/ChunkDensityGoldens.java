package ferrumc.rootgoldens;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.mojang.serialization.JsonOps;
import java.lang.reflect.Field;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import java.util.TreeSet;
import net.minecraft.SharedConstants;
import net.minecraft.core.registries.Registries;
import net.minecraft.data.registries.VanillaRegistries;
import net.minecraft.resources.RegistryOps;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.levelgen.DensityFunction;
import net.minecraft.world.level.levelgen.DensityFunctions;
import net.minecraft.world.level.levelgen.NoiseChunk;
import net.minecraft.world.level.levelgen.NoiseGeneratorSettings;
import net.minecraft.world.level.levelgen.NoiseSettings;
import net.minecraft.world.level.levelgen.RandomState;
import net.minecraft.world.level.levelgen.blending.Blender;

/** Calls actual Paper NoiseChunk constructor material density and its entire driver. */
public final class ChunkDensityGoldens {
    private static Object field(Object owner, String name) throws Exception {
        Field field = owner.getClass().getDeclaredField(name);
        field.setAccessible(true);
        return field.get(owner);
    }

    private static DensityFunction actualMaterialDensity(NoiseChunk chunk) throws Exception {
        Object rules = field(chunk, "blockStateRule");
        Object[] fillers = (Object[]) field(rules, "materialRuleList");
        DensityFunction result = null;
        for (Field field : fillers[0].getClass().getDeclaredFields()) {
            if (DensityFunction.class.isAssignableFrom(field.getType())) {
                if (result != null) throw new IllegalStateException("ambiguous material capture");
                field.setAccessible(true);
                result = (DensityFunction) field.get(fillers[0]);
            }
        }
        if (result == null) throw new IllegalStateException("missing actual material capture");
        return result;
    }

    private static byte[] run(NoiseChunk chunk, NoiseSettings geometry, int originX, int originZ) throws Exception {
        int width = geometry.getCellWidth(), height = geometry.getCellHeight();
        int samples = Math.multiplyExact(256, geometry.height());
        ByteBuffer values = ByteBuffer.allocate(Math.multiplyExact(samples, 8)).order(ByteOrder.BIG_ENDIAN);
        boolean[] written = new boolean[samples];
        DensityFunction density = actualMaterialDensity(chunk);
        chunk.initializeForFirstCellX();
        int count = 0;
        for (int cx = 0; cx < 16 / width; cx++) {
            chunk.advanceCellX(cx);
            for (int cz = 0; cz < 16 / width; cz++) {
                for (int cy = geometry.height() / height - 1; cy >= 0; cy--) {
                    chunk.selectCellYZ(cy, cz);
                    for (int ly = height - 1; ly >= 0; ly--) {
                        int y = geometry.minY() + cy * height + ly;
                        chunk.updateForY(y, (double) ly / height);
                        for (int lx = 0; lx < width; lx++) {
                            int x = originX + cx * width + lx;
                            chunk.updateForX(x, (double) lx / width);
                            for (int lz = 0; lz < width; lz++) {
                                int z = originZ + cz * width + lz;
                                chunk.updateForZ(z, (double) lz / width);
                                double value = density.compute(chunk);
                                if (!Double.isFinite(value)) throw new IllegalStateException("nonfinite actual Paper density");
                                int index = ((y - geometry.minY()) * 16 + x - originX) * 16 + z - originZ;
                                if (index < 0 || index >= samples || written[index]) throw new IllegalStateException("invalid or duplicate output slot");
                                written[index] = true;
                                values.putLong(index * 8, Double.doubleToRawLongBits(value));
                                count++;
                            }
                        }
                    }
                }
            }
            chunk.swapSlices();
        }
        chunk.stopInterpolation();
        if (count != samples) throw new IllegalStateException("incomplete actual driver");
        for (boolean slot : written) if (!slot) throw new IllegalStateException("missing output slot");
        return values.array();
    }

    public static void main(String[] args) throws Exception {
        Path inner = Path.of(args[2]).toRealPath();
        Path output = Path.of(args[1]).toAbsolutePath();
        Path classes = output.resolve("classes");
        if (!Files.isSameFile(Path.of(ChunkDensityGoldens.class.getProtectionDomain().getCodeSource().getLocation().toURI()), classes)) {
            throw new IllegalStateException("wrong fresh helper origin");
        }
        String[] names = {
            "net.minecraft.SharedConstants", "net.minecraft.server.Bootstrap", "net.minecraft.data.registries.VanillaRegistries",
            "net.minecraft.util.Mth", "net.minecraft.world.level.levelgen.NoiseChunk",
            "net.minecraft.world.level.levelgen.NoiseChunk$NoiseInterpolator", "net.minecraft.world.level.levelgen.NoiseChunk$FlatCache",
            "net.minecraft.world.level.levelgen.NoiseChunk$Cache2D", "net.minecraft.world.level.levelgen.NoiseChunk$CacheOnce",
            "net.minecraft.world.level.levelgen.NoiseChunk$CacheAllInCell", "net.minecraft.world.level.levelgen.DensityFunction",
            "net.minecraft.world.level.levelgen.DensityFunctions", "net.minecraft.world.level.levelgen.DensityFunctions$EndIslandDensityFunction",
            "net.minecraft.world.level.levelgen.RandomState", "net.minecraft.world.level.levelgen.NoiseGeneratorSettings",
            "net.minecraft.world.level.levelgen.NoiseSettings", "net.minecraft.world.level.levelgen.NoiseRouter",
            "net.minecraft.world.level.levelgen.blending.Blender"
        };
        for (String name : names) {
            Class<?> type = Class.forName(name, false, ChunkDensityGoldens.class.getClassLoader());
            if (!Files.isSameFile(Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()), inner)) {
                throw new IllegalStateException("wrong original origin " + name);
            }
        }
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        var lookup = VanillaRegistries.createLookup();
        var ops = RegistryOps.create(JsonOps.INSTANCE, lookup);
        var definitions = JsonParser.parseString(Files.readString(Path.of(args[0]))).getAsJsonObject()
                .getAsJsonObject("registries").getAsJsonObject("minecraft:worldgen/noise_settings").getAsJsonObject("entries");
        Class<?> beardClass = Class.forName("net.minecraft.world.level.levelgen.DensityFunctions$BeardifierMarker");
        Field beardField = beardClass.getDeclaredField("INSTANCE");
        beardField.setAccessible(true);
        var beard = (DensityFunctions.BeardifierOrMarker) beardField.get(null);
        int[][] near = {{0,0},{1,0},{0,1},{-1,0},{0,-1},{-1,-1}};
        int[][] far = {{-32769,0},{32768,0},{0,-32769},{0,32768},{-1875000,-1875000},{1874999,1874999}};
        long[] nearSeeds = {0, 0x0123456789abcdefL};
        long[] farSeeds = {0,1,-1,Long.MIN_VALUE,Long.MAX_VALUE,0x0123456789abcdefL,-0x1234567890abcdeL};
        List<String> rows = new ArrayList<>();
        long offset = 0;
        try (var binary = Files.newOutputStream(output.resolve("material_density.bits"))) {
            for (String name : new TreeSet<>(definitions.keySet())) {
                NoiseGeneratorSettings settings = NoiseGeneratorSettings.CODEC.parse(ops, definitions.get(name)).getOrThrow().value();
                NoiseSettings geometry = settings.noiseSettings();
                for (boolean distant : new boolean[]{false, true}) {
                    if (distant && !name.equals("minecraft:end")) continue;
                    for (long seed : distant ? farSeeds : nearSeeds) {
                        RandomState random = RandomState.create(settings, lookup.lookupOrThrow(Registries.NOISE), seed);
                        for (int[] position : distant ? far : near) {
                            int x = Math.multiplyExact(position[0], 16), z = Math.multiplyExact(position[1], 16);
                            if (x < -30000000 || z < -30000000 || (long)x + 16 > 30000000 || (long)z + 16 > 30000000) {
                                throw new IllegalStateException("fixture outside approved chunk domain");
                            }
                            NoiseChunk chunk = new NoiseChunk(16 / geometry.getCellWidth(), random, x, z, geometry, beard, settings,
                                    (bx,by,bz)->{throw new IllegalStateException("unexpected fluid call in density-only oracle");}, Blender.empty());
                            byte[] values = run(chunk, geometry, x, z);
                            JsonObject row = new JsonObject();
                            row.addProperty("settings", name); row.addProperty("seed", Long.toString(seed));
                            row.addProperty("chunk_x", position[0]); row.addProperty("chunk_z", position[1]); row.addProperty("far_end", distant);
                            row.addProperty("min_y", geometry.minY()); row.addProperty("height", geometry.height());
                            row.addProperty("cell_width", geometry.getCellWidth()); row.addProperty("cell_height", geometry.getCellHeight());
                            row.addProperty("offset", offset); row.addProperty("bytes", values.length);
                            row.addProperty("sha256", HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(values)));
                            binary.write(values); offset = Math.addExact(offset, values.length);
                            rows.add(row.toString());
                        }
                    }
                }
            }
        }
        Files.write(output.resolve("chunks.jsonl"), rows);
        System.out.println("actual Paper complete density chunks=" + rows.size() + " bytes=" + offset);
    }
}
