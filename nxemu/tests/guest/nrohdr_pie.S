// Cabecera NRO0 + estructura MOD0 para módulos enlazados en la dirección 0 (PIE). Los campos de tamaño y desplazamiento
// dependen del enlazado y los rellena `patch_nro.py` a partir de los símbolos del ELF.
    .section .nro_header, "ax"
    .global _nro_header
_nro_header:
    b _start
    .word _mod0 - _nro_header          // desplazamiento de MOD0
    .quad 0
    .ascii "NRO0"
    .word 0
    .word 0                            // tamaño de la imagen (lo rellena patch_nro.py)
    .word 0
    .word 0, 0                         // .text: desplazamiento, tamaño
    .word 0, 0                         // .rodata
    .word 0, 0                         // .data
    .word 0                            // tamaño de .bss
    .word 0
    .space 0x80 - (. - _nro_header)
_mod0:
    .ascii "MOD0"
    .word 0                            // desplazamiento de _DYNAMIC respecto a MOD0 (patch_nro.py)
    .word 0, 0, 0, 0, 0
    .balign 16
