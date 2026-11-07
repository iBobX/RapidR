//! minisign signatures, checked: bytes and text in, a yes or a reason out.
//!
//! The format is minisign's own and public (jedisct1.github.io/minisign):
//!
//! ```text
//! untrusted comment: <anything>
//! <base64: "Ed" or "ED", the key's 8-byte id, a 64-byte Ed25519 signature>
//! trusted comment: <text>
//! <base64: a 64-byte Ed25519 signature of (the signature above || the trusted comment)>
//! ```
//!
//! "Ed" signs the file; "ED" (what `minisign -S` makes) signs the file's
//! BLAKE2b-512. The public key is base64 of "Ed", the key id and the
//! 32-byte Ed25519 key. Both signatures must hold: the second makes the
//! trusted comment (a time stamp, the file's name) as good as the file.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::blake2b;

/// A key that checks signatures: Ed25519 and the 8-byte id minisign puts in
/// both the key and its signatures, so a signature by another key is said so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKey {
    id: [u8; 8],
    key: [u8; 32],
}

/// Why a signature or a key was not accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The key text is not a minisign public key.
    BadKey(String),
    /// The signature text is not a minisign signature.
    BadSignature(String),
    /// The signature is by another key.
    OtherKey,
    /// The signature does not match the bytes (or its trusted comment).
    Mismatch,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BadKey(why) => write!(f, "not a minisign public key ({why})"),
            Error::BadSignature(why) => write!(f, "not a minisign signature ({why})"),
            Error::OtherKey => f.write_str("signed by another key"),
            Error::Mismatch => f.write_str("the signature does not match"),
        }
    }
}

impl std::error::Error for Error {}

/// What a good signature says besides "this is the file".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verified {
    /// The signed ("trusted") comment: minisign puts a time stamp and the
    /// file's name there; `-t` sets it (finish.sh: "RapidR <version>").
    pub trusted_comment: String,
}

impl PublicKey {
    /// From the base64 line of a `.pub` file or `minisign -V -P`
    /// (`RWT4oRaArdWh…`). The whole `.pub` file's text works too: its
    /// "untrusted comment:" line is skipped.
    pub fn parse(text: &str) -> Result<PublicKey, Error> {
        let line = text
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with("untrusted comment:"))
            .ok_or_else(|| Error::BadKey("empty".into()))?;
        let bytes = STANDARD.decode(line).map_err(|_| Error::BadKey("not base64".into()))?;
        if bytes.len() != 42 {
            return Err(Error::BadKey(format!("{} bytes, expected 42", bytes.len())));
        }
        if &bytes[..2] != b"Ed" {
            return Err(Error::BadKey("not an Ed25519 key".into()));
        }
        let mut id = [0u8; 8];
        id.copy_from_slice(&bytes[2..10]);
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes[10..42]);
        Ok(PublicKey { id, key })
    }

    /// The key's id as minisign prints it (hex, most significant byte first).
    pub fn id_hex(&self) -> String {
        self.id.iter().rev().map(|b| format!("{b:02X}")).collect()
    }

    /// Checks `data` against the signature in `signature_text` (a `.minisig`
    /// file's text): the key's id, the signature over the file (or its
    /// BLAKE2b), and the one over the trusted comment.
    pub fn verify(&self, data: &[u8], signature_text: &str) -> Result<Verified, Error> {
        let sig = Parsed::parse(signature_text)?;
        if sig.key_id != self.id {
            return Err(Error::OtherKey);
        }
        let key = ed25519_compact::PublicKey::new(self.key);
        let signature = ed25519_compact::Signature::new(sig.signature);
        let ok = if sig.prehashed { key.verify(blake2b::hash(data), &signature) } else { key.verify(data, &signature) };
        ok.map_err(|_| Error::Mismatch)?;
        // (the second signature: the first one's bytes and the trusted comment)
        let mut signed = Vec::with_capacity(64 + sig.trusted_comment.len());
        signed.extend_from_slice(&sig.signature);
        signed.extend_from_slice(sig.trusted_comment.as_bytes());
        key.verify(&signed, &ed25519_compact::Signature::new(sig.global_signature)).map_err(|_| Error::Mismatch)?;
        Ok(Verified { trusted_comment: sig.trusted_comment })
    }
}

struct Parsed {
    prehashed: bool,
    key_id: [u8; 8],
    signature: [u8; 64],
    trusted_comment: String,
    global_signature: [u8; 64],
}

impl Parsed {
    fn parse(text: &str) -> Result<Parsed, Error> {
        let bad = |why: &str| Error::BadSignature(why.to_string());
        let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
        let first = lines.next().ok_or_else(|| bad("empty"))?;
        if !first.starts_with("untrusted comment:") {
            return Err(bad("the first line is not an untrusted comment"));
        }
        let sig_line = lines.next().ok_or_else(|| bad("no signature line"))?;
        let bytes = STANDARD.decode(sig_line.trim()).map_err(|_| bad("the signature is not base64"))?;
        if bytes.len() != 74 {
            return Err(bad("the signature is not 74 bytes"));
        }
        let prehashed = match &bytes[..2] {
            b"ED" => true,
            b"Ed" => false,
            _ => return Err(bad("an algorithm other than Ed25519")),
        };
        let comment_line = lines.next().ok_or_else(|| bad("no trusted comment"))?;
        let trusted_comment = comment_line.strip_prefix("trusted comment: ").ok_or_else(|| bad("no trusted comment"))?.to_string();
        let global_line = lines.next().ok_or_else(|| bad("no signature of the trusted comment"))?;
        let global = STANDARD.decode(global_line.trim()).map_err(|_| bad("the comment's signature is not base64"))?;
        if global.len() != 64 {
            return Err(bad("the comment's signature is not 64 bytes"));
        }
        let mut key_id = [0u8; 8];
        key_id.copy_from_slice(&bytes[2..10]);
        let mut signature = [0u8; 64];
        signature.copy_from_slice(&bytes[10..74]);
        let mut global_signature = [0u8; 64];
        global_signature.copy_from_slice(&global);
        Ok(Parsed { prehashed, key_id, signature, trusted_comment, global_signature })
    }
}

