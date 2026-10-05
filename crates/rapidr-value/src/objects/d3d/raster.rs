//! The software rasterizer Direct3D Retained Mode's scenes are drawn with
//! (docs/directx-plan.md, stage D4's reference renderer): triangles in the
//! camera's space, clipped to the front and back planes, projected as
//! D3DRM's viewport did — a square field of view (`field`, 0.5 at the front
//! plane by default) over the viewport's larger side — and filled with a
//! depth buffer: flat or Gouraud colours, textures (nearest or bilinear,
//! perspective-correct, repeating), alpha blended where asked. Lines and
//! points for wireframe and point modes. Colours out are RapidQ's
//! &HBBGGRR; in, linear 0 … 1 RGBA.

use std::rc::Rc;

use super::math::Vec3;
use crate::objects::bitmap::Bitmap;

/// How a triangle is filled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    Points,
    Wireframe,
    Solid,
}

/// A triangle to draw: corners in the camera's space, their lit colours
/// (RGBA 0 … 1), texture coordinates when textured.
#[derive(Clone, Debug)]
pub struct Tri {
    pub p: [Vec3; 3],
    pub c: [[f32; 4]; 3],
    pub uv: Option<[[f32; 2]; 3]>,
    pub texture: Option<Rc<Bitmap>>,
    /// Texture filtering: bilinear (D3DRMTEXTURE_LINEAR and up).
    pub linear: bool,
    pub fill: Fill,
}

/// The viewport's projection (pixels of the target).
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub width: usize,
    pub height: usize,
    /// The front and back clipping planes' distances.
    pub front: f64,
    pub back: f64,
    /// Half the front plane's side (SetField; 0.5).
    pub field: f64,
}

impl View {
    /// Pixels per unit of the front plane: its square field over the
    /// viewport's larger side.
    fn scale(&self) -> f64 {
        self.width.max(self.height) as f64 / 2.0 / self.field.max(1e-6) * self.front.max(1e-6)
    }

    /// A camera-space point on the target: x, y (pixels) and 1/z.
    pub fn project(&self, p: Vec3) -> (f64, f64, f64) {
        let k = self.scale();
        let inv = 1.0 / p.z;
        (self.width as f64 / 2.0 + p.x * inv * k, self.height as f64 / 2.0 - p.y * inv * k, inv)
    }
}

/// A target: colours (&HBBGGRR) and depths (1/z, larger nearer).
pub struct Target<'a> {
    pub width: usize,
    pub height: usize,
    pub color: &'a mut [u32],
    pub depth: &'a mut [f32],
}

fn to_bgr(c: [f32; 4]) -> u32 {
    let ch = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    ch(c[2]) << 16 | ch(c[1]) << 8 | ch(c[0])
}

fn from_bgr(c: u32) -> [f32; 3] {
    [(c & 0xFF) as f32 / 255.0, (c >> 8 & 0xFF) as f32 / 255.0, (c >> 16 & 0xFF) as f32 / 255.0]
}

/// A corner after clipping: position, colour, texture coordinates.
#[derive(Clone, Copy, Debug)]
struct Corner {
    p: Vec3,
    c: [f32; 4],
    uv: [f32; 2],
}

fn lerp_corner(a: &Corner, b: &Corner, t: f64) -> Corner {
    let tf = t as f32;
    let mut c = [0.0; 4];
    for (i, v) in c.iter_mut().enumerate() {
        *v = a.c[i] + (b.c[i] - a.c[i]) * tf;
    }
    Corner { p: a.p + (b.p - a.p) * t, c, uv: [a.uv[0] + (b.uv[0] - a.uv[0]) * tf, a.uv[1] + (b.uv[1] - a.uv[1]) * tf] }
}

/// The polygon's part with z ≥ `z` (`keep_greater`) or ≤ `z`.
fn clip_z(poly: &[Corner], z: f64, keep_greater: bool) -> Vec<Corner> {
    let inside = |c: &Corner| if keep_greater { c.p.z >= z } else { c.p.z <= z };
    let mut out = Vec::with_capacity(poly.len() + 2);
    for i in 0..poly.len() {
        let (a, b) = (&poly[i], &poly[(i + 1) % poly.len()]);
        if inside(a) {
            out.push(*a);
        }
        if inside(a) != inside(b) {
            let t = (z - a.p.z) / (b.p.z - a.p.z);
            out.push(lerp_corner(a, b, t));
        }
    }
    out
}

