//! Instrucciones SIMD (NEON) de AArch64: operaciones vectoriales y sus variantes escalares.
//!
//! Se decodifica por grupos de codificación del manual de ARM (tres registros iguales/distintos, dos registros,
//! a través de carriles, copia, inmediato modificado, desplazamientos, elemento indexado, permutaciones…).
//! Las operaciones de un solo carril se comparten entre las formas vectorial y escalar.

use super::fpu::{expand_imm, fmax, fmin, round_mode, Float};
use super::{sext, Cpu};

fn m(es: u32) -> u64 {
    if es >= 64 { u64::MAX } else { (1u64 << es) - 1 }
}
fn gl(v: u128, es: u32, i: u32) -> u64 {
    ((v >> (i * es)) as u64) & m(es)
}
fn put(v: &mut u128, es: u32, i: u32, x: u64) {
    let sh = i * es;
    let mk = (m(es) as u128) << sh;
    *v = (*v & !mk) | (((x & m(es)) as u128) << sh);
}
fn si(x: u64, es: u32) -> i64 {
    sext(x, es)
}
fn sat_s(x: i128, es: u32) -> u64 {
    let (max, min) = ((1i128 << (es - 1)) - 1, -(1i128 << (es - 1)));
    (x.clamp(min, max) as i64 as u64) & m(es)
}
fn sat_u(x: i128, es: u32) -> u64 {
    x.clamp(0, (1i128 << es) - 1) as u64
}
fn rep(x: u64, es: u32) -> u64 {
    let mut r = 0u64;
    let mut p = 0;
    while p < 64 {
        r |= (x & m(es)) << p;
        p += es;
    }
    r
}
/// Máscara de `bits` (64 o 128) bits.
fn dmask(q: bool) -> u128 {
    if q { u128::MAX } else { u64::MAX as u128 }
}

/// Desplazamiento por registro (SSHL/USHL/SRSHL/URSHL) de un carril.
fn shl_reg(x: u64, sh: u64, es: u32, signed: bool, round: bool) -> u64 {
    let s = (sh as u8) as i8 as i64;
    if s >= 0 {
        return if s >= es as i64 { 0 } else { (x << s) & m(es) };
    }
    let r = (-s) as u32;
    let v: i128 = if signed { si(x, es) as i128 } else { x as i128 };
    let rnd = if round { 1i128 << (r - 1) } else { 0 };
    (((v + rnd) >> r.min(127)) as u64) & m(es)
}

/// Operación entera de tres registros sobre un carril. `None` si no está soportada.
fn int3(u: u32, opc: u32, es: u32, x: u64, y: u64, acc: u64) -> Option<u64> {
    let (sx, sy) = (si(x, es) as i128, si(y, es) as i128);
    let (ux, uy) = (x as i128, y as i128);
    let (a, b) = if u == 0 { (sx, sy) } else { (ux, uy) };
    let ones = m(es);
    let r: u64 = match opc {
        0x00 => ((a + b) >> 1) as u64,
        0x01 => if u == 0 { sat_s(sx + sy, es) } else { sat_u(ux + uy, es) },
        0x02 => ((a + b + 1) >> 1) as u64,
        0x04 => ((a - b) >> 1) as u64,
        0x05 => if u == 0 { sat_s(sx - sy, es) } else { sat_u(ux - uy, es) },
        0x06 => if a > b { ones } else { 0 },
        0x07 => if a >= b { ones } else { 0 },
        0x08 => shl_reg(x, y, es, u == 0, false),
        0x0A => shl_reg(x, y, es, u == 0, true),
        0x0C => a.max(b) as u64,
        0x0D => a.min(b) as u64,
        0x0E => (a - b).abs() as u64,
        0x0F => acc.wrapping_add((a - b).abs() as u64),
        0x10 => if u == 0 { x.wrapping_add(y) } else { x.wrapping_sub(y) },
        0x11 => if u == 0 { if x & y != 0 { ones } else { 0 } } else if x == y { ones } else { 0 },
        0x12 => if u == 0 { acc.wrapping_add(x.wrapping_mul(y)) } else { acc.wrapping_sub(x.wrapping_mul(y)) },
        0x13 => if u == 0 { x.wrapping_mul(y) } else { return None },
        0x16 => {
            // SQDMULH / SQRDMULH
            let p = 2 * sx * sy + if u == 1 { 1i128 << (es - 1) } else { 0 };
            sat_s(p >> es, es)
        }
        _ => return None,
    };
    Some(r & ones)
}

/// Operación de coma flotante de tres registros sobre un carril (f32 o f64).
fn fp3<T: Float>(u: u32, a: u32, opc: u32, x: u64, y: u64, acc: u64, ones: u64) -> Option<u64> {
    let (x, y, c) = (T::from_bits64(x), T::from_bits64(y), T::from_bits64(acc));
    let sel = |b: bool| if b { ones } else { 0 };
    Some(match (u, opc, a) {
        (0, 0x18, 0) => fmax(x, y, true).bits64(),
        (0, 0x18, 1) => fmin(x, y, true).bits64(),
        (0, 0x19, 0) => x.fma(y, c).bits64(),
        (0, 0x19, 1) => (-x).fma(y, c).bits64(),
        (0, 0x1A, 0) => (x + y).bits64(),
        (0, 0x1A, 1) => (x - y).bits64(),
        (0, 0x1B, _) => (x * y).bits64(),
        (0, 0x1C, 0) => sel(x == y),
        (0, 0x1E, 0) => fmax(x, y, false).bits64(),
        (0, 0x1E, 1) => fmin(x, y, false).bits64(),
        (1, 0x1A, 1) => (x - y).abs_().bits64(),
        (1, 0x1B, 0) => (x * y).bits64(),
        (1, 0x1C, 0) => sel(x >= y),
        (1, 0x1C, 1) => sel(x > y),
        (1, 0x1D, 0) => sel(x.abs_() >= y.abs_()),
        (1, 0x1D, 1) => sel(x.abs_() > y.abs_()),
        (1, 0x1F, 0) => (x / y).bits64(),
        _ => return None,
    })
}

fn fp3_dispatch(sz: bool, u: u32, a: u32, opc: u32, x: u64, y: u64, acc: u64) -> Option<u64> {
    if sz { fp3::<f64>(u, a, opc, x, y, acc, u64::MAX) } else { fp3::<f32>(u, a, opc, x, y, acc, 0xFFFF_FFFF) }
}

impl Cpu {
    /// Escribe un vector de 64 o 128 bits (el resto del registro se pone a cero).
    fn setv(&mut self, r: u32, v: u128, q: bool) {
        self.vr[r as usize] = v & dmask(q);
    }

    /// Decodifica instrucciones del banco SIMD que no son de coma flotante escalar clásica.
    pub(super) fn simd_vector(&mut self, i: u32) -> Option<()> {
        let q = (i >> 30) & 1 == 1;

        // ---- formas escalares: `01 U 11110 …` y `01 U 11111 …`
        if i & 0xDF00_0000 == 0x5E00_0000 {
            return self.simd_scalar(i);
        }
        if i & 0xDF00_0000 == 0x5F00_0000 {
            return self.simd_scalar_shift_indexed(i);
        }
        if i >> 31 != 0 {
            return None;
        }

        if i & 0x9F20_0400 == 0x0E20_0400 {
            self.three_same(i, q)
        } else if i & 0x9F20_0C00 == 0x0E20_0000 {
            self.three_diff(i, q)
        } else if i & 0x9F3E_0C00 == 0x0E20_0800 {
            self.two_reg_misc(i, q)
        } else if i & 0x9F3E_0C00 == 0x0E30_0800 {
            self.across_lanes(i, q)
        } else if i & 0x9FE0_8400 == 0x0E00_0400 {
            self.copy(i, q)
        } else if i & 0xBF20_8C00 == 0x0E00_0800 {
            self.permute(i, q)
        } else if i & 0xBF20_8C00 == 0x0E00_0000 {
            self.table_lookup(i, q)
        } else if i & 0xBFE0_8400 == 0x2E00_0000 {
            self.extract(i, q)
        } else if i & 0x9FF8_0400 == 0x0F00_0400 {
            self.mod_imm(i, q)
        } else if i & 0x9F80_0400 == 0x0F00_0400 {
            self.shift_imm(i, q)
        } else if i & 0x9F00_0400 == 0x0F00_0000 {
            self.indexed(i, q)
        } else {
            None
        }
    }

    // ------------------------------------------------------------------ tres iguales
    fn three_same(&mut self, i: u32, q: bool) -> Option<()> {
        let (rd, rn, rm) = ((i & 31) as usize, ((i >> 5) & 31) as usize, ((i >> 16) & 31) as usize);
        let (u, size, opc) = ((i >> 29) & 1, (i >> 22) & 3, (i >> 11) & 0x1F);
        let (a, b, d) = (self.vr[rn], self.vr[rm], self.vr[rd]);
        let dm = dmask(q);

        if opc == 0x03 {
            let r = match (u, size) {
                (0, 0) => a & b,
                (0, 1) => a & !b,
                (0, 2) => a | b,
                (0, 3) => a | !b,
                (1, 0) => a ^ b,
                (1, 1) => (d & a) | (!d & b),
                (1, 2) => (d & !b) | (a & b),
                _ => (d & b) | (a & !b),
            };
            self.vr[rd] = r & dm;
            return Some(());
        }

        let bits = if q { 128 } else { 64 };
        let mut res = 0u128;

        if opc >= 0x18 {
            // coma flotante
            let sz = size & 1 == 1;
            let aflag = size >> 1;
            let es = if sz { 64 } else { 32 };
            if sz && !q {
                return None;
            }
            // formas por pares
            let pairwise = matches!((u, opc), (1, 0x18) | (1, 0x1A) | (1, 0x1E)) && !(u == 1 && opc == 0x1A && aflag == 1);
            if pairwise {
                let n = bits / es;
                let cat = |k: u32| if k < n { gl(a, es, k) } else { gl(b, es, k - n) };
                for k in 0..n {
                    let (x, y) = (cat(2 * k), cat(2 * k + 1));
                    let r = match (opc, aflag) {
                        (0x1A, 0) => if sz { (f64::from_bits(x) + f64::from_bits(y)).to_bits() } else { (f32::from_bits(x as u32) + f32::from_bits(y as u32)).to_bits() as u64 },
                        (0x1E, 0) => if sz { fmax(f64::from_bits(x), f64::from_bits(y), false).to_bits() } else { fmax(f32::from_bits(x as u32), f32::from_bits(y as u32), false).to_bits() as u64 },
                        (0x1E, 1) => if sz { fmin(f64::from_bits(x), f64::from_bits(y), false).to_bits() } else { fmin(f32::from_bits(x as u32), f32::from_bits(y as u32), false).to_bits() as u64 },
                        (0x18, 0) => if sz { fmax(f64::from_bits(x), f64::from_bits(y), true).to_bits() } else { fmax(f32::from_bits(x as u32), f32::from_bits(y as u32), true).to_bits() as u64 },
                        (0x18, 1) => if sz { fmin(f64::from_bits(x), f64::from_bits(y), true).to_bits() } else { fmin(f32::from_bits(x as u32), f32::from_bits(y as u32), true).to_bits() as u64 },
                        _ => return None,
                    };
                    put(&mut res, es, k, r);
                }
            } else {
                for k in 0..bits / es {
                    let r = fp3_dispatch(sz, u, aflag, opc, gl(a, es, k), gl(b, es, k), gl(d, es, k))?;
                    put(&mut res, es, k, r);
                }
            }
            self.vr[rd] = res & dm;
            return Some(());
        }

        let es = 8 << size;
        if size == 3 && !q && opc != 0x10 {
            // 1D solo existe en formas escalares
        }
        let n = bits / es;
        // por pares: ADDP, SMAXP/UMAXP, SMINP/UMINP
        if matches!(opc, 0x14 | 0x15 | 0x17) {
            let cat = |k: u32| if k < n { gl(a, es, k) } else { gl(b, es, k - n) };
            for k in 0..n {
                let (x, y) = (cat(2 * k), cat(2 * k + 1));
                let (sa, sb) = if u == 0 { (si(x, es) as i128, si(y, es) as i128) } else { (x as i128, y as i128) };
                let r = match opc {
                    0x17 if u == 0 => x.wrapping_add(y),
                    0x14 => sa.max(sb) as u64,
                    0x15 => sa.min(sb) as u64,
                    _ => return None,
                };
                put(&mut res, es, k, r);
            }
        } else {
            for k in 0..n {
                let r = int3(u, opc, es, gl(a, es, k), gl(b, es, k), gl(d, es, k))?;
                put(&mut res, es, k, r);
            }
        }
        self.vr[rd] = res & dm;
        Some(())
    }

