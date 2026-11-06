' A RapidQ program that receives MIDI System Exclusive messages the way
' RapidQ programs do (as a reported one does): a MIDIHDR TYPE and a string
' buffer handed to winmm by VARPTR, MM_MIM_LONGDATA heard in the form's
' message handler, which reads dwBytesRecorded and the buffer, then gives
' the buffer back with midiInAddBuffer. The DLL is the stand-in
' (tests/fixtures/fake_winmm: fakewinmm.dll, built beside it) — its
' "driver" writes the header and the buffer after midiInAddBuffer returned.
' tools/release/windows/dll-memory-check.ps1 clicks Search, picks the
' device, clicks Start, and prints the log.
$TYPECHECK ON
$ESCAPECHARS OFF
CONST MM_MIM_DATA = &H3C3
CONST MM_MIM_LONGDATA = &H3C4
CONST CALLBACK_WINDOW = &H10000

TYPE MIDIHDR
    lpData AS LONG
    dwBufferLength AS LONG
    dwBytesRecorded AS LONG
    dwUser AS LONG
    dwFlags AS LONG
    lpNext AS LONG
    reserved AS LONG
    dwOffset AS LONG
    dwReserved AS STRING * 64
END TYPE

TYPE MIDIINCAPS
    wMid AS WORD
    wPid AS WORD
    vDriverVersion AS LONG
    szPname AS STRING * 32
    dwSupport AS DWORD
END TYPE

DECLARE FUNCTION midiInGetNumDevs LIB "fakewinmm" ALIAS "midiInGetNumDevs" () AS LONG
DECLARE FUNCTION midiInGetDevCaps LIB "fakewinmm" ALIAS "midiInGetDevCapsA" (uDeviceID AS LONG, lpCaps AS MIDIINCAPS, cbCaps AS LONG) AS LONG
DECLARE FUNCTION midiInOpen LIB "fakewinmm" ALIAS "midiInOpen" (BYREF lphMidiIn AS LONG, uDeviceID AS LONG, dwCallback AS LONG, dwInstance AS LONG, dwFlags AS LONG) AS LONG
DECLARE FUNCTION midiInStart LIB "fakewinmm" ALIAS "midiInStart" (hMidiIn AS LONG) AS LONG
DECLARE FUNCTION midiInPrepareHeader LIB "fakewinmm" ALIAS "midiInPrepareHeader" (BYVAL hMidiIn AS LONG, lpHdr AS MIDIHDR, BYVAL cbHdr AS LONG) AS LONG
DECLARE FUNCTION midiInAddBuffer LIB "fakewinmm" ALIAS "midiInAddBuffer" (BYVAL hMidiIn AS LONG, lpHdr AS MIDIHDR, BYVAL cbHdr AS LONG) AS LONG

DECLARE SUB Search
DECLARE SUB Start
DECLARE SUB Heard (hWnd AS LONG, uMsg AS LONG, wParam AS LONG, lParam AS LONG)

DIM HdrIN AS MIDIHDR
DIM Caps AS MIDIINCAPS
DIM hMidiIn AS LONG, Counter AS LONG, RetVal AS LONG
DIM inBuffer AS STRING, RecMessage AS STRING

CREATE MainForm AS QFORM
    Caption = "SysEx in"
    Width = 520
    Height = 360
    OnMessage = Heard
    CREATE SearchMIDIDevsBTN AS QBUTTON
        Caption = "Search"
        Left = 10: Top = 10: Width = 90
        OnClick = Search
    END CREATE
    CREATE StartStopMIDIIn AS QBUTTON
        Caption = "Start"
        Left = 110: Top = 10: Width = 90
        OnClick = Start
    END CREATE
    CREATE DevINList AS QLISTBOX
        Left = 10: Top = 44: Width = 190: Height = 270
    END CREATE
    CREATE LogsRichEdit AS QRICHEDIT
        Left = 210: Top = 10: Width = 290: Height = 304
        PlainText = 1
        ReadOnly = 1
    END CREATE
END CREATE

SUB Search
    DevINList.Clear
    FOR Counter = 0 TO midiInGetNumDevs - 1
        midiInGetDevCaps(Counter, Caps, SIZEOF(Caps))
        DevINList.AddItems Caps.szPname
    NEXT Counter
    LogsRichEdit.AddStrings "Found " + STR$(DevINList.ItemCount) + " IN device(s)"
END SUB

SUB Start
    IF DevINList.ItemIndex < 0 THEN
        LogsRichEdit.AddStrings "No device selected"
        EXIT SUB
    END IF
    RetVal = midiInOpen(hMidiIn, DevINList.ItemIndex, MainForm.Handle, 0, CALLBACK_WINDOW)
    LogsRichEdit.AddStrings "midiInOpen: " + STR$(RetVal)
    inBuffer = STRING$(1024, 0)
    HdrIN.lpData = VARPTR(inBuffer)
    HdrIN.dwBufferLength = LEN(inBuffer)
    HdrIN.dwBytesRecorded = 0
    HdrIN.dwFlags = 0
    midiInPrepareHeader(hMidiIn, HdrIN, SIZEOF(HdrIN))
    LogsRichEdit.AddStrings "midiInAddBuffer: " + STR$(midiInAddBuffer(hMidiIn, HdrIN, SIZEOF(HdrIN)))
    LogsRichEdit.AddStrings "midiInStart: " + STR$(midiInStart(hMidiIn))
END SUB

SUB Heard (hWnd AS LONG, uMsg AS LONG, wParam AS LONG, lParam AS LONG)
    IF uMsg = MM_MIM_DATA THEN
        LogsRichEdit.AddStrings "MIDI IN Message: " + RIGHT$(HEX$(lParam AND &HFF), 2)
    ELSEIF uMsg = MM_MIM_LONGDATA THEN
        RecMessage = ""
        FOR Counter = 1 TO HdrIN.dwBytesRecorded
            RecMessage = RecMessage + RIGHT$("0" + HEX$(ASC(inBuffer[Counter])), 2) + " "
        NEXT Counter
        LogsRichEdit.AddStrings "SysEx (" + STR$(HdrIN.dwBytesRecorded) + " bytes): " + RecMessage
        LogsRichEdit.AddStrings "midiInAddBuffer again: " + STR$(midiInAddBuffer(hMidiIn, HdrIN, SIZEOF(HdrIN)))
    END IF
END SUB

MainForm.ShowModal
