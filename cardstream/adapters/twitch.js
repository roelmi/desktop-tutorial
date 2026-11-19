// Read-only Twitch chat via anonymous IRC (no login, no token). Commands: !play <cardId>
export function startTwitch(channel, onCommand) {
  if (!channel) return;
  const ws = new WebSocket("wss://irc-ws.chat.twitch.tv:443");
  ws.onopen = () => {
    ws.send("NICK justinfan" + Math.floor(Math.random() * 90000 + 10000));
    ws.send("JOIN #" + channel.toLowerCase());
  };
  ws.onmessage = (ev) => {
    for (const line of String(ev.data).split("\r\n")) {
      if (line.startsWith("PING")) ws.send("PONG :tmi.twitch.tv");
      const m = /^:(\w+)!\w+@\w+\.tmi\.twitch\.tv PRIVMSG #\w+ :(.*)$/.exec(line);
      if (m) onCommand({ platform: "twitch", viewer: m[1], text: m[2].trim() });
    }
  };
  ws.onclose = () => setTimeout(() => startTwitch(channel, onCommand), 5000);
  ws.onerror = () => {};
  console.log("Twitch: listening to #" + channel);
}
