' ROUTPUTCONSOLE (RapidR Studio's output console, docs/ide-components.md
' 3.8): a program's output with ANSI colours, CLS and LOCATE (the
' sequences RapidQ's COLOR, CLS and LOCATE print), a build log with a
' compiler's message, two problems. The user clicks a file:line link in
' the output, the Build tab and the link there, the Problems tab and a
' problem, then the Output tab; the program searches ("e": the matches
' marked, the first shown) and steps to the next match.
DIM log AS STRING
DIM e AS STRING
e = CHR$(27)

SUB LinkClicked (File AS STRING, Line AS INTEGER)
  log = log + " link:" + File + ":" + STR$(Line)
END SUB

SUB PageChanged (Page AS STRING)
  log = log + " page:" + Page
END SUB

SUB FindIt
  log = log + " find:" + STR$(Cons.Find("line")) + " next:" + STR$(Cons.FindNext)
END SUB

SUB Report
  lbl.Caption = log
  info.Caption = Cons.Page + " " + STR$(Cons.LineCount) + " " + STR$(Cons.ProblemCount) + " [" + Cons.Line(0) + "] [" + Cons.Line(5) + "] " + Cons.Filter
END SUB

CREATE Form AS QFORM
  Caption = "Output console"
  Width = 660: Height = 420
  CREATE Cons AS ROUTPUTCONSOLE
    Left = 8: Top = 8: Width = 628: Height = 300
    OnLinkClick = LinkClicked
    OnPageChange = PageChanged
  END CREATE
  CREATE BFind AS QBUTTON
    Left = 8: Top = 316: Caption = "Find"
    OnClick = FindIt
  END CREATE
  CREATE BReport AS QBUTTON
    Left = 90: Top = 316: Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 8: Top = 348: Width = 628
  END CREATE
  CREATE Info AS QLABEL
    Left = 8: Top = 368: Width = 628
  END CREATE
END CREATE

Cons.WriteLine "this line goes with CLS"
' CLS
Cons.Write e + "[2J" + e + "[H"
Cons.WriteLine "RapidR output console"
' COLOR 4: red; COLOR 14, 1: yellow on blue; COLOR: back to the theme's
Cons.WriteLine e + "[31mred line" + e + "[0m, then " + e + "[93;44myellow on blue" + e + "[0m"
Cons.WriteLine "see Main.rr:12 for the line"
Cons.Write e + "[32m"
Cons.WriteLine "green, then a tab:" + CHR$(9) + "after it"
Cons.Write e + "[0m"
' LOCATE 6, 12
Cons.Write e + "[6;12HLOCATE 6, 12"
Cons.Write e + "[7;1H" + e + "[96mlast line" + e + "[0m"
Cons.AddBuildLine "Compiling Main.rr"
Cons.AddBuildLine "src/app.bas:3:5: error: Undefined variable X"
Cons.AddBuildLine "Build failed: 1 error, 1 warning"
Cons.AddProblem "Main.rr", 12, 5, "error", "Undefined variable X"
Cons.AddProblem "src/app.bas", 3, 1, "warning", "Unused variable Y"
Form.ShowModal
