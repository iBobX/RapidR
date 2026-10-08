//! RapidQ's Direct3D objects (manual, Appendix B: QD3DFRAME, QD3DMESHBUILDER,
//! QD3DMESH, QD3DFACE, QD3DLIGHT, QD3DTEXTURE, QD3DWRAP, QD3DVECTOR,
//! QD3DVISUAL, QD3DANIMATION, QD3DANIMATIONSET, and QDXSCREEN's 3D methods;
//! chapter 13; docs/directx-plan.md
//! stages D3–D4): Direct3D Retained Mode's scene, kept here for every
//! runtime and drawn by the software rasterizer (`raster.rs`) into the
//! QDXSCREEN's back buffer at `Render` — so 2D drawing after it, `Pixel`
//! and `Flip` work as with the 2D layer.
//!
//! - **Handles, as COM's interfaces.** A QD3D* variable holds a reference
//!   to a scene object: `DXScreen.CreateFrame(F)` makes a new frame and
//!   points F at it; a frame keeps what was added to it (`AddVisual`,
//!   `AddLight`), so making a new face or light into the same variable
//!   (RapidQ programs do, in a loop) leaves the old one where it was put.
//! - **The scene**: a hidden root frame per screen, the camera a frame on it
//!   (SetCameraPosition, SetCameraOrientation, CameraLookAt). A frame's
//!   matrix places it in its parent's space (SetPosition, SetOrientation,
//!   AddScale); SetRotation gives it a turn per `Move` about an axis in its
//!   parent's space (an axis of (0, 0, 0): x, as D3DRM's); `DXScreen.Move(d)` turns
//!   and moves every frame d times its step, as `IDirect3DRMFrame::Move`.
//! - **Meshes**: faces of their own vertices (`Face.AddVertex`,
//!   `MeshBuilder.AddFace`), or a `.X` file (`Load`: every mesh of it in
//!   one, its frames' matrices applied; materials' colours and texture file
//!   names), a colour per face (Face.SetColorRGB; MeshBuilder.SetRGB /
//!   SetRGBA all of them), a quality (D3DRMRENDER_*: points, wireframe or
//!   solid; flat or Gouraud — Phong as Gouraud, as D3DRM did; lit or not),
//!   a texture (LoadTexture, SetTexture) with the file's coordinates or a
//!   wrap's (QD3DWRAP: flat, cylinder, sphere — chrome as sphere).
//! - **Light**: D3DRM's RGB model — ambient lights' colours plus each
//!   directional, point, spot and parallel-point light's colour times the
//!   cosine of its angle to the surface, times the surface's colour; no
//!   light, no colour. Back faces (counter-clockwise from the camera) are
//!   not drawn.
//! - **The view**: D3DRM's viewport — a square field of view of half-side
//!   0.5 at the front plane (1), over the viewport's larger side; the back
//!   plane 5000 (the manual's View.SetBack note).

pub mod math;
pub mod raster;
pub mod xfile;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use math::{v3, Mat4, Vec3};
use raster::{Fill, Tri};

use super::bitmap::Bitmap;
use super::directx::DxScreen;
use crate::{v_int, Value};

/// RapidR's names for the QD3D* types.
pub const TYPES: &[&str] = &["RD3DFRAME", "RD3DMESHBUILDER", "RD3DMESH", "RD3DFACE", "RD3DLIGHT", "RD3DTEXTURE", "RD3DVISUAL", "RD3DWRAP", "RD3DVECTOR", "RD3DANIMATION", "RD3DANIMATIONSET"];

// D3DRMRENDERQUALITY's parts (RapidQ_D3D.inc).
const SHADE_MASK: i64 = 7;
const LIGHT_ON: i64 = 8;
const FILL_MASK: i64 = 448;
const FILL_WIREFRAME: i64 = 64;
const FILL_SOLID: i64 = 128;
/// D3DRMRENDER_GOURAUD: a mesh's quality until SetQuality.
const RENDER_GOURAUD: i64 = 1 + LIGHT_ON + FILL_SOLID;
/// D3DRMRENDERMODE_BLENDEDTRANSPARENCY.
const BLENDED: i64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Frame,
    MeshBuilder,
    Mesh,
    Face,
    Light,
    Texture,
    Visual,
    Wrap,
    Vector,
    /// QD3DANIMATION / QD3DANIMATIONSET: RC.EXE knows them with one member,
    /// Parent, and nothing makes or plays one (no QDXSCREEN method creates
    /// them, RapidQ's manual and examples never use them) — a handle that
    /// refers to nothing, as QD3DTEXTURE's before LoadTexture.
    Animation,
    AnimationSet,
}

fn kind_of(type_name: &str) -> Option<Kind> {
    Some(match type_name.to_ascii_uppercase().as_str() {
        "RD3DFRAME" => Kind::Frame,
        "RD3DMESHBUILDER" => Kind::MeshBuilder,
        "RD3DMESH" => Kind::Mesh,
        "RD3DFACE" => Kind::Face,
        "RD3DLIGHT" => Kind::Light,
        "RD3DTEXTURE" => Kind::Texture,
        "RD3DVISUAL" => Kind::Visual,
        "RD3DWRAP" => Kind::Wrap,
        "RD3DVECTOR" => Kind::Vector,
        "RD3DANIMATION" => Kind::Animation,
        "RD3DANIMATIONSET" => Kind::AnimationSet,
        _ => return None,
    })
}

#[derive(Clone, Debug)]
struct Frame {
    parent: Option<usize>,
    children: Vec<usize>,
    /// Its place in its parent's space.
    m: Mat4,
    /// SetRotation: axis (parent's space) and radians a Move step.
    rot: Option<(Vec3, f64)>,
    /// SetVelocity: a Move step's way (parent's space).
    vel: Vec3,
    visuals: Vec<usize>,
    lights: Vec<usize>,
}

impl Frame {
    fn new(parent: Option<usize>) -> Self {
        Frame { parent, children: Vec::new(), m: Mat4::IDENTITY, rot: None, vel: Vec3::default(), visuals: Vec::new(), lights: Vec::new() }
    }
}

#[derive(Clone, Debug)]
struct MFace {
    idx: Vec<usize>,
    /// Normals per corner (indexes into `Mesh::normals`); empty: the
    /// vertices' own.
    nidx: Vec<usize>,
    color: [f32; 4],
    /// The face's own texture (a `.X` file's material's); none: the
    /// mesh's.
    texture: Option<Rc<Bitmap>>,
    /// The QD3DFACE this face is (AddFace, GetFace), if any.
    face: Option<usize>,
    /// Its material's specular highlight, `(power, colour)` (a `.X`
    /// file's: `power` above 0 and a colour), if any.
    spec: Option<(f32, [f32; 3])>,
}

#[derive(Clone, Debug)]
struct Mesh {
    verts: Vec<Vec3>,
    /// Given normals (a `.X` file's), indexed by faces' `nidx`.
    normals: Vec<Vec3>,
    /// Texture coordinates, one a vertex (empty: none).
    uvs: Vec<[f32; 2]>,
    faces: Vec<MFace>,
    texture: Option<Rc<Bitmap>>,
    quality: i64,
}

impl Default for Mesh {
    fn default() -> Self {
        Mesh { verts: Vec::new(), normals: Vec::new(), uvs: Vec::new(), faces: Vec::new(), texture: None, quality: RENDER_GOURAUD }
    }
}

impl Mesh {
    /// SetTexture / LoadTexture: every face's texture (D3DRM's
    /// IDirect3DRMMeshBuilder::SetTexture), the faces' own ones replaced.
    fn set_texture(&mut self, t: Option<Rc<Bitmap>>) {
        for f in &mut self.faces {
            f.texture = None;
        }
        self.texture = t;
    }
}

#[derive(Clone, Debug, Default)]
struct Face {
    verts: Vec<Vec3>,
    color: Option<[f32; 4]>,
    /// The mesh builder it was added to (D3DRM: a face added to one is
    /// the mesh's — what's done to it after shows there).
    owner: Option<usize>,
}

#[derive(Clone, Debug)]
struct Light {
    kind: i64,
    color: Vec3,
    range: f64,
    umbra: f64,
    penumbra: f64,
}

