' build.inc: Build (the program made into an app for this computer's system:
' Name.app on macOS, the .exe with its icon on Windows, Name.AppDir on
' Linux — RPROJECT.Build runs `rapidr build`), Reveal in Finder / File
' Explorer / Files, and the project's app settings (icon, name, bundle ID,
' version) with the icon's preview. docs/manual/building-apps.md

DECLARE SUB BuildOutput(Text AS STRING)
DECLARE SUB BuildDone(Code AS INTEGER, Built AS STRING)
DECLARE SUB AppIconBrowse
DECLARE SUB AppIconDefault
DECLARE SUB AppSettingsOk
DECLARE SUB AppSettingsCancel

DIM AppIconBmp AS QBITMAP
DIM BuildStarted AS DOUBLE

' ---- Project > App Settings ----------------------------------------------------------
CREATE AppDialog AS QFORM
  Caption = "Project Options"
  Width = 520: Height = 330
  BorderStyle = 3
  CREATE AppIconImage AS QIMAGE
    Left = 16: Top = 16: Width = 64: Height = 64
    Stretch = 1
    AccessibleName = "The app's icon"
  END CREATE
  CREATE AppIconNote AS QLABEL
    Left = 96: Top = 16: Width = 330: Height = 40
    WordWrap = 1
    Caption = ""
  END CREATE
  CREATE AppNameLabel AS QLABEL
    Left = 16: Top = 100: Width = 90: Caption = "App &name:"
  END CREATE
  CREATE AppNameEdit AS QEDIT
    Left = 120: Top = 96: Width = 320
  END CREATE
  CREATE AppIdLabel AS QLABEL
    Left = 16: Top = 130: Width = 90: Caption = "&Bundle ID:"
  END CREATE
  CREATE AppIdEdit AS QEDIT
    Left = 120: Top = 126: Width = 320
  END CREATE
  CREATE AppVersionLabel AS QLABEL
    Left = 16: Top = 160: Width = 90: Caption = "&Version:"
  END CREATE
  CREATE AppVersionEdit AS QEDIT
    Left = 120: Top = 156: Width = 320
  END CREATE
  CREATE AppCompanyLabel AS QLABEL
    Left = 16: Top = 190: Width = 90: Caption = "&Company:"
  END CREATE
  CREATE AppCompanyEdit AS QEDIT
    Left = 120: Top = 186: Width = 320
  END CREATE
  CREATE AppIconLabel AS QLABEL
    Left = 16: Top = 220: Width = 90: Caption = "&Icon:"
  END CREATE
  CREATE AppIconEdit AS QEDIT
    Left = 120: Top = 216: Width = 250
  END CREATE
  CREATE AppIconBtn AS QBUTTON
    Left = 376: Top = 215: Width = 30: Caption = "..."
    AccessibleName = "Choose the icon file"
    OnClick = AppIconBrowse
  END CREATE
  CREATE AppIconReset AS QBUTTON
    Left = 410: Top = 215: Width = 30: Caption = "x"
    AccessibleName = "Use RapidR's icon"
    OnClick = AppIconDefault
  END CREATE
  CREATE AppKindLabel AS QLABEL
    Left = 16: Top = 250: Width = 90: Caption = "B&uild:"
  END CREATE
  CREATE AppKind AS QCOMBOBOX
    Left = 120: Top = 246: Width = 320
    AddItems "Compiled (native: needs Rust)", "Interpreted (no Rust needed)"
  END CREATE
  CREATE AppOk AS QBUTTON
    Left = 254: Top = 280: Width = 84: Caption = "OK"
    Default = 1
    OnClick = AppSettingsOk
  END CREATE
  CREATE AppNo AS QBUTTON
    Left = 346: Top = 280: Width = 84: Caption = "Cancel"
    Cancel = 1
    OnClick = AppSettingsCancel
  END CREATE
END CREATE

' (RapidQ's sizes in the classic look, the RAPIDR look's elsewhere — as
' LayOutDialogs does for Studio's other dialogs)
SUB LayOutAppDialog
  DIM row AS INTEGER, fh AS INTEGER, bh AS INTEGER, bw AS INTEGER, x AS INTEGER, w AS INTEGER, y AS INTEGER, pad AS INTEGER, ic AS INTEGER
  IF ThemeName = "classic" THEN
    row = 30: fh = 21: bh = 25: bw = 84: x = 110: w = 320: pad = 16: ic = 48
  ELSE
    row = 40: fh = 28: bh = 32: bw = 96: x = 112: w = 340: pad = 20: ic = 64
  END IF
  AppDialog.Font.Name = UIFont.Name: AppDialog.Font.Size = UIFont.Size
  y = pad
  AppIconImage.Left = pad: AppIconImage.Top = y: AppIconImage.Width = ic: AppIconImage.Height = ic
  AppIconNote.Left = pad + ic + 16: AppIconNote.Top = y + 4: AppIconNote.Width = x + w - AppIconNote.Left: AppIconNote.Height = ic - 4
  y = y + ic + pad
  AppNameLabel.Left = pad: AppNameLabel.Top = y + (fh - 16) / 2
  AppNameEdit.Left = x: AppNameEdit.Top = y: AppNameEdit.Width = w: AppNameEdit.Height = fh
  y = y + row
  AppIdLabel.Left = pad: AppIdLabel.Top = y + (fh - 16) / 2
  AppIdEdit.Left = x: AppIdEdit.Top = y: AppIdEdit.Width = w: AppIdEdit.Height = fh
  y = y + row
  AppVersionLabel.Left = pad: AppVersionLabel.Top = y + (fh - 16) / 2
  AppVersionEdit.Left = x: AppVersionEdit.Top = y: AppVersionEdit.Width = w: AppVersionEdit.Height = fh
  y = y + row
  AppCompanyLabel.Left = pad: AppCompanyLabel.Top = y + (fh - 16) / 2
  AppCompanyEdit.Left = x: AppCompanyEdit.Top = y: AppCompanyEdit.Width = w: AppCompanyEdit.Height = fh
  y = y + row
  AppIconLabel.Left = pad: AppIconLabel.Top = y + (fh - 16) / 2
  AppIconEdit.Left = x: AppIconEdit.Top = y: AppIconEdit.Width = w - 2 * (fh + 6): AppIconEdit.Height = fh
  AppIconBtn.Left = x + w - 2 * fh - 6: AppIconBtn.Top = y: AppIconBtn.Width = fh: AppIconBtn.Height = fh
  AppIconReset.Left = x + w - fh: AppIconReset.Top = y: AppIconReset.Width = fh: AppIconReset.Height = fh
  y = y + row
  AppKindLabel.Left = pad: AppKindLabel.Top = y + (fh - 16) / 2
  AppKind.Left = x: AppKind.Top = y: AppKind.Width = w
  y = y + row + pad / 2
  AppNo.Width = bw: AppNo.Height = bh: AppNo.Left = x + w - bw: AppNo.Top = y
  AppOk.Width = bw: AppOk.Height = bh: AppOk.Left = x + w - 2 * bw - 8: AppOk.Top = y
  AppDialog.ClientWidth = x + w + pad
  AppDialog.ClientHeight = y + bh + pad
