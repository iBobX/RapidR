// The Windows 7 builds (`*-win7-windows-gnu`, tools/win7/build.sh): load the
// DLLs that only Windows 8 and later have on first use instead of at
// start-up, so that Windows 7 starts the program (src/win7.rs answers when
// they are missing). Every other target: nothing to do.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let var = |k: &str| std::env::var(k).unwrap_or_default();
    if var("CARGO_CFG_TARGET_OS") == "windows" && var("CARGO_CFG_TARGET_VENDOR") == "win7" && var("CARGO_CFG_TARGET_ENV") == "gnu" {
        // ole32.dll: Windows 7 has it, but not CoIncrementMTAUsage
        for dll in ["api-ms-win-core-winrt-l1-1-0.dll", "api-ms-win-core-winrt-error-l1-1-0.dll", "api-ms-win-core-path-l1-1-0.dll", "combase.dll", "ole32.dll"] {
            println!("cargo:rustc-link-arg-bins=-Wl,--delayload={dll}");
        }
    }
}
