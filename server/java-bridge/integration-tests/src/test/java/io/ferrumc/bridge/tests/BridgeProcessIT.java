package io.ferrumc.bridge.tests;

import static org.junit.jupiter.api.Assertions.*;
import com.google.gson.*;
import java.io.*;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.jar.*;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;
import org.bukkit.configuration.file.YamlConfiguration;

/** Executes the packaged bridge and a separately compiled, ordinary Bukkit plugin. */
class BridgeProcessIT {
    private static final String UUID_A = "12345678-1234-5678-1234-567812345678";
    private static final String UUID_B = "87654321-4321-8765-4321-876543218765";
    private static final Path MODULE = Path.of("..").toAbsolutePath().normalize();
    private static final Path BRIDGE = MODULE.resolve("runtime/target/ferrumc-java-bridge-0.1.0.jar");
    private static final Path FIXTURE = MODULE.resolve("fixture/target/standard-bukkit-fixture-0.1.0.jar");
    @TempDir Path directory;

    private JsonObject player(String name, String uuid, boolean op, String... permissions) {
        JsonObject player = new JsonObject();
        player.addProperty("name", name); player.addProperty("uuid", uuid); player.addProperty("op", op);
        JsonArray values = new JsonArray(); for (String permission : permissions) values.add(permission);
        player.add("permissions", values); return player;
    }
    private JsonObject request(String op) {
        JsonObject request = new JsonObject(); request.addProperty("op", op); return request;
    }
    private JsonObject event(String op, JsonObject player) {
        JsonObject request = request(op); request.add("player", player); return request;
    }
    private JsonObject command(JsonObject player, String name, String... args) {
        JsonObject request = event("command", player); request.addProperty("command", name);
        JsonArray values = new JsonArray(); for (String arg : args) values.add(arg);
        request.add("args", values); return request;
    }
    private List<String> texts(JsonObject response) {
        List<String> result = new ArrayList<>();
        for (JsonElement message : response.getAsJsonArray("messages")) result.add(message.getAsJsonObject().get("text").getAsString());
        return result;
    }
    private JsonObject complete(JsonObject player, String name, String... args) {
        JsonObject request = command(player, name, args); request.addProperty("op", "complete"); return request;
    }
    private List<String> suggestions(JsonObject response) {
        return response.getAsJsonArray("completions").asList().stream().map(JsonElement::getAsString).toList();
    }
    private void fixture() throws IOException { Files.copy(FIXTURE, directory.resolve("fixture.jar")); }
    private void ok(JsonObject response) { assertTrue(response.get("ok").getAsBoolean(), response.toString()); assertTrue(response.get("error").isJsonNull()); }
    private void error(JsonObject response, String fragment) {
        assertFalse(response.get("ok").getAsBoolean(), response.toString());
        assertTrue(response.get("error").getAsString().contains(fragment), response.toString());
    }
    private void counters(int enabled, int disabled) {
        YamlConfiguration config = YamlConfiguration.loadConfiguration(directory.resolve("BridgeFixture/config.yml").toFile());
        assertEquals(enabled, config.getInt("enabled")); assertEquals(disabled, config.getInt("disabled"));
    }

    @Test void unchangedJarExecutesLifecycleAliasPermissionEventsAndPersistsAcrossRestart() throws Exception {
        fixture();
        for (int run = 1; run <= 2; run++) {
            try (Child child = new Child(directory)) {
                JsonObject loaded = child.load(); ok(loaded);
                JsonObject plugin = loaded.getAsJsonArray("plugins").get(0).getAsJsonObject();
                assertEquals("BridgeFixture", plugin.get("name").getAsString());
                assertEquals("fixture.use", plugin.getAsJsonArray("commands").get(0).getAsJsonObject().get("permission").getAsString());
                JsonObject spec = player("Tester", UUID_A, false);
                assertEquals(List.of("join:lowest", "join:monitor"), texts(child.call(event("join", spec))));
                JsonObject denied = child.call(command(spec, "bfixture")); ok(denied);
                assertEquals(List.of("Missing fixture.use"), texts(denied));
                spec = player("Tester", UUID_A, false, "fixture.use");
                JsonObject allowed = child.call(command(spec, "BFIXTURE")); ok(allowed);
                assertEquals(List.of("fixture:bfixture:Tester:" + UUID_A + ":false:true:FerrumC:1.0.0"), texts(allowed));
                JsonObject namespaced = child.call(command(spec, "bridgefixture:bfixture")); ok(namespaced);
                assertTrue(texts(namespaced).get(0).startsWith("fixture:bridgefixture:bfixture:Tester:"));
                JsonObject unicode = child.call(command(spec, "bridgefixture", "unicode")); ok(unicode);
                assertEquals(List.of("Grüße 🌍"), texts(unicode));
                JsonObject quit = child.call(event("quit", spec)); ok(quit); assertEquals(List.of("quit:fixture"), texts(quit));
                JsonObject stop = child.call(request("shutdown")); ok(stop); child.finished(0);
                counters(run, run);
                YamlConfiguration config = YamlConfiguration.loadConfiguration(directory.resolve("BridgeFixture/config.yml").toFile());
                assertEquals(run, config.getInt("join-count")); assertEquals(run, config.getInt("quit-count"));
                assertTrue(Files.readString(child.stderr).contains("fixture stdout onDisable"));
            }
        }
    }

