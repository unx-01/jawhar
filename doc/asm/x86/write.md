# write(2) — x86 assembly

## NAME

write - invoke the write system call from 32-bit x86 assembly

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

### Assembly (AT&T/GAS, cdecl)

```asm
    .section .rodata
msg:
    .ascii "hello from jawhar\n"
    .set msg_len, . - msg

    .text
    pushl $0               # a5
    pushl $0               # a4
    pushl $msg_len         # a3 = count
    pushl $msg             # a2 = buffer
    pushl $1               # a1 = stdout
    pushl $1               # n  = SYS_WRITE
    call jawhar_syscall
    addl $24, %esp
    # eax = bytes written, or -1
```

## FILES

- `arch/x86/boot.S`, `arch/x86/linker.ld`.
- `kernel/mod.rs`, `kernel/write.rs`.
- `process/init.rs` — cdecl `global_asm!` stubs.

## SEE ALSO

[x86 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
