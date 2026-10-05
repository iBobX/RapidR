//! QDXJOYSTICK (docs/directx-plan.md §2.4, stage D6). RapidQ's compiler
//! (RC.EXE) has it though the manual doesn't: `IsLeft`, `IsRight`, `IsUp`,
//! `IsDown` (read-only), `Button(n)` (read-only), `Update` — DelphiX's
//! TJoystick, the state `Update` read. RapidR's additions on the same object:
//! `Index` (which joystick; 0 the first), `Connected`, `Name`, the axes `X`
//! `Y` `Z` `R` `U` `V` in winmm's JOYINFOEX units (0 … 65535, 32767 at rest,
//! Y growing downward), `Buttons` (bit n − 1 for button n), `POV`
//! (hundredths of a degree clockwise from forward, −1 centred), and the
//! events OnButtonDown(Button) / OnButtonUp(Button) / OnMove, which the
//! runtimes poll for while the program waits.
//!
//! Where the gamepads come from is the runtime's [`Source`]: gilrs on
//! Windows and macOS, the kernel's evdev on Linux, the Gamepad API in the
//! browser, a script for the tests ([`Script`]); none installed, no
//! joystick. Every one of them reports a gamepad the W3C "standard" way
//! ([`Standard`]), which [`Pad::from_standard`] lays out as winmm showed an
//! Xbox controller — so a program reads the same numbers everywhere.
//! Without a joystick RapidR's QDXJOYSTICK says Connected 0 and everything
//! at rest; RapidQ's raised EStringListError (List index out of bounds) —
//! not copied.

use std::cell::RefCell;

use crate::{v_int, v_str, Value};

/// An axis at rest (JOYINFOEX's centre).
pub const CENTRE: i64 = 32767;
/// POV centred.
pub const POV_CENTRED: i64 = -1;

/// One gamepad as the program reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pad {
    pub name: String,
    /// X, Y, Z, R, U, V: 0 … 65535.
    pub axes: [i64; 6],
    /// Buttons down: bit n − 1 for button n (1 … 32).
    pub buttons: u32,
    /// Hundredths of a degree clockwise from forward; −1 centred.
    pub pov: i64,
}

impl Pad {
    /// A gamepad at rest.
    pub fn resting(name: &str) -> Pad {
        Pad { name: name.to_string(), axes: [CENTRE; 6], buttons: 0, pov: POV_CENTRED }
    }

    /// A gamepad reported the W3C standard way, laid out as winmm showed
    /// an Xbox controller: X / Y the left stick, Z the triggers (the left
    /// one towards 65535, the right one towards 0), R / U the right stick's
    /// Y / X, V at rest; buttons 1 A, 2 B, 3 X, 4 Y, 5 LB, 6 RB, 7 Back, 8
    /// Start, 9 / 10 the sticks pressed, 11 the Guide button; the D-pad the
    /// POV.
    pub fn from_standard(s: &Standard) -> Pad {
        // (−1 … 1 onto 0 … 65535, 0 onto 32767)
        let axis = |v: f32| (((v.clamp(-1.0, 1.0) as f64 + 1.0) / 2.0 * 65535.0).floor() as i64).clamp(0, 65535);
        let mut buttons = 0u32;
        // (standard index → button number)
        for (i, n) in [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (8, 7), (9, 8), (10, 9), (11, 10), (16, 11)] {
            if s.pressed(i) {
                buttons |= 1 << (n - 1);
            }
        }
        // (up − down, right − left)
        let vertical = s.pressed(12) as i8 - s.pressed(13) as i8;
        let horizontal = s.pressed(15) as i8 - s.pressed(14) as i8;
        let pov = match (vertical, horizontal) {
            (1, 0) => 0,
            (1, 1) => 4500,
            (0, 1) => 9000,
            (-1, 1) => 13500,
            (-1, 0) => 18000,
            (-1, -1) => 22500,
            (0, -1) => 27000,
            (1, -1) => 31500,
            _ => POV_CENTRED,
        };
        let triggers = s.value(6) - s.value(7);
        Pad {
            name: s.name.clone(),
            axes: [axis(s.axes[0]), axis(s.axes[1]), axis(triggers), axis(s.axes[3]), axis(s.axes[2]), CENTRE],
            buttons,
            pov,
        }
    }

