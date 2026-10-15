' settings.inc: Tools > Settings, Studio's own settings (kept in the
' per-user store: project.inc's LoadSettings / SaveSettings). The include
' folders: where $INCLUDE looks after the program's own folder, in order, as
' RapidQ's include folder — for everything Studio compiles: Open, the
' language service, Run, Build, Import RapidQ (RPROJECT.IncludeFolders;
' docs/manual/importing-rapidq.md, "Include files"). The themes and the
' views stay where they were: the command palette's View commands.

DECLARE SUB SettingsAddFolder
DECLARE SUB SettingsRemoveFolder
DECLARE SUB SettingsPicked
DECLARE SUB SettingsViews
DECLARE SUB SettingsOk
DECLARE SUB SettingsCancel
DECLARE SUB SetIncludeFolders(Folders AS STRING)

CREATE SettingsDialog AS RForm
  Caption = "Settings"
  Width = 560: Height = 420
  BorderStyle = 3
  CREATE IncFoldersLabel AS RLabel
    Caption = "&Include folders"
  END CREATE
  CREATE IncFoldersNote AS RLabel
    WordWrap = 1: AutoSize = 0
    Caption = "Where $INCLUDE looks after the program's own folder, in this order (as RapidQ's include folder): add the folder that holds your RapidQ includes (WINDOWS.INC, RAPIDQ2.INC, your libraries). RAPIDQ.INC needs none: RapidR has it built in."
  END CREATE
  CREATE IncFolders AS RListBox
    AccessibleName = "Include folders"
    OnClick = SettingsPicked
  END CREATE
  CREATE IncAdd AS RButton
    Caption = "&Add Folder..."
    AccessibleName = "Add an include folder"
    OnClick = SettingsAddFolder
  END CREATE
  CREATE IncRemove AS RButton
    Caption = "&Remove"
    AccessibleName = "Remove the selected include folder"
    OnClick = SettingsRemoveFolder
  END CREATE
  CREATE SettingsViewsBtn AS RButton
    Caption = "Themes and &Views..."
    AccessibleName = "The themes and views, in the command palette"
    OnClick = SettingsViews
  END CREATE
  CREATE SettingsOkBtn AS RButton
    Caption = "OK"
    Default = 1
    OnClick = SettingsOk
  END CREATE
  CREATE SettingsNo AS RButton
    Caption = "Cancel"
    Cancel = 1
    OnClick = SettingsCancel
  END CREATE
END CREATE

' (RapidQ's sizes in the classic look, the RAPIDR look's elsewhere — as
' LayOutAppDialog does)
SUB LayOutSettingsDialog
  DIM pad AS INTEGER, w AS INTEGER, bh AS INTEGER, bw AS INTEGER, y AS INTEGER, nh AS INTEGER, lh AS INTEGER, side AS INTEGER
  IF ThemeName = "classic" THEN
    pad = 16: w = 470: bh = 25: bw = 84: nh = 58: lh = 130: side = 120
  ELSE
    pad = 20: w = 500: bh = 32: bw = 96: nh = 66: lh = 150: side = 140
  END IF
  SettingsDialog.Font.Name = UIFont.Name: SettingsDialog.Font.Size = UIFont.Size
  y = pad
  IncFoldersLabel.Left = pad: IncFoldersLabel.Top = y
  IncFoldersLabel.Font.Bold = 1
  y = y + 24
  IncFoldersNote.Left = pad: IncFoldersNote.Top = y: IncFoldersNote.Width = w: IncFoldersNote.Height = nh
  IncFoldersNote.Font.Color = TC("page.dim")
  y = y + nh + 8
  IncFolders.Left = pad: IncFolders.Top = y: IncFolders.Width = w - side - 12: IncFolders.Height = lh
  IncAdd.Left = pad + w - side: IncAdd.Top = y: IncAdd.Width = side: IncAdd.Height = bh
  IncRemove.Left = pad + w - side: IncRemove.Top = y + bh + 8: IncRemove.Width = side: IncRemove.Height = bh
  y = y + lh + pad
  SettingsViewsBtn.Left = pad: SettingsViewsBtn.Top = y: SettingsViewsBtn.Width = side + 40: SettingsViewsBtn.Height = bh
  SettingsNo.Width = bw: SettingsNo.Height = bh: SettingsNo.Left = pad + w - bw: SettingsNo.Top = y
  SettingsOkBtn.Width = bw: SettingsOkBtn.Height = bh: SettingsOkBtn.Left = pad + w - 2 * bw - 8: SettingsOkBtn.Top = y
  SettingsDialog.ClientWidth = w + 2 * pad
  SettingsDialog.ClientHeight = y + bh + pad
END SUB

SUB ShowSettings
  DIM n AS INTEGER, i AS INTEGER, f AS STRING, all AS STRING
  LayOutSettingsDialog
  IncFolders.Clear
  all = Proj.IncludeFolders
  n = TALLY(all, CHR$(10)) + 1
  FOR i = 1 TO n
    f = FIELD$(all, CHR$(10), i)
    IF f <> "" THEN IncFolders.AddItems f
  NEXT
  SettingsPicked
  SettingsDialog.Center
  SettingsDialog.ShowModal
END SUB

SUB SettingsPicked
  IncRemove.Enabled = (IncFolders.ItemIndex >= 0)
END SUB

SUB SettingsAddFolder
  DIM i AS INTEGER, f AS STRING
  OpenDlg.Caption = "Add an Include Folder"
  OpenDlg.PickFolder = 1
  IF OpenDlg.Execute THEN f = OpenDlg.FileName
  OpenDlg.PickFolder = 0
  IF f = "" THEN EXIT SUB
  ' (once each)
  FOR i = 0 TO IncFolders.ItemCount - 1
    IF LCASE$(IncFolders.Item(i)) = LCASE$(f) THEN EXIT SUB
  NEXT
  IncFolders.AddItems f
  IncFolders.ItemIndex = IncFolders.ItemCount - 1
  SettingsPicked
END SUB

SUB SettingsRemoveFolder
  IF IncFolders.ItemIndex < 0 THEN EXIT SUB
  IncFolders.DelItems IncFolders.ItemIndex
  SettingsPicked
END SUB

SUB SettingsViews
  SettingsDialog.Close
  ShowPalette "View: "
END SUB

SUB SettingsOk
  DIM i AS INTEGER, all AS STRING
  FOR i = 0 TO IncFolders.ItemCount - 1
    IF all <> "" THEN all = all + CHR$(10)
    all = all + IncFolders.Item(i)
  NEXT
  SetIncludeFolders all
  SettingsDialog.Close
  SaveSettings
END SUB

SUB SettingsCancel
  SettingsDialog.Close
END SUB

' The include folders from now on (a folder a line), said in Output; the
' open program's problems found again with them.
SUB SetIncludeFolders(Folders AS STRING)
  Proj.IncludeFolders = Folders
  IF Folders = "" THEN
    OutputLine "Include folders: none ($INCLUDE looks in the program's folder)"
  ELSE
    OutputLine "Include folders: " + REPLACESUBSTR$(Folders, CHR$(10), ", ")
  END IF
  IF ProjectOpen THEN FillProblemsOf ProjectMain
END SUB
