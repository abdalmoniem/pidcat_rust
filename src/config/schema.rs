// Copyright (C) AbdAlMoniem AlHifnawy <hifnawy_moniem@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![deny(clippy::unwrap_used)]

//! JSON Schema (Draft 07) generation for the config and theme TOML files.

use clap::ValueEnum;

use schemars::JsonSchema;
use schemars::Schema;
use schemars::SchemaGenerator;
use schemars::generate::SchemaSettings;
use schemars::json_schema;
use schemars::transform::RecursiveTransform;

use serde_json::json;

use super::file::Config;
use super::theme::BUNDLED_THEMES;
use super::theme::ThemeFile;

/// URL of the published JSON Schema for [`Config`], referenced from generated `config.toml`.
pub const CONFIG_SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/abdalmoniem/pidcatrs/main/schemas/config.schema.json";
/// URL of the published JSON Schema for [`ThemeFile`], referenced from theme TOML files.
pub const THEME_SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/abdalmoniem/pidcatrs/main/schemas/theme.schema.json";

/// Regular expression pattern for `#rrggbb` hex colors in schema `pattern` fields.
pub const HEX_COLOR_PATTERN: &str = "^#[0-9a-fA-F]{6}$";

/// Returns the TOML language-server schema directive, a blank `#` comment line, then an empty body prefix.
pub fn schema_directive_prefix(url: &str) -> String {
    format!("#:schema {url}\n#\n")
}

/// Builds a JSON Schema `string` enum from a clap [`ValueEnum`] type's names and aliases.
pub fn value_enum_schema<T: ValueEnum>(_: &mut SchemaGenerator) -> Schema {
    let values = T::value_variants()
        .iter()
        .filter_map(ValueEnum::to_possible_value)
        .flat_map(|possible| {
            possible
                .get_name_and_aliases()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    json_schema!({
        "type": "string",
        "enum": values,
    })
}

/// JSON Schema for a TOML table whose values are hex color strings.
pub fn hex_color_map_schema(_: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": "object",
        "additionalProperties": {
            "type": "string",
            "pattern": HEX_COLOR_PATTERN,
        },
    })
}

/// JSON Schema for the `theme` config key: bundled names plus any custom name or path string.
///
/// `anyOf` keeps a top-level-style `enum` for editor completion while a plain `string` branch
/// accepts custom names and paths. (`oneOf` fails on bundled names because both branches match.)
pub fn theme_name_schema(_: &mut SchemaGenerator) -> Schema {
    let names = BUNDLED_THEMES
        .iter()
        .map(|theme| json!(theme.name))
        .collect::<Vec<_>>();

    serde_json::from_value(json!({
        "anyOf": [
            {
                "type": "string",
                "enum": names,
            },
            {
                "type": "string",
                "minLength": 1,
            }
        ]
    }))
    .expect("valid theme name schema")
}

/// Serializes `T`'s JSON Schema as pretty-printed Draft-07 JSON, stripping null defaults and types.
///
/// TOML has no null, so unset optional keys are described as absent rather than
/// nullable. Draft-07 is the newest draft supported by common TOML language servers.
fn schema_json<T: JsonSchema>() -> String {
    let drop_null = RecursiveTransform(|schema: &mut Schema| {
        if schema.get("default").is_some_and(|value| value.is_null()) {
            schema.remove("default");
        }

        let single_type = schema
            .get_mut("type")
            .and_then(|kind| kind.as_array_mut())
            .and_then(|types| {
                types.retain(|kind| kind != "null");

                match types.as_slice() {
                    [kind] => Some(kind.clone()),
                    _ => None,
                }
            });

        if let Some(kind) = single_type {
            schema.insert("type".to_string(), kind);
        }
    });
    let schema = SchemaSettings::draft07()
        .with_transform(drop_null)
        .into_generator()
        .into_root_schema_for::<T>();

    format!("{:#}\n", schema.as_value())
}

/// Returns the JSON Schema document for [`Config`].
pub fn config_schema() -> String {
    schema_json::<Config>()
}

/// Returns the JSON Schema document for [`ThemeFile`].
pub fn theme_schema() -> String {
    schema_json::<ThemeFile>()
}
