// Servidor estático mínimo para desarrollo: node scripts/serve.mjs [puerto]
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { join, extname, normalize, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css',
  '.json': 'application/json', '.webmanifest': 'application/manifest+json', '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml', '.png': 'image/png', '.data': 'application/octet-stream' };

export function serve(port = 8080) {
  const srv = createServer(async (req, res) => {
    try {
      let p = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname));
      if (p.endsWith('/')) p += 'index.html';
      const file = join(root, p);
      if (!file.startsWith(root)) throw new Error('fuera de la raíz');
      if (!(await stat(file)).isFile()) throw new Error('no es archivo');
      res.writeHead(200, { 'Content-Type': TYPES[extname(file)] || 'application/octet-stream' });
      res.end(await readFile(file));
    } catch {
      res.writeHead(404).end('404');
    }
  });
  return new Promise((r) => srv.listen(port, () => r(srv)));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const port = Number(process.argv[2]) || 8080;
  await serve(port);
  console.log(`Nexus en http://localhost:${port}`);
}
