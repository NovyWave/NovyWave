param(
    [Parameter(Mandatory = $true)][string]$AppPath,
    [string]$Label = "Windows package"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $AppPath)) {
    throw "App executable not found: $AppPath"
}

$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("novywave-windows-smoke-" + [System.Guid]::NewGuid().ToString("N"))
$appData = Join-Path $workDir "AppData\Roaming"
$localAppData = Join-Path $workDir "AppData\Local"
$userProfile = Join-Path $workDir "UserProfile"
$tempDir = Join-Path $workDir "Temp"
$appStdoutLog = Join-Path $workDir "app.stdout.log"
$appStderrLog = Join-Path $workDir "app.stderr.log"
New-Item -ItemType Directory -Force -Path $appData, $localAppData, $userProfile, $tempDir | Out-Null

$port = Get-Random -Minimum 20000 -Maximum 60000
$bridgeUrl = "http://127.0.0.1:$port"
$timeoutSeconds = [int]($env:NOVYWAVE_SMOKE_TIMEOUT_SECONDS ?? "60")
$oldEnv = @{}
foreach ($name in @("NOVYWAVE_DESKTOP_TEST_PORT", "APPDATA", "LOCALAPPDATA", "USERPROFILE", "TEMP", "TMP")) {
    $oldEnv[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

function Restore-SmokeEnvironment {
    foreach ($entry in $oldEnv.GetEnumerator()) {
        [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, "Process")
    }
}

function Show-SmokeLogs {
    if (Test-Path $appStdoutLog) {
        Write-Host "=== $Label stdout log: $appStdoutLog ==="
        Get-Content $appStdoutLog -ErrorAction SilentlyContinue
    }
    if (Test-Path $appStderrLog) {
        Write-Host "=== $Label stderr log: $appStderrLog ==="
        Get-Content $appStderrLog -ErrorAction SilentlyContinue
    }
    $startupLog = Join-Path $localAppData "NovyWave\novywave-tauri-startup.log"
    if (Test-Path $startupLog) {
        Write-Host "=== $Label startup log: $startupLog ==="
        Get-Content $startupLog -ErrorAction SilentlyContinue
    }
}

function Get-BackendCwd {
    if (-not (Test-Path $appStdoutLog)) {
        return $null
    }
    $match = Select-String -Path $appStdoutLog -Pattern "BACKEND cwd = (.+)$" | Select-Object -Last 1
    if (-not $match) {
        return $null
    }
    return $match.Matches[0].Groups[1].Value.Trim()
}

function Invoke-SmokeGet {
    param([Parameter(Mandatory = $true)][string]$Path)
    Invoke-RestMethod -Uri "$bridgeUrl$Path" -TimeoutSec 5
}

function Invoke-SmokeEval {
    param([Parameter(Mandatory = $true)][string]$Expression)
    $body = @{ expression = $Expression } | ConvertTo-Json -Compress
    Invoke-RestMethod -Method Post -Uri "$bridgeUrl/eval" -Body $body -ContentType "application/json" -TimeoutSec 5
}

try {
    $env:NOVYWAVE_DESKTOP_TEST_PORT = [string]$port
    $env:APPDATA = $appData
    $env:LOCALAPPDATA = $localAppData
    $env:USERPROFILE = $userProfile
    $env:TEMP = $tempDir
    $env:TMP = $tempDir

    $process = Start-Process `
        -FilePath $AppPath `
        -RedirectStandardOutput $appStdoutLog `
        -RedirectStandardError $appStderrLog `
        -PassThru
    try {
        $deadline = (Get-Date).AddSeconds($timeoutSeconds)
        $health = $null
        while ((Get-Date) -lt $deadline) {
            if ($process.HasExited) {
                Show-SmokeLogs
                throw "$Label exited before desktop bridge became ready with code $($process.ExitCode)"
            }
            try {
                $health = Invoke-SmokeGet -Path "/health"
                if ($health.ok -and $health.windowFound) {
                    break
                }
            } catch {
            }
            Start-Sleep -Milliseconds 250
        }
        if (-not $health -or -not $health.ok -or -not $health.windowFound) {
            Show-SmokeLogs
            throw "$Label desktop bridge did not report a main window at $bridgeUrl. Last health: $($health | ConvertTo-Json -Compress)"
        }

        $deadline = (Get-Date).AddSeconds($timeoutSeconds)
        $config = $null
        while ((Get-Date) -lt $deadline) {
            $config = Invoke-SmokeGet -Path "/state/config-debug"
            if ($config.ok -and $config.value.configLoaded -and $config.value.serverReady) {
                break
            }
            Start-Sleep -Milliseconds 250
        }
        if (-not $config -or -not $config.ok -or -not $config.value.configLoaded -or -not $config.value.serverReady) {
            Show-SmokeLogs
            throw "$Label config state was not ready: $($config | ConvertTo-Json -Compress -Depth 8)"
        }

        $deadline = (Get-Date).AddSeconds($timeoutSeconds)
        $roots = $null
        while ((Get-Date) -lt $deadline) {
            $roots = Invoke-SmokeGet -Path "/state/file-picker-roots"
            if ($roots.ok -and @($roots.value.roots).Count -gt 0) {
                break
            }
            Start-Sleep -Milliseconds 250
        }
        if (-not $roots -or -not $roots.ok -or @($roots.value.roots).Count -eq 0) {
            Show-SmokeLogs
            throw "$Label platform roots did not load: $($roots | ConvertTo-Json -Compress -Depth 8)"
        }

        $inputs = Invoke-SmokeEval -Expression "Array.from(document.querySelectorAll('input')).map(input => input.value)"
        $values = @($inputs.value)
        if (-not $inputs.ok) {
            Show-SmokeLogs
            throw "$Label failed to read workspace inputs: $($inputs | ConvertTo-Json -Compress -Depth 8)"
        }
        if ($values | Where-Object { [string]$_ -like "*Loading workspace*" }) {
            Show-SmokeLogs
            throw "$Label workspace input is still loading: $($values -join ', ')"
        }
        $workspaceReady = $values | Where-Object {
            $value = [string]$_
            $value -eq "No workspace" -or $value -like "Default (*)"
        }
        if (-not $workspaceReady) {
            Show-SmokeLogs
            throw "$Label expected fresh run to show 'No workspace' or packaged default workspace, got: $($values -join ', ')"
        }

        $console = Invoke-SmokeGet -Path "/state/console-log"
        if (-not $console.ok) {
            Show-SmokeLogs
            throw "$Label failed to read startup console log: $($console | ConvertTo-Json -Compress -Depth 8)"
        }
        $blockingConsoleErrors = @($console.value | Where-Object {
            $message = [string]($_.message ?? $_)
            $expectedCiConfigWarning =
                $message -like "*No platform config directory is available for global workspace history*"
            (-not $expectedCiConfigWarning) -and
                ($message -match "(?i)(Config error|Workspace load timeout|panic|failed to start)")
        })
        if ($blockingConsoleErrors.Count -gt 0) {
            Show-SmokeLogs
            throw "$Label startup console contained blocking errors: $($blockingConsoleErrors | ConvertTo-Json -Compress -Depth 8)"
        }

        $defaultWorkspaceConfig = Join-Path $appData "novywave\default-workspace\.novywave"
        if (Test-Path $defaultWorkspaceConfig) {
            Show-SmokeLogs
            throw "$Label created workspace config in default workspace: $defaultWorkspaceConfig"
        }

        $backendCwd = Get-BackendCwd
        if ($backendCwd) {
            $backendCwdConfig = Join-Path $backendCwd ".novywave"
            if (Test-Path $backendCwdConfig) {
                Show-SmokeLogs
                throw "$Label created workspace config in backend cwd: $backendCwdConfig"
            }
        }

        Write-Host "Windows Tauri package smoke passed"
        Write-Host "  label: $Label"
        Write-Host "  app: $AppPath"
        Write-Host "  bridge: $bridgeUrl"
        Write-Host "  config: $($config | ConvertTo-Json -Compress -Depth 8)"
        Write-Host "  roots: $($roots | ConvertTo-Json -Compress -Depth 8)"
    } finally {
        if ($process -and -not $process.HasExited) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            $process.WaitForExit(5000) | Out-Null
        }
    }
} finally {
    Restore-SmokeEnvironment
    if ($env:NOVYWAVE_SMOKE_KEEP_LOGS) {
        Write-Host "Windows smoke logs kept at: $workDir"
    } else {
        Remove-Item -Recurse -Force $workDir -ErrorAction SilentlyContinue
    }
}
