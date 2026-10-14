//! Mensajes IPC (HIPC/CMIF) de Horizon: análisis de la petición en el buffer TLS y construcción de la respuesta.
//!
//! Formato (según la documentación pública de la Switch original): cabecera de 2 palabras, descriptor de
//! handles opcional, descriptores X/A/B/W, datos crudos alineados a 16 bytes con magia `SFCI` (petición) o
//! `SFCO` (respuesta).

use crate::memory::{MemError, Memory};

pub const SFCI: u32 = 0x4943_4653;
pub const SFCO: u32 = 0x4F43_4653;

/// Tipos de mensaje (`w0 & 0xFFFF`).
pub const TYPE_CLOSE: u32 = 2;
pub const TYPE_REQUEST: u32 = 4;
pub const TYPE_CONTROL: u32 = 5;
pub const TYPE_REQUEST_WITH_CONTEXT: u32 = 6;
pub const TYPE_CONTROL_WITH_CONTEXT: u32 = 7;

#[derive(Debug, Default, Clone)]
pub struct IpcRequest {
    pub kind: u32,
    pub command: u32,
    /// Carga útil tras la cabecera `SFCI` y el id de comando.
    pub data: Vec<u8>,
    pub pid: Option<u64>,
    pub copy_handles: Vec<u32>,
    pub move_handles: Vec<u32>,
    /// Buffers de puntero (X): (dirección, tamaño).
    pub x_bufs: Vec<(u64, u64)>,
    /// Buffers de envío (A) y de recepción (B).
    pub a_bufs: Vec<(u64, u64)>,
    pub b_bufs: Vec<(u64, u64)>,
    pub w_bufs: Vec<(u64, u64)>,
}

impl IpcRequest {
    pub fn u32_at(&self, off: usize) -> u32 {
        self.data.get(off..off + 4).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap()))
    }
    pub fn u64_at(&self, off: usize) -> u64 {
        self.data.get(off..off + 8).map_or(0, |b| u64::from_le_bytes(b.try_into().unwrap()))
    }
}

#[derive(Default)]
pub struct IpcResponse {
    pub result: u32,
    pub data: Vec<u8>,
    pub copy_handles: Vec<u32>,
    /// Handles ya existentes que se mueven al cliente.
    pub move_handles: Vec<u32>,
    /// Sesiones nuevas que el kernel registrará y devolverá como handles movidos.
    pub new_sessions: Vec<Box<dyn super::services::Service>>,
}

