#![deny(clippy::unwrap_used)]

use colored::Color;
use colored::Colorize;

use itertools::Itertools;

use is_terminal::IsTerminal;

use once_cell::sync::Lazy;

use pidcat::AdbDevice;
use pidcat::AdbState;
use pidcat::AnsiSegment;
use pidcat::CliArgs;
use pidcat::LogLevel;
use pidcat::LogSource;
use pidcat::State;
use pidcat::ValueOrPanic;
use pidcat::Writer;

use regex::Regex;

use std::collections::HashMap;
use std::collections::HashSet;
use std::panic::PanicHookInfo;

use std::fs::File;

use std::io::BufRead;
use std::io::BufReader;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::stdin;

use std::panic;

use std::process::Command;
use std::process::Stdio;
use std::process::exit;
use std::process::id;
use std::sync::Mutex;

use strip_ansi_escapes::strip;

/// ELLIPSIS is a unicode ellipsis character.
/// It is used to represent truncated lines.
static ELLIPSIS: Lazy<&str> = Lazy::new(|| "…");

/// ELLIPSIS_COUNT is the number of characters in [ELLIPSIS]
/// It is used to represent truncated lines.
static ELLIPSIS_COUNT: Lazy<usize> = Lazy::new(|| ELLIPSIS.chars().count());

static BACKTRACE_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^#(.*?)pc\s(.*?)$").unwrap_or_panic("Invalid Regex for BACKTRACE_LINE")
});

static NATIVE_TAGS_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r".*nativeGetEnabledTags.*").unwrap_or_panic("Invalid Regex for NATIVE_TAGS_LINE")
});

static LOG_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([A-Z])/(.+?)\( *(\d+)\): (.*?)$").unwrap_or_panic("Invalid Regex for LOG_LINE")
});

static PID_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\w+\s+(\w+)\s+\w+\s+\w+\s+\w+\s+\w+\s+\w+\s+\w\s(.*?)$")
        .unwrap_or_panic("Invalid Regex for PID_LINE")
});

static PID_START: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^.*: Start proc (\d+):([a-zA-Z0-9._:]+)/[a-z0-9]+ for .*? \{(.*?)\}$")
        .unwrap_or_panic("Invalid Regex for PID_START")
});

static PID_START_UGID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^.*: Start proc ([a-zA-Z0-9._:]+) for ([a-z]+ [^:]+): pid=(\d+) uid=(\d+) gids=(.*)$",
    )
    .unwrap_or_panic("Invalid Regex for PID_START_UGID")
});

static PID_START_DALVIK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^E/dalvikvm\(\s*(\d+)\): >>>>> ([a-zA-Z0-9._:]+) \[ userId:0 \| appId:(\d+) \]$")
        .unwrap_or_panic("Invalid Regex for PID_START_DALVIK")
});

static PID_KILL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^Killing (\d+):([a-zA-Z0-9._:]+)/[^:]+: (.*)$")
        .unwrap_or_panic("Invalid Regex for PID_KILL")
});

static PID_LEAVE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^No longer want ([a-zA-Z0-9._:]+) \(pid (\d+)\): .*$")
        .unwrap_or_panic("Invalid Regex for PID_LEAVE")
});

static PID_DEATH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^Process ([a-zA-Z0-9._:]+) \(pid (\d+)\) has died.?$")
        .unwrap_or_panic("Invalid Regex for PID_DEATH")
});

static STRICT_MODE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(StrictMode policy violation)(; ~duration=)(\d+ ms)")
        .unwrap_or_panic("Invalid Regex for STRICT_MODE")
});

static GC_COLOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(GC_(?:CONCURRENT|FOR_M?ALLOC|EXTERNAL_ALLOC|EXPLICIT) )(freed <?\d+.)(, \d+\% free \d+./\d+., )(paused \d+ms(?:\+\d+ms)?)"
    ).unwrap_or_panic("Invalid Regex for GC_COLOR")
});

static VISIBLE_ACTIVITIES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"VisibleActivityProcess:\[\s*(?:(?:ProcessRecord\{\w+\s*\d+:(?:[a-zA-Z.]+)/\w+\})\s*)+\]",
    )
    .unwrap_or_panic("Invalid Regex for VISIBLE_ACTIVITIES")
});

static VISIBLE_PACKAGES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"ProcessRecord\{\w+\s*\d+:([a-zA-Z.]+)/\w+\}")
        .unwrap_or_panic("Invalid Regex for VISIBLE_PACKAGES")
});

static REGEX_CACHE: Lazy<Mutex<HashMap<String, Option<Regex>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static SYSTEM_TAGS: Lazy<&[&str]> = Lazy::new(|| {
    &[
        r"Tile",
        r"HWUI",
        r"skia",
        r"libc",
        r"libEGL",
        r"Dialog",
        r"System",
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
    ]
});

fn get_console_width() -> i16 {
    terminal_size::terminal_size()
        .map(|(terminal_size::Width(width), _)| width as i16)
        .unwrap_or(80)
}

fn get_ansi_segments(text: &str) -> Vec<AnsiSegment> {
    let mut segments = Vec::default();
    let mut chars = text.chars().peekable();
    let mut visible_pos = 0;

    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            let mut code = String::from("\x1b");
            let cmd = chars
                .next()
                .unwrap_or_panic("Unexpected end of input after ESC");
            code.push(cmd); // '['

            while let Some(&next_ch) = chars.peek() {
                let param = chars
                    .next()
                    .unwrap_or_panic("Unexpected end of input in ANSI code");
                code.push(param);

                if next_ch.is_ascii_alphabetic() {
                    break;
                }
            }

            segments.push(AnsiSegment { visible_pos, code });
        } else {
            visible_pos += 1;
        }
    }

    segments
}

fn get_active_codes_at_pos(segments: &[AnsiSegment], pos: usize) -> Vec<String> {
    let mut active = Vec::default();

    for seg in segments {
        if seg.visible_pos >= pos {
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

fn insert_ansi_codes_in_range(
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

    let mut segment_idx = 0;

    while segment_idx < segments.len() && segments[segment_idx].visible_pos < start_pos {
        segment_idx += 1;
    }

    for (index, char) in chars.iter().enumerate() {
        let absolute_pos = start_pos + index;

        while segment_idx < segments.len() {
            let seg = &segments[segment_idx];

            if seg.visible_pos >= end_pos {
                break;
            }

            if seg.visible_pos == absolute_pos {
                result.push_str(&seg.code);
                segment_idx += 1;
            } else if seg.visible_pos > absolute_pos {
                break;
            } else {
                segment_idx += 1;
            }
        }

        result.push(*char);
    }

    result
}

fn get_wrapped_indent(
    message: &str,
    show_colors: bool,
    width: i16,
    header_width: usize,
    level_foreground: Color,
    level_background: Color,
) -> String {
    if width == -1 {
        return message.to_string();
    }

    let message = message.replace('\t', "    ");
    let wrap_width = (width as usize).saturating_sub(header_width);

    if wrap_width == 0 {
        return message;
    }

    let message_bytes = message.as_bytes();
    let plain_message_bytes = strip(message_bytes);
    let plain_message = String::from_utf8_lossy(&plain_message_bytes).to_string();

    // Check if wrapping is needed
    if plain_message.chars().count() <= wrap_width {
        return message;
    }

    let ansi_segments = get_ansi_segments(&message);
    let chars = plain_message.chars().collect::<Vec<_>>();

    let mut current = 0;
    let mut message_buffer = String::default();

    while current < chars.len() {
        let next_index = std::cmp::min(current + wrap_width, chars.len());
        let segment: String = chars[current..next_index].iter().collect();

        // Get active codes at the start of this segment (for continuation lines)
        let active_codes = if current > 0 {
            get_active_codes_at_pos(&ansi_segments, current)
        } else {
            Vec::default()
        };

        // Reconstruct segment with ANSI codes
        let colored_segment = insert_ansi_codes_in_range(
            &segment,
            &ansi_segments,
            current,
            next_index,
            &active_codes,
        );
        message_buffer.push_str(&colored_segment);

        if next_index < chars.len() {
            // Add reset to prevent color bleeding
            message_buffer.push_str("\x1b[0m");

            message_buffer.push('\n');

            let indent_len = header_width.saturating_sub(5);
            let spaces = if level_foreground == level_background {
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
                "    "
            } else if !is_last_line {
                " ╠═"
            } else {
                " ╚═"
            };

            let colored_connector = connector
                .color(level_foreground)
                .on_color(level_background)
                .to_string();

            if show_colors {
                message_buffer.push_str(&colored_connector);
            } else {
                message_buffer.push_str(connector);
            }
            message_buffer.push(' ');
        } else {
            // Add reset at the end
            message_buffer.push_str("\x1b[0m");
        }

        current = next_index;
    }

    message_buffer
}

fn get_token_color(token: &str, state: &mut State) -> Color {
    if !state.known_tokens.contains_key(token) {
        if !state.token_colors.is_empty() {
            let color = state.token_colors[0];
            state.known_tokens.insert(token.to_string(), color);
            state.token_colors.rotate_left(0);
        } else {
            return Color::White;
        }
    }

    let color = *state
        .known_tokens
        .get(token)
        .unwrap_or_panic(&format!("Unknown tag '{}' in known tags", token));

    // Move to end of list (LRU logic)
    if let Some(pos) = state.token_colors.iter().position(|&col| col == color) {
        state.token_colors.remove(pos);
        state.token_colors.rotate_left(0);
    }
    state.token_colors.push(color);

    color
}

fn get_adb_command(args: &CliArgs) -> Vec<String> {
    let adb_path = args.adb_path.clone().unwrap_or("adb".to_string());
    let mut base_adb_command = vec![adb_path];

    if args.use_device {
        base_adb_command.push("-d".to_string());
    } else if args.use_emulator {
        base_adb_command.push("-e".to_string());
    } else if let Some(device_serial) = &args.device_serial {
        base_adb_command.push("-s".to_string());
        base_adb_command.push(device_serial.clone());
    }

    base_adb_command
}

fn get_adb_devices(base_adb_command: &[String]) -> Option<Vec<AdbDevice>> {
    let output = Command::new(&base_adb_command[0])
        .args(&base_adb_command[1..])
        .arg("devices")
        .output();

    match output {
        Ok(output) => {
            let re = Regex::new(r"\s+").unwrap_or_panic("Invalid Regex");
            let devices = output
                .stdout
                .split(|&byte| byte == b'\n')
                .skip(1)
                .map(|line| String::from_utf8_lossy(line).trim().to_string())
                .filter(|line| !line.is_empty())
                .map(|device| {
                    let (device_id_str, device_state_str) = re
                        .split(&device)
                        .map(|str| str.to_string()) // Convert &str to String
                        .collect_tuple::<(String, String)>()
                        .unwrap_or_panic("Failed to get device id and type");

                    AdbDevice {
                        device_id: device_id_str,
                        device_state: AdbState::from(device_state_str),
                    }
                })
                .collect::<Vec<_>>();

            if !devices.is_empty() {
                Some(devices)
            } else {
                None
            }
        }

        Err(_) => None,
    }
}

fn get_current_app_package(base_adb_command: &[String]) -> Option<Vec<String>> {
    let mut cmd = Command::new(&base_adb_command[0]);
    if base_adb_command.len() > 1 {
        cmd.args(&base_adb_command[1..]);
    }

    let output = cmd
        .args(["shell", "dumpsys", "activity", "activities"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .ok()?;

    let system_dump = String::from_utf8_lossy(&output.stdout);

    let visible_activities = VISIBLE_ACTIVITIES.find(&system_dump)?.as_str();

    let packages: Vec<String> = VISIBLE_PACKAGES
        .captures_iter(visible_activities)
        .filter_map(|cap| cap.get(1).map(|mat| mat.as_str().to_string()))
        .collect();

    if packages.is_empty() {
        None
    } else {
        Some(packages)
    }
}

fn get_processes(
    base_adb_command: &[String],
    catchall_package: &[String],
    args: &CliArgs,
) -> HashMap<String, String> {
    let mut pids_map = HashMap::default();
    let mut cmd = Command::new(&base_adb_command[0]);

    if base_adb_command.len() > 1 {
        cmd.args(&base_adb_command[1..]);
    }

    let output = cmd.args(["shell", "ps"]).stdout(Stdio::piped()).output();

    if let Ok(out) = output {
        let stdout = BufReader::new(&out.stdout[..]);
        for line in stdout.lines().map_while(Result::ok) {
            if let Some(caps) = PID_LINE.captures(&line) {
                let pid = caps
                    .get(1)
                    .map_or(String::default(), |mat| mat.as_str().to_string());
                let process = caps
                    .get(2)
                    .map_or(String::default(), |mat| mat.as_str().to_string());

                let is_target_package = catchall_package.contains(&process);

                if args.all || is_target_package {
                    pids_map.insert(pid, process);
                }
            }
        }
    }

    pids_map
}

fn get_started_process(line: &str) -> Option<(String, String, String, String, String)> {
    if let Some(caps) = PID_START.captures(line) {
        return Some((
            caps[1].to_string(), // started_pid
            String::default(),   // started_uid
            String::default(),   // started_gids
            caps[2].to_string(), // started_package
            caps[3].to_string(), // started_target
        ));
    }

    if let Some(caps) = PID_START_UGID.captures(line) {
        return Some((
            caps[3].to_string(), // started_pid
            caps[4].to_string(), // started_uid
            caps[5].to_string(), // started_gids
            caps[1].to_string(), // started_package
            caps[2].to_string(), // started_target
        ));
    }

    if let Some(caps) = PID_START_DALVIK.captures(line) {
        return Some((
            caps[1].to_string(), // started_pid
            caps[3].to_string(), // started_uid
            String::default(),   // started_gids
            caps[2].to_string(), // started_package
            String::default(),   // started_target
        ));
    }

    None
}

fn get_dead_process(
    tag: &str,
    message: &str,
    pids_set: &HashSet<String>,
    named_processes: &[String],
    catchall_package: &[String],
) -> Option<(String, String)> {
    if tag != "ActivityManager" {
        return None;
    }

    if let Some(caps) = PID_KILL.captures(message) {
        let pid = caps[1].to_string();
        let package_line = caps[2].to_string();
        if is_matching_package(&package_line, named_processes, catchall_package)
            && pids_set.contains(&pid)
        {
            return Some((pid, package_line));
        }
    }

    if let Some(caps) = PID_LEAVE.captures(message) {
        let package_line = caps[1].to_string();
        let pid = caps[2].to_string();
        if is_matching_package(&package_line, named_processes, catchall_package)
            && pids_set.contains(&pid)
        {
            return Some((pid, package_line));
        }
    }

    if let Some(caps) = PID_DEATH.captures(message) {
        let package_line = caps[1].to_string();
        let pid = caps[2].to_string();
        if is_matching_package(&package_line, named_processes, catchall_package)
            && pids_set.contains(&pid)
        {
            return Some((pid, package_line));
        }
    }

    None
}

fn is_matching_package(
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

fn is_matching_tag(tag: &str, tags: &[String]) -> bool {
    let regex_chars = r".*+?[]{}()|\^$";

    for m_tag in tags.iter().map(|tag| tag.trim()) {
        let is_regex = m_tag.chars().any(|char| regex_chars.contains(char));

        if is_regex {
            let pattern = if m_tag.starts_with('^') {
                m_tag
            } else {
                &format!("^{}", m_tag)
            };

            let mut cache = REGEX_CACHE
                .lock()
                .unwrap_or_panic("Failed to lock regex cache");
            let re_opt = cache
                .entry(pattern.to_string())
                .or_insert_with(|| Regex::new(pattern).ok());

            match re_opt {
                Some(re) if re.is_match(tag) => return true,
                _ => continue,
            }
        } else if tag.contains(m_tag) {
            return true;
        }
    }

    false
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
        let buffer = if wrap && writer.width != -1 {
            writer.width = get_console_width();

            get_wrapped_indent(
                token,
                writer.show_colors,
                writer.width,
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
        writer.flush();
    }

    local_header
}

fn write_started_process(
    line: &str,
    state: &mut State,
    writers: &mut [Writer],
    header_width: usize,
) -> bool {
    let spaces = " ".repeat(header_width.saturating_sub(1));

    if let Some(procs) = get_started_process(line) {
        let (started_pid, started_uid, started_gids, started_package, started_target) = procs;

        let spaces = spaces
            .color(Color::Green)
            .on_color(Color::Green)
            .to_string();

        let started_process_message = format!(
            " Process {} created for {}\n",
            &started_package.color(Color::Yellow),
            &started_target.color(Color::Yellow)
        );

        let pugid_message = format!(
            " PID: {}   UID: {}   GIDs: {}",
            &started_pid.color(Color::Yellow),
            &started_uid.color(Color::Yellow),
            &started_gids.color(Color::Yellow)
        );

        if is_matching_package(
            &started_package,
            &state.named_processes,
            &state.catchall_package,
        ) {
            state
                .pids_map
                .insert(started_pid.clone(), started_package.clone());
            state.app_pid = Some(started_pid.clone());

            write_token(
                &spaces,
                writers,
                false,
                header_width,
                Color::Green,
                Color::Green,
            );

            write_token(
                "\n",
                writers,
                false,
                header_width,
                Color::Green,
                Color::Green,
            );

            write_token(
                &spaces,
                writers,
                false,
                header_width,
                Color::Green,
                Color::Green,
            );

            write_token(
                &started_process_message,
                writers,
                true,
                header_width,
                Color::Green,
                Color::Green,
            );

            write_token(
                &spaces,
                writers,
                false,
                header_width,
                Color::Green,
                Color::Green,
            );

            write_token(
                &pugid_message,
                writers,
                true,
                header_width,
                Color::Green,
                Color::Green,
            );

            write_token(
                "\n",
                writers,
                false,
                header_width,
                Color::Green,
                Color::Green,
            );

            write_token(
                &spaces,
                writers,
                false,
                header_width,
                Color::Green,
                Color::Green,
            );

            write_token(
                "\n",
                writers,
                false,
                header_width,
                Color::Green,
                Color::Green,
            );

            state.last_tag = None;

            return true;
        }
    }

    false
}

fn write_dead_process(
    tag: &str,
    message: &str,
    state: &mut State,
    writers: &mut [Writer],
    header_width: usize,
) -> bool {
    let spaces = " ".repeat(header_width.saturating_sub(1));

    if let Some((dead_pid, dead_process_name)) = get_dead_process(
        tag,
        message,
        &state.pids_map.keys().cloned().collect(),
        &state.named_processes,
        &state.catchall_package,
    ) {
        let spaces = spaces.color(Color::Red).on_color(Color::Red).to_string();

        let dead_process_message = format!(
            " Process {} (PID: {}) ended\n",
            &dead_process_name.color(Color::Yellow),
            &dead_pid.color(Color::Yellow)
        );

        if state.pids_map.contains_key(&dead_pid) {
            state.pids_map.remove(&dead_pid);
        }

        write_token(
            &spaces,
            writers,
            false,
            header_width,
            Color::Red,
            Color::Red,
        );

        write_token("\n", writers, false, header_width, Color::Red, Color::Red);

        write_token(
            &spaces,
            writers,
            false,
            header_width,
            Color::Red,
            Color::Red,
        );

        write_token(
            &dead_process_message,
            writers,
            true,
            header_width,
            Color::Red,
            Color::Red,
        );

        write_token(
            &spaces,
            writers,
            false,
            header_width,
            Color::Red,
            Color::Red,
        );

        write_token("\n", writers, false, header_width, Color::Red, Color::Red);

        state.last_tag = None;

        return true;
    }

    false
}

fn write_pid(
    state: &mut State,
    args: &CliArgs,
    writers: &mut [Writer],
    header_width: &mut usize,
    owner: &str,
    level_foreground: Color,
    level_background: Color,
) {
    let pid_width = args.pid_width as usize;

    if args.show_pid && !&owner.is_empty() {
        let mut display_owner = owner.to_string();
        let pid_color = get_token_color(owner, state);

        if display_owner.len() > pid_width {
            display_owner.truncate(pid_width - *ELLIPSIS_COUNT);
            display_owner = format!("{}{}", &display_owner, *ELLIPSIS);
        }

        let pid_display = format!("{:width$}", display_owner, width = pid_width);

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
        *header_width += pid_width + 1;
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

    if args.show_package && !&owner.is_empty() {
        let package_name = state
            .pids_map
            .get(owner)
            .cloned()
            .unwrap_or(format!("UNKNOWN({owner})"));
        let mut display_pkg = package_name.clone();
        let pkg_color = get_token_color(&package_name, state);

        if display_pkg.len() > package_width {
            display_pkg.truncate(package_width - *ELLIPSIS_COUNT);
            display_pkg = format!("{}{}", &display_pkg, *ELLIPSIS);
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
        *header_width += package_width + 1;
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

    if tag_width > 0 {
        if Some(tag.to_string()) != state.last_tag || args.always_show_tags {
            state.last_tag = Some(tag.to_string());

            let mut display_tag = tag.to_string();

            if display_tag.len() > tag_width {
                display_tag.truncate(tag_width - *ELLIPSIS_COUNT);
                display_tag = format!("{}{}", &display_tag, *ELLIPSIS);
            }

            let tag_color = get_token_color(tag, state);
            let tag_display = if args.show_pid || args.show_package {
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
        *header_width += tag_width + 1;
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

fn apply_message_rules(args: &CliArgs, message: &str) -> String {
    let mut message = message.to_string();
    if STRICT_MODE.is_match(&message) {
        message = STRICT_MODE
            .replace(&message, |caps: &regex::Captures| {
                format!(
                    "{}{}{}",
                    &caps[1],
                    caps[2].color(Color::Red),
                    caps[3].color(Color::Yellow)
                )
            })
            .to_string();
    }

    if args.gc_color && GC_COLOR.is_match(&message) {
        message = GC_COLOR
            .replace(&message, |caps: &regex::Captures| {
                format!(
                    "{}{}{}{}",
                    &caps[1],
                    caps[2].color(Color::Green),
                    &caps[3],
                    caps[4].color(Color::Yellow)
                )
            })
            .to_string();
    }

    message
}

fn write_message(
    message: &str,
    writers: &mut [Writer],
    header_width: usize,
    level_foreground: Color,
    level_background: Color,
) {
    write_token(
        message,
        writers,
        true,
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
}

fn write_log_line(line: &str, state: &mut State, args: &CliArgs, writers: &mut [Writer]) {
    let base_level_size = 1 + 1 + 3;
    let header_width = &mut 0;

    if NATIVE_TAGS_LINE.is_match(line) {
        return;
    }

    let log_line = match LOG_LINE.captures(line) {
        Some(cap) => cap,
        None => return,
    };

    let owner = log_line
        .get(3)
        .map_or(String::default(), |mat| mat.as_str().to_string())
        .trim()
        .to_string();

    let tag = log_line
        .get(2)
        .map_or(String::default(), |mat| mat.as_str().to_string())
        .trim()
        .to_string();

    let level = log_line
        .get(1)
        .map_or(LogLevel::default(), |mat| LogLevel::from(mat.as_str()));

    let mut message = log_line
        .get(4)
        .map_or(String::default(), |mat| mat.as_str().to_string())
        .trim()
        .to_string();

    let level_foreground = Color::Black;

    let level_background = match level {
        LogLevel::DEBUG => Color::BrightBlue,
        LogLevel::INFO => Color::BrightGreen,
        LogLevel::WARN => Color::BrightYellow,
        LogLevel::ERROR => Color::TrueColor { r: 255, g: 100, b: 0 }, // DarkOrange
        LogLevel::FATAL => Color::BrightRed,
        LogLevel::VERBOSE => Color::BrightCyan,
    };

    if args.show_pid {
        *header_width += args.pid_width as usize
    }

    if args.show_package {
        *header_width += args.package_width as usize
    }

    *header_width += (2 + args.tag_width + base_level_size) as usize;

    if write_started_process(line, state, writers, *header_width) {
        return;
    }

    if write_dead_process(&tag, &message, state, writers, *header_width) {
        return;
    }

    if !args.all && !state.pids_map.contains_key(&owner) {
        return;
    }

    if level < state.log_level {
        return;
    }

    if let Some(ignore_tag) = &args.ignore_tag
        && is_matching_tag(&tag, ignore_tag)
    {
        return;
    }

    if let Some(tag_args) = &args.tag
        && !is_matching_tag(&tag, tag_args)
    {
        return;
    }

    if tag == "DEBUG"
        && let Some(_) = BACKTRACE_LINE.captures(message.trim_start())
    {
        message = message.trim_start().to_string();
    }

    *header_width = 0;

    write_pid(
        state,
        args,
        writers,
        header_width,
        &owner,
        level_foreground,
        level_background,
    );

    write_package_name(
        &owner,
        args,
        state,
        writers,
        header_width,
        level_foreground,
        level_background,
    );

    write_tag(
        &tag,
        args,
        state,
        writers,
        header_width,
        level_foreground,
        level_background,
    );

    write_log_level(
        level,
        args,
        writers,
        header_width,
        level_foreground,
        level_background,
    );

    *header_width += base_level_size as usize;

    message = apply_message_rules(args, &message);

    write_message(
        &message,
        writers,
        *header_width,
        level_foreground,
        level_background,
    );
}

fn panic_hook(info: &PanicHookInfo) {
    let err_loc = info.location().unwrap_or(panic::Location::caller());
    let err_msg = match info.payload().downcast_ref::<&str>() {
        Some(str) => *str,
        None => match info.payload().downcast_ref::<String>() {
            Some(str) => &str[..],
            None => "Box<Any>",
        },
    };

    let err_msg = format!(
        "{err_msg} => {}:{}:{}",
        err_loc.file(),
        err_loc.line(),
        err_loc.column()
    )
    .red()
    .bold();

    let thread_err_msg = format!(
        "thread 'main' ({}) panicked at {}:{}:{}",
        id(),
        err_loc.file(),
        err_loc.line(),
        err_loc.column()
    )
    .red()
    .bold();

    eprintln!("{thread_err_msg}");
    eprintln!("{err_msg}");
}

fn ctrlc_handler() {
    let bin_name = env!("CARGO_BIN_NAME").cyan().bold();
    let message = "Stopped by user.".cyan().bold();

    println!("{bin_name} {message}");
    exit(0);
}

fn main() {
    panic::set_hook(Box::new(panic_hook));
    ctrlc::set_handler(ctrlc_handler).unwrap_or_panic("Failed to set CTRL+C handler");

    let mut adb_child = None;

    let args = &mut CliArgs::parse_args();
    let stdin = stdin();
    let base_adb_command = &get_adb_command(args);
    let logcat_command = ["logcat", "-v", "brief"].map(|item| item.to_string());
    let adb_command = &mut base_adb_command.clone();
    let console_width = get_console_width();
    let stdout_writer = Writer::new_console(console_width, !args.no_color);
    let writers = &mut vec![stdout_writer];
    let packages = &mut args
        .packages
        .iter()
        .map(|package| package.to_string())
        .collect::<HashSet<_>>();

    adb_command.extend(logcat_command);

    match get_adb_devices(base_adb_command) {
        // TODO: implement device selection
        Some(devices) => {
            for (index, device) in devices.iter().enumerate() {
                let message = format!("Found Device #{index}: {device:?}").cyan().bold();
                println!("{message}");
            }
        }

        None => {
            let err = Error::from(ErrorKind::NotConnected);
            let err_code = err.raw_os_error().unwrap_or(1);
            let err = err.to_string().red().bold();
            let err_header = format!("error: {err}").red().bold();
            let error_message = concat!(
                "ADB cannot find any attached devices!",
                "\n",
                "Attach a device and try again!"
            )
            .red()
            .bold();

            if stdin.is_terminal() {
                eprintln!("{err_header}");
                eprintln!("{error_message}");
                exit(err_code);
            }
        }
    }

    if args.ignore_system_tags {
        let mut system_tags: Vec<String> =
            SYSTEM_TAGS.iter().map(|tag| format!("^{tag}$")).collect();
        args.ignore_tag = match args.ignore_tag.as_mut() {
            Some(existing) => {
                existing.append(&mut system_tags);
                Some(existing.to_vec())
            }
            None => Some(system_tags),
        }
    }

    if let Some(ignore_tags) = args.ignore_tag.clone() {
        args.ignore_tag = Some(
            ignore_tags
                .iter()
                .flat_map(|tag_arg| tag_arg.split(','))
                .map(|tag| tag.trim().to_string())
                .filter(|tag| !tag.is_empty())
                .collect(),
        );
    }

    if let Some(tags) = args.tag.clone() {
        args.tag = Some(
            tags.iter()
                .flat_map(|tag_arg| tag_arg.split(','))
                .map(|tag| tag.trim().to_string())
                .filter(|tag| !tag.is_empty())
                .collect(),
        );
    }

    if let Some(path) = args.output_path.clone() {
        let file_writer =
            Writer::new_file(File::create(path).unwrap_or_panic("Failed to create output file"));
        writers.push(file_writer);
    }

    if args.current_app
        && let Some(running_packages) = get_current_app_package(base_adb_command)
        && !running_packages.is_empty()
    {
        packages.extend(
            running_packages
                .iter()
                .map(|package| package.to_string())
                .collect::<HashSet<_>>(),
        );
    }

    if let Some(regex) = args.regex.clone() {
        adb_command.extend(["-e".to_string(), regex]);
    }

    if !args.keep_logcat && stdin.is_terminal() {
        let message = format!("Clearing logcat{}", *ELLIPSIS).cyan().bold();
        println!("{message}");

        let clear_cmd = [
            base_adb_command.clone(),
            vec!["logcat".to_string(), "-c".to_string()],
        ]
        .concat();
        let _ = Command::new(&clear_cmd[0]).args(&clear_cmd[1..]).output();
    }

    let catchall_package = &packages
        .iter()
        .filter(|package| !package.contains(':'))
        .cloned()
        .collect::<Vec<_>>();

    let named_processes = packages
        .iter()
        .filter(|package| package.contains(':'))
        .map(|package| package.strip_suffix(':').unwrap_or(package).to_string())
        .collect::<Vec<_>>();

    if packages.is_empty() {
        args.all = true;
    }

    let pids_map = get_processes(base_adb_command, catchall_package, args);

    let tag_colors = vec![
        Color::BrightRed,
        Color::BrightBlue,
        Color::BrightCyan,
        Color::BrightGreen,
        Color::BrightYellow,
        Color::BrightMagenta,
    ];

    let known_tags = HashMap::from([
        ("jdwp".to_string(), Color::White),
        ("DEBUG".to_string(), Color::Yellow),
        ("Process".to_string(), Color::White),
        ("dalvikvm".to_string(), Color::White),
        ("StrictMode".to_string(), Color::White),
        ("AndroidRuntime".to_string(), Color::Cyan),
        ("ActivityThread".to_string(), Color::White),
        ("ActivityManager".to_string(), Color::White),
    ]);

    let mut state = State {
        pids_map,
        last_tag: None,
        app_pid: None,
        log_level: args.log_level,
        named_processes,
        catchall_package: catchall_package.clone(),
        token_colors: tag_colors,
        known_tokens: known_tags,
    };

    if stdin.is_terminal() {
        adb_child = Some(
            Command::new(&adb_command[0])
                .args(&adb_command[1..])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap_or_panic("Failed to start adb logcat process"),
        );
    }

    let mut log_source = if let Some(adb_child) = adb_child {
        LogSource::Process(adb_child)
    } else {
        LogSource::Stdin
    };

    let (stdout_source, stderr_source) = match log_source {
        LogSource::Process(ref mut child) => {
            let stdout = child
                .stdout
                .take()
                .map(|stdout| Box::new(stdout) as Box<dyn Read>)
                .unwrap_or_panic("Failed to capture stdout");

            let stderr = child
                .stderr
                .take()
                .map(|stderr| Box::new(stderr) as Box<dyn Read>);

            (stdout, stderr)
        }

        LogSource::Stdin => (Box::new(stdin) as Box<dyn Read>, None),
    };

    let mut stdout = BufReader::new(stdout_source);
    let mut stderr = stderr_source.map(BufReader::new);

    let message = if !packages.is_empty() {
        let packages_vec = packages.iter().cloned().collect::<Vec<_>>();
        let packages_str = packages_vec.join(", ");

        format!(
            "Capturing logcat messages from packages: [{packages_str}]{}",
            *ELLIPSIS
        )
        .cyan()
        .bold()
    } else {
        format!("Capturing all logcat messages{}", *ELLIPSIS)
            .cyan()
            .bold()
    };

    println!("{message}");

    loop {
        if let LogSource::Process(ref mut adb_child) = log_source {
            let exit_status = adb_child.try_wait();

            match exit_status {
                Ok(exit_status) => {
                    if let Some(status) = exit_status {
                        let message = format!(
                            "Child process {} exited with status: {status}",
                            adb_child.id()
                        )
                        .cyan()
                        .bold();

                        println!("{message}");
                        break;
                    }
                }
                Err(err) => {
                    let message = format!(
                        "Failed to wait for child process {}: {}",
                        adb_child.id(),
                        err
                    )
                    .red()
                    .bold();

                    eprintln!("{message}");
                    break;
                }
            }
        }

        let stdout_buffer = &mut vec![];
        let stderr_buffer = &mut vec![];

        let stdout_bytes_read = stdout
            .read_until(b'\n', stdout_buffer)
            .unwrap_or_panic("Error reading stream");

        if stdout_bytes_read == 0 {
            if let Some(ref mut stderr) = stderr
                && let Ok(stderr_bytes_read) = stderr.read_to_end(stderr_buffer)
                && stderr_bytes_read > 0
            {
                let err = String::from_utf8_lossy(stderr_buffer)
                    .trim_end_matches(['\r', '\n'])
                    .red()
                    .bold();

                let err_msg = format!("Error reading stream:\n{}", err).red().bold();
                eprintln!("{err_msg}");
            }

            break;
        }

        let line = String::from_utf8_lossy(stdout_buffer)
            .trim_end_matches(['\r', '\n'])
            .to_string();
        write_log_line(&line, &mut state, args, writers);
    }

    if let LogSource::Process(mut adb_child) = log_source {
        let kill_fail_message = format!("Failed to kill child process {}", adb_child.id())
            .red()
            .bold();
        let wait_fail_message = format!("Failed to wait for child process {}", adb_child.id())
            .red()
            .bold();

        adb_child.kill().unwrap_or_panic(&kill_fail_message);
        adb_child.wait().unwrap_or_panic(&wait_fail_message);
    }
}
