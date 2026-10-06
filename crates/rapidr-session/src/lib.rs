//! RapidR's program session protocol (docs/ide-plan.md §3.3, I0): how
//! RapidR Studio runs, stops, debugs and inspects the program under
//! development, which runs in its own process (desktop) or sandboxed frame
//! (web).
//!
//! - [`protocol`]: the messages — the IDE's [`protocol::Request`]s (start,
//!   stop, pause, breakpoints, continue / steps, stack, scopes, variables,
//!   evaluate, set variable, set property, input) and the program's
//!   [`protocol::Event`]s (replies, stopped, continued, output, forms,
//!   exited), as JSON; the framing that lets events share the program's
//!   standard output.
//! - [`client`]: the IDE's end, [`client::ProgramSession`] — the model of
//!   the non-visual RProgramSession component — over a
//!   [`client::Transport`]; [`process`] is the desktop's (a child process,
//!   `rapidr run --session`).
//! - `program` (feature `program`): the program's end, served on the VM by
//!   the hosts (`rapidr-vm-host-native`, `rapidr-vm-host-web`).
//!
//! The DAP server (I6), the MCP tools (I8) and the IDE's own views are all
//! clients of the same protocol.

pub mod client;
#[cfg(not(target_arch = "wasm32"))]
pub mod process;
#[cfg(feature = "program")]
pub mod program;
pub mod protocol;

pub use client::{ProgramSession, SessionEvent, State, Transport};
pub use protocol::{Command, Event, EventBody, Incoming, Request};
