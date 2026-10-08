use nxgpu::asm::assemble;
use nxgpu::engine::{reg, Gpu, CLASS_TOY3D};
use nxgpu::fifo::{decode, read_gpfifo, FifoError, GpFifoEntry, MethodCall, PushBuilder};
use nxgpu::ir::*;
use nxgpu::raster::*;
use nxgpu::{FlatMemory, GpuMemory};

// ------------------------------------------------------------------------------------- FIFO
#[test]
fn decodifica_todos_los_modos_de_envio() {
    let mut b = PushBuilder::new();
    b.inc(0, 0x100, &[1, 2, 3]).non_inc(1, 0x200, &[7, 8]).imm(2, 0x300, 0x1ABC & 0x1FFF).inc_once(3, 0x400, &[5, 6, 7]);
    let calls = decode(&b.words).unwrap();
    let c = |s, m, a| MethodCall { subchannel: s, method: m, arg: a };
    assert_eq!(calls, vec![c(0, 0x100, 1), c(0, 0x101, 2), c(0, 0x102, 3), c(1, 0x200, 7), c(1, 0x200, 8), c(2, 0x300, 0x1ABC & 0x1FFF), c(3, 0x400, 5), c(3, 0x401, 6), c(3, 0x401, 7)]);
}

#[test]
fn errores_de_fifo() {
    // cabecera incremental con 3 argumentos pero solo 1 palabra
    let words = [(1u32 << 29) | (3 << 16) | 0x10, 1];
    assert_eq!(decode(&words), Err(FifoError::Truncated { header_index: 0 }));
    assert_eq!(decode(&[7u32 << 29]), Err(FifoError::BadMode { header_index: 0, mode: 7 }));
    let mem = FlatMemory::new(64);
    assert_eq!(read_gpfifo(&mem, &[GpFifoEntry { address: 0x1000, length_words: 4 }.to_raw()]), Err(FifoError::Memory(0x1000)));
}

#[test]
fn entrada_gpfifo_ida_y_vuelta_y_lectura_de_memoria() {
    let e = GpFifoEntry { address: 0x12_3456_7890 & !3, length_words: 7 };
    assert_eq!(GpFifoEntry::from_raw(e.to_raw()), e);
    let mut mem = FlatMemory::new(0x1000);
    let mut b = PushBuilder::new();
    b.inc(0, 0x10, &[0xAA, 0xBB]).imm(0, 0x20, 5);
    assert!(mem.write(0x100, &b.bytes()));
    let calls = read_gpfifo(&mem, &[GpFifoEntry { address: 0x100, length_words: b.words.len() as u32 }.to_raw()]).unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[1], MethodCall { subchannel: 0, method: 0x11, arg: 0xBB });
}

// ------------------------------------------------------------------------------- traducción
#[cfg(feature = "translate")]
mod translate {
    use super::*;
    use nxgpu::translate::{to_wgsl, translate};

    const VS: &str = "DP4 o0.x, i0, c0; DP4 o0.y, i0, c1; DP4 o0.z, i0, c2; DP4 o0.w, i0, c3; MOV o1, i1; MOV o2, i2;";
    const FS_ALL_OPS: &str = "
        MOV r0, i1; ADD r0, r0, c0; SUB r1, r0, c1; MUL r2, r0, r1; MAD r3, r0, r1, r2;
        DP3 r4, r0, r1; DP4 r5, r0, r1; MIN r6, r0, r1; MAX r7, r0, r1; FLR r8, r0; FRC r9, r0;
        SLT r10, r0, r1; SGE r11, r0, r1; CMP r12, r0, r1, r2;
        RCP r13, c2; RSQ r14, c2; EX2 r15, r0; LG2 r16, r1;
        TEX r17, i2, t0; MUL_SAT r17.xyz, r17, |c3.wwww|; ADD r17, r17, -r8;
        ADD r17, r17, r3; ADD r17, r17, r4; ADD r17, r17, r5; ADD r17, r17, r6; ADD r17, r17, r7;
        ADD r17, r17, r9; ADD r17, r17, r10; ADD r17, r17, r11; ADD r17, r17, r12;
        ADD r17, r17, r13; ADD r17, r17, r14; ADD r17, r17, r15; ADD r17, r17, r16;
        MOV o0, r17;";

