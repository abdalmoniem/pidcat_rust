// Copyright (C) AbdAlMoniem AlHifnawy <hifnawy_moniem@hotmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![deny(clippy::unwrap_used)]

//! Generates shell completion scripts and patches them for dynamic theme and option values.

use std::io::Write;

use clap::Arg;
use clap::Command;
use clap::ValueHint;

use clap_complete::Shell;
use clap_complete::generate;

use itertools::Itertools;

/// Long flag name used to locate theme completion hooks in generated scripts.
const THEME_FLAG: &str = "--theme";
/// Placeholder replaced with the CLI binary name in embedded script fragments.
const BIN_PLACEHOLDER: &str = "{bin}";

/// Default zsh value action appended to clap-generated specs when no custom completion exists.
const ZSH_DEFAULT_ACTION: &str = ":_default' \\";
/// zsh completion action that delegates theme values to `_{bin}_themes`.
const ZSH_THEME_ACTION: &str = ":_{bin}_themes' \\";
/// zsh function body listing themes via `{bin} --list-themes` or completing `.toml` paths.
const ZSH_THEME_FUNCTION: &str = r#"(( $+functions[_{bin}_themes] )) ||
_{bin}_themes() {
    if [[ $PREFIX == */* ]]; then
        _files -g '*.toml(-.)'
        return
    fi

    local -a themes expl
    themes=(${(f)"$(_call_program themes {bin} --list-themes 2>/dev/null)"})

    _wanted themes expl 'theme' compadd -a themes
}
"#;

/// Default bash file completion line replaced for `--theme`.
const BASH_FILE_COMPLETION: &str = r#"COMPREPLY=($(compgen -f "${cur}"))"#;
/// bash completion block for `--theme`: files when the value looks like a path, else theme names.
const BASH_THEME_COMPLETION: &str = r#"if [[ ${cur} == */* ]]; then
    COMPREPLY=($(compgen -f "${cur}"))
else
    COMPREPLY=($(compgen -W "$({bin} --list-themes 2>/dev/null)" -- "${cur}"))
fi"#;

/// fish `-a` argument list suffix for `--theme`: live theme names plus `.toml` file completion.
const FISH_THEME_VALUES: &str =
    r#" -f -a "({bin} --list-themes 2>/dev/null; __fish_complete_suffix .toml)""#;

/// Elvish import line used as an insertion anchor before adding `use os;`.
const ELVISH_STR_IMPORT: &str = "use str;";
/// Elvish import inserted after [`ELVISH_STR_IMPORT`].
const ELVISH_OS_IMPORT: &str = "use os;";
/// Elvish variable assignment anchoring insertion of custom completers.
const ELVISH_COMMAND: &str = "var command = '{bin}'";
/// Shared Elvish helpers for file and theme path completion.
const ELVISH_SHARED_COMPLETERS: &str = r#"var complete-files = {|prefix value|
    edit:complete-filename $value | each {|file| edit:complex-candidate $prefix$file[stem] }
}
var complete-themes = {|prefix value|
    if (str:contains $value '/') {
        $complete-files $prefix $value
    } else {
        try {
            {bin} --list-themes 2>$os:dev-null | from-lines | each {|theme| put $prefix$theme }
        } catch { }
    }
}"#;
/// Elvish dispatch logic routing the current word to a value completer by flag name.
const ELVISH_DISPATCH: &str = r#"var value-option = $nil
var value-prefix = ''
var value = $words[-1]
if (has-key $value-completers $words[-2]) {
    set value-option = $words[-2]
} elif (and (str:has-prefix $words[-1] '--') (str:contains $words[-1] '=')) {
    var parts = [(str:split &max=2 '=' $words[-1])]
    if (has-key $value-completers $parts[0]) {
        set value-option = $parts[0]
        set value-prefix = $parts[0]'='
        set value = $parts[1]
    }
}
if (not-eq $value-option $nil) {
    $value-completers[$value-option] $value-prefix $value
    return
}"#;

