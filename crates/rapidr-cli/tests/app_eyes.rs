//! RapidR Studio's eyes and hands on a running program, end to end on the
//! desktop: the IDE's end of the session (rapidr_session::ProgramSession)
//! sends `app` requests to `rapidr run --session` running a small
//! calculator (headless: a GUI test's host, its script far away) — the
//! layout report, a click by caption that the program hears, a picture of
//! its window, a window it hasn't got refused, and, paused in the
//! debugger, a picture still taken but a click refused with where it is.
//! Its own process: the GUI test's environment it sets is its own.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use rapidr_session::protocol::{Command, EventBody};
use rapidr_session::{ProgramSession, SessionEvent};
use serde_json::{json, Value as Json};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rapidr-app-eyes-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("RAPIDR_PRINT_TO", dir.join("prints"));
    std::env::set_var("RAPIDR_REGISTRY", dir.join("registry.reg"));
    std::env::set_var("RAPIDR_AI_KEYSTORE", "memory");
    // (headless: a GUI test's host; its script — the end capture — two
    // minutes away, so the session decides when it ends)
    std::env::set_var("RAPIDR_CAPTURE", dir.join("cap"));
    std::env::set_var("RAPIDR_CAPTURE_DELAY", "120");
    std::env::set_var("RAPIDR_SCALE", "2");
    dir
}

fn runtime() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rapidr"))
}

fn wait_for(s: &mut ProgramSession, output: &mut String, want: impl Fn(&SessionEvent) -> bool) -> SessionEvent {
    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        for e in s.poll(Some(Duration::from_millis(50))) {
            if let SessionEvent::Output { text, .. } = &e {
                output.push_str(text);
            }
            if want(&e) {
                return e;
            }
        }
    }
    panic!("timed out; output so far: {output:?}");
}

/// An `app` request and its result.
fn app(s: &mut ProgramSession, output: &mut String, request: Json) -> Json {
    let seq = s.request(Command::App { request }).unwrap();
    match wait_for(s, output, |e| matches!(e, SessionEvent::Reply { re, .. } if *re == seq)) {
        SessionEvent::Reply { body: EventBody::App { result }, .. } => result,
        other => panic!("not an app reply: {other:?}"),
    }
}

const CALC: &str = r#"$THEME rapidr
DECLARE SUB Digit(Sender AS RButton)
DECLARE SUB Clear
CREATE Form1 AS RForm
    Caption = "Calculator"
    ClientWidth = 200: ClientHeight = 120
    CREATE Display AS RLabel
        Caption = "0"
        Left = 10: Top = 10: Width = 180
    END CREATE
    CREATE B7 AS RButton
        Caption = "7"
        Left = 10: Top = 40: Width = 40: Height = 30
        OnClick = Digit
    END CREATE
    CREATE B8 AS RButton
        Caption = "8"
        Left = 60: Top = 40: Width = 40: Height = 30
        OnClick = Digit
    END CREATE
    CREATE BC AS RButton
        Caption = "C"
        Left = 110: Top = 40: Width = 40: Height = 30
        OnClick = Clear
    END CREATE
END CREATE
SUB Digit(Sender AS RButton)
    IF Display.Caption = "0" THEN Display.Caption = Sender.Caption ELSE Display.Caption = Display.Caption + Sender.Caption
    PRINT "display "; Display.Caption
END SUB
SUB Clear
    Display.Caption = "0"
END SUB
Form1.ShowModal
"#;

