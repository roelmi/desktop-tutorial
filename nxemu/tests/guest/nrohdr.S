// Cabecera NRO0 (0x80 bytes) al principio del segmento .text. Los símbolos los define link.ld.
    .section .nro_header, "ax"
    .global _nro_header
_nro_header:
    b _start
    .word 0
    .quad 0
    .ascii "NRO0"
    .word 0
    .word __image_size
    .word 0
    .word 0, __text_size
    .word __ro_off, __ro_size
    .word __data_off, __data_size
    .word __bss_size
    .word 0
    .space 0x80 - (. - _nro_header)
