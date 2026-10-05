#!/usr/bin/env python3
"""Writes the AVI files the video decoder's tests read (QVIDEO,
crates/rapidr-value/src/objects/avi.rs) into tests/fixtures/video/:
32x24 pixels, 8 frames at 10 fps (800 ms), one file per codec, each with
encoders of RapidR's own (no Pillow, no numpy, no ffmpeg):

    dib1.avi      uncompressed 1-bit, black and white, idx1 relative
    dib4.avi      uncompressed 4-bit, 16-entry palette, idx1 relative
    dib8.avi      uncompressed 8-bit, 256-entry palette, idx1 relative
    dib16.avi     uncompressed 16-bit X1R5G5B5, idx1 absolute, frame 5 an
                  empty (drop) chunk: it shows frame 4 again
    dib16bf.avi   BI_BITFIELDS 16-bit 5-6-5, idx1 relative
    dib24.avi     uncompressed 24-bit, no idx1 (the reader scans movi)
    dib32.avi     uncompressed 32-bit, top-down (negative biHeight)
    rle8.avi      BI_RLE8: key frames 0 and 4, delta frames between, a
                  ##pc palette change before frame 6 (entry 1)
    rle4.avi      BI_RLE4: key frame 0, delta frames after
    cram8.avi     Microsoft Video 1, 8-bit (palettized)
    cram16.avi    Microsoft Video 1, 16-bit RGB555
    cinepak.avi   Cinepak: two strips, V1/V4 codebooks (full, selective,
                  greyscale), inter frames with skips, key frames 0 and 4
    mjpeg.avi     Motion JPEG, YCbCr 4:2:2, quality 95, no DHT segment
                  (the standard tables are implied, as MJPEG has them)
    audio.avi     8-bit video + PCM 8-bit mono 11025 Hz (a 441 Hz square
                  wave), interleaved in LIST rec groups

Every frame draws the same pattern (avi.rs' tests compute it again): 4x4
blocks of eight colors (each exactly what Cinepak's YUV gives), a 2x2
square of blocks moving one block a frame (frames 0-4, then still), a
block that changes color every frame, and a corner block with 1-pixel
detail. Same bytes every run.

The conformance case media_video takes copies of four of them
(tests/conformance/cases/video_files/video_*.avi), written too.

Usage (repo root): python3 tools/make_avi_fixtures.py
"""
import math
import os
import shutil
import struct

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "tests", "fixtures", "video")

W, H, FRAMES, FPS = 32, 24, 8, 10

# The colors, as Cinepak's (Y, U, V) entries: R = Y + 2V, G = Y - U/2 - V,
# B = Y + 2U (U/2 truncated toward zero).
YUV = [(16, 0, 0), (200, 0, 0), (100, -40, 60), (120, 50, -30),
       (140, -30, -20), (180, 30, 30), (60, 20, -10), (220, -10, 10)]


def yuv_rgb(y, u, v):
    return (y + 2 * v, y - int(u / 2) - v, y + 2 * u)


PALETTE = [yuv_rgb(*t) for t in YUV]
# rle8.avi's palette change: entry 1 from frame 6 on.
PC_FRAME, PC_ENTRY, PC_COLOR = 6, 1, (250, 250, 10)


def index_at(f, x, y):
    """The pattern: color index of pixel (x, y) (top row 0) in frame f."""
    bx, by = x // 4, y // 4
    sx = min(f, 4)
    if bx == 0 and by == 0:
        # (each 2x2's top-right pixel light: Cinepak's Y order, CRAM's bits)
        return 1 if (x % 2 == 1 and y % 2 == 0) else 0
    if sx <= bx <= sx + 1 and 1 <= by <= 2:
        # (the moving square; its top-left 2x2 pixels another color)
        return 7 if (bx == sx and by == 1 and x % 4 < 2 and y % 4 < 2) else 2
    if bx == 7 and by == 5:
        return (3 + f) % 8
    return (0, 6, 1)[(bx + by) % 3]


def frame_rows(f):
    return [[index_at(f, x, y) for x in range(W)] for y in range(H)]


def fourcc_int(s):
    return struct.unpack("<I", s)[0]


# --- RIFF ---------------------------------------------------------------

def chunk(fourcc, data):
    out = fourcc + struct.pack("<I", len(data)) + data
    return out + b"\0" if len(data) % 2 else out


