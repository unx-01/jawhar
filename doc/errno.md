# errno(3) — errno

## NAME

errno - global error code for kernel calls

## SYNOPSIS

`set_errno(i32)`; `errno() -> i32`; `clear_errno()`

## DESCRIPTION

Implementation: `lib/errno.rs`. A single global `AtomicI32` holds the current
error code.

## API

```rust
pub fn set_errno(err: i32);
pub fn errno() -> i32;
pub fn clear_errno();
```

Loads and stores use `Ordering::Relaxed`. The value is process-global, not
thread-local, and there is no pointer-based "errno address" syscall.

## Defined codes

| Constant | Value | Constant | Value |
| --- | --- | --- | --- |
| `EPERM` | 1 | `ENOENT` | 2 |
| `ESRCH` | 3 | `EINTR` | 4 |
| `EIO` | 5 | `ENXIO` | 6 |
| `E2BIG` | 7 | `ENOEXEC` | 8 |
| `EBADF` | 9 | `ECHILD` | 10 |
| `EAGAIN` | 11 | `ENOMEM` | 12 |
| `EACCES` | 13 | `EFAULT` | 14 |
| `EBUSY` | 16 | `EEXIST` | 17 |
| `ENODEV` | 19 | `ENOTDIR` | 20 |
| `EISDIR` | 21 | `EINVAL` | 22 |
| `ENFILE` | 23 | `EMFILE` | 24 |
| `ENOTTY` | 25 | `EFBIG` | 27 |
| `ENOSPC` | 28 | `ESPIPE` | 29 |
| `EROFS` | 30 | `EPIPE` | 32 |
| `ERANGE` | 34 | `ENOSYS` | 38 |

These follow the common POSIX numbering, with gaps (e.g. 15, 18, 26, 31,
33, 35–37) left undefined.

## Current users

| Call | Condition | Code set |
| --- | --- | --- |
| `write` | fd is not 1 or 2 | `EBADF` |
| `write` | null buffer with non-zero count | `EFAULT` |
| `times` | null `Tms` buffer | `EFAULT` |

No code clears `errno` on success, and successful calls do not reset it to 0.

Other syscall failures simply return `-1`: the unimplemented `read`/`open`/
`close` calls, a failed `fork` (no slot/pages), `wait` with no matching child,
and unknown syscall numbers. `ENOSYS` is defined but never set.

## POSIX notes

In POSIX, `errno` is thread-local and only meaningful after a `-1` return.
Here it is one global atomic that can be overwritten by any code path that
calls `set_errno`. See [POSIX-style interfaces](posix.md) and
[System calls](syscall.md).

## EXAMPLES

### `set_errno`, `errno`, `clear_errno` — Rust

```rust
fn checked_fd(fd: i32) -> isize {
    if fd != 1 && fd != 2 {
        crate::errno::set_errno(crate::errno::EBADF);
        return -1;
    }
    0
}

if crate::errno::errno() == crate::errno::EBADF {
    crate::errno::clear_errno();
}
```

### `set_errno` — C

The plain Rust functions have mangled symbols; export a C wrapper first:

```rust
#[no_mangle]
pub extern "C" fn errno_set(value: i32) {
    crate::errno::set_errno(value);
}
```

```c
extern void errno_set(int value);

errno_set(9);   /* EBADF */
```

## FILES

- `lib/errno.rs` — code constants and the global `AtomicI32`.

## SEE ALSO

[syscall](syscall.md), [posix](posix.md), [lib](lib.md)
