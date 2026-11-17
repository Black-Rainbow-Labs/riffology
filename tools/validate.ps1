# Everything a pull request must pass, in one command (docs/testing.md): formatting, clippy, the unit tests, the input
# tests and the app end to end test with the real guitar model library, the documentation check, and the pull request
# check's own tests. Stops at the first failure.
#
#   pwsh tools/validate.ps1
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root "Cargo.toml"

<#
.SYNOPSIS
Runs one step and stops the script when it fails.
#>
function Invoke-Step([string]$name, [scriptblock]$run) {
    Write-Host "== $name" -ForegroundColor Cyan
    & $run
    if ($LASTEXITCODE -ne 0) { throw "$name failed" }
}

Invoke-Step "guitar model library" { pwsh (Join-Path $PSScriptRoot "fetch-guitar-abi.ps1") }
Invoke-Step "cargo fmt" { cargo fmt --manifest-path $manifest --all -- --check }
Invoke-Step "cargo clippy" { cargo clippy --manifest-path $manifest --workspace --all-targets -- -D warnings }
Invoke-Step "unit tests" { cargo test --manifest-path $manifest -p riffology-core -p riffology-song -p riffology-fretboard -p riffology-game }
Invoke-Step "input tests with the real model" { cargo test --release --manifest-path $manifest -p riffology-input }
Invoke-Step "app end to end" { cargo test --release --manifest-path $manifest -p riffology-app }
Invoke-Step "documentation" { node (Join-Path $PSScriptRoot "doc-style/check.mjs") }
Invoke-Step "pull request check tests" { node --test (Join-Path $root ".github/scripts/check-pull-request.test.mjs") }
Write-Host "Everything passed." -ForegroundColor Green
