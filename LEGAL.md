# Legal: RapidR and the programs you build

This page says, in plain language, what you may do with RapidR and with the
programs you make with it, and what to ship with them. It describes the
licences of RapidR and of the open-source software inside it as the project
understands them; it is not legal advice. The details, output by output and
licence by licence, are in [docs/licensing.md](docs/licensing.md).

## In short

- **RapidR is free and open source**, under the [MIT License](LICENSE).
  Its copyright holder is Ruta Internet SRL (Uruguay), the company that develops it.
- **The programs you build with RapidR are yours.** Sell them, give them
  away, keep their source closed, use them in a business: no fee, no
  royalty, no registration, and no need to publish your source code.
- **What to ship:** your program, plus the `THIRD-PARTY-NOTICES.txt` that
  RapidR writes next to it every time you build. That's all.

RapidR's runtime and the open-source libraries it is built from go into your
program. Every one of them is under a permissive licence — MIT, Apache-2.0,
BSD, ISC, zlib, Boost, Unicode, public domain, and the SIL Open Font License
for the fonts — that allows commercial and closed-source use and asks only
that its copyright notice and licence text go with the program.
`THIRD-PARTY-NOTICES.txt` is exactly that, made from the build's real list
of components, with every licence text in full. Nothing in your program is
copyleft (no GPL, LGPL, MPL or similar), nothing asks you to publish source
code, to credit anyone in your documentation or advertising, or to do
anything else, and RapidR's checks fail if that ever changes.

**Encryption.** HTTPS (`RHttp`, `QDownload`) and encrypted MySQL/MariaDB
connections (`RMySQL.SSLMode`) use the operating system's
own TLS (macOS, Windows, and on Linux the system's OpenSSL 3, which every
current distribution installs): no TLS library and no AES code is compiled
into your program. A desktop program does contain a little standard
cryptography, from open-source libraries named in its notices, and nothing
else:

- **RSA public-key encryption** (OAEP padding) and **SHA-1 and SHA-256
  hashing**, in the MySQL client (`RMySQL`; the `mysql_common` crate). They
  are used only to send a password to a MySQL server that asks for it, over a
  connection that has neither TLS nor a local socket.
- **A Diffie-Hellman key exchange**, on Linux and the BSDs only, in the
  keyring client (the `secret-service` crate) that keeps the AI component's
  API keys and programs' `RSecrets` in the system's Secret Service (GNOME Keyring, KWallet). The AES
  encryption and the key derivation that follow it are the system's OpenSSL
  3, not compiled-in code. On macOS and Windows the keys go to the system's
  Keychain and Credential Manager, and no code of RapidR's is involved.
- **Hashing** (SHA-1, SHA-256, CRC-32), which is not encryption.

A web build contains none of these: the browser does the HTTPS.
[docs/licensing.md](docs/licensing.md) §2.5 says where each one comes from.

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
  notice (the notices file already does).
