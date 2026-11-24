# status(7) — Status and limitations

## NAME

status - implemented features, stubs and known gaps

## SYNOPSIS

`Working`; `Stubs and gaps`; `Possible next steps`

## DESCRIPTION

A snapshot of what the current tree does and does not implement. This is meant
to keep the [other pages](README.md) honest when code and expectations differ.

## Working

- Multiboot boot on x86/x86-64 and QEMU `virt` boot on AArch64, with per-arch
  page-table setup.
- Physical page allocation from a bitmap and virtual address-space helpers on
  all three architectures.
- A 16-entry process table with `fork`, `exit`, `wait`, cooperative context
  switching and CPU-time accounting.
- Syscalls `write`, `fork`, `exit`, `wait4`, `times` and `clock`.
- Serial output, VGA text console on x86/x86-64, PS/2 keyboard polling and a
  line-editing echo TTY.
- Timers: PIT-calibrated TSC on x86, architectural counter on AArch64, plus
  CMOS/PL031 wall-clock seconds.

## Stubs and gaps

- **Syscalls are direct calls.** There is no `int 0x80`, `syscall` or `svc`
  entry, no IDT, and no user/kernel privilege separation.
- **`read`, `open`, `close`** return `-1`; no filesystem, file descriptors or
  VFS exist.
- **No real scheduler.** Context switches happen on `yield`/`wait` paths; no
  timer interrupt preempts a process.
- **No separate address spaces for processes.** `AddressSpace::new()` exists,
  but `process::fork()` copies stacks only; all processes run in kernel mode
  with the same page tables. There is no ELF loader or user-mode entry.
- **TTY is echo-only.** No command parsing, history or shell.
- **AArch64 display and keyboard** are stubs: VGA output is a no-op and
  `keyboard::poll()` always returns `None`; input works through the PL011 UART.
- **Small fixed buffers**: 128-byte TTY line, 256-byte boot log, 16-byte
  keyboard queue, 16 processes, 8 CPUs.
- **No synchronization** around several `static mut` structures (VGA cursor,
  process table, PMM bitmap) if SMP is enabled.
- **Error reporting is partial**: several failure paths return `-1` without
  setting `errno`.
- `doc/` previously had no files; the Makefile's original `DOCS` variable only
  matches manual pages named `*.[1-9]`. Markdown pages are now also listed by
  `make docs`.

## Possible next steps

1. Add an interrupt descriptor table (x86) / exception handling (AArch64) and a
   timer IRQ to make scheduling preemptive.
2. Wire `vmm::AddressSpace::new()`/`activate()` into `fork()` and load user
   processes.
3. Implement `read`/`open`/`close` with a minimal device/file abstraction.
4. Turn the echo loop into a basic command shell.

## FILES

- Whole `arch/`, `kernel/`, `mm/`, `drives/`, `lib/` and `process/` trees.

## SEE ALSO

[README](README.md), [posix](posix.md), [syscall](syscall.md), [build](build.md)
