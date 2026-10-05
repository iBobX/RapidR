' QDIRLISTVIEW for RapidR: a QLISTVIEW that lists a folder — its folders,
' then its files, with their size, type and date — and lets the user walk
' through it. RapidQ programs know it from the include library
' QDirListView.inc (the manual's Appendix A); this is RapidR's own code for
' the same members and behaviour, with no Windows calls and no icon files
' (docs/desktop-host-plan.md, "RapidQ's remaining UI objects"). A program
' gets it when it names QDIRLISTVIEW or includes QDirListView.inc.
$IFNDEF __RAPIDR_QDIRLISTVIEW
$DEFINE __RAPIDR_QDIRLISTVIEW
DECLARE SUB RapidR_DirListView_FileSelect (Tag AS INTEGER)
' (RapidR's own icons: a folder, a page)
CONST RAPIDR_DIRLISTVIEW_FOLDER = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAxNiAxNiI+PHBhdGggZD0iTTEgMy41aDVsMS41IDEuNUgxNXY4LjVIMXoiIGZpbGw9IiNmNWM1NDIiIHN0cm9rZT0iIzljN2ExNiIvPjxwYXRoIGQ9Ik0xIDYuNWgxNCIgc3Ryb2tlPSIjOWM3YTE2Ii8+PC9zdmc+"
CONST RAPIDR_DIRLISTVIEW_FILE = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAxNiAxNiI+PHBhdGggZD0iTTMgMS41aDdsMyAzdjEwSDN6IiBmaWxsPSIjZmZmIiBzdHJva2U9IiM2YjZiNmIiLz48cGF0aCBkPSJNMTAgMS41djNoMyIgZmlsbD0ibm9uZSIgc3Ryb2tlPSIjNmI2YjZiIi8+PHBhdGggZD0iTTUgNy41aDZNNSA5LjVoNk01IDExLjVoNCIgc3Ryb2tlPSIjYTBhMGEwIi8+PC9zdmc+"

