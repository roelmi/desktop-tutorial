use nxemu::kernel::Kernel;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("uso: nxemu <archivo.nro>");
        std::process::exit(2);
    };
    let data = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("no se pudo leer {path}: {e}");
        std::process::exit(1);
    });
    let mut k = Kernel::new();
    if let Err(e) = k.boot_nro(&data) {
        eprintln!("{e}");
        std::process::exit(1);
    }
    let result = k.run(2_000_000_000);
    print!("{}", k.debug_output);
    for l in &k.log {
        eprintln!("[aviso] {l}");
    }
    match result {
        Ok(code) => std::process::exit(code as i32),
        Err(e) => {
            eprintln!("error de emulación: {e}");
            std::process::exit(1);
        }
    }
}
