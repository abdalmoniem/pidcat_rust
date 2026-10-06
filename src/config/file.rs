#![deny(clippy::unwrap_used)]

use std::fs;
use std::path::Path;

use clap::ArgMatches;
use clap::ValueEnum;
use clap::parser::ValueSource;

use itertools::Itertools;

use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error;

use crate::CliArgs;
use crate::LogFormat;
use crate::LogFormatKind;
use crate::LogLevel;

use super::paths::default_config_file;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    pub packages: Option<Vec<String>>,
    pub adb: Option<String>,
    pub device: Option<bool>,
    pub emulator: Option<bool>,
    pub serial: Option<String>,
    pub all: Option<bool>,
    pub keep: Option<bool>,
    pub current: Option<bool>,
    pub ignore_system_tags: Option<bool>,
    pub tag: Option<Vec<String>>,
    pub ignore_tag: Option<Vec<String>>,
    #[serde(deserialize_with = "parse_value_enum")]
    pub log_level: Option<LogLevel>,
    pub regex: Option<String>,
    #[serde(deserialize_with = "parse_value_enum")]
    pub log_format: Option<LogFormatKind>,
    pub show_pid: Option<bool>,
    pub show_uid: Option<bool>,
    pub show_package: Option<bool>,
    pub always_show_tags: Option<bool>,
    pub puid_width: Option<u8>,
    pub package_width: Option<u8>,
    pub tag_width: Option<u8>,
    pub gc_color: Option<bool>,
    pub no_color: Option<bool>,
    pub output: Option<String>,
    pub plain: Option<bool>,
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
    }
}
