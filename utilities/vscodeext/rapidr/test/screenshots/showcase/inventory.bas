' Inventory: a RapidQ program, edited in VS Code with RapidR
$APPTYPE GUI
$TYPECHECK ON
$INCLUDE "RAPIDQ.INC"
$INCLUDE "stock.inc"

DIM total AS DOUBLE

CREATE Form AS QFORM
    Caption = "Inventory"
    Width = 420
    Height = 300
    CREATE ItemList AS QLISTBOX
        Left = 12
        Top = 12
        Width = 260
        Height = 230
    END CREATE
    CREATE AddButton AS QBUTTON
        Caption = "&Add"
        Left = 288
        Top = 12
        OnClick = AddItem
    END CREATE
END CREATE

SUB AddItem
    DIM item AS STRING
    item = "Item " + STR$(ItemList.ItemCount + 1)
    ItemList.AddItems item
    total = total + PriceOf(item, 2)
    Form.Caption = "Inventory: " + STR$(total)
END SUB

Form.ShowModal
