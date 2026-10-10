package io.ferrumc.bridge;

import static org.junit.jupiter.api.Assertions.*;
import com.google.gson.*;
import java.io.*;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;

class FramesTest {
    private byte[] frame(String value) throws IOException {
        byte[] body = value.getBytes(StandardCharsets.UTF_8);
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        new DataOutputStream(out).writeInt(body.length);
        out.write(body);
        return out.toByteArray();
    }

    @Test void roundTripKeepsUnsignedIdsAndUtf8() throws Exception {
        JsonObject response = new JsonObject();
        response.addProperty("id", new BigInteger("18446744073709551615"));
        response.addProperty("text", "Grüße 🌍");
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        Frames.write(output, response);
        JsonObject decoded = Frames.read(new ByteArrayInputStream(output.toByteArray()));
        assertEquals(new BigInteger("18446744073709551615"), Requests.id(decoded));
        assertEquals("Grüße 🌍", decoded.get("text").getAsString());
        assertNull(Frames.read(new ByteArrayInputStream(new byte[0])));
    }

    @Test void rejectsEmptyTruncatedOversizedAndInvalidUtf8Frames() {
        for (byte[] malformed : new byte[][] {
            {0}, {0,0,0}, {0,0,0,0}, {0,1,0,1}, {(byte)255,0,0,0}, {0,0,0,2,'{'},
            {0,0,0,2,(byte)0xc3,0x28}
        }) assertThrows(IOException.class, () -> Frames.read(new ByteArrayInputStream(malformed)));
    }

    @Test void rejectsAmbiguousOrNonJsonRequests() throws Exception {
        for (String value : new String[] {"[]", "{}{}", "{\"id\":1,\"id\":2}", "{id:1}", "{\"id\":NaN}", "{\"id\":1,}", "/*x*/{}", ""})
            assertThrows(IOException.class, () -> Frames.read(new ByteArrayInputStream(frame(value))), value);
        assertThrows(IOException.class, () -> Frames.read(new ByteArrayInputStream(frame("{\"x\":" + "[".repeat(18) + "0" + "]".repeat(18) + "}"))));
        assertThrows(IOException.class, () -> Frames.read(new ByteArrayInputStream(frame("{\"x\":[" + "0,".repeat(4096) + "0]}"))));
    }

    @Test void boundaryFrameAndOversizedResponseAreBounded() throws Exception {
        String body = "{\"x\":\"" + "a".repeat(65_528) + "\"}";
        assertEquals(65_536, body.length());
        assertEquals(65_528, Frames.read(new ByteArrayInputStream(frame(body))).get("x").getAsString().length());
        JsonObject response = new JsonObject();
        response.addProperty("id", 2);
        response.addProperty("text", "x".repeat(70_000));
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        Frames.write(output, response);
        JsonObject decoded = Frames.read(new ByteArrayInputStream(output.toByteArray()));
        assertFalse(decoded.get("ok").getAsBoolean());
        assertTrue(decoded.get("error").getAsString().contains("limit"));
    }

    @Test void rejectsBadIdTypesAndPlayerFieldsBeforeCallbacks() {
        for (String value : new String[] {"-1", "18446744073709551616", "1.5", "\"1\"", "true", "null", "1e3"})
            assertThrows(IllegalArgumentException.class, () -> Requests.id(JsonParser.parseString("{\"id\":" + value + "}").getAsJsonObject()));
        JsonObject player = JsonParser.parseString("{\"player\":{\"name\":\"Test\",\"uuid\":\"12345678-1234-5678-1234-567812345678\",\"op\":false,\"permissions\":[]}}").getAsJsonObject();
        assertEquals("Test", Requests.player(player).name());
        player.getAsJsonObject("player").addProperty("uuid", "1-1-1-1-1");
        assertThrows(IllegalArgumentException.class, () -> Requests.player(player));
        assertThrows(IllegalArgumentException.class, () -> Requests.fields(JsonParser.parseString("{\"extra\":0}").getAsJsonObject(), "id"));
    }
}
