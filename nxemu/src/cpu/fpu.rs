//! Coma flotante escalar de AArch64 (IEEE 754 de 32 y 64 bits) y cargas/guardados de registros v.

use super::{mask, sext, Cpu, Res};
use crate::memory::Memory;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Operaciones comunes a f32 y f64 para escribir cada instrucción una sola vez.
trait Float: Copy + PartialOrd + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self> + Neg<Output = Self> {
    fn from_bits64(b: u64) -> Self;
    fn bits64(self) -> u64;
    fn is_nan_(self) -> bool;
    fn neg_sign(self) -> bool;
    fn sqrt_(self) -> Self;
    fn abs_(self) -> Self;
    fn fma(self, a: Self, b: Self) -> Self;
    fn to_f64(self) -> f64;
    fn from_f64(v: f64) -> Self;
    fn quiet(self) -> Self;
}

macro_rules! impl_float {
    ($t:ty, $b:ty, $qnan:expr) => {
        impl Float for $t {
            fn from_bits64(b: u64) -> Self { <$t>::from_bits(b as $b) }
            fn bits64(self) -> u64 { self.to_bits() as u64 }
            fn is_nan_(self) -> bool { self.is_nan() }
            fn neg_sign(self) -> bool { self.is_sign_negative() }
            fn sqrt_(self) -> Self { self.sqrt() }
            fn abs_(self) -> Self { self.abs() }
            fn fma(self, a: Self, b: Self) -> Self { self.mul_add(a, b) }
            fn to_f64(self) -> f64 { self as f64 }
            fn from_f64(v: f64) -> Self { v as $t }
            fn quiet(self) -> Self { <$t>::from_bits(self.to_bits() | $qnan) }
        }
    };
}
impl_float!(f32, u32, 0x0040_0000);
impl_float!(f64, u64, 0x0008_0000_0000_0000);

fn fmax<T: Float>(a: T, b: T, num: bool) -> T {
    if a.is_nan_() {
        return if num && !b.is_nan_() { b } else { a.quiet() };
    }
    if b.is_nan_() {
        return if num { a } else { b.quiet() };
    }
    if a > b { a } else if b > a { b } else if a.neg_sign() { b } else { a }
}

fn fmin<T: Float>(a: T, b: T, num: bool) -> T {
    if a.is_nan_() {
        return if num && !b.is_nan_() { b } else { a.quiet() };
    }
    if b.is_nan_() {
        return if num { a } else { b.quiet() };
    }
    if a < b { a } else if b < a { b } else if a.neg_sign() { a } else { b }
}

/// Redondeo a entero con el modo dado: 0 al par más cercano, 1 hacia +inf, 2 hacia -inf, 3 a cero, 4 empates lejos de cero.
fn round_mode(v: f64, mode: u32) -> f64 {
    match mode {
        0 => v.round_ties_even(),
        1 => v.ceil(),
        2 => v.floor(),
        3 => v.trunc(),
        _ => v.round(),
    }
}

fn half_to_f32(h: u16) -> f32 {
    let (s, e, m) = (((h >> 15) & 1) as u32, ((h >> 10) & 0x1F) as u32, (h & 0x3FF) as u32);
    let bits = if e == 0 {
        if m == 0 {
            s << 31
        } else {
            // subnormal: normalizar
            let shift = m.leading_zeros() - 21;
            let m2 = (m << shift) & 0x3FF;
            (s << 31) | ((113 - shift) << 23) | (m2 << 13)
        }
    } else if e == 31 {
        (s << 31) | (0xFF << 23) | (m << 13) | if m != 0 { 0x0040_0000 } else { 0 }
    } else {
        (s << 31) | ((e + 112) << 23) | (m << 13)
    };
    f32::from_bits(bits)
}

