use build_print::custom_println;
use scope_functions::Apply;
use scope_functions::Run;

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
        custom_println!("INFO:", green, $($arg)+)
    }
}

/// Can be used to print warning messages during a build script.
///
/// Follows the same calling semantics as [std::println!]. Messages are prefixed with
/// **`WARNING:`** in **`yellow`**.
macro_rules! warn {
    ($($arg:tt)+) => {
        custom_println!("WARN:", yellow, $($arg)+)
    }
}

/// Can be used to print error messages during a build script without aborting the build.
///
/// Follows the same calling semantics as [std::println!]. Messages are prefixed with
/// **`ERROR:`** in **`red`**.
macro_rules! error {
    ($($arg:tt)+) => {
        custom_println!("ERROR:", red, $($arg)+)
    }
}

/// Can be used to print note messages during a build script.
///
/// Follows the same calling semantics as [std::println!]. Messages are prefixed with
/// **`NOTE:`** in **`cyan`**.
macro_rules! note {
    ($($arg:tt)+) => {
        custom_println!("NOTE:", cyan, $($arg)+)
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
            "thread 'main' ({pid}) panicked at {file}:{line}:{column}",
            pid = process::id(),
            file = err_loc.file(),
            line = err_loc.line(),
            column = err_loc.column()
        );

        error!("{err_msg}");
    }));

    const VERSION: &str = env!("CARGO_PKG_VERSION");
    const CARGO_TOML: &str = "Cargo.toml";
    const RESOURCES_RC: &str = "resources.rc";
    const SETUP_PATH: &str = "build/setup/setup.iss";

    println!("cargo:rerun-if-changed={CARGO_TOML}");
    println!("cargo:rerun-if-changed={RESOURCES_RC}");
    println!("cargo:rerun-if-changed={SETUP_PATH}");

    info!("CARGO_PKG_VERSION: {VERSION}");

    match read_to_string(SETUP_PATH) {
        Ok(content) => {
            let mut lines = content
                .lines()
                .map(|str| str.to_string())
                .collect::<Vec<_>>();

            let ver_def = "#define AppVersion";
            let new_ver = format!("{ver_def} \"{VERSION}\"");

            let index = match lines.iter().position(|line| line.contains(ver_def)) {
                Some(index) => index,
                None => Error::from(ErrorKind::NotFound)
                    .run(|err| panic!("'AppVersion' was NOT defined in {SETUP_PATH}: {err}")),
            };

            let line_number = index + 1usize;

            let updated_msg =
                format!("Updated AppVersion to {VERSION} in {SETUP_PATH}:{line_number}");
            let already_updated_msg =
                format!("AppVersion is already set to {VERSION} in {SETUP_PATH}:{line_number}");

            match lines[index].trim() != new_ver {
                true => lines
                    .apply_mut(|lines| lines[index] = new_ver)
                    .run(|lines| write(SETUP_PATH, lines.join("\r\n")))
                    .run(|res| match res {
                        Ok(_) => info!("{updated_msg}"),
                        Err(err) => panic!("Failed to write updated setup file: {err}"),
                    }),
                false => note!("{already_updated_msg}"),
            }
        }

        Err(err) => panic!("Failed to read setup file: {err}"),
    }

    embed_resource::compile(RESOURCES_RC, embed_resource::NONE)
        .manifest_optional()
        .run(|res| match res {
            Ok(_) => info!("Manifest data embedded succesfully!"),
            Err(err) => warn!("Failed to embed resources: {err}"),
        });
}
