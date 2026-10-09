' OnDropFiles (RapidR's): files dropped on a form — here by the test hook
' form.__drop, RAPIDR_TEST_DROP's files — come one a line, in order; the
' program opens them as any file. A form without the handler hears nothing
' (Other's drop is ignored). The program writes the files it then "drops".
DECLARE SUB Dropped(Files AS STRING)

CREATE Form AS QFORM
    Caption = "Drop files"
    Width = 320: Height = 160
    OnDropFiles = Dropped
    CREATE Lbl AS QLABEL
        Left = 8: Top = 8: Width = 300
    END CREATE
    CREATE Lbl2 AS QLABEL
        Left = 8: Top = 32: Width = 300
    END CREATE
END CREATE

CREATE Other AS QFORM
    Caption = "No handler"
    Width = 200: Height = 100
END CREATE

SUB Dropped(Files AS STRING)
    DIM f AS STRING, first AS STRING
    Lbl.Caption = STR$(TALLY(Files, CHR$(10)) + 1) + " " + REPLACESUBSTR$(Files, CHR$(10), "|")
    f = FIELD$(Files, CHR$(10), 2)
    OPEN f FOR INPUT AS #1
    LINE INPUT #1, first
    CLOSE #1
    Lbl2.Caption = Lbl2.Caption + "[" + first + "]"
END SUB

OPEN "drop_a.txt" FOR OUTPUT AS #1
PRINT #1, "alpha"
CLOSE #1
OPEN "drop_b.csv" FOR OUTPUT AS #1
PRINT #1, "x,y"
PRINT #1, "1,2"
CLOSE #1
Form.ShowModal
