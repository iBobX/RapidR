' QCOMPORT's line events on the tests' scripted port (RAPIDR_TEST_COMPORT:
' COM5 echoes, rings once and sends a break once after it opens — never a
' real device): a write's OnWriteString, then OnTxEmpty (everything sent);
' then, looked for as OnRxChar is, OnBreak, OnRing and OnRxChar (the echo).
ENVIRON "RAPIDR_TEST_COMPORT=COM5:echo+ring+break"
DECLARE SUB Go
DECLARE SUB Opened
DECLARE SUB Written
DECLARE SUB TxEmpty
DECLARE SUB Rang
DECLARE SUB Broke
DECLARE SUB RxChar (InQue AS INTEGER)
DIM Log AS STRING
DIM Port AS QCOMPORT
Port.Port = 5
Port.OnOpen = Opened
Port.OnWriteString = Written
Port.OnTxEmpty = TxEmpty
Port.OnRing = Rang
Port.OnBreak = Broke
Port.OnRxChar = RxChar
CREATE Form AS QFORM
  Caption = "comport events"
  Width = 320
  Height = 160
  CREATE Btn AS QBUTTON
    Caption = "go"
    Left = 10
    Top = 10
    OnClick = Go
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10
    Top = 50
    Width = 290
    Caption = "-"
  END CREATE
END CREATE

SUB Note(S AS STRING)
  Log = Log + " " + S
  Lbl.Caption = Log
END SUB

SUB Go
  Port.Open
  Port.WriteString("hi", 0)
END SUB

SUB Opened
  Note "open"
END SUB

SUB Written
  Note "written"
END SUB

SUB TxEmpty
  Note "txempty"
END SUB

SUB Rang
  Note "ring"
END SUB

SUB Broke
  Note "break"
END SUB

SUB RxChar (InQue AS INTEGER)
  Note "rx" + STR$(InQue)
END SUB

Form.ShowModal
