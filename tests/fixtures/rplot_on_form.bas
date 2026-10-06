' RPLOT on a form: a UI-kernel component that draws its chart (the one
' chart renderer, rapidr_value::datascience::chart) in its rectangle, the
' same pixels natively, interpreted and in the browser
' (tests/gui_parity_cases.mjs' rplot_on_form). A line chart anchored left,
' top and right (it stretches when the user widens the form), a bar chart
' aligned to the bottom; a click adds a series and a legend, titles the
' bars and renders: drawn again. (An RPLOT never placed on a form is only
' saved or loaded into a picture: tests/conformance/cases/datascience_*.)
DECLARE SUB More
' (RAPIDQ.INC's)
CONST alBottom = 2
CREATE Form AS QFORM
  Caption = "Plots": Width = 420: Height = 350
  CREATE Lbl AS QLABEL
    Left = 8: Top = 6: Width = 310: Caption = "-"
  END CREATE
  CREATE Btn AS QBUTTON
    Caption = "More": Left = 330: Top = 2: Width = 80: Height = 24
    OnClick = More
  END CREATE
  CREATE Plot1 AS RPLOT
    Left = 8: Top = 30: Width = 250: Height = 170
    Anchors = akLeft + akTop + akRight
    Title = "Squares"
    XLabel = "n"
  END CREATE
  CREATE Plot2 AS RPLOT
    Align = alBottom: Height = 110
  END CREATE
END CREATE

DIM xs(4) AS DOUBLE
DIM ys(4) AS DOUBLE
FOR i = 0 TO 4
  xs(i) = i
  ys(i) = i * i
NEXT
Plot1.Plot(xs, ys, "squares", "red")
Plot2.Bar("North,South,East,West", "12,15,9,18", "units", "steelblue")
Plot1.Render

SUB More
  Plot1.Plot("0,1,2,3,4", "0,4,8,12,16", "linear", "blue", "--")
  Plot1.Legend
  Plot2.Title = "Sales"
  Plot1.Show
  Lbl.Caption = STR$(Plot1.SeriesCount) + " " + STR$(Plot1.Width) + "x" + STR$(Plot1.Height) + " " + STR$(Plot2.Left) + "," + STR$(Plot2.Top) + " " + STR$(Plot2.Width) + "x" + STR$(Plot2.Height) + " " + Plot2.Title + " " + STR$(Plot1.Visible)
END SUB

Form.ShowModal
