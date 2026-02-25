set export := true

alias f := fmt
alias l := lint
alias b := build
alias br := build-release
alias t := test
alias r := run
alias rr := run-release
alias i := install

[doc('List available recipes')]
default:
    @just --list --unsorted

[doc('Format code with rustfmt')]
[group('lint')]
[script]
fmt:
    cargo fmt --all -- --check

[doc('Run Clippy linter')]
[group('lint')]
[script]
lint:
    cargo clippy --workspace -- -D warnings

[doc('Build the pidcat binary')]
[group('build')]
[script]
build: fmt lint
    cargo build

[doc('Build the pidcat release binary')]
[group('build')]
[script]
build-release: fmt lint
    cargo build --release

[doc('Run all tests')]
[group('test')]
[script]
test:
    cargo test --workspace -- --include-ignored

[doc('Run all tests using cargo nextest')]
[group('test')]
[script]
nextest:
    cargo nextest run --no-fail-fast --no-output-indent --workspace \
                       --run-ignored=all --final-status-level=all \
                       --no-tests=warn --status-level=all

[doc('Run the pidcat binary')]
[group('build')]
[script]
run:
    cargo run

[doc('Run the pidcat release binary')]
[group('build')]
[script]
run-release:
    cargo run --release

[doc('Install the pidcat binary')]
[group('install')]
[script]
install: fmt lint
    cargo install --path .

    echo
    echo "installed pidcat to $(which pidcat)" | ccze --raw-ansi 2>/dev/null || echo "installed pidcat to $(which pidcat)"

    strip $(which pidcat)
    file $(which pidcat) | ccze --raw-ansi 2>/dev/null || file $(which pidcat)
    ldd $(which pidcat) | ccze --raw-ansi 2>/dev/null || ldd $(which pidcat)
    du -hs --time --time-style=+'%a, %d/%b/%Y - %r' $(which pidcat) | ccze --raw-ansi 2>/dev/null || du -hs --time --time-style=+'%a, %d/%b/%Y - %r' $(which pidcat)
