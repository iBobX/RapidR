//! Ad-hoc code signatures for Mach-O programs and macOS apps, in pure Rust
//! (docs/macho-signer-design.md).
//!
//! An Apple-silicon Mac runs no arm64 program without a valid signature. An
//! ad-hoc one carries no certificate: it is a CodeDirectory (the SHA-256 of
//! every page of the program, and of the app's Info.plist and seal), an
//! empty requirements set and an empty signature slot, flagged ad hoc. This
//! module makes it from bytes, with no process and no file, so a Mac app is
//! signed the same on macOS, Windows, Linux and in the web Studio (wasm32).
//!
//! The output is what `codesign --force --sign -` writes on macOS 27.2, byte
//! for byte (the golden tests compare). Written from public format
//! descriptions only: Apple's SDK headers (`kern/cs_blobs.h`,
//! `mach-o/loader.h`, `mach-o/fat.h`), Apple's technical notes on code
//! signing, and `codesign`'s observed output (the design page lists them).
//!
//! - [`sign`]: a thin (arm64, x86_64) or universal Mach-O;
//! - [`sign_app`]: a whole `.app` in memory: the main executable signed with
//!   its Info.plist and `_CodeSignature/CodeResources` bound;
//! - [`check`]: reads a signature back and re-hashes the code.

use sha2::{Digest, Sha256};

// mach/machine.h: the two CPU types macOS runs (arm64e is a subtype of arm64)
use crate::macho::{CPU_TYPE_ARM64, CPU_TYPE_X86_64};

// mach-o/loader.h
const MH_MAGIC: u32 = 0xfeed_face;
const MH_CIGAM: u32 = 0xcefa_edfe;
const MH_MAGIC_64: u32 = 0xfeed_facf;
const MH_CIGAM_64: u32 = 0xcffa_edfe;
const MH_EXECUTE: u32 = 0x2;
const LC_SEGMENT_64: u32 = 0x19;
const LC_UUID: u32 = 0x1b;
const LC_CODE_SIGNATURE: u32 = 0x1d;
const SECTION_TYPE: u32 = 0xff;
const S_ZEROFILL: u32 = 0x1;
const S_GB_ZEROFILL: u32 = 0xc;
const S_THREAD_LOCAL_ZEROFILL: u32 = 0x12;
const HEADER_64: usize = 32;
const SEGMENT_64: usize = 72;
const SECTION_64: usize = 80;
const LINKEDIT_DATA: usize = 16;

// mach-o/fat.h
const FAT_MAGIC: u32 = 0xcafe_babe;
const FAT_MAGIC_64: u32 = 0xcafe_babf;

// kern/cs_blobs.h (big-endian on disk)
const CSMAGIC_REQUIREMENTS: u32 = 0xfade_0c01;
const CSMAGIC_CODEDIRECTORY: u32 = 0xfade_0c02;
const CSMAGIC_EMBEDDED_SIGNATURE: u32 = 0xfade_0cc0;
const CSMAGIC_BLOBWRAPPER: u32 = 0xfade_0b01;
const CSSLOT_CODEDIRECTORY: u32 = 0;
const CSSLOT_INFOSLOT: usize = 1;
const CSSLOT_REQUIREMENTS: u32 = 2;
const CSSLOT_RESOURCEDIR: usize = 3;
const CSSLOT_SIGNATURESLOT: u32 = 0x1_0000;
const CS_SUPPORTSEXECSEG: u32 = 0x2_0400;
/// The CodeDirectory flag of an ad-hoc signature.
pub const CS_ADHOC: u32 = 0x2;
/// The flag of a signature the linker made (dropped when we sign).
pub const CS_LINKER_SIGNED: u32 = 0x2_0000;
const CS_HASHTYPE_SHA256: u8 = 2;
const CS_EXECSEG_MAIN_BINARY: u64 = 0x1;
/// A version-0x20400 CodeDirectory's fixed part.
const CD_HEADER: usize = 88;
const HASH_SIZE: usize = 32;
/// The empty requirements set (a Requirements blob: magic, length 12, no entries).
const EMPTY_REQUIREMENTS: [u8; 12] = {
    let m = CSMAGIC_REQUIREMENTS.to_be_bytes();
    [m[0], m[1], m[2], m[3], 0, 0, 0, 12, 0, 0, 0, 0]
};
/// The empty signature slot (a BlobWrapper of length 8, no CMS in it): ad hoc.
const EMPTY_WRAPPER: [u8; 8] = {
    let m = CSMAGIC_BLOBWRAPPER.to_be_bytes();
    [m[0], m[1], m[2], m[3], 0, 0, 0, 8]
};
/// SuperBlob header + its three index entries.
const SUPERBLOB_HEAD: usize = 12 + 3 * 8;
/// __LINKEDIT's vmsize is its filesize rounded up to this (codesign's way, on both CPUs).
const LINKEDIT_VM_ROUND: u64 = 0x4000;
/// A universal file's slices start on (at least) 2^14-byte boundaries.
const FAT_MIN_ALIGN: u32 = 14;

/// What the signature binds besides the code: an app's Info.plist (special
/// slot 1) and its `_CodeSignature/CodeResources` (special slot 3). Both
/// `None` for a program on its own.
#[derive(Debug, Default, Clone, Copy)]
pub struct Bound<'a> {
    pub info_plist: Option<&'a [u8]>,
    pub code_resources: Option<&'a [u8]>,
}

/// Signs a Mach-O program ad hoc: a thin arm64 or x86_64 one, or a universal
/// one (each slice signed). An old signature (the linker's, or anyone's) is
/// replaced. `identifier`: an app's CFBundleIdentifier, or for a program on
/// its own any name ([`default_identifier`] gives codesign's).
pub fn sign(macho: &[u8], identifier: &str, bound: Bound) -> Result<Vec<u8>, String> {
    if identifier.is_empty() || identifier.contains('\0') {
        return Err(format!("the signature's identifier {identifier:?} must be a non-empty name without NUL"));
    }
    match be32(macho, 0) {
        Some(FAT_MAGIC) | Some(FAT_MAGIC_64) => sign_fat(macho, identifier, &bound),
        _ => sign_thin(macho, identifier, &bound),
    }
}

/// The identifier `codesign` gives a program that isn't in an app: its file
/// name without the extension, then `-` and the hex of `"UUID"` followed by
/// its LC_UUID (in a universal file, the arm64 slice's: what `codesign`
/// picks on Apple silicon). Just the name when it has no UUID (`codesign`
/// adds a hash of its own then; every linked program has a UUID).
pub fn default_identifier(file_name: &str, macho: &[u8]) -> String {
    let base = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    let stem = match base.rfind('.') {
        Some(dot) if dot > 0 => &base[..dot],
        _ => base,
    };
    let uuid = slices(macho).ok().and_then(|s| {
        let uuids: Vec<(u32, [u8; 16])> = s.iter().filter_map(|&(start, len)| parse_thin(&macho[start..start + len]).ok().and_then(|t| Some((t.cputype, t.uuid?)))).collect();
        uuids.iter().find(|(cpu, _)| *cpu == CPU_TYPE_ARM64).or(uuids.first()).map(|(_, u)| *u)
    });
    match uuid {
        Some(u) => format!("{stem}-55554944{}", hex(&u)),
        None => stem.to_string(),
    }
}

/// One slice's signature, read back by [`check`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed {
    pub cpu_type: u32,
    pub identifier: String,
    /// CodeDirectory flags ([`CS_ADHOC`], ...).
    pub flags: u32,
    pub page_size: usize,
    /// The signed range: bytes `0..code_limit` of the slice.
    pub code_limit: usize,
    /// Special slots 1, 2, 3 ...: Info.plist, requirements, CodeResources.
    pub special: Vec<[u8; 32]>,
    /// The page hashes.
    pub pages: Vec<[u8; 32]>,
    /// SHA-256 of the CodeDirectory (`codesign -dv`'s CDHash is its first 20 bytes).
    pub cd_hash: [u8; 32],
}

