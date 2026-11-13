//! Options de recherche partagées (Windows / Linux / macOS) : mode étendu (\n \t \xNN), recherche dans la sélection, historique.
use std::cell::{Cell, RefCell};

const BS: u8 = 92; // barre oblique inverse

thread_local! {
    static EXT: Cell<bool> = const { Cell::new(false) };
    static SEL: Cell<Option<(isize, isize)>> = const { Cell::new(None) };
    static HIST: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static PENDING: Cell<Option<(isize, isize)>> = const { Cell::new(None) };
    static HPOS: Cell<usize> = const { Cell::new(0) };
}

pub fn set_ext(b: bool) { EXT.with(|e| e.set(b)) }
pub fn ext() -> bool { EXT.with(|e| e.get()) }
pub fn set_sel(s: Option<(isize, isize)>) { SEL.with(|e| e.set(s)) }
/// Bornes de recherche : la sélection mémorisée, sinon tout le document.
pub fn range(len: isize) -> (isize, isize) {
    SEL.with(|e| e.get()).map_or((0, len), |(a, b)| (a.min(len), b.min(len)))
}
/// Après un remplacement dans la sélection : la fin se déplace de `d` octets.
pub fn shift_end(d: isize) {
    SEL.with(|e| if let Some((a, b)) = e.get() { e.set(Some((a, (b + d).max(a)))) })
}
/// Sélection multi-lignes présente à l'ouverture de la barre : c'est elle que « dans la sélection » vise.
pub fn set_pending(s: Option<(isize, isize)>) { PENDING.with(|e| e.set(s)) }
pub fn pending() -> Option<(isize, isize)> { PENDING.with(|e| e.get()) }
pub fn in_sel() -> bool { SEL.with(|e| e.get()).is_some() }

/// Interprète \n \r \t \0 \\ et \xNN (mode « étendu » de Notepad++).
pub fn unescape(b: &[u8]) -> Vec<u8> {
    let (mut o, mut i) = (Vec::with_capacity(b.len()), 0);
    while i < b.len() {
        if b[i] == BS && i + 1 < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            match b[i + 1] {
                b'n' => o.push(10),
                b'r' => o.push(13),
                b't' => o.push(9),
                b'0' => o.push(0),
                BS => o.push(BS),
                b'x' if i + 3 < b.len() && hex(b[i + 2]).is_some() && hex(b[i + 3]).is_some() => {
                    o.push((hex(b[i + 2]).unwrap_or(0) * 16 + hex(b[i + 3]).unwrap_or(0)) as u8);
                    i += 2;
                }
                c => o.extend_from_slice(&[BS, c]),
            }
            i += 2;
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    o
}

/// Applique le mode étendu si actif (sauf en regex, qui gère déjà \n \t).
pub fn prep(b: Vec<u8>, regex: bool) -> Vec<u8> {
    if ext() && !regex { unescape(&b) } else { b }
}

pub fn hist_push(q: &str) {
    if q.is_empty() { return; }
    HIST.with(|h| {
        let mut h = h.borrow_mut();
        h.retain(|x| x != q);
        h.push(q.to_string());
        if h.len() > 30 { h.remove(0); }
        HPOS.with(|p| p.set(h.len()));
    });
}
/// Historique : `up` = plus ancien. Renvoie l'entrée à afficher.
pub fn hist_step(up: bool) -> Option<String> {
    HIST.with(|h| {
        let h = h.borrow();
        if h.is_empty() { return None; }
        let p = HPOS.with(|p| p.get()).min(h.len());
        let np = if up { p.saturating_sub(1) } else { (p + 1).min(h.len()) };
        HPOS.with(|x| x.set(np));
        Some(h.get(np).cloned().unwrap_or_default())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn esc() {
        assert_eq!(unescape(&[b'a', BS, b'n', b'b', BS, b't', BS, b'x', b'4', b'1', BS, BS]), vec![b'a', 10, b'b', 9, b'A', BS]);
        assert_eq!(unescape(&[BS, b'd']), vec![BS, b'd']);
        hist_push("a");
        hist_push("b");
        assert_eq!(hist_step(true).as_deref(), Some("b"));
        assert_eq!(hist_step(true).as_deref(), Some("a"));
        assert_eq!(hist_step(false).as_deref(), Some("b"));
        set_sel(Some((2, 10)));
        assert_eq!(range(5), (2, 5));
        shift_end(-1);
        set_sel(None);
    }
}
