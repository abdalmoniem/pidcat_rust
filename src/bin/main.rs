#![deny(clippy::unwrap_used)]

use std::panic;
use std::panic::PanicHookInfo;
use std::process;

use clap::CommandFactory;
use colored::Color;

use is_terminal::IsTerminal;

use pidcat::CliArgs;
use pidcat::Config;
use pidcat::ValueOrPanic;
use pidcat::available_themes;
use pidcat::colored;
use pidcat::exit_with_error;
use pidcat::format_columns;
use pidcat::install_bundled_themes;
use pidcat::load_theme;
use pidcat::print_paged;
use pidcat::restore_tui_terminal;
use pidcat::run_plain;
use pidcat::run_tui;
use pidcat::set_active_theme;
use pidcat::set_running;
use pidcat::write_completions;

use scope_functions::Run;
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
