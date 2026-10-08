# nxemu

Base educativa de un emulador de Nintendo Switch, escrita en Rust.

## Qué hace hoy
- CPU AArch64 (enteros): aritmética con flags, lógicas (incluye inmediatos), campos de bits (UBFM/SBFM/BFM/EXTR),
  mul/div (incl. `UMULH`/`SMULH`), `CLZ`/`RBIT`/`REV`, selecciones condicionales (`CSEL`…), todas las formas de
  `LDR`/`STR`/`LDP`/`STP` (con signo, registro, pre/post-índice), exclusivos, saltos (`B.cond`, `CBZ`, `TBZ`, `BR`/`BLR`),
  `SVC` y `TPIDR_EL0`.
- Coma flotante escalar f32/f64 (aritmética, `FMADD` fusionado, raíz, redondeos, conversiones, comparaciones).
- NEON: aritmética/lógica/desplazamientos enteros, comparaciones, saturación, coma flotante vectorial, permutaciones
  (`ZIP`/`UZP`/`TRN`/`EXT`/`TBL`), ensanchado/estrechado, carriles, `LD1`–`LD4`/`ST1`–`ST4`/`LD1R`.
- Atómicos LSE (`LDADD`, `SWP`, `CAS`…) y exclusivos.
- Memoria virtual con permisos por página (R/W/X), `protect`/`unmap`/`query`, y fallos de acceso.
- **Verificación diferencial**: `tests/c/*.c` se compilan con LLVM a AArch64 (5 niveles de optimización) y su resultado
  debe coincidir con la ejecución nativa (`tests/c/build.sh` regenera los fixtures).
- Memoria virtual por páginas.
- Cargador de ejecutables homebrew `.nro`.
- Kernel HLE mínimo: `ExitProcess`, `Break`, `OutputDebugString`.

## Qué NO hace
No ejecuta juegos comerciales. Faltan: parte de NEON (estimaciones, criptografía, media precisión), `CASP`, tablas de páginas ARM reales, el resto de SVC y servicios de Horizon,
formatos NSO/NSP/XCI, descifrado, GPU (Maxwell → Vulkan/Metal), audio y entrada.

## Uso
    cargo test
    cargo run -- programa.nro

El programa de prueba está en `tests/asm/prog.s` (se ensambla con `llvm-mc`).
