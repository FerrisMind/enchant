# Capture demo PNG/GIF for enchant-iced examples (README assets).
# Pipeline: launch → dark-mode toggle → optional mouse sweep → frames → GIF or PNG.
#
# Usage:
#   pwsh tools/visual-verify/capture-example-demos.ps1
#   pwsh tools/visual-verify/capture-example-demos.ps1 -Only magic_card,particles
#   pwsh tools/visual-verify/capture-example-demos.ps1 -SkipExisting

param(
    [string[]]$Only = @(),
    [string[]]$Exclude = @(),
    [string[]]$ForceGif = @(),
    [string[]]$ForcePng = @(),
    [string]$OutDir = "",
    [string]$BinDir = "",
    [switch]$SkipExisting,
    [switch]$Rebuild,
    [switch]$KeepLight,
    [int]$WarmupMs = 1200,
    [int]$ThemeSettleMs = 2200,
    [int]$FrameCount = 28,
    [int]$FrameIntervalMs = 70,
    [double]$MotionThreshold = 1.2,
    [int]$MaxWidth = 720,
    [int]$MinWindowW = 500,
    [int]$MinWindowH = 260,
    [int]$LaunchTimeoutSec = 45
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
if (-not $OutDir) {
    $OutDir = Join-Path $RepoRoot "crates/enchant-iced/docs/demos"
}
if (-not $BinDir) {
    $BinDir = Join-Path $RepoRoot "target/debug/examples"
}
$TmpRoot = Join-Path $RepoRoot "target/demo-capture-tmp"
$LogPath = Join-Path $RepoRoot "logs/demo-capture.log"
$ManifestPath = Join-Path $OutDir "manifest.json"

# Continuous / looping motion — prefer GIF even when pixel delta is subtle.
$DefaultForceGif = @(
    "animated_beam", "animated_beam_multiple_outputs", "animated_circular_progress_bar",
    "animated_gradient_text", "animated_grid_pattern", "animated_list_demo",
    "animated_shiny_text", "animated_subscribe_button", "animated_theme_toggler",
    "aurora_text", "avatar_circles", "backlight", "blur_fade", "border_beam", "border_track",
    "bot_avatar", "comic_text", "confetti", "cool_mode", "dia_text_reveal", "dock",
    "flickering_grid", "glare_hover", "globe", "glyph_matrix", "gooey", "hero_video_dialog",
    "highlighter", "hyper_text", "icon_cloud", "interactive_grid_pattern",
    "interactive_hover_button", "kinetic_text", "lens", "light_rays", "line_shadow_text",
    "magic_card", "marquee", "metal_fx", "meteors", "morphing_text", "motion_grid",
    "neon_gradient_card", "noise_texture", "number_ticker", "orbiting_circles", "particles",
    "pixel_image", "pointer", "progressive_blur", "pulsating_button", "rainbow_button",
    "retro_grid", "ripple", "ripple_button", "scroll_based_velocity", "scroll_progress",
    "shimmer_button", "shine_border", "shiny_button", "smooth_cursor", "sparkles_text",
    "spinning_text", "terminal", "text_3d_flip", "text_animate", "text_reveal",
    "thinking_orb", "typing_animation", "video_text", "voice_glow", "warp_background",
    "word_rotate"
)

# Pointer-driven demos — sweep mouse across the client while recording.
$HoverSweep = @(
    "magic_card", "glare_hover", "lens", "pointer", "smooth_cursor",
    "interactive_hover_button", "dock", "cool_mode", "interactive_grid_pattern",
    "particles", "icon_cloud"
)

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
New-Item -ItemType Directory -Force -Path $TmpRoot | Out-Null
New-Item -ItemType Directory -Force -Path (Split-Path $LogPath) | Out-Null

function Write-Log([string]$msg) {
    $line = "{0}`t{1}" -f (Get-Date -Format "o"), $msg
    Add-Content -LiteralPath $LogPath -Value $line -Encoding UTF8
    Write-Host $msg
}

function Expand-NameList([string[]]$items) {
    $out = @()
    foreach ($item in $items) {
        if ([string]::IsNullOrWhiteSpace($item)) { continue }
        foreach ($part in ($item -split "[,\s]+")) {
            if (-not [string]::IsNullOrWhiteSpace($part)) {
                $out += $part.Trim()
            }
        }
    }
    return $out
}

Add-Type -AssemblyName System.Drawing

$native = @"
using System;
using System.Text;
using System.Runtime.InteropServices;

public static class DemoCap5 {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X; public int Y; }

    [StructLayout(LayoutKind.Sequential)]
    public struct INPUT {
        public uint type;
        public MOUSEINPUT mi;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct MOUSEINPUT {
        public int dx, dy;
        public uint mouseData, dwFlags, time;
        public IntPtr dwExtraInfo;
    }

    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    public const uint PW_RENDERFULLCONTENT = 2;
    public const int SW_RESTORE = 9;
    public const uint INPUT_MOUSE = 0;
    public const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
    public const uint MOUSEEVENTF_LEFTUP = 0x0004;
    public const uint MOUSEEVENTF_MOVE = 0x0001;
    public const uint MOUSEEVENTF_ABSOLUTE = 0x8000;

    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref POINT lpPoint);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdcBlt, uint nFlags);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
    [DllImport("user32.dll")] public static extern uint SendInput(uint nInputs, INPUT[] pInputs, int cbSize);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int nIndex);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr hWnd, StringBuilder lpClassName, int nMaxCount);

    static bool IsConsoleLike(IntPtr hWnd) {
        var cls = new StringBuilder(256);
        GetClassName(hWnd, cls, cls.Capacity);
        string c = cls.ToString();
        if (c == "ConsoleWindowClass" || c == "PseudoConsoleWindow") return true;
        var title = new StringBuilder(512);
        GetWindowText(hWnd, title, title.Capacity);
        string t = title.ToString();
        if (t.EndsWith(".exe", StringComparison.OrdinalIgnoreCase)) return true;
        if (t.StartsWith("Select ", StringComparison.OrdinalIgnoreCase)) return true;
        if (t.StartsWith("Выбрать ", StringComparison.Ordinal)) return true;
        return false;
    }

    public static IntPtr FindAppWindow(int pid, int minW, int minH) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((hWnd, lParam) => {
            if (!IsWindowVisible(hWnd)) return true;
            uint windowPid;
            GetWindowThreadProcessId(hWnd, out windowPid);
            if ((int)windowPid != pid) return true;
            if (IsConsoleLike(hWnd)) return true;
            RECT r;
            if (!GetWindowRect(hWnd, out r)) return true;
            int w = r.Right - r.Left;
            int h = r.Bottom - r.Top;
            if (w < minW || h < minH) return true;
            found = hWnd;
            return false;
        }, IntPtr.Zero);
        return found;
    }

    public const int SW_HIDE = 0;

    public static void HideConsoleWindows(int pid) {
        EnumWindows((hWnd, lParam) => {
            uint windowPid;
            GetWindowThreadProcessId(hWnd, out windowPid);
            if ((int)windowPid != pid) return true;
            if (IsConsoleLike(hWnd)) {
                ShowWindow(hWnd, SW_HIDE);
            }
            return true;
        }, IntPtr.Zero);
    }

    public static void MoveTo(int x, int y) {
        SetCursorPos(x, y);
    }

    public static void ClickAt(int x, int y) {
        SetCursorPos(x, y);
        INPUT[] down = new INPUT[1];
        down[0].type = INPUT_MOUSE;
        down[0].mi.dwFlags = MOUSEEVENTF_LEFTDOWN;
        SendInput(1, down, Marshal.SizeOf(typeof(INPUT)));
        INPUT[] up = new INPUT[1];
        up[0].type = INPUT_MOUSE;
        up[0].mi.dwFlags = MOUSEEVENTF_LEFTUP;
        SendInput(1, up, Marshal.SizeOf(typeof(INPUT)));
    }

    public static POINT ClientPoint(IntPtr hwnd, double fx, double fy) {
        RECT client;
        GetClientRect(hwnd, out client);
        POINT pt = new POINT();
        pt.X = (int)(client.Left + (client.Right - client.Left) * fx);
        pt.Y = (int)(client.Top + (client.Bottom - client.Top) * fy);
        ClientToScreen(hwnd, ref pt);
        return pt;
    }
}
"@

