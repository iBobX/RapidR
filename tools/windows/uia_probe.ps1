# What Windows' UI Automation (Narrator, NVDA, JAWS) sees of a RapidR
# program's window, and its actions run through it (AccessKit's adapter ->
# the UI kernel -> the program's handlers).
#
#   powershell -ExecutionPolicy Bypass -File tools\windows\uia_probe.ps1 <program.exe> [-Invoke Name] [-Toggle Name] [-Set "Name=Text"]
#
# Starts the program (RAPIDR_PRINT_TO / RAPIDR_REGISTRY pointed at %TEMP%),
# prints its window's tree (control type, name, value, toggle state, range,
# disabled), runs the actions asked for in the order Set, Toggle, Invoke,
# prints the tree's texts after each, then stops the program. The macOS
# counterpart is tools/macos/ax_dump.swift.
param(
  [Parameter(Mandatory = $true)][string]$Exe,
  [string]$Invoke,
  [string]$Toggle,
  [string]$Set
)
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$env:RAPIDR_PRINT_TO = "$env:TEMP\rapidr-prints"; $env:RAPIDR_REGISTRY = "$env:TEMP\rapidr-probe.reg"
$p = Start-Process -PassThru $Exe -RedirectStandardError "$env:TEMP\uia_probe_err.txt"
$A = [System.Windows.Automation.AutomationElement]
$Scope = [System.Windows.Automation.TreeScope]
$cond = New-Object System.Windows.Automation.PropertyCondition($A::ProcessIdProperty, $p.Id)
$win = $null
for ($i = 0; $i -lt 40 -and -not $win; $i++) {
  Start-Sleep -Milliseconds 250
  $win = $A::RootElement.FindFirst($Scope::Children, $cond)
}
if (-not $win) { Write-Output "no window"; if (-not $p.HasExited) { Stop-Process -Id $p.Id }; exit 1 }

$walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
function Walk($e, $d) {
  if ($d -gt 12) { return }
  $c = $e.Current
  $extra = ""
  try { $extra += " value='" + $e.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).Current.Value + "'" } catch {}
  try { $extra += " toggle=" + $e.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Current.ToggleState } catch {}
  try { $r = $e.GetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern).Current; $extra += " range=" + $r.Value + "/" + $r.Maximum } catch {}
  if (-not $c.IsEnabled) { $extra += " disabled" }
  if ($c.HelpText) { $extra += " help='" + $c.HelpText + "'" }
  Write-Output ((" " * (2 * $d)) + $c.ControlType.ProgrammaticName.Replace("ControlType.", "") + " '" + $c.Name + "'" + $extra)
  $ch = $walker.GetFirstChild($e)
  while ($ch) { Walk $ch ($d + 1); $ch = $walker.GetNextSibling($ch) }
}
function Named($name, $type) {
  foreach ($e in $win.FindAll($Scope::Descendants, (New-Object System.Windows.Automation.PropertyCondition($A::NameProperty, $name)))) {
    if (-not $type -or $e.Current.ControlType -eq $type) { return $e }
  }
}
function Texts() {
  $all = $win.FindAll($Scope::Descendants, (New-Object System.Windows.Automation.PropertyCondition($A::ControlTypeProperty, [System.Windows.Automation.ControlType]::Text)))
  "texts: " + (($all | ForEach-Object { $_.Current.Name }) -join " | ")
}

Walk $win 0
if ($Set) {
  $name, $text = $Set.Split("=", 2)
  (Named $name ([System.Windows.Automation.ControlType]::Edit)).GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($text)
  Start-Sleep -Milliseconds 800; Write-Output ("after Set " + $Set + ": " + (Texts))
}
if ($Toggle) {
  (Named $Toggle).GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle()
  Start-Sleep -Milliseconds 800; Write-Output ("after Toggle " + $Toggle + ": " + (Texts))
}
if ($Invoke) {
  (Named $Invoke ([System.Windows.Automation.ControlType]::Button)).GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
  Start-Sleep -Milliseconds 800; Write-Output ("after Invoke " + $Invoke + ": " + (Texts))
}
if ($p.HasExited) { Write-Output ("the program exited: 0x{0:X8}" -f $p.ExitCode) } else { Stop-Process -Id $p.Id }
$err = Get-Content "$env:TEMP\uia_probe_err.txt" -ErrorAction SilentlyContinue
if ($err) { Write-Output "== stderr"; $err | Select-Object -Last 20 }
