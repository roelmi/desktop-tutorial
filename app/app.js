import { listGames, getGame, putGame, deleteGame, newId } from './db.js';
import { SYSTEMS, SWITCH_REMOTE_EXT, detect, extOf, titleOf } from './systems.js';
import { runNro } from './nx.js';

const $ = (s) => document.querySelector(s);
const el = (tag, props = {}, ...kids) => {
  const e = Object.assign(document.createElement(tag), props);
  e.append(...kids);
  return e;
};

let games = [];
let filter = 'all';

function toast(msg, ms = 3200) {
  const t = $('#toast');
  t.textContent = msg;
  t.classList.add('show');
  clearTimeout(toast.t);
  toast.t = setTimeout(() => t.classList.remove('show'), ms);
}

/* ---------- Pestañas ---------- */
const TITLES = { library: 'Biblioteca', switch: 'Switch', settings: 'Ajustes' };
function showTab(name) {
  document.querySelectorAll('.tab').forEach((t) => t.classList.toggle('active', t.id === `tab-${name}`));
  document.querySelectorAll('.tabs button').forEach((b) => b.classList.toggle('active', b.dataset.tab === name));
  $('#title').textContent = TITLES[name];
  $('#add').hidden = name !== 'library';
  if (name === 'settings') updateUsage();
}
document.querySelectorAll('.tabs button').forEach((b) => b.addEventListener('click', () => showTab(b.dataset.tab)));

/* ---------- Biblioteca ---------- */
async function refresh() {
  games = await listGames();
  render();
}

function render() {
  const q = $('#search').value.trim().toLowerCase();
  const used = [...new Set(games.map((g) => g.system))];
  const chips = $('#chips');
  chips.replaceChildren(
    ...['all', ...used].map((k) =>
      el('button', {
        className: 'chip' + (filter === k ? ' active' : ''),
        textContent: k === 'all' ? 'Todos' : SYSTEMS[k].name,
        onclick: () => { filter = k; render(); },
      }),
    ),
  );
  if (filter !== 'all' && !used.includes(filter)) filter = 'all';

  const list = games
    .filter((g) => (filter === 'all' || g.system === filter) && g.title.toLowerCase().includes(q))
    .sort((a, b) => (b.lastPlayed || 0) - (a.lastPlayed || 0) || b.added - a.added);
  $('#empty').hidden = games.length > 0;
  $('#grid').replaceChildren(...list.map(card));
}

function card(g) {
  const sys = SYSTEMS[g.system];
  const initials = g.title.replace(/[^\p{L}\p{N} ]/gu, '').split(/\s+/).slice(0, 2).map((w) => w[0] || '').join('').toUpperCase();
  const cover = el('div', { className: 'cover', textContent: initials || '?' });
  cover.style.background = `linear-gradient(135deg, ${sys.color}, ${sys.color}99)`;
  const open = el('button', { className: 'game', onclick: () => play(g.id) }, cover, el('b', { textContent: g.title }), el('small', { textContent: sys.name }));
  const more = el('button', { className: 'more', textContent: '⋯', ariaLabel: 'Opciones', onclick: (e) => { e.stopPropagation(); gameMenu(g); } });
  return el('div', { style: 'position:relative;min-width:0' }, open, more);
}

function gameMenu(g) {
  const d = $('#menu');
  d.replaceChildren(
    el('h2', { textContent: g.title }),
    el('button', { className: 'btn', textContent: 'Renombrar', onclick: async () => {
      const t = prompt('Nuevo nombre', g.title);
      if (t?.trim()) { g.title = t.trim(); await putGame(g); await refresh(); }
      d.close();
    } }),
    el('button', { className: 'btn danger', textContent: 'Eliminar (y sus partidas guardadas)', onclick: async () => {
      if (confirm(`¿Eliminar "${g.title}"?`)) { await deleteGame(g.id); await refresh(); }
      d.close();
    } }),
    el('button', { className: 'btn', textContent: 'Cerrar', onclick: () => d.close() }),
  );
  d.showModal();
}

