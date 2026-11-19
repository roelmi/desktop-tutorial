// Reads the category (game) your Twitch channel is streaming, using public Helix data (app token, no user login).
// Needs TWITCH_CLIENT_ID + TWITCH_CLIENT_SECRET (free, from dev.twitch.tv). Untested without real credentials.
export function watchTwitchGame(channel, clientId, secret, onGame) {
  if (!channel || !clientId || !secret) return;
  let token = "", last = null;
  async function tick() {
    try {
      if (!token) token = (await (await fetch(`https://id.twitch.tv/oauth2/token?client_id=${clientId}&client_secret=${secret}&grant_type=client_credentials`, { method: "POST" })).json()).access_token || "";
      const r = await fetch(`https://api.twitch.tv/helix/streams?user_login=${encodeURIComponent(channel)}`, { headers: { "client-id": clientId, authorization: `Bearer ${token}` } });
      if (r.status === 401) { token = ""; return; }
      const g = (await r.json()).data?.[0]?.game_name || null;
      if (g && g !== last) { last = g; onGame(g); }
    } catch (e) { console.error("Twitch game watch:", e.message); }
  }
  tick(); setInterval(tick, 60000).unref();
  console.log("Twitch: watching the category of #" + channel);
}
