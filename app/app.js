import { listGames, putGame, deleteGame, newId } from './db.js';
import { SYSTEMS, SWITCH_UNSUPPORTED_EXT, detect, extOf, titleOf, initialsOf } from './systems.js';
import { runNro } from './nx.js';

/* ---------------------------------------------------------------- utilidades */
const $ = (s) => document.querySelector(s);
const el = (tag, props = {}, ...kids) => {
  const e = Object.assign(document.createElement(tag), props);
  e.append(...kids);
  return e;
};

const store = {
  get(k, d) { try { const v = localStorage.getItem(k); return v == null ? d : JSON.parse(v); } catch { return d; } },
  set(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch { /* almacenamiento bloqueado */ } },
};

const DEFAULTS = { theme: 'auto', group: true, sort: 'recent', snapshot: true, wake: true, pause: true, onboarded: false, installDismissed: false, checklist: [] };
const cfg = { ...DEFAULTS, ...store.get('nexus.cfg', {}) };
const saveCfg = () => store.set('nexus.cfg', cfg);

function toast(msg, ms = 3200) {
  const t = $('#toast');
  t.textContent = msg;
  t.classList.add('show');
  clearTimeout(toast.t);
  toast.t = setTimeout(() => t.classList.remove('show'), ms);
}

const mb = (n) => (n / 1048576).toFixed(n < 10485760 ? 1 : 0);

function applyTheme() {
  document.documentElement.dataset.theme = cfg.theme;
  const dark = cfg.theme === 'dark' || (cfg.theme === 'auto' && !matchMedia('(prefers-color-scheme: light)').matches);
  document.querySelector('meta[name=theme-color]').content = dark ? '#0e0f13' : '#f3f4f8';
}

/* ------------------------------------------------------------------ pestañas */
const TITLES = { library: 'Biblioteca', switch: 'Switch', settings: 'Ajustes' };
function showTab(name) {
  document.querySelectorAll('.tab').forEach((t) => t.classList.toggle('active', t.id === `tab-${name}`));
  document.querySelectorAll('.tabs button').forEach((b) => {
    b.classList.toggle('active', b.dataset.tab === name);
    b.setAttribute('aria-current', b.dataset.tab === name ? 'page' : 'false');
  });
  $('#title').textContent = TITLES[name];
  $('#add').hidden = $('#search-toggle').hidden = name !== 'library';
  if (name === 'settings') updateStorage();
  scrollTo(0, 0);
}
document.querySelectorAll('.tabs button').forEach((b) => b.addEventListener('click', () => showTab(b.dataset.tab)));

/* ----------------------------------------------------------------- biblioteca */
let games = [];
let query = '';
let filter = 'all';
let urls = [];

const byRecent = (a, b) => (b.lastPlayed || 0) - (a.lastPlayed || 0) || b.added - a.added;
const SORTS = {
  recent: byRecent,
  name: (a, b) => a.title.localeCompare(b.title, 'es', { sensitivity: 'base' }),
  added: (a, b) => b.added - a.added,
};

async function refresh() {
  games = await listGames();
  render();
}

function coverEl(g) {
  const sys = SYSTEMS[g.system];
  const c = el('div', { className: 'cover' });
  c.style.background = `linear-gradient(135deg, ${sys.color}, ${sys.color}aa)`;
  if (g.cover) {
    const u = URL.createObjectURL(g.cover);
    urls.push(u);
    c.append(el('img', { src: u, alt: '', loading: 'lazy', decoding: 'async' }));
  } else {
    c.append(initialsOf(g.title));
  }
  c.append(el('span', { className: 'badge', textContent: sys.short }));
  if (g.favorite) c.append(el('span', { className: 'fav', textContent: '★', ariaLabel: 'Favorito' }));
  return c;
}

