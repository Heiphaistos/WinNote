//! Mise à jour (Linux / macOS) : dernière release GitHub, téléchargement avec contrôle SHA-256, remplacement de l'AppImage et relance.
//! Hors AppImage (.deb, .rpm, .tar.gz, Arch, macOS) on signale la nouvelle version et on ouvre la page de téléchargement :
//! un paquet géré par le système doit être mis à jour par le système, pas par l'application.
use crate::vers::{json_str, newer};
use crate::{app, edit};
use gtk::prelude::*;
use std::process::Command;
use std::sync::Mutex;

pub const REPO: &str = "Heiphaistos/WinNote";
const PAGE: &str = "https://github.com/Heiphaistos/WinNote/releases/latest";

#[derive(Clone)]
pub struct Found {
    pub version: String,
    pub url: String,
    pub sha: String,
}
pub type Check = Result<Option<Found>, String>;

enum Msg {
    Found(Found),
    Info(bool, String),
    Ready(Result<String, String>),
}
static INBOX: Mutex<Vec<Msg>> = Mutex::new(Vec::new());
static PROMPTING: Mutex<bool> = Mutex::new(false);

fn push(m: Msg) {
    if let Ok(mut v) = INBOX.lock() {
        v.push(m);
    }
}

fn api_url() -> String {
    // WINNOTE_UPDATE_URL permet de tester la chaîne sur un fichier local (file://...)
    std::env::var("WINNOTE_UPDATE_URL").unwrap_or_else(|_| format!("https://api.github.com/repos/{REPO}/releases/latest"))
}

fn curl(url: &str, out: Option<&str>) -> Result<Vec<u8>, String> {
    let mut c = Command::new("curl");
    c.args(["-fsSL", "--max-time", "300", "-A", "WinNote-updater"]);
    if let Some(o) = out {
        c.args(["-o", o]);
    }
    let o = c.arg(url).output().map_err(|e| format!("curl indisponible : {e}"))?;
    if !o.status.success() {
        return Err(format!("téléchargement impossible ({}) : {url}", o.status));
    }
    Ok(o.stdout)
}

pub fn sha256_file(p: &str) -> Result<String, String> {
    for (prog, args) in [("sha256sum", vec![]), ("shasum", vec!["-a", "256"])] {
        if let Ok(o) = Command::new(prog).args(&args).arg(p).output() {
            if o.status.success() {
                return Ok(String::from_utf8_lossy(&o.stdout).split_whitespace().next().unwrap_or("").to_lowercase());
            }
        }
    }
    Err("ni sha256sum ni shasum n'est disponible".into())
}

/// L'AppImage en cours d'exécution (seul format que l'application remplace elle-même).
fn appimage() -> Option<String> {
    std::env::var("APPIMAGE").ok().filter(|p| !p.is_empty())
}

/// Analyse la réponse de l'API « dernière release ». Un `Found` sans `url` = nouvelle version sans AppImage téléchargeable.
pub fn parse_release(body: &str, local: &str) -> Check {
    let tag = json_str(body, "tag_name").ok_or("réponse GitHub illisible (pas de tag_name)")?;
    if !newer(&tag, local) {
        return Ok(None);
    }
    let none = Found { version: tag.clone(), url: String::new(), sha: String::new() };
    let Some(ai) = body.find("-x86_64.AppImage\"") else { return Ok(Some(none)) };
    let Some(url) = json_str(&body[ai..], "browser_download_url") else { return Ok(Some(none)) };
    let sha = match body[ai..].find("\"digest\":\"sha256:") {
        Some(d) if body.len() >= ai + d + 17 + 64 => body[ai + d + 17..ai + d + 17 + 64].to_string(),
        _ => return Err("la release ne fournit pas l'empreinte SHA-256 de l'AppImage".into()),
    };
    Ok(Some(Found { version: tag, url, sha }))
}

pub fn fetch() -> Check {
    let body = String::from_utf8_lossy(&curl(&api_url(), None)?).to_string();
    parse_release(&body, env!("CARGO_PKG_VERSION"))
}

/// Vérification en arrière-plan ; le résultat revient par la boîte aux lettres lue par le minuteur GTK.
pub fn check(manual: bool) {
    std::thread::spawn(move || match fetch() {
        Ok(Some(f)) => push(Msg::Found(f)),
        Ok(None) => push(Msg::Info(manual, "WinNote est à jour.".into())),
        Err(e) => push(Msg::Info(manual, format!("Recherche de mise à jour impossible : {e}"))),
    });
}

