//! Langages : extension -> lexer Lexilla, mots-clés, correspondance style -> classe de couleur.
use crate::theme::Cls::{self, *};

pub struct Lang {
    pub name: &'static str,
    pub lexer: &'static str, // "" = texte brut
    pub exts: &'static [&'static str],
    pub kw: &'static [&'static str],
    pub map: &'static [(u8, Cls)],
}

const CPP_MAP: &[(u8, Cls)] = &[
    (1, Comment), (2, Comment), (3, Comment), (15, Comment), (4, Num), (5, Kw), (6, Str), (7, Str), (13, Str),
    (20, Str), (27, Num), (9, Pre), (23, Pre), (10, Op), (16, Kw2), (17, Kw2), (19, Kw2),
];
const HTML_MAP: &[(u8, Cls)] = &[
    (1, Kw), (2, Kw), (11, Kw), (3, Attr), (4, Attr), (5, Num), (6, Str), (7, Str), (19, Str), (17, Str),
    (9, Comment), (10, Pre), (12, Pre), (13, Pre), (18, Pre), (42, Comment), (43, Comment), (44, Comment),
    (45, Num), (47, Kw), (48, Str), (49, Str), (53, Str), (50, Op), (52, Str),
];
const C_KW: &str = "alignas alignof asm auto break case catch class const constexpr continue default delete do else enum explicit extern false for friend goto if inline namespace new noexcept nullptr operator private protected public register return sizeof static static_assert struct switch template this throw true try typedef typename union using virtual volatile while";
const C_TY: &str = "bool char double float int long short signed unsigned void size_t int8_t int16_t int32_t int64_t uint8_t uint16_t uint32_t uint64_t string vector map";
const JS_KW: &str = "async await break case catch class const continue debugger default delete do else export extends false finally for function if import in instanceof let new null return super switch this throw true try typeof undefined var void while with yield of from as interface type enum implements public private protected readonly";

