# build(7) — Build and run

## NAME

build - Makefile toolchain, targets and QEMU invocation

## SYNOPSIS

`make [all|build|run|run-vnc|clean|docs] [ARCH=x86|x86-64|aarch64] [VNC=:0]`

## DESCRIPTION

## Toolchain

There is no Cargo project; the Makefile invokes the tools directly.

| Tool | Used for |
| --- | --- |
| `rustc` | Compiles all Rust sources as one `no_std` crate named `jawhar`, using `arch/init.rs` as the crate root. Edition 2021, `-O`, `panic=abort`, `-C relocation-model=static`, no debug info. |
| `clang` | Assembles `arch/<arch>/boot.S` for a bare-metal target. |
| `ld.lld` (`LD`) | Links with the architecture linker script, `--gc-sections -static -nostdlib`. |
| `llvm-objcopy` or `objcopy` | Produces `jawhar32.elf` for x86-64. |
| QEMU | `qemu-system-i386`, `qemu-system-x86_64`, `qemu-system-aarch64`. |

Rust targets are discovered from `rustc --print sysroot`. The first installed
target from each list is used, otherwise the last entry is the fallback:

| `ARCH` | Rust target candidates | Clang target | Linker emulation |
| --- | --- | --- | --- |
| `x86` | `i686-unknown-linux-gnu`, `i686-linux-android` | `i386-unknown-none-elf` | `elf_i386` |
| `x86-64` | `x86_64-unknown-none`, `x86_64-linux-android` | `x86_64-unknown-none-elf` | `elf_x86_64` |
| `aarch64` | `aarch64-unknown-none`, `aarch64-linux-android` | `aarch64-unknown-none-elf` | `aarch64elf` |

For x86 and x86-64, the Rust command also disables SSSE3/SSE3/SSE4.1/SSE4.2/popcnt
target features. `arch::init` separately enables SSE/SSE2 in CR4.

## Make variables and targets

- `ARCH ?= x86-64` — selected architecture.
- `VNC ?= :0` — VNC display used by the `run-vnc` targets.

| Target | Action |
| --- | --- |
| `all` (default) | Build `x86`, `x86-64` and `aarch64`. |
| `build` | Build `$(ARCH)` only. |
| `x86`, `x86-64`, `aarch64` | Build the named architecture. x86-64 produces both `jawhar.elf` and `jawhar32.elf`. |
| `run` | Build and boot `$(ARCH)` with `-display none -serial stdio`. |
| `run-vnc` | Build and boot `$(ARCH)` with VNC; serial output goes to `bin/$(ARCH)/serial.log`. |
| `run-x86`, `run-x86-64`, `run-aarch64` | Build and run one architecture with the serial console. |
| `run-vnc-<arch>` | Same as `run-vnc` for an explicit architecture. |
| `stop`, `stop-all` | Kill the QEMU process recorded in `qemu.pid`. |
| `clean` | Stop all QEMU instances and remove `bin/`. |
| `docs` / `doc` | List documentation in `doc/` (Markdown pages and manual pages). |
| `help` | Print the Makefile help text. |

QEMU memory is 128 MiB for every architecture. AArch64 uses
`-machine virt -cpu cortex-a57`.

## Build rules

1. `bin/<arch>/boot.o` ← `arch/<arch>/boot.S` (clang).
2. `bin/<arch>/init.o` ← every source file except `bin/`, `obj/`, `.git/` and
   `doc/` (rustc, via `#[path]` modules in `arch/init.rs`). Any source change
   rebuilds this object.
3. `bin/<arch>/jawhar.elf` ← `boot.o` + `init.o` + `arch/<arch>/linker.ld`
   (ld.lld, `ENTRY(_start)`).
4. `bin/x86-64/jawhar32.elf` ← `jawhar.elf` via `objcopy -O elf32-i386`.
   This is the file the x86-64 `run` targets pass to QEMU.

Linker scripts place x86/x86-64 at `1M` and AArch64 at `0x40080000`. Sections
`.eh_frame`, `.comment`, notes and LLVM address signatures are discarded.

## Output and logs

- `bin/x86/jawhar.elf`, `bin/x86-64/jawhar.elf`, `bin/x86-64/jawhar32.elf`,
  `bin/aarch64/jawhar.elf`.
- `bin/<arch>/serial.log` — serial console output from VNC runs.
- `bin/<arch>/qemu.pid` — PID of the daemonized QEMU process.

Example serial output after boot:

```
[boot] arch: 93 ms
[boot] pmm: 3 ms
[boot] vmm: 1 ms
[boot] process: 0 ms
[boot] console: 6 ms
```

After that, the [TTY](tty.md) echoes whatever is typed on the serial console or
keyboard.

## FILES

- `Makefile` — build rules and targets.
- `bin/<arch>/` — kernels, serial logs and QEMU PID files.

## SEE ALSO

[boot](boot.md), [console](console.md), [tty](tty.md)
