//! SEC-24 regressions (docs/security/audit-preview4.md, docs/security/test-mode.md):
//! the test-mode token. Compiled into rapidr-testmode's tests
//! (`cargo test -p rapidr-testmode security_`).
//!
//! The fixture key is made here, from a fixed seed, in the tests only: no
//! build trusts it (the last test checks the compiled-in key isn't it), so a
//! fixture-signed token turns on nothing outside these tests —
//! tests/security/test_mode_release.mjs runs a release binary with one.

use crate::*;

/// (the fixture key's seed: tests only, never a real key)
const FIXTURE: [u8; 32] = [0x5e; 32];
const OTHER: [u8; 32] = [0x17; 32];

fn day(s: &str) -> Day {
    Day::parse(s).unwrap()
}

fn fixture() -> Key {
    public_of(&FIXTURE)
}

fn check(token: &str, today: &str, machine: Option<&str>) -> Result<Token, Refused> {
    verify(token, &fixture(), day(today), machine)
}

#[test]
fn security_a_fixture_signed_token_is_taken_by_the_fixture_key() {
    let t = sign(&FIXTURE, PURPOSE, day("2026-10-30"), 30, None);
    let token = check(&t, "2026-10-30", None).unwrap();
    assert_eq!(token.expires, day("2026-11-29"));
    assert_eq!(token.key_id, key_id(&fixture()));
    // (valid through its last day, not after)
    assert!(check(&t, "2026-11-29", None).is_ok());
    assert_eq!(check(&t, "2026-11-30", None), Err(Refused::Expired("2026-11-29".into())));
}

#[test]
fn security_expired_tokens_are_refused() {
    let t = sign(&FIXTURE, PURPOSE, day("2026-01-01"), 30, None);
    assert_eq!(check(&t, "2026-10-30", None), Err(Refused::Expired("2026-01-31".into())));
}

#[test]
fn security_tokens_for_another_purpose_are_refused() {
    for purpose in ["rapidr-release", "rapidr-test-mode2", "RAPIDR-TEST-MODE", "x"] {
        let t = sign(&FIXTURE, purpose, day("2026-10-30"), 30, None);
        assert_eq!(check(&t, "2026-10-30", None), Err(Refused::WrongPurpose(purpose.into())), "{purpose}");
    }
}

#[test]
fn security_a_token_runs_a_year_at_most() {
    let t = sign(&FIXTURE, PURPOSE, day("2026-10-30"), MAX_DAYS + 30, None);
    assert!(matches!(check(&t, "2026-10-30", None), Err(Refused::TooLong(_))));
    let t = sign(&FIXTURE, PURPOSE, day("2026-10-30"), MAX_DAYS, None);
    assert!(check(&t, "2026-10-30", None).is_ok());
}

#[test]
fn security_garbled_tokens_are_refused() {
    let good = sign(&FIXTURE, PURPOSE, day("2026-10-30"), 30, None);
    let garbled = |t: &str| matches!(check(t, "2026-10-30", None), Err(Refused::Garbled(_)));
    assert!(garbled(""));
    assert!(garbled("\u{0}\u{1}binary\u{ff}"));
    assert!(garbled(&good.replace("rapidr-test-mode 1", "rapidr-test-mode 2")));
    assert!(garbled(&good.replace("purpose:", "porpose:")));
    assert!(garbled(&format!("{good}purpose: {PURPOSE}\n")));
    assert!(garbled(&format!("{good}hooks: all\n")));
    assert!(garbled(&good.replace("expires: 2026-11-29", "expires: 2026-02-30")));
    assert!(garbled(&good.lines().filter(|l| !l.starts_with("signature:")).collect::<Vec<_>>().join("\n")));
    let sig = good.lines().find_map(|l| l.strip_prefix("signature: ")).unwrap();
    assert!(garbled(&good.replace(sig, &sig[..100])));
    assert!(garbled(&good.replace(sig, &format!("{}zz", &sig[..126]))));
}