impl IpcResponse {
    pub fn ok() -> Self {
        Self::default()
    }
    pub fn err(code: u32) -> Self {
        Self { result: code, ..Self::default() }
    }
    pub fn push_u32(&mut self, v: u32) -> &mut Self {
        self.data.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn push_u64(&mut self, v: u64) -> &mut Self {
        self.data.extend_from_slice(&v.to_le_bytes());
        self
    }
}

fn rd32(mem: &Memory, a: u64) -> Result<u32, MemError> {
    Ok(mem.read_uint(a, 4)? as u32)
}

fn x_desc(mem: &Memory, a: u64) -> Result<(u64, u64), MemError> {
    let (w0, w1) = (rd32(mem, a)? as u64, rd32(mem, a + 4)? as u64);
    let addr = w1 | (((w0 >> 12) & 0xF) << 32) | (((w0 >> 6) & 7) << 36);
    Ok((addr, w0 >> 16))
}

fn abw_desc(mem: &Memory, a: u64) -> Result<(u64, u64), MemError> {
    let (size_lo, addr_lo, w2) = (rd32(mem, a)? as u64, rd32(mem, a + 4)? as u64, rd32(mem, a + 8)? as u64);
    let addr = addr_lo | ((w2 >> 28) << 32) | (((w2 >> 2) & 7) << 36);
    let size = size_lo | (((w2 >> 24) & 0xF) << 32);
    Ok((addr, size))
}

/// Analiza el mensaje que hay en `tls` (buffer de comandos del hilo).
pub fn parse_request(mem: &Memory, tls: u64) -> Result<IpcRequest, MemError> {
    let w0 = rd32(mem, tls)?;
    let w1 = rd32(mem, tls + 4)?;
    let kind = w0 & 0xFFFF;
    let (nx, na, nb, nw) = ((w0 >> 16) & 0xF, (w0 >> 20) & 0xF, (w0 >> 24) & 0xF, (w0 >> 28) & 0xF);
    let raw_words = (w1 & 0x3FF) as u64;
    let mut req = IpcRequest { kind, ..Default::default() };
    let mut cur = tls + 8;
    if (w1 >> 31) & 1 == 1 {
        let hd = rd32(mem, cur)?;
        cur += 4;
        if hd & 1 == 1 {
            req.pid = Some(mem.read_uint(cur, 8)?);
            cur += 8;
        }
        for _ in 0..((hd >> 1) & 0xF) {
            req.copy_handles.push(rd32(mem, cur)?);
            cur += 4;
        }
        for _ in 0..((hd >> 5) & 0xF) {
            req.move_handles.push(rd32(mem, cur)?);
            cur += 4;
        }
    }
    for _ in 0..nx {
        req.x_bufs.push(x_desc(mem, cur)?);
        cur += 8;
    }
    for _ in 0..na {
        req.a_bufs.push(abw_desc(mem, cur)?);
        cur += 12;
    }
    for _ in 0..nb {
        req.b_bufs.push(abw_desc(mem, cur)?);
        cur += 12;
    }
    for _ in 0..nw {
        req.w_bufs.push(abw_desc(mem, cur)?);
        cur += 12;
    }
    // Los datos crudos empiezan alineados a 16 bytes respecto al inicio del buffer.
    let raw = tls + ((cur - tls + 15) & !15);
    let mut bytes = vec![0u8; (raw_words * 4) as usize];
    if !bytes.is_empty() {
        mem.read(raw, &mut bytes)?;
    }
    // Las peticiones llevan 16 bytes de cabecera: magia SFCI + id de comando (u64).
    if bytes.len() >= 16 && matches!(kind, TYPE_REQUEST | TYPE_CONTROL | TYPE_REQUEST_WITH_CONTEXT | TYPE_CONTROL_WITH_CONTEXT) {
        req.command = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        req.data = bytes[16..].to_vec();
    } else {
        req.data = bytes;
    }
    Ok(req)
}

/// Escribe la respuesta en el buffer de comandos `tls`. `handles_move` son los handles ya creados.
pub fn write_response(mem: &mut Memory, tls: u64, resp: &IpcResponse, move_handles: &[u32]) -> Result<(), MemError> {
    let mut raw = Vec::with_capacity(16 + resp.data.len());
    raw.extend_from_slice(&SFCO.to_le_bytes());
    raw.extend_from_slice(&0u32.to_le_bytes());
    raw.extend_from_slice(&(resp.result as u64).to_le_bytes());
    raw.extend_from_slice(&resp.data);
    while raw.len() % 4 != 0 {
        raw.push(0);
    }
    let has_handles = !resp.copy_handles.is_empty() || !move_handles.is_empty();
    let w0 = 0u32;
    let w1 = (raw.len() as u32 / 4) | if has_handles { 1 << 31 } else { 0 };
    mem.write_uint(tls, 4, w0 as u64)?;
    mem.write_uint(tls + 4, 4, w1 as u64)?;
    let mut cur = tls + 8;
    if has_handles {
        let hd = ((resp.copy_handles.len() as u32) << 1) | ((move_handles.len() as u32) << 5);
        mem.write_uint(cur, 4, hd as u64)?;
        cur += 4;
        for h in resp.copy_handles.iter().chain(move_handles.iter()) {
            mem.write_uint(cur, 4, *h as u64)?;
            cur += 4;
        }
    }
    let at = tls + ((cur - tls + 15) & !15);
    mem.write(at, &raw)?;
    Ok(())
}

/// Construye una petición (útil para pruebas y para clientes HLE).
pub fn build_request(mem: &mut Memory, tls: u64, kind: u32, command: u32, payload: &[u8], copy: &[u32], mv: &[u32], send_pid: bool) -> Result<(), MemError> {
    let mut raw = Vec::new();
    raw.extend_from_slice(&SFCI.to_le_bytes());
    raw.extend_from_slice(&0u32.to_le_bytes());
    raw.extend_from_slice(&(command as u64).to_le_bytes());
    raw.extend_from_slice(payload);
    while raw.len() % 4 != 0 {
        raw.push(0);
    }
    let has_handles = send_pid || !copy.is_empty() || !mv.is_empty();
    mem.write_uint(tls, 4, kind as u64)?;
    mem.write_uint(tls + 4, 4, ((raw.len() as u32 / 4) | if has_handles { 1 << 31 } else { 0 }) as u64)?;
    let mut cur = tls + 8;
    if has_handles {
        let hd = (send_pid as u32) | ((copy.len() as u32) << 1) | ((mv.len() as u32) << 5);
        mem.write_uint(cur, 4, hd as u64)?;
        cur += 4;
        if send_pid {
            mem.write_uint(cur, 8, 0)?;
            cur += 8;
        }
        for h in copy.iter().chain(mv.iter()) {
            mem.write_uint(cur, 4, *h as u64)?;
            cur += 4;
        }
    }
    mem.write(tls + ((cur - tls + 15) & !15), &raw)
}
