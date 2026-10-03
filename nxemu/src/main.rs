use nxemu::{cpu::Cpu, memory::Memory, nro, svc};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("uso: nxemu <archivo.nro>");
        std::process::exit(2);
    };
    let data = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("no se pudo leer {path}: {e}");
        std::process::exit(1);
    });
    let mut mem = Memory::new();
    let loaded = nro::load(&data, &mut mem).unwrap_or_else(|e| {
        eprintln!("NRO inválido: {e:?}");
        std::process::exit(1);
    });
    let mut cpu = Cpu::new();
    cpu.pc = loaded.entry;
    cpu.sp = loaded.stack_top;
    match svc::run_program(&mut cpu, &mut mem, 1_000_000) {
        Ok((code, out)) => {
            print!("{out}");
            std::process::exit(code as i32);
        }
        Err(e) => {
            eprintln!("error de emulación: {e}");
            std::process::exit(1);
        }
    }
}
