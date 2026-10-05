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
//   webKernel: "pending: why" — the case doesn't run on the web's kernel
//             host yet (RAPIDR_WEB_HOST=kernel, Stage W3); every other
//             browser case must
//   webClick: { comp: "css selector" } — where a browser click lands for a
//             component whose clicks go through its rows / cells (the
//             desktop test hook fires the handler directly)
//   pixels: [[x, y, "rrggbb"], …] — what the desktop's capture of the
//           first window shows at (x, y) of its client area (logical
//           pixels; `clientWidth`, the window's ClientWidth, tells the
//           capture's scale — RAPIDR_SCALE, or a real screen's)

export const cases = [
  { name: "oop_events", events: "b1.onclick,b1.onclick,b2.onclick,b3.onclick", dump: "b1.caption,b2.caption,b3.caption",
    expect: ["b1.caption=Clicked 2", "b2.caption=Clicked 1", "b3.caption=Sender works"] },
  { name: "component_array_events", events: "btn(2).onclick,btn(3).onclick,btn(3).onclick", dump: "btn(1).caption,btn(2).caption,btn(3).caption",
    expect: ["btn(1).caption=Button1", "btn(2).caption=Hit Button2", "btn(3).caption=Hit Hit Button3"] },
  { name: "statusbar_panels", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=Ready|Line 42|INS|3|150"] },
  { name: "listview_columns", events: "lv.__mousedown_20_67,lv.__mouseup_20_67", dump: "lbl.caption",
    expect: ["lbl.caption=2|photo.jpg|Deflated|5|3|200|Method"] },
  { name: "listview_views", events: "lv.__mousedown_45_31,lv.__mouseup_45_31,lv.__mousedown_8_49,lv.__mouseup_8_49,lv.__key_40,lv.__key_32,lv.__mousedown_150_10,lv.__mouseup_150_10,btn2.onclick,lv.__mousedown_190_20,lv.__mouseup_190_20,lv.__edit,lv.__enter,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=c0:2 k c1:2 k c0:2 c1:2 c1:2 h1 c1:2 c2:2 k c2:0 | 2 0 Apple 0 2 0"] },
  { name: "grid_moving", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=5 3 H1a1 40 R2 3"] },
  { name: "string_grid", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=P2|P1|Lima|6|3|64|P1|Lima|4|41|-1"] },
  { name: "align_layout", events: "btn.onclick", dump: "loose.caption,side.caption,bar.caption,status.simpletext", resize: "600,350", split: "split:60",
    expect: ["loose.caption=105,40,233,205|100|245", "side.caption=moved160|160|165", "bar.caption=600x350|373x255|538|150", "status.simpletext=433|255|5|598"] },
  { name: "list_items", events: "items.onclick", dump: "summary.caption,lbl.caption",
    expect: ["summary.caption=5|zero|four|a/b & c|3|Applepear|2|2", "lbl.caption=picked 3 four"] },
  { name: "picture_resource", events: "img.onclick,img.onclick", dump: "summary.caption,lbl.caption",
    expect: ["summary.caption=1|20x10|0000FF00|00FF0000|0080FFFF|40|000000FF|00FFFFFF|-1", "lbl.caption=click;click;"] },
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
    expect: ["lk.caption=fd65 d65,0 fpa p97 u65 fd13 d13,0 fp\r p13 u13 fd38 d38,0 u38 ", "lm.caption=down010200 move11210 up01222 panel34"] },
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
    expect: ["lbl.caption=48 1 0000D4FF 00FFFFFF"] },
  { name: "outline", events: "outline.__toggle_3,outline.__node_4,btn.onclick", dump: "lbl.caption,outline.row",
    expect: ["lbl.caption=6 First Child of Parent 2 2", "outline.row=4"] },
  // (`colorDialog`: what each colour dialog answers in turn, `;`-separated
  // — a colour (decimal, one of the basic swatches for the browser) for
  // OK, empty for Cancel: RAPIDR_TEST_COLOR_DIALOG on the desktop, the page
  // dialog clicked in the browser)
  { name: "color_dialog", events: "b1.onclick,b2.onclick", dump: "lbl.caption,lbl2.caption", colorDialog: "255;",
    expect: ["lbl.caption=00000000|00000002|00000080|00FF00FF", "lbl2.caption=ok 000000FF 00123456|cancel 000000FF"] },
  // (`fontDialog`: likewise, `Name,Size,styles (b i u s),colour`)
  { name: "font_dialog", events: "b1.onclick,b2.onclick", dump: "lbl.caption,lbl2.caption", fontDialog: "Courier New,14,bu,255;",
    expect: ["lbl.caption=Arial|10|8|Courier New|Times New Roman12", "lbl2.caption=ok Courier New14 -10-1 000000FF|cancel Courier New"] },
  // (QFORM.WindowState: maximize, restore, minimize; OnResize counted by a
  // later click)
  { name: "window_state", headlessOnly: "a real window manager animates (macOS: ~40 OnResize) or answers later (GNOME's restore), and Wayland never tells a window where it is",
    events: "b1.onclick,b4.onclick,b2.onclick,b4.onclick,b3.onclick,b4.onclick", dump: "lbl.caption,lbl2.caption,lbl3.caption",
    expect: ["lbl.caption=2 -1-1-1|0 300x200 -1-1", "lbl2.caption=1 300 1|0 300", "lbl3.caption=1;2;2;"] },
  { name: "file_dialogs", events: "b1.onclick,b2.onclick,b3.onclick", dump: "lbl.caption,lbl2.caption,lbl3.caption", fileDialog: "notes;b.txt",
    expect: ["lbl.caption=open notes", "lbl2.caption=save notes.txt", "lbl3.caption=2 notes b.txt "] },
  { name: "header", events: "header.__mousedown_20_5,header.__mouseup_20_5,header.__mousedown_120_5,header.__mouseup_120_5,header.__mousedown_100_5,header.__mousemove_140_5,header.__mouseup_140_5,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=c0 t0:100:0 t0:140:1 t0:140:2 r0 | 3 140 Chart 0000FF00"] },
  { name: "icons", events: "", dump: "lbl.caption,img.width,img2.width", expect: ["lbl.caption=16x16 00C85A14", "img.width=16", "img2.width=16"],
    // (web: each form's title bar icon — its own, else the application's — and the page's)
    webCheck: `(() => { const src = (n) => { const i = document.querySelector('.rr-form[data-rr-name="' + n + '"] .rr-form-icon'); return i && i.style.display !== "none" ? i.src : ""; };
      return [src("FORM").startsWith("data:image/png"), src("OTHER").startsWith("data:image/png"), src("FORM") !== src("OTHER"), !!document.querySelector("link[rel~='icon'][href^='data:image/png']")].join(","); })()`,
    webExpect: "true,true,true,true" },
  { name: "option_icon", events: "", dump: "lbl.caption", expect: ["lbl.caption=-1"],
    webCheck: `(() => { const i = document.querySelector('.rr-form[data-rr-name="FORM"] .rr-form-icon'); return !!i && i.style.display !== "none" && i.src.startsWith("data:image/png"); })()`,
    webExpect: true },
  { name: "doevents_loop", events: "", dump: "lbl.caption", expect: ["lbl.caption=ticked -1 -1"] },
  { name: "inkey_wait", events: "lbl.__key_65,lbl.__key_38", dump: "lbl.caption", expect: ["lbl.caption=97/1 27/2"] },
  { name: "inkey_trapall", events: "lbl.__key_16,lbl.__key_65", dump: "lbl.caption", expect: ["lbl.caption=27:42 97:97 "] },
  { name: "grid_draw_hidden", events: "", dump: "lbl.caption", expect: ["lbl.caption=before=0 mark=m"] },
  { name: "border_icons", events: "", dump: "form.bordericons", expect: ["form.bordericons=11"],
    webCheck: `["min", "max", "close"].map(b => document.querySelector('.rr-form[data-rr-name="Form" i] .rr-form-btn-' + b).disabled).join(",")`,
    webExpect: "false,true,false" },
  { name: "message_dialogs", events: "", dump: "lbl.caption", expect: ["lbl.caption=shown"],
    web: false, why: "the browser's alert / confirm dialogs block the page the test reads" },
  // (the page's own dialog: its caption, the icon left of the text, the
  // buttons with their mnemonics)
  { name: "message_icons", events: "", dump: "lbl.caption", expect: ["lbl.caption=asked"],
    webCheck: `[...document.querySelectorAll(".rr-dialog")].map((d) => [d.querySelector(".rr-dialog-title")?.textContent, d.querySelector(".rr-dialog-icon")?.dataset.icon,
      d.querySelector(".rr-dialog-icon svg polygon") ? "svg" : "-", [...d.querySelectorAll(".rr-dialog-button")].map((b) => b.textContent + (b.querySelector("u")?.textContent || "")).join(",")].join("|")).join(";")`,
    webExpect: "Files|Question|svg|YesY,NoN" },
  { name: "menus", events: "expert.onclick,newitem.onclick", dump: "lbl.caption,beg.checked,expert.checked", expect: ["lbl.caption=new0154Ctrl+N", "beg.checked=0", "expert.checked=1"],
    webCheck: `[...document.querySelectorAll('nav[data-rr-type="RMAINMENU"] .rr-menu-item-sub')].map(e => e.querySelector('.rr-menu-mark').textContent + e.querySelector('.rr-menu-text').textContent + e.querySelector('.rr-menu-keys').textContent + (e.classList.contains('rr-menu-disabled') ? '!' : '')).join('|') + ' ' + document.querySelectorAll('nav .rr-menu-sep').length`,
    webExpect: "NewCtrl+N|Beginner|●Expert|Exit! 1" },
  { name: "modal_result", events: "ed.__key_65,okbtn.onclick,nobtn.onclick", dump: "lbl.caption", expect: ["lbl.caption=17OKe"] },
  { name: "input_chars", events: "lbl.__key_65,lbl.__key_66,lbl.__key_67", dump: "lbl.caption", expect: ["lbl.caption=[abc]-1"] },
  { name: "inherit_event", events: "c.onclick,plain.onclick,btn.onclick", dump: "lbl.caption", expect: ["lbl.caption=own mine1 own | 1"] },
  { name: "text_edits", events: "btn.onclick", dump: "lbl.caption,ed.text",
    expect: ["lbl.caption=2two|ell|hEYo|3|two|ONE|2|8|-1|1", "ed.text=hEYo"],
    webCheck: `JSON.stringify([document.querySelector('[data-rr-name="Ed" i]').value, document.querySelector('[data-rr-name="Re" i]').value])`,
    webExpect: '["hEYo","ONE\\n2\\n"]' },
  { name: "trackbar", events: "tb.__key_39,tb.__key_34,tb.__key_36,tb.__mousedown_190_10,tb.__key_35,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=102111|10|4,6,0,2,10,|4"],
    webCheck: `["Tb", "Vt"].map(n => document.querySelectorAll('[data-rr-name="' + n + '" i] svg polyline').length).join(",")`,
    webExpect: "5,6" },
  { name: "tab_control", events: "tab.__key_39,tab.__key_39,tab.__mousedown_8_10,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=0Tab 2340200|1FirstTab 2|2Tab 2,0First,10|4332196"],
    webCheck: `[...document.querySelectorAll('[data-rr-name="Tab" i] .rr-tab-back text')].map(t => t.textContent).join(",")`,
    webExpect: "First,Tab 2,Tab 1" },
  { name: "autoscroll", events: "box.__mousedown_140_90,box.__mouseup_140_90,box.__mousedown_102_90,box.__mouseup_102_90,box.__mousedown_50_30,box.__mouseup_50_30,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=301209425290-1|14679300|216184-96|881121|020096"] },
  { name: "onshow_scroll", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=show;333x283"] },
  { name: "form_visible", events: "btn.onclick,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=start 010;11110;11111;"] },
  { name: "screen_scale", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=96 -1 -1"] },
  { name: "anchors", events: "ok.onclick", dump: "a.caption,b.caption,c.caption", resize: "250,180",
    expect: ["a.caption=300,230|200|300|200|150|400x300|12|3|0|300|200|15", "b.caption=200,130|100|250|150|100|300x200",
      "c.caption=400,330|300|400|300|200|500x400 300,330|150|300|200|150|400x400 250,150 200,100,200 200,330|50|200|100|100|300x400|10"] },
  { name: "font_size", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=36x18 30x15"],
    // (web: a 12-point label's text is 16 pixels, as on the desktop)
    webCheck: `getComputedStyle(document.getElementById("rr-l")).fontSize`, webExpect: "16px" },
  { name: "nested_modal", events: "btn.onclick", dump: "lbl.caption,lbl2.caption",
    expect: ["lbl.caption=open;timer-close;closed;", "lbl2.caption=ticking"] },
  // (Stage 10: the IDE's components)
  { name: "design_surface",
    events: "ds.__mousedown_30_30,ds.__mousemove_43_36,ds.__mouseup_43_36,ds.__mousedown_113_47,ds.__mousemove_130_60,ds.__mouseup_130_60,ds.__mousedown_250_100,ds.__mouseup_250_100,ds.__dblclick_150_20,btn.onclick",
    dump: "lbl.caption,log.caption",
    expect: ["lbl.caption=4|Main|Label1|RCHECKBOX|Button1||3|Tick|32,24,96,40|208|&H00FFFF|Label1|Other|300", "log.caption=s0/m0:32,24,80,24/m0:32,24,96,40/b250,100/s2/d2/"],
    web: false, why: "the web draws RDESIGNSURFACE as a plain panel: no designer drawing or mouse there yet (its model answers)" },
  { name: "code_editor", events: "btn.onclick", dump: "lbl.caption",
    expect: ['lbl.caption=7|SUB Hello(x AS INTEGER)|118|0|Hello/Twice|4,70|  PRINT|  BEEP "hi" \' greet|37|33|0'] },
  // (`comp.__dblclick_x_y`: a double click at (x, y) in it — press, release,
  // press, release)
  { name: "dbl_clicks", events: "pn.__dblclick_5_5,lb.__dblclick_3_3,gb.__dblclick_10_30,img.__dblclick_2_2,cv.__dblclick_4_4,form.__dblclick_300_250", dump: "lbl.caption",
    expect: ["lbl.caption=pd5pcpu pDpd5pu ldlclu lDldlu gdgcgu gDgdgu idiciu iDidiu cdcccu cdcccu fdfcfu fDfdfu "] },
  // (a click on the selected node / item, then the double-click time —
  // a dozen events of nothing, 50 ms each — then "Renamed" and Enter;
  // a double click instead: no edit)
  { name: "pause_edit", events: "tv.__node_1,tv.__node_1,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,tv.__enter,lv.__mousedown_10_31,lv.__mouseup_10_31,lv.__mousedown_10_31,lv.__mouseup_10_31,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lv.__enter,lv.__mousedown_10_48,lv.__mouseup_10_48,lv.__dblclick_10_48,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lv.__enter,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=ing1 ed1:Renamed c0:2 c0:0 c0:2 c1:2 dbl |Renamed Renamed two"] },
  // (QSTATUSBAR's size grip dragged 50 across, 40 down; then a press on
  // the bar, and on the corner once SizeGrip is off)
  { name: "size_grip", events: "bar.__mousedown_306_18,bar.__mousemove_356_58,bar.__mouseup_356_58,bar.__mousedown_100_10,bar.__mouseup_100_10,btn.onclick,bar.__mousedown_356_18,bar.__mouseup_356_18", dump: "lbl.caption,form.width,form.height",
    expect: ["lbl.caption=w318 g1 r370x280 d100 off d356 ", "form.width=370", "form.height=280"], web: false,
    why: "the browser's forms aren't resized by the user (no frame drag, so no size grip)" },
  // (its accessibility tree and keys: tests/web_a11y.mjs)
  { name: "a11y_form", events: "", dump: "lbl.caption", expect: ["lbl.caption=ready"] },
  // (timers during native menu tracking: `__hold_600`, a menu held open
  // 600 ms on the headless host — the timer ticks through the tracking
  // tick; what needs the pump inside it answers as the plan says)
  { name: "menu_hold_timers", headlessOnly: "`__hold_ms` makes the headless host hold its pump as a held native menu would; real windows have no such hook",
    events: "b1.onclick,form.__hold_600,b2.onclick", dump: "lbl.caption,dlg.__shown",
    expect: ["lbl.caption=pop;de;modal2;ask7;|-1", "dlg.__shown=0"],
    web: false, why: "a page's menus never hold its loop: the hold and its tracking tick are the desktop host's" },
  // (timers while a dialog waits — the interpreter serves each dialog's
  // wait as ShowModal's: `dialogHold`, every answered dialog shown and
  // waited for that many ms first; `messageDialog`: what each message box
  // answers in turn, by caption; `delay`: the script starts after the
  // program's dialogs, in seconds)
  { name: "dialog_timers", events: "", dump: "lbl.caption,form2.__shown",
    messageDialog: "No;Yes;OK;OK;OK;OK", fileDialog: "notes.txt", colorDialog: "255", fontDialog: "Courier New,14", dialogHold: 250, delay: 4,
    expect: ["lbl.caption=inner6-1;dlg7-1;shown-1;msgbox0-1;open notes.txt-1;color000000FF-1;font Courier New14-1;box1-1;modal2;both1-1;", "form2.__shown=0"],
    web: false, why: "the browser harness answers a dialog only after an event it fired, and has no hold; the page's own dialogs let the timers run (tests/web_ide_dialogs.mjs)" },
  // (the DirectX lane's: QDXSCREEN, QDXIMAGELIST, QDXTIMER — the screen at
  // (10, 10) shows its last Flip: blue Fill, the red corner, the sprite's
  // see-through white, the strip's two patterns)
  { name: "dx_screen", events: "dx.__mousedown_12_34,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=init surface t3 down01234 |16711680,255,65280,65535|16711680,255,65535,65280|10,160x100"],
    pixels: [[150, 100, "0000ff"], [15, 15, "ff0000"], [111, 11, "0000ff"], [118, 18, "ff0000"], [131, 41, "ffff00"], [139, 41, "00ff00"]], clientWidth: 200,
    webCheck: `(() => { const c = document.getElementById("rr-dx-screen"); const s = c.width / 160; const g = c.getContext("2d");
      return [[140, 90], [5, 5], [101, 1], [108, 8], [121, 31], [129, 31]].map(([x, y]) => [...g.getImageData(Math.floor((x + 0.5) * s), Math.floor((y + 0.5) * s), 1, 1).data.slice(0, 3)].map(v => v.toString(16).padStart(2, "0")).join("")).join(",")
        + " " + c.style.width + " " + getComputedStyle(c.parentElement).backgroundColor; })()`,
    webExpect: "0000ff,ff0000,0000ff,ff0000,ffff00,00ff00 160px rgb(0, 0, 0)" },
  // (the DirectX lane's, stage D1b: the default font, Rotate, View.*, a
  // screen put on a shown form, a hidden form's screen, FullScreen,
  // ActiveOnly; the rotated line and the late screen's blue in the capture,
  // Cursor and the full screen's 4:3 picture in the page)
  { name: "dx_more", events: "b1.onclick,b2.onclick,b3.onclick,b4.onclick,b5.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=tick rot255,0 view10.5,5000 parented late second120 full |10x13,18|0|wide0,-1"],
    pixels: [[30, 50, "ff0000"], [40, 40, "000000"], [140, 20, "0000ff"]], clientWidth: 240,
    webCheck: `(() => { const px = (id, x, y) => { const c = document.getElementById(id); const s = c.width / parseFloat(c.style.width); return [...c.getContext("2d").getImageData(Math.floor((x + 0.5) * s), Math.floor((y + 0.5) * s), 1, 1).data.slice(0, 3)].map(v => v.toString(16).padStart(2, "0")).join(""); };
      const full = document.getElementById("rr-dx3-screen");
      return [getComputedStyle(document.getElementById("rr-dx")).cursor, px("rr-dx-screen", 20, 40), px("rr-late-screen", 5, 5), Math.round(parseFloat(full.style.width) / parseFloat(full.style.height) * 100)].join(" "); })()`,
    webExpect: "none ff0000 0000ff 133" },
  // (the DirectX lane's, stage D2: QDXSOUND — no sound under the tests:
  // Playing and Position follow the clock)
  { name: "dx_sound", events: "b1.onclick,b2.onclick,b3.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=4000,8000,100,0,0 play-1 still-1 in stop0 kept 1000,16000 end0,0 |80,-30,0,dx_beep.wav"] },
  // (the DirectX lane's, stages D3-D4: a Direct3D scene drawn by the
  // software rasterizer — lit faces, Move, CameraLookAt)
  { name: "d3d_scene", events: "b1.onclick,b2.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=faces2 lit255,65280,0 turned230 look230,0"],
    pixels: [[80, 60, "e60000"], [5, 5, "000000"]], clientWidth: 160 },
  // (the DirectX lane's: a .X model — frame matrix, materials, a texture)
  { name: "d3d_xfile", events: "b1.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=faces2 verts8 255,16711680,65535,0 green65280,65280 tex16711680,65535"],
    pixels: [[20, 60, "0000ff"], [60, 60, "ffff00"], [105, 60, "0000ff"], [135, 60, "ffff00"], [80, 60, "000000"]], clientWidth: 160 },
  // (the DirectX lane's, stage D6: QDXJOYSTICK — RapidQ's Update / IsLeft …
  // / Button(n), RapidR's X / Buttons / POV and events; `joystick`: the
  // tests' gamepad, a step a read)
  // (the I/O lane's: QDOWNLOAD from the tests' slow local server —
  // tests/http_test_server.mjs — its timer ticking while LeechFile waits;
  // OutVar / State, OutFile / StateGauge, a 404)
  { name: "download", events: "b1.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=1 1850 100 [Line ]1850 ticked -1 | 1 100 1850 | 0 11 The Server doesn't know the file"] },
  // (the I/O lane's: QMIDI and QWAVE driven by their Timers' OnChange —
  // no MIDI output, a scripted recording input; the Wait clicks give them
  // the time to play to their ends)
  { name: "media", events: "b1.onclick," + Array(50).fill("b2.onclick").join(","), dump: "lbl.caption",
    expect: ["lbl.caption=0 Cannot find the specified  0 | 1 999 3 50 1 1 end 0 -1 0 | 8 11025 1 0 4 rec 300 1 1 300 8 played 0 -1"] },
  // (the I/O lane's: QVIDEO on a form, a Cinepak clip played to its end by
  // the clock through its Timer's OnChange; a window of its own's sizes;
  // left on frame 5's key frame — what the window shows: frame 4's pixels,
  // as an independent decoder gives them, the picture at 200 %)
  { name: "video_player", events: "b1.onclick," + Array(50).fill("b2.onclick").join(","), dump: "lbl.caption",
    expect: ["lbl.caption=0 Cannot find the specified  | 1 8 0 32x24 32x24 -1 3 3 1 1 end 0 0 -1 0 | 1 34x26 video_clip.tmp.avi 128 122x55 Clip 5 3"],
    pixels: [[23, 73, "101010"], [55, 89, "dc3c14"], [81, 115, "f0d7c8"]], clientWidth: 318,
    webCheck: `(() => { const c = document.getElementById("rr-v.screen"); if (!c) return "no canvas"; const k = c.width / parseFloat(c.style.width); const g = c.getContext("2d"); const p = (x, y) => Array.from(g.getImageData(Math.floor((x + 0.5) * k), Math.floor((y + 0.5) * k), 1, 1).data.slice(0, 3)).map((v) => v.toString(16).padStart(2, "0")).join(""); return [p(3, 3), p(35, 19), p(61, 45)].join(","); })()`,
    webExpect: "101010,dc3c14,f0d7c8" },
  { name: "dx_joystick", events: "b1.onclick,b2.onclick,b3.onclick", dump: "lbl.caption",
    joystick: "x=0,b=1,name=Pad;x=65535,y=0,b=2;y=65535,pov=9000,b=3,name=Pad;b=1;b=1,x=0;b=0,x=0;-",
    expect: ["lbl.caption=-1000-10 0-1-100-1 000-1-1-1 Pad,-1,32767,65535,3,9000 |down1 move0 up1 move32767 |0"] },
  // (kernel themes: a click switches to dark at run time — Application.Theme;
  // `themes`: the desktop also captures the form under each of these,
  // RAPIDR_THEME, without the events — the web keeps its own look and only
  // reads the names back)
  { name: "themes", events: "btndark.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=theme classic then dark"], themes: ["modern", "dark", "highcontrast"] },
];
