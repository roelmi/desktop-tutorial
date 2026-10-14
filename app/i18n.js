// Idiomas de Nexus: inglés (por defecto) y español. Las claves son comunes; cada idioma aporta su texto.
// Los textos marcados como HTML son cadenas propias de la app (nunca datos del usuario).
export const LANGS = { en: 'English', es: 'Español' };
export const DEFAULT_LANG = 'en';
const KEY = 'nexus.lang';

const en = {
  'tab.library': 'Library', 'tab.switch': 'Switch', 'tab.settings': 'Settings',

  'a11y.console': 'Console mode', 'a11y.search': 'Search', 'a11y.add': 'Add games', 'a11y.close': 'Close',
  'a11y.filter': 'Filter by console', 'a11y.bigClose': 'Exit console mode', 'a11y.exit': 'Leave game', 'a11y.game': 'Game',
  'a11y.fav': 'Favorite', 'a11y.play': 'Play {title}', 'a11y.options': 'Options for {title}', 'search.placeholder': 'Search games',

  'install.title': 'Install Nexus',
  'install.body': 'Tap <span class="share">Share</span> and then <b>Add to Home Screen</b> to use it like an app: full screen and offline.',

  'empty.title': 'Your library is empty',
  'empty.body': 'Add game files from <b>your own cartridges or discs</b>. Nexus does not include or download games.',
  'empty.add': 'Add games', 'empty.formats': 'Supported formats', 'formats.extra': 'also .zip and .7z.',

  'lib.noResults': 'No results for “{q}”.', 'lib.noFilter': 'No games in this filter.',
  'lib.continue': 'Continue playing', 'lib.favorites': 'Favorites', 'lib.all': 'All games', 'lib.results': 'Results',
  'chip.all': 'All', 'chip.fav': '★ Favorites',

  'sheet.shortcut': 'shortcut', 'sheet.played': 'played {date}',
  'sheet.play': '▶  Play', 'sheet.unfav': '☆  Remove from favorites', 'sheet.fav': '★  Add to favorites',
  'sheet.rename': '✎  Rename', 'sheet.cover': '🖼  Choose cover…', 'sheet.removeCover': 'Remove cover',
  'sheet.delete': '🗑  Delete from device', 'sheet.close': 'Close', 'prompt.rename': 'New name',
  'confirm.removeShortcut': 'Remove the shortcut “{title}”? (the game in MeloNX is not touched)',
  'confirm.deleteGame': 'Delete “{title}” and its saved games?',

  'toast.coverUpdated': 'Cover updated', 'toast.coverFail': 'Could not use that image',
  'picker.title': 'Choose the console', 'picker.skip': 'Skip this file',
  'import.progress': 'Importing {i} of {n}…',
  'import.switchUnsupported': 'Switch games (XCI/NSP) cannot run in Nexus. See the Switch tab.',
  'import.badFormat': 'Unsupported format: .{ext}',
  'import.noSpace': 'Not enough space on the device', 'import.saveFail': 'Could not save the game',
  'import.added.one': '{n} game added', 'import.added.other': '{n} games added',
  'import.dup.one': '{n} was already there', 'import.dup.other': '{n} were already there',
  'import.nothing': 'Nothing to import',

  'melonx.opening': 'Opening MeloNX… (if nothing happens, install it and open it once)',
  'melonx.promptName': 'Game name exactly as it appears in MeloNX',
  'melonx.promptId': 'Title ID (optional, 16 hexadecimal characters; leave empty to search by name)',
  'melonx.opens': 'opens MeloNX',

  'big.empty': 'No games yet', 'big.emptySub': 'Add ROMs from the library.',
  'big.play': 'Play', 'big.fav': 'Favorite', 'big.exit': 'Exit',

  'switch.hero.title': 'Nintendo Switch',
  'switch.hero.body': 'Nexus is a web app and <b>cannot run Switch games</b>: that needs JIT compilation and native code that the iPhone browser does not allow. On iPhone there is <b>MeloNX</b> (based on Ryujinx), which is installed separately. This checklist helps you get ready.',
  'switch.site': 'MeloNX website', 'switch.source': 'Source code',
  'switch.short.title': 'Your Switch games in Nexus',
  'switch.short.body': 'Create a shortcut for each game you already have in MeloNX: it appears in your library next to the others and, when tapped, Nexus opens MeloNX and launches it. Nexus does not store or copy the game; it only remembers its name. MeloNX enables JIT according to its own settings.',
  'switch.addShortcut': 'Add game shortcut', 'switch.openMelonx': 'Open MeloNX',
  'switch.prep': 'Preparation checklist',
  'switch.progress': '{a} of {b} steps ready',
  'switch.devices': 'Which emulator for each device', 'switch.th.device': 'Device', 'switch.th.option': 'Option',
  'switch.dev.ios': 'iPhone / iPad', 'switch.opt.ios': 'MeloNX (sideload + JIT)',
  'switch.dev.android': 'Android (recent Snapdragon)', 'switch.dev.pc': 'Windows / Linux PC', 'switch.dev.mac': 'Mac',
  'switch.opt.eden': 'Eden', 'switch.opt.edenRyu': 'Eden or Ryubing',
  'switch.devices.note': 'The original Ryujinx is no longer maintained; Ryubing is its continuation. All of them require keys and firmware from <b>your own console</b> and copies of your games.',
  'switch.lab.title': 'Lab: <span class="mono">nxemu</span> <span class="tag">experimental</span>',
  'switch.lab.body': 'Our own core (ARM64 CPU + high-level kernel in WebAssembly) that runs simple <b>homebrew</b> <code>.nro</code> programs. It does not run commercial games.',
  'switch.lab.run': '▶ Run demo', 'switch.lab.open': 'Open .nro…',
  'lab.running': 'Running {name}…', 'lab.error': '[error] {msg}', 'lab.exited': '[process exited with code {code}]',

  'check.0': 'My iPhone is an iPhone 12 or later (recommended: 15 Pro or newer).',
  'check.1': 'I have a computer (Windows or Mac) for the first sideload installation.',
  'check.2': 'I installed SideStore or AltStore from their official page.',
  'check.3': 'I installed MeloNX from its official page (not from sites that repackage the IPA).',
  'check.4': 'I know how to enable JIT (for example with StikDebug) every time I open MeloNX.',
  'check.5': 'I have the keys and firmware from MY OWN Switch.',
  'check.6': 'I have copies of MY OWN games.',

  'set.appearance': 'Appearance', 'set.theme': 'Theme', 'theme.auto': 'Automatic', 'theme.dark': 'Dark', 'theme.light': 'Light',
  'set.group': 'Group by console', 'set.sort': 'Sort by', 'sort.recent': 'Most recent', 'sort.name': 'Name', 'sort.added': 'Date added',
  'set.language': 'Language',
  'set.play': 'While playing', 'set.cover': 'Cover from the last game frame', 'set.wake': 'Keep the screen on', 'set.pause': 'Pause when leaving the app',
  'pad.none': 'Controllers: none connected. Pair one over Bluetooth in iOS Settings and press a button.',
  'pad.connected': '🎮 Controller connected: {names}', 'toast.padConnected': 'Controller connected',
  'set.storage': 'Storage', 'usage.calc': 'Calculating…',
  'usage.games.one': '{n} game · {mb} MB', 'usage.games.other': '{n} games · {mb} MB',
  'usage.quota': ' · used by Nexus (total): {a} of {b} MB',
  'persist.yes': '✓ Data protected: the browser will not delete it for lack of use.',
  'persist.no': 'Your data is not yet protected against automatic deletion.',
  'persist.btn': 'Protect my data', 'cache.btn': 'Clear app cache', 'clear.btn': 'Delete the whole library',
  'storage.note': 'Safari may delete the data of a site you have not used for weeks. Install Nexus on your Home Screen and tap “Protect my data” to avoid it.',
  'toast.persistOk': 'Data protected',
  'toast.persistNo': 'Safari did not grant it. Install Nexus on your Home Screen and try again.',
  'toast.cacheOk': 'Cache cleared', 'toast.cacheFail': 'Could not clear the cache',
  'confirm.clear': 'Delete ALL games from the library? This cannot be undone.',
  'set.about': 'About',
  'about.body': 'Nexus stores your games and saves only on this device. The retro cores are <a href="https://emulatorjs.org" target="_blank" rel="noopener">EmulatorJS</a> / libretro (GPL, LGPL and others). It includes no games, BIOS, keys or firmware: use only copies you own.',
  'about.help': 'Show the welcome guide',

  'help.title': 'Welcome to Nexus', 'help.sub': 'Your classic consoles in one app.',
  'help.1.b': 'Install it: ', 'help.1.t': 'in Safari tap Share and “Add to Home Screen”. It then opens full screen and works offline.',
  'help.2.b': 'Add your games: ', 'help.2.t': 'tap ＋ and pick files from your own cartridges or discs. Nexus does not include or download games.',
  'help.3.b': 'Play: ', 'help.3.t': 'tap a game. Use the touch controls or a Bluetooth controller. The game menu saves and loads states.',
  'help.4.b': 'Organize: ', 'help.4.t': 'press and hold a game to favorite it, change its cover or delete it.',
  'help.switch': 'Switch: Nexus cannot run Switch games. The “Switch” tab guides you to use MeloNX on your iPhone.',
  'help.ok': 'Got it',

  'player.noGame': 'The game was not found in the library.', 'player.noEjs': 'This system does not use EmulatorJS.',
  'player.noCores': 'The emulation cores are missing. Run `npm run vendor` in app/.',
};

