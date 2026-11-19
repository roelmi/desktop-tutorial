// Per-game card sets + original card art. No copyrighted game assets are used or fetched.
import crypto from "node:crypto";

const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
const RAR = { common: "#9aa4b8", rare: "#3fa7ff", epic: "#b66bff", legendary: "#ffb53f" };

// ---- themes: mood/palette only ----
const THEMES = {
  gta: { match: /gta|grand theft/i, label: "GTA", motif: "sunset", c1: "#ff3d81", c2: "#2b0f5c", c3: "#ffd34d", tag: "neon coastal city at sunset, palm trees, synthwave" },
  battlefield: { match: /battlefield/i, label: "Battlefield", motif: "hex", c1: "#ff7a1a", c2: "#1a1f2b", c3: "#ffd9a0", tag: "large-scale war, smoke and explosions, orange and steel" },
  cod: { match: /call of duty|\bcod\b|warzone|modern warfare|black ops/i, label: "Call of Duty", motif: "tactical", c1: "#6b8f3a", c2: "#0c0f0a", c3: "#ff8a1f", tag: "tactical special forces, night vision, olive and black" },
};
export function themeFor(game) {
  for (const t of Object.values(THEMES)) if (t.match.test(game)) return t;
  const h = parseInt(crypto.createHash("md5").update(game.toLowerCase()).digest("hex").slice(0, 4), 16) % 360;
  return { label: game, motif: "generic", c1: `hsl(${h} 80% 55%)`, c2: `hsl(${(h + 200) % 360} 60% 14%)`, c3: `hsl(${(h + 40) % 360} 95% 65%)`, tag: "bold stylized gaming" };
}

// ---- fallback card ideas (used with no LLM key) ----
const IDEAS = {
  gta: [
    ["Sunset Cruise", "🌴", "common", "challenge", "Drive only at the speed limit for 60 seconds", 60],
    ["Hot Pursuit", "🚔", "rare", "banner", "Police chase time! Chat go crazy!", 8],
    ["Pedestrian Mode", "🚶", "common", "challenge", "No vehicles for the next 2 minutes", 120],
    ["Pink Slip", "🏎️", "rare", "alert", "Steal the next fancy car you see", 7],
    ["Heist Crew", "🎭", "epic", "challenge", "Complete the next objective without shooting", 90],
    ["Gold Rush", "💰", "common", "emojirain", "Cash everywhere!", 6],
    ["Beach Day", "🏖️", "common", "confetti", "Take a beach break in-game", 6],
    ["Most Wanted", "⭐", "legendary", "wanted", "Max wanted level! Run!", 45],
    ["Wild Ride", "🛵", "epic", "challenge", "Only use the slowest vehicle for 90 seconds", 90],
    ["Neon Night", "🌃", "rare", "alert", "Show off the best view in the city", 8],
  ],
  battlefield: [
    ["Squad Up", "🪖", "common", "alert", "Play for the squad: revive every teammate you see", 7],
    ["Smoke Screen", "💨", "common", "challenge", "Use smoke before every push for 2 minutes", 120],
    ["Sledgehammer", "🔨", "rare", "challenge", "Only melee and sidearm for 60 seconds", 60],
    ["Tank Rush", "🛡️", "epic", "banner", "Hop in the nearest vehicle and charge!", 8],
    ["Knife Fight", "🗡️", "rare", "challenge", "Get a knife kill or chat picks your loadout", 60],
    ["Air Support", "🚁", "epic", "alert", "Take to the skies with the next vehicle", 7],
    ["Medals Rain", "🎖️", "common", "emojirain", "Medals for everyone!", 6],
    ["Total War", "💥", "legendary", "wanted", "All-out push: no retreating for 45 seconds", 45],
    ["Silent Sniper", "🎯", "rare", "challenge", "Sniper only until your next death", 90],
    ["Victory Lap", "🏁", "common", "confetti", "Celebrate the win!", 6],
  ],
  cod: [
    ["Drop In", "🪂", "common", "alert", "Land in the hottest spot on the map", 7],
    ["Pistol Only", "🔫", "rare", "challenge", "Sidearm only for 90 seconds", 90],
    ["Quickscope", "🎯", "epic", "challenge", "Sniper only, no scoping for the next 2 kills", 90],
    ["Night Vision", "🌙", "common", "alert", "Play the next fight with the brightness turned down", 8],
    ["Kill Streak", "🔥", "rare", "banner", "Chat demands a streak! Go off!", 8],
    ["Loot Box", "📦", "common", "emojirain", "Loot incoming!", 6],
    ["Camper's Revenge", "🏕️", "rare", "challenge", "Hold one spot until you get a kill", 60],
    ["Squad Wipe", "☠️", "legendary", "wanted", "Win the next fight or lose the loadout", 45],
    ["Sprint Only", "🏃", "epic", "challenge", "No walking or crouching for 60 seconds", 60],
    ["Clutch Time", "🏆", "common", "confetti", "Pull off a clutch!", 6],
  ],
  generic: [
    ["Hype Moment", "🔥", "common", "banner", "Chat go crazy!", 6],
    ["Confetti Pop", "🎉", "common", "confetti", "Celebrate!", 6],
    ["Speed Run", "⚡", "rare", "challenge", "Rush the next objective", 60],
    ["No Look", "🙈", "rare", "challenge", "Play 30 seconds with eyes half closed", 30],
    ["Chat's Choice", "🗳️", "common", "alert", "Chat picks your next move", 8],
    ["Wrong Weapon", "🔧", "epic", "challenge", "Use the worst weapon you have for 90 seconds", 90],
    ["Coin Shower", "🪙", "common", "emojirain", "Make it rain!", 6],
    ["Boss Mode", "👑", "legendary", "wanted", "No dying for 45 seconds!", 45],
    ["Lucky Break", "🍀", "rare", "alert", "Take a risky shortcut", 8],
    ["Plot Twist", "🌀", "epic", "challenge", "Do the opposite of your plan for 60 seconds", 60],
  ],
};
export function fallbackCards(game, count) {
  const key = Object.keys(THEMES).find((k) => THEMES[k].match.test(game)) || "generic";
  return IDEAS[key].slice(0, count).map(([name, emoji, rarity, effect, text, seconds]) => ({ name, emoji, rarity, effect, text, seconds, art: `${name}: ${text}` }));
}

