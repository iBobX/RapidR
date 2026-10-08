' examples/gui/hello_form.rr's window as RapidQ writes it, and a line of
' every lower-case letter and digit: the letters' sizes and the gaps
' between them in RapidQ's default font (RapidR Sans).
CREATE Form AS QFORM
  Caption = "Hello, RapidR": Left = 40: Top = 40: Width = 340: Height = 190
  CREATE NameLabel AS QLABEL
    Caption = "Your &name:": Left = 16: Top = 20: Width = 92
  END CREATE
  CREATE NameEdit AS QEDIT
    Text = "World": Left = 112: Top = 16: Width = 200
  END CREATE
  CREATE GreetButton AS QBUTTON
    Caption = "&Greet": Left = 112: Top = 52: Width = 90: Default = 1
  END CREATE
  CREATE Answer AS QLABEL
    Caption = "Type a name, then Greet.": Left = 16: Top = 96: Width = 300
  END CREATE
  CREATE Fox AS QLABEL
    Caption = "The quick brown fox jumps over the lazy dog 0123456789": Left = 16: Top = 120: Width = 310
  END CREATE
  CREATE Caps AS QLABEL
    Caption = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG": Left = 16: Top = 140: Width = 310
  END CREATE
END CREATE
Form.ShowModal
