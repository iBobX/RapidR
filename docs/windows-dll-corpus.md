# RapidQ's DLL-calling examples on Windows and macOS

Every program of RapidQ's examples (`~/Downloads/Rapidq/examples`) that
declares DLL routines — 175, `tools/dll_corpus_list.py` — built and run by
`tools/dll_breadth.py`, 2026-10-08 (C-SYS-2; what the results mean:
[windows-dll-calls.md](windows-dll-calls.md) §6).

- **Windows**: the Windows 11 VM (ARM64), each program interpreted
  (`rapidr build-bc` + `run-bc`) and as a native build, run three seconds
  with its windows captured. *Before*: the design as the previous pass
  left it (with this pass's first fixes to the call table); *after*: this
  pass.
- **macOS**: interpreted, the same three seconds.
- "runs": no error within the three seconds (the program waits for its
  user, or ends); "builds (not run: …)": it would print, write the
  registry, install fonts, start programs, inject keystrokes or shut the
  machine down, so it was only built; "compile: …" the first compile
  error (the same on every system).

| Program | Windows before (interp) | Windows after (interp) | Windows after (native) | macOS (interp) |
|---|---|---|---|---|
| AVI/VideoPlayer.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| AVI/animateAVI.bas | compile: SENDMESSAGE (a RapidQ built-in) isn't supported yet | runs | runs | stops: needs Windows |
| AVI/aviplay.bas | compile: Invalid $RESOURCE syntax: $RESOURCE PLAYBMP AS <PLAY.BMP> | compile: Invalid $RESOURCE syntax: $RESOURCE PLAYBMP AS <PLAY.BMP> | compile: Invalid $RESOURCE syntax: $RESOURCE PLAYBMP AS <PLAY.BMP> | compile error |
| AVI/testAVItoBMP.bas | runtime-error: crash in a DLL | runtime-error: crash in a DLL | runtime-error: crash in a DLL | stops: needs Windows |
| Calendar/ApiDate.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| Calendar/qcalendarTEST.bas | runs | runs | native build: error[E0282]: type annotations needed | runs (no Windows call in 3 s) |
| CallBacks/NewWndProcDemo.Bas | runtime-error: memory a DLL returned used as the program's | runtime-error: memory a DLL returned used as the program's | runtime-error: memory a DLL returned used as the program's | stops: needs Windows |
| CallBacks/OverIncludingTest.Bas | compile: Syntax error: unexpected '.' | compile: Syntax error: unexpected '.' | compile: Syntax error: unexpected '.' | compile error |
| CallBacks/TestNoMouse.Bas | runtime-error: memory a DLL returned used as the program's | runtime-error: memory a DLL returned used as the program's | runtime-error: memory a DLL returned used as the program's | stops: needs Windows |
| ComPort/ComPort_example.bas | runs | runs | runs | stops: needs Windows |
| Compile/CodeBox.bas | compile: Invalid $RESOURCE syntax: Please check your original-code. Y | compile: Invalid $RESOURCE syntax: Please check your original-code. Y | compile: Invalid $RESOURCE syntax: Please check your original-code. Y | compile error |
| Compile/RunTime/RunTimeSimpl.bas | compile: Include file not found: 'eMail.Inc' | compile: Include file not found: 'eMail.Inc' | compile: Include file not found: 'eMail.Inc' | compile error |
| Compile/rapidq_boosta.bas | compile: Unknown SUB or FUNCTION 'RQ_ExecuteFile' | compile: Unknown SUB or FUNCTION 'RQ_ExecuteFile' | compile: Unknown SUB or FUNCTION 'RQ_ExecuteFile' | compile error |
| Database/QODBC.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| Database/SQL_blobs/archive.bas | runs | runs | native build: error: could not compile `__014_archive` (bin "__014_archive | stops: needs Windows |
| Database/SQL_blobs/hexconvert.bas | compile: Expected end-of-line but got ptr | compile: Expected end-of-line but got ptr | compile: Expected end-of-line but got ptr | compile error |
| Database/baseGG/baseGG3.bas | builds (not run: prints) | builds (not run: prints) | native build: error: could not compile `__016_baseGG3` (bin "__016_baseGG3 | builds |
| Database/mdb/1-CreateDB.bas | runs | runs | runs | stops: needs Windows |
| Database/qDB2.bas | runs | runs | runs | stops: needs Windows |
| Fonts/add & remove font.bas | builds (not run: installs fonts) | builds (not run: installs fonts) | builds (not run: installs fonts) | builds |
| IPicture/RQ_IPictureDemo.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| IPicture/RQ_IPictureDemo2.bas | runs | runs | runs | stops: needs Windows |
| Math/ProbCalc.bas | compile: Include file not found: 'C:\rapidq\qstat\ProbDists.inc' | compile: Include file not found: 'C:\rapidq\qstat\ProbDists.inc' | compile: Include file not found: 'C:\rapidq\qstat\ProbDists.inc' | compile error |
| Mouse/MoveMouse.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| Network/BlockingQSocket.bas | runs | runs | runs | stops: needs Windows |
| Network/InetIsOffline.bas | runs | runs | runs | stops: needs Windows |
| Network/Internet Connection.bas | compile: Syntax error in END statement | compile: Syntax error in END statement | compile: Syntax error in END statement | compile error |
| Network/QHTML/QHTML example.bas | compile: Syntax error in DIM statement | compile: Syntax error in DIM statement | compile: Syntax error in DIM statement | compile error |
| Network/QHTML/QHTMLexample.bas | compile: Syntax error in DIM statement | compile: Syntax error in DIM statement | compile: Syntax error in DIM statement | compile error |
| Network/Server_Client95/clientSocket.BAS | builds (not run: writes the registry) | builds (not run: writes the registry) | builds (not run: writes the registry) | builds |
| Network/Server_Client95/serverSocket.BAS | builds (not run: writes the registry) | builds (not run: writes the registry) | builds (not run: writes the registry) | builds |
| Network/UDP.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| Network/URLDownload.bas | runs | runs | runs | stops: needs Windows |
| Network/bind port to IP.bas | runs | runs | runs | stops: needs Windows |
| Network/ftp/ChangeMyWebPage.Bas | compile: Syntax error near 'ShowMessage' | compile: Syntax error near 'ShowMessage' | compile: Syntax error near 'ShowMessage' | compile error |
| Network/ftp/QFTPCLIENT.BAS | builds (not run: deletes or moves files) | builds (not run: deletes or moves files) | builds (not run: deletes or moves files) | builds |
| Network/ftp/QFTPGUITEST.BAS | builds (not run: deletes or moves files) | builds (not run: deletes or moves files) | builds (not run: deletes or moves files) | builds |
| Network/ra1d.bas | compile: Expected end-of-line but got AS | compile: Expected end-of-line but got AS | compile: Expected end-of-line but got AS | compile error |
| OLE/IE_sim.bas | builds (not run: starts other programs) | builds (not run: starts other programs) | builds (not run: starts other programs) | builds |
| OLE/OLE_IExplorer_Demo.Bas | builds (not run: starts other programs) | builds (not run: starts other programs) | builds (not run: starts other programs) | builds |
| OpenGL/GL_test1.bas | runs | runs | runs | stops: needs Windows |
| OpenGL/GL_test2.bas | runs | runs | runs | stops: needs Windows |
| OpenGL/GL_test3.bas | runs | runs | runs | stops: needs Windows |
| OpenGL/GL_test4.bas | runs | runs | runs | stops: needs Windows |
| OpenGL/OGL_Demo1.bas | compile: Include file not found: 'gl.inc' | compile: Include file not found: 'gl.inc' | compile: Include file not found: 'gl.inc' | compile error |
| OpenGL/OGL_Demo2.bas | compile: Include file not found: 'glBMP.inc' | compile: Include file not found: 'glBMP.inc' | compile: Include file not found: 'glBMP.inc' | compile error |
| OpenGL/QGLobject.bas | compile: unsupported function call target | compile: unsupported function call target | compile: unsupported function call target | compile error |
| OpenGL/QGLobject2.bas | compile: unsupported function call target | compile: unsupported function call target | compile: unsupported function call target | compile error |
| OpenGL/QGLobject3.bas | compile: unsupported function call target | compile: unsupported function call target | compile: unsupported function call target | compile error |
| Printer/printer.BAS | builds (not run: prints) | builds (not run: prints) | builds (not run: prints) | builds |
| QCursorListBox/TestQCursorListBox.bas | runs | runs | runs | stops: needs Windows |
| QGrid/TestQGrid..bas | runs | runs | runs | stops: needs Windows |
| QIcon/$noname.bas | compile: Include file not found: 'INC\QFuncLib.inc' | compile: Include file not found: 'INC\QFuncLib.inc' | compile: Include file not found: 'INC\QFuncLib.inc' | compile error |
| QTabControl/QTabContol_add_del.bas | runs | runs | native build: error: could not compile `__053_QTabContol_add_del` (bin "__ | stops: needs Windows |
| QTabControl/TabIcon/Tab_Icon_Demo.bas | runs | runs | runs | stops: needs Windows |
| QToolbar/QReBar/ReBarTest.bas | compile: SENDMESSAGE (a RapidQ built-in) isn't supported yet | builds (not run: deletes or moves files) | builds (not run: deletes or moves files) | builds |
| QToolbar/QToolBar2/TestQToolBar.bas | runs | runs | runs | stops: needs Windows |
| QToolbar/QToolBarEx/MyToolBar.bas | builds (not run: deletes or moves files) | builds (not run: deletes or moves files) | builds (not run: deletes or moves files) | builds |
| System/CreateShellLink.bas | runtime-error: DLL not there | runtime-error: DLL not there | runtime-error: DLL not there | stops: needs Windows |
| System/SPYINFO3A.bas | compile: SENDMESSAGE (a RapidQ built-in) isn't supported yet | runs | runs | stops: needs Windows |
| System/ShellExecute.bas | builds (not run: starts other programs) | builds (not run: starts other programs) | builds (not run: starts other programs) | builds |
| System/computerOff/ComputerOff.BAS | builds (not run: shuts down / locks / suspends the machine) | builds (not run: shuts down / locks / suspends the machine) | builds (not run: shuts down / locks / suspends the machine) | builds |
| System/showhide.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| System/shutdownTest.bas | builds (not run: shuts down / locks / suspends the machine) | builds (not run: shuts down / locks / suspends the machine) | builds (not run: shuts down / locks / suspends the machine) | builds |
| System/thread.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| VideoCapture/DirectShow_test.bas | compile: Unexpected character: ! | compile: Unexpected character: ! | compile: Unexpected character: ! | compile error |
| VideoCapture/Direct_Show_ex.bas | compile: Expected end-of-line but got * | compile: Expected end-of-line but got * | compile: Expected end-of-line but got * | compile error |
| VideoCapture/VideoCapture.bas | runs | runs | runs | stops: needs Windows |
| VideoCapture/WebCam2.BAS | compile: Include file not found: 'rapidq2_2.inc' | compile: Include file not found: 'rapidq2_2.inc' | compile: Include file not found: 'rapidq2_2.inc' | compile error |
| asm/Nasm/Bin_To_Inc/Demo/ReverseStringDemo_1.Bas | runtime-error: crash in a DLL | runtime-error: x86 machine code | runtime-error: x86 machine code | stops: needs Windows |
| asm/Nasm/Bin_To_Inc/RQ_Sources/BinToInc.Bas | runs | runs | runs | stops: needs Windows |
| asm/Nasm/Bin_To_Inc/RQ_Sources/BinToInc_Plus.Bas | runs | runs | runs | stops: needs Windows |
| asm/TestRqAsmUtils.Bas | runtime-error: crash in a DLL | runtime-error: x86 machine code | runtime-error: x86 machine code | stops: needs Windows |
| asm/TestRqUtilsDll.Bas | compile: Syntax error in DEFSTR statement | compile: Syntax error in DEFSTR statement | compile: Syntax error in DEFSTR statement | compile error |
| bmp/BMPClipboard.bas | compile: Syntax error in END statement | compile: Syntax error in END statement | compile: Syntax error in END statement | compile error |
| bmp/ReScaleBMP.bas | compile: $RESOURCE jpeg_DLL: file not found: 'jpeg.dll' | compile: $RESOURCE jpeg_DLL: file not found: 'jpeg.dll' | compile: $RESOURCE jpeg_DLL: file not found: 'jpeg.dll' | compile error |
| bmp/bmpdll/bmpdll.bas | runs | runs | runs | stops: needs Windows |
| buttons/ButtonBar.bas | builds (not run: starts other programs) | builds (not run: starts other programs) | builds (not run: starts other programs) | builds |
| buttons/ColorButton.bas | runtime-error: callback (CODEPTR to a DLL) | runtime-error: callback (CODEPTR to a DLL) | waits past 3 s | stops: needs Windows |
| buttons/XPBtn.bas | compile: Syntax error in SUB statement | compile: Syntax error in SUB statement | compile: Syntax error in SUB statement | compile error |
| cursors/animated/cursors.bas | runs | runs | runs | stops: needs Windows |
| cursors/loadcur/newcur.bas | runs | runs | runs | stops: needs Windows |
| devices/joystick.bas | compile: Syntax error in IF statement | compile: Syntax error in IF statement | compile: Syntax error in IF statement | compile error |
| dialogs/colordlg/colordlg.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| dialogs/printdlg/printdlg.bas | runs | runs | runs | stops: needs Windows |
| direct3d/3DPong_aDelic2.bas | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile error |
| direct3d/3dConvert/xview2.bas | runs | runs | runs | stops: needs Windows |
| direct3d/3d_clock/3d_orologio.bas | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile error |
| direct3d/Lights_pyramid.bas | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile error |
| direct3d/RQ_3DTerrain.bas | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile error |
| direct3d/RQ_3DTerrain/Terrain16.bas | compile: Syntax error in END statement | compile: Syntax error in END statement | compile: Syntax error in END statement | compile error |
| direct3d/circularScreen/Circular3DScreen.bas | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile error |
| direct3d/lights_motion.bas | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile error |
| dll/GetDllsFuncList.Bas | runtime-error: crash in a DLL | runtime-error: x86 machine code | runtime-error: x86 machine code | stops: needs Windows |
| dll/simple/DLL.BAS | runtime-error: DLL not there | runtime-error: 32-bit DLL | runtime-error: 32-bit DLL | stops: needs Windows |
| files/DiskVolumeInfo.bas | runs | runs | runs | stops: needs Windows |
| files/FileSearch.bas | builds (not run: writes the registry) | builds (not run: writes the registry) | builds (not run: writes the registry) | builds |
| files/GetShortFileName.bas | runs | runs | runs | stops: needs Windows |
| files/QFolder/QFolder.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| forms/AlwaysOnTop.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| forms/IEWindowMaximizer.BAS | waits past 3 s | waits past 3 s | waits past 3 s | stops: needs Windows |
| forms/LockWin.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| forms/MenuBarColor.bas | runs | runs | runs | stops: needs Windows |
| forms/MinToTaskbar.bas | runtime-error: callback (CODEPTR to a DLL) | runtime-error: callback (CODEPTR to a DLL) | runtime-error: callback (CODEPTR to a DLL) | stops: needs Windows |
| forms/MultiCaptiveChildWnds.bas | compile: SENDMESSAGE (a RapidQ built-in) isn't supported yet | runs | runs | stops: needs Windows |
| forms/QTabControl/QTabVertical.bas | runtime-error: crash in a DLL | runtime-error: crash in a DLL | runtime-error: crash in a DLL | stops: needs Windows |
| forms/Qeform.bas | runs | runs | runs | stops: needs Windows |
| forms/QrForm.bas | crash: exit 1:  | runs | runs | stops: needs Windows |
| forms/Window Sizer.BAS | compile: Syntax error in ELSE statement | compile: Syntax error in ELSE statement | compile: Syntax error in ELSE statement | compile error |
| forms/deskform/deskform.bas | runs | runs | runs | stops: needs Windows |
| forms/dragdrop/dragdrop.bas | runs | runs | runs | stops: needs Windows |
| forms/findWinTest.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| forms/minimize to taskbar0.bas | runs | runs | runs | stops: needs Windows |
| forms/multform/multform.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| forms/previnst/previnst.bas | runs | runs | runs | stops: needs Windows |
| forms/purewindows/wnd.bas | waits past 3 s | waits past 3 s | waits past 3 s | stops: needs Windows |
| forms/setfocus.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| forms/titlebtn.bas | compile: SENDMESSAGE (a RapidQ built-in) isn't supported yet | runtime-error: callback (CODEPTR to a DLL) | runtime-error: callback (CODEPTR to a DLL) | stops: needs Windows |
| forms/wndproc/winex.bas | compile: SENDMESSAGE (a RapidQ built-in) isn't supported yet | runs | runs | stops: needs Windows |
| forms/wndproc/winexPanel.bas | compile: SENDMESSAGE (a RapidQ built-in) isn't supported yet | runs | runs | stops: needs Windows |
| games/WIP_asteroids3D.bas | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile: Datatype QRECT not supported in STRUCT | compile error |
| games/eye_game.bas | compile: $RESOURCE steen0: file not found: 'leeg.bmp' | compile: $RESOURCE steen0: file not found: 'leeg.bmp' | compile: $RESOURCE steen0: file not found: 'leeg.bmp' | compile error |
| graphics/AlphaTest/AlphaTest1.bas | compile: Include file not found: 'XpGroupBox.inc' | compile: Include file not found: 'XpGroupBox.inc' | compile: Include file not found: 'XpGroupBox.inc' | compile error |
| graphics/AlphaTest/AlphaTest2.bas | runtime-error: crash in a DLL | runtime-error: x86 machine code | runtime-error: x86 machine code | stops: needs Windows |
| graphics/Choosecolor.bas | runs | runs | runs | stops: needs Windows |
| graphics/GDI_p.bas | runs | runs | runs | stops: needs Windows |
| graphics/Gamma.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| graphics/LoadJPG.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| graphics/Picview.bas | compile: Expected end-of-line but got , | compile: Expected end-of-line but got , | compile: Expected end-of-line but got , | compile error |
| graphics/PolyDrawLineTo.bas | builds (not run: prints) | builds (not run: prints) | builds (not run: prints) | builds |
| graphics/bitblt.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| graphics/jpeg_Blend.bas | runtime-error: this variable does not refer to an object (setting a field of "TmpBMP" | runtime-error: this variable does not refer to an object (setting a field of "TmpBMP" | native build: error: could not compile `__131_jpeg_Blend` (bin "__131_jpeg | stops: needs Windows |
| grids/ManageGridEditing.bas | compile: KILLMESSAGE (a RapidQ built-in) isn't supported yet | runtime-error: memory a DLL returned used as the program's | runtime-error: memory a DLL returned used as the program's | stops: needs Windows |
| grids/QGridEx/QGridExDemo.bas | compile: Include file not found: 'QDebug2.inc' | compile: Include file not found: 'QDebug2.inc' | compile: Include file not found: 'QDebug2.inc' | compile error |
| grids/QStringGridsTwoLinesBitMap.Bas | runs | runs | native build: error[E0425]: cannot find function `onselectcell_griddemo` i | runs (no Windows call in 3 s) |
| grids/datagrid.BAS | runs | runs | runs | runs (no Windows call in 3 s) |
| html/QHTML/QHTML example.bas | compile: Syntax error in DIM statement | compile: Syntax error in DIM statement | compile: Syntax error in DIM statement | compile error |
| html/QHTML/QHTMLexample.bas | compile: Syntax error in DIM statement | compile: Syntax error in DIM statement | compile: Syntax error in DIM statement | compile error |
| io/IO_RQ.bas | runtime-error: DLL not there | runtime-error: DLL not there | runtime-error: DLL not there | stops: needs Windows |
| keyboard/KEYMAP1a.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| keyboard/hook.bas | compile: Unknown SUB or FUNCTION 'that' | compile: Unknown SUB or FUNCTION 'that' | compile: Unknown SUB or FUNCTION 'that' | compile error |
| keyboard/hotkeys/hotkeys.bas | runs | runs | runs | stops: needs Windows |
| keyboard/sendkeys_to_app.bas | builds (not run: injects or hooks input) | builds (not run: injects or hooks input) | builds (not run: injects or hooks input) | builds |
| menu/ownerdraw/menu.bas | runs | runs | runs | stops: needs Windows |
| mysql/SQLServerTutorial/Taxes.bas | compile: Invalid $INCLUDE syntax: $INCLUDE "mysql.inc | compile: Invalid $INCLUDE syntax: $INCLUDE "mysql.inc | compile: Invalid $INCLUDE syntax: $INCLUDE "mysql.inc | compile error |
| mysql/rqlibsql/MySQL_Qlistview.bas | compile: Include file not found: 'qodbc.bas' | compile: Include file not found: 'qodbc.bas' | compile: Include file not found: 'qodbc.bas' | compile error |
| reminder/addrbook.bas | builds (not run: deletes or moves files) | builds (not run: deletes or moves files) | builds (not run: deletes or moves files) | builds |
| reminder/reminder.bas | compile: Undefined symbol GWL_HWNDPARENT | compile: Undefined symbol GWL_HWNDPARENT | compile: Undefined symbol GWL_HWNDPARENT | compile error |
| richedit/FindReplace.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| richedit/QRedExDemo.Bas | runs | runs | runs | stops: needs Windows |
| richedit/SearchRich.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| sockets/BlockingQSocket.bas | runs | runs | runs | stops: needs Windows |
| sockets/FD_File_Receiver.bas | runs | runs | runs | stops: needs Windows |
| sockets/FD_File_Sender.bas | runs | runs | runs | stops: needs Windows |
| sockets/RQ_Ychat.bas | compile: Include file not found: 'QRichEdit.inc' | compile: Include file not found: 'QRichEdit.inc' | compile: Include file not found: 'QRichEdit.inc' | compile error |
| sockets/UDP.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| sockets/bind port to IP.bas | runs | runs | runs | stops: needs Windows |
| sockets/chat/chatserv_test.bas | runs | runs | runs | stops: needs Windows |
| sound/midi/playmidi.bas | waits past 3 s | waits past 3 s | waits past 3 s | stops: needs Windows |
| sound/record.bas | waits past 3 s | waits past 3 s | waits past 3 s | stops: needs Windows |
| sound/soundgenerator.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| thread/Pipe.BAS | runtime-error: crash in a DLL | runtime-error: crash in a DLL | runtime-error: crash in a DLL | stops: needs Windows |
| thread/RQprocess/testProcessInfo.bas | builds (not run: starts other programs) | builds (not run: starts other programs) | builds (not run: starts other programs) | builds |
| thread/RQprocess/testProcessLview.bas | builds (not run: starts other programs) | builds (not run: starts other programs) | builds (not run: starts other programs) | builds |
| thread/SPYINFO3A.bas | compile: SENDMESSAGE (a RapidQ built-in) isn't supported yet | runs | runs | stops: needs Windows |
| thread/SendKeys.bas | compile: SENDMESSAGE (a RapidQ built-in) isn't supported yet | builds (not run: injects or hooks input) | builds (not run: injects or hooks input) | builds |
| thread/Talk_between_Windows.bas | compile: Syntax error: unexpected '-' | compile: Syntax error: unexpected '-' | compile: Syntax error: unexpected '-' | compile error |
| thread/previnst.bas | runs | runs | runs | stops: needs Windows |
| thread/sendMessage2.bas | compile: KILLMESSAGE (a RapidQ built-in) isn't supported yet | runs | runs | stops: needs Windows |
| thread/sendkeys_to_app.bas | builds (not run: injects or hooks input) | builds (not run: injects or hooks input) | builds (not run: injects or hooks input) | builds |
| thread/showhide.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| thread/thread1.bas | runs | runs | runs | runs (no Windows call in 3 s) |
| weatherQ/WeatherQ.bas | builds (not run: deletes or moves files) | builds (not run: deletes or moves files) | builds (not run: deletes or moves files) | builds |
| zlib/z_test.bas | compile: Syntax error in DECLARE statement | compile: Syntax error in DECLARE statement | compile: Syntax error in DECLARE statement | compile error |
| zlib/zunzip.bas | compile: Syntax error in DECLARE statement | compile: Syntax error in DECLARE statement | compile: Syntax error in DECLARE statement | compile error |
