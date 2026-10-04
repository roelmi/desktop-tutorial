// Genera los PNG de icono (iOS no acepta SVG para apple-touch-icon) a partir de icons/icon.svg.
import { chromium } from 'playwright';
import { readFileSync, writeFileSync } from 'node:fs';
const svg = readFileSync(new URL('../icons/icon.svg', import.meta.url), 'utf8');
const b = await chromium.launch({ executablePath: process.env.CHROMIUM });
for (const s of [180, 192, 512]) {
  const p = await b.newPage({ viewport: { width: s, height: s } });
  await p.setContent(`<body style="margin:0">${svg.replace('<svg ', `<svg width="${s}" height="${s}" `)}</body>`);
  writeFileSync(new URL(`../icons/icon-${s}.png`, import.meta.url), await p.screenshot());
}
await b.close();
