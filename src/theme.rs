//! Thèmes préfaits et dispositions préfaites.
#[derive(Clone, Copy, PartialEq)]
pub enum Cls { Comment, Str, Num, Kw, Kw2, Pre, Op, Attr, Func, Link, Head, Strong, Emph }

pub struct Theme {
    pub name: &'static str,
    pub dark: bool,
    pub bg: u32,
    pub fg: u32,
    pub sel: u32,
    pub line: u32,
    pub gutter: u32,
    pub ui_bg: u32,
    pub ui_fg: u32,
    pub accent: u32,
    /// commentaire, chaîne, nombre, mot-clé, type, préprocesseur, opérateur, attribut, fonction
    pub c: [u32; 9],
}

/// 0xRRGGBB -> COLORREF (BGR)
pub const fn rgb(x: u32) -> u32 {
    ((x & 0xFF) << 16) | (x & 0xFF00) | ((x >> 16) & 0xFF)
}

impl Theme {
    pub fn color(&self, c: Cls) -> u32 {
        match c {
            Cls::Comment => self.c[0],
            Cls::Str => self.c[1],
            Cls::Num => self.c[2],
            Cls::Kw | Cls::Head => self.c[3],
            Cls::Kw2 => self.c[4],
            Cls::Pre => self.c[5],
            Cls::Op => self.c[6],
            Cls::Attr => self.c[7],
            Cls::Func => self.c[8],
            Cls::Link => self.accent,
            Cls::Strong | Cls::Emph => self.fg,
        }
    }
}

#[allow(clippy::too_many_arguments)]
const fn t(name: &'static str, dark: bool, bg: u32, fg: u32, sel: u32, line: u32, gutter: u32, ui_bg: u32, ui_fg: u32, accent: u32, c: [u32; 9]) -> Theme {
    Theme {
        name, dark, bg: rgb(bg), fg: rgb(fg), sel: rgb(sel), line: rgb(line), gutter: rgb(gutter),
        ui_bg: rgb(ui_bg), ui_fg: rgb(ui_fg), accent: rgb(accent),
        c: [rgb(c[0]), rgb(c[1]), rgb(c[2]), rgb(c[3]), rgb(c[4]), rgb(c[5]), rgb(c[6]), rgb(c[7]), rgb(c[8])],
    }
}

