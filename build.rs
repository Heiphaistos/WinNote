use std::path::Path;

fn cxx_files(dir: &str, skip: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).unwrap()
        .filter_map(|e| e.ok()).map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |x| x == "cxx"))
        .filter(|p| !skip.contains(&p.file_name().unwrap().to_str().unwrap()))
        .map(|p| p.to_string_lossy().into_owned()).collect();
    v.sort();
    v
}

fn main() {
    let mut b = cc::Build::new();
    b.cpp(true).std("c++17").flag_if_supported("/EHsc").define("STATIC_BUILD", "1").define("NDEBUG", "1")
        .define("UNICODE", "1").define("_UNICODE", "1").warnings(false);
    for i in ["vendor/scintilla/include", "vendor/scintilla/src", "vendor/scintilla/win32",
              "vendor/lexilla/include", "vendor/lexilla/lexlib", "vendor/lexilla/src"] { b.include(i); }
    for d in ["vendor/scintilla/src", "vendor/lexilla/lexlib", "vendor/lexilla/lexers"] { b.files(cxx_files(d, &[])); }
    b.files(cxx_files("vendor/scintilla/win32", &["ScintillaDLL.cxx"]));
    b.file("vendor/lexilla/src/Lexilla.cxx");
    b.compile("scintilla_lexilla");
    for l in ["user32", "gdi32", "imm32", "ole32", "oleaut32", "msimg32", "d2d1", "dwrite", "uuid", "advapi32", "shell32"] {
        println!("cargo:rustc-link-lib={l}");
    }
    // Manifeste : PerMonitorV2, contrôles v6, UTF-8
    let m = Path::new("winnote.manifest").canonicalize().unwrap();
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", m.display());
    println!("cargo:rerun-if-changed=winnote.manifest");
}
