# Nexus

App web instalable (PWA) al estilo Delta: biblioteca de juegos, estados guardados, controles táctiles y mandos
Bluetooth, todo en el dispositivo. Pensada para iPhone (Safari → Compartir → *Añadir a pantalla de inicio*).

## Qué incluye
| Sistema | Cómo se ejecuta |
|---|---|
| NES, SNES, Game Boy/Color, GBA, N64, DS, Mega Drive, PlayStation, Master System, Game Gear, PC Engine, Atari 2600 | Núcleos libretro de [EmulatorJS](https://emulatorjs.org) en WebAssembly, autoalojados (sin CDN, funciona sin conexión) |
| **Switch · homebrew `.nro`** | Núcleo propio [`nxemu`](../nxemu) (CPU AArch64 en Rust → WebAssembly). Experimental: solo programas de prueba |

> Los juegos comerciales de Switch no se pueden ejecutar en un iPhone: la Switch necesita un JIT de CPU (iOS lo bloquea),
> una GPU Maxwell traducida a Metal y cientos de servicios del SO. `nxemu` es la base para ir ampliando, no un emulador jugable.

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

## Nexus (versión actual)
- 17 sistemas retro (EmulatorJS/libretro en WASM), biblioteca con portadas, favoritos, "Seguir jugando", búsqueda y ajustes.
- Switch: Nexus **no** ejecuta juegos retail (el navegador de iOS no permite JIT). La pestaña es una guía para instalar MeloNX aparte, más un laboratorio de homebrew con nxemu.
- Pruebas: `CHROMIUM=/ruta/chrome npm test` (8 pruebas e2e).
