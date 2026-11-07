//! Signatures made by the real minisign program, checked by RapidR's code:
//! the release key's own signature of a published release's checksums, and
//! a throwaway key's signatures in both of minisign's modes.

use rapidr_runners::minisign::{Error, PublicKey};
use rapidr_runners::verify::RELEASE_KEY;

const RELEASE_SUMS: &[u8] = include_bytes!("fixtures/release-2.123.0.SHA256SUMS");
const RELEASE_SIG: &str = include_str!("fixtures/release-2.123.0.SHA256SUMS.minisig");

/// The signature Robert's `minisign -S` made over 2.123.0's SHA256SUMS,
/// verified with the key built into RapidR. This is the proof the pieces
/// (BLAKE2b, both Ed25519 checks, the file format) are minisign's.
#[test]
fn the_release_key_verifies_a_real_release_signature() {
    let key = PublicKey::parse(RELEASE_KEY).unwrap();
    let ok = key.verify(RELEASE_SUMS, RELEASE_SIG).expect("a real signature by the release key");
    assert!(ok.trusted_comment.contains("file:SHA256SUMS"), "{}", ok.trusted_comment);
}

#[test]
fn one_changed_byte_of_the_signed_file_fails() {
    let key = PublicKey::parse(RELEASE_KEY).unwrap();
    for at in [0, RELEASE_SUMS.len() / 2, RELEASE_SUMS.len() - 1] {
        let mut bad = RELEASE_SUMS.to_vec();
        bad[at] ^= 0x01;
        assert_eq!(key.verify(&bad, RELEASE_SIG), Err(Error::Mismatch), "byte {at}");
    }
}

#[test]
fn the_trusted_comment_is_signed_too() {
    let key = PublicKey::parse(RELEASE_KEY).unwrap();
    let forged = RELEASE_SIG.replace("file:SHA256SUMS", "file:SHA256SUMX");
    assert_eq!(key.verify(RELEASE_SUMS, &forged), Err(Error::Mismatch));
}

#[test]
fn a_signature_by_another_key_is_not_the_release_keys() {
    let other = PublicKey::parse(include_str!("fixtures/interop.pub")).unwrap();
    assert_eq!(other.verify(RELEASE_SUMS, RELEASE_SIG), Err(Error::OtherKey));
    let release = PublicKey::parse(RELEASE_KEY).unwrap();
    assert_eq!(release.verify(include_bytes!("fixtures/interop.txt"), include_str!("fixtures/interop.minisig")), Err(Error::OtherKey));
}

/// `minisign -S` (prehashed, "ED") and `minisign -S -l` (the legacy "Ed"),
/// from a key made for this test with `minisign -G -W`.
#[test]
fn both_of_minisigns_modes_verify() {
    let key = PublicKey::parse(include_str!("fixtures/interop.pub")).unwrap();
    let text = include_bytes!("fixtures/interop.txt");
    let prehashed = key.verify(text, include_str!("fixtures/interop.minisig")).unwrap();
    assert_eq!(prehashed.trusted_comment, "RapidR interop test");
    let legacy = key.verify(text, include_str!("fixtures/interop-legacy.minisig")).unwrap();
    assert_eq!(legacy.trusted_comment, "legacy test");
    // (the two modes are not interchangeable: a prehashed signature relabelled
    // "Ed" is read as a signature of the file itself, and is not one)
    let relabelled = include_str!("fixtures/interop.minisig").replace("RUSA7IEA", "RWSA7IEA");
    assert_eq!(key.verify(text, &relabelled), Err(Error::Mismatch));
}
