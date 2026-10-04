// Biblioteca de juegos en IndexedDB (todo queda en el dispositivo; nada se sube a ningún servidor).
const DB = 'nexus', STORE = 'games';

function open() {
  return new Promise((res, rej) => {
    const r = indexedDB.open(DB, 1);
    r.onupgradeneeded = () => r.result.createObjectStore(STORE, { keyPath: 'id' });
    r.onsuccess = () => res(r.result);
    r.onerror = () => rej(r.error);
  });
}

async function tx(mode, fn) {
  const db = await open();
  return new Promise((res, rej) => {
    const t = db.transaction(STORE, mode);
    const out = fn(t.objectStore(STORE));
    t.oncomplete = () => { db.close(); res(out.result); };
    t.onerror = t.onabort = () => { db.close(); rej(t.error); };
  });
}

export const listGames = () => tx('readonly', (s) => s.getAll());
export const getGame = (id) => tx('readonly', (s) => s.get(id));
export const putGame = (g) => tx('readwrite', (s) => s.put(g));
export const deleteGame = (id) => tx('readwrite', (s) => s.delete(id));
export const newId = () => crypto.randomUUID();
