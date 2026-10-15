//! The design-time side of the interface (docs/ai-architecture.md §3.5),
//! through its JSON front door as a client (MCP, RAI) sees it.

use rapidr_introspect::design::DesignModel;
use rapidr_introspect::{handle_json, ErrorCode, Introspect, ListQuery, Reply, Source};
use serde_json::{json, Value};

const MAIN: &str = r#"' A notes program
$INCLUDE "RAPIDQ.INC"

DECLARE SUB SaveClick

CREATE Form1 AS RForm
    Caption = "Notes"
    Width = 320: Height = 240
    Color = clBtnFace
    CREATE Panel1 AS RPanel
        Align = alTop
        Height = 40
        CREATE Save AS RButton
            Caption = "Save"
            Left = 10: Top = 8
            Anchors = akLeft + akTop + akRight
            Font.Name = "Arial"
            Constraints.MinWidth = 60
            Visible = True
            OnClick = SaveClick
        END CREATE
    END CREATE
    CREATE Memo1 AS RMemo
        Align = alClient
        Width = Screen.Width / 2
        Color = &HFF0000
    END CREATE
END CREATE

CREATE DB AS RMySQL
    Host = "localhost"
    Password = "hunter2"
END CREATE

SUB SaveClick
END SUB

Form1.ShowModal
"#;

// Another file of the project, in RapidQ's names and without RAPIDQ.INC:
// `clRed` isn't defined there (RapidQ reads it as an empty variable); a
// second `Save` (another file) and two `Twin`s (one file) make names
// ambiguous.
const SETTINGS: &str = r#"CREATE Settings AS QFORM
    Caption = "Settings"
    Color = clRed
    CREATE Save AS QBUTTON
        Caption = "OK"
    END CREATE
END CREATE

CREATE Twin AS QFORM
END CREATE

CREATE Twin AS QFORM
    Caption = "second"
END CREATE
"#;

fn model() -> DesignModel {
    DesignModel::in_memory(&[("main.rr", MAIN), ("settings.inc", SETTINGS)])
}

/// Sends a request as JSON; the reply's result, or its error code.
fn ask(m: &DesignModel, request: &str) -> Result<Value, ErrorCode> {
    match Reply::from_json(&handle_json(Some(m), request)).unwrap() {
        Reply::Ok(v) => Ok(v),
        Reply::Err(e) => Err(e.code),
    }
}

fn value<'a>(get: &'a Value, name: &str) -> &'a Value {
    get["values"].as_array().unwrap().iter().find(|v| v["name"] == name).unwrap_or_else(|| panic!("no {name} in {get}"))
}