    // ------------------------------------------------------------------ tres distintos
    fn three_diff(&mut self, i: u32, q: bool) -> Option<()> {
        let (rd, rn, rm) = ((i & 31) as usize, ((i >> 5) & 31) as usize, ((i >> 16) & 31) as usize);
        let (u, size, opc) = ((i >> 29) & 1, (i >> 22) & 3, (i >> 12) & 0xF);
        if size == 3 {
            return None;
        }
        let es = 8 << size; // tamaño de carril estrecho
        let wide = es * 2;
        let (a, b, d) = (self.vr[rn], self.vr[rm], self.vr[rd]);
        let signed = u == 0;
        let ext = |v: u64, bits: u32| -> i128 { if signed { si(v, bits) as i128 } else { v as i128 } };
        let half = if q { 64 / es } else { 0 };
        let n = 64 / es;
        let mut res = 0u128;
        match opc {
            // ADDHN / SUBHN (y redondeo): fuentes anchas, resultado estrecho
            4 | 6 => {
                let n2 = 128 / wide;
                let mut out = if q { d & u64::MAX as u128 } else { 0 };
                for k in 0..n2 {
                    let (x, y) = (gl(a, wide, k) as u128, gl(b, wide, k) as u128);
                    let mut s = if opc == 4 { x.wrapping_add(y) } else { x.wrapping_sub(y) };
                    if u == 1 {
                        s = s.wrapping_add(1u128 << (wide - 1));
                    }
                    let hi = ((s >> es) as u64) & m(es);
                    put(&mut out, es, k + if q { n2 } else { 0 }, hi);
                }
                self.vr[rd] = out;
                return Some(());
            }
            _ => {}
        }
        for k in 0..n {
            let (x, y) = (gl(a, es, k + half), gl(b, es, k + half));
            let acc = gl(d, wide, k);
            let r: u64 = match opc {
                0 => (ext(x, es) + ext(y, es)) as u64,
                1 => {
                    let xw = gl(a, wide, k);
                    (ext(xw, wide) + ext(y, es)) as u64
                }
                2 => (ext(x, es) - ext(y, es)) as u64,
                3 => {
                    let xw = gl(a, wide, k);
                    (ext(xw, wide) - ext(y, es)) as u64
                }
                5 => acc.wrapping_add((ext(x, es) - ext(y, es)).abs() as u64),
                7 => (ext(x, es) - ext(y, es)).abs() as u64,
                8 => acc.wrapping_add((ext(x, es) * ext(y, es)) as u64),
                10 => acc.wrapping_sub((ext(x, es) * ext(y, es)) as u64),
                12 => (ext(x, es) * ext(y, es)) as u64,
                _ => return None,
            };
            put(&mut res, wide, k, r);
        }
        self.vr[rd] = res;
        Some(())
    }

    // ------------------------------------------------------------ dos registros, misceláneo
    fn two_reg_misc(&mut self, i: u32, q: bool) -> Option<()> {
        let (rd, rn) = ((i & 31) as usize, ((i >> 5) & 31) as usize);
        let (u, size, opc) = ((i >> 29) & 1, (i >> 22) & 3, (i >> 12) & 0x1F);
        let a = self.vr[rn];
        let d = self.vr[rd];
        let bits = if q { 128 } else { 64 };
        let es = 8 << size;
        let mut res = 0u128;

        // operaciones de coma flotante
        let sz = size & 1 == 1;
        let aflag = size >> 1;
        let fes = if sz { 64 } else { 32 };
        let is_fp = matches!(opc, 0x0C..=0x0F | 0x16..=0x1F);
        if is_fp {
            match opc {
                0x16 => {
                    // FCVTN: f64 -> f32 (sz=1)
                    if !sz {
                        return None;
                    }
                    let mut out = if q { d & u64::MAX as u128 } else { 0 };
                    for k in 0..2u32 {
                        let v = f64::from_bits(gl(a, 64, k)) as f32;
                        put(&mut out, 32, k + if q { 2 } else { 0 }, v.to_bits() as u64);
                    }
                    self.vr[rd] = out;
                    return Some(());
                }
                0x17 => {
                    // FCVTL: f32 -> f64 (sz=1)
                    if !sz {
                        return None;
                    }
                    let off = if q { 2 } else { 0 };
                    for k in 0..2u32 {
                        let v = f32::from_bits(gl(a, 32, k + off) as u32) as f64;
                        put(&mut res, 64, k, v.to_bits());
                    }
                    self.vr[rd] = res;
                    return Some(());
                }
                _ => {}
            }
            if sz && !q {
                return None;
            }
            for k in 0..bits / fes {
                let x = gl(a, fes, k);
                let r = fp_misc(u, aflag, opc, sz, x)?;
                put(&mut res, fes, k, r);
            }
            self.vr[rd] = res & dmask(q);
            return Some(());
        }

        let n = bits / es;
        match (u, opc) {
            // REV64 / REV32 / REV16
            (0, 0x00) | (1, 0x00) | (0, 0x01) => {
                let group = match (u, opc) { (0, 0) => 64, (1, 0) => 32, _ => 16 };
                let per = group / es;
                for k in 0..n {
                    let (blk, pos) = (k / per, k % per);
                    put(&mut res, es, blk * per + (per - 1 - pos), gl(a, es, k));
                }
            }
            (0, 0x05) | (1, 0x05) if size == 0 || (u == 1 && size == 1) => {
                for k in 0..bits / 8 {
                    let x = gl(a, 8, k);
                    let r = if u == 0 {
                        x.count_ones() as u64
                    } else if size == 0 {
                        !x & 0xFF
                    } else {
                        (x as u8).reverse_bits() as u64
                    };
                    put(&mut res, 8, k, r);
                }
            }
            (_, 0x04) => {
                for k in 0..n {
                    let x = gl(a, es, k);
                    let r = if u == 1 {
                        (x << (64 - es)).leading_zeros().min(es) as u64
                    } else {
                        let s = si(x, es);
                        let t = if s < 0 { !(s as u64) & m(es) } else { x };
                        ((t << (64 - es)).leading_zeros().min(es) as u64).saturating_sub(1)
                    };
                    put(&mut res, es, k, r);
                }
            }
            (0, 0x08) | (0, 0x09) | (0, 0x0A) | (1, 0x08) | (1, 0x09) => {
                for k in 0..n {
                    let s = si(gl(a, es, k), es);
                    let t = match (u, opc) {
                        (0, 0x08) => s > 0,
                        (0, 0x09) => s == 0,
                        (0, 0x0A) => s < 0,
                        (1, 0x08) => s >= 0,
                        _ => s <= 0,
                    };
                    put(&mut res, es, k, if t { m(es) } else { 0 });
                }
            }
            (0, 0x0B) | (1, 0x0B) => {
                for k in 0..n {
                    let s = si(gl(a, es, k), es);
                    let r = if u == 0 { s.wrapping_abs() } else { s.wrapping_neg() };
                    put(&mut res, es, k, r as u64);
                }
            }
            // SADDLP/UADDLP/SADALP/UADALP
            (_, 0x02) | (_, 0x06) => {
                let wide = es * 2;
                for k in 0..bits / wide {
                    let (x, y) = (gl(a, es, 2 * k), gl(a, es, 2 * k + 1));
                    let s = if u == 0 { si(x, es) as i128 + si(y, es) as i128 } else { x as i128 + y as i128 };
                    let acc = if opc == 0x06 { gl(d, wide, k) } else { 0 };
                    put(&mut res, wide, k, acc.wrapping_add(s as u64));
                }
            }
            // XTN / SQXTN / UQXTN / SQXTUN
            (0, 0x12) | (1, 0x12) | (0, 0x14) | (1, 0x14) => {
                if size == 3 {
                    return None;
                }
                let wide = es * 2;
                let mut out = if q { d & u64::MAX as u128 } else { 0 };
                for k in 0..64 / es {
                    let x = gl(a, wide, k);
                    let r = match (u, opc) {
                        (0, 0x12) => x & m(es),
                        (1, 0x12) => sat_u(si(x, wide) as i128, es),
                        (0, 0x14) => sat_s(si(x, wide) as i128, es),
                        _ => sat_u(x as i128, es),
                    };
                    put(&mut out, es, k + if q { 64 / es } else { 0 }, r);
                }
                self.vr[rd] = out;
                return Some(());
            }
            // SHLL / SHLL2: ensancha y desplaza por el tamaño del elemento
            (1, 0x13) => {
                let off = if q { 64 / es } else { 0 };
                for k in 0..64 / es {
                    put(&mut res, es * 2, k, gl(a, es, k + off) << es);
                }
                self.vr[rd] = res;
                return Some(());
            }
            _ => return None,
        }
        self.vr[rd] = res & dmask(q);
        Some(())
    }

