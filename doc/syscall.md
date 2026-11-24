# syscall(2) — System calls

## NAME

syscall - kernel system call table and dispatch

## SYNOPSIS

`__syscall!(number, ...) -> isize`

## DESCRIPTION

The syscall layer lives in `kernel/`. Numbers are defined in `kernel/mod.rs` and
dispatched by `kernel::syscall::__syscall_raw`, behind the variadic
`__syscall!` macro.

> Important: these are direct Rust function calls, not a hardware trap. There is
> no `int 0x80`, `syscall` instruction or `svc` instruction, and no user/kernel
> privilege boundary yet.

## Call numbers

| Number | Name | Handler | Status |
| --- | --- | --- | --- |
| 0 | `SYS_SYSCALL` | — | Returns `-1` |
| 1 | `SYS_WRITE` | `write::do_write` | Implemented |
| 2 | `SYS_READ` | `read::do_read` | Stub, returns `-1` |
| 3 | `SYS_OPEN` | — | Stub, returns `-1` |
| 4 | `SYS_CLOSE` | — | Stub, returns `-1` |
| 6 | `SYS_FORK` | `fork::do_fork` | Implemented |
| 7 | `SYS_EXIT` | `exit::do_exit` | Implemented |
| 9 | `SYS_WAIT4` | `wait::do_wait` | Implemented |
| 10 | `SYS_TIMES` | `times::do_times` | Implemented |
| 11 | `SYS_CLOCK` | `clock::do_clock` | Implemented |

5, 8 and 12+ are unassigned; any unknown number returns `-1`.

## Dispatch signature

```rust
__syscall!(number, ...) -> isize
```

`__syscall!` is a variadic macro, like Linux's `syscall(long number, ...)`.
Stable Rust cannot define a C-variadic `fn`, so the macro accepts 0 to 6
arguments, casts each to `usize`, pads the missing ones with zero and forwards
to the fixed-arity dispatcher:

```rust
pub fn __syscall_raw(
    number: u32,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    _arg4: usize,
    _arg5: usize,
) -> isize
```

Arguments 4 and 5 are currently ignored by every handler. `syscall!` is an
alias for `__syscall!`.

Each subsystem also exports a convenience macro (`write!`, `read!`, `fork!`,
`exit!`, `wait!`, `times!`, `clock!`) that calls its `do_*` function directly.

## `write(fd, buf, count)`

`SYS_WRITE`, macro `write!(fd, buf, count)`.

- Only `STDOUT_FILENO = 1` and `STDERR_FILENO = 2` are accepted; both behave
  identically. Other descriptors set `EBADF` and return `-1`.
- `count == 0` returns `0` without touching the buffer.
- A null buffer sets `EFAULT` and returns `-1`. There is no bounds check on
  `buf + count`.
- Otherwise each byte goes through `io::put_byte`, i.e. to the serial UART and
  the VGA console. Returns `count`.

`printk` uses `write!(stdout, ...)` internally, so kernel messages take this
same path.

## `read`, `open`, `close`

`SYS_READ`, `SYS_OPEN` and `SYS_CLOSE` are placeholders. `do_read` always
returns `-1` and does not set `errno`. Input is currently obtained only through
the console/TTY layer's polling loop, not through a file descriptor.

## `fork()`

`SYS_FORK`, macro `fork!()`. Returns the child PID in the parent, `0` in the
child, and `-1` when no process slot or stack pages are available. See
[Processes and scheduling](process.md) for the implementation details.

## `exit(status)`

`SYS_EXIT`, macro `exit!(status)`. `status` is cast to `u16` by the macro, then
to `i32` by `do_exit`. It marks the current process a zombie and switches to
another runnable process; it never returns.

## `wait4(pid, status)`

`SYS_WAIT4`, macro `wait!(pid, status)`.

- `pid == -1` waits for any child; a positive PID waits for that child.
- If a matching child is a zombie, it is reaped, its exit status is written to
  `status` (when non-null), its stack is freed and its PID is returned.
