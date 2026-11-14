//! Outil de signature des mises a jour (hors binaire de release) :
//!   cargo run --example sign -- pub  <cle.hex>                 -> cle publique (hex)
//! (format du message = src/vers.rs::signed_msg)
//!   cargo run --example sign -- sign <cle.hex> <tag> <fichier> <sha256hex> -> ecrit <fichier>.sig (64 octets)
//! La cle privee (graine de 32 octets en hex) vit hors depot : D:\MDP\winnote-updater\update-signing.hex
use ed25519_compact::{KeyPair, Seed};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed = |p: &str| -> KeyPair {
        let h = std::fs::read_to_string(p).expect("cle illisible");
        let b: Vec<u8> = (0..32).map(|i| u8::from_str_radix(&h.trim()[i * 2..i * 2 + 2], 16).expect("hex")).collect();
        KeyPair::from_seed(Seed::from_slice(&b).expect("graine"))
    };
    match a.get(1).map(String::as_str) {
        Some("pub") => println!("{}", seed(&a[2]).pk.iter().map(|b| format!("{b:02x}")).collect::<String>()),
        Some("sign") => {
            let name = std::path::Path::new(&a[4]).file_name().unwrap().to_string_lossy().to_string();
            let msg = format!("winnote-update-v1\n{}\n{name}\n{}", a[3], a[5].to_lowercase()).into_bytes();
            let sig = seed(&a[2]).sk.sign(msg, None);
            std::fs::write(format!("{}.sig", a[4]), sig.as_ref()).expect("ecriture .sig");
        }
        _ => eprintln!("usage : sign pub <cle> | sign sign <cle> <tag> <fichier> <sha256>"),
    }
}