function cell(g) {
  const open = el('button', { className: 'game', ariaLabel: `Jugar a ${g.title}`, onclick: () => play(g.id) }, coverEl(g), el('b', { textContent: g.title }));
  // pulsación larga = opciones (como en el resto de apps de iOS)
  let timer;
  open.addEventListener('touchstart', () => { timer = setTimeout(() => { timer = null; navigator.vibrate?.(10); openSheet(g); }, 520); }, { passive: true });
  ['touchend', 'touchmove', 'touchcancel'].forEach((ev) => open.addEventListener(ev, () => { clearTimeout(timer); }, { passive: true }));
  open.addEventListener('contextmenu', (e) => { e.preventDefault(); openSheet(g); });
  const more = el('button', { className: 'more', textContent: '⋯', ariaLabel: `Opciones de ${g.title}`, onclick: () => openSheet(g) });
  return el('div', { className: 'cell' }, open, more);
}

function section(title, list, { scroll = false, count = false } = {}) {
  const wrap = el('section');
  wrap.append(el('div', { className: 'section-title' }, el('h2', { textContent: title }), count ? el('small', { textContent: String(list.length) }) : ''));
  wrap.append(el('div', { className: scroll ? 'row-scroll' : 'grid' }, ...list.map(cell)));
  return wrap;
}

function render() {
  urls.forEach((u) => URL.revokeObjectURL(u));
  urls = [];
  const q = query.trim().toLowerCase();
  const used = Object.keys(SYSTEMS).filter((k) => games.some((g) => g.system === k));
  if (filter !== 'all' && filter !== 'fav' && !used.includes(filter)) filter = 'all';

  $('#chips').replaceChildren(
    ...[['all', 'Todos'], ...(games.some((g) => g.favorite) ? [['fav', '★ Favoritos']] : []), ...used.map((k) => [k, SYSTEMS[k].name])].map(([k, label]) =>
      el('button', { className: 'chip' + (filter === k ? ' active' : ''), textContent: label, role: 'tab', ariaSelected: String(filter === k), onclick: () => { filter = k; render(); } }),
    ),
  );
  $('#chips').hidden = games.length === 0;
  $('#empty').hidden = games.length > 0;

  const sorter = SORTS[cfg.sort] || byRecent;
  const matches = games.filter((g) => (!q || g.title.toLowerCase().includes(q) || SYSTEMS[g.system].name.toLowerCase().includes(q)) && (filter === 'all' || (filter === 'fav' ? g.favorite : g.system === filter)));
  const lib = $('#library');
  lib.replaceChildren();
  if (games.length && !matches.length) {
    lib.append(el('p', { className: 'empty', textContent: q ? `Sin resultados para «${query}».` : 'No hay juegos en este filtro.' }));
    return;
  }
  const browsing = !q && filter === 'all';
  if (browsing) {
    const recent = games.filter((g) => g.lastPlayed).sort(byRecent).slice(0, 8);
    if (recent.length) lib.append(section('Seguir jugando', recent, { scroll: true }));
    const favs = games.filter((g) => g.favorite).sort(sorter);
    if (favs.length) lib.append(section('Favoritos', favs, { scroll: true }));
  }
  if (cfg.group && browsing) {
    for (const k of used) lib.append(section(SYSTEMS[k].name, matches.filter((g) => g.system === k).sort(sorter), { count: true }));
  } else {
    lib.append(section(browsing ? 'Todos los juegos' : 'Resultados', matches.sort(sorter), { count: true }));
  }
}

$('#search-toggle').onclick = () => {
  const bar = $('#searchbar');
  bar.hidden = !bar.hidden;
  $('#search-toggle').setAttribute('aria-expanded', String(!bar.hidden));
  if (bar.hidden) { query = ''; $('#search').value = ''; render(); } else $('#search').focus();
};
$('#search').oninput = (e) => { query = e.target.value; render(); };

/* --------------------------------------------------------------- hoja de opciones */
function sheetButton(label, fn, cls = '') {
  return el('button', { className: `btn ${cls}`.trim(), textContent: label, onclick: async () => { await fn(); } });
}

