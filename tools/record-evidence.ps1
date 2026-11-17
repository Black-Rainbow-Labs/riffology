# Records the Riffology window to an MP4 for a pull request's Evidence section.
#
#   pwsh tools/record-evidence.ps1                                  # replays a GuitarSet fixture, 14 seconds
#   pwsh tools/record-evidence.ps1 -Replay my-riff.wav -Seconds 30  # your own recording
#   pwsh tools/record-evidence.ps1 -Live -Seconds 20                # the live audio input: play while it records
#   pwsh tools/record-evidence.ps1 -AppArgs '--chart','songs/demo.json'
#
# It builds the release app, starts it, records the screen region of its window with ffmpeg (gdigrab), and for a replay puts the replayed
# file's audio under the video, lined up with the moment the replay started (from the app's trace). The video, the
# trace and a summary of the trace go to _agent_output/evidence/<name>/. Needs ffmpeg on the PATH.
param(
    [string]$Replay = "tests/fixtures/guitar/05_Rock1-130-A_solo.48k.wav",
    [switch]$Live,
    [double]$Seconds = 14,
    [string]$Name = "",
    [string[]]$AppArgs = @()
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) { throw "ffmpeg is not on the PATH" }
if (-not $Name) { $Name = (Get-Date -Format "yyyyMMdd-HHmmss") }
$out = Join-Path $root "_agent_output/evidence/$Name"
New-Item -ItemType Directory -Force $out | Out-Null
$video = Join-Path $out "window.mp4"
$final = Join-Path $out "riffology-evidence.mp4"
$trace = Join-Path $out "trace.json"

cargo build --release --manifest-path (Join-Path $root "Cargo.toml") -p riffology-app
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
$exe = Join-Path $root "target/release/riffology.exe"
if (-not (Test-Path $exe)) { $exe = Join-Path $root "target/release/riffology" }

$delay = 3.0
$runArgs = @("--trace", $trace, "--exit-after", ($Seconds + $delay + 1).ToString([Globalization.CultureInfo]::InvariantCulture)) + $AppArgs
if (-not $Live) {
    $wav = (Resolve-Path (Join-Path $root $Replay) -ErrorAction SilentlyContinue)
    if (-not $wav) { $wav = Resolve-Path $Replay }
    $runArgs = @("--replay", $wav.Path, "--replay-delay", "$delay") + $runArgs
}
$app = Start-Process -FilePath $exe -ArgumentList $runArgs -PassThru

# Wait for the window, then record it.
$deadline = (Get-Date).AddSeconds(30)
while (-not ((Get-Process -Id $app.Id -ErrorAction SilentlyContinue).MainWindowTitle -eq "Riffology")) {
    if ((Get-Date) -gt $deadline) { throw "the Riffology window did not open" }
    Start-Sleep -Milliseconds 100
}
# A GPU window's swapchain cannot be read through its window handle, so record the desktop region the window covers,
# with the window brought to the front.
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class RiffWin {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int size);
}
"@
[void][RiffWin]::SetProcessDPIAware()
$hwnd = (Get-Process -Id $app.Id).MainWindowHandle
[void][RiffWin]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 500
$r = New-Object RiffWin+RECT
# DWMWA_EXTENDED_FRAME_BOUNDS (9) is the visible window without its invisible resize border.
[void][RiffWin]::DwmGetWindowAttribute($hwnd, 9, [ref]$r, 16)
$w = ($r.Right - $r.Left) - (($r.Right - $r.Left) % 2); $h = ($r.Bottom - $r.Top) - (($r.Bottom - $r.Top) % 2)
$recStart = [DateTimeOffset]::Now.ToUnixTimeMilliseconds()
$dur = ($Seconds + $delay).ToString([Globalization.CultureInfo]::InvariantCulture)
ffmpeg -hide_banner -loglevel error -y -f gdigrab -framerate 60 -offset_x $r.Left -offset_y $r.Top -video_size "$($w)x$($h)" -t $dur -i desktop -c:v libx264 -preset veryfast -crf 18 -pix_fmt yuv420p $video
if ($LASTEXITCODE -ne 0) { throw "ffmpeg recording failed" }
$app.WaitForExit()
if (-not (Test-Path $trace)) { throw "the app wrote no trace" }

$t = Get-Content $trace -Raw | ConvertFrom-Json
if (-not $Live -and $t.replay_started_unix_ms) {
    $offset = (($t.replay_started_unix_ms - $recStart) / 1000.0).ToString("0.000", [Globalization.CultureInfo]::InvariantCulture)
    ffmpeg -hide_banner -loglevel error -y -i $video -itsoffset $offset -i $wav.Path -map 0:v -map 1:a -c:v copy -c:a aac -b:a 192k $final
    if ($LASTEXITCODE -ne 0) { throw "ffmpeg mux failed" }
} else {
    Copy-Item $video $final -Force
}

$ons = 0; foreach ($f in $t.frames) { foreach ($e in $f.events) { if ($e.is_on) { $ons++ } } }
$summary = @"
Riffology evidence
input: $($t.input)
model: $($t.model_version)
note ons heard: $ons
frames drawn: $($t.frame_count)
events dropped: $($t.dropped)
largest audio block time in Riffology's input code: $([math]::Round($t.max_block_ms, 3)) ms
video: $final
"@
$summary | Set-Content (Join-Path $out "summary.txt")
Write-Host $summary
