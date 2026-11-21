//! Semantic theme tokens (docs/ui-design.md §5) and motion tokens (§6).
//!
//! Every color in presentation code comes from here — never a raw
//! literal. Custom tokens live in `theme.json` and are read by name;
//! each accessor wraps the lookup with an unmistakable fallback.
//! `AdaptiveColor` resolves light/dark at render time.

use nsh_base::{theme, tokens_of};
use nsh_core::color::Hsla;
use nsh_core::element::{AdaptiveColor, FONT_SANS};
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
token!(brand_bg, "brand-bg", secondary);
token!(brand_fg, "brand-fg", secondary_foreground);
token!(panel_bg, "panel-bg", background);
token!(panel_subtle, "panel-subtle", muted);
token!(panel_raised, "panel-raised", card);
token!(daybreak_bg, "daybreak-bg", card);
token!(daybreak_art, "daybreak-art", muted_foreground);
token!(daybreak_fg, "daybreak-fg", foreground);
token!(control_bg, "control-bg", secondary);
token!(control_hover, "control-hover", accent);
token!(fg_primary, "fg-primary", foreground);
token!(fg_body, "fg-body", foreground);
token!(fg_muted, "fg-muted", muted_foreground);
token!(border_subtle, "border-subtle", border);
token!(border_default, "border-default", border);
token!(accent, "accent", primary);
token!(accent_ink, "accent-ink", ring);
token!(accent_soft, "accent-soft", secondary);
token!(positive, "positive", primary);
token!(
    positive_foreground,
    "positive-foreground",
    primary_foreground
);
token!(warning, "warning", primary);
token!(warning_soft, "warning-soft", secondary);
token!(info, "info", primary);
token!(info_soft, "info-soft", secondary);
token!(danger, "danger", destructive);

// ---------------------------------------------------------------------
// Main accent presets (docs/ui-design.md §5)
// ---------------------------------------------------------------------

/// The persistence key of the default accent preset: `theme.json`'s
/// shipped coral, what every surface draws until a choice is persisted.
pub const DEFAULT_ACCENT_KEY: &str = "coral";

/// A named main accent preset (docs/ui-design.md §5). The brand accent
/// is user data: each preset carries the three semantic accent tokens
/// for both schemes, plus a check-glyph color that stays readable on
/// the marker disc. Hex strings, parsed when applied.
pub struct AccentPreset {
    /// The workspace-preference value that selects this preset.
    pub key: &'static str,
    /// The settings label.
    pub label: &'static str,
    /// `accent`, both schemes: markers, discs and rings.
    pub marker: &'static str,
    /// `accent-ink` in light: text on the soft wash; the picker's ring.
    pub ink_light: &'static str,
    /// `accent-ink` in dark.
    pub ink_dark: &'static str,
    /// `accent-soft` in light: highlighted chips and selected rows.
    pub soft_light: &'static str,
    /// `accent-soft` in dark.
    pub soft_dark: &'static str,
    /// The picker's check glyph: contrasts with the marker.
    pub on_marker: &'static str,
}

/// The five presets of docs/ui-design.md §5, the default coral first.
/// The coral values equal `theme.json`'s shipped tokens, so selecting
/// it restores the palette the app ships with. Every `ink` meets WCAG AA
/// on its `soft` wash (4.5:1) in both schemes.
pub const ACCENT_PRESETS: [AccentPreset; 5] = [
    AccentPreset {
        key: "coral",
        label: "珊瑚红",
        marker: "#ff4f4f",
        ink_light: "#b92e2d",
        ink_dark: "#ff8c8c",
        soft_light: "#fdece9",
        soft_dark: "#332321",
        on_marker: "#26200f",
    },
    AccentPreset {
        key: "green",
        label: "语雀绿",
        marker: "#00b96b",
        ink_light: "#0a7646",
        ink_dark: "#93dcb8",
        soft_light: "#e4f6ed",
        soft_dark: "#193124",
        on_marker: "#26200f",
    },
    AccentPreset {
        key: "cobalt",
        label: "钴蓝",
        marker: "#3b82f6",
        ink_light: "#325fa5",
        ink_dark: "#a9c8fa",
        soft_light: "#eaf1fb",
        soft_dark: "#202a34",
        on_marker: "#26200f",
    },
    AccentPreset {
        key: "amber",
        label: "琥珀金",
        marker: "#d99026",
        ink_light: "#8a6220",
        ink_dark: "#e8bd7d",
        soft_light: "#f9f2e6",
        soft_dark: "#332c1b",
        on_marker: "#26200f",
    },
    AccentPreset {
        key: "violet",
        label: "紫罗兰",
        marker: "#7c5dfa",
        ink_light: "#5a48a7",
        ink_dark: "#cabcfc",
        soft_light: "#f0edfb",
        soft_dark: "#282635",
        on_marker: "#ffffff",
    },
];

