// Tiny persistent JSON store: debounced atomic writes, flushed on exit.
import fs from "node:fs";
import path from "node:path";

export function openStore(file, defaults) {
  let data = structuredClone(defaults);
  try {
    const saved = JSON.parse(fs.readFileSync(file, "utf8"));
    data = { ...data, ...saved, settings: { ...defaults.settings, ...(saved.settings || {}) } };
  } catch { /* first run */ }
  let timer = null;
  const flush = () => {
    clearTimeout(timer); timer = null;
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file + ".tmp", JSON.stringify(data));
    fs.renameSync(file + ".tmp", file);
  };
  const save = () => { if (!timer) timer = setTimeout(flush, 300); };
  for (const sig of ["SIGINT", "SIGTERM"]) process.on(sig, () => { flush(); process.exit(0); });
  process.on("exit", () => { if (timer) flush(); });
  return { data, save, flush };
}
