# DLL memory on Windows (docs/windows-dll-calls.md section 7): the branch's sources (prepare: a `git
# archive` in -Prep's src.tar) built in the VM, then the memory model's and the DLL calls' own
# tests, the conformance suite's DLL and memory cases (the interpreter, the JIT, native builds),
# the Windows examples that call winmm, and RapidQ programs that receive a MIDI SysEx message
# from a stand-in for winmm (tests/fixtures/fake_winmm: its "driver" writes the program's
# MIDIHDR and buffer after midiInAddBuffer returned and posts MM_MIM_LONGDATA), each run with
# this build and with -Old's rapidr.exe (the memory model before), their logs and window
# captures in %USERPROFILE%\rapidr-release\out\dllmem-*.
#
#   tools/release/windows-vm.sh dllmem [<folder of program folders>] [Old=<rapidr.exe in the VM>]
#
# The programs: tests\fixtures\fake_winmm\programs\*, and the folders in -Prep's programs\ (the
# optional folder given: each a .bas and its includes, with the DLL they load named fakewinmm,
# the components named as sysex_in.bas's). Its own folder (%USERPROFILE%\rapidr-dllmem), kept for the next run's
# incremental build, seeded once from the JIT check's (rapidr-jit\host). Nothing here prints,
# touches the user's registry store or settings. The programs show real windows (a window's
# handle must be real for PostMessage): they close themselves once captured.
# (This file is read as ANSI by Windows PowerShell: no character outside ASCII in its code.)
param(
    [Parameter(Mandatory = $true)][string]$Prep,
    [string]$Old = "$env:USERPROFILE\rapidr-jit\host\target\release\rapidr.exe",
    # RapidQ's examples that call DLLs (-Prep's corpus-list.txt, tools/dll_corpus_list.py), each
    # interpreted with this build and with -Old's (tools/dll_breadth.py), and what differs
    [switch]$Corpus,
    # only the corpus (the build, the tests and the programs skipped: a build there already)
    [switch]$CorpusOnly,
    # RapidQ's folder (its examples and includes: only read)
    [string]$RapidQ = "\\Mac\Home\Downloads\Rapidq"
)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
$node = "$env:USERPROFILE\node\node.exe"
$W = "$env:USERPROFILE\rapidr-dllmem"
$src = "$W\src"
$out = "$env:USERPROFILE\rapidr-release\out"
cmd /c "rmdir /s /q `"$src`" 2>nul" | Out-Null
New-Item -ItemType Directory -Force $src, "$W\prints", "$W\config", $out | Out-Null
& tar.exe -m -xf "$Prep\src.tar" -C $src
if ($LASTEXITCODE -ne 0) { Write-Host "extracting the sources failed"; exit 1 }
if (-not (Test-Path "$W\target") -and (Test-Path "$env:USERPROFILE\rapidr-jit\host\target")) {
    Write-Host "== seeding the build cache from rapidr-jit\host"
    & robocopy "$env:USERPROFILE\rapidr-jit\host\target" "$W\target" /E /NFL /NDL /NJH /NJS /NP | Out-Null
}
if (Test-Path $Old) { Copy-Item -Force $Old "$W\rapidr-old.exe" }
$env:CARGO_TARGET_DIR = "$W\target"
$env:RAPIDR_PRINT_TO = "$W\prints"; $env:RAPIDR_REGISTRY = "$W\registry.reg"; $env:RAPIDR_AI_KEYSTORE = "memory"
$env:RAPIDR_CONFIG_DIR = "$W\config"; $env:RAPIDR_BUILD_CACHE = "$W\build-cache"; $env:LANGUAGE = "en_US.UTF-8"; $env:RAPIDR_LANG = "en"
if (-not $env:RAPIDR_TEST_MODE -and (Test-Path "$env:USERPROFILE\.rapidr\test-mode.token")) { $env:RAPIDR_TEST_MODE = "$env:USERPROFILE\.rapidr\test-mode.token" }
Push-Location $src
$fail = 0
Write-Host "== $(& rustc -vV | Select-String host), building rapidr"
& cargo build --release -q -p rapidr-cli 2>&1 | Select-String -Pattern "^error" -Context 0, 6 | Select-Object -First 20 | Write-Host
if (-not (Test-Path "$W\target\release\rapidr.exe")) { Write-Host "the build failed"; Pop-Location; exit 1 }
Copy-Item -Force "$W\target\release\rapidr.exe" "$src\rapidr.exe"
if (-not $CorpusOnly) {
Write-Host "== the memory model's tests, the DLL calls' tests"
& cargo test --release -q -p rapidr-value --lib memory:: 2>&1 | Select-String -Pattern "test result|panicked|FAILED|^error" | Select-Object -First 20 | Write-Host
if ($LASTEXITCODE -ne 0) { $fail = 1 }
& cargo test --release -q -p rapidr-runtime-core --lib ffi:: 2>&1 | Select-String -Pattern "test result|panicked|FAILED|^error" | Select-Object -First 20 | Write-Host
if ($LASTEXITCODE -ne 0) { $fail = 1 }
if (Test-Path $node) {
    try { [Console]::OutputEncoding = [Text.Encoding]::UTF8 } catch { }
    New-Item -ItemType Directory -Force "$src\tests\conformance\.work" | Out-Null
    Write-Host "== conformance: DLL and memory cases (vm, vm-jit, codegen)"
    & $node tests\conformance\run.mjs dll_ peek sizeof varptr memstream memcopy stream_udt filestream sendmessage *> "$W\conformance.txt"
    $c = Get-Content "$W\conformance.txt"
    $c | Where-Object { $_ -match "^FAIL|^XPASS|passed," } | Select-Object -First 40 | Write-Host
    $c | Where-Object { $_ -match "^\s+\|" } | Select-Object -First 40 | Write-Host
    if (-not (($c -join "`n") -match "\d+ passed, \d+ known failures \(xfail\), 0 failed")) { $fail = 1 }
    Write-Host "== the examples that call winmm (rapidr run, interpreted build, native build)"
    & $node tests\examples_run.mjs --runtimes run,interp,native windows/midi_in windows/midi_sysex *> "$W\examples.txt"
    $e = Get-Content "$W\examples.txt"
    $e | Where-Object { $_ -match "^Examples:|midi" } | Select-Object -First 20 | Write-Host
    if (-not (($e -join "`n") -match "Examples: \d+ checks passed, 0 failed")) { $fail = 1 }
} else { Write-Host "no node.exe: conformance and examples skipped" }
if (Test-Path "$W\rapidr-old.exe") {
    Write-Host "== the Windows DLL memory cases with -Old's rapidr.exe (the model before)"
    foreach ($case in Get-ChildItem "$src\tests\conformance\cases\dll_*_windows.bas") {
        Write-Host "-- $($case.Name)"
        Push-Location "$src\tests\conformance\cases"
        & "$W\rapidr-old.exe" run $case.Name 2>&1 | Select-Object -First 12 | ForEach-Object { "   $_" } | Write-Host
        Pop-Location
    }
}
Write-Host "== the stand-in for winmm"
$env:CARGO_TARGET_DIR = "$W\fake-target"
Push-Location "$src\tests\fixtures\fake_winmm"
& cargo build --release -q 2>&1 | Select-String -Pattern "^error" -Context 0, 6 | Select-Object -First 20 | Write-Host
Pop-Location
$env:CARGO_TARGET_DIR = "$W\target"
$fake = "$W\fake-target\release\fake_winmm.dll"
if (-not (Test-Path $fake)) { Write-Host "the stand-in didn't build"; Pop-Location; exit 1 }
Write-Host "== RapidQ programs receiving SysEx through it (this build, and -Old's)"
$env:RAPIDR_CAPTURE_WINDOWS = "1"; $env:RAPIDR_CAPTURE_DELAY = "1"
$env:RAPIDR_TEST_EVENTS = "SearchMIDIDevsBTN.onclick,DevINList.__item_0,StartStopMIDIIn.onclick,MainForm.__wait_1800"
$env:RAPIDR_TEST_DUMP = "LogsRichEdit.text"
$programs = @(Get-ChildItem -Directory "$src\tests\fixtures\fake_winmm\programs")
if (Test-Path "$Prep\programs") { $programs += @(Get-ChildItem -Directory "$Prep\programs") }
foreach ($dir in $programs) {
    foreach ($which in @("new", "old")) {
        $exe = if ($which -eq "new") { "$src\rapidr.exe" } else { "$W\rapidr-old.exe" }
        if (-not (Test-Path $exe)) { continue }
        $run = "$W\runs\$($dir.Name)-$which"
        cmd /c "rmdir /s /q `"$run`" 2>nul" | Out-Null
        New-Item -ItemType Directory -Force $run | Out-Null
        Copy-Item "$($dir.FullName)\*" $run
        Copy-Item $fake "$run\fakewinmm.dll"
        $name = "dllmem-$($dir.Name)-$which"
        $env:FAKE_WINMM_LOG = "$out\$name-driver.txt"
        Remove-Item -Force -ErrorAction SilentlyContinue "$out\$name-driver.txt", "$out\$name.txt", "$out\$name-*.bmp"
        $env:RAPIDR_CAPTURE = "$out\$name"
        $bas = (Get-ChildItem "$run\*.bas" | Select-Object -First 1).Name
        Push-Location $run
        $p = Start-Process -FilePath $exe -ArgumentList "run", $bas -NoNewWindow -PassThru -RedirectStandardOutput "$out\$name.txt" -RedirectStandardError "$out\$name-err.txt"
        if (-not $p.WaitForExit(60000)) { Stop-Process -Id $p.Id -Force; Write-Host "   $name didn't end in 60 s" }
        Pop-Location
        Write-Host "-- $name (exit $($p.ExitCode))"
        Get-Content "$out\$name.txt", "$out\$name-err.txt" -ErrorAction SilentlyContinue | Select-Object -First 40 | ForEach-Object { "   $_" } | Write-Host
        Write-Host "   driver:"
        Get-Content "$out\$name-driver.txt" -ErrorAction SilentlyContinue | ForEach-Object { "     $_" } | Write-Host
    }
}
Remove-Item Env:\RAPIDR_CAPTURE, Env:\RAPIDR_CAPTURE_WINDOWS, Env:\RAPIDR_TEST_EVENTS, Env:\RAPIDR_TEST_DUMP, Env:\FAKE_WINMM_LOG -ErrorAction SilentlyContinue
}
if ($Corpus -or $CorpusOnly) {
    $py = "$env:LOCALAPPDATA\Programs\Python\Python313-arm64\python.exe"
    if (-not (Test-Path $py)) { $pc = Get-Command python.exe -ErrorAction SilentlyContinue; if ($pc) { $py = $pc.Source } }
    if (-not (Test-Path "$Prep\corpus-list.txt")) { Write-Host "no corpus-list.txt in -Prep: the corpus skipped" }
    elseif (-not (Test-Path $py)) { Write-Host "no python.exe: the corpus skipped" }
    else {
        Write-Host "== RapidQ's DLL examples, interpreted, this build and -Old's (tools/dll_breadth.py)"
        # (names of their own: dll_breadth.py ends a program that hangs by its image name)
        Copy-Item -Force "$src\rapidr.exe" "$W\rapidr-dllmem-new.exe"
        if (Test-Path "$W\rapidr-old.exe") { Copy-Item -Force "$W\rapidr-old.exe" "$W\rapidr-dllmem-old.exe" }
        foreach ($which in @("new", "old")) {
            if (-not (Test-Path "$W\rapidr-dllmem-$which.exe")) { continue }
            cmd /c "rmdir /s /q `"$W\breadth-$which`" 2>nul" | Out-Null
            & $py tools\dll_breadth.py --rapidr "$W\rapidr-dllmem-$which.exe" --corpus "$RapidQ\examples" --include "$RapidQ\include" --list "$Prep\corpus-list.txt" --out "$W\breadth-$which" *> "$W\breadth-$which.txt"
            Write-Host "   $which`: $((Get-Content "$W\breadth-$which\results.jsonl" -ErrorAction SilentlyContinue | Measure-Object).Count) programs"
        }
        & $py tools\dll_breadth_compare.py "$W\breadth-old\results.jsonl" "$W\breadth-new\results.jsonl" 2>&1 | Write-Host
        Copy-Item -Force "$W\breadth-new\results.jsonl" "$out\dllmem-breadth-new.jsonl" -ErrorAction SilentlyContinue
        Copy-Item -Force "$W\breadth-old\results.jsonl" "$out\dllmem-breadth-old.jsonl" -ErrorAction SilentlyContinue
    }
}
Pop-Location
if ($fail -eq 0) { Write-Host "dll-memory-check: all passed" } else { Write-Host "dll-memory-check: FAILED" }
exit $fail
