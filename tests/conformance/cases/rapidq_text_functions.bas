' VAL, HEX$, BIN$ and REPLACE$ as RapidQ has them — the .expected is
' RC.EXE's output (docs/rapidq-ground-truth.md): VAL skips spaces anywhere
' and reads the longest number at the start; HEX$ is 8 digits (32 bits, a
' real rounded half to even); BIN$ the 32 bits without leading zeros;
' REPLACE$ writes over from a position, before the start it goes in front,
' past the end it is appended.
PRINT "a "; VAL("2.5"); " "; VAL("abc"); " "; VAL("12abc"); " "; VAL(" 3"); " "; VAL("1e3"); " "; VAL("&H10"); " "; VAL("-.5")
PRINT "b "; VAL("3."); " "; VAL("1,5"); " "; VAL("  -4  "); " "; VAL("+7"); " "; VAL("1.5e"); " "; VAL("0x10"); " "; VAL("12 34")
PRINT "c "; VAL("1.2.3"); " "; VAL("- 5"); " "; VAL("1 e 2"); " "; VAL(".5.5"); " "; VAL("1d2"); " "; VAL("$5"); " "; VAL("  ")
PRINT "d "; VAL("12abc34"); " "; VAL("1-2"); " "; VAL("1E+2"); " "; VAL("9e-2"); " "; VAL("1e"); " "; VAL("."); " "; VAL("1."); " "; VAL("-.5e1")
PRINT "e ["; HEX$(-1); "] ["; HEX$(255); "] ["; HEX$(2.7); "] ["; HEX$(3000000000); "] ["; HEX$(2.5); "] ["; HEX$(3.5); "]"
PRINT "f ["; HEX$(1E10); "] ["; HEX$(-2.5); "] ["; HEX$(65535); "] ["; HEX$(1E20); "]"
PRINT "g ["; BIN$(5); "] ["; BIN$(-1); "]"
PRINT "h ["; REPLACE$("Hello", "J", 1); "] ["; REPLACE$("abcdef", "XY", 2); "] ["; REPLACE$("abc", "XYZ", 3); "] ["; REPLACE$("abc", "Z", 9); "]"
PRINT "i ["; REPLACE$("abc", "Z", 4); "] ["; REPLACE$("abc", "Z", 0); "] ["; REPLACE$("abc", "Z", -1); "] ["; REPLACE$("abc", "XY", 3); "]"
