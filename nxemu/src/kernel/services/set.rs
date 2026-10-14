use super::{IpcRequest, IpcResponse, Service, RESULT_UNKNOWN_COMMAND};

/// Código de idioma «en-US» como lo devuelve `set` (cadena ASCII en un u64).
pub const LANGUAGE_EN_US: u64 = u64::from_le_bytes(*b"en-US\0\0\0");

/// `set` — ajustes del sistema.
pub struct Set;

impl Service for Set {
    fn name(&self) -> &'static str {
        "set"
    }

    fn call(&mut self, req: &IpcRequest) -> IpcResponse {
        let mut r = IpcResponse::ok();
        match req.command {
            // GetLanguageCode() -> u64
            0 => {
                r.push_u64(LANGUAGE_EN_US);
            }
            // GetRegionCode() -> u32 (0 = Japón, 1 = EE. UU., 2 = Europa…)
            4 => {
                r.push_u32(1);
            }
            _ => return IpcResponse::err(RESULT_UNKNOWN_COMMAND),
        }
        r
    }
}
