package io.ferrumc.bridge;

import com.google.gson.*;
import io.papermc.paper.plugin.lifecycle.event.LifecycleEventManager;
import java.lang.reflect.*;
import java.math.BigInteger;
import java.nio.file.*;
import java.util.*;
import java.util.logging.Logger;
import net.kyori.adventure.text.Component;
import net.kyori.adventure.text.serializer.plain.PlainTextComponentSerializer;
import org.bukkit.*;
import org.bukkit.command.*;
import org.bukkit.entity.Player;
import org.bukkit.event.*;
import org.bukkit.event.player.*;
import org.bukkit.permissions.Permission;
import org.bukkit.plugin.*;
import org.bukkit.plugin.java.JavaPlugin;

/** Single-threaded owner of lifecycle, command routing and the supported Bukkit event bus. */
final class BridgeEngine implements AutoCloseable {
    private final Thread owner = Thread.currentThread();
    private final Logger logger = Logger.getLogger("FerrumC.JavaBridge");
    private final Map<String, JavaPlugin> plugins = new LinkedHashMap<>();
    private final Map<String, PluginCommand> commands = new LinkedHashMap<>();
    private final List<PluginClassLoader> loaders = new ArrayList<>();
    private final Map<UUID, PlayerState> online = new LinkedHashMap<>();
    private final List<RegisteredListener> listeners = new ArrayList<>();
    private final JsonArray messages = new JsonArray();
    private int messageBytes;
    private boolean loaded;
    private boolean active;
    private boolean stopped;
    private String recipient;
    private Path pluginsDirectory;
    private final Server server;
    private final PluginManager manager;
    private final UnsafeValues unsafe;
    private final ConsoleCommandSender console;
    private final CommandMap commandMap;

    BridgeEngine() {
        manager = adapter(PluginManager.class, this::managerCall);
        unsafe = adapter(UnsafeValues.class, (proxy, method, args) -> {
            checkThread();
            if (method.getName().equals("createPluginLifecycleEventManager"))
                return adapter(LifecycleEventManager.class, (p, m, a) -> { throw unsupported(m); });
            throw unsupported(method);
        });
        console = adapter(ConsoleCommandSender.class, this::consoleCall);
        commandMap = adapter(CommandMap.class, (proxy, method, args) -> {
            checkThread();
            if (method.getName().equals("getCommand")) return commands.get(((String) args[0]).toLowerCase(Locale.ROOT));
            throw unsupported(method);
        });
        server = adapter(Server.class, this::serverCall);
        Bukkit.setServer(server);
    }

    boolean stopped() { return stopped; }

