// Service worker: guarda la app (y los núcleos al usarlos) para jugar sin conexión.
const CACHE = 'nexus-v4';
const SHELL = ['./', 'index.html', 'style.css', 'app.js', 'db.js', 'systems.js', 'i18n.js', 'nx.js', 'play.html', 'manifest.webmanifest',
  'emu/nxemu.wasm', 'emu/demo.nro', 'icons/icon.svg', 'icons/icon-180.png', 'icons/icon-192.png', 'icons/icon-512.png'];

self.addEventListener('install', (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(SHELL)).then(() => self.skipWaiting()));
});
self.addEventListener('activate', (e) => {
  e.waitUntil(caches.keys().then((ks) => Promise.all(ks.filter((k) => k !== CACHE).map((k) => caches.delete(k)))).then(() => self.clients.claim()));
});
self.addEventListener('fetch', (e) => {
  const { request } = e;
  if (request.method !== 'GET' || new URL(request.url).origin !== location.origin) return;
  // Código de la app: red primero (así las actualizaciones llegan); resto (núcleos grandes): caché primero.
  const isCode = /\.(html|js|css|webmanifest)$/.test(new URL(request.url).pathname) || request.mode === 'navigate';
  e.respondWith(
    isCode
      ? fetch(request).then((res) => { if (res.ok) { const copy = res.clone(); caches.open(CACHE).then((c) => c.put(request, copy)); } return res; }).catch(() => caches.match(request))
      : caches.match(request).then((hit) => hit || fetch(request).then((res) => { if (res.ok) { const copy = res.clone(); caches.open(CACHE).then((c) => c.put(request, copy)); } return res; })),
  );
});