fn f32_to_half(f: f32) -> u16 {
    let b = f.to_bits();
    let s = ((b >> 16) & 0x8000) as u16;
    let e = ((b >> 23) & 0xFF) as i32;
    let m = b & 0x007F_FFFF;
    if e == 0xFF {
        return s | 0x7C00 | if m != 0 { 0x0200 | (m >> 13) as u16 } else { 0 };
    }
    let exp = e - 127 + 15;
    if exp >= 31 {
        return s | 0x7C00;
    }
    if exp <= 0 {
        if exp < -10 {
            return s;
        }
        let m = m | 0x0080_0000;
        let shift = (14 - exp) as u32;
        let half = m >> shift;
        let rem = m & ((1 << shift) - 1);
        let mid = 1 << (shift - 1);
        let r = if rem > mid || (rem == mid && half & 1 == 1) { half + 1 } else { half };
        return s | r as u16;
    }
    let half = ((exp as u32) << 10) | (m >> 13);
    let rem = m & 0x1FFF;
    let r = if rem > 0x1000 || (rem == 0x1000 && half & 1 == 1) { half + 1 } else { half };
    s | r as u16
}

/// Expande el inmediato de 8 bits de FMOV (VFPExpandImm).
fn expand_imm(imm8: u32, double: bool) -> u64 {
    let (a, b, cdefgh) = ((imm8 >> 7) & 1, (imm8 >> 6) & 1, imm8 & 0x3F);
    if double {
        let exp = ((b ^ 1) << 10) | ((if b == 1 { 0xFFu32 } else { 0 }) << 2) | (cdefgh >> 4);
        ((a as u64) << 63) | ((exp as u64) << 52) | (((cdefgh & 0xF) as u64) << 48)
    } else {
        let exp = ((b ^ 1) << 7) | ((if b == 1 { 0x1Fu32 } else { 0 }) << 2) | (cdefgh >> 4);
        (((a << 31) | (exp << 23) | ((cdefgh & 0xF) << 19)) as u64) & 0xFFFF_FFFF
    }
}

impl Cpu {
    fn vlow(&self, r: u32) -> u64 {
        self.vr[r as usize] as u64
    }
    /// Escribe un escalar de `bits` bits, poniendo a cero el resto del registro.
    fn set_scalar(&mut self, r: u32, v: u64) {
        self.vr[r as usize] = v as u128;
    }

    fn fp_binary<T: Float>(&mut self, rd: u32, a: u64, b: u64, op: u32) -> Option<()> {
        let (a, b) = (T::from_bits64(a), T::from_bits64(b));
        let r = match op {
            0 => a * b,
            1 => a / b,
            2 => a + b,
            3 => a - b,
            4 => fmax(a, b, false),
            5 => fmin(a, b, false),
            6 => fmax(a, b, true),
            7 => fmin(a, b, true),
            8 => -(a * b),
            _ => return None,
        };
        self.set_scalar(rd, r.bits64());
        Some(())
    }

    fn fp_compare<T: Float>(&mut self, a: u64, b: u64) {
        let (a, b) = (T::from_bits64(a), T::from_bits64(b));
        let (n, z, c, v) = if a.is_nan_() || b.is_nan_() {
            (false, false, true, true)
        } else if a == b {
            (false, true, true, false)
        } else if a < b {
            (true, false, false, false)
        } else {
            (false, false, true, false)
        };
        self.n = n;
        self.z = z;
        self.c = c;
        self.v = v;
    }