    JsonObject execute(BigInteger id, JsonObject request) {
        checkThread();
        messages.asList().clear();
        messageBytes = 0;
        recipient = null;
        JsonObject response = new JsonObject();
        response.addProperty("id", id);
        String error = null;
        try {
            String op = Requests.string(request, "op", 16);
            switch (op) {
                case "load" -> {
                    Requests.fields(request, "id", "op", "plugins_dir");
                    if (loaded || stopped) throw new IllegalStateException("plugins may be loaded exactly once");
                    pluginsDirectory = Path.of(Requests.string(request, "plugins_dir", 4096)).toAbsolutePath().normalize();
                    loaded = true;
                    try { load(); active = true; }
                    catch (Throwable failure) {
                        try { disableAll(); }
                        catch (Throwable cleanup) { failure.addSuppressed(cleanup); }
                        throw failure;
                    }
                    response.add("plugins", metadata());
                }
                case "command" -> {
                    Requests.fields(request, "id", "op", "command", "args", "player");
                    requireReady();
                    String label = Requests.string(request, "command", 130).toLowerCase(Locale.ROOT);
                    if (!label.matches("[a-z0-9_\\-.]{1,64}(:[a-z0-9_\\-.]{1,64})?"))
                        throw new IllegalArgumentException("invalid command label");
                    String[] args = Requests.strings(request, "args", 64, 1024).toArray(String[]::new);
                    PlayerState player = player(Requests.player(request), false);
                    recipient = player.spec.uuid().toString();
                    dispatch(player.proxy, label, args);
                }
                case "join", "quit" -> {
                    Requests.fields(request, "id", "op", "player");
                    requireReady();
                    Requests.PlayerSpec spec = Requests.player(request);
                    recipient = spec.uuid().toString();
                    if (op.equals("join")) {
                        if (online.containsKey(spec.uuid())) throw new IllegalStateException("duplicate player join");
                        if (online.size() >= 1024) throw new IllegalStateException("online player limit exceeded (1024)");
                        PlayerState state = player(spec, true);
                        PlayerJoinEvent event = new PlayerJoinEvent(state.proxy, (Component) null);
                        error = fire(event);
                        broadcast(event.joinMessage());
                    } else {
                        PlayerState state = online.get(spec.uuid());
                        if (state == null) throw new IllegalStateException("quit for an unknown player");
                        state.spec = spec;
                        PlayerQuitEvent event = new PlayerQuitEvent(state.proxy, (Component) null, PlayerQuitEvent.QuitReason.DISCONNECTED);
                        try { error = fire(event); }
                        finally { online.remove(spec.uuid()); }
                        broadcast(event.quitMessage());
                    }
                }
                case "shutdown" -> {
                    Requests.fields(request, "id", "op");
                    close();
                }
                default -> throw new UnsupportedOperationException("unsupported operation: " + op);
            }
        } catch (Throwable failure) {
            Throwable cause = unwrap(failure);
            cause.printStackTrace(System.err);
            error = describe(cause);
        }
        response.addProperty("ok", error == null);
        response.add("messages", messages.deepCopy());
        if (error == null) response.add("error", JsonNull.INSTANCE);
        else response.addProperty("error", error);
        if (request.has("op") && "load".equals(request.get("op").isJsonPrimitive() ? request.get("op").getAsString() : "")
                && !response.has("plugins")) response.add("plugins", new JsonArray());
        return response;
    }

    private void load() throws Exception {
        List<PluginFiles.Spec> files = PluginFiles.scan(pluginsDirectory);
        Map<String, PluginClassLoader> byName = new HashMap<>();
        for (PluginFiles.Spec spec : files) {
            List<PluginClassLoader> dependencies = new ArrayList<>();
            List<String> declared = new ArrayList<>(spec.description().getDepend());
            declared.addAll(spec.description().getSoftDepend());
            for (String name : declared) {
                PluginClassLoader loader = byName.get(name.toLowerCase(Locale.ROOT));
                if (loader != null) dependencies.add(loader);
            }
            Path data = pluginsDirectory.resolve(spec.description().getName());
            Files.createDirectories(data);
            PluginClassLoader loader = new PluginClassLoader(spec, server, data, dependencies);
            loaders.add(loader);
            byName.put(spec.key(), loader);
            Class<?> main = loader.loadClass(spec.description().getMain());
            if (main.getClassLoader() != loader || !JavaPlugin.class.isAssignableFrom(main))
                throw new IllegalArgumentException("main must extend JavaPlugin in its own JAR: " + spec.key());
            JavaPlugin plugin = (JavaPlugin) main.getDeclaredConstructor().newInstance();
            plugins.put(spec.key(), plugin);
            registerCommands(plugin);
        }
        verifyMetadata();
        for (JavaPlugin plugin : plugins.values()) plugin.onLoad();
        for (JavaPlugin plugin : plugins.values()) plugin.setEnabled(true);
        verifyMetadata();
    }

    private void verifyMetadata() {
        if (new Gson().toJson(metadata()).getBytes(java.nio.charset.StandardCharsets.UTF_8).length > 24_576)
            throw new IllegalArgumentException("combined plugin/command metadata exceeds 24576 UTF-8 bytes");
    }