function openSheet(g) {
  const d = $('#sheet');
  const sys = SYSTEMS[g.system];
  const close = () => d.close();
  const head = el('div', { className: 'sheet-head' }, coverEl(g), el('div', {}, el('h2', { textContent: g.title }), el('div', { className: 'sub', textContent: `${sys.name} · ${mb(g.size)} MB${g.lastPlayed ? ' · jugado ' + new Date(g.lastPlayed).toLocaleDateString('es') : ''}` })));
  d.replaceChildren(
    head,
    el('div', { className: 'list' },
      sheetButton('▶  Jugar', () => { close(); play(g.id); }, 'primary'),
      sheetButton(g.favorite ? '☆  Quitar de favoritos' : '★  Añadir a favoritos', async () => { g.favorite = !g.favorite; await putGame(g); close(); refresh(); }),
      sheetButton('✎  Renombrar', async () => { const t = prompt('Nuevo nombre', g.title); if (t?.trim()) { g.title = t.trim(); await putGame(g); close(); refresh(); } }),
      sheetButton('🖼  Elegir carátula…', () => { close(); pickCover(g); }),
      ...(g.cover ? [sheetButton('Quitar carátula', async () => { delete g.cover; g.coverCustom = false; await putGame(g); close(); refresh(); })] : []),
      sheetButton('🗑  Eliminar del dispositivo', async () => { if (confirm(`¿Eliminar «${g.title}» y sus partidas guardadas?`)) { await deleteGame(g.id); close(); refresh(); } }, 'danger'),
      sheetButton('Cerrar', close),
    ),
  );
  d.showModal();
}
document.querySelectorAll('dialog.sheet').forEach((d) => d.addEventListener('click', (e) => { if (e.target === d) d.close(); }));

/* ---------------------------------------------------------------- carátulas */
let coverTarget = null;
function pickCover(g) { coverTarget = g; $('#file-cover').click(); }

/** Reduce una imagen a 512 px (lado mayor) y la devuelve como JPEG. */
async function shrink(blob, max = 512) {
  const bmp = await createImageBitmap(blob);
  const k = Math.min(1, max / Math.max(bmp.width, bmp.height));
  const c = document.createElement('canvas');
  c.width = Math.round(bmp.width * k);
  c.height = Math.round(bmp.height * k);
  c.getContext('2d').drawImage(bmp, 0, 0, c.width, c.height);
  bmp.close?.();
  return new Promise((res) => c.toBlob((b) => res(b), 'image/jpeg', 0.85));
}

$('#file-cover').onchange = async (e) => {
  const f = e.target.files[0];
  e.target.value = '';
  if (!f || !coverTarget) return;
  try {
    coverTarget.cover = await shrink(f);
    coverTarget.coverCustom = true;
    await putGame(coverTarget);
    toast('Carátula actualizada');
    refresh();
  } catch { toast('No se pudo usar esa imagen'); }
  coverTarget = null;
};

/* ------------------------------------------------------------------ importar */
function askSystem(filename, choices) {
  return new Promise((resolve) => {
    const d = $('#picker');
    let chosen = null;
    d.replaceChildren(
      el('h2', { textContent: 'Elige la consola' }),
      el('div', { className: 'sub', textContent: filename }),
      el('div', { className: 'list' },
        ...choices.map((k) => sheetButton(SYSTEMS[k].name, () => { chosen = k; d.close(); })),
        sheetButton('Omitir este archivo', () => d.close()),
      ),
    );
    d.addEventListener('close', () => resolve(chosen), { once: true });
    d.showModal();
  });
}

