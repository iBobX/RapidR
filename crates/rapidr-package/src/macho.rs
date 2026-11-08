//! Mach-O executables (macOS), as far as packing needs them: which
//! processors one is for, one slice taken out of a universal ("fat")
//! executable, and a universal one made from thin slices — what `lipo`
//! does, in pure Rust from the format in Apple's `<mach-o/fat.h>` and
//! `<mach-o/loader.h>`, so a build makes it on any system.

/// Mach-O CPU types (`<mach/machine.h>`).
pub const CPU_TYPE_X86_64: u32 = 0x0100_0007;
pub const CPU_TYPE_ARM64: u32 = 0x0100_000C;

const FAT_MAGIC: u32 = 0xCAFE_BABE;
const FAT_MAGIC_64: u32 = 0xCAFE_BABF;
const MH_MAGIC_64: u32 = 0xFEED_FACF;

/// A slice's alignment in a universal file: 2^14 (16 KiB, Apple silicon's
/// page), as Apple's tools align arm64 and x86_64 slices.
const ALIGN_LOG2: u32 = 14;

/// The processor's name for messages.
pub fn cpu_name(cpu: u32) -> &'static str {
    match cpu {
        CPU_TYPE_ARM64 => "Apple silicon (arm64)",
        CPU_TYPE_X86_64 => "Intel (x86_64)",
        _ => "another processor",
    }
}

