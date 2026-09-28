//! A program's `$RESOURCE`s (RapidQ manual, chapter 8): files built into
//! the program. The preprocessor gives each one a handle,
//! `RESOURCE_BASE + i` for the `i`-th (`rapidr_preprocessor::RESOURCE_BASE`),
//! and defines the resource's name as that handle; at startup the program
//! registers the bytes here — a native build from `include_bytes!`, the
//! interpreter from its bytecode's resource section.
//!
//! Anything that takes a file also takes a resource as the pseudo path
//! `rapidr-resource:<handle>` ([`path`]): `BMPHandle = GRID_BMP` loads the
//! image through that path, so both runtimes need nothing else.

use std::sync::{Arc, RwLock};

/// Handle of the first resource (the same as the preprocessor's).
pub const RESOURCE_BASE: i64 = 65536;

/// The pseudo path a resource is read through.
pub const PATH_PREFIX: &str = "rapidr-resource:";

static RESOURCES: RwLock<Vec<(String, Arc<[u8]>)>> = RwLock::new(Vec::new());

/// Adds the next resource (in `$RESOURCE` order).
pub fn register(name: &str, bytes: impl Into<Arc<[u8]>>) {
    if let Ok(mut r) = RESOURCES.write() {
        r.push((name.to_string(), bytes.into()));
    }
}

/// The resources of the program about to run, replacing any earlier
/// program's (the web IDE runs several in one page).
pub fn set_all(list: &[(String, Vec<u8>)]) {
    if let Ok(mut r) = RESOURCES.write() {
        *r = list.iter().map(|(n, b)| (n.clone(), Arc::from(b.as_slice()))).collect();
    }
}

/// `RESOURCECOUNT`.
pub fn count() -> i64 {
    RESOURCES.read().map_or(0, |r| r.len() as i64)
}

/// `RESOURCE(n)`: the handle of the `n`-th resource, 0 if there's none.
pub fn handle(n: i64) -> i64 {
    if (0..count()).contains(&n) {
        RESOURCE_BASE + n
    } else {
        0
    }
}

/// The bytes of the resource with this handle.
pub fn bytes(handle: i64) -> Option<Arc<[u8]>> {
    let i = usize::try_from(handle.checked_sub(RESOURCE_BASE)?).ok()?;
    RESOURCES.read().ok()?.get(i).map(|(_, b)| b.clone())
}

/// The pseudo path of a handle (`rapidr-resource:65536`), or `None` when no
/// resource has it.
pub fn path(handle: i64) -> Option<String> {
    bytes(handle).map(|_| format!("{PATH_PREFIX}{handle}"))
}

/// The bytes behind a path, if it's a resource's pseudo path.
pub fn read_path(path: &str) -> Option<Result<Vec<u8>, String>> {
    let h = path.strip_prefix(PATH_PREFIX)?;
    Some(h.parse::<i64>().ok().and_then(bytes).map(|b| b.to_vec()).ok_or_else(|| format!("no resource {h}")))
}

/// `RESOURCE(n)`.
pub fn rp_resource(n: &crate::Value) -> crate::Value {
    crate::v_int(handle(n.to_i64()))
}

/// `RESOURCECOUNT`.
pub fn rp_resourcecount() -> crate::Value {
    crate::v_int(count())
}

/// `EXTRACTRESOURCE handle, file`: writes the resource's bytes to a file
/// (through the runtime's file hooks).
pub fn rp_extractresource(handle: &crate::Value, file: &crate::Value) {
    let Some(b) = bytes(handle.to_i64()) else {
        eprintln!("[rapidr] EXTRACTRESOURCE: no resource {}", handle.to_i64());
        return;
    };
    if let Err(e) = crate::objects::write_file(&file.to_string_val(), &b) {
        eprintln!("[rapidr] EXTRACTRESOURCE: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_and_paths() {
        register("A", vec![1u8, 2, 3]);
        assert!(count() >= 1);
        assert_eq!(handle(0), RESOURCE_BASE);
        assert_eq!(handle(-1), 0);
        assert_eq!(read_path(&path(RESOURCE_BASE).unwrap()).unwrap().unwrap(), [1, 2, 3]);
        assert!(read_path("rapidr-resource:5").unwrap().is_err());
        assert!(read_path("grid.bmp").is_none());
    }
}
