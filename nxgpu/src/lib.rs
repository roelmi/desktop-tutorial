//! nxgpu: base de una GPU emulada.
//!
//! - [`fifo`]: decodifica *pushbuffers* (formato de comandos de las GPU de NVIDIA).
//! - [`ir`], [`asm`]: lenguaje intermedio de shaders y su ensamblador de texto.
//! - [`interp`]: intérprete de shaders en CPU.
//! - `translate` (característica `translate`): IR → WGSL → SPIR-V (Vulkan) y MSL (Metal), validado con naga.
//! - [`raster`]: rasterizador por software (referencia exacta para futuros motores Vulkan/Metal).
//! - [`engine`]: motor 3D de juguete que consume comandos del FIFO y dibuja con el rasterizador.
//!
//! Importante: no decodifica shaders binarios reales de la Switch (SASS de Maxwell/Ampere) ni usa la tabla de
//! registros real de sus motores; eso requiere documentación y volcados verificables que no están aquí.

pub mod asm;
pub mod engine;
pub mod fifo;
pub mod interp;
pub mod ir;
pub mod raster;
#[cfg(feature = "translate")]
pub mod translate;

/// Memoria a la que accede la GPU (en una consola real, memoria del sistema vista por la MMU de la GPU).
pub trait GpuMemory {
    fn read(&self, addr: u64, buf: &mut [u8]) -> bool;
    fn write(&mut self, addr: u64, data: &[u8]) -> bool;
}

/// Memoria plana de pruebas: la dirección es un desplazamiento en el vector.
#[derive(Default)]
pub struct FlatMemory(pub Vec<u8>);

impl FlatMemory {
    pub fn new(size: usize) -> Self {
        Self(vec![0; size])
    }
}

impl GpuMemory for FlatMemory {
    fn read(&self, addr: u64, buf: &mut [u8]) -> bool {
        let a = addr as usize;
        match self.0.get(a..a + buf.len()) {
            Some(s) => {
                buf.copy_from_slice(s);
                true
            }
            None => false,
        }
    }
    fn write(&mut self, addr: u64, data: &[u8]) -> bool {
        let a = addr as usize;
        match self.0.get_mut(a..a + data.len()) {
            Some(s) => {
                s.copy_from_slice(data);
                true
            }
            None => false,
        }
    }
}