    private void registerCommands(JavaPlugin plugin) throws Exception {
        Constructor<PluginCommand> constructor = PluginCommand.class.getDeclaredConstructor(String.class, Plugin.class);
        constructor.setAccessible(true);
        for (var declaration : plugin.getDescription().getCommands().entrySet()) {
            String name = PluginFiles.label(declaration.getKey());
            PluginCommand command = constructor.newInstance(name, plugin);
            Map<String, Object> options = declaration.getValue();
            if (options.containsKey("description")) command.setDescription(PluginFiles.boundedText(options.get("description"), 512, "description"));
            if (options.containsKey("usage")) command.setUsage(PluginFiles.boundedText(options.get("usage"), 512, "usage"));
            if (options.containsKey("permission")) command.setPermission(PluginFiles.permissionNode(options.get("permission")));
            if (options.containsKey("permission-message")) command.setPermissionMessage(PluginFiles.boundedText(options.get("permission-message"), 512, "permission-message"));
            command.setAliases(PluginFiles.aliases(options.get("aliases")));
            List<String> labels = new ArrayList<>(command.getAliases());
            labels.add(name);
            for (String label : labels) {
                commands.put(label, command);
                commands.put(plugin.getName().toLowerCase(Locale.ROOT) + ":" + label, command);
            }
            if (!command.register(commandMap)) throw new IllegalStateException("command registration failed: " + name);
        }
    }

    private JsonArray metadata() {
        JsonArray result = new JsonArray();
        for (JavaPlugin plugin : plugins.values()) {
            JsonObject record = new JsonObject();
            record.addProperty("name", plugin.getName());
            record.addProperty("version", PluginFiles.boundedText(plugin.getDescription().getVersion(), 128, "plugin version"));
            JsonArray declared = new JsonArray();
            for (String name : plugin.getDescription().getCommands().keySet()) {
                PluginCommand command = commands.get(name.toLowerCase(Locale.ROOT));
                JsonObject info = new JsonObject();
                info.addProperty("name", command.getName());
                if (!command.getName().equals(PluginFiles.label(name))) throw new UnsupportedOperationException("dynamic command names are not supported");
                info.addProperty("description", PluginFiles.boundedText(command.getDescription(), 512, "command description"));
                info.addProperty("permission", command.getPermission() == null ? null : PluginFiles.permissionNode(command.getPermission()));
                JsonArray aliases = new JsonArray();
                if (!command.getAliases().equals(PluginFiles.aliases(plugin.getDescription().getCommands().get(name).get("aliases"))))
                    throw new UnsupportedOperationException("dynamic command aliases are not supported");
                for (String alias : command.getAliases()) aliases.add(PluginFiles.label(alias));
                info.add("aliases", aliases);
                declared.add(info);
            }
            record.add("commands", declared);
            result.add(record);
        }
        return result;
    }

    private Object serverCall(Object proxy, Method method, Object[] args) throws Throwable {
        checkThread();
        return switch (method.getName()) {
            case "getName" -> "FerrumC";
            case "getVersion" -> "FerrumC Java bridge 0.1.0 (Paper API 1.21.8 subset)";
            case "getBukkitVersion" -> "1.21.8-R0.1-SNAPSHOT";
            case "getMinecraftVersion" -> "1.21.8";
            case "getLogger" -> logger;
            case "getUnsafe" -> unsafe;
            case "getPluginManager" -> manager;
            case "getPluginsFolder" -> {
                if (pluginsDirectory == null) throw new IllegalStateException("plugins have not been loaded");
                yield pluginsDirectory.toFile();
            }
            case "getPluginCommand" -> commands.get(((String) args[0]).toLowerCase(Locale.ROOT));
            case "getCommandMap" -> commandMap;
            case "getConsoleSender" -> console;
            case "isPrimaryThread" -> true;
            case "dispatchCommand" -> {
                String[] tokens = ((String) args[1]).trim().split("\\s+", 65);
                yield dispatch((CommandSender) args[0], tokens[0].toLowerCase(Locale.ROOT), Arrays.copyOfRange(tokens, 1, tokens.length));
            }
            default -> throw unsupported(method);
        };
    }