const es = {
  'tab.library': 'Biblioteca', 'tab.switch': 'Switch', 'tab.settings': 'Ajustes',

  'a11y.console': 'Modo consola', 'a11y.search': 'Buscar', 'a11y.add': 'Añadir juegos', 'a11y.close': 'Cerrar',
  'a11y.filter': 'Filtrar por consola', 'a11y.bigClose': 'Salir del modo consola', 'a11y.exit': 'Salir del juego', 'a11y.game': 'Juego',
  'a11y.fav': 'Favorito', 'a11y.play': 'Jugar a {title}', 'a11y.options': 'Opciones de {title}', 'search.placeholder': 'Buscar juegos',

  'install.title': 'Instala Nexus',
  'install.body': 'Pulsa <span class="share">Compartir</span> y luego <b>Añadir a pantalla de inicio</b> para usarlo como una app, a pantalla completa y sin conexión.',

  'empty.title': 'Tu biblioteca está vacía',
  'empty.body': 'Añade archivos de juegos de <b>tus propios cartuchos o discos</b>. Nexus no incluye ni descarga juegos.',
  'empty.add': 'Añadir juegos', 'empty.formats': 'Formatos admitidos', 'formats.extra': 'también .zip y .7z.',

  'lib.noResults': 'Sin resultados para «{q}».', 'lib.noFilter': 'No hay juegos en este filtro.',
  'lib.continue': 'Seguir jugando', 'lib.favorites': 'Favoritos', 'lib.all': 'Todos los juegos', 'lib.results': 'Resultados',
  'chip.all': 'Todos', 'chip.fav': '★ Favoritos',

  'sheet.shortcut': 'atajo', 'sheet.played': 'jugado {date}',
  'sheet.play': '▶  Jugar', 'sheet.unfav': '☆  Quitar de favoritos', 'sheet.fav': '★  Añadir a favoritos',
  'sheet.rename': '✎  Renombrar', 'sheet.cover': '🖼  Elegir carátula…', 'sheet.removeCover': 'Quitar carátula',
  'sheet.delete': '🗑  Eliminar del dispositivo', 'sheet.close': 'Cerrar', 'prompt.rename': 'Nuevo nombre',
  'confirm.removeShortcut': '¿Quitar el atajo «{title}»? (el juego en MeloNX no se toca)',
  'confirm.deleteGame': '¿Eliminar «{title}» y sus partidas guardadas?',

  'toast.coverUpdated': 'Carátula actualizada', 'toast.coverFail': 'No se pudo usar esa imagen',
  'picker.title': 'Elige la consola', 'picker.skip': 'Omitir este archivo',
  'import.progress': 'Importando {i} de {n}…',
  'import.switchUnsupported': 'Los juegos de Switch (XCI/NSP) no se pueden ejecutar en Nexus. Mira la pestaña Switch.',
  'import.badFormat': 'Formato no soportado: .{ext}',
  'import.noSpace': 'No hay espacio suficiente en el dispositivo', 'import.saveFail': 'No se pudo guardar el juego',
  'import.added.one': '{n} juego añadido', 'import.added.other': '{n} juegos añadidos',
  'import.dup.one': '{n} ya estaba', 'import.dup.other': '{n} ya estaban',
  'import.nothing': 'Nada que importar',

  'melonx.opening': 'Abriendo MeloNX… (si no pasa nada, instálalo y ábrelo una vez)',
  'melonx.promptName': 'Nombre del juego tal como aparece en MeloNX',
  'melonx.promptId': 'ID del título (opcional, 16 caracteres hexadecimales; déjalo vacío para buscar por nombre)',
  'melonx.opens': 'abre MeloNX',

  'big.empty': 'Sin juegos todavía', 'big.emptySub': 'Añade ROMs desde la biblioteca.',
  'big.play': 'Jugar', 'big.fav': 'Favorito', 'big.exit': 'Salir',

  'switch.hero.title': 'Nintendo Switch',
  'switch.hero.body': 'Nexus es una app web y <b>no puede ejecutar juegos de Switch</b>: eso exige compilación JIT y código nativo que el navegador del iPhone no permite. En iPhone existe <b>MeloNX</b> (basado en Ryujinx), que se instala por separado. Esta lista te ayuda a prepararlo.',
  'switch.site': 'Web de MeloNX', 'switch.source': 'Código fuente',
  'switch.short.title': 'Tus juegos de Switch en Nexus',
  'switch.short.body': 'Crea un atajo por cada juego que ya tengas en MeloNX: aparecerá en tu biblioteca junto a los demás y, al tocarlo, Nexus abre MeloNX y lo lanza. Nexus no guarda ni copia el juego; solo recuerda su nombre. MeloNX activa el JIT según su propia configuración.',
  'switch.addShortcut': 'Añadir atajo de juego', 'switch.openMelonx': 'Abrir MeloNX',
  'switch.prep': 'Lista de preparación',
  'switch.progress': '{a} de {b} pasos listos',
  'switch.devices': 'Qué emulador usar en cada dispositivo', 'switch.th.device': 'Dispositivo', 'switch.th.option': 'Opción',
  'switch.dev.ios': 'iPhone / iPad', 'switch.opt.ios': 'MeloNX (sideload + JIT)',
  'switch.dev.android': 'Android (Snapdragon reciente)', 'switch.dev.pc': 'PC Windows / Linux', 'switch.dev.mac': 'Mac',
  'switch.opt.eden': 'Eden', 'switch.opt.edenRyu': 'Eden o Ryubing',
  'switch.devices.note': 'Ryujinx original ya no se mantiene; Ryubing es su continuación. Todos exigen claves y firmware de <b>tu propia consola</b> y copias de tus juegos.',
  'switch.lab.title': 'Laboratorio: <span class="mono">nxemu</span> <span class="tag">experimental</span>',
  'switch.lab.body': 'Núcleo propio (CPU ARM64 + kernel de alto nivel en WebAssembly) que ejecuta programas <b>homebrew</b> sencillos <code>.nro</code>. No ejecuta juegos comerciales.',
  'switch.lab.run': '▶ Ejecutar demo', 'switch.lab.open': 'Abrir .nro…',
  'lab.running': 'Ejecutando {name}…', 'lab.error': '[error] {msg}', 'lab.exited': '[proceso terminó con código {code}]',

  'check.0': 'Mi iPhone es un iPhone 12 o posterior (recomendado: 15 Pro o superior).',
  'check.1': 'Tengo un ordenador (Windows o Mac) para la primera instalación por sideload.',
  'check.2': 'He instalado SideStore o AltStore desde su página oficial.',
  'check.3': 'He instalado MeloNX desde su página oficial (no desde sitios que reempaquetan el IPA).',
  'check.4': 'Sé activar el JIT (por ejemplo con StikDebug) cada vez que abro MeloNX.',
  'check.5': 'Tengo las claves y el firmware de MI PROPIA Switch.',
  'check.6': 'Tengo copias de MIS propios juegos.',

  'set.appearance': 'Apariencia', 'set.theme': 'Tema', 'theme.auto': 'Automático', 'theme.dark': 'Oscuro', 'theme.light': 'Claro',
  'set.group': 'Agrupar por consola', 'set.sort': 'Ordenar por', 'sort.recent': 'Más recientes', 'sort.name': 'Nombre', 'sort.added': 'Fecha de alta',
  'set.language': 'Idioma',
  'set.play': 'Al jugar', 'set.cover': 'Carátula con la última imagen del juego', 'set.wake': 'Mantener la pantalla encendida', 'set.pause': 'Pausar al salir de la app',
  'pad.none': 'Mandos: ninguno conectado. Empareja uno por Bluetooth en Ajustes de iOS y pulsa un botón.',
  'pad.connected': '🎮 Mando conectado: {names}', 'toast.padConnected': 'Mando conectado',
  'set.storage': 'Almacenamiento', 'usage.calc': 'Calculando…',
  'usage.games.one': '{n} juego · {mb} MB', 'usage.games.other': '{n} juegos · {mb} MB',
  'usage.quota': ' · usado por Nexus (total): {a} de {b} MB',
  'persist.yes': '✓ Datos protegidos: el navegador no los borrará por falta de uso.',
  'persist.no': 'Los datos aún no están protegidos contra borrado automático.',
  'persist.btn': 'Proteger mis datos', 'cache.btn': 'Vaciar caché de la app', 'clear.btn': 'Borrar toda la biblioteca',
  'storage.note': 'Safari puede borrar los datos de un sitio que no se usa en semanas. Instala Nexus en la pantalla de inicio y pulsa «Proteger mis datos» para evitarlo.',
  'toast.persistOk': 'Datos protegidos',
  'toast.persistNo': 'Safari no lo concedió. Instala Nexus en la pantalla de inicio e inténtalo de nuevo.',
  'toast.cacheOk': 'Caché vaciada', 'toast.cacheFail': 'No se pudo vaciar la caché',
  'confirm.clear': '¿Borrar TODOS los juegos de la biblioteca? No se puede deshacer.',
  'set.about': 'Acerca de',
  'about.body': 'Nexus guarda tus juegos y partidas solo en este dispositivo. Los núcleos retro son <a href="https://emulatorjs.org" target="_blank" rel="noopener">EmulatorJS</a> / libretro (GPL, LGPL y otras). No incluye juegos, BIOS, claves ni firmware: usa únicamente copias de tu propiedad.',
  'about.help': 'Ver la guía de inicio',

  'help.title': 'Bienvenido a Nexus', 'help.sub': 'Tus consolas clásicas, en una sola app.',
  'help.1.b': 'Instálala: ', 'help.1.t': 'en Safari pulsa Compartir y «Añadir a pantalla de inicio». Así se abre a pantalla completa y funciona sin conexión.',
  'help.2.b': 'Añade tus juegos: ', 'help.2.t': 'pulsa ＋ y elige archivos de tus propios cartuchos o discos. Nexus no incluye ni descarga juegos.',
  'help.3.b': 'Juega: ', 'help.3.t': 'toca un juego. Usa los controles táctiles o un mando Bluetooth. El menú del juego guarda y carga partidas.',
  'help.4.b': 'Organiza: ', 'help.4.t': 'mantén pulsado un juego para marcarlo como favorito, cambiar su carátula o eliminarlo.',
  'help.switch': 'Switch: Nexus no puede ejecutar juegos de Switch. La pestaña «Switch» te guía para usar MeloNX en tu iPhone.',
  'help.ok': 'Entendido',

  'player.noGame': 'No se encontró el juego en la biblioteca.', 'player.noEjs': 'Este sistema no usa EmulatorJS.',
  'player.noCores': 'Faltan los núcleos de emulación. Ejecuta `npm run vendor` en app/.',
};