    fn axis(&self, i: usize) -> i64 {
        self.axes[i]
    }
}

/// A gamepad the W3C Gamepad API's "standard" way: axes 0 / 1 the left
/// stick, 2 / 3 the right one (−1 … 1, y down); buttons 0 A, 1 B, 2 X, 3 Y,
/// 4 LB, 5 RB, 6 LT, 7 RT, 8 Back, 9 Start, 10 / 11 the sticks, 12 … 15
/// the D-pad up, down, left, right, 16 Guide — each pressed or not, with a
/// value 0 … 1 (the triggers').
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Standard {
    pub name: String,
    pub axes: [f32; 4],
    pub buttons: [(bool, f32); 17],
}

impl Standard {
    pub fn new(name: &str) -> Standard {
        Standard { name: name.to_string(), ..Standard::default() }
    }

    fn pressed(&self, i: usize) -> bool {
        self.buttons[i].0
    }

    fn value(&self, i: usize) -> f32 {
        let (down, v) = self.buttons[i];
        if v > 0.0 {
            v
        } else if down {
            1.0
        } else {
            0.0
        }
    }
}

/// Where the gamepads come from (the runtime's).
pub trait Source {
    /// The gamepads connected now, in order (Index 0 the first).
    fn pads(&mut self) -> Vec<Pad>;
}

thread_local! {
    static SOURCE: RefCell<Option<Box<dyn Source>>> = const { RefCell::new(None) };
}

/// The runtime's gamepads.
pub fn set_source(source: Box<dyn Source>) {
    SOURCE.with(|s| *s.borrow_mut() = Some(source));
}

/// Whether a source was installed.
pub fn has_source() -> bool {
    SOURCE.with(|s| s.borrow().is_some())
}

/// The gamepads now (none without a source).
fn poll() -> Vec<Pad> {
    SOURCE.with(|s| s.borrow_mut().as_mut().map(|s| s.pads()).unwrap_or_default())
}

/// The tests' gamepads (RAPIDR_TEST_JOYSTICK on the desktop, the page's
/// `RAPIDR_TEST_JOYSTICK` in the browser): a step a poll — `Update`, or the
/// runtime's look for the events — the last one kept. Steps are separated
/// by `;`, the gamepads of a step by `/`, a gamepad's fields by `,`:
/// `name=…`, `x` `y` `z` `r` `u` `v` (0 … 65535), `b` (the Buttons mask),
/// `pov`; `-` (or nothing) is no gamepad. A field left out is at rest.
/// `x=0,b=1;x=32767;-` is a gamepad pushed left with button 1 down, then at
/// rest, then unplugged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Script {
    steps: Vec<Vec<Pad>>,
    next: usize,
}

impl Script {
    pub fn parse(text: &str) -> Result<Script, String> {
        let mut steps = Vec::new();
        for step in text.split(';') {
            let mut pads = Vec::new();
            for pad in step.split('/').map(str::trim).filter(|p| !p.is_empty() && *p != "-") {
                let mut p = Pad::resting("Test pad");
                for field in pad.split(',').map(str::trim).filter(|f| !f.is_empty()) {
                    let (k, v) = field.split_once('=').ok_or_else(|| format!("RAPIDR_TEST_JOYSTICK: {field}: not key=value"))?;
                    let k = k.trim().to_ascii_lowercase();
                    if k == "name" {
                        p.name = v.trim().to_string();
                        continue;
                    }
                    let n: i64 = v.trim().parse().map_err(|_| format!("RAPIDR_TEST_JOYSTICK: {field}: not a number"))?;
                    match k.as_str() {
                        "x" => p.axes[0] = n,
                        "y" => p.axes[1] = n,
                        "z" => p.axes[2] = n,
                        "r" => p.axes[3] = n,
                        "u" => p.axes[4] = n,
                        "v" => p.axes[5] = n,
                        "b" => p.buttons = n as u32,
                        "pov" => p.pov = n,
                        _ => return Err(format!("RAPIDR_TEST_JOYSTICK: {k}: not one of name x y z r u v b pov")),
                    }
                }
                pads.push(p);
            }
            steps.push(pads);
        }
        Ok(Script { steps, next: 0 })
    }
}