#[derive(Clone, Debug)]
struct Wrap {
    kind: i64,
    origin: Vec3,
    dir: Vec3,
    up: Vec3,
    offset: (f64, f64),
    scale: (f64, f64),
}

/// CreateShadow's visual: `visual`'s shadow, cast by `light` onto the
/// plane through `point` with normal `normal` (the scene's space).
#[derive(Clone, Debug)]
struct Shadow {
    visual: usize,
    light: usize,
    point: Vec3,
    normal: Vec3,
}

#[derive(Clone, Debug)]
enum Ent {
    Frame(Frame),
    Mesh(Mesh),
    Face(Face),
    Light(Light),
    Texture(Rc<Bitmap>),
    Wrap(Wrap),
    Shadow(Shadow),
}

/// A screen's scene and view.
#[derive(Clone, Debug)]
struct Scene {
    root: usize,
    camera: usize,
    render_mode: i64,
    texture_quality: i64,
    background: [f32; 3],
    background_image: Option<Rc<Bitmap>>,
}

#[derive(Clone, Copy, Debug)]
struct Ref {
    kind: Kind,
    handle: Option<usize>,
}

#[derive(Default)]
struct Store {
    ents: Vec<Ent>,
    refs: HashMap<String, Ref>,
    vectors: HashMap<String, [f64; 3]>,
    scenes: HashMap<String, Scene>,
}

thread_local! {
    static D3D: RefCell<Store> = RefCell::new(Store::default());
}

fn store<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    D3D.with(|s| f(&mut s.borrow_mut()))
}

/// Makes QD3D* object `id` (a variable referring to nothing yet);
/// `false` if `type_name` isn't one.
pub fn create(id: &str, type_name: &str) -> bool {
    let Some(kind) = kind_of(type_name) else { return false };
    store(|s| {
        s.refs.entry(id.to_lowercase()).or_insert(Ref { kind, handle: None });
    });
    true
}

/// Whether `id` is a QD3D* object.
pub fn exists(id: &str) -> bool {
    store(|s| s.refs.contains_key(&id.to_lowercase()))
}

impl Store {
    fn add(&mut self, e: Ent) -> usize {
        self.ents.push(e);
        self.ents.len() - 1
    }

    fn handle(&self, id: &str, kind: Kind) -> Option<usize> {
        self.refs.get(&id.to_lowercase()).filter(|r| r.kind == kind || (kind == Kind::Mesh && r.kind == Kind::MeshBuilder) || (kind == Kind::MeshBuilder && r.kind == Kind::Mesh)).and_then(|r| r.handle)
    }

    /// The visual a variable refers to: a mesh builder or a mesh.
    fn visual(&self, id: &str) -> Option<usize> {
        let r = self.refs.get(&id.to_lowercase())?;
        matches!(r.kind, Kind::MeshBuilder | Kind::Mesh | Kind::Visual).then_some(r.handle)?
    }

    fn point(&mut self, id: &str, kind: Kind, h: usize) {
        self.refs.insert(id.to_lowercase(), Ref { kind, handle: Some(h) });
    }

    fn frame(&mut self, h: usize) -> Option<&mut Frame> {
        match self.ents.get_mut(h) {
            Some(Ent::Frame(f)) => Some(f),
            _ => None,
        }
    }

    fn mesh(&mut self, h: usize) -> Option<&mut Mesh> {
        match self.ents.get_mut(h) {
            Some(Ent::Mesh(m)) => Some(m),
            _ => None,
        }
    }

    /// Screen `id`'s scene, made the first time.
    fn scene(&mut self, id: &str) -> Scene {
        let key = id.to_lowercase();
        if let Some(s) = self.scenes.get(&key) {
            return s.clone();
        }
        let root = self.add(Ent::Frame(Frame::new(None)));
        let camera = self.add(Ent::Frame(Frame::new(Some(root))));
        if let Some(f) = self.frame(root) {
            f.children.push(camera);
        }
        let s = Scene { root, camera, render_mode: 0, texture_quality: 0, background: [0.0; 3], background_image: None };
        self.scenes.insert(key, s.clone());
        s
    }

    fn scene_mut(&mut self, id: &str) -> &mut Scene {
        self.scene(id);
        self.scenes.get_mut(&id.to_lowercase()).expect("made above")
    }

    /// The scene frame `h` is in (its topmost frame's).
    fn scene_of(&mut self, h: usize) -> Option<&mut Scene> {
        let mut top = h;
        for _ in 0..256 {
            match self.frame(top).and_then(|f| f.parent) {
                Some(p) => top = p,
                None => break,
            }
        }
        self.scenes.values_mut().find(|sc| sc.root == top)
    }

    /// Puts frame `child` under `parent` (out of where it was).
    fn reparent(&mut self, child: usize, parent: Option<usize>) {
        if let Some(old) = self.frame(child).and_then(|f| f.parent) {
            if let Some(p) = self.frame(old) {
                p.children.retain(|&c| c != child);
            }
        }
        if let Some(f) = self.frame(child) {
            f.parent = parent;
        }
        if let Some(p) = parent.and_then(|p| self.frame(p)) {
            if !p.children.contains(&child) {
                p.children.push(child);
            }
        }
    }

    /// A frame's matrix to the scene's space.
    fn world(&self, h: usize) -> Mat4 {
        let mut m = Mat4::IDENTITY;
        let mut at = Some(h);
        for _ in 0..256 {
            let Some(i) = at else { break };
            let Some(Ent::Frame(f)) = self.ents.get(i) else { break };
            m = m.mul(&f.m);
            at = f.parent;
        }
        m
    }
}

fn arg_f(args: &[Value], i: usize) -> f64 {
    args.get(i).map_or(0.0, Value::to_f64)
}

fn arg_v(args: &[Value], i: usize) -> Vec3 {
    v3(arg_f(args, i), arg_f(args, i + 1), arg_f(args, i + 2))
}

fn arg_id(args: &[Value], i: usize) -> String {
    args.get(i).map(Value::to_string_val).unwrap_or_default().to_lowercase()
}

fn rgba(r: f64, g: f64, b: f64, a: f64) -> [f32; 4] {
    [r as f32, g as f32, b as f32, a as f32]
}

// ------------------------------------------------------------ QDXSCREEN --

/// Whether `method` is one of QDXSCREEN's 3D methods.
pub fn is_screen_method(method: &str) -> bool {
    matches!(
        method,
        "createframe"
            | "createmeshbuilder"
            | "createface"
            | "createlightrgb"
            | "addlight"
            | "setcameraposition"
            | "setcameraorientation"
            | "cameralookat"
            | "move"
            | "render"
            | "forceupdate"
            | "setrendermode"
            | "settexturequality"
            | "loadtexture"
            | "setbackgroundimage"
            | "createwrap"
            | "createshadow"
            | "setvelocity"
    )
}

/// A QDXSCREEN's 3D method (`screen`: its 2D side, whose back buffer
/// `Render` draws on).
pub fn screen_call(id: &str, screen: &mut DxScreen, method: &str, args: &[Value]) -> Option<Result<Value, String>> {
    let r = match method {
        "createframe" => {
            let target = arg_id(args, 0);
            store(|s| {
                let scene = s.scene(id);
                let h = s.add(Ent::Frame(Frame::new(Some(scene.root))));
                if let Some(r) = s.frame(scene.root) {
                    r.children.push(h);
                }
                s.point(&target, Kind::Frame, h);
            });
            Ok(())
        }
        "createmeshbuilder" => {
            let target = arg_id(args, 0);
            store(|s| {
                let h = s.add(Ent::Mesh(Mesh::default()));
                s.point(&target, Kind::MeshBuilder, h);
            });
            Ok(())
        }
        "createface" => {
            let target = arg_id(args, 0);
            store(|s| {
                let h = s.add(Ent::Face(Face::default()));
                s.point(&target, Kind::Face, h);
            });
            Ok(())
        }
        // CreateLightRGB(LightType, R, G, B, Light)
        "createlightrgb" => {
            let target = arg_id(args, 4);
            let light = Light { kind: args.first().map_or(0, Value::to_i64), color: arg_v(args, 1), range: 256.0, umbra: 0.4, penumbra: 0.5 };
            store(|s| {
                let h = s.add(Ent::Light(light));
                s.point(&target, Kind::Light, h);
            });
            Ok(())
        }
        "addlight" => {
            let light = arg_id(args, 0);
            store(|s| {
                let scene = s.scene(id);
                if let Some(l) = s.handle(&light, Kind::Light) {
                    if let Some(f) = s.frame(scene.root) {
                        f.lights.push(l);
                    }
                }
            });
            Ok(())
        }
        "setcameraposition" => {
            store(|s| {
                let cam = s.scene(id).camera;
                if let Some(f) = s.frame(cam) {
                    f.m.set_origin(arg_v(args, 0));
                }
            });
            Ok(())
        }
        "setcameraorientation" => {
            store(|s| {
                let cam = s.scene(id).camera;
                if let Some(f) = s.frame(cam) {
                    f.m = Mat4::oriented(arg_v(args, 0), arg_v(args, 3), f.m.origin());
                }
            });
            Ok(())
        }
        "cameralookat" => {
            let target = arg_id(args, 0);
            store(|s| {
                let cam = s.scene(id).camera;
                if let Some(t) = s.handle(&target, Kind::Frame) {
                    look_at(s, cam, t);
                }
            });
            Ok(())
        }
        "move" => {
            store(|s| {
                let root = s.scene(id).root;
                move_frames(s, root, arg_f(args, 0));
            });
            Ok(())
        }
        "render" => {
            render(id, screen);
            Ok(())
        }
        // (the whole view is drawn at each Render)
        "forceupdate" => Ok(()),
        "setrendermode" => {
            store(|s| s.scene_mut(id).render_mode = args.first().map_or(0, Value::to_i64));
            Ok(())
        }
        "settexturequality" => {
            store(|s| s.scene_mut(id).texture_quality = args.first().map_or(0, Value::to_i64));
            Ok(())
        }
        // LoadTexture(File, Texture)
        "loadtexture" => load_texture(&args.first().map(Value::to_string_val).unwrap_or_default()).map(|t| {
            let target = arg_id(args, 1);
            store(|s| {
                let h = s.add(Ent::Texture(t));
                s.point(&target, Kind::Texture, h);
            });
        }),
        "setbackgroundimage" => {
            let tex = arg_id(args, 0);
            store(|s| {
                let image = s.handle(&tex, Kind::Texture).and_then(|h| match s.ents.get(h) {
                    Some(Ent::Texture(t)) => Some(t.clone()),
                    _ => None,
                });
                s.scene_mut(id).background_image = image;
            });
            Ok(())
        }
        // CreateWrap(Type, ox, oy, oz, dx, dy, dz, ux, uy, uz, ou, ov, su, sv, Wrap)
        "createwrap" => {
            let target = arg_id(args, 14);
            let w = Wrap {
                kind: args.first().map_or(0, Value::to_i64),
                origin: arg_v(args, 1),
                dir: arg_v(args, 4),
                up: arg_v(args, 7),
                offset: (arg_f(args, 10), arg_f(args, 11)),
                scale: (arg_f(args, 12), arg_f(args, 13)),
            };
            store(|s| {
                let h = s.add(Ent::Wrap(w));
                s.point(&target, Kind::Wrap, h);
            });
            Ok(())
        }
        // CreateShadow(Visual, Light, px, py, pz, nx, ny, nz, Shadow)
        "createshadow" => {
            let (visual, light, target) = (arg_id(args, 0), arg_id(args, 1), arg_id(args, 8));
            store(|s| {
                let (Some(visual), Some(light)) = (s.visual(&visual), s.handle(&light, Kind::Light)) else {
                    return Err(format!("{id}.CreateShadow: the visual or the light isn't made yet"));
                };
                let h = s.add(Ent::Shadow(Shadow { visual, light, point: arg_v(args, 2), normal: arg_v(args, 5) }));
                s.point(&target, Kind::Visual, h);
                Ok(())
            })
        }
        // SetVelocity(x, y, z, WithRotation): the camera's (a hidden frame,
        // the manual says).
        "setvelocity" => {
            let camera = store(|s| s.scene(id).camera);
            return frame_call(id, Some(camera), method, args).map(|r| r.map(|_| Value::Null));
        }
        _ => return None,
    };
    Some(r.map(|_| Value::Null))
}

/// Frame `h` turned to look at frame `target` (its z axis toward it), its
/// up kept as near its old one as can be.
fn look_at(s: &mut Store, h: usize, target: usize) {
    let to = s.world(target).origin();
    let parent = s.frame(h).and_then(|f| f.parent);
    let parent_world = parent.map_or(Mat4::IDENTITY, |p| s.world(p));
    let to_local = parent_world.inverse().point(to);
    if let Some(f) = s.frame(h) {
        let from = f.m.origin();
        let up = f.m.axis(1);
        f.m = Mat4::oriented(to_local - from, up, from);
    }
}

/// `Move(delta)` on frame `h` and those under it: each turned about its own
/// origin by its rotation's step times delta, moved by its velocity.
fn move_frames(s: &mut Store, h: usize, delta: f64) {
    let mut todo = vec![h];
    let mut seen = 0;
    while let Some(i) = todo.pop() {
        seen += 1;
        if seen > 100_000 {
            break;
        }
        let Some(f) = s.frame(i) else { continue };
        if let Some((axis, theta)) = f.rot {
            let at = f.m.origin();
            let mut m = f.m;
            m.set_origin(Vec3::default());
            m = m.mul(&Mat4::rotation(axis, theta * delta));
            m.set_origin(at + f.vel * delta);
            f.m = m;
        } else if f.vel != Vec3::default() {
            let at = f.m.origin() + f.vel * delta;
            f.m.set_origin(at);
        }
        todo.extend(f.children.iter().copied());
    }
}

/// A texture file (BMP, PNG, JPEG — what the shared decoder reads).
fn load_texture(file: &str) -> Result<Rc<Bitmap>, String> {
    let bytes = super::read_file(file)?;
    let mut b = Bitmap::default();
    b.load_bmp_bytes(&bytes)?;
    Ok(Rc::new(b))
}

// -------------------------------------------------- the QD3D* objects --

/// A QD3D* object's property; `None` if it isn't one's.
pub fn get(id: &str, prop: &str) -> Option<Value> {
    store(|s| {
        let r = *s.refs.get(&id.to_lowercase())?;
        match (r.kind, prop) {
            // QD3DVECTOR: X / Y / Z, the same as DVX / DVY / DVZ (a union).
            (Kind::Vector, "x" | "dvx" | "y" | "dvy" | "z" | "dvz") => {
                let v = s.vectors.get(&id.to_lowercase()).copied().unwrap_or_default();
                let i = match prop {
                    "x" | "dvx" => 0,
                    "y" | "dvy" => 1,
                    _ => 2,
                };
                Some(Value::Double(v[i]))
            }
            (Kind::Face, "vertexcount") => Some(v_int(match r.handle.and_then(|h| s.ents.get(h)) {
                Some(Ent::Face(f)) => f.verts.len() as i64,
                _ => 0,
            })),
            (Kind::MeshBuilder | Kind::Mesh, "facecount") => Some(v_int(r.handle.and_then(|h| s.mesh(h)).map_or(0, |m| m.faces.len() as i64))),
            (Kind::MeshBuilder | Kind::Mesh, "vertexcount") => Some(v_int(r.handle.and_then(|h| s.mesh(h)).map_or(0, |m| m.verts.len() as i64))),
            // QD3DMESH's MaxY / MinY (RC.EXE's, read-only): the highest and
            // lowest Y of its vertices, in the mesh's own space; a mesh with
            // none reads empty (RC.EXE prints nothing for a new QD3DMESH).
            (Kind::Mesh, "maxy" | "miny") => {
                let ys: Vec<f64> = r.handle.and_then(|h| s.mesh(h)).map(|m| m.verts.iter().map(|v| v.y).collect()).unwrap_or_default();
                Some(if ys.is_empty() {
                    crate::v_str("")
                } else if prop == "maxy" {
                    Value::Double(ys.iter().copied().fold(f64::MIN, f64::max))
                } else {
                    Value::Double(ys.iter().copied().fold(f64::MAX, f64::min))
                })
            }
            _ => None,
        }
    })
}

