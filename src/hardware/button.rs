use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{self, BufReader};
use std::num::{NonZeroU16, NonZeroU32};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::db::admin::pomodoro::{runtime_enabled_settings, PomodoroSettings};
use crate::events::types::{ButtonBehavior, ButtonEvent, ButtonMapping, ContentPack, Response};
use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::{Parser, ValueEnum};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;
use ratatui::Terminal;
use rodio::{ChannelCount, SampleRate, Source};
use rusqlite::{params, Connection};

const POMODORO_FOCUS_LABEL: &str = "generated binaural focus tone";
const FOCUS_SAMPLE_RATE_HZ: u32 = 44_100;
const FOCUS_CHANNELS: u16 = 2;
const FOCUS_LEFT_HZ: f32 = 220.0;
const FOCUS_RIGHT_HZ: f32 = 226.0;
const FOCUS_VOLUME: f32 = 0.10;
const FOCUS_FADE_SECONDS: f32 = 3.0;
const FOCUS_SLOW_MOD_HZ: f32 = 0.035;
#[allow(dead_code)]
const POMODORO_COMBO_BUTTONS: [u8; 3] = [1, 2, 4];
#[allow(dead_code)]
const POMODORO_CHORD_ARM_WINDOW: Duration = Duration::from_millis(180);
#[allow(dead_code)]
const POMODORO_HOLD_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug, Parser)]
#[command(about = "T-Cube child-facing device runtime")]
struct Cli {
    #[arg(long, value_enum, default_value_t = Backend::Sim)]
    backend: Backend,

    #[arg(long, default_value = "content/content.json")]
    content: PathBuf,

    #[arg(long, default_value = "data/tcube.sqlite3")]
    database: PathBuf,

    #[arg(long, value_enum, default_value_t = AudioBackend::Terminal)]
    audio: AudioBackend,

