<# : batch
@echo off
start "" powershell -NoProfile -ExecutionPolicy Bypass -STA -WindowStyle Hidden -Command "$env:GITGUI_FILE='%~f0'; $env:GITGUI_DIR='%~dp0'; iex (Get-Content -LiteralPath $env:GITGUI_FILE -Raw -Encoding UTF8)"
exit /b
#>

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$font = New-Object System.Drawing.Font('Segoe UI', 9)
$mono = New-Object System.Drawing.Font('Consolas', 9.5)

$form = New-Object System.Windows.Forms.Form
$form.Text = 'Git Pull / Push'
$form.Size = New-Object System.Drawing.Size(860, 600)
$form.MinimumSize = New-Object System.Drawing.Size(640, 420)
$form.StartPosition = 'CenterScreen'
$form.Font = $font

$lblRepo = New-Object System.Windows.Forms.Label
$lblRepo.Text = 'Repo:'
$lblRepo.Location = '12,15'
$lblRepo.AutoSize = $true

$txtRepo = New-Object System.Windows.Forms.TextBox
$txtRepo.Location = '60,12'
$txtRepo.Size = '660,24'
$txtRepo.Anchor = 'Top,Left,Right'
$txtRepo.Text = $env:GITGUI_DIR.TrimEnd('\')

$btnBrowse = New-Object System.Windows.Forms.Button
$btnBrowse.Text = 'Chọn...'
$btnBrowse.Location = '730,10'
$btnBrowse.Size = '100,27'
$btnBrowse.Anchor = 'Top,Right'

$lblBranch = New-Object System.Windows.Forms.Label
$lblBranch.Location = '12,46'
$lblBranch.Size = '820,20'
$lblBranch.Anchor = 'Top,Left,Right'
$lblBranch.ForeColor = [System.Drawing.Color]::DarkBlue

$flow = New-Object System.Windows.Forms.FlowLayoutPanel
$flow.Location = '8,70'
$flow.Size = '830,36'
$flow.Anchor = 'Top,Left,Right'
$flow.WrapContents = $false

function New-Btn($text, $width = 100) {
    $b = New-Object System.Windows.Forms.Button
    $b.Text = $text
    $b.Size = "$width,30"
    $flow.Controls.Add($b)
    return $b
}
$btnStatus  = New-Btn 'Status'
$btnFetch   = New-Btn 'Fetch'
$btnPull    = New-Btn 'Pull'
$btnRebase  = New-Btn 'Pull --rebase' 110
$btnPush    = New-Btn 'Push'
$btnLog     = New-Btn 'Log'
$btnClear   = New-Btn 'Xoá log'

$lblMsg = New-Object System.Windows.Forms.Label
$lblMsg.Text = 'Commit message:'
$lblMsg.Location = '12,117'
$lblMsg.AutoSize = $true

$txtMsg = New-Object System.Windows.Forms.TextBox
$txtMsg.Location = '120,114'
$txtMsg.Size = '480,24'
$txtMsg.Anchor = 'Top,Left,Right'

$btnCommit = New-Object System.Windows.Forms.Button
$btnCommit.Text = 'Commit tất cả'
$btnCommit.Location = '610,112'
$btnCommit.Size = '105,27'
$btnCommit.Anchor = 'Top,Right'

$btnCommitPush = New-Object System.Windows.Forms.Button
$btnCommitPush.Text = 'Commit + Push'
$btnCommitPush.Location = '725,112'
$btnCommitPush.Size = '105,27'
$btnCommitPush.Anchor = 'Top,Right'

$out = New-Object System.Windows.Forms.RichTextBox
$out.Location = '12,148'
$out.Size = '820,400'
$out.Anchor = 'Top,Bottom,Left,Right'
$out.Font = $mono
$out.ReadOnly = $true
$out.BackColor = [System.Drawing.Color]::FromArgb(30, 30, 30)
$out.ForeColor = [System.Drawing.Color]::Gainsboro
$out.WordWrap = $false

$form.Controls.AddRange(@($lblRepo, $txtRepo, $btnBrowse, $lblBranch, $flow, $lblMsg, $txtMsg, $btnCommit, $btnCommitPush, $out))
$actionButtons = @($btnStatus, $btnFetch, $btnPull, $btnRebase, $btnPush, $btnLog, $btnCommit, $btnCommitPush, $btnBrowse)

function Write-Out([string]$text, [System.Drawing.Color]$color = [System.Drawing.Color]::Gainsboro) {
    $out.SelectionStart = $out.TextLength
    $out.SelectionColor = $color
    $out.AppendText($text + "`r`n")
    $out.ScrollToCaret()
}

# Async reads + DoEvents keep the window responsive during long push/pull;
# GIT_TERMINAL_PROMPT=0 stops git from blocking on a hidden console prompt.
function Invoke-Git([string]$gitArgs, [switch]$Quiet) {
    $repo = $txtRepo.Text.Trim()
    if (-not $Quiet) { Write-Out "> git $gitArgs" ([System.Drawing.Color]::DeepSkyBlue) }
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = 'git'
    $psi.Arguments = $gitArgs
    $psi.WorkingDirectory = $repo
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [System.Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [System.Text.Encoding]::UTF8
    $psi.EnvironmentVariables['GIT_TERMINAL_PROMPT'] = '0'
    try { $p = [System.Diagnostics.Process]::Start($psi) }
    catch { Write-Out "Không chạy được git: $($_.Exception.Message)" ([System.Drawing.Color]::Tomato); return @{ Code = -1; Out = '' } }
    $so = $p.StandardOutput.ReadToEndAsync()
    $se = $p.StandardError.ReadToEndAsync()
    while (-not $p.HasExited) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 40 }
    $p.WaitForExit()
    $stdout = $so.Result.TrimEnd()
    $stderr = $se.Result.TrimEnd()
    if (-not $Quiet) {
        if ($stdout) { Write-Out $stdout }
        if ($stderr) {
            $c = if ($p.ExitCode -eq 0) { [System.Drawing.Color]::Khaki } else { [System.Drawing.Color]::Tomato }
            Write-Out $stderr $c
        }
        if ($p.ExitCode -eq 0) { Write-Out '✔ Xong' ([System.Drawing.Color]::LightGreen) }
        else { Write-Out "✘ Lỗi (exit $($p.ExitCode))" ([System.Drawing.Color]::Tomato) }
        Write-Out ''
    }
    return @{ Code = $p.ExitCode; Out = $stdout }
}

function Update-Branch {
    $r = Invoke-Git 'rev-parse --abbrev-ref HEAD' -Quiet
    if ($r.Code -ne 0) { $lblBranch.Text = 'Không phải git repo'; $lblBranch.ForeColor = [System.Drawing.Color]::Firebrick; return }
    $branch = $r.Out
    $up = Invoke-Git 'rev-parse --abbrev-ref --symbolic-full-name @{u}' -Quiet
    $info = "Branch: $branch"
    if ($up.Code -eq 0) {
        $ab = Invoke-Git 'rev-list --left-right --count HEAD...@{u}' -Quiet
        $parts = $ab.Out -split '\s+'
        if ($parts.Count -ge 2) { $info += "   →  $($up.Out)   ↑$($parts[0]) ↓$($parts[1])" }
    } else { $info += '   (chưa có upstream)' }
    $dirty = Invoke-Git 'status --porcelain' -Quiet
    if ($dirty.Out) { $info += "   •  $(@($dirty.Out -split "`n").Count) file thay đổi" }
    $lblBranch.Text = $info
    $lblBranch.ForeColor = [System.Drawing.Color]::DarkBlue
}

function Invoke-Action([scriptblock]$action) {
    if (-not (Test-Path -LiteralPath $txtRepo.Text.Trim())) { Write-Out 'Thư mục repo không tồn tại.' ([System.Drawing.Color]::Tomato); return }
    $actionButtons | ForEach-Object { $_.Enabled = $false }
    $form.Cursor = 'WaitCursor'
    try { & $action } finally {
        $actionButtons | ForEach-Object { $_.Enabled = $true }
        $form.Cursor = 'Default'
        Update-Branch
    }
}

function Invoke-Push {
    $up = Invoke-Git 'rev-parse --abbrev-ref --symbolic-full-name @{u}' -Quiet
    if ($up.Code -eq 0) { return Invoke-Git 'push' }
    return Invoke-Git 'push -u origin HEAD'
}

function Invoke-Commit {
    $msg = $txtMsg.Text.Trim()
    if (-not $msg) { Write-Out 'Nhập commit message trước.' ([System.Drawing.Color]::Tomato); return $false }
    $add = Invoke-Git 'add -A'
    if ($add.Code -ne 0) { return $false }
    $tmp = [System.IO.Path]::GetTempFileName()
    [System.IO.File]::WriteAllText($tmp, $msg, (New-Object System.Text.UTF8Encoding($false)))
    $c = Invoke-Git "commit -F `"$tmp`""
    Remove-Item -LiteralPath $tmp -ErrorAction SilentlyContinue
    if ($c.Code -eq 0) { $txtMsg.Clear(); return $true }
    return $false
}

function Confirm-Push {
    $ans = [System.Windows.Forms.MessageBox]::Show("Push lên remote?`n`n$($lblBranch.Text)", 'Xác nhận push', 'YesNo', 'Question')
    return $ans -eq 'Yes'
}

$btnBrowse.Add_Click({
    $dlg = New-Object System.Windows.Forms.FolderBrowserDialog
    $dlg.SelectedPath = $txtRepo.Text
    if ($dlg.ShowDialog() -eq 'OK') { $txtRepo.Text = $dlg.SelectedPath; Update-Branch }
})
$txtRepo.Add_Leave({ Update-Branch })
$btnStatus.Add_Click({ Invoke-Action { Invoke-Git 'status' | Out-Null } })
$btnFetch.Add_Click({ Invoke-Action { Invoke-Git 'fetch --all --prune' | Out-Null } })
$btnPull.Add_Click({ Invoke-Action { Invoke-Git 'pull' | Out-Null } })
$btnRebase.Add_Click({ Invoke-Action { Invoke-Git 'pull --rebase' | Out-Null } })
$btnPush.Add_Click({ if (Confirm-Push) { Invoke-Action { Invoke-Push | Out-Null } } })
$btnLog.Add_Click({ Invoke-Action { Invoke-Git 'log --oneline --graph --decorate -20' | Out-Null } })
$btnClear.Add_Click({ $out.Clear() })
$btnCommit.Add_Click({ Invoke-Action { Invoke-Commit | Out-Null } })
$btnCommitPush.Add_Click({
    if (-not (Confirm-Push)) { return }
    Invoke-Action { if (Invoke-Commit) { Invoke-Push | Out-Null } }
})

$form.Add_Shown({ Update-Branch; Write-Out 'Sẵn sàng. Chọn repo rồi bấm nút.' ([System.Drawing.Color]::Gray) })
[void]$form.ShowDialog()
