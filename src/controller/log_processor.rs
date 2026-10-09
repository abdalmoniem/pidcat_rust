#![deny(clippy::unwrap_used)]

use std::collections::HashMap;
use std::sync::Mutex;

use colored::Color;
use colored::Colorize;
use lazy_static::lazy_static;
use regex::Regex;
use regex::RegexBuilder;
use strip_ansi_escapes::strip;

use crate::AnsiSegment;
use crate::CliArgs;
use crate::LogEntry;
use crate::LogEntryKind;
use crate::LogLevel;
use crate::State;
use crate::ValueOrPanic;
use crate::Writer;
use crate::active_theme;
use crate::exit_with_error;
use crate::is_ignored_tag;
use crate::model::timestamp::format_log_timestamp;
use crate::model::timestamp::timestamp_from_log_line;
use crate::passes_log_level;
use crate::passes_package_ownership;
use crate::passes_tag_filter;

lazy_static! {
    /// ELLIPSIS is a unicode ellipsis character.
    /// It is used to represent truncated lines.
    pub static ref ELLIPSIS: String = String::from("…");

    /// ELLIPSIS_COUNT is the number of characters in [ELLIPSIS]
    /// It is used to represent truncated lines.
    pub static ref ELLIPSIS_COUNT: usize = ELLIPSIS.chars().count();

    static ref BACKTRACE_LINE: Regex =
        Regex::new(r"^#(.*?)pc\s(.*?)$").unwrap_or_panic("Invalid Regex for BACKTRACE_LINE");

    static ref NATIVE_TAGS_LINE: Regex =
        Regex::new(r".*nativeGetEnabledTags.*").unwrap_or_panic("Invalid Regex for NATIVE_TAGS_LINE");

    static ref PID_START: Regex =
        Regex::new(r"^.*: Start proc (\d+):([a-zA-Z0-9._:]+)/[a-z0-9]+ for .*? \{(.*?)\}$")
            .unwrap_or_panic("Invalid Regex for PID_START");

    static ref PID_START_UGID: Regex =
        Regex::new(r"^.*: Start proc ([a-zA-Z0-9._:]+) for ([a-z]+ [^:]+): pid=(\d+) uid=(\d+) gids=(.*)$")
            .unwrap_or_panic("Invalid Regex for PID_START_UGID");

    static ref PID_START_DALVIK: Regex =
        Regex::new(r"^E/dalvikvm\(\s*(\d+)\): >>>>> ([a-zA-Z0-9._:]+) \[ userId:0 \| appId:(\d+) \]$")
            .unwrap_or_panic("Invalid Regex for PID_START_DALVIK");

    static ref PID_KILL: Regex =
        Regex::new(r"^Killing (\d+):([a-zA-Z0-9._:]+)/[^:]+: (.*)$")
            .unwrap_or_panic("Invalid Regex for PID_KILL");

    static ref PID_LEAVE: Regex =
        Regex::new(r"^No longer want ([a-zA-Z0-9._:]+) \(pid (\d+)\): .*$")
            .unwrap_or_panic("Invalid Regex for PID_LEAVE");

    static ref PID_DEATH: Regex =
        Regex::new(r"^Process ([a-zA-Z0-9._:]+) \(pid (\d+)\) has died.*$")
            .unwrap_or_panic("Invalid Regex for PID_DEATH");

    static ref STRICT_MODE: Regex =
        Regex::new(r"^(StrictMode policy violation)(; ~duration=)(\d+ ms)")
            .unwrap_or_panic("Invalid Regex for STRICT_MODE");

    static ref GC_COLOR: Regex = Regex::new(
        r"^(GC_(?:CONCURRENT|FOR_M?ALLOC|EXTERNAL_ALLOC|EXPLICIT) )(freed <?\d+.)(, \d+\% free \d+./\d+., )(paused \d+ms(?:\+\d+ms)?)"
    )
    .unwrap_or_panic("Invalid Regex for GC_COLOR");

    static ref REGEX_CACHE: Mutex<HashMap<String, Option<Regex>>> = Mutex::new(HashMap::new());

    pub static ref SYSTEM_TAGS: &'static [&'static str] = &[
        r"Tile",
        r"HWUI",
        r"skia",
        r"libc",
        r"libEGL",
        r"Dialog",
        r"System",
        r"Surface",
        r"OneTrace",
        r"PreCache",
        r"PlayCore",
        r"BpBinder",
        r"VRI\[.*?\]",
        r"AudioTrack",
        r"ImeTracker",
        r"cutils-dev",
        r"JavaBinder",
        r"FrameEvents",
        r"QualityInfo",
        r"ViewExtract",
        r"FirebaseApp",
        r"AdrenoUtils",
        r"ViewRootImpl",
        r"nativeloader",
        r"WindowManager",
        r"OverlayHandler",
        r"ActivityThread",
        r"SurfaceControl",
        r"\[UAH_CLIENT\]",
        r"DisplayManager",
        r"AdrenoGLES-.*?",
        r"VelocityTracker",
        r"OplusBracketLog",
        r"PipelineWatcher",
        r"AppWidgetManager",
        r"BLASTBufferQueue",
        r"InsetsController",
        r"FirebaseSessions",
        r"ProfileInstaller",
        r"ExtensionsLoader",
        r"SurfaceSyncGroup",
        r"DesktopModeFlags",
        r"AppCompatDelegate",
        r"AppWidgetProvider",
        r"AppWidgetHostView",
        r"ApplicationLoaders",
        r"OplusGraphicsEvent",
        r"OplusAppHeapManager",
        r"FirebaseCrashlytics",
        r"ViewRootImplExtImpl",
        r"BufferQueueConsumer",
        r"BufferQueueProducer",
        r"OplusCursorFeedback",
        r"FirebaseInitProvider",
        r"OplusActivityManager",
        r"CompatChangeReporter",
        r"SessionsDependencies",
        r"OplusInputMethodUtil",
        r"BufferPoolAccessor.*?",
        r"OplusViewDebugManager",
        r"WindowOnBackDispatcher",
        r"CompactWindowAppManager",
        r"OplusScrollToTopManager",
        r"ResourcesManagerExtImpl",
        r"ScrollOptimizationHelper",
        r"OplusActivityThreadExtImpl",
        r"DynamicFramerate\s*\[.*?\]",
        r"OplusViewDragTouchViewHelper",
        r"OplusPredictiveBackController",
        r"OplusSystemUINavigationGesture",
        r"OplusInputMethodManagerInternal",
        r"OplusCustomizeRestrictionManager",
        r"oplus\.android\.OplusFrameworkFactoryImpl",
    ];
}

pub fn get_started_process(line: &str) -> Option<(String, String, String, String, String)> {
    if let Some(caps) = PID_START.captures(line) {
        return Some((
            caps[1usize].to_string(),
            String::default(),
            String::default(),
            caps[2usize].to_string(),
            caps[3usize].to_string(),
        ));
    }

    if let Some(caps) = PID_START_UGID.captures(line) {
        return Some((
            caps[3usize].to_string(),
            caps[4usize].to_string(),
            caps[5usize].to_string(),
            caps[1usize].to_string(),
            caps[2usize].to_string(),
        ));
    }

    if let Some(caps) = PID_START_DALVIK.captures(line) {
        return Some((
            caps[1usize].to_string(),
            caps[3usize].to_string(),
            String::default(),
            caps[2usize].to_string(),
            String::default(),
        ));
    }

    None
}

pub fn get_dead_process(line: &str) -> Option<(String, String)> {
    if let Some(caps) = PID_KILL.captures(line) {
        let pid = caps[1usize].to_string();
        let package_line = caps[2usize].to_string();

        return Some((pid, package_line));
    }

    if let Some(caps) = PID_LEAVE.captures(line) {
        let package_line = caps[1usize].to_string();
        let pid = caps[2usize].to_string();

        return Some((pid, package_line));
    }

    if let Some(caps) = PID_DEATH.captures(line) {
        let package_line = caps[1usize].to_string();
        let pid = caps[2usize].to_string();

        return Some((pid, package_line));
    }

    None
}

pub fn is_matching_package(
    token: &String,
    named_processes: &[String],
    catchall_package: &[String],
) -> bool {
    if catchall_package.is_empty() && named_processes.is_empty() {
        return true;
    }

    if named_processes.contains(token) {
        return true;
    }

    match token.find(':') {
        None => catchall_package.contains(token),
        Some(index) => catchall_package.contains(&token[..index].to_string()),
    }
}

pub fn is_matching_tag(tag: &str, tags: &[String]) -> bool {
    let regex_chars = r".*+?[]{}()|\^$";

    for m_tag in tags.iter().map(|tag| tag.trim()) {
        let is_regex = m_tag.chars().any(|char| regex_chars.contains(char));

        if is_regex {
            let pattern = if m_tag.starts_with('^') {
                m_tag
            } else {
                &format!("^{m_tag}")
            };

            let mut cache = REGEX_CACHE
                .lock()
                .unwrap_or_panic("Failed to lock regex cache");
            let re_opt = cache.entry(pattern.to_string()).or_insert_with(|| {
                RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .ok()
            });

            match re_opt {
                Some(re) if re.is_match(tag) => return true,
                _ => continue,
            }
        } else if tag
            .to_ascii_lowercase()
            .contains(&m_tag.to_ascii_lowercase())
        {
            return true;
        }
    }

    false
}

pub fn level_color(level: LogLevel) -> (Color, Color) {
    let colors = &active_theme().log;
    let level_background = match level {
        LogLevel::VERBOSE => colors.verbose,
        LogLevel::DEBUG => colors.debug,
        LogLevel::INFO => colors.info,
        LogLevel::WARN => colors.warn,
        LogLevel::ERROR => colors.error,
        LogLevel::FATAL => colors.fatal,
    };

    (colors.level_fg.into(), level_background.into())
}

