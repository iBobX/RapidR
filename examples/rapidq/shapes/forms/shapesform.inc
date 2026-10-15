' shapesform.inc: the Shapes example's window. Pick a shape, type its
' size, and Area works it out with the library (lib\geometry.inc).
DECLARE SUB ShowArea (Sender AS QBUTTON)

CREATE ShapesForm AS QFORM
    Caption = "Shapes"
    Width = 330: Height = 190
    CREATE ShapeLabel AS QLABEL
        Caption = "Shape:"
        Left = 16: Top = 20
    END CREATE
    CREATE ShapeBox AS QCOMBOBOX
        Left = 100: Top = 16: Width = 196
        AddItems "Circle (radius)", "Square (side)", "Triangle (side)"
        ItemIndex = 0
    END CREATE
    CREATE SizeLabel AS QLABEL
        Caption = "Size in cm:"
        Left = 16: Top = 56
    END CREATE
    CREATE SizeEdit AS QEDIT
        Left = 100: Top = 52: Width = 90
        Text = "2"
    END CREATE
    CREATE AreaButton AS QBUTTON
        Caption = "&Area"
        Left = 212: Top = 50: Width = 84
        OnClick = ShowArea
    END CREATE
    CREATE AnswerLabel AS QLABEL
        Left = 16: Top = 100: Width = 280
        Caption = "Pick a shape and its size, then Area."
    END CREATE
END CREATE

SUB ShowArea (Sender AS QBUTTON)
    DIM Size AS DOUBLE
    Size = VAL(SizeEdit.Text)
    SELECT CASE ShapeBox.ItemIndex
    CASE 0
        AnswerLabel.Caption = "Circle: " + AreaText(CircleArea(Size))
    CASE 1
        AnswerLabel.Caption = "Square: " + AreaText(SquareArea(Size))
    CASE ELSE
        AnswerLabel.Caption = "Triangle: " + AreaText(TriangleArea(Size))
    END SELECT
END SUB
