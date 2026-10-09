#![deny(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

use clap::ArgMatches;
use clap::ValueEnum;
use clap::parser::ValueSource;

use itertools::Itertools;

use schemars::JsonSchema;

use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error;

use crate::CliArgs;
use crate::LogFormat;
use crate::LogFormatKind;
use crate::LogLevel;

use super::doc_toml::DocItem;
use super::doc_toml::DocSection;
use super::doc_toml::render;
use super::paths::default_config_file;
use super::schema::CONFIG_SCHEMA_URL;
use super::schema::schema_directive_prefix;
use super::schema::theme_name_schema;
use super::schema::value_enum_schema;

pub const PACKAGES_DOC: &[&str] = &[
    "Application package names whose log messages are shown.",
    "A plain name such as \"com.example.app\" matches the package and all of its",
    "processes. A name containing ':' such as \"com.example.app:remote\" matches only",
    "that named process, and a trailing ':' (\"com.example.app:\") matches only the",
    "main process. When the list is empty and `current` is false, messages from all",
    "packages are shown.",
    "type: array of strings",
    "default: []",
    "command line: PACKAGE positional arguments (they replace this list)",
    "example: packages = [\"com.example.app\", \"com.example.app:remote\"]",
];

pub const ADB_DOC: &[&str] = &[
    "Path to the adb executable used to read logs and query devices.",
    "Leave unset to run \"adb\" from the PATH.",
    "type: string (file path)",
    "default: unset (adb from the PATH)",
    "command line: -A, --adb <ADB_PATH>",
    "example: adb = \"/opt/android-sdk/platform-tools/adb\"",
];

pub const DEVICE_DOC: &[&str] = &[
    "Read logs from the first connected physical (non-emulator) device.",
    "type: boolean",
    "default: false",
    "command line: -d, --device",
    "note: when true here, it cannot be switched off from the command line",
    "example: device = true",
];

pub const EMULATOR_DOC: &[&str] = &[
    "Read logs from the first running emulator.",
    "type: boolean",
    "default: false",
    "command line: -e, --emulator",
    "note: when true here, it cannot be switched off from the command line",
    "example: emulator = true",
];

pub const SERIAL_DOC: &[&str] = &[
    "Serial number of the device to read logs from, as listed by \"adb devices\".",
    "type: string",
    "default: unset (the only connected device is used; the TUI asks when several",
    "are connected)",
    "command line: -s, --serial <DEVICE_SERIAL>",
    "example: serial = \"emulator-5554\"",
];

pub const ALL_DOC: &[&str] = &[
    "Show log messages from all packages instead of only the selected ones.",
    "type: boolean",
    "default: false",
    "command line: -a, --all",
    "note: when true here, it cannot be switched off from the command line",
    "example: all = true",
];

pub const KEEP_DOC: &[&str] = &[
    "Keep the existing logcat buffer instead of clearing it at startup, so older",
    "messages are shown too.",
    "type: boolean",
    "default: false",
    "command line: -k, --keep",
    "note: when true here, it cannot be switched off from the command line",
    "example: keep = true",
];

pub const CURRENT_DOC: &[&str] = &[
    "Add the package of the app currently in the foreground to the package filter.",
    "type: boolean",
    "default: false",
    "command line: -c, --current",
    "note: when true here, it cannot be switched off from the command line",
    "example: current = true",
];

pub const IGNORE_SYSTEM_TAGS_DOC: &[&str] = &[
    "Hide messages from a built-in list of noisy Android system tags such as HWUI,",
    "libEGL and ViewRootImpl. Use `ignore-tag` to hide additional tags.",
    "type: boolean",
    "default: false",
    "command line: -I, --ignore-system-tags",
    "note: when true here, it cannot be switched off from the command line",
    "example: ignore-system-tags = true",
];

