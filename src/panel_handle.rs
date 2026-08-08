//! PanelHandle — object-safe wrapper around `PodiumPanel`.
//!
//! `PodiumPanel` is not object-safe because it has a static method (`name()`)
//! and requires `Sized`. `PanelHandle` is a separate object-safe trait that
//! mirrors every *instance* method on `PodiumPanel`, and is implemented via a
//! blanket impl on `Entity<T: PodiumPanel>`.
//!
//! The dock holds panels as `Arc<dyn PanelHandle>` and calls these methods to
//! render tab buttons, dispatch toggle actions, manage position and sizing, and
//! drive lifecycle events.
//!
//! ## Design notes
//! - `Send + Sync` required because `Arc<dyn PanelHandle>` crosses thread
//!   boundaries in GPUI's async task system.
//! - No `Window` parameter on any method — Podium's `PodiumPanel` trait
//!   deliberately excludes it from sizing and icon queries.
//! - No `pane`, `remote_id`, `min_size`, `flexible_size`, `icon_label`,
//!   `is_agent_panel`, or `hide_button_setting` — all Zed-specific, excluded
//!   by Session 3b design decisions.

use gpui::{Action, AnyView, App, Entity, EntityId, Pixels};
use gpui_component::IconName;
use std::sync::Arc;

use crate::panel::{PanelPosition, PodiumPanel};

// ---------------------------------------------------------------------------
// PanelHandle trait
// ---------------------------------------------------------------------------

/// Object-safe interface for a registered Podium panel.
///
/// The dock stores panels as `Arc<dyn PanelHandle>` so it can manage a
/// heterogeneous collection without knowing each panel's concrete type.
///
/// Every method here corresponds directly to a method on `PodiumPanel`.
/// See that trait's documentation for full semantics.
pub trait PanelHandle: Send + Sync {
    // --- Identity -----------------------------------------------------------

    /// Stable runtime identifier for this panel instance's type.
    ///
    /// Delegates to `T::name()` on the concrete panel type.
    fn name(&self) -> &'static str;

    /// GPUI entity ID of the underlying `Entity<T>`.
    ///
    /// Used by the dock to find a specific panel in its list (e.g., when
    /// handling a `PanelEvent::Activate` to locate the emitting panel).
    fn panel_id(&self) -> EntityId;

    // --- Focus --------------------------------------------------------------

    /// The panel's root focus handle.
    ///
    /// Used by the dock to focus the panel when it becomes active, and to
    /// check whether the panel currently contains focus.
    fn focus_handle(&self, cx: &App) -> gpui::FocusHandle;

    // --- View ---------------------------------------------------------------

    /// Erase the concrete type to `AnyView` for rendering inside the dock.
    fn to_any(&self) -> AnyView;

    // --- Position -----------------------------------------------------------

    /// The panel's current position in the layout.
    fn position(&self, cx: &App) -> PanelPosition;

    /// Returns true if this panel can be placed at `position`.
    fn position_is_valid(&self, position: PanelPosition, cx: &App) -> bool;

    /// Move this panel to `position`.
    ///
    /// Calls `set_position` on the inner panel via `Entity::update`.
    fn set_position(&self, position: PanelPosition, cx: &mut App);

    // --- Sizing -------------------------------------------------------------

    /// The panel's default pixel size along its primary axis.
    fn default_size(&self, cx: &App) -> Pixels;

    // --- Tab button ---------------------------------------------------------

    /// The icon for this panel's tab button.
    fn icon(&self, cx: &App) -> IconName;

    /// The tooltip for this panel's tab button.
    fn icon_tooltip(&self, cx: &App) -> &'static str;

    /// The action dispatched when this panel's tab button is clicked.
    fn toggle_action(&self, cx: &App) -> Box<dyn Action>;

    // --- Ordering -----------------------------------------------------------

    /// Sort key for the tab button row. Lower values appear first.
    ///
    /// The dock panics in debug builds if two panels share a priority.
    fn activation_priority(&self, cx: &App) -> u32;

    // --- Lifecycle ----------------------------------------------------------

    /// Notify the panel that it has become active or inactive.
    fn set_active(&self, active: bool, cx: &mut App);

    /// Notify the panel that it has entered or exited zoom mode.
    fn set_zoomed(&self, zoomed: bool, cx: &mut App);
}

// ---------------------------------------------------------------------------
// Blanket impl: Entity<T: PodiumPanel> implements PanelHandle
// ---------------------------------------------------------------------------

impl<T> PanelHandle for Entity<T>
where
    T: PodiumPanel,
{
    fn name(&self) -> &'static str {
        T::name()
    }

    fn panel_id(&self) -> EntityId {
        Entity::entity_id(self)
    }

    fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        self.read(cx).focus_handle(cx)
    }

    fn to_any(&self) -> AnyView {
        self.clone().into()
    }

    fn position(&self, cx: &App) -> PanelPosition {
        self.read(cx).position(cx)
    }

    fn position_is_valid(&self, position: PanelPosition, cx: &App) -> bool {
        self.read(cx).position_is_valid(position)
    }

    fn set_position(&self, position: PanelPosition, cx: &mut App) {
        self.update(cx, |this, cx| this.set_position(position, cx))
    }

    fn default_size(&self, cx: &App) -> Pixels {
        self.read(cx).default_size(cx)
    }

    fn icon(&self, cx: &App) -> IconName {
        self.read(cx).icon(cx)
    }

    fn icon_tooltip(&self, cx: &App) -> &'static str {
        self.read(cx).icon_tooltip(cx)
    }

    fn toggle_action(&self, cx: &App) -> Box<dyn Action> {
        self.read(cx).toggle_action()
    }

    fn activation_priority(&self, cx: &App) -> u32 {
        self.read(cx).activation_priority()
    }

    fn set_active(&self, active: bool, cx: &mut App) {
        self.update(cx, |this, cx| this.set_active(active, cx))
    }

    fn set_zoomed(&self, zoomed: bool, cx: &mut App) {
        self.update(cx, |this, cx| this.set_zoomed(zoomed, cx))
    }
}

// ---------------------------------------------------------------------------
// AnyView conversion
// ---------------------------------------------------------------------------

impl From<&dyn PanelHandle> for AnyView {
    fn from(val: &dyn PanelHandle) -> Self {
        val.to_any()
    }
}

// ---------------------------------------------------------------------------
// Type alias
// ---------------------------------------------------------------------------

/// Convenience alias used throughout the dock.
pub type ArcPanelHandle = Arc<dyn PanelHandle>;