    private Object managerCall(Object proxy, Method method, Object[] args) throws Exception {
        checkThread();
        return switch (method.getName()) {
            case "getPlugins" -> plugins.values().toArray(Plugin[]::new);
            case "getPlugin" -> plugins.get(((String) args[0]).toLowerCase(Locale.ROOT));
            case "isPluginEnabled" -> {
                Plugin plugin = args[0] instanceof Plugin value ? value : plugins.get(((String) args[0]).toLowerCase(Locale.ROOT));
                yield plugin != null && plugin.isEnabled();
            }
            case "registerEvents" -> { registerEvents((Listener) args[0], (Plugin) args[1]); yield null; }
            case "registerEvent" -> {
                @SuppressWarnings("unchecked") Class<? extends Event> type = (Class<? extends Event>) args[0];
                register(type, (Listener) args[1], (EventPriority) args[2], (EventExecutor) args[3], (Plugin) args[4], args.length == 6 && (Boolean) args[5]);
                yield null;
            }
            case "callEvent" -> {
                Event event = (Event) args[0];
                handlers(event.getClass());
                String error = fire(event);
                if (error != null) throw new IllegalStateException(error);
                yield null;
            }
            default -> throw unsupported(method);
        };
    }

    private void registerEvents(Listener listener, Plugin plugin) throws Exception {
        Set<String> signatures = new HashSet<>();
        for (Class<?> type = listener.getClass(); type != null; type = type.getSuperclass()) {
            Method[] methods = type.getDeclaredMethods();
            Arrays.sort(methods, Comparator.comparing(Method::toGenericString));
            for (Method method : methods) {
                // An override suppresses an inherited annotation just as Bukkit's method discovery does.
                String signature = method.getName() + Arrays.toString(method.getParameterTypes());
                if (!signatures.add(signature) || method.isBridge() || method.isSynthetic()) continue;
                EventHandler annotation = method.getAnnotation(EventHandler.class);
                if (annotation == null) continue;
                if (method.getParameterCount() != 1 || !Event.class.isAssignableFrom(method.getParameterTypes()[0]))
                    throw new IllegalArgumentException("invalid @EventHandler signature: " + method);
                @SuppressWarnings("unchecked") Class<? extends Event> eventType = (Class<? extends Event>) method.getParameterTypes()[0];
                method.setAccessible(true);
                register(eventType, listener, annotation.priority(), (target, event) -> {
                    try { method.invoke(target, event); }
                    catch (ReflectiveOperationException failure) { throw new EventException(unwrap(failure)); }
                }, plugin, annotation.ignoreCancelled());
            }
        }
    }

    private void register(Class<? extends Event> type, Listener listener, EventPriority priority, EventExecutor executor, Plugin plugin, boolean ignoreCancelled) {
        if (!plugins.containsValue(plugin)) throw new IllegalArgumentException("listener owner is not a loaded plugin");
        if (listeners.size() >= 512) throw new IllegalStateException("at most 512 event listeners supported");
        RegisteredListener registered = new RegisteredListener(listener, executor, priority, plugin, ignoreCancelled);
        handlers(type).register(registered);
        listeners.add(registered);
    }

    private HandlerList handlers(Class<?> type) {
        if (type == PlayerJoinEvent.class) return PlayerJoinEvent.getHandlerList();
        if (type == PlayerQuitEvent.class) return PlayerQuitEvent.getHandlerList();
        throw new UnsupportedOperationException("FerrumC Java bridge supports only PlayerJoinEvent and PlayerQuitEvent; requested " + type.getName());
    }

