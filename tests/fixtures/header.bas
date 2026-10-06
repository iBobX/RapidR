' QHEADER: AddSections, Sections(i)
' properties, a click, a drag of a section's edge (OnSectionTrack's
' begin / move / end, then OnSectionResize), and an owner-drawn section
' drawn by OnDrawSection on the header itself.
$INCLUDE "RAPIDQ.INC"
DECLARE SUB DrawSection (Index AS INTEGER, Pressed AS INTEGER, Rect AS QRECT, Sender AS QHEADER)
DECLARE SUB SectionTrack (Index AS INTEGER, Width AS INTEGER, State AS INTEGER, Sender AS QHEADER)
DECLARE SUB SectionClick (Index AS INTEGER, Sender AS QHEADER)
DECLARE SUB SectionResize (Index AS INTEGER, Sender AS QHEADER)
DECLARE SUB Report
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "QHeader"
  Width = 360
  Height = 200
  CREATE Header AS QHEADER
    Width = 300
    AddSections "Name", "Size", "Chart"
    Sections(0).Width = 100
    Sections(1).AllowClick = 0
    Sections(2).Style = hsOwnerDraw
    OnDrawSection = DrawSection
    OnSectionTrack = SectionTrack
    OnSectionClick = SectionClick
    OnSectionResize = SectionResize
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 5 : Top = 40
    Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5 : Top = 80 : Width = 340
    Caption = "-"
  END CREATE
END CREATE

SUB DrawSection (Index AS INTEGER, Pressed AS INTEGER, Rect AS QRECT, Sender AS QHEADER)
  Sender.FillRect(Rect.Left + 5, Rect.Top + 3, Rect.Right - 5, Rect.Bottom - 3, &H00FF00)
END SUB

SUB SectionTrack (Index AS INTEGER, Width AS INTEGER, State AS INTEGER, Sender AS QHEADER)
  Log = Log + "t" + STR$(Index) + ":" + STR$(Width) + ":" + STR$(State) + " "
END SUB

SUB SectionClick (Index AS INTEGER, Sender AS QHEADER)
  Log = Log + "c" + STR$(Index) + " "
END SUB

SUB SectionResize (Index AS INTEGER, Sender AS QHEADER)
  Log = Log + "r" + STR$(Index) + " "
END SUB

SUB Report
  Lbl.Caption = Log + "| " + STR$(Header.SectionsCount) + " " + STR$(Header.Sections(0).Width) + " " + Header.Sections(2).Caption + " " + HEX$(Header.Pixel(215, 10))
END SUB

Form.ShowModal
