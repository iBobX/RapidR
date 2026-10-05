//! Standard MIDI files (`.mid`, and the RIFF `RMID` wrapper `.rmi`) as
//! QMIDI plays them: every track's channel and system-exclusive messages
//! merged in time order, each at its time in microseconds by the file's
//! tempo map, and the song's length. Times are counted as Windows' MCI
//! sequencer counts them — whole microseconds per tick at each tempo — so
//! the length a program reads (`QMIDI.Lenght`, MCI's `status length` in
//! milliseconds) is the same: `town.mid` (Windows' own) is 78994 ms there
//! and here. docs/io-media-plan.md §4.

/// A message and when it plays (µs from the start).
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub at_us: u64,
    /// A channel message (status byte first) or a system-exclusive one
    /// (F0 … F7).
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    pub events: Vec<Event>,
    /// The end of the longest track, in µs.
    pub length_us: u64,
}

impl Song {
    /// Its length as MCI gives it (milliseconds, the fraction dropped).
    pub fn length_ms(&self) -> i64 {
        (self.length_us / 1000) as i64
    }
}

fn be32(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

fn vlq(b: &[u8], i: &mut usize, end: usize) -> Option<u32> {
    let mut v: u32 = 0;
    for _ in 0..4 {
        if *i >= end {
            return None;
        }
        let c = b[*i];
        *i += 1;
        v = (v << 7) | u32::from(c & 0x7F);
        if c < 0x80 {
            return Some(v);
        }
    }
    None
}

/// Reads a MIDI file; `None` for anything else (MCI's "cannot be played").
pub fn parse(bytes: &[u8]) -> Option<Song> {
    // (RIFF RMID: the `data` chunk holds the standard file)
    let b = if bytes.get(0..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"RMID") {
        let mut at = 12;
        let mut found = None;
        while at + 8 <= bytes.len() {
            let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().ok()?) as usize;
            if &bytes[at..at + 4] == b"data" {
                found = bytes.get(at + 8..(at + 8 + len).min(bytes.len()));
                break;
            }
            at += 8 + len + (len & 1);
        }
        found?
    } else {
        bytes
    };
    if b.get(0..4) != Some(b"MThd") {
        return None;
    }
    let header_len = be32(b, 4)? as usize;
    let tracks = u16::from_be_bytes(b.get(10..12)?.try_into().ok()?);
    let division = u16::from_be_bytes(b.get(12..14)?.try_into().ok()?);
    if division == 0 {
        return None;
    }
    // (tick, order, message) and (tick, tempo µs a quarter)
    let mut messages: Vec<(u64, usize, Vec<u8>)> = Vec::new();
    let mut tempos: Vec<(u64, u32)> = Vec::new();
    let mut end_tick = 0u64;
    let mut at = 8 + header_len;
    for _ in 0..tracks {
        if b.get(at..at + 4) != Some(b"MTrk") {
            break;
        }
        let len = be32(b, at + 4)? as usize;
        let mut i = at + 8;
        let end = (i + len).min(b.len());
        let (mut tick, mut running) = (0u64, 0u8);
        while i < end {
            tick += u64::from(vlq(b, &mut i, end)?);
            let status = *b.get(i)?;
            match status {
                0xFF => {
                    let kind = *b.get(i + 1)?;
                    i += 2;
                    let l = vlq(b, &mut i, end)? as usize;
                    if kind == 0x51 && l >= 3 {
                        let t = b.get(i..i + 3)?;
                        tempos.push((tick, u32::from_be_bytes([0, t[0], t[1], t[2]])));
                    }
                    i += l;
                    if kind == 0x2F {
                        break;
                    }
                }
                0xF0 | 0xF7 => {
                    i += 1;
                    let l = vlq(b, &mut i, end)? as usize;
                    let data = b.get(i..i + l)?;
                    let mut m = Vec::with_capacity(l + 1);
                    if status == 0xF0 {
                        m.push(0xF0);
                    }
                    m.extend_from_slice(data);
                    messages.push((tick, messages.len(), m));
                    i += l;
                }
                _ => {
                    if status & 0x80 != 0 {
                        running = status;
                        i += 1;
                    }
                    if running & 0x80 == 0 {
                        return None;
                    }
                    let n = if matches!(running & 0xF0, 0xC0 | 0xD0) { 1 } else { 2 };
                    let data = b.get(i..i + n)?;
                    let mut m = vec![running];
                    m.extend_from_slice(data);
                    messages.push((tick, messages.len(), m));
                    i += n;
                }
            }
        }
        end_tick = end_tick.max(tick);
        at = at + 8 + len;
    }
    tempos.sort_by_key(|t| t.0);
    messages.sort_by_key(|m| (m.0, m.1));
    // Ticks to µs: SMPTE divisions are ticks a second; else µs a tick
    // (whole) at each tempo (500000 µs a quarter until one is given).
    let us_per_tick = |tempo: u32| -> u64 {
        if division & 0x8000 != 0 {
            let fps = u64::from(256 - u32::from(division >> 8)).max(1);
            let sub = u64::from(division & 0xFF).max(1);
            1_000_000 / (fps * sub)
        } else {
            u64::from(tempo) / u64::from(division)
        }
    };
    let time_of = |tick: u64| -> u64 {
        let (mut us, mut last, mut tempo) = (0u64, 0u64, 500_000u32);
        for &(t, v) in &tempos {
            if t >= tick {
                break;
            }
            us += (t - last) * us_per_tick(tempo);
            last = t;
            tempo = v;
        }
        us + (tick - last) * us_per_tick(tempo)
    };
    let events = messages.into_iter().map(|(t, _, bytes)| Event { at_us: time_of(t), bytes }).collect();
    Some(Song { events, length_us: time_of(end_tick) })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A format 1 file: a tempo track, a track with two notes.
    fn file() -> Vec<u8> {
        let mut b = b"MThd\0\0\0\x06\0\x01\0\x02\x01\x80".to_vec(); // 384 ticks a quarter
        let t0 = [0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20, 0x00, 0xFF, 0x2F, 0x00]; // 500000
        b.extend(b"MTrk");
        b.extend((t0.len() as u32).to_be_bytes());
        b.extend(t0);
        let t1 = [0x00, 0x90, 60, 100, 0x83, 0x00, 60, 0, 0x00, 0xC0, 5, 0x83, 0x00, 0x80, 64, 0, 0x00, 0xFF, 0x2F, 0x00];
        b.extend(b"MTrk");
        b.extend((t1.len() as u32).to_be_bytes());
        b.extend(t1);
        b
    }

    #[test]
    fn events_and_length() {
        let s = parse(&file()).unwrap();
        // (384 ticks = a quarter at 500000 µs: 1302 µs a tick, as MCI)
        assert_eq!(s.length_us, 768 * 1302);
        assert_eq!(s.length_ms(), 999);
        let at: Vec<(u64, Vec<u8>)> = s.events.iter().map(|e| (e.at_us, e.bytes.clone())).collect();
        assert_eq!(at, vec![(0, vec![0x90, 60, 100]), (384 * 1302, vec![0x90, 60, 0]), (384 * 1302, vec![0xC0, 5]), (768 * 1302, vec![0x80, 64, 0])]);
        assert_eq!(parse(b"not midi"), None);
        // (cut short: refused, never a panic)
        let f = file();
        for n in 0..f.len() {
            let _ = parse(&f[..n]);
        }
    }

    /// Windows' own songs (not shipped: `RAPIDR_WINDOWS_MEDIA` names a
    /// folder holding C:\Windows\Media's .mid files) against MCI's
    /// `status length` there.
    #[test]
    #[ignore]
    fn windows_media() {
        let dir = std::env::var("RAPIDR_WINDOWS_MEDIA").expect("RAPIDR_WINDOWS_MEDIA");
        let town = parse(&std::fs::read(format!("{dir}/town.mid")).unwrap()).unwrap();
        assert_eq!(town.length_ms(), 78994);
    }

    #[test]
    fn riff_rmid() {
        let smf = file();
        let mut b = b"RIFF".to_vec();
        b.extend(((smf.len() + 12) as u32).to_le_bytes());
        b.extend(b"RMIDdata");
        b.extend((smf.len() as u32).to_le_bytes());
        b.extend(&smf);
        assert_eq!(parse(&b).unwrap().length_ms(), 999);
    }
}