/// Sets a QD3D* object's property; `None` if it isn't one's.
pub fn set(id: &str, prop: &str, val: &Value) -> Option<()> {
    store(|s| {
        let r = *s.refs.get(&id.to_lowercase())?;
        if r.kind == Kind::Vector {
            let i = match prop {
                "x" | "dvx" => 0,
                "y" | "dvy" => 1,
                "z" | "dvz" => 2,
                _ => return None,
            };
            s.vectors.entry(id.to_lowercase()).or_default()[i] = val.to_f64();
            return Some(());
        }
        None
    })
}

/// A QD3D* object's method; `None` if it isn't one's. `dir`: where its
/// files are read from (the runtime's current directory).
pub fn call(id: &str, method: &str, args: &[Value]) -> Option<Result<Value, String>> {
    let r = store(|s| s.refs.get(&id.to_lowercase()).copied())?;
    let res = match r.kind {
        Kind::Frame => frame_call(id, r.handle, method, args),
        Kind::MeshBuilder | Kind::Mesh => mesh_call(id, r.handle, method, args),
        Kind::Face => face_call(r.handle, method, args),
        Kind::Light => light_call(r.handle, method, args),
        Kind::Wrap => wrap_call(r.handle, method, args),
        Kind::Texture | Kind::Visual | Kind::Vector | Kind::Animation | Kind::AnimationSet => None,
    }?;
    Some(res.map(|_| Value::Null))
}

fn frame_call(id: &str, h: Option<usize>, method: &str, args: &[Value]) -> Option<Result<(), String>> {
    let Some(h) = h else {
        return Some(Err(format!("{id}.{method}: the frame isn't made yet (DXScreen.CreateFrame)")));
    };
    let other = arg_id(args, 0);
    if method == "load" {
        // A frame's .X file: its meshes in one, on a new mesh builder the
        // frame shows.
        let file = args.first().map(Value::to_string_val).unwrap_or_default();
        return Some(load_x(&file).map(|mesh| {
            store(|s| {
                let m = s.add(Ent::Mesh(mesh));
                if let Some(f) = s.frame(h) {
                    f.visuals.push(m);
                }
            })
        }));
    }
    Some(store(|s| {
        match method {
            "setposition" => {
                if let Some(f) = s.frame(h) {
                    f.m.set_origin(arg_v(args, 0));
                }
            }
            "setorientation" => {
                if let Some(f) = s.frame(h) {
                    let at = f.m.origin();
                    f.m = Mat4::oriented(arg_v(args, 0), arg_v(args, 3), at);
                }
            }
            "setrotation" => {
                let axis = arg_v(args, 0);
                if let Some(f) = s.frame(h) {
                    // (an axis of (0, 0, 0) turns about x, as D3DRM's)
                    f.rot = Some((axis.d3drm_unit(), arg_f(args, 3)));
                }
            }
            "setvelocity" => {
                if let Some(f) = s.frame(h) {
                    f.vel = arg_v(args, 0);
                }
            }
            // AddScale(CombineType, sx, sy, sz): D3DRMCOMBINE_REPLACE 0,
            // BEFORE 1, AFTER 2.
            "addscale" => {
                let m = Mat4::scaling(arg_v(args, 1));
                if let Some(f) = s.frame(h) {
                    f.m = match args.first().map_or(0, Value::to_i64) {
                        0 => {
                            let mut r = m;
                            r.set_origin(f.m.origin());
                            r
                        }
                        1 => m.mul(&f.m),
                        _ => f.m.mul(&m),
                    };
                }
            }
            "addvisual" => {
                if let Some(v) = s.visual(&other) {
                    if let Some(f) = s.frame(h) {
                        if !f.visuals.contains(&v) {
                            f.visuals.push(v);
                        }
                    }
                }
            }
            "deletevisual" => {
                if let Some(v) = s.visual(&other) {
                    if let Some(f) = s.frame(h) {
                        f.visuals.retain(|&x| x != v);
                    }
                }
            }
            "addlight" => {
                if let Some(l) = s.handle(&other, Kind::Light) {
                    if let Some(f) = s.frame(h) {
                        if !f.lights.contains(&l) {
                            f.lights.push(l);
                        }
                    }
                }
            }
            "deletelight" => {
                if let Some(l) = s.handle(&other, Kind::Light) {
                    if let Some(f) = s.frame(h) {
                        f.lights.retain(|&x| x != l);
                    }
                }
            }
            "addframe" => {
                if let Some(c) = s.handle(&other, Kind::Frame) {
                    if c != h {
                        s.reparent(c, Some(h));
                    }
                }
            }
            "deleteframe" => {
                if let Some(c) = s.handle(&other, Kind::Frame) {
                    if s.frame(c).and_then(|f| f.parent) == Some(h) {
                        s.reparent(c, None);
                    }
                }
            }
            "createframe" => {
                let c = s.add(Ent::Frame(Frame::new(Some(h))));
                if let Some(f) = s.frame(h) {
                    f.children.push(c);
                }
                s.point(&other, Kind::Frame, c);
            }
            "lookat" => {
                if let Some(t) = s.handle(&other, Kind::Frame) {
                    look_at(s, h, t);
                }
            }
            "move" => move_frames(s, h, arg_f(args, 0)),
            // SetBackgroundRGB / SetBackgroundImage: the background of the
            // scene the frame is in (D3DRM's SetSceneBackground…).
            "setbackgroundrgb" => {
                let c = arg_v(args, 0);
                if let Some(scene) = s.scene_of(h) {
                    scene.background = [c.x as f32, c.y as f32, c.z as f32];
                }
            }
            "setbackgroundimage" => {
                let image = s.handle(&other, Kind::Texture).and_then(|t| match s.ents.get(t) {
                    Some(Ent::Texture(b)) => Some(b.clone()),
                    _ => None,
                });
                if let Some(scene) = s.scene_of(h) {
                    scene.background_image = image;
                }
            }
            // Fog (the manual: "Don't expect this to work!") and SetTexture
            // ("does not work, use QD3DMeshBuilder.SetTexture"): nothing.
            "setfogparams" | "settexture" | "fogenabled" | "fogmode" | "fogcolor" => {}
            _ => return Err(format!("{id}.{method}: not a QD3DFRAME method RapidR has")),
        }
        Ok(())
    }))
}

