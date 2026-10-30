# Recipes for systems programs: memory, bits, a CPU, a screen, a keyboard

Some programs handle a machine's insides: an emulator, a virtual machine, a
debugger, a memory viewer, a reader for a file format. They are made of the
same few techniques: memory as an array of bytes, addresses that wrap, bits
and flags, a loop that fetches and decodes, a stack, a screen drawn from
memory. Each of the ten programs below is **one** of those techniques, in one
file you can run, with a small window that shows the result.

They are in [`examples/recipes/`](../../examples/recipes/) beside the
others, and they are what RapidR Studio's assistant starts from when you ask
it for a CPU, an emulator or a memory viewer: it finds the closest recipe by
itself, in English, Spanish or Portuguese ([the assistant](ai-assistant.md#recipes-and-friendly-hints)).
They are as good to copy by hand: a program that works is the best place to
start a program that has to work.

## One tiny computer

The ten tell one story: a tiny 8-bit computer of our own. It is made up for
these pages (nothing in it is another machine's), so there is nothing to
look up: everything it does is in the files.

- **Memory:** 65,536 bytes, addresses `0000` to `FFFF`. A word is two bytes,
  the low one first.
- **Registers:** `A` and `B` (8 bits), `PC` (the next instruction), `SP` (the
  top of the stack), and three flags: Zero, Carry, Sign.
- **The map:** the program at `0000`, its data at `0100` and `0200`, the
  stack just below `0200` growing down, the screen at `8000`, its colours at
  `9800`.
- **Sixteen instructions:**

| Opcode | Instruction | Does | Cycles (the stack recipe doesn't count them) |
|---|---|---|---|
| `00` | `NOP` | nothing | 1 |
| `01 n` | `LDA n` | `A = n` | 2 |
| `02 n` | `LDB n` | `B = n` | 2 |
| `03 nn` | `LDA [nn]` | `A` = the byte at address `nn` | 4 |
| `04 nn` | `STA [nn]` | the byte at `nn` = `A` | 4 |
| `05` | `ADD` | `A = A + B` (flags) | 1 |
| `06` | `SUB` | `A = A - B` (flags) | 1 |
| `07` | `INC` | `A = A + 1` | 1 |
| `08` | `DEC` | `A = A - 1` | 1 |
| `09 nn` | `JMP nn` | `PC = nn` | 3 |
| `0A nn` | `JNZ nn` | `PC = nn` unless Zero | 3 (2 if not) |
| `0B` | `PUSH` | push `A` | – |
| `0C` | `POP` | pop into `A` | – |
| `0D nn` | `CALL nn` | push `PC`, `PC = nn` | – |
| `0E` | `RET` | pop `PC` | – |
| `FF` | `HALT` | stop | 1 |

The CPU recipe runs this program (a sum, then a count down); the dump has it
at `0020` and the file recipe at `0100`. The disassembler lists a longer one
that uses every instruction.

```
0000  01 05      LDA 05        A = 5
0002  02 03      LDB 03        B = 3
0004  05         ADD           A = 8
0005  04 00 02   STA [0200]    store it
0008  02 01      LDB 01
000A  06         SUB           A = A - 1   (the loop)
000B  0A 0A 00   JNZ 000A      until A is 0
000E  FF         HALT
```

## The ten recipes

| Recipe | The one technique | Try |
|---|---|---|
| [memory_bytes.rr](../../examples/recipes/memory_bytes.rr) | **Memory as a byte array.** `DIM mem(0 TO 65535) AS BYTE`; read and write a byte and a 16-bit word through four small helpers that wrap the address with `AND &HFFFF` and keep a value to a byte | Poke word `1234` at `FFFF`: the high byte lands at `0000` |
| [flags_bits.rr](../../examples/recipes/flags_bits.rr) | **Bits and flags.** A flag is a bit of one number: set with `OR`, clear with `AND (NOT mask)`, test with `AND`. Carry, Zero, Sign, Overflow, Half-carry and N after an add or a subtract (with the carry in); rotate through the carry; a byte as unsigned and as signed | `7F + 01`: Sign and Overflow, because 127 + 1 isn't -128 |
| [cpu_step.rr](../../examples/recipes/cpu_step.rr) | **Fetch, decode, execute.** The CPU above without the stack: `PC` moves on as each byte is fetched, a `SELECT CASE` decodes, the cycles add up; Step, Run and Reset | Step three times: `PC` is `0005` and `A` is `08` |
| [cpu_stack.rr](../../examples/recipes/cpu_stack.rr) | **The stack.** `PUSH`, `POP`, `CALL` and `RET` on the memory array: `SP` counts down on a push, a call pushes its return address (high byte first), the stack shown as it grows | Step three times and watch `01FD: 06 00 05` |
| [hex_dump.rr](../../examples/recipes/hex_dump.rr) | **A memory viewer.** Address, 16 bytes in hex, the same as text; the hex of each value looked up in a table made once, a row built by itself | Show all 64 KB: 4,096 rows |
| [disassembler.rr](../../examples/recipes/disassembler.rr) | **An opcode table.** Bytes back to mnemonics with a table read from `DATA` (no `SELECT CASE`): the name and the kind of operand of each opcode | List from `0006`, the middle of an instruction: the listing goes wrong |
| [load_binary.rr](../../examples/recipes/load_binary.rr) | **A binary file into memory.** `RFileStream`: the header's fields at fixed offsets, `ReadNum(Num_Byte)` and `ReadNum(Num_Word)`, then the memory in a loop | Make the sample file, then load it |
| [video_memory.rr](../../examples/recipes/video_memory.rr) | **Video memory on a canvas.** A 1-bit bitmap and colour attributes in the memory array drawn on an `RCanvas`; every write marks its cell dirty, a frame redraws only those; a ball drawn with `XOR` | Run: a 50 Hz timer |
| [keyboard_matrix.rr](../../examples/recipes/keyboard_matrix.rr) | **A keyboard matrix.** `OnKeyDown` and `OnKeyUp` clear and set a bit in 8 rows of 5; `IN` selects rows with a mask and gives their AND | Press `Q`, then `Enter` |
| [cpu_timing.rr](../../examples/recipes/cpu_timing.rr) | **Timing.** A budget of cycles for each tick of an `RTimer` (a frame, 20 ms), the overspent cycles carried to the next frame, so the speed is exact; Pause and Frame without freezing the window | Faster, then Frame |

And the machine's sound: [frame_audio.rr](../../examples/recipes/frame_audio.rr)
notes the cycle of each flip of a one-bit speaker during a frame and turns
the flips into the frame's 882 samples for an `RAudioOut`
([Emulators: sound from a level timeline](directx-and-media.md#emulators-sound-from-a-level-timeline)).

![The tiny CPU after three Steps: PC 0005, A 08, B 03, 5 cycles](images/systems/cpu-step.png)

![The hex dump: a text and the tiny CPU's program, with the ASCII column](images/systems/hex-dump.png)

![The video memory drawn on a canvas: a bright yellow ball on a chessboard of blue and black cells](images/systems/video-memory.png)

## What they share

- **Hex in and out.** `HEX$` gives 8 digits, so a byte is `RIGHT$(HEX$(v), 2)`
  and an address `RIGHT$(HEX$(v), 4)`. Text to a number is
  `VAL(CONVBASE$(s, 16, 10))`: `VAL("&HFF")` reads 0.
- **Wrap every address.** `addr AND &HFFFF` after every add, and a byte
  with `AND &HFF`: a `BYTE` array already wraps a stored value, but a
  register is an `INTEGER`.
- **Programs are text.** A program is a string of hex bytes
  (`"01 05 02 03 …"`) loaded with `FIELD$` and `CONVBASE$`: easy to read,
  to edit, to hand to a test.
- **Make a table once.** `hexTab(255)` for the hex of each byte, an opcode
  table for the disassembler: a lookup beats a computation in a loop that
  runs 65,536 times.
- **Hand the window back.** A program that loops forever freezes the
  window: run a budget of work for each tick of an `RTimer`.

## RapidR things these programs run into

Found while writing them; each one is worked around in the recipes.

- `DATA &H10` reads as 0; write the number as text (`DATA "10"`) and convert
  it.
- `FORMAT$("%02X", 10)` pads with a space (`" A"`); `FORMAT$("%.2X", 10)` gives
  `0A`, and so does `RIGHT$(HEX$(10), 2)`.
- `RFileStream.ReadStr(n)` gives n + 1 characters (a space at the end);
  `ReadBinStr(n)` gives exactly n.
- `Stream.LoadArray(a(0), n)` and `SaveArray` take an element of the array,
  not its name, and are loops of `ReadNum` and `WriteNum`.
- `STEP` and `WORD` are words of the language: not names for a SUB or a
  variable. A component and a variable can't share a name (`Matrix` and
  `matrix`): the variable is then "not an array".
- A form has to ask for the keys: `KeyPreview = 1` gives the form
  `OnKeyDown` and `OnKeyUp` before the list or edit that has the focus.
- Don't walk a long string with `MID$(s, i, 1)` or `s[i]`: 64 KB takes seconds
  today (each access starts from the beginning). Read a file's bytes with
  `ReadNum`.

## How fast

Measured on a Mac (Apple silicon), 2026-10-29, by
[`tools/bench/systems_loops.rr`](../../tools/bench/systems_loops.rr), which you
can run on your own computer: `rapidr run tools/bench/systems_loops.rr` (every
loop; or one, by its word: `memory`, `hexdump`, `filewrite`, `fileread`,
`cpu`, `cpu2`) and, for the native column, `rapidr build tools/bench/systems_loops.rr out`.
Each time is the program's own `TIMER` for the loop, the fastest of six runs.
The computer was busy with other work while these were measured (a load
average of about 250), so these are upper bounds: on a quiet computer they
are the same or better.

| The loop | The RapidR Runtime (`rapidr run`) | A native build |
|---|---|---|
| [memory_bytes.rr](../../examples/recipes/memory_bytes.rr): 1,310,720 calls of the `MemWrite` / `MemRead` helpers | 0.30 s | 0.06 s |
| [hex_dump.rr](../../examples/recipes/hex_dump.rr): all 4,096 rows (64 KB) built | 0.05 s | 0.02 s |
| [load_binary.rr](../../examples/recipes/load_binary.rr): 64 KB read with `ReadNum(Num_Byte)`, a byte a call | 0.09 s | 0.08 s |
| the same 64 KB **written** with `WriteNum(, 1)`, a byte a call | 0.6 s | 0.8 s |
| the same 64 KB written with one `WriteBinStr` | under 0.01 s | under 0.01 s |
| [cpu_step.rr](../../examples/recipes/cpu_step.rr)'s loop, 2,000,000 cycles (750,001 instructions), the fetch written out in the loop | 0.28 s | 0.04 s |
| the same as [cpu_timing.rr](../../examples/recipes/cpu_timing.rr) writes it (a function for each fetch) | 0.50 s | 0.05 s |

- **How fast a CPU runs.** The CPU of the recipes runs about **4 million
  cycles a second** in the RapidR Runtime (7 million with the fetch written out in the loop) and about
  **40 million** in a native build. At 50 frames a second that is about
  80,000 cycles a frame in the Runtime and about 800,000 in a native build;
  a machine clocked at 3.5 MHz needs 70,000 a frame for the clock alone, so
  its CPU, which does more per instruction than this one, wants the native
  build or a leaner step.
- **The screen.** In [video_memory.rr](../../examples/recipes/video_memory.rr)
  a frame (the ball moved, its few dirty cells redrawn) takes about 0.3 ms of
  the 20 ms a 50 Hz frame has, and redrawing all 768 cells takes about 13 ms
  (CPU time, the Runtime, drawing on the headless canvas).
- **The one slow thing found: writing a file a byte at a time.** A write is a
  system call each time (the computer spends 0.8 s of that second in the
  system), a read is buffered: 64 KB takes 0.6 to 1 s to write and 0.09 s to
  read. Write whole blocks (`WriteBinStr`) when you can.

## Running them

Each file runs as every RapidR program does: `rapidr run memory_bytes.rr`,
as a standalone executable (`rapidr build … --interp`), as a native build
and in a browser (`rapidr bundle-bc`). The examples suite
(`tests/examples_run.mjs`) clicks all ten on all four: Step three times and
`PC` is `0005`; the dump shows `48 65 6C 6C 6F`; `7F + 01` sets Sign and
Overflow; Space is a `0` bit in row 7.