    // ---------------------------------------------------------------- a través de carriles
    fn across_lanes(&mut self, i: u32, q: bool) -> Option<()> {
        let (rd, rn) = ((i & 31) as usize, ((i >> 5) & 31) as usize);
        let (u, size, opc) = ((i >> 29) & 1, (i >> 22) & 3, (i >> 12) & 0x1F);
        let a = self.vr[rn];
        let bits = if q { 128 } else { 64 };
        if matches!(opc, 0x0C | 0x0F) && u == 1 {
            // FMAXNMV / FMAXV / FMINNMV / FMINV (solo f32 de 4 carriles)
            if size & 1 == 1 || !q {
                return None;
            }
            let aflag = size >> 1;
            let num = opc == 0x0C;
            let mut r = f32::from_bits(gl(a, 32, 0) as u32);
            for k in 1..4 {
                let x = f32::from_bits(gl(a, 32, k) as u32);
                r = if aflag == 0 { fmax(r, x, num) } else { fmin(r, x, num) };
            }
            self.vr[rd] = r.to_bits() as u128;
            return Some(());
        }
        let es = 8 << size;
        if size == 3 {
            return None;
        }
        let n = bits / es;
        let lane = |k: u32| gl(a, es, k);
        let r: (u64, u32) = match (u, opc) {
            (_, 0x1B) if u == 0 => ((0..n).fold(0u64, |s, k| s.wrapping_add(lane(k))), es),
            (_, 0x03) => {
                let t: i128 = (0..n).map(|k| if u == 0 { si(lane(k), es) as i128 } else { lane(k) as i128 }).sum();
                (t as u64, es * 2)
            }
            (0, 0x0A) => ((0..n).map(|k| si(lane(k), es)).max().unwrap() as u64, es),
            (1, 0x0A) => ((0..n).map(lane).max().unwrap(), es),
            (0, 0x1A) => ((0..n).map(|k| si(lane(k), es)).min().unwrap() as u64, es),
            (1, 0x1A) => ((0..n).map(lane).min().unwrap(), es),
            _ => return None,
        };
        self.vr[rd] = (r.0 & m(r.1)) as u128;
        Some(())
    }

    // ------------------------------------------------------------------------- copia
    fn copy(&mut self, i: u32, q: bool) -> Option<()> {
        let (rd, rn) = ((i & 31) as usize, ((i >> 5) & 31) as usize);
        let (imm5, imm4, op) = ((i >> 16) & 31, (i >> 11) & 15, (i >> 29) & 1);
        let (es, idx): (u32, u32) = if imm5 & 1 == 1 { (8, imm5 >> 1) } else if imm5 & 2 != 0 { (16, imm5 >> 2) } else if imm5 & 4 != 0 { (32, imm5 >> 3) } else if imm5 & 8 != 0 { (64, imm5 >> 4) } else { return None };
        if op == 1 {
            // INS (elemento)
            let src = imm4 >> (es.trailing_zeros() - 3);
            let x = gl(self.vr[rn], es, src);
            let mut v = self.vr[rd];
            put(&mut v, es, idx, x);
            self.vr[rd] = v;
            return Some(());
        }
        match imm4 {
            0 | 1 => {
                let x = if imm4 == 0 { gl(self.vr[rn], es, idx) } else { self.rz(rn as u32) & m(es) };
                let mut r = 0u128;
                for k in 0..(if q { 128 } else { 64 }) / es {
                    put(&mut r, es, k, x);
                }
                self.vr[rd] = r;
            }
            3 => {
                let mut v = self.vr[rd];
                put(&mut v, es, idx, self.rz(rn as u32));
                self.vr[rd] = v;
            }
            5 => {
                let x = si(gl(self.vr[rn], es, idx), es) as u64;
                self.wz(rd as u32, if q { x } else { x & 0xFFFF_FFFF });
            }
            7 => {
                let x = gl(self.vr[rn], es, idx);
                self.wz(rd as u32, x);
            }
            _ => return None,
        }
        Some(())
    }

    // ----------------------------------------------------------------- permutaciones
    fn permute(&mut self, i: u32, q: bool) -> Option<()> {
        let (rd, rn, rm) = ((i & 31) as usize, ((i >> 5) & 31) as usize, ((i >> 16) & 31) as usize);
        let (size, opc) = ((i >> 22) & 3, (i >> 12) & 7);
        let es = 8 << size;
        let n = (if q { 128 } else { 64 }) / es;
        let (a, b) = (self.vr[rn], self.vr[rm]);
        let mut res = 0u128;
        let half = n / 2;
        for k in 0..n {
            let r = match opc {
                1 | 5 => {
                    // UZP1 / UZP2
                    let idx = 2 * (k % half.max(1)) + (opc >> 2);
                    if k < half { gl(a, es, idx) } else { gl(b, es, idx) }
                }
                2 | 6 => {
                    // TRN1 / TRN2
                    let base = (k & !1) + (opc >> 2);
                    if k & 1 == 0 { gl(a, es, base) } else { gl(b, es, base) }
                }
                3 | 7 => {
                    // ZIP1 / ZIP2
                    let src = k / 2 + if opc == 7 { half } else { 0 };
                    if k & 1 == 0 { gl(a, es, src) } else { gl(b, es, src) }
                }
                _ => return None,
            };
            put(&mut res, es, k, r);
        }
        self.vr[rd] = res;
        Some(())
    }

