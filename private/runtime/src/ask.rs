//! Filagree "Ask" box: the presenter types a question in the viewer and the answer
//! streams into a side panel. Known tours play instantly, data questions go to
//! Snowflake Cortex through `ask/ask.py`, and everything else goes to Claude
//! headless, whose MCP bridge drives this same viewer over its socket.
//!
//! Child processes run on std threads; the main thread only polls channels, so a
//! Claude tool call coming back through the IPC inbox never deadlocks.
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

use bevy::{
    input::{
        ButtonState,
        keyboard::KeyboardInput,
        mouse::{MouseScrollUnit, MouseWheel},
    },
    prelude::*,
    window::PrimaryWindow,
};
use serde_json::{Value, json};

use crate::{
    camera::CameraInputLock,
    ipc::{self, Served},
    layout_scene::{LayoutStatus, ViewerActions},
    playback::Playback,
};

/// Answers kept in the panel.
const HISTORY: usize = 4;
const MAX_INPUT: usize = 400;
const TOUR_TIMEOUT: Duration = Duration::from_secs(60);
const ACCENT: Color = Color::srgb(0.45, 0.85, 1.0);
const MIC_IDLE: Color = Color::srgb(0.16, 0.21, 0.27);
const MIC_RECORDING: Color = Color::srgb(0.9, 0.22, 0.2);

const BADGE_TOUR: &str = "Tour";
const BADGE_DATA: &str = "Snowflake Cortex / Llama 3.3";
const BADGE_TOUR_DATA: &str = "Tour + Snowflake Cortex / Llama 3.3";
const BADGE_CLAUDE: &str = "Claude";
const ASKING_CORTEX: &str = "Asking Llama 3.3 in Snowflake Cortex...";

const IN_APP_PROMPT: &str = "You are answering inside the Filagree viewer; your text appears in a small side panel. \
Answer in at most 5 short sentences of plain text: no markdown at all (no headings, bold, bullets or tables). \
The model is already published from .visualyze/model.json; read it and reuse the walkthroughs in .visualyze/ instead of rebuilding. \
Use get_scene_state when the user says \"this\". Use present_walkthrough / set_view to show what you describe. \
For data/number questions run `uv run --with snowflake-connector-python ask/ask.py \"<q>\" [TABLE]`. Never edit files.";

// ---------------------------------------------------------------------------
// Routing (pure)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tour {
    QuestionToAnswer,
    CustomerOrder,
    LateOrder,
    Region1997,
    ProposalStreaming,
}

impl Tour {
    /// Model and walkthrough files under `<project>/.visualyze/`.
    fn files(self) -> (&'static str, &'static str) {
        match self {
            Self::QuestionToAnswer => ("model.json", "walkthrough.json"),
            Self::CustomerOrder => ("model.json", "customer-order.walkthrough.json"),
            Self::LateOrder => ("model.json", "late-order.walkthrough.json"),
            Self::Region1997 => ("model.json", "region-1997.walkthrough.json"),
            Self::ProposalStreaming => (
                "proposal-streaming.model.json",
                "proposal-streaming.walkthrough.json",
            ),
        }
    }

