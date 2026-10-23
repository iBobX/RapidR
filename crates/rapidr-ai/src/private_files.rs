//! Files only their user reads (SEC-28): the assistant's kept chats and the
//! audit log are made 0600, their folders 0700, on Unix — another account
//! on the computer can't read them. (Windows' profile folders are the
//! user's own already.)

use std::io::Write;
use std::path::Path;

/// Makes `dir` and the folders above it that are missing (new ones only
/// the user's); `own` — a folder of RapidR's own, made only the user's even
/// when it was there.
pub fn make_dir(dir: &Path, own: bool) -> std::io::Result<()> {
    let mut b = std::fs::DirBuilder::new();
    b.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        b.mode(0o700);
    }
    b.create(dir)?;
    if own {
        only_user(dir, 0o700);
    }
    Ok(())
}

/// Writes `text` to a new (or emptied) file `path` only the user can read.
pub fn write(path: &Path, text: &str) -> std::io::Result<()> {
    let mut f = open(path, false)?;
    f.write_all(text.as_bytes())
}

/// Opens `path` to add to (made if missing), only the user's.
pub fn append(path: &Path) -> std::io::Result<std::fs::File> {
    open(path, true)
}

fn open(path: &Path, append: bool) -> std::io::Result<std::fs::File> {
    let mut o = std::fs::OpenOptions::new();
    o.create(true);
    if append {
        o.append(true);
    } else {
        o.write(true).truncate(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    let f = o.open(path)?;
    // (one made before, with the usual permissions: made the user's)
    only_user(path, 0o600);
    Ok(f)
}

/// `path`'s permissions `mode` when others could read it (Unix).
fn only_user(path: &Path, mode: u32) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(m) = std::fs::metadata(path) {
            if m.permissions().mode() & 0o077 != 0 {
                let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
            }
        }
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
}
