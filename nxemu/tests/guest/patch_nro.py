#!/usr/bin/env python3
"""Rellena la cabecera NRO0 y MOD0 de un módulo PIE a partir de los símbolos del ELF. Uso: patch_nro.py x.elf x.nro"""
import struct, subprocess, sys
elf, nro = sys.argv[1:3]
syms = {}
for line in subprocess.run(["llvm-nm", elf], capture_output=True, text=True, check=True).stdout.splitlines():
    parts = line.split()
    if len(parts) == 3:
        syms[parts[2]] = int(parts[0], 16)
data = bytearray(open(nro, "rb").read())
while len(data) < syms["__bss_start"] - syms["_nro_header"]:
    data.append(0)   # objcopy omite el relleno final de alineación del último segmento
hdr = syms["_nro_header"]; mod0 = syms["_mod0"]
ro_start, data_start, bss_start, bss_end = syms["__ro_start"], syms["__data_start"], syms["__bss_start"], syms["__bss_end"]
def put(off, v): data[off:off + 4] = struct.pack("<I", v & 0xFFFFFFFF)
put(0x18, bss_start - hdr)                    # tamaño de la imagen en archivo
put(0x20, 0); put(0x24, ro_start - hdr)       # .text
put(0x28, ro_start - hdr); put(0x2C, data_start - ro_start)   # .rodata
put(0x30, data_start - hdr); put(0x34, bss_start - data_start)  # .data
put(0x38, bss_end - bss_start)                # .bss
m = mod0 - hdr
put(m + 4, syms["_DYNAMIC"] - mod0)
put(m + 8, bss_start - mod0); put(m + 12, bss_end - mod0)
open(nro, "wb").write(data)
