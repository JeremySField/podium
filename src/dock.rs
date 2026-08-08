//! PodiumDock — panel container for one edge of the Podium layout.
//!
//! Each dock occupies a fixed position (Left, Bottom, or Right) and holds an
//! ordered list of panels sorted by `activation_priority`. The dock renders
//! its active panel's content when open, and collapses to nothing when closed.
//!
//! ## Phase 1 scope
//!
//! - Panel registration sorted by `activation_priority`
//! - Panel activation on tab click (via `toggle_panel_by_priority` in main.rs)
//! - `PanelEvent::Activate` and `PanelEvent::Close` handled live
//! - Resize handle placeholder — correct cursor, no drag wiring
//! - `set_active()` lifecycle calls on open/close and panel switch
//!
//! ## Deferred to later phases
//!
//! - Phase 2: persistence / serialization of dock open state and panel sizes
//! - Phase 2: zoom layout (`PanelEvent::ZoomIn / ZoomOut`)
//! - Phase 2: panel repositioning between docks
//! - Phase 2: settings observation
//! - Phase 2: focus-follows-mouse
//! - Phase 2: resize handle drag wiring (fix `.absolute()` positioning too —
//!   see note in `Render` impl)

use std::sync::Arc;

use gpui::{
    AnyView, App, Context, Entity, EntityId, EventEmitter, Focusable, IntoElement,
    InteractiveElement, ParentElement, Pixels, Render, StyleRefinement, Styled, Subscription,
    Window, div, px,
};
use gpui::prelude::FluentBuilder;
use gpui_component::StyledExt;

use crate::colors::PodiumColorsExt as _;
use crate::panel::{PanelEvent, PanelPosition, PodiumPanel};
use crate::panel_handle::ArcPanelHandle;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Width of the drag handle strip on the panel edge.
///
/// The handle is rendered at this size in Phase 1 (cursor only). Phase 2 will
/// wire `on_drag` events to resize the active panel.
pub const RESIZE_HANDLE_SIZE: Pixels = px(6.);

// ---------------------------------------------------------------------------
// PanelEntry — internal storage for a registered panel
// ---------------------------------------------------------------------------

struct PanelEntry {
    panel: ArcPanelHandle,
    /// User-adjusted pixel size along the dock's primary axis.
    ///
    /// `None` means "use the panel's `default_size()`". Set by the resize
    /// handle drag handler.
    ///
    /// Phase 2: load initial value from persisted config; write back on change.
    size: Option<Pixels>,
    _subscriptions: [Subscription; 2],
}

// ---------------------------------------------------------------------------
// PodiumDock
// ---------------------------------------------------------------------------

/// A container fixed to one edge of the Podium window.
///
/// Holds an ordered list of panels (sorted by `activation_priority`) and
/// renders the active panel's content when open. When closed, renders nothing
/// and takes no layout space (the outer div in `main.rs` collapses it to
/// zero width/height).
pub struct PodiumDock {
    position: PanelPosition,
    panel_entries: Vec<PanelEntry>,
    is_open: bool,
    active_panel_index: Option<usize>,
    focus_handle: gpui::FocusHandle,
}

