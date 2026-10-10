package io.ferrumc.bridge;

import com.google.gson.*;
import com.google.gson.stream.JsonReader;
import com.google.gson.stream.JsonToken;
import java.io.*;
import java.nio.ByteBuffer;
import java.nio.charset.*;
import java.util.HashSet;

/** Bounded, strict UTF-8 JSON frames shared with the Rust process. */
final class Frames {
    static final int MAX_BYTES = 65_536;
    private static final Gson JSON = new GsonBuilder().serializeNulls().disableHtmlEscaping().create();

    static JsonObject read(InputStream input) throws IOException {
        int first = input.read();
        if (first < 0) return null;
        byte[] header = input.readNBytes(3);
        if (header.length != 3) throw new EOFException("truncated frame header");
        long length = ((long) first << 24) | ((long) (header[0] & 255) << 16)
                | ((long) (header[1] & 255) << 8) | (header[2] & 255);
        if (length == 0 || length > MAX_BYTES) throw new IOException("frame length outside 1..65536");
        byte[] bytes = input.readNBytes((int) length);
        if (bytes.length != length) throw new EOFException("truncated frame body");
        try {
            String text = StandardCharsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString();
            JsonReader reader = new JsonReader(new StringReader(text));
            reader.setStrictness(Strictness.STRICT);
            JsonElement value = readJson(reader, 0, new int[] {0});
            if (!value.isJsonObject() || reader.peek() != JsonToken.END_DOCUMENT)
                throw new IOException("request must be exactly one JSON object");
            return value.getAsJsonObject();
        } catch (IllegalStateException | IllegalArgumentException ex) {
            throw new IOException("invalid JSON request", ex);
        }
    }

    private static JsonElement readJson(JsonReader reader, int depth, int[] nodes) throws IOException {
        if (depth > 16 || ++nodes[0] > 4096) throw new IOException("JSON nesting/node limit exceeded");
        return switch (reader.peek()) {
            case BEGIN_OBJECT -> {
                reader.beginObject();
                JsonObject object = new JsonObject();
                HashSet<String> names = new HashSet<>();
                while (reader.hasNext()) {
                    String name = reader.nextName();
                    if (!names.add(name)) throw new IOException("duplicate JSON field: " + name);
                    object.add(name, readJson(reader, depth + 1, nodes));
                }
                reader.endObject();
                yield object;
            }
            case BEGIN_ARRAY -> {
                reader.beginArray();
                JsonArray array = new JsonArray();
                while (reader.hasNext()) array.add(readJson(reader, depth + 1, nodes));
                reader.endArray();
                yield array;
            }
            case STRING -> new JsonPrimitive(reader.nextString());
            case NUMBER -> new JsonPrimitive(new java.math.BigDecimal(reader.nextString()));
            case BOOLEAN -> new JsonPrimitive(reader.nextBoolean());
            case NULL -> { reader.nextNull(); yield JsonNull.INSTANCE; }
            default -> throw new IOException("invalid JSON value");
        };
    }

    static void write(OutputStream output, JsonObject response) throws IOException {
        byte[] body = JSON.toJson(response).getBytes(StandardCharsets.UTF_8);
        if (body.length > MAX_BYTES) {
            JsonObject bounded = new JsonObject();
            bounded.add("id", response.get("id"));
            bounded.addProperty("ok", false);
            bounded.add("messages", new JsonArray());
            if (response.has("plugins")) bounded.add("plugins", new JsonArray());
            bounded.addProperty("error", "response exceeded 65536-byte limit");
            body = JSON.toJson(bounded).getBytes(StandardCharsets.UTF_8);
        }
        DataOutputStream framed = new DataOutputStream(output);
        framed.writeInt(body.length);
        framed.write(body);
        framed.flush();
    }
}
