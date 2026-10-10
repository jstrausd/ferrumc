package io.ferrumc.bridge;

import java.io.File;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;
import org.bukkit.event.Event;
import org.bukkit.event.Listener;
import org.bukkit.plugin.*;

/** Paper JavaPlugin initialization requires this legacy SPI, even for its new init overload. */
@SuppressWarnings("removal")
public final class BridgePluginLoader implements PluginLoader {
    /** Public constructor required by ServiceLoader. */
    public BridgePluginLoader() {}
    private UnsupportedOperationException unsupported(String method) {
        return new UnsupportedOperationException("FerrumC Java bridge does not support PluginLoader." + method);
    }
    @Override public Plugin loadPlugin(File file) { throw unsupported("loadPlugin"); }
    @Override public PluginDescriptionFile getPluginDescription(File file) { throw unsupported("getPluginDescription"); }
    @Override public Pattern[] getPluginFileFilters() { throw unsupported("getPluginFileFilters"); }
    @Override public Map<Class<? extends Event>, Set<RegisteredListener>> createRegisteredListeners(Listener listener, Plugin plugin) { throw unsupported("createRegisteredListeners"); }
    @Override public void enablePlugin(Plugin plugin) { throw unsupported("enablePlugin"); }
    @Override public void disablePlugin(Plugin plugin) { throw unsupported("disablePlugin"); }
}
