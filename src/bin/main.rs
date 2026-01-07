use clap::ValueEnum;
use colored::*;
use once_cell::sync::Lazy;
use pidcat::CliArgs;
use pidcat::LogLevel;
use pidcat::State;
use pidcat::Writer;
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::process::Command;
use std::process::Stdio;

static BACKTRACE_LINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#(.*?)pc\s(.*?)$").expect("Invalid Regex for BACKTRACE_LINE"));

static NATIVE_TAGS_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r".*nativeGetEnabledTags.*").expect("Invalid Regex for NATIVE_TAGS_LINE")
});

static LOG_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([A-Z])/(.+?)\( *(\d+)\): (.*?)$").expect("Invalid Regex for LOG_LINE")
});

static PID_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\w+\s+(\w+)\s+\w+\s+\w+\s+\w+\s+\w+\s+\w+\s+\w\s(.*?)$")
        .expect("Invalid Regex for PID_LINE")
});

static PID_START: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^.*: Start proc (\d+):([a-zA-Z0-9._:]+)/[a-z0-9]+ for .*? \{(.*?)\}$")
        .expect("Invalid Regex for PID_START")
});

static PID_START_UGID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^.*: Start proc ([a-zA-Z0-9._:]+) for ([a-z]+ [^:]+): pid=(\d+) uid=(\d+) gids=(.*)$",
    )
    .expect("Invalid Regex for PID_START_UGID")
});

static PID_START_DALVIK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^E/dalvikvm\(\s*(\d+)\): >>>>> ([a-zA-Z0-9._:]+) \[ userId:0 \| appId:(\d+) \]$")
        .expect("Invalid Regex for PID_START_DALVIK")
});

static PID_KILL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^Killing (\d+):([a-zA-Z0-9._:]+)/[^:]+: (.*)$")
        .expect("Invalid Regex for PID_KILL")
});

static PID_LEAVE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^No longer want ([a-zA-Z0-9._:]+) \(pid (\d+)\): .*$")
        .expect("Invalid Regex for PID_LEAVE")
});

static PID_DEATH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^Process ([a-zA-Z0-9._:]+) \(pid (\d+)\) has died.?$")
        .expect("Invalid Regex for PID_DEATH")
});

static STRICT_MODE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(StrictMode policy violation)(; ~duration=)(\d+ ms)")
        .expect("Invalid Regex for STRICT_MODE")
});

static GC_COLOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(GC_(?:CONCURRENT|FOR_M?ALLOC|EXTERNAL_ALLOC|EXPLICIT) )(freed <?\d+.)(, \d+\% free \d+./\d+., )(paused \d+ms(?:\+\d+ms)?)"
    ).expect("Invalid Regex for GC_COLOR")
});

static VISIBLE_ACTIVITIES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"VisibleActivityProcess:\[\s*(?:(?:ProcessRecord\{\w+\s*\d+:(?:[a-zA-Z.]+)/\w+\})\s*)+\]",
    )
    .expect("Invalid Regex for VISIBLE_ACTIVITIES")
});

static VISIBLE_PACKAGES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"ProcessRecord\{\w+\s*\d+:([a-zA-Z.]+)/\w+\}")
        .expect("Invalid Regex for VISIBLE_PACKAGES")
});

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

fn get_wrapped_indent(
    message: &str,
    width: i16,
    header_size: usize,
    level_foreground: colored::Color,
    level_background: colored::Color,
) -> String {
    if width == -1 {
        return message.to_string();
    }

    let message = message.replace('\t', "    ");
    let wrap_area = (width as usize).saturating_sub(header_size);

    if wrap_area == 0 {
        return message;
    }

    let mut message_buffer = String::new();
    let chars = message.chars().collect::<Vec<_>>();
    let mut current = 0;

    while current < chars.len() {
        let next_index = std::cmp::min(current + wrap_area, chars.len());
        let segment = chars[current..next_index].iter().collect::<String>();

        message_buffer.push_str(&segment);

        if next_index < chars.len() {
            message_buffer.push('\n');

            let indent_len = header_size.saturating_sub(4);
            let spaces = if level_foreground == level_background {
                " ".repeat(indent_len)
                    .color(level_foreground)
                    .on_color(level_background)
                    .to_string()
            } else {
                " ".repeat(indent_len)
            };
            message_buffer.push_str(&spaces);

            let future_index = next_index + wrap_area;
            let is_last_line = future_index >= chars.len();
            let connector = if level_foreground == level_background {
                "   "
            } else if is_last_line {
                " └ "
            } else {
                " | "
            };

            let colored_connector = connector
                .color(level_foreground)
                .on_color(level_background)
                .bold()
                .to_string();

            message_buffer.push_str(&colored_connector);
            message_buffer.push(' ');
        }

        current = next_index;
    }

    message_buffer
}

