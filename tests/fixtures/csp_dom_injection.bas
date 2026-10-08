' tests/web_bundle_csp.mjs (docs/security-audit.md SEC-15): a web program
' that shows data it didn't write (here a "server reply") as markup through
' RDOM.InnerHTML. The bundle's Content-Security-Policy stops the markup's
' script: its picture's onerror never runs.
$APPTYPE WEB

CREATE Form AS QFORM
  Caption = "csp"
  Width = 320
  Height = 160
  CREATE Lbl AS QLABEL
    Left = 8
    Top = 8
    Width = 280
    Caption = "-"
  END CREATE
END CREATE

DIM reply AS STRING
reply = "<b id='shown'>reply</b><img src='nothing.png' onerror='window.__pwned = 1'>"

CREATE Box AS RDOM
  TagName = "div"
  Left = 10
  Top = 400
  Width = 200
  Height = 40
END CREATE
Box.InnerHTML = reply
Lbl.Caption = "shown"
Form.ShowModal
