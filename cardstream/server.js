import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";
import { openStore } from "./store.js";
import { startTwitch } from "./adapters/twitch.js";
import { startYouTube } from "./adapters/youtube.js";
import { watchTwitchGame } from "./adapters/twitchgame.js";
import { fallbackCards, llmCards, svgArt, aiImage } from "./generator.js";

const dir = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.PORT || 3000);
const DATA_DIR = process.env.DATA_DIR || path.join(dir, "data");
const UPLOADS = path.join(DATA_DIR, "uploads");
fs.mkdirSync(UPLOADS, { recursive: true });

const ADMIN_PASSWORD = process.env.DASH_PASSWORD || crypto.randomBytes(6).toString("hex");
if (!process.env.DASH_PASSWORD) console.log(`Dashboard password (set DASH_PASSWORD to choose your own): ${ADMIN_PASSWORD}`);

const RARITIES = ["common", "rare", "epic", "legendary"];
const WEIGHTS = { common: 60, rare: 28, epic: 9, legendary: 3 };
const EFFECTS = ["alert", "confetti", "emojirain", "banner", "challenge", "wanted"];

const store = openStore(path.join(DATA_DIR, "db.json"), {
  settings: {
    streamerName: "My Stream", autoAccept: false, activeCollection: "",
    packCost: 50, packSize: 3, guestCoins: 0, startCoins: 50,
    chatCoins: 2, chatCooldownSec: 60, dailyCoins: 25,
    playCooldownSec: 3, goalTarget: 20, goalReward: "rare",
    currentGame: "", autoGame: false, artMode: "local", cardsPerGame: 10,
  },
  cards: JSON.parse(fs.readFileSync(path.join(dir, "cards.json"), "utf8")),
  users: {}, sessions: {}, queue: [], stats: { opened: 0, played: 0 },
  goal: { progress: 0, contributors: [], completed: 0 },
});
const db = store.data, S = db.settings, save = store.save;

