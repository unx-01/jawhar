# console(9) — Console

## NAME

console - VGA text output and cursor control

## SYNOPSIS

`console::putc(u8)`; `console::clear()`; `console::gotoxy(x, y)`

## DESCRIPTION

Implementation: `drives/console/console.rs`.

The console is the *output* side of the display. Text input is handled by the
[keyboard](keyboard.md) and [TTY](tty.md) modules. All kernel output normally
reaches both the serial port (via `io::put_byte`) and the console.

## x86 / x86-64: VGA text mode

| Constant | Value |
| --- | --- |
| `VGA_BUFFER` | `0xB8000` |
| `VGA_WIDTH` | 80 columns |
| `VGA_HEIGHT` | 25 rows |
| Attribute byte | `0x07` (light grey on black) |

Each cell is two bytes: character then attribute. Writes use
`core::ptr::write_volatile` so the compiler cannot cache or reorder them.

Functions:

- `console()` — the boot service; only calls `clear()`.
- `putc(b)` — handles `\n` (column 0, next row), `\r` (column 0), `0x08`/`0x7F`
  (move back one column and erase with a space) and printable bytes. After a
  row overflow it calls `vga_scroll()`.
- `vga_scroll()` — copies every row up by one with volatile 16-bit accesses,
  clears the last row and leaves `VGA_ROW` at row 24.
- `clear()` — fills all 2000 cells with spaces and resets the cursor.
- `gotoxy(x, y)` — clamps to the screen and updates the hardware cursor.
- `vga_update_cursor()` — writes the linear position to CRT controller registers
  `0x3D4` (index 14/15) and `0x3D5` (high/low byte) through `io::outb`.

`VGA_ROW` and `VGA_COL` are `static mut`; access is not synchronized.

## AArch64

`putc`, `clear` and `gotoxy` are no-ops. Text output on AArch64 goes to the
PL011 UART in `lib/io.rs`. The file contains private helpers
(`ansi_push_byte`, `ansi_push_u16`) that look like scaffolding for ANSI-style
output but are not called.

## Usage

- `io::putc` calls `console::putc` after writing the byte to the UART, so
  `print`, `println` and `SYS_WRITE` mirror their output to the VGA buffer.
- `printk` writes through `SYS_WRITE`, and the boot log is flushed with
  `printk` after `console()` has cleared the screen.
- The [TTY](tty.md) echo path also uses `SYS_WRITE`, so typed characters appear
  on both serial and VGA.

## EXAMPLES

### `putc`, `gotoxy`, `clear` — Rust

```rust
crate::console::putc(b'A');
crate::console::gotoxy(0, 2);     // column 0, row 2
crate::console::clear();          // blank screen, cursor at (0, 0)
```

### Writing a VGA cell — C

```c
#include <stdint.h>

volatile uint16_t *vga = (volatile uint16_t *)0xB8000;
vga[0] = 0x0741;   /* 0x41 = 'A', 0x07 = light grey on black */
```

### Moving the hardware cursor — C

```c
/* io_outb is exported from lib/io.rs. */
extern void io_outb(uint16_t port, uint8_t value);

io_outb(0x3D4, 14);   /* CRT index: cursor high byte */
io_outb(0x3D5, 0);    /* CRT data: high byte of position 0 */
io_outb(0x3D4, 15);   /* CRT index: cursor low byte */
io_outb(0x3D5, 0);    /* CRT data: low byte of position 0 */
```

## FILES

- `drives/console/console.rs` — VGA buffer, scrolling and cursor handling.
- `lib/io.rs` — serial output that calls `console::putc`.

## SEE ALSO

[tty](tty.md), [keyboard](keyboard.md), [io](io.md), [syscall](syscall.md)
