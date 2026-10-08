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
