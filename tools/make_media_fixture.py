#!/usr/bin/env python3
"""Writes tests/fixtures/media_song.mid for tests/fixtures/media.bas (QMIDI):
a standard MIDI file of RapidR's own — format 0, 96 ticks a quarter, 120
bpm (500000 us a quarter), a piano playing C and E, a quarter note each:
192 ticks, which MCI's timing (whole microseconds a tick: 5208) makes
999 ms.

Usage (repo root): python3 tools/make_media_fixture.py
"""
import os
import struct

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "tests", "fixtures", "media_song.mid")

track = bytes([
    0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20,  # tempo 500000
    0x00, 0xC0, 0x00,                          # piano
    0x00, 0x90, 60, 100,                       # C on
    0x60, 0x80, 60, 0,                         # 96 ticks later: off
    0x00, 0x90, 64, 100,                       # E on
    0x60, 0x80, 64, 0,                         # off
    0x00, 0xFF, 0x2F, 0x00,                    # end of track
])
data = b"MThd" + struct.pack(">IHHH", 6, 0, 1, 96) + b"MTrk" + struct.pack(">I", len(track)) + track
with open(OUT, "wb") as f:
    f.write(data)
print(f"wrote {OUT} ({len(data)} bytes)")
