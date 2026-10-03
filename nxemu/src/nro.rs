use crate::memory::Memory;

pub const NRO_BASE: u64 = 0x0800_0000;
pub const STACK_TOP: u64 = 0x1000_0000;
pub const STACK_SIZE: u64 = 0x10_0000;

#[derive(Debug)]
pub enum NroError {
    TooShort,
    BadMagic,
    BadSegment,
}

pub struct LoadedNro {
    pub entry: u64,
    pub stack_top: u64,
}

fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}

/// Carga un ejecutable NRO0 (homebrew de Switch): .text, .rodata y .data
/// contiguos desde `NRO_BASE`, más .bss y una pila.
pub fn load(data: &[u8], mem: &mut Memory) -> Result<LoadedNro, NroError> {
    if data.len() < 0x80 {
        return Err(NroError::TooShort);
    }
    if &data[0x10..0x14] != b"NRO0" {
        return Err(NroError::BadMagic);
    }
    let size = u32_at(data, 0x18) as u64;
    let seg = |i: usize| (u32_at(data, 0x20 + i * 8) as u64, u32_at(data, 0x24 + i * 8) as u64);
    let (text, ro, dat) = (seg(0), seg(1), seg(2));
    let bss = u32_at(data, 0x38) as u64;
    for (off, sz) in [text, ro, dat] {
        if off + sz > data.len() as u64 {
            return Err(NroError::BadSegment);
        }
    }
    mem.map(NRO_BASE, size.max(data.len() as u64) + bss);
    mem.write(NRO_BASE, data).map_err(|_| NroError::BadSegment)?;
    mem.map(STACK_TOP - STACK_SIZE, STACK_SIZE);
    Ok(LoadedNro {
        entry: NRO_BASE + text.0,
        stack_top: STACK_TOP,
    })
}
