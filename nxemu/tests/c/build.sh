#!/bin/sh
# Regenera los fixtures: compila cada prueba para ARM64 (varios niveles de optimización) y calcula
# el resultado esperado ejecutándola de forma nativa. Requiere clang + lld.
set -e
cd "$(dirname "$0")"
rm -f out/*
for src in *.c; do
  name=${src%.c}
  clang -O1 -DNATIVE -fwrapv -ffp-contract=off -fno-math-errno -Wall -Wno-unused-function "$src" -o "out/$name.native" -lm
  ./out/$name.native > "out/$name.expected"
  rm "out/$name.native"
  # Las pruebas de coma flotante (fp*.c) usan los registros v; el resto se limita a enteros.
  case "$name" in fp*) regs=""; vec="-fno-vectorize -fno-slp-vectorize" ;;
    neon*|vec*) regs=""; vec="" ;;
    atomic*) regs="-mgeneral-regs-only -march=armv8.2-a"; vec="-fno-vectorize -fno-slp-vectorize" ;;
    *) regs="-mgeneral-regs-only"; vec="-fno-vectorize -fno-slp-vectorize" ;; esac
  for opt in O0 O1 O2 O3 Os; do
    clang --target=aarch64-none-elf -$opt -ffreestanding -fno-builtin -nostdlib $regs -ffp-contract=off -fno-math-errno \
      $vec -fwrapv -fuse-ld=lld -Wl,-T,link.ld -Wl,--build-id=none \
      -Wno-unused-function "$src" -o "out/$name.$opt.elf"
    llvm-objcopy -O binary "out/$name.$opt.elf" "out/$name.$opt.bin"
    rm "out/$name.$opt.elf"
  done
done
ls out | wc -l