/* ---------- Importar ---------- */
function askSystem(filename) {
  return new Promise((resolve) => {
    const d = $('#menu');
    const sel = el('select', {}, ...Object.entries(SYSTEMS).filter(([, s]) => s.ejs).map(([k, s]) => el('option', { value: k, textContent: s.name })));
    d.replaceChildren(
      el('h2', { textContent: 'Elige la consola' }),
      el('p', { textContent: filename }),
      sel,
      el('button', { className: 'btn primary', textContent: 'Importar', onclick: () => { d.close(); resolve(sel.value); } }),
      el('button', { className: 'btn', textContent: 'Omitir', onclick: () => { d.close(); resolve(null); } }),
    );
    d.addEventListener('close', () => resolve(null), { once: true });
    d.showModal();
  });
}

async function importFiles(files) {
  let added = 0;
  for (const f of files) {
    const ext = extOf(f.name);
    if (SWITCH_REMOTE_EXT.includes(ext)) {
      toast('Los juegos de Switch (XCI/NSP) no se pueden ejecutar en este móvil. Mira la pestaña Switch.', 6000);
      continue;
    }
    let system = detect(f.name);
    if (!system && ['zip', '7z'].includes(ext)) system = await askSystem(f.name);
    if (!system) { if (!['zip', '7z'].includes(ext)) toast(`Formato no soportado: .${ext}`); continue; }
    if (system === 'switch') { await runNroFile(f); showTab('switch'); continue; }
    await putGame({ id: newId(), title: titleOf(f.name), filename: f.name, system, size: f.size, added: Date.now(), blob: f });
    added++;
  }
  if (added) toast(`${added} juego${added > 1 ? 's' : ''} añadido${added > 1 ? 's' : ''}`);
  await refresh();
}

$('#add').onclick = () => $('#file').click();
$('#file').onchange = (e) => { importFiles([...e.target.files]); e.target.value = ''; };
addEventListener('dragover', (e) => e.preventDefault());
addEventListener('drop', (e) => { e.preventDefault(); importFiles([...e.dataTransfer.files]); });
$('#search').oninput = render;

/* ---------- Jugar ---------- */
function play(id) {
  $('#frame').src = `play.html?id=${encodeURIComponent(id)}`;
  $('#player').hidden = false;
  document.documentElement.style.overflow = 'hidden';
}
function exitGame() {
  $('#frame').src = 'about:blank';   // descarga el núcleo y detiene el audio
  $('#player').hidden = true;
  document.documentElement.style.overflow = '';
  refresh();
}
$('#exit').onclick = exitGame;

/* ---------- Switch ---------- */
const con = $('#console');
function show(text) { con.hidden = false; con.textContent = text; }

async function runNroBytes(bytes, name) {
  show(`Ejecutando ${name}…`);
  try {
    const { code, output, error } = await runNro(bytes);
    show(`${output}${error ? `\n[error] ${error}` : `\n[proceso terminó con código ${code}]`}`);
  } catch (e) {
    show(`[error] ${e.message}`);
  }
}
const runNroFile = async (f) => runNroBytes(new Uint8Array(await f.arrayBuffer()), f.name);

$('#run-demo').onclick = async () => runNroBytes(new Uint8Array(await (await fetch('emu/demo.nro')).arrayBuffer()), 'demo.nro');
$('#run-nro').onclick = () => $('#file-nro').click();
$('#file-nro').onchange = (e) => { if (e.target.files[0]) runNroFile(e.target.files[0]); e.target.value = ''; };

/* ---------- Ajustes ---------- */
async function updateUsage() {
  const mb = (n) => (n / 1048576).toFixed(1);
  const total = games.reduce((s, g) => s + g.size, 0);
  let extra = '';
  try { const e = await navigator.storage.estimate(); extra = ` · cuota del navegador: ${mb(e.quota)} MB`; } catch { /* sin API */ }
  $('#usage').textContent = `${games.length} juegos · ${mb(total)} MB${extra}`;
}
$('#clear').onclick = async () => {
  if (!confirm('¿Borrar TODOS los juegos de la biblioteca?')) return;
  for (const g of games) await deleteGame(g.id);
  await refresh(); updateUsage();
};

/* ---------- Arranque ---------- */
showTab('library');
refresh();
if ('serviceWorker' in navigator) navigator.serviceWorker.register('sw.js').catch(() => {});
try { navigator.storage?.persist?.(); } catch { /* opcional */ }
export { getGame }; // usado por pruebas
