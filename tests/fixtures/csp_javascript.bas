' tests/web_bundle_csp.mjs (docs/security-audit.md SEC-15): a program that
' uses RJAVASCRIPT gets 'unsafe-eval' in its page's policy, so Eval works.
$APPTYPE WEB

CREATE JS AS RJAVASCRIPT
END CREATE

CREATE Form AS QFORM
  Caption = "js"
  Width = 320
  Height = 120
  CREATE Lbl AS QLABEL
    Left = 8
    Top = 8
    Width = 280
    Caption = "-"
  END CREATE
END CREATE

Lbl.Caption = "eval " + STR$(JS.Eval("20 + 22"))
Form.ShowModal
