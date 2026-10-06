#![deny(clippy::unwrap_used)]

use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::fs;
use std::path::MAIN_SEPARATOR;
use std::path::Path;
use std::sync::OnceLock;

use itertools::Itertools;

use schemars::JsonSchema;

use serde::Deserialize;

use toml::Table;
use toml::Value;

use crate::ValueOrPanic;

use super::doc_toml::DocItem;
use super::doc_toml::DocSection;
use super::doc_toml::render;
use super::paths::themes_dir;
use super::schema::HEX_COLOR_PATTERN;
use super::schema::hex_color_map_schema;

pub const DEFAULT_THEME_NAME: &str = "gruber-darker";
pub const DEFAULT_THEME_SOURCE: &str = include_str!("themes/gruber-darker.toml");

pub struct BundledTheme {
    pub name: &'static str,
    pub source: &'static str,
}

pub const BUNDLED_THEMES: &[BundledTheme] = &[
    BundledTheme {
        name: DEFAULT_THEME_NAME,
        source: DEFAULT_THEME_SOURCE,
    },
    BundledTheme {
        name: "monokai",
        source: include_str!("themes/monokai.toml"),
    },
    BundledTheme {
        name: "gruvbox",
        source: include_str!("themes/gruvbox.toml"),
    },
];

static ACTIVE_THEME: OnceLock<Theme> = OnceLock::new();

pub const THEME_FILE_DOC: &[&str] = &[
    "Colors are hex strings in the form \"#rrggbb\". Every key in [ui] and [log]",
    "except `tokens` takes either a hex color or the name of a [palette] entry.",
    "All [ui] and [log] keys are required; unknown keys are rejected.",
];

pub const PALETTE_DOC: &[&str] = &[
    "Named colors that the [ui] and [log] tables can refer to by name.",
    "Names are free-form (letters, digits, '-' and '_'), values are hex colors in",
    "the form \"#rrggbb\". The palette is optional; [ui] and [log] keys may also use",
    "hex colors directly.",
    "example: bg0 = \"#181818\"",
];

pub const UI_DOC: &[&str] = &[
    "Colors of the interactive TUI chrome. Plain output does not use this table.",
    "Each key takes a [palette] name or a hex color \"#rrggbb\".",
];

pub const LOG_DOC: &[&str] = &[
    "Colors of the log lines, used both in the TUI log view and in plain output",
    "(and in copied or exported text). Each key except `tokens` takes a [palette]",
    "name or a hex color \"#rrggbb\". Log lines use these exact 24-bit colors when",
    "the environment sets COLORTERM to \"truecolor\" or \"24bit\", and the closest",
    "basic ANSI colors otherwise.",
];

pub const BACKGROUND_DOC: &[&str] = &[
    "Background of the whole TUI: log view, status bar, panels, menus, dialogs and",
    "the file browser. Also the text color on accent-colored badges such as the",
    "running status indicator and the selected device.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"bg0\" (#181818)",
    "example: background = \"#1d2021\"",
];

pub const TEXT_DOC: &[&str] = &[
    "Main foreground text: menu and dialog content, help descriptions, device",
    "names, key hint descriptions, file browser entries, and selected log line",
    "text that has no color of its own.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"fg0\" (#e4e4ef)",
    "example: text = \"#ebdbb2\"",
];

pub const SUBTEXT_DOC: &[&str] = &[
    "Dimmed text: status bar separators and entry counters, device states, input",
    "placeholders, hints, unavailable commands in the help menu and file browser",
    "metadata.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"fg3\" (#a89984)",
    "example: subtext = \"#928374\"",
];

pub const ACCENT_DOC: &[&str] = &[
    "Primary accent: panel borders, section headings, key names in the border",
    "hints, the running status badge, the log source in the status bar, the",
    "selected help and menu row, the selected device, and the file browser title,",
    "directories and highlights.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"yellow\" (#ffdd33)",
    "example: accent = \"#fabd2f\"",
];

pub const SECONDARY_DOC: &[&str] = &[
    "Secondary accent: the device serial in the status bar.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"niagara\" (#96a6c8)",
    "example: secondary = \"#83a598\"",
];

pub const SUCCESS_DOC: &[&str] = &[
    "Success feedback: status bar messages such as copy confirmations, and success",
    "messages in the file browser.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"green\" (#73c936)",
    "example: success = \"#b8bb26\"",
];

