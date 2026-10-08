use std::collections::HashMap;

pub const PAGE_SIZE: u64 = 0x1000;

/// Permisos de página.
pub const PERM_R: u8 = 1;
pub const PERM_W: u8 = 2;
pub const PERM_X: u8 = 4;
pub const PERM_RW: u8 = PERM_R | PERM_W;
pub const PERM_RX: u8 = PERM_R | PERM_X;
pub const PERM_RWX: u8 = PERM_R | PERM_W | PERM_X;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Access {
    Read,
    Write,
    Execute,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MemError {
    /// Dirección sin mapear.
    Unmapped(u64),
    /// Dirección mapeada pero sin el permiso necesario.
    Permission { addr: u64, access: Access },
}

struct Page {
    data: Box<[u8; PAGE_SIZE as usize]>,
    perm: u8,
}

/// Información de una región contigua con los mismos permisos (para `QueryMemory`).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Region {
    pub start: u64,
    pub size: u64,
    pub perm: u8,
    pub mapped: bool,
}

/// Memoria virtual dispersa por páginas de 4 KiB, con permisos R/W/X por página.
#[derive(Default)]
pub struct Memory {
    pages: HashMap<u64, Page>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    fn page_range(addr: u64, size: u64) -> std::ops::Range<u64> {
        addr / PAGE_SIZE..(addr + size).div_ceil(PAGE_SIZE)
    }

    /// Mapea (con ceros) el rango con permisos RWX. Las páginas ya mapeadas se conservan.
    pub fn map(&mut self, addr: u64, size: u64) {
        self.map_perm(addr, size, PERM_RWX);
    }

    /// Mapea el rango con los permisos dados. Las páginas ya mapeadas se conservan tal cual.
    pub fn map_perm(&mut self, addr: u64, size: u64, perm: u8) {
        for p in Self::page_range(addr, size) {
            self.pages.entry(p).or_insert_with(|| Page { data: Box::new([0; PAGE_SIZE as usize]), perm });
        }
    }

    /// Cambia los permisos de un rango. Devuelve `Err` si alguna página no está mapeada (sin cambiar nada).
    pub fn protect(&mut self, addr: u64, size: u64, perm: u8) -> Result<(), MemError> {
        for p in Self::page_range(addr, size) {
            if !self.pages.contains_key(&p) {
                return Err(MemError::Unmapped(p * PAGE_SIZE));
            }
        }
        for p in Self::page_range(addr, size) {
            self.pages.get_mut(&p).unwrap().perm = perm;
        }
        Ok(())
    }

    pub fn unmap(&mut self, addr: u64, size: u64) {
        for p in Self::page_range(addr, size) {
            self.pages.remove(&p);
        }
    }

    pub fn perm_at(&self, addr: u64) -> Option<u8> {
        self.pages.get(&(addr / PAGE_SIZE)).map(|p| p.perm)
    }

    /// Región contigua (mapeada con los mismos permisos, o hueco sin mapear) que contiene `addr`.
    pub fn query(&self, addr: u64) -> Region {
        let page = addr / PAGE_SIZE;
        let state = self.pages.get(&page).map(|p| p.perm);
        let same = |p: u64| self.pages.get(&p).map(|x| x.perm) == state;
        let mut lo = page;
        while lo > 0 && same(lo - 1) {
            lo -= 1;
            if page - lo > 0x10_0000 {
                break;
            }
        }
        let mut hi = page;
        while same(hi + 1) {
            hi += 1;
            if hi - page > 0x10_0000 {
                break;
            }
        }
        Region { start: lo * PAGE_SIZE, size: (hi - lo + 1) * PAGE_SIZE, perm: state.unwrap_or(0), mapped: state.is_some() }
    }

    fn access(&self, addr: u64, need: u8, kind: Access) -> Result<&Page, MemError> {
        let page = self.pages.get(&(addr / PAGE_SIZE)).ok_or(MemError::Unmapped(addr))?;
        if page.perm & need == 0 {
            return Err(MemError::Permission { addr, access: kind });
        }
        Ok(page)
    }