pub const TAG_DOC: &[&str] = &[
    "Only show messages whose tag matches one of these entries.",
    "Matching is case-insensitive. An entry containing regex characters is a regular",
    "expression anchored at the start of the tag; any other entry matches anywhere",
    "in the tag. An entry may also be a comma-separated list of tags.",
    "type: array of strings",
    "default: unset (messages with any tag are shown)",
    "command line: -t, --tag <TAG> (repeatable; replaces this list)",
    "example: tag = [\"MainActivity\", \"OkHttp\"]",
];

pub const IGNORE_TAG_DOC: &[&str] = &[
    "Hide messages whose tag matches one of these entries.",
    "Entries are matched exactly like `tag` entries.",
    "type: array of strings",
    "default: unset (no tag is hidden)",
    "command line: -i, --ignore-tag <IGNORED_TAG> (repeatable; replaces this list)",
    "example: ignore-tag = [\"chatty\", \"^Binder.*\"]",
];

pub const LOG_LEVEL_DOC: &[&str] = &[
    "Hide messages below this minimum log level.",
    "values (case-insensitive): \"verbose\" or \"V\", \"debug\" or \"D\", \"info\" or \"I\",",
    "\"warn\" or \"W\", \"error\" or \"E\", \"fatal\" or \"F\"",
    "default: \"verbose\"",
    "command line: -l, --log-level <LEVEL>",
    "example: log-level = \"info\"",
];

pub const REGEX_DOC: &[&str] = &[
    "Only show messages matching this regular expression.",
    "It is passed to \"adb logcat -e\", and the TUI also seeds its filter bar with it.",
    "type: string (regular expression)",
    "default: unset (no message filter)",
    "command line: -r, --regex <REGEX>",
    "example: regex = \"Exception|Error\"",
];

pub const TIMESTAMPS_DOC: &[&str] = &[
    "Show a timestamp column as the first output field.",
    "type: boolean",
    "default: false",
    "command line: -T, --timestamps",
    "note: when true here, it cannot be switched off from the command line",
    "note: use log-format = \"threadtime\" so adb includes a clock time in each line;",
    "with \"brief\", pidcatrs uses the local time when the line is processed",
    "example: timestamps = true",
];

pub const TIMESTAMP_FORMAT_DOC: &[&str] = &[
    "chrono strftime format for the timestamp column. Column width is the longest render over",
    "representative date/times; shorter values are padded. See the chrono specifiers at",
    crate::model::timestamp::CHRONO_STRFTIME_DOCS,
    "type: string",
    "default: \"%I:%M:%S%.3f%p\" (example output: 03:04:05.123pm)",
    "command line: -Z, --timestamp-format <FORMAT>",
    "example: timestamp-format = \"%I:%M:%S%.3f%p\"",
];

pub const LOG_FORMAT_DOC: &[&str] = &[
    "Log format requested from adb (\"adb logcat -v\") and expected in piped input.",
    "values (case-insensitive): \"brief\" or \"B\", \"long\" or \"L\", \"process\" or \"P\",",
    "\"raw\" or \"R\", \"tag\" or \"T\", \"thread\" or \"Th\", \"threadtime\" or \"Tht\",",
    "\"time\" or \"Ti\"",
    "default: \"brief\"",
    "command line: -f, --log-format <FORMAT>",
    "example: log-format = \"threadtime\"",
];

pub const SHOW_PID_DOC: &[&str] = &[
    "Show the process ID column.",
    "type: boolean",
    "default: false",
    "command line: -P, --show-pid",
    "note: when true here, it cannot be switched off from the command line",
    "example: show-pid = true",
];

pub const SHOW_UID_DOC: &[&str] = &[
    "Show the user ID column. The UID is only known when adb includes it in the log",
    "lines.",
    "type: boolean",
    "default: false",
    "command line: -U, --show-uid",
    "note: when true here, it cannot be switched off from the command line",
    "example: show-uid = true",
];

pub const SHOW_PACKAGE_DOC: &[&str] = &[
    "Show the package name column.",
    "type: boolean",
    "default: false",
    "command line: -p, --show-package",
    "note: when true here, it cannot be switched off from the command line",
    "example: show-package = true",
];

