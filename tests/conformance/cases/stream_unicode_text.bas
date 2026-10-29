' A stream written with text that is not Latin-1 keeps every character
' (Solar System 3D, 2.121.3: Studio wrote its translation files with
' F.WriteStr(Text, LEN(Text)); the Chinese came out cut short or as `?`,
' and an accent beside it as one byte, in a file that was no UTF-8).
'
' RapidR's rule (docs/manual/differences.md): a string of characters up to
' 255 is bytes, one each, as RapidQ's — WriteStr(CHR$(200)) writes byte 200;
' a string with any other character is text and is written whole as UTF-8.
' WriteStr's count is of characters, so WriteStr(S, LEN(S)) is all of S.
' WriteText always writes UTF-8; ReadAllText reads UTF-8 (or Windows-1252).
$INCLUDE "RAPIDQ.INC"
DIM M AS QMEMORYSTREAM
DIM F AS QFILESTREAM
DIM S AS STRING
DIM T AS STRING
DIM I AS INTEGER

SUB Bytes (Tag AS STRING)
  DIM J AS INTEGER
  M.Position = 0
  PRINT Tag; " size"; M.Size; " [";
  FOR J = 1 TO M.Size
    PRINT HEX$(M.ReadByte AND 255); " ";
  NEXT
  PRINT "]"
END SUB

' Chinese, an accent, a dot and a euro sign: 22 bytes of UTF-8.
S = "Español 中文 · €"
PRINT "chars"; LEN(S)

M.WriteStr(S, LEN(S))
Bytes("count ")
M.Position = 0
T = M.ReadAllText
PRINT "readalltext same: "; IIF(T = S, "yes", "no"); " chars"; LEN(T)

' the count is of characters
M.Close
M.WriteStr("中文字", 2)
Bytes("two   ")

' no count, and WriteText: the same bytes
M.Close
M.WriteStr(S)
M.WriteText("é")
Bytes("plain ")

' a string of byte characters stays bytes (the accent is one byte)
M.Close
M.WriteStr("A" + CHR$(200) + CHR$(255) + "é", 4)
Bytes("bytes ")
M.Position = 0
T = M.ReadStr(4)
PRINT "readstr same: "; ASC(MID$(T, 2, 1)); " "; ASC(MID$(T, 4, 1))

' a file stream the same
F.Open("tests/conformance/.work/stream_unicode_text.dat", fmCreate)
F.WriteStr(S, LEN(S))
PRINT "file size"; F.Size
F.Position = 0
T = F.ReadAllText
PRINT "file same: "; IIF(T = S, "yes", "no")
F.Close
