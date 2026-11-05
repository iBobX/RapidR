@echo off
rem RapidR on Windows 7: the smoke tests. Copy the whole "rapidr" folder to C:\
rem first (the disc is read-only), then double-click this file in C:\rapidr\tests.
rem Everything goes to results.txt in this folder, and is shown at the end.
setlocal
cd /d "%~dp0"
for %%i in ("%~dp0..") do set "TOP=%%~fi"
set RAPIDR_AI_KEYSTORE=memory
set OUT=%~dp0results.txt
rem The alpha zip is an install (bin\rapidr.exe finds lib\rapidr\ by itself); tools/win7/stage.sh's test drop
rem has rapidr.exe beside ide\ and examples\, which RAPIDR_HOME names.
set RAPIDR_HOME=
set "RR=%TOP%\bin\rapidr.exe"
if not exist "%TOP%\bin\rapidr.exe" set "RR=%TOP%\rapidr.exe"
if not exist "%TOP%\bin\rapidr.exe" set "RAPIDR_HOME=%TOP%"

echo RapidR on Windows 7: smoke tests > "%OUT%"
date /t >> "%OUT%"
time /t >> "%OUT%"
ver >> "%OUT%"
echo. >> "%OUT%"

echo [1/6] rapidr version
echo --- 1. rapidr version >> "%OUT%"
"%RR%" version >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"
echo --- 1b. the six functions only newer Windows has: found, or answered by RapidR >> "%OUT%"
"%RR%" __win7_check >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"

echo [2/6] console program: hello.rr
echo --- 2. console program: hello.rr >> "%OUT%"
"%RR%" run hello.rr >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"

echo [3/6] language tour: language.rr
echo --- 3. language tour: language.rr >> "%OUT%"
"%RR%" run language.rr >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"

echo [3b/6] DLL calls: numbers, text, BYREF, DOUBLE, SINGLE
echo --- 3b. DLL calls: dll_calls.rr (each value should match the one in brackets) >> "%OUT%"
"%RR%" run dll_calls.rr >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"

echo [4/6] window, the renderer chosen by RapidR (closes by itself after 20 s)
echo --- 4. window, default renderer: form_auto.rr >> "%OUT%"
"%RR%" run form_auto.rr >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"

echo [5/6] window, CPU renderer forced (closes by itself after 20 s)
echo --- 5. window, RAPIDR_RENDERER=cpu: form_auto.rr >> "%OUT%"
set RAPIDR_RENDERER=cpu
"%RR%" run form_auto.rr >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"
set RAPIDR_RENDERER=

echo [6/6] Build (interpreted): hello.rr and form_auto.rr become programs of their own, and run
echo --- 6. Build (interpreted): hello.exe, then form_auto.exe (closes by itself after 20 s) >> "%OUT%"
"%RR%" build hello.rr --interp >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"
if exist hello.exe hello.exe >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"
"%RR%" build form_auto.rr --interp >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"
if exist form_auto.exe form_auto.exe >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"

echo.
echo ================ results.txt ================
type "%OUT%"
echo.
echo Done. Studio (optional): run-studio.bat
pause
