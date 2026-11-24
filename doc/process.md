# process(9) — Processes and scheduling

## NAME

process - process table, fork, context switching and accounting

## SYNOPSIS

`fork() -> i32`; `exit(status) -> !`; `wait(pid, status) -> i32`

## DESCRIPTION

Implementation: `process/init.rs`, with syscall wrappers in `kernel/fork.rs`,
`kernel/exit.rs`, `kernel/wait.rs`, `kernel/times.rs` and `kernel/clock.rs`.

## Limits and layout

| Constant | Value | Meaning |
| --- | --- | --- |
| `MAX_PROC` | 16 | Size of the static process table. |
| `STACK_PAGES` | 16 | Pages allocated for a forked child stack. |
| `STACK_SIZE` | 64 KiB | `16 × 4096` bytes. |
| `HZ` | 100 | Tick rate used for accounting (10 ms per tick). |

`Process` records: PID, parent PID, state, saved context (stack pointer), stack
bounds, whether the stack was dynamically allocated, exit status, and four CPU
time counters (`utime`, `stime`, `cutime`, `cstime`) plus `last_tick`.

## States

```
Unused -> Ready -> Running -> Zombie -> Unused
```

- `init()` fills the table with `Unused` entries and installs the boot thread
  as PID 1, `Running`, using the linker-provided `stack_bottom..stack_top`
  (`stack_owned = false`). `NEXT_PID` starts at 2.
- `Ready` means runnable but not current. `Zombie` means exited and waiting to
  be reaped by `wait`.

## `fork()`

`process::fork()` (syscall `SYS_FORK`) does the following:

1. Accounts CPU time for the current process.
2. Finds a free table slot and allocates `STACK_PAGES` physical pages with
   `pmm::alloc_pages`.
3. Initializes the child with a new PID, the parent's PID, `Ready` state and
   the new stack bounds (`stack_owned = true`).
4. Calls the assembly routine `fork_save(parent_context, child_context,
   old_stack_low, old_stack_high, new_stack_low, new_stack_high)`.
5. `fork_save` pushes callee-saved registers, saves the current `SP` into the
   parent context and calls `fork_copy`.

`fork_copy` copies the *used* portion of the parent stack (`old_high - saved_sp`)
to the top of the child stack and rewrites pointer-shaped words that lie inside
the old stack range so they point at the corresponding location in the new
stack. It returns `1` on success and `0` if the ranges are invalid or the used
stack does not fit.

Return values:

- Parent: child PID, or `-1` if no slot/pages are available or the copy fails
  (in which case the slot and pages are released).
- Child: `0`, detected by checking whether the current process index changed
  after `fork_save` returned.

There are assembly implementations of `fork_save` and `context_switch` for all
three architectures in `process/init.rs` (`global_asm!` blocks). The callee-
saved registers are RBX/RBP/R12–R15 (x86-64), EBX/EBP/ESI/EDI (x86) and
X19–X30 (AArch64).

## Context switching and scheduling

`context_switch(old, new)` saves the current stack pointer into `old` and loads
the stack pointer from `new`, then pops the saved registers and returns into the
other process's call stack.

Scheduling is cooperative and round-robin over table slots:

- `pick_next(start)` scans forward modulo `MAX_PROC` for a `Ready` or `Running`
  entry.
- `switch_to_next()` accounts time, marks the old process `Ready` and the new
  one `Running`, updates `CURRENT` and calls `context_switch`.
- `yield_now()` is the public wrapper. It is called by `wait()` while blocking;
  there is no timer interrupt that forces a switch.

`account_current()` adds elapsed ticks since `last_tick` to `utime`.
`times(buf)` copies the four counters into a `Tms` and returns the current tick
value. `clock()` returns `utime + stime`.

## `exit()` and `wait()`

`exit(status)`:

1. Accounts time and reparents any children of the exiting process to PID 1.
2. Marks the process `Zombie` and stores the exit status.
3. Picks the next runnable process and switches to it. If none exists, the CPU
   halts forever.

`wait(pid, status)`:

1. Scans for children of the current process matching `pid` (`-1` matches all).
2. When a matching child is a zombie, adds its user/system times to the
   parent's `cutime`/`cstime`, clears the slot, frees its stack, writes the exit
   status through `status` if non-null and returns the child PID.
3. If matching children exist but none is a zombie, yields with
   `switch_to_next()` and scans again.
4. Returns `-1` if there is no matching child.

## Current scope

`fork()` copies only stacks; it does not create a new virtual address space via
`vmm::AddressSpace::new()`, although that API exists in
[memory management](memory.md). The whole kernel shares one address space and
one privilege level, and there is no ELF loader or user-mode entry point.

## EXAMPLES

### `fork`, `exit`, `wait` — Rust

```rust
let pid = crate::process::fork();
if pid == 0 {
    crate::write!(1, b"child here\n", 11);
    crate::process::exit(0);       // never returns
}

let mut status = 0i32;
let reaped = crate::process::wait(pid, &mut status);
```

### `fork_save` — C

`fork_save` is declared `extern "C"`, so its symbol can be called directly
from C. Arguments: parent context, child context, old stack bounds, new stack
bounds.

```c
#include <stdint.h>

typedef struct Context Context;   /* defined in process/init.rs */
extern int fork_save(Context *parent, Context *child,
                     uintptr_t old_lo, uintptr_t old_hi,
                     uintptr_t new_lo, uintptr_t new_hi);

int copied = fork_save(&parent_ctx, &child_ctx,
                       0x100000, 0x104000, 0x200000, 0x204000);
/* copied == 1: stack copied, 0: no slot or no pages */
```

### `context_switch` — C

```c
extern void context_switch(Context *old, Context *new);

context_switch(&old_ctx, &new_ctx);   /* returns on the new stack */
```

### `yield_now`, `times`, `clock` — Rust

```rust
crate::process::yield_now();      // cooperative round-robin step

let mut t = crate::time::Tms::default();
let now = crate::process::times(&mut t);
let cpu_ticks = crate::process::clock();
```

### `yield_now` equivalent — C

```c
extern void process_yield_now(void);   /* Rust wrapper around yield_now() */

process_yield_now();                   /* switch to the next runnable slot */
```

## FILES

- `process/init.rs` — table, fork/stack copy, context switch and syscalls.
- `kernel/fork.rs`, `kernel/exit.rs`, `kernel/wait.rs`, `kernel/times.rs`.

## SEE ALSO

[syscall](syscall.md), [memory](memory.md), [time](time.md), [smp](smp.md)
