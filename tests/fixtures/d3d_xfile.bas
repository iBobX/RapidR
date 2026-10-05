' Direct3D Retained Mode (docs/directx-plan.md, stages D3-D4): a DirectX .X
' model (d3d_model.x, with its texture d3d_tex.bmp from
' tools/make_dx_fixture.py; both extracted from $RESOURCEs) loaded into a
' QD3DMESHBUILDER — its frame's matrix, a referenced material's colour, a
' texture on one face only — the faces lit by a white ambient light (their
' own colours exactly); then MeshBuilder.SetRGB (the texture modulated by
' it) and SetTexture (now on every face). Checked natively and interpreted by
' tests/native_gui_events.mjs, in the browser by tests/web_gui_parity.mjs.
$INCLUDE "RAPIDQ.INC"
$RESOURCE MODEL_X AS "d3d_model.x"
$RESOURCE TEX_BMP AS "d3d_tex.bmp"
CONST D3DRMLIGHT_AMBIENT = 0
DECLARE SUB Setup
DECLARE SUB Paint
DIM log AS STRING
DIM MeshFrame AS QD3DFRAME
DIM MB AS QD3DMESHBUILDER
CREATE Form AS QFORM
  Caption = "d3d xfile"
  ClientWidth = 160
  ClientHeight = 180
  CREATE DX AS QDXSCREEN
    Left = 0
    Top = 0
    Width = 160
    Height = 120
    Init(160, 120)
    Use3D = 1
    OnInitialize = Setup
  END CREATE
  CREATE b1 AS QBUTTON
    Left = 0
    Top = 125
    Caption = "Paint"
    OnClick = Paint
  END CREATE
  CREATE lbl AS QLABEL
    Left = 0
    Top = 152
    Width = 160
    Caption = "-"
  END CREATE
END CREATE

SUB Setup
  DIM Ambient AS QD3DLIGHT
  EXTRACTRESOURCE MODEL_X, "d3d_model.x"
  EXTRACTRESOURCE TEX_BMP, "d3d_tex.bmp"
  DX.CreateFrame(MeshFrame)
  DX.CreateMeshBuilder(MB)
  DX.CreateLightRGB(D3DRMLIGHT_AMBIENT, 1, 1, 1, Ambient)
  DX.AddLight(Ambient)
  MB.Load("d3d_model.x")
  MeshFrame.AddVisual(MB)
  MeshFrame.SetPosition(0, 0, 6)
  DX.Render
  DX.Flip
  ' the red square (left), the texture's blue and yellow (right), between
  log = "faces" + STR$(MB.FaceCount) + " verts" + STR$(MB.VertexCount) + " " + STR$(DX.Pixel(40, 60)) + "," + STR$(DX.Pixel(105, 60)) + "," + STR$(DX.Pixel(135, 60)) + "," + STR$(DX.Pixel(80, 60)) + " "
END SUB

SUB Paint
  DIM Tex AS QD3DTEXTURE
  ' every face green: the yellow texel modulated by it, green
  MB.SetRGB(0, 1, 0)
  DX.Render
  log = log + "green" + STR$(DX.Pixel(40, 60)) + "," + STR$(DX.Pixel(135, 60)) + " "
  ' the texture on both squares, white again
  DX.LoadTexture("d3d_tex.bmp", Tex)
  MB.SetTexture(Tex)
  MB.SetRGB(1, 1, 1)
  DX.Render
  DX.Flip
  lbl.Caption = log + "tex" + STR$(DX.Pixel(20, 60)) + "," + STR$(DX.Pixel(60, 60))
  KILL "d3d_model.x"
  KILL "d3d_tex.bmp"
END SUB

Form.ShowModal
