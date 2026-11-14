# Audit de securite WinNote 0.7.0 - 2026-11-14

Perimetre : tout `src/` (Windows + port GTK `src/linux/` + modules partages), `Cargo.lock`, historique git.
Methode : lecture du code (unsafe/FFI, parseurs, fichiers, liens, mise a jour), `cargo audit`, `gitleaks`, test d'entrees hostiles
pseudo-aleatoires, essais reels sur la fenetre WinNote (captures PrintWindow). Numeros de ligne = etat de la branche `night` apres correctifs.
Aucune version incrementee (reste 0.7.0), rien de publie.

## Corriges

| Niveau | Faille | Fichier:ligne (apres correctif) | Correctif |
|---|---|---|---|
| HAUTE | Un lien Markdown `[x](C:\Windows\System32\calc.exe)` ou `[x](\\serveur\partage\a.exe)` etait passe tel quel a `ShellExecuteW("open")` / `xdg-open` : lancement de code ou fuite d'identifiants NTLM par un simple clic sur un document hostile | `src/textops.rs:273` (`web_url`), `:306-345` (`open_link`) | seuls `http://`, `https://`, `mailto:` partent au systeme ; un lien relatif ne s'ouvre que dans WinNote, et seulement sous le dossier du document |
| MOYENNE | `[[../../x]]`, `[[C:\x]]`, `[[\\srv\s\f]]` : `Path::join` remplacait la base par un chemin absolu / UNC ; ouverture hors du coffre de notes, fuite NTLM | `src/textops.rs:279` (`local_target`) | refus de tout chemin absolu, lecteur, UNC, `..`, flux NTFS `:` ou caractere de controle |
| MOYENNE | « Ouvrir dans le navigateur » faisait `ShellExecuteW` sur n'importe quel fichier du document courant (un `.bat` / `.exe` / `.lnk` s'executait) | `src/textops.rs:291` (`browser_safe`), `src/cmds.rs:779` commande `BROWSER` | liste blanche html/htm/xhtml/mhtml/svg/pdf/txt/md/markdown |
| MOYENNE | Mise a jour : `parse_release` decoupait le JSON par decalages d'octets : panique (donc arret, `panic = abort`) sur reponse a caracteres multi-octets ; l'empreinte pouvait venir d'un AUTRE actif ; aucune verification de l'hote ni de l'hexadecimal | `src/vers.rs:17` (`digest_in`), `:26` (`trusted_download`), `src/updater.rs:93-120`, `src/linux/update.rs:81,102` | empreinte lue dans l'actif seulement, 64 chiffres hexadecimaux exiges, HTTPS vers github.com / *.githubusercontent.com uniquement ; curl Linux force `--proto =https` |
| MOYENNE | Mise a jour Windows : le fichier verifie restait dans `%TEMP%\winnote-update.exe` (nom previsible) jusqu'a l'installation : un autre processus du meme utilisateur pouvait le remplacer apres la verification SHA-256 | `src/updater.rs:138-175` (`LAST_SHA`, `download`, `apply`) | nom unique + `create_new`, empreinte re-calculee juste avant le remplacement (fichier refuse et supprime s'il a change) |
| MOYENNE | Enregistrement non atomique (`fs::write` direct) : disque plein, coupure ou plantage pendant l'ecriture = fichier tronque, perte du document | `src/fileops.rs:55` (`write_atomic`), appelants `src/cmds.rs` (`save_doc`), `src/features.rs:60`, `src/linux/edit.rs` | ecriture complete dans un voisin temporaire, `sync_all`, puis remplacement ; liens symboliques et fichiers en lecture seule ecrits directement (liens/droits conserves) |
| MOYENNE | Ouvrir un peripherique (`CON`, `NUL`, tube) ou un dossier lisait indefiniment (interface figee) ; fichier > 2 Gio : `len as i32` negatif dans les conversions d'encodage | `src/fileops.rs:38` (`check_openable`, plafond 1 Gio), `src/cmds.rs` (`open_path`, `reload_with`), `src/linux/edit.rs` | ouverture refusee avec message clair |
| BASSE | `restore()` de la session lisait `session\<champ backup>` : un `session.txt` retouche (`..\..\x`) faisait lire un fichier arbitraire | `src/fileops.rs:30` (`plain_name`), `src/session.rs:87`, `src/linux/edit.rs` | nom simple exige |
| BASSE | `alive(pid)` : un processus mort restait « vivant » tant qu'un handle trainait (recuperation jamais proposee, test `two_live_windows` instable) | `src/extras.rs:203` | `GetExitCodeProcess == STILL_ACTIVE` |
| BASSE | Copies de secours : dossier efface avant reecriture, un arret brutal pendant ces millisecondes ne laissait rien | `src/extras.rs:272` (`recovery_snapshot`) | ecriture dans `<pid>.new` puis echange ; `.new` nettoye a la sortie et par `recovery_discard` |
| BASSE | Parcours de dossier (recherche, liens entrants, etiquettes) suivait liens/jonctions (boucle jusqu'a 5000 entrees), lisait tubes/peripheriques et fichiers sans limite | `src/notes.rs:5` (`read_small`, 4 Mio), `:9` (`walk` via `file_type()`) | liens ignores, fichiers ordinaires seulement |
| BASSE | Copie d'un dossier (`duplicate`) suivait les liens : recursion infinie sur une jonction bouclante | `src/fileops.rs:127` (`copy_dir`) | liens ignores |
| BASSE | Noms reserves Windows (`CON`, `NUL`, `COM1`...), noms finissant par un point ou > 255 octets acceptes par l'explorateur ; renommage en changeant seulement la casse refuse | `src/fileops.rs:5-30` (`valid_name`, `reserved_windows`), `rename` | refus (Windows) ; renommage de casse accepte |
| BASSE | `open_link` : decalage d'octets venu de Scintilla pouvant couper un caractere (texte decode avec remplacement) : panique | `src/textops.rs:297` (`floor_boundary`) | decalage ramene a une frontiere |

## Constate, non corrige (a decider)

| Niveau | Sujet | Detail |
|---|---|---|
| MOYENNE | Mise a jour : integrite mais pas authenticite | l'empreinte vient de la meme release GitHub que l'executable (champ `digest` ou `winnote.exe.sha256`). Elle protege d'une corruption / d'un miroir, pas d'un depot ou d'un compte compromis. Il faudrait une signature (cle Ed25519 epinglee, comme ForgeRemote) ou la signature Authenticode verifiee par `WinVerifyTrust`. |
| BASSE | Messages `WM_APP+1..3` (0x8001-0x8003) acceptes de n'importe quel processus du meme niveau d'integrite, avec un pointeur `Box::from_raw` | exploitable seulement par un processus deja local au meme utilisateur (qui peut faire pire) ; filtrer par un jeton partage si on veut fermer. |
| BASSE | `WINNOTE_UPDATE_URL` (Linux) accepte une URL `file://` | crochet de test ; exige de controler l'environnement du processus. Pourrait etre retire du binaire de release. |
| BASSE | La session et les copies de secours contiennent le texte non enregistre en clair dans `%APPDATA%\WinNote` | comportement attendu (ACL du profil) ; sous Linux creees avec l'umask de l'utilisateur (0700 non force sur `~/.config/winnote`). |
| BASSE | `settings::path()` / `session::dir()` : si `APPDATA` est vide, chemin relatif au dossier courant | cas hors Windows normal. |
| INFO | `cargo audit` : 0 vulnerabilite ; 2 avertissements (proc-macro-error non maintenu, glib 0.18.5 `VariantStrIter` unsound) | dependances GTK du port Linux uniquement (`gtk 0.18`) ; non utilisees par la version Windows. `cargo deny` non installe. |
| INFO | `gitleaks detect` : 32 commits, aucun secret | |

## Preuves

- `cargo test` : 36 tests passent (dont nouveaux : `fileops::reserved_and_limits`, `textops::link_tests::*`, `vers::tests::*`, `updater::hostile_release_refused`, `extras::hostile_inputs_never_panic` = 3000 entrees hostiles sur YAML / Markdown / mise en page / encodages, `two_live_windows` desormais stable).
- Selftest Windows (APPDATA redirige) : 0 echec, dont 3 nouveaux controles (dossier / peripherique refuses, enregistrement atomique sans fichier `.tmp`).
- Selftest Linux (WSL Ubuntu 22.04, Xvfb) : 94 controles OK, 0 echec.
- Essais reels : fichier de 87 Mo + 4 autres onglets ouverts, fenetre non figee ; fermeture 40-60 ms, 0 processus restant ; arret brutal (kill) apres frappe : le texte revient au redemarrage dans un seul onglet ; impression PDF (`--print-to`) 1 page.
