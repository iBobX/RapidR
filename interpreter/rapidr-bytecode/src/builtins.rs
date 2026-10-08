//! The builtin functions every VM host implements (`CallBuiltin` targets).
//!
//! The bytecode generator uses this list to reject calls to unknown names at
//! compile time — otherwise a typo, or a statement the VM doesn't support
//! (e.g. `INC i`), would compile to a call that silently does nothing.
//! A test keeps it identical to the native and web hosts' dispatch tables.

/// Normalized names (see [`builtin_key`]).
pub const BUILTINS: &[&str] = &[
    "__gui_register_timer", "__set_theme",
    // Arrays of components and dynamic event binding (bcgen)
    "__component_array", "__bind_event",
    // SUBI / FUNCTIONI arguments (the parser inserts these calls)
    "__pack", "__paramstr", "__paramstrcount", "__paramval", "__paramvalcount",
    // DATA / READ / RESTORE (the parser inserts these calls)
    "__data_add", "__data_label", "__data_reset", "__read", "__restore",
    // REDIM (resize keeping data)
    "__redim", "__input_value", "__decimal", "__environ_set", "__inkey_trapall", "__quicksort", "__lprint", "lflush",
    "atan", "tab", "get", "setconsoletitle", "chdrive",
    // Objects (rapidr_ast::objects; shared in rapidr_value)
    "__newobject", "__getfield", "__setfield", "__objectarray", "__newarray", "__aget", "__aset", "__null",
    // A component type's name as a value (rapidr_ast::type_values; rapidr_value)
    "__lastoftype",
    // The system tray (rapidr_ast::tray_calls; rapidr_value::tray)
    "__shell_notifyicon",
    // Memory: VARPTR, MEMCPY, SIZEOF, … (rapidr_ast::memory; rapidr_value::memory)
    "__varptr_var", "__varptr_elem", "__mem_refresh", "__mem_sync", "__sizeof", "__sizeof_type", "__cstring", "memcpy", "memset", "memcmp",
    // Stores into declared numeric types (rapidr_ast::numeric; shared in rapidr_value)
    "__to_byte", "__to_word", "__to_short", "__to_long", "__to_dword", "__to_double", "__to_single", "__to_fixed", "__arg_round",
    // Components reached through objects (rapidr_ast::objects; hosts)
    "__objget", "__objset", "__objcall", "__objcreate", "__bind_event_this",
    "abs", "acos", "asc", "asin", "atn",
    "beep", "bin",
    "cbool", "cdbl", "ceil", "chdir", "chr", "cint", "cls", "clng", "close", "color", "command", "commandcount", "convbase",
    "cos", "csng", "csrlin", "curdir",
    "date", "date_func", "delete", "dir", "direxists", "doevents", "inkey",
    "e", "end", "environ", "eof", "exp",
    "extractresource", "field", "fileexists", "filelen", "fix", "floor", "format", "frac", "freefile",
    "hex", "hextodec",
    "iif", "input", "input_func", "insert", "instr", "int", "inv", "isnumeric",
    "kill",
    "input_field", "lbound", "lcase", "left", "len", "line_input", "locate", "lof", "log", "ltrim",
    "math.e", "math.pi", "messagebox", "messagedlg", "mid", "mkdir", "mousex", "mousey", "msgbox",
    "oct", "open",
    "pi", "playsound", "playwav", "pos", "print", "print_hash", "println",
    "rapidr__waitkey", "randomize", "rename", "resource", "resourcecount", "replace", "replacesubstr", "reverse", "rgb", "right", "rinstr", "rmdir",
    "rnd", "round", "rtrim",
    "run", "seek", "sgn", "shell", "shellwait", "shl", "shr", "showmessage", "sin", "sleep",
    "sound", "space", "sqr", "str", "strf", "string",
    "tally", "tan", "time", "time_func", "timer", "trim",
    "ubound", "ucase",
    "val", "vartype",
    "write_hash",
];

/// Builtins every host hands to `rapidr_value::shared_builtin` before its own
/// dispatch table (DATA / READ / RESTORE and REDIM share one implementation).
pub const SHARED_DATA_BUILTINS: &[&str] = &["__data_add", "__data_label", "__data_reset", "__read", "__restore", "__redim", "__input_value", "__decimal", "__inkey_trapall", "__quicksort", "__lprint", "lflush", "__environ_set", "__newobject", "__getfield", "__setfield", "__objectarray", "__newarray", "__aget", "__aset", "__null", "__lastoftype", "__shell_notifyicon", "__to_byte", "__to_word", "__to_short", "__to_long", "__to_dword", "__to_double", "__to_single", "__to_fixed", "__arg_round", "__varptr_var", "__varptr_elem", "__mem_refresh", "__mem_sync", "__sizeof", "__sizeof_type", "__cstring", "memcpy", "memset", "memcmp"];

