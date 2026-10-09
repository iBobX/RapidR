' build.inc: Build (the program made into an app for this computer's system:
' Name.app on macOS, the .exe with its icon on Windows, Name.AppDir on
' Linux — RPROJECT.Build runs `rapidr build`, a release build, into the
' project's output folder), its log on the Output panel's Build page with
' links that show what it made, Reveal in Finder / File Explorer / Files,
' and Project Options: the app's settings (icon, name, bundle ID, version)
' with the icon's preview, and the build's (native or interpreted, the
' output folder, the generated Rust kept or not). docs/manual/building-apps.md

DECLARE SUB BuildOutput(Text AS STRING)
DECLARE SUB BuildDone(Code AS INTEGER, Built AS STRING)
DECLARE SUB AppIconBrowse
DECLARE SUB AppIconDefault
DECLARE SUB AppOutputBrowse
DECLARE SUB AppBuildChanged
DECLARE SUB AppSettingsOk
DECLARE SUB AppSettingsCancel

DIM AppIconBmp AS RBitmap
DIM BuildStarted AS DOUBLE

' ---- Project > Project Options -------------------------------------------------------
CREATE AppDialog AS RForm
  Caption = "Project Options"
  Width = 520: Height = 480
  BorderStyle = 3
  CREATE AppIconImage AS RImage
    Left = 16: Top = 16: Width = 64: Height = 64
    Stretch = 1
    AccessibleName = "The app's icon"
  END CREATE
  CREATE AppIconNote AS RLabel
    Left = 96: Top = 16: Width = 330: Height = 40
    WordWrap = 1: AutoSize = 0
    Caption = ""
  END CREATE
  CREATE AppNameLabel AS RLabel
    Left = 16: Top = 100: Width = 90: Caption = "App &name:"
  END CREATE
  CREATE AppNameEdit AS REdit
    Left = 120: Top = 96: Width = 320
  END CREATE
  CREATE AppIdLabel AS RLabel
    Left = 16: Top = 130: Width = 90: Caption = "&Bundle ID:"
  END CREATE
  CREATE AppIdEdit AS REdit
    Left = 120: Top = 126: Width = 320
  END CREATE
  CREATE AppVersionLabel AS RLabel
    Left = 16: Top = 160: Width = 90: Caption = "&Version:"
  END CREATE
  CREATE AppVersionEdit AS REdit
    Left = 120: Top = 156: Width = 320
  END CREATE
  CREATE AppCompanyLabel AS RLabel
    Left = 16: Top = 190: Width = 90: Caption = "&Company:"
  END CREATE
  CREATE AppCompanyEdit AS REdit
    Left = 120: Top = 186: Width = 320
  END CREATE
  CREATE AppIconLabel AS RLabel
    Left = 16: Top = 220: Width = 90: Caption = "&Icon:"
  END CREATE
  CREATE AppIconEdit AS REdit
    Left = 120: Top = 216: Width = 250
  END CREATE
  CREATE AppIconBtn AS RButton
    Left = 376: Top = 215: Width = 30: Caption = "..."
    AccessibleName = "Choose the icon file"
    OnClick = AppIconBrowse
  END CREATE
  CREATE AppIconReset AS RButton
    Left = 410: Top = 215: Width = 30: Caption = "x"
    AccessibleName = "Use RapidR's icon"
    OnClick = AppIconDefault
  END CREATE
  CREATE AppKindLabel AS RLabel
    Left = 16: Top = 250: Width = 90: Caption = "B&uild:"
  END CREATE
  CREATE AppKind AS RComboBox
    Left = 120: Top = 246: Width = 320
    AddItems "Compiled (native: needs Rust)", "Interpreted (no Rust needed)"
    OnChange = AppBuildChanged
  END CREATE
  CREATE AppKindNote AS QLABEL
    Left = 120: Top = 276: Width = 320: Height = 32
    WordWrap = 1: AutoSize = 0
    Caption = "A release build: optimized, what you ship. Run (F5) runs the program to debug it."
  END CREATE
  CREATE AppOutputLabel AS QLABEL
    Left = 16: Top = 316: Width = 90: Caption = "&Output folder:"
  END CREATE
  CREATE AppOutputEdit AS QEDIT
    Left = 120: Top = 312: Width = 284
    AccessibleName = "Output folder: where Build puts the app"
    OnChange = AppBuildChanged
  END CREATE
  CREATE AppOutputBtn AS QBUTTON
    Left = 410: Top = 311: Width = 30: Caption = "..."
    AccessibleName = "Choose the output folder"
    OnClick = AppOutputBrowse
  END CREATE
  CREATE AppKeepRust AS QCHECKBOX
    Left = 120: Top = 346: Width = 320
    Caption = "&Keep the generated Rust source"
    OnClick = AppBuildChanged
  END CREATE
  CREATE AppKeepNote AS QLABEL
    Left = 140: Top = 368: Width = 300: Height = 32
    WordWrap = 1: AutoSize = 0
    Caption = ""
  END CREATE
  CREATE AppOk AS RButton
    Left = 254: Top = 410: Width = 84: Caption = "OK"
    Default = 1
    OnClick = AppSettingsOk
  END CREATE
  CREATE AppNo AS RButton
    Left = 346: Top = 410: Width = 84: Caption = "Cancel"
    Cancel = 1
    OnClick = AppSettingsCancel
  END CREATE