pub const ALWAYS_SHOW_TAGS_DOC: &[&str] = &[
    "Print the tag on every line instead of only when it differs from the previous",
    "line.",
    "type: boolean",
    "default: false",
    "command line: -S, --always-show-tags",
    "note: when true here, it cannot be switched off from the command line",
    "example: always-show-tags = true",
];

pub const PUID_WIDTH_DOC: &[&str] = &[
    "Width of the PID and UID columns in characters. Longer values are truncated",
    "with an ellipsis.",
    "type: integer from 1 to 255",
    "default: 5",
    "command line: -x, --puid-width <WIDTH>",
    "example: puid-width = 7",
];

pub const PACKAGE_WIDTH_DOC: &[&str] = &[
    "Width of the package name column in characters. Longer names are truncated",
    "with an ellipsis.",
    "type: integer from 1 to 255",
    "default: 20",
    "command line: -m, --package-width <WIDTH>",
    "example: package-width = 30",
];

pub const TAG_WIDTH_DOC: &[&str] = &[
    "Width of the tag column in characters. Longer tags are truncated with an",
    "ellipsis, and 0 hides the column.",
    "type: integer from 0 to 255",
    "default: 20",
    "command line: -n, --tag-width <WIDTH>",
    "example: tag-width = 24",
];

pub const GC_COLOR_DOC: &[&str] = &[
    "Highlight the freed memory and pause time in garbage collector messages.",
    "type: boolean",
    "default: false",
    "command line: -g, --gc-color",
    "note: when true here, it cannot be switched off from the command line",
    "example: gc-color = true",
];

pub const NO_COLOR_DOC: &[&str] = &[
    "Disable colors in the log output.",
    "type: boolean",
    "default: false",
    "command line: -N, --no-color",
    "note: when true here, it cannot be switched off from the command line",
    "example: no-color = true",
];

pub const OUTPUT_DOC: &[&str] = &[
    "Also save the log output to this file. The file is created, or truncated if it",
    "exists, at startup.",
    "type: string (file path)",
    "default: unset (no file output)",
    "command line: -o, --output <FILE_PATH>",
    "example: output = \"logcat.txt\"",
];

pub const THEME_DOC: &[&str] = &[
    "Color theme of the TUI and of the log output (also in plain mode).",
    "Use a bundled theme name (see the JSON schema enum), a custom theme file name",
    "in the `themes` directory next to the default config file (without the .toml",
    "extension), or a path to a theme file. A value containing a path separator or",
    "ending in .toml is a path; relative paths are resolved from the current",
    "directory. Bundled themes are written to the themes directory when missing.",
    "type: string (bundled name, custom name, or file path)",
    "default: \"gruber-darker\"",
    "command line: --theme <THEME>",
    "example: theme = \"monokai\"",
];

pub const PLAIN_DOC: &[&str] = &[
    "Use plain text output instead of the interactive TUI. Plain output is also",
    "used automatically when standard output is not a terminal.",
    "type: boolean",
    "default: false",
    "command line: --plain",
    "note: when true here, it cannot be switched off from the command line",
    "example: plain = true",
];

pub const CONFIG_DOC: &[&str] = &[
    "Every key mirrors the command-line flag of the same name. A flag passed on the",
    "command line wins over the value in this file, which wins over the built-in",
    "default.",
    "",
    "Boolean keys can only switch options on: when a key is true here, there is no",
    "command-line flag to switch it back off for a single run.",
    "",
    "Commented-out keys have no value; uncomment and edit them to set one.",
    "Unknown keys are rejected.",
];

