<#
 Signe (Ed25519) les fichiers d'une release WinNote : ecrit <fichier>.sig (64 octets) a cote de chaque fichier.
 La mise a jour automatique refuse tout fichier sans .sig valide (cle publique epinglee dans src/vers.rs).
 A lancer APRES la signature Authenticode (qui modifie l'exe) et APRES le calcul de l'empreinte finale.
 Usage : .\scripts\signer-release.ps1 -Version 0.7.1 -Files target\release\winnote.exe,target\installer\WinNote-Setup-0.7.1.exe[,paquets Linux...]
 Cle privee : D:\MDP\winnote-updater\update-signing.hex (hors depot, jamais commitee).
 Le nom de l'actif publie doit etre exactement le nom du fichier signe (le nom fait partie du message signe).
#>
param(
    [Parameter(Mandatory)][string]$Version,
    [Parameter(Mandatory, Position = 0)][string[]]$Files,
    [string]$Key = 'D:\MDP\winnote-updater\update-signing.hex'
)
$ErrorActionPreference = 'Continue' # cargo ecrit sur stderr : les erreurs sont testees par $LASTEXITCODE
$Files = $Files | ForEach-Object { (Resolve-Path $_).Path } # avant Set-Location (chemins relatifs)
Set-Location (Split-Path $PSScriptRoot)
if (-not (Test-Path $Key)) { throw "cle de signature introuvable : $Key" }
$tag = "v$($Version.TrimStart('v'))"
cargo build --release --example sign; if ($LASTEXITCODE) { throw 'outil de signature : compilation en echec' }
foreach ($f in $Files) {
    $p = (Resolve-Path $f).Path
    $sha = (Get-FileHash $p -Algorithm SHA256).Hash.ToLower()
    & target\release\examples\sign.exe sign $Key $tag $p $sha; if ($LASTEXITCODE) { throw "signature en echec : $p" }
    if ((Get-Item "$p.sig").Length -ne 64) { throw "signature invalide : $p.sig" }
    Write-Host "signe : $p.sig"
}
