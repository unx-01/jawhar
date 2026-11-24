# lib(3) — Support library

## NAME

lib - spinlocks, errno, printk and memory helpers

## SYNOPSIS

`SpinLock::new(value)`; `printk(msg)`; `memcpy`, `memset`, `memmove`, `memcmp`

## DESCRIPTION

Small free-standing helpers live in `lib/`.

## Spinlocks (`lib/spinlock.rs`)

`SpinLock<T>` combines an `AtomicBool` flag with an `UnsafeCell<T>`. It is
`Send`/`Sync` when `T: Send`.

- `SpinLock::new(value)` — const constructor.
- `lock()` — spins with `compare_exchange_weak(false, true, Acquire, Relaxed)`;
  while the flag is set it calls `cpu_relax()`.
- `try_lock()` — returns `None` if the lock is already held.
- The returned `SpinLockGuard` derefs to `T` (and `DerefMut` for `&mut` access)
  and releases the lock with a `Release` store in `Drop`.

`cpu_relax()` maps to `pause` on x86/x86-64 and `yield` on AArch64.

The lock does not disable interrupts and has no recursion protection. It is
used by `arch/time.rs` for tick/millisecond/microsecond state.

## errno (`lib/errno.rs`)

Defines common positive error numbers (EPERM 1 through ENOSYS 38, with the
usual gaps) and one process-global `AtomicI32`:

- `set_errno(err)`, `errno()`, `clear_errno()`.

Only `write` (`EBADF`, `EFAULT`) and `times` (`EFAULT`) currently use it; other
syscalls return `-1` without setting a code. See [errno](errno.md) for the full
code list and [System calls](syscall.md) for usage.

## printk (`lib/printk.rs`)

```rust
pub fn printk(msg: &str) {
    crate::write!(stdout, msg, msg.len());
}
```

It routes through `SYS_WRITE` to stdout (fd 1), which ends up in
`io::put_byte` → serial + console. The boot log is printed with this function.

## Memory helpers (`mm/mem.rs`)

Exported C-ABI functions:

| Function | Behavior |
| --- | --- |
| `memcpy(dst, src, n)` | Byte-wise forward copy; returns `dst`. |
| `memset(dst, c, n)` | Fill with the low byte of `c`; returns `dst`. |
| `memmove(dst, src, n)` | Copies forward when `dst <= src`, backwards otherwise; returns `dst`. |
| `memcmp(a, b, n)` | Byte-wise comparison returning the first difference or `0`. |

They are kept in the linked image by a reference in `arch/init.rs`, since the
linker uses `--gc-sections`.

## Time types (`lib/time.rs`)

`ClockT`, `Tms`, `HZ`, `CLK_TCK` and `CLOCKS_PER_SEC` are described in
[Time and clocks](time.md).

## EXAMPLES

### `SpinLock`, `errno`, `printk`, `memcpy` — Rust

```rust
static COUNTER: crate::spinlock::SpinLock<usize> =
    crate::spinlock::SpinLock::new(0);

{
    let mut guard = COUNTER.lock();
    *guard += 1;
}                                  // guard drops, lock released

crate::errno::set_errno(crate::errno::EINVAL);
crate::printk("kernel message\n");

let mut dst = [0u8; 4];
unsafe {
    crate::mem::memcpy(dst.as_mut_ptr().cast(), b"abc\0".as_ptr().cast(), 4);
}
```

### `cpu_relax` — C

```c
/* Exported from Rust with #[no_mangle] extern "C". */
extern void cpu_relax(void);   /* PAUSE on x86, WFE hint on AArch64 */

while (lock_is_held()) {
    cpu_relax();
}
```

### `memcpy` — C

```c
#include <string.h>   /* provided by lib/mem.rs in this kernel */

unsigned char dst[4];
memcpy(dst, "abc", 4);
```

## FILES

- `lib/spinlock.rs`, `lib/errno.rs`, `lib/printk.rs`, `lib/time.rs`.
- `mm/mem.rs` — exported C memory helpers.

## SEE ALSO

[errno](errno.md), [time](time.md), [memory](memory.md), [syscall](syscall.md)
