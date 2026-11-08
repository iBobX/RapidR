' build.inc: Run > Build, the Build dialog (design/build-dialog: the approved
' design; docs/build-for-every-platform-plan.md §2): where the app runs,
' the app's details, what goes in, then Build — a zip per system (Windows,
' macOS, Linux, the web), made at once, with a progress row per app and
' how to open it the first time on each system. Four steps on a list at
' the left that is also the way between them (one scrolling page when the
' dialog is narrow), one main button per screen, every choice saved in the
' project as it is made (no Cancel: the close X and Esc close it, nothing
' is lost). Project > Project Options is the same dialog on its second
' step: one form for the app's details. Build Again (Ctrl+Shift+B) builds
' the saved choices with no dialog: progress in the status bar, a toast at
' the end.
'
' The desktop builds through `rapidr build --targets …` (RPROJECT's
' BuildTargets: crate::build_plan, the one plan the AI's build_app uses
' too), its lines read with RPROJECT.BuildNews; the web Studio packs the
' apps in the page (crate::build's web build) and the browser saves them.
' The build's log is on the Output panel's Build page as before.
' docs/manual/build-for-every-system.md

DECLARE SUB BuildOutput(Text AS STRING)
DECLARE SUB BuildDone(Code AS INTEGER, Built AS STRING)
DECLARE SUB BdStepDraw(Index AS INTEGER, State AS INTEGER, R AS RRect)
DECLARE SUB BdStepPicked
DECLARE SUB BdStepKey(Key AS WORD, Shift AS INTEGER)
DECLARE SUB BdKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB BdClosed(Action AS INTEGER)
DECLARE SUB BdCardPaint(Sender AS RCanvas)
DECLARE SUB BdFramePaint(Sender AS RCanvas)
DECLARE SUB BdBarPaint(Sender AS RCanvas)
DECLARE SUB BdLinePaint(Sender AS RCanvas)
DECLARE SUB BdChipPaint(Sender AS RCanvas)
DECLARE SUB BdNumPaint(Sender AS RCanvas)
DECLARE SUB BdSysClick(Sender AS RCheckBox)
DECLARE SUB BdSysNameClick(Sender AS RLabel)
DECLARE SUB BdArchClick(Sender AS RCheckBox)
DECLARE SUB BdAllDesktopClick
DECLARE SUB BdEverythingClick
DECLARE SUB BdFieldChanged
DECLARE SUB BdKindClick
DECLARE SUB BdCategoryPicked
DECLARE SUB BdIconChoose
DECLARE SUB BdIconDefault
DECLARE SUB BdAdvClick(Sender AS RLabel)
DECLARE SUB BdFileClick(Sender AS RCheckBox)
DECLARE SUB BdFileNameClick(Sender AS RLabel)
DECLARE SUB BdAddFiles
DECLARE SUB BdOptionClick
DECLARE SUB BdFolderChoose
DECLARE SUB BdFolderChanged
DECLARE SUB BdPackClick
DECLARE SUB BdNextClick
DECLARE SUB BdBackClick
DECLARE SUB BdMoreClick
DECLARE SUB BdMorePicked(Sender AS RMenuItem)
DECLARE SUB BdRowActClick(Sender AS RButton)
DECLARE SUB BdSecondClick
DECLARE SUB BdSaveTick
DECLARE SUB BdTick
DECLARE SUB BdResized
DECLARE SUB BdShownNow
DECLARE SUB BdLayOut
DECLARE SUB BdGo(StepNo AS INTEGER)
DECLARE SUB BdRefresh
DECLARE SUB BdStart
DECLARE SUB BdStop
DECLARE SUB BdShowIcon
DECLARE SUB BdFillFiles
DECLARE SUB BdCheckFields
DECLARE FUNCTION BdTicked AS STRING
DECLARE FUNCTION BdApps AS INTEGER
DECLARE FUNCTION BdSystems AS INTEGER
DECLARE FUNCTION BdIndex(Target AS STRING) AS INTEGER
DECLARE FUNCTION BdLabel(i AS INTEGER) AS STRING
DECLARE FUNCTION BdSize(Bytes AS DOUBLE) AS STRING
DECLARE FUNCTION BdNeedsCare AS INTEGER
DECLARE FUNCTION AskRust AS STRING

' (the systems, in the dialog's order: rapidr_package::Target::ALL)
CONST BD_APPS = 7
' (the data files and the left-out ones step 3 can show)
CONST BD_FILES = 40
CONST BD_OUTS = 20

DIM BdTarget(6) AS STRING
DIM BdOn(6) AS INTEGER
' (each app's row while building: waiting, download, working, built,
' failed, stopped; its zip, its size, why it failed)
DIM BdState(6) AS STRING
DIM BdZip(6) AS STRING
DIM BdBytes(6) AS DOUBLE
DIM BdWhy(6) AS STRING
DIM BdNote(6) AS STRING
' (the step shown: 1-4; 5 the build's own page)
DIM BdStepNo AS INTEGER
' ("" choosing, "building", "done")
DIM BdPhase AS STRING
DIM BdNarrow AS INTEGER
DIM BdFilling AS INTEGER
DIM BdShown AS INTEGER
' (the build: when it started, its folder, the zips it made, a line
' each; native code asked for after the zips; the dialog was closed and
' the build goes on in the status bar)
DIM BdStarted AS DOUBLE
DIM BdOutDir AS STRING
DIM BdMade AS STRING
DIM BdNativeNext AS INTEGER
DIM BdNativeRun AS INTEGER
DIM BdQuiet AS INTEGER
DIM BdSeconds AS DOUBLE
' (step 3's files: in the app (path, size, ticked), and left out (name, why))
DIM BdFileCount AS INTEGER
DIM BdFilePath(BD_FILES) AS STRING
DIM BdFileBytes(BD_FILES) AS DOUBLE
DIM BdOutCount AS INTEGER
DIM BdOutName(BD_OUTS) AS STRING
DIM BdOutWhy(BD_OUTS) AS STRING
' (Advanced open on steps 2 and 3)
DIM BdAdv2 AS INTEGER
DIM BdAdv3 AS INTEGER
' (the icon's picture's size, when it is a picture: a warning under 512)
DIM BdIconPx AS INTEGER
DIM BdIconSquare AS INTEGER
' (what the old Build did, still the Output's: native, interpreted, web, setup)
DIM BuildMode AS STRING
DIM BuildStarted AS DOUBLE

DIM BdIconBmp AS RBitmap
DIM BdIconSmall(3) AS RBitmap
DIM BdBmp(30) AS RBitmap
DIM BdMeasure AS RBitmap
DIM BdSaveTimer AS RTimer
DIM BdTickTimer AS RTimer
DIM BdMoreMenu AS RPopupMenu
DIM BdMoreItem(2) AS RMenuItem
DIM BdToastId AS INTEGER
' (the web Studio's first choice: the system the browser says it runs on)
DIM BdSysInfo AS RSystemInfo

' ---- the dialog --------------------------------------------------------------------------------

CREATE BuildDialog AS RForm
  Caption = TR$("Build", "the Build dialog")
  Width = 860: Height = 600
  BorderStyle = 3
  KeyPreview = 1
  OnKeyDown = BdKey
  OnClose = BdClosed
  OnResize = BdResized
  OnShow = BdShownNow
  ' ---- the steps, at the left: a list, also the way between them ----
  CREATE BdRail AS RPanel
    BevelOuter = 0
    CREATE BdSteps AS RListBox
      Style = lbOwnerDrawFixed
      BorderStyle = 0
      AccessibleName = TR$("Steps")
      AccessibleDescription = TR$("The arrows move, Enter opens a step. Ctrl+Tab goes to the next step from anywhere in the dialog.")
      OnDrawItem = BdStepDraw
      OnClick = BdStepPicked
      OnKeyDown = BdStepKey
    END CREATE
    CREATE BdSavedIcon AS RImage
      Width = 16: Height = 16
      AccessibleHidden = 1
    END CREATE
    CREATE BdSavedNote AS RLabel
      WordWrap = 1: AutoSize = 0
      Caption = TR$("Saved in the project as you go")
    END CREATE
  END CREATE
  CREATE BdRailLine AS RCanvas
    OnPaint = BdLinePaint
    AccessibleHidden = 1
  END CREATE
  ' ---- the footer: More, a line, Back and the step's one main button ----
  CREATE BdFoot AS RPanel
    BevelOuter = 0
    CREATE BdFootLine AS RCanvas
      OnPaint = BdLinePaint
      AccessibleHidden = 1
    END CREATE
    CREATE BdMore AS RButton
      Caption = "..."
      AccessibleName = TR$("More")
      AccessibleDescription = TR$("More build actions: Copy Command Line, Save All in One Zip, Help")
      OnClick = BdMoreClick
    END CREATE
    CREATE BdFootNote AS RLabel
      AutoSize = 0
    END CREATE
    CREATE BdSecond AS RButton
      OnClick = BdSecondClick
    END CREATE
    CREATE BdNext AS RButton
      Caption = TR$("&Next")
      Default = 1
      OnClick = BdNextClick
    END CREATE
  END CREATE
  ' ---- the pages: they scroll when the dialog is short, and become one
  ' page under 640 px ----
  CREATE BdBody AS RScrollBox
    BorderStyle = 0
    HorzVisible = 0
    ' ---- 1. Where it runs ----
    CREATE BdP1 AS RPanel
      BevelOuter = 0
      CREATE BdH1 AS RLabel
        Caption = TR$("Where should it run?")
      END CREATE
      CREATE BdS1 AS RLabel
        WordWrap = 1: AutoSize = 0
        Caption = TR$("Tick every system you want. Build makes them all at once, on this computer.")
      END CREATE
      CREATE BdAllDesktop AS RLabel
        Caption = TR$("All &Desktop")
        TabStop = 1
        AccessibleDescription = TR$("Ticks Windows, macOS and Linux.")
        OnClick = BdAllDesktopClick
      END CREATE
      CREATE BdEverything AS RLabel
        Caption = TR$("&Everything")
        TabStop = 1
        AccessibleDescription = TR$("Ticks every system.")
        OnClick = BdEverythingClick
      END CREATE
      ' (the four cards: a canvas under each, its parts on top)
      CREATE BdCard0 AS RCanvas
        Tag = 0
        OnPaint = BdCardPaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdCard1 AS RCanvas
        Tag = 1
        OnPaint = BdCardPaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdCard2 AS RCanvas
        Tag = 2
        OnPaint = BdCardPaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdCard3 AS RCanvas
        Tag = 3
        OnPaint = BdCardPaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdHelpIcon AS RImage
        Width = 16: Height = 16
        AccessibleHidden = 1
      END CREATE
      CREATE BdHelpText AS RLabel
        WordWrap = 1: AutoSize = 0
        Caption = TR$("Not sure? x64 is most PCs since 2005. ARM64 is Snapdragon PCs and the Raspberry Pi. The macOS app is made for Apple silicon and Intel Macs.")
      END CREATE
      CREATE BdLaterLabel AS RLabel
        Caption = TR$("Coming later:")
      END CREATE
      CREATE BdLater0 AS RCanvas
        Tag = 0
        OnPaint = BdChipPaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdLater1 AS RCanvas
        Tag = 1
        OnPaint = BdChipPaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdLater2 AS RCanvas
        Tag = 2
        OnPaint = BdChipPaint
        AccessibleHidden = 1
      END CREATE
    END CREATE
    ' ---- 2. Your app (Project Options too: one form) ----
    CREATE BdP2 AS RPanel
      BevelOuter = 0
      CREATE BdH2 AS RLabel
        Caption = TR$("Your app")
      END CREATE
      CREATE BdS2 AS RLabel
        WordWrap = 1: AutoSize = 0
        Caption = TR$("Filled in once and saved in the project. Every system shows these.")
      END CREATE
      CREATE BdNameLabel AS RLabel
        Caption = TR$("Na&me:")
      END CREATE
      CREATE BdName AS REdit
        AccessibleName = TR$("Name")
        AccessibleDescription = TR$("The app's name: its title, the Dock, the Start menu")
        OnChange = BdFieldChanged
      END CREATE
      CREATE BdVersionLabel AS RLabel
        Caption = TR$("&Version:")
      END CREATE
      CREATE BdVersion AS REdit
        AccessibleName = TR$("Version")
        OnChange = BdFieldChanged
      END CREATE
      CREATE BdAuthorLabel AS RLabel
        Caption = TR$("&Author or company:")
      END CREATE
      CREATE BdAuthor AS REdit
        AccessibleName = TR$("Author or company")
        OnChange = BdFieldChanged
      END CREATE
      CREATE BdCopyrightLabel AS RLabel
        Caption = TR$("Co&pyright:")
      END CREATE
      CREATE BdCopyright AS REdit
        AccessibleName = TR$("Copyright")
        OnChange = BdFieldChanged
      END CREATE
      CREATE BdDescLabel AS RLabel
        Caption = TR$("&Description:")
      END CREATE
      CREATE BdDesc AS REdit
        AccessibleName = TR$("Description")
        AccessibleDescription = TR$("One line about the app: Windows' file description, macOS' Get Info, Linux's menu, the web page")
        OnChange = BdFieldChanged
      END CREATE
      CREATE BdSiteLabel AS RLabel
        Caption = TR$("&Website:")
      END CREATE
      CREATE BdSite AS REdit
        AccessibleName = TR$("Website")
        OnChange = BdFieldChanged
      END CREATE
      CREATE BdIdLabel AS RLabel
        Caption = TR$("App &id:")
      END CREATE
      CREATE BdId AS REdit
        AccessibleName = TR$("App id")
        OnChange = BdFieldChanged
      END CREATE
      CREATE BdIdNote AS RLabel
        WordWrap = 1: AutoSize = 0
      END CREATE
      CREATE BdCatLabel AS RLabel
        Caption = TR$("&Category:")
      END CREATE
      CREATE BdCat AS RComboBox
        Style = 2
        AccessibleName = TR$("Category")
        OnChange = BdCategoryPicked
      END CREATE
      CREATE BdKindLabel AS RLabel
        Caption = TR$("&Kind:")
      END CREATE
      CREATE BdKindWin AS RRadioButton
        Caption = TR$("Windowed app")
        OnClick = BdKindClick
      END CREATE
      CREATE BdKindCon AS RRadioButton
        Caption = TR$("Console app")
        OnClick = BdKindClick
      END CREATE
      CREATE BdKindNote AS RLabel
        WordWrap = 1: AutoSize = 0
        Caption = TR$("Found in the program; change it only if it's wrong.")
      END CREATE
      CREATE BdAdv2Icon AS RImage
        Width = 16: Height = 16
        AccessibleHidden = 1
      END CREATE
      CREATE BdAdv2Head AS RLabel
        TabStop = 1
        Tag = 2
        OnClick = BdAdvClick
      END CREATE
      CREATE BdAdv2Sum AS RLabel
        Tag = 2
        AccessibleHidden = 1
        OnClick = BdAdvClick
      END CREATE
      CREATE BdColorLabel AS RLabel
        Caption = TR$("&Theme colour:")
      END CREATE
      CREATE BdColor AS REdit
        AccessibleName = TR$("Theme colour")
        AccessibleDescription = TR$("The web app's colour, #RRGGBB; empty: the page's")
        OnChange = BdFieldChanged
      END CREATE
      CREATE BdLangLabel AS RLabel
        Caption = TR$("&README language:")
      END CREATE
      CREATE BdLang AS RComboBox
        Style = 2
        AccessibleName = TR$("README language")
        AccessibleDescription = TR$("The language of the README.txt in every zip: how to open the app the first time")
        OnChange = BdCategoryPicked
      END CREATE
      ' (the icon, at the right: one picture made into every size)
      CREATE BdIconFrame AS RCanvas
        OnPaint = BdFramePaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdIconHead AS RLabel
        Caption = TR$("Icon")
      END CREATE
      CREATE BdIconBig AS RImage
        Stretch = 1
        AccessibleName = TR$("The app's icon")
      END CREATE
      CREATE BdIcon16 AS RImage
        Stretch = 1
        AccessibleHidden = 1
      END CREATE
      CREATE BdIcon24 AS RImage
        Stretch = 1
        AccessibleHidden = 1
      END CREATE
      CREATE BdIcon32 AS RImage
        Stretch = 1
        AccessibleHidden = 1
      END CREATE
      CREATE BdIcon48 AS RImage
        Stretch = 1
        AccessibleHidden = 1
      END CREATE
      CREATE BdSz16 AS RLabel
        Caption = "16": Alignment = 2: AutoSize = 0
        AccessibleHidden = 1
      END CREATE
      CREATE BdSz24 AS RLabel
        Caption = "24": Alignment = 2: AutoSize = 0
        AccessibleHidden = 1
      END CREATE
      CREATE BdSz32 AS RLabel
        Caption = "32": Alignment = 2: AutoSize = 0
        AccessibleHidden = 1
      END CREATE
      CREATE BdSz48 AS RLabel
        Caption = "48": Alignment = 2: AutoSize = 0
        AccessibleHidden = 1
      END CREATE
      CREATE BdIconFile AS RLabel
        WordWrap = 1: AutoSize = 0
      END CREATE
      CREATE BdIconWarn AS RLabel
        WordWrap = 1: AutoSize = 0
      END CREATE
      CREATE BdIconBtn AS RButton
        Caption = TR$("C&hoose Picture...")
        OnClick = BdIconChoose
      END CREATE
      CREATE BdIconReset AS RLabel
        Caption = TR$("&Use RapidR's Icon")
        TabStop = 1
        OnClick = BdIconDefault
      END CREATE
      CREATE BdIconNote AS RLabel
        WordWrap = 1: AutoSize = 0
        Caption = TR$("Made in every size each system needs, 16 to 1024.")
      END CREATE
    END CREATE
    ' ---- 3. What goes in ----
    CREATE BdP3 AS RPanel
      BevelOuter = 0
      CREATE BdH3 AS RLabel
        Caption = TR$("What goes in")
      END CREATE
      CREATE BdS3 AS RLabel
        WordWrap = 1: AutoSize = 0
        Caption = TR$("Your program and the files it reads. Untick a file to leave it out.")
      END CREATE
      CREATE BdProgIcon AS RImage
        Width = 16: Height = 16
        AccessibleHidden = 1
      END CREATE
      CREATE BdProgName AS RLabel
      END CREATE
      CREATE BdProgNote AS RLabel
        AutoSize = 0
        Caption = TR$("Compiled once for every system.")
      END CREATE
      CREATE BdDataHead AS RLabel
      END CREATE
      CREATE BdAddBtn AS RButton
        Caption = TR$("&Add Files...")
        AccessibleDescription = TR$("Adds files the program reads to the project: they go in every app")
        OnClick = BdAddFiles
      END CREATE
      CREATE BdFilesFrame AS RCanvas
        OnPaint = BdFramePaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdNoFiles AS RLabel
        WordWrap = 1: AutoSize = 0
        Caption = TR$("No data files: the program names none beside it.")
      END CREATE
      CREATE BdOutHead AS RLabel
      END CREATE
      CREATE BdOutFrame AS RCanvas
        OnPaint = BdFramePaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdSizeIcon AS RImage
        Width = 16: Height = 16
        AccessibleHidden = 1
      END CREATE
      CREATE BdSizeNote AS RLabel
        WordWrap = 1: AutoSize = 0
      END CREATE
      CREATE BdAdv3Icon AS RImage
        Width = 16: Height = 16
        AccessibleHidden = 1
      END CREATE
      CREATE BdAdv3Head AS RLabel
        TabStop = 1
        Tag = 3
        OnClick = BdAdvClick
      END CREATE
      CREATE BdAdv3Sum AS RLabel
        Tag = 3
        AccessibleHidden = 1
        OnClick = BdAdvClick
      END CREATE
      CREATE BdSource AS RCheckBox
        Caption = TR$("Include the &source code")
        AccessibleDescription = TR$("Your program's source goes in every zip too, in a folder named source")
        OnClick = BdOptionClick
      END CREATE
      CREATE BdNative AS RCheckBox
        OnClick = BdOptionClick
      END CREATE
      CREATE BdNativeNote AS RLabel
        WordWrap = 1: AutoSize = 0
      END CREATE
    END CREATE
    ' ---- 4. Ready to build ----
    CREATE BdP4 AS RPanel
      BevelOuter = 0
      CREATE BdH4 AS RLabel
        Caption = TR$("Ready to build")
      END CREATE
      CREATE BdS4 AS RLabel
        WordWrap = 1: AutoSize = 0
      END CREATE
      CREATE BdReadyFrame AS RCanvas
        OnPaint = BdFramePaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdFolderLabel AS RLabel
        Caption = TR$("&Save in:")
      END CREATE
      CREATE BdFolder AS REdit
        AccessibleName = TR$("Save in")
        AccessibleDescription = TR$("The folder the zips go in")
        OnChange = BdFolderChanged
      END CREATE
      CREATE BdFolderBtn AS RButton
        Caption = TR$("Ch&oose...")
        AccessibleName = TR$("Choose the folder")
        OnClick = BdFolderChoose
      END CREATE
      CREATE BdPackLabel AS RLabel
        Caption = TR$("&Pack as:")
      END CREATE
      CREATE BdPackEach AS RRadioButton
        Caption = TR$("A zip for each system")
        OnClick = BdPackClick
      END CREATE
      CREATE BdPackOne AS RRadioButton
        Caption = TR$("Everything in one zip")
        OnClick = BdPackClick
      END CREATE
      CREATE BdDlFrame AS RCanvas
        Tag = 9
        OnPaint = BdCardPaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdDlIcon AS RImage
        Width = 16: Height = 16
        AccessibleHidden = 1
      END CREATE
      CREATE BdDlHead AS RLabel
        Transparent = 1
      END CREATE
      CREATE BdDlText AS RLabel
        WordWrap = 1: AutoSize = 0
        Transparent = 1
      END CREATE
      CREATE BdReadmeIcon AS RImage
        Width = 16: Height = 16
        AccessibleHidden = 1
      END CREATE
      CREATE BdReadmeNote AS RLabel
        WordWrap = 1: AutoSize = 0
        Caption = TR$("Each zip has a README.txt on opening the app the first time, and SHA256SUMS lists every zip's checksum.")
      END CREATE
    END CREATE
    ' ---- building, then done ----
    CREATE BdP5 AS RPanel
      BevelOuter = 0
      CREATE BdH5Icon AS RImage
        Width = 20: Height = 20
        AccessibleHidden = 1
      END CREATE
      CREATE BdH5 AS RLabel
      END CREATE
      CREATE BdS5 AS RLabel
        WordWrap = 1: AutoSize = 0
      END CREATE
      CREATE BdAllBar AS RCanvas
        Tag = -1
        OnPaint = BdBarPaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdAllNote AS RLabel
        AutoSize = 0
      END CREATE
      CREATE BdRunFrame AS RCanvas
        OnPaint = BdFramePaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdFirstFrame AS RCanvas
        OnPaint = BdFramePaint
        AccessibleHidden = 1
      END CREATE
      CREATE BdFirstHead AS RLabel
        Caption = TR$("Opening it the first time on another computer")
      END CREATE
      CREATE BdFirstFoot AS RLabel
        WordWrap = 1: AutoSize = 0
        Caption = TR$("Every zip's README.txt says the same, for its system.")
      END CREATE
    END CREATE
  END CREATE