END CREATE

' (RapidQ's sizes in the classic look, the RAPIDR look's elsewhere — as
' LayOutDialogs does for Studio's other dialogs)
SUB LayOutAppDialog
  DIM row AS INTEGER, fh AS INTEGER, bh AS INTEGER, bw AS INTEGER, x AS INTEGER, w AS INTEGER, y AS INTEGER, pad AS INTEGER, ic AS INTEGER, nh AS INTEGER, ch AS INTEGER
  IF ThemeName = "classic" THEN
    row = 30: fh = 21: bh = 25: bw = 84: x = 110: w = 320: pad = 16: ic = 48: nh = 28: ch = 17
  ELSE
    row = 40: fh = 28: bh = 32: bw = 96: x = 112: w = 340: pad = 20: ic = 64: nh = 32: ch = 20
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
  ' (the build's settings below the app's, a little apart)
  y = y + row + pad / 2
  AppKindLabel.Left = pad: AppKindLabel.Top = y + (fh - 16) / 2
  AppKind.Left = x: AppKind.Top = y: AppKind.Width = w
  y = y + fh + 4
  AppKindNote.Left = x: AppKindNote.Top = y: AppKindNote.Width = w: AppKindNote.Height = nh
  y = y + nh + 8
  AppOutputLabel.Left = pad: AppOutputLabel.Top = y + (fh - 16) / 2
  AppOutputEdit.Left = x: AppOutputEdit.Top = y: AppOutputEdit.Width = w - fh - 6: AppOutputEdit.Height = fh
  AppOutputBtn.Left = x + w - fh: AppOutputBtn.Top = y: AppOutputBtn.Width = fh: AppOutputBtn.Height = fh
  y = y + row
  AppKeepRust.Left = x: AppKeepRust.Top = y: AppKeepRust.Width = w: AppKeepRust.Height = ch
  y = y + ch + 2
  AppKeepNote.Left = x + 20: AppKeepNote.Top = y: AppKeepNote.Width = w - 20: AppKeepNote.Height = nh
  y = y + nh + pad
  AppNo.Width = bw: AppNo.Height = bh: AppNo.Left = x + w - bw: AppNo.Top = y
  AppOk.Width = bw: AppOk.Height = bh: AppOk.Left = x + w - 2 * bw - 8: AppOk.Top = y
  AppDialog.ClientWidth = x + w + pad
  AppDialog.ClientHeight = y + bh + pad
  ' (the notes in the theme's dim text)
  AppKindNote.Font.Color = TC("page.dim")
  AppKeepNote.Font.Color = TC("page.dim")
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

' (the output folder as the dialog shows it: "build" when the project names none)
FUNCTION OutputShown AS STRING
  Result = AppOutputEdit.Text
  IF Result = "" THEN Result = "build"
END FUNCTION

