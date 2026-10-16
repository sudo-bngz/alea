// Embeds the browser engine (web/dist/alea.wasm, built by web/build.sh) into the
// command-line tool when it exists, so `alea --web` can export pages.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(alea_wasm)");
    println!("cargo::rerun-if-changed=web/dist/alea.wasm");
    let native = !std::env::var("TARGET").unwrap_or_default().starts_with("wasm32");
    if native && std::path::Path::new("web/dist/alea.wasm").exists() {
        println!("cargo::rustc-cfg=alea_wasm");
    }
}
