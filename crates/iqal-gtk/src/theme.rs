//! Iqal's look, the same as on Windows: ivory and deep red in light mode,
//! charcoal with ivory in dark mode. One style sheet, made from the colours
//! below, styles every window; it is replaced when the mode changes.

use std::cell::{Cell, RefCell};

use gtk::prelude::*;
use iqal_app::settings::GuiSettings;

/// Iqal's colours (the same values as the Windows app's).
pub struct Palette {
    /// The window's background.
    pub page: &'static str,
    /// Lists.
    pub list: &'static str,
    /// Text boxes.
    pub field: &'static str,
    pub text: &'static str,
    /// Explanations and other quiet text.
    pub grey: &'static str,
    /// Main buttons, switches that are on, ticked boxes, the chosen tab.
    pub accent: &'static str,
    pub on_accent: &'static str,
    pub accent_hot: &'static str,
    pub accent_down: &'static str,
    /// Outlines.
    pub line: &'static str,
    /// The chosen row of a list.
    pub chosen: &'static str,
    pub chosen_text: &'static str,
    /// Other buttons.
    pub button: &'static str,
    pub button_hot: &'static str,
    pub button_down: &'static str,
    /// The stripe on the edge of the chosen block in the block list.
    pub stripe: &'static str,
    /// The gold labels in the block list.
    pub badge: &'static str,
    pub badge_text: &'static str,
    pub badge_line: &'static str,
    /// Warnings (amber) and errors.
    pub warn: &'static str,
    pub error: &'static str,
    /// The mark on a message's round icon.
    pub on_sign: &'static str,
}

pub const LIGHT: Palette = Palette {
    page: "#F6F1E7",
    list: "#FFFCF6",
    field: "#FFFFFF",
    text: "#3A1F18",
    grey: "#7A5F55",
    accent: "#6B1F2A",
    on_accent: "#F6F1E7",
    accent_hot: "#802B37",
    accent_down: "#561821",
    line: "#D9CBB5",
    chosen: "#F3E1DE",
    chosen_text: "#6B1F2A",
    button: "#FFFCF6",
    button_hot: "#F1E6D6",
    button_down: "#E6D6C0",
    stripe: "#6B1F2A",
    badge: "#EFE3CC",
    badge_text: "#6B4E1F",
    badge_line: "#C9A24A",
    warn: "#A8620A",
    error: "#6B1F2A",
    on_sign: "#F6F1E7",
};

pub const DARK: Palette = Palette {
    page: "#1A1A1A",
    list: "#242424",
    field: "#202020",
    text: "#F6F1E7",
    grey: "#B5AEA2",
    accent: "#F6F1E7",
    on_accent: "#6B1F2A",
    accent_hot: "#FFFCF6",
    accent_down: "#DED5C6",
    line: "#383838",
    chosen: "#3A2226",
    chosen_text: "#F6F1E7",
    button: "#242424",
    button_hot: "#2E2E2E",
    button_down: "#383838",
    stripe: "#B84A5A",
    badge: "#3B3020",
    badge_text: "#E8C98A",
    badge_line: "#8A6D2E",
    warn: "#E8A84C",
    error: "#E06C75",
    on_sign: "#1A1A1A",
};

thread_local! {
    static DARK_MODE: Cell<bool> = const { Cell::new(false) };
    static PROVIDER: RefCell<Option<gtk::CssProvider>> = const { RefCell::new(None) };
}

pub fn is_dark() -> bool {
    DARK_MODE.with(Cell::get)
}

pub fn palette() -> &'static Palette {
    if is_dark() { &DARK } else { &LIGHT }
}

/// Whether the desktop is set to dark (for "same as the system").
fn system_is_dark() -> bool {
    let Some(settings) = gtk::Settings::default() else { return false };
    if settings.is_gtk_application_prefer_dark_theme() {
        return true;
    }
    settings.gtk_theme_name().is_some_and(|n| n.to_lowercase().contains("dark"))
}

/// Works out light or dark from Iqal's setting (or the desktop's) and styles
/// every window with it. Called at start and when the setting changes.
pub fn apply() {
    let Some(display) = gtk::gdk::Display::default() else { return };
    let dark = match GuiSettings::load().theme.as_deref() {
        Some("dark") => true,
        Some("light") => false,
        _ => system_is_dark(),
    };
    DARK_MODE.with(|d| d.set(dark));
    // Iqal's own icons (the globe) are installed with the package in the
    // standard icon folder; when run from a build, from the source folder.
    let icons = gtk::IconTheme::for_display(&display);
    for dir in [concat!(env!("CARGO_MANIFEST_DIR"), "/../../packaging/linux/icons")] {
        if std::path::Path::new(dir).is_dir() && !icons.search_path().iter().any(|p| p.as_path() == std::path::Path::new(dir)) {
            icons.add_search_path(dir);
        }
    }
    // (GTK's own light or dark style is left as it is: switching it makes
    // GTK reload its whole style sheet, which slowed opening by about 15 ms,
    // and Iqal's style sheet below colours every part anyway.)
    let css = gtk::CssProvider::new();
    css.load_from_string(&style(palette(), display.is_composited()));
    PROVIDER.with(|p| {
        if let Some(old) = p.borrow_mut().replace(css.clone()) {
            gtk::style_context_remove_provider_for_display(&display, &old);
        }
    });
    gtk::style_context_add_provider_for_display(&display, &css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
}

/// The style sheet for one set of colours.
fn style(p: &Palette, composited: bool) -> String {
    let mut css = format!(
        r#"
window, window.background, dialog, .background {{ background-color: {page}; color: {text}; }}
headerbar {{ background: {page}; color: {text}; box-shadow: none; border-bottom: 1px solid {line}; min-height: 34px; }}
headerbar .title {{ font-weight: normal; }}
label {{ color: inherit; }}
.dim-label, label.note {{ color: {grey}; opacity: 1; }}
.heading {{ font-weight: 600; }}
label.warning-text {{ color: {warn}; }}
separator {{ background: {line}; }}

/* Buttons: rounded, lighter under the mouse, darker pressed. */
button {{
    background: {button}; color: {text}; border: 1px solid {line}; border-radius: 6px;
    box-shadow: none; text-shadow: none; -gtk-icon-shadow: none; padding: 5px 14px; min-height: 22px;
}}
button:hover {{ background: {button_hot}; }}
button:active, button:checked {{ background: {button_down}; }}
button:disabled {{ background: {page}; color: {grey}; border-color: {line}; }}
button.main, button.suggested-action {{ background: {accent}; color: {on_accent}; border-color: {accent}; }}
button.main:hover, button.suggested-action:hover {{ background: {accent_hot}; }}
button.main:active, button.suggested-action:active {{ background: {accent_down}; }}
button.main:disabled, button.suggested-action:disabled {{ background: {page}; color: {grey}; border-color: {line}; }}
button.quiet {{ color: {grey}; }}
button.quiet image.danger {{ color: {error}; }}
button.flat {{ background: transparent; border-color: transparent; }}
button.flat:hover {{ background: {button_hot}; }}
button image {{ -gtk-icon-size: 16px; }}
button > box {{ border-spacing: 6px; }}

/* Text boxes: white (charcoal in dark mode) with a thin outline; the one
   being typed in has the accent, thicker below. */
entry, spinbutton, textview.boxed, .box-frame {{
    background: {field}; color: {text}; border: 1px solid {line}; border-radius: 0; box-shadow: none;
}}
entry:focus-within, spinbutton:focus-within {{ border-color: {accent}; box-shadow: inset 0 -1px {accent}; outline: none; }}
entry:disabled, spinbutton:disabled, entry:disabled text {{ background: {page}; color: {grey}; }}
textview text, textview {{ background: {field}; color: {text}; }}
entry placeholder, text placeholder {{ color: {grey}; }}
selection {{ background: {chosen}; color: {chosen_text}; }}

/* Lists. */
list, listview, .list-box {{ background: {list}; color: {text}; }}
list > row, listview > row {{ background: transparent; border-radius: 0; }}
list > row:hover, listview > row:hover {{ background: {button_hot}; }}
list > row:selected, listview > row:selected {{ background: {chosen}; color: {chosen_text}; }}
.list-frame {{ border: 1px solid {line}; background: {list}; }}

/* Drop-down lists, like plain buttons with a small arrow. */
dropdown > button {{ background: {button}; padding: 5px 10px; }}
popover > contents, popover.menu > contents {{ background: {list}; color: {text}; border: 1px solid {line}; border-radius: 0; padding: 0; }}
popover listview > row, popover modelbutton {{ padding: 6px 12px; }}
popover listview > row:selected, popover listview > row:hover {{ background: {chosen}; color: {chosen_text}; }}

/* Switches and check boxes in the accent. */
switch {{ background: {page}; border: 1px solid {grey}; border-radius: 12px; min-height: 20px; }}
switch:checked {{ background: {accent}; border-color: {accent}; }}
switch slider {{ background: {grey}; border: none; box-shadow: none; min-width: 14px; min-height: 14px; margin: 2px; }}
switch:checked slider {{ background: {on_accent}; }}
check, radio {{ background: {field}; border: 1px solid {grey}; box-shadow: none; color: {on_accent}; }}
check:checked, radio:checked {{ background: {accent}; border-color: {accent}; color: {on_accent}; }}

/* Tabs: an icon and a word, the chosen one bold with a line under both. */
notebook, notebook > stack {{ background: {page}; border: none; box-shadow: none; }}
scrolledwindow.box-frame {{ border: 1px solid {line}; background: {field}; }}
scrolledwindow.box-frame:focus-within {{ border-color: {accent}; box-shadow: inset 0 -1px {accent}; }}
frame, .frame {{ border-color: {line}; }}
windowcontrols button, headerbar button.titlebutton {{ background: transparent; border: none; box-shadow: none; padding: 4px; min-height: 24px; min-width: 24px; }}
windowcontrols button:hover {{ background: {button_hot}; }}
notebook > header {{ background: {page}; border-bottom: 1px solid {line}; box-shadow: none; }}
notebook > header > tabs > tab {{ background: transparent; color: {grey}; border: none; box-shadow: none; padding: 6px 12px 7px; margin: 0 6px; min-height: 0; }}
notebook > header > tabs > tab:hover {{ color: {text}; box-shadow: none; }}
notebook > header > tabs > tab:checked {{ color: {accent}; font-weight: 600; box-shadow: inset 0 -3px {accent}; }}
notebook > stack {{ background: {page}; }}

/* The block list: two lines per block, gold labels, a stripe on the
   chosen one. */
.blocks {{ background: {list}; border: 1px solid {line}; }}
.blocks > row {{ padding: 0; border-left: 4px solid transparent; }}
.blocks > row:selected {{ background: {chosen}; border-left-color: {stripe}; }}
.blocks > row:selected label.name {{ color: {chosen_text}; }}
label.name {{ font-weight: 600; }}
.badge {{ background: {badge}; color: {badge_text}; border: 1px solid {badge_line}; border-radius: 4px; padding: 0 7px; }}
.badge image {{ -gtk-icon-size: 14px; }}

/* The status line at the bottom. */
.statusline {{ border-top: 1px solid {line}; color: {grey}; padding: 4px 10px; }}
.savebar {{ padding: 6px 0; }}

/* Messages: a round icon in the accent, amber or deep red. */
.sign {{ border-radius: 999px; min-width: 32px; min-height: 32px; font-weight: bold; font-size: 15pt; }}
.sign.info {{ background: {accent}; color: {on_accent}; }}
.sign.warning {{ background: {warn}; color: {on_sign}; }}
.sign.error {{ background: {error}; color: {on_sign}; }}

/* Statistics. */
.stat-card {{ background-color: {list}; border: 1px solid {line}; border-radius: 10px; padding: 12px 14px; }}
.stat-value {{ font-size: 20pt; font-weight: bold; }}
.stat-bar {{ background-color: #1D9E75; border-radius: 3px; }}
.stat-bar-empty {{ background-color: {line}; border-radius: 1px; }}
.stat-bar-allowance {{ background-color: #BA7517; border-radius: 3px; }}
.stat-today {{ font-weight: bold; }}

scrollbar {{ background: transparent; }}
scrollbar slider {{ background: {line}; }}
"#,
        page = p.page,
        list = p.list,
        field = p.field,
        text = p.text,
        grey = p.grey,
        accent = p.accent,
        on_accent = p.on_accent,
        accent_hot = p.accent_hot,
        accent_down = p.accent_down,
        line = p.line,
        chosen = p.chosen,
        chosen_text = p.chosen_text,
        button = p.button,
        button_hot = p.button_hot,
        button_down = p.button_down,
        stripe = p.stripe,
        badge = p.badge,
        badge_text = p.badge_text,
        badge_line = p.badge_line,
        warn = p.warn,
        error = p.error,
        on_sign = p.on_sign,
    );
    // Without a compositor (e.g. Xfce with compositing off) the see-through
    // shadows and round corners of menus and tooltips show as black.
    if !composited {
        css.push_str(
            "popover > contents, popover.menu > contents, tooltip, tooltip.background { box-shadow: none; border-radius: 0; margin: 0; }",
        );
    }
    css
}
