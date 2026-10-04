// Service worker: guarda la app (y los núcleos al usarlos) para jugar sin conexión.
const CACHE = 'nexus-v1';
const SHELL = ['./', 'index.html', 'style.css', 'app.js', 'db.js', 'systems.js', 'nx.js', 'play.html',
  'manifest.webmanifest', 'emu/nxemu.wasm', 'emu/demo.nro', 'icons/icon.svg'];

self.addEventListener('install', (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(SHELL)).then(() => self.skipWaiting()));
});
self.addEventListener('activate', (e) => {
  e.waitUntil(caches.keys().then((ks) => Promise.all(ks.filter((k) => k !== CACHE).map((k) => caches.delete(k)))).then(() => self.clients.claim()));
});
self.addEventListener('fetch', (e) => {
  const { request } = e;
  if (request.method !== 'GET' || new URL(request.url).origin !== location.origin) return;
  e.respondWith(
    caches.match(request).then((hit) => hit || fetch(request).then((res) => {
      if (res.ok) { const copy = res.clone(); caches.open(CACHE).then((c) => c.put(request, copy)); }
      return res;
    })),
  );
});
