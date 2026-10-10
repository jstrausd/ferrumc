package net04;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import io.netty.buffer.ByteBuf;
import io.netty.buffer.Unpooled;
import java.nio.file.Files;
import java.nio.file.Path;
import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HexFormat;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.core.component.DataComponents;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.network.FriendlyByteBuf;
import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.Packet;
import net.minecraft.network.protocol.game.*;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.server.MinecraftServer;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.component.CustomData;
import net.minecraft.world.phys.Vec3;
import org.bukkit.Bukkit;
import org.bukkit.plugin.java.JavaPlugin;

/** Calls only the actual pinned Paper packet/metadata codecs after server init. */
public final class ItemEntityWirePlugin extends JavaPlugin {
    private static final HexFormat HEX = HexFormat.of();
    private final List<String> rows = new ArrayList<>();
    private final Map<String, Object> evidence = new LinkedHashMap<>();
    private final Map<String, String> origins = new LinkedHashMap<>();

    @Override public void onEnable() {
        try {
            export();
            Files.writeString(Path.of("SUCCESS"), "actual-paper-packet-codecs\n");
        } catch (Throwable error) {
            getLogger().log(java.util.logging.Level.SEVERE, "Reference export failed", error);
            try { Files.writeString(Path.of("FAILED"), error.toString() + "\n"); }
            catch (java.io.IOException ignored) { }
        } finally {
            Bukkit.getScheduler().runTask(this, Bukkit::shutdown);
        }
    }

    private void origin(Class<?> type) throws Exception {
        Path actual = Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
        Path expected = Path.of(System.getProperty("net04.paper.inner")).toRealPath();
        if (!Files.isSameFile(actual, expected)) {
            throw new IllegalStateException("unexpected original class origin: " + type + " -> " + actual);
        }
        origins.put(type.getName(), actual.toString());
    }

    private RegistryFriendlyByteBuf registryBuffer(byte[] bytes) {
        return new RegistryFriendlyByteBuf(Unpooled.buffer().writeBytes(bytes), MinecraftServer.getServer().registryAccess());
    }

    private byte[] encode(Packet<? super ClientGamePacketListener> packet) {
        ByteBuf buf = Unpooled.buffer();
        try {
            GameProtocols.CLIENTBOUND_TEMPLATE
                .bind(RegistryFriendlyByteBuf.decorator(MinecraftServer.getServer().registryAccess()))
                .codec().encode(buf, packet);
            byte[] result = new byte[buf.readableBytes()];
            buf.readBytes(result);
            return result;
        } finally { buf.release(); }
    }

    private <B extends ByteBuf, P> byte[] body(StreamCodec<B, P> codec, B buf, P packet) {
        try {
            codec.encode(buf, packet);
            byte[] result = new byte[buf.readableBytes()];
            buf.readBytes(result);
            return result;
        } finally { buf.release(); }
    }

    private void verifyBody(byte[] full, byte[] body, int expectedId) {
        FriendlyByteBuf buf = new FriendlyByteBuf(Unpooled.wrappedBuffer(full));
        try {
            if (buf.readVarInt() != expectedId) throw new IllegalStateException("original dispatch id changed");
            byte[] actual = new byte[buf.readableBytes()]; buf.readBytes(actual);
            if (!Arrays.equals(actual, body)) throw new IllegalStateException("dispatch/body mismatch");
        } finally { buf.release(); }
    }

    private void metadata(String name, int id, List<SynchedEntityData.DataValue<?>> values) {
        var packet = new ClientboundSetEntityDataPacket(id, values);
        byte[] full = encode(packet);
        byte[] body = body(ClientboundSetEntityDataPacket.STREAM_CODEC, registryBuffer(new byte[0]), packet);
        verifyBody(full, body, 0x5c);
        var buf = registryBuffer(body);
        try {
            if (buf.readVarInt() != id) throw new IllegalStateException("entity id changed");
            byte[] tail = new byte[buf.readableBytes()]; buf.readBytes(tail);
            rows.add(name + "\tmetadata\t" + id + "\t0\t0\t0\t" + HEX.formatHex(tail) + "\t" + HEX.formatHex(full));
        } finally { buf.release(); }
        var decode = registryBuffer(body);
        try {
            var actual = ClientboundSetEntityDataPacket.STREAM_CODEC.decode(decode);
            if (actual.id() != id || actual.packedItems().size() != values.size() || decode.readableBytes() != 0)
                throw new IllegalStateException("original metadata decode mismatch");
        } finally { decode.release(); }
    }

