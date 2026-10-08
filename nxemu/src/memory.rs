use std::collections::HashMap;

pub const PAGE_SIZE: u64 = 0x1000;

#[derive(Debug, PartialEq, Eq)]
pub enum MemError {
    Unmapped(u64),
}

/// Memoria virtual dispersa por páginas de 4 KiB.
#[derive(Default)]
pub struct Memory {
    pages: HashMap<u64, Box<[u8; PAGE_SIZE as usize]>>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mapea (con ceros) el rango [addr, addr+size), redondeado a páginas.
    pub fn map(&mut self, addr: u64, size: u64) {
        let start = addr / PAGE_SIZE;
        let end = (addr + size).div_ceil(PAGE_SIZE);
        for p in start..end {
            self.pages
                .entry(p)
                .or_insert_with(|| Box::new([0; PAGE_SIZE as usize]));
        }
    }

    pub fn read(&self, addr: u64, buf: &mut [u8]) -> Result<(), MemError> {
        for (i, b) in buf.iter_mut().enumerate() {
            let a = addr.wrapping_add(i as u64);
            let page = self.pages.get(&(a / PAGE_SIZE)).ok_or(MemError::Unmapped(a))?;
            *b = page[(a % PAGE_SIZE) as usize];
        }
        Ok(())
    }

    pub fn write(&mut self, addr: u64, data: &[u8]) -> Result<(), MemError> {
        for (i, b) in data.iter().enumerate() {
            let a = addr.wrapping_add(i as u64);
            let page = self
                .pages
                .get_mut(&(a / PAGE_SIZE))
                .ok_or(MemError::Unmapped(a))?;
            page[(a % PAGE_SIZE) as usize] = *b;
        }
        Ok(())
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
