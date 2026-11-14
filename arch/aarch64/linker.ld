ENTRY(_start)

SECTIONS
{
    . = 0x40080000;

    .text : ALIGN(4)
    {
        *(.text .text.*)
    }

    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
        *(.got .got.plt)
    }

    .data : ALIGN(4K)
    {
        *(.data .data.*)
        *(.data.rel.ro .data.rel.ro.*)
    }

    .bss : ALIGN(4K)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(8);
        __bss_end = .;
    }

    /DISCARD/ :
    {
        *(.eh_frame .eh_frame.*)
        *(.comment)
        *(.note .note.*)
        *(.llvm_addrsig)
    }
}
