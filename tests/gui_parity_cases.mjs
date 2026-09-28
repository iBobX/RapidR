// GUI fixtures (tests/fixtures/*.bas) with the events to fire on them, the
// properties to read afterwards and what the desktop AND the web must show:
// the shared table of tests/native_gui_events.mjs (desktop, native and
// interpreted builds) and tests/web_gui_parity.mjs (the browser).
//
//   events: "b1.onclick,b1.onclick,…"   fired in order (a click on the component)
//   dump:   "b1.caption,lbl.caption"    properties read afterwards
//   expect: ["b1.caption=Clicked 2", …] exact `name.property=value` lines
//   resize / split: desktop test hooks (a form resized by the user, a
//                   splitter dragged) with no browser counterpart
//   web:    false + why — the case can't be compared in a browser
//   webClick: { comp: "css selector" } — where a browser click lands for a
//             component whose clicks go through its rows / cells (the
//             desktop test hook fires the handler directly)

export const cases = [
  { name: "oop_events", events: "b1.onclick,b1.onclick,b2.onclick,b3.onclick", dump: "b1.caption,b2.caption,b3.caption",
    expect: ["b1.caption=Clicked 2", "b2.caption=Clicked 1", "b3.caption=Sender works"] },
  { name: "component_array_events", events: "btn(2).onclick,btn(3).onclick,btn(3).onclick", dump: "btn(1).caption,btn(2).caption,btn(3).caption",
    expect: ["btn(1).caption=Button1", "btn(2).caption=Hit Button2", "btn(3).caption=Hit Hit Button3"] },
  { name: "statusbar_panels", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=Ready|Line 42|INS|3|150"] },
  { name: "listview_columns", events: "lv.onclick", dump: "lbl.caption", webClick: { lv: 'tr[data-row="2"] td' },
    expect: ["lbl.caption=2|photo.jpg|Deflated|5|3|200|Method"] },
  { name: "string_grid", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=P2|P1|Lima|6|3|64|P1|Lima|4|41|-1"] },
  { name: "align_layout", events: "btn.onclick", dump: "loose.caption,side.caption,bar.caption,status.simpletext", resize: "600,350", split: "split:60",
    expect: ["loose.caption=105,40,233,205|100|245", "side.caption=moved160|160|165", "bar.caption=600x350|373x255|538|150", "status.simpletext=433|255|5|598"] },
  { name: "list_items", events: "items.onclick", dump: "summary.caption,lbl.caption",
    expect: ["summary.caption=5|zero|four|a/b & c|3|Applepear|2|2", "lbl.caption=picked 3 four"] },
  { name: "picture_resource", events: "img.onclick,img.onclick", dump: "summary.caption,lbl.caption",
    expect: ["summary.caption=1|20x10|FF00|FF0000|80FFFF|40|FF|FFFFFF|-1", "lbl.caption=click;click;"] },
  { name: "grid_draw_cell", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=round2:25|0,130,25,194,49,two|fixed4 selected3"] },
  { name: "grid_range_list", events: "", dump: "lbl.caption",
    expect: ["lbl.caption=selected1"] },
  { name: "file_browser", events: "", dump: "lbl.caption", web: false, why: "a browser has no directories to list",
    expect: ["lbl.caption=resource_files|2|hello.txt|1"] },
  { name: "canvas_onpaint", events: "btn.onclick,big.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=paints3|form1|255|65280|220x80|36"] },
  { name: "form_draw", events: "big.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=paints2|255|14737632|36|"] },
  { name: "owner_list", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=r2 0:1;1:1;2:0; 0,48,180,72 h24"] },
  { name: "nested_modal", events: "btn.onclick", dump: "lbl.caption,lbl2.caption",
    expect: ["lbl.caption=open;timer-close;closed;", "lbl2.caption=ticking"] },
];
