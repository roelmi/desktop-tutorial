use super::{create, rc, IpcRequest, IpcResponse, Service, RESULT_UNKNOWN_COMMAND};

/// `sm:` — gestor de servicios: `Initialize` y `GetService`.
pub struct ServiceManager {
    initialized: bool,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

/// El nombre del servicio llega como 8 bytes ASCII rellenos con ceros.
fn service_name(data: &[u8]) -> String {
    data.iter().take(8).take_while(|b| **b != 0).map(|b| *b as char).collect()
}

impl Service for ServiceManager {
    fn name(&self) -> &'static str {
        "sm:"
    }

    fn call(&mut self, req: &IpcRequest) -> IpcResponse {
        match req.command {
            // Initialize(pid)
            0 => {
                self.initialized = true;
                IpcResponse::ok()
            }
            // GetService(name) -> sesión
            1 => {
                if !self.initialized {
                    return IpcResponse::err(rc(21, 3)); // NotInitialized
                }
                let name = service_name(&req.data);
                match create(&name) {
                    Some(s) => {
                        let mut r = IpcResponse::ok();
                        r.new_sessions.push(s);
                        r
                    }
                    None => IpcResponse::err(rc(21, 7)), // NotRegistered
                }
            }
            _ => IpcResponse::err(RESULT_UNKNOWN_COMMAND),
        }
    }
}