pub fn process_line(line: &str, state: &mut State, args: &CliArgs) -> Option<LogEntry> {
    if NATIVE_TAGS_LINE.is_match(line) {
        return None;
    }

    if let Some(procs) = get_started_process(line) {
        let (started_pid, started_uid, started_gids, started_package, started_target) = procs;

        if is_matching_package(
            &started_package,
            &state.named_processes,
            &state.catchall_packages,
        ) || args.tui_mode
        {
            state
                .pids_map
                .insert(started_pid.clone(), started_package.clone());
            state.app_pid = Some(started_pid.clone());

            if !started_uid.is_empty() {
                state
                    .uids_map
                    .insert(started_uid.clone(), started_package.clone());
            }

            state.last_tag = None;

            let banner_text = format!(
                "Process {started_package} created for {started_target}\nPID: {started_pid}   UID: {started_uid}   GIDs: {started_gids}",
                started_package = started_package,
                started_target = started_target,
                started_pid = started_pid,
                started_uid = started_uid,
                started_gids = started_gids,
            );

            return Some(LogEntry {
                kind: LogEntryKind::ProcessStart,
                timestamp: timestamp_from_log_line(args, line),
                pid: started_pid,
                uid: started_uid.clone(),
                owner: started_uid,
                package: started_package,
                tag: started_gids,
                level: LogLevel::INFO,
                message: started_target,
                banner_text,
                raw: raw_line(line),
            });
        }

        return None;
    }

    let log_line_regex = args.log_format.regex();

    let log_line = log_line_regex.captures(line)?;

    let pid = log_line
        .get(
            args.log_format
                .pid_index()
                .unwrap_or_panic("log format pid index is not set"),
        )
        .map_or(String::default(), |mat| mat.as_str().to_string())
        .trim()
        .to_string();

    let tag = log_line
        .get(
            args.log_format
                .tag_index()
                .unwrap_or_panic("log format tag index is not set"),
        )
        .map_or(String::default(), |mat| mat.as_str().to_string())
        .trim()
        .to_string();

    let level = log_line
        .get(
            args.log_format
                .level_index()
                .unwrap_or_panic("log format level index is not set"),
        )
        .map_or(LogLevel::default(), |mat| LogLevel::from(mat.as_str()));

    let mut message = log_line
        .get(
            args.log_format
                .msg_index()
                .unwrap_or_panic("log format msg index is not set"),
        )
        .map_or(String::default(), |mat| mat.as_str().to_string())
        .trim()
        .to_string();

    if let Some((dead_pid, dead_process_name)) = get_dead_process(&message) {
        if state.pids_map.contains_key(&dead_pid) {
            state.pids_map.remove(&dead_pid);
        }

        state.last_tag = None;

        let banner_text = format!(
            "Process {dead_process_name} (PID: {dead_pid}) ended",
            dead_process_name = dead_process_name,
            dead_pid = dead_pid,
        );

        return Some(LogEntry {
            kind: LogEntryKind::ProcessDeath,
            timestamp: timestamp_from_log_line(args, line),
            pid: dead_pid.clone(),
            uid: String::default(),
            owner: dead_pid,
            package: dead_process_name.clone(),
            tag: String::default(),
            level: LogLevel::INFO,
            message: dead_process_name,
            banner_text,
            raw: raw_line(line),
        });
    }

    let uid = log_line
        .get(
            args.log_format
                .uid_index()
                .unwrap_or_panic("log format uid index is not set"),
        )
        .map_or(String::default(), |mat| mat.as_str().to_string())
        .trim()
        .to_string();

    let identify_by_uid = !uid.is_empty() && state.uids_map.contains_key(&uid);
    let owner = match identify_by_uid {
        true => uid.clone(),
        false => pid.clone(),
    };
    let package_map = match identify_by_uid {
        true => &state.uids_map,
        false => &state.pids_map,
    };

    if !args.tui_mode && !passes_package_ownership(&owner, package_map, args.all) {
        return None;
    }

    if !args.tui_mode && !passes_log_level(level, state.log_level) {
        return None;
    }

    if let Some(ignore_tag) = &args.ignore_tag
        && is_ignored_tag(&tag, ignore_tag)
    {
        return None;
    }

    if !args.tui_mode
        && let Some(tag_args) = &args.tag
        && !passes_tag_filter(&tag, tag_args)
    {
        return None;
    }

    if tag == "DEBUG"
        && get_dead_process(message.trim_start()).is_none()
        && BACKTRACE_LINE.captures(message.trim_start()).is_some()
    {
        message = message.trim_start().to_string();
    }

    let package = package_map.get(&owner).cloned().unwrap_or_default();

    Some(LogEntry {
        kind: LogEntryKind::Normal,
        timestamp: timestamp_from_log_line(args, line),
        pid,
        uid,
        owner,
        package,
        tag,
        level,
        message,
        banner_text: String::default(),
        raw: raw_line(line),
    })
}

fn raw_line(line: &str) -> String {
    line.trim_end_matches(['\r', '\n']).to_string()
}

pub fn render_entry(entry: &LogEntry, state: &mut State, args: &CliArgs, writers: &mut [Writer]) {
    match entry.kind {
        LogEntryKind::ProcessStart => render_process_start(entry, state, args, writers),
        LogEntryKind::ProcessDeath => render_process_death(entry, state, args, writers),
        LogEntryKind::Normal => render_normal_entry(entry, state, args, writers),
    }
}

/// Render one entry exactly like plain mode and return its output lines.
pub fn render_entry_lines(
    entry: &LogEntry,
    state: &mut State,
    args: &CliArgs,
    width: i16,
) -> Vec<String> {
    let mut writers = vec![Writer::new_buffer(width, !args.no_color)];
    render_entry(entry, state, args, &mut writers);
    split_rendered_lines(&writers.remove(0).take_buffer())
}

fn split_rendered_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::default();
    }

    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }

    lines
}

pub fn write_log_line(line: &str, state: &mut State, args: &CliArgs, writers: &mut [Writer]) {
    if let Some(entry) = process_line(line, state, args) {
        render_entry(&entry, state, args, writers);
        writers.iter_mut().for_each(Writer::flush);
    }
}

pub fn format_process_start_messages(
    entry: &LogEntry,
    args: &CliArgs,
    colorize: bool,
) -> (String, String) {
    let puid_width = args.puid_width as usize;

    let started_package = if !entry.package.is_empty() {
        entry.package.clone()
    } else {
        "-".repeat(puid_width)
    };
    let started_target = if !entry.message.is_empty() {
        entry.message.clone()
    } else {
        "-".repeat(puid_width)
    };
    let started_pid = if !entry.pid.is_empty() {
        entry.pid.clone()
    } else {
        "-".repeat(puid_width)
    };
    let started_uid = if !entry.uid.is_empty() {
        entry.uid.clone()
    } else {
        "-".repeat(puid_width)
    };
    let started_gids = if !entry.tag.is_empty() {
        entry.tag.clone()
    } else {
        "-".repeat(puid_width)
    };

    let highlight: Color = active_theme().log.highlight.into();

    let started_process_msg = if colorize && !args.no_color {
        format!(
            "Process {started_package} created for {started_target}\n",
            started_package = started_package.color(highlight),
            started_target = started_target.color(highlight)
        )
    } else {
        format!("Process {started_package} created for {started_target}\n")
    };

    let pugid_msg = if colorize && !args.no_color {
        format!(
            "PID: {started_pid}   UID: {started_uid}   GIDs: {started_gids}\n",
            started_pid = started_pid.color(highlight),
            started_uid = started_uid.color(highlight),
            started_gids = started_gids.color(highlight)
        )
    } else {
        format!("PID: {started_pid}   UID: {started_uid}   GIDs: {started_gids}\n")
    };

    (started_process_msg, pugid_msg)
}

