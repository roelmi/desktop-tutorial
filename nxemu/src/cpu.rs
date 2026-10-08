//! Intérprete de AArch64 (el ISA de la CPU de la Switch): enteros, memoria, saltos y sistema básico.
//!
//! El decodificador sigue la tabla de clases de instrucciones del manual de ARM
//! (bits 28:25): inmediatos, saltos/sistema, cargas/guardados y registros.

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
    /// Registro de hilo local (TLS), `TPIDR_EL0`.
    pub tpidr_el0: u64,
    /// Registros SIMD/FP v0–v31 (128 bits).
    pub vr: [u128; 32],
    pub fpcr: u64,
    pub fpsr: u64,
}

pub(crate) type Res = Result<(), CpuError>;

mod fpu;
mod neon;

fn mask(sf: bool, v: u64) -> u64 {
    if sf { v } else { v & 0xFFFF_FFFF }
}

fn sext(v: u64, bits: u32) -> i64 {
    ((v << (64 - bits)) as i64) >> (64 - bits)
}

fn ones(n: u32) -> u64 {
    if n >= 64 { u64::MAX } else { (1u64 << n) - 1 }
}

fn ror(v: u64, amt: u32, size: u32) -> u64 {
    let amt = amt % size;
    if amt == 0 {
        v
    } else {
        ((v >> amt) | (v << (size - amt))) & ones(size)
    }
}

