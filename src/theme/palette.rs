//! The named colours of a theme, and the built-in themes.
//!
//! Every colour the app draws with is one field of [`Palette`]. The fields are
//! declared once, in the `palette!` list below, with a group and a plain-words
//! label: that single list drives the JSON file format, the colour editor in
//! Settings and the accessors (`theme::TEXT()` and friends) the widgets call.

use std::collections::BTreeMap;
use std::sync::RwLock;

use eframe::egui::Color32;

macro_rules! palette {
    ($( $group:literal => { $( $field:ident / $acc:ident : $label:literal, $hint:literal ),* $(,)? } ),* $(,)?) => {
        /// Every colour a theme sets.
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct Palette { $($( pub $field: Color32, )*)* }

        /// One colour of the palette as the editor describes it.
        pub struct ColourInfo { pub key: &'static str, pub label: &'static str, pub hint: &'static str }

        impl Palette {
            /// Groups of colours, in the order the editor shows them.
            pub const GROUPS: &'static [(&'static str, &'static [ColourInfo])] = &[
                $( ($group, &[ $( ColourInfo { key: stringify!($field), label: $label, hint: $hint } ),* ]) ),*
            ];

            pub fn get(&self, key: &str) -> Option<Color32> {
                $($( if key == stringify!($field) { return Some(self.$field); } )*)*
                None
            }

            /// Set one colour by key; false when the key is not a colour of the palette.
            pub fn set(&mut self, key: &str, color: Color32) -> bool {
                $($( if key == stringify!($field) { self.$field = color; return true; } )*)*
                false
            }
        }

        $($(
            /// The current theme's colour of this name.
            #[allow(non_snake_case)]
            #[inline]
            pub fn $acc() -> Color32 { current().$field }
        )*)*
    };
}

