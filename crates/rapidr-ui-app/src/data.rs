//! (I7) Linked data between the user and the program, for every host: an
//! REdit bound to a data set's field (DataSource + DataField) that the user
//! changed writes the field back; and after every event of the program's
//! windows, what the linked data changed meanwhile (a grid's row clicked,
//! an edit typed in) is carried out — the bound controls show their
//! fields, grids are drawn again, OnDataChange fires
//! (`rapidr_value::datascience::link`, shared with the runtimes, which do
//! the same after the program's own calls).

use rapidr_value::datascience::link::{self, Update};

use crate::windows::invalidate;
use crate::{Program, Windows};

/// The user changed component `id` (OnChange): when it is a bound REdit,
/// its field takes its text.
pub fn changed<R: Program>(rt: R, id: &str) {
    if rapidr_value::objects::is_textedit(id) {
        link::edited(id, &rt.get(id, "text").to_string_val());
    }
}

/// What the linked data changed, carried out.
pub fn apply<R: Program + Windows>(rt: R) {
    for _ in 0..8 {
        if !link::pending() {
            return;
        }
        let updates = link::sync();
        if updates.is_empty() {
            return;
        }
        for u in updates {
            match u {
                Update::Set(id, prop, value) => rt.set(&id, &prop, value),
                Update::Redraw(_) => invalidate(),
                Update::Fire(id, event, args) => rt.fire_args(&id, &event, &args),
                Update::Warn(text) => eprintln!("[WARN] {text}"),
            }
        }
        invalidate();
    }
}
