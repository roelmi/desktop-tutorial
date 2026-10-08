//! Dibuja una escena de prueba con el rasterizador por software y la guarda como PNG: `cargo run --example demo -- salida.png`.
use nxgpu::asm::assemble;
use nxgpu::ir::Stage;
use nxgpu::raster::*;

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

fn png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::new();
    for row in rgba.chunks(w as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    // zlib sin compresión (bloques almacenados)
    let mut z = vec![0x78, 0x01];
    let mut chunks = raw.chunks(65535).peekable();
    while let Some(c) = chunks.next() {
        z.push(if chunks.peek().is_none() { 1 } else { 0 });
        z.extend_from_slice(&(c.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(c.len() as u16)).to_le_bytes());
        z.extend_from_slice(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut chunk = |ty: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = ty.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    out
}

fn f32s(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn main() {
    let (w, h) = (320u32, 240u32);
    let mut rt = RenderTarget::new(w, h);
    // fondo en degradado vertical: dos triángulos con colores distintos arriba y abajo
    let vs = assemble(Stage::Vertex, "MOV o0, i0; MOV o1, i1;").unwrap();
    let fs_col = assemble(Stage::Fragment, "MOV o0, i1;").unwrap();
    let at = [
        VertexAttrib { location: 0, buffer: 0, offset: 0, format: AttribFormat::F32x4 },
        VertexAttrib { location: 1, buffer: 0, offset: 16, format: AttribFormat::F32x4 },
    ];
    let v = |p: [f32; 4], c: [f32; 4]| [p, c].concat();
    let bg = [
        v([-1.0, 1.0, 0.9, 1.0], [0.05, 0.05, 0.25, 1.0]), v([-1.0, -1.0, 0.9, 1.0], [0.6, 0.35, 0.2, 1.0]), v([1.0, -1.0, 0.9, 1.0], [0.6, 0.35, 0.2, 1.0]),
        v([-1.0, 1.0, 0.9, 1.0], [0.05, 0.05, 0.25, 1.0]), v([1.0, -1.0, 0.9, 1.0], [0.6, 0.35, 0.2, 1.0]), v([1.0, 1.0, 0.9, 1.0], [0.05, 0.05, 0.25, 1.0]),
    ];
    let mut st = State::new(w, h);
    st.depth_test = true;
    let draw_with = |rt: &mut RenderTarget, vs, fs, data: Vec<f32>, count, st: State, tex: &[Option<Texture>]| {
        let bufs = [VertexBuffer { data: f32s(&data), stride: 32 }];
        draw(rt, &Draw { topology: Topology::TriangleList, indices: None, first: 0, count, vs, fs, attribs: &at, buffers: &bufs, vs_consts: &[], fs_consts: &[], textures: tex, state: st });
    };
    draw_with(&mut rt, &vs, &fs_col, bg.concat(), 6, st, &[]);
    // triángulo con colores RGB interpolados, delante
    let tri = [v([-0.8, -0.7, 0.4, 1.0], [1.0, 0.1, 0.1, 1.0]), v([0.1, -0.7, 0.4, 1.0], [0.1, 1.0, 0.1, 1.0]), v([-0.35, 0.8, 0.4, 1.0], [0.1, 0.3, 1.0, 1.0])];
    draw_with(&mut rt, &vs, &fs_col, tri.concat(), 3, st, &[]);
    // cuadrilátero texturizado (tablero) con mezcla alfa, parcialmente delante y detrás del triángulo (prueba de profundidad)
    let tex = Texture {
        width: 8,
        height: 8,
        rgba: (0..64).map(|i| if ((i % 8) + (i / 8)) % 2 == 0 { [250, 250, 250, 255] } else { [30, 30, 60, 255] }).collect(),
        filter: Filter::Nearest,
        wrap: Wrap::Repeat,
    };
    let fs_tex = assemble(Stage::Fragment, "TEX r0, i1, t0; MUL o0, r0, {1.0, 1.0, 1.0, 0.8};").unwrap();
    let q = |p: [f32; 4], uv: [f32; 2]| [p.to_vec(), uv.to_vec(), vec![0.0, 1.0]].concat();
    let quad = [
        q([-0.1, 0.7, 0.3, 1.0], [0.0, 0.0]), q([-0.1, -0.5, 0.5, 1.0], [0.0, 1.0]), q([0.85, -0.5, 0.5, 1.0], [1.0, 1.0]),
        q([-0.1, 0.7, 0.3, 1.0], [0.0, 0.0]), q([0.85, -0.5, 0.5, 1.0], [1.0, 1.0]), q([0.85, 0.7, 0.3, 1.0], [1.0, 0.0]),
    ];
    let mut st2 = st;
    st2.blend = Blend::ALPHA;
    st2.depth_write = false;
    draw_with(&mut rt, &vs, &fs_tex, quad.concat(), 6, st2, &[Some(tex)]);
    let path = std::env::args().nth(1).unwrap_or_else(|| "demo.png".into());
    std::fs::write(&path, png(w, h, &rt.to_rgba_bytes())).unwrap();
    println!("escrito {path}");
}