pub const WARNING_DOC: &[&str] = &[
    "Idle status indicator in the status bar, shown while paused, waiting for a",
    "device or not reading live logs.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"brown\" (#cc8c3c)",
    "example: warning = \"#fe8019\"",
];

pub const ERROR_DOC: &[&str] = &[
    "Error messages in the device picker and the file dialogs.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"red\" (#f43841)",
    "example: error = \"#fb4934\"",
];

pub const MATCH_DOC: &[&str] = &[
    "File browser entries matching the current search.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"aqua\" (#8ec07c)",
    "example: match = \"#8ec07c\"",
];

pub const KEYS_DOC: &[&str] = &[
    "Key bindings listed in the help, copy and export menus.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"cyan\" (#8cd0d3)",
    "example: keys = \"#83a598\"",
];

pub const SELECTION_DOC: &[&str] = &[
    "Background of the selected log line in select mode, the selected help and",
    "menu row, and the selected file browser entry.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"bg5\" (#404040)",
    "example: selection = \"#504945\"",
];

pub const LEVEL_FG_DOC: &[&str] = &[
    "Text color of the log level badges (V, D, I, W, E, F) and of the connectors",
    "(╠═ and ╚═) in front of wrapped message lines. The badge background is the",
    "color of the message's level below.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"black\" (#000000)",
    "example: level-fg = \"#1d2021\"",
];

pub const VERBOSE_DOC: &[&str] = &[
    "Level badge and wrap connector background of verbose (V) messages.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"cyan\" (#8cd0d3)",
    "example: verbose = \"#83a598\"",
];

pub const DEBUG_DOC: &[&str] = &[
    "Level badge and wrap connector background of debug (D) messages.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"niagara\" (#96a6c8)",
    "example: debug = \"#458588\"",
];

pub const INFO_DOC: &[&str] = &[
    "Level badge and wrap connector background of info (I) messages.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"green\" (#73c936)",
    "example: info = \"#b8bb26\"",
];

pub const WARN_DOC: &[&str] = &[
    "Level badge and wrap connector background of warning (W) messages.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"yellow\" (#ffdd33)",
    "example: warn = \"#fabd2f\"",
];

pub const LOG_ERROR_DOC: &[&str] = &[
    "Level badge and wrap connector background of error (E) messages.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"brown\" (#cc8c3c)",
    "example: error = \"#fe8019\"",
];

pub const FATAL_DOC: &[&str] = &[
    "Level badge and wrap connector background of fatal (F) messages.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"red\" (#f43841)",
    "example: fatal = \"#fb4934\"",
];

pub const HIGHLIGHT_DOC: &[&str] = &[
    "Highlighted values inside the process banners: the package, target, PID, UID",
    "and GIDs of started processes and the name and PID of ended processes.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"yellow\" (#ffdd33)",
    "example: highlight = \"#fabd2f\"",
];

pub const PROCESS_START_DOC: &[&str] = &[
    "Background of the banner shown when a process starts.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"green\" (#73c936)",
    "example: process-start = \"#98971a\"",
];

pub const PROCESS_DEATH_DOC: &[&str] = &[
    "Background of the banner shown when a process ends.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"red\" (#f43841)",
    "example: process-death = \"#cc241d\"",
];

pub const GC_DURATION_DOC: &[&str] = &[
    "The \"; ~duration=\" label of StrictMode policy violation messages.",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"red\" (#f43841)",
    "example: gc-duration = \"#fb4934\"",
];

pub const GC_FREE_DOC: &[&str] = &[
    "The \"freed <size>\" part of garbage collector messages. Only used when",
    "gc-color is enabled (gc-color = true in the config file, or -g).",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"green\" (#73c936)",
    "example: gc-free = \"#b8bb26\"",
];

pub const GC_UNIT_DOC: &[&str] = &[
    "The duration value of StrictMode policy violation messages, and the",
    "\"paused <time>\" part of garbage collector messages (the latter only when",
    "gc-color is enabled).",
    "value: a [palette] name or a hex color \"#rrggbb\"",
    "default (gruber-darker): \"yellow\" (#ffdd33)",
    "example: gc-unit = \"#fabd2f\"",
];