END CREATE

' (made once, when Studio starts: the cards' parts, the rows — arrays of
' components parented here — and their events)
DIM BdSys(3) AS RCheckBox
DIM BdSysIcon(3) AS RImage
DIM BdSysName(3) AS RLabel
DIM BdSysText(3) AS RLabel
DIM BdArch(4) AS RCheckBox
DIM BdArchNote(4) AS RLabel
DIM BdCardNote(3) AS RLabel
DIM BdFileChk(BD_FILES) AS RCheckBox
DIM BdFileIcon(BD_FILES) AS RImage
DIM BdFileKind(BD_FILES) AS RLabel
DIM BdFileName(BD_FILES) AS RLabel
DIM BdFileSize(BD_FILES) AS RLabel
DIM BdFileBmp(BD_FILES) AS RBitmap
DIM BdOutIcon(BD_OUTS) AS RImage
DIM BdOutLabel(BD_OUTS) AS RLabel
DIM BdOutText(BD_OUTS) AS RLabel
DIM BdOutBmp(BD_OUTS) AS RBitmap
DIM BdReadyIcon(6) AS RImage
DIM BdReadyName(6) AS RLabel
DIM BdReadyZip(6) AS RLabel
DIM BdReadySize(6) AS RLabel
DIM BdRunIcon(6) AS RImage
DIM BdRunName(6) AS RLabel
DIM BdRunStage(6) AS RLabel
DIM BdRunSize(6) AS RLabel
DIM BdRunMark(6) AS RImage
DIM BdRunBar(6) AS RCanvas
DIM BdRunPct(6) AS RLabel
DIM BdRunAct(6) AS RButton
DIM BdRunLine(6) AS RCanvas
DIM BdFirstIcon(3) AS RImage
DIM BdFirstName(3) AS RLabel
DIM BdFirstText(3) AS RLabel
DIM BdAppBmp(6) AS RBitmap
DIM BdMarkBmp(6) AS RBitmap
DIM BdSysBmp(3) AS RBitmap
DIM BdFirstBmp(3) AS RBitmap
DIM BdRowsMade AS INTEGER
DIM BdLoaded AS INTEGER
DIM BdLaterText(2) AS RLabel
' (one page, narrow: each step's number in a circle before its heading)
DIM BdNum(3) AS RCanvas
DIM BdNumText(3) AS RLabel

' (the card a system is on: 0 Windows, 1 macOS, 2 Linux, 3 the web)
FUNCTION BdCardOf(i AS INTEGER) AS INTEGER
  SELECT CASE i
    CASE 0, 1, 2: Result = 0
    CASE 3: Result = 1
    CASE 4, 5: Result = 2
    CASE ELSE: Result = 3
  END SELECT
END FUNCTION

FUNCTION BdIconName(Card AS INTEGER) AS STRING
  SELECT CASE Card
    CASE 0: Result = "target-windows"
    CASE 1: Result = "target-macos"
    CASE 2: Result = "target-linux"
    CASE ELSE: Result = "target-web"
  END SELECT
END FUNCTION

' Each app's name as the dialog says it: "Windows 64-bit (x64)".
FUNCTION BdLabel(i AS INTEGER) AS STRING
  SELECT CASE i
    CASE 0: Result = TR$("Windows 64-bit (x64)")
    CASE 1: Result = TR$("Windows ARM64")
    CASE 2: Result = TR$("Windows 32-bit (x86)")
    CASE 3: Result = TR$("macOS Universal")
    CASE 4: Result = TR$("Linux 64-bit (x64)")
    CASE 5: Result = TR$("Linux ARM64")
    CASE ELSE: Result = TR$("Web")
  END SELECT
END FUNCTION

FUNCTION BdIndex(Target AS STRING) AS INTEGER
  DIM i AS INTEGER
  Result = 0
  Result = -1
  FOR i = 0 TO BD_APPS - 1
    IF BdTarget(i) = Target THEN Result = i
  NEXT
END FUNCTION

' (a number with one decimal, in the language's way: 2.8 / 2,8)
FUNCTION BdOne(X AS DOUBLE) AS STRING
  Result = REPLACESUBSTR$(FORMAT$("%.1f", X), ".", TR$(".", "the decimal separator"))
END FUNCTION

' (a size as people read it: 12 KB, 21 MB, 1.4 MB)
FUNCTION BdSize(Bytes AS DOUBLE) AS STRING
  IF Bytes < 1024 THEN
    Result = TR$("{n} bytes", "n", LTRIM$(STR$(INT(Bytes))))
  ELSEIF Bytes < 1048576 THEN
    Result = TR$("{n} KB", "n", LTRIM$(STR$(INT((Bytes + 1023) / 1024))))
  ELSEIF Bytes < 10485760 THEN
    Result = TR$("{n} MB", "n", BdOne(Bytes / 1048576))
  ELSE
    Result = TR$("{n} MB", "n", LTRIM$(STR$(INT(Bytes / 1048576 + 0.5))))
  END IF
END FUNCTION

' The ticked systems as `rapidr build --targets` names them.
FUNCTION BdTicked AS STRING
  DIM i AS INTEGER
  Result = ""
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) THEN
      IF Result <> "" THEN Result = Result + ","
      Result = Result + BdTarget(i)
    END IF
  NEXT
END FUNCTION

FUNCTION BdApps AS INTEGER
  DIM i AS INTEGER
  Result = 0
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) THEN Result = Result + 1
  NEXT
END FUNCTION

' (how many systems: Windows counts once however many processors)
FUNCTION BdSystems AS INTEGER
  DIM c AS INTEGER, seen(3) AS INTEGER, i AS INTEGER
  Result = 0
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) THEN seen(BdCardOf(i)) = 1
  NEXT
  FOR c = 0 TO 3
    Result = Result + seen(c)
  NEXT
END FUNCTION

' (the systems ticked, in words, for the steps' list: "Windows, macOS, Web")
FUNCTION BdSystemWords AS STRING
  DIM c AS INTEGER, i AS INTEGER, on AS INTEGER
  Result = ""
  FOR c = 0 TO 3
    on = 0
    FOR i = 0 TO BD_APPS - 1
      IF BdOn(i) AND BdCardOf(i) = c THEN on = 1
    NEXT
    IF on THEN
      IF Result <> "" THEN Result = Result + ", "
      SELECT CASE c
        CASE 0: Result = Result + "Windows"
        CASE 1: Result = Result + "macOS"
        CASE 2: Result = Result + "Linux"
        CASE 3: Result = Result + TR$("Web")
      END SELECT
    END IF
  NEXT
  IF Result = "" THEN Result = TR$("Nothing ticked yet")
END FUNCTION

' (the systems and apps, for the footer: "3 systems, 4 apps")
FUNCTION BdCountWords AS STRING
  Result = TRN$("{n} system", "{n} systems", BdSystems) + ", " + TRN$("{n} app", "{n} apps", BdApps)
END FUNCTION

' ---- making the parts ---------------------------------------------------------------------

SUB BdMake
  DIM i AS INTEGER, c AS INTEGER
  IF BdRowsMade THEN EXIT SUB
  BdRowsMade = 1
  BdTarget(0) = "windows-x64": BdTarget(1) = "windows-arm64": BdTarget(2) = "windows-x86"
  BdTarget(3) = "macos-universal": BdTarget(4) = "linux-x64": BdTarget(5) = "linux-arm64": BdTarget(6) = "web"
  FOR c = 0 TO 3
    BdSys(c).Parent = BdP1
    BdSys(c).Tag = c
    BdSys(c).Caption = ""
    BdSys(c).OnClick = BdSysClick
    BdSysIcon(c).Parent = BdP1
    BdSysIcon(c).Width = 20: BdSysIcon(c).Height = 20
    BdSysIcon(c).AccessibleHidden = 1
    BdSysName(c).Parent = BdP1
    BdSysName(c).Tag = c
    BdSysName(c).Transparent = 1
    BdSysName(c).AccessibleHidden = 1
    BdSysName(c).OnClick = BdSysNameClick
    BdSysText(c).Parent = BdP1
    BdSysText(c).WordWrap = 1: BdSysText(c).AutoSize = 0
    BdSysText(c).Transparent = 1
    BdCardNote(c).Parent = BdP1
    BdCardNote(c).WordWrap = 1: BdCardNote(c).AutoSize = 0
    BdCardNote(c).Transparent = 1
  NEXT
  FOR i = 0 TO 4
    BdArch(i).Parent = BdP1
    BdArch(i).Tag = i
    BdArch(i).OnClick = BdArchClick
    BdArchNote(i).Parent = BdP1
    BdArchNote(i).Transparent = 1
    BdArchNote(i).AccessibleHidden = 1
  NEXT
  FOR i = 0 TO BD_FILES - 1
    BdFileChk(i).Parent = BdP3
    BdFileChk(i).Tag = i
    BdFileChk(i).OnClick = BdFileClick
    BdFileChk(i).Visible = 0
    BdFileIcon(i).Parent = BdP3
    BdFileIcon(i).Width = 16: BdFileIcon(i).Height = 16
    BdFileIcon(i).AccessibleHidden = 1
    BdFileIcon(i).Visible = 0
    BdFileName(i).Parent = BdP3
    BdFileName(i).Transparent = 1
    BdFileName(i).Tag = i
    BdFileName(i).AccessibleHidden = 1
    BdFileName(i).OnClick = BdFileNameClick
    BdFileName(i).Visible = 0
    BdFileKind(i).Parent = BdP3
    BdFileKind(i).Transparent = 1
    BdFileKind(i).AccessibleHidden = 1
    BdFileKind(i).Visible = 0
    BdFileSize(i).Parent = BdP3
    BdFileSize(i).Transparent = 1
    BdFileSize(i).Alignment = 1: BdFileSize(i).AutoSize = 0
    BdFileSize(i).AccessibleHidden = 1
    BdFileSize(i).Visible = 0
  NEXT
  FOR i = 0 TO BD_OUTS - 1
    BdOutIcon(i).Parent = BdP3
    BdOutIcon(i).Width = 16: BdOutIcon(i).Height = 16
    BdOutIcon(i).AccessibleHidden = 1
    BdOutIcon(i).Visible = 0
    BdOutLabel(i).Parent = BdP3
    BdOutLabel(i).Transparent = 1
    BdOutLabel(i).AccessibleHidden = 1
    BdOutLabel(i).Visible = 0
    BdOutText(i).Parent = BdP3
    BdOutText(i).Transparent = 1
    BdOutText(i).AutoSize = 0
    BdOutText(i).Visible = 0
  NEXT
  FOR i = 0 TO BD_APPS - 1
    BdReadyIcon(i).Parent = BdP4
    BdReadyIcon(i).Width = 16: BdReadyIcon(i).Height = 16
    BdReadyIcon(i).AccessibleHidden = 1
    BdReadyName(i).Parent = BdP4
    BdReadyName(i).Transparent = 1
    BdReadyZip(i).Parent = BdP4
    BdReadyZip(i).Transparent = 1
    BdReadyZip(i).AutoSize = 0
    BdReadySize(i).Parent = BdP4
    BdReadySize(i).Transparent = 1
    BdReadySize(i).Alignment = 1: BdReadySize(i).AutoSize = 0
    BdRunLine(i).Parent = BdP5
    BdRunLine(i).OnPaint = BdLinePaint
    BdRunLine(i).AccessibleHidden = 1
    BdRunIcon(i).Parent = BdP5
    BdRunIcon(i).Width = 16: BdRunIcon(i).Height = 16
    BdRunIcon(i).AccessibleHidden = 1
    BdRunName(i).Parent = BdP5
    BdRunName(i).Transparent = 1
    BdRunStage(i).Parent = BdP5
    BdRunStage(i).Transparent = 1
    BdRunStage(i).AutoSize = 0: BdRunStage(i).WordWrap = 1
    BdRunSize(i).Parent = BdP5
    BdRunSize(i).Transparent = 1
    BdRunSize(i).Alignment = 1: BdRunSize(i).AutoSize = 0
    BdRunMark(i).Parent = BdP5
    BdRunMark(i).Width = 16: BdRunMark(i).Height = 16
    BdRunMark(i).AccessibleHidden = 1
    BdRunBar(i).Parent = BdP5
    BdRunBar(i).Tag = i
    BdRunBar(i).OnPaint = BdBarPaint
    BdRunPct(i).Parent = BdP5
    BdRunPct(i).Transparent = 1
    BdRunPct(i).Alignment = 1: BdRunPct(i).AutoSize = 0
    BdRunPct(i).AccessibleHidden = 1
    BdRunAct(i).Parent = BdP5
    BdRunAct(i).Tag = i
    BdRunAct(i).OnClick = BdRowActClick
  NEXT
  FOR c = 0 TO 3
    BdFirstIcon(c).Parent = BdP5
    BdFirstIcon(c).Width = 16: BdFirstIcon(c).Height = 16
    BdFirstIcon(c).AccessibleHidden = 1
    BdFirstName(c).Parent = BdP5
    BdFirstName(c).Transparent = 1
    BdFirstText(c).Parent = BdP5
    BdFirstText(c).Transparent = 1
    BdFirstText(c).WordWrap = 1: BdFirstText(c).AutoSize = 0
  NEXT
  FOR i = 0 TO 2
    BdLaterText(i).Parent = BdP1
    BdLaterText(i).AccessibleHidden = 1
  NEXT
  BdNum(0).Parent = BdP1: BdNum(1).Parent = BdP2: BdNum(2).Parent = BdP3: BdNum(3).Parent = BdP4
  BdNumText(0).Parent = BdP1: BdNumText(1).Parent = BdP2: BdNumText(2).Parent = BdP3: BdNumText(3).Parent = BdP4
  FOR i = 0 TO 3
    BdNum(i).Tag = i
    BdNum(i).OnPaint = BdNumPaint
    BdNum(i).AccessibleHidden = 1
    BdNum(i).Visible = 0
    BdNumText(i).Caption = LTRIM$(STR$(i + 1))
    BdNumText(i).Transparent = 1
    BdNumText(i).Alignment = 2: BdNumText(i).AutoSize = 0
    BdNumText(i).AccessibleHidden = 1
    BdNumText(i).Visible = 0
  NEXT
  BdTickTimer.Interval = 500
  BdTickTimer.Enabled = 0
  BdTickTimer.OnTimer = BdTick
  BdRail.TabOrder = 0
  BdBody.TabOrder = 1
  BdFoot.TabOrder = 2
  BdSys(0).TabOrder = 10: BdArch(0).TabOrder = 11: BdArch(1).TabOrder = 12: BdArch(2).TabOrder = 13
  BdSys(1).TabOrder = 14
  BdSys(2).TabOrder = 15: BdArch(3).TabOrder = 16: BdArch(4).TabOrder = 17
  BdSys(3).TabOrder = 18
  BdSaveTimer.Interval = 400
  BdSaveTimer.Enabled = 0
  BdSaveTimer.OnTimer = BdSaveTick
  FOR i = 0 TO 2
    BdMoreItem(i).Tag = i
    BdMoreItem(i).OnClick = BdMorePicked
    BdMoreMenu.AddItems BdMoreItem(i)
  NEXT
END SUB

' (the words that are made again in the language of the moment: the
' lists' items, the cards' words)
SUB BdWords
  BdSysName(0).Caption = "Windows": BdSysName(1).Caption = "macOS"
  BdSysName(2).Caption = "Linux": BdSysName(3).Caption = TR$("Web")
  BdSys(0).AccessibleName = "Windows": BdSys(1).AccessibleName = "macOS"
  BdSys(2).AccessibleName = "Linux": BdSys(3).AccessibleName = TR$("Web")
  BdSysText(1).Caption = TR$("One Universal app for Macs with Apple silicon and Intel.")
  BdSysText(3).Caption = TR$("A zip for any web host. Runs in any modern browser, phones too.")
  BdSys(1).AccessibleDescription = TR$("One Universal app for Apple silicon and Intel Macs")
  BdSys(3).AccessibleDescription = TR$("A zip for any web host")
  BdArch(0).Caption = TR$("64-bit (x64)"): BdArchNote(0).Caption = TR$("most PCs")
  BdArch(1).Caption = "ARM64": BdArchNote(1).Caption = TR$("Snapdragon PCs")
  BdArch(2).Caption = TR$("32-bit (x86)"): BdArchNote(2).Caption = TR$("Windows 7, 8, old PCs")
  BdArch(3).Caption = TR$("64-bit (x64)"): BdArchNote(3).Caption = TR$("most PCs")
  BdArch(4).Caption = "ARM64": BdArchNote(4).Caption = TR$("Raspberry Pi 4, 5")
  BdArch(0).AccessibleDescription = TR$("most PCs")
  BdArch(1).AccessibleDescription = TR$("Snapdragon PCs")
  BdArch(2).AccessibleDescription = TR$("Runs on old PCs; a little slower.")
  BdArch(3).AccessibleDescription = TR$("most PCs")
  BdArch(4).AccessibleDescription = TR$("Raspberry Pi 4, 5")
  BdCardNote(0).Caption = TR$("Runs on old PCs; a little slower.")
  BdCardNote(2).Caption = TR$("A .tar.gz with an install script.")
  BdLaterLabel.AccessibleName = TR$("Coming later: Android, iOS, boards (ESP32, micro:bit)")
  BdSteps.Clear
  BdCat.Clear
  BdCat.AddItems TR$("Utilities"), TR$("Productivity"), TR$("Education"), TR$("Games"), TR$("Developer tools"), TR$("Graphics"), TR$("Music"), TR$("Video"), TR$("Business"), TR$("Finance"), TR$("Science")
  BdLang.Clear
  BdLang.AddItems "English", "Español"
  BdMoreItem(0).Caption = TR$("&Copy Command Line")
  BdMoreItem(1).Caption = TR$("Save All in &One Zip")
  BdMoreItem(2).Caption = TR$("&Help")
  BdNative.Caption = TR$("&Native code")
  IF Proj.NativeBuilds = 0 THEN BdNative.Caption = TR$("Native code (not on 32-bit Windows)")
END SUB

' (the category's words as the project keeps them)
FUNCTION BdCatWord(i AS INTEGER) AS STRING
  SELECT CASE i
    CASE 1: Result = "productivity"
    CASE 2: Result = "education"
    CASE 3: Result = "games"
    CASE 4: Result = "developer-tools"
    CASE 5: Result = "graphics"
    CASE 6: Result = "music"
    CASE 7: Result = "video"
    CASE 8: Result = "business"
    CASE 9: Result = "finance"
    CASE 10: Result = "science"
    CASE ELSE: Result = "utilities"
  END SELECT
END FUNCTION

' ---- opening it ------------------------------------------------------------------------

' (the dialog's parts made and the project's choices in them: before a
' test step or Build Again changes one)
SUB BdEnsure
  IF BdLoaded THEN EXIT SUB
  BdMake
  BdWords
  BdLoad
END SUB

