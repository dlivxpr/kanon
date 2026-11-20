//! Semantic theme tokens (docs/ui-design.md §5) and motion tokens (§6).
//!
//! Every color in presentation code comes from here — never a raw
//! literal. Custom tokens live in `theme.json` and are read by name;
//! each accessor wraps the lookup with an unmistakable fallback.
//! `AdaptiveColor` resolves light/dark at render time.

use nsh_base::theme;
use nsh_core::element::AdaptiveColor;
use nsh_core::prelude::*;

macro_rules! token {
    ($fn_name:ident, $name:literal, $fallback:ident) => {
        pub fn $fn_name() -> AdaptiveColor {
            theme().color($name).unwrap_or_else(|| theme().$fallback)
        }
    };
}

token!(shell_bg, "shell-bg", background);
token!(shell_surface, "shell-surface", muted);
token!(shell_surface_active, "shell-surface-active", secondary);
token!(shell_fg, "shell-fg", foreground);
token!(shell_muted, "shell-muted", muted_foreground);
token!(shell_border, "shell-border", border);
token!(panel_bg, "panel-bg", background);
token!(panel_subtle, "panel-subtle", muted);
token!(control_bg, "control-bg", secondary);
token!(control_hover, "control-hover", accent);
token!(fg_primary, "fg-primary", foreground);
token!(fg_body, "fg-body", foreground);
token!(fg_muted, "fg-muted", muted_foreground);
token!(border_subtle, "border-subtle", border);
token!(border_default, "border-default", border);
token!(accent, "accent", primary);
token!(accent_soft, "accent-soft", secondary);
token!(positive, "positive", primary);
token!(warning, "warning", primary);
token!(info, "info", primary);
token!(info_soft, "info-soft", secondary);
token!(danger, "danger", destructive);

/// A radius token in points (e.g. `radius-card` = 12).
pub fn radius(token: &str, fallback: f32) -> Points {
    px(theme().number(token).unwrap_or(fallback))
}

/// The main panel corner (`radius-panel` = 14).
pub fn radius_panel() -> Points {
    radius("radius-panel", 14.0)
}

/// Cards, popups and date cells (`radius-card` = 12).
pub fn radius_card() -> Points {
    radius("radius-card", 12.0)
}

/// Sidebar rows, tabs, list rows (`radius-row` = 8).
pub fn radius_row() -> Points {
    radius("radius-row", 8.0)
}

// ---------------------------------------------------------------------
// Motion tokens (docs/ui-design.md §6.1)
// ---------------------------------------------------------------------

/// Hover washes, icon color swaps.
pub const MOTION_INSTANT_MS: f32 = 120.0;
/// Press scales, checkbox checks, tab open/close.
pub const MOTION_FAST_MS: f32 = 180.0;

/// A transition duration that collapses to instant under
/// `prefers-reduced-motion`.
pub fn ms(duration: f32) -> impl Fn() -> f32 + 'static {
    move || if reduce_motion() { 0.0 } else { duration }
}
