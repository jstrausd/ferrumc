package io.ferrumc.bridge;

import io.papermc.paper.ServerBuildInfo;
import java.time.Instant;
import java.util.Optional;
import java.util.OptionalInt;
import net.kyori.adventure.key.Key;

/** Required by Bukkit.setServer; describes this bridge without claiming the Paper server brand. */
public final class BridgeBuildInfo implements ServerBuildInfo {
    /** Public constructor required by ServiceLoader. */
    public BridgeBuildInfo() {}
    @Override public Key brandId() { return Key.key("ferrumc", "java_bridge"); }
    @Override public boolean isBrandCompatible(Key brand) { return brandId().equals(brand); }
    @Override public String brandName() { return "FerrumC Java bridge"; }
    @Override public String minecraftVersionId() { return "1.21.8"; }
    @Override public String minecraftVersionName() { return "1.21.8"; }
    @Override public OptionalInt buildNumber() { return OptionalInt.empty(); }
    @Override public Instant buildTime() { return Instant.parse("2026-10-10T00:00:00Z"); }
    @Override public Optional<String> gitBranch() { return Optional.empty(); }
    @Override public Optional<String> gitCommit() { return Optional.empty(); }
    @Override public String asString(StringRepresentation representation) { return "1.21.8-bridge-0.1.0 (Paper API subset)"; }
}
