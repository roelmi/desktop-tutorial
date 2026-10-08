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
- **Kernel HLE (estilo Horizon)** en `src/kernel`: proceso con varios hilos (planificador por prioridades), tabla de
  handles, tiempo virtual, SVC de memoria (`SetHeapSize`, `MapMemory`, `QueryMemory`, permisos…), hilos, mutex y
  variables de condición, eventos, `WaitSynchronization`, `WaitForAddress`, `GetInfo`, IPC (HIPC/CMIF) y servicios
  `sm:`, `set` y `apm`. Los comandos o SVC desconocidos se registran en `Kernel::log` en lugar de fallar en silencio.
  Se basa en la documentación pública de la Switch original: del sistema de la Switch 2 no hay documentación pública.
- **Formatos sin cifrar** (`src/formats`): NRO con reubicaciones MOD0 (`R_AARCH64_RELATIVE`/`ABS64`/`GLOB_DAT`/`JUMP_SLOT`),
  NSO (LZ4 + SHA-256), contenedores PFS0/HFS0 y arranque desde un ExeFS (`Kernel::boot_exefs`). **No** incluye descifrado de
  contenido comercial (NCA/XCI cifrados) ni gestión de claves.
- **Verificación diferencial**: `tests/c/*.c` se compilan con LLVM a AArch64 (5 niveles de optimización) y su resultado
  debe coincidir con la ejecución nativa (`tests/c/build.sh` regenera los fixtures).
- Memoria virtual por páginas.
- Cargador de ejecutables homebrew `.nro`.
- Kernel HLE mínimo: `ExitProcess`, `Break`, `OutputDebugString`.

## Qué NO hace
No ejecuta juegos comerciales. Faltan: la mayoría de servicios del sistema (fs, hid, vi, nvdrv, audren…), dominios IPC, parte de NEON (estimaciones, criptografía, media precisión), `CASP`, tablas de páginas ARM reales, el resto de SVC y servicios de Horizon,
formatos NSO/NSP/XCI, descifrado, GPU (Maxwell → Vulkan/Metal), audio y entrada.

## Uso
    cargo test
    cargo run -- programa.nro

Pruebas: `tests/asm/prog.s` (ensamblador), `tests/c` (verificación diferencial de la CPU) y `tests/guest`
(programas NRO reales que ejercitan el kernel; se compilan con `tests/guest/build.sh`).
