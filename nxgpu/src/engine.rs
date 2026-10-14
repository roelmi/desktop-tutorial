//! Motor 3D de juguete que consume llamadas a métodos del FIFO y dibuja con el rasterizador por software.
//!
//! IMPORTANTE: los números de registro de este motor son propios (clase [`CLASS_TOY3D`]) y NO coinciden con los
//! de los motores reales de NVIDIA (Maxwell/Ampere). Sirve para mostrar el recorrido completo
//! FIFO → motor → backend; sustituir la tabla de registros por la real exigiría documentación verificable.

use crate::fifo::MethodCall;
use crate::ir::{Shader, Stage};
use crate::raster::*;
use crate::GpuMemory;

pub const CLASS_TOY3D: u32 = 0xF3D0;

/// Registros (índices de palabra).
pub mod reg {
    pub const SET_OBJECT: u32 = 0x000;
    pub const RT_ADDR_HI: u32 = 0x010;
    pub const RT_ADDR_LO: u32 = 0x011;
    pub const RT_SIZE: u32 = 0x012; // ancho | alto << 16
    pub const VIEWPORT_X: u32 = 0x020; // 4 floats consecutivos: x, y, ancho, alto
    pub const CLEAR_COLOR: u32 = 0x030; // 4 floats consecutivos
    pub const CLEAR_DEPTH: u32 = 0x034;
    pub const CLEAR: u32 = 0x035; // bit 0 color, bit 1 profundidad
    pub const VB_SELECT: u32 = 0x040;
    pub const VB_ADDR_HI: u32 = 0x041;
    pub const VB_ADDR_LO: u32 = 0x042;
    pub const VB_SIZE: u32 = 0x043;
    pub const VB_STRIDE: u32 = 0x044;
    pub const ATTR_SELECT: u32 = 0x050;
    pub const ATTR_CONFIG: u32 = 0x051; // buffer (0-3) | offset (4-15) << 4 | formato (16-19) << 16
    pub const SHADER_SELECT: u32 = 0x060; // 0 = vértice, 1 = fragmento
    pub const SHADER_ADDR_HI: u32 = 0x061;
    pub const SHADER_ADDR_LO: u32 = 0x062;
    pub const SHADER_SIZE: u32 = 0x063; // al escribirlo se carga y ensambla el shader
    pub const CONST_SELECT: u32 = 0x070; // etapa << 8 | índice
    pub const CONST_DATA: u32 = 0x071; // 4 floats consecutivos; al escribir w se confirma
    pub const TEX_SELECT: u32 = 0x080;
    pub const TEX_ADDR_HI: u32 = 0x081;
    pub const TEX_ADDR_LO: u32 = 0x082;
    pub const TEX_SIZE: u32 = 0x083;
    pub const TEX_CONFIG: u32 = 0x084; // filtro (bit 0) | wrap (bits 1-2); confirma la textura
    pub const STATE_DEPTH: u32 = 0x090; // test (0) | escritura (1) | función (4-6)
    pub const STATE_BLEND: u32 = 0x091;
    pub const STATE_CULL: u32 = 0x092;
    pub const COLOR_MASK: u32 = 0x093;
    pub const TOPOLOGY: u32 = 0x0A0; // 0 lista, 1 tira, 2 abanico
    pub const DRAW_FIRST: u32 = 0x0A1;
    pub const DRAW_ARRAYS: u32 = 0x0A2; // arg = número de vértices
    pub const INDEX_ADDR_HI: u32 = 0x0A3;
    pub const INDEX_ADDR_LO: u32 = 0x0A4;
    pub const INDEX_FORMAT: u32 = 0x0A5; // 0 = u16, 1 = u32
    pub const DRAW_INDEXED: u32 = 0x0A6; // arg = número de índices
    pub const FLUSH: u32 = 0x0B0; // escribe el color en memoria (RGBA8)
}

#[derive(Default, Clone)]
struct Slot {
    addr_hi: u32,
    addr_lo: u32,
    size: u32,
    stride: u32,
}

fn addr(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | lo as u64
}

fn f(x: u32) -> f32 {
    f32::from_bits(x)
}