pub fn format_process_death_message(entry: &LogEntry, args: &CliArgs, colorize: bool) -> String {
    let puid_width = args.puid_width as usize;

    let dead_pid = if !entry.pid.is_empty() {
        entry.pid.clone()
    } else {
        "-".repeat(puid_width)
    };
    let dead_process_name = if !entry.package.is_empty() {
        entry.package.clone()
    } else {
        "-".repeat(puid_width)
    };

    let highlight: Color = active_theme().log.highlight.into();

    if colorize && !args.no_color {
        format!(
            "Process {dead_process_name} (PID: {dead_pid}) ended\n",
            dead_process_name = dead_process_name.color(highlight),
            dead_pid = dead_pid.color(highlight)
        )
    } else {
        format!("Process {dead_process_name} (PID: {dead_pid}) ended\n")
    }
}

fn timestamp_field_width(args: &CliArgs) -> usize {
    if args.show_timestamps {
        args.timestamp_width + 1usize
    } else {
        0usize
    }
}

fn formatted_log_timestamp(entry: &LogEntry, args: &CliArgs, width: usize) -> String {
    format_log_timestamp(entry.timestamp, &args.timestamp_format, width).unwrap_or_else(|err| {
        exit_with_error(&err, !args.no_color);
    })
}

/// Log header width used for process banner layout (unchanged when timestamps are enabled).
fn process_banner_layout_header_width(args: &CliArgs) -> usize {
    compute_header_width(args).saturating_sub(timestamp_field_width(args))
}

fn write_banner_line_timestamp(
    entry: &LogEntry,
    args: &CliArgs,
    writers: &mut [Writer],
    banner: Color,
) {
    if !args.show_timestamps {
        return;
    }

    let width = args.timestamp_width;
    let text = formatted_log_timestamp(entry, args, width);
    let timestamp_color: Color = active_theme().log.timestamp.into();
    let display = if args.no_color {
        text
    } else {
        text.color(timestamp_color).to_string()
    };

    write_token(&display, writers, false, 0usize, banner, banner);
    write_token(" ", writers, false, 0usize, banner, banner);
}

fn write_process_banner_with_timestamps(
    rendered: &str,
    entry: &LogEntry,
    args: &CliArgs,
    writers: &mut [Writer],
    banner: Color,
) {
    if rendered.is_empty() {
        return;
    }

    let ends_with_newline = rendered.ends_with('\n');
    let mut lines: Vec<&str> = rendered.split('\n').collect();
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    for (index, line) in lines.iter().enumerate() {
        write_banner_line_timestamp(entry, args, writers, banner);
        write_token(line, writers, false, 0usize, banner, banner);
        let is_last = index == lines.len() - 1usize;
        if !is_last || ends_with_newline {
            write_token("\n", writers, false, 0usize, banner, banner);
        }
    }
}

fn write_timestamp_prefix(
    entry: &LogEntry,
    args: &CliArgs,
    writers: &mut [Writer],
    row_header: &mut usize,
    blank: bool,
) {
    if !args.show_timestamps {
        return;
    }

    let width = args.timestamp_width;
    let text = if blank {
        " ".repeat(width)
    } else {
        formatted_log_timestamp(entry, args, width)
    };
    let timestamp_color: Color = active_theme().log.timestamp.into();
    let display = if args.no_color {
        text
    } else {
        text.color(timestamp_color).to_string()
    };

    *row_header = write_token(
        &display,
        writers,
        false,
        *row_header,
        Color::White,
        Color::Black,
    );
    *row_header = write_token(" ", writers, false, *row_header, Color::White, Color::Black);
}

fn render_process_start_body(entry: &LogEntry, args: &CliArgs, writers: &mut [Writer]) {
    let header_width = process_banner_layout_header_width(args);
    let banner_width = header_width.saturating_sub(1usize);
    let banner: Color = active_theme().log.process_start.into();

    let spaces = " "
        .repeat(banner_width)
        .color(banner)
        .on_color(banner)
        .to_string();

    let (started_process_msg, pugid_msg) = format_process_start_messages(entry, args, true);

    write_token(
        &format!("{spaces}\n{spaces}"),
        writers,
        false,
        header_width,
        banner,
        banner,
    );

    write_token(" ", writers, false, header_width, banner, banner);

    write_token(
        &started_process_msg,
        writers,
        true,
        header_width,
        banner,
        banner,
    );

    write_token(&spaces, writers, false, header_width, banner, banner);

    write_token(" ", writers, false, header_width, banner, banner);

    write_token(&pugid_msg, writers, true, header_width, banner, banner);

    write_token(
        &format!("{spaces}\n"),
        writers,
        false,
        header_width,
        banner,
        banner,
    );
}

fn render_process_start(
    entry: &LogEntry,
    state: &mut State,
    args: &CliArgs,
    writers: &mut [Writer],
) {
    let banner: Color = active_theme().log.process_start.into();

    if args.show_timestamps {
        let width = writers
            .first()
            .and_then(|writer| writer.width)
            .unwrap_or(-1i16);
        let show_colors = writers.first().is_some_and(|writer| writer.show_colors);
        let mut buffer = vec![Writer::new_buffer(width, show_colors)];
        render_process_start_body(entry, args, &mut buffer);
        write_process_banner_with_timestamps(
            &buffer.remove(0).take_buffer(),
            entry,
            args,
            writers,
            banner,
        );
    } else {
        render_process_start_body(entry, args, writers);
    }

    state.last_tag = None;
}

fn render_process_death_body(entry: &LogEntry, args: &CliArgs, writers: &mut [Writer]) {
    let header_width = process_banner_layout_header_width(args);
    let banner_width = header_width.saturating_sub(1usize);
    let banner: Color = active_theme().log.process_death.into();

    let spaces = " "
        .repeat(banner_width)
        .color(banner)
        .on_color(banner)
        .to_string();

    let dead_process_msg = format_process_death_message(entry, args, true);

    write_token(
        &format!("{spaces}\n{spaces}"),
        writers,
        false,
        header_width,
        banner,
        banner,
    );

    write_token(" ", writers, false, header_width, banner, banner);

    write_token(
        &dead_process_msg,
        writers,
        true,
        header_width,
        banner,
        banner,
    );

    write_token(
        &format!("{spaces}\n"),
        writers,
        false,
        header_width,
        banner,
        banner,
    );
}

