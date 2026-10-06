//! The program's environment strings: `ENVIRON$(name)` reads one, the
//! `ENVIRON "name=text"` statement sets one (as RapidQ's manual describes
//! it: the name is what comes before the first `=` or space, the value what
//! follows). The process's own environment on the
//! desktop — a program it starts (SHELL, RUN) sees them, as RapidQ's
//! SetEnvironmentVariable gave them on; a table of the page's own in the
//! browser, which has no environment (empty until the program sets one).
//! QCGI reads the CGI variables from here (objects::cgi).

#[cfg(target_arch = "wasm32")]
thread_local! {
    static PAGE: std::cell::RefCell<std::collections::HashMap<String, String>> = std::cell::RefCell::new(Default::default());
}

/// `ENVIRON$(name)`: the variable's text, "" when it isn't set.
pub fn get(name: &str) -> String {
    if name.is_empty() || name.contains(['=', '\0']) {
        return String::new();
    }
    // (any case, as Windows' — RapidQ's — names are: the exact name first)
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var_os(name)
            .or_else(|| std::env::vars_os().find(|(k, _)| k.to_str().is_some_and(|k| k.eq_ignore_ascii_case(name))).map(|(_, v)| v))
            .map(|v| v.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
    #[cfg(target_arch = "wasm32")]
    {
        PAGE.with(|p| p.borrow().get(&name.to_ascii_uppercase()).cloned().unwrap_or_default())
    }
}

/// The name and text of an `ENVIRON` statement's string: split at its first
/// `=`, or without one at its first space (RapidQ: `"Z = sp"` is "Z " set
/// to " sp"); `None` without either, or without a name.
pub fn split(spec: &str) -> Option<(&str, &str)> {
    let at = spec.find('=').or_else(|| spec.find(' '))?;
    let name = &spec[..at];
    (!name.trim().is_empty()).then(|| (name, &spec[at + 1..]))
}

/// `ENVIRON "name=text"`: the variable set (an empty text takes it away).
/// A string without `=` or a space, or without a name, changes nothing.
pub fn set(spec: &str) {
    let Some((name, text)) = split(spec) else { return };
    if name.contains('\0') || text.contains('\0') {
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if text.is_empty() {
            std::env::remove_var(name);
        } else {
            std::env::set_var(name, text);
        }
    }
    #[cfg(target_arch = "wasm32")]
    PAGE.with(|p| {
        let mut p = p.borrow_mut();
        if text.is_empty() {
            p.remove(&name.to_ascii_uppercase());
        } else {
            p.insert(name.to_ascii_uppercase(), text.to_string());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statement_strings() {
        assert_eq!(split("PATH=c:\\windows"), Some(("PATH", "c:\\windows")));
        assert_eq!(split("TEST what"), Some(("TEST", "what")));
        assert_eq!(split("Q=a=b c"), Some(("Q", "a=b c")));
        assert_eq!(split("A B=C"), Some(("A B", "C")));
        assert_eq!(split("Z = sp"), Some(("Z ", " sp")));
        assert_eq!(split("=x"), None);
        assert_eq!(split("NOTHING"), None);
    }

    #[test]
    fn set_and_get() {
        set("RAPIDR_ENVIRON_TEST=a=1&b=2");
        assert_eq!(get("RAPIDR_ENVIRON_TEST"), "a=1&b=2");
        set("RAPIDR_ENVIRON_TEST=");
        assert_eq!(get("RAPIDR_ENVIRON_TEST"), "");
        assert_eq!(get(""), "");
    }
}