pub struct Toy3d {
    rt_addr: (u32, u32),
    rt: Option<RenderTarget>,
    viewport: [f32; 4],
    clear_color: [f32; 4],
    clear_depth: f32,
    vb_sel: usize,
    vbs: Vec<Slot>,
    attr_sel: usize,
    attrs: Vec<Option<VertexAttrib>>,
    shader_sel: usize,
    shader_slots: [Slot; 2],
    shaders: [Option<Shader>; 2],
    const_sel: (usize, usize),
    consts: [Vec<[f32; 4]>; 2],
    const_tmp: [f32; 4],
    tex_sel: usize,
    tex_slots: Vec<Slot>,
    textures: Vec<Option<Texture>>,
    depth: u32,
    blend: u32,
    cull: u32,
    color_mask: u32,
    topology: u32,
    first: u32,
    index_addr: (u32, u32),
    index_format: u32,
    /// Métodos no soportados (subcanal, método, argumento), para depuración.
    pub unknown: Vec<(u8, u32, u32)>,
    /// Errores de recursos (shader inválido, memoria ilegible…).
    pub errors: Vec<String>,
    pub draws: u64,
}

impl Default for Toy3d {
    fn default() -> Self {
        Self {
            rt_addr: (0, 0),
            rt: None,
            viewport: [0.0; 4],
            clear_color: [0.0; 4],
            clear_depth: 1.0,
            vb_sel: 0,
            vbs: vec![Slot::default(); 16],
            attr_sel: 0,
            attrs: vec![None; 16],
            shader_sel: 0,
            shader_slots: [Slot::default(), Slot::default()],
            shaders: [None, None],
            const_sel: (0, 0),
            consts: [vec![[0.0; 4]; 64], vec![[0.0; 4]; 64]],
            const_tmp: [0.0; 4],
            tex_sel: 0,
            tex_slots: vec![Slot::default(); 8],
            textures: vec![None; 8],
            depth: 0,
            blend: 0,
            cull: 0,
            color_mask: 0xF,
            topology: 0,
            first: 0,
            index_addr: (0, 0),
            index_format: 0,
            unknown: Vec::new(),
            errors: Vec::new(),
            draws: 0,
        }
    }
}

fn compare_from(v: u32) -> Compare {
    [Compare::Never, Compare::Less, Compare::Equal, Compare::LessEqual, Compare::Greater, Compare::NotEqual, Compare::GreaterEqual, Compare::Always][(v & 7) as usize]
}

fn factor_from(v: u32) -> BlendFactor {
    use BlendFactor::*;
    [Zero, One, SrcColor, OneMinusSrcColor, SrcAlpha, OneMinusSrcAlpha, DstColor, OneMinusDstColor, DstAlpha, OneMinusDstAlpha][(v as usize).min(9)]
}

fn op_from(v: u32) -> BlendOp {
    [BlendOp::Add, BlendOp::Subtract, BlendOp::ReverseSubtract, BlendOp::Min, BlendOp::Max][(v as usize).min(4)]
}

impl Toy3d {
    pub fn render_target(&self) -> Option<&RenderTarget> {
        self.rt.as_ref()
    }

    fn state(&self) -> State {
        let rt = self.rt.as_ref().unwrap();
        let mut st = State::new(rt.width, rt.height);
        if self.viewport[2] > 0.0 {
            st.viewport.x = self.viewport[0];
            st.viewport.y = self.viewport[1];
            st.viewport.width = self.viewport[2];
            st.viewport.height = self.viewport[3];
        }
        st.depth_test = self.depth & 1 == 1;
        st.depth_write = self.depth >> 1 & 1 == 1;
        st.depth_func = compare_from(self.depth >> 4);
        let b = self.blend;
        st.blend = Blend {
            enabled: b & 1 == 1,
            src_rgb: factor_from(b >> 1 & 15),
            dst_rgb: factor_from(b >> 5 & 15),
            op_rgb: op_from(b >> 9 & 7),
            src_a: factor_from(b >> 12 & 15),
            dst_a: factor_from(b >> 16 & 15),
            op_a: op_from(b >> 20 & 7),
        };
        st.cull = match self.cull {
            1 => CullMode::Back,
            2 => CullMode::Front,
            _ => CullMode::None,
        };
        st.color_mask = [self.color_mask & 1 != 0, self.color_mask & 2 != 0, self.color_mask & 4 != 0, self.color_mask & 8 != 0];
        st
    }

    fn read_region(&mut self, mem: &dyn GpuMemory, s: &Slot, what: &str) -> Option<Vec<u8>> {
        let mut buf = vec![0u8; s.size as usize];
        if mem.read(addr(s.addr_hi, s.addr_lo), &mut buf) {
            Some(buf)
        } else {
            self.errors.push(format!("{what}: no se pudo leer la memoria en {:#x}", addr(s.addr_hi, s.addr_lo)));
            None
        }
    }

