//! Outils de version / JSON minimal partagés par les mises à jour Windows et Linux.

pub fn json_str(body: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":");
    let i = body.find(&pat)? + pat.len();
    let rest = body[i..].trim_start().strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_string())
}

pub fn newer(remote: &str, local: &str) -> bool {
    let p = |v: &str| v.trim_start_matches('v').split('.').map(|x| x.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
    p(remote) > p(local)
}

/// Empreinte SHA-256 (64 hex) de l'actif qui commence à `asset` (le JSON à partir de son nom) : le champ `digest` d'un AUTRE actif
/// n'est jamais pris, une empreinte tronquée ou non hexadécimale est refusée (jamais de découpe hors frontière de caractère).
pub fn digest_in(asset: &str) -> Option<String> {
    let end = asset.get(1..).and_then(|r| r.find("\"name\":")).map_or(asset.len(), |i| i + 1);
    let r = asset.get(..end)?;
    let i = r.find("\"digest\":\"sha256:")? + 17;
    let h = r.get(i..i + 64)?;
    h.bytes().all(|b| b.is_ascii_hexdigit()).then(|| h.to_ascii_lowercase())
}

/// Clé publique Ed25519 épinglée (hex) qui signe les mises à jour. La clé privée vit hors dépôt (D:\MDP\winnote-updater).
const UPDATE_PUBKEY_HEX: &str = "db15d404c3bef932114322dab42746131e21cbc0a0f03a3e97a7b1e83d33b47f";

/// Clé de vérification : celle épinglée ; les builds `test-hooks` (jamais publiés) peuvent la remplacer par `WINNOTE_UPDATE_PUBKEY`.
fn pubkey_hex() -> String {
    #[cfg(feature = "test-hooks")]
    if let Ok(k) = std::env::var("WINNOTE_UPDATE_PUBKEY") {
        return k;
    }
    UPDATE_PUBKEY_HEX.to_string()
}

/// Message signé : lie la signature à la version, au nom du fichier et à son empreinte (pas de rejeu d'une vieille version ni d'un autre actif).
pub fn signed_msg(tag: &str, name: &str, sha_hex: &str) -> Vec<u8> {
    format!("winnote-update-v1\n{tag}\n{name}\n{}", sha_hex.to_ascii_lowercase()).into_bytes()
}

/// Nom du fichier = dernier segment de l'URL (sans requête).
pub fn url_name(url: &str) -> &str {
    url.split(['?', '#']).next().unwrap_or("").rsplit('/').next().unwrap_or("")
}

/// Vérifie la signature Ed25519 brute (64 octets) du fichier de version `tag`, nom `url_name(url)`, empreinte `sha_hex` (celle calculée sur les octets reçus).
pub fn verify_update(tag: &str, url: &str, sha_hex: &str, sig: &[u8]) -> Result<(), String> {
    verify_with(&pubkey_hex(), tag, url_name(url), sha_hex, sig)
}

fn verify_with(key_hex: &str, tag: &str, name: &str, sha_hex: &str, sig: &[u8]) -> Result<(), String> {
    let kb: Vec<u8> = (0..key_hex.len() / 2).filter_map(|i| key_hex.get(i * 2..i * 2 + 2).and_then(|h| u8::from_str_radix(h, 16).ok())).collect();
    let key = ed25519_compact::PublicKey::from_slice(&kb).map_err(|_| "clé de mise à jour invalide")?;
    let sig = ed25519_compact::Signature::from_slice(sig).map_err(|_| "signature de mise à jour illisible")?;
    key.verify(signed_msg(tag, name, sha_hex), &sig).map_err(|_| "signature de mise à jour invalide : installation refusée".to_string())
}

/// Lien de téléchargement acceptable : HTTPS vers GitHub uniquement (ni http, ni identifiants, ni autre hôte).
pub fn trusted_download(url: &str) -> bool {
    #[cfg(feature = "test-hooks")]
    if std::env::var_os("WINNOTE_UPDATE_URL").is_some() {
        return true; // faux serveur local des tests
    }
    let Some(r) = url.strip_prefix("https://") else { return false };
    let host = r.split('/').next().unwrap_or("");
    !host.contains(['@', ':']) && (host == "github.com" || host.ends_with(".githubusercontent.com"))
}

#[cfg(test)]
mod tests {
    use super::*;
    const H: &str = "7e525ac96a9f330b49f375b7d9a1af32486ce8c26b1588b7034b2242d4cb64da";
    #[test]
    fn digest_scoped_and_hex() {
        let a = format!("winnote.exe\",\"digest\":\"sha256:{H}\",\"x\":1}},{{\"name\":\"other\"");
        assert_eq!(digest_in(&a).as_deref(), Some(H));
        // pas d'empreinte pour cet actif : celle de l'actif suivant ne compte pas
        let b = format!("winnote.exe\",\"size\":1}},{{\"name\":\"autre\",\"digest\":\"sha256:{H}\"");
        assert_eq!(digest_in(&b), None);
        assert_eq!(digest_in("winnote.exe\",\"digest\":\"sha256:zz25\""), None);
        assert_eq!(digest_in(&format!("w\",\"digest\":\"sha256:{}ééé", &H[..40])), None, "pas de panique sur une frontière UTF-8");
        assert_eq!(digest_in(""), None);
    }
    #[test]
    fn signature_binds_version_name_and_hash() {
        use ed25519_compact::{KeyPair, Seed};
        let kp = KeyPair::from_seed(Seed::new([7; 32]));
        let key: String = kp.pk.iter().map(|b| format!("{b:02x}")).collect();
        let sig = kp.sk.sign(signed_msg("v1.0.0", "winnote.exe", H), None);
        let ok = |t: &str, n: &str, h: &str, s: &[u8]| verify_with(&key, t, n, h, s).is_ok();
        assert!(ok("v1.0.0", "winnote.exe", H, sig.as_ref()));
        assert!(!ok("v1.0.1", "winnote.exe", H, sig.as_ref()), "autre version");
        assert!(!ok("v1.0.0", "autre.exe", H, sig.as_ref()), "autre nom");
        assert!(!ok("v1.0.0", "winnote.exe", &H.replace('7', "8"), sig.as_ref()), "autre contenu");
        assert!(!ok("v1.0.0", "winnote.exe", H, &[0; 64]) && !ok("v1.0.0", "winnote.exe", H, &[]), "signature fausse / absente");
        let other = KeyPair::from_seed(Seed::new([8; 32])).pk;
        let okey: String = other.iter().map(|b| format!("{b:02x}")).collect();
        assert!(verify_with(&okey, "v1.0.0", "winnote.exe", H, sig.as_ref()).is_err(), "autre clé");
        assert_eq!(url_name("https://github.com/a/b/releases/download/v1/winnote.exe?x=1"), "winnote.exe");
        assert!(UPDATE_PUBKEY_HEX.len() == 64);
    }
    #[test]
    fn download_hosts() {
        assert!(trusted_download("https://github.com/a/b/releases/download/v1/x.exe"));
        assert!(trusted_download("https://objects.githubusercontent.com/x"));
        for u in ["http://github.com/x", "https://evil.com/x", "https://github.com@evil.com/x", "https://github.com:444/x", "https://evilgithub.com/x", "file:///x", "ftp://github.com/x"] {
            assert!(!trusted_download(u), "{u}");
        }
    }
}
