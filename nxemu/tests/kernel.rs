//! Pruebas del kernel HLE con programas invitados reales (AArch64, formato NRO).
//! Regenerar: `tests/guest/build.sh` (requiere clang + lld). Un `CHECK` fallido termina con su número de línea.

use nxemu::kernel::Kernel;
use std::path::PathBuf;

fn run(name: &str) -> (Result<u64, String>, Kernel) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/guest/out").join(format!("{name}.nro"));
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("falta {path:?}: {e} (ejecuta tests/guest/build.sh)"));
    let mut k = Kernel::new();
    k.boot_nro(&data).unwrap();
    let r = k.run(300_000_000);
    (r, k)
}

fn ok(name: &str) -> Kernel {
    let (r, k) = run(name);
    match r {
        Ok(0) => k,
        Ok(line) => panic!("{name}.c: CHECK fallido en la línea {line}\nlog: {:?}", k.log),
        Err(e) => panic!("{name}: {e}\nlog: {:?}", k.log),
    }
}

#[test]
fn memoria_del_proceso() {
    ok("memory");
}

#[test]
fn hilos_mutex_y_variables_de_condicion() {
    let k = ok("threads");
    assert!(k.now() > 0);
}

#[test]
fn ipc_con_servicios() {
    let k = ok("ipc");
    assert_eq!(k.debug_output, "hola desde el invitado\n");
    // el comando 99 de `set` no existe: queda registrado para depuración
    assert!(k.log.iter().any(|l| l.contains("set: comando 99")), "{:?}", k.log);
}

#[test]
fn reubicaciones_mod0_en_un_nro_pic() {
    // Tablas de punteros a funciones/cadenas y un puntero a un global: solo funcionan si se aplican las reubicaciones.
    ok("reloc");
}

#[test]
fn nro_con_reubicacion_corrupta_se_rechaza() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/guest/out/reloc.nro");
    let mut data = std::fs::read(path).unwrap();
    let mod_off = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    data[mod_off] = b'X'; // rompe la magia MOD0
    let mut k = Kernel::new();
    assert!(k.boot_nro(&data).unwrap_err().contains("MOD0"));
}

// ------------------------------------------------------------------ formatos: NSO y ExeFS (PFS0)
mod exefs {
    use super::*;
    use nxemu::formats::{nso, pfs};

    /// Parte un NRO ya compilado en sus segmentos (la cabecera NRO queda dentro de .text y sigue siendo código válido).
    fn segmentos(nro: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>, u32) {
        let w = |o: usize| u32::from_le_bytes(nro[o..o + 4].try_into().unwrap()) as usize;
        let (t, r, d) = ((w(0x20), w(0x24)), (w(0x28), w(0x2C)), (w(0x30), w(0x34)));
        (nro[t.0..t.0 + t.1].to_vec(), nro[r.0..r.0 + r.1].to_vec(), nro[d.0..d.0 + d.1].to_vec(), w(0x38) as u32)
    }

    fn empaqueta(name: &str, compress: bool) -> Vec<u8> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/guest/out").join(format!("{name}.nro"));
        let (t, r, d, bss) = segmentos(&std::fs::read(path).unwrap());
        let main = nso::build(&t, &r, &d, bss, compress);
        pfs::build_pfs0(&[("main", &main), ("main.npdm", b"npdm de relleno")])
    }

    fn run_exefs(img: &[u8]) -> (Result<u64, String>, Kernel) {
        let mut k = Kernel::new();
        k.boot_exefs(img).unwrap();
        let r = k.run(300_000_000);
        (r, k)
    }

    #[test]
    fn arranca_programas_desde_un_exefs_con_nso_comprimido_y_sin_comprimir() {
        for name in ["memory", "ipc", "threads"] {
            for compress in [true, false] {
                let (r, k) = run_exefs(&empaqueta(name, compress));
                assert_eq!(r, Ok(0), "{name} (lz4={compress}): {:?}", k.log);
            }
        }
    }

    #[test]
    fn exefs_sin_main_o_con_nso_corrupto_se_rechaza() {
        let mut k = Kernel::new();
        let sin_main = pfs::build_pfs0(&[("main.npdm", b"x")]);
        assert!(k.boot_exefs(&sin_main).unwrap_err().contains("main"));
        for compress in [true, false] {
            let mut img = empaqueta("memory", compress);
            let main = pfs::parse(&img).unwrap().files.into_iter().find(|f| f.name == "main").unwrap();
            // un byte cualquiera del NSO (en un segmento): el hash SHA-256 o el LZ4 deben detectarlo
            img[main.offset + main.size - 3] ^= 0x55;
            let mut k = Kernel::new();
            let e = k.boot_exefs(&img).unwrap_err();
            assert!(e.starts_with("main:"), "{e}");
        }
        assert!(Kernel::new().boot_exefs(b"no es un PFS0 de verdad").unwrap_err().contains("ExeFS"));
    }
}
