' skip-on: win32 — there the message is sent (docs/windows-dll-calls.md §1)
' RapidQ's SENDMESSAGE / POSTMESSAGE / KILLMESSAGE are Windows' own
' message functions: the program compiles everywhere and, off Windows, stops
' at the first one with the error that names it
CREATE Form AS QFORM
END CREATE
PRINT "before"
SENDMESSAGE Form.Handle, &H112, &HF020, 0
PRINT "not reached"
