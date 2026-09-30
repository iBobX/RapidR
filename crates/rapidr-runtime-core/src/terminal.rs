//! INKEY$ on a terminal (console programs): the keys pressed, read without
//! waiting and without echo. On Unix the terminal leaves line mode at the
//! first INKEY$ ([`read_keys`]), goes back to it for INPUT ([`line_mode`])
//! and when the program ends; on Windows the C runtime's `_kbhit` /
//! `_getch` read the console. Keys go to rapidr_value::console's queue.

#[cfg(unix)]
mod imp {
    use std::sync::Mutex;

    /// The terminal's settings before INKEY$ changed them.
    static SAVED: Mutex<Option<libc::termios>> = Mutex::new(None);

    extern "C" fn restore_at_exit() {
        super::line_mode();
    }

    /// The terminal in character mode (no line editing, no echo, reads
    /// that don't wait); false when stdin isn't a terminal.
    fn char_mode() -> bool {
        // SAFETY: plain termios calls on stdin with a local struct.
        unsafe {
            if libc::isatty(0) == 0 {
                return false;
            }
            let mut saved = SAVED.lock().unwrap_or_else(|e| e.into_inner());
            if saved.is_some() {
                return true;
            }
            let mut t: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(0, &mut t) != 0 {
                return false;
            }
            let first = saved.is_none();
            *saved = Some(t);
            t.c_lflag &= !(libc::ICANON | libc::ECHO);
            t.c_cc[libc::VMIN] = 0;
            t.c_cc[libc::VTIME] = 0;
            libc::tcsetattr(0, libc::TCSANOW, &t);
            if first {
                libc::atexit(restore_at_exit);
            }
            true
        }
    }

    pub fn read() -> Vec<String> {
        if !char_mode() {
            return Vec::new();
        }
        let mut bytes = Vec::new();
        let mut buf = [0u8; 64];
        loop {
            // SAFETY: reading into a local buffer of its length.
            let n = unsafe { libc::read(0, buf.as_mut_ptr().cast(), buf.len()) };
            if n <= 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..n as usize]);
            if bytes.len() > 4096 {
                break;
            }
        }
        rapidr_value::console::terminal_keys(&bytes)
    }

    pub fn line_mode() {
        let saved = SAVED.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(t) = saved {
            // SAFETY: restoring the settings read from this terminal.
            unsafe {
                libc::tcsetattr(0, libc::TCSANOW, &t);
            }
        }
    }
}

#[cfg(windows)]
mod imp {
    extern "C" {
        fn _kbhit() -> i32;
        fn _getch() -> i32;
    }

    pub fn read() -> Vec<String> {
        let mut keys = Vec::new();
        // SAFETY: the C runtime's console functions.
        unsafe {
            while _kbhit() != 0 && keys.len() < 256 {
                let c = _getch();
                if c == 0 || c == 0xE0 {
                    // An extended key: CHR$(0) + its scan code, as QBasic.
                    keys.push(format!("\0{}", char::from(_getch() as u8)));
                } else {
                    keys.push(char::from(c as u8).to_string());
                }
            }
        }
        keys
    }

    pub fn line_mode() {}
}

#[cfg(not(any(unix, windows)))]
mod imp {
    pub fn read() -> Vec<String> {
        Vec::new()
    }
    pub fn line_mode() {}
}

/// Reads the keys pressed so far into INKEY$'s queue.
pub fn read_keys() {
    for k in imp::read() {
        rapidr_value::console::push_key(k);
    }
}

/// The terminal back in line mode (INPUT reads a line; the program ends).
pub fn line_mode() {
    imp::line_mode();
}