fn mesh_call(id: &str, h: Option<usize>, method: &str, args: &[Value]) -> Option<Result<(), String>> {
    let Some(h) = h else {
        return Some(Err(format!("{id}.{method}: the mesh builder isn't made yet (DXScreen.CreateMeshBuilder)")));
    };
    let other = arg_id(args, 0);
    let text = || args.first().map(Value::to_string_val).unwrap_or_default();
    let r = match method {
        "load" => load_x(&text()).map(|loaded| {
            store(|s| {
                if let Some(m) = s.mesh(h) {
                    append_mesh(m, loaded);
                }
            })
        }),
        "loadtexture" => load_texture(&text()).map(|t| {
            store(|s| {
                if let Some(m) = s.mesh(h) {
                    m.set_texture(Some(t));
                }
            })
        }),
        _ => store(|s| {
            match method {
            // AddFace(Face): the face the mesh's (D3DRM's AddFace).
            "addface" => {
                let fh = s.handle(&other, Kind::Face);
                let face = fh.and_then(|f| match s.ents.get(f) {
                    Some(Ent::Face(f)) => Some(f.clone()),
                    _ => None,
                });
                if let (Some(fh), Some(face), Some(m)) = (fh, face, s.mesh(h)) {
                    if !m.faces.iter().any(|mf| mf.face == Some(fh)) {
                        let base = m.verts.len();
                        m.verts.extend(face.verts.iter().copied());
                        if !m.uvs.is_empty() {
                            m.uvs.resize(m.verts.len(), [0.0; 2]);
                        }
                        m.faces.push(MFace { idx: (base..base + face.verts.len()).collect(), nidx: Vec::new(), color: face.color.unwrap_or([1.0; 4]), texture: None, face: Some(fh), spec: None });
                        if let Some(Ent::Face(f)) = s.ents.get_mut(fh) {
                            f.owner = Some(h);
                        }
                    }
                }
            }
            // GetFace(Index, Face): the mesh's face Index (from 0) into Face.
            "getface" => {
                let i = args.first().map_or(-1, Value::to_i64);
                let target = arg_id(args, 1);
                let Some(m) = s.mesh(h) else { return Some(()) };
                let Some(mf) = usize::try_from(i).ok().and_then(|i| m.faces.get(i)).cloned() else { return Some(()) };
                let fh = match mf.face {
                    Some(fh) => fh,
                    None => {
                        let verts = mf.idx.iter().filter_map(|&k| m.verts.get(k).copied()).collect();
                        let fh = s.add(Ent::Face(Face { verts, color: Some(mf.color), owner: Some(h) }));
                        if let Some(mf) = s.mesh(h).and_then(|m| m.faces.get_mut(i as usize)) {
                            mf.face = Some(fh);
                        }
                        fh
                    }
                };
                s.point(&target, Kind::Face, fh);
            }
            // DeleteFace(Face): out of the mesh.
            "deleteface" => {
                if let Some(fh) = s.handle(&other, Kind::Face) {
                    if let Some(m) = s.mesh(h) {
                        m.faces.retain(|mf| mf.face != Some(fh));
                    }
                    if let Some(Ent::Face(f)) = s.ents.get_mut(fh) {
                        f.owner = f.owner.filter(|&o| o != h);
                    }
                }
            }
            "createface" => {
                let f = s.add(Ent::Face(Face::default()));
                s.point(&other, Kind::Face, f);
            }
            "addvertex" => {
                if let Some(m) = s.mesh(h) {
                    m.verts.push(arg_v(args, 0));
                }
            }
            "scale" => {
                let k = arg_v(args, 0);
                if let Some(m) = s.mesh(h) {
                    m.verts.iter_mut().for_each(|v| *v = v3(v.x * k.x, v.y * k.y, v.z * k.z));
                }
            }
            "translate" => {
                let t = arg_v(args, 0);
                if let Some(m) = s.mesh(h) {
                    m.verts.iter_mut().for_each(|v| *v = *v + t);
                }
            }
            "setrgb" | "setcolorrgb" => {
                let c = rgba(arg_f(args, 0), arg_f(args, 1), arg_f(args, 2), 1.0);
                if let Some(m) = s.mesh(h) {
                    m.faces.iter_mut().for_each(|f| f.color = c);
                }
            }
            "setrgba" => {
                let c = rgba(arg_f(args, 0), arg_f(args, 1), arg_f(args, 2), arg_f(args, 3));
                if let Some(m) = s.mesh(h) {
                    m.faces.iter_mut().for_each(|f| f.color = c);
                }
            }
            "setquality" => {
                if let Some(m) = s.mesh(h) {
                    m.quality = args.first().map_or(RENDER_GOURAUD, Value::to_i64);
                }
            }
            "settexture" => {
                let t = s.handle(&other, Kind::Texture).and_then(|t| match s.ents.get(t) {
                    Some(Ent::Texture(b)) => Some(b.clone()),
                    _ => None,
                });
                if let Some(m) = s.mesh(h) {
                    m.set_texture(t);
                }
            }
            // CreateMesh(Mesh): a copy, a visual of its own.
            "createmesh" => {
                let copy = s.mesh(h).cloned();
                if let Some(copy) = copy {
                    let n = s.add(Ent::Mesh(copy));
                    s.point(&other, Kind::Mesh, n);
                }
            }
            _ => return None,
            }
            Some(())
        })
        .ok_or_else(|| format!("{id}.{method}: not a QD3DMESHBUILDER method RapidR has")),
    };
    Some(r)
}

fn face_call(h: Option<usize>, method: &str, args: &[Value]) -> Option<Result<(), String>> {
    let h = h?;
    store(|s| {
        let Some(Ent::Face(f)) = s.ents.get_mut(h) else { return None };
        match method {
            "addvertex" => f.verts.push(arg_v(args, 0)),
            "setcolorrgb" | "setrgb" => f.color = Some(rgba(arg_f(args, 0), arg_f(args, 1), arg_f(args, 2), 1.0)),
            "setcolorrgba" | "setrgba" => f.color = Some(rgba(arg_f(args, 0), arg_f(args, 1), arg_f(args, 2), arg_f(args, 3))),
            // GetVertex(Index, Position, Normal): QD3DVECTORs.
            "getvertex" => {
                let face = f.clone();
                let i = args.first().map_or(-1, Value::to_i64);
                let Some(at) = usize::try_from(i).ok().and_then(|i| face.verts.get(i)).copied() else { return Some(Ok(())) };
                let normal = match face.verts.as_slice() {
                    [a, b, c, ..] => (*b - *a).cross(*c - *a).unit().unwrap_or_default(),
                    _ => Vec3::default(),
                };
                s.vectors.insert(arg_id(args, 1), [at.x, at.y, at.z]);
                s.vectors.insert(arg_id(args, 2), [normal.x, normal.y, normal.z]);
                return Some(Ok(()));
            }
            _ => return None,
        }
        // A face that's a mesh's: the mesh's face too.
        let (owner, face) = (f.owner, f.clone());
        if let Some(m) = owner.and_then(|o| s.mesh(o)) {
            if let Some(i) = m.faces.iter().position(|mf| mf.face == Some(h)) {
                if method == "addvertex" {
                    let k = m.verts.len();
                    m.verts.push(face.verts.last().copied().unwrap_or_default());
                    if !m.uvs.is_empty() {
                        m.uvs.resize(m.verts.len(), [0.0; 2]);
                    }
                    m.faces[i].idx.push(k);
                    m.faces[i].nidx.clear();
                } else {
                    m.faces[i].color = face.color.unwrap_or([1.0; 4]);
                }
            }
        }
        Some(Ok(()))
    })
}

fn light_call(h: Option<usize>, method: &str, args: &[Value]) -> Option<Result<(), String>> {
    let h = h?;
    store(|s| {
        let Some(Ent::Light(l)) = s.ents.get_mut(h) else { return None };
        match method {
            "setlightrgb" => {
                l.kind = args.first().map_or(l.kind, Value::to_i64);
                l.color = arg_v(args, 1);
            }
            "setumbra" => l.umbra = arg_f(args, 0),
            "setpenumbra" => l.penumbra = arg_f(args, 0),
            "setrange" => l.range = arg_f(args, 0),
            _ => return None,
        }
        Some(Ok(()))
    })
}

fn wrap_call(h: Option<usize>, method: &str, args: &[Value]) -> Option<Result<(), String>> {
    let h = h?;
    let target = arg_id(args, if method == "applyrelative" { 1 } else { 0 });
    store(|s| {
        let Some(Ent::Wrap(w)) = s.ents.get(h).cloned() else { return None };
        if !matches!(method, "apply" | "applyrelative") {
            return None;
        }
        if let Some(m) = s.visual(&target).and_then(|v| s.mesh(v)) {
            apply_wrap(&w, m);
        }
        Some(Ok(()))
    })
}

/// Texture coordinates for every vertex of `m` from wrap `w`, as RapidQ's
/// D3DRM computes them (RC.EXE with RapidQ's d3drm.dll in the Windows VM:
/// RapidR's probe scenes captured — docs/directx-plan.md, "D3DRM's wraps").
/// In the wrap's frame (origin, z along `dir`, y along `up`, x to the
/// right):
/// - flat: u = su·x − ou, v = sv·y − ov (v grows upward);
/// - cylinder: the axis is `dir`; u = su·θ/2π − ou with θ the angle around
///   it from `up` (towards x), v = sv·z − ov, the distance along it;
/// - sphere (and chrome): u as the cylinder's, v = sv·φ/π − ov with φ the
///   angle from `dir`.
fn apply_wrap(w: &Wrap, m: &mut Mesh) {
    let frame = Mat4::oriented(w.dir, w.up, w.origin).inverse();
    let (ou, ov) = w.offset;
    let (su, sv) = w.scale;
    m.uvs = m
        .verts
        .iter()
        .map(|v| {
            let p = frame.point(*v);
            let around = || p.x.atan2(p.y).rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU;
            let (u, vv) = match w.kind {
                0 => (su * p.x - ou, sv * p.y - ov),
                1 => (su * around() - ou, sv * p.z - ov),
                _ => {
                    let r = p.len().max(1e-12);
                    let phi = (p.z / r).clamp(-1.0, 1.0).acos();
                    (su * around() - ou, sv * phi / std::f64::consts::PI - ov)
                }
            };
            [u as f32, vv as f32]
        })
        .collect();
}

