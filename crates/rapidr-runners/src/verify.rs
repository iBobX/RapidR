//! Is this runner the one RapidR released? Bytes in, a yes or a reason out:
//! no file, no network, no clock — the same code runs in the desktop's
//! fetch and in the web Studio's (wasm), after the browser's fetch.
//!
//! The order matters and is the whole point:
//!
//! 1. [`check_list`]: the list's minisign signature against the key built
//!    into RapidR. Nothing in an unsigned list is read, not even its version.
//! 2. The list is for THIS RapidR (`version` equals the running one's): a
//!    2.123 Studio never uses 2.122's runners, nor 2.124's.
//! 3. [`check_file`]: the downloaded (or cached) file has the list's size and
//!    SHA-256 for its target.
//! 4. [`unpack_checked`]: the bundle is what the list says it is.
//!
//! [`check_all`] does all four.

use sha2::{Digest, Sha256};

use crate::bundle::{self, Bundle};
use crate::manifest::{Entry, Manifest};
use crate::minisign::{self, PublicKey};
use crate::names;

/// RapidR's release key (minisign): the key of `SHA256SUMS.minisig` on every
/// release's page. A runner list is signed with the same key.
pub const RELEASE_KEY: &str = "RWT4oRaArdWhjGXn5bIqoxGsqPvaLFJxHRpQqZXKUFDFuDZWgMPC3srG";

/// What a runner has to match: the key it is signed with and RapidR's version.
#[derive(Clone, Debug)]
pub struct Trust {
    pub key: PublicKey,
    /// RapidR's version (`2.123.1`).
    pub version: String,
}

impl Trust {
    /// The release key, for `version`.
    pub fn release(version: &str) -> Trust {
        Trust { key: PublicKey::parse(RELEASE_KEY).expect("the built-in release key is a minisign key"), version: version.to_string() }
    }
}

/// Why a runner was not used. [`Refusal`'s `Display`](std::fmt::Display) is
/// the sentence a person reads; match on the variant for anything else.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The list is not signed by RapidR's key (or its signature is broken).
    ListNotSigned { version: String, why: String },
    /// The list is signed, but it is not readable.
    ListUnreadable { version: String, why: String },
    /// The list is for another RapidR.
    WrongVersion { found: String, wanted: String },
    /// The list has no runner for this target.
    NotListed { version: String, target: String },
    /// The file is not the one the list describes (its size or its SHA-256).
    DoesNotMatch { version: String, target: String },
    /// The file matches the list but is not a good bundle.
    BadBundle { version: String, target: String, why: String },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::ListNotSigned { version, why } => write!(
                f,
                "The list of runners is not signed by RapidR ({why}), so RapidR {version} did not use anything from it."
            ),
            Refusal::ListUnreadable { version, why } => write!(f, "RapidR {version}'s list of runners can't be read ({why}), so nothing from it was used."),
            Refusal::WrongVersion { found, wanted } => write!(
                f,
                "These runners are for RapidR {found}, not {wanted}: they were not used. A runner always has to be the same version as the RapidR that builds with it."
            ),
            Refusal::NotListed { version, target } => write!(f, "RapidR {version}'s list has no runner for {}.", names::describe(target)),
            Refusal::DoesNotMatch { version, target } => write!(
                f,
                "This runner doesn't match RapidR {version}'s list: it was not used. ({} runner; it was damaged on the way or is not the one RapidR released.)",
                names::describe(target)
            ),
            Refusal::BadBundle { version, target, why } => write!(
                f,
                "The {} runner matches RapidR {version}'s list but can't be unpacked ({why}): it was not used.",
                names::describe(target)
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// Steps 1 and 2: `manifest_json` against `signature` (the `.minisig` text),
/// then parsed, and for `trust.version`.
pub fn check_list(trust: &Trust, manifest_json: &[u8], signature: &str) -> Result<Manifest, Refusal> {
    let version = trust.version.clone();
    trust.key.verify(manifest_json, signature).map_err(|e: minisign::Error| Refusal::ListNotSigned { version: version.clone(), why: e.to_string() })?;
    let manifest = Manifest::parse(manifest_json).map_err(|why| Refusal::ListUnreadable { version: version.clone(), why })?;
    if manifest.version != trust.version {
        return Err(Refusal::WrongVersion { found: manifest.version, wanted: trust.version.clone() });
    }
    Ok(manifest)
}

/// Step 3: `bytes` is `target`'s file in `manifest`: the size and the SHA-256.
/// Gives the list's entry for it.
pub fn check_file<'a>(manifest: &'a Manifest, target: &str, bytes: &[u8]) -> Result<&'a Entry, Refusal> {
    let entry = manifest.entry(target).ok_or_else(|| Refusal::NotListed { version: manifest.version.clone(), target: target.to_string() })?;
    let mismatch = || Refusal::DoesNotMatch { version: manifest.version.clone(), target: target.to_string() };
    if bytes.len() as u64 != entry.size {
        return Err(mismatch());
    }
    if sha256_hex(bytes) != entry.sha256 {
        return Err(mismatch());
    }
    Ok(entry)
}