pub const TOKENS_DOC: &[&str] = &[
    "Colors rotated through for the PID, UID, package and tag columns. A newly seen",
    "value gets the least recently used color and keeps it, so the same tag or",
    "package is always shown in the same color.",
    "type: non-empty array of hex colors \"#rrggbb\" (palette names are not",
    "accepted); there is no upper limit on the number of colors",
    "default (gruber-darker): [\"#f43841\", \"#96a6c8\", \"#8cd0d3\", \"#73c936\",",
    "\"#ffdd33\", \"#9e95c7\", \"#8ec07c\", \"#cc8c3c\"]",
    "example: tokens = [\"#fb4934\", \"#83a598\", \"#b8bb26\", \"#fabd2f\"]",
];

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(title = concat!(env!("CARGO_PKG_NAME"), " color theme"))]
#[schemars(description = THEME_FILE_DOC.join("\n"))]
pub struct ThemeFile {
    #[serde(default)]
    #[schemars(schema_with = "hex_color_map_schema")]
    #[schemars(description = PALETTE_DOC.join("\n"))]
    pub palette: Table,
    #[schemars(description = UI_DOC.join("\n"))]
    pub ui: UiSection,
    #[schemars(description = LOG_DOC.join("\n"))]
    pub log: LogSection,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct UiSection {
    #[schemars(description = BACKGROUND_DOC.join("\n"))]
    pub background: String,
    #[schemars(description = TEXT_DOC.join("\n"))]
    pub text: String,
    #[schemars(description = SUBTEXT_DOC.join("\n"))]
    pub subtext: String,
    #[schemars(description = ACCENT_DOC.join("\n"))]
    pub accent: String,
    #[schemars(description = SECONDARY_DOC.join("\n"))]
    pub secondary: String,
    #[schemars(description = SUCCESS_DOC.join("\n"))]
    pub success: String,
    #[schemars(description = WARNING_DOC.join("\n"))]
    pub warning: String,
    #[schemars(description = ERROR_DOC.join("\n"))]
    pub error: String,
    #[schemars(description = MATCH_DOC.join("\n"))]
    pub r#match: String,
    #[schemars(description = KEYS_DOC.join("\n"))]
    pub keys: String,
    #[schemars(description = SELECTION_DOC.join("\n"))]
    pub selection: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LogSection {
    #[schemars(description = LEVEL_FG_DOC.join("\n"))]
    pub level_fg: String,
    #[schemars(description = VERBOSE_DOC.join("\n"))]
    pub verbose: String,
    #[schemars(description = DEBUG_DOC.join("\n"))]
    pub debug: String,
    #[schemars(description = INFO_DOC.join("\n"))]
    pub info: String,
    #[schemars(description = WARN_DOC.join("\n"))]
    pub warn: String,
    #[schemars(description = LOG_ERROR_DOC.join("\n"))]
    pub error: String,
    #[schemars(description = FATAL_DOC.join("\n"))]
    pub fatal: String,
    #[schemars(description = HIGHLIGHT_DOC.join("\n"))]
    pub highlight: String,
    #[schemars(description = PROCESS_START_DOC.join("\n"))]
    pub process_start: String,
    #[schemars(description = PROCESS_DEATH_DOC.join("\n"))]
    pub process_death: String,
    #[schemars(description = GC_DURATION_DOC.join("\n"))]
    pub gc_duration: String,
    #[schemars(description = GC_FREE_DOC.join("\n"))]
    pub gc_free: String,
    #[schemars(description = GC_UNIT_DOC.join("\n"))]
    pub gc_unit: String,
    #[schemars(length(min = 1), inner(pattern(HEX_COLOR_PATTERN)))]
    #[schemars(description = TOKENS_DOC.join("\n"))]
    pub tokens: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Debug)]
pub struct UiColors {
    pub background: Rgb,
    pub text: Rgb,
    pub subtext: Rgb,
    pub accent: Rgb,
    pub secondary: Rgb,
    pub success: Rgb,
    pub warning: Rgb,
    pub error: Rgb,
    pub r#match: Rgb,
    pub keys: Rgb,
    pub selection: Rgb,
}

#[derive(Clone, Debug)]
pub struct LogColors {
    pub level_fg: Rgb,
    pub verbose: Rgb,
    pub debug: Rgb,
    pub info: Rgb,
    pub warn: Rgb,
    pub error: Rgb,
    pub fatal: Rgb,
    pub highlight: Rgb,
    pub process_start: Rgb,
    pub process_death: Rgb,
    pub gc_duration: Rgb,
    pub gc_free: Rgb,
    pub gc_unit: Rgb,
    pub tokens: Vec<Rgb>,
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub ui: UiColors,
    pub log: LogColors,
}

impl Rgb {
    pub fn from_hex(hex: &str) -> Option<Self> {
        let digits = hex.strip_prefix('#')?;
        if digits.len() != 6usize || !digits.chars().all(|char| char.is_ascii_hexdigit()) {
            return None;
        }

        let channel = |index: usize| u8::from_str_radix(&digits[index..index + 2usize], 16).ok();

        Some(Self {
            r: channel(0usize)?,
            g: channel(2usize)?,
            b: channel(4usize)?,
        })
    }
}

impl Display for Rgb {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        write!(formatter, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl From<Rgb> for ratatui::style::Color {
    fn from(rgb: Rgb) -> Self {
        Self::Rgb(rgb.r, rgb.g, rgb.b)
    }
}

impl From<Rgb> for colored::Color {
    fn from(rgb: Rgb) -> Self {
        Self::TrueColor {
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        }
    }
}

fn hex_color(key: &str, value: &str) -> Result<Rgb, String> {
    Rgb::from_hex(value)
        .ok_or_else(|| format!("{key}: invalid color '{value}', expected a hex color \"#rrggbb\""))
}

pub fn parse_theme(source: &str) -> Result<ThemeFile, String> {
    toml::from_str(source).map_err(|err| err.to_string().trim_end().to_string())
}

/// Falls back to the embedded default theme when no theme has been activated.
pub fn active() -> &'static Theme {
    ACTIVE_THEME.get_or_init(|| {
        parse_theme(DEFAULT_THEME_SOURCE)
            .and_then(|theme_file| theme_file.resolve())
            .unwrap_or_panic("the embedded default theme is invalid")
    })
}

/// Has no effect once a theme is active, so it must run before anything reads [`active`].
pub fn set_active(theme: Theme) {
    let _ = ACTIVE_THEME.set(theme);
}

/// Writes each bundled theme that is missing from the themes directory or whose
/// file is an outdated rendering of it; failures are ignored so a read-only home
/// still works.
pub fn install_bundled_themes() {
    let Some(dir) = themes_dir() else {
        return;
    };

    if fs::create_dir_all(&dir).is_err() {
        return;
    }

    for bundled in BUNDLED_THEMES {
        let path = dir.join(format!("{name}.toml", name = bundled.name));
        let Ok(theme_file) = parse_theme(bundled.source) else {
            continue;
        };

        let credits = bundled
            .source
            .lines()
            .take_while(|line| line.starts_with('#'))
            .map(|line| format!("{line}\n"))
            .collect::<String>();
        let contents = format!("{credits}#\n{}", theme_file.to_doc_toml());

        let outdated = match path.exists() {
            false => true,
            true => fs::read_to_string(&path).is_ok_and(|existing| {
                existing != contents
                    && parse_theme(&existing).is_ok_and(|existing| existing == theme_file)
            }),
        };

        if outdated {
            let _ = fs::write(&path, contents);
        }
    }
}

pub fn available_themes() -> Vec<String> {
    let installed = themes_dir()
        .and_then(|dir| fs::read_dir(dir).ok())
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .filter_map(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        });

