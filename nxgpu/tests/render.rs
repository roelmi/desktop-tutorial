use nxgpu::asm::{assemble, disassemble};
use nxgpu::interp::{self, NoTextures};
use nxgpu::ir::*;
use nxgpu::raster::*;

fn vs_pass() -> Shader {
    assemble(Stage::Vertex, "MOV o0, i0; MOV o1, i1;").unwrap()
}
fn fs_color() -> Shader {
    assemble(Stage::Fragment, "MOV o0, i1;").unwrap()
}
fn fs_const() -> Shader {
    assemble(Stage::Fragment, "MOV o0, c0;").unwrap()
}

fn f32s(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// Vértices con posición (xyzw) y color (rgba): 8 floats.
fn verts(v: &[([f32; 4], [f32; 4])]) -> VertexBuffer {
    let mut data = Vec::new();
    for (p, c) in v {
        data.extend(f32s(p));
        data.extend(f32s(c));
    }
    VertexBuffer { data, stride: 32 }
}

fn attribs() -> Vec<VertexAttrib> {
    vec![
        VertexAttrib { location: 0, buffer: 0, offset: 0, format: AttribFormat::F32x4 },
        VertexAttrib { location: 1, buffer: 0, offset: 16, format: AttribFormat::F32x4 },
    ]
}

fn run_draw(rt: &mut RenderTarget, vs: &Shader, fs: &Shader, vb: VertexBuffer, count: u32, state: State, consts: &[[f32; 4]], tex: &[Option<Texture>]) -> u64 {
    let at = attribs();
    let bufs = [vb];
    let call = Draw {
        topology: Topology::TriangleList,
        indices: None,
        first: 0,
        count,
        vs,
        fs,
        attribs: &at,
        buffers: &bufs,
        vs_consts: &[],
        fs_consts: consts,
        textures: tex,
        state,
    };
    draw(rt, &call)
}

fn p(x: f32, y: f32) -> [f32; 4] {
    [x, y, 0.5, 1.0]
}
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

#[test]
fn clear_rellena_todo() {
    let mut rt = RenderTarget::new(8, 4);
    rt.clear_color([1.0, 0.0, 0.5, 1.0]);
    assert!(rt.color.iter().all(|c| *c == [255, 0, 128, 255]));
    rt.clear_depth(0.25);
    assert!(rt.depth.iter().all(|d| *d == 0.25));
}

#[test]
fn triangulo_cubre_la_mitad_y_el_resto_queda_intacto() {
    let mut rt = RenderTarget::new(16, 16);
    let n = run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&[(p(-1.0, -1.0), WHITE), (p(1.0, -1.0), WHITE), (p(-1.0, 1.0), WHITE)]), 3, State::new(16, 16), &[], &[]);
    let lit = rt.color.iter().filter(|c| c[3] == 255).count() as u64;
    assert_eq!(lit, n);
    // la diagonal pasa por centros de píxel: 120 (debajo) + 0 o 16 (según la regla de relleno), pero nunca más
    assert!(lit == 120 || lit == 136, "{lit}");
    assert_eq!(rt.pixel(0, 15), [255, 255, 255, 255]);
    assert_eq!(rt.pixel(15, 0), [0, 0, 0, 0]);
}

