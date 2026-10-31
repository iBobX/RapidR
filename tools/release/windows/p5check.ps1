# Preview 5's features, looked at headlessly in an installed RapidR Studio on Windows
# (tools/release/p5check.sh does the same on macOS and Linux; smoke.ps1 runs this one):
#
#   * Versions: a copy of a small project is put under versions, "first" is saved, a word is
#     changed and "second" saved, Go Back to "first" (the question answered): three versions,
#     the file as it was
#   * the Coach dial is in RAI's pane (Build / Coach), Coach can be chosen
#   * Set Up RAI > On this computer: with no local AI app, a calm "No local AI app found running
#     on this computer" (nothing started)
#   * RapidR Studio in Spanish (--lang es): Spanish menus, the dial in Spanish
#   * School Lock's dialog opens (Tools > School Lock...)
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File p5check.ps1 -Rapidr <rapidr.exe> -Work <folder>
#
# The test hooks of a release build act only with a signed test-mode token (SEC-24,
# docs/security/test-mode.md): RAPIDR_TEST_MODE, or %USERPROFILE%\.rapidr\test-mode.token. Every run
# has its own folders; nothing touches the user's settings, registry store, documents or projects.
# The folder is kept (the pictures are in it); the caller removes it. Exit status 0 = all passed.
param(
    [Parameter(Mandatory = $true)][string]$Rapidr,
    [Parameter(Mandatory = $true)][string]$Work
)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
$R = $Rapidr
New-Item -ItemType Directory -Force $Work | Out-Null
$Work = (Resolve-Path $Work).Path
if (-not $env:RAPIDR_TEST_MODE -and (Test-Path "$env:USERPROFILE\.rapidr\test-mode.token")) { $env:RAPIDR_TEST_MODE = "$env:USERPROFILE\.rapidr\test-mode.token" }
$script:fail = 0
function Ok($what) { Write-Host "  ok    $what" }
function Bad($what) { Write-Host "  FAIL  $what"; $script:fail = 1 }
# a line of the file's text matches (the dump's lines end in CR LF here)
function Check($what, $text, $pattern) {
    if ($text -match "(?m)$pattern") { Ok $what } else { Bad $what; Write-Host "        (wanted /$pattern/ in:)"; ($text -split "`r?`n" | Select-Object -First 12) | ForEach-Object { Write-Host "        | $_" } }
}
function Out-Of($exe, [string[]]$a) { (& $exe @a 2>&1 | Out-String) }

# Studio $name $lang $steps $keys $events $file : one headless Studio run on its own folders;
# what it printed (the dump, anything it said) comes back, and is in $Work\<name>\out.txt;
# the window's picture is $Work\<name>\window-1.bmp
function Studio($name, $lang, $steps, $keys, $events, $file) {
    $d = Join-Path $Work $name
    if (Test-Path $d) { Remove-Item -Recurse -Force $d }
    New-Item -ItemType Directory -Force "$d\docs", "$d\prints" | Out-Null
    $a = @("ide", "--fresh", "--theme", "rapidr-light")
    if ($lang) { $a += @("--lang", $lang) }
    $a += @("--do", $steps)
    if ($file) { $a += $file }
    $set = @{
        RAPIDR_CAPTURE = "$d\window"; RAPIDR_CAPTURE_DELAY = "5"; RAPIDR_MENU = "window"
        RAPIDR_TEST_DUMP = "$keys,studio.caption"
        RAPIDR_PRINT_TO = "$d\prints"; RAPIDR_REGISTRY = "$d\reg.reg"; RAPIDR_CONFIG_DIR = "$d\config"; RAPIDR_DOCUMENTS = "$d\docs"
        RAPIDR_AI_KEYSTORE = "memory"; RAPIDR_AI_CHATS = "$d\chats"; RAPIDR_STUDIO_DATA = "$d\studio"; RAPIDR_LOGIN_PATH = "off"
        ANTHROPIC_API_KEY = ""; OPENAI_API_KEY = ""; DEEPSEEK_API_KEY = ""; ANTHROPIC_WORKSPACE_ID = ""
    }
    if ($events) { $set["RAPIDR_TEST_EVENTS"] = $events }
    $before = @{}
    foreach ($k in $set.Keys) { $before[$k] = [Environment]::GetEnvironmentVariable($k); [Environment]::SetEnvironmentVariable($k, $set[$k]) }
    $text = Out-Of $R $a
    foreach ($k in $set.Keys) { [Environment]::SetEnvironmentVariable($k, $before[$k]) }
    Set-Content -Path "$d\out.txt" -Value $text
    return $text
}
function Click($comp, $x, $y) { "$comp.__mousedown_${x}_$y,$comp.__mouseup_${x}_$y" }
function Idle($n) { (1..$n | ForEach-Object { "aionnote.onclick" }) -join "," }
$aiOn = "ai:on,ai:provider=compatible,ai:base=http://127.0.0.1:9/v1,ai:model=mock-small"

$ver = (Out-Of $R @("version")).Trim()
$hooks = (Out-Of $R @("dev", "test-mode", "status")) -split "`r?`n" | Where-Object { $_ -like "hooks:*" }
Write-Host "== $ver, test hooks: $hooks"