    fn intro(self) -> &'static str {
        match self {
            Self::QuestionToAnswer => {
                "A spoken question becomes a Cortex call over the revenue mart, built from the free TPC-H tables. \
                 Follow it from source tables to the answer you hear."
            }
            Self::CustomerOrder => {
                "A customer finds a part in stock on PartsHub and places the order; the warehouse ships it. \
                 Overnight the order lands in Snowflake as ORDERS and LINEITEM rows."
            }
            Self::LateOrder => {
                "Fulfillment records ship, commit and receipt dates on every line item. \
                 Those dates reach LINEITEM overnight and roll up into each supplier's late share."
            }
            Self::Region1997 => {
                "ASK_LAKE answers from the revenue mart, which joins five TPC-H tables. \
                 The tour shows the four boxes involved while Cortex works out the actual number."
            }
            Self::ProposalStreaming => {
                "Proposal: stream orders into ORDERS_LIVE and keep revenue fresh with a dynamic table instead of the nightly load. \
                 Rehearsed in zero-copy clone FILAGREE_WHATIF, built in 4.7s; 10,000 rows on both sides, 0 mismatched rows. \
                 Freshness goes from a day to a minute, but a FULL refresh re-reads 6M lines every minute."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    Tour { tour: Tour, also_data: bool },
    Data,
    Claude,
}

fn words(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Decide where a question goes. Tours are matched first, then data questions.
pub(crate) fn classify(question: &str, has_selection: bool) -> Route {
    let q = question.to_lowercase();
    let w = words(&q);
    let has_word = |word: &str| w.contains(&word);
    let has = |phrase: &str| q.contains(phrase);
    if has("what if") || w.iter().any(|w| w.starts_with("stream")) {
        return Route::Tour {
            tour: Tour::ProposalStreaming,
            also_data: false,
        };
    }
    if has("spoken question") || has("question get answered") || has("question gets answered") {
        return Route::Tour {
            tour: Tour::QuestionToAnswer,
            also_data: false,
        };
    }
    if has_word("late") || has_word("lateness") {
        return Route::Tour {
            tour: Tour::LateOrder,
            also_data: false,
        };
    }
    if has_word("1997") || has("which region") {
        return Route::Tour {
            tour: Tour::Region1997,
            also_data: true,
        };
    }
    if [
        "orders a part",
        "order a part",
        "customer order",
        "customers order",
    ]
    .iter()
    .any(|p| has(p))
    {
        return Route::Tour {
            tour: Tour::CustomerOrder,
            also_data: false,
        };
    }
    let data_words = ["which", "top", "most", "revenue", "total", "average"];
    if data_words.iter().any(|d| has_word(d))
        || has("how many")
        || has("how much")
        || (has_selection && (has("this table") || has("in this")))
    {
        return Route::Data;
    }
    Route::Claude
}

/// The default font only has ASCII glyphs.
pub(crate) fn ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\u{2014}' | '\u{2013}' | '\u{2212}' => out.push('-'),
            '\u{2018}' | '\u{2019}' | '\u{2032}' => out.push('\''),
            '\u{201C}' | '\u{201D}' => out.push('"'),
            '\u{2026}' => out.push_str("..."),
            '\u{2192}' => out.push_str("->"),
            '\u{2190}' => out.push_str("<-"),
            '\u{00B7}' | '\u{2022}' => out.push('-'),
            '\u{00A0}' | '\t' => out.push(' '),
            '\u{00D7}' => out.push('x'),
            '\n' => out.push('\n'),
            c if c.is_ascii() && !c.is_control() => out.push(c),
            c if c.is_control() => {}
            _ => out.push('?'),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Child process events (pure parsing)

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Msg {
    /// Claude's session exists, so later questions can `--resume` it.
    Started,
    Text(String),
    /// A new text block begins.
    Break,
    Tool(String),
    Result {
        text: String,
        error: bool,
    },
    /// One `ask.py` answer.
    Answer(String),
    Exited {
        ok: bool,
        detail: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Claude,
    Data,
}

pub(crate) fn parse_claude_line(line: &str) -> Option<Msg> {
    let value: Value = serde_json::from_str(line).ok()?;
    match value["type"].as_str()? {
        "system" if value["subtype"] == "init" => Some(Msg::Started),
        "stream_event" => {
            let event = &value["event"];
            match event["type"].as_str()? {
                "content_block_start" => match event["content_block"]["type"].as_str()? {
                    "text" => Some(Msg::Break),
                    "tool_use" => Some(Msg::Tool(
                        event["content_block"]["name"].as_str()?.to_string(),
                    )),
                    _ => None,
                },
                "content_block_delta" if event["delta"]["type"] == "text_delta" => {
                    Some(Msg::Text(event["delta"]["text"].as_str()?.to_string()))
                }
                _ => None,
            }
        }
        "result" => Some(Msg::Result {
            text: value["result"]
                .as_str()
                .map(str::to_string)
                .or_else(|| value["errors"].as_array().map(|e| format!("{e:?}")))
                .unwrap_or_else(|| value["subtype"].as_str().unwrap_or("").to_string()),
            error: value["is_error"].as_bool().unwrap_or(false),
        }),
        _ => None,
    }
}

pub(crate) fn parse_data_line(line: &str) -> Option<Msg> {
    let value: Value = serde_json::from_str(line.trim()).ok()?;
    Some(Msg::Answer(value["answer"].as_str()?.to_string()))
}

fn friendly_tool(name: &str) -> &'static str {
    match name.rsplit("__").next().unwrap_or(name) {
        "present_walkthrough" => "Presenting walkthrough...",
        "set_view" => "Setting the view...",
        "get_scene_state" => "Reading scene...",
        "publish_model" => "Publishing the model...",
        "check_model" | "validate_model" => "Checking the model...",
        "Bash" => "Asking Snowflake...",
        "Read" => "Reading files...",
        "Glob" | "Grep" => "Searching the project...",
        _ if name.starts_with("mcp__visualyze") => "Driving the viewer...",
        _ => "Working...",
    }
}

// ---------------------------------------------------------------------------
// Processes

struct Job {
    entry: u64,
    kind: Kind,
    child: Arc<Mutex<Child>>,
    receive: Mutex<mpsc::Receiver<Msg>>,
}

impl Drop for Job {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }
}

fn spawn_job(mut command: Command, entry: u64, kind: Kind) -> Result<Job, String> {
    let program = command.get_program().to_string_lossy().into_owned();
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start {program}: {error}"))?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;
    let child = Arc::new(Mutex::new(child));
    let (send, receive) = mpsc::channel();
    let tail = Arc::new(Mutex::new(String::new()));
    let err_tail = tail.clone();
    let err_thread = std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);
        if let Ok(mut tail) = err_tail.lock() {
            *tail = buf;
        }
    });
    let waiter = child.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let msg = match kind {
                Kind::Claude => parse_claude_line(&line),
                Kind::Data => parse_data_line(&line),
            };
            if let Some(msg) = msg
                && send.send(msg).is_err()
            {
                return;
            }
        }
        let _ = err_thread.join();
        let ok = loop {
            let polled = match waiter.lock() {
                Ok(mut child) => child.try_wait(),
                Err(_) => break false,
            };
            match polled {
                Ok(Some(status)) => break status.success(),
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(_) => break false,
            }
        };
        let detail = tail.lock().map(|t| last_lines(&t, 6)).unwrap_or_default();
        let _ = send.send(Msg::Exited { ok, detail });
    });
    Ok(Job {
        entry,
        kind,
        child,
        receive: Mutex::new(receive),
    })
}

fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| {
            !l.trim().is_empty()
                && !l.contains("FutureWarning")
                && !l.contains("options = self.config")
        })
        .collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// `env` if set, else `name` on PATH, else the first existing fallback.
