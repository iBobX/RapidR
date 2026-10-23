//! Windows: a native program is linked *with* its version information and
//! manifest, not given them afterwards.
//!
//! `package::finish` stamps the icon and version into the finished `.exe`
//! (rapidr-package, windows.rs) — but the antivirus sees the program the
//! moment the linker closes it, long before that. A program with no
//! resources at all, stripped, is what a heuristic antivirus likes least: the
//! 2.121.0 release test on Windows x64 saw one refuse (read denied, error 5,
//! held by the antivirus' scanner for good) every one of the six small native
//! test programs, and accept the same program once it carried any resource —
//! version information, a manifest, or both. So the program is linked with
//! them: RapidR writes a resource script, compiles it with the `llvm-windres`
//! the SDK ships (the LLVM-MinGW toolchain) into an object file, and cargo
//! hands that object to the linker for the program's own executable only
//! (`cargo rustc … -- -C link-arg=<object>`: the runtime's crates stay built,
//! nothing of them changes). The stamp afterwards writes the same details and
//! the icon over them.
//!
//! Best effort: no `llvm-windres` (the msvc toolchain, a runtime-only
//! install), or it fails — the build goes on without, as before.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rapidr_package::AppInfo;

/// The object made for one build, in its own folder of the build cache.
pub struct LinkResource {
    dir: PathBuf,
    pub object: PathBuf,
}

impl LinkResource {
    /// Its folder removed (the object is only needed while the program is linked).
    pub fn remove(&self) {
        let _ = fs::remove_dir_all(&self.dir);
    }

    /// What follows `--` in `cargo rustc`: the object handed to the linker.
    pub fn rustc_args(&self) -> Vec<String> {
        rustc_args(&self.object)
    }
}

fn rustc_args(object: &Path) -> Vec<String> {
    vec!["--".into(), "-C".into(), format!("link-arg={}", object.display())]
}

/// The object file for `info`'s program, made in `<cache_dir>/res/`; None
/// when there is nothing to do or it can't be done (see the module).
pub fn make(info: &AppInfo, cache_dir: &Path) -> Option<LinkResource> {
    if !cfg!(windows) {
        return None;
    }
    let tool = crate::home::Home::find()?.windows_toolchain()?.join("bin").join("llvm-windres.exe");
    if !tool.is_file() {
        return None;
    }
    let dir = cache_dir.join("res").join(std::process::id().to_string());
    let made = (|| -> Result<PathBuf, String> {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        fs::write(dir.join("program.manifest"), rapidr_package::windows::LINK_MANIFEST).map_err(|e| e.to_string())?;
        fs::write(dir.join("program.rc"), rapidr_package::windows::link_script(info, "program.manifest")?).map_err(|e| e.to_string())?;
        let status = Command::new(&tool)
            // (this machine's architecture: the SDK's toolchain links for it)
            .arg(format!("--target={}-w64-mingw32", std::env::consts::ARCH))
            .args(["-i", "program.rc", "-o", "program.o"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("llvm-windres: {status}"));
        }
        let object = dir.join("program.o");
        object.is_file().then_some(object).ok_or_else(|| "llvm-windres made no object".to_string())
    })();
    match made {
        Ok(object) => Some(LinkResource { dir, object }),
        Err(_) => {
            let _ = fs::remove_dir_all(&dir);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_object_is_handed_to_the_linker_for_the_programs_executable_only() {
        let args = rustc_args(Path::new(r"C:\Users\Jo Smith\AppData\Local\RapidR\Cache\build\k\res\42\program.o"));
        // (after cargo's own arguments: `--`, then rustc's — one argument, spaces and all)
        assert_eq!(args, ["--", "-C", r"link-arg=C:\Users\Jo Smith\AppData\Local\RapidR\Cache\build\k\res\42\program.o"]);
    }

    /// Not Windows: nothing is made, nothing changes.
    #[cfg(not(windows))]
    #[test]
    fn elsewhere_there_is_no_resource() {
        let info = AppInfo::new("Notepad", "notepad");
        assert!(make(&info, &std::env::temp_dir()).is_none());
    }
}