- If matching children exist but none has exited, the caller yields repeatedly
  until one does. If no matching child exists, `-1` is returned.
- No `errno` value is set on the `-1` path.

## `times(buf)` and `clock()`

`SYS_TIMES` fills a `time::Tms` with user, system, children-user and
children-system ticks and returns the current tick count. A null buffer sets
`EFAULT` and returns `-1`.

`SYS_CLOCK` returns `utime + stime` for the current process. Tick unit is
`HZ = 100`, i.e. 10 ms; see [Time and clocks](time.md).

## Error reporting

`lib/errno.rs` defines common Linux-style positive error codes and a single
global `AtomicI32`. `set_errno`, `errno` and `clear_errno` are available, but
only `write` and `times` currently set it. Most failures are reported simply as
`-1`. See [errno](errno.md) for the code table.

## EXAMPLES

The C examples call a small exported wrapper, because `__syscall_raw` is a
plain Rust function with a mangled symbol:

```rust
#[no_mangle]
pub extern "C" fn jawhar_syscall(
    n: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize,
) -> isize {
    crate::__syscall!(n, a1, a2, a3, a4, a5)
}
```

The matching C declarations (freestanding C, `<stdint.h>`) are:

```c
#include <stdint.h>

#define SYS_WRITE 1
#define SYS_FORK  6
#define SYS_WAIT4 9
#define SYS_TIMES 10
#define SYS_CLOCK 11

extern intptr_t jawhar_syscall(uint32_t n, uintptr_t a1, uintptr_t a2,
                               uintptr_t a3, uintptr_t a4, uintptr_t a5);
```

### `write` — Rust

```rust
let msg = b"hello from jawhar\n";
let n = crate::write!(1, msg, msg.len());   // n == msg.len()
```

### `write` — C

```c
const char msg[] = "hello from jawhar\n";
intptr_t n = jawhar_syscall(SYS_WRITE, 1, (uintptr_t)msg,
                            sizeof(msg) - 1, 0, 0);
```

### `fork` — Rust

```rust
let pid = crate::fork!();
if pid == 0 {
    crate::write!(1, b"child\n", 6);
    crate::exit!(0);
} else if pid > 0 {
    let mut status = 0;
    crate::wait!(pid, &mut status);
}
```

### `fork` — C

```c
intptr_t pid = jawhar_syscall(SYS_FORK, 0, 0, 0, 0, 0);
if (pid == 0) {
    const char msg[] = "child\n";
    jawhar_syscall(SYS_WRITE, 1, (uintptr_t)msg, sizeof(msg) - 1, 0, 0);
} else if (pid > 0) {
    int status = 0;
    jawhar_syscall(SYS_WAIT4, (uintptr_t)pid, (uintptr_t)&status, 0, 0, 0);
}
```

### `times` / `clock` — Rust

```rust
let mut t = crate::time::Tms::default();
let now = crate::times!(&mut t);  // tick count
let cpu = crate::clock!();        // utime + stime
```

### `times` / `clock` — C

```c
typedef long clock_t;
typedef struct {
    clock_t tms_utime, tms_stime, tms_cutime, tms_cstime;
} Tms;

Tms t = {0};
intptr_t now = jawhar_syscall(SYS_TIMES, (uintptr_t)&t, 0, 0, 0, 0);
intptr_t cpu = jawhar_syscall(SYS_CLOCK, 0, 0, 0, 0, 0);
```

## FILES

- `kernel/mod.rs` — numbers and the fixed-arity dispatcher.
- `kernel/syscall.rs` — the variadic `__syscall!` macro.
- `kernel/syscall.rs`, `kernel/write.rs`, `kernel/fork.rs`, `kernel/exit.rs`,
  `kernel/wait.rs`, `kernel/times.rs`, `kernel/clock.rs`, `kernel/read.rs`.

## SEE ALSO

[errno](errno.md), [process](process.md), [time](time.md), [posix](posix.md)