- **The built-in fonts** (Liberation Sans, Serif and Mono, and JetBrains
  Mono, the code editor's; on Linux also
  Cantarell, for window titles on GNOME's Wayland) are inside your program
  (the Regular faces unmodified; their Bold, Italic and Bold Italic cut to the
  Latin scripts and renamed "RapidR Text …", as the licence asks; JetBrains
  Mono's Italic as the Latin subset Google Fonts distributes), under the
  SIL Open Font License: fine for any program,
  commercial included. Don't extract them to sell on their own, and if you
  change them, give your version another name.
- **On Linux**, your program uses the system's OpenSSL 3 library
  (`libssl3`, present on every current distribution) for HTTPS and for the
  AES and key derivation of the keyring session that protects the AI
  component's keys; if you package it (e.g. as a `.deb`), list it as a
  dependency. Nothing of OpenSSL is inside your program.
- **Your toolchain on Windows.** With the `gnullvm` toolchain (LLVM-MinGW,
  open source) nothing is needed. With Microsoft's `msvc` toolchain, the
  Visual Studio Build Tools licence applies to you as the developer (it is
  free for individuals, open-source projects and small organisations;
  larger organisations need a Visual Studio licence). `rapidr setup` says
  which you have.
- **Laws that apply to software in general** (privacy, consumer law, export
  rules — your program uses the operating system's encryption for HTTPS and
  the few standard pieces listed under "Encryption" above — and so on) are
  outside what licences cover. RapidR's own export position is under
  "Export" below.

## RapidQ, and the names RapidR mentions

RapidR is an independent, original implementation, written from scratch in
Rust, of a BASIC language compatible with **RapidQ** (also written
"Rapid-Q"), the freeware compiler William Yu wrote in 1999–2000.

- **Nothing of RapidQ is in RapidR.** No code, no text of its manual, no
  include files, examples, images or icons, and RapidR doesn't include or
  distribute any part of RapidQ's distribution, its compiler `RC.EXE`
  included.
- **What RapidR does take is what compatibility needs**: the language's
  syntax and keywords, the names of its components, properties, methods,
  events and constants with their numbers, its file formats and its
  behaviour — the interface existing programs are written against, used
  only so that those programs run. RapidR's documentation describes them in
  its own words.
- **How compatibility is checked**: by running test programs, written for
  RapidR, under a locally installed copy of RapidQ and comparing their
  output, the compiler being used as a black box. The process is recorded
  in [docs/legal/clean-room.md](docs/legal/clean-room.md), and the review
  behind this section in
  [docs/legal/rapidq-review.md](docs/legal/rapidq-review.md).
- **Programs written for RapidQ belong to their authors**, as they always
  have; running them with RapidR changes nothing about that. RapidQ's own
  terms already let programs made with it be used and sold freely.

RapidR is **not affiliated with, endorsed by or sponsored by** William Yu,
by anyone who holds rights in RapidQ or distributes it, or by any company
named in RapidR's documentation. "RapidQ" is used only to say what RapidR
is compatible with ("compatible with RapidQ", "as in RapidQ"); RapidR never
uses it, or a logo of RapidQ, as its own name or mark.

The same goes for the other products named in the documentation. RapidR
implements, for compatibility, interfaces that RapidQ programs use: Windows
API constants, the names and values of Delphi VCL types, the DelphiX image
library format (`.DXG`) and DirectX's `.X` model format. It contains no
code, artwork or documentation of Microsoft, Embarcadero, DelphiX's author
or Xojo. Microsoft, Windows, Visual Basic, Visual Studio, DirectX and
Direct3D are trademarks of the Microsoft group of companies; Delphi is a
trademark of Embarcadero Technologies, Inc.; Xojo and REALbasic are
trademarks of Xojo, Inc.; Apple and macOS are trademarks of Apple Inc.;
Linux is a trademark of Linus Torvalds; other names belong to their owners.
They are named only to describe compatibility or to identify a platform,
never to suggest endorsement.

If you hold rights in something RapidR names and have a concern, please
open an issue (below) or write to the maintainer; it will be looked at
promptly.

## Trademarks

RapidR works with some other products and services, and names them so that
you can tell what it works with: Git™ repositories, the sites that host them,
and several databases. It writes their names in plain text, never uses their
logos, and never puts them inside a name of its own (RapidR's version control
is called Versions, and it is built on Git). Their owners' notices:

- Git and the Git logo are either registered trademarks or trademarks of
  Software Freedom Conservancy, Inc., corporate home of the Git Project, in
  the United States and/or other countries.
- GitHub® is a trademark of GitHub, Inc., registered in the United States
  and other countries.
- GitLab™: GITLAB is a trademark of GitLab Inc. in the United States and
  other countries and regions.
- Bitbucket is a trademark of Atlassian.
- SQLite is a trademark of Hipp, Wyrick & Company, Inc. (Hwaci).
- MySQL is a registered trademark of Oracle and/or its affiliates.
- The MariaDB® mark is a trademark of MariaDB Corporation Ab.
- Postgres, PostgreSQL and the Slonik Logo are trademarks or registered
  trademarks of the PostgreSQL Community Association of Canada, and used with
  their permission.

RapidR is not affiliated with or endorsed by these owners. Names are used
only to say what RapidR works with.

## Export

RapidR is published open-source software: its complete source code is public,
under the MIT licence. It uses only standard cryptography, and mostly the
operating system's own (TLS for HTTPS, the system's keychain); the few
pieces compiled into a program are listed under "Encryption" above. The
project's reading of the United States export rules is that publicly
available source code like this is not subject to the Export Administration
Regulations (15 CFR 734.3(b)(3)), and it has made no export filing. That is
a description of what RapidR contains and of how the project reads that
rule, not legal advice. If you ship a program built with RapidR where other
export or import rules apply, or add encryption of your own to it, checking
those rules is up to you.

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
