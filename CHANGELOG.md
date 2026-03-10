# [1.2.1] - Mon, 09/Mar/2026

## ♻️ Refactors
- (a14eb9) Code refactoring
- (e52aea) **(xtask)** Code refactoring
- (de1a42) **(xtask)** Code refactoring
- (db6890) Code refactoring and improvements
- (6f061e) **(build.rs)** Refactor `build.rs`

## 🎨 Code Style
- (0bd7dc) Fix cargo format issues

## 🛠️ Maintenance
- (dde6eb) Add Justfile
- (12bcce) Use Just to build the executable in github actions
- (a552e2) **(Justfile)** Remove strip from windows targets
- (0fc452) **(Justfile)** Invoke inno-setup installer on windows targets
- (cc43c4) **(xtask)** Add xtask workspace to manage the build/install system
- (36ba78) **(build_installer)** Allow ci on non-main branches
- (291ce1) **(changelogs)** Add CHANGELOG.md
- (09b9ee) **(xshell)** Add custom xshell fork
- (ebd784) **(Justfile)** Update Justfile to use xtask
- (d2ea77) **(Justfile)** Change reinstall recipe to use xtask
- (b70e68) New app icon
---
# [1.2.0] - Wed, 25/Feb/2026

## 🚀 Features
- (73dced) Add shell completions

## 🐛 Bug Fixes
- (0b4075) Panic due to adb server not started

## ♻️ Refactors
- (c64875) Renamed makefile.toml to Makefile.toml

## 🛠️ Maintenance
- (cf4054) Add helix editor config
- (c1c6cf) Add git-cliff configuration
---
# [1.1.3] - Fri, 23/Jan/2026

## 🚀 Features
- (aa53f3) Re-introduce reading logs from file

## 🐛 Bug Fixes
- (ab55c6) Fixed ansi color codes showing in output files
- (62c417) Writer not using write function from WriterTarget
- (5ad4c5) Fix token color skipping

## ⚡ Performance Improvements
- (597101) Optimize `write_log_line` performance

## ♻️ Refactors
- (607b03) Code refactoring and bug fixes
- (ada9d5) Code refactoring
- (a4f288) Code refactoring and bug fixes

## 🛠️ Maintenance
- (985781) Use github outputs in publish_release_on_tag.yml
- (607f5b) Improve build system
---
# [1.0.0] - Sun, 11/Jan/2026

## ♻️ Refactors
- (40f0e8) Add custom Result trait
- (816fac) Code refactoring and bug fixes

## 📚 Documentation
- (6cdf56) Add LICENSE.md and README.md

## 🛠️ Maintenance
- (ee43e8) Integrate github workflows
---