// A deliberately small set. Each colour has one job, and the same few hues
// are reused everywhere, so the app reads as calm and consistent.
palette! {
    "Surfaces" => {
        bg / BG: "Window", "Behind everything: the window, the side rail and the area around the page.",
        panel / PANEL: "Panels", "Side panels, dialogs, menus and cards.",
        raised / RAISED: "Fields and buttons", "Text boxes, buttons, tabs and the toolbar background.",
        hover / HOVER: "Hover", "A button or row while the pointer is over it.",
        border / BORDER: "Lines", "Borders around controls and the dividers between sections.",
    },
    "Text" => {
        text / TEXT: "Text", "Titles, names and anything you read first.",
        muted / MUTED: "Quiet text", "Labels, hints, captions, word counts and icons.",
    },
    "Accent" => {
        accent / ACCENT: "Accent", "The selected item, progress bars, focus rings and main buttons.",
        accent_soft / ACCENT_SOFT: "Accent tint", "Soft background behind the selected item, selected text and pills.",
        folder / FOLDER: "Folders", "Folder icons in the binder.",
        on_accent / ON_ACCENT: "Text on accent", "Lettering on accent-coloured buttons and tabs. Needs to contrast with the accent.",
    },
    "Status" => {
        success / SUCCESS: "Success", "Done, saved and revised states, and the 'latest' tags.",
        info / INFO: "Information", "Drafts, links in your text and general information.",
        danger / DANGER: "Danger", "Errors, delete buttons and spelling underlines.",
        special / SPECIAL: "Special", "Scripts, characters and other special items.",
        warning / WARNING: "Warning", "Cautions and things to double-check.",
    },
    "Writing page" => {
        paper / PAPER: "Page", "The page you write on.",
        paper_text / PAPER_TEXT: "Page text", "Normal writing on the page.",
    },
    "Writing highlights" => {
        heading / HEADING: "Chapter headings", "Chapter and other heading paragraphs. Set it apart from the page text to make them stand out.",
        scene_heading / SCENE_HEADING: "Scene headings", "INT./EXT. scene lines in a script.",
        character / CHARACTER: "Character names", "The character cue above dialogue in a script.",
        find_match / FIND_MATCH: "Find matches", "Every match while searching. Usually see-through.",
        find_current / FIND_CURRENT: "Current match", "The match you are on. Usually see-through.",
    },
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// A colour with transparency; `a` is 0-255 and the colour is not premultiplied.
const fn hx(rgb24: u32) -> Color32 {
    Color32::from_rgb((rgb24 >> 16) as u8, (rgb24 >> 8) as u8, rgb24 as u8)
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

impl Palette {
    /// The original look: quiet warm grays and a soft-white page.
    pub const LIGHT: Palette = Palette {
        bg: rgb(244, 245, 246),
        panel: rgb(248, 248, 249),
        raised: rgb(240, 241, 243),
        hover: rgb(230, 232, 235),
        border: rgb(220, 223, 226),
        text: rgb(40, 43, 48),
        // Clears a 4.5:1 contrast ratio on `panel` at small sizes.
        muted: rgb(92, 99, 108),
        accent: rgb(54, 104, 98),
        accent_soft: rgb(214, 232, 229),
        on_accent: rgb(255, 255, 255),
        success: rgb(56, 104, 72),
        info: rgb(52, 88, 134),
        danger: rgb(150, 62, 56),
        special: rgb(96, 78, 128),
        warning: rgb(150, 110, 40),
        folder: rgb(184, 142, 66),
        paper: rgb(251, 250, 247),
        paper_text: rgb(44, 42, 38),
        heading: rgb(44, 42, 38),
        scene_heading: rgb(44, 42, 38),
        character: rgb(44, 42, 38),
        // Translucent on purpose: they are laid over the text they mark
        // (premultiplied values of 250,214,70 @ 110 and 240,150,40 @ 150).
        find_match: Color32::from_rgba_premultiplied(108, 92, 30, 110),
        find_current: Color32::from_rgba_premultiplied(141, 88, 24, 150),
    };

    pub fn dark() -> Palette {
        Palette {
            bg: rgb(24, 26, 30),
            panel: rgb(33, 36, 41),
            raised: rgb(41, 44, 50),
            hover: rgb(52, 56, 63),
            border: rgb(62, 67, 75),
            text: rgb(226, 229, 234),
            muted: rgb(160, 167, 178),
            accent: rgb(108, 178, 168),
            accent_soft: rgb(42, 70, 68),
            on_accent: rgb(16, 30, 29),
            success: rgb(130, 180, 147),
            info: rgb(125, 165, 215),
            danger: rgb(220, 125, 115),
            special: rgb(165, 143, 205),
            warning: rgb(214, 170, 90),
            folder: rgb(214, 170, 90),
            paper: rgb(40, 42, 47),
            paper_text: rgb(222, 222, 218),
            heading: rgb(222, 222, 218),
            scene_heading: rgb(222, 222, 218),
            character: rgb(222, 222, 218),
            find_match: rgba(250, 214, 70, 70),
            find_current: rgba(240, 150, 40, 120),
        }
    }

    pub fn colourful() -> Palette {
        Palette {
            bg: rgb(255, 236, 214),
            panel: rgb(255, 250, 242),
            raised: rgb(255, 238, 218),
            hover: rgb(252, 222, 190),
            border: rgb(236, 200, 164),
            text: rgb(54, 36, 44),
            muted: rgb(112, 70, 84),
            accent: rgb(190, 44, 92),
            accent_soft: rgb(252, 214, 226),
            on_accent: rgb(255, 255, 255),
            success: rgb(30, 120, 76),
            info: rgb(36, 96, 184),
            danger: rgb(196, 44, 36),
            special: rgb(110, 64, 184),
            warning: rgb(176, 108, 16),
            folder: rgb(176, 108, 16),
            paper: rgb(255, 253, 247),
            paper_text: rgb(40, 32, 36),
            heading: rgb(168, 40, 80),
            scene_heading: rgb(30, 86, 170),
            character: rgb(100, 56, 170),
            find_match: rgba(255, 220, 60, 120),
            find_current: rgba(255, 140, 30, 170),
        }
    }

    pub fn monochrome() -> Palette {
        Palette {
            bg: rgb(238, 238, 238),
            panel: rgb(248, 248, 248),
            raised: rgb(236, 236, 236),
            hover: rgb(222, 222, 222),
            border: rgb(206, 206, 206),
            text: rgb(24, 24, 24),
            muted: rgb(84, 84, 84),
            accent: rgb(40, 40, 40),
            accent_soft: rgb(214, 214, 214),
            on_accent: rgb(255, 255, 255),
            success: rgb(110, 110, 110),
            info: rgb(70, 70, 70),
            danger: rgb(40, 40, 40),
            special: rgb(135, 135, 135),
            warning: rgb(90, 90, 90),
            folder: rgb(120, 120, 120),
            paper: rgb(255, 255, 255),
            paper_text: rgb(20, 20, 20),
            heading: rgb(0, 0, 0),
            scene_heading: rgb(20, 20, 20),
            character: rgb(20, 20, 20),
            find_match: rgba(160, 160, 160, 130),
            find_current: rgba(70, 70, 70, 150),
        }
    }

    // Popular editor colour schemes, translated to this palette's roles.
    fn cobalt() -> Palette {
        Palette {
            bg: hx(0x122738),
            panel: hx(0x173043),
            raised: hx(0x1d4260),
            hover: hx(0x285a7d),
            border: hx(0x2d5878),
            text: hx(0xe6eef5),
            muted: hx(0x9db4c7),
            accent: hx(0xffc600),
            accent_soft: hx(0x0d3a58),
            on_accent: hx(0x122738),
            success: hx(0x3ad900),
            info: hx(0x9effff),
            danger: hx(0xff628c),
            special: hx(0xfb94ff),
            warning: hx(0xff9d00),
            folder: hx(0xff9d00),
            paper: hx(0x193549),
            paper_text: hx(0xf0f4f8),
            heading: hx(0xffc600),
            scene_heading: hx(0x9effff),
            character: hx(0xff9d00),
            find_match: rgba(255, 198, 0, 70),
            find_current: rgba(255, 157, 0, 130),
        }
    }

    fn solarized_light() -> Palette {
        Palette {
            bg: hx(0xeee8d5),
            panel: hx(0xf7f0dc),
            raised: hx(0xf1ebd6),
            hover: hx(0xe6dfca),
            border: hx(0xd6cfb9),
            text: hx(0x073642),
            muted: hx(0x586e75),
            accent: hx(0x1f77b4),
            accent_soft: hx(0xd5e5ee),
            on_accent: hx(0xfdf6e3),
            success: hx(0x738500),
            info: hx(0x268bd2),
            danger: hx(0xdc322f),
            special: hx(0x6c71c4),
            warning: hx(0xb58900),
            folder: hx(0xb58900),
            paper: hx(0xfdf6e3),
            paper_text: hx(0x073642),
            heading: hx(0xcb4b16),
            scene_heading: hx(0x268bd2),
            character: hx(0x6c71c4),
            find_match: rgba(181, 137, 0, 90),
            find_current: rgba(203, 75, 22, 130),
        }
    }

    fn solarized_dark() -> Palette {
        Palette {
            bg: hx(0x00212b),
            panel: hx(0x002b36),
            raised: hx(0x073642),
            hover: hx(0x0d4452),
            border: hx(0x1c4b57),
            text: hx(0xb6c4c4),
            muted: hx(0x6f8a91),
            accent: hx(0x2aa198),
            accent_soft: hx(0x0b4a50),
            on_accent: hx(0x002b36),
            success: hx(0x859900),
            info: hx(0x268bd2),
            danger: hx(0xe5504d),
            special: hx(0x6c71c4),
            warning: hx(0xb58900),
            folder: hx(0xb58900),
            paper: hx(0x01323d),
            paper_text: hx(0xb6c4c4),
            heading: hx(0xe5683a),
            scene_heading: hx(0x268bd2),
            character: hx(0xd33682),
            find_match: rgba(181, 137, 0, 80),
            find_current: rgba(203, 75, 22, 130),
        }
    }

    fn nord() -> Palette {
        Palette {
            bg: hx(0x272c36),
            panel: hx(0x2e3440),
            raised: hx(0x3b4252),
            hover: hx(0x434c5e),
            border: hx(0x4c566a),
            text: hx(0xe5e9f0),
            muted: hx(0x9aa5b8),
            accent: hx(0x88c0d0),
            accent_soft: hx(0x34495a),
            on_accent: hx(0x2e3440),
            success: hx(0xa3be8c),
            info: hx(0x81a1c1),
            danger: hx(0xd06f79),
            special: hx(0xb48ead),
            warning: hx(0xebcb8b),
            folder: hx(0xebcb8b),
            paper: hx(0x323946),
            paper_text: hx(0xe5e9f0),
            heading: hx(0x88c0d0),
            scene_heading: hx(0x81a1c1),
            character: hx(0xebcb8b),
            find_match: rgba(235, 203, 139, 70),
            find_current: rgba(208, 135, 112, 130),
        }
    }

    fn dracula() -> Palette {
        Palette {
            bg: hx(0x21222c),
            panel: hx(0x282a36),
            raised: hx(0x343746),
            hover: hx(0x44475a),
            border: hx(0x4b4f66),
            text: hx(0xf8f8f2),
            muted: hx(0x9aa5ce),
            accent: hx(0xbd93f9),
            accent_soft: hx(0x3d3b5c),
            on_accent: hx(0x282a36),
            success: hx(0x50fa7b),
            info: hx(0x8be9fd),
            danger: hx(0xff5555),
            special: hx(0xff79c6),
            warning: hx(0xffb86c),
            folder: hx(0xffb86c),
            paper: hx(0x2b2d3b),
            paper_text: hx(0xf8f8f2),
            heading: hx(0xbd93f9),
            scene_heading: hx(0x8be9fd),
            character: hx(0xffb86c),
            find_match: rgba(241, 250, 140, 60),
            find_current: rgba(255, 184, 108, 120),
        }
    }

    fn one_dark() -> Palette {
        Palette {
            bg: hx(0x21252b),
            panel: hx(0x282c34),
            raised: hx(0x2c313a),
            hover: hx(0x3a3f4b),
            border: hx(0x3e4451),
            text: hx(0xbcc3cf),
            muted: hx(0x858d9b),
            accent: hx(0x61afef),
            accent_soft: hx(0x2c4259),
            on_accent: hx(0x1d2026),
            success: hx(0x98c379),
            info: hx(0x61afef),
            danger: hx(0xe06c75),
            special: hx(0xc678dd),
            warning: hx(0xe5c07b),
            folder: hx(0xe5c07b),
            paper: hx(0x2b303a),
            paper_text: hx(0xbcc3cf),
            heading: hx(0xe5c07b),
            scene_heading: hx(0x61afef),
            character: hx(0xd19a66),
            find_match: rgba(229, 192, 123, 70),
            find_current: rgba(209, 154, 102, 130),
        }
    }

    fn monokai() -> Palette {
        Palette {
            bg: hx(0x1e1f1c),
            panel: hx(0x272822),
            raised: hx(0x31322c),
            hover: hx(0x3e3d32),
            border: hx(0x49483e),
            text: hx(0xf8f8f2),
            muted: hx(0xa09f93),
            accent: hx(0xa6e22e),
            accent_soft: hx(0x39421f),
            on_accent: hx(0x272822),
            success: hx(0xa6e22e),
            info: hx(0x66d9ef),
            danger: hx(0xf92672),
            special: hx(0xae81ff),
            warning: hx(0xfd971f),
            folder: hx(0xfd971f),
            paper: hx(0x2b2c26),
            paper_text: hx(0xf8f8f2),
            heading: hx(0xa6e22e),
            scene_heading: hx(0x66d9ef),
            character: hx(0xfd971f),
            find_match: rgba(230, 219, 116, 60),
            find_current: rgba(253, 151, 31, 120),
        }
    }

    /// Every colour shifted together: `hue` in degrees (-180..180), `saturation`
    /// and `lightness` from -1.0 (all the way down) to 1.0 (all the way up).
    /// Transparency is kept. All zeros returns the palette unchanged.
    pub fn adjusted(&self, hue: f32, saturation: f32, lightness: f32) -> Palette {
        let mut out = *self;
        if hue == 0.0 && saturation == 0.0 && lightness == 0.0 {
            return out;
        }
        for (_, items) in Self::GROUPS {
            for info in *items {
                if let Some(c) = self.get(info.key) {
                    out.set(info.key, adjust_colour(c, hue, saturation, lightness));
                }
            }
        }
        out
    }

    /// Whether the window background is dark. egui's own controls (scroll
    /// bars, the text cursor, menus) start from dark defaults when it is.
    pub fn is_dark(&self) -> bool {
        let [r, g, b, _] = self.bg.to_array();
        0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b) < 128.0
    }
}

fn to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < 1e-6 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } * 60.0;
    (h, s.clamp(0.0, 1.0), l)
}