/// The preset the key names, if it names one.
pub fn accent_preset(key: &str) -> Option<&'static AccentPreset> {
    ACCENT_PRESETS.iter().find(|preset| preset.key == key)
}

impl AccentPreset {
    /// `accent`: the marker color of both schemes.
    pub fn marker_color(&self) -> AdaptiveColor {
        let marker = preset_hex(self.marker);
        AdaptiveColor::new(marker, marker)
    }

    /// `accent-ink`: readable on the soft wash of each scheme.
    pub fn ink_color(&self) -> AdaptiveColor {
        AdaptiveColor::new(preset_hex(self.ink_light), preset_hex(self.ink_dark))
    }

    /// `accent-soft`: the highlighted wash of each scheme.
    pub fn soft_color(&self) -> AdaptiveColor {
        AdaptiveColor::new(preset_hex(self.soft_light), preset_hex(self.soft_dark))
    }

    /// The picker's check glyph: readable on the marker disc.
    pub fn on_marker_color(&self) -> AdaptiveColor {
        let on_marker = preset_hex(self.on_marker);
        AdaptiveColor::new(on_marker, on_marker)
    }
}

/// Parses a preset's hard-coded hex; a broken literal is a bug, not a
/// runtime condition, so it fails loudly.
fn preset_hex(value: &str) -> Hsla {
    Hsla::parse_hex(value).unwrap_or_else(|error| panic!("preset hex {value:?} parses: {error}"))
}

/// The app's shipped tokens of `theme.json` (what a `None` choice
/// restores); a broken theme file is a bug, but an absent restore source
/// must not wipe the live palette, so it stays `None`.
fn shipped_tokens() -> Option<nsh_core::element::Tokens> {
    tokens_of(Some(include_str!("../../theme.json"))).ok()
}

/// Clones the live tokens, lets `edit` change them, and installs them.
/// Every applied choice (accent preset, UI font, text size, density)
/// survives the others' apply calls — they share this one path.
/// Bindings that read the tokens re-run with the new values.
fn edit_tokens(edit: impl FnOnce(&mut nsh_core::element::Tokens)) {
    let mut tokens = nsh_core::element::tokens().as_ref().clone();
    edit(&mut tokens);
    nsh_core::element::set_tokens(tokens);
}

/// Applies the preset named `key` to the semantic accent tokens
/// (`accent`, `accent-ink`, `accent-soft`) of both schemes; `None` or an
/// unknown key restores `theme.json`'s shipped accent. The rest of the
/// palette, the radii and the titlebar theme are untouched.
///
/// Bindings that read the tokens re-run with the new values; a control
/// that captured an accent color *by value* (not `move ||`) keeps the
/// old color until it is rebuilt — the same constraint as any runtime
/// token change in NSH.
pub fn apply_accent(key: Option<&str>) {
    edit_tokens(|tokens| {
        if let Some(preset) = key.and_then(accent_preset) {
            tokens.set_color("accent", preset.marker_color());
            tokens.set_color("accent-ink", preset.ink_color());
            tokens.set_color("accent-soft", preset.soft_color());
        } else if let Some(shipped) = shipped_tokens() {
            for name in ["accent", "accent-ink", "accent-soft"] {
                if let Some(color) = shipped.color(name) {
                    tokens.set_color(name, color);
                }
            }
        }
    });
}