const id = () => crypto.randomBytes(6).toString("hex");
const now = () => Date.now();
const clean = (s, max) => String(s ?? "").replace(/[\u0000-\u001f<>]/g, "").trim().slice(0, max);
const num = (v, lo, hi, d) => { const n = Number(v); return Number.isFinite(n) ? Math.min(hi, Math.max(lo, Math.round(n))) : d; };
const cleanUrl = (u) => { u = String(u || "").trim(); return u === "" || /^\/uploads\/[\w.-]+$/.test(u) || /^https:\/\/[^\s"'<>]+$/.test(u) ? u : null; };
const safeEq = (a, b) => { const x = Buffer.from(String(a)), y = Buffer.from(String(b)); return x.length === y.length && crypto.timingSafeEqual(x, y); };

// ---------- live events (overlay) ----------
const overlays = new Set();
function broadcast(ev) { for (const r of overlays) r.write(`data: ${JSON.stringify(ev)}\n\n`); }
setInterval(() => { for (const r of overlays) r.write(": ping\n\n"); }, 25000).unref();

// ---------- users ----------
function getUser(key, name, platform, coins) {
  let u = db.users[key];
  if (!u) { u = db.users[key] = { key, name, platform, coins, inventory: {}, lastChatAt: 0, lastDaily: 0, lastPlayAt: 0, stats: { opened: 0, played: 0 } }; save(); }
  return u;
}
const chatKey = (platform, viewer) => `${platform}:${viewer.toLowerCase()}`;
const cardById = (cid) => db.cards.find((c) => c.id === cid);
const invCount = (u) => Object.values(u.inventory).reduce((a, b) => a + b, 0);
function publicUser(u, s) {
  const inv = {}; for (const [k, v] of Object.entries(u.inventory)) if (v > 0 && cardById(k)) inv[k] = v;
  return { name: u.name, platform: u.platform, linked: !u.key.startsWith("web:"), coins: u.coins, inventory: inv, stats: u.stats,
    code: s?.code || null, dailyReady: now() - u.lastDaily >= 86400000, nextDailyAt: u.lastDaily + 86400000 };
}

// ---------- game rules ----------
function roll() {
  const pool = db.cards.filter((c) => !S.activeCollection || c.collection === S.activeCollection);
  const present = RARITIES.filter((r) => pool.some((c) => c.rarity === r));
  if (!present.length) return null;
  let t = Math.random() * present.reduce((a, r) => a + WEIGHTS[r], 0), rar = present[0];
  for (const r of present) { t -= WEIGHTS[r]; if (t <= 0) { rar = r; break; } }
  const of = pool.filter((c) => c.rarity === rar);
  return of[Math.floor(Math.random() * of.length)];
}
function give(u, cid, n = 1) { u.inventory[cid] = (u.inventory[cid] || 0) + n; }

function openPack(u) {
  if (u.coins < S.packCost) return { ok: false, error: `You need ${S.packCost} coins` };
  const pulled = []; for (let i = 0; i < S.packSize; i++) { const c = roll(); if (c) pulled.push(c); }
  if (!pulled.length) return { ok: false, error: "No cards available" };
  u.coins -= S.packCost; pulled.forEach((c) => give(u, c.id)); u.stats.opened++; db.stats.opened++;
  if (!db.goal.contributors.includes(u.key)) db.goal.contributors.push(u.key);
  db.goal.progress++;
  if (db.goal.progress >= S.goalTarget) completeGoal();
  save();
  return { ok: true, cards: pulled, coins: u.coins };
}
function completeGoal() {
  const pool = db.cards.filter((c) => c.rarity === S.goalReward);
  const reward = pool.length ? pool[Math.floor(Math.random() * pool.length)] : roll();
  if (reward) for (const k of db.goal.contributors) if (db.users[k]) give(db.users[k], reward.id);
  broadcast({ type: "goal", reward, count: db.goal.contributors.length });
  db.goal = { progress: 0, contributors: [], completed: db.goal.completed + 1 };
}

const lastCardPlay = new Map();
function playCard(u, cid) {
  const card = cardById(cid);
  if (!card) return { ok: false, error: "Unknown card" };
  if (!(u.inventory[cid] > 0)) return { ok: false, error: "You don't own that card" };
  if (now() - u.lastPlayAt < S.playCooldownSec * 1000) return { ok: false, error: "Slow down a few seconds" };
  const cd = (card.cooldownSec || 0) * 1000, left = (lastCardPlay.get(cid) || 0) + cd - now();
  if (left > 0) return { ok: false, error: `This card is cooling down (${Math.ceil(left / 1000)}s)` };
  if (db.queue.filter((q) => q.status === "pending").length >= 50) return { ok: false, error: "Queue is full, try later" };
  u.inventory[cid]--; u.lastPlayAt = now(); u.stats.played++; db.stats.played++;
  const item = { id: id(), cardId: cid, userKey: u.key, viewer: u.name, platform: u.platform, at: now(), status: "pending" };
  db.queue.push(item); if (db.queue.length > 300) db.queue.splice(0, db.queue.length - 300);
  if (S.autoAccept) accept(item);
  save();
  return { ok: true, queued: !S.autoAccept };
}
function accept(item) {
  const card = cardById(item.cardId); if (!card) return;
  item.status = "accepted"; lastCardPlay.set(card.id, now());
  broadcast({ type: "play", card, viewer: item.viewer, platform: item.platform });
}
function reject(item) {
  item.status = "rejected"; const u = db.users[item.userKey]; if (u) give(u, item.cardId);
}

// ---------- chat ----------
function mergeInto(target, guest) {
  target.coins += guest.coins;
  for (const [k, v] of Object.entries(guest.inventory)) give(target, k, v);
  target.stats.opened += guest.stats.opened; target.stats.played += guest.stats.played;
  delete db.users[guest.key];
}
function onChat({ platform, viewer, text }) {
  viewer = clean(viewer, 40); if (!viewer) return;
  const key = chatKey(platform, viewer);
  const u = getUser(key, viewer, platform, S.startCoins);
  if (now() - u.lastChatAt >= S.chatCooldownSec * 1000) { u.lastChatAt = now(); u.coins += S.chatCoins; save(); }
  const m = /^!(\w+)\s*(\w*)/.exec(text || ""); if (!m) return;
  const [, cmd, arg] = m;
  if (cmd === "link" && arg) {
    const entry = Object.entries(db.sessions).find(([, s]) => s.code && s.code === arg.toUpperCase() && db.users[s.key]?.key.startsWith("web:"));
    if (entry) { const [, s] = entry; mergeInto(u, db.users[s.key]); s.key = key; s.code = null; save(); console.log(`[link] ${key}`); }
  } else if (cmd === "play" && arg) console.log(`[${platform}] ${viewer} play ${arg}:`, JSON.stringify(playCard(u, arg.toLowerCase())));
  else if (cmd === "open") console.log(`[${platform}] ${viewer} open:`, JSON.stringify(openPack(u)).slice(0, 80));
}

// ---------- per-game card sets ----------
const slug = (g) => g.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 20) || "game";
let job = { status: "idle", game: "", done: 0, total: 0, errors: [], note: "" };

function switchGame(game, { regenerate = false } = {}) {
  game = clean(game, 40); if (!game) return { ok: false, error: "Enter a game name" };
  if (job.status === "running") return { ok: false, error: `Still generating cards for ${job.game}` };
  const existing = db.cards.find((c) => c.collection.toLowerCase() === game.toLowerCase());
  if (existing && !regenerate) {
    S.activeCollection = existing.collection; S.currentGame = existing.collection; save();
    return { ok: true, generated: false, collection: existing.collection };
  }
  generateSet(game, regenerate).catch((e) => { job = { ...job, status: "error", note: e.message }; });
  return { ok: true, generated: true };
}

async function generateSet(game, regenerate) {
  const n = S.cardsPerGame, mode = S.artMode === "ai" && process.env.OPENAI_API_KEY ? "ai" : "local";
  job = { status: "running", game, done: 0, total: n, errors: [], note: "Writing cards…" };
  let raw, source = "built-in ideas";
  if (process.env.ANTHROPIC_API_KEY) { try { raw = await llmCards(game, n, process.env.ANTHROPIC_API_KEY); source = "Claude"; } catch (e) { job.errors.push("Card writing fell back to built-in ideas: " + e.message); } }
  raw ||= fallbackCards(game, n);
  const collection = existing_collection_name(game), prefix = slug(game) + "-";
  if (regenerate) db.cards = db.cards.filter((c) => c.collection !== collection);
  const made = [];
  for (let i = 0; i < raw.length; i++) {
    job.note = `Drawing card ${i + 1} of ${raw.length} (${mode === "ai" ? "AI images" : "local art"}, text from ${source})`;
    const card = cardFromBody({ ...raw[i], id: prefix + (i + 1), collection }, null);
    card.id = prefix + (i + 1);
    try {
      if (mode === "ai") { const buf = await aiImage({ ...card, art: clean(raw[i].art, 160) }, game, process.env.OPENAI_API_KEY); fs.writeFileSync(path.join(UPLOADS, card.id + ".png"), buf); card.image = `/uploads/${card.id}.png`; }
      else { fs.writeFileSync(path.join(UPLOADS, card.id + ".svg"), svgArt(card, game)); card.image = `/uploads/${card.id}.svg`; }
    } catch (e) {
      job.errors.push(`${card.name}: AI image failed (${e.message}); used local art`);
      fs.writeFileSync(path.join(UPLOADS, card.id + ".svg"), svgArt(card, game)); card.image = `/uploads/${card.id}.svg`;
    }
    const at = db.cards.findIndex((c) => c.id === card.id); if (at >= 0) db.cards[at] = card; else db.cards.push(card);
    made.push(card); job.done = i + 1;
  }
  S.activeCollection = collection; S.currentGame = collection; save();
  job = { ...job, status: "done", note: `Created ${made.length} cards for ${collection} and switched packs to it.` };
}
const existing_collection_name = (game) => db.cards.find((c) => c.collection.toLowerCase() === game.toLowerCase())?.collection || game;

// ---------- http ----------
const adminTokens = new Set(); const loginTries = new Map();
const types = { ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css", ".json": "application/json",
  ".png": "image/png", ".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".gif": "image/gif", ".webp": "image/webp",
  ".svg": "image/svg+xml", ".mp3": "audio/mpeg", ".ogg": "audio/ogg", ".wav": "audio/wav", ".mp4": "video/mp4", ".webm": "video/webm" };
const UPLOAD_EXT = new Set([".png", ".jpg", ".jpeg", ".gif", ".webp", ".mp3", ".ogg", ".wav", ".mp4", ".webm"]);
const json = (res, code, obj) => { res.writeHead(code, { "content-type": "application/json" }); res.end(JSON.stringify(obj)); };
const readBody = (req, limit = 8192) => new Promise((resolve, reject) => {
  let b = "", n = 0; req.on("data", (d) => { n += d.length; if (n > limit) { reject(new Error("too large")); req.destroy(); } else b += d; });
  req.on("end", () => { try { resolve(b ? JSON.parse(b) : {}); } catch { reject(new Error("bad json")); } });
  req.on("error", reject);
});
const sess = (req) => { const t = req.headers["x-token"], s = db.sessions[t]; const u = s && db.users[s.key]; return u ? { t, s, u } : null; };
const isAdmin = (req) => adminTokens.has(req.headers["x-admin"]);
const newCode = () => crypto.randomBytes(3).toString("hex").toUpperCase();

function findUserByName(name) {
  const n = String(name || "").toLowerCase();
  const all = Object.values(db.users).filter((u) => u.name.toLowerCase() === n);
  return all.find((u) => !u.key.startsWith("web:")) || all[0];
}

function cardFromBody(b, existing) {
  const out = { ...(existing || {}) };
  out.id = existing?.id || clean(b.id, 24).toLowerCase().replace(/[^a-z0-9]/g, "") || id();
  out.name = clean(b.name, 40) || "Untitled"; out.collection = clean(b.collection, 40) || "Classics";
  out.emoji = clean(b.emoji, 8) || "🃏"; out.text = clean(b.text, 140);
  out.rarity = RARITIES.includes(b.rarity) ? b.rarity : "common"; out.effect = EFFECTS.includes(b.effect) ? b.effect : "alert";
  out.seconds = num(b.seconds, 2, 300, 6); out.cooldownSec = num(b.cooldownSec, 0, 3600, 0); out.tts = !!b.tts;
  for (const f of ["image", "sound", "video"]) { const u = cleanUrl(b[f]); out[f] = u === null ? (existing?.[f] || "") : u; }
  return out;
}

async function api(req, res, url) {
  const p = url.pathname, method = req.method;
  // --- viewer ---
  if (p === "/api/session" && method === "POST") {
    const b = await readBody(req); let x = sess(req);
    if (!x) {
      const t = crypto.randomBytes(16).toString("hex"), key = "web:" + id();
      const name = clean(b.name, 24) || "Guest" + Math.floor(1000 + Math.random() * 9000);
      getUser(key, name, "web", S.guestCoins); db.sessions[t] = { key, code: newCode() }; save(); x = sess({ headers: { "x-token": t } });
    } else if (b.name && x.u.key.startsWith("web:")) { x.u.name = clean(b.name, 24) || x.u.name; save(); }
    if (!x.s.code && x.u.key.startsWith("web:")) x.s.code = newCode();
    return json(res, 200, { token: x.t, user: publicUser(x.u, x.s) });
  }
  if (p === "/api/me") { const x = sess(req); return x ? json(res, 200, publicUser(x.u, x.s)) : json(res, 401, { ok: false, error: "No session" }); }
  if (p === "/api/public") return json(res, 200, { streamerName: S.streamerName, cards: db.cards, packCost: S.packCost, packSize: S.packSize,
    goal: { progress: db.goal.progress, target: S.goalTarget, reward: S.goalReward, completed: db.goal.completed }, collections: [...new Set(db.cards.map((c) => c.collection))], activeCollection: S.activeCollection, currentGame: S.currentGame });
  if (p === "/api/open-pack" && method === "POST") { const x = sess(req); if (!x) return json(res, 401, { ok: false, error: "No session" }); const r = openPack(x.u); return json(res, r.ok ? 200 : 400, r); }
  if (p === "/api/play" && method === "POST") { const x = sess(req); if (!x) return json(res, 401, { ok: false, error: "No session" }); const b = await readBody(req); const r = playCard(x.u, String(b.cardId || "")); return json(res, r.ok ? 200 : 400, r); }
  if (p === "/api/daily" && method === "POST") {
    const x = sess(req); if (!x) return json(res, 401, { ok: false, error: "No session" });
    if (x.u.key.startsWith("web:")) return json(res, 400, { ok: false, error: "Link your chat account first" });
    if (now() - x.u.lastDaily < 86400000) return json(res, 400, { ok: false, error: "Already claimed today" });
    x.u.lastDaily = now(); x.u.coins += S.dailyCoins; save(); return json(res, 200, { ok: true, coins: x.u.coins });
  }
  if (p === "/api/gift" && method === "POST") {
    const x = sess(req); if (!x) return json(res, 401, { ok: false, error: "No session" }); const b = await readBody(req);
    const to = findUserByName(b.to), cid = String(b.cardId || "");
    if (!to || to.key === x.u.key) return json(res, 400, { ok: false, error: "Viewer not found" });
    if (!(x.u.inventory[cid] > 0)) return json(res, 400, { ok: false, error: "You don't own that card" });
    x.u.inventory[cid]--; give(to, cid); save(); return json(res, 200, { ok: true });
  }
  // --- admin ---
  if (p === "/api/admin/login" && method === "POST") {
    const ip = req.socket.remoteAddress, rec = loginTries.get(ip) || { n: 0, t: now() };
    if (now() - rec.t > 60000) { rec.n = 0; rec.t = now(); }
    if (rec.n >= 5) return json(res, 429, { ok: false, error: "Too many attempts, wait a minute" });
    const b = await readBody(req); rec.n++; loginTries.set(ip, rec);
    if (!safeEq(b.password || "", ADMIN_PASSWORD)) return json(res, 401, { ok: false, error: "Wrong password" });
    const t = crypto.randomBytes(24).toString("hex"); adminTokens.add(t); return json(res, 200, { ok: true, token: t });
  }
  if (p.startsWith("/api/admin/")) {
    if (!isAdmin(req)) return json(res, 401, { ok: false, error: "Not logged in" });
    if (p === "/api/admin/state") return json(res, 200, { settings: S, cards: db.cards, stats: db.stats, job, providers: { llm: !!process.env.ANTHROPIC_API_KEY, ai: !!process.env.OPENAI_API_KEY, twitchGame: !!(process.env.TWITCH_CLIENT_ID && process.env.TWITCH_CLIENT_SECRET && process.env.TWITCH_CHANNEL) }, goal: { progress: db.goal.progress, target: S.goalTarget, completed: db.goal.completed },
      queue: db.queue.slice(-100).reverse().map((q) => ({ ...q, card: cardById(q.cardId) })),
      users: Object.values(db.users).sort((a, b) => b.coins - a.coins).slice(0, 100).map((u) => ({ name: u.name, platform: u.platform, coins: u.coins, cards: invCount(u), opened: u.stats.opened, played: u.stats.played })) });
    const qm = /^\/api\/admin\/queue\/(\w+)\/(accept|reject|replay)$/.exec(p);
    if (qm && method === "POST") {
      const item = db.queue.find((q) => q.id === qm[1]); if (!item) return json(res, 404, { ok: false, error: "Not found" });
      if (qm[2] === "accept" && item.status === "pending") accept(item);
      else if (qm[2] === "reject" && item.status === "pending") reject(item);
      else if (qm[2] === "replay" && item.status === "accepted") accept(item);
      else return json(res, 400, { ok: false, error: "Not allowed for this status" });
      save(); return json(res, 200, { ok: true });
    }
    if (p === "/api/admin/accept-all" && method === "POST") { db.queue.filter((q) => q.status === "pending").forEach(accept); save(); return json(res, 200, { ok: true }); }
    if (p === "/api/admin/card" && method === "POST") {
      const b = await readBody(req, 16384), ex = cardById(clean(b.id, 24).toLowerCase());
      const card = cardFromBody(b, ex); if (ex) Object.assign(ex, card); else db.cards.push(card); save(); return json(res, 200, { ok: true, card });
    }
    if (p === "/api/admin/card/delete" && method === "POST") { const b = await readBody(req); db.cards = db.cards.filter((c) => c.id !== b.id); save(); return json(res, 200, { ok: true }); }
    if (p === "/api/admin/settings" && method === "POST") {
      const b = await readBody(req, 16384);
      S.streamerName = clean(b.streamerName, 40) || S.streamerName; S.autoAccept = !!b.autoAccept; S.activeCollection = clean(b.activeCollection, 40);
      S.packCost = num(b.packCost, 1, 100000, S.packCost); S.packSize = num(b.packSize, 1, 10, S.packSize); S.guestCoins = num(b.guestCoins, 0, 100000, S.guestCoins);
      S.startCoins = num(b.startCoins, 0, 100000, S.startCoins); S.chatCoins = num(b.chatCoins, 0, 1000, S.chatCoins); S.chatCooldownSec = num(b.chatCooldownSec, 5, 3600, S.chatCooldownSec);
      S.dailyCoins = num(b.dailyCoins, 0, 100000, S.dailyCoins); S.playCooldownSec = num(b.playCooldownSec, 0, 600, S.playCooldownSec);
      S.goalTarget = num(b.goalTarget, 1, 100000, S.goalTarget); S.goalReward = RARITIES.includes(b.goalReward) ? b.goalReward : S.goalReward;
      S.autoGame = !!b.autoGame; S.artMode = b.artMode === "ai" ? "ai" : "local"; S.cardsPerGame = num(b.cardsPerGame, 4, 16, S.cardsPerGame);
      save(); return json(res, 200, { ok: true });
    }
    if (p === "/api/admin/grant" && method === "POST") {
      const b = await readBody(req), u = findUserByName(b.name); if (!u) return json(res, 404, { ok: false, error: "Viewer not found" });
      if (b.coins) u.coins = Math.max(0, u.coins + num(b.coins, -1e6, 1e6, 0));
      if (b.cardId && cardById(b.cardId)) give(u, b.cardId, num(b.count, 1, 100, 1)); save(); return json(res, 200, { ok: true });
    }
    if (p === "/api/admin/test" && method === "POST") { const b = await readBody(req), c = cardById(b.cardId); if (!c) return json(res, 404, { ok: false }); broadcast({ type: "play", card: c, viewer: "Test", platform: "web" }); return json(res, 200, { ok: true }); }
    if (p === "/api/admin/sim-chat" && method === "POST") { const b = await readBody(req); onChat({ platform: ["twitch", "youtube"].includes(b.platform) ? b.platform : "twitch", viewer: b.viewer, text: String(b.text || "") }); return json(res, 200, { ok: true }); }
    if (p === "/api/admin/game" && method === "POST") {
      const b = await readBody(req), r = switchGame(clean(b.game, 40), { regenerate: !!b.regenerate });
      return json(res, r.ok ? 200 : 400, r);
    }
    if (p === "/api/admin/upload" && method === "POST") {
      const b = await readBody(req, 8 * 1024 * 1024), ext = path.extname(String(b.filename || "")).toLowerCase();
      if (!UPLOAD_EXT.has(ext)) return json(res, 400, { ok: false, error: "File type not allowed" });
      const buf = Buffer.from(String(b.data || ""), "base64"); if (!buf.length || buf.length > 5 * 1024 * 1024) return json(res, 400, { ok: false, error: "File must be under 5 MB" });
      const name = id() + ext; fs.writeFileSync(path.join(UPLOADS, name), buf); return json(res, 200, { ok: true, url: "/uploads/" + name });
    }
    return json(res, 404, { ok: false, error: "Unknown route" });
  }
  return json(res, 404, { ok: false, error: "Unknown route" });
}

http.createServer(async (req, res) => {
  const url = new URL(req.url, "http://x");
  try {
    if (url.pathname === "/events") {
      res.writeHead(200, { "content-type": "text/event-stream", "cache-control": "no-cache", connection: "keep-alive" });
      res.write(": connected\n\n"); overlays.add(res); req.on("close", () => overlays.delete(res)); return;
    }
    if (url.pathname.startsWith("/api/")) return await api(req, res, url);
    let full;
    if (url.pathname.startsWith("/uploads/")) full = path.join(UPLOADS, path.basename(url.pathname));
    else {
      const f = { "/": "index.html", "/overlay": "overlay.html", "/dashboard": "dashboard.html" }[url.pathname] || path.basename(url.pathname);
      full = path.join(dir, "public", f);
    }
    if (!fs.existsSync(full) || fs.statSync(full).isDirectory()) { res.writeHead(404); return res.end("Not found"); }
    const h = { "content-type": types[path.extname(full)] || "application/octet-stream", "x-content-type-options": "nosniff" };
    if (url.pathname.startsWith("/uploads/")) h["content-security-policy"] = "default-src 'none'; img-src 'self' data:; style-src 'unsafe-inline'; sandbox";
    res.writeHead(200, h);
    fs.createReadStream(full).pipe(res);
  } catch (e) { if (!res.headersSent) json(res, 400, { ok: false, error: e.message === "too large" ? "Request too large" : "Bad request" }); }
}).listen(PORT, () => console.log(`CardStream on http://localhost:${PORT}   viewers: /   dashboard: /dashboard   OBS overlay: /overlay`));

startTwitch(process.env.TWITCH_CHANNEL, onChat);
startYouTube(process.env.YT_API_KEY, process.env.YT_LIVE_CHAT_ID, onChat);
watchTwitchGame(process.env.TWITCH_CHANNEL, process.env.TWITCH_CLIENT_ID, process.env.TWITCH_CLIENT_SECRET, (g) => { if (S.autoGame) { console.log("[game] now playing:", g); switchGame(g); } });