fn from_hsl(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h.rem_euclid(360.0) / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (r + m, g + m, b + m)
}

fn adjust_colour(color: Color32, hue: f32, saturation: f32, lightness: f32) -> Color32 {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    let (h, s, l) = to_hsl(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0);
    // A slider at its end reaches full saturation / white / black, and the
    // steps in between are proportional, so nothing ever overshoots.
    let push = |v: f32, amount: f32| if amount >= 0.0 { v + amount * (1.0 - v) } else { v * (1.0 + amount) };
    let (r, g, b) = from_hsl(h + hue, push(s, saturation).clamp(0.0, 1.0), push(l, lightness).clamp(0.0, 1.0));
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(byte(r), byte(g), byte(b), a)
}

static CURRENT: RwLock<Palette> = RwLock::new(Palette::LIGHT);

/// The palette every widget is currently drawn with.
pub fn current() -> Palette {
    *CURRENT.read().unwrap_or_else(|e| e.into_inner())
}

static DARK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Make a theme the one the whole app draws with. The caller re-runs
/// `theme::install` so egui's own controls follow.
pub fn activate(theme: &Theme) {
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = theme.palette;
    DARK.store(theme.palette.is_dark(), std::sync::atomic::Ordering::Relaxed);
}

pub fn current_dark() -> bool {
    DARK.load(std::sync::atomic::Ordering::Relaxed)
}

