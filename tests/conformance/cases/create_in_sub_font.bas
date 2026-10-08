' A CREATE inside a SUB sets its object's font (`Font.Name = …` in the
' block): native builds compiled it as a Rust field (RapidQ's
' direct3d/3DPong_aDelic2.bas); the output is RC.EXE's.
$APPTYPE CONSOLE
SUB Intro
  CREATE B AS QBITMAP
    Width = 64
    Font.Name = "Impact"
    Font.Size = 42
  END CREATE
  PRINT B.Font.Name; " "; B.Font.Size; " "; B.Width
END SUB
CREATE C AS QBITMAP
  Font.Name = "Arial"
END CREATE
PRINT C.Font.Name
Intro