    BUNDLED_THEMES
        .iter()
        .map(|bundled| bundled.name.to_string())
        .chain(installed)
        .sorted()
        .dedup()
        .collect()
}

pub fn bundled_theme_names() -> String {
    BUNDLED_THEMES.iter().map(|bundled| bundled.name).join(", ")
}

fn is_theme_path(spec: &str) -> bool {
    spec.contains('/') || spec.contains(MAIN_SEPARATOR) || spec.ends_with(".toml")
}

fn read_theme_file(path: &Path) -> Result<(ThemeFile, Theme), String> {
    let path_display = path.display();
    let source = fs::read_to_string(path)
        .map_err(|err| format!("failed to read theme file '{path_display}': {err}"))?;
    let theme_file = parse_theme(&source)
        .map_err(|err| format!("invalid theme file '{path_display}':\n{err}"))?;
    let theme = theme_file
        .resolve()
        .map_err(|err| format!("invalid theme file '{path_display}': {err}"))?;

    Ok((theme_file, theme))
}

/// `spec` is a path when it contains a path separator or ends in `.toml`, otherwise a theme
/// name looked up in the themes directory with a fallback to the bundled copy.
pub fn load_theme(spec: &str) -> Result<(ThemeFile, Theme), String> {
    let available = || available_themes().join(", ");

    if is_theme_path(spec) {
        let path = Path::new(spec);
        return match path.is_file() {
            true => read_theme_file(path),
            false => Err(format!(
                "theme file '{spec}' not found; available themes: {}",
                available()
            )),
        };
    }

    if let Some(path) = themes_dir().map(|dir| dir.join(format!("{spec}.toml")))
        && path.is_file()
    {
        return read_theme_file(&path);
    }

    match BUNDLED_THEMES.iter().find(|bundled| bundled.name == spec) {
        Some(bundled) => {
            let theme_file = parse_theme(bundled.source)
                .map_err(|err| format!("invalid bundled theme '{spec}':\n{err}"))?;
            let theme = theme_file
                .resolve()
                .map_err(|err| format!("invalid bundled theme '{spec}': {err}"))?;

            Ok((theme_file, theme))
        }
        None => Err(format!(
            "unknown theme '{spec}'; available themes: {}",
            available()
        )),
    }
}

impl ThemeFile {
    fn palette_color(&self, key: &str, value: &str) -> Result<Rgb, String> {
        if value.starts_with('#') {
            return hex_color(key, value);
        }

        match self.palette.get(value) {
            Some(Value::String(hex)) => hex_color(&format!("palette.{value}"), hex),
            Some(_) => Err(format!(
                "palette.{value}: expected a hex color string \"#rrggbb\""
            )),
            None => Err(format!(
                "{key}: '{value}' is neither a hex color \"#rrggbb\" nor a [palette] name"
            )),
        }
    }