/// Reads each slice's signature and checks every page hash against the code
/// (what the kernel checks as it loads the program). An error names the
/// first thing wrong: not signed, not SHA-256, a page that changed.
pub fn check(macho: &[u8]) -> Result<Vec<Signed>, String> {
    let mut out = Vec::new();
    for (start, len) in slices(macho)? {
        let b = &macho[start..start + len];
        let t = parse_thin(b)?;
        let name = cpu_name(t.cputype);
        let (_, dataoff, datasize) = t.signature.ok_or_else(|| format!("the {name} program isn't signed"))?;
        let (dataoff, datasize) = (dataoff as usize, datasize as usize);
        let sig = &b[dataoff..dataoff + datasize];
        let bad = |what: &str| format!("the {name} signature is broken: {what}");
        if be32(sig, 0) != Some(CSMAGIC_EMBEDDED_SIGNATURE) {
            return Err(bad("no SuperBlob where LC_CODE_SIGNATURE points"));
        }
        let count = be32(sig, 8).ok_or_else(|| bad("cut short"))? as usize;
        let mut cd = None;
        for i in 0..count.min(64) {
            let (Some(kind), Some(off)) = (be32(sig, 12 + 8 * i), be32(sig, 16 + 8 * i)) else { return Err(bad("index cut short")) };
            if kind == CSSLOT_CODEDIRECTORY {
                let off = off as usize;
                let blob = off.checked_add(4).and_then(|at| be32(sig, at)).and_then(|len| sig.get(off..off.checked_add(len as usize)?));
                cd = Some(blob.ok_or_else(|| bad("CodeDirectory outside the signature"))?);
            }
        }
        let cd = cd.ok_or_else(|| bad("no CodeDirectory"))?;
        let field = |at: usize| be32(cd, at).ok_or_else(|| bad("CodeDirectory cut short"));
        if field(0)? != CSMAGIC_CODEDIRECTORY {
            return Err(bad("the CodeDirectory's magic is wrong"));
        }
        let flags = field(12)?;
        let hash_offset = field(16)? as usize;
        let ident_offset = field(20)? as usize;
        let n_special = field(24)? as usize;
        let n_code = field(28)? as usize;
        let code_limit = field(32)? as usize;
        let (hash_size, hash_type, page_shift) = (cd.get(36).copied(), cd.get(37).copied(), cd.get(39).copied());
        if hash_size != Some(HASH_SIZE as u8) || hash_type != Some(CS_HASHTYPE_SHA256) {
            return Err(bad("its hashes aren't SHA-256"));
        }
        let page_shift = page_shift.unwrap_or(0);
        if !(9..=16).contains(&page_shift) {
            return Err(bad("its page size isn't one RapidR checks"));
        }
        let page = 1usize << page_shift;
        if code_limit > dataoff || n_code != code_limit.div_ceil(page) {
            return Err(bad("its code range doesn't fit the program"));
        }
        let ident_end = cd.get(ident_offset..).and_then(|s| s.iter().position(|&c| c == 0)).ok_or_else(|| bad("identifier cut short"))?;
        let identifier = String::from_utf8_lossy(&cd[ident_offset..ident_offset + ident_end]).into_owned();
        let hash_at = |slot: i64| -> Result<[u8; 32], String> {
            let at = usize::try_from(hash_offset as i64 + slot * HASH_SIZE as i64).ok();
            at.and_then(|at| cd.get(at..at.checked_add(HASH_SIZE)?)).map(|h| h.try_into().unwrap()).ok_or_else(|| bad("hash slots outside the CodeDirectory"))
        };
        if n_special > 64 {
            return Err(bad("more special slots than there are kinds"));
        }
        let special = (1..=n_special as i64).map(|i| hash_at(-i)).collect::<Result<Vec<_>, _>>()?;
        let pages = (0..n_code as i64).map(hash_at).collect::<Result<Vec<_>, _>>()?;
        for (i, (chunk, want)) in b[..code_limit].chunks(page).zip(&pages).enumerate() {
            if Sha256::digest(chunk).as_slice() != want {
                return Err(format!("the {name} program changed after it was signed (page {i} of {n_code} doesn't match its hash)"));
            }
        }
        out.push(Signed { cpu_type: t.cputype, identifier, flags, page_size: page, code_limit, special, pages, cd_hash: Sha256::digest(cd).into() });
    }
    Ok(out)
}

/// Signs a whole `.app` in memory. `files`: every file of the app, by its
/// path inside it (`Contents/Info.plist`, `Contents/MacOS/hello`,
/// `Contents/Resources/AppIcon.icns`, ...). The main executable (Info.plist's
/// CFBundleExecutable) is replaced by its signed self, and
/// `Contents/_CodeSignature/CodeResources` (the seal of every file in
/// Contents/Resources) is added; an old seal is replaced. The identifier is
/// Info.plist's CFBundleIdentifier. Gives the program's path.
///
/// Refused (nothing changes): a file outside Contents/Resources besides
/// Info.plist, PkgInfo and the program (it would be nested code, which
/// needs a signature of its own first), and a path that isn't a plain one
/// inside the app.
pub fn sign_app(files: &mut Vec<(String, Vec<u8>)>) -> Result<String, String> {
    for (path, _) in files.iter() {
        let plain = !path.is_empty()
            && !path.starts_with('/')
            && !path.contains('\\')
            && !path.contains('\0')
            && path.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
        if !plain {
            return Err(format!("{path:?} isn't a path inside the app (no `..`, no leading `/`, `/` between folders)"));
        }
    }
    let info = files.iter().find(|(p, _)| p == "Contents/Info.plist").map(|(_, b)| b.as_slice()).ok_or("the app has no Contents/Info.plist")?;
    let executable = plist_string(info, "CFBundleExecutable")?;
    let identifier = plist_string(info, "CFBundleIdentifier")?;
    if executable.is_empty() || executable.contains('/') {
        return Err(format!("Info.plist's CFBundleExecutable {executable:?} isn't a file name"));
    }
    let main_path = format!("Contents/MacOS/{executable}");
    let mut resources: Vec<(&str, &[u8])> = Vec::new();
    let mut main = None;
    for (i, (path, bytes)) in files.iter().enumerate() {
        if *path == main_path {
            main = Some(i);
        } else if path.starts_with("Contents/_CodeSignature/") || path == "Contents/Info.plist" || path == "Contents/PkgInfo" {
            // (the old seal: made again; the other two: bound or left out by the rules)
        } else if path.starts_with("Contents/Resources/") {
            resources.push((&path["Contents/".len()..], bytes));
        } else {
            return Err(format!(
                "{path}: an app RapidR signs keeps its files in Contents/Resources (besides Info.plist, PkgInfo and {main_path}); \
                 other code would need a signature of its own first"
            ));
        }
    }
    let main = main.ok_or_else(|| format!("the app's program {main_path} isn't there (Info.plist's CFBundleExecutable)"))?;
    let seal = code_resources(&resources);
    let signed = sign(&files[main].1, &identifier, Bound { info_plist: Some(info), code_resources: Some(&seal) }).map_err(|e| format!("{main_path}: {e}"))?;
    files[main].1 = signed;
    files.retain(|(p, _)| !p.starts_with("Contents/_CodeSignature/"));
    files.push(("Contents/_CodeSignature/CodeResources".to_string(), seal));
    Ok(main_path)
}

/// `_CodeSignature/CodeResources` for an app's resources: `resources` are
/// (path inside Contents, contents), each path starting `Resources/`. The
/// text is the XML plist `codesign` writes: each file's SHA-1 (`files`) and
/// SHA-256 (`files2`), `.lproj` files optional, then Apple's default rules.
pub fn code_resources(resources: &[(&str, &[u8])]) -> Vec<u8> {
    struct Entry<'a> {
        path: &'a str,
        bytes: &'a [u8],
        optional: bool,
    }
    let mut entries: Vec<Entry> = resources
        .iter()
        .filter_map(|&(path, bytes)| {
            let name = path.rsplit('/').next().unwrap_or(path);
            // rules2's `^(.*/)?\.DS_Store$` (omit, 2000) and `^Resources/.*\.lproj/locversion.plist$` (omit, 1100)
            if name == ".DS_Store" || locversion(path) {
                return None;
            }
            // `^Resources/Base\.lproj/` (1010) outweighs `^Resources/.*\.lproj/` (1000, optional)
            let optional = lproj(path) && !path.starts_with("Resources/Base.lproj/");
            Some(Entry { path, bytes, optional })
        })
        .collect();
    entries.sort_by(|a, b| a.path.encode_utf16().cmp(b.path.encode_utf16()));

    let mut x = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n",
    );
    for (key, version) in [("files", 1), ("files2", 2)] {
        plist_key(&mut x, 1, key);
        if entries.is_empty() {
            x.push_str("\t<dict/>\n");
            continue;
        }
        x.push_str("\t<dict>\n");
        for e in &entries {
            plist_key(&mut x, 2, e.path);
            let (sha1_hash, sha256_hash);
            let hash: &[u8] = if version == 1 {
                sha1_hash = sha1(e.bytes);
                &sha1_hash
            } else {
                sha256_hash = Sha256::digest(e.bytes);
                &sha256_hash
            };
            if version == 1 && !e.optional {
                plist_data(&mut x, 2, hash);
            } else {
                x.push_str("\t\t<dict>\n");
                plist_key(&mut x, 3, if version == 1 { "hash" } else { "hash2" });
                plist_data(&mut x, 3, hash);
                if e.optional {
                    plist_key(&mut x, 3, "optional");
                    x.push_str("\t\t\t<true/>\n");
                }
                x.push_str("\t\t</dict>\n");
            }
        }
        x.push_str("\t</dict>\n");
    }
    for (key, rules) in [("rules", RULES), ("rules2", RULES2)] {
        plist_key(&mut x, 1, key);
        x.push_str("\t<dict>\n");
        for (pattern, attrs) in rules {
            plist_key(&mut x, 2, pattern);
            if attrs.is_empty() {
                x.push_str("\t\t<true/>\n");
                continue;
            }
            x.push_str("\t\t<dict>\n");
            for attr in *attrs {
                match attr {
                    Rule::Nested => plist_flag(&mut x, "nested"),
                    Rule::Omit => plist_flag(&mut x, "omit"),
                    Rule::Optional => plist_flag(&mut x, "optional"),
                    Rule::Weight(w) => {
                        plist_key(&mut x, 3, "weight");
                        x.push_str(&format!("\t\t\t<real>{w}</real>\n"));
                    }
                }
            }
            x.push_str("\t\t</dict>\n");
        }
        x.push_str("\t</dict>\n");
    }
    x.push_str("</dict>\n</plist>\n");
    x.into_bytes()
}