END SUB

' The icon's preview: the file in the Icon field (the project's own), else
' RapidR's, drawn at twice the size shown (sharp on a Retina screen).
SUB ShowAppIcon
  DIM keep AS STRING, png AS STRING
  keep = Proj.Icon
  Proj.Icon = AppIconEdit.Text
  png = Proj.IconPreview(128)
  IF png = "" THEN
    AppIconNote.Caption = Proj.Error
    Proj.Icon = ""
    png = Proj.IconPreview(128)
  ELSEIF AppIconEdit.Text = "" THEN
    AppIconNote.Caption = "RapidR's icon for programs. Choose an .icns, .ico, .png (1024 x 1024 is best) or .svg for the app's own."
  ELSE
    AppIconNote.Caption = "The app's icon, made into every size each system shows."
  END IF
  Proj.Icon = keep
  IF png <> "" THEN
    AppIconBmp.LoadFromFile png
    AppIconImage.BMPHandle = AppIconBmp
  END IF
END SUB

SUB ShowAppSettings
  IF ProjectOpen = 0 THEN EXIT SUB
  LayOutAppDialog
  AppNameEdit.Text = Proj.AppName
  AppIdEdit.Text = Proj.BundleID
  AppVersionEdit.Text = Proj.Version
  AppCompanyEdit.Text = Proj.Company
  AppIconEdit.Text = Proj.Icon
  IF Proj.BuildKind = "interpreted" THEN AppKind.ItemIndex = 1 ELSE AppKind.ItemIndex = 0
  ShowAppIcon
  AppDialog.ShowModal
END SUB

SUB AppIconBrowse
  OpenDlg.Caption = "The App's Icon"
  OpenDlg.Filter = "Icons and pictures (*.icns;*.ico;*.png;*.svg)|*.icns;*.ico;*.png;*.svg|All files|*.*"
  IF OpenDlg.Execute THEN
    AppIconEdit.Text = OpenDlg.FileName
    ShowAppIcon
  END IF
END SUB

SUB AppIconDefault
  AppIconEdit.Text = ""
  ShowAppIcon
END SUB

SUB AppSettingsOk
  Proj.AppName = AppNameEdit.Text
  Proj.BundleID = AppIdEdit.Text
  Proj.Version = AppVersionEdit.Text
  Proj.Company = AppCompanyEdit.Text
  Proj.Icon = AppIconEdit.Text
  IF AppKind.ItemIndex = 1 THEN Proj.BuildKind = "interpreted" ELSE Proj.BuildKind = "native"
  AppDialog.Close
  SaveProjectFile
END SUB

SUB AppSettingsCancel
  AppDialog.Close
END SUB

' ---- Build ---------------------------------------------------------------------------

SUB BuildProject
  IF ProjectOpen = 0 OR Proj.Building THEN EXIT SUB
  SaveAll
  AnalyzeActive
  IF Lang.ErrorCount > 0 THEN
    OutputLine "Build: " + LTRIM$(STR$(Lang.ErrorCount)) + " errors (see Problems)."
    SetState "error", "Can't build: the program has errors"
    OutputBox.Page = "problems"
    Dock.FocusPane("outputbox")
    EXIT SUB
  END IF
  OutputBox.Clear
  Dock.ShowPane("outputbox")
  OutputLine "Building " + FileTitle(ProjectMain) + " (" + Proj.BuildKind + ")..."
  IF Proj.Build THEN
    BuildStarted = TIMER
    SetState "building", "Building " + FileTitle(ProjectMain) + "..."
  ELSE
    OutputLine "Can't build: " + Proj.Error
    SetState "error", "Can't build " + FileTitle(ProjectMain)
  END IF
END SUB

SUB BuildOutput(Text AS STRING)
  OutputLine Text
END SUB

SUB BuildDone(Code AS INTEGER, Built AS STRING)
  IF Code = 0 THEN
    OutputLine "Built " + Built + " in " + LTRIM$(STR$(INT(TIMER - BuildStarted))) + " s. Run > Reveal in " + Proj.FileManager + " shows it."
    SetState "stopped", "Built " + FileTitle(Built)
  ELSE
    OutputLine "The build failed (exit code " + LTRIM$(STR$(Code)) + "): see above."
    SetState "error", "The build failed"
  END IF
END SUB

SUB RevealBuild
  IF Proj.BuiltPath = "" THEN
    OutputLine "Nothing built yet: Run > Build (Ctrl+Shift+B) makes the app."
  ELSEIF Proj.Reveal = 0 THEN
    OutputLine "Can't show it: " + Proj.Error
  END IF
END SUB

SUB SetUpBuild
  Proj.OnBuildOutput = BuildOutput
  Proj.OnBuildDone = BuildDone
END SUB
