#!/bin/sh
# Compila los programas de prueba del kernel a .nro (cabecera + segmentos con permisos).
set -e
cd "$(dirname "$0")"
mkdir -p out
clang --target=aarch64-none-elf -c nrohdr.S -o out/nrohdr.o
for src in *.c; do
  name=${src%.c}
  clang --target=aarch64-none-elf -O1 -ffreestanding -fno-builtin -nostdlib -mgeneral-regs-only -fno-vectorize -fno-slp-vectorize \
    -fwrapv -fuse-ld=lld -Wl,-T,link.ld -Wl,--build-id=none -Wno-unused-function "$src" out/nrohdr.o -o "out/$name.elf"
  llvm-objcopy -O binary "out/$name.elf" "out/$name.nro"
  rm "out/$name.elf"
done
ls out/*.nro
