//! PodiumColors — Podium-specific semantic color tokens.
//!
//! gpui-component's `ThemeColor` provides a rich set of named tokens. After
//! inspecting the `theme_color.rs` source, the following UI-element tokens
//! are directly available and used here:
//!
//!   `title_bar`        — title bar background
//!   `title_bar_border` — title bar bottom border
//!   `tab_bar`          — tab bar row background
//!   `status_bar`       — status bar background
//!   `popover`          — dropdown / popup background (set in main.rs init)
//!
//! `PodiumColors` reads these tokens from `cx.theme()` directly where they
//! exist, and falls back to the single remaining hex constant (`TAB_BAR_BG`)
//! only for the tab bar background, which `ThemeColor::dark()` initialises
//! too dark by default for Podium's three-level visual hierarchy.
//!
//! It is the **single source of truth for all color decisions** in Podium
//! render code. All render methods call `cx.podium_colors()`.
//!
//! ## Why not lift Zed's theme crate?
//!
//! Zed's `crates/theme/` is GPL-3.0-or-later. Podium is Apache-2.0. The
//! GPL crate cannot be incorporated. `PodiumColors` independently implements
//! the same *pattern* without copying any GPL code.
//!
//! ## Phase 10 upgrade path
//!
//! Phase 10 will add a JSON-driven theme system. `PodiumColors::from_cx`
//! will then read from a `GlobalPodiumTheme` rather than the constants here.
//! Call sites in render code do not change — only this file changes.

use gpui::{App, Hsla, rgb};
use gpui_component::ActiveTheme as _;

// ---------------------------------------------------------------------------
// Remaining hex constant
//
// Gruvbox Dark bg1 (`#3c3836`) — used for the tab bar background.
//
// `ThemeColor::dark()` sets `tab_bar` to a near-black value that makes the
// tab bar indistinguishable from the content area. Overriding it here to
// Gruvbox bg1 creates the three-level visual hierarchy:
//   TitleBar  (cx.theme().title_bar  ≈ #4c4642)  warm chrome
//   TabBar    (TAB_BAR_BG            = #3c3836)  one step cooler
//   Content   (cx.theme().background ≈ #282828)  darkest floor
//
// Phase 10: replace with a theme-driven value from GlobalPodiumTheme.
// ---------------------------------------------------------------------------

/// Tab bar background override — Gruvbox Dark bg1 (`#3c3836`).
const TAB_BAR_BG: u32 = 0x3c3836;

// ---------------------------------------------------------------------------
// PodiumColors
// ---------------------------------------------------------------------------

/// Podium-specific semantic color tokens, derived from the active
/// gpui-component theme at render time.
///
/// Construction is cheap — no allocation, no locking, just field reads and
/// `Hsla` copies. Call `cx.podium_colors()` at the top of any render method.
///
/// # Usage
///
/// ```rust
/// use crate::colors::PodiumColorsExt as _;
///
/// let colors = cx.podium_colors();
/// TitleBar::new().bg(colors.title_bar_background)
/// div().bg(colors.panel_background)
/// ```
pub struct PodiumColors {
    // --- Chrome -------------------------------------------------------------
    /// Background for `TitleBar` and `StatusBar`.
    ///
    /// Reads `cx.theme().title_bar` — the dedicated title bar token confirmed
    /// present in gpui-component's `ThemeColor` struct.
    pub title_bar_background: Hsla,

    // --- Content areas ------------------------------------------------------
    /// Background for panel content areas (the interior of open docks).
    ///
    /// Mapped to `cx.theme().secondary` (Gruvbox bg2, `#504945`) — slightly
    /// lighter than content so open docks have a subtle visual presence.
    pub panel_background: Hsla,

    /// Background for the main center content area (editor pane in later phases).
    ///
    /// Mapped to `cx.theme().background` (Gruvbox bg0, `#282828`) — the
    /// darkest surface, the visual floor of the UI.
    pub content_background: Hsla,

    // --- Tab bar ------------------------------------------------------------
    /// Background for the tab bar row between the TitleBar and content area.
    ///
    /// Uses `TAB_BAR_BG` (`#3c3836`) rather than `cx.theme().tab_bar` because
    /// gpui-component's dark palette initialises `tab_bar` too dark — it
    /// renders indistinguishable from the content background. The override
    /// creates the visible three-level chrome → tab bar → content hierarchy.
    pub tab_bar_background: Hsla,

    /// Text color for the active (selected) tab label.
    pub tab_active_foreground: Hsla,

    /// Text color for inactive (unselected) tab labels.
    pub tab_inactive_foreground: Hsla,

    /// Color of the underline drawn beneath the active tab label.
    pub tab_active_indicator: Hsla,

    // --- Borders ------------------------------------------------------------
    /// Border drawn on the inner edge of each open dock (facing the content area).
    pub panel_border: Hsla,

    /// Border drawn on the bottom edge of the tab bar row.
    pub tab_bar_border: Hsla,

    /// Border drawn on the bottom edge of the title bar.
    ///
    /// Reads `cx.theme().title_bar_border` — confirmed present in ThemeColor.
    pub title_bar_border: Hsla,
}

impl PodiumColors {
    /// Derive `PodiumColors` from the active gpui-component theme.
    pub fn from_cx(cx: &App) -> Self {
        let theme = cx.theme();

        Self {
            // Use the dedicated title_bar token — confirmed in ThemeColor source.
            title_bar_background: theme.title_bar,
            panel_background: theme.secondary,
            content_background: theme.background,
            // Override tab_bar: gpui-component's dark default is too dark.
            // TAB_BAR_BG (#3c3836) sits visually between title_bar and background.
            tab_bar_background: rgb(TAB_BAR_BG).into(),
            tab_active_foreground: theme.foreground,
            tab_inactive_foreground: theme.muted_foreground,
            tab_active_indicator: theme.primary,
            panel_border: theme.border,
            tab_bar_border: theme.border,
            // Use the dedicated title_bar_border token — confirmed in ThemeColor source.
            title_bar_border: theme.title_bar_border,
        }
    }
}

// ---------------------------------------------------------------------------
// PodiumColorsExt — convenience accessor on the GPUI context
// ---------------------------------------------------------------------------

/// Extension trait adding `podium_colors()` to the GPUI app context.
///
/// Implemented on `App` so it is accessible from `&mut Context<T>` in render
/// methods via `Deref` coercion.
pub trait PodiumColorsExt {
    /// Derive `PodiumColors` from the current theme state.
    fn podium_colors(&self) -> PodiumColors;
}

impl PodiumColorsExt for App {
    fn podium_colors(&self) -> PodiumColors {
        PodiumColors::from_cx(self)
    }
}
