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

/// Lien de téléchargement acceptable : HTTPS vers GitHub uniquement (ni http, ni identifiants, ni autre hôte).
pub fn trusted_download(url: &str) -> bool {
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
    fn download_hosts() {
        assert!(trusted_download("https://github.com/a/b/releases/download/v1/x.exe"));
        assert!(trusted_download("https://objects.githubusercontent.com/x"));
        for u in ["http://github.com/x", "https://evil.com/x", "https://github.com@evil.com/x", "https://github.com:444/x", "https://evilgithub.com/x", "file:///x", "ftp://github.com/x"] {
            assert!(!trusted_download(u), "{u}");
        }
    }
}
