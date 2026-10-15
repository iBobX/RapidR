' main.bas: a RapidQ program whose library is in a shared include folder
' (tests/fixtures/rapidq_import_lib, Studio's Settings > Include folders):
' IMPORT-INC's flows. Without that folder its include is reported missing.
$INCLUDE "RAPIDQ.INC"
$INCLUDE "Inc\Tools.INC"

CREATE Form AS QFORM
    Caption = ToolsTitle
    CREATE Bar AS QPANEL
        Align = alTop
    END CREATE
END CREATE
