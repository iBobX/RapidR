' RapidR Studio's panels (I1 / L-PANELS) on one form: each one made, placed
' and drawn by the UI kernel on every runtime.
CREATE Form AS QFORM
    Caption = "Panels"
    Width = 900: Height = 600
    CREATE Bar AS RTOOLBAR
        Align = 1
    END CREATE
    CREATE Insp AS RPROPERTYINSPECTOR
        Left = 0: Top = 40: Width = 280: Height = 520
    END CREATE
    CREATE Box AS RTOOLBOX
        Left = 290: Top = 40: Width = 200: Height = 300
    END CREATE
    CREATE Tree AS RPROJECTTREE
        Left = 500: Top = 40: Width = 200: Height = 300
    END CREATE
    CREATE Cons AS ROUTPUTCONSOLE
        Left = 290: Top = 350: Width = 580: Height = 200
    END CREATE
    CREATE Pal AS RCOMMANDPALETTE
    END CREATE
    CREATE Lbl AS QLABEL
        Left = 710: Top = 40: Width = 180
    END CREATE
END CREATE
Lbl.Caption = STR$(Insp.Width) + STR$(Pal.Visible)
Form.ShowModal