    #[test]
    fn traduce_a_spirv_y_msl_y_naga_los_valida() {
        for (stage, src) in [(Stage::Vertex, VS), (Stage::Fragment, FS_ALL_OPS), (Stage::Fragment, "MOV o0, i1;"), (Stage::Vertex, "MOV o0, {0.0, 0.0, 0.0, 1.0};")] {
            let shader = assemble(stage, src).unwrap();
            let t = translate(&shader).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(t.spirv[0], 0x0723_0203, "magia de SPIR-V");
            let entry = if stage == Stage::Vertex { "vs_main" } else { "fs_main" };
            assert!(t.wgsl.contains(entry));
            assert!(t.msl.contains(if stage == Stage::Vertex { "vertex" } else { "fragment" }), "{}", t.msl);
            // el SPIR-V generado se vuelve a leer con naga
            let bytes: Vec<u8> = t.spirv.iter().flat_map(|w| w.to_le_bytes()).collect();
            let module = naga::front::spv::parse_u8_slice(&bytes, &naga::front::spv::Options::default()).expect("SPIR-V releíble");
            assert!(!module.entry_points.is_empty());
        }
    }

    #[test]
    fn escritura_con_mascara_y_saturacion_en_wgsl() {
        let s = assemble(Stage::Fragment, "MOV_SAT r0.xz, i1; MOV o0, r0;").unwrap();
        let w = to_wgsl(&s).unwrap();
        assert!(w.contains("clamp("));
        assert!(w.contains("r0.x = v.x;") && w.contains("r0.z = v.z;") && !w.contains("r0.y = v.y;"), "{w}");
    }

    #[test]
    fn rechaza_lo_que_no_se_puede_traducir() {
        let s = assemble(Stage::Fragment, "MOV o1, i1;").unwrap();
        assert!(to_wgsl(&s).unwrap_err().contains("o0"));
        let inf = assemble(Stage::Fragment, "MOV o0, {1e39, 0, 0, 0};");
        // 1e39 desborda f32: se convierte en infinito y debe rechazarse con un mensaje claro
        if let Ok(sh) = inf {
            assert!(to_wgsl(&sh).unwrap_err().contains("no finito"));
        }
    }
}

// ------------------------------------------------------------------- recorrido de extremo a extremo
fn f32w(v: f32) -> u32 {
    v.to_bits()
}

