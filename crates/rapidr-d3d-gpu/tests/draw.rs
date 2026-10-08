//! What the GPU draws for a list of lit triangles: D3DRM's projection,
//! depth and the front plane, colours (past 1 too), textures, blending,
//! wireframes, points, the background picture. (The software rasterizer
//! these were compared with — 0 to 0.4 % of pixels apart, on edges — was
//! the reference until stage D5; docs/directx-plan.md.)

use std::rc::Rc;

use rapidr_d3d_gpu::Gpu;
use rapidr_value::objects::bitmap::Bitmap;
use rapidr_value::objects::d3d::math::v3;
use rapidr_value::objects::d3d::renderer::{Fill, RenderList, Renderer, Tri, View};

const W: usize = 40;
const H: usize = 30;

fn tri(p: [[f64; 3]; 3], c: [f32; 4]) -> Tri {
    Tri { p: p.map(|q| v3(q[0], q[1], q[2])), c: [c; 3], uv: None, texture: None, linear: false, fill: Fill::Solid }
}

/// The deleted rasterizer's test triangle: a point at the top, its base up.
fn at(z: f64, c: [f32; 4]) -> Tri {
    tri([[-1.0, 1.0, z], [1.0, 1.0, z], [0.0, -1.0, z]], c)
}

fn draw(gpu: &mut Gpu, w: usize, h: usize, background: u32, tris: Vec<Tri>) -> Vec<u32> {
    let list = RenderList { view: View { width: w, height: h, front: 1.0, back: 100.0, field: 0.5 }, background, background_image: None, tris };
    gpu.render(&list).unwrap_or_else(|e| panic!("{e}"))
}

fn near(a: u32, b: u32) -> bool {
    (0..3).all(|k| ((a >> (k * 8) & 0xFF) as i32 - (b >> (k * 8) & 0xFF) as i32).abs() <= 1)
}

fn picture(w: usize, h: usize, f: impl Fn(usize, usize) -> u32) -> Rc<Bitmap> {
    let mut b = Bitmap::default();
    b.resize(w as i64, h as i64);
    for y in 0..h {
        for x in 0..w {
            b.img.pixels[y * w + x] = f(x, y);
        }
    }
    Rc::new(b)
}