async function importFiles(files) {
  let added = 0, dup = 0, skipped = 0;
  const total = files.length;
  for (const [i, f] of files.entries()) {
    if (total > 1) toast(`Importando ${i + 1} de ${total}…`, 60000);
    const ext = extOf(f.name);
    if (SWITCH_UNSUPPORTED_EXT.includes(ext)) {
      toast('Los juegos de Switch (XCI/NSP) no se pueden ejecutar en Nexus. Mira la pestaña Switch.', 6000);
      skipped++;
      continue;
    }
    const det = detect(f.name);
    let system = det?.system;
    if (det?.choices) system = await askSystem(f.name, det.choices);
    if (!system) { if (!det) toast(`Formato no soportado: .${ext}`); skipped++; continue; }
    if (system === 'switch') { await runNroFile(f); showTab('switch'); continue; }
    if (games.some((g) => g.filename === f.name && g.size === f.size)) { dup++; continue; }
    try {
      const g = { id: newId(), title: titleOf(f.name), filename: f.name, system, size: f.size, added: Date.now(), blob: f };
      await putGame(g);
      games.push(g);
      added++;
    } catch (e) {
      toast(e?.name === 'QuotaExceededError' ? 'No hay espacio suficiente en el dispositivo' : 'No se pudo guardar el juego', 5000);
      skipped++;
    }
  }
  const parts = [];
  if (added) parts.push(`${added} juego${added > 1 ? 's' : ''} añadido${added > 1 ? 's' : ''}`);
  if (dup) parts.push(`${dup} ya estaba${dup > 1 ? 'n' : ''}`);
  if (parts.length) toast(parts.join(' · '));
  else if (!skipped) toast('Nada que importar');
  await refresh();
}

$('#add').onclick = $('#empty-add').onclick = () => $('#file').click();
$('#file').onchange = (e) => { const f = [...e.target.files]; e.target.value = ''; importFiles(f); };
addEventListener('dragover', (e) => e.preventDefault());
addEventListener('drop', (e) => { e.preventDefault(); importFiles([...e.dataTransfer.files]); });
$('#formats').textContent = Object.values(SYSTEMS).filter((s) => s.ejs).map((s) => `${s.name}: ${s.ext.map((x) => '.' + x).join(' ')}`).join(' · ') + ' · también .zip y .7z.';

/* -------------------------------------------------------------------- jugar */
let playing = null;
let wake = null;
let pausedByUs = false;
const frame = () => $('#frame');

async function lockScreen() {
  if (!cfg.wake || !('wakeLock' in navigator)) return;
  try {
    wake = await navigator.wakeLock.request('screen');
    wake.addEventListener('release', () => { wake = null; });
  } catch { /* no disponible */ }
}
const unlockScreen = () => { wake?.release?.(); wake = null; };

function play(id) {
  playing = id;
  frame().src = `play.html?id=${encodeURIComponent(id)}`;
  $('#player').hidden = false;
  document.documentElement.style.overflow = 'hidden';
  lockScreen();
}

function requestSnapshot(ms = 1500) {
  return new Promise((resolve) => {
    const done = (blob) => { clearTimeout(timer); removeEventListener('message', on); resolve(blob); };
    const on = (e) => { if (e.origin === location.origin && e.source === frame().contentWindow && e.data?.type === 'nexus-snapshot') done(e.data.blob); };
    const timer = setTimeout(() => done(null), ms);
    addEventListener('message', on);
    frame().contentWindow?.postMessage({ type: 'nexus-snapshot-request' }, location.origin);
  });
}

async function exitGame() {
  if (!playing) return;
  const id = playing;
  playing = null;
  let blob = null;
  if (cfg.snapshot) { playing = id; blob = await requestSnapshot(); playing = null; }
  const g = games.find((x) => x.id === id);
  if (g && blob && !g.coverCustom) {
    try { g.cover = await shrink(blob); await putGame(g); } catch { /* sin carátula */ }
  }
  frame().src = 'about:blank'; // descarga el núcleo y detiene el audio
  $('#player').hidden = true;
  document.documentElement.style.overflow = '';
  unlockScreen();
  refresh();
}
$('#exit').onclick = exitGame;

