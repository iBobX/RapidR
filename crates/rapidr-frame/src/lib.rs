//! RDATAFRAME — a table in the manner of pandas, on polars: the one
//! implementation every RapidR runtime uses (decision D7, docs/ide-plan.md).
//! The desktop runtime (native and interpreted programs) links this crate;
//! the web loads it as a wasm module of its own (`rapidr-frame-web`), only
//! when a program uses data frames.
//!
//! The interface is pure, so it crosses a module boundary as data: a
//! [`Call`] in — the frame's name, the method, its arguments, the bytes of
//! a file the method reads (the runtime reads it: [`file_arg`]), a seed
//! from the program's random numbers — and a [`Reply`] out: the value, text
//! to print, a file to write, a grid to fill, an error to report. Frames
//! live here by name (not case-sensitive), as the runtimes' other
//! components do.
//!
//! Column names are matched without regard to case (BASIC's way); methods
//! change the frame in place; what a cell reads is its text (`""` for a
//! missing value). Where the two former implementations differed, the
//! desktop's meaning was kept — `SetCell(row, column, value)` as
//! `Cell(row, column)` — and the web's extra members (`Create`, `AddRow`, a
//! CSV text for `LoadFromCsv`) joined it.

use serde::{Deserialize, Serialize};

// The engine (polars): the desktop links it, the web module builds it;
// the web runtime itself takes only the types above (no polars in it).
#[cfg(feature = "engine")]
mod engine;
#[cfg(feature = "engine")]
pub use engine::{call, get_prop};

/// An argument as BASIC passed it: its text, its number, and whether it
/// was a number (rather than a string).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Arg {
    pub text: String,
    pub num: f64,
    pub is_num: bool,
}

impl Arg {
    pub fn text(s: &str) -> Arg {
        Arg { text: s.to_string(), num: s.trim().parse().unwrap_or(0.0), is_num: false }
    }
    pub fn num(n: f64, text: String) -> Arg {
        Arg { text, num: n, is_num: true }
    }
    #[cfg_attr(not(feature = "engine"), allow(dead_code))]
    fn int(&self) -> i64 {
        if self.is_num {
            self.num as i64
        } else {
            self.text.trim().parse::<f64>().map(|f| f as i64).unwrap_or(0)
        }
    }
}

/// A value a method returns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Out {
    Null,
    Int(i64),
    Dbl(f64),
    Str(String),
}

/// One method call.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Call {
    pub name: String,
    /// Lower case.
    pub method: String,
    pub args: Vec<Arg>,
    /// The bytes of the file [`file_arg`] named (or why it couldn't be read).
    #[serde(skip)]
    pub file: Option<Result<Vec<u8>, String>>,
    /// From the program's random numbers (RANDOMIZE): what `Sample` draws with.
    pub seed: u64,
}

/// What a call did.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reply {
    pub value: Out,
    /// Text to PRINT (a line each).
    pub print: Option<String>,
    /// A file to write: its name (as the program gave it) and contents.
    pub write: Option<(String, Vec<u8>)>,
    /// A QSTRINGGRID to fill: its name and rows (the column names first).
    pub grid: Option<(String, Vec<Vec<String>>)>,
    /// A problem to report (the runtime's warning stream).
    pub error: Option<String>,
}

#[cfg_attr(not(feature = "engine"), allow(dead_code))]
impl Reply {
    fn value(value: Out) -> Reply {
        Reply { value, print: None, write: None, grid: None, error: None }
    }
    fn null() -> Reply {
        Reply::value(Out::Null)
    }
    fn error(method: &str, e: impl std::fmt::Display) -> Reply {
        Reply { error: Some(format!("RDataFrame.{method}: {e}")), ..Reply::null() }
    }
}

/// The file a call reads, when it reads one: the runtime passes its bytes
/// in [`Call::file`]. (`LoadFromCsv` given CSV text itself — with a line
/// break in it — reads none.)
pub fn file_arg(method: &str, args: &[Arg]) -> Option<String> {
    let path = args.first().map(|a| a.text.clone()).unwrap_or_default();
    match method {
        "loadfromcsv" | "readcsv" | "read_csv" | "loadfromjson" | "read_json" if !path.contains('\n') => Some(path),
        _ => None,
    }
}