#[test]
fn pushbuffer_a_imagen_coincide_con_el_dibujo_directo() {
    const VB: u64 = 0x1000;
    const VS: u64 = 0x2000;
    const FS: u64 = 0x2400;
    const TEX: u64 = 0x3000;
    const OUT: u64 = 0x8000;
    const PB: u64 = 0x100;
    let (w, h) = (24u32, 24u32);
    let vs_src = "MOV o0, i0; MOV o1, i1;";
    let fs_src = "TEX r0, i1, t0; MUL o0, r0, c0;";
    // quad texturado (2 triángulos) con uv
    let quad: [([f32; 4], [f32; 2]); 6] = [
        ([-0.8, 0.8, 0.5, 1.0], [0.0, 0.0]), ([-0.8, -0.8, 0.5, 1.0], [0.0, 1.0]), ([0.8, -0.8, 0.5, 1.0], [1.0, 1.0]),
        ([-0.8, 0.8, 0.5, 1.0], [0.0, 0.0]), ([0.8, -0.8, 0.5, 1.0], [1.0, 1.0]), ([0.8, 0.8, 0.5, 1.0], [1.0, 0.0]),
    ];
    let mut vb = Vec::new();
    for (p, uv) in quad {
        for c in p {
            vb.extend(c.to_le_bytes());
        }
        for c in uv {
            vb.extend(c.to_le_bytes());
        }
    }
    let tex: Vec<u8> = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255];

    let mut mem = FlatMemory::new(0x20000);
    mem.write(VB, &vb);
    mem.write(VS, vs_src.as_bytes());
    mem.write(FS, fs_src.as_bytes());
    mem.write(TEX, &tex);

    let mut b = PushBuilder::new();
    b.inc(0, reg::SET_OBJECT, &[CLASS_TOY3D]);
    b.inc(0, reg::RT_ADDR_HI, &[0, OUT as u32]);
    b.inc(0, reg::RT_SIZE, &[w | (h << 16)]);
    b.inc(0, reg::CLEAR_COLOR, &[f32w(0.1), f32w(0.2), f32w(0.3), f32w(1.0)]);
    b.inc(0, reg::CLEAR, &[1]);
    b.inc(0, reg::SHADER_SELECT, &[0]).inc(0, reg::SHADER_ADDR_HI, &[0, VS as u32]).inc(0, reg::SHADER_SIZE, &[vs_src.len() as u32]);
    b.inc(0, reg::SHADER_SELECT, &[1]).inc(0, reg::SHADER_ADDR_HI, &[0, FS as u32]).inc(0, reg::SHADER_SIZE, &[fs_src.len() as u32]);
    b.inc(0, reg::VB_SELECT, &[0]).inc(0, reg::VB_ADDR_HI, &[0, VB as u32, vb.len() as u32, 24]);
    b.inc(0, reg::ATTR_SELECT, &[0]).inc(0, reg::ATTR_CONFIG, &[3 << 16]); // buffer 0, offset 0, F32x4
    b.inc(0, reg::ATTR_SELECT, &[1]).inc(0, reg::ATTR_CONFIG, &[(16 << 4) | (1 << 16)]); // offset 16, F32x2
    b.inc(0, reg::CONST_SELECT, &[1 << 8]).inc(0, reg::CONST_DATA, &[f32w(1.0), f32w(1.0), f32w(1.0), f32w(0.75)]);
    b.inc(0, reg::TEX_SELECT, &[0]).inc(0, reg::TEX_ADDR_HI, &[0, TEX as u32, 2 | (2 << 16)]).inc(0, reg::TEX_CONFIG, &[0]);
    b.inc(0, reg::TOPOLOGY, &[0]).inc(0, reg::DRAW_FIRST, &[0]).inc(0, reg::DRAW_ARRAYS, &[6]);
    b.inc(0, reg::FLUSH, &[0]);
    mem.write(PB, &b.bytes());

    let calls = read_gpfifo(&mem, &[GpFifoEntry { address: PB, length_words: b.words.len() as u32 }.to_raw()]).unwrap();
    let mut gpu = Gpu::new();
    gpu.submit(&mut mem, &calls);
    assert!(gpu.engine.errors.is_empty(), "{:?}", gpu.engine.errors);
    assert!(gpu.engine.unknown.is_empty(), "{:?}", gpu.engine.unknown);
    assert_eq!(gpu.engine.draws, 1);

    // el mismo dibujo hecho directamente con el rasterizador
    let vs = assemble(Stage::Vertex, vs_src).unwrap();
    let fs = assemble(Stage::Fragment, fs_src).unwrap();
    let at = [VertexAttrib { location: 0, buffer: 0, offset: 0, format: AttribFormat::F32x4 }, VertexAttrib { location: 1, buffer: 0, offset: 16, format: AttribFormat::F32x2 }];
    let bufs = [VertexBuffer { data: vb.clone(), stride: 24 }];
    let texs = [Some(Texture { width: 2, height: 2, rgba: tex.chunks(4).map(|c| [c[0], c[1], c[2], c[3]]).collect(), filter: Filter::Nearest, wrap: Wrap::Repeat })];
    let mut rt = RenderTarget::new(w, h);
    rt.clear_color([0.1, 0.2, 0.3, 1.0]);
    let consts = [[1.0, 1.0, 1.0, 0.75]];
    let call = Draw { topology: Topology::TriangleList, indices: None, first: 0, count: 6, vs: &vs, fs: &fs, attribs: &at, buffers: &bufs, vs_consts: &[], fs_consts: &consts, textures: &texs, state: State::new(w, h) };
    draw(&mut rt, &call);
    let mut written = vec![0u8; (w * h * 4) as usize];
    assert!(mem.read(OUT, &mut written));
    assert_eq!(written, rt.to_rgba_bytes(), "la imagen escrita en memoria difiere del dibujo directo");
    // contenido razonable: fondo en las esquinas, texels en el interior (con alfa 0.75 del shader)
    assert_eq!(rt.pixel(0, 0), [26, 51, 77, 255]);
    assert_eq!(rt.pixel(6, 6), [255, 0, 0, 191]);
    assert_eq!(rt.pixel(17, 17), [255, 255, 0, 191]);
}

