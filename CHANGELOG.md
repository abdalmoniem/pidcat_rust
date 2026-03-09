
# [1.2.1] - Mon, 09/Mar/2026

## ♻️ Refactors
- Code refactoring
- **(xtask)** Code refactoring
- **(xtask)** Code refactoring
- Code refactoring and improvements
- **(build.rs)** Refactor `build.rs`

## 🎨 Code Style
- Fix cargo format issues

## 🛠️ Maintenance
- Add Justfile
- Use Just to build the executable in github actions
- **(Justfile)** Remove strip from windows targets
- **(Justfile)** Invoke inno-setup installer on windows targets
- **(xtask)** Add xtask workspace to manage the build/install system
- **(build_installer)** Allow ci on non-main branches
- **(changelogs)** Add CHANGELOG.md
- **(xshell)** Add custom xshell fork
- **(Justfile)** Update Justfile to use xtask
- **(Justfile)** Change reinstall recipe to use xtask
- New app icon
---
# [1.2.0] - Wed, 25/Feb/2026

## 🚀 Features
- Add shell completions

## 🐛 Bug Fixes
- Panic due to adb server not started

## ♻️ Refactors
- Renamed makefile.toml to Makefile.toml

## 🛠️ Maintenance
- Add helix editor config
- Add git-cliff configuration
---
# [1.1.3] - Fri, 23/Jan/2026

## 🚀 Features
- Re-introduce reading logs from file

## 🐛 Bug Fixes
- Fixed ansi color codes showing in output files
- Writer not using write function from WriterTarget
- Fix token color skipping

## ⚡ Performance Improvements
- Optimize `write_log_line` performance

## ♻️ Refactors
- Code refactoring and bug fixes
- Code refactoring
- Code refactoring and bug fixes

## 🛠️ Maintenance
- Use github outputs in publish_release_on_tag.yml
- Improve build system
---
# [1.0.0] - Sun, 11/Jan/2026

## ♻️ Refactors
- Add custom Result trait
- Code refactoring and bug fixes

## 📚 Documentation
- Add LICENSE.md and README.md

## 🛠️ Maintenance
- Integrate github workflows
---