impl Focusable for PodiumDock {
    fn focus_handle(&self, _cx: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl PodiumDock {
    pub fn new(position: PanelPosition, cx: &mut Context<Self>) -> Self {
        Self {
            position,
            panel_entries: Vec::new(),
            is_open: false,
            active_panel_index: None,
            focus_handle: cx.focus_handle(),
        }
    }

    // --- Registration -------------------------------------------------------

    /// Register a panel with this dock.
    ///
    /// Panels are inserted in `activation_priority` order (ascending). Panics
    /// in debug builds if two panels share a priority — priorities must be
    /// unique so tab button order is deterministic (ADR-028).
    ///
    /// Returns the index at which the panel was inserted.
    pub fn add_panel<T: PodiumPanel>(
        &mut self,
        panel: Entity<T>,
        cx: &mut Context<Self>,
    ) -> usize {
        let priority = panel.read(cx).activation_priority();

        // Binary search on activation_priority for the insertion point.
        let index = match self
            .panel_entries
            .binary_search_by_key(&priority, |entry| entry.panel.activation_priority(cx))
        {
            Ok(_) => {
                // Duplicate priority — hard error in debug, append in release.
                // Priorities must be unique so tab order is deterministic.
                if cfg!(debug_assertions) {
                    panic!(
                        "Panel `{}` has the same activation priority ({}) as an existing panel. \
                         Each panel must have a unique priority so tab button order is deterministic.",
                        T::name(),
                        priority,
                    );
                }
                // Release: append at end rather than silently clobbering order.
                self.panel_entries.len()
            }
            Err(ix) => ix,
        };

        // Adjust active_panel_index if the insertion shifts it.
        if let Some(active) = self.active_panel_index.as_mut() {
            if *active >= index {
                *active += 1;
            }
        }

        // Subscribe to the panel entity so the dock redraws when it changes,
        // and to panel events so the dock can respond to Activate / Close.
        let subscriptions = [
            cx.observe(&panel, |_, _, cx| cx.notify()),
            cx.subscribe(&panel, {
                let panel_id = Entity::entity_id(&panel);
                move |dock, _panel, event, cx| match event {
                    PanelEvent::Activate => {
                        if let Some(ix) = dock.index_for_id(panel_id) {
                            dock.set_open(true, cx);
                            dock.activate_panel(ix, cx);
                        }
                    }
                    PanelEvent::Close => {
                        if dock
                            .visible_panel()
                            .is_some_and(|p| p.panel_id() == panel_id)
                        {
                            dock.set_open(false, cx);
                        }
                    }
                    PanelEvent::ZoomIn | PanelEvent::ZoomOut => {
                        // Phase 2: implement zoom layout — expand panel to fill
                        // the full window, hiding the other docks temporarily.
                    }
                }
            }),
        ];

        self.panel_entries.insert(
            index,
            PanelEntry {
                panel: Arc::new(panel),
                size: None, // Phase 2: restore from persisted config on load
                _subscriptions: subscriptions,
            },
        );

        cx.notify();
        index
    }

    // --- Activation ---------------------------------------------------------

    /// Make the panel at `index` the active panel.
    ///
    /// Calls `set_active(false)` on the previously active panel and
    /// `set_active(true)` on the new one. No-op if `index` is already active.
    pub fn activate_panel(&mut self, index: usize, cx: &mut Context<Self>) {
        if Some(index) == self.active_panel_index {
            return;
        }

        // Deactivate the current panel.
        if let Some(prev) = self.active_panel_entry() {
            prev.panel.set_active(false, cx);
        }

        self.active_panel_index = Some(index);

        // Activate the new panel.
        if let Some(entry) = self.active_panel_entry() {
            entry.panel.set_active(true, cx);
        }

        cx.notify();
    }

    /// Open or close this dock.
    ///
    /// Notifies the active panel of its new active state when the dock
    /// transitions between open and closed.
    pub fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if open == self.is_open {
            return;
        }

        self.is_open = open;

        if let Some(entry) = self.active_panel_entry() {
            entry.panel.set_active(open, cx);
        }

        cx.notify();
    }

    // --- Read accessors -----------------------------------------------------

    pub fn position(&self) -> PanelPosition {
        self.position
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn active_panel_index(&self) -> Option<usize> {
        self.active_panel_index
    }

    pub fn panels_len(&self) -> usize {
        self.panel_entries.len()
    }

    /// The active panel handle, regardless of whether the dock is open.
    pub fn active_panel(&self) -> Option<&ArcPanelHandle> {
        self.active_panel_entry().map(|e| &e.panel)
    }

    /// The active panel handle, only when the dock is open.
    ///
    /// Returns `None` when the dock is closed, even if a panel is active.
    pub fn visible_panel(&self) -> Option<&ArcPanelHandle> {
        if self.is_open {
            self.active_panel()
        } else {
            None
        }
    }

    /// Pixel size to use when rendering the active panel along the dock's
    /// primary axis (width for Left/Right, height for Bottom).
    ///
    /// Returns the user-adjusted size if one has been set by dragging the
    /// resize handle, otherwise falls back to the panel's `default_size()`.
    ///
    /// Phase 2: replace the hard-coded `px(280.)` / `px(240.)` values in
    /// `main.rs` with calls to this method:
    /// `dock.active_panel_size(cx).unwrap_or(px(280.))`
    pub fn active_panel_size(&self, cx: &App) -> Option<Pixels> {
        self.active_panel_entry().map(|entry| {
            entry
                .size
                .unwrap_or_else(|| entry.panel.default_size(cx))
        })
    }

    /// Iterate over all registered panel handles in activation-priority order.
    pub fn panels(&self) -> impl Iterator<Item = &ArcPanelHandle> {
        self.panel_entries.iter().map(|e| &e.panel)
    }

    // --- Internal helpers ---------------------------------------------------

    fn active_panel_entry(&self) -> Option<&PanelEntry> {
        self.active_panel_index
            .and_then(|i| self.panel_entries.get(i))
    }

    fn index_for_id(&self, id: EntityId) -> Option<usize> {
        self.panel_entries
            .iter()
            .position(|e| e.panel.panel_id() == id)
    }
}

// ---------------------------------------------------------------------------
// EventEmitter
// ---------------------------------------------------------------------------

/// Events the dock emits to the shell (`main.rs`).
pub enum DockEvent {
    /// The dock's open state or active panel changed.
    ///
    /// Phase 2: subscribe to this in `main.rs` to trigger persistence writes
    /// when the user opens, closes, or switches panels. Currently emitted
    /// as `cx.notify()` only — the typed event is not dispatched yet.
    Changed,
}

impl EventEmitter<DockEvent> for PodiumDock {}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for PodiumDock {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.podium_colors();

        if let Some(entry) = self.visible_entry() {
            let panel_view: AnyView = entry.panel.to_any();
            let position = self.position;

            // Phase 2 note: the resize handle below uses `.absolute()` for
            // positioning. Absolute children anchor to the nearest ancestor
            // with `.relative()`. This parent div does not call `.relative()`,
            // so in Phase 1 the handle may anchor to the window root rather
            // than the dock edge. This is harmless in Phase 1 (handle is
            // cursor-only, no visual rendering). Fix in Phase 2 when drag
            // wiring is added: add `.relative()` to this outer div.
            div()
                .id("dock")
                .track_focus(&self.focus_handle(cx))
                .flex()
                .overflow_hidden()
                // Axis orientation: Left/Right docks are vertical columns,
                // Bottom dock is a horizontal row.
                .map(|this| match position {
                    PanelPosition::Left | PanelPosition::Right => {
                        this.flex_col().w_full().h_full()
                    }
                    PanelPosition::Bottom => this.flex_row().h_full().w_full(),
                })
                // Border on the edge that faces the content area.
                .map(|this| match position {
                    PanelPosition::Left => this.border_r_1(),
                    PanelPosition::Right => this.border_l_1(),
                    PanelPosition::Bottom => this.border_t_1(),
                })
                .border_color(colors.panel_border)
                .bg(colors.panel_background)
                // Panel content — cached to avoid unnecessary redraws when
                // the dock itself hasn't changed.
                .child(
                    div()
                        .flex_1()
                        .size_full()
                        .child(panel_view.cached(StyleRefinement::default().v_flex().size_full())),
                )
                // Resize handle — renders the correct resize cursor but has
                // no drag wiring in Phase 1.
                //
                // Phase 2: wire on_drag to update PanelEntry.size and call
                // cx.notify(). Also add .relative() to the parent div above
                // so this absolute child anchors to the dock edge correctly.
                .child(match position {
                    PanelPosition::Left => div()
                        .id("resize-handle")
                        .absolute()
                        .right(px(0.) - RESIZE_HANDLE_SIZE / 2.)
                        .top(px(0.))
                        .h_full()
                        .w(RESIZE_HANDLE_SIZE)
                        .cursor_col_resize(),
                    PanelPosition::Right => div()
                        .id("resize-handle")
                        .absolute()
                        .left(px(0.) - RESIZE_HANDLE_SIZE / 2.)
                        .top(px(0.))
                        .h_full()
                        .w(RESIZE_HANDLE_SIZE)
                        .cursor_col_resize(),
                    PanelPosition::Bottom => div()
                        .id("resize-handle")
                        .absolute()
                        .top(px(0.) - RESIZE_HANDLE_SIZE / 2.)
                        .left(px(0.))
                        .w_full()
                        .h(RESIZE_HANDLE_SIZE)
                        .cursor_row_resize(),
                })
        } else {
            // Dock is closed — render nothing, take no space.
            // The outer div in main.rs collapses the dock to zero width/height.
            div().id("dock").track_focus(&self.focus_handle(cx))
        }
    }
}

// ---------------------------------------------------------------------------
// Private helper — visible_entry
// ---------------------------------------------------------------------------

impl PodiumDock {
    fn visible_entry(&self) -> Option<&PanelEntry> {
        if self.is_open {
            self.active_panel_entry()
        } else {
            None
        }
    }
}
