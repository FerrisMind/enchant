# Capture iced Cool Mode: idle + press-hold Click Me! + Svelte Image.
# Waits for real client window (>=650px tall), SendInput press-hold, PrintWindow.

param(
    [Parameter(Mandatory = $true)][string]$ExePath,
    [string]$OutDir = ""
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $OutDir) {
    $OutDir = if ($env:COOL_MODE_VISUAL_OUT) {
        $env:COOL_MODE_VISUAL_OUT
    } else {
        Join-Path $ScriptDir "../../.superpowers/sdd/visual"
    }
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

if (-not [System.IO.Path]::IsPathRooted($ExePath)) {
    $ExePath = Join-Path (Get-Location) $ExePath
}
if (-not (Test-Path -LiteralPath $ExePath)) {
    throw "cool_mode.exe not found: $ExePath"
}

Add-Type -AssemblyName System.Drawing

$native = @"
using System;
using System.Text;
using System.Runtime.InteropServices;

public static class CoolCap2 {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X, Y; }

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

    public const uint INPUT_MOUSE = 0;
    public const uint MOUSEEVENTF_MOVE = 0x0001;
    public const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
    public const uint MOUSEEVENTF_LEFTUP = 0x0004;
    public const uint MOUSEEVENTF_ABSOLUTE = 0x8000;
    public const uint PW_RENDERFULLCONTENT = 2;
    public const int SW_RESTORE = 9;

    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref POINT lpPoint);
    [DllImport("user32.dll")] public static extern uint SendInput(uint nInputs, INPUT[] pInputs, int cbSize);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int nIndex);
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, UIntPtr dwExtraInfo);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hdcBlt, uint nFlags);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);

    public static IntPtr FindAppWindow(int pid, int minW, int minH) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((hWnd, lParam) => {
            if (!IsWindowVisible(hWnd)) return true;
            uint windowPid;
            GetWindowThreadProcessId(hWnd, out windowPid);
            if ((int)windowPid != pid) return true;
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

    public static string TitleOf(IntPtr hWnd) {
        var sb = new StringBuilder(512);
        GetWindowText(hWnd, sb, sb.Capacity);
        return sb.ToString();
    }

    static void SendMouse(uint flags, int absX, int absY) {
        INPUT[] inputs = new INPUT[1];
        inputs[0].type = INPUT_MOUSE;
        inputs[0].mi.dx = absX;
        inputs[0].mi.dy = absY;
        inputs[0].mi.dwFlags = flags;
        uint sent = SendInput(1, inputs, Marshal.SizeOf(typeof(INPUT)));
        if (sent != 1) {
            mouse_event(flags, absX, absY, 0, UIntPtr.Zero);
        }
    }

    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);

    public static void MoveCursorAbsolute(int screenX, int screenY) {
        // Prefer SetCursorPos (physical screen coords). SendInput absolute must use
        // the VIRTUAL desktop (SM_CXVIRTUAL/SM_CYVIRTUAL) on multi-monitor setups.
        if (SetCursorPos(screenX, screenY)) return;
        int sx = Math.Max(GetSystemMetrics(78), Math.Max(GetSystemMetrics(0), 2)); // SM_CXVIRTUAL
        int sy = Math.Max(GetSystemMetrics(79), Math.Max(GetSystemMetrics(1), 2)); // SM_CYVIRTUAL
        int absX = (int)Math.Round(screenX * 65535.0 / (sx - 1));
        int absY = (int)Math.Round(screenY * 65535.0 / (sy - 1));
        SendMouse(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, absX, absY);
    }

    public static void LeftDown() {
        SendMouse(MOUSEEVENTF_LEFTDOWN, 0, 0);
    }

    public static void LeftUp() {
        SendMouse(MOUSEEVENTF_LEFTUP, 0, 0);
    }
}
"@

try { Add-Type -TypeDefinition $native -ErrorAction Stop } catch {
    if (-not ("CoolCap2" -as [type])) { throw }
}

function Start-CoolMode {
    Get-Process -Name "cool_mode" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
    $p = Start-Process -FilePath $ExePath -PassThru
    for ($i = 0; $i -lt 160; $i++) {
        Start-Sleep -Milliseconds 250
        # Require tall window matching Task 4 (900x720 + chrome ~759).
        $hwnd = [CoolCap2]::FindAppWindow($p.Id, 700, 650)
        if ($hwnd -ne [IntPtr]::Zero) {
            return @{ Proc = $p; Hwnd = $hwnd; Title = [CoolCap2]::TitleOf($hwnd) }
        }
    }
    throw "Timed out waiting for Cool Mode client window (>=700x650)"
}

function Capture-Hwnd([IntPtr]$hwnd, [string]$path) {
    $rect = New-Object CoolCap2+RECT
    [void][CoolCap2]::GetWindowRect($hwnd, [ref]$rect)
    $w = [Math]::Max(1, $rect.Right - $rect.Left)
    $h = [Math]::Max(1, $rect.Bottom - $rect.Top)
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    try {
        # Prefer CopyFromScreen for iced (PrintWindow often blanks GPU content).
        $g.CopyFromScreen($rect.Left, $rect.Top, 0, 0, (New-Object System.Drawing.Size($w, $h)))
        $mid = $bmp.GetPixel([int]($w / 2), [int]($h / 2))
        $dark = (($mid.R + $mid.G + $mid.B) -lt 45)
        if ($dark) {
            $hdc = $g.GetHdc()
            try {
                [void][CoolCap2]::PrintWindow($hwnd, $hdc, [CoolCap2]::PW_RENDERFULLCONTENT)
            } finally {
                $g.ReleaseHdc($hdc)
            }
        }
        $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $g.Dispose()
        $bmp.Dispose()
    }
    return @{ w = $w; h = $h }
}

