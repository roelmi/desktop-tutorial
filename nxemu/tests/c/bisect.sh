#!/bin/sh
# Uso: ./bisect.sh prueba.c N [opt]  — compara cada grupo G(n) por separado (ejecución nativa vs nxemu).
# Requiere `cargo build --release --example runbin` (o debug) ya hecho.
set -e
cd "$(dirname "$0")"
src=$1; n=$2; opt=${3:-O2}
for g in $(seq 0 $n); do
  clang -O1 -DNATIVE -DONLY=$((1<<g)) -fwrapv -ffp-contract=off -fno-math-errno "$src" -o /tmp/bis.native -lm
  want=$(/tmp/bis.native)
  clang --target=aarch64-none-elf -$opt -ffreestanding -fno-builtin -nostdlib -ffp-contract=off -fno-math-errno -DONLY=$((1<<g)) \
    -fwrapv -fuse-ld=lld -Wl,-T,link.ld -Wl,--build-id=none "$src" -o /tmp/bis.elf
  llvm-objcopy -O binary /tmp/bis.elf /tmp/bis.bin
  got=$(../../target/debug/examples/runbin /tmp/bis.bin 2>&1 || true)
  [ "$want" = "$got" ] && echo "grupo $g: ok" || echo "grupo $g: FALLA (esperado $want, obtenido $got)"
done
