' RapidR test helper for the Windows XP test VM (lane WINXP, 2026-11-05).
'
' Double-click it once in the VM. It opens a small console window and asks
' the Mac (tools/winxp/xphost.py, on the host-only network only: the VM has
' no internet) what to do next: run a command line, fetch a file from the
' Mac, send a file back, list or stop a process. It runs as the user at the
' desktop, so the programs it starts show their windows there.
' Close its window to stop it. It installs nothing and changes no setting.
'
' Written for Windows XP SP3's own Windows Script Host 5.7 (VBScript),
' MSXML 3 and ADODB.Stream: nothing to install.
Option Explicit

Const HOST = "http://192.168.128.1:8097"
Const WORK = "C:\xp"

Dim sh, fso
Set sh = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")

' (a console window of its own, so it can be seen and closed)
If InStr(LCase(WScript.FullName), "cscript.exe") = 0 Then
    sh.Run "cscript.exe //nologo """ & WScript.ScriptFullName & """", 1, False
    WScript.Quit
End If

If Not fso.FolderExists(WORK) Then fso.CreateFolder WORK
WScript.Echo "RapidR test helper for Windows XP."
WScript.Echo "Talking to " & HOST & " (the Mac, host-only network)."
WScript.Echo "Leave this window open while the tests run; close it to stop."
WScript.Echo ""

Dim seq, said, idle
seq = 0
said = False
idle = 0
Do
    Dim job
    job = HttpText(HOST & "/next?seq=" & seq & "&host=" & sh.ExpandEnvironmentStrings("%COMPUTERNAME%"))
    If job = "#offline" Then
        If said Then WScript.Echo Now & "  the Mac does not answer; trying again"
        said = False
        WScript.Sleep 3000
    ElseIf job = "" Then
        If Not said Then WScript.Echo Now & "  connected; waiting for work"
        said = True
        WScript.Sleep 700
    Else
        said = True
        seq = seq + 1
        RunJob job
    End If
Loop

' One job: "<id>" newline "<kind>" newline "<argument>" [newline "<argument 2>"]
Sub RunJob(text)
    Dim parts, id, kind, a, b, answer
    parts = Split(Replace(text, vbCr, ""), vbLf)
    If UBound(parts) < 1 Then Exit Sub
    id = parts(0)
    kind = parts(1)
    a = "" : b = ""
    If UBound(parts) >= 2 Then a = parts(2)
    If UBound(parts) >= 3 Then b = parts(3)
    WScript.Echo Now & "  " & kind & " " & Left(a, 70)
    Select Case kind
        Case "exec"
            answer = RunCommand(id, a)
            Post id, "text", answer
        Case "get"
            Post id, "text", GetFile(a, b)
        Case "put"
            PutFile id, a
        Case "ps"
            Post id, "text", Processes()
        Case "kill"
            Post id, "text", KillPid(a)
        Case "quit"
            Post id, "text", "bye"
            WScript.Quit
        Case Else
            Post id, "text", "unknown job kind " & kind
    End Select
End Sub

' A command line through cmd.exe, from a batch file of its own (any quotes
' and brackets are cmd's business); its output and exit code.
Function RunCommand(id, line)
    Dim bat, outp, f, code, s
    bat = WORK & "\job-" & id & ".cmd"
    outp = WORK & "\job-" & id & ".txt"
    Set f = fso.CreateTextFile(bat, True)
    f.WriteLine "@echo off"
    f.WriteLine line
    f.WriteLine "@exit /b %errorlevel%"
    f.Close
    ' cmd /c with several quoted parts: wrap the whole line in one more pair, so cmd strips only
    ' the outer pair (its documented rule) and the inner quotes around the paths survive. Without the
    ' extra pair, cmd strips the first quote and the last quote on the line and the .cmd path breaks.
    Dim q
    q = Chr(34)
    code = sh.Run("cmd.exe /c " & q & q & bat & q & " > " & q & outp & q & " 2>&1" & q, 0, True)
    s = ""
    If fso.FileExists(outp) Then
        If fso.GetFile(outp).Size > 0 Then
            Set f = fso.OpenTextFile(outp, 1)
            s = f.ReadAll
            f.Close
        End If
        fso.DeleteFile outp
    End If
    fso.DeleteFile bat
    RunCommand = s & vbLf & "#exit " & code
End Function

' A file from the Mac's share (the helper's folder there) to a path here.
Function GetFile(name, dest)
    Dim x, st
    On Error Resume Next
    Set x = CreateObject("MSXML2.ServerXMLHTTP")
    x.setTimeouts 5000, 5000, 600000, 600000
    x.Open "GET", HOST & "/files/" & name, False
    x.Send
    If Err.Number <> 0 Then GetFile = "#error " & Err.Description : Exit Function
    If x.Status <> 200 Then GetFile = "#error HTTP " & x.Status : Exit Function
    Set st = CreateObject("ADODB.Stream")
    st.Type = 1
    st.Open
    st.Write x.responseBody
    st.SaveToFile dest, 2
    st.Close
    If Err.Number <> 0 Then GetFile = "#error " & Err.Description : Exit Function
    GetFile = "#ok " & fso.GetFile(dest).Size
End Function

' A file here to the Mac (its results folder).
Sub PutFile(id, path)
    Dim st, x
    If Not fso.FileExists(path) Then Post id, "text", "#error no file " & path : Exit Sub
    On Error Resume Next
    Set st = CreateObject("ADODB.Stream")
    st.Type = 1
    st.Open
    st.LoadFromFile path
    Set x = CreateObject("MSXML2.ServerXMLHTTP")
    x.setTimeouts 5000, 5000, 600000, 600000
    x.Open "POST", HOST & "/done/" & id & "?kind=bin", False
    x.Send st.Read
    st.Close
    If Err.Number <> 0 Then Post id, "text", "#error " & Err.Description
End Sub

Function Processes()
    Dim w, p, s
    s = ""
    Set w = GetObject("winmgmts:\\.\root\cimv2")
    For Each p In w.ExecQuery("SELECT ProcessId, Name, CommandLine FROM Win32_Process")
        s = s & p.ProcessId & vbTab & p.Name & vbTab & p.CommandLine & vbLf
    Next
    Processes = s
End Function

' Stops one process, by its number (never by name).
Function KillPid(pid)
    Dim w, p, n
    n = 0
    Set w = GetObject("winmgmts:\\.\root\cimv2")
    For Each p In w.ExecQuery("SELECT * FROM Win32_Process WHERE ProcessId = " & CLng(pid))
        p.Terminate
        n = n + 1
    Next
    KillPid = "#stopped " & n
End Function

Function HttpText(url)
    Dim x
    On Error Resume Next
    Set x = CreateObject("MSXML2.ServerXMLHTTP")
    x.setTimeouts 3000, 3000, 10000, 10000
    x.Open "GET", url, False
    x.Send
    If Err.Number <> 0 Then HttpText = "#offline" : Exit Function
    If x.Status <> 200 Then HttpText = "#offline" : Exit Function
    HttpText = x.responseText
End Function

Sub Post(id, kind, text)
    Dim x
    On Error Resume Next
    Set x = CreateObject("MSXML2.ServerXMLHTTP")
    x.setTimeouts 3000, 3000, 60000, 60000
    x.Open "POST", HOST & "/done/" & id & "?kind=" & kind, False
    x.setRequestHeader "Content-Type", "text/plain; charset=utf-8"
    x.Send text
End Sub
