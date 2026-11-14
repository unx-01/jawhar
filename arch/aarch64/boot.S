.section .text
.global _start
_start:
    mov x19, x0
    mov x20, x1

    adrp x2, __bss_start
    add  x2, x2, :lo12:__bss_start
    adrp x3, __bss_end
    add  x3, x3, :lo12:__bss_end
1:
    cmp  x2, x3
    b.hs 2f
    str  xzr, [x2], #8
    b    1b
2:
    adrp x0, stack_top
    add  x0, x0, :lo12:stack_top
    mov  sp, x0

    mov x0, x19
    mov x1, x20
    bl __init

3:  wfe
    b   3b

.section .bss
.align 16
stack_bottom: .skip 16384
stack_top:
