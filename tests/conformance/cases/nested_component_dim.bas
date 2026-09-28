' Components DIMmed inside a block (IF / FOR), in the main program and in a
' SUB, are created by their DIM (RapidQ's rqb2html DIMs its QOPENDIALOG in an IF).
DECLARE SUB Inner
IF 1 THEN
  DIM L AS QLABEL
  L.Caption = "main-if"
  PRINT L.Caption
END IF
Inner
SUB Inner
  FOR i = 1 TO 1
    DIM M AS QLABEL
    M.Caption = "sub-for"
    PRINT M.Caption
  NEXT
END SUB
