#![deny(clippy::unwrap_used)]

use clap::ValueEnum;

use schemars::JsonSchema;
use schemars::Schema;
use schemars::SchemaGenerator;
use schemars::generate::SchemaSettings;
use schemars::json_schema;
use schemars::transform::RecursiveTransform;

use super::file::Config;
use super::theme::ThemeFile;

pub const HEX_COLOR_PATTERN: &str = "^#[0-9a-fA-F]{6}$";

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

pub fn hex_color_map_schema(_: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": "object",
        "additionalProperties": {
            "type": "string",
            "pattern": HEX_COLOR_PATTERN,
        },
    })
}

/// TOML has no null, so unset optional keys are described as absent rather than
/// nullable. Draft-07 is the newest draft supported by common TOML language
/// servers.
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

pub fn config_schema() -> String {
    schema_json::<Config>()
}

pub fn theme_schema() -> String {
    schema_json::<ThemeFile>()
}
