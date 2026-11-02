//! Impression (GTK PrintOperation + Pango) : texte noir sur blanc, retour à la ligne, en-tête (nom + date), pied « Page n / N ».
use crate::{app, edit};
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

const SCALE: f64 = gtk::pango::SCALE as f64;

struct Job {
    text: String,
    title: String,
    date: String,
    /// début (y) de chaque page dans la mise en page complète, et hauteur du corps
    pages: Vec<f64>,
    body_h: f64,
    line_h: f64,
}

fn layout_for(ctx: &gtk::PrintContext, text: &str, width: f64) -> Option<gtk::pango::Layout> {
    let l = ctx.create_pango_layout()?;
    l.set_font_description(Some(&gtk::pango::FontDescription::from_string("Monospace 10")));
    l.set_width((width * SCALE) as i32);
    l.set_wrap(gtk::pango::WrapMode::WordChar);
    l.set_text(text);
    Some(l)
}

/// Répartit les lignes de la mise en page en pages sans jamais couper une ligne.
fn paginate(l: &gtk::pango::Layout, body_h: f64) -> Vec<f64> {
    let mut pages = vec![0.0];
    let mut it = l.iter();
    loop {
        let (_, ext) = it.line_extents();
        let (top, bottom) = (ext.y() as f64 / SCALE, (ext.y() + ext.height()) as f64 / SCALE);
        let start = *pages.last().unwrap();
        if bottom - start > body_h && top > start {
            pages.push(top);
        }
        if !it.next_line() {
            break;
        }
    }
    pages
}

fn run(action: gtk::PrintOperationAction, export: Option<&str>) -> Result<usize, String> {
    let a = app();
    let d = &a.docs[a.cur];
    let text = String::from_utf8_lossy(&edit::text_of(a.cur)).replace('\t', "    ").replace('\r', "");
    let now = gtk::glib::DateTime::now_local().ok().and_then(|t| t.format("%d/%m/%Y %H:%M").ok()).map(|s| s.to_string()).unwrap_or_default();
    let job = Rc::new(RefCell::new(Job { text, title: edit::title_of(d), date: now, pages: vec![0.0], body_h: 0.0, line_h: 0.0 }));
    let op = gtk::PrintOperation::new();
    op.set_job_name(&format!("WinNote - {}", job.borrow().title));
    op.set_unit(gtk::Unit::Points);
    op.set_use_full_page(false);
    if let Some(f) = export {
        op.set_export_filename(f);
    }
    {
        let job = job.clone();
        op.connect_begin_print(move |op, ctx| {
            let mut j = job.borrow_mut();
            let (Some(probe), Some(l)) = (layout_for(ctx, "Mg", ctx.width()), layout_for(ctx, &j.text, ctx.width())) else { return };
            j.line_h = probe.pixel_size().1 as f64;
            j.body_h = (ctx.height() - 3.0 * j.line_h).max(j.line_h);
            j.pages = paginate(&l, j.body_h);
            op.set_n_pages(j.pages.len() as i32);
        });
    }
    {
        let job = job.clone();
        op.connect_draw_page(move |_, ctx, n| {
            let j = job.borrow();
            let Some(cr) = ctx.cairo_context() else { return };
            let (w, h) = (ctx.width(), ctx.height());
            cr.set_source_rgb(0.0, 0.0, 0.0);
            let n = n as usize;
            // en-tête
            let (Some(head), Some(body), Some(foot)) = (layout_for(ctx, &format!("{}    {}", j.title, j.date), w), layout_for(ctx, &j.text, w), layout_for(ctx, &format!("Page {} / {}", n + 1, j.pages.len()), w)) else { return };
            head.set_wrap(gtk::pango::WrapMode::Char);
            cr.move_to(0.0, 0.0);
            pangocairo::functions::show_layout(&cr, &head);
            cr.set_line_width(0.5);
            cr.move_to(0.0, j.line_h * 1.4);
            cr.line_to(w, j.line_h * 1.4);
            let _ = cr.stroke();
            // corps : on translate la mise en page complète et on découpe sur la page
            let start = j.pages[n];
            let end = j.pages.get(n + 1).copied().unwrap_or(f64::MAX);
            let _ = cr.save();
            cr.rectangle(0.0, j.line_h * 1.8, w, (end - start).min(j.body_h) + 1.0);
            cr.clip();
            cr.move_to(0.0, j.line_h * 1.8 - start);
            pangocairo::functions::show_layout(&cr, &body);
            let _ = cr.restore();
            // pied
            foot.set_alignment(gtk::pango::Alignment::Right);
            cr.move_to(0.0, h - j.line_h);
            pangocairo::functions::show_layout(&cr, &foot);
        });
    }
    match op.run(action, Some(&a.win)) {
        Ok(_) => Ok(job.borrow().pages.len()),
        Err(e) => Err(e.to_string()),
    }
}

/// Boîte de dialogue d'impression.
pub fn print_doc() {
    if let Err(e) = run(gtk::PrintOperationAction::PrintDialog, None) {
        edit::msg_box(&format!("Impression impossible : {e}"));
    }
}

/// Imprime dans un fichier PDF sans dialogue ; renvoie le nombre de pages (tests, `--print-to`).
pub fn export_pdf(path: &str) -> Result<usize, String> {
    run(gtk::PrintOperationAction::Export, Some(path))
}
