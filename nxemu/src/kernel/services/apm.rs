use super::{IpcRequest, IpcResponse, Service, RESULT_UNKNOWN_COMMAND};

/// `apm` — gestión de rendimiento. Se devuelve una sesión que admite SetPerformanceConfiguration.
pub struct Apm;

impl Service for Apm {
    fn name(&self) -> &'static str {
        "apm"
    }

    fn call(&mut self, req: &IpcRequest) -> IpcResponse {
        match req.command {
            // OpenSession() -> ISession
            0 => {
                let mut r = IpcResponse::ok();
                r.new_sessions.push(Box::new(ApmSession::default()));
                r
            }
            // GetPerformanceMode() -> u32 (0 = portátil, 1 = en base)
            1 => {
                let mut r = IpcResponse::ok();
                r.push_u32(0);
                r
            }
            _ => IpcResponse::err(RESULT_UNKNOWN_COMMAND),
        }
    }
}

#[derive(Default)]
struct ApmSession {
    config: [u32; 2],
}

impl Service for ApmSession {
    fn name(&self) -> &'static str {
        "apm:session"
    }

    fn call(&mut self, req: &IpcRequest) -> IpcResponse {
        match req.command {
            // SetPerformanceConfiguration(mode u32, config u32)
            0 => {
                let (mode, cfg) = (req.u32_at(0) as usize, req.u32_at(4));
                if mode < 2 {
                    self.config[mode] = cfg;
                }
                IpcResponse::ok()
            }
            // GetPerformanceConfiguration(mode u32) -> u32
            1 => {
                let mode = req.u32_at(0) as usize;
                let mut r = IpcResponse::ok();
                r.push_u32(*self.config.get(mode).unwrap_or(&0));
                r
            }
            _ => IpcResponse::err(RESULT_UNKNOWN_COMMAND),
        }
    }
}
