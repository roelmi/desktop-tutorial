//! Procesador de comandos: entradas GPFIFO y *pushbuffers*.
//!
//! Una entrada GPFIFO (64 bits) apunta a un búfer de comandos (dirección en los bits 2..39 y longitud en
//! palabras en los bits 42..62). El búfer es una secuencia de cabeceras de 32 bits seguidas de argumentos:
//!
//! ```text
//! bits 0-12  método (índice de palabra del registro)
//! bits 13-15 subcanal
//! bits 16-28 número de argumentos (o el dato inmediato en el modo 4)
//! bits 29-31 modo: 1 incremental, 3 no incremental, 4 dato inmediato, 5 incrementa una vez
//! ```

use crate::GpuMemory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodCall {
    pub subchannel: u8,
    /// Índice de palabra del registro.
    pub method: u32,
    pub arg: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FifoError {
    /// El búfer termina antes de recibir todos los argumentos anunciados.
    Truncated { header_index: usize },
    /// Modo de envío no soportado.
    BadMode { header_index: usize, mode: u32 },
    /// No se pudo leer la memoria del búfer.
    Memory(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpFifoEntry {
    pub address: u64,
    pub length_words: u32,
}

impl GpFifoEntry {
    pub fn from_raw(raw: u64) -> Self {
        Self { address: raw & 0xFF_FFFF_FFFC, length_words: ((raw >> 42) & 0x1F_FFFF) as u32 }
    }

    pub fn to_raw(self) -> u64 {
        (self.address & 0xFF_FFFF_FFFC) | ((self.length_words as u64) << 42)
    }
}

/// Decodifica un búfer de comandos en llamadas a métodos.
pub fn decode(words: &[u32]) -> Result<Vec<MethodCall>, FifoError> {
    let mut calls = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let h = words[i];
        let (method, sub, count_or_data, mode) = (h & 0x1FFF, ((h >> 13) & 7) as u8, (h >> 16) & 0x1FFF, h >> 29);
        let header_index = i;
        i += 1;
        match mode {
            4 => calls.push(MethodCall { subchannel: sub, method, arg: count_or_data }),
            1 | 3 | 5 | 0 | 2 => {
                let n = count_or_data as usize;
                let args = words.get(i..i + n).ok_or(FifoError::Truncated { header_index })?;
                for (k, &arg) in args.iter().enumerate() {
                    let m = match mode {
                        0 | 1 => method + k as u32,
                        2 | 3 => method,
                        _ => method + (k > 0) as u32, // incrementa una vez
                    };
                    calls.push(MethodCall { subchannel: sub, method: m, arg });
                }
                i += n;
            }
            m => return Err(FifoError::BadMode { header_index, mode: m }),
        }
    }
    Ok(calls)
}

/// Lee y decodifica todas las entradas GPFIFO.
pub fn read_gpfifo(mem: &dyn GpuMemory, entries: &[u64]) -> Result<Vec<MethodCall>, FifoError> {
    let mut all = Vec::new();
    for &raw in entries {
        let e = GpFifoEntry::from_raw(raw);
        let mut bytes = vec![0u8; e.length_words as usize * 4];
        if !mem.read(e.address, &mut bytes) {
            return Err(FifoError::Memory(e.address));
        }
        let words: Vec<u32> = bytes.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
        all.extend(decode(&words)?);
    }
    Ok(all)
}

/// Constructor de búferes de comandos (para pruebas y para clientes).
#[derive(Default)]
pub struct PushBuilder {
    pub words: Vec<u32>,
}

impl PushBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    fn header(mode: u32, sub: u8, method: u32, n: u32) -> u32 {
        (mode << 29) | (n << 16) | ((sub as u32) << 13) | method
    }
    /// Los argumentos van a `method`, `method+1`, …
    pub fn inc(&mut self, sub: u8, method: u32, args: &[u32]) -> &mut Self {
        self.words.push(Self::header(1, sub, method, args.len() as u32));
        self.words.extend_from_slice(args);
        self
    }
    /// Todos los argumentos van al mismo método.
    pub fn non_inc(&mut self, sub: u8, method: u32, args: &[u32]) -> &mut Self {
        self.words.push(Self::header(3, sub, method, args.len() as u32));
        self.words.extend_from_slice(args);
        self
    }
    pub fn inc_once(&mut self, sub: u8, method: u32, args: &[u32]) -> &mut Self {
        self.words.push(Self::header(5, sub, method, args.len() as u32));
        self.words.extend_from_slice(args);
        self
    }
    /// Un único argumento de hasta 13 bits dentro de la propia cabecera.
    pub fn imm(&mut self, sub: u8, method: u32, data: u32) -> &mut Self {
        assert!(data < 0x2000);
        self.words.push(Self::header(4, sub, method, data));
        self
    }
    pub fn bytes(&self) -> Vec<u8> {
        self.words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
}
