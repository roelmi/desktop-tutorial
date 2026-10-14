import { listGames, putGame, deleteGame, newId } from './db.js';
import { SYSTEMS, SWITCH_UNSUPPORTED_EXT, detect, extOf, titleOf, initialsOf } from './systems.js';
import { runNro } from './nx.js';
import { t, tn, getLang, setLang, applyStatic } from './i18n.js';

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
let currentTab = 'library';
function showTab(name) {
  currentTab = name;
  document.querySelectorAll('.tab').forEach((t) => t.classList.toggle('active', t.id === `tab-${name}`));
  document.querySelectorAll('.tabs button').forEach((b) => {
    b.classList.toggle('active', b.dataset.tab === name);
    b.setAttribute('aria-current', b.dataset.tab === name ? 'page' : 'false');
  });
  $('#title').textContent = t(`tab.${name}`);
  $('#add').hidden = $('#search-toggle').hidden = $('#big-open').hidden = name !== 'library';
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
  name: (a, b) => a.title.localeCompare(b.title, getLang(), { sensitivity: 'base' }),
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
  if (g.favorite) c.append(el('span', { className: 'fav', textContent: '★', ariaLabel: t('a11y.fav') }));
  return c;
}

function cell(g) {
  const open = el('button', { className: 'game', ariaLabel: t('a11y.play', { title: g.title }), onclick: () => play(g.id) }, coverEl(g), el('b', { textContent: g.title }));
  // pulsación larga = opciones (como en el resto de apps de iOS)
  let timer;
  open.addEventListener('touchstart', () => { timer = setTimeout(() => { timer = null; navigator.vibrate?.(10); openSheet(g); }, 520); }, { passive: true });
  ['touchend', 'touchmove', 'touchcancel'].forEach((ev) => open.addEventListener(ev, () => { clearTimeout(timer); }, { passive: true }));
  open.addEventListener('contextmenu', (e) => { e.preventDefault(); openSheet(g); });
  const more = el('button', { className: 'more', textContent: '⋯', ariaLabel: t('a11y.options', { title: g.title }), onclick: () => openSheet(g) });
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
    ...[['all', t('chip.all')], ...(games.some((g) => g.favorite) ? [['fav', t('chip.fav')]] : []), ...used.map((k) => [k, SYSTEMS[k].name])].map(([k, label]) =>
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
    lib.append(el('p', { className: 'empty', textContent: q ? t('lib.noResults', { q: query }) : t('lib.noFilter') }));
    return;
  }
  const browsing = !q && filter === 'all';
  if (browsing) {
    const recent = games.filter((g) => g.lastPlayed).sort(byRecent).slice(0, 8);
    if (recent.length) lib.append(section(t('lib.continue'), recent, { scroll: true }));
    const favs = games.filter((g) => g.favorite).sort(sorter);
    if (favs.length) lib.append(section(t('lib.favorites'), favs, { scroll: true }));
  }
  if (cfg.group && browsing) {
    for (const k of used) lib.append(section(SYSTEMS[k].name, matches.filter((g) => g.system === k).sort(sorter), { count: true }));
  } else {
    lib.append(section(browsing ? t('lib.all') : t('lib.results'), matches.sort(sorter), { count: true }));
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
  const head = el('div', { className: 'sheet-head' }, coverEl(g), el('div', {}, el('h2', { textContent: g.title }), el('div', { className: 'sub', textContent: `${sys.name}${g.system === 'melonx' ? ' · ' + t('sheet.shortcut') : ' · ' + mb(g.size) + ' MB'}${g.lastPlayed ? ' · ' + t('sheet.played', { date: new Date(g.lastPlayed).toLocaleDateString(getLang()) }) : ''}` })));
  d.replaceChildren(
    head,
    el('div', { className: 'list' },
      sheetButton(t('sheet.play'), () => { close(); play(g.id); }, 'primary'),
      sheetButton(g.favorite ? t('sheet.unfav') : t('sheet.fav'), async () => { g.favorite = !g.favorite; await putGame(g); close(); refresh(); }),
      sheetButton(t('sheet.rename'), async () => { const nn = prompt(t('prompt.rename'), g.title); if (nn?.trim()) { g.title = nn.trim(); await putGame(g); close(); refresh(); } }),
      sheetButton(t('sheet.cover'), () => { close(); pickCover(g); }),
      ...(g.cover ? [sheetButton(t('sheet.removeCover'), async () => { delete g.cover; g.coverCustom = false; await putGame(g); close(); refresh(); })] : []),
      sheetButton(t('sheet.delete'), async () => { if (confirm(g.system === 'melonx' ? t('confirm.removeShortcut', { title: g.title }) : t('confirm.deleteGame', { title: g.title }))) { await deleteGame(g.id); close(); refresh(); } }, 'danger'),
      sheetButton(t('sheet.close'), close),
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
    toast(t('toast.coverUpdated'));
    refresh();
  } catch { toast(t('toast.coverFail')); }
  coverTarget = null;
};

/* ------------------------------------------------------------------ importar */
function askSystem(filename, choices) {
  return new Promise((resolve) => {
    const d = $('#picker');
    let chosen = null;
    d.replaceChildren(
      el('h2', { textContent: t('picker.title') }),
      el('div', { className: 'sub', textContent: filename }),
      el('div', { className: 'list' },
        ...choices.map((k) => sheetButton(SYSTEMS[k].name, () => { chosen = k; d.close(); })),
        sheetButton(t('picker.skip'), () => d.close()),
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
    if (total > 1) toast(t('import.progress', { i: i + 1, n: total }), 60000);
    const ext = extOf(f.name);
    if (SWITCH_UNSUPPORTED_EXT.includes(ext)) {
      toast(t('import.switchUnsupported'), 6000);
      skipped++;
      continue;
    }
    const det = detect(f.name);
    let system = det?.system;
    if (det?.choices) system = await askSystem(f.name, det.choices);
    if (!system) { if (!det) toast(t('import.badFormat', { ext })); skipped++; continue; }
    if (system === 'switch') { await runNroFile(f); showTab('switch'); continue; }
    if (games.some((g) => g.filename === f.name && g.size === f.size)) { dup++; continue; }
    try {
      const g = { id: newId(), title: titleOf(f.name), filename: f.name, system, size: f.size, added: Date.now(), blob: f };
      await putGame(g);
      games.push(g);
      added++;
    } catch (e) {
      toast(e?.name === 'QuotaExceededError' ? t('import.noSpace') : t('import.saveFail'), 5000);
      skipped++;
    }
  }
  const parts = [];
  if (added) parts.push(tn('import.added', added));
  if (dup) parts.push(tn('import.dup', dup));
  if (parts.length) toast(parts.join(' · '));
  else if (!skipped) toast(t('import.nothing'));
  await refresh();
}

$('#add').onclick = $('#empty-add').onclick = () => $('#file').click();
$('#file').onchange = (e) => { const f = [...e.target.files]; e.target.value = ''; importFiles(f); };
addEventListener('dragover', (e) => e.preventDefault());
addEventListener('drop', (e) => { e.preventDefault(); importFiles([...e.dataTransfer.files]); });
const renderFormats = () => { $('#formats').textContent = Object.values(SYSTEMS).filter((s) => s.ejs).map((s) => `${s.name}: ${s.ext.map((x) => '.' + x).join(' ')}`).join(' · ') + ' · ' + t('formats.extra'); };

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

/** Los «juegos» de MeloNX son atajos: Nexus no guarda el juego, solo abre MeloNX con su esquema melonx://. */
async function launchMeloNX(g) {
  g.lastPlayed = Date.now();
  await putGame(g);
  const q = g.titleId ? `id=${encodeURIComponent(g.titleId)}` : `name=${encodeURIComponent(g.title)}`;
  toast(t('melonx.opening'));
  location.href = `melonx://game?${q}`;
}

function addSwitchShortcut() {
  const title = prompt(t('melonx.promptName'));
  if (!title?.trim()) return;
  const titleId = (prompt(t('melonx.promptId')) || '').trim();
  putGame({ id: newId(), title: title.trim(), filename: '', system: 'melonx', size: 0, added: Date.now(), titleId: titleId || undefined }).then(refresh);
}
$('#add-shortcut').onclick = addSwitchShortcut;
$('#open-melonx').onclick = () => { location.href = 'melonx://'; };

function play(id) {
  const g = games.find((x) => x.id === id);
  if (g?.system === 'melonx') { launchMeloNX(g); return; }
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

/* ------------------------------------------------------------- modo consola (estilo SteamOS) */
const big = { open: false, list: [], i: 0, raf: 0, last: 0 };
const bigEl = () => $('#big');

function bigRender() {
  big.list = [...games].sort(byRecent);
  const track = $('#big-track');
  track.replaceChildren(...big.list.map((g, idx) => {
    const b = el('button', { className: 'card-big' + (idx === big.i ? ' sel' : ''), ariaLabel: g.title, onclick: () => (idx === big.i ? bigPlay() : bigSelect(idx)) }, coverEl(g));
    return b;
  }));
  if (!big.list.length) { $('#big-title').textContent = t('big.empty'); $('#big-sub').textContent = t('big.emptySub'); return; }
  bigSelect(Math.min(big.i, big.list.length - 1), false);
}

function bigSelect(i, scroll = true) {
  if (!big.list.length) return;
  big.i = (i + big.list.length) % big.list.length;
  const cards = [...$('#big-track').children];
  cards.forEach((c, k) => c.classList.toggle('sel', k === big.i));
  const g = big.list[big.i];
  $('#big-title').textContent = (g.favorite ? '★ ' : '') + g.title;
  $('#big-sub').textContent = SYSTEMS[g.system].name + (g.system === 'melonx' ? ' · ' + t('melonx.opens') : '');
  if (scroll) cards[big.i]?.scrollIntoView({ inline: 'center', block: 'nearest', behavior: 'smooth' });
}

function bigPlay() { const g = big.list[big.i]; if (g) play(g.id); }
async function bigFav() { const g = big.list[big.i]; if (!g) return; g.favorite = !g.favorite; await putGame(g); bigSelect(big.i, false); }

function openBig() {
  big.open = true; big.i = 0;
  bigEl().hidden = false;
  bigRender();
  const tick = () => { $('#big-clock').textContent = new Date().toLocaleTimeString(getLang(), { hour: '2-digit', minute: '2-digit' }); };
  tick(); big.clock = setInterval(tick, 20000);
  big.raf = requestAnimationFrame(bigPoll);
  navigator.vibrate?.(8);
}
function closeBig() { big.open = false; bigEl().hidden = true; clearInterval(big.clock); cancelAnimationFrame(big.raf); refresh(); }

/** Mando: cruceta/stick = mover, A = jugar, Y = favorito, B = salir. Con repetición al mantener. */
let padPrev = {};
function bigPoll(t) {
  if (!big.open) return;
  big.raf = requestAnimationFrame(bigPoll);
  if (playing) return; // el juego tiene el mando
  const pad = [...(navigator.getGamepads?.() || [])].find(Boolean);
  if (!pad) return;
  const b = (k) => !!pad.buttons[k]?.pressed;
  const ax = pad.axes[0] || 0;
  const dir = b(15) || ax > 0.6 ? 1 : b(14) || ax < -0.6 ? -1 : 0;
  if (dir && t - big.last > (padPrev.dir === dir ? 220 : 380)) { bigSelect(big.i + dir); big.last = t; }
  if (!dir) big.last = 0;
  if (b(0) && !padPrev.a) bigPlay();
  if (b(3) && !padPrev.y) bigFav();
  if (b(1) && !padPrev.b) closeBig();
  padPrev = { dir, a: b(0), y: b(3), b: b(1) };
}

addEventListener('keydown', (e) => {
  if (!big.open || playing) return;
  if (e.key === 'ArrowRight') bigSelect(big.i + 1);
  else if (e.key === 'ArrowLeft') bigSelect(big.i - 1);
  else if (e.key === 'Enter') bigPlay();
  else if (e.key.toLowerCase() === 'y') bigFav();
  else if (e.key === 'Escape') closeBig();
});
// al deslizar con el dedo, selecciona la carátula centrada
$('#big-track').addEventListener('scroll', () => {
  clearTimeout(big.st);
  big.st = setTimeout(() => {
    const t = $('#big-track'), mid = t.scrollLeft + t.clientWidth / 2;
    const idx = [...t.children].findIndex((c) => Math.abs(c.offsetLeft + c.offsetWidth / 2 - mid) < c.offsetWidth / 2);
    if (idx >= 0 && idx !== big.i) bigSelect(idx, false);
  }, 90);
}, { passive: true });
$('#big-open').onclick = openBig;
$('#big-close').onclick = closeBig;

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
const CHECKLIST = [0, 1, 2, 3, 4, 5, 6];

function renderChecklist() {
  const ul = $('#checklist');
  ul.replaceChildren(...CHECKLIST.map((text, i) => {
    const id = `chk${i}`;
    const box = el('input', { type: 'checkbox', id, checked: !!cfg.checklist[i], onchange: (e) => { cfg.checklist[i] = e.target.checked; saveCfg(); progress(); } });
    return el('li', {}, box, el('label', { htmlFor: id, textContent: t(`check.${text}`) }));
  }));
  const progress = () => { $('#checklist-progress').textContent = t('switch.progress', { a: CHECKLIST.filter((_, i) => cfg.checklist[i]).length, b: CHECKLIST.length }); };
  progress();
}

const con = $('#console');
const show = (t) => { con.hidden = false; con.textContent = t; };
async function runNroBytes(bytes, name) {
  show(t('lab.running', { name }));
  try {
    const { code, output, error } = await runNro(bytes);
    show(`${output}${error ? '\n' + t('lab.error', { msg: error }) : '\n' + t('lab.exited', { code })}`);
  } catch (e) { show(t('lab.error', { msg: e.message })); }
}
const runNroFile = async (f) => runNroBytes(new Uint8Array(await f.arrayBuffer()), f.name);
$('#run-demo').onclick = async () => runNroBytes(new Uint8Array(await (await fetch('emu/demo.nro')).arrayBuffer()), 'demo.nro');
$('#run-nro').onclick = () => $('#file-nro').click();
$('#file-nro').onchange = (e) => { if (e.target.files[0]) runNroFile(e.target.files[0]); e.target.value = ''; };

/* ------------------------------------------------------------------- ajustes */
function bindSettings() {
  const map = [['#set-theme', 'theme', 'value'], ['#set-lang', 'lang', 'value'], ['#set-sort', 'sort', 'value'], ['#set-group', 'group', 'checked'], ['#set-cover', 'snapshot', 'checked'], ['#set-wake', 'wake', 'checked'], ['#set-pause', 'pause', 'checked']];
  for (const [sel, key, prop] of map) {
    const e = $(sel);
    e[prop] = key === 'lang' ? getLang() : cfg[key];
    e.onchange = () => {
      if (key === 'lang') { setLang(e[prop]); retranslate(); return; }
      cfg[key] = e[prop]; saveCfg(); if (key === 'theme') applyTheme(); render();
    };
  }
}

async function updateStorage() {
  const total = games.reduce((s, g) => s + g.size, 0);
  let quota = '';
  try { const e = await navigator.storage.estimate(); quota = t('usage.quota', { a: mb(e.usage), b: mb(e.quota) }); } catch { /* sin API */ }
  $('#usage').textContent = tn('usage.games', games.length, { mb: mb(total) }) + quota;
  let persisted = false;
  try { persisted = await navigator.storage.persisted(); } catch { /* sin API */ }
  $('#persist-status').textContent = persisted ? t('persist.yes') : t('persist.no');
  $('#persist').hidden = persisted;
}

$('#persist').onclick = async () => {
  let ok = false;
  try { ok = await navigator.storage.persist(); } catch { /* sin API */ }
  toast(ok ? t('toast.persistOk') : t('toast.persistNo'), 5000);
  updateStorage();
};
$('#clear-cache').onclick = async () => {
  try { for (const k of await caches.keys()) await caches.delete(k); toast(t('toast.cacheOk')); } catch { toast(t('toast.cacheFail')); }
};
$('#clear').onclick = async () => {
  if (!confirm(t('confirm.clear'))) return;
  for (const g of games) await deleteGame(g.id);
  await refresh();
  updateStorage();
};

/* mandos Bluetooth */
const pads = new Set();
function padStatus() {
  $('#pad-status').textContent = pads.size ? t('pad.connected', { names: [...pads].join(', ') }) : t('pad.none');
}
addEventListener('gamepadconnected', (e) => { pads.add(e.gamepad.id.split('(')[0].trim()); padStatus(); toast(t('toast.padConnected')); });
addEventListener('gamepaddisconnected', (e) => { pads.delete(e.gamepad.id.split('(')[0].trim()); padStatus(); });

/* ------------------------------------------------------- guía de inicio y aviso */
function openHelp() {
  const d = $('#help');
  d.replaceChildren(
    el('h2', { textContent: t('help.title') }),
    el('div', { className: 'sub', textContent: t('help.sub') }),
    el('ol', { className: 'steps' },
      ...[1, 2, 3, 4].map((n) => el('li', {}, el('b', { textContent: t(`help.${n}.b`) }), t(`help.${n}.t`))),
    ),
    el('p', { className: 'muted', textContent: t('help.switch') }),
    el('div', { className: 'list' }, sheetButton(t('help.ok'), () => { cfg.onboarded = true; saveCfg(); d.close(); }, 'primary')),
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

/* -------------------------------------------------------------------- idioma */
/** Vuelve a pintar todo el texto al cambiar de idioma. */
function retranslate() {
  applyStatic();
  renderFormats();
  renderChecklist();
  padStatus();
  showTab(currentTab);
  if (big.open) bigRender();
  if ($('#help').open) openHelp();
  render();
  updateStorage();
}

/* -------------------------------------------------------------------- arranque */
applyStatic();
renderFormats();
applyTheme();
matchMedia('(prefers-color-scheme: light)').addEventListener?.('change', applyTheme);
bindSettings();
renderChecklist();
padStatus();
showTab('library');
maybeInstallBanner();
refresh().then(() => { if (!cfg.onboarded && games.length === 0) setTimeout(openHelp, 400); });
if ('serviceWorker' in navigator) navigator.serviceWorker.register('sw.js').catch(() => {});
try { navigator.storage?.persist?.(); } catch { /* opcional */ }
