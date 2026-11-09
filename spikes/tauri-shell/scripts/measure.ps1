param(
  [int]$RootPid,
  [int]$Seconds = 20,
  [int]$Tick = 2,
  [string]$Out
)
function Get-Tree([int]$root) {
  $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name
  $ids = New-Object System.Collections.Generic.HashSet[int]
  [void]$ids.Add($root)
  $changed = $true
  while ($changed) {
    $changed = $false
    foreach ($p in $all) {
      if ($ids.Contains([int]$p.ParentProcessId) -and -not $ids.Contains([int]$p.ProcessId)) {
        [void]$ids.Add([int]$p.ProcessId); $changed = $true
      }
    }
  }
  $names = @{}
  foreach ($p in $all) { if ($ids.Contains([int]$p.ProcessId)) { $names[[int]$p.ProcessId] = $p.Name } }
  return $names
}
$samples = @()
$cpuStart = $null
$t0 = Get-Date
$ticks = [math]::Floor($Seconds / $Tick)
for ($i = 0; $i -lt $ticks; $i++) {
  $names = Get-Tree $RootPid
  $perf = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object { $names.ContainsKey([int]$_.IDProcess) }
  $priv = 0; $ws = 0
  foreach ($p in $perf) { $priv += $p.WorkingSetPrivate; $ws += $p.WorkingSet }
  $cpu = 0.0
  $roles = @{}
  foreach ($id in $names.Keys) {
    $pr = Get-Process -Id $id -ErrorAction SilentlyContinue
    if ($pr) { $cpu += $pr.TotalProcessorTime.TotalSeconds }
  }
  foreach ($p in $perf) {
    $n = $names[[int]$p.IDProcess]
    if (-not $roles.ContainsKey($n)) { $roles[$n] = @{ procs = 0; privateMB = 0.0 } }
    $roles[$n].procs++
    $roles[$n].privateMB += [math]::Round($p.WorkingSetPrivate / 1MB, 1)
  }
  $samples += [pscustomobject]@{ t = ((Get-Date) - $t0).TotalSeconds; procs = $names.Count; privateMB = [math]::Round($priv / 1MB, 1); wsMB = [math]::Round($ws / 1MB, 1); cpuSec = $cpu; roles = $roles }
  Start-Sleep -Seconds $Tick
}
$first = $samples[0]; $last = $samples[-1]
$elapsed = $last.t - $first.t
$cpuPct = if ($elapsed -gt 0) { [math]::Round((($last.cpuSec - $first.cpuSec) / $elapsed) * 100, 1) } else { 0 }
$avgPriv = [math]::Round((($samples | Measure-Object privateMB -Average).Average), 1)
$maxPriv = ($samples | Measure-Object privateMB -Maximum).Maximum
$avgWs = [math]::Round((($samples | Measure-Object wsMB -Average).Average), 1)
$res = [pscustomobject]@{ rootPid = $RootPid; seconds = $Seconds; ticks = $samples.Count; procs = $last.procs; privateMBAvg = $avgPriv; privateMBMax = $maxPriv; wsMBAvg = $avgWs; cpuPctOfOneCore = $cpuPct; lastRoles = $last.roles }
$json = $res | ConvertTo-Json -Depth 6
if ($Out) { Set-Content -Path $Out -Value $json } else { $json }