    fn extract(&mut self, i: u32, q: bool) -> Option<()> {
        let (rd, rn, rm) = ((i & 31) as usize, ((i >> 5) & 31) as usize, ((i >> 16) & 31) as usize);
        let idx = (i >> 11) & 15;
        let n = if q { 16 } else { 8 };
        if idx >= n {
            return None;
        }
        let (a, b) = (self.vr[rn], self.vr[rm]);
        let mut res = 0u128;
        for k in 0..n {
            let s = k + idx;
            let byte = if s < n { gl(a, 8, s) } else { gl(b, 8, s - n) };
            put(&mut res, 8, k, byte);
        }
        self.vr[rd] = res;
        Some(())
    }

    fn table_lookup(&mut self, i: u32, q: bool) -> Option<()> {
        let (rd, rn, rm) = ((i & 31) as usize, ((i >> 5) & 31) as usize, ((i >> 16) & 31) as usize);
        let (len, tbx) = (((i >> 13) & 3) + 1, (i >> 12) & 1 == 1);
        let idxv = self.vr[rm];
        let old = self.vr[rd];
        let mut res = 0u128;
        for k in 0..(if q { 16 } else { 8 }) {
            let ix = gl(idxv, 8, k) as u32;
            let r = if ix < 16 * len {
                gl(self.vr[(rn + (ix / 16) as usize) % 32], 8, ix % 16)
            } else if tbx {
                gl(old, 8, k)
            } else {
                0
            };
            put(&mut res, 8, k, r);
        }
        self.vr[rd] = res;
        Some(())
    }

    // ------------------------------------------------------------- inmediato modificado
    fn mod_imm(&mut self, i: u32, q: bool) -> Option<()> {
        let rd = (i & 31) as usize;
        let op = (i >> 29) & 1;
        let cmode = (i >> 12) & 0xF;
        let imm8 = (((i >> 16) & 7) << 5 | ((i >> 5) & 31)) as u64;
        let mut orr_bic = false;
        let mut invert = op == 1;
        let imm: u64 = match cmode {
            0 | 2 | 4 | 6 => rep(imm8 << (8 * (cmode >> 1)), 32),
            1 | 3 | 5 | 7 => {
                orr_bic = true;
                rep(imm8 << (8 * (cmode >> 1)), 32)
            }
            8 | 10 => rep(imm8 << (8 * ((cmode >> 1) & 1)), 16),
            9 | 11 => {
                orr_bic = true;
                rep(imm8 << (8 * ((cmode >> 1) & 1)), 16)
            }
            12 => rep((imm8 << 8) | 0xFF, 32),
            13 => rep((imm8 << 16) | 0xFFFF, 32),
            14 => {
                if op == 0 {
                    rep(imm8, 8)
                } else {
                    invert = false;
                    (0..8).fold(0u64, |acc, b| if imm8 >> b & 1 == 1 { acc | (0xFFu64 << (8 * b)) } else { acc })
                }
            }
            _ => {
                if op == 0 {
                    rep(expand_imm(imm8 as u32, false), 32)
                } else if q {
                    invert = false;
                    expand_imm(imm8 as u32, true)
                } else {
                    return None;
                }
            }
        };
        let full = (imm as u128) | ((imm as u128) << 64);
        let r = if orr_bic {
            if op == 0 { self.vr[rd] | full } else { self.vr[rd] & !full }
        } else if invert {
            !full
        } else {
            full
        };
        self.vr[rd] = r & dmask(q);
        Some(())
    }

    // ------------------------------------------------------------ desplazamiento inmediato
    fn shift_imm(&mut self, i: u32, q: bool) -> Option<()> {
        let (rd, rn) = ((i & 31) as usize, ((i >> 5) & 31) as usize);
        let u = (i >> 29) & 1;
        let immh = (i >> 19) & 15;
        let immhb = (i >> 16) & 0x7F;
        let opc = (i >> 11) & 0x1F;
        let hb = 31 - immh.leading_zeros();
        let es = 8u32 << hb;
        let (a, d) = (self.vr[rn], self.vr[rd]);
        let bits = if q { 128 } else { 64 };
        let mut res = 0u128;
        let rs = 2 * es - immhb; // desplazamiento a la derecha
        let ls = immhb.wrapping_sub(es); // a la izquierda
        let n = bits / es;
        match opc {
            0x00 | 0x02 | 0x04 | 0x06 => {
                if es == 64 && !q {
                    return None;
                }
                let round = opc >= 0x04;
                let acc = opc & 2 != 0;
                for k in 0..n {
                    let x = gl(a, es, k);
                    let v: i128 = if u == 0 { si(x, es) as i128 } else { x as i128 };
                    let rnd = if round { 1i128 << (rs - 1) } else { 0 };
                    let r = ((v + rnd) >> rs) as u64;
                    put(&mut res, es, k, if acc { gl(d, es, k).wrapping_add(r) } else { r });
                }
            }
            0x0A => {
                for k in 0..n {
                    let x = gl(a, es, k);
                    let r = if u == 0 { x << ls } else { (gl(d, es, k) & !(m(es) << ls)) | (x << ls) };
                    put(&mut res, es, k, r);
                }
            }
            0x08 if u == 1 => {
                for k in 0..n {
                    let x = gl(a, es, k);
                    let keep = !(m(es) >> rs) & m(es);
                    put(&mut res, es, k, (gl(d, es, k) & keep) | (x >> rs));
                }
            }
            0x0E => {
                for k in 0..n {
                    let x = gl(a, es, k);
                    let r = if u == 0 { sat_s((si(x, es) as i128) << ls, es) } else { sat_u((x as i128) << ls, es) };
                    put(&mut res, es, k, r);
                }
            }
            // SHRN / RSHRN: estrecha
            0x10 | 0x11 if u == 0 => {
                let wide = es * 2;
                let rs = 2 * es - immhb;
                let mut out = if q { d & u64::MAX as u128 } else { 0 };
                for k in 0..64 / es {
                    let x = gl(a, wide, k) as u128;
                    let rnd = if opc == 0x11 { 1u128 << (rs - 1) } else { 0 };
                    put(&mut out, es, k + if q { 64 / es } else { 0 }, ((x + rnd) >> rs) as u64);
                }
                self.vr[rd] = out;
                return Some(());
            }
            // SSHLL / USHLL: ensancha y desplaza
            0x14 => {
                let off = if q { 64 / es } else { 0 };
                for k in 0..64 / es {
                    let x = gl(a, es, k + off);
                    let v: i128 = if u == 0 { si(x, es) as i128 } else { x as i128 };
                    put(&mut res, es * 2, k, (v << ls) as u64);
                }
                self.vr[rd] = res;
                return Some(());
            }
            // SCVTF/UCVTF/FCVTZS/FCVTZU con coma fija
            0x1C | 0x1F => {
                if es < 32 || (es == 64 && !q) {
                    return None;
                }
                let k = (2f64).powi(rs as i32);
                for l in 0..n {
                    let x = gl(a, es, l);
                    let r = if opc == 0x1C {
                        if es == 64 {
                            (if u == 0 { si(x, 64) as f64 } else { x as f64 } / k).to_bits()
                        } else {
                            let v = if u == 0 { si(x, 32) as f32 } else { x as u32 as f32 };
                            ((v as f64 / k) as f32).to_bits() as u64
                        }
                    } else {
                        let v = if es == 64 { f64::from_bits(x) } else { f32::from_bits(x as u32) as f64 } * k;
                        match (u == 1, es) {
                            (false, 64) => v as i64 as u64,
                            (false, _) => v as i32 as u32 as u64,
                            (true, 64) => v as u64,
                            (true, _) => v as u32 as u64,
                        }
                    };
                    put(&mut res, es, l, r);
                }
            }
            _ => return None,
        }
        self.vr[rd] = res & dmask(q);
        Some(())
    }

