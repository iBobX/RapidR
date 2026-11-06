// RMIDIPort on the web's real path (crates/rapidr-runtime-web's
// media_web/port.rs), with the browser's MIDI interface played by a mock
// `navigator.requestMIDIAccess` (an input and an output that loop back, a
// keyboard): no test hooks on this page, the program's own calls, real
// clicks on RapidR's explanation and the program's buttons. Checks the
// legal review's C2 / C3 (docs/legal/review-2026-11-05-midi.md):
//
//   1. nothing is asked when the page loads or the RMIDIPort is made;
//   2. the first device call shows RapidR's explanation first, and only
//      "Continue" calls requestMIDIAccess — with no options (no SysEx);
//   3. notes go out and come back, on the program's events;
//   4. a program that only plays notes never asks for SysEx;
//   5. SysEx is asked separately (a second explanation, then {sysex:
//      true}) and a 4 KB SysEx goes out and comes back whole;
//   6. a refused SysEx is a plain Error, notes still work;
//   7. "Not now": a plain Error, nothing asked; OpenInput asks again;
//   8. a browser without MIDI: a plain Error, no explanation shown;
//   9. a device plugged in (statechange): OnDevicesChanged.
//
// Usage (repo root, after tools/build_web_artifacts.sh, with the repo served
// on RAPIDR_URL, default http://localhost:8765): node tests/web_midi_port.mjs

import { chromium } from "playwright";
import * as k from "./web_kernel_page.mjs";

const URL_BASE = process.env.RAPIDR_URL || "http://localhost:8765";
let failed = 0;
const ok = (cond, msg) => { console.log(`${cond ? "✓" : "✗"} ${msg}`); if (!cond) failed++; };

const SOURCE = `
DIM M AS RMIDIPort
DIM notes AS INTEGER
SUB NoteOn (Channel AS INTEGER, Note AS INTEGER, Velocity AS INTEGER)
    notes = notes + 1
    Got.Caption = "note " + STR$(Note) + " ch " + STR$(Channel) + " x " + STR$(notes)
END SUB
SUB SysEx (Data AS STRING)
    SysGot.Caption = "sysex " + STR$(LEN(Data)) + " " + M.Hex(LEFT$(Data, 3)) + " " + M.Hex(RIGHT$(Data, 1))
END SUB
SUB Changed
    Devs.Caption = "changed " + STR$(M.InputCount)
END SUB
SUB OpenClick
    Info.Caption = "in " + STR$(M.InputCount) + " out " + STR$(M.OutputCount) + " [" + M.Error + "]"
    IF M.OpenInput("Loop") AND M.OpenOutput("Loop") THEN Info.Caption = Info.Caption + " open"
END SUB
SUB NoteClick
    M.SendNoteOn 3, 64, 90
END SUB
SUB SysExClick
    DIM s AS STRING, i AS INTEGER
    M.IgnoreSysEx = 0
    s = CHR$(240) + CHR$(125)
    FOR i = 1 TO 4096
        s = s + CHR$(i MOD 128)
    NEXT
    s = s + CHR$(247)
    Info.Caption = "sent " + STR$(M.SendSysEx(s)) + " [" + M.Error + "]"
    M.SendNoteOn 1, 60, 1
END SUB
M.OnNoteOn = NoteOn
M.OnSysEx = SysEx
M.OnDevicesChanged = Changed
CREATE Form AS RForm
    Caption = "midi"
    ClientWidth = 420: ClientHeight = 170 + 30
    CREATE OpenBtn AS RButton
        Caption = "Open": Left = 10: Top = 10: Width = 90: Height = 30
        OnClick = OpenClick
    END CREATE
    CREATE NoteBtn AS RButton
        Caption = "Note": Left = 110: Top = 10: Width = 90: Height = 30
        OnClick = NoteClick
    END CREATE
    CREATE SysExBtn AS RButton
        Caption = "SysEx": Left = 210: Top = 10: Width = 90: Height = 30
        OnClick = SysExClick
    END CREATE
    CREATE Info AS RLabel
        Left = 10: Top = 50: Width = 400
    END CREATE
    CREATE Got AS RLabel
        Left = 10: Top = 80: Width = 400
    END CREATE
    CREATE Devs AS RLabel
        Left = 10: Top = 110: Width = 400
    END CREATE
    CREATE SysGot AS RLabel
        Left = 10: Top = 135: Width = 400
    END CREATE
END CREATE
Form.ShowModal
`;