/// A thin 64-bit Mach-O's CPU type (little-endian, as both Macs' are).
fn thin_cpu(b: &[u8]) -> Option<u32> {
    let magic = u32::from_le_bytes(b.get(0..4)?.try_into().ok()?);
    if magic != MH_MAGIC_64 {
        return None;
    }
    Some(u32::from_le_bytes(b.get(4..8)?.try_into().ok()?))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn be64(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// A universal file's slices: (cpu type, offset, size).
fn fat_slices(b: &[u8]) -> Result<Option<Vec<(u32, usize, usize)>>, String> {
    let wide = match be32(b, 0) {
        Some(FAT_MAGIC) => false,
        Some(FAT_MAGIC_64) => true,
        _ => return Ok(None),
    };
    let short = || "a damaged universal Mach-O (cut short)".to_string();
    let count = be32(b, 4).ok_or_else(short)? as usize;
    // (Java class files start CAFEBABE too: a count that can't be is not a fat file)
    if count == 0 || count > 16 {
        return Err("not a Mach-O executable RapidR knows (a universal header with no slices)".into());
    }
    let entry = if wide { 32 } else { 20 };
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let at = 8 + i * entry;
        let cpu = be32(b, at).ok_or_else(short)?;
        let (offset, size) = if wide {
            (be64(b, at + 8).ok_or_else(short)?, be64(b, at + 16).ok_or_else(short)?)
        } else {
            (u64::from(be32(b, at + 8).ok_or_else(short)?), u64::from(be32(b, at + 12).ok_or_else(short)?))
        };
        let end = offset.checked_add(size).filter(|&e| e <= b.len() as u64).ok_or("a damaged universal Mach-O (a slice past its end)")?;
        out.push((cpu, offset as usize, (end - offset) as usize));
    }
    Ok(Some(out))
}

/// The processors an executable runs on (one for a thin Mach-O, each
/// slice's for a universal one); an error for what isn't a Mach-O.
pub fn cpus(b: &[u8]) -> Result<Vec<u32>, String> {
    if let Some(slices) = fat_slices(b)? {
        return Ok(slices.iter().map(|s| s.0).collect());
    }
    thin_cpu(b).map(|c| vec![c]).ok_or_else(|| "not a macOS executable (Mach-O)".into())
}

/// The slice for `cpu`: out of a universal executable, or a thin one for
/// that processor as it is.
pub fn thin(b: &[u8], cpu: u32) -> Result<Vec<u8>, String> {
    match fat_slices(b)? {
        Some(slices) => slices
            .iter()
            .find(|s| s.0 == cpu)
            .map(|&(_, at, size)| b[at..at + size].to_vec())
            .ok_or_else(|| format!("the universal executable has no slice for {}", cpu_name(cpu))),
        None => match thin_cpu(b) {
            Some(c) if c == cpu => Ok(b.to_vec()),
            Some(c) => Err(format!("the executable is for {}, not {}", cpu_name(c), cpu_name(cpu))),
            None => Err("not a macOS executable (Mach-O)".into()),
        },
    }
}

/// One universal executable from thin 64-bit slices (each a different
/// processor): the fat header, then each slice at a 16 KiB boundary.
pub fn universal(slices: &[&[u8]]) -> Result<Vec<u8>, String> {
    if slices.is_empty() {
        return Err("a universal executable needs at least one slice".into());
    }
    let mut cpus = Vec::new();
    for s in slices {
        let cpu = thin_cpu(s).ok_or("a slice that isn't a thin 64-bit macOS executable")?;
        if cpus.contains(&cpu) {
            return Err(format!("two slices for {}", cpu_name(cpu)));
        }
        cpus.push(cpu);
    }
    let align = 1usize << ALIGN_LOG2;
    let up = |n: usize| n.div_ceil(align) * align;
    let mut out = Vec::new();
    out.extend_from_slice(&FAT_MAGIC.to_be_bytes());
    out.extend_from_slice(&(slices.len() as u32).to_be_bytes());
    let mut offset = up(8 + 20 * slices.len());
    let mut places = Vec::new();
    for (s, &cpu) in slices.iter().zip(&cpus) {
        let sub = u32::from_le_bytes(s[8..12].try_into().unwrap_or([0; 4]));
        let off32 = u32::try_from(offset).map_err(|_| "the universal executable is over 4 GB")?;
        let len32 = u32::try_from(s.len()).map_err(|_| "a slice is over 4 GB")?;
        out.extend_from_slice(&cpu.to_be_bytes());
        out.extend_from_slice(&sub.to_be_bytes());
        out.extend_from_slice(&off32.to_be_bytes());
        out.extend_from_slice(&len32.to_be_bytes());
        out.extend_from_slice(&ALIGN_LOG2.to_be_bytes());
        places.push(offset);
        offset = up(offset + s.len());
    }
    for (s, at) in slices.iter().zip(places) {
        out.resize(at, 0);
        out.extend_from_slice(s);
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A thin Mach-O header's first bytes for `cpu`, then `body`.
    pub(crate) fn thin_macho(cpu: u32, body: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&MH_MAGIC_64.to_le_bytes());
        b.extend_from_slice(&cpu.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&2u32.to_le_bytes()); // MH_EXECUTE
        b.extend_from_slice(&[0; 16]);
        b.extend_from_slice(body);
        b
    }

    #[test]
    fn universal_and_back() {
        let arm = thin_macho(CPU_TYPE_ARM64, b"apple silicon");
        let intel = thin_macho(CPU_TYPE_X86_64, b"intel");
        let fat = universal(&[&arm, &intel]).unwrap();
        assert_eq!(cpus(&fat).unwrap(), [CPU_TYPE_ARM64, CPU_TYPE_X86_64]);
        assert_eq!(thin(&fat, CPU_TYPE_ARM64).unwrap(), arm);
        assert_eq!(thin(&fat, CPU_TYPE_X86_64).unwrap(), intel);
        // (each slice on a 16 KiB boundary)
        assert_eq!(be32(&fat, 8 + 8).unwrap() % (1 << 14), 0);
        assert_eq!(be32(&fat, 8 + 20 + 8).unwrap() % (1 << 14), 0);
        // (a thin one asked for its own processor: as it is; for the other: said)
        assert_eq!(thin(&arm, CPU_TYPE_ARM64).unwrap(), arm);
        assert!(thin(&arm, CPU_TYPE_X86_64).unwrap_err().contains("Apple silicon"));
        assert!(universal(&[&arm, &arm]).unwrap_err().contains("two slices"));
        assert!(cpus(b"MZ\x90\0").is_err());
        assert!(cpus(&[0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 50]).is_err(), "a Java class file is no Mach-O");
        let mut cut = fat.clone();
        cut.truncate(100);
        assert!(cpus(&cut).unwrap_err().contains("damaged"));
    }
}
