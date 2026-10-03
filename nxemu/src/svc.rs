//! Un "kernel" HLE (alto nivel) mínimo: solo las llamadas al sistema de Horizon
//! necesarias para programas de prueba.

use crate::cpu::Cpu;
use crate::memory::Memory;

pub enum SvcResult {
    Continue,
    Exit(u64),
}

/// Maneja una SVC. Los números siguen a Horizon OS (Switch).
pub fn handle(num: u32, cpu: &mut Cpu, mem: &Memory, out: &mut String) -> Result<SvcResult, String> {
    match num {
        // ExitProcess
        0x07 => Ok(SvcResult::Exit(cpu.x[0])),
        // Break
        0x26 => Err(format!("svcBreak (reason={:#x})", cpu.x[0])),
        // OutputDebugString(ptr, size) -> result
        0x27 => {
            let mut buf = vec![0u8; cpu.x[1] as usize];
            mem.read(cpu.x[0], &mut buf).map_err(|e| format!("{e:?}"))?;
            out.push_str(&String::from_utf8_lossy(&buf));
            cpu.x[0] = 0;
            Ok(SvcResult::Continue)
        }
        n => Err(format!("SVC {n:#x} no implementada")),
    }
}

/// Ejecuta el programa hasta ExitProcess; devuelve (código de salida, salida de debug).
pub fn run_program(cpu: &mut Cpu, mem: &mut Memory, max_svcs: u32) -> Result<(u64, String), String> {
    let mut out = String::new();
    for _ in 0..max_svcs {
        let exit = cpu.run(mem).map_err(|e| format!("{e:?} en pc={:#x}", cpu.pc))?;
        let crate::cpu::Exit::Svc(n) = exit;
        if let SvcResult::Exit(code) = handle(n, cpu, mem, &mut out)? {
            return Ok((code, out));
        }
    }
    Err("demasiadas SVC".into())
}