/// Makes a minisign signature file's text with a key made from `seed` — for
/// the tests of everything that checks one (never a release key: those live
/// in minisign's own files, which RapidR's code does not read).
#[cfg(any(test, feature = "testing"))]
pub fn sign_for_tests(seed: [u8; 32], data: &[u8], trusted_comment: &str, prehashed: bool) -> (PublicKey, String) {
    let pair = ed25519_compact::KeyPair::from_seed(ed25519_compact::Seed::new(seed));
    let id = *b"RAPIDRTS";
    let mut pk = [0u8; 32];
    pk.copy_from_slice(pair.pk.as_ref());
    let key = PublicKey { id, key: pk };
    let sig = if prehashed { pair.sk.sign(blake2b::hash(data), None) } else { pair.sk.sign(data, None) };
    let mut signed = sig.as_ref().to_vec();
    signed.extend_from_slice(trusted_comment.as_bytes());
    let global = pair.sk.sign(&signed, None);
    let mut line = Vec::new();
    line.extend_from_slice(if prehashed { b"ED" } else { b"Ed" });
    line.extend_from_slice(&id);
    line.extend_from_slice(sig.as_ref());
    let text = format!(
        "untrusted comment: signature from a test key\n{}\ntrusted comment: {}\n{}\n",
        STANDARD.encode(line),
        trusted_comment,
        STANDARD.encode(global.as_ref())
    );
    (key, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASE_KEY: &str = "RWT4oRaArdWhjGXn5bIqoxGsqPvaLFJxHRpQqZXKUFDFuDZWgMPC3srG";

    #[test]
    fn the_release_key_parses_and_has_minisigns_id() {
        let key = PublicKey::parse(RELEASE_KEY).unwrap();
        // (what `minisign -V -p` prints for RapidR's key: the id of the signatures)
        assert_eq!(key.id_hex().len(), 16);
        // a .pub file's text is accepted too
        let file = format!("untrusted comment: minisign public key: {}\n{RELEASE_KEY}\n", key.id_hex());
        assert_eq!(PublicKey::parse(&file).unwrap(), key);
    }

    #[test]
    fn bad_keys_are_said() {
        assert!(matches!(PublicKey::parse(""), Err(Error::BadKey(_))));
        assert!(matches!(PublicKey::parse("not base64 !!"), Err(Error::BadKey(_))));
        assert!(matches!(PublicKey::parse("RWQ="), Err(Error::BadKey(_))));
    }

    #[test]
    fn both_algorithms_verify_and_say_the_trusted_comment() {
        for prehashed in [false, true] {
            let (key, sig) = sign_for_tests([7; 32], b"the manifest", "RapidR 2.123.1", prehashed);
            let ok = key.verify(b"the manifest", &sig).unwrap();
            assert_eq!(ok.trusted_comment, "RapidR 2.123.1");
            assert_eq!(key.verify(b"the manifesT", &sig), Err(Error::Mismatch), "prehashed={prehashed}");
        }
    }

    #[test]
    fn a_changed_trusted_comment_is_refused() {
        let (key, sig) = sign_for_tests([7; 32], b"data", "RapidR 2.123.1", true);
        let forged = sig.replace("trusted comment: RapidR 2.123.1", "trusted comment: RapidR 2.200.0");
        assert_eq!(key.verify(b"data", &forged), Err(Error::Mismatch));
    }

    #[test]
    fn another_key_is_said_not_just_mismatch() {
        let (_, sig) = sign_for_tests([7; 32], b"data", "c", true);
        let release = PublicKey::parse(RELEASE_KEY).unwrap();
        assert_eq!(release.verify(b"data", &sig), Err(Error::OtherKey));
    }

    #[test]
    fn same_id_other_key_is_a_mismatch() {
        // (an attacker copies the id: the Ed25519 check still stops it)
        let (key, sig) = sign_for_tests([7; 32], b"data", "c", true);
        let (other, _) = sign_for_tests([8; 32], b"data", "c", true);
        assert_eq!(other.verify(b"data", &sig), Err(Error::Mismatch));
        assert!(key.verify(b"data", &sig).is_ok());
    }

    #[test]
    fn broken_signature_text_is_said() {
        let (key, sig) = sign_for_tests([7; 32], b"data", "c", true);
        assert!(matches!(key.verify(b"data", ""), Err(Error::BadSignature(_))));
        assert!(matches!(key.verify(b"data", "hello"), Err(Error::BadSignature(_))));
        let no_comment: String = sig.lines().take(2).collect::<Vec<_>>().join("\n");
        assert!(matches!(key.verify(b"data", &no_comment), Err(Error::BadSignature(_))));
        // CRLF line ends (a file copied through Windows) still verify
        assert!(key.verify(b"data", &sig.replace('\n', "\r\n")).is_ok());
    }
}
