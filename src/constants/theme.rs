//! CDC-branded Plotly style config.
//!
//! Colors and fonts mirror `MIRA/frontend/src/theme.css` so every figure
//! mira-oxide emits matches the web app. There are three consumers:
//!   * the Rust `plotly` crate  -> [`cdc_template`] (a `plotly::Template`)
//!   * hand-built JSON figures  -> [`layout_json`] / [`title_font_json`]
//!   * embedded HTML/JS         -> the `*_CSS` / [`js_layout_defaults`] helpers

use plotly::{
    common::{Font, Title},
    layout::{Axis, LayoutTemplate, Template},
};
use serde_json::{Value, json};

// ── CDC brand palette (from theme.css) ──────────────────────────────────────
pub const CDC_BLUE: &str = "#0057B7"; // primary
pub const CDC_NAVY: &str = "#032659"; // secondary / titles
pub const CDC_BLUE_1: &str = "#3382CF"; // info
pub const CDC_TEAL: &str = "#0081A1"; // accent
pub const CDC_RED: &str = "#CC1B22"; // destructive
pub const CDC_YELLOW: &str = "#DE8A05"; // warning
pub const CHARCOAL: &str = "#333333"; // body text
pub const GRAY: &str = "#6D6E71"; // muted text / axis lines
pub const WHITE: &str = "#FFFFFF"; // canvas
pub const MUTED: &str = "#ECF5FF"; // pale blue panel
pub const BORDER: &str = "#B8D4ED"; // pale blue border / gridlines

// ── CDC brand fonts (from theme.css) ────────────────────────────────────────
pub const TITLE_FONT: &str = "Roboto, system-ui, -apple-system, sans-serif";
pub const BODY_FONT: &str = "'Nunito Sans', system-ui, -apple-system, sans-serif";
pub const MONO_FONT: &str = "'Roboto Mono', SFMono-Regular, Menlo, Consolas, monospace";

/// Qualitative data-series colorway, CDC brand order.
#[must_use]
pub fn colorway() -> Vec<&'static str> {
    vec![
        CDC_BLUE, CDC_TEAL, CDC_YELLOW, CDC_RED, CDC_BLUE_1, CDC_NAVY, "#00B1CE", "#5796D9",
        "#F0695E", GRAY,
    ]
}

/// CDC palette for ORF/gene boxes and non-flu (RSV, SC2) coverage segments.
#[must_use]
pub fn orf_palette() -> Vec<&'static str> {
    vec![
        CDC_BLUE, CDC_RED, CDC_TEAL, CDC_NAVY, CDC_YELLOW, CDC_BLUE_1,
        "#00B1CE", // light teal
        "#F0695E", // coral
        "#5796D9", // light blue
        GRAY, BORDER, "#A31419", // dark red
    ]
}

/// Body font for plot text (Nunito Sans, charcoal).
#[must_use]
pub fn body_font() -> Font {
    Font::new().family(BODY_FONT).size(13).color(CHARCOAL)
}

/// Title font for plot titles (Roboto, CDC navy).
#[must_use]
pub fn title_font() -> Font {
    Font::new().family(TITLE_FONT).size(20).color(CDC_NAVY)
}

/// Shared `plotly` crate template. Apply with `Layout::new().template(cdc_template())`.
#[must_use]
pub fn cdc_template() -> Template {
    let axis = || {
        Axis::new()
            .grid_color(BORDER)
            .line_color(GRAY)
            .zero_line_color(BORDER)
    };
    let layout = LayoutTemplate::new()
        .font(body_font())
        .title(Title::new().font(title_font()))
        .paper_background_color(WHITE)
        .plot_background_color(WHITE)
        .colorway(colorway())
        .x_axis(axis())
        .y_axis(axis());
    Template::new().layout(layout)
}

/// Title `font` object for hand-built JSON figures.
#[must_use]
pub fn title_font_json() -> Value {
    json!({ "family": TITLE_FONT, "size": 20, "color": CDC_NAVY })
}

/// Layout defaults for hand-built JSON figures (font, title, colorway, backgrounds).
#[must_use]
pub fn layout_json() -> Value {
    json!({
        "font": { "family": BODY_FONT, "color": CHARCOAL, "size": 13 },
        "title": { "font": title_font_json(), "x": 0.05 },
        "colorway": colorway(),
        "paper_bgcolor": WHITE,
        "plot_bgcolor": WHITE,
    })
}

/// JS object literal of layout defaults, merged under each figure via
/// `Object.assign({}, THEME, fig.layout)` in embedded HTML.
#[must_use]
pub fn js_layout_defaults() -> String {
    layout_json().to_string()
}

// ── CSS snippets for embedded HTML reports ──────────────────────────────────
pub const FONT_IMPORT: &str = "<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">\n<link href=\"https://fonts.googleapis.com/css2?family=Nunito+Sans:wght@400;600;700&family=Roboto:wght@400;700&family=Roboto+Mono&display=swap\" rel=\"stylesheet\">";
