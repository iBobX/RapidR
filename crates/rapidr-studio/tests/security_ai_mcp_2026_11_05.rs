//! Proof tests for docs/security/audit-2026-11-05-ai-mcp.md (lane
//! SECURITY-AUDIT-AI-MCP, 2026-11-05).
//!
//! Each test says what must hold. The ones marked `#[ignore]` fail today:
//! they are the finding's evidence, kept out of the default run so the
//! build stays green; the fix lane removes the `#[ignore]` with its fix.
//! Everything happens in temporary folders with marker words; nothing
//! touches the network, a keychain or the user's settings.
//!
//! ```text
//! cargo test -p rapidr-studio --test security_ai_mcp_2026_11_05 -- --include-ignored
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rapidr_studio::ai;
use rapidr_studio::ai_tools::{project_path, Project};
use rapidr_studio::tools_core;
use rapidr_value::Value;
use serde_json::json;

fn text(v: &str) -> Value {
    Value::String(v.to_string().into())
}

/// A fresh temporary folder for one test.
fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rapidr-audit-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn slashed(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// A first version of the files in `dir`, saved with the Versions engine.
fn first_version(dir: &Path) {
    let who = rapidr_vcs::Person { name: "Audit".into(), email: "audit@example.invalid".into() };
    let mut v = rapidr_vcs::Versions::start(dir, &who).expect("Versions start in a temporary folder");
    v.save_version_with("First", &who, &rapidr_vcs::SaveOptions { files: None, allow_secrets: true }).expect("the first version saved");
}

/// An AI app's call of a core tool, as Studio's MCP server makes it
/// (RAIProjectTools.CallTool: the same path ide/mcp.inc's McpRun takes).
fn app_call(component: &str, tool: &str, args: &str) -> String {
    ai::call(component, "calltool", &[text(tool), text(args)]).expect("calltool").to_string_val()
}

/// SEC-40 (a): `read_file` refuses a path that leaves the project through a
/// folder link (SEC-23's fix), but `compare_versions` reads every path a
/// version lists from the disk with `Project::text`, which checks nothing.
#[cfg(unix)]
#[test]
#[ignore = "SEC-40: compare_versions reads a version's paths from disk without the project's path checks"]
fn sec40_compare_versions_never_reads_through_a_link_out_of_the_project() {
    use std::os::unix::fs::symlink;
    let base = scratch("sec40-link");
    let (project, outside) = (base.join("project"), base.join("outside"));
    std::fs::create_dir_all(project.join("notes")).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(project.join("main.rr"), "PRINT 1\n").unwrap();
    std::fs::write(project.join("notes/todo.txt"), "first\n").unwrap();
    first_version(&project);
    // (the folder is a link to a place outside the project now, as a
    // project unpacked from an archive or cloned can have it)
    std::fs::remove_dir_all(project.join("notes")).unwrap();
    std::fs::write(outside.join("todo.txt"), "SEC40-OUTSIDE-MARKER\n").unwrap();
    symlink(&outside, project.join("notes")).unwrap();
    let folder = slashed(&project);
    // (the gate read_file uses refuses this very path)
    assert!(project_path(&folder, "notes/todo.txt").is_err(), "SEC-23's gate refuses the link");
    let files = vec!["main.rr".to_string()];
    let buffers = HashMap::new();
    let p = Project { folder: &folder, files: &files, main: "main.rr", compat: "", buffers: &buffers };
    let out = tools_core::compare_versions(&json!({}), &p);
    let _ = std::fs::remove_dir_all(&base);
    assert!(!out.content.contains("SEC40-OUTSIDE-MARKER"), "compare_versions read a file outside the project through a link:\n{}", out.content);
}

/// SEC-40 (b): AI apps may never read a hidden file (`outside_path_ok`, the
/// manual's "nothing ... hidden in it"); the rule is checked on the `path`
/// argument only, so the whole-project comparison shows a hidden file a
/// version holds.
#[test]
#[ignore = "SEC-40: AI apps reach hidden files through compare_versions"]
fn sec40_ai_apps_never_see_a_hidden_file_through_compare_versions() {
    let dir = scratch("sec40-hidden");
    std::fs::write(dir.join("main.rr"), "PRINT 1\n").unwrap();
    std::fs::write(dir.join(".env"), "SEC40-HIDDEN-THEN\n").unwrap();
    first_version(&dir);
    std::fs::write(dir.join(".env"), "SEC40-HIDDEN-NOW\n").unwrap();
    let t = "audit_sec40_outside_app";
    ai::set(t, "projectfolder", &text(&slashed(&dir)));
    ai::set(t, "projectfiles", &text("main.rr"));
    ai::set(t, "mainfile", &text("main.rr"));
    ai::set(t, "permission", &text("propose"));
    // (named, a hidden file is refused …)
    assert!(app_call(t, "read_file", r#"{"path": ".env"}"#).contains("hidden"));
    assert!(app_call(t, "compare_versions", r#"{"path": ".env"}"#).contains("hidden"));
    // (… and it must not come back through the comparison of the whole project)
    let all = app_call(t, "compare_versions", "{}");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!all.contains("SEC40-HIDDEN"), "an AI app read a hidden file through compare_versions:\n{all}");
}

/// SEC-40 (c): a project's own file list (its `.rrproj`, RProject's Files)
/// may name a file outside its folder; `read_file` refuses that path, but the
/// tools that read every source (`project_outline`, `check_code`, the
/// designer's) read it through `Project::text` and show what it holds.
#[test]
#[ignore = "SEC-40: the source-reading tools follow a project file list entry out of the project"]
fn sec40_a_listed_file_outside_the_project_is_never_read() {
    let base = scratch("sec40-listed");
    let (project, outside) = (base.join("project"), base.join("outside"));
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(project.join("main.rr"), "PRINT 1\n").unwrap();
    std::fs::write(outside.join("lib.rr"), "SUB Sec40ListedOutsideMarker\nEND SUB\n").unwrap();
    let t = "audit_sec40_listed_app";
    ai::set(t, "projectfolder", &text(&slashed(&project)));
    ai::set(t, "projectfiles", &text("main.rr\n../outside/lib.rr"));
    ai::set(t, "mainfile", &text("main.rr"));
    ai::set(t, "permission", &text("propose"));
    // (named, it is refused …)
    assert!(app_call(t, "read_file", r#"{"path": "../outside/lib.rr"}"#).contains("outside the project"));
    // (… and the outline must not read it either)
    let outline = app_call(t, "project_outline", "{}");
    let _ = std::fs::remove_dir_all(&base);
    assert!(!outline.contains("Sec40ListedOutsideMarker"), "project_outline read a file outside the project:\n{outline}");
}

/// SEC-42: set_property's `value` goes into the code as it is written for
/// typed properties (and for quoted ones): one value may carry more lines.
/// A property's value must stay one value.
#[test]
#[ignore = "SEC-42: set_property writes its value into the source as it is"]
fn sec42_set_property_takes_one_value_never_more_lines() {
    let dir = scratch("sec42");
    let main = "CREATE Form1 AS RForm\n  Caption = \"Hello\"\n  CREATE Button1 AS RButton\n    Caption = \"OK\"\n    Width = 80\n  END CREATE\nEND CREATE\nForm1.ShowModal\n";
    std::fs::write(dir.join("main.rr"), main).unwrap();
    let t = "audit_sec42_outside_app";
    ai::set(t, "projectfolder", &text(&slashed(&dir)));
    ai::set(t, "projectfiles", &text("main.rr"));
    ai::set(t, "mainfile", &text("main.rr"));
    ai::set(t, "permission", &text("propose"));
    let said = app_call(t, "set_property", r#"{"component": "Button1", "property": "Width", "value": "120\n    Caption = \"SEC42-MARKER\""}"#);
    let proposed = ai::call(t, "proposalnew", &[Value::Integer(0)]).expect("proposalnew").to_string_val();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!proposed.contains("SEC42-MARKER"), "set_property put more than one value into main.rr:\n{said}\n---\n{proposed}");
}
