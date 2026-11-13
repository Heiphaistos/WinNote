//! Document : encodage (détection/conversion) et état d'un onglet.
#[cfg(windows)]
use windows_sys::Win32::Globalization::{MultiByteToWideChar, WideCharToMultiByte};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Enc {
    Utf8, Utf8Bom, Utf16Le, Utf16Be, Ansi,
    Win1250, Win1251, Win1253, Win1254, Win1255, Win1256, Win1257, Win1258, Latin9, Koi8R, ShiftJis, Gbk, Big5, EucKr,
}

impl Enc {
    pub fn name(self) -> &'static str {
        match self {
            Enc::Utf8 => "UTF-8", Enc::Utf8Bom => "UTF-8 avec BOM", Enc::Utf16Le => "UTF-16 LE", Enc::Utf16Be => "UTF-16 BE", Enc::Ansi => "ANSI",
            Enc::Win1250 => "Europe centrale (Windows-1250)", Enc::Win1251 => "Cyrillique (Windows-1251)", Enc::Win1253 => "Grec (Windows-1253)",
            Enc::Win1254 => "Turc (Windows-1254)", Enc::Win1255 => "Hébreu (Windows-1255)", Enc::Win1256 => "Arabe (Windows-1256)",
            Enc::Win1257 => "Baltique (Windows-1257)", Enc::Win1258 => "Vietnamien (Windows-1258)", Enc::Latin9 => "Latin-9 (ISO-8859-15)",
            Enc::Koi8R => "Cyrillique (KOI8-R)", Enc::ShiftJis => "Japonais (Shift-JIS)", Enc::Gbk => "Chinois simplifié (GBK)",
            Enc::Big5 => "Chinois traditionnel (Big5)", Enc::EucKr => "Coréen (EUC-KR)",
        }
    }
    /// Les 5 premiers sont les encodages usuels ; les suivants vont dans « Autres encodages ».
    pub const ALL: [Enc; 19] = [
        Enc::Utf8, Enc::Utf8Bom, Enc::Utf16Le, Enc::Utf16Be, Enc::Ansi,
        Enc::Win1250, Enc::Win1251, Enc::Win1253, Enc::Win1254, Enc::Win1255, Enc::Win1256, Enc::Win1257, Enc::Win1258, Enc::Latin9, Enc::Koi8R,
        Enc::ShiftJis, Enc::Gbk, Enc::Big5, Enc::EucKr,
    ];
    /// Page de code Windows (0 = ANSI du système) pour les encodages à un ou deux octets.
    #[cfg(windows)]
    fn codepage(self) -> u32 {
        match self {
            Enc::Win1250 => 1250, Enc::Win1251 => 1251, Enc::Win1253 => 1253, Enc::Win1254 => 1254, Enc::Win1255 => 1255, Enc::Win1256 => 1256,
            Enc::Win1257 => 1257, Enc::Win1258 => 1258, Enc::Latin9 => 28605, Enc::Koi8R => 20866, Enc::ShiftJis => 932, Enc::Gbk => 936,
            Enc::Big5 => 950, Enc::EucKr => 949, _ => 0,
        }
    }
    /// Étiquette `encoding_rs` (Linux / macOS).
    #[cfg(unix)]
    fn label(self) -> &'static str {
        match self {
            Enc::Win1250 => "windows-1250", Enc::Win1251 => "windows-1251", Enc::Win1253 => "windows-1253", Enc::Win1254 => "windows-1254",
            Enc::Win1255 => "windows-1255", Enc::Win1256 => "windows-1256", Enc::Win1257 => "windows-1257", Enc::Win1258 => "windows-1258",
            Enc::Latin9 => "iso-8859-15", Enc::Koi8R => "koi8-r", Enc::ShiftJis => "shift_jis", Enc::Gbk => "gbk", Enc::Big5 => "big5",
            Enc::EucKr => "euc-kr", _ => "windows-1252",
        }
    }
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
    pub follow: bool,
    pub pinned: bool,
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

#[cfg(windows)]
fn to_utf8(b: &[u8], e: Enc) -> Vec<u8> {
    if b.is_empty() { return Vec::new(); }
    let cp = e.codepage();
    unsafe {
        let n = MultiByteToWideChar(cp, 0, b.as_ptr(), b.len() as i32, std::ptr::null_mut(), 0);
        let mut w = vec![0u16; n.max(0) as usize];
        MultiByteToWideChar(cp, 0, b.as_ptr(), b.len() as i32, w.as_mut_ptr(), n);
        String::from_utf16_lossy(&w).into_bytes()
    }
}