/// `^Resources/.*\.lproj/`: a file in a localization folder.
fn lproj(path: &str) -> bool {
    path.strip_prefix("Resources/").is_some_and(|rest| rest.contains(".lproj/"))
}

/// `^Resources/.*\.lproj/locversion.plist$` (its `.` unescaped: any character).
fn locversion(path: &str) -> bool {
    let Some((dir, name)) = path.strip_prefix("Resources/").and_then(|rest| rest.rsplit_once('/')) else { return false };
    dir.ends_with(".lproj") && name.chars().count() == 16 && name.starts_with("locversion") && name.ends_with("plist")
}

/// A resource rule's attributes (in the order the plist sorts them).
enum Rule {
    Nested,
    Omit,
    Optional,
    Weight(u32),
}

/// Apple's default resource rules (TN2206), version 1: each pattern, and
/// `<true/>` when it has no attributes.
const RULES: &[(&str, &[Rule])] = &[
    ("^Resources/", &[]),
    ("^Resources/.*\\.lproj/", &[Rule::Optional, Rule::Weight(1000)]),
    ("^Resources/.*\\.lproj/locversion.plist$", &[Rule::Omit, Rule::Weight(1100)]),
    ("^Resources/Base\\.lproj/", &[Rule::Weight(1010)]),
    ("^version.plist$", &[]),
];

/// ... and version 2 (what macOS checks).
const RULES2: &[(&str, &[Rule])] = &[
    (".*\\.dSYM($|/)", &[Rule::Weight(11)]),
    ("^(.*/)?\\.DS_Store$", &[Rule::Omit, Rule::Weight(2000)]),
    ("^(Frameworks|SharedFrameworks|PlugIns|Plug-ins|XPCServices|Helpers|MacOS|Library/(Automator|Spotlight|LoginItems))/", &[Rule::Nested, Rule::Weight(10)]),
    ("^.*", &[]),
    ("^Info\\.plist$", &[Rule::Omit, Rule::Weight(20)]),
    ("^PkgInfo$", &[Rule::Omit, Rule::Weight(20)]),
    ("^Resources/", &[Rule::Weight(20)]),
    ("^Resources/.*\\.lproj/", &[Rule::Optional, Rule::Weight(1000)]),
    ("^Resources/.*\\.lproj/locversion.plist$", &[Rule::Omit, Rule::Weight(1100)]),
    ("^Resources/Base\\.lproj/", &[Rule::Weight(1010)]),
    ("^[^/]+$", &[Rule::Nested, Rule::Weight(10)]),
    ("^embedded\\.provisionprofile$", &[Rule::Weight(20)]),
    ("^version\\.plist$", &[Rule::Weight(20)]),
];

fn plist_key(x: &mut String, depth: usize, key: &str) {
    x.extend(std::iter::repeat_n('\t', depth));
    x.push_str("<key>");
    for c in key.chars() {
        match c {
            '&' => x.push_str("&amp;"),
            '<' => x.push_str("&lt;"),
            '>' => x.push_str("&gt;"),
            c => x.push(c),
        }
    }
    x.push_str("</key>\n");
}

fn plist_data(x: &mut String, depth: usize, bytes: &[u8]) {
    let tabs: String = std::iter::repeat_n('\t', depth).collect();
    x.push_str(&format!("{tabs}<data>\n{tabs}{}\n{tabs}</data>\n", base64(bytes)));
}

fn plist_flag(x: &mut String, key: &str) {
    plist_key(x, 3, key);
    x.push_str("\t\t\t<true/>\n");
}

/// A string value of an XML Info.plist (`<key>name</key> <string>...</string>`).
fn plist_string(plist: &[u8], name: &str) -> Result<String, String> {
    if plist.starts_with(b"bplist") {
        return Err("Info.plist is a binary plist: RapidR signs apps whose Info.plist is XML".into());
    }
    let text = std::str::from_utf8(plist).map_err(|_| "Info.plist isn't UTF-8 text".to_string())?;
    let missing = || format!("Info.plist has no {name} (a <key>{name}</key> with a <string>)");
    let at = text.find(&format!("<key>{name}</key>")).ok_or_else(missing)? + name.len() + 11;
    let rest = text[at..].trim_start();
    if rest.starts_with("<string/>") {
        return Ok(String::new());
    }
    let value = rest.strip_prefix("<string>").and_then(|r| r.find("</string>").map(|end| &r[..end])).ok_or_else(missing)?;
    unescape_xml(value).ok_or_else(|| format!("Info.plist's {name} has a broken `&...;` in it"))
}

fn unescape_xml(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let end = rest[amp..].find(';')? + amp;
        let entity = &rest[amp + 1..end];
        let c = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let n = if let Some(h) = entity.strip_prefix("#x").or_else(|| entity.strip_prefix("#X")) {
                    u32::from_str_radix(h, 16).ok()?
                } else {
                    entity.strip_prefix('#')?.parse().ok()?
                };
                char::from_u32(n)?
            }
        };
        out.push(c);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

// ---------------------------------------------------------------- Mach-O

/// What signing needs to know about a thin 64-bit Mach-O.
struct Thin {
    cputype: u32,
    filetype: u32,
    ncmds: u32,
    sizeofcmds: u32,
    /// __LINKEDIT: its load command's offset, file offset and size.
    linkedit: (usize, u64, u64),
    /// __TEXT's file offset and size (the executable segment).
    text: (u64, u64),
    /// LC_CODE_SIGNATURE: its load command's offset, dataoff, datasize.
    signature: Option<(usize, u32, u32)>,
    uuid: Option<[u8; 16]>,
    /// Where the first section's (or segment's) data starts after the header.
    first_data: u64,
}

fn parse_thin(b: &[u8]) -> Result<Thin, String> {
    let magic = le32(b, 0).ok_or_else(|| format!("too short to be a Mach-O program ({} bytes)", b.len()))?;
    match magic {
        MH_MAGIC_64 => {}
        MH_MAGIC | MH_CIGAM => return Err("a 32-bit Mach-O: macOS runs 64-bit programs only (arm64, x86_64)".into()),
        MH_CIGAM_64 => return Err("a big-endian Mach-O: only arm64 and x86_64 programs are signed".into()),
        _ => return Err(not_macho(b)),
    }
    if b.len() < HEADER_64 {
        return Err(format!("the Mach-O header is cut short ({} bytes of {HEADER_64})", b.len()));
    }
    let cputype = le32(b, 4).unwrap();
    if cputype != CPU_TYPE_ARM64 && cputype != CPU_TYPE_X86_64 {
        return Err(format!("CPU type {cputype:#x} isn't one macOS runs (arm64, x86_64)"));
    }
    let filetype = le32(b, 12).unwrap();
    let ncmds = le32(b, 16).unwrap();
    let sizeofcmds = le32(b, 20).unwrap();
    let cmds_end = HEADER_64 as u64 + u64::from(sizeofcmds);
    if cmds_end > b.len() as u64 {
        return Err(format!("the Mach-O is cut short: its load commands end at byte {cmds_end}, the file at {}", b.len()));
    }
    let cmds_end = cmds_end as usize;
    let mut linkedit = None;
    let mut text = None;
    let mut signature = None;
    let mut uuid = None;
    let mut first_data = u64::MAX;
    let mut last_end = 0u64;
    let mut at = HEADER_64;
    for i in 0..ncmds {
        let broken = |what: &str| format!("load command {i} (at byte {at}) is broken: {what}");
        let (Some(cmd), Some(size)) = (le32(b, at), le32(b, at + 4)) else { return Err(broken("it runs past the load commands' end")) };
        let size = size as usize;
        if size < 8 || at.checked_add(size).is_none_or(|end| end > cmds_end) {
            return Err(broken(&format!("its size {size} doesn't fit")));
        }
        match cmd {
            LC_SEGMENT_64 => {
                if size < SEGMENT_64 {
                    return Err(broken("a segment command shorter than 72 bytes"));
                }
                let name = cstr(&b[at + 8..at + 24]);
                let (fileoff, filesize) = (le64(b, at + 40).unwrap(), le64(b, at + 48).unwrap());
                let nsects = le32(b, at + 64).unwrap() as usize;
                if nsects.checked_mul(SECTION_64).and_then(|n| n.checked_add(SEGMENT_64)).is_none_or(|n| n > size) {
                    return Err(broken(&format!("segment {name} says {nsects} sections, more than its command holds")));
                }
                let end = fileoff.checked_add(filesize).filter(|&e| e <= b.len() as u64).ok_or_else(|| {
                    format!("segment {name} runs past the end of the file (it ends at byte {}, the file at {}): cut short?", fileoff.saturating_add(filesize), b.len())
                })?;
                if filesize > 0 {
                    last_end = last_end.max(end);
                    if fileoff > 0 {
                        first_data = first_data.min(fileoff);
                    }
                }
                for s in 0..nsects {
                    let sect = at + SEGMENT_64 + s * SECTION_64;
                    let (size, offset, flags) = (le64(b, sect + 40).unwrap(), le32(b, sect + 48).unwrap(), le32(b, sect + 64).unwrap());
                    let zerofill = matches!(flags & SECTION_TYPE, S_ZEROFILL | S_GB_ZEROFILL | S_THREAD_LOCAL_ZEROFILL);
                    if !zerofill && size > 0 && offset > 0 {
                        first_data = first_data.min(u64::from(offset));
                    }
                }
                match name.as_str() {
                    "__LINKEDIT" if linkedit.is_some() => return Err("two __LINKEDIT segments".into()),
                    "__LINKEDIT" => linkedit = Some((at, fileoff, filesize)),
                    "__TEXT" if text.is_none() => text = Some((fileoff, filesize)),
                    _ => {}
                }
            }
            LC_CODE_SIGNATURE => {
                if size != LINKEDIT_DATA {
                    return Err(broken("LC_CODE_SIGNATURE isn't 16 bytes"));
                }
                if signature.is_some() {
                    return Err("two LC_CODE_SIGNATURE load commands".into());
                }
                let (off, len) = (le32(b, at + 8).unwrap(), le32(b, at + 12).unwrap());
                if u64::from(off) + u64::from(len) > b.len() as u64 {
                    return Err(format!("the old signature runs past the end of the file (bytes {off}..{}, the file has {}): cut short?", u64::from(off) + u64::from(len), b.len()));
                }
                signature = Some((at, off, len));
            }
            LC_UUID if size >= 24 => uuid = Some(b[at + 8..at + 24].try_into().unwrap()),
            _ => {}
        }
        at += size;
    }
    let linkedit = linkedit.ok_or("no __LINKEDIT segment (an object file, not a linked program?)")?;
    let linkedit_end = linkedit.1 + linkedit.2;
    if last_end > linkedit_end {
        return Err("__LINKEDIT isn't the last segment in the file: a signature must come after everything else".into());
    }
    if (b.len() as u64) > linkedit_end {
        return Err(format!(
            "{} bytes after the end of __LINKEDIT: a signed program can carry nothing after its code (put data in the app's Contents/Resources)",
            b.len() as u64 - linkedit_end
        ));
    }
    if let Some((_, off, len)) = signature {
        // (signing again drops it and what follows it: it must be the last thing)
        let (off, end) = (u64::from(off), u64::from(off) + u64::from(len));
        if off < linkedit.1 || end != linkedit_end {
            return Err("the old signature isn't at the end of __LINKEDIT".into());
        }
    }
    Ok(Thin { cputype, filetype, ncmds, sizeofcmds, linkedit, text: text.unwrap_or((0, 0)), signature, uuid, first_data })
}

