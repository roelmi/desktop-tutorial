ENTRY(_start)
SECTIONS {
  . = 0;
  .text   : { KEEP(*(.nro_header)) *(.text.start) *(.text .text.*) . = ALIGN(0x1000); }
  __ro_start = .;
  .rodata : { *(.rodata .rodata.*) }
  .dynsym : { *(.dynsym) }
  .dynstr : { *(.dynstr) }
  .hash   : { *(.hash) }
  .gnu.hash : { *(.gnu.hash) }
  .rela.dyn : { *(.rela.dyn) *(.rela.plt) }
  . = ALIGN(0x1000);
  __data_start = .;
  .data   : { *(.data .data.* .data.rel.ro*) }
  .got    : { *(.got .got.plt) }
  .dynamic : { *(.dynamic) }
  . = ALIGN(0x1000);
  __bss_start = .;
  .bss    : { *(.bss .bss.*) *(COMMON) . = ALIGN(0x1000); }
  __bss_end = .;
  __text_size = __ro_start - 0;
  __ro_off = __ro_start;
  __ro_size = __data_start - __ro_start;
  __data_off = __data_start;
  __data_size = __bss_start - __data_start;
  __bss_size = __bss_end - __bss_start;
  __image_size = __bss_start;
  /DISCARD/ : { *(.comment) *(.note*) *(.eh_frame*) }
}
