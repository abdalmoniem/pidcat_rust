set export := true

alias f := fmt
alias l := lint
alias c := clean
alias b := build
alias br := build-release
alias bi := build-installer
alias t := test
alias r := run
alias rr := run-release
alias i := install
alias ri := reinstall

export TARGET_OS := os()

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
    @cargo xtask clean --profile=both

[doc('Build the pidcat binary')]
[group('build')]
build: fmt lint
    @cargo xtask build --profile=dev

[doc('Build the pidcat release binary')]
[group('build')]
build-release: fmt lint
    @cargo xtask build --profile=release

[doc('Build the installer using Inno Setup Compiler')]
[group('build')]
build-installer:
    @cargo xtask build-installer

[arg('profile', help='the pidcat binary build profile')]
[doc('Perform a full rebuild and create the installer')]
[group('build')]
[windows]
build-all profile:
    @cargo xtask build-all --profile=$profile

[doc('Run the pidcat binary')]
[group('build')]
run args:
    @cargo xtask run -- $args

[doc('Run the pidcat release binary')]
[group('build')]
run-release args:
    @cargo xtask run --profile=release -- $args

[doc('Run all tests')]
[group('test')]
test:
    @cargo test --workspace

[doc('Run all tests using cargo nextest')]
[group('test')]
nextest:
    @cargo nextest run --no-fail-fast --no-output-indent --workspace \
                       --final-status-level=all --no-tests=warn \
                       --status-level=all

[doc('Install the application by running the generated installer')]
[group('install')]
[script]
install:
    if [ "$TARGET_OS" != "windows" ]; then
        cargo xtask install
    else
        cargo xtask install --silent
    fi

    just post_install

[doc('Perform a full rebuild, create the installer, and install the application')]
[group('install')]
[script]
reinstall:
    if [ "$TARGET_OS" != "windows" ]; then
        cargo xtask reinstall
    else
        cargo xtask reinstall --silent
    fi

    just post_install

[arg('tag', help='the tag to show changelog for')]
[doc('shows changelog for tag')]
[group('changelog')]
tag_changelog tag:
    @git-cliff --offline --body="$(cat cliff_body.tera)" \
               "$(git describe --tags --abbrev=0 $tag^ 2>/dev/null || git rev-list --max-parents=0 HEAD)..$tag"

[doc('shows changelog for all tagged commits')]
[group('changelog')]
tags_changelog:
    @git-cliff --offline --body="$(cat cliff_body.tera)" --tag "$(git describe --tags --abbrev=0)"

[doc('shows changelog for untagged commits')]
[group('changelog')]
unreleased_changelog:
    @git-cliff --offline --body="$(cat cliff_body.tera)" "$(git describe --tags --abbrev=0)..HEAD"

[doc('shows changelog for all commits')]
[group('changelog')]
all_changelog:
    @git-cliff --offline --body="$(cat cliff_body.tera)"

[doc('updates CHANGELOG.md with changelog from all tagged commits')]
[group('changelog')]
update_changelog:
    @git-cliff --offline --body="$(cat cliff_body.tera)" --tag "$(git describe --tags --abbrev=0)" | tee CHANGELOG.md
    @echo
    @echo "changelog written to '$(realpath CHANGELOG.md)'!"

[private]
[script]
post_install:
    pidcat_exe="$(which pidcat)"
    pidcat_exe_basename="$(basename "$pidcat_exe")"

    echo
    if command -v ccze >/dev/null 2>&1; then
        just installed_message "$pidcat_exe" | ccze --raw-ansi
    else
        just installed_message "$pidcat_exe"
    fi

    if [ "$TARGET_OS" != "windows" ]; then
        strip "$pidcat_exe" 2>/dev/null || echo "could not strip $pidcat_exe_basename"
    fi

    if command -v ccze >/dev/null 2>&1; then
        just file_info "$pidcat_exe" | ccze --raw-ansi
        just ldd_info  "$pidcat_exe" | ccze --raw-ansi
        just du_info   "$pidcat_exe" | ccze --raw-ansi
    else
        just file_info "$pidcat_exe"
        just ldd_info  "$pidcat_exe"
        just du_info   "$pidcat_exe"
    fi

[private]
installed_message pidcat_exe:
    @echo "installed pidcat to "$pidcat_exe""

[private]
file_info pidcat_exe:
    @file "$pidcat_exe"

[private]
ldd_info pidcat_exe:
    @ldd "$pidcat_exe"

[private]
du_info pidcat_exe:
    @du -hs --time --time-style=+'%a, %d/%b/%Y - %r' "$pidcat_exe"
