//! Pruebas diferenciales: programas en C compilados por LLVM para AArch64 se ejecutan en nxemu y su
//! resultado debe coincidir con el de la misma fuente ejecutada de forma nativa.
//! Regenerar fixtures: `tests/c/build.sh` (requiere clang + lld).

use nxemu::cpu::{Cpu, CpuError, Exit};
use nxemu::memory::Memory;
use std::fs;
use std::path::PathBuf;

const BASE: u64 = 0x0800_0000;
const STACK_TOP: u64 = 0x1000_0000;
const MAX_STEPS: u64 = 400_000_000;

fn run_bin(code: &[u8]) -> Result<u64, String> {
    let mut mem = Memory::new();
    mem.map(BASE, code.len() as u64 + 0x10_0000); // código + datos + bss
    mem.map(STACK_TOP - 0x10_0000, 0x10_0000);
    mem.write(BASE, code).unwrap();
    let mut cpu = Cpu::new();
    cpu.pc = BASE;
    for _ in 0..MAX_STEPS {
        match cpu.step(&mut mem) {
            Ok(None) => {}
            Ok(Some(Exit::Svc(7))) => return Ok(cpu.x[0]),
            Ok(Some(Exit::Svc(n))) => return Err(format!("SVC inesperada {n:#x}")),
            Err(CpuError::Unimplemented { pc, insn }) => {
                return Err(format!("instrucción no implementada {insn:#010x} en {:#x} (offset {:#x})", pc, pc - BASE))
            }
            Err(e) => return Err(format!("{e:?} en pc={:#x}", cpu.pc)),
        }
    }
    Err("demasiados pasos (¿bucle infinito?)".into())
}

#[test]
fn programas_c_coinciden_con_ejecucion_nativa() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/c/out");
    let mut bins: Vec<_> = fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).filter(|p| p.extension().is_some_and(|e| e == "bin")).collect();
    bins.sort();
    assert!(!bins.is_empty(), "faltan fixtures: ejecuta tests/c/build.sh");
    let mut fallos = Vec::new();
    for bin in &bins {
        let nombre = bin.file_stem().unwrap().to_string_lossy().to_string(); // p. ej. "sort.O2"
        let prog = nombre.split('.').next().unwrap();
        let esperado: u64 = fs::read_to_string(dir.join(format!("{prog}.expected"))).unwrap().trim().parse().unwrap();
        match run_bin(&fs::read(bin).unwrap()) {
            Ok(v) if v == esperado => {}
            Ok(v) => fallos.push(format!("{nombre}: resultado {v} != esperado {esperado}")),
            Err(e) => fallos.push(format!("{nombre}: {e}")),
        }
    }
    assert!(fallos.is_empty(), "{} de {} fallan:\n{}", fallos.len(), bins.len(), fallos.join("\n"));
}
