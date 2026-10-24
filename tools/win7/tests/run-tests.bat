@echo off
rem RapidR on Windows 7: the smoke tests. Copy the whole "rapidr" folder to C:\
rem first (the disc is read-only), then double-click this file in C:\rapidr\tests.
rem Everything goes to results.txt in this folder, and is shown at the end.
setlocal
cd /d "%~dp0"
for %%i in ("%~dp0..") do set "RAPIDR_HOME=%%~fi"
set RAPIDR_AI_KEYSTORE=memory
set OUT=%~dp0results.txt
set RR=%RAPIDR_HOME%\rapidr.exe

echo RapidR on Windows 7: smoke tests > "%OUT%"
date /t >> "%OUT%"
time /t >> "%OUT%"
ver >> "%OUT%"
echo. >> "%OUT%"

echo [1/5] rapidr version
echo --- 1. rapidr version >> "%OUT%"
"%RR%" version >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"
echo --- 1b. the six functions only newer Windows has: found, or answered by RapidR >> "%OUT%"
"%RR%" __win7_check >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"

echo [2/5] console program: hello.rr
echo --- 2. console program: hello.rr >> "%OUT%"
"%RR%" run hello.rr >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"

echo [3/5] language tour: language.rr
echo --- 3. language tour: language.rr >> "%OUT%"
"%RR%" run language.rr >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"

echo [4/5] window, the renderer chosen by RapidR (closes by itself after 20 s)
echo --- 4. window, default renderer: form_auto.rr >> "%OUT%"
"%RR%" run form_auto.rr >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"

echo [5/5] window, CPU renderer forced (closes by itself after 20 s)
echo --- 5. window, RAPIDR_RENDERER=cpu: form_auto.rr >> "%OUT%"
set RAPIDR_RENDERER=cpu
"%RR%" run form_auto.rr >> "%OUT%" 2>&1
echo exit=%errorlevel% >> "%OUT%"
set RAPIDR_RENDERER=

echo.
echo ================ results.txt ================
type "%OUT%"
echo.
echo Done. Studio (optional): run-studio.bat
pause
