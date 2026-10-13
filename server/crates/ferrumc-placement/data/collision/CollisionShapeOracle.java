import java.io.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;
import net.minecraft.SharedConstants;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.Bootstrap;
import net.minecraft.world.level.EmptyBlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.Property;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.BooleanOp;
import net.minecraft.world.entity.EntityDimensions;

/** Exports genuine Paper 1.21.8 Build 60 placement collision boxes; no substitute methods. */
public final class CollisionShapeOracle {
    static void require(boolean condition, String label) { if (!condition) throw new IllegalStateException(label); }
    static String hash(Path path) throws Exception { return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(path))); }
    static void original(Class<?> type, Path inner) throws Exception {
        Path actual = Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
        require(Files.isSameFile(actual, inner), "unexpected original class origin: " + type.getName());
    }
    @SuppressWarnings("unchecked")
    static <T extends Comparable<T>> String propertyValue(Property<T> property, Comparable<?> value) { return property.getName((T)value); }
    static String boxes(VoxelShape shape) {
        List<String> out = new ArrayList<>();
        for (AABB box : shape.toAabbs()) {
            out.add(box.minX + "," + box.minY + "," + box.minZ + "," + box.maxX + "," + box.maxY + "," + box.maxZ);
        }
        Collections.sort(out);
        return String.join(";", out);
    }
    public static void main(String[] args) throws Exception {
        require(args.length == 2, "inner-jar output-tsv");
        Path inner = Path.of(args[0]).toRealPath();
        require(hash(inner).equals("c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507"), "wrong pinned Paper inner");
        for (Class<?> type : List.of(SharedConstants.class, Bootstrap.class, BlockState.class, Block.class, CollisionContext.class, VoxelShape.class, EmptyBlockGetter.class, AABB.class, Shapes.class, EntityDimensions.class)) original(type, inner);
        SharedConstants.tryDetectVersion(); Bootstrap.bootStrap();
        require(SharedConstants.getCurrentVersion().name().equals("1.21.8") && SharedConstants.getCurrentVersion().protocolVersion() == 772, "wrong release");
        CollisionContext placement = CollisionContext.placementContext(null);
        require(placement.isPlacement(), "actual placement context"); original(placement.getClass(), inner);
        try (BufferedWriter output = Files.newBufferedWriter(Path.of(args[1]));
             BufferedWriter probes = Files.newBufferedWriter(Path.of(args[1]).resolveSibling("probes.tsv"))) {
            probes.write("id\tcase\tpx\tpy\tpz\tx\ty\tz\tintersects\n");
            output.write("id\tblock\tproperties\tclass\tdynamic\toffset\tplacement_boxes\tempty_boxes\tshifted_boxes\n");
            int count = 0;
            for (BlockState state : Block.BLOCK_STATE_REGISTRY) {
                original(state.getBlock().getClass(), inner);
                String name = BuiltInRegistries.BLOCK.getKey(state.getBlock()).toString();
                TreeMap<String, String> properties = new TreeMap<>();
                state.getValues().forEach((property, value) -> properties.put(property.getName(), propertyValue(property, value)));
                VoxelShape shape = state.getCollisionShape(EmptyBlockGetter.INSTANCE, BlockPos.ZERO, placement);
                VoxelShape empty = state.getCollisionShape(EmptyBlockGetter.INSTANCE, BlockPos.ZERO, CollisionContext.empty());
                VoxelShape shifted = state.getCollisionShape(EmptyBlockGetter.INSTANCE, new BlockPos(3, 0, -7), placement);
                output.write(Block.getId(state) + "\t" + name + "\t" + properties + "\t" + state.getBlock().getClass().getName() + "\t" + state.getBlock().hasDynamicShape() + "\t" + state.hasOffsetFunction() + "\t" + boxes(shape) + "\t" + boxes(empty) + "\t" + boxes(shifted) + "\n");
                double half = (double)(0.6F / 2.0F);
                double[][] feet = {
                    {0.5, 0.0, 0.5}, {1.0 + half, 0.0, 0.5}, {0.5, 0.5, 0.5},
                    {0.5, 1.0, 0.5}, {0.5, 1.5, 0.5}, {1.0 + half - 5.0E-8, 0.0, 0.5},
                    {1.0 + half - 2.0E-7, 0.0, 0.5}, {-half, 0.0, 0.5}
                };
                EntityDimensions dimensions = EntityDimensions.scalable(0.6F, 1.8F);
                BlockPos[] origins = {BlockPos.ZERO, new BlockPos(10, 64, -7)};
                for (int originIndex = 0; originIndex < origins.length; originIndex++) {
                    BlockPos origin = origins[originIndex];
                    VoxelShape translated = state.getCollisionShape(EmptyBlockGetter.INSTANCE, origin, placement)
                        .move(origin.getX(), origin.getY(), origin.getZ());
                    for (int i = 0; i < feet.length; i++) {
                        double x = origin.getX() + feet[i][0], y = origin.getY() + feet[i][1], z = origin.getZ() + feet[i][2];
                        AABB body = dimensions.makeBoundingBox(x, y, z);
                        boolean collision = Shapes.joinIsNotEmpty(translated, Shapes.create(body), BooleanOp.AND);
                        probes.write(Block.getId(state) + "\t" + (originIndex * feet.length + i) + "\t" + origin.getX() + "\t" + origin.getY() + "\t" + origin.getZ() + "\t" + x + "\t" + y + "\t" + z + "\t" + collision + "\n");
                    }
                }
                count++;
            }
            System.out.println("COLLISION_ORACLE_DONE states=" + count + " inner=" + hash(inner));
        }
    }
}
