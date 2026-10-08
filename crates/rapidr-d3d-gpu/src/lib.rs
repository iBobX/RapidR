//! RapidQ's Direct3D on wgpu (docs/directx-plan.md, stage D5).
//!
//! A QDXSCREEN's `Render` hands over the scene as Direct3D Retained Mode
//! lit it (`rapidr_value::objects::d3d`: triangles in the camera's space,
//! their colours, texture coordinates and textures, in drawing order —
//! the same on every runtime). [`Gpu`] rasterizes it on wgpu — Metal on
//! macOS, Direct3D 12 on Windows (WARP where there's no GPU), Vulkan or
//! OpenGL on Linux, WebGL 2 in a browser — into a texture the size of the
//! screen's back buffer at its display scale, and reads the pixels back:
//! 2D drawing after `Render` (the FPS text), `Pixel` and `Flip` keep
//! working on the back buffer as RapidQ's did.
//!
//! - **The projection** is D3DRM's viewport: a square field of half-side
//!   0.5 at the front plane, over the target's larger side; a depth of
//!   1/z, 1 at the front plane and 0 at the back one, the larger (nearer)
//!   winning ties to what's drawn later.
//! - **Colours** past 1 (a light brighter than white) are interpolated as
//!   they are, times the texel, then held to 1 by the target's format.
//! - **Blended** triangles (an alpha under 1) are drawn over what's
//!   behind them without hiding what comes later; points and wireframes
//!   are a pixel wide.
//! - **Textures** repeat, nearest or bilinear (D3DRMTEXTURE_LINEAR), and
//!   stay on the GPU while their picture lives.
//! - **In a browser**, WebGL 2: its read-back is synchronous, as a Render's
//!   must be (the program reads the pixels right after it, and a native web
//!   build can't wait for a promise). WebGPU's isn't.

use std::collections::HashMap;
use std::rc::{Rc, Weak};

use rapidr_value::objects::bitmap::Bitmap;
use rapidr_value::objects::d3d::renderer::{set_renderer_factory, Fill, RenderList, Renderer, Tri};

const SHADER: &str = include_str!("d3d.wgsl");
const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// A vertex: position (3), colour (4), texture coordinates (2).
const VERTEX_FLOATS: usize = 9;

/// Gives QDXSCREEN's `Render` this renderer (made at the first Render).
pub fn install() {
    set_renderer_factory(|| Gpu::new().map(|g| Box::new(g) as Box<dyn Renderer>));
}

/// A texture on the GPU, while its picture lives.
struct GpuTexture {
    picture: Weak<Bitmap>,
    nearest: wgpu::BindGroup,
    linear: wgpu::BindGroup,
}

/// The target a size has: colour, depth, the read-back buffer.
struct Target {
    width: u32,
    height: u32,
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    readback: wgpu::Buffer,
    /// Bytes per row of the read-back (wgpu's 256-byte alignment).
    row: u32,
}

/// The pipelines: per fill mode, opaque or blended; the background's.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Pass {
    Solid,
    Blended,
    Wireframe,
    Points,
}

/// Direct3D on wgpu: the device, its pipelines and what it keeps between
/// frames.
pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// The adapter, for messages ("Apple M2 (Metal)").
    pub adapter: String,
    tex_layout: wgpu::BindGroupLayout,
    pipelines: HashMap<Pass, wgpu::RenderPipeline>,
    background: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    view_group: wgpu::BindGroup,
    samplers: [wgpu::Sampler; 2],
    white: wgpu::BindGroup,
    textures: HashMap<usize, GpuTexture>,
    target: Option<Target>,
    vertices: Option<wgpu::Buffer>,
    max_side: u32,
    /// (the browser's WebGL context lives on a canvas of its own, never
    /// shown: kept while the device is)
    _keep: Option<Box<dyn std::any::Any>>,
}

/// A future wgpu-core has already finished (adapters and devices on every
/// backend but the browser's WebGPU, which this crate doesn't use).
fn now<F: std::future::Future>(f: F) -> Result<F::Output, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(pollster::block_on(f))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut f = std::pin::pin!(f);
        match f.as_mut().poll(&mut std::task::Context::from_waker(std::task::Waker::noop())) {
            std::task::Poll::Ready(v) => Ok(v),
            std::task::Poll::Pending => Err("the GPU didn't answer at once".into()),
        }
    }
}

