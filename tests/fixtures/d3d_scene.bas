' Direct3D Retained Mode (docs/directx-plan.md, stages D3-D4): a scene on a
' QDXSCREEN — frames, a mesh builder of two faces made into the same
' QD3DFACE variable (each keeps its own), an ambient and a directional light
' (on a frame), the camera; Render draws it into the back buffer (Pixel
' reads it), Flip shows it; SetRotation + Move turn the mesh's frame (the
' face then catches less light), CameraLookAt. Checked natively and
' interpreted by tests/native_gui_events.mjs, in the browser by
' tests/web_gui_parity.mjs.
$INCLUDE "RAPIDQ.INC"
CONST D3DRMLIGHT_AMBIENT = 0
CONST D3DRMLIGHT_DIRECTIONAL = 3
CONST D3DRMRENDER_FLAT = 136
DECLARE SUB Setup
DECLARE SUB Turn
DECLARE SUB Report
DIM log AS STRING
DIM MeshFrame AS QD3DFRAME
DIM LightFrame AS QD3DFRAME
DIM MB AS QD3DMESHBUILDER
CREATE Form AS QFORM
  Caption = "d3d scene"
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
    Caption = "Turn"
    OnClick = Turn
  END CREATE
  CREATE b2 AS QBUTTON
    Left = 80
    Top = 125
    Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE lbl AS QLABEL
    Left = 0
    Top = 152
    Width = 160
    Caption = "-"
  END CREATE
END CREATE

SUB Setup
  DIM Face AS QD3DFACE
  DIM Light AS QD3DLIGHT
  DIM Ambient AS QD3DLIGHT
  DX.CreateFrame(MeshFrame)
  DX.CreateFrame(LightFrame)
  DX.CreateMeshBuilder(MB)
  DX.CreateLightRGB(D3DRMLIGHT_AMBIENT, 0.2, 0.2, 0.2, Ambient)
  DX.AddLight(Ambient)
  DX.CreateLightRGB(D3DRMLIGHT_DIRECTIONAL, 0.8, 0.8, 0.8, Light)
  LightFrame.AddLight(Light)
  ' A red square facing the camera (clockwise as it sees it: its front) ...
  DX.CreateFace(Face)
  Face.AddVertex(-1, 1, 0)
  Face.AddVertex(1, 1, 0)
  Face.AddVertex(1, -1, 0)
  Face.AddVertex(-1, -1, 0)
  Face.SetColorRGB(1, 0, 0)
  MB.AddFace(Face)
  ' ... and a green one to its right, made into the same variable.
  DX.CreateFace(Face)
  Face.AddVertex(1.5, 1, 0)
  Face.AddVertex(2.5, 1, 0)
  Face.AddVertex(2.5, -1, 0)
  Face.AddVertex(1.5, -1, 0)
  Face.SetColorRGB(0, 1, 0)
  MB.AddFace(Face)
  MB.SetQuality(D3DRMRENDER_FLAT)
  MeshFrame.AddVisual(MB)
  MeshFrame.SetPosition(0, 0, 6)
  DX.SetCameraPosition(0, 0, 0)
  DX.Render
  DX.Flip
  log = "faces" + STR$(MB.FaceCount) + " lit" + STR$(DX.Pixel(80, 60)) + "," + STR$(DX.Pixel(130, 60)) + "," + STR$(DX.Pixel(5, 5)) + " "
END SUB

SUB Turn
  ' half a radian about y, one Move step: the faces turn from the light
  MeshFrame.SetRotation(0, 1, 0, 0.5)
  DX.Move(1)
  MeshFrame.SetRotation(0, 0, 0, 0)
  DX.Render
  DX.Flip
  log = log + "turned" + STR$(DX.Pixel(80, 60)) + " "
END SUB

SUB Report
  ' the camera moved to the left, looking at the mesh's frame
  DX.SetCameraPosition(-6, 0, 6)
  DX.CameraLookAt(MeshFrame, 0)
  DX.Render
  lbl.Caption = log + "look" + STR$(DX.Pixel(80, 60)) + "," + STR$(DX.Pixel(5, 5))
  DX.Flip
END SUB

Form.ShowModal