/// Draws a triangle (clipped to the view's planes).
pub fn draw(t: &mut Target, view: &View, tri: &Tri) {
    let uv = tri.uv.unwrap_or([[0.0; 2]; 3]);
    let poly: Vec<Corner> = (0..3).map(|i| Corner { p: tri.p[i], c: tri.c[i], uv: uv[i] }).collect();
    let poly = clip_z(&poly, view.front, true);
    let poly = clip_z(&poly, view.back, false);
    if poly.len() < 3 {
        return;
    }
    match tri.fill {
        Fill::Points => {
            for c in &poly {
                let (x, y, w) = view.project(c.p);
                plot(t, x as i64, y as i64, w as f32, to_bgr(c.c));
            }
        }
        Fill::Wireframe => {
            for i in 0..poly.len() {
                line(t, view, &poly[i], &poly[(i + 1) % poly.len()]);
            }
        }
        Fill::Solid => {
            for i in 1..poly.len() - 1 {
                fill(t, view, [&poly[0], &poly[i], &poly[i + 1]], tri);
            }
        }
    }
}

fn plot(t: &mut Target, x: i64, y: i64, w: f32, c: u32) {
    if x < 0 || y < 0 || x as usize >= t.width || y as usize >= t.height {
        return;
    }
    let i = y as usize * t.width + x as usize;
    if w >= t.depth[i] {
        t.depth[i] = w;
        t.color[i] = c;
    }
}

fn line(t: &mut Target, view: &View, a: &Corner, b: &Corner) {
    let (x0, y0, w0) = view.project(a.p);
    let (x1, y1, w1) = view.project(b.p);
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).ceil().max(1.0) as i64;
    for s in 0..=steps.min(1 << 16) {
        let k = s as f64 / steps as f64;
        let c = [0, 1, 2, 3].map(|i| a.c[i] + (b.c[i] - a.c[i]) * k as f32);
        plot(t, (x0 + (x1 - x0) * k) as i64, (y0 + (y1 - y0) * k) as i64, (w0 + (w1 - w0) * k) as f32 * 1.000_01, to_bgr(c));
    }
}

/// A texel of `tex` at (u, v), repeating (RGB 0 … 1).
fn texel(tex: &Bitmap, u: f32, v: f32, linear: bool) -> [f32; 3] {
    let (w, h) = (tex.img.width, tex.img.height);
    if w == 0 || h == 0 {
        return [1.0; 3];
    }
    let at = |x: i64, y: i64| from_bgr(tex.img.pixels[y.rem_euclid(h as i64) as usize * w + x.rem_euclid(w as i64) as usize]);
    let (fx, fy) = (u * w as f32, v * h as f32);
    if !linear {
        return at(fx.floor() as i64, fy.floor() as i64);
    }
    let (fx, fy) = (fx - 0.5, fy - 0.5);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let (x0, y0) = (x0 as i64, y0 as i64);
    let (a, b, c, d) = (at(x0, y0), at(x0 + 1, y0), at(x0, y0 + 1), at(x0 + 1, y0 + 1));
    [0, 1, 2].map(|i| (a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty) + (c[i] * (1.0 - tx) + d[i] * tx) * ty)
}

