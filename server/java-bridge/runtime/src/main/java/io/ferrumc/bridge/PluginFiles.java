package io.ferrumc.bridge;

import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.jar.*;
import org.bukkit.plugin.PluginDescriptionFile;
import org.yaml.snakeyaml.LoaderOptions;
import org.yaml.snakeyaml.Yaml;
import org.yaml.snakeyaml.constructor.SafeConstructor;

/** Preflight every bounded archive before creating a plugin class or invoking lifecycle. */
final class PluginFiles {
    static final int MAX_PLUGINS = 64;
    static final int MAX_COMMANDS = 128;
    static final int MAX_TOTAL_COMMANDS = 256;
    static final int MAX_METADATA = 16_384;
    static final long MAX_JAR = 64L * 1024 * 1024;
    record Spec(Path jar, PluginDescriptionFile description) {
        String key() { return description.getName().toLowerCase(Locale.ROOT); }
    }

    static List<Spec> scan(Path directory) throws Exception {
        Files.createDirectories(directory);
        List<Path> jars;
        try (var paths = Files.list(directory)) {
            jars = paths.filter(path -> path.getFileName().toString().endsWith(".jar"))
                    .sorted().limit(MAX_PLUGINS + 1).toList();
        }
        if (jars.size() > MAX_PLUGINS) throw new IllegalArgumentException("at most 64 plugin JARs supported");
        Map<String, Spec> specs = new LinkedHashMap<>();
        Set<String> labels = new HashSet<>();
        int commands = 0;
        int aliasCount = 0;
        for (Path path : jars) {
            Spec spec = read(path);
            if (specs.putIfAbsent(spec.key(), spec) != null)
                throw new IllegalArgumentException("duplicate plugin name: " + spec.description().getName());
            Map<String, Map<String, Object>> declared = spec.description().getCommands();
            if (declared.size() > MAX_COMMANDS || (commands += declared.size()) > MAX_TOTAL_COMMANDS)
                throw new IllegalArgumentException("plugin/total command count exceeds 128/256");
            for (var entry : declared.entrySet()) {
                String label = label(entry.getKey());
                claim(labels, label);
                claim(labels, spec.key() + ":" + label);
                List<String> aliases = aliases(entry.getValue().get("aliases"));
                if ((aliasCount += aliases.size()) > 256) throw new IllegalArgumentException("at most 256 total command aliases supported");
                for (String alias : aliases) {
                    claim(labels, alias);
                    claim(labels, spec.key() + ":" + alias);
                }
                boundedText(entry.getValue().get("description"), 512, "command description");
                boundedText(entry.getValue().get("usage"), 512, "command usage");
                if (entry.getValue().containsKey("permission")) permissionNode(entry.getValue().get("permission"));
                boundedText(entry.getValue().get("permission-message"), 512, "permission message");
            }
        }
        List<Spec> ordered = new ArrayList<>();
        Set<String> done = new HashSet<>();
        while (ordered.size() != specs.size()) {
            boolean advanced = false;
            for (Spec spec : specs.values()) {
                if (done.contains(spec.key())) continue;
                Set<String> required = new LinkedHashSet<>();
                for (String dep : spec.description().getDepend()) {
                    String key = dep.toLowerCase(Locale.ROOT);
                    if (!specs.containsKey(key)) throw new IllegalArgumentException("missing dependency " + dep + " for " + spec.key());
                    required.add(key);
                }
                for (String dep : spec.description().getSoftDepend()) {
                    String key = dep.toLowerCase(Locale.ROOT);
                    if (specs.containsKey(key)) required.add(key);
                }
                for (Spec other : specs.values())
                    for (String before : other.description().getLoadBefore())
                        if (before.equalsIgnoreCase(spec.description().getName())) required.add(other.key());
                if (!done.containsAll(required)) continue;
                ordered.add(spec);
                done.add(spec.key());
                advanced = true;
            }
            if (!advanced) throw new IllegalArgumentException("cyclic plugin dependency/load order");
        }
        return List.copyOf(ordered);
    }

