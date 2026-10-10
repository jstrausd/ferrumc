package io.ferrumc.bridge;

import io.papermc.paper.plugin.configuration.PluginMeta;
import io.papermc.paper.plugin.provider.classloader.ConfiguredPluginClassLoader;
import io.papermc.paper.plugin.provider.classloader.PluginClassLoaderGroup;
import java.net.*;
import java.nio.file.*;
import java.util.List;
import java.util.logging.Logger;
import org.bukkit.Server;
import org.bukkit.plugin.java.JavaPlugin;

/** Loads unmodified bytecode, exposing only explicitly declared dependencies. */
final class PluginClassLoader extends URLClassLoader implements ConfiguredPluginClassLoader {
    private final PluginFiles.Spec spec;
    private final Server server;
    private final Path data;
    private final List<PluginClassLoader> dependencies;
    private JavaPlugin plugin;

    PluginClassLoader(PluginFiles.Spec spec, Server server, Path data, List<PluginClassLoader> dependencies) throws Exception {
        super(new URL[] {spec.jar().toUri().toURL()}, PluginClassLoader.class.getClassLoader());
        this.spec = spec;
        this.server = server;
        this.data = data;
        this.dependencies = List.copyOf(dependencies);
    }

    @Override protected Class<?> loadClass(String name, boolean resolve) throws ClassNotFoundException {
        return loadClass(name, resolve, true, true);
    }

    @Override public Class<?> loadClass(String name, boolean resolve, boolean checkGlobal, boolean checkLibraries) throws ClassNotFoundException {
        synchronized (getClassLoadingLock(name)) {
            Class<?> found = findLoadedClass(name);
            if (found == null) {
                try { found = getParent().loadClass(name); }
                catch (ClassNotFoundException absent) {
                    try { found = findClass(name); }
                    catch (ClassNotFoundException missing) {
                        if (checkGlobal) {
                            for (PluginClassLoader dependency : dependencies) {
                                try { found = dependency.loadClass(name, false, true, false); break; }
                                catch (ClassNotFoundException ignored) { /* Try the next declared dependency. */ }
                            }
                        }
                        if (found == null) throw missing;
                    }
                }
            }
            if (resolve && found.getClassLoader() == this) resolveClass(found);
            return found;
        }
    }

    @Override public PluginMeta getConfiguration() { return spec.description(); }
    @Override public JavaPlugin getPlugin() { return plugin; }
    @Override public PluginClassLoaderGroup getGroup() { return null; }
    @Override public void init(JavaPlugin instance) {
        if (plugin != null) throw new IllegalStateException("plugin classloader initialized twice");
        if (!instance.getClass().getName().equals(spec.description().getMain()))
            throw new IllegalStateException("only the declared main class may initialize JavaPlugin");
        plugin = instance;
        Logger logger = Logger.getLogger("FerrumC.Plugin." + spec.description().getName());
        instance.init(server, spec.description(), data.toFile(), spec.jar().toFile(), this, spec.description(), logger);
    }
}