document.addEventListener('visibilitychange', () => {
  if (!playing) return;
  const w = frame().contentWindow;
  if (document.hidden) {
    if (cfg.pause) { w?.postMessage({ type: 'nexus-pause' }, location.origin); pausedByUs = true; }
  } else {
    if (pausedByUs) { w?.postMessage({ type: 'nexus-resume' }, location.origin); pausedByUs = false; }
    if (!wake) lockScreen();
  }
});

/* ------------------------------------------------------------- Switch (guía) */
const CHECKLIST = [
  'Mi iPhone es un iPhone 12 o posterior (recomendado: 15 Pro o superior).',
  'Tengo un ordenador (Windows o Mac) para la primera instalación por sideload.',
  'He instalado SideStore o AltStore desde su página oficial.',
  'He instalado MeloNX desde su página oficial (no desde sitios que reempaquetan el IPA).',
  'Sé activar el JIT (por ejemplo con StikDebug) cada vez que abro MeloNX.',
  'Tengo las claves y el firmware de MI PROPIA Switch.',
  'Tengo copias de MIS propios juegos.',
];

function renderChecklist() {
  const ul = $('#checklist');
  ul.replaceChildren(...CHECKLIST.map((text, i) => {
    const id = `chk${i}`;
    const box = el('input', { type: 'checkbox', id, checked: !!cfg.checklist[i], onchange: (e) => { cfg.checklist[i] = e.target.checked; saveCfg(); progress(); } });
    return el('li', {}, box, el('label', { htmlFor: id, textContent: text }));
  }));
  const progress = () => { $('#checklist-progress').textContent = `${CHECKLIST.filter((_, i) => cfg.checklist[i]).length} de ${CHECKLIST.length} pasos listos`; };
  progress();
}

const con = $('#console');
const show = (t) => { con.hidden = false; con.textContent = t; };
async function runNroBytes(bytes, name) {
  show(`Ejecutando ${name}…`);
  try {
    const { code, output, error } = await runNro(bytes);
    show(`${output}${error ? `\n[error] ${error}` : `\n[proceso terminó con código ${code}]`}`);
  } catch (e) { show(`[error] ${e.message}`); }
}
const runNroFile = async (f) => runNroBytes(new Uint8Array(await f.arrayBuffer()), f.name);
$('#run-demo').onclick = async () => runNroBytes(new Uint8Array(await (await fetch('emu/demo.nro')).arrayBuffer()), 'demo.nro');
$('#run-nro').onclick = () => $('#file-nro').click();
$('#file-nro').onchange = (e) => { if (e.target.files[0]) runNroFile(e.target.files[0]); e.target.value = ''; };

/* ------------------------------------------------------------------- ajustes */
function bindSettings() {
  const map = [['#set-theme', 'theme', 'value'], ['#set-sort', 'sort', 'value'], ['#set-group', 'group', 'checked'], ['#set-cover', 'snapshot', 'checked'], ['#set-wake', 'wake', 'checked'], ['#set-pause', 'pause', 'checked']];
  for (const [sel, key, prop] of map) {
    const e = $(sel);
    e[prop] = cfg[key];
    e.onchange = () => { cfg[key] = e[prop]; saveCfg(); if (key === 'theme') applyTheme(); render(); };
  }
}

async function updateStorage() {
  const total = games.reduce((s, g) => s + g.size, 0);
  let quota = '';
  try { const e = await navigator.storage.estimate(); quota = ` · usado por Nexus (total): ${mb(e.usage)} de ${mb(e.quota)} MB`; } catch { /* sin API */ }
  $('#usage').textContent = `${games.length} juego${games.length === 1 ? '' : 's'} · ${mb(total)} MB${quota}`;
  let persisted = false;
  try { persisted = await navigator.storage.persisted(); } catch { /* sin API */ }
  $('#persist-status').textContent = persisted ? '✓ Datos protegidos: el navegador no los borrará por falta de uso.' : 'Los datos aún no están protegidos contra borrado automático.';
  $('#persist').hidden = persisted;
}

