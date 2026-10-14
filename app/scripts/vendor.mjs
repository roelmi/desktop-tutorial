// Copia EmulatorJS y los núcleos elegidos desde node_modules a vendor/ejs
// (autoalojado: la app funciona sin conexión y sin depender de un CDN).
import { cpSync, mkdirSync, rmSync, existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const CORES = ['fceumm', 'snes9x', 'gambatte', 'mgba', 'mupen64plus_next', 'melonds', 'genesis_plus_gx', 'pcsx_rearmed', 'mednafen_pce', 'stella2014',
  'mednafen_ngp', 'mednafen_wswan', 'handy', 'prosystem', 'beetle_vb'];
const LANGS = ['en-US.json', 'es-ES.json', 'pt-BR.json', 'README.md'];

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const nm = join(root, 'node_modules', '@emulatorjs');
const out = join(root, 'vendor', 'ejs');
if (!existsSync(nm)) throw new Error('Ejecuta `npm install` primero.');

rmSync(out, { recursive: true, force: true });
cpSync(join(nm, 'emulatorjs', 'data'), out, {
  recursive: true,
  filter: (src) => !/[\\/]localization[\\/]/.test(src) || LANGS.some((l) => src.endsWith(l)),
});
mkdirSync(join(out, 'cores'), { recursive: true });
let n = 0;
for (const core of CORES) {
  // Solo variantes sin hilos: no exigen cabeceras COOP/COEP (SharedArrayBuffer),
  // así funcionan en hosting estático y en iOS.
  for (const variant of ['wasm', 'legacy-wasm']) {
    cpSync(join(nm, `core-${core}`, `${core}-${variant}.data`), join(out, 'cores', `${core}-${variant}.data`));
    n++;
  }
}

// El cargador busca primero emulator.min.js/.css; los generamos (mismo orden que loader.js) para evitar 404 y peticiones extra.
const SCRIPTS = ['emulator.js', 'nipplejs.js', 'shaders.js', 'storage.js', 'gamepad.js', 'GameManager.js', 'socket.io.min.js', 'compression.js'];
writeFileSync(join(out, 'emulator.min.js'), SCRIPTS.map((f) => readFileSync(join(out, 'src', f), 'utf8')).join('\n;\n'));
cpSync(join(out, 'emulator.css'), join(out, 'emulator.min.css'));
console.log(`vendor/ejs listo: ${n} archivos de núcleo.`);
