' Screen.Scale / Form.Scale (RapidR's): device pixels per pixel — the
' same for a form as for its screen, never below 1 here; PixelsPerInch stays
' RapidQ's 96 whatever the screen.
DECLARE SUB Clicked
CREATE f AS QFORM
  CREATE btn AS QBUTTON
    OnClick = Clicked
  END CREATE
  CREATE lbl AS QLABEL
    Top = 40 : Width = 300
  END CREATE
END CREATE
f.ShowModal

SUB Clicked
  lbl.Caption = STR$(Screen.PixelsPerInch) + " " + STR$(f.Scale = Screen.Scale) + " " + STR$(Screen.Scale >= 1)
END SUB