    // ----------------------------------------------------------------- elemento indexado
    fn indexed(&mut self, i: u32, q: bool) -> Option<()> {
        let (rd, rn) = ((i & 31) as usize, ((i >> 5) & 31) as usize);
        let (u, size, opc) = ((i >> 29) & 1, (i >> 22) & 3, (i >> 12) & 0xF);
        let (l, mm, h) = ((i >> 21) & 1, (i >> 20) & 1, (i >> 11) & 1);
        let rm_lo = ((i >> 16) & 15) as usize;
        let (a, d) = (self.vr[rn], self.vr[rd]);
        let mut res = 0u128;

        if matches!(opc, 0x1 | 0x5 | 0x9) {
            // FMLA / FMLS / FMUL / FMULX por elemento
            let sz = size & 1 == 1;
            let (rm, idx) = if sz { (rm_lo | (mm as usize) << 4, h) } else { (rm_lo | (mm as usize) << 4, (h << 1) | l) };
            if size < 2 {
                return None; // media precisión
            }
            let es = if sz { 64 } else { 32 };
            if sz && !q {
                return None;
            }
            let y = gl(self.vr[rm], es, idx);
            for k in 0..(if q { 128 } else { 64 }) / es {
                let x = gl(a, es, k);
                let acc = gl(d, es, k);
                let aflag = if opc == 0x5 { 1 } else { 0 };
                let code = match opc { 0x9 => 0x1B, _ => 0x19 };
                let r = fp3_dispatch(sz, if opc == 0x9 { 0 } else { 0 }, aflag, code, x, y, acc)?;
                let _ = u;
                put(&mut res, es, k, r);
            }
            self.vr[rd] = res & dmask(q);
            return Some(());
        }

        let (rm, idx) = match size {
            1 => (rm_lo, (h << 2) | (l << 1) | mm),
            2 => (rm_lo | (mm as usize) << 4, (h << 1) | l),
            _ => return None,
        };
        let es = 8 << size;
        let y = gl(self.vr[rm], es, idx);
        let signed = u == 0;
        match opc {
            // MLA / MLS / MUL (entero)
            0x0 | 0x4 | 0x8 => {
                for k in 0..(if q { 128 } else { 64 }) / es {
                    let x = gl(a, es, k);
                    let p = x.wrapping_mul(y);
                    let acc = gl(d, es, k);
                    let r = match (u, opc) {
                        (1, 0x0) => acc.wrapping_add(p),
                        (1, 0x4) => acc.wrapping_sub(p),
                        (0, 0x8) => p,
                        _ => return None,
                    };
                    put(&mut res, es, k, r);
                }
            }
            // SMLAL / SMLSL / SMULL (y variantes sin signo)
            0x2 | 0x6 | 0xA => {
                let wide = es * 2;
                let off = if q { 64 / es } else { 0 };
                for k in 0..64 / es {
                    let x = gl(a, es, k + off);
                    let (xs, ys): (i128, i128) = if signed { (si(x, es) as i128, si(y, es) as i128) } else { (x as i128, y as i128) };
                    let p = (xs * ys) as u64;
                    let acc = gl(d, wide, k);
                    let r = match opc { 0x2 => acc.wrapping_add(p), 0x6 => acc.wrapping_sub(p), _ => p };
                    put(&mut res, wide, k, r);
                }
                self.vr[rd] = res;
                return Some(());
            }
            _ => return None,
        }
        self.vr[rd] = res & dmask(q);
        Some(())
    }

