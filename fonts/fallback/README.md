# Fallback fonts

These fonts draw what the built-in Liberation fonts (`crates/rapidr-value/fonts`) lack: symbols such as ✓, and Chinese, Japanese and Korean. They are Noto fonts by the Noto Project Authors and Adobe, under the SIL Open Font License 1.1 (`OFL.txt`). See LICENSES.md §4 and docs/web-host-plan.md §3.7.

- **Here, unmodified:** Noto Sans, Noto Sans Symbols and Noto Sans Symbols 2 (Regular, unhinted OTF, about 0.8 MB), so a source build without the network still covers symbols.
- **Fetched, not in the repo:** Noto Sans SC and Noto Sans KR (Noto CJK Sans 2.004). `python3 tools/fonts.py fetch` gets them into `target/fonts-src`. They come from the official release assets pinned in `fonts.toml`, by version and SHA-256.
- **Chunks:** `python3 tools/fonts.py build target/web/fonts` writes what the web runtime loads on demand: `index.json` (which file has which characters), the fonts split by Unicode range (each CJK chunk renamed `<family> NNN`), and `OFL.txt`. `tools/build_web_artifacts.sh` runs it.
- **Who ships the chunks:** web bundles, `rapidr build --web` sites and an install's `lib/rapidr/web/fonts` carry them. An installed RapidR never downloads fonts.

None of these fonts declares a Reserved Font Name, so the OFL allows the subsets and their names.