    private void motion(String name, ClientboundSetEntityMotionPacket packet) throws Exception {
        byte[] full = encode(packet);
        byte[] body = body(ClientboundSetEntityMotionPacket.STREAM_CODEC, new FriendlyByteBuf(Unpooled.buffer()), packet);
        verifyBody(full, body, 0x5e);
        int[] values = new int[3];
        String[] names = {"xa", "ya", "za"};
        for (int i = 0; i < names.length; i++) {
            Field field = ClientboundSetEntityMotionPacket.class.getDeclaredField(names[i]);
            field.setAccessible(true); values[i] = field.getInt(packet);
        }
        rows.add(name + "\tmotion\t" + packet.getId() + "\t" + values[0] + "\t" + values[1] + "\t" + values[2] + "\t-\t" + HEX.formatHex(full));
    }

    private void take(String name, int item, int collector, int count) {
        var packet = new ClientboundTakeItemEntityPacket(item, collector, count);
        byte[] full = encode(packet);
        byte[] body = body(ClientboundTakeItemEntityPacket.STREAM_CODEC, new FriendlyByteBuf(Unpooled.buffer()), packet);
        verifyBody(full, body, 0x75);
        var decode = new FriendlyByteBuf(Unpooled.wrappedBuffer(body));
        try {
            var actual = ClientboundTakeItemEntityPacket.STREAM_CODEC.decode(decode);
            if (actual.getItemId() != item || actual.getPlayerId() != collector || actual.getAmount() != count || decode.readableBytes() != 0)
                throw new IllegalStateException("original take decode mismatch");
        } finally { decode.release(); }
        rows.add(name + "\ttake\t" + item + "\t" + collector + "\t" + count + "\t0\t-\t" + HEX.formatHex(full));
    }

