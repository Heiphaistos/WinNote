//! Intégration à l'explorateur Windows (menu « Modifier avec WinNote »), par utilisateur, sans droits administrateur.
use crate::w;
use windows_sys::Win32::System::Registry::*;

const KEY: &str = "Software\\Classes\\*\\shell\\WinNote";

fn set(key: HKEY, name: Option<&str>, val: &str) {
    let v = w(val);
    let n = name.map(w);
    unsafe { RegSetValueExW(key, n.as_ref().map_or(std::ptr::null(), |x| x.as_ptr()), 0, REG_SZ, v.as_ptr() as *const u8, (v.len() * 2) as u32) };
}

pub fn register() -> bool {
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    unsafe {
        let (mut k, mut c): (HKEY, HKEY) = (std::ptr::null_mut(), std::ptr::null_mut());
        if RegCreateKeyExW(HKEY_CURRENT_USER, w(KEY).as_ptr(), 0, std::ptr::null(), 0, KEY_WRITE, std::ptr::null(), &mut k, std::ptr::null_mut()) != 0 {
            return false;
        }
        set(k, None, "Modifier avec WinNote");
        set(k, Some("Icon"), &format!("\"{exe}\",0"));
        let ok = RegCreateKeyExW(k, w("command").as_ptr(), 0, std::ptr::null(), 0, KEY_WRITE, std::ptr::null(), &mut c, std::ptr::null_mut()) == 0;
        if ok {
            set(c, None, &format!("\"{exe}\" \"%1\""));
            RegCloseKey(c);
        }
        RegCloseKey(k);
        ok
    }
}

pub fn unregister() -> bool {
    unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, w(KEY).as_ptr()) == 0 }
}
