//! Traducción IR → WGSL → (validación con naga) → SPIR-V para Vulkan y MSL para Metal.

use crate::ir::*;
use std::fmt::Write;

pub struct Translated {
    pub wgsl: String,
    /// Palabras SPIR-V del punto de entrada (`vs_main` o `fs_main`).
    pub spirv: Vec<u32>,
    pub msl: String,
}

fn f(x: f32) -> Result<String, String> {
    if !x.is_finite() {
        return Err(format!("literal no finito {x}"));
    }
    Ok(format!("{x:?}"))
}

fn operand(o: &Operand, stage: Stage) -> Result<String, String> {
    let sw: String = o.swizzle.iter().map(|c| ['x', 'y', 'z', 'w'][*c as usize]).collect();
    let base = match o.src {
        Src::Temp(r) => format!("r{r}"),
        Src::Input(0) if stage == Stage::Fragment => "inp_pos".to_string(),
        Src::Input(r) => format!("inp.i{r}"),
        Src::Const(r) => format!("consts.c[{r}]"),
        Src::Imm(v) => format!("vec4<f32>({}, {}, {}, {})", f(v[0])?, f(v[1])?, f(v[2])?, f(v[3])?),
    };
    let mut e = format!("({base}).{sw}");
    if o.abs {
        e = format!("abs({e})");
    }
    if o.neg {
        e = format!("(-{e})");
    }
    Ok(e)
}