#[test]
fn arista_compartida_sin_huecos_ni_solapes() {
    // Un cuadrilátero dividido en dos triángulos, con mezcla aditiva de 0.5: cada píxel debe recibir exactamente un dibujo.
    let add = Blend { enabled: true, src_rgb: BlendFactor::One, dst_rgb: BlendFactor::One, op_rgb: BlendOp::Add, src_a: BlendFactor::One, dst_a: BlendFactor::One, op_a: BlendOp::Add };
    for (w, h, quad) in [
        (16u32, 16u32, [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]),
        (13, 7, [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]),
        (23, 17, [(-0.9, -0.77), (0.83, -0.91), (0.95, 0.71), (-0.88, 0.8)]),
        (31, 29, [(-1.2, -1.3), (1.4, -1.1), (1.5, 1.2), (-1.1, 1.6)]),
    ] {
        for diagonal in 0..2 {
            let q = quad.map(|(x, y)| p(x, y));
            let tris = if diagonal == 0 { [q[0], q[1], q[2], q[0], q[2], q[3]] } else { [q[0], q[1], q[3], q[1], q[2], q[3]] };
            let mut rt = RenderTarget::new(w, h);
            let mut st = State::new(w, h);
            st.blend = add;
            let vb = verts(&tris.map(|t| (t, [0.0; 4])));
            run_draw(&mut rt, &vs_pass(), &fs_const(), vb, 6, st, &[[0.5, 0.5, 0.5, 0.5]], &[]);
            // ¿qué píxeles cubre el cuadrilátero? Se cuentan los que reciben 128 (una vez); nunca debe haber 255 (doble)
            let doubles = rt.color.iter().filter(|c| c[0] == 255).count();
            assert_eq!(doubles, 0, "{w}x{h} diagonal {diagonal}: píxeles dibujados dos veces");
            // los píxeles interiores (lejos del borde) deben estar cubiertos: sin huecos en la costura
            for y in 2..h - 2 {
                for x in 2..w - 2 {
                    let covered = rt.pixel(x, y)[0] == 128;
                    // dentro de este cuadrilátero convexo, el píxel central de cada celda interior está cubierto
                    let inside_quad = {
                        let (nx, ny) = ((x as f32 + 0.5) / w as f32 * 2.0 - 1.0, 1.0 - (y as f32 + 0.5) / h as f32 * 2.0);
                        let mut s = [0.0f32; 4];
                        for k in 0..4 {
                            let (a, b) = (quad[k], quad[(k + 1) % 4]);
                            s[k] = (b.0 - a.0) * (ny - a.1) - (b.1 - a.1) * (nx - a.0);
                        }
                        s.iter().all(|v| *v > 1e-4) || s.iter().all(|v| *v < -1e-4)
                    };
                    assert_eq!(covered, inside_quad, "{w}x{h} diag {diagonal} en ({x},{y})");
                }
            }
        }
    }
}

#[test]
fn interpolacion_lineal_de_colores() {
    let mut rt = RenderTarget::new(32, 32);
    let red = [1.0, 0.0, 0.0, 1.0];
    let green = [0.0, 1.0, 0.0, 1.0];
    let blue = [0.0, 0.0, 1.0, 1.0];
    run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&[(p(-1.0, -1.0), red), (p(3.0, -1.0), green), (p(-1.0, 3.0), blue)]), 3, State::new(32, 32), &[], &[]);
    for (px, py) in [(0u32, 31u32), (8, 20), (16, 16), (5, 3), (25, 6)] {
        let (x, y) = ((px as f64 + 0.5) / 16.0 - 1.0, 1.0 - (py as f64 + 0.5) / 16.0);
        let (g, b) = ((x + 1.0) / 4.0, (y + 1.0) / 4.0);
        let r = 1.0 - g - b;
        let want = [r, g, b].map(|v| (v * 255.0).round() as i32);
        let got = rt.pixel(px, py);
        for k in 0..3 {
            assert!((got[k] as i32 - want[k]).abs() <= 1, "({px},{py}) canal {k}: {} vs {}", got[k], want[k]);
        }
    }
}