fn render_process_death(
    entry: &LogEntry,
    state: &mut State,
    args: &CliArgs,
    writers: &mut [Writer],
) {
    let banner: Color = active_theme().log.process_death.into();

    if args.show_timestamps {
        let width = writers
            .first()
            .and_then(|writer| writer.width)
            .unwrap_or(-1i16);
        let show_colors = writers.first().is_some_and(|writer| writer.show_colors);
        let mut buffer = vec![Writer::new_buffer(width, show_colors)];
        render_process_death_body(entry, args, &mut buffer);
        write_process_banner_with_timestamps(
            &buffer.remove(0).take_buffer(),
            entry,
            args,
            writers,
            banner,
        );
    } else {
        render_process_death_body(entry, args, writers);
    }

    state.last_tag = None;
}

fn render_normal_entry(
    entry: &LogEntry,
    state: &mut State,
    args: &CliArgs,
    writers: &mut [Writer],
) {
    let base_header_width = 3usize + 1usize;
    let mut header_width = 0usize;
    let (level_foreground, level_background) = level_color(entry.level);
    let mut message = entry.message.clone();

    write_timestamp_prefix(entry, args, writers, &mut header_width, false);
    header_width += timestamp_field_width(args);

    write_owner(
        state,
        args,
        writers,
        &mut header_width,
        &entry.pid,
        args.show_pid,
        level_foreground,
        level_background,
    );

    write_owner(
        state,
        args,
        writers,
        &mut header_width,
        &entry.uid,
        args.show_uid,
        level_foreground,
        level_background,
    );

    write_package_name(
        &entry.owner,
        args,
        state,
        writers,
        &mut header_width,
        level_foreground,
        level_background,
    );

    write_tag(
        &entry.tag,
        args,
        state,
        writers,
        &mut header_width,
        level_foreground,
        level_background,
    );

    write_log_level(
        entry.level,
        args,
        writers,
        &mut header_width,
        level_foreground,
        level_background,
    );

    header_width += base_header_width;

    message = format_log_message(args, &message);

    write_message(
        entry,
        args,
        &message,
        writers,
        header_width,
        level_foreground,
        level_background,
    );
}

pub fn compute_header_width(args: &CliArgs) -> usize {
    let base_header_width = 3usize + 1usize;
    let mut header_width = timestamp_field_width(args);

    if args.show_pid {
        header_width += args.puid_width as usize + 1usize;
    }

    if args.show_uid {
        header_width += args.puid_width as usize + 1usize;
    }

    if args.show_package {
        header_width += args.package_width as usize + 1usize;
    }

    header_width += base_header_width + args.tag_width as usize + 1usize;
    header_width
}

pub fn get_ansi_segments(text: &str) -> Vec<AnsiSegment> {
    use crate::model::ansi::AnsiToken;
    use crate::model::ansi::tokenize_ansi;

    let mut segments = Vec::default();
    let mut pos = 0usize;

    for token in tokenize_ansi(text) {
        match token {
            AnsiToken::Text(text) => pos += text.chars().count(),
            AnsiToken::Escape(code) => segments.push(AnsiSegment { pos, code }),
        }
    }

    segments
}

pub fn get_active_codes_at_pos(segments: &[AnsiSegment], pos: usize) -> Vec<String> {
    let mut active = Vec::default();

    for seg in segments {
        if seg.pos >= pos {
            break;
        }

        if seg.code.contains("0m") {
            active.clear();
        } else {
            active.push(seg.code.clone());
        }
    }

    active
}

pub fn insert_ansi_codes_in_range(
    plain_text: &str,
    segments: &[AnsiSegment],
    start_pos: usize,
    end_pos: usize,
    active_codes: &[String],
) -> String {
    let mut result = String::default();
    let chars: Vec<char> = plain_text.chars().collect();

    for code in active_codes {
        result.push_str(code);
    }

    let mut segment_idx = 0usize;

    while segment_idx < segments.len() && segments[segment_idx].pos < start_pos {
        segment_idx += 1usize;
    }

    for (index, char) in chars.iter().enumerate() {
        let absolute_pos = start_pos + index;

        while segment_idx < segments.len() {
            let seg = &segments[segment_idx];

            if seg.pos >= end_pos {
                break;
            }

            if seg.pos == absolute_pos {
                result.push_str(&seg.code);
                segment_idx += 1usize;
            } else if seg.pos > absolute_pos {
                break;
            } else {
                segment_idx += 1usize;
            }
        }

        result.push(*char);
    }

    result
}

