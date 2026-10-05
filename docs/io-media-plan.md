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

## Open

- RapidR's `VAL("12abc")` is 0; RapidQ's is presumably 12 (QCGI's ContentLength reads 12 for "12abc" through it) — to check with RC.EXE and fix in `rapidr_value::builtins::rp_val` (outside this lane).
