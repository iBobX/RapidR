//! Security regressions (docs/security/audit-preview4.md): whose keys and
//! secrets a program reaches. Compiled into rapidr-ai's tests
//! (`cargo test -p rapidr-ai --features component security_`), always with
//! the memory keystore: the user's keychain is never touched.
//!
//! - SEC-21: a program never reads RapidR Studio's AI keys, nor another
//!   program's; Studio still reads its own; a program naming Studio's key is
//!   told, in plain words, to give the program its own.
//! - SEC-22: RSecrets are kept by the program's identity (its path; a built
//!   macOS app's bundle identifier), so two programs of the same file name
//!   don't share them, and items kept by name alone are never read.

use crate::keys::{self, Scope};
use crate::secrets;

fn memory() {
    std::env::set_var("RAPIDR_AI_KEYSTORE", "memory");
}

/// (made-up keys, never a real one's shape in a file)
fn key(tail: &str) -> String {
    format!("{}{}-{tail}", "sk", "-test-not-a-key")
}

fn folder(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("rapidr-sec-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn sec21_a_program_never_reads_studios_key() {
    memory();
    let studio_key = key("studio-0001");
    keys::set_named_in(&Scope::Studio, "RapidR Studio/anthropic", &studio_key, false).unwrap();
    // (Studio reads its own, as before)
    assert_eq!(keys::resolve_in(&Scope::Studio, "keychain:RapidR Studio/anthropic").0.as_deref(), Some(studio_key.as_str()));
    // (a program, by its path, finds nothing under Studio's name …)
    let evil = Scope::of_program("/Users/someone/Downloads/evil.rr");
    assert_eq!(keys::resolve_in(&evil, "keychain:RapidR Studio/anthropic"), (None, keys::Source::None));
    // (… and this process — a program, not Studio — neither, by the plain API
    // a program's RAI uses)
    assert!(!keys::is_studio(), "the test process isn't Studio");
    assert_ne!(keys::own_scope(), Scope::Studio);
    assert_eq!(keys::resolve("keychain:RapidR Studio/anthropic|env:RAPIDR_TEST_UNSET_VAR").0, None);
    // (told why, in plain words, never with the key)
    let note = keys::studio_key_note(&evil, "keychain:RapidR Studio/anthropic|ask").expect("a note");
    assert!(note.contains("RapidR Studio's own key") && note.contains("its own key"), "{note}");
    assert!(!note.contains(&studio_key));
    assert!(keys::studio_key_note(&evil, "keychain:MyApp/OpenAI|ask").is_none());
    assert!(keys::studio_key_note(&Scope::Studio, "keychain:RapidR Studio/anthropic").is_none());
    // (Studio's service is unchanged: its entries stay where they are)
    assert_eq!(Scope::Studio.service(), keys::SERVICE);
    assert!(evil.service().starts_with("rapidr-app:") && evil.service().ends_with(":RAI"), "{}", evil.service());
    keys::delete_named_in(&Scope::Studio, "RapidR Studio/anthropic");
}

#[test]
fn sec21_programs_keys_are_their_own() {
    memory();
    let (a, b) = (Scope::of_program("/work/a/app.rr"), Scope::of_program("/work/b/other.rr"));
    let ka = key("program-a-0002");
    keys::set_named_in(&a, "MyApp/OpenAI", &ka, false).unwrap();
    assert_eq!(keys::get_named_in(&a, "MyApp/OpenAI").0.as_deref(), Some(ka.as_str()));
    assert_eq!(keys::get_named_in(&b, "MyApp/OpenAI").0, None, "another program's key");
    assert_eq!(keys::get_named_in(&Scope::Studio, "MyApp/OpenAI").0, None, "Studio doesn't see it as its own either");
    // (Studio's designer keeps a key for the program it designs: the
    // program, at run time, finds it — its path the same file)
    let dir = folder("designer");
    let main = dir.join("main.rr");
    std::fs::write(&main, "PRINT 1\n").unwrap();
    let designed = Scope::of_program(&main.to_string_lossy());
    let kd = key("designed-0003");
    keys::set_named_in(&designed, "RapidR/Main/AI1", &kd, false).unwrap();
    let at_run = Scope::of_program(&std::fs::canonicalize(&main).unwrap().to_string_lossy());
    assert_eq!(keys::resolve_in(&at_run, "keychain:RapidR/Main/AI1|ask").0.as_deref(), Some(kd.as_str()));
    keys::delete_named_in(&a, "MyApp/OpenAI");
    keys::delete_named_in(&designed, "RapidR/Main/AI1");
    let _ = std::fs::remove_dir_all(dir);
}

/// RAI in a program: KeyOwner is refused, and Studio's key isn't found —
/// Error says what to do.
#[cfg(feature = "component")]
#[test]
fn sec21_rai_in_a_program() {
    use rapidr_value::Value;
    memory();
    keys::set_named_in(&Scope::Studio, "RapidR Studio/openai", &key("studio-0004"), false).unwrap();
    let name = "sec21ai";
    let s = |v: &str| Value::String(v.to_string());
    crate::rai::set(name, "provider", &s("openai"));
    crate::rai::set(name, "keyowner", &s("/Users/someone/ide/studio.rr"));
    assert!(crate::rai::get(name, "error").unwrap().to_string_val().contains("KeyOwner is RapidR Studio's own"));
    assert_eq!(crate::rai::get(name, "keyowner").unwrap().to_string_val(), "", "not taken");
    crate::rai::set(name, "apikeysource", &s("keychain:RapidR Studio/openai"));
    assert_eq!(crate::rai::get(name, "haskey").unwrap().to_i64(), 0, "Studio's key isn't the program's");
    let asked = crate::rai::call(name, "ask", &[s("hello")], |v| v.to_string_val()).unwrap();
    assert_eq!(asked.to_i64(), 0);
    let e = crate::rai::get(name, "error").unwrap().to_string_val();
    assert!(e.contains("RapidR Studio's own key") && e.contains("APIKey row"), "{e}");
    keys::delete_named_in(&Scope::Studio, "RapidR Studio/openai");
}

#[test]
fn sec22_two_programs_of_one_name_are_two_programs() {
    memory();
    let (one, two) = (folder("one"), folder("two"));
    let (p1, p2) = (one.join("main.rr"), two.join("main.rr"));
    std::fs::write(&p1, "PRINT 1\n").unwrap();
    std::fs::write(&p2, "PRINT 2\n").unwrap();
    let (id1, id2) = (secrets::program_id(&p1.to_string_lossy()), secrets::program_id(&p2.to_string_lossy()));
    assert_ne!(id1, id2, "same name, two folders");
    assert!(id1.starts_with("main~") && id1.len() == "main~".len() + 16, "{id1}");
    // (one file, however the path is written)
    let relative = std::env::current_dir().ok().and_then(|cwd| pathdiff(&p1, &cwd));
    if let Some(r) = relative {
        assert_eq!(secrets::program_id(&r), id1);
    }
    let dotted = one.join(".").join("main.rr");
    assert_eq!(secrets::program_id(&dotted.to_string_lossy()), id1);
    secrets::save(&id1, "MySQL", "office", "pw-of-one").unwrap();
    assert_eq!(secrets::read(&id1, "MySQL", "office").unwrap().as_deref(), Some("pw-of-one"));
    assert_eq!(secrets::read(&id2, "MySQL", "office").unwrap(), None, "the other main.rr");
    // (kept by name alone before Preview 4: never read, by either)
    secrets::save("main", "MySQL", "office", "legacy-pw").unwrap();
    assert_eq!(secrets::read(&id2, "MySQL", "office").unwrap(), None);
    assert_ne!(secrets::keychain_service(&id1, "MySQL"), secrets::keychain_service("main", "MySQL"));
    secrets::delete(&id1, "MySQL", "office").unwrap();
    secrets::delete("main", "MySQL", "office").unwrap();
    let _ = std::fs::remove_dir_all(one);
    let _ = std::fs::remove_dir_all(two);
}

#[test]
fn sec22_a_built_mac_app_is_its_bundle_id() {
    let dir = folder("bundle");
    let macos = dir.join("Note Pad.app").join("Contents").join("MacOS");
    std::fs::create_dir_all(&macos).unwrap();
    std::fs::write(macos.join("Note Pad"), b"").unwrap();
    std::fs::write(
        dir.join("Note Pad.app/Contents/Info.plist"),
        "<?xml version=\"1.0\"?>\n<plist version=\"1.0\"><dict>\n\t<key>CFBundleIdentifier</key>\n\t<string>com.Example.NotePad</string>\n</dict></plist>\n",
    )
    .unwrap();
    let exe = macos.join("Note Pad");
    assert_eq!(secrets::program_id(&exe.to_string_lossy()), "com.example.notepad");
    // (no bundle: the path's identity)
    std::fs::remove_file(dir.join("Note Pad.app/Contents/Info.plist")).unwrap();
    assert!(secrets::program_id(&exe.to_string_lossy()).starts_with("note_pad~"));
    let _ = std::fs::remove_dir_all(dir);
}

/// `path` relative to `base`, when `path` is under it.
fn pathdiff(path: &std::path::Path, base: &std::path::Path) -> Option<String> {
    let path = std::fs::canonicalize(path).ok()?;
    let base = std::fs::canonicalize(base).ok()?;
    path.strip_prefix(&base).ok().map(|p| p.to_string_lossy().into_owned())
}

/// SEC-26: a model's answer can't carry Studio's own action links (`ai:`,
/// `rai:`): however it writes one — a plain link, an autolink, a reference,
/// an entity, a code fence opened in an earlier answer —, it shows as
/// words. Studio's own links (a card's Review) and a web link stay links.
#[cfg(feature = "component")]
#[test]
fn sec26_model_text_carries_no_studio_links() {
    use rapidr_value::panels::markdown::Doc;
    let card = format!("##### ● Proposed a change to main.rr · +1 −1 · [Review]({})", crate::rai::own_link("ai:review"));
    let answer = format!(
        "{card}\n\nDone. [See the docs](https://example.com). [Open your program](ai:undo/MyApp) <rai:retry> [ok][r] [x](&#97;i:open/X) [y](ai&#58;undo/Y) [z](<AI:undo/Z>)\n\n[r]: Rai:edit/1\n\n```"
    );
    let t = crate::rai::transcript_of(&[("user", "make it [blue](ai:open/Q)"), ("assistant", &answer), ("assistant", "```\n[later](ai:undo/Late)")]);
    let links = Doc::parse(&t).links;
    assert_eq!(links, vec!["ai:review".to_string(), "https://example.com".to_string()], "{t}");
    // (the words stay, plain; Studio's mark is never shown)
    assert!(t.contains("Open your program") && t.contains("[Review](ai:review)"), "{t}");
    assert!(!t.contains("rapidr-own-"), "{t}");
}