fn get_token_color(tag: &str, state: &mut State) -> colored::Color {
    if !state.known_tags.contains_key(tag) {
        if !state.tag_colors.is_empty() {
            // Equivalent to TAG_COLORS.pop(0)
            let color = state.tag_colors.remove(0);
            state.known_tags.insert(tag.to_string(), color);
        } else {
            // Fallback if we run out of colors (though logic below prevents this)
            return colored::Color::White;
        }
    }

    let color = *state.known_tags.get(tag).unwrap();

    // Move to end of list (LRU logic)
    if let Some(pos) = state.tag_colors.iter().position(|&c| c == color) {
        state.tag_colors.remove(pos);
    }
    state.tag_colors.push(color);

    color
}

fn get_adb_command(args: &CliArgs) -> Vec<String> {
    let mut base_adb_command = vec!["adb".to_string()];

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
    let mut pids_map = HashMap::new();
    let mut cmd = Command::new(&base_adb_command[0]);

    if base_adb_command.len() > 1 {
        cmd.args(&base_adb_command[1..]);
    }

    let output = cmd.args(["shell", "ps"]).stdout(Stdio::piped()).output();

    if let Ok(out) = output {
        let reader = BufReader::new(&out.stdout[..]);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(caps) = PID_LINE.captures(&line) {
                let pid = caps.get(1).map_or("", |m| m.as_str()).to_string();
                let process = caps.get(2).map_or("", |m| m.as_str()).to_string();

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
            "".to_string(),      // started_uid
            "".to_string(),      // started_gids
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
            "".to_string(),      // started_gids
            caps[2].to_string(), // started_package
            "".to_string(),      // started_target
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
    token: &str,
    named_processes: &[String],
    catchall_package: &[String],
) -> bool {
    if catchall_package.is_empty() && named_processes.is_empty() {
        return true;
    }

    if named_processes.contains(&token.to_string()) {
        return true;
    }

    match token.find(':') {
        None => catchall_package.contains(&token.to_string()),
        Some(index) => catchall_package.contains(&token[..index].to_string()),
    }
}

fn is_matching_tag(tag: &str, tags: &[String]) -> bool {
    let regex_chars = r".*+?[]{}()|\^$";

    for m_tag in tags.iter().map(|tag| tag.trim()) {
        let is_regex = m_tag.chars().any(|char| regex_chars.contains(char));

        if is_regex {
            let pattern = if m_tag.starts_with('^') {
                m_tag.to_string()
            } else {
                format!("^{}", m_tag)
            };

            if let Ok(re) = Regex::new(&pattern)
                && re.is_match(tag)
            {
                return true;
            }
        } else if tag.contains(m_tag) {
            return true;
        }
    }

    false
}

fn write_token(
    token: &str,
    wrap: bool,
    level_foreground: colored::Color,
    level_background: colored::Color,
    header_size: usize,
    writers: &mut [Writer],
) -> usize {
    let local_header = header_size;
    for writer in writers.iter_mut() {
        writer.width = get_console_width();

        let buffer = if wrap && writer.width != -1 {
            get_wrapped_indent(
                token,
                writer.width,
                header_size,
                level_foreground,
                level_background,
            )
        } else {
            token.to_string()
        };

        let line = if writer.show_colors {
            &buffer
        } else {
            &buffer.normal().clear().to_string()
        };

        writer.write(line);
        writer.flush();
    }

    local_header
}

fn write_started_process(
    line: &str,
    state: &mut State,
    writers: &mut [Writer],
    current_header_size: usize,
    level_foreground: Color,
    spaces: &str,
) {
    if let Some(procs) = get_started_process(line) {
        let (started_pid, started_uid, started_gids, started_package, started_target) = procs;

        let spaces = spaces
            .color(colored::Color::Green)
            .on_color(colored::Color::Green)
            .to_string();

        let started_process_message = format!(
            " Process {} created for {}\n",
            &started_package.color(colored::Color::Yellow),
            &started_target.color(colored::Color::Yellow)
        );

        let pugid_message = format!(
            " PID: {}   UID: {}   GIDs: {}",
            &started_pid.color(colored::Color::Yellow),
            &started_uid.color(colored::Color::Yellow),
            &started_gids.color(colored::Color::Yellow)
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
                "\n",
                false,
                level_foreground,
                colored::Color::Green,
                current_header_size,
                writers,
            );

            write_token(
                &spaces,
                false,
                colored::Color::Green,
                colored::Color::Green,
                current_header_size,
                writers,
            );

            write_token(
                &started_process_message,
                true,
                colored::Color::Green,
                colored::Color::Green,
                current_header_size,
                writers,
            );

            write_token(
                &spaces,
                false,
                colored::Color::Green,
                colored::Color::Green,
                current_header_size,
                writers,
            );

            write_token(
                &pugid_message,
                false,
                colored::Color::Green,
                colored::Color::Green,
                current_header_size,
                writers,
            );

            write_token(
                "\n",
                false,
                colored::Color::Green,
                colored::Color::Green,
                current_header_size,
                writers,
            );

            state.last_tag = None;
        }
    }
}

fn write_dead_process(
    state: &mut State,
    writers: &mut [Writer],
    current_header_size: usize,
    tag: &str,
    message: &str,
    spaces: String,
) {
    if let Some((dead_pid, dead_process_name)) = get_dead_process(
        tag,
        message,
        &state.pids_map.keys().cloned().collect(),
        &state.named_processes,
        &state.catchall_package,
    ) {
        let spaces = spaces
            .color(colored::Color::Red)
            .on_color(colored::Color::Red)
            .to_string();

        let dead_process_message = format!(
            " Process {} (PID: {}) ended\n",
            &dead_process_name.color(colored::Color::Yellow),
            &dead_pid.color(colored::Color::Yellow)
        );

        if state.pids_map.contains_key(&dead_pid) {
            state.pids_map.remove(&dead_pid);
        }

        write_token(
            "\n",
            false,
            colored::Color::Red,
            colored::Color::Red,
            current_header_size,
            writers,
        );
        write_token(
            &spaces,
            false,
            colored::Color::Red,
            colored::Color::Red,
            current_header_size,
            writers,
        );
        write_token(
            &dead_process_message,
            false,
            colored::Color::Red,
            colored::Color::Red,
            current_header_size,
            writers,
        );
        write_token(
            "\n",
            false,
            colored::Color::Red,
            colored::Color::Red,
            current_header_size,
            writers,
        );

        state.last_tag = None;
    }
}

fn write_pid(
    state: &mut State,
    args: &CliArgs,
    writers: &mut [Writer],
    current_header_size: &mut usize,
    pid_width: usize,
    owner: &str,
    level_foreground: Color,
    level_background: Color,
) {
    if args.show_pid && !&owner.is_empty() {
        let mut display_owner = owner.to_string();
        let pid_color = get_token_color(owner, state);

        if display_owner.len() > pid_width {
            display_owner = format!("{}...", &display_owner[..pid_width - 3]);
        }

        let pid_display = format!("{:width$}", display_owner, width = pid_width);

        let pid_display = if args.no_color {
            pid_display
        } else {
            pid_display.color(pid_color).to_string()
        };
        *current_header_size = write_token(
            &pid_display,
            false,
            level_foreground,
            level_background,
            *current_header_size,
            writers,
        );
        *current_header_size = write_token(
            " ",
            false,
            level_foreground,
            level_background,
            *current_header_size,
            writers,
        );
        *current_header_size += pid_width + 1;
    }
}

fn write_package_name(
    state: &mut State,
    args: &CliArgs,
    writers: &mut [Writer],
    current_header_size: &mut usize,
    package_width: usize,
    owner: &str,
    level_foreground: Color,
    level_background: Color,
) {
    if args.show_package && !&owner.is_empty() {
        let package_name = state
            .pids_map
            .get(owner)
            .cloned()
            .unwrap_or(format!("UNKNOWN({})", owner));
        let mut display_pkg = package_name.clone();
        let pkg_color = get_token_color(&package_name, state);

        if display_pkg.len() > package_width {
            display_pkg = format!("{}...", &display_pkg[..package_width - 3]);
        }

        let pkg_display = format!("{:width$}", display_pkg, width = package_width);
        let pkg_display = if args.no_color {
            pkg_display
        } else {
            pkg_display.color(pkg_color).to_string()
        };

        *current_header_size = write_token(
            &pkg_display,
            false,
            level_foreground,
            level_background,
            *current_header_size,
            writers,
        );
        *current_header_size = write_token(
            " ",
            false,
            level_foreground,
            level_background,
            *current_header_size,
            writers,
        );
        *current_header_size += package_width + 1;
    }
}

fn write_tag(
    state: &mut State,
    args: &CliArgs,
    writers: &mut [Writer],
    current_header_size: &mut usize,
    tag_width: usize,
    tag: &str,
    level_foreground: Color,
    level_background: Color,
) {
    if tag_width > 0 {
        if Some(tag.to_string()) != state.last_tag || args.always_show_tags {
            state.last_tag = Some(tag.to_string());

            let mut display_tag = tag.to_string();

            if display_tag.len() > tag_width {
                display_tag = format!("{}...", &display_tag[..tag_width - 3]);
            }

            let tag_color = get_token_color(&tag, state);
            let tag_display = if args.show_package {
                format!("{:>width$}", display_tag, width = tag_width)
            } else {
                format!("{:width$}", display_tag, width = tag_width)
            };

            let tag_display = if args.no_color {
                tag_display
            } else {
                tag_display.color(tag_color).to_string()
            };

            *current_header_size = write_token(
                &tag_display,
                false,
                level_foreground,
                level_background,
                *current_header_size,
                writers,
            );
        } else {
            *current_header_size = write_token(
                &" ".repeat(tag_width),
                false,
                level_foreground,
                level_background,
                *current_header_size,
                writers,
            );
        }
        *current_header_size = write_token(
            " ",
            false,
            level_foreground,
            level_background,
            *current_header_size,
            writers,
        );
        *current_header_size += tag_width + 1;
    }
}

fn write_log_level(
    args: &CliArgs,
    writers: &mut [Writer],
    base_level_size: usize,
    current_header_size: &mut usize,
    level: LogLevel,
    level_foreground: Color,
    level_background: Color,
) {
    let level_str = if args.no_color {
        format!(" {level} ")
    } else {
        let space = " ".color(level_foreground).on_color(level_background);
        format!(
            "{}{}{}",
            space,
            level
                .to_string()
                .bold()
                .color(level_foreground)
                .on_color(level_background),
            space
        )
    };

    *current_header_size = write_token(
        &level_str,
        false,
        level_foreground,
        level_background,
        *current_header_size,
        writers,
    );
    *current_header_size = write_token(
        " ",
        false,
        level_foreground,
        level_background,
        *current_header_size,
        writers,
    );
    *current_header_size += base_level_size;
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
    writers: &mut [Writer],
    current_header_size: usize,
    message: &str,
    level_foreground: Color,
    level_background: Color,
) {
    write_token(
        message,
        true,
        level_foreground,
        level_background,
        current_header_size,
        writers,
    );
    write_token(
        "\n",
        false,
        level_foreground,
        level_background,
        current_header_size,
        writers,
    );
}

fn write_log_line(line: &str, state: &mut State, args: &CliArgs, writers: &mut [Writer]) {
    let base_level_size = 3 + 1;
    let mut current_header_size;

    let pid_width = args.pid_width as usize;
    let package_width = args.package_width as usize;
    let tag_width = args.tag_width as usize;

    if NATIVE_TAGS_LINE.is_match(line) {
        return;
    }

    let log_line = match LOG_LINE.captures(line) {
        Some(cap) => cap,
        None => return,
    };

    let owner = log_line
        .get(3)
        .map_or("", |mat| mat.as_str())
        .trim()
        .to_string();
    let tag = log_line
        .get(2)
        .map_or("", |mat| mat.as_str())
        .trim()
        .to_string();
    let level = log_line.get(1).map_or(LogLevel::VERBOSE, |mat| {
        LogLevel::from_str(mat.as_str(), true).expect("Invalid log level")
    });

    let mut message = log_line
        .get(4)
        .map_or("", |mat| mat.as_str())
        .trim()
        .to_string();

    let level_foreground = match level {
        LogLevel::VERBOSE => colored::Color::White,
        _ => colored::Color::Black,
    };

    let level_background = match level {
        LogLevel::DEBUG => colored::Color::BrightBlue,
        LogLevel::INFO => colored::Color::Green,
        LogLevel::WARN => colored::Color::Yellow,
        LogLevel::ERROR | LogLevel::FATAL => colored::Color::Red,
        _ => colored::Color::Black,
    };

    current_header_size = if args.show_package { package_width } else { 0 };
    current_header_size += 1 + tag_width + base_level_size;
    let spaces = " ".repeat(current_header_size - 1);

    write_started_process(
        line,
        state,
        writers,
        current_header_size,
        level_foreground,
        &spaces,
    );

    write_dead_process(state, writers, current_header_size, &tag, &message, spaces);

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

    current_header_size = 0;

    write_pid(
        state,
        args,
        writers,
        &mut current_header_size,
        pid_width,
        &owner,
        level_foreground,
        level_background,
    );

    write_package_name(
        state,
        args,
        writers,
        &mut current_header_size,
        package_width,
        &owner,
        level_foreground,
        level_background,
    );

    write_tag(
        state,
        args,
        writers,
        &mut current_header_size,
        tag_width,
        &tag,
        level_foreground,
        level_background,
    );

    write_log_level(
        args,
        writers,
        base_level_size,
        &mut current_header_size,
        level,
        level_foreground,
        level_background,
    );

    message = apply_message_rules(args, &message);

    write_message(
        writers,
        current_header_size,
        &message,
        level_foreground,
        level_background,
    );
}

fn main() {
    let mut args = CliArgs::parse_args();
    let base_adb_command = get_adb_command(&args);
    let mut adb_command = [
        base_adb_command.clone(),
        vec!["logcat".to_string(), "-v".to_string(), "brief".to_string()],
    ]
    .concat();

    let mut packages = args
        .packages
        .iter()
        .map(|package| package.to_string())
        .collect::<HashSet<_>>();

    let console_width = get_console_width();

    let stdout_writer = Writer::new_console(console_width, !args.no_color);
    let mut writers = vec![stdout_writer];

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

    if !args.keep_logcat {
        let clear_cmd = [
            base_adb_command.clone(),
            vec!["logcat".to_string(), "-c".to_string()],
        ]
        .concat();
        let _ = Command::new(&clear_cmd[0]).args(&clear_cmd[1..]).output();
    }

    if let Some(path) = args.output_path.clone() {
        let file_writer =
            Writer::new_file(File::create(path).expect("Failed to create output file"));
        writers.push(file_writer);
    }

    if args.current_app
        && let Some(running_packages) = get_current_app_package(&base_adb_command)
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

    if !packages.is_empty() {
        let packages_vec = packages.iter().cloned().collect::<Vec<_>>();
        println!(
            "{}",
            format!(
                "Capturing logcat messages from packages: [{}]...",
                packages_vec.join(", ")
            )
            .cyan()
            .bold()
        );
    } else {
        args.all = true;
        println!("{}", "Capturing all logcat messages...".cyan().bold());
    }

    let catchall_package = packages
        .iter()
        .filter(|package| !package.contains(':'))
        .cloned()
        .collect::<Vec<_>>();

    let named_processes = packages
        .iter()
        .filter(|package| package.contains(':'))
        .map(|package| package.strip_suffix(':').unwrap_or(package).to_string())
        .collect::<Vec<_>>();

    let pids_map = get_processes(&base_adb_command, &catchall_package, &args);

    let tag_colors = vec![
        colored::Color::Red,
        colored::Color::Blue,
        colored::Color::Cyan,
        colored::Color::Green,
        colored::Color::Yellow,
        colored::Color::Magenta,
    ];

    let known_tags = HashMap::from([
        ("jdwp".to_string(), colored::Color::White),
        ("DEBUG".to_string(), colored::Color::Yellow),
        ("Process".to_string(), colored::Color::White),
        ("dalvikvm".to_string(), colored::Color::White),
        ("StrictMode".to_string(), colored::Color::White),
        ("AndroidRuntime".to_string(), colored::Color::Cyan),
        ("ActivityThread".to_string(), colored::Color::White),
        ("ActivityManager".to_string(), colored::Color::White),
    ]);

    let mut state = State {
        pids_map,
        last_tag: None,
        app_pid: None,
        log_level: args.log_level,
        named_processes,
        catchall_package,
        tag_colors,
        known_tags,
    };

    let _ = ctrlc::set_handler(|| {});

    let mut child = Command::new(&adb_command[0])
        .args(&adb_command[1..])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start adb logcat");

    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout);

        loop {
            let mut buffer = vec![];
            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) => break,
                Ok(_) => {
                    let content = String::from_utf8_lossy(&buffer);
                    let trimmed = content.trim_end_matches(['\r', '\n']);
                    write_log_line(trimmed, &mut state, &args, &mut writers);
                }
                Err(err) => {
                    eprintln!(
                        "{}{}",
                        "Error reading stream: ".red().bold().italic(),
                        err.to_string().red().bold().italic()
                    );
                    break;
                }
            }
        }
    }

    let _ = child.kill();
    let _ = child.wait();

    println!(
        "\n{}{}",
        env!("CARGO_BIN_NAME").cyan().bold(),
        " Stopped by user.".cyan().bold()
    );
}