fn not_macho(b: &[u8]) -> String {
    let head = hex(&b[..b.len().min(4)]);
    let guess = if b.starts_with(b"MZ") {
        ": a Windows .exe?"
    } else if b.starts_with(b"\x7fELF") {
        ": a Linux program?"
    } else if b.starts_with(b"#!") {
        ": a script? Only Mach-O programs are signed"
    } else {
        ""
    };
    format!("not a Mach-O program (it starts with {head}{guess})")
}

fn sign_thin(b: &[u8], identifier: &str, bound: &Bound) -> Result<Vec<u8>, String> {
    let t = parse_thin(b)?;
    let (linkedit_cmd, linkedit_off, linkedit_size) = t.linkedit;
    // the code: up to the old signature, or all of __LINKEDIT
    let code_end = match t.signature {
        Some((_, off, _)) => u64::from(off),
        None => linkedit_off + linkedit_size,
    } as usize;
    if t.signature.is_none() && (HEADER_64 + t.sizeofcmds as usize + LINKEDIT_DATA) as u64 > t.first_data {
        return Err(format!(
            "no room for the signature's load command (the load commands end at byte {}, the program's data starts at {}): link it with more header padding",
            HEADER_64 + t.sizeofcmds as usize,
            t.first_data
        ));
    }
    let sig_off = code_end.next_multiple_of(16);
    let page_shift: u8 = if t.cputype == CPU_TYPE_ARM64 { 14 } else { 12 };
    let page = 1usize << page_shift;
    let n_code = sig_off.div_ceil(page);
    let n_special = if bound.code_resources.is_some() { CSSLOT_RESOURCEDIR } else { CSSLOT_REQUIREMENTS as usize };
    let cd_len = CD_HEADER + identifier.len() + 1 + (n_special + n_code) * HASH_SIZE;
    let blob_len = SUPERBLOB_HEAD + cd_len + EMPTY_REQUIREMENTS.len() + EMPTY_WRAPPER.len();
    let datasize = (blob_len + 8).next_multiple_of(16);
    let total = sig_off + datasize;
    if total > u32::MAX as usize {
        return Err("the program is too big to sign (over 4 GiB)".into());
    }

    // (an old signature's bytes past the new one stay, as codesign leaves them: they aren't hashed)
    let mut out = b.to_vec();
    out.resize(total, 0);
    out[code_end..sig_off].fill(0);
    match t.signature {
        Some((at, _, _)) => {
            put_le32(&mut out, at + 8, sig_off as u32);
            put_le32(&mut out, at + 12, datasize as u32);
        }
        None => {
            let at = HEADER_64 + t.sizeofcmds as usize;
            put_le32(&mut out, at, LC_CODE_SIGNATURE);
            put_le32(&mut out, at + 4, LINKEDIT_DATA as u32);
            put_le32(&mut out, at + 8, sig_off as u32);
            put_le32(&mut out, at + 12, datasize as u32);
            put_le32(&mut out, 16, t.ncmds + 1);
            put_le32(&mut out, 20, t.sizeofcmds + LINKEDIT_DATA as u32);
        }
    }
    let linkedit_filesize = total as u64 - linkedit_off;
    put_le64(&mut out, linkedit_cmd + 32, linkedit_filesize.next_multiple_of(LINKEDIT_VM_ROUND));
    put_le64(&mut out, linkedit_cmd + 48, linkedit_filesize);

    let pages: Vec<[u8; 32]> = out[..sig_off].chunks(page).map(|p| Sha256::digest(p).into()).collect();
    let mut special = vec![[0u8; 32]; n_special];
    if let Some(info) = bound.info_plist {
        special[CSSLOT_INFOSLOT - 1] = Sha256::digest(info).into();
    }
    special[CSSLOT_REQUIREMENTS as usize - 1] = Sha256::digest(EMPTY_REQUIREMENTS).into();
    if let Some(seal) = bound.code_resources {
        special[CSSLOT_RESOURCEDIR - 1] = Sha256::digest(seal).into();
    }

    let mut cd = Vec::with_capacity(cd_len);
    for v in [
        CSMAGIC_CODEDIRECTORY,
        cd_len as u32,
        CS_SUPPORTSEXECSEG,
        CS_ADHOC,
        (CD_HEADER + identifier.len() + 1 + n_special * HASH_SIZE) as u32, // hashOffset
        CD_HEADER as u32,                                                  // identOffset
        n_special as u32,
        n_code as u32,
        sig_off as u32, // codeLimit
    ] {
        cd.extend_from_slice(&v.to_be_bytes());
    }
    cd.extend_from_slice(&[HASH_SIZE as u8, CS_HASHTYPE_SHA256, 0, page_shift]); // hashSize, hashType, platform, pageSize
    cd.extend_from_slice(&[0; 4 + 4 + 4 + 4 + 8]); // spare2, scatterOffset, teamOffset, spare3, codeLimit64
    let exec_flags = if t.filetype == MH_EXECUTE { CS_EXECSEG_MAIN_BINARY } else { 0 };
    for v in [t.text.0, t.text.1, exec_flags] {
        cd.extend_from_slice(&v.to_be_bytes());
    }
    debug_assert_eq!(cd.len(), CD_HEADER);
    cd.extend_from_slice(identifier.as_bytes());
    cd.push(0);
    for h in special.iter().rev() {
        cd.extend_from_slice(h);
    }
    for h in &pages {
        cd.extend_from_slice(h);
    }
    debug_assert_eq!(cd.len(), cd_len);

    let mut blob = Vec::with_capacity(blob_len);
    let req_at = SUPERBLOB_HEAD + cd_len;
    for v in [
        CSMAGIC_EMBEDDED_SIGNATURE,
        blob_len as u32,
        3,
        CSSLOT_CODEDIRECTORY,
        SUPERBLOB_HEAD as u32,
        CSSLOT_REQUIREMENTS,
        req_at as u32,
        CSSLOT_SIGNATURESLOT,
        (req_at + EMPTY_REQUIREMENTS.len()) as u32,
    ] {
        blob.extend_from_slice(&v.to_be_bytes());
    }
    blob.extend_from_slice(&cd);
    blob.extend_from_slice(&EMPTY_REQUIREMENTS);
    blob.extend_from_slice(&EMPTY_WRAPPER);
    out[sig_off..sig_off + blob_len].copy_from_slice(&blob);
    Ok(out)
}

