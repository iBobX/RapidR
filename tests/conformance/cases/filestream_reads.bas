' QFILESTREAM's ReadStr, Read(S$) and ReadLine against QMEMORYSTREAM's, as
' RapidQ does them: the .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md). A file stream's ReadStr(n) and Read(S$)
' give one more character than asked, a space (ReadStr(0) too, a negative
' count nothing); a memory stream's give n; ReadBinStr gives n on both.
' ReadLine (both): up to the next LF, less one CR right before it; a NUL
' before the LF ends the text and sends Position to the end. LineCount
' counts the LFs.
$INCLUDE "RAPIDQ.INC"
DIM F AS QFILESTREAM
DIM M AS QMEMORYSTREAM
DIM S AS STRING
DIM D AS STRING

SUB Show (Tag AS STRING, T AS STRING, Q AS LONG)
  DIM J AS INTEGER
  PRINT Tag; " len"; LEN(T); " pos"; Q; " [";
  FOR J = 1 TO LEN(T)
    PRINT ASC(MID$(T, J, 1)); " ";
  NEXT
  PRINT "]"
END SUB

SUB Lines (Tag AS STRING, D AS STRING)
  DIM K AS INTEGER
  M.Close
  M.WriteStr(D, LEN(D))
  M.Position = 0
  FOR K = 1 TO 3
    S = M.ReadLine
    Show(Tag + "m" + STR$(K), S, M.Position)
  NEXT
  F.Open("tests/conformance/.work/filestream_lines.dat", fmCreate)
  F.WriteStr(D, LEN(D))
  F.Position = 0
  FOR K = 1 TO 3
    S = F.ReadLine
    Show(Tag + "f" + STR$(K), S, F.Position)
  NEXT
  PRINT Tag; " lines m"; M.LineCount; " f"; F.LineCount
  F.Close
END SUB

D = "ab" + CHR$(0) + "cd" + CHR$(13) + CHR$(10) + "ef" + CHR$(255) + "g" + CHR$(10) + "last"
F.Open("tests/conformance/.work/filestream_reads.dat", fmCreate)
F.WriteStr(D, LEN(D))
F.Close
M.WriteStr(D, LEN(D))

F.Open("tests/conformance/.work/filestream_reads.dat", fmOpenRead)
F.Position = 0: S = F.ReadStr(0): Show("f0", S, F.Position)
F.Position = 0: S = F.ReadStr(-2): Show("fneg", S, F.Position)
F.Position = 0: S = F.ReadStr(1): Show("f1", S, F.Position)
F.Position = 0: S = F.ReadStr(3): Show("f3", S, F.Position)
F.Position = 0: S = F.ReadStr(5): Show("f5", S, F.Position)
M.Position = 0: S = M.ReadStr(5): Show("m5", S, M.Position)
M.Position = 0: S = M.ReadStr(0): Show("m0", S, M.Position)
F.Position = 0: S = F.ReadStr(F.Size): Show("fall", S, F.Position)
F.Position = 0: S = F.ReadStr(F.Size + 3): Show("fmore", S, F.Position)
S = F.ReadStr(2): Show("fend", S, F.Position)
F.Position = 30: S = F.ReadStr(2): Show("fpast", S, F.Position)
F.Position = 1: S = "xyz": F.Read(S): Show("fread", S, F.Position)
M.Position = 1: S = "xyz": M.Read(S): Show("mread", S, M.Position)
F.Position = 0: S = "": F.Read(S): Show("fread0", S, F.Position)
F.Position = 3: S = F.ReadLine: Show("fline", S, F.Position)
S = F.ReadStr(3): Show("fafter", S, F.Position)
M.Position = 3: S = M.ReadLine: Show("mline", S, M.Position)
S = M.ReadStr(3): Show("mafter", S, M.Position)
F.Position = 0: S = F.ReadBinStr(3): Show("fbin", S, F.Position)
M.Position = 0: S = M.ReadBinStr(3): Show("mbin", S, M.Position)
F.Close

Lines("a", "ab" + CHR$(10) + "cd" + CHR$(0) + "ef" + CHR$(10) + "gh")
Lines("b", "ab" + CHR$(0) + CHR$(10) + "cd")
Lines("c", CHR$(0) + "abc" + CHR$(10) + "def")
Lines("d", "ab" + CHR$(13) + "cd" + CHR$(10) + "ef" + CHR$(13) + CHR$(10) + "gh" + CHR$(13))
Lines("e", "one" + CHR$(10) + CHR$(10) + "three")
Lines("f", "x" + CHR$(13) + CHR$(13) + CHR$(10) + "y")
