use super::*;

#[test]
fn days_round_trip() {
    for s in ["1970-01-01", "2000-02-29", "2026-10-30", "2027-12-31", "2100-03-01"] {
        assert_eq!(Day::parse(s).unwrap().to_string(), s);
    }
    assert_eq!(Day::parse("1970-01-01"), Some(Day(0)));
    assert_eq!(Day::parse("2026-10-31").unwrap().0 - Day::parse("2026-10-30").unwrap().0, 1);
    for bad in ["2026-02-30", "2025-02-29", "2026-13-01", "2026-00-10", "26-10-30", "2026/10/30", "2026-1-030", "+026-10-30", ""] {
        assert_eq!(Day::parse(bad), None, "{bad}");
    }
    assert!(today() > Day::parse("2026-01-01").unwrap());
}

#[test]
fn hex_both_ways() {
    assert_eq!(hex(&[0, 0xab, 0x10]), "00ab10");
    assert_eq!(unhex::<3>("00AB10"), Some([0, 0xab, 0x10]));
    assert_eq!(unhex::<3>("00ab1"), None);
    assert_eq!(unhex::<3>("00ab1g"), None);
}

#[test]
fn the_hooks_names() {
    for h in ["RAPIDR_CAPTURE", "RAPIDR_CAPTURE_DELAY", "RAPIDR_CAPTURE_WINDOWS", "RAPIDR_TEST_EVENTS", "RAPIDR_TEST_DUMP", "rapidr_test_dump", "RAPIDR_TEST_SYSTEM_LOOK", "RAPIDR_TEST_CRASH", "RAPIDR_TEST_MCP_TOKEN", "RAPIDR_SCHOOL_POLICY", "RAPIDR_AI_TEST_SERVER", "RAPIDR_LOGIN_SHELL"] {
        assert!(is_hook(h), "{h}");
    }
    // (not hooks: the token's own variable, and the runs' settings that
    // only narrow what a run touches)
    for n in ["RAPIDR_TEST_MODE", "RAPIDR_REGISTRY", "RAPIDR_PRINT_TO", "RAPIDR_AI_KEYSTORE", "RAPIDR_THEME", "PATH"] {
        assert!(!is_hook(n), "{n}");
    }
}

#[test]
fn the_compiled_in_key_file_reads() {
    // (a malformed test-mode.pub would quietly trust nothing: it must read)
    assert!(parse_public_key(TRUSTED_PUB).is_ok(), "crates/rapidr-testmode/test-mode.pub doesn't read: {:?}", parse_public_key(TRUSTED_PUB));
    assert_eq!(parse_public_key("public: none"), Ok(None));
    assert!(parse_public_key("# nothing").is_err());
    assert!(parse_public_key(&format!("public: {}", "0".repeat(64))).is_err());
    assert!(parse_public_key("public: abcd").is_err());
    let key = [7u8; 32];
    assert_eq!(parse_public_key(&public_key_file(&key)), Ok(Some(key)));
}

#[test]
fn machines_match_loosely() {
    assert!(same_machine("Roberts-Mac.local", "roberts-mac"));
    assert!(same_machine("BUILD1", "build1"));
    assert!(!same_machine("build1", "build2"));
}
