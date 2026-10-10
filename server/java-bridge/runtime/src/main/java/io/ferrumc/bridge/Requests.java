package io.ferrumc.bridge;

import com.google.gson.*;
import java.math.BigInteger;
import java.util.*;

/** Typed validation runs before invoking any plugin callback. */
final class Requests {
    private static final BigInteger MAX_ID = new BigInteger("18446744073709551615");

    static BigInteger id(JsonObject object) {
        JsonElement value = object.get("id");
        if (value == null || !value.isJsonPrimitive() || !value.getAsJsonPrimitive().isNumber())
            throw new IllegalArgumentException("id must be an unsigned 64-bit integer");
        String encoded = value.getAsString();
        if (!encoded.matches("[0-9]{1,20}")) throw new IllegalArgumentException("invalid request id");
        BigInteger id = new BigInteger(encoded);
        if (id.compareTo(MAX_ID) > 0) throw new IllegalArgumentException("request id exceeds u64");
        return id;
    }

    static void fields(JsonObject object, String... allowed) {
        Set<String> valid = Set.of(allowed);
        for (String field : object.keySet())
            if (!valid.contains(field)) throw new IllegalArgumentException("unknown field: " + field);
    }

    static String string(JsonObject object, String field, int limit) {
        JsonElement value = object.get(field);
        if (value == null || !value.isJsonPrimitive() || !value.getAsJsonPrimitive().isString())
            throw new IllegalArgumentException(field + " must be a string");
        String text = value.getAsString();
        if (text.length() > limit || text.indexOf('\0') >= 0)
            throw new IllegalArgumentException(field + " exceeds limit or contains NUL");
        return text;
    }

    static List<String> strings(JsonObject object, String field, int count, int width) {
        JsonElement value = object.get(field);
        if (value == null || !value.isJsonArray() || value.getAsJsonArray().size() > count)
            throw new IllegalArgumentException(field + " must be an array with at most " + count + " strings");
        List<String> values = new ArrayList<>();
        for (JsonElement element : value.getAsJsonArray()) {
            JsonObject wrapper = new JsonObject();
            wrapper.add(field, element);
            values.add(string(wrapper, field, width));
        }
        return List.copyOf(values);
    }

    record PlayerSpec(String name, UUID uuid, boolean op, Set<String> permissions) {}

    static PlayerSpec player(JsonObject request) {
        JsonElement value = request.get("player");
        if (value == null || !value.isJsonObject()) throw new IllegalArgumentException("player must be an object");
        JsonObject object = value.getAsJsonObject();
        fields(object, "name", "uuid", "op", "permissions");
        String name = string(object, "name", 16);
        if (!name.matches("[A-Za-z0-9_]{1,16}")) throw new IllegalArgumentException("invalid player name");
        String uuidText = string(object, "uuid", 36);
        UUID uuid = UUID.fromString(uuidText);
        if (!uuid.toString().equalsIgnoreCase(uuidText)) throw new IllegalArgumentException("UUID must use canonical format");
        JsonElement operator = object.get("op");
        if (operator == null || !operator.isJsonPrimitive() || !operator.getAsJsonPrimitive().isBoolean())
            throw new IllegalArgumentException("op must be a boolean");
        Set<String> permissions = new HashSet<>();
        for (String node : strings(object, "permissions", 256, 128)) {
            if (!node.matches("[A-Za-z0-9_.:*\\-]{1,128}")) throw new IllegalArgumentException("invalid permission node");
            permissions.add(node.toLowerCase(Locale.ROOT));
        }
        return new PlayerSpec(name, uuid, operator.getAsBoolean(), Set.copyOf(permissions));
    }
}
