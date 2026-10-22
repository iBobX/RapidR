# Truly native UI for RapidR: one component model, the real widgets of every OS (plan, 2026-10-19)

Robert (2026-10-19): bring declarative UI (live previews, adaptive layout, state that drives the screen) to every platform, with **real** native UI. That means a UI layer that does for widgets what a graphics translation layer does for DirectX → Vulkan/Metal: one RapidR program, and on each OS the platform's own buttons, lists, menus and text fields. It ships as a Rust crate others can use, and RapidR uses it for `$UI native`. It's a long road, and that's fine: it lets students and people with few resources target real OSes and devices (macOS, Windows, Linux, iOS/iPadOS, Android).

**Decision for v1:** RapidR 1.0 ships with today's drawn UI kernel and native-*looking* themes (Fluent, macOS, Adwaita) under `$THEME native`. The truly native layer is the post-1.0 project described here.

## The idea in one picture

```
                RapidR program (.rr / .bas)   ·   Rust apps using the crate
                                   │
         ┌─────────────────────────┴──────────────────────────┐
         │  The model (shared, pure Rust, exists today)       │
         │  components · properties · events · layout ·       │
         │  data binding · accessibility metadata · registry  │
         └─────────────────────────┬──────────────────────────┘
                                   │   the contract: a UI "HAL"
     ┌──────────┬──────────┬───────┼──────┬──────────┬──────────┬──────────┐
  kernel     AppKit     Win32 →  UIKit   Android     GTK 4      DOM
 (drawn;    (macOS)     WinUI 3  (iOS,   Views       (Linux,    (the web:
 reference, today)      (Windows) iPadOS) (JNI)      decision)  real HTML)
```

wgpu does this for graphics: one API, a backend per platform, capability flags, and one conformance suite that runs on every backend.

## The three principles that make it work

1. **Our layout, their widgets.** RapidR computes every frame (`Left/Top/Width/Height`, `Align`, `Anchors`, and the stack and grid panels on the roadmap) in shared code. A backend only creates the native widget and places it. Programs keep the same geometry everywhere, while each control looks, feels and speaks like the OS. Native metrics (a macOS button's height, a Windows edit's padding) are taken into account by the layout, so nothing looks squeezed.
2. **Native where it exists, drawn where it doesn't.** Buttons, edits, lists, trees, tabs, menus, dialogs and the tray are native. `RCanvas`, `RPlot`, the data grid, DirectX and rich custom components are drawn by our kernel inside a native view: a wgpu surface on the native window, with AccessKit for accessibility. The two mix freely in one form. A component with no native backend yet simply falls back to its drawn version, so a program never fails to run.
3. **Measured parity, not promised parity.** The conformance suite we already have (scripted events plus dumps of what the program sees) runs against **every** backend. Where an OS behaves differently (when `OnChange` fires, for example), the difference is written down per backend. RapidQ-exact behaviour stays guaranteed on the kernel, the default. `$UI native` is opt-in.

## What "declarative magic" means, and how we get it

| Declarative magic | RapidR |
|---|---|
| One code, native on every Apple device | One code, native on every OS (this plan) |
| Live previews | Already there: the live designer shows real data while you design |
| Adaptive layout (stacks) | `RStackPanel` and a grid panel (roadmap), plus responsive `Anchors` |
| State drives the UI | Data binding, already started (`DataSource` / `DataField`); next: `Caption = BIND(Total)` for any variable |
| Declarative views | Already the spirit of `CREATE … END CREATE`; a declarative layout block later, to be designed |

## The backends, in order

| # | Backend | Why this order | Built on (licence) |
|---|---|---|---|
| N1 | **The contract** | The kernel moves behind the backend trait, becoming backend #1 and the reference. Nothing changes for users | — |
| N2 | **macOS: AppKit** | Robert develops on a Mac; `objc2` is excellent; AppKit maps closely to RapidQ's controls (NSButton, NSTextField, NSTableView, NSOutlineView) | `objc2` (MIT/Apache); AppKit is the system's |
| N3 | **Windows: Win32 common controls**, later **WinUI 3** | RapidQ itself was Win32, so `$UI native` on Windows brings RapidQ programs home to real Win32 controls. WinUI 3 follows for the Windows 11 look | `windows-rs` (MIT/Apache); comctl32 is the system's; Windows App SDK (MIT, a redistributable runtime) |
| N4 | **iOS / iPadOS: UIKit** | Reuses N2's Apple glue. Touch, the app lifecycle, safe areas, and a form becoming a screen. The bytecode VM is an interpreter (no JIT), so it's App Store-friendly | `objc2` (MIT/Apache); Xcode builds; a device or App Store build needs the user's own Apple Developer account |
| N5 | **Android: Views through JNI** | Mobile for everyone, the most widespread device in schools | `jni`, `ndk` (MIT/Apache); the Android framework (Apache-2.0); Studio generates the Gradle project |
| N6 | **Linux: decided, our drawn kernel with the GNOME (Adwaita) look** | Linux has no single "native", and GTK is LGPL. Under the zero-risk rule (2026-10-19, Robert: "you decide") RapidR ships no GTK. Revisit only if many users ask, and then only GTK 4 linked from the system, never bundled, with Robert's approval | our own code |
| N7 | **The web: real DOM controls** | `<button>`, `<input>`, `<select>`: native for browsers, great for mobile browsers and screen readers. *Note:* Robert decided in 2026 that the canvas kernel is the only web host; this would be an opt-in `$UI native` for the web, to be re-decided then | `web-sys` (MIT/Apache) |

## How RapidR uses it

- `$UI native` (the program) or a Studio project setting picks the native backend. `$THEME native` remains the *look* of the drawn kernel, so the two are distinct and both useful.
- Studio's designer shows the native look per target ("preview as macOS / Windows / iPad / Android"), as a live preview shows a device.
- Builds: `rapidr build --target ios|android` generates and drives the Xcode or Gradle project. `rapidr setup` installs what's missing, after asking, as it does for Rust today.
- **The crate is public.** `rapidr-ui` (name to be decided after a trademark check; never Apple's or Microsoft's names) is published under MIT for any Rust developer. Every Rust app that uses it is a door to RapidR.

## Quality bar (each backend, before it's called done)

1. The conformance suite passes on it, with the differences documented.
2. The OS's own accessibility tree checks out: our AX probe on macOS, UI Automation on Windows, AT-SPI on Linux, TalkBack and VoiceOver on mobile.
3. The examples and the RapidQ compatibility suite run under `$UI native`.
4. Studio itself runs on it. That's the WOW proof, and our habit.
5. It's walked like a user, with real clicks, on real machines, VMs, simulators and emulators.

## Risks, said plainly

- **Size:** each desktop backend is a big project on its own, and mobile is bigger (a different app model). With AI lanes it's feasible step by step, but it competes with the weekly budget, so it goes after 1.0 and after the open gaps (our "close gaps first" rule).
- **Maintenance:** OS updates change behaviour. The conformance suite per backend, capability flags and the drawn fallback keep programs running.
- **Linux licensing:** decided at N6: no GTK, so the drawn kernel with the GNOME look.
- **App stores:** distributing on iOS or Android needs the user's own developer accounts. We document the path; the cost is theirs.

## First concrete steps (when this starts)

1. Write the backend contract (`UiBackend`, capability flags) from what the kernel already does, and move the kernel behind it. The tests stay green, and that's the proof.
2. Spike AppKit with `hello_form` (a form, a label, a button and its OnClick), then `pantry` (lists, edits), then the conformance cases.
3. Decide the crate's name (with a trademark check) and publish an early preview to collect feedback from Rust developers.
