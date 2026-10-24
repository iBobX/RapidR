// The Windows 7 builds (`*-win7-windows-gnu`, tools/win7/build.sh):
//
// 1. (nothing to link: src/win7.rs answers for the six functions only
//    Windows 8 and later have, so that Windows 7 starts the program. No
//    delay-loading, no helper from any toolchain);
// 2. link the program with its version information and a manifest. A
//    program with no resources at all is what a heuristic antivirus likes
//    least (the same finding as native programs on Windows x64:
//    crates/rapidr-cli/src/winres.rs), so rapidr.exe says who made it.
//
// Every other target: nothing to do.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The same manifest as rapidr_package::windows::LINK_MANIFEST: no
/// administrator rights, nothing else.
const MANIFEST: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n",
    "<assembly xmlns=\"urn:schemas-microsoft-com:asm.v1\" manifestVersion=\"1.0\">\n",
    "  <trustInfo xmlns=\"urn:schemas-microsoft-com:asm.v3\">\n",
    "    <security><requestedPrivileges><requestedExecutionLevel level=\"asInvoker\" uiAccess=\"false\"/></requestedPrivileges></security>\n",
    "  </trustInfo>\n",
    "</assembly>\n",
);

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=WINDRES");
    println!("cargo:rerun-if-env-changed=RAPIDR_WIN7_DESCRIPTION");
    let var = |k: &str| std::env::var(k).unwrap_or_default();
    if var("CARGO_CFG_TARGET_OS") == "windows" && var("CARGO_CFG_TARGET_VENDOR") == "win7" && var("CARGO_CFG_TARGET_ENV") == "gnu" {
        let arch = if var("CARGO_CFG_TARGET_ARCH") == "x86" { "i686" } else { "x86_64" };
        match version_resource(arch, &var("CARGO_PKG_VERSION"), &var("OUT_DIR")) {
            Ok(object) => println!("cargo:rustc-link-arg-bins={}", object.display()),
            Err(e) => println!("cargo:warning=rapidr.exe has no version information ({e}): set WINDRES or put {arch}-w64-mingw32-windres on PATH"),
        }
    }
}

/// The resource object: the version information and manifest, compiled by
/// windres (LLVM-MinGW's, the toolchain that links the program).
fn version_resource(arch: &str, version: &str, out_dir: &str) -> Result<PathBuf, String> {
    let dir = Path::new(out_dir);
    let mut n = version.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).map(|s| s.parse::<u32>().unwrap_or(0));
    let v = [n.next().unwrap_or(0), n.next().unwrap_or(0), n.next().unwrap_or(0), n.next().unwrap_or(0)];
    let description = std::env::var("RAPIDR_WIN7_DESCRIPTION").unwrap_or_else(|_| "RapidR (Windows 7 alpha)".into());
    let rc = format!(
        "1 24 \"program.manifest\"\n1 VERSIONINFO\nFILEVERSION {a},{b},{c},{d}\nPRODUCTVERSION {a},{b},{c},{d}\nFILEOS 0x40004\nFILETYPE 0x1\n{{\n  BLOCK \"StringFileInfo\"\n  {{\n    BLOCK \"040904B0\"\n    {{\n\
      VALUE \"CompanyName\", \"Ruta Internet S.R.L.\"\n\
      VALUE \"FileDescription\", \"{description}\"\n\
      VALUE \"FileVersion\", \"{a}.{b}.{c}.{d}\"\n\
      VALUE \"InternalName\", \"rapidr\"\n\
      VALUE \"LegalCopyright\", \"Copyright (c) 2025-2026 Ruta Internet SRL. MIT License.\"\n\
      VALUE \"OriginalFilename\", \"rapidr.exe\"\n\
      VALUE \"ProductName\", \"RapidR\"\n\
      VALUE \"ProductVersion\", \"{version}\"\n\
    }}\n  }}\n  BLOCK \"VarFileInfo\"\n  {{\n    VALUE \"Translation\", 0x409, 1200\n  }}\n}}\n",
        a = v[0],
        b = v[1],
        c = v[2],
        d = v[3],
    );
    std::fs::write(dir.join("program.manifest"), MANIFEST).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("program.rc"), rc).map_err(|e| e.to_string())?;
    let tool = std::env::var("WINDRES").unwrap_or_else(|_| format!("{arch}-w64-mingw32-windres"));
    let object = dir.join("program.o");
    let _ = std::fs::remove_file(&object);
    let status = Command::new(&tool)
        .args(["-i", "program.rc", "-o", "program.o"])
        .current_dir(dir)
        .stdin(Stdio::null())
        .status()
        .map_err(|e| format!("{tool}: {e}"))?;
    if !status.success() {
        return Err(format!("{tool}: {status}"));
    }
    if object.is_file() {
        Ok(object)
    } else {
        Err(format!("{tool} made no object"))
    }
}
