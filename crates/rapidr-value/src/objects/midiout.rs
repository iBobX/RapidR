//! Which MIDI output RMIDI (QMIDI) plays a song on: a system output that
//! makes sound, or RapidR's own synthesizer (`objects::synth`). Shared by
//! the desktop (runtime-core's `media.rs`, midir) and the browser (Web
//! MIDI), so a song is heard the same way on both.
//!
//! Windows' MCI sequencer played on the first MIDI output, the GS Wavetable
//! Synth, and that is why "the first output" worked there. It doesn't
//! elsewhere: a Mac has no synthesizer among its CoreMIDI outputs, and the
//! outputs it does have are buses that carry messages to other programs
//! (an IAC bus, a network session, a virtual port another app made); a
//! Linux machine always has "Midi Through", a loop that ends nowhere. A
//! song sent to one of those is "playing" - the position moves, the volume
//! is set - and nothing is heard. So an output is only used when it is
//! known to sound:
//!
//! - one named in `RAPIDR_MIDI_OUT` (a part of its name, any case), or
//!   `builtin` for RapidR's synthesizer whatever else is there;
//! - else one whose name says it is a synthesizer (Windows' GS Wavetable
//!   Synth, FluidSynth, TiMidity, …);
//! - on Windows, else the first one that isn't a loop or a bus (a
//!   keyboard or sound module with its own sounds: what MCI used);
//! - else none: RapidR's synthesizer plays it on the sound device.

/// What `RAPIDR_MIDI_OUT` says to mean "no system output".
const BUILTIN: &[&str] = &["builtin", "built-in", "synth", "none", "off", "rapidr"];

/// Parts of the names of outputs that make sound on their own.
const SOUNDS: &[&str] = &["wavetable", "fluid", "timidity", "synth"];

/// Parts of the names of outputs that only pass messages on.
const CARRIERS: &[&str] = &["through", "loop", "iac", "session", "virtual", "bus", "internal", "network", "rtpmidi"];

fn has(name: &str, parts: &[&str]) -> bool {
    let n = name.to_lowercase();
    parts.iter().any(|p| n.contains(p))
}

/// Whether `RAPIDR_MIDI_OUT` asks for RapidR's own synthesizer.
pub fn builtin(wanted: &str) -> bool {
    BUILTIN.contains(&wanted.trim().to_lowercase().as_str())
}

/// Whether the output named `name` is a synthesizer by its name.
pub fn sounds(name: &str) -> bool {
    has(name, SOUNDS)
}

/// Whether the output named `name` only carries messages to other programs.
pub fn carries(name: &str) -> bool {
    !sounds(name) && has(name, CARRIERS)
}

/// The output to play on: the index in `names` (the system's outputs, in
/// its order), or `None` for RapidR's own synthesizer. `wanted` is
/// `RAPIDR_MIDI_OUT` (empty: automatic); `windows`: the system is Windows.
pub fn choose(names: &[String], wanted: &str, windows: bool) -> Option<usize> {
    let wanted = wanted.trim().to_lowercase();
    if !wanted.is_empty() {
        if builtin(&wanted) {
            return None;
        }
        return names.iter().position(|n| n.to_lowercase().contains(&wanted));
    }
    if let Some(i) = names.iter().position(|n| sounds(n)) {
        return Some(i);
    }
    if windows {
        return names.iter().position(|n| !carries(n));
    }
    None
}

/// What `RAPIDR_MIDI_OUT` asked for when no output has that name: the
/// message for the program's author.
pub fn not_found(wanted: &str, names: &[String]) -> String {
    let list = if names.is_empty() { "there are none".to_string() } else { names.join(", ") };
    format!("RAPIDR_MIDI_OUT={wanted}: no MIDI output has that name ({list}); RapidR's own synthesizer plays")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(n: &[&str]) -> Vec<String> {
        n.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_mac_with_only_buses_uses_the_built_in_synthesizer() {
        // (the outputs a Mac has with nothing plugged in: buses to other
        // programs. Sending to one is silence.)
        for outs in [vec![], vec!["Studio Link-Internal"], vec!["IAC Driver Bus 1"], vec!["Session 1", "IAC Driver Bus 1", "Virtual Output"]] {
            assert_eq!(choose(&names(&outs), "", false), None, "{outs:?}");
        }
    }

    #[test]
    fn linux_midi_through_is_not_a_synthesizer() {
        assert_eq!(choose(&names(&["Midi Through:Midi Through Port-0 14:0"]), "", false), None);
        // (a software synthesizer beside it is)
        assert_eq!(choose(&names(&["Midi Through:Midi Through Port-0 14:0", "FLUID Synth (4242):Synth input port (4242:0) 128:0"]), "", false), Some(1));
        assert_eq!(choose(&names(&["TiMidity:TiMidity port 0 128:0"]), "", false), Some(0));
    }

    #[test]
    fn windows_keeps_the_gs_wavetable_synth_and_first_real_output() {
        let w = names(&["Microsoft GS Wavetable Synth"]);
        assert_eq!(choose(&w, "", true), Some(0));
        assert_eq!(choose(&w, "", false), Some(0));
        // (a loop in front of it doesn't take the song)
        assert_eq!(choose(&names(&["loopMIDI Port", "Microsoft GS Wavetable Synth"]), "", true), Some(1));
        // (no GS Wavetable Synth: a sound module that is there, as MCI did)
        assert_eq!(choose(&names(&["loopMIDI Port", "UM-ONE"]), "", true), Some(1));
        assert_eq!(choose(&names(&["loopMIDI Port"]), "", true), None);
        // (not on a Mac: there, only a named one)
        assert_eq!(choose(&names(&["UM-ONE"]), "", false), None);
    }

    #[test]
    fn rapidr_midi_out_chooses() {
        let outs = names(&["IAC Driver Bus 1", "UM-ONE", "Microsoft GS Wavetable Synth"]);
        assert_eq!(choose(&outs, "um-one", false), Some(1));
        assert_eq!(choose(&outs, " IAC ", false), Some(0));
        assert_eq!(choose(&outs, "builtin", true), None);
        assert_eq!(choose(&outs, "Built-In", false), None);
        // (a name nothing has: the synthesizer, and the program's author is told)
        assert_eq!(choose(&outs, "nothing", false), None);
        assert!(not_found("nothing", &outs).contains("UM-ONE"));
        assert!(not_found("x", &[]).contains("there are none"));
    }
}
