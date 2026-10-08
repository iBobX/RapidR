//! `DECLARE … LIB`: what both compilers and every runtime share about
//! calling DLLs (docs/windows-dll-calls.md) — which libraries are
//! Windows' own, what a program could use instead, the DECLARE's calling
//! *spec* the compilers hand to the runtime, and the error texts for the
//! systems that can't make the call.

/// Windows system DLLs (`DECLARE … LIB "user32"` …). Calls into other DLLs
/// are the program's own libraries.
pub fn is_windows_system_library(lib: &str) -> bool {
    let base = library_base(lib);
    matches!(
        base.as_str(),
        "user32" | "kernel32" | "gdi32" | "gdiplus" | "shell32" | "winmm" | "advapi32" | "comctl32"
            | "comdlg32" | "wsock32" | "ws2_32" | "ole32" | "oleaut32" | "odbc32" | "odbccp32"
            | "wininet" | "winspool.drv" | "winspool" | "version" | "shlwapi" | "psapi" | "ntdll"
            | "msvcrt" | "mpr" | "netapi32" | "iphlpapi" | "urlmon" | "imm32" | "msimg32" | "avifil32"
            | "msvfw32" | "vfw32" | "opengl32" | "glu32" | "ddraw" | "dsound" | "dinput" | "rasapi32"
            | "setupapi" | "powrprof" | "secur32" | "crypt32" | "dwmapi" | "uxtheme" | "winhttp"
            | "crtdll" | "lz32" | "mapi32" | "msacm32" | "rpcrt4" | "winscard" | "wintrust" | "hid"
            | "avicap32" | "tapi32" | "icmp" | "url" | "hhctrl.ocx" | "msvbvm60"
    )
}

/// `"USER32.DLL"`, `"c:\x\user32.dll"`, `"user32"` → `user32`.
pub fn library_base(lib: &str) -> String {
    let base = lib.trim_matches('"').rsplit(['\\', '/']).next().unwrap_or(lib).to_ascii_lowercase();
    base.strip_suffix(".dll").map(str::to_string).unwrap_or(base)
}

/// What to use instead of a Windows API call, by API name (lowercase,
/// with or without its A / W suffix).
pub fn windows_api_hint(api: &str) -> Option<&'static str> {
    let a = api.to_ascii_lowercase();
    let a = a.as_str();
    let hint = hint_for(a).or_else(|| hint_for(a.trim_end_matches(['a', 'w'])));
    hint
}

fn hint_for(a: &str) -> Option<&'static str> {
    Some(if a.starts_with("sql") {
        "use the RSQLITE or RMYSQL component for databases"
    } else if a.starts_with("gdip") {
        "load, draw and save images with RIMAGE / RCANVAS"
    } else if a.starts_with("mcisend") || matches!(a, "playsound" | "sndplaysound" | "waveoutopen") {
        "play sounds with PLAYSOUND (or RWEBAUDIO on the web)"
    } else if matches!(a, "shellexecute" | "shellexecuteex" | "winexec" | "createprocess") {
        "run programs and open files with SHELL / SHELLWAIT"
    } else if a.starts_with("joy") {
        "read joysticks and gamepads with a QDXJOYSTICK (Update, IsLeft / IsRight / IsUp / IsDown, Button(n); RapidR's X, Y, Buttons, POV, OnButtonDown …)"
    } else if matches!(a, "sleep") {
        "use SLEEP"
    } else if matches!(a, "messagebox" | "messageboxex") {
        "use SHOWMESSAGE"
    } else if matches!(a, "messagebeep" | "beep") {
        "use BEEP"
    } else if matches!(a, "gettickcount" | "timegettime" | "queryperformancecounter") {
        "use TIMER"
    } else if matches!(a, "getsystemmetrics") {
        "use Screen.Width / Screen.Height"
    } else if matches!(a, "getenvironmentvariable") {
        "use ENVIRON$"
    } else if matches!(a, "getcurrentdirectory" | "setcurrentdirectory") {
        "use CURDIR$ / CHDIR"
    } else if matches!(a, "createdirectory" | "removedirectory" | "deletefile" | "movefile") {
        "use MKDIR / RMDIR / KILL / RENAME"
    } else if matches!(
        a,
        "selectobject" | "createpen" | "createsolidbrush" | "deleteobject" | "getstockobject" | "getobject"
            | "getcurrentobject" | "ellipse" | "rectangle" | "lineto" | "moveto" | "movetoex" | "bitblt"
            | "stretchblt" | "getdc" | "releasedc" | "setpixel" | "getpixel" | "textout" | "setgraphicsmode"
            | "setworldtransform" | "createfont" | "createfontindirect" | "polygon"
    ) {
        "draw with an RCANVAS (Line, Circle, Rectangle, TextOut, …)"
    } else if matches!(
        a,
        "setwindowlong" | "getwindowlong" | "callwindowproc" | "setwindowpos" | "showwindow" | "movewindow"
            | "setfocus" | "getfocus" | "findwindow" | "getwindowrect" | "getclientrect" | "createwindowex"
            | "windowfrompoint" | "sendmessage" | "sendmessageapi" | "postmessage" | "setwindowtext"
            | "getwindowtext" | "getactivewindow" | "setforegroundwindow" | "enablewindow" | "destroywindow"
            | "setparent" | "getsyscolor" | "registerhotkey" | "setcapture" | "releasecapture"
    ) {
        "use the component's own properties, methods and events (Left, Top, Visible, Caption, SetFocus, OnKeyDown, …)"
    } else if matches!(
        a,
        "socket" | "recv" | "send" | "connect" | "bind" | "listen" | "accept" | "closesocket" | "htons" | "htonl"
            | "ntohs" | "inet_addr" | "gethostbyname" | "wsastartup" | "wsacleanup" | "wsaasyncselect" | "ioctlsocket"
    ) {
        "use the RSOCKET / RSERVERSOCKET components"
    } else if a.starts_with("internet") || a.starts_with("qftp_internet") || a.starts_with("http") || a == "urldownloadtofile" {
        "use the RHTTP component"
    } else if matches!(a, "multibytetowidechar" | "widechartomultibyte" | "lstrlen" | "lstrcpy") {
        "RapidR strings are Unicode already; use the string functions (LEN, MID$, …)"
    } else if matches!(a, "copymemory" | "rtlmovememory" | "movememory" | "zeromemory" | "fillmemory") {
        "use MEMCPY / MEMSET on VARPTR addresses"
    } else if a.starts_with("reg") {
        "use QREGISTRY (Windows' registry there, a per-user store elsewhere)"
    } else {
        return None;
    })
}

