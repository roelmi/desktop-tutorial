use nxemu::{cpu::Cpu, memory::Memory, nro, svc};

/// Ensambla un NRO0 mínimo: cabecera de 0x80 bytes (primera instrucción = `b` al código).
fn make_nro(code: &[u8]) -> Vec<u8> {
    let total = (0x80 + code.len()).next_multiple_of(0x1000);
    let mut f = vec![0u8; total];
    f[0..4].copy_from_slice(&(0x1400_0000u32 | (0x80 >> 2)).to_le_bytes()); // b #0x80
    f[0x10..0x14].copy_from_slice(b"NRO0");
    f[0x18..0x1C].copy_from_slice(&(total as u32).to_le_bytes());
    f[0x20..0x24].copy_from_slice(&0u32.to_le_bytes()); // text off
    f[0x24..0x28].copy_from_slice(&(total as u32).to_le_bytes()); // text size
    f[0x80..0x80 + code.len()].copy_from_slice(code);
    f
}

fn boot(code: &[u8]) -> Result<(u64, String), String> {
    let mut mem = Memory::new();
    let n = nro::load(&make_nro(code), &mut mem).map_err(|e| format!("{e:?}"))?;
    let mut cpu = Cpu::new();
    cpu.pc = n.entry;
    cpu.sp = n.stack_top;
    svc::run_program(&mut cpu, &mut mem, 100)
}

#[test]
fn programa_ensamblado_con_llvm() {
    let (code, out) = boot(include_bytes!("asm/prog.bin")).unwrap();
    assert_eq!(out, "Hola, Switch!\n");
    assert_eq!(code, 42);
}

#[test]
fn rechaza_nro_invalido() {
    let mut mem = Memory::new();
    assert!(nro::load(&[0u8; 0x100], &mut mem).is_err());
}

#[test]
fn instruccion_no_implementada_da_error() {
    // 0x00000000 (udf) no está soportada.
    let e = boot(&[0, 0, 0, 0]).unwrap_err();
    assert!(e.contains("Unimplemented"), "{e}");
}

#[test]
fn flags_de_cmp() {
    // mov x0,#5 ; cmp x0,#5 ; cset-like vía b.eq -> exit(1), si no exit(0)
    //   d28000a0 mov x0,#5 | f100141f cmp x0,#5 | 54000060 b.eq +12 | d2800000 mov x0,#0 | d40000e1 svc #7 | d2800020 mov x0,#1 | d40000e1 svc #7
    let words: [u32; 7] = [0xd28000a0, 0xf100141f, 0x54000060, 0xd2800000, 0xd40000e1, 0xd2800020, 0xd40000e1];
    let code: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    assert_eq!(boot(&code).unwrap().0, 1);
}

fn words(w: &[u32]) -> Vec<u8> {
    w.iter().flat_map(|x| x.to_le_bytes()).collect()
}

#[test]
fn escribir_en_el_codigo_falla_por_permisos() {
    // adr x0, . ; str x0, [x0]   (el .text es R+X, no W)
    let e = boot(&words(&[0x1000_0000, 0xF900_0000])).unwrap_err();
    assert!(e.contains("Permission") && e.contains("Write"), "{e}");
}

#[test]
fn ejecutar_desde_la_pila_falla_por_permisos() {
    // movz x0, #0xFF0, lsl #16 ; br x0   (la pila es RW, no X)
    let e = boot(&words(&[0xD2A1_FE00, 0xD61F_0000])).unwrap_err();
    assert!(e.contains("Permission") && e.contains("Execute"), "{e}");
}
