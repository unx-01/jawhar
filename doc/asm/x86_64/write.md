# write(2) — x86_64 assembly

## NAME

write - invoke the write system call from x86-64 assembly

## SYNOPSIS

`jawhar_syscall(SYS_WRITE, ...)` — call number `1`; `__syscall!` is variadic (0 to 6 arguments).

## DESCRIPTION

Writes `count` bytes from `buf` to `fd`. Only `1` (`stdout`) and
`2` (`stderr`) are valid. Returns the byte count, or `-1` with `EBADF` for a bad
descriptor and `EFAULT` for a null buffer with a non-zero count.

The Rust convenience call is listed first. C and assembly reach the same
dispatch through an exported wrapper, because the Rust symbol is mangled:

```rust
#[no_mangle]
pub extern "C" fn jawhar_syscall(
    n: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize,
) -> isize {
    crate::__syscall!(n, a1, a2, a3, a4, a5)
}
```

## EXAMPLES

### Rust

```rust
let msg = b"hello from jawhar\n";
let n = crate::write!(1, msg, msg.len());   // n == msg.len()
```

### C

```c
#include <stdint.h>

extern intptr_t jawhar_syscall(uint32_t n, uintptr_t a1, uintptr_t a2,
                               uintptr_t a3, uintptr_t a4, uintptr_t a5);
#define SYS_WRITE 1

const char msg[] = "hello from jawhar\n";
intptr_t n = jawhar_syscall(SYS_WRITE, 1, (uintptr_t)msg,
                            sizeof(msg) - 1, 0, 0);
```

### Assembly (AT&T/GAS, System V)

```asm
    .section .rodata
msg:
    .ascii "hello from jawhar\n"
    .set msg_len, . - msg

    .text
    subq $8, %rsp          # after a call RSP is 8 mod 16; align it
    leaq msg(%rip), %rdx   # a2 = buffer
    movq $msg_len, %rcx    # a3 = count
    movl $1, %esi          # a1 = stdout
    movl $1, %edi          # n  = SYS_WRITE
    xorl %r8d, %r8d        # a4
    xorl %r9d, %r9d        # a5
    call jawhar_syscall
    addq $8, %rsp
    # rax = bytes written, or -1
```

## FILES

- `arch/x86-64/boot.S`, `arch/x86-64/linker.ld`.
- `kernel/mod.rs`, `kernel/write.rs`.
- `process/init.rs` — SysV `global_asm!` stubs.

## SEE ALSO

[x86_64 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
