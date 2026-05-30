# Third-Party Licenses

RapidR itself is licensed under the MIT License — see the top-level
[LICENSE](LICENSE) file. The redistributable artifacts that ship with
RapidR (the desktop binary, the in-browser IDE, and built web bundles)
include the third-party software listed below. Each entry names the
component, its upstream license, and how RapidR uses it.

If you redistribute RapidR or a built web bundle, please keep this file
alongside the binary and preserve the upstream copyright notices below.

---

## 1. Monaco Editor — MIT License

Vendored under `web-ide/vendor/monaco/` and used by the in-browser IDE for
source editing, syntax highlighting, and IntelliSense.

> Copyright (c) 2016 Microsoft Corporation. All rights reserved.
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in
> all copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND.

Upstream: <https://github.com/microsoft/monaco-editor>

---

## 2. wasm-bindgen / wasm-pack generated glue — MIT OR Apache-2.0

`web-ide/runtime/rapidrintr.js` and `rapidrintr_bg.wasm` are produced by
`wasm-bindgen` from the `crates/rapidr-runtime-web` Rust crate. The
generated JavaScript wrapper inherits the wasm-bindgen license terms.

> Copyright (c) 2014 Alex Crichton
> Licensed under either of Apache-2.0 or MIT, at your option.

Upstream: <https://github.com/rustwasm/wasm-bindgen>

---

## 3. Rust standard library and direct Cargo dependencies — MIT OR Apache-2.0

The compiled `rapidrintr_bg.wasm` and the desktop `rapidr` binary statically
link the Rust standard library and several third-party Rust crates declared
in `Cargo.toml` files under `crates/`. Each crate's individual license is
recorded in its source repository; the dominant licenses are MIT and
Apache-2.0. A complete dependency licence inventory can be generated with:

```
cargo install cargo-license
cargo license --json > LICENSES-cargo.json
```

---

## 4. Inter, Tahoma, MS Sans Serif fonts

The IDE references the system-installed `Inter`, `Tahoma`, `Arial`,
`Verdana`, `Times New Roman`, `Courier New`, `Segoe UI`, and
`MS Sans Serif` fonts via CSS only. No font files are vendored or
redistributed. End users supply their own fonts via the operating system
or browser. If `Inter` is bundled in a future release it will be added
here under the SIL Open Font License 1.1.

---

## 5. SQLite (via `rusqlite`) — Public Domain

The desktop runtime statically links SQLite via the `rusqlite` crate.
SQLite's source is in the public domain. See <https://www.sqlite.org/copyright.html>.

The browser-side `RSqlite` component uses `sqlite-wasm` only when explicitly
loaded by user code; nothing from sqlite-wasm is vendored in the IDE.

---

## 6. PKZIP file format (in-browser exporter)

`web-ide/zip.js` is an original implementation of the PKZIP "stored"
format (no compression), written from the public PKZIP APPNOTE.TXT
specification and licensed under the same MIT terms as the rest of
RapidR. It is **not** derived from any GPL/LGPL ZIP library.

---

## 7. RapidR (this project) — MIT License

See [LICENSE](LICENSE).