    fn do_draw(&mut self, mem: &dyn GpuMemory, count: u32, indexed: bool) {
        let (Some(vs), Some(fs)) = (self.shaders[0].clone(), self.shaders[1].clone()) else {
            self.errors.push("dibujo sin shaders de vértice y fragmento".into());
            return;
        };
        if self.rt.is_none() {
            self.errors.push("dibujo sin destino de renderizado".into());
            return;
        }
        let slots = self.vbs.clone();
        let mut buffers = Vec::new();
        for s in &slots {
            let data = if s.size > 0 { self.read_region(mem, s, "búfer de vértices").unwrap_or_default() } else { Vec::new() };
            buffers.push(VertexBuffer { data, stride: s.stride });
        }
        let attribs: Vec<VertexAttrib> = self.attrs.iter().flatten().copied().collect();
        let indices: Option<Vec<u32>> = if indexed {
            let (w, base) = if self.index_format == 0 { (2usize, addr(self.index_addr.0, self.index_addr.1)) } else { (4usize, addr(self.index_addr.0, self.index_addr.1)) };
            let mut raw = vec![0u8; (self.first as usize + count as usize) * w];
            if !mem.read(base, &mut raw) {
                self.errors.push("índices ilegibles".into());
                return;
            }
            Some(raw.chunks_exact(w).map(|c| if w == 2 { u16::from_le_bytes([c[0], c[1]]) as u32 } else { u32::from_le_bytes(c.try_into().unwrap()) }).collect())
        } else {
            None
        };
        let state = self.state();
        let call = Draw {
            topology: match self.topology {
                1 => Topology::TriangleStrip,
                2 => Topology::TriangleFan,
                _ => Topology::TriangleList,
            },
            indices: indices.as_deref(),
            first: self.first,
            count,
            vs: &vs,
            fs: &fs,
            attribs: &attribs,
            buffers: &buffers,
            vs_consts: &self.consts[0],
            fs_consts: &self.consts[1],
            textures: &self.textures,
            state,
        };
        draw(self.rt.as_mut().unwrap(), &call);
        self.draws += 1;
    }