/// Step 4: the checked file, unpacked.
pub fn unpack_checked(manifest: &Manifest, entry: &Entry, bytes: &[u8]) -> Result<Bundle, Refusal> {
    bundle::unpack(entry, bytes).map_err(|why| Refusal::BadBundle { version: manifest.version.clone(), target: entry.target.clone(), why })
}

/// All four steps: the list and its signature, `target`'s file, the bundle.
/// What the web Studio calls after its fetches (and the desktop for a
/// download).
pub fn check_all(trust: &Trust, manifest_json: &[u8], signature: &str, target: &str, file: &[u8]) -> Result<Bundle, Refusal> {
    let manifest = check_list(trust, manifest_json, signature)?;
    let entry = check_file(&manifest, target, file)?;
    unpack_checked(&manifest, entry, file)
}

/// The SHA-256 of `bytes`, as lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// A list and its files, signed with a made-up key, for the tests of
/// everything that gets runners (never a release key).
#[cfg(any(test, feature = "testing"))]
pub mod fixture {
    use super::*;

    /// One runner to put in a test list.
    pub struct Runner<'a> {
        pub target: &'a str,
        /// The files in its bundle.
        pub files: Vec<(&'a str, Vec<u8>)>,
    }

    /// A signed list and the `.zst` files it names.
    pub struct Release {
        pub key: PublicKey,
        pub trust: Trust,
        pub manifest: Vec<u8>,
        pub signature: String,
        /// `(file name, bytes)` as served next to the list.
        pub files: Vec<(String, Vec<u8>)>,
    }

    /// The files a bundle for `target` needs, with small made-up contents.
    pub fn files_for(target: &str) -> Vec<(&'static str, Vec<u8>)> {
        names::required_files(target).into_iter().map(|n| (n, format!("{target}:{n}:contents").into_bytes())).collect()
    }

