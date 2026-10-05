//! QDXJOYSTICK's gamepads on the desktop (docs/directx-plan.md §2.4): the
//! source `rapidr_value::objects::joystick` reads them from, installed when
//! a program makes its first QDXJOYSTICK — gilrs on Windows and macOS (the
//! `gamepad` feature), the kernel's evdev on Linux (`evdev.rs`: libc only,
//! no libudev to build against), `RAPIDR_TEST_JOYSTICK`'s script for the
//! tests (none under the GUI tests without one: no real device is opened).
//! No device, or no permission to read one: no gamepad, never an error.

use rapidr_value::objects::joystick::{set_source, Pad, Script, Source};

#[cfg(target_os = "linux")]
mod evdev;

/// No gamepads (no backend, or the tests without a script).
struct NoPads;

impl Source for NoPads {
    fn pads(&mut self) -> Vec<Pad> {
        Vec::new()
    }
}

/// The source, once: the first QDXJOYSTICK made.
pub fn install() {
    thread_local! {
        static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    if let Some(text) = std::env::var_os("RAPIDR_TEST_JOYSTICK") {
        match Script::parse(&text.to_string_lossy()) {
            Ok(script) => set_source(Box::new(script)),
            Err(e) => {
                eprintln!("[rapidr] {e}");
                set_source(Box::new(NoPads));
            }
        }
        return;
    }
    let testing = std::env::var_os("RAPIDR_CAPTURE").is_some() || std::env::var_os("RAPIDR_TEST_EVENTS").is_some();
    if testing {
        set_source(Box::new(NoPads));
        return;
    }
    set_source(device_source());
}

#[cfg(all(feature = "gamepad", any(windows, target_os = "macos")))]
fn device_source() -> Box<dyn Source> {
    match gilrs::Gilrs::new() {
        Ok(g) => Box::new(Gilrs(g)),
        Err(_) => Box::new(NoPads),
    }
}

#[cfg(target_os = "linux")]
fn device_source() -> Box<dyn Source> {
    Box::new(evdev::Evdev::default())
}

#[cfg(not(any(target_os = "linux", all(feature = "gamepad", any(windows, target_os = "macos")))))]
fn device_source() -> Box<dyn Source> {
    Box::new(NoPads)
}

/// gilrs's gamepads (its SDL mappings make every known controller the
/// standard layout).
#[cfg(all(feature = "gamepad", any(windows, target_os = "macos")))]
struct Gilrs(gilrs::Gilrs);

#[cfg(all(feature = "gamepad", any(windows, target_os = "macos")))]
impl Source for Gilrs {
    fn pads(&mut self) -> Vec<Pad> {
        use gilrs::{Axis, Button};
        use rapidr_value::objects::joystick::Standard;
        // (gilrs keeps each gamepad's state from its events)
        while self.0.next_event().is_some() {}
        self.0
            .gamepads()
            .filter(|(_, g)| g.is_connected())
            .map(|(_, g)| {
                let mut s = Standard::new(g.name());
                // (gilrs's y is up; the standard's is down)
                s.axes = [g.value(Axis::LeftStickX), -g.value(Axis::LeftStickY), g.value(Axis::RightStickX), -g.value(Axis::RightStickY)];
                let order = [
                    Button::South,
                    Button::East,
                    Button::West,
                    Button::North,
                    Button::LeftTrigger,
                    Button::RightTrigger,
                    Button::LeftTrigger2,
                    Button::RightTrigger2,
                    Button::Select,
                    Button::Start,
                    Button::LeftThumb,
                    Button::RightThumb,
                    Button::DPadUp,
                    Button::DPadDown,
                    Button::DPadLeft,
                    Button::DPadRight,
                    Button::Mode,
                ];
                for (i, b) in order.into_iter().enumerate() {
                    s.buttons[i] = (g.is_pressed(b), g.button_data(b).map_or(0.0, |d| d.value()));
                }
                Pad::from_standard(&s)
            })
            .collect()
    }
}

/// An evdev gamepad as the standard layout: the kernel's codes
/// (Documentation/input/gamepad.rst) read as xpad (Xbox) reports them —
/// BTN_A / BTN_SOUTH A, BTN_B B, BTN_X (0x133) X, BTN_Y (0x134) Y, BTN_TL /
/// BTN_TR the shoulders, BTN_TL2 / BTN_TR2 the triggers' clicks, BTN_SELECT,
/// BTN_START, BTN_THUMBL / R, BTN_MODE the Guide button; ABS_X / ABS_Y the
/// left stick, ABS_RX / ABS_RY the right one, ABS_Z / ABS_RZ the triggers,
/// ABS_HAT0X / Y (or BTN_DPAD_*) the D-pad. `axes`: each ABS code's (value,
/// minimum, maximum), when the device has it.
#[cfg(any(target_os = "linux", test))]
fn standard_of_evdev(name: &str, axes: &[Option<(i32, i32, i32)>; 0x12], keys: &std::collections::HashSet<u16>) -> rapidr_value::objects::joystick::Standard {
    use rapidr_value::objects::joystick::Standard;
    let mut s = Standard::new(name);
    // (−1 … 1 over the axis's range; a trigger 0 … 1)
    let stick = |code: usize| axes[code].map_or(0.0, |(v, lo, hi)| if hi > lo { (v - lo) as f32 / (hi - lo) as f32 * 2.0 - 1.0 } else { 0.0 });
    let trigger = |code: usize| axes[code].map_or(0.0, |(v, lo, hi)| if hi > lo { ((v - lo) as f32 / (hi - lo) as f32).clamp(0.0, 1.0) } else { 0.0 });
    s.axes = [stick(0x00), stick(0x01), stick(0x03), stick(0x04)];
    let down = |code: u16| keys.contains(&code);
    // (standard buttons 0 … 11, in order)
    for (i, code) in [0x130u16, 0x131, 0x133, 0x134, 0x136, 0x137, 0x138, 0x139, 0x13a, 0x13b, 0x13d, 0x13e].into_iter().enumerate() {
        s.buttons[i] = (down(code), 0.0);
    }
    s.buttons[16] = (down(0x13c), 0.0);
    let (lt, rt) = (trigger(0x02), trigger(0x05));
    s.buttons[6] = (s.buttons[6].0 || lt > 0.5, lt);
    s.buttons[7] = (s.buttons[7].0 || rt > 0.5, rt);
    let hat = |code: usize| axes[code].map_or(0, |(v, _, _)| v.signum());
    let (hx, hy) = (hat(0x10), hat(0x11));
    s.buttons[12] = (hy < 0 || down(0x220), 0.0);
    s.buttons[13] = (hy > 0 || down(0x221), 0.0);
    s.buttons[14] = (hx < 0 || down(0x222), 0.0);
    s.buttons[15] = (hx > 0 || down(0x223), 0.0);
    s
}

#[cfg(test)]
mod tests {
    use rapidr_value::objects::joystick::Pad;

    #[test]
    fn evdev_codes_as_an_xbox_pad() {
        let mut axes = [None; 0x12];
        axes[0x00] = Some((-32768, -32768, 32767));
        axes[0x01] = Some((0, -32768, 32767));
        axes[0x02] = Some((255, 0, 255));
        axes[0x05] = Some((0, 0, 255));
        axes[0x10] = Some((1, -1, 1));
        axes[0x11] = Some((-1, -1, 1));
        let keys = [0x130u16, 0x134, 0x13b].into_iter().collect();
        let p = Pad::from_standard(&super::standard_of_evdev("Xbox", &axes, &keys));
        assert_eq!(p.axes[0], 0, "X left");
        assert_eq!(p.axes[2], 65535, "Z: the left trigger pulled");
        assert_eq!(p.buttons, 1 | 1 << 3 | 1 << 7, "A 1, Y 4, Start 8");
        assert_eq!(p.pov, 4500, "the hat up and right");
        assert_eq!(p.name, "Xbox");
    }
}

