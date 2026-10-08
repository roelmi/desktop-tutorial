#!/bin/sh
# Regenera los fixtures: compila cada prueba para ARM64 (varios niveles de optimización) y calcula
# el resultado esperado ejecutándola de forma nativa. Requiere clang + lld.
set -e
cd "$(dirname "$0")"
rm -f out/*
for src in *.c; do
  name=${src%.c}
  clang -O1 -DNATIVE -fwrapv -Wall -Wno-unused-function "$src" -o "out/$name.native"
  ./out/$name.native > "out/$name.expected"
  rm "out/$name.native"
  for opt in O0 O1 O2 O3 Os; do
    clang --target=aarch64-none-elf -$opt -ffreestanding -fno-builtin -nostdlib -mgeneral-regs-only \
      -fno-vectorize -fno-slp-vectorize -fwrapv -fuse-ld=lld -Wl,-T,link.ld -Wl,--build-id=none \
      -Wno-unused-function "$src" -o "out/$name.$opt.elf"
    llvm-objcopy -O binary "out/$name.$opt.elf" "out/$name.$opt.bin"
    rm "out/$name.$opt.elf"
  done
done
ls out | wc -l