pub fn get_wrapped_indent(
    message: &str,
    show_colors: bool,
    width: i16,
    header_width: usize,
    level_foreground: Color,
    level_background: Color,
) -> String {
    if width == -1i16 {
        return message.to_string();
    }

    let message = message.replace('\t', "    ");
    let wrap_width = (width as usize).saturating_sub(header_width);

    if wrap_width == 0usize {
        return message;
    }

    let message_bytes = message.as_bytes();
    let plain_message_bytes = strip(message_bytes);
    let plain_message = String::from_utf8_lossy(&plain_message_bytes).to_string();

    if plain_message.chars().count() <= wrap_width {
        return message;
    }

    let ansi_segments = get_ansi_segments(&message);
    let chars = plain_message.chars().collect::<Vec<_>>();

    let mut current = 0usize;
    let mut message_buffer = String::default();

    while current < chars.len() {
        let next_index = std::cmp::min(current + wrap_width, chars.len());
        let segment: String = chars[current..next_index].iter().collect();

        let active_codes = if current > 0usize {
            get_active_codes_at_pos(&ansi_segments, current)
        } else {
            Vec::default()
        };

        let colored_segment = insert_ansi_codes_in_range(
            &segment,
            &ansi_segments,
            current,
            next_index,
            &active_codes,
        );
        message_buffer.push_str(&colored_segment);

        if next_index < chars.len() {
            message_buffer.push_str("\x1b[0m");
            message_buffer.push('\n');

            let indent_len = header_width.saturating_sub(4usize);
            let spaces = if level_foreground == level_background && show_colors {
                " ".repeat(indent_len)
                    .color(level_foreground)
                    .on_color(level_background)
                    .to_string()
            } else {
                " ".repeat(indent_len)
            };
            message_buffer.push_str(&spaces);

            let future_index = next_index + wrap_width;
            let is_last_line = future_index >= chars.len();
            let connector = if level_foreground == level_background {
                "   "
            } else if !is_last_line {
                " ╠═"
            } else {
                " ╚═"
            };

            if show_colors {
                let colored_connector = connector
                    .color(level_foreground)
                    .on_color(level_background)
                    .to_string();
                message_buffer.push_str(&colored_connector);
            } else {
                message_buffer.push_str(connector);
            }
            message_buffer.push(' ');
        } else {
            message_buffer.push_str("\x1b[0m");
        }

        current = next_index;
    }

    message_buffer
}

/// Plain mode level field width: `" {L} "` (space, letter, space).
pub const PLAIN_LEVEL_DISPLAY_WIDTH: usize = 3usize;

/// Separator space plain mode writes after the level field via `write_token(" ")`.
pub const PLAIN_MESSAGE_SEPARATOR_WIDTH: usize = 1usize;

/// Level field plus separator — used for wrap/indent math in plain and TUI.
pub const PLAIN_LEVEL_HEADER_WIDTH: usize =
    PLAIN_LEVEL_DISPLAY_WIDTH + PLAIN_MESSAGE_SEPARATOR_WIDTH;

/// Level column text matching plain mode: `" {L} "`.
pub fn format_log_level_field(level: LogLevel) -> String {
    format!(" {level} ")
}

/// Header width used when wrapping inside the TUI message column.
/// Matches plain-mode connector layout without leading header indent.
pub const TUI_MESSAGE_WRAP_HEADER_WIDTH: usize = PLAIN_LEVEL_HEADER_WIDTH;

/// Colored banner padding width in plain mode (`compute_header_width - 1`).
pub fn plain_banner_prefix_width(args: &CliArgs) -> usize {
    compute_header_width(args).saturating_sub(1usize)
}

fn fit_column_label(label: &str, width: usize) -> String {
    let char_count = label.chars().count();
    if char_count <= width {
        return label.to_string();
    }

    let keep = width.saturating_sub(*ELLIPSIS_COUNT);
    if keep == 0 {
        return label.chars().take(width).collect();
    }

    format!(
        "{}{}",
        label.chars().take(keep).collect::<String>(),
        *ELLIPSIS
    )
}

/// Fixed-column segments for the log table top border (`PID`, `UID`, …).
pub fn tui_log_border_columns(args: &CliArgs) -> Vec<(String, usize)> {
    let mut columns = Vec::default();

    if args.show_timestamps {
        let width = args.timestamp_width + 1usize;
        columns.push((fit_column_label("TIME", width), width));
    }

    if args.show_pid {
        let width = args.puid_width as usize + 1usize;
        columns.push((fit_column_label("PID", width), width));
    }
    if args.show_uid {
        let width = args.puid_width as usize + 1usize;
        columns.push((fit_column_label("UID", width), width));
    }
    if args.show_package {
        let width = args.package_width as usize + 1usize;
        columns.push((fit_column_label("PACKAGE", width), width));
    }
    if args.tag_width > 0 {
        let width = args.tag_width as usize + 1usize;
        columns.push((fit_column_label("TAG", width), width));
    }
    columns.push((
        fit_column_label("L", PLAIN_LEVEL_HEADER_WIDTH),
        PLAIN_LEVEL_HEADER_WIDTH,
    ));

    columns
}

/// TUI log panel title — column names aligned like plain-mode output fields.
pub fn tui_column_header(args: &CliArgs) -> String {
    let mut header = String::new();

    if args.show_timestamps {
        let width = args.timestamp_width;
        let label = fit_column_label("TIME", width);
        header.push_str(&format!("{:width$}", label, width = width));
        header.push(' ');
    }

    if args.show_pid {
        let width = args.puid_width as usize;
        let label = fit_column_label("PID", width);
        header.push_str(&format!("{:width$}", label, width = width));
        header.push(' ');
    }

    if args.show_uid {
        let width = args.puid_width as usize;
        let label = fit_column_label("UID", width);
        header.push_str(&format!("{:width$}", label, width = width));
        header.push(' ');
    }

    if args.show_package {
        let width = args.package_width as usize;
        let label = fit_column_label("PACKAGE", width);
        header.push_str(&format!("{:width$}", label, width = width));
        header.push(' ');
    }

    if args.tag_width > 0 {
        let width = args.tag_width as usize;
        let label = fit_column_label("TAG", width);
        let tag_display = if args.show_pid || args.show_uid || args.show_package {
            format!("{:>width$}", label, width = width)
        } else {
            format!("{:width$}", label, width = width)
        };
        header.push_str(&tag_display);
        header.push(' ');
    }

    header.push_str(" L ");
    header.push(' ');

    header
}

/// Prefix column widths for the TUI table — mirrors plain-mode field + separator layout.
pub fn plain_prefix_column_widths(args: &CliArgs) -> Vec<usize> {
    let mut widths = Vec::default();

    if args.show_timestamps {
        widths.push(args.timestamp_width + 1usize);
    }

    if args.show_pid {
        widths.push(args.puid_width as usize + 1usize);
    }
    if args.show_uid {
        widths.push(args.puid_width as usize + 1usize);
    }
    if args.show_package {
        widths.push(args.package_width as usize + 1usize);
    }
    if args.tag_width > 0 {
        widths.push(args.tag_width as usize + 1usize);
    }
    widths.push(PLAIN_LEVEL_DISPLAY_WIDTH);

    widths
}