#[test]
fn what_the_gpu_draws() {
    let mut gpu = Gpu::new().expect("a GPU");
    eprintln!("on {}", gpu.adapter);

    // The projection: the front plane's square field over the larger side
    // (x = ±0.5 at z = 1 at the left and right edges).
    let v = View { width: W, height: H, front: 1.0, back: 100.0, field: 0.5 };
    assert_eq!(v.project(v3(0.5, 0.0, 1.0)).0, 40.0);
    assert_eq!(v.project(v3(0.0, 0.5, 2.0)).1, 15.0 - 10.0);

    // Nearer wins whatever the order; in front of the front plane: nothing.
    let px = draw(&mut gpu, W, H, 0, vec![at(10.0, [1.0, 0.0, 0.0, 1.0]), at(5.0, [0.0, 1.0, 0.0, 1.0]), at(20.0, [0.0, 0.0, 1.0, 1.0])]);
    assert_eq!(px[15 * W + 20], 0x00FF00, "the nearest (green) shows");
    let px = draw(&mut gpu, W, H, 0x302010, vec![at(0.5, [1.0; 4])]);
    assert!(px.iter().all(|&p| p == 0x302010), "in front of the front plane: nothing, the background");
    // A triangle through the front plane: the part behind it shows.
    let px = draw(&mut gpu, W, H, 0, vec![tri([[-0.2, 0.2, 0.5], [3.0, 0.2, 6.0], [-0.2, -3.0, 6.0]], [1.0; 4])]);
    assert!(px.contains(&0xFFFFFF) && px.contains(&0));

    // Half red blended over green, without hiding what's behind.
    let px = draw(&mut gpu, W, H, 0, vec![at(5.0, [0.0, 1.0, 0.0, 1.0]), at(4.0, [1.0, 0.0, 0.0, 0.5])]);
    assert!(near(px[15 * W + 20], 0x008080), "red at half over green: {:06x}", px[15 * W + 20]);

    // Colours past 1 are held to 1 at the end, not before: 1.6 × a
    // texel of 0.5 is 0.8.
    let half = picture(1, 1, |_, _| 0x808080);
    let mut t = at(5.0, [1.6, 0.4, 0.0, 1.0]);
    t.uv = Some([[0.0; 2]; 3]);
    t.texture = Some(half);
    let px = draw(&mut gpu, W, H, 0, vec![t, at(5.5, [1.6, 0.0, 0.0, 1.0])]);
    assert!(near(px[15 * W + 20], 0x0033CD), "(1.6, 0.4, 0) × 0.5: {:06x}", px[15 * W + 20]);

    // Textures repeat and modulate: two texels across, the triangle's
    // u running 0 … 2 — each texel shows twice.
    let tex = picture(2, 1, |x, _| [0x0000FF, 0x00FF00][x]);
    for linear in [false, true] {
        let mut t = at(5.0, [1.0, 1.0, 1.0, 1.0]);
        t.uv = Some([[0.0, 0.0], [2.0, 0.0], [1.0, 1.0]]);
        t.texture = Some(tex.clone());
        t.linear = linear;
        let px = draw(&mut gpu, W, H, 0, vec![t]);
        let row: Vec<u32> = (0..W).map(|x| px[10 * W + x]).filter(|&p| p != 0).collect();
        if linear {
            assert!(row.iter().any(|&p| p != 0x0000FF && p != 0x00FF00), "bilinear: texels blended");
        } else {
            assert!(row.iter().all(|&p| p == 0x0000FF || p == 0x00FF00), "nearest: texels only");
            let changes = row.windows(2).filter(|w| w[0] != w[1]).count();
            assert!(changes >= 2, "red, green, red …");
        }
    }

    // A wireframe: its edges, not its inside; points: its corners.
    let mut lines = at(5.0, [1.0, 1.0, 0.0, 1.0]);
    lines.fill = Fill::Wireframe;
    let px = draw(&mut gpu, W, H, 0, vec![lines]);
    assert_eq!(px[15 * W + 20], 0, "inside");
    assert!(px[6 * W..8 * W].contains(&0x00FFFF), "the top edge (y = 1 at z = 5: 15 - 40 / 5 = row 7)");
    assert!(px[20 * W..21 * W].iter().filter(|&&p| p == 0x00FFFF).count() == 2, "the sides, crossing row 20");
    let mut points = at(5.3, [0.0, 1.0, 1.0, 1.0]);
    points.fill = Fill::Points;
    let px = draw(&mut gpu, W, H, 0, vec![points]);
    assert_eq!(px.iter().filter(|&&p| p == 0xFFFF00).count(), 3, "three corners");

    // A picture wider than the GPU takes: reduced to fit, still drawn.
    let wide = picture(gpu.max_side() + 3, 2, |x, _| if x * 2 < gpu.max_side() { 0x0000FF } else { 0x00FF00 });
    let mut t = at(5.0, [1.0; 4]);
    t.uv = Some([[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]]);
    t.texture = Some(wide.clone());
    let px = draw(&mut gpu, W, H, 0, vec![t]);
    assert!(px.contains(&0x0000FF) && px.contains(&0x00FF00), "both halves");
    let l = RenderList { view: View { width: W, height: H, front: 1.0, back: 100.0, field: 0.5 }, background: 0, background_image: Some(wide), tris: vec![] };
    let px = gpu.render(&l).expect("render");
    assert_eq!((px[0], px[W - 1]), (0x0000FF, 0x00FF00), "the background picture too");

    // The background picture: a texel a pixel as the pixel lands in it.
    let pic = picture(7, 5, |x, y| (x as u32 * 36) | (y as u32 * 50) << 8 | 0x80 << 16);
    let list = RenderList { view: View { width: 150, height: 100, front: 1.0, back: 100.0, field: 0.5 }, background: 0, background_image: Some(pic.clone()), tris: vec![] };
    let px = gpu.render(&list).expect("render");
    for (x, y) in [(0, 0), (149, 99), (21, 14), (22, 20), (75, 50), (100, 61)] {
        assert_eq!(px[y * 150 + x], pic.img.pixels[(y * 5 / 100) * 7 + x * 7 / 150], "({x}, {y})");
    }
}

