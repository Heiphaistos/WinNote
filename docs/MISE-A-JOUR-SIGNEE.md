# Mise a jour authentifiee (signature Ed25519 epinglee)

Depuis 0.7.1 (a publier), WinNote verifie avant toute installation, sous Windows comme sous Linux (AppImage) :

1. l'empreinte SHA-256 annoncee par la release (integrite) ;
2. la signature Ed25519 `<actif>.sig` (authenticite) : 64 octets bruts, verifiee avec la cle publique ECRITE EN DUR dans le binaire
   (`src/vers.rs`, `UPDATE_PUBKEY_HEX`). Un depot ou un compte GitHub compromis ne suffit plus : il faut la cle privee.

Message signe : `winnote-update-v1\n<tag>\n<nom de l'actif>\n<sha256 du fichier>` : la signature est liee a la version, au nom du
fichier et a son contenu (pas de rejeu d'une ancienne version signee, ni d'un autre actif).

**Absence de `.sig`, signature fausse, autre cle, contenu modifie = installation refusee, fichier supprime.**

## Transition 0.7.0 -> 0.7.1

- 0.7.0 (deja installee chez les utilisateurs) ne verifie pas de signature : elle installe la 0.7.1 sur la seule empreinte SHA-256, comme avant.
- 0.7.1 embarque la verification : la release 0.7.1 doit donc deja contenir les `.sig` (sinon la mise a jour 0.7.1 -> 0.7.2 serait refusee).
- Une release SANS `.sig` ne peut etre installee que par une version qui ne verifie pas encore (<= 0.7.0). Aucun reglage ne desactive la verification
  dans une release.
- Perte de la cle privee = plus aucune mise a jour automatique possible : l'utilisateur doit reinstaller a la main une version avec une nouvelle cle.
  Sauvegarde : `D:\MDP\winnote-updater\update-signing.hex` (copier sur E:\MDP comme les autres cles).

## Publier

`scripts/publier-release.ps1 -Publier` signe (`scripts/signer-release.ps1`) l'exe et l'installeur apres la signature Authenticode, et uploade les `.sig`.
Linux : l'AppImage (et seulement elle) s'auto-met a jour ; signer `WinNote-<version>-x86_64.AppImage` avec
`.\scripts\signer-release.ps1 -Version X.Y.Z -Files WinNote-X.Y.Z-x86_64.AppImage` puis `gh release upload vX.Y.Z WinNote-X.Y.Z-x86_64.AppImage.sig`
(la CI ne detient pas la cle).

## Crochets de test (feature `test-hooks`, JAMAIS dans une release)

`WINNOTE_UPDATE_URL` (API locale http:// sous Windows, http:// ou file:// sous Linux), `WINNOTE_UPDATE_PUBKEY` (cle de test) et la commande
`--apply-update` n'existent que dans un build `--features test-hooks`. Une release normale les ignore.

- Windows : `cargo build --features test-hooks` puis `winnote.exe --selftest journal.txt` (faux serveur http local : signature valide acceptee,
  autre cle / absente / contenu modifie refuses).
- Linux : `scripts/wsl-test.sh` (selftest) et `scripts/wsl-updtest.sh` (dialogue reel sous Xvfb : valide installee, autre cle refusee, pas de .sig refusee).