fn find_program(env: &str, name: &str, fallbacks: &[&str]) -> PathBuf {
    if let Some(path) = std::env::var_os(env).filter(|v| !v.is_empty()) {
        return PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    for fallback in fallbacks {
        let candidate = match fallback.strip_prefix("~/") {
            Some(rest) => home.join(rest),
            None => PathBuf::from(fallback),
        };
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from(name)
}

fn project_dir() -> PathBuf {
    let dir = std::env::var_os("FILAGREE_PROJECT")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    dir.canonicalize().unwrap_or(dir)
}

/// A random v4-shaped UUID without new dependencies.
fn new_uuid() -> String {
    let mut bytes = [0u8; 16];
    let filled = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .is_ok();
    if !filled {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let seed = nanos ^ (u128::from(std::process::id()) << 64);
        bytes = seed.to_le_bytes();
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub(crate) fn claude_args(
    question: &str,
    exe: &Path,
    project: &Path,
    session: &str,
    resume: bool,
) -> Vec<String> {
    let mcp = json!({"mcpServers": {"visualyze": {
        "type": "stdio",
        "command": exe,
        "args": ["mcp", "--root", project],
    }}});
    let mut args: Vec<String> = [
        "-p",
        question,
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--strict-mcp-config",
        "--mcp-config",
        &mcp.to_string(),
        "--permission-mode",
        "dontAsk",
        "--permission-prompts",
        "none",
        "--append-system-prompt",
        IN_APP_PROMPT,
        "--model",
        "sonnet",
        "--effort",
        "low",
        "--max-budget-usd",
        "1",
        if resume { "--resume" } else { "--session-id" },
        session,
        // Variadic, so it goes last.
        "--allowedTools",
        "mcp__visualyze",
        "Read",
        "Glob",
        "Grep",
        "Bash(uv run *)",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    args.shrink_to_fit();
    args
}

fn data_command(project: &Path, question: &str, table: Option<&str>) -> Command {
    let uv = find_program(
        "FILAGREE_UV",
        "uv",
        &[
            "/opt/homebrew/bin/uv",
            "~/.local/bin/uv",
            "~/.cargo/bin/uv",
            "/usr/local/bin/uv",
        ],
    );
    let mut command = Command::new(uv);
    command.current_dir(project).args([
        "run",
        "--with",
        "snowflake-connector-python",
        "ask/ask.py",
        question,
    ]);
    if let Some(table) = table {
        command.arg(table);
    }
    command
}

// ---------------------------------------------------------------------------
// State

struct Entry {
    id: u64,
    question: String,
    badge: &'static str,
    tour_status: String,
    status: String,
    answer: String,
    error: bool,
}

struct PendingTour {
    entry: u64,
    model_id: String,
    walkthrough: Value,
    title: String,
    since: Instant,
}

/// What the layout controls need to know: whether keys and the cursor belong to the ask box.
#[derive(Resource, Default)]
pub(crate) struct AskUi {
    pub focused: bool,
    pub over: bool,
}

#[derive(Resource)]
struct Ask {
    serving: bool,
    project: PathBuf,
    input: String,
    /// A question and whether it was spoken.
    submitted: Option<(String, bool)>,
    history: VecDeque<Entry>,
    next_id: u64,
    job: Option<Job>,
    tour: Option<PendingTour>,
    session: String,
    session_started: bool,
    dirty: bool,
    stick_bottom: bool,
}

impl Ask {
    fn entry(&mut self, id: u64) -> Option<&mut Entry> {
        self.history.iter_mut().find(|e| e.id == id)
    }
}

#[derive(Component)]
struct AskBar;
#[derive(Component)]
struct AskBarText;
#[derive(Component)]
struct AskPanel;
#[derive(Component)]
struct AskMic;

pub(crate) struct AskPlugin {
    pub serving: bool,
}

impl Plugin for AskPlugin {
    fn build(&self, app: &mut App) {
        let voice = VoiceConfig::from_env();
        voice.prewarm();
        app.init_resource::<AskUi>()
            .insert_resource(Voice {
                config: voice,
                recording: None,
                transcribing: None,
                count: 0,
            })
            .insert_resource(Ask {
                serving: self.serving,
                project: project_dir(),
                input: String::new(),
                submitted: None,
                history: VecDeque::new(),
                next_id: 1,
                job: None,
                tour: None,
                session: new_uuid(),
                session_started: false,
                dirty: true,
                stick_bottom: true,
            })
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (ask_input, ask_voice, ask_route, ask_poll, ask_render)
                    .chain()
                    .after(crate::layout_scene::LayoutUpdate)
                    .before(crate::camera::CameraControls),
            );
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn((
            AskBar,
            Interaction::default(),
            GlobalZIndex(18),
            BackgroundColor(Color::srgba(0.05, 0.07, 0.1, 0.9)),
            BorderColor::all(Color::srgb_u8(60, 78, 96)),
            Node {
                position_type: PositionType::Absolute,
                right: px(16),
                bottom: px(16),
                width: percent(32),
                min_height: px(40),
                padding: UiRect::axes(px(12), px(9)),
                border: UiRect::all(px(1)),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                ..default()
            },
        ))
        .with_children(|bar| {
            bar.spawn((
                AskBarText,
                Text::new(""),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb_u8(120, 140, 160)),
                Node {
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    ..default()
                },
            ));
            bar.spawn((
                AskMic,
                Interaction::default(),
                BackgroundColor(MIC_IDLE),
                Node {
                    padding: UiRect::axes(px(8), px(4)),
                    flex_shrink: 0.0,
                    align_self: AlignSelf::Center,
                    ..default()
                },
            ))
            .with_children(|mic| {
                mic.spawn((
                    Text::new("MIC"),
                    TextFont {
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(Color::srgb_u8(236, 242, 248)),
                ));
            });
        });
    commands.spawn((
        AskPanel,
        Interaction::default(),
        GlobalZIndex(18),
        ScrollPosition::default(),
        BackgroundColor(Color::srgba(0.05, 0.07, 0.1, 0.95)),
        BorderColor::all(ACCENT),
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            right: px(16),
            top: px(120),
            width: percent(32),
            max_height: percent(60),
            padding: UiRect::new(px(16), px(16), px(12), px(14)),
            border: UiRect::left(px(3)),
            flex_direction: FlexDirection::Column,
            row_gap: px(6),
            overflow: Overflow::scroll_y(),
            ..default()
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn ask_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut wheel: MessageReader<MouseWheel>,
    mut lock: ResMut<CameraInputLock>,
    mut ui: ResMut<AskUi>,
    mut ask: ResMut<Ask>,
    bar: Query<&Interaction, With<AskBar>>,
    mic: Query<&Interaction, With<AskMic>>,
    mut panel: Query<(&Interaction, &ComputedNode, &mut ScrollPosition), With<AskPanel>>,
) {
    let hovered = |i: &Interaction| *i != Interaction::None;
    let over_bar = bar.iter().any(hovered);
    let over_mic = mic.iter().any(hovered);
    let over_panel = panel.iter().any(|(i, _, _)| hovered(i));
    ui.over = over_bar || over_panel;

    // Wheel scrolls the panel; the camera is locked while the cursor is over it.
    let mut wheel_delta = 0.0;
    for event in wheel.read() {
        wheel_delta -= event.y
            * match event.unit {
                MouseScrollUnit::Line => 28.0,
                MouseScrollUnit::Pixel => 1.0,
            };
    }
    for (_, computed, mut scroll) in &mut panel {
        let maximum = ((computed.content_size().y - computed.size().y)
            * computed.inverse_scale_factor())
        .max(0.0);
        if over_panel && wheel_delta != 0.0 {
            scroll.y = (scroll.y + wheel_delta).clamp(0.0, maximum);
            ask.stick_bottom = scroll.y >= maximum - 2.0;
        } else if ask.stick_bottom {
            scroll.y = maximum;
        }
    }

    if !window.focused {
        keyboard.clear();
        ui.focused = false;
        return;
    }
    if mouse.just_pressed(MouseButton::Left) {
        if over_mic {
            // The mic button records; it does not focus the text box.
        } else if over_bar {
            ui.focused = true;
        } else if !over_panel {
            ui.focused = false;
        }
    }
    if !ui.focused {
        // Q focuses unless another text field (search) or panel owns the keys.
        if keys.just_pressed(KeyCode::KeyQ) && !lock.0 {
            ui.focused = true;
        }
        keyboard.clear();
    } else {
        for event in keyboard.read().filter(|e| e.state == ButtonState::Pressed) {
            match event.key_code {
                KeyCode::Escape => {
                    ui.focused = false;
                    break;
                }
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    let question = ask.input.trim().to_string();
                    ask.input.clear();
                    if !question.is_empty() {
                        ask.submitted = Some((question, false));
                    }
                    ui.focused = false;
                    break;
                }
                KeyCode::Backspace => {
                    ask.input.pop();
                }
                _ => {
                    if let Some(text) = &event.text {
                        for c in text.chars().filter(|c| !c.is_control()) {
                            if ask.input.len() + c.len_utf8() <= MAX_INPUT {
                                ask.input.push(c);
                            }
                        }
                    }
                }
            }
        }
        keyboard.clear();
    }
    if ui.focused || ui.over {
        lock.0 = true;
    }
}

fn ask_route(
    mut ask: ResMut<Ask>,
    mut served: Option<ResMut<Served>>,
    actions: Option<ResMut<ViewerActions>>,
    status: Res<LayoutStatus>,
    playback: Option<ResMut<Playback>>,
) {
    let Some((question, voice)) = ask.submitted.take() else {
        return;
    };
    let selected = match served.as_deref() {
        Some(served) => ipc::selected_label(served, &status),
        None => status.selected.clone(),
    };
    let route = classify(&question, selected.is_some());

    // A new question replaces whatever is still running.
    if let Some(job) = ask.job.take() {
        let id = job.entry;
        drop(job);
        if let Some(entry) = ask.entry(id)
            && !entry.status.is_empty()
        {
            entry.status = "Stopped for a newer question.".into();
        }
    }
    ask.tour = None;

    let id = ask.next_id;
    ask.next_id += 1;
    let badge = match route {
        Route::Tour {
            also_data: true, ..
        } => BADGE_TOUR_DATA,
        Route::Tour { .. } => BADGE_TOUR,
        Route::Data => BADGE_DATA,
        Route::Claude => BADGE_CLAUDE,
    };
    ask.history.push_back(Entry {
        id,
        question: if voice {
            format!("(voice) {question}")
        } else {
            question.clone()
        },
        badge,
        tour_status: String::new(),
        status: String::new(),
        answer: String::new(),
        error: false,
    });
    while ask.history.len() > HISTORY {
        ask.history.pop_front();
    }
    ask.dirty = true;
    ask.stick_bottom = true;

    let project = ask.project.clone();
    let start_data = |ask: &mut Ask| {
        let command = data_command(&project, &question, selected.as_deref());
        match spawn_job(command, id, Kind::Data) {
            Ok(job) => {
                ask.job = Some(job);
                if let Some(entry) = ask.entry(id) {
                    entry.status = ASKING_CORTEX.into();
                }
            }
            Err(error) => {
                if let Some(entry) = ask.entry(id) {
                    entry.error = true;
                    entry.status = error;
                }
            }
        }
    };

    match route {
        Route::Tour { tour, also_data } => {
            if let Some(entry) = ask.entry(id) {
                entry.answer = tour.intro().into();
            }
            let started = match (served.as_deref_mut(), actions, playback) {
                (Some(served), Some(mut actions), Some(mut playback)) => {
                    start_tour(&project, tour, served, &mut actions, &status, &mut playback)
                }
                _ => Err("Tours need the viewer started with --serve.".into()),
            };
            match started {
                Ok((model_id, walkthrough, title)) => {
                    if let Some(entry) = ask.entry(id) {
                        entry.tour_status = format!("Loading tour: {title}...");
                    }
                    ask.tour = Some(PendingTour {
                        entry: id,
                        model_id,
                        walkthrough,
                        title,
                        since: Instant::now(),
                    });
                }
                Err(error) => {
                    if let Some(entry) = ask.entry(id) {
                        entry.error = true;
                        entry.tour_status = error;
                    }
                }
            }
            if also_data {
                start_data(&mut ask);
            }
        }
        Route::Data => start_data(&mut ask),
        Route::Claude => {
            if !ask.serving {
                if let Some(entry) = ask.entry(id) {
                    entry.error = true;
                    entry.status = "Claude answers need the viewer started with --serve, so its tools can drive this window.".into();
                }
                return;
            }
            let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("visualyze"));
            let claude = find_program(
                "FILAGREE_CLAUDE",
                "claude",
                &[
                    "~/.npm-global/bin/claude",
                    "~/.claude/local/claude",
                    "/opt/homebrew/bin/claude",
                ],
            );
            let mut command = Command::new(claude);
            command.current_dir(&project).args(claude_args(
                &question,
                &exe,
                &project,
                &ask.session,
                ask.session_started,
            ));
            match spawn_job(command, id, Kind::Claude) {
                Ok(job) => {
                    ask.job = Some(job);
                    if let Some(entry) = ask.entry(id) {
                        entry.status = "Claude is thinking...".into();
                    }
                }
                Err(error) => {
                    if let Some(entry) = ask.entry(id) {
                        entry.error = true;
                        entry.status = error;
                    }
                }
            }
        }
    }
}

/// Publish the tour's model if another one is showing; the walkthrough is
/// presented once that model is on screen.
fn start_tour(
    project: &Path,
    tour: Tour,
    served: &mut Served,
    actions: &mut ViewerActions,
    status: &LayoutStatus,
    playback: &mut Playback,
) -> Result<(String, Value, String), String> {
    let dir = project.join(".visualyze");
    let (model_file, walkthrough_file) = tour.files();
    let walkthrough_path = dir.join(walkthrough_file);
    let walkthrough: Value = std::fs::read(&walkthrough_path)
        .map_err(|e| format!("Could not read {}: {e}", walkthrough_path.display()))
        .and_then(|bytes| {
            serde_json::from_slice(&bytes)
                .map_err(|e| format!("{} is not JSON: {e}", walkthrough_path.display()))
        })?;
    let model_id = walkthrough["model_id"]
        .as_str()
        .ok_or("walkthrough has no model_id")?
        .to_string();
    let title = ascii(walkthrough["title"].as_str().unwrap_or("tour"));
    if status.model_id.as_deref() != Some(model_id.as_str()) || served.current(status).is_none() {
        ipc::publish_local(&dir.join(model_file), served, actions, status, playback)?;
    }
    Ok((model_id, walkthrough, title))
}

fn ask_poll(
    mut ask: ResMut<Ask>,
    served: Option<Res<Served>>,
    status: Res<LayoutStatus>,
    playback: Option<ResMut<Playback>>,
) {
    // Present a pending tour once its model is on screen.
    if let (Some(tour), Some(served), Some(mut playback)) = (ask.tour.as_ref(), served, playback) {
        let shown = served
            .current(&status)
            .is_some_and(|m| m.snapshot().model_id == tour.model_id)
            && status.ready
            && !status.loading;
        let outcome = if shown {
            Some(
                ipc::present_local(tour.walkthrough.clone(), &served, &status, &mut playback)
                    .map(|()| format!("Playing tour: {}", tour.title)),
            )
        } else if tour.since.elapsed() > TOUR_TIMEOUT {
            Some(Err(format!(
                "Timed out waiting for {} to show{}",
                tour.model_id,
                status
                    .action_error
                    .as_ref()
                    .or(status.error.as_ref())
                    .map(|e| format!(": {e}"))
                    .unwrap_or_default()
            )))
        } else {
            None
        };
        if let Some(outcome) = outcome {
            let id = tour.entry;
            ask.tour = None;
            ask.dirty = true;
            if let Some(entry) = ask.entry(id) {
                match outcome {
                    Ok(text) => entry.tour_status = text,
                    Err(error) => {
                        entry.error = true;
                        entry.tour_status = ascii(&error);
                    }
                }
            }
        }
    }

    let Some(job) = ask.job.as_ref() else {
        return;
    };
    let (id, kind) = (job.entry, job.kind);
    let msgs: Vec<Msg> = match job.receive.lock() {
        Ok(receive) => receive.try_iter().take(512).collect(),
        Err(_) => Vec::new(),
    };
    if msgs.is_empty() {
        return;
    }
    ask.dirty = true;
    let mut finished = false;
    for msg in msgs {
        if msg == Msg::Started {
            ask.session_started = true;
            continue;
        }
        let Some(entry) = ask.entry(id) else {
            finished = true;
            continue;
        };
        match msg {
            Msg::Started => {}
            Msg::Text(text) => {
                entry.answer.push_str(&ascii(&text));
                entry.status.clear();
            }
            Msg::Break => {
                if !entry.answer.is_empty() && !entry.answer.ends_with('\n') {
                    entry.answer.push('\n');
                }
            }
            Msg::Tool(name) => entry.status = friendly_tool(&name).into(),
            Msg::Result { text, error } => {
                if error {
                    entry.error = true;
                    entry.status = ascii(&format!("Claude stopped: {text}"));
                } else {
                    if entry.answer.trim().is_empty() {
                        entry.answer = ascii(&text);
                    }
                    entry.status.clear();
                }
            }
            Msg::Answer(answer) => {
                let answer = ascii(answer.trim());
                if kind == Kind::Data && !entry.answer.is_empty() {
                    entry.answer.push_str("\n\nCortex: ");
                    entry.answer.push_str(&answer);
                } else {
                    entry.answer = answer;
                }
                entry.status.clear();
            }
            Msg::Exited { ok, detail } => {
                finished = true;
                let answered = match kind {
                    Kind::Claude => !entry.answer.trim().is_empty(),
                    // The answer may follow a tour intro; a cleared status means it arrived.
                    Kind::Data => entry.status.is_empty(),
                };
                if entry.error {
                    // Already explained (for example a Claude error result).
                } else if ok && answered {
                    entry.status.clear();
                } else {
                    let nothing = match (kind, ok) {
                        (Kind::Data, true) => "ask.py gave no answer",
                        (Kind::Data, false) => "ask.py failed",
                        (Kind::Claude, true) => "Claude exited without an answer",
                        (Kind::Claude, false) => "Claude failed",
                    };
                    entry.error = true;
                    entry.status = ascii(&if detail.is_empty() {
                        nothing.to_string()
                    } else {
                        format!("{nothing}:\n{detail}")
                    });
                }
            }
        }
    }
    if finished {
        ask.job = None;
    }
}

#[allow(clippy::too_many_arguments)]
fn ask_render(
    mut commands: Commands,
    mut ask: ResMut<Ask>,
    ui: Res<AskUi>,
    voice: Res<Voice>,
    time: Res<Time<Real>>,
    mut mic: Query<&mut BackgroundColor, With<AskMic>>,
    panel: Query<Entity, With<AskPanel>>,
    mut panel_node: Query<&mut Node, With<AskPanel>>,
    mut bar: Query<&mut BorderColor, With<AskBar>>,
    mut bar_text: Query<(&mut Text, &mut TextColor), With<AskBarText>>,
) {
    // Input bar.
    let cursor = if ((time.elapsed_secs() * 2.0) as u32).is_multiple_of(2) {
        "_"
    } else {
        " "
    };
    let (text, color) = if let Some(recording) = &voice.recording {
        (
            if recording.by_key {
                "Listening... release W to ask".to_string()
            } else {
                "Listening... click MIC to ask".to_string()
            },
            Color::srgb_u8(255, 120, 110),
        )
    } else if voice.transcribing.is_some() {
        ("Transcribing...".into(), Color::srgb_u8(236, 242, 248))
    } else if ui.focused {
        if ask.input.is_empty() {
            (
                format!("{cursor}  Enter asks, Esc cancels"),
                Color::srgb_u8(120, 140, 160),
            )
        } else {
            (
                format!("{}{cursor}", ascii(&ask.input)),
                Color::srgb_u8(236, 242, 248),
            )
        }
    } else if ask.job.is_some() || ask.tour.is_some() {
        (
            "Working... press Q to ask something else".into(),
            Color::srgb_u8(120, 140, 160),
        )
    } else {
        (
            "Press Q to ask, or hold W to talk".into(),
            Color::srgb_u8(120, 140, 160),
        )
    };
    let blink = ((time.elapsed_secs() * 2.5) as u32).is_multiple_of(2);
    for mut background in &mut mic {
        let want = match (&voice.recording, voice.transcribing.is_some()) {
            (Some(_), _) if blink => MIC_RECORDING,
            (Some(_), _) => MIC_RECORDING.with_alpha(0.55),
            (None, true) => ACCENT.with_alpha(0.45),
            _ => MIC_IDLE,
        };
        if background.0 != want {
            background.0 = want;
        }
    }
    for (mut t, mut c) in &mut bar_text {
        if t.0 != text {
            t.0 = text.clone();
        }
        if c.0 != color {
            c.0 = color;
        }
    }
    for mut border in &mut bar {
        let want = BorderColor::all(if ui.focused {
            ACCENT
        } else {
            Color::srgb_u8(60, 78, 96)
        });
        if *border != want {
            *border = want;
        }
    }

    if !ask.dirty {
        return;
    }
    ask.dirty = false;
    for mut node in &mut panel_node {
        node.display = if ask.history.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
    }
    let Ok(panel) = panel.single() else {
        return;
    };
    let font = |size: f32| TextFont {
        font_size: size,
        ..default()
    };
    commands
        .entity(panel)
        .despawn_related::<Children>()
        .with_children(|p| {
            p.spawn((
                Text::new("ASK FILAGREE"),
                font(12.0),
                TextColor(Color::srgb_u8(140, 170, 196)),
            ));
            for entry in &ask.history {
                p.spawn((
                    Text::new(format!("> {}", ascii(&entry.question))),
                    font(15.0),
                    TextColor(ACCENT),
                    Node {
                        margin: UiRect::top(px(8)),
                        ..default()
                    },
                ));
                p.spawn((
                    Text::new(entry.badge),
                    font(10.0),
                    TextColor(Color::srgb_u8(210, 196, 160)),
                    BackgroundColor(Color::srgba(0.12, 0.1, 0.05, 0.75)),
                    Node {
                        align_self: AlignSelf::Start,
                        padding: UiRect::axes(px(6), px(2)),
                        ..default()
                    },
                ));
                if !entry.answer.is_empty() {
                    p.spawn((
                        Text::new(plain(&entry.answer)),
                        font(15.0),
                        TextColor(Color::srgb_u8(232, 238, 244)),
                    ));
                }
                for line in [&entry.tour_status, &entry.status] {
                    if line.is_empty() {
                        continue;
                    }
                    p.spawn((
                        Text::new(line.clone()),
                        font(12.0),
                        TextColor(if entry.error {
                            Color::srgb_u8(240, 140, 120)
                        } else {
                            Color::srgb_u8(140, 170, 196)
                        }),
                    ));
                }
            }
        });
}

// ---------------------------------------------------------------------------
// Voice: push-to-talk W (or the MIC button), ffmpeg records, local Whisper
// transcribes on a worker thread, and the text goes through the same router.

const MIN_RECORDING: Duration = Duration::from_millis(400);
const WHISPER_PROMPT: &str = "Filagree, TPC-H, LINEITEM, ORDERS, PARTSUPP, PartsHub, Snowflake, Cortex, ASK_LAKE, walkthrough";

#[derive(Clone)]
struct VoiceConfig {
    ffmpeg: PathBuf,
    whisper: PathBuf,
    model: PathBuf,
    mic: String,
    /// `FILAGREE_STT=whisperkit` and `whisperkit-cli` was found. Untested.
    whisperkit: Option<PathBuf>,
}

impl VoiceConfig {
    fn from_env() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        let whisperkit = (std::env::var("FILAGREE_STT").as_deref() == Ok("whisperkit"))
            .then(|| {
                find_program(
                    "FILAGREE_WHISPERKIT",
                    "whisperkit-cli",
                    &["/opt/homebrew/bin/whisperkit-cli"],
                )
            })
            .filter(|p| p.is_file());
        Self {
            ffmpeg: find_program(
                "FILAGREE_FFMPEG",
                "ffmpeg",
                &["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"],
            ),
            whisper: find_program(
                "FILAGREE_WHISPER",
                "whisper-cli",
                &[
                    "/opt/homebrew/bin/whisper-cli",
                    "/usr/local/bin/whisper-cli",
                ],
            ),
            model: std::env::var_os("FILAGREE_WHISPER_MODEL")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    home.join(".cache/hyperframes/whisper/models/ggml-small.en.bin")
                }),
            mic: std::env::var("FILAGREE_MIC")
                .ok()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| ":MacBook Pro Microphone".into()),
            whisperkit,
        }
    }

    /// Why voice cannot work, in one line.
    fn problem(&self) -> Option<String> {
        if !self.ffmpeg.is_file() {
            return Some("Voice needs ffmpeg; install it or set FILAGREE_FFMPEG.".into());
        }
        if self.whisperkit.is_some() {
            return None;
        }
        if !self.whisper.is_file() {
            return Some(
                "Voice needs whisper-cli (brew install whisper-cpp) or FILAGREE_WHISPER.".into(),
            );
        }
        if !self.model.is_file() {
            return Some(format!(
                "Whisper model not found at {}; set FILAGREE_WHISPER_MODEL.",
                self.model.display()
            ));
        }
        None
    }

    /// The first cold transcription takes seconds; warm the model up in the background.
    fn prewarm(&self) {
        let sample = Path::new("/opt/homebrew/share/whisper.cpp/jfk.wav");
        if self.whisperkit.is_some() || self.problem().is_some() || !sample.is_file() {
            return;
        }
        let config = self.clone();
        std::thread::spawn(move || {
            let _ = config.transcribe(sample);
        });
    }

    fn transcribe(&self, wav: &Path) -> Result<String, String> {
        let mut command = match &self.whisperkit {
            Some(kit) => {
                let mut command = Command::new(kit);
                command.args(["transcribe", "--audio-path"]).arg(wav);
                command
            }
            None => {
                let mut command = Command::new(&self.whisper);
                command.arg("-m").arg(&self.model).arg("-f").arg(wav).args([
                    "-nt",
                    "-np",
                    "--prompt",
                    WHISPER_PROMPT,
                ]);
                command
            }
        };
        let output = command
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map_err(|e| format!("Could not start the transcriber: {e}"))?;
        if !output.status.success() {
            return Err(format!("Transcription failed ({})", output.status));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

struct Recording {
    child: Child,
    path: PathBuf,
    started: Instant,
    by_key: bool,
}

enum Heard {
    Text(String),
    Nothing,
    Failed(String),
}

#[derive(Resource)]
struct Voice {
    config: VoiceConfig,
    recording: Option<Recording>,
    transcribing: Option<Mutex<mpsc::Receiver<Heard>>>,
    count: u64,
}

impl Drop for Voice {
    fn drop(&mut self) {
        if let Some(recording) = &mut self.recording {
            let _ = recording.child.kill();
            let _ = std::fs::remove_file(&recording.path);
        }
    }
}

/// Whisper output to a question: one line, without non-speech tags such as
/// `[BLANK_AUDIO]`, `(music)` or `*laughs*`. None when nothing was said.
pub(crate) fn clean_transcript(raw: &str) -> Option<String> {
    let mut kept = String::with_capacity(raw.len());
    let mut depth = 0usize;
    let mut starred = false;
    for c in raw.chars() {
        match c {
            '[' | '(' | '{' | '<' => depth += 1,
            ']' | ')' | '}' | '>' => depth = depth.saturating_sub(1),
            '*' => starred = !starred,
            _ if depth == 0 && !starred => kept.push(c),
            _ => {}
        }
    }
    let text = kept.split_whitespace().collect::<Vec<_>>().join(" ");
    let text = text.trim_matches(|c: char| c == '"' || c == '-' || c.is_whitespace());
    text.chars()
        .any(char::is_alphanumeric)
        .then(|| text.to_string())
}

impl Ask {
    /// A one-line problem in the panel, outside any question.
    fn note(&mut self, title: &str, problem: String) {
        let id = self.next_id;
        self.next_id += 1;
        self.history.push_back(Entry {
            id,
            question: title.into(),
            badge: "Voice",
            tour_status: String::new(),
            status: ascii(&problem),
            answer: String::new(),
            error: true,
        });
        while self.history.len() > HISTORY {
            self.history.pop_front();
        }
        self.dirty = true;
        self.stick_bottom = true;
    }
}

fn start_recording(voice: &mut Voice, by_key: bool) -> Result<(), String> {
    if let Some(problem) = voice.config.problem() {
        return Err(problem);
    }
    voice.count += 1;
    let path = std::env::temp_dir().join(format!(
        "filagree-ask-{}-{}.wav",
        std::process::id(),
        voice.count
    ));
    let child = Command::new(&voice.config.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "avfoundation",
            "-i",
        ])
        .arg(&voice.config.mic)
        .args(["-ar", "16000", "-ac", "1", "-y"])
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start ffmpeg: {e}"))?;
    voice.recording = Some(Recording {
        child,
        path,
        started: Instant::now(),
        by_key,
    });
    Ok(())
}

/// Stop ffmpeg gracefully (q finalizes the WAV), then transcribe, all off the main thread.
fn stop_recording(voice: &mut Voice) {
    let Some(Recording {
        mut child,
        path,
        started,
        ..
    }) = voice.recording.take()
    else {
        return;
    };
    let too_short = started.elapsed() < MIN_RECORDING;
    let config = voice.config.clone();
    let (send, receive) = mpsc::channel();
    voice.transcribing = Some(Mutex::new(receive));
    std::thread::spawn(move || {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"q\n");
        }
        let deadline = Instant::now() + Duration::from_millis(1500);
        loop {
            match child.try_wait() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) if Instant::now() > deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(30)),
            }
        }
        let mut errors = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.read_to_string(&mut errors);
        }
        let heard = if too_short {
            Heard::Nothing
        } else if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) <= 44 {
            let line = last_lines(&errors, 1);
            Heard::Failed(if line.is_empty() {
                "No audio was recorded".into()
            } else {
                format!("No audio was recorded: {line}")
            })
        } else {
            match config.transcribe(&path) {
                Ok(raw) => clean_transcript(&raw).map_or_else(
                    || Heard::Failed("Did not catch that; hold W and try again.".into()),
                    Heard::Text,
                ),
                Err(error) => Heard::Failed(error),
            }
        };
        let _ = std::fs::remove_file(&path);
        let _ = send.send(heard);
    });
}

