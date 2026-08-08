mod dock;
mod panel;
mod panel_handle;
mod panels;

use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Render, Styled,
    Window, WindowAppearance, WindowOptions, actions, div, px, rgb,
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

use dock::PodiumDock;
use panel::PanelPosition;
use panels::{AgentsPanel, FilesPanel, HealthPanel, KnowledgePanel, ReviewPanel, TerminalPanel};

// ---------------------------------------------------------------------------
// Podium chrome color — TitleBar and StatusBar background.
// Tune this constant to adjust the chrome color without hunting through code.
// ---------------------------------------------------------------------------

/// Matches Gruvbox Dark title_bar.background — `#4c4642`.
const PODIUM_CHROME: u32 = 0x4c4642;

// ---------------------------------------------------------------------------
// Global actions
// ---------------------------------------------------------------------------

actions!(podium, [Quit]);

// ---------------------------------------------------------------------------
// PodiumApp — root view
// ---------------------------------------------------------------------------

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
        left_dock.update(cx, |dock, cx| {
            let files = cx.new(|cx| FilesPanel::new(cx));
            dock.add_panel(files, cx);
            let agents = cx.new(|cx| AgentsPanel::new(cx));
            dock.add_panel(agents, cx);
            let knowledge = cx.new(|cx| KnowledgePanel::new(cx));
            dock.add_panel(knowledge, cx);
            let review = cx.new(|cx| ReviewPanel::new(cx));
            dock.add_panel(review, cx);
            let health = cx.new(|cx| HealthPanel::new(cx));
            dock.add_panel(health, cx);
        });

        // Bottom dock: Terminal (500)
        bottom_dock.update(cx, |dock, cx| {
            let terminal = cx.new(|cx| TerminalPanel::new(cx));
            dock.add_panel(terminal, cx);
        });

        // Right dock: no panels in Phase 1 — available for user repositioning.

        Self { left_dock, bottom_dock, right_dock }
    }

    /// Toggle the panel with `priority` in whichever dock owns it.
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
        // Collect tab info: (priority, tooltip, is_active)
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

        let left_dock = self.left_dock.clone();
        let bottom_dock = self.bottom_dock.clone();
        let right_dock = self.right_dock.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            // --- TitleBar ---------------------------------------------------
            .child(
                TitleBar::new()
                    .bg(rgb(PODIUM_CHROME))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            // [≡] Application menu — hamburger + dropdown
                            .child(
                                Button::new("app-menu")
                                    .icon(IconName::Menu)
                                    .ghost()
                                    .small()
                                    .dropdown_menu(|menu, _window, _cx| {
                                        menu.menu("Quit Podium", Box::new(Quit))
                                    }),
                            )
                            // Podium name
                            .child(
                                div()
                                    .text_color(cx.theme().foreground)
                                    .text_sm()
                                    .child("Podium"),
                            )
                            // Project switcher placeholder — Phase 2: Combobox
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
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .children(tab_info.into_iter().map(|(priority, tooltip, is_active)| {
                        div()
                            .px_3()
                            .py_2()
                            .text_sm()
                            .cursor_pointer()
                            .when(is_active, |this| {
                                this.border_b_2()
                                    .border_color(cx.theme().primary)
                                    .text_color(cx.theme().foreground)
                            })
                            .when(!is_active, |this| {
                                this.text_color(cx.theme().muted_foreground)
                            })
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
                            // Left dock
                            .child(
                                div()
                                    .when(left_open, |this| this.w(px(280.)))
                                    .when(!left_open, |this| this.w(px(0.)))
                                    .h_full()
                                    .overflow_hidden()
                                    .child(left_dock),
                            )
                            // Center (empty for Phase 1)
                            .child(
                                div()
                                    .flex_1()
                                    .h_full()
                                    .bg(cx.theme().background)
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
                            // Right dock (no panels in Phase 1)
                            .child(
                                div()
                                    .when(right_open, |this| this.w(px(280.)))
                                    .when(!right_open, |this| this.w(px(0.)))
                                    .h_full()
                                    .overflow_hidden()
                                    .child(right_dock),
                            ),
                    )
                    // Bottom dock
                    .child(
                        div()
                            .when(bottom_open, |this| this.h(px(240.)))
                            .when(!bottom_open, |this| this.h(px(0.)))
                            .w_full()
                            .overflow_hidden()
                            .child(bottom_dock),
                    ),
            )
            // --- StatusBar --------------------------------------------------
            .child(StatusBar::new().bg(rgb(PODIUM_CHROME)).left("Podium").right("Phase 1"))
            // --- Overlay layers ---------------------------------------------
            .children(Root::render_dialog_layer(window, cx))
            .children(Root::render_sheet_layer(window, cx))
            .children(Root::render_notification_layer(window, cx))
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx: &mut App| {
        gpui_component::init(cx);

        // Apply dark color palette, mode, and window appearance — all three required.
        {
            let theme = Theme::global_mut(cx);
            theme.colors = *ThemeColor::dark();
            theme.mode = ThemeMode::Dark;
        }
        cx.set_window_appearance(Some(WindowAppearance::Dark));

        cx.on_action(|_: &Quit, cx| cx.quit());

        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitleBar::title_bar_options()),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| PodiumApp::new(cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}
