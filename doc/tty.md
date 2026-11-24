# tty(4) — TTY

## NAME

tty - line input, editing and echo loop

## SYNOPSIS

`tty::tty() -> !`; `console::readline(prompt) -> &str`

## DESCRIPTION

Implementation: `drives/tty/tty.rs` and `console::readline` in
`drives/console/console.rs`.

## Main loop

```rust
pub fn tty() {
    loop {
        let line = console::readline("");
        crate::write!(1, line, line.len());
        crate::write!(1, "\n", 1);
    }
}
```

`__init` enters this loop as its final action. There is no command parsing: the
line is echoed back through `SYS_WRITE` followed by a newline, which makes the
kernel behave like an echo terminal. Nothing is dispatched to a shell.

Input arrives from either source polled by `console::wait_byte()`:

1. `io::pollc()` — serial UART.
2. `keyboard::poll()` — PS/2 keyboard on x86/x86-64 (returns `None` on AArch64).

The loop spins with `core::hint::spin_loop()` while no byte is available.

## `readline(prompt)`

- Prints `prompt` when non-empty (the TTY passes `""`).
- Uses a single static `LINE_BUF` of `LINE_MAX = 128` bytes. Each caller gets a
  `&str` view over this buffer; the previous line is overwritten on the next
  call.
- A parallel `echoed` array tracks which bytes were displayed, so erasing a
  byte only backspaces over characters that were actually echoed (escape
  sequences are stored but not displayed).
- Returns when Enter is pressed (or on Ctrl-C/Ctrl-D with an empty line). The
  byte slice is converted with `from_utf8_unchecked`; there is no UTF-8
  validation, but continuation bytes are handled when erasing.

## Editing keys

| Input | Behavior |
| --- | --- |
| `\r`, `\n` | Print `\r\n` and return the line. |
| `0x03` (Ctrl-C) | Print `^C\r\n` and call exit(1), return an empty string. |
| `0x04` (Ctrl-D) | Return an empty string only if the line is empty; otherwise ignored. |
| `0x15` (Ctrl-U) | Erase the entire line. |
| `0x17` (Ctrl-W) | Erase the previous whitespace-delimited word. |
| `0x08`, `0x7F` (Backspace/DEL) | Erase the last UTF-8 character. |
| `0x00..=0x1F` | Ignored (except the controls above). |
| `0x1B` (ESC) | Start of an escape sequence; see below. |
| Printable byte | Append and echo it. |

`remove_last_char` walks back over UTF-8 continuation bytes (`0x80..=0xBF`)
so backspace removes a whole character.

## Escape sequences

When ESC is followed by `[` or `O`, `readline` stores ESC, the second byte and
up to 16 more bytes until a byte in `0x40..=0x7E` terminates the sequence, which
is typical for arrow and function keys. None of these bytes are echoed. An ESC
not followed by `[`/`O` is stored without echo and the following byte is then
processed normally.

The keyboard driver supplies ANSI-style payloads for arrows, Home/End,
Page Up/Down, Insert and Delete; Alt+key is reported as ESC followed by the
character. See [Keyboard](keyboard.md).

## Limitations

- Lines longer than 128 bytes are truncated.
- The returned string is a view of a mutable static buffer; it is not preserved
  across the next `readline` call.
- Output uses `\r\n` on Enter but the echo after `readline` appends a bare
  `\n`; on the VGA console `\n` moves down a row and resets the column, while
  the serial port sees only the line feed.
- No history, tab completion or command execution is implemented.

## EXAMPLES

### `readline` — Rust

```rust
let line = crate::console::readline("> ");
crate::write!(1, line, line.len());
```

### `tty::tty` — Rust

```rust
pub fn tty() -> ! {
    loop {
        let line = crate::console::readline("");
        crate::write!(1, line, line.len());
        crate::write!(1, "\n", 1);
    }
}
```

### Polling the serial input used by `readline` — C

```c
#include <stdint.h>

/* io_pollc is exported from lib/io.rs. */
extern int io_pollc(uint8_t *byte);   /* 1 when a byte is ready */

uint8_t byte;
while (!io_pollc(&byte)) {
    /* spin until the serial port has input */
}
console_putc(byte);
```

## FILES

- `drives/tty/tty.rs` — echo loop.
- `drives/console/console.rs` — `readline` and line buffer.
- `drives/keyboard/keyboard.rs`, `lib/io.rs` — input sources.

## SEE ALSO

[console](console.md), [keyboard](keyboard.md), [io](io.md), [syscall](syscall.md)