// ---------------------------------------------------------------------
// Appearance settings: UI font, text size, density
// (docs/kanon.html 设置·外观, applied through theme tokens so every
// binding that reads them re-runs, like the accent preset above)
// ---------------------------------------------------------------------

/// The `text-scale` number token: the text-size setting's live value.
pub const TOKEN_TEXT_SCALE: &str = "text-scale";
/// The `density` number token: 0 comfortable, 1 compact.
pub const TOKEN_DENSITY: &str = "density";

/// A named text-size preset (docs/kanon.html `textScale`: 0.92 / 1 / 1.08).
pub struct TextSizePreset {
    /// The workspace-preference value (`PREF_TEXT_SIZE`).
    pub key: &'static str,
    /// The settings label.
    pub label: &'static str,
    /// The multiplier applied to every text size that opts in via
    /// [`text_px`].
    pub scale: f32,
}

/// The three choices of docs/kanon.html 设置·外观, the default first.
pub const TEXT_SIZE_PRESETS: [TextSizePreset; 3] = [
    TextSizePreset {
        key: "standard",
        label: "默认",
        scale: 1.0,
    },
    TextSizePreset {
        key: "small",
        label: "较小",
        scale: 0.92,
    },
    TextSizePreset {
        key: "large",
        label: "较大",
        scale: 1.08,
    },
];

/// The preset the key names; `None` and unknown keys are the default.
pub fn text_size_preset(key: Option<&str>) -> &'static TextSizePreset {
    key.and_then(|key| TEXT_SIZE_PRESETS.iter().find(|preset| preset.key == key))
        .unwrap_or(&TEXT_SIZE_PRESETS[0])
}

/// Applies the text-size preset named `key` by setting the `text-scale`
/// token; `None` or an unknown key restores the default scale.
pub fn apply_text_size(key: Option<&str>) {
    let scale = text_size_preset(key).scale;
    edit_tokens(|tokens| tokens.set_number(TOKEN_TEXT_SCALE, scale));
}

/// The live text scale (1.0 until a choice is applied). Bindings re-run.
pub fn text_scale() -> f32 {
    theme().number(TOKEN_TEXT_SCALE).unwrap_or(1.0)
}

/// A text size that follows the text-size setting:
/// `.text_size(text_px(13.0))`. Sizes written as raw `px(...)` stay
/// unscaled — that is the documented reachable coverage of the setting.
pub fn text_px(base: f32) -> impl Fn() -> Points + Copy + 'static {
    move || px(base * text_scale())
}

/// The UI font choice (docs/kanon.html `fontFamily`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiFont {
    /// The app ships with it: `theme.json`'s `font-sans` stack.
    System,
    /// The platform's rounded sans where it has one.
    Rounded,
    /// A serif reading face.
    Serif,
}

impl UiFont {
    /// The three choices in settings order (docs/kanon.html).
    pub const ALL: [UiFont; 3] = [UiFont::System, UiFont::Rounded, UiFont::Serif];

    /// The workspace-preference value (`PREF_UI_FONT`).
    pub const fn key(self) -> &'static str {
        match self {
            UiFont::System => "system",
            UiFont::Rounded => "rounded",
            UiFont::Serif => "serif",
        }
    }

    /// The settings label.
    pub const fn label(self) -> &'static str {
        match self {
            UiFont::System => "系统默认",
            UiFont::Rounded => "圆润无衬线",
            UiFont::Serif => "衬线阅读",
        }
    }

    /// The choice the key names; `None` and unknown keys are the system
    /// font (the shipped `theme.json` stack).
    pub fn from_key(key: Option<&str>) -> UiFont {
        match key {
            Some("rounded") => UiFont::Rounded,
            Some("serif") => UiFont::Serif,
            _ => UiFont::System,
        }
    }

    /// The `font-family` stack (docs/kanon.html `fontStacks`), resolved
    /// by the platform best-available first.
    pub fn family(self) -> &'static str {
        match self {
            UiFont::System => {
                "\"PingFang SC\", \"Microsoft YaHei\", -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif"
            }
            UiFont::Rounded => {
                "ui-rounded, \"SF Pro Rounded\", \"PingFang SC\", \"Hiragino Sans GB\", \"Microsoft YaHei\", sans-serif"
            }
            UiFont::Serif => {
                "\"Songti SC\", \"Noto Serif SC\", Georgia, \"Times New Roman\", serif"
            }
        }
    }
}

