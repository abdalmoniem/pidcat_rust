use build_print::custom_println;

use std::fs::read_to_string;
use std::fs::write;

use std::io::Error;
use std::io::ErrorKind;

use std::panic;
use std::process;

/// Can be used to print info messages during a build script.
///
/// Follows the same calling semantics as [std::println!]. Messages are prefixed with
/// **`INFO:`** in **`green`**.
macro_rules! info {
    ($($arg:tt)+) => {
        custom_println!("INFO:", green, $($arg)+);
    }
}

/// Can be used to print warning messages during a build script.
///
/// Follows the same calling semantics as [std::println!]. Messages are prefixed with
/// **`WARNING:`** in **`yellow`**.
macro_rules! warn {
    ($($arg:tt)+) => {
        custom_println!("WARN:", yellow, $($arg)+);
    }
}

/// Can be used to print error messages during a build script without aborting the build.
///
/// Follows the same calling semantics as [std::println!]. Messages are prefixed with
/// **`ERROR:`** in **`red`**.
macro_rules! error {
    ($($arg:tt)+) => {
        custom_println!("ERROR:", red, $($arg)+);
    }
}

/// Can be used to print note messages during a build script.
///
/// Follows the same calling semantics as [std::println!]. Messages are prefixed with
/// **`NOTE:`** in **`cyan`**.
macro_rules! note {
    ($($arg:tt)+) => {
        custom_println!("NOTE:", cyan, $($arg)+);
    }
}

/// The main entry point for the build script.
///
/// This function is responsible for setting a `custom panic hook`, reading the setup file and updating
/// the `AppVersion` if necessary.
///
/// The `panic hook` is responsible for catching any [std::panic]s that occur during the execution of the build
/// script and printing a custom error message.
///
/// The build script reads the setup file and checks if the `AppVersion` is already defined. If it is not,
/// the script will panic with a custom error message.
///
/// If the `AppVersion` is already defined, the script will check if the version matches the one defined in
/// the `Cargo.toml` file. If it does not, the script will update the `AppVersion` in the setup file.
///
/// The script will then use the [embed_resource::compile] to compile the resource file and embed it into the
/// executable.
///
/// If the compilation fails, the script will print a warning message with the error message.
fn main() {
    panic::set_hook(Box::new(|info| {
        let err_loc = info.location().unwrap_or(panic::Location::caller());
        let err_msg = match info.payload().downcast_ref::<&str>() {
            Some(str) => *str,
            None => match info.payload().downcast_ref::<String>() {
                Some(str) => &str[..],
                None => "Box<Any>",
            },
        };

        error!(
            "thread 'main' ({}) panicked at {}:{}:{}",
            process::id(),
            err_loc.file(),
            err_loc.line(),
            err_loc.column()
        );

        error!("{err_msg}");
    }));

    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=resources.rc");

    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const SETUP_PATH: &str = "build/setup/setup.iss";

    info!("CARGO_PKG_VERSION: {}", VERSION);

    match read_to_string(SETUP_PATH) {
        Ok(content) => {
            let mut lines = content
                .lines()
                .map(|str| str.to_string())
                .collect::<Vec<_>>();
            let mut updated = false;
            let mut match_index = None;

            let defined_version = "#define AppVersion";
            let new_line = format!("{} \"{}\"", defined_version, VERSION);

            for (index, line) in lines.iter().enumerate() {
                if line.contains(defined_version) {
                    match_index = Some(index);
                    break;
                }
            }

            if let Some(index) = match_index
                && lines[index].trim() != new_line
            {
                lines[index] = new_line;
                updated = true;
            }

            if let Some(match_index) = match_index
                && updated
            {
                if let Err(err) = write(SETUP_PATH, lines.join("\r\n")) {
                    panic!("Failed to write updated setup file: {}", err);
                }

                info!(
                    "Updated AppVersion to {} in {}:{}",
                    VERSION,
                    SETUP_PATH,
                    match_index + 1
                );
            } else if let Some(match_index) = match_index {
                note!(
                    "AppVersion is already set to {} in {}:{}",
                    VERSION,
                    SETUP_PATH,
                    match_index + 1
                );
            } else {
                let err = Error::from(ErrorKind::NotFound);
                let err_message = format!("'AppVersion' was NOT defined in {}", SETUP_PATH);

                panic!("{}: {}", err_message, err);
            }
        }

        Err(err) => panic!("Failed to read setup file: {}", err),
    }


    let manifest_result =
        embed_resource::compile("resources.rc", embed_resource::NONE).manifest_optional();

    if let Err(err) = manifest_result {
        warn!("Failed to embed resources: {}", err);
    }
}
