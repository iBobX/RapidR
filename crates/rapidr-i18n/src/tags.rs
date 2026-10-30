//! Language tags as RapidR writes them (`es-AR`, `zh-Hans`), the
//! catalogs a language looks in, and the system's language.

// -------------------------------------------------------------- languages --

/// A language tag as RapidR writes it: `es-AR`, `zh-Hans-CN`, `pt-BR` —
/// from `es_ar`, `ES-ar`, a system's `es_AR.UTF-8@euro`. "" for none
/// (`C`, `POSIX`, empty).
pub fn normalize_tag(tag: &str) -> String {
    let tag = tag.trim();
    let tag = tag.split(['.', '@']).next().unwrap_or("");
    if tag.eq_ignore_ascii_case("c") || tag.eq_ignore_ascii_case("posix") {
        return String::new();
    }
    let mut out = String::new();
    for (i, part) in tag.split(['-', '_']).filter(|p| !p.is_empty()).enumerate() {
        if !part.chars().all(|c| c.is_ascii_alphanumeric()) {
            break;
        }
        if i > 0 {
            out.push('-');
        }
        if i == 0 {
            out.push_str(&part.to_ascii_lowercase());
        } else if part.len() == 4 && part.chars().all(|c| c.is_ascii_alphabetic()) {
            // a script: Hans, Latn
            out.push_str(&part[..1].to_ascii_uppercase());
            out.push_str(&part[1..].to_ascii_lowercase());
        } else if (part.len() == 2 && part.chars().all(|c| c.is_ascii_alphabetic())) || (part.len() == 3 && part.chars().all(|c| c.is_ascii_digit())) {
            // a region: AR, BR, 419
            out.push_str(&part.to_ascii_uppercase());
        } else {
            out.push_str(&part.to_ascii_lowercase());
        }
    }
    out
}

/// The catalogs a language looks in, most specific first: `es-AR` →
/// `es-AR`, `es`; `zh-Hans-CN` → `zh-Hans-CN`, `zh-Hans`, `zh`. Chinese
/// named by its region alone takes its script's file too (`zh-CN` →
/// `zh-CN`, `zh-Hans`, `zh`; `zh-TW` → … `zh-Hant` …).
pub fn fallback_chain(tag: &str) -> Vec<String> {
    let tag = normalize_tag(tag);
    let mut out = Vec::new();
    let mut t = tag.as_str();
    while !t.is_empty() {
        out.push(t.to_string());
        t = t.rfind('-').map_or("", |i| &t[..i]);
    }
    let parts: Vec<&str> = tag.split('-').collect();
    if parts.len() == 2 && parts[0] == "zh" {
        let script = match parts[1] {
            "CN" | "SG" | "MY" => Some("zh-Hans"),
            "TW" | "HK" | "MO" => Some("zh-Hant"),
            _ => None,
        };
        if let Some(s) = script {
            out.insert(1, s.to_string());
        }
    }
    out
}

#[cfg(target_os = "macos")]
pub fn os_language() -> Option<String> {
    use std::ffi::{c_char, c_void, CStr};
    type CfRef = *const c_void;
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFLocaleCopyPreferredLanguages() -> CfRef;
        fn CFArrayGetCount(array: CfRef) -> isize;
        fn CFArrayGetValueAtIndex(array: CfRef, index: isize) -> CfRef;
        fn CFStringGetCString(string: CfRef, buffer: *mut c_char, size: isize, encoding: u32) -> u8;
        fn CFRelease(cf: CfRef);
    }
    const UTF8: u32 = 0x0800_0100;
    // SAFETY: CoreFoundation's documented calls: the array is ours (Copy)
    // and released once; its first string is read into a buffer of the
    // size given, NUL-terminated on success.
    unsafe {
        let langs = CFLocaleCopyPreferredLanguages();
        if langs.is_null() {
            return None;
        }
        let mut out = None;
        if CFArrayGetCount(langs) > 0 {
            let first = CFArrayGetValueAtIndex(langs, 0);
            let mut buf = [0 as c_char; 128];
            if !first.is_null() && CFStringGetCString(first, buf.as_mut_ptr(), buf.len() as isize, UTF8) != 0 {
                out = Some(CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned());
            }
        }
        CFRelease(langs);
        out
    }
}

#[cfg(windows)]
pub fn os_language() -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultLocaleName(name: *mut u16, size: i32) -> i32;
    }
    let mut buf = [0u16; 85];
    // SAFETY: the buffer and its size in characters (LOCALE_NAME_MAX_LENGTH).
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    (n > 1).then(|| String::from_utf16_lossy(&buf[..n as usize - 1]))
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn os_language() -> Option<String> {
    None
}

/// The system's language on a computer: `LANGUAGE`'s first, `LC_ALL`,
/// `LC_MESSAGES` (the user's explicit choice), then the system's own
/// setting (macOS' preferred languages, Windows' user locale), then
/// `LANG`; English when none says. (A browser's is the page's to ask.)
pub fn system_language() -> String {
    let env = |var: &str| std::env::var(var).ok().map(|v| normalize_tag(v.split(':').next().unwrap_or(""))).filter(|t| !t.is_empty());
    if let Some(t) = env("LANGUAGE").or_else(|| env("LC_ALL")).or_else(|| env("LC_MESSAGES")) {
        return t;
    }
    if let Some(t) = os_language().map(|l| normalize_tag(&l)).filter(|t| !t.is_empty()) {
        return t;
    }
    env("LANG").unwrap_or_else(|| "en".to_string())
}

/// A language's own name, for the lists people pick from ("Español"),
/// else the tag itself. Names are never translated: each is written as
/// its speakers write it.
pub fn native_name(tag: &str) -> String {
    let t = normalize_tag(tag);
    let name = match t.as_str() {
        "en" => "English",
        "es" => "Español",
        "pt" => "Português",
        "pt-BR" => "Português (Brasil)",
        "pt-PT" => "Português (Portugal)",
        "zh-Hans" => "中文（简体）",
        "zh-Hant" => "中文（繁體）",
        "fr" => "Français",
        "de" => "Deutsch",
        "it" => "Italiano",
        "ja" => "日本語",
        "ko" => "한국어",
        crate::pseudo::TAG => "Pšéûðö (xx)",
        _ => return t,
    };
    name.to_string()
}
