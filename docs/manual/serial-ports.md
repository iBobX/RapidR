# Serial ports and IoT boards

ESP32, Arduino, Raspberry Pi Pico and most other microcontroller boards talk
to a computer through a USB serial adapter. A RapidR program reaches them with
**RComPort** — RapidQ's `QCOMPORT` (RAPIDQ2.INC's `COMPORT`), which works as it
did in RapidQ — plus a few RapidR extras made for these boards: the ports
listed with their USB IDs, the DTR / RTS lines that reset a board, whole lines
in, and an event when a board is plugged in or out.

It works the same way in native builds, in the interpreter (`rapidr run`)
and in the browser (Chrome and Edge, through Web Serial). The example
[`examples/iot/esp32_monitor.rr`](../../examples/iot/esp32_monitor.rr) is a
complete serial monitor built on what follows.

## Opening a port

```basic
DIM Board AS RComPort        ' or QCOMPORT: the same component
Board.Port = "COM3"          ' Windows; /dev/cu.usbserial-0001 (macOS), /dev/ttyUSB0 (Linux)
Board.BaudRate = 115200
Board.Open
IF Board.Connected THEN PRINT "open"
```

`DataBits` (8), `Parity` (0: none) and `StopBits` (1) are read at `Open`.
`OnOpen` fires when it opens; `OnComError` gets the message when it doesn't
("…The system cannot find the file specified." for a port that isn't there,
"…Access is denied." for one another program has open). `Close` closes it.

