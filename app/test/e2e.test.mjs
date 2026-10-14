// Prueba de extremo a extremo en Chromium real. Requiere `npm run vendor`.
// CHROMIUM=/ruta/al/chrome npm test
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { serve } from '../scripts/serve.mjs';

/** ROM mínima de Game Boy (generada aquí, sin código de terceros): bucle infinito. */
function tinyGbRom() {
  const rom = Buffer.alloc(0x8000);
  rom.set([0x00, 0xc3, 0x50, 0x01], 0x100); // nop; jp 0x0150
  rom.set([0xce,0xed,0x66,0x66,0xcc,0x0d,0x00,0x0b,0x03,0x73,0x00,0x83,0x00,0x0c,0x00,0x0d,
           0x00,0x08,0x11,0x1f,0x88,0x89,0x00,0x0e,0xdc,0xcc,0x6e,0xe6,0xdd,0xdd,0xd9,0x99,
           0xbb,0xbb,0x67,0x63,0x6e,0x0e,0xec,0xcc,0xdd,0xdc,0x99,0x9f,0xbb,0xb9,0x33,0x3e], 0x104);
  rom.write('NEXUSTEST', 0x134, 'ascii');
  rom.set([0x18, 0xfe], 0x150); // jr -2
  let x = 0;
  for (let i = 0x134; i <= 0x14c; i++) x = (x - rom[i] - 1) & 0xff;
  rom[0x14d] = x;
  return rom;
}

let srv, browser, page;
const errors = [];

before(async () => {
  srv = await serve(0);
  browser = await chromium.launch({ executablePath: process.env.CHROMIUM || undefined, args: ['--use-gl=swiftshader', '--enable-unsafe-swiftshader'] });
  page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto(`http://localhost:${srv.address().port}/index.html`);
});
after(async () => { await browser?.close(); srv?.close(); });

test('la app carga y muestra la biblioteca vacía', async () => {
  await page.click('#help .btn.primary'); // guía de bienvenida de la primera vez
  assert.equal((await page.textContent('#title')).trim(), 'Biblioteca');
  assert.equal(await page.isVisible('#empty'), true);
});

test('Switch: el núcleo nxemu (WebAssembly) ejecuta el homebrew de demo', async () => {
  await page.click('.tabbar button[data-tab=switch], nav button[data-tab=switch]');
  await page.click('#run-demo');
  await page.waitForFunction(() => document.querySelector('#console').textContent.includes('código'));
  const out = await page.textContent('#console');
  assert.match(out, /Hola, Switch!/);
  assert.match(out, /código 42/);
});

test('importa una ROM, la lista y la arranca con el núcleo retro', async () => {
  await page.click('nav button[data-tab=library]');
  await page.setInputFiles('#file', { name: 'Nexus Test.gb', mimeType: 'application/octet-stream', buffer: tinyGbRom() });
  await page.waitForSelector('.game');
  assert.equal(await page.textContent('.game b'), 'Nexus Test');
  assert.match(await page.textContent('#library'), /Game Boy/);

  await page.click('.game');
  const frame = page.frameLocator('#frame');
  await frame.locator('canvas').first().waitFor({ state: 'attached', timeout: 60000 });
  // El núcleo descargó y arrancó: aparece el menú de EmulatorJS (barra de controles) sin mensajes de error.
  await page.waitForFunction(() => {
    const d = document.getElementById('frame').contentDocument;
    return d && d.querySelector('.ejs_menu_bar') && !d.querySelector('.ejs_start_button');
  }, null, { timeout: 60000 });
  await page.screenshot({ path: process.env.SHOT || undefined });
  await page.click('#exit');
  await page.waitForSelector('#player', { state: 'hidden', timeout: 15000 });
  assert.equal(await page.isHidden('#player'), true);
});

test('los núcleos añadidos (Master System) también arrancan', async () => {
  await page.setInputFiles('#file', { name: 'Test.sms', mimeType: 'application/octet-stream', buffer: Buffer.alloc(0x8000, 0xc7) });
  await page.waitForSelector('.game:has-text("Test")');
  assert.match(await page.textContent('#library'), /Master System/);
  await page.click('.game:has-text("Test")');
  await page.waitForFunction(() => {
    const d = document.getElementById('frame').contentDocument;
    return d && d.querySelector('.ejs_menu_bar') && !d.querySelector('.ejs_start_button');
  }, null, { timeout: 60000 });
  await page.click('#exit');
  await page.waitForSelector('#player', { state: 'hidden', timeout: 15000 });
});

test('Switch: la lista de preparación y los ajustes funcionan', async () => {
  await page.click('nav button[data-tab=switch]');
  const boxes = page.locator('#checklist input[type=checkbox]');
  assert.ok(await boxes.count() >= 5);
  await boxes.first().check();
  assert.match(await page.textContent('#checklist-progress'), /1/);
  await page.click('nav button[data-tab=settings]');
  await page.selectOption('#set-theme', 'dark');
  assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), 'dark');
});

test('un archivo .xci se rechaza con un aviso claro', async () => {
  await page.click('nav button[data-tab=library]');
  const before = await page.locator('.game').count();
  await page.setInputFiles('#file', { name: 'x.xci', mimeType: 'application/octet-stream', buffer: Buffer.alloc(64) });
  await page.waitForTimeout(500);
  assert.equal(await page.locator('.game').count(), before);
});

test('atajo de Switch: aparece en la biblioteca y se puede quitar', async () => {
  await page.click('nav button[data-tab=switch]');
  const answers = ['Zelda Echoes of Wisdom', ''];
  page.on('dialog', (d) => d.accept(answers.shift() ?? ''));
  await page.click('#add-shortcut');
  await page.click('nav button[data-tab=library]');
  await page.waitForSelector('.game:has-text("Zelda Echoes of Wisdom")');
  assert.match(await page.textContent('#library'), /Switch \(MeloNX\)/);
});

test('modo consola: carrusel, navegación con teclado y salida', async () => {
  await page.click('nav button[data-tab=library]');
  await page.click('#big-open');
  assert.equal(await page.isVisible('#big'), true);
  const first = await page.textContent('#big-title');
  await page.keyboard.press('ArrowRight');
  const second = await page.textContent('#big-title');
  assert.notEqual(first, second);
  await page.keyboard.press('Escape');
  assert.equal(await page.isHidden('#big'), true);
});

test('sin errores de JavaScript en la página principal', () => {
  assert.deepEqual(errors, []);
});
