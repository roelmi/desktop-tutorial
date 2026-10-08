//! Instrucciones SIMD (NEON) de AArch64: variantes escalares del banco SIMD y operaciones vectoriales.

use super::Cpu;

impl Cpu {
    /// Decodifica instrucciones del banco SIMD que no son de coma flotante escalar clásica.
    pub(super) fn simd_vector(&mut self, i: u32) -> Option<()> {
        let (rd, rn) = (i & 31, (i >> 5) & 31);

        // Escalar, dos registros, misceláneo: `01 U 11110 size 10000 opcode 10 Rn Rd`
        if i & 0xDF3E_0C00 == 0x5E20_0800 {
            let u = (i >> 29) & 1;
            let size = (i >> 22) & 3;
            let opcode = (i >> 12) & 0x1F;
            let dbl = size & 1 == 1; // sz: 0 = simple, 1 = doble (para conversiones)
            let src = self.vr[rn as usize] as u64;
            let r: u64 = match (u, opcode) {
                // SCVTF / UCVTF: entero en el registro vectorial -> flotante
                (0, 0x1D) | (1, 0x1D) => {
                    let signed = u == 0;
                    if dbl {
                        if signed { (src as i64 as f64).to_bits() } else { (src as f64).to_bits() }
                    } else if signed {
                        (src as u32 as i32 as f32).to_bits() as u64
                    } else {
                        (src as u32 as f32).to_bits() as u64
                    }
                }
                // FCVTZS / FCVTZU: flotante -> entero con truncamiento
                (0, 0x1B) | (1, 0x1B) => {
                    let v = if dbl { f64::from_bits(src) } else { f32::from_bits(src as u32) as f64 };
                    let t = v.trunc();
                    match (u == 1, dbl) {
                        (false, true) => t as i64 as u64,
                        (false, false) => t as i32 as u32 as u64,
                        (true, true) => t as u64,
                        (true, false) => t as u32 as u64,
                    }
                }
                _ => return None,
            };
            self.vr[rd as usize] = r as u128;
            return Some(());
        }

        None
    }
}