    /// Instrucciones escalares de coma flotante (clase 0x1E / 0x1F) y movimientos con registros generales.
    pub(super) fn simd_fp(&mut self, i: u32) -> Option<()> {
        let (rd, rn, rm) = (i & 31, (i >> 5) & 31, (i >> 16) & 31);
        let ty = (i >> 22) & 3;
        let double = ty == 1;
        let sf = i >> 31 == 1;

        if i & 0xFF00_0000 == 0x1F00_0000 {
            // FMADD / FMSUB / FNMADD / FNMSUB
            let ra = (i >> 10) & 31;
            let (o1, o0) = ((i >> 21) & 1, (i >> 15) & 1);
            macro_rules! fma3 {
                ($t:ty) => {{
                    let (n, m, a) = (<$t>::from_bits64(self.vlow(rn)), <$t>::from_bits64(self.vlow(rm)), <$t>::from_bits64(self.vlow(ra)));
                    let r = match (o1, o0) {
                        (0, 0) => n.fma(m, a),
                        (0, 1) => (-n).fma(m, a),
                        (1, 0) => (-n).fma(m, -a),
                        _ => n.fma(m, -a),
                    };
                    self.set_scalar(rd, r.bits64());
                }};
            }
            match ty {
                0 => fma3!(f32),
                1 => fma3!(f64),
                _ => return None,
            }
            return Some(());
        }

        if i & 0xFF20_0C00 == 0x1E20_0800 {
            // FADD/FSUB/FMUL/FDIV/FMAX/FMIN…
            let (a, b) = (self.vlow(rn), self.vlow(rm));
            let op = (i >> 12) & 0xF;
            return match ty {
                0 => self.fp_binary::<f32>(rd, a, b, op),
                1 => self.fp_binary::<f64>(rd, a, b, op),
                _ => None,
            };
        }

        if i & 0xFF20_7C00 == 0x1E20_4000 {
            // 1 fuente: FMOV, FABS, FNEG, FSQRT, FCVT, FRINT*
            let opcode = (i >> 15) & 0x3F;
            let src = self.vlow(rn);
            macro_rules! one {
                ($t:ty) => {{
                    let x = <$t>::from_bits64(src);
                    let r: Option<u64> = match opcode {
                        0 => Some(src),
                        1 => Some(x.abs_().bits64()),
                        2 => Some((-x).bits64()),
                        3 => Some(x.sqrt_().bits64()),
                        8..=12 | 14 | 15 => {
                            let mode = match opcode { 8 => 0, 9 => 1, 10 => 2, 11 => 3, 12 => 4, _ => 0 };
                            Some(<$t>::from_f64(round_mode(x.to_f64(), mode)).bits64())
                        }
                        4 | 5 | 7 => {
                            // FCVT a precisión simple (4), doble (5) o media (7)
                            let v = x.to_f64();
                            Some(match opcode {
                                4 => (v as f32).to_bits() as u64,
                                5 => v.to_bits(),
                                _ => f32_to_half(v as f32) as u64,
                            })
                        }
                        _ => None,
                    };
                    r
                }};
            }
            let r = match ty {
                0 => one!(f32),
                1 => one!(f64),
                3 => {
                    // origen de media precisión: solo FCVT hacia simple/doble
                    let v = half_to_f32(src as u16);
                    match opcode {
                        4 => Some(v.to_bits() as u64),
                        5 => Some((v as f64).to_bits()),
                        _ => None,
                    }
                }
                _ => None,
            }?;
            self.set_scalar(rd, r);
            return Some(());
        }

        if i & 0xFF20_FC07 == 0x1E20_2000 {
            // FCMP / FCMPE
            let a = self.vlow(rn);
            let b = if i & 8 != 0 { 0 } else { self.vlow(rm) };
            match ty {
                0 => self.fp_compare::<f32>(a, b),
                1 => self.fp_compare::<f64>(a, b),
                _ => return None,
            }
            return Some(());
        }

        if i & 0xFF20_0C10 == 0x1E20_0400 {
            // FCCMP / FCCMPE
            if self.cond((i >> 12) & 0xF) {
                let (a, b) = (self.vlow(rn), self.vlow(rm));
                match ty {
                    0 => self.fp_compare::<f32>(a, b),
                    1 => self.fp_compare::<f64>(a, b),
                    _ => return None,
                }
            } else {
                let f = i & 0xF;
                self.n = f & 8 != 0;
                self.z = f & 4 != 0;
                self.c = f & 2 != 0;
                self.v = f & 1 != 0;
            }
            return Some(());
        }

        if i & 0xFF20_0C00 == 0x1E20_0C00 {
            // FCSEL
            let src = if self.cond((i >> 12) & 0xF) { self.vlow(rn) } else { self.vlow(rm) };
            if ty > 1 {
                return None;
            }
            self.set_scalar(rd, src);
            return Some(());
        }

        if i & 0xFF20_1FE0 == 0x1E20_1000 {
            // FMOV (inmediato)
            if ty > 1 {
                return None;
            }
            self.set_scalar(rd, expand_imm((i >> 13) & 0xFF, double));
            return Some(());
        }

        if i & 0x7F20_0000 == 0x1E00_0000 {
            // conversiones con coma fija: SCVTF/UCVTF/FCVTZS/FCVTZU con `#fbits`
            let (rmode, opcode, scale) = ((i >> 19) & 3, (i >> 16) & 7, (i >> 10) & 63);
            if !sf && scale < 32 {
                return None;
            }
            let k = (2f64).powi((64 - scale) as i32);
            match (rmode, opcode) {
                (0, 2) | (0, 3) => {
                    let x = self.rz(rn);
                    let signed = opcode == 2;
                    let r = match ty {
                        0 => {
                            let v = match (signed, sf) {
                                (true, true) => x as i64 as f32,
                                (true, false) => x as u32 as i32 as f32,
                                (false, true) => x as f32,
                                (false, false) => x as u32 as f32,
                            };
                            ((v as f64 / k) as f32).to_bits() as u64
                        }
                        1 => {
                            let v = match (signed, sf) {
                                (true, true) => x as i64 as f64,
                                (true, false) => x as u32 as i32 as f64,
                                (false, true) => x as f64,
                                (false, false) => x as u32 as f64,
                            };
                            (v / k).to_bits()
                        }
                        _ => return None,
                    };
                    self.set_scalar(rd, r);
                }
                (3, 0) | (3, 1) => {
                    let v = match ty {
                        0 => f32::from_bits(self.vlow(rn) as u32) as f64,
                        1 => f64::from_bits(self.vlow(rn)),
                        _ => return None,
                    } * k;
                    let r = v.trunc();
                    let out = match (opcode == 1, sf) {
                        (false, true) => r as i64 as u64,
                        (false, false) => r as i32 as u32 as u64,
                        (true, true) => r as u64,
                        (true, false) => r as u32 as u64,
                    };
                    self.wz(rd, out);
                }
                _ => return None,
            }
            return Some(());
        }

        if i & 0x7F20_FC00 == 0x1E20_0000 {
            // conversiones entre coma flotante y enteros, y FMOV general <-> vector
            let (rmode, opcode) = ((i >> 19) & 3, (i >> 16) & 7);
            match (rmode, opcode) {
                (0, 2) | (0, 3) => {
                    // SCVTF / UCVTF
                    let x = self.rz(rn);
                    let signed = opcode == 2;
                    let f: f64 = match (signed, sf) {
                        (true, true) => x as i64 as f64,
                        (true, false) => x as u32 as i32 as f64,
                        (false, true) => x as f64,
                        (false, false) => (x as u32) as f64,
                    };
                    let r = match ty {
                        0 => {
                            // convertir directamente al entero→f32 evita doble redondeo
                            let v = match (signed, sf) {
                                (true, true) => x as i64 as f32,
                                (true, false) => x as u32 as i32 as f32,
                                (false, true) => x as f32,
                                (false, false) => (x as u32) as f32,
                            };
                            v.to_bits() as u64
                        }
                        1 => f.to_bits(),
                        _ => return None,
                    };
                    self.set_scalar(rd, r);
                }
                (_, 0) | (_, 1) | (0, 4) | (0, 5) => {
                    // FCVT{N,P,M,Z,A}{S,U}
                    let v = match ty {
                        0 => f32::from_bits(self.vlow(rn) as u32) as f64,
                        1 => f64::from_bits(self.vlow(rn)),
                        _ => return None,
                    };
                    let mode = if opcode >= 4 { 4 } else { match rmode { 0 => 0, 1 => 1, 2 => 2, _ => 3 } };
                    let r = round_mode(v, mode);
                    let unsigned = opcode & 1 == 1;
                    let out = match (unsigned, sf) {
                        (false, true) => r as i64 as u64,
                        (false, false) => r as i32 as u32 as u64,
                        (true, true) => r as u64,
                        (true, false) => r as u32 as u64,
                    };
                    self.wz(rd, out);
                }
                (0, 6) => {
                    // FMOV Wd/Xd, Sn/Dn
                    let v = self.vlow(rn);
                    self.wz(rd, if sf { v } else { v & 0xFFFF_FFFF });
                }
                (0, 7) => {
                    // FMOV Sn/Dn, Wn/Xn
                    let v = mask(sf, self.rz(rn));
                    self.set_scalar(rd, v);
                }
                (1, 6) if sf && ty == 2 => {
                    let hi = (self.vr[rn as usize] >> 64) as u64;
                    self.wz(rd, hi);
                }
                (1, 7) if sf && ty == 2 => {
                    let lo = self.vr[rd as usize] & u64::MAX as u128;
                    self.vr[rd as usize] = lo | ((self.rz(rn) as u128) << 64);
                }
                _ => return None,
            }
            return Some(());
        }

        self.simd_vector(i)
    }

