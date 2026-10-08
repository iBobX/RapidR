// A small ANSI/VT screen for program output: RapidQ's CLS, COLOR and LOCATE
// compile to the same escape sequences on every platform (see
// crates/rapidr-value/src/console.rs), and this renders them in the IDE.
//
// The element holds one inline <span> per row, separated by "\n" text nodes,
// so its textContent is exactly the text printed (plain PRINT output looks
// and copies as before). Printed text only ever becomes text nodes.

// VGA-style 16-color palette, ANSI order (black red green yellow blue
// magenta cyan white, then the bright variants).
const PALETTE = [
  "#000000", "#aa0000", "#00aa00", "#aa5500", "#0000aa", "#aa00aa", "#00aaaa", "#aaaaaa",
  "#555555", "#ff5555", "#55ff55", "#ffff55", "#5555ff", "#ff55ff", "#55ffff", "#ffffff",
];
const MAX_ROWS = 5000;

export class AnsiScreen {
  constructor(el) {
    this.el = el;
    this.clear();
  }

  clear() {
    this.rows = [[]];       // rows of cells { ch, fg, bg }
    this.row = 0;
    this.col = 0;
    this.fg = null;          // palette index or null (theme default)
    this.bg = null;
    this.pending = "";       // an escape sequence split across writes
    this.rowEls = [];
    this.dirty = new Set([0]);
    this.el.textContent = "";
  }

  write(text) {
    const s = this.pending + text;
    this.pending = "";
    for (let i = 0; i < s.length; i++) {
      const ch = s[i];
      if (ch === "\x1b") {
        if (i + 1 >= s.length) { this.pending = s.slice(i); break; }
        if (s[i + 1] !== "[") continue;
        let j = i + 2;
        while (j < s.length && !(s.charCodeAt(j) >= 0x40 && s.charCodeAt(j) <= 0x7e)) j++;
        if (j >= s.length) { this.pending = s.slice(i); break; }
        this.csi(s.slice(i + 2, j), s[j]);
        i = j;
      } else if (ch === "\n") {
        this.moveTo(this.row + 1, 0);
        this.ensureRow();
      } else if (ch === "\r") {
        this.col = 0;
      } else {
        this.put(ch);
      }
    }
    this.render();
  }

  // Moving the cursor doesn't create rows; writing does (so a cleared
  // screen doesn't keep empty rows down to where the cursor was).
  moveTo(row, col) {
    this.row = Math.max(0, row);
    this.col = Math.max(0, col);
  }

  ensureRow() {
    while (this.rows.length <= this.row) {
      this.rows.push([]);
      this.dirty.add(this.rows.length - 1);
    }
  }

  put(ch) {
    this.ensureRow();
    const cells = this.rows[this.row];
    while (cells.length < this.col) cells.push({ ch: " ", fg: null, bg: null });
    cells[this.col] = { ch, fg: this.fg, bg: this.bg };
    this.col++;
    this.dirty.add(this.row);
  }

  csi(params, final) {
    const nums = params.split(";").map((p) => (p === "" ? null : Number(p)));
    if (final === "H" || final === "f") {
      this.moveTo((nums[0] || 1) - 1, (nums[1] || 1) - 1);
    } else if (final === "J" && (nums[0] === 2 || nums[0] === 3)) {
      const [row, col, fg, bg] = [this.row, this.col, this.fg, this.bg];
      this.clear();
      [this.fg, this.bg] = [fg, bg];
      this.moveTo(row, col);
    } else if (final === "K") {
      this.ensureRow();
      this.rows[this.row].length = Math.min(this.rows[this.row].length, this.col);
      this.dirty.add(this.row);
    } else if (final === "m") {
      for (const n of nums.length ? nums : [0]) {
        if (n === null || n === 0) { this.fg = null; this.bg = null; }
        else if (n >= 30 && n <= 37) this.fg = n - 30;
        else if (n >= 90 && n <= 97) this.fg = n - 90 + 8;
        else if (n === 39) this.fg = null;
        else if (n >= 40 && n <= 47) this.bg = n - 40;
        else if (n >= 100 && n <= 107) this.bg = n - 100 + 8;
        else if (n === 49) this.bg = null;
      }
    }
  }

  render() {
    // Keep at most MAX_ROWS rows: drop the oldest and rebuild.
    if (this.rows.length > MAX_ROWS) {
      const drop = this.rows.length - MAX_ROWS;
      this.rows.splice(0, drop);
      this.row = Math.max(0, this.row - drop);
      this.rowEls = [];
      this.el.textContent = "";
      this.dirty = new Set(this.rows.keys());
    }
    for (let r = this.rowEls.length; r < this.rows.length; r++) {
      if (r > 0) this.el.appendChild(document.createTextNode("\n"));
      const span = document.createElement("span");
      this.el.appendChild(span);
      this.rowEls.push(span);
      this.dirty.add(r);
    }
    for (const r of this.dirty) {
      const span = this.rowEls[r];
      if (!span) continue;
      span.textContent = "";
      let run = null;
      for (const cell of this.rows[r]) {
        if (!run || run.fg !== cell.fg || run.bg !== cell.bg) {
          run = { fg: cell.fg, bg: cell.bg, text: "" };
          run.el = document.createElement("span");
          if (cell.fg !== null) run.el.style.color = PALETTE[cell.fg];
          if (cell.bg !== null) run.el.style.backgroundColor = PALETTE[cell.bg];
          span.appendChild(run.el);
        }
        run.el.textContent += cell.ch;
      }
    }
    this.dirty.clear();
    this.el.scrollTop = this.el.scrollHeight;
  }
}
