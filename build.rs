#[cfg(windows)]
use std::path::Path;
use std::process::Command;

fn files(dir: &str, ext: &str, skip: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap()
        .filter_map(|e| e.ok()).map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |x| x == ext))
        .filter(|p| !skip.contains(&p.file_name().unwrap().to_str().unwrap()))
        .map(|p| p.to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

fn pkg(args: &[&str]) -> Vec<String> {
    let out = Command::new("pkg-config").args(args).arg("gtk+-3.0").output().expect("pkg-config introuvable (installer libgtk-3-dev)");
    assert!(out.status.success(), "pkg-config gtk+-3.0 a echoue");
    String::from_utf8_lossy(&out.stdout).split_whitespace().map(String::from).collect()
}

fn main() {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    if os == "windows" {
        windows();
    } else if std::env::var("CARGO_FEATURE_GTK_UI").is_ok() {
        gtk();
    }
}

fn gtk() {
    let mut cxx = cc::Build::new();
    cxx.cpp(true).std("c++17").define("GTK", "1").define("NDEBUG", "1").warnings(false);
    let mut c = cc::Build::new();
    c.define("GTK", "1").warnings(false);
    for i in ["vendor/scintilla/include", "vendor/scintilla/src", "vendor/scintilla/gtk", "vendor/lexilla/include", "vendor/lexilla/lexlib", "vendor/lexilla/src"] {
        cxx.include(i);
        c.include(i);
    }
    for f in pkg(&["--cflags"]) {
        if let Some(i) = f.strip_prefix("-I") {
            cxx.include(i);
            c.include(i);
        }
    }
    for d in ["vendor/scintilla/src", "vendor/lexilla/lexlib", "vendor/lexilla/lexers"] {
        cxx.files(files(d, "cxx", &[]));
    }
    cxx.files(["vendor/scintilla/gtk/PlatGTK.cxx", "vendor/scintilla/gtk/ScintillaGTK.cxx", "vendor/scintilla/gtk/ScintillaGTKAccessible.cxx", "vendor/lexilla/src/Lexilla.cxx"]);
    c.file("vendor/scintilla/gtk/scintilla-marshal.c");
    cxx.compile("scintilla_lexilla_gtk");
    c.compile("scintilla_marshal");
    for f in pkg(&["--libs"]) {
        if let Some(l) = f.strip_prefix("-l") {
            println!("cargo:rustc-link-lib={l}");
        } else if let Some(p) = f.strip_prefix("-L") {
            println!("cargo:rustc-link-search=native={p}");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}

fn windows() {
    let mut b = cc::Build::new();
    b.cpp(true).std("c++17").flag_if_supported("/EHsc").define("STATIC_BUILD", "1").define("NDEBUG", "1")
        .define("UNICODE", "1").define("_UNICODE", "1").warnings(false);
    for i in ["vendor/scintilla/include", "vendor/scintilla/src", "vendor/scintilla/win32",
              "vendor/lexilla/include", "vendor/lexilla/lexlib", "vendor/lexilla/src"] { b.include(i); }
    for d in ["vendor/scintilla/src", "vendor/lexilla/lexlib", "vendor/lexilla/lexers"] { b.files(files(d, "cxx", &[])); }
    b.files(files("vendor/scintilla/win32", "cxx", &["ScintillaDLL.cxx"]));
    b.file("vendor/lexilla/src/Lexilla.cxx");
    b.compile("scintilla_lexilla");
    for l in ["user32", "gdi32", "imm32", "ole32", "oleaut32", "msimg32", "d2d1", "dwrite", "uuid", "advapi32", "shell32"] {
        println!("cargo:rustc-link-lib={l}");
    }
    #[cfg(windows)]
    {
        embed_resource::compile("winnote.rc", embed_resource::NONE).manifest_optional().unwrap();
        println!("cargo:rerun-if-changed=winnote.rc");
        let m = Path::new("winnote.manifest").canonicalize().unwrap();
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", m.display());
        println!("cargo:rerun-if-changed=winnote.manifest");
    }
}