/// What a `CODEPTR(Proc)` / `CALLBACK(Proc)` argument of a DLL call
/// becomes, followed by the routine's name (`rapidr_ast::memory`'s
/// `DLL_CALLBACK_MARKER`): the DLL would call it, which RapidR can't do yet.
pub const CALLBACK_MARKER: &str = "\u{0}rapidr-callback:";

/// The error for a SUB or FUNCTION handed to a DLL as a callback.
pub fn callback_error(name: &str, routine: &str) -> String {
    format!("'{name}' is given CODEPTR({routine}), a callback the DLL would call back into the program; RapidR doesn't pass SUBs and FUNCTIONs to DLLs yet")
}

/// The error for a DLL call in a sandboxed run (`RAPIDR_SANDBOX`: code
/// RapidR runs without the user having started it themselves, such as an
/// assistant's run): no library is loaded.
pub fn sandboxed_error(name: &str) -> String {
    format!("'{name}' is a DLL function, and this run is sandboxed (RAPIDR_SANDBOX): it calls no DLL; run the program yourself to let it")
}

/// A library of macOS's or Linux's own format, named as such (`LIB
/// "libfoo.dylib"`, `"libm.so.6"`): RapidR's addition, loaded on those
/// systems. Anything else a DECLARE names is a Windows DLL.
pub fn is_unix_library(lib: &str) -> bool {
    let name = lib.trim_matches('"').rsplit(['\\', '/']).next().unwrap_or(lib).to_ascii_lowercase();
    name.ends_with(".dylib") || name.ends_with(".so") || name.contains(".so.")
}

/// The error for a call into a Windows DLL on a system that isn't Windows
/// (`web` for the browser, which can't load any DLL).
pub fn needs_windows_error(lib: &str, name: &str, web: bool) -> String {
    let base = library_base(lib);
    let hint = windows_api_hint(name).map(|h| format!(". For every system, {h}")).unwrap_or_default();
    if web {
        format!("'{name}' is a function of a DLL ({base}): this program calls Windows itself, and the web can't load DLLs, so it runs on Windows only{hint}")
    } else if is_windows_system_library(lib) {
        format!("'{name}' is a Windows function ({base}): this program calls Windows itself, so it runs on Windows only{hint}")
    } else {
        format!("'{name}' is a function of {base}.dll, a Windows DLL: this program calls a DLL, so it runs on Windows only{hint}")
    }
}

/// A DLL routine's calling spec, what the compilers write into the call:
/// `RET|TYPE:v,TYPE:r,…` — the declared result type (empty for a SUB) and
/// each parameter's declared type with `v` (by value) or `r` (BYREF).
pub fn spec_of(params: &[(String, bool)], return_type: Option<&str>) -> String {
    let ps: Vec<String> = params
        .iter()
        .map(|(t, by_ref)| format!("{}:{}", normal_type(t), if *by_ref { 'r' } else { 'v' }))
        .collect();
    format!("{}|{}", return_type.map(normal_type).unwrap_or_default(), ps.join(","))
}

fn normal_type(t: &str) -> String {
    t.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_uppercase()
}

/// A parameter of a parsed spec.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    /// The declared type, upper case (`LONG`, `STRING`, `STRING*260`, a
    /// TYPE's name).
    pub type_name: String,
    pub by_ref: bool,
}