#[cfg(windows)]
fn from_utf8(b: &[u8], e: Enc) -> Vec<u8> {
    let w: Vec<u16> = String::from_utf8_lossy(b).encode_utf16().collect();
    if w.is_empty() { return Vec::new(); }
    let cp = e.codepage();
    unsafe {
        let n = WideCharToMultiByte(cp, 0, w.as_ptr(), w.len() as i32, std::ptr::null_mut(), 0, std::ptr::null(), std::ptr::null_mut());
        let mut o = vec![0u8; n.max(0) as usize];
        WideCharToMultiByte(cp, 0, w.as_ptr(), w.len() as i32, o.as_mut_ptr(), n, std::ptr::null(), std::ptr::null_mut());
        o
    }
}

/// Windows-1252 (page de code « ANSI » usuelle) : table des 0x80..0x9F, le reste = Latin-1.
#[cfg(unix)]
const CP1252: [char; 32] = ['\u{20AC}', '\u{81}', '\u{201A}', '\u{192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}', '\u{2C6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8D}', '\u{17D}', '\u{8F}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}', '\u{2DC}', '\u{2122}', '\u{161}', '\u{203A}', '\u{153}', '\u{9D}', '\u{17E}', '\u{178}'];

#[cfg(unix)]
fn to_utf8(b: &[u8], e: Enc) -> Vec<u8> {
    if e != Enc::Ansi {
        return encoding_rs::Encoding::for_label(e.label().as_bytes()).map_or_else(|| b.to_vec(), |x| x.decode_without_bom_handling(b).0.into_owned().into_bytes());
    }
    b.iter().map(|&c| if (0x80..0xA0).contains(&c) { CP1252[(c - 0x80) as usize] } else { c as char }).collect::<String>().into_bytes()
}

#[cfg(unix)]
fn from_utf8(b: &[u8], e: Enc) -> Vec<u8> {
    if e != Enc::Ansi {
        return encoding_rs::Encoding::for_label(e.label().as_bytes()).map_or_else(|| b.to_vec(), |x| x.encode(&String::from_utf8_lossy(b)).0.into_owned());
    }
    String::from_utf8_lossy(b)
        .chars()
        .map(|c| match c as u32 {
            0..=0x7F | 0xA0..=0xFF => c as u8,
            _ => CP1252.iter().position(|&x| x == c).map_or(b'?', |i| 0x80 + i as u8),
        })
        .collect()
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
    if std::str::from_utf8(b).is_ok() { (b.to_vec(), Enc::Utf8) } else { (to_utf8(b, Enc::Ansi), Enc::Ansi) }
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
        _ => from_utf8(t, e),
    }
}

/// Réinterprète les octets d'un fichier avec l'encodage choisi (« Recharger avec l'encodage »).
pub fn decode_as(b: &[u8], e: Enc) -> Vec<u8> {
    match e {
        Enc::Utf8 | Enc::Utf8Bom => String::from_utf8_lossy(b.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(b)).into_owned().into_bytes(),
        Enc::Utf16Le => utf16(b.strip_prefix(&[0xFF, 0xFE]).unwrap_or(b), false),
        Enc::Utf16Be => utf16(b.strip_prefix(&[0xFE, 0xFF]).unwrap_or(b), true),
        _ => to_utf8(b, e),
    }
}

/// Vrai si l'enregistrement avec cet encodage ne perd aucun caractère.
pub fn lossless(t: &[u8], e: Enc) -> bool {
    matches!(e, Enc::Utf8 | Enc::Utf8Bom | Enc::Utf16Le | Enc::Utf16Be) || decode_as(&encode(t, e), e) == t
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
        for e in [Enc::Utf8, Enc::Utf8Bom, Enc::Utf16Le, Enc::Utf16Be, Enc::Ansi, Enc::Win1250, Enc::Latin9] {
            let s = "héllo\r\nwörld".as_bytes();
            assert_eq!(decode_as(&encode(s, e), e), s, "{}", e.name());
        }
        let (d, _) = decode(&encode("héllo".as_bytes(), Enc::Ansi));
        assert_eq!(d, "héllo".as_bytes());
        let cyr = "Привет";
        assert_eq!(encode(cyr.as_bytes(), Enc::Win1251)[0], 0xCF);
        assert_eq!(decode_as(&encode(cyr.as_bytes(), Enc::Win1251), Enc::Win1251), cyr.as_bytes());
        assert!(lossless(cyr.as_bytes(), Enc::Win1251) && !lossless(cyr.as_bytes(), Enc::Ansi));
        let jp = "日本語";
        assert_eq!(decode_as(&encode(jp.as_bytes(), Enc::ShiftJis), Enc::ShiftJis), jp.as_bytes());
        assert_eq!(detect_eol(b"a\nb"), 2);
        assert_eq!(detect_eol(b"a\r\nb"), 0);
    }
}
