' RapidR test helper for the Windows XP test VM (lane WINXP, 2026-11-05), version 3.
'
' Double-click it once in the VM. It opens a small console window and asks
' the Mac (tools/winxp/xphost.py, on the host-only network only: the VM has
' no internet) what to do next: run a command line, start a program with a
' window, fetch a file from the Mac, send a file back, list or stop a process,
' or replace itself with a newer helper. It runs as the user at the desktop,
' so the programs it starts show their windows there.
' Close its window to stop it. It installs nothing and changes no setting.
'
' Version 3: "keys" types into a window by its title, as a person would.
' Version 2: one job's failure never stops the helper (it is reported and the
' next job runs); a command has a time limit (default 600 s) and is stopped
' when it runs over; "spawn" starts a program with its own window and does not
' wait or capture its output (a window program keeps handles open: capturing
' its output was what stopped version 1); "update" replaces this file.
'
' Written for Windows XP SP3's own Windows Script Host 5.7 (VBScript),
' MSXML 3, ADODB.Stream and WMI: nothing to install.
Option Explicit

Const HOST = "http://192.168.128.1:8097"
Const WORK = "C:\xp"
Const VERSION = 3

Dim sh, fso, q
Set sh = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
q = Chr(34)

' (a console window of its own, so it can be seen and closed)
If InStr(LCase(WScript.FullName), "cscript.exe") = 0 Then
    sh.Run "cscript.exe //nologo " & q & WScript.ScriptFullName & q, 1, False
    WScript.Quit
End If

If Not fso.FolderExists(WORK) Then fso.CreateFolder WORK
WScript.Echo "RapidR test helper for Windows XP (version " & VERSION & ")."
WScript.Echo "Talking to " & HOST & " (the Mac, host-only network)."
WScript.Echo "Leave this window open while the tests run; close it to stop."
WScript.Echo ""

Dim seq, said
seq = 0
said = False
Do
    Dim job
    job = HttpText(HOST & "/next?seq=" & seq & "&v=" & VERSION & "&host=" & sh.ExpandEnvironmentStrings("%COMPUTERNAME%"))
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
        SafeJob job
    End If
Loop

' A job, whatever happens in it: an error is reported as its answer, and the
' helper goes on with the next one.
Sub SafeJob(text)
    Dim id, parts
    parts = Split(Replace(text, vbCr, ""), vbLf)
    If UBound(parts) < 1 Then Exit Sub
    id = parts(0)
    On Error Resume Next
    RunJob parts
    If Err.Number <> 0 Then
        Dim why
        why = "#error in the helper: " & Err.Number & " " & Err.Description
        Err.Clear
        WScript.Echo Now & "  " & why
        Post id, "text", why
    End If
    On Error GoTo 0
End Sub

' One job: "<id>" newline "<kind>" newline "<argument>" [newline "<argument 2>"]
Sub RunJob(parts)
    Dim id, kind, a, b
    id = parts(0)
    kind = parts(1)
    a = "" : b = ""
    If UBound(parts) >= 2 Then a = parts(2)
    If UBound(parts) >= 3 Then b = parts(3)
    WScript.Echo Now & "  " & kind & " " & Left(a, 70)
    Select Case kind
        Case "exec"
            Dim limit
            limit = 600
            If b <> "" Then limit = CLng(b)
            Post id, "text", RunCommand(id, a, limit)
        Case "spawn"
            Post id, "text", Spawn(a)
        Case "get"
            Post id, "text", GetFile(a, b)
        Case "put"
            PutFile id, a
        Case "ps"
            Post id, "text", Processes()
        Case "kill"
            Post id, "text", KillPid(a)
        Case "keys"
            Post id, "text", Keys(a, b)
        Case "update"
            UpdateSelf id, a
        Case "quit"
            Post id, "text", "bye"
            WScript.Quit
        Case Else
            Post id, "text", "unknown job kind " & kind
    End Select
End Sub

' A command line through cmd.exe, from a batch file of its own (any quotes and
' brackets are cmd's business): its output and exit code, or "#timeout" after
' `limit` seconds (the command's processes are then stopped).
Function RunCommand(id, line, limit)
    Dim bat, body, outp, donef, f, code, s, waited
    bat = WORK & "\job-" & id & ".cmd"
    body = WORK & "\job-" & id & "-body.cmd"
    outp = WORK & "\job-" & id & ".txt"
    donef = WORK & "\job-" & id & ".done"
    ' (the command in a batch of its own, run by a child cmd: an "exit N" in it
    ' ends that child with N, and the outer batch still writes the code down)
    Set f = fso.CreateTextFile(body, True)
    f.WriteLine "@echo off"
    f.WriteLine line
    f.Close
    Set f = fso.CreateTextFile(bat, True)
    f.WriteLine "@echo off"
    f.WriteLine "cmd /c " & body
    ' (the redirection first: "echo 2> file" would be read as stderr's redirection)
    f.WriteLine ">" & q & donef & q & " echo %errorlevel%"
    f.Close
    ' (cmd /c with several quoted parts: the whole line in one more pair, which
    ' cmd strips; the quotes around the paths survive)
    sh.Run "cmd.exe /c " & q & q & bat & q & " > " & q & outp & q & " 2>&1" & q, 0, False
    waited = 0
    Do While Not fso.FileExists(donef) And waited < limit * 2
        WScript.Sleep 500
        waited = waited + 1
    Loop
    code = "timeout"
    If fso.FileExists(donef) Then
        WScript.Sleep 200
        code = Trim(Replace(Replace(ReadText(donef), vbCr, ""), vbLf, ""))
    Else
        StopJob id
    End If
    s = ReadText(outp)
    On Error Resume Next
    fso.DeleteFile outp
    fso.DeleteFile donef
    fso.DeleteFile bat
    fso.DeleteFile body
    Err.Clear
    On Error GoTo 0
    RunCommand = s & vbLf & "#exit " & code