/// PowerShell `param(...)` line used as the insertion anchor for custom completers.
const POWERSHELL_PARAMS: &str = "param($wordToComplete, $commandAst, $cursorPosition)";
/// Shared PowerShell scriptblocks for file and theme completion.
const POWERSHELL_SHARED_COMPLETERS: &str = r#"$completeFiles = {
    param($prefix, $value)
    [CompletionCompleters]::CompleteFilename($value) | ForEach-Object {
        [CompletionResult]::new($prefix + $_.CompletionText, $_.ListItemText, $_.ResultType, $_.ToolTip)
    }
}
$completeThemes = {
    param($prefix, $value)
    if ($value -match '[\\/]') {
        & $completeFiles $prefix $value
        return
    }
    & '{bin}' --list-themes 2>$null | Where-Object { $_ -like "$value*" } | ForEach-Object {
        [CompletionResult]::new($prefix + $_, $_, [CompletionResultType]::ParameterValue, $_)
    }
}
$valueCompleters = [System.Collections.Generic.Dictionary[string, scriptblock]]::new([System.StringComparer]::Ordinal)"#;
/// PowerShell dispatch logic matching Elvish flag and `--flag=value` handling.
const POWERSHELL_DISPATCH: &str = r#"$valueOption = $null
$valuePrefix = ''
$value = $wordToComplete
$wordStart = $cursorPosition - $wordToComplete.Length
$previousElement = @($commandAst.CommandElements | Where-Object { $_.Extent.EndOffset -lt $wordStart })[-1]
if ($previousElement -and $valueCompleters.ContainsKey($previousElement.Extent.Text)) {
    $valueOption = $previousElement.Extent.Text
} elseif ($wordToComplete -match '^(--[^=]+)=(.*)$' -and $valueCompleters.ContainsKey($Matches[1])) {
    $valueOption = $Matches[1]
    $valuePrefix = $valueOption + '='
    $value = $Matches[2]
}
if ($valueOption) {
    & $valueCompleters[$valueOption] $valuePrefix $value
    return
}"#;

/// How the value of a CLI option is completed in Elvish and PowerShell scripts.
enum ValueCompletion {
    /// Theme name or path to a `.toml` theme file.
    Themes,
    /// Arbitrary file or directory path.
    Files,
    /// Fixed clap enumerated values with optional help text.
    Values(Vec<(String, String)>),
}

/// One CLI option that takes a value and needs custom shell completion.
struct ValueOption {
    /// clap argument id used to name generated completer functions.
    id: String,
    /// Short and long flag strings that map to this completer.
    flags: Vec<String>,
    /// Completion strategy for the option's value.
    completion: ValueCompletion,
}

/// Writes the clap-generated completion script for `shell`, with `--theme` values completed
/// from `<bin> --list-themes` at completion time, so themes added to the themes directory show
/// up without regenerating the script. Elvish and PowerShell scripts also get the possible
/// values and file paths of every other option, which the other shells already complete.
pub fn write_completions(
    shell: Shell,
    cmd: &mut Command,
    out: &mut impl Write,
) -> Result<(), String> {
    let bin = cmd.get_name().to_string();

    let mut generated = Vec::new();
    generate(shell, cmd, &bin, &mut generated);

    let options = value_options(cmd);

    let script = String::from_utf8(generated).map_err(|err| err.to_string())?;

    let script = match shell {
        Shell::Zsh => zsh_with_theme_values(&script, &bin),
        Shell::Bash => bash_with_theme_values(&script, &bin),
        Shell::Fish => fish_with_theme_values(&script, &bin),
        Shell::Elvish => elvish_with_values(&script, &bin, &options),
        Shell::PowerShell => powershell_with_values(&script, &bin, &options),
        _ => Some(script),
    }
    .ok_or_else(|| {
        format!("cannot add option value completions: the generated {shell} completion script has an unexpected layout")
    })?;

    out.write_all(script.as_bytes())
        .map_err(|err| err.to_string())
}

/// Substitutes [`BIN_PLACEHOLDER`] in `template` with `bin`.
fn with_bin(template: &str, bin: &str) -> String {
    template.replace(BIN_PLACEHOLDER, bin)
}

/// Joins lines with trailing newlines into one script string.
fn join_lines(lines: Vec<String>) -> String {
    lines.into_iter().map(|line| format!("{line}\n")).collect()
}

