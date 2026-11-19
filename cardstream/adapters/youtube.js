// YouTube Live chat via the official Data API (polling). Needs YT_API_KEY and YT_LIVE_CHAT_ID.
// Untested against a real stream in this prototype.
export function startYouTube(apiKey, liveChatId, onCommand) {
  if (!apiKey || !liveChatId) return;
  let pageToken = "";
  async function poll() {
    let wait = 5000;
    try {
      const u = new URL("https://www.googleapis.com/youtube/v3/liveChat/messages");
      u.search = new URLSearchParams({ liveChatId, part: "snippet,authorDetails", key: apiKey, ...(pageToken && { pageToken }) });
      const r = await (await fetch(u)).json();
      pageToken = r.nextPageToken || pageToken;
      wait = Math.max(r.pollingIntervalMillis || 5000, 5000);
      for (const it of r.items || [])
        onCommand({ platform: "youtube", viewer: it.authorDetails.displayName, text: (it.snippet.displayMessage || "").trim() });
    } catch (e) { console.error("YouTube poll error:", e.message); }
    setTimeout(poll, wait);
  }
  poll();
  console.log("YouTube: polling live chat");
}