    @SuppressWarnings("unchecked") private void export() throws Exception {
        for (Class<?> type : List.of(ClientboundSetEntityDataPacket.class, ClientboundSetEntityMotionPacket.class,
             ClientboundTakeItemEntityPacket.class, GameProtocols.class, ItemEntity.class, ItemStack.class,
             SynchedEntityData.class, EntityDataSerializers.class, net.minecraft.core.component.DataComponentPatch.class,
             io.papermc.paper.util.sanitizer.ItemComponentSanitizer.class,
             io.papermc.paper.util.sanitizer.ItemObfuscationSession.class,
             io.papermc.paper.util.sanitizer.OversizedItemComponentSanitizer.class)) origin(type);
        evidence.put("original_class_origins", origins);
        try (var stream = getClass().getResourceAsStream("/net04/ItemEntityWirePlugin.class")) {
            String classSha = HEX.formatHex(java.security.MessageDigest.getInstance("SHA-256").digest(stream.readAllBytes()));
            if (!classSha.equals(System.getProperty("net04.helper.class.sha256"))) throw new IllegalStateException("helper class drift");
            evidence.put("helper_class_sha256", classSha);
        }
        evidence.put("helper_origin", Path.of(getClass().getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().toString());
        evidence.put("paper_version", Bukkit.getVersion());
        var settings = io.papermc.paper.configuration.GlobalConfiguration.get().anticheat.obfuscation.items;
        evidence.put("item_obfuscation_enabled", settings.enableItemObfuscation);
        evidence.put("item_obfuscation_level", settings.binding.level.toString());
        if (settings.enableItemObfuscation) throw new IllegalStateException("unexpected reference obfuscation config");
        Field accessorField = ItemEntity.class.getDeclaredField("DATA_ITEM"); accessorField.setAccessible(true);
        var itemAccessor = (EntityDataAccessor<ItemStack>) accessorField.get(null);
        evidence.put("item_entity_metadata_index", itemAccessor.id());
        evidence.put("item_stack_serializer_id", EntityDataSerializers.getSerializedId(itemAccessor.serializer()));
        rows.add("case\tkind\tentity_id\tv1\tv2\tv3\tmetadata_hex\tpacket_hex");
        int[] ids = {0, 127, 128, 255, 16384, Integer.MAX_VALUE, Integer.MIN_VALUE, -1};
        for (int i = 0; i < ids.length; i++) {
            metadata("metadata_empty_" + i, ids[i], List.of());
            take("take_all_" + i, ids[i], ids[(i+1)%ids.length], ids[(i+2)%ids.length]);
        }
        metadata("metadata_primitives", 128, List.of(
            new SynchedEntityData.DataValue<>((int)0, EntityDataSerializers.BYTE, (byte)-1),
            new SynchedEntityData.DataValue<>((int)1, EntityDataSerializers.INT, Integer.MIN_VALUE),
            new SynchedEntityData.DataValue<>((int)2, EntityDataSerializers.FLOAT, -1.25f),
            new SynchedEntityData.DataValue<>((int)3, EntityDataSerializers.STRING, "stone ä")));
        metadata("metadata_item_empty", 1, List.of(SynchedEntityData.DataValue.create(itemAccessor, ItemStack.EMPTY)));
        metadata("metadata_item_stone_1", 255, List.of(SynchedEntityData.DataValue.create(itemAccessor, new ItemStack(Items.STONE, 1))));
        metadata("metadata_item_stone_64", 16384, List.of(SynchedEntityData.DataValue.create(itemAccessor, new ItemStack(Items.STONE, 64))));
        var custom = new ItemStack(Items.STONE, 3);
        var nbt = new CompoundTag(); nbt.putByteArray("bytes", new byte[]{0, -1, 1, -1});
        custom.set(DataComponents.CUSTOM_DATA, CustomData.of(nbt));
        custom.set(DataComponents.CUSTOM_NAME, net.minecraft.network.chat.Component.literal("wire ä"));
        custom.set(DataComponents.REPAIR_COST, 128);
        metadata("metadata_item_components", -1, List.of(SynchedEntityData.DataValue.create(itemAccessor, custom)));
        var removed = new ItemStack(Items.STONE, 2); removed.remove(DataComponents.MAX_STACK_SIZE);
        metadata("metadata_item_removed", 128, List.of(SynchedEntityData.DataValue.create(itemAccessor, removed)));
        double[][] motions = {{0.0, -0.0, 0.0}, {0.0001249, -0.0001249, 0.000125},
            {3.9, -3.9, 5.0}, {Math.nextDown(3.9), Math.nextUp(-3.9), -5.0},
            {Math.nextUp(3.9), Math.nextDown(-3.9), 1.23456789}};
        List<List<String>> motionInputs = new ArrayList<>();
        for (int i = 0; i < motions.length; i++) {
            double[] v = motions[i];
            motionInputs.add(List.of(Double.toHexString(v[0]), Double.toHexString(v[1]), Double.toHexString(v[2])));
            motion("motion_constructor_" + i, new ClientboundSetEntityMotionPacket(ids[i], new Vec3(v[0], v[1], v[2])));
        }
        evidence.put("motion_constructor_inputs_hexfloat", motionInputs);
        // Explicit wire controls are decoded by the ORIGINAL codec, not quantized.
        for (int i = 0; i < 3; i++) {
            short[][] controls = {{Short.MIN_VALUE, Short.MAX_VALUE, -1}, {0, 1, Short.MIN_VALUE}, {Short.MAX_VALUE, -31200, 31200}};
            var buf = new FriendlyByteBuf(Unpooled.buffer());
            try {
                buf.writeVarInt(ids[5+i]); for (short value : controls[i]) buf.writeShort(value);
                motion("motion_wire_control_" + i, ClientboundSetEntityMotionPacket.STREAM_CODEC.decode(buf));
                if (buf.readableBytes() != 0) throw new IllegalStateException("motion control trailing bytes");
            } finally { buf.release(); }
        }
        List<Map<String, Object>> negatives = new ArrayList<>();
        String[][] controls = {{"empty_tail", "00"}, {"missing_eof", "00000001"},
            {"unknown_serializer", "000063ff"}, {"truncated_item", "00080701"}, {"trailing_after_eof", "00ffee"}};
        for (String[] c : controls) {
            var buf = registryBuffer(HEX.parseHex(c[1]));
            Map<String, Object> result = new LinkedHashMap<>(); result.put("case", c[0]); result.put("body_hex", c[1]);
            try {
                var packet = ClientboundSetEntityDataPacket.STREAM_CODEC.decode(buf);
                result.put("status", "decoded"); result.put("values", packet.packedItems().size());
            } catch (Exception error) { result.put("status", "rejected"); result.put("error_class", error.getClass().getName()); }
            finally { result.put("reader_position", buf.readerIndex()); result.put("remaining", buf.readableBytes()); buf.release(); }
            negatives.add(result);
        }
        evidence.put("original_metadata_negative_controls", negatives);
        evidence.put("rows", rows.size()-1);
        Files.writeString(Path.of("paper.tsv"), String.join("\n", rows) + "\n");
        Gson gson = new GsonBuilder().setPrettyPrinting().disableHtmlEscaping().create();
        Files.writeString(Path.of("paper-observations.json"), gson.toJson(evidence) + "\n");
    }
}
