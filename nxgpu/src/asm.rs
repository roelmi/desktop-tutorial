//! Ensamblador y desensamblador de texto del IR de shaders.
//!
//! ```text
//! MAD_SAT r0.xyz, -i1, c0.xxxx, {0.5, 0.5, 0.5, 0};
//! TEX o0, i1, t0;
//! ```

use crate::ir::*;

fn parse_index(s: &str, prefix: char) -> Option<u8> {
    s.strip_prefix(prefix)?.parse().ok()
}

fn swizzle(s: &str) -> Result<[u8; 4], String> {
    let comp = |c: char| match c {
        'x' | 'r' => Ok(0),
        'y' | 'g' => Ok(1),
        'z' | 'b' => Ok(2),
        'w' | 'a' => Ok(3),
        _ => Err(format!("componente «{c}» inválido")),
    };
    let cs: Vec<char> = s.chars().collect();
    if cs.is_empty() || cs.len() > 4 {
        return Err(format!("máscara de intercambio «{s}» inválida"));
    }
    // un intercambio corto replica su último componente (.x = .xxxx, .xy = .xyyy)
    let mut out = [0u8; 4];
    for k in 0..4 {
        out[k] = comp(cs[k.min(cs.len() - 1)])?;
    }
    Ok(out)
}

fn parse_operand(tok: &str) -> Result<Operand, String> {
    let mut t = tok.trim();
    let mut neg = false;
    if let Some(r) = t.strip_prefix('-') {
        neg = true;
        t = r.trim();
    }
    let mut abs = false;
    if t.starts_with('|') && t.ends_with('|') && t.len() > 2 {
        abs = true;
        t = &t[1..t.len() - 1];
    }
    if let Some(inner) = t.strip_prefix('{') {
        let inner = inner.strip_suffix('}').ok_or("falta «}»")?;
        let v: Vec<f32> = inner.split(',').map(|x| x.trim().parse::<f32>().map_err(|e| format!("{x}: {e}"))).collect::<Result<_, _>>()?;
        if v.len() != 4 {
            return Err("un literal necesita 4 componentes".into());
        }
        return Ok(Operand { src: Src::Imm([v[0], v[1], v[2], v[3]]), swizzle: [0, 1, 2, 3], neg, abs });
    }
    if let Ok(f) = t.parse::<f32>() {
        return Ok(Operand { src: Src::Imm([f; 4]), swizzle: [0, 1, 2, 3], neg, abs });
    }
    let (reg, sw) = match t.split_once('.') {
        Some((r, s)) => (r, swizzle(s)?),
        None => (t, [0, 1, 2, 3]),
    };
    let src = if let Some(n) = parse_index(reg, 'r') {
        Src::Temp(n)
    } else if let Some(n) = parse_index(reg, 'i') {
        Src::Input(n)
    } else if let Some(n) = parse_index(reg, 'c') {
        Src::Const(n)
    } else {
        return Err(format!("operando «{tok}» inválido"));
    };
    Ok(Operand { src, swizzle: sw, neg, abs })
}

fn parse_dst(tok: &str) -> Result<Dst, String> {
    let t = tok.trim();
    let (reg, mask) = match t.split_once('.') {
        Some((r, m)) => {
            let mut bits = 0u8;
            for c in m.chars() {
                bits |= 1 << match c {
                    'x' | 'r' => 0,
                    'y' | 'g' => 1,
                    'z' | 'b' => 2,
                    'w' | 'a' => 3,
                    _ => return Err(format!("máscara «{m}» inválida")),
                };
            }
            (r, bits)
        }
        None => (t, 0xF),
    };
    let kind = if let Some(n) = parse_index(reg, 'r') {
        DstKind::Temp(n)
    } else if let Some(n) = parse_index(reg, 'o') {
        DstKind::Output(n)
    } else {
        return Err(format!("destino «{tok}» inválido"));
    };
    Ok(Dst { kind, mask })
}

const OPS: &[(&str, Op)] = &[
    ("MOV", Op::Mov), ("ADD", Op::Add), ("SUB", Op::Sub), ("MUL", Op::Mul), ("MAD", Op::Mad), ("DP3", Op::Dp3), ("DP4", Op::Dp4),
    ("RCP", Op::Rcp), ("RSQ", Op::Rsq), ("MIN", Op::Min), ("MAX", Op::Max), ("FLR", Op::Flr), ("FRC", Op::Frc), ("SLT", Op::Slt),
    ("SGE", Op::Sge), ("CMP", Op::Cmp), ("EX2", Op::Ex2), ("LG2", Op::Lg2), ("TEX", Op::Tex),
];

