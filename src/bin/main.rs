// Copyright (c) AbdAlMoniem AlHifnawy <hifnawy_moniem@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! pidcatrs binary entry point: CLI parsing, early-exit utilities, and plain or TUI log viewing.

#![deny(clippy::unwrap_used)]

use std::panic;
use std::panic::PanicHookInfo;
use std::process;

use clap::CommandFactory;
use colored::Color;

use is_terminal::IsTerminal;

use pidcatrs::CliArgs;
use pidcatrs::Config;
use pidcatrs::ValueOrPanic;
use pidcatrs::available_themes;
use pidcatrs::colored;
use pidcatrs::exit_with_error;
use pidcatrs::format_columns;
use pidcatrs::install_bundled_themes;
use pidcatrs::load_theme;
use pidcatrs::print_paged;
use pidcatrs::restore_tui_terminal;
use pidcatrs::run_plain;
use pidcatrs::run_tui;
use pidcatrs::set_active_theme;
use pidcatrs::set_running;
use pidcatrs::write_completions;

use scope_functions::Run;

/// Custom panic hook that restores the terminal, prints location-aware messages, and respects `--no-color`.
///
/// Called for any thread panic after [`main`] installs this hook. Ensures the TUI does not leave
/// the terminal in raw mode when the process aborts.
fn panic_hook(info: &PanicHookInfo, show_colors: bool) {
    let err_loc = info.location().unwrap_or(panic::Location::caller());
    let err_msg = match info.payload().downcast_ref::<&str>() {
        Some(str) => *str,
        None => match info.payload().downcast_ref::<String>() {
            Some(str) => &str[..],
            None => "Box<Any>",
        },
    };

    let err_msg = format!(
        "{err_msg} => {file}:{line}:{column}",
        file = err_loc.file(),
        line = err_loc.line(),
        column = err_loc.column()
    )
    .run(|msg| colored(msg, show_colors, Color::BrightRed));

    let thread_err_msg = format!(
        "thread 'main' ({pid}) panicked at {file}:{line}:{column}",
        pid = process::id(),
        file = err_loc.file(),
        line = err_loc.line(),
        column = err_loc.column()
    )
    .run(|msg| colored(msg, show_colors, Color::BrightRed));

    restore_tui_terminal();
    eprintln!("{thread_err_msg}");
    eprintln!("{err_msg}");
}

/// Parses CLI flags, handles utility subcommands, loads theme and config, then runs TUI or plain mode.
fn main() {
    let args = &mut CliArgs::parse_args();
    let show_colors = !args.no_color;

    panic::set_hook(Box::new(move |info| panic_hook(info, show_colors)));
    ctrlc::set_handler(move || set_running(false)).unwrap_or_panic("Failed to set CTRL+C handler");

    if let Some(shell) = args.completions {
        write_completions(shell, &mut CliArgs::command(), &mut std::io::stdout())
            .unwrap_or_else(|err| exit_with_error(&err, show_colors));

        process::exit(0i32);
    }

    if args.print_config {
        print!("{}", Config::from_args(args).to_doc_toml());

        process::exit(0i32);
    }

    install_bundled_themes();

    if args.list_themes {
        let themes = available_themes();

        let listing = match std::io::stdout().is_terminal() {
            true => format_columns(&themes),
            false => themes.iter().map(|name| format!("{name}\n")).collect(),
        };

        print_paged(&listing);

        process::exit(0i32);
    }

    let (theme_file, theme) =
        load_theme(&args.theme).unwrap_or_else(|err| exit_with_error(&err, show_colors));

    if args.print_theme {
        print!("{}", theme_file.to_doc_toml());

        process::exit(0i32);
    }

    set_active_theme(theme);

    let use_tui = !args.plain && std::io::stdout().is_terminal();

    match use_tui {
        true => run_tui(args),
        false => run_plain(args),
    }
}