    pub fn read(&self, addr: u64, buf: &mut [u8]) -> Result<(), MemError> {
        for (i, b) in buf.iter_mut().enumerate() {
            let a = addr.wrapping_add(i as u64);
            *b = self.access(a, PERM_R, Access::Read)?.data[(a % PAGE_SIZE) as usize];
        }
        Ok(())
    }

    pub fn write(&mut self, addr: u64, data: &[u8]) -> Result<(), MemError> {
        // Se comprueba todo el rango antes de escribir, para que un fallo no deje escrituras parciales.
        for i in (0..data.len() as u64).step_by(PAGE_SIZE as usize).chain(data.len().checked_sub(1).map(|l| l as u64)) {
            self.access(addr.wrapping_add(i), PERM_W, Access::Write)?;
        }
        for (i, b) in data.iter().enumerate() {
            let a = addr.wrapping_add(i as u64);
            self.pages.get_mut(&(a / PAGE_SIZE)).unwrap().data[(a % PAGE_SIZE) as usize] = *b;
        }
        Ok(())
    }

    /// Lee una instrucción de 32 bits (requiere permiso de ejecución).
    pub fn fetch32(&self, addr: u64) -> Result<u32, MemError> {
        let mut b = [0u8; 4];
        for (i, x) in b.iter_mut().enumerate() {
            let a = addr.wrapping_add(i as u64);
            *x = self.access(a, PERM_X, Access::Execute)?.data[(a % PAGE_SIZE) as usize];
        }
        Ok(u32::from_le_bytes(b))
    }

    /// Lee un valor little-endian de `n` bytes (1, 2, 4 u 8).
    pub fn read_uint(&self, addr: u64, n: usize) -> Result<u64, MemError> {
        let mut b = [0u8; 8];
        self.read(addr, &mut b[..n])?;
        Ok(u64::from_le_bytes(b))
    }

    pub fn write_uint(&mut self, addr: u64, n: usize, v: u64) -> Result<(), MemError> {
        self.write(addr, &v.to_le_bytes()[..n])
    }

    pub fn read_u128(&self, addr: u64) -> Result<u128, MemError> {
        let mut b = [0u8; 16];
        self.read(addr, &mut b)?;
        Ok(u128::from_le_bytes(b))
    }

    pub fn write_u128(&mut self, addr: u64, v: u128) -> Result<(), MemError> {
        self.write(addr, &v.to_le_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permisos_se_aplican() {
        let mut m = Memory::new();
        m.map_perm(0x1000, 0x2000, PERM_R);
        assert!(m.read_uint(0x1000, 8).is_ok());
        assert_eq!(m.write_uint(0x1000, 4, 1), Err(MemError::Permission { addr: 0x1000, access: Access::Write }));
        assert_eq!(m.fetch32(0x1000), Err(MemError::Permission { addr: 0x1000, access: Access::Execute }));
        m.protect(0x1000, 0x1000, PERM_RX).unwrap();
        assert_eq!(m.fetch32(0x1000), Ok(0));
        assert_eq!(m.read_uint(0x5000, 1), Err(MemError::Unmapped(0x5000)));
    }

    #[test]
    fn escritura_atravesando_paginas_es_atomica() {
        let mut m = Memory::new();
        m.map_perm(0x1000, 0x1000, PERM_RW);
        m.map_perm(0x2000, 0x1000, PERM_R);
        // cruza de una página RW a una R: debe fallar sin escribir nada
        assert!(m.write(0x1FFE, &[1, 2, 3, 4]).is_err());
        assert_eq!(m.read_uint(0x1FFE, 2), Ok(0));
    }

    #[test]
    fn query_agrupa_regiones() {
        let mut m = Memory::new();
        m.map_perm(0x4000, 0x3000, PERM_RW);
        m.map_perm(0x7000, 0x1000, PERM_RX);
        assert_eq!(m.query(0x5000), Region { start: 0x4000, size: 0x3000, perm: PERM_RW, mapped: true });
        assert_eq!(m.query(0x7000).perm, PERM_RX);
        let hueco = m.query(0x2000);
        assert!(!hueco.mapped);
    }

    #[test]
    fn unmap_libera_paginas() {
        let mut m = Memory::new();
        m.map(0x1000, 0x1000);
        m.unmap(0x1000, 0x1000);
        assert_eq!(m.perm_at(0x1000), None);
    }
}