#[allow(clippy::too_many_arguments)]
fn ask_voice(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    lock: Res<CameraInputLock>,
    ui: Res<AskUi>,
    mic: Query<&Interaction, With<AskMic>>,
    mut voice: ResMut<Voice>,
    mut ask: ResMut<Ask>,
) {
    // A finished transcription becomes a question.
    let heard = voice
        .transcribing
        .as_ref()
        .and_then(|r| r.lock().ok().and_then(|r| r.try_recv().ok()));
    if let Some(heard) = heard {
        voice.transcribing = None;
        match heard {
            Heard::Text(text) => {
                ask.input.clear();
                ask.submitted = Some((text, true));
            }
            Heard::Nothing => {}
            Heard::Failed(error) => ask.note("(voice)", error),
        }
    }

    // ffmpeg quit on its own (no microphone, permission denied).
    let died = voice
        .recording
        .as_mut()
        .is_some_and(|r| matches!(r.child.try_wait(), Ok(Some(_))));
    if died && let Some(mut recording) = voice.recording.take() {
        let mut errors = String::new();
        if let Some(mut stderr) = recording.child.stderr.take() {
            let _ = stderr.read_to_string(&mut errors);
        }
        let _ = std::fs::remove_file(&recording.path);
        let line = last_lines(&errors, 1);
        let detail = if line.is_empty() {
            format!("ffmpeg could not open {}", voice.config.mic)
        } else {
            line
        };
        ask.note("(voice)", format!("Recording stopped: {detail}"));
    }

    let clicked =
        mouse.just_pressed(MouseButton::Left) && mic.iter().any(|i| *i != Interaction::None);
    let key_free = window.focused && !ui.focused && (!lock.0 || ui.over);
    match &voice.recording {
        None if voice.transcribing.is_none() => {
            let by_key = key_free && keys.just_pressed(KeyCode::KeyW);
            if (by_key || clicked)
                && let Err(error) = start_recording(&mut voice, by_key)
            {
                ask.note("(voice)", error);
            }
        }
        Some(recording) => {
            let release = if recording.by_key {
                !keys.pressed(KeyCode::KeyW) || !window.focused
            } else {
                clicked
            };
            if release {
                stop_recording(&mut voice);
            }
        }
        None => {}
    }
}

