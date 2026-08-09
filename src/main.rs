//! Podium application shell — Phase 1 complete, Phase 2 in progress.
//!
//! `PodiumApp` is the root GPUI view. It holds three `PodiumDock` entities
//! (left, bottom, right) and renders the full shell: TitleBar, tab bar,
//! content area with docks, StatusBar, and overlay layers.
//!
//! ## Dark theme — three calls required
//!
//! gpui-component dark mode requires all three of the following in `app.run()`,
//! in this order. Any subset produces either a light palette, a missing mode
//! flag, or OS-level light popup contexts:
//!
//! ```rust
//! theme.colors = *ThemeColor::dark();          // swap the color palette
//! theme.mode = ThemeMode::Dark;                // set the mode flag
//! cx.set_window_appearance(Some(Dark));        // OS-level dark context
//! ```
//!
//! ## Popup/dropdown theming — two fields required
//!
//! gpui-component's `PopupMenu` renders its background via `popover_style(cx)`
//! which reads `cx.theme().tokens.popover` (a `ThemeToken`) — NOT
//! `cx.theme().colors.popover` (an `Hsla`). Both must be set to get a dark
//! dropdown background:
//!
//! ```rust
//! colors.popover = dark_color;                 // sets ThemeColor.popover
//! theme.tokens.popover = dark_color.into();    // sets ThemeTokens.popover (what popover_style reads)
//! ```
//!
//! `ThemeToken` implements `From<Hsla>` so `.into()` works directly.
//!
//! ## ThemeColor fields of note (confirmed from theme_color.rs source)
//!
//! - `popover`        — dropdown/popup Hsla color
//! - `popover_foreground` — dropdown/popup text
//! - `title_bar`      — title bar background
//! - `title_bar_border`
//! - `status_bar`     — status bar background
//! - `tab_bar`        — tab bar background
//! - `overlay`        — modal overlay background
//!
//! ## Phase 1 complete — bugs fixed in Phase 2 start
//!
//! - `.relative()` added to dock render div (resize handle anchor fix)
//! - Hard-coded dock widths replaced with `active_panel_size()` calls
//!
//! ## Deferred to later phases
//!
//! - Phase 2: project switcher Combobox in TitleBar (replaces placeholder)
//! - Phase 2: dock resize handle drag wiring
//! - Phase 2: dock open/close state persisted to config
//! - Phase 2: panel repositioning between docks
//! - Phase 10: full JSON-driven theme system

mod colors;
mod config;
mod dock;
mod panel;
mod panel_handle;
mod panels;
mod ssh_config;
mod state;
mod watch;
mod watch_error;

use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Render, Styled,
    Window, WindowAppearance, WindowOptions, actions, div, px,
};
use gpui::prelude::FluentBuilder as _;
use gpui_component::{
    ActiveTheme as _,
    IconName,
    Root,
    Sizable as _,
    TitleBar,
    button::{Button, ButtonVariants as _},
    menu::DropdownMenu as _,
    status_bar::StatusBar,
    theme::{Theme, ThemeColor, ThemeMode},
};

use colors::PodiumColorsExt as _;
use dock::PodiumDock;
use panel::PanelPosition;
use panels::{AgentsPanel, FilesPanel, HealthPanel, KnowledgePanel, ReviewPanel, TerminalPanel};

// ---------------------------------------------------------------------------
// Global actions
// ---------------------------------------------------------------------------

actions!(podium, [Quit]);

// ---------------------------------------------------------------------------
// PodiumApp — root view
// ---------------------------------------------------------------------------

/// Root view for the Podium application.
///
/// Owns the three docks (left, bottom, right) and renders the complete shell.
/// Panels are registered into their default docks in `new()` per ADR-028.
struct PodiumApp {
    left_dock: Entity<PodiumDock>,
    bottom_dock: Entity<PodiumDock>,
    right_dock: Entity<PodiumDock>,
}

impl PodiumApp {
    fn new(cx: &mut Context<Self>) -> Self {
        let left_dock = cx.new(|cx| PodiumDock::new(PanelPosition::Left, cx));
        let bottom_dock = cx.new(|cx| PodiumDock::new(PanelPosition::Bottom, cx));
        let right_dock = cx.new(|cx| PodiumDock::new(PanelPosition::Right, cx));

        // Left dock: Files (100), Agents (200), Knowledge (300), Review (400), Health (600)
        // Priority order determines tab bar position — see ADR-028.
        left_dock.update(cx, |dock, cx| {
            dock.add_panel(cx.new(|cx| FilesPanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| AgentsPanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| KnowledgePanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| ReviewPanel::new(cx)), cx);
            dock.add_panel(cx.new(|cx| HealthPanel::new(cx)), cx);
        });

        // Bottom dock: Terminal (500) — wide horizontal tool, bottom placement
        // per ADR-028.
        bottom_dock.update(cx, |dock, cx| {
            dock.add_panel(cx.new(|cx| TerminalPanel::new(cx)), cx);
        });

        // Right dock: no panels in Phase 1.
        // Available for user repositioning in Phase 2 (ADR-028).

        Self { left_dock, bottom_dock, right_dock }
    }

