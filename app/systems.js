// Sistemas soportados. `ejs` es el identificador de sistema de EmulatorJS.
export const SYSTEMS = {
  nes:  { name: 'NES',               ext: ['nes'],               ejs: 'nes',    color: '#d6342c' },
  snes: { name: 'SNES',              ext: ['sfc', 'smc'],        ejs: 'snes',   color: '#7a62e8' },
  gb:   { name: 'Game Boy / Color',  ext: ['gb', 'gbc'],         ejs: 'gb',     color: '#5aa13a' },
  gba:  { name: 'Game Boy Advance',  ext: ['gba'],               ejs: 'gba',    color: '#3b6fd8' },
  n64:  { name: 'Nintendo 64',       ext: ['n64', 'z64', 'v64'], ejs: 'n64',    color: '#e0a21b' },
  nds:  { name: 'Nintendo DS',       ext: ['nds'],               ejs: 'nds',    color: '#8a8f99' },
  md:   { name: 'Mega Drive',        ext: ['md', 'gen', 'smd'],  ejs: 'segaMD', color: '#1f1f1f' },
  psx:  { name: 'PlayStation',       ext: ['chd', 'pbp', 'iso'], ejs: 'psx',    color: '#2c4a9a' },
  sms:  { name: 'Master System',     ext: ['sms'],               ejs: 'segaMS', color: '#c43a3a' },
  gg:   { name: 'Game Gear',         ext: ['gg'],                ejs: 'segaGG', color: '#3a3a8c' },
  pce:  { name: 'PC Engine',         ext: ['pce'],               ejs: 'pce',    color: '#d67a1f' },
  a2600:{ name: 'Atari 2600',        ext: ['a26'],               ejs: 'atari2600', color: '#8a5a2a' },
  switch: { name: 'Switch (homebrew)', ext: ['nro'],             ejs: null,     color: '#e4002b' },
};

export const SWITCH_REMOTE_EXT = ['xci', 'nsp', 'nca'];

export const extOf = (filename) => (filename.split('.').pop() || '').toLowerCase();

export function detect(filename) {
  const e = extOf(filename);
  return Object.keys(SYSTEMS).find((k) => SYSTEMS[k].ext.includes(e)) || null;
}

export const titleOf = (filename) =>
  filename.replace(/\.[^.]+$/, '').replace(/[_]+/g, ' ').replace(/\s*[\[(].*?[\])]/g, '').trim() || filename;