/// Claude sometimes writes markdown anyway; the panel shows plain text.
fn plain(answer: &str) -> String {
    answer
        .replace("**", "")
        .replace("__", "")
        .lines()
        .map(|line| line.trim_start_matches('#').trim_start())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tour(t: Tour) -> Route {
        Route::Tour {
            tour: t,
            also_data: false,
        }
    }

    #[test]
    fn tours_are_matched_by_keyword() {
        assert_eq!(
            classify("What happens when a customer orders a part?", false),
            tour(Tour::CustomerOrder)
        );
        assert_eq!(
            classify("Why was this order late?", false),
            tour(Tour::LateOrder)
        );
        assert_eq!(
            classify("What if orders streamed in instead of nightly?", false),
            tour(Tour::ProposalStreaming)
        );
        assert_eq!(
            classify("How does a question get answered?", false),
            tour(Tour::QuestionToAnswer)
        );
        assert_eq!(
            classify("Walk me through a spoken question", false),
            tour(Tour::QuestionToAnswer)
        );
        assert_eq!(
            classify("Which region earned the most in 1997?", false),
            Route::Tour {
                tour: Tour::Region1997,
                also_data: true
            }
        );
    }

    #[test]
    fn late_is_a_word_not_a_substring() {
        assert_eq!(
            classify("How do these tables relate?", false),
            Route::Claude
        );
        assert_eq!(classify("Translate the plan", false), Route::Claude);
    }

    #[test]
    fn number_questions_go_to_cortex() {
        assert_eq!(
            classify("Which supplier ships slowest?", false),
            Route::Data
        );
        assert_eq!(
            classify("How many customers are there?", false),
            Route::Data
        );
        assert_eq!(classify("What was total revenue?", false), Route::Data);
        assert_eq!(classify("Average order size", false), Route::Data);
        assert_eq!(classify("What is in this table?", true), Route::Data);
        assert_eq!(classify("What is in this table?", false), Route::Claude);
    }

    #[test]
    fn everything_else_goes_to_claude() {
        assert_eq!(
            classify("Explain the codebase and the data it represents", false),
            Route::Claude
        );
        assert_eq!(classify("almost done?", false), Route::Claude);
    }

    #[test]
    fn claude_stream_lines_parse() {
        let init = r#"{"type":"system","subtype":"init","cwd":"/x","session_id":"s"}"#;
        assert_eq!(parse_claude_line(init), Some(Msg::Started));
        let start = r#"{"type":"stream_event","event":{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}},"session_id":"s"}"#;
        assert_eq!(parse_claude_line(start), Some(Msg::Break));
        let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello, gl"}},"session_id":"s"}"#;
        assert_eq!(
            parse_claude_line(delta),
            Some(Msg::Text("Hello, gl".into()))
        );
        let json_delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{"}}}"#;
        assert_eq!(parse_claude_line(json_delta), None);
        let tool = r#"{"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"t","name":"mcp__visualyze__present_walkthrough","input":{}}}}"#;
        assert_eq!(
            parse_claude_line(tool),
            Some(Msg::Tool("mcp__visualyze__present_walkthrough".into()))
        );
        let result = r#"{"duration_api_ms":2881,"is_error":false,"subtype":"success","result":"Done.","type":"result"}"#;
        assert_eq!(
            parse_claude_line(result),
            Some(Msg::Result {
                text: "Done.".into(),
                error: false
            })
        );
        let assistant =
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"x"}]}}"#;
        assert_eq!(parse_claude_line(assistant), None);
        assert_eq!(parse_claude_line("not json"), None);
    }

    #[test]
    fn ask_py_answer_parses() {
        let line = r#"{"question": "q", "focus_table": null, "answer": "Five regions."}"#;
        assert_eq!(
            parse_data_line(line),
            Some(Msg::Answer("Five regions.".into()))
        );
        assert_eq!(parse_data_line("  options = self.config"), None);
    }

    #[test]
    fn friendly_tool_names() {
        assert_eq!(
            friendly_tool("mcp__visualyze__get_scene_state"),
            "Reading scene..."
        );
        assert_eq!(friendly_tool("Bash"), "Asking Snowflake...");
    }

    #[test]
    fn text_is_made_ascii_for_the_default_font() {
        assert_eq!(
            ascii("a \u{2014} b \u{201C}c\u{201D}\u{2026}"),
            "a - b \"c\"..."
        );
        assert_eq!(ascii("caf\u{e9}"), "caf?");
    }

    #[test]
    fn uuid_has_v4_shape() {
        let id = new_uuid();
        assert_eq!(id.len(), 36);
        assert_eq!(&id[14..15], "4");
        assert_ne!(new_uuid(), id);
    }

    #[test]
    fn claude_args_put_the_variadic_tools_last_and_resume_later_turns() {
        let args = claude_args("hi", Path::new("/bin/v"), Path::new("/p"), "u", false);
        assert_eq!(&args[..2], ["-p", "hi"]);
        assert!(args.windows(2).any(|w| w == ["--session-id", "u"]));
        assert_eq!(args.last().map(String::as_str), Some("Bash(uv run *)"));
        let resumed = claude_args("hi", Path::new("/bin/v"), Path::new("/p"), "u", true);
        assert!(resumed.windows(2).any(|w| w == ["--resume", "u"]));
        let mcp: Value = serde_json::from_str(&args[8]).unwrap();
        assert_eq!(
            mcp["mcpServers"]["visualyze"]["args"],
            json!(["mcp", "--root", "/p"])
        );
    }

    #[test]
    fn transcripts_drop_non_speech_and_blank_audio() {
        assert_eq!(clean_transcript(" [BLANK_AUDIO]\n"), None);
        assert_eq!(clean_transcript("(music)"), None);
        assert_eq!(clean_transcript(" *laughs* "), None);
        assert_eq!(clean_transcript(""), None);
        assert_eq!(clean_transcript(" - ... "), None);
        assert_eq!(
            clean_transcript("\n Which region earned the most\n in 1997? [BLANK_AUDIO]\n"),
            Some("Which region earned the most in 1997?".into())
        );
        assert_eq!(
            clean_transcript(" (wind blowing) What if orders streamed?"),
            Some("What if orders streamed?".into())
        );
    }

    /// Needs whisper-cli, the model and the sample; run with --ignored.
    #[test]
    #[ignore]
    fn local_whisper_transcribes_the_sample() {
        let config = VoiceConfig::from_env();
        assert_eq!(config.problem(), None);
        let raw = config
            .transcribe(Path::new("/opt/homebrew/share/whisper.cpp/jfk.wav"))
            .unwrap();
        let text = clean_transcript(&raw).unwrap();
        assert!(text.contains("ask not what your country"), "{text}");
    }

    #[test]
    fn markdown_is_stripped_for_the_panel() {
        assert_eq!(plain("## Title\n**Bold** text"), "Title\nBold text");
    }
}
