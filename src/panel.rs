//! PodiumPanel — the core trait all Podium panels implement.
//!
//! Every panel in Podium (Files, Agents, Knowledge, Review, Terminal, Health)
//! implements this trait. The dock holds panels as `Arc<dyn PanelHandle>` (see
//! `panel_handle.rs`) and calls these methods to render tab buttons, manage
//! layout, and drive lifecycle events.
//!
//! ## Object safety
//!
//! `PodiumPanel` is **not object-safe**. It has a static method (`name()`)
//! and requires `Sized`. The dock cannot hold `dyn PodiumPanel` directly.
//! Instead, panels are wrapped in `Entity<T: PodiumPanel>` and exposed
//! through the object-safe `PanelHandle` trait via a blanket impl.
//! See `panel_handle.rs` for that layer.
//!
//! ## Zed reference
//!
//! This trait was designed by studying Zed's `panel::Panel` trait
//! (`zed/crates/panel/src/panel.rs`). Zed-specific methods that are not
//! applicable to Podium's simpler architecture were intentionally excluded:
//! `min_size`, `flexible_size`, `icon_label`, `is_agent_panel`,
//! `hide_button_setting`, `remote_id`, `pane`. The `Window` parameter was
//! removed from sizing and icon methods — Podium panels do not need window
//! context to answer those queries.

use gpui::{Action, App, Context, EventEmitter, Focusable, Pixels, Render};
use gpui_component::IconName;

// ---------------------------------------------------------------------------
// PanelPosition
// ---------------------------------------------------------------------------

/// The three positions a panel can occupy in the Podium layout.
///
/// Panels have a default position (set in `panels.rs`) but can be moved by
/// the user in Phase 2. Which positions are valid for a given panel is
/// declared via `position_is_valid`. The dock persists the user's choice and
/// restores it on load (ADR-027).
///
/// Default assignments are specified in ADR-028:
/// - Left:   FilesPanel, AgentsPanel, KnowledgePanel, ReviewPanel, HealthPanel
/// - Bottom: TerminalPanel
/// - Right:  (empty in Phase 1 — available for user repositioning)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelPosition {
    Left,
    Bottom,
    Right,
}

// ---------------------------------------------------------------------------
// PanelEvent
// ---------------------------------------------------------------------------

/// Events a panel emits to communicate state changes to the dock.
///
/// The dock subscribes to these events on each registered panel and updates
/// layout, zoom state, and open/closed state accordingly.
pub enum PanelEvent {
    /// Panel requests to become the active visible panel in its dock.
    Activate,
    /// Panel requests to close (hide) itself.
    Close,
    /// Panel requests to expand to fill the full window (zoom in).
    ///
    /// Phase 2: dock responds by hiding all other docks and expanding this
    /// panel to fill the content area.
    ZoomIn,
    /// Panel requests to return to its normal docked size (zoom out).
    ///
    /// Phase 2: dock responds by restoring the normal layout.
    ZoomOut,
}

// ---------------------------------------------------------------------------
// PodiumPanel trait
// ---------------------------------------------------------------------------

/// The interface every Podium panel must implement.
///
/// Panels are registered with a dock at startup. The dock uses this trait to
/// render tab buttons in the tab bar, manage panel sizing and position, and
/// drive the panel lifecycle (activate, deactivate, zoom).
///
/// ## Required bounds
///
/// - `Focusable` — panel owns a `FocusHandle` and participates in focus routing
/// - `EventEmitter<PanelEvent>` — panel emits events to the dock
/// - `Render` — panel renders its own content via GPUI
/// - `Sized` — required for the blanket `impl<T: PodiumPanel> PanelHandle for Entity<T>`
///
/// The `Sized` bound is what makes this trait not object-safe. Do not attempt
/// to use `dyn PodiumPanel` — use `dyn PanelHandle` (from `panel_handle.rs`)
/// instead, which provides the same interface in an object-safe form.
pub trait PodiumPanel: Focusable + EventEmitter<PanelEvent> + Render + Sized {
    // --- Identity -----------------------------------------------------------