/// Applies the UI font by setting the engine's `font-sans` token: every
/// text that sets no family of its own (the whole app and the platform's
/// own chrome) redraws in it, with text relaid out. `None` or an unknown
/// key restores `theme.json`'s shipped stack.
pub fn apply_ui_font(key: Option<&str>) {
    edit_tokens(|tokens| {
        let family = match UiFont::from_key(key) {
            UiFont::System => {
                match shipped_tokens().and_then(|shipped| shipped.font(FONT_SANS).cloned()) {
                    Some(shipped) => shipped,
                    // theme.json unreadable: keep whatever stack is live.
                    None => return,
                }
            }
            font => font.family().into(),
        };
        tokens.set_font(FONT_SANS, family);
    });
}

/// The density choice (docs/kanon.html `density`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Density {
    Comfortable,
    Compact,
}

impl Density {
    /// The two choices in settings order (docs/kanon.html).
    pub const ALL: [Density; 2] = [Density::Comfortable, Density::Compact];

    /// The workspace-preference value (`PREF_DENSITY`).
    pub const fn key(self) -> &'static str {
        match self {
            Density::Comfortable => "comfortable",
            Density::Compact => "compact",
        }
    }

    /// The settings label.
    pub const fn label(self) -> &'static str {
        match self {
            Density::Comfortable => "舒适",
            Density::Compact => "紧凑",
        }
    }

    /// The choice the key names; `None` and unknown keys are comfortable.
    pub fn from_key(key: Option<&str>) -> Density {
        match key {
            Some("compact") => Density::Compact,
            _ => Density::Comfortable,
        }
    }
}

/// Applies the density named `key` by setting the `density` token;
/// `None` or an unknown key restores comfortable.
pub fn apply_density(key: Option<&str>) {
    let compact = Density::from_key(key) == Density::Compact;
    edit_tokens(|tokens| tokens.set_number(TOKEN_DENSITY, if compact { 1.0 } else { 0.0 }));
}

/// The live density (comfortable until a choice is applied). Bindings
/// re-run.
pub fn density() -> Density {
    match theme().number(TOKEN_DENSITY) {
        Some(1.0) => Density::Compact,
        _ => Density::Comfortable,
    }
}

/// Whether the density setting is compact; for `style_when`/`when`.
pub fn is_compact() -> impl Fn() -> bool + Copy + 'static {
    move || density() == Density::Compact
}

/// Buttons and input fields (docs/ui-design.md §5.2): 32 pt standard,
/// 28 pt compact.
pub fn control_h() -> impl Fn() -> Points + Copy + 'static {
    move || {
        px(if density() == Density::Compact {
            28.0
        } else {
            32.0
        })
    }
}

/// Task and list rows (docs/ui-design.md §5.2): 50 pt standard,
/// 42 pt compact.
pub fn row_min_h() -> impl Fn() -> Points + Copy + 'static {
    move || {
        px(if density() == Density::Compact {
            42.0
        } else {
            50.0
        })
    }
}

/// Sidebar nav and catalog rows (docs/ui-design.md §5.2: 32–34 pt
/// standard; docs/kanon.html compact side items 28 pt).
pub fn side_row_h() -> impl Fn() -> Points + Copy + 'static {
    move || {
        px(if density() == Density::Compact {
            28.0
        } else {
            32.0
        })
    }
}

pub fn overlay() -> AdaptiveColor {
    shell_bg().opacity(0.48)
}

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
