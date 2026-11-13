//! Macros (enregistrer / rejouer, comme Notepad++) : partagées Windows / Linux / macOS, basées sur SCI_STARTRECORD.
use crate::sci::{send, HWND};
use std::cell::{Cell, RefCell};

const STARTRECORD: u32 = 3001;
const STOPRECORD: u32 = 3002;
/// Messages dont lParam est un texte (copié pendant l'enregistrement) : REPLACESEL, ADDTEXT, INSERTTEXT, SEARCHNEXT, SEARCHPREV.
const TEXT_MSGS: [u32; 5] = [2170, 2001, 2003, 2367, 2368];

struct Step { msg: u32, w: usize, l: isize, text: Option<Vec<u8>> }

thread_local! {
    static REC: Cell<bool> = const { Cell::new(false) };
    static CUR: RefCell<Vec<Step>> = const { RefCell::new(Vec::new()) };
    static SAVED: RefCell<Vec<Step>> = const { RefCell::new(Vec::new()) };
}

pub fn recording() -> bool { REC.with(|r| r.get()) }

/// Démarre ou arrête l'enregistrement sur la vue `s`. Renvoie le nouvel état.
pub fn toggle(s: HWND) -> bool {
    if recording() {
        send(s, STOPRECORD, 0, 0);
        REC.with(|r| r.set(false));
        let steps = CUR.with(|c| std::mem::take(&mut *c.borrow_mut()));
        if !steps.is_empty() {
            SAVED.with(|v| *v.borrow_mut() = steps);
        }
    } else {
        CUR.with(|c| c.borrow_mut().clear());
        REC.with(|r| r.set(true));
        send(s, STARTRECORD, 0, 0);
    }
    recording()
}

/// SCN_MACRORECORD (2009) : `msg`, `w`, `l` lus dans la notification.
pub fn on_record(msg: u32, w: usize, l: isize) {
    if !recording() {
        return;
    }
    let text = (TEXT_MSGS.contains(&msg) && l != 0).then(|| {
        let p = l as *const u8;
        let mut v = Vec::new();
        let mut i = 0;
        // texte terminé par un zéro (ADDTEXT : longueur w)
        unsafe {
            while (msg != 2001 || i < w) && *p.add(i) != 0 {
                v.push(*p.add(i));
                i += 1;
            }
        }
        v.push(0);
        v
    });
    CUR.with(|c| c.borrow_mut().push(Step { msg, w, l, text }));
}

/// Rejoue la macro `times` fois (0 = jusqu'à la fin du document, comme « Exécuter jusqu'à la fin »).
pub fn play(s: HWND, times: usize) {
    send(s, crate::sci::BEGINUNDO, 0, 0);
    let mut n = 0;
    loop {
        if times == 0 {
            let (p, len) = (send(s, crate::sci::GETCURRENTPOS, 0, 0), send(s, crate::sci::GETLENGTH, 0, 0));
            if p >= len || n > 100_000 {
                break;
            }
        } else if n >= times {
            break;
        }
        let before = send(s, crate::sci::GETCURRENTPOS, 0, 0);
        SAVED.with(|v| {
            for st in v.borrow().iter() {
                let l = st.text.as_ref().map_or(st.l, |t| t.as_ptr() as isize);
                send(s, st.msg, st.w, l);
            }
        });
        n += 1;
        if times == 0 && send(s, crate::sci::GETCURRENTPOS, 0, 0) == before {
            break; // la macro n'avance pas : on évite une boucle infinie
        }
    }
    send(s, crate::sci::ENDUNDO, 0, 0);
}