' Run > Build (StepNo 1, the step to go on from), Project Options (2).
SUB ShowBuildDialog(StepNo AS INTEGER)
  IF ProjectOpen = 0 THEN EXIT SUB
  BdMake
  BdWords
  BdLoad
  ' (5: the build's own page, as it is — building or done)
  IF BdPhase <> "building" AND StepNo < 5 THEN BdPhase = ""
  BdQuiet = 0
  IF StepNo = 2 THEN BuildDialog.Caption = TR$("Project Options") ELSE BuildDialog.Caption = TR$("Build", "the Build dialog")
  BdSize2Screen
  BdGo StepNo
  BuildDialog.Center
  BdShown = 1
  BuildDialog.ShowModal
  BdShown = 0
END SUB

' (the dialog's size: 860 x 600, at most 85 % of the screen; narrow, one
' page, under 640)
SUB BdSize2Screen
  DIM w AS INTEGER, h AS INTEGER, fw AS INTEGER, fh AS INTEGER
  w = 860: h = 600
  IF ThemeName = "classic" THEN w = 820: h = 580
  ' (the window with its frame and title bar: at most 85 % of the screen)
  fw = BuildDialog.Width - BuildDialog.ClientWidth
  fh = BuildDialog.Height - BuildDialog.ClientHeight
  IF w + fw > Screen.ClientWidth * 85 \ 100 THEN w = Screen.ClientWidth * 85 \ 100 - fw
  IF h + fh > Screen.ClientHeight * 85 \ 100 THEN h = Screen.ClientHeight * 85 \ 100 - fh
  IF w < 320 THEN w = 320
  IF h < 360 THEN h = 360
  BuildDialog.ClientWidth = w
  BuildDialog.ClientHeight = h
END SUB

' The project's choices into the dialog (the first time: this computer's
' system ticked, the app's details made from the project).
SUB BdLoad
  DIM i AS INTEGER, list AS STRING, nm AS STRING
  BdLoaded = 1
  BdFilling = 1
  list = Proj.BuildFor
  IF list = "" THEN list = Proj.FirstTargets(BdSysInfo.OS)
  FOR i = 0 TO BD_APPS - 1
    BdOn(i) = Flag(INSTR("," + list + ",", "," + BdTarget(i) + ",") > 0)
  NEXT
  ' (RapidR Studio on the web builds the web app, in the page: the other
  ' systems are the desktop Studio's for now)
  IF Proj.FileManager = "" THEN
    FOR i = 0 TO BD_APPS - 1
      BdOn(i) = Flag(i = 6)
    NEXT
  END IF
  nm = Proj.AppName
  IF nm = "" THEN nm = Proj.Name
  IF nm = "" THEN nm = BaseName(ProjectMain)
  BdName.Text = nm
  BdVersion.Text = Proj.Version
  IF BdVersion.Text = "" THEN BdVersion.Text = "1.0.0"
  BdAuthor.Text = Proj.Company
  BdCopyright.Text = Proj.Copyright
  IF BdCopyright.Text = "" AND BdAuthor.Text <> "" THEN BdCopyright.Text = "© " + LEFT$(DATE$, 0) + MID$(DATE$, 7, 4) + " " + BdAuthor.Text
  BdDesc.Text = Proj.Description
  BdSite.Text = Proj.Website
  BdId.Text = Proj.BundleID
  IF BdId.Text = "" THEN BdId.Text = Proj.SuggestedAppId(BdAuthor.Text, nm)
  BdColor.Text = Proj.ThemeColor
  BdCat.ItemIndex = 0
  FOR i = 0 TO 10
    IF BdCatWord(i) = LCASE$(Proj.Category) THEN BdCat.ItemIndex = i
  NEXT
  IF Proj.AppKind = "console" THEN
    BdKindCon.Checked = 1: BdKindWin.Checked = 0
  ELSE
    BdKindWin.Checked = 1: BdKindCon.Checked = 0
  END IF
  IF LEFT$(Proj.Language, 2) = "es" THEN BdLang.ItemIndex = 1 ELSE BdLang.ItemIndex = 0
  BdSource.Checked = Proj.IncludeSource
  BdNative.Checked = Proj.NativeCode
  BdNative.Enabled = Flag(Proj.FileManager <> "" AND Proj.NativeBuilds <> 0)
  ' (the folder in full, the home folder as ~: where the zips really go)
  BdFolder.Text = BdShortPath(Proj.OutputPath())
  IF Proj.PackAs = "one" THEN
    BdPackOne.Checked = 1: BdPackEach.Checked = 0
  ELSE
    BdPackEach.Checked = 1: BdPackOne.Checked = 0
  END IF
  BdFilling = 0
  ' (what was made from the project — the name, the version, the app id —
  ' is kept at once, so it stays the same in every version)
  BdSaveNow
  BdFillFiles
  BdShowIcon
  BdCheckFields
END SUB

' ---- saved as you go -----------------------------------------------------------------

' The dialog's choices into the project, then the project file a moment
' later (a burst of typing is one save).
SUB BdKeep
  DIM i AS INTEGER
  IF BdFilling THEN EXIT SUB
  Proj.BuildFor = BdTicked
  Proj.AppName = BdName.Text
  Proj.Version = BdVersion.Text
  Proj.Company = BdAuthor.Text
  Proj.Copyright = BdCopyright.Text
  Proj.Description = BdDesc.Text
  Proj.Website = BdSite.Text
  Proj.BundleID = BdId.Text
  Proj.ThemeColor = BdColor.Text
  Proj.Category = BdCatWord(BdCat.ItemIndex)
  IF BdKindCon.Checked THEN Proj.AppKind = "console" ELSE Proj.AppKind = "window"
  IF BdLang.ItemIndex = 1 THEN Proj.Language = "es" ELSE Proj.Language = ""
  Proj.IncludeSource = BdSource.Checked
  Proj.NativeCode = BdNative.Checked
  BdKeepFolder
  IF BdPackOne.Checked THEN Proj.PackAs = "one" ELSE Proj.PackAs = "each"
  ' (the Problems pane's notes follow where it runs: checked again when that changes)
  IF Lang.Targets <> Proj.Targets THEN
    Lang.Targets = Proj.Targets
    AnalyzeActive
  END IF
  BdSaveTimer.Enabled = 0
  BdSaveTimer.Enabled = 1
END SUB

SUB BdTick
  IF BdPhase <> "building" THEN
    BdTickTimer.Enabled = 0
    EXIT SUB
  END IF
  IF BdShown THEN
    BdAllNote.Caption = TR$("{n} of {m} done, {s} s", "n", LTRIM$(STR$(BdDoneCount)), "m", LTRIM$(STR$(BdApps)), "s", BdOne(TIMER - BdStarted))
    BdAllNote.Repaint
  END IF
END SUB


' Save in: the folder typed or chosen, kept in the project (its own
' folder's build is kept as none: the default); ~ is the home folder.
SUB BdKeepFolder
  DIM t AS STRING
  t = TRIM$(BdFolder.Text)
  IF t = BdShortPath(Proj.OutputPath()) THEN EXIT SUB
  IF LEFT$(t, 1) = "~" THEN t = ENVIRON$("HOME") + MID$(t, 2)
  IF t = "" OR t = "build" THEN
    Proj.OutputFolder = ""
  ELSE
    Proj.OutputFolder = t
  END IF
  IF Proj.OutputFolder = "build" THEN Proj.OutputFolder = ""
END SUB

SUB BdSaveTick
  BdSaveTimer.Enabled = 0
  BdWrite
END SUB

' (now, not in a moment: before a build, when the dialog closes)
SUB BdSaveNow
  BdKeep
  BdSaveTimer.Enabled = 0
  BdWrite
END SUB

' The project file written quietly (no Output line for every key typed).
SUB BdWrite
  IF ProjectOpen = 0 THEN EXIT SUB
  IF Proj.Save THEN
    ProjectFile = Proj.FileName
    ExtOwnWrite ProjectFile
  END IF
END SUB

SUB BdClosed(Action AS INTEGER)
  IF BdFilling = 0 AND ProjectOpen THEN BdSaveNow
  AnalyzeActive
END SUB

' ---- the steps ---------------------------------------------------------------------------

' Step StepNo (1-4) shown; 5 the build's page.
SUB BdGo(StepNo AS INTEGER)
  IF BdPhase <> "" AND StepNo < 5 THEN BdPhase = ""
  BdStepNo = StepNo
  IF StepNo >= 1 AND StepNo <= 4 THEN BdSteps.ItemIndex = StepNo - 1
  IF StepNo = 5 THEN BdSteps.ItemIndex = 3
  BdLayOut
  BdBody.VertPosition = 0
  ' (the step's heading, said once when it opens)
  SELECT CASE StepNo
    CASE 1: Application.Announce TR$("Step {n} of 4: {name}", "n", "1", "name", BdH1.Caption)
    CASE 2: Application.Announce TR$("Step {n} of 4: {name}", "n", "2", "name", BdH2.Caption)
    CASE 3: Application.Announce TR$("Step {n} of 4: {name}", "n", "3", "name", BdH3.Caption)
    CASE 4: Application.Announce TR$("Step {n} of 4: {name}", "n", "4", "name", BdH4.Caption)
  END SELECT
END SUB

SUB BdStepPicked
  IF BdPhase = "building" THEN
    BdSteps.ItemIndex = 3
    EXIT SUB
  END IF
  IF BdSteps.ItemIndex >= 0 THEN BdGo BdSteps.ItemIndex + 1
END SUB

SUB BdStepKey(Key AS WORD, Shift AS INTEGER)
  IF Key = 13 OR Key = 32 THEN BdStepPicked
END SUB

' The dialog's keys: Ctrl+Tab / Ctrl+Shift+Tab the next / previous step,
' Esc closes (stops a build), F1 Help for this step.
SUB BdKey(Key AS INTEGER, Shift AS INTEGER)
  IF Key = 9 AND (Shift AND 16) <> 0 THEN
    IF BdPhase = "building" THEN EXIT SUB
    IF (Shift AND 256) <> 0 THEN
      IF BdStepNo > 1 THEN BdGo BdStepNo - 1
    ELSE
      IF BdStepNo < 4 THEN BdGo BdStepNo + 1
    END IF
  ELSEIF Key = 27 THEN
    IF BdPhase = "building" THEN BdStop ELSE BuildDialog.Close
  ELSEIF Key = 112 THEN
    HelpLink "manual:build-for-every-system"
  END IF
END SUB

' One step of the list: its mark (the number in a circle; a tick when it
' is done), its name and the one line that sums up its choice.
SUB BdStepDraw(Index AS INTEGER, State AS INTEGER, R AS RRect)
  DIM bg AS LONG, fg AS LONG, dimc AS LONG, cx AS INTEGER, cy AS INTEGER, rad AS INTEGER, cur AS INTEGER, sum AS STRING, done AS INTEGER, tx AS INTEGER, need AS INTEGER
  cur = Flag(Index = BdStepNo - 1 OR (BdStepNo = 5 AND Index = 3))
  bg = TC("face"): fg = TC("page.text"): dimc = TC("page.dim")
  BdSteps.FillRect R.Left, R.Top, R.Right, R.Bottom, bg
  IF cur THEN
    ' (the step shown in the soft "chosen" colour; framed in high contrast,
    ' where every control is)
    IF INSTR(ThemeName, "contrast") > 0 THEN
      BdSteps.RoundRect R.Left + 2, R.Top + 2, R.Right - 2, R.Bottom - 2, 8, 8, TC("status.info.border"), TC("status.info")
    ELSE
      BdSteps.RoundRect R.Left + 2, R.Top + 2, R.Right - 2, R.Bottom - 2, 8, 8, TC("status.info"), TC("status.info")
    END IF
    fg = TC("status.info.text")
  END IF
  sum = FIELD$(BdSteps.Item(Index), CHR$(9), 2)
  need = Flag(Index = 1 AND BdNeedsCare)
  done = Flag((BdPhase = "done" AND Index = 3) OR (cur = 0 AND Index < BdStepNo - 1 AND need = 0))
  rad = 11
  cx = R.Left + 20: cy = R.Top + 22
  IF need THEN
    BdBmp(20).LoadIcon "warning", 16
    BdSteps.Draw cx - 8, cy - 8, BdBmp(20)
  ELSEIF done THEN
    BdBmp(21).LoadIcon "success", 16
    BdSteps.Draw cx - 8, cy - 8, BdBmp(21)
  ELSEIF cur THEN
    BdSteps.Circle cx - rad, cy - rad, cx + rad, cy + rad, TC("accent"), TC("accent")
    BdSteps.TextOut cx - BdSteps.TextWidth(LTRIM$(STR$(Index + 1))) \ 2, cy - BdSteps.TextHeight("1") \ 2, LTRIM$(STR$(Index + 1)), TC("highlight_text"), -1
  ELSE
    BdSteps.Circle cx - rad, cy - rad, cx + rad, cy + rad, dimc, bg
    BdSteps.TextOut cx - BdSteps.TextWidth(LTRIM$(STR$(Index + 1))) \ 2, cy - BdSteps.TextHeight("1") \ 2, LTRIM$(STR$(Index + 1)), dimc, -1
  END IF
  tx = R.Left + 40
  BdSteps.TextOut tx, R.Top + 12, FIELD$(BdSteps.Item(Index), CHR$(9), 1), fg, -1
  BdSteps.TextOut tx, R.Top + 31, FitMiddle(sum, R.Right - tx - 4), dimc, -1
END SUB

' (the steps' list for a screen reader: which step, of four; each item is
' the step's name, the line under it is drawn)
SUB BdStepNames
  BdSteps.AccessibleDescription = TR$("Step {n} of 4. The arrows move, Enter opens a step.", "n", LTRIM$(STR$(BdStepNo)))
END SUB

FUNCTION BdFilesIn AS INTEGER
  DIM i AS INTEGER
  Result = 0
  FOR i = 0 TO BdFileCount - 1
    IF i < BD_FILES THEN
      IF BdFileChk(i).Checked THEN Result = Result + 1
    END IF
  NEXT
END FUNCTION

FUNCTION BdDoneCount AS INTEGER
  DIM i AS INTEGER
  Result = 0
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) AND (BdState(i) = "built" OR BdState(i) = "failed") THEN Result = Result + 1
  NEXT
END FUNCTION

FUNCTION BdBuiltCount AS INTEGER
  DIM i AS INTEGER
  Result = 0
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) AND BdState(i) = "built" THEN Result = Result + 1
  NEXT
END FUNCTION

' ---- the cards and frames ------------------------------------------------------------

' A card: round corners, a 1-pixel edge; a ticked system's card in the soft
' "chosen" colour with an accent edge (Tag 9: the download note, the same).
SUB BdCardPaint(Sender AS RCanvas)
  DIM fill AS LONG, edge AS LONG, w AS INTEGER, h AS INTEGER, on AS INTEGER, i AS INTEGER
  w = Sender.Width: h = Sender.Height
  IF Sender.Tag = 9 THEN
    on = 1
  ELSE
    FOR i = 0 TO BD_APPS - 1
      IF BdCardOf(i) = Sender.Tag AND BdOn(i) THEN on = 1
    NEXT
  END IF
  IF on THEN
    fill = TC("status.info"): edge = TC("accent")
  ELSE
    fill = TC("window"): edge = TC("border")
  END IF
  Sender.FillRect 0, 0, w, h, TC("face")
  Sender.RoundRect 0, 0, w, h, 12, 12, edge
  Sender.RoundRect 1, 1, w - 1, h - 1, 10, 10, fill
  IF INSTR(ThemeName, "contrast") > 0 THEN
    Sender.RoundRect 1, 1, w - 1, h - 1, 10, 10, edge
    Sender.RoundRect 2, 2, w - 2, h - 2, 8, 8, fill
  END IF
END SUB

' (a list's frame: the window's colour, a 1-pixel edge, round corners)
SUB BdFramePaint(Sender AS RCanvas)
  DIM w AS INTEGER, h AS INTEGER
  w = Sender.Width: h = Sender.Height
  Sender.FillRect 0, 0, w, h, TC("face")
  Sender.RoundRect 0, 0, w, h, 12, 12, TC("border")
  Sender.RoundRect 1, 1, w - 1, h - 1, 10, 10, TC("window")
END SUB

' (a hairline: the rail's edge, the footer's top, a row's top)
SUB BdLinePaint(Sender AS RCanvas)
  Sender.FillRect 0, 0, Sender.Width, Sender.Height, TC("border")
END SUB

' (a "coming later" chip: a framed pill, its icon and words greyed)
SUB BdChipPaint(Sender AS RCanvas)
  DIM w AS INTEGER, h AS INTEGER, t AS STRING, ic AS STRING
  w = Sender.Width: h = Sender.Height
  SELECT CASE Sender.Tag
    CASE 0: t = "Android": ic = "target-phone"
    CASE 1: t = "iOS": ic = "target-phone"
    CASE ELSE: t = TR$("Boards: ESP32, micro:bit"): ic = "target-board"
  END SELECT
  Sender.FillRect 0, 0, w, h, TC("face")
  Sender.RoundRect 0, 0, w, h, 8, 8, TC("border")
  Sender.RoundRect 1, 1, w - 1, h - 1, 6, 6, TC("face")
  BdBmp(22 + Sender.Tag).LoadIcon ic, 16, "disabled"
  Sender.Draw 10, (h - 16) \ 2, BdBmp(22 + Sender.Tag)
END SUB

' (a progress bar: Tag -1 the whole build's, else an app's)
SUB BdBarPaint(Sender AS RCanvas)
  DIM w AS INTEGER, h AS INTEGER, part AS DOUBLE, i AS INTEGER
  w = Sender.Width: h = Sender.Height
  IF Sender.Tag < 0 THEN
    IF BdApps > 0 THEN part = BdDoneCount / BdApps
  ELSE
    i = Sender.Tag
    SELECT CASE BdState(i)
      CASE "built", "failed": part = 1
      CASE "working": part = 0.5
      CASE "download": part = 0.2
      CASE ELSE: part = 0
    END SELECT
  END IF
  Sender.FillRect 0, 0, w, h, TC("face")
  Sender.RoundRect 0, 0, w, h, h, h, TC("border"), TC("border")
  IF part > 0 THEN Sender.RoundRect 0, 0, INT(w * part), h, h, h, TC("accent"), TC("accent")
END SUB

' ---- step 1: where it runs ---------------------------------------------------------------

SUB BdSysClick(Sender AS RCheckBox)
  DIM c AS INTEGER, i AS INTEGER, any AS INTEGER
  IF BdFilling THEN EXIT SUB
  c = Sender.Tag
  ' (a system ticked: its first processor too, if none is; unticked: all of them)
  IF Sender.Checked THEN
    FOR i = 0 TO BD_APPS - 1
      IF BdCardOf(i) = c AND BdOn(i) THEN any = 1
    NEXT
    IF any = 0 THEN
      SELECT CASE c
        CASE 0: BdOn(0) = 1
        CASE 1: BdOn(3) = 1
        CASE 2: BdOn(4) = 1
        CASE 3: BdOn(6) = 1
      END SELECT
    END IF
  ELSE
    FOR i = 0 TO BD_APPS - 1
      IF BdCardOf(i) = c THEN BdOn(i) = 0
    NEXT
  END IF
  BdKeep
  BdRefresh
END SUB

SUB BdSysNameClick(Sender AS RLabel)
  BdSys(Sender.Tag).Checked = 1 - BdSys(Sender.Tag).Checked
  BdSysClick BdSys(Sender.Tag)
END SUB

' (a processor: 0-2 Windows x64, ARM64, x86; 3-4 Linux x64, ARM64)
SUB BdArchClick(Sender AS RCheckBox)
  DIM i AS INTEGER
  IF BdFilling THEN EXIT SUB
  SELECT CASE Sender.Tag
    CASE 0, 1, 2: i = Sender.Tag
    CASE 3: i = 4
    CASE ELSE: i = 5
  END SELECT
  BdOn(i) = Flag(Sender.Checked <> 0)
  BdKeep
  BdRefresh
END SUB

SUB BdAllDesktopClick
  DIM i AS INTEGER
  FOR i = 0 TO BD_APPS - 1
    BdOn(i) = Flag(i = 0 OR i = 3 OR i = 4)
  NEXT
  BdKeep
  BdRefresh
END SUB

SUB BdEverythingClick
  DIM i AS INTEGER
  FOR i = 0 TO BD_APPS - 1
    BdOn(i) = 1
  NEXT
  BdKeep
  BdRefresh
END SUB

' ---- step 2: your app ----------------------------------------------------------------------

SUB BdFieldChanged
  IF BdFilling THEN EXIT SUB
  BdKeep
  BdCheckFields
  BdRailRedraw
END SUB

SUB BdKindClick
  IF BdFilling THEN EXIT SUB
  BdKeep
END SUB

SUB BdCategoryPicked
  IF BdFilling THEN EXIT SUB
  BdKeep
END SUB

