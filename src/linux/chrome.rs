//! Barre de titre « style macOS » (boutons rouge / jaune / vert à gauche), comme sous Windows. Linux seulement :
//! sous macOS la fenêtre a déjà ses vrais boutons. Le choix « Système » garde les décorations du gestionnaire de fenêtres.
use crate::app;
use gtk::prelude::*;

const EDGE: i32 = 6;

pub struct Bar {
    pub root: gtk::Box,
    pub title: gtk::Label,
}

fn light(class: &str, tip: &str, f: impl Fn() + 'static) -> gtk::Button {
    let b = gtk::Button::new();
    b.set_widget_name("traffic");
    b.style_context().add_class(class);
    b.set_tooltip_text(Some(tip));
    b.set_can_focus(false);
    b.connect_clicked(move |_| f());
    b
}

impl Bar {
    pub fn new() -> Bar {
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        root.set_widget_name("titlebar");
        let lights = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        lights.set_margin_start(12);
        lights.pack_start(&light("tl-close", "Fermer", crate::ui::quit), false, false, 0);
        lights.pack_start(&light("tl-min", "Réduire", || app().win.iconify()), false, false, 0);
        lights.pack_start(
            &light("tl-max", "Agrandir", || {
                let w = &app().win;
                if w.is_maximized() { w.unmaximize() } else { w.maximize() }
            }),
            false,
            false,
            0,
        );
        let title = gtk::Label::new(Some("WinNote"));
        title.set_xalign(0.5);
        root.pack_start(&lights, false, false, 0);
        root.pack_start(&title, true, true, 0);
        // une zone vide de la largeur des boutons garde le titre centré
        let pad = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        pad.set_size_request(72, 1);
        root.pack_end(&pad, false, false, 0);
        // glisser la barre déplace la fenêtre, double-clic = agrandir / restaurer
        let ev = root.clone();
        ev.add_events(gtk::gdk::EventMask::BUTTON_PRESS_MASK);
        ev.connect_button_press_event(|_, e| {
            let w = &app().win;
            if e.button() == 1 {
                if e.event_type() == gtk::gdk::EventType::DoubleButtonPress {
                    if w.is_maximized() { w.unmaximize() } else { w.maximize() }
                } else {
                    let (x, y) = e.root();
                    w.begin_move_drag(1, x as i32, y as i32, e.time());
                }
            }
            gtk::glib::Propagation::Stop
        });
        Bar { root, title }
    }
}

/// Applique le style choisi (0 = système, 1 = macOS).
pub fn apply() {
    let a = app();
    let mac = a.chrome == 1 && !cfg!(target_os = "macos");
    a.win.set_decorated(!mac);
    if mac {
        a.bar.root.show_all();
    } else {
        a.bar.root.hide();
    }
    // marge autour du contenu : poignées de redimensionnement d'une fenêtre sans décorations
    a.vbox.set_margin(if mac { EDGE } else { 0 });
}

fn edge_at(x: f64, y: f64) -> Option<gtk::gdk::WindowEdge> {
    use gtk::gdk::WindowEdge as E;
    let w = &app().win;
    let (width, height) = (w.allocated_width() as f64, w.allocated_height() as f64);
    let e = EDGE as f64 + 2.0;
    let (l, r, t, b) = (x < e, x > width - e, y < e, y > height - e);
    Some(match (l, r, t, b) {
        (true, _, true, _) => E::NorthWest,
        (_, true, true, _) => E::NorthEast,
        (true, _, _, true) => E::SouthWest,
        (_, true, _, true) => E::SouthEast,
        (true, ..) => E::West,
        (_, true, ..) => E::East,
        (_, _, true, _) => E::North,
        (.., true) => E::South,
        _ => return None,
    })
}

/// Branche le redimensionnement par les bords sur la fenêtre principale.
pub fn hook_resize(win: &gtk::Window) {
    win.add_events(gtk::gdk::EventMask::BUTTON_PRESS_MASK | gtk::gdk::EventMask::POINTER_MOTION_MASK);
    win.connect_button_press_event(|w, e| {
        if app().chrome == 1 && e.button() == 1 && !w.is_maximized() {
            let (x, y) = e.position();
            if let Some(edge) = edge_at(x, y) {
                let (rx, ry) = e.root();
                w.begin_resize_drag(edge, 1, rx as i32, ry as i32, e.time());
                return gtk::glib::Propagation::Stop;
            }
        }
        gtk::glib::Propagation::Proceed
    });
    win.connect_motion_notify_event(|w, e| {
        if app().chrome == 1 {
            let (x, y) = e.position();
            let name = edge_at(x, y).map(|e| {
                use gtk::gdk::WindowEdge as E;
                match e {
                    E::NorthWest => "nw-resize",
                    E::NorthEast => "ne-resize",
                    E::SouthWest => "sw-resize",
                    E::SouthEast => "se-resize",
                    E::West => "w-resize",
                    E::East => "e-resize",
                    E::North => "n-resize",
                    _ => "s-resize",
                }
            });
            if let (Some(win), Some(d)) = (w.window(), gtk::gdk::Display::default()) {
                win.set_cursor(name.and_then(|n| gtk::gdk::Cursor::from_name(&d, n)).as_ref());
            }
        }
        gtk::glib::Propagation::Proceed
    });
}
