//! rmatrix — a digital-rain screen toy with richer effects than cmatrix:
//! true-color glow trails, katakana glyphs, in-place mutation, and themes.

mod glyphs;
mod rain;
mod ripple;
mod theme;

use std::io::{self, Write};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    MouseEventKind,
};
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor,
};
use crossterm::terminal::{
    self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{ExecutableCommand, QueueableCommand};

use rain::Rain;
use theme::Theme;

struct Config {
    theme: Theme,
    density: f32,
    mutation: f32,
    fps: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: Theme::Green,
            density: 0.9,
            mutation: 8.0,
            fps: 60,
        }
    }
}

const HELP: &str = "\
rmatrix — digital rain for your terminal

USAGE:
    rmatrix [OPTIONS]

OPTIONS:
    -c, --color <THEME>   green | cyan | amber | purple | rainbow  (default: green)
    -d, --density <N>     stream spawn density, higher = denser     (default: 0.9)
    -m, --mutation <N>    glyph flicker rate, higher = busier        (default: 8.0)
    -f, --fps <N>         target frames per second                   (default: 60)
    -h, --help            show this help

KEYS (while running):
    q / Esc / Ctrl-C   quit
    space              pause / resume
    1..5               switch theme (green/cyan/amber/purple/rainbow)
    + / -              faster / slower glyph mutation
    r                  drop a ripple at a random spot
    w                  send a wide wave sweeping across
    mouse click        drop a ripple right there
";

fn parse_args() -> Result<Config, String> {
    let mut cfg = Config::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            "-c" | "--color" => {
                cfg.theme = args.next().ok_or("--color needs a value")?.parse()?
            }
            "-d" | "--density" => {
                cfg.density = args
                    .next()
                    .ok_or("--density needs a value")?
                    .parse()
                    .map_err(|_| "bad --density")?
            }
            "-m" | "--mutation" => {
                cfg.mutation = args
                    .next()
                    .ok_or("--mutation needs a value")?
                    .parse()
                    .map_err(|_| "bad --mutation")?
            }
            "-f" | "--fps" => {
                cfg.fps = args
                    .next()
                    .ok_or("--fps needs a value")?
                    .parse()
                    .map_err(|_| "bad --fps")?;
                if cfg.fps == 0 {
                    return Err("--fps must be > 0".into());
                }
            }
            other => return Err(format!("unknown argument '{other}'")),
        }
    }
    Ok(cfg)
}

fn main() -> ExitCode {
    let cfg = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rmatrix: {e}\n\nTry 'rmatrix --help'.");
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = run(cfg) {
        eprintln!("rmatrix: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run(mut cfg: Config) -> io::Result<()> {
    let mut out = io::stdout();

    terminal::enable_raw_mode()?;
    out.execute(EnterAlternateScreen)?;
    out.execute(Hide)?;
    // Capture the mouse so the terminal stops doing native click-drag text
    // selection — the rain should feel like a graphic, not selectable text.
    out.execute(EnableMouseCapture)?;
    out.execute(Clear(ClearType::All))?;

    // Run the loop, capturing any error so we can always restore the terminal.
    let result = event_loop(&mut out, &mut cfg);

    // Teardown — restore terminal regardless of how the loop ended.
    let _ = out.execute(DisableMouseCapture);
    let _ = out.execute(Show);
    let _ = out.execute(SetAttribute(Attribute::Reset));
    let _ = out.execute(ResetColor);
    let _ = out.execute(LeaveAlternateScreen);
    let _ = terminal::disable_raw_mode();

    result
}

fn event_loop(out: &mut io::Stdout, cfg: &mut Config) -> io::Result<()> {
    let (mut cols, mut rows) = terminal::size()?;
    let mut rain = Rain::new(cols, rows, cfg.theme, cfg.density, cfg.mutation);

    let frame = Duration::from_secs_f64(1.0 / cfg.fps as f64);
    let mut last = Instant::now();
    let mut paused = false;

    loop {
        // Drain all pending input first so the toy stays responsive.
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(k) => {
                    if handle_key(k, cfg, &mut rain, &mut paused) {
                        return Ok(()); // quit requested
                    }
                }
                Event::Mouse(m) => {
                    if let MouseEventKind::Down(_) = m.kind {
                        rain.splash(m.column, m.row);
                    }
                }
                Event::Resize(c, r) => {
                    cols = c;
                    rows = r;
                    rain.resize(cols, rows);
                    out.queue(Clear(ClearType::All))?;
                }
                _ => {}
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f32();
        last = now;

        if !paused {
            rain.update(dt);
            render(out, &mut rain)?;
        }

        // Sleep to hit the target frame time.
        let elapsed = Instant::now().duration_since(now);
        if elapsed < frame {
            std::thread::sleep(frame - elapsed);
        }
    }
}

/// Returns `true` if the user asked to quit.
fn handle_key(k: KeyEvent, cfg: &mut Config, rain: &mut Rain, paused: &mut bool) -> bool {
    match k.code {
        KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => return true,
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char(' ') => *paused = !*paused,
        KeyCode::Char('1') => set_theme(cfg, rain, Theme::Green),
        KeyCode::Char('2') => set_theme(cfg, rain, Theme::Cyan),
        KeyCode::Char('3') => set_theme(cfg, rain, Theme::Amber),
        KeyCode::Char('4') => set_theme(cfg, rain, Theme::Purple),
        KeyCode::Char('5') => set_theme(cfg, rain, Theme::Rainbow),
        KeyCode::Char('r') => rain.splash_random(),
        KeyCode::Char('w') => rain.wave(),
        KeyCode::Char('+') | KeyCode::Char('=') => {
            cfg.mutation = (cfg.mutation * 1.5).min(120.0);
            rebuild(cfg, rain);
        }
        KeyCode::Char('-') | KeyCode::Char('_') => {
            cfg.mutation = (cfg.mutation / 1.5).max(0.1);
            rebuild(cfg, rain);
        }
        _ => {}
    }
    false
}

fn set_theme(cfg: &mut Config, rain: &mut Rain, theme: Theme) {
    cfg.theme = theme;
    rebuild(cfg, rain);
}

/// Rebuild the simulation in place at the current terminal size with new params.
fn rebuild(cfg: &Config, rain: &mut Rain) {
    if let Ok((c, r)) = terminal::size() {
        *rain = Rain::new(c, r, cfg.theme, cfg.density, cfg.mutation);
    }
}

fn render(out: &mut io::Stdout, rain: &mut Rain) -> io::Result<()> {
    let mut last_color: Option<Color> = None;
    let mut last_bold: Option<bool> = None;
    for (col, row, ch, color, bold) in rain.diff() {
        out.queue(MoveTo(col, row))?;
        if last_bold != Some(bold) {
            out.queue(SetAttribute(if bold {
                Attribute::Bold
            } else {
                Attribute::NormalIntensity
            }))?;
            last_bold = Some(bold);
        }
        if last_color != Some(color) {
            out.queue(SetForegroundColor(color))?;
            last_color = Some(color);
        }
        out.queue(Print(ch))?;
    }
    out.flush()
}
