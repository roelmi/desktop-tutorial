//! Servicios HLE (implementados en alto nivel dentro del emulador) accesibles por IPC.

mod apm;
mod set;
mod sm;

use super::ipc::{IpcRequest, IpcResponse};
pub use sm::ServiceManager;

/// Códigos de resultado de servicios (módulo, descripción) → `module | desc << 9`.
pub const fn rc(module: u32, desc: u32) -> u32 {
    module | (desc << 9)
}

pub trait Service {
    fn name(&self) -> &'static str;
    /// Atiende un comando CMIF. Devuelve la respuesta (con su código de resultado).
    fn call(&mut self, req: &IpcRequest) -> IpcResponse;
}

/// Crea el servicio con ese nombre, si está implementado.
pub fn create(name: &str) -> Option<Box<dyn Service>> {
    match name {
        "sm:" => Some(Box::new(ServiceManager::new())),
        "set" => Some(Box::new(set::Set)),
        "apm" => Some(Box::new(apm::Apm)),
        _ => None,
    }
}

/// Resultado de "servicio no implementado" que se devuelve al invocar un comando desconocido.
pub const RESULT_UNKNOWN_COMMAND: u32 = rc(10, 221);
