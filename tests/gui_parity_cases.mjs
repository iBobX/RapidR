// GUI fixtures (tests/fixtures/*.bas) with the events to fire on them, the
// properties to read afterwards and what the desktop AND the web must show:
// the shared table of tests/native_gui_events.mjs (desktop, native and
// interpreted builds) and tests/web_gui_parity.mjs (the browser).
//
//   events: "b1.onclick,b1.onclick,…"   fired in order (a click on the component)
//   dump:   "b1.caption,lbl.caption"    properties read afterwards
//   expect: ["b1.caption=Clicked 2", …] exact `name.property=value` lines
//   resize / split: test hooks (the frontmost form resized by the user to
//                   "w,h", a splitter dragged "name:delta"), before the
//                   events — RAPIDR_TEST_RESIZE / RAPIDR_TEST_SPLIT on the
//                   desktop, rapidr_test_resize in the browser
//   web:    false + why — the case can't be compared in a browser
//   webClick: { comp: "css selector" } — where a browser click lands for a
//             component whose clicks go through its rows / cells (the
//             desktop test hook fires the handler directly)
//   kernel: true — the case runs on the UI kernel host too (RAPIDR_HOSTS
//           matrix of tests/native_gui_events.mjs); "pending: why" or absent:
//           not yet (docs/desktop-host-plan.md §2.4)