/// A theme: a name and its colours.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Stable identifier: the built-in's name, or the file name without `.json`.
    pub id: String,
    pub name: String,
    pub palette: Palette,
    pub builtin: bool,
}

pub fn builtin_themes() -> Vec<Theme> {
    let make = |id: &str, name: &str, palette: Palette| Theme {
        id: id.into(),
        name: name.into(),
        palette,
        builtin: true,
    };
    vec![
        make("light", "Light", Palette::LIGHT),
        make("dark", "Dark", Palette::dark()),
        make("colourful", "Colourful", Palette::colourful()),
        make("monochrome", "Monochrome", Palette::monochrome()),
        make("cobalt", "Cobalt", Palette::cobalt()),
        make("solarized-light", "Solarized Light", Palette::solarized_light()),
        make("solarized-dark", "Solarized Dark", Palette::solarized_dark()),
        make("nord", "Nord", Palette::nord()),
        make("dracula", "Dracula", Palette::dracula()),
        make("one-dark", "One Dark", Palette::one_dark()),
        make("monokai", "Monokai", Palette::monokai()),
    ]
}

fn hex(color: Color32) -> String {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

/// `#rrggbb` or `#rrggbbaa` (the `#` is optional).
pub fn parse_hex(text: &str) -> Option<Color32> {
    let digits = text.trim().trim_start_matches('#');
    if !digits.is_ascii() || !(digits.len() == 6 || digits.len() == 8) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    let (r, g, b) = (byte(0)?, byte(2)?, byte(4)?);
    let a = if digits.len() == 8 { byte(6)? } else { 255 };
    Some(Color32::from_rgba_unmultiplied(r, g, b, a))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ThemeFile {
    name: String,
    /// Colours by key. Any key left out takes the Light built-in's value, so
    /// a shared file stays valid when later versions add colours.
    #[serde(default)]
    colors: BTreeMap<String, String>,
}

impl Theme {
    /// Read a shared theme file. Unknown colour keys are ignored; a colour
    /// that is not `#rrggbb` is an error naming the key.
    pub fn from_json(id: &str, json: &str) -> Result<Theme, String> {
        let file: ThemeFile =
            serde_json::from_str(json).map_err(|e| format!("not a theme file: {e}"))?;
        let name = file.name.trim();
        if name.is_empty() {
            return Err("the theme needs a name".into());
        }
        let mut palette = Palette::LIGHT;
        for (key, value) in &file.colors {
            if palette.get(key).is_none() {
                continue;
            }
            let color = parse_hex(value)
                .ok_or_else(|| format!("the colour \"{key}\" is not a hex colour like #a1b2c3 (got \"{value}\")"))?;
            palette.set(key, color);
        }
        Ok(Theme { id: id.into(), name: name.into(), palette, builtin: false })
    }

    pub fn to_json(&self) -> String {
        let mut colors = BTreeMap::new();
        for (_, items) in Palette::GROUPS {
            for info in *items {
                if let Some(color) = self.palette.get(info.key) {
                    colors.insert(info.key.to_string(), hex(color));
                }
            }
        }
        let file = ThemeFile { name: self.name.clone(), colors };
        serde_json::to_string_pretty(&file).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_including_transparency() {
        assert_eq!(parse_hex("#102030"), Some(Color32::from_rgb(16, 32, 48)));
        assert_eq!(hex(Color32::from_rgb(16, 32, 48)), "#102030");
        let c = rgba(250, 214, 70, 70);
        assert_eq!(parse_hex(&hex(c)), Some(c));
        assert_eq!(parse_hex("#12"), None);
        assert_eq!(parse_hex("#gg0000"), None);
        assert_eq!(parse_hex("#ééé"), None);
    }

    #[test]
    fn every_builtin_survives_a_trip_through_json() {
        for theme in builtin_themes() {
            let back = Theme::from_json(&theme.id, &theme.to_json()).unwrap();
            assert_eq!(back.palette, theme.palette, "{}", theme.name);
        }
    }

    #[test]
    fn missing_colours_fall_back_and_unknown_ones_are_ignored() {
        let t = Theme::from_json("x", r##"{"name":"Mine","colors":{"accent":"#ff0000","future_thing":"#000000"}}"##).unwrap();
        assert_eq!(t.palette.accent, Color32::from_rgb(255, 0, 0));
        assert_eq!(t.palette.bg, Palette::LIGHT.bg);
        assert!(Palette::dark().is_dark() && !Palette::LIGHT.is_dark() && !Palette::colourful().is_dark());
    }

    #[test]
    fn bad_files_are_refused_with_a_reason() {
        assert!(Theme::from_json("x", "nope").is_err());
        assert!(Theme::from_json("x", r#"{"name":" "}"#).is_err());
        let err = Theme::from_json("x", r#"{"name":"A","colors":{"text":"red"}}"#).unwrap_err();
        assert!(err.contains("text"), "{err}");
    }

    #[test]
    fn adjusting_by_nothing_changes_nothing_and_hue_turns_colours() {
        let p = Palette::colourful();
        assert_eq!(p.adjusted(0.0, 0.0, 0.0), p);
        // Red turned 120 degrees is green, 240 is blue.
        let red = adjust_colour(Color32::from_rgb(255, 0, 0), 120.0, 0.0, 0.0);
        assert_eq!(red, Color32::from_rgb(0, 255, 0));
        let blue = adjust_colour(Color32::from_rgb(255, 0, 0), -120.0, 0.0, 0.0);
        assert_eq!(blue, Color32::from_rgb(0, 0, 255));
        // Greys have no hue to turn.
        assert_eq!(adjust_colour(Color32::from_rgb(90, 90, 90), 77.0, 0.0, 0.0), Color32::from_rgb(90, 90, 90));
    }

    #[test]
    fn lightness_and_saturation_reach_their_ends_and_keep_transparency() {
        let c = Color32::from_rgb(54, 104, 98);
        assert_eq!(adjust_colour(c, 0.0, 0.0, 1.0), Color32::WHITE);
        assert_eq!(adjust_colour(c, 0.0, 0.0, -1.0), Color32::BLACK);
        let grey = adjust_colour(c, 0.0, -1.0, 0.0);
        let [r, g, b, _] = grey.to_array();
        assert!(r.abs_diff(g) <= 1 && g.abs_diff(b) <= 1);
        let glass = rgba(250, 214, 70, 70);
        assert_eq!(adjust_colour(glass, 40.0, 0.2, 0.1).a(), glass.a());
    }

    fn luminance(c: Color32) -> f32 {
        let lin = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn built_in_themes_are_readable_and_uniquely_named() {
        let themes = builtin_themes();
        let mut ids: Vec<_> = themes.iter().map(|t| t.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), themes.len());
        let mut failures = Vec::new();
        for t in &themes {
            let p = t.palette;
            let checks = [
                ("text on panel", contrast(p.text, p.panel), 7.0),
                ("quiet text on panel", contrast(p.muted, p.panel), 4.0),
                ("quiet text on window", contrast(p.muted, p.bg), 3.8),
                ("page text on page", contrast(p.paper_text, p.paper), 7.0),
                ("text on accent", contrast(p.on_accent, p.accent), 3.5),
                ("success on accent tint", contrast(p.success, p.accent_soft), 3.0),
                ("danger on page", contrast(p.danger, p.paper), 3.0),
                ("heading on page", contrast(p.heading, p.paper), 3.0),
                ("scene heading on page", contrast(p.scene_heading, p.paper), 3.0),
                ("character on page", contrast(p.character, p.paper), 3.0),
            ];
            for (what, ratio, need) in checks {
                if ratio < need {
                    failures.push(format!("{}: {what} is {ratio:.1}:1, needs {need}:1", t.name));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn palette_groups_cover_every_field_once() {
        let mut keys: Vec<&str> = Palette::GROUPS.iter().flat_map(|(_, i)| i.iter().map(|c| c.key)).collect();
        let n = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), n);
        assert!(keys.iter().all(|k| Palette::LIGHT.get(k).is_some()));
    }
}