/// A `.X` file as a mesh builder's mesh: every mesh in it, its frames'
/// matrices applied; its materials' colours and textures a face's (a
/// texture file read from beside the `.X` file or the current directory).
fn load_x(file: &str) -> Result<Mesh, String> {
    let bytes = super::read_file(file)?;
    let x = xfile::parse_x(&bytes)?;
    let flat = xfile::flatten(&x);
    let mut m = Mesh { verts: flat.positions.iter().map(|p| v3(p[0] as f64, p[1] as f64, p[2] as f64)).collect(), ..Mesh::default() };
    m.normals = flat.normals.iter().map(|n| v3(n[0] as f64, n[1] as f64, n[2] as f64)).collect();
    if flat.texcoords.len() == flat.positions.len() {
        m.uvs = flat.texcoords.clone();
    }
    let dir = std::path::Path::new(file).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let textures: Vec<Option<Rc<Bitmap>>> = flat
        .materials
        .iter()
        .map(|mat| {
            let name = mat.texture.as_ref()?;
            load_texture(&dir.join(name).to_string_lossy()).or_else(|_| load_texture(name)).ok()
        })
        .collect();
    for (i, idx) in flat.faces.iter().enumerate() {
        if idx.len() < 3 || idx.iter().any(|&k| k >= m.verts.len()) {
            continue;
        }
        let color = flat.face_materials.get(i).and_then(|&k| flat.materials.get(k)).map_or([1.0; 4], |mat| mat.color);
        let nidx = flat.face_normals.get(i).filter(|n| n.len() == idx.len() && n.iter().all(|&k| k < m.normals.len())).cloned().unwrap_or_default();
        let texture = flat.face_materials.get(i).and_then(|&k| textures.get(k)).cloned().flatten();
        let spec = flat.face_materials.get(i).and_then(|&k| flat.materials.get(k)).filter(|mat| mat.power > 0.0 && mat.specular.iter().any(|c| *c > 0.0)).map(|mat| (mat.power, mat.specular));
        m.faces.push(MFace { idx: idx.clone(), nidx, color, texture, face: None, spec });
    }
    Ok(m)
}

/// `loaded`'s vertices and faces added to `m` (and its texture, if `m` had
/// none).
fn append_mesh(m: &mut Mesh, loaded: Mesh) {
    let (base, nbase) = (m.verts.len(), m.normals.len());
    let had_uvs = !m.uvs.is_empty() || m.verts.is_empty();
    m.verts.extend(loaded.verts.iter().copied());
    m.normals.extend(loaded.normals.iter().copied());
    if had_uvs && !loaded.uvs.is_empty() {
        m.uvs.extend(loaded.uvs.iter().copied());
    } else {
        m.uvs.clear();
    }
    for f in loaded.faces {
        m.faces.push(MFace { idx: f.idx.iter().map(|i| i + base).collect(), nidx: f.nidx.iter().map(|i| i + nbase).collect(), color: f.color, texture: f.texture, face: None, spec: f.spec });
    }
    if m.texture.is_none() {
        m.texture = loaded.texture;
    }
}

// ---------------------------------------------------------- rendering --

/// A light as the scene places it.
struct Placed {
    /// The light's handle.
    h: usize,
    light: Light,
    at: Vec3,
    dir: Vec3,
}

/// The colour `color` takes at `p` with normal `n` (scene space) under
/// `lights` (D3DRM's RGB model; no light: black), with the material's
/// specular highlight `spec` (`(power, colour)`) added for each light that
/// reaches the surface: its colour times the specular colour times
/// (n · h)^power, h halfway between the way to the light and `eye`, the
/// way back to the camera — D3DRM's default, the viewer at infinity
/// (without D3DRMRENDERMODE_VIEWDEPENDENTSPECULAR). RapidQ's
/// `xview/myearth.x` shows it (RC.EXE with RapidQ's d3drm.dll).
fn lit(color: [f32; 4], p: Vec3, n: Vec3, lights: &[Placed], spec: Option<(f32, [f32; 3])>, eye: Vec3) -> [f32; 4] {
    let mut sum = Vec3::default();
    let mut shine = Vec3::default();
    for l in lights {
        let k = match l.light.kind {
            // D3DRMLIGHT_AMBIENT
            0 => 1.0,
            // D3DRMLIGHT_DIRECTIONAL
            3 => {
                let k = n.dot(-l.dir).max(0.0);
                if let (Some((power, sc)), true) = (spec, k > 0.0) {
                    let h = (-l.dir + eye).unit().unwrap_or(n);
                    let s = n.dot(h).max(0.0).powf(power as f64);
                    shine = shine + v3(l.light.color.x * sc[0] as f64, l.light.color.y * sc[1] as f64, l.light.color.z * sc[2] as f64) * s;
                }
                k
            }
            kind => {
                let to = l.at - p;
                let d = to.len();
                if d > l.light.range && l.light.range > 0.0 {
                    0.0
                } else {
                    let ldir = to.unit().unwrap_or(-l.dir);
                    let mut k = n.dot(ldir).max(0.0);
                    if let (Some((power, sc)), true) = (spec, k > 0.0) {
                        let h = (ldir + eye).unit().unwrap_or(n);
                        let s = n.dot(h).max(0.0).powf(power as f64);
                        shine = shine + v3(l.light.color.x * sc[0] as f64, l.light.color.y * sc[1] as f64, l.light.color.z * sc[2] as f64) * s;
                    }
                    // D3DRMLIGHT_SPOT: full within the umbra's cone, none
                    // outside the penumbra's, between in between.
                    if kind == 2 {
                        let angle = (-ldir).dot(l.dir).clamp(-1.0, 1.0).acos();
                        let (inner, outer) = (l.light.umbra.max(0.0), l.light.penumbra.max(l.light.umbra));
                        k *= if angle <= inner {
                            1.0
                        } else if angle >= outer {
                            0.0
                        } else {
                            (outer - angle) / (outer - inner).max(1e-9)
                        };
                    }
                    k
                }
            }
        };
        sum = sum + l.light.color * k;
    }
    [(color[0] as f64 * sum.x + shine.x) as f32, (color[1] as f64 * sum.y + shine.y) as f32, (color[2] as f64 * sum.z + shine.z) as f32, color[3]]
}

