//! What a `Render` hands the GPU (docs/directx-plan.md, stage D5): the
//! scene as Direct3D Retained Mode lit it — triangles in the camera's
//! space with their colours, texture coordinates and textures, in the
//! order they're drawn — and the viewport it's projected through. The
//! renderer (`rapidr-d3d-gpu`: wgpu on Metal, Vulkan, Direct3D 12 and the
//! browser's WebGL 2) rasterizes it and gives the pixels back, which the
//! QDXSCREEN's back buffer takes — so 2D drawing after `Render`, `Pixel`
//! and `Flip` work as with the 2D layer.
//!
//! The runtime installs the renderer (`set_renderer_factory`); it's made
//! at the first `Render`, so a program without 3D never opens a GPU.

use std::cell::RefCell;
use std::rc::Rc;

use super::math::Vec3;
use crate::objects::bitmap::Bitmap;

/// How a triangle is drawn (D3DRMRENDER_*'s fill mode).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fill {
    /// Its corners, a pixel each.
    Points,
    /// Its edges, a pixel wide.
    Wireframe,
    Solid,
}

/// A triangle to draw: corners in the camera's space (left-handed: x right,
/// y up, z away from the camera), their lit colours (RGBA, 0 … 1 —
/// past 1 where the light is brighter than white: the texture's texel
/// times it is held to 1 at the end), texture coordinates when textured.
#[derive(Clone, Debug)]
pub struct Tri {
    pub p: [Vec3; 3],
    pub c: [[f32; 4]; 3],
    pub uv: Option<[[f32; 2]; 3]>,
    pub texture: Option<Rc<Bitmap>>,
    /// Texture filtering: bilinear (D3DRMTEXTURE_LINEAR), else nearest.
    pub linear: bool,
    pub fill: Fill,
}

impl Tri {
    /// Drawn over what's behind it (an alpha under 1), without hiding
    /// what comes later behind it.
    pub fn blended(&self) -> bool {
        self.fill == Fill::Solid && self.c.iter().any(|c| c[3] < 0.999)
    }
}

/// D3DRM's viewport, in pixels of the target.
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
    /// Pixels per unit of the front plane's square field over the
    /// viewport's larger side, at a distance of 1.
    pub fn scale(&self) -> f64 {
        self.width.max(self.height) as f64 / 2.0 / self.field.max(1e-6) * self.front.max(1e-6)
    }

    /// A camera-space point on the target: x, y (pixels, y down) and 1/z.
    pub fn project(&self, p: Vec3) -> (f64, f64, f64) {
        let k = self.scale();
        let inv = 1.0 / p.z;
        (self.width as f64 / 2.0 + p.x * inv * k, self.height as f64 / 2.0 - p.y * inv * k, inv)
    }
}

/// One `Render`: the view, what's behind everything, the triangles in the
/// order they're drawn (the opaque ones, then the blended ones far to
/// near).
#[derive(Clone, Debug)]
pub struct RenderList {
    pub view: View,
    /// The background colour (&HBBGGRR).
    pub background: u32,
    /// SetBackgroundImage's picture, stretched over the view (nearest).
    pub background_image: Option<Rc<Bitmap>>,
    pub tris: Vec<Tri>,
}

/// What draws a [`RenderList`].
pub trait Renderer {
    /// The pixels (&HBBGGRR, `view.width` × `view.height`, rows top down).
    fn render(&mut self, list: &RenderList) -> Result<Vec<u32>, String>;
    /// The largest side a target may have.
    fn max_side(&self) -> usize;
}

/// Makes the renderer (the GPU's device) at the first `Render`.
pub type Factory = fn() -> Result<Box<dyn Renderer>, String>;

enum State {
    None,
    Factory(Factory),
    Ready(Box<dyn Renderer>),
    Failed,
}

thread_local! {
    // (never dropped: the GPU goes with the process — dropped while the
    // thread's other locals are being destroyed, its pictures reached for
    // ones already gone)
    static STATE: std::mem::ManuallyDrop<RefCell<State>> = const { std::mem::ManuallyDrop::new(RefCell::new(State::None)) };
}

/// The runtime's renderer (once per program; replaced if given again
/// before the first `Render`).
pub fn set_renderer_factory(f: Factory) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if !matches!(*s, State::Ready(_)) {
            *s = State::Factory(f);
        }
    });
}

/// Makes the renderer if it isn't yet (the first `Render`); whether there
/// is one. Without one (a runtime that installs none, or no GPU: said once
/// on stderr) a `Render` shows the background.
pub(crate) fn ready() -> bool {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let State::Factory(f) = *s {
            *s = match f() {
                Ok(r) => State::Ready(r),
                Err(e) => {
                    super::warn(&format!("Direct3D: {e}"));
                    State::Failed
                }
            };
        }
        matches!(*s, State::Ready(_))
    })
}

/// Draws `list` with the renderer ([`ready`] first).
pub(crate) fn draw(list: &RenderList) -> Option<Vec<u32>> {
    STATE.with(|s| match &mut *s.borrow_mut() {
        State::Ready(r) => match r.render(list) {
            Ok(px) if px.len() == list.view.width * list.view.height => Some(px),
            Ok(_) => None,
            Err(e) => {
                super::warn(&format!("Direct3D: {e}"));
                None
            }
        },
        _ => None,
    })
}

/// The largest side the renderer draws.
pub(crate) fn max_side() -> usize {
    STATE.with(|s| match &*s.borrow() {
        State::Ready(r) => r.max_side().max(1),
        _ => usize::MAX,
    })
}