On macOS and Linux, `Port` can also be `COMn`: the n-th port the system lists.
In the browser, `COMn` is the n-th port the page was allowed; a port the page
hasn't been given yet opens the browser's chooser, which only works from a
click (an `OnClick` handler's `Open`).

## Which ports there are

```basic
FOR i = 0 TO Board.ListPorts - 1
    PRINT Board.PortName(i), Board.PortDescription(i), HEX$(Board.PortVendorID(i))
NEXT
```

| Member | What it gives |
|---|---|
| `ListPorts` | Looks at the ports there are now; returns how many |
| `PortCount` | How many the last `ListPorts` found |
| `PortName(i)` | The name `Port` takes (`i` from 0) |
| `PortDescription(i)` | What the device is: the system's name for it ("CP2102N USB to UART Bridge Controller", "M5stack"), else the chip its USB IDs name ("CH340 USB to serial", "USB JTAG/serial debug unit (ESP32)") |
| `PortManufacturer(i)`, `PortSerialNumber(i)` | As the USB device says (the serial number tells two identical boards apart) |
| `PortVendorID(i)`, `PortProductID(i)` | The USB IDs: `&H10C4` Silicon Labs, `&H1A86` WCH (CH340), `&H0403` FTDI, `&H303A` Espressif; 0 when not USB |
| `FillList(Control)` | Puts the ports in a list or combo box, in `PortName` order, the port `Port` names selected; returns how many |

```basic
Board.FillList(Ports)                       ' Ports is a combo box
Board.Port = Board.PortName(Ports.ItemIndex)
```

The desktop lists every port the system has: on macOS the `/dev/cu.*` ports
(the `tty.*` twins wait for a carrier, which a board never raises), with the
USB details from IOKit; on Windows the `COMn` ports with SetupAPI's; on Linux
the `/dev/tty*` serial ports with sysfs's. The browser lists only the ports the
page was allowed, as `COM1`, `COM2` …, with their USB IDs (the browser gives no
names, so the description comes from the IDs). The browser answers a moment after
the program makes its first RComPort, so a list taken in the program's first
instant can be empty: `OnPortsChanged` fires when the ports arrive.

## Plugged in, unplugged

```basic
Board.OnPortsChanged = PortsChanged
SUB PortsChanged(Added AS STRING, Removed AS STRING)
    IF INSTR(Removed, Board.Port) THEN Board.Close
    Board.FillList(Ports)
END SUB
```

`OnPortsChanged` gets the names that came and the names that went (a CR LF
between two). The desktop looks about once a second while the program
handles the event; the browser tells at once.

## Lines

Boards print text a line at a time. `ReadLine` gives the next whole line,
without its end:

```basic
l$ = Board.ReadLine(500)     ' waits up to 500 ms for a line; "" if none came
```

Without a timeout it waits up to a second; `ReadLine(0)` doesn't wait. A line
only partly arrived stays where it is for the next read. `HasLine` is 1 when a
whole line is waiting.

`LineEnd` is what ends a line: `CHR$(10)` unless you set it. With `CHR$(10)` a
CR just before it is dropped too, so `CR LF` (Arduino's `println`, the ESP32
ROM) and `LF` lines both come out clean. Set `LineEnd = CHR$(13)` for a device
that ends lines with CR alone, or any other text for other protocols.

Or let the lines come to you:

```basic
Board.OnLine = GotLine
SUB GotLine(Received AS STRING)
    Log.AddStrings Received
END SUB
```

`OnLine` fires for each whole line (the runtime looks every 50 ms). A line it
gives is read: `ReadString` and `ReadLine` don't see it again. `OnRxChar`
(RapidQ's: bytes arrived) still fires as before.

`ReadString(Count, Wait)` and `WriteString(Text, Wait)` work as RapidQ's do,
for anything that isn't line by line.

## The modem lines: resetting a board

| Member | |
|---|---|
| `DTR`, `RTS` | Data Terminal Ready and Request To Send: 1 set, 0 clear. Read back as set. A change happens at once on an open port, else at `Open`. Until a program sets them they follow `DcbFlags` (both set, as RapidQ opens a port) |
| `CTS`, `DSR`, `CD`, `RI` | The lines the other end drives (read only; 0 while closed) |
| `SendBreak(ms)` | Holds the line in a break (250 ms without an argument) |

ESP32 boards (and ESP8266, and many others) wire DTR and RTS through two
transistors to the chip's **EN** (reset) and **IO0** (boot mode) pins:

| DTR | RTS | EN | IO0 |
|---|---|---|---|
| 1 | 1 | high (runs) | high |
| 0 | 0 | high (runs) | high |
| 0 | 1 | **low (held in reset)** | high |
| 1 | 0 | high | **low** |

So a plain reset — the board restarts into its own program — is:

```basic
Board.DTR = 0
Board.RTS = 1      ' EN low: the chip stops
SLEEP 0.1
Board.RTS = 0      ' EN high, IO0 high: it boots its program
```

and the ESP32's ROM prints its boot log at 115200 baud:

```
ets Jun  8 2016 00:22:57

rst:0x1 (POWERON_RESET),boot:0x13 (SPI_FAST_FLASH_BOOT)
configsip: 188777542, SPIWP:0xee
…
entry 0x400805e4
```

then whatever the board's program prints (that one came from an M5StickC
Plus). Holding IO0 low while EN rises starts the chip's download mode
instead: that is what flashing tools do, and nothing in this page needs it.

Arduino Uno-style boards reset when DTR goes from clear to set (a capacitor
on the reset pin), which is why opening their port restarts the sketch.

## Per system

- **macOS**: USB adapters appear as `/dev/cu.usbserial-…` (FTDI, CP210x),
  `/dev/cu.SLAB_USBtoUART` (Silicon Labs' own driver), `/dev/cu.wchusbserial…`
  (WCH's driver) or `/dev/cu.usbmodem…` (boards with native USB, ESP32-S3 /
  C3, Pico). Recent macOS versions drive CP210x, CH340 and FTDI without extra
  drivers.
- **Windows**: `COM3`, `COM4` …; `COM10` and above work too. CH340 boards may
  need WCH's driver on older Windows versions.
- **Linux**: `/dev/ttyUSB0` (adapters), `/dev/ttyACM0` (native USB). Your
  user needs to be in the `dialout` group (`uucp` on Arch) to open them.
- **The browser**: Web Serial, in Chrome and Edge (not Firefox or Safari),
  on `https://` or `localhost` pages. The page asks the user to pick a port
  the first time; after that it may open it again without asking. `ReadLine`
  and `ReadString` waits let the page paint while they wait.

Only one program can have a port open at a time: close the Arduino IDE's or
`esptool`'s monitor before opening the port from your program, and the other
way round.

## Testing without a board

Tests never open a real device. Set `RAPIDR_TEST_COMPORT` (an environment
variable on the desktop, a `window.RAPIDR_TEST_COMPORT` on the page, or the
program's own `ENVIRON` before its first port) to scripted ports:
`COM2:echo` (what's written comes back), `COM3:reply:OK\r\n` (every write is
answered), `COM4:busy` (in use) and `COM5:esp32` (an ESP32 behind a CP2102N:
`ListPorts` describes it, DTR / RTS reset it and it prints a boot log). The
lines loop back as a test plug's do: RTS to CTS, DTR to DSR and CD. Changing
the variable while the program runs plugs ports in and out (`OnPortsChanged`).

`tools/esp32_check.sh` checks a real board by hand: it lists the ports,
opens the board at 115200, resets it and prints the boot log — interpreted
and native — and never writes a byte to the board.