// The browser's MIDI interface, played: `opts` says what the page refuses.
function mock(opts) {
  window.__midi = { requests: [], sent: [], opts };
  if (opts.none) {
    delete Navigator.prototype.requestMIDIAccess;
    return;
  }
  const port = (id, name, type) => ({ id, name, manufacturer: "RapidR test", type, state: "connected", connection: "closed", onmidimessage: null,
    open() { this.connection = "open"; return Promise.resolve(this); }, close() { this.connection = "closed"; return Promise.resolve(this); } });
  const make = (sysex) => {
    const loopIn = port("in-loop", "Loop", "input");
    const keys = port("in-keys", "Keys", "input");
    const loopOut = port("out-loop", "Loop", "output");
    loopOut.send = (data) => {
      const bytes = Array.from(data);
      if (bytes[0] === 0xF0 && !sysex) throw new DOMException("SysEx not allowed", "InvalidAccessError");
      window.__midi.sent.push(bytes);
      // (what comes back arrives later, as from a cable; a SysEx masked
      // without the permission, as browsers do)
      setTimeout(() => {
        for (const p of window.__midi.inputs()) {
          if (p.name === "Loop" && p.onmidimessage) p.onmidimessage({ data: new Uint8Array(bytes), timeStamp: performance.now() });
        }
      }, 5);
    };
    return { inputs: new Map([["in-loop", loopIn], ["in-keys", keys]]), outputs: new Map([["out-loop", loopOut]]), sysexEnabled: sysex, onstatechange: null };
  };
  const accesses = [];
  window.__midi.inputs = () => accesses.flatMap((a) => [...a.inputs.values()]);
  window.__midi.plug = () => {
    for (const a of accesses) {
      a.inputs.set("in-new", port("in-new", "New Pad", "input"));
      if (a.onstatechange) a.onstatechange({ port: a.inputs.get("in-new") });
    }
  };
  navigator.requestMIDIAccess = function (o) {
    window.__midi.requests.push(o === undefined ? "none" : JSON.stringify(o));
    const sysex = !!(o && o.sysex);
    if (window.__midi.opts.refuseAll || (sysex && window.__midi.opts.refuseSysex)) return Promise.reject(new DOMException("denied", "NotAllowedError"));
    const a = make(sysex);
    accesses.push(a);
    return Promise.resolve(a);
  };
}

const browser = await chromium.launch();
const errors = [];

async function open(opts) {
  const page = await browser.newPage();
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("dialog", (d) => { errors.push(`a page dialog: ${d.message()}`); d.dismiss(); });
  await page.addInitScript(mock, opts);
  await page.goto(`${URL_BASE}/${process.env.RAPIDR_KERNEL_PAGE || "tests/web_kernel.html"}`, { waitUntil: "load" });
  await page.waitForFunction(() => window.rrReady, null, { timeout: 15000 });
  await page.evaluate((src) => window.rr.rapidr_run_bc(window.rr.compile(src, "midi", {})), SOURCE);
  await k.waitFor(page, "OpenBtn");
  return page;
}
const requests = (page) => page.evaluate(() => window.__midi.requests);
const caption = async (page, name) => (await k.prop(page, name, "Caption")).trim();
async function waitCaption(page, name, re, ms = 4000) {
  const t = Date.now();
  while (Date.now() - t < ms) {
    const c = await caption(page, name);
    if (re.test(c)) return c;
    await page.waitForTimeout(50);
  }
  return caption(page, name);
}
async function waitDialog(page, ms = 4000) {
  const t = Date.now();
  while (Date.now() - t < ms) {
    const d = await k.dialog(page);
    // (the program's own form is modal too: a dialog over it)
    if (d && d.title !== "midi") return d;
    await page.waitForTimeout(50);
  }
  return null;
}