    static Spec read(Path path) throws Exception {
        if (!Files.isRegularFile(path) || Files.size(path) > MAX_JAR)
            throw new IllegalArgumentException("plugin JAR must be a regular file at most 64 MiB: " + path.getFileName());
        try (JarFile jar = new JarFile(path.toFile())) {
            Set<String> entries = new HashSet<>();
            long inflated = 0;
            var enumeration = jar.entries();
            while (enumeration.hasMoreElements()) {
                JarEntry entry = enumeration.nextElement();
                if (!entries.add(entry.getName())) throw new IllegalArgumentException("duplicate ZIP entry: " + entry.getName());
                if (entries.size() > 4096 || entry.getSize() < 0 || entry.getSize() > 16L * 1024 * 1024
                        || (inflated += entry.getSize()) > 128L * 1024 * 1024)
                    throw new IllegalArgumentException("plugin ZIP entry/expanded-size limit exceeded");
            }
            if (jar.getJarEntry("paper-plugin.yml") != null)
                throw new UnsupportedOperationException("paper-plugin.yml bootstrap/lifecycle is not supported; use a standard plugin.yml plugin");
            JarEntry metadata = jar.getJarEntry("plugin.yml");
            if (metadata == null || metadata.getSize() > MAX_METADATA)
                throw new IllegalArgumentException("missing or oversized plugin.yml (limit 16384 bytes)");
            byte[] bytes;
            try (InputStream input = jar.getInputStream(metadata)) {
                bytes = input.readNBytes(MAX_METADATA + 1);
            }
            if (bytes.length > MAX_METADATA) throw new IllegalArgumentException("plugin.yml exceeds 16384 bytes");
            LoaderOptions options = new LoaderOptions();
            options.setAllowDuplicateKeys(false);
            options.setMaxAliasesForCollections(0);
            options.setNestingDepthLimit(16);
            options.setCodePointLimit(MAX_METADATA);
            Object decoded = new Yaml(new SafeConstructor(options)).load(new ByteArrayInputStream(bytes));
            if (!(decoded instanceof Map<?, ?> yaml)) throw new IllegalArgumentException("plugin.yml must be a YAML mapping");
            if (yaml.containsKey("libraries") && !(yaml.get("libraries") instanceof List<?> list && list.isEmpty()))
                throw new UnsupportedOperationException("plugin.yml libraries resolution is not supported; bundle required libraries in the plugin JAR");
            PluginDescriptionFile description = new PluginDescriptionFile(new ByteArrayInputStream(bytes));
            if (!description.getName().matches("[A-Za-z0-9_\\-]{1,64}")) throw new IllegalArgumentException("unsafe/oversized plugin name");
            if (!description.getMain().matches("[A-Za-z_$][A-Za-z0-9_$]*(\\.[A-Za-z_$][A-Za-z0-9_$]*)+")
                    || description.getMain().length() > 256)
                throw new IllegalArgumentException("invalid plugin main class");
            boundedText(description.getVersion(), 128, "plugin version");
            // Remapping an older NMS ABI is a separate, unsupported feature.
            String api = description.getAPIVersion();
            if (api != null && !api.matches("1\\.21(\\.[0-8])?"))
                throw new UnsupportedOperationException("unsupported api-version: " + api + " (target 1.21.8)");
            return new Spec(path.toRealPath(), description);
        }
    }

    static String label(String value) {
        String label = value.toLowerCase(Locale.ROOT);
        if (!label.matches("[a-z0-9_\\-.]{1,64}")) throw new IllegalArgumentException("invalid command label: " + value);
        return label;
    }

    static List<String> aliases(Object declared) {
        if (declared == null) return List.of();
        List<?> raw = declared instanceof List<?> list ? list : List.of(declared);
        if (raw.size() > 16) throw new IllegalArgumentException("at most 16 aliases per command supported");
        List<String> result = new ArrayList<>();
        for (Object alias : raw) {
            if (!(alias instanceof String text)) throw new IllegalArgumentException("command aliases must be strings");
            result.add(label(text));
        }
        return List.copyOf(result);
    }

    private static void claim(Set<String> labels, String label) {
        if (!labels.add(label)) throw new IllegalArgumentException("command/alias collision: " + label);
    }

    static String boundedText(Object value, int max, String field) {
        if (value == null) return "";
        if (!(value instanceof String text) || text.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > max || text.indexOf('\0') >= 0)
            throw new IllegalArgumentException("invalid/oversized " + field);
        return text;
    }

    static String permissionNode(Object value) {
        String node = boundedText(value, 128, "command permission").toLowerCase(Locale.ROOT);
        if (!node.matches("\\*|[a-z0-9_\\-]+(?:\\.[a-z0-9_\\-]+)*(?:\\.\\*)?"))
            throw new UnsupportedOperationException("unsupported permission node grammar: " + node + "; expected a simple dotted ASCII node or trailing wildcard, not Bukkit OR expressions");
        return node;
    }
}
