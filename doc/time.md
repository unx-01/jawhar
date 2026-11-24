# time(3) — Time and clocks

## NAME

time - ticks, milliseconds, microseconds and wall-clock seconds

## SYNOPSIS

`ticks() -> ClockT`; `millis() -> u64`; `micros() -> u64`; `wall_seconds() -> u64`

## DESCRIPTION

Two files define time: `lib/time.rs` (types and constants) and `arch/time.rs`
(per-architecture clock sources and counters).

## Types and constants (`lib/time.rs`)

```rust
pub type ClockT = isize;

#[repr(C)]
pub struct Tms {
    pub tms_utime: ClockT,
    pub tms_stime: ClockT,
    pub tms_cutime: ClockT,
    pub tms_cstime: ClockT,
}

pub const HZ: u64 = 100;
pub const CLK_TCK: ClockT = 100;
pub const CLOCKS_PER_SEC: ClockT = 100;
```

A tick is therefore 10 ms. `Tms` is the structure filled by `SYS_TIMES`.

## Public functions (`arch/time.rs`)

| Function | Meaning |
| --- | --- |
| `init()` | Called from `arch::init()`; initializes the hardware source, resets the counters and reads the firmware wall clock. |
| `cycles()` | Raw hardware counter value (`RDTSC` or `CNTVCT_EL0`). |
| `ticks()` | Monotonic 10 ms tick count, used for process accounting. |
| `millis()` | Monotonic milliseconds. |
| `micros()` | Monotonic microseconds; used for the boot log timestamps. |
| `cycles_to_ms(c)`, `cycles_to_us(c)` | Convert raw counter deltas. |
| `wall_seconds()` | Seconds since the firmware epoch, if available, otherwise `0`. |

`ticks()`, `millis()` and `micros()` each keep their own
`last`-counter plus accumulated total behind a `SpinLock`. They compute how
many units elapsed since the previous call and add that to the total, so values
advance even if no interrupt fires.

## x86 / x86-64 source

- The Programmable Interval Timer (PIT, ports `0x40`/`0x43`) is programmed with
  divisor 11932 (~100 Hz). `arch_init` measures eight PIT periods with `RDTSC`
  to calibrate `CYCLES_PER_TICK`.
- `ticks()` divides elapsed TSC cycles by `CYCLES_PER_TICK`; `millis()` and
  `micros()` use 1/10 and 1/1000 of that value. On 32-bit x86, division uses an
  inline `div` helper and counters are tracked as 32-bit values with wrapping
  subtraction.
- Wall-clock seconds come from CMOS registers `0x00..0x09` (BCD or binary,
  12/24-hour, year assumed to be 2000+). Two consecutive identical reads are
  required; otherwise the wall clock is marked invalid.

## AArch64 source

- `CNTFRQ_EL0` supplies the counter frequency; `CNTVCT_EL0` is the raw counter.
- `ticks`, milliseconds and microseconds are derived by dividing the frequency
  by `HZ`, 1000 and 1 000 000 respectively.
- Wall-clock seconds come from the PL031 RTC at `0x09010000`; `u32::MAX` is
  treated as invalid.

## Relationship to processes

`process::account_current()` uses `ticks()` to charge elapsed time to
`tms_utime`. `SYS_TIMES` returns the counters and the current tick value;
`SYS_CLOCK` returns user plus system time. There is no interval timer that
sends signals or preempts a running process.

## EXAMPLES

### `millis`, `ticks`, `wall_seconds` — Rust

```rust
let start = crate::arch::time::millis();
// ... work ...
let elapsed_ms = crate::arch::time::millis() - start;

let current_tick = crate::arch::time::ticks();   // 10 ms units
let wall = crate::arch::time::wall_seconds();    // CMOS / PL031 seconds
```

### `cycles` — C

```c
#include <stdint.h>

/* Exported from Rust: RDTSC on x86, CNTVCT_EL0 on AArch64. */
extern uint64_t arch_cycles(void);

uint64_t start = arch_cycles();
/* ... work ... */
uint64_t elapsed = arch_cycles() - start;
```

### PIT setup — C

```c
#include <stdint.h>

/* io_outb/io_inb are exported from lib/io.rs. */
extern void io_outb(uint16_t port, uint8_t value);
extern uint8_t io_inb(uint16_t port);

io_outb(0x43, 0x34);   /* channel 0, lobyte/hibyte, mode 2 */
uint8_t low = io_inb(0x40);   /* channel 0 data port */
```

## FILES

- `lib/time.rs` — `ClockT`, `Tms`, `HZ`.
- `arch/time.rs` — PIT/TSC, architectural counter and RTC sources.

## SEE ALSO

[syscall](syscall.md), [process](process.md), [lib](lib.md)
