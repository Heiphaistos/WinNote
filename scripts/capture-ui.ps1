<#
 Captures de la fenetre WinNote (PrintWindow, jamais la souris ni l'ecran) pour un DPI simule et un theme donnes.
 Build requis : cargo build --release --features test-hooks (WINNOTE_TEST_DPI n'existe que dans ce build).
 Usage : .\scripts\capture-ui.ps1 -Dpi 150 -Theme 16 -Cmds 920,832 -Out D:\Temp\wn_ui
 -Cmds : identifiants de commande postes (WM_COMMAND) dans l'ordre ; une capture apres chacun (id >= 100000 = menu deroulant par mnemonique (100000 + n : n-ieme menu de la barre, 0 = Fichier) capture par sa classe #32768).
 APPDATA est redirige dans un dossier temporaire : les reglages de l'utilisateur ne sont jamais touches.
#>
param([int]$Dpi = 96, [int]$Theme = 0, [int[]]$Cmds = @(), [string]$Out = 'D:\Temp\wn_ui', [string]$Exe = 'D:\Projet\WinNote\target\release\winnote.exe', [string]$Root = '', [int]$Wait = 2500)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force $Out | Out-Null
$ad = Join-Path $Out "appdata_$Dpi"
New-Item -ItemType Directory -Force "$ad\WinNote" | Out-Null
$ini = "theme=$Theme`nsidebar=1`nsidemode=0`nlines=1`n"
if ($Root) { $ini += "root=$Root`n" }
Set-Content -Path "$ad\WinNote\settings.ini" -Value $ini -Encoding ascii
$doc = Join-Path $Out 'note.md'
if (-not (Test-Path $doc)) {
  Set-Content -Path $doc -Encoding utf8 -Value "---`ntitre: Essai`ntags: [a, b]`n---`n# Titre un`n`nTexte avec [[autre]] et **gras**.`n`n## Sous-titre`n`n- [ ] tache`n- liste`n"
}
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class WnPost { [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l); }
'@
$env:APPDATA = $ad
$env:WINNOTE_TEST_DPI = "$Dpi"
foreach ($c in (@(0) + $Cmds)) {
  $p = Start-Process -FilePath $Exe -ArgumentList "`"$doc`"" -PassThru
  try {
    Start-Sleep -Seconds 3
    $p.Refresh()
    $h = $p.MainWindowHandle
    if ($h -eq [IntPtr]::Zero) { throw 'pas de fenetre principale' }
    $tag = "d${Dpi}_t${Theme}_cmd$c"
    if ($c -eq 0) { & "$PSScriptRoot\capture-fenetre.ps1" -ProcId $p.Id -Out "$Out\$tag.png" -Titre 'WinNote'; continue }
    if ($c -ge 100000) {
      [void][WnPost]::PostMessage($h, 0x112, [IntPtr]0xF100, [IntPtr]0); Start-Sleep -Milliseconds 400
      [void][WnPost]::PostMessage($h, 0x100, [IntPtr]0x28, [IntPtr]0)
      for ($k = 0; $k -lt ($c - 100000); $k++) { Start-Sleep -Milliseconds 300; [void][WnPost]::PostMessage($h, 0x100, [IntPtr]0x27, [IntPtr]0) }
    } else { [void][WnPost]::PostMessage($h, 0x111, [IntPtr]$c, [IntPtr]::Zero) }
    Start-Sleep -Milliseconds $Wait
    if ($c -ge 100000) { & "$PSScriptRoot\capture-fenetre.ps1" -ProcId $p.Id -Out "$Out\$tag.png" -Classe '#32768' }
    else { & "$PSScriptRoot\capture-fenetre.ps1" -ProcId $p.Id -Out "$Out\$tag.png" -Titre '' }
  } finally {
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
  }
}