/// A universal file's slices: (offset, length) each; a thin file is one.
fn slices(b: &[u8]) -> Result<Vec<(usize, usize)>, String> {
    let wide = match be32(b, 0) {
        Some(FAT_MAGIC) => false,
        Some(FAT_MAGIC_64) => true,
        _ => return Ok(vec![(0, b.len())]),
    };
    let n = be32(b, 4).ok_or("the universal header is cut short")? as usize;
    if n == 0 {
        return Err("a universal file with no programs in it".into());
    }
    if n > 16 {
        return Err(format!("not a universal Mach-O ({n} slices in its header: a Java .class file?)"));
    }
    let entry = if wide { 32 } else { 20 };
    let header = 8 + n * entry;
    if b.len() < header {
        return Err(format!("the universal header is cut short ({} bytes of {header})", b.len()));
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let at = 8 + i * entry;
        let (offset, size) = if wide {
            (be64(b, at + 8).unwrap(), be64(b, at + 16).unwrap())
        } else {
            (u64::from(be32(b, at + 8).unwrap()), u64::from(be32(b, at + 12).unwrap()))
        };
        let name = cpu_name(be32(b, at).unwrap());
        let end = offset.checked_add(size).filter(|&e| e <= b.len() as u64 && offset >= header as u64).ok_or_else(|| {
            format!("the universal file's {name} slice (bytes {offset}..{}) isn't inside the file ({} bytes): cut short?", offset.saturating_add(size), b.len())
        })?;
        out.push((offset as usize, (end - offset) as usize));
    }
    let mut sorted = out.clone();
    sorted.sort();
    if sorted.windows(2).any(|w| w[0].0 + w[0].1 > w[1].0) {
        return Err("the universal file's slices overlap".into());
    }
    Ok(out)
}

fn sign_fat(b: &[u8], identifier: &str, bound: &Bound) -> Result<Vec<u8>, String> {
    let wide = be32(b, 0) == Some(FAT_MAGIC_64);
    let entry = if wide { 32 } else { 20 };
    let parts = slices(b)?;
    let mut signed = Vec::with_capacity(parts.len());
    for (i, &(start, len)) in parts.iter().enumerate() {
        let at = 8 + i * entry;
        let name = cpu_name(be32(b, at).unwrap());
        let align = be32(b, at + if wide { 24 } else { 16 }).unwrap();
        if align > 20 {
            return Err(format!("the universal file's {name} slice asks for a 2^{align} alignment"));
        }
        signed.push((sign_thin(&b[start..start + len], identifier, bound).map_err(|e| format!("the {name} slice: {e}"))?, align.max(FAT_MIN_ALIGN)));
    }
    let header = 8 + parts.len() * entry;
    let mut out = b[..header].to_vec();
    for (i, (bytes, align)) in signed.iter().enumerate() {
        let offset = out.len().next_multiple_of(1 << align);
        let at = 8 + i * entry;
        if wide {
            out[at + 8..at + 16].copy_from_slice(&(offset as u64).to_be_bytes());
            out[at + 16..at + 24].copy_from_slice(&(bytes.len() as u64).to_be_bytes());
            out[at + 24..at + 28].copy_from_slice(&align.to_be_bytes());
        } else {
            if offset + bytes.len() > u32::MAX as usize {
                return Err("the universal file would be over 4 GiB".into());
            }
            out[at + 8..at + 12].copy_from_slice(&(offset as u32).to_be_bytes());
            out[at + 12..at + 16].copy_from_slice(&(bytes.len() as u32).to_be_bytes());
            out[at + 16..at + 20].copy_from_slice(&align.to_be_bytes());
        }
        out.resize(offset, 0);
        out.extend_from_slice(bytes);
    }
    Ok(out)
}

fn cpu_name(cpu: u32) -> &'static str {
    match cpu {
        CPU_TYPE_ARM64 => "arm64",
        CPU_TYPE_X86_64 => "x86_64",
        0x0000_0007 => "i386",
        0x0000_000c => "arm",
        _ => "unknown-CPU",
    }
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at.checked_add(4)?).map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}

fn le64(b: &[u8], at: usize) -> Option<u64> {
    b.get(at..at.checked_add(8)?).map(|s| u64::from_le_bytes(s.try_into().unwrap()))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at.checked_add(4)?).map(|s| u32::from_be_bytes(s.try_into().unwrap()))
}

fn be64(b: &[u8], at: usize) -> Option<u64> {
    b.get(at..at.checked_add(8)?).map(|s| u64::from_be_bytes(s.try_into().unwrap()))
}

fn put_le32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn put_le64(b: &mut [u8], at: usize, v: u64) {
    b[at..at + 8].copy_from_slice(&v.to_le_bytes());
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

// ------------------------------------------- SHA-1 and base64 (FIPS 180-4, RFC 4648)

/// SHA-1, for CodeResources' version-1 `files` (old systems read it; macOS
/// checks the SHA-256 `files2`).
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476, 0xc3d2_e1f0];
    let (blocks, rest) = data.as_chunks::<64>();
    for block in blocks {
        sha1_block(&mut h, block);
    }
    let mut tail = [0u8; 128];
    tail[..rest.len()].copy_from_slice(rest);
    tail[rest.len()] = 0x80;
    let n = if rest.len() < 56 { 64 } else { 128 };
    tail[n - 8..n].copy_from_slice(&((data.len() as u64).wrapping_mul(8)).to_be_bytes());
    for block in tail[..n].as_chunks::<64>().0 {
        sha1_block(&mut h, block);
    }
    let mut out = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn sha1_block(h: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
        w[i] = u32::from_be_bytes(*word);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let [mut a, mut b, mut c, mut d, mut e] = *h;
    for (i, wi) in w.iter().enumerate() {
        let (f, k) = match i {
            0..=19 => ((b & c) | (!b & d), 0x5a82_7999),
            20..=39 => (b ^ c ^ d, 0x6ed9_eba1),
            40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
            _ => (b ^ c ^ d, 0xca62_c1d6),
        };
        let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = t;
    }
    for (x, v) in h.iter_mut().zip([a, b, c, d, e]) {
        *x = x.wrapping_add(v);
    }
}

fn base64(b: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = u32::from(c[0]) << 16 | u32::from(*c.get(1).unwrap_or(&0)) << 8 | u32::from(*c.get(2).unwrap_or(&0));
        s.push(ALPHABET[(n >> 18) as usize & 63] as char);
        s.push(ALPHABET[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 { ALPHABET[(n >> 6) as usize & 63] as char } else { '=' });
        s.push(if c.len() > 2 { ALPHABET[n as usize & 63] as char } else { '=' });
    }
    s
}

#[cfg(test)]
pub(crate) mod synth {
    //! Small Mach-O programs the tests build themselves (no Apple tools,
    //! nothing checked in): a header, __PAGEZERO, __TEXT with one __text
    //! section of `code` bytes, __LINKEDIT of `linkedit` bytes, and an LC_UUID.

    use super::*;

    pub fn macho(cpu: u32, filetype: u32, code: usize, linkedit: usize, seed: u8) -> Vec<u8> {
        let seg_align: u64 = if cpu == CPU_TYPE_ARM64 { 0x4000 } else { 0x1000 };
        let text_at = 0x400u64; // header padding: room for more load commands
        let text_size = (text_at + code as u64).next_multiple_of(seg_align);
        let linkedit_at = text_size;
        let mut cmds: Vec<u8> = Vec::new();
        let segment = |cmds: &mut Vec<u8>, name: &str, vmaddr: u64, vmsize: u64, fileoff: u64, filesize: u64, prot: u32, sections: &[(&str, u64, u32)]| {
            cmds.extend_from_slice(&LC_SEGMENT_64.to_le_bytes());
            cmds.extend_from_slice(&((SEGMENT_64 + sections.len() * SECTION_64) as u32).to_le_bytes());
            let mut n = [0u8; 16];
            n[..name.len()].copy_from_slice(name.as_bytes());
            cmds.extend_from_slice(&n);
            for v in [vmaddr, vmsize, fileoff, filesize] {
                cmds.extend_from_slice(&v.to_le_bytes());
            }
            for v in [prot, prot, sections.len() as u32, 0] {
                cmds.extend_from_slice(&v.to_le_bytes());
            }
            for &(sect, size, offset) in sections {
                let mut s = [0u8; 16];
                s[..sect.len()].copy_from_slice(sect.as_bytes());
                cmds.extend_from_slice(&s);
                cmds.extend_from_slice(&n);
                cmds.extend_from_slice(&(vmaddr + u64::from(offset)).to_le_bytes());
                cmds.extend_from_slice(&size.to_le_bytes());
                for v in [offset, 4, 0, 0, 0x8000_0400, 0, 0, 0] {
                    cmds.extend_from_slice(&v.to_le_bytes());
                }
            }
        };
        let base = 0x1_0000_0000u64;
        segment(&mut cmds, "__PAGEZERO", 0, base, 0, 0, 0, &[]);
        segment(&mut cmds, "__TEXT", base, text_size, 0, text_size, 5, &[("__text", code as u64, text_at as u32)]);
        segment(&mut cmds, "__LINKEDIT", base + text_size, (linkedit as u64).next_multiple_of(seg_align), linkedit_at, linkedit as u64, 1, &[]);
        cmds.extend_from_slice(&LC_UUID.to_le_bytes());
        cmds.extend_from_slice(&24u32.to_le_bytes());
        cmds.extend((0..16).map(|i| seed.wrapping_mul(31).wrapping_add(i)));
        // LC_BUILD_VERSION (macOS, minimum 11.0 / 10.13, as RapidR's runners): every
        // linker writes one, and codesign picks its CodeDirectory version by it
        let minos: u32 = if cpu == CPU_TYPE_ARM64 { 0x000b_0000 } else { 0x000a_0d00 };
        for v in [0x32, 24, 1, minos, 0x001b_0200, 0] {
            cmds.extend_from_slice(&u32::to_le_bytes(v));
        }
        let mut b = Vec::new();
        let subtype: u32 = if cpu == CPU_TYPE_X86_64 { 3 } else { 0 };
        for v in [MH_MAGIC_64, cpu, subtype, filetype, 5, cmds.len() as u32, 0x0020_0085, 0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&cmds);
        b.resize(text_at as usize, 0);
        // (code and symbols: a byte pattern that differs page to page)
        let mut x = u32::from(seed) | 1;
        let mut noise = |n: usize| -> Vec<u8> {
            (0..n)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 17;
                    x ^= x << 5;
                    x as u8
                })
                .collect()
        };
        b.extend(noise(code));
        b.resize(linkedit_at as usize, 0);
        b.extend(noise(linkedit));
        b
    }

    /// A small linked-looking program for `cpu` whose code is `body` (the
    /// packer's tests tell slices apart by it).
    pub fn program(cpu: u32, body: &[u8]) -> Vec<u8> {
        let mut b = macho(cpu, MH_EXECUTE, body.len(), 64, body.len() as u8);
        b[0x400..0x400 + body.len()].copy_from_slice(body);
        b
    }

    /// A universal file of `slices` (cpu type, bytes), lipo's way: each at a 2^align offset.
    pub fn fat(slices: &[(u32, &[u8], u32)]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&FAT_MAGIC.to_be_bytes());
        b.extend_from_slice(&(slices.len() as u32).to_be_bytes());
        let mut offset = 8 + 20 * slices.len();
        let mut table = Vec::new();
        for &(cpu, bytes, align) in slices {
            offset = offset.next_multiple_of(1 << align);
            let subtype: u32 = if cpu == CPU_TYPE_X86_64 { 3 } else { 0 };
            for v in [cpu, subtype, offset as u32, bytes.len() as u32, align] {
                table.extend_from_slice(&v.to_be_bytes());
            }
            offset += bytes.len();
        }
        b.extend_from_slice(&table);
        for &(_, bytes, align) in slices {
            b.resize(b.len().next_multiple_of(1 << align), 0);
            b.extend_from_slice(bytes);
        }
        b
    }
}

