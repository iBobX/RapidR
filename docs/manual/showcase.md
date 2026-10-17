# Showcase: bigger programs

These programs are in [`examples/showcase/`](../../examples/showcase/),
each one a whole program in one file, written from an idea and from nothing
else: no one's code was copied, the data is made up by the programs
(except the weather one, which asks a public weather service), and each
uses only what RapidR has today. Open them in RapidR Studio from **Open an
Example** on the Welcome page (it lists them with the other examples), or
run them from a terminal; the first lines of each say how.

All of them run as the interpreter runs them (`rapidr run`), as a
standalone executable made with `--interp`, as a native build and in a
browser, and they follow the system's light, dark or high-contrast look.
Where a program needs something a browser can't give it, its section says so.

| Program | What it shows |
|---|---|
| [Fireworks](#fireworks) | thousands of sparks, each with its own physics; RNum draws the shapes |
| [A MIDI visualizer](#a-midi-visualizer) | a 3D scene that dances to a song the program reads itself |

## Fireworks

[`fireworks.rr`](../../examples/showcase/fireworks.rr): click the sky, or
press Space, and a rocket flies up and bursts into a ball, a ring, two
rings or a heart. Keys 1 to 4 pick the shape, 0 lets the program choose, A
turns on an automatic show. The corner shows how many sparks are flying and
how many frames a second the program draws.

![Fireworks: a heart, a ball and two rings burst in the night sky; the corner says how many sparks and how many frames a second](images/showcase/fireworks.png)

Every spark is one place in a set of arrays. This is all a spark's life,
once a frame, for each spark that is still flying:

```basic
svy(i) = (svy(i) + GRAVITY * dt) * drag      ' gravity pulls it down, the air slows it
svx(i) = svx(i) * drag
sx(i) = sx(i) + svx(i) * dt                  ' it moves
sy(i) = sy(i) + svy(i) * dt
f = f - fade(i) * dt                         ' and its life runs out
```

`dt` is the time since the last frame, so the sparks fly the same speed on
a slow computer and a fast one. A spark is drawn as a short line from where
it is to where it just was, in a colour that dims as its life ends:

```basic
c = RGB(8 + cr(i) * br, 10 + cg(i) * br, 28 + cb(i) * br)
Sky.Line sx(i), sy(i), sx(i) - svx(i) * dt * 4, sy(i) - svy(i) * dt * 4, c
```

New sparks take the next free place in the arrays, and when the arrays are
full the oldest spark makes room (`head = (head + 1) MOD MAXSPARKS`), so
the program never allocates anything while it runs.

**The shapes are sums on whole arrays.** A burst of `n` sparks needs `n`
angles round a circle and a speed for each. `RNum`, RapidR's array of
numbers, makes them in one step each, with no loop:

```basic
Ang.Linspace 0, PI2, n                 ' n angles round the circle
Ang.Add(RND * PI2)                     ' turned a random amount
Spd.Uniform 0.35, 1, n                 ' speeds, from 35 % to the full 100 %
Vx.Data = Ang.Data: Vx.Cos             ' the direction: cos and sin of the angle
Vy.Data = Ang.Data: Vy.Sin
```

A ring is the same with every speed equal (`Spd.Full n, 1`). The heart is
the classic heart curve, worked out on the whole array: `Vx.Sin` then
`Vx.Power 3` then `Vx.Multiply 16` is sixteen times the cube of the sine of
every angle, in three statements.

**Frames come from an `RDXTimer`.** `RDXScreen` is a surface the program
draws on and shows with `Flip`; the timer with `Interval = 0` fires once a
frame, and its `FrameRate` property is the number in the corner. The program
draws about two thousand lines a frame, and measures:

| Where | Sparks | Frames a second |
|---|---|---|
| The interpreter on a Mac (headless, software drawing) | about 1,000 | 38 to 46 |
| The browser (headless Chromium) | about 1,800 | 35 |
| The browser (headless Chromium) | 2,500 to 2,900 | 27 to 40 |

The cost is the drawing calls: each takes about 5 microseconds, so 2,000
sparks are about 10 milliseconds of the frame's 16 or 33; the arithmetic for
all of them takes less than one millisecond.

## A MIDI visualizer

[`midi_visualizer.rr`](../../examples/showcase/midi_visualizer.rr): a song
plays and a 3D scene reacts to it, note by note. **Load MIDI…** opens any
`.mid` file and the same scene follows it; **Play**, **Pause** and **Stop**
do what they say; **Loop** plays the song again when it ends; **Calm** makes
every movement gentler. The song it opens with is `song.mid`, an original
32-bar loop made by [`make_song.rr`](../../examples/showcase/make_song.rr)
(below).

![The visualizer in the middle of the song: a crystal in a ring of columns, the tall ones in the colours of the notes just played, sparks flying up](images/showcase/midi-visualizer.png)

What moves what:

| In the song | In the scene |
|---|---|
| a kick drum | the crystal pulses (and the background brightens a little) |
| the pitch of a note | the colour of its column and spark: low notes deep blue, then violet and pink, the high ones orange; and how high they go |
| a bass note | a low jump of its column and a shake of the camera |
| the notes a second | how fast the ring turns |
| a crash or snare | a push to the turning |

The colour of the light over the floor follows the notes smoothly, over a
third of a second, and kick pulses are at least a third of a second apart,
so nothing flashes: the scene is built not to be a problem for people who
are sensitive to flashing lights (it follows the idea of the three-flashes
rule, WCAG 2.3.1; it hasn't been tested by anyone medical, and **Calm**
softens the movement further).

**The program reads the MIDI file itself.** A MIDI file is a header and
chunks of events, each event after a "delta time", the ticks since the one
before, written as a variable-length number: seven bits a byte, the high
bit set on every byte but the last. This function reads one:

```basic
FUNCTION ReadVLQ AS LONG
    DIM v AS LONG, b AS INTEGER
    DO
        b = Rd.ReadByte
        v = v * 128 + (b AND 127)
    LOOP WHILE b >= 128
    ReadVLQ = v
END FUNCTION
```

`Rd` is an `RFileStream`; `ReadByte` reads the next byte. A track is read
event by event. A status byte starts an event, but a MIDI file may leave it
out when it is the same as the last one ("running status"), so the reader
puts the byte back when it isn't a status:

```basic
b = Rd.ReadByte
IF b >= 128 THEN
    status = b
ELSE
    status = run                      ' running status: the byte was data
    Rd.Position = Rd.Position - 1
END IF
```

A meta event (`&HFF`) carries the tempo (`&H51`, the microseconds in a
beat) and names; the reader keeps those and every Note On with a velocity
above 0, and skips the rest by its length. Notes are counted in ticks, so
`TickMs` walks the tempo changes to turn each into milliseconds. The notes
of all the tracks are then put in time order with one sort: each note
becomes the number `milliseconds * 100000 + its number`, and `QUICKSORT`
sorts those numbers:

```basic
sortKey(i) = INT(TickMs(rawTick(i))) * 100000 + i
...
QUICKSORT sortKey(0), sortKey(noteCount - 1), 0
```

**RMIDI plays; the program watches the clock.** `Midi.Timer.Interval = 20`
makes RMIDI tell the program the song's position in milliseconds fifty times
a second (`Midi.OnChange = Moved`). Each frame, every note that is due
moves something:

```basic
DO WHILE nextNote < noteCount AND Midi.State = MD_PLAY
    IF noteMs(nextNote) > songPos THEN EXIT DO
    NoteOn noteChan(nextNote), noteKey(nextNote), noteVel(nextNote)
    nextNote = nextNote + 1
LOOP
```

RapidR has a General MIDI synthesizer of its own, so the song is heard on
every system and in the browser, whether or not the system has one. The 3D is
`RDXScreen` with Direct3D objects: frames, lights, a camera and meshes the
program builds from faces (a box, a crystal ball and a disc: `BuildBox`,
`BuildBall`, `BuildDisc`). The ring turns by setting a frame's rotation and
calling `DX.Move`; a pulse is `GemPulse.AddScale(0, size, size, size)`; the
shake is a small random move of `DX.SetCameraPosition`.

Run `rapidr run midi_visualizer.rr 40` to start the song at its 40th second.

### make_song.rr: the song

[`make_song.rr`](../../examples/showcase/make_song.rr) composes `song.mid`
and writes it as a standard MIDI file, with an `RFileStream`'s `WriteByte`.
It makes five tracks (the tempo, drums, bass, arpeggio and pad), 32 bars at
128 beats a minute in A minor on the chords Am, F/A, C and G: four on the
floor, an off-beat bass, an arpeggio in sixteenths and a pad. A new layer
joins every eight bars, and the last bar of each section is a snare roll.
Every note comes from the rules in the program; nothing is copied from a
song. Notes are put in time order the same way the visualizer does it, with
`QUICKSORT` on numbers that hold the tick and the note's place, and each
event's delta time is written with this:

```basic
SUB Delta (n AS LONG)
    DIM groups(4) AS INTEGER, g AS INTEGER, v AS LONG
    v = n
    groups(0) = v AND 127                         ' the last 7 bits
    g = 0
    v = v \ 128
    DO WHILE v > 0
        g = g + 1
        groups(g) = (v AND 127) OR 128            ' more bits: the high bit set
        v = v \ 128
    LOOP
    DO WHILE g >= 0
        PutByte groups(g)                         ' the biggest group first
        g = g - 1
    LOOP
END SUB
```

The song in the repository is the one the program makes: the examples' tests
run `make_song.rr` and compare its `song.mid` with the shipped one, byte
for byte.
