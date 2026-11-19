import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { startTwitch } from "./adapters/twitch.js";
import { startYouTube } from "./adapters/youtube.js";

const dir = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.PORT || 3000);
const cards = JSON.parse(fs.readFileSync(path.join(dir, "cards.json"), "utf8"));
const START_COINS = 100, COOLDOWN_MS = 5000, QUEUE_MAX = 20;

const wallets = new Map();   // "platform:viewer" -> coins   (in memory; resets on restart)
const lastPlay = new Map();  // "platform:viewer" -> timestamp
const overlays = new Set();  // SSE clients
const history = [];

const key = (p, v) => `${p}:${v.toLowerCase()}`;
const coins = (k) => { if (!wallets.has(k)) wallets.set(k, START_COINS); return wallets.get(k); };

function play({ platform, viewer, cardId }) {
  const card = cards.find((c) => c.id === cardId);
  if (!card) return { ok: false, error: "Unknown card" };
  if (!viewer || viewer.length > 40) return { ok: false, error: "Enter a name" };
  const k = key(platform, viewer);
  const now = Date.now();
  if (now - (lastPlay.get(k) || 0) < COOLDOWN_MS) return { ok: false, error: "Cooldown: wait a few seconds" };
  if (coins(k) < card.cost) return { ok: false, error: "Not enough coins" };
  if (history.filter((h) => now - h.at < 60000).length >= QUEUE_MAX) return { ok: false, error: "Stream is busy, try again" };
  wallets.set(k, coins(k) - card.cost);
  lastPlay.set(k, now);
  const event = { type: "card", card, viewer, platform, at: now };
  history.push(event); if (history.length > 100) history.shift();
  for (const res of overlays) res.write(`data: ${JSON.stringify(event)}\n\n`);
  return { ok: true, coins: coins(k) };
}

// Chat command: !play <cardId>  (viewer name comes from the platform)
function onChat({ platform, viewer, text }) {
  const m = /^!play\s+(\w+)/i.exec(text);
  if (m) console.log(`[${platform}] ${viewer}:`, JSON.stringify(play({ platform, viewer, cardId: m[1].toLowerCase() })));
}

const types = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json" };
const json = (res, code, obj) => { res.writeHead(code, { "content-type": "application/json" }); res.end(JSON.stringify(obj)); };

http.createServer((req, res) => {
  const url = new URL(req.url, "http://x");
  if (url.pathname === "/events") {
    res.writeHead(200, { "content-type": "text/event-stream", "cache-control": "no-cache", connection: "keep-alive" });
    res.write(": connected\n\n"); overlays.add(res); req.on("close", () => overlays.delete(res)); return;
  }
  if (url.pathname === "/api/cards") return json(res, 200, cards);
  if (url.pathname === "/api/wallet") return json(res, 200, { coins: coins(key(url.searchParams.get("platform") || "web", url.searchParams.get("viewer") || "")) });
  if (url.pathname === "/api/play" && req.method === "POST") {
    let body = ""; req.on("data", (d) => { body += d; if (body.length > 4096) req.destroy(); });
    req.on("end", () => {
      try { const b = JSON.parse(body); const p = ["twitch", "youtube", "web"].includes(b.platform) ? b.platform : "web";
        const r = play({ platform: p, viewer: String(b.viewer || "").trim(), cardId: String(b.cardId || "") }); json(res, r.ok ? 200 : 400, r);
      } catch { json(res, 400, { ok: false, error: "Bad request" }); }
    }); return;
  }
  const file = url.pathname === "/" ? "/index.html" : url.pathname === "/overlay" ? "/overlay.html" : url.pathname;
  const full = path.join(dir, "public", path.normalize(file).replace(/^(\.\.[/\\])+/, ""));
  if (!full.startsWith(path.join(dir, "public")) || !fs.existsSync(full) || fs.statSync(full).isDirectory()) { res.writeHead(404); return res.end("Not found"); }
  res.writeHead(200, { "content-type": types[path.extname(full)] || "application/octet-stream" });
  fs.createReadStream(full).pipe(res);
}).listen(PORT, () => console.log(`CardStream on http://localhost:${PORT}  (overlay: /overlay)`));

startTwitch(process.env.TWITCH_CHANNEL, onChat);
startYouTube(process.env.YT_API_KEY, process.env.YT_LIVE_CHAT_ID, onChat);
