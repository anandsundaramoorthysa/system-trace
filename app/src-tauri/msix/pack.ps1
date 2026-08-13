<#
.SYNOPSIS
  Package the System Trace (Store edition) as an MSIX for Microsoft Store submission.

.DESCRIPTION
  Path A of docs/microsoft-store-submission.md: the Store signs the package for
  free, so we upload an unsigned MSIX and Microsoft re-signs it after
  certification. This script assembles the payload (the msstore-edition exe +
  the tile assets Tauri already generates) and runs MakeAppx.

  Run the Store build FIRST so the exe exists:
    cd app
    pnpm tauri build --features msstore --bundles none   # or: --bundles msi

  Then package (use the identity values from Partner Center -> Product identity):
    pwsh app/src-tauri/msix/pack.ps1 `
      -IdentityName "1234Publisher.SystemTrace" `
      -Publisher "CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX" `
      -PublisherDisplayName "Anand Sundaramoorthy SA"

  Output: app/src-tauri/target/msix/SystemTrace_<version>.msix  <-- upload this.

  For LOCAL install testing only, add -SelfSign to sign the package with a
  throwaway certificate (installing a self-signed MSIX also requires trusting
  that cert in an elevated session - see the doc). NEVER submit a self-signed
  package; upload the unsigned one and let the Store sign it.
#>
[CmdletBinding()]
param(
  # Package/Identity/Name from Partner Center (defaults to the reserved app's).
  [string]$IdentityName = "ANANDSUNDARAMOORTHYSA.SystemTrace",
  # Package/Identity/Publisher from Partner Center (must match exactly).
  [string]$Publisher = "CN=131F8D99-005E-4C53-87D7-752F5C65CA7C",
  # Package/Properties/PublisherDisplayName from Partner Center.
  [string]$PublisherDisplayName = "ANAND SUNDARAMOORTHY SA",
  # Four-part version; the Store reserves the 4th (revision) part, keep it 0.
  [string]$Version = "0.5.0.0",
  # Sign with a throwaway cert for local install testing (never for submission).
  [switch]$SelfSign
)

$ErrorActionPreference = "Stop"

# --- Resolve paths -----------------------------------------------------------
$here      = Split-Path -Parent $MyInvocation.MyCommand.Path      # .../src-tauri/msix
$srcTauri  = Split-Path -Parent $here                             # .../src-tauri
$exe       = Join-Path $srcTauri "target\release\system-trace.exe"
$iconsDir  = Join-Path $srcTauri "icons"
$outDir    = Join-Path $srcTauri "target\msix"
$stage     = Join-Path $outDir "stage"
$assets    = Join-Path $stage "Assets"
$msixOut   = Join-Path $outDir "SystemTrace_$Version.msix"

if (-not (Test-Path $exe)) {
  throw "Build the Store edition first (missing $exe). Run: cd app; pnpm tauri build --features msstore"
}

# --- Locate MakeAppx / SignTool from the newest Windows SDK ------------------
function Find-SdkTool([string]$name) {
  $roots = @("${env:ProgramFiles(x86)}\Windows Kits\10\bin", "$env:ProgramFiles\Windows Kits\10\bin")
  $tool = Get-ChildItem -Path $roots -Recurse -Filter $name -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match "\\x64\\" } |
    Sort-Object FullName -Descending | Select-Object -First 1
  if (-not $tool) { throw "$name not found. Install the Windows 10/11 SDK." }
  return $tool.FullName
}
$makeappx = Find-SdkTool "makeappx.exe"

# --- Assemble the payload ----------------------------------------------------
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $assets | Out-Null

Copy-Item $exe (Join-Path $stage "system-trace.exe") -Force

# Tile/logo assets referenced by AppxManifest.xml (Tauri already generates them).
$logos = @(
  "StoreLogo.png",
  "Square44x44Logo.png",
  "Square71x71Logo.png",
  "Square150x150Logo.png"
)
foreach ($l in $logos) {
  $srcLogo = Join-Path $iconsDir $l
  if (-not (Test-Path $srcLogo)) { throw "Missing logo asset: $srcLogo" }
  Copy-Item $srcLogo (Join-Path $assets $l) -Force
}

# Manifest with the identity values substituted in.
$manifest = Get-Content (Join-Path $here "AppxManifest.xml") -Raw
$manifest = $manifest.
  Replace("__IDENTITY_NAME__", $IdentityName).
  Replace("__PUBLISHER__", $Publisher).
  Replace("__PUBLISHER_DISPLAY_NAME__", $PublisherDisplayName).
  Replace("__VERSION__", $Version)
Set-Content -Path (Join-Path $stage "AppxManifest.xml") -Value $manifest -Encoding UTF8

# --- Pack --------------------------------------------------------------------
Write-Host "Packing MSIX -> $msixOut"
& $makeappx pack /d $stage /p $msixOut /o
if ($LASTEXITCODE -ne 0) { throw "makeappx failed with exit code $LASTEXITCODE" }

# --- Optional: self-sign for LOCAL testing only ------------------------------
if ($SelfSign) {
  $signtool = Find-SdkTool "signtool.exe"
  $pfx = Join-Path $outDir "local-test.pfx"
  $pwd = ConvertTo-SecureString "test" -AsPlainText -Force
  Write-Host "Creating throwaway self-signed cert (subject must match Publisher: $Publisher)"
  $cert = New-SelfSignedCertificate -Type Custom -Subject $Publisher `
    -KeyUsage DigitalSignature -FriendlyName "System Trace Local Test" `
    -CertStoreLocation "Cert:\CurrentUser\My" `
    -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}")
  Export-PfxCertificate -Cert $cert -FilePath $pfx -Password $pwd | Out-Null
  & $signtool sign /fd SHA256 /a /f $pfx /p "test" $msixOut
  if ($LASTEXITCODE -ne 0) { throw "signtool failed with exit code $LASTEXITCODE" }
  Write-Host "Signed for local testing. To trust + install (ELEVATED PowerShell):"
  Write-Host "  Import-Certificate -FilePath (export the .cer) -CertStoreLocation Cert:\LocalMachine\TrustedPeople"
  Write-Host "  Add-AppxPackage '$msixOut'"
}

Write-Host ""
Write-Host "Done: $msixOut"
if ($SelfSign) {
  Write-Warning "This package is SELF-SIGNED for local testing only. For the Store, run WITHOUT -SelfSign and upload the unsigned package (Microsoft signs it)."
}
