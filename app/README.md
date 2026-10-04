# Nexus

App web instalable (PWA) al estilo Delta: biblioteca de juegos, estados guardados, controles táctiles y mandos
Bluetooth, todo en el dispositivo. Pensada para iPhone (Safari → Compartir → *Añadir a pantalla de inicio*).

## Qué incluye
| Sistema | Cómo se ejecuta |
|---|---|
| NES, SNES, Game Boy/Color, GBA, N64, DS, Mega Drive | Núcleos libretro de [EmulatorJS](https://emulatorjs.org) en WebAssembly, autoalojados (sin CDN, funciona sin conexión) |
| **Switch · homebrew `.nro`** | Núcleo propio [`nxemu`](../nxemu) (CPU AArch64 en Rust → WebAssembly). Experimental: solo programas de prueba |
| **Switch · juegos completos** | Juego remoto: emulas en un PC/Mac (Eden/Ryubing) y transmites al iPhone (Sunshine + Moonlight, Steam Link, Parsec) |

> Ninguna web puede ejecutar juegos comerciales de Switch de forma local en un iPhone: la Switch necesita un JIT de CPU,
> una GPU Maxwell traducida a Metal y cientos de servicios del SO. Por eso Nexus combina un núcleo propio (base) con juego remoto (completo).

## Desarrollo
    cd app
    npm install
    npm run vendor      # copia EmulatorJS y los núcleos a vendor/ (no se versiona)
    npm run serve       # http://localhost:8080
    CHROMIUM=/ruta/chrome npm test   # prueba e2e en navegador real

Para recompilar el núcleo de Switch: `rustup target add wasm32-unknown-unknown && npm run wasm`.

## Publicar (necesario HTTPS para instalar en iPhone)
El workflow `.github/workflows/pages.yml` despliega `app/` en GitHub Pages
(Settings → Pages → Source: *GitHub Actions*).

## Privacidad y legalidad
Los juegos se guardan solo en IndexedDB del dispositivo. Nexus no incluye juegos, BIOS, claves ni firmware; usa
únicamente copias de tu propiedad. Los núcleos de EmulatorJS/libretro tienen sus propias licencias (GPL/LGPL, etc.).
