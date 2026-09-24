# Security Policy

RapidR compiles and runs code, hosts an in-browser IDE, and (soon) talks to AI providers on
your behalf, so we take security seriously. Thank you for helping keep RapidR and its users
safe.

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Report it privately through GitHub:
**[Report a vulnerability](https://github.com/iBobX/RapidR/security/advisories/new)**
(repository → *Security* tab → *Report a vulnerability*).

Please include:

- the affected component (compiler, bytecode VM, native runtime, web runtime, web IDE,
  build server, VS Code extension, …) and the version (`rapidr version`, or the version
  shown in the IDE title);
- steps to reproduce, ideally a minimal `.rr` program or project;
- the impact you expect (what an attacker could do, and who the attacker is: the author of a
  program, someone sending data to a running program, a website, another local user, …).

We will acknowledge your report, keep you updated as we investigate, and credit you in the
release notes if you want. RapidR is a small open-source project, so please allow reasonable
time for a fix before disclosing publicly.

## Supported versions

Security fixes are made on the latest release (currently the 2.9.x line on the
`development` branch, merged to `main`). Older versions are not patched.

## Scope

In scope, for example:

- a program run in the web IDE preview reaching the IDE itself (its storage, DOM, or
  messages), or escaping its sandbox;
- data processed by a RapidR program (user input, HTTP responses, database rows, AI output)
  causing script/HTML injection or code execution without the developer opting in;
- the compiler, preprocessor, or VM crashing, hanging, or misbehaving on crafted source or
  bytecode (`.rr`, `.rrbc`) in a way that affects safety;
- the build server being reachable or exploitable beyond the local machine;
- vulnerable dependencies that are actually reachable.

By design, and not vulnerabilities on their own:

- `RJavaScript.Eval`, `RDOM.InnerHTML`, `RWebView.HTML`, `RUSTSTART … RUSTEND`, and
  `DECLARE … LIB` run whatever code or markup the *program author* supplies;
- a RapidR program doing what its author wrote (for example deleting files it was told
  to delete).

Known issues and planned hardening are tracked in [ROADMAP.md](ROADMAP.md) (`SEC-xx`
entries) and fixes are listed in [CHANGELOG.md](CHANGELOG.md).
