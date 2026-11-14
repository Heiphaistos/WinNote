<#
 Capture d'UNE fenetre (PrintWindow, jamais l'ecran ni la souris) vers un PNG.
 Usage : .\scripts\capture-fenetre.ps1 -Pid 1234 -Out D:\Temp\x.png [-Titre "Mise en page"]
 -Titre : choisit, parmi les fenetres du processus (y compris dialogues), celle dont le titre contient ce texte.
#>
param([Parameter(Mandatory)][int]$ProcId, [Parameter(Mandatory)][string]$Out, [string]$Titre = '')
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System; using System.Text; using System.Collections.Generic; using System.Runtime.InteropServices;
public static class WinCap {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
  [DllImport("dwmapi.dll")] static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int sz);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public static List<IntPtr> Of(uint pid) {
    var l = new List<IntPtr>();
    EnumWindows((h, _) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && IsWindowVisible(h)) l.Add(h); return true; }, IntPtr.Zero);
    return l;
  }
  public static string Title(IntPtr h) { var s = new StringBuilder(512); GetWindowText(h, s, 512); return s.ToString(); }
  public static System.Drawing.Bitmap Shot(IntPtr h) {
    RECT r; GetWindowRect(h, out r);
    int w = r.R - r.L, hh = r.B - r.T;
    var bmp = new System.Drawing.Bitmap(w, hh);
    using (var g = System.Drawing.Graphics.FromImage(bmp)) { IntPtr dc = g.GetHdc(); PrintWindow(h, dc, 2); g.ReleaseHdc(dc); }
    return bmp;
  }
}
'@ -ReferencedAssemblies System.Drawing
$h = [WinCap]::Of($ProcId) | Where-Object { $t = [WinCap]::Title($_); $t -and ($Titre -eq '' -or $t -like "*$Titre*") } | Select-Object -First 1
if (-not $h) { throw "aucune fenetre visible pour le processus $ProcId (titre '$Titre')" }
$b = [WinCap]::Shot($h)
$b.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png); $b.Dispose()
Write-Host "$Out : $([WinCap]::Title($h))"