#[cfg(test)]
mod tests {
    use super::synth::{fat, macho};
    use super::*;

    /// The signed slice's layout, checked against the format: the load
    /// command, __LINKEDIT, the SuperBlob, the CodeDirectory's fields.
    fn layout(signed: &[u8], unsigned_code_end: usize, ident: &str, page: usize, specials: usize) {
        let t = parse_thin(signed).unwrap();
        let (_, off, size) = t.signature.expect("LC_CODE_SIGNATURE");
        let (off, size) = (off as usize, size as usize);
        assert_eq!(off, unsigned_code_end.next_multiple_of(16), "signature 16-aligned after the code");
        assert_eq!(off % 16, 0);
        assert_eq!(signed.len(), off + size, "the signature ends the file");
        let (cmd, fileoff, filesize) = t.linkedit;
        assert_eq!(fileoff + filesize, signed.len() as u64, "__LINKEDIT covers it");
        assert_eq!(le64(signed, cmd + 32).unwrap(), filesize.next_multiple_of(0x4000), "vmsize");
        let blob_len = be32(signed, off + 4).unwrap() as usize;
        assert_eq!(size, (blob_len + 8).next_multiple_of(16), "datasize");
        assert_eq!(be32(signed, off + 8), Some(3));
        let s = check(signed).unwrap();
        assert_eq!(s.len(), 1);
        let s = &s[0];
        assert_eq!(s.identifier, ident);
        assert_eq!(s.flags, CS_ADHOC);
        assert_eq!(s.page_size, page);
        assert_eq!(s.code_limit, off);
        assert_eq!(s.pages.len(), off.div_ceil(page));
        assert_eq!(s.special.len(), specials);
        assert_eq!(s.special[1], <[u8; 32]>::from(Sha256::digest(EMPTY_REQUIREMENTS)), "requirements slot");
        // pages recomputed here, independently of the signer
        for (i, p) in signed[..off].chunks(page).enumerate() {
            assert_eq!(<[u8; 32]>::from(Sha256::digest(p)), s.pages[i], "page {i}");
        }
        // the CodeDirectory's own fields
        let cd = off + be32(signed, off + 16).unwrap() as usize;
        assert_eq!(be32(signed, cd), Some(CSMAGIC_CODEDIRECTORY));
        assert_eq!(be32(signed, cd + 8), Some(0x20400), "version");
        assert_eq!(be32(signed, cd + 20), Some(88), "identOffset");
        assert_eq!(signed[cd + 37], CS_HASHTYPE_SHA256);
        assert_eq!(signed[cd + 38], 0, "platform");
        assert_eq!(&signed[cd + 44..cd + 64], &[0u8; 20], "no scatter, no team, codeLimit64 0");
        assert_eq!(be64(signed, cd + 64), Some(t.text.0), "execSegBase");
        assert_eq!(be64(signed, cd + 72), Some(t.text.1), "execSegLimit");
        let wrapper = off + be32(signed, off + 32).unwrap() as usize;
        assert_eq!(&signed[wrapper..wrapper + 8], &EMPTY_WRAPPER, "the empty signature slot: ad hoc");
    }

    #[test]
    fn thin_arm64() {
        let m = macho(CPU_TYPE_ARM64, MH_EXECUTE, 40_000, 3_001, 1);
        let s = sign(&m, "dev.rapidr.test", Bound::default()).unwrap();
        layout(&s, m.len(), "dev.rapidr.test", 16384, 2);
        let t = parse_thin(&s).unwrap();
        assert_eq!(t.ncmds, 6, "LC_CODE_SIGNATURE added");
        assert_eq!(t.sizeofcmds, parse_thin(&m).unwrap().sizeofcmds + 16);
        // the code before the header's changes is untouched
        assert_eq!(&s[0x400..m.len()], &m[0x400..]);
        // an executable: its __TEXT is the main binary's (execSegFlags, 80 bytes into
        // the CodeDirectory, which starts after the SuperBlob's 36-byte head)
        let off = t.signature.unwrap().1 as usize;
        assert_eq!(be64(&s, off + SUPERBLOB_HEAD + 80), Some(CS_EXECSEG_MAIN_BINARY));
    }

    #[test]
    fn thin_x86_64() {
        let m = macho(CPU_TYPE_X86_64, MH_EXECUTE, 9_000, 1_000, 2);
        let s = sign(&m, "hello", Bound::default()).unwrap();
        layout(&s, m.len(), "hello", 4096, 2);
        assert_eq!(check(&s).unwrap()[0].cpu_type, CPU_TYPE_X86_64);
    }

    #[test]
    fn a_dylib_is_no_main_binary() {
        let m = macho(CPU_TYPE_ARM64, 6, 1_000, 500, 3);
        let s = sign(&m, "libx", Bound::default()).unwrap();
        let off = parse_thin(&s).unwrap().signature.unwrap().1 as usize;
        assert_eq!(be64(&s, off + SUPERBLOB_HEAD + 80), Some(0), "execSegFlags");
    }

    #[test]
    fn universal() {
        let a = macho(CPU_TYPE_ARM64, MH_EXECUTE, 20_000, 2_000, 4);
        let x = macho(CPU_TYPE_X86_64, MH_EXECUTE, 7_000, 900, 5);
        let f = fat(&[(CPU_TYPE_X86_64, &x, 12), (CPU_TYPE_ARM64, &a, 14)]);
        let s = sign(&f, "dev.rapidr.fat", Bound::default()).unwrap();
        let parts = slices(&s).unwrap();
        assert_eq!(parts.len(), 2);
        // each slice signed as it would be alone; in order, 2^14-aligned
        assert_eq!(&s[parts[0].0..parts[0].0 + parts[0].1], &sign(&x, "dev.rapidr.fat", Bound::default()).unwrap()[..]);
        assert_eq!(&s[parts[1].0..parts[1].0 + parts[1].1], &sign(&a, "dev.rapidr.fat", Bound::default()).unwrap()[..]);
        assert_eq!(parts[0].0, 16384);
        assert_eq!(parts[1].0, (parts[0].0 + parts[0].1).next_multiple_of(16384));
        assert_eq!(be32(&s, 8 + 16), Some(14), "x86_64's alignment raised to 2^14");
        assert_eq!(s.len(), parts[1].0 + parts[1].1);
        assert!(s[8 + 40..16384].iter().all(|&b| b == 0));
        let checked = check(&s).unwrap();
        assert_eq!(checked.iter().map(|c| (c.cpu_type, c.page_size)).collect::<Vec<_>>(), [(CPU_TYPE_X86_64, 4096), (CPU_TYPE_ARM64, 16384)]);
        // signed again: the same
        assert_eq!(sign(&s, "dev.rapidr.fat", Bound::default()).unwrap(), s);
    }

    #[test]
    fn signing_again_replaces_the_signature() {
        let m = macho(CPU_TYPE_ARM64, MH_EXECUTE, 30_000, 1_234, 6);
        let once = sign(&m, "one", Bound::default()).unwrap();
        assert_eq!(sign(&once, "one", Bound::default()).unwrap(), once, "idempotent");
        let other = sign(&once, "another.name", Bound::default()).unwrap();
        assert_eq!(check(&other).unwrap()[0].identifier, "another.name");
        assert_eq!(parse_thin(&other).unwrap().ncmds, 6, "the load command reused, not added twice");
        layout(&other, m.len(), "another.name", 16384, 2);
    }