def bih(bits, compression, size_image, clr_used=0, height=H, extra=b""):
    if isinstance(compression, bytes):
        compression = fourcc_int(compression)
    return struct.pack("<IiiHHIIiiII", 40, W, height, 1, bits, compression, size_image,
                       0, 0, clr_used, 0) + extra


def palette_bytes(colors, count):
    out = b""
    for i in range(count):
        r, g, b = colors[i] if i < len(colors) else (0, 0, 0)
        out += bytes([b, g, r, 0])
    return out


def write_avi(name, handler, strf, frames, audio=None, index="rel", rec=False,
              palchanges=False):
    """frames: (kind, data, key) per chunk in order — kind "dc"/"db" (video,
    one a frame) or "pc" (a palette change before the next frame); audio:
    the PCM bytes that go with each video frame (8-bit mono 11025 Hz)."""
    nframes = sum(1 for k, _, _ in frames if k != "pc")
    biggest = max(len(d) for _, d, _ in frames)
    streams = 2 if audio else 1
    avih = struct.pack("<10I", 1000000 // FPS, biggest * FPS, 0,
                       (0x10 if index else 0) | (0x100 if audio else 0),
                       nframes, 0, streams, biggest, W, H) + b"\0" * 16
    strh = struct.pack("<4s4sIHHIIIIIIIIhhhh", b"vids", handler,
                       0x10000 if palchanges else 0, 0, 0, 0, 1, FPS, 0, nframes,
                       biggest, 0xFFFFFFFF, 0, 0, 0, W, H)
    hdrl = chunk(b"avih", avih) + chunk(b"LIST", b"strl" + chunk(b"strh", strh) + chunk(b"strf", strf))
    if audio:
        samples = sum(len(a) for a in audio)
        astrh = struct.pack("<4s4sIHHIIIIIIIIhhhh", b"auds", b"\0\0\0\0", 0, 0, 0, 0, 1, 11025, 0,
                            samples, max(len(a) for a in audio), 0xFFFFFFFF, 1, 0, 0, 0, 0)
        astrf = struct.pack("<HHIIHHH", 1, 1, 11025, 11025, 1, 8, 0)
        hdrl += chunk(b"LIST", b"strl" + chunk(b"strh", astrh) + chunk(b"strf", astrf))
    hdrl_list = chunk(b"LIST", b"hdrl" + hdrl)
    movi_fourcc_at = 12 + len(hdrl_list) + 8
    # movi's contents, and the index (offsets from the 'movi' fourcc)
    movi = bytearray()
    entries = []

    def put(ckid, data, flags):
        entries.append((ckid, flags, 4 + len(movi), len(data)))
        movi.extend(chunk(ckid, data))

    vi = 0
    for kind, data, key in frames:
        ckid = b"00" + kind.encode()
        flags = 0x10 if key else 0
        if kind == "pc":
            put(ckid, data, 0x100)  # (AVIIF_NO_TIME)
            continue
        if rec:
            group = chunk(b"01wb", audio[vi]) + chunk(ckid, data)
            entries.append((b"rec ", 0x01, 4 + len(movi), len(group) + 4))
            inner = 4 + len(movi) + 12
            entries.append((b"01wb", 0x10, inner, len(audio[vi])))
            entries.append((ckid, flags, inner + len(chunk(b"01wb", audio[vi])), len(data)))
            movi.extend(chunk(b"LIST", b"rec " + group))
        else:
            if audio:
                put(b"01wb", audio[vi], 0x10)
            put(ckid, data, flags)
        vi += 1
    body = b"AVI " + hdrl_list + chunk(b"LIST", b"movi" + bytes(movi))
    if index:
        base = movi_fourcc_at if index == "abs" else 0
        body += chunk(b"idx1", b"".join(struct.pack("<4sIII", c, fl, off + base, size)
                                         for c, fl, off, size in entries))
    data = b"RIFF" + struct.pack("<I", len(body)) + body
    path = os.path.join(OUT, name)
    with open(path, "wb") as f:
        f.write(data)
    print(f"wrote {os.path.relpath(path)} ({len(data)} bytes)")


# --- uncompressed DIB ---------------------------------------------------

