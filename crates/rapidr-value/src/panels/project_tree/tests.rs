//! RPROJECTTREE's model without a runtime: loading, the rows, renaming,
//! moving, new names, Save's text.

use std::collections::HashMap;

use rapidr_project::{FileKind, Project};

use super::model::*;

const PROJ: &str = r#"format = 2
name = "Demo"
main = "Main.rr"

[[files]]
path = "Main.rr"
kind = "module"

[[files]]
path = "forms/Form1.rr"
kind = "form"

[[files]]
path = "Util.inc"

[[files]]
path = "forms/About.rr"
kind = "form"

[[files]]
path = "Helpers.rr"
kind = "module"

[[files]]
path = "data/prices.csv"
"#;

const FORM1: &str = "CREATE Form1 AS QFORM\n  CREATE Panel1 AS QPANEL\n    CREATE Button1 AS QBUTTON\n    END CREATE\n  END CREATE\n  CREATE Edit1 AS QEDIT\n  END CREATE\nEND CREATE\n";

fn files(map: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let m: HashMap<String, String> = map.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |p: &str| m.get(p).cloned()
}

fn demo() -> ProjectTree {
    let mut t = ProjectTree::default();
    assert!(t.load("proj/Demo.rrproj", None, &files(&[("proj/Demo.rrproj", PROJ), ("proj/forms/Form1.rr", FORM1), ("proj/forms/About.rr", "CREATE About AS QFORM\nEND CREATE\n")])));
    t
}

fn shown(t: &ProjectTree) -> Vec<String> {
    t.rows().iter().map(|r| format!("{}{}{}", "  ".repeat(r.level), r.label, if r.expandable { if r.expanded { " -" } else { " +" } } else { "" })).collect()
}

#[test]
fn a_project_file_grouped_by_kind() {
    let t = demo();
    assert_eq!(t.project.name, "Demo");
    assert_eq!(
        shown(&t),
        ["Demo -", "  Forms -", "    forms -", "      Form1.rr +", "      About.rr +", "  Modules -", "    Main.rr", "    Helpers.rr", "  Includes -", "    Util.inc", "  Data -", "    data -", "      prices.csv"]
    );
    let rows = t.rows();
    assert_eq!(rows[1].count, Some(2));
    assert!(rows.iter().find(|r| r.label == "Main.rr").unwrap().main);
    assert_eq!(rows[3].parent, Some(2));
}

#[test]
fn a_form_opens_into_its_components() {
    let mut t = demo();
    t.set_open("forms/Form1.rr", true);
    let s = shown(&t);
    assert_eq!(&s[3..8], ["      Form1.rr -", "        Form1 -", "          Panel1 -", "            Button1", "          Edit1"]);
    let b = t.rows().into_iter().find(|r| r.label == "Button1").unwrap();
    assert_eq!(b.key, "forms/Form1.rr#Button1");
    assert_eq!(b.kind, NodeKind::Component { file: "forms/Form1.rr".into(), name: "Button1".into(), ty: "QBUTTON".into() });
    // (ShowComponents off: forms don't open)
    t.show_components = false;
    assert!(!t.rows().iter().any(|r| r.label == "Button1"));
    // (ShowForms off: no Forms group; ShowFiles off: only it)
    t.show_components = true;
    t.show_forms = false;
    assert!(!t.rows().iter().any(|r| r.label == "Forms"));
    t.show_forms = true;
    t.show_files = false;
    assert_eq!(t.rows().iter().filter(|r| r.level == 1).map(|r| r.label.as_str()).collect::<Vec<_>>(), ["Forms"]);
}

#[test]
fn reveal_opens_what_is_above() {
    let mut t = demo();
    t.open_all(false);
    assert_eq!(shown(&t), ["Demo -", "  Forms +", "  Modules +", "  Includes +", "  Data +"]);
    let key = t.key_of("FORMS/form1.rr#button1").unwrap();
    assert_eq!(key, "forms/Form1.rr#Button1");
    t.open_above(&key);
    assert!(t.rows().iter().any(|r| r.key == key));
    assert_eq!(ProjectTree::public(&key), key);
    let folder = t.key_of("data/").unwrap();
    assert_eq!(ProjectTree::public(&folder), "data/");
    assert_eq!(ProjectTree::public(KEY_PROJECT), "");
    t.open_all(true);
    assert!(shown(&t).iter().all(|r| !r.ends_with('+')));
}

#[test]
fn an_implicit_project_follows_its_includes() {
    let mut t = ProjectTree::default();
    let main = "$INCLUDE \"RAPIDQ.INC\"\n$INCLUDE \"lib/util.inc\"\n$INCLUDE \"Win.rr\"\nPRINT 1\n";
    let read = files(&[("src/lib/util.inc", "' tools\n"), ("src/Win.rr", "CREATE Win AS QFORM\n CREATE B AS QBUTTON\n END CREATE\nEND CREATE\n")]);
    assert!(t.load("src/App.bas", Some(main.into()), &read));
    assert_eq!(t.project.name, "App");
    let got: Vec<(&str, FileKind)> = t.project.files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
    assert_eq!(got, [("App.bas", FileKind::Module), ("lib/util.inc", FileKind::Include), ("Win.rr", FileKind::Form)]);
    assert_eq!(t.comps["win.rr"][0].children[0].name, "B");
    // (read from the files: the main file through `read` too)
    let mut u = ProjectTree::default();
    assert!(u.load("src/Win.rr", None, &read));
    assert_eq!(u.project.files[0].kind, FileKind::Form);
    // (no such file, not a project: nothing changes)
    assert!(!u.load("src/Gone.rr", None, &read));
    assert!(!u.load("x.rrproj", Some("format = 9".into()), &read));
    assert_eq!(u.project.name, "Win");
}

