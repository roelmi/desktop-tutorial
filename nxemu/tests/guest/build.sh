#!/bin/sh
# Compila los programas de prueba del kernel a .nro (cabecera + segmentos con permisos).
set -e
cd "$(dirname "$0")"
mkdir -p out
clang --target=aarch64-none-elf -c nrohdr.S -o out/nrohdr.o
clang --target=aarch64-none-elf -c nrohdr_pie.S -o out/nrohdr_pie.o
for src in *.c; do
  name=${src%.c}
  # reloc*.c se enlaza como ejecutable independiente de la posición (PIE) en la dirección 0 y necesita reubicación
  case "$name" in
    reloc*) clang --target=aarch64-none-elf -O1 -fPIE -ffreestanding -fno-builtin -nostdlib -mgeneral-regs-only -fno-vectorize -fno-slp-vectorize \
              -fwrapv -fuse-ld=lld -Wl,-pie -Wl,-z,notext -Wl,--no-dynamic-linker -Wl,-T,link_pie.ld -Wl,--build-id=none -Wno-unused-function \
              "$src" out/nrohdr_pie.o -o "out/$name.elf"
            llvm-objcopy -O binary "out/$name.elf" "out/$name.nro"; python3 patch_nro.py "out/$name.elf" "out/$name.nro"; rm "out/$name.elf"; continue ;;
  esac
  clang --target=aarch64-none-elf -O1 -ffreestanding -fno-builtin -nostdlib -mgeneral-regs-only -fno-vectorize -fno-slp-vectorize \
    -fwrapv -fuse-ld=lld -Wl,-T,link.ld -Wl,--build-id=none -Wno-unused-function "$src" out/nrohdr.o -o "out/$name.elf"
  llvm-objcopy -O binary "out/$name.elf" "out/$name.nro"
  rm "out/$name.elf"
done
ls out/*.nro