pub static LANGS: &[Lang] = &[
    Lang { name: "Texte brut", lexer: "", exts: &["txt", "log", "text", "cfg", "conf"], kw: &[], map: &[] },
    Lang { name: "C / C++", lexer: "cpp", exts: &["c", "h", "cpp", "hpp", "cc", "cxx", "hh", "ino", "cu"], kw: &[C_KW, C_TY], map: CPP_MAP },
    Lang { name: "C#", lexer: "cpp", exts: &["cs"], kw: &["abstract as base bool break byte case catch char checked class const continue decimal default delegate do double else enum event explicit extern false finally fixed float for foreach goto if implicit in int interface internal is lock long namespace new null object operator out override params private protected public readonly ref return sbyte sealed short sizeof static string struct switch this throw true try typeof uint ulong unchecked unsafe ushort using var virtual void volatile while async await record"], map: CPP_MAP },
    Lang { name: "Java / Kotlin", lexer: "cpp", exts: &["java", "kt", "kts", "scala", "groovy"], kw: &["abstract assert boolean break byte case catch char class const continue default do double else enum extends final finally float for goto if implements import instanceof int interface long native new package private protected public return short static super switch synchronized this throw throws transient try void volatile while true false null var record fun val"], map: CPP_MAP },
    Lang { name: "JavaScript / TypeScript", lexer: "cpp", exts: &["js", "mjs", "cjs", "jsx", "ts", "tsx"], kw: &[JS_KW], map: CPP_MAP },
    Lang { name: "Go", lexer: "cpp", exts: &["go"], kw: &["break case chan const continue default defer else fallthrough for func go goto if import interface map package range return select struct switch type var true false nil", "bool byte error float32 float64 int int8 int16 int32 int64 rune string uint uint8 uint16 uint32 uint64 uintptr"], map: CPP_MAP },
    Lang { name: "Rust", lexer: "rust", exts: &["rs"], kw: &["as async await break const continue crate dyn else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while", "bool char f32 f64 i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize str String Vec Option Result Box"],
        map: &[(1, Comment), (2, Comment), (3, Comment), (4, Comment), (5, Num), (6, Kw), (7, Kw2), (13, Str), (14, Str), (15, Str), (21, Str), (22, Str), (24, Str), (25, Str), (16, Op), (18, Kw2), (19, Pre)] },
    Lang { name: "Python", lexer: "python", exts: &["py", "pyw", "pyi"], kw: &["False None True and as assert async await break class continue def del elif else except finally for from global if import in is lambda nonlocal not or pass raise return try while with yield self"],
        map: &[(1, Comment), (12, Comment), (2, Num), (3, Str), (4, Str), (6, Str), (7, Str), (16, Str), (17, Str), (18, Str), (19, Str), (5, Kw), (8, Func), (9, Func), (10, Op), (14, Kw2), (15, Pre)] },
    Lang { name: "HTML / PHP", lexer: "hypertext", exts: &["html", "htm", "xhtml", "php", "vue", "svelte", "asp", "aspx"], kw: &["a abbr article aside audio b body br button canvas code div em footer form h1 h2 h3 h4 h5 h6 head header hr html i iframe img input label li link main meta nav ol p pre script section select small span strong style table tbody td textarea th thead title tr ul video", JS_KW], map: HTML_MAP },
    Lang { name: "XML / SVG", lexer: "xml", exts: &["xml", "xsl", "xslt", "xsd", "svg", "csproj", "vcxproj", "props", "targets", "plist", "xaml", "rss", "atom", "manifest"], kw: &[], map: HTML_MAP },
    Lang { name: "CSS / SCSS", lexer: "css", exts: &["css", "scss", "less"], kw: &["color background margin padding border font display position width height top left right bottom flex grid overflow opacity transform transition animation content cursor z-index"],
        map: &[(1, Kw), (2, Func), (3, Pre), (4, Pre), (5, Op), (6, Attr), (7, Attr), (15, Attr), (17, Attr), (8, Num), (9, Comment), (10, Func), (11, Kw), (12, Kw), (13, Str), (14, Str), (23, Pre)] },
    Lang { name: "JSON", lexer: "json", exts: &["json", "jsonc", "json5", "geojson", "webmanifest"], kw: &["true false null"],
        map: &[(1, Num), (2, Str), (4, Attr), (5, Num), (6, Comment), (7, Comment), (8, Op), (11, Kw), (12, Kw)] },
    Lang { name: "Markdown", lexer: "markdown", exts: &["md", "markdown", "mdx"], kw: &[],
        map: &[(2, Strong), (3, Strong), (4, Emph), (5, Emph), (6, Head), (7, Head), (8, Head), (9, Head), (10, Head), (11, Head), (13, Kw), (14, Kw), (15, Comment), (16, Comment), (17, Op), (18, Link), (19, Str), (20, Str), (21, Str)] },
    Lang { name: "SQL", lexer: "sql", exts: &["sql"], kw: &["select from where insert into values update set delete create alter drop table index view join left right inner outer on group by order having limit as and or not null is in like between exists union all distinct case when then else end primary key foreign references default begin commit rollback"],
        map: &[(1, Comment), (2, Comment), (3, Comment), (13, Comment), (15, Comment), (4, Num), (5, Kw), (6, Str), (7, Str), (23, Attr), (10, Op), (16, Kw2)] },
    Lang { name: "Shell (bash)", lexer: "bash", exts: &["sh", "bash", "zsh", "ksh", "profile", "bashrc", "dockerfile"], kw: &["if then else elif fi for while until do done case esac in function select time return exit break continue export local readonly declare echo cd ls grep sed awk cat rm cp mv mkdir"],
        map: &[(2, Comment), (3, Num), (4, Kw), (5, Str), (6, Str), (7, Op), (9, Attr), (10, Attr), (11, Pre), (12, Str), (13, Str)] },
    Lang { name: "Batch", lexer: "batch", exts: &["bat", "cmd"], kw: &["if else for in do goto call exit not exist errorlevel set setlocal endlocal echo pause rem shift start", "copy del dir md rd move ren type cls cd mkdir rmdir xcopy robocopy taskkill findstr"],
        map: &[(1, Comment), (4, Comment), (2, Kw), (3, Func), (5, Kw2), (7, Op)] },
    Lang { name: "PowerShell", lexer: "powershell", exts: &["ps1", "psm1", "psd1"], kw: &["if else elseif switch while for foreach do until break continue function filter param begin process end return throw try catch finally in class enum using", "Get-ChildItem Get-Content Set-Content Write-Host Write-Output Get-Item New-Item Remove-Item Invoke-WebRequest ForEach-Object Where-Object Select-Object"],
        map: &[(1, Comment), (13, Comment), (2, Str), (3, Str), (14, Str), (15, Str), (4, Num), (5, Attr), (6, Op), (8, Kw), (9, Kw2), (10, Kw2), (11, Func)] },
    Lang { name: "INI / Config", lexer: "props", exts: &["ini", "inf", "properties", "env", "editorconfig", "gitconfig", "desktop"], kw: &[],
        map: &[(1, Comment), (2, Func), (3, Op), (4, Str), (5, Attr)] },
    Lang { name: "YAML", lexer: "yaml", exts: &["yml", "yaml"], kw: &["true false null yes no on off"],
        map: &[(1, Comment), (2, Attr), (3, Kw), (4, Num), (5, Pre), (6, Pre), (7, Str), (9, Op)] },
    Lang { name: "TOML", lexer: "toml", exts: &["toml", "lock"], kw: &["true false"],
        map: &[(1, Comment), (3, Kw), (4, Num), (5, Func), (6, Attr), (8, Op), (9, Str), (10, Str), (11, Str), (12, Str), (13, Num), (14, Num)] },
    Lang { name: "Lua", lexer: "lua", exts: &["lua"], kw: &["and break do else elseif end false for function goto if in local nil not or repeat return then true until while", "print require pairs ipairs type tostring tonumber table string math os io"],
        map: &[(1, Comment), (2, Comment), (3, Comment), (4, Num), (5, Kw), (6, Str), (7, Str), (8, Str), (9, Pre), (10, Op), (13, Kw2), (14, Kw2), (20, Func)] },
    Lang { name: "Diff / Patch", lexer: "diff", exts: &["diff", "patch"], kw: &[], map: &[(1, Comment), (2, Kw), (3, Kw2), (4, Func), (5, Pre), (9, Pre), (6, Str), (8, Str), (7, Num)] },
    Lang { name: "Makefile", lexer: "makefile", exts: &["mk", "makefile", "mak"], kw: &[], map: &[(1, Comment), (2, Pre), (3, Attr), (4, Op), (5, Func), (9, Num)] },
    Lang { name: "CMake", lexer: "cmake", exts: &["cmake", "cmakelists.txt"], kw: &["add_executable add_library add_subdirectory cmake_minimum_required project set target_link_libraries include find_package if else endif foreach endforeach function endfunction message option install"],
        map: &[(1, Comment), (2, Str), (3, Str), (4, Str), (13, Str), (5, Kw), (6, Attr), (7, Pre), (8, Func), (14, Num)] },
    Lang { name: "Ruby", lexer: "ruby", exts: &["rb", "rake", "gemspec", "ru"], kw: &["alias and begin break case class def defined do else elsif end ensure false for if in module next nil not or redo rescue retry return self super then true undef unless until when while yield require include extend attr_accessor puts"],
        map: &[(2, Comment), (3, Comment), (4, Num), (5, Kw), (6, Str), (7, Str), (24, Str), (25, Str), (26, Str), (27, Str), (28, Str), (8, Func), (9, Func), (10, Op), (12, Str), (13, Attr), (16, Attr), (17, Attr), (14, Num), (18, Pre)] },
    Lang { name: "Perl", lexer: "perl", exts: &["pl", "pm", "t"], kw: &["if elsif else unless while until for foreach do last next redo return sub my our local use no package require print say die warn eval defined undef shift push pop split join map grep sort keys values"],
        map: &[(2, Comment), (3, Comment), (4, Num), (5, Kw), (6, Str), (7, Str), (26, Str), (27, Str), (28, Str), (10, Op), (12, Attr), (13, Attr), (14, Attr), (17, Str), (18, Str)] },
    Lang { name: "Visual Basic", lexer: "vb", exts: &["vb", "vbs", "bas", "vba"], kw: &["as byref byval case class const dim do each else elseif end enum exit false for function if imports in loop module namespace new next nothing not or private property public return select set sub then true try catch finally while with"],
        map: &[(1, Comment), (19, Comment), (2, Num), (17, Num), (3, Kw), (4, Str), (5, Pre), (6, Op), (10, Kw2)] },
    Lang { name: "Assembleur", lexer: "asm", exts: &["asm", "s", "nasm", "inc"], kw: &["mov push pop add sub mul div jmp je jne jz jnz call ret cmp test lea inc dec and or xor shl shr nop int", "", "eax ebx ecx edx esi edi esp ebp rax rbx rcx rdx rsi rdi rsp rbp ax bx cx dx", "section global extern db dw dd dq resb resw resd equ times"],
        map: &[(1, Comment), (11, Comment), (2, Num), (3, Str), (12, Str), (6, Kw), (7, Kw), (8, Kw2), (9, Pre)] },
    Lang { name: "LaTeX", lexer: "latex", exts: &["tex", "sty", "cls", "bib"], kw: &[], map: &[(1, Kw), (2, Func), (3, Num), (6, Num), (4, Comment), (7, Comment), (8, Str)] },
    Lang { name: "R", lexer: "r", exts: &["r", "rmd"], kw: &["if else repeat while function for in next break TRUE FALSE NULL Inf NaN NA library require return"],
        map: &[(1, Comment), (2, Kw), (3, Kw), (4, Kw2), (5, Num), (6, Str), (7, Str), (8, Op)] },
    Lang { name: "Lisp / Scheme", lexer: "lisp", exts: &["lisp", "el", "cl", "scm", "ss", "clj"], kw: &["defun defvar defmacro lambda let if cond when unless progn setq define car cdr cons list quote loop"],
        map: &[(1, Comment), (12, Comment), (2, Num), (3, Kw), (4, Kw2), (5, Attr), (6, Str)] },
    Lang { name: "Dart", lexer: "dart", exts: &["dart"], kw: &["abstract as assert async await break case catch class const continue default do else enum extends false final finally for if implements import in is new null on rethrow return super switch this throw true try var void while with yield", "bool double dynamic int num String List Map Set Future Stream"],
        map: &[(1, Comment), (2, Comment), (3, Comment), (4, Comment), (5, Str), (6, Str), (7, Str), (8, Str), (9, Str), (10, Str), (13, Num), (20, Num), (22, Pre), (23, Kw), (24, Kw), (25, Kw), (26, Kw2)] },
];

pub fn by_path(path: &str) -> usize {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path).to_ascii_lowercase();
    let ext = name.rsplit('.').next().unwrap_or("");
    LANGS.iter().position(|l| l.exts.contains(&ext) || l.exts.contains(&name.trim_start_matches('.'))).unwrap_or(0)
}