/// Draws screen `id`'s scene on its back buffer (`Render`): cleared to the
/// background, then every visual of every frame, from the camera.
pub fn render(id: &str, screen: &mut DxScreen) {
    let scale = super::bitmap::display_scale().max(1);
    let (w, h) = (screen.back.img.width, screen.back.img.height);
    if w == 0 || h == 0 {
        return;
    }
    let (dw, dh) = (w * scale, h * scale);
    let (tris, scene) = store(|s| {
        let scene = s.scene(id);
        (triangles(s, &scene), scene)
    });
    let bg = scene.background;
    let bgc = (bg[2].clamp(0.0, 1.0) * 255.0) as u32 * 0x10000 + ((bg[1].clamp(0.0, 1.0) * 255.0) as u32) * 0x100 + (bg[0].clamp(0.0, 1.0) * 255.0) as u32;
    let mut color = vec![bgc; dw * dh];
    if let Some(img) = scene.background_image.as_deref().filter(|i| i.img.width > 0 && i.img.height > 0) {
        for y in 0..dh {
            for x in 0..dw {
                color[y * dw + x] = img.img.pixels[(y * img.img.height / dh) * img.img.width + x * img.img.width / dw];
            }
        }
    }
    let mut depth = vec![0.0f32; dw * dh];
    let view = raster::View { width: dw, height: dh, front: screen.view_front.max(1e-3), back: screen.view_back.max(screen.view_front + 1e-3), field: 0.5 };
    let mut target = raster::Target { width: dw, height: dh, color: &mut color, depth: &mut depth };
    let blended = scene.render_mode & BLENDED != 0;
    let (mut opaque, mut clear): (Vec<&Tri>, Vec<&Tri>) = tris.iter().partition(|t| !blended || t.c.iter().all(|c| c[3] >= 0.999));
    clear.sort_by(|a, b| {
        let z = |t: &Tri| t.p.iter().map(|p| p.z).sum::<f64>();
        z(b).partial_cmp(&z(a)).unwrap_or(std::cmp::Ordering::Equal)
    });
    opaque.extend(clear);
    for t in opaque {
        raster::draw(&mut target, &view, t);
    }
    // What the screen shows at its scale, and the pixels programs read.
    let back = &mut screen.back;
    if scale > 1 {
        if let Some(hi) = back.display_mut() {
            if hi.img.width == dw && hi.img.height == dh {
                hi.img.pixels.copy_from_slice(&color);
                if let Some(a) = hi.alpha.as_mut() {
                    a.iter_mut().for_each(|v| *v = 255);
                }
            }
        }
        for y in 0..h {
            for x in 0..w {
                back.img.pixels[y * w + x] = color[(y * scale + scale / 2) * dw + x * scale + scale / 2];
            }
        }
    } else {
        back.img.pixels.copy_from_slice(&color);
    }
    back.touch();
}

/// The scene's triangles in the camera's space, lit.
fn triangles(s: &Store, scene: &Scene) -> Vec<Tri> {
    // Every frame under the root, with its matrix to the scene's space.
    let mut frames = Vec::new();
    let mut todo = vec![(scene.root, Mat4::IDENTITY)];
    while let Some((h, parent)) = todo.pop() {
        if frames.len() > 100_000 {
            break;
        }
        let Some(Ent::Frame(f)) = s.ents.get(h) else { continue };
        let world = f.m.mul(&parent);
        frames.push((h, world));
        todo.extend(f.children.iter().map(|&c| (c, world)));
    }
    let lights: Vec<Placed> = frames
        .iter()
        .flat_map(|(h, world)| {
            let Some(Ent::Frame(f)) = s.ents.get(*h) else { return Vec::new() };
            f.lights
                .iter()
                .filter_map(|&l| match s.ents.get(l) {
                    Some(Ent::Light(light)) => Some(Placed { h: l, light: light.clone(), at: world.origin(), dir: world.axis(2).unit().unwrap_or(v3(0.0, 0.0, 1.0)) }),
                    _ => None,
                })
                .collect()
        })
        .collect();
    let camera = frames.iter().find(|(h, _)| *h == scene.camera).map_or(Mat4::IDENTITY, |(_, w)| *w);
    let view = camera.inverse();
    // (the way back to the camera, for specular highlights)
    let eye = (-camera.axis(2)).unit().unwrap_or(v3(0.0, 0.0, -1.0));
    // (D3DRMTEXTURE_LINEAR alone: RC.EXE with RapidQ's d3drm.dll draws the
    // mipmap qualities, LINEARMIP* too, magnified as NEAREST — probes pt_*)
    let linear = scene.texture_quality == 1;
    let mut out = Vec::new();
    for (h, world) in &frames {
        let Some(Ent::Frame(f)) = s.ents.get(*h) else { continue };
        for &v in &f.visuals {
            match s.ents.get(v) {
                Some(Ent::Mesh(m)) => mesh_triangles(m, world, &view, &lights, linear, eye, &mut out),
                Some(Ent::Shadow(sh)) => shadow_triangles(s, sh, world, &view, camera.origin(), &lights, &mut out),
                _ => {}
            }
        }
    }
    out
}

