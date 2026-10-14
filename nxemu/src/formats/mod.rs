//! Formatos de archivo **sin cifrar** de la plataforma: ejecutables NSO, contenedores PFS0/HFS0 (ExeFS, NSP sin
//! cifrar), y utilidades de compresión/hash que necesitan (LZ4, SHA-256).
//!
//! No incluye descifrado de contenido comercial (NCA/XCI cifrados) ni gestión de claves.

pub mod lz4;
pub mod nso;
pub mod pfs;
pub mod sha256;
