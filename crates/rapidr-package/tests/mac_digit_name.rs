//! An app whose name starts with a digit, written to the current folder, signs
//! and verifies: `rapidr build 3dcube.bas` makes `3dcube.app`, a relative path
//! `codesign --verify` took for a process id ("3dcube.app: No such process").
//!
//! Its own test binary (one test): it changes the process' current folder,
//! which would disturb tests that share the process.

#![cfg(target_os = "macos")]

use std::path::Path;
use std::process::Command;

use rapidr_package::macos::{sign, write, MacApp};
use rapidr_package::{AppInfo, Icon};

#[test]
fn a_digit_named_app_in_the_current_folder_signs_and_verifies() {
    let dir = std::env::temp_dir().join(format!("rapidr-package-digit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_current_dir(&dir).unwrap();

    let info = AppInfo::new("3dcube", "3dcube");
    let icon = Icon::rapidr_default();
    let app = MacApp { info: &info, icon: &icon, executable: b"#!/bin/sh\necho hi\n", resources: Vec::new(), minimum_system: "10.13" };
    // (`rapidr build 3dcube.bas`: the program's folder is "" — the app's path is `3dcube.app`, relative)
    let path = write(Path::new(""), &app).unwrap();
    assert_eq!(path, Path::new("3dcube.app"));
    assert!(sign(&path).unwrap(), "signed");

    // the signature holds, asked for the way the bug asked: the bare relative name
    // (and by the absolute path: the same app)
    assert!(Command::new("codesign").args(["--verify", "--strict"]).arg("./3dcube.app").status().unwrap().success());
    assert!(Command::new("codesign").args(["--verify", "--strict"]).arg(dir.join("3dcube.app")).status().unwrap().success());

    std::env::set_current_dir(std::env::temp_dir()).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}