/// Patches a zsh completion script to add theme completion function and spec action.
fn zsh_with_theme_values(script: &str, bin: &str) -> Option<String> {
    let theme_spec = format!("'{THEME_FLAG}=[");
    let dispatch = format!("if [ \"$funcstack[1]\" = \"_{bin}\" ]; then");
    let theme_action = with_bin(ZSH_THEME_ACTION, bin);

    let mut has_spec = false;
    let mut has_dispatch = false;
    let mut lines = Vec::new();

    for line in script.lines() {
        if line == dispatch {
            has_dispatch = true;
            lines.push(with_bin(ZSH_THEME_FUNCTION, bin));
        }

        match line.starts_with(&theme_spec) && line.ends_with(ZSH_DEFAULT_ACTION) {
            true => {
                has_spec = true;
                let spec = &line[..line.len() - ZSH_DEFAULT_ACTION.len()];
                lines.push(format!("{spec}{theme_action}"));
            }
            false => lines.push(line.to_string()),
        }
    }

    (has_spec && has_dispatch).then(|| join_lines(lines))
}

/// Patches a bash completion script to replace default file completion for `--theme`.
fn bash_with_theme_values(script: &str, bin: &str) -> Option<String> {
    let theme_case = format!("{THEME_FLAG})");
    let theme_completion = with_bin(BASH_THEME_COMPLETION, bin);

    let mut replaced = false;
    let mut after_theme_case = false;
    let mut lines = Vec::new();

    for line in script.lines() {
        let trimmed = line.trim();

        match after_theme_case && trimmed == BASH_FILE_COMPLETION {
            true => {
                replaced = true;
                let indent = &line[..line.len() - line.trim_start().len()];
                theme_completion
                    .lines()
                    .for_each(|completion| lines.push(format!("{indent}{completion}")));
            }
            false => lines.push(line.to_string()),
        }

        after_theme_case = trimmed == theme_case;
    }

    replaced.then(|| join_lines(lines))
}

/// Inserts each `(anchor, block)` block after the first line equal to its trimmed anchor,
/// indented like that line, or `None` when an anchor is missing.
fn insert_after_lines(script: &str, insertions: &[(&str, &str)]) -> Option<String> {
    let mut inserted = vec![false; insertions.len()];
    let mut lines = Vec::new();

    for line in script.lines() {
        lines.push(line.to_string());

        let indent = &line[..line.len() - line.trim_start().len()];

        for (index, (anchor, block)) in insertions.iter().enumerate() {
            if !inserted[index] && line.trim() == *anchor {
                inserted[index] = true;
                block
                    .lines()
                    .for_each(|block_line| lines.push(format!("{indent}{block_line}")));
            }
        }
    }

    inserted
        .into_iter()
        .all(|done| done)
        .then(|| join_lines(lines))
}

/// Collects every visible clap option that takes a value and supports custom completion.
fn value_options(cmd: &Command) -> Vec<ValueOption> {
    cmd.get_arguments()
        .filter(|arg| !arg.is_positional() && !arg.is_hide_set() && arg.get_action().takes_values())
        .filter_map(|arg| {
            let completion = value_completion(arg)?;

            let shorts = arg
                .get_short_and_visible_aliases()
                .unwrap_or_default()
                .into_iter()
                .map(|short| format!("-{short}"));

            let longs = arg
                .get_long_and_visible_aliases()
                .unwrap_or_default()
                .into_iter()
                .map(|long| format!("--{long}"));

            Some(ValueOption {
                id: arg.get_id().to_string(),
                flags: shorts.chain(longs).collect(),
                completion,
            })
        })
        .collect()
}

/// Derives the completion kind for one clap [`Arg`], if any.
fn value_completion(arg: &Arg) -> Option<ValueCompletion> {
    if arg.get_long() == Some(THEME_FLAG.trim_start_matches('-')) {
        return Some(ValueCompletion::Themes);
    }

    let values = arg
        .get_possible_values()
        .into_iter()
        .filter(|value| !value.is_hide_set())
        .map(|value| {
            let name = value.get_name().to_string();
            let help = value
                .get_help()
                .map(|help| help.to_string().split_whitespace().join(" "))
                .unwrap_or_default();

            (name, help)
        })
        .collect::<Vec<_>>();

    if !values.is_empty() {
        return Some(ValueCompletion::Values(values));
    }

    match arg.get_value_hint() {
        ValueHint::AnyPath
        | ValueHint::FilePath
        | ValueHint::DirPath
        | ValueHint::ExecutablePath => Some(ValueCompletion::Files),
        _ => None,
    }
}