pub fn to_wgsl(s: &Shader) -> Result<String, String> {
    let mut w = String::new();
    let stage = s.stage;
    if s.num_consts > 0 {
        writeln!(w, "struct Consts {{ c: array<vec4<f32>, {}> }};\n@group(0) @binding(0) var<uniform> consts: Consts;", s.num_consts).unwrap();
    }
    if stage == Stage::Fragment {
        for t in 0..s.num_textures {
            writeln!(w, "@group(1) @binding({}) var tex{t}: texture_2d<f32>;\n@group(1) @binding({}) var smp{t}: sampler;", 2 * t, 2 * t + 1).unwrap();
        }
    } else {
        for t in 0..s.num_textures {
            writeln!(w, "@group(1) @binding({}) var tex{t}: texture_2d<f32>;\n@group(1) @binding({}) var smp{t}: sampler;", 2 * t, 2 * t + 1).unwrap();
        }
    }
    let ninputs = s.num_inputs;
    let noutputs = s.num_outputs.max(1);
    match stage {
        Stage::Vertex => {
            if ninputs > 0 {
                write!(w, "struct VsIn {{").unwrap();
                for i in 0..ninputs {
                    write!(w, " @location({i}) i{i}: vec4<f32>,").unwrap();
                }
                writeln!(w, " }};").unwrap();
            }
            write!(w, "struct VsOut {{ @builtin(position) o0: vec4<f32>,").unwrap();
            for o in 1..noutputs {
                write!(w, " @location({o}) o{o}: vec4<f32>,").unwrap();
            }
            writeln!(w, " }};").unwrap();
            if ninputs > 0 {
                writeln!(w, "@vertex fn vs_main(inp: VsIn) -> VsOut {{").unwrap();
            } else {
                writeln!(w, "@vertex fn vs_main() -> VsOut {{").unwrap();
            }
            writeln!(w, "  var outp: VsOut;\n  outp.o0 = vec4<f32>(0.0);").unwrap();
            for o in 1..noutputs {
                writeln!(w, "  outp.o{o} = vec4<f32>(0.0);").unwrap();
            }
        }
        Stage::Fragment => {
            if ninputs > 1 {
                write!(w, "struct FsIn {{").unwrap();
                for i in 1..ninputs {
                    write!(w, " @location({i}) i{i}: vec4<f32>,").unwrap();
                }
                writeln!(w, " }};").unwrap();
            }
            let mut params = String::from("@builtin(position) inp_pos: vec4<f32>");
            if ninputs > 1 {
                params += ", inp: FsIn";
            }
            writeln!(w, "@fragment fn fs_main({params}) -> @location(0) vec4<f32> {{\n  var outp: vec4<f32> = vec4<f32>(0.0);").unwrap();
        }
    }
    for r in 0..s.num_temps {
        writeln!(w, "  var r{r}: vec4<f32> = vec4<f32>(0.0);").unwrap();
    }
    for inst in &s.code {
        let a = |k: usize| operand(&inst.srcs[k], stage);
        let expr = match inst.op {
            Op::Mov => a(0)?,
            Op::Add => format!("{} + {}", a(0)?, a(1)?),
            Op::Sub => format!("{} - {}", a(0)?, a(1)?),
            Op::Mul => format!("{} * {}", a(0)?, a(1)?),
            Op::Mad => format!("{} * {} + {}", a(0)?, a(1)?, a(2)?),
            Op::Dp3 => format!("vec4<f32>(dot(({}).xyz, ({}).xyz))", a(0)?, a(1)?),
            Op::Dp4 => format!("vec4<f32>(dot({}, {}))", a(0)?, a(1)?),
            Op::Rcp => format!("vec4<f32>(1.0 / ({}).x)", a(0)?),
            Op::Rsq => format!("vec4<f32>(inverseSqrt(abs(({}).x)))", a(0)?),
            Op::Min => format!("min({}, {})", a(0)?, a(1)?),
            Op::Max => format!("max({}, {})", a(0)?, a(1)?),
            Op::Flr => format!("floor({})", a(0)?),
            Op::Frc => format!("fract({})", a(0)?),
            Op::Slt => format!("select(vec4<f32>(0.0), vec4<f32>(1.0), {} < {})", a(0)?, a(1)?),
            Op::Sge => format!("select(vec4<f32>(0.0), vec4<f32>(1.0), {} >= {})", a(0)?, a(1)?),
            Op::Cmp => format!("select({}, {}, {} < vec4<f32>(0.0))", a(2)?, a(1)?, a(0)?),
            Op::Ex2 => format!("vec4<f32>(exp2(({}).x))", a(0)?),
            Op::Lg2 => format!("vec4<f32>(log2(abs(({}).x)))", a(0)?),
            Op::Tex => {
                let uv = format!("({}).xy", a(0)?);
                if stage == Stage::Vertex {
                    format!("textureSampleLevel(tex{0}, smp{0}, {uv}, 0.0)", inst.tex)
                } else {
                    format!("textureSample(tex{0}, smp{0}, {uv})", inst.tex)
                }
            }
        };
        let expr = if inst.saturate { format!("clamp({expr}, vec4<f32>(0.0), vec4<f32>(1.0))") } else { expr };
        let target = match (inst.dst.kind, stage) {
            (DstKind::Temp(r), _) => format!("r{r}"),
            (DstKind::Output(o), Stage::Vertex) => format!("outp.o{o}"),
            (DstKind::Output(0), Stage::Fragment) => "outp".to_string(),
            (DstKind::Output(o), Stage::Fragment) => return Err(format!("el fragment shader solo admite la salida o0 (usa o{o})")),
        };
        write!(w, "  {{ let v: vec4<f32> = {expr};").unwrap();
        if inst.dst.mask == 0xF {
            write!(w, " {target} = v;").unwrap();
        } else {
            for (k, c) in ['x', 'y', 'z', 'w'].iter().enumerate() {
                if inst.dst.mask >> k & 1 == 1 {
                    write!(w, " {target}.{c} = v.{c};").unwrap();
                }
            }
        }
        writeln!(w, " }}").unwrap();
    }
    writeln!(w, "  return outp;\n}}").unwrap();
    Ok(w)
}

/// Traduce el shader a SPIR-V y MSL, validando el módulo con naga.
pub fn translate(s: &Shader) -> Result<Translated, String> {
    let wgsl = to_wgsl(s)?;
    let module = naga::front::wgsl::parse_str(&wgsl).map_err(|e| format!("WGSL inválido: {}\n{wgsl}", e.emit_to_string(&wgsl)))?;
    let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all())
        .validate(&module)
        .map_err(|e| format!("naga: {e:?}\n{wgsl}"))?;
    let (stage, entry) = match s.stage {
        Stage::Vertex => (naga::ShaderStage::Vertex, "vs_main"),
        Stage::Fragment => (naga::ShaderStage::Fragment, "fs_main"),
    };
    let spirv = naga::back::spv::write_vec(
        &module,
        &info,
        &naga::back::spv::Options::default(),
        Some(&naga::back::spv::PipelineOptions { shader_stage: stage, entry_point: entry.to_string() }),
    )
    .map_err(|e| format!("SPIR-V: {e:?}"))?;
    let (msl, _) = naga::back::msl::write_string(&module, &info, &naga::back::msl::Options::default(), &naga::back::msl::PipelineOptions::default())
        .map_err(|e| format!("MSL: {e:?}"))?;
    Ok(Translated { wgsl, spirv, msl })
}
