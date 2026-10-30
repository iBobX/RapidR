# Legal review, 2026-10-30: the signed test mode (lane SEC-24, `crates/rapidr-testmode`)

This is the "lawyer agent" review (standing rule since 2026-10-22) for a new
dependency, a new file format and new commands. It is a careful risk review
from the files themselves, **not legal advice** (US/EU practice).

- **What is being built:** `rapidr-testmode`, the gate RapidR's GUI test hooks
  read their variables through (SEC-24, `docs/security/test-mode.md`). A
  release build's hooks act only with a token file signed with Ed25519 by a
  key Robert makes; `rapidr dev test-mode new-key | issue | status` make the
  key and the tokens. The crate is linked into every desktop runtime, so into
  users' (possibly commercial) programs.
- **Checked:** the crate's files in
  `~/.cargo/registry/src/*/ed25519-compact-2.6.0/` (`LICENSE`, `Cargo.toml`,
  `README.md`, every file in `src/`), `Cargo.lock`, `deny.toml`, the
  regenerated `THIRD_PARTY_NOTICES.md`.

## Verdict: MERGE

| # | Item | Risk | Decision |
|---|---|---|---|
| 1 | `ed25519-compact` 2.6.0 (Frank Denis): licence | None | MIT (`LICENSE`: "MIT License, Copyright (c) 2020-2026 Frank Denis"), on the `deny.toml` allowlist. Listed in `THIRD_PARTY_NOTICES.md` (MIT, 138). |
| 2 | Its dependencies as built | None | `default-features = false`: none. The optional `getrandom` (MIT OR Apache-2.0), `ct-codecs` (MIT) and `ed25519` (Apache-2.0 OR MIT) are off. |
| 3 | Provenance | None | Pure Rust by the author; `src/sha512.rs` says "(C) Frank Denis, public domain". No GPL/LGPL code, no file under another licence found in `src/`. |
| 4 | `getrandom` 0.4 (the `issue` feature, the CLI only) | None | MIT OR Apache-2.0, already in the tree (runtime-core's tokens). No new crate. |
| 5 | `libc` 0.2 (the computer's name for a token's optional machine hint) | None | MIT OR Apache-2.0, already in the tree. |
| 6 | Ed25519 (the algorithm, RFC 8032) | None | Public, royalty-free; no patent claim known. Signature *verification* inside programs is not encryption of user data. |
| 7 | Export rules (US EAR, EU 2021/821) | None | Signatures used only to check that a file is authentic are generally not treated as controlled encryption; nothing here encrypts data, and RapidR already ships TLS through the operating system. |
| 8 | The token and key file formats | None | RapidR's own plain-text formats; no third-party format copied. |
| 9 | Names (`test mode`, `rapidr dev test-mode`) | None | Plain descriptive words; no other product named. |
| 10 | Keys and tokens | None | No key is created by the lane or stored in the repository (a placeholder that trusts nothing); Robert makes the real key (`docs/security/test-mode.md`). Fixture keys exist only inside unit tests. |

Nothing to fix. `THIRD_PARTY_NOTICES.md` was regenerated
(`python3 tools/third_party_notices.py`).