// ---- optional: Claude writes the card set for ANY game ----
export async function llmCards(game, count, apiKey) {
  const prompt = `Create ${count} interactive stream cards for viewers of a streamer playing "${game}". Each card is a short, harmless action or challenge a viewer triggers live on stream.
Return ONLY a JSON array. Each item: {"name": max 28 chars, "emoji": one emoji, "text": max 90 chars, the instruction shown on stream, "rarity": "common"|"rare"|"epic"|"legendary", "effect": "alert"|"confetti"|"emojirain"|"banner"|"challenge"|"wanted", "seconds": integer 5-90, "art": max 120 chars describing an ORIGINAL illustration (no trademarked characters or logos)}.
Rarity mix roughly: 40% common, 30% rare, 20% epic, 10% legendary. Use game-appropriate actions. Nothing dangerous, hateful or harassing; do not use the names of copyrighted characters.`;
  const r = await fetch("https://api.anthropic.com/v1/messages", {
    method: "POST", headers: { "x-api-key": apiKey, "anthropic-version": "2023-06-01", "content-type": "application/json" },
    body: JSON.stringify({ model: process.env.CARD_MODEL || "claude-sonnet-5-5", max_tokens: 3000, messages: [{ role: "user", content: prompt }] }),
  });
  if (!r.ok) throw new Error(`Claude API ${r.status}`);
  const txt = (await r.json()).content?.map((b) => b.text || "").join("") || "";
  const arr = JSON.parse(txt.slice(txt.indexOf("["), txt.lastIndexOf("]") + 1));
  if (!Array.isArray(arr) || !arr.length) throw new Error("No cards returned");
  return arr.slice(0, count);
}

// ---- local original art (SVG) ----
function rng(seed) { let s = parseInt(crypto.createHash("md5").update(seed).digest("hex").slice(0, 8), 16) || 1; return () => ((s = (s * 1664525 + 1013904223) >>> 0) / 4294967296); }

