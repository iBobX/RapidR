//! INKEY$ on a terminal (console programs): the keys pressed, read without
//! waiting and without echo. On Unix the terminal leaves line mode at the
//! first INKEY$ ([`read_keys`]), goes back to it for INPUT ([`line_mode`])
//! and when the program ends; on Windows the C runtime's `_kbhit` /
//! `_getch` read the console. From a pipe or a file, INPUT$ reads its bytes
//! as they come. Keys go to rapidr_value::console's queue.

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

    /// Sleeps until the terminal has a key (true), or stdin ends (false).
    /// Not a terminal: one byte read as it comes (a pipe, a file).
    pub fn wait() -> bool {
        if !char_mode() {
            let mut b = [0u8; 1];
            // SAFETY: a blocking read of one byte into a local buffer.
            let n = unsafe { libc::read(0, b.as_mut_ptr().cast(), 1) };
            if n == 1 {
                rapidr_value::console::push_key(char::from(b[0]).to_string());
                return true;
            }
            return false;
        }
        let mut fd = libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 };
        // SAFETY: poll on one local pollfd, no timeout: the key wakes it.
        let r = unsafe { libc::poll(&mut fd, 1, -1) };
        if r < 0 {
            return std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted;
        }
        if fd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 && fd.revents & libc::POLLIN == 0 {
            return false;
        }
        let keys = read();
        if keys.is_empty() && fd.revents & libc::POLLIN != 0 {
            // (readable but nothing: the end of input)
            return false;
        }
        for k in keys {
            rapidr_value::console::push_key(k);
        }
        true
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

    /// Whether stdin is the console (not a pipe or a file).
    fn console() -> bool {
        std::io::IsTerminal::is_terminal(&std::io::stdin())
    }

    pub fn read() -> Vec<String> {
        let mut keys = Vec::new();
        if !console() {
            return keys;
        }
        // SAFETY: the C runtime's console functions.
        unsafe {
            while _kbhit() != 0 && keys.len() < 256 {
                let c = _getch();
                if c == 0 || c == 0xE0 {
                    // An extended key: CHR$(27) + its scan code, as RapidQ.
                    keys.push(format!("\x1b{}", char::from(_getch() as u8)));
                } else {
                    keys.push(char::from(c as u8).to_string());
                }
            }
        }
        keys
    }

    /// Sleeps in `_getch` until a key is pressed. Not the console: one
    /// byte read as it comes (a pipe, a file; false when it ends).
    pub fn wait() -> bool {
        if !console() {
            let mut b = [0u8; 1];
            return match std::io::Read::read(&mut std::io::stdin().lock(), &mut b) {
                Ok(1) => {
                    rapidr_value::console::push_key(char::from(b[0]).to_string());
                    true
                }
                _ => false,
            };
        }
        // SAFETY: the C runtime's console functions.
        let key = unsafe {
            let c = _getch();
            if c == 0 || c == 0xE0 {
                format!("\x1b{}", char::from(_getch() as u8))
            } else {
                char::from(c as u8).to_string()
            }
        };
        rapidr_value::console::push_key(key);
        true
    }

    pub fn line_mode() {}

    /// The console shows the escape sequences RapidR prints for CLS,
    /// LOCATE and COLOR (Windows' console only does once asked to — RapidQ
    /// drew with the console's own functions, so its programs looked right
    /// there).
    pub fn start() {
        use windows_sys::Win32::System::Console::{
            GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_PROCESSED_OUTPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING, STD_OUTPUT_HANDLE,
        };
        // SAFETY: the process's standard output handle (or an invalid one,
        // which the console calls refuse), its mode read into a local and
        // set with one more flag; no memory is shared.
        unsafe {
            let h = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut mode = 0u32;
            if GetConsoleMode(h, &mut mode) != 0 {
                SetConsoleMode(h, mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            }
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod imp {
    pub fn read() -> Vec<String> {
        Vec::new()
    }
    pub fn wait() -> bool {
        false
    }
    pub fn line_mode() {}
}

/// Once, as the program starts: the terminal ready for what the program
/// prints (Windows' console told to show escape sequences).
pub fn start() {
    #[cfg(windows)]
    {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(imp::start);
    }
}

/// Reads the keys pressed so far into INKEY$'s queue.
pub fn read_keys() {
    for k in imp::read() {
        rapidr_value::console::push_key(k);
    }
}

/// Sleeps until a key is pressed in the terminal (INKEY$ then has it);
/// false when no key can come (stdin ended).
pub fn wait_key() -> bool {
    rapidr_value::console::key_waiting() || imp::wait()
}

/// The terminal back in line mode (INPUT reads a line; the program ends).
pub fn line_mode() {
    imp::line_mode();
}