/// Quotes `text` as a single-quoted Elvish or PowerShell string.
fn single_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// Builds the Elvish value-completer definitions and dispatch table for `options`.
fn elvish_value_block(bin: &str, options: &[ValueOption]) -> String {
    let completers = options.iter().map(|option| {
        let body = match &option.completion {
            ValueCompletion::Themes => "    $complete-themes $prefix $value\n".to_string(),
            ValueCompletion::Files => "    $complete-files $prefix $value\n".to_string(),
            ValueCompletion::Values(values) => values
                .iter()
                .map(|(name, help)| {
                    format!(
                        "    cand $prefix{name} {help}\n",
                        name = single_quoted(name),
                        help = single_quoted(help)
                    )
                })
                .collect(),
        };

        format!(
            "var complete-{id} = {{|prefix value|\n{body}}}\n",
            id = option.id
        )
    });

    let entries = options.iter().flat_map(|option| {
        option.flags.iter().map(|flag| {
            format!(
                "    &{flag}= $complete-{id}\n",
                flag = single_quoted(flag),
                id = option.id
            )
        })
    });

    format!(
        "{shared}\n{completers}var value-completers = [\n{entries}]\n{dispatch}",
        shared = with_bin(ELVISH_SHARED_COMPLETERS, bin),
        completers = completers.collect::<String>(),
        entries = entries.collect::<String>(),
        dispatch = ELVISH_DISPATCH,
    )
}

/// Builds the PowerShell value-completer scriptblocks and dispatch for `options`.
fn powershell_value_block(bin: &str, options: &[ValueOption]) -> String {
    let completers = options.iter().map(|option| {
        let body = match &option.completion {
            ValueCompletion::Themes => "    & $completeThemes $prefix $value\n".to_string(),
            ValueCompletion::Files => "    & $completeFiles $prefix $value\n".to_string(),
            ValueCompletion::Values(values) => {
                let results = values
                    .iter()
                    .map(|(name, help)| {
                        let tooltip = match help.is_empty() {
                            true => name,
                            false => help,
                        };

                        format!(
                            "        [CompletionResult]::new($prefix + {name}, {name}, [CompletionResultType]::ParameterValue, {tooltip})\n",
                            name = single_quoted(name),
                            tooltip = single_quoted(tooltip)
                        )
                    })
                    .collect::<String>();

                format!("    @(\n{results}    ) | Where-Object {{ $_.ListItemText -like \"$value*\" }}\n")
            }
        };

        format!(
            "$complete_{id} = {{\n    param($prefix, $value)\n{body}}}\n",
            id = option.id
        )
    });

    let entries = options.iter().flat_map(|option| {
        option.flags.iter().map(|flag| {
            format!(
                "$valueCompleters[{flag}] = $complete_{id}\n",
                flag = single_quoted(flag),
                id = option.id
            )
        })
    });

    format!(
        "{shared}\n{completers}{entries}{dispatch}",
        shared = with_bin(POWERSHELL_SHARED_COMPLETERS, bin),
        completers = completers.collect::<String>(),
        entries = entries.collect::<String>(),
        dispatch = POWERSHELL_DISPATCH,
    )
}

/// Patches an Elvish completion script with shared and per-option value completers.
fn elvish_with_values(script: &str, bin: &str, options: &[ValueOption]) -> Option<String> {
    let command = with_bin(ELVISH_COMMAND, bin);
    let values = elvish_value_block(bin, options);

    insert_after_lines(
        script,
        &[(ELVISH_STR_IMPORT, ELVISH_OS_IMPORT), (&command, &values)],
    )
}

/// Patches a PowerShell completion script with shared and per-option value completers.
fn powershell_with_values(script: &str, bin: &str, options: &[ValueOption]) -> Option<String> {
    let values = powershell_value_block(bin, options);

    insert_after_lines(script, &[(POWERSHELL_PARAMS, &values)])
}

/// Patches a fish completion script to add dynamic theme names and `.toml` suffix completion.
fn fish_with_theme_values(script: &str, bin: &str) -> Option<String> {
    let theme_option = format!(
        "complete -c {bin} -l {} ",
        THEME_FLAG.trim_start_matches('-')
    );
    let theme_values = with_bin(FISH_THEME_VALUES, bin);

    let mut replaced = false;
    let mut lines = Vec::new();

    for line in script.lines() {
        match line.starts_with(&theme_option) && line.ends_with(" -r") {
            true => {
                replaced = true;
                lines.push(format!("{line}{theme_values}"));
            }
            false => lines.push(line.to_string()),
        }
    }

    replaced.then(|| join_lines(lines))
}
