# FerrumC Java bridge: first supported API subset

This module executes **unmodified standard `plugin.yml` JARs** in a dedicated
Java 21 process. It uses the official Paper API
`1.21.8-R0.1-20250906.215025-55` and FerrumC's own server adapter. It contains
no Paper server implementation and does not provide general Paper compatibility.

The separately compiled fixture is an ordinary `JavaPlugin`, with only Paper
API and Java standard-library imports. It proves the API subset listed here,
not compatibility with arbitrary external plugins. NMS/CraftBukkit, packet
hooks, world APIs, inventories, scheduling and Paper bootstrap plugins need
separate implementation and acceptance tests.

## Build and test

With JDK 21 and Maven 3.9 available:

```sh
cd server/java-bridge
mvn -B clean verify
```

Artifacts:

- Executable: `runtime/target/ferrumc-java-bridge-0.1.0.jar`
- Bridge sources: `runtime/target/ferrumc-java-bridge-0.1.0-sources.jar`
- Pinned upstream API sources: `runtime/target/third-party-sources/paper-api-sources.jar`
- Normal plugin fixture: `fixture/target/standard-bukkit-fixture-0.1.0.jar`
- Rollback fixture: `fixture/target/standard-bukkit-fixture-0.1.0-collision.jar`
  (same ordinary plugin bytecode, with a declared `spawn` alias for Rust's builtin-collision gate).

The executable's main class is `io.ferrumc.bridge.BridgeMain`. Run it with
`java -jar <executable>` and **no CLI arguments**. Rust owns startup and the
stdin/stdout pipes; there is no TCP listener or JNI layer. `clean verify`
executes malformed-input unit tests and real subprocess integrations covering
lifecycle, permissions, aliases, listener priorities, failures, shutdown,
UTF-8 limits and restart persistence. Integration tests require packaged JARs
and run in Maven's `verify` lifecycle.

## Implemented behavior

- Standard `JavaPlugin` construction through a configured URL class loader;
  public Paper `init`, `onLoad`, `setEnabled(true)` / `onEnable`, and
  `setEnabled(false)` / `onDisable`. Shutdown, EOF and failed enable rollbacks
  invoke disable at most once for each enabled instance, in reverse load order.
- Plugin name/version/description/meta, logger, data folder, resources and the
  API's real YAML configuration implementation, including save/reload and
  default configuration persistence.
- Declared `PluginCommand` objects and their executors, descriptions, usage,
  permissions, labels and aliases. Both plain and `plugin:command` labels route
  to the same object. `getCommand`, `Bukkit.getPluginCommand`, and command-map
  lookup work. Runtime command registration and dynamic name/alias changes are
  unsupported. Mutations during startup are validated before metadata export.
- Plugin argument completion invokes the real `PluginCommand.tabComplete`:
  explicit `TabCompleter`, then executor completion when the explicit callback
  returns `null`. Rust declares `minecraft:ask_server` on these argument nodes
  and checks the command permission before sending a callback. Aliases and
  namespaces, original alias casing, current sender context, interior/trailing
  empty arguments from Bukkit's literal-space split and UTF-16
  replacement ranges are supported. Default online-player-name completion
  remains unsupported: a callback returning `null` through both paths reports
  a concrete unsupported API. Suggestions have no hover tooltip.
- The current request's `Player` / `CommandSender` name, UUID, operator status,
  `hasPermission`, `isPermissionSet`, `getServer` and `sendMessage`; strings,
  string arrays and text components return bounded text messages. Components
  currently serialize as plain text; rich formatting is not carried by this
  protocol. A console sender is available for console text output.
- `Server` / `Bukkit` name, version, Minecraft/API version, logger, plugin
  manager, plugin folder, plugin-command/command-map lookup, console sender,
  `isPrimaryThread` and plugin-command dispatch.
- Real `PlayerJoinEvent` and `PlayerQuitEvent` instances, real `HandlerList`
  and `RegisteredListener`, `@EventHandler` discovery and `registerEvent`.
  Priorities run `LOWEST` through `MONITOR`; the API's `ignoreCancelled` rules
  apply. These two events are not cancellable. Private handlers are supported;
  listener failures are reported while subsequent listeners still run.

Permission context comes from Rust. Explicit nodes, `*`, hierarchical `x.*`
grants and operators allow access; there is no implemented permission
attachment/default/child graph. Requests update the current player's context.
A `player_state` RPC replaces the permission snapshot of an already joined
player on the Java owner thread, preserving the same proxy object and emitting
no join/quit events. Plugin-held proxies therefore see the refreshed grants.
Unknown online identities, identity changes and malformed snapshots are refused.
This is a Java-side prerequisite: Rust operator mutations with Java plugins
still require synchronized refresh/acknowledgement and failure handling before
activation. The RPC alone does not establish that cross-process contract.
Command permissions must use a simple dotted ASCII node or a final wildcard;
Bukkit's semicolon OR expressions and empty nodes are explicitly unsupported.
The player roster only pairs join/quit identities, with a maximum of 1024;
`getOnlinePlayers`, cross-player lookup and messaging are unsupported.

Join/quit event messages start as `null`. Setting a non-null join/quit message
currently returns an explicit unsupported-broadcast error after listener
execution. Sending text to the event's current player works.
`Bukkit.broadcastMessage` is unsupported. This is **not** complete Bukkit join/
quit broadcast behavior.

Every unimplemented server/player/plugin-manager/unsafe/lifecycle API throws
`UnsupportedOperationException` with the requested API name. Scheduler and
world objects are never fabricated. All bridge API calls must run on the
protocol thread; plugin-created asynchronous threads may log but cannot invoke
bridge APIs. Plugin code is trusted and runs without an OS sandbox. It can
access files and start threads; the host process's resource limits remain
necessary. Normal `System.out` plugin output is redirected to stderr before
loading; the original stdout descriptor belongs exclusively to framed RPC.
A plugin deliberately writing `FileDescriptor.out` can violate this trusted
process contract and will be rejected by Rust's frame decoder.

## Load preflight and limits

Preflight scans every JAR before running any plugin constructor or callback.
Plugin names are case-insensitively unique. Missing hard dependencies, cyclic
load order and command/alias collisions are errors. Hard and present soft
dependencies/load-before edges determine deterministic load order. Only declared
dependency classes are shared; there is no global unrelated-plugin class pool.

`paper-plugin.yml` bootstrap/loading and nonempty `plugin.yml` `libraries`
are explicitly unsupported. Bundle libraries in the standard plugin JAR.
`api-version`, if present, must be `1.21` through `1.21.8`. There is no legacy
NMS remapping. The exact timestamped API is pinned rather than a floating
snapshot; Maven obtains it from the official Paper repository.

| Boundary | Limit |
| --- | --- |
| Frame body | 1–65,536 UTF-8 bytes |
| JSON tree | depth 16; 4096 nodes; duplicate fields rejected |
| Plugin JARs | 64; each regular file at most 64 MiB |
| ZIP | 4096 unique entries; each at most 16 MiB; total expanded size at most 128 MiB |
| `plugin.yml` | 16,384 bytes; nesting 16; duplicate keys and collection aliases rejected |
| Plugin name | 64 ASCII letters/digits/underscore/hyphen |
| Plugin version | 128 UTF-8 bytes |
| Commands | 128 per plugin; 256 total |
| Command/alias label | 64 lowercase ASCII letters/digits/underscore/hyphen/dot |
| Aliases | 16 per command; 256 total |
| Description/usage | 512 UTF-8 bytes |
| Permission | 128 UTF-8 bytes |
| Combined exported metadata | 24,576 serialized UTF-8 bytes, checked before and after lifecycle |
| Listeners | 512 total |
| Request args | 64 strings; each 1024 UTF-16 code units |
| Permission request context | 256 nodes; each 128 UTF-16 code units, restricted ASCII |
| Callback output | 64 messages; each 1024 UTF-16 code units and at most 4096 UTF-8 bytes; 32,768 total text bytes |
| Error | 2048 UTF-8 bytes, truncated at code-point boundary; NUL sanitized |
| Completions | 64 strings; each at most 256 UTF-8 bytes; 16,384 serialized UTF-8 bytes total |

NUL is rejected in metadata and outgoing text. Oversized output becomes an
explicit bounded error response; it cannot become an oversized frame.
Completion strings also reject null entries and unpaired UTF-16 surrogates.
The combined response is checked after JSON escaping; oversized callback output
returns a correlated error with empty messages/completions, keeping the next
request usable.

## Exact RPC schema

A frame is a 4-byte **big-endian unsigned length**, followed by exactly that
many UTF-8 bytes of one strict JSON object. No newline delimiter is used.
Each request requires a nonnegative integral numeric `id` from `0` through
`18446744073709551615`. Unknown fields and operations return a correlated error.
Malformed framing/JSON, duplicate JSON fields or an invalid/missing ID terminate
the process with a stderr diagnostic because safe correlation is impossible.
EOF performs the normal disable/close path.

Player fields are required in command, complete, join and quit requests:

```json
{"name":"Tester","uuid":"12345678-1234-5678-1234-567812345678","op":false,"permissions":["fixture.use"]}
```

Names use the normal 1–16 ASCII Minecraft-name characters. UUIDs use canonical
36-character representation. `permissions` is an array of strings, not an
attachment graph. `op` is a JSON boolean.

Requests:

```json
{"id":1,"op":"load","plugins_dir":"/absolute/path/to/plugins"}
{"id":2,"op":"command","command":"bfixture","args":[],"player":{"name":"Tester","uuid":"12345678-1234-5678-1234-567812345678","op":false,"permissions":["fixture.use"]}}
{"id":3,"op":"join","player":{"name":"Tester","uuid":"12345678-1234-5678-1234-567812345678","op":false,"permissions":[]}}
{"id":4,"op":"quit","player":{"name":"Tester","uuid":"12345678-1234-5678-1234-567812345678","op":false,"permissions":[]}}
{"id":5,"op":"shutdown"}
{"id":6,"op":"complete","command":"bfixture","args":["probe",""],"player":{"name":"Tester","uuid":"12345678-1234-5678-1234-567812345678","op":false,"permissions":["fixture.use"]}}
```

`load` runs exactly once and creates a missing plugin directory. All JARs are
validated before lifecycle. A failed load returns an empty plugin array and
rolls back already-enabled plugins. Further command/event requests reject an
inactive bridge. Command matching is case-insensitive; the executor receives
the matched lowercase label. Commands/events require successful load. A quit
requires a prior join for the same UUID; duplicate joins are rejected.
`complete` requires at least one argument, including an empty current token after
trailing whitespace. Permission denial yields an empty successful completion
response without invoking a plugin. Callback failures yield an error and empty
completions. Completion responses require the `completions` array; it is absent
on other operations.

```json
{"id":6,"ok":true,"messages":[],"completions":["alpha","beta"],"error":null}
```

Successful load response:

```json
{"id":1,"ok":true,"plugins":[{"name":"BridgeFixture","version":"1.0.0","commands":[{"name":"bridgefixture","description":"Report the sender and command label","permission":"fixture.use","aliases":["bfixture"]}]}],"messages":[],"error":null}
```

`permission` is explicitly `null` when absent. `aliases`, `commands`, `plugins`
and `messages` are arrays, including when empty. Namespaced command labels are
derived from plugin metadata and are not additional metadata entries.

Command/event/shutdown response:

```json
{"id":2,"ok":true,"messages":[{"player":"12345678-1234-5678-1234-567812345678","text":"hello"}],"error":null}
```

`messages[].player` is the current request player's **UUID**, never their name;
`null` denotes console output. Rust remains the message-route owner. Messages
to another player explicitly fail. Callback errors can retain earlier bounded
messages and set `ok:false`. Rust must check `ok` and decide its delivery policy.

```json
{"id":2,"ok":false,"messages":[],"error":"UnsupportedOperationException: FerrumC Java bridge does not implement Entity.getWorld"}
```

A load error also contains `"plugins":[]`. Plugin permission denial is a normal
command result with its permission message and `ok:true`; an unknown command or
thrown executor/listener exception is `ok:false`. `shutdown` invokes disable,
writes the final response, then exits. EOF also disables once. The main method
exits even if a plugin left non-daemon threads running.

## Dependency licensing and sources

FerrumC bridge source follows the repository's MIT license. The executable
also contains unmodified upstream API/dependency classes with their own
licenses. In particular, Paper inherits GPLv3 from Bukkit/Spigot, with MIT
exceptions for listed contributors. It is not accurate to label the entire
shaded executable as MIT-only.

The build preserves each dependency's available LICENSE/NOTICE files under
`META-INF/third-party/<artifact>/`, plus Paper's complete licensing notice,
GPLv3 and MIT texts under `META-INF/third-party/paper/`. The accompanying bridge
sources JAR and this repository provide the adapter sources. Paper's exact API
sources are available from the same official repository with the `sources`
classifier:

<https://repo.papermc.io/repository/maven-public/io/papermc/paper/paper-api/1.21.8-R0.1-SNAPSHOT/paper-api-1.21.8-R0.1-20250906.215025-55-sources.jar>

`package` copies that exact source artifact to
`runtime/target/third-party-sources/paper-api-sources.jar`; distribute it alongside
the bridge sources and this module's source/POM files. Reproduce the bridge by
building those files with Java 21 and `mvn -B clean verify`; the parent POM pins
the API and build-plugin versions, and dependency notices remain inside the
resulting shaded executable. The source-only upstream artifact is not loaded
at runtime.

See `runtime/src/main/resources/META-INF/third-party/DEPENDENCIES.md` for
dependency provenance. No Paper server JAR or server implementation is loaded.
