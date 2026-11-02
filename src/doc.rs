//! Document : encodage (détection/conversion) et état d'un onglet.
use windows_sys::Win32::Globalization::{MultiByteToWideChar, WideCharToMultiByte};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Enc { Utf8, Utf8Bom, Utf16Le, Utf16Be, Ansi }

impl Enc {
    pub fn name(self) -> &'static str {
        match self { Enc::Utf8 => "UTF-8", Enc::Utf8Bom => "UTF-8 avec BOM", Enc::Utf16Le => "UTF-16 LE",
                     Enc::Utf16Be => "UTF-16 BE", Enc::Ansi => "ANSI" }
    }
    pub const ALL: [Enc; 5] = [Enc::Utf8, Enc::Utf8Bom, Enc::Utf16Le, Enc::Utf16Be, Enc::Ansi];
}

pub struct Doc {
    pub ptr: isize,
    pub path: Option<String>,
    pub enc: Enc,
    pub lang: usize,
    pub caret: isize,
    pub first: isize,
    pub untitled: u32,
    pub dirty: bool,
    pub mtime: Option<std::time::SystemTime>,
}

impl Doc {
    pub fn title(&self) -> String {
        match &self.path {
            Some(p) => p.rsplit(['\\', '/']).next().unwrap_or(p).to_string(),
            None if self.untitled <= 1 => "Sans titre".into(),
            None => format!("Sans titre {}", self.untitled),
        }
    }
}

fn ansi_to_utf8(b: &[u8]) -> Vec<u8> {
    if b.is_empty() { return Vec::new(); }
    unsafe {
        let n = MultiByteToWideChar(0, 0, b.as_ptr(), b.len() as i32, std::ptr::null_mut(), 0);
        let mut w = vec![0u16; n as usize];
        MultiByteToWideChar(0, 0, b.as_ptr(), b.len() as i32, w.as_mut_ptr(), n);
        String::from_utf16_lossy(&w).into_bytes()
    }
}

fn utf8_to_ansi(b: &[u8]) -> Vec<u8> {
    let w: Vec<u16> = String::from_utf8_lossy(b).encode_utf16().collect();
    if w.is_empty() { return Vec::new(); }
    unsafe {
        let n = WideCharToMultiByte(0, 0, w.as_ptr(), w.len() as i32, std::ptr::null_mut(), 0, std::ptr::null(), std::ptr::null_mut());
        let mut o = vec![0u8; n as usize];
        WideCharToMultiByte(0, 0, w.as_ptr(), w.len() as i32, o.as_mut_ptr(), n, std::ptr::null(), std::ptr::null_mut());
        o
    }
}

fn utf16(b: &[u8], be: bool) -> Vec<u8> {
    let u: Vec<u16> = b.chunks_exact(2).map(|c| if be { u16::from_be_bytes([c[0], c[1]]) } else { u16::from_le_bytes([c[0], c[1]]) }).collect();
    String::from_utf16_lossy(&u).into_bytes()
}

/// Octets du fichier -> (texte UTF-8, encodage détecté).
pub fn decode(b: &[u8]) -> (Vec<u8>, Enc) {
    if b.starts_with(&[0xEF, 0xBB, 0xBF]) { return (b[3..].to_vec(), Enc::Utf8Bom); }
    if b.starts_with(&[0xFF, 0xFE]) { return (utf16(&b[2..], false), Enc::Utf16Le); }
    if b.starts_with(&[0xFE, 0xFF]) { return (utf16(&b[2..], true), Enc::Utf16Be); }
    if std::str::from_utf8(b).is_ok() { (b.to_vec(), Enc::Utf8) } else { (ansi_to_utf8(b), Enc::Ansi) }
}

/// Texte UTF-8 de l'éditeur -> octets à écrire.
pub fn encode(t: &[u8], e: Enc) -> Vec<u8> {
    match e {
        Enc::Utf8 => t.to_vec(),
        Enc::Utf8Bom => { let mut v = vec![0xEF, 0xBB, 0xBF]; v.extend_from_slice(t); v }
        Enc::Utf16Le | Enc::Utf16Be => {
            let be = e == Enc::Utf16Be;
            let mut v = if be { vec![0xFE, 0xFF] } else { vec![0xFF, 0xFE] };
            for u in String::from_utf8_lossy(t).encode_utf16() {
                v.extend_from_slice(&if be { u.to_be_bytes() } else { u.to_le_bytes() });
            }
            v
        }
        Enc::Ansi => utf8_to_ansi(t),
    }
}

/// Mode de fin de ligne dominant (0 CRLF, 1 CR, 2 LF) ; CRLF par défaut.
pub fn detect_eol(t: &[u8]) -> usize {
    for (i, &c) in t.iter().enumerate() {
        if c == b'\n' { return 2; }
        if c == b'\r' { return if t.get(i + 1) == Some(&b'\n') { 0 } else { 1 }; }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        for e in Enc::ALL {
            let s = "héllo\r\nwörld".as_bytes();
            let (d, _) = decode(&encode(s, e));
            assert_eq!(d, s, "{}", e.name());
        }
        assert_eq!(detect_eol(b"a\nb"), 2);
        assert_eq!(detect_eol(b"a\r\nb"), 0);
    }
}
