#!/usr/bin/env python3
"""Writes tests/fixtures/dx_sprites.dxg: a DelphiX image library (.DXG) for
tests/fixtures/dx_screen.bas, made the way DelphiX's TDXImageList saves one
(a resource header, then a TPictureCollectionComponent streamed as a Delphi
binary form, TPF0, each picture an 8-bit TDIB). RapidR's own drawing, so the
test needs none of RapidQ's example files.

  item 0 "sprite": 16 x 16, white with a red square at (4,4)-(12,12);
                   Transparent, TransparentColor clWhite
  item 1 "strip":  16 x 8, PatternWidth / PatternHeight 8: pattern 0 green,
                   pattern 1 yellow; not transparent

Also tests/fixtures/dx_beep.wav for tests/fixtures/dx_sound.bas: half a
second of a 500 Hz square wave, 8-bit mono at 8000 Hz (4000 bytes of sound).

And tests/fixtures/d3d_tex.bmp, the texture tests/fixtures/d3d_model.x
names (for tests/fixtures/d3d_xfile.bas): 2 x 2, 24-bit, the left column
blue, the right one yellow.

Usage (repo root): python3 tools/make_dx_fixture.py
"""
import os
import struct

WHITE, RED, GREEN, YELLOW = 1, 2, 3, 4
# Palette entries are B, G, R, 0.
PALETTE = {WHITE: (255, 255, 255), RED: (0, 0, 255), GREEN: (0, 255, 0), YELLOW: (0, 255, 255)}


def dib(rows):
    """An 8-bit TDIB: BITMAPINFOHEADER, 256-entry palette, rows bottom-up."""
    h, w = len(rows), len(rows[0])
    stride = (w + 3) & ~3
    bits = b"".join(bytes(r) + b"\0" * (stride - w) for r in reversed(rows))
    header = struct.pack("<IiiHHIIiiII", 40, w, h, 1, 8, 0, len(bits), 0, 0, 0, 0)
    palette = b"".join(bytes(PALETTE.get(i, (0, 0, 0))) + b"\0" for i in range(256))
    return b"\x04TDIB" + header + palette + bits


def short(s):
    return bytes([len(s)]) + s.encode("latin-1")


def item(name, rows, pattern, transparent):
    out = short("Name") + b"\x06" + short(name)
    out += short("PatternHeight") + b"\x02" + bytes([pattern[1]])
    out += short("PatternWidth") + b"\x02" + bytes([pattern[0]])
    data = dib(rows)
    out += short("Picture.Data") + b"\x0a" + struct.pack("<I", len(data)) + data
    out += short("SystemMemory") + b"\x09"
    out += short("Transparent") + (b"\x09" if transparent else b"\x08")
    out += short("TransparentColor") + b"\x07" + short("clWhite")
    return b"\x01" + out + b"\x00"


def main():
    sprite = [[RED if 4 <= x < 12 and 4 <= y < 12 else WHITE for x in range(16)] for y in range(16)]
    strip = [[GREEN if x < 8 else YELLOW for x in range(16)] for _ in range(8)]
    form = b"TPF0" + short("TPictureCollectionComponent") + short("")
    form += short("List") + b"\x0e" + item("sprite", sprite, (0, 0), True) + item("strip", strip, (8, 8), False) + b"\x00"
    form += b"\x00\x00"
    resource = b"\xff\x0a\x00DELPHIXPICTURECOLLECTION\x00\x30\x10" + struct.pack("<I", len(form))
    root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
    with open(os.path.join(root, "tests", "fixtures", "dx_sprites.dxg"), "wb") as f:
        f.write(resource + form)
    sound = bytes(0xC0 if (i // 8) % 2 == 0 else 0x40 for i in range(4000))
    fmt = struct.pack("<HHIIHH", 1, 1, 8000, 8000, 1, 8)
    wav = b"WAVEfmt " + struct.pack("<I", len(fmt)) + fmt + b"data" + struct.pack("<I", len(sound)) + sound
    with open(os.path.join(root, "tests", "fixtures", "dx_beep.wav"), "wb") as f:
        f.write(b"RIFF" + struct.pack("<I", len(wav)) + wav)
    # A 24-bit BMP: rows bottom-up, B, G, R, each row padded to 4 bytes.
    row = bytes((255, 0, 0)) + bytes((0, 255, 255)) + b"\0\0"
    bits = row * 2
    info = struct.pack("<IiiHHIIiiII", 40, 2, 2, 1, 24, 0, len(bits), 2835, 2835, 0, 0)
    bmp = b"BM" + struct.pack("<IHHI", 14 + len(info) + len(bits), 0, 0, 14 + len(info)) + info + bits
    with open(os.path.join(root, "tests", "fixtures", "d3d_tex.bmp"), "wb") as f:
        f.write(bmp)


if __name__ == "__main__":
    main()