#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(title = concat!(env!("CARGO_PKG_NAME"), " configuration"))]
#[schemars(description = CONFIG_DOC.join("\n"))]
pub struct Config {
    #[schemars(description = PACKAGES_DOC.join("\n"))]
    pub packages: Option<Vec<String>>,
    #[schemars(description = ADB_DOC.join("\n"))]
    pub adb: Option<String>,
    #[schemars(description = DEVICE_DOC.join("\n"))]
    pub device: Option<bool>,
    #[schemars(description = EMULATOR_DOC.join("\n"))]
    pub emulator: Option<bool>,
    #[schemars(description = SERIAL_DOC.join("\n"))]
    pub serial: Option<String>,
    #[schemars(description = ALL_DOC.join("\n"))]
    pub all: Option<bool>,
    #[schemars(description = KEEP_DOC.join("\n"))]
    pub keep: Option<bool>,
    #[schemars(description = CURRENT_DOC.join("\n"))]
    pub current: Option<bool>,
    #[schemars(description = IGNORE_SYSTEM_TAGS_DOC.join("\n"))]
    pub ignore_system_tags: Option<bool>,
    #[schemars(description = TAG_DOC.join("\n"))]
    pub tag: Option<Vec<String>>,
    #[schemars(description = IGNORE_TAG_DOC.join("\n"))]
    pub ignore_tag: Option<Vec<String>>,
    #[serde(deserialize_with = "parse_value_enum")]
    #[schemars(schema_with = "value_enum_schema::<LogLevel>")]
    #[schemars(description = LOG_LEVEL_DOC.join("\n"))]
    pub log_level: Option<LogLevel>,
    #[schemars(description = REGEX_DOC.join("\n"))]
    pub regex: Option<String>,
    #[serde(deserialize_with = "parse_value_enum")]
    #[schemars(schema_with = "value_enum_schema::<LogFormatKind>")]
    #[schemars(description = LOG_FORMAT_DOC.join("\n"))]
    pub log_format: Option<LogFormatKind>,
    #[schemars(description = TIMESTAMPS_DOC.join("\n"))]
    pub timestamps: Option<bool>,
    #[schemars(description = TIMESTAMP_FORMAT_DOC.join("\n"))]
    pub timestamp_format: Option<String>,
    #[schemars(description = SHOW_PID_DOC.join("\n"))]
    pub show_pid: Option<bool>,
    #[schemars(description = SHOW_UID_DOC.join("\n"))]
    pub show_uid: Option<bool>,
    #[schemars(description = SHOW_PACKAGE_DOC.join("\n"))]
    pub show_package: Option<bool>,
    #[schemars(description = ALWAYS_SHOW_TAGS_DOC.join("\n"))]
    pub always_show_tags: Option<bool>,
    #[schemars(description = PUID_WIDTH_DOC.join("\n"))]
    #[schemars(range(min = 1))]
    pub puid_width: Option<u8>,
    #[schemars(description = PACKAGE_WIDTH_DOC.join("\n"))]
    #[schemars(range(min = 1))]
    pub package_width: Option<u8>,
    #[schemars(description = TAG_WIDTH_DOC.join("\n"))]
    pub tag_width: Option<u8>,
    #[schemars(description = GC_COLOR_DOC.join("\n"))]
    pub gc_color: Option<bool>,
    #[schemars(description = NO_COLOR_DOC.join("\n"))]
    pub no_color: Option<bool>,
    #[schemars(description = OUTPUT_DOC.join("\n"))]
    pub output: Option<String>,
    #[schemars(description = PLAIN_DOC.join("\n"))]
    pub plain: Option<bool>,
    #[schemars(description = THEME_DOC.join("\n"), schema_with = "theme_name_schema")]
    pub theme: Option<String>,
}

