<#
.SYNOPSIS
  Build, sign and package GuitarCraft for Windows.

.DESCRIPTION
  Produces, in $env:DIST (default: dist/release):
    guitarcraft-<version>-windows-<arch>.msi            installer (WiX v5), for one user or everyone
    guitarcraft-<version>-windows-<arch>-portable.zip   guitarcraft.exe + README + LICENSE

  The version comes from `version` in the root Cargo.toml (`cargo xtask version`), or from
  $env:GUITARCRAFT_VERSION for a one-off test build.

  The exe links the C runtime statically (+crt-static), so neither the MSI nor the portable zip
  needs the Visual C++ redistributable. Signing is delegated to sign.ps1 (skipped with a warning
  when no signing secrets are set).

  Adapted from storytold's packaging/windows/package.ps1. Only x64 is built for now.

  Needs: Rust (MSVC toolchain), the Windows SDK (rc.exe, signtool.exe), and WiX v5:
    dotnet tool install --global wix --version 5.0.2

.EXAMPLE
  powershell -File packaging/windows/package.ps1
  powershell -File packaging/windows/package.ps1 -SkipBuild
#>
param(
  [ValidateSet('x64')] [string] $Arch = 'x64',
  [switch] $SkipBuild
)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$WixVersion = '5.0.2'

function Invoke-Native([string] $What, [scriptblock] $Block) {
  Write-Output "==> $What"
  & $Block
  if ($LASTEXITCODE -ne 0) { throw "$What failed with exit code $LASTEXITCODE" }
}

# The version lives in one place: [package] version in the root Cargo.toml.
$Version = $env:GUITARCRAFT_VERSION
if (-not $Version) {
  $inPkg = $false
  foreach ($line in Get-Content (Join-Path $Root 'Cargo.toml')) {
    if ($line -match '^\s*\[') { $inPkg = ($line.Trim() -eq '[package]'); continue }
    if ($inPkg -and $line -match '^\s*version\s*=\s*"([^"]+)"') { $Version = $Matches[1]; break }
  }
}
if (-not $Version) { throw 'could not read [package] version from Cargo.toml' }
# MSI ProductVersion is numeric (major.minor.build); pre-release tags are dropped there.
$MsiVersion = ($Version -split '-')[0]

$Target = switch ($Arch) { 'x64' { 'x86_64-pc-windows-msvc' } }
$Dist = if ($env:DIST) { $env:DIST } else { Join-Path $Root 'dist\release' }
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
New-Item -ItemType Directory -Force -Path $Dist | Out-Null

Write-Output "GuitarCraft $Version for Windows $Arch ($Target)"

if (-not $SkipBuild) {
  # Static CRT: no VC++ redistributable needed. Scoped to the target so host build scripts and
  # proc-macros are unaffected.
  $flagVar = 'CARGO_TARGET_' + ($Target.ToUpper() -replace '-', '_') + '_RUSTFLAGS'
  [Environment]::SetEnvironmentVariable($flagVar, '-C target-feature=+crt-static')
  # Fail the build (rather than warn) if the icon/VERSIONINFO can't be embedded.
  $env:GUITARCRAFT_REQUIRE_WINRES = '1'
  # A test-run version override must reach the binary too, not just the file names.
  if ($env:GUITARCRAFT_VERSION) { Write-Warning "GUITARCRAFT_VERSION=$Version only names the files; the exe reports Cargo.toml's version" }
  Invoke-Native "cargo build ($Target)" { cargo build --release --locked -p guitarcraft --target $Target }
}

$Bin = Join-Path $TargetDir "$Target\release"
$Exe = Join-Path $Bin 'guitarcraft.exe'

# Check the PE header before packaging. Machine (COFF header) must match -Arch, and the
# subsystem (optional header) must be 2 = Windows GUI, so no console window opens with the app.
function Get-PeHeader([string] $Path) {
  $bytes = [System.IO.File]::ReadAllBytes($Path)
  $pe = [BitConverter]::ToInt32($bytes, 0x3C)
  return @{ Machine = [BitConverter]::ToUInt16($bytes, $pe + 4); Subsystem = [BitConverter]::ToUInt16($bytes, $pe + 0x5C) }
}
$Machine = switch ($Arch) { 'x64' { 0x8664 } }
$h = Get-PeHeader $Exe
if ($h.Machine -ne $Machine) { throw "guitarcraft.exe is for machine 0x$('{0:X}' -f $h.Machine), expected 0x$('{0:X}' -f $Machine) ($Arch)" }
if ($h.Subsystem -ne 2) { throw "guitarcraft.exe has PE subsystem $($h.Subsystem), expected 2 (Windows GUI)" }
Write-Output "ok guitarcraft.exe: $Arch, PE subsystem $($h.Subsystem)"
# The exe must report the version being packaged.
$fileVersion = (Get-Item $Exe).VersionInfo.ProductVersion
if (-not $env:GUITARCRAFT_VERSION -and $fileVersion -ne $Version) { throw "guitarcraft.exe reports version '$fileVersion', expected $Version (rebuild without -SkipBuild)" }

$Stage = Join-Path $TargetDir "windows-package\$Arch"
Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item $Exe $Stage
Copy-Item -LiteralPath (Join-Path $Root 'LICENSE') -Destination $Stage

& (Join-Path $PSScriptRoot 'sign.ps1') (Join-Path $Stage 'guitarcraft.exe')

# ---- MSI ---------------------------------------------------------------------------------------
$Msi = Join-Path $Dist "guitarcraft-$Version-windows-$Arch.msi"
Invoke-Native 'WiX UI extension' { wix extension add -g "WixToolset.UI.wixext/$WixVersion" }
Invoke-Native 'wix build' {
  wix build (Join-Path $PSScriptRoot 'guitarcraft.wxs') -arch $Arch -ext WixToolset.UI.wixext `
    -d "Version=$MsiVersion" -d "BinDir=$Stage" -d "IconPath=$(Join-Path $Root 'assets\app-icon\guitarcraft.ico')" `
    -o $Msi
}
# wix writes its debug symbols (.wixpdb) next to the MSI; keep them out of the release assets.
Remove-Item -Force -ErrorAction SilentlyContinue ([IO.Path]::ChangeExtension($Msi, '.wixpdb'))
& (Join-Path $PSScriptRoot 'sign.ps1') $Msi

# ---- portable zip ------------------------------------------------------------------------------
$Portable = Join-Path $TargetDir "windows-package\guitarcraft-$Version-windows-$Arch-portable"
Remove-Item -Recurse -Force $Portable -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Portable | Out-Null
Copy-Item (Join-Path $Stage 'guitarcraft.exe') $Portable
foreach ($f in 'README.md', 'LICENSE') {
  $p = Join-Path $Root $f
  if (Test-Path $p) { Copy-Item $p $Portable }
}
$Zip = Join-Path $Dist "guitarcraft-$Version-windows-$Arch-portable.zip"
Remove-Item -Force $Zip -ErrorAction SilentlyContinue
Compress-Archive -Path $Portable -DestinationPath $Zip

# Checksums for the release page.
$Sums = Join-Path $Dist 'SHA256SUMS.txt'
Get-Item $Msi, $Zip | ForEach-Object { "$((Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant())  $($_.Name)" } |
  Set-Content -Path $Sums -Encoding ascii

Get-Item $Msi, $Zip, $Sums | Format-Table Name, Length