/// Wrap text the same way as plain mode, adjusting continuation lines for TUI columns.
pub fn wrap_text_for_tui(
    text: &str,
    show_colors: bool,
    width: i16,
    header_width: usize,
    level_foreground: Color,
    level_background: Color,
) -> String {
    let wrapped = get_wrapped_indent(
        text,
        show_colors,
        width,
        header_width,
        level_foreground,
        level_background,
    );

    adjust_wrapped_message_for_tui(&wrapped, header_width)
}

/// Wrap a log message the same way as plain mode, adjusting continuation lines for TUI columns.
pub fn wrap_message_for_tui(
    args: &CliArgs,
    message: &str,
    width: i16,
    header_width: usize,
    level_foreground: Color,
    level_background: Color,
) -> String {
    wrap_text_for_tui(
        &format_log_message(args, message),
        !args.no_color,
        width,
        header_width,
        level_foreground,
        level_background,
    )
}

fn adjust_wrapped_message_for_tui(wrapped: &str, header_width: usize) -> String {
    let indent_len = header_width.saturating_sub(4usize);
    let mut lines = wrapped.split('\n');

    let Some(first) = lines.next() else {
        return String::default();
    };

    let mut result = vec![first.to_string()];
    for line in lines {
        result.push(strip_leading_plain_spaces(line, indent_len));
    }

    result.join("\n")
}

