# Downloads the guitar model library pinned in tools/guitar-abi.json, checks the zip's SHA-256 and the library's
# SHA-256 against it, and unpacks it into vendor/guitar-abi/<platform>/ (docs/guitar-model.md).
#
#   pwsh tools/fetch-guitar-abi.ps1
#
# Uses `gh release download` when the GitHub CLI is signed in (or GH_TOKEN is set), and the public release URL
# otherwise. Does nothing when the pinned library is already in place.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$pin = Get-Content (Join-Path $PSScriptRoot "guitar-abi.json") -Raw | ConvertFrom-Json

if ($IsWindows) { $platform = "windows-x64" }
elseif ($IsMacOS) { $platform = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq "Arm64") { "macos-arm64" } else { "macos-x64" } }
else { $platform = "linux-x64" }

$entry = $pin.platforms.$platform
if (-not $entry) { throw "tools/guitar-abi.json has no library for $platform yet. See docs/guitar-model.md, Platforms." }

$dest = Join-Path $root "vendor/guitar-abi/$platform"
$lib = Join-Path $dest $entry.library
if ((Test-Path $lib) -and ((Get-FileHash $lib -Algorithm SHA256).Hash.ToLower() -eq $entry.sha256)) {
    Write-Host "The pinned library is already in $dest"
    exit 0
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("riffology-abi-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $tmp | Out-Null
try {
    $zip = Join-Path $tmp $entry.zip
    $usedGh = $false
    if (Get-Command gh -ErrorAction SilentlyContinue) {
        gh release download $pin.tag --repo $pin.repo --pattern $entry.zip --dir $tmp 2>$null
        $usedGh = ($LASTEXITCODE -eq 0) -and (Test-Path $zip)
    }
    if (-not $usedGh) {
        $url = "https://github.com/$($pin.repo)/releases/download/$($pin.tag)/$($entry.zip)"
        Invoke-WebRequest -Uri $url -OutFile $zip
    }
    $zipHash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
    if ($zipHash -ne $entry.zipSha256) { throw "$($entry.zip) has SHA-256 $zipHash; tools/guitar-abi.json pins $($entry.zipSha256)" }
    $unpacked = Join-Path $tmp "unpacked"
    Expand-Archive -Path $zip -DestinationPath $unpacked
    $libHash = (Get-FileHash (Join-Path $unpacked $entry.library) -Algorithm SHA256).Hash.ToLower()
    if ($libHash -ne $entry.sha256) { throw "$($entry.library) has SHA-256 $libHash; tools/guitar-abi.json pins $($entry.sha256)" }
    New-Item -ItemType Directory -Force $dest | Out-Null
    Copy-Item (Join-Path $unpacked "*") $dest -Force
    Write-Host "Fetched $($entry.library) ($($pin.modelVersion), ABI $($pin.abiVersion)) into $dest"
} finally {
    Remove-Item -Recurse -Force $tmp -Confirm:$false -ErrorAction SilentlyContinue
}