/// Builtins that may be written without parentheses (`x = TIMER`): a bare
/// name that isn't a variable calls them with no arguments.
pub const BARE_BUILTINS: &[&str] = &["command", "commandcount", "csrlin", "curdir", "date", "dir", "freefile", "inkey", "mousex", "mousey", "pi", "resourcecount", "rnd", "time", "timer"];

/// Hosts dispatch on the lowercased name with one BASIC type suffix
/// (`$ % # & !`) removed, so `MID$`, `Mid` and `mid` are the same builtin.
pub fn builtin_key(name: &str) -> String {
    let mut key = name.to_ascii_lowercase();
    if matches!(key.chars().last(), Some('$' | '%' | '#' | '&' | '!')) && key.len() > 1 {
        key.pop();
    }
    key
}

pub fn is_builtin(name: &str) -> bool {
    BUILTINS.contains(&builtin_key(name).as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Names in the `match` arms of a host's builtin dispatch function
    /// (from `match lower.as_str() {` to its `_ =>` fallback).
    fn host_arms(source: &str, fn_name: &str) -> BTreeSet<String> {
        let start = source.find(fn_name).expect("dispatch fn not found");
        let body = &source[start..];
        let body = &body[body.find("match lower.as_str()").expect("dispatch match not found")..];
        let body = &body[..body.find("\n        _ =>").expect("fallback arm not found")];
        let mut names = BTreeSet::new();
        for line in body.lines() {
            let t = line.trim_start();
            if !t.starts_with('"') {
                continue;
            }
            let Some(pattern) = t.split("=>").next() else { continue };
            for part in pattern.split('|') {
                let part = part.trim();
                if let Some(name) = part.strip_prefix('"').and_then(|p| p.strip_suffix('"')) {
                    names.insert(name.to_string());
                }
            }
        }
        names
    }

    fn registry() -> BTreeSet<String> {
        BUILTINS.iter().map(|s| s.to_string()).collect()
    }

    /// Every name a host handles must be in the registry (after the host's
    /// own normalization), and every registry name must be handled.
    fn assert_matches(host: &str, source: &str, fn_name: &str) {
        let arms: BTreeSet<String> = host_arms(source, fn_name)
            .iter()
            .filter(|n| *n != "print#" && *n != "write#") // aliases of print_hash/write_hash
            .map(|n| builtin_key(n))
            .collect();
        let reg: BTreeSet<String> = registry()
            .into_iter()
            .filter(|n| !SHARED_DATA_BUILTINS.contains(&n.as_str()))
            .collect();
        let missing_in_registry: Vec<_> = arms.difference(&reg).collect();
        let missing_in_host: Vec<_> = reg.difference(&arms).collect();
        assert!(
            missing_in_registry.is_empty() && missing_in_host.is_empty(),
            "{host} host and BUILTINS differ\n  handled by host, not in BUILTINS: {missing_in_registry:?}\n  in BUILTINS, not handled by host: {missing_in_host:?}"
        );
    }

    #[test]
    fn registry_matches_native_host() {
        let src = include_str!("../../rapidr-vm-host-native/src/lib.rs");
        assert_matches("native", src, "fn call_builtin_native");
    }

    #[test]
    fn shared_data_builtins_are_handled_and_both_hosts_use_them() {
        // (LFLUSH among them: never on the machine's real printer)
        rapidr_value::objects::set_print_hook(|_| Ok(()));
        for name in SHARED_DATA_BUILTINS {
            assert!(BUILTINS.contains(name), "{name} missing from BUILTINS");
            let args = [rapidr_value::Value::String("x".into()), rapidr_value::Value::Integer(0)];
            assert!(rapidr_value::shared_builtin(name, &args).is_some(), "{name} not handled by rapidr_value::shared_builtin");
        }
        rapidr_value::data::reset();
        for host in [include_str!("../../rapidr-vm-host-native/src/lib.rs"), include_str!("../../rapidr-vm-host-web/src/lib.rs")] {
            assert!(host.contains("rapidr_value::shared_builtin(&key, args)"), "a host doesn't dispatch the shared builtins");
        }
    }

    #[test]
    fn registry_matches_web_host() {
        let src = include_str!("../../rapidr-vm-host-web/src/lib.rs");
        let arms: BTreeSet<String> = host_arms(src, "fn call_builtin_web").iter().map(|n| builtin_key(n)).collect();
        // File-handle I/O (OPEN/PRINT #...) is native-only for now; the web
        // host reports those as unknown at run time.
        let native_only: BTreeSet<String> = ["print_hash", "write_hash"]
            .iter()
            .chain(SHARED_DATA_BUILTINS)
            .map(|s| s.to_string())
            .collect();
        let expected: BTreeSet<String> = registry().difference(&native_only).cloned().collect();
        assert_eq!(arms, expected, "web host and BUILTINS (minus native-only) differ");
    }

    #[test]
    fn keys_are_case_and_suffix_insensitive() {
        assert!(is_builtin("MID$"));
        assert!(is_builtin("Len"));
        assert!(is_builtin("ucase$"));
        assert!(!is_builtin("INC"));
        assert!(!is_builtin("NoSuchRoutine"));
    }
}