    @Test void playerStateRefreshUpdatesRetainedProxyWithoutEventsAndRejectsIdentityChanges() throws Exception {
        fixture();
        try (Child child = new Child(directory)) {
            error(child.call(event("player_state", player("Tester", UUID_A, true))), "not active");
            ok(child.load());
            JsonObject operator = player("Tester", UUID_A, true);
            ok(child.call(event("join", operator)));
            // A different sender inspects the object retained by the join listener.
            // Sending another command as Tester would itself refresh Tester's state.
            JsonObject observer = player("Observer", UUID_B, true);
            assertEquals(List.of("held:" + UUID_A + ":true:true:false"),
                    texts(child.call(command(observer, "bridgefixture", "previous-status"))));
            JsonObject revoked = child.call(event("player_state", player("Tester", UUID_A, false, "fixture.retained")));
            ok(revoked); assertTrue(texts(revoked).isEmpty());
            assertEquals(List.of("held:" + UUID_A + ":false:false:true"),
                    texts(child.call(command(observer, "bridgefixture", "previous-status"))));
            ok(child.call(event("player_state", player("Tester", UUID_A, false, "fixture.use"))));
            assertEquals(List.of("held:" + UUID_A + ":false:true:false"),
                    texts(child.call(command(observer, "bridgefixture", "previous-status"))));
            ok(child.call(event("player_state", operator)));
            error(child.call(event("player_state", player("Observer", UUID_B, false))), "unknown online player");
            error(child.call(event("player_state", player("Changed", UUID_A, false))), "identity");
            JsonObject malformed = player("Tester", UUID_A, false);
            malformed.addProperty("op", "false");
            error(child.call(event("player_state", malformed)), "boolean");
            JsonObject extra = event("player_state", player("Tester", UUID_A, false));
            extra.addProperty("unexpected", 0);
            error(child.call(extra), "unknown field");
            assertEquals(List.of("held:" + UUID_A + ":true:true:false"),
                    texts(child.call(command(observer, "bridgefixture", "previous-status"))));
            ok(child.call(event("quit", operator)));
            error(child.call(event("player_state", operator)), "unknown online player");
            ok(child.call(request("shutdown"))); child.finished(0); counters(1, 1);
            YamlConfiguration config = YamlConfiguration.loadConfiguration(directory.resolve("BridgeFixture/config.yml").toFile());
            assertEquals(1, config.getInt("join-count"));
            assertEquals(1, config.getInt("quit-count"));
        }
    }

    @Test void explicitUnsupportedApisAndListenerFailuresKeepNextFrameUsable() throws Exception {
        fixture();
        try (Child child = new Child(directory)) {
            ok(child.load());
            JsonObject faulty = player("Faulty", UUID_A, true);
            JsonObject joined = child.call(event("join", faulty)); error(joined, "fixture listener failed");
            assertEquals(List.of("join:lowest", "join:monitor"), texts(joined));
            error(child.call(command(faulty, "bridgefixture", "world")), ".getWorld");
            error(child.call(command(faulty, "bridgefixture", "scheduler")), ".getScheduler");
            error(child.call(command(faulty, "bridgefixture", "fail")), "fixture command failed");
            JsonObject unicodeError = child.call(command(faulty, "bridgefixture", "unicode-error"));
            error(unicodeError, "IllegalStateException");
            assertTrue(unicodeError.get("error").getAsString().getBytes(StandardCharsets.UTF_8).length <= 2048);
            assertFalse(unicodeError.get("error").getAsString().contains("\0"));
            error(child.call(command(faulty, "bridgefixture", "large")), "message limit");
            JsonObject spam = child.call(command(faulty, "bridgefixture", "spam")); error(spam, "message limit");
            assertEquals(64, spam.getAsJsonArray("messages").size());
            ok(child.call(command(faulty, "bridgefixture", "unicode")));
            JsonObject broad = player("Broadcaster", UUID_B, true);
            error(child.call(event("join", broad)), "broadcasting is not implemented");
            error(child.call(command(faulty, "bridgefixture", "previous")), "cross-player messaging");
            ok(child.call(request("shutdown"))); child.finished(0); counters(1, 1);
        }
    }

