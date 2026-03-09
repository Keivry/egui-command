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
    pub fn is_enabled(self) -> bool { self == CommandState::Enabled }

    pub fn is_visible(self) -> bool { self != CommandState::Hidden }
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
    pub fn new(id: CommandId, source: CommandSource) -> Self { Self { id, source } }
}