$('#persist').onclick = async () => {
  let ok = false;
  try { ok = await navigator.storage.persist(); } catch { /* sin API */ }
  toast(ok ? 'Datos protegidos' : 'Safari no lo concedió. Instala Nexus en la pantalla de inicio e inténtalo de nuevo.', 5000);
  updateStorage();
};
$('#clear-cache').onclick = async () => {
  try { for (const k of await caches.keys()) await caches.delete(k); toast('Caché vaciada'); } catch { toast('No se pudo vaciar la caché'); }
};
$('#clear').onclick = async () => {
  if (!confirm('¿Borrar TODOS los juegos de la biblioteca? No se puede deshacer.')) return;
  for (const g of games) await deleteGame(g.id);
  await refresh();
  updateStorage();
};

/* mandos Bluetooth */
const pads = new Set();
function padStatus() {
  $('#pad-status').textContent = pads.size ? `🎮 Mando conectado: ${[...pads].join(', ')}` : 'Mandos: ninguno conectado. Empareja uno por Bluetooth en Ajustes de iOS y pulsa un botón.';
}
addEventListener('gamepadconnected', (e) => { pads.add(e.gamepad.id.split('(')[0].trim()); padStatus(); toast('Mando conectado'); });
addEventListener('gamepaddisconnected', (e) => { pads.delete(e.gamepad.id.split('(')[0].trim()); padStatus(); });

/* ------------------------------------------------------- guía de inicio y aviso */
function openHelp() {
  const d = $('#help');
  d.replaceChildren(
    el('h2', { textContent: 'Bienvenido a Nexus' }),
    el('div', { className: 'sub', textContent: 'Tus consolas clásicas, en una sola app.' }),
    el('ol', { className: 'steps' },
      el('li', {}, el('b', { textContent: 'Instálala: ' }), 'en Safari pulsa Compartir y «Añadir a pantalla de inicio». Así se abre a pantalla completa y funciona sin conexión.'),
      el('li', {}, el('b', { textContent: 'Añade tus juegos: ' }), 'pulsa ＋ y elige archivos de tus propios cartuchos o discos. Nexus no incluye ni descarga juegos.'),
      el('li', {}, el('b', { textContent: 'Juega: ' }), 'toca un juego. Usa los controles táctiles o un mando Bluetooth. El menú del juego guarda y carga partidas.'),
      el('li', {}, el('b', { textContent: 'Organiza: ' }), 'mantén pulsado un juego para marcarlo como favorito, cambiar su carátula o eliminarlo.'),
    ),
    el('p', { className: 'muted', textContent: 'Switch: Nexus no puede ejecutar juegos de Switch. La pestaña «Switch» te guía para usar MeloNX en tu iPhone.' }),
    el('div', { className: 'list' }, sheetButton('Entendido', () => { cfg.onboarded = true; saveCfg(); d.close(); }, 'primary')),
  );
  d.showModal();
}
$('#show-help').onclick = openHelp;

function maybeInstallBanner() {
  const ios = /iPad|iPhone|iPod/.test(navigator.userAgent) || (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);
  const standalone = navigator.standalone === true || matchMedia('(display-mode: standalone)').matches;
  $('#install').hidden = !(ios && !standalone && !cfg.installDismissed);
}
$('#install-close').onclick = () => { cfg.installDismissed = true; saveCfg(); $('#install').hidden = true; };

/* -------------------------------------------------------------------- arranque */
applyTheme();
matchMedia('(prefers-color-scheme: light)').addEventListener?.('change', applyTheme);
bindSettings();
renderChecklist();
showTab('library');
maybeInstallBanner();
refresh().then(() => { if (!cfg.onboarded && games.length === 0) setTimeout(openHelp, 400); });
if ('serviceWorker' in navigator) navigator.serviceWorker.register('sw.js').catch(() => {});
try { navigator.storage?.persist?.(); } catch { /* opcional */ }
