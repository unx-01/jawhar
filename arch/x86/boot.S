# -----------------------------------------------------------------------------
# Multiboot x86 (32-bit) entry, pure assembly (AT&T/GAS)
# GRUB enters here already in 32-bit protected mode with A20 on.
# No paging, no long mode, no printing. Just set up and halt.
# -----------------------------------------------------------------------------

# --------------------------- Multiboot header --------------------------------
.set MB_MAGIC,    0x1BADB002
.set MB_FLAGS,    0x00000003        # align modules + memory info
.set MB_CHECKSUM, -(MB_MAGIC + MB_FLAGS)

.section .multiboot, "a"
.align 4
mb_header:
    .long MB_MAGIC
    .long MB_FLAGS
    .long MB_CHECKSUM

# --------------------------- Writable data -----------------------------------
.section .bss
.align 16
.global stack_bottom
.global stack_top
stack_bottom:   .skip 16384
stack_top:

.section .data
.align 4
mb_magic:       .long 0
mb_info:        .long 0

# --------------------------- 32-bit entry ------------------------------------
.section .text
.code32
.global _start
_start:
    cli
    movl %eax, mb_magic
    movl %ebx, mb_info

    # Set up our own GDT (GRUB's may not be what we want)
    lgdt gdt_desc

    # Reload segment registers with our data selector
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %fs
    movw %ax, %gs
    movw %ax, %ss

    # Far-jump to reload CS with our code selector
    ljmp $0x08, $reload_cs

reload_cs:
    # Zero .bss (the stack lives there).
    movl $__bss_start, %edi
    movl $__bss_end, %ecx
    subl %edi, %ecx
    xorl %eax, %eax
    rep stosb

    # Set up stack
    movl $stack_top, %esp
    andl $-16, %esp
    movl %esp, %ebp

    subl $8, %esp
    pushl mb_info
    pushl mb_magic
    call __init
    addl $16, %esp
    sti

    # Done: halt forever
1:  cli
    hlt
    jmp 1b

# AP 32-bit entry after trampoline enables protected mode and paging (if used).
.global ap_start32
ap_start32:
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs
    movl ap_stack_top, %esp
    andl $-16, %esp
    xorl %ebp, %ebp
    call ap_entry_rust
1:  cli
    hlt
    jmp 1b

# --------------------------- AP trampoline (copied to low memory) -----------
.section .rodata
.align 4096
.global ap_trampoline_start
.global ap_trampoline_end
.code16
ap_trampoline_start:
    cli
    xorw %ax, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss

    lgdtl ap_tr_gdt_desc - ap_trampoline_start
    movl %cr0, %eax
    orl  $1, %eax
    movl %eax, %cr0
    ljmpl $0x08, $(ap_tr_prot32 - ap_trampoline_start)

.code32
ap_tr_prot32:
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs

    # optional CR3 load (if paging enabled by BSP)
    movl (ap_tr_cr3 - ap_trampoline_start), %eax
    testl %eax, %eax
    jz 1f
    movl %eax, %cr3
    movl %cr0, %eax
    orl  $(1 << 31), %eax
    movl %eax, %cr0
1:
    movl (ap_tr_stack - ap_trampoline_start), %esp
    movl (ap_tr_entry32 - ap_trampoline_start), %eax
    jmp *%eax

.align 16
ap_tr_gdt:
    .quad 0x0000000000000000
    .word 0xFFFF, 0x0000
    .byte 0x00, 0x9A, 0xCF, 0x00
    .word 0xFFFF, 0x0000
    .byte 0x00, 0x92, 0xCF, 0x00
ap_tr_gdt_desc:
    .word (ap_tr_gdt_desc - ap_tr_gdt - 1)
    .long 0

.align 4
ap_tr_cr3:      .long 0
ap_tr_stack:    .long 0
ap_tr_entry32:  .long 0
ap_trampoline_end:

.section .data
.align 4
.global ap_stack_top
ap_stack_top: .long 0

# --------------------------- GDT (32-bit) ------------------------------------
.section .rodata
.align 8
gdt:
    .quad 0x0000000000000000        # 0x00 null

    # 0x08: 32-bit code, ring 0
    # base=0, limit=0xFFFFF, access=0x9A, flags=0xCF (4K gran, 32-bit)
    .word 0xFFFF
    .word 0x0000
    .byte 0x00
    .byte 0x9A
    .byte 0xCF
    .byte 0x00

    # 0x10: 32-bit data, ring 0
    .word 0xFFFF
    .word 0x0000
    .byte 0x00
    .byte 0x92
    .byte 0xCF
    .byte 0x00

gdt_desc:
    .word gdt_desc - gdt - 1
    .long gdt
