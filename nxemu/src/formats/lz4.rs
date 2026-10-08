//! Descompresión de bloques LZ4 (formato de bloque, sin cabecera de marco).

#[derive(Debug, PartialEq, Eq)]
pub enum Lz4Error {
    Truncated,
    BadOffset,
    TooLong,
    WrongSize { expected: usize, got: usize },
}

/// Descomprime un bloque LZ4. `expected` es el tamaño descomprimido esperado (se comprueba al final).
pub fn decompress(src: &[u8], expected: usize) -> Result<Vec<u8>, Lz4Error> {
    let mut out: Vec<u8> = Vec::with_capacity(expected);
    let mut i = 0;
    let read_len = |i: &mut usize, mut n: usize| -> Result<usize, Lz4Error> {
        if n == 15 {
            loop {
                let b = *src.get(*i).ok_or(Lz4Error::Truncated)?;
                *i += 1;
                n += b as usize;
                if b != 255 {
                    break;
                }
            }
        }
        Ok(n)
    };
    while i < src.len() {
        let token = src[i];
        i += 1;
        let lit = read_len(&mut i, (token >> 4) as usize)?;
        let lits = src.get(i..i + lit).ok_or(Lz4Error::Truncated)?;
        if out.len() + lit > expected {
            return Err(Lz4Error::TooLong);
        }
        out.extend_from_slice(lits);
        i += lit;
        if i >= src.len() {
            break; // la última secuencia solo tiene literales
        }
        let offset = u16::from_le_bytes(src.get(i..i + 2).ok_or(Lz4Error::Truncated)?.try_into().unwrap()) as usize;
        i += 2;
        if offset == 0 || offset > out.len() {
            return Err(Lz4Error::BadOffset);
        }
        let mlen = read_len(&mut i, (token & 15) as usize)? + 4;
        if out.len() + mlen > expected {
            return Err(Lz4Error::TooLong);
        }
        let start = out.len() - offset;
        for k in 0..mlen {
            let b = out[start + k]; // las coincidencias pueden solaparse con lo que se acaba de escribir
            out.push(b);
        }
    }
    if out.len() != expected {
        return Err(Lz4Error::WrongSize { expected, got: out.len() });
    }
    Ok(out)
}

/// Compresor LZ4 sencillo (búsqueda voraz con tabla hash) para generar datos de prueba y empaquetar NSO.
pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut table = vec![usize::MAX; 1 << 14];
    let (mut i, mut anchor) = (0usize, 0usize);
    let hash = |b: &[u8]| ((u32::from_le_bytes(b[..4].try_into().unwrap()).wrapping_mul(2654435761)) >> 18) as usize;
    let push_len = |out: &mut Vec<u8>, mut n: usize| {
        while n >= 255 {
            out.push(255);
            n -= 255;
        }
        out.push(n as u8);
    };
    // las últimas 12 bytes se dejan como literales (requisito del formato)
    while i + 12 < data.len() {
        let h = hash(&data[i..]);
        let cand = table[h];
        table[h] = i;
        if cand != usize::MAX && i - cand <= 65535 && data[cand..cand + 4] == data[i..i + 4] {
            let mut m = 4;
            while i + m < data.len() - 5 && data[cand + m] == data[i + m] {
                m += 1;
            }
            let lit = i - anchor;
            let token = ((lit.min(15) as u8) << 4) | ((m - 4).min(15) as u8);
            out.push(token);
            if lit >= 15 {
                push_len(&mut out, lit - 15);
            }
            out.extend_from_slice(&data[anchor..i]);
            out.extend_from_slice(&((i - cand) as u16).to_le_bytes());
            if m - 4 >= 15 {
                push_len(&mut out, m - 4 - 15);
            }
            i += m;
            anchor = i;
        } else {
            i += 1;
        }
    }
    let lit = data.len() - anchor;
    out.push((lit.min(15) as u8) << 4);
    if lit >= 15 {
        push_len(&mut out, lit - 15);
    }
    out.extend_from_slice(&data[anchor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_literales() {
        let mut b = vec![0x50];
        b.extend_from_slice(b"hello");
        assert_eq!(decompress(&b, 5).unwrap(), b"hello");
    }

    #[test]
    fn coincidencia_solapada() {
        // 3 literales "abc", coincidencia de longitud 9 con desplazamiento 3 → "abcabcabcabc"
        let b = [0x35, b'a', b'b', b'c', 3, 0];
        assert_eq!(decompress(&b, 12).unwrap(), b"abcabcabcabc");
    }

    #[test]
    fn longitudes_extendidas() {
        // 20 literales (15 + 5) seguidos de nada
        let mut b = vec![0xF0, 5];
        b.extend((0..20u8).collect::<Vec<_>>());
        assert_eq!(decompress(&b, 20).unwrap(), (0..20u8).collect::<Vec<_>>());
        // coincidencia larga: 1 literal 'x', desplazamiento 1, longitud 4 + 15 + 255 + 3 = 277
        let b = [0x1F, b'x', 1, 0, 255, 3, 0];
        let mut b = b.to_vec();
        b.truncate(6); // token, 'x', offset(2), extensiones 255 y 3
        let out = decompress(&b, 1 + 4 + 15 + 255 + 3).unwrap();
        assert!(out.iter().all(|c| *c == b'x') && out.len() == 278);
    }

    #[test]
    fn errores() {
        assert_eq!(decompress(&[0x50, b'h'], 5), Err(Lz4Error::Truncated));
        assert_eq!(decompress(&[0x11, b'a', 5, 0], 9), Err(Lz4Error::BadOffset));
        assert_eq!(decompress(&[0x11, b'a', 0, 0], 9), Err(Lz4Error::BadOffset));
        assert_eq!(decompress(&[0x30, b'a', b'b', b'c'], 2), Err(Lz4Error::TooLong));
        assert!(matches!(decompress(&[0x30, b'a', b'b', b'c'], 9), Err(Lz4Error::WrongSize { expected: 9, got: 3 })));
    }

    #[test]
    fn ida_y_vuelta_con_datos_variados() {
        let mut s = 1u64;
        let mut rnd = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        for len in [0usize, 1, 11, 12, 13, 100, 4096, 70_000] {
            // datos con repeticiones, ceros y ruido
            let data: Vec<u8> = (0..len).map(|i| if i % 97 < 60 { (i % 7) as u8 } else { (rnd() & 0xFF) as u8 }).collect();
            let c = compress(&data);
            assert_eq!(decompress(&c, len).unwrap(), data, "longitud {len}");
        }
        let zeros = vec![0u8; 100_000];
        let c = compress(&zeros);
        assert!(c.len() < 1000, "los ceros deben comprimirse bien: {}", c.len());
        assert_eq!(decompress(&c, zeros.len()).unwrap(), zeros);
    }
}
