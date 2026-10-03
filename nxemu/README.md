# nxemu

Base educativa de un emulador de Nintendo Switch, escrita en Rust.

## Qué hace hoy
- CPU AArch64: subconjunto de instrucciones enteras (mov, add/sub con flags, lógicas, mul/div, lsl/lsr, ldr/str, ldp/stp, saltos, b.cond, cbz, adr/adrp, svc).
- Memoria virtual por páginas.
- Cargador de ejecutables homebrew `.nro`.
- Kernel HLE mínimo: `ExitProcess`, `Break`, `OutputDebugString`.

## Qué NO hace
No ejecuta juegos comerciales. Faltan SIMD/FP, el resto de SVC y servicios de Horizon,
formatos NSO/NSP/XCI, descifrado, GPU (Maxwell → Vulkan/Metal), audio y entrada.

## Uso
    cargo test
    cargo run -- programa.nro

El programa de prueba está en `tests/asm/prog.s` (se ensambla con `llvm-mc`).
