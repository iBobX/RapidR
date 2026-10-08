' RapidQ's console pages (manual chapter 6.3, appendix C): PEEK and POKE
' read and write the 80 x 25 screen as QBasic's screen memory — a cell's
' character at (Row - 1) * 160 + (Col - 1) * 2, its attribute (background
' * 16 + foreground) at the next address — on page 0, the screen, or an
' off-screen page 1 to 7 given first; PCOPY copies one page onto another.
' What's printed is on page 0 in the colours COLOR set (RapidQ's 3DBOX
' example POKEs the last cell's attribute: POKE 3999, 3 * 16).
CLS
PRINT "Hi"
LOCATE 3, 5
COLOR 14, 1
PRINT "Q";
COLOR
LOCATE 10, 1
PRINT CHR$(PEEK(0)); CHR$(PEEK(2)); PEEK(1); PEEK(3 * 2 - 1)
PRINT CHR$(PEEK(2 * 160 + 4 * 2)); PEEK(2 * 160 + 4 * 2 + 1)
' (off screen: nothing shows; PCOPY to another off-screen page)
POKE 1, 0, ASC("Z")
POKE 1, 1, 79
PCOPY 1, 2
PRINT CHR$(PEEK(1, 0)); PEEK(1, 1); CHR$(PEEK(2, 0)); PEEK(2, 1); CHR$(PEEK(0))
' (page 0: shown at once, the cursor and colours put back)
POKE 3999, 3 * 16
POKE 3998, ASC("!")
PRINT PEEK(3999); CHR$(PEEK(3998)); CSRLIN; POS(0)
' (outside a page: nothing)
POKE 4000, 1
PRINT PEEK(4000); PEEK(9, 0)