function Client-XY([IntPtr]$hwnd, [double]$fx, [double]$fy) {
    $client = New-Object CoolCap2+RECT
    [void][CoolCap2]::GetClientRect($hwnd, [ref]$client)
    $pt = New-Object CoolCap2+POINT
    $pt.X = [int]($client.Left + ($client.Right - $client.Left) * $fx)
    $pt.Y = [int]($client.Top + ($client.Bottom - $client.Top) * $fy)
    [void][CoolCap2]::ClientToScreen($hwnd, [ref]$pt)
    return $pt
}

function Count-Colorful([string]$path) {
    $bmp = [System.Drawing.Bitmap]::FromFile($path)
    try {
        $n = 0
        # Sample every 4th pixel for speed.
        for ($y = 0; $y -lt $bmp.Height; $y += 4) {
            for ($x = 0; $x -lt $bmp.Width; $x += 4) {
                $c = $bmp.GetPixel($x, $y)
                $max = [Math]::Max($c.R, [Math]::Max($c.G, $c.B))
                $min = [Math]::Min($c.R, [Math]::Min($c.G, $c.B))
                if (($max - $min) -gt 40 -and (($c.R + $c.G + $c.B) -lt 700)) { $n++ }
            }
        }
        return $n
    } finally {
        $bmp.Dispose()
    }
}

function Press-Hold-Capture([IntPtr]$hwnd, [double]$fx, [double]$fy, [string]$path, [int]$holdMs = 800) {
    $pt = Client-XY $hwnd $fx $fy
    [CoolCap2]::MoveCursorAbsolute(($pt.X - 6), ($pt.Y - 4))
    Start-Sleep -Milliseconds 60
    [CoolCap2]::MoveCursorAbsolute($pt.X, $pt.Y)
    Start-Sleep -Milliseconds 100
    [CoolCap2]::LeftDown()
    Start-Sleep -Milliseconds $holdMs
    $size = Capture-Hwnd $hwnd $path
    [CoolCap2]::LeftUp()
    Start-Sleep -Milliseconds 400
    return $size
}

$info = Start-CoolMode
$hwnd = $info.Hwnd
[void][CoolCap2]::ShowWindow($hwnd, [CoolCap2]::SW_RESTORE)
[void][CoolCap2]::SetForegroundWindow($hwnd)
Start-Sleep -Milliseconds 2000

# Upper stage Click Me!
$pressPath = Join-Path $OutDir "iced-cool-press.png"
$pressColor = 0
$pressFy = 0.30
foreach ($fy in @(0.28, 0.30, 0.32, 0.34)) {
    Press-Hold-Capture $hwnd 0.50 $fy $pressPath 850 | Out-Null
    $pressColor = Count-Colorful $pressPath
    if ($pressColor -ge 80) { $pressFy = $fy; break }
    Start-Sleep -Milliseconds 800
}
Copy-Item -LiteralPath $pressPath -Destination (Join-Path $OutDir "iced-cool-pressed.png") -Force

# Park and let particles clear before custom capture.
$away = Client-XY $hwnd 0.97 0.06
[CoolCap2]::MoveCursorAbsolute($away.X, $away.Y)
Start-Sleep -Milliseconds 1500

# Lower stage Svelte Image (screenshot fy~0.77 ≈ client ~0.75–0.80).
$customPath = Join-Path $OutDir "iced-cool-custom-press.png"
$customColor = 0
$customFy = 0.77
foreach ($fy in @(0.72, 0.74, 0.76, 0.77, 0.78, 0.80)) {
    Press-Hold-Capture $hwnd 0.50 $fy $customPath 900 | Out-Null
    $customColor = Count-Colorful $customPath
    if ($customColor -ge 40) { $customFy = $fy; break }
    Start-Sleep -Milliseconds 800
}

# Re-capture idle last so it is not overwritten by an older process.
$away = Client-XY $hwnd 0.97 0.06
[CoolCap2]::MoveCursorAbsolute($away.X, $away.Y)
Start-Sleep -Milliseconds 1200
$idlePath = Join-Path $OutDir "iced-cool-idle.png"
$idleSize = Capture-Hwnd $hwnd $idlePath

$rect = New-Object CoolCap2+RECT
[void][CoolCap2]::GetWindowRect($hwnd, [ref]$rect)

Stop-Process -Id $info.Proc.Id -Force -ErrorAction SilentlyContinue

@{
    idlePath = $idlePath
    pressPath = $pressPath
    customPath = $customPath
    title = $info.Title
    hwnd = "$hwnd"
    width = ($rect.Right - $rect.Left)
    height = ($rect.Bottom - $rect.Top)
    idleWH = "$($idleSize.w)x$($idleSize.h)"
    idleBytes = (Get-Item -LiteralPath $idlePath).Length
    pressBytes = (Get-Item -LiteralPath $pressPath).Length
    customBytes = (Get-Item -LiteralPath $customPath).Length
    pressColorful = $pressColor
    customColorful = $customColor
    pressFy = $pressFy
    customFy = $customFy
} | ConvertTo-Json
