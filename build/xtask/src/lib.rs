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

//! Library surface for the `xtask` build helper crate.
//!
//! The binary entry point lives in `main.rs`; this module re-exports the CLI types defined in
//! [`cli_args`] so integration tests or other workspace tools can parse the same command-line
//! interface without duplicating definitions.

mod cli_args;

pub use cli_args::CliArgs;
pub use cli_args::Command;
pub use cli_args::Profile;
