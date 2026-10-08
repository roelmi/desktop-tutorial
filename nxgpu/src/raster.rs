//! Rasterizador por software: búsqueda de vértices, sombreado, recorte, rasterización con reglas de relleno
//! superior-izquierda, interpolación con corrección de perspectiva, prueba de profundidad y mezcla.
//!
//! Convenciones: +y es «arriba» en NDC y la fila 0 de la imagen es la de arriba. La cara frontal es antihoraria.
//! La profundidad de la ventana está en [min_depth, max_depth].

use crate::interp::{self, TextureSampler};
use crate::ir::Shader;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topology {
    TriangleList,
    TriangleStrip,
    TriangleFan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttribFormat {
    F32x1,
    F32x2,
    F32x3,
    F32x4,
    U8x4Norm,
}

#[derive(Clone, Copy, Debug)]
pub struct VertexAttrib {
    pub location: u8,
    pub buffer: u8,
    pub offset: u32,
    pub format: AttribFormat,
}

#[derive(Clone, Debug, Default)]
pub struct VertexBuffer {
    pub data: Vec<u8>,
    pub stride: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compare {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

impl Compare {
    fn test(self, new: f32, old: f32) -> bool {
        match self {
            Compare::Never => false,
            Compare::Less => new < old,
            Compare::Equal => new == old,
            Compare::LessEqual => new <= old,
            Compare::Greater => new > old,
            Compare::NotEqual => new != old,
            Compare::GreaterEqual => new >= old,
            Compare::Always => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstColor,
    OneMinusDstColor,
    DstAlpha,
    OneMinusDstAlpha,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendOp {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

#[derive(Clone, Copy, Debug)]
pub struct Blend {
    pub enabled: bool,
    pub src_rgb: BlendFactor,
    pub dst_rgb: BlendFactor,
    pub op_rgb: BlendOp,
    pub src_a: BlendFactor,
    pub dst_a: BlendFactor,
    pub op_a: BlendOp,
}

impl Blend {
    pub const OFF: Blend = Blend {
        enabled: false,
        src_rgb: BlendFactor::One,
        dst_rgb: BlendFactor::Zero,
        op_rgb: BlendOp::Add,
        src_a: BlendFactor::One,
        dst_a: BlendFactor::Zero,
        op_a: BlendOp::Add,
    };
    pub const ALPHA: Blend = Blend {
        enabled: true,
        src_rgb: BlendFactor::SrcAlpha,
        dst_rgb: BlendFactor::OneMinusSrcAlpha,
        op_rgb: BlendOp::Add,
        src_a: BlendFactor::One,
        dst_a: BlendFactor::OneMinusSrcAlpha,
        op_a: BlendOp::Add,
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CullMode {
    None,
    Back,
    Front,
}

#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    Nearest,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    Repeat,
    Clamp,
    Mirror,
}

#[derive(Clone, Debug)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<[u8; 4]>,
    pub filter: Filter,
    pub wrap: Wrap,
}

fn wrap_coord(i: i64, n: i64, w: Wrap) -> i64 {
    match w {
        Wrap::Repeat => i.rem_euclid(n),
        Wrap::Clamp => i.clamp(0, n - 1),
        Wrap::Mirror => {
            let p = i.rem_euclid(2 * n);
            if p < n { p } else { 2 * n - 1 - p }
        }
    }
}

impl Texture {
    fn texel(&self, x: i64, y: i64) -> [f32; 4] {
        let (x, y) = (wrap_coord(x, self.width as i64, self.wrap), wrap_coord(y, self.height as i64, self.wrap));
        let t = self.rgba[(y as usize) * self.width as usize + x as usize];
        [t[0] as f32 / 255.0, t[1] as f32 / 255.0, t[2] as f32 / 255.0, t[3] as f32 / 255.0]
    }

    pub fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        let (fx, fy) = (u as f64 * self.width as f64, v as f64 * self.height as f64);
        match self.filter {
            Filter::Nearest => self.texel(fx.floor() as i64, fy.floor() as i64),
            Filter::Linear => {
                let (x, y) = (fx - 0.5, fy - 0.5);
                let (x0, y0) = (x.floor(), y.floor());
                let (tx, ty) = ((x - x0) as f32, (y - y0) as f32);
                let (x0, y0) = (x0 as i64, y0 as i64);
                let (a, b, c, d) = (self.texel(x0, y0), self.texel(x0 + 1, y0), self.texel(x0, y0 + 1), self.texel(x0 + 1, y0 + 1));
                let mut r = [0.0; 4];
                for k in 0..4 {
                    let top = a[k] + (b[k] - a[k]) * tx;
                    let bot = c[k] + (d[k] - c[k]) * tx;
                    r[k] = top + (bot - top) * ty;
                }
                r
            }
        }
    }
}

pub struct Textures<'a>(pub &'a [Option<Texture>]);

impl TextureSampler for Textures<'_> {
    fn sample(&self, unit: u8, u: f32, v: f32) -> [f32; 4] {
        match self.0.get(unit as usize).and_then(|t| t.as_ref()) {
            Some(t) => t.sample(u, v),
            None => [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub viewport: Viewport,
    /// [x, y, ancho, alto] en píxeles.
    pub scissor: Option<[i32; 4]>,
    pub depth_test: bool,
    pub depth_write: bool,
    pub depth_func: Compare,
    pub blend: Blend,
    pub cull: CullMode,
    pub color_mask: [bool; 4],
    /// Profundidad de recorte en [0, w] (Vulkan/D3D) en lugar de [-w, w] (OpenGL).
    pub depth_zero_to_one: bool,
}

impl State {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            viewport: Viewport { x: 0.0, y: 0.0, width: width as f32, height: height as f32, min_depth: 0.0, max_depth: 1.0 },
            scissor: None,
            depth_test: false,
            depth_write: true,
            depth_func: Compare::Less,
            blend: Blend::OFF,
            cull: CullMode::None,
            color_mask: [true; 4],
            depth_zero_to_one: true,
        }
    }
}

pub struct RenderTarget {
    pub width: u32,
    pub height: u32,
    pub color: Vec<[u8; 4]>,
    pub depth: Vec<f32>,
}

impl RenderTarget {
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width * height) as usize;
        Self { width, height, color: vec![[0, 0, 0, 0]; n], depth: vec![1.0; n] }
    }

    pub fn clear_color(&mut self, c: [f32; 4]) {
        let px = to_u8(c);
        self.color.fill(px);
    }

    pub fn clear_depth(&mut self, d: f32) {
        self.depth.fill(d);
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        self.color[(y * self.width + x) as usize]
    }

    /// RGBA8 en filas de arriba a abajo.
    pub fn to_rgba_bytes(&self) -> Vec<u8> {
        self.color.iter().flatten().copied().collect()
    }
}

fn to_u8(c: [f32; 4]) -> [u8; 4] {
    let q = |x: f32| ((x.clamp(0.0, 1.0) * 255.0) + 0.5).floor() as u8;
    [q(c[0]), q(c[1]), q(c[2]), q(c[3])]
}

pub struct Draw<'a> {
    pub topology: Topology,
    /// Índices (si es `None`, se dibujan `count` vértices desde `first`).
    pub indices: Option<&'a [u32]>,
    pub first: u32,
    pub count: u32,
    pub vs: &'a Shader,
    pub fs: &'a Shader,
    pub attribs: &'a [VertexAttrib],
    pub buffers: &'a [VertexBuffer],
    pub vs_consts: &'a [[f32; 4]],
    pub fs_consts: &'a [[f32; 4]],
    pub textures: &'a [Option<Texture>],
    pub state: State,
}

#[derive(Clone)]
struct Vert {
    pos: [f32; 4],
    vary: Vec<[f32; 4]>,
}

fn fetch(call: &Draw, vertex: u32) -> Vec<[f32; 4]> {
    let n = call.vs.num_inputs as usize;
    let mut inputs = vec![[0.0, 0.0, 0.0, 1.0]; n.max(1)];
    for a in call.attribs {
        let Some(buf) = call.buffers.get(a.buffer as usize) else { continue };
        let off = (vertex as usize) * buf.stride as usize + a.offset as usize;
        let f32_at = |i: usize| buf.data.get(off + 4 * i..off + 4 * i + 4).map(|b| f32::from_le_bytes(b.try_into().unwrap()));
        let mut v = [0.0, 0.0, 0.0, 1.0];
        match a.format {
            AttribFormat::F32x1 | AttribFormat::F32x2 | AttribFormat::F32x3 | AttribFormat::F32x4 => {
                let k = match a.format { AttribFormat::F32x1 => 1, AttribFormat::F32x2 => 2, AttribFormat::F32x3 => 3, _ => 4 };
                for (c, slot) in v.iter_mut().enumerate().take(k) {
                    *slot = f32_at(c).unwrap_or(0.0);
                }
            }
            AttribFormat::U8x4Norm => {
                if let Some(b) = buf.data.get(off..off + 4) {
                    for c in 0..4 {
                        v[c] = b[c] as f32 / 255.0;
                    }
                }
            }
        }
        if (a.location as usize) < inputs.len() {
            inputs[a.location as usize] = v;
        }
    }
    inputs
}

fn lerp(a: &Vert, b: &Vert, t: f32) -> Vert {
    let l = |x: [f32; 4], y: [f32; 4]| [x[0] + (y[0] - x[0]) * t, x[1] + (y[1] - x[1]) * t, x[2] + (y[2] - x[2]) * t, x[3] + (y[3] - x[3]) * t];
    Vert { pos: l(a.pos, b.pos), vary: a.vary.iter().zip(&b.vary).map(|(x, y)| l(*x, *y)).collect() }
}

/// Recorta el polígono contra el plano `dist(v) >= 0` (Sutherland–Hodgman).
fn clip_plane(poly: Vec<Vert>, dist: impl Fn(&Vert) -> f32) -> Vec<Vert> {
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (a, b) = (&poly[i], &poly[(i + 1) % poly.len()]);
        let (da, db) = (dist(a), dist(b));
        if da >= 0.0 {
            out.push(a.clone());
        }
        if (da >= 0.0) != (db >= 0.0) {
            out.push(lerp(a, b, da / (da - db)));
        }
    }
    out
}

#[derive(Clone)]
struct ScreenVert {
    x: i64,
    y: i64,
    z: f32,
    inv_w: f32,
    vary: Vec<[f32; 4]>,
}

const SUBPIXEL: f64 = 256.0;

fn factor(f: BlendFactor, src: [f32; 4], dst: [f32; 4]) -> [f32; 4] {
    match f {
        BlendFactor::Zero => [0.0; 4],
        BlendFactor::One => [1.0; 4],
        BlendFactor::SrcColor => src,
        BlendFactor::OneMinusSrcColor => [1.0 - src[0], 1.0 - src[1], 1.0 - src[2], 1.0 - src[3]],
        BlendFactor::SrcAlpha => [src[3]; 4],
        BlendFactor::OneMinusSrcAlpha => [1.0 - src[3]; 4],
        BlendFactor::DstColor => dst,
        BlendFactor::OneMinusDstColor => [1.0 - dst[0], 1.0 - dst[1], 1.0 - dst[2], 1.0 - dst[3]],
        BlendFactor::DstAlpha => [dst[3]; 4],
        BlendFactor::OneMinusDstAlpha => [1.0 - dst[3]; 4],
    }
}

fn combine(op: BlendOp, s: f32, d: f32, sf: f32, df: f32) -> f32 {
    match op {
        BlendOp::Add => s * sf + d * df,
        BlendOp::Subtract => s * sf - d * df,
        BlendOp::ReverseSubtract => d * df - s * sf,
        BlendOp::Min => s.min(d),
        BlendOp::Max => s.max(d),
    }
}

fn blend(b: &Blend, src: [f32; 4], dst: [f32; 4]) -> [f32; 4] {
    if !b.enabled {
        return src;
    }
    let (sr, dr) = (factor(b.src_rgb, src, dst), factor(b.dst_rgb, src, dst));
    let (sa, da) = (factor(b.src_a, src, dst), factor(b.dst_a, src, dst));
    [
        combine(b.op_rgb, src[0], dst[0], sr[0], dr[0]),
        combine(b.op_rgb, src[1], dst[1], sr[1], dr[1]),
        combine(b.op_rgb, src[2], dst[2], sr[2], dr[2]),
        combine(b.op_a, src[3], dst[3], sa[3], da[3]),
    ]
}

fn top_left(ax: i64, ay: i64, bx: i64, by: i64) -> bool {
    let (dx, dy) = (bx - ax, by - ay);
    dy < 0 || (dy == 0 && dx > 0)
}

/// Dibuja la llamada en el destino. Devuelve el número de fragmentos sombreados.
pub fn draw(rt: &mut RenderTarget, call: &Draw) -> u64 {
    // 1) vértices → sombreado de vértices
    let order: Vec<u32> = match call.indices {
        Some(ix) => ix.iter().skip(call.first as usize).take(call.count as usize).copied().collect(),
        None => (call.first..call.first + call.count).collect(),
    };
    let sampler = Textures(call.textures);
    let verts: Vec<Vert> = order
        .iter()
        .map(|&v| {
            let out = interp::run(call.vs, &fetch(call, v), call.vs_consts, &sampler);
            Vert { pos: out[0], vary: out[1..].to_vec() }
        })
        .collect();
    // 2) ensamblado de triángulos
    let mut tris: Vec<[usize; 3]> = Vec::new();
    let n = verts.len();
    match call.topology {
        Topology::TriangleList => (0..n / 3).for_each(|i| tris.push([3 * i, 3 * i + 1, 3 * i + 2])),
        Topology::TriangleStrip => (0..n.saturating_sub(2)).for_each(|i| tris.push(if i % 2 == 0 { [i, i + 1, i + 2] } else { [i + 1, i, i + 2] })),
        Topology::TriangleFan => (1..n.saturating_sub(1)).for_each(|i| tris.push([0, i, i + 1])),
    }
    let st = &call.state;
    let mut shaded = 0u64;
    let nvary = call.fs.num_inputs.saturating_sub(1) as usize;
    for t in tris {
        let mut poly: Vec<Vert> = t.iter().map(|&i| verts[i].clone()).collect();
        // 3) recorte contra el plano cercano y w > 0
        poly = clip_plane(poly, |v| v.pos[3] - 1e-6);
        let zero_one = st.depth_zero_to_one;
        poly = clip_plane(poly, |v| if zero_one { v.pos[2] } else { v.pos[2] + v.pos[3] });
        if poly.len() < 3 {
            continue;
        }
        let vp = &st.viewport;
        let screen: Vec<ScreenVert> = poly
            .iter()
            .map(|v| {
                let iw = 1.0 / v.pos[3];
                let (nx, ny, nz) = (v.pos[0] * iw, v.pos[1] * iw, v.pos[2] * iw);
                let sx = vp.x as f64 + (nx as f64 * 0.5 + 0.5) * vp.width as f64;
                let sy = vp.y as f64 + (0.5 - ny as f64 * 0.5) * vp.height as f64;
                let z01 = if zero_one { nz } else { nz * 0.5 + 0.5 };
                ScreenVert {
                    x: (sx * SUBPIXEL).round() as i64,
                    y: (sy * SUBPIXEL).round() as i64,
                    z: vp.min_depth + z01 * (vp.max_depth - vp.min_depth),
                    inv_w: iw,
                    vary: v.vary.clone(),
                }
            })
            .collect();
        for k in 1..screen.len() - 1 {
            shaded += raster_triangle(rt, call, &[&screen[0], &screen[k], &screen[k + 1]], nvary);
        }
    }
    shaded
}

fn raster_triangle(rt: &mut RenderTarget, call: &Draw, tri: &[&ScreenVert; 3], nvary: usize) -> u64 {
    let st = &call.state;
    let (a, b, c) = (tri[0], tri[1], tri[2]);
    let cross = (b.x - a.x) as i128 * (c.y - a.y) as i128 - (c.x - a.x) as i128 * (b.y - a.y) as i128;
    if cross == 0 {
        return 0;
    }
    // y hacia abajo en pantalla: antihorario en NDC ⇒ cross < 0
    let front = cross < 0;
    match st.cull {
        CullMode::Back if !front => return 0,
        CullMode::Front if front => return 0,
        _ => {}
    }
    // normaliza la orientación para que las funciones de arista sean positivas dentro
    let (v1, v2) = if cross > 0 { (b, c) } else { (c, b) };
    let v0 = a;
    let verts = [v0, v1, v2];
    let area = ((v1.x - v0.x) as i128 * (v2.y - v0.y) as i128 - (v2.x - v0.x) as i128 * (v1.y - v0.y) as i128) as f64;

    let min_x = verts.iter().map(|v| v.x).min().unwrap().div_euclid(256).max(0) as i32;
    let max_x = (verts.iter().map(|v| v.x).max().unwrap().div_euclid(256) as i32).min(rt.width as i32 - 1);
    let min_y = verts.iter().map(|v| v.y).min().unwrap().div_euclid(256).max(0) as i32;
    let max_y = (verts.iter().map(|v| v.y).max().unwrap().div_euclid(256) as i32).min(rt.height as i32 - 1);
    let (sx0, sy0, sx1, sy1) = match st.scissor {
        Some([x, y, w, h]) => (x.max(0), y.max(0), (x + w - 1).min(rt.width as i32 - 1), (y + h - 1).min(rt.height as i32 - 1)),
        None => (0, 0, rt.width as i32 - 1, rt.height as i32 - 1),
    };
    let (x_lo, x_hi, y_lo, y_hi) = (min_x.max(sx0), max_x.min(sx1), min_y.max(sy0), max_y.min(sy1));
    let mut count = 0;
    let sampler = Textures(call.textures);
    let edges = [(v1, v2), (v2, v0), (v0, v1)];
    let tl: Vec<bool> = edges.iter().map(|(p, q)| top_left(p.x, p.y, q.x, q.y)).collect();
    for py in y_lo..=y_hi {
        for px in x_lo..=x_hi {
            let (cx, cy) = ((px as i64) * 256 + 128, (py as i64) * 256 + 128);
            let mut e = [0i128; 3];
            let mut inside = true;
            for (k, (p, q)) in edges.iter().enumerate() {
                e[k] = (q.x - p.x) as i128 * (cy - p.y) as i128 - (q.y - p.y) as i128 * (cx - p.x) as i128;
                if e[k] < 0 || (e[k] == 0 && !tl[k]) {
                    inside = false;
                    break;
                }
            }
            if !inside {
                continue;
            }
            // coordenadas baricéntricas: e[0] pesa v0, e[1] pesa v1, e[2] pesa v2 (arista opuesta a cada vértice)
            let (l0, l1, l2) = (e[0] as f64 / area, e[1] as f64 / area, e[2] as f64 / area);
            // la arista k = (verts[k+1], verts[k+2]) es la opuesta a verts[k]
            let lam = [l0, l1, l2];
            let z = (lam[0] * verts[0].z as f64 + lam[1] * verts[1].z as f64 + lam[2] * verts[2].z as f64) as f32;
            let idx = (py as u32 * rt.width + px as u32) as usize;
            if st.depth_test && !st.depth_func.test(z, rt.depth[idx]) {
                continue;
            }
            // interpolación con corrección de perspectiva
            let pw = [lam[0] * verts[0].inv_w as f64, lam[1] * verts[1].inv_w as f64, lam[2] * verts[2].inv_w as f64];
            let sum = pw[0] + pw[1] + pw[2];
            let mut inputs = vec![[0.0f32; 4]; nvary + 1];
            inputs[0] = [px as f32 + 0.5, py as f32 + 0.5, z, (sum) as f32];
            for k in 0..nvary {
                let mut v = [0.0f32; 4];
                for c in 0..4 {
                    let mut acc = 0.0f64;
                    for (i, vert) in verts.iter().enumerate() {
                        acc += pw[i] * vert.vary.get(k).map_or(0.0, |x| x[c]) as f64;
                    }
                    v[c] = (acc / sum) as f32;
                }
                inputs[k + 1] = v;
            }
            let out = interp::run(call.fs, &inputs, call.fs_consts, &sampler);
            count += 1;
            let src = out[0];
            let old = rt.color[idx];
            let dst = [old[0] as f32 / 255.0, old[1] as f32 / 255.0, old[2] as f32 / 255.0, old[3] as f32 / 255.0];
            let res = to_u8(blend(&st.blend, src, dst));
            let mut px_out = old;
            for k in 0..4 {
                if st.color_mask[k] {
                    px_out[k] = res[k];
                }
            }
            rt.color[idx] = px_out;
            if st.depth_test && st.depth_write {
                rt.depth[idx] = z;
            }
        }
    }
    count
}
