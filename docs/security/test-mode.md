# The signed test mode (SEC-24)

RapidR's GUI test hooks let a test drive a program and read it back:
scripted clicks and keys, captures of its windows, its fields printed,
dialog and message-box answers, the system's look, a crash on purpose,
Studio's MCP test token. They are compiled into the binaries RapidR ships,
because RapidR tests the binary it ships. Since Preview 5 no environment
variable alone turns them on in a released runtime or Studio: a release
build's hooks act only with a **test-mode token** signed by the key that
build trusts (audit finding SEC-24, [audit-preview4.md](audit-preview4.md)).

The gate is one small crate, `crates/rapidr-testmode`. Every hook reads its
variable through `rapidr_testmode::var` (or `var_os`, `env_var`,
`ENVIRON$` for the hooks' names). The process works out its answer once, the
first time a hook asks, and keeps it: one environment lookup when there is
no token, one file read and one Ed25519 signature check when there is.

## Which builds trust what

| Build | Made by | Test hooks |
|---|---|---|
| Debug (`cargo build`, `cargo test`) | developers | act, no token needed |
| Local release with `--features dev-hooks` (`cargo build --release -p rapidr-cli --features dev-hooks`) | developers, until the real key is installed | act, no token needed |
| **Release**: `tools/release/*.sh`, `tools/studio_app.sh` (release), the installers | what users get | act **only** with a token signed by the key in `crates/rapidr-testmode/test-mode.pub` |
| Programs built by `rapidr build` / `--interp` runners | the compiler | a development `rapidr` (debug or dev-hooks) builds development programs and runners; a released `rapidr` builds release ones |
| Web (the browser) | — | unchanged: the hooks come from the page's `window.RAPIDR_STUDIO_TEST`, which only the tests' pages set; a deployed program's page never does. There is no process environment and no token on the web. |
| Unit tests | `#[cfg(test)]` | fixture keys made inside the tests only; no build trusts them (`tests/security/test_mode_tokens.rs` checks) |

`dev-hooks` is a cargo feature of `rapidr-testmode`, passed through by
`rapidr-cli`, `rapidr-runtime-core` and `rapidr-runner-stub`. No crate turns
it on by default and no release script passes it
(`tests/security/test_mode_release.mjs` checks both).

`rapidr dev test-mode status` says which kind a binary is, the key it
trusts, and whether its hooks act now.

## The token

A short text file, never in a repository:

```
# RapidR test-mode token (docs/security/test-mode.md): keep it outside the repository.
rapidr-test-mode 1
purpose: rapidr-test-mode
key: 1a2b3c4d5e6f7a8b
issued: 2026-10-30
expires: 2027-01-28
machine: roberts-mac          (optional)
signature: <128 hex digits>
```

- **Signed**: Ed25519 over the claims in one fixed form (spacing, line ends
  and `#` comments don't matter; any changed claim breaks the signature).
- **Purpose**: `rapidr-test-mode`, nothing else.
- **Key**: the signing key's id (its first 8 bytes); a token for another key
  is refused with a clear reason.
- **Expiry**: valid through its `expires` day (UTC); at most 366 days ahead.
- **Machine** (optional): the computer's name. A guard against a token
  copied by mistake, not a lock.

`RAPIDR_TEST_MODE=<path>` names the token. The test harnesses set it
themselves to `~/.rapidr/test-mode.token` when that file exists
(`tests/test_mode.mjs`, `tools/regress.sh`); a process Studio starts
inherits it.

When the hooks are off and a hook variable is set, the first one seen says
so, once, on stderr:

```
RapidR: test hooks are off (no RAPIDR_TEST_MODE token): RAPIDR_CAPTURE and the other test variables are ignored.
RapidR: the test-mode token was refused (the token expired on 2027-01-28); the test hooks are off.
```

## For Robert: the key and the tokens

The repository holds a **placeholder** `test-mode.pub` (`public: none`): a
release build trusts no token until you install your key. Nobody but you
makes the key (it is a signing key). Run these from the RapidR checkout,
with a `rapidr` built from it:

1. **Make the key, once** (outside any repository; `new-key` refuses a folder
   inside one, and never overwrites a key):

   ```
   rapidr dev test-mode new-key --out ~/.rapidr/keys
   ```

   It writes `~/.rapidr/keys/test-mode.key` (the secret, readable by you
   only) and `~/.rapidr/keys/test-mode.pub`. Back the `.key` file up offline
   (a USB stick in a drawer). It never goes in Git, in a chat, or in a
   cloud folder.

2. **Install the public half** and rebuild:

   ```
   cp ~/.rapidr/keys/test-mode.pub crates/rapidr-testmode/test-mode.pub
   git commit -m "Test mode: install the test-mode public key" crates/rapidr-testmode/test-mode.pub
   cargo build --release -p rapidr-cli
   ```

   From then on, every release build trusts tokens signed by your key.

3. **Issue a token** (90 days is a good rhythm; a year at most):

   ```
   rapidr dev test-mode issue --key ~/.rapidr/keys/test-mode.key --days 90
   ```

   It writes `~/.rapidr/test-mode.token` (readable by you only), where the
   harnesses find it. Add `--this-machine` to tie it to this Mac; `--out
   FILE` for a token elsewhere (a test VM: copy it there and set
   `RAPIDR_TEST_MODE`).

4. **Where things live**: the secret key in `~/.rapidr/keys/` (and its
   backup); the token in `~/.rapidr/test-mode.token`; the public key in the
   repository. `rapidr dev test-mode status` shows what a binary trusts and
   whether the token is taken.

5. **Renew**: run step 3 again before the `expires` date (it overwrites the
   old token). An expired token turns the hooks off and says so.

6. **If the secret key is lost or leaks**: make a new key (step 1, a new
   folder), install its public half (step 2) and rebuild. Every token of the
   old key stops working with the next build.

Until step 2, test with a development build: `cargo build --release -p
rapidr-cli --features dev-hooks` (then copy it to `./rapidr` as
`tools/regress.sh` says), or a debug build.

## What the gate covers

Every hook below is ignored by a release build without a valid token.

| Variable | What it does | Where it's read |
|---|---|---|
| `RAPIDR_CAPTURE`, `RAPIDR_CAPTURE_DELAY`, `RAPIDR_CAPTURE_WINDOWS` | headless run, captures of the windows | `rapidr-ui-app` testhooks, runtime-core `ui/kernel.rs`, `studio.rs`, Studio's login PATH |
| `RAPIDR_TEST_EVENTS` | scripted clicks, keys, mouse | testhooks |
| `RAPIDR_TEST_DUMP` | `component.property` values printed | testhooks, `panels.rs` |
| `RAPIDR_TEST_RESIZE`, `RAPIDR_TEST_SPLIT`, `RAPIDR_TEST_NOFOCUS` | forms resized, splitters dragged, no focus | testhooks |
| `RAPIDR_TEST_SCREEN` | the headless host's screen a given size (a dialog checked on a 1280 × 720 screen) | testhooks |
| `RAPIDR_TEST_FRAMES`, `RAPIDR_TEST_FRAMES_OF` | each turn's place and size of the named components written to a file (the splitter-resize stability test) | `rapidr-ui-app` `desktop.rs` (`Desktop::trace_frame`) |
| `RAPIDR_TEST_FILE_DIALOG`, `RAPIDR_TEST_DROP`, `RAPIDR_TEST_COLOR_DIALOG`, `RAPIDR_TEST_FONT_DIALOG`, `RAPIDR_TEST_MESSAGE_DIALOG`, `RAPIDR_TEST_DIALOG_HOLD` | dialogs and message boxes answered, files dropped | testhooks |
| `RAPIDR_TEST_SYSTEM_LOOK` (`file:PATH` too) | the system's look; `file:` reads a file | testhooks |
| `RAPIDR_DUMP_MENUS` | the menus printed | testhooks (`menus.rs`) |
| `RAPIDR_TEST_A11Y`, `RAPIDR_TEST_CONTRAST` | the accessibility tree and contrast list written | runtime-core `ui/kernel.rs` |
| `RAPIDR_TEST_CRASH` | a crash at a line, on purpose | `rapidr-crash` `place.rs` |
| `RAPIDR_TEST_COMPORT`, `RAPIDR_TEST_JOYSTICK`, `RAPIDR_TEST_MIDI`, `RAPIDR_TEST_MIDI_PORTS`, `RAPIDR_TEST_MIDI_SECS`, `RAPIDR_TEST_WAVE_IN`, `RAPIDR_TEST_SOUND`, `RAPIDR_TEST_CLIPBOARD` | scripted devices, silent sound, a private clipboard | runtime-core `serial.rs`, `joystick.rs`, `media.rs`, `sound.rs`, `audio_out.rs`, `globals.rs`; `rapidr-value` `comport.rs` |
| `RAPIDR_TEST_PROGRAM_*` | Studio passes a program its hooks | runtime-core `studio.rs` (only while Studio's own hooks act) |
| `RAPIDR_TEST_MCP_TOKEN`, `RAPIDR_TEST_EXT` | Studio's MCP token stand-in; the external-changes test folder | Studio (`ide/mcp.inc`, `ide/external.inc`) through `ENVIRON$`, which answers "" for the hooks' names |
| `RAPIDR_SCHOOL_POLICY` | a test School Lock policy file | `rapidr-studio` `school_lock.rs` |
| `RAPIDR_LOGIN_SHELL` | a stand-in login shell for Studio's PATH | `rapidr-studio` `login_path.rs` |
| `RAPIDR_DEMO`, `RAPIDR_DEMO_OUT`, `RAPIDR_DEMO_LANG` | demo mode: a script played in Studio and recorded headless (input automation; docs/video-pipeline.md) | `rapidr-ui-app` `demo`; `ide/demo.inc` through `ENVIRON$` |
| `RAPIDR_AI_TEST_SERVER`, `RAPIDR_AI_TEST_DNS_FAIL_ONCE` | the AI provider on a loopback test server; a DNS failure | `rapidr-ai` `provider.rs`, `http.rs` |
| every other `RAPIDR_TEST_*` read with `ENVIRON$` | — | `rapidr-value` `environ.rs` |

`rapidr_testmode::is_hook` names them: every `RAPIDR_TEST_*` (but
`RAPIDR_TEST_MODE`), every `RAPIDR_CAPTURE*`, and the list above.
`tests/security/test_mode_release.mjs` fails when a shipped crate reads one
with `std::env::var` instead of through the gate.

**Not hooks, left as they are**: the run's settings that only narrow or
redirect what a run touches, or turn on diagnostics without user data —
`RAPIDR_REGISTRY`, `RAPIDR_PRINT_TO`, `RAPIDR_AI_KEYSTORE=memory`,
`RAPIDR_KEYSTORE`, `RAPIDR_BUILD_CACHE`, `RAPIDR_CRASH_DIR`, `RAPIDR_HOME`,
`RAPIDR_LANG`, `RAPIDR_THEME`,
`RAPIDR_SCALE`, `RAPIDR_RENDERER`, `RAPIDR_MENU`, `RAPIDR_SANDBOX`,
`RAPIDR_MIDI_OUT`, `RAPIDR_LOGIN_PATH=off`, `RAPIDR_HOST_LOG`,
`RAPIDR_STUDIO_PERF`. The tests rely on the first ones to keep the user's
registry, keychain and printer out of every run; turning them off would put
those at risk. Studio's `--do` list is a command-line option (Studio's own
commands, documented in docs/ide-plan.md); its MCP test steps need
`RAPIDR_TEST_MCP_TOKEN`, which the gate covers.

## Tests

- `cargo test -p rapidr-testmode` (`tools/regress.sh security`): tokens
  signed by a fixture key are taken by that key only; expired, wrong-purpose,
  too-long, garbled, tampered, other-key and other-machine tokens are refused;
  no build trusts the fixture key (`tests/security/test_mode_tokens.rs`).
- `RAPIDR_RELEASE=<release rapidr> node tests/security/test_mode_release.mjs
  [--gui]`: the source checks above; a release binary with every hook
  variable set, then with a fixture-signed, a tampered, a garbled and a
  missing token, runs a program as if none were set (no crash at
  `RAPIDR_TEST_CRASH`'s line, `ENVIRON$` empty for the hooks' names, one line
  on stderr); `--gui`: a windowed program gets no capture, no scripted click,
  no dump. `RAPIDR_DEV=<development rapidr>` adds the control: the same run
  with the hooks on.