    #[test]
    fn a_bundles_program_binds_info_plist_and_the_seal() {
        let m = macho(CPU_TYPE_ARM64, MH_EXECUTE, 5_000, 700, 7);
        let (plist, seal) = (b"<plist/>".as_slice(), b"seal".as_slice());
        let s = sign(&m, "dev.rapidr.app.x", Bound { info_plist: Some(plist), code_resources: Some(seal) }).unwrap();
        layout(&s, m.len(), "dev.rapidr.app.x", 16384, 3);
        let c = &check(&s).unwrap()[0];
        assert_eq!(c.special[0], <[u8; 32]>::from(Sha256::digest(plist)));
        assert_eq!(c.special[2], <[u8; 32]>::from(Sha256::digest(seal)));
        // without a bundle the Info.plist slot is zero
        assert_eq!(check(&sign(&m, "x", Bound::default()).unwrap()).unwrap()[0].special[0], [0; 32]);
    }

    #[test]
    fn a_changed_page_is_found() {
        let m = macho(CPU_TYPE_X86_64, MH_EXECUTE, 9_000, 600, 8);
        let mut s = sign(&m, "x", Bound::default()).unwrap();
        s[5000] ^= 1;
        let e = check(&s).unwrap_err();
        assert!(e.contains("changed after it was signed (page 1 of"), "{e}");
        assert!(check(&m).unwrap_err().contains("isn't signed"));
    }

    #[test]
    fn corrupted_inputs_are_refused_with_a_reason() {
        let good = macho(CPU_TYPE_ARM64, MH_EXECUTE, 3_000, 800, 9);
        let refuse = |b: &[u8], want: &str| {
            let e = sign(b, "x", Bound::default()).unwrap_err();
            assert!(e.contains(want), "wanted {want:?}, got {e:?}");
        };
        refuse(b"", "too short to be a Mach-O program (0 bytes)");
        refuse(b"MZ\x90\0rest", "not a Mach-O program (it starts with 4d5a9000: a Windows .exe?)");
        refuse(b"\x7fELF\x02\x01", "a Linux program?");
        refuse(b"#!/bin/sh\necho hi\n", "a script?");
        refuse(&good[..20], "the Mach-O header is cut short (20 bytes of 32)");
        refuse(&good[..100], "its load commands end at byte 376, the file at 100");
        refuse(&good[..good.len() - 10], "segment __LINKEDIT runs past the end of the file (it ends at byte 17184, the file at 17174): cut short?");
        let mut extra = good.clone();
        extra.extend_from_slice(b"RRBCEXE1payload");
        refuse(&extra, "15 bytes after the end of __LINKEDIT: a signed program can carry nothing after its code");
        let mut b32 = good.clone();
        b32[..4].copy_from_slice(&MH_MAGIC.to_le_bytes());
        refuse(&b32, "a 32-bit Mach-O");
        let mut be = good.clone();
        be[..4].copy_from_slice(&MH_CIGAM_64.to_le_bytes());
        refuse(&be, "big-endian");
        let mut cpu = good.clone();
        cpu[4..8].copy_from_slice(&18u32.to_le_bytes());
        refuse(&cpu, "CPU type 0x12 isn't one macOS runs");
        let mut zero = good.clone();
        zero[32 + 4..32 + 8].copy_from_slice(&0u32.to_le_bytes());
        refuse(&zero, "load command 0 (at byte 32) is broken: its size 0 doesn't fit");
        let mut no_linkedit = good.clone();
        let at = 32 + 72 + 152 + 8;
        no_linkedit[at..at + 10].copy_from_slice(b"__DATA\0\0\0\0");
        refuse(&no_linkedit, "no __LINKEDIT segment");
        let mut cramped = good.clone();
        // the __text section's data right after the load commands: no room for one more
        let sect_offset_at = 32 + 72 + 72 + 48;
        cramped[sect_offset_at..sect_offset_at + 4].copy_from_slice(&(32 + 344u32).to_le_bytes());
        refuse(&cramped, "no room for the signature's load command (the load commands end at byte 376, the program's data starts at 376)");
        // universal files
        let f = fat(&[(CPU_TYPE_ARM64, &good, 14)]);
        refuse(&f[..f.len() - 1], "the universal file's arm64 slice (bytes 16384..");
        let mut none = f.clone();
        none[4..8].copy_from_slice(&0u32.to_be_bytes());
        refuse(&none, "a universal file with no programs in it");
        refuse(&[0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 52, 0, 0], "a Java .class file?");
        let mut bad_slice = f.clone();
        bad_slice[16384..16388].copy_from_slice(b"junk");
        refuse(&bad_slice, "the arm64 slice: not a Mach-O program");
        let i386 = fat(&[(7, &b32, 12)]);
        refuse(&i386, "the i386 slice: a 32-bit Mach-O");
        // and a bad identifier
        assert!(sign(&good, "", Bound::default()).is_err());
    }

    #[test]
    fn default_identifiers_are_codesigns() {
        let m = macho(CPU_TYPE_ARM64, MH_EXECUTE, 100, 100, 10);
        let uuid = hex(&parse_thin(&m).unwrap().uuid.unwrap());
        assert_eq!(default_identifier("hello", &m), format!("hello-55554944{uuid}"));
        assert_eq!(default_identifier("dir/libf.dylib", &m), format!("libf-55554944{uuid}"));
        // a universal file: the arm64 slice's
        let x = macho(CPU_TYPE_X86_64, MH_EXECUTE, 100, 100, 11);
        assert_eq!(default_identifier("u", &fat(&[(CPU_TYPE_X86_64, &x, 12), (CPU_TYPE_ARM64, &m, 14)])), format!("u-55554944{uuid}"));
        assert_eq!(default_identifier(".hidden", b"junk"), ".hidden");
    }

    fn app(exe: Vec<u8>) -> Vec<(String, Vec<u8>)> {
        let plist = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\">\n<dict>\n\t<key>CFBundleExecutable</key>\n\t<string>hello</string>\n\t<key>CFBundleIdentifier</key>\n\t<string>dev.rapidr.app.a&amp;b</string>\n</dict>\n</plist>\n";
        vec![
            ("Contents/Info.plist".into(), plist.as_bytes().to_vec()),
            ("Contents/PkgInfo".into(), b"APPL????".to_vec()),
            ("Contents/MacOS/hello".into(), exe),
            ("Contents/Resources/AppIcon.icns".into(), vec![0; 5000]),
            ("Contents/Resources/lang/es.po".into(), b"msgid".to_vec()),
            ("Contents/Resources/es.lproj/Localizable.strings".into(), b"y".to_vec()),
            ("Contents/Resources/es.lproj/locversion.plist".into(), b"z".to_vec()),
            // (not directly in the .lproj folder: sealed, optional)
            ("Contents/Resources/es.lproj/sub/locversion.plist".into(), b"v".to_vec()),
            ("Contents/Resources/Base.lproj/Main.strings".into(), b"w".to_vec()),
            ("Contents/Resources/a&b<c>\"d'e.txt".into(), b"x".to_vec()),
            ("Contents/Resources/.DS_Store".into(), b"junk".to_vec()),
        ]
    }

    #[test]
    fn an_app_is_signed_and_sealed() {
        let exe = macho(CPU_TYPE_ARM64, MH_EXECUTE, 10_000, 1_000, 12);
        let mut files = app(exe.clone());
        files.push(("Contents/_CodeSignature/CodeResources".into(), b"old seal".to_vec()));
        sign_app(&mut files).unwrap();
        let get = |p: &str| files.iter().filter(|(q, _)| q == p).map(|(_, b)| b.clone()).collect::<Vec<_>>();
        let seal = get("Contents/_CodeSignature/CodeResources");
        assert_eq!(seal.len(), 1, "one seal, the new one");
        let text = String::from_utf8(seal[0].clone()).unwrap();
        let signed = &get("Contents/MacOS/hello")[0];
        let c = &check(signed).unwrap()[0];
        assert_eq!(c.identifier, "dev.rapidr.app.a&b", "Info.plist's CFBundleIdentifier, unescaped");
        assert_eq!(c.special.len(), 3);
        assert_eq!(c.special[0], <[u8; 32]>::from(Sha256::digest(&get("Contents/Info.plist")[0])));
        assert_eq!(c.special[2], <[u8; 32]>::from(Sha256::digest(&seal[0])));
        // the seal: sorted, escaped, lproj optional, locversion and .DS_Store left out
        let keys: Vec<&str> = text.lines().filter_map(|l| l.strip_prefix("\t\t<key>")).map(|k| k.trim_end_matches("</key>")).collect();
        let files_keys = [
            "Resources/AppIcon.icns",
            "Resources/Base.lproj/Main.strings",
            "Resources/a&amp;b&lt;c&gt;\"d'e.txt",
            "Resources/es.lproj/Localizable.strings",
            "Resources/es.lproj/sub/locversion.plist",
            "Resources/lang/es.po",
        ];
        assert_eq!(&keys[..6], &files_keys, "files");
        assert_eq!(&keys[6..12], &files_keys, "files2");
        assert!(text.contains("\t\t<key>Resources/AppIcon.icns</key>\n\t\t<data>\n\t\tBE70ivJk/T4wSrnmT5ZW83r3Y6Y=\n\t\t</data>\n"), "SHA-1 of 5000 zeros");
        assert!(text.contains("\t\t\t<key>hash2</key>\n\t\t\t<data>\n\t\t\tfKW9h585PZ3QWxTzit2cD8a2eSj38tJhsuR6Mu6CGeM=\n\t\t\t</data>\n"), "SHA-256 of 5000 zeros");
        assert!(text.contains("<key>Resources/es.lproj/Localizable.strings</key>\n\t\t<dict>\n\t\t\t<key>hash</key>\n\t\t\t<data>\n\t\t\tlcsL/Sl3x2EpjZYk5LTUxyo5l0o=\n\t\t\t</data>\n\t\t\t<key>optional</key>\n\t\t\t<true/>\n\t\t</dict>\n"));
        assert!(text.ends_with("\t\t\t<real>20</real>\n\t\t</dict>\n\t</dict>\n</dict>\n</plist>\n"));
        // the other files are as they were
        assert_eq!(get("Contents/Resources/.DS_Store")[0], b"junk");
    }