pub fn assemble(stage: Stage, text: &str) -> Result<Shader, String> {
    let mut code = Vec::new();
    let (mut temps, mut inputs, mut outputs, mut consts, mut textures) = (0u8, 0u8, 0u8, 0u8, 0u8);
    // Cada línea puede contener varias instrucciones separadas por «;».
    let stmts = text.lines().enumerate().flat_map(|(ln, raw)| raw.split('#').next().unwrap().split(';').map(move |s| (ln, s.trim())).collect::<Vec<_>>());
    for (ln, line) in stmts {
        if line.is_empty() || line.starts_with("!!") {
            continue;
        }
        let err = |m: String| format!("línea {}: {m}", ln + 1);
        let (mnemonic, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let (name, sat) = match mnemonic.strip_suffix("_SAT") {
            Some(n) => (n, true),
            None => (mnemonic, false),
        };
        let op = OPS.iter().find(|(n, _)| *n == name).map(|(_, o)| *o).ok_or_else(|| err(format!("instrucción «{name}» desconocida")))?;
        // separar por comas que no estén dentro de llaves
        let (mut parts, mut depth, mut cur) = (Vec::new(), 0, String::new());
        for c in rest.chars() {
            match c {
                '{' => { depth += 1; cur.push(c) }
                '}' => { depth -= 1; cur.push(c) }
                ',' if depth == 0 => parts.push(std::mem::take(&mut cur)),
                _ => cur.push(c),
            }
        }
        parts.push(cur);
        let n = op.num_srcs();
        let expected = if op == Op::Tex { 3 } else { 1 + n };
        if parts.len() != expected {
            return Err(err(format!("{name} espera {expected} operandos")));
        }
        let dst = parse_dst(&parts[0]).map_err(&err)?;
        let mut srcs = [Operand::new(Src::Imm([0.0; 4])); 3];
        let mut tex = 0u8;
        if op == Op::Tex {
            // TEX dst, coord, tN
            srcs[0] = parse_operand(&parts[1]).map_err(&err)?;
            tex = parse_index(parts[2].trim(), 't').ok_or_else(|| err("TEX espera una unidad «tN»".into()))?;
            textures = textures.max(tex + 1);
        } else {
            for k in 0..n {
                srcs[k] = parse_operand(&parts[1 + k]).map_err(&err)?;
            }
        }
        match dst.kind {
            DstKind::Temp(r) => temps = temps.max(r + 1),
            DstKind::Output(o) => outputs = outputs.max(o + 1),
        }
        for s in srcs.iter().take(op.num_srcs().max(1)) {
            match s.src {
                Src::Temp(r) => temps = temps.max(r + 1),
                Src::Input(i) => inputs = inputs.max(i + 1),
                Src::Const(c) => consts = consts.max(c + 1),
                Src::Imm(_) => {}
            }
        }
        code.push(Inst { op, dst, srcs, saturate: sat, tex });
    }
    Ok(Shader { stage, num_temps: temps, num_inputs: inputs, num_outputs: outputs, num_consts: consts, num_textures: textures, code })
}

fn comps(sw: &[u8; 4]) -> String {
    sw.iter().map(|c| ['x', 'y', 'z', 'w'][*c as usize]).collect()
}

pub fn disassemble(s: &Shader) -> String {
    let mut out = String::from(match s.stage {
        Stage::Vertex => "!!VS\n",
        Stage::Fragment => "!!FS\n",
    });
    for i in &s.code {
        let name = OPS.iter().find(|(_, o)| *o == i.op).unwrap().0;
        let dst = match i.dst.kind {
            DstKind::Temp(r) => format!("r{r}"),
            DstKind::Output(o) => format!("o{o}"),
        };
        let mask: String = (0..4).filter(|b| i.dst.mask >> b & 1 == 1).map(|b| ['x', 'y', 'z', 'w'][b]).collect();
        let mut line = format!("{name}{} {dst}.{mask}", if i.saturate { "_SAT" } else { "" });
        for k in 0..i.op.num_srcs() {
            let o = &i.srcs[k];
            let mut t = match o.src {
                Src::Temp(r) => format!("r{r}.{}", comps(&o.swizzle)),
                Src::Input(r) => format!("i{r}.{}", comps(&o.swizzle)),
                Src::Const(r) => format!("c{r}.{}", comps(&o.swizzle)),
                Src::Imm(v) => format!("{{{:?}, {:?}, {:?}, {:?}}}", v[0], v[1], v[2], v[3]),
            };
            if o.abs {
                t = format!("|{t}|");
            }
            if o.neg {
                t = format!("-{t}");
            }
            line += &format!(", {t}");
        }
        if i.op == Op::Tex {
            line += &format!(", t{}", i.tex);
        }
        out += &line;
        out += ";\n";
    }
    out
}