Write-Host "== Versions: save, change, save, go back (a copy of a small project)"
$P = Join-Path $Work "versions-project"
if (Test-Path $P) { Remove-Item -Recurse -Force $P }
New-Item -ItemType Directory -Force $P | Out-Null
Set-Content -Path "$P\Hello.rrproj" -Value "# RapidR Studio project (format 2)`r`n`r`nformat = 2`r`nname = `"Hello`"`r`nmain = `"main.rr`"`r`n`r`n[[files]]`r`npath = `"main.rr`"`r`nkind = `"module`"`r`n`r`n[[files]]`r`npath = `"greet.rr`"`r`nkind = `"module`"`r`n"
Set-Content -Path "$P\greet.rr" -Value "' greet.rr: the greeting Hello prints.`r`nFUNCTION Greeting(Who AS STRING) AS STRING`r`n    Result = `"Hello, `" + Who + `"!`"`r`nEND FUNCTION`r`n"
Set-Content -Path "$P\main.rr" -Value "' Hello: a greeting and a count.`r`n`$INCLUDE `"greet.rr`"`r`n`r`nDIM Name AS STRING`r`nName = `"Ana`"`r`nPRINT Greeting(Name)`r`nPRINT `"Counting to three:`"`r`nFOR i = 1 TO 3`r`n    PRINT i`r`nNEXT`r`nPRINT `"Done.`"`r`n"
$t = Studio "versions" $null 'versions.pane,versions.start,versions:author=Ana,versions:save=first,code:three=>four,file.save,versions:save=second,versions:pick=first,versions.goBack,wait,versions.pane' `
    'repo.versioned,vertimeline.itemcount,repo.experiment,verundonote.visible,outputbox.text' "$(Click consentyes 40 14),verpanel.__wait_2500" "$P\Hello.rrproj"
Check "Studio started and kept running" $t '^studio\.caption=Hello - RapidR Studio'
Check "the project is under versions" $t '^repo\.versioned=(1|-1|True)'
Check "three versions: first, second, and the one that went back" $t '^vertimeline\.itemcount=3'
Check "Go Back said what it did" $t 'Went back to .+first'
Check "Undo is offered" $t '^verundonote\.visible=(1|-1|True)'
Check "main.rr is as it was in the first version" (Get-Content -Raw "$P\main.rr") 'Counting to three:'
Check "the versions are a repository on disk" ((Get-ChildItem "$P\.git" -Name) -join "`n") 'HEAD'

Write-Host "== Coach: the dial, and Coach chosen"
$t = Studio "coach" $null "$aiOn,ai.assistant,ai:coach:mode=coach,wait" 'aidialbuildtext.caption,aidialcoachtext.caption,aitools.mode,ai.insertlinks' $null "$P\main.rr"
Check "the dial says Build" $t '^aidialbuildtext\.caption=Build'
Check "the dial says Coach" $t '^aidialcoachtext\.caption=Coach'
Check "Coach is the mode" $t '^aitools\.mode=coach'
Check "no Insert in Coach mode" $t '^ai\.insertlinks=(0|False)'

Write-Host "== Set Up RAI > On this computer: no local AI app found"
$t = Studio "localai" $null "ai:setup,wait" 'aisufoundtext.caption,aisumodels.visible' "$(Idle 8),$(Click aisunextbtn 30 14),$(Idle 4)" "$P\main.rr"
$laName = "localai"
# (a computer that has a local AI app: its models are found, and the empty state is looked at with a closed port instead)
if (($t -notmatch '(?m)^aisufoundtext\.caption=No local AI app found') -and ($t -match '(?m)^aisufoundtext\.caption=.*Found \d+ model')) {
    Ok "this computer has a local AI app: Set Up RAI found its models; the empty state next, with a closed port"
    $t = Studio "localai-empty" $null "ai:setupprobes=http://127.0.0.1:9/v1,ai:setup,wait" 'aisufoundtext.caption,aisumodels.visible' "$(Idle 8),$(Click aisunextbtn 30 14),$(Idle 4)" "$P\main.rr"
    $laName = "localai-empty"
}
Check "says calmly that no local AI app was found" $t '^aisufoundtext\.caption=No local AI app found running on this computer'
Check "lists no models" $t '^aisumodels\.visible=(0|False)'

Write-Host "== Studio in Spanish"
$t = Studio "spanish" "es" "$aiOn,ai.assistant,wait" 'mi(0).caption,mi(1).caption,mi(2).caption,mi(9).caption,aidialbuildtext.caption,aidialcoachtext.caption' $null "$P\main.rr"
Check "the File menu is Archivo" $t '^mi\(0\)\.caption=&Archivo'
Check "the Edit menu is Edicion" $t '^mi\(1\)\.caption=&Edici.+n'
Check "the Help menu is Ayuda" $t '^mi\(9\)\.caption=Ay&uda'
Check "the dial is Construir" $t '^aidialbuildtext\.caption=Construir'
Check "the dial's Coach is Tutor" $t '^aidialcoachtext\.caption=Tutor'

Write-Host "== School Lock's dialog opens"
$t = Studio "lock" $null "tools.schoolLock" 'lockdialog.visible,lockpwedit.visible,lockdialog.caption' $null "$P\main.rr"
Check "the dialog is open" $t '^lockdialog\.visible=(1|-1|True)'
Check "it asks for the password" $t '^lockpwedit\.visible=(1|-1|True)'
Check "it is School Lock" $t '^lockdialog\.caption=.*School Lock'

Write-Host "== the pictures"
foreach ($c in "versions", "coach", $laName, "spanish", "lock") {
    if (Test-Path "$Work\$c\window-1.bmp") { Ok "$c`: $((Get-Item "$Work\$c\window-1.bmp").Length) bytes of window" } else { Bad "$c`: no picture of Studio's window" }
}
if ($script:fail -eq 0) { Write-Host "Preview 5 checks: all passed" } else { Write-Host "Preview 5 checks: FAILED" }
exit $script:fail
