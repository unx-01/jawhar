/*
 * console
 */
use crate::io;

pub fn console() {
    clear();
}

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

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn write_at(row: usize, col: usize, byte: u8, attr: u8) {
    let offset = (row * VGA_WIDTH + col) * 2;
    core::ptr::write_volatile((VGA_BUFFER + offset) as *mut u8, byte);
    core::ptr::write_volatile((VGA_BUFFER + offset + 1) as *mut u8, attr);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn update_cursor() {
    let pos = (VGA_ROW * VGA_WIDTH + VGA_COL) as u16;
    io::outb(0x3D4, 14);
    io::outb(0x3D5, (pos >> 8) as u8);
    io::outb(0x3D4, 15);
    io::outb(0x3D5, pos as u8);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
unsafe fn scroll() {
    for row in 1..VGA_HEIGHT {
        for col in 0..VGA_WIDTH {
            let src = (VGA_BUFFER + (row * VGA_WIDTH + col) * 2) as *const u16;
            let dst = (VGA_BUFFER + ((row - 1) * VGA_WIDTH + col) * 2) as *mut u16;
            core::ptr::write_volatile(dst, core::ptr::read_volatile(src));
        }
    }

    for col in 0..VGA_WIDTH {
        write_at(VGA_HEIGHT - 1, col, b' ', 0x07);
    }

    VGA_ROW = VGA_HEIGHT - 1;
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub unsafe fn putc(b: u8) {
    match b {
        b'\n' => {
            VGA_COL = 0;
            VGA_ROW += 1;
        }
        b'\r' => {
            VGA_COL = 0;
        }
        0x08 | 0x7F => {
            if VGA_COL > 0 {
                VGA_COL -= 1;
                write_at(VGA_ROW, VGA_COL, b' ', 0x07);
            }
        }
        _ => {
            write_at(VGA_ROW, VGA_COL, b, 0x07);
            VGA_COL += 1;
            if VGA_COL >= VGA_WIDTH {
                VGA_COL = 0;
                VGA_ROW += 1;
            }
        }
    }

    while VGA_ROW >= VGA_HEIGHT {
        scroll();
    }

    update_cursor();
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn clear() {
    unsafe {
        for row in 0..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                write_at(row, col, b' ', 0x07);
            }
        }
        VGA_ROW = 0;
        VGA_COL = 0;
        update_cursor();
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub fn gotoxy(x: u16, y: u16) {
    unsafe {
        VGA_ROW = (y as usize).min(VGA_HEIGHT - 1);
        VGA_COL = (x as usize).min(VGA_WIDTH - 1);
        update_cursor();
    }
}

#[cfg(target_arch = "aarch64")]
fn push_byte(buf: &mut [u8; 16], len: &mut usize, b: u8) {
    if *len < buf.len() {
        buf[*len] = b;
        *len += 1;
    }
}

#[cfg(target_arch = "aarch64")]
fn push_u16(buf: &mut [u8; 16], len: &mut usize, mut n: u16) {
    let mut digits = [0u8; 5];
    let mut count = 0;

    if n == 0 {
        push_byte(buf, len, b'0');
        return;
    }

    while n > 0 {
        digits[count] = b'0' + (n % 10) as u8;
        n /= 10;
        count += 1;
    }

    while count > 0 {
        count -= 1;
        push_byte(buf, len, digits[count]);
    }
}

#[cfg(target_arch = "aarch64")]
pub fn clear() {}

#[cfg(target_arch = "aarch64")]
pub fn gotoxy(x: u16, y: u16) {
    let mut buf = [0u8; 16];
    let mut len = 0;

    push_byte(&mut buf, &mut len, 0x1b);
    push_byte(&mut buf, &mut len, b'[');
    push_u16(&mut buf, &mut len, y + 1);
    push_byte(&mut buf, &mut len, b';');
    push_u16(&mut buf, &mut len, x + 1);
    push_byte(&mut buf, &mut len, b'H');

    io::print(unsafe { core::str::from_utf8_unchecked(&buf[..len]) });
}
