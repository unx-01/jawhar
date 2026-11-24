use crate::io;
use crate::keyboard;

const LINE_MAX: usize = 128;

static mut LINE_BUF: [u8; LINE_MAX] = [0; LINE_MAX];

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const VGA_WIDTH: usize = 80;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const VGA_HEIGHT: usize = 25;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const VGA_BUFFER: usize = 0xB8000;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static mut VGA_ROW: usize = 0;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static mut VGA_COL: usize = 0;

pub fn console() {
    clear();
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn vga_write_at(row: usize, col: usize, byte: u8, attr: u8) {
    let offset = (row * VGA_WIDTH + col) * 2;
    core::ptr::write_volatile((VGA_BUFFER + offset) as *mut u8, byte);
    core::ptr::write_volatile((VGA_BUFFER + offset + 1) as *mut u8, attr);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn vga_update_cursor() {
    let row = *core::ptr::addr_of!(VGA_ROW);
    let col = *core::ptr::addr_of!(VGA_COL);
    let pos = (row * VGA_WIDTH + col) as u16;
    io::outb(0x3D4, 14);
    io::outb(0x3D5, (pos >> 8) as u8);
    io::outb(0x3D4, 15);
    io::outb(0x3D5, pos as u8);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn vga_scroll() {
    for row in 1..VGA_HEIGHT {
        for col in 0..VGA_WIDTH {
            let src = (VGA_BUFFER + (row * VGA_WIDTH + col) * 2) as *const u16;
            let dst = (VGA_BUFFER + ((row - 1) * VGA_WIDTH + col) * 2) as *mut u16;
            core::ptr::write_volatile(dst, core::ptr::read_volatile(src));
        }
    }
    for col in 0..VGA_WIDTH {
        vga_write_at(VGA_HEIGHT - 1, col, b' ', 0x07);
    }
    *core::ptr::addr_of_mut!(VGA_ROW) = VGA_HEIGHT - 1;
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub unsafe fn putc(b: u8) {
    match b {
        b'\n' => {
            *core::ptr::addr_of_mut!(VGA_COL) = 0;
            *core::ptr::addr_of_mut!(VGA_ROW) += 1;
        }
        b'\r' => {
            *core::ptr::addr_of_mut!(VGA_COL) = 0;
        }
        0x08 | 0x7F => {
            let col = *core::ptr::addr_of!(VGA_COL);
            if col > 0 {
                let new_col = col - 1;
                *core::ptr::addr_of_mut!(VGA_COL) = new_col;
                let row = *core::ptr::addr_of!(VGA_ROW);
                vga_write_at(row, new_col, b' ', 0x07);
            }
        }
        _ => {
            let row = *core::ptr::addr_of!(VGA_ROW);
            let col = *core::ptr::addr_of!(VGA_COL);
            vga_write_at(row, col, b, 0x07);
            let new_col = col + 1;
            if new_col >= VGA_WIDTH {
                *core::ptr::addr_of_mut!(VGA_COL) = 0;
                *core::ptr::addr_of_mut!(VGA_ROW) = row + 1;
            } else {
                *core::ptr::addr_of_mut!(VGA_COL) = new_col;
            }
        }
    }

    while *core::ptr::addr_of!(VGA_ROW) >= VGA_HEIGHT {
        vga_scroll();
    }
    vga_update_cursor();
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn clear() {
    unsafe {
        for row in 0..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                vga_write_at(row, col, b' ', 0x07);
            }
        }
        *core::ptr::addr_of_mut!(VGA_ROW) = 0;
        *core::ptr::addr_of_mut!(VGA_COL) = 0;
        vga_update_cursor();
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn gotoxy(x: u16, y: u16) {
    unsafe {
        let row = (y as usize).min(VGA_HEIGHT - 1);
        let col = (x as usize).min(VGA_WIDTH - 1);
        *core::ptr::addr_of_mut!(VGA_ROW) = row;
        *core::ptr::addr_of_mut!(VGA_COL) = col;
        vga_update_cursor();
    }
}

#[cfg(target_arch = "aarch64")]
pub unsafe fn putc(_b: u8) {}

#[cfg(target_arch = "aarch64")]
fn ansi_push_byte(buf: &mut [u8; 16], len: &mut usize, b: u8) {
    if *len < buf.len() {
        unsafe {
            *buf.get_unchecked_mut(*len) = b;
        }
        *len += 1;
    }
}

#[cfg(target_arch = "aarch64")]
fn ansi_push_u16(buf: &mut [u8; 16], len: &mut usize, mut n: u16) {
    if n == 0 {
        ansi_push_byte(buf, len, b'0');
        return;
    }
    let mut digits = [0u8; 5];
    let mut count = 0usize;
    while n > 0 && count < digits.len() {
        unsafe {
            *digits.get_unchecked_mut(count) = b'0' + (n % 10) as u8;
        }
        n /= 10;
        count += 1;
    }
    while count > 0 {
        count -= 1;
        let d = unsafe { *digits.get_unchecked(count) };
        ansi_push_byte(buf, len, d);
    }
}

#[cfg(target_arch = "aarch64")]
pub fn clear() {}

#[cfg(target_arch = "aarch64")]
pub fn gotoxy(_x: u16, _y: u16) {}

fn wait_byte() -> u8 {
    loop {
        if let Some(b) = io::pollc() {
            return b;
        }
        if let Some(b) = keyboard::poll() {
            return b;
        }
        core::hint::spin_loop();
    }
}

fn is_space(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

fn remove_last_char(buf: &[u8], mut len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    len -= 1;
    while len > 0 {
        let b = unsafe { *buf.get_unchecked(len) };
        if (b & 0xC0) != 0x80 {
            break;
        }
        len -= 1;
    }
    len
}

fn line_push_byte(
    buf: &mut [u8; LINE_MAX],
    echoed: &mut [bool; LINE_MAX],
    len: usize,
    byte: u8,
    echo: bool,
) -> usize {
    if len >= LINE_MAX {
        return len;
    }
    unsafe {
        *buf.get_unchecked_mut(len) = byte;
        *echoed.get_unchecked_mut(len) = echo;
    }
    if echo {
        io::put_byte(byte);
    }
    len + 1
}

fn line_erase_byte(buf: &[u8; LINE_MAX], echoed: &[bool; LINE_MAX], len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if unsafe { *echoed.get_unchecked(len - 1) } {
        io::print("\x08 \x08");
    }
    remove_last_char(buf, len)
}

pub fn readline(prompt: &str) -> &str {
    if !prompt.is_empty() {
        io::print(prompt);
    }

    let line = unsafe { &mut *core::ptr::addr_of_mut!(LINE_BUF) };
    let mut echoed = [false; LINE_MAX];
    let mut len = 0usize;

    loop {
        let mut byte = wait_byte();

        if byte == 0x1B {
            let next = wait_byte();
            if next == b'[' || next == b'O' {
                len = line_push_byte(line, &mut echoed, len, 0x1B, false);
                len = line_push_byte(line, &mut echoed, len, next, false);
                let mut count = 0;
                while count < 16 {
                    let c = wait_byte();
                    len = line_push_byte(line, &mut echoed, len, c, false);
                    if (0x40..=0x7E).contains(&c) {
                        break;
                    }
                    count += 1;
                }
                continue;
            }
            len = line_push_byte(line, &mut echoed, len, 0x1B, false);
            byte = next;
        }

        match byte {
            b'\r' | b'\n' => {
                io::print("\r\n");
                let slice = unsafe { core::slice::from_raw_parts(line.as_ptr(), len) };
                return unsafe { core::str::from_utf8_unchecked(slice) };
            }
            0x03 => {
                io::print("^C\r\n");
                return "";
            }
            0x04 => {
                if len == 0 {
                    return "";
                }
            }
            0x15 => {
                while len > 0 {
                    len = line_erase_byte(line, &echoed, len);
                }
            }
            0x17 => {
                while len > 0 && is_space(unsafe { *line.get_unchecked(len - 1) }) {
                    len = line_erase_byte(line, &echoed, len);
                }
                while len > 0 && !is_space(unsafe { *line.get_unchecked(len - 1) }) {
                    len = line_erase_byte(line, &echoed, len);
                }
            }
            0x08 | 0x7F => {
                len = line_erase_byte(line, &echoed, len);
            }
            0x00..=0x1F => {}
            _ => {
                len = line_push_byte(line, &mut echoed, len, byte, true);
            }
        }
    }
}