def dib_frame(f, bits, top_down=False, color=None):
    rows = frame_rows(f)
    out = bytearray()
    for row in (rows if top_down else rows[::-1]):
        line = bytearray()
        if bits == 1:
            for x in range(0, W, 8):
                byte = 0
                for i in range(8):
                    byte |= (1 if row[x + i] == 1 else 0) << (7 - i)
                line.append(byte)
        elif bits == 4:
            for x in range(0, W, 2):
                line.append(row[x] << 4 | row[x + 1])
        elif bits == 8:
            line.extend(row)
        else:
            for i in row:
                line.extend(color(PALETTE[i]))
        while len(line) % 4:
            line.append(0)
        out += line
    return bytes(out)


def x1r5g5b5(c):
    r, g, b = c
    return struct.pack("<H", (r >> 3) << 10 | (g >> 3) << 5 | b >> 3)


def r5g6b5(c):
    r, g, b = c
    return struct.pack("<H", (r >> 3) << 11 | (g >> 2) << 5 | b >> 3)


def write_dibs():
    # 1-bit: index 1 white, the rest black
    frames = [("db", dib_frame(f, 1), True) for f in range(FRAMES)]
    write_avi("dib1.avi", b"DIB ", bih(1, 0, len(frames[0][1]), extra=palette_bytes([(0, 0, 0), (255, 255, 255)], 2)), frames)
    frames = [("db", dib_frame(f, 4), True) for f in range(FRAMES)]
    write_avi("dib4.avi", b"\0\0\0\0", bih(4, 0, len(frames[0][1]), extra=palette_bytes(PALETTE, 16)), frames)
    frames = [("db", dib_frame(f, 8), True) for f in range(FRAMES)]
    write_avi("dib8.avi", b"DIB ", bih(8, 0, len(frames[0][1]), extra=palette_bytes(PALETTE, 256)), frames)
    frames = [("db", dib_frame(f, 16, color=x1r5g5b5) if f != 5 else b"", f != 5) for f in range(FRAMES)]
    write_avi("dib16.avi", b"DIB ", bih(16, 0, len(frames[0][1])), frames, index="abs")
    masks = struct.pack("<III", 0xF800, 0x07E0, 0x001F)
    frames = [("db", dib_frame(f, 16, color=r5g6b5), True) for f in range(FRAMES)]
    write_avi("dib16bf.avi", b"DIB ", bih(16, 3, len(frames[0][1]), extra=masks), frames)
    frames = [("db", dib_frame(f, 24, color=lambda c: bytes(c[::-1])), True) for f in range(FRAMES)]
    write_avi("dib24.avi", b"RGB ", bih(24, 0, len(frames[0][1])), frames, index=None)
    frames = [("db", dib_frame(f, 32, top_down=True, color=lambda c: bytes(c[::-1]) + b"\0"), True) for f in range(FRAMES)]
    write_avi("dib32.avi", b"DIB ", bih(32, 0, len(frames[0][1]), height=-H), frames)


# --- Microsoft RLE ------------------------------------------------------

def runs(pixels):
    out = []
    for p in pixels:
        if out and out[-1][0] == p:
            out[-1][1] += 1
        else:
            out.append([p, 1])
    return out


def rle_pixels(pixels, four):
    """Encoded runs and absolute (literal) runs for a stretch of a line."""
    out = bytearray()
    lit = []

    def flush():
        if len(lit) >= 3:
            out.extend([0, len(lit)])
            if four:
                packed = bytes((lit[i] << 4 | (lit[i + 1] if i + 1 < len(lit) else 0)) for i in range(0, len(lit), 2))
            else:
                packed = bytes(lit)
            out.extend(packed)
            if len(packed) % 2:
                out.append(0)
        else:
            for p in lit:
                out.extend([1, p << 4 | p if four else p])
        lit.clear()

    for v, n in runs(pixels):
        if n == 1:
            lit.append(v)
            continue
        flush()
        out.extend([n, v << 4 | v if four else v])
    flush()
    return out


def rle_frame(f, prev, four):
    rows = frame_rows(f)
    out = bytearray()
    if prev is None:
        for line in range(H):
            out += rle_pixels(rows[H - 1 - line], four)
            out += b"\0\1" if line == H - 1 else b"\0\0"
        return bytes(out)
    old = frame_rows(prev)
    cx, cl = 0, 0
    for line in range(H):
        cur, was = rows[H - 1 - line], old[H - 1 - line]
        x = 0
        while x < W:
            if cur[x] == was[x]:
                x += 1
                continue
            x0 = x
            while x < W and cur[x] != was[x]:
                x += 1
            # (move there: delta right/up, or end of line then delta)
            if line == cl and x0 >= cx:
                if x0 > cx:
                    out += bytes([0, 2, x0 - cx, 0])
            elif x0 >= cx:
                out += bytes([0, 2, x0 - cx, line - cl])
            else:
                out += b"\0\0"
                if line > cl + 1 or x0 > 0:
                    out += bytes([0, 2, x0, line - cl - 1])
            out += rle_pixels(cur[x0:x], four)
            cx, cl = x, line
    out += b"\0\1"
    return bytes(out)