pub static THEMES: [Theme; 17] = [
    t("Clair (Windows)", false, 0xFFFFFF, 0x1B1B1B, 0xADD6FF, 0xF5F5F5, 0x8A8A8A, 0xF3F3F3, 0x1B1B1B, 0x0067C0, [0x008000, 0xA31515, 0x098658, 0x0000FF, 0x267F99, 0xAF00DB, 0x1B1B1B, 0x001080, 0x795E26]),
    t("Sombre (Windows)", true, 0x1E1E1E, 0xD4D4D4, 0x264F78, 0x2A2A2A, 0x858585, 0x2B2B2B, 0xFFFFFF, 0x60CDFF, [0x6A9955, 0xCE9178, 0xB5CEA8, 0x569CD6, 0x4EC9B0, 0xC586C0, 0xD4D4D4, 0x9CDCFE, 0xDCDCAA]),
    t("Monokai", true, 0x272822, 0xF8F8F2, 0x49483E, 0x3E3D32, 0x90908A, 0x1E1F1C, 0xF8F8F2, 0xA6E22E, [0x75715E, 0xE6DB74, 0xAE81FF, 0xF92672, 0x66D9EF, 0xF92672, 0xF8F8F2, 0xA6E22E, 0xA6E22E]),
    t("Dracula", true, 0x282A36, 0xF8F8F2, 0x44475A, 0x2F3241, 0x6272A4, 0x21222C, 0xF8F8F2, 0xBD93F9, [0x6272A4, 0xF1FA8C, 0xBD93F9, 0xFF79C6, 0x8BE9FD, 0xFF79C6, 0xFF79C6, 0x50FA7B, 0x50FA7B]),
    t("Nord", true, 0x2E3440, 0xD8DEE9, 0x434C5E, 0x3B4252, 0x616E88, 0x272C36, 0xD8DEE9, 0x88C0D0, [0x616E88, 0xA3BE8C, 0xB48EAD, 0x81A1C1, 0x8FBCBB, 0x5E81AC, 0x81A1C1, 0x8FBCBB, 0x88C0D0]),
    t("Solarized clair", false, 0xFDF6E3, 0x657B83, 0xEEE8D5, 0xEEE8D5, 0x93A1A1, 0xEEE8D5, 0x586E75, 0x268BD2, [0x93A1A1, 0x2AA198, 0xD33682, 0x859900, 0xB58900, 0xCB4B16, 0x657B83, 0x268BD2, 0x268BD2]),
    t("Solarized sombre", true, 0x002B36, 0x839496, 0x073642, 0x073642, 0x586E75, 0x00212B, 0x93A1A1, 0x268BD2, [0x586E75, 0x2AA198, 0xD33682, 0x859900, 0xB58900, 0xCB4B16, 0x839496, 0x268BD2, 0x268BD2]),
    t("Papier (sépia)", false, 0xF4ECD8, 0x433422, 0xE0D2B0, 0xEBE1C8, 0x9C8C6C, 0xE8DEC4, 0x433422, 0x8B5E34, [0x8A7A5C, 0x9C4A1A, 0x7A5C00, 0x7A3E9D, 0x2F6F6F, 0xB5552B, 0x433422, 0x2F5F8F, 0x8B5E34]),
    t("Gruvbox sombre", true, 0x282828, 0xEBDBB2, 0x504945, 0x32302F, 0x7C6F64, 0x1D2021, 0xEBDBB2, 0xFABD2F, [0x928374, 0xB8BB26, 0xD3869B, 0xFB4934, 0xFABD2F, 0x8EC07C, 0xEBDBB2, 0x83A598, 0xB8BB26]),
    t("Nuit (contraste élevé)", true, 0x000000, 0xFFFFFF, 0x264F78, 0x1A1A1A, 0x9A9A9A, 0x0A0A0A, 0xFFFFFF, 0xFFD700, [0x7CA668, 0xCE9178, 0xB5CEA8, 0x569CD6, 0x4EC9B0, 0xC586C0, 0xFFFFFF, 0x9CDCFE, 0xDCDCAA]),
    t("Apple Notes (clair)", false, 0xFFFEF8, 0x1D1D1F, 0xFBE7A1, 0xFFF7DB, 0xB0AFA8, 0xF2EFE6, 0x1D1D1F, 0xE5A100, [0x5D6C79, 0xD12F1B, 0x272AD8, 0xAD3DA4, 0x3E8087, 0x78492A, 0x1D1D1F, 0x4B21B0, 0x0F68A0]),
    t("Apple Notes (sombre)", true, 0x1C1C1E, 0xF2F2F7, 0x5C4B14, 0x2C2C2E, 0x636366, 0x2C2C2E, 0xF2F2F7, 0xFFD60A, [0x7F8C98, 0xFC6A5D, 0xD0BF69, 0xFC5FA3, 0x5DD8FF, 0xFD8F3F, 0xF2F2F7, 0x78C2B3, 0x67B7A4]),
    t("GitHub clair", false, 0xFFFFFF, 0x24292F, 0xB6D6FD, 0xF6F8FA, 0x8C959F, 0xF6F8FA, 0x24292F, 0x0969DA, [0x6E7781, 0x0A3069, 0x0550AE, 0xCF222E, 0x953800, 0x8250DF, 0x24292F, 0x116329, 0x8250DF]),
    t("Tokyo Night", true, 0x1A1B26, 0xC0CAF5, 0x33467C, 0x1F2335, 0x3B4261, 0x16161E, 0xC0CAF5, 0x7AA2F7, [0x565F89, 0x9ECE6A, 0xFF9E64, 0xBB9AF7, 0x2AC3DE, 0x7DCFFF, 0x89DDFF, 0x73DACA, 0x7AA2F7]),
    t("One Dark", true, 0x282C34, 0xABB2BF, 0x3E4451, 0x2C313A, 0x495162, 0x21252B, 0xABB2BF, 0x61AFEF, [0x5C6370, 0x98C379, 0xD19A66, 0xC678DD, 0xE5C07B, 0x56B6C2, 0x56B6C2, 0xD19A66, 0x61AFEF]),
    t("Catppuccin Mocha", true, 0x1E1E2E, 0xCDD6F4, 0x45475A, 0x313244, 0x6C7086, 0x181825, 0xCDD6F4, 0xCBA6F7, [0x6C7086, 0xA6E3A1, 0xFAB387, 0xCBA6F7, 0xF9E2AF, 0xF5C2E7, 0x89DCEB, 0x89B4FA, 0x89B4FA]),
    t("Obsidian", true, 0x1E1E1E, 0xDADADA, 0x3B3552, 0x262626, 0x6C6C6C, 0x262626, 0xDADADA, 0x8B6CEF, [0x6A737D, 0x7EE787, 0xF2CC60, 0xA78BFA, 0x79C0FF, 0xFF7B72, 0xDADADA, 0x79C0FF, 0xD2A8FF]),
];

pub struct Layout {
    pub name: &'static str,
    pub font: &'static str,
    pub size: i32,
    pub wrap: bool,
    pub lines: bool,
    pub margin: i32,
    pub chrome: u8,
    pub theme: Option<usize>,
    pub sidebar: bool,
    pub preview: bool,
    pub fold: bool,
    pub spacing: i32,
}

const fn l(name: &'static str, font: &'static str, size: i32, wrap: bool, lines: bool, margin: i32) -> Layout {
    Layout { name, font, size, wrap, lines, margin, chrome: 0, theme: None, sidebar: false, preview: false, fold: false, spacing: 0 }
}

pub static LAYOUTS: [Layout; 8] = [
    l("Classique (Bloc-notes)", "Consolas", 11, true, false, 6),
    Layout { fold: true, ..l("Code (Notepad++)", "Consolas", 11, false, true, 4) },
    Layout { spacing: 3, ..l("Écriture", "Georgia", 14, true, false, 56) },
    Layout { chrome: 1, theme: Some(10), spacing: 4, ..l("Notes (style Apple, clair)", "Segoe UI", 15, true, false, 48) },
    Layout { chrome: 1, theme: Some(11), spacing: 4, ..l("Notes (style Apple, sombre)", "Segoe UI", 15, true, false, 48) },
    Layout { theme: Some(16), sidebar: true, spacing: 3, ..l("Obsidian (explorateur + plan)", "Segoe UI", 14, true, false, 40) },
    Layout { preview: true, spacing: 2, ..l("Markdown (édition + aperçu)", "Segoe UI", 13, true, false, 20) },
    Layout { spacing: 5, ..l("Sans distraction", "Georgia", 16, true, false, 150) },
];