impl Param {
    /// A STRING of any kind: the DLL gets the characters' address.
    pub fn is_string(&self) -> bool {
        self.type_name == "STRING" || self.type_name.starts_with("STRING*")
    }

    /// A floating-point value passed by value.
    pub fn is_float(&self) -> bool {
        !self.by_ref && matches!(self.type_name.as_str(), "DOUBLE" | "SINGLE" | "CURRENCY")
    }

    /// A number of RapidQ's (passed by value unless BYREF).
    pub fn is_numeric(&self) -> bool {
        matches!(
            self.type_name.as_str(),
            "" | "BYTE" | "WORD" | "SHORT" | "INTEGER" | "LONG" | "DWORD" | "SINGLE" | "DOUBLE" | "CURRENCY" | "INT64" | "VARIANT" | "BOOLEAN" | "BOOL"
        )
    }

    /// A TYPE (or an object): always passed by address.
    pub fn is_udt(&self) -> bool {
        !self.is_string() && !self.is_numeric()
    }
}

/// A parsed spec.
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub return_type: String,
    pub params: Vec<Param>,
}

impl Spec {
    pub fn parse(spec: &str) -> Spec {
        let (ret, ps) = spec.split_once('|').unwrap_or((spec, ""));
        let params = ps
            .split(',')
            .filter(|p| !p.is_empty())
            .map(|p| {
                let (t, m) = p.rsplit_once(':').unwrap_or((p, "v"));
                Param { type_name: t.to_string(), by_ref: m == "r" }
            })
            .collect();
        Spec { return_type: ret.to_string(), params }
    }

    /// The result is a floating-point value.
    pub fn returns_float(&self) -> bool {
        matches!(self.return_type.as_str(), "DOUBLE" | "SINGLE" | "CURRENCY")
    }
}

/// The first 32-bit stand-in for a 64-bit pointer a DLL returned where the
/// DECLARE says LONG (`rapidr_runtime_core::ffi`: it turns back into the
/// pointer when the program hands it to a DLL again). 0xD1E00000
/// (-773849088): a negative LONG with a bit pattern no flag combination of
/// Windows' makes (0xC0000000 would be `GENERIC_READ OR GENERIC_WRITE`),
/// outside the program's own addresses (`memory`: 1 MB to 2 GB).
pub const POINTER_STAND_IN_BASE: i64 = 0xD1E0_0000u32 as i32 as i64;
/// How many stand-ins there can be.
pub const POINTER_STAND_INS: usize = 1 << 16;

/// Whether `v` is in the stand-ins' range.
pub fn is_pointer_stand_in(v: i64) -> bool {
    (POINTER_STAND_IN_BASE..POINTER_STAND_IN_BASE + POINTER_STAND_INS as i64).contains(&v)
}

/// The size in bytes of a numeric declared type (a LONG for anything else).
pub fn numeric_size(type_name: &str) -> usize {
    match type_name {
        "BYTE" => 1,
        "WORD" | "SHORT" => 2,
        "DOUBLE" | "INT64" | "CURRENCY" => 8,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libraries_and_hints() {
        assert!(is_windows_system_library("USER32.DLL"));
        assert!(is_windows_system_library("c:\\windows\\system32\\kernel32.dll"));
        assert!(!is_windows_system_library("mylib.dll"));
        assert_eq!(windows_api_hint("ShellExecuteA"), Some("run programs and open files with SHELL / SHELLWAIT"));
        assert_eq!(windows_api_hint("GetDC"), Some("draw with an RCANVAS (Line, Circle, Rectangle, TextOut, …)"));
        assert_eq!(windows_api_hint("LoadCursorFromFileA"), None);
        let e = needs_windows_error("user32", "GetDC", false);
        assert!(e.starts_with("'GetDC' is a Windows function (user32): this program calls Windows itself, so it runs on Windows only. For every system, draw"), "{e}");
        let e = needs_windows_error("FreeImage.dll", "FreeImage_Load", false);
        assert_eq!(e, "'FreeImage_Load' is a function of freeimage.dll, a Windows DLL: this program calls a DLL, so it runs on Windows only");
        assert!(is_unix_library("libSystem.B.dylib") && is_unix_library("/usr/lib/libm.so.6") && is_unix_library("libz.so"));
        assert!(!is_unix_library("zlib") && !is_unix_library("mylib.dll") && !is_unix_library("user32"));
    }

    #[test]
    fn specs_round_trip() {
        let s = spec_of(&[("String".into(), false), ("Long".into(), true), ("TRect".into(), false)], Some("Long"));
        assert_eq!(s, "LONG|STRING:v,LONG:r,TRECT:v");
        let p = Spec::parse(&s);
        assert_eq!(p.return_type, "LONG");
        assert!(p.params[0].is_string() && !p.params[0].by_ref);
        assert!(p.params[1].by_ref && p.params[1].is_numeric());
        assert!(p.params[2].is_udt());
        assert_eq!(Spec::parse("|").params.len(), 0);
    }
}