' The Rust source's note follows the build's settings: where it goes, or
' that an interpreted build has none.
SUB AppBuildChanged
  AppKeepRust.Enabled = (AppKind.ItemIndex = 0)
  IF AppKind.ItemIndex = 1 THEN
    AppKeepNote.Caption = "An interpreted build has no Rust: the app runs the program's bytecode."
  ELSEIF AppKeepRust.Checked THEN
    AppKeepNote.Caption = "Left in " + OutputShown + "/" + BaseName(ProjectMain) + "-rust-source, beside the app, to read."
  ELSE
    AppKeepNote.Caption = "Off: the Rust RapidR generates is deleted after the build."
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
  AppOutputEdit.Text = Proj.OutputFolder
  IF AppOutputEdit.Text = "" THEN AppOutputEdit.Text = "build"
  AppKeepRust.Checked = Proj.KeepRust
  AppBuildChanged
  ShowAppIcon
  AppDialog.Center
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

SUB AppOutputBrowse
  OpenDlg.Caption = "The Output Folder"
  OpenDlg.PickFolder = 1
  IF OpenDlg.Execute THEN AppOutputEdit.Text = OpenDlg.FileName
  OpenDlg.PickFolder = 0
END SUB

SUB AppSettingsOk
  Proj.AppName = AppNameEdit.Text
  Proj.BundleID = AppIdEdit.Text
  Proj.Version = AppVersionEdit.Text
  Proj.Company = AppCompanyEdit.Text
  Proj.Icon = AppIconEdit.Text
  IF AppKind.ItemIndex = 1 THEN Proj.BuildKind = "interpreted" ELSE Proj.BuildKind = "native"
  ' (the default, build, isn't written in the project file)
  IF OutputShown = "build" THEN Proj.OutputFolder = "" ELSE Proj.OutputFolder = AppOutputEdit.Text
  Proj.KeepRust = AppKeepRust.Checked
  AppDialog.Close
  SaveProjectFile
END SUB

SUB AppSettingsCancel
  AppDialog.Close
END SUB

' ---- Build ---------------------------------------------------------------------------

' Text that is a link to Target on the Output panel (a terminal's OSC 8
' hyperlink): clicking it gives OutputLink the target.
FUNCTION Hyperlink(Text AS STRING, Target AS STRING) AS STRING
  Result = CHR$(27) + "]8;;" + Target + CHR$(27) + "\" + Text + CHR$(27) + "]8;;" + CHR$(27) + "\"
END FUNCTION

' A line of the build's log (the Output panel's Build page).
SUB BuildLine(Text AS STRING)
  OutputBox.AddBuildLine Text
END SUB

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
  OutputBox.Clear "build"
  OutputBox.Page = "build"
  Dock.ShowPane("outputbox")
  IF Proj.Build THEN
    BuildStarted = TIMER
    SetState "building", "Building " + FileTitle(ProjectMain) + "..."
  ELSE
    BuildLine CHR$(27) + "[31mCan't build: " + Proj.Error + CHR$(27) + "[0m"
    SetState "error", "Can't build " + FileTitle(ProjectMain)
  END IF
END SUB

' A line rapidr build printed: what it made and the Rust it kept are links
' that show them in the file manager.
SUB BuildOutput(Text AS STRING)
  DIM k AS INTEGER, head AS STRING, path AS STRING
  k = INSTR(Text, ": ")
  IF k > 0 THEN
    head = LEFT$(Text, k - 1)
    path = MID$(Text, k + 2)
    IF (head = "App" OR head = "AppDir" OR head = "Executable" OR head = "Rust source" OR head = "Output folder") AND Proj.FileManager <> "" THEN
      BuildLine head + ": " + Hyperlink(path, "reveal:" + path)
      EXIT SUB
    END IF
  END IF
  BuildLine Text
END SUB

SUB BuildDone(Code AS INTEGER, Built AS STRING)
  DIM took AS STRING
  took = LTRIM$(STR$(INT(TIMER - BuildStarted)))
  IF Code = 0 THEN
    BuildLine ""
    BuildLine CHR$(27) + "[32m" + "✓ Built " + FileTitle(Built) + " in " + took + " s" + CHR$(27) + "[0m   " + Hyperlink("Reveal in " + Proj.FileManager, "reveal:" + Built)
    SetState "stopped", "Built " + FileTitle(Built)
  ELSE
    BuildLine ""
    BuildLine CHR$(27) + "[31m" + "✗ The build failed (exit code " + LTRIM$(STR$(Code)) + ") after " + took + " s: see above." + CHR$(27) + "[0m"
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
