// RapidQ's Direct3D on wgpu (docs/directx-plan.md, stage D5).
//
// The scene arrives lit (rapidr-value's objects::d3d: D3DRM's RGB model,
// per face or per vertex) in the camera's space; here it's projected through
// D3DRM's viewport and filled: colours and texture coordinates interpolated
// perspective-correctly, the texture modulated by the colour, a depth of 1/z.

struct View {
    // x and y scales (pixels per unit at z = 1, over the target's half sides)
    kx: f32,
    ky: f32,
    // the depth's a·z + b: 1 at the front plane, 0 at the back one (the
    // depth test keeps the larger — the nearer — as 1/z does)
    a: f32,
    b: f32,
    // (the background picture's pass) the target's and the picture's sizes
    dw: u32,
    dh: u32,
    iw: u32,
    ih: u32,
};

@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(@location(0) p: vec3<f32>, @location(1) c: vec4<f32>, @location(2) uv: vec2<f32>) -> Out {
    var o: Out;
    o.pos = vec4<f32>(p.x * view.kx, p.y * view.ky, view.a * p.z + view.b, p.z);
    o.color = c;
    o.uv = uv;
    return o;
}

@fragment
fn fs_main(i: Out) -> @location(0) vec4<f32> {
    let t = textureSample(tex, samp, i.uv);
    return vec4<f32>(i.color.rgb * t.rgb, i.color.a);
}

// SetBackgroundImage's picture over the whole view, a texel a pixel as the
// view's pixel lands in it (nearest).
@vertex
fn vs_back(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

@fragment
fn fs_back(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    let x = min(u32(p.x), view.dw - 1u);
    let y = min(u32(p.y), view.dh - 1u);
    let t = textureLoad(tex, vec2<i32>(i32(x * view.iw / view.dw), i32(y * view.ih / view.dh)), 0);
    return vec4<f32>(t.rgb, 1.0);
}