fn strip_leading_plain_spaces(line: &str, count: usize) -> String {
    if count == 0 {
        return line.to_string();
    }

    let plain = strip_ansi_escapes::strip_str(line);
    if plain.chars().take(count).any(|ch| ch != ' ') {
        return line.to_string();
    }

    let mut remaining = count;
    let mut chars = line.chars().peekable();

    while remaining > 0 {
        if chars.peek() == Some(&'\x1b') {
            chars.next();
            for ch in chars.by_ref() {
                if ch.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }

        match chars.next() {
            Some(' ') => remaining -= 1,
            Some(_) | None => return line.to_string(),
        }
    }

    chars.collect()
}

pub fn get_token_color(token: &str, state: &mut State) -> Color {
    if !state.known_tokens.contains_key(token) {
        if !state.token_colors.is_empty() {
            let color = state.token_colors[0usize];
            state.known_tokens.insert(token.to_string(), color);
        } else {
            return Color::BrightWhite;
        }
    }

    let color = *state
        .known_tokens
        .get(token)
        .unwrap_or_panic(&format!("Unknown tag '{token}' in known tags"));

    if let Some(pos) = state.token_colors.iter().position(|&col| col == color) {
        state.token_colors.remove(pos);
        state.token_colors.push(color);
    }

    color
}

fn write_token(
    token: &str,
    writers: &mut [Writer],
    wrap: bool,
    header_width: usize,
    level_foreground: Color,
    level_background: Color,
) -> usize {
    let local_header = header_width;
    for writer in writers.iter_mut() {
        let buffer = if wrap && let Some(width) = writer.width {
            get_wrapped_indent(
                token,
                writer.show_colors,
                width,
                header_width,
                level_foreground,
                level_background,
            )
        } else {
            token.to_string()
        };

        let token = if writer.show_colors {
            buffer.clone()
        } else {
            let buffer_bytes = buffer.as_bytes();
            let plain_buffer_bytes = strip(buffer_bytes);

            String::from_utf8_lossy(&plain_buffer_bytes).to_string()
        };

        writer.write(&token);
    }

    local_header
}

#[allow(clippy::too_many_arguments)]
fn write_owner(
    state: &mut State,
    args: &CliArgs,
    writers: &mut [Writer],
    header_width: &mut usize,
    owner: &str,
    show: bool,
    level_foreground: Color,
    level_background: Color,
) {
    let puid_width = args.puid_width as usize;

    if show && !owner.is_empty() {
        let mut display_owner = owner.to_string();
        let pid_color = get_token_color(owner, state);

        if display_owner.len() > puid_width {
            display_owner.truncate(puid_width - *ELLIPSIS_COUNT);
            display_owner = format!(
                "{display_owner}{ellipsis}",
                display_owner = display_owner,
                ellipsis = *ELLIPSIS
            );
        }

        let pid_display = format!("{:width$}", display_owner, width = puid_width);

        let pid_display = if args.no_color {
            pid_display
        } else {
            pid_display.color(pid_color).to_string()
        };
        *header_width = write_token(
            &pid_display,
            writers,
            false,
            *header_width,
            level_foreground,
            level_background,
        );
        *header_width = write_token(
            " ",
            writers,
            false,
            *header_width,
            level_foreground,
            level_background,
        );
        *header_width += puid_width + 1usize;
    }
}

fn write_package_name(
    owner: &str,
    args: &CliArgs,
    state: &mut State,
    writers: &mut [Writer],
    header_width: &mut usize,
    level_foreground: Color,
    level_background: Color,
) {
    let package_width = args.package_width as usize;

    if args.show_package && !owner.is_empty() {
        let package_name = crate::owner_display_package(owner, state);
        let mut display_pkg = package_name.clone();
        let pkg_color = get_token_color(&package_name, state);

        if display_pkg.len() > package_width {
            display_pkg.truncate(package_width - *ELLIPSIS_COUNT);
            display_pkg = format!(
                "{display_pkg}{ellipsis}",
                display_pkg = display_pkg,
                ellipsis = *ELLIPSIS
            );
        }

        let pkg_display = format!("{:width$}", display_pkg, width = package_width);
        let pkg_display = if args.no_color {
            pkg_display
        } else {
            pkg_display.color(pkg_color).to_string()
        };

        *header_width = write_token(
            &pkg_display,
            writers,
            false,
            *header_width,
            level_foreground,
            level_background,
        );
        *header_width = write_token(
            " ",
            writers,
            false,
            *header_width,
            level_foreground,
            level_background,
        );
        *header_width += package_width + 1usize;
    }
}

fn write_tag(
    tag: &str,
    args: &CliArgs,
    state: &mut State,
    writers: &mut [Writer],
    header_width: &mut usize,
    level_foreground: Color,
    level_background: Color,
) {
    let tag_width = args.tag_width as usize;

    if tag_width > 0usize {
        if Some(tag.to_string()) != state.last_tag || args.always_show_tags {
            state.last_tag = Some(tag.to_string());

            let mut display_tag = tag.to_string();

            if display_tag.len() > tag_width {
                display_tag.truncate(tag_width - *ELLIPSIS_COUNT);
                display_tag = format!(
                    "{display_tag}{ellipsis}",
                    display_tag = display_tag,
                    ellipsis = *ELLIPSIS
                );
            }

            let tag_color = get_token_color(tag, state);
            let tag_display = if args.show_pid || args.show_uid || args.show_package {
                format!("{:>width$}", display_tag, width = tag_width)
            } else {
                format!("{:width$}", display_tag, width = tag_width)
            };

            let tag_display = if args.no_color {
                tag_display
            } else {
                tag_display.color(tag_color).to_string()
            };

            *header_width = write_token(
                &tag_display,
                writers,
                false,
                *header_width,
                level_foreground,
                level_background,
            );
        } else {
            *header_width = write_token(
                &" ".repeat(tag_width),
                writers,
                false,
                *header_width,
                level_foreground,
                level_background,
            );
        }
        *header_width = write_token(
            " ",
            writers,
            false,
            *header_width,
            level_foreground,
            level_background,
        );
        *header_width += tag_width + 1usize;
    }
}

fn write_log_level(
    level: LogLevel,
    args: &CliArgs,
    writers: &mut [Writer],
    header_width: &mut usize,
    level_foreground: Color,
    level_background: Color,
) {
    let mut level_str = format!(" {level} ");

    if !args.no_color {
        level_str = level_str
            .color(level_foreground)
            .on_color(level_background)
            .to_string();
    }

    *header_width = write_token(
        &level_str,
        writers,
        false,
        *header_width,
        level_foreground,
        level_background,
    );
    *header_width = write_token(
        " ",
        writers,
        false,
        *header_width,
        level_foreground,
        level_background,
    );
}

pub fn format_log_message(args: &CliArgs, message: &str) -> String {
    let colors = &active_theme().log;
    let mut message = message.to_string();
    if STRICT_MODE.is_match(&message) {
        message = STRICT_MODE
            .replace(&message, |caps: &regex::Captures| {
                format!(
                    "{message}{duration}{unit}",
                    message = &caps[1usize],
                    duration = caps[2usize].color(Color::from(colors.gc_duration)),
                    unit = caps[3usize].color(Color::from(colors.gc_unit))
                )
            })
            .to_string();
    }

    if args.gc_color && GC_COLOR.is_match(&message) {
        message = GC_COLOR
            .replace(&message, |caps: &regex::Captures| {
                format!(
                    "{freed}{free}{paused}{unit}",
                    freed = &caps[1usize],
                    free = caps[2usize].color(Color::from(colors.gc_free)),
                    paused = &caps[3usize],
                    unit = caps[4usize].color(Color::from(colors.gc_unit))
                )
            })
            .to_string();
    }

    message
}

fn colored_level_padding(
    len: usize,
    show_colors: bool,
    level_foreground: Color,
    level_background: Color,
) -> String {
    if len == 0usize {
        return String::default();
    }

    if level_foreground == level_background && show_colors {
        " ".repeat(len)
            .color(level_foreground)
            .on_color(level_background)
            .to_string()
    } else {
        " ".repeat(len)
    }
}

fn write_message(
    entry: &LogEntry,
    args: &CliArgs,
    message: &str,
    writers: &mut [Writer],
    header_width: usize,
    level_foreground: Color,
    level_background: Color,
) {
    let show_colors = writers.first().is_some_and(|writer| writer.show_colors);
    let width = writers
        .first()
        .and_then(|writer| writer.width)
        .unwrap_or(-1i16);

    let wrapped = if args.show_timestamps && width != -1i16 {
        get_wrapped_indent(
            message,
            show_colors,
            width,
            header_width,
            level_foreground,
            level_background,
        )
    } else {
        String::default()
    };

    if !args.show_timestamps || !wrapped.contains('\n') {
        write_token(
            if wrapped.is_empty() {
                message
            } else {
                &wrapped
            },
            writers,
            !args.show_timestamps,
            header_width,
            level_foreground,
            level_background,
        );
        write_token(
            "\n",
            writers,
            false,
            header_width,
            level_foreground,
            level_background,
        );
        return;
    }

    let indent_len = header_width.saturating_sub(4usize);
    let continuation_pad = header_width
        .saturating_sub(timestamp_field_width(args))
        .saturating_sub(4usize);
    let mut lines = wrapped.split('\n');

    if let Some(first) = lines.next() {
        write_token(
            first,
            writers,
            false,
            header_width,
            level_foreground,
            level_background,
        );
    }

    for line in lines {
        write_token(
            "\n",
            writers,
            false,
            header_width,
            level_foreground,
            level_background,
        );
        let mut row_header = 0usize;
        write_timestamp_prefix(entry, args, writers, &mut row_header, false);
        let suffix = strip_leading_plain_spaces(line, indent_len);
        let padding = colored_level_padding(
            continuation_pad,
            show_colors,
            level_foreground,
            level_background,
        );
        write_token(
            &format!("{padding}{suffix}"),
            writers,
            false,
            header_width,
            level_foreground,
            level_background,
        );
    }

    write_token(
        "\n",
        writers,
        false,
        header_width,
        level_foreground,
        level_background,
    );
}
