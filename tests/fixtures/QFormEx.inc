' A stand-in, written for RapidR's tests, for the QFormEx a common RapidQ
' community include (Rapidq2.inc) defines: a TYPE with the same public
' members (the same names, kinds and event parameters), so RapidR takes it
' for that one and makes the form an RFormEx written QFormEx (it behaves as
' the include's form: rapidr_ast::formex). Its bodies are placeholders: a
' program that included it under RapidQ would get a form that does nothing
' more than a QFORM.
TYPE FxStandInPoint
    X AS LONG
    Y AS LONG
END TYPE

DECLARE SUB FxStandIn_File (File AS STRING)
DECLARE SUB FxStandIn_None
DECLARE SUB FxStandIn_Wheel (Rotation AS INTEGER, X AS LONG, Y AS LONG, Keys AS INTEGER)
DECLARE FUNCTION FxStandIn_Message (W AS INTEGER, M AS INTEGER, A AS INTEGER, B AS INTEGER) AS LONG

TYPE QFormEx EXTENDS QFORM
  PRIVATE:
    Busy                AS INTEGER
    Spare               AS QFORM

  PUBLIC:
    DragZone            AS QRECT
    DragEnable          AS INTEGER PROPERTY SET SetDragEnable
    DropPoint           AS FxStandInPoint
    FormStyle           AS INTEGER PROPERTY SET SetFormStyle
    TrayICO             AS QIMAGELIST
    TrayIconHint        AS STRING  PROPERTY SET SetTrayIconHint
    TrayIconIndex       AS INTEGER PROPERTY SET SetTrayIconIndex
    TrayIconUpdate      AS INTEGER PROPERTY SET SetTrayIconUpdate
    Transparent         AS INTEGER PROPERTY SET SetTransparent
    TransparentClient   AS INTEGER PROPERTY SET SetTransparentClient

    OnDrag              AS EVENT(FxStandIn_File)
    OnTrayClick         AS EVENT(FxStandIn_None)
    OnTrayDblClick      AS EVENT(FxStandIn_None)
    OnMinimise          AS EVENT(FxStandIn_None)
    OnMouseWheel        AS EVENT(FxStandIn_Wheel)
    OnLostFocus         AS EVENT(FxStandIn_None)
    OnGetFocus          AS EVENT(FxStandIn_None)
    OnMessage           AS EVENT(FxStandIn_Message)

  PRIVATE:
    SUB Note
        QFormEx.Busy = 1
    END SUB

  PUBLIC:
    SUB RemoveTaskBar
        QFormEx.Note
    END SUB
    PROPERTY SET SetFormStyle(Style AS INTEGER)
        QFormEx.FormStyle = Style
    END PROPERTY
    PROPERTY SET SetDragEnable(Flag AS INTEGER)
        QFormEx.DragEnable = Flag
    END PROPERTY
    SUB AddTrayIcon
        QFormEx.Note
    END SUB
    SUB DelTrayIcon
        QFormEx.Note
    END SUB
    SUB ModifyTrayIcon
        QFormEx.Note
    END SUB
    PROPERTY SET SetTrayIconHint(Text AS STRING)
        QFormEx.TrayIconHint = Text
    END PROPERTY
    PROPERTY SET SetTrayIconUpdate(Ms AS INTEGER)
        QFormEx.TrayIconUpdate = Ms
    END PROPERTY
    SUB SetTrayIconIndex(Index AS INTEGER)
        QFormEx.TrayIconIndex = Index
    END SUB
    SUB Center
        QFormEx.Note
    END SUB
    PROPERTY SET SetTransparentClient(Value AS INTEGER)
        QFormEx.TransparentClient = Value
    END PROPERTY
    PROPERTY SET SetTransparent(Value AS INTEGER)
        QFormEx.Transparent = Value
    END PROPERTY

    EVENT OnShow
        QFormEx.Note
    END EVENT
    EVENT OnClose
        QFormEx.Note
    END EVENT
END TYPE
