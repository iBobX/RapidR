//! The zip-bomb guard: every unpacking reads at most a cap of bytes — the
//! bytes really produced, never the size an archive claims — and an error
//! says which limit it met, in plain words.

use std::io::Read;

use crate::Limits;

/// Below this many unpacked bytes the ratio isn't checked (a small file of
/// one repeated byte packs better than 1000 to 1, harmlessly).
pub const RATIO_FLOOR: u64 = 1 << 20;

/// The error [`read_capped`] gives when there was more than the cap.
pub(crate) const OVER: &str = "\u{0}over the cap";

/// The most `packed` bytes may unpack to: MaxSize, and MaxRatio times them
/// (at least [`RATIO_FLOOR`]).
pub(crate) fn cap(packed: u64, limits: &Limits) -> u64 {
    ratio_cap(packed, limits).min(limits.max_size)
}

fn ratio_cap(packed: u64, limits: &Limits) -> u64 {
    if limits.max_ratio == 0 {
        u64::MAX
    } else {
        packed.max(1).saturating_mul(limits.max_ratio).max(RATIO_FLOOR)
    }
}

/// All of `r`, at most `limit` bytes; more is [`OVER`] (the reading stops
/// at the cap: memory grows only with real data).
pub(crate) fn read_capped(r: &mut dyn Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    r.take(limit.saturating_add(1)).read_to_end(&mut out).map_err(|e| e.to_string())?;
    if out.len() as u64 > limit {
        return Err(OVER.into());
    }
    Ok(out)
}

/// `err` from unpacking `what` (`"game.z80"`, `the gzip data`) of `packed`
/// bytes, in plain words: which limit it met, or that it's damaged.
pub(crate) fn explain(err: String, what: &str, packed: u64, limits: &Limits) -> String {
    if err != OVER {
        return damaged(what, &err);
    }
    if ratio_cap(packed, limits) < limits.max_size {
        format!("{what} unpacks to more than MaxRatio ({}) times its packed size: not unpacked — a zip bomb, or set MaxRatio higher", limits.max_ratio)
    } else {
        too_big(what, limits.max_size)
    }
}

/// `what` unpacks past MaxSize.
pub(crate) fn too_big(what: &str, max: u64) -> String {
    let what = if what.starts_with('"') || what.starts_with("the ") { what.to_string() } else { format!("\"{what}\"") };
    format!("{what} unpacks to more than MaxSize ({max} bytes): not unpacked — a zip bomb, or set MaxSize higher")
}

/// `what` couldn't be unpacked: `err`, the library's words, after ours.
pub(crate) fn damaged(what: &str, err: &str) -> String {
    let err = err.trim();
    if err.is_empty() {
        format!("{what} is damaged and can't be unpacked")
    } else {
        format!("{what} is damaged and can't be unpacked ({err})")
    }
}
