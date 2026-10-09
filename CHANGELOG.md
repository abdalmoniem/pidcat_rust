# [2.0.0] - Fri, 09/Oct/2026
## 🚀 Features
- ([38b5cc](https://github.com/abdalmoniem/pidcatrs/commit/38b5cc)) **(log format)** Added configureable input log formats
- ([0ea48c](https://github.com/abdalmoniem/pidcatrs/commit/0ea48c)) **(tui)** Add ratatui log viewer with Studio-style filters
- ([e84dd7](https://github.com/abdalmoniem/pidcatrs/commit/e84dd7)) **(tui)** Overhaul border rendering, titles, and log ingest pipeline
- ([9a58d7](https://github.com/abdalmoniem/pidcatrs/commit/9a58d7)) **(tui)** Optimize pause handling, live ingest, and entry counting
- ([d66d60](https://github.com/abdalmoniem/pidcatrs/commit/d66d60)) **(tui)** Add editable path input to file explorer
- ([d09b39](https://github.com/abdalmoniem/pidcatrs/commit/d09b39)) **(tui)** Add export functionality and time/chrono support
- ([27d637](https://github.com/abdalmoniem/pidcatrs/commit/27d637)) **(tui)** Add export format selection for pidcat and raw adb output
- ([1edd1a](https://github.com/abdalmoniem/pidcatrs/commit/1edd1a)) **(tui)** Export currently filtered log entries instead of all
- ([09d9b6](https://github.com/abdalmoniem/pidcatrs/commit/09d9b6)) **(config)** Add app config and theme customizations
- ([3fd17b](https://github.com/abdalmoniem/pidcatrs/commit/3fd17b)) **(config)** Load config.toml mirroring all CLI flags with --config override
- ([576dab](https://github.com/abdalmoniem/pidcatrs/commit/576dab)) **(config)** Add --print-config to dump the documented effective configuration
- ([ab7da6](https://github.com/abdalmoniem/pidcatrs/commit/ab7da6)) **(theme)** Drive TUI and log colors from a runtime theme
- ([d6253f](https://github.com/abdalmoniem/pidcatrs/commit/d6253f)) **(theme)** Select theme from config and install bundled themes
- ([bfe68e](https://github.com/abdalmoniem/pidcatrs/commit/bfe68e)) **(config)** Add JSON schemas for config and theme files
- ([d6fcf2](https://github.com/abdalmoniem/pidcatrs/commit/d6fcf2)) **(theme)** Ship bundled theme sources with full documentation
- ([619558](https://github.com/abdalmoniem/pidcatrs/commit/619558)) **(theme)** Generate the bundled theme registry from the theme files
- ([57d624](https://github.com/abdalmoniem/pidcatrs/commit/57d624)) **(theme)** Bundle 215 imported color themes
- ([38c5c0](https://github.com/abdalmoniem/pidcatrs/commit/38c5c0)) **(cli)** Add `--list-themes` flag, dynamic shell completion, and terminal pagination
- ([839479](https://github.com/abdalmoniem/pidcatrs/commit/839479)) **(timestamp)** Add configurable log timestamps and formatting options
- ([efd606](https://github.com/abdalmoniem/pidcatrs/commit/efd606)) **(log format)** Implement remaining adb logcat output formats
- ([940033](https://github.com/abdalmoniem/pidcatrs/commit/940033)) **(config)** Add JSON schema directives and enrich schema validation

## 🐛 Bug Fixes
- ([26cde5](https://github.com/abdalmoniem/pidcatrs/commit/26cde5)) **(bin/cli_args)** Fix help message spacing
- ([76a39c](https://github.com/abdalmoniem/pidcatrs/commit/76a39c)) **(tui)** Fix unknown packages filtering
- ([b46795](https://github.com/abdalmoniem/pidcatrs/commit/b46795)) **(clipboard)** Replace arboard with terminal-clipboard
- ([8f048e](https://github.com/abdalmoniem/pidcatrs/commit/8f048e)) **(tui)** Account for scrollbar gutter and borders in layout
- ([132822](https://github.com/abdalmoniem/pidcatrs/commit/132822)) **(theme)** Refresh outdated bundled theme files
- ([ef924c](https://github.com/abdalmoniem/pidcatrs/commit/ef924c)) **(config)** Create the documented default config file when missing
- ([2f3fd4](https://github.com/abdalmoniem/pidcatrs/commit/2f3fd4)) **(timestamp)** Support variable-width chrono strftime formats

## ⚡ Performance Improvements
- ([cc473e](https://github.com/abdalmoniem/pidcatrs/commit/cc473e)) **(writer)** Massive performance improvement
- ([76567b](https://github.com/abdalmoniem/pidcatrs/commit/76567b)) **(tui)** Optimize log ingestion, filtering, and rendering performance

## ♻️ Refactors
- ([d93fd8](https://github.com/abdalmoniem/pidcatrs/commit/d93fd8)) **(general)** Code Refactoring and Improvements
- ([8440c1](https://github.com/abdalmoniem/pidcatrs/commit/8440c1)) **(tui)** Use unicode symbols for arrow keys in help catalog
- ([22d8de](https://github.com/abdalmoniem/pidcatrs/commit/22d8de)) **(tui)** Optimize palette search selection navigation logic
- ([ea25b9](https://github.com/abdalmoniem/pidcatrs/commit/ea25b9)) **(tui)** Offload clipboard operations to background threads
- ([d01a40](https://github.com/abdalmoniem/pidcatrs/commit/d01a40)) **(tui)** Replace polling with event stream
- ([98b203](https://github.com/abdalmoniem/pidcatrs/commit/98b203)) **(xtask)** Add explicit generate and check commands for schemas and themes
- ([3aa964](https://github.com/abdalmoniem/pidcatrs/commit/3aa964)) **(project)** Rename crate from pidcat to pidcatrs across all files

## 🎨 Code Style
- ([71e9c1](https://github.com/abdalmoniem/pidcatrs/commit/71e9c1)) **(Justfile)** Reformat Justfile

## 📚 Documentation
- ([1d9c3f](https://github.com/abdalmoniem/pidcatrs/commit/1d9c3f)) **(changelog)** Update CHANGELOG.md
- ([388258](https://github.com/abdalmoniem/pidcatrs/commit/388258)) **(model::mod.rs)** Add copyright header and comprehensive rustdocs
- ([02226d](https://github.com/abdalmoniem/pidcatrs/commit/02226d)) **(model::adb_device.rs)** Add copyright header and comprehensive rustdocs
- ([cd3fee](https://github.com/abdalmoniem/pidcatrs/commit/cd3fee)) **(model::adb_state.rs)** Add copyright header and comprehensive rustdocs
- ([af2317](https://github.com/abdalmoniem/pidcatrs/commit/af2317)) **(model::ansi.rs)** Add copyright header and comprehensive rustdocs
- ([bef769](https://github.com/abdalmoniem/pidcatrs/commit/bef769)) **(model::ansi_segment.rs)** Add copyright header and comprehensive rustdocs
- ([a13897](https://github.com/abdalmoniem/pidcatrs/commit/a13897)) **(model::cli_args.rs)** Add copyright header and comprehensive rustdocs
- ([4f32c1](https://github.com/abdalmoniem/pidcatrs/commit/4f32c1)) **(model::filter.rs)** Add copyright header and comprehensive rustdocs
- ([cfceb6](https://github.com/abdalmoniem/pidcatrs/commit/cfceb6)) **(model::log_entry.rs)** Add copyright header and comprehensive rustdocs
- ([77b9c2](https://github.com/abdalmoniem/pidcatrs/commit/77b9c2)) **(model::log_format.rs)** Add copyright header and comprehensive rustdocs
- ([618af8](https://github.com/abdalmoniem/pidcatrs/commit/618af8)) **(model::log_level.rs)** Add copyright header and comprehensive rustdocs
- ([674d4a](https://github.com/abdalmoniem/pidcatrs/commit/674d4a)) **(model::log_source.rs)** Add copyright header and comprehensive rustdocs
- ([cee5b3](https://github.com/abdalmoniem/pidcatrs/commit/cee5b3)) **(model::option_unwrap.rs)** Add copyright header and comprehensive rustdocs
- ([4e81f7](https://github.com/abdalmoniem/pidcatrs/commit/4e81f7)) **(model::result_unwrap.rs)** Add copyright header and comprehensive rustdocs
- ([38f2f3](https://github.com/abdalmoniem/pidcatrs/commit/38f2f3)) **(model::state.rs)** Add copyright header and comprehensive rustdocs
- ([9819a7](https://github.com/abdalmoniem/pidcatrs/commit/9819a7)) **(model::timestamp.rs)** Add copyright header and comprehensive rustdocs
- ([e7aab7](https://github.com/abdalmoniem/pidcatrs/commit/e7aab7)) **(model::tui_filter.rs)** Add copyright header and comprehensive rustdocs
- ([18f2cd](https://github.com/abdalmoniem/pidcatrs/commit/18f2cd)) **(model::value_unwrap.rs)** Add copyright header and comprehensive rustdocs
- ([fe18a5](https://github.com/abdalmoniem/pidcatrs/commit/fe18a5)) **(config::mod.rs)** Add copyright header and comprehensive rustdocs
- ([7a3f01](https://github.com/abdalmoniem/pidcatrs/commit/7a3f01)) **(config::paths.rs)** Add copyright header and comprehensive rustdocs
- ([70920c](https://github.com/abdalmoniem/pidcatrs/commit/70920c)) **(config::doc_toml.rs)** Add copyright header and comprehensive rustdocs
- ([8d07e8](https://github.com/abdalmoniem/pidcatrs/commit/8d07e8)) **(config::schema.rs)** Add copyright header and comprehensive rustdocs
- ([84a0ee](https://github.com/abdalmoniem/pidcatrs/commit/84a0ee)) **(config::completions.rs)** Add copyright header and comprehensive rustdocs
- ([fc7a35](https://github.com/abdalmoniem/pidcatrs/commit/fc7a35)) **(config::file.rs)** Add copyright header and comprehensive rustdocs
- ([9085b1](https://github.com/abdalmoniem/pidcatrs/commit/9085b1)) **(config::theme.rs)** Add copyright header and comprehensive rustdocs
- ([f96bd6](https://github.com/abdalmoniem/pidcatrs/commit/f96bd6)) **(controller::mod.rs)** Add copyright header and comprehensive rustdocs
- ([d01da2](https://github.com/abdalmoniem/pidcatrs/commit/d01da2)) **(controller::adb.rs)** Add copyright header and comprehensive rustdocs
- ([baa1e2](https://github.com/abdalmoniem/pidcatrs/commit/baa1e2)) **(controller::ansi.rs)** Add copyright header and comprehensive rustdocs
- ([125731](https://github.com/abdalmoniem/pidcatrs/commit/125731)) **(controller::log_processor.rs)** Add copyright header and comprehensive rustdocs
- ([c478d4](https://github.com/abdalmoniem/pidcatrs/commit/c478d4)) **(controller::plain.rs)** Add copyright header and comprehensive rustdocs
- ([a46c7d](https://github.com/abdalmoniem/pidcatrs/commit/a46c7d)) **(controller::setup.rs)** Add copyright header and comprehensive rustdocs
- ([c662dd](https://github.com/abdalmoniem/pidcatrs/commit/c662dd)) **(controller::terminal.rs)** Add copyright header and comprehensive rustdocs
- ([aa5a2e](https://github.com/abdalmoniem/pidcatrs/commit/aa5a2e)) **(controller::util.rs)** Add copyright header and comprehensive rustdocs
- ([583db0](https://github.com/abdalmoniem/pidcatrs/commit/583db0)) **(controller::writer.rs)** Add copyright header and comprehensive rustdocs
- ([0018fa](https://github.com/abdalmoniem/pidcatrs/commit/0018fa)) **(controller::tui::mod.rs)** Add copyright header and comprehensive rustdocs
- ([d0dba0](https://github.com/abdalmoniem/pidcatrs/commit/d0dba0)) **(controller::tui::app.rs)** Add copyright header and comprehensive rustdocs
- ([4c2e2c](https://github.com/abdalmoniem/pidcatrs/commit/4c2e2c)) **(controller::tui::border.rs)** Add copyright header and comprehensive rustdocs
- ([bc6184](https://github.com/abdalmoniem/pidcatrs/commit/bc6184)) **(controller::tui::copy.rs)** Add copyright header and comprehensive rustdocs
- ([645ccc](https://github.com/abdalmoniem/pidcatrs/commit/645ccc)) **(controller::tui::device_picker.rs)** Add copyright header and comprehensive rustdocs
- ([d5b4fa](https://github.com/abdalmoniem/pidcatrs/commit/d5b4fa)) **(controller::tui::display_cache.rs)** Add copyright header and comprehensive rustdocs
- ([b56dde](https://github.com/abdalmoniem/pidcatrs/commit/b56dde)) **(controller::tui::export.rs)** Add copyright header and comprehensive rustdocs
- ([ef36d8](https://github.com/abdalmoniem/pidcatrs/commit/ef36d8)) **(controller::tui::file_source.rs)** Add copyright header and comprehensive rustdocs
- ([a91004](https://github.com/abdalmoniem/pidcatrs/commit/a91004)) **(controller::tui::help.rs)** Add copyright header and comprehensive rustdocs
- ([5763b4](https://github.com/abdalmoniem/pidcatrs/commit/5763b4)) **(controller::tui::log_ingest.rs)** Add copyright header and comprehensive rustdocs
- ([4bc292](https://github.com/abdalmoniem/pidcatrs/commit/4bc292)) **(controller::tui::palette.rs)** Add copyright header and comprehensive rustdocs
- ([e6a123](https://github.com/abdalmoniem/pidcatrs/commit/e6a123)) **(controller::tui::theme.rs)** Add copyright header and comprehensive rustdocs
- ([8bde74](https://github.com/abdalmoniem/pidcatrs/commit/8bde74)) **(controller::tui::ui.rs)** Add copyright header and comprehensive rustdocs
- ([faa65e](https://github.com/abdalmoniem/pidcatrs/commit/faa65e)) **(crate root)** Add copyright header and comprehensive rustdocs
- ([36c099](https://github.com/abdalmoniem/pidcatrs/commit/36c099)) **(bin)** Add copyright header and comprehensive rustdocs
- ([ab7abd](https://github.com/abdalmoniem/pidcatrs/commit/ab7abd)) **(build)** Add copyright header and comprehensive rustdocs
- ([c7719f](https://github.com/abdalmoniem/pidcatrs/commit/c7719f)) **(xshell)** Add copyright header and comprehensive rustdocs
- ([d502dd](https://github.com/abdalmoniem/pidcatrs/commit/d502dd)) **(xshell::error)** Add copyright header and comprehensive rustdocs
- ([c2171e](https://github.com/abdalmoniem/pidcatrs/commit/c2171e)) **(xtask)** Add copyright header and comprehensive rustdocs
- ([55096c](https://github.com/abdalmoniem/pidcatrs/commit/55096c)) **(xtask::main)** Add copyright header and comprehensive rustdocs
- ([f756db](https://github.com/abdalmoniem/pidcatrs/commit/f756db)) **(xtask::cli_args)** Add copyright header and comprehensive rustdocs
- ([2d9ef9](https://github.com/abdalmoniem/pidcatrs/commit/2d9ef9)) **(changelog)** Update CHANGELOG.md

## 🛠️ Maintenance
- ([ee6adb](https://github.com/abdalmoniem/pidcatrs/commit/ee6adb)) **(cliff.toml)** Remove body template from `cliff.toml`
- ([60f799](https://github.com/abdalmoniem/pidcatrs/commit/60f799)) **(cliff)** Add commit hash to changelog
- ([cc7178](https://github.com/abdalmoniem/pidcatrs/commit/cc7178)) **(lint)** Fix linter errrors
- ([20f5e7](https://github.com/abdalmoniem/pidcatrs/commit/20f5e7)) **(release)** Add automated tagging
- ([1263f8](https://github.com/abdalmoniem/pidcatrs/commit/1263f8)) **(justfile)** Update changelog commit message
- ([47c6e8](https://github.com/abdalmoniem/pidcatrs/commit/47c6e8)) **(package)** Bump version to 2.0.0

**Full Changelog**: [v1.2.1...v2.0.0](https://github.com/abdalmoniem/pidcatrs/compare/v1.2.1...v2.0.0)

---
# [1.2.1] - Mon, 09/Mar/2026
## ♻️ Refactors
- ([a14eb9](https://github.com/abdalmoniem/pidcatrs/commit/a14eb9)) Code refactoring
- ([e52aea](https://github.com/abdalmoniem/pidcatrs/commit/e52aea)) **(xtask)** Code refactoring
- ([de1a42](https://github.com/abdalmoniem/pidcatrs/commit/de1a42)) **(xtask)** Code refactoring
- ([db6890](https://github.com/abdalmoniem/pidcatrs/commit/db6890)) Code refactoring and improvements
- ([6f061e](https://github.com/abdalmoniem/pidcatrs/commit/6f061e)) **(build.rs)** Refactor `build.rs`

## 🎨 Code Style
- ([0bd7dc](https://github.com/abdalmoniem/pidcatrs/commit/0bd7dc)) Fix cargo format issues

## 🛠️ Maintenance
- ([dde6eb](https://github.com/abdalmoniem/pidcatrs/commit/dde6eb)) Add Justfile
- ([12bcce](https://github.com/abdalmoniem/pidcatrs/commit/12bcce)) Use Just to build the executable in github actions
- ([a552e2](https://github.com/abdalmoniem/pidcatrs/commit/a552e2)) **(Justfile)** Remove strip from windows targets
- ([0fc452](https://github.com/abdalmoniem/pidcatrs/commit/0fc452)) **(Justfile)** Invoke inno-setup installer on windows targets
- ([cc43c4](https://github.com/abdalmoniem/pidcatrs/commit/cc43c4)) **(xtask)** Add xtask workspace to manage the build/install system
- ([36ba78](https://github.com/abdalmoniem/pidcatrs/commit/36ba78)) **(build_installer)** Allow ci on non-main branches
- ([291ce1](https://github.com/abdalmoniem/pidcatrs/commit/291ce1)) **(changelogs)** Add CHANGELOG.md
- ([09b9ee](https://github.com/abdalmoniem/pidcatrs/commit/09b9ee)) **(xshell)** Add custom xshell fork
- ([ebd784](https://github.com/abdalmoniem/pidcatrs/commit/ebd784)) **(Justfile)** Update Justfile to use xtask
- ([d2ea77](https://github.com/abdalmoniem/pidcatrs/commit/d2ea77)) **(Justfile)** Change reinstall recipe to use xtask
- ([b70e68](https://github.com/abdalmoniem/pidcatrs/commit/b70e68)) New app icon

**Full Changelog**: [v1.2.0...v1.2.1](https://github.com/abdalmoniem/pidcatrs/compare/v1.2.0...v1.2.1)

---
# [1.2.0] - Wed, 25/Feb/2026
## 🚀 Features
- ([73dced](https://github.com/abdalmoniem/pidcatrs/commit/73dced)) Add shell completions

## 🐛 Bug Fixes
- ([0b4075](https://github.com/abdalmoniem/pidcatrs/commit/0b4075)) Panic due to adb server not started

## ♻️ Refactors
- ([c64875](https://github.com/abdalmoniem/pidcatrs/commit/c64875)) Renamed makefile.toml to Makefile.toml

## 🛠️ Maintenance
- ([cf4054](https://github.com/abdalmoniem/pidcatrs/commit/cf4054)) Add helix editor config
- ([c1c6cf](https://github.com/abdalmoniem/pidcatrs/commit/c1c6cf)) Add git-cliff configuration

**Full Changelog**: [v1.1.3...v1.2.0](https://github.com/abdalmoniem/pidcatrs/compare/v1.1.3...v1.2.0)

---
# [1.1.3] - Fri, 23/Jan/2026
## 🚀 Features
- ([aa53f3](https://github.com/abdalmoniem/pidcatrs/commit/aa53f3)) Re-introduce reading logs from file

## 🐛 Bug Fixes
- ([ab55c6](https://github.com/abdalmoniem/pidcatrs/commit/ab55c6)) Fixed ansi color codes showing in output files
- ([62c417](https://github.com/abdalmoniem/pidcatrs/commit/62c417)) Writer not using write function from WriterTarget
- ([5ad4c5](https://github.com/abdalmoniem/pidcatrs/commit/5ad4c5)) Fix token color skipping

## ⚡ Performance Improvements
- ([597101](https://github.com/abdalmoniem/pidcatrs/commit/597101)) Optimize `write_log_line` performance

## ♻️ Refactors
- ([607b03](https://github.com/abdalmoniem/pidcatrs/commit/607b03)) Code refactoring and bug fixes
- ([ada9d5](https://github.com/abdalmoniem/pidcatrs/commit/ada9d5)) Code refactoring
- ([a4f288](https://github.com/abdalmoniem/pidcatrs/commit/a4f288)) Code refactoring and bug fixes

## 🛠️ Maintenance
- ([985781](https://github.com/abdalmoniem/pidcatrs/commit/985781)) Use github outputs in publish_release_on_tag.yml
- ([607f5b](https://github.com/abdalmoniem/pidcatrs/commit/607f5b)) Improve build system

**Full Changelog**: [v1.0.0...v1.1.3](https://github.com/abdalmoniem/pidcatrs/compare/v1.0.0...v1.1.3)

---
# [1.0.0] - Sun, 11/Jan/2026
## ♻️ Refactors
- ([40f0e8](https://github.com/abdalmoniem/pidcatrs/commit/40f0e8)) Add custom Result trait
- ([816fac](https://github.com/abdalmoniem/pidcatrs/commit/816fac)) Code refactoring and bug fixes

## 📚 Documentation
- ([6cdf56](https://github.com/abdalmoniem/pidcatrs/commit/6cdf56)) Add LICENSE.md and README.md

## 🛠️ Maintenance
- ([ee43e8](https://github.com/abdalmoniem/pidcatrs/commit/ee43e8)) Integrate github workflows

---
