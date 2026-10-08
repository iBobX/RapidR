' PEEK / POKE / PCOPY on the console's pages (RapidQ manual 6.3, checked
' with RC.EXE on screen: docs/windows-dll-calls.md §2): page 0 is the
' screen — even addresses the character, odd the attribute, a blank cell
' 32 and 7 — pages 1 to 7 off-screen buffers that start as zeros.
' (The row comes from CSRLIN: a test harness may have printed a line first.)
r = CSRLIN
PRINT "Hello"
b = (r - 1) * 160
PRINT PEEK(b); " "; PEEK(b + 1); " "; PEEK(b + 2); " "; PEEK(b + 1132); " "; PEEK(3999)
PRINT PEEK(1, 0); " "; PEEK(#1, 1)
POKE 1, 0, 66: POKE #1, 1, 95
PRINT PEEK(1, 0); " "; PEEK(1, 1)
PCOPY 1, 3
PRINT PEEK(3, 0); " "; PEEK(2, 0)
PCOPY 0, 7
PRINT CHR$(PEEK(7, b)); CHR$(PEEK(7, b + 2)); CHR$(PEEK(7, b + 4)); " "; PEEK(7, b + 5)
