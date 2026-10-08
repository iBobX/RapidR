' RPROPERTYINSPECTOR's looks, for tools/panels_gallery.mjs: by categories
' with the anchors editor and a colour picker open; A to Z, searched, with
' RapidR's extensions badged; the Events page with SUBs bound.
CREATE Form AS QFORM
    Caption = "Inspector gallery"
    Width = 980: Height = 600
    ' (the inspected components, under the inspectors)
    CREATE Button1 AS QBUTTON
        Left = 20: Top = 20: Caption = "Save"
        Color = &H00D7FF
        Anchors = akLeft + akTop + akRight
        Hint = "Saves the file"
    END CREATE
    CREATE Edit1 AS QEDIT
        Left = 20: Top = 60
    END CREATE
    CREATE Insp AS RPROPERTYINSPECTOR
        Left = 8: Top = 8: Width = 310: Height = 550
    END CREATE
    CREATE Insp2 AS RPROPERTYINSPECTOR
        Left = 328: Top = 8: Width = 310: Height = 550
        View = "alphabetic"
    END CREATE
    CREATE Insp3 AS RPROPERTYINSPECTOR
        Left = 648: Top = 8: Width = 310: Height = 270
        Page = "events"
    END CREATE
    CREATE Insp4 AS RPROPERTYINSPECTOR
        Left = 648: Top = 288: Width = 310: Height = 270
        ShowEvents = 0
    END CREATE
END CREATE

Insp.Target = "Button1"
Insp.Expand "Anchors"
Insp.Collapse "Appearance"
Insp.Collapse "Behavior"
Insp.Collapse "Help"
Insp.Selected = "Anchors"

Insp2.Target = "Button1"
Insp2.Filter = "co"
Insp2.Expand "Color"
Insp2.Selected = "Color"

Button1.OnClick = "SaveClick"
Insp3.Target = "Button1"
Insp3.Selected = "OnClick"

Insp4.Target = "Button1, Edit1"
Insp4.AddProperty "Speed", "int", "5", "Engine"
Insp4.AddProperty "Mode", "fast|slow|off", "fast", "Engine"
Insp4.SetValue "Mode", "slow"
Insp4.AddProperty "Lines", "strings", "first" + CHR$(10) + "second", "Engine"
Insp4.Expand "Lines"
Insp4.Collapse "Appearance"
Insp4.Collapse "Behavior"
Form.ShowModal
