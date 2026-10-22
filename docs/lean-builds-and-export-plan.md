# Lean builds and Export anywhere (plan, 2026-10-21, for 1.0)

Robert (2026-10-21) asked two questions:
1. Why should a program that uses a text box and a button carry everything RapidR has, the code editor included?
2. Could the web Studio build executables for Windows, macOS and Linux, not only for the web?

The answers: yes and yes. Both are part of 1.0's WOW.

## 1. Where we are
- **Programs' runners never include Studio's own components** (the project, the language service, the session: the `studio` feature).
- **Everything else is in every program.** `rapidr-runtime-core`'s default features are on for every build: database, network (with the AI client), gui, datascience, audio, ffi and gamepad.
  - The interpreted build's runner (about 44 MB unpacked) carries all of it.
  - A native build depends on the runtime with its default features.
- **Native builds can't drop much by themselves.** Rust's linker removes code nothing reaches, but a component reached through the runtime's name-based dispatch counts as reached, so it stays.
- **The kernel holds the code editor.** The public IDE components (RCodeEditor, RDiffView, RDesignSurface…) live in the UI kernel, so every GUI program carries them.

## 2. Lean builds: each program carries what it uses
1. **Finer features.** The runtime's features become one per component family:
   - `gui-core` (forms, buttons, edits, lists…);
   - `gui-ide` (the IDE's public components);
   - `canvas`, `d3d`, `r3d`;
   - `datascience`;
   - `db-sqlite` and the other database drivers, one feature each;
   - `network`, `ai`, `audio`, `midi`, `gamepad`, `ffi`, `serial`.

   The registry records each component's and builtin's feature (a new facet), and a test keeps every one of them assigned.
2. **The compiler knows what a program uses.** The semantic model already lists every component, member and builtin a program reaches. The build computes the feature set from that list:
   - **Native builds** write only those features into the generated `Cargo.toml` (`default-features = false`), plus the size profile (LTO, `strip`, `panic = "abort"`). Rust then drops everything else.
   - **Interpreted builds** pick the smallest prebuilt **runner flavour** that covers the program:
     - *console*;
     - *gui* (gui-core);
     - *gui+data*;
     - *full* (everything).

     A few flavours, built and signed with each release, are better than one runner per program.
3. **Why not dynamic libraries per component** (loading components as `.dll`/`.dylib`/`.so` at run time):
   - Rust has no stable ABI, so every component would need a C boundary;
   - versions would have to match exactly;
   - loading libraries is an attack surface (library hijacking);
   - macOS signing and notarisation get harder with many dylibs;
   - the gain over flavours is small.

   Static flavours are simpler, faster to start, and safer. It is the same code, only switched on or off, so there is no second implementation.
4. **Studio shows the size.** "Your app uses: GUI, Audio. Native ≈ N MB · Interpreted: the gui flavour". RAI sees it too.
5. **The web runtime** gets the same flavours as separate `.wasm` files: the export picks one, and the web Studio loads the one a program needs.
6. **Tested like everything else.** Every flavour runs the conformance suite and the examples it covers, and a program needing a feature its flavour lacks fails at build time with a clear message, never at run time. Size budgets per flavour are checked by `tools/regress.sh` (master-plan M3).

## 3. Export anywhere: Windows, macOS, Linux and the web, from any Studio, even the browser
An interpreted executable is a **runner plus the program's bytecode and resources**. Runners for every OS and CPU are built and signed with each release. So **any** Studio can export for **every** OS: the web Studio, or Studio on a Mac building for Windows. It attaches the bytecode to the right runner flavour. No compiler and no SDK are needed.

| Target | What the export makes | Honest notes |
|---|---|---|
| Windows x64 / arm64 | A folder (zip) with the runner `.exe` and the program's data file beside it, or one `.exe` | Keep the data **beside** the exe so the runner's signature stays valid. An unsigned exe shows SmartScreen's "unknown publisher" (the signing decision, M0) |
| macOS universal | A `.app` (zip) | Changing a signed app's contents breaks its seal, so the export makes an **ad-hoc signature with our own small signer**: a code directory of SHA-256 page hashes, no certificates, written from public descriptions (legal review 2026-10-22, 7b). We don't use the MPL apple-codesign crate (it brings `ring` and other banned crates), and we copy nothing from ldid or xnu. Note that macOS moves downloaded apps to a random read-only place, so the program must live inside the bundle, not beside it. Downloaded apps still show Apple's warning unless they carry a Developer ID signature, which is made on a Mac with the user's own Apple account. The manual explains "Open Anyway" plainly |
| Linux x64 / arm64 | `tar.gz` (runner + data); an AppImage later | No signing needed |
| The web | What we have today (one folder, or later a single HTML file: vmx-components-plan W5) | — |

- **Native, compiled builds stay on the user's own machine.** They need Rust and each OS's SDK. Building for macOS anywhere but a Mac would go against Apple's SDK licence, so **never**, and no build server. Export anywhere is the interpreted kind, which is fast and good for most programs.
- **Security:**
  - The web Studio downloads runners only from our release host, checks each one against the hashes and minisign signature shipped in the web Studio, and caches them.
  - The host must allow the browser to fetch them (CORS). GitHub's release downloads don't, so we use rapidr.dev or GitHub Pages.
  - No user code ever goes to a server.
- **Licences:** our runners and code only; any ad-hoc signer must pass `cargo deny` or get Robert's approval.
- **Signing identity (legal review 2026-10-22, 7c, High):** RapidR's own signing identity (Ruta Internet's Windows certificate or Apple Developer ID) signs only RapidR's own tools, **never export runners or users' code**. Otherwise any exported app, malware included, would show Ruta as its publisher, and a revocation would break every RapidR release. Export runners stay unsigned or ad-hoc; users sign their apps with their own certificates. Decided before any certificate is bought.

## 4. Where it sits
- **Lean builds:** master-plan **M3** (speed and size), since boards need the same small runtime.
- **Export anywhere:** **M7**, before 1.0, as part of its WOW ("Write it once. Build it for every computer, from any computer, even a browser").
- **Lanes:**

| Lane | Work | Model, size |
|---|---|---|
| L1 | The feature facet in the registry, finer runtime features, and the compiler's feature set | Opus L |
| L2 | Native builds with the minimal feature set and size profile; a size report | Sonnet M |
| L3 | Runner flavours for desktop and wasm, in the release chain, signed | Opus M |
| L4 | Export anywhere: desktop Studio, then the web Studio (download, verify, attach, zip) | Opus M |
| L5 | macOS export signing (needs a decision) and the manual's "first launch" pages | Opus S + Robert |