def write_rles():
    frames = []
    for f in range(FRAMES):
        if f == PC_FRAME:
            r, g, b = PC_COLOR
            frames.append(("pc", bytes([PC_ENTRY, 1, 0, 0, r, g, b, 0]), False))
        key = f in (0, 4)
        frames.append(("dc", rle_frame(f, None if key else f - 1, False), key))
    size = max(len(d) for _, d, _ in frames)
    write_avi("rle8.avi", b"mrle", bih(8, 1, size, clr_used=8, extra=palette_bytes(PALETTE, 8)), frames, palchanges=True)
    frames = [("dc", rle_frame(f, None if f == 0 else f - 1, True), f == 0) for f in range(FRAMES)]
    size = max(len(d) for _, d, _ in frames)
    write_avi("rle4.avi", b"mrle", bih(4, 2, size, clr_used=8, extra=palette_bytes(PALETTE, 8)), frames)


# --- Microsoft Video 1 (CRAM) -------------------------------------------

def cram_frame(f, prev, eight):
    rows = frame_rows(f)
    old = frame_rows(prev) if prev is not None else None
    color = (lambda i: i) if eight else (lambda i: (PALETTE[i][0] >> 3) << 10 | (PALETTE[i][1] >> 3) << 5 | PALETTE[i][2] >> 3)
    out = bytearray()
    skip = 0

    def flush_skip():
        nonlocal skip
        while skip:
            n = min(skip, 0x3FF)
            out.extend([n & 0xFF, 0x84 + (n >> 8)])
            skip -= n

    # (blocks bottom-up, left to right; a block's pixels bottom row first,
    # flag bit y*4 + x)
    for brow in range(H // 4):
        top = H - 4 * (brow + 1)
        for bx in range(W // 4):
            px = [[rows[top + 3 - y][bx * 4 + x] for x in range(4)] for y in range(4)]
            if old is not None and px == [[old[top + 3 - y][bx * 4 + x] for x in range(4)] for y in range(4)]:
                skip += 1
                continue
            flush_skip()
            cols = sorted({c for r in px for c in r})
            moving_tl = bx == min(f, 4) and top == 4
            if len(cols) == 1 and not moving_tl:
                c = color(cols[0])
                out.extend([c, 0x80] if eight else struct.pack("<H", 0x8000 | c))
            elif len(cols) == 2 and not moving_tl:
                c1 = px[3][3]  # (its bit is 0: keeps the flags' top bit clear)
                c0 = cols[0] if cols[1] == c1 else cols[1]
                flags = sum(1 << (y * 4 + x) for y in range(4) for x in range(4) if px[y][x] == c0)
                if eight:
                    out.extend([flags & 0xFF, flags >> 8, color(c0), color(c1)])
                else:
                    out.extend(struct.pack("<HHH", flags, color(c0), color(c1)))
            else:
                # 8 colors: a pair per quadrant (bottom-left, bottom-right,
                # top-left, top-right)
                pairs, flags = [], 0
                for qy, qx in ((0, 0), (0, 2), (2, 0), (2, 2)):
                    q = [(y, x) for y in (qy, qy + 1) for x in (qx, qx + 1)]
                    if eight:
                        # (8-bit: flags' top byte must be >= 0x90: the top
                        # row's x=0 and x=3 pixels take bit 1)
                        ca = px[qy + 1][qx + 1] if (qy, qx) == (2, 2) else px[qy + 1][qx] if (qy, qx) == (2, 0) else px[qy][qx]
                        cb = next((px[y][x] for y, x in q if px[y][x] != ca), ca)
                        for y, x in q:
                            if px[y][x] == ca:
                                flags |= 1 << (y * 4 + x)
                    else:
                        # (16-bit: the top bit must be 0)
                        cb = px[qy + 1][qx + 1] if (qy, qx) == (2, 2) else px[qy][qx]
                        ca = next((px[y][x] for y, x in q if px[y][x] != cb), cb)
                        for y, x in q:
                            if px[y][x] != cb:
                                flags |= 1 << (y * 4 + x)
                    pairs += [color(ca), color(cb)]
                if eight:
                    out.extend([flags & 0xFF, flags >> 8] + pairs)
                else:
                    pairs[0] |= 0x8000
                    out.extend(struct.pack("<H8H", flags, *pairs))
    flush_skip()
    return bytes(out)


def write_crams():
    frames = [("dc", cram_frame(f, None if f == 0 else f - 1, True), f == 0) for f in range(FRAMES)]
    size = max(len(d) for _, d, _ in frames)
    write_avi("cram8.avi", b"CRAM", bih(8, b"CRAM", size, extra=palette_bytes(PALETTE, 256)), frames)
    frames = [("dc", cram_frame(f, None if f == 0 else f - 1, False), f == 0) for f in range(FRAMES)]
    size = max(len(d) for _, d, _ in frames)
    write_avi("cram16.avi", b"MSVC", bih(16, b"CRAM", size), frames)


# --- Cinepak ------------------------------------------------------------

class Vectors:
    """A vector chunk's bytes, with its 32-bit flag words put in where the
    decoder reads them (before the first bit it needs)."""

    def __init__(self):
        self.buf = bytearray()
        self.at = 0
        self.used = 32

    def bit(self, b):
        if self.used == 32:
            self.at = len(self.buf)
            self.buf += b"\0\0\0\0"
            self.used = 0
        if b:
            word = int.from_bytes(self.buf[self.at:self.at + 4], "big") | 0x80000000 >> self.used
            self.buf[self.at:self.at + 4] = word.to_bytes(4, "big")
        self.used += 1


def cv_chunk(cid, data):
    return struct.pack(">HH", cid, len(data) + 4) + data


def cv_entry(i):
    y, u, v = YUV[i]
    return bytes([y, y, y, y, u & 0xFF, v & 0xFF])


GREY_V4 = 8   # V4 entry: 2x2 dark, top-right light (block (0,0))
PARTIAL_V1 = 9  # V1 entry frame 2's strip 1 adds (color 5)


def cinepak_block(f, bx, by):
    """(V1 index,) or (V4 indexes) for block (bx, by) of frame f."""
    if bx == 0 and by == 0:
        return (GREY_V4,) * 4
    if bx == min(f, 4) and by == 1:
        return (7, 2, 2, 2)
    c = index_at(f, bx * 4, by * 4)
    if f == 2 and (bx, by) == (7, 5):
        return (PARTIAL_V1,)
    if (bx + by) % 4 == 0:
        return (c,) * 4
    return (c,)


def cinepak_frame(f):
    key = f in (0, 4)
    strips = b""
    for s, (y1, y2) in enumerate(((0, 12), (12, 24))):
        body = b""
        if key and s == 0:
            body += cv_chunk(0x2000, b"".join(cv_entry(i) for i in range(8)))
            # (selective, greyscale: entry 8 only)
            body += cv_chunk(0x2500, struct.pack(">I", 0x80000000 >> GREY_V4) + bytes([16, 200, 16, 16]))
            body += cv_chunk(0x2200, b"".join(cv_entry(i) for i in range(8)))
        if f == 2 and s == 1:
            body += cv_chunk(0x2300, struct.pack(">I", 0x80000000 >> PARTIAL_V1) + cv_entry(5))
        vec = Vectors()
        if key and s == 1 and f == 4:
            cid = 0x3200  # (all V1)
            for by in range(y1 // 4, y2 // 4):
                for bx in range(W // 4):
                    vec.buf.append(index_at(f, bx * 4, by * 4))
        else:
            cid = 0x3000 if key else 0x3100
            for by in range(y1 // 4, y2 // 4):
                for bx in range(W // 4):
                    if not key:
                        same = all(index_at(f, x, y) == index_at(f - 1, x, y)
                                   for y in range(by * 4, by * 4 + 4) for x in range(bx * 4, bx * 4 + 4))
                        vec.bit(not same)
                        if same:
                            continue
                    idx = cinepak_block(f, bx, by)
                    vec.bit(len(idx) == 4)
                    vec.buf.extend(idx)
        body += cv_chunk(cid, bytes(vec.buf))
        # (strip 1 of odd frames gives absolute rows; the rest y1 = 0:
        # "below the strip before", y2 its height)
        if s == 1 and f % 2:
            coords = (y1, 0, y2, W)
        else:
            coords = (0, 0, y2 - y1, W)
        strips += bytes([0x10 if key else 0x11]) + (len(body) + 12).to_bytes(3, "big") + struct.pack(">4H", *coords) + body
    head = bytes([0 if key else 1]) + (len(strips) + 10).to_bytes(3, "big") + struct.pack(">HHH", W, H, 2)
    return head + strips, key


def write_cinepak():
    frames = []
    for f in range(FRAMES):
        data, key = cinepak_frame(f)
        frames.append(("dc", data, key))
    size = max(len(d) for _, d, _ in frames)
    write_avi("cinepak.avi", b"cvid", bih(24, b"cvid", size), frames)


# --- Motion JPEG (baseline, 4:2:2, the standard Huffman tables) ----------

ZIGZAG = [0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48,
          41, 34, 27, 20, 13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15,
          23, 30, 37, 44, 51, 58, 59, 52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63]
Q_LUMA = [16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56,
          14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113, 92,
          49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99]
Q_CHROMA = [17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99,
            47, 66, 99, 99, 99, 99, 99, 99] + [99] * 32
DC_LUMA = ([0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0], list(range(12)))
DC_CHROMA = ([0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0], list(range(12)))
AC_LUMA = ([0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7D], bytes.fromhex(
    "01020300041105122131410613516107227114328191a1082342b1c11552d1f0"
    "2433627282090a161718191a25262728292a3435363738393a434445464748494a"
    "535455565758595a636465666768696a737475767778797a838485868788898a"
    "92939495969798999aa2a3a4a5a6a7a8a9aab2b3b4b5b6b7b8b9bac2c3c4c5c6"
    "c7c8c9cad2d3d4d5d6d7d8d9dae1e2e3e4e5e6e7e8e9eaf1f2f3f4f5f6f7f8f9fa"))
AC_CHROMA = ([0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77], bytes.fromhex(
    "000102031104052131061241510761711322328108144291a1b1c109233352f0"
    "156272d10a162434e125f11718191a262728292a35363738393a434445464748"
    "494a535455565758595a636465666768696a737475767778797a828384858687"
    "88898a92939495969798999aa2a3a4a5a6a7a8a9aab2b3b4b5b6b7b8b9bac2c3"
    "c4c5c6c7c8c9cad2d3d4d5d6d7d8d9dae2e3e4e5e6e7e8e9eaf2f3f4f5f6f7f8f9fa"))
QUALITY = 95


def scaled(q):
    s = 200 - 2 * QUALITY
    return [min(255, max(1, (v * s + 50) // 100)) for v in q]


def huff_codes(table):
    bits, vals = table
    codes, code, k = {}, 0, 0
    for length in range(1, 17):
        for _ in range(bits[length - 1]):
            codes[vals[k]] = (code, length)
            code += 1
            k += 1
        code <<= 1
    return codes


class Bits:
    def __init__(self):
        self.out = bytearray()
        self.acc = 0
        self.n = 0

    def put(self, code, length):
        self.acc = self.acc << length | code
        self.n += length
        while self.n >= 8:
            byte = self.acc >> (self.n - 8) & 0xFF
            self.out.append(byte)
            if byte == 0xFF:
                self.out.append(0)
            self.n -= 8
        self.acc &= (1 << self.n) - 1

    def finish(self):
        if self.n:
            self.put((1 << (8 - self.n)) - 1, 8 - self.n)
        return bytes(self.out)


COS = [[math.cos((2 * x + 1) * u * math.pi / 16) for x in range(8)] for u in range(8)]


def fdct(block):
    out = [0.0] * 64
    for v in range(8):
        for u in range(8):
            s = 0.0
            for y in range(8):
                for x in range(8):
                    s += block[y * 8 + x] * COS[u][x] * COS[v][y]
            cu = 1 / math.sqrt(2) if u == 0 else 1.0
            cv = 1 / math.sqrt(2) if v == 0 else 1.0
            out[v * 8 + u] = 0.25 * cu * cv * s
    return out


def magnitude(v):
    n = abs(v).bit_length()
    return n, (v if v >= 0 else v + (1 << n) - 1)


def encode_block(bits, block, q, dc_codes, ac_codes, prev_dc):
    coef = fdct([p - 128 for p in block])
    zz = [int(round(coef[ZIGZAG[i]] / q[ZIGZAG[i]])) for i in range(64)]
    n, val = magnitude(zz[0] - prev_dc)
    bits.put(*dc_codes[n])
    if n:
        bits.put(val, n)
    run = 0
    for i in range(1, 64):
        if zz[i] == 0:
            run += 1
            continue
        while run > 15:
            bits.put(*ac_codes[0xF0])
            run -= 16
        n, val = magnitude(zz[i])
        bits.put(*ac_codes[run << 4 | n])
        bits.put(val, n)
        run = 0
    if run:
        bits.put(*ac_codes[0x00])
    return zz[0]


def jpeg_frame(f):
    rows = frame_rows(f)
    ycc = []
    for row in rows:
        line = []
        for i in row:
            r, g, b = PALETTE[i]
            line.append((0.299 * r + 0.587 * g + 0.114 * b,
                         -0.168736 * r - 0.331264 * g + 0.5 * b + 128,
                         0.5 * r - 0.418688 * g - 0.081312 * b + 128))
        ycc.append(line)
    clamp = lambda v: min(255, max(0, int(round(v))))
    ql, qc = scaled(Q_LUMA), scaled(Q_CHROMA)
    dcl, acl = huff_codes(DC_LUMA), huff_codes(AC_LUMA)
    dcc, acc = huff_codes(DC_CHROMA), huff_codes(AC_CHROMA)
    bits = Bits()
    prev = [0, 0, 0]
    for my in range(0, H, 8):
        for mx in range(0, W, 16):
            for half in (0, 8):
                blk = [clamp(ycc[my + y][mx + half + x][0]) for y in range(8) for x in range(8)]
                prev[0] = encode_block(bits, blk, ql, dcl, acl, prev[0])
            for c in (1, 2):
                # (4:2:2: two pixels' average across)
                blk = [clamp((ycc[my + y][mx + 2 * x][c] + ycc[my + y][mx + 2 * x + 1][c]) / 2)
                       for y in range(8) for x in range(8)]
                prev[c] = encode_block(bits, blk, qc, dcc, acc, prev[c])
    dqt = (b"\xff\xdb" + struct.pack(">H", 2 + 65 * 2) + bytes([0] + [ql[z] for z in ZIGZAG])
           + bytes([1] + [qc[z] for z in ZIGZAG]))  # (zigzag order)
    sof = b"\xff\xc0" + struct.pack(">HBHHB", 17, 8, H, W, 3) + bytes([1, 0x21, 0, 2, 0x11, 1, 3, 0x11, 1])
    sos = b"\xff\xda" + struct.pack(">HB", 12, 3) + bytes([1, 0x00, 2, 0x11, 3, 0x11, 0, 63, 0])
    return b"\xff\xd8" + dqt + sof + sos + bits.finish() + b"\xff\xd9"


def write_mjpeg():
    frames = [("dc", jpeg_frame(f), True) for f in range(FRAMES)]
    size = max(len(d) for _, d, _ in frames)
    write_avi("mjpeg.avi", b"MJPG", bih(24, b"MJPG", size), frames)


# --- PCM sound ----------------------------------------------------------

def square(i):
    """Sample i of the 441 Hz square wave (8-bit unsigned)."""
    return 0xC0 if (i * 882 // 11025) % 2 == 0 else 0x40


def write_audio():
    frames = [("db", dib_frame(f, 8), True) for f in range(FRAMES)]
    audio = []
    for f in range(FRAMES):
        a, b = f * 11025 // FPS, (f + 1) * 11025 // FPS
        audio.append(bytes(square(i) for i in range(a, b)))
    write_avi("audio.avi", b"DIB ", bih(8, 0, len(frames[0][1]), clr_used=8, extra=palette_bytes(PALETTE, 8)),
              frames, audio=audio, rec=True)


os.makedirs(OUT, exist_ok=True)
write_dibs()
write_rles()
write_crams()
write_cinepak()
write_mjpeg()
write_audio()

# (the conformance case media_video's clips: copies, as the conformance
# runners take a case's files from its own folders)
CASES = os.path.join(HERE, "..", "tests", "conformance", "cases", "video_files")
os.makedirs(CASES, exist_ok=True)
for name in ("cinepak", "rle8", "mjpeg", "audio"):
    shutil.copyfile(os.path.join(OUT, name + ".avi"), os.path.join(CASES, "video_" + name + ".avi"))
