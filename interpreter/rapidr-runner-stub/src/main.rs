//! `rapidrintr-runner` — bytecode runtime stub for self-contained
//! native executables.
//!
//! At build time the RapidR CLI copies this binary to the user's
//! desired output path and **appends** the program's `.rrbc` bytes plus
//! a 12-byte footer of the form:
//!
//! ```text
//! ... [stub elf/macho bytes] [rrbc bytes] [magic 8B "RRBCEXE1"] [u32 LE length]
//! ```
//!
//! At startup the runner opens its own executable, seeks to the end,
//! reads the footer, and passes the payload bytes to
//! [`rapidr_vm_host_native::run_bytes`].
//!
//! When invoked with no payload (i.e. a freshly built stub) it falls
//! back to `rapidrintr-runner --bytecode <file.rrbc>` for testing /
//! development.

use std::env;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::process::ExitCode;

const FOOTER_MAGIC: &[u8; 8] = b"RRBCEXE1";
const FOOTER_LEN: usize = 12; // 8 magic + 4 length

fn read_appended_payload() -> Option<Vec<u8>> {
    let exe = env::current_exe().ok()?;
    let mut f = File::open(&exe).ok()?;
    let len = f.metadata().ok()?.len();
    if len < FOOTER_LEN as u64 {
        return None;
    }
    f.seek(SeekFrom::End(-(FOOTER_LEN as i64))).ok()?;
    let mut footer = [0u8; FOOTER_LEN];
    f.read_exact(&mut footer).ok()?;
    if &footer[..8] != FOOTER_MAGIC {
        return None;
    }
    let payload_len = u32::from_le_bytes([footer[8], footer[9], footer[10], footer[11]]) as u64;
    if payload_len == 0 || payload_len + (FOOTER_LEN as u64) > len {
        return None;
    }
    let payload_offset = len - (FOOTER_LEN as u64) - payload_len;
    f.seek(SeekFrom::Start(payload_offset)).ok()?;
    let mut payload = vec![0u8; payload_len as usize];
    f.read_exact(&mut payload).ok()?;
    Some(payload)
}

fn read_external_bytecode(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("read {path}: {e}"))
}

fn print_usage() {
    eprintln!(
"rapidrintr-runner — RapidR bytecode runtime stub

Usage:
  (run a built program)         <this exe>
  (run an external bytecode)    <this exe> --bytecode <file.rrbc>

When invoked with no arguments the runner expects a `.rrbc` payload
appended to the end of its own binary by `rapidr build --interp`.
With `--bytecode` it loads the given file instead — useful for
development."
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let bytes = if let Some(payload) = read_appended_payload() {
        payload
    } else {
        match args.first().map(|s| s.as_str()) {
            Some("--bytecode") | Some("-b") => match args.get(1) {
                Some(path) => match read_external_bytecode(path) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("{e}");
                        return ExitCode::from(1);
                    }
                },
                None => {
                    eprintln!("error: --bytecode requires a path");
                    return ExitCode::from(2);
                }
            },
            Some("--help") | Some("-h") | None => {
                print_usage();
                return ExitCode::from(2);
            }
            Some(other) => {
                eprintln!("unknown argument: {other}");
                print_usage();
                return ExitCode::from(2);
            }
        }
    };

    match rapidr_vm_host_native::run_bytes(&bytes) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rapidrintr: {e}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_round_trips() {
        // Simulate "stub bytes" + "rrbc payload" + footer, then
        // verify our extractor can recover the payload.
        let stub = b"\x7fELF...stub bytes...";
        let payload = b"RRBC\x01\x00...fake bytecode...";
        let mut blob = Vec::new();
        blob.extend_from_slice(stub);
        blob.extend_from_slice(payload);
        blob.extend_from_slice(FOOTER_MAGIC);
        blob.extend_from_slice(&(payload.len() as u32).to_le_bytes());

        // Inline copy of the extractor logic (read_appended_payload
        // reads from current_exe, which we can't override in tests).
        let len = blob.len();
        let footer = &blob[len - FOOTER_LEN..];
        assert_eq!(&footer[..8], FOOTER_MAGIC);
        let plen = u32::from_le_bytes([footer[8], footer[9], footer[10], footer[11]]) as usize;
        let recovered = &blob[len - FOOTER_LEN - plen..len - FOOTER_LEN];
        assert_eq!(recovered, payload);
    }
}