// ---- 1-5: allowed ---------------------------------------------------------------------
{
  const page = await open({});
  await page.waitForTimeout(400);
  ok((await requests(page)).length === 0, "nothing asked when the page loads and the RMIDIPort is made");
  await k.click(page, "OpenBtn");
  const d = await waitDialog(page);
  ok(d && /MIDI devices/.test(d.title) && /RapidR does not send the list of your devices anywhere/.test(d.text), `RapidR's explanation first: "${d?.title}" — ${d?.text?.slice(0, 70)}…`);
  ok(d && d.buttons.join(",") === "Continue,Not now", `its buttons: ${d?.buttons}`);
  ok((await requests(page)).length === 0, "the browser isn't asked before Continue");
  await k.clickButton(page, "Continue");
  const info = await waitCaption(page, "Info", /open/);
  ok(info === "in 2 out 1 [] open", `after Continue the program goes on with the devices: "${info}"`);
  ok(JSON.stringify(await requests(page)) === '["none"]', `requestMIDIAccess() with no options: ${JSON.stringify(await requests(page))}`);
  await k.click(page, "NoteBtn");
  const got = await waitCaption(page, "Got", /note/);
  ok(got === "note 64 ch 3 x 1", `a note goes out and comes back on OnNoteOn: "${got}"`);
  ok(JSON.stringify(await requests(page)) === '["none"]', "a program that only plays notes never asks for SysEx");
  await k.click(page, "SysExBtn");
  const d2 = await waitDialog(page);
  ok(d2 && /SysEx/.test(d2.title) && /replace its firmware/.test(d2.text), `SysEx: a second explanation: "${d2?.title}" — ${d2?.text?.slice(0, 60)}…`);
  await k.clickButton(page, "Continue");
  const sent = await waitCaption(page, "Info", /sent/);
  ok(sent === "sent 1 []", `then the SysEx goes: "${sent}"`);
  ok(JSON.stringify(await requests(page)) === '["none","{\\"sysex\\":true}"]', `asked separately with {sysex: true}: ${JSON.stringify(await requests(page))}`);
  const back = await waitCaption(page, "SysGot", /sysex/);
  ok(back === "sysex 4099 F0 7D 01 F7", `a 4 KB SysEx came back whole on OnSysEx (the input moved to the new access): "${back}"`);
  await page.evaluate(() => window.__midi.plug());
  const ch = await waitCaption(page, "Devs", /changed/);
  ok(ch === "changed 3", `a device plugged in: OnDevicesChanged ("${ch}")`);
  await page.close();
}

// ---- 6: SysEx refused ---------------------------------------------------------------------
{
  const page = await open({ refuseSysex: true });
  await k.click(page, "OpenBtn");
  await waitDialog(page);
  await k.clickButton(page, "Continue");
  await waitCaption(page, "Info", /open/);
  await k.click(page, "SysExBtn");
  await waitDialog(page);
  await k.clickButton(page, "Continue");
  const sent = await waitCaption(page, "Info", /sent/);
  ok(sent === "sent 0 [SysEx messages were not allowed in this browser (notes and controls still work)]", `SysEx refused: a plain Error ("${sent}")`);
  const got = await waitCaption(page, "Got", /note/);
  ok(/^note 60 ch 1/.test(got), `notes still work after the refusal: "${got}"`);
  await page.close();
}

// ---- 7: Not now -----------------------------------------------------------------------
{
  const page = await open({});
  await k.click(page, "OpenBtn");
  await waitDialog(page);
  await k.clickButton(page, "Not now");
  const info = await waitCaption(page, "Info", /\[/);
  ok(/^in 0 out 0 \[MIDI wasn't turned on/.test(info), `"Not now": nothing asked, a plain Error: "${info}"`);
  ok((await requests(page)).length === 0, "the browser wasn't asked");
  await k.click(page, "OpenBtn");
  const again = await waitDialog(page);
  ok(!!again, "OpenInput asks again after \"Not now\"");
  await k.clickButton(page, "Not now");
  await page.close();
}

// ---- 8: no MIDI in this browser -----------------------------------------------------------
{
  const page = await open({ none: true });
  await k.click(page, "OpenBtn");
  const info = await waitCaption(page, "Info", /\[/);
  ok(/^in 0 out 0 \[This browser has no MIDI support/.test(info), `a browser without MIDI: a plain Error, no explanation shown: "${info}"`);
  const dlg = await k.dialog(page);
  ok(!dlg || dlg.title === "midi", `(no dialog over the program: ${dlg?.title})`);
  await page.close();
}

ok(errors.length === 0, `no page errors (${errors.join(" / ")})`);
await browser.close();
if (failed) { console.log(`\nWeb MIDI port: ${failed} CHECK(S) FAILED`); process.exit(1); }
console.log("\nWeb MIDI port: ALL CHECKS PASSED");
