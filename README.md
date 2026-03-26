# egui-command

`egui-command` is the pure command-model layer used by SAide's egui stack.
It provides stable command identifiers, display metadata, runtime availability
state, and trigger payload types without depending on egui itself.

## Core types

- `CommandId` — stable hash-backed identifier for an app-defined command value
- `CommandSpec` — user-facing metadata such as label, description, and shortcut hint
- `CommandState` — runtime availability (`Enabled`, `Disabled`, `Hidden`)
- `CommandRegistry<C>` — registry for command specs and states keyed by `CommandId`
- `CommandTriggered` / `CommandSource` — payloads for command dispatch systems

## `CommandRegistry` quick start

```rust
use egui_command::{CommandId, CommandRegistry, CommandSpec, CommandState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AppCommand {
    ShowHelp,
    ToggleRecording,
}

impl From<AppCommand> for CommandId {
    fn from(value: AppCommand) -> Self { CommandId::new(value) }
}

let mut registry = CommandRegistry::new()
    .with(
        AppCommand::ShowHelp,
        CommandSpec::new(CommandId::new(AppCommand::ShowHelp), "Show help")
            .with_description("Open the help dialog"),
    )
    .with(
        AppCommand::ToggleRecording,
        CommandSpec::new(CommandId::new(AppCommand::ToggleRecording), "Record")
            .with_shortcut_hint("F9"),
    );

assert_eq!(registry.state(AppCommand::ShowHelp), Some(CommandState::Enabled));
assert_eq!(registry.spec(AppCommand::ToggleRecording).unwrap().label, "Record");

registry.set_state(AppCommand::ToggleRecording, CommandState::Disabled);
assert_eq!(registry.state(AppCommand::ToggleRecording), Some(CommandState::Disabled));

let id = CommandId::new(AppCommand::ShowHelp);
assert_eq!(registry.spec_by_id(id).unwrap().description.as_deref(), Some("Open the help dialog"));
```

## API summary

### Register commands

- `CommandRegistry::new()` creates an empty registry
- `register(cmd, spec)` inserts or replaces a command spec
- `with(cmd, spec)` is the builder-style equivalent for chained construction

### Query metadata and state

- `spec(cmd)` / `spec_by_id(id)` return `Option<&CommandSpec>`
- `state(cmd)` / `state_by_id(id)` return `Option<CommandState>`
- `iter_specs()` iterates over all registered command specs

### Update runtime state

- `set_state(cmd, state)` updates a registered command's state
- `set_state_by_id(id, state)` performs the same update via raw `CommandId`
- `spec_by_id_mut(id)` allows in-place metadata edits such as filling shortcut hints

`egui-command-binding` integrates with this registry via
`ShortcutManager::fill_shortcut_hints`, allowing the egui input layer to keep
display-only shortcut text synchronized with registered commands.
