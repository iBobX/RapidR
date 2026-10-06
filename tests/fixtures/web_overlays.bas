' The web-only components on the UI kernel's page (tests/web_overlays.mjs,
' docs/web-host-plan.md §3.6): an RWEBVIEW, RDOM elements and an
' RWEBVIDEO are the page's own elements, placed by the kernel over the
' form's canvas; a drop-down list opens over them. (An RPLOT is the
' kernel's own component: tests/fixtures/rplot_on_form.bas.)
$APPTYPE WEB

CREATE Form AS RFORM
  Caption = "overlays"
  Width = 520
  Height = 420
  CREATE Lbl AS RLABEL
    Left = 8
    Top = 8
    Width = 300
    Caption = "-"
  END CREATE
  CREATE Combo AS RCOMBOBOX
    Style = 2
    Left = 320
    Top = 4
    Width = 120
    AddItems "one", "two", "three", "four", "five", "six"
  END CREATE
  CREATE Move AS RBUTTON
    Left = 450
    Top = 4
    Width = 60
    Caption = "Move"
    OnClick = MoveIt
  END CREATE
  CREATE Web AS RWEBVIEW
    Left = 10
    Top = 34
    Width = 240
    Height = 120
  END CREATE
  CREATE Box AS RPANEL
    Left = 260
    Top = 34
    Width = 120
    Height = 60
    CREATE Inner AS RDOM
      TagName = "div"
      Left = 10
      Top = 10
      Width = 200
      Height = 30
      InnerHTML = "<b>inside</b>"
      OnClick = DomClicked
    END CREATE
  END CREATE
  CREATE Video AS RWEBVIDEO
    Left = 10
    Top = 170
    Width = 160
    Height = 100
  END CREATE
END CREATE

CREATE Free AS RDOM
  TagName = "p"
  InnerText = "free element"
  Left = 700
  Top = 600
END CREATE

SUB MoveIt
  Web.Left = Web.Left + 30
  Web.Top = Web.Top + 5
  IF Video.Visible THEN Video.Visible = 0 ELSE Video.Visible = 1
END SUB

SUB DomClicked
  Lbl.Caption = "dom clicked " + Inner.InnerText
END SUB

Web.SetHtml("<html><body><p id='p'>hello from the frame</p></body></html>")
Form.ShowModal
