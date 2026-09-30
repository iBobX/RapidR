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
    expect: ["lbl.caption=round2:25|0,130,25,194,49,two|fixed4 selected3"],
    // (web: each owner-drawn cell drawn at the screen's scale — sharp at 2×)
    webCheck: `(() => { const c = [...document.querySelectorAll("td canvas")]; return c.length > 0 && c.every(k => k.width === Math.round(parseFloat(k.style.width) * Math.min(3, Math.max(1, Math.ceil(devicePixelRatio))))); })()`,
    webExpect: true },
  { name: "grid_range_list", events: "", dump: "lbl.caption",
    expect: ["lbl.caption=selected1"] },
  { name: "file_browser", events: "", dump: "lbl.caption", web: false, why: "a browser has no directories to list",
    expect: ["lbl.caption=resource_files|2|hello.txt|1"] },
  { name: "late_parent", events: "btn.onclick,late.onclick", dump: "lbl.caption,late.__shown,inner.__shown,inner.caption",
    expect: ["lbl.caption=late clicked", "late.__shown=1", "inner.__shown=1", "inner.caption=inside"] },
  { name: "mdi_children", events: "badd.onclick,btile.onclick,bclose.onclick,ball.onclick", dump: "lbl.caption,geo.caption,ed(1).__shown,ed(0).__shown",
    expect: ["lbl.caption=A0 A1 A2 A0 C0 A2 C2 A1 C1 |n1|Two|i1|free-1-1|get10", "geo.caption=209,25,200,361|206|vis0-1", "ed(1).__shown=1", "ed(0).__shown=0"] },
  { name: "timer_default", events: "", dump: "lbl.caption,t.enabled",
    expect: ["lbl.caption=ticking", "t.enabled=-1"] },
  { name: "coolbtn_group", events: "b.onclick,b.onclick,d.onclick,e.onclick,e.onclick,setter.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=B-1 B-1 D0 E-1 E0 setC |00-100"] },
  { name: "canvas_onpaint", events: "btn.onclick,big.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=paints3|form1|255|65280|220x80|36"] },
  { name: "form_draw", events: "big.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=paints2|255|14737632|36|"] },
  { name: "owner_list", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=r2 0:1;1:1;2:0; 0,48,180,72 h24"] },
  { name: "dotted_paint", events: "", dump: "lbl.caption",
    expect: ["lbl.caption=painted 255"] },
  { name: "event_answers", events: "show.onclick,dlg.__close,dlg2.__close,grid.__cell_2_2,grid.__cell_3_1,g2.__cell_2_1,g2.__cell_1_2,code.onclick",
    dump: "lbl.caption,lbl2.caption,dlg.__shown,dlg2.__shown,grid.col,grid.row",
    expect: ["lbl.caption=keep1 let1lets close cell22 cell31 keep1 code |2,2|2,1,1", "lbl2.caption=0-16 16-40 40-72 ", "dlg.__shown=1", "dlg2.__shown=0", "grid.col=2", "grid.row=2"] },
  { name: "input_events", events: "ed.__key_65,ed.__key_13,ed.__key_38,cv.__mousedown_10_20,cv.__mousemove_11_21,cv.__mouseup_12_22,pn.__mousedown_3_4",
    dump: "lk.caption,lm.caption",
    expect: ["lk.caption=d65,0 p97 fd65 fpa u65 d13,0 p13 fd13 fp\r u13 d38,0 fd38 u38 ", "lm.caption=down010200 move11210 up01222 panel34"] },
  { name: "list_columns", events: "lst.__item_4,cb.__item_2", dump: "lbl.caption,lbl2.caption,cb.itemindex",
    expect: ["lbl.caption=i4 a4 cols2", "lbl2.caption=c2 0-18/112 18-38/112 38-60/112", "cb.itemindex=2"] },
  { name: "startup_modal", events: "dlgok.onclick,rp.onclick,chk.onclick", dump: "lbl.caption,lbl2.caption,dlg.__shown,form.__shown",
    expect: ["lbl.caption=before after", "lbl2.caption=repainted1", "dlg.__shown=0", "form.__shown=1"] },
  { name: "tree_view", events: "tv.__toggle_0,tv.__node_2,tv.__node_1,tv.__toggle_4,btn.onclick", dump: "lbl.caption,lbl2.caption,tv.itemindex",
    expect: ["lbl.caption=exp0 chg1 |8|Sub 1|31-10-1", "lbl2.caption=del4 del5 del6 5", "tv.itemindex=1"] },
  { name: "tree_images", events: "btn.onclick", dump: "lbl.caption", expect: ["lbl.caption=-1 -1 1 1 0 1 -1 -1"],
    // (web: the node's state image beside its own; the selection hidden while the tree hasn't focus)
    webCheck: `[...document.querySelectorAll('[data-rr-name="Tv" i] [data-node] canvas')].map(c => c.width > c.height * 1.5).join(",") + " " + document.querySelectorAll('[data-rr-name="Tv" i] .rr-tree-text[style*="background"]').length`,
    webExpect: "true,false,false 0" },
  { name: "tree_edit", events: "tv.__node_0,tv.__edit,tv.__enter,tv.__node_2,tv.__edit,tv.__enter,tv.__node_1,tv.__edit,tv.__escape,ro.onclick,tv.__edit,tv.__enter,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=ing0 ed0:Renamed ing2 ing1 |RENAMED Pear Plum -1"] },
  { name: "panel_bevels", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=201 123 112"] },
  { name: "svg_picture", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=48 1 D4FF FFFFFF"] },
  { name: "outline", events: "outline.__toggle_3,outline.__node_4,btn.onclick", dump: "lbl.caption,outline.row",
    expect: ["lbl.caption=6 First Child of Parent 2 2", "outline.row=4"] },
  { name: "file_dialogs", events: "b1.onclick,b2.onclick,b3.onclick", dump: "lbl.caption,lbl2.caption,lbl3.caption", fileDialog: "notes;b.txt",
    expect: ["lbl.caption=open notes", "lbl2.caption=save notes.txt", "lbl3.caption=2 notes b.txt "] },
  { name: "header", events: "header.__mousedown_20_5,header.__mouseup_20_5,header.__mousedown_120_5,header.__mouseup_120_5,header.__mousedown_100_5,header.__mousemove_140_5,header.__mouseup_140_5,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=c0 t0:100:0 t0:140:1 t0:140:2 r0 | 3 140 Chart FF00"] },
  { name: "icons", events: "", dump: "lbl.caption,img.width", expect: ["lbl.caption=16x16 C85A14", "img.width=16"],
    // (web: each form's title bar icon — its own, else the application's — and the page's)
    webCheck: `(() => { const src = (n) => { const i = document.querySelector('.rr-form[data-rr-name="' + n + '"] .rr-form-icon'); return i && i.style.display !== "none" ? i.src : ""; };
      return [src("FORM").startsWith("data:image/png"), src("OTHER").startsWith("data:image/png"), src("FORM") !== src("OTHER"), !!document.querySelector("link[rel~='icon'][href^='data:image/png']")].join(","); })()`,
    webExpect: "true,true,true,true" },
  { name: "nested_modal", events: "btn.onclick", dump: "lbl.caption,lbl2.caption",
    expect: ["lbl.caption=open;timer-close;closed;", "lbl2.caption=ticking"] },
];
