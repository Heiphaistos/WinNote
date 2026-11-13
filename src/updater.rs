//! Mise à jour automatique : vérifie la dernière release GitHub, télécharge, contrôle le SHA-256, remplace l'exe et relance.
//! Exige une release publique contenant `winnote.exe` (empreinte `digest` fournie par GitHub, ou fichier `winnote.exe.sha256`).
use crate::{app, w};
use std::ptr::null;
use windows_sys::Win32::{
    Foundation::GetLastError,
    Networking::WinHttp::*,
    Security::Cryptography::{BCryptHash, BCRYPT_SHA256_ALG_HANDLE},
    UI::WindowsAndMessaging::PostMessageW,
};

pub const REPO: &str = "Heiphaistos/WinNote";
pub const WM_UPD_FOUND: u32 = 0x8001;
pub const WM_UPD_READY: u32 = 0x8002;
pub const WM_UPD_NONE: u32 = 0x8003;

#[derive(Debug)]
pub struct Found { pub version: String, pub url: String, pub sha: String }

/// Résultat d'une vérification : Ok(Some) = nouvelle version, Ok(None) = à jour, Err = raison lisible.
pub type Check = Result<Option<Found>, String>;

struct Handle(*mut std::ffi::c_void);
impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { WinHttpCloseHandle(self.0) };
        }
    }
}

fn http_get(host: &str, path: &str) -> Result<Vec<u8>, String> {
    unsafe {
        let err = |what: &str| format!("{what} (code Windows {})", GetLastError());
        let ses = Handle(WinHttpOpen(w("WinNote-updater").as_ptr(), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, null(), null(), 0));
        if ses.0.is_null() {
            return Err(err("WinHTTP indisponible"));
        }
        let con = Handle(WinHttpConnect(ses.0, w(host).as_ptr(), 443, 0));
        if con.0.is_null() {
            return Err(err("connexion impossible"));
        }
        let req = Handle(WinHttpOpenRequest(con.0, w("GET").as_ptr(), w(path).as_ptr(), null(), null(), null(), WINHTTP_FLAG_SECURE));
        if req.0.is_null() {
            return Err(err("requête impossible"));
        }
        if WinHttpSendRequest(req.0, null(), 0, null(), 0, 0, 0) == 0 {
            return Err(err("envoi impossible"));
        }
        if WinHttpReceiveResponse(req.0, std::ptr::null_mut()) == 0 {
            return Err(err("pas de réponse"));
        }
        let (mut status, mut len) = (0u32, 4u32);
        WinHttpQueryHeaders(req.0, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, null(), &mut status as *mut u32 as *mut _, &mut len, std::ptr::null_mut());
        if status != 200 {
            return Err(format!("le serveur a répondu {status} pour {host}{path}"));
        }
        let mut out = Vec::new();
        loop {
            let mut n = 0u32;
            if WinHttpQueryDataAvailable(req.0, &mut n) == 0 || n == 0 {
                break;
            }
            let mut buf = vec![0u8; n as usize];
            let mut got = 0u32;
            if WinHttpReadData(req.0, buf.as_mut_ptr() as *mut _, n, &mut got) == 0 || got == 0 {
                break;
            }
            out.extend_from_slice(&buf[..got as usize]);
            if out.len() > 64 * 1024 * 1024 {
                return Err("fichier trop gros".into());
            }
        }
        Ok(out)
    }
}

fn split_url(u: &str) -> Option<(String, String)> {
    let r = u.strip_prefix("https://")?;
    let (h, p) = r.split_once('/')?;
    Some((h.to_string(), format!("/{p}")))
}

pub use crate::vers::{digest_in, json_str, newer, trusted_download};

pub fn sha256_hex(data: &[u8]) -> String {
    let mut out = [0u8; 32];
    unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, std::ptr::null_mut(), 0, data.as_ptr() as *mut u8, data.len() as u32, out.as_mut_ptr(), 32) };
    out.iter().map(|b| format!("{b:02x}")).collect()
}

/// Analyse la réponse de l'API « dernière release ». `local` = version actuelle.
pub fn parse_release(body: &str, local: &str) -> Check {
    let tag = json_str(body, "tag_name").ok_or("réponse GitHub illisible (pas de tag_name)")?;
    if !newer(&tag, local) {
        return Ok(None);
    }
    let ai = body.find("winnote.exe\"").ok_or("la release ne contient pas winnote.exe")?;
    let url = json_str(&body[ai..], "browser_download_url").ok_or("lien de téléchargement introuvable")?;
    if !trusted_download(&url) {
        return Err(format!("lien de téléchargement refusé (seul github.com en HTTPS est accepté) : {url}"));
    }
    let sha = match digest_in(&body[ai..]) {
        Some(h) => h,
        None => {
            let (h, p) = split_url(&format!("{url}.sha256")).ok_or("URL invalide")?;
            let t = String::from_utf8_lossy(&http_get(&h, &p)?).to_string();
            let h = t.split_whitespace().next().ok_or("fichier .sha256 vide")?.to_lowercase();
            if h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("fichier .sha256 invalide".into());
            }
            h
        }
    };
    Ok(Some(Found { version: tag, url, sha }))
}