    /// A stable string identifier for this panel type.
    ///
    /// Used as a persistence key (Phase 2) and in debug output. Must be unique
    /// across all registered panels. Convention: the struct name as a string
    /// literal — `"FilesPanel"`, `"AgentsPanel"`, etc.
    ///
    /// This is a **static method** (no `self`). It returns a type-level
    /// identity, not instance state. This is one of the two reasons
    /// `PodiumPanel` is not object-safe (the other being `Sized`).
    fn name() -> &'static str;

    // --- Position -----------------------------------------------------------

    /// The panel's current position in the layout.
    ///
    /// Returns the panel's default position on first launch. After the user
    /// moves the panel in Phase 2, the dock calls `set_position` and the
    /// panel stores the new value. The dock reads this to know which dock
    /// the panel belongs to.
    fn position(&self, cx: &App) -> PanelPosition;

    /// Returns true if this panel can be placed at `position`.
    ///
    /// Used by the dock to filter the "Move to…" context menu entries for
    /// this panel's tab button in Phase 2. A panel that only makes sense on
    /// the left (e.g. a narrow file tree) should return `false` for Bottom
    /// and Right.
    fn position_is_valid(&self, position: PanelPosition) -> bool;

    /// Called by the dock when the user moves this panel to a new position.
    ///
    /// The panel stores the new position in its own state so `position()`
    /// returns the correct value going forward. The dock handles writing to
    /// config (ADR-027) — the panel does not write to config directly.
    fn set_position(&mut self, position: PanelPosition, cx: &mut Context<Self>);

    // --- Sizing -------------------------------------------------------------

    /// The panel's default pixel size along its primary axis.
    ///
    /// For Left/Right panels this is width; for Bottom panels this is height.
    /// Returned when no user-adjusted size has been stored, or when the user
    /// double-clicks the resize handle to reset to default.
    fn default_size(&self, cx: &App) -> Pixels;

    // --- Tab button ---------------------------------------------------------

    /// The icon shown in this panel's tab button in the tab bar.
    fn icon(&self, cx: &App) -> IconName;

    /// The tooltip shown when hovering over this panel's tab button.
    fn icon_tooltip(&self, cx: &App) -> &'static str;

    /// The action dispatched when this panel's tab button is clicked.
    ///
    /// Typically a panel-specific toggle action — e.g. `ToggleFilesPanel`.
    /// In Phase 1, tab clicks call `toggle_panel_by_priority` directly.
    ///
    /// Phase 2: wire keyboard shortcuts by registering this action with the
    /// command palette once a focus anchor exists in the window.
    fn toggle_action(&self) -> Box<dyn Action>;

    // --- Ordering -----------------------------------------------------------

    /// Sort key determining this panel's position in the tab bar.
    ///
    /// Must be unique across all registered panels — the dock panics in debug
    /// builds on duplicate priorities. Lower values appear first (leftmost).
    ///
    /// Default assignments (ADR-028):
    /// - FilesPanel:     100
    /// - AgentsPanel:    200
    /// - KnowledgePanel: 300
    /// - ReviewPanel:    400
    /// - TerminalPanel:  500
    /// - HealthPanel:    600
    ///
    /// Gaps of 100 are intentional — leave room for inserting future panels
    /// without renumbering existing ones.
    fn activation_priority(&self) -> u32;

    // --- Lifecycle hooks ----------------------------------------------------

    /// Called by the dock when this panel becomes the active visible panel,
    /// or when the dock containing it opens or closes.
    ///
    /// `active = true` when the panel comes into view; `false` when it leaves.
    /// Default no-op. Override to start background work, focus an inner
    /// element, or refresh stale data when the panel comes into view.
    fn set_active(&mut self, _active: bool, _cx: &mut Context<Self>) {}

    /// Called by the dock when this panel enters or exits zoom mode.
    ///
    /// `zoomed = true` when expanding to fill the window; `false` on return
    /// to normal docked size. Default no-op. Override if the panel needs to
    /// adjust its layout between the two modes.
    ///
    /// Phase 2: zoom layout is not yet implemented. This hook is wired but
    /// will only be called once `PanelEvent::ZoomIn / ZoomOut` are handled
    /// in the dock.
    fn set_zoomed(&mut self, _zoomed: bool, _cx: &mut Context<Self>) {}
}
