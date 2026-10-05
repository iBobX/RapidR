//! macOS executables are universal (arm64 + x86_64): RapidR ships one runner
//! for both (`runners/macos/`), native builds make both slices when Rust has
//! both targets, and `--target macos-arm64` / `macos-x86_64` takes one slice
//! out of the universal runner. Apple silicon runs the arm64 slice, Intel
//! Macs (up to macOS 26) the x86_64 one — nothing Intel-only is needed where
//! Rosetta is going away (macOS 28).

use std::path::Path;
use std::process::Command;

/// The oldest macOS RapidR's executables run on: `MACOSX_DEPLOYMENT_TARGET`
/// for both slices (Rust raises it to 11.0 for arm64, the first macOS there
/// was for Apple silicon; 10.13 is what winit and Metal need on Intel).
pub const DEPLOYMENT_TARGET: &str = "10.13";

/// The two Rust targets of a universal executable.
pub const TRIPLES: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];

/// Mach-O CPU types (`<mach/machine.h>`).
const CPU_TYPE_X86_64: u32 = 0x0100_0007;
const CPU_TYPE_ARM64: u32 = 0x0100_000C;

/// The CPU type a thin `--target` asks for: `macos-arm64`, `macos-x86_64`.
pub fn slice_of(target: &str) -> Option<u32> {
    match target {
        "macos-arm64" => Some(CPU_TYPE_ARM64),
        "macos-x86_64" => Some(CPU_TYPE_X86_64),
        _ => None,
    }
}

/// The slice for `cpu` out of a universal (fat) Mach-O.
pub fn thin(fat: &[u8], cpu: u32) -> Result<Vec<u8>, String> {
    let be32 = |at: usize| fat.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    let be64 = |at: usize| fat.get(at..at + 8).map(|b| u64::from_be_bytes(b.try_into().unwrap()));
    let wide = match be32(0) {
        Some(0xCAFE_BABE) => false,
        Some(0xCAFE_BABF) => true,
        _ => return Err("not a universal (fat) Mach-O".into()),
    };
    let count = be32(4).ok_or("truncated Mach-O")? as usize;
    let entry = if wide { 32 } else { 20 };
    for i in 0..count {
        let at = 8 + i * entry;
        let cputype = be32(at).ok_or("truncated Mach-O")?;
        let (offset, size) = if wide {
            (be64(at + 8).ok_or("truncated Mach-O")?, be64(at + 16).ok_or("truncated Mach-O")?)
        } else {
            (u64::from(be32(at + 8).ok_or("truncated Mach-O")?), u64::from(be32(at + 12).ok_or("truncated Mach-O")?))
        };
        if cputype == cpu {
            let (start, end) = (offset as usize, (offset + size) as usize);
            return fat.get(start..end).map(<[u8]>::to_vec).ok_or_else(|| "a slice outside the file".into());
        }
    }
    Err("the universal executable has no slice for that CPU".into())
}

/// Rust has both macOS targets (`rustup target add` both): a native build
/// makes a universal executable.
pub fn rust_has_both_targets(rustc: &Path) -> bool {
    let sysroot = Command::new(rustc).args(["--print", "sysroot"]).output().ok().filter(|o| o.status.success());
    let Some(sysroot) = sysroot.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()) else { return false };
    TRIPLES.iter().all(|t| Path::new(&sysroot).join("lib/rustlib").join(t).join("lib").is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fat(slices: &[(u32, &[u8])]) -> Vec<u8> {
        let mut out = vec![0xCA, 0xFE, 0xBA, 0xBE];
        out.extend_from_slice(&(slices.len() as u32).to_be_bytes());
        let mut offset = 8 + 20 * slices.len();
        let mut body = Vec::new();
        for (cpu, data) in slices {
            out.extend_from_slice(&cpu.to_be_bytes());
            out.extend_from_slice(&0u32.to_be_bytes());
            out.extend_from_slice(&(offset as u32).to_be_bytes());
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            out.extend_from_slice(&0u32.to_be_bytes());
            offset += data.len();
            body.extend_from_slice(data);
        }
        out.extend(body);
        out
    }

    #[test]
    fn thins_a_universal_executable() {
        let f = fat(&[(CPU_TYPE_X86_64, b"intel"), (CPU_TYPE_ARM64, b"apple silicon")]);
        assert_eq!(thin(&f, CPU_TYPE_ARM64).unwrap(), b"apple silicon");
        assert_eq!(thin(&f, CPU_TYPE_X86_64).unwrap(), b"intel");
        assert!(thin(b"\xcf\xfa\xed\xfe thin", CPU_TYPE_ARM64).is_err());
        assert_eq!(slice_of("macos-arm64"), Some(CPU_TYPE_ARM64));
        assert_eq!(slice_of("macos"), None);
    }
}
