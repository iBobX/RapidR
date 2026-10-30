//! Where the RapidR program is: the file, line and SUB the interpreter ran
//! last (it notes each call into the runtime: [`at`], a relaxed atomic
//! store), the event handlers running ([`event_enter`]), and the program's
//! map from code to lines ([`set_program_map`]), kept for good so that a
//! signal handler can read it without a lock or an allocation.
//!
//! Also the test hook `RAPIDR_TEST_CRASH=<kind>@[file:]line` (a panic, a
//! stack overflow, an abort at the first call into the runtime on that
//! line): how the tests crash a program for real.

use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::OnceLock;

use crate::slots::Slot;

/// The interpreter's last call into the runtime: function index << 32 |
/// instruction offset; `u64::MAX` before any.
static AT: AtomicU64 = AtomicU64::new(u64::MAX);

/// The program's map (leaked on purpose: read by a signal handler).
static MAP: AtomicPtr<ProgramMap> = AtomicPtr::new(std::ptr::null_mut());

/// How many event handlers run inside each other now.
static DEPTH: AtomicUsize = AtomicUsize::new(0);
const EVENT_SLOTS: usize = 8;
/// Each running handler's event and component (`ontimer` 0x1F `Clock`), the
/// outermost first.
static EVENTS: [Slot<96>; EVENT_SLOTS] = [const { Slot::new() }; EVENT_SLOTS];
static HANDLERS: [AtomicU32; EVENT_SLOTS] = [const { AtomicU32::new(u32::MAX) }; EVENT_SLOTS];

/// A function of the compiled program.
#[derive(Clone, Debug, Default)]
pub struct FnInfo {
    /// Its name as written (`__main` for the main program).
    pub name: String,
    /// A FUNCTION (it has a result), else a SUB.
    pub function: bool,
    /// (instruction offset, compiled line), in code order.
    pub lines: Vec<(u32, u32)>,
}

/// The compiled program's map: its functions, and its compiled lines' files
/// (names only, never paths) and lines.
#[derive(Clone, Debug, Default)]
pub struct ProgramMap {
    pub functions: Vec<FnInfo>,
    pub files: Vec<String>,
    /// (first compiled line, file index, its line in that file, count)
    pub runs: Vec<(u32, u32, u32, u32)>,
    /// The names as the program writes them (`Clock`): the runtime knows
    /// its components in lower case.
    pub names: Vec<String>,
}

/// `name` as the program writes it (a name the map knows, any case), else
/// as it is.
pub fn spelled(name: &str) -> &str {
    map().and_then(|m| m.names.iter().find(|n| n.eq_ignore_ascii_case(name))).map_or(name, String::as_str)
}

/// The program that runs now (a new one replaces it; the old map stays in
/// memory, never freed: a signal handler may still be reading it).
pub fn set_program_map(map: ProgramMap) {
    let p = Box::into_raw(Box::new(map));
    MAP.store(p, Ordering::Release);
    test::arm();
}

fn map() -> Option<&'static ProgramMap> {
    let p = MAP.load(Ordering::Acquire);
    // SAFETY: every pointer stored is a leaked Box, never freed.
    (!p.is_null()).then(|| unsafe { &*p })
}

/// The interpreter is about to call into the runtime from `function` at
/// instruction `ip`.
#[inline]
pub fn at(function: u32, ip: u32) {
    AT.store((u64::from(function) << 32) | u64::from(ip), Ordering::Relaxed);
    if test::ARMED.load(Ordering::Relaxed) {
        test::check(function, ip);
    }
}

/// An event's handler runs (until the guard goes): `event` as the runtime
/// fired it (`ontimer`), `component` its Sender (`Clock`), `handler` its
/// function.
pub fn event_enter(handler: u32, event: &str, component: &str) -> EventGuard {
    let d = DEPTH.load(Ordering::Relaxed);
    if d < EVENT_SLOTS {
        EVENTS[d].set_parts(&[event, "\u{1f}", component]);
        HANDLERS[d].store(handler, Ordering::Relaxed);
    }
    DEPTH.store(d + 1, Ordering::Release);
    EventGuard(d)
}

/// What [`event_enter`] gives: dropping it, the handler has ended.
pub struct EventGuard(usize);

impl Drop for EventGuard {
    fn drop(&mut self) {
        DEPTH.store(self.0, Ordering::Release);
    }
}

/// A place in the program, as written (no allocation: the strings are the
/// map's).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Place {
    pub file: &'static str,
    pub line: u32,
    /// The SUB or FUNCTION ("" for the main program).
    pub routine: &'static str,
    pub function: bool,
}

/// Where `function` at `ip` is.
pub fn resolve(function: u32, ip: u32) -> Option<Place> {
    let map = map()?;
    let f = map.functions.get(function as usize)?;
    let after = f.lines.partition_point(|&(off, _)| off <= ip);
    let compiled = f.lines.get(after.checked_sub(1)?)?.1;
    let (file, line) = map.runs.iter().find(|r| (r.0..r.0.saturating_add(r.3)).contains(&compiled)).and_then(|r| Some((map.files.get(r.1 as usize)?.as_str(), r.2 + (compiled - r.0)))).unwrap_or(("", compiled));
    let routine = if f.name.starts_with("__") { "" } else { f.name.as_str() };
    Some(Place { file, line, routine, function: f.function })
}

/// Where the interpreter called into the runtime last.
pub fn current() -> Option<Place> {
    let v = AT.load(Ordering::Relaxed);
    if v == u64::MAX {
        return None;
    }
    resolve((v >> 32) as u32, v as u32)
}