impl Gpu {
    /// The GPU: the best adapter there is (high performance), else the
    /// system's software one (WARP, llvmpipe).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Result<Gpu, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let mut adapter = None;
        for fallback in [false, true] {
            let opts = wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, force_fallback_adapter: fallback, compatible_surface: None, apply_limit_buckets: false };
            if let Ok(Ok(a)) = now(instance.request_adapter(&opts)) {
                adapter = Some(a);
                break;
            }
        }
        let adapter = adapter.ok_or("wgpu found no GPU (Metal, Vulkan, Direct3D 12 or OpenGL)")?;
        Gpu::with_adapter(adapter)
    }

    /// The browser's WebGL 2, on a canvas of its own.
    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Result<Gpu, String> {
        use wasm_bindgen::JsCast;
        let canvas: web_sys::HtmlCanvasElement = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.create_element("canvas").ok())
            .and_then(|e| e.dyn_into().ok())
            .ok_or("no canvas for WebGL")?;
        canvas.set_width(1);
        canvas.set_height(1);
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::GL;
        // (WebGL's fences never signal before the page's turn ends; its
        // read-back — getBufferSubData — is synchronous anyway)
        desc.backend_options.gl.fence_behavior = wgpu::GlFenceBehavior::AutoFinish;
        let instance = wgpu::Instance::new(desc);
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone())).map_err(|e| format!("no WebGL 2 ({e})"))?;
        let opts = wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, force_fallback_adapter: false, compatible_surface: Some(&surface), apply_limit_buckets: false };
        let adapter = now(instance.request_adapter(&opts))?.map_err(|e| format!("no WebGL 2 ({e})"))?;
        let mut gpu = Gpu::with_adapter(adapter)?;
        gpu._keep = Some(Box::new((canvas, surface, instance)));
        Ok(gpu)
    }

    fn with_adapter(adapter: wgpu::Adapter) -> Result<Gpu, String> {
        let info = adapter.get_info();
        let name = format!("{} ({:?})", info.name, info.backend);
        let limits = adapter.limits();
        let (device, queue) = now(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("rapidr d3d"),
            required_features: wgpu::Features::empty(),
            required_limits: limits.clone(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))?
        .map_err(|e| format!("{name}: {e}"))?;
        // (an error the program can't cause is said, not a panic)
        device.on_uncaptured_error(std::sync::Arc::new(|e: wgpu::Error| eprintln!("[rapidr] Direct3D (wgpu): {e}")));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("d3d"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("d3d view"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("d3d texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("d3d"), bind_group_layouts: &[Some(&view_layout), Some(&tex_layout)], immediate_size: 0 });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor { label: Some("d3d view"), size: 32, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let view_group = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("d3d view"), layout: &view_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() }] });
        let sampler = |filter: wgpu::FilterMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("d3d"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        let samplers = [sampler(wgpu::FilterMode::Nearest), sampler(wgpu::FilterMode::Linear)];
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4, 2 => Float32x2];
        let vertex_layout = wgpu::VertexBufferLayout { array_stride: (VERTEX_FLOATS * 4) as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &attributes };
        let pipeline = |pass: Pass| {
            let topology = match pass {
                Pass::Solid | Pass::Blended => wgpu::PrimitiveTopology::TriangleList,
                Pass::Wireframe => wgpu::PrimitiveTopology::LineList,
                Pass::Points => wgpu::PrimitiveTopology::PointList,
            };
            let blend = (pass == Pass::Blended).then_some(wgpu::BlendState::ALPHA_BLENDING);
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("d3d"),
                layout: Some(&layout),
                vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[Some(vertex_layout.clone())] },
                primitive: wgpu::PrimitiveState { topology, cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: Some(pass != Pass::Blended),
                    depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: COLOR, blend, write_mask: wgpu::ColorWrites::ALL })] }),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipelines = [Pass::Solid, Pass::Blended, Pass::Wireframe, Pass::Points].into_iter().map(|p| (p, pipeline(p))).collect();
        let background = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("d3d background"),
            layout: Some(&layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_back"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(false), depth_compare: Some(wgpu::CompareFunction::Always), stencil: Default::default(), bias: Default::default() }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_back"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: COLOR, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            multiview_mask: None,
            cache: None,
        });
        let max_side = limits.max_texture_dimension_2d.min(16384);
        Ok(Gpu {
            white: tex_layout_group(&device, &queue, &tex_layout, &samplers[0], &white_picture()),
            device,
            queue,
            adapter: name,
            tex_layout,
            pipelines,
            background,
            uniforms,
            view_group,
            samplers,
            textures: HashMap::new(),
            target: None,
            vertices: None,
            max_side,
            _keep: None,
        })
    }

    /// The target for a size (kept while the size is).
    fn target(&mut self, width: u32, height: u32) -> &Target {
        if self.target.as_ref().is_none_or(|t| t.width != width || t.height != height) {
            let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
            let texture = |format, usage| {
                self.device.create_texture(&wgpu::TextureDescriptor { label: Some("d3d target"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format, usage, view_formats: &[] })
            };
            let color = texture(COLOR, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC);
            let depth = texture(DEPTH, wgpu::TextureUsages::RENDER_ATTACHMENT);
            let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let readback = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("d3d read-back"), size: u64::from(row) * u64::from(height), usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            self.target = Some(Target {
                width,
                height,
                color_view: color.create_view(&Default::default()),
                depth_view: depth.create_view(&Default::default()),
                color,
                readback,
                row,
            });
        }
        self.target.as_ref().expect("made above")
    }

    /// The bind group of `picture` (uploaded the first time it's seen).
    fn texture(&mut self, picture: &Rc<Bitmap>, linear: bool) -> &wgpu::BindGroup {
        let key = Rc::as_ptr(picture) as usize;
        let alive = self.textures.get(&key).and_then(|t| t.picture.upgrade()).is_some_and(|p| Rc::ptr_eq(&p, picture));
        if !alive {
            let nearest = tex_layout_group(&self.device, &self.queue, &self.tex_layout, &self.samplers[0], picture);
            let linear = tex_layout_group(&self.device, &self.queue, &self.tex_layout, &self.samplers[1], picture);
            self.textures.insert(key, GpuTexture { picture: Rc::downgrade(picture), nearest, linear });
        }
        let t = &self.textures[&key];
        if linear {
            &t.linear
        } else {
            &t.nearest
        }
    }
}