    @Test void completersUseAliasesSenderArgumentsPermissionsAndBoundOutput() throws Exception {
        fixture();
        try (Child child = new Child(directory)) {
            ok(child.load());
            JsonObject denied = child.call(complete(player("Tester", UUID_A, false), "bfixture", "fail"));
            ok(denied); assertTrue(suggestions(denied).isEmpty());
            JsonObject spec = player("Tester", UUID_A, false, "fixture.use");
            JsonObject explicit = child.call(complete(spec, "BFIXTURE", "explicit")); ok(explicit);
            assertEquals(List.of("explicit:BFIXTURE:Tester"), suggestions(explicit));
            JsonObject inherited = child.call(complete(spec, "bridgefixture:bfixture", "probe", "")); ok(inherited);
            assertEquals(List.of("probe:bridgefixture:bfixture:" + UUID_A + ":2:"), suggestions(inherited));
            JsonObject prefix = child.call(complete(spec, "bridgefixture", "a")); ok(prefix);
            assertEquals(List.of("alpha"), suggestions(prefix));
            JsonObject unicode = child.call(complete(spec, "bridgefixture", "")); ok(unicode);
            assertEquals(List.of("alpha", "beta", "Grüße Ω", "🌍"), suggestions(unicode));
            for (String bad : List.of("fail", "large", "many", "nul", "surrogate", "escaped", "null-entry", "fallback", "combined")) {
                JsonObject failed = child.call(complete(spec, "bridgefixture", bad));
                assertFalse(failed.get("ok").getAsBoolean(), failed.toString());
                assertTrue(suggestions(failed).isEmpty(), failed.toString());
            }
            error(child.call(complete(spec, "bridgefixture")), "current argument");
            error(child.call(complete(spec, "absent", "")), "unknown plugin command");
            error(child.call(command(spec, "bridgefixture", "escaped-message")), "serialized callback output");
            ok(child.call(command(spec, "bfixture", "unicode")));
            ok(child.call(request("shutdown"))); child.finished(0); counters(1, 1);
        }
    }

    @Test void invalidRequestsCannotMutateLifecycleAndMaximumUnsignedIdIsPreserved() throws Exception {
        fixture();
        try (Child child = new Child(directory)) {
            error(child.call(request("unrecognized")), "unsupported operation");
            JsonObject bad = request("load"); bad.addProperty("plugins_dir", directory.toString()); bad.addProperty("extra", 0);
            error(child.call(bad), "unknown field");
            JsonObject load = request("load"); load.addProperty("plugins_dir", directory.toString());
            load.addProperty("id", new BigInteger("18446744073709551615"));
            JsonObject loaded = child.call(load); ok(loaded); assertEquals("18446744073709551615", loaded.get("id").getAsString());
            error(child.load(), "exactly once");
            JsonObject spec = player("Tester", UUID_A, false);
            spec.addProperty("uuid", "1-1-1-1-1"); error(child.call(event("join", spec)), "canonical");
            spec = player("Tester", UUID_A, true); ok(child.call(event("join", spec)));
            error(child.call(event("join", spec)), "duplicate player join");
            error(child.call(command(spec, "absent")), "unknown plugin command");
            JsonObject badArgs = command(spec, "bridgefixture"); JsonArray args = new JsonArray(); args.add(false); badArgs.add("args", args);
            error(child.call(badArgs), "must be a string");
            ok(child.call(request("shutdown"))); child.finished(0); counters(1, 1);
        }
    }

    @Test void failedEnableRollsBackAndDisablesExactlyOnce() throws Exception {
        fixture();
        Files.createDirectories(directory.resolve("BridgeFixture"));
        Files.writeString(directory.resolve("BridgeFixture/config.yml"), "fail-enable: true\nenabled: 0\ndisabled: 0\n");
        try (Child child = new Child(directory)) {
            error(child.load(), ".getScheduler"); counters(1, 1);
            error(child.call(command(player("Tester", UUID_A, true), "bridgefixture")), "not active");
            ok(child.call(request("shutdown"))); child.finished(0); counters(1, 1);
        }
    }

