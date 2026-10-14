//! Ejecuta un binario crudo AArch64 (cargado en 0x0800_0000) y muestra x0 al terminar con `svc #7`.
use nxemu::{cpu::{Cpu, Exit}, memory::Memory};
fn main() {
    let path = std::env::args().nth(1).expect("uso: runbin <archivo.bin>");
    let code = std::fs::read(path).unwrap();
    let mut mem = Memory::new();
    mem.map(0x0800_0000, code.len() as u64 + 0x10_0000);
    mem.map(0x0FF0_0000, 0x10_0000);
    mem.write(0x0800_0000, &code).unwrap();
    let mut cpu = Cpu::new();
    cpu.pc = 0x0800_0000;
    loop {
        match cpu.step(&mut mem) {
            Ok(None) => {}
            Ok(Some(Exit::Svc(7))) => { println!("{}", cpu.x[0]); return; }
            Ok(Some(Exit::Svc(n))) => { eprintln!("svc {n:#x}"); std::process::exit(2) }
            Err(e) => { eprintln!("{e:?} pc={:#x}", cpu.pc); std::process::exit(1) }
        }
    }
}
