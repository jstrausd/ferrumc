# ferrumc-command

A Brigadier-lite command system: a tree of literal and argument nodes with
parsing, tab-completion suggestions, and dispatch that delegates the actual
permission check to the calling layer.

Depends on `ferrumc-core` and `ferrumc-codec`. It does not know how permissions are stored or
evaluated: a node declares a required permission *level* and/or an opaque
permission *node string*, and the caller supplies the check — a permission level
on the `CommandSource` (compared during `dispatch`), or an `is_allowed` closure
passed to `dispatch_with`.

## Example

```rust
use ferrumc_command::{argument, literal, ArgumentType, CommandResult, CommandSource, CommandTree};
use ferrumc_core::TextComponent;

let mut tree = CommandTree::new();
tree.register(
    literal("gamemode")
        .requires_level(2)
        .then(argument("mode", ArgumentType::integer(0, 3)).executes(|ctx| {
            let mode = ctx.integer("mode").unwrap_or_default();
            CommandResult::success(TextComponent::text(format!("gamemode set to {mode}")))
        })),
);

let op = CommandSource::console(4);
let result = tree.dispatch("gamemode 1", &op);
assert!(matches!(&result, Ok(outcome) if outcome.is_success()));

// Suggestions complete the next token.
assert_eq!(tree.suggest("game"), vec!["gamemode".to_string()]);
```

Argument builders can declare server-driven completion with
`argument("args", ArgumentType::GreedyString).suggests_ask_server()`. The lowered
Brigadier graph then sets the custom-suggestions flag and encodes the vanilla
`minecraft:ask_server` provider after the argument properties. The application
handles the client's completion request; this declaration does not invoke a
provider or change parsing and dispatch. Permission filtering preserves the
metadata only for arguments the player may reach.

## Invariants

See `INVARIANTS.md` in this directory.
