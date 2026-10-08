//! Scenes built through the QD3D* objects' API, as a program builds them,
//! and drawn by `Render` on the GPU: what the back buffer then holds.

use rapidr_value::objects::d3d::{call, create, render, screen_call};
use rapidr_value::objects::directx::{self, DxScreen};
use rapidr_value::{v_int, v_str, Value};

// D3DRMRENDERQUALITY's parts (RapidQ_D3D.inc): lit, solid.
const LIGHT_ON: i64 = 8;
const FILL_SOLID: i64 = 128;

fn call_ok(id: &str, method: &str, args: &[Value]) {
    let r = call(id, method, args).unwrap_or_else(|| panic!("{id}.{method}: not handled"));
    r.unwrap_or_else(|e| panic!("{id}.{method}: {e}"));
}

fn d(v: f64) -> Value {
    Value::Double(v)
}

fn screen(w: i64, h: i64) -> DxScreen {
    rapidr_d3d_gpu::install();
    let mut screen = DxScreen::default();
    screen.call("init", &[v_int(w), v_int(h)]);
    directx::initialize(&mut screen, w, h, true);
    screen
}

/// A lit square facing the camera fills the middle of the view in its
/// colour times the light's; its back isn't drawn; with no light it's
/// black; Move turns its frame.
#[test]
fn a_lit_face_in_the_middle() {
    let mut screen = screen(40, 30);
    for (id, t) in [("t_mb", "RD3DMESHBUILDER"), ("t_f", "RD3DFRAME"), ("t_face", "RD3DFACE"), ("t_light", "RD3DLIGHT"), ("t_lf", "RD3DFRAME")] {
        create(id, t);
    }
    let sc = |m: &str, a: &[Value]| screen_call("t_dx", &mut DxScreen::default(), m, a).unwrap().unwrap();
    sc("createframe", &[v_str("t_f")]);
    sc("createframe", &[v_str("t_lf")]);
    sc("createmeshbuilder", &[v_str("t_mb")]);
    sc("createface", &[v_str("t_face")]);
    // A square at z = 5 (the camera at the origin looks along +z),
    // clockwise from the camera: its front.
    for (x, y) in [(-1.0, 1.0), (1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        call_ok("t_face", "addvertex", &[d(x), d(y), d(0.0)]);
    }
    call_ok("t_face", "setcolorrgb", &[d(1.0), d(0.5), d(0.0)]);
    call_ok("t_mb", "addface", &[v_str("t_face")]);
    call_ok("t_mb", "setquality", &[v_int(LIGHT_ON + FILL_SOLID)]);
    call_ok("t_f", "addvisual", &[v_str("t_mb")]);
    call_ok("t_f", "setposition", &[d(0.0), d(0.0), d(5.0)]);
    // No light: black.
    render("t_dx", &mut screen);
    assert_eq!(screen.back.pixel(20, 15), Some(0));
    // A white directional light pointing along +z (at the face's front).
    sc("createlightrgb", &[v_int(3), d(1.0), d(1.0), d(1.0), v_str("t_light")]);
    call_ok("t_lf", "addlight", &[v_str("t_light")]);
    render("t_dx", &mut screen);
    assert_eq!(screen.back.pixel(20, 15), Some(0x0080FF), "orange: (1, 0.5, 0) under white");
    assert_eq!(screen.back.pixel(1, 1), Some(0), "the background");
    // The square: ±1 at z = 5 is ±0.2 of the view's half-width 0.5 at the
    // front plane — 40 × 0.2 = 8 pixels each side of the middle.
    let row: Vec<bool> = (0..40).map(|x| screen.back.pixel(x, 15) == Some(0x0080FF)).collect();
    assert_eq!(row.iter().position(|&b| b), Some(12));
    assert_eq!(row.iter().rposition(|&b| b), Some(27));
    // Turned half a turn about y by Move: its back faces the camera.
    call_ok("t_f", "setrotation", &[d(0.0), d(1.0), d(0.0), d(std::f64::consts::PI)]);
    sc("move", &[d(1.0)]);
    render("t_dx", &mut screen);
    assert_eq!(screen.back.pixel(20, 15), Some(0), "the back isn't drawn");
}

/// A shadow on a floor: none until CreateShadow, then black under the
/// square from the point light above it; none from an ambient light.
#[test]
fn shadows() {
    let mut screen = screen(40, 30);
    for (id, t) in [
        ("t_s_floor", "RD3DMESHBUILDER"),
        ("t_s_box", "RD3DMESHBUILDER"),
        ("t_s_face", "RD3DFACE"),
        ("t_s_ff", "RD3DFRAME"),
        ("t_s_bf", "RD3DFRAME"),
        ("t_s_lf", "RD3DFRAME"),
        ("t_s_amb", "RD3DLIGHT"),
        ("t_s_lamp", "RD3DLIGHT"),
        ("t_s_shadow", "RD3DVISUAL"),
    ] {
        create(id, t);
    }
    let sc = |m: &str, a: &[Value]| screen_call("t_s_dx", &mut DxScreen::default(), m, a).unwrap().unwrap();
    for f in ["t_s_ff", "t_s_bf", "t_s_lf"] {
        sc("createframe", &[v_str(f)]);
    }
    // A white floor at y = -1 (its top towards the camera) ...
    sc("createmeshbuilder", &[v_str("t_s_floor")]);
    sc("createface", &[v_str("t_s_face")]);
    for (x, z) in [(-3.0, 8.0), (3.0, 8.0), (3.0, 2.0), (-3.0, 2.0)] {
        call_ok("t_s_face", "addvertex", &[d(x), d(-1.0), d(z)]);
    }
    call_ok("t_s_floor", "addface", &[v_str("t_s_face")]);
    call_ok("t_s_ff", "addvisual", &[v_str("t_s_floor")]);
    // ... a small square above it, a white ambient light and a point
    // light above the square.
    sc("createmeshbuilder", &[v_str("t_s_box")]);
    sc("createface", &[v_str("t_s_face")]);
    for (x, z) in [(-0.5, 5.5), (0.5, 5.5), (0.5, 4.5), (-0.5, 4.5)] {
        call_ok("t_s_face", "addvertex", &[d(x), d(1.5), d(z)]);
    }
    call_ok("t_s_box", "addface", &[v_str("t_s_face")]);
    call_ok("t_s_bf", "addvisual", &[v_str("t_s_box")]);
    sc("createlightrgb", &[v_int(0), d(1.0), d(1.0), d(1.0), v_str("t_s_amb")]);
    sc("addlight", &[v_str("t_s_amb")]);
    sc("createlightrgb", &[v_int(1), d(1.0), d(1.0), d(1.0), v_str("t_s_lamp")]);
    call_ok("t_s_lf", "addlight", &[v_str("t_s_lamp")]);
    call_ok("t_s_lf", "setposition", &[d(0.0), d(3.0), d(5.0)]);
    render("t_s_dx", &mut screen);
    // (the floor under the square: (0, -1, 5), at (20, 23))
    assert_eq!(screen.back.pixel(20, 23), Some(0xFFFFFF), "no shadow yet");
    // The square's shadow on the floor's plane, from the point light:
    // 4 / 1.5 times the square's size, under it.
    sc("createshadow", &[v_str("t_s_box"), v_str("t_s_lamp"), d(0.0), d(-1.0), d(0.0), d(0.0), d(1.0), d(0.0), v_str("t_s_shadow")]);
    call_ok("t_s_bf", "addvisual", &[v_str("t_s_shadow")]);
    render("t_s_dx", &mut screen);
    assert_eq!(screen.back.pixel(20, 23), Some(0), "in the shadow");
    assert_eq!(screen.back.pixel(36, 23), Some(0xFFFFFF), "(2, -1, 5): beside it");
    // From an ambient light: none.
    call_ok("t_s_bf", "deletevisual", &[v_str("t_s_shadow")]);
    let none = screen_call("t_s_dx", &mut DxScreen::default(), "createshadow", &[v_str("t_s_box"), v_str("t_s_amb"), d(0.0), d(-1.0), d(0.0), d(0.0), d(1.0), d(0.0), v_str("t_s_shadow")]);
    assert!(matches!(none, Some(Ok(_))));
    call_ok("t_s_bf", "addvisual", &[v_str("t_s_shadow")]);
    render("t_s_dx", &mut screen);
    assert_eq!(screen.back.pixel(20, 23), Some(0xFFFFFF));
}
