// Sistemas soportados. `ejs` es el identificador de sistema de EmulatorJS (null = no lo ejecuta EmulatorJS).
export const SYSTEMS = {
  nes:    { name: 'NES',               short: 'NES',  ext: ['nes'],                ejs: 'nes',       color: '#d6342c' },
  snes:   { name: 'SNES',              short: 'SNES', ext: ['sfc', 'smc'],         ejs: 'snes',      color: '#7a62e8' },
  gb:     { name: 'Game Boy / Color',  short: 'GB',   ext: ['gb', 'gbc'],          ejs: 'gb',        color: '#5aa13a' },
  gba:    { name: 'Game Boy Advance',  short: 'GBA',  ext: ['gba'],                ejs: 'gba',       color: '#3b6fd8' },
  n64:    { name: 'Nintendo 64',       short: 'N64',  ext: ['n64', 'z64', 'v64'],  ejs: 'n64',       color: '#e0a21b' },
  nds:    { name: 'Nintendo DS',       short: 'DS',   ext: ['nds'],                ejs: 'nds',       color: '#8a8f99' },
  vb:     { name: 'Virtual Boy',       short: 'VB',   ext: ['vb', 'vboy'],         ejs: 'vb',        color: '#b0151c' },
  md:     { name: 'Mega Drive',        short: 'MD',   ext: ['md', 'gen', 'smd'],   ejs: 'segaMD',    color: '#1f1f1f' },
  sms:    { name: 'Master System',     short: 'SMS',  ext: ['sms'],                ejs: 'segaMS',    color: '#c43a3a' },
  gg:     { name: 'Game Gear',         short: 'GG',   ext: ['gg'],                 ejs: 'segaGG',    color: '#3a3a8c' },
  psx:    { name: 'PlayStation',       short: 'PS1',  ext: ['pbp', 'iso', 'cue', 'chd'], ejs: 'psx', color: '#2c4a9a' },
  pce:    { name: 'PC Engine',         short: 'PCE',  ext: ['pce'],                ejs: 'pce',       color: '#d67a1f' },
  a2600:  { name: 'Atari 2600',        short: '2600', ext: ['a26'],                ejs: 'atari2600', color: '#8a5a2a' },
  a7800:  { name: 'Atari 7800',        short: '7800', ext: ['a78'],                ejs: 'atari7800', color: '#6a4a2a' },
  lynx:   { name: 'Atari Lynx',        short: 'LYNX', ext: ['lnx'],                ejs: 'lynx',      color: '#a34b19' },
  ngp:    { name: 'Neo Geo Pocket',    short: 'NGP',  ext: ['ngp', 'ngc'],         ejs: 'ngp',       color: '#2a7f8f' },
  ws:     { name: 'WonderSwan',        short: 'WS',   ext: ['ws', 'wsc'],          ejs: 'ws',        color: '#4c6ef5' },
  switch: { name: 'Switch (homebrew)', short: 'NSW',  ext: ['nro'],                ejs: null,        color: '#e4002b' },
};

/** Formatos de Switch que Nexus no puede ejecutar (se explica al usuario). */
export const SWITCH_UNSUPPORTED_EXT = ['xci', 'nsp', 'nca', 'nsz', 'xcz'];

/** Extensiones que pueden pertenecer a varias consolas (el usuario elige). `*` = cualquiera. */
export const AMBIGUOUS = {
  zip: '*',
  '7z': '*',
  bin: ['md', 'a2600', 'a7800', 'psx'],
};

export const extOf = (filename) => (filename.split('.').pop() || '').toLowerCase();

/** Devuelve `{ system }`, `{ choices: [claves] }` o `null` si el formato no se reconoce. */
export function detect(filename) {
  const e = extOf(filename);
  const direct = Object.keys(SYSTEMS).filter((k) => SYSTEMS[k].ext.includes(e));
  if (direct.length === 1 && !(e in AMBIGUOUS)) return { system: direct[0] };
  if (e in AMBIGUOUS) {
    const all = AMBIGUOUS[e] === '*' ? Object.keys(SYSTEMS).filter((k) => SYSTEMS[k].ejs) : AMBIGUOUS[e];
    return { choices: all };
  }
  return direct.length > 1 ? { choices: direct } : null;
}

export const titleOf = (filename) =>
  filename.replace(/\.[^.]+$/, '').replace(/_+/g, ' ').replace(/\s*[\[(].*?[\])]/g, '').replace(/\s+/g, ' ').trim() || filename;

export const initialsOf = (title) =>
  title.replace(/[^\p{L}\p{N} ]/gu, '').split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]).join('').toUpperCase() || '?';
