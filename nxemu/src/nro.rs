use crate::memory::{Memory, PERM_R, PERM_RW, PERM_RX};

pub const NRO_BASE: u64 = 0x0800_0000;
pub const STACK_TOP: u64 = 0x1000_0000;
pub const STACK_SIZE: u64 = 0x10_0000;

#[derive(Debug)]
pub enum NroError {
    TooShort,
    BadMagic,
    BadSegment,
    /// Reubicación no soportada o datos dinámicos corruptos.
    BadRelocation(String),
}

pub struct LoadedNro {
    pub entry: u64,
    pub stack_top: u64,
    /// Tamaño de la imagen en memoria (segmentos + .bss), redondeado a páginas.
    pub image_size: u64,
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
    // Se carga como RW y luego se aplican los permisos reales de cada segmento (.text RX, .rodata R, .data/.bss RW).
    let image = size.max(data.len() as u64);
    mem.map_perm(NRO_BASE, image + bss, PERM_RW);
    mem.write(NRO_BASE, data).map_err(|_| NroError::BadSegment)?;
    apply_relocations(mem, NRO_BASE, u32_at(data, 4) as u64)?;
    let page = crate::memory::PAGE_SIZE;
    let seg_end = |s: (u64, u64)| (s.0 + s.1).div_ceil(page) * page;
    mem.protect(NRO_BASE + text.0, seg_end(text) - text.0, PERM_RX).map_err(|_| NroError::BadSegment)?;
    if ro.1 > 0 {
        mem.protect(NRO_BASE + ro.0, seg_end(ro) - ro.0, PERM_R).map_err(|_| NroError::BadSegment)?;
    }
    mem.map_perm(STACK_TOP - STACK_SIZE, STACK_SIZE, PERM_RW);
    Ok(LoadedNro {
        entry: NRO_BASE + text.0,
        stack_top: STACK_TOP,
        image_size: (image + bss).div_ceil(crate::memory::PAGE_SIZE) * crate::memory::PAGE_SIZE,
    })
}

const MOD0: u32 = 0x3044_4F4D; // "MOD0"
const DT_NULL: u64 = 0;
const DT_PLTRELSZ: u64 = 2;
const DT_SYMTAB: u64 = 6;
const DT_RELA: u64 = 7;
const DT_RELASZ: u64 = 8;
const DT_RELAENT: u64 = 9;
const DT_JMPREL: u64 = 23;
const R_AARCH64_ABS64: u64 = 257;
const R_AARCH64_GLOB_DAT: u64 = 1025;
const R_AARCH64_JUMP_SLOT: u64 = 1026;
const R_AARCH64_RELATIVE: u64 = 1027;

/// Aplica las reubicaciones dinámicas de un módulo enlazado en la dirección 0 (formato MOD0 de homebrew).
/// `mod_off` es el desplazamiento de la estructura `MOD0` respecto a la base; 0 significa «sin MOD0».
pub fn apply_relocations(mem: &mut Memory, base: u64, mod_off: u64) -> Result<(), NroError> {
    if mod_off == 0 {
        return Ok(());
    }
    let bad = |m: &str| NroError::BadRelocation(m.to_string());
    let rd = |mem: &Memory, a: u64, n: usize| mem.read_uint(a, n).map_err(|_| NroError::BadRelocation(format!("lectura fuera de rango en {a:#x}")));
    let m0 = base + mod_off;
    if rd(mem, m0, 4)? as u32 != MOD0 {
        return Err(bad("falta la magia MOD0"));
    }
    let dynamic = (m0 as i64 + rd(mem, m0 + 4, 4)? as u32 as i32 as i64) as u64;
    let (mut rela, mut relasz, mut relaent, mut jmprel, mut pltrelsz, mut symtab) = (0u64, 0u64, 24u64, 0u64, 0u64, 0u64);
    for i in 0..256u64 {
        let (tag, val) = (rd(mem, dynamic + i * 16, 8)?, rd(mem, dynamic + i * 16 + 8, 8)?);
        match tag {
            DT_NULL => break,
            DT_RELA => rela = val,
            DT_RELASZ => relasz = val,
            DT_RELAENT => relaent = val,
            DT_JMPREL => jmprel = val,
            DT_PLTRELSZ => pltrelsz = val,
            DT_SYMTAB => symtab = val,
            _ => {}
        }
    }
    if relaent < 24 {
        return Err(bad("DT_RELAENT inválido"));
    }
    for (table, size) in [(rela, relasz), (jmprel, pltrelsz)] {
        if table == 0 {
            continue;
        }
        for k in 0..size / relaent {
            let e = base + table + k * relaent;
            let (offset, info, addend) = (rd(mem, e, 8)?, rd(mem, e + 8, 8)?, rd(mem, e + 16, 8)?);
            let (ty, sym) = (info & 0xFFFF_FFFF, info >> 32);
            let target = base + offset;
            let sym_value = |mem: &Memory| -> Result<u64, NroError> {
                if symtab == 0 {
                    return Err(bad("reubicación con símbolo sin DT_SYMTAB"));
                }
                let ent = base + symtab + sym * 24;
                let (shndx, value) = (rd(mem, ent + 6, 2)?, rd(mem, ent + 8, 8)?);
                if shndx == 0 {
                    return Err(bad("símbolo sin definir (el módulo debe ser autocontenido)"));
                }
                Ok(base + value)
            };
            let v = match ty {
                R_AARCH64_RELATIVE => base.wrapping_add(addend),
                R_AARCH64_ABS64 => sym_value(mem)?.wrapping_add(addend),
                R_AARCH64_GLOB_DAT | R_AARCH64_JUMP_SLOT => sym_value(mem)?,
                t => return Err(NroError::BadRelocation(format!("tipo de reubicación {t} no soportado"))),
            };
            mem.write_uint(target, 8, v).map_err(|_| NroError::BadRelocation(format!("escritura fuera de rango en {target:#x}")))?;
        }
    }
    Ok(())
}
