' notepad.bas: a small text editor written the way RapidQ programs are —
' $TYPECHECK, DECLAREd SUBs, CREATE blocks of Q components, a QRICHEDIT,
' open / save dialogs, a status bar with panels. Nothing in it is
' RapidR's own: it is RapidQ code, and RapidR runs it as it is.
'
'   rapidr run notepad.bas
$TYPECHECK ON
$APPTYPE GUI
$INCLUDE "RAPIDQ.INC"

DECLARE SUB FileNew
DECLARE SUB FileOpen
DECLARE SUB FileSave
DECLARE SUB FileExit
DECLARE SUB EditChanged
DECLARE SUB ShowCounts
DECLARE SUB FormClose (Action AS INTEGER)

DIM FileName AS STRING
DIM Dirty AS INTEGER

CREATE OpenDialog AS QOPENDIALOG
    Filter = "Text files (*.txt)|*.txt|All files (*.*)|*.*"
END CREATE
CREATE SaveDialog AS QSAVEDIALOG
    Filter = "Text files (*.txt)|*.txt|All files (*.*)|*.*"
END CREATE

CREATE Form AS QFORM
    Caption = "Notepad"
    Width = 480
    Height = 340
    Center
    OnClose = FormClose
    CREATE MainMenu AS QMAINMENU
        CREATE FileMenu AS QMENUITEM
            Caption = "&File"
            CREATE NewItem AS QMENUITEM
                Caption = "&New": ShortCut = "Ctrl+N": OnClick = FileNew
            END CREATE
            CREATE OpenItem AS QMENUITEM
                Caption = "&Open...": ShortCut = "Ctrl+O": OnClick = FileOpen
            END CREATE
            CREATE SaveItem AS QMENUITEM
                Caption = "&Save...": ShortCut = "Ctrl+S": OnClick = FileSave
            END CREATE
            CREATE Break1 AS QMENUITEM
                Caption = "-"
            END CREATE
            CREATE ExitItem AS QMENUITEM
                Caption = "E&xit": OnClick = FileExit
            END CREATE
        END CREATE
    END CREATE
    CREATE Editor AS QRICHEDIT
        Align = alClient
        ScrollBars = ssVertical
        PlainText = 1
        OnChange = EditChanged
    END CREATE
    CREATE Status AS QSTATUSBAR
        AddPanels "", ""
        Panel(0).Width = 300
    END CREATE
END CREATE

SUB ShowCounts
    DIM Words AS INTEGER, I AS INTEGER, InWord AS INTEGER, C AS STRING
    FOR I = 1 TO LEN(Editor.Text)
        C = MID$(Editor.Text, I, 1)
        IF C = " " OR C = CHR$(13) OR C = CHR$(10) OR C = CHR$(9) THEN
            InWord = 0
        ELSEIF InWord = 0 THEN
            InWord = 1
            Words = Words + 1
        END IF
    NEXT I
    Status.Panel(1).Caption = STR$(Editor.LineCount) + " lines, " + STR$(Words) + " words"
END SUB

SUB EditChanged
    Dirty = 1
    ShowCounts
END SUB

SUB FileNew
    Editor.Clear
    FileName = ""
    Dirty = 0
    Form.Caption = "Notepad - untitled"
    Status.Panel(0).Caption = "New file"
    ShowCounts
END SUB

SUB FileOpen
    IF OpenDialog.Execute THEN
        Editor.LoadFromFile(OpenDialog.FileName)
        FileName = OpenDialog.FileName
        Dirty = 0
        Form.Caption = "Notepad - " + FileName
        Status.Panel(0).Caption = "Opened " + FileName
        ShowCounts
    END IF
END SUB

SUB FileSave
    SaveDialog.FileName = FileName
    IF SaveDialog.Execute THEN
        Editor.SaveToFile(SaveDialog.FileName)
        FileName = SaveDialog.FileName
        Dirty = 0
        Form.Caption = "Notepad - " + FileName
        Status.Panel(0).Caption = "Saved " + FileName
    END IF
END SUB

SUB FileExit
    Form.Close
END SUB

SUB FormClose (Action AS INTEGER)
    ' (Action = caNone keeps the form open)
    IF Dirty THEN
        IF MESSAGEDLG("Quit without saving?", mtConfirmation, mbYes OR mbNo, 0) = mrNo THEN Action = caNone
    END IF
END SUB

Editor.Text = "Welcome to RapidR." + CHR$(13) + CHR$(10) + "This program is plain RapidQ code." + CHR$(13) + CHR$(10) + "Save it, start a new one, open it again."
Dirty = 0
Form.Caption = "Notepad - untitled"
Status.Panel(0).Caption = "Ready"
ShowCounts
Form.ShowModal