    private String fire(Event event) {
        String first = null;
        for (RegisteredListener registered : event.getHandlers().getRegisteredListeners()) {
            if (!registered.getPlugin().isEnabled()) continue;
            try { registered.callEvent(event); }
            catch (Throwable failure) {
                Throwable cause = unwrap(failure);
                registered.getPlugin().getLogger().log(java.util.logging.Level.SEVERE, "listener failed for " + event.getEventName(), cause);
                if (first == null) first = boundedError(registered.getPlugin().getName() + ": " + describe(cause));
            }
        }
        return first;
    }

    private boolean dispatch(CommandSender sender, String label, String[] args) {
        PluginCommand command = commands.get(label);
        if (command == null) throw new IllegalArgumentException("unknown plugin command: " + label);
        if (!command.getPlugin().isEnabled()) throw new IllegalStateException("command plugin is disabled");
        return command.execute(sender, label, args);
    }

    private PlayerState player(Requests.PlayerSpec spec, boolean joining) {
        PlayerState state = online.get(spec.uuid());
        if (state == null) state = new PlayerState(spec);
        else state.spec = spec;
        if (joining) online.put(spec.uuid(), state);
        return state;
    }

    private final class PlayerState {
        private Requests.PlayerSpec spec;
        private final Player proxy;
        PlayerState(Requests.PlayerSpec spec) { this.spec = spec; proxy = adapter(Player.class, this::call); }
        private Object call(Object target, Method method, Object[] args) {
            checkThread();
            return switch (method.getName()) {
                case "getName" -> spec.name();
                case "getUniqueId" -> spec.uuid();
                case "isOp" -> spec.op();
                case "hasPermission" -> permitted(spec, permission(args[0]));
                case "isPermissionSet" -> spec.permissions().contains(permission(args[0]));
                case "getServer" -> server;
                case "getPlayer" -> proxy;
                case "isOnline" -> online.containsKey(spec.uuid());
                case "sendMessage" -> { send(spec.uuid().toString(), args); yield null; }
                default -> throw unsupported(method);
            };
        }
    }

    private Object consoleCall(Object proxy, Method method, Object[] args) {
        checkThread();
        return switch (method.getName()) {
            case "getName" -> "CONSOLE";
            case "isOp", "hasPermission" -> true;
            case "isPermissionSet" -> false;
            case "getServer" -> server;
            case "sendMessage" -> { send(null, args); yield null; }
            default -> throw unsupported(method);
        };
    }

    private static String permission(Object value) {
        return (value instanceof Permission permission ? permission.getName() : (String) value).toLowerCase(Locale.ROOT);
    }
    private static boolean permitted(Requests.PlayerSpec spec, String permission) {
        if (spec.op() || spec.permissions().contains("*") || spec.permissions().contains(permission)) return true;
        for (int dot = permission.lastIndexOf('.'); dot > 0; dot = permission.lastIndexOf('.', dot - 1))
            if (spec.permissions().contains(permission.substring(0, dot) + ".*")) return true;
        return false;
    }

    private void broadcast(Component message) {
        if (message != null)
            throw new UnsupportedOperationException("join/quit message broadcasting is not implemented; event message changes cannot be applied to Rust yet");
    }

    private void send(String player, Object[] args) {
        // UUID/Identity arguments identify the source, while the recipient is this sender.
        boolean found = false;
        for (Object value : args) {
            if (value instanceof String text) { addMessage(player, text); found = true; }
            else if (value instanceof String[] texts) { for (String text : texts) addMessage(player, text); found = true; }
            else if (value instanceof Component text) { addMessage(player, PlainTextComponentSerializer.plainText().serialize(text)); found = true; }
            else if (value instanceof net.md_5.bungee.api.chat.BaseComponent[] texts) {
                addMessage(player, net.md_5.bungee.api.chat.BaseComponent.toPlainText(texts)); found = true;
            } else if (value instanceof net.md_5.bungee.api.chat.BaseComponent text) {
                addMessage(player, text.toPlainText()); found = true;
            }
        }
        if (!found) throw new UnsupportedOperationException("unsupported sendMessage overload/payload");
    }

