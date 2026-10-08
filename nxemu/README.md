# nxemu

Base educativa de un emulador de Nintendo Switch, escrita en Rust.

## Qué hace hoy
- CPU AArch64 (enteros): aritmética con flags, lógicas (incluye inmediatos), campos de bits (UBFM/SBFM/BFM/EXTR),
  mul/div (incl. `UMULH`/`SMULH`), `CLZ`/`RBIT`/`REV`, selecciones condicionales (`CSEL`…), todas las formas de
  `LDR`/`STR`/`LDP`/`STP` (con signo, registro, pre/post-índice), exclusivos, saltos (`B.cond`, `CBZ`, `TBZ`, `BR`/`BLR`),
  `SVC` y `TPIDR_EL0`.
- **Verificación diferencial**: `tests/c/*.c` se compilan con LLVM a AArch64 (5 niveles de optimización) y su resultado
  debe coincidir con la ejecución nativa (`tests/c/build.sh` regenera los fixtures).
- Memoria virtual por páginas.
- Cargador de ejecutables homebrew `.nro`.
- Kernel HLE mínimo: `ExitProcess`, `Break`, `OutputDebugString`.

## Qué NO hace
No ejecuta juegos comerciales. Faltan SIMD/FP (NEON), atómicos LSE, tablas de páginas, el resto de SVC y servicios de Horizon,
formatos NSO/NSP/XCI, descifrado, GPU (Maxwell → Vulkan/Metal), audio y entrada.

## Uso
    cargo test
    cargo run -- programa.nro

El programa de prueba está en `tests/asm/prog.s` (se ensambla con `llvm-mc`).
