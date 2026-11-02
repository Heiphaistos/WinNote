//! Mise à jour automatique : vérifie la dernière release GitHub, télécharge, contrôle le SHA-256, remplace l'exe et relance.
//! Exige une release publique contenant `winnote.exe` (et son empreinte `digest` GitHub ou un fichier `winnote.exe.sha256`).
use crate::{app, w};
use std::ptr::null;
use windows_sys::Win32::{
    Networking::WinHttp::*,
    Security::Cryptography::{BCryptHash, BCRYPT_SHA256_ALG_HANDLE},
    UI::WindowsAndMessaging::PostMessageW,
};

pub const REPO: &str = "Heiphaistos/WinNote";
pub const WM_UPD_FOUND: u32 = 0x8001;
pub const WM_UPD_READY: u32 = 0x8002;
pub const WM_UPD_NONE: u32 = 0x8003;

pub struct Found { pub version: String, pub url: String, pub sha: String, pub manual: bool }

fn http_get(host: &str, path: &str) -> Option<Vec<u8>> {
    unsafe {
        let ses = WinHttpOpen(w("WinNote-updater").as_ptr(), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, null(), null(), 0);
        if ses.is_null() {
            return None;
        }
        let con = WinHttpConnect(ses, w(host).as_ptr(), 443, 0);
        let req = WinHttpOpenRequest(con, w("GET").as_ptr(), w(path).as_ptr(), null(), null(), null(), WINHTTP_FLAG_SECURE);
        let ok = !req.is_null() && WinHttpSendRequest(req, null(), 0, null(), 0, 0, 0) != 0 && WinHttpReceiveResponse(req, std::ptr::null_mut()) != 0;
        let mut out = Vec::new();
        if ok {
            let mut status = 0u32;
            let mut len = 4u32;
            WinHttpQueryHeaders(req, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, null(), &mut status as *mut u32 as *mut _, &mut len, std::ptr::null_mut());
            if status == 200 {
                loop {
                    let mut n = 0u32;
                    if WinHttpQueryDataAvailable(req, &mut n) == 0 || n == 0 {
                        break;
                    }
                    let mut buf = vec![0u8; n as usize];
                    let mut got = 0u32;
                    if WinHttpReadData(req, buf.as_mut_ptr() as *mut _, n, &mut got) == 0 || got == 0 {
                        break;
                    }
                    out.extend_from_slice(&buf[..got as usize]);
                    if out.len() > 64 * 1024 * 1024 {
                        return None;
                    }
                }
            } else {
                ok.then_some(()).map(|_| ());
                out.clear();
                if status != 200 {
                    WinHttpCloseHandle(req);
                    WinHttpCloseHandle(con);
                    WinHttpCloseHandle(ses);
                    return None;
                }
            }
        }
        for h in [req, con, ses] {
            if !h.is_null() {
                WinHttpCloseHandle(h);
            }
        }
        ok.then_some(out)
    }
}

fn split_url(u: &str) -> Option<(String, String)> {
    let r = u.strip_prefix("https://")?;
    let (h, p) = r.split_once('/')?;
    Some((h.to_string(), format!("/{p}")))
}

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

pub fn sha256_hex(data: &[u8]) -> String {
    let mut out = [0u8; 32];
    unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, std::ptr::null_mut(), 0, data.as_ptr() as *mut u8, data.len() as u32, out.as_mut_ptr(), 32) };
    out.iter().map(|b| format!("{b:02x}")).collect()
}

fn post<T>(msg: u32, v: T) {
    let p = Box::into_raw(Box::new(v)) as isize;
    unsafe { PostMessageW(app().hwnd, msg, 0, p) };
}

/// Vérification en arrière-plan ; le résultat revient par message à la fenêtre principale.
pub fn check(manual: bool) {
    std::thread::spawn(move || {
        let found = (|| {
            let body = String::from_utf8_lossy(&http_get("api.github.com", &format!("/repos/{REPO}/releases/latest"))?).to_string();
            let tag = json_str(&body, "tag_name")?;
            if !newer(&tag, env!("CARGO_PKG_VERSION")) {
                return None;
            }
            // asset winnote.exe : on cherche son URL de téléchargement puis son empreinte
            let ai = body.find("winnote.exe\"")?;
            let url = json_str(&body[ai..], "browser_download_url")?;
            let sha = body[ai..].find("\"digest\":\"sha256:").map(|d| body[ai + d + 17..ai + d + 17 + 64].to_string());
            let sha = match sha {
                Some(s) => s,
                None => {
                    let (h, p) = split_url(&format!("{url}.sha256"))?;
                    String::from_utf8_lossy(&http_get(&h, &p)?).split_whitespace().next()?.to_lowercase()
                }
            };
            Some(Found { version: tag, url, sha, manual })
        })();
        match found {
            Some(f) => post(WM_UPD_FOUND, f),
            None => post(WM_UPD_NONE, manual),
        }
    });
}

/// Télécharge, vérifie l'empreinte, puis signale que la mise à jour est prête.
pub fn download(f: Found) {
    std::thread::spawn(move || {
        let res = (|| -> Result<String, String> {
            let (h, p) = split_url(&f.url).ok_or("URL invalide")?;
            let bytes = http_get(&h, &p).ok_or("téléchargement impossible")?;
            if sha256_hex(&bytes) != f.sha.to_lowercase() {
                return Err("empreinte SHA-256 incorrecte : mise à jour refusée".into());
            }
            let tmp = std::env::temp_dir().join("winnote-update.exe");
            std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
            Ok(tmp.to_string_lossy().to_string())
        })();
        post(WM_UPD_READY, res);
    });
}

/// Remplace l'exe courant par le fichier téléchargé et relance WinNote.
pub fn apply(new_exe: &str) -> Result<(), String> {
    let cur = std::env::current_exe().map_err(|e| e.to_string())?;
    let old = cur.with_extension("old.exe");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&cur, &old).map_err(|e| format!("remplacement impossible ({e}) : dossier non inscriptible ?"))?;
    if let Err(e) = std::fs::copy(new_exe, &cur) {
        let _ = std::fs::rename(&old, &cur);
        return Err(e.to_string());
    }
    std::process::Command::new(&cur).arg("--updated").spawn().map_err(|e| e.to_string())?;
    Ok(())
}

/// Au démarrage : supprime l'ancien exe laissé par la mise à jour précédente.
pub fn cleanup() {
    if let Ok(cur) = std::env::current_exe() {
        let _ = std::fs::remove_file(cur.with_extension("old.exe"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_and_json() {
        assert!(newer("v0.2.0", "0.1.9") && !newer("0.1.0", "0.1.0") && newer("v1.0.0", "0.9.9"));
        assert_eq!(json_str("{\"tag_name\":\"v0.2.0\",\"x\":1}", "tag_name").as_deref(), Some("v0.2.0"));
    }
    #[test]
    fn sha() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