#[test]
fn interpolacion_con_correccion_de_perspectiva() {
    // Un vértice más lejano (w = 4): el atributo debe interpolarse como (Σ λ·a/w) / (Σ λ/w).
    let mut rt = RenderTarget::new(32, 32);
    let w = [1.0f32, 4.0, 1.0];
    let pos = [[-1.0, -1.0, 0.5, 1.0], [4.0, -4.0, 2.0, 4.0], [-1.0, 1.0, 0.5, 1.0]]; // x/w, y/w = (1,-1)
    let attr = [0.0f32, 1.0, 0.5];
    let v: Vec<_> = (0..3).map(|i| (pos[i], [attr[i], 0.0, 0.0, 1.0])).collect();
    run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&v), 3, State::new(32, 32), &[], &[]);
    // vértices en pantalla (ndc → píxeles)
    let ndc = [(-1.0f64, -1.0f64), (1.0, -1.0), (-1.0, 1.0)];
    let scr: Vec<(f64, f64)> = ndc.iter().map(|(x, y)| ((x * 0.5 + 0.5) * 32.0, (0.5 - y * 0.5) * 32.0)).collect();
    let mut checked = 0;
    for py in 0..32u32 {
        for px in 0..32u32 {
            let c = (px as f64 + 0.5, py as f64 + 0.5);
            let area = (scr[1].0 - scr[0].0) * (scr[2].1 - scr[0].1) - (scr[2].0 - scr[0].0) * (scr[1].1 - scr[0].1);
            let l1 = ((c.0 - scr[0].0) * (scr[2].1 - scr[0].1) - (scr[2].0 - scr[0].0) * (c.1 - scr[0].1)) / area;
            let l2 = ((scr[1].0 - scr[0].0) * (c.1 - scr[0].1) - (c.0 - scr[0].0) * (scr[1].1 - scr[0].1)) / area;
            let l0 = 1.0 - l1 - l2;
            if l0 < 0.05 || l1 < 0.05 || l2 < 0.05 {
                continue;
            }
            let (a, ww) = (l0 * attr[0] as f64 / w[0] as f64 + l1 * attr[1] as f64 / w[1] as f64 + l2 * attr[2] as f64 / w[2] as f64, l0 / w[0] as f64 + l1 / w[1] as f64 + l2 / w[2] as f64);
            let want = (a / ww * 255.0).round() as i32;
            assert!((rt.pixel(px, py)[0] as i32 - want).abs() <= 1, "({px},{py}): {} vs {want}", rt.pixel(px, py)[0]);
            checked += 1;
        }
    }
    assert!(checked > 200);
}

#[test]
fn prueba_de_profundidad_independiente_del_orden() {
    let far = ([1.0, 0.0, 0.0, 1.0], 0.8f32);
    let near = ([0.0, 1.0, 0.0, 1.0], 0.2f32);
    for order in [[far, near], [near, far]] {
        let mut rt = RenderTarget::new(8, 8);
        let mut st = State::new(8, 8);
        st.depth_test = true;
        for (color, z) in order {
            let tri = [[-1.0, -1.0, z, 1.0], [3.0, -1.0, z, 1.0], [-1.0, 3.0, z, 1.0]];
            run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&tri.map(|t| (t, color))), 3, st, &[], &[]);
        }
        assert_eq!(rt.pixel(4, 4), [0, 255, 0, 255]);
        assert!((rt.depth[4 * 8 + 4] - 0.2).abs() < 1e-6);
    }
}

#[test]
fn mezcla_alfa_y_mascara_de_color() {
    let mut rt = RenderTarget::new(4, 4);
    rt.clear_color([0.0, 0.0, 1.0, 1.0]);
    let mut st = State::new(4, 4);
    st.blend = Blend::ALPHA;
    let tri = [[-1.0, -1.0, 0.5, 1.0], [3.0, -1.0, 0.5, 1.0], [-1.0, 3.0, 0.5, 1.0]];
    run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&tri.map(|t| (t, [1.0, 0.0, 0.0, 0.5]))), 3, st, &[], &[]);
    let c = rt.pixel(2, 2);
    assert!((c[0] as i32 - 128).abs() <= 1 && c[1] == 0 && (c[2] as i32 - 128).abs() <= 1, "{c:?}");
    // máscara: solo se escribe el canal verde
    let mut rt = RenderTarget::new(4, 4);
    let mut st = State::new(4, 4);
    st.color_mask = [false, true, false, false];
    run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&tri.map(|t| (t, [1.0, 1.0, 1.0, 1.0]))), 3, st, &[], &[]);
    assert_eq!(rt.pixel(1, 1), [0, 255, 0, 0]);
}