#[test]
fn indexado_tiras_abanicos_y_subcanales_desconocidos() {
    let mut mem = FlatMemory::new(0x10000);
    // 4 vértices (posición xyzw + color) y 2 triángulos indexados u16
    let pts = [[-1.0f32, -1.0, 0.5, 1.0], [1.0, -1.0, 0.5, 1.0], [1.0, 1.0, 0.5, 1.0], [-1.0, 1.0, 0.5, 1.0]];
    let mut vb = Vec::new();
    for p in pts {
        for c in p.iter().chain([1.0f32, 0.0, 0.0, 1.0].iter()) {
            vb.extend(c.to_le_bytes());
        }
    }
    mem.write(0x1000, &vb);
    let idx: Vec<u8> = [0u16, 1, 2, 0, 2, 3].iter().flat_map(|i| i.to_le_bytes()).collect();
    mem.write(0x2000, &idx);
    mem.write(0x3000, b"MOV o0, i0; MOV o1, i1;");
    mem.write(0x3100, b"MOV o0, i1;");
    let mut b = PushBuilder::new();
    b.inc(0, reg::SET_OBJECT, &[CLASS_TOY3D]).inc(0, reg::RT_SIZE, &[8 | (8 << 16)]).inc(0, reg::CLEAR, &[1]);
    b.inc(0, reg::SHADER_SELECT, &[0]).inc(0, reg::SHADER_ADDR_HI, &[0, 0x3000]).inc(0, reg::SHADER_SIZE, &[23]);
    b.inc(0, reg::SHADER_SELECT, &[1]).inc(0, reg::SHADER_ADDR_HI, &[0, 0x3100]).inc(0, reg::SHADER_SIZE, &[11]);
    b.inc(0, reg::VB_SELECT, &[0]).inc(0, reg::VB_ADDR_HI, &[0, 0x1000, vb.len() as u32, 32]);
    b.inc(0, reg::ATTR_SELECT, &[0]).inc(0, reg::ATTR_CONFIG, &[3 << 16]).inc(0, reg::ATTR_SELECT, &[1]).inc(0, reg::ATTR_CONFIG, &[(16 << 4) | (3 << 16)]);
    b.inc(0, reg::INDEX_ADDR_HI, &[0, 0x2000, 0]).inc(0, reg::DRAW_INDEXED, &[6]);
    // llamadas en un subcanal sin clase y un método que el motor no conoce
    b.inc(5, 0x50, &[1]).inc(0, 0x7FF, &[9]);
    let calls = decode(&b.words).unwrap();
    let mut gpu = Gpu::new();
    gpu.submit(&mut mem, &calls);
    assert!(gpu.engine.errors.is_empty(), "{:?}", gpu.engine.errors);
    let rt = gpu.engine.render_target().unwrap();
    assert!(rt.color.iter().all(|c| *c == [255, 0, 0, 255]), "el cuadrilátero indexado cubre todo el destino");
    assert_eq!(gpu.bad_calls.len(), 1);
    assert_eq!(gpu.engine.unknown, vec![(0, 0x7FF, 9)]);
}
