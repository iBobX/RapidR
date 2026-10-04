' Double clicks in the VCL's order (Windows' WM_LBUTTONDBLCLK, Delphi's
' TControl): the first press is OnMouseDown, its release OnClick then
' OnMouseUp; the second press is OnDblClick then OnMouseDown, its release
' only OnMouseUp — on a QPANEL, a QLABEL, a QGROUPBOX, a QIMAGE and the
' form's open area. A QCANVAS has no OnDblClick in RapidQ ("does not
' work"): each of its clicks is a click.
DECLARE SUB Note (S AS STRING)
DECLARE SUB PnDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB PnUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB PnClick
DECLARE SUB PnDbl
DECLARE SUB LbDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB LbUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB LbClick
DECLARE SUB LbDbl
DECLARE SUB GbDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB GbUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB GbClick
DECLARE SUB GbDbl
DECLARE SUB ImgDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB ImgUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB ImgClick
DECLARE SUB ImgDbl
DECLARE SUB CvDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB CvUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB CvClick
DECLARE SUB FormDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB FormUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB FormClick
DECLARE SUB FormDbl
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "double clicks"
  Width = 420
  Height = 320
  OnMouseDown = FormDown
  OnMouseUp = FormUp
  OnClick = FormClick
  OnDblClick = FormDbl
  CREATE Pn AS QPANEL
    Left = 5: Top = 5: Width = 90: Height = 60
    OnMouseDown = PnDown: OnMouseUp = PnUp: OnClick = PnClick: OnDblClick = PnDbl
  END CREATE
  CREATE Lb AS QLABEL
    Left = 105: Top = 5: Width = 90: Height = 20: Caption = "label"
    OnMouseDown = LbDown: OnMouseUp = LbUp: OnClick = LbClick: OnDblClick = LbDbl
  END CREATE
  CREATE Gb AS QGROUPBOX
    Left = 205: Top = 5: Width = 90: Height = 60: Caption = "group"
    OnMouseDown = GbDown: OnMouseUp = GbUp: OnClick = GbClick: OnDblClick = GbDbl
  END CREATE
  CREATE Img AS QIMAGE
    Left = 5: Top = 80: Width = 90: Height = 60
    OnMouseDown = ImgDown: OnMouseUp = ImgUp: OnClick = ImgClick: OnDblClick = ImgDbl
  END CREATE
  CREATE Cv AS QCANVAS
    Left = 105: Top = 80: Width = 90: Height = 60
    OnMouseDown = CvDown: OnMouseUp = CvUp: OnClick = CvClick
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5: Top = 160: Width = 400: Caption = "-"
  END CREATE
END CREATE
Form.ShowModal

SUB Note (S AS STRING)
  Log = Log + S: Lbl.Caption = Log
END SUB
SUB PnDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "pd" + STR$(X)
END SUB
SUB PnUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "pu "
END SUB
SUB PnClick
  Note "pc"
END SUB
SUB PnDbl
  Note "pD"
END SUB
SUB LbDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "ld"
END SUB
SUB LbUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "lu "
END SUB
SUB LbClick
  Note "lc"
END SUB
SUB LbDbl
  Note "lD"
END SUB
SUB GbDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "gd"
END SUB
SUB GbUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "gu "
END SUB
SUB GbClick
  Note "gc"
END SUB
SUB GbDbl
  Note "gD"
END SUB
SUB ImgDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "id"
END SUB
SUB ImgUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "iu "
END SUB
SUB ImgClick
  Note "ic"
END SUB
SUB ImgDbl
  Note "iD"
END SUB
SUB CvDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "cd"
END SUB
SUB CvUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "cu "
END SUB
SUB CvClick
  Note "cc"
END SUB
SUB FormDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "fd"
END SUB
SUB FormUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "fu "
END SUB
SUB FormClick
  Note "fc"
END SUB
SUB FormDbl
  Note "fD"
END SUB
