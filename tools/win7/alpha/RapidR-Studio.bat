@echo off
rem RapidR Studio (Windows 7 / 8 / 8.1 alpha): double-click this file.
rem A black window stays open behind Studio and shows any error text.
setlocal
cd /d "%~dp0"
for %%i in ("%~dp0.") do set "RAPIDR_HOME=%%~fi"
rem (the AI helper's keys are kept in memory only in this alpha)
set RAPIDR_AI_KEYSTORE=memory
"%RAPIDR_HOME%\rapidr.exe" ide
if errorlevel 1 (
    echo.
    echo RapidR Studio stopped with an error ^(see above^). Please tell us what it said.
    echo See README-ALPHA.txt, "How to tell us what happened".
    pause
)