    pub fn resolve(&self) -> Result<Theme, String> {
        for (name, value) in &self.palette {
            match value {
                Value::String(hex) => hex_color(&format!("palette.{name}"), hex).map(|_| ())?,
                _ => {
                    return Err(format!(
                        "palette.{name}: expected a hex color string \"#rrggbb\""
                    ));
                }
            }
        }

        let UiSection {
            background,
            text,
            subtext,
            accent,
            secondary,
            success,
            warning,
            error,
            r#match,
            keys,
            selection,
        } = &self.ui;

        let LogSection {
            level_fg,
            verbose,
            debug,
            info,
            warn,
            error: log_error,
            fatal,
            highlight,
            process_start,
            process_death,
            gc_duration,
            gc_free,
            gc_unit,
            tokens,
        } = &self.log;

        if tokens.is_empty() {
            return Err("log.tokens: at least one color is required".to_string());
        }

        let tokens = tokens
            .iter()
            .enumerate()
            .map(|(index, token)| hex_color(&format!("log.tokens[{index}]"), token))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Theme {
            ui: UiColors {
                background: self.palette_color("ui.background", background)?,
                text: self.palette_color("ui.text", text)?,
                subtext: self.palette_color("ui.subtext", subtext)?,
                accent: self.palette_color("ui.accent", accent)?,
                secondary: self.palette_color("ui.secondary", secondary)?,
                success: self.palette_color("ui.success", success)?,
                warning: self.palette_color("ui.warning", warning)?,
                error: self.palette_color("ui.error", error)?,
                r#match: self.palette_color("ui.match", r#match)?,
                keys: self.palette_color("ui.keys", keys)?,
                selection: self.palette_color("ui.selection", selection)?,
            },
            log: LogColors {
                level_fg: self.palette_color("log.level-fg", level_fg)?,
                verbose: self.palette_color("log.verbose", verbose)?,
                debug: self.palette_color("log.debug", debug)?,
                info: self.palette_color("log.info", info)?,
                warn: self.palette_color("log.warn", warn)?,
                error: self.palette_color("log.error", log_error)?,
                fatal: self.palette_color("log.fatal", fatal)?,
                highlight: self.palette_color("log.highlight", highlight)?,
                process_start: self.palette_color("log.process-start", process_start)?,
                process_death: self.palette_color("log.process-death", process_death)?,
                gc_duration: self.palette_color("log.gc-duration", gc_duration)?,
                gc_free: self.palette_color("log.gc-free", gc_free)?,
                gc_unit: self.palette_color("log.gc-unit", gc_unit)?,
                tokens,
            },
        })
    }