' (an app id people can't use: letters, digits, - and dots, no space,
' at least two parts)
FUNCTION BdIdProblem(Id AS STRING) AS STRING
  DIM i AS INTEGER, c AS STRING, parts AS INTEGER
  Result = ""
  IF Id = "" THEN Result = TR$("The app needs an id: two words or more with dots between them, like com.example.notepad."): EXIT FUNCTION
  IF INSTR(Id, " ") > 0 THEN Result = TR$("No spaces in an app id: use dots or dashes, like com.example.tip-calculator."): EXIT FUNCTION
  FOR i = 1 TO LEN(Id)
    c = MID$(Id, i, 1)
    IF INSTR("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-.", c) = 0 THEN
      Result = TR$("Only letters, digits, dashes and dots in an app id: {c} can't be in it.", "c", "“" + c + "”")
      EXIT FUNCTION
    END IF
  NEXT
  parts = TALLY(Id, ".") + 1
  IF parts < 2 OR LEFT$(Id, 1) = "." OR RIGHT$(Id, 1) = "." OR INSTR(Id, "..") > 0 THEN Result = TR$("Two words or more with dots between them, like com.example.notepad.")
END FUNCTION

FUNCTION BdVersionProblem(V AS STRING) AS STRING
  DIM i AS INTEGER, c AS STRING
  Result = ""
  IF V = "" THEN Result = TR$("The app needs a version, like 1.0.0."): EXIT FUNCTION
  FOR i = 1 TO LEN(V)
    c = MID$(V, i, 1)
    IF INSTR("0123456789.", c) = 0 THEN Result = TR$("Up to four numbers with dots between them, like 1.0 or 2.3.1."): EXIT FUNCTION
  NEXT
  IF TALLY(V, ".") > 3 OR LEFT$(V, 1) = "." OR RIGHT$(V, 1) = "." OR INSTR(V, "..") > 0 THEN Result = TR$("Up to four numbers with dots between them, like 1.0 or 2.3.1.")
END FUNCTION

FUNCTION BdSiteProblem(S AS STRING) AS STRING
  Result = ""
  IF S = "" THEN EXIT FUNCTION
  IF (LEFT$(S, 8) <> "https://" AND LEFT$(S, 7) <> "http://") OR INSTR(S, " ") > 0 OR LEN(S) < 10 THEN Result = TR$("A web address starting with https://, like https://example.com.")
END FUNCTION

FUNCTION BdColorProblem(S AS STRING) AS STRING
  DIM i AS INTEGER
  Result = ""
  IF S = "" THEN EXIT FUNCTION
  IF LEN(S) <> 7 OR LEFT$(S, 1) <> "#" THEN Result = TR$("A colour as #RRGGBB, like #2F5BFF."): EXIT FUNCTION
  FOR i = 2 TO 7
    IF INSTR("0123456789abcdefABCDEF", MID$(S, i, 1)) = 0 THEN Result = TR$("A colour as #RRGGBB, like #2F5BFF."): EXIT FUNCTION
  NEXT
END FUNCTION

' (step 2 has something a build would refuse)
FUNCTION BdNeedsCare AS INTEGER
  Result = 0
  Result = Flag(BdIdProblem(BdId.Text) <> "" OR BdVersionProblem(BdVersion.Text) <> "" OR BdSiteProblem(BdSite.Text) <> "" OR BdColorProblem(BdColor.Text) <> "" OR TRIM$(BdName.Text) = "")
END FUNCTION

' The fields' problems said under them, in words (and to a screen reader as
' the field's description); the app id's note when it has none.
SUB BdCheckFields
  DIM p AS STRING
  p = BdIdProblem(BdId.Text)
  IF p = "" THEN
    BdIdNote.Caption = TR$("Made from the author and the name. Keep it the same in every version.")
    BdIdNote.Font.Color = TC("page.dim")
    BdId.AccessibleDescription = BdIdNote.Caption
  ELSE
    BdIdNote.Caption = p
    BdIdNote.Font.Color = TC("status.danger.text")
    BdId.AccessibleDescription = p
  END IF
  p = BdVersionProblem(BdVersion.Text)
  IF p = "" THEN BdVersion.AccessibleDescription = "" ELSE BdVersion.AccessibleDescription = p
  BdVersion.Hint = p: BdVersion.ShowHint = Flag(p <> "")
  p = BdSiteProblem(BdSite.Text)
  BdSite.AccessibleDescription = p
  BdSite.Hint = p: BdSite.ShowHint = Flag(p <> "")
  p = BdColorProblem(BdColor.Text)
  IF p <> "" THEN BdColor.AccessibleDescription = p ELSE BdColor.AccessibleDescription = TR$("The web app's colour, #RRGGBB; empty: the page's")
END SUB

' The icon: the project's picture (else RapidR's) big, and at 16 to 48 as
' each system shows it; its file and size; a soft warning when it is small
' or not square.
SUB BdShowIcon
  DIM png AS STRING, k AS INTEGER, px AS INTEGER
  png = Proj.IconPreview(256)
  IF png = "" THEN
    BdIconWarn.Caption = Proj.Error
  ELSE
    BdIconBmp.LoadFromFile png
    BdIconBig.BMPHandle = BdIconBmp
    FOR k = 0 TO 3
      SELECT CASE k
        CASE 0: px = 16
        CASE 1: px = 24
        CASE 2: px = 32
        CASE 3: px = 48
      END SELECT
      png = Proj.IconPreview(px * 2)
      IF png <> "" THEN BdIconSmall(k).LoadFromFile png
    NEXT
    BdIcon16.BMPHandle = BdIconSmall(0)
    BdIcon24.BMPHandle = BdIconSmall(1)
    BdIcon32.BMPHandle = BdIconSmall(2)
    BdIcon48.BMPHandle = BdIconSmall(3)
  END IF
  BdIconWarn.Caption = ""
  IF Proj.Icon = "" THEN
    BdIconFile.Caption = TR$("RapidR's icon for programs.")
    BdIconReset.Visible = 0
  ELSE
    BdIconFile.Caption = FileTitle(Proj.Icon) + BdIconSizeWords
    BdIconReset.Visible = 1
  END IF
  BdIconBig.AccessibleName = TR$("The app's icon") + ": " + BdIconFile.Caption
END SUB

' (", 1024 × 1024" after the icon's file name, and a soft warning under it
' when the picture is small or not square)
FUNCTION BdIconSizeWords AS STRING
  DIM t AS STRING, w AS INTEGER, h AS INTEGER, side AS INTEGER
  t = Proj.IconSize
  Result = ""
  IF t = "" THEN EXIT FUNCTION
  w = VAL(FIELD$(t, CHR$(9), 1)): h = VAL(FIELD$(t, CHR$(9), 2))
  IF FIELD$(t, CHR$(9), 3) = "1" THEN
    Result = ", SVG"
  ELSE
    Result = ", " + LTRIM$(STR$(w)) + " × " + LTRIM$(STR$(h))
    side = w
    IF h > side THEN side = h
    IF side < 512 THEN BdIconWarn.Caption = TR$("This picture is {w} × {h}: the larger sizes look blurry. A 1024 × 1024 PNG or an SVG is sharp everywhere.", "w", LTRIM$(STR$(w)), "h", LTRIM$(STR$(h)))
  END IF
  IF w > 0 AND h > 0 AND w <> h AND BdIconWarn.Caption = "" THEN BdIconWarn.Caption = TR$("It isn't square: it is centred on a transparent square.")
END FUNCTION

SUB BdIconChoose
  OpenDlg.Caption = TR$("The App's Icon")
  OpenDlg.Filter = TR$("Pictures") + " (*.png;*.svg;*.icns;*.ico)|*.png;*.svg;*.icns;*.ico|" + TR$("All files") + "|*.*"
  IF OpenDlg.Execute THEN
    Proj.Icon = OpenDlg.FileName
    BdSaveNow
    BdShowIcon
    BdLayOut
  END IF
END SUB

SUB BdIconDefault
  Proj.Icon = ""
  BdSaveNow
  BdShowIcon
  BdLayOut
END SUB

SUB BdAdvClick(Sender AS RLabel)
  IF Sender.Tag = 2 THEN BdAdv2 = 1 - BdAdv2 ELSE BdAdv3 = 1 - BdAdv3
  BdLayOut
END SUB

' ---- step 3: what goes in ---------------------------------------------------------------

' What goes in, as a build packs it (RPROJECT.BuildFiles: the same list).
SUB BdFillFiles
  DIM t AS STRING, n AS INTEGER, i AS INTEGER, ln AS STRING
  t = Proj.BuildFiles()
  BdFileCount = 0: BdOutCount = 0
  IF t <> "" THEN n = TALLY(t, CHR$(10)) + 1 ELSE n = 0
  BdFilling = 1
  FOR i = 1 TO n
    ln = FIELD$(t, CHR$(10), i)
    IF FIELD$(ln, CHR$(9), 1) = "in" AND BdFileCount < BD_FILES THEN
      BdFilePath(BdFileCount) = FIELD$(ln, CHR$(9), 2)
      BdFileBytes(BdFileCount) = VAL(FIELD$(ln, CHR$(9), 3))
      BdFileChk(BdFileCount).Checked = Flag(FIELD$(ln, CHR$(9), 4) = "1")
      BdFileCount = BdFileCount + 1
    ELSEIF FIELD$(ln, CHR$(9), 1) = "out" AND BdOutCount < BD_OUTS THEN
      BdOutName(BdOutCount) = FIELD$(ln, CHR$(9), 2)
      BdOutWhy(BdOutCount) = FIELD$(ln, CHR$(9), 3)
      BdOutCount = BdOutCount + 1
    END IF
  NEXT
  BdFilling = 0
END SUB

' (what kind of file, in a word, and its icon)
FUNCTION BdFileWord(Path AS STRING) AS STRING
  SELECT CASE FileExt(Path)
    CASE "png", "jpg", "jpeg", "gif", "bmp", "svg", "ico", "webp": Result = TR$("image")
    CASE "wav", "mp3", "ogg", "flac", "mid", "midi": Result = TR$("sound")
    CASE "csv", "json", "xml", "db", "sqlite", "dat", "tsv": Result = TR$("data")
    CASE "txt", "md", "ini", "cfg": Result = TR$("text")
    CASE "po": Result = TR$("translation")
    CASE "ttf", "otf": Result = TR$("font")
    CASE ELSE: Result = TR$("file")
  END SELECT
END FUNCTION

FUNCTION BdFileIconName(Path AS STRING) AS STRING
  SELECT CASE FileExt(Path)
    CASE "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp": Result = "files/image"
    CASE "svg": Result = "files/svg"
    CASE "wav", "mp3", "ogg", "flac": Result = "files/audio"
    CASE "mid", "midi": Result = "files/midi"
    CASE "csv", "dat", "tsv", "db", "sqlite": Result = "files/data"
    CASE "json": Result = "files/json"
    CASE "xml": Result = "files/xml"
    CASE "po": Result = "files/translation"
    CASE "ttf", "otf": Result = "files/font"
    CASE ELSE: Result = "files/text"
  END SELECT
END FUNCTION

FUNCTION BdOutWords(Why AS STRING) AS STRING
  SELECT CASE Why
    CASE "written": Result = TR$("The program writes it: your copy may hold your own data.")
    CASE "outside": Result = TR$("Outside the project folder: copy it in to take it along.")
    CASE ELSE: Result = TR$("The program never reads it.")
  END SELECT
END FUNCTION

SUB BdFileNameClick(Sender AS RLabel)
  BdFileChk(Sender.Tag).Checked = 1 - BdFileChk(Sender.Tag).Checked
  BdFileClick BdFileChk(Sender.Tag)
END SUB

SUB BdFileClick(Sender AS RCheckBox)
  DIM i AS INTEGER, left AS STRING
  IF BdFilling THEN EXIT SUB
  FOR i = 0 TO BdFileCount - 1
    IF BdFileChk(i).Checked = 0 THEN left = left + BdFilePath(i) + CHR$(10)
  NEXT
  Proj.LeaveOut = left
  BdKeep
  BdRefresh
END SUB

' Add Files...: files the program reads, added to the project as its data
' (they go in every app).
SUB BdAddFiles
  OpenDlg.Caption = TR$("Add Files")
  OpenDlg.Filter = TR$("All files") + "|*.*"
  IF OpenDlg.Execute THEN
    IF Proj.AddFile(OpenDlg.FileName, "data") THEN
      BdSaveNow
      FillProjectTree
    ELSE
      OutputLine TR$("Can't add {file}: {error}", "file", FileTitle(OpenDlg.FileName), "error", Proj.Error)
    END IF
    BdFillFiles
    BdLayOut
  END IF
END SUB

SUB BdOptionClick
  IF BdFilling THEN EXIT SUB
  BdKeep
  BdRefresh
END SUB

' ---- step 4: ready to build ---------------------------------------------------------------

SUB BdFolderChanged
  IF BdFilling THEN EXIT SUB
  BdKeep
END SUB

SUB BdFolderChoose
  OpenDlg.Caption = TR$("Where the Zips Go")
  OpenDlg.PickFolder = 1
  IF OpenDlg.Execute THEN BdFolder.Text = OpenDlg.FileName
  OpenDlg.PickFolder = 0
END SUB

SUB BdPackClick
  IF BdFilling THEN EXIT SUB
  BdKeep
END SUB

' ---- the footer --------------------------------------------------------------------------

SUB BdNextClick
  SELECT CASE BdPhase
    CASE "building": BdStop
    CASE "done": BdShowMade
    CASE ELSE
      IF BdStepNo < 4 AND BdNarrow = 0 THEN
        BdGo BdStepNo + 1
      ELSE
        BdStart
      END IF
  END SELECT
END SUB

SUB BdBackClick
  IF BdStepNo > 1 THEN BdGo BdStepNo - 1
END SUB

' (the footer's second button: Back while choosing, Try the Web App when done)
SUB BdSecondClick
  IF BdPhase = "done" THEN
    BuildDialog.Close
    RunCommand "run.browser"
  ELSE
    BdBackClick
  END IF
END SUB

SUB BdMoreClick
  BdMoreItem(1).Enabled = Flag(BdPhase = "done" AND BdBuiltCount > 1 AND Proj.FileManager <> "")
  BdMoreMenu.Popup Screen.MouseX, Screen.MouseY
END SUB

SUB BdMorePicked(Sender AS RMenuItem)
  DIM p AS STRING
  SELECT CASE Sender.Tag
    CASE 0
      BdSaveNow
      Clipboard.Text = Proj.BuildCommandLine()
      BdToastId = Toast(TR$("The command line was copied"), "success", "", "build")
    CASE 1
      p = Proj.PackOneZip(BdMade)
      IF p <> "" THEN
        BdToastId = Toast(TR$("{file} holds every app", "file", FileTitle(p)), "success", TR$("Show"), "build")
      ELSE
        BdToastId = Toast(TR$("Can't make one zip: {error}", "error", Proj.Error), "error", "", "build")
      END IF
    CASE 2
      HelpLink "manual:build-for-every-system"
  END SELECT
END SUB

' ---- laying it out -----------------------------------------------------------------------

SUB BdResized
  IF BdShown THEN BdLayOut
END SUB

' (the keyboard on the step's first choice as the dialog opens, as the
' design's Tab order starts: the first system, the name, the first file)
SUB BdShownNow
  BdFocusFirst
END SUB

SUB BdFocusFirst
  IF BdPhase <> "" THEN
    BdNext.SetFocus
    EXIT SUB
  END IF
  SELECT CASE BdStepNo
    CASE 1: BdSys(0).SetFocus
    CASE 2: BdName.SetFocus
    CASE 3
      IF BdFileCount > 0 THEN BdFileChk(0).SetFocus ELSE BdNext.SetFocus
    CASE ELSE: BdNext.SetFocus
  END SELECT
END SUB

' Everything placed for the step shown, the look and the width: the steps
' at the left (one page under 640 px), the footer pinned, the page between.
SUB BdLayOut
  DIM cw AS INTEGER, ch AS INTEGER, rail AS INTEGER, pad AS INTEGER, foot AS INTEGER, bh AS INTEGER, bw AS INTEGER, x AS INTEGER, y AS INTEGER, w AS INTEGER, fh AS INTEGER, k AS INTEGER, top AS INTEGER, pageH AS INTEGER
  IF BdRowsMade = 0 THEN EXIT SUB
  cw = BuildDialog.ClientWidth: ch = BuildDialog.ClientHeight
  bh = UiButtonH: fh = UiCompactH
  IF ThemeName = "classic" THEN bw = 84 ELSE bw = 96
  pad = 20
  BdNarrow = Flag(cw < 640)
  IF BdNarrow THEN pad = 16
  foot = bh + 2 * 16
  BuildDialog.Color = TC("face")
  BuildDialog.Font.Name = UIFont.Name: BuildDialog.Font.Size = UIFont.Size
  BdApplyColors
  ' (the steps: 204 wide; none when narrow)
  IF BdNarrow THEN
    rail = 0
    BdRail.Visible = 0: BdRailLine.Visible = 0
  ELSE
    rail = 204
    BdRail.Visible = 1: BdRailLine.Visible = 1
    BdRail.Left = 0: BdRail.Top = 0: BdRail.Width = rail: BdRail.Height = ch - foot
    BdRail.Color = TC("face")
    BdRailLine.Left = rail: BdRailLine.Top = 0: BdRailLine.Width = 1: BdRailLine.Height = ch - foot
    IF ThemeName = "classic" THEN BdSteps.ItemHeight = 40 ELSE BdSteps.ItemHeight = 52
    BdSteps.Left = 8: BdSteps.Top = 12: BdSteps.Width = rail - 12: BdSteps.Height = 4 * BdSteps.ItemHeight + 4
    BdSteps.Color = TC("face")
    BdSavedNote.Width = rail - 50
    BdSavedNote.Height = BdTextHigh(BdSavedNote, rail - 50)
    BdSavedNote.Left = 38: BdSavedNote.Top = BdRail.Height - 14 - BdSavedNote.Height
    BdSavedIcon.Left = 14: BdSavedIcon.Top = BdSavedNote.Top + 2
  END IF
  ' (the footer)
  BdFoot.Left = 0: BdFoot.Top = ch - foot: BdFoot.Width = cw: BdFoot.Height = foot
  BdFoot.Color = TC("face")
  BdFootLine.Left = 0: BdFootLine.Top = 0: BdFootLine.Width = cw: BdFootLine.Height = 1
  BdMore.Left = pad: BdMore.Top = 16: BdMore.Width = bh: BdMore.Height = bh
  BdNext.Height = bh: BdNext.Top = 16
  BdSecond.Height = bh: BdSecond.Top = 16
  BdFootButtons
  BdNext.Width = UITextWidth(StripAmp(BdNext.Caption)) + 32
  IF BdNext.Width < bw THEN BdNext.Width = bw
  BdNext.Left = cw - pad - BdNext.Width
  BdSecond.Width = UITextWidth(StripAmp(BdSecond.Caption)) + 32
  IF BdSecond.Width < bw THEN BdSecond.Width = bw
  BdSecond.Left = BdNext.Left - 8 - BdSecond.Width
  BdFootNote.Left = BdMore.Left + BdMore.Width + 12
  BdFootNote.Top = 16 + (bh - 18) \ 2: BdFootNote.Height = 18
  IF BdSecond.Visible THEN
    BdFootNote.Width = BdSecond.Left - 12 - BdFootNote.Left
  ELSE
    BdFootNote.Width = BdNext.Left - 12 - BdFootNote.Left
  END IF
  IF BdNarrow THEN BdFootNote.Visible = 0 ELSE BdFootNote.Visible = 1
  ' (the pages)
  BdBody.Left = rail + 1: BdBody.Top = 0: BdBody.Width = cw - rail - 1: BdBody.Height = ch - foot
  BdBody.Color = TC("face")
  w = BdBody.Width - 2 * pad - 16
  IF w < 200 THEN w = 200
  BdP1.Visible = 0: BdP2.Visible = 0: BdP3.Visible = 0: BdP4.Visible = 0: BdP5.Visible = 0
  top = 0
  IF BdPhase <> "" THEN
    pageH = BdLayP5(pad, w)
    BdP5.Left = 0: BdP5.Top = 0: BdP5.Width = BdBody.Width - 16: BdP5.Height = pageH: BdP5.Visible = 1
  ELSEIF BdNarrow THEN
    ' (one page: the four steps down it, each with its number)
    pageH = BdLayP1(pad, w): BdP1.Left = 0: BdP1.Top = top: BdP1.Width = BdBody.Width - 16: BdP1.Height = pageH: BdP1.Visible = 1: top = top + pageH
    pageH = BdLayP2(pad, w): BdP2.Left = 0: BdP2.Top = top: BdP2.Width = BdBody.Width - 16: BdP2.Height = pageH: BdP2.Visible = 1: top = top + pageH
    pageH = BdLayP3(pad, w): BdP3.Left = 0: BdP3.Top = top: BdP3.Width = BdBody.Width - 16: BdP3.Height = pageH: BdP3.Visible = 1: top = top + pageH
    pageH = BdLayP4(pad, w): BdP4.Left = 0: BdP4.Top = top: BdP4.Width = BdBody.Width - 16: BdP4.Height = pageH: BdP4.Visible = 1
  ELSE
    SELECT CASE BdStepNo
      CASE 1: pageH = BdLayP1(pad, w): BdP1.Left = 0: BdP1.Top = 0: BdP1.Width = BdBody.Width - 16: BdP1.Height = pageH: BdP1.Visible = 1
      CASE 2: pageH = BdLayP2(pad, w): BdP2.Left = 0: BdP2.Top = 0: BdP2.Width = BdBody.Width - 16: BdP2.Height = pageH: BdP2.Visible = 1
      CASE 3: pageH = BdLayP3(pad, w): BdP3.Left = 0: BdP3.Top = 0: BdP3.Width = BdBody.Width - 16: BdP3.Height = pageH: BdP3.Visible = 1
      CASE ELSE: pageH = BdLayP4(pad, w): BdP4.Left = 0: BdP4.Top = 0: BdP4.Width = BdBody.Width - 16: BdP4.Height = pageH: BdP4.Visible = 1
    END SELECT
  END IF
  BdStepNames
  BdRailRedraw