/// How deep event handlers run now, and the innermost one kept: its event
/// and component (raw: `ontimer`, `Clock`) into `event` / `component`
/// (their lengths).
pub fn innermost_event(event: &mut [u8], component: &mut [u8]) -> (usize, usize, usize) {
    let depth = DEPTH.load(Ordering::Acquire);
    if depth == 0 {
        return (0, 0, 0);
    }
    let slot = (depth - 1).min(EVENT_SLOTS - 1);
    let mut buf = [0u8; 96];
    let n = EVENTS[slot].read_into(&mut buf);
    let sep = buf[..n].iter().position(|&b| b == 0x1f).unwrap_or(n);
    let e = sep.min(event.len());
    event[..e].copy_from_slice(&buf[..e]);
    let rest = if sep < n { &buf[sep + 1..n] } else { &[][..] };
    let c = rest.len().min(component.len());
    component[..c].copy_from_slice(&rest[..c]);
    (depth, e, c)
}

static TITLER: OnceLock<fn(&str) -> String> = OnceLock::new();

/// How an event's name is written for people (`ontimer` → `OnTimer`): the
/// host's (`rapidr_value::events::event_title`).
pub fn set_event_titler(f: fn(&str) -> String) {
    let _ = TITLER.set(f);
}

/// The innermost event, as people read it: `OnTimer of Clock` ("" when no
/// handler runs), and how deep.
pub fn event_text() -> (String, usize) {
    let mut e = [0u8; 96];
    let mut c = [0u8; 96];
    let (depth, en, cn) = innermost_event(&mut e, &mut c);
    if depth == 0 {
        return (String::new(), 0);
    }
    let raw = String::from_utf8_lossy(&e[..en]).into_owned();
    let event = match TITLER.get() {
        Some(t) => t(&raw),
        None => raw,
    };
    let comp = spelled(&String::from_utf8_lossy(&c[..cn])).to_string();
    (if comp.is_empty() { event } else { format!("{event} of {comp}") }, depth)
}

/// The place in words: `main.rr line 120, in SUB Tick (OnTimer of Clock)`.
pub fn describe(place: Option<Place>) -> String {
    let mut out = String::new();
    if let Some(p) = place {
        if !p.file.is_empty() {
            out.push_str(&format!("{} line {}", p.file, p.line));
        } else {
            out.push_str(&format!("line {}", p.line));
        }
        if !p.routine.is_empty() {
            out.push_str(&format!(", in {} {}", if p.function { "FUNCTION" } else { "SUB" }, p.routine));
        } else {
            out.push_str(", in the main program");
        }
    }
    let (event, depth) = event_text();
    if !event.is_empty() {
        if out.is_empty() {
            out.push_str("in a handler");
        }
        if depth > 1 {
            out.push_str(&format!(" ({event}, {depth} handlers deep)"));
        } else {
            out.push_str(&format!(" ({event})"));
        }
    }
    out
}

/// Forgets the program (tests).
pub fn reset() {
    AT.store(u64::MAX, Ordering::Relaxed);
    DEPTH.store(0, Ordering::Relaxed);
}

/// `RAPIDR_TEST_CRASH`: a real crash where a test asks for it.
pub(crate) mod test {
    use super::*;

    pub(crate) static ARMED: AtomicBool = AtomicBool::new(false);
    static WHAT: OnceLock<(String, String, u32)> = OnceLock::new();

    /// Reads `RAPIDR_TEST_CRASH` (`panic@12`, `overflow@main.rr:12`,
    /// `abort@12`) when a program's map is set.
    pub(crate) fn arm() {
        let Ok(spec) = rapidr_testmode::env_var("RAPIDR_TEST_CRASH") else { return };
        let Some((kind, at)) = spec.split_once('@') else { return };
        let (file, line) = match at.rsplit_once(':') {
            Some((f, l)) => (f.to_string(), l),
            None => (String::new(), at),
        };
        let Ok(line) = line.trim().parse::<u32>() else { return };
        if WHAT.set((kind.trim().to_ascii_lowercase(), file, line)).is_ok() {
            ARMED.store(true, Ordering::Release);
        }
    }

    pub(crate) fn check(function: u32, ip: u32) {
        let Some((kind, file, line)) = WHAT.get() else { return };
        let Some(p) = resolve(function, ip) else { return };
        if p.line != *line || !(file.is_empty() || p.file.eq_ignore_ascii_case(file)) {
            return;
        }
        ARMED.store(false, Ordering::Release);
        crate::test_crash(kind);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_and_events_in_words() {
        set_program_map(ProgramMap {
            functions: vec![
                FnInfo { name: "__main".into(), function: false, lines: vec![(0, 1), (10, 2)] },
                FnInfo { name: "Tick".into(), function: false, lines: vec![(0, 5), (8, 6)] },
                FnInfo { name: "Area".into(), function: true, lines: vec![(0, 9)] },
            ],
            files: vec!["main.rr".into(), "clock.inc".into()],
            runs: vec![(1, 0, 1, 4), (5, 1, 20, 10)],
            names: vec!["Clock".into()],
        });
        assert_eq!(resolve(1, 9), Some(Place { file: "clock.inc", line: 21, routine: "Tick", function: false }));
        assert_eq!(describe(resolve(0, 12)), "main.rr line 2, in the main program");
        at(1, 9);
        let g = event_enter(1, "ontimer", "clock");
        assert_eq!(describe(current()), "clock.inc line 21, in SUB Tick (ontimer of Clock)");
        {
            let _inner = event_enter(2, "onpaint", "Canvas1");
            assert_eq!(event_text(), ("onpaint of Canvas1".to_string(), 2));
        }
        assert_eq!(event_text(), ("ontimer of Clock".to_string(), 1));
        drop(g);
        assert_eq!(event_text(), (String::new(), 0));
        assert_eq!(describe(resolve(2, 0)), "clock.inc line 24, in FUNCTION Area");
        reset();
    }
}