pub fn fetch() -> Check {
    let body = String::from_utf8_lossy(&http_get("api.github.com", &format!("/repos/{REPO}/releases/latest"))?).to_string();
    parse_release(&body, env!("CARGO_PKG_VERSION"))
}

fn post<T>(msg: u32, v: T) {
    let p = Box::into_raw(Box::new(v)) as isize;
    unsafe { PostMessageW(app().hwnd, msg, 0, p) };
}

/// Vérification en arrière-plan ; le résultat revient par message à la fenêtre principale (`manual` = afficher aussi « à jour » et les erreurs).
pub fn check(manual: bool) {
    std::thread::spawn(move || match fetch() {
        Ok(Some(f)) => post(WM_UPD_FOUND, f),
        Ok(None) => post(WM_UPD_NONE, (manual, "WinNote est à jour.".to_string())),
        Err(e) => post(WM_UPD_NONE, (manual, format!("Recherche de mise à jour impossible : {e}"))),
    });
}

/// Télécharge, vérifie l'empreinte, puis signale que la mise à jour est prête.
pub fn download(f: Found) {
    std::thread::spawn(move || {
        let res = (|| -> Result<String, String> {
            let (h, p) = split_url(&f.url).ok_or("URL invalide")?;
            let bytes = http_get(&h, &p)?;
            if sha256_hex(&bytes) != f.sha.to_lowercase() {
                return Err("empreinte SHA-256 incorrecte : mise à jour refusée".into());
            }
            // create_new : un fichier ou un lien déposé d'avance sous ce nom dans %TEMP% est refusé, pas écrasé
            let tmp = std::env::temp_dir().join(format!("winnote-update-{}.exe", std::process::id()));
            let _ = std::fs::remove_file(&tmp);
            {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp).map_err(|e| e.to_string())?;
                file.write_all(&bytes).map_err(|e| e.to_string())?;
            }
            *LAST_SHA.lock().unwrap_or_else(|e| e.into_inner()) = Some(f.sha.to_lowercase());
            Ok(tmp.to_string_lossy().to_string())
        })();
        post(WM_UPD_READY, res);
    });
}

/// Empreinte vérifiée du dernier téléchargement : contrôlée à nouveau juste avant l'installation (le fichier est resté dans %TEMP%).
static LAST_SHA: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Remplace l'exe courant par le fichier téléchargé et relance WinNote.
/// Si une empreinte est mémorisée (téléchargement par l'application), le fichier est re-haché : modifié depuis, il est refusé.
pub fn apply(new_exe: &str) -> Result<(), String> {
    let want = LAST_SHA.lock().unwrap_or_else(|e| e.into_inner()).clone();
    if let Some(want) = want {
        let bytes = std::fs::read(new_exe).map_err(|e| e.to_string())?;
        if sha256_hex(&bytes) != want {
            let _ = std::fs::remove_file(new_exe);
            return Err("le fichier téléchargé a changé depuis la vérification : mise à jour refusée".into());
        }
    }
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
    const API: &str = r#"{"tag_name":"v0.9.0","assets":[{"name":"winnote.exe","digest":"sha256:7e525ac96a9f330b49f375b7d9a1af32486ce8c26b1588b7034b2242d4cb64da","browser_download_url":"https://github.com/x/y/releases/download/v0.9.0/winnote.exe"}]}"#;

    #[test]
    fn versions_and_json() {
        assert!(newer("v0.2.0", "0.1.9") && !newer("0.1.0", "0.1.0") && newer("v1.0.0", "0.9.9"));
        assert_eq!(json_str("{\"tag_name\":\"v0.2.0\",\"x\":1}", "tag_name").as_deref(), Some("v0.2.0"));
    }
    #[test]
    fn sha() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
    #[test]
    fn release_parsing() {
        let f = parse_release(API, "0.3.0").unwrap().unwrap();
        assert_eq!(f.version, "v0.9.0");
        assert!(f.url.ends_with("/winnote.exe") && f.sha.len() == 64);
        assert!(parse_release(API, "0.9.0").unwrap().is_none());
        assert!(parse_release("{}", "0.1.0").is_err());
    }
    #[test]
    fn hostile_release_refused() {
        let evil = API.replace("https://github.com/x/y", "https://evil.example/x");
        assert!(parse_release(&evil, "0.3.0").unwrap_err().contains("refusé"));
        let http = API.replace("https://github.com", "http://github.com");
        assert!(parse_release(&http, "0.3.0").is_err());
        // empreinte tronquée par des caractères multi-octets : erreur propre, pas de panique (panic = abort)
        let cut = API.replace("7e525ac96a9f330b49f375b7d9a1af32486ce8c26b1588b7034b2242d4cb64da", "7e525ac96a9f330b49f375b7d9a1af32éééééééééééééééééééééééééééééé");
        let _ = parse_release(&cut, "0.3.0");
    }
}