END SUB

' The steps' list drawn again: its items made anew (an owner-drawn list
' keeps each item's drawing), the step shown still chosen.
SUB BdRailRedraw
  DIM k AS INTEGER, t(3) AS STRING, same AS INTEGER, i AS INTEGER
  t(0) = TR$("Where it runs") + CHR$(9) + BdStepSum(0)
  t(1) = TR$("Your app") + CHR$(9) + BdStepSum(1)
  t(2) = TR$("What goes in") + CHR$(9) + BdStepSum(2)
  t(3) = TR$("Build", "the Build dialog") + CHR$(9) + BdStepSum(3)
  same = Flag(BdSteps.ItemCount = 4)
  IF same THEN
    FOR i = 0 TO 3
      IF BdSteps.Item(i) <> t(i) THEN same = 0
    NEXT
  END IF
  IF same THEN EXIT SUB
  k = BdSteps.ItemIndex
  BdSteps.Clear
  BdSteps.AddItems t(0), t(1), t(2), t(3)
  BdSteps.ItemIndex = k
END SUB

' The line under step Index in the list: what its choice is now.
FUNCTION BdStepSum(Index AS INTEGER) AS STRING
  SELECT CASE Index
    CASE 0: Result = BdSystemWords
    CASE 1
      IF BdNeedsCare THEN Result = TR$("Needs attention") ELSE Result = BdName.Text + " " + BdVersion.Text
    CASE 2: Result = TRN$("{n} data file", "{n} data files", BdFilesIn)
    CASE ELSE
      IF BdPhase = "building" THEN
        Result = TR$("Building: {n} of {m} done", "n", LTRIM$(STR$(BdDoneCount)), "m", LTRIM$(STR$(BdApps)))
      ELSEIF BdPhase = "done" THEN
        Result = TRN$("{n} app built", "{n} apps built", BdBuiltCount)
      ELSE
        Result = TRN$("{n} app", "{n} apps", BdApps)
      END IF
  END SELECT
END FUNCTION

' (a caption without its mnemonic's &)
FUNCTION StripAmp(T AS STRING) AS STRING
  Result = REPLACESUBSTR$(T, "&", "")
END FUNCTION

' The footer's buttons and line for the moment: Next (Build N Apps on
' step 4 and on the one page), Stop while building, Show in Finder when
' done; Back on steps 2-4, Try the Web App when the web app was built.
SUB BdFootButtons
  BdSecond.Visible = 0
  SELECT CASE BdPhase
    CASE "building"
      BdNext.Caption = TR$("&Stop")
      BdFootNote.Caption = TR$("Esc stops the build; nothing half-made is kept.")
    CASE "done"
      IF Proj.FileManager = "" THEN
        BdNext.Caption = TR$("&Done")
      ELSE
        BdNext.Caption = TR$("&Show in {manager}", "manager", Proj.FileManager)
      END IF
      IF BdOn(6) AND BdState(6) = "built" AND Proj.FileManager <> "" THEN
        BdSecond.Caption = TR$("&Try the Web App")
        BdSecond.Visible = 1
      END IF
      BdFootNote.Caption = TR$("Build Again any time: Ctrl+Shift+B")
    CASE ELSE
      IF BdStepNo >= 4 OR BdNarrow THEN
        BdNext.Caption = TRN$("&Build {n} App", "&Build {n} Apps", BdApps)
        BdNext.Enabled = Flag(BdApps > 0 AND BdNeedsCare = 0)
      ELSE
        BdNext.Caption = TR$("&Next")
        BdNext.Enabled = 1
      END IF
      IF BdStepNo > 1 AND BdNarrow = 0 THEN
        BdSecond.Caption = TR$("&Back")
        BdSecond.Visible = 1
      END IF
      IF BdStepNo >= 4 THEN
        BdFootNote.Caption = BdCountWords + ", " + TR$("about {size}", "size", BdSize(BdTotalBytes))
        IF BdTotalBytes = 0 THEN BdFootNote.Caption = BdCountWords
      ELSE
        BdFootNote.Caption = BdCountWords
      END IF
  END SELECT
  IF BdPhase <> "" THEN BdNext.Enabled = 1
END SUB

' (the colours of the look, and the fonts)
SUB BdApplyColors
  DIM k AS INTEGER
  BdH1.Font = SubFont: BdH2.Font = SubFont: BdH3.Font = SubFont: BdH4.Font = SubFont: BdH5.Font = SubFont
  BdH1.Font.Color = TC("page.text"): BdH2.Font.Color = TC("page.text"): BdH3.Font.Color = TC("page.text"): BdH4.Font.Color = TC("page.text"): BdH5.Font.Color = TC("page.text")
  BdS1.Font.Color = TC("page.dim"): BdS2.Font.Color = TC("page.dim"): BdS3.Font.Color = TC("page.dim"): BdS4.Font.Color = TC("page.dim"): BdS5.Font.Color = TC("page.dim")
  BdAllDesktop.Font = LinkFont: BdAllDesktop.Font.Color = TC("page.link")
  BdEverything.Font = LinkFont: BdEverything.Font.Color = TC("page.link")
  BdIconReset.Font = LinkFont: BdIconReset.Font.Color = TC("page.link")
  BdSavedNote.Font.Color = TC("page.dim")
  BdFootNote.Font.Color = TC("page.dim")
  BdHelpText.Font.Color = TC("page.dim")
  BdLaterLabel.Font = CardFont: BdLaterLabel.Font.Color = TC("page.text")
  BdKindNote.Font.Color = TC("page.dim")
  ' (the app id's note: dimmed, or its problem in the danger colour)
  IF BdIdProblem(BdId.Text) = "" THEN BdIdNote.Font.Color = TC("page.dim") ELSE BdIdNote.Font.Color = TC("status.danger.text")
  BdIconHead.Font = CardFont: BdIconHead.Font.Color = TC("page.text")
  BdIconFile.Font.Color = TC("page.dim"): BdIconNote.Font.Color = TC("page.dim")
  BdIconWarn.Font.Color = TC("status.warning.text")
  BdProgName.Font = CardFont: BdProgName.Font.Color = TC("page.text")
  BdProgNote.Font.Color = TC("page.dim")
  BdDataHead.Font = CardFont: BdDataHead.Font.Color = TC("page.text")
  BdOutHead.Font = CardFont: BdOutHead.Font.Color = TC("page.text")
  BdNoFiles.Font.Color = TC("page.dim")
  BdSizeNote.Font.Color = TC("page.dim")
  BdNativeNote.Font.Color = TC("page.dim")
  BdReadmeNote.Font.Color = TC("page.dim")
  BdDlHead.Font = CardFont: BdDlHead.Font.Color = TC("status.info.text")
  BdDlText.Font.Color = TC("status.info.text")
  BdAllNote.Font.Color = TC("page.dim")
  BdFirstHead.Font = CardFont: BdFirstHead.Font.Color = TC("page.text")
  BdFirstFoot.Font.Color = TC("page.dim")
  BdSz16.Font.Color = TC("page.dim"): BdSz24.Font.Color = TC("page.dim"): BdSz32.Font.Color = TC("page.dim"): BdSz48.Font.Color = TC("page.dim")
  BdP1.Color = TC("face"): BdP2.Color = TC("face"): BdP3.Color = TC("face"): BdP4.Color = TC("face"): BdP5.Color = TC("face")
  BdSavedIcon.BMPHandle = BdBmp(0)
  BdBmp(0).LoadIcon "files/rrproj", 16
  BdSavedIcon.BMPHandle = BdBmp(0)
  BdBmp(1).LoadIcon "help", 16
  BdHelpIcon.BMPHandle = BdBmp(1)
  BdBmp(2).LoadIcon "files/rr", 16
  BdProgIcon.BMPHandle = BdBmp(2)
  BdBmp(3).LoadIcon "files/archive", 16
  BdSizeIcon.BMPHandle = BdBmp(3)
  BdBmp(4).LoadIcon "components/download", 16
  BdDlIcon.BMPHandle = BdBmp(4)
  BdBmp(5).LoadIcon "files/text", 16
  BdReadmeIcon.BMPHandle = BdBmp(5)
  FOR k = 0 TO 3
    BdSysBmp(k).LoadIcon BdIconName(k), 24
    BdSysIcon(k).BMPHandle = BdSysBmp(k)
  NEXT
END SUB

' (a card's colour: the soft "chosen" one when a system on it is ticked)
FUNCTION BdCardFill(Card AS INTEGER) AS LONG
  DIM i AS INTEGER, on AS INTEGER
  Result = 0
  FOR i = 0 TO BD_APPS - 1
    IF BdCardOf(i) = Card AND BdOn(i) THEN on = 1
  NEXT
  IF on THEN Result = TC("status.info") ELSE Result = TC("window")
END FUNCTION

' A label's height for its text at a width (wrapped), in its font.
FUNCTION BdTextHigh(L AS RLabel, W AS INTEGER) AS INTEGER
  DIM t AS STRING, lines AS INTEGER, lh AS INTEGER, k AS INTEGER, wd AS STRING, cur AS INTEGER, sp AS INTEGER, ww AS INTEGER, room AS INTEGER
  Result = 0
  t = REPLACESUBSTR$(L.Caption, "&&", "&")
  IF t = "" THEN EXIT FUNCTION
  ' (the label's own font — the dialog's when it names none)
  IF L.Font.Name <> "" THEN BdMeasure.Font.Name = L.Font.Name ELSE BdMeasure.Font.Name = UIFont.Name
  IF L.Font.Size > 0 THEN BdMeasure.Font.Size = L.Font.Size ELSE BdMeasure.Font.Size = UIFont.Size
  BdMeasure.Font.Bold = L.Font.Bold
  lh = BdMeasure.TextHeight("Ag") + 2
  sp = BdMeasure.TextWidth(" ")
  room = W - 10
  ' (the words laid out as the label wraps them: a word that doesn't fit
  ' starts the next line)
  lines = 1: cur = 0
  t = t + " "
  DO WHILE LEN(t) > 0
    k = INSTR(t, " ")
    wd = LEFT$(t, k - 1)
    t = MID$(t, k + 1)
    ww = BdMeasure.TextWidth(wd)
    IF cur = 0 THEN
      cur = ww
    ELSEIF cur + sp + ww <= room THEN
      cur = cur + sp + ww
    ELSE
      lines = lines + 1
      cur = ww
    END IF
  LOOP
  Result = lines * lh
END FUNCTION

' (a step's number in the accent's circle: the one page, narrow)
SUB BdNumPaint(Sender AS RCanvas)
  Sender.FillRect 0, 0, Sender.Width, Sender.Height, TC("face")
  Sender.Circle 0, 0, Sender.Width, Sender.Height, TC("accent"), TC("accent")
END SUB

