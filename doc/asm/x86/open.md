# open(2) — x86 assembly

## NAME

open - invoke the open system call from 32-bit x86 assembly

## SYNOPSIS

`jawhar_syscall(SYS_OPEN, ...)` — call number `3`; `__syscall!` is variadic (0 to 6 arguments).

## DESCRIPTION

Placeholder for opening a path with flags. There is no VFS or file
descriptor table, so the dispatch returns `-1` without setting `errno`. There is
no dedicated `open!` Rust macro; use `syscall!` or `__syscall!`.

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
let path = b"/etc/passwd\0";
let fd = crate::__syscall!(
    crate::syscall::SYS_OPEN, path.as_ptr() as usize, 0, 0, 0, 0,
);   // -1
```

### C

```c
#include <stdint.h>

extern intptr_t jawhar_syscall(uint32_t n, uintptr_t a1, uintptr_t a2,
                               uintptr_t a3, uintptr_t a4, uintptr_t a5);
#define SYS_OPEN 3

const char path[] = "/etc/passwd";
intptr_t fd = jawhar_syscall(SYS_OPEN, (uintptr_t)path, 0, 0, 0, 0);
```

### Assembly (AT&T/GAS, cdecl)

```asm
    .section .rodata
path:
    .asciz "/etc/passwd"

    .text
    pushl $0               # a5
    pushl $0               # a4
    pushl $0               # a3
    pushl $0               # a2 = flags
    pushl $path            # a1 = path
    pushl $3               # n  = SYS_OPEN
    call jawhar_syscall
    addl $24, %esp
    # eax = -1 (no VFS yet)
```

## FILES

- `arch/x86/boot.S`, `arch/x86/linker.ld`.
- `kernel/mod.rs`, `kernel/open.rs`.
- `process/init.rs` — cdecl `global_asm!` stubs.

## SEE ALSO

[x86 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