#[test]
fn security_a_changed_claim_breaks_the_signature() {
    let good = sign(&FIXTURE, PURPOSE, day("2026-10-30"), 30, None);
    let later = good.replace("expires: 2026-11-29", "expires: 2027-10-29");
    assert_eq!(check(&later, "2026-10-30", None), Err(Refused::BadSignature));
    let machine = good.replace("expires: 2026-11-29\n", "expires: 2026-11-29\nmachine: any\n");
    assert_eq!(check(&machine, "2026-10-30", Some("any")), Err(Refused::BadSignature));
    // (a flipped bit in the signature)
    let sig = good.lines().find_map(|l| l.strip_prefix("signature: ")).unwrap();
    let flipped = format!("{}{}", if sig.starts_with('0') { "1" } else { "0" }, &sig[1..]);
    assert_eq!(check(&good.replace(sig, &flipped), "2026-10-30", None), Err(Refused::BadSignature));
}

#[test]
fn security_another_keys_tokens_are_refused() {
    let other = sign(&OTHER, PURPOSE, day("2026-10-30"), 30, None);
    assert_eq!(check(&other, "2026-10-30", None), Err(Refused::OtherKey(key_id(&public_of(&OTHER)))));
    // (signed by another key, claiming the fixture's id)
    let forged = other.replace(&key_id(&public_of(&OTHER)), &key_id(&fixture()));
    assert_eq!(check(&forged, "2026-10-30", None), Err(Refused::BadSignature));
}

#[test]
fn security_a_machine_hint_is_checked() {
    let t = sign(&FIXTURE, PURPOSE, day("2026-10-30"), 30, Some("lab-1"));
    assert!(check(&t, "2026-10-30", Some("LAB-1.local")).is_ok());
    assert_eq!(check(&t, "2026-10-30", Some("lab-2")), Err(Refused::OtherMachine("lab-1".into())));
    assert_eq!(check(&t, "2026-10-30", None), Err(Refused::OtherMachine("lab-1".into())));
}

#[test]
fn security_line_ends_spacing_and_comments_dont_matter() {
    let t = sign(&FIXTURE, PURPOSE, day("2026-10-30"), 30, None);
    let crlf = t.replace('\n', "\r\n");
    assert!(check(&crlf, "2026-10-30", None).is_ok());
    let spaced = format!("# a note\n\n{}", t.replace(": ", ":   "));
    assert!(check(&spaced, "2026-10-30", None).is_ok());
}

#[test]
fn security_the_decision() {
    let t = sign(&FIXTURE, PURPOSE, day("2026-10-30"), 30, None);
    let read = |_: &std::path::Path| Ok(t.clone());
    let path = Some(std::ffi::OsString::from("/x/test-mode.token"));
    let today = day("2026-10-30");
    // (a development build: on, token or not)
    assert_eq!(decide(true, None, None, today, None, read), Mode::Dev);
    // (a release build: off without a token, without a trusted key, with a
    // token it can't read; on with a good one)
    assert_eq!(decide(false, None, Some(fixture()), today, None, read), Mode::Off(Refused::NoToken));
    assert_eq!(decide(false, path.clone(), None, today, None, read), Mode::Off(Refused::NoTrustedKey));
    assert!(matches!(decide(false, path.clone(), Some(fixture()), today, None, |_| Err("gone".into())), Mode::Off(Refused::Unreadable(_))));
    assert_eq!(decide(false, path.clone(), Some(fixture()), today, None, read), Mode::Token { expires: "2026-11-29".into() });
    assert!(matches!(decide(false, path, Some(public_of(&OTHER)), today, None, read), Mode::Off(Refused::OtherKey(_))));
}

#[test]
fn security_no_build_trusts_the_fixture_key() {
    // (what release builds trust: test-mode.pub — the placeholder, or
    // Robert's key; never a key the tests make)
    for seed in [FIXTURE, OTHER] {
        assert_ne!(trusted_key(), Some(public_of(&seed)));
    }
    let t = sign(&FIXTURE, PURPOSE, today(), 30, None);
    let refused = match trusted_key() {
        None => decide(false, Some("/x".into()), trusted_key(), today(), None, |_| Ok(t.clone())) == Mode::Off(Refused::NoTrustedKey),
        Some(key) => matches!(verify(&t, &key, today(), None), Err(Refused::OtherKey(_))),
    };
    assert!(refused);
}
