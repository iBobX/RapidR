' RPROPERTYINSPECTOR following an RDESIGNSURFACE (Designer = Surface): what
' the user selects on the surface is inspected; a value changed in the
' inspector is the designer's (its GetProp reads it, as the CREATE block
' writes it); the program's own properties (AddProperty) beside them.
DECLARE SUB Changed (Prop AS STRING, Value AS STRING)
DECLARE SUB Report

CREATE Form AS QFORM
    Caption = "Designer"
    Width = 640: Height = 480
    CREATE Insp AS RPROPERTYINSPECTOR
        Left = 8: Top = 8: Width = 280: Height = 430
        OnPropertyChange = Changed
    END CREATE
    CREATE Surface AS RDESIGNSURFACE
        Left = 300: Top = 8: Width = 320: Height = 300
    END CREATE
    CREATE Log AS QLABEL
        Left = 300: Top = 320: Width = 320
        Visible = 0
    END CREATE
    CREATE BReport AS QBUTTON
        Left = 300: Top = 410: Caption = "Report"
        OnClick = Report
    END CREATE
END CREATE

SUB Changed (Prop AS STRING, Value AS STRING)
    Log.Caption = Log.Caption + Prop + "=" + Value + " | "
END SUB

SUB Report
    Log.Caption = Log.Caption + "[" + Insp.TargetType + " " + Surface.GetProp(0, "Caption") + " " + Surface.GetProp(0, "Anchors") + " " + Surface.GetProp(1, "Hint") + " " + Insp.Value("Speed") + "]"
END SUB

Surface.AddComponent "QBUTTON", "OkButton", 16, 16, 80, 24
Surface.AddComponent "QEDIT", "NameEdit", 16, 56, 120, 21
Surface.SelectComp 0
Insp.Designer = Surface
Insp.AddProperty "Speed", "int", "5", "Engine"
Form.ShowModal