/// A 1 × 1 white picture: what an untextured triangle's colour is
/// multiplied by.
fn white_picture() -> Bitmap {
    let mut b = Bitmap::default();
    b.resize(1, 1);
    b.img.pixels = vec![0xFF_FFFF];
    b
}

/// `picture` (RapidQ's &HBBGGRR) as a texture, with `sampler`.
fn tex_layout_group(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, sampler: &wgpu::Sampler, picture: &Bitmap) -> wgpu::BindGroup {
    let (w, h) = (picture.img.width.max(1) as u32, picture.img.height.max(1) as u32);
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    if picture.img.pixels.len() >= (w * h) as usize {
        for &p in &picture.img.pixels[..(w * h) as usize] {
            rgba.extend_from_slice(&[(p & 0xFF) as u8, (p >> 8 & 0xFF) as u8, (p >> 16 & 0xFF) as u8, 255]);
        }
    } else {
        rgba.resize(w as usize * h as usize * 4, 255);
    }
    let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("d3d texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        &rgba,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
        size,
    );
    let view = texture.create_view(&Default::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("d3d texture"),
        layout,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) }],
    })
}

/// A run of triangles drawn with one pipeline and texture.
struct Batch {
    pass: Pass,
    texture: Option<(Rc<Bitmap>, bool)>,
    start: u32,
    end: u32,
}

/// The vertices of `tris` (a triangle's three corners; a wireframe's three
/// edges), in runs.
fn vertices(tris: &[Tri]) -> (Vec<f32>, Vec<Batch>) {
    let mut v: Vec<f32> = Vec::with_capacity(tris.len() * 3 * VERTEX_FLOATS);
    let mut batches: Vec<Batch> = Vec::new();
    for t in tris {
        let pass = match t.fill {
            Fill::Solid if t.blended() => Pass::Blended,
            Fill::Solid => Pass::Solid,
            Fill::Wireframe => Pass::Wireframe,
            Fill::Points => Pass::Points,
        };
        let texture = match (&t.texture, t.uv) {
            (Some(tex), Some(_)) => Some((tex.clone(), t.linear)),
            _ => None,
        };
        let uv = t.uv.unwrap_or([[0.0; 2]; 3]);
        let corners: &[usize] = if pass == Pass::Wireframe { &[0, 1, 1, 2, 2, 0] } else { &[0, 1, 2] };
        let start = (v.len() / VERTEX_FLOATS) as u32;
        for &k in corners {
            let p = t.p[k];
            v.extend_from_slice(&[p.x as f32, p.y as f32, p.z as f32]);
            v.extend_from_slice(&t.c[k]);
            v.extend_from_slice(&uv[k]);
        }
        let end = (v.len() / VERTEX_FLOATS) as u32;
        let same = batches.last().is_some_and(|b| {
            b.pass == pass
                && match (&b.texture, &texture) {
                    (None, None) => true,
                    (Some((a, la)), Some((b, lb))) => Rc::ptr_eq(a, b) && la == lb,
                    _ => false,
                }
        });
        if same {
            batches.last_mut().expect("checked").end = end;
        } else {
            batches.push(Batch { pass, texture, start, end });
        }
    }
    (v, batches)
}

