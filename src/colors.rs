//! PodiumColors — Podium-specific semantic color tokens.
//!
//! All colors are driven by the active gpui-component theme, which is set to
//! Gruvbox Dark at startup via `ThemeRegistry::load_themes_from_str` in
//! `main.rs`. `PodiumColors` reads semantic tokens from `cx.theme()` and
//! provides them under Podium-specific names to render code.
//!
//! ## Single remaining constant
//!
//! `CONTENT_BG` (`#1d2021`) is the only color not driven by the theme file.
//! It is the content area floor — intentionally darker than `theme.background`
//! (`#32302f`, the Sheet surface) to create depth beneath all overlays and
//! panels. The theme file drives everything else.
//!
//! ## Why not lift Zed's theme crate?
//!
//! Zed's `crates/theme/` is GPL-3.0-or-later. Podium is Apache-2.0. The
//! GPL crate cannot be incorporated. `PodiumColors` independently implements
//! the same *pattern* without copying any GPL code.
//!
//! ## Phase 10 upgrade path
//!
//! Phase 10 adds additional theme choices (One Dark, Solarized, etc.) and a
//! theme selector in Settings. The `CONTENT_BG` constant moves into the theme
//! JSON as a custom field, or is derived as a darkened `background`. Call
//! sites in render code do not change — only this file changes.

use gpui::{App, Hsla, rgb};
use gpui_component::ActiveTheme as _;

// ---------------------------------------------------------------------------
// Content floor constant
//
// Gruvbox bg0_hard (#1d2021) — the darkest surface in the hierarchy.
// Sits below the Sheet background (#32302f) driven by theme.background.
//
// Phase 10: derive from theme file or compute as theme.background.darken(x).
// ---------------------------------------------------------------------------

/// Gruvbox Dark bg0_hard — main content area floor.
const CONTENT_BG: u32 = 0x1d2021;

// ---------------------------------------------------------------------------
// PodiumColors
// ---------------------------------------------------------------------------

/// Podium-specific semantic color tokens, derived from the active theme.
///
/// Construction is cheap — no allocation, no locking, just field reads.
/// Call `cx.podium_colors()` at the top of any render method that needs
/// multiple color values.
pub struct PodiumColors {
    /// TitleBar and StatusBar background. Reads `theme.title_bar`.
    pub title_bar_background: Hsla,
    /// Title bar bottom border. Reads `theme.title_bar_border`.
    pub title_bar_border: Hsla,
    /// Open dock interior background. Reads `theme.secondary`.
    pub panel_background: Hsla,
    /// Main content area floor. Darker than Sheet/overlay bg for depth.
    pub content_background: Hsla,
    /// Tab bar row background. Reads `theme.tab_bar`.
    pub tab_bar_background: Hsla,
    /// Active tab label text color. Reads `theme.foreground`.
    pub tab_active_foreground: Hsla,
    /// Inactive tab label text color. Reads `theme.muted_foreground`.
    pub tab_inactive_foreground: Hsla,
    /// Active tab underline indicator color. Reads `theme.primary`.
    pub tab_active_indicator: Hsla,
    /// Dock inner edge border. Reads `theme.border`.
    pub panel_border: Hsla,
    /// Tab bar bottom border. Reads `theme.border`.
    pub tab_bar_border: Hsla,
}

impl PodiumColors {
    /// Derive `PodiumColors` from the active gpui-component theme.
    pub fn from_cx(cx: &App) -> Self {
        let theme = cx.theme();
        Self {
            title_bar_background:    theme.title_bar,
            title_bar_border:        theme.title_bar_border,
            panel_background:        theme.secondary,
            content_background:      rgb(CONTENT_BG).into(),
            tab_bar_background:      theme.tab_bar,
            tab_active_foreground:   theme.foreground,
            tab_inactive_foreground: theme.muted_foreground,
            tab_active_indicator:    theme.primary,
            panel_border:            theme.border,
            tab_bar_border:          theme.border,
        }
    }
}

// ---------------------------------------------------------------------------
// PodiumColorsExt — convenience accessor on the GPUI context
// ---------------------------------------------------------------------------

/// Extension trait adding `podium_colors()` to the GPUI app context.
pub trait PodiumColorsExt {
    fn podium_colors(&self) -> PodiumColors;
}

impl PodiumColorsExt for App {
    fn podium_colors(&self) -> PodiumColors {
        PodiumColors::from_cx(self)
    }
}