#[test]
fn caras_traseras_y_recorte() {
    let ccw = [[-1.0, -1.0, 0.5, 1.0], [1.0, -1.0, 0.5, 1.0], [0.0, 1.0, 0.5, 1.0]];
    let mut cw = ccw;
    cw.swap(1, 2);
    let mut st = State::new(16, 16);
    st.cull = CullMode::Back;
    let mut rt = RenderTarget::new(16, 16);
    run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&ccw.map(|t| (t, WHITE))), 3, st, &[], &[]);
    let front = rt.color.iter().filter(|c| c[3] == 255).count();
    assert!(front > 50);
    let mut rt = RenderTarget::new(16, 16);
    run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&cw.map(|t| (t, WHITE))), 3, st, &[], &[]);
    assert_eq!(rt.color.iter().filter(|c| c[3] == 255).count(), 0);
    // un vértice detrás de la cámara (w < 0) se recorta sin fallar ni dibujar basura fuera del triángulo visible
    let mut rt = RenderTarget::new(16, 16);
    let tri = [[-0.5, -0.5, 0.5, 1.0], [0.5, -0.5, 0.5, 1.0], [0.0, -0.5, -1.0, -1.0]];
    run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&tri.map(|t| (t, WHITE))), 3, State::new(16, 16), &[], &[]);
    let lit = rt.color.iter().filter(|c| c[3] == 255).count();
    assert!(lit > 30 && lit < 256, "{lit}"); // cubre parte de la pantalla, sin desbordarse
    // z fuera del plano cercano (z < 0 en [0, w]) se recorta por completo
    let mut rt = RenderTarget::new(16, 16);
    let tri = [[-1.0, -1.0, -0.5, 1.0], [3.0, -1.0, -0.5, 1.0], [-1.0, 3.0, -0.5, 1.0]];
    run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&tri.map(|t| (t, WHITE))), 3, State::new(16, 16), &[], &[]);
    assert!(rt.color.iter().all(|c| c[3] == 0));
}

#[test]
fn scissor_limita_el_dibujo() {
    let mut rt = RenderTarget::new(16, 16);
    let mut st = State::new(16, 16);
    st.scissor = Some([4, 4, 4, 8]);
    let tri = [[-1.0, -1.0, 0.5, 1.0], [3.0, -1.0, 0.5, 1.0], [-1.0, 3.0, 0.5, 1.0]];
    run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&tri.map(|t| (t, WHITE))), 3, st, &[], &[]);
    let lit: Vec<(u32, u32)> = (0..16).flat_map(|y| (0..16).map(move |x| (x, y))).filter(|&(x, y)| rt.pixel(x, y)[3] == 255).collect();
    assert_eq!(lit.len(), 32);
    assert!(lit.iter().all(|&(x, y)| (4..8).contains(&x) && (4..12).contains(&y)));
}

fn checker() -> Texture {
    // 2x2: rojo, verde / azul, blanco
    Texture { width: 2, height: 2, rgba: vec![[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 255, 255]], filter: Filter::Nearest, wrap: Wrap::Repeat }
}

#[test]
fn texturas_vecino_mas_cercano_bilineal_y_envoltura() {
    let vs = assemble(Stage::Vertex, "MOV o0, i0; MOV o1, i1;").unwrap();
    let fs = assemble(Stage::Fragment, "TEX o0, i1, t0;").unwrap();
    // cuadrilátero a pantalla completa con uv = (0..1)
    let q = [([-1.0, 1.0, 0.5, 1.0], [0.0, 0.0, 0.0, 1.0]), ([-1.0, -1.0, 0.5, 1.0], [0.0, 1.0, 0.0, 1.0]), ([1.0, -1.0, 0.5, 1.0], [1.0, 1.0, 0.0, 1.0]),
             ([-1.0, 1.0, 0.5, 1.0], [0.0, 0.0, 0.0, 1.0]), ([1.0, -1.0, 0.5, 1.0], [1.0, 1.0, 0.0, 1.0]), ([1.0, 1.0, 0.5, 1.0], [1.0, 0.0, 0.0, 1.0])];
    let mut rt = RenderTarget::new(8, 8);
    run_draw(&mut rt, &vs, &fs, verts(&q), 6, State::new(8, 8), &[], &[Some(checker())]);
    assert_eq!(rt.pixel(1, 1), [255, 0, 0, 255]);
    assert_eq!(rt.pixel(6, 1), [0, 255, 0, 255]);
    assert_eq!(rt.pixel(1, 6), [0, 0, 255, 255]);
    assert_eq!(rt.pixel(6, 6), [255, 255, 255, 255]);
    // bilineal: el centro de la imagen mezcla los cuatro texels
    let mut lin = checker();
    lin.filter = Filter::Linear;
    lin.wrap = Wrap::Clamp;
    let mut rt = RenderTarget::new(8, 8);
    run_draw(&mut rt, &vs, &fs, verts(&q), 6, State::new(8, 8), &[], &[Some(lin.clone())]);
    let c = rt.pixel(4, 4);
    assert!((c[0] as i32 - 128).abs() <= 40 && (c[1] as i32 - 128).abs() <= 40 && c[3] == 255, "{c:?}");
    // envoltura: u en [0,2] repite la textura
    let mut st = lin.clone();
    st.filter = Filter::Nearest;
    st.wrap = Wrap::Repeat;
    assert_eq!(st.sample(1.25, 0.25), st.sample(0.25, 0.25));
    st.wrap = Wrap::Clamp;
    assert_eq!(st.sample(1.25, 0.25), st.sample(0.99, 0.25));
    st.wrap = Wrap::Mirror;
    assert_eq!(st.sample(1.25, 0.25), st.sample(0.75, 0.25));
}