End Function

' a file's text ("" if it is not there or cannot be read)
Function ReadText(path)
    Dim f
    ReadText = ""
    On Error Resume Next
    If fso.FileExists(path) Then
        If fso.GetFile(path).Size > 0 Then
            Set f = fso.OpenTextFile(path, 1)
            ReadText = f.ReadAll
            f.Close
        End If
    End If
    Err.Clear
    On Error GoTo 0
End Function

' stops a timed-out job's processes, by their numbers: the cmd.exe instances
' whose command line names its batch files, and everything they started
Sub StopJob(id)
    Dim w, p, tree, grew
    On Error Resume Next
    Set w = GetObject("winmgmts:\\.\root\cimv2")
    tree = ","
    For Each p In w.ExecQuery("SELECT ProcessId, CommandLine FROM Win32_Process WHERE Name = 'cmd.exe'")
        If InStr(p.CommandLine, "job-" & id) > 0 Then tree = tree & p.ProcessId & ","
    Next
    If tree = "," Then Exit Sub
    Do
        grew = False
        For Each p In w.ExecQuery("SELECT ProcessId, ParentProcessId FROM Win32_Process")
            If InStr(tree, "," & p.ParentProcessId & ",") > 0 And InStr(tree, "," & p.ProcessId & ",") = 0 Then
                tree = tree & p.ProcessId & ","
                grew = True
            End If
        Next
    Loop While grew
    For Each p In w.ExecQuery("SELECT * FROM Win32_Process")
        If InStr(tree, "," & p.ProcessId & ",") > 0 Then p.Terminate
    Next
    Err.Clear
    On Error GoTo 0
End Sub

' A program started with a window of its own (visible, on this desktop), not
' waited for, its output not captured. Its process number: vm.sh ps.
Function Spawn(line)
    On Error Resume Next
    sh.CurrentDirectory = WORK
    sh.Run line, 1, False
    If Err.Number <> 0 Then
        Spawn = "#error " & Err.Description
    Else
        Spawn = "#started"
    End If
    Err.Clear
    On Error GoTo 0
End Function

' Keys typed into a window, as a person would: the window whose title begins or
' ends with `title` is brought to the front (WshShell.AppActivate), then `keys`
' (WshShell.SendKeys notation: "{F5}", "^+b" for Ctrl+Shift+B, "%{F4}").
Function Keys(title, k)
    On Error Resume Next
    If Not sh.AppActivate(title) Then
        Keys = "#error no window titled " & title
        Exit Function
    End If
    WScript.Sleep 400
    sh.SendKeys k
    If Err.Number <> 0 Then
        Keys = "#error " & Err.Description
    Else
        Keys = "#typed"
    End If
    Err.Clear
    On Error GoTo 0
End Function

' A newer helper: fetched from the Mac's share as `name`, written over this
' file, and started; this one ends.
Sub UpdateSelf(id, name)
    Dim r, mine
    mine = WScript.ScriptFullName
    r = GetFile(name, mine & ".new")
    If Left(r, 3) <> "#ok" Then
        Post id, "text", "#error " & r
        Exit Sub
    End If
    fso.CopyFile mine & ".new", mine, True
    fso.DeleteFile mine & ".new"
    Post id, "text", "#updated; starting the new helper"
    sh.Run "cscript.exe //nologo " & q & mine & q, 1, False
    WScript.Quit
End Sub

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
