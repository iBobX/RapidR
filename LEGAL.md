# Legal: RapidR and the programs you build

This page says, in plain language, what you may do with RapidR and with the
programs you make with it, and what to ship with them. It describes the
licences of RapidR and of the open-source software inside it as the project
understands them; it is not legal advice. The details, output by output and
licence by licence, are in [docs/licensing.md](docs/licensing.md).

## In short

- **RapidR is free and open source**, under the [MIT License](LICENSE).
- **The programs you build with RapidR are yours.** Sell them, give them
  away, keep their source closed, use them in a business: no fee, no
  royalty, no registration, and no need to publish your source code.
- **What to ship:** your program, plus the `THIRD-PARTY-NOTICES.txt` that
  RapidR writes next to it every time you build. That's all.

RapidR's runtime and the open-source libraries it is built from go into your
program. Their licences (MIT, Apache-2.0, BSD, ISC, zlib, Unicode, MPL-2.0,
the SIL Open Font License, …) all allow commercial and closed-source use;
what they ask in return is that their copyright notices and licence texts go
with the program. `THIRD-PARTY-NOTICES.txt` is exactly that, made from the
build's real list of components, with every licence text in full.

## What to ship, for each kind of output

| You built | Ship |
|---|---|
| A native executable (`rapidr build`) | the executable and `THIRD-PARTY-NOTICES.txt` from the same folder |
| An interpreted executable (`rapidr build --interp`) | the executable and `THIRD-PARTY-NOTICES.txt` from the same folder |
| A web build (`rapidr bundle-bc`, `rapidr build --web`, the web IDE's **Build**) | the whole folder or `.zip` as built: `THIRD-PARTY-NOTICES.txt` is in it, and `index.html` links it |
| A `.rrbc` (`rapidr build-bc`) | just the `.rrbc`: it holds only your program. Whoever runs it uses the RapidR Runtime, which carries its own notices (if you hand out the Runtime's installer too, hand it out unmodified) |

- Keep the file's text as it is. You may rename it, put it in an installer,
  in an app bundle (e.g. `Contents/Resources` on macOS), in a "Licences" or
  "About" screen or in your manual: what matters is that it reaches the
  people who get your program.
- Several programs built for the same system share one
  `THIRD-PARTY-NOTICES.txt`: the file depends on the kind of output, never on
  your program.
- You can make the file at any time: `rapidr notices` (this computer's
  executables), `rapidr notices windows-x86_64`, `rapidr notices web`.

## What stays your responsibility

- **Your own content.** Images, sounds, fonts, data and code you add to your
  program (`$RESOURCE` files, files next to a web build, `$INCLUDE` files
  from elsewhere) come with their own terms. What RapidR provides for
  RapidQ's include names (`QDirListView.inc`, `QDockForm.inc`, the
  constants of `RAPIDQ.INC`, `qcgi.inc`'s CGI functions, …) is RapidR's own
  code, under MIT; if a program `$INCLUDE`s an include file from another
  source, that file's licence applies to it (RapidQ's own `qcgi.inc`, for
  one, is GPL — RapidR's built-in CGI support means you don't need it).
- **If you change RapidR itself** and ship the changed runtime, keep the MIT
  notice (the notices file already does). If you change one of the MPL-2.0
  libraries' files (listed in the notices file; RapidR doesn't change them),
  make your version of those files available as the MPL asks.
- **The built-in fonts** (Liberation Sans, Serif and Mono) are inside your
  program unmodified, under the SIL Open Font License: fine for any program,
  commercial included. Don't extract them to sell on their own, and if you
  change them, give your version another name.
- **Your toolchain on Windows.** With the `gnullvm` toolchain (LLVM-MinGW,
  open source) nothing is needed. With Microsoft's `msvc` toolchain, the
  Visual Studio Build Tools licence applies to you as the developer (it is
  free for individuals, open-source projects and small organisations;
  larger organisations need a Visual Studio licence). `rapidr setup` says
  which you have.
- **Laws that apply to software in general** (privacy, consumer law, export
  rules for encryption — desktop programs contain HTTPS code — and so on)
  are outside what licences cover.

## RapidQ, and the names RapidR mentions

RapidR is an independent, original implementation, written from scratch in
Rust, of a BASIC language compatible with **RapidQ**, the freeware compiler
by William Yu. RapidR contains no code from RapidQ, and does not include
RapidQ's distribution (its compiler, manual, include files or examples): its
compatibility is built from the language's documented behaviour and from
comparing programs' output. RapidR is **not affiliated with, endorsed by or
sponsored by** William Yu, by anyone who distributes RapidQ, or by any
company named in its documentation.

"RapidQ" is used only to say what RapidR is compatible with. Microsoft,
Windows, Visual Basic, Visual Studio and DirectX are trademarks of the
Microsoft group of companies; Delphi is a trademark of Embarcadero
Technologies; Apple and macOS are trademarks of Apple Inc.; Linux is a
trademark of Linus Torvalds; other names belong to their owners. They are
named only to describe compatibility or to identify a platform, never to
suggest endorsement.

## No warranty

RapidR is provided "as is", without warranty of any kind, and its authors
are not liable for any claim or damages arising from its use (the [MIT
License](LICENSE) has the exact words). The same holds for the open-source
components inside it, under their own licences.

## A word on legal advice

The project has checked every component's licence and keeps the checks
running (`cargo deny`, and a test that every kind of output carries a
complete notices file), and this page reflects that work honestly. It is
still not a lawyer's opinion. If you are building a commercial product, or
your organisation has rules about open-source software, have your own
counsel look at it, as you would for any toolchain. The project itself
intends to have its licensing reviewed by a professional before its first
public release.

Questions and corrections: open an issue at
<https://github.com/iBobX/RapidR/issues>.
