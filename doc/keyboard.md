# keyboard(9) — Keyboard

## NAME

keyboard - PS/2 scancode translation and key queue

## SYNOPSIS

`keyboard::poll() -> Option<u8>`; `keyboard::modifiers() -> Modifiers`

## DESCRIPTION

Implementation: `drives/keyboard/keyboard.rs`. x86/x86-64 only; the AArch64
`poll()` returns `None`.

## Hardware interface

- Data port `0x60`, status port `0x64`, read with `in al, dx`.
- Polling only — there is no IRQ/IDT. `keyboard::poll()` is called by the
  [TTY](tty.md) input loop.
- Scancodes are assumed to be IBM PC set 1, with bit `0x80` marking key release.
- A 16-byte ring buffer (`QUEUE`, `QUEUE_HEAD`, `QUEUE_TAIL`) holds bytes that
  must be emitted together, such as Alt sequences or escape-sequence payloads.

## Modifier state

`Modifiers { shift, ctrl, alt, sup }` is returned by `keyboard::modifiers()`.
Internal atomic flags track Shift (scancodes `0x2A`/`0x36`), Ctrl (`0x1D`),
Alt (`0x38`), Super (`0x5B`/`0x5C`), Caps Lock (`0x3A`, toggled on press) and
an `EXTENDED` flag for the `0xE0` prefix.

## Translation

Release codes never produce output; they only clear modifier flags. Press codes
within the 64-entry `NORMAL` table become ASCII bytes. For alphabetic keys,
Shift XOR Caps Lock selects uppercase; for other keys Shift selects the
`SHIFTED` table (e.g. `1` → `!`). A zero entry produces no byte.

With Ctrl held, letters become control codes `1..26`, `@`/space → `0x00`,
`[` → `ESC`, `\` → `0x1C`, `]` → `0x1D`, `^` → `0x1E`, `_` → `0x1F`,
`?` → `0x7F`. With Alt held, the resulting byte is queued and `ESC` (`0x1B`)
is returned first.

Extended (`0xE0`) keys:

- Ctrl/Alt/Super modifiers from their extended scancodes update the flags.
- Navigation keys are queued as ANSI payloads and `ESC` is returned:

| Key | Payload |
| --- | --- |
| Up / Down / Right / Left | `[A`, `[B`, `[C`, `[D` |
| Home / End | `[H`, `[F` |
| Insert / Delete | `[2~`, `[3~` |
| Page Up / Page Down | `[5~`, `[6~` |

The TTY's `readline` recognises ESC followed by `[` or `O` and stores the whole
sequence without echoing it.

## Data flow

```
PS/2 port 0x64/0x60 -> poll() -> optional byte -> console::wait_byte()
                                               -> readline() -> SYS_WRITE -> UART + VGA
```

Bytes placed in the ring buffer are returned on subsequent `poll()` calls,
before new hardware scancodes are read. If the queue is full, new bytes are
dropped.

## EXAMPLES

### `poll` and `modifiers` — Rust

```rust
if let Some(byte) = crate::keyboard::poll() {
    crate::console::putc(byte);
}

let mods = crate::keyboard::modifiers();
if mods.shift && mods.ctrl {
    crate::io::println("shift+ctrl");
}
```

### Reading a scancode — C

```c
#include <stdint.h>

extern uint8_t io_inb(uint16_t port);

if (io_inb(0x64) & 1) {             /* status port: output buffer full */
    uint8_t scancode = io_inb(0x60); /* data port */
    console_putc(scancode);         /* or pass to keyboard::poll() */
}
```

## FILES

- `drives/keyboard/keyboard.rs` — scancode tables, ring queue and modifiers.

## SEE ALSO

[tty](tty.md), [console](console.md), [io](io.md)