/// RapidQ's largest model (`direct3d/Park.x`: 16,586 vertices, 29,174
/// faces), lit, at 640 × 480 and twice that: a Render's time (the
/// triangles lit on the CPU, drawn on the GPU, read back). Needs RapidQ's
/// examples (`RAPIDQ_DIR`, default ~/Downloads/Rapidq):
/// `cargo test --release -p rapidr-d3d-gpu -- --ignored --nocapture`.
#[test]
#[ignore]
fn park_x_timing() {
    use rapidr_value::objects::d3d;
    use rapidr_value::objects::directx::DxScreen;
    use rapidr_value::{v_int, v_str, Value};
    let dir = std::env::var("RAPIDQ_DIR").unwrap_or_else(|_| format!("{}/Downloads/Rapidq", std::env::var("HOME").unwrap_or_default()));
    let park = format!("{dir}/examples/direct3d/Park.x");
    assert!(std::path::Path::new(&park).exists(), "{park}");
    let d = Value::Double;
    let mut screen = DxScreen::default();
    for (id, t) in [("p_mb", "RD3DMESHBUILDER"), ("p_f", "RD3DFRAME"), ("p_amb", "RD3DLIGHT"), ("p_l", "RD3DLIGHT"), ("p_lf", "RD3DFRAME")] {
        d3d::create(id, t);
    }
    let sc = |s: &mut DxScreen, m: &str, a: &[Value]| d3d::screen_call("p_dx", s, m, a).unwrap().unwrap();
    sc(&mut screen, "createframe", &[v_str("p_f")]);
    sc(&mut screen, "createframe", &[v_str("p_lf")]);
    sc(&mut screen, "createmeshbuilder", &[v_str("p_mb")]);
    d3d::call("p_mb", "load", &[v_str(&park)]).unwrap().unwrap();
    d3d::call("p_f", "addvisual", &[v_str("p_mb")]).unwrap().unwrap();
    d3d::call("p_f", "setposition", &[d(0.0), d(-5.0), d(60.0)]).unwrap().unwrap();
    sc(&mut screen, "createlightrgb", &[v_int(0), d(0.4), d(0.4), d(0.4), v_str("p_amb")]);
    sc(&mut screen, "addlight", &[v_str("p_amb")]);
    sc(&mut screen, "createlightrgb", &[v_int(3), d(1.0), d(1.0), d(1.0), v_str("p_l")]);
    d3d::call("p_lf", "addlight", &[v_str("p_l")]).unwrap().unwrap();
    d3d::call("p_lf", "setorientation", &[d(1.0), d(-1.0), d(1.0), d(0.0), d(1.0), d(0.0)]).unwrap().unwrap();
    let mut gpu = Gpu::new().expect("a GPU");
    for scale in [1, 2] {
        let (w, h) = (640 * scale, 480 * scale);
        let t0 = std::time::Instant::now();
        let list = d3d::render_list("p_dx", &screen, w, h);
        let lit = t0.elapsed();
        let px = gpu.render(&list).expect("render");
        assert!(px.iter().filter(|&&p| p != 0).count() > w * h / 50, "the park is drawn");
        let frames = 30;
        let t1 = std::time::Instant::now();
        for _ in 0..frames {
            let list = d3d::render_list("p_dx", &screen, w, h);
            gpu.render(&list).expect("render");
        }
        eprintln!("Park.x {w}x{h}: {} triangles, lit in {lit:?}; a Render on {}: {:?}", list.tris.len(), gpu.adapter, t1.elapsed() / frames);
    }
}