    private void addMessage(String player, String text) {
        if (player != null && !player.equals(recipient))
            throw new UnsupportedOperationException("cross-player messaging is not implemented; only the current request player is supported");
        if (messages.size() >= 64 || text.length() > 1024 || text.indexOf('\0') >= 0
                || text.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > 4096
                || (messageBytes += text.getBytes(java.nio.charset.StandardCharsets.UTF_8).length) > 32_768)
            throw new IllegalStateException("plugin message limit exceeded (64 messages/1024 characters/32768 UTF-8 bytes)");
        JsonObject message = new JsonObject();
        message.addProperty("player", player);
        message.addProperty("text", text);
        messages.add(message);
    }

    private void requireReady() {
        if (!active || stopped) throw new IllegalStateException("plugins are not active");
    }

    private void checkThread() {
        if (Thread.currentThread() != owner) throw new UnsupportedOperationException("FerrumC bridge API must be called on the protocol thread; scheduler/async calls are not supported");
    }

    private static UnsupportedOperationException unsupported(Method method) {
        return new UnsupportedOperationException("FerrumC Java bridge does not implement " + method.getDeclaringClass().getSimpleName() + "." + method.getName());
    }

    private static <T> T adapter(Class<T> type, InvocationHandler handler) {
        return type.cast(Proxy.newProxyInstance(type.getClassLoader(), new Class<?>[] {type}, (proxy, method, args) -> {
            if (method.getDeclaringClass() == Object.class) return switch (method.getName()) {
                case "equals" -> proxy == args[0];
                case "hashCode" -> System.identityHashCode(proxy);
                case "toString" -> "FerrumC adapter for " + type.getSimpleName();
                default -> throw unsupported(method);
            };
            return handler.invoke(proxy, method, args == null ? new Object[0] : args);
        }));
    }

    private static Throwable unwrap(Throwable failure) {
        while ((failure instanceof InvocationTargetException || failure instanceof EventException || failure instanceof CommandException
                || failure instanceof ExceptionInInitializerError)
                && failure.getCause() != null) failure = failure.getCause();
        return failure;
    }
    private static String describe(Throwable failure) {
        return boundedError(failure.getClass().getSimpleName() + ": " + Objects.toString(failure.getMessage(), "no detail"));
    }
    private static String boundedError(String value) {
        String message = value.replace('\0', '\ufffd');
        int bytes = 0;
        int end = 0;
        while (end < message.length()) {
            int codePoint = message.codePointAt(end);
            int width = codePoint <= 0x7f ? 1 : codePoint <= 0x7ff ? 2 : codePoint <= 0xffff ? 3 : 4;
            if (bytes + width > 2048) break;
            bytes += width;
            end += Character.charCount(codePoint);
        }
        return message.substring(0, end);
    }

    private void disableAll() throws Exception {
        Exception error = null;
        List<JavaPlugin> reverse = new ArrayList<>(plugins.values());
        Collections.reverse(reverse);
        for (JavaPlugin plugin : reverse) {
            try { if (plugin.isEnabled()) plugin.setEnabled(false); }
            catch (Throwable failure) {
                if (error == null) error = new Exception("plugin disable failed: " + describe(unwrap(failure)));
                failure.printStackTrace(System.err);
            }
            HandlerList.unregisterAll(plugin);
        }
        listeners.clear();
        for (PluginClassLoader loader : loaders) {
            try { loader.close(); }
            catch (Exception failure) { if (error == null) error = failure; }
        }
        loaders.clear();
        commands.clear();
        if (error != null) throw error;
    }

    @Override public void close() throws Exception {
        checkThread();
        if (stopped) return;
        stopped = true;
        active = false;
        disableAll();
        online.clear();
    }
}
