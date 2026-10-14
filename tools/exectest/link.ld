/* tools/exectest/link.ld - the execution tests' flat layout
   (tests/exec/README.md): code at 0x10000, then data, then bss.
   _etext: the runner refuses writes below it (--text-end). */
SECTIONS
{
  . = 0x10000;
  .text : { *(.text) *(.text.*) }
  _etext = .;
  . = ALIGN(4);
  .data : { *(.data) *(.data.*) *(.moddata*) }
  . = ALIGN(4);
  .bss : { *(.bss) *(.bss.*) *(COMMON) }
  . = ALIGN(4);
  _end = .;
  /DISCARD/ : { *(.comment) *(.note*) }
}