    @Test void archiveFailuresRunNoPluginCodeAndReturnEmptyMetadata() throws Exception {
        fixture(); Files.copy(FIXTURE, directory.resolve("duplicate.jar"));
        try (Child child = new Child(directory)) {
            JsonObject failed = child.load(); error(failed, "duplicate plugin name");
            assertTrue(failed.getAsJsonArray("plugins").isEmpty());
            assertFalse(Files.exists(directory.resolve("BridgeFixture/config.yml")));
            ok(child.call(request("shutdown"))); child.finished(0);
            assertFalse(Files.readString(child.stderr).contains("fixture stdout onLoad"));
        }
    }

    @Test void eofRunsDisableOnceAndMalformedFramesTerminateWithoutUnframedOutput() throws Exception {
        fixture();
        try (Child child = new Child(directory)) {
            ok(child.load()); child.process.getOutputStream().close(); child.finished(0); counters(1, 1);
            assertEquals(-1, child.process.getInputStream().read());
        }
        byte[][] malformed = {{0}, {0,0,0,0}, {0,1,0,1}, {(byte)255,0,0,0}, {0,0,0,2,'{'}, {0,0,0,2,(byte)0xc3,0x28}};
        for (byte[] bytes : malformed) {
            try (Child child = new Child(directory)) {
                child.process.getOutputStream().write(bytes); child.process.getOutputStream().close(); child.finished(1);
                assertEquals(-1, child.process.getInputStream().read(), "no non-frame stdout");
            }
        }
        for (String malformedJson : List.of("{\"id\":1,\"id\":2,\"op\":\"shutdown\"}", "{\"id\":-1,\"op\":\"shutdown\"}", "{\"id\":18446744073709551616,\"op\":\"shutdown\"}")) {
            try (Child child = new Child(directory)) {
                child.raw(malformedJson); child.process.getOutputStream().close(); child.finished(1);
                assertEquals(-1, child.process.getInputStream().read());
            }
        }
    }

    private static final class Child implements AutoCloseable {
        final Process process;
        final Path stderr;
        final Path plugins;
        final ExecutorService reads = Executors.newSingleThreadExecutor(r -> { Thread t = new Thread(r, "bridge-test-reader"); t.setDaemon(true); return t; });
        long nextId = 1;
        Child(Path directory) throws IOException {
            plugins = directory;
            stderr = Files.createTempFile(directory, "stderr-", ".log");
            process = new ProcessBuilder(Path.of(System.getProperty("java.home"), "bin/java").toString(), "-jar", BRIDGE.toString())
                    .redirectError(stderr.toFile()).start();
        }
        JsonObject load() throws Exception {
            JsonObject request = new JsonObject(); request.addProperty("op", "load"); request.addProperty("plugins_dir", plugins.toString());
            return call(request);
        }
        void raw(String json) throws IOException {
            byte[] body = json.getBytes(StandardCharsets.UTF_8);
            DataOutputStream out = new DataOutputStream(process.getOutputStream()); out.writeInt(body.length); out.write(body); out.flush();
        }
        JsonObject call(JsonObject request) throws Exception {
            if (!request.has("id")) request.addProperty("id", nextId++);
            raw(new Gson().toJson(request));
            Future<JsonObject> pending = reads.submit(() -> {
                DataInputStream input = new DataInputStream(process.getInputStream()); int size = input.readInt();
                assertTrue(size > 0 && size <= 65_536, "frame size " + size);
                byte[] body = input.readNBytes(size); assertEquals(size, body.length);
                return JsonParser.parseString(new String(body, StandardCharsets.UTF_8)).getAsJsonObject();
            });
            JsonObject response = pending.get(10, TimeUnit.SECONDS);
            assertEquals(request.get("id").getAsString(), response.get("id").getAsString());
            return response;
        }
        void finished(int code) throws Exception {
            assertTrue(process.waitFor(10, TimeUnit.SECONDS), "child did not exit; stderr " + stderr);
            assertEquals(code, process.exitValue(), () -> {
                try { return Files.readString(stderr); } catch (IOException e) { return e.toString(); }
            });
        }
        @Override public void close() throws Exception {
            process.getOutputStream().close();
            if (process.isAlive() && !process.waitFor(10, TimeUnit.SECONDS)) process.destroyForcibly();
            reads.shutdownNow();
        }
    }
}
