//! Lenguaje intermedio de shaders: registros `vec4<f32>` con instrucciones al estilo de los ensambladores de
//! shaders clásicos (MOV, ADD, MAD, DP4, RCP, TEX…). Es lo que traduciría un decodificador de shaders binarios.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Vertex,
    Fragment,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Src {
    /// Registro temporal `rN`.
    Temp(u8),
    /// Entrada `iN` (atributo de vértice o varying; en fragmento `i0` es la posición de ventana).
    Input(u8),
    /// Constante `cN` (vec4 del búfer de constantes).
    Const(u8),
    /// Literal.
    Imm([f32; 4]),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Operand {
    pub src: Src,
    pub swizzle: [u8; 4],
    pub neg: bool,
    pub abs: bool,
}

impl Operand {
    pub fn new(src: Src) -> Self {
        Self { src, swizzle: [0, 1, 2, 3], neg: false, abs: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DstKind {
    Temp(u8),
    /// Salida `oN`: en vértice `o0` es la posición y `o1…` los varyings; en fragmento `o0` es el color.
    Output(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dst {
    pub kind: DstKind,
    /// Máscara de escritura: bit 0 = x, 1 = y, 2 = z, 3 = w.
    pub mask: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Mov,
    Add,
    Sub,
    Mul,
    Mad,
    Dp3,
    Dp4,
    Rcp,
    Rsq,
    Min,
    Max,
    Flr,
    Frc,
    Slt,
    Sge,
    /// `dst = (a < 0) ? b : c`
    Cmp,
    Ex2,
    Lg2,
    /// Muestreo de textura 2D: `TEX dst, coord, tN`.
    Tex,
}

impl Op {
    pub fn num_srcs(self) -> usize {
        match self {
            Op::Mov | Op::Rcp | Op::Rsq | Op::Flr | Op::Frc | Op::Ex2 | Op::Lg2 | Op::Tex => 1,
            Op::Add | Op::Sub | Op::Mul | Op::Dp3 | Op::Dp4 | Op::Min | Op::Max | Op::Slt | Op::Sge => 2,
            Op::Mad | Op::Cmp => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Inst {
    pub op: Op,
    pub dst: Dst,
    pub srcs: [Operand; 3],
    pub saturate: bool,
    /// Unidad de textura (solo `Tex`).
    pub tex: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shader {
    pub stage: Stage,
    pub num_temps: u8,
    pub num_inputs: u8,
    pub num_outputs: u8,
    pub num_consts: u8,
    pub num_textures: u8,
    pub code: Vec<Inst>,
}

impl Shader {
    /// Serialización binaria simple (para guardar shaders en memoria de GPU).
    pub fn to_bytes(&self) -> Vec<u8> {
        crate::asm::disassemble(self).into_bytes()
    }

    pub fn from_bytes(stage: Stage, b: &[u8]) -> Result<Self, String> {
        crate::asm::assemble(stage, std::str::from_utf8(b).map_err(|e| e.to_string())?)
    }
}
