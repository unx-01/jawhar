# -----------------------------------------------------------------------------
# x86_64 Multiboot entry, pure assembly (AT&T/GAS)
# يدخل الوضع الطويل 64-بت ثم يتوقف. لا طباعة، لا C.
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
.align 4096
pml4:           .skip 4096
pdpt:           .skip 4096
pd:             .skip 4096
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

    # Zero .bss before using it: the page tables and stack live there.
    movl $__bss_start, %edi
    movl $__bss_end, %ecx
    subl %edi, %ecx
    xorl %eax, %eax
    rep stosb

    movl $stack_top, %esp
    movl %esp, %ebp

    # --- PML4[0] -> PDPT ---
    movl $pdpt, %eax
    orl  $0x03, %eax                # present | writable
    movl %eax, pml4

    # --- PDPT[0] -> PD ---
    movl $pd, %eax
    orl  $0x03, %eax
    movl %eax, pdpt

    # --- PD: identity-map first 1 GiB with 2 MiB pages ---
    movl $pd, %edi
    movl $0x00000083, %eax          # present | writable | PS
    movl $512, %ecx
1:
    movl %eax, (%edi)
    addl $0x200000, %eax
    addl $8, %edi
    loop 1b

    # Load PML4
    movl $pml4, %eax
    movl %eax, %cr3

    # Enable PAE (CR4.PAE)
    movl %cr4, %eax
    orl  $(1 << 5), %eax
    movl %eax, %cr4

    # Set LME in EFER
    movl $0xC0000080, %ecx
    rdmsr
    orl  $(1 << 8), %eax
    wrmsr

    # Enable paging (CR0.PG)
    movl %cr0, %eax
    orl  $(1 << 31), %eax
    movl %eax, %cr0

    # Load 64-bit GDT and far-jump into long mode
    lgdt gdt64_desc
    ljmp $0x08, $long_mode

# --------------------------- 64-bit long mode --------------------------------
.code64
long_mode:
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs

    leaq stack_top(%rip), %rsp
    movq %rsp, %rbp

    # Pass the Multiboot arguments to the kernel.
    movl mb_magic(%rip), %edi     # arg0
    movl mb_info(%rip),  %esi     # arg1
    call __init
    sti

    # لا شيء آخر: توقف نهائي
1:  cli
    hlt
    jmp 1b

# AP long-mode entry: trampoline sets stack and jumps here.
.global ap_start64
ap_start64:
    movw $0x10, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs
    movq ap_stack_top(%rip), %rsp
    xorq %rbp, %rbp
    call ap_entry_rust
1:  cli
    hlt
    jmp 1b

# --------------------------- AP trampoline (copied to low memory) -----------
# Relocatable blob copied by BSP to a low physical page and patched:
#   ap_tr_gdt_desc.base -> copied ap_tr_gdt
#   ap_tr_cr3            -> kernel/root page table physical address
#   ap_tr_stack          -> AP stack top
#   ap_tr_entry64        -> absolute address of ap_start64
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

    # load patched GDT (base must point to copied ap_tr_gdt)
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

    # load CR3 for long mode
    movl (ap_tr_cr3 - ap_trampoline_start), %eax
    movl %eax, %cr3

    # enable PAE
    movl %cr4, %eax
    orl  $(1 << 5), %eax
    movl %eax, %cr4

    # enable LME
    movl $0xC0000080, %ecx
    rdmsr
    orl  $(1 << 8), %eax
    wrmsr

    # enable paging
    movl %cr0, %eax
    orl  $(1 << 31), %eax
    movl %eax, %cr0

    # far jump to 64-bit code segment in copied GDT
    ljmp $0x18, $(ap_tr_long64 - ap_trampoline_start)

.code64
ap_tr_long64:
    movw $0x20, %ax
    movw %ax, %ds
    movw %ax, %es
    movw %ax, %ss
    movw %ax, %fs
    movw %ax, %gs
    movq (ap_tr_stack - ap_trampoline_start), %rsp
    movq (ap_tr_entry64 - ap_trampoline_start), %rax
    jmp *%rax

.align 16
ap_tr_gdt:
    .quad 0x0000000000000000        # 0x00 null
    .quad 0x00CF9A000000FFFF        # 0x08 32-bit code
    .quad 0x00CF92000000FFFF        # 0x10 32-bit data
    .quad 0x00AF9A000000FFFF        # 0x18 64-bit code
    .quad 0x00AF92000000FFFF        # 0x20 64-bit data
ap_tr_gdt_desc:
    .word (ap_tr_gdt_desc - ap_tr_gdt - 1)
    .long 0                          # base patched by BSP to copied ap_tr_gdt

.align 8
ap_tr_cr3:      .quad 0
ap_tr_stack:    .quad 0
ap_tr_entry64:  .quad 0
ap_trampoline_end:

.code64
.section .data
.align 8
.global ap_stack_top
ap_stack_top: .quad 0

# --------------------------- Read-only data ----------------------------------
.section .rodata
.align 8
gdt64:
    .quad 0x0000000000000000        # 0x00 null
    .quad 0x00AF9A000000FFFF        # 0x08 64-bit code (L=1)
    .quad 0x00AF92000000FFFF        # 0x10 64-bit data
gdt64_desc:
    .word gdt64_desc - gdt64 - 1
    .quad gdt64
