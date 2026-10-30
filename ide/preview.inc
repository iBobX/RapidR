' preview.inc: "Preview in Classic" — one Studio setting that shows the
' program under development as RapidQ drew it, without editing its source:
' Run draws it in the classic look (a program that names no $THEME takes
' RPROGRAMSESSION.Theme, as it would RAPIDR_THEME), and so does the form
' designer. Studio itself keeps its own look. In the View menu, on the tool
' bar and in the command palette (view.previewClassic); saved with the
' other settings.

DIM PreviewClassic AS INTEGER
' The look the main program names with $THEME, as the designer draws it
' ("" while no project is open; "RapidR" follows the system): the designer
' previews the program, so a look the program names it keeps whatever look
' Studio has.
DIM ProgramLook AS STRING

' (the command, beside the themes in the View menu; DefineCommands)
SUB DefinePreviewCommands
  AddCmd "view.previewClassic", "Preview in &Classic", "View", "", "", "", "actions/visible"
END SUB

' (its button on the tool bar; BuildToolBar)
SUB AddPreviewTool
  DIM i AS INTEGER
  i = CmdIndex("view.previewClassic")
  IF i < 0 THEN EXIT SUB
  ToolBar.AddToggle Cmd(i).Id, CmdIcon(i), PlainTitle(Cmd(i).Title), Cmd(i).Id
  Cmd(i).Tool = 1
  ToolBar.ButtonDown Cmd(i).Id, Flag(PreviewClassic)
END SUB

' The look the program under development is drawn in: RapidQ's while the
' preview is on, else its own (its $THEME, or RapidR's).
FUNCTION PreviewTheme AS STRING
  IF PreviewClassic THEN Result = "classic" ELSE Result = ""
END FUNCTION

' The look the form designer draws in: RapidQ's while the preview is on,
' else the look the main program names ($THEME) — whatever look Studio has
' itself —, and when it names none (RapidR: as the system is) Studio's own,
' which follows the system unless View > Theme chose another.
FUNCTION DesignTheme AS STRING
  IF PreviewClassic THEN Result = "classic" ELSE Result = ProgramLook
END FUNCTION

' The setting applied: the next Run's look, the designer's, the menu's
' check mark and the tool bar button's state.
SUB ApplyPreview
  DIM i AS INTEGER
  Session.Theme = PreviewTheme
  DesignPreviewTheme DesignTheme
  CheckCmd "view.previewClassic", PreviewClassic
  i = CmdIndex("view.previewClassic")
  IF i >= 0 THEN
    IF Cmd(i).Tool >= 0 THEN ToolBar.ButtonDown Cmd(i).Id, Flag(PreviewClassic)
  END IF
END SUB

SUB TogglePreviewClassic
  PreviewClassic = Flag(PreviewClassic = 0)
  ApplyPreview
  IF PreviewClassic THEN
    OutputLine TR$("Preview in Classic: Run and the designer show the program in the classic look.")
  ELSE
    OutputLine TR$("Preview in Classic off: the program in its own look.")
  END IF
END SUB

' (every open designer, and the ones opened later: OpenDocument)
SUB DesignPreviewTheme(Name AS STRING)
  DIM d AS INTEGER
  FOR d = 0 TO MAXDOCS
    DesignDoc(d).Theme = Name
  NEXT
END SUB