    /// Cargas y guardados de registros SIMD/FP (8 a 128 bits), con todos los modos de direccionamiento.
    pub(super) fn ldst_vec(&mut self, i: u32, pc: u64, mem: &mut Memory) -> Option<Res> {
        let (rt, rn) = (i & 31, (i >> 5) & 31);
        let size = i >> 30;
        let opc = (i >> 22) & 3;
        let load = |m: &Memory, addr: u64, bytes: u32| -> Result<u128, crate::memory::MemError> {
            if bytes == 16 { m.read_u128(addr) } else { m.read_uint(addr, bytes as usize).map(|v| v as u128) }
        };
        let store = |m: &mut Memory, addr: u64, bytes: u32, v: u128| -> Result<(), crate::memory::MemError> {
            if bytes == 16 { m.write_u128(addr, v) } else { m.write_uint(addr, bytes as usize, v as u64) }
        };

        if i & 0x3B00_0000 == 0x1800_0000 {
            // LDR (literal) de S, D o Q
            let bytes = match size { 0 => 4, 1 => 8, 2 => 16, _ => return None };
            let addr = pc.wrapping_add((sext(((i >> 5) & 0x7FFFF) as u64, 19) << 2) as u64);
            return Some(load(mem, addr, bytes).map(|v| self.vr[rt as usize] = v).map_err(Into::into));
        }

        if i & 0x3A00_0000 == 0x2800_0000 {
            // LDP / STP de S, D o Q
            let bytes: u32 = match size { 0 => 4, 1 => 8, 2 => 16, _ => return None };
            let l = (i >> 22) & 1 == 1;
            let off = sext(((i >> 15) & 0x7F) as u64, 7) * bytes as i64;
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
                    let (a, b) = (load(mem, addr, bytes)?, load(mem, addr + bytes as u64, bytes)?);
                    self.vr[rt as usize] = a;
                    self.vr[rt2 as usize] = b;
                } else {
                    store(mem, addr, bytes, self.vr[rt as usize])?;
                    store(mem, addr + bytes as u64, bytes, self.vr[rt2 as usize])?;
                }
                if let Some(w) = wb {
                    self.ws(rn, w);
                }
                Ok(())
            })();
            return Some(r);
        }

        if i & 0x3A00_0000 != 0x3800_0000 {
            return None;
        }
        let q = size == 0 && opc & 2 != 0;
        let scale = if q { 4 } else { size };
        let bytes = 1u32 << scale;
        let is_load = opc & 1 == 1;
        let (addr, wb) = if (i >> 24) & 1 == 1 {
            (self.rs(rn).wrapping_add((((i >> 10) & 0xFFF) as u64) << scale), None)
        } else if (i >> 21) & 1 == 1 {
            if (i >> 10) & 3 != 2 {
                return None;
            }
            let option = (i >> 13) & 7;
            let s = (i >> 12) & 1;
            let off = Self::extend(self.rz((i >> 16) & 31), option, if s == 1 { scale } else { 0 });
            (self.rs(rn).wrapping_add(off), None)
        } else {
            let imm = sext(((i >> 12) & 0x1FF) as u64, 9) as u64;
            let base = self.rs(rn);
            match (i >> 10) & 3 {
                0 => (base.wrapping_add(imm), None),
                1 => (base, Some(base.wrapping_add(imm))),
                3 => (base.wrapping_add(imm), Some(base.wrapping_add(imm))),
                _ => return None,
            }
        };
        let r = (|| -> Res {
            if is_load {
                let v = load(mem, addr, bytes)?;
                self.vr[rt as usize] = v;
            } else {
                let v = self.vr[rt as usize];
                store(mem, addr, bytes, v)?;
            }
            if let Some(w) = wb {
                self.ws(rn, w);
            }
            Ok(())
        })();
        Some(r)
    }
}
