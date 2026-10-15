' A QTRACKBAR turned (Orientation 0 to 1 or back) swaps its Width and Height,
' as RapidQ's does (RapidQ's GMSynth sets 160 x 40, then Orientation = 1: a
' tall slider); setting the Orientation it has changes nothing. The .expected
' is RC.EXE's output.
$APPTYPE CONSOLE
CREATE Form AS QFORM
  CREATE T AS QTRACKBAR
    Left = 7: Top = 50: Width = 160: Height = 40: Min = 0: Max = 127: Position = 127: Orientation = 1
  END CREATE
  CREATE T2 AS QTRACKBAR
    Orientation = 1
  END CREATE
END CREATE
PRINT T.Width; " "; T.Height; " "; T.Orientation
PRINT T2.Width; " "; T2.Height
T2.Orientation = 1
PRINT T2.Width; " "; T2.Height
T2.Orientation = 0
PRINT T2.Width; " "; T2.Height