' The number before step N's heading when narrow (the heading moves right of
' it); none on the wide dialog, where the steps' list numbers them.
SUB BdNumPlace(N AS INTEGER, H AS RLabel, X AS INTEGER, Y AS INTEGER)
  BdNum(N).Visible = BdNarrow: BdNumText(N).Visible = BdNarrow
  IF BdNarrow THEN
    BdNum(N).Left = X: BdNum(N).Top = Y: BdNum(N).Width = 24: BdNum(N).Height = 24
    BdNum(N).Repaint
    BdNumText(N).Left = X: BdNumText(N).Top = Y + 3: BdNumText(N).Width = 24: BdNumText(N).Height = 18
    BdNumText(N).Font.Color = TC("highlight_text")
    BdNumText(N).Font.Bold = 1
    H.Left = X + 34
  ELSE
    H.Left = X
  END IF
END SUB

' Step 1's page; its height.
FUNCTION BdLayP1(Pad AS INTEGER, W AS INTEGER) AS INTEGER
  DIM y AS INTEGER, x AS INTEGER, cwid AS INTEGER, c AS INTEGER, cx AS INTEGER, cy AS INTEGER, ch(3) AS INTEGER, row AS INTEGER, k AS INTEGER, inner AS INTEGER, lx AS INTEGER, i AS INTEGER, rowH AS INTEGER, lw AS INTEGER
  x = Pad: y = Pad
  BdH1.Caption = TR$("Where should it run?")
  BdH1.Top = y
  BdNumPlace 0, BdH1, x, y
  ' (the quick choices at the right of the heading; under it when narrow)
  BdFit BdEverything
  BdFit BdAllDesktop
  BdEverything.Top = y + 4: BdAllDesktop.Top = y + 4
  BdEverything.Left = x + W - BdEverything.Width
  BdAllDesktop.Left = BdEverything.Left - 24 - BdAllDesktop.Width
  y = y + 26
  BdS1.Left = x: BdS1.Top = y: BdS1.Width = W
  BdS1.Height = BdTextHigh(BdS1, BdS1.Width)
  y = y + BdS1.Height + 16
  IF BdNarrow THEN
    BdAllDesktop.Left = x: BdAllDesktop.Top = y
    BdEverything.Left = x + BdAllDesktop.Width + 24: BdEverything.Top = y
    y = y + 28
  END IF
  ' (the cards: two columns, one when narrow)
  IF BdNarrow THEN cwid = W ELSE cwid = (W - 12) \ 2
  inner = cwid - 56
  IF ThemeName = "classic" THEN rowH = 22 ELSE rowH = 22
  ' (each card placed, its height from its words — a longer language wraps,
  ' never clips —; two in a row are as tall as the taller)
  IF BdNarrow THEN
    FOR c = 0 TO 3
      ch(c) = BdPlaceCard(c, x, y, cwid)
      BdCardSize c, x, y, cwid, ch(c)
      y = y + ch(c) + 12
    NEXT
    y = y + 4
  ELSE
    ch(0) = BdPlaceCard(0, x, y, cwid)
    ch(1) = BdPlaceCard(1, x + cwid + 12, y, cwid)
    IF ch(1) > ch(0) THEN ch(0) = ch(1)
    BdCardSize 0, x, y, cwid, ch(0)
    BdCardSize 1, x + cwid + 12, y, cwid, ch(0)
    y = y + ch(0) + 12
    ch(2) = BdPlaceCard(2, x, y, cwid)
    ch(3) = BdPlaceCard(3, x + cwid + 12, y, cwid)
    IF ch(3) > ch(2) THEN ch(2) = ch(3)
    BdCardSize 2, x, y, cwid, ch(2)
    BdCardSize 3, x + cwid + 12, y, cwid, ch(2)
    y = y + ch(2) + 16
  END IF
  ' (the 32-bit note shows only when it's ticked; Linux's when Linux is)
  BdCardNote(0).Visible = BdOn(2)
  BdHelpIcon.Left = x: BdHelpIcon.Top = y + 2
  BdHelpText.Left = x + 24: BdHelpText.Top = y: BdHelpText.Width = W - 24
  BdHelpText.Height = BdTextHigh(BdHelpText, W - 24)
  y = y + BdHelpText.Height + 16
  BdLaterLabel.Left = x: BdLaterLabel.Top = y + 7
  lx = x + BdLaterLabel.Width + 12
  IF BdNarrow THEN
    lx = x
    y = y + 28
  END IF
  BdMeasure.Font.Name = UIFont.Name: BdMeasure.Font.Size = UIFont.Size: BdMeasure.Font.Bold = 0
  lw = BdMeasure.TextWidth("Android") + 44
  BdLater0.Left = lx: BdLater0.Top = y: BdLater0.Width = lw: BdLater0.Height = 32
  lx = lx + lw + 8
  lw = BdMeasure.TextWidth("iOS") + 44
  BdLater1.Left = lx: BdLater1.Top = y: BdLater1.Width = lw: BdLater1.Height = 32
  lx = lx + lw + 8
  lw = BdMeasure.TextWidth(TR$("Boards: ESP32, micro:bit")) + 44
  IF lx + lw > x + W THEN
    lx = x
    y = y + 40
  END IF
  BdLater2.Left = lx: BdLater2.Top = y: BdLater2.Width = lw: BdLater2.Height = 32
  BdLater0.Repaint: BdLater1.Repaint: BdLater2.Repaint
  BdLaterText(0).Caption = "Android": BdLaterText(1).Caption = "iOS": BdLaterText(2).Caption = TR$("Boards: ESP32, micro:bit")
  BdLaterText(0).Left = BdLater0.Left + 32: BdLaterText(0).Top = BdLater0.Top + 7
  BdLaterText(1).Left = BdLater1.Left + 32: BdLaterText(1).Top = BdLater1.Top + 7
  BdLaterText(2).Left = BdLater2.Left + 32: BdLaterText(2).Top = BdLater2.Top + 7
  FOR k = 0 TO 2
    BdLaterText(k).Font.Color = TC("page.dim")
    BdLaterText(k).Color = TC("face")
  NEXT
  y = y + 32 + Pad
  BdLayP1Ticks
  Result = y
END FUNCTION

' A card's canvas at its place and size.
SUB BdCardSize(C AS INTEGER, CX AS INTEGER, CY AS INTEGER, CW AS INTEGER, CH AS INTEGER)
  SELECT CASE C
    CASE 0: BdCard0.Left = CX: BdCard0.Top = CY: BdCard0.Width = CW: BdCard0.Height = CH: BdCard0.Repaint
    CASE 1: BdCard1.Left = CX: BdCard1.Top = CY: BdCard1.Width = CW: BdCard1.Height = CH: BdCard1.Repaint
    CASE 2: BdCard2.Left = CX: BdCard2.Top = CY: BdCard2.Width = CW: BdCard2.Height = CH: BdCard2.Repaint
    CASE 3: BdCard3.Left = CX: BdCard3.Top = CY: BdCard3.Width = CW: BdCard3.Height = CH: BdCard3.Repaint
  END SELECT
END SUB

' Card C's parts placed at (CX, CY), CW wide: its box, picture and name,
' then its processors (each with its note, wrapped when long) or its line,
' then its note; its height.
FUNCTION BdPlaceCard(C AS INTEGER, CX AS INTEGER, CY AS INTEGER, CW AS INTEGER) AS INTEGER
  DIM y AS INTEGER, i AS INTEGER, inner AS INTEGER, nw AS INTEGER, nh AS INTEGER
  inner = CW - 56
  BdSys(C).Left = CX + 14: BdSys(C).Top = CY + 14: BdSys(C).Width = 20: BdSys(C).Height = 20
  BdSys(C).Color = BdCardFill(C)
  BdSysIcon(C).Left = CX + 44: BdSysIcon(C).Top = CY + 14
  BdSysName(C).Font = CardFont: BdSysName(C).Font.Color = TC("page.text")
  BdFit BdSysName(C)
  BdSysName(C).Left = CX + 74: BdSysName(C).Top = CY + 16
  y = CY + 44
  BdSysText(C).Visible = Flag(C = 1 OR C = 3)
  IF C = 1 OR C = 3 THEN
    BdSysText(C).Left = CX + 42: BdSysText(C).Top = y + 2: BdSysText(C).Width = inner
    BdSysText(C).Font.Color = TC("page.dim")
    BdSysText(C).Height = BdTextHigh(BdSysText(C), inner)
    y = y + BdSysText(C).Height + 4
  END IF
  ' (Windows' and Linux's processors, a row each; dimmed and skipped until
  ' the system is ticked)
  FOR i = 0 TO 4
    IF (C = 0 AND i <= 2) OR (C = 2 AND i >= 3) THEN
      BdArch(i).Left = CX + 42: BdArch(i).Top = y: BdArch(i).Height = 20
      BdArch(i).Width = UITextWidth(BdArch(i).Caption) + 28
      BdArch(i).Color = BdCardFill(C)
      BdArchNote(i).AutoSize = 0: BdArchNote(i).WordWrap = 1
      BdArchNote(i).Left = BdArch(i).Left + BdArch(i).Width + 2: BdArchNote(i).Top = y + 2
      nw = CX + CW - 12 - BdArchNote(i).Left
      IF nw < 60 THEN nw = 60
      BdArchNote(i).Width = nw
      BdArchNote(i).Font.Color = TC("page.dim")
      nh = BdTextHigh(BdArchNote(i), nw)
      IF nh < 18 THEN nh = 18
      BdArchNote(i).Height = nh
      y = y + nh + 4
    END IF
  NEXT
  BdCardNote(C).Visible = Flag(C = 0 OR C = 2)
  IF C = 0 OR C = 2 THEN
    IF C = 0 THEN BdCardNote(C).Left = CX + 70 ELSE BdCardNote(C).Left = CX + 42
    BdCardNote(C).Width = CX + CW - 12 - BdCardNote(C).Left
    BdCardNote(C).Font.Color = TC("page.dim")
    BdCardNote(C).Top = y + 2
    BdCardNote(C).Height = BdTextHigh(BdCardNote(C), BdCardNote(C).Width)
    y = y + BdCardNote(C).Height + 4
  END IF
  Result = y - CY + 12
END FUNCTION

' (the ticks as the choices are: a system ticked when one of its apps is)
SUB BdLayP1Ticks
  DIM c AS INTEGER, i AS INTEGER, on AS INTEGER
  BdFilling = 1
  FOR c = 0 TO 3
    on = 0
    FOR i = 0 TO BD_APPS - 1
      IF BdCardOf(i) = c AND BdOn(i) THEN on = 1
    NEXT
    BdSys(c).Checked = on
  NEXT
  BdArch(0).Checked = BdOn(0): BdArch(1).Checked = BdOn(1): BdArch(2).Checked = BdOn(2)
  BdArch(3).Checked = BdOn(4): BdArch(4).Checked = BdOn(5)
  FOR i = 0 TO 2
    BdArch(i).Enabled = BdSys(0).Checked
  NEXT
  BdArch(3).Enabled = BdSys(2).Checked: BdArch(4).Enabled = BdSys(2).Checked
  ' (the web Studio: the web app only)
  IF Proj.FileManager = "" THEN
    FOR c = 0 TO 2
      BdSys(c).Enabled = 0
    NEXT
    BdSys(3).Enabled = 0
    BdAllDesktop.Visible = 0: BdEverything.Visible = 0
  END IF
  BdSys(0).AccessibleDescription = BdArchWords(0)
  BdSys(2).AccessibleDescription = BdArchWords(2)
  BdFilling = 0
END SUB

' (Windows' or Linux's processors chosen, in words, for a screen reader)
FUNCTION BdArchWords(Card AS INTEGER) AS STRING
  DIM i AS INTEGER
  Result = ""
  FOR i = 0 TO BD_APPS - 1
    IF BdCardOf(i) = Card AND BdOn(i) THEN
      IF Result <> "" THEN Result = Result + ", "
      SELECT CASE i
        CASE 0, 4: Result = Result + TR$("64-bit (x64)")
        CASE 1, 5: Result = Result + "ARM64"
        CASE 2: Result = Result + TR$("32-bit (x86)")
      END SELECT
    END IF
  NEXT
  IF Result = "" THEN Result = TR$("not chosen") ELSE Result = TR$("{list} chosen", "list", Result)
END FUNCTION

' Step 2's page; its height.
FUNCTION BdLayP2(Pad AS INTEGER, W AS INTEGER) AS INTEGER
  DIM y AS INTEGER, x AS INTEGER, lw AS INTEGER, fw AS INTEGER, fx AS INTEGER, row AS INTEGER, fh AS INTEGER, iw AS INTEGER, ix AS INTEGER, iy AS INTEGER, k AS INTEGER, sx AS INTEGER, labelsAbove AS INTEGER, t AS INTEGER
  fh = UiCompactH
  IF ThemeName = "classic" THEN row = 30 ELSE row = 32
  x = Pad: y = Pad
  BdH2.Caption = TR$("Your app")
  BdH2.Top = y
  BdNumPlace 1, BdH2, x, y
  y = y + 26
  BdS2.Left = x: BdS2.Top = y: BdS2.Width = W: BdS2.Height = BdTextHigh(BdS2, W)
  y = y + BdS2.Height + 16
  ' (the icon at the right; under the fields when narrow)
  IF BdNarrow THEN iw = W ELSE iw = 212
  lw = 128
  labelsAbove = BdNarrow
  IF BdNarrow THEN
    fx = x: fw = W
  ELSE
    fx = x + lw: fw = W - iw - 20 - lw
  END IF
  iy = y
  t = BdFieldTop(BdNameLabel, x, y, fh, labelsAbove): BdName.Left = fx: BdName.Top = t: BdName.Width = fw: BdName.Height = fh: y = t + row
  t = BdFieldTop(BdVersionLabel, x, y, fh, labelsAbove): BdVersion.Left = fx: BdVersion.Top = t: BdVersion.Width = 96: BdVersion.Height = fh: y = t + row
  t = BdFieldTop(BdAuthorLabel, x, y, fh, labelsAbove): BdAuthor.Left = fx: BdAuthor.Top = t: BdAuthor.Width = fw: BdAuthor.Height = fh: y = t + row
  t = BdFieldTop(BdCopyrightLabel, x, y, fh, labelsAbove): BdCopyright.Left = fx: BdCopyright.Top = t: BdCopyright.Width = fw: BdCopyright.Height = fh: y = t + row
  t = BdFieldTop(BdDescLabel, x, y, fh, labelsAbove): BdDesc.Left = fx: BdDesc.Top = t: BdDesc.Width = fw: BdDesc.Height = fh: y = t + row
  t = BdFieldTop(BdSiteLabel, x, y, fh, labelsAbove): BdSite.Left = fx: BdSite.Top = t: BdSite.Width = fw: BdSite.Height = fh: y = t + row
  t = BdFieldTop(BdIdLabel, x, y, fh, labelsAbove): BdId.Left = fx: BdId.Top = t: BdId.Width = fw: BdId.Height = fh: y = t + fh + 4
  BdIdNote.Left = fx: BdIdNote.Top = y: BdIdNote.Width = fw: BdIdNote.Height = BdTextHigh(BdIdNote, fw)
  y = y + BdIdNote.Height + 10
  t = BdFieldTop(BdCatLabel, x, y, fh, labelsAbove): BdCat.Left = fx: BdCat.Top = t: BdCat.Width = 180: BdCat.Height = fh: y = t + row
  BdKindLabel.Left = x: BdKindLabel.Top = y + 3
  IF labelsAbove THEN y = y + 22: BdKindLabel.Top = y - 20
  BdKindWin.Left = fx: BdKindWin.Top = y: BdKindWin.Height = 20: BdKindWin.Width = UITextWidth(BdKindWin.Caption) + 28
  BdKindCon.Left = fx + BdKindWin.Width + 16: BdKindCon.Top = y: BdKindCon.Height = 20: BdKindCon.Width = UITextWidth(BdKindCon.Caption) + 28
  y = y + 24
  BdKindNote.Left = fx: BdKindNote.Top = y: BdKindNote.Width = fw: BdKindNote.Height = BdTextHigh(BdKindNote, fw)
  y = y + BdKindNote.Height + 12
  ' (Advanced: the web app's colour, the README's language)
  y = y + BdAdvPlace(BdAdv2Icon, BdAdv2Head, BdAdv2Sum, BdAdv2, TR$("The web app's colour, the README's language"), x, y, fx + fw - x)
  BdColorLabel.Visible = BdAdv2: BdColor.Visible = BdAdv2: BdLangLabel.Visible = BdAdv2: BdLang.Visible = BdAdv2
  IF BdAdv2 THEN
    t = BdFieldTop(BdColorLabel, x, y, fh, labelsAbove): BdColor.Left = fx: BdColor.Top = t: BdColor.Width = 120: BdColor.Height = fh: y = t + row
    t = BdFieldTop(BdLangLabel, x, y, fh, labelsAbove): BdLang.Left = fx: BdLang.Top = t: BdLang.Width = 180: BdLang.Height = fh: y = t + row
  END IF
  ' (the icon's card)
  IF BdNarrow THEN
    ix = x: iy = y + 4
  ELSE
    ix = x + W - iw
  END IF
  BdIconFrame.Left = ix: BdIconFrame.Top = iy: BdIconFrame.Width = iw
  BdIconHead.Left = ix + 12: BdIconHead.Top = iy + 12
  BdIconBig.Left = ix + (iw - 128) \ 2: BdIconBig.Top = iy + 40: BdIconBig.Width = 128: BdIconBig.Height = 128
  sx = ix + (iw - (16 + 24 + 32 + 48 + 3 * 14)) \ 2
  k = iy + 180
  BdIcon16.Left = sx: BdIcon16.Top = k + 48 - 16: BdIcon16.Width = 16: BdIcon16.Height = 16
  BdSz16.Left = sx - 8: BdSz16.Top = k + 52: BdSz16.Width = 32
  sx = sx + 16 + 14
  BdIcon24.Left = sx: BdIcon24.Top = k + 48 - 24: BdIcon24.Width = 24: BdIcon24.Height = 24
  BdSz24.Left = sx - 4: BdSz24.Top = k + 52: BdSz24.Width = 32
  sx = sx + 24 + 14
  BdIcon32.Left = sx: BdIcon32.Top = k + 48 - 32: BdIcon32.Width = 32: BdIcon32.Height = 32
  BdSz32.Left = sx: BdSz32.Top = k + 52: BdSz32.Width = 32
  sx = sx + 32 + 14
  BdIcon48.Left = sx: BdIcon48.Top = k: BdIcon48.Width = 48: BdIcon48.Height = 48
  BdSz48.Left = sx + 8: BdSz48.Top = k + 52: BdSz48.Width = 32
  k = k + 76
  BdIconFile.Left = ix + 12: BdIconFile.Top = k: BdIconFile.Width = iw - 24: BdIconFile.Height = BdTextHigh(BdIconFile, iw - 24)
  k = k + BdIconFile.Height + 4
  BdIconWarn.Left = ix + 12: BdIconWarn.Top = k: BdIconWarn.Width = iw - 24: BdIconWarn.Height = BdTextHigh(BdIconWarn, iw - 24)
  k = k + BdIconWarn.Height + 6
  BdIconBtn.Left = ix + 12: BdIconBtn.Top = k: BdIconBtn.Height = UiCompactH
  BdIconBtn.Width = UITextWidth(StripAmp(BdIconBtn.Caption)) + 24
  k = k + UiCompactH + 10
  BdFit BdIconReset
  BdIconReset.Left = ix + 12: BdIconReset.Top = k
  IF BdIconReset.Visible THEN k = k + 24
  BdIconNote.Left = ix + 12: BdIconNote.Top = k: BdIconNote.Width = iw - 24: BdIconNote.Height = BdTextHigh(BdIconNote, iw - 24)
  k = k + BdIconNote.Height + 12
  BdIconFrame.Height = k - iy
  BdIconFrame.Repaint
  IF BdNarrow THEN y = k + 12
  IF k + 12 > y THEN y = k + 12
  Result = y + Pad
END FUNCTION

' An Advanced row: the chevron (right closed, down open), "Advanced" in
' bold — the control, with a mnemonic —, what is in it dimmed beside it.
FUNCTION BdAdvPlace(Ic AS RImage, Head AS RLabel, Sum AS RLabel, IsOpen AS INTEGER, Words AS STRING, X AS INTEGER, Y AS INTEGER, W AS INTEGER) AS INTEGER
  IF IsOpen THEN BdBmp(25 + Head.Tag).LoadIcon "chevron-down", 16 ELSE BdBmp(25 + Head.Tag).LoadIcon "chevron-right", 16
  Ic.BMPHandle = BdBmp(25 + Head.Tag)
  Ic.Left = X: Ic.Top = Y + 2
  Head.Caption = TR$("Advanc&ed")
  Head.Font = CardFont: Head.Font.Color = TC("page.text")
  BdFit Head
  Head.Left = X + 22: Head.Top = Y
  Head.AccessibleName = TR$("Advanced")
  Head.AccessibleDescription = Words
  Sum.Caption = Words
  Sum.Font.Color = TC("page.dim")
  Sum.AutoSize = 0: Sum.WordWrap = 1
  Sum.Left = Head.Left + Head.Width + 8: Sum.Top = Y + 1
  Sum.Width = X + W - Sum.Left
  Sum.Height = BdTextHigh(Sum, Sum.Width)
  IF Sum.Height < 20 THEN Sum.Height = 20
  Result = Sum.Height + 8
  IF Result < 28 THEN Result = 28
END FUNCTION

' (a label as wide as its words in its own font: AutoSize measures with the
' font it had when it was made)
SUB BdFit(L AS RLabel)
  L.AutoSize = 0
  L.Width = BdTextWide(L) + 4
  L.Height = 20
END SUB

FUNCTION BdTextWide(L AS RLabel) AS INTEGER
  BdMeasure.Font.Name = L.Font.Name: BdMeasure.Font.Size = L.Font.Size: BdMeasure.Font.Bold = L.Font.Bold
  Result = BdMeasure.TextWidth(REPLACESUBSTR$(L.Caption, "&", ""))
END FUNCTION

' One label beside its field (above it when narrow): the field's top.
FUNCTION BdFieldTop(L AS RLabel, X AS INTEGER, Y AS INTEGER, FH AS INTEGER, Above AS INTEGER) AS INTEGER
  IF Above THEN
    L.Left = X: L.Top = Y
    Result = Y + 20
  ELSE
    L.Left = X: L.Top = Y + (FH - 16) \ 2
    Result = Y
  END IF
END FUNCTION

' Step 3's page; its height.
FUNCTION BdLayP3(Pad AS INTEGER, W AS INTEGER) AS INTEGER
  DIM y AS INTEGER, x AS INTEGER, i AS INTEGER, rowH AS INTEGER, fy AS INTEGER, n AS INTEGER, nameW AS INTEGER, total AS DOUBLE, words AS STRING
  x = Pad: y = Pad
  rowH = 28
  BdH3.Caption = TR$("What goes in")
  BdH3.Top = y
  BdNumPlace 2, BdH3, x, y
  y = y + 26
  BdS3.Left = x: BdS3.Top = y: BdS3.Width = W: BdS3.Height = BdTextHigh(BdS3, W)
  y = y + BdS3.Height + 14
  BdProgIcon.Left = x: BdProgIcon.Top = y + 1
  BdProgName.Caption = FileTitle(ProjectMain)
  BdProgName.Left = x + 24: BdProgName.Top = y
  BdProgNote.Left = BdProgName.Left + BdProgName.Width + 10: BdProgNote.Top = y + 1: BdProgNote.Width = x + W - BdProgNote.Left: BdProgNote.Height = 18
  y = y + 32
  ' (the data files: ticked ones go in)
  BdDataHead.Caption = TR$("Data files") + "  " + LTRIM$(STR$(BdFileCount))
  BdDataHead.Left = x: BdDataHead.Top = y + 4
  BdAddBtn.Height = UiCompactH: BdAddBtn.Width = UITextWidth(StripAmp(BdAddBtn.Caption)) + 24
  BdAddBtn.Left = x + W - BdAddBtn.Width: BdAddBtn.Top = y
  BdAddBtn.Visible = Flag(Proj.FileManager <> "")
  y = y + UiCompactH + 8
  fy = y
  FOR i = 0 TO BD_FILES - 1
    IF i < BdFileCount THEN
      BdFileChk(i).Visible = 1: BdFileIcon(i).Visible = 1: BdFileKind(i).Visible = 1: BdFileSize(i).Visible = 1: BdFileName(i).Visible = 1
      BdFileChk(i).Caption = ""
      BdFileChk(i).AccessibleName = BdFilePath(i)
      BdFileChk(i).AccessibleDescription = BdFileWord(BdFilePath(i)) + ", " + BdSize(BdFileBytes(i))
      BdFileChk(i).Left = x + 12: BdFileChk(i).Top = y + 8 + i * rowH: BdFileChk(i).Height = 20: BdFileChk(i).Width = 20
      BdFileChk(i).Color = TC("window")
      BdFileIcon(i).Left = x + 40: BdFileIcon(i).Top = y + 10 + i * rowH
      BdFileName(i).Caption = BdFilePath(i)
      BdFileName(i).Font.Color = TC("page.text")
      BdFit BdFileName(i)
      BdFileName(i).Left = x + 62: BdFileName(i).Top = y + 9 + i * rowH
      BdFileBmp(i).LoadIcon BdFileIconName(BdFilePath(i)), 16
      BdFileIcon(i).BMPHandle = BdFileBmp(i)
      BdFileKind(i).Caption = BdFileWord(BdFilePath(i))
      BdFileKind(i).Left = BdFileName(i).Left + BdFileName(i).Width + 8: BdFileKind(i).Top = y + 9 + i * rowH
      BdFileKind(i).Font.Color = TC("page.dim")
      BdFileSize(i).Caption = BdSize(BdFileBytes(i))
      BdFileSize(i).Left = x + W - 112: BdFileSize(i).Width = 100: BdFileSize(i).Top = y + 10 + i * rowH: BdFileSize(i).Height = 18
      BdFileSize(i).Font.Color = TC("page.dim")
    ELSE
      BdFileChk(i).Visible = 0: BdFileIcon(i).Visible = 0: BdFileKind(i).Visible = 0: BdFileSize(i).Visible = 0: BdFileName(i).Visible = 0
    END IF
  NEXT
  n = BdFileCount
  IF n > BD_FILES THEN n = BD_FILES
  IF n = 0 THEN
    BdNoFiles.Visible = 1
    BdNoFiles.Left = x + 12: BdNoFiles.Top = y + 10: BdNoFiles.Width = W - 24: BdNoFiles.Height = 20
    BdFilesFrame.Left = x: BdFilesFrame.Top = y: BdFilesFrame.Width = W: BdFilesFrame.Height = 40
    y = y + 40 + 20
  ELSE
    BdNoFiles.Visible = 0
    BdFilesFrame.Left = x: BdFilesFrame.Top = y: BdFilesFrame.Width = W: BdFilesFrame.Height = n * rowH + 12
    y = y + n * rowH + 12 + 20
  END IF
  BdFilesFrame.Repaint
  ' (left out, with the reason)
  BdOutHead.Caption = TR$("Left out") + "  " + LTRIM$(STR$(BdOutCount))
  BdOutHead.Visible = Flag(BdOutCount > 0): BdOutFrame.Visible = Flag(BdOutCount > 0)
  FOR i = 0 TO BD_OUTS - 1
    IF i < BdOutCount THEN
      BdOutIcon(i).Visible = 1: BdOutLabel(i).Visible = 1: BdOutText(i).Visible = 1
    ELSE
      BdOutIcon(i).Visible = 0: BdOutLabel(i).Visible = 0: BdOutText(i).Visible = 0
    END IF
  NEXT
  IF BdOutCount > 0 THEN
    BdOutHead.Left = x: BdOutHead.Top = y
    y = y + 26
    FOR i = 0 TO BdOutCount - 1
      IF BdOutWhy(i) = "unread" THEN BdOutBmp(i).LoadIcon "info", 16 ELSE BdOutBmp(i).LoadIcon "warning", 16
      BdOutIcon(i).BMPHandle = BdOutBmp(i)
      BdOutIcon(i).Left = x + 12: BdOutIcon(i).Top = y + 10 + i * rowH
      BdOutLabel(i).Caption = BdOutName(i)
      BdOutLabel(i).Left = x + 36: BdOutLabel(i).Top = y + 9 + i * rowH
      BdOutLabel(i).Font.Color = TC("page.text")
      BdOutText(i).Caption = BdOutWords(BdOutWhy(i))
      BdOutText(i).Left = BdOutLabel(i).Left + BdOutLabel(i).Width + 10: BdOutText(i).Top = y + 9 + i * rowH
      BdOutText(i).Width = x + W - 12 - BdOutText(i).Left: BdOutText(i).Height = 18
      BdOutText(i).Font.Color = TC("page.dim")
      BdOutText(i).AccessibleName = BdOutName(i) + ": " + BdOutText(i).Caption
    NEXT
    BdOutFrame.Left = x: BdOutFrame.Top = y: BdOutFrame.Width = W: BdOutFrame.Height = BdOutCount * rowH + 12
    BdOutFrame.Repaint
    y = y + BdOutCount * rowH + 12 + 20
  END IF
  ' (each app's size, when a build made it before)
  BdSizeIcon.Left = x: BdSizeIcon.Top = y + 2
  total = 0
  FOR i = 0 TO n - 1
    IF BdFileChk(i).Checked THEN total = total + BdFileBytes(i)
  NEXT
  words = BdKnownSizes
  IF words = "" THEN
    BdSizeNote.Caption = TR$("Most of each app is the RapidR runner for its system; your program's files add {size}.", "size", BdSize(total))
  ELSE
    BdSizeNote.Caption = TR$("Each app: {list}. Most of it is the RapidR runner for that system; your program's files add {size}.", "list", words, "size", BdSize(total))
  END IF
  BdSizeNote.Left = x + 24: BdSizeNote.Top = y: BdSizeNote.Width = W - 24: BdSizeNote.Height = BdTextHigh(BdSizeNote, W - 24)
  y = y + BdSizeNote.Height + 18
  ' (Advanced: the source, native code)
  y = y + BdAdvPlace(BdAdv3Icon, BdAdv3Head, BdAdv3Sum, BdAdv3, TR$("Include the source code, native code"), x, y, W)
  BdSource.Visible = BdAdv3
  BdNative.Visible = Flag(BdAdv3 AND Proj.FileManager <> "")
  BdNativeNote.Visible = BdNative.Visible
  IF BdAdv3 THEN
    BdSource.Left = x + 24: BdSource.Top = y: BdSource.Height = 20: BdSource.Width = UITextWidth(StripAmp(BdSource.Caption)) + 32
    y = y + 26
    IF BdNative.Visible THEN
      BdNative.Left = x + 24: BdNative.Top = y: BdNative.Height = 20: BdNative.Width = UITextWidth(StripAmp(BdNative.Caption)) + 32
      y = y + 22
      BdNativeNote.Caption = TR$("This computer's app is compiled with Rust: the fastest, this system only. Needs Rust (free); the other systems' apps run the bytecode as always.")
      BdNativeNote.Left = x + 52: BdNativeNote.Top = y: BdNativeNote.Width = W - 52: BdNativeNote.Height = BdTextHigh(BdNativeNote, W - 52)
      y = y + BdNativeNote.Height + 8
    END IF
  END IF
  Result = y + Pad
