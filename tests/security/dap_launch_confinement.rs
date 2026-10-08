// SEC-18 regressions for `rapidr dap` (docs/security-audit.md). Compiled
// into crates/rapidr-dap's unit tests (src/confine.rs, `mod
// security_regressions`); run by `tools/regress.sh security`
// (cargo test -p rapidr-dap security).
//
// A launch request's `env` went to the debuggee as given: a project's
// .vscode/launch.json could set LD_PRELOAD / DYLD_INSERT_LIBRARIES and have
// `rapidr` load its library when the user pressed F5; its `program` could
// name any file (`~/.ssh/id_rsa`), which `rapidr run` then compiled and
// quoted in its errors.

use super::*;
use std::collections::BTreeMap;

#[test]
fn security_sec18_loader_variables_never_reach_the_debuggee() {
    let mut env = BTreeMap::new();
    for (k, v) in [
        ("LD_PRELOAD", Some("/tmp/evil.so")),
        ("LD_LIBRARY_PATH", Some("/tmp")),
        ("DYLD_INSERT_LIBRARIES", Some("/tmp/evil.dylib")),
        ("dyld_library_path", Some("/tmp")),
        ("_RLD_LIST", Some("x")),
        ("Path", Some("C:\\evil")),
        ("GCONV_PATH", Some("/tmp")),
        ("__COMPAT_LAYER", Some("RunAsInvoker")),
        ("BAD=NAME", Some("1")),
        ("", Some("1")),
        ("1ABC", Some("1")),
        ("NUL_VALUE", Some("a\0b")),
        ("API_URL", Some("https://example.com")),
        ("RAPIDR_PRINT_TO", Some("/tmp/prints")),
        ("REMOVE_ME", None),
    ] {
        env.insert(k.to_string(), v.map(str::to_string));
    }
    let (passed, refused) = debuggee_env(&env);
    let passed: Vec<&str> = passed.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(passed, ["API_URL", "RAPIDR_PRINT_TO", "REMOVE_ME"], "refused: {refused:?}");
    for bad in ["LD_PRELOAD", "DYLD_INSERT_LIBRARIES", "dyld_library_path", "Path", "BAD=NAME", "NUL_VALUE"] {
        assert!(refused.iter().any(|r| r == bad), "{bad} not refused: {refused:?}");
    }
}

#[test]
fn security_sec18_a_launch_names_only_a_program() {
    let dir = std::env::temp_dir().join(format!("rapidr-dap-sec18-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let key = dir.join("id_rsa");
    std::fs::write(&key, "-----BEGIN PRIVATE KEY-----").unwrap();
    let prog = dir.join("hello.bas");
    std::fs::write(&prog, "PRINT 1").unwrap();
    assert!(check_program(&key).is_err(), "a launch ran a key file");
    assert!(check_program(&dir.join("x.so")).is_err());
    assert!(check_program(&prog).is_ok());
    assert!(check_program(&dir.join("missing.bas")).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
