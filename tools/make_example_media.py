#!/usr/bin/env python3
"""Writes the media files RapidR's examples use, all of RapidR's own making
(no downloads, no Pillow, numpy or ffmpeg; the same bytes every run):

    examples/media/tune.mid        a short original tune: piano, bass and
                                   drums, format 0, 120 bpm (QMIDI)
    examples/media/chime.wav       a bell-like chime, 16-bit mono 11025 Hz
                                   (QWAVE)
    examples/media/clip.avi        a bouncing ball, 64 x 48, 8-bit
                                   uncompressed frames, 24 frames at 12 fps
                                   (QVIDEO)
    examples/directx/sprites.dxg   a DelphiX image library (QDXIMAGELIST):
                                   "ball" (16 x 16, transparent) and "star"
                                   (16 x 16, transparent)
    examples/directx/cube.x        a text DirectX .X model: a cube with a
                                   colour a face (QD3DMESHBUILDER)

Usage (repo root): python3 tools/make_example_media.py
"""
import math
import os
import struct

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")


def write(rel, data):
    path = os.path.join(ROOT, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)
    print(f"wrote {rel} ({len(data)} bytes)")


# --- tune.mid -----------------------------------------------------------

def vlq(n):
    out = [n & 0x7F]
    n >>= 7
    while n:
        out.insert(0, (n & 0x7F) | 0x80)
        n >>= 7
    return bytes(out)