    /// Toggle the panel with `priority` in whichever dock owns it.
    ///
    /// Searches all three docks in order (left, bottom, right). On match:
    /// - If the dock is open and the matched panel is already active → close.
    /// - Otherwise → activate the panel and open the dock.
    ///
    /// Returns after the first match; a panel can only live in one dock.
    /// If no panel with `priority` is found across all docks, this is a no-op
    /// (should not occur in normal operation, but is safe to call speculatively).
    fn toggle_panel_by_priority(&mut self, priority: u32, cx: &mut Context<Self>) {
        for dock_entity in [&self.left_dock, &self.bottom_dock, &self.right_dock] {
            let handled = dock_entity.update(cx, |dock, cx| {
                let index = dock
                    .panels()
                    .enumerate()
                    .find(|(_, p)| p.activation_priority(cx) == priority)
                    .map(|(i, _)| i);

                let Some(index) = index else { return false; };

                if dock.is_open() && dock.active_panel_index() == Some(index) {
                    dock.set_open(false, cx);
                } else {
                    dock.activate_panel(index, cx);
                    dock.set_open(true, cx);
                }
                true
            });

            if handled {
                cx.notify();
                return;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for PodiumApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Derive all colors from the active theme via PodiumColors.
        // This is the single call site for colors — no raw rgb() literals below.
        let colors = cx.podium_colors();

        // Collect tab info from all docks: (priority, tooltip, is_active).
        // Sorted by priority so tab order matches activation_priority, which
        // matches the panel registration order in new() (ADR-028).
        let mut tab_info: Vec<(u32, &'static str, bool)> = Vec::new();
        for dock_entity in [&self.left_dock, &self.bottom_dock, &self.right_dock] {
            let dock = dock_entity.read(cx);
            let active_idx = if dock.is_open() { dock.active_panel_index() } else { None };
            dock.panels().enumerate().for_each(|(i, p)| {
                tab_info.push((
                    p.activation_priority(cx),
                    p.icon_tooltip(cx),
                    active_idx == Some(i),
                ));
            });
        }
        tab_info.sort_by_key(|(priority, _, _)| *priority);

        let left_open = self.left_dock.read(cx).is_open();
        let bottom_open = self.bottom_dock.read(cx).is_open();
        let right_open = self.right_dock.read(cx).is_open();

        // Read dock sizes from the active panel's default_size().
        // Falls back to sensible defaults when no panel is active.
        let left_width = self.left_dock.read(cx)
            .active_panel_size(cx)
            .unwrap_or(px(280.));
        let right_width = self.right_dock.read(cx)
            .active_panel_size(cx)
            .unwrap_or(px(280.));
        let bottom_height = self.bottom_dock.read(cx)
            .active_panel_size(cx)
            .unwrap_or(px(240.));

        let left_dock = self.left_dock.clone();
        let bottom_dock = self.bottom_dock.clone();
        let right_dock = self.right_dock.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.content_background)
            // --- TitleBar ---------------------------------------------------
            .child(
                TitleBar::new()
                    .bg(colors.title_bar_background)
                    .border_b_1()
                    .border_color(colors.title_bar_border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            // [≡] Application menu — hamburger + dropdown.
                            // AppMenuBar was attempted and abandoned: it does not
                            // render visibly on Windows in the current
                            // gpui-component version. Button.dropdown_menu() is
                            // the reliable alternative.
                            .child(
                                Button::new("app-menu")
                                    .icon(IconName::Menu)
                                    .ghost()
                                    .small()
                                    .dropdown_menu(|menu, _window, _cx| {
                                        menu.menu("Quit Podium", Box::new(Quit))
                                    }),
                            )
                            // Podium name label.
                            .child(
                                div()
                                    .text_color(cx.theme().foreground)
                                    .text_sm()
                                    .child("Podium"),
                            )
                            // Project switcher placeholder.
                            // Phase 2: replace with gpui-component Combobox
                            // backed by the loaded project list.
                            .child(
                                div()
                                    .px_2()
                                    .py(px(2.))
                                    .rounded(px(4.))
                                    .bg(cx.theme().secondary)
                                    .text_color(cx.theme().secondary_foreground)
                                    .text_sm()
                                    .child("No project"),
                            ),
                    ),
            )
            // --- Tab bar ----------------------------------------------------
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .border_b_1()
                    .border_color(colors.tab_bar_border)
                    .bg(colors.tab_bar_background)
                    .children(tab_info.into_iter().map(|(priority, tooltip, is_active)| {
                        div()
                            .px_3()
                            .py_2()
                            .text_sm()
                            .cursor_pointer()
                            .when(is_active, |this| {
                                this.border_b_2()
                                    .border_color(colors.tab_active_indicator)
                                    .text_color(colors.tab_active_foreground)
                            })
                            .when(!is_active, |this| {
                                this.text_color(colors.tab_inactive_foreground)
                            })
                            // Tab click handler — uses cx.listener() on mouse_down
                            // rather than dispatch_action / on_action because
                            // on_action only fires when the element is in the
                            // focused element's ancestor chain. With no focused
                            // element in Phase 1, dispatched actions are silently
                            // dropped. cx.listener() goes through the entity
                            // system and does not require focus tree membership.
                            //
                            // Phase 2: wire keyboard shortcuts via the action
                            // system using toggle_action() from PanelHandle once
                            // a focus anchor exists in the window.
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _: &MouseDownEvent, _window, cx| {
                                    this.toggle_panel_by_priority(priority, cx);
                                }),
                            )
                            .child(tooltip)
                    })),
            )
            // --- Content area -----------------------------------------------
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_row()
                            .overflow_hidden()
                            // Left dock — collapses to zero when closed.
                            .child(
                                div()
                                    .when(left_open, |this| this.w(left_width))
                                    .when(!left_open, |this| this.w(px(0.)))
                                    .h_full()
                                    .overflow_hidden()
                                    .child(left_dock),
                            )
                            // Center content area — empty placeholder.
                            // Phase 6: replaced by the Editor panel.
                            .child(
                                div()
                                    .flex_1()
                                    .h_full()
                                    .bg(colors.content_background)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_color(cx.theme().muted_foreground)
                                            .text_sm()
                                            .child("Podium"),
                                    ),
                            )
                            // Right dock — collapses to zero when closed.
                            // No panels registered in Phase 1 (ADR-028).
                            .child(
                                div()
                                    .when(right_open, |this| this.w(right_width))
                                    .when(!right_open, |this| this.w(px(0.)))
                                    .h_full()
                                    .overflow_hidden()
                                    .child(right_dock),
                            ),
                    )
                    // Bottom dock — collapses to zero when closed.
                    .child(
                        div()
                            .when(bottom_open, |this| this.h(bottom_height))
                            .when(!bottom_open, |this| this.h(px(0.)))
                            .w_full()
                            .overflow_hidden()
                            .child(bottom_dock),
                    ),
            )
            // --- StatusBar --------------------------------------------------
            .child(
                StatusBar::new()
                    .bg(colors.title_bar_background)
                    .left("Podium")
                    .right("Phase 2"),
            )
            // --- Overlay layers ---------------------------------------------
            // Required by gpui-component's Root wrapper. These are zero-cost
            // when no dialog/sheet/notification is active.
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    // Register gpui-component's asset bundle before building the app.
    // Required for icons and other bundled assets to render.
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx: &mut App| {
        // Initialize gpui-component — must be the first call inside app.run().
        gpui_component::init(cx);

        // Apply dark theme. All three blocks are required — see module doc.
        //
        // The popover fix requires setting TWO separate fields (confirmed by
        // reading the gpui-component source):
        //
        //   1. theme.colors.popover (ThemeColor field, Hsla)
        //      — read by components that access cx.theme().popover directly
        //
        //   2. theme.tokens.popover (ThemeTokens field, ThemeToken)
        //      — read by popover_style(cx) in styled.rs:
        //        `self.bg(cx.theme().tokens.popover)`
        //        This is what PopupMenu calls for its background.
        //
        //   ThemeToken implements From<Hsla>, so `.into()` on an Hsla works.
        //
        // Gruvbox Dark bg1 (#3c3836) is the target value for popups — same
        // level as the tab bar, reads as floating above the content floor.
        {
            let theme = Theme::global_mut(cx);
            let mut colors = *ThemeColor::dark();
            let popup_bg: gpui::Hsla = gpui::rgb(0x3c3836).into();
            colors.popover = popup_bg;
            theme.colors = colors;
            // Set tokens.popover — this is the field popover_style(cx) actually reads.
            theme.tokens.popover = popup_bg.into();
            theme.mode = ThemeMode::Dark;
        }
        // OS-level dark appearance — required for popup/overlay render contexts
        // to receive the correct dark window appearance on Windows.
        cx.set_window_appearance(Some(WindowAppearance::Dark));

        cx.on_action(|_: &Quit, cx| cx.quit());

        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    // Use gpui-component's TitleBar window options so the
                    // TitleBar component integrates correctly with the OS
                    // window decorations.
                    titlebar: Some(TitleBar::title_bar_options()),
                    ..Default::default()
                },
                |window, cx| {
                    // Root is mandatory for all gpui-component features:
                    // theming, dialogs, sheets, notifications. The inner
                    // PodiumApp view is passed to Root::new().
                    let view = cx.new(|cx| PodiumApp::new(cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}
