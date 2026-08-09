//! Podium UI state — podium_state.toml.
//!
//! Handles reading and writing `%APPDATA%\podium\podium_state.toml`, which
//! persists UI state across launches: the last active project and per-project
//! dock open/close state and active panel.
//!
//! This is separate from `config.rs` (projects.toml, kb_sources.toml) because
//! it changes on every session whereas project config changes only when the
//! user explicitly edits a project. Keeping them separate avoids unnecessary
//! writes to the authoritative project config on every dock interaction.
//!
//! ## File location
//!
//! `%APPDATA%\podium\podium_state.toml`
//!
//! ## Schema
//!
//! ```toml
//! last_project = "showflyer"
//!
//! [dock_state.showflyer]
//! left_visible = true
//! left_active_panel = "files-panel"
//! bottom_visible = false
//! bottom_active_panel = ""
//! right_visible = false
//! right_active_panel = ""
//! ```
//!
//! ## Design notes
//!
//! - Dock state is keyed by project id so each project remembers its own
//!   layout independently (ADR-027).
//! - Panel identity in `*_active_panel` uses the panel's `name()` string
//!   from the `PodiumPanel` trait. Empty string means no panel active.
//! - The dock owns panel persistence — panels are storage-agnostic (ADR-027).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::podium_config_dir;

// ---------------------------------------------------------------------------
// Path helper
// ---------------------------------------------------------------------------

/// Returns the path to `podium_state.toml`.
pub fn podium_state_toml_path() -> PathBuf {
    podium_config_dir().join("podium_state.toml")
}

// ---------------------------------------------------------------------------
// PodiumState — top-level container
// ---------------------------------------------------------------------------

/// Root of `podium_state.toml`. Persists UI state across launches.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PodiumState {
    /// ID of the last active project. Restored on next launch.
    /// `None` on first launch or if the last session had no project loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_project: Option<String>,

    /// Per-project dock state, keyed by project id (ADR-027).
    /// Each project remembers its own dock layout independently.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub dock_state: HashMap<String, DockState>,
}

impl PodiumState {
    /// Read `podium_state.toml` from disk. Returns a default empty state if
    /// the file does not exist yet (first launch).
    pub fn load() -> Result<Self> {
        let path = podium_state_toml_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))
    }

    /// Write `podium_state.toml` to disk, creating the config directory if
    /// needed. Called on project unload and on clean shutdown.
    pub fn save(&self) -> Result<()> {
        let path = podium_state_toml_path();
        std::fs::create_dir_all(path.parent().unwrap())
            .context("failed to create podium config directory")?;
        let content = toml::to_string_pretty(self)
            .context("failed to serialize podium_state.toml")?;
        std::fs::write(&path, content)
            .with_context(|| format!("failed to write {}", path.display()))
    }

    /// Get the dock state for a project, returning a default if none exists.
    pub fn dock_state_for(&self, project_id: &str) -> DockState {
        self.dock_state
            .get(project_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Set the dock state for a project.
    /// Does not write to disk — call `save()` after.
    pub fn set_dock_state(&mut self, project_id: &str, state: DockState) {
        self.dock_state.insert(project_id.to_string(), state);
    }

    /// Set `last_project` to the given project id.
    /// Does not write to disk — call `save()` after.
    pub fn set_last_project(&mut self, project_id: &str) {
        self.last_project = Some(project_id.to_string());
    }

    /// Clear `last_project`. Called when no project is loaded.
    /// Does not write to disk — call `save()` after.
    pub fn clear_last_project(&mut self) {
        self.last_project = None;
    }
}

// ---------------------------------------------------------------------------
// DockState
// ---------------------------------------------------------------------------

/// Persisted open/close and active panel state for all three docks of one
/// project. Restored when the project is loaded (ADR-027).
///
/// Panel identity is the panel's `name()` string from the `PodiumPanel`
/// trait. Empty string means no panel is currently active in that dock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockState {
    /// Whether the left dock is open.
    #[serde(default)]
    pub left_visible: bool,

    /// Name of the active panel in the left dock. Empty if none.
    #[serde(default)]
    pub left_active_panel: String,

    /// Whether the bottom dock is open.
    #[serde(default)]
    pub bottom_visible: bool,

    /// Name of the active panel in the bottom dock. Empty if none.
    #[serde(default)]
    pub bottom_active_panel: String,

    /// Whether the right dock is open.
    #[serde(default)]
    pub right_visible: bool,

    /// Name of the active panel in the right dock. Empty if none.
    #[serde(default)]
    pub right_active_panel: String,
}

impl Default for DockState {
    /// Default dock state: all docks closed, no active panels.
    /// Applied on first project load before any user interaction.
    fn default() -> Self {
        Self {
            left_visible: false,
            left_active_panel: String::new(),
            bottom_visible: false,
            bottom_active_panel: String::new(),
            right_visible: false,
            right_active_panel: String::new(),
        }
    }
}
