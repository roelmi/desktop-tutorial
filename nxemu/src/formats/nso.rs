//! Ejecutables NSO0: segmentos `.text`, `.rodata` y `.data` (opcionalmente comprimidos con LZ4 y con hash SHA-256),
//! más el tamaño de `.bss`. Es el formato de `main`, `rtld` y `subsdk*` dentro de un ExeFS.

use super::{lz4, sha256::sha256};
use crate::memory::{Memory, PAGE_SIZE, PERM_R, PERM_RW, PERM_RX};

#[derive(Debug, PartialEq, Eq)]
pub enum NsoError {
    TooShort,
    BadMagic,
    Segment(&'static str),
    Lz4(&'static str, lz4::Lz4Error),
    HashMismatch(&'static str),
    Memory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nso {
    pub text: Vec<u8>,
    pub ro: Vec<u8>,
    pub data: Vec<u8>,
    pub text_off: u32,
    pub ro_off: u32,
    pub data_off: u32,
    pub bss_size: u32,
    pub build_id: [u8; 32],
}

fn u32_at(d: &[u8], o: usize) -> Result<u32, NsoError> {
    d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or(NsoError::TooShort)
}

/// Analiza un NSO, descomprime sus segmentos y, si el archivo lo indica, comprueba los hashes.
pub fn parse(d: &[u8]) -> Result<Nso, NsoError> {
    if d.len() < 0x100 {
        return Err(NsoError::TooShort);
    }
    if &d[0..4] != b"NSO0" {
        return Err(NsoError::BadMagic);
    }
    let flags = u32_at(d, 0x0C)?;
    let names = ["text", "ro", "data"];
    let mut segs = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let h = 0x10 + i * 0x10;
        let (file_off, mem_off, size) = (u32_at(d, h)? as usize, u32_at(d, h + 4)?, u32_at(d, h + 8)? as usize);
        let comp_size = u32_at(d, 0x60 + 4 * i)? as usize;
        let compressed = flags >> i & 1 == 1;
        let raw_len = if compressed { comp_size } else { size };
        let raw = d.get(file_off..file_off.checked_add(raw_len).ok_or(NsoError::Segment(name))?).ok_or(NsoError::Segment(name))?;
        let bytes = if compressed { lz4::decompress(raw, size).map_err(|e| NsoError::Lz4(name, e))? } else { raw.to_vec() };
        if flags >> (3 + i) & 1 == 1 && sha256(&bytes)[..] != d[0xA0 + 0x20 * i..0xA0 + 0x20 * (i + 1)] {
            return Err(NsoError::HashMismatch(name));
        }
        segs.push((mem_off, bytes));
    }
    let mut build_id = [0u8; 32];
    build_id.copy_from_slice(&d[0x40..0x60]);
    let (data, ro, text) = (segs.pop().unwrap(), segs.pop().unwrap(), segs.pop().unwrap());
    Ok(Nso { text_off: text.0, ro_off: ro.0, data_off: data.0, text: text.1, ro: ro.1, data: data.1, bss_size: u32_at(d, 0x3C)?, build_id })
}

impl Nso {
    /// Tamaño total que ocupa en memoria (segmentos + `.bss`), redondeado a páginas.
    pub fn image_size(&self) -> u64 {
        let end = (self.data_off as u64 + self.data.len() as u64).div_ceil(PAGE_SIZE) * PAGE_SIZE + self.bss_size as u64;
        end.div_ceil(PAGE_SIZE) * PAGE_SIZE
    }

    /// Carga el módulo en `base` con los permisos de cada segmento y devuelve el final de la imagen.
    pub fn load(&self, mem: &mut Memory, base: u64) -> Result<u64, NsoError> {
        let size = self.image_size();
        mem.map_perm(base, size, PERM_RW);
        for (off, bytes) in [(self.text_off, &self.text), (self.ro_off, &self.ro), (self.data_off, &self.data)] {
            mem.write(base + off as u64, bytes).map_err(|_| NsoError::Memory)?;
        }
        let page = |n: usize| (n as u64).div_ceil(PAGE_SIZE) * PAGE_SIZE;
        mem.protect(base + self.text_off as u64, page(self.text.len()), PERM_RX).map_err(|_| NsoError::Memory)?;
        if !self.ro.is_empty() {
            mem.protect(base + self.ro_off as u64, page(self.ro.len()), PERM_R).map_err(|_| NsoError::Memory)?;
        }
        Ok(base + size)
    }
}

/// Construye un NSO (para pruebas y para empaquetar): comprime con LZ4 y escribe los hashes.
pub fn build(text: &[u8], ro: &[u8], data: &[u8], bss_size: u32, compress: bool) -> Vec<u8> {
    let page = |n: usize| (n as u32).div_ceil(0x1000) * 0x1000;
    let (t_off, r_off) = (0u32, page(text.len()));
    let d_off = r_off + page(ro.len());
    let segs = [(text, t_off), (ro, r_off), (data, d_off)];
    let blobs: Vec<Vec<u8>> = segs.iter().map(|(s, _)| if compress { lz4::compress(s) } else { s.to_vec() }).collect();
    let mut out = vec![0u8; 0x100];
    out[0..4].copy_from_slice(b"NSO0");
    let flags = if compress { 0b111 } else { 0 } | 0b111000;
    out[0x0C..0x10].copy_from_slice(&(flags as u32).to_le_bytes());
    let mut file_off = 0x100u32;
    for (i, ((s, mem_off), blob)) in segs.iter().zip(&blobs).enumerate() {
        let h = 0x10 + i * 0x10;
        out[h..h + 4].copy_from_slice(&file_off.to_le_bytes());
        out[h + 4..h + 8].copy_from_slice(&mem_off.to_le_bytes());
        out[h + 8..h + 12].copy_from_slice(&(s.len() as u32).to_le_bytes());
        out[0x60 + 4 * i..0x64 + 4 * i].copy_from_slice(&(blob.len() as u32).to_le_bytes());
        out[0xA0 + 0x20 * i..0xC0 + 0x20 * i].copy_from_slice(&sha256(s));
        file_off += blob.len() as u32;
    }
    out[0x3C..0x40].copy_from_slice(&bss_size.to_le_bytes());
    for b in &blobs {
        out.extend_from_slice(b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta_con_y_sin_compresion() {
        let text: Vec<u8> = (0..5000u32).map(|i| (i % 13) as u8).collect();
        let ro = b"cadena de solo lectura\0".repeat(50);
        let data = vec![7u8; 300];
        for compress in [false, true] {
            let img = build(&text, &ro, &data, 0x2000, compress);
            let n = parse(&img).unwrap();
            assert_eq!((n.text, n.ro, n.data, n.bss_size), (text.clone(), ro.clone(), data.clone(), 0x2000));
        }
    }

    #[test]
    fn detecta_hash_corrupto_y_formato_invalido() {
        let mut img = build(&[1; 100], &[2; 100], &[3; 100], 0, false);
        let last = img.len() - 1;
        img[last] ^= 1; // un byte del segmento .data
        assert_eq!(parse(&img), Err(NsoError::HashMismatch("data")));
        assert_eq!(parse(&[0u8; 0x100]), Err(NsoError::BadMagic));
        assert_eq!(parse(b"NSO0"), Err(NsoError::TooShort));
        let mut img = build(&[1; 100], &[2; 100], &[3; 100], 0, true);
        img[0x10] = 0xFF; // desplazamiento de archivo del .text fuera de rango
        img[0x11] = 0xFF;
        img[0x12] = 0xFF;
        assert!(parse(&img).is_err());
    }

    #[test]
    fn carga_con_permisos_por_segmento() {
        let img = build(&[0x1F, 0x20, 0x03, 0xD5], &[9; 10], &[5; 10], 0x1000, true);
        let n = parse(&img).unwrap();
        let mut mem = Memory::new();
        let end = n.load(&mut mem, 0x0800_0000).unwrap();
        assert_eq!(mem.perm_at(0x0800_0000), Some(PERM_RX));
        assert_eq!(mem.perm_at(0x0800_1000), Some(PERM_R));
        assert_eq!(mem.perm_at(0x0800_2000), Some(PERM_RW));
        assert_eq!(mem.perm_at(0x0800_3000), Some(PERM_RW)); // .bss
        assert_eq!(end, 0x0800_4000);
        assert_eq!(mem.fetch32(0x0800_0000), Ok(0xD503_201F));
    }
}
