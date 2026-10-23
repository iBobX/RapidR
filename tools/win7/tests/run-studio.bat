@echo off
rem RapidR Studio on Windows 7. Needs the "rapidr" folder (rapidr.exe, ide\,
rem examples\) copied to C:\ ; double-click this file in C:\rapidr\tests.
setlocal
cd /d "%~dp0"
for %%i in ("%~dp0..") do set "RAPIDR_HOME=%%~fi"
set RAPIDR_AI_KEYSTORE=memory
"%RAPIDR_HOME%\rapidr.exe" ide > "%~dp0studio-out.txt" 2>&1
echo Studio exited with %errorlevel% >> "%~dp0studio-out.txt"
type "%~dp0studio-out.txt"
pause