#[test]
fn list_gives_every_component_in_creation_order_with_ids() {
    let m = model();
    let r = ask(&m, r#"{"op":"list"}"#).unwrap();
    let ids: Vec<&str> = r["objects"].as_array().unwrap().iter().map(|o| o["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["Form1", "Panel1", "Save@main.rr", "Memo1", "DB", "Settings", "Save@settings.inc", "Twin@settings.inc#1", "Twin@settings.inc#2"]);
    assert_eq!(r["total"], 9);
    assert!(r.get("next").is_none());
    let save = &r["objects"][2];
    assert_eq!(save["name"], "Save");
    assert_eq!(save["type"], "RButton");
    assert!(save.get("typeWritten").is_none(), "written as RapidR's name");
    assert_eq!((save["parent"].clone(), save["form"].clone()), (json!("Panel1"), json!("Form1")));
    assert_eq!((save["file"].clone(), save["line"].clone()), (json!("main.rr"), json!(13)));
    assert_eq!((save["visual"].clone(), save["container"].clone(), save["children"].clone()), (json!(true), json!(false), json!(0)));
    let qsave = &r["objects"][6];
    assert_eq!((qsave["type"].clone(), qsave["typeWritten"].clone()), (json!("RButton"), json!("QBUTTON")));
    let db = &r["objects"][4];
    assert_eq!((db["visual"].clone(), db["form"].clone()), (json!(false), json!("DB")));
    assert_eq!(r["objects"][0]["container"], true);
    assert_eq!(r["objects"][1]["children"], 1);
}

#[test]
fn list_filters_and_pages() {
    let m = model();
    // by type, any of its names
    let r = ask(&m, r#"{"op":"list","type":"QButton"}"#).unwrap();
    assert_eq!(r["total"], 2);
    // inside a component (not itself)
    let r = ask(&m, r#"{"op":"list","scope":"Form1"}"#).unwrap();
    let names: Vec<&str> = r["objects"].as_array().unwrap().iter().map(|o| o["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Panel1", "Save", "Memo1"]);
    // pages: a cursor carries the revision
    let p1 = ask(&m, r#"{"op":"list","limit":4}"#).unwrap();
    assert_eq!((p1["objects"].as_array().unwrap().len(), p1["total"].clone()), (4, json!(9)));
    let next = p1["next"].as_str().unwrap();
    let p2 = ask(&m, &format!(r#"{{"op":"list","limit":4,"cursor":"{next}"}}"#)).unwrap();
    assert_eq!(p2["objects"][0]["id"], "DB");
    let p3 = ask(&m, &format!(r#"{{"op":"list","limit":4,"cursor":"{}"}}"#, p2["next"].as_str().unwrap())).unwrap();
    assert_eq!((p3["objects"].as_array().unwrap().len(), p3.get("next")), (1, None));
    // a cursor from another revision is stale
    let other = DesignModel::in_memory(&[("main.rr", MAIN)]);
    assert_eq!(ask(&other, &format!(r#"{{"op":"list","cursor":"{next}"}}"#)), Err(ErrorCode::Stale));
    assert_eq!(ask(&m, r#"{"op":"list","limit":0}"#), Err(ErrorCode::BadRequest));
    assert_eq!(ask(&m, r#"{"op":"list","type":"QNOTHING"}"#), Err(ErrorCode::UnknownType));
    assert_eq!(ask(&m, r#"{"op":"list","scope":"Nope"}"#), Err(ErrorCode::NotFound));
}

#[test]
fn get_types_values_with_their_source_and_line() {
    let m = model();
    let g = ask(&m, r#"{"op":"get","id":"save@MAIN.RR"}"#);
    assert_eq!(g, Err(ErrorCode::NotFound), "file names are exact");
    let g = ask(&m, r#"{"op":"get","id":"Save@main.rr"}"#).unwrap();
    assert_eq!((g["id"].clone(), g["type"].clone()), (json!("Save@main.rr"), json!("RButton")));
    let caption = value(&g, "Caption");
    assert_eq!(caption["value"], "Save");
    assert_eq!((caption["written"].clone(), caption["source"].clone(), caption["line"].clone()), (json!("\"Save\""), json!("set"), json!(14)));
    assert_eq!(value(&g, "Left")["value"], 10);
    assert_eq!(value(&g, "Anchors")["value"], json!({"set": ["akLeft", "akTop", "akRight"], "value": 7}));
    assert_eq!(value(&g, "Visible")["value"], true);
    // not written: the registry's default
    let align = value(&g, "Align");
    assert_eq!((align["value"].clone(), align["source"].clone(), align.get("line")), (json!({"const": "alNone", "value": 0}), json!("default"), None));
    assert_eq!(value(&g, "Width")["source"], "unset");
    // Constraints.MinWidth is MinWidth, as written
    let mw = value(&g, "MinWidth");
    assert_eq!((mw["value"].clone(), mw["written"].clone()), (json!(60), json!("60")));
    // a font's part, an event bound
    assert_eq!(value(&g, "Font.Name")["value"], "Arial");
    assert_eq!(g["events"], json!([{"name": "OnClick", "handler": "SaveClick", "line": 20}]));
    assert_eq!(Source::Set, serde_json::from_value(caption["source"].clone()).unwrap());
}

#[test]
fn values_the_designer_cant_evaluate_and_constants() {
    let m = model();
    let memo = ask(&m, r#"{"op":"get","id":"Memo1","props":["Width","Color","align"]}"#).unwrap();
    assert_eq!(value(&memo, "Width")["value"], json!({"code": "Screen.Width / 2"}));
    assert_eq!(value(&memo, "Color")["value"], json!({"color": "#0000FF", "bgr": 0xFF0000}));
    // (RMemo's registry entry has no Align: written, it comes back as
    // written and marked unknown)
    let align = value(&memo, "Align");
    assert_eq!((align["value"].clone(), align["unknown"].clone()), (json!(5), json!(true)));
    let panel = ask(&m, r#"{"op":"get","id":"Panel1","props":["align"]}"#).unwrap();
    assert_eq!(value(&panel, "Align")["value"], json!({"const": "alTop", "value": 1}), "the registry's spelling");
    let form = ask(&m, r#"{"op":"get","id":"Form1","props":["Color"]}"#).unwrap();
    assert_eq!(value(&form, "Color")["value"], json!({"const": "clBtnFace", "bgr": -2147483633i64, "system": true}));
    // a default typed as its property is (RForm's Color: a system colour)
    let twin = ask(&m, r#"{"op":"get","id":"Twin@settings.inc#1","props":["Color"]}"#).unwrap();
    assert_eq!((value(&twin, "Color")["value"]["system"].clone(), value(&twin, "Color")["source"].clone()), (json!(true), json!("default")));
    // no RAPIDQ.INC: clRed is an undefined name there, as in RapidQ
    let settings = ask(&m, r#"{"op":"get","id":"Settings","props":["Color"]}"#).unwrap();
    assert_eq!(value(&settings, "Color")["value"], json!({"const": "clRed"}));
}

#[test]
fn secrets_are_never_returned() {
    let m = model();
    let db = ask(&m, r#"{"op":"get","id":"DB"}"#).unwrap();
    let pw = value(&db, "Password");
    assert_eq!(pw["value"], json!({"redacted": true}));
    assert!(pw.get("written").is_none());
    assert_eq!(value(&db, "Host")["value"], "localhost");
    let text = handle_json(Some(&m), r#"{"op":"get","id":"DB","props":["Password"]}"#);
    assert!(!text.contains("hunter2"), "{text}");
    let text = handle_json(Some(&m), r#"{"op":"get","id":"DB"}"#);
    assert!(!text.contains("hunter2"), "{text}");
}

#[test]
fn ids_names_and_errors() {
    let m = model();
    // a name two components have
    assert_eq!(ask(&m, r#"{"op":"get","id":"Save"}"#), Err(ErrorCode::Ambiguous));
    let e = Reply::from_json(&handle_json(Some(&m), r#"{"op":"get","id":"save"}"#)).unwrap();
    assert!(e.error().unwrap().message.contains("Save@main.rr, Save@settings.inc"), "{e:?}");
    // any case; the CREATE's spelling comes back
    assert_eq!(ask(&m, r#"{"op":"get","id":"PANEL1","props":["Height"]}"#).unwrap()["name"], "Panel1");
    assert_eq!(ask(&m, r#"{"op":"get","id":"Twin@settings.inc#2","props":["Caption"]}"#).unwrap()["values"][0]["value"], "second");
    assert_eq!(ask(&m, r#"{"op":"get","id":"Twin@settings.inc#3"}"#), Err(ErrorCode::NotFound));
    assert_eq!(ask(&m, r#"{"op":"get","id":"Twin@settings.inc#x"}"#), Err(ErrorCode::BadRequest));
    assert_eq!(ask(&m, r#"{"op":"get","id":"Nothing"}"#), Err(ErrorCode::NotFound));
    assert_eq!(ask(&m, r#"{"op":"get","id":"Buttons(3)"}"#), Err(ErrorCode::NotFound), "arrays are run-time");
    assert_eq!(ask(&m, r#"{"op":"get","id":"Form1","props":["Frobnicate"]}"#), Err(ErrorCode::NotFound));
    // RapidQ's members RapidR doesn't have aren't offered
    let missing = rapidr_lang::component("RFORM").unwrap().properties.iter().find(|p| p.missing).map(|p| p.name);
    if let Some(p) = missing {
        assert_eq!(ask(&m, &format!(r#"{{"op":"get","id":"Form1","props":["{p}"]}}"#)), Err(ErrorCode::NotFound));
    }
    // an unbound event asked for by name
    let g = ask(&m, r#"{"op":"get","id":"Memo1","props":["OnChange"]}"#).unwrap();
    assert_eq!(g["events"], json!([{"name": "OnChange"}]));
}

#[test]
fn the_revision_follows_the_source() {
    let a = model();
    let b = model();
    assert_eq!(a.rev(), b.rev(), "the same source, the same rev");
    let changed = DesignModel::in_memory(&[("main.rr", &MAIN.replace("\"Notes\"", "\"Notes 2\"")), ("settings.inc", SETTINGS)]);
    assert_ne!(a.rev(), changed.rev());
    assert_eq!(a.list(&ListQuery::default()).unwrap().rev, a.rev());
    assert_eq!(a.rev().len(), 16);
}

#[test]
fn describe_is_answered_beside_a_model() {
    let m = model();
    let d = ask(&m, r#"{"op":"describe","type":"RMySQL"}"#).unwrap();
    let pw = d["properties"].as_array().unwrap().iter().find(|p| p["name"] == "Password").unwrap();
    assert_eq!(pw["secret"], true);
}

/// docs/ai-architecture.md §3.8's budgets, measured on a form of 1,000
/// buttons: `cargo test -p rapidr-introspect --features design --release
/// -- --ignored budgets --nocapture` (checked in release builds only).
#[test]
#[ignore]
fn budgets() {
    use std::time::Instant;
    let mut src = String::from("CREATE Form1 AS RForm\n");
    for i in 1..=1000 {
        src.push_str(&format!("    CREATE B{i} AS RButton\n        Caption = \"B{i}\"\n        Left = {}: Top = {}\n    END CREATE\n", i % 40 * 20, i / 40 * 20));
    }
    src.push_str("END CREATE\n");
    let t = Instant::now();
    let m = DesignModel::in_memory(&[("many.rr", &src)]);
    let open = t.elapsed();
    let t = Instant::now();
    let all = m.list(&ListQuery { limit: Some(1000), ..ListQuery::default() }).unwrap();
    let list = t.elapsed();
    assert_eq!(all.total, 1001);
    let t = Instant::now();
    let g = m.get("B500", None).unwrap();
    let get = t.elapsed();
    assert!(g.values.iter().any(|v| v.name == "Caption" && v.value == "B500"));
    let t = Instant::now();
    let d = handle_json(None, r#"{"op":"describe","type":"RStringGrid"}"#);
    let describe = t.elapsed();
    assert!(d.starts_with(r#"{"ok":true"#));
    println!("open {open:?} (the designer's parse), list 1,000 {list:?}, get {get:?}, describe {describe:?}");
    if !cfg!(debug_assertions) {
        assert!(list.as_millis() < 5, "list {list:?}");
        assert!(get.as_millis() < 1, "get {get:?}");
        assert!(describe.as_millis() < 1, "describe {describe:?}");
    }
}

#[test]
fn files_on_disk_stay_inside_their_root() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("introspect-{}", std::process::id()));
    let inside = dir.join("project");
    std::fs::create_dir_all(&inside).unwrap();
    std::fs::write(inside.join("main.rr"), "CREATE Form1 AS RForm\n    Caption = \"Hi\"\nEND CREATE\n").unwrap();
    std::fs::write(dir.join("secret.rr"), "CREATE Spy AS RForm\nEND CREATE\n").unwrap();
    let m = DesignModel::open(&inside, &["main.rr".into()]).unwrap();
    assert_eq!(m.paths(), ["main.rr"]);
    assert_eq!(ask(&m, r#"{"op":"get","id":"Form1","props":["Caption"]}"#).unwrap()["values"][0]["value"], "Hi");
    assert!(DesignModel::open(&inside, &["../secret.rr".into()]).is_err());
    assert!(DesignModel::open(&inside, &[dir.join("secret.rr")]).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
