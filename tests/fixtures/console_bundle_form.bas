' A program with a form that also PRINTs (tests/web_bundle_console.mjs).
CREATE Form AS QFORM
  Caption = "With a form"
END CREATE
PRINT "log line"
Form.ShowModal