    fn method(&mut self, mem: &mut dyn GpuMemory, sub: u8, m: u32, a: u32) {
        match m {
            reg::RT_ADDR_HI => self.rt_addr.0 = a,
            reg::RT_ADDR_LO => self.rt_addr.1 = a,
            reg::RT_SIZE => self.rt = Some(RenderTarget::new(a & 0xFFFF, a >> 16)),
            m if (reg::VIEWPORT_X..reg::VIEWPORT_X + 4).contains(&m) => self.viewport[(m - reg::VIEWPORT_X) as usize] = f(a),
            m if (reg::CLEAR_COLOR..reg::CLEAR_COLOR + 4).contains(&m) => self.clear_color[(m - reg::CLEAR_COLOR) as usize] = f(a),
            reg::CLEAR_DEPTH => self.clear_depth = f(a),
            reg::CLEAR => {
                if let Some(rt) = self.rt.as_mut() {
                    if a & 1 != 0 {
                        rt.clear_color(self.clear_color);
                    }
                    if a & 2 != 0 {
                        rt.clear_depth(self.clear_depth);
                    }
                } else {
                    self.errors.push("clear sin destino de renderizado".into());
                }
            }
            reg::VB_SELECT => self.vb_sel = (a as usize).min(15),
            reg::VB_ADDR_HI => self.vbs[self.vb_sel].addr_hi = a,
            reg::VB_ADDR_LO => self.vbs[self.vb_sel].addr_lo = a,
            reg::VB_SIZE => self.vbs[self.vb_sel].size = a,
            reg::VB_STRIDE => self.vbs[self.vb_sel].stride = a,
            reg::ATTR_SELECT => self.attr_sel = (a as usize).min(15),
            reg::ATTR_CONFIG => {
                let format = match a >> 16 & 15 {
                    0 => AttribFormat::F32x1,
                    1 => AttribFormat::F32x2,
                    2 => AttribFormat::F32x3,
                    3 => AttribFormat::F32x4,
                    _ => AttribFormat::U8x4Norm,
                };
                self.attrs[self.attr_sel] = Some(VertexAttrib { location: self.attr_sel as u8, buffer: (a & 15) as u8, offset: (a >> 4) & 0xFFF, format });
            }
            reg::SHADER_SELECT => self.shader_sel = (a as usize).min(1),
            reg::SHADER_ADDR_HI => self.shader_slots[self.shader_sel].addr_hi = a,
            reg::SHADER_ADDR_LO => self.shader_slots[self.shader_sel].addr_lo = a,
            reg::SHADER_SIZE => {
                let sel = self.shader_sel;
                self.shader_slots[sel].size = a;
                let slot = self.shader_slots[sel].clone();
                if let Some(bytes) = self.read_region(mem, &slot, "shader") {
                    let stage = if sel == 0 { Stage::Vertex } else { Stage::Fragment };
                    match Shader::from_bytes(stage, &bytes) {
                        Ok(s) => self.shaders[sel] = Some(s),
                        Err(e) => self.errors.push(format!("shader inválido: {e}")),
                    }
                }
            }
            reg::CONST_SELECT => self.const_sel = (((a >> 8) & 1) as usize, (a & 63) as usize),
            m if (reg::CONST_DATA..reg::CONST_DATA + 4).contains(&m) => {
                let k = (m - reg::CONST_DATA) as usize;
                self.const_tmp[k] = f(a);
                if k == 3 {
                    let (stage, idx) = self.const_sel;
                    self.consts[stage][idx] = self.const_tmp;
                }
            }
            reg::TEX_SELECT => self.tex_sel = (a as usize).min(7),
            reg::TEX_ADDR_HI => self.tex_slots[self.tex_sel].addr_hi = a,
            reg::TEX_ADDR_LO => self.tex_slots[self.tex_sel].addr_lo = a,
            reg::TEX_SIZE => self.tex_slots[self.tex_sel].size = a,
            reg::TEX_CONFIG => {
                let slot = self.tex_slots[self.tex_sel].clone();
                let (w, h) = (slot.size & 0xFFFF, slot.size >> 16);
                let mut raw = vec![0u8; (w * h * 4) as usize];
                if mem.read(addr(slot.addr_hi, slot.addr_lo), &mut raw) {
                    self.textures[self.tex_sel] = Some(Texture {
                        width: w,
                        height: h,
                        rgba: raw.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect(),
                        filter: if a & 1 == 1 { Filter::Linear } else { Filter::Nearest },
                        wrap: match (a >> 1) & 3 {
                            1 => Wrap::Clamp,
                            2 => Wrap::Mirror,
                            _ => Wrap::Repeat,
                        },
                    });
                } else {
                    self.errors.push("textura ilegible".into());
                }
            }
            reg::STATE_DEPTH => self.depth = a,
            reg::STATE_BLEND => self.blend = a,
            reg::STATE_CULL => self.cull = a,
            reg::COLOR_MASK => self.color_mask = a,
            reg::TOPOLOGY => self.topology = a,
            reg::DRAW_FIRST => self.first = a,
            reg::DRAW_ARRAYS => self.do_draw(mem, a, false),
            reg::INDEX_ADDR_HI => self.index_addr.0 = a,
            reg::INDEX_ADDR_LO => self.index_addr.1 = a,
            reg::INDEX_FORMAT => self.index_format = a,
            reg::DRAW_INDEXED => self.do_draw(mem, a, true),
            reg::FLUSH => {
                if let Some(rt) = &self.rt {
                    if !mem.write(addr(self.rt_addr.0, self.rt_addr.1), &rt.to_rgba_bytes()) {
                        self.errors.push("flush: no se pudo escribir el destino".into());
                    }
                }
            }
            _ => self.unknown.push((sub, m, a)),
        }
    }
}

/// «GPU» con ocho subcanales; cada uno enlaza una clase de motor con `SET_OBJECT`.
#[derive(Default)]
pub struct Gpu {
    subchannels: [Option<u32>; 8],
    pub engine: Toy3d,
    /// Llamadas a subcanales sin clase asociada o con una clase desconocida.
    pub bad_calls: Vec<MethodCall>,
}

impl Gpu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn submit(&mut self, mem: &mut dyn GpuMemory, calls: &[MethodCall]) {
        for c in calls {
            if c.method == reg::SET_OBJECT {
                self.subchannels[c.subchannel as usize & 7] = Some(c.arg & 0xFFFF);
                continue;
            }
            match self.subchannels[c.subchannel as usize & 7] {
                Some(CLASS_TOY3D) => self.engine.method(mem, c.subchannel, c.method, c.arg),
                _ => self.bad_calls.push(*c),
            }
        }
    }
}

