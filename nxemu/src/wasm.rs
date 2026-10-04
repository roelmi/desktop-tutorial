//! Interfaz C-ABI para usar el emulador desde JavaScript (sin wasm-bindgen).
//!
//! Flujo: `nx_alloc` → JS copia el .nro → `nx_run(ptr, len, max_svcs)` →
//! `nx_output_ptr/len` (texto de debug) y `nx_error_ptr/len` (si falló).

use crate::{cpu::Cpu, memory::Memory, nro, svc};
use std::sync::Mutex;

#[derive(Default)]
struct State {
    out: Vec<u8>,
    err: Vec<u8>,
}

static STATE: Mutex<State> = Mutex::new(State { out: Vec::new(), err: Vec::new() });

#[no_mangle]
pub extern "C" fn nx_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

#[no_mangle]
pub unsafe extern "C" fn nx_free(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len));
}

/// Ejecuta un NRO. Devuelve el código de salida, o -1 si hubo error (ver `nx_error_*`).
#[no_mangle]
pub unsafe extern "C" fn nx_run(ptr: *const u8, len: usize, max_svcs: u32) -> i64 {
    let data = std::slice::from_raw_parts(ptr, len);
    let mut st = STATE.lock().unwrap();
    st.out.clear();
    st.err.clear();
    let mut mem = Memory::new();
    let loaded = match nro::load(data, &mut mem) {
        Ok(l) => l,
        Err(e) => {
            st.err = format!("NRO inválido: {e:?}").into_bytes();
            return -1;
        }
    };
    let mut cpu = Cpu::new();
    cpu.pc = loaded.entry;
    cpu.sp = loaded.stack_top;
    match svc::run_program(&mut cpu, &mut mem, max_svcs) {
        Ok((code, out)) => {
            st.out = out.into_bytes();
            code as i64
        }
        Err(e) => {
            st.err = e.into_bytes();
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn nx_output_ptr() -> *const u8 { STATE.lock().unwrap().out.as_ptr() }
#[no_mangle]
pub extern "C" fn nx_output_len() -> usize { STATE.lock().unwrap().out.len() }
#[no_mangle]
pub extern "C" fn nx_error_ptr() -> *const u8 { STATE.lock().unwrap().err.as_ptr() }
#[no_mangle]
pub extern "C" fn nx_error_len() -> usize { STATE.lock().unwrap().err.len() }
