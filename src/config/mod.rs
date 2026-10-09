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

//! Configuration paths, file format, JSON Schema, shell completions, and color themes.
//!
//! The [`file::Config`] type mirrors CLI flags; [`paths`] resolves the default config
//! and themes directories; [`schema`] builds Draft-07 JSON Schema strings; [`completions`]
//! augments generated shell scripts; [`theme`] loads and resolves color themes.

/// Shell completion generation with dynamic theme and option value completion.
pub mod completions;
/// Commented TOML rendering for documented config and theme files.
pub mod doc_toml;
/// Main configuration struct, loading, merging, and documented TOML export.
pub mod file;
/// Platform-specific config and themes directory paths.
pub mod paths;
/// JSON Schema generation for config and theme files.
pub mod schema;
/// Theme file parsing, bundled themes, and resolved runtime colors.
pub mod theme;
