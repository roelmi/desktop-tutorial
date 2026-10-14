//! Intérprete de shaders en CPU. Sirve de referencia y es lo que usa el rasterizador por software.

use crate::ir::*;

/// Acceso a texturas desde un shader (`TEX`).
pub trait TextureSampler {
    fn sample(&self, unit: u8, u: f32, v: f32) -> [f32; 4];
}

pub struct NoTextures;
impl TextureSampler for NoTextures {
    fn sample(&self, _: u8, _: f32, _: f32) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }
}

fn read(op: &Operand, temps: &[[f32; 4]], inputs: &[[f32; 4]], consts: &[[f32; 4]]) -> [f32; 4] {
    let base = match op.src {
        Src::Temp(r) => temps.get(r as usize).copied().unwrap_or([0.0; 4]),
        Src::Input(r) => inputs.get(r as usize).copied().unwrap_or([0.0, 0.0, 0.0, 1.0]),
        Src::Const(r) => consts.get(r as usize).copied().unwrap_or([0.0; 4]),
        Src::Imm(v) => v,
    };
    let mut v = [base[op.swizzle[0] as usize], base[op.swizzle[1] as usize], base[op.swizzle[2] as usize], base[op.swizzle[3] as usize]];
    for c in &mut v {
        if op.abs {
            *c = c.abs();
        }
        if op.neg {
            *c = -*c;
        }
    }
    v
}

fn map2(a: [f32; 4], b: [f32; 4], f: impl Fn(f32, f32) -> f32) -> [f32; 4] {
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), f(a[3], b[3])]
}

/// Ejecuta el shader y devuelve las salidas `o0…`. `inputs` son los registros `i0…`.
pub fn run(shader: &Shader, inputs: &[[f32; 4]], consts: &[[f32; 4]], tex: &dyn TextureSampler) -> Vec<[f32; 4]> {
    let mut temps = vec![[0.0f32; 4]; shader.num_temps as usize];
    let mut outs = vec![[0.0f32; 4]; shader.num_outputs.max(1) as usize];
    for inst in &shader.code {
        let s: Vec<[f32; 4]> = (0..3).map(|k| read(&inst.srcs[k], &temps, inputs, consts)).collect();
        let (a, b, c) = (s[0], s[1], s[2]);
        let rep = |x: f32| [x; 4];
        let mut r = match inst.op {
            Op::Mov => a,
            Op::Add => map2(a, b, |x, y| x + y),
            Op::Sub => map2(a, b, |x, y| x - y),
            Op::Mul => map2(a, b, |x, y| x * y),
            Op::Mad => [a[0] * b[0] + c[0], a[1] * b[1] + c[1], a[2] * b[2] + c[2], a[3] * b[3] + c[3]],
            Op::Dp3 => rep(a[0] * b[0] + a[1] * b[1] + a[2] * b[2]),
            Op::Dp4 => rep(a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]),
            Op::Rcp => rep(1.0 / a[0]),
            Op::Rsq => rep(1.0 / a[0].abs().sqrt()),
            Op::Min => map2(a, b, f32::min),
            Op::Max => map2(a, b, f32::max),
            Op::Flr => [a[0].floor(), a[1].floor(), a[2].floor(), a[3].floor()],
            Op::Frc => [a[0] - a[0].floor(), a[1] - a[1].floor(), a[2] - a[2].floor(), a[3] - a[3].floor()],
            Op::Slt => map2(a, b, |x, y| if x < y { 1.0 } else { 0.0 }),
            Op::Sge => map2(a, b, |x, y| if x >= y { 1.0 } else { 0.0 }),
            Op::Cmp => [
                if a[0] < 0.0 { b[0] } else { c[0] },
                if a[1] < 0.0 { b[1] } else { c[1] },
                if a[2] < 0.0 { b[2] } else { c[2] },
                if a[3] < 0.0 { b[3] } else { c[3] },
            ],
            Op::Ex2 => rep(a[0].exp2()),
            Op::Lg2 => rep(a[0].abs().log2()),
            Op::Tex => tex.sample(inst.tex, a[0], a[1]),
        };
        if inst.saturate {
            for x in &mut r {
                *x = x.clamp(0.0, 1.0);
            }
        }
        let dst = match inst.dst.kind {
            DstKind::Temp(i) => &mut temps[i as usize],
            DstKind::Output(i) => &mut outs[i as usize],
        };
        for k in 0..4 {
            if inst.dst.mask >> k & 1 == 1 {
                dst[k] = r[k];
            }
        }
    }
    outs
}
