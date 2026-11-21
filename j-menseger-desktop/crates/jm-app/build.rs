//! Metadados do executável no Windows: ícone, nome e versão.
//!
//! Sem isto o programa aparece com o ícone genérico do Windows e sem nenhuma
//! informação em "Propriedades". Os dados vão embutidos no próprio executável.

fn main() {
    println!("cargo:rerun-if-changed=../../packaging/windows/j-menseger.ico");

    // Atenção: `cfg!(windows)` aqui diria respeito à máquina que compila, não
    // ao sistema de destino. Quem responde isso é a variável do cargo.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../packaging/windows/j-menseger.ico");
    res.set("ProductName", "J-Menseger");
    res.set("FileDescription", "J-Menseger");
    res.set("CompanyName", "J-Menseger");
    res.set("LegalCopyright", "Licença MIT");
    res.set("OriginalFilename", "j-menseger.exe");

    // Compilando do Linux para o Windows, a ferramenta de recursos tem o nome
    // completo do alvo; no próprio Windows ela se chama só `windres`.
    if !cfg!(windows) {
        res.set_windres_path("x86_64-w64-mingw32-windres");
    }

    if let Err(e) = res.compile() {
        // Falhar calado deixaria um executável sem ícone parecendo pronto.
        panic!(
            "não foi possível embutir o ícone e os dados do programa: {e}\n\
             No Linux: apt install binutils-mingw-w64-x86-64\n\
             No Windows: use o Visual Studio Build Tools ou o mingw-w64."
        );
    }
}