#[test]
fn studio_sees_and_drives_the_running_program_and_a_paused_one_only_looks() {
    let dir = scratch("calc");
    let main = dir.join("calc.rr");
    std::fs::write(&main, CALC).unwrap();
    let mut s = ProgramSession::new(main.to_string_lossy());
    s.start_process(&runtime()).unwrap();
    let mut out = String::new();
    wait_for(&mut s, &mut out, |e| matches!(e, SessionEvent::FormShown { .. }));

    // (the layout: the buttons and their captions, places in the window's inside)
    let r = app(&mut s, &mut out, json!({"do": "layout"}));
    let text = r["text"].as_str().unwrap();
    assert!(r["ok"] == json!(true) && text.contains("Window form1 \"Calculator\" (RForm), inside 200 × 120, in front"), "{r}");
    assert!(text.contains("b7 (RButton) \"7\" at 10,40 size 40×30"), "{text}");
    assert!(r["layout"]["windows"][0]["components"].as_array().unwrap().iter().any(|c| c["caption"] == "C"), "{r}");

    // (a click by caption: the program hears it)
    let r = app(&mut s, &mut out, json!({"do": "click", "caption": "7"}));
    assert_eq!(r["ok"], json!(true), "{r}");
    assert!(r["text"].as_str().unwrap().starts_with("Clicked b7 (RButton \"7\") in form1 (\"Calculator\") at 30,55."), "{r}");
    let r = app(&mut s, &mut out, json!({"do": "click", "name": "B8"}));
    assert_eq!(r["ok"], json!(true), "{r}");
    while !out.contains("display 78") {
        wait_for(&mut s, &mut out, |e| matches!(e, SessionEvent::Output { .. }));
    }
    let r = app(&mut s, &mut out, json!({"do": "layout", "window": "Calculator"}));
    assert!(r["text"].as_str().unwrap().contains("display (RLabel) \"78\""), "{r}");

    // (what isn't there: said with what is)
    let r = app(&mut s, &mut out, json!({"do": "click", "caption": "Go"}));
    assert_eq!(r["ok"], json!(false));
    assert!(r["text"].as_str().unwrap().contains("No button (or other control) shows \"Go\" in form1. Buttons now: \"7\", \"8\", \"C\""), "{r}");

    // (a picture: the window at the screen's scale (2×), framed; at 1× its inside)
    let r = app(&mut s, &mut out, json!({"do": "screenshot"}));
    assert_eq!(r["ok"], json!(true), "{r}");
    let png = rapidr_value::objects::codec::base64_decode(r["png"].as_str().unwrap()).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
    assert!(r["width"].as_u64().unwrap() > 400 && r["height"].as_u64().unwrap() > 240, "{r}");
    std::fs::write(dir.join("shot.png"), &png).unwrap();
    // (kept to look at: RAPIDR_APP_EYES_KEEP=<folder>)
    if let Some(keep) = std::env::var_os("RAPIDR_APP_EYES_KEEP") {
        let _ = std::fs::write(PathBuf::from(keep).join("app-screenshot-78.png"), &png);
    }
    let r = app(&mut s, &mut out, json!({"do": "screenshot", "window": "form1", "scale": "1x", "frame": false}));
    assert_eq!((r["width"].as_u64(), r["height"].as_u64()), (Some(200), Some(120)), "{r}");
    // (only its own windows: anything else is refused, its windows listed)
    let r = app(&mut s, &mut out, json!({"do": "screenshot", "window": "Finder"}));
    assert_eq!(r["ok"], json!(false));
    assert!(r["text"].as_str().unwrap().contains("No window named \"Finder\" in this program. Its windows now: form1 (\"Calculator\")."), "{r}");

    // (paused in the debugger: a picture still, input refused with where)
    s.pause().unwrap();
    wait_for(&mut s, &mut out, |e| matches!(e, SessionEvent::Stopped { .. }));
    let r = app(&mut s, &mut out, json!({"do": "click", "caption": "C"}));
    assert_eq!(r["ok"], json!(false), "{r}");
    let said = r["text"].as_str().unwrap();
    assert!(said.starts_with("The program is paused in the debugger at line ") && said.contains("continue it first"), "{said}");
    let r = app(&mut s, &mut out, json!({"do": "screenshot", "scale": "1x"}));
    assert_eq!(r["ok"], json!(true), "{r}");
    let r = app(&mut s, &mut out, json!({"do": "layout"}));
    assert!(r["text"].as_str().unwrap().contains("\"78\""), "{r}");

    // (going on: input again)
    s.continue_().unwrap();
    wait_for(&mut s, &mut out, |e| matches!(e, SessionEvent::Continued));
    let r = app(&mut s, &mut out, json!({"do": "click", "caption": "C"}));
    assert_eq!(r["ok"], json!(true), "{r}");
    let r = app(&mut s, &mut out, json!({"do": "layout"}));
    assert!(r["text"].as_str().unwrap().contains("display (RLabel) \"0\""), "{r}");
    s.stop();
    let _ = std::fs::remove_dir_all(&dir);
}
