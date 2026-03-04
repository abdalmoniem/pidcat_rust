set export := true

alias f := fmt
alias l := lint
alias c := clean
alias b := build
alias br := build-release
alias t := test
alias r := run
alias rr := run-release
alias i := install
alias ri := reinstall

TARGET_OS := os()

[doc('List available recipes')]
default:
    @just --list --unsorted

[doc('Format code with rustfmt')]
[group('lint')]
fmt:
    @cargo fmt --all -- --check

[doc('Run Clippy linter')]
[group('lint')]
lint:
    @cargo clippy --workspace -- -D warnings

[doc('Clean the build directory')]
[group('build')]
clean:
    @cargo clean

[doc('Build the pidcat binary')]
[group('build')]
build: fmt lint
    @cargo build

[doc('Build the pidcat release binary')]
[group('build')]
build-release: fmt lint
    @cargo build --release

[doc('Run all tests')]
[group('test')]
test:
    @cargo test --workspace -- --include-ignored

[doc('Run all tests using cargo nextest')]
[group('test')]
nextest:
    @cargo nextest run --no-fail-fast --no-output-indent --workspace \
                       --run-ignored=all --final-status-level=all \
                       --no-tests=warn --status-level=all

[doc('Run the pidcat binary')]
[group('build')]
run:
    @cargo run

[doc('Run the pidcat release binary')]
[group('build')]
run-release:
    @cargo run --release

[doc('Install the pidcat binary')]
[group('install')]
[script]
install: fmt lint
    cargo install --path .

    pidcat_exe="$(which pidcat)"
    pidcat_exe_basename="$(basename "$pidcat_exe")"

    installed_message() {
        echo "installed pidcat to "$pidcat_exe""
    }
    
    file_info() {
        file "$pidcat_exe"
    }

    ldd_info() {
        ldd "$pidcat_exe"
    }

    du_info() {
        du -hs --time --time-style=+'%a, %d/%b/%Y - %r' "$pidcat_exe"
    }

    echo
    echo $(installed_message) | ccze --raw-ansi 2>/dev/null || echo $(installed_message)

    if [ "$TARGET_OS" != "windows" ]; then
        strip "$pidcat_exe" 2>/dev/null || echo "could not strip $pidcat_exe_basename"
    fi

    file_info | ccze --raw-ansi 2>/dev/null || file_info
    ldd_info | ccze --raw-ansi 2>/dev/null || ldd_info
    du_info | ccze --raw-ansi 2>/dev/null || du_info

[doc('Perform a full rebuild, create the installer, and install the application')]
[group('install')]
reinstall: clean install
