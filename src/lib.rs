// SPDX-License-Identifier: MIT OR Apache-2.0

//! `egui-command` — pure command model, no egui dependency.
//!
//! Defines the core types for representing user-facing commands:
//! their identity, specification (metadata), state, and trigger events.
//!
//! # Architecture
//! ```text
//! egui-event  (typed event bus)
//!     ↓
//! egui-command  (this crate — command model)
//!     ↓
//! egui-command-binding  (egui integration: shortcut → CommandId)
//!     ↓
//! app  (AppCommand enum, business logic)
//! ```

/// Opaque command identifier.  Wrap an enum variant (or a `u32`) to make it
/// comparable and hashable without storing strings at runtime.
///
/// # Example
/// ```rust
/// use egui_command::CommandId;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum AppCmd {
///     ShowHelp,
///     RenameProfile,
/// }
///
/// let id = CommandId::new(AppCmd::ShowHelp);
/// assert_eq!(id, CommandId::new(AppCmd::ShowHelp));
/// assert_ne!(id, CommandId::new(AppCmd::RenameProfile));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandId(u64);

impl CommandId {
    /// Create a `CommandId` from any value that can be hashed.
    ///
    /// Uses `FxHasher` — a deterministic, platform-stable hasher — so that the
    /// same value always produces the same `CommandId` across process restarts,
    /// Rust versions, and platforms.
    pub fn new<T: std::hash::Hash>(value: T) -> Self {
        use {
            rustc_hash::FxHasher,
            std::hash::{BuildHasher, BuildHasherDefault},
        };
        Self(BuildHasherDefault::<FxHasher>::default().hash_one(value))
    }

    /// Raw numeric value.
    ///
    /// The underlying hash is stable within a build (same input → same output
    /// across runs, versions, and platforms when using the same `FxHasher`).
    /// Suitable for in-memory keying; treat persistence across binary upgrades
    /// with caution unless the hashed type's discriminant is stable.
    pub fn raw(self) -> u64 { self.0 }

    /// Construct from a raw value (e.g. round-tripping through an integer key).
    pub fn from_raw(v: u64) -> Self { Self(v) }
}

/// Human-readable metadata for a command.
///
/// Used by UI widgets (menu items, toolbar buttons, help overlays) to render
/// labels, tooltips, and shortcut hints without knowing about egui or input
/// handling.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    /// Stable identifier.
    pub id: CommandId,
    /// Short display label shown in menus / buttons.
    pub label: String,
    /// Optional longer description for tooltips / help text.
    pub description: Option<String>,
    /// Human-readable shortcut hint ("F2", "Ctrl+S", …).  Display-only;
    /// actual shortcut matching lives in `egui-command-binding`.
    pub shortcut_hint: Option<String>,
}

impl CommandSpec {
    /// Minimal constructor — just an id and a label.
    pub fn new(id: CommandId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            description: None,
            shortcut_hint: None,
        }
    }

    /// Builder: set description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Builder: set the shortcut hint string.
    pub fn with_shortcut_hint(mut self, hint: impl Into<String>) -> Self {
        self.shortcut_hint = Some(hint.into());
        self
    }
}

/// Runtime availability state of a command.
///
/// The app is responsible for computing and storing this; `egui-command-binding`
/// reads it to grey-out or hide menu items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandState {
    /// Normal — can be triggered.
    #[default]
    Enabled,
    /// Visually present but not actionable (greyed out).
    Disabled,
    /// Hidden from menus / toolbar.
    Hidden,
}

impl CommandState {
    /// Returns `true` if the command can currently be triggered (not disabled or hidden).
    pub fn is_enabled(self) -> bool { self == CommandState::Enabled }

    /// Returns `true` if the command should be shown in menus and toolbars.
    pub fn is_visible(self) -> bool { self != CommandState::Hidden }
}

