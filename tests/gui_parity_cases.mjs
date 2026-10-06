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
//                   events — RAPIDR_TEST_RESIZE / RAPIDR_TEST_SPLIT, on the
//                   desktop and in the browser
//   web:    false + why — the case can't be compared in a browser (the
//           web's GUI host is the UI kernel, as the desktop's: the same
//           test hooks play the events there, and each window's capture
//           and accessibility tree are compared with the desktop's)
//   webKernel: "pending: why" — the case doesn't run on the web's kernel
//             host yet; every other browser case must
//   webCheck / webExpect: a JS expression read in the browser's page once
//             the script has ended, and the value it must give — what the
//             page shows outside its windows' insides and accessibility
//             trees (the tray strip, a frame's title bar buttons, the
//             page's icon), which the captures don't compare
//   pixels: [[x, y, "rrggbb"], …] — what the desktop's capture of the
//           first window shows at (x, y) of its client area (logical
//           pixels; `clientWidth`, the window's ClientWidth, tells the
//           capture's scale — RAPIDR_SCALE, or a real screen's)

export const cases = [
  // RapidR Studio's panels (I1 / L-PANELS, rapidr_value::panels): each made,
  // placed and drawn by the kernel (the scaffold's check; each panel has its own case).
  { name: "panels_smoke", events: "", dump: "lbl.caption", expect: ["lbl.caption=2800"] },
  // RPROPERTYINSPECTOR (I1 / L-PANELS, rapidr_value::panels::inspector): a
  // QBUTTON inspected — Caption typed ("Renamed", Enter), Default's box
  // ticked, Cursor picked from its dropped list (Down, Down, Enter),
  // Anchors found by typing "anc", opened, its top pin turned off by Space
  // and its right pin on by a click, Cursor put back by Delete; the Events
  // page: OnClick double-clicked, its SUB picked from the list of those that
  // fit; the form inspected (QFORM), then both buttons (their Captions
  // differ: blank); the search "cap"; the columns' line dragged 30 right.
  { name: "panel_inspector", events: "insp.__mousedown_200_148,insp.__mouseup_200_148,insp.__enter,insp.__mousedown_135_368,insp.__mouseup_135_368,insp.__mousedown_60_192,insp.__mouseup_60_192,insp.__mousedown_270_192,insp.__mouseup_270_192,insp.__key_40,insp.__key_40,insp.__key_13,insp.__key_65,insp.__key_78,insp.__key_67,insp.__key_39,insp.__key_40,insp.__key_32,insp.__mousedown_196_352,insp.__mouseup_196_352,insp.__key_67,insp.__key_85,insp.__key_46,insp.__mousedown_100_46,insp.__mouseup_100_46,insp.__dblclick_60_104,insp.__mousedown_285_104,insp.__mouseup_285_104,insp.__key_40,insp.__key_13,bform.onclick,breport.onclick,bboth.onclick,insp.__mousedown_40_46,insp.__mouseup_40_46,insp.__mousedown_100_76,insp.__mouseup_100_76,insp.__key_67,insp.__key_65,insp.__key_80,insp.__mousedown_122_126,insp.__mousemove_152_126,insp.__mouseup_152_126,breport.onclick",
    dump: "log.caption",
    expect: ["log.caption=sel Caption | change Caption=Renamed | sel Default | change Default=True | sel Cursor | change Cursor=crArrow | sel Align | sel Anchors | change Anchors=akLeft | change Anchors=akLeft + akRight | sel Caption | sel Cursor | change Cursor=crDefault | sel OnClick | dbl OnClick | change OnClick=Button1Click | [QFORM caption=Inspector anchors=akLeft, akTop-1 page=events filter= rows=14 OnClose= nw=120] | [QBUTTON caption= anchors=0 page=properties filter=cap rows=2 Caption= nw=150] | "] },
  // RPROJECTTREE (I1 / L-PANELS, rapidr_value::panels::project_tree): a
  // project given as text (LoadText, SetFileText). Form1.rr opened by its
  // chevron, Button1 double-clicked (OnSelect, OnOpen "Form1.rr#Button1"),
  // Utils.rr renamed in place (F2, "tools" typed, Enter: OnRename),
  // Report.rr deleted (Delete, Enter on the strip's Remove: OnDelete, the
  // selection moves on), tools.rr dragged above Main.rr and About.rr out of
  // its folder onto the Forms group (OnMove), Main.rr renamed by the
  // program (OnRename's Cancel refuses), a new form named by typing
  // (OnNewFile); the files in their order, Modified, ProjectText.
  { name: "panel_project", events: "tree.__mousedown_45_103,tree.__mouseup_45_103,tree.__dblclick_150_169,tree.__mousedown_120_279,tree.__mouseup_120_279,tree.__key_113,tree.__key_84,tree.__key_79,tree.__key_79,tree.__key_76,tree.__key_83,tree.__key_13,tree.__mousedown_120_301,tree.__mouseup_120_301,tree.__key_46,tree.__key_13,tree.__mousedown_120_279,tree.__mousemove_120_268,tree.__mousemove_120_250,tree.__mouseup_120_250,tree.__mousedown_120_81,tree.__mousemove_120_70,tree.__mousemove_120_37,tree.__mouseup_120_37,brename.onclick,bnew.onclick,tree.__key_68,tree.__key_73,tree.__key_65,tree.__key_76,tree.__key_79,tree.__key_71,tree.__key_13,breport.onclick",
    dump: "lbl.caption,info.caption",
    expect: ["lbl.caption= S:Form1.rr#Button1 O:Form1.rr#Button1 S:Utils.rr R:Utils.rr>tools.rr S:Report.rr D:Report.rr S: S:tools.rr M:tools.rr>tools.rr@0 S:forms/About.rr M:forms/About.rr>About.rr@3 R:Main.rr>Summary.rr new:Form2.rr N:dialog.rr/form",
      "info.caption=Inventory: tools.rr Main.rr Form1.rr About.rr lib/strings.inc data/stock.csv dialog.rr |-1-1 dialog.rr form"] },
  // RPROJECTTREE reading files (Project =): an .rrproj and an implicit
  // project ($INCLUDEs followed); the keyboard (Down, Right, End, Left,
  // type-ahead, Enter: OnOpen), Reveal of a component.
  { name: "panel_project_files", events: "t1.__key_40,t1.__key_40,t1.__key_40,t1.__key_39,t1.__key_39,t1.__key_35,t1.__key_37,t1.__key_77,t1.__key_40,t1.__key_13,breport.onclick",
    dump: "lbl.caption,info.caption",
    expect: ["lbl.caption= S: S: S:panel_project_form.rr S:panel_project_form.rr#Form1 S:panel_project_util.inc S: S: S:panel_project_main.rr O:panel_project_main.rr | panel_project_main.rr | panel_project_form.rr#Button1 0",
      "info.caption=Demo3: panel_project_main.rr/module panel_project_form.rr/form panel_project_util.inc/include | panel_project_main3: panel_project_main.rr/module panel_project_util.inc/include panel_project_form.rr/form"] },
  // (L-PANELS D) ROUTPUTCONSOLE: ANSI colours, CLS, LOCATE written; a link
  // clicked in the output, the Build tab and its compiler message's link,
  // the Problems tab and a problem, the Output tab; Find / FindNext, F3.
  // RTOOLBAR: Save, the disabled Stop (nothing), the Grid toggle, a
  // QCOOLBTN on it, the strip; the narrow bar's "»" and Redo in its menu,
  // then its menu's "Cut" line (hidden: Layout); the mouse over Open.
  { name: "panel_console", events: "cons.__mousedown_70_72,cons.__mouseup_70_72,cons.__mousedown_88_14,cons.__mouseup_88_14,cons.__mousedown_40_55,cons.__mouseup_40_55,cons.__mousedown_140_14,cons.__mouseup_140_14,cons.__mousedown_200_61,cons.__mouseup_200_61,cons.__mousedown_30_14,cons.__mouseup_30_14,bfind.onclick,cons.__key_114,breport.onclick",
    dump: "lbl.caption,info.caption",
    expect: ["lbl.caption= link:Main.rr:12 page:build link:src/app.bas:3 page:problems link:src/app.bas:3 page:output find:3 next:2",
      "info.caption=output 7 2 [RapidR output console] [           LOCATE 6, 12] line"] },
  { name: "panel_toolbar", events: "bar.__mousedown_74_16,bar.__mouseup_74_16,bar.__mousedown_142_16,bar.__mouseup_142_16,bar.__mousedown_181_16,bar.__mouseup_181_16,bar.__mousedown_230_16,bar.__mouseup_230_16,bar.__mousedown_400_16,bar.__mouseup_400_16,bar2.__mousedown_125_16,bar2.__mouseup_125_16,bar2.__mousedown_102_37,bar2.__mouseup_102_37,bar2.__mousedown_125_16,bar2.__mouseup_125_16,bar2.__mousedown_102_63,bar2.__mouseup_102_63,breport.onclick,bar.__mousemove_45_16",
    dump: "lbl.caption,info.caption",
    expect: ["lbl.caption= save/file.save click:save grid/designer.showGrid click:grid cool click: redo/edit.redo", "info.caption=8 - -1 0 [cut] Open a file4"] },
  // RDOCKMANAGER (I1, rapidr_value::dock): an IDE's layout. A tab clicked
  // (Output), the Explorer's splitter dragged 40 to the right, the
  // Toolbox's strip tab clicked twice (slid out, in), Properties dragged
  // by its header onto the compass's bottom arm over Explorer, Output
  // moved by the keyboard (MovePane, Left, Enter: left of its group), F6
  // in Properties (the next area: Output); SaveLayout / changes / LoadLayout give the
  // same text; tabbed documents, both closed (OnDocumentClose's Cancel
  // keeps Doc1), Properties floated (its window). Pixels: Explorer's inactive classic title bar, a gutter.
  { name: "dock_manager", events: "dock.__mousedown_300_352,dock.__mouseup_300_352,dock.__mousedown_265_200,dock.__mousemove_285_200,dock.__mousemove_305_200,dock.__mouseup_305_200,dock.__mousedown_12_40,dock.__mouseup_12_40,dock.__mousedown_12_40,dock.__mouseup_12_40,dock.__mousedown_700_12,dock.__mousemove_710_20,dock.__mousemove_164_296,dock.__mouseup_164_296,bmove.onclick,dock.__key_37,dock.__key_13,props.__key_117,bsave.onclick,btabs.onclick,breport.onclick",
    dump: "lbl.caption,info.caption",
    expect: ["lbl.caption=- p:output L p:props p:props L p:output L p:output p:explorer L p:props L [hidden autohide] L loaded same L c:doc1 c:doc2 a:doc1 p:doc2 L p:props L",
      "info.caption=tabs 6 explorer output doc1 | docked floating docked docked autohide document | 280x495 298x304"],
    pixels: [[150, 5, "808080"], [306, 200, "f0f0f0"]], clientWidth: 898 },
  // QGLASSFRAME: the default black glass over the form's face (60 % see-
  // through), red glass at 50 over a cyan panel; Moveable: the form
  // follows a drag on it (20, 10), a glass not Moveable doesn't; clicks.
  { name: "glass_frame", events: "g.__mousedown_20_20,g.__mousemove_40_30,g.__mouseup_40_30,r.__mousedown_5_5,r.__mousemove_25_15,r.__mouseup_25_15,g.onclick", dump: "lbl.caption,form.left,form.top",
    expect: ["lbl.caption=- click120110 click120110", "form.left=120", "form.top=110"],
    pixels: [[50, 50, "909090"], [160, 40, "808080"], [5, 5, "f0f0f0"], [200, 100, "00ffff"]], clientWidth: 358 },
  // QDOCKFORM built in (RAPIDQ2.INC's dockable form, RapidR's own
  // library): docked at its alternative place, floated, brought home,
  // closed (OnClose); the toolbar-style one's grip (the capture's pixels).
  { name: "dock_form", events: "b1.onclick,b2.onclick,b3.onclick,b4.onclick", dump: "lbl.caption,p.__shown",
    expect: ["lbl.caption=- r11 f00 h10 closed c1", "p.__shown=0"],
    pixels: [[6, 4, "808080"], [6, 5, "c0c0c0"]], clientWidth: 478 },
  // QDIRLISTVIEW built in (QDirListView.inc's component, RapidR's own
  // library): a folder the program made, a file picked and Enter
  // (OnFileSelect), a folder double-clicked (into it), Backspace (up).
  { name: "dir_list_view", events: "dirlist.__mousedown_30_58,dirlist.__mouseup_30_58,dirlist.__key_13,dirlist.__dblclick_30_40,btn.onclick,dirlist.__key_8,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=- pick:dirlv/notes.txt [-1>/Inner2 ../0 deep.bas/1 |] [-1>/Inner3 ../0 Inner/0 notes.txt/1 2 KB|TXT File]"],
    inWork: true, web: false, why: "a browser has no folders to list (DIR$)" },
  // The system tray (QNOTIFYICONDATA, Shell_NotifyIcon, the form's
  // WndProc): added once (a second NIM_ADD fails), its tip modified, the
  // form hidden (its ShowModal waits on); the icon's press and release heard
  // with wParam its uID; the release deletes it (a second NIM_DELETE fails)
  // and shows the form; a press after that says nothing.
  { name: "tray_icon", events: "btn.onclick,form.__tray_513,form.__tray_514,form.__tray_513", dump: "lbl.caption,form.__shown",
    expect: ["lbl.caption=add10 mod1 7:00000201f 7:00000202f del10", "form.__shown=1"],
    // (web: the page's tray strip, tray_web.rs — no icon left in it, hidden)
    webCheck: `document.querySelectorAll(".rr-tray-icon").length + " " + getComputedStyle(document.getElementById("rr-tray")).display`,
    webExpect: "0 none" },
  // QBEVEL and QDIGDISPLAY built in (no include library): Shape / Style
  // set the bevels or draw a line pair; the display's size and segments.
  // Pixels: the top line's light and dark rows, the right line's dark and
  // light columns, a lit segment (cyan), an unlit one's dither.
  { name: "bevel_display", events: "btn.onclick,edge.onclick,clock.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=-02021 12:34602460 00FFFF00|00008000|00000000cc"],
    pixels: [[50, 10, "ffffff"], [50, 11, "a0a0a0"], [328, 30, "a0a0a0"], [329, 30, "ffffff"], [23, 85, "00ffff"], [11, 77, "008000"], [11, 78, "000000"]], clientWidth: 358 },
  { name: "oop_events", events: "b1.onclick,b1.onclick,b2.onclick,b3.onclick", dump: "b1.caption,b2.caption,b3.caption",
    expect: ["b1.caption=Clicked 2", "b2.caption=Clicked 1", "b3.caption=Sender works"] },
  { name: "component_array_events", events: "btn(2).onclick,btn(3).onclick,btn(3).onclick", dump: "btn(1).caption,btn(2).caption,btn(3).caption",
    expect: ["btn(1).caption=Button1", "btn(2).caption=Hit Button2", "btn(3).caption=Hit Hit Button3"] },
  { name: "statusbar_panels", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=Ready|Line 42|INS|3|150"] },
  // (a click on the third row: under a 17-pixel header, rows 14 high —
  // Windows' classic list view in MS Sans Serif 8)
  { name: "listview_columns", events: "lv.__mousedown_20_57,lv.__mouseup_20_57", dump: "lbl.caption",
    expect: ["lbl.caption=2|photo.jpg|Deflated|5|3|200|Method"] },
  { name: "listview_views", events: "lv.__mousedown_45_31,lv.__mouseup_45_31,lv.__mousedown_8_49,lv.__mouseup_8_49,lv.__key_40,lv.__key_32,lv.__mousedown_150_10,lv.__mouseup_150_10,btn2.onclick,lv.__mousedown_190_20,lv.__mouseup_190_20,lv.__edit,lv.__enter,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=c0:2 k c1:2 k c0:2 c1:2 c1:2 h1 c1:2 c2:2 k c2:0 | 2 0 Apple 0 2 0"] },
  { name: "grid_moving", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=5 3 H1a1 40 R2 3"] },
  { name: "string_grid", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=P2|P1|Lima|6|3|64|P1|Lima|4|41|-1"] },
  { name: "align_layout", events: "btn.onclick", dump: "loose.caption,side.caption,bar.caption,status.simpletext", resize: "600,350", split: "split:60",
    expect: ["loose.caption=103,40,235,210|100|250", "side.caption=moved160|160|163", "bar.caption=600x350|375x260|538|150", "status.simpletext=435|260|5|598"] },
  { name: "list_items", events: "items.onclick", dump: "summary.caption,lbl.caption",
    expect: ["summary.caption=5|zero|four|a/b & c|3|Applepear|2|2", "lbl.caption=picked 3 four"] },
  { name: "picture_resource", events: "img.onclick,img.onclick", dump: "summary.caption,lbl.caption",
    expect: ["summary.caption=1|20x10|0000FF00|00FF0000|0080FFFF|40|000000FF|00FFFFFF|-1", "lbl.caption=click;click;"] },
  { name: "grid_draw_cell", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=round2:25|0,130,25,194,49,two|fixed4 selected3"] },
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
    expect: ["lbl.caption=paints3|form1|255|15790320|220x80|35"] },
  { name: "form_draw", events: "big.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=paints2|255|14737632|35|338"] },
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
    expect: ["lbl.caption=exp0 chg1 |8|Hill|31-10-1", "lbl2.caption=del4 del5 del6 5", "tv.itemindex=1"] },
  { name: "tree_images", events: "btn.onclick", dump: "lbl.caption", expect: ["lbl.caption=-1 -1 1 1 0 1 -1 -1"] },
  { name: "tree_edit", events: "tv.__node_0,tv.__edit,tv.__enter,tv.__node_2,tv.__edit,tv.__enter,tv.__node_1,tv.__edit,tv.__escape,ro.onclick,tv.__edit,tv.__enter,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=ing0 ed0:Renamed ing2 ing1 |RENAMED Pear Plum -1"] },
  { name: "panel_bevels", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=201 123 112"] },
  { name: "svg_picture", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=48 1 0000D4FF 00FFFFFF"] },
  { name: "outline", events: "outline.__toggle_3,outline.__node_4,btn.onclick", dump: "lbl.caption,outline.row",
    expect: ["lbl.caption=6 First Carrots 2", "outline.row=4"] },
  // (`colorDialog`: what each colour dialog answers in turn, `;`-separated
  // — a colour (decimal) for OK, empty for Cancel: RAPIDR_TEST_COLOR_DIALOG,
  // on the desktop and in the browser)
  { name: "color_dialog", events: "b1.onclick,b2.onclick", dump: "lbl.caption,lbl2.caption", colorDialog: "255;",
    expect: ["lbl.caption=00000000|00000002|00000080|00FF00FF", "lbl2.caption=ok 000000FF 00123456|cancel 000000FF"] },
  // (`fontDialog`: likewise, `Name,Size,styles (b i u s),colour`)
  { name: "font_dialog", events: "b1.onclick,b2.onclick", dump: "lbl.caption,lbl2.caption", fontDialog: "Courier New,14,bu,255;",
    expect: ["lbl.caption=MS Sans Serif|8|8|Courier New|Times New Roman12", "lbl2.caption=ok Courier New14 -10-1 000000FF|cancel Courier New"] },
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
    // (web: the application's icon is the page's)
    webCheck: `!!document.querySelector("link[rel~='icon'][href^='data:image/png']")`,
    webExpect: true },
  { name: "option_icon", events: "", dump: "lbl.caption", expect: ["lbl.caption=-1"] },
  { name: "doevents_loop", events: "", dump: "lbl.caption", expect: ["lbl.caption=ticked -1 -1"] },
  { name: "inkey_wait", events: "lbl.__key_65,lbl.__key_38", dump: "lbl.caption", expect: ["lbl.caption=97/1 27/2"] },
  { name: "inkey_trapall", events: "lbl.__key_16,lbl.__key_65", dump: "lbl.caption", expect: ["lbl.caption=27:42 97:97 "] },
  { name: "grid_draw_hidden", events: "", dump: "lbl.caption", expect: ["lbl.caption=before=0 mark=m"] },
  { name: "border_icons", events: "", dump: "form.bordericons", expect: ["form.bordericons=11"],
    // (web: the title bar the page draws, outside the window's capture —
    // each button's glyph, minimize / maximize / close, in the text's ink
    // or greyed; the title bar's metrics: the host's frame.rs, 28-pixel
    // buttons from the right edge of a 320-pixel window)
    webCheck: `(() => { const c = document.querySelector('.rr-kwin[data-rr-form="form"] canvas.rr-kframe'); const s = c.width / parseFloat(c.style.width); const g = c.getContext("2d");
      const ink = ([x, y]) => { const r = g.getImageData(Math.floor((x + 0.5) * s), Math.floor((y + 0.5) * s), 1, 1).data[0]; return r < 0x40 ? "ink" : r >= 0x60 && r <= 0xa0 ? "grey" : r.toString(16); };
      return [[247, 17], [275, 11], [304, 15]].map(ink).join(","); })()`,
    webExpect: "ink,grey,ink" },
  { name: "message_dialogs", events: "", dump: "lbl.caption", expect: ["lbl.caption=shown"] },
  // (a message box: its caption, the icon left of the text, the buttons
  // with their mnemonics)
  { name: "message_icons", events: "", dump: "lbl.caption", expect: ["lbl.caption=asked"] },
  { name: "menus", events: "expert.onclick,newitem.onclick", dump: "lbl.caption,beg.checked,expert.checked", expect: ["lbl.caption=new0154Ctrl+N", "beg.checked=0", "expert.checked=1"] },
  { name: "modal_result", events: "ed.__key_65,okbtn.onclick,nobtn.onclick", dump: "lbl.caption", expect: ["lbl.caption=17OKe"] },
  { name: "input_chars", events: "lbl.__key_65,lbl.__key_66,lbl.__key_67", dump: "lbl.caption", expect: ["lbl.caption=[abc]-1"] },
  { name: "inherit_event", events: "c.onclick,plain.onclick,btn.onclick", dump: "lbl.caption", expect: ["lbl.caption=own mine1 own | 1"] },
  { name: "text_edits", events: "btn.onclick", dump: "lbl.caption,ed.text",
    expect: ["lbl.caption=2two|ell|hEYo|3|two|ONE|2|8|-1|1", "ed.text=hEYo"] },
  { name: "trackbar", events: "tb.__key_39,tb.__key_34,tb.__key_36,tb.__mousedown_190_10,tb.__key_35,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=102111|10|4,6,0,2,10,|4"] },
  { name: "tab_control", events: "tab.__key_39,tab.__key_39,tab.__mousedown_8_10,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=0Tab 2340200|1FirstTab 2|2Tab 2,0First,10|4332196"] },
  { name: "autoscroll", events: "box.__mousedown_140_90,box.__mouseup_140_90,box.__mousedown_102_90,box.__mouseup_102_90,box.__mousedown_50_30,box.__mouseup_50_30,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=3012094252901|14679220|216184-96|741261|020096"] },
  { name: "onshow_scroll", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=show;333x283"] },
  { name: "form_visible", events: "btn.onclick,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=start 010;11110;11111;"] },
  { name: "screen_scale", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=96 -1 -1"] },
  { name: "anchors", events: "ok.onclick", dump: "a.caption,b.caption,c.caption", resize: "250,180",
    expect: ["a.caption=300,230|200|300|200|150|400x300|12|3|0|300|200|15", "b.caption=200,130|100|250|150|100|300x200",
      "c.caption=400,330|300|400|300|200|500x400 300,330|150|300|200|150|400x400 250,150 200,100,200 200,330|50|200|100|100|300x400|10"] },
  // (I4 L-DMODEL) The designer's resize preview is the running program:
  // crates/rapidr-designer/tests/anchors.rs reads this expect list and must
  // give the same rectangles from the CREATE block resized in the designer.
  { name: "designer_anchors", events: "", resize: "600,450", dump: "bar.left,bar.top,bar.width,bar.height,status.left,status.top,status.width,status.height,namelbl.left,namelbl.top,namelbl.width,namelbl.height,nameed.left,nameed.top,nameed.width,nameed.height,notes.left,notes.top,notes.width,notes.height,side.left,side.top,side.width,side.height,pick.left,pick.top,pick.width,pick.height,ok.left,ok.top,ok.width,ok.height,cancel.left,cancel.top,cancel.width,cancel.height,mid.left,mid.top,mid.width,mid.height",
    expect: ["bar.left=0", "bar.top=0", "bar.width=598", "bar.height=32", "status.left=0", "status.top=397", "status.width=598", "status.height=22", "namelbl.left=12", "namelbl.top=48", "namelbl.width=31", "namelbl.height=13", "nameed.left=64", "nameed.top=44", "nameed.width=420", "nameed.height=21", "notes.left=12", "notes.top=80", "notes.width=472", "notes.height=280", "side.left=496", "side.top=44", "side.width=92", "side.height=316", "pick.left=8", "pick.top=280", "pick.width=75", "pick.height=25", "ok.left=428", "ok.top=366", "ok.width=75", "ok.height=25", "cancel.left=512", "cancel.top=366", "cancel.width=75", "cancel.height=25", "mid.left=222", "mid.top=370", "mid.width=36", "mid.height=13"] },
  { name: "font_size", events: "btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=35x19 28x16"] },
  { name: "nested_modal", events: "btn.onclick", dump: "lbl.caption,lbl2.caption",
    expect: ["lbl.caption=open;timer-close;closed;", "lbl2.caption=ticking"] },
  // (Stage 10: the IDE's components)
  { name: "design_surface",
    events: "ds.__mousedown_30_30,ds.__mousemove_43_36,ds.__mouseup_43_36,ds.__mousedown_113_47,ds.__mousemove_130_60,ds.__mouseup_130_60,ds.__mousedown_250_100,ds.__mouseup_250_100,ds.__dblclick_150_20,btn.onclick",
    dump: "lbl.caption,log.caption",
    expect: ["lbl.caption=4|Main|Label1|RCHECKBOX|Button1||3|Tick|32,24,96,40|208|&H00FFFF|Label1|Other|300", "log.caption=s0/m0:32,24,80,24/m0:32,24,96,40/b250,100/s2/d2/"] },
  { name: "code_editor", events: "btn.onclick", dump: "lbl.caption",
    expect: ['lbl.caption=7|SUB Hello(x AS INTEGER)|118|0|Hello/Twice|4,70|  PRINT|  BEEP "hi" \' greet|37|33|0'] },
  // (`comp.__dblclick_x_y`: a double click at (x, y) in it — press, release,
  // press, release)
  { name: "dbl_clicks", events: "pn.__dblclick_5_5,lb.__dblclick_3_3,gb.__dblclick_10_30,img.__dblclick_2_2,cv.__dblclick_4_4,form.__dblclick_300_250", dump: "lbl.caption",
    expect: ["lbl.caption=pd5pcpu pDpd5pu ldlclu lDldlu gdgcgu gDgdgu idiciu iDidiu cdcccu cdcccu fdfcfu fDfdfu "] },
  // (a click on the selected node / item, then the double-click time —
  // a dozen events of nothing, 50 ms each — then "Renamed" and Enter;
  // a double click instead: no edit; the list view's rows 14 high under a
  // 17-pixel header)
  { name: "pause_edit", events: "tv.__node_1,tv.__node_1,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,tv.__enter,lv.__mousedown_10_31,lv.__mouseup_10_31,lv.__mousedown_10_31,lv.__mouseup_10_31,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lv.__enter,lv.__mousedown_10_40,lv.__mouseup_10_40,lv.__dblclick_10_40,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lbl.onclick,lv.__enter,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=ing1 ed1:Renamed c0:2 c0:0 c0:2 c1:2 dbl |Renamed Renamed two"] },
  // (QSTATUSBAR's size grip dragged 50 across, 40 down; then a press on
  // the bar, and on the corner once SizeGrip is off)
  { name: "size_grip", events: "bar.__mousedown_306_18,bar.__mousemove_356_58,bar.__mouseup_356_58,bar.__mousedown_100_10,bar.__mouseup_100_10,btn.onclick,bar.__mousedown_356_18,bar.__mouseup_356_18", dump: "lbl.caption,form.width,form.height",
    expect: ["lbl.caption=w318 g1 r370x280 d100 off d356 ", "form.width=370", "form.height=280"] },
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
    web: false, why: "timing: the browser's timers and frames make a step's first tick come too late now and then (most runs give the desktop's dump; the open file dialog's step is the one that misses) — its ticks are counted in tens of milliseconds" },
  // (the DirectX lane's: QDXSCREEN, QDXIMAGELIST, QDXTIMER — the screen at
  // (10, 10) shows its last Flip: blue Fill, the red corner, the sprite's
  // see-through white, the strip's two patterns)
  { name: "dx_screen", events: "dx.__mousedown_12_34,btn.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=init surface t3 down01234 |16711680,255,65280,65535|16711680,255,65535,65280|10,160x100"],
    pixels: [[150, 100, "0000ff"], [15, 15, "ff0000"], [111, 11, "0000ff"], [118, 18, "ff0000"], [131, 41, "ffff00"], [139, 41, "00ff00"]], clientWidth: 200 },
  // (the DirectX lane's, stage D1b: the default font, Rotate, View.*, a
  // screen put on a shown form, a hidden form's screen, FullScreen,
  // ActiveOnly; the rotated line, the late screen's blue and the full
  // screen's 4:3 picture in the captures)
  { name: "dx_more", events: "b1.onclick,b2.onclick,b3.onclick,b4.onclick,b5.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=tick rot255,0 view10.5,5000 parented late second120 full |10x13,18|0|wide0,-1"],
    pixels: [[30, 50, "ff0000"], [40, 40, "000000"], [140, 20, "0000ff"]], clientWidth: 240 },
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
    pixels: [[23, 73, "101010"], [55, 89, "dc3c14"], [81, 115, "f0d7c8"]], clientWidth: 318 },
  // (shown, never played: the frame it was sought to — frame 5's key
  // frame, 4, as video_player's window is left — on the window)
  { name: "video_show", events: "b1.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=1 3 5 0"],
    pixels: [[23, 73, "101010"], [55, 89, "dc3c14"], [81, 115, "f0d7c8"]], clientWidth: 318 },
  { name: "dx_joystick", events: "b1.onclick,b2.onclick,b3.onclick", dump: "lbl.caption",
    joystick: "x=0,b=1,name=Pad;x=65535,y=0,b=2;y=65535,pov=9000,b=3,name=Pad;b=1;b=1,x=0;b=0,x=0;-",
    expect: ["lbl.caption=-1000-10 0-1-100-1 000-1-1-1 Pad,-1,32767,65535,3,9000 |down1 move0 up1 move32767 |0"] },
  // (kernel themes: a click switches to dark at run time — Application.Theme;
  // `themes`: the desktop also captures the form under each of these,
  // RAPIDR_THEME, without the events — the web's run is compared with the
  // events' capture only)
  { name: "themes", events: "btndark.onclick", dump: "lbl.caption",
    expect: ["lbl.caption=theme classic then dark"], themes: ["modern", "dark", "highcontrast"] },
  // RPLOT on a form (the UI kernel's component, the one chart renderer): a
  // line chart anchored left / top / right, widened with the form (500 ×
  // 350); a bar chart aligned to the bottom. The click adds a dashed series
  // and a legend, titles the bars and renders: drawn again. Pixels: the
  // line chart's background, the form between the charts, a bar (steelblue),
  // the legend's red and blue swatches; on the web, the red line drawn on
  // the form's canvas (the window's capture is compared with the desktop's).
  { name: "rplot_on_form", events: "btn.onclick", dump: "lbl.caption", resize: "500,350",
    expect: ["lbl.caption=2 330x170 0,209 498x110 Sales 1"],
    pixels: [[12, 34, "ffffff"], [400, 100, "f0f0f0"], [420, 270, "4682b4"], [57, 85, "ff0000"], [60, 101, "0000ff"]], clientWidth: 498,
    webCheck: `(() => { const c = document.querySelector('.rr-kwin[data-rr-form="form"] canvas.rr-kclient'); const g = c && c.getContext("2d"); if (!g) return "no canvas";
      const d = g.getImageData(0, 0, c.width, c.height).data; let n = 0; for (let i = 0; i < d.length; i += 4) if (d[i] > 200 && d[i + 1] < 60 && d[i + 2] < 60) n++;
      return n > 200 ? "red line drawn" : "red pixels: " + n; })()`,
    webExpect: "red line drawn" },
];