#[test]
fn texts_given_later_list_components() {
    let mut t = ProjectTree::default();
    assert!(t.load("Demo.rrproj", Some(PROJ.into()), &|_| None));
    assert!(!t.rows().iter().any(|r| r.label == "Form1.rr" && r.expandable));
    t.set_text("forms/Form1.rr", FORM1);
    assert!(t.rows().iter().any(|r| r.label == "Form1.rr" && r.expandable));
    // (given texts survive loading the same project again)
    assert!(t.load("Demo.rrproj", Some(PROJ.into()), &|_| None));
    assert!(t.comps.contains_key("forms/form1.rr"));
}

#[test]
fn rename_is_validated() {
    let t = demo();
    assert_eq!(t.validate("Main.rr", "App").unwrap(), "App.rr");
    assert_eq!(t.validate("Main.rr", " App.bas ").unwrap(), "App.bas");
    assert_eq!(t.validate("forms/About.rr", "Info").unwrap(), "forms/Info.rr");
    assert_eq!(t.validate("Main.rr", "main").unwrap(), "main.rr", "a change of case");
    assert_eq!(t.validate("Main.rr", "Main").unwrap(), "Main.rr");
    assert!(t.validate("Main.rr", "  ").is_err());
    assert!(t.validate("Main.rr", "a/b").is_err());
    assert!(t.validate("Main.rr", "a\\b").is_err());
    assert!(t.validate("Main.rr", "what?").is_err());
    assert!(t.validate("Main.rr", "..").is_err());
    assert_eq!(t.validate("Main.rr", "helpers").unwrap_err(), "helpers.rr is already in the project.");
}

#[test]
fn rename_carries_the_trees_state() {
    let mut t = demo();
    t.set_open("forms/Form1.rr", true);
    t.selected = "forms/Form1.rr#Button1".into();
    t.apply_rename("forms/Form1.rr", "forms/Main1.rr");
    assert_eq!(t.selected, "forms/Main1.rr#Button1");
    assert!(t.expanded("forms/Main1.rr"));
    assert!(t.comps.contains_key("forms/main1.rr"));
    assert!(t.modified);
    t.apply_rename("Main.rr", "App.rr");
    assert_eq!(t.project.main, "App.rr");
}

#[test]
fn files_are_dragged_into_place() {
    let mut t = demo();
    // (Helpers.rr above Main.rr: index 0)
    assert_eq!(t.drop_result("Helpers.rr", &DropAt::Before("Main.rr".into())), Some(("Helpers.rr".into(), 0)));
    // (where it is: nothing)
    assert_eq!(t.drop_result("Helpers.rr", &DropAt::After("Main.rr".into())), None);
    assert_eq!(t.drop_result("Main.rr", &DropAt::Before("Helpers.rr".into())), None);
    // (another kind: no)
    assert_eq!(t.drop_result("Main.rr", &DropAt::Before("Util.inc".into())), None);
    // (into a folder, beside the files there)
    assert_eq!(t.drop_result("forms/About.rr", &DropAt::Into(String::new())), Some(("About.rr".into(), 3)));
    assert_eq!(t.drop_result("Main.rr", &DropAt::Into("forms/".into())), Some(("forms/Main.rr".into(), 0)));
    let (new, i) = t.drop_result("Helpers.rr", &DropAt::Before("Main.rr".into())).unwrap();
    t.apply_move("Helpers.rr", &new, i);
    assert_eq!(t.project.files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["Helpers.rr", "Main.rr", "forms/Form1.rr", "Util.inc", "forms/About.rr", "data/prices.csv"]);
    // (the keyboard's steps)
    assert_eq!(t.step("Helpers.rr", true), Some(("Helpers.rr".into(), 1)));
    assert_eq!(t.step("Helpers.rr", false), None);
    assert_eq!(t.step("forms/About.rr", false), Some(("forms/About.rr".into(), 2)));
}

#[test]
fn new_files_get_free_names() {
    let mut t = demo();
    assert_eq!(t.free_name(FileKind::Form, None), "Form1.rr");
    assert_eq!(t.free_name(FileKind::Module, None), "Module1.rr");
    assert!(t.add_file("Form1.rr", Some(FileKind::Form), &|_| None));
    assert_eq!(t.free_name(FileKind::Form, None), "Form2.rr");
    assert_eq!(t.free_name(FileKind::Form, Some("Main")), "Main2.rr");
    assert_eq!(t.free_name(FileKind::Data, Some("table.json")), "table.json");
    assert_eq!(t.free_name(FileKind::Include, None), "Include1.inc");
    assert!(!t.add_file("form1.RR", None, &|_| None), "already there");
}

#[test]
fn removing_a_file_moves_the_selection() {
    let mut t = demo();
    t.selected = "Main.rr".into();
    assert!(t.remove_file("main.rr"));
    assert_eq!(t.selected, "Helpers.rr");
    assert_eq!(t.project.main, "");
    assert!(!t.remove_file("Main.rr"));
}

#[test]
fn save_writes_what_was_read() {
    let mut t = demo();
    t.apply_rename("Util.inc", "Tools.inc");
    let text = t.project.to_toml();
    let back = Project::from_toml(&text).unwrap();
    assert_eq!(back, t.project);
    assert!(text.starts_with(rapidr_project::HEADER));
    assert!(text.contains("Tools.inc"));
}