export function svgArt(card, game) {
  const t = themeFor(game), R = rng(card.id + game), rc = RAR[card.rarity] || RAR.common;
  const W = 512, H = 512; let layers = "";
  if (t.motif === "sunset") {
    layers += `<circle cx="256" cy="300" r="150" fill="${t.c3}"/>` + [0, 1, 2, 3, 4].map((i) => `<rect x="90" y="${290 + i * 22}" width="340" height="${4 + i * 3}" fill="${t.c2}"/>`).join("");
    for (let i = 0; i < 9; i++) { const w = 28 + R() * 26, h = 60 + R() * 120; layers += `<rect x="${i * 58 - 10}" y="${H - 120 - h * 0.4}" width="${w}" height="${h + 60}" fill="#14072e" opacity=".92"/>`; }
    for (const x of [60, 430]) layers += `<rect x="${x}" y="250" width="8" height="200" fill="#0b0420"/><path d="M${x + 4} 250 q-60 -20 -80 20 M${x + 4} 250 q60 -20 80 20 M${x + 4} 250 q-30 -50 -60 -30 M${x + 4} 250 q30 -50 60 -30" stroke="#0b0420" stroke-width="9" fill="none" stroke-linecap="round"/>`;
  } else if (t.motif === "hex") {
    layers += `<defs><pattern id="hx" width="48" height="42" patternUnits="userSpaceOnUse" patternTransform="rotate(0)"><path d="M12 0h24l12 21-12 21H12L0 21z" fill="none" stroke="${t.c3}" stroke-opacity=".18" stroke-width="2"/></pattern><filter id="bl"><feGaussianBlur stdDeviation="14"/></filter></defs><rect width="${W}" height="${H}" fill="url(#hx)"/>`;
    for (let i = 0; i < 7; i++) layers += `<circle cx="${R() * W}" cy="${200 + R() * 300}" r="${40 + R() * 70}" fill="#0b0d12" opacity=".55" filter="url(#bl)"/>`;
    layers += [0, 1, 2].map((i) => `<path d="M40 ${70 + i * 26} l60 -22 60 22 -60 -8z" fill="${t.c3}" opacity="${0.7 - i * 0.2}"/>`).join("");
  } else if (t.motif === "tactical") {
    for (let i = 0; i < 8; i++) layers += `<ellipse cx="${R() * W}" cy="${R() * H}" rx="${50 + R() * 90}" ry="${30 + R() * 60}" fill="${i % 2 ? "#3c4d22" : "#1a2210"}" opacity=".7"/>`;
    layers += `<g stroke="${t.c3}" stroke-opacity=".25">${Array.from({ length: 9 }, (_, i) => `<line x1="${i * 64}" y1="0" x2="${i * 64}" y2="${H}"/><line x1="0" y1="${i * 64}" x2="${W}" y2="${i * 64}"/>`).join("")}</g>`;
    layers += `<g stroke="${t.c3}" stroke-width="3" fill="none" opacity=".8"><circle cx="256" cy="256" r="170"/><circle cx="256" cy="256" r="120" stroke-opacity=".5"/><path d="M256 60v50M256 402v50M60 256h50M402 256h50"/></g>`;
  } else {
    for (let i = 0; i < 6; i++) layers += `<rect x="${-100 + i * 110}" y="-20" width="46" height="${H + 40}" fill="${t.c3}" opacity=".12" transform="rotate(25 256 256)"/>`;
    for (let i = 0; i < 6; i++) layers += `<circle cx="${R() * W}" cy="${R() * H}" r="${20 + R() * 60}" fill="${t.c3}" opacity=".15"/>`;
  }
  const name = esc(card.name.slice(0, 22));
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${W} ${H}" width="${W}" height="${H}">
<defs><linearGradient id="bg" x1="0" y1="0" x2="${R() > 0.5 ? 1 : 0.4}" y2="1"><stop offset="0" stop-color="${t.c1}"/><stop offset="1" stop-color="${t.c2}"/></linearGradient>
<radialGradient id="gl"><stop offset="0" stop-color="${rc}" stop-opacity=".55"/><stop offset="1" stop-color="${rc}" stop-opacity="0"/></radialGradient></defs>
<rect width="${W}" height="${H}" fill="url(#bg)"/>${layers}
<circle cx="256" cy="230" r="180" fill="url(#gl)"/><circle cx="256" cy="230" r="112" fill="#0008" stroke="${rc}" stroke-width="5"/>
<text x="256" y="230" font-size="128" text-anchor="middle" dominant-baseline="central" font-family="Segoe UI Emoji,Apple Color Emoji,Noto Color Emoji,sans-serif">${esc(card.emoji)}</text>
<rect x="0" y="392" width="${W}" height="120" fill="#000b"/>
<text x="256" y="438" font-size="40" font-weight="800" fill="#fff" text-anchor="middle" font-family="Segoe UI,Arial,sans-serif">${name}</text>
<text x="256" y="476" font-size="20" fill="${rc}" text-anchor="middle" letter-spacing="4" font-family="Segoe UI,Arial,sans-serif">${esc(card.rarity.toUpperCase())} · ${esc(t.label.slice(0, 20).toUpperCase())}</text>
<rect x="5" y="5" width="${W - 10}" height="${H - 10}" rx="22" fill="none" stroke="${rc}" stroke-width="10"/></svg>`;
}

// ---- optional: AI-generated images (OpenAI Images API) ----
export async function aiImage(card, game, apiKey) {
  const t = themeFor(game);
  const prompt = `Original trading-card illustration themed for the video game "${game}" (${t.tag}). Subject: ${card.art || card.name}. Stylized digital painting, dramatic lighting, square composition. Do NOT include any text, logos, watermarks, real people, or copyrighted characters.`;
  const r = await fetch("https://api.openai.com/v1/images/generations", {
    method: "POST", headers: { authorization: `Bearer ${apiKey}`, "content-type": "application/json" },
    body: JSON.stringify({ model: process.env.IMAGE_MODEL || "gpt-image-1", prompt, size: "1024x1024", n: 1 }),
  });
  if (!r.ok) throw new Error(`Image API ${r.status}`);
  const b64 = (await r.json()).data?.[0]?.b64_json; if (!b64) throw new Error("No image returned");
  return Buffer.from(b64, "base64");
}
