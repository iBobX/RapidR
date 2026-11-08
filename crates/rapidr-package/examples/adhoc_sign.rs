//! Signs a Mach-O program or a `.app` folder ad hoc with RapidR's own
//! signer (`rapidr_package::macho_sign`), for checking it by hand:
//!
//! ```sh
//! cargo run -p rapidr-package --example adhoc_sign -- program signed-program [identifier]
//! cargo run -p rapidr-package --example adhoc_sign -- Name.app
//! ```
//!
//! A program's identifier is the one `codesign` would give it when none is
//! given. An app is signed in place: its program and
//! `Contents/_CodeSignature/CodeResources` are written back. Nothing here
//! runs Apple's tools.

use std::path::Path;
use std::process::ExitCode;

use rapidr_package::macho_sign;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [app] if app.trim_end_matches('/').ends_with(".app") => sign_app_folder(Path::new(app)),
        [input, output, rest @ ..] if rest.len() <= 1 => sign_program(input, output, rest.first()),
        _ => Err("usage: adhoc_sign <program> <signed-program> [identifier] | adhoc_sign <Name.app>".to_string()),
    };
    match result {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn sign_program(input: &str, output: &str, identifier: Option<&String>) -> Result<String, String> {
    let bytes = std::fs::read(input).map_err(|e| format!("{input}: {e}"))?;
    let name = Path::new(output).file_name().and_then(|n| n.to_str()).unwrap_or(output);
    let identifier = identifier.cloned().unwrap_or_else(|| macho_sign::default_identifier(name, &bytes));
    let started = std::time::Instant::now();
    let signed = macho_sign::sign(&bytes, &identifier, Default::default())?;
    let took = started.elapsed();
    std::fs::write(output, &signed).map_err(|e| format!("{output}: {e}"))?;
    make_executable(Path::new(output))?;
    let slices = macho_sign::check(&signed)?;
    let cdhashes: Vec<String> = slices.iter().map(|s| hex(&s.cd_hash[..20])).collect();
    Ok(format!("signed {output} as {identifier} in {} ms ({} slice(s), CDHash {})", took.as_millis(), slices.len(), cdhashes.join(", ")))
}

fn sign_app_folder(app: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    read_tree(app, app, &mut files)?;
    let before = files.clone();
    let started = std::time::Instant::now();
    let main = macho_sign::sign_app(&mut files)?;
    let took = started.elapsed();
    for (name, bytes) in &files {
        if !before.iter().any(|(n, b)| n == name && b == bytes) {
            let at = app.join(name);
            if let Some(dir) = at.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            }
            // (the program keeps its 0755: written over, not made again)
            std::fs::write(&at, bytes).map_err(|e| format!("{}: {e}", at.display()))?;
        }
    }
    for (name, _) in before.iter().filter(|(n, _)| !files.iter().any(|(m, _)| m == n)) {
        std::fs::remove_file(app.join(name)).map_err(|e| format!("{name}: {e}"))?;
    }
    let on_disk = std::fs::read(app.join(&main)).map_err(|e| format!("{main}: {e}"))?;
    let slices = macho_sign::check(&on_disk)?;
    Ok(format!("signed {} in {} ms ({main}: {} slice(s) as {})", app.display(), took.as_millis(), slices.len(), slices[0].identifier))
}

/// Every file under `dir`, by its `/`-separated path below `root`; links refused.
fn read_tree(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), String> {
    let mut entries: Vec<_> = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| format!("{}: {e}", path.display()))?;
        if kind.is_dir() {
            read_tree(root, &path, out)?;
        } else if kind.is_file() {
            let rel = path.strip_prefix(root).unwrap().to_str().ok_or_else(|| format!("{}: not UTF-8", path.display()))?.replace('\\', "/");
            out.push((rel, std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?));
        } else {
            return Err(format!("{} is a link: an app RapidR signs holds files and folders only", path.display()));
        }
    }
    Ok(())
}

fn make_executable(_p: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(_p, std::fs::Permissions::from_mode(0o755)).map_err(|e| format!("{}: {e}", _p.display()))?;
    }
    Ok(())
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