    // ------------------------------------------------------------------- formas escalares
    fn simd_scalar(&mut self, i: u32) -> Option<()> {
        let (rd, rn, rm) = ((i & 31) as usize, ((i >> 5) & 31) as usize, ((i >> 16) & 31) as usize);
        let u = (i >> 29) & 1;
        let size = (i >> 22) & 3;

        // dos registros, misceláneo
        if i & 0xDF3E_0C00 == 0x5E20_0800 {
            let opcode = (i >> 12) & 0x1F;
            let dbl = size & 1 == 1;
            let src = self.vr[rn] as u64;
            let r: u64 = match (u, opcode) {
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
                (0, 0x1B) | (1, 0x1B) | (0, 0x1A) | (1, 0x1A) | (0, 0x1C) | (1, 0x1C) => {
                    let v = if dbl { f64::from_bits(src) } else { f32::from_bits(src as u32) as f64 };
                    let aflag = size >> 1;
                    let mode = match (opcode, aflag) {
                        (0x1A, 0) => 0,
                        (0x1A, _) => 1,
                        (0x1B, 0) => 2,
                        (0x1B, _) => 3,
                        _ => 4,
                    };
                    let t = round_mode(v, mode);
                    match (u == 1, dbl) {
                        (false, true) => t as i64 as u64,
                        (false, false) => t as i32 as u32 as u64,
                        (true, true) => t as u64,
                        (true, false) => t as u32 as u64,
                    }
                }
                (0, 0x08) | (0, 0x09) | (0, 0x0A) | (1, 0x08) | (1, 0x09) if size == 3 => {
                    let s = src as i64;
                    let t = match (u, opcode) {
                        (0, 0x08) => s > 0,
                        (0, 0x09) => s == 0,
                        (0, 0x0A) => s < 0,
                        (1, 0x08) => s >= 0,
                        _ => s <= 0,
                    };
                    if t { u64::MAX } else { 0 }
                }
                (0, 0x0B) if size == 3 => (src as i64).wrapping_abs() as u64,
                (1, 0x0B) if size == 3 => (src as i64).wrapping_neg() as u64,
                _ => return None,
            };
            self.vr[rd] = r as u128;
            return Some(());
        }

        // tres iguales (escalar)
        if i & 0xDF20_0400 == 0x5E20_0400 {
            let opc = (i >> 11) & 0x1F;
            let (x, y) = (self.vr[rn] as u64, self.vr[rm] as u64);
            let r = if opc >= 0x18 {
                let sz = size & 1 == 1;
                fp3_dispatch(sz, u, size >> 1, opc, x, y, self.vr[rd] as u64)?
            } else if size == 3 {
                int3(u, opc, 64, x, y, 0)?
            } else {
                return None;
            };
            self.vr[rd] = r as u128;
            return Some(());
        }

        // por pares (escalar): ADDP, FADDP, FMAXP…
        if i & 0xDF3E_0C00 == 0x5E30_0800 {
            let opc = (i >> 12) & 0x1F;
            let a = self.vr[rn];
            let r = match (u, opc) {
                (0, 0x1B) if size == 3 => gl(a, 64, 0).wrapping_add(gl(a, 64, 1)),
                (1, 0x0D) | (1, 0x0C) | (1, 0x0F) => {
                    let aflag = size >> 1;
                    let (es, sz) = if size & 1 == 1 { (64, true) } else { (32, false) };
                    let (x, y) = (gl(a, es, 0), gl(a, es, 1));
                    let (num, op) = match (opc, aflag) {
                        (0x0D, _) => (false, 0),
                        (0x0C, 0) | (0x0F, 0) => (opc == 0x0C, 1),
                        _ => (opc == 0x0C, 2),
                    };
                    macro_rules! pr { ($t:ty) => {{ let (p, q2) = (<$t>::from_bits64(x), <$t>::from_bits64(y)); match op { 0 => (p + q2).bits64(), 1 => fmax(p, q2, num).bits64(), _ => fmin(p, q2, num).bits64() } }}; }
                    if sz { pr!(f64) } else { pr!(f32) }
                }
                _ => return None,
            };
            self.vr[rd] = r as u128;
            return Some(());
        }

        // copia (DUP elemento escalar)
        if i & 0xDFE0_8400 == 0x5E00_0400 && (i >> 11) & 15 == 0 {
            let imm5 = (i >> 16) & 31;
            let (es, idx): (u32, u32) = if imm5 & 1 == 1 { (8, imm5 >> 1) } else if imm5 & 2 != 0 { (16, imm5 >> 2) } else if imm5 & 4 != 0 { (32, imm5 >> 3) } else if imm5 & 8 != 0 { (64, imm5 >> 4) } else { return None };
            self.vr[rd] = gl(self.vr[rn], es, idx) as u128;
            return Some(());
        }

        None
    }

    fn simd_scalar_shift_indexed(&mut self, i: u32) -> Option<()> {
        let (rd, rn) = ((i & 31) as usize, ((i >> 5) & 31) as usize);
        let u = (i >> 29) & 1;
        // desplazamiento inmediato escalar (solo D)
        if i & 0xDF80_0400 == 0x5F00_0400 {
            let immh = (i >> 19) & 15;
            let immhb = (i >> 16) & 0x7F;
            let opc = (i >> 11) & 0x1F;
            let a = self.vr[rn] as u64;
            let d = self.vr[rd] as u64;
            if immh & 8 != 0 {
                let rs = 128 - immhb;
                let ls = immhb - 64;
                let r = match (u, opc) {
                    (0, 0x00) => ((a as i64) >> rs.min(63)) as u64,
                    (1, 0x00) => if rs >= 64 { 0 } else { a >> rs },
                    (0, 0x02) => d.wrapping_add(((a as i64) >> rs.min(63)) as u64),
                    (1, 0x02) => d.wrapping_add(if rs >= 64 { 0 } else { a >> rs }),
                    (0, 0x04) => (((a as i64 as i128) + (1i128 << (rs - 1))) >> rs) as u64,
                    (1, 0x04) => (((a as i128) + (1i128 << (rs - 1))) >> rs) as u64,
                    (0, 0x0A) => a << ls,
                    (1, 0x0A) => (d & !(u64::MAX << ls)) | (a << ls),
                    (1, 0x08) => (d & !(u64::MAX >> rs.min(63)).wrapping_add(0)) | (if rs >= 64 { 0 } else { a >> rs }),
                    _ => return None,
                };
                self.vr[rd] = r as u128;
                return Some(());
            }
            // conversiones de coma fija escalares (S o D)
            let es = if immh & 4 != 0 { 32 } else { return None };
            let rs = 2 * es - immhb;
            let k = (2f64).powi(rs as i32);
            let r = match (u, opc) {
                (0, 0x1C) => ((a as u32 as i32 as f64 / k) as f32).to_bits() as u64,
                (1, 0x1C) => ((a as u32 as f64 / k) as f32).to_bits() as u64,
                (0, 0x1F) => (f32::from_bits(a as u32) as f64 * k) as i32 as u32 as u64,
                (1, 0x1F) => (f32::from_bits(a as u32) as f64 * k) as u32 as u64,
                _ => return None,
            };
            self.vr[rd] = r as u128;
            return Some(());
        }
        // elemento indexado escalar: FMUL / FMLA / FMLS / FMULX
        if i & 0xDF00_0400 == 0x5F00_0000 {
            let (size, opc) = ((i >> 22) & 3, (i >> 12) & 0xF);
            let (l, mm, h) = ((i >> 21) & 1, (i >> 20) & 1, (i >> 11) & 1);
            let sz = size & 1 == 1;
            if size < 2 || !matches!(opc, 0x1 | 0x5 | 0x9) {
                return None;
            }
            let rm = (((i >> 16) & 15) | (mm << 4)) as usize;
            let (es, idx) = if sz { (64, h) } else { (32, (h << 1) | l) };
            let y = gl(self.vr[rm], es, idx);
            let x = gl(self.vr[rn], es, 0);
            let acc = gl(self.vr[rd], es, 0);
            let aflag = if opc == 0x5 { 1 } else { 0 };
            let code = if opc == 0x9 { 0x1B } else { 0x19 };
            let r = fp3_dispatch(sz, 0, aflag, code, x, y, acc)?;
            self.vr[rd] = r as u128;
            return Some(());
        }
        None
    }
}