    /// A list for `version` of these runners, signed (as `minisign -S` does,
    /// with a made-up key from `seed`), and the files.
    pub fn release(version: &str, runners: &[Runner<'_>], seed: u8) -> Release {
        let mut entries = Vec::new();
        let mut files = Vec::new();
        for r in runners {
            let borrowed: Vec<(&str, &[u8])> = r.files.iter().map(|(n, d)| (*n, d.as_slice())).collect();
            // (padded to whole records, as runners.py's tar is: a reader that wants the exact size fails here)
            let tar = bundle::tar_in_records_for_tests(&borrowed);
            let zst = ruzstd::encoding::compress_to_vec(tar.as_slice(), ruzstd::encoding::CompressionLevel::Fastest);
            let file = format!("{}.zst", r.target);
            let names: Vec<String> = r.files.iter().map(|(n, _)| n.to_string()).collect();
            let unpacked: usize = r.files.iter().map(|(_, d)| d.len()).sum();
            entries.push(format!(
                "{{\"target\":\"{}\",\"file\":\"{file}\",\"size\":{},\"sha256\":\"{}\",\"unpacked\":{unpacked},\"files\":[{}]}}",
                r.target,
                zst.len(),
                sha256_hex(&zst),
                names.iter().map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(",")
            ));
            files.push((file, zst));
        }
        let manifest = format!("{{\"format\":1,\"version\":\"{version}\",\"runners\":[{}]}}\n", entries.join(",")).into_bytes();
        let (key, signature) = minisign::sign_for_tests([seed; 32], &manifest, &format!("RapidR {version}"), true);
        Release { trust: Trust { key: key.clone(), version: version.to_string() }, key, manifest, signature, files }
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::*;
    use super::*;

    fn one(target: &'static str) -> Release {
        release("2.123.1", &[Runner { target, files: files_for(target) }], 1)
    }

    #[test]
    fn a_good_runner_is_used() {
        let r = one("windows-x86_64");
        let bundle = check_all(&r.trust, &r.manifest, &r.signature, "windows-x86_64", &r.files[0].1).unwrap();
        // (one Windows program: windowed or not, the packer sets the subsystem)
        assert_eq!(bundle.runner(true).unwrap(), b"windows-x86_64:rapidrintr-runner.exe:contents");
        assert_eq!(bundle.runner(false).unwrap(), bundle.runner(true).unwrap());
        assert!(bundle.notices().is_some());
    }

    #[test]
    fn a_changed_byte_is_refused_in_the_words_the_brief_asks_for() {
        let r = one("linux-x86_64");
        for at in [0, r.files[0].1.len() / 2, r.files[0].1.len() - 1] {
            let mut bad = r.files[0].1.clone();
            bad[at] ^= 1;
            let err = check_all(&r.trust, &r.manifest, &r.signature, "linux-x86_64", &bad).unwrap_err();
            assert!(matches!(err, Refusal::DoesNotMatch { .. }), "byte {at}: {err:?}");
            assert!(err.to_string().starts_with("This runner doesn't match RapidR 2.123.1's list: it was not used."), "{err}");
        }
    }

    #[test]
    fn a_file_of_the_right_size_with_other_contents_is_refused() {
        let r = one("linux-x86_64");
        let other = vec![0u8; r.files[0].1.len()];
        assert!(matches!(check_all(&r.trust, &r.manifest, &r.signature, "linux-x86_64", &other), Err(Refusal::DoesNotMatch { .. })));
        let shorter = &r.files[0].1[..r.files[0].1.len() - 1];
        assert!(matches!(check_all(&r.trust, &r.manifest, &r.signature, "linux-x86_64", shorter), Err(Refusal::DoesNotMatch { .. })));
    }

    #[test]
    fn a_list_for_another_version_is_refused() {
        let r = release("2.122.1", &[Runner { target: "web", files: files_for("web") }], 1);
        let mut trust = r.trust.clone();
        trust.version = "2.123.1".into();
        let err = check_all(&trust, &r.manifest, &r.signature, "web", &r.files[0].1).unwrap_err();
        assert_eq!(err, Refusal::WrongVersion { found: "2.122.1".into(), wanted: "2.123.1".into() });
        assert!(err.to_string().contains("for RapidR 2.122.1, not 2.123.1: they were not used"), "{err}");
    }

    #[test]
    fn a_list_signed_by_another_key_is_refused_before_it_is_read() {
        let r = one("web");
        let release_key = Trust::release("2.123.1");
        let err = check_list(&release_key, &r.manifest, &r.signature).unwrap_err();
        assert!(matches!(err, Refusal::ListNotSigned { .. }), "{err:?}");
        assert!(err.to_string().contains("not signed by RapidR"), "{err}");
        // (an unsigned list that is not even JSON: still "not signed", never "unreadable")
        let err = check_list(&release_key, b"{ nonsense", &r.signature).unwrap_err();
        assert!(matches!(err, Refusal::ListNotSigned { .. }));
    }

    #[test]
    fn a_changed_list_is_refused() {
        let r = one("web");
        let forged = String::from_utf8(r.manifest.clone()).unwrap().replace("\"size\":", "\"size\": 1");
        assert!(matches!(check_list(&r.trust, forged.as_bytes(), &r.signature), Err(Refusal::ListNotSigned { .. })));
    }

    #[test]
    fn a_target_the_list_lacks_is_said() {
        let r = one("web");
        let manifest = check_list(&r.trust, &r.manifest, &r.signature).unwrap();
        let err = check_file(&manifest, "linux-aarch64", &[1]).unwrap_err();
        assert_eq!(err.to_string(), "RapidR 2.123.1's list has no runner for Linux ARM64.");
    }

    #[test]
    fn a_bundle_that_is_not_what_the_list_says_is_refused() {
        // the list names rapidrintr-runner and notices; this tar has an extra file
        let mut files = files_for("linux-x86_64");
        let r = release("2.123.1", &[Runner { target: "linux-x86_64", files: files.clone() }], 1);
        files.push(("extra.bin", vec![1, 2, 3]));
        let borrowed: Vec<(&str, &[u8])> = files.iter().map(|(n, d)| (*n, d.as_slice())).collect();
        let zst = ruzstd::encoding::compress_to_vec(bundle::tar_for_tests(&borrowed).as_slice(), ruzstd::encoding::CompressionLevel::Fastest);
        // (a list that matches these bytes, but names only two files)
        let manifest = String::from_utf8(r.manifest.clone()).unwrap();
        let old_sha = r.files[0].1.as_slice();
        let manifest = manifest.replace(&sha256_hex(old_sha), &sha256_hex(&zst)).replace(&format!("\"size\":{}", old_sha.len()), &format!("\"size\":{}", zst.len()));
        let (key, signature) = minisign::sign_for_tests([2; 32], manifest.as_bytes(), "t", true);
        let trust = Trust { key, version: "2.123.1".into() };
        let err = check_all(&trust, manifest.as_bytes(), &signature, "linux-x86_64", &zst).unwrap_err();
        assert!(matches!(err, Refusal::BadBundle { .. }), "{err:?}");
        assert!(err.to_string().contains("can't be unpacked"), "{err}");
    }
}