try { Add-Type -TypeDefinition $native -ErrorAction Stop } catch {
    if (-not ("DemoCap5" -as [type])) { throw }
}

function Get-ExampleNames {
    $dir = Join-Path $RepoRoot "crates/enchant-iced/examples"
    Get-ChildItem -LiteralPath $dir -Filter "*.rs" |
        Where-Object { -not $_.Name.StartsWith("_") } |
        ForEach-Object { $_.BaseName } |
        Sort-Object
}

function Stop-ExampleProcess([string]$name) {
    Get-Process -Name $name -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
}

function Capture-Hwnd([IntPtr]$hwnd, [string]$path) {
    $rect = New-Object DemoCap5+RECT
    [void][DemoCap5]::GetWindowRect($hwnd, [ref]$rect)
    $w = [Math]::Max(1, $rect.Right - $rect.Left)
    $h = [Math]::Max(1, $rect.Bottom - $rect.Top)
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    try {
        $g.CopyFromScreen($rect.Left, $rect.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
        $mid = $bmp.GetPixel([int]($w / 2), [int]($h / 2))
        $dark = (($mid.R + $mid.G + $mid.B) -lt 45)
        if ($dark) {
            # Re-check: very dark mid can be legitimate dark theme. Only PrintWindow
            # when the whole capture looks empty (near-black corners too).
            $c1 = $bmp.GetPixel(8, 8)
            $c2 = $bmp.GetPixel([Math]::Max(0, $w - 9), [Math]::Max(0, $h - 9))
            $blank = (($c1.R + $c1.G + $c1.B) -lt 30) -and (($c2.R + $c2.G + $c2.B) -lt 30)
            if ($blank) {
                $hdc = $g.GetHdc()
                try {
                    [void][DemoCap5]::PrintWindow($hwnd, $hdc, [DemoCap5]::PW_RENDERFULLCONTENT)
                } finally {
                    $g.ReleaseHdc($hdc)
                }
            }
        }
        $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $g.Dispose()
        $bmp.Dispose()
    }
    return @{ w = $w; h = $h }
}

function Compare-FramePair([string]$aPath, [string]$bPath) {
    $a = [System.Drawing.Bitmap]::FromFile($aPath)
    $b = [System.Drawing.Bitmap]::FromFile($bPath)
    try {
        $w = [Math]::Min($a.Width, $b.Width)
        $h = [Math]::Min($a.Height, $b.Height)
        if ($w -lt 8 -or $h -lt 8) { return 0.0 }
        $sum = 0.0
        $n = 0
        $step = 5
        for ($y = 0; $y -lt $h; $y += $step) {
            for ($x = 0; $x -lt $w; $x += $step) {
                $pa = $a.GetPixel($x, $y)
                $pb = $b.GetPixel($x, $y)
                $sum += [Math]::Abs(([int]$pa.R) - ([int]$pb.R))
                $sum += [Math]::Abs(([int]$pa.G) - ([int]$pb.G))
                $sum += [Math]::Abs(([int]$pa.B) - ([int]$pb.B))
                $n += 3
            }
        }
        if ($n -eq 0) { return 0.0 }
        return ($sum / $n)
    } finally {
        $a.Dispose()
        $b.Dispose()
    }
}

function Get-FrameMotion([string[]]$paths) {
    if ($paths.Count -lt 2) { return 0.0 }
    $maxDelta = 0.0
    $lastIdx = $paths.Count - 1
    $indices = [System.Collections.Generic.List[int[]]]::new()
    $indices.Add([int[]]@(0, $lastIdx))
    $stride = [Math]::Max(1, [int]($paths.Count / 6))
    for ($i = 0; $i -lt $lastIdx; $i += $stride) {
        $indices.Add([int[]]@($i, ($i + 1)))
    }
    foreach ($pair in $indices) {
        $delta = Compare-FramePair $paths[$pair[0]] $paths[$pair[1]]
        if ($delta -gt $maxDelta) { $maxDelta = $delta }
    }
    return $maxDelta
}

function New-DemoGif([string]$frameDir, [string]$outGif, [int]$maxW) {
    $ffmpeg = Get-Command ffmpeg -ErrorAction Stop
    $palette = Join-Path $frameDir "palette.png"
    $fps = [Math]::Max(1, [int](1000 / $FrameIntervalMs))
    $scale = "scale=min($maxW\,iw):-1:flags=lanczos"
    & $ffmpeg.Source -y -hide_banner -loglevel error `
        -framerate $fps -i (Join-Path $frameDir "frame_%03d.png") `
        -vf "$scale,palettegen=stats_mode=diff" $palette
    if ($LASTEXITCODE -ne 0) { throw "ffmpeg palettegen failed" }
    & $ffmpeg.Source -y -hide_banner -loglevel error `
        -framerate $fps -i (Join-Path $frameDir "frame_%03d.png") -i $palette `
        -lavfi "$scale [x]; [x][1:v] paletteuse=dither=bayer:bayer_scale=5" $outGif
    if ($LASTEXITCODE -ne 0) { throw "ffmpeg paletteuse failed" }
}

function Ensure-Binary([string]$name) {
    $exe = Join-Path $BinDir "$name.exe"
    if ((Test-Path -LiteralPath $exe) -and -not $Rebuild) {
        return $exe
    }
    Write-Log "BUILD`t$name"
    Push-Location $RepoRoot
    try {
        $cargoArgs = @("build", "-p", "enchant-iced", "--example", $name)
        if ($name -eq "voice_glow") {
            $cargoArgs += @("--features", "microphone")
        }
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed for $name" }
    } finally {
        Pop-Location
    }
    if (-not (Test-Path -LiteralPath $exe)) {
        throw "binary missing after build: $exe"
    }
    return $exe
}

function Invoke-DarkModeToggle([IntPtr]$hwnd) {
    # Single click on the shared theme toggler (top-right). Multiple clicks
    # restart the circle-reveal and poison motion detection / GIF content.
    $pt = [DemoCap5]::ClientPoint($hwnd, 0.955, 0.065)
    [DemoCap5]::ClickAt($pt.X, $pt.Y)
}

function Invoke-MouseSweep([IntPtr]$hwnd, [int]$steps) {
    for ($i = 0; $i -le $steps; $i++) {
        $t = $i / [double]$steps
        $fx = 0.18 + (0.64 * $t)
        $fy = 0.28 + (0.22 * [Math]::Sin($t * [Math]::PI))
        $pt = [DemoCap5]::ClientPoint($hwnd, $fx, $fy)
        [DemoCap5]::MoveTo($pt.X, $pt.Y)
        Start-Sleep -Milliseconds ([Math]::Max(20, [int]($FrameIntervalMs * 0.6)))
    }
}

function Capture-Example([string]$name) {
    $pngOut = Join-Path $OutDir "$name.png"
    $gifOut = Join-Path $OutDir "$name.gif"
    if ($SkipExisting -and ((Test-Path $pngOut) -or (Test-Path $gifOut))) {
        Write-Log "SKIP`t$name`texisting"
        return @{ name = $name; status = "skip"; kind = $(if (Test-Path $gifOut) { "gif" } else { "png" }) }
    }

    Stop-ExampleProcess $name
    Start-Sleep -Milliseconds 200

    $exe = Ensure-Binary $name
    $frameDir = Join-Path $TmpRoot $name
    if (Test-Path $frameDir) { Remove-Item -Recurse -Force $frameDir }
    New-Item -ItemType Directory -Force -Path $frameDir | Out-Null

    $proc = $null
    try {
        # Launch without a console host — Rust example bins are often console
        # subsystem, and that black terminal steals focus over the iced window.
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $exe
        $psi.WorkingDirectory = $RepoRoot
        $psi.UseShellExecute = $false
        $psi.CreateNoWindow = $true
        $psi.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
        $proc = [System.Diagnostics.Process]::Start($psi)
        if ($null -eq $proc) { throw "failed to start $exe" }

        $hwnd = [IntPtr]::Zero
        $deadline = (Get-Date).AddSeconds($LaunchTimeoutSec)
        while ((Get-Date) -lt $deadline) {
            Start-Sleep -Milliseconds 250
            if ($proc.HasExited) {
                throw "process exited early (code $($proc.ExitCode))"
            }
            $hwnd = [DemoCap5]::FindAppWindow($proc.Id, $MinWindowW, $MinWindowH)
            if ($hwnd -ne [IntPtr]::Zero) { break }
        }
        if ($hwnd -eq [IntPtr]::Zero) {
            throw "timed out waiting for window (>=${MinWindowW}x${MinWindowH})"
        }

        [DemoCap5]::HideConsoleWindows($proc.Id)
        [void][DemoCap5]::ShowWindow($hwnd, [DemoCap5]::SW_RESTORE)
        # Park window at a predictable origin so CopyFromScreen is stable.
        [void][DemoCap5]::SetWindowPos($hwnd, [IntPtr]::Zero, 40, 40, 0, 0, 0x0001) # SWP_NOSIZE
        [void][DemoCap5]::SetForegroundWindow($hwnd)
        Start-Sleep -Milliseconds $WarmupMs

        if (-not $KeepLight) {
            Invoke-DarkModeToggle $hwnd
            Start-Sleep -Milliseconds $ThemeSettleMs
            [void][DemoCap5]::SetForegroundWindow($hwnd)
        }

        $doSweep = $HoverSweep -contains $name
        if ($doSweep) {
            # Start sweep a bit early so hover effects are live before frame 0.
            $sweepJob = {
                param($h, $n, $interval)
                # inlined below — PowerShell jobs can't use DemoCap easily
            }
            # Synchronous interleaved sweep + capture instead:
            $paths = @()
            $steps = $FrameCount - 1
            for ($i = 0; $i -lt $FrameCount; $i++) {
                $t = if ($steps -eq 0) { 0.5 } else { $i / [double]$steps }
                $fx = 0.20 + (0.60 * $t)
                $fy = 0.30 + (0.20 * [Math]::Sin($t * [Math]::PI))
                $pt = [DemoCap5]::ClientPoint($hwnd, $fx, $fy)
                [DemoCap5]::MoveTo($pt.X, $pt.Y)
                Start-Sleep -Milliseconds 40
                $path = Join-Path $frameDir ("frame_{0:D3}.png" -f $i)
                Capture-Hwnd $hwnd $path | Out-Null
                $paths += $path
                if ($i -lt $steps) {
                    Start-Sleep -Milliseconds $FrameIntervalMs
                }
            }
        } else {
            $paths = @()
            for ($i = 0; $i -lt $FrameCount; $i++) {
                $path = Join-Path $frameDir ("frame_{0:D3}.png" -f $i)
                Capture-Hwnd $hwnd $path | Out-Null
                $paths += $path
                if ($i -lt ($FrameCount - 1)) {
                    Start-Sleep -Milliseconds $FrameIntervalMs
                }
            }
        }

        $motion = Get-FrameMotion $paths
        $wantGif = ($script:ForceGifSet -contains $name) -or ($ForceGif -contains $name)
        $wantPng = $ForcePng -contains $name
        $asGif = if ($wantPng) { $false } elseif ($wantGif) { $true } else { $motion -ge $MotionThreshold }

        Copy-Item -LiteralPath $paths[[int]($paths.Count / 2)] -Destination $pngOut -Force

        if ($asGif) {
            try {
                New-DemoGif $frameDir $gifOut $MaxWidth
                $kind = "gif"
                $preferred = $gifOut
            } catch {
                Write-Log ("WARN`t{0}`tgif failed, keeping png: {1}" -f $name, $_.Exception.Message)
                if (Test-Path $gifOut) { Remove-Item -Force $gifOut -ErrorAction SilentlyContinue }
                $kind = "png"
                $preferred = $pngOut
            }
        } else {
            if (Test-Path $gifOut) { Remove-Item -Force $gifOut }
            $kind = "png"
            $preferred = $pngOut
        }

        $bytes = (Get-Item -LiteralPath $preferred).Length
        Write-Log ("OK`t{0}`t{1}`tmotion={2:N2}`tbytes={3}" -f $name, $kind, $motion, $bytes)
        return @{
            name = $name
            status = "ok"
            kind = $kind
            motion = [Math]::Round($motion, 2)
            file = [IO.Path]::GetFileName($preferred)
            bytes = $bytes
        }
    } catch {
        Write-Log ("FAIL`t{0}`t{1}" -f $name, $_.Exception.Message)
        return @{
            name = $name
            status = "fail"
            kind = $null
            error = $_.Exception.Message
        }
    } finally {
        if ($null -ne $proc -and -not $proc.HasExited) {
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        }
        Stop-ExampleProcess $name
        if (Test-Path $frameDir) {
            Remove-Item -Recurse -Force $frameDir -ErrorAction SilentlyContinue
        }
    }
}

# --- main ---
$Only = @(Expand-NameList $Only)
$Exclude = @(Expand-NameList $Exclude)
$ForceGif = @(Expand-NameList $ForceGif)
$ForcePng = @(Expand-NameList $ForcePng)
$script:ForceGifSet = @($DefaultForceGif + $ForceGif | Select-Object -Unique)

$names = Get-ExampleNames
if ($Only.Count -gt 0) {
    $names = $names | Where-Object { $Only -contains $_ }
}
if ($Exclude.Count -gt 0) {
    $names = $names | Where-Object { $Exclude -notcontains $_ }
}

Write-Log ("START`tcount={0}`tout={1}" -f $names.Count, $OutDir)
$results = @()
$i = 0
foreach ($name in $names) {
    $i++
    Write-Host ""
    Write-Host ("=== [{0}/{1}] {2} ===" -f $i, $names.Count, $name)
    $results += Capture-Example $name
}

$manifest = [ordered]@{
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    out_dir = "crates/enchant-iced/docs/demos"
    total = $results.Count
    ok = @($results | Where-Object { $_.status -eq "ok" }).Count
    fail = @($results | Where-Object { $_.status -eq "fail" }).Count
    skip = @($results | Where-Object { $_.status -eq "skip" }).Count
    examples = $results
}
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $ManifestPath -Encoding UTF8
Write-Log ("DONE`tok={0}`tfail={1}`tskip={2}`tmanifest={3}" -f $manifest.ok, $manifest.fail, $manifest.skip, $ManifestPath)
$manifest | ConvertTo-Json -Depth 4