    #[arg(long, default_value = ".")]
    audio_root: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Backend {
    Sim,
    Pi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum AudioBackend {
    Terminal,
    Local,
}

impl ContentPack {
    fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read content pack at {}", path.display()))?;
        let pack: Self = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse content pack at {}", path.display()))?;
        pack.validate()?;
        Ok(pack)
    }

    fn validate(&self) -> Result<()> {
        let mut seen = HashMap::new();
        for mode_content in &self.modes {
            if mode_content.responses.is_empty() {
                bail!("mode {} has no responses", mode_content.mode);
            }
            if seen.insert(mode_content.mode.clone(), true).is_some() {
                bail!("mode {} is defined more than once", mode_content.mode);
            }
        }

        if self.button_mappings.is_empty() {
            self.validate_legacy_modes(&seen)?;
        } else {
            for mapping in &self.button_mappings {
                if !(1..=5).contains(&mapping.button_id) {
                    bail!("unsupported button id {}", mapping.button_id);
                }
                if matches!(
                    mapping.behavior,
                    ButtonBehavior::Language | ButtonBehavior::Animals | ButtonBehavior::Music
                ) {
                    let mode = mapping.mode.as_ref().with_context(|| {
                        format!("button {} is missing a mode", mapping.button_id)
                    })?;
                    if !seen.contains_key(mode) {
                        bail!(
                            "button {} references missing mode {}",
                            mapping.button_id,
                            mode
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_legacy_modes(&self, seen: &HashMap<String, bool>) -> Result<()> {
        for mode in ["english", "vietnamese", "french", "animal_sounds", "music"] {
            if !seen.contains_key(mode) {
                bail!("content pack is missing mode {}", mode);
            }
        }
        Ok(())
    }

    fn response_for(&self, mode: &str, index: usize) -> Option<&Response> {
        self.modes
            .iter()
            .find(|mode_content| mode_content.mode == mode)
            .and_then(|mode_content| {
                let responses = &mode_content.responses;
                responses.get(index % responses.len())
            })
    }

    fn mapping_for(&self, button_id: u8) -> Result<ButtonMapping> {
        if let Some(mapping) = self
            .button_mappings
            .iter()
            .find(|mapping| mapping.button_id == button_id)
        {
            return Ok(mapping.clone());
        }

        let mode = match button_id {
            1 => "english",
            2 => "vietnamese",
            3 => "french",
            4 => "animal_sounds",
            5 => "music",
            _ => bail!("unsupported button id {}", button_id),
        };
        Ok(ButtonMapping {
            button_id,
            behavior: match button_id {
                4 => ButtonBehavior::Animals,
                5 => ButtonBehavior::Music,
                _ => ButtonBehavior::Language,
            },
            mode: Some(mode.to_string()),
        })
    }

    fn setup_help_response(&self) -> Response {
        let text = match &self.dashboard_ip {
            Some(ip) => format!("{} {}", self.setup_help_text, ip),
            None => self.setup_help_text.clone(),
        };
        Response {
            id: "setup-help".to_string(),
            text,
            audio_path: None,
        }
    }
}

#[derive(Clone, Debug)]
struct ButtonPress {
    button_id: u8,
    behavior: ButtonBehavior,
    mode: Option<String>,
}

#[derive(Clone, Debug)]
struct SetupDebugEvent {
    event_type: String,
    button_id: u8,
    details: String,
}

trait ButtonInput {
    fn next_press(&mut self) -> Result<InputEvent>;
    fn feedback(&mut self, _feedback: DeviceFeedback) -> Result<()> {
        Ok(())
    }
    fn wait_for_pomodoro_cancel(&mut self, duration: Duration) -> Result<bool> {
        thread::sleep(duration);
        Ok(false)
    }
}

trait AudioOutput {
    fn play(&self, response: &Response) -> Result<AudioPlayback>;
    fn stop(&self) -> Result<()> {
        Ok(())
    }
    fn play_chime(&self, _chime: PomodoroChime) -> Result<()> {
        Ok(())
    }
    fn play_focus(&self, _duration: Duration) -> Result<()> {
        Ok(())
    }
}

trait LedOutput {
    fn pulse(&self, label: &str) -> Result<()>;
    fn blink_inactive(&self) -> Result<()>;
}

enum InputEvent {
    Button(ButtonPress),
    PomodoroShortcut,
    Quit,
}

#[derive(Clone, Debug)]
struct AudioPlayback {
    resolved_path: Option<PathBuf>,
    source_path: Option<String>,
}

#[derive(Clone, Debug)]
enum DeviceFeedback {
    Playback {
        occurred_at: String,
        button_id: u8,
        mode: String,
        response: Response,
        audio: AudioPlayback,
    },
    Pomodoro {
        label: String,
        detail: String,
    },
    Led {
        label: String,
        state: LedFeedbackState,
    },
    Quit,
}

#[derive(Clone, Debug)]
enum LedFeedbackState {
    Pulse,
    Inactive,
}

struct TerminalButtonInput {
    content: ContentPack,
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    state: TuiState,
}

impl TerminalButtonInput {
    fn new(content: ContentPack) -> Result<Self> {
        enable_raw_mode().context("failed to enable terminal raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("failed to enter terminal screen")?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).context("failed to initialize simulator TUI")?;
        terminal.clear().context("failed to clear simulator TUI")?;
        let state = TuiState::new(&content);
        Ok(Self {
            content,
            terminal,
            state,
        })
    }

    fn draw(&mut self) -> Result<()> {
        self.state.frame_count = self.state.frame_count.wrapping_add(1);
        self.terminal
            .draw(|frame| render_tui(frame, &self.state))
            .context("failed to draw simulator TUI")?;
        Ok(())
    }
}

impl Drop for TerminalButtonInput {
    fn drop(&mut self) {
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
        let _ = self.terminal.show_cursor();
    }
}

impl ButtonInput for TerminalButtonInput {
    fn next_press(&mut self) -> Result<InputEvent> {
        loop {
            self.draw()?;
            if event::poll(Duration::from_millis(80)).context("failed to poll terminal input")? {
                if let Event::Key(key) = event::read().context("failed to read terminal input")? {
                    if key.kind == KeyEventKind::Release {
                        continue;
                    }
                    return match key.code {
                        KeyCode::Char('1') => {
                            self.state.note_key_press(1);
                            Ok(InputEvent::Button(button_press(1, &self.content)?))
                        }
                        KeyCode::Char('2') => {
                            self.state.note_key_press(2);
                            Ok(InputEvent::Button(button_press(2, &self.content)?))
                        }
                        KeyCode::Char('3') => {
                            self.state.note_key_press(3);
                            Ok(InputEvent::Button(button_press(3, &self.content)?))
                        }
                        KeyCode::Char('4') => {
                            self.state.note_key_press(4);
                            Ok(InputEvent::Button(button_press(4, &self.content)?))
                        }
                        KeyCode::Char('5') => {
                            self.state.note_key_press(5);
                            Ok(InputEvent::Button(button_press(5, &self.content)?))
                        }
                        KeyCode::Char('p') => {
                            self.state.note_pomodoro_shortcut();
                            Ok(InputEvent::PomodoroShortcut)
                        }
                        KeyCode::Char('q') | KeyCode::Esc => Ok(InputEvent::Quit),
                        _ => continue,
                    };
                }
            }
        }
    }

    fn feedback(&mut self, feedback: DeviceFeedback) -> Result<()> {
        self.state.apply_feedback(feedback);
        self.draw()
    }

    fn wait_for_pomodoro_cancel(&mut self, duration: Duration) -> Result<bool> {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            self.draw()?;
            let remaining = deadline.saturating_duration_since(Instant::now());
            let poll_for = remaining.min(Duration::from_millis(100));
            if event::poll(poll_for).context("failed to poll terminal input")? {
                if let Event::Key(key) = event::read().context("failed to read terminal input")? {
                    if key.kind == KeyEventKind::Release {
                        continue;
                    }
                    if matches!(key.code, KeyCode::Char('p') | KeyCode::Esc) {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }
}

#[derive(Clone, Debug)]
struct ButtonCardState {
    button_id: u8,
    label: String,
    behavior: ButtonBehavior,
}

#[derive(Clone, Debug)]
struct TuiLogEntry {
    occurred_at: String,
    button_id: Option<u8>,
    title: String,
    detail: String,
    color: Color,
}

#[derive(Clone, Debug)]
struct TuiState {
    buttons: Vec<ButtonCardState>,
    dashboard: String,
    frame_count: u64,
    last_audio_path: String,
    last_button: Option<u8>,
    last_key_at: Option<String>,
    last_led: String,
    last_mode: String,
    last_response: String,
    logs: Vec<TuiLogEntry>,
    started_at: Instant,
}

impl TuiState {
    fn new(content: &ContentPack) -> Self {
        let buttons = (1..=5)
            .map(|button_id| match content.mapping_for(button_id) {
                Ok(mapping) => ButtonCardState {
                    button_id,
                    label: mapping_summary_label(&mapping),
                    behavior: mapping.behavior,
                },
                Err(error) => ButtonCardState {
                    button_id,
                    label: format!("unsupported: {error}"),
                    behavior: ButtonBehavior::Disabled,
                },
            })
            .collect();
        let dashboard = match &content.dashboard_ip {
            Some(ip) => format!("{} ({ip})", content.dashboard_host),
            None => content.dashboard_host.clone(),
        };

        Self {
            buttons,
            dashboard,
            frame_count: 0,
            last_audio_path: "waiting for button press".to_string(),
            last_button: None,
            last_key_at: None,
            last_led: "idle".to_string(),
            last_mode: "ready".to_string(),
            last_response: "Press 1-5 to trigger the cube.".to_string(),
            logs: Vec::new(),
            started_at: Instant::now(),
        }
    }

    fn apply_feedback(&mut self, feedback: DeviceFeedback) {
        match feedback {
            DeviceFeedback::Playback {
                occurred_at,
                button_id,
                mode,
                response,
                audio,
            } => {
                self.last_button = Some(button_id);
                self.last_key_at = Some(occurred_at.clone());
                self.last_mode = mode;
                self.last_response = response.text.clone();
                self.last_audio_path = match audio.resolved_path {
                    Some(path) => path.display().to_string(),
                    None => audio
                        .source_path
                        .unwrap_or_else(|| "no local audio asset".to_string()),
                };
                self.push_log(TuiLogEntry {
                    occurred_at,
                    button_id: Some(button_id),
                    title: format!("Button {button_id}"),
                    detail: format!("{} | {}", response.id, self.last_audio_path),
                    color: button_color(button_id),
                });
            }
            DeviceFeedback::Led { label, state } => {
                self.last_led = match state {
                    LedFeedbackState::Pulse => format!("pulse {label}"),
                    LedFeedbackState::Inactive => "inactive blink".to_string(),
                };
            }
            DeviceFeedback::Pomodoro { label, detail } => {
                self.last_mode = label.clone();
                self.last_response = detail.clone();
                self.last_audio_path = POMODORO_FOCUS_LABEL.to_string();
                self.last_led = "button LEDs off".to_string();
                self.push_log(TuiLogEntry {
                    occurred_at: Utc::now().format("%H:%M:%S%.3f").to_string(),
                    button_id: None,
                    title: label,
                    detail,
                    color: Color::LightCyan,
                });
            }
            DeviceFeedback::Quit => {
                self.push_log(TuiLogEntry {
                    occurred_at: Utc::now().format("%H:%M:%S%.3f").to_string(),
                    button_id: None,
                    title: "Shutdown".to_string(),
                    detail: "simulator stopped".to_string(),
                    color: Color::Gray,
                });
            }
        }
    }

    fn note_key_press(&mut self, button_id: u8) {
        self.last_button = Some(button_id);
        self.last_key_at = Some(Utc::now().format("%H:%M:%S%.3f").to_string());
    }

    fn note_pomodoro_shortcut(&mut self) {
        self.last_button = None;
        self.last_key_at = Some(Utc::now().format("%H:%M:%S%.3f").to_string());
    }

    fn push_log(&mut self, entry: TuiLogEntry) {
        self.logs.insert(0, entry);
        self.logs.truncate(8);
    }
}

fn render_tui(frame: &mut Frame, state: &TuiState) {
    let area = frame.size();
    let shell = Block::default()
        .title(" T-CUBE DEVICE SIM ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .style(Style::default().bg(Color::Black));
    frame.render_widget(shell, area);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(9),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .margin(2)
        .split(area);

    render_header(frame, outer[0], state);
    render_button_menu(frame, outer[1], state);
    render_activity(frame, outer[2], state);
    render_footer(frame, outer[3], state);
}

fn render_header(frame: &mut Frame, area: Rect, state: &TuiState) {
    let uptime = state.started_at.elapsed().as_secs();
    let pulse = if state.frame_count % 12 < 6 {
        "◆"
    } else {
        "◇"
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(
                pulse,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                "T-Cube Simulator",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(
                "screen-free child runtime",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::styled("Dashboard ", Style::default().fg(Color::DarkGray)),
            Span::styled(&state.dashboard, Style::default().fg(Color::LightCyan)),
            Span::raw("   "),
            Span::styled("Uptime ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{uptime}s"), Style::default().fg(Color::Green)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        area,
    );
}

fn render_button_menu(frame: &mut Frame, area: Rect, state: &TuiState) {
    let constraints = vec![Constraint::Percentage(20); 5];
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (index, button) in state.buttons.iter().enumerate() {
        let color = button_color(button.button_id);
        let active = state.last_button == Some(button.button_id);
        let border = if active { color } else { Color::DarkGray };
        let gauge_value = if active {
            ((state.frame_count % 10) as u16 + 1) * 10
        } else {
            18
        };
        let title = format!(" {} ", button.button_id);
        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border));
        let label = Line::from(vec![
            Span::styled(
                behavior_label(&button.behavior),
                Style::default().fg(Color::Gray),
            ),
            Span::raw(" "),
            Span::styled(
                button.label.to_uppercase(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        let inner = block.inner(chunks[index]);
        frame.render_widget(block, chunks[index]);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(2)])
            .margin(1)
            .split(inner);
        frame.render_widget(
            Paragraph::new(label)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            rows[0],
        );
        frame.render_widget(
            Gauge::default()
                .gauge_style(Style::default().fg(color))
                .ratio(f64::from(gauge_value) / 100.0)
                .label(if active { "active" } else { "ready" }),
            rows[1],
        );
    }
}

fn render_activity(frame: &mut Frame, area: Rect, state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);
    let key_time = state.last_key_at.as_deref().unwrap_or("none yet");
    let button = state
        .last_button
        .map(|id| format!("Button {id}"))
        .unwrap_or_else(|| "waiting".to_string());
    let lines = vec![
        Line::from(vec![
            Span::styled("Last key ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                button,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" at "),
            Span::styled(key_time, Style::default().fg(Color::LightYellow)),
        ]),
        Line::from(vec![
            Span::styled("Mode      ", Style::default().fg(Color::DarkGray)),
            Span::styled(&state.last_mode, Style::default().fg(Color::LightCyan)),
        ]),
        Line::from(vec![
            Span::styled("Response  ", Style::default().fg(Color::DarkGray)),
            Span::styled(&state.last_response, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Sound     ", Style::default().fg(Color::DarkGray)),
            Span::styled(&state.last_audio_path, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("LED       ", Style::default().fg(Color::DarkGray)),
            Span::styled(&state.last_led, Style::default().fg(Color::Magenta)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(" Live Feedback ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        chunks[0],
    );

    let log_items = state.logs.iter().map(|entry| {
        let button = entry
            .button_id
            .map(|id| format!("#{id}"))
            .unwrap_or_else(|| "--".to_string());
        ListItem::new(Line::from(vec![
            Span::styled(&entry.occurred_at, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(
                button,
                Style::default()
                    .fg(entry.color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(&entry.title, Style::default().fg(Color::White)),
            Span::raw(" "),
            Span::styled(&entry.detail, Style::default().fg(Color::Gray)),
        ]))
    });
    frame.render_widget(
        List::new(log_items).block(
            Block::default()
                .title(" Event Stream ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        chunks[1],
    );
}

fn render_footer(frame: &mut Frame, area: Rect, state: &TuiState) {
    let spinner = ["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"][(state.frame_count as usize) % 8];
    let line = Line::from(vec![
        Span::styled(spinner, Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(
            "Press 1-5",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" to play mapped content   "),
        Span::styled(
            "p",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" focus routine   "),
        Span::styled(
            "q / Esc",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" to quit"),
    ]);
    frame.render_widget(
        Paragraph::new(line).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        ),
        area,
    );
}

fn button_color(button_id: u8) -> Color {
    match button_id {
        1 => Color::Cyan,
        2 => Color::Magenta,
        3 => Color::Yellow,
        4 => Color::Red,
        5 => Color::LightBlue,
        _ => Color::Gray,
    }
}

fn behavior_label(behavior: &ButtonBehavior) -> &'static str {
    match behavior {
        ButtonBehavior::Language => "LANG",
        ButtonBehavior::Animals => "ANIMAL",
        ButtonBehavior::Music => "MUSIC",
        ButtonBehavior::Disabled => "OFF",
        ButtonBehavior::SetupHelp => "SETUP",
    }
}

struct TerminalAudioOutput;

impl AudioOutput for TerminalAudioOutput {
    fn play(&self, response: &Response) -> Result<AudioPlayback> {
        Ok(AudioPlayback {
            resolved_path: None,
            source_path: response.audio_path.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PomodoroChime {
    Start,
    BreakStart,
    BreakEnd,
    Complete,
    Cancel,
}

struct LocalAudioOutput {
    _sink_handle: rodio::MixerDeviceSink,
    player: Mutex<rodio::Player>,
    audio_root: PathBuf,
}

impl LocalAudioOutput {
    fn new(audio_root: PathBuf) -> Result<Self> {
        let sink_handle = rodio::DeviceSinkBuilder::open_default_sink()
            .context("failed to open default audio output device")?;
        let player = rodio::Player::connect_new(sink_handle.mixer());
        Ok(Self {
            _sink_handle: sink_handle,
            player: Mutex::new(player),
            audio_root,
        })
    }
}

impl AudioOutput for LocalAudioOutput {
    fn play(&self, response: &Response) -> Result<AudioPlayback> {
        let Some(path) = audio_asset_path(response, &self.audio_root) else {
            return Ok(AudioPlayback {
                resolved_path: None,
                source_path: response.audio_path.clone(),
            });
        };

        let source = decode_audio_file(&path)?;
        let player = self
            .player
            .lock()
            .map_err(|_| anyhow::anyhow!("audio player lock was poisoned"))?;

        player.stop();
        player.append(source);
        Ok(AudioPlayback {
            resolved_path: Some(path),
            source_path: response.audio_path.clone(),
        })
    }

    fn stop(&self) -> Result<()> {
        let player = self
            .player
            .lock()
            .map_err(|_| anyhow::anyhow!("audio player lock was poisoned"))?;
        player.stop();
        Ok(())
    }

    fn play_chime(&self, chime: PomodoroChime) -> Result<()> {
        let player = self
            .player
            .lock()
            .map_err(|_| anyhow::anyhow!("audio player lock was poisoned"))?;
        player.stop();
        for frequency in chime_frequencies(chime) {
            player.append(
                rodio::source::SineWave::new(*frequency)
                    .take_duration(Duration::from_millis(180))
                    .amplify(0.12),
            );
        }
        player.sleep_until_end();
        Ok(())
    }

    fn play_focus(&self, duration: Duration) -> Result<()> {
        let source = BinauralFocusSource::new(duration);
        let player = self
            .player
            .lock()
            .map_err(|_| anyhow::anyhow!("audio player lock was poisoned"))?;
        player.stop();
        player.append(source);
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct BinauralFocusSource {
    sample_index: u64,
    total_samples: u64,
    duration: Duration,
}

impl BinauralFocusSource {
    fn new(duration: Duration) -> Self {
        let frames = duration
            .as_secs()
            .saturating_mul(u64::from(FOCUS_SAMPLE_RATE_HZ))
            .saturating_add(
                u64::from(duration.subsec_nanos()).saturating_mul(u64::from(FOCUS_SAMPLE_RATE_HZ))
                    / 1_000_000_000,
            );
        Self {
            sample_index: 0,
            total_samples: frames.saturating_mul(u64::from(FOCUS_CHANNELS)),
            duration,
        }
    }
}

impl Iterator for BinauralFocusSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.sample_index >= self.total_samples {
            return None;
        }

        let channel = self.sample_index % u64::from(FOCUS_CHANNELS);
        let frame = self.sample_index / u64::from(FOCUS_CHANNELS);
        let total_frames = self.total_samples / u64::from(FOCUS_CHANNELS);
        let t = frame as f32 / FOCUS_SAMPLE_RATE_HZ as f32;
        let frequency = if channel == 0 {
            FOCUS_LEFT_HZ
        } else {
            FOCUS_RIGHT_HZ
        };
        let fade_in = (t / FOCUS_FADE_SECONDS).clamp(0.0, 1.0);
        let remaining_t = total_frames.saturating_sub(frame) as f32 / FOCUS_SAMPLE_RATE_HZ as f32;
        let fade_out = (remaining_t / FOCUS_FADE_SECONDS).clamp(0.0, 1.0);
        let fade = fade_in.min(fade_out);
        let modulation = 0.92 + 0.08 * (std::f32::consts::TAU * FOCUS_SLOW_MOD_HZ * t).sin();
        let sample =
            (std::f32::consts::TAU * frequency * t).sin() * FOCUS_VOLUME * fade * modulation;
        self.sample_index = self.sample_index.wrapping_add(1);
        Some(sample)
    }
}

impl Source for BinauralFocusSource {
    fn current_span_len(&self) -> Option<usize> {
        let remaining = self.total_samples.saturating_sub(self.sample_index);
        Some(remaining.min(usize::MAX as u64) as usize)
    }

    fn channels(&self) -> ChannelCount {
        NonZeroU16::new(FOCUS_CHANNELS).expect("focus channel count is nonzero")
    }

    fn sample_rate(&self) -> SampleRate {
        NonZeroU32::new(FOCUS_SAMPLE_RATE_HZ).expect("focus sample rate is nonzero")
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.duration)
    }
}

fn chime_frequencies(chime: PomodoroChime) -> &'static [f32] {
    match chime {
        PomodoroChime::Start => &[440.0, 554.37, 659.25],
        PomodoroChime::BreakStart => &[659.25, 554.37],
        PomodoroChime::BreakEnd => &[523.25, 659.25],
        PomodoroChime::Complete => &[523.25, 659.25, 783.99],
        PomodoroChime::Cancel => &[392.0, 329.63],
    }
}

fn decode_audio_file(path: &Path) -> Result<rodio::Decoder<BufReader<File>>> {
    let file = File::open(path)
        .with_context(|| format!("failed to open audio asset {}", path.display()))?;
    rodio::Decoder::try_from(BufReader::new(file))
        .with_context(|| format!("failed to decode audio asset {}", path.display()))
}

fn audio_asset_path(response: &Response, audio_root: &Path) -> Option<PathBuf> {
    response
        .audio_path
        .as_deref()
        .map(|path| resolve_audio_path(path, audio_root))
}

fn resolve_audio_path(audio_path: &str, audio_root: &Path) -> PathBuf {
    let path = Path::new(audio_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        audio_root.join(path)
    }
}

struct TerminalLedOutput;

impl LedOutput for TerminalLedOutput {
    fn pulse(&self, _label: &str) -> Result<()> {
        Ok(())
    }

    fn blink_inactive(&self) -> Result<()> {
        Ok(())
    }
}

enum StoreEvent {
    Button(ButtonEvent),
    SetupDebug(SetupDebugEvent),
}

struct EventStore {
    sender: mpsc::Sender<StoreEvent>,
    worker: Option<thread::JoinHandle<Result<()>>>,
}

impl EventStore {
    fn start(database_path: PathBuf) -> Result<Self> {
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create database directory {}", parent.display())
            })?;
        }

        let (sender, receiver) = mpsc::channel::<StoreEvent>();
        let worker = thread::spawn(move || {
            let conn = Connection::open(&database_path).with_context(|| {
                format!("failed to open SQLite database {}", database_path.display())
            })?;
            init_schema(&conn)?;

            for event in receiver {
                match event {
                    StoreEvent::Button(event) => {
                        conn.execute(
                            "insert into button_events \
                             (occurred_at, button_id, mode, response_id, response_text) \
                             values (?1, ?2, ?3, ?4, ?5)",
                            params![
                                event.occurred_at,
                                event.button_id,
                                event.mode,
                                event.response_id,
                                event.response_text
                            ],
                        )
                        .context("failed to write button event")?;
                    }
                    StoreEvent::SetupDebug(event) => {
                        conn.execute(
                            "insert into setup_debug_events \
                             (event_type, button_id, details) values (?1, ?2, ?3)",
                            params![event.event_type, event.button_id, event.details],
                        )
                        .context("failed to write setup debug event")?;
                    }
                }
            }

            Ok(())
        });

        Ok(Self {
            sender,
            worker: Some(worker),
        })
    }

    fn record(&self, event: ButtonEvent) -> Result<()> {
        self.sender
            .send(StoreEvent::Button(event))
            .context("failed to enqueue button event")
    }

    fn record_setup_debug(&self, event: SetupDebugEvent) -> Result<()> {
        self.sender
            .send(StoreEvent::SetupDebug(event))
            .context("failed to enqueue setup debug event")
    }

    fn shutdown(mut self) -> Result<()> {
        drop(self.sender);
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| anyhow::anyhow!("event store worker panicked"))??;
        }
        Ok(())
    }
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "create table if not exists button_events (
            id integer primary key autoincrement,
            occurred_at text not null,
            button_id integer not null,
            mode text not null,
            response_id text not null,
            response_text text not null
        );
        create table if not exists setup_debug_events (
            id integer primary key autoincrement,
            occurred_at text not null default current_timestamp,
            event_type text not null,
            button_id integer,
            details text
        );
        create table if not exists pomodoro_settings (
          id integer primary key check (id = 1),
          enabled integer not null default 0,
          child_age_years integer check (child_age_years between 3 and 18),
          focus_minutes integer not null default 10 check (focus_minutes between 5 and 60),
          break_minutes integer not null default 3 check (break_minutes between 1 and 30),
          cycles integer not null default 2 check (cycles between 1 and 8),
          preset text not null default 'mini' check (preset in ('mini', 'focus', 'full', 'custom')),
          validated_at text,
          updated_at text not null default current_timestamp
        );",
    )
    .context("failed to initialize SQLite schema")
}

fn run_sim(
    content: ContentPack,
    database_path: PathBuf,
    audio_backend: AudioBackend,
    audio_root: PathBuf,
) -> Result<()> {
    let mut input = TerminalButtonInput::new(content.clone())?;
    let led = TerminalLedOutput;
    match audio_backend {
        AudioBackend::Terminal => {
            let audio = TerminalAudioOutput;
            run_device_loop(&mut input, &audio, &led, content, database_path)
        }
        AudioBackend::Local => {
            let audio = LocalAudioOutput::new(audio_root)?;
            run_device_loop(&mut input, &audio, &led, content, database_path)
        }
    }
}

fn run_device_loop(
    input: &mut dyn ButtonInput,
    audio: &dyn AudioOutput,
    led: &dyn LedOutput,
    content: ContentPack,
    database_path: PathBuf,
) -> Result<()> {
    let store = EventStore::start(database_path.clone())?;
    let mut response_counts: HashMap<String, usize> = HashMap::new();

    loop {
        match input.next_press()? {
            InputEvent::Button(press) => match press.behavior {
                ButtonBehavior::Language | ButtonBehavior::Animals | ButtonBehavior::Music => {
                    let occurred_at = Utc::now().to_rfc3339();
                    let mode = press
                        .mode
                        .as_ref()
                        .with_context(|| format!("button {} is missing a mode", press.button_id))?;
                    let count = response_counts.entry(mode.clone()).or_insert(0);
                    let response = content
                        .response_for(mode, *count)
                        .with_context(|| format!("no response found for mode {}", mode))?;
                    *count += 1;

                    let audio_feedback = audio.play(response)?;
                    led.pulse(mode)?;
                    input.feedback(DeviceFeedback::Playback {
                        occurred_at: display_timestamp(&occurred_at),
                        button_id: press.button_id,
                        mode: mode.clone(),
                        response: response.clone(),
                        audio: audio_feedback,
                    })?;
                    input.feedback(DeviceFeedback::Led {
                        label: mode.clone(),
                        state: LedFeedbackState::Pulse,
                    })?;

                    if content.setup_complete {
                        store.record(ButtonEvent {
                            occurred_at,
                            button_id: press.button_id,
                            mode: mode.clone(),
                            response_id: response.id.clone(),
                            response_text: response.text.clone(),
                        })?;
                    } else {
                        store.record_setup_debug(SetupDebugEvent {
                            event_type: "first_run_button_press".to_string(),
                            button_id: press.button_id,
                            details: format!(
                                "{{\"mode\":\"{}\",\"response_id\":\"{}\"}}",
                                mode, response.id
                            ),
                        })?;
                    }
                }
                ButtonBehavior::SetupHelp => {
                    let occurred_at = Utc::now().to_rfc3339();
                    let response = content.setup_help_response();
                    let audio_feedback = audio.play(&response)?;
                    led.pulse("setup_help")?;
                    input.feedback(DeviceFeedback::Playback {
                        occurred_at: display_timestamp(&occurred_at),
                        button_id: press.button_id,
                        mode: "setup_help".to_string(),
                        response: response.clone(),
                        audio: audio_feedback,
                    })?;
                    input.feedback(DeviceFeedback::Led {
                        label: "setup_help".to_string(),
                        state: LedFeedbackState::Pulse,
                    })?;
                    store.record_setup_debug(SetupDebugEvent {
                        event_type: "setup_help_button_press".to_string(),
                        button_id: press.button_id,
                        details: "{}".to_string(),
                    })?;
                }
                ButtonBehavior::Disabled => {
                    let occurred_at = Utc::now().to_rfc3339();
                    if content.setup_complete {
                        let response = Response {
                            id: "inactive-button".to_string(),
                            text: "inactive".to_string(),
                            audio_path: None,
                        };
                        let audio_feedback = audio.play(&response)?;
                        led.blink_inactive()?;
                        input.feedback(DeviceFeedback::Playback {
                            occurred_at: display_timestamp(&occurred_at),
                            button_id: press.button_id,
                            mode: "disabled".to_string(),
                            response,
                            audio: audio_feedback,
                        })?;
                        input.feedback(DeviceFeedback::Led {
                            label: "disabled".to_string(),
                            state: LedFeedbackState::Inactive,
                        })?;
                    } else {
                        let response = content.setup_help_response();
                        let audio_feedback = audio.play(&response)?;
                        led.pulse("setup_help")?;
                        input.feedback(DeviceFeedback::Playback {
                            occurred_at: display_timestamp(&occurred_at),
                            button_id: press.button_id,
                            mode: "setup_help".to_string(),
                            response,
                            audio: audio_feedback,
                        })?;
                        input.feedback(DeviceFeedback::Led {
                            label: "setup_help".to_string(),
                            state: LedFeedbackState::Pulse,
                        })?;
                    }
                    store.record_setup_debug(SetupDebugEvent {
                        event_type: "disabled_button_press".to_string(),
                        button_id: press.button_id,
                        details: format!("{{\"setup_complete\":{}}}", content.setup_complete),
                    })?;
                }
            },
            InputEvent::PomodoroShortcut => {
                handle_pomodoro_shortcut(input, audio, &store, &database_path)?;
            }
            InputEvent::Quit => {
                input.feedback(DeviceFeedback::Quit)?;
                break;
            }
        }
    }

    store.shutdown()
}

fn handle_pomodoro_shortcut(
    input: &mut dyn ButtonInput,
    audio: &dyn AudioOutput,
    store: &EventStore,
    database_path: &Path,
) -> Result<()> {
    let conn = Connection::open(database_path)
        .with_context(|| format!("failed to open SQLite database {}", database_path.display()))?;
    let Some(settings) = runtime_enabled_settings(&conn)? else {
        store.record_setup_debug(SetupDebugEvent {
            event_type: "pomodoro_skipped".to_string(),
            button_id: 0,
            details: "{\"reason\":\"disabled_or_unvalidated\"}".to_string(),
        })?;
        input.feedback(DeviceFeedback::Pomodoro {
            label: "Focus skipped".to_string(),
            detail: "Owner must validate the focus routine in Settings.".to_string(),
        })?;
        return Ok(());
    };

    run_pomodoro_routine(input, audio, store, &settings)
}

fn run_pomodoro_routine(
    input: &mut dyn ButtonInput,
    audio: &dyn AudioOutput,
    store: &EventStore,
    settings: &PomodoroSettings,
) -> Result<()> {
    audio.stop()?;
    input.feedback(DeviceFeedback::Pomodoro {
        label: "Focus routine".to_string(),
        detail: format!(
            "{} min focus, {} min break, {} cycles",
            settings.focus_minutes, settings.break_minutes, settings.cycles
        ),
    })?;
    audio.play_chime(PomodoroChime::Start)?;

    for cycle in 1..=settings.cycles {
        input.feedback(DeviceFeedback::Pomodoro {
            label: "Focus".to_string(),
            detail: format!("Cycle {cycle} of {}", settings.cycles),
        })?;
        let focus_duration = Duration::from_secs(u64::from(settings.focus_minutes) * 60);
        audio.play_focus(focus_duration)?;
        if input.wait_for_pomodoro_cancel(focus_duration)? {
            audio.stop()?;
            audio.play_chime(PomodoroChime::Cancel)?;
            store.record_setup_debug(SetupDebugEvent {
                event_type: "pomodoro_cancelled".to_string(),
                button_id: 0,
                details: format!("{{\"cycle\":{cycle}}}"),
            })?;
            return Ok(());
        }
        audio.stop()?;
        audio.play_chime(PomodoroChime::BreakStart)?;
        input.feedback(DeviceFeedback::Pomodoro {
            label: "Break".to_string(),
            detail: format!("Cycle {cycle} of {}", settings.cycles),
        })?;
        let break_duration = Duration::from_secs(u64::from(settings.break_minutes) * 60);
        if input.wait_for_pomodoro_cancel(break_duration)? {
            audio.play_chime(PomodoroChime::Cancel)?;
            store.record_setup_debug(SetupDebugEvent {
                event_type: "pomodoro_cancelled".to_string(),
                button_id: 0,
                details: format!("{{\"cycle\":{cycle},\"phase\":\"break\"}}"),
            })?;
            return Ok(());
        }
        audio.play_chime(PomodoroChime::BreakEnd)?;
    }

    audio.play_chime(PomodoroChime::Complete)?;
    store.record_setup_debug(SetupDebugEvent {
        event_type: "pomodoro_completed".to_string(),
        button_id: 0,
        details: format!("{{\"cycles\":{}}}", settings.cycles),
    })?;
    input.feedback(DeviceFeedback::Pomodoro {
        label: "Focus complete".to_string(),
        detail: "Routine finished.".to_string(),
    })?;
    Ok(())
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ButtonGestureEventKind {
    Down,
    Up,
    Tick,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ButtonGestureEvent {
    button_id: u8,
    kind: ButtonGestureEventKind,
    at: Duration,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PomodoroGesture {
    HoldCompleted,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct PomodoroGestureRecognizer {
    pressed_since: HashMap<u8, Duration>,
    chord_started_at: Option<Duration>,
    completed: bool,
}

#[allow(dead_code)]
impl PomodoroGestureRecognizer {
    fn new() -> Self {
        Self {
            pressed_since: HashMap::new(),
            chord_started_at: None,
            completed: false,
        }
    }

    fn handle(&mut self, event: ButtonGestureEvent) -> Option<PomodoroGesture> {
        match event.kind {
            ButtonGestureEventKind::Down => {
                self.pressed_since
                    .entry(event.button_id)
                    .or_insert(event.at);
                self.update_chord_state(event.at);
            }
            ButtonGestureEventKind::Up => {
                self.pressed_since.remove(&event.button_id);
                if POMODORO_COMBO_BUTTONS.contains(&event.button_id) {
                    self.chord_started_at = None;
                    self.completed = false;
                }
            }
            ButtonGestureEventKind::Tick => {
                self.update_chord_state(event.at);
            }
        }

        if !self.completed {
            if let Some(started_at) = self.chord_started_at {
                if event.at.saturating_sub(started_at) >= POMODORO_HOLD_DURATION {
                    self.completed = true;
                    return Some(PomodoroGesture::HoldCompleted);
                }
            }
        }
        None
    }

    fn update_chord_state(&mut self, at: Duration) {
        if !POMODORO_COMBO_BUTTONS
            .iter()
            .all(|button_id| self.pressed_since.contains_key(button_id))
        {
            return;
        }

        let first = POMODORO_COMBO_BUTTONS
            .iter()
            .filter_map(|button_id| self.pressed_since.get(button_id))
            .min()
            .copied()
            .unwrap_or(at);
        let last = POMODORO_COMBO_BUTTONS
            .iter()
            .filter_map(|button_id| self.pressed_since.get(button_id))
            .max()
            .copied()
            .unwrap_or(at);

        if last.saturating_sub(first) <= POMODORO_CHORD_ARM_WINDOW {
            self.chord_started_at = Some(last);
        } else {
            self.chord_started_at = None;
        }
    }
}

fn display_timestamp(rfc3339: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .map(|timestamp| timestamp.format("%H:%M:%S%.3f").to_string())
        .unwrap_or_else(|_| rfc3339.to_string())
}

fn button_press(button_id: u8, content: &ContentPack) -> Result<ButtonPress> {
    let mapping = content.mapping_for(button_id)?;
    Ok(ButtonPress {
        button_id,
        behavior: mapping.behavior,
        mode: mapping.mode,
    })
}

fn mapping_summary_label(mapping: &ButtonMapping) -> String {
    match mapping.behavior {
        ButtonBehavior::Language | ButtonBehavior::Animals | ButtonBehavior::Music => mapping
            .mode
            .as_deref()
            .unwrap_or("unconfigured")
            .to_string(),
        ButtonBehavior::Disabled => "disabled".to_string(),
        ButtonBehavior::SetupHelp => "setup/help".to_string(),
    }
}

pub fn run_from_cli() -> Result<()> {
    let cli = Cli::parse();
    let content = ContentPack::load(&cli.content)?;

    match cli.backend {
        Backend::Sim => run_sim(content, cli.database, cli.audio, cli.audio_root),
        Backend::Pi => {
            bail!("Pi backend is not implemented yet; use --backend sim for local development")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::types::ModeContent;
    use tempfile::NamedTempFile;

    struct ScriptedInput {
        events: Vec<InputEvent>,
        cancel_waits: std::cell::RefCell<Vec<Duration>>,
    }

    impl ButtonInput for ScriptedInput {
        fn next_press(&mut self) -> Result<InputEvent> {
            Ok(self.events.remove(0))
        }

        fn wait_for_pomodoro_cancel(&mut self, duration: Duration) -> Result<bool> {
            self.cancel_waits.borrow_mut().push(duration);
            Ok(false)
        }
    }

    struct CapturingAudio {
        played: std::cell::RefCell<Vec<String>>,
        routine: std::cell::RefCell<Vec<String>>,
    }

    impl AudioOutput for CapturingAudio {
        fn play(&self, response: &Response) -> Result<AudioPlayback> {
            self.played.borrow_mut().push(response.id.clone());
            Ok(AudioPlayback {
                resolved_path: None,
                source_path: response.audio_path.clone(),
            })
        }

        fn stop(&self) -> Result<()> {
            self.routine.borrow_mut().push("stop".to_string());
            Ok(())
        }

        fn play_chime(&self, chime: PomodoroChime) -> Result<()> {
            self.routine.borrow_mut().push(format!("chime:{chime:?}"));
            Ok(())
        }

        fn play_focus(&self, duration: Duration) -> Result<()> {
            self.routine
                .borrow_mut()
                .push(format!("focus:{}", duration.as_secs()));
            Ok(())
        }
    }

    struct NoopLed;

    impl LedOutput for NoopLed {
        fn pulse(&self, _label: &str) -> Result<()> {
            Ok(())
        }

        fn blink_inactive(&self) -> Result<()> {
            Ok(())
        }
    }

    fn test_content() -> ContentPack {
        ContentPack {
            modes: vec![
                ModeContent {
                    mode: "language:English".to_string(),
                    responses: vec![
                        Response {
                            id: "en-1".to_string(),
                            text: "Hello".to_string(),
                            audio_path: None,
                        },
                        Response {
                            id: "en-2".to_string(),
                            text: "Good job".to_string(),
                            audio_path: None,
                        },
                    ],
                },
                ModeContent {
                    mode: "animals".to_string(),
                    responses: vec![Response {
                        id: "animal-1".to_string(),
                        text: "Moo".to_string(),
                        audio_path: None,
                    }],
                },
                ModeContent {
                    mode: "music".to_string(),
                    responses: vec![Response {
                        id: "music-1".to_string(),
                        text: "La la".to_string(),
                        audio_path: None,
                    }],
                },
            ],
            setup_complete: true,
            dashboard_host: "tcube.local".to_string(),
            dashboard_ip: Some("192.168.4.20".to_string()),
            setup_help_text: "Open t cube dot local, or the IP address, to set me up.".to_string(),
            button_mappings: vec![
                ButtonMapping {
                    button_id: 1,
                    behavior: ButtonBehavior::Language,
                    mode: Some("language:English".to_string()),
                },
                ButtonMapping {
                    button_id: 2,
                    behavior: ButtonBehavior::Animals,
                    mode: Some("animals".to_string()),
                },
                ButtonMapping {
                    button_id: 3,
                    behavior: ButtonBehavior::Music,
                    mode: Some("music".to_string()),
                },
                ButtonMapping {
                    button_id: 4,
                    behavior: ButtonBehavior::SetupHelp,
                    mode: None,
                },
                ButtonMapping {
                    button_id: 5,
                    behavior: ButtonBehavior::Disabled,
                    mode: None,
                },
            ],
        }
    }

    #[test]
    fn maps_button_ids_to_modes() {
        let content = test_content();
        assert_eq!(
            button_press(1, &content).unwrap().mode.unwrap(),
            "language:English"
        );
        assert_eq!(button_press(2, &content).unwrap().mode.unwrap(), "animals");
        assert_eq!(button_press(3, &content).unwrap().mode.unwrap(), "music");
        assert_eq!(
            button_press(4, &content).unwrap().behavior,
            ButtonBehavior::SetupHelp
        );
        assert_eq!(
            button_press(5, &content).unwrap().behavior,
            ButtonBehavior::Disabled
        );
        assert!(button_press(6, &content).is_err());
    }

    #[test]
    fn summarizes_configured_button_mapping() {
        let content = test_content();
        let state = TuiState::new(&content);

        assert_eq!(
            state
                .buttons
                .iter()
                .map(|button| format!("{} {}", button.button_id, button.label))
                .collect::<Vec<_>>()
                .join(" | "),
            "1 language:English | 2 animals | 3 music | 4 setup/help | 5 disabled"
        );
    }

    #[test]
    fn selects_responses_deterministically_per_mode() {
        let content = test_content();

        assert_eq!(
            content.response_for("language:English", 0).unwrap().id,
            "en-1"
        );
        assert_eq!(
            content.response_for("language:English", 1).unwrap().id,
            "en-2"
        );
        assert_eq!(
            content.response_for("language:English", 2).unwrap().id,
            "en-1"
        );
    }

    #[test]
    fn resolves_relative_audio_paths_from_audio_root() {
        let response = Response {
            id: "with-audio".to_string(),
            text: "Hello".to_string(),
            audio_path: Some("content/audio/english/hello.wav".to_string()),
        };

        assert_eq!(
            audio_asset_path(&response, Path::new("/runtime")).unwrap(),
            PathBuf::from("/runtime/content/audio/english/hello.wav")
        );
    }

    #[test]
    fn keeps_absolute_audio_paths() {
        let response = Response {
            id: "with-audio".to_string(),
            text: "Hello".to_string(),
            audio_path: Some("/media/tcube/hello.wav".to_string()),
        };

        assert_eq!(
            audio_asset_path(&response, Path::new("/runtime")).unwrap(),
            PathBuf::from("/media/tcube/hello.wav")
        );
    }

    #[test]
    fn allows_responses_without_audio_assets() {
        let response = Response {
            id: "setup-help".to_string(),
            text: "Open t cube dot local.".to_string(),
            audio_path: None,
        };

        assert_eq!(audio_asset_path(&response, Path::new("/runtime")), None);
    }

    #[test]
    fn decodes_default_wav_audio_asset() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("content/audio/english/hello-litle-one.wav");

        let _source = decode_audio_file(&path).unwrap();
    }

    #[test]
    fn decodes_default_mp3_audio_asset() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("content/audio/music/ba-oi-ba.mp3");

        let _source = decode_audio_file(&path).unwrap();
    }

    #[test]
    fn rejects_invalid_audio_assets() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "not audio").unwrap();

        assert!(decode_audio_file(file.path()).is_err());
    }

    #[test]
    fn logs_simulated_button_events_to_sqlite() {
        let database = NamedTempFile::new().unwrap();
        let content = test_content();
        let mut input = ScriptedInput {
            events: vec![
                InputEvent::Button(button_press(1, &content).unwrap()),
                InputEvent::Button(button_press(1, &content).unwrap()),
                InputEvent::Quit,
            ],
            cancel_waits: std::cell::RefCell::new(Vec::new()),
        };
        let audio = CapturingAudio {
            played: std::cell::RefCell::new(Vec::new()),
            routine: std::cell::RefCell::new(Vec::new()),
        };

        run_device_loop(
            &mut input,
            &audio,
            &NoopLed,
            content,
            database.path().to_path_buf(),
        )
        .unwrap();

        assert_eq!(audio.played.borrow().as_slice(), ["en-1", "en-2"]);

        let conn = Connection::open(database.path()).unwrap();
        let count: i64 = conn
            .query_row("select count(*) from button_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn first_run_presses_use_setup_debug_log_only() {
        let database = NamedTempFile::new().unwrap();
        let mut content = test_content();
        content.setup_complete = false;
        let mut input = ScriptedInput {
            events: vec![
                InputEvent::Button(button_press(1, &content).unwrap()),
                InputEvent::Button(button_press(4, &content).unwrap()),
                InputEvent::Quit,
            ],
            cancel_waits: std::cell::RefCell::new(Vec::new()),
        };
        let audio = CapturingAudio {
            played: std::cell::RefCell::new(Vec::new()),
            routine: std::cell::RefCell::new(Vec::new()),
        };

        run_device_loop(
            &mut input,
            &audio,
            &NoopLed,
            content,
            database.path().to_path_buf(),
        )
        .unwrap();

        assert_eq!(audio.played.borrow().as_slice(), ["en-1", "setup-help"]);

        let conn = Connection::open(database.path()).unwrap();
        let button_count: i64 = conn
            .query_row("select count(*) from button_events", [], |row| row.get(0))
            .unwrap();
        let setup_count: i64 = conn
            .query_row("select count(*) from setup_debug_events", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(button_count, 0);
        assert_eq!(setup_count, 2);
    }

    #[test]
    fn generated_pomodoro_focus_source_is_stereo_and_non_silent() {
        let focus_duration = Duration::from_secs(8 * 60);
        let mut source = BinauralFocusSource::new(focus_duration);

        assert_eq!(source.channels().get(), FOCUS_CHANNELS);
        assert_eq!(source.sample_rate().get(), FOCUS_SAMPLE_RATE_HZ);
        assert_eq!(source.total_duration(), Some(focus_duration));

        let first_samples = source.by_ref().take(16).collect::<Vec<_>>();
        assert!(first_samples.iter().all(|sample| sample.abs() < 0.001));

        let later_samples = source
            .skip((FOCUS_SAMPLE_RATE_HZ as usize * FOCUS_CHANNELS as usize) - 16)
            .take(64)
            .collect::<Vec<_>>();
        assert!(later_samples.iter().any(|sample| sample.abs() > 0.01));
        assert_ne!(later_samples[0], later_samples[1]);

        let after_old_fade_cutoff = BinauralFocusSource::new(focus_duration)
            .skip((FOCUS_SAMPLE_RATE_HZ as usize * FOCUS_CHANNELS as usize * 4) + 11)
            .take(128)
            .collect::<Vec<_>>();
        assert!(after_old_fade_cutoff
            .iter()
            .any(|sample| sample.abs() > 0.01));
    }

    #[test]
    fn pomodoro_gesture_starts_after_together_hold() {
        let mut recognizer = PomodoroGestureRecognizer::new();

        assert_eq!(recognizer.handle(gesture_down(1, 0)), None);
        assert_eq!(recognizer.handle(gesture_down(2, 50)), None);
        assert_eq!(recognizer.handle(gesture_down(4, 100)), None);
        assert_eq!(
            recognizer.handle(gesture_tick(5_100)),
            Some(PomodoroGesture::HoldCompleted)
        );
    }

    #[test]
    fn pomodoro_gesture_release_before_hold_cancels() {
        let mut recognizer = PomodoroGestureRecognizer::new();

        assert_eq!(recognizer.handle(gesture_down(1, 0)), None);
        assert_eq!(recognizer.handle(gesture_down(2, 50)), None);
        assert_eq!(recognizer.handle(gesture_down(4, 100)), None);
        assert_eq!(recognizer.handle(gesture_up(2, 2_000)), None);
        assert_eq!(recognizer.handle(gesture_tick(6_000)), None);
    }

    #[test]
    fn pomodoro_gesture_rejects_staggered_chord() {
        let mut recognizer = PomodoroGestureRecognizer::new();

        assert_eq!(recognizer.handle(gesture_down(1, 0)), None);
        assert_eq!(recognizer.handle(gesture_down(2, 50)), None);
        assert_eq!(recognizer.handle(gesture_down(4, 500)), None);
        assert_eq!(recognizer.handle(gesture_tick(5_500)), None);
    }

    #[test]
    fn pomodoro_routine_runs_focus_break_sequence() {
        let database = NamedTempFile::new().unwrap();
        let store = EventStore::start(database.path().to_path_buf()).unwrap();
        let mut input = ScriptedInput {
            events: Vec::new(),
            cancel_waits: std::cell::RefCell::new(Vec::new()),
        };
        let audio = CapturingAudio {
            played: std::cell::RefCell::new(Vec::new()),
            routine: std::cell::RefCell::new(Vec::new()),
        };
        let settings = PomodoroSettings {
            enabled: true,
            child_age_years: Some(9),
            focus_minutes: 20,
            break_minutes: 5,
            cycles: 2,
            preset: "focus".to_string(),
            validated_at: Some("2026-07-01T00:00:00.000Z".to_string()),
            updated_at: "2026-07-01T00:00:00.000Z".to_string(),
        };

        run_pomodoro_routine(&mut input, &audio, &store, &settings).unwrap();
        store.shutdown().unwrap();

        assert_eq!(
            audio.routine.borrow().as_slice(),
            [
                "stop",
                "chime:Start",
                "focus:1200",
                "stop",
                "chime:BreakStart",
                "chime:BreakEnd",
                "focus:1200",
                "stop",
                "chime:BreakStart",
                "chime:BreakEnd",
                "chime:Complete",
            ]
        );
        assert_eq!(
            input.cancel_waits.borrow().as_slice(),
            [
                Duration::from_secs(20 * 60),
                Duration::from_secs(5 * 60),
                Duration::from_secs(20 * 60),
                Duration::from_secs(5 * 60),
            ]
        );
    }

    #[test]
    fn pomodoro_shortcut_skips_when_unvalidated() {
        let database = NamedTempFile::new().unwrap();
        let content = test_content();
        let mut input = ScriptedInput {
            events: vec![InputEvent::PomodoroShortcut, InputEvent::Quit],
            cancel_waits: std::cell::RefCell::new(Vec::new()),
        };
        let audio = CapturingAudio {
            played: std::cell::RefCell::new(Vec::new()),
            routine: std::cell::RefCell::new(Vec::new()),
        };

        run_device_loop(
            &mut input,
            &audio,
            &NoopLed,
            content,
            database.path().to_path_buf(),
        )
        .unwrap();

        assert!(audio.routine.borrow().is_empty());
        let conn = Connection::open(database.path()).unwrap();
        let event_type: String = conn
            .query_row(
                "select event_type from setup_debug_events order by id desc limit 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_type, "pomodoro_skipped");
    }

    fn gesture_down(button_id: u8, millis: u64) -> ButtonGestureEvent {
        gesture_event(button_id, ButtonGestureEventKind::Down, millis)
    }

    fn gesture_up(button_id: u8, millis: u64) -> ButtonGestureEvent {
        gesture_event(button_id, ButtonGestureEventKind::Up, millis)
    }

    fn gesture_tick(millis: u64) -> ButtonGestureEvent {
        gesture_event(0, ButtonGestureEventKind::Tick, millis)
    }

    fn gesture_event(
        button_id: u8,
        kind: ButtonGestureEventKind,
        millis: u64,
    ) -> ButtonGestureEvent {
        ButtonGestureEvent {
            button_id,
            kind,
            at: Duration::from_millis(millis),
        }
    }
}
