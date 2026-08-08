//! PodiumDock — panel container for one edge of the Podium layout.
//!
//! Each dock occupies a fixed position (Left, Bottom, or Right) and holds an
//! ordered list of panels. The dock renders its active panel's content when
//! open, and an empty placeholder when closed.
//!
//! ## Phase 1 scope
//! - Panel registration sorted by `activation_priority`
//! - Tab button rendering in priority order (via `PanelButtons`)
//! - Panel activation on tab click
//! - Resize handle placeholder
//! - `PanelEvent::Activate` and `PanelEvent::Close` handled live
//!
//! ## Deferred
//! - Phase 2: persistence / serialization
//! - Phase 2: zoom layout
//! - Phase 2: panel repositioning between docks
//! - Phase 2: settings observation
//! - Phase 2: focus-follows-mouse

use std::sync::Arc;

use gpui::{
    AnyView, App, Context, Entity, EntityId, EventEmitter, Focusable, IntoElement,
    InteractiveElement, ParentElement, Pixels, Render, StyleRefinement, Styled, Subscription,
    Window, div, px,
};
use gpui::prelude::FluentBuilder;
use gpui_component::{ActiveTheme as _, StyledExt};

use crate::panel::{PanelEvent, PanelPosition, PodiumPanel};
use crate::panel_handle::ArcPanelHandle;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Width of the drag handle strip on the panel edge.
pub const RESIZE_HANDLE_SIZE: Pixels = px(6.);

// ---------------------------------------------------------------------------
// PanelEntry — internal storage for a registered panel
// ---------------------------------------------------------------------------

struct PanelEntry {
    panel: ArcPanelHandle,
    /// Stored pixel size along the dock's primary axis.
    /// None means "use the panel's default_size()".
    ///
    /// Phase 2: load/persist this via key-value store.
    size: Option<Pixels>,
    _subscriptions: [Subscription; 2],
}

// ---------------------------------------------------------------------------
// PodiumDock
// ---------------------------------------------------------------------------

/// A container fixed to one edge of the Podium window.
///
/// Holds an ordered list of panels (sorted by `activation_priority`) and
/// renders the active panel's content when open.
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
    /// unique so the tab button order is deterministic.
    ///
    /// Returns the index at which the panel was inserted.
    pub fn add_panel<T: PodiumPanel>(
        &mut self,
        panel: Entity<T>,
        cx: &mut Context<Self>,
    ) -> usize {
        let priority = panel.read(cx).activation_priority();

        // Find insertion index — binary search on activation_priority.
        let index = match self
            .panel_entries
            .binary_search_by_key(&priority, |entry| entry.panel.activation_priority(cx))
        {
            Ok(_) => {
                // Duplicate priority: panic in debug, graceful fallback in release.
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

        // Adjust active_panel_index if the insert shifts it.
        if let Some(active) = self.active_panel_index.as_mut() {
            if *active >= index {
                *active += 1;
            }
        }

        // Subscribe to PanelEvent from this panel.
        let subscriptions = [
            // Observe the entity so the dock redraws when the panel changes.
            cx.observe(&panel, |_, _, cx| cx.notify()),
            // Subscribe to events emitted by the panel.
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
                        // Phase 2: zoom layout
                    }
                }
            }),
        ];

        self.panel_entries.insert(
            index,
            PanelEntry {
                panel: Arc::new(panel),
                size: None, // Phase 2: restore from persisted size
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
    /// `set_active(true)` on the newly active panel.
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

    /// The active panel, regardless of whether the dock is open.
    pub fn active_panel(&self) -> Option<&ArcPanelHandle> {
        self.active_panel_entry().map(|e| &e.panel)
    }

    /// The active panel, only when the dock is open.
    pub fn visible_panel(&self) -> Option<&ArcPanelHandle> {
        if self.is_open {
            self.active_panel()
        } else {
            None
        }
    }

    /// Pixel size to use when rendering the active panel.
    ///
    /// Returns the stored size if one has been set by the user, otherwise
    /// falls back to the panel's `default_size()`.
    ///
    /// Phase 2: load initial size from persisted key-value store.
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

/// Events the dock emits to `main.rs` / the shell.
pub enum DockEvent {
    /// The dock's open state or active panel changed — shell should redraw.
    Changed,
}

impl EventEmitter<DockEvent> for PodiumDock {}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for PodiumDock {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(entry) = self.visible_entry() {
            let panel_view: AnyView = entry.panel.to_any();
            let position = self.position;

            div()
                .id("dock")
                .track_focus(&self.focus_handle(cx))
                .flex()
                .overflow_hidden()
                // Axis orientation: Left/Right are vertical columns,
                // Bottom is a horizontal row.
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
                .border_color(cx.theme().border)
                .bg(cx.theme().secondary)
                // Panel content — cached to avoid unnecessary redraws.
                .child(
                    div()
                        .flex_1()
                        .size_full()
                        .child(panel_view.cached(StyleRefinement::default().v_flex().size_full())),
                )
                // Resize handle — position on the inner edge.
                // Phase 2: wire drag events to resize_active_panel().
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