/// Télécharge à côté de la cible, vérifie l'empreinte, puis remplace (bloquant).
pub fn install(f: &Found, target: &str) -> Result<(), String> {
    let tmp = format!("{target}.new");
    curl(&f.url, Some(&tmp))?;
    if sha256_file(&tmp).as_deref() != Ok(f.sha.to_lowercase().as_str()) {
        let _ = std::fs::remove_file(&tmp);
        return Err("empreinte SHA-256 incorrecte : mise à jour refusée".into());
    }
    replace(&tmp, target)
}

fn download(f: Found, target: String) {
    std::thread::spawn(move || push(Msg::Ready(install(&f, &target).map(|_| target.clone()))));
}

/// Rend `new` exécutable et le renomme par-dessus `target` (atomique, même volume).
pub fn replace(new: &str, target: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(new, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    std::fs::rename(new, target).map_err(|e| format!("remplacement impossible ({e}) : dossier non inscriptible ?"))
}

fn ask_install(f: &Found) {
    let a = app();
    let can = appimage().filter(|_| !f.url.is_empty());
    let text = format!("WinNote {} est disponible (version installée : {}).", f.version.trim_start_matches('v'), env!("CARGO_PKG_VERSION"));
    let d = gtk::MessageDialog::new(Some(&a.win), gtk::DialogFlags::MODAL, gtk::MessageType::Question, gtk::ButtonsType::None, &text);
    d.add_button("Plus tard", gtk::ResponseType::Cancel);
    d.add_button(if can.is_some() { "Installer et redémarrer" } else { "Ouvrir la page de téléchargement" }, gtk::ResponseType::Accept);
    let r = d.run();
    d.close();
    if r != gtk::ResponseType::Accept {
        return;
    }
    match can {
        Some(target) => download(f.clone(), target),
        None => crate::textops::open_url(PAGE),
    }
}

/// Lit la boîte aux lettres (appelé par le minuteur GTK, thread principal).
fn drain() {
    let msgs: Vec<Msg> = INBOX.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default();
    for m in msgs {
        match m {
            Msg::Found(f) => {
                let mut busy = PROMPTING.lock().unwrap();
                if !*busy {
                    *busy = true;
                    drop(busy);
                    ask_install(&f);
                    *PROMPTING.lock().unwrap() = false;
                }
            }
            Msg::Info(manual, t) => {
                if manual {
                    edit::msg_box(&t);
                }
            }
            Msg::Ready(Ok(target)) => {
                if Command::new(&target).arg("--updated").spawn().is_ok() {
                    crate::ui::quit();
                } else {
                    edit::msg_box("Mise à jour installée. Relancez WinNote.");
                }
            }
            Msg::Ready(Err(e)) => edit::msg_box(&format!("Mise à jour impossible : {e}")),
        }
    }
}

/// Démarre la surveillance : lecture de la boîte aux lettres, vérification au démarrage puis toutes les 6 h.
pub fn start() {
    gtk::glib::timeout_add_seconds_local(1, || {
        drain();
        gtk::glib::ControlFlow::Continue
    });
    gtk::glib::timeout_add_seconds_local_once(8, || {
        if app().auto_update {
            check(false);
        }
    });
    gtk::glib::timeout_add_seconds_local(6 * 3600, || {
        if app().auto_update {
            check(false);
        }
        gtk::glib::ControlFlow::Continue
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    const API: &str = r#"{"tag_name":"v0.9.0","assets":[{"name":"WinNote-0.9.0-x86_64.AppImage","digest":"sha256:7e525ac96a9f330b49f375b7d9a1af32486ce8c26b1588b7034b2242d4cb64da","browser_download_url":"https://github.com/x/y/releases/download/v0.9.0/WinNote-0.9.0-x86_64.AppImage"}]}"#;

    #[test]
    fn release_parsing() {
        let f = parse_release(API, "0.3.0").unwrap().unwrap();
        assert!(f.url.ends_with(".AppImage") && f.sha.len() == 64);
        assert!(parse_release(API, "0.9.0").unwrap().is_none());
        assert!(parse_release(r#"{"tag_name":"v1.0.0"}"#, "0.9.0").unwrap().unwrap().url.is_empty());
        assert!(parse_release("{}", "0.1.0").is_err());
    }
}
