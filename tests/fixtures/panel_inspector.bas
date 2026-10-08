' RPROPERTYINSPECTOR (I1 / L-PANELS): Delphi's object inspector on the
' program's own components. Insp inspects Button1 (a QBUTTON); the log shows
' what it heard (OnPropertyChange, OnSelect, OnEventDblClick); the buttons
' at the bottom inspect the form, two buttons at once, the Events page.
DECLARE SUB Changed (Prop AS STRING, Value AS STRING)
DECLARE SUB Picked (Prop AS STRING)
DECLARE SUB Dbl (Event AS STRING)
DECLARE SUB AskEditor (Prop AS STRING)
DECLARE SUB InspectForm
DECLARE SUB InspectBoth
DECLARE SUB Report
DECLARE SUB Button1Click (Sender AS QBUTTON)
DECLARE SUB AnyClick

CREATE Form AS QFORM
    Caption = "Inspector"
    Width = 620: Height = 520
    Center
    CREATE Insp AS RPROPERTYINSPECTOR
        Left = 8: Top = 8: Width = 300: Height = 470
        Handlers = "Button1Click(Sender AS QBUTTON)" + CHR$(10) + "AnyClick" + CHR$(10) + "KeyDown(Key AS WORD, Shift AS INTEGER)"
        OnPropertyChange = Changed
        OnSelect = Picked
        OnEventDblClick = Dbl
        OnEditorRequest = AskEditor
    END CREATE
    CREATE Button1 AS QBUTTON
        Left = 330: Top = 20: Width = 120: Caption = "Button1"
    END CREATE
    CREATE Button2 AS QBUTTON
        Left = 330: Top = 60: Width = 120: Caption = "Button2"
    END CREATE
    CREATE Log AS QLABEL
        Left = 330: Top = 100: Width = 270
        Visible = 0
        Caption = ""
    END CREATE
    CREATE Status AS QLABEL
        Left = 330: Top = 100: Width = 270
        Caption = "Ready"
    END CREATE
    CREATE BForm AS QBUTTON
        Left = 330: Top = 440: Width = 80: Caption = "Form"
        OnClick = InspectForm
    END CREATE
    CREATE BBoth AS QBUTTON
        Left = 420: Top = 440: Width = 80: Caption = "Both"
        OnClick = InspectBoth
    END CREATE
    CREATE BReport AS QBUTTON
        Left = 510: Top = 440: Width = 80: Caption = "Report"
        OnClick = Report
    END CREATE
END CREATE

SUB Changed (Prop AS STRING, Value AS STRING)
    Log.Caption = Log.Caption + "change " + Prop + "=" + Value + " | "
    Status.Caption = Prop + " = " + Value
END SUB

SUB Picked (Prop AS STRING)
    Log.Caption = Log.Caption + "sel " + Prop + " | "
END SUB

SUB Dbl (Event AS STRING)
    Log.Caption = Log.Caption + "dbl " + Event + " | "
END SUB

SUB AskEditor (Prop AS STRING)
    Log.Caption = Log.Caption + "editor " + Prop + " | "
END SUB

SUB InspectForm
    Insp.Target = "Form"
END SUB

SUB InspectBoth
    Insp.Target = "Button1, Button2"
END SUB

SUB Report
    Log.Caption = Log.Caption + "[" + Insp.TargetType + " caption=" + Insp.Value("Caption") + " anchors=" + Insp.Value("Anchors") + STR$(Insp.IsDefault("Anchors")) + " page=" + Insp.Page + " filter=" + Insp.Filter + " rows=" + STR$(Insp.RowCount) + " " + Insp.Row(1) + " nw=" + STR$(Insp.NameWidth) + "] | "
END SUB

SUB Button1Click (Sender AS QBUTTON)
    Log.Caption = Log.Caption + "clicked | "
END SUB

SUB AnyClick
END SUB

Insp.Target = "Button1"
Form.ShowModal
