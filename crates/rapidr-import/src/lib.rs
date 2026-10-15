//! RapidR's RapidQ importer: RapidR's own names in a copy of a RapidQ
//! program (docs/q-and-r-components.md §6; RapidR Studio's "Import RapidQ
//! Project or File…" and "Upgrade this file to RapidR names" build on it,
//! docs/ide-plan.md, R-NAMES).
//!
//! RapidR's compilers accept both names of a component — RapidQ's
//! (`QBUTTON`) and RapidR's (`RButton`) — always, so a RapidQ program runs
//! as it is. The importer writes RapidR's names into a **copy** (the
//! original is never touched):
//!
//! - **Token-aware, from the compiler's own parse**: the names changed are
//!   exactly the tokens the parser read as type names ([`rapidr_parser::
//!   ToolsParse::type_names`]) — after `AS` in `DIM` / `CREATE` / a
//!   parameter / a TYPE field / a FUNCTION's result, after `EXTENDS` —
//!   that mean one of RapidR's components. Strings, comments, the
//!   program's own names (`QButtonCount`), its own TYPEs (an include
//!   library's `TYPE QBEVEL`) and RapidR's BASIC libraries (`QDockForm`)
//!   are never changed; neither is anything in an `$IFDEF` branch this
//!   build doesn't compile, or a name a `$DEFINE` / `$MACRO` makes (both
//!   reported).
//! - **Every include is copied with the program** ([`includes`]), found
//!   where the compiler finds it (RC.EXE's rules, then RapidR's: any case,
//!   `\` or `/`, Windows paths by their ending), the program's own and an
//!   include folder's alike, each with its own extension (`.bas`, `.rqw`,
//!   `.rqb`, `.rq`, `.inc` …); a line that would find the original instead
//!   of its copy names the copy. The includes RapidR supplies built in
//!   (RapidQ's `RAPIDQ.INC`, `QMidi.inc` …) are never carried, a user's
//!   own copy neither ([`rapidq_inc`]). The `$INCLUDE "RAPIDQ.INC"` line
//!   stays as long as those constants come with it: without it they'd be
//!   undeclared names. An include found nowhere is reported with what to
//!   do ([`import::missing_fix`]).
//! - **Proved, not assumed** ([`verify`]): each program the original
//!   compiles to is compared with what its copy compiles to, byte for
//!   byte; the report says so for each program.
//! - **The report** ([`report`]): every change by file, line and column,
//!   and everything that wasn't converted, with the reason.

pub mod convert;
pub mod diff;
pub mod files;
pub mod import;
pub mod includes;
pub mod rapidq_inc;
pub mod report;
pub mod verify;

pub use convert::{plan_program, plan_program_with, Change, Edit, FilePlan, NameStyle, Note, Options, ProgramPlan};
pub use files::{Disk, Files, Memory};
pub use import::{import, import_with, missing_fix, upgrade_file, ImportReport, IncludeReport, ProgramReport, Upgrade, Verification};
pub use includes::{IncludeRef, Place};
