# CardStream

A self-hosted interactive card system for your own streams. Viewers earn coins by chatting, open card packs,
collect cards by rarity, and play them live; you approve each play and it appears on your OBS overlay.
Original code and design, not affiliated with any other service.

## Run
```
cd cardstream
DASH_PASSWORD=choose-a-password TWITCH_CHANNEL=yourchannel node server.js      # Node 22+
```
Optional YouTube chat: `YT_API_KEY=... YT_LIVE_CHAT_ID=...`. Data is saved in `data/db.json` (set `DATA_DIR` to move it).
Without `DASH_PASSWORD`, a random one is printed at startup.

| Page | Who | What |
|---|---|---|
| `/` | viewers | open packs, inventory, play and gift cards, community goal |
| `/dashboard` | you | play queue (accept / reject / replay), card editor with image, sound and video upload, settings, viewers, tools |
| `/overlay` | OBS | Browser Source, 1920×1080, transparent |

## How viewers get started
1. Open the page; they get a guest account and a link code.
2. They type `!link CODE` in your chat. Their coins and cards move to their chat identity.
3. Chatting earns coins (amount and cooldown in Settings). `!open` opens a pack, `!play cardid` plays a card.

## Features
Rarities and weighted pack odds, collections, per-viewer inventory, manual or auto-accept queue with refunds on reject,
per-card cooldowns, per-viewer play cooldown, daily coins, gifting, community goal reward, text-to-speech,
custom art / sound / video, coin grants, chat simulator, persistent storage.

## Limits (read this)
- **No real payments.** Coins are earned in chat or granted by you. Selling packs would need a payment provider plus legal and tax work.
- Twitch chat is read-only and anonymous; the bot cannot reply in chat. YouTube chat support is untested against a live stream.
- Chat identity is the display name the platform sends. Run it for your own channel; it has no spam or bot protection beyond cooldowns.
- Host it behind HTTPS (e.g. a tunnel or reverse proxy) before exposing the dashboard to the internet.