END FUNCTION

' (each ticked app's size as a build made it before: "Windows 64-bit 21 MB,
' Web 6 MB"; "" when none was built)
FUNCTION BdKnownSizes AS STRING
  DIM t AS STRING, n AS INTEGER, i AS INTEGER, k AS INTEGER, ln AS STRING
  t = Proj.BuiltSizes()
  Result = ""
  IF t = "" THEN EXIT FUNCTION
  n = TALLY(t, CHR$(10)) + 1
  FOR i = 1 TO n
    ln = FIELD$(t, CHR$(10), i)
    k = BdIndex(FIELD$(ln, CHR$(9), 1))
    IF k >= 0 THEN
      IF BdOn(k) THEN
        IF Result <> "" THEN Result = Result + ", "
        Result = Result + BdLabel(k) + " " + BdSize(VAL(FIELD$(ln, CHR$(9), 2)))
      END IF
    END IF
  NEXT
END FUNCTION

' (the size of an app as a build made it before; 0 unknown)
FUNCTION BdKnownSize(i AS INTEGER) AS DOUBLE
  DIM t AS STRING, n AS INTEGER, k AS INTEGER, ln AS STRING
  Result = 0
  t = Proj.BuiltSizes()
  IF t = "" THEN EXIT FUNCTION
  n = TALLY(t, CHR$(10)) + 1
  FOR k = 1 TO n
    ln = FIELD$(t, CHR$(10), k)
    IF FIELD$(ln, CHR$(9), 1) = BdTarget(i) THEN Result = VAL(FIELD$(ln, CHR$(9), 2))
  NEXT
END FUNCTION

FUNCTION BdTotalBytes AS DOUBLE
  DIM i AS INTEGER, s AS DOUBLE
  Result = 0
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) THEN
      s = BdKnownSize(i)
      IF s = 0 THEN
        Result = 0
        EXIT FUNCTION
      END IF
      Result = Result + s
    END IF
  NEXT
END FUNCTION

' Step 4's page; its height.
FUNCTION BdLayP4(Pad AS INTEGER, W AS INTEGER) AS INTEGER
  DIM y AS INTEGER, x AS INTEGER, i AS INTEGER, rowH AS INTEGER, r AS INTEGER, fh AS INTEGER, s AS DOUBLE, need AS STRING, needN AS INTEGER, lx AS INTEGER
  x = Pad: y = Pad: fh = UiCompactH
  rowH = 32
  BdH4.Caption = TR$("Ready to build")
  BdH4.Top = y
  BdNumPlace 3, BdH4, x, y
  y = y + 26
  IF BdPackOne.Checked THEN
    BdS4.Caption = TRN$("{n} app", "{n} apps", BdApps) + " " + TRN$("for {n} system, all in one zip.", "for {n} systems, all in one zip.", BdSystems)
  ELSE
    BdS4.Caption = TRN$("{n} app", "{n} apps", BdApps) + " " + TRN$("for {n} system, each in its own zip.", "for {n} systems, each in its own zip.", BdSystems)
  END IF
  BdS4.Left = x: BdS4.Top = y: BdS4.Width = W: BdS4.Height = BdTextHigh(BdS4, W)
  y = y + BdS4.Height + 14
  ' (one row per app: its name, its zip, its size when known)
  r = 0
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) THEN
      BdReadyIcon(i).Visible = 1: BdReadyName(i).Visible = 1: BdReadyZip(i).Visible = 1: BdReadySize(i).Visible = 1
      BdAppBmp(i).LoadIcon BdIconName(BdCardOf(i)), 16
      BdReadyIcon(i).BMPHandle = BdAppBmp(i)
      BdReadyIcon(i).Left = x + 12: BdReadyIcon(i).Top = y + 12 + r * rowH
      BdReadyName(i).Caption = BdLabel(i)
      BdReadyName(i).Left = x + 36: BdReadyName(i).Top = y + 11 + r * rowH
      BdReadyName(i).Font.Color = TC("page.text")
      BdReadyZip(i).Caption = Proj.ZipName(BdTarget(i))
      BdReadyZip(i).Left = BdReadyName(i).Left + BdReadyName(i).Width + 10: BdReadyZip(i).Top = y + 11 + r * rowH
      BdReadyZip(i).Width = x + W - 80 - BdReadyZip(i).Left: BdReadyZip(i).Height = 18
      BdReadyZip(i).Font.Color = TC("page.dim")
      BdReadyZip(i).Visible = Flag(BdNarrow = 0)
      s = BdKnownSize(i)
      IF s > 0 THEN BdReadySize(i).Caption = BdSize(s) ELSE BdReadySize(i).Caption = ""
      BdReadySize(i).Left = x + W - 92: BdReadySize(i).Width = 80: BdReadySize(i).Top = y + 11 + r * rowH: BdReadySize(i).Height = 18
      BdReadySize(i).Font.Color = TC("page.dim")
      BdReadyName(i).AccessibleName = BdLabel(i) + ": " + BdReadyZip(i).Caption
      r = r + 1
    ELSE
      BdReadyIcon(i).Visible = 0: BdReadyName(i).Visible = 0: BdReadyZip(i).Visible = 0: BdReadySize(i).Visible = 0
    END IF
  NEXT
  BdReadyFrame.Left = x: BdReadyFrame.Top = y: BdReadyFrame.Width = W: BdReadyFrame.Height = r * rowH + 16
  BdReadyFrame.Visible = Flag(r > 0)
  BdReadyFrame.Repaint
  y = y + r * rowH + 16 + 18
  ' (where they go — the browser saves them on the web — and how)
  BdFolderLabel.Visible = Flag(Proj.FileManager <> ""): BdFolder.Visible = BdFolderLabel.Visible: BdFolderBtn.Visible = BdFolderLabel.Visible
  IF BdFolderLabel.Visible THEN
    lx = x + UITextWidth(StripAmp(BdFolderLabel.Caption)) + 12
    IF lx < x + 80 THEN lx = x + 80
    BdFolderLabel.Left = x: BdFolderLabel.Top = y + (fh - 16) \ 2
    BdFolderBtn.Height = fh: BdFolderBtn.Width = UITextWidth(StripAmp(BdFolderBtn.Caption)) + 24
    BdFolderBtn.Left = x + W - BdFolderBtn.Width: BdFolderBtn.Top = y
    BdFolder.Left = lx: BdFolder.Top = y: BdFolder.Height = fh: BdFolder.Width = BdFolderBtn.Left - 8 - lx
    y = y + fh + 12
  END IF
  lx = x + UITextWidth(StripAmp(BdPackLabel.Caption)) + 12
  IF lx < x + 80 THEN lx = x + 80
  BdPackLabel.Visible = Flag(Proj.FileManager <> ""): BdPackEach.Visible = BdPackLabel.Visible: BdPackOne.Visible = BdPackLabel.Visible
  BdPackLabel.Left = x: BdPackLabel.Top = y + 2
  BdPackEach.Left = lx: BdPackEach.Top = y: BdPackEach.Height = 20: BdPackEach.Width = UITextWidth(BdPackEach.Caption) + 28
  BdPackOne.Left = lx + BdPackEach.Width + 16: BdPackOne.Top = y: BdPackOne.Height = 20: BdPackOne.Width = UITextWidth(BdPackOne.Caption) + 28
  IF BdNarrow THEN
    BdPackOne.Left = lx: BdPackOne.Top = y + 24
    y = y + 24
  END IF
  y = y + 34
  ' (the runners that download first: those the install doesn't have)
  need = Proj.RunnersToFetch(BdTicked)
  IF need = "" THEN needN = 0 ELSE needN = TALLY(need, ",") + 1
  BdDlFrame.Visible = Flag(needN > 0): BdDlIcon.Visible = BdDlFrame.Visible: BdDlHead.Visible = BdDlFrame.Visible: BdDlText.Visible = BdDlFrame.Visible
  IF needN > 0 THEN
    BdDlHead.Caption = TRN$("{n} runner downloads first, once", "{n} runners download first, once", needN)
    IF BdRunnerBytes(need) > 0 THEN
      BdDlText.Caption = TR$("{list}, {size} from {host}. Each is checked against RapidR's signature before it's used.", "list", BdRunnerWords(need), "size", BdSize(BdRunnerBytes(need)), "host", "rapidr.dev")
    ELSE
      BdDlText.Caption = TR$("{list}, from {host}. Each is checked against RapidR's signature before it's used.", "list", BdRunnerWords(need), "host", "rapidr.dev")
    END IF
    BdDlFrame.Left = x: BdDlFrame.Top = y: BdDlFrame.Width = W
    BdDlIcon.Left = x + 14: BdDlIcon.Top = y + 15
    BdDlHead.Left = x + 40: BdDlHead.Top = y + 14
    BdDlText.Left = x + 40: BdDlText.Top = y + 36: BdDlText.Width = W - 56: BdDlText.Height = BdTextHigh(BdDlText, W - 56)
    BdDlFrame.Height = BdDlText.Height + 50
    BdDlFrame.Repaint
    y = y + BdDlFrame.Height + 16
  END IF
  BdReadmeIcon.Left = x: BdReadmeIcon.Top = y + 2
  BdReadmeNote.Left = x + 24: BdReadmeNote.Top = y: BdReadmeNote.Width = W - 24: BdReadmeNote.Height = BdTextHigh(BdReadmeNote, W - 24)
  y = y + BdReadmeNote.Height + Pad
  Result = y
END FUNCTION

' (the systems whose runners download, in words: "Windows 32-bit and macOS")
FUNCTION BdRunnerWords(List AS STRING) AS STRING
  DIM n AS INTEGER, i AS INTEGER, k AS INTEGER
  n = TALLY(List, ",") + 1
  FOR i = 1 TO n
    k = BdIndex(FIELD$(FIELD$(List, ",", i), ":", 1))
    IF k >= 0 THEN
      IF i > 1 AND i = n THEN
        Result = Result + " " + TR$("and") + " "
      ELSEIF i > 1 THEN
        Result = Result + ", "
      END IF
      Result = Result + BdLabel(k)
    END IF
  NEXT
END FUNCTION

' (the runners' sizes, when the list of runners says them; 0 unknown)
FUNCTION BdRunnerBytes(List AS STRING) AS DOUBLE
  DIM n AS INTEGER, i AS INTEGER, part AS STRING
  Result = 0
  n = TALLY(List, ",") + 1
  FOR i = 1 TO n
    part = FIELD$(List, ",", i)
    IF INSTR(part, ":") = 0 THEN
      Result = 0
      EXIT FUNCTION
    END IF
    Result = Result + VAL(FIELD$(part, ":", 2))
  NEXT
END FUNCTION

' The build's page — building, then done; its height.
FUNCTION BdLayP5(Pad AS INTEGER, W AS INTEGER) AS INTEGER
  DIM y AS INTEGER, x AS INTEGER, i AS INTEGER, r AS INTEGER, rowH AS INTEGER, ry AS INTEGER, c AS INTEGER, on AS INTEGER, k AS INTEGER, built AS INTEGER, fx AS INTEGER, nameW AS INTEGER
  x = Pad: y = Pad
  built = BdBuiltCount
  BdH5Icon.Visible = Flag(BdPhase = "done")
  IF BdPhase = "done" THEN
    IF built = BdApps THEN
      BdBmp(6).LoadIcon "success", 20
      BdH5.Caption = TRN$("{n} app built in {s} s", "{n} apps built in {s} s", built, "s", BdOne(BdSeconds))
    ELSE
      BdBmp(6).LoadIcon "warning", 20
      BdH5.Caption = TR$("{n} of {m} apps built", "n", LTRIM$(STR$(built)), "m", LTRIM$(STR$(BdApps)))
    END IF
    BdH5Icon.BMPHandle = BdBmp(6)
    BdH5Icon.Left = x: BdH5Icon.Top = y + 2
    BdH5.Left = x + 30
    IF Proj.FileManager = "" THEN
      BdS5.Caption = TR$("Your browser saved them. Nothing left your computer: the apps were packed in this page.")
    ELSE
      BdS5.Caption = TR$("In {folder}, with SHA256SUMS beside them.", "folder", FitMiddle(BdShortPath(BdOutDir), W - 320))
    END IF
  ELSE
    BdH5.Left = x
    BdH5.Caption = TRN$("Building {n} app", "Building {n} apps", BdApps)
    BdS5.Caption = TR$("{file} is compiled once. Now each system's app is packed.", "file", FileTitle(ProjectMain))
  END IF
  BdH5.Top = y
  y = y + 26
  BdS5.Left = x: BdS5.Top = y: BdS5.Width = W: BdS5.Height = BdTextHigh(BdS5, W)
  y = y + BdS5.Height + 16
  ' (the whole build's bar while it builds)
  BdAllBar.Visible = Flag(BdPhase = "building"): BdAllNote.Visible = BdAllBar.Visible
  IF BdPhase = "building" THEN
    BdAllBar.Left = x: BdAllBar.Top = y + 5: BdAllBar.Width = W * 6 \ 10: BdAllBar.Height = 8
    BdAllBar.Repaint
    BdAllNote.Left = x + BdAllBar.Width + 16: BdAllNote.Top = y: BdAllNote.Width = W - BdAllBar.Width - 16: BdAllNote.Height = 18
    BdAllNote.Caption = TR$("{n} of {m} done, {s} s", "n", LTRIM$(STR$(BdDoneCount)), "m", LTRIM$(STR$(BdApps)), "s", BdOne(TIMER - BdStarted))
    y = y + 30
  END IF
  ' (a row per app)
  IF BdPhase = "done" THEN rowH = 34 ELSE rowH = 56
  r = 0
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) THEN
      ry = y + r * rowH
      BdRunIcon(i).Visible = 1: BdRunName(i).Visible = 1: BdRunStage(i).Visible = 1: BdRunSize(i).Visible = 1: BdRunMark(i).Visible = 1
      BdRunLine(i).Visible = Flag(r > 0)
      BdRunLine(i).Left = x + 1: BdRunLine(i).Top = ry: BdRunLine(i).Width = W - 2: BdRunLine(i).Height = 1
      BdAppBmp(i).LoadIcon BdIconName(BdCardOf(i)), 16
      BdRunIcon(i).BMPHandle = BdAppBmp(i)
      BdRunIcon(i).Left = x + 12: BdRunIcon(i).Top = ry + 9
      BdRunName(i).Caption = BdLabel(i)
      BdRunName(i).Font = CardFont: BdRunName(i).Font.Color = TC("page.text")
      BdRunName(i).Left = x + 36: BdRunName(i).Top = ry + 8
      ' (its stage, in words, with the marks done before it)
      BdRunStage(i).Caption = BdStageWords(i)
      IF BdState(i) = "failed" THEN BdRunStage(i).Font.Color = TC("status.danger.text") ELSE BdRunStage(i).Font.Color = TC("page.dim")
      IF BdPhase = "done" THEN
        ' (done: the zip's name after the system's)
        BdRunStage(i).Left = BdRunName(i).Left + BdRunName(i).Width + 10: BdRunStage(i).Top = ry + 9
        BdRunStage(i).Width = x + W - 110 - BdRunStage(i).Left: BdRunStage(i).Height = 18
      ELSE
        BdRunStage(i).Left = x + 36: BdRunStage(i).Top = ry + 30
        BdRunStage(i).Width = W - 36 - 180: BdRunStage(i).Height = 18
      END IF
      IF BdBytes(i) > 0 THEN
        BdRunSize(i).Caption = BdSize(BdBytes(i))
      ELSEIF BdState(i) = "built" THEN
        BdRunSize(i).Caption = "—"
      ELSE
        BdRunSize(i).Caption = ""
      END IF
      BdRunSize(i).Left = x + W - 120: BdRunSize(i).Width = 80: BdRunSize(i).Top = ry + 8: BdRunSize(i).Height = 18
      BdRunSize(i).Font.Color = TC("page.dim")
      BdRunMark(i).Left = x + W - 30: BdRunMark(i).Top = ry + 9
      SELECT CASE BdState(i)
        CASE "built": BdMarkBmp(i).LoadIcon "success", 16
        CASE "failed": BdMarkBmp(i).LoadIcon "error", 16
        CASE ELSE: BdMarkBmp(i).LoadIcon "glyphs/dot", 16, "disabled"
      END SELECT
      BdRunMark(i).BMPHandle = BdMarkBmp(i)
      BdRunMark(i).Visible = Flag(BdState(i) = "built" OR BdState(i) = "failed")
      ' (its own bar while it works)
      BdRunBar(i).Visible = Flag(BdPhase = "building" AND (BdState(i) = "working" OR BdState(i) = "download"))
      BdRunPct(i).Visible = 0
      IF BdRunBar(i).Visible THEN
        BdRunBar(i).Left = x + W - 172: BdRunBar(i).Top = ry + 35: BdRunBar(i).Width = 120: BdRunBar(i).Height = 6
        BdRunBar(i).Repaint
      END IF
      ' (a failed one says what to do: Try Again)
      BdRunAct(i).Visible = Flag(BdState(i) = "failed" AND BdPhase = "done")
      IF BdRunAct(i).Visible THEN
        BdRunAct(i).Caption = TR$("Try Again")
        BdRunAct(i).Height = UiCompactH: BdRunAct(i).Width = UITextWidth(BdRunAct(i).Caption) + 24
        BdRunAct(i).Left = x + W - 40 - BdRunAct(i).Width: BdRunAct(i).Top = ry + 5
        BdRunStage(i).Width = BdRunAct(i).Left - 8 - BdRunStage(i).Left
        BdRunSize(i).Visible = 0
      END IF
      BdRunName(i).AccessibleName = BdLabel(i) + ": " + BdRunStage(i).Caption
      r = r + 1
    ELSE
      BdRunIcon(i).Visible = 0: BdRunName(i).Visible = 0: BdRunStage(i).Visible = 0: BdRunSize(i).Visible = 0: BdRunMark(i).Visible = 0
      BdRunBar(i).Visible = 0: BdRunPct(i).Visible = 0: BdRunAct(i).Visible = 0: BdRunLine(i).Visible = 0
    END IF
  NEXT
  BdRunFrame.Left = x: BdRunFrame.Top = y: BdRunFrame.Width = W: BdRunFrame.Height = r * rowH + 2
  BdRunFrame.Repaint
  y = y + r * rowH + 2 + 20
  ' (done: how to open it the first time, per system built)
  k = 0
  FOR c = 0 TO 3
    on = 0
    FOR i = 0 TO BD_APPS - 1
      IF BdOn(i) AND BdCardOf(i) = c AND BdState(i) = "built" THEN on = 1
    NEXT
    BdFirstIcon(c).Visible = Flag(on AND BdPhase = "done"): BdFirstName(c).Visible = BdFirstIcon(c).Visible: BdFirstText(c).Visible = BdFirstIcon(c).Visible
    IF BdFirstIcon(c).Visible THEN k = k + 1
  NEXT
  BdFirstFrame.Visible = Flag(k > 0): BdFirstHead.Visible = BdFirstFrame.Visible: BdFirstFoot.Visible = BdFirstFrame.Visible
  IF k > 0 THEN
    BdFirstHead.Left = x + 16: BdFirstHead.Top = y + 14
    ry = y + 44
    fx = x + 140
    IF BdNarrow THEN fx = x + 16
    FOR c = 0 TO 3
      IF BdFirstIcon(c).Visible THEN
        BdFirstBmp(c).LoadIcon BdIconName(c), 16
        BdFirstIcon(c).BMPHandle = BdFirstBmp(c)
        BdFirstIcon(c).Left = x + 16: BdFirstIcon(c).Top = ry + 1
        SELECT CASE c
          CASE 0: BdFirstName(c).Caption = "Windows:": BdFirstText(c).Caption = TR$("If Windows warns, choose More info, then Run anyway.")
          CASE 1: BdFirstName(c).Caption = "macOS:": BdFirstText(c).Caption = TR$("Open it once, then choose Open Anyway in System Settings, Privacy & Security.")
          CASE 2: BdFirstName(c).Caption = "Linux:": BdFirstText(c).Caption = TR$("Unpack it, then run install.sh, or the app in its folder.")
          CASE 3: BdFirstName(c).Caption = TR$("Web") + ":": BdFirstText(c).Caption = TR$("Put the folder on any web host, or try it here.")
        END SELECT
        BdFirstName(c).Font = CardFont: BdFirstName(c).Font.Color = TC("page.text")
        BdFirstName(c).Left = x + 40: BdFirstName(c).Top = ry
        IF BdNarrow THEN
          BdFirstText(c).Left = x + 40: BdFirstText(c).Top = ry + 22: BdFirstText(c).Width = W - 56
        ELSE
          BdFirstText(c).Left = fx: BdFirstText(c).Top = ry: BdFirstText(c).Width = x + W - 16 - fx
        END IF
        BdFirstText(c).Caption = REPLACESUBSTR$(BdFirstText(c).Caption, "&", "&&")
        BdFirstText(c).Height = BdTextHigh(BdFirstText(c), BdFirstText(c).Width)
        BdFirstText(c).Font.Color = TC("page.text")
        BdFirstText(c).AccessibleName = BdFirstName(c).Caption + " " + BdFirstText(c).Caption
        ry = BdFirstText(c).Top + BdFirstText(c).Height + 8
      END IF
    NEXT
    BdFirstFoot.Left = x + 16: BdFirstFoot.Top = ry + 2: BdFirstFoot.Width = W - 32: BdFirstFoot.Height = 20
    BdFirstFrame.Left = x: BdFirstFrame.Top = y: BdFirstFrame.Width = W: BdFirstFrame.Height = ry + 32 - y
    BdFirstFrame.Repaint
    y = y + BdFirstFrame.Height + 16
  END IF
  Result = y + Pad