/// A shadow's triangles: its visual's faces (placed as the frame the shadow
/// is in places them) projected from its light onto its plane, black —
/// from a point, spot or parallel point light's position, along a
/// directional light's direction; none from an ambient light, or a light
/// not in the scene. Lifted a little toward the light's side of the plane,
/// to lie on the plane's own faces.
fn shadow_triangles(s: &Store, sh: &Shadow, world: &Mat4, view: &Mat4, eye: Vec3, lights: &[Placed], out: &mut Vec<Tri>) {
    let Some(Ent::Mesh(m)) = s.ents.get(sh.visual) else { return };
    let Some(l) = lights.iter().find(|l| l.h == sh.light && l.light.kind != 0) else { return };
    let Some(n) = sh.normal.unit() else { return };
    let p0 = sh.point;
    let directional = l.light.kind == 3;
    let toward = if directional { -l.dir } else { l.at - p0 };
    let side = if toward.dot(n) >= 0.0 { n } else { -n };
    let project = |p: Vec3| -> Option<Vec3> {
        let (from, d) = if directional { (p, l.dir) } else { (l.at, p - l.at) };
        let den = d.dot(n);
        if den.abs() < 1e-12 {
            return None;
        }
        let t = (p0 - from).dot(n) / den;
        // The plane beyond the point, as the light sees it.
        if (directional && t < -1e-9) || (!directional && t < 1.0 - 1e-9) {
            return None;
        }
        let x = from + d * t;
        Some(x + side * ((x - eye).len() * 1e-3))
    };
    let wp: Vec<Option<Vec3>> = m.verts.iter().map(|&v| project(world.point(v))).collect();
    let black = [0.0, 0.0, 0.0, 1.0];
    for f in &m.faces {
        if f.idx.len() < 3 || f.idx.iter().any(|&i| wp.get(i).copied().flatten().is_none()) {
            continue;
        }
        let cp = |k: usize| view.point(wp[f.idx[k]].unwrap_or_default());
        for j in 1..f.idx.len() - 1 {
            out.push(Tri { p: [cp(0), cp(j), cp(j + 1)], c: [black; 3], uv: None, texture: None, linear: false, fill: Fill::Solid });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn mesh_triangles(m: &Mesh, world: &Mat4, view: &Mat4, lights: &[Placed], linear: bool, eye: Vec3, out: &mut Vec<Tri>) {
    let fill = match m.quality & FILL_MASK {
        FILL_SOLID => Fill::Solid,
        FILL_WIREFRAME => Fill::Wireframe,
        0 => Fill::Points,
        _ => Fill::Solid,
    };
    let light_on = m.quality & LIGHT_ON != 0;
    let gouraud = m.quality & SHADE_MASK != 0;
    let wp: Vec<Vec3> = m.verts.iter().map(|v| world.point(*v)).collect();
    let cp: Vec<Vec3> = wp.iter().map(|p| view.point(*p)).collect();
    // Each face's normal (scene space), and each vertex's (the given ones,
    // else the mean of its faces').
    let face_normal = |f: &MFace| -> Vec3 {
        let (a, b, c) = (wp[f.idx[0]], wp[f.idx[1]], wp[f.idx[2]]);
        (b - a).cross(c - a).unit().unwrap_or(v3(0.0, 0.0, -1.0))
    };
    let mut vnormals = vec![Vec3::default(); wp.len()];
    if gouraud {
        for f in &m.faces {
            if f.idx.len() >= 3 && f.idx.iter().all(|&i| i < wp.len()) {
                let n = face_normal(f);
                f.idx.iter().for_each(|&i| vnormals[i] = vnormals[i] + n);
            }
        }
    }
    let given: Vec<Vec3> = m.normals.iter().map(|n| world.vector(*n).unit().unwrap_or(*n)).collect();
    for f in &m.faces {
        if f.idx.len() < 3 || f.idx.iter().any(|&i| i >= wp.len()) {
            continue;
        }
        let n = face_normal(f);
        // (back faces aren't drawn: wireframes and points are)
        let center = f.idx.iter().fold(Vec3::default(), |a, &i| a + wp[i]) * (1.0 / f.idx.len() as f64);
        let cam_n = view.vector(n);
        if fill == Fill::Solid && cam_n.dot(cp[f.idx[0]]) >= 0.0 {
            continue;
        }
        let flat = if light_on { lit(f.color, center, n, lights, f.spec, eye) } else { f.color };
        let corner = |k: usize| -> [f32; 4] {
            if !light_on || !gouraud {
                return flat;
            }
            let i = f.idx[k];
            let vn = f.nidx.get(k).and_then(|&j| given.get(j)).copied().or_else(|| vnormals[i].unit()).unwrap_or(n);
            lit(f.color, wp[i], vn, lights, f.spec, eye)
        };
        let uv = |k: usize| m.uvs.get(f.idx[k]).copied().unwrap_or([0.0; 2]);
        let texture = if m.uvs.len() == m.verts.len() { f.texture.as_ref().or(m.texture.as_ref()) } else { None };
        for j in 1..f.idx.len() - 1 {
            let ks = [0, j, j + 1];
            out.push(Tri {
                p: ks.map(|k| cp[f.idx[k]]),
                c: ks.map(corner),
                uv: texture.is_some().then(|| ks.map(uv)),
                texture: texture.cloned(),
                linear,
                fill,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v_str;

    fn call_ok(id: &str, method: &str, args: &[Value]) {
        let r = call(id, method, args).unwrap_or_else(|| panic!("{id}.{method}: not handled"));
        r.unwrap_or_else(|e| panic!("{id}.{method}: {e}"));
    }

    fn d(v: f64) -> Value {
        Value::Double(v)
    }

    /// A lit square facing the camera fills the middle of the view in its
    /// colour times the light's; its back isn't drawn; with no light it's
    /// black; Move turns its frame.
    #[test]
    fn a_lit_face_in_the_middle() {
        let mut screen = DxScreen::default();
        screen.call("init", &[v_int(40), v_int(30)]);
        crate::objects::directx::initialize(&mut screen, 40, 30, true);
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
        assert_eq!(get("t_face", "vertexcount").unwrap().to_i64(), 4);
        call_ok("t_mb", "addface", &[v_str("t_face")]);
        call_ok("t_mb", "setquality", &[v_int(LIGHT_ON + FILL_SOLID)]);
        assert_eq!(get("t_mb", "facecount").unwrap().to_i64(), 1);
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
        // Turned half a turn about y by Move: its back faces the camera.
        call_ok("t_f", "setrotation", &[d(0.0), d(1.0), d(0.0), d(std::f64::consts::PI)]);
        sc("move", &[d(1.0)]);
        render("t_dx", &mut screen);
        assert_eq!(screen.back.pixel(20, 15), Some(0), "the back isn't drawn");
    }

    /// The wraps' texture coordinates as RapidQ's D3DRM computed them in
    /// RapidR's probe scenes (RC.EXE in the VM, docs/directx-plan.md).
    #[test]
    fn wraps_as_d3drm() {
        let uv = |kind: i64, offset: (f64, f64), scale: (f64, f64), p: Vec3| {
            let w = Wrap { kind, origin: v3(0.0, 0.0, 0.0), dir: v3(0.0, 0.0, 1.0), up: v3(0.0, 1.0, 0.0), offset, scale };
            let mut m = Mesh { verts: vec![p], ..Mesh::default() };
            apply_wrap(&w, &mut m);
            let [u, v] = m.uvs[0];
            ((u * 1000.0).round() / 1000.0, (v * 1000.0).round() / 1000.0)
        };
        // flat: u along x, v along y — upward
        assert_eq!(uv(0, (0.25, 0.0), (1.0, 1.0), v3(-1.0, 1.0, 3.0)), (-1.25, 1.0));
        assert_eq!(uv(0, (0.0, 0.0), (0.5, 2.0), v3(1.0, -1.0, 3.0)), (0.5, -2.0));
        // cylinder: around the direction from up (towards x), v along it
        assert_eq!(uv(1, (0.0, 0.0), (1.0, 1.0), v3(1.0, 0.0, 3.0)), (0.25, 3.0));
        assert_eq!(uv(1, (0.25, 0.1), (2.0, 0.5), v3(0.0, 1.0, 2.0)), (-0.25, 0.9));
        // sphere: u the same, v the angle from the direction over π
        assert_eq!(uv(2, (0.0, 0.0), (1.0, 1.0), v3(0.0, -1.0, 0.0)), (0.5, 0.5));
    }

    #[test]
    fn a_meshs_highest_and_lowest_y_and_animations() {
        for (id, t) in [("t_my_mb", "RD3DMESHBUILDER"), ("t_my_m", "RD3DMESH"), ("t_my_face", "RD3DFACE"), ("t_my_a", "RD3DANIMATION"), ("t_my_s", "RD3DANIMATIONSET")] {
            assert!(create(id, t), "{t}");
        }
        // (a QD3DMESH with nothing in it reads empty, as RC.EXE prints it)
        assert_eq!(get("t_my_m", "maxy").unwrap().to_string_val(), "");
        let mut screen = DxScreen::default();
        screen_call("t_my_dx", &mut screen, "createmeshbuilder", &[v_str("t_my_mb")]).unwrap().unwrap();
        screen_call("t_my_dx", &mut screen, "createface", &[v_str("t_my_face")]).unwrap().unwrap();
        for (x, y) in [(0.0, -2.0), (1.0, 3.5), (1.0, 0.0)] {
            call_ok("t_my_face", "addvertex", &[d(x), d(y), d(0.0)]);
        }
        call_ok("t_my_mb", "addface", &[v_str("t_my_face")]);
        call_ok("t_my_mb", "createmesh", &[v_str("t_my_m")]);
        assert_eq!((get("t_my_m", "maxy").unwrap().to_f64(), get("t_my_m", "miny").unwrap().to_f64()), (3.5, -2.0));
        // (QD3DMESHBUILDER has no MaxY in RC.EXE; the animations are handles)
        assert!(get("t_my_mb", "maxy").is_none());
        assert!(call("t_my_a", "settime", &[]).is_none() && exists("t_my_s"));
    }

    #[test]
    fn vectors_and_handles() {
        create("t_v", "RD3DVECTOR");
        set("t_v", "dvx", &d(1.5));
        assert_eq!(get("t_v", "x").unwrap().to_f64(), 1.5, "X is DVX (a union)");
        // A variable made again points at a new object; the frame keeps the
        // old one.
        create("t_f2", "RD3DFRAME");
        create("t_l2", "RD3DLIGHT");
        let mut screen = DxScreen::default();
        screen_call("t_dx2", &mut screen, "createframe", &[v_str("t_f2")]).unwrap().unwrap();
        screen_call("t_dx2", &mut screen, "createlightrgb", &[v_int(0), d(0.2), d(0.2), d(0.2), v_str("t_l2")]).unwrap().unwrap();
        call_ok("t_f2", "addlight", &[v_str("t_l2")]);
        let first = store(|s| s.handle("t_l2", Kind::Light));
        screen_call("t_dx2", &mut screen, "createlightrgb", &[v_int(1), d(1.0), d(1.0), d(1.0), v_str("t_l2")]).unwrap().unwrap();
        let second = store(|s| s.handle("t_l2", Kind::Light));
        assert_ne!(first, second);
        let kept = store(|s| {
            let f = s.handle("t_f2", Kind::Frame).unwrap();
            s.frame(f).unwrap().lights.clone()
        });
        assert_eq!(kept, vec![first.unwrap()]);
        assert!(call("t_f9", "setposition", &[]).is_none(), "not a QD3D object");
    }

    #[test]
    fn shadows_and_the_cameras_velocity() {
        let mut screen = DxScreen::default();
        screen.call("init", &[v_int(40), v_int(30)]);
        crate::objects::directx::initialize(&mut screen, 40, 30, true);
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
        // QDXSCREEN.SetVelocity: the camera's; Move moves it.
        sc("setvelocity", &[d(0.0), d(0.0), d(1.0), v_int(0)]);
        sc("move", &[d(2.0)]);
        let at = store(|s| {
            let c = s.scene("t_s_dx").camera;
            s.world(c).origin()
        });
        assert_eq!((at.x, at.y, at.z), (0.0, 0.0, 2.0));
    }
}
