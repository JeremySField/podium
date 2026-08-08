//! PodiumPanel — the core trait all Podium panels implement.
//!
//! Every panel in Podium (Files, Agents, Knowledge, Review, Terminal, Health)
//! implements this trait. The dock holds panels as `Arc<dyn PanelHandle>` and
//! calls these methods to render tab buttons, manage layout, and drive
//! lifecycle events.
//!
//! Design notes:
//! - Panels are repositionable (ADR-027 pending storage design)
//! - The dock owns position persistence — panels do not write to config directly
//! - Panels emit `PanelEvent` to signal state changes to the dock

use gpui::{Action, App, Context, EventEmitter, Focusable, Pixels, Render};
use gpui_component::IconName;

// ---------------------------------------------------------------------------
// PanelPosition
// ---------------------------------------------------------------------------

/// The three positions a panel can occupy in the Podium layout.
///
/// Panels have a default position but can be moved by the user. Which
/// positions are valid for a given panel is declared via `position_is_valid`.
/// The dock persists the user's choice and restores it on load (ADR-027).
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
    /// Panel requests to expand to fill the full window.
    ZoomIn,
    /// Panel requests to return to its normal docked size.
    ZoomOut,
}

// ---------------------------------------------------------------------------
// PodiumPanel trait
// ---------------------------------------------------------------------------

/// The interface every Podium panel must implement.
///
/// Panels are registered with the dock at startup. The dock uses this trait
/// to render tab buttons in the status bar, manage panel sizing and position,
/// and drive panel lifecycle (activate, deactivate, zoom).
///
/// ## Required bounds
/// - `Focusable` — panel owns a `FocusHandle` and tracks focus
/// - `EventEmitter<PanelEvent>` — panel emits events to the dock
/// - `Render` — panel renders its own content via GPUI
/// - `Sized` — required for object-safe blanket impls on `Entity<T>`
pub trait PodiumPanel: Focusable + EventEmitter<PanelEvent> + Render + Sized {
    // --- Identity -----------------------------------------------------------

    /// A stable string identifier for this panel type.
    ///
    /// Used for persistence keys and debug output. Must be unique across all
    /// registered panels. Convention: `"FilesPanel"`, `"AgentsPanel"`, etc.
    fn name() -> &'static str;

    // --- Position -----------------------------------------------------------

    /// The panel's current position in the layout.
    ///
    /// On first launch this returns the panel's default position. After the
    /// user moves the panel, the dock calls `set_position` and the panel
    /// stores the new value in its own state. The dock reads this to know
    /// where to render the panel.
    fn position(&self, cx: &App) -> PanelPosition;

    /// Returns true if this panel can be placed at the given position.
    ///
    /// Used by the dock to filter the "Move to..." context menu entries for
    /// this panel's tab button. A panel that only makes sense on the left
    /// should return false for Bottom and Right.
    fn position_is_valid(&self, position: PanelPosition) -> bool;

    /// Called by the dock when the user moves this panel to a new position.
    ///
    /// The panel stores the new position in its own state so `position()`
    /// returns the correct value going forward. The dock handles writing the
    /// new position to config (ADR-027) — the panel does not write to config.
    fn set_position(&mut self, position: PanelPosition, cx: &mut Context<Self>);

    // --- Sizing -------------------------------------------------------------

    /// The panel's default pixel size along its primary axis.
    ///
    /// For Left/Right panels this is width. For Bottom panels this is height.
    /// Returned when no persisted size exists or the user double-clicks the
    /// resize handle to reset.
    fn default_size(&self, cx: &App) -> Pixels;

    // --- Tab button ---------------------------------------------------------

    /// The icon shown in this panel's tab button in the status bar.
    fn icon(&self, cx: &App) -> IconName;

    /// The tooltip shown when hovering this panel's tab button.
    fn icon_tooltip(&self, cx: &App) -> &'static str;

    /// The action dispatched when this panel's tab button is clicked.
    ///
    /// Typically a panel-specific toggle action, e.g. `ToggleFilesPanel`.
    /// The dock dispatches this action to show or hide the panel.
    fn toggle_action(&self) -> Box<dyn Action>;

    // --- Ordering -----------------------------------------------------------

    /// Determines this panel's position in the status bar tab button row.
    ///
    /// Must be unique across all registered panels — the dock panics in debug
    /// builds if two panels share a priority. Lower values appear first.
    ///
    /// Recommended assignments (leave gaps for future panels):
    /// - FilesPanel:     100
    /// - AgentsPanel:    200
    /// - KnowledgePanel: 300
    /// - ReviewPanel:    400
    /// - TerminalPanel:  500
    /// - HealthPanel:    600
    fn activation_priority(&self) -> u32;

    // --- Lifecycle hooks ----------------------------------------------------

    /// Called by the dock when this panel becomes the active visible panel.
    ///
    /// Default no-op. Override to start background work, focus an inner
    /// element, or refresh stale data when the panel comes into view.
    fn set_active(&mut self, _active: bool, _cx: &mut Context<Self>) {}

    /// Called by the dock when this panel enters or exits zoom mode.
    ///
    /// Default no-op. Override if the panel needs to adjust its layout when
    /// it expands to fill the full window vs. returning to docked size.
    fn set_zoomed(&mut self, _zoomed: bool, _cx: &mut Context<Self>) {}
}
