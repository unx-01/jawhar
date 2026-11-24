# posix(7) — POSIX-style interfaces

## NAME

posix - Unix conventions used by jawhar and the gaps that remain

## SYNOPSIS

`write`, `read`, `fork`, `wait4`, `times`, `clock`, `errno`

## DESCRIPTION

jawhar borrows names and conventions from POSIX/Unix, but it is **not** a POSIX
implementation and has no C library or binary-compatible syscall ABI.

## What is POSIX-like

- System call names and constants: `write`, `read`, `open`, `close`, `fork`,
  `exit`, `wait4`, `times`, `clock`.
- Standard file descriptors: `STDOUT_FILENO = 1`, `STDERR_FILENO = 2`.
- `errno` constants with familiar numbering, plus `set_errno`/`errno`.
- `clock_t`-style type `ClockT` and `struct tms`-style `Tms` with
  `CLK_TCK = CLOCKS_PER_SEC = HZ = 100`.
- `fork()` returning child PID / `0` / `-1`, zombie children and `wait4`
  reaping, with child CPU time accumulated into `tms_cutime`/`tms_cstime`.
- Error returns of `-1`.

See [System calls](syscall.md), [Processes and scheduling](process.md),
[Time and clocks](time.md) and [errno](errno.md).

## Interface map

| POSIX API | jawhar equivalent | Status |
| --- | --- | --- |
| `write(fd, buf, nbyte)` | `SYS_WRITE` / `write!` | fd 1 or 2 only, byte-wise to serial + VGA. |
| `read(fd, buf, nbyte)` | `SYS_READ` / `read!` | Stub: returns `-1`. |
| `open`, `close` | `SYS_OPEN`, `SYS_CLOSE` | Stubs: return `-1`. |
| `fork()` | `SYS_FORK` / `fork!` | Copies stacks only; no new address space or privilege level. |
| `_exit(status)` | `SYS_EXIT` / `exit!` | Marks zombie and switches away; status narrowed to `u16`. |
| `wait4(pid, ...)` | `SYS_WAIT4` / `wait!` | Two arguments (pid, status pointer); no `rusage`, no options. |
| `times(struct tms *)` | `SYS_TIMES` / `times!` | Fills four counters, returns tick count. |
| `clock()` | `SYS_CLOCK` / `clock!` | Returns `utime + stime` in ticks, not CPU seconds. |
| `getpid`, `getppid` | — | Not implemented (values exist only inside the process table). |
| `execve`, `waitpid`, `kill`, `signal` | — | Not implemented. |

## Deliberate differences

1. **Call mechanism.** Syscalls are direct Rust function calls
   (`__syscall!(number, ...)`), not `int 0x80`, `syscall` or `svc` instructions.
   Call numbers are project-defined and do not match Linux or POSIX ABI tables.
2. **No user space.** All code runs in kernel mode with one shared address
   space. `fork()` does not activate a new `AddressSpace`, and there is no ELF
   loader.
3. **errno.** One global `AtomicI32` rather than a thread-local variable; many
   errors do not set it at all.
4. **Limited descriptors.** No VFS, inodes, pipes, sockets, device files or
   fd table. `read`/`open`/`close` are placeholders and the TTY is an echo loop,
   not a terminal driver with termios.
5. **No signals or job control.** `EINTR`, process groups, sessions and signal
   masks are only names/constants, not behavior.
6. **Cooperative scheduling.** No preemptive timer interrupt; a process runs
   until it yields, blocks in `wait`, exits, or the TTY loop polls input.
7. **Time units.** `CLK_TCK`/`HZ` are 100, but `clock()` returns ticks rather
   than the POSIX `CLOCKS_PER_SEC`-scaled value.

## Toward POSIX compatibility

Natural next steps are a real syscall entry with privilege transition, a
per-process address space and user-mode entry, a minimal fd/device layer,
thread-local errno, and standard call numbering. See
[Status and limitations](status.md).

## EXAMPLES

### POSIX-style `write` — Rust

```rust
let msg = b"ok\n";
let ret = crate::write!(1, msg, msg.len());   // 3 on success, -1 on error
```

### Equivalent direct dispatch — C

```c
/* Wrapper declared in syscall(2). */
#include <stdint.h>
extern intptr_t jawhar_syscall(uint32_t n, uintptr_t, uintptr_t,
                               uintptr_t, uintptr_t, uintptr_t);

const char msg[] = "ok\n";
intptr_t ret = jawhar_syscall(1, (uintptr_t)msg, sizeof(msg) - 1, 0, 0, 0);
```

## FILES

- `kernel/mod.rs` — syscall numbers and dispatch.
- `lib/errno.rs`, `lib/time.rs` — errno values and `Tms`.

## SEE ALSO

[syscall](syscall.md), [errno](errno.md), [process](process.md), [status](status.md)