impl Source for Script {
    fn pads(&mut self) -> Vec<Pad> {
        let i = self.next.min(self.steps.len().saturating_sub(1));
        self.next = (self.next + 1).min(self.steps.len());
        self.steps.get(i).cloned().unwrap_or_default()
    }
}

/// A QDXJOYSTICK.
#[derive(Clone, Debug, Default)]
pub struct DxJoystick {
    /// Which joystick (RapidR's Index).
    pub index: i64,
    /// What Update (or the events' look) read: none, not connected.
    state: Option<Pad>,
    /// What the events' look saw last.
    seen: Option<Pad>,
}

/// DelphiX's TJoystick: a direction is held when its axis is past half
/// the way from the centre.
const HELD: i64 = 16384;

fn flag(b: bool) -> Value {
    v_int(if b { -1 } else { 0 })
}

impl DxJoystick {
    fn read(&self) -> Pad {
        self.state.clone().unwrap_or_else(|| Pad::resting(""))
    }

    /// RapidQ's Update: the joystick's state now.
    pub fn update(&mut self) {
        self.state = usize::try_from(self.index).ok().and_then(|i| poll().into_iter().nth(i));
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        let p = self.read();
        Some(match prop {
            "isleft" => flag(p.axis(0) < CENTRE - HELD),
            "isright" => flag(p.axis(0) > CENTRE + HELD),
            "isup" => flag(p.axis(1) < CENTRE - HELD),
            "isdown" => flag(p.axis(1) > CENTRE + HELD),
            "index" => v_int(self.index),
            "connected" => flag(self.state.is_some()),
            "name" => v_str(&p.name),
            "x" => v_int(p.axis(0)),
            "y" => v_int(p.axis(1)),
            "z" => v_int(p.axis(2)),
            "r" => v_int(p.axis(3)),
            "u" => v_int(p.axis(4)),
            "v" => v_int(p.axis(5)),
            "buttons" => v_int(p.buttons as i64),
            "pov" => v_int(p.pov),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> Option<Result<(), String>> {
        match prop {
            "index" => {
                self.index = val.to_i64();
                self.state = None;
                self.seen = None;
                Some(Ok(()))
            }
            "isleft" | "isright" | "isup" | "isdown" | "connected" | "name" | "x" | "y" | "z" | "r" | "u" | "v" | "buttons" | "pov" => {
                Some(Err(format!("{prop} is a read-only value")))
            }
            _ => None,
        }
    }

    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        match method {
            "update" => {
                self.update();
                Some(Value::Null)
            }
            // Button(n): button n (1 … 32) down.
            "button" => {
                let n = args.first().map_or(0, Value::to_i64);
                Some(flag((1..=32).contains(&n) && self.read().buttons & (1 << (n - 1)) != 0))
            }
            _ => None,
        }
    }

    /// The events' look (when the program has a handler for one): the
    /// joystick read, the state Update reads too, and what changed since
    /// the last look — OnButtonUp / OnButtonDown for each button (by
    /// number), then OnMove when an axis or the POV moved. A joystick
    /// plugged in or out changes from (or to) the resting state.
    pub fn look(&mut self) -> Vec<(&'static str, Vec<Value>)> {
        self.update();
        let now = self.read();
        let before = self.seen.clone().unwrap_or_else(|| Pad::resting(""));
        self.seen = Some(now.clone());
        let mut events = Vec::new();
        for n in 1..=32u32 {
            let bit = 1 << (n - 1);
            match (before.buttons & bit != 0, now.buttons & bit != 0) {
                (true, false) => events.push(("onbuttonup", vec![v_int(n as i64)])),
                (false, true) => events.push(("onbuttondown", vec![v_int(n as i64)])),
                _ => {}
            }
        }
        if before.axes != now.axes || before.pov != now.pov {
            events.push(("onmove", Vec::new()));
        }
        events
    }
}

/// The events QDXJOYSTICK's look fires.
pub const EVENTS: [&str; 3] = ["onbuttondown", "onbuttonup", "onmove"];

/// How often the runtimes look for the events (ms).
pub const LOOK_MS: u64 = 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_layout() {
        let mut s = Standard::new("Pad");
        s.axes = [-1.0, 1.0, 0.0, -1.0];
        s.buttons[0] = (true, 1.0);
        s.buttons[9] = (true, 1.0);
        s.buttons[6] = (false, 1.0);
        s.buttons[12] = (true, 1.0);
        s.buttons[15] = (true, 1.0);
        let p = Pad::from_standard(&s);
        assert_eq!(p.axes, [0, 65535, 65535, 0, CENTRE, CENTRE], "X left, Y down, Z the left trigger, R up, U the right stick at rest");
        assert_eq!(p.buttons, 1 | 1 << 7, "A is 1, Start is 8");
        assert_eq!(p.pov, 4500, "up and right");
        assert_eq!(Pad::from_standard(&Standard::new("")).pov, POV_CENTRED);
    }

    #[test]
    fn script_and_rapidq_members() {
        let mut j = DxJoystick::default();
        // No source: nothing connected, everything at rest — never an error.
        j.update();
        assert_eq!(j.get("connected").unwrap().to_i64(), 0);
        assert_eq!(j.get("isleft").unwrap().to_i64(), 0);
        assert_eq!(j.call("button", &[v_int(1)]).unwrap().to_i64(), 0);
        set_source(Box::new(Script::parse("x=0,b=5,name=Pad;x=65535,y=0;x=32767/x=1;-").unwrap()));
        j.update();
        assert_eq!((j.get("isleft").unwrap().to_i64(), j.get("isright").unwrap().to_i64(), j.get("connected").unwrap().to_i64()), (-1, 0, -1));
        assert_eq!((j.call("button", &[v_int(1)]).unwrap().to_i64(), j.call("button", &[v_int(2)]).unwrap().to_i64(), j.call("button", &[v_int(3)]).unwrap().to_i64()), (-1, 0, -1));
        assert_eq!(j.get("name").unwrap().to_string_val(), "Pad");
        // (the state stays until the next Update)
        assert_eq!(j.get("x").unwrap().to_i64(), 0);
        j.update();
        assert_eq!((j.get("isright").unwrap().to_i64(), j.get("isup").unwrap().to_i64(), j.get("buttons").unwrap().to_i64()), (-1, -1, 0));
        // Index 1: the second gamepad of the step.
        j.set("index", &v_int(1)).unwrap().unwrap();
        j.update();
        assert_eq!((j.get("x").unwrap().to_i64(), j.get("name").unwrap().to_string_val()), (1, "Test pad".to_string()));
        j.update();
        assert_eq!(j.get("connected").unwrap().to_i64(), 0, "unplugged");
        j.update();
        assert_eq!(j.get("connected").unwrap().to_i64(), 0, "the last step kept");
        assert!(j.set("isleft", &v_int(1)).unwrap().is_err(), "read-only");
    }

    #[test]
    fn events() {
        set_source(Box::new(Script::parse("b=1;b=3,x=0;b=2,x=0;-").unwrap()));
        let mut j = DxJoystick::default();
        let names = |e: Vec<(&'static str, Vec<Value>)>| e.into_iter().map(|(n, a)| format!("{n}{}", a.first().map_or(String::new(), |v| v.to_i64().to_string()))).collect::<Vec<_>>().join(" ");
        assert_eq!(names(j.look()), "onbuttondown1");
        assert_eq!(names(j.look()), "onbuttondown2 onmove");
        assert_eq!(names(j.look()), "onbuttonup1");
        assert_eq!(names(j.look()), "onbuttonup2 onmove", "unplugged: back to rest");
        assert_eq!(names(j.look()), "");
    }

    #[test]
    fn bad_scripts() {
        assert!(Script::parse("x=left").is_err());
        assert!(Script::parse("w=1").is_err());
        assert!(Script::parse("x").is_err());
        assert_eq!(Script::parse("").unwrap().pads(), Vec::new());
    }
}
