//! Scripted input (`--script [file]`): one step per `ui::step`, when it is
//! due and nothing the last one caused is still waiting to be handled.
//!
//! ```text
//! wait 300                 # ms before the next step
//! click form2 btnNested    # mouse down + up at the component's centre
//! key form3 Tab            # Tab Enter Space Escape Left Right Up Down Home End or one character
//! close form2              # the window's close box
//! menu file.open           # a menu pick (as muda's event)
//! resize form2 420 260     # the client area, as a user's drag would
//! capture                  # every shown form -> <RAPIDR_CAPTURE>-<n>.bmp
//! expect form2 lblTicks ticking
//! print some text
//! ```

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::form::Key;

/// The gap between two steps (windows paint, handlers run).
const GAP: Duration = Duration::from_millis(30);

#[derive(Debug, Clone)]
pub enum Step {
    Wait(u64),
    Click(String, String),
    Key(String, Key),
    Close(String),
    Menu(String),
    Resize(String, i64, i64),
    Capture,
    Expect(String, String, String),
    Print(String),
    /// macOS probes (real windows only; see `macos_probe.rs`):
    /// `liveresize FORM DX MS` drags the window's right edge,
    LiveResize(String, f64, u64),
    /// `cmdkey FORM CHAR KEYCODE` posts Cmd+CHAR (a menu key equivalent),
    CmdKey(String, char, u16),
    /// `cancelpanels MS` cancels open/save panels after MS,
    CancelPanels(u64),
    /// `poke FORM MS` resizes FORM's window from a main-queue block after MS.
    Poke(String, u64),
    /// `menutrack FORM MS` opens a pop-up menu over FORM for MS (tracking loop).
    MenuTrack(String, u64),
}

pub struct Script {
    steps: VecDeque<Step>,
    next: Instant,
    pub failures: u32,
}

fn key(s: &str) -> Key {
    match s {
        "Tab" => Key::Tab,
        "Enter" => Key::Enter,
        "Space" => Key::Space,
        "Escape" => Key::Escape,
        "Left" => Key::Left,
        "Right" => Key::Right,
        "Up" => Key::Up,
        "Down" => Key::Down,
        "Home" => Key::Home,
        "End" => Key::End,
        _ => s.chars().next().map_or(Key::Other, Key::Char),
    }
}

impl Script {
    pub fn parse(text: &str) -> Result<Script, String> {
        let mut steps = VecDeque::new();
        for (n, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let w: Vec<&str> = line.split_whitespace().collect();
            let bad = || format!("script line {}: {line}", n + 1);
            let arg = |i: usize| w.get(i).map(|s| s.to_string()).ok_or_else(bad);
            let num = |i: usize| w.get(i).and_then(|s| s.parse::<i64>().ok()).ok_or_else(bad);
            steps.push_back(match w[0] {
                "wait" => Step::Wait(num(1)? as u64),
                "click" => Step::Click(arg(1)?, arg(2)?),
                "key" => Step::Key(arg(1)?, key(&arg(2)?)),
                "close" => Step::Close(arg(1)?),
                "menu" => Step::Menu(arg(1)?),
                "resize" => Step::Resize(arg(1)?, num(2)?, num(3)?),
                "capture" => Step::Capture,
                "expect" => Step::Expect(arg(1)?, arg(2)?, w[3..].join(" ")),
                "print" => Step::Print(w[1..].join(" ")),
                "liveresize" => Step::LiveResize(arg(1)?, num(2)? as f64, num(3)? as u64),
                "cmdkey" => Step::CmdKey(arg(1)?, arg(2)?.chars().next().ok_or_else(bad)?, num(3)? as u16),
                "cancelpanels" => Step::CancelPanels(num(1)? as u64),
                "poke" => Step::Poke(arg(1)?, num(2)? as u64),
                "menutrack" => Step::MenuTrack(arg(1)?, num(2)? as u64),
                _ => return Err(bad()),
            });
        }
        Ok(Script { steps, next: Instant::now(), failures: 0 })
    }

    /// When the next step is due (None: the script is done).
    pub fn next_at(&self) -> Option<Instant> {
        (!self.steps.is_empty()).then_some(self.next)
    }

    pub fn take_due(&mut self, now: Instant) -> Option<Step> {
        if now < self.next {
            return None;
        }
        let s = self.steps.pop_front()?;
        self.next = now
            + match s {
                Step::Wait(ms) => Duration::from_millis(ms),
                _ => GAP,
            };
        Some(s)
    }
}