const DICT = { en, es };

function readLang() {
  try { const v = localStorage.getItem(KEY); if (v && v in DICT) return v; } catch { /* almacenamiento bloqueado */ }
  return DEFAULT_LANG;
}
let lang = readLang();

export const getLang = () => lang;
export function setLang(l) {
  if (!(l in DICT)) return;
  lang = l;
  try { localStorage.setItem(KEY, l); } catch { /* almacenamiento bloqueado */ }
  document.documentElement.lang = l;
}

/** t('clave', { n: 3 }) → texto traducido con {n} sustituido. Si falta en el idioma activo cae al inglés. */
export function t(key, vars = {}) {
  const s = DICT[lang][key] ?? DICT.en[key] ?? key;
  return s.replace(/\{(\w+)\}/g, (_, k) => (k in vars ? String(vars[k]) : `{${k}}`));
}

/** Texto con singular/plural: tn('import.added', 2, { n: 2 }) usa «.one» o «.other». */
export const tn = (key, n, vars = {}) => t(`${key}.${n === 1 ? 'one' : 'other'}`, { n, ...vars });

/**
 * Traduce el HTML estático:
 *  data-i18n="clave"            → textContent
 *  data-i18n-html="clave"       → innerHTML (solo cadenas propias)
 *  data-i18n-attr="aria-label:clave;placeholder:clave" → atributos
 */
export function applyStatic(root = document) {
  document.documentElement.lang = lang;
  root.querySelectorAll('[data-i18n]').forEach((e) => { e.textContent = t(e.dataset.i18n); });
  root.querySelectorAll('[data-i18n-html]').forEach((e) => { e.innerHTML = t(e.dataset.i18nHtml); });
  root.querySelectorAll('[data-i18n-attr]').forEach((e) => {
    for (const pair of e.dataset.i18nAttr.split(';')) {
      const [attr, key] = pair.split(':');
      if (attr && key) e.setAttribute(attr.trim(), t(key.trim()));
    }
  });
}
