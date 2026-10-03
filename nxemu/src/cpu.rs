//! Intérprete de un subconjunto de AArch64 (el ISA de la CPU de la Switch).

use crate::memory::{MemError, Memory};

#[derive(Debug, PartialEq, Eq)]
pub enum Exit {
    /// Llamada al sistema (SVC) con su número; el pc ya apunta a la siguiente instrucción.
    Svc(u32),
}

#[derive(Debug, PartialEq, Eq)]
pub enum CpuError {
    Mem(MemError),
    Unimplemented { pc: u64, insn: u32 },
}

impl From<MemError> for CpuError {
    fn from(e: MemError) -> Self {
        CpuError::Mem(e)
    }
}

#[derive(Default)]
pub struct Cpu {
    pub x: [u64; 31],
    pub sp: u64,
    pub pc: u64,
    pub n: bool,
    pub z: bool,
    pub c: bool,
    pub v: bool,
}

fn mask(sf: bool, v: u64) -> u64 {
    if sf { v } else { v & 0xFFFF_FFFF }
}

fn sext(v: u64, bits: u32) -> i64 {
    ((v << (64 - bits)) as i64) >> (64 - bits)
}

impl Cpu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lectura de registro con 31 = XZR.
    fn rz(&self, r: u32) -> u64 {
        if r == 31 { 0 } else { self.x[r as usize] }
    }
    fn wz(&mut self, r: u32, v: u64) {
        if r != 31 {
            self.x[r as usize] = v;
        }
    }
    /// Lectura de registro con 31 = SP.
    fn rs(&self, r: u32) -> u64 {
        if r == 31 { self.sp } else { self.x[r as usize] }
    }
    fn ws(&mut self, r: u32, v: u64) {
        if r == 31 { self.sp = v } else { self.x[r as usize] = v }
    }

    /// a + b + carry con flags NZCV opcionales.
    fn add_flags(&mut self, sf: bool, a: u64, b: u64, carry: bool, set: bool) -> u64 {
        let bits = if sf { 64 } else { 32 };
        let (a, b) = (mask(sf, a), mask(sf, b));
        let full = a as u128 + b as u128 + carry as u128;
        let r = mask(sf, full as u64);
        if set {
            self.n = (r >> (bits - 1)) & 1 == 1;
            self.z = r == 0;
            self.c = (full >> bits) & 1 == 1;
            let (sa, sb, sr) = ((a >> (bits - 1)) & 1, (b >> (bits - 1)) & 1, (r >> (bits - 1)) & 1);
            self.v = sa == sb && sa != sr;
        }
        r
    }

    fn cond(&self, c: u32) -> bool {
        let r = match c >> 1 {
            0 => self.z,
            1 => self.c,
            2 => self.n,
            3 => self.v,
            4 => self.c && !self.z,
            5 => self.n == self.v,
            6 => self.n == self.v && !self.z,
            _ => true,
        };
        if c & 1 == 1 && c != 0xF { !r } else { r }
    }

    fn shift(sf: bool, v: u64, kind: u32, amt: u32) -> u64 {
        let bits = if sf { 64 } else { 32 };
        let v = mask(sf, v);
        let r = match kind {
            0 => v << amt,
            1 => v >> amt,
            2 => ((if sf { v as i64 } else { v as u32 as i32 as i64 }) >> amt) as u64,
            _ => (v >> amt) | (v << (bits - amt)),
        };
        mask(sf, r)
    }

    /// Ejecuta instrucciones hasta que ocurra un SVC.
    pub fn run(&mut self, mem: &mut Memory) -> Result<Exit, CpuError> {
        loop {
            if let Some(e) = self.step(mem)? {
                return Ok(e);
            }
        }
    }

    pub fn step(&mut self, mem: &mut Memory) -> Result<Option<Exit>, CpuError> {
        let pc = self.pc;
        let i = mem.read_uint(pc, 4)? as u32;
        let rd = i & 31;
        let rn = (i >> 5) & 31;
        let rm = (i >> 16) & 31;
        let sf = i >> 31 == 1;
        let mut next = pc.wrapping_add(4);

        if i == 0xD503201F {
            // NOP
        } else if i & 0xFFE0_001F == 0xD400_0001 {
            self.pc = next;
            return Ok(Some(Exit::Svc((i >> 5) & 0xFFFF)));
        } else if i & 0x7C00_0000 == 0x1400_0000 {
            // B / BL
            if i >> 31 == 1 {
                self.x[30] = next;
            }
            next = pc.wrapping_add((sext((i & 0x03FF_FFFF) as u64, 26) << 2) as u64);
        } else if i & 0xFFFF_FC1F == 0xD61F_0000 || i & 0xFFFF_FC1F == 0xD63F_0000 || i & 0xFFFF_FC1F == 0xD65F_0000 {
            // BR / BLR / RET
            let target = self.x[rn as usize];
            if i & 0xFFFF_FC1F == 0xD63F_0000 {
                self.x[30] = next;
            }
            next = target;
        } else if i & 0xFF00_0010 == 0x5400_0000 {
            // B.cond
            if self.cond(i & 0xF) {
                next = pc.wrapping_add((sext(((i >> 5) & 0x7FFFF) as u64, 19) << 2) as u64);
            }
        } else if i & 0x7E00_0000 == 0x3400_0000 {
            // CBZ / CBNZ
            let v = mask(sf, self.rz(rd));
            if (v == 0) == (i & (1 << 24) == 0) {
                next = pc.wrapping_add((sext(((i >> 5) & 0x7FFFF) as u64, 19) << 2) as u64);
            }
        } else if i & 0x1F00_0000 == 0x1000_0000 {
            // ADR / ADRP
            let imm = (((i >> 5) & 0x7FFFF) << 2 | (i >> 29) & 3) as u64;
            let imm = sext(imm, 21);
            let v = if i >> 31 == 1 {
                (pc & !0xFFF).wrapping_add((imm << 12) as u64)
            } else {
                pc.wrapping_add(imm as u64)
            };
            self.wz(rd, v);
        } else if i & 0x1F80_0000 == 0x1280_0000 {
            // MOVN / MOVZ / MOVK
            let imm = ((i >> 5) & 0xFFFF) as u64;
            let sh = ((i >> 21) & 3) * 16;
            let v = match (i >> 29) & 3 {
                0 => !(imm << sh),
                2 => imm << sh,
                3 => (self.rz(rd) & !(0xFFFFu64 << sh)) | (imm << sh),
                _ => return Err(CpuError::Unimplemented { pc, insn: i }),
            };
            self.wz(rd, mask(sf, v));
        } else if i & 0x1F00_0000 == 0x1100_0000 {
            // ADD/SUB (inmediato)
            let imm = ((i >> 10) & 0xFFF) as u64;
            let imm = if (i >> 22) & 1 == 1 { imm << 12 } else { imm };
            let (sub, set) = ((i >> 30) & 1 == 1, (i >> 29) & 1 == 1);
            let a = self.rs(rn);
            let r = if sub {
                self.add_flags(sf, a, !imm, true, set)
            } else {
                self.add_flags(sf, a, imm, false, set)
            };
            if set { self.wz(rd, r) } else { self.ws(rd, mask(sf, r)) }
        } else if i & 0x1F20_0000 == 0x0B00_0000 {
            // ADD/SUB (registro desplazado)
            let b = Self::shift(sf, self.rz(rm), (i >> 22) & 3, (i >> 10) & 63);
            let (sub, set) = ((i >> 30) & 1 == 1, (i >> 29) & 1 == 1);
            let a = self.rz(rn);
            let r = if sub {
                self.add_flags(sf, a, !b, true, set)
            } else {
                self.add_flags(sf, a, b, false, set)
            };
            self.wz(rd, r);
        } else if i & 0x1F00_0000 == 0x0A00_0000 {
            // AND/ORR/EOR/ANDS (registro desplazado)
            let mut b = Self::shift(sf, self.rz(rm), (i >> 22) & 3, (i >> 10) & 63);
            if (i >> 21) & 1 == 1 {
                b = mask(sf, !b);
            }
            let a = mask(sf, self.rz(rn));
            let opc = (i >> 29) & 3;
            let r = match opc {
                0 | 3 => a & b,
                1 => a | b,
                _ => a ^ b,
            };
            if opc == 3 {
                self.n = (r >> (if sf { 63 } else { 31 })) & 1 == 1;
                self.z = r == 0;
                self.c = false;
                self.v = false;
            }
            self.wz(rd, r);
        } else if i & 0x7FE0_8000 == 0x1B00_0000 {
            // MADD (MUL cuando ra = XZR)
            let ra = (i >> 10) & 31;
            let r = self.rz(ra).wrapping_add(self.rz(rn).wrapping_mul(self.rz(rm)));
            self.wz(rd, mask(sf, r));
        } else if i & 0x7FE0_F800 == 0x1AC0_0800 {
            // UDIV / SDIV (división por cero = 0, como en el hardware)
            let (a, b) = (mask(sf, self.rz(rn)), mask(sf, self.rz(rm)));
            let r = if b == 0 {
                0
            } else if i & 0x400 == 0 {
                a / b
            } else {
                let (sa, sb) = if sf {
                    (a as i64, b as i64)
                } else {
                    (a as u32 as i32 as i64, b as u32 as i32 as i64)
                };
                sa.wrapping_div(sb) as u64
            };
            self.wz(rd, mask(sf, r));
        } else if i & 0x7F80_0000 == 0x5300_0000 {
            // UBFM (LSL/LSR/UBFX/UXTB... inmediatos)
            let (immr, imms) = ((i >> 16) & 63, (i >> 10) & 63);
            let d = if sf { 64 } else { 32 };
            let src = mask(sf, self.rz(rn));
            let m = |w: u32| if w >= 64 { u64::MAX } else { (1u64 << w) - 1 };
            let r = if imms >= immr {
                (src >> immr) & m(imms - immr + 1)
            } else {
                (src & m(imms + 1)) << (d - immr)
            };
            self.wz(rd, mask(sf, r));
        } else if i & 0x3F00_0000 == 0x3900_0000 && (i >> 22) & 3 < 2 {
            // LDR/STR (inmediato sin signo): size 8/16/32/64
            let size = 1usize << (i >> 30);
            let addr = self.rs(rn).wrapping_add((((i >> 10) & 0xFFF) as u64) * size as u64);
            if (i >> 22) & 1 == 1 {
                let v = mem.read_uint(addr, size)?;
                self.wz(rd, v);
            } else {
                mem.write_uint(addr, size, self.rz(rd))?;
            }
        } else if i & 0x3E00_0000 == 0x2800_0000 && (i >> 30) & 1 == 0 && (i >> 26) & 1 == 0 {
            // LDP/STP (64 o 32 bits): post-index, offset, pre-index
            let size = if (i >> 31) & 1 == 1 { 8u64 } else { 4 };
            let off = sext(((i >> 15) & 0x7F) as u64, 7) * size as i64;
            let rt2 = (i >> 10) & 31;
            let base = self.rs(rn);
            let (addr, wb) = match (i >> 23) & 7 {
                1 => (base, Some(base.wrapping_add(off as u64))),
                2 => (base.wrapping_add(off as u64), None),
                3 => (base.wrapping_add(off as u64), Some(base.wrapping_add(off as u64))),
                _ => return Err(CpuError::Unimplemented { pc, insn: i }),
            };
            if (i >> 22) & 1 == 1 {
                let a = mem.read_uint(addr, size as usize)?;
                let b = mem.read_uint(addr + size, size as usize)?;
                self.wz(rd, a);
                self.wz(rt2, b);
            } else {
                mem.write_uint(addr, size as usize, self.rz(rd))?;
                mem.write_uint(addr + size, size as usize, self.rz(rt2))?;
            }
            if let Some(w) = wb {
                self.ws(rn, w);
            }
        } else {
            return Err(CpuError::Unimplemented { pc, insn: i });
        }
        self.pc = next;
        Ok(None)
    }
}
