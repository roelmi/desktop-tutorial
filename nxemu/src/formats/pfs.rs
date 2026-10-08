//! Contenedores PFS0 (ExeFS y NSP sin cifrar) y HFS0 (partición de cartucho sin cifrar).

#[derive(Debug, PartialEq, Eq)]
pub enum PfsError {
    TooShort,
    BadMagic,
    OutOfRange { file: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    /// Desplazamiento absoluto de los datos en el archivo.
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Container {
    pub kind: Kind,
    pub files: Vec<Entry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Pfs0,
    Hfs0,
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()))
}
fn u64_at(d: &[u8], o: usize) -> Option<u64> {
    d.get(o..o + 8).map(|b| u64::from_le_bytes(b.try_into().unwrap()))
}

pub fn parse(d: &[u8]) -> Result<Container, PfsError> {
    let kind = match d.get(0..4) {
        Some(b"PFS0") => Kind::Pfs0,
        Some(b"HFS0") => Kind::Hfs0,
        Some(_) => return Err(PfsError::BadMagic),
        None => return Err(PfsError::TooShort),
    };
    let (n, strtab) = (u32_at(d, 4).ok_or(PfsError::TooShort)? as usize, u32_at(d, 8).ok_or(PfsError::TooShort)? as usize);
    let entry_size = if kind == Kind::Pfs0 { 0x18 } else { 0x40 };
    let strings_at = 0x10 + n * entry_size;
    let data_at = strings_at + strtab;
    let strings = d.get(strings_at..data_at).ok_or(PfsError::TooShort)?;
    let mut files = Vec::with_capacity(n);
    for i in 0..n {
        let e = 0x10 + i * entry_size;
        let (off, size, name_off) = (u64_at(d, e).ok_or(PfsError::TooShort)? as usize, u64_at(d, e + 8).ok_or(PfsError::TooShort)? as usize, u32_at(d, e + 16).ok_or(PfsError::TooShort)? as usize);
        let name: String = strings.get(name_off..).ok_or(PfsError::TooShort)?.iter().take_while(|b| **b != 0).map(|b| *b as char).collect();
        let (start, end) = (data_at.checked_add(off), data_at.checked_add(off).and_then(|s| s.checked_add(size)));
        match (start, end) {
            (Some(s), Some(e)) if e <= d.len() => files.push(Entry { name, offset: s, size }),
            _ => return Err(PfsError::OutOfRange { file: name }),
        }
    }
    Ok(Container { kind, files })
}

impl Container {
    pub fn get<'a>(&self, d: &'a [u8], name: &str) -> Option<&'a [u8]> {
        self.files.iter().find(|f| f.name == name).map(|f| &d[f.offset..f.offset + f.size])
    }
}

/// Construye un PFS0 (para pruebas y para empaquetar ExeFS sin cifrar).
pub fn build_pfs0(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut strings = Vec::new();
    let mut offs = Vec::new();
    for (n, _) in files {
        offs.push(strings.len() as u32);
        strings.extend_from_slice(n.as_bytes());
        strings.push(0);
    }
    while strings.len() % 0x20 != 0 {
        strings.push(0);
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"PFS0");
    out.extend_from_slice(&(files.len() as u32).to_le_bytes());
    out.extend_from_slice(&(strings.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    let mut data_off = 0u64;
    for (i, (_, data)) in files.iter().enumerate() {
        out.extend_from_slice(&data_off.to_le_bytes());
        out.extend_from_slice(&(data.len() as u64).to_le_bytes());
        out.extend_from_slice(&offs[i].to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        data_off += data.len() as u64;
    }
    out.extend_from_slice(&strings);
    for (_, data) in files {
        out.extend_from_slice(data);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pfs0_ida_y_vuelta() {
        let img = build_pfs0(&[("main", b"AAAA"), ("main.npdm", b"BB"), ("rtld", b"")]);
        let c = parse(&img).unwrap();
        assert_eq!(c.kind, Kind::Pfs0);
        assert_eq!(c.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["main", "main.npdm", "rtld"]);
        assert_eq!(c.get(&img, "main"), Some(&b"AAAA"[..]));
        assert_eq!(c.get(&img, "main.npdm"), Some(&b"BB"[..]));
        assert_eq!(c.get(&img, "rtld"), Some(&b""[..]));
        assert_eq!(c.get(&img, "nada"), None);
    }

    #[test]
    fn hfs0_con_entradas_de_64_bytes() {
        // un HFS0 a mano: 1 archivo «secure» de 3 bytes
        let mut d = Vec::new();
        d.extend_from_slice(b"HFS0");
        d.extend_from_slice(&1u32.to_le_bytes());
        d.extend_from_slice(&0x20u32.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&0u64.to_le_bytes());
        d.extend_from_slice(&3u64.to_le_bytes());
        d.extend_from_slice(&0u32.to_le_bytes()); // desplazamiento del nombre
        d.extend_from_slice(&0u32.to_le_bytes());
        d.extend_from_slice(&[0u8; 8]);
        d.extend_from_slice(&[0u8; 0x20]); // hash
        let mut strings = b"secure\0".to_vec();
        strings.resize(0x20, 0);
        d.extend_from_slice(&strings);
        d.extend_from_slice(b"xyz");
        let c = parse(&d).unwrap();
        assert_eq!(c.kind, Kind::Hfs0);
        assert_eq!(c.get(&d, "secure"), Some(&b"xyz"[..]));
    }

    #[test]
    fn rechaza_entradas_corruptas() {
        assert_eq!(parse(b"PF"), Err(PfsError::TooShort));
        assert_eq!(parse(b"XXXXYYYYZZZZWWWW"), Err(PfsError::BadMagic));
        let mut img = build_pfs0(&[("a", b"1234")]);
        img[0x10 + 8] = 0xFF; // el tamaño del archivo desborda el contenedor
        assert!(matches!(parse(&img), Err(PfsError::OutOfRange { .. })));
        let mut img = build_pfs0(&[("a", b"1234")]);
        img[4] = 200; // anuncia 200 archivos
        assert!(parse(&img).is_err());
    }
}
