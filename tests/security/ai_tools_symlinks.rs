//! Security regression SEC-23 (docs/security/audit-preview4.md): RAI's
//! project tools (read_file, list, propose_edit, write_file) never follow a
//! link inside the project to a place outside it. Compiled into
//! rapidr-studio's tests (`cargo test -p rapidr-studio security_`).

use crate::ai_tools::project_path;

#[cfg(unix)]
#[test]
fn sec23_links_out_of_the_project_are_refused() {
    use std::os::unix::fs::symlink;
    let base = std::env::temp_dir().join(format!("rapidr-sec23-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (project, outside) = (base.join("project"), base.join("outside"));
    std::fs::create_dir_all(project.join("forms")).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(project.join("main.rr"), "PRINT 1\n").unwrap();
    std::fs::write(outside.join("id_secret"), "not for the model\n").unwrap();
    // (a folder link and a file link out of the project, a link to nowhere,
    // and a link that stays inside)
    symlink(&outside, project.join("notes")).unwrap();
    symlink(outside.join("id_secret"), project.join("readme.txt")).unwrap();
    symlink(base.join("missing"), project.join("gone")).unwrap();
    symlink(project.join("forms"), project.join("alias")).unwrap();
    let folder = project.to_string_lossy().into_owned();

    for bad in ["notes/id_secret", "readme.txt", "notes/new.rr", "gone/new.rr", "gone", "./notes/../notes/id_secret"] {
        let e = project_path(&folder, bad).expect_err(bad);
        assert!(e.contains("outside the project"), "{bad}: {e}");
    }
    // (the project's own files, through a link that stays inside too)
    assert_eq!(project_path(&folder, "main.rr").unwrap(), "main.rr");
    assert_eq!(project_path(&folder, "forms/new.inc").unwrap(), "forms/new.inc");
    assert_eq!(project_path(&folder, "alias/x.inc").unwrap(), "alias/x.inc");
    assert_eq!(project_path(&folder, "new/deeper/file.rr").unwrap(), "new/deeper/file.rr");
    // (the textual checks still hold)
    assert!(project_path(&folder, "../outside/id_secret").is_err());
    let _ = std::fs::remove_dir_all(&base);
}
