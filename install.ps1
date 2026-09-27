# Install a published envx release on Windows.
#
#   powershell -c "irm https://raw.githubusercontent.com/envx-project/cli/main/install.ps1 | iex"
#
# Installs per user (no administrator rights) and adds the folder to the user PATH.
# Environment: ENVX_VERSION, ENVX_INSTALL_DIR, ENVX_PLATFORM (msvc or gnu),
# ENVX_BASE_URL, ENVX_NO_MODIFY_PATH=1, ENVX_UNINSTALL=1.
# Compatible with Windows PowerShell 5.1 and PowerShell 7.

& {
  $ErrorActionPreference = 'Stop'
  # Invoke-WebRequest renders progress very slowly on Windows PowerShell 5.1.
  $ProgressPreference = 'SilentlyContinue'
  # Windows PowerShell 5.1 may default to TLS 1.0.
  [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

  function Fail($message) { throw "envx: $message" }
  $onWindows = $PSVersionTable.PSEdition -eq 'Desktop' -or $IsWindows

  $installDir = $env:ENVX_INSTALL_DIR
  if (-not $installDir) {
    if (-not $env:LOCALAPPDATA) { Fail 'LOCALAPPDATA is not set; set ENVX_INSTALL_DIR' }
    $installDir = Join-Path $env:LOCALAPPDATA 'Programs\envx'
  }
  $exe = Join-Path $installDir 'envx.exe'

  function Update-UserPath([bool]$add) {
    if (-not $onWindows -or $env:ENVX_NO_MODIFY_PATH -eq '1') { return }
    $current = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @(if ($current) { $current -split ';' | Where-Object { $_ } })
    $present = $entries | Where-Object { $_.TrimEnd('\') -ieq $installDir.TrimEnd('\') }
    if ($add -and -not $present) {
      [Environment]::SetEnvironmentVariable('Path', (($entries + $installDir) -join ';'), 'User')
      $env:Path = "$env:Path;$installDir"
      Write-Host "Added $installDir to your user PATH. Restart other terminals to pick it up."
    } elseif (-not $add -and $present) {
      $kept = $entries | Where-Object { $_.TrimEnd('\') -ine $installDir.TrimEnd('\') }
      [Environment]::SetEnvironmentVariable('Path', ($kept -join ';'), 'User')
    }
  }

  if ($env:ENVX_UNINSTALL -eq '1') {
    if (Test-Path -LiteralPath $exe) { Remove-Item -LiteralPath $exe -Force }
    Remove-Item -LiteralPath "$exe.old" -Force -ErrorAction SilentlyContinue
    Update-UserPath $false
    Write-Host 'Removed envx'
    return
  }

  $baseUrl = if ($env:ENVX_BASE_URL) { $env:ENVX_BASE_URL.TrimEnd('/') } else { 'https://github.com/envx-project/cli/releases' }
  if (-not $baseUrl.StartsWith('https://')) { Fail 'Release downloads require an HTTPS base URL' }

  $arch = $env:PROCESSOR_ARCHITECTURE
  if ($arch -and $arch -notin @('AMD64', 'ARM64')) {
    # ARM64 Windows runs the x86_64 build under emulation; 32-bit has no build.
    Fail "No published build for $arch Windows; see https://github.com/envx-project/cli/releases"
  }
  $platform = if ($env:ENVX_PLATFORM) { $env:ENVX_PLATFORM } else { 'msvc' }
  if ($platform -notin @('msvc', 'gnu')) { Fail 'ENVX_PLATFORM must be msvc or gnu' }
  $target = "x86_64-pc-windows-$platform"

  $version = $env:ENVX_VERSION
  if (-not $version) {
    # /latest redirects to /tag/vX.Y.Z; read the tag without following it.
    $request = [Net.HttpWebRequest]::Create("$baseUrl/latest")
    $request.Method = 'HEAD'
    $request.AllowAutoRedirect = $false
    $response = $request.GetResponse()
    try { $location = $response.Headers['Location'] } finally { $response.Close() }
    if (-not $location) { Fail 'Could not determine the latest release' }
    $version = ($location -split '/')[-1]
  }
  $version = $version -replace '^v', ''
  if ($version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$') { Fail "Invalid release version: $version" }

  $asset = "envx-$version-$target.zip"
  $url = "$baseUrl/download/v$version/$asset"
  $work = Join-Path ([IO.Path]::GetTempPath()) ("envx-install-" + [Guid]::NewGuid())
  New-Item -ItemType Directory -Path $work | Out-Null
  try {
    $archive = Join-Path $work $asset
    try { Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $archive } catch { Fail "Release download failed: $url" }

    # Older published releases predate checksum sidecars. Never silently
    # downgrade verification for a new release or on a mismatch.
    $checksum = Join-Path $work 'checksum'
    $haveChecksum = $true
    try { Invoke-WebRequest -UseBasicParsing -Uri "$url.sha256" -OutFile $checksum } catch { $haveChecksum = $false }
    if ($haveChecksum) {
      $expected = ((Get-Content -LiteralPath $checksum -TotalCount 1) -split '\s+')[0]
      if ($expected -notmatch '^[0-9a-fA-F]{64}$') { Fail 'Invalid SHA256 checksum' }
      $actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
      if ($actual -ine $expected) { Fail 'Release checksum mismatch; installation aborted' }
    } else {
      $parts = $version -split '\.'
      if ([int]$parts[0] -lt 2 -or ([int]$parts[0] -eq 2 -and [int]$parts[1] -le 13)) {
        Write-Warning "Legacy release $version has no SHA256 sidecar; relying on HTTPS transport."
      } else {
        Fail 'Release checksum unavailable; installation aborted'
      }
    }

    # Extract only envx.exe; never unpack archive paths into the install folder.
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $staged = Join-Path $work 'envx.exe'
    $zip = [IO.Compression.ZipFile]::OpenRead($archive)
    try {
      $entry = $zip.Entries | Where-Object { $_.FullName -eq 'envx.exe' } | Select-Object -First 1
      if (-not $entry -or $entry.Length -eq 0) { Fail 'Release archive contains no executable' }
      [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $staged, $true)
    } finally { $zip.Dispose() }

    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    # A running envx.exe cannot be overwritten, but it can be renamed aside.
    Remove-Item -LiteralPath "$exe.old" -Force -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $exe) { Move-Item -LiteralPath $exe -Destination "$exe.old" -Force }
    Move-Item -LiteralPath $staged -Destination $exe -Force
    Remove-Item -LiteralPath "$exe.old" -Force -ErrorAction SilentlyContinue
  } finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
  }

  Write-Host "Installed envx $version to $exe"
  Update-UserPath $true
}