impl Renderer for Gpu {
    fn render(&mut self, list: &RenderList) -> Result<Vec<u32>, String> {
        let (w, h) = (list.view.width as u32, list.view.height as u32);
        if w == 0 || h == 0 {
            return Ok(Vec::new());
        }
        if w > self.max_side || h > self.max_side {
            return Err(format!("{w} × {h} is larger than {} takes", self.adapter));
        }
        // The view: D3DRM's projection and the depth's 1/z.
        let v = &list.view;
        let k = v.scale();
        let (front, back) = (v.front, v.back.max(v.front + 1e-6));
        let a = -front / (back - front);
        let b = front * back / (back - front);
        let picture = list.background_image.clone().filter(|i| i.img.width > 0 && i.img.height > 0);
        let (iw, ih) = picture.as_ref().map_or((1, 1), |i| (i.img.width as u32, i.img.height as u32));
        let mut u = Vec::with_capacity(32);
        for f in [2.0 * k / f64::from(w), 2.0 * k / f64::from(h), a, b] {
            u.extend_from_slice(&(f as f32).to_le_bytes());
        }
        for n in [w, h, iw, ih] {
            u.extend_from_slice(&n.to_le_bytes());
        }
        self.queue.write_buffer(&self.uniforms, 0, &u);
        // The triangles.
        let (data, batches) = vertices(&list.tris);
        let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        if !bytes.is_empty() {
            let need = (bytes.len() as u64).next_power_of_two().max(4096);
            if self.vertices.as_ref().is_none_or(|b| b.size() < bytes.len() as u64) {
                self.vertices = Some(self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("d3d vertices"), size: need, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }));
            }
            self.queue.write_buffer(self.vertices.as_ref().expect("made above"), 0, &bytes);
        }
        let groups: Vec<Option<wgpu::BindGroup>> = batches.iter().map(|b| b.texture.as_ref().map(|(t, linear)| self.texture(t, *linear).clone())).collect();
        let back_group = picture.as_ref().map(|p| self.texture(p, false).clone());
        self.target(w, h);
        let target = self.target.as_ref().expect("made above");
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("d3d") });
        {
            let bg = list.background;
            let ch = |s: u32| f64::from(bg >> s & 0xFF) / 255.0;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("d3d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: ch(0), g: ch(8), b: ch(16), a: 1.0 }), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &target.depth_view, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Discard }), stencil_ops: None }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.view_group, &[]);
            if let Some(group) = &back_group {
                pass.set_pipeline(&self.background);
                pass.set_bind_group(1, group, &[]);
                pass.draw(0..3, 0..1);
            }
            if let Some(vb) = &self.vertices {
                pass.set_vertex_buffer(0, vb.slice(..));
                for (batch, group) in batches.iter().zip(&groups) {
                    pass.set_pipeline(&self.pipelines[&batch.pass]);
                    pass.set_bind_group(1, group.as_ref().unwrap_or(&self.white), &[]);
                    pass.draw(batch.start..batch.end, 0..1);
                }
            }
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &target.color, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &target.readback, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(target.row), rows_per_image: Some(h) } },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        // The pixels back (the GPU done first).
        let slice = target.readback.slice(..);
        let done = std::sync::Arc::new(std::sync::Mutex::new(None::<bool>));
        let mapped = done.clone();
        slice.map_async(wgpu::MapMode::Read, move |r| *mapped.lock().unwrap_or_else(|e| e.into_inner()) = Some(r.is_ok()));
        self.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| format!("{}: {e}", self.adapter))?;
        if *done.lock().unwrap_or_else(|e| e.into_inner()) != Some(true) {
            return Err(format!("{}: the frame couldn't be read back", self.adapter));
        }
        let mut out = vec![0u32; (w * h) as usize];
        {
            let view = slice.get_mapped_range().map_err(|e| format!("{}: {e}", self.adapter))?;
            for y in 0..h as usize {
                let row = &view[y * target.row as usize..y * target.row as usize + w as usize * 4];
                for (x, px) in row.as_chunks::<4>().0.iter().enumerate() {
                    out[y * w as usize + x] = u32::from(px[0]) | u32::from(px[1]) << 8 | u32::from(px[2]) << 16;
                }
            }
        }
        target.readback.unmap();
        // (pictures no longer in any scene leave the GPU)
        self.textures.retain(|_, t| t.picture.strong_count() > 0);
        Ok(out)
    }

    fn max_side(&self) -> usize {
        self.max_side as usize
    }
}
