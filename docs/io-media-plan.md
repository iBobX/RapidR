# RapidQ's input / output and media objects: QCGI, QCOMPORT, QDOWNLOAD, QMIDI, QWAVE, QCDAUDIO, QVIDEO

The last objects of `rapidr_ast::RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED` that talk to the world outside the program. This document records what each was in RapidQ (with the evidence), what RapidR does on each runtime (native builds, the interpreter, the browser), the judgement calls, and what is open.

## 0. What these objects were in RapidQ

The manual's Appendix A lists all seven as components, but **only QCOMPORT is in RapidQ's compiler**, and that one can't be used:

- `strings RC.EXE` has the built-in objects' names and their members in pairs (docs/rapidq-ground-truth.md). QCOMPORT is there (BaudRate, DataBits, Parity, Port, StopBits, ReadBufSize, WriteBufSize, AddFlowControl, DelFlowControl, Open, Close, Connected, WaitForLastIO, AbortAllIO, WriteString, Write, ReadString, Read, OnOpen, OnClose, OnRxChar, OnTxEmpty, OnRing, OnError, OnBreak, PendingIO, PurgeIn, PurgeOut, InQue, OutQue, Handle); QCGI, QDOWNLOAD, QMIDI, QWAVE, QVIDEO and QCDAUDIO are not.
- **RC.EXE's QCOMPORT can't be built**: any program using it stops at link time with `Can't open …\Lib\rapidq5c.lib` — a library level RapidQ never shipped (probes in the Windows VM, 2026-10-05). The manual says it "was dropped from RapidQ, but now lives!": RAPIDQ2.INC (John Kelly, 2004–2006) has a `TYPE COMPORT EXTENDS QOBJECT` on kernel32's serial calls and `$DEFINE QCOMPORT COMPORT` ("Just get it over, QCOMPORT does nothing"). The manual's QCOMPORT page documents that library (BytesNotRead, OnComError, OnReadString, …) with the built-in's names marked "no longer supported".
- The others are **RapidQ libraries** — BASIC `TYPE … EXTENDS QOBJECT` in include files a program `$INCLUDE`s: QCGI is Chris Warrington's `qcgi.inc` 1.6 (GPL; RapidR reimplements its behaviour and ships none of its code), QDOWNLOAD Andreas Fink's `Qdownload.inc` (RapidQ's own QSOCKET, HTTP/1.0), QMIDI / QWAVE / QVIDEO / QCDAUDIO D. Glodt's `QMidi.inc`, `QWave.inc`, `QVideo.inc`, `Qcdaudio.inc` (Windows' MCI through `mciSendString`).

So "RapidQ-exact" means: the library's behaviour (what its code does, run by RC.EXE where it can be), its messages, its defaults.

**How RapidR takes them** (`rapidr_ast::library`, run first by both compilers): these objects are RapidR components (QCGI is RCGI, …, in `rapidr_value::objects::rqlib`, one store for every runtime). A program that `$INCLUDE`s the library gets RapidR's object — the file's `TYPE` is left out, its constants and declarations stay (unused, so its Windows calls are never reached); when the file isn't there (RapidR doesn't ship RapidQ's include folder, and the web IDE has no files), the `$INCLUDE` gives the file's constants (`rapidr_preprocessor`'s LIBRARY_INCLUDES). RAPIDQ2.INC's COMPORT (and Pete Kleinschmidt's ComPort.cmp it came from) is RapidR's QCOMPORT the same way.

## 1. QCGI (done)

**RapidQ**: `qcgi.inc` 1.6. Its constructor copies 21 CGI variables (HTTP_ACCEPT, AUTH_TYPE, CONTENT_LENGTH as VAL, …, HTTP_USER_AGENT); `Get(Name, BYREF Value)` parses once — a POST's body by `GET$(CONTENT_LENGTH)` (at most MaxInput), a GET's QUERY_STRING as `ENVIRON$` gives it then (cut to MaxInput), any other method nothing — into at most 256 pairs kept sorted by their capitalised names.

**Ground truth**: 16 probe programs, each setting the CGI variables with the `ENVIRON` statement and making one QCGI (a QCGI DIMmed inside a SUB doesn't compile in RapidQ: RC re-reads the TYPE there and fails at its `total AS INTEGER` field), run by RC.EXE in the Windows VM; 13 became `tests/conformance/cases/cgi_*.bas` with RapidQ's output as `.expected` (POST bodies as `.input`). What they settled:

- `%xx` is decoded only when it is a printable character (32 … 126): `%0A`, `%7f`, `%c3`, `%4G` stay as written, `%` and all; `%%41` is `%A`.
- `+` is a space, except before another `+` or a space: `a++b` is `a+ b`, `+++` at the end is `++ `.
- A second `=` in a pair is dropped (`t=a=b=c` is `abc`); a pair without `=` has the value ""; names are decoded too (`my+name`, `x%3Dy` is the name `X=Y`).
- An empty name: `=x&=y&k=1` makes Get("") find `y`; a last pair without a name is left out (`k=1&=x`: Get("") finds nothing).
- MaxInput takes only values above 0; AutoConvert is 1 or (anything else) 0; ContentLength "12abc" is 12 (VAL reads the leading number — RapidR's own VAL gives 0 there: see Open).
- A QCGI's variables are copied when it's made (`DIM` runs the constructor where it stands), QUERY_STRING read again by the parse; Parse runs once.
- `Get`'s Value by reference: the compilers turn `Get(Name, Value)` into `Value = CGI.__get(Name, Value)` before the statement and read the call as `CGI.__found` (a `WHILE`/`DO` condition's Get is moved before the loop the same way — read once).

**RapidR** (`rapidr_value::objects::cgi`): the same on every runtime. The browser has no CGI environment: its variables are the page's own table of `ENVIRON` strings (empty until the program sets them) and a POST has no body.

**The `ENVIRON` statement** came with it (QCGI's cases need it, and RapidR compiled it to a call that did nothing): RC.EXE splits the string at its first `=`, or at a space when there's none (`"Z = sp"` sets `Z ` to ` sp`); an empty text removes the variable; `ENVIRON$` finds a name in any case, as Windows does (`tests/conformance/cases/environ_statement.bas`). The desktop's is the process's environment; the browser's a table of the page's.

## 2. QCOMPORT (done)

**RapidQ**: RC.EXE knows QCOMPORT (members above; `C.CONNECTED`, `HANDLE`, `INQUE`, `OUTQUE`, `PENDINGIO` are "read-only values"; `Read` / `Write` take three arguments, `ReadString` / `WriteString` two) but can't build a program with it (`rapidq5c.lib`). What programs used is RAPIDQ2.INC's COMPORT, which the manual documents. RC.EXE ran that library in the VM without opening any device (`tests/conformance/cases/comport_defaults.bas`: its defaults, `Close` with nothing open, `Open` of a port name that isn't a device, `WriteString` with nothing open — each message exactly, Windows' own text and CR LF included).

**RapidR** (`rapidr_value::objects::comport`, every runtime): the library's members and messages — Port ("COM1"), BaudRate (9600), DataBits (8), Parity (0), StopBits (1), ReadBufSize / WriteBufSize (1024), DCBflags (20625), Connected, Handle (0; -1 after a failed Open), BytesNotRead / BytesNotWritten (the counts the library's status call saw: after WriteString / ReadString), InQue / OutQue (now); Open, Close, PurgeIn, PurgeOut, WriteString(S, Wait), Write(Stream, Count, Wait), ReadString(Count, Wait); OnComError(Message), OnOpen, OnClose, OnWriteString, OnReadString — plus RC.EXE's own names: Read(Stream, Count, Wait), PendingIO, WaitForLastIO / AbortAllIO / AddFlowControl / DelFlowControl (nothing to do), OnError (with OnComError), OnRxChar(InQue) (fired when bytes arrive: the runtime looks every 50 ms while the program handles it, as for QDXJOYSTICK), OnTxEmpty / OnRing / OnBreak (never fired). RC.EXE's read-only messages at compile time (`comport_read_only`).

**Ports**: serial2 on the desktop (BSD-2-Clause OR Apache-2.0; libc / windows-sys — no system package to build on Linux), its reader thread filling the queue; Web Serial in the browser (Chrome, Edge; a port the page was given before, else the browser's chooser — which needs the user's click, so an OnClick's Open; Open waits for it as for a dialog). The tests' ports are scripted (`RAPIDR_TEST_COMPORT` — set empty by the conformance runner and the GUI runner, so a test never opens a real device — or the page's `RAPIDR_TEST_COMPORT`; a program can set it with ENVIRON before its first QCOMPORT: `comport_echo`): `COM2:echo`, `COM3:reply:OK\r\n`, `COM4:busy`.

Judgement calls:

- **BaudRate**: the library's is the rate (9600); RAPIDQ.INC's `br110` … `br115200` (0 … 12, RC.EXE's own QCOMPORT) are taken as the rates they name — no real port runs at 0 … 12 baud.
- **StopBits 1**: the library's default is 1, which a Windows DCB reads as 1.5 stop bits (most drivers refuse that with 8 data bits: Open's "settings" error). Its author meant one stop bit (its manual's `sbOneStopBit`), and RapidR opens with one; 2 is two.
- **Port names**: a number n is "COMn" (additive). On Linux and macOS a device path (`/dev/ttyUSB0`), or `COMn` for the n-th port the system lists (sorted).
- **Mark / space parity** are refused as Windows refuses settings it can't take (Open's "…changing the Comm Port settings.  The parameter is incorrect."): serial2 and Web Serial have no mark / space. XON / XOFF (DCBflags 21393) has no Web Serial equivalent: none there.
- **Waits**: WriteString's / ReadString's Wait sleeps (RapidQ: `SLEEP.ms`); the page can't sleep inside a call, so in the browser the program pauses for it after the call (its result kept).
- **Read's 1-second timeout** (the library's COMMTIMEOUTS): ReadString waits up to a second for a first byte when nothing has arrived, on the desktop; in the browser it returns what has arrived.

Tests: `comport_defaults` (RapidQ's output), `comport_echo` (scripted ports: Open, OnOpen, WriteString and its event, counts, ReadString, PurgeIn, Write / Read of a stream, a second Open, a port in use, mark parity, Close), `comport_read_only` (RC.EXE's messages) — VM, native, browser; `objects::comport::tests`; the real serial2 path with a pseudo-terminal (`serial::tests`, Linux only: macOS' pseudo-terminals refuse the IOSSIOSPEED ioctl real ports take).

## 3. QDOWNLOAD (done)

**RapidQ**: `Qdownload.inc` v2 — Server, Port (80), File (the path without its leading `/`), OutFile, OutVar, OutDevice (1: OutVar, 2: OutFile), StateDevice (1: State in percent, 2: StateGauge's Position and SpeedLbl's "n B/s"), State, Size, LastError, LastStringError, StateGauge (a QGAUGE, 200 × 20), SpeedLbl (a QLABEL); LeechFile, Check, Percent(Now, Complete), Speed(Now$, Before$, Complete). LeechFile is an `HTTP/1.0` GET through QSOCKET, blocking; its errors: 1 "No Server Specified", 2 "No Serverport Specified", 3 "No File Specified", 4 "No Outputfile Specified", 5 "No Connection to specified host could be established" / 6 "You are not connected to the internet" (by Check), 7 "Connection closed by peer", 8 "No idea what's wrong", 9 "Filesize couldn't be determined, or file is 0 bytes long" (Content-Length 0), 10 "A transmission error has occured, retry" (fewer bytes than Content-Length), 11 "The Server doesn't know the file" (404). LastError is never cleared.

**RapidR** (`rapidr_value::objects::download`, the transfer the runtime's): ureq on the desktop (rustls; SChannel on Windows), `fetch` in the browser (a compiled page's code can't wait: a synchronous request there). **The program doesn't freeze**: LeechFile waits as for a dialog — its windows paint, its timers tick, the gauge moves — on the desktop (native: the runtime steps until the transfer is done; interpreted: a wait the VM serves, `Wait::Dialog` with a background task, `rapidr_ui_app::dialogs::task_wait`) and in the browser (`dialog_web::wait_task`); a console program just waits. StateGauge and SpeedLbl are components of their own (`d.StateGauge.Parent = Form` places it).

Judgement calls:

- **Check** is True: the library read a dial-up flag (`HKLM\System\CurrentControlSet\Services\RemoteAccess`) that today's systems don't have — on them RapidQ's Check is always False and a failed connection always error 6. RapidR's is error 5, the accurate one.
- **Port 443 is HTTPS** (the library would have spoken plain HTTP to it); **redirects are followed** (a server moved to HTTPS answers 301 — the library downloaded the redirect page); **Content-Length in any case**, and **a response without one is taken whole** (Size is what came; the library failed with error 10). Each changes only what failed in RapidQ.
- Any other status (403, 500) is downloaded as the file, as the library did; only 404 is error 11.
- **Speed** within the first second (the library divided by zero): the bytes so far.

Tests: `download_errors` (errors 1 … 5, the defaults, Percent, Speed; the connection refused on 127.0.0.1 — never the internet) on every backend and the browser; the GUI fixture `download` against the tests' own slow local server (`tests/http_test_server.mjs`, in a worker thread; RAPIDR_TEST_HTTP / the page's RAPIDR_TEST_ENV): OutVar with State, OutFile with the gauge, a 404 — and the program's QTIMER ticking while it waits — native, interpreted and in the browser.

## Open

- RapidR's `VAL("12abc")` is 0; RapidQ's is presumably 12 (QCGI's ContentLength reads 12 for "12abc" through it) — to check with RC.EXE and fix in `rapidr_value::builtins::rp_val` (outside this lane).