impl Cpu {
    /// `LD1`–`LD4` / `ST1`–`ST4` (varios registros o un solo carril) y `LD1R`–`LD4R`, con o sin post-incremento.
    pub(super) fn ldst_struct(&mut self, i: u32, mem: &mut crate::memory::Memory) -> Option<super::Res> {
        let q = (i >> 30) & 1;
        let (l, post, single) = ((i >> 22) & 1 == 1, (i >> 23) & 1 == 1, (i >> 24) & 1 == 1);
        let (rt, rn, rm) = (i & 31, (i >> 5) & 31, (i >> 16) & 31);
        let size = (i >> 10) & 3;
        let base = self.rs(rn);
        let read = |mem: &crate::memory::Memory, addr: u64, bytes: u32| mem.read_uint(addr, bytes as usize);
        let total: u64;
        let result: super::Res = if !single {
            let opcode = (i >> 12) & 0xF;
            let (nregs, interleave) = match opcode {
                0 => (4, true),
                2 => (4, false),
                4 => (3, true),
                6 => (3, false),
                7 => (1, false),
                8 => (2, true),
                10 => (2, false),
                _ => return None,
            };
            let bytes = 1u32 << size;
            let dbytes = if q == 1 { 16 } else { 8 };
            let n = dbytes / bytes;
            total = (nregs * dbytes) as u64;
            (|| -> super::Res {
                for r in 0..nregs {
                    let reg = ((rt + r) % 32) as usize;
                    let mut val = if l { 0u128 } else { self.vr[reg] };
                    for k in 0..n {
                        let off = if interleave { (k * nregs + r) * bytes } else { r * dbytes + k * bytes } as u64;
                        let addr = base.wrapping_add(off);
                        if l {
                            let x = read(mem, addr, bytes)?;
                            put(&mut val, bytes * 8, k, x);
                        } else {
                            mem.write_uint(addr, bytes as usize, gl(val, bytes * 8, k))?;
                        }
                    }
                    if l {
                        self.vr[reg] = val;
                    }
                }
                Ok(())
            })()
        } else {
            let (r, opcode, s) = ((i >> 21) & 1, (i >> 13) & 7, (i >> 12) & 1);
            let selem = (((opcode & 1) << 1) | r) + 1;
            let scale = opcode >> 1;
            let (ebytes, idx, replicate) = match scale {
                0 => (1u32, (q << 3) | (s << 2) | size, false),
                1 => {
                    if size & 1 != 0 {
                        return None;
                    }
                    (2, (q << 2) | (s << 1) | (size >> 1), false)
                }
                2 => {
                    if size & 1 == 0 {
                        (4, (q << 1) | s, false)
                    } else {
                        if s != 0 {
                            return None;
                        }
                        (8, q, false)
                    }
                }
                _ => {
                    if !l || s != 0 {
                        return None;
                    }
                    (1u32 << size, 0, true)
                }
            };
            total = (selem * ebytes) as u64;
            (|| -> super::Res {
                for k in 0..selem {
                    let reg = ((rt + k) % 32) as usize;
                    let addr = base.wrapping_add((k * ebytes) as u64);
                    if replicate {
                        let x = read(mem, addr, ebytes)?;
                        let mut v = 0u128;
                        for e in 0..(if q == 1 { 128 } else { 64 }) / (ebytes * 8) {
                            put(&mut v, ebytes * 8, e, x);
                        }
                        self.vr[reg] = v;
                    } else if l {
                        let x = read(mem, addr, ebytes)?;
                        let mut v = self.vr[reg];
                        put(&mut v, ebytes * 8, idx, x);
                        self.vr[reg] = v;
                    } else {
                        mem.write_uint(addr, ebytes as usize, gl(self.vr[reg], ebytes * 8, idx))?;
                    }
                }
                Ok(())
            })()
        };
        if result.is_ok() && post {
            let inc = if rm == 31 { total } else { self.rz(rm) };
            self.ws(rn, base.wrapping_add(inc));
        }
        Some(result)
    }
}

/// Operaciones vectoriales de dos registros sobre un carril de coma flotante.
fn fp_misc(u: u32, a: u32, opc: u32, sz: bool, x: u64) -> Option<u64> {
    macro_rules! go {
        ($t:ty, $ones:expr) => {{
            let v = <$t>::from_bits64(x);
            let zero = <$t>::from_bits64(0);
            let sel = |b: bool| if b { $ones } else { 0 };
            let rnd = |mode: u32| <$t>::from_f64(round_mode(v.to_f64(), mode)).bits64();
            let cvt = |mode: u32, unsigned: bool| -> u64 {
                let t = round_mode(v.to_f64(), mode);
                if sz {
                    if unsigned { t as u64 } else { t as i64 as u64 }
                } else if unsigned {
                    t as u32 as u64
                } else {
                    t as i32 as u32 as u64
                }
            };
            Some(match (u, opc, a) {
                (0, 0x0C, 1) => sel(v > zero),
                (0, 0x0D, 1) => sel(v == zero),
                (0, 0x0E, 1) => sel(v < zero),
                (0, 0x0F, 1) => v.abs_().bits64(),
                (1, 0x0C, 1) => sel(v >= zero),
                (1, 0x0D, 1) => sel(v <= zero),
                (1, 0x0F, 1) => (-v).bits64(),
                (1, 0x1F, 1) => v.sqrt_().bits64(),
                (0, 0x18, 0) => rnd(0),
                (0, 0x18, 1) => rnd(1),
                (0, 0x19, 0) => rnd(2),
                (0, 0x19, 1) => rnd(3),
                (1, 0x18, 0) => rnd(4),
                (1, 0x19, _) => rnd(0),
                (0, 0x1A, 0) => cvt(0, false),
                (0, 0x1A, 1) => cvt(1, false),
                (1, 0x1A, 0) => cvt(0, true),
                (1, 0x1A, 1) => cvt(1, true),
                (0, 0x1B, 0) => cvt(2, false),
                (0, 0x1B, 1) => cvt(3, false),
                (1, 0x1B, 0) => cvt(2, true),
                (1, 0x1B, 1) => cvt(3, true),
                (0, 0x1C, 0) => cvt(4, false),
                (1, 0x1C, 0) => cvt(4, true),
                (0, 0x1D, 0) => {
                    // SCVTF: el carril contiene un entero con signo
                    if sz { (x as i64 as f64).to_bits() } else { (x as u32 as i32 as f32).to_bits() as u64 }
                }
                (1, 0x1D, 0) => {
                    if sz { (x as f64).to_bits() } else { (x as u32 as f32).to_bits() as u64 }
                }
                _ => return None,
            })
        }};
    }
    if sz { go!(f64, u64::MAX) } else { go!(f32, 0xFFFF_FFFFu64) }
}
