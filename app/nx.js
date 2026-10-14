// Núcleo de Switch (nxemu compilado a WebAssembly): ejecuta homebrew .nro de prueba.
// NO ejecuta juegos comerciales: faltan casi todos los servicios de Horizon, la GPU y el audio.
let inst;

async function load() {
  if (inst) return inst;
  const { instance } = await WebAssembly.instantiateStreaming(fetch('emu/nxemu.wasm'));
  inst = instance.exports;
  return inst;
}

const text = (x, ptr, len) => new TextDecoder().decode(new Uint8Array(x.memory.buffer, ptr, len));

/** Ejecuta un .nro y devuelve { code, output, error }. */
export async function runNro(bytes, maxSvcs = 1_000_000) {
  const x = await load();
  const ptr = x.nx_alloc(bytes.length);
  new Uint8Array(x.memory.buffer, ptr, bytes.length).set(bytes);
  const code = Number(x.nx_run(ptr, bytes.length, maxSvcs));
  x.nx_free(ptr, bytes.length);
  return {
    code,
    output: text(x, x.nx_output_ptr(), x.nx_output_len()),
    error: text(x, x.nx_error_ptr(), x.nx_error_len()),
  };
}