TYPE QDirListView EXTENDS QListView
  Directory AS STRING PROPERTY SET RapidR_Set_Directory
  FileName AS STRING
  Mask AS STRING PROPERTY SET RapidR_Set_Mask
  ShowRoot AS INTEGER PROPERTY SET RapidR_Set_ShowRoot
  LargeImageList AS QImageList
  SmallImageList AS QImageList
  ExtensionsList AS QStringList
  dlw_PopupMenu AS QPopupMenu
  dlw_Enter AS QMenuItem
  dlw_BackSpace AS QMenuItem
  OnFileSelect AS EVENT (RapidR_DirListView_FileSelect)
  ColumnsInitialized AS INTEGER

  ' "/" when the folder's path is written with slashes, else "\".
  FUNCTION RapidR_Sep AS STRING
    Result = "\"
    IF INSTR(QDirListView.Directory, "/") THEN Result = "/"
  END FUNCTION

  ' Picks the picture of the row just added: an extension found in
  ' ExtensionsList at place 2 or later shows the picture at that place;
  ' anything else shows picture 1, the plain page.
  SUB SetItemImage (Extension AS STRING)
    DIM wanted AS STRING, slot AS INTEGER, pic AS INTEGER
    wanted = LCASE$(Extension)
    pic = 1
    slot = 2
    WHILE pic = 1 AND slot < QDirListView.ExtensionsList.ItemCount
      IF QDirListView.ExtensionsList.Item(slot) = wanted THEN pic = slot
      slot = slot + 1
    WEND
    QDirListView.Item(QDirListView.ItemCount - 1).ImageIndex = pic
  END SUB

  ' Fills the Size, Type and Date Modified columns of the row just added,
  ' from FileRec (still the file the last DIR$ returned).
  SUB AddFileProperties (Extension AS STRING)
    DIM row AS INTEGER, kb AS DOUBLE, stamp AS STRING, p1 AS INTEGER, p2 AS INTEGER
    row = QDirListView.ItemCount - 1
    ' (whole kilobytes; a smaller file still says 1 KB)
    kb = INT(FileRec.Size / 1024)
    IF kb < 1 THEN kb = 1
    QDirListView.AddSubItem(row, STR$(kb) + " KB")
    QDirListView.AddSubItem(row, UCASE$(Extension) + " File")
    ' (FileRec.Date is month-day-year; the column reads day.month.year,
    ' the month always two digits, then the time)
    stamp = FileRec.Date
    p1 = INSTR(stamp, "-")
    p2 = INSTR(p1 + 1, stamp, "-")
    QDirListView.AddSubItem(row, MID$(stamp, p1 + 1, p2 - p1 - 1) + "." + RIGHT$("0" + LEFT$(stamp, p1 - 1), 2) + "." + RIGHT$(stamp, 4) + " " + FileRec.Time)
  END SUB

  ' One row per sub-folder; "." never, ".." only with ShowRoot.
  SUB LoadDirectories
    DIM entry AS STRING, keep AS INTEGER
    entry = DIR$(QDirListView.Directory + "*.*", &H10)
    WHILE LEN(entry)
      SELECT CASE entry
        CASE "."
          keep = 0
        CASE ".."
          keep = QDirListView.ShowRoot
        CASE ELSE
          keep = 1
      END SELECT
      IF keep THEN
        IF DIREXISTS(QDirListView.Directory + entry) THEN QDirListView.AddItems entry
      END IF
      entry = DIR$
    WEND
  END SUB

  ' One row per file the Mask lets through, with its picture and columns.
  SUB LoadFiles
    DIM entry AS STRING, ext AS STRING, dot AS INTEGER
    entry = DIR$(QDirListView.Directory + QDirListView.Mask, 0)
    WHILE LEN(entry)
      dot = RINSTR(entry, ".")
      IF dot THEN ext = MID$(entry, dot + 1) ELSE ext = ""
      QDirListView.AddItems entry
      IF dot THEN
        QDirListView.SetItemImage(ext)
      ELSE
        ' (no extension at all: the plain page)
        QDirListView.Item(QDirListView.ItemCount - 1).ImageIndex = 1
      END IF
      QDirListView.AddFileProperties(ext)
      entry = DIR$
    WEND
  END SUB

  ' Lists Directory again from scratch. The four columns are made on the
  ' first listing only, so a program's own column changes stay.
  SUB Refresh
    QDirListView.Clear
    QDirListView.LoadDirectories
    QDirListView.LoadFiles
    IF QDirListView.ColumnsInitialized THEN EXIT SUB
    QDirListView.ColumnsInitialized = 1
    QDirListView.ClearColumns
    QDirListView.AddColumns "Name", "Size", "Type", "Date Modified"
    QDirListView.Column(0).Width = 150
    QDirListView.Column(1).Width = 60
    QDirListView.Column(2).Width = 60
    QDirListView.Column(3).Width = 100
  END SUB

  ' Goes to the parent folder: the path cut after the separator that comes
  ' before its trailing one. A path with no such separator (a drive's or
  ' the file system's top) stays where it is.
  SUB ChangeDirUp
    DIM path AS STRING, sep AS STRING, k AS INTEGER, cut AS INTEGER
    path = QDirListView.Directory
    sep = QDirListView.RapidR_Sep
    cut = 0
    FOR k = LEN(path) - 1 TO 1 STEP -1
      IF MID$(path, k, 1) = sep THEN
        cut = k
        EXIT FOR
      END IF
    NEXT
    IF cut = 0 THEN EXIT SUB
    ' (stored directly — a method's store doesn't run the PROPERTY SET —
    ' so the listing is asked for here)
    QDirListView.Directory = LEFT$(path, cut)
    QDirListView.Refresh
  END SUB

  ' Opens the folder FileName names; the selected row being ".." means up.
  SUB ChangeDir
    IF QDirListView.Item(QDirListView.ItemIndex).Caption <> ".." THEN
      QDirListView.Directory = QDirListView.FileName + QDirListView.RapidR_Sep
      QDirListView.Refresh
    ELSE
      QDirListView.ChangeDirUp
    END IF
  END SUB

  ' What Enter and a double click do: a folder is opened, a file is handed
  ' to the program (OnFileSelect); with nothing chosen, nothing happens.
  SUB TryToChangeDirOrMakeEvent
    IF LEN(QDirListView.FileName) = 0 THEN EXIT SUB
    IF DIREXISTS(QDirListView.FileName) = 0 THEN
      CALLFUNC(QDirListView.OnFileSelect, 0)
    ELSE
      QDirListView.ChangeDir
    END IF
  END SUB

  ' FileName follows the selection: the folder's path plus the row's name,
  ' or "" with no row selected. (A program's own OnChange calls
  ' InheritOnChange to keep this.)
  EVENT OnChange
    DIM sel AS INTEGER
    sel = QDirListView.ItemIndex
    QDirListView.FileName = ""
    IF sel > -1 THEN QDirListView.FileName = QDirListView.Directory + QDirListView.Item(sel).Caption
  END EVENT

  EVENT OnDblClick
    QDirListView.TryToChangeDirOrMakeEvent
  END EVENT

  ' Enter opens (or picks), Backspace goes up — the keys RapidQ's
  ' component gives the list through its hidden menu items.
  EVENT OnKeyDown (Key AS WORD, Shift AS INTEGER)
    SELECT CASE Key
      CASE 13
        QDirListView.TryToChangeDirOrMakeEvent
      CASE 8
        QDirListView.ChangeDirUp
    END SELECT
  END EVENT

  ' A folder set by the program is kept with a separator at its end (the
  ' kind its path already uses) and listed at once.
  PROPERTY SET RapidR_Set_Directory (NewDir AS STRING)
    DIM path AS STRING, last AS STRING
    path = NewDir
    last = RIGHT$(path, 1)
    IF last <> "/" AND last <> "\" THEN
      IF INSTR(path, "/") THEN path = path + "/" ELSE path = path + "\"
    END IF
    QDirListView.Directory = path
    QDirListView.Refresh
  END PROPERTY

  ' (ShowRoot and Mask change what's listed: listed again)
  PROPERTY SET RapidR_Set_ShowRoot (NewShowRoot AS INTEGER)
    QDirListView.ShowRoot = NewShowRoot
    QDirListView.Refresh
  END PROPERTY

  PROPERTY SET RapidR_Set_Mask (NewMask AS STRING)
    QDirListView.Mask = NewMask
    QDirListView.Refresh
  END PROPERTY

  CONSTRUCTOR
    Width = 400
    Height = 138
    ReadOnly = 1
    FileName = ""
    ShowRoot = 0
    Mask = "*.*"
    ' (the program's own folder until the program names one; nothing is
    ' listed before it sets Directory, Mask or ShowRoot)
    Directory = Application.Path + "\"
    IF INSTR(Application.Path, "/") THEN Directory = Application.Path + "/"
    ' (picture 0 a folder, 1 a page, small and large; ExtensionsList holds
    ' a place for each, so the next picture a program adds and the
    ' extension it adds with it share a number)
    LargeImageList.Width = 32
    LargeImageList.Height = 32
    SmallImageList.AddBMPFile RAPIDR_DIRLISTVIEW_FOLDER, 0
    SmallImageList.AddBMPFile RAPIDR_DIRLISTVIEW_FILE, 0
    LargeImageList.AddBMPFile RAPIDR_DIRLISTVIEW_FOLDER, 0
    LargeImageList.AddBMPFile RAPIDR_DIRLISTVIEW_FILE, 0
    ExtensionsList.AddItems "<dir>", "<dir>"
    SmallImages = QDirListView.SmallImageList
    LargeImages = QDirListView.LargeImageList
    ' (the popup menu with its two hidden items, members a program may use)
    dlw_Enter.Visible = 0
    dlw_BackSpace.Visible = 0
    dlw_PopupMenu.AddItems(QDirListView.dlw_Enter, QDirListView.dlw_BackSpace)
    PopupMenu = QDirListView.dlw_PopupMenu
  END CONSTRUCTOR
END TYPE
$ENDIF
