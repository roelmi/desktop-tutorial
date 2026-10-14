# desktop-tutorial

- [`app/`](app) — **Nexus**: app web instalable (PWA) estilo Delta con emuladores retro y soporte Switch (solo homebrew local; los juegos comerciales de Switch no corren en iPhone).
- [`nxgpu/`](nxgpu) — base de GPU emulada: procesador de comandos, IR de shaders, traducción a SPIR-V/MSL y rasterizador por software.
- [`nxemu/`](nxemu) — núcleo de Switch en Rust (CPU AArch64 + cargador NRO), compilable a WebAssembly.
