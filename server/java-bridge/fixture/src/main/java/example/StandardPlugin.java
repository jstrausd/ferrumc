package example;

import java.util.Objects;
import org.bukkit.Bukkit;
import org.bukkit.command.Command;
import org.bukkit.command.CommandSender;
import org.bukkit.entity.Player;
import org.bukkit.event.*;
import org.bukkit.event.player.*;
import org.bukkit.plugin.java.JavaPlugin;

/** Ordinary Bukkit plugin: no FerrumC imports, custom interfaces, or transformed bytecode. */
public final class StandardPlugin extends JavaPlugin implements Listener {
    private Player lastJoined;

    @Override public void onLoad() {
        System.out.println("fixture stdout onLoad");
        getLogger().info("loaded " + getName() + " " + getDescription().getVersion());
    }

    @Override public void onEnable() {
        saveDefaultConfig();
        getConfig().set("enabled", getConfig().getInt("enabled") + 1);
        saveConfig();
        if (getConfig().getBoolean("fail-enable")) Bukkit.getScheduler();
        Objects.requireNonNull(getCommand("bridgefixture"), "plugin.yml command").setExecutor(this);
        getServer().getPluginManager().registerEvents(this, this);
        if (Bukkit.getPluginCommand("bfixture") != getCommand("bridgefixture"))
            throw new IllegalStateException("alias lookup must return the same command");
        System.out.println("fixture stdout onEnable");
    }

    @Override public void onDisable() {
        getConfig().set("disabled", getConfig().getInt("disabled") + 1);
        saveConfig();
        System.out.println("fixture stdout onDisable");
    }

    @Override public boolean onCommand(CommandSender sender, Command command, String label, String[] args) {
        System.out.println("fixture stdout command");
        if (args.length != 0) {
            switch (args[0]) {
                case "world" -> { ((Player) sender).getWorld(); }
                case "scheduler" -> { Bukkit.getScheduler(); }
                case "fail" -> throw new IllegalStateException("fixture command failed");
                case "unicode-error" -> throw new IllegalStateException("🌍".repeat(1000) + "\0");
                case "previous" -> { Objects.requireNonNull(lastJoined).sendMessage("previous player"); return true; }
                case "unicode" -> { sender.sendMessage("Grüße 🌍"); return true; }
                case "unicode-bmp" -> { sender.sendMessage("Grüße Ω"); return true; }
                case "large" -> { sender.sendMessage("x".repeat(1025)); return true; }
                case "spam" -> { for (int i = 0; i < 80; i++) sender.sendMessage("spam:" + i); return true; }
                default -> { /* Normal command report below. */ }
            }
        }
        Player player = (Player) sender;
        sender.sendMessage("fixture:" + label + ":" + sender.getName() + ":" + player.getUniqueId()
                + ":" + sender.isOp() + ":" + sender.hasPermission("fixture.use")
                + ":" + Bukkit.getName() + ":" + getDescription().getVersion());
        return true;
    }

    @EventHandler(priority = EventPriority.MONITOR, ignoreCancelled = true)
    private void joinMonitor(PlayerJoinEvent event) {
        event.getPlayer().sendMessage("join:monitor");
    }

    @EventHandler(priority = EventPriority.LOWEST)
    private void joinLowest(PlayerJoinEvent event) {
        getConfig().set("join-count", getConfig().getInt("join-count") + 1);
        saveConfig();
        lastJoined = event.getPlayer();
        event.getPlayer().sendMessage("join:lowest");
        if (event.getPlayer().getName().equals("Faulty")) throw new IllegalStateException("fixture listener failed");
        if (event.getPlayer().getName().equals("Broadcaster")) event.setJoinMessage("requested broadcast");
    }

    @EventHandler
    public void quit(PlayerQuitEvent event) {
        getConfig().set("quit-count", getConfig().getInt("quit-count") + 1);
        saveConfig();
        event.getPlayer().sendMessage("quit:fixture");
    }
}