/// `DecodeBitMasks` del manual de ARM. Devuelve (wmask, tmask).
fn decode_bit_masks(immn: u32, imms: u32, immr: u32, immediate: bool, datasize: u32) -> Option<(u64, u64)> {
    let combined = (immn << 6) | (!imms & 0x3F);
    if combined == 0 {
        return None;
    }
    let len = 31 - combined.leading_zeros();
    if len < 1 || (1u32 << len) > datasize {
        return None;
    }
    let levels = (1u32 << len) - 1;
    if immediate && (imms & levels) == levels {
        return None;
    }
    let (s, r) = (imms & levels, immr & levels);
    let d = s.wrapping_sub(r) & levels;
    let esize = 1u32 << len;
    let welem = ones(s + 1);
    let telem = ones(d + 1);
    let rotated = ror(welem, r, esize);
    let (mut wmask, mut tmask) = (0u64, 0u64);
    let mut pos = 0;
    while pos < datasize {
        wmask |= rotated << pos;
        tmask |= telem << pos;
        pos += esize;
    }
    Some((mask(datasize == 64, wmask), mask(datasize == 64, tmask)))
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

    /// a + b + carry; actualiza NZCV si `set`.
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

    fn set_nz(&mut self, sf: bool, r: u64) {
        self.n = (r >> (if sf { 63 } else { 31 })) & 1 == 1;
        self.z = r == 0;
        self.c = false;
        self.v = false;
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
        let size = if sf { 64 } else { 32 };
        let v = mask(sf, v);
        let r = match kind {
            0 => v << amt,
            1 => v >> amt,
            2 => ((if sf { v as i64 } else { v as u32 as i32 as i64 }) >> amt) as u64,
            _ => ror(v, amt, size),
        };
        mask(sf, r)
    }

    fn extend(v: u64, option: u32, shift: u32) -> u64 {
        let v = match option {
            0 => v & 0xFF,
            1 => v & 0xFFFF,
            2 => v & 0xFFFF_FFFF,
            3 => v,
            4 => sext(v & 0xFF, 8) as u64,
            5 => sext(v & 0xFFFF, 16) as u64,
            6 => sext(v & 0xFFFF_FFFF, 32) as u64,
            _ => v,
        };
        v << shift
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
        let mut next = pc.wrapping_add(4);
        let unimpl = || CpuError::Unimplemented { pc, insn: i };

        match (i >> 25) & 0xF {
            0b1000 | 0b1001 => self.dp_imm(i, pc).ok_or_else(unimpl)?,
            0b1010 | 0b1011 => {
                if let Some(exit) = self.branch_sys(i, pc, &mut next).ok_or_else(unimpl)? {
                    self.pc = next;
                    return Ok(Some(exit));
                }
            }
            op if op & 0b0101 == 0b0100 => self.load_store(i, pc, mem).ok_or_else(unimpl)??,
            op if op & 0b0111 == 0b0101 => self.dp_reg(i).ok_or_else(unimpl)?,
            op if op & 0b0111 == 0b0111 => self.simd_fp(i).ok_or_else(unimpl)?,
            _ => return Err(unimpl()),
        }
        self.pc = next;
        Ok(None)
    }

    // ---------------------------------------------------------------- inmediatos
    fn dp_imm(&mut self, i: u32, pc: u64) -> Option<()> {
        let (rd, rn) = (i & 31, (i >> 5) & 31);
        let sf = i >> 31 == 1;
        let datasize = if sf { 64 } else { 32 };
        match (i >> 23) & 7 {
            0 | 1 => {
                // ADR / ADRP
                let imm = sext(((((i >> 5) & 0x7FFFF) << 2) | ((i >> 29) & 3)) as u64, 21);
                let v = if i >> 31 == 1 {
                    (pc & !0xFFF).wrapping_add((imm << 12) as u64)
                } else {
                    pc.wrapping_add(imm as u64)
                };
                self.wz(rd, v);
            }
            2 => {
                // ADD/SUB (inmediato)
                let imm = ((i >> 10) & 0xFFF) as u64;
                let imm = if (i >> 22) & 1 == 1 { imm << 12 } else { imm };
                let (sub, set) = ((i >> 30) & 1 == 1, (i >> 29) & 1 == 1);
                let a = self.rs(rn);
                let r = if sub { self.add_flags(sf, a, !imm, true, set) } else { self.add_flags(sf, a, imm, false, set) };
                if set { self.wz(rd, r) } else { self.ws(rd, mask(sf, r)) }
            }
            4 => {
                // AND/ORR/EOR/ANDS (inmediato lógico)
                let (n, immr, imms) = ((i >> 22) & 1, (i >> 16) & 63, (i >> 10) & 63);
                if !sf && n == 1 {
                    return None;
                }
                let (wmask, _) = decode_bit_masks(n, imms, immr, true, datasize)?;
                let a = mask(sf, self.rz(rn));
                let opc = (i >> 29) & 3;
                let r = match opc {
                    0 | 3 => a & wmask,
                    1 => a | wmask,
                    _ => a ^ wmask,
                };
                if opc == 3 {
                    self.set_nz(sf, r);
                    self.wz(rd, r);
                } else {
                    self.ws(rd, r);
                }
            }
            5 => {
                // MOVN / MOVZ / MOVK
                let imm = ((i >> 5) & 0xFFFF) as u64;
                let sh = ((i >> 21) & 3) * 16;
                if !sf && sh > 16 {
                    return None;
                }
                let v = match (i >> 29) & 3 {
                    0 => !(imm << sh),
                    2 => imm << sh,
                    3 => (self.rz(rd) & !(0xFFFFu64 << sh)) | (imm << sh),
                    _ => return None,
                };
                self.wz(rd, mask(sf, v));
            }
            6 => {
                // SBFM / BFM / UBFM
                let (n, immr, imms) = ((i >> 22) & 1, (i >> 16) & 63, (i >> 10) & 63);
                if (n == 1) != sf {
                    return None;
                }
                let (wmask, tmask) = decode_bit_masks(n, imms, immr, false, datasize)?;
                let src = mask(sf, self.rz(rn));
                let bot = ror(src, immr, datasize) & wmask;
                let r = match (i >> 29) & 3 {
                    0 => {
                        let top = if (src >> imms) & 1 == 1 { ones(64) } else { 0 };
                        (top & !tmask) | (bot & tmask)
                    }
                    1 => {
                        let dst = mask(sf, self.rz(rd));
                        let merged = (dst & !wmask) | bot;
                        (dst & !tmask) | (merged & tmask)
                    }
                    2 => bot & tmask,
                    _ => return None,
                };
                self.wz(rd, mask(sf, r));
            }
            7 => {
                // EXTR
                let (rm, lsb) = ((i >> 16) & 31, (i >> 10) & 63);
                let (hi, lo) = (mask(sf, self.rz(rn)), mask(sf, self.rz(rm)));
                let r = if lsb == 0 {
                    lo
                } else if sf {
                    (lo >> lsb) | (hi << (64 - lsb))
                } else {
                    ((lo >> lsb) | (hi << (32 - lsb))) & 0xFFFF_FFFF
                };
                self.wz(rd, r);
            }
            _ => return None,
        }
        Some(())
    }

    // ------------------------------------------------------------ saltos y sistema
    fn branch_sys(&mut self, i: u32, pc: u64, next: &mut u64) -> Option<Option<Exit>> {
        let (rd, rn) = (i & 31, (i >> 5) & 31);
        let sf = i >> 31 == 1;
        if i & 0x7C00_0000 == 0x1400_0000 {
            // B / BL
            if i >> 31 == 1 {
                self.x[30] = *next;
            }
            *next = pc.wrapping_add((sext((i & 0x03FF_FFFF) as u64, 26) << 2) as u64);
        } else if i & 0xFF00_0010 == 0x5400_0000 {
            // B.cond
            if self.cond(i & 0xF) {
                *next = pc.wrapping_add((sext(((i >> 5) & 0x7FFFF) as u64, 19) << 2) as u64);
            }
        } else if i & 0x7E00_0000 == 0x3400_0000 {
            // CBZ / CBNZ
            let v = mask(sf, self.rz(rd));
            if (v == 0) == (i & (1 << 24) == 0) {
                *next = pc.wrapping_add((sext(((i >> 5) & 0x7FFFF) as u64, 19) << 2) as u64);
            }
        } else if i & 0x7E00_0000 == 0x3600_0000 {
            // TBZ / TBNZ
            let bit = ((i >> 31) << 5) | ((i >> 19) & 31);
            let set = (self.rz(rd) >> bit) & 1 == 1;
            if set == (i & (1 << 24) != 0) {
                *next = pc.wrapping_add((sext(((i >> 5) & 0x3FFF) as u64, 14) << 2) as u64);
            }
        } else if i & 0xFFDF_FC1F == 0xD61F_0000 {
            // BR / BLR (bit 21 distingue BLR)
            let target = self.x[rn as usize];
            if i & (1 << 21) != 0 {
                self.x[30] = *next;
            }
            *next = target;
        } else if i & 0xFFFF_FC1F == 0xD65F_0000 {
            // RET
            *next = self.x[rn as usize];
        } else if i & 0xFFE0_001F == 0xD400_0001 {
            return Some(Some(Exit::Svc((i >> 5) & 0xFFFF)));
        } else if i & 0xFFFF_F01F == 0xD503_201F || i & 0xFFFF_F01F == 0xD503_301F {
            // NOP, YIELD, WFE… y barreras DMB/DSB/ISB/CLREX: sin efecto en un solo hilo
        } else if i & 0xFFF0_0000 == 0xD530_0000 {
            // MRS: solo TPIDR_EL0 (TLS) y contador de tiempo
            let sysreg = (i >> 5) & 0x7FFF;
            match sysreg {
                0x5E82 => self.wz(rd, self.tpidr_el0),
                0x5F02 => self.wz(rd, 0), // CNTVCT_EL0 (sin reloj aún)
                0x5A20 => self.wz(rd, self.fpcr),
                0x5A21 => self.wz(rd, self.fpsr),
                _ => return None,
            }
        } else if i & 0xFFF0_0000 == 0xD510_0000 {
            // MSR: TPIDR_EL0
            match (i >> 5) & 0x7FFF {
                0x5E82 => self.tpidr_el0 = self.rz(rd),
                0x5A20 => self.fpcr = self.rz(rd),
                0x5A21 => self.fpsr = self.rz(rd),
                _ => return None,
            }
        } else {
            return None;
        }
        Some(None)
    }

    // ----------------------------------------------------------- cargas y guardados
    fn load_store(&mut self, i: u32, pc: u64, mem: &mut Memory) -> Option<Res> {
        let (rt, rn) = (i & 31, (i >> 5) & 31);
        let size_log = i >> 30;
        let vec = (i >> 26) & 1 == 1;
        if vec {
            return self.ldst_vec(i, pc, mem);
        }

        // LDR (literal)
        if i & 0x3B00_0000 == 0x1800_0000 {
            let addr = pc.wrapping_add((sext(((i >> 5) & 0x7FFFF) as u64, 19) << 2) as u64);
            let r = match size_log {
                0 => mem.read_uint(addr, 4),
                1 => mem.read_uint(addr, 8),
                2 => mem.read_uint(addr, 4).map(|v| sext(v, 32) as u64),
                _ => return Some(Ok(())), // PRFM
            };
            return Some(r.map(|v| self.wz(rt, v)).map_err(Into::into));
        }

        // LDP / STP
        if i & 0x3A00_0000 == 0x2800_0000 {
            let l = (i >> 22) & 1 == 1;
            let opc = i >> 30;
            let (size, signed) = match (opc, l) {
                (0, _) => (4u64, false),
                (1, true) => (4, true), // LDPSW
                (2, _) => (8, false),
                _ => return None,
            };
            let off = sext(((i >> 15) & 0x7F) as u64, 7) * size as i64;
            let rt2 = (i >> 10) & 31;
            let base = self.rs(rn);
            let (addr, wb) = match (i >> 23) & 7 {
                1 => (base, Some(base.wrapping_add(off as u64))),
                2 => (base.wrapping_add(off as u64), None),
                3 => (base.wrapping_add(off as u64), Some(base.wrapping_add(off as u64))),
                _ => return None,
            };
            let r = (|| -> Res {
                if l {
                    let (mut a, mut b) = (mem.read_uint(addr, size as usize)?, mem.read_uint(addr + size, size as usize)?);
                    if signed {
                        a = sext(a, 32) as u64;
                        b = sext(b, 32) as u64;
                    }
                    self.wz(rt, a);
                    self.wz(rt2, b);
                } else {
                    mem.write_uint(addr, size as usize, self.rz(rt))?;
                    mem.write_uint(addr + size, size as usize, self.rz(rt2))?;
                }
                if let Some(w) = wb {
                    self.ws(rn, w);
                }
                Ok(())
            })();
            return Some(r);
        }

        // Exclusivos / adquirir-liberar: STXR, LDXR, STLR, LDAR… (un solo hilo: acceso normal)
        if i & 0x3F00_0000 == 0x0800_0000 {
            if (i >> 21) & 1 == 1 {
                return None; // pares exclusivos / CAS
            }
            let size = 1usize << size_log;
            let l = (i >> 22) & 1 == 1;
            let addr = self.rs(rn);
            let r = (|| -> Res {
                if l {
                    let v = mem.read_uint(addr, size)?;
                    self.wz(rt, v);
                } else {
                    mem.write_uint(addr, size, self.rz(rt))?;
                    let o2 = (i >> 23) & 1;
                    if o2 == 0 {
                        // STXR/STLXR: siempre con éxito
                        let rs = (i >> 16) & 31;
                        self.wz(rs, 0);
                    }
                }
                Ok(())
            })();
            return Some(r);
        }

        // LDR/STR en sus cuatro formas
        if i & 0x3A00_0000 != 0x3800_0000 {
            return None;
        }
        let opc = (i >> 22) & 3;
        let unsigned = (i >> 24) & 1 == 1;
        let (addr, wb) = if unsigned {
            (self.rs(rn).wrapping_add((((i >> 10) & 0xFFF) as u64) << size_log), None)
        } else if (i >> 21) & 1 == 1 {
            if (i >> 10) & 3 != 2 {
                return None;
            }
            // registro desplazado/extendido
            let rm = (i >> 16) & 31;
            let option = (i >> 13) & 7;
            let s = (i >> 12) & 1;
            let off = Self::extend(self.rz(rm), option, if s == 1 { size_log } else { 0 });
            (self.rs(rn).wrapping_add(off), None)
        } else {
            let imm = sext(((i >> 12) & 0x1FF) as u64, 9) as u64;
            let base = self.rs(rn);
            match (i >> 10) & 3 {
                0 => (base.wrapping_add(imm), None),                                    // LDUR/STUR
                1 => (base, Some(base.wrapping_add(imm))),                              // post-index
                3 => (base.wrapping_add(imm), Some(base.wrapping_add(imm))),            // pre-index
                _ => return None,                                                       // acceso no privilegiado
            }
        };
        let size = 1usize << size_log;
        let r = (|| -> Res {
            match opc {
                0 => mem.write_uint(addr, size, self.rz(rt))?,
                1 => {
                    let v = mem.read_uint(addr, size)?;
                    self.wz(rt, v);
                }
                2 | 3 => {
                    if size_log == 3 {
                        return Ok(()); // PRFM
                    }
                    let v = sext(mem.read_uint(addr, size)?, (size * 8) as u32) as u64;
                    self.wz(rt, if opc == 3 { v & 0xFFFF_FFFF } else { v });
                }
                _ => unreachable!(),
            }
            if let Some(w) = wb {
                self.ws(rn, w);
            }
            Ok(())
        })();
        Some(r)
    }

    // ---------------------------------------------------------------- registros
    fn dp_reg(&mut self, i: u32) -> Option<()> {
        let (rd, rn, rm) = (i & 31, (i >> 5) & 31, (i >> 16) & 31);
        let sf = i >> 31 == 1;
        let bits = if sf { 64 } else { 32 };

        if (i >> 28) & 1 == 0 {
            if (i >> 24) & 1 == 0 {
                // AND/ORR/EOR/ANDS (registro desplazado) y sus variantes con negación
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
                    self.set_nz(sf, r);
                }
                self.wz(rd, r);
            } else {
                // ADD/SUB (registro desplazado o extendido)
                let (sub, set) = ((i >> 30) & 1 == 1, (i >> 29) & 1 == 1);
                let extended = (i >> 21) & 1 == 1;
                let (a, b) = if extended {
                    let b = Self::extend(self.rz(rm), (i >> 13) & 7, (i >> 10) & 7);
                    (self.rs(rn), b)
                } else {
                    (self.rz(rn), Self::shift(sf, self.rz(rm), (i >> 22) & 3, (i >> 10) & 63))
                };
                let r = if sub { self.add_flags(sf, a, !b, true, set) } else { self.add_flags(sf, a, b, false, set) };
                if extended && !set { self.ws(rd, r) } else { self.wz(rd, r) }
            }
            return Some(());
        }

        match (i >> 21) & 0xF {
            0b0000 => {
                // ADC / SBC
                let (sub, set) = ((i >> 30) & 1 == 1, (i >> 29) & 1 == 1);
                let b = if sub { !self.rz(rm) } else { self.rz(rm) };
                let r = self.add_flags(sf, self.rz(rn), b, self.c, set);
                self.wz(rd, r);
            }
            0b0010 => {
                // CCMN / CCMP
                if (i >> 29) & 1 == 0 {
                    return None;
                }
                let sub = (i >> 30) & 1 == 1;
                if self.cond((i >> 12) & 0xF) {
                    let b = if (i >> 11) & 1 == 1 { ((i >> 16) & 31) as u64 } else { self.rz(rm) };
                    let a = self.rz(rn);
                    if sub { self.add_flags(sf, a, !b, true, true) } else { self.add_flags(sf, a, b, false, true) };
                } else {
                    let f = i & 0xF;
                    self.n = f & 8 != 0;
                    self.z = f & 4 != 0;
                    self.c = f & 2 != 0;
                    self.v = f & 1 != 0;
                }
            }
            0b0100 => {
                // CSEL / CSINC / CSINV / CSNEG
                if (i >> 29) & 1 == 1 {
                    return None;
                }
                let (op, op2) = ((i >> 30) & 1, (i >> 10) & 1);
                let r = if self.cond((i >> 12) & 0xF) {
                    self.rz(rn)
                } else {
                    let m = self.rz(rm);
                    match (op, op2) {
                        (0, 0) => m,
                        (0, 1) => m.wrapping_add(1),
                        (1, 0) => !m,
                        _ => (!m).wrapping_add(1),
                    }
                };
                self.wz(rd, mask(sf, r));
            }
            0b0110 => {
                if (i >> 29) & 1 == 1 {
                    return None;
                }
                let opcode = (i >> 10) & 0x3F;
                if (i >> 30) & 1 == 1 {
                    // datos de 1 fuente
                    let v = mask(sf, self.rz(rn));
                    let r = match (opcode, sf) {
                        (0, _) => (if sf { v.reverse_bits() } else { (v as u32).reverse_bits() as u64 }),
                        (1, _) => {
                            let w = if sf { v } else { v & 0xFFFF_FFFF };
                            ((w & 0x00FF_00FF_00FF_00FF) << 8) | ((w >> 8) & 0x00FF_00FF_00FF_00FF)
                        }
                        (2, false) => (v as u32).swap_bytes() as u64,
                        (2, true) => {
                            let lo = (v as u32).swap_bytes() as u64;
                            let hi = ((v >> 32) as u32).swap_bytes() as u64;
                            (hi << 32) | lo
                        }
                        (3, true) => v.swap_bytes(),
                        (4, _) => (if sf { v.leading_zeros() } else { (v as u32).leading_zeros() }) as u64,
                        (5, _) => {
                            // CLS: bits iguales al de signo que lo siguen
                            if sf {
                                let t = if v >> 63 == 1 { !v } else { v };
                                t.leading_zeros() as u64 - 1
                            } else {
                                let w = v as u32;
                                let t = if w >> 31 == 1 { !w } else { w };
                                t.leading_zeros() as u64 - 1
                            }
                        }
                        _ => return None,
                    };
                    self.wz(rd, mask(sf, r));
                } else {
                    // datos de 2 fuentes
                    let (a, b) = (mask(sf, self.rz(rn)), mask(sf, self.rz(rm)));
                    let r = match opcode {
                        2 => if b == 0 { 0 } else { a / b },
                        3 => {
                            if b == 0 {
                                0
                            } else if sf {
                                (a as i64).wrapping_div(b as i64) as u64
                            } else {
                                (a as u32 as i32).wrapping_div(b as u32 as i32) as u32 as u64
                            }
                        }
                        8..=11 => {
                            let amt = (b & (bits as u64 - 1)) as u32;
                            Self::shift(sf, a, opcode - 8, amt)
                        }
                        _ => return None,
                    };
                    self.wz(rd, mask(sf, r));
                }
            }
            op if op & 0b1000 != 0 => {
                // datos de 3 fuentes: MADD, MSUB, SMADDL…, SMULH, UMULH
                let ra = (i >> 10) & 31;
                let o0 = (i >> 15) & 1;
                let op31 = (i >> 21) & 7;
                let (a, b, c) = (self.rz(rn), self.rz(rm), self.rz(ra));
                let acc = |p: u64| if o0 == 0 { c.wrapping_add(p) } else { c.wrapping_sub(p) };
                let r = match op31 {
                    0 => mask(sf, acc(a.wrapping_mul(b))),
                    1 if sf => acc((a as u32 as i32 as i64).wrapping_mul(b as u32 as i32 as i64) as u64),
                    5 if sf => acc((a & 0xFFFF_FFFF).wrapping_mul(b & 0xFFFF_FFFF)),
                    2 if sf && o0 == 0 => (((a as i64 as i128) * (b as i64 as i128)) >> 64) as u64,
                    6 if sf && o0 == 0 => (((a as u128) * (b as u128)) >> 64) as u64,
                    _ => return None,
                };
                self.wz(rd, r);
            }
            _ => return None,
        }
        Some(())
    }
}
