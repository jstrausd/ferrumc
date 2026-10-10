package io.ferrumc.bridge;

import static org.junit.jupiter.api.Assertions.*;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.util.jar.*;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class PluginFilesTest {
    @TempDir Path directory;
    private Path jar(String file, String metadata, String extra) throws Exception {
        Path path = directory.resolve(file);
        try (JarOutputStream archive = new JarOutputStream(Files.newOutputStream(path))) {
            if (metadata != null) {
                archive.putNextEntry(new JarEntry("plugin.yml"));
                archive.write(metadata.getBytes(StandardCharsets.UTF_8));
                archive.closeEntry();
            }
            if (extra != null) { archive.putNextEntry(new JarEntry(extra)); archive.write(1); archive.closeEntry(); }
        }
        return path;
    }
    private String metadata(String name, String tail) { return "name: " + name + "\nversion: 1.0\nmain: example.Main\napi-version: '1.21.8'\n" + tail; }

    @Test void scansDeterministicDependencyOrder() throws Exception {
        jar("a.jar", metadata("Dependent", "depend: [Base]\n"), null);
        jar("z.jar", metadata("Base", ""), null);
        assertEquals(java.util.List.of("base", "dependent"), PluginFiles.scan(directory).stream().map(PluginFiles.Spec::key).toList());
    }

    @Test void rejectsDuplicateNamesMissingDependenciesAndCommandCollisions() throws Exception {
        jar("a.jar", metadata("Same", ""), null);
        jar("b.jar", metadata("sAME", ""), null);
        assertThrows(IllegalArgumentException.class, () -> PluginFiles.scan(directory));
        Files.delete(directory.resolve("b.jar"));
        jar("a.jar", metadata("Same", "depend: [Absent]\n"), null);
        assertThrows(IllegalArgumentException.class, () -> PluginFiles.scan(directory));
        jar("a.jar", metadata("One", "commands:\n  hello:\n    aliases: [hi]\n"), null);
        jar("b.jar", metadata("Two", "commands:\n  hi: {}\n"), null);
        assertThrows(IllegalArgumentException.class, () -> PluginFiles.scan(directory));
    }

    @Test void rejectsUnsupportedOrMalformedBoundedMetadata() throws Exception {
        Path path = jar("bad.jar", null, null);
        assertThrows(IllegalArgumentException.class, () -> PluginFiles.read(path));
        jar("bad.jar", metadata("Paper", ""), "paper-plugin.yml");
        assertThrows(UnsupportedOperationException.class, () -> PluginFiles.read(path));
        jar("bad.jar", metadata("Library", "libraries: [com.example:lib:1]\n"), null);
        assertThrows(UnsupportedOperationException.class, () -> PluginFiles.read(path));
        jar("bad.jar", metadata("Old", "").replace("1.21.8", "1.22"), null);
        assertThrows(UnsupportedOperationException.class, () -> PluginFiles.read(path));
        jar("bad.jar", metadata("Duplicate", "name: Other\n"), null);
        assertThrows(Exception.class, () -> PluginFiles.read(path));
        jar("bad.jar", metadata("Huge", "description: " + "x".repeat(16_384)), null);
        assertThrows(IllegalArgumentException.class, () -> PluginFiles.read(path));
        jar("bad.jar", metadata("Aliased", "commands: &cmd {hello: {}}\nx: *cmd\n"), null);
        assertThrows(Exception.class, () -> PluginFiles.read(path));
        jar("bad.jar", metadata("../Escape", ""), null);
        assertThrows(Exception.class, () -> PluginFiles.read(path));
    }

    @Test void rejectsPluginAndCommandCountLimitsAndCycles() throws Exception {
        for (int i = 0; i <= PluginFiles.MAX_PLUGINS; i++) jar("plugin-" + i + ".jar", metadata("Plugin" + i, ""), null);
        assertThrows(IllegalArgumentException.class, () -> PluginFiles.scan(directory));
        for (Path path : Files.list(directory).toList()) Files.delete(path);
        StringBuilder commands = new StringBuilder("commands:\n");
        for (int i = 0; i <= PluginFiles.MAX_COMMANDS; i++) commands.append("  c").append(i).append(": {}\n");
        jar("one.jar", metadata("One", commands.toString()), null);
        assertThrows(IllegalArgumentException.class, () -> PluginFiles.scan(directory));
        jar("one.jar", metadata("One", "depend: [Two]\n"), null);
        jar("two.jar", metadata("Two", "depend: [One]\n"), null);
        assertThrows(IllegalArgumentException.class, () -> PluginFiles.scan(directory));
    }

    @Test void metadataLimitsCountUtf8BytesAndTotalAliases() throws Exception {
        jar("wide.jar", metadata("Wide", "").replace("version: 1.0", "version: " + "🌍".repeat(33)), null);
        assertThrows(IllegalArgumentException.class, () -> PluginFiles.scan(directory));
        jar("wide.jar", metadata("Wide", "").replace("version: 1.0", "version: " + "🌍".repeat(32)), null);
        assertEquals(1, PluginFiles.scan(directory).size());
        StringBuilder commands = new StringBuilder("commands:\n");
        for (int i = 0; i < 17; i++) {
            commands.append("  c").append(i).append(":\n    aliases: [");
            for (int j = 0; j < 16; j++) { if (j > 0) commands.append(", "); commands.append("a").append(i).append('_').append(j); }
            commands.append("]\n");
        }
        jar("wide.jar", metadata("Wide", commands.toString()), null);
        assertThrows(IllegalArgumentException.class, () -> PluginFiles.scan(directory));
    }

    @Test void permissionGrammarRejectsOrExpressionsEmptyNodesAndInteriorWildcards() throws Exception {
        for (String node : java.util.List.of("foo.use;bar.use", "", "x..y", "x.*.y", "x:use", "é.use")) {
            jar("permission.jar", metadata("Permission", "commands:\n  test:\n    permission: '" + node + "'\n"), null);
            assertThrows(UnsupportedOperationException.class, () -> PluginFiles.scan(directory), node);
        }
        assertEquals("foo.use", PluginFiles.permissionNode("FOO.USE"));
        assertEquals("foo.*", PluginFiles.permissionNode("FOO.*"));
        assertEquals("*", PluginFiles.permissionNode("*"));
    }
}
