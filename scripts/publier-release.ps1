<#
 Construit, teste, signe et (avec -Publier) publie une release WinNote : exe signe + empreinte SHA-256 + installeur.
 Usage : .\scripts\publier-release.ps1 [-Publier]
 Le depot doit etre PUBLIC pour que la mise a jour automatique puisse telecharger la release.
#>
param([switch]$Publier)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot)
$ver = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
Write-Host "== WinNote $ver"
cargo test --release; if ($LASTEXITCODE) { throw 'tests unitaires en echec' }
cargo build --release; if ($LASTEXITCODE) { throw 'compilation en echec' }
$log = Join-Path $env:TEMP 'winnote-selftest.log'
$p = Start-Process target\release\winnote.exe -ArgumentList '--selftest', "`"$log`"" -PassThru
if (-not $p.WaitForExit(120000) -or $p.ExitCode -ne 0) { Get-Content $log | Select-String '^FAIL'; throw 'autotest en echec' }
& 'D:\Projet\outils\signer\signer.ps1' (Resolve-Path target\release\winnote.exe).Path
$exe = (Resolve-Path target\release\winnote.exe).Path
$sha = (Get-FileHash $exe -Algorithm SHA256).Hash.ToLower()
"$sha *winnote.exe" | Set-Content target\release\winnote.exe.sha256 -Encoding ascii
& 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe' "/DAppVersion=$ver" installer\winnote.iss; if ($LASTEXITCODE) { throw 'installeur en echec' }
$setup = "target\installer\WinNote-Setup-$ver.exe"
& 'D:\Projet\outils\signer\signer.ps1' (Resolve-Path $setup).Path
# signature Ed25519 epinglee (obligatoire : sans .sig la mise a jour automatique refuse la release)
$setupFull = (Resolve-Path $setup).Path
& "$PSScriptRoot\signer-release.ps1" -Version $ver -Files $exe,$setupFull
Write-Host "exe sha256 = $sha"
if ($Publier) {
    gh release create "v$ver" $exe "$exe.sig" target\release\winnote.exe.sha256 $setup "$setupFull.sig" --title "WinNote $ver" --generate-notes
}