END FUNCTION

' (a folder as people read it: the home folder as ~)
FUNCTION BdShortPath(P AS STRING) AS STRING
  DIM h AS STRING
  h = ENVIRON$("HOME")
  IF h <> "" AND LEFT$(P, LEN(h)) = h THEN Result = "~" + MID$(P, LEN(h) + 1) ELSE Result = P
END FUNCTION

' (an app's row in words: "Runner checked · Packed · Zipped", "Downloading
' its runner (19 MB)", the zip's name when done, why it failed)
FUNCTION BdStageWords(i AS INTEGER) AS STRING
  SELECT CASE BdState(i)
    CASE "download": Result = TR$("Downloading its runner ({size})...", "size", BdNote(i))
    CASE "working": Result = TR$("Runner checked · Packing and zipping...")
    CASE "built"
      IF BdPhase = "done" THEN Result = BdZip(i) ELSE Result = TR$("Runner checked · Packed · Zipped")
    CASE "failed": Result = BdWhy(i)
    CASE "stopped": Result = TR$("Stopped: nothing kept.")
    CASE ELSE: Result = TR$("Waiting")
  END SELECT
END FUNCTION

' (every change on the page: the ticks, the rows, the summaries)
SUB BdRefresh
  BdLayOut
  BdCard0.Repaint: BdCard1.Repaint: BdCard2.Repaint: BdCard3.Repaint
END SUB

' ---- building ------------------------------------------------------------------------------

' Build: the project's files saved and checked, then `rapidr build
' --targets …` (the desktop) or the page's packer (the web); the rows follow
' its lines. Build Again too (BdQuiet: no dialog).
SUB BdStart
  DIM i AS INTEGER, started AS INTEGER
  IF ProjectOpen = 0 OR Proj.Building THEN EXIT SUB
  IF BdRowsMade = 0 THEN BdMake: BdWords: BdLoad
  IF BdApps = 0 THEN EXIT SUB
  BdSaveNow
  IF SaveAllOk = 0 THEN EXIT SUB
  AnalyzeActive
  IF Lang.ErrorCount > 0 THEN
    OutputLine TRN$("Build: {n} error (see Problems).", "Build: {n} errors (see Problems).", Lang.ErrorCount)
    SetState "error", TR$("Can't build: the program has errors")
    IF BdShown THEN BuildDialog.Close
    OutputBox.Page = "problems"
    Dock.FocusPane("outputbox")
    EXIT SUB
  END IF
  FOR i = 0 TO BD_APPS - 1
    BdState(i) = "": BdZip(i) = "": BdBytes(i) = 0: BdWhy(i) = "": BdNote(i) = ""
  NEXT
  BdMade = ""
  BdOutDir = Proj.OutputPath()
  BdNativeNext = Flag(Proj.NativeCode AND Proj.FileManager <> "" AND Proj.NativeBuilds <> 0)
  BdNativeRun = 0
  ' (native code without Rust: asked first — build the zips without it,
  ' install Rust, or nothing: prompts.inc's question)
  IF BdNativeNext THEN
    IF Proj.RustReady() = 0 THEN
      SELECT CASE AskRust
        CASE "interpreted": BdNativeNext = 0
        CASE "install"
          IF BdShown THEN BuildDialog.Close
          InstallRust
          EXIT SUB
        CASE ELSE: EXIT SUB
      END SELECT
    END IF
  END IF
  OutputBox.Clear "build"
  BuildMode = "targets"
  BdStarted = TIMER
  BuildStarted = TIMER
  IF Proj.FileManager = "" THEN
    ' (the web Studio: the page packs the web app and the browser saves it)
    BuildMode = "webpage"
    started = Proj.Build("web")
  ELSE
    started = Proj.BuildTargets()
  END IF
  IF started = 0 THEN
    BuildLine CHR$(27) + "[31m" + TR$("Can't build: {error}", "error", Proj.Error) + CHR$(27) + "[0m"
    SetState "error", TR$("Can't build {file}", "file", FileTitle(ProjectMain))
    IF BdQuiet AND ToolsBuildTicket = "" THEN BdToastId = Toast(TR$("Can't build: {error}", "error", Proj.Error), "error", TR$("Open Build"), "build")
    EXIT SUB
  END IF
  BdPhase = "building"
  BdTickTimer.Enabled = 1
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) THEN BdState(i) = "waiting"
  NEXT
  SetState "building", TRN$("Building {n} app...", "Building {n} apps...", BdApps)
  Application.Announce TRN$("Building {n} app", "Building {n} apps", BdApps)
  IF BdShown THEN BdGo 5
END SUB

' Build Again (Ctrl+Shift+B): the saved choices, no dialog — the status bar
' says how it goes, a toast says how it ended.
SUB BuildAgain
  IF ProjectOpen = 0 OR Proj.Building THEN EXIT SUB
  BdEnsure
  BdLoad
  BdQuiet = 1
  BdStart
END SUB

' Stop (Esc while building): the build ends and nothing half-made is kept
' — the zips of the apps not finished are deleted.
SUB BdStop
  DIM i AS INTEGER, p AS STRING
  IF BdPhase <> "building" THEN EXIT SUB
  Proj.StopBuild
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) AND BdState(i) <> "built" THEN
      BdState(i) = "stopped"
      p = BdOutDir + "/" + Proj.ZipName(BdTarget(i))
      IF FILEEXISTS(p) THEN KILL p
    END IF
  NEXT
  BdPhase = "done"
  BdSeconds = TIMER - BdStarted
  IF ToolsBuildTicket <> "" THEN ToolsBuildDone 1
  SetState "stopped", TR$("The build was stopped")
  Application.Announce TR$("The build was stopped")
  IF BdShown THEN BdLayOut
END SUB

' A line `rapidr build` printed: the Output's Build page keeps it, and what
' it says about an app moves its row (RPROJECT.BuildNews).
SUB BuildOutput(Text AS STRING)
  DIM news AS STRING, kind AS STRING, i AS INTEGER, k AS INTEGER, path AS STRING, head AS STRING, nxt AS INTEGER
  ' (a build an AI asked for, build_app: its answer is what the build says)
  IF ToolsBuildTicket <> "" THEN ToolsBuildLog = ToolsBuildLog + Text + CHR$(10)
  k = INSTR(Text, ": ")
  IF k > 0 AND Proj.FileManager <> "" THEN
    head = LEFT$(Text, k - 1)
    path = MID$(Text, k + 2)
    IF head = "App" OR head = "AppDir" OR head = "Executable" OR head = "Web" OR head = "Rust source" OR head = "Output folder" OR head = "Checksums" THEN
      BuildLine head + ": " + Hyperlink(path, "reveal:" + path)
    ELSE
      BuildLine Text
    END IF
  ELSE
    BuildLine Text
  END IF
  IF BuildMode <> "targets" THEN EXIT SUB
  news = Proj.BuildNews(Text)
  IF news = "" THEN EXIT SUB
  kind = FIELD$(news, CHR$(9), 1)
  i = BdIndex(FIELD$(news, CHR$(9), 2))
  SELECT CASE kind
    CASE "start"
      ' (the first app is being packed)
      BdNextWorking
    CASE "folder"
      BdOutDir = FIELD$(news, CHR$(9), 2)
    CASE "download"
      IF i >= 0 THEN
        BdState(i) = "download": BdNote(i) = FIELD$(news, CHR$(9), 3)
      END IF
    CASE "built"
      IF i >= 0 THEN
        BdState(i) = "built"
        BdZip(i) = FIELD$(news, CHR$(9), 3)
        BdBytes(i) = VAL(FIELD$(news, CHR$(9), 4))
        BdMade = BdMade + BdZip(i) + CHR$(10)
        Application.Announce TR$("{app} built, {n} of {m}", "app", BdLabel(i), "n", LTRIM$(STR$(BdDoneCount)), "m", LTRIM$(STR$(BdApps)))
        BdNextWorking
      END IF
    CASE "failed"
      IF i >= 0 THEN
        BdState(i) = "failed"
        BdWhy(i) = FIELD$(news, CHR$(9), 3)
        Application.Announce TR$("{app} was not built: {why}", "app", BdLabel(i), "why", BdWhy(i)), 1
        BdNextWorking
      END IF
  END SELECT
  SetState "building", TR$("Building: {n} of {m} done", "n", LTRIM$(STR$(BdDoneCount)), "m", LTRIM$(STR$(BdApps)))
  IF BdShown THEN BdLayOut
END SUB

' (the next app waiting is the one being packed now)
SUB BdNextWorking
  DIM i AS INTEGER
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) AND (BdState(i) = "waiting" OR BdState(i) = "download") THEN
      IF BdState(i) = "waiting" THEN BdState(i) = "working"
      EXIT SUB
    END IF
  NEXT
END SUB

' Rust installed with `rapidr setup` (the Build page shows what it says):
' native code builds work from then on.
SUB InstallRust
  OutputBox.Clear "build"
  OutputBox.Page = "build"
  Dock.ShowPane("outputbox")
  BuildMode = "setup"
  IF Proj.InstallRust THEN
    BuildStarted = TIMER
    SetState "building", "Installing Rust..."
  ELSE
    BuildLine CHR$(27) + "[31m" + TR$("Can't install Rust: {error}", "error", Proj.Error) + CHR$(27) + "[0m"
    SetState "error", "Can't install Rust"
  END IF
END SUB

' The build ended (the zips' build, then native code when asked for).
SUB BuildDone(Code AS INTEGER, Built AS STRING)
  DIM took AS STRING, i AS INTEGER, p AS STRING, built AS INTEGER
  took = LTRIM$(STR$(INT(TIMER - BuildStarted)))
  IF BuildMode = "setup" THEN
    BuildLine ""
    IF Code = 0 THEN
      BuildLine CHR$(27) + "[32m" + TR$("✓ Rust is installed ({seconds} s): native code builds are ready.", "seconds", took) + CHR$(27) + "[0m"
      SetState "stopped", "Rust is installed"
    ELSE
      BuildLine CHR$(27) + "[31m" + TR$("✗ Installing Rust failed (exit code {code}) after {seconds} s: see above. `rapidr setup` in a terminal asks the same questions.", "code", Code, "seconds", took) + CHR$(27) + "[0m"
      SetState "error", "Installing Rust failed"
    END IF
    EXIT SUB
  END IF
  IF BuildMode = "webpage" THEN
    ' (the page's web app: its one row)
    BuildMode = "targets"
    IF Code = 0 THEN
      BdState(6) = "built": BdZip(6) = FileTitle(Built)
      BdMade = BdZip(6) + CHR$(10)
    ELSE
      BdState(6) = "failed": BdWhy(6) = TR$("Not built: see the Output's Build page.")
    END IF
    BdFinish
    IF Code = 0 THEN Proj.BuiltPath = Built
    EXIT SUB
  END IF
  IF BuildMode = "native" THEN
    BuildLine ""
    IF Code = 0 THEN
      BuildLine CHR$(27) + "[32m" + TR$("✓ Built {file} (native code)", "file", FileTitle(Built)) + CHR$(27) + "[0m   " + Hyperlink(TR$("Reveal in {manager}", "manager", Proj.FileManager), "reveal:" + Built)
    ELSE
      BuildLine CHR$(27) + "[31m" + TR$("✗ The native code build failed (exit code {code}): see above.", "code", Code) + CHR$(27) + "[0m"
    END IF
    BuildMode = "targets"
    BdFinish
    EXIT SUB
  END IF
  ' (an app the build never reached: not built)
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) AND BdState(i) <> "built" AND BdState(i) <> "failed" AND BdState(i) <> "stopped" THEN
      BdState(i) = "failed"
      IF BdWhy(i) = "" THEN BdWhy(i) = TR$("Not built: see the Output's Build page.")
    END IF
  NEXT
  ' (Everything in one zip: the zips put in one, beside them)
  IF Proj.PackAs = "one" AND BdMade <> "" AND Proj.FileManager <> "" THEN
    p = Proj.PackOneZip(BdMade)
    IF p <> "" THEN BuildLine TR$("All in one zip: {file}", "file", Hyperlink(p, "reveal:" + p))
  END IF
  IF BdNativeNext AND BdPhase = "building" THEN
    BdNativeNext = 0
    IF Proj.RustReady() = 0 THEN
      BuildLine TR$("Native code: Rust isn't installed on this computer (Build > Advanced > Native code; `rapidr setup` installs it)")
    ELSE
      BuildMode = "native"
      IF Proj.BuildNative() THEN
        BdNativeRun = 1
        SetState "building", TR$("Building {file} as native code...", "file", FileTitle(ProjectMain))
        EXIT SUB
      END IF
      BuildMode = "targets"
    END IF
  END IF
  BdFinish
END SUB

' The end of a build: the Done page (or the toast of Build Again).
SUB BdFinish
  DIM built AS INTEGER, first AS STRING, i AS INTEGER
  IF BdPhase <> "building" AND BdPhase <> "done" THEN EXIT SUB
  BdPhase = "done"
  BdSeconds = TIMER - BdStarted
  built = BdBuiltCount
  ' (build_app's answer: every app built, or not)
  IF ToolsBuildTicket <> "" THEN ToolsBuildDone Flag(built < BdApps)
  FOR i = 0 TO BD_APPS - 1
    IF BdOn(i) AND BdState(i) = "built" AND first = "" THEN first = BdOutDir + "/" + BdZip(i)
  NEXT
  IF first <> "" THEN Proj.BuiltPath = first
  BuildLine ""
  IF built = BdApps THEN
    BuildLine CHR$(27) + "[32m" + TRN$("✓ {n} app built in {s} s", "✓ {n} apps built in {s} s", built, "s", BdOne(BdSeconds)) + CHR$(27) + "[0m   " + Hyperlink(TR$("Reveal in {manager}", "manager", Proj.FileManager), "reveal:" + first)
    SetState "stopped", TRN$("{n} app built", "{n} apps built", built)
    Application.Announce TRN$("{n} app built in {s} seconds", "{n} apps built in {s} seconds", built, "s", BdOne(BdSeconds))
    IF BdShown = 0 THEN BdToastId = Toast(TRN$("{n} app built", "{n} apps built", built), "success", TR$("Show"), "build")
  ELSE
    BuildLine CHR$(27) + "[31m" + TR$("✗ {n} of {m} apps built: see above.", "n", LTRIM$(STR$(built)), "m", LTRIM$(STR$(BdApps))) + CHR$(27) + "[0m"
    SetState "error", TR$("{n} of {m} apps built", "n", LTRIM$(STR$(built)), "m", LTRIM$(STR$(BdApps)))
    IF BdShown = 0 THEN BdToastId = Toast(TR$("{n} of {m} apps built", "n", LTRIM$(STR$(built)), "m", LTRIM$(STR$(BdApps))), "error", TR$("Open Build"), "build")
  END IF
  IF BdShown THEN BdGo 5
END SUB

' Done's main button: the zips in Finder / File Explorer / Files (on the
' web: Done closes it).
SUB BdShowMade
  IF Proj.FileManager = "" THEN
    BuildDialog.Close
  ELSE
    RevealBuild
  END IF
END SUB

' A failed app's Try Again: the build again (the ones built stay).
SUB BdRowActClick(Sender AS RButton)
  BdStart
END SUB

' A toast's action ("build" key): Show the zips, or Open Build.
FUNCTION BuildToastAction(Key AS STRING, Index AS INTEGER) AS INTEGER
  Result = 0
  IF Key <> "build" THEN EXIT FUNCTION
  Result = 1
  IF BdPhase = "done" AND BdBuiltCount = BdApps THEN
    RevealBuild
  ELSE
    ShowBuildDialog 4
  END IF
END FUNCTION

' ---- the log -----------------------------------------------------------------------------

' Text that is a link to Target on the Output panel (a terminal's OSC 8
' hyperlink): clicking it gives OutputLink the target.
FUNCTION Hyperlink(Text AS STRING, Target AS STRING) AS STRING
  Result = CHR$(27) + "]8;;" + Target + CHR$(27) + "\" + Text + CHR$(27) + "]8;;" + CHR$(27) + "\"
END FUNCTION

' A line of the build's log (the Output panel's Build page).
SUB BuildLine(Text AS STRING)
  OutputBox.AddBuildLine Text
END SUB

SUB RevealBuild
  IF Proj.BuiltPath = "" THEN
    OutputLine TR$("Nothing built yet: Run > Build makes the apps.")
  ELSEIF Proj.Reveal = 0 THEN
    OutputLine TR$("Can't show it: {error}", "error", Proj.Error)
  END IF
END SUB

SUB SetUpBuild
  Proj.OnBuildOutput = BuildOutput
  Proj.OnBuildDone = BuildDone
  ' (the outline and the problems once typing pauses: documents.inc)
  Lang.OnAnalyzed = LangAnalyzed
END SUB

' (the look changed: the dialog's colours and icons with it)
SUB LayOutBuildDialog
  IF BdShown THEN BdLayOut
END SUB

' ---- test steps (the --do list): build:step=N, build:tick=target,
' build:untick=target, build:adv=2|3, build:state=<target>~<state>~<words>,
' build:phase=building|done, build:field=id~<text>, build:icon=<file>,
' build:start, build:close ----
FUNCTION BuildStep(S AS STRING) AS INTEGER
  DIM k AS INTEGER, what AS STRING, v AS STRING, i AS INTEGER
  Result = 0
  IF LEFT$(S, 6) <> "build:" THEN EXIT FUNCTION
  Result = 1
  k = INSTR(S, "=")
  IF k > 0 THEN
    what = MID$(S, 7, k - 7): v = MID$(S, k + 1)
  ELSE
    what = MID$(S, 7): v = ""
  END IF
  IF what <> "open" THEN BdEnsure
  SELECT CASE what
    CASE "open": ShowBuildDialog VAL(v)
    CASE "step": BdGo VAL(v)
    CASE "tick"
      i = BdIndex(v)
      IF i >= 0 THEN BdOn(i) = 1: BdKeep: BdRefresh
    CASE "untick"
      i = BdIndex(v)
      IF i >= 0 THEN BdOn(i) = 0: BdKeep: BdRefresh
    CASE "adv"
      IF v = "2" THEN BdAdv2 = 1 ELSE BdAdv3 = 1
      BdLayOut
    CASE "state"
      i = BdIndex(FIELD$(v, "~", 1))
      IF i >= 0 THEN
        BdState(i) = FIELD$(v, "~", 2)
        BdWhy(i) = FIELD$(v, "~", 3)
        BdNote(i) = FIELD$(v, "~", 3)
        IF BdState(i) = "built" THEN BdZip(i) = Proj.ZipName(BdTarget(i)): BdBytes(i) = VAL(FIELD$(v, "~", 3))
      END IF
      BdLayOut
    CASE "phase"
      BdPhase = v
      IF v = "building" THEN BdStarted = TIMER - 1.4
      IF v = "done" THEN BdSeconds = 2.8: BdOutDir = Proj.OutputPath()
      IF BdShown THEN BdGo 5
    CASE "field"
      SELECT CASE FIELD$(v, "~", 1)
        CASE "id": BdId.Text = FIELD$(v, "~", 2)
        CASE "name": BdName.Text = FIELD$(v, "~", 2)
        CASE "version": BdVersion.Text = FIELD$(v, "~", 2)
      END SELECT
      BdFieldChanged
      BdLayOut
    CASE "icon"
      Proj.Icon = v
      BdSaveNow
      BdShowIcon
      BdLayOut
    CASE "native"
      BdNative.Checked = VAL(v)
      BdKeep
    CASE "start": BdStart
    CASE "close": BuildDialog.Close
  END SELECT
END FUNCTION
