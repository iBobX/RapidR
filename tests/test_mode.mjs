// The signed test mode (SEC-24, docs/security/test-mode.md): a release
// build's test hooks (RAPIDR_CAPTURE, RAPIDR_TEST_EVENTS, RAPIDR_TEST_DUMP,
// the dialogs' answers…) act only with a test-mode token signed by the key it
// trusts; a development build (debug, or `cargo build --features dev-hooks`)
// needs none.
//
// A harness imports this first (`import "./test_mode.mjs";`): when
// RAPIDR_TEST_MODE isn't set and ~/.rapidr/test-mode.token exists (made by
// `rapidr dev test-mode issue`), every process the harness starts gets it.
// The token lives outside the repository, never in it.

import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

export const TOKEN = join(homedir(), ".rapidr", "test-mode.token");

if (!process.env.RAPIDR_TEST_MODE && existsSync(TOKEN)) process.env.RAPIDR_TEST_MODE = TOKEN;
