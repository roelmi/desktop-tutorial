pub mod cpu;
pub mod memory;
pub mod nro;
pub mod svc;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