/// Fills a (clipped) triangle: edge functions over its bounding box,
/// attributes interpolated perspective-correctly (over 1/z).
fn fill(t: &mut Target, view: &View, c: [&Corner; 3], tri: &Tri) {
    let s: Vec<(f64, f64, f64)> = c.iter().map(|k| view.project(k.p)).collect();
    let area = (s[1].0 - s[0].0) * (s[2].1 - s[0].1) - (s[2].0 - s[0].0) * (s[1].1 - s[0].1);
    if area.abs() < 1e-12 {
        return;
    }
    let min_x = s.iter().map(|p| p.0).fold(f64::INFINITY, f64::min).floor().max(0.0) as i64;
    let max_x = s.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max).ceil().min(t.width as f64) as i64;
    let min_y = s.iter().map(|p| p.1).fold(f64::INFINITY, f64::min).floor().max(0.0) as i64;
    let max_y = s.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max).ceil().min(t.height as f64) as i64;
    let textured = tri.uv.is_some() && tri.texture.is_some();
    for y in min_y..max_y {
        for x in min_x..max_x {
            let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
            let edge = |a: (f64, f64, f64), b: (f64, f64, f64)| (b.0 - a.0) * (py - a.1) - (px - a.0) * (b.1 - a.1);
            let w0 = edge(s[1], s[2]) / area;
            let w1 = edge(s[2], s[0]) / area;
            let w2 = 1.0 - w0 - w1;
            // (inside, ties on the top-left edges as the pixel's)
            if w0 < -1e-9 || w1 < -1e-9 || w2 < -1e-9 {
                continue;
            }
            let inv_z = w0 * s[0].2 + w1 * s[1].2 + w2 * s[2].2;
            let i = y as usize * t.width + x as usize;
            if (inv_z as f32) < t.depth[i] {
                continue;
            }
            // Perspective-correct weights.
            let (q0, q1, q2) = (w0 * s[0].2 / inv_z, w1 * s[1].2 / inv_z, w2 * s[2].2 / inv_z);
            let at = |k: usize| (c[0].c[k] as f64 * q0 + c[1].c[k] as f64 * q1 + c[2].c[k] as f64 * q2) as f32;
            let mut rgb = [at(0), at(1), at(2)];
            let alpha = at(3);
            if textured {
                let u = (c[0].uv[0] as f64 * q0 + c[1].uv[0] as f64 * q1 + c[2].uv[0] as f64 * q2) as f32;
                let v = (c[0].uv[1] as f64 * q0 + c[1].uv[1] as f64 * q1 + c[2].uv[1] as f64 * q2) as f32;
                let tx = texel(tri.texture.as_deref().unwrap_or(&Bitmap::default()), u, v, tri.linear);
                rgb = [rgb[0] * tx[0], rgb[1] * tx[1], rgb[2] * tx[2]];
            }
            if alpha < 0.999 {
                // (blended: behind what's drawn, the depth kept)
                let under = from_bgr(t.color[i]);
                let a = alpha.clamp(0.0, 1.0);
                rgb = [0, 1, 2].map(|k| rgb[k] * a + under[k] * (1.0 - a));
                t.color[i] = to_bgr([rgb[0], rgb[1], rgb[2], 1.0]);
            } else {
                t.depth[i] = inv_z as f32;
                t.color[i] = to_bgr([rgb[0], rgb[1], rgb[2], 1.0]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::d3d::math::v3;

    fn target(w: usize, h: usize) -> (Vec<u32>, Vec<f32>) {
        (vec![0; w * h], vec![0.0; w * h])
    }

    const VIEW: View = View { width: 40, height: 30, front: 1.0, back: 100.0, field: 0.5 };

    fn tri(z: f64, color: [f32; 4]) -> Tri {
        Tri { p: [v3(-1.0, 1.0, z), v3(1.0, 1.0, z), v3(0.0, -1.0, z)], c: [color; 3], uv: None, texture: None, linear: false, fill: Fill::Solid }
    }

    /// The projection: the front plane's square field over the larger side
    /// (x = ±0.5 at z = 1 at the left and right edges); nearer wins; the
    /// part behind the front plane is cut.
    #[test]
    fn projection_depth_and_clipping() {
        assert_eq!(VIEW.project(v3(0.5, 0.0, 1.0)).0, 40.0);
        assert_eq!(VIEW.project(v3(0.0, 0.5, 2.0)).1, 15.0 - 10.0);
        let (mut c, mut d) = target(40, 30);
        let mut t = Target { width: 40, height: 30, color: &mut c, depth: &mut d };
        draw(&mut t, &VIEW, &tri(10.0, [1.0, 0.0, 0.0, 1.0]));
        draw(&mut t, &VIEW, &tri(5.0, [0.0, 1.0, 0.0, 1.0]));
        draw(&mut t, &VIEW, &tri(20.0, [0.0, 0.0, 1.0, 1.0]));
        assert_eq!(t.color[15 * 40 + 20], 0x00FF00, "the nearest (green) shows");
        let (mut c2, mut d2) = target(40, 30);
        let mut t2 = Target { width: 40, height: 30, color: &mut c2, depth: &mut d2 };
        draw(&mut t2, &VIEW, &tri(0.5, [1.0, 1.0, 1.0, 1.0]));
        assert!(t2.color.iter().all(|&p| p == 0), "in front of the front plane: nothing");
        // Half blended over green.
        draw(&mut t, &VIEW, &tri(4.0, [1.0, 0.0, 0.0, 0.5]));
        assert_eq!(t.color[15 * 40 + 20], 0x0080 << 8 | 0x80, "red at half over green");
    }

    #[test]
    fn textures_repeat_and_modulate() {
        let mut tex = Bitmap::default();
        tex.resize(2, 1);
        tex.img.pixels = vec![0x0000FF, 0x00FF00];
        let tex = Rc::new(tex);
        let mut t = tri(5.0, [1.0, 1.0, 1.0, 1.0]);
        t.uv = Some([[0.0, 0.0], [2.0, 0.0], [1.0, 1.0]]);
        t.texture = Some(tex);
        let (mut c, mut d) = target(40, 30);
        let mut tg = Target { width: 40, height: 30, color: &mut c, depth: &mut d };
        draw(&mut tg, &VIEW, &t);
        let row: Vec<u32> = (0..40).map(|x| tg.color[10 * 40 + x]).filter(|&p| p != 0).collect();
        assert!(row.contains(&0x0000FF) && row.contains(&0x00FF00), "both texels, repeated");
    }
}
