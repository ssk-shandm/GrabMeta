# Isolated NSIS resource test. Never install/uninstall the production identity.
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$overlay = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $root 'frontend\src-tauri\tauri.install-test.conf.json') | ConvertFrom-Json
if ($overlay.identifier -ne 'com.ssk-shandm.grabmeta.install-test' -or $overlay.productName -ne 'GrabMeta Install Test' -or $overlay.bundle.windows.webviewInstallMode.type -ne 'skip') { throw 'Unsafe test identity or WebView2 installation policy.' }
$base = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $root 'frontend\src-tauri\tauri.conf.json') | ConvertFrom-Json
$testKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\GrabMeta Install Test'
if (Test-Path -LiteralPath $testKey) { throw 'An isolated test installation already exists; refusing to overwrite it.' }
$artifact = Join-Path $root ('.grabmeta-install-resources-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
$target = Join-Path $artifact '中文 空格 安装目录'
New-Item -ItemType Directory -Path $artifact | Out-Null
$installer = Join-Path $root "frontend\src-tauri\target\release\bundle\nsis\GrabMeta Install Test_$($base.version)_x64-setup.exe"
$protectedKeys = @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\yt-dlp GUI','HKCU:\Software\ssk-shandm\yt-dlp GUI','HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Microsoft EdgeWebView','HKLM:\Software\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}','HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}')
function Get-ProtectedState {
    $state = [ordered]@{}
    foreach ($key in $protectedKeys) { $state[$key] = if (Test-Path -LiteralPath $key) { Get-ItemProperty -LiteralPath $key | Select-Object * -ExcludeProperty PSPath,PSParentPath,PSChildName,PSDrive,PSProvider } else { $null } }
    $config = Join-Path $env:APPDATA 'com.ssk-shandm.grabmeta\settings.json'
    $state.settings = if (Test-Path -LiteralPath $config) { (Get-FileHash -LiteralPath $config -Algorithm SHA256).Hash } else { $null }
    $state | ConvertTo-Json -Depth 10 -Compress
}
function Write-Json($path, $value) { [IO.File]::WriteAllText($path, ($value | ConvertTo-Json -Depth 10), [Text.UTF8Encoding]::new($false)) }
$before = Get-ProtectedState
[IO.File]::WriteAllText((Join-Path $artifact 'protected-before.json'), $before, [Text.UTF8Encoding]::new($false))
$result = [ordered]@{ testedAt = (Get-Date).ToUniversalTime().ToString('o'); installer = $installer; installerSha256 = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash; identity = $overlay.identifier; installed = $target }
try {
    $process = Start-Process -FilePath $installer -ArgumentList "/S /D=$target" -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Installer exit code: $($process.ExitCode)" }
    if (-not (Test-Path -LiteralPath (Join-Path $target 'GrabMeta-install-test.exe'))) { throw 'Test executable missing.' }
    $registered = [string](Get-ItemProperty -LiteralPath $testKey).InstallLocation
    if ([IO.Path]::GetFullPath($registered.Trim('"')).TrimEnd('\') -ne [IO.Path]::GetFullPath($target).TrimEnd('\')) { throw 'Test registry target mismatch.' }
    foreach ($name in 'yt-dlp.exe','ffmpeg.exe','ffprobe.exe') { if (Test-Path -LiteralPath (Join-Path $target "bin\$name")) { throw "Unexpected bundled tool: $name" } }
    $report = & node (Join-Path $root 'frontend\scripts\verify-installed-licenses.mjs') $target
    if ($LASTEXITCODE -ne 0) { throw 'Installed license verification failed.' }
    $result.resources = ($report -join "`n") | ConvertFrom-Json
    $result.status = 'passed'
} catch { $result.status = 'failed'; $result.error = $_.Exception.Message; throw }
finally {
    $uninstaller = Join-Path $target 'uninstall.exe'
    $resolved = [IO.Path]::GetFullPath($target)
    if (-not $resolved.StartsWith([IO.Path]::GetFullPath($artifact) + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe uninstall target outside this test directory.' }
    if (Test-Path -LiteralPath $uninstaller) {
        $process = Start-Process -FilePath $uninstaller -ArgumentList "/S _?=$target" -WindowStyle Hidden -Wait -PassThru
        $result.uninstallExitCode = $process.ExitCode
        $result.uninstalled = -not (Test-Path -LiteralPath (Join-Path $target 'GrabMeta-install-test.exe')) -and -not (Test-Path -LiteralPath $testKey)
    }
    $after = Get-ProtectedState
    [IO.File]::WriteAllText((Join-Path $artifact 'protected-after.json'), $after, [Text.UTF8Encoding]::new($false))
    $result.productionAndWebView2Unchanged = ($before -eq $after)
    if (-not $result.productionAndWebView2Unchanged -or -not $result.uninstalled) { $result.status = 'failed' }
    Write-Json (Join-Path $artifact 'result.json') $result
    $result | ConvertTo-Json -Depth 10
    Write-Output "Evidence retained: $artifact"
    if ($before -ne $after) { throw 'Production or WebView2 snapshot changed.' }
    if (-not $result.uninstalled) { throw 'Test uninstall incomplete.' }
}