fn eval(text: &str, inputs: &[[f32; 4]], consts: &[[f32; 4]]) -> Vec<[f32; 4]> {
    interp::run(&assemble(Stage::Fragment, text).unwrap(), inputs, consts, &NoTextures)
}

#[test]
fn cada_operacion_del_interprete() {
    let c = [[1.0, -2.0, 3.0, -4.0], [0.5, 0.5, 0.5, 0.5], [9.0, 4.0, 1.0, 16.0]];
    let r = |t: &str| eval(t, &[], &c)[0];
    assert_eq!(r("MOV o0, c0;"), [1.0, -2.0, 3.0, -4.0]);
    assert_eq!(r("ADD o0, c0, c1;"), [1.5, -1.5, 3.5, -3.5]);
    assert_eq!(r("SUB o0, c0, c1;"), [0.5, -2.5, 2.5, -4.5]);
    assert_eq!(r("MUL o0, c0, c2;"), [9.0, -8.0, 3.0, -64.0]);
    assert_eq!(r("MAD o0, c0, c2, c1;"), [9.5, -7.5, 3.5, -63.5]);
    assert_eq!(r("DP3 o0, c0, c2;"), [4.0; 4]); // 9 - 8 + 3
    assert_eq!(r("DP4 o0, c0, c2;"), [-60.0; 4]);
    assert_eq!(r("RCP o0, c1;"), [2.0; 4]);
    assert_eq!(r("RSQ o0, c2;"), [1.0 / 3.0; 4]);
    assert_eq!(r("MIN o0, c0, c1;"), [0.5, -2.0, 0.5, -4.0]);
    assert_eq!(r("MAX o0, c0, c1;"), [1.0, 0.5, 3.0, 0.5]);
    assert_eq!(r("FLR o0, {1.5, -1.5, 2.0, -0.25};"), [1.0, -2.0, 2.0, -1.0]);
    assert_eq!(r("FRC o0, {1.5, -1.5, 2.0, -0.25};"), [0.5, 0.5, 0.0, 0.75]);
    assert_eq!(r("SLT o0, c0, c1;"), [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(r("SGE o0, c0, c1;"), [1.0, 0.0, 1.0, 0.0]);
    assert_eq!(r("CMP o0, c0, {10.0,10.0,10.0,10.0}, {20.0,20.0,20.0,20.0};"), [20.0, 10.0, 20.0, 10.0]);
    assert_eq!(r("EX2 o0, {3.0, 0, 0, 0};"), [8.0; 4]);
    assert_eq!(r("LG2 o0, {8.0, 0, 0, 0};"), [3.0; 4]);
    // intercambio, negación, valor absoluto, saturación y máscara de escritura
    assert_eq!(r("MOV o0, c0.wzyx;"), [-4.0, 3.0, -2.0, 1.0]);
    assert_eq!(r("MOV o0, c0.xxyy;"), [1.0, 1.0, -2.0, -2.0]);
    assert_eq!(r("MOV o0, -|c0|;"), [-1.0, -2.0, -3.0, -4.0]);
    assert_eq!(r("MOV_SAT o0, c0;"), [1.0, 0.0, 1.0, 0.0]);
    assert_eq!(r("MOV o0, {7, 7, 7, 7}; MOV o0.yw, c0;"), [7.0, -2.0, 7.0, -4.0]);
    assert_eq!(r("MOV r0, c0; ADD o0, r0, r0;"), [2.0, -4.0, 6.0, -8.0]);
    // entradas
    assert_eq!(eval("MUL o0, i0, i1;", &[[1.0, 2.0, 3.0, 4.0], [2.0, 2.0, 2.0, 2.0]], &[])[0], [2.0, 4.0, 6.0, 8.0]);
}

#[test]
fn el_ensamblador_detecta_errores_y_ida_y_vuelta() {
    assert!(assemble(Stage::Vertex, "FOO o0, i0;").unwrap_err().contains("línea 1"));
    assert!(assemble(Stage::Vertex, "ADD o0, i0;").is_err());
    assert!(assemble(Stage::Vertex, "MOV o0, q0;").is_err());
    assert!(assemble(Stage::Fragment, "TEX o0, i1, x0;").is_err());
    for src in [
        "MOV o0, i0; MOV o1, i1;",
        "MAD_SAT r0.xyz, -i1, c0.xxxx, {0.5, 0.5, 0.5, 0.0}; MOV o0, r0;",
        "TEX r0, i1.xy, t1; MUL o0.xyz, r0, c3.wwww;",
        "DP4 r1, |i0|, -c2; RCP r2, r1; CMP o0, r2, i1, c0;",
    ] {
        let s = assemble(Stage::Fragment, src).unwrap();
        let again = assemble(Stage::Fragment, &disassemble(&s)).unwrap();
        assert_eq!(s, again, "{src}");
        assert_eq!(Shader::from_bytes(Stage::Fragment, &s.to_bytes()).unwrap(), s);
    }
}

#[test]
fn aristas_horizontales_y_verticales_que_pasan_por_centros_de_pixel() {
    // En un destino de 8x8 las líneas y = -0.125 y x = 0.125 (NDC) pasan exactamente por centros de píxel.
    // Dos rectángulos que comparten esa arista deben cubrir cada píxel una sola vez.
    let add = Blend { enabled: true, src_rgb: BlendFactor::One, dst_rgb: BlendFactor::One, op_rgb: BlendOp::Add, src_a: BlendFactor::One, dst_a: BlendFactor::One, op_a: BlendOp::Add };
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32| [(x0, y1), (x0, y0), (x1, y0), (x0, y1), (x1, y0), (x1, y1)];
    for (name, rects) in [
        ("horizontal", [rect(-1.0, -0.125, 1.0, 1.0), rect(-1.0, -1.0, 1.0, -0.125)]),
        ("vertical", [rect(-1.0, -1.0, 0.125, 1.0), rect(0.125, -1.0, 1.0, 1.0)]),
    ] {
        let mut rt = RenderTarget::new(8, 8);
        let mut st = State::new(8, 8);
        st.blend = add;
        for r in rects {
            run_draw(&mut rt, &vs_pass(), &fs_const(), verts(&r.map(|(x, y)| (p(x, y), [0.0; 4]))), 6, st, &[[0.5, 0.5, 0.5, 0.5]], &[]);
        }
        let bad: Vec<_> = (0..8).flat_map(|y| (0..8).map(move |x| (x, y))).filter(|&(x, y)| rt.pixel(x, y)[0] != 128).collect();
        assert!(bad.is_empty(), "arista {name}: píxeles con cobertura distinta de uno: {bad:?}");
    }
}

#[test]
fn convencion_arriba_izquierda_en_un_solo_triangulo() {
    // y = 2.5 (NDC 0.375) es el borde superior del triángulo: la fila 2 se incluye.
    // y = 5.5 (NDC -0.375) es el borde inferior del otro: la fila 5 se excluye.
    let mut rt = RenderTarget::new(8, 8);
    let top = [(-1.0, 0.375), (1.0, 0.375), (0.0, -1.0)];
    run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&top.map(|(x, y)| (p(x, y), WHITE))), 3, State::new(8, 8), &[], &[]);
    assert_eq!(rt.pixel(4, 2)[3], 255, "el borde superior se incluye");
    let mut rt = RenderTarget::new(8, 8);
    let bottom = [(-1.0, -0.375), (1.0, -0.375), (0.0, 1.0)];
    run_draw(&mut rt, &vs_pass(), &fs_color(), verts(&bottom.map(|(x, y)| (p(x, y), WHITE))), 3, State::new(8, 8), &[], &[]);
    assert_eq!(rt.pixel(4, 5)[3], 0, "el borde inferior se excluye");
    assert_eq!(rt.pixel(4, 4)[3], 255);
}
