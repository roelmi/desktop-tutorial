// Sistemas soportados. `ejs` es el identificador de sistema de EmulatorJS.
export const SYSTEMS = {
  nes:  { name: 'NES',               ext: ['nes'],               ejs: 'nes',    color: '#d6342c' },
  snes: { name: 'SNES',              ext: ['sfc', 'smc'],        ejs: 'snes',   color: '#7a62e8' },
  gb:   { name: 'Game Boy / Color',  ext: ['gb', 'gbc'],         ejs: 'gb',     color: '#5aa13a' },
  gba:  { name: 'Game Boy Advance',  ext: ['gba'],               ejs: 'gba',    color: '#3b6fd8' },
  n64:  { name: 'Nintendo 64',       ext: ['n64', 'z64', 'v64'], ejs: 'n64',    color: '#e0a21b' },
  nds:  { name: 'Nintendo DS',       ext: ['nds'],               ejs: 'nds',    color: '#8a8f99' },
  md:   { name: 'Mega Drive',        ext: ['md', 'gen', 'smd'],  ejs: 'segaMD', color: '#1f1f1f' },
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
