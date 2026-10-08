' preview.inc: "Preview in Classic" — one Studio setting that shows the
' program under development as RapidQ drew it, without editing its source:
' Run draws it in the classic look (a program that names no $THEME takes
' RPROGRAMSESSION.Theme, as it would RAPIDR_THEME), and so does the form
' designer. Studio itself keeps its own look. In the View menu, on the tool
' bar and in the command palette (view.previewClassic); saved with the
' other settings.

DIM PreviewClassic AS INTEGER

' (the command, beside the themes in the View menu; DefineCommands)
SUB DefinePreviewCommands
  AddCmd "view.previewClassic", "Preview in &Classic", "View", "", "", "", "actions/visible"
END SUB

' (its button on the tool bar; BuildToolBar)
SUB AddPreviewTool
  AddTool "view.previewClassic"
END SUB

' The look the program under development is drawn in: RapidQ's while the
' preview is on, else its own (its $THEME, or RapidR's).
FUNCTION PreviewTheme AS STRING
  IF PreviewClassic THEN Result = "classic" ELSE Result = ""
END FUNCTION

' The setting applied: the next Run's look, the designer's, the menu's
' check mark and the tool bar button's state.
SUB ApplyPreview
  DIM i AS INTEGER
  Session.Theme = PreviewTheme
  DesignPreviewTheme PreviewTheme
  CheckCmd "view.previewClassic", PreviewClassic
  i = CmdIndex("view.previewClassic")
  IF i >= 0 THEN
    IF Cmd(i).Tool >= 0 THEN TB(Cmd(i).Tool).Down = Flag(PreviewClassic)
  END IF
END SUB

SUB TogglePreviewClassic
  PreviewClassic = Flag(PreviewClassic = 0)
  ApplyPreview
  IF PreviewClassic THEN
    OutputLine "Preview in Classic: Run and the designer show the program as RapidQ drew it."
  ELSE
    OutputLine "Preview in Classic off: the program in its own look."
  END IF
END SUB

' (every open designer, and the ones opened later: OpenDocument)
SUB DesignPreviewTheme(Name AS STRING)
  DIM d AS INTEGER
  FOR d = 0 TO MAXDOCS
    DesignDoc(d).Theme = Name
  NEXT
END SUB
