//! The designer's defaults: what the language registry says of a new
//! component (its RapidQ name to write, its properties, its default event)
//! and the size the runtimes give it (`rapidr_value::layout::default_size`,
//! which the designer's layout replays). The registry's `size` must be the
//! runtimes' (RC.EXE-measured: they decide where a component is).

use rapidr_value::designer::text::new_component;
use rapidr_value::designer::{FormDesign, Layout};
use rapidr_value::layout::Rect;

#[test]
fn new_components_from_the_registry_sized_as_the_runtimes() {
    let mut differ = Vec::new();
    for name in rapidr_lang::COMPONENT_TYPES {
        let c = rapidr_lang::component(name).unwrap();
        // (written under RapidQ's name when RapidQ has it)
        let tree = new_component(&FormDesign::new("Form1", "QFORM"), name, Rect::new(8, 8, 10, 10));
        assert_eq!(tree.type_written, c.written_name(), "{name}");
        if let (Some((rw, rh)), Some((w, h))) = (c.size, rapidr_value::layout::default_size(name)) {
            if (rw as i64, rh as i64) != (w, h) {
                differ.push(format!("{name}: registry {rw}x{rh}, runtimes {w}x{h}"));
            }
        }
    }
    // A component the designer adds without a size gets the runtimes' size.
    let mut form = rapidr_value::designer::Subtree::new("Form1", "QFORM", &[]);
    form.body.push(rapidr_value::designer::SubItem::Child(rapidr_value::designer::Subtree::new("Lv", "QLISTVIEW", &[])));
    let d = FormDesign::from_subtree(form);
    let r = Layout::of(&d).rect(d.find("Lv").unwrap()).unwrap();
    assert_eq!(Some((r.width, r.height)), rapidr_value::layout::default_size("RLISTVIEW"));
    assert!(differ.is_empty(), "the registry's sizes are the runtimes' (one truth):\n{}", differ.join("\n"));
}