def tune():
    """Four bars of an original little tune at 96 ticks a quarter: a piano
    melody over a walking bass, a kick and a hi-hat on channel 10."""
    q = 96
    events = []  # (tick, order, bytes)

    def note(ch, start, length, key, vel):
        events.append((start, 1, bytes([0x90 | ch, key, vel])))
        events.append((start + length, 0, bytes([0x80 | ch, key, 0])))

    melody = [  # (beat, beats, key)
        (0, 1, 72), (1, 1, 76), (2, 1, 79), (3, 1, 76),
        (4, 1, 77), (5, 1, 81), (6, 2, 79),
        (8, 1, 76), (9, 1, 74), (10, 1, 72), (11, 1, 74),
        (12, 1, 76), (13, 1, 74), (14, 2, 72),
    ]
    for beat, beats, key in melody:
        note(0, beat * q, beats * q - 8, key, 96)
    bass = [48, 52, 55, 52, 53, 57, 55, 53, 52, 50, 48, 50, 52, 55, 48, 48]
    for beat, key in enumerate(bass):
        note(1, beat * q, q - 12, key, 80)
    for beat in range(16):
        if beat % 2 == 0:
            note(9, beat * q, q // 2, 36, 100)  # kick
        note(9, beat * q + q // 2, q // 4, 42, 60)  # closed hi-hat
    events.sort(key=lambda e: (e[0], e[1]))
    track = bytearray()
    track += b"\x00\xff\x51\x03" + (500000).to_bytes(3, "big")  # 120 bpm
    track += b"\x00\xc0\x00"  # channel 1: acoustic grand piano
    track += b"\x00\xc1\x20"  # channel 2: acoustic bass
    now = 0
    for tick, _, data in events:
        track += vlq(tick - now) + data
        now = tick
    # (the end of the track at the end of the fourth bar: 8 seconds)
    track += vlq(16 * q - now) + b"\xff\x2f\x00"
    return b"MThd" + struct.pack(">IHHH", 6, 0, 1, q) + b"MTrk" + struct.pack(">I", len(track)) + bytes(track)


# --- chime.wav ----------------------------------------------------------

def chime():
    rate, seconds = 11025, 1.2
    partials = [(880.0, 1.0, 3.0), (1760.0, 0.5, 5.0), (2640.0, 0.25, 7.0), (1320.0, 0.3, 4.0)]
    samples = bytearray()
    for i in range(int(rate * seconds)):
        t = i / rate
        attack = min(1.0, t / 0.005)
        v = sum(a * math.exp(-d * t) * math.sin(2 * math.pi * f * t) for f, a, d in partials) / 2.05
        samples += struct.pack("<h", int(max(-1.0, min(1.0, v * attack)) * 30000))
    fmt = struct.pack("<HHIIHH", 1, 1, rate, rate * 2, 2, 16)
    body = b"WAVEfmt " + struct.pack("<I", len(fmt)) + fmt + b"data" + struct.pack("<I", len(samples)) + bytes(samples)
    return b"RIFF" + struct.pack("<I", len(body)) + body


# --- clip.avi -----------------------------------------------------------

def chunk(fourcc, data):
    out = fourcc + struct.pack("<I", len(data)) + data
    return out + b"\0" if len(data) % 2 else out


def clip():
    w, h, frames, fps = 64, 48, 24, 12
    # palette: 0 sky, 1 ground, 2 ball, 3 ball's shine, 4 shadow
    palette = [(40, 90, 170), (60, 150, 70), (230, 60, 40), (255, 220, 200), (30, 90, 40)]
    pal = b"".join(bytes([b, g, r, 0]) for r, g, b in palette) + b"\0\0\0\0" * (256 - len(palette))
    ground = 38

    def frame(n):
        # the ball bounces across: x moves steadily, y a bounce (|sin|)
        cx = 8 + n * 2
        cy = ground - 6 - int(26 * abs(math.sin(math.pi * n / 12)))
        rows = []
        for y in range(h):
            row = bytearray()
            for x in range(w):
                c = 1 if y >= ground else 0
                if y == ground + 1 and abs(x - cx) <= 6:
                    c = 4
                dx, dy = x - cx, y - cy
                if dx * dx + dy * dy <= 36:
                    c = 3 if (dx + 2) ** 2 + (dy + 2) ** 2 <= 2 else 2
                row.append(c)
            rows.append(bytes(row))
        return b"".join(reversed(rows))  # bottom-up DIB rows (64: no padding)

    data = [frame(n) for n in range(frames)]
    size = w * h
    avih = struct.pack("<10I", 1000000 // fps, size * fps, 0, 0x10, frames, 0, 1, size, w, h) + b"\0" * 16
    strh = struct.pack("<4s4sIHHIIIIIIIIhhhh", b"vids", b"DIB ", 0, 0, 0, 0, 1, fps, 0, frames,
                       size, 0xFFFFFFFF, 0, 0, 0, w, h)
    strf = struct.pack("<IiiHHIIiiII", 40, w, h, 1, 8, 0, size, 0, 0, len(palette), 0) + pal
    hdrl = chunk(b"LIST", b"hdrl" + chunk(b"avih", avih) + chunk(b"LIST", b"strl" + chunk(b"strh", strh) + chunk(b"strf", strf)))
    movi = bytearray()
    index = bytearray()
    for d in data:
        index += struct.pack("<4sIII", b"00db", 0x10, 4 + len(movi), len(d))
        movi += chunk(b"00db", d)
    body = b"AVI " + hdrl + chunk(b"LIST", b"movi" + bytes(movi)) + chunk(b"idx1", bytes(index))
    return b"RIFF" + struct.pack("<I", len(body)) + body


# --- sprites.dxg --------------------------------------------------------
# (the layout tools/make_dx_fixture.py documents: DelphiX's TDXImageList
# saved — a resource header, then a TPictureCollectionComponent streamed as
# a Delphi binary form, each picture an 8-bit TDIB)

def short(s):
    return bytes([len(s)]) + s.encode("latin-1")


def dib(rows, palette):
    h, w = len(rows), len(rows[0])
    stride = (w + 3) & ~3
    bits = b"".join(bytes(r) + b"\0" * (stride - w) for r in reversed(rows))
    header = struct.pack("<IiiHHIIiiII", 40, w, h, 1, 8, 0, len(bits), 0, 0, 0, 0)
    pal = b"".join(bytes((b, g, r, 0)) for r, g, b in palette) + b"\0\0\0\0" * (256 - len(palette))
    return b"\x04TDIB" + header + pal + bits


def item(name, rows, palette):
    out = short("Name") + b"\x06" + short(name)
    out += short("PatternHeight") + b"\x02\x00"
    out += short("PatternWidth") + b"\x02\x00"
    data = dib(rows, palette)
    out += short("Picture.Data") + b"\x0a" + struct.pack("<I", len(data)) + data
    out += short("SystemMemory") + b"\x09"
    out += short("Transparent") + b"\x09"
    out += short("TransparentColor") + b"\x07" + short("clWhite")
    return b"\x01" + out + b"\x00"


def sprites():
    # palette: 0 white (see-through), 1 red, 2 light red, 3 dark red,
    # 4 yellow, 5 orange
    palette = [(255, 255, 255), (220, 40, 40), (255, 150, 140), (130, 10, 10), (255, 220, 0), (255, 140, 0)]
    ball = []
    for y in range(16):
        row = []
        for x in range(16):
            dx, dy = x - 7.5, y - 7.5
            d = dx * dx + dy * dy
            if d > 56:
                row.append(0)
            elif (dx + 2.5) ** 2 + (dy + 2.5) ** 2 < 6:
                row.append(2)
            elif d > 40:
                row.append(3)
            else:
                row.append(1)
        ball.append(row)
    star = []
    for y in range(16):
        row = []
        for x in range(16):
            dx, dy = x - 7.5, y - 7.5
            r = math.hypot(dx, dy)
            a = math.atan2(dy, dx)
            edge = 4 + 3.5 * (0.5 + 0.5 * math.cos(5 * a - math.pi / 2))
            row.append(0 if r > edge else (5 if r > edge - 1.5 else 4))
        star.append(row)
    form = b"TPF0" + short("TPictureCollectionComponent") + short("")
    form += short("List") + b"\x0e" + item("ball", ball, palette) + item("star", star, palette) + b"\x00"
    form += b"\x00\x00"
    return b"\xff\x0a\x00DELPHIXPICTURECOLLECTION\x00\x30\x10" + struct.pack("<I", len(form)) + form


# --- cube.x -------------------------------------------------------------

def cube():
    corners = [(x, y, z) for x in (-1, 1) for y in (-1, 1) for z in (-1, 1)]
    index = {c: i for i, c in enumerate(corners)}
    faces = []
    for axis in range(3):
        for sign in (-1, 1):
            quad = [c for c in corners if c[axis] == sign]
            # around the face's centre, then clockwise seen from outside
            # (Direct3D's front faces; its axes are left-handed)
            u, v = [a for a in range(3) if a != axis]
            quad.sort(key=lambda c: math.atan2(c[v], c[u]))
            a, b, c = quad[0], quad[1], quad[2]
            e1 = [b[i] - a[i] for i in range(3)]
            e2 = [c[i] - a[i] for i in range(3)]
            n = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]]
            if n[axis] * sign < 0:
                quad.reverse()
            faces.append([index[c] for c in quad])
    colours = [(0.9, 0.2, 0.2), (0.2, 0.8, 0.3), (0.2, 0.4, 0.95), (0.95, 0.85, 0.2), (0.85, 0.3, 0.85), (0.2, 0.85, 0.85)]
    out = ["xof 0303txt 0032", "// A cube for examples/directx/d3d_cube.rr, made by tools/make_example_media.py:",
           "// two units wide, centred on its frame, each face a colour of its own.", "",
           "Header {", " 1;", " 0;", " 1;", "}", "", "Mesh Cube {", f" {len(corners)};"]
    out += [f" {x:.6f};{y:.6f};{z:.6f};" + ("," if i < len(corners) - 1 else ";") for i, (x, y, z) in enumerate(corners)]
    out.append(f" {len(faces)};")
    out += [f" 4;{','.join(map(str, f))};" + ("," if i < len(faces) - 1 else ";") for i, f in enumerate(faces)]
    out += ["", " MeshMaterialList {", f"  {len(colours)};", f"  {len(faces)};"]
    out += [f"  {i}" + ("," if i < len(faces) - 1 else ";;") for i in range(len(faces))]
    for r, g, b in colours:
        out += ["  Material {", f"   {r:.6f};{g:.6f};{b:.6f};1.000000;;", "   0.000000;",
                "   0.000000;0.000000;0.000000;;", "   0.000000;0.000000;0.000000;;", "  }"]
    out += [" }", "}", ""]
    return "\n".join(out).encode()


def main():
    write("examples/media/tune.mid", tune())
    write("examples/media/chime.wav", chime())
    write("examples/media/clip.avi", clip())
    write("examples/directx/sprites.dxg", sprites())
    write("examples/directx/cube.x", cube())


if __name__ == "__main__":
    main()
