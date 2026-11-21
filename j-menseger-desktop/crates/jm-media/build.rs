//! Gera as ligações com o libvpx a partir dos cabeçalhos instalados.
//!
//! Usar ligações prontas de um pacote fixo não serve: a disposição das
//! estruturas e a versão de ABI do libvpx mudam entre versões, e o codificador
//! recusa a inicialização quando não batem. Gerando aqui, o código acompanha a
//! biblioteca que a máquina realmente tem.

fn main() {
    println!("cargo:rerun-if-changed=csrc/vpx_abi.c");
    println!("cargo:rerun-if-changed=csrc/vpx.h");

    let lib = pkg_config::Config::new()
        .probe("vpx")
        .expect("libvpx não encontrada. No Debian/Ubuntu: apt install libvpx-dev");

    let mut bindings = bindgen::Builder::default()
        .header("csrc/vpx.h")
        .allowlist_function("vpx_.*")
        .allowlist_type("vpx_.*")
        .allowlist_var("VPX_.*")
        .derive_default(true)
        .layout_tests(false)
        .generate_comments(false);
    for p in &lib.include_paths {
        bindings = bindings.clang_arg(format!("-I{}", p.display()));
    }

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .generate()
        .expect("não foi possível ler os cabeçalhos do libvpx")
        .write_to_file(out.join("vpx.rs"))
        .expect("não foi possível escrever as ligações do libvpx");

    // A versão de ABI é uma macro, e macros não viram símbolo: um arquivo C
    // mínimo a expõe como função.
    let mut build = cc::Build::new();
    build.file("csrc/vpx_abi.c");
    for p in &lib.include_paths {
        build.include(p);
    }
    build.compile("jm_vpx_abi");
}
