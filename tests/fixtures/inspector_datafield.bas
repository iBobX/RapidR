' (I7) The inspector's DataField picks a column of the bound data set: a
' designed form (an RCSV in its tray, an REdit bound to it) on an
' RDESIGNSURFACE, the inspector following it — DataField's list is the
' columns the designer read (name, age, city), "age" picked from it.
DECLARE SUB Changed (Prop AS STRING, Value AS STRING)
DECLARE SUB Report


CREATE Form AS RFORM
    Caption = "DataField"
    Width = 640: Height = 480
    CREATE Insp AS RPROPERTYINSPECTOR
        Left = 8: Top = 8: Width = 280: Height = 430
        OnPropertyChange = Changed
    END CREATE
    CREATE Surface AS RDESIGNSURFACE
        Left = 300: Top = 8: Width = 320: Height = 300
    END CREATE
    CREATE Log AS RLABEL
        Left = 300: Top = 320: Width = 320
        Visible = 0
    END CREATE
    CREATE BReport AS RBUTTON
        Left = 300: Top = 410: Caption = "Report"
        OnClick = Report
    END CREATE
END CREATE

SUB Changed (Prop AS STRING, Value AS STRING)
    Log.Caption = Log.Caption + Prop + "=" + Value + " | "
END SUB

SUB Report
    Log.Caption = Log.Caption + "[" + Insp.TargetType + " " + Insp.Value("DataField") + "]"
END SUB

' (the designed form made through the surface's API, as a build without
' RapidR Studio's source reader can)
Surface.AddComponent "RCSV", "People", 0, 0, 0, 0
Surface.AddComponent "REDIT", "NameEdit", 16, 16, 120, 21
Surface.SetProp 0, "Text", "name,age,city"
Surface.SetProp 1, "DataSource", "People"
Surface.SetProp 1, "DataField", "name"
Surface.SelectComp 1
Insp.Designer = Surface
Insp.Filter = "Data"
Form.ShowModal
