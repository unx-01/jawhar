# io(9) — I/O and serial output

## NAME

io - UART, port I/O and text output helpers

## SYNOPSIS

`io::putc(u8)`; `io::print(&str)`; `io::pollc() -> Option<u8>`

## DESCRIPTION

Implementation: `lib/io.rs`.

## Output path

`io::putc(b)` writes one byte to the UART and then calls `console::putc(b)`, so
output is mirrored to the serial console and the VGA text buffer (the console
part is a no-op on AArch64).

| Architecture | Device | Registers |
| --- | --- | --- |
| x86 / x86-64 | 16550-style COM1 | Data `0x3F8`, line status `0x3FD` (`SERIAL_PORT + 5`). |
| AArch64 | PL011 at `0x09000000` | Data/base `0x09000000`, flag register `0x09000018`. |

- x86 output polls line status bit 5 (transmit holding register empty) before
  writing the data port.
- AArch64 output polls `UARTFR` bit 5 (transmit FIFO full) and writes when the
  bit is clear.

Higher-level helpers:

- `print(s)` iterates over bytes using the `core::fmt::Write` implementation on
  the internal `Uart` struct.
- `println(s)` calls `print(s)` then `print("\r\n")`.
- `put_byte(b)` is the unsafe `putc` wrapped in a safe function; it is what
  `SYS_WRITE` uses.

## Input polling

`io::pollc()` returns `Some(byte)` when a byte is waiting, otherwise `None`.

- x86: line status bit 0 (data ready), then read `0x3F8`.
- AArch64: `UARTFR` bit 4 (receive FIFO empty) clear, then read the data
  register and keep the low 8 bits.

The [TTY](tty.md) polls this before the PS/2 keyboard.

## Port helpers (x86 only)

- `io::inb(port)` — `in al, dx`.
- `io::outb(port, val)` — `out dx, al`.

Both are `unsafe` and use `options(nostack, preserves_flags)`. They are used by
the console for the VGA cursor, by the keyboard for ports `0x60`/`0x64`, by the
PIT/CMOS timer code and by the APIC setup.

## Relationship to `printk`

`printk` (in `lib/printk.rs`) does not call `io::print` directly. It calls
`write!(stdout, ...)`, i.e. `SYS_WRITE`, which validates the descriptor and
then calls `io::put_byte` for each byte. This keeps kernel messages on the same
path as other output.

## EXAMPLES

### `print`, `pollc`, `outb` — Rust

```rust
crate::io::println("ready");

if let Some(byte) = crate::io::pollc() {
    crate::console::putc(byte);
}

unsafe { crate::io::outb(0x3F8, b'X'); }   // raw COM1 write
```

### `outb` — C

```c
#include <stdint.h>

/* io_outb is exported from lib/io.rs. */
extern void io_outb(uint16_t port, uint8_t value);

io_outb(0x3F8, 'X');   /* write 'X' to COM1 */
```

### `inb` / `pollc` — C

```c
#include <stdint.h>

extern uint8_t io_inb(uint16_t port);
extern int io_pollc(uint8_t *byte);   /* 1 when a byte is ready */

if (io_inb(0x3FD) & 1) {             /* COM1 line status: data ready */
    uint8_t byte = io_inb(0x3F8);    /* COM1 data register */
    console_putc(byte);
}

uint8_t next;
if (io_pollc(&next)) {
    console_putc(next);
}
```

## FILES

- `lib/io.rs` — COM1/PL011 access, `print`, `println`, `inb`, `outb`.
- `lib/printk.rs` — kernel message helper.

## SEE ALSO

[console](console.md), [tty](tty.md), [keyboard](keyboard.md), [time](time.md)
