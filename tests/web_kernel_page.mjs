// What a browser test reads and does on a page the UI kernel hosts
// (docs/web-host-plan.md): the program's windows are canvases with the
// kernel's accessibility mirror over them, so a test
//
//   * finds a component by its mirror element: `#rr-<name>` (lowercase; the
//     ids the DOM runtime gave its elements) with `data-rr-name`, the ARIA
//     role / name / states the kernel describes, a label's text, an edit's
//     value;
//   * acts as the user does: a real mouse click at the element's place (the
//     mirror takes no pointer events, the canvas under it gets the click),
//     real keys to the focused field;
//   * reads the program's state through the runtime (`rapidr_get_prop`) and
//     what is drawn through the window's canvas (its device pixels).
//
// Every function takes a Playwright Page or Frame (the IDE's preview).

/// The window elements shown, frontmost last: [{ form, title, modal }].
export function windows(where) {
  return where.evaluate(() => [...document.querySelectorAll(".rr-kwin")]
    .filter((w) => w.style.display !== "none")
    .sort((a, b) => (Number(a.style.zIndex) || 0) - (Number(b.style.zIndex) || 0))
    .map((w) => {
      const root = w.querySelector(".rr-a11y > *");
      return { form: w.dataset.rrForm, title: root?.getAttribute("aria-label") ?? "", modal: root?.getAttribute("aria-modal") === "true" };
    }));
}

/// Whether form `name`'s window is shown.
export function shown(where, name) {
  return where.evaluate((n) => {
    const w = [...document.querySelectorAll(".rr-kwin")].find((w) => w.dataset.rrForm.toLowerCase() === n.toLowerCase());
    return !!w && w.style.display !== "none";
  }, name);
}

/// Waits until component `name`'s element is in the mirror.
export async function waitFor(where, name, timeout = 15000) {
  await where.waitForFunction((n) => !!document.getElementById("rr-" + n.toLowerCase()), name, { timeout });
}

/// Component `name`'s text as the mirror has it: a label's caption, an
/// edit's / combo box's value, otherwise its accessible name (a button's
/// caption) — `null` if it isn't there.
export function text(where, name) {
  return where.evaluate((n) => {
    const el = document.getElementById("rr-" + n.toLowerCase());
    if (!el) return null;
    if (el.tagName === "INPUT" || el.tagName === "TEXTAREA") return el.value;
    if (!el.getAttribute("role") && el.children.length === 0) return el.textContent;
    return el.getAttribute("aria-label") ?? el.textContent;
  }, name);
}

/// The accessible names of the elements with ARIA role `role` inside
/// component `name` (the whole page when `name` is null).
export function roles(where, role, name = null) {
  return where.evaluate(({ role, name }) => {
    const root = name ? document.getElementById("rr-" + name.toLowerCase()) : document;
    return root ? [...root.querySelectorAll(`[role="${role}"]`)].map((e) => e.getAttribute("aria-label") ?? e.textContent) : [];
  }, { role, name });
}

/// Property `prop` of component `name`, as the program reads it.
export function prop(where, name, prop) {
  return where.evaluate(({ name, prop }) => {
    const rt = window.__rapidr_rt || window.rr;
    return rt.rapidr_get_prop(name, prop);
  }, { name, prop });
}

/// Component `name`'s rectangle on the page (CSS pixels), or null.
export function rect(where, name) {
  return where.evaluate((n) => {
    const el = document.getElementById("rr-" + n.toLowerCase());
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return { x: r.left, y: r.top, width: r.width, height: r.height };
  }, name);
}

/// A real click on component `name` (at `at` = [x, y] inside it, else its
/// middle), as the user's mouse does it.
export async function click(where, name, at = null, opts = {}) {
  const sel = "#rr-" + name.toLowerCase();
  await where.locator(sel).first().click({ force: true, ...(at ? { position: { x: at[0], y: at[1] } } : {}), ...opts });
}

/// A real click on the frontmost window's button captioned `caption` (a
/// message box's, a dialog's).
export async function clickButton(where, caption) {
  const handle = await where.evaluateHandle((c) => {
    const wins = [...document.querySelectorAll(".rr-kwin")].filter((w) => w.style.display !== "none")
      .sort((a, b) => (Number(b.style.zIndex) || 0) - (Number(a.style.zIndex) || 0));
    for (const w of wins) {
      const b = [...w.querySelectorAll('[role="button"]')].find((e) => (e.getAttribute("aria-label") ?? "").replace(/&/g, "") === c);
      if (b) return b;
    }
    return null;
  }, caption);
  const el = handle.asElement();
  if (!el) throw new Error(`no button '${caption}'`);
  await el.click({ force: true });
}

/// The frontmost window if it's a message box or another dialog the
/// kernel draws: { title, text, buttons, input } (`text`: its labels'
/// texts joined by newlines), else null.
export function dialog(where) {
  return where.evaluate(() => {
    const w = [...document.querySelectorAll(".rr-kwin")].filter((w) => w.style.display !== "none")
      .sort((a, b) => (Number(b.style.zIndex) || 0) - (Number(a.style.zIndex) || 0))[0];
    const root = w?.querySelector(".rr-a11y > *");
    if (!root || root.getAttribute("aria-modal") !== "true") return null;
    const labels = [...root.querySelectorAll("div:not([role])")].filter((e) => e.children.length === 0 && e.textContent);
    return {
      form: w.dataset.rrForm,
      title: root.getAttribute("aria-label") ?? "",
      text: labels.map((e) => e.textContent).join("\n"),
      buttons: [...root.querySelectorAll('[role="button"]')].map((e) => (e.getAttribute("aria-label") ?? "").replace(/&/g, "")),
      input: !!root.querySelector("input, textarea"),
    };
  });
}

/// The device pixels drawn at component `name`'s place (RGBA rows), with
/// the scale: { width, height, scale, data } — data an Array of bytes.
export function pixels(where, name) {
  return where.evaluate((n) => {
    const el = document.getElementById("rr-" + n.toLowerCase());
    const win = el?.closest(".rr-kwin");
    const canvas = win?.querySelector("canvas.rr-kclient");
    if (!canvas) return null;
    const c = canvas.getBoundingClientRect(), r = el.getBoundingClientRect();
    const scale = canvas.width / c.width;
    const x = Math.round((r.left - c.left) * scale), y = Math.round((r.top - c.top) * scale);
    const w = Math.round(r.width * scale), h = Math.round(r.height * scale);
    const d = canvas.getContext("2d").getImageData(x, y, w, h).data;
    return { width: w, height: h, scale, data: Array.from(d) };
  }, name);
}

/// The colour [r, g, b] at (x, y) inside component `name` (CSS pixels).
export async function pixelAt(where, name, x, y) {
  const p = await pixels(where, name);
  if (!p) return null;
  const i = (Math.floor(y * p.scale) * p.width + Math.floor(x * p.scale)) * 4;
  return [p.data[i], p.data[i + 1], p.data[i + 2]];
}