fn parse_value_enum<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: ValueEnum,
{
    let Some(value) = Option::<String>::deserialize(deserializer)? else {
        return Ok(None);
    };

    T::from_str(&value, true).map(Some).map_err(|_| {
        let accepted = T::value_variants()
            .iter()
            .filter_map(ValueEnum::to_possible_value)
            .flat_map(|possible| {
                possible
                    .get_name_and_aliases()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .join(", ");

        D::Error::custom(format!(
            "invalid value '{value}', expected one of: {accepted}"
        ))
    })
}

fn set_unless_cli<T>(target: &mut T, value: Option<T>, matches: &ArgMatches, id: &str) {
    if let Some(value) = value
        && matches.value_source(id) != Some(ValueSource::CommandLine)
    {
        *target = value;
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        let path_display = path.display();
        let text = fs::read_to_string(path)
            .map_err(|err| format!("failed to read config file '{path_display}': {err}"))?;

        toml::from_str(&text).map_err(|err| {
            let err = err.to_string();
            format!("invalid config file '{path_display}':\n{}", err.trim_end())
        })
    }

    /// Writes the documented `defaults` to the default config file when it is missing and
    /// never overwrites it; failures are ignored so a read-only home still works.
    pub fn install_default_file(defaults: &CliArgs) {
        let Some(path) = default_config_file() else {
            return;
        };

        if path.exists() {
            return;
        }

        if let Some(dir) = path.parent()
            && fs::create_dir_all(dir).is_err()
        {
            return;
        }

        let _ = fs::write(&path, Self::from_args(defaults).to_doc_toml());
    }

    /// An explicit path must exist; a missing default config file falls back to built-in defaults.
    pub fn load_effective(explicit_path: Option<&str>) -> Result<Self, String> {
        match explicit_path {
            Some(path) => Self::load(Path::new(path)),
            None => match default_config_file() {
                Some(path) if path.is_file() => Self::load(&path),
                _ => Ok(Self::default()),
            },
        }
    }

    pub fn merge_into(self, args: &mut CliArgs, matches: &ArgMatches) {
        let Self {
            packages,
            adb,
            device,
            emulator,
            serial,
            all,
            keep,
            current,
            ignore_system_tags,
            tag,
            ignore_tag,
            log_level,
            regex,
            log_format,
            timestamps,
            timestamp_format,
            show_pid,
            show_uid,
            show_package,
            always_show_tags,
            puid_width,
            package_width,
            tag_width,
            gc_color,
            no_color,
            output,
            plain,
            theme,
        } = self;

        set_unless_cli(&mut args.packages, packages, matches, "packages");
        set_unless_cli(&mut args.adb_path, adb.map(Some), matches, "adb_path");
        set_unless_cli(&mut args.use_device, device, matches, "use_device");
        set_unless_cli(&mut args.use_emulator, emulator, matches, "use_emulator");
        set_unless_cli(
            &mut args.device_serial,
            serial.map(Some),
            matches,
            "device_serial",
        );
        set_unless_cli(&mut args.all, all, matches, "all");
        set_unless_cli(&mut args.keep_logcat, keep, matches, "keep_logcat");
        set_unless_cli(&mut args.current_app, current, matches, "current_app");
        set_unless_cli(
            &mut args.ignore_system_tags,
            ignore_system_tags,
            matches,
            "ignore_system_tags",
        );
        set_unless_cli(&mut args.tag, tag.map(Some), matches, "tag");
        set_unless_cli(
            &mut args.ignore_tag,
            ignore_tag.map(Some),
            matches,
            "ignore_tag",
        );
        set_unless_cli(&mut args.log_level, log_level, matches, "log_level");
        set_unless_cli(&mut args.regex, regex.map(Some), matches, "regex");
        set_unless_cli(
            &mut args.log_format,
            log_format.map(LogFormat::new),
            matches,
            "log_format",
        );
        set_unless_cli(
            &mut args.show_timestamps,
            timestamps,
            matches,
            "show_timestamps",
        );
        set_unless_cli(
            &mut args.timestamp_format,
            timestamp_format,
            matches,
            "timestamp_format",
        );
        set_unless_cli(&mut args.show_pid, show_pid, matches, "show_pid");
        set_unless_cli(&mut args.show_uid, show_uid, matches, "show_uid");
        set_unless_cli(
            &mut args.show_package,
            show_package,
            matches,
            "show_package",
        );
        set_unless_cli(
            &mut args.always_show_tags,
            always_show_tags,
            matches,
            "always_show_tags",
        );
        set_unless_cli(&mut args.puid_width, puid_width, matches, "puid_width");
        set_unless_cli(
            &mut args.package_width,
            package_width,
            matches,
            "package_width",
        );
        set_unless_cli(&mut args.tag_width, tag_width, matches, "tag_width");
        set_unless_cli(&mut args.gc_color, gc_color, matches, "gc_color");
        set_unless_cli(&mut args.no_color, no_color, matches, "no_color");
        set_unless_cli(
            &mut args.output_path,
            output.map(Some),
            matches,
            "output_path",
        );
        set_unless_cli(&mut args.plain, plain, matches, "plain");
        set_unless_cli(&mut args.theme, theme, matches, "theme");
    }

    pub fn from_args(args: &CliArgs) -> Self {
        Self {
            packages: Some(args.packages.clone()),
            adb: args.adb_path.clone(),
            device: Some(args.use_device),
            emulator: Some(args.use_emulator),
            serial: args.device_serial.clone(),
            all: Some(args.all),
            keep: Some(args.keep_logcat),
            current: Some(args.current_app),
            ignore_system_tags: Some(args.ignore_system_tags),
            tag: args.tag.clone(),
            ignore_tag: args.ignore_tag.clone(),
            log_level: Some(args.log_level),
            regex: args.regex.clone(),
            log_format: Some(args.log_format.kind),
            timestamps: Some(args.show_timestamps),
            timestamp_format: Some(args.timestamp_format.clone()),
            show_pid: Some(args.show_pid),
            show_uid: Some(args.show_uid),
            show_package: Some(args.show_package),
            always_show_tags: Some(args.always_show_tags),
            puid_width: Some(args.puid_width),
            package_width: Some(args.package_width),
            tag_width: Some(args.tag_width),
            gc_color: Some(args.gc_color),
            no_color: Some(args.no_color),
            output: args.output_path.clone(),
            plain: Some(args.plain),
            theme: Some(args.theme.clone()),
        }
    }

    pub fn doc_items(&self) -> Vec<DocSection> {
        let Self {
            packages,
            adb,
            device,
            emulator,
            serial,
            all,
            keep,
            current,
            ignore_system_tags,
            tag,
            ignore_tag,
            log_level,
            regex,
            log_format,
            timestamps,
            timestamp_format,
            show_pid,
            show_uid,
            show_package,
            always_show_tags,
            puid_width,
            package_width,
            tag_width,
            gc_color,
            no_color,
            output,
            plain,
            theme,
        } = self.clone();

        vec![
            DocSection {
                title: "Positional arguments",
                table: None,
                doc: &[],
                items: vec![DocItem::optional(
                    "packages",
                    PACKAGES_DOC,
                    packages,
                    vec!["com.example.app"],
                )],
            },
            DocSection {
                title: "Options",
                table: None,
                doc: &[],
                items: vec![DocItem::optional(
                    "adb",
                    ADB_DOC,
                    adb,
                    "/opt/android-sdk/platform-tools/adb",
                )],
            },
            DocSection {
                title: "Device options",
                table: None,
                doc: &["Select which connected device or emulator logs are read from."],
                items: vec![
                    DocItem::optional("device", DEVICE_DOC, device, true),
                    DocItem::optional("emulator", EMULATOR_DOC, emulator, true),
                    DocItem::optional("serial", SERIAL_DOC, serial, "emulator-5554"),
                ],
            },
            DocSection {
                title: "Filtering options",
                table: None,
                doc: &["Choose which log messages are shown."],
                items: vec![
                    DocItem::optional("all", ALL_DOC, all, true),
                    DocItem::optional("keep", KEEP_DOC, keep, true),
                    DocItem::optional("current", CURRENT_DOC, current, true),
                    DocItem::optional(
                        "ignore-system-tags",
                        IGNORE_SYSTEM_TAGS_DOC,
                        ignore_system_tags,
                        true,
                    ),
                    DocItem::optional("tag", TAG_DOC, tag, vec!["MainActivity", "OkHttp"]),
                    DocItem::optional(
                        "ignore-tag",
                        IGNORE_TAG_DOC,
                        ignore_tag,
                        vec!["chatty", "^Binder.*"],
                    ),
                    DocItem::optional(
                        "log-level",
                        LOG_LEVEL_DOC,
                        log_level.map(LogLevel::filter_name),
                        "info",
                    ),
                    DocItem::optional("regex", REGEX_DOC, regex, "Exception|Error"),
                ],
            },
            DocSection {
                title: "Formatting options",
                table: None,
                doc: &["Control the input log format and the columns of the output."],
                items: vec![
                    DocItem::optional(
                        "log-format",
                        LOG_FORMAT_DOC,
                        log_format.map(|kind| kind.to_string()),
                        "threadtime",
                    ),
                    DocItem::optional("timestamps", TIMESTAMPS_DOC, timestamps, true),
                    DocItem::optional(
                        "timestamp-format",
                        TIMESTAMP_FORMAT_DOC,
                        timestamp_format,
                        crate::model::timestamp::DEFAULT_TIMESTAMP_FORMAT,
                    ),
                    DocItem::optional("show-pid", SHOW_PID_DOC, show_pid, true),
                    DocItem::optional("show-uid", SHOW_UID_DOC, show_uid, true),
                    DocItem::optional("show-package", SHOW_PACKAGE_DOC, show_package, true),
                    DocItem::optional(
                        "always-show-tags",
                        ALWAYS_SHOW_TAGS_DOC,
                        always_show_tags,
                        true,
                    ),
                    DocItem::optional(
                        "puid-width",
                        PUID_WIDTH_DOC,
                        puid_width.map(i64::from),
                        7i64,
                    ),
                    DocItem::optional(
                        "package-width",
                        PACKAGE_WIDTH_DOC,
                        package_width.map(i64::from),
                        30i64,
                    ),
                    DocItem::optional("tag-width", TAG_WIDTH_DOC, tag_width.map(i64::from), 24i64),
                ],
            },
            DocSection {
                title: "Color options",
                table: None,
                doc: &[],
                items: vec![
                    DocItem::optional("gc-color", GC_COLOR_DOC, gc_color, true),
                    DocItem::optional("no-color", NO_COLOR_DOC, no_color, true),
                ],
            },
            DocSection {
                title: "Output options",
                table: None,
                doc: &[],
                items: vec![
                    DocItem::optional("output", OUTPUT_DOC, output, "logcat.txt"),
                    DocItem::optional("plain", PLAIN_DOC, plain, true),
                ],
            },
            DocSection {
                title: "Config options",
                table: None,
                doc: &[],
                items: vec![DocItem::optional("theme", THEME_DOC, theme, "monokai")],
            },
        ]
    }

    pub fn to_doc_toml(&self) -> String {
        let pkg = env!("CARGO_PKG_NAME");
        let default_path = default_config_file()
            .map(|path| path.display().to_string())
            .unwrap_or_default();

        let header = [
            format!("{pkg} configuration file"),
            String::default(),
            format!("Default location: {default_path}"),
            "It is created with the built-in defaults when missing and is never".to_string(),
            "overwritten; delete it to restore the defaults.".to_string(),
            format!("Load another file with: {pkg} --config <CONFIG_PATH>"),
            format!("Print the effective configuration with: {pkg} --print-config"),
        ]
        .into_iter()
        .chain([String::default()])
        .chain(CONFIG_DOC.iter().map(|line| line.to_string()))
        .collect::<Vec<_>>();

        format!(
            "{}{}",
            schema_directive_prefix(CONFIG_SCHEMA_URL),
            render(&header, &self.doc_items())
        )
    }
}
