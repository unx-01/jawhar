.section .text
.global _start
_start:
    mov x19, x0
    mov x20, x1

    adrp x0, stack_top
    add  x0, x0, :lo12:stack_top
    mov  sp, x0

    mov x0, x19
    mov x1, x20
    bl __init

1:  wfe
    b   1b

.section .bss
.align 16
stack_bottom: .skip 16384
stack_top:
