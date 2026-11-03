Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class W5 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  public struct RECT { public int Left, Top, Right, Bottom; }
  public static IntPtr Find(uint pid, string prefix) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h)) {
        var sb = new StringBuilder(512); GetWindowTextW(h, sb, 512);
        if (sb.ToString().StartsWith(prefix)) { found = h; return false; }
      }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
"@
$LD = 0x0002; $LU = 0x0004
$proc = Get-Process nexora-studio -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $proc) { Write-Output "APP_NOT_RUNNING"; exit 1 }
$h = [W5]::Find([uint32]$proc.Id, "NEXORA")
if ($h -eq [IntPtr]::Zero) { Write-Output "NO_APP_WINDOW"; exit 1 }
[W5]::ShowWindow($h, 3) | Out-Null
Start-Sleep -Milliseconds 900
[W5]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 500
$r = New-Object W5+RECT
[W5]::GetWindowRect($h, [ref]$r) | Out-Null
Write-Output ("RECT {0},{1} - {2},{3}" -f $r.Left, $r.Top, $r.Right, $r.Bottom)

function Shot($name) {
  $b = New-Object System.Drawing.Bitmap(($r.Right - $r.Left), ($r.Bottom - $r.Top))
  $g = [System.Drawing.Graphics]::FromImage($b)
  $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $b.Size)
  $b.Save("$PSScriptRoot\.tmp_$name.png", [System.Drawing.Imaging.ImageFormat]::Png)
  $g.Dispose(); $b.Dispose()
}
Shot "before"

# Image-space coordinates of the two SOURCE rows (see .tmp_before.png).
$sx = $r.Left + 400
$sy = $r.Top + 436   # row 2: "Clock & Timer"
$ey = $r.Top + 400   # above row-1 centre -> slot 0 (front-most layer)

[W5]::SetCursorPos($sx, $sy) | Out-Null
Start-Sleep -Milliseconds 400
[W5]::mouse_event($LD, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 250
for ($i = 1; $i -le 16; $i++) {
  $y = $sy + (($ey - $sy) * $i / 16)
  [W5]::SetCursorPos($sx, [int]$y) | Out-Null
  Start-Sleep -Milliseconds 55
}
Start-Sleep -Milliseconds 350
Shot "dragging"
[W5]::mouse_event($LU, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 800
Shot "after"
Write-Output "DRAG_DONE"