    #[test]
    fn an_empty_app_seals_nothing() {
        let seal = String::from_utf8(code_resources(&[])).unwrap();
        assert!(seal.contains("\t<key>files</key>\n\t<dict/>\n\t<key>files2</key>\n\t<dict/>\n\t<key>rules</key>\n"));
    }

    #[test]
    fn apps_it_cant_seal_are_refused() {
        let exe = macho(CPU_TYPE_ARM64, MH_EXECUTE, 1_000, 100, 13);
        type Files = Vec<(String, Vec<u8>)>;
        let refuse = |change: &dyn Fn(&mut Files), want: &str| {
            let mut files = app(exe.clone());
            change(&mut files);
            let before = files.clone();
            let e = sign_app(&mut files).unwrap_err();
            assert!(e.contains(want), "wanted {want:?}, got {e:?}");
            assert_eq!(files, before, "nothing changed");
        };
        refuse(&|f| f.push(("Contents/MacOS/helper".into(), vec![1])), "Contents/MacOS/helper: an app RapidR signs keeps its files in Contents/Resources");
        refuse(&|f| f.push(("Contents/Frameworks/x.dylib".into(), vec![1])), "other code would need a signature of its own first");
        refuse(&|f| f.push(("Contents/Resources/../../evil".into(), vec![1])), "isn't a path inside the app");
        refuse(&|f| f.push(("/etc/passwd".into(), vec![1])), "isn't a path inside the app");
        refuse(&|f| f.retain(|(p, _)| p != "Contents/MacOS/hello"), "the app's program Contents/MacOS/hello isn't there");
        refuse(&|f| f.retain(|(p, _)| p != "Contents/Info.plist"), "the app has no Contents/Info.plist");
        refuse(&|f| f[0].1 = b"bplist00...".to_vec(), "binary plist");
        refuse(&|f| f[0].1 = b"<plist><dict></dict></plist>".to_vec(), "Info.plist has no CFBundleExecutable");
        refuse(&|f| f[2].1 = b"#!/bin/sh\n".to_vec(), "Contents/MacOS/hello: not a Mach-O program (it starts with 23212f62: a script?");
    }

    #[test]
    fn sha1_and_base64_meet_their_standards() {
        assert_eq!(hex(&sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(hex(&sha1(b"abc")), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(hex(&sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")), "84983e441c3bd26ebaae4aa1f95129e5e54670f1");
        assert_eq!(hex(&sha1(&vec![b'a'; 1_000_000])), "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
        for (i, want) in ["", "Zg==", "Zm8=", "Zm9v", "Zm9vYg==", "Zm9vYmE=", "Zm9vYmFy"].iter().enumerate() {
            assert_eq!(base64(&b"foobar"[..i]), *want);
        }
    }

    #[test]
    fn plist_strings_are_read() {
        let p = b"<dict>\n\t<key>CFBundleIdentifier</key>\n\t<string>a&lt;b&#65;&#x42;</string>\n\t<key>E</key><string/></dict>";
        assert_eq!(plist_string(p, "CFBundleIdentifier").unwrap(), "a<bAB");
        assert_eq!(plist_string(p, "E").unwrap(), "");
        assert!(plist_string(b"<key>X</key><string>a&bogus</string>", "X").is_err());
    }

    /// The golden tests: what `codesign --force --sign -` writes, byte for
    /// byte, for the same input (macOS only; `codesign` is used to compare,
    /// never to sign RapidR's apps).
    #[cfg(target_os = "macos")]
    mod golden {
        use super::*;
        use std::path::{Path, PathBuf};
        use std::process::Command;

        fn temp(name: &str) -> PathBuf {
            let dir = std::env::temp_dir().join(format!("rapidr-macho-golden-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            dir
        }

        fn codesign(path: &Path, identifier: Option<&str>) {
            let mut c = Command::new("codesign");
            c.args(["--force", "--sign", "-"]);
            if let Some(id) = identifier {
                c.args(["--identifier", id]);
            }
            let out = c.arg(path).output().expect("codesign");
            assert!(out.status.success(), "codesign {}: {}", path.display(), String::from_utf8_lossy(&out.stderr));
        }

        fn same_as_codesign(name: &str, input: &[u8], identifier: &str) {
            let dir = temp(name);
            let path = dir.join(name);
            std::fs::write(&path, input).unwrap();
            codesign(&path, Some(identifier));
            let theirs = std::fs::read(&path).unwrap();
            let ours = sign(input, identifier, Bound::default()).unwrap();
            // (both kept for a look when they differ: the folder goes only when they match)
            std::fs::write(dir.join(format!("{name}.rapidr")), &ours).unwrap();
            let (a, b) = (check(&theirs).unwrap(), check(&ours).unwrap());
            assert_eq!(a, b, "{name}: the same CodeDirectories (cdhash, page hashes, slots)");
            assert!(theirs == ours, "{name}: byte-identical ({} vs {} bytes; first difference at {:?})", theirs.len(), ours.len(), theirs.iter().zip(&ours).position(|(x, y)| x != y));
            std::fs::remove_dir_all(&dir).unwrap();
        }

        #[test]
        fn synthetic_programs() {
            let a = macho(CPU_TYPE_ARM64, MH_EXECUTE, 40_000, 3_001, 21);
            let x = macho(CPU_TYPE_X86_64, MH_EXECUTE, 9_000, 1_000, 22);
            same_as_codesign("arm64", &a, "dev.rapidr.golden.arm64");
            same_as_codesign("x86_64", &x, "dev.rapidr.golden.x86");
            same_as_codesign("fat", &fat(&[(CPU_TYPE_X86_64, &x, 12), (CPU_TYPE_ARM64, &a, 14)]), "dev.rapidr.golden.fat");
            same_as_codesign("dylib", &macho(CPU_TYPE_ARM64, 6, 2_000, 300, 23), "dev.rapidr.golden.dylib");
            // already signed (by us), signed again with a longer name
            same_as_codesign("resigned", &sign(&a, "short", Bound::default()).unwrap(), "a.much.longer.identifier.than.before");
        }

        #[test]
        fn real_programs() {
            // this test program: a linker-signed arm64 (or x86_64) Rust program
            let me = std::fs::read(std::env::current_exe().unwrap()).unwrap();
            same_as_codesign("rust-test-program", &me, "dev.rapidr.golden.me");
            // a copy of a system program: universal x86_64 + arm64e, Apple-signed; and its x86_64 slice
            let tru = std::fs::read("/usr/bin/true").unwrap();
            same_as_codesign("true", &tru, "dev.rapidr.golden.true");
            let parts = slices(&tru).unwrap();
            let x86 = parts.iter().find(|&&(s, l)| parse_thin(&tru[s..s + l]).map(|t| t.cputype) == Ok(CPU_TYPE_X86_64));
            if let Some(&(s, l)) = x86 {
                same_as_codesign("true-x86_64", &tru[s..s + l], "dev.rapidr.golden.true86");
            }
        }

        #[test]
        fn an_app() {
            let me = std::fs::read(std::env::current_exe().unwrap()).unwrap();
            let dir = temp("app");
            let app_dir = dir.join("Hello.app");
            let files = {
                let mut f = app(me);
                // (.DS_Store: codesign leaves it out too, but a Finder file makes --strict unhappy elsewhere; keep the app clean)
                f.retain(|(p, _)| !p.ends_with(".DS_Store"));
                f.push(("Contents/Resources/caf\u{e9}.txt".into(), "\u{fc}".as_bytes().to_vec()));
                f
            };
            for (p, b) in &files {
                let at = app_dir.join(p);
                std::fs::create_dir_all(at.parent().unwrap()).unwrap();
                std::fs::write(&at, b).unwrap();
            }
            codesign(&app_dir, None);
            let mut ours = files.clone();
            sign_app(&mut ours).unwrap();
            for (p, b) in &ours {
                let theirs = std::fs::read(app_dir.join(p)).unwrap();
                assert!(theirs == *b, "{p}: byte-identical to codesign's ({} vs {} bytes)", theirs.len(), b.len());
            }
            std::fs::remove_dir_all(&dir).unwrap();
        }
    }
}
