//! Linked data (docs/data-science-plan.md §4): sources, data sets, the
//! current row, and the controls bound to them. (Being built: the frame
//! engine tells it when a frame changes.)

/// Frame `name` changed (its version moved on).
pub fn frame_changed(_name: &str) {}
