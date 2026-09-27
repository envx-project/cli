# Exercise the documented `irm ... | iex` path against the latest published release.
$ErrorActionPreference = 'Stop'
$env:ENVX_INSTALL_DIR = Join-Path ([IO.Path]::GetTempPath()) ("envx-bin-" + [Guid]::NewGuid())
$script = Get-Content -Raw (Join-Path $PSScriptRoot '..\install.ps1')
function UserPath { [Environment]::GetEnvironmentVariable('Path', 'User') }

$script | Invoke-Expression
& (Join-Path $env:ENVX_INSTALL_DIR 'envx.exe') --version
if ($LASTEXITCODE -ne 0) { throw 'installed envx did not run' }
if ((UserPath) -notlike "*$env:ENVX_INSTALL_DIR*") { throw 'user PATH not updated' }

# Reinstall while the old binary exists; the entry must not be duplicated.
$script | Invoke-Expression
if (Test-Path (Join-Path $env:ENVX_INSTALL_DIR 'envx.exe.old')) { throw 'stale .old left behind' }
if (((UserPath) -split ';' | Where-Object { $_ -eq $env:ENVX_INSTALL_DIR }).Count -ne 1) { throw 'PATH entry duplicated' }

$env:ENVX_UNINSTALL = '1'
$script | Invoke-Expression
$env:ENVX_UNINSTALL = ''
if (Test-Path (Join-Path $env:ENVX_INSTALL_DIR 'envx.exe')) { throw 'still installed' }
if ((UserPath) -like "*$env:ENVX_INSTALL_DIR*") { throw 'user PATH not cleaned' }
'PASS: Windows installer install, reinstall, uninstall'