/// Registry that maps command identifiers to their specs and runtime states.
///
/// `CommandRegistry<C>` is the single source of truth for all registered
/// commands in an application.  It stores the human-readable [`CommandSpec`]
/// and the runtime [`CommandState`] for each command, keyed by
/// [`CommandId`].
///
/// # Type Parameter
/// `C` is your application's command enum (or any `Copy + Hash + Eq` type
/// that can be converted into a [`CommandId`] via `Into<CommandId>`).
///
/// # Example
/// ```rust
/// use egui_command::{CommandId, CommandRegistry, CommandSpec, CommandState};
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum AppCmd {
///     Save,
///     Quit,
/// }
///
/// impl From<AppCmd> for CommandId {
///     fn from(c: AppCmd) -> Self { CommandId::new(c) }
/// }
///
/// let registry = CommandRegistry::new()
///     .with(
///         AppCmd::Save,
///         CommandSpec::new(CommandId::new(AppCmd::Save), "Save"),
///     )
///     .with(
///         AppCmd::Quit,
///         CommandSpec::new(CommandId::new(AppCmd::Quit), "Quit"),
///     );
///
/// assert!(registry.spec(AppCmd::Save).is_some());
/// assert_eq!(registry.state(AppCmd::Save), Some(CommandState::Enabled));
/// ```
#[derive(Debug, Default)]
pub struct CommandRegistry<C: Copy + std::hash::Hash + Eq + Into<CommandId>> {
    specs: std::collections::HashMap<CommandId, CommandSpec>,
    states: std::collections::HashMap<CommandId, CommandState>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Copy + std::hash::Hash + Eq + Into<CommandId>> CommandRegistry<C> {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            specs: std::collections::HashMap::new(),
            states: std::collections::HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Register a command with its spec, setting state to
    /// [`CommandState::Enabled`] if not already present.
    ///
    /// Returns `&mut Self` to allow chained `register` calls.
    ///
    /// # Panics
    ///
    /// Panics if `spec.id != cmd.into()` — i.e. the spec was built for a
    /// different command than the one being registered.
    pub fn register(&mut self, cmd: C, spec: CommandSpec) -> &mut Self {
        let id: CommandId = cmd.into();
        assert_eq!(
            spec.id, id,
            "CommandSpec::id does not match the registered command; \
             build the spec with CommandId::new(cmd) or CommandSpec::new(id, label)"
        );
        self.states.entry(id).or_insert(CommandState::Enabled);
        self.specs.insert(id, spec);
        self
    }

    /// Builder-style registration.  Consumes and returns `Self` so that
    /// registrations can be chained on construction:
    ///
    /// ```rust
    /// # use egui_command::{CommandId, CommandRegistry, CommandSpec};
    /// # #[derive(Clone, Copy, Hash, Eq, PartialEq)] enum C { A }
    /// # impl From<C> for CommandId { fn from(c: C) -> Self { CommandId::new(c) } }
    /// let reg = CommandRegistry::new().with(C::A, CommandSpec::new(CommandId::new(C::A), "A"));
    /// ```
    pub fn with(mut self, cmd: C, spec: CommandSpec) -> Self {
        self.register(cmd, spec);
        self
    }

    /// Look up the [`CommandSpec`] for a command.
    ///
    /// Returns `None` if the command was never registered.
    pub fn spec(&self, cmd: C) -> Option<&CommandSpec> { self.specs.get(&cmd.into()) }

    /// Look up the [`CommandSpec`] by raw [`CommandId`].
    pub fn spec_by_id(&self, id: CommandId) -> Option<&CommandSpec> { self.specs.get(&id) }

    /// Look up the current [`CommandState`] for a command.
    ///
    /// Returns `None` if the command was never registered.
    pub fn state(&self, cmd: C) -> Option<CommandState> { self.states.get(&cmd.into()).copied() }

    /// Look up the current [`CommandState`] by raw [`CommandId`].
    pub fn state_by_id(&self, id: CommandId) -> Option<CommandState> {
        self.states.get(&id).copied()
    }

    /// Update the runtime state of a registered command.
    ///
    /// Has no effect if the command has not been registered.
    pub fn set_state(&mut self, cmd: C, state: CommandState) {
        let id: CommandId = cmd.into();
        if self.specs.contains_key(&id) {
            self.states.insert(id, state);
        }
    }

    /// Update the runtime state by raw [`CommandId`].
    ///
    /// Has no effect if the id is not registered.
    pub fn set_state_by_id(&mut self, id: CommandId, state: CommandState) {
        if self.specs.contains_key(&id) {
            self.states.insert(id, state);
        }
    }

    /// Iterate over all registered `(CommandId, &CommandSpec)` pairs.
    pub fn iter_specs(&self) -> impl Iterator<Item = (CommandId, &CommandSpec)> {
        self.specs.iter().map(|(&id, spec)| (id, spec))
    }

    /// Mutable look up of a [`CommandSpec`] by raw [`CommandId`].
    ///
    /// Returns `None` if the id has not been registered.
    pub fn spec_by_id_mut(&mut self, id: CommandId) -> Option<&mut CommandSpec> {
        self.specs.get_mut(&id)
    }
}

/// What produced a `CommandTriggered` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    /// User pressed a keyboard shortcut.
    Keyboard,
    /// User clicked a menu item.
    Menu,
    /// User clicked a toolbar / context button.
    Button,
    /// Programmatically dispatched (e.g. from a test or macro-action).
    Programmatic,
}

/// Event emitted when a command is triggered.
///
/// The app receives a `Vec<CommandTriggered>` (or handles them one-by-one)
/// and converts them into domain `AppCommand` variants.
#[derive(Debug, Clone)]
pub struct CommandTriggered {
    /// Which command fired.
    pub id: CommandId,
    /// How it was triggered.
    pub source: CommandSource,
}

impl CommandTriggered {
    /// Creates a `CommandTriggered` event from a command id and its trigger source.
    pub fn new(id: CommandId, source: CommandSource) -> Self { Self { id, source } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum AppCmd {
        ShowHelp,
        Save,
        Quit,
    }

    #[test]
    fn command_id_same_value_is_equal() {
        let a = CommandId::new(AppCmd::ShowHelp);
        let b = CommandId::new(AppCmd::ShowHelp);
        assert_eq!(a, b);
    }

    #[test]
    fn command_id_different_variants_are_not_equal() {
        let a = CommandId::new(AppCmd::Save);
        let b = CommandId::new(AppCmd::Quit);
        assert_ne!(a, b);
    }

    #[test]
    fn command_id_raw_roundtrip() {
        let id = CommandId::new(AppCmd::Save);
        assert_eq!(CommandId::from_raw(id.raw()), id);
    }

    #[test]
    fn command_id_hashable_in_map() {
        let mut map = std::collections::HashMap::new();
        map.insert(CommandId::new(AppCmd::ShowHelp), "help");
        map.insert(CommandId::new(AppCmd::Save), "save");
        assert_eq!(map[&CommandId::new(AppCmd::ShowHelp)], "help");
        assert_eq!(map[&CommandId::new(AppCmd::Save)], "save");
    }

    #[test]
    fn command_spec_builder_chain() {
        let id = CommandId::new(AppCmd::Save);
        let spec = CommandSpec::new(id, "Save")
            .with_description("Save the current file")
            .with_shortcut_hint("Ctrl+S");
        assert_eq!(spec.id, id);
        assert_eq!(spec.label, "Save");
        assert_eq!(spec.description.as_deref(), Some("Save the current file"));
        assert_eq!(spec.shortcut_hint.as_deref(), Some("Ctrl+S"));
    }

    #[test]
    fn command_spec_minimal_has_no_optional_fields() {
        let spec = CommandSpec::new(CommandId::new(AppCmd::Quit), "Quit");
        assert_eq!(spec.label, "Quit");
        assert!(spec.description.is_none());
        assert!(spec.shortcut_hint.is_none());
    }

    #[test]
    fn command_state_is_enabled() {
        assert!(CommandState::Enabled.is_enabled());
        assert!(!CommandState::Disabled.is_enabled());
        assert!(!CommandState::Hidden.is_enabled());
    }

    #[test]
    fn command_state_is_visible() {
        assert!(CommandState::Enabled.is_visible());
        assert!(CommandState::Disabled.is_visible());
        assert!(!CommandState::Hidden.is_visible());
    }

    #[test]
    fn command_state_default_is_enabled() {
        assert_eq!(CommandState::default(), CommandState::Enabled);
    }

    #[test]
    fn command_triggered_stores_id_and_source() {
        let id = CommandId::new(AppCmd::Save);
        let triggered = CommandTriggered::new(id, CommandSource::Keyboard);
        assert_eq!(triggered.id, id);
        assert_eq!(triggered.source, CommandSource::Keyboard);
    }

    #[test]
    fn command_source_variants_are_distinct() {
        assert_ne!(CommandSource::Keyboard, CommandSource::Menu);
        assert_ne!(CommandSource::Button, CommandSource::Programmatic);
    }

    impl From<AppCmd> for CommandId {
        fn from(c: AppCmd) -> Self { CommandId::new(c) }
    }

    fn make_spec(cmd: AppCmd, label: &str) -> CommandSpec {
        CommandSpec::new(CommandId::new(cmd), label)
    }

    #[test]
    fn registry_register_and_query_spec() {
        let mut reg = CommandRegistry::new();
        reg.register(AppCmd::Save, make_spec(AppCmd::Save, "Save"));
        assert!(reg.spec(AppCmd::Save).is_some());
        assert_eq!(reg.spec(AppCmd::Save).unwrap().label, "Save");
    }

    #[test]
    fn registry_unregistered_returns_none() {
        let reg: CommandRegistry<AppCmd> = CommandRegistry::new();
        assert!(reg.spec(AppCmd::Quit).is_none());
        assert!(reg.state(AppCmd::Quit).is_none());
    }

    #[test]
    fn registry_default_state_is_enabled() {
        let mut reg = CommandRegistry::new();
        reg.register(AppCmd::Save, make_spec(AppCmd::Save, "Save"));
        assert_eq!(reg.state(AppCmd::Save), Some(CommandState::Enabled));
    }

    #[test]
    fn registry_set_state_updates_value() {
        let mut reg = CommandRegistry::new();
        reg.register(AppCmd::Save, make_spec(AppCmd::Save, "Save"));
        reg.set_state(AppCmd::Save, CommandState::Disabled);
        assert_eq!(reg.state(AppCmd::Save), Some(CommandState::Disabled));
    }

    #[test]
    fn registry_set_state_unregistered_is_noop() {
        let mut reg: CommandRegistry<AppCmd> = CommandRegistry::new();
        reg.set_state(AppCmd::Quit, CommandState::Hidden);
        assert!(reg.state(AppCmd::Quit).is_none());
    }

    #[test]
    fn registry_builder_chain() {
        let reg = CommandRegistry::new()
            .with(AppCmd::ShowHelp, make_spec(AppCmd::ShowHelp, "Help"))
            .with(AppCmd::Save, make_spec(AppCmd::Save, "Save"))
            .with(AppCmd::Quit, make_spec(AppCmd::Quit, "Quit"));
        assert!(reg.spec(AppCmd::ShowHelp).is_some());
        assert!(reg.spec(AppCmd::Save).is_some());
        assert!(reg.spec(AppCmd::Quit).is_some());
    }

    #[test]
    fn registry_register_id_mismatch_panics() {
        let mut reg = CommandRegistry::new();
        let wrong_id = CommandId::new(AppCmd::Quit);
        let spec = CommandSpec::new(wrong_id, "Save");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            reg.register(AppCmd::Save, spec);
        }));
        assert!(result.is_err(), "expected panic on id mismatch");
    }

    #[test]
    fn registry_iter_specs_covers_all_registered() {
        let reg = CommandRegistry::new()
            .with(AppCmd::Save, make_spec(AppCmd::Save, "Save"))
            .with(AppCmd::Quit, make_spec(AppCmd::Quit, "Quit"));
        let ids: Vec<CommandId> = reg.iter_specs().map(|(id, _)| id).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&CommandId::new(AppCmd::Save)));
        assert!(ids.contains(&CommandId::new(AppCmd::Quit)));
    }

    #[test]
    fn registry_spec_by_id() {
        let mut reg = CommandRegistry::new();
        reg.register(AppCmd::Save, make_spec(AppCmd::Save, "Save"));
        let id = CommandId::new(AppCmd::Save);
        assert!(reg.spec_by_id(id).is_some());
        assert_eq!(reg.spec_by_id(id).unwrap().label, "Save");
    }
}
