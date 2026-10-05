# RapidR DirectX: RapidQ's QDX* / QD3D* objects on the UI kernel and wgpu

RapidQ's DirectX objects are the next compatibility block after the portable API (ROADMAP Phase 1: "DirectX objects on wgpu (desktop and web)"). This plan lists every DirectX object RapidQ has, what the RapidQ corpus actually uses, and how each maps onto RapidR — without Win32 emulation, with permissive dependencies only, the same on native builds, the interpreter and the web. The first slice (the 2D layer: QDXSCREEN, QDXIMAGELIST, QDXTIMER) is done; its results are at the end.

## 0. Key findings that shape the plan

- **RapidQ built its DirectX objects on DelphiX** (John Kelly's D3D notes: "It appears that RapidQ used an older package called DelphiX"): QDXSCREEN is TDXDraw, QDXIMAGELIST is TDXImageList, QDXTIMER is TDXTimer, QDXSOUND is TDXSound / TDXWave, and the QD3D* objects wrap Direct3D **Retained Mode** (D3DRM, `IDirect3DRMFrame`, `…MeshBuilder`, `…Light` …). DelphiX's semantics answer most of what the manual leaves open (`.DXG` image libraries, pattern indexes, AutoSize, the timer's frame rate).
- **The 2D layer is a CPU surface.** Everything a program does to a QDXSCREEN is "draw on the off-screen page, then Flip" (manual, chapter 12.5), the same drawing calls as QCANVAS / QBITMAP. RapidR already has those on a shared CPU bitmap with a high-DPI layer (`rapidr_value::objects::bitmap`) shown by the kernel as `Op::Image` (desktop GPU, vello_cpu captures, the headless host, the web's canvas). A screen is two of them (back buffer, front) — no GPU path is needed or wanted for 2D: pixels are exact everywhere and `Pixel` reads stay synchronous.
- **The 3D layer is small, old and retained.** The corpus's 3D programs build scenes of a few hundred to a few thousand triangles (faces by hand, `.X` models, BMP textures, a few lights) rendered at 320 × 240 – 640 × 480, then draw 2D text on the same surface and Flip (`DXScreen.Render: DXScreen.TextOut(…"FPS"…): DXScreen.Flip`). So the 3D renderer writes into the QDXSCREEN back buffer, and 2D drawing, `Pixel` and Flip keep one model.
- **There is no joystick object.** RapidQ's manual says it "does not yet support … joystick" (chapter 12.5); the one corpus program (`examples/devices/joystick.bas`) DECLAREs winmm's `joyGetPosEx`, which RapidR doesn't emulate (policy). QDXSOUND exists (manual Appendix B; not in `KEYWORD.LST`). `KEYWORD.LST` also lists QD3DANIMATION and QD3DANIMATIONSET, which have no documentation and no corpus use.
- **Some "QD3D" names are RapidQ code**, not built-ins: QD3DCAMERA, QD3DPRIMITIVE, QD3DCLONEMESH, QD3DORIENTVECTOR and QD3DRGBA are TYPEs in `include/RapidQ_D3D.inc` (John Kelly's library) built on the built-in objects. They run once the built-ins do.
- **The `.X` files are text and binary, never compressed**: of the 55 `.x` files in the RapidQ install, 38 are `xof 0302txt` / `0303txt`, 17 `xof 0302bin` / `0303bin`, none `tzip` / `bzip`. They use `Header`, `Frame`, `FrameTransformMatrix`, `Mesh`, `MeshNormals`, `MeshTextureCoords`, `MeshMaterialList`, `Material`, `TextureFilename` (BMP, one JPEG); no `AnimationSet`. `.3DS` files were converted offline (CONV3DS.EXE, chapter 13.2); RapidQ never loaded them.

---

## 1. RapidQ's DirectX objects, and what the corpus uses

The manual is `.reference/txt/rapidq/qdx*.txt`, `qd3d*.txt`, `D3D_Details.txt`, `D3DRMConst.txt`, chapter 12.5 (DirectX) and chapter 13 (Direct3D). Corpus counts are uses (`Obj.Member` and bare members inside a CREATE) and programs, over the 38 files of `~/Downloads/Rapidq` that mention a QDX* / QD3D* name (a rough count: comments excluded, `WITH` blocks not resolved).

### 1.1 QDXSCREEN — 28 programs

| Kind | Members (manual) | Corpus use (uses / programs) |
|---|---|---|
| Properties | Align, AllowStretch (True), AutoSize (True), BitCount (8), Color, Cursor, Enabled, Font, FullScreen, Height, Hint, Left, Parent, Pixel(x, y), ShowHint, Top, Use3D, UseHardware (True), Visible, Width | Align 18/18, Use3D 16/16, UseHardware 16/16, BitCount 15/15, Width 11/9, Height 10/9, Pixel 7/3, Cursor 6/4, FullScreen 5/5, Font 4/3, Parent 3/3, Hint 3/3, Top 5/4, Left 2/2, AllowStretch 1/1 |
| 2D methods | Circle, CopyRect, Draw, FastPset, Fill, FillRect, Flip, Init, Line, Paint, Pset, Rectangle, Release, Rotate, RoundRect, StretchDraw, TextHeight, TextWidth, TextRect, TextOut | Flip 42/25, TextOut 37/21, Init 18/18, Draw 14/2, Line 11/1, Fill 5/5, Circle 3/2, Rectangle 2/1, FillRect 2/1, TextWidth 1/1, TextHeight 1/1, CopyRect 1/1, StretchDraw 1/1, Pset 1/1 |
| 3D methods | AddLight, CameraLookAt, CreateFace, CreateFrame, CreateLightRGB, CreateMeshBuilder, CreateShadow, CreateWrap, ForceUpdate, LoadTexture, Move, SetBackgroundImage, SetCameraPosition, SetCameraOrientation, SetRenderMode, SetVelocity (undocumented), SetTextureQuality; View.Clear / GetFront / SetFront / GetBack / SetBack / SetPlane (undocumented) | CreateFace 53/6, CreateFrame 53/16, CreateMeshBuilder 48/17, CreateLightRGB 29/15, SetCameraPosition 25/15, SetTextureQuality 25/5, CreateWrap 25/9, AddLight 18/15, **Render 17/15 (not in the manual)**, SetRenderMode 15/15, SetCameraOrientation 13/8, ForceUpdate 12/12, LoadTexture 10/6, CameraLookAt 8/7, Move 6/6, SetBackgroundImage 4/4, CreateShadow 2/2, View.SetPlane 2/2, View.Clear 2/1, View.GetFront 1/1 |
| Events | OnClick, OnDblClick, OnInitialize, OnInitializeSurface, OnKeyDown, OnKeyPress, OnMouseDown (Button, X, Y), OnMouseMove (X, Y), OnMouseUp | OnInitialize 16/16, OnInitializeSurface 16/16, OnMouseMove 7/7, OnMouseDown 7/7, OnClick 2/2, OnKeyDown 2/2, OnKeyPress 2/2, OnMouseUp 1/1 |

The 2D-only programs: `directx/dxball.bas`, `directx/starfield/stars.bas`, `directx/scroll/dx.bas`, `directx/shooter/shooter.bas`, `Mouse/select2dx.bas`, `graphics/image_test.bas`, `imageLibrary/mgplist.bas` (`forms/newform/newform.bas` only DIMs one). The other 20 are 3D.

### 1.2 QDXIMAGELIST — 3 programs

Parent (a QDXSCREEN); Draw(Item, X, Y, Mask), LoadFromFile, LoadFromResource, LoadFromStream; no events. Corpus: Draw 8/3, Parent 3/2, LoadFromFile 3/2 (the manual's own example uses `$RESOURCE` + LoadFromResource). The files are DelphiX image libraries (`.DXG`: a TPictureCollection streamed as a Delphi form, each picture an 8-bit TDIB with Name, PatternWidth / PatternHeight, SkipWidth / SkipHeight, Transparent, TransparentColor).

### 1.3 QDXTIMER — 24 programs

ActiveOnly (True), Enabled (False), FrameRate (read only), Interval; OnTimer; "ONLY one QDXTIMER can be used". Corpus: Enabled 25/23, Interval 23/23, OnTimer 23/23, FrameRate 18/18, ActiveOnly 8/8. Interval 0 ("let DirectX handle FPS") in most.

### 1.4 QDXSOUND — 3 programs

AutoUpdate, BufferLength, FileName (a .WAV), Frequency, Looped, Pan (−100 … 100), Parent, Playing, Position, Size, StickyFocus, Tag, Volume (0 … 100); Play, Stop, Update. Corpus (`sound/QDXSound.bas`, `sound/qdxsound2.bas`, `music/WaveRec&Play.rqb`): Frequency 13/2, Stop 12/2, Position 10/2, Play 9/3, Volume 8/2, FileName 6/3, Looped 5/1, Playing 4/2, Pan 1/1.

### 1.5 The Direct3D objects

| Object | Manual | Corpus (uses / programs) |
|---|---|---|
| QD3DFRAME (16 programs) | FogColor, FogEnabled, FogMode, Parent; AddFrame, AddLight, AddScale, AddVisual, CreateFrame, DeleteFrame, DeleteLight, DeleteVisual, Load (.X), LookAt, Move, SetBackgroundImage, SetBackgroundRGB, SetFogParams, SetOrientation, SetPosition, SetRotation, SetTexture | SetPosition 60/16, AddVisual 58/16, DeleteVisual 48/9, SetRotation 47/11, AddLight 15/14, Move 14/6, SetOrientation 8/4, AddFrame 4/2, DeleteFrame 4/2, FogEnabled 4/2, FogMode 2/2, FogColor 2/2, SetFogParams 2/2, SetBackgroundRGB 2/2 |
| QD3DMESHBUILDER (15) | FaceCount; AddFace, AddVertex, CreateFace, CreateMesh, DeleteFace, GetFace, Load (.X), LoadTexture, Scale, SetQuality, SetRGB, SetRGBA, SetTexture, Translate | AddFace 53/5, SetQuality 36/7, Scale 26/8, Load 20/11, SetRGBA 20/5, LoadTexture 16/8, SetTexture 4/1, Translate 4/1, SetRGB 1/1 |
| QD3DLIGHT (16) | SetLightRGB, SetPenUmbra, SetRange, SetUmbra (lights come from QDXSCREEN.CreateLightRGB) | SetPenUmbra 1, SetUmbra 1, SetRange 1 |
| QD3DFACE (7) | VertexCount; AddVertex, GetVertex, SetColorRGB | AddVertex 278/6, SetColorRGB 20/3 |
| QD3DWRAP (11) | Apply, ApplyRelative (made by CreateWrap: flat, cylinder, sphere, chrome) | Apply 26/9, ApplyRelative 3/2 |
| QD3DVECTOR (6) | DVX / DVY / DVZ, X / Y / Z (a union) | X / Y / Z about 420 uses in 5 |
| QD3DTEXTURE, QD3DVISUAL, QD3DMESH | handles only (QD3DMESH "has no real purpose") | declared and passed |
| QD3DANIMATION, QD3DANIMATIONSET | `KEYWORD.LST` only | none |
| QD3DCAMERA, QD3DPRIMITIVE, QD3DCLONEMESH, QD3DORIENTVECTOR, QD3DRGBA | TYPEs of `RapidQ_D3D.inc` | 3, 4, 3, 1, 1 programs |

Files loaded by the 3D programs: `.x` 15 uses, `.bmp` textures 8.

---

## 2. How each maps onto RapidR

### 2.1 The 2D layer (done: "Stage D1 results" below)

- **QDXSCREEN is a shared model** (`rapidr_value::objects::directx::DxScreen`): a back buffer and a front buffer, both `Bitmap`s, so every drawing call, the text, the high-DPI layer and the drawing revision are QCANVAS's. `Flip` copies the back buffer to the front (it keeps its pixels, as a windowed DirectDraw blit does). The kernel component (`rapidr_ui_kernel::components::dxscreen`, one line in `KINDS`) shows the front as an `Op::Image` named by the screen's id and its revision, converted again only after a Flip; the host caches it by `(id, revision)` as it does canvases. The web's DOM runtime puts the same front on a `<canvas>` (`gui_web::render_dxscreen`); the kernel web host (docs/web-host-plan.md) will show it like any other `Op::Image`.
- **Set-up** as DirectX had it: when the form is first shown (runtime-core `build_form` → `directx::form_built`; the web's `take_onshow` / ShowModal / `gui_web_finalize` → `directx_web::form_shown`), the surface is sized and cleared to black, then OnInitialize and OnInitializeSurface fire — before the form's OnShow and first OnPaint (Windows creates child windows with the form's handle). What a program drew before that is lost, as on RapidQ ("you must wait for QDXScreen surface to initialize").
- **Size**: Init(W, H)'s, or the control's; with AutoSize (default) a control resized after the surface took its size gives the surface its size (DelphiX's AutoSize on WM_SIZE — RapidQ programs call Init, then set `Align = alClient`; `mgplist.bas` passes Init's width and height swapped and only shows right this way). AllowStretch (default) stretches the picture over the control when they differ, else it shows at its own size over black.
- **Colours**: RapidQ's &HBBGGRR, except `Fill` and `FastPset`, which take DirectDraw's own pixel value — &HRRGGBB on a 32-bit surface; the manual: "Fill … (&HFF = Blue, &HFF00 = Green, &HFF0000 = Red)", while its example draws "a red line" with `Line(…, &HFF)`.
- **Input**: OnClick / OnDblClick in the VCL's order, OnMouseDown (Button, X, Y, Shift) / OnMouseMove (X, Y, Shift) / OnMouseUp through the kernel's routing (the documented parameters come first). The screen takes no focus (a TCustomControl), so keys go to the form — what `stars.bas` / `shooter.bas` expect of `Form.OnKeyDown`.
- **QDXIMAGELIST** (`DxImageList`): `.DXG` parsed in Rust (the resource header, Delphi's binary form stream `TPF0`, TDIB / TBitmap pictures turned into BMPs for the shared decoder). `Draw(Item, X, Y, Pattern)`: the manual's fourth argument "Mask" is DelphiX's pattern index (0 = the whole picture when PatternWidth / PatternHeight are 0, as in every corpus library); Transparent pictures leave out TransparentColor (Delphi colour names or numbers). Pictures are shared (`Rc`) and keep their high-DPI layer, so a per-frame Draw copies nothing.
- **QDXTIMER**: a timer like QTIMER on both runtimes' timer machinery (`rapidr_ast::is_timer_type`; both compilers register it), Enabled False and Interval 1000 by default; Interval 0 fires once a 60 Hz frame (`DX_FRAME_MS` = 16: DelphiX's idle-time timer paced by a Flip waiting for the vertical blank); FrameRate is the OnTimers of the last whole second.
- **The rest of the 2D surface** (stage D1b, results below): FullScreen, ActiveOnly, Rotate, View.*, Cursor, a screen put on a form already shown, the screen's font. Not reproduced: BitCount's 8 / 16-bit colour reduction (RapidR draws 32-bit, as a windowed DirectDraw on today's screens does) and the display mode change of a full screen.

### 2.2 The 3D layer: a retained scene, rendered into the back buffer

**The model** — `rapidr_value::objects::d3d` (GUI-free, wasm-safe, shared by every runtime):

- A scene per QDXSCREEN: the hidden root frame, the camera frame (SetCameraPosition / SetCameraOrientation / CameraLookAt with D3DRMCONSTRAIN_*), the viewport (front 1, back 5000 as D3DRM's defaults; View.SetFront / SetBack / SetPlane), background colour or image, render mode (D3DRMRENDERMODE_BLENDEDTRANSPARENCY …), texture quality.
- Frames: a tree (AddFrame / DeleteFrame / CreateFrame relative to a parent), each with a 4 × 4 transform (SetPosition, SetOrientation, SetRotation axis + angle per Move, AddScale with D3DRMCOMBINE_*), velocity (SetVelocity), visuals and lights attached; `Move(delta)` advances every frame's rotation / velocity by delta, as `IDirect3DRM::Tick`.
- Visuals: mesh builders (vertices, faces with per-face colour, normals computed as D3DRM does, per-vertex texture coordinates, colour / alpha, quality = fill mode × shade mode × light mode — points / wireframe / solid, flat / Gouraud / (Phong as Gouraud, as D3DRM did)), textures (BMP, JPEG and PNG through the shared decoder; D3DRM's colour-count reduction is not reproduced), wraps (flat, cylinder, sphere, chrome — computed on the CPU into texture coordinates, `Apply` / `ApplyRelative`), shadows (CreateShadow: the mesh projected on a plane from the light, a visual of its own).
- Lights: ambient, point, spot (umbra / penumbra, range), directional, parallel point; colours as D3DRM's RGB lighting model (the ramp model's monochrome quirks are not reproduced). Fog (linear) on frames.
- **`.X` loading**: a parser of DirectX's text and binary formats (templates, `Header`, `Frame`, `FrameTransformMatrix`, `Mesh` + `MeshNormals` / `MeshTextureCoords` / `MeshMaterialList` / `Material` / `TextureFilename`, `MeshVertexColors`), in Rust (no crate covers both formats under a permissive licence), tested on hand-written files. MeshBuilder.Load takes the meshes merged (CONV3DS `-m` files), Frame.Load the hierarchy.
- QD3DVECTOR is a TYPE-like value (X / Y / Z unions DVX / DVY / DVZ), QD3DFACE / QD3DLIGHT / QD3DWRAP / QD3DTEXTURE handles into the scene.

**The renderer** writes into the QDXSCREEN's back buffer at `Render` (and at `ForceUpdate`'s rectangle), so the 2D drawing after it, `Pixel`, Flip and captures need nothing new. Two implementations behind one trait (`fn render(&Scene, &mut Bitmap /* back buffer, at the display scale */)`), as the desktop already has vello and vello_cpu:

1. **A software rasterizer** (`rapidr-d3d-soft`, pure Rust): perspective-correct, z-buffered triangles with flat / Gouraud shading, point-sampled or bilinear textures (texture quality), alpha blending, wireframe and points. It is the reference (captures and goldens exact, as every other GUI test), the headless host's and the CPU renderer's path, and **the web's path without WebGPU** (Firefox on Linux and Android in 2026, and every page today: the web host plan's renderer is the CPU). The corpus's scenes are small enough for it at full frame rate at 1× and 2× (to be measured in D4; budget: ≤ 4 ms for 2,000 textured triangles at 640 × 480 on one core).
2. **wgpu** (`rapidr-d3d-gpu`, behind a `gpu` feature): the same scene as vertex / index buffers, a WGSL pipeline per render mode, lights as a uniform block; rendered offscreen and read back into the back buffer (desktop: synchronous `device.poll`; the web's WebGPU: the readback's `mapAsync` suspends the VM at `Render` as the VM already suspends for ShowModal). Chosen as the desktop host chooses vello — the GPU when wgpu finds an adapter, the software rasterizer otherwise, `RAPIDR_RENDERER=cpu|gpu` to force, captures on the CPU — and held to the matrix's tolerance against the reference.

**Recommendation**: build the scene model and the software rasterizer first (D3, D4): they are needed whatever else exists (tests, no-GPU machines, the web without WebGPU), they make the 3D programs run on all three runtimes at once, and they fix the semantics. Then wgpu (D5), the user's chosen translation layer, as the accelerated path for the desktop and WebGPU, adopted when its captures match the reference within tolerance and measurements show it pays (large windows at 2× / 3×). This is the "one implementation" rule as the desktop host keeps it: one model, two renderers required by the platforms, not an old path kept as a fallback. (If the user prefers wgpu first, D5 can go before D4; the GUI tests then need a GPU, and the web without WebGPU shows no 3D until D4.)

### 2.3 Sound: QDXSOUND (done: "Stage D2 results" below)

A shared model (`objects::directx::DxSound`) holds the WAV (8 / 16-bit PCM, mono or stereo), FileName, Size, Frequency (the file's rate once loaded), Volume, Pan, Looped and where it plays; Playing and Position follow the runtime's clock, not the device, so they read the same everywhere and with no sound card. The runtime's sound device plays what the model asks for: rodio on the desktop (the `audio` feature PLAYWAV uses; a sink per QDXSOUND), Web Audio in the browser (an `AudioBufferSourceNode` with `playbackRate` through a `GainNode`, the page's shared AudioContext). AutoUpdate / BufferLength / Update / StickyFocus are DirectSound streaming details: kept, no effect.

### 2.4 Joysticks: a RapidR extension through gilrs, not winmm

RapidQ has no joystick object. Programs that DECLARE winmm's `joyGetPos` / `joyGetPosEx` keep RapidR's clear error for Windows DLL calls (the portability policy), naming the RapidR equivalent: a new, additive `RJOYSTICK` object (Index, Connected, X / Y / Z / R / U / V axes scaled like JOYINFOEX's 0 … 65535, Buttons bit mask, POV in hundredths of degrees like JOY_POV*, Name; OnButtonDown / OnButtonUp / OnMove events), backed by **gilrs** on the desktop (XInput / DirectInput on Windows, IOKit on macOS, evdev on Linux) and the browser's Gamepad API on the web (polled in the page's frame loop). Low priority: one corpus program, and only through a DLL.

### 2.5 Licences

Everything here is MIT / Apache-2.0 / BSD / Zlib, allowed by `deny.toml`; `cargo deny check licenses` and `tools/third_party_notices.py` run when a stage adds a crate (the 2D slice adds none).

| Component | Stage | Licence | Notes |
|---|---|---|---|
| RapidR's own code (directx.rs, `.DXG` and `.X` parsers, the software rasterizer) | D1–D4 | MIT (RapidR's) | |
| rapidr-value's image decoders (png, jpeg-decoder, resvg / tiny-skia) | D1, D3 | MIT OR Apache-2.0, BSD-3-Clause | already shipped |
| wgpu, wgpu-core, wgpu-hal, wgpu-types, naga (30.0.1, the workspace lock's, through vello) | D5 | MIT OR Apache-2.0 | already shipped by the desktop host |
| bytemuck | D5 | Zlib OR Apache-2.0 OR MIT | already shipped |
| glam (optional; the math can stay hand-written) | D3 | MIT OR Apache-2.0 | to add only if used |
| rodio, cpal, hound (rodio's WAV decoder) | D2 | MIT OR Apache-2.0 / Apache-2.0 | already shipped (`audio`) |
| gilrs, gilrs-core | D6 | MIT OR Apache-2.0 | new; on Linux it links the system's libudev dynamically (LGPL-2.1+, a system library like libc, through the MIT `libudev-sys`), on macOS IOKit, on Windows the `windows` crate (MIT OR Apache-2.0); version pinned and `cargo deny` run when adopted |
| web-sys (Gamepad, WebGPU, Web Audio bindings) | D2, D5, D6 | MIT OR Apache-2.0 | already shipped; new features only |

No RapidQ example file is shipped or used by the tests: the slice's `.DXG` fixture is RapidR's own drawing (`tools/make_dx_fixture.py`).

---

## 3. Stages

A session is one focused agent session ending in a green commit.

| Stage | Content | Sessions |
|---|---|---|
| D1 | The 2D layer: QDXSCREEN (drawing, Flip, Init / AutoSize / AllowStretch, set-up events, mouse), QDXIMAGELIST (`.DXG`), QDXTIMER (Interval 0, FrameRate) on the kernel, the interpreter, native builds and the web's DOM runtime; the fixture; four corpus programs by eye | **done** (results below) |
| D1b | The 2D leftovers: FullScreen, ActiveOnly, Cursor over the screen, Rotate, View.*, a QDXSCREEN put on a form already shown, the screen's font | **done** (results below) |
| D2 | QDXSOUND on rodio and Web Audio (§2.3); `sound/*.bas` | **done** (results below) |
| D3 | The scene model and the `.X` loader (§2.2), with unit tests over hand-written text and binary files; QD3D* types in the compilers' tables, `RapidQ_D3D.inc` compiling | 4–5 |
| D4 | The software rasterizer; `Render` / `ForceUpdate`; goldens; the 20 3D corpus programs run on the three runtimes and compared by eye with RapidQ's look | 3–4 |
| D5 | The wgpu renderer (desktop GPU, WebGPU with VM suspension at the readback), selection and `RAPIDR_RENDERER`; tolerance tests against D4 | 3–4 |
| D6 | `RJOYSTICK` on gilrs and the Gamepad API; the winmm error names it | 1–2 |

Total about 15–19 sessions after D1. D3 → D4 → D5 are in order; D1b, D2 and D6 are independent lanes.

### 3.1 Tests

- **GUI fixtures** in `tests/gui_parity_cases.mjs`, run by `tests/native_gui_events.mjs` (native and interpreted, 1× and `RAPIDR_SCALE=2`) and `tests/web_gui_parity.mjs` (`RAPIDR_DPR=1` and `2`): dumps of what the program reads (`Pixel`, sizes, event order, FrameRate's presence) and, new with D1, **`pixels`**: colours read from the desktop capture at given points, and a `webCheck` reading the page's canvas. CPU paths are exact.
- **The GPU path** (D5): a capture of each 3D fixture by both renderers, at most 0.5 % of pixels off by more than 8 per channel (the desktop matrix's rule), plus an exact-pixel test of the software rasterizer.
- **Unit tests** in the models: Flip / colours / TextRect / AutoSize (`objects::directx::tests`), the kernel component's revision caching (`components::dxscreen::tests`), the `.DXG` parser on a hand-made stream; D3: the `.X` parser, frame transforms, Move, wraps; D4: rasterizer edge cases (clipping, z ties, texture wrap).
- **The corpus** (by eye, at 1× and 2×): `directx/*`, `Mouse/select2dx.bas`, `imageLibrary/mgplist.bas`, then `direct3d/*`, against RapidQ's screenshots where the phatcode mirror has them.

### 3.2 Risks

1. **D3DRM's exact semantics** (the most serious): its lighting (RGB vs ramp model), `Move` / SetRotation's per-tick quaternion steps, the left-handed coordinate system and camera conventions, CreateShadow's projection, wrap formulas. Mitigation: Microsoft's D3DRM documentation describes most formulas; implement the model against it and compare the corpus by eye; record each judgement in the model's docs.
2. **The manual's gaps for 2D**: whether Pixel / FillRect take &HBBGGRR (chosen, as the manual's Line example and the corpus's colours suggest) or a device colour like Fill; Init vs AutoSize (chosen from DelphiX and `mgplist.bas`). Mitigation: the choices are in one model (`directx.rs`), documented, easy to flip if RapidQ evidence turns up.
3. **`.X` edge cases** (binary token streams, templates with restrictions, files written by old exporters). Mitigation: a tolerant parser that skips unknown templates; the 55 corpus files as a smoke set.
4. **CPU cost at high DPI**: a 640 × 480 screen at 3× is 2.8 M pixels a frame for Fill + conversion; 3D at 3× quadruples the rasterizer's work. Mitigation: measure in D4; the wgpu path (D5); damage tracking in the hosts (web plan W9).
5. **WebGPU readback latency** (async): one frame of delay at `Render` on the web's GPU path. Mitigation: the software path is the web's default.
6. **Timers and frame pacing**: Interval 0 is a 16 ms timer, not the display's vsync (a 120 Hz screen still gets 60 OnTimers). Mitigation: a host frame callback (winit's redraw, the browser's requestAnimationFrame) could drive QDXTIMER later; programs only see FrameRate.

---

## Stage D1 results (2026-10-05) — the 2D layer

Code:

- `crates/rapidr-value/src/objects/directx.rs` (new): `DxScreen` (back / front `Bitmap`s, Init, Flip, Fill / FastPset's device colour, Pixel, TextRect clipped, `follow_control` for AutoSize), `DxImageList` with the `.DXG` reader (Delphi's form stream, TDIB / TBitmap pictures, patterns, Delphi colour names), `DxTimer` (FrameRate), `timer_interval_ms`. Wired in `objects/mod.rs`: `Object::DxScreen / DxImageList / DxTimer`, `create` for RDXSCREEN / RDXIMAGELIST / RDXTIMER, get / set (`Font = QFont`, `Font.*`), call (Draw / StretchDraw / CopyRect of bitmaps, TextRect, the image list's LoadFrom* and Draw onto its Parent screen), `is_dxscreen`, `with_dxscreen`, `dxscreen_initialize`, `dxscreen_control`, `dxtimer_fired`; RDXSCREEN's default size 100 × 100 (`layout.rs`) and accessibility role Canvas (`a11y.rs`).
- `crates/rapidr-ui-kernel/src/components/dxscreen.rs` (new) and one `KINDS` line: the front as `Op::Image` by revision, stretched with AllowStretch, black around it without; no focus; clicks as the VCL's.
- `crates/rapidr-runtime-core/src/directx.rs` (new): `form_built` (set-up and its two events), `timer_interval`, `timer_fired`. Marked one-line hooks in `ui/kernel.rs` (`build_form`, `timer_interval`, the timer loop) and in `object.rs` (defaults for RDXSCREEN / RDXTIMER, QDXTIMER in the timer property hook and `rp_stop_all_timers`, a Flip redraws, `is_component_type`).
- `crates/rapidr-runtime-web/src/directx_web.rs` (new) and `gui_web.rs` / `object_web.rs`: the screen as a black `<div>` holding a `<canvas>` (`render_dxscreen` on Flip and set-up), set-up at OnShow / ShowModal / the first show, QDXTIMER in `update_timer` (Interval 0 → 16 ms, FrameRate), the screen's clicks as the VCL's, defaults, the type list.
- Compilers: `rapidr_ast::COMPONENT_TYPES` gains RDXSCREEN, RDXIMAGELIST, RDXTIMER (QDXSCREEN … map to them), `RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED` loses QDXSCREEN, QDXIMAGELIST, QDXTIMER (QDXSOUND and the QD3D* stay); `rapidr_ast::is_timer_type` registers QDXTIMERs in both backends (codegen `lib.rs`, `jumps.rs`; bcgen).

Tests:

- `tests/fixtures/dx_screen.bas` with `tests/fixtures/dx_sprites.dxg` (made by `tools/make_dx_fixture.py`), case `dx_screen` in `tests/gui_parity_cases.mjs`: OnInitialize then OnInitializeSurface, a QDXTIMER with Interval 0 ticking three times, the screen's OnMouseDown (Button, X, Y), Fill's blue against FillRect's red, Pixel, Circle's fill, a transparent sprite and two patterns of a strip from the library, TextWidth, the control's size; `pixels` checks the desktop capture, `webCheck` the page's canvas. Passing native and interpreted at 1× and `RAPIDR_SCALE=2`, and on the web at `RAPIDR_DPR=1` and `2`.
- `tests/native_gui_events.mjs` learned `pixels` (a case's colours read from the window's capture).
- Unit tests: `objects::directx::tests` (7), `components::dxscreen::tests::shows_the_last_flip`.
- Corpus, by eye in captures (interpreted; the shooter natively too, the scroller at 2× too; the ball and the scroller in the browser too): `directx/dxball.bas` (the rotating ball and "FPS: 55"), `directx/starfield/stars.bas`, `directx/scroll/dx.bas` (the `.DXG` scene and car, Arial 14 bold FPS text), `directx/shooter/shooter.bas` (both ships with see-through white), `imageLibrary/mgplist.bas` (its swapped Init followed by AutoSize). The manual and the phatcode mirror have no screenshots of these programs; the look matches what their code draws.

Open after D1: the D1b list (done below); QDXSOUND and the 3D objects (D2–D5) still compile as generic objects whose methods warn.

## Stage D1b results (2026-10-05) — the rest of the 2D surface

Each item, with the judgement it needed (the manual says little; DelphiX and the corpus decide):

- **The screen's font: MS Sans Serif 8** until the program gives one (`Font = QFont`, `Font.Size`). DelphiX's surface canvas was a TCanvas with Delphi's default TFont, and the manual's chapter 13 screenshots (`.reference/phatcode/chap13d3.gif`, `chap13d4.gif`) show "FPS: 20" in that small font. TextWidth("Hi") is 10 now (Arial 10's was 12).
- **FullScreen** (4 corpus programs, all bsNone forms): the form's window covers the screen without a frame — winit's borderless full screen (`HostCmd::Fullscreen`), the headless host's form taking the screen size itself; on the web the form takes the page (`position: fixed`, 100vw × 100vh) because the browser's own full screen needs a user gesture the program's start doesn't have. DirectDraw's exclusive mode changed the display to the surface's size; RapidR changes no display mode: the surface keeps Init's size (AutoSize doesn't follow the control in full screen) and shows scaled to fit with its proportions kept, centred, black around (`directx::picture_rect`; a modern screen's "keep aspect ratio" scaling). There is no way back, as the manual says.
- **ActiveOnly** (default True): a QDXTIMER's OnTimer fires only while the program is the active application — on the desktop while one of its windows has the keyboard (`Host::active`, winit's Focused events), always on the headless host (the tests); on the web while the page shows (`document.hidden`: a page can't tell more). FrameRate counts only the frames that fired.
- **Rotate(xOrigin, yOrigin, Angle)**: what the back buffer holds turned about the point, clockwise, nearest pixel, where nothing turns onto the pixels stay. Angle is in DelphiX's units, 256 a whole turn (its `Cos256` / `Sin256` tables) — a judgement call: no corpus program calls Rotate and the manual gives no unit.
- **View.*** (`DX.View.SetFront(10)` compiles to the method `view.setfront`, `DX.View.GetFront` to the property `view.getfront`): SetFront / SetBack / SetPlane keep the 3D viewport's values for stage D3 (front 1.0, D3DRM's default; back 5000, as the manual's comment says); GetFront / GetBack read them; View.Clear clears the back buffer to the scene's background (black until D3's SetBackgroundRGB).
- **A screen put on a form already shown** (`Late.Parent = Form` in a handler): set up — OnInitialize, OnInitializeSurface — once the program's code returns (a deferred job on the desktop, a zero timeout on the web), so the CREATE block or handler has given it its size and handlers first. A hidden form's screens are set up when it's first shown, on both runtimes.
- **Cursor = crNone** over the screen: the components' Cursor already worked on both runtimes (the kernel host's `platform::cursor_at`, the web's CSS); checked in the page.
- **A native build divergence fixed on the way**: `Obj.Sub.Method(…)` statements called the method on a value read from the object; they now call `sub.method` on the object, as the VM always did (`rapidr-codegen-rust`'s `object_method_call`).

Tests: `tests/fixtures/dx_more.bas`, case `dx_more` (dumps: the timer's ticks with ActiveOnly, the rotated line's pixels, View's values, the late screen's and the hidden form's set-up, the full screen's width and its surface keeping 64 × 48, the font's sizes; `pixels`: the rotated line and the late screen's blue in the capture; `webCheck`: the screen's `cursor: none`, the same pixels in the page, the full-screen picture's 4:3 proportions). Unit tests: `objects::directx::tests::{font_view_rotate, picture_placement}`, the kernel's `dxscreen::tests` with FullScreen's placement. Passing native and interpreted at 1× and 2×, and on the web at DPR 1 and 2.

Open: ActiveOnly can't be exercised by the GUI tests (their host is always active); a page in a background tab is "inactive" on the web, a window of another application in front on the desktop.

## Stage D2 results (2026-10-05) — QDXSOUND

Code: `DxSound`, `Wav` / `parse_wav`, `SoundDevice` / `SoundPlay`, `set_clock` in `rapidr-value/src/objects/directx.rs` (FileName read in `objects::set`); rodio's device in `rapidr-runtime-core/src/sound.rs` (`install_dx_device`, made when the first QDXSOUND is; never under a GUI test); Web Audio's in `rapidr-runtime-web/src/directx_web.rs` (`install_sound_device`, the clock from `Date.now`); RDXSOUND in both compilers' component types (QDXSOUND off the not-yet list).

Judgement calls (the manual leaves them open):

- **Volume is decibels**: DirectSound attenuates in hundredths of a decibel, and a percent is 100 − dB (gain 10^((v − 100) / 20); 0 is silent). The corpus's `qdxsound2.bas` gives its volume scroll bar 70 … 100 — below 70 it's next to silence, which a linear percent wouldn't be. **Pan** the same way: the other side attenuated by |Pan| dB, −100 muting the right channel and 100 the left (the manual).
- **Size is the sound's bytes** (the WAV's `data` chunk), the manual's "file size" minus its header, because Position runs over the same bytes (the manual's example gives a track bar Max = Size, Position = Position).
- **The end of a sound played once**: Playing turns False and Position goes back to 0, as a DirectSound buffer's play cursor did. Stop keeps the place; Play goes on from Position.
- **A change while it plays is heard at once**: Volume through the device's gain; Pan, Frequency, Looped and Position play again from where the sound is (a click may be heard, where DirectSound changed a buffer's parameters in place).
- **No sound under the GUI tests** (`RAPIDR_CAPTURE` / `RAPIDR_TEST_EVENTS`); the model's Playing and Position still run by the clock.

Tests: `tests/fixtures/dx_sound.bas` with `tests/fixtures/dx_beep.wav` (made by `tools/make_dx_fixture.py`; extracted from a `$RESOURCE` with EXTRACTRESOURCE, as a RapidQ program would, and KILLed at the end), case `dx_sound`: Size and Frequency from the file, the defaults, Play / Playing, Stop keeping the place, Position and Frequency set, the end of a sound played once. Native and interpreted, and the browser (through Web Audio itself: no page errors). Unit test `objects::directx::tests::sound` (a fake clock and device: positions, the end, looping, what the device is asked — gains, speed, frames). The device paths ran for real once each, inaudibly (Volume 1). Corpus: `sound/qdxsound2.bas` runs (Size 16052, Frequency 11025, Playing after its button).

Open: WAV only (DirectSound's buffers were PCM; `rodio` could decode more); 8-bit and 16-bit PCM; one device sink per QDXSOUND.