export const cases = [
  { name: "oop_events", kernel: true, events: "b1.onclick,b1.onclick,b2.onclick,b3.onclick", dump: "b1.caption,b2.caption,b3.caption",
    expect: ["b1.caption=Clicked 2", "b2.caption=Clicked 1", "b3.caption=Sender works"] },
  { name: "component_array_events", kernel: true, events: "btn(2).onclick,btn(3).onclick,btn(3).onclick", dump: "btn(1).caption,btn(2).caption,btn(3).caption",
    expect: ["btn(1).caption=Button1", "btn(2).caption=Hit Button2", "btn(3).caption=Hit Hit Button3"] },
  { name: "statusbar_panels", kernel: true, events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=Ready|Line 42|INS|3|150"] },
  { name: "listview_columns", kernel: true, events: "lv.__mousedown_20_67,lv.__mouseup_20_67", dump: "lbl.caption",
    expect: ["lbl.caption=2|photo.jpg|Deflated|5|3|200|Method"] },
  { name: "listview_views", kernel: true, events: "lv.__mousedown_45_31,lv.__mouseup_45_31,lv.__mousedown_8_49,lv.__mouseup_8_49,lv.__key_40,lv.__key_32,lv.__mousedown_150_10,lv.__mouseup_150_10,btn2.onclick,lv.__mousedown_190_20,lv.__mouseup_190_20,lv.__edit,lv.__enter,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=c0:2 k c1:2 k c0:2 c1:2 c1:2 h1 c1:2 c2:2 k c2:0 | 2 0 Apple 0 2 0"] },
  { name: "grid_moving", kernel: true, events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=5 3 H1a1 40 R2 3"] },
  { name: "string_grid", kernel: true, events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=P2|P1|Lima|6|3|64|P1|Lima|4|41|-1"] },
  { name: "align_layout", kernel: true, events: "btn.onclick", dump: "loose.caption,side.caption,bar.caption,status.simpletext", resize: "600,350", split: "split:60",
    expect: ["loose.caption=105,40,233,205|100|245", "side.caption=moved160|160|165", "bar.caption=600x350|373x255|538|150", "status.simpletext=433|255|5|598"] },
  { name: "list_items", kernel: true, events: "items.onclick", dump: "summary.caption,lbl.caption",
    expect: ["summary.caption=5|zero|four|a/b & c|3|Applepear|2|2", "lbl.caption=picked 3 four"] },
  { name: "picture_resource", kernel: true, events: "img.onclick,img.onclick", dump: "summary.caption,lbl.caption",
    expect: ["summary.caption=1|20x10|FF00|FF0000|80FFFF|40|FF|FFFFFF|-1", "lbl.caption=click;click;"] },
  { name: "grid_draw_cell", kernel: true, events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=round2:25|0,130,25,194,49,two|fixed4 selected3"],
    // (web: each owner-drawn cell drawn at the screen's scale — sharp at 2×)
    webCheck: `(() => { const c = [...document.querySelectorAll("td canvas")]; return c.length > 0 && c.every(k => k.width === Math.round(parseFloat(k.style.width) * Math.min(3, Math.max(1, Math.ceil(devicePixelRatio))))); })()`,
    webExpect: true },
  { name: "grid_range_list", kernel: true, events: "", dump: "lbl.caption",
    expect: ["lbl.caption=selected1"] },
  { name: "file_browser", kernel: true, events: "", dump: "lbl.caption", web: false, why: "a browser has no directories to list",
    expect: ["lbl.caption=resource_files|2|hello.txt|1"] },
  { name: "late_parent", kernel: true, events: "btn.onclick,late.onclick", dump: "lbl.caption,late.__shown,inner.__shown,inner.caption",
    expect: ["lbl.caption=late clicked", "late.__shown=1", "inner.__shown=1", "inner.caption=inside"] },
  { name: "mdi_children", kernel: true, events: "badd.onclick,btile.onclick,bclose.onclick,ball.onclick", dump: "lbl.caption,geo.caption,ed(1).__shown,ed(0).__shown",
    expect: ["lbl.caption=A0 A1 A2 A0 C0 A2 C2 A1 C1 |n1|Two|i1|free-1-1|get10", "geo.caption=209,25,200,361|206|vis0-1", "ed(1).__shown=1", "ed(0).__shown=0"] },
  { name: "timer_default", kernel: true, events: "", dump: "lbl.caption,t.enabled",
    expect: ["lbl.caption=ticking", "t.enabled=-1"] },
  { name: "coolbtn_group", kernel: true, events: "b.onclick,b.onclick,d.onclick,e.onclick,e.onclick,setter.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=B-1 B-1 D0 E-1 E0 setC |00-100"] },
  { name: "canvas_onpaint", kernel: true, events: "btn.onclick,big.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=paints3|form1|255|65280|220x80|36"] },
  { name: "form_draw", kernel: true, events: "big.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=paints2|255|14737632|36|"] },
  { name: "owner_list", kernel: true, events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=r2 0:1;1:1;2:0; 0,48,180,72 h24"] },
  { name: "dotted_paint", kernel: true, events: "", dump: "lbl.caption",
    expect: ["lbl.caption=painted 255"] },
  { name: "event_answers", kernel: true, events: "show.onclick,dlg.__close,dlg2.__close,grid.__cell_2_2,grid.__cell_3_1,g2.__cell_2_1,g2.__cell_1_2,code.onclick",
    dump: "lbl.caption,lbl2.caption,dlg.__shown,dlg2.__shown,grid.col,grid.row",
    expect: ["lbl.caption=keep1 let1lets close cell22 cell31 keep1 code |2,2|2,1,1", "lbl2.caption=0-16 16-40 40-72 ", "dlg.__shown=1", "dlg2.__shown=0", "grid.col=2", "grid.row=2"] },
  { name: "input_events", kernel: true, events: "ed.__key_65,ed.__key_13,ed.__key_38,cv.__mousedown_10_20,cv.__mousemove_11_21,cv.__mouseup_12_22,pn.__mousedown_3_4",
    dump: "lk.caption,lm.caption",
    expect: ["lk.caption=fd65 d65,0 fpa p97 u65 fd13 d13,0 fp\r p13 u13 fd38 d38,0 u38 ", "lm.caption=down010200 move11210 up01222 panel34"] },
  { name: "list_columns", kernel: true, events: "lst.__item_4,cb.__item_2", dump: "lbl.caption,lbl2.caption,cb.itemindex",
    expect: ["lbl.caption=i4 a4 cols2", "lbl2.caption=c2 0-18/112 18-38/112 38-60/112", "cb.itemindex=2"] },
  { name: "startup_modal", kernel: true, events: "dlgok.onclick,rp.onclick,chk.onclick", dump: "lbl.caption,lbl2.caption,dlg.__shown,form.__shown",
    expect: ["lbl.caption=before after", "lbl2.caption=repainted1", "dlg.__shown=0", "form.__shown=1"] },
  { name: "tree_view", kernel: true, events: "tv.__toggle_0,tv.__node_2,tv.__node_1,tv.__toggle_4,btn.onclick", dump: "lbl.caption,lbl2.caption,tv.itemindex",
    expect: ["lbl.caption=exp0 chg1 |8|Sub 1|31-10-1", "lbl2.caption=del4 del5 del6 5", "tv.itemindex=1"] },
  { name: "tree_images", kernel: true, events: "btn.onclick", dump: "lbl.caption", expect: ["lbl.caption=-1 -1 1 1 0 1 -1 -1"],
    // (web: the node's state image beside its own; the selection hidden while the tree hasn't focus)
    webCheck: `[...document.querySelectorAll('[data-rr-name="Tv" i] [data-node] canvas')].map(c => c.width > c.height * 1.5).join(",") + " " + document.querySelectorAll('[data-rr-name="Tv" i] .rr-tree-text[style*="background"]').length`,
    webExpect: "true,false,false 0" },
  { name: "tree_edit", kernel: true, events: "tv.__node_0,tv.__edit,tv.__enter,tv.__node_2,tv.__edit,tv.__enter,tv.__node_1,tv.__edit,tv.__escape,ro.onclick,tv.__edit,tv.__enter,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=ing0 ed0:Renamed ing2 ing1 |RENAMED Pear Plum -1"] },
  { name: "panel_bevels", kernel: true, events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=201 123 112"] },
  { name: "svg_picture", kernel: true, events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=48 1 D4FF FFFFFF"] },
  { name: "outline", kernel: true, events: "outline.__toggle_3,outline.__node_4,btn.onclick", dump: "lbl.caption,outline.row",
    expect: ["lbl.caption=6 First Child of Parent 2 2", "outline.row=4"] },
  // (`colorDialog`: what each colour dialog answers in turn, `;`-separated
  // — a colour (decimal, one of the basic swatches for the browser) for
  // OK, empty for Cancel: RAPIDR_TEST_COLOR_DIALOG on the desktop, the page
  // dialog clicked in the browser)
  { name: "color_dialog", kernel: true, events: "b1.onclick,b2.onclick", dump: "lbl.caption,lbl2.caption", colorDialog: "255;",
    expect: ["lbl.caption=0|2|80|FF00FF", "lbl2.caption=ok FF 123456|cancel FF"] },
  // (`fontDialog`: likewise, `Name,Size,styles (b i u s),colour`)
  { name: "font_dialog", kernel: true, events: "b1.onclick,b2.onclick", dump: "lbl.caption,lbl2.caption", fontDialog: "Courier New,14,bu,255;",
    expect: ["lbl.caption=Arial|10|8|Courier New|Times New Roman12", "lbl2.caption=ok Courier New14 -10-1 FF|cancel Courier New"] },
  // (QFORM.WindowState: maximize, restore, minimize; OnResize counted by a
  // later click)
  { name: "window_state", kernel: true, events: "b1.onclick,b4.onclick,b2.onclick,b4.onclick,b3.onclick,b4.onclick", dump: "lbl.caption,lbl2.caption,lbl3.caption",
    expect: ["lbl.caption=2 -1-1-1|0 300x200 -1-1", "lbl2.caption=1 300 -1|0 300", "lbl3.caption=1;2;2;"] },
  { name: "file_dialogs", kernel: true, events: "b1.onclick,b2.onclick,b3.onclick", dump: "lbl.caption,lbl2.caption,lbl3.caption", fileDialog: "notes;b.txt",
    expect: ["lbl.caption=open notes", "lbl2.caption=save notes.txt", "lbl3.caption=2 notes b.txt "] },
  { name: "header", kernel: true, events: "header.__mousedown_20_5,header.__mouseup_20_5,header.__mousedown_120_5,header.__mouseup_120_5,header.__mousedown_100_5,header.__mousemove_140_5,header.__mouseup_140_5,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=c0 t0:100:0 t0:140:1 t0:140:2 r0 | 3 140 Chart FF00"] },
  { name: "icons", kernel: true, events: "", dump: "lbl.caption,img.width,img2.width", expect: ["lbl.caption=16x16 C85A14", "img.width=16", "img2.width=16"],
    // (web: each form's title bar icon — its own, else the application's — and the page's)
    webCheck: `(() => { const src = (n) => { const i = document.querySelector('.rr-form[data-rr-name="' + n + '"] .rr-form-icon'); return i && i.style.display !== "none" ? i.src : ""; };
      return [src("FORM").startsWith("data:image/png"), src("OTHER").startsWith("data:image/png"), src("FORM") !== src("OTHER"), !!document.querySelector("link[rel~='icon'][href^='data:image/png']")].join(","); })()`,
    webExpect: "true,true,true,true" },
  { name: "option_icon", kernel: true, events: "", dump: "lbl.caption", expect: ["lbl.caption=-1"],
    webCheck: `(() => { const i = document.querySelector('.rr-form[data-rr-name="FORM"] .rr-form-icon'); return !!i && i.style.display !== "none" && i.src.startsWith("data:image/png"); })()`,
    webExpect: true },
  { name: "doevents_loop", kernel: true, events: "", dump: "lbl.caption", expect: ["lbl.caption=ticked -1 -1"] },
  { name: "inkey_wait", kernel: true, events: "lbl.__key_65,lbl.__key_38", dump: "lbl.caption", expect: ["lbl.caption=97/1 27/2"] },
  { name: "inkey_trapall", kernel: true, events: "lbl.__key_16,lbl.__key_65", dump: "lbl.caption", expect: ["lbl.caption=27:42 97:97 "] },
  { name: "grid_draw_hidden", kernel: true, events: "", dump: "lbl.caption", expect: ["lbl.caption=before=0 mark=m"] },
  { name: "border_icons", kernel: true, events: "", dump: "form.bordericons", expect: ["form.bordericons=11"],
    webCheck: `["min", "max", "close"].map(b => document.querySelector('.rr-form[data-rr-name="Form" i] .rr-form-btn-' + b).disabled).join(",")`,
    webExpect: "false,true,false" },
  { name: "message_dialogs", kernel: true, events: "", dump: "lbl.caption", expect: ["lbl.caption=shown"],
    web: false, why: "the browser's alert / confirm dialogs block the page the test reads" },
  // (the page's own dialog: its caption, the icon left of the text, the
  // buttons with their mnemonics)
  { name: "message_icons", kernel: true, events: "", dump: "lbl.caption", expect: ["lbl.caption=asked"],
    webCheck: `[...document.querySelectorAll(".rr-dialog")].map((d) => [d.querySelector(".rr-dialog-title")?.textContent, d.querySelector(".rr-dialog-icon")?.dataset.icon,
      d.querySelector(".rr-dialog-icon svg polygon") ? "svg" : "-", [...d.querySelectorAll(".rr-dialog-button")].map((b) => b.textContent + (b.querySelector("u")?.textContent || "")).join(",")].join("|")).join(";")`,
    webExpect: "Files|Question|svg|YesY,NoN" },
  { name: "menus", kernel: true, events: "expert.onclick,newitem.onclick", dump: "lbl.caption,beg.checked,expert.checked", expect: ["lbl.caption=new0154Ctrl+N", "beg.checked=0", "expert.checked=1"],
    webCheck: `[...document.querySelectorAll('nav[data-rr-type="RMAINMENU"] .rr-menu-item-sub')].map(e => e.querySelector('.rr-menu-mark').textContent + e.querySelector('.rr-menu-text').textContent + e.querySelector('.rr-menu-keys').textContent + (e.classList.contains('rr-menu-disabled') ? '!' : '')).join('|') + ' ' + document.querySelectorAll('nav .rr-menu-sep').length`,
    webExpect: "NewCtrl+N|Beginner|●Expert|Exit! 1" },
  { name: "modal_result", kernel: true, events: "ed.__key_65,okbtn.onclick,nobtn.onclick", dump: "lbl.caption", expect: ["lbl.caption=17OKe"] },
  { name: "input_chars", kernel: true, events: "lbl.__key_65,lbl.__key_66,lbl.__key_67", dump: "lbl.caption", expect: ["lbl.caption=[abc]"] },
  { name: "inherit_event", kernel: true, events: "c.onclick,plain.onclick,btn.onclick", dump: "lbl.caption", expect: ["lbl.caption=own mine1 own | 1"] },
  { name: "text_edits", kernel: true, events: "btn.onclick", dump: "lbl.caption,ed.text",
    expect: ["lbl.caption=2two|ell|hEYo|3|two|ONE|2|8|-1|1", "ed.text=hEYo"],
    webCheck: `JSON.stringify([document.querySelector('[data-rr-name="Ed" i]').value, document.querySelector('[data-rr-name="Re" i]').value])`,
    webExpect: '["hEYo","ONE\\n2\\n"]' },
  { name: "trackbar", kernel: true, events: "tb.__key_39,tb.__key_34,tb.__key_36,tb.__mousedown_190_10,tb.__key_35,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=102111|10|4,6,0,2,10,|4"],
    webCheck: `["Tb", "Vt"].map(n => document.querySelectorAll('[data-rr-name="' + n + '" i] svg polyline').length).join(",")`,
    webExpect: "5,6" },
  { name: "tab_control", kernel: true, events: "tab.__key_39,tab.__key_39,tab.__mousedown_8_10,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=0Tab 2340200|1FirstTab 2|2Tab 2,0First,10|4332196"],
    webCheck: `[...document.querySelectorAll('[data-rr-name="Tab" i] .rr-tab-back text')].map(t => t.textContent).join(",")`,
    webExpect: "First,Tab 2,Tab 1" },
  { name: "autoscroll", kernel: true, events: "box.__mousedown_140_90,box.__mouseup_140_90,box.__mousedown_102_90,box.__mouseup_102_90,box.__mousedown_50_30,box.__mouseup_50_30,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=301209425290-1|14679300|216184-96|881121|020096"] },
  { name: "onshow_scroll", kernel: true, events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=show;333x283"] },
  { name: "form_visible", kernel: true, events: "btn.onclick,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=start 0-10;-1-1-110;-1-1-111;"] },
  { name: "screen_scale", kernel: true, events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=96 -1 -1"] },
  { name: "anchors", kernel: true, events: "ok.onclick", dump: "a.caption,b.caption,c.caption", resize: "250,180",
    expect: ["a.caption=300,230|200|300|200|150|400x300|12|3|0|300|200|15", "b.caption=200,130|100|250|150|100|300x200",
      "c.caption=400,330|300|400|300|200|500x400 300,330|150|300|200|150|400x400 250,150 200,100,200 200,330|50|200|100|100|300x400|10"] },
  { name: "font_size", kernel: true, events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=36x18 30x15"],
    // (web: a 12-point label's text is 16 pixels, as on the desktop)
    webCheck: `getComputedStyle(document.getElementById("rr-l")).fontSize`, webExpect: "16px" },
  { name: "nested_modal", kernel: true, events: "btn.onclick", dump: "lbl.caption,lbl2.caption",
    expect: ["lbl.caption=open;timer-close;closed;", "lbl2.caption=ticking"] },
  // (Stage 10: the IDE's components)
  { name: "design_surface", kernel: true,
    events: "ds.__mousedown_30_30,ds.__mousemove_43_36,ds.__mouseup_43_36,ds.__mousedown_113_47,ds.__mousemove_130_60,ds.__mouseup_130_60,ds.__mousedown_250_100,ds.__mouseup_250_100,ds.__dblclick_150_20,btn.onclick",
    dump: "lbl.caption,log.caption",
    expect: ["lbl.caption=4|Main|Label1|RCHECKBOX|Button1||3|Tick|32,24,96,40|208|&H00FFFF|Label1|Other|300", "log.caption=s0/m0:32,24,80,24/m0:32,24,96,40/b250,100/s2/d2/"],
    web: false, why: "the web draws RDESIGNSURFACE as a plain panel: no designer drawing or mouse there yet (its model answers)" },
  { name: "code_editor", kernel: true, events: "btn.onclick", dump: "lbl.caption",
    expect: ['lbl.caption=7|SUB Hello(x AS INTEGER)|118|0|Hello/Twice|4,70|  PRINT|  BEEP "hi" \' greet|37|33|0'] },
  // (`comp.__dblclick_x_y`: a double click at (x, y) in it — press, release,
  // press, release)
  { name: "dbl_clicks", kernel: true, events: "pn.__dblclick_5_5,lb.__dblclick_3_3,gb.__dblclick_10_30,img.__dblclick_2_2,cv.__dblclick_4_4,form.__dblclick_300_250", dump: "lbl.caption",
    expect: ["lbl.caption=pd5pcpu pDpd5pu ldlclu lDldlu gdgcgu gDgdgu idiciu iDidiu cdcccu cdcccu fdfcfu fDfdfu "] },
  // (a click on the selected node / item, then the double-click time —
  // a dozen events of nothing, 50 ms each — then "Renamed" and Enter;
  // a double click instead: no edit)
  { name: "pause_edit", kernel: true, events: "tv.__node_1,tv.__node_1,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,tv.__enter,lv.__mousedown_10_31,lv.__mouseup_10_31,lv.__mousedown_10_31,lv.__mouseup_10_31,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lv.__enter,lv.__mousedown_10_48,lv.__mouseup_10_48,lv.__dblclick_10_48,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lv.__enter,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=ing1 ed1:Renamed c0:2 c0:0 c0:2 c1:2 dbl |Renamed Renamed two"] },
  // (QSTATUSBAR's size grip dragged 50 across, 40 down; then a press on
  // the bar, and on the corner once SizeGrip is off)
  { name: "size_grip", kernel: true, events: "bar.__mousedown_306_18,bar.__mousemove_356_58,bar.__mouseup_356_58,bar.__mousedown_100_10,bar.__mouseup_100_10,btn.onclick,bar.__mousedown_356_18,bar.__mouseup_356_18", dump: "lbl.caption,form.width,form.height",
    expect: ["lbl.caption=w318 g-1 r370x280 d100 off d356 ", "form.width=370", "form.height=280"], web: false,
    why: "the browser's forms aren't resized by the user (no frame drag, so no size grip)" },
  // (its accessibility tree and keys: tests/web_a11y.mjs)
  { name: "a11y_form", kernel: true, events: "", dump: "lbl.caption", expect: ["lbl.caption=ready"] },
];