    pub fn doc_items(&self) -> Vec<DocSection> {
        let Self { palette, ui, log } = self.clone();

        let UiSection {
            background,
            text,
            subtext,
            accent,
            secondary,
            success,
            warning,
            error,
            r#match,
            keys,
            selection,
        } = ui;

        let LogSection {
            level_fg,
            verbose,
            debug,
            info,
            warn,
            error: log_error,
            fatal,
            highlight,
            process_start,
            process_death,
            gc_duration,
            gc_free,
            gc_unit,
            tokens,
        } = log;

        vec![
            DocSection {
                title: "Palette",
                table: Some("palette"),
                doc: PALETTE_DOC,
                items: palette
                    .into_iter()
                    .map(|(name, value)| DocItem::set(&name, &[], value))
                    .collect(),
            },
            DocSection {
                title: "User interface",
                table: Some("ui"),
                doc: UI_DOC,
                items: vec![
                    DocItem::set("background", BACKGROUND_DOC, background),
                    DocItem::set("text", TEXT_DOC, text),
                    DocItem::set("subtext", SUBTEXT_DOC, subtext),
                    DocItem::set("accent", ACCENT_DOC, accent),
                    DocItem::set("secondary", SECONDARY_DOC, secondary),
                    DocItem::set("success", SUCCESS_DOC, success),
                    DocItem::set("warning", WARNING_DOC, warning),
                    DocItem::set("error", ERROR_DOC, error),
                    DocItem::set("match", MATCH_DOC, r#match),
                    DocItem::set("keys", KEYS_DOC, keys),
                    DocItem::set("selection", SELECTION_DOC, selection),
                ],
            },
            DocSection {
                title: "Log colors",
                table: Some("log"),
                doc: LOG_DOC,
                items: vec![
                    DocItem::set("level-fg", LEVEL_FG_DOC, level_fg),
                    DocItem::set("verbose", VERBOSE_DOC, verbose),
                    DocItem::set("debug", DEBUG_DOC, debug),
                    DocItem::set("info", INFO_DOC, info),
                    DocItem::set("warn", WARN_DOC, warn),
                    DocItem::set("error", LOG_ERROR_DOC, log_error),
                    DocItem::set("fatal", FATAL_DOC, fatal),
                    DocItem::set("highlight", HIGHLIGHT_DOC, highlight),
                    DocItem::set("process-start", PROCESS_START_DOC, process_start),
                    DocItem::set("process-death", PROCESS_DEATH_DOC, process_death),
                    DocItem::set("gc-duration", GC_DURATION_DOC, gc_duration),
                    DocItem::set("gc-free", GC_FREE_DOC, gc_free),
                    DocItem::set("gc-unit", GC_UNIT_DOC, gc_unit),
                    DocItem::set("tokens", TOKENS_DOC, tokens),
                ],
            },
        ]
    }

    pub fn to_doc_toml(&self) -> String {
        let pkg = env!("CARGO_PKG_NAME");
        let dir = themes_dir()
            .map(|dir| dir.display().to_string())
            .unwrap_or_default();

        let header = [
            format!("{pkg} color theme"),
            String::default(),
            format!("Themes directory: {dir}"),
            format!("Select a theme with: {pkg} --theme <NAME|PATH>"),
            "or with `theme = \"<NAME|PATH>\"` in the config file. NAME is a file name in"
                .to_string(),
            "the themes directory without the .toml extension; PATH is a theme file path."
                .to_string(),
            format!("Print the active theme with: {pkg} --print-theme"),
            format!(
                "Export a theme for editing with: {pkg} --theme <NAME> --print-theme > my-theme.toml"
            ),
            String::default(),
            format!(
                "The bundled themes ({}) are written to the",
                bundled_theme_names()
            ),
            "themes directory when missing and their comments are refreshed while their".to_string(),
            "colors are unchanged. Edited themes are never overwritten; delete a file to".to_string(),
            "restore its bundled version.".to_string(),
            String::default(),
        ]
        .into_iter()
        .chain(THEME_FILE_DOC.iter().map(|line| line.to_string()))
        .collect::<Vec<_>>();

        render(&header, &self.doc_items())
    }
}